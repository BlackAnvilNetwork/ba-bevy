#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

//! Studio-host prototype IPC. Validates BaCharacterV1 + GLB before writing
//! character-scoped files under app data, then updates the Bevy `active.*` pointer
//! for local AssetServer loads (dev only — not an installed-release store).

use std::fs;
use std::path::{Path, PathBuf};

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::Deserialize;

const MAX_GLB: usize = 32 * 1024 * 1024;
const JOBS: &[&str] = &[
    "TimeMage", "Knight", "Rogue", "Cleric", "Sorcerer", "Fighter", "BattleMage", "Druid",
];

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

fn exports_dir() -> PathBuf {
    repo_root().join("assets").join("character").join("exports")
}

fn app_characters_dir() -> PathBuf {
    dirs_next::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("BlackAnvil")
        .join("characters")
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeAsset {
    format: String,
    #[serde(default)]
    relative_path: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BaCharacterV1 {
    schema_version: u32,
    revision: u64,
    character_id: String,
    name: String,
    starting_job: String,
    pack: String,
    manifest_id: String,
    catalog_version: String,
    runtime: RuntimeAsset,
}

fn validate_character(json: &str) -> Result<BaCharacterV1, String> {
    let c: BaCharacterV1 =
        serde_json::from_str(json).map_err(|e| format!("invalid BaCharacterV1: {e}"))?;
    if c.schema_version != 1 {
        return Err("schemaVersion must be 1".into());
    }
    if c.revision < 1 {
        return Err("revision must be >= 1".into());
    }
    if c.character_id.trim().is_empty() {
        return Err("characterId required".into());
    }
    if c.character_id.contains('/') || c.character_id.contains('\\') || c.character_id.contains("..")
    {
        return Err("characterId path-like".into());
    }
    if c.name.is_empty() || c.name.len() > 24 {
        return Err("name length".into());
    }
    if !JOBS.contains(&c.starting_job.as_str()) {
        return Err("startingJob".into());
    }
    if c.pack != "anata" {
        return Err("pack".into());
    }
    if c.manifest_id != "anata/male" && c.manifest_id != "anata/female" {
        return Err("manifestId".into());
    }
    if c.catalog_version.is_empty() {
        return Err("catalogVersion".into());
    }
    if c.runtime.format != "glb" {
        return Err("runtime.format".into());
    }
    let _ = c.runtime.relative_path; // ignored — server owns path
    Ok(c)
}

fn assert_glb(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() < 12 {
        return Err("glb too small".into());
    }
    if bytes.len() > MAX_GLB {
        return Err("glb too large".into());
    }
    if &bytes[0..4] != b"glTF" {
        return Err("glb magic must be glTF".into());
    }
    Ok(())
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    fs::rename(&tmp, path).map_err(|e| e.to_string())?;
    Ok(())
}

/// Persist BaCharacterV1 JSON + GLB (validated) for Bevy.
#[tauri::command]
fn save_draft(character_json: String, glb_base64: String) -> Result<serde_json::Value, String> {
    let c = validate_character(&character_json)?;
    let glb = B64
        .decode(glb_base64.as_bytes())
        .map_err(|e| format!("invalid glb base64: {e}"))?;
    assert_glb(&glb)?;

    // Force authoritative runtime path in stored JSON.
    let mut value: serde_json::Value =
        serde_json::from_str(&character_json).map_err(|e| e.to_string())?;
    if let Some(runtime) = value.get_mut("runtime") {
        runtime["format"] = serde_json::json!("glb");
        runtime["relativePath"] = serde_json::json!("character/exports/active.glb");
    }
    let normalized = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;

    let scoped = app_characters_dir().join(&c.character_id);
    fs::create_dir_all(&scoped).map_err(|e| e.to_string())?;
    atomic_write(&scoped.join("character.json"), normalized.as_bytes())?;
    atomic_write(&scoped.join("character.glb"), &glb)?;

    // Dev Bevy pointer (repo exports) — prototype only.
    let exports = exports_dir();
    fs::create_dir_all(&exports).map_err(|e| e.to_string())?;
    atomic_write(&exports.join("active.json"), normalized.as_bytes())?;
    atomic_write(&exports.join("active.glb"), &glb)?;

    Ok(serde_json::json!({
        "ok": true,
        "prototype": true,
        "json": "assets/character/exports/active.json",
        "glb": "assets/character/exports/active.glb",
        "scoped": scoped.display().to_string(),
        "bytes": glb.len(),
    }))
}

#[tauri::command]
fn get_draft() -> Result<Option<String>, String> {
    let path = exports_dir().join("active.json");
    if !path.is_file() {
        return Ok(None);
    }
    fs::read_to_string(path).map(Some).map_err(|e| e.to_string())
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![save_draft, get_draft])
        .run(tauri::generate_context!())
        .expect("error while running Black Anvil launcher");
}
