/**
 * Black Anvil's explicit editor entry contract:
 * a manifest alone is not a character. A loaded VRM must be present.
 */
export function hasLoadedAvatar(characterManager) {
  return Boolean(
    characterManager?.manifestDataManager?.hasExistingManifest?.() &&
    Object.values(characterManager.avatar || {}).some((trait) => Boolean(trait?.vrm)),
  )
}

/**
 * Runs only when the player explicitly chooses Create Character.
 * Returns after the character is actually placed into the Three.js scene.
 * The caller owns view navigation and error presentation.
 */
export async function loadCreatorAvatar(characterManager, selectedClass, manifestId, requireAvatar = false) {
  if (!characterManager || !selectedClass?.manifest) {
    throw new Error("Character manifest is not available.")
  }
  await characterManager.loadManifest(selectedClass.manifest, manifestId)
  await Promise.all(
    (selectedClass.manifestAppend || []).map((append) =>
      characterManager.loadManifest(append.manifest, append.name),
    ),
  )
  await characterManager.loadInitialTraits()
  if (requireAvatar && !hasLoadedAvatar(characterManager)) {
    throw new Error("The character model did not finish loading.")
  }
}
