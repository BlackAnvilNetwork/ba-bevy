#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

//! Studio-host prototype IPC. Validates BaCharacterV1 + GLB before writing
//! character-scoped files under app data, then updates the Bevy `active.*` pointer
//! for local AssetServer loads (dev only — not an installed-release store).

use std::fs;
use std::process::{Child, Command};
use std::sync::Mutex;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::Deserialize;
use tauri::Manager;

// A native Play action must not spawn duplicate game processes on double clicks.
static RUNNING_GAME: Mutex<Option<Child>> = Mutex::new(None);

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

fn assert_safe_character_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > 64 {
        return Err("characterId length".into());
    }
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
    {
        return Err("characterId must be alphanumeric/[_-] only".into());
    }
    if id == "." || id == ".." {
        return Err("characterId reserved".into());
    }
    Ok(())
}

fn scoped_under(root: &Path, id: &str) -> Result<PathBuf, String> {
    assert_safe_character_id(id)?;
    let root = root
        .canonicalize()
        .unwrap_or_else(|_| root.to_path_buf());
    fs::create_dir_all(&root).map_err(|e| e.to_string())?;
    let candidate = root.join(id);
    let resolved = if candidate.exists() {
        candidate.canonicalize().map_err(|e| e.to_string())?
    } else {
        candidate
    };
    if !resolved.starts_with(&root) {
        return Err("characterId path escape blocked".into());
    }
    Ok(resolved)
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
    assert_safe_character_id(&c.character_id)?;
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
    let _ = c.runtime.relative_path;
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

/// Write JSON+GLB into a staging directory with a COMMIT marker, then promote.
fn transactional_character_write(
    scoped: &Path,
    json: &str,
    glb: &[u8],
    active_json: &Path,
    active_glb: &Path,
) -> Result<(), String> {
    fs::create_dir_all(scoped).map_err(|e| e.to_string())?;
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let staging = scoped.join(format!(".staging-{}-{}", std::process::id(), stamp));
    fs::create_dir_all(&staging).map_err(|e| e.to_string())?;
    fs::write(staging.join("character.json"), json.as_bytes()).map_err(|e| e.to_string())?;
    fs::write(staging.join("character.glb"), glb).map_err(|e| e.to_string())?;
    // COMMIT last — incomplete staging is discarded.
    fs::write(staging.join("COMMIT"), b"ok\n").map_err(|e| e.to_string())?;

    atomic_write(&scoped.join("character.json"), json.as_bytes())?;
    atomic_write(&scoped.join("character.glb"), glb)?;
    fs::write(scoped.join("COMMIT"), format!("{stamp}\n")).map_err(|e| e.to_string())?;

    if let Some(parent) = active_json.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    atomic_write(active_json, json.as_bytes())?;
    atomic_write(active_glb, glb)?;

    let _ = fs::remove_dir_all(&staging);
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

    let mut value: serde_json::Value =
        serde_json::from_str(&character_json).map_err(|e| e.to_string())?;
    if let Some(runtime) = value.get_mut("runtime") {
        runtime["format"] = serde_json::json!("glb");
        runtime["relativePath"] = serde_json::json!("character/exports/active.glb");
    }
    let normalized = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;

    let scoped = scoped_under(&app_characters_dir(), &c.character_id)?;
    let exports = exports_dir();
    transactional_character_write(
        &scoped,
        &normalized,
        &glb,
        &exports.join("active.json"),
        &exports.join("active.glb"),
    )?;

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


/// Launch a native Bevy game process without accepting an arbitrary executable path.
///
/// The development preview uses this checkout's compiled Bevy binary. A production
/// installer must bundle a signed executable and matching assets before this path
/// is eligible for release. Discord session-ticket handoff is a separate gate.
#[tauri::command]
fn launch_game(app: tauri::AppHandle, multiplayer: bool) -> Result<serde_json::Value, String> {
    let draft = get_draft()?.ok_or("Create and save a character before playing")?;
    validate_character(&draft)?;
    if !exports_dir().join("active.glb").is_file() {
        return Err("The character's avatar is missing. Save it again in Character Studio.".into());
    }

    let mut running = RUNNING_GAME.lock().map_err(|_| "game process lock poisoned")?;
    if let Some(child) = running.as_mut() {
        if child.try_wait().map_err(|e| e.to_string())?.is_none() {
            return Ok(serde_json::json!({ "alreadyRunning": true, "pid": child.id() }));
        }
    }
    *running = None;

    #[cfg(debug_assertions)]
    let (binary, working_dir) = {
        let root = repo_root();
        let filename = if cfg!(windows) { "black_anvil.exe" } else { "black_anvil" };
        let debug = root.join("target").join("debug").join(filename);
        let release = root.join("target").join("release").join(filename);
        (if debug.is_file() { debug } else { release }, root)
    };

    #[cfg(not(debug_assertions))]
    let (binary, working_dir) = {
        let game = app.path_resolver()
            .resolve_resource(if cfg!(windows) { "game/black_anvil.exe" } else { "game/black_anvil" })
            .ok_or("Game is not packaged with this launcher release")?;
        let root = game.parent().ok_or("Invalid packaged game path")?.to_path_buf();
        (game, root)
    };
    let _ = &app; // Debug builds do not need the production resource resolver.

    if !binary.is_file() {
        return Err(format!(
            "Bevy game executable not installed at {}. Development: run cargo build --bin black_anvil first.",
            binary.display()
        ));
    }
    let mut command = Command::new(&binary);
    command.current_dir(&working_dir);
    if multiplayer {
        command.env("BA_ZONE", "1");
    } else {
        command.env_remove("BA_ZONE");
        command.env_remove("BA_ZONE_URL");
    }
    let child = command.spawn().map_err(|e| format!("Could not start Bevy: {e}"))?;
    let pid = child.id();
    *running = Some(child);
    Ok(serde_json::json!({
        "pid": pid,
        "alreadyRunning": false,
        "mode": if multiplayer { "loopback-zone" } else { "local-preview" }
    }))
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![save_draft, get_draft, launch_game])
        .run(tauri::generate_context!())
        .expect("error while running Black Anvil launcher");
}
