import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react-swc'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'
import crypto from 'node:crypto'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const repoRoot = path.resolve(__dirname, '..')
const exportsDir = path.join(repoRoot, 'assets', 'character', 'exports')

const MAX_BODY = 32 * 1024 * 1024 // 32 MiB
const JOBS = new Set([
  'TimeMage',
  'Knight',
  'Rogue',
  'Cleric',
  'Sorcerer',
  'Fighter',
  'BattleMage',
  'Druid',
])
const MANIFESTS = new Set(['anata/male', 'anata/female'])
/** UUID or safe slug only — never path segments. */
const SAFE_CHARACTER_ID = /^[A-Za-z0-9][A-Za-z0-9_-]{0,63}$/

function isLoopback(req) {
  const host = String(req.headers.host || '')
  const ip = req.socket?.remoteAddress || ''
  return (
    host.startsWith('127.0.0.1') ||
    host.startsWith('localhost') ||
    ip === '127.0.0.1' ||
    ip === '::1' ||
    ip === '::ffff:127.0.0.1'
  )
}

function assertSafeCharacterId(id) {
  if (typeof id !== 'string' || !SAFE_CHARACTER_ID.test(id)) {
    throw new Error('characterId must be alphanumeric/[_-] only (no path separators)')
  }
  if (id === '.' || id === '..') throw new Error('characterId reserved')
}

/** Resolve scoped dir and ensure it stays under exports/by-id. */
function scopedCharacterDir(characterId) {
  assertSafeCharacterId(characterId)
  const byIdRoot = path.resolve(exportsDir, 'by-id')
  const charDir = path.resolve(byIdRoot, characterId)
  const rel = path.relative(byIdRoot, charDir)
  if (rel.startsWith('..') || path.isAbsolute(rel)) {
    throw new Error('characterId path escape blocked')
  }
  return charDir
}

function validateBaCharacter(raw) {
  const c = JSON.parse(raw)
  if (c.schemaVersion !== 1) throw new Error('schemaVersion must be 1')
  if (!Number.isInteger(c.revision) || c.revision < 1) throw new Error('revision')
  assertSafeCharacterId(c.characterId)
  if (typeof c.name !== 'string' || c.name.length < 1 || c.name.length > 24) {
    throw new Error('name')
  }
  if (!JOBS.has(c.startingJob)) throw new Error('startingJob')
  if (!MANIFESTS.has(c.manifestId)) throw new Error('manifestId')
  if (c.pack !== 'anata') throw new Error('pack')
  if (!c.runtime || c.runtime.format !== 'glb') throw new Error('runtime.format')
  c.runtime.relativePath = 'character/exports/active.glb'
  return c
}

function assertGlb(buf) {
  if (!buf || buf.length < 12) throw new Error('glb too small')
  if (buf.length > MAX_BODY) throw new Error('glb too large')
  const magic = buf.toString('ascii', 0, 4)
  if (magic !== 'glTF') throw new Error('glb magic must be glTF')
}

function writeTransactionalPair(charDir, jsonText, glbBuf, activeJson, activeGlb) {
  fs.mkdirSync(charDir, { recursive: true })
  const staging = path.join(charDir, `.staging-${process.pid}-${Date.now()}`)
  fs.mkdirSync(staging, { recursive: true })
  const jsonPath = path.join(staging, 'character.json')
  const glbPath = path.join(staging, 'character.glb')
  const markerPath = path.join(staging, 'COMMIT')
  fs.writeFileSync(jsonPath, jsonText)
  fs.writeFileSync(glbPath, glbBuf)
  // Marker last — incomplete staging dirs without COMMIT are ignored/cleaned.
  fs.writeFileSync(markerPath, 'ok\n')

  const liveJson = path.join(charDir, 'character.json')
  const liveGlb = path.join(charDir, 'character.glb')
  const tmpJson = `${liveJson}.${process.pid}.tmp`
  const tmpGlb = `${liveGlb}.${process.pid}.tmp`
  fs.copyFileSync(jsonPath, tmpJson)
  fs.copyFileSync(glbPath, tmpGlb)
  fs.renameSync(tmpJson, liveJson)
  fs.renameSync(tmpGlb, liveGlb)
  fs.writeFileSync(path.join(charDir, 'COMMIT'), `${Date.now()}\n`)

  // Active pointer: write both to staging names then rename (best-effort pair).
  const aJsonTmp = `${activeJson}.${process.pid}.tmp`
  const aGlbTmp = `${activeGlb}.${process.pid}.tmp`
  fs.copyFileSync(liveJson, aJsonTmp)
  fs.copyFileSync(liveGlb, aGlbTmp)
  fs.renameSync(aJsonTmp, activeJson)
  fs.renameSync(aGlbTmp, activeGlb)

  fs.rmSync(staging, { recursive: true, force: true })
}

