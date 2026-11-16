/**
 * Black Anvil character draft helpers (BaCharacterV1).
 * Body traits stay on the character; Chest/Shoes/etc. are starter equipment.
 */

export const BA_SCHEMA_VERSION = 1
export const BA_CATALOG_VERSION = "anata-2026-11"

const BODY_KEYS = new Set(["body", "head", "eyes", "basic colors"])
const EQUIP_KEYS = new Set([
  "chest",
  "legs",
  "shoes",
  "hands",
  "waist",
  "neck",
  "weapon",
])

const JOBS = [
  "TimeMage",
  "Knight",
  "Rogue",
  "Cleric",
  "Sorcerer",
  "Fighter",
  "BattleMage",
  "Druid",
]

/** Pull forge query params into session so Save can stamp BaCharacterV1. */
export function ingestBaForgeQuery() {
  const params = new URLSearchParams(window.location.search)
  const gender = params.get("ba_gender")
  if (gender) sessionStorage.setItem("ba_gender", gender.toLowerCase())
  const name = params.get("ba_name")
  if (name) {
    sessionStorage.setItem("ba_name", name)
    localStorage.setItem("name", name)
  }
  const job = params.get("ba_job")
  if (job && JOBS.includes(job)) sessionStorage.setItem("ba_job", job)
}

function jobFromStorage() {
  const raw = sessionStorage.getItem("ba_job") || localStorage.getItem("ba_job")
  return JOBS.includes(raw) ? raw : "TimeMage"
}

function nameFromStorage() {
  return (
    sessionStorage.getItem("ba_name") ||
    localStorage.getItem("name") ||
    "Chrona"
  ).slice(0, 24)
}

function genderKey() {
  const fromUrl = new URLSearchParams(window.location.search).get("ba_gender")
  const stored = sessionStorage.getItem("ba_gender")
  const g = (fromUrl || stored || "male").toLowerCase()
  return g === "female" ? "female" : "male"
}

function partitionSelection(selection) {
  const bodyTraits = {}
  const starterEquipment = {}
  for (const [key, value] of Object.entries(selection || {})) {
    if (!value?.id) continue
    const entry = {
      id: value.id,
      name: value.name || value.id,
    }
    const lower = key.toLowerCase()
    if (EQUIP_KEYS.has(lower)) {
      starterEquipment[key] = entry
    } else if (BODY_KEYS.has(lower) || !EQUIP_KEYS.has(lower)) {
      bodyTraits[key] = entry
    }
  }
  return { bodyTraits, starterEquipment }
}

/**
 * Build a BaCharacterV1 object from CharacterManager selection.
 */
export function buildBaCharacterV1(characterManager, overrides = {}) {
  const selection = characterManager.getAvatarSelection()
  const { bodyTraits, starterEquipment } = partitionSelection(selection)
  const gender = genderKey()
  const existingId =
    sessionStorage.getItem("ba_character_id") || crypto.randomUUID()
  sessionStorage.setItem("ba_character_id", existingId)

  const prevRev = Number(sessionStorage.getItem("ba_revision") || "0")
  const revision = prevRev + 1
  sessionStorage.setItem("ba_revision", String(revision))

  return {
    schemaVersion: BA_SCHEMA_VERSION,
    revision,
    characterId: existingId,
    name: overrides.name || nameFromStorage(),
    startingJob: overrides.startingJob || jobFromStorage(),
    pack: "anata",
    manifestId: `anata/${gender}`,
    catalogVersion: BA_CATALOG_VERSION,
    bodyTraits,
    starterEquipment,
    runtime: {
      format: "glb",
      relativePath: "character/exports/active.glb",
    },
  }
}

function blobToBase64(blob) {
  return new Promise((resolve, reject) => {
    const reader = new FileReader()
    reader.onload = () => {
      const dataUrl = String(reader.result || "")
      const comma = dataUrl.indexOf(",")
      resolve(comma >= 0 ? dataUrl.slice(comma + 1) : dataUrl)
    }
    reader.onerror = () => reject(reader.error || new Error("FileReader failed"))
    reader.readAsDataURL(blob)
  })
}

/**
 * Persist draft JSON + GLB via Tauri IPC when hosted in the launcher,
 * otherwise POST to the Vite `/ba/save-draft` middleware.
 */
export async function saveBaCharacterDraft(character, glbBlob) {
  const invoke = window.__TAURI__?.tauri?.invoke || window.__TAURI__?.invoke
  if (typeof invoke === "function") {
    const glbBase64 = await blobToBase64(glbBlob)
    return invoke("save_draft", {
      characterJson: JSON.stringify(character),
      glbBase64,
    })
  }

  const form = new FormData()
  form.append("character", JSON.stringify(character))
  form.append("glb", glbBlob, "active.glb")

  const res = await fetch("/ba/save-draft", {
    method: "POST",
    body: form,
  })
  if (!res.ok) {
    const text = await res.text()
    throw new Error(text || `save-draft failed (${res.status})`)
  }
  return res.json()
}
