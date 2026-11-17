// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from "vitest"
import { Bone, Group } from "three"

// VRM exporters initialize WebGL KTX decoders as a module side effect.
// This GLB unit test does not exercise VRM/KTX; stub them so Node/jsdom
// never attempts to create a native WebGL compression context.
vi.mock("./VRMExporter", () => ({ default: class VRMExporter {} }))
vi.mock("./VRMExporterv0", () => ({ default: class VRMExporterv0 {} }))


const mocks = vi.hoisted(() => ({
  combine: vi.fn(async (model) => model),
  combineNoAtlas: vi.fn(async (model) => model),
  parse: vi.fn((_model, success) => success(new ArrayBuffer(24))),
}))

vi.mock("./merge-geometry", () => ({
  cloneSkeleton: vi.fn(),
  combine: mocks.combine,
  combineNoAtlas: mocks.combineNoAtlas,
}))
vi.mock("three/examples/jsm/exporters/GLTFExporter", () => ({
  GLTFExporter: class {
    parse(model, success, failure, options) {
      return mocks.parse(model, success, failure, options)
    }
  },
}))

import { getGLBBlobData } from "./download-utils"

function makeRig() {
  const model = new Group()
  const hips = new Bone()
  hips.name = "hips"
  model.add(hips)
  return model
}

beforeEach(() => {
  mocks.combine.mockReset().mockImplementation(async (model) => model)
  mocks.combineNoAtlas.mockReset().mockImplementation(async (model) => model)
  mocks.parse.mockReset().mockImplementation((_model, success) =>
    success(new ArrayBuffer(24))
  )
})

describe("Black Anvil optimized GLB export argument contract", () => {
  it("forwards loaded avatar traits and export options to atlas merging", async () => {
    const avatar = { body: { id: "body-01", vrm: { scene: {} } } }
    const options = { optimized: true, createTextureAtlas: true, stdAtlasSize: 512 }
    const result = await getGLBBlobData(makeRig(), avatar, options)

    expect(result).toBeInstanceOf(Blob)
    expect(result.size).toBe(24)
    expect(mocks.combine).toHaveBeenCalledOnce()
    expect(mocks.combine.mock.calls[0][1]).toBe(avatar)
    expect(mocks.combine.mock.calls[0][2]).toEqual(expect.objectContaining({
      optimized: true,
      createTextureAtlas: true,
      stdAtlasSize: 512,
      mergeAppliedMorphs: true,
    }))
    expect(options).not.toHaveProperty("mergeAppliedMorphs")
    expect(mocks.parse).toHaveBeenCalledOnce()
    expect(mocks.parse.mock.calls[0][3].animations.map((clip) => clip.name))
      .toEqual(["BA_Idle", "BA_Walk"])
  })

  it("forwards the same avatar to the no-atlas optimized export", async () => {
    const avatar = { body: { id: "body-02", vrm: { scene: {} } } }
    await getGLBBlobData(makeRig(), avatar, {
      optimized: true,
      createTextureAtlas: false,
    })
    expect(mocks.combine).not.toHaveBeenCalled()
    expect(mocks.combineNoAtlas).toHaveBeenCalledOnce()
    expect(mocks.combineNoAtlas.mock.calls[0][1]).toBe(avatar)
    expect(mocks.combineNoAtlas.mock.calls[0][2].mergeAppliedMorphs).toBe(true)
  })

  it("fails with an actionable error if avatar trait data is missing", async () => {
    await expect(getGLBBlobData(makeRig(), undefined, { optimized: true }))
      .rejects.toThrow("without loaded trait data")
    expect(mocks.combine).not.toHaveBeenCalled()
  })

  it("rejects exporter failures instead of leaving Done stuck saving", async () => {
    mocks.parse.mockImplementationOnce((_model, _success, failure) => {
      failure(new Error("GLTF exporter failed"))
    })
    await expect(
      getGLBBlobData(makeRig(), { body: {} }, { optimized: true })
    ).rejects.toThrow("GLTF exporter failed")
  })
})
