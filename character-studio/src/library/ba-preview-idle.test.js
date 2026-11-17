import { describe, expect, it } from "vitest"
import { Bone, Group } from "three"
import { BaIdlePreview } from "./ba-preview-idle"

function rig() {
  const scene = new Group()
  const torso = new Bone()
  torso.name = "spine"
  const left = new Bone()
  left.name = "leftUpperArm"
  const right = new Bone()
  right.name = "rightUpperArm"
  torso.add(left, right)
  scene.add(torso)
  return { scene, left, right }
}

describe("Black Anvil Character Studio idle preview", () => {
  it("poses both arms down at the first idle frame, rather than T-pose", () => {
    const { scene, left, right } = rig()
    const preview = new BaIdlePreview()
    preview.update({ Body: { vrm: { scene } } }, 1 / 60)
    expect(preview.active.size).toBe(1)
    expect(Math.abs(left.quaternion.z)).toBeGreaterThan(0.2)
    expect(Math.abs(right.quaternion.z)).toBeGreaterThan(0.2)
    preview.dispose()
    // The editor may serialize the avatar immediately after Done. Neutral
    // bind-pose values must be restored so exported clips do not double-pose.
    expect(left.quaternion.z).toBeCloseTo(0, 5)
    expect(right.quaternion.z).toBeCloseTo(0, 5)
  })

  it("does not create duplicate mixers on rerender and cleans up when a body changes", () => {
    const preview = new BaIdlePreview()
    const a = rig()
    const b = rig()
    preview.update({ Body: { vrm: { scene: a.scene } } }, 0.016)
    preview.update({ Body: { vrm: { scene: a.scene } } }, 0.016)
    expect(preview.active.size).toBe(1)
    preview.update({ Body: { vrm: { scene: b.scene } } }, 0.016)
    expect(preview.active.size).toBe(1)
    expect(preview.active.has(b.scene)).toBe(true)
    preview.dispose()
    expect(preview.active.size).toBe(0)
  })

  it("ignores static outfit meshes and missing avatar data", () => {
    const preview = new BaIdlePreview()
    preview.update({}, 0.016)
    preview.update({ Hat: { vrm: { scene: new Group() } } }, 0.016)
    expect(preview.active.size).toBe(0)
  })
})
