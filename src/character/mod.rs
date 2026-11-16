//! Canonical Black Anvil character draft (BaCharacterV1).
//!
//! Written by Character Studio (BA mode) via the local save bridge;
//! read by the forge menu and Bevy avatar spawn.

mod avatar;

pub use avatar::{
    attach_player_avatar, despawn_stale_avatars, HasPlayerAvatar, PlayerAvatarMesh,
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
    "Chest", "Legs", "Shoes", "Hands", "Waist", "Neck", "Weapon",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TraitRef {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub blendshapes: BTreeMap<String, f32>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub colors: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeAsset {
    pub format: String,
    pub relative_path: String,
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

impl BaCharacterV1 {
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
    exports_dir().join("active.glb")
}

pub fn load_active_draft() -> Option<BaCharacterV1> {
    let path = active_json_path();
    load_draft_from_path(&path)
}

pub fn load_draft_from_path(path: &Path) -> Option<BaCharacterV1> {
    let raw = fs::read_to_string(path).ok()?;
    match serde_json::from_str::<BaCharacterV1>(&raw) {
        Ok(draft) if draft.schema_version == SCHEMA_VERSION => Some(draft),
        Ok(draft) => {
            warn!(
                "BaCharacter schemaVersion {} unsupported (want {})",
                draft.schema_version, SCHEMA_VERSION
            );
            None
        }
        Err(err) => {
            warn!("Failed to parse {}: {err}", path.display());
            None
        }
    }
}

pub fn load_character_profile(mut commands: Commands) {
    let draft = load_active_draft();
    if let Some(ref d) = draft {
        info!(
            "Loaded BaCharacterV1 '{}' job={} manifest={} glb={}",
            d.name,
            d.starting_job,
            d.manifest_id,
            d.glb_asset_path()
        );
    } else {
        info!("No active BaCharacterV1 at {}", active_json_path().display());
    }
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
