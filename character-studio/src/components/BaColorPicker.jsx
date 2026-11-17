import { useRef } from "react"
import styles from "./BaColorPicker.module.css"

function clamp(value, min, max) {
  return Math.min(max, Math.max(min, value))
}

function hexToRgb(hex) {
  const h = String(hex || "").replace("#", "")
  if (h.length !== 6) return [255, 255, 255]
  return [0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16) || 0)
}

function rgbToHex(r, g, b) {
  return `#${[r, g, b].map((v) => clamp(Math.round(v), 0, 255).toString(16).padStart(2, "0")).join("")}`
}

function rgbToHsv(r, g, b) {
  const rn = r / 255
  const gn = g / 255
  const bn = b / 255
  const max = Math.max(rn, gn, bn)
  const min = Math.min(rn, gn, bn)
  const d = max - min
  let h = 0
  if (d !== 0) {
    if (max === rn) h = ((gn - bn) / d) % 6
    else if (max === gn) h = (bn - rn) / d + 2
    else h = (rn - gn) / d + 4
    h *= 60
    if (h < 0) h += 360
  }
  const s = max === 0 ? 0 : d / max
  return [h, s, max]
}

function hsvToHex(h, s, v) {
  const c = v * s
  const x = c * (1 - Math.abs(((h / 60) % 2) - 1))
  const m = v - c
  let r = 0
  let g = 0
  let b = 0
  if (h < 60) [r, g, b] = [c, x, 0]
  else if (h < 120) [r, g, b] = [x, c, 0]
  else if (h < 180) [r, g, b] = [0, c, x]
  else if (h < 240) [r, g, b] = [0, x, c]
  else if (h < 300) [r, g, b] = [x, 0, c]
  else [r, g, b] = [c, 0, x]
  return rgbToHex((r + m) * 255, (g + m) * 255, (b + m) * 255)
}

export default function BaColorPicker({ hex, onChange }) {
  const [h, s, v] = rgbToHsv(...hexToRgb(hex))
  const pad = useRef(null)

  const pickPad = (event) => {
    const rect = pad.current?.getBoundingClientRect()
    if (!rect) return
    const nextS = clamp((event.clientX - rect.left) / rect.width, 0, 1)
    const nextV = clamp(1 - (event.clientY - rect.top) / rect.height, 0, 1)
    onChange(hsvToHex(h, nextS, nextV))
  }

  const dragPad = (event) => {
    if (event.buttons !== 1 && event.type !== "pointerdown") return
    pickPad(event)
  }

  return (
    <div className={styles.picker}>
      <div
        ref={pad}
        className={styles.pad}
        style={{ background: `hsl(${h} 100% 50%)` }}
        onPointerDown={(event) => {
          event.currentTarget.setPointerCapture(event.pointerId)
          pickPad(event)
        }}
        onPointerMove={dragPad}
      >
        <div className={styles.white} />
        <div className={styles.black} />
        <span className={styles.knob} style={{ left: `${s * 100}%`, top: `${(1 - v) * 100}%` }} />
      </div>
      <input
        className={styles.hue}
        type="range"
        min="0"
        max="360"
        step="1"
        aria-label="Hue"
        value={Math.round(h)}
        onChange={(event) => onChange(hsvToHex(Number(event.target.value), s, v))}
      />
      <label className={styles.hexRow}>
        <span className={styles.chip} style={{ background: hex }} />
        <input
          className={styles.hex}
          aria-label="Hex color"
          value={String(hex || "").toUpperCase()}
          spellCheck={false}
          onChange={(event) => {
            const next = event.target.value.trim()
            if (/^#?[0-9a-fA-F]{6}$/.test(next)) onChange(next.startsWith("#") ? next : `#${next}`)
          }}
        />
      </label>
    </div>
  )
}
