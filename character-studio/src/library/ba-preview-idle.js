/**
 * Black Anvil editor default idle. It is the same rig-bound BA_Idle clip that
 * gets exported with the character's GLB, not a static bind/T pose.
 *
 * Each equipped trait may have its own VRM scene and skeleton. Animate each
 * scene separately so hair and clothing on their own rigs stay registered,
 * while never touching the saved selection or game ownership.
 */
import { AnimationMixer, LoopRepeat } from "three"
import { buildBaAvatarAnimations } from "./ba-procedural-animation"

export class BaIdlePreview {
  constructor() {
    this.active = new Map()
  }

  update(avatar, seconds) {
    const present = new Set()
    for (const trait of Object.values(avatar || {})) {
      const scene = trait?.vrm?.scene
      if (!scene || present.has(scene)) continue
      present.add(scene)
      if (!this.active.has(scene)) {
        try {
          const [idle] = buildBaAvatarAnimations(scene)
          const mixer = new AnimationMixer(scene)
          const action = mixer.clipAction(idle)
          action.setLoop(LoopRepeat, Infinity)
          action.play()
          this.active.set(scene, { mixer, action })
        } catch (error) {
          // E.g. decals/textures have no bones. Do not interrupt creation.
          console.debug("Skipping static character trait for idle preview", error)
        }
      }
    }
    for (const [scene, value] of this.active) {
      if (!present.has(scene)) {
        value.action.stop()
        value.mixer.stopAllAction()
        value.mixer.uncacheRoot(scene)
        this.active.delete(scene)
      } else {
        value.mixer.update(Math.min(0.1, Math.max(0, seconds || 0)))
      }
    }
  }

  dispose() {
    for (const [scene, value] of this.active) {
      value.action.stop()
      value.mixer.stopAllAction()
      value.mixer.uncacheRoot(scene)
    }
    this.active.clear()
  }
}
