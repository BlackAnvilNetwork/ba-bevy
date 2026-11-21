//! Canonical Black Anvil character draft (BaCharacterV1).
//!
//! Written by Character Studio (BA mode) via the local save bridge;
//! read by the forge menu and Bevy avatar spawn.

mod avatar;
mod animation;
mod equip;
mod materials;
pub use animation::{LocalEye, SwingPose, prepare_avatar_animation, bind_avatar_animators, update_avatar_animation, discover_avatar_bones, pose_humanoid_bones, attach_held_weapon, hide_held_weapon_hulls, stow_held_swords_while_resting, swing_overlay_label};
pub use equip::{cycle_equip_mode, EquipMode, EquipState};
#[allow(unused_imports)]
pub use equip::OffhandSlot;
pub use materials::matte_avatar_materials;

pub use avatar::{
    attach_player_avatar, resolve_pending_avatars, HasPlayerAvatar,
    PlayerAvatarMesh,
};

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::rpg::JobId;

pub const SCHEMA_VERSION: u32 = 1;
pub const ACTIVE_JSON: &str = "character/exports/active.json";
pub const ACTIVE_GLB: &str = "character/exports/active.glb";

const BODY_TRAIT_KEYS: &[&str] = &["Body", "Head", "Eyes", "Basic Colors"];
const EQUIP_TRAIT_KEYS: &[&str] = &[
    "Chest", "Legs", "Shoes", "Hands", "Waist", "Neck", "Weapon", "Offhand",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TraitRef {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    // The current Studio stores blendshape trait IDs, booleans and sometimes
    // numeric values. Do not reject valid edited avatars by assuming weights.
    // A later canonical editor schema should normalize these to typed controls.
    pub blendshapes: BTreeMap<String, serde_json::Value>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub colors: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeAsset {
    pub format: String,
    pub relative_path: String,
    #[serde(default)]
    pub content_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Resource)]
#[serde(rename_all = "camelCase")]
pub struct BaCharacterV1 {
    pub schema_version: u32,
    pub revision: u64,
    pub character_id: String,
    pub name: String,
    pub starting_job: String,
    pub pack: String,
    pub manifest_id: String,
    pub catalog_version: String,
    #[serde(default)]
    pub body_traits: BTreeMap<String, TraitRef>,
    #[serde(default)]
    pub starter_equipment: BTreeMap<String, TraitRef>,
    pub runtime: RuntimeAsset,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HeldWeapon {
    Unarmed,
    OneHand,
    DualWield,
    GreatWeapon,
    PoleArm,
}

impl HeldWeapon {
    pub fn is_two_handed(self) -> bool {
        matches!(self, Self::GreatWeapon | Self::PoleArm)
    }
}

impl BaCharacterV1 {
    fn equip_id(&self, slot: &str) -> Option<&str> {
        self.starter_equipment.iter().find_map(|(name, item)| {
            (name.eq_ignore_ascii_case(slot) && !item.id.is_empty()).then_some(item.id.as_str())
        })
    }

    pub fn held_weapon(&self) -> HeldWeapon {
        let weapon = self.equip_id("weapon");
        let offhand = self
            .equip_id("offhand")
            .or_else(|| self.equip_id("off_hand"));
        let two_hand = |id: &str| -> Option<HeldWeapon> {
            let id = id.to_ascii_lowercase();
            if id.contains("spear")
                || id.contains("staff")
                || id.contains("pole")
                || id.contains("pike")
                || id.contains("halberd")
                || id.contains("lance")
            {
                Some(HeldWeapon::PoleArm)
            } else if id.contains("2h")
                || id.contains("great")
                || id.contains("zwei")
                || id.contains("greataxe")
            {
                Some(HeldWeapon::GreatWeapon)
            } else {
                None
            }
        };
        let dual = |id: &str| {
            let id = id.to_ascii_lowercase();
            id.contains("dual") || id.contains("_dw") || id.ends_with("dw")
        };
        match (weapon, offhand) {
            (None, None) => HeldWeapon::Unarmed,
            (Some(id), _) => {
                if let Some(kind) = two_hand(id) {
                    kind
                } else if dual(id) {
                    HeldWeapon::DualWield
                } else if offhand.is_some() {
                    HeldWeapon::DualWield
                } else {
                    HeldWeapon::OneHand
                }
            }
            (None, Some(_)) => HeldWeapon::DualWield,
        }
    }

    pub fn job_id(&self) -> JobId {
        match self.starting_job.as_str() {
            "Knight" => JobId::Knight,
            "Rogue" => JobId::Rogue,
            "Cleric" => JobId::Cleric,
            "Sorcerer" => JobId::Sorcerer,
            "Fighter" => JobId::Fighter,
            "BattleMage" => JobId::BattleMage,
            "Druid" => JobId::Druid,
            _ => JobId::TimeMage,
        }
    }

    pub fn is_female(&self) -> bool {
        self.manifest_id.contains("female")
    }

    pub fn glb_asset_path(&self) -> &str {
        if self.runtime.relative_path.is_empty() {
            ACTIVE_GLB
        } else {
            self.runtime.relative_path.as_str()
        }
    }
}

/// In-memory profile loaded at startup (or empty if no draft).
#[derive(Resource, Default, Debug, Clone)]
pub struct CharacterProfile {
    pub draft: Option<BaCharacterV1>,
}

pub fn exports_dir() -> PathBuf {
    let from_manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join("character")
        .join("exports");
    if from_manifest.parent().is_some_and(|p| p.is_dir()) || from_manifest.is_dir() {
        from_manifest
    } else {
        PathBuf::from("assets").join("character").join("exports")
    }
}

pub fn active_json_path() -> PathBuf {
    exports_dir().join("active.json")
}

pub fn active_glb_path() -> PathBuf {
    if let Some(path) = std::env::var_os("BA_CHARACTER_GLB_PATH") {
        return PathBuf::from(path);
    }
    exports_dir().join("active.glb")
}

pub fn load_active_draft() -> Option<BaCharacterV1> {
    if let Some(path) = std::env::var_os("BA_CHARACTER_DRAFT_PATH") {
        return load_draft_from_path(&PathBuf::from(path));
    }
    load_draft_from_path(&active_json_path())
}

/// On launcher handoff, only the expected launcher-managed content-addressed
/// revision is a valid asset path. Ignore attacker-edited paths in local JSON.
fn valid_character_asset(draft: &BaCharacterV1) -> bool {
    if draft.character_id.is_empty() || draft.character_id.len() > 64
        || !draft.character_id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        || draft.revision == 0 || draft.runtime.format != "glb"
    {
        return false;
    }
    if std::env::var_os("BA_CHARACTER_ROOT").is_some() {
        let expected = format!(
            "character://{}/revisions/r{}/character.glb",
            draft.character_id, draft.revision
        );
        let valid_hash = draft.runtime.content_hash.as_ref().is_some_and(|hash| {
            hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit())
        });
        return draft.runtime.relative_path == expected && valid_hash;
    }
    true // standalone developer forge keeps its historical local export path
}

pub fn load_draft_from_path(path: &Path) -> Option<BaCharacterV1> {
    let raw = fs::read_to_string(path).ok()?;
    match serde_json::from_str::<BaCharacterV1>(&raw) {
        Ok(draft) if draft.schema_version == SCHEMA_VERSION && valid_character_asset(&draft) => Some(draft),
        Ok(draft) => {
            warn!(
                "Rejected BaCharacter revision {}: schemaVersion {}, asset URI {}",
                draft.revision, draft.schema_version, draft.runtime.relative_path
            );
            None
        }
        Err(err) => {
            warn!("Failed to parse {}: {err}", path.display());
            None
        }
    }
}

pub fn load_character_profile(mut commands: Commands, mut flow: ResMut<crate::forge_menu::GameFlow>) {
    let draft = load_active_draft();
    // Only the explicit native Play command bypasses the developer forge, and
    // only after a locally saved character can be loaded. Production game
    // admission will additionally require a verified Discord session ticket.
    if std::env::var("BA_LAUNCH_MODE").as_deref() == Ok("game") && draft.is_some() {
        *flow = crate::forge_menu::GameFlow::InYard;
        info!("Launcher handoff: saved character selected, entering yard directly");
    }
    if let Some(ref d) = draft {
        info!(
            "Loaded BaCharacterV1 '{}' job={} manifest={} glb={}",
            d.name,
            d.starting_job,
            d.manifest_id,
            d.glb_asset_path()
        );
    } else {
        info!("No valid selected BaCharacterV1 (standalone path: {})", active_json_path().display());
    }
    let held = draft
        .as_ref()
        .map(BaCharacterV1::held_weapon)
        .unwrap_or(HeldWeapon::Unarmed);
    commands.insert_resource(equip::EquipState::from_loadout(held));
    commands.insert_resource(CharacterProfile { draft });
}

/// Split Studio `getAvatarSelection()`-style maps into body vs equipment.
#[allow(dead_code)]
pub fn partition_trait_key(key: &str) -> TraitKind {
    if EQUIP_TRAIT_KEYS.iter().any(|k| k.eq_ignore_ascii_case(key)) {
        TraitKind::Equipment
    } else if BODY_TRAIT_KEYS.iter().any(|k| k.eq_ignore_ascii_case(key)) {
        TraitKind::Body
    } else {
        // Unknown mesh traits default to body so we do not lose data.
        TraitKind::Body
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TraitKind {
    Body,
    Equipment,
}

#[cfg(test)]
mod character_schema_tests {
    use super::*;

    #[test]
    fn studio_traits_round_trip_with_mixed_blendshape_selection_types() {
        let json = r##"{
            "schemaVersion": 1, "revision": 2, "characterId": "test-id",
            "name": "Chrona", "startingJob": "TimeMage", "pack": "anata",
            "manifestId": "anata/male", "catalogVersion": "anata-2026-11",
            "bodyTraits": {
                "Head": {
                    "id": "head_1", "name": "Head",
                    "blendshapes": {"smile": "wide-smile", "squint": true, "strength": 0.75},
                    "colors": {"c0": "#dbbea7"}
                }
            },
            "starterEquipment": {},
            "runtime": {
                "format": "glb",
                "relativePath": "character://test-id/revisions/r2/character.glb",
                "contentHash": "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
            }
        }"##;
        let parsed: BaCharacterV1 = serde_json::from_str(json).unwrap();
        let head = &parsed.body_traits["Head"];
        assert_eq!(head.blendshapes["smile"], "wide-smile");
        assert_eq!(head.blendshapes["squint"], true);
        assert_eq!(head.blendshapes["strength"], 0.75);
        let roundtrip = serde_json::to_string(&parsed).unwrap();
        let restored: BaCharacterV1 = serde_json::from_str(&roundtrip).unwrap();
        assert_eq!(parsed, restored);
        assert_eq!(parsed.held_weapon(), HeldWeapon::Unarmed);
    }

    #[test]
    fn weapon_slot_selects_one_hand_or_two_hand() {
        let mut armed = BaCharacterV1 {
            schema_version: 1,
            revision: 2,
            character_id: "test-id".into(),
            name: "Chrona".into(),
            starting_job: "TimeMage".into(),
            pack: "anata".into(),
            manifest_id: "anata/male".into(),
            catalog_version: "anata-2026-11".into(),
            body_traits: BTreeMap::new(),
            starter_equipment: BTreeMap::new(),
            runtime: RuntimeAsset {
                format: "glb".into(),
                relative_path: "character://test-id/revisions/r2/character.glb".into(),
                content_hash: None,
            },
        };
        assert_eq!(armed.held_weapon(), HeldWeapon::Unarmed);
        armed.starter_equipment.insert(
            "weapon".into(),
            TraitRef {
                id: "short_sword".into(),
                name: Some("Short Sword".into()),
                blendshapes: BTreeMap::new(),
                colors: BTreeMap::new(),
            },
        );
        assert_eq!(armed.held_weapon(), HeldWeapon::OneHand);
        armed.starter_equipment.insert(
            "offhand".into(),
            TraitRef {
                id: "short_sword".into(),
                name: Some("Short Sword".into()),
                blendshapes: BTreeMap::new(),
                colors: BTreeMap::new(),
            },
        );
        assert_eq!(armed.held_weapon(), HeldWeapon::DualWield);
        armed.starter_equipment.remove("offhand");
        armed.starter_equipment.get_mut("weapon").unwrap().id = "greatsword".into();
        assert_eq!(armed.held_weapon(), HeldWeapon::GreatWeapon);
        armed.starter_equipment.get_mut("weapon").unwrap().id = "spear".into();
        assert_eq!(armed.held_weapon(), HeldWeapon::PoleArm);
        armed.starter_equipment.get_mut("weapon").unwrap().id = "oak_staff".into();
        assert_eq!(armed.held_weapon(), HeldWeapon::PoleArm);
    }
}
