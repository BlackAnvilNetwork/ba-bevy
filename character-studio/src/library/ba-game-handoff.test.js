// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest"

vi.mock("./store", () => ({
  local: {
    mergeOptions_sel_option: 0,
    mergeOptions_create_atlas: true,
  },
}))
vi.mock("./utils", () => ({
  getAtlasSize: vi.fn((size) => size),
}))
vi.mock("./ba-character", async (importOriginal) => {
  const actual = await importOriginal()
  return {
    ...actual,
    saveBaCharacterDraft: vi.fn(),
  }
})

import { finishBaCharacterForGame } from "./ba-game-handoff"
import { rememberBaCharacterDraft, saveBaCharacterDraft } from "./ba-character"

const saved = vi.mocked(saveBaCharacterDraft)
function characterManager() {
  return {
    getAvatarSelection: vi.fn(() => ({
      body: { id: "body-1", name: "Body" },
    })),
    getGLBBlob: vi.fn(async () => new Blob([new Uint8Array(24)])),
  }
}
beforeEach(() => {
  sessionStorage.clear()
  localStorage.clear()
  sessionStorage.setItem("ba_character_id", "existing-character")
  sessionStorage.setItem("ba_revision", "4")
  sessionStorage.setItem("ba_job", "Knight")
  sessionStorage.setItem("ba_gender", "male")
  saved.mockReset()
})

describe("Black Anvil Done -> native launcher handoff", () => {
  it("saves the model once and advances the selected revision after success", async () => {
    saved.mockResolvedValue({ ok: true, revision: 5, contentHash: "a".repeat(64) })
    const creator = characterManager()
    const { character } = await finishBaCharacterForGame(creator)
    expect(creator.getGLBBlob).toHaveBeenCalledTimes(1)
    expect(saved).toHaveBeenCalledTimes(1)
    expect(saved).toHaveBeenCalledWith(
      expect.objectContaining({
        schemaVersion: 1,
        revision: 5,
        characterId: "existing-character",
        startingJob: "Knight",
      }),
      expect.any(Blob),
    )
    expect(character.revision).toBe(5)
    expect(sessionStorage.getItem("ba_revision")).toBe("5")
    expect(sessionStorage.getItem("ba_character_id")).toBe("existing-character")
  })

  it("does not advance identity or revision if the native save fails", async () => {
    saved.mockRejectedValue(new Error("GLB rejected"))
    await expect(finishBaCharacterForGame(characterManager()))
      .rejects.toThrow("GLB rejected")
    expect(sessionStorage.getItem("ba_revision")).toBe("4")
    expect(sessionStorage.getItem("ba_character_id")).toBe("existing-character")
  })

  it("never sends a missing or empty avatar for native save", async () => {
    const creator = characterManager()
    creator.getGLBBlob.mockResolvedValue(new Blob([]))
    await expect(finishBaCharacterForGame(creator))
      .rejects.toThrow("model could not be exported")
    expect(saved).not.toHaveBeenCalled()
    expect(sessionStorage.getItem("ba_revision")).toBe("4")
  })

  it("restores identity, revision, job and name from the selected launcher draft", () => {
    rememberBaCharacterDraft({
      schemaVersion: 1,
      characterId: "persisted-character",
      revision: 21,
      name: "Alder",
      startingJob: "Rogue",
      manifestId: "anata/female",
    })
    expect(sessionStorage.getItem("ba_character_id")).toBe("persisted-character")
    expect(sessionStorage.getItem("ba_revision")).toBe("21")
    expect(sessionStorage.getItem("ba_job")).toBe("Rogue")
    expect(sessionStorage.getItem("ba_gender")).toBe("female")
    expect(sessionStorage.getItem("ba_name")).toBe("Alder")
  })

  it("does not select a revision that the launcher did not confirm", async () => {
    saved.mockResolvedValue({ ok: true, revision: 3 })
    await expect(finishBaCharacterForGame(characterManager()))
      .rejects.toThrow("different character revision")
    expect(sessionStorage.getItem("ba_revision")).toBe("4")
  })
})
