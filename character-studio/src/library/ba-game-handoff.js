import { getAtlasSize } from "./utils"
import { local } from "./store"
import {
  buildBaCharacterV1,
  rememberBaCharacterDraft,
  saveBaCharacterDraft,
} from "./ba-character"

/** Fixed in-game export configuration: never ask players for file/download options. */
export function getBaGameExportOptions() {
  const currentOption = local["mergeOptions_sel_option"] || 0
  const createTextureAtlas = local["mergeOptions_create_atlas"] == null
    ? true
    : local["mergeOptions_create_atlas"]

  return {
    createTextureAtlas,
    mToonAtlasSize: getAtlasSize(local["mergeOptions_atlas_mtoon_size"] || 6),
    mToonAtlasSizeTransp: getAtlasSize(local["mergeOptions_atlas_mtoon_transp_size"] || 6),
    stdAtlasSize: getAtlasSize(local["mergeOptions_atlas_std_size"] || 6),
    stdAtlasSizeTransp: getAtlasSize(local["mergeOptions_atlas_std_transp_size"] || 6),
    ktxCompression: local["merge_options_ktx_compression"],
    exportStdAtlas: currentOption === 0 || currentOption == 2,
    exportMtoonAtlas: currentOption === 1 || currentOption == 2,
    twoSidedMaterial: local["mergeOptions_two_sided_mat"] || false,
    optimized: true,
  }
}

/**
 * Finalize the current customized character for the game.
 *
 * Explicit contract: selected traits -> GLB -> native save_draft ->
 * committed local revision -> launcher Home. Launcher and Bevy read that
 * same immutable revision; no downloading through a file picker.
 * Multiplayer sync and game ticket issuance remain part of Play.
 */
export async function finishBaCharacterForGame(characterManager) {
  if (!characterManager?.getAvatarSelection || !characterManager?.getGLBBlob) {
    throw new Error("Character creator is not ready")
  }

  const character = buildBaCharacterV1(characterManager)
  const glbBlob = await characterManager.getGLBBlob(getBaGameExportOptions())
  if (!(glbBlob instanceof Blob) || glbBlob.size < 20) {
    throw new Error("Character model could not be exported. No character was saved.")
  }

  const result = await saveBaCharacterDraft(character, glbBlob)
  if (!result || result.ok === false) {
    throw new Error("The launcher did not confirm the saved character.")
  }
  if (result.revision != null && result.revision !== character.revision) {
    throw new Error("The launcher reported a different character revision.")
  }

  rememberBaCharacterDraft(character)
  return { character, result }
}
