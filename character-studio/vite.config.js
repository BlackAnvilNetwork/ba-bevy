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

function validateBaCharacter(raw) {
  const c = JSON.parse(raw)
  if (c.schemaVersion !== 1) throw new Error('schemaVersion must be 1')
  if (!Number.isInteger(c.revision) || c.revision < 1) throw new Error('revision')
  if (typeof c.characterId !== 'string' || !c.characterId.trim()) throw new Error('characterId')
  if (typeof c.name !== 'string' || c.name.length < 1 || c.name.length > 24) {
    throw new Error('name')
  }
  if (!JOBS.has(c.startingJob)) throw new Error('startingJob')
  if (!MANIFESTS.has(c.manifestId)) throw new Error('manifestId')
  if (c.pack !== 'anata') throw new Error('pack')
  if (!c.runtime || c.runtime.format !== 'glb') throw new Error('runtime.format')
  // Server/Bevy owns asset path — ignore client relativePath.
  c.runtime.relativePath = 'character/exports/active.glb'
  return c
}

function assertGlb(buf) {
  if (!buf || buf.length < 12) throw new Error('glb too small')
  if (buf.length > MAX_BODY) throw new Error('glb too large')
  const magic = buf.toString('ascii', 0, 4)
  if (magic !== 'glTF') throw new Error('glb magic must be glTF')
}

/** Dev-only save bridge — loopback, size-capped, schema+GLB checks. Disabled in build. */
function baSaveDraftPlugin() {
  return {
    name: 'ba-save-draft',
    apply: 'serve',
    configureServer(server) {
      server.middlewares.use(async (req, res, next) => {
        if (req.method !== 'POST' || req.url?.split('?')[0] !== '/ba/save-draft') {
          next()
          return
        }
        try {
          if (!isLoopback(req)) {
            res.statusCode = 403
            res.end('loopback only')
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
          const charDir = path.join(exportsDir, 'by-id', character.characterId)
          fs.mkdirSync(charDir, { recursive: true })
          const jsonText = JSON.stringify(character, null, 2)
          const tmpJson = path.join(charDir, `.active.json.${process.pid}.tmp`)
          const tmpGlb = path.join(charDir, `.active.glb.${process.pid}.tmp`)
          fs.writeFileSync(tmpJson, jsonText)
          fs.writeFileSync(tmpGlb, glbPart.data)
          fs.renameSync(tmpJson, path.join(charDir, 'character.json'))
          fs.renameSync(tmpGlb, path.join(charDir, 'character.glb'))
          // Dev pointer for Bevy AssetServer (prototype — not installed-release storage).
          fs.copyFileSync(path.join(charDir, 'character.json'), path.join(exportsDir, 'active.json'))
          fs.copyFileSync(path.join(charDir, 'character.glb'), path.join(exportsDir, 'active.glb'))

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
    if (buffer[start] === 45 && buffer[start + 1] === 45) break // --
    if (buffer[start] === 13 && buffer[start + 1] === 10) start += 2
    const next = buffer.indexOf(sep, start)
    if (next < 0) break
    let partBuf = buffer.subarray(start, next - 2) // trim CRLF before boundary
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

// https://vitejs.dev/config/
export default defineConfig({
  plugins: [react(), baSaveDraftPlugin()],
  server: {
    host: '127.0.0.1',
    port: 5173,
    strictPort: true,
  },
  //  base: '/CharacterStudio/', NOTE: For Github pages, you need to add the base URL (name of the repo)
  build: {
    outDir: './build',
  },
  resolve: {
    alias: {
      buffer: 'buffer/',
    },
  },
})
