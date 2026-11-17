import { describe, expect, it, vi } from "vitest"
import { hasLoadedAvatar, loadCreatorAvatar } from "./ba-creator"

const pack = {
  manifest: "/character-assets/neurohacker/manifest.json",
  manifestAppend: [{ manifest: "/extra.json", name: "extra" }],
}

function fixture() {
  const order = []
  const manager = {
    avatar: {},
    manifestDataManager: { hasExistingManifest: () => order.includes("manifest") },
    loadManifest: vi.fn(async (url) => {
      order.push(url === pack.manifest ? "manifest" : "append")
    }),
    loadInitialTraits: vi.fn(async () => {
      order.push("traits")
      manager.avatar.body = { vrm: { scene: {} } }
    }),
  }
  return { manager, order }
}

describe("Black Anvil native launcher creator entry", () => {
  it("does not treat a manifest without an avatar as ready", () => {
    const { manager, order } = fixture()
    order.push("manifest")
    expect(hasLoadedAvatar(manager)).toBe(false)
    manager.avatar.body = { vrm: { scene: {} } }
    expect(hasLoadedAvatar(manager)).toBe(true)
  })

  it("loads the selected pack and initial 3D avatar before entering editor", async () => {
    const { manager, order } = fixture()
    await loadCreatorAvatar(manager, pack, "ba-male", true)
    expect(order).toEqual(["manifest", "append", "traits"])
    expect(manager.loadManifest).toHaveBeenCalledWith(pack.manifest, "ba-male")
    expect(hasLoadedAvatar(manager)).toBe(true)
  })

  it("waits for asynchronous VRM loading rather than opening an empty editor", async () => {
    const { manager } = fixture()
    let finish
    manager.loadInitialTraits = vi.fn(() => new Promise((resolve) => {
      finish = () => {
        manager.avatar.body = { vrm: { scene: {} } }
        resolve()
      }
    }))
    let resolved = false
    const pending = loadCreatorAvatar(manager, pack, "ba-male", true).then(() => { resolved = true })
    await vi.waitFor(() => expect(finish).toBeTypeOf("function"))
    expect(resolved).toBe(false)
    finish()
    await pending
    expect(resolved).toBe(true)
  })

  it("reports trait load failures instead of hanging", async () => {
    const { manager } = fixture()
    manager.loadInitialTraits = vi.fn().mockRejectedValue(new Error("Missing VRM"))
    await expect(loadCreatorAvatar(manager, pack, "ba-male", true))
      .rejects.toThrow("Missing VRM")
  })

  it("rejects empty scene even if a manifest and trait loader completed", async () => {
    const { manager } = fixture()
    manager.loadInitialTraits = vi.fn().mockResolvedValue(undefined)
    await expect(loadCreatorAvatar(manager, pack, "ba-male", true))
      .rejects.toThrow("model did not finish loading")
  })
})
