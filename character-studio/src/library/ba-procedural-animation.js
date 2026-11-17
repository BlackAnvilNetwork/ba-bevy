/**
 * Procedural animation clips for the final exported Black Anvil GLB.
 *
 * Character Studio's own FBX preview clips are tied to source VRM controllers
 * and are not automatically present in the optimized GLB. These two original
 * bone-space clips give every correctly rigged exported avatar a deterministic
 * Idle and Walk baseline that Bevy can use immediately without separate
 * retargeting or external animation files.
 */
import { AnimationClip, Quaternion, QuaternionKeyframeTrack, Vector3 } from "three"

const X_AXIS = new Vector3(1, 0, 0)
const Z_AXIS = new Vector3(0, 0, 1)

function normalizedName(name) {
  return String(name || "").toLowerCase().replace(/[^a-z0-9]/g, "")
}

export function buildBaAvatarAnimations(model) {
  const bones = []
  model.traverse((child) => {
    if (child?.isBone && child.name) bones.push(child)
  })
  if (!bones.length) {
    throw new Error("Black Anvil avatar export requires an animated skeleton")
  }

  const findBone = (names) => bones.find((b) => {
    const key = normalizedName(b.name)
    return names.some((name) => key.includes(name))
  })

  const torso = findBone(["spine", "chest", "upperbody", "hips"]) || bones[0]
  const leftLeg = findBone(["leftupperleg", "leftupleg", "leftthigh"])
  const rightLeg = findBone(["rightupperleg", "rightupleg", "rightthigh"])
  const leftArm = findBone(["leftupperarm", "leftarm"])
  const rightArm = findBone(["rightupperarm", "rightarm"])

  // GLTFExporter target lookups use exact Object3D bone names. Never export
  // animations referencing a bone omitted by the optimized model.
  // T-pose arms must be brought down to a comfortable, readable idle.
  // These poses are stored inside the exported GLB and also used by the
  // Character Studio preview. They are independent of an FBX URL or network.
  function track(bone, duration, phases, amplitude, swingAxis = X_AXIS, restAngle = 0) {
    if (!bone) return null
    const initial = bone.quaternion.clone()
    const rest = new Quaternion().setFromAxisAngle(Z_AXIS, restAngle)
    const times = phases.map((_, i) => (duration * i) / (phases.length - 1))
    const values = []
    for (const phase of phases) {
      const motion = new Quaternion().setFromAxisAngle(swingAxis, amplitude * phase)
      const current = initial.clone().multiply(rest).multiply(motion).normalize()
      values.push(current.x, current.y, current.z, current.w)
    }
    return new QuaternionKeyframeTrack(`${bone.name}.quaternion`, times, values)
  }

  const breathing = [0, 0.7, 1, 0.7, 0, -0.6, -1, -0.6, 0]
  const gait = [0, 1, 0, -1, 0]
  const invert = gait.map((x) => -x)
  const still = [0, 0, 0, 0, 0, 0, 0, 0, 0]
  // VRM humanoid arm axes: left arm points to the model's left in bind pose,
  // right arm points to its right. A ~60° turn down removes the T-pose without
  // collapsing the arms into the torso.
  const leftRest = -1.0
  const rightRest = 1.0
  const idle = new AnimationClip("BA_Idle", 2.4, [
    track(torso, 2.4, breathing, 0.025, Z_AXIS),
    track(leftArm, 2.4, still, 0, Z_AXIS, leftRest),
    track(rightArm, 2.4, still, 0, Z_AXIS, rightRest),
  ].filter(Boolean))

  const walkTracks = [
    track(leftLeg, 0.8, gait, 0.30),
    track(rightLeg, 0.8, invert, 0.30),
    track(leftArm, 0.8, invert, 0.18, X_AXIS, leftRest),
    track(rightArm, 0.8, gait, 0.18, X_AXIS, rightRest),
  ].filter(Boolean)
  // Some imported rigs only expose the torso. Preserve a stable animation
  // rather than binding tracks to nonexistent bones.
  if (!walkTracks.length) {
    walkTracks.push(track(torso, 0.8, gait, 0.045, Z_AXIS))
  }
  const walking = new AnimationClip("BA_Walk", 0.8, walkTracks)
  if (!idle.tracks.length || !walking.tracks.length) {
    throw new Error("Black Anvil could not build animation tracks for this rig")
  }
  return [idle, walking]
}
