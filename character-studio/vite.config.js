import { defineConfig } from 'vite'
import react from '@vitejs/plugin-react-swc'
import fs from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

const __dirname = path.dirname(fileURLToPath(import.meta.url))
const repoRoot = path.resolve(__dirname, '..')
const exportsDir = path.join(repoRoot, 'assets', 'character', 'exports')

function baSaveDraftPlugin() {
  return {
    name: 'ba-save-draft',
    configureServer(server) {
      server.middlewares.use(async (req, res, next) => {
        if (req.method !== 'POST' || req.url?.split('?')[0] !== '/ba/save-draft') {
          next()
          return
        }
        try {
          const chunks = []
          for await (const chunk of req) {
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
          const characterJson = characterPart.data.toString('utf8')
          JSON.parse(characterJson) // validate
          fs.mkdirSync(exportsDir, { recursive: true })
          fs.writeFileSync(path.join(exportsDir, 'active.json'), characterJson)
          fs.writeFileSync(path.join(exportsDir, 'active.glb'), glbPart.data)
          res.setHeader('Content-Type', 'application/json')
          res.end(
            JSON.stringify({
              ok: true,
              json: 'assets/character/exports/active.json',
              glb: 'assets/character/exports/active.glb',
              bytes: glbPart.data.length,
            }),
          )
        } catch (err) {
          res.statusCode = 500
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