/** Dev-only save bridge — loopback + CSRF token + size/schema/path checks. */
function baSaveDraftPlugin() {
  const csrfToken = crypto.randomBytes(32).toString('hex')
  return {
    name: 'ba-save-draft',
    apply: 'serve',
    configureServer(server) {
      server.middlewares.use(async (req, res, next) => {
        const urlPath = req.url?.split('?')[0]

        if (req.method === 'GET' && urlPath === '/ba/save-token') {
          if (!isLoopback(req)) {
            res.statusCode = 403
            res.end('loopback only')
            return
          }
          res.setHeader('Content-Type', 'application/json')
          res.setHeader('Cache-Control', 'no-store')
          res.end(JSON.stringify({ token: csrfToken }))
          return
        }

        if (req.method !== 'POST' || urlPath !== '/ba/save-draft') {
          next()
          return
        }
        try {
          if (!isLoopback(req)) {
            res.statusCode = 403
            res.end('loopback only')
            return
          }
          const origin = req.headers.origin
          if (origin && origin !== 'http://127.0.0.1:5173' && origin !== 'http://localhost:5173') {
            res.statusCode = 403
            res.end('origin not allowed')
            return
          }
          const token = req.headers['x-ba-save-token']
          if (token !== csrfToken) {
            res.statusCode = 403
            res.end('missing or invalid X-BA-Save-Token (GET /ba/save-token first)')
            return
          }

          const chunks = []
          let total = 0
          for await (const chunk of req) {
            total += chunk.length
            if (total > MAX_BODY) {
              res.statusCode = 413
              res.end('payload too large')
              return
            }
            chunks.push(chunk)
          }
          const buf = Buffer.concat(chunks)
          const contentType = req.headers['content-type'] || ''
          const boundaryMatch = /boundary=(?:"([^"]+)"|([^;]+))/i.exec(contentType)
          if (!boundaryMatch) {
            res.statusCode = 400
            res.end('multipart boundary required')
            return
          }
          const boundary = boundaryMatch[1] || boundaryMatch[2]
          const parts = parseMultipart(buf, boundary)
          const characterPart = parts.find((p) => p.name === 'character')
          const glbPart = parts.find((p) => p.name === 'glb')
          if (!characterPart || !glbPart) {
            res.statusCode = 400
            res.end('form fields character + glb required')
            return
          }
          const character = validateBaCharacter(characterPart.data.toString('utf8'))
          assertGlb(glbPart.data)
          const hash = crypto.createHash('sha256').update(glbPart.data).digest('hex')
          character.runtime.contentHash = hash

          fs.mkdirSync(exportsDir, { recursive: true })
          const charDir = scopedCharacterDir(character.characterId)
          const jsonText = JSON.stringify(character, null, 2)
          writeTransactionalPair(
            charDir,
            jsonText,
            glbPart.data,
            path.join(exportsDir, 'active.json'),
            path.join(exportsDir, 'active.glb'),
          )

          res.setHeader('Content-Type', 'application/json')
          res.end(
            JSON.stringify({
              ok: true,
              prototype: true,
              json: 'assets/character/exports/active.json',
              glb: 'assets/character/exports/active.glb',
              scoped: `assets/character/exports/by-id/${character.characterId}/`,
              contentHash: hash,
              bytes: glbPart.data.length,
            }),
          )
        } catch (err) {
          res.statusCode = 400
          res.end(String(err?.message || err))
        }
      })
    },
  }
}

function parseMultipart(buffer, boundary) {
  const sep = Buffer.from(`--${boundary}`)
  const parts = []
  let start = buffer.indexOf(sep) + sep.length
  while (start < buffer.length) {
    if (buffer[start] === 45 && buffer[start + 1] === 45) break
    if (buffer[start] === 13 && buffer[start + 1] === 10) start += 2
    const next = buffer.indexOf(sep, start)
    if (next < 0) break
    let partBuf = buffer.subarray(start, next - 2)
    const headerEnd = partBuf.indexOf('\r\n\r\n')
    if (headerEnd >= 0) {
      const header = partBuf.subarray(0, headerEnd).toString('utf8')
      const data = partBuf.subarray(headerEnd + 4)
      const nameMatch = /name="([^"]+)"/i.exec(header)
      const filenameMatch = /filename="([^"]*)"/i.exec(header)
      parts.push({
        name: nameMatch?.[1],
        filename: filenameMatch?.[1],
        data,
      })
    }
    start = next + sep.length
  }
  return parts
}

export default defineConfig({
  plugins: [react(), baSaveDraftPlugin()],
  server: {
    host: '127.0.0.1',
    port: 5173,
    strictPort: true,
  },
  build: {
    outDir: './build',
  },
  resolve: {
    alias: {
      buffer: 'buffer/',
    },
  },
})
