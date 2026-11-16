#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use std::fs;
use std::path::PathBuf;

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};

fn repo_root() -> PathBuf {
    // launcher/src-tauri → repo root
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

fn exports_dir() -> PathBuf {
    repo_root().join("assets").join("character").join("exports")
}

/// Persist BaCharacterV1 JSON + GLB for Bevy (`assets/character/exports/active.*`).
#[tauri::command]
fn save_draft(character_json: String, glb_base64: String) -> Result<serde_json::Value, String> {
    let _: serde_json::Value =
        serde_json::from_str(&character_json).map_err(|e| format!("invalid character JSON: {e}"))?;
    let glb = B64
        .decode(glb_base64.as_bytes())
        .map_err(|e| format!("invalid glb base64: {e}"))?;
    if glb.is_empty() {
        return Err("glb is empty".into());
    }

    let dir = exports_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let json_path = dir.join("active.json");
    let glb_path = dir.join("active.glb");
    fs::write(&json_path, character_json.as_bytes()).map_err(|e| e.to_string())?;
    fs::write(&glb_path, &glb).map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "ok": true,
        "json": "assets/character/exports/active.json",
        "glb": "assets/character/exports/active.glb",
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
