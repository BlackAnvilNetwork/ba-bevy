import { describe, expect, it } from "vitest"
import { Bone, Group } from "three"
import { buildBaAvatarAnimations } from "./ba-procedural-animation"

function rig() {
  const root = new Group()
  const hips = new Bone()
  hips.name = "hips"
  const spine = new Bone()
  spine.name = "spine"
  const left = new Bone()
  left.name = "leftUpperLeg"
  const right = new Bone()
  right.name = "rightUpperLeg"
  root.add(hips)
  hips.add(spine, left, right)
  return root
}

describe("Black Anvil procedural export rig", () => {
  it("emits stable Idle and Walk clips targeting only real bones", () => {
    const model = rig()
    const clips = buildBaAvatarAnimations(model)
    expect(clips.map((clip) => clip.name)).toEqual(["BA_Idle", "BA_Walk"])
    expect(clips[0].tracks).toHaveLength(1)
    expect(clips[1].tracks.length).toBeGreaterThanOrEqual(2)
    const boneNames = new Set()
    model.traverse((node) => { if (node.isBone) boneNames.add(node.name) })
    for (const clip of clips) {
      for (const track of clip.tracks) {
        expect(boneNames.has(track.name.split(".")[0])).toBe(true)
        expect(track.values.every((value) => Number.isFinite(value))).toBe(true)
        expect(track.times[0]).toBe(0)
        expect(track.times[track.times.length - 1]).toBeCloseTo(clip.duration, 5)
        // The clips have a matching first/last pose for seamless loops.
        const stride = 4
        const first = track.values.slice(0, stride)
        const last = track.values.slice(-stride)
        first.forEach((value, i) => expect(last[i]).toBeCloseTo(value, 5))
      }
    }
  })

  it("rejects unrigged models instead of silently producing static avatars", () => {
    expect(() => buildBaAvatarAnimations(new Group())).toThrow(/skeleton/)
  })

  it("always produces an animation even for a minimal humanoid root", () => {
    const root = new Group()
    const bone = new Bone()
    bone.name = "hips"
    root.add(bone)
    const clips = buildBaAvatarAnimations(root)
    expect(clips[1].tracks).toHaveLength(1)
  })
})
