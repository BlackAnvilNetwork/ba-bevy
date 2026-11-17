#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

//! Black Anvil launcher: explicit versioned character save and native
//! Bevy handoff. Production requires Discord tickets and signed game packs.
//! No game login or AA ownership is inferred from local filesystem contents.

mod discord;

use std::fs::{self, File};
use std::io::{Read,Write};
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::Mutex;

use atomicwrites::{AllowOverwrite, AtomicFile, DisallowOverwrite};
use base64::{engine::general_purpose::STANDARD as B64, Engine as _};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tauri::Manager;

static RUNNING_GAME: Mutex<Option<Child>> = Mutex::new(None);
static SAVE_LOCK: Mutex<()> = Mutex::new(());
static VERIFIED_GAME: Mutex<Option<PathBuf>> = Mutex::new(None);

#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
struct GameManifest {
    schema_version: u32,
    files: Vec<GameManifestFile>,
}

#[derive(Deserialize)]
struct GameManifestFile {
    path: String,
    bytes: u64,
    sha256: String,
}

/// Verify every packaged game asset before first launch. The release pipeline
/// signs the EXE and installer; this manifest detects broken/corrupt installs.
/// An unsigned staging MSI is NOT a secure production update.
fn verify_game_bundle(root:&Path) -> Result<(),String> {
    let mut verified=VERIFIED_GAME.lock().map_err(|_|"Game verification lock poisoned")?;
    if verified.as_ref().is_some_and(|path| path==root) {return Ok(());}
    let raw=fs::read_to_string(root.join("manifest.json"))
        .map_err(|_|"Game manifest missing; repair or reinstall Black Anvil")?;
    let manifest: GameManifest=serde_json::from_str(&raw)
        .map_err(|_|"Malformed game content manifest")?;
    if manifest.schema_version!=1 || manifest.files.len()<2 || manifest.files.len()>50_000 {
        return Err("Invalid game content manifest".into());
    }
    for item in manifest.files {
        let rel=Path::new(&item.path);
        if !rel.components().all(|part| matches!(part,std::path::Component::Normal(_)))
            || item.path.contains(':') || item.sha256.len()!=64 {
            return Err("Manifest contains an unsafe file path or hash".into());
        }
        let path=root.join(rel);
        let mut file=fs::File::open(&path)
            .map_err(|_|format!("Game file missing: {}",item.path))?;
        let metadata=file.metadata().map_err(|_|"Cannot inspect game file")?;
        if !metadata.is_file() || metadata.len()!=item.bytes {
            return Err(format!("Game file size mismatch: {}",item.path));
        }
        let mut checksum=Sha256::new();
        let mut buffer=[0_u8;65536];
        loop {
            let n=file.read(&mut buffer).map_err(|_|"Cannot validate game file")?;
            if n==0 {break;}
            checksum.update(&buffer[..n]);
        }
        if !format!("{:x}",checksum.finalize()).eq_ignore_ascii_case(&item.sha256) {
            return Err(format!("Game file hash mismatch: {}",item.path));
        }
    }
    *verified=Some(root.to_path_buf());
    Ok(())
}


const MAX_GLB: usize = 32 * 1024 * 1024;
const MAX_JSON: usize = 128 * 1024;
const JOBS: &[&str] = &[
    "TimeMage", "Knight", "Rogue", "Cleric", "Sorcerer", "Fighter", "BattleMage", "Druid",
];

#[cfg(debug_assertions)]
fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .canonicalize()
        .unwrap_or_else(|_| PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../.."))
}

fn app_characters_dir() -> Result<PathBuf, String> {
    let data_dir = dirs_next::data_local_dir()
        .ok_or("Could not locate operating-system application data directory")?;
    Ok(data_dir.join("BlackAnvil").join("characters"))
}

fn assert_safe_character_id(id: &str) -> Result<(), String> {
    if id.is_empty() || id.len() > 64
        || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        || id == "." || id == ".."
    {
        return Err("characterId must be a safe 1-64 character identifier".into());
    }
    Ok(())
}

fn scoped_under(root: &Path, id: &str) -> Result<PathBuf, String> {
    assert_safe_character_id(id)?;
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let canonical_root = root.canonicalize().map_err(|e| e.to_string())?;
    let candidate = canonical_root.join(id);
    let resolved = if candidate.exists() {
        candidate.canonicalize().map_err(|e| e.to_string())?
    } else {
        candidate
    };
    if !resolved.starts_with(&canonical_root) {
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SelectedCharacter {
    character_id: String,
    revision: u64,
    content_hash: String,
}

fn validate_character(json: &str) -> Result<BaCharacterV1, String> {
    if json.len() > MAX_JSON {
        return Err("character definition too large".into());
    }
    let c: BaCharacterV1 =
        serde_json::from_str(json).map_err(|e| format!("invalid BaCharacterV1: {e}"))?;
    if c.schema_version != 1 || c.revision < 1 {
        return Err("unsupported schemaVersion or revision".into());
    }
    assert_safe_character_id(&c.character_id)?;
    if c.name.trim().is_empty() || c.name.chars().count() > 24 {
        return Err("invalid character name".into());
    }
    if !JOBS.contains(&c.starting_job.as_str()) {
        return Err("unknown startingJob".into());
    }
    if c.pack != "anata"
        || (c.manifest_id != "anata/male" && c.manifest_id != "anata/female")
    {
        return Err("unknown character catalog".into());
    }
    if c.catalog_version.is_empty() || c.runtime.format != "glb" {
        return Err("invalid catalogVersion or asset format".into());
    }
    let _ = c.runtime.relative_path; // Never trust unvalidated paths supplied by Studio.
    Ok(c)
}

fn assert_glb(bytes: &[u8]) -> Result<(), String> {
    if bytes.len() < 20 || bytes.len() > MAX_GLB || &bytes[0..4] != b"glTF" {
        return Err("invalid GLB size or magic".into());
    }
    let version = u32::from_le_bytes(bytes[4..8].try_into().unwrap());
    let declared_size = u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize;
    let json_chunk_size = u32::from_le_bytes(bytes[12..16].try_into().unwrap()) as usize;
    if version != 2 || declared_size != bytes.len() || &bytes[16..20] != b"JSON"
        || json_chunk_size > bytes.len().saturating_sub(20)
    {
        return Err("GLB v2 header or JSON chunk is invalid".into());
    }
    Ok(())
}

fn glb_hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn revision_folder(root: &Path, selected: &SelectedCharacter) -> Result<PathBuf, String> {
    let character_dir = scoped_under(root, &selected.character_id)?;
    if selected.revision == 0 {
        return Err("invalid selected revision".into());
    }
    Ok(character_dir.join("revisions").join(format!("r{}", selected.revision)))
}

fn selection_path(root: &Path) -> PathBuf {
    root.join("selected.json")
}

fn read_selection(root: &Path) -> Result<Option<SelectedCharacter>, String> {
    let path = selection_path(root);
    if !path.is_file() {
        return Ok(None);
    }
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let selected: SelectedCharacter =
        serde_json::from_str(&raw).map_err(|e| format!("Corrupt selected character: {e}"))?;
    assert_safe_character_id(&selected.character_id)?;
    if selected.revision == 0 || selected.content_hash.len() != 64
        || !selected.content_hash.chars().all(|c| c.is_ascii_hexdigit())
    {
        return Err("Invalid selected character pointer".into());
    }
    Ok(Some(selected))
}

fn read_selected_character(root: &Path) -> Result<Option<(SelectedCharacter, String, PathBuf)>, String> {
    let Some(selected) = read_selection(root)? else {
        return Ok(None);
    };
    let rev = revision_folder(root, &selected)?;
    let json = fs::read_to_string(rev.join("character.json"))
        .map_err(|e| format!("Character revision missing: {e}"))?;
    let character = validate_character(&json)?;
    if character.character_id != selected.character_id || character.revision != selected.revision {
        return Err("Selected character revision does not match its record".into());
    }
    let glb_path = rev.join("character.glb");
    let bytes = fs::read(&glb_path).map_err(|e| format!("Character avatar missing: {e}"))?;
    assert_glb(&bytes)?;
    if glb_hash(&bytes) != selected.content_hash {
        return Err("Character avatar has changed since save; re-save it in Studio".into());
    }
    Ok(Some((selected, json, glb_path)))
}

fn write_synced(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let mut file = File::create(path).map_err(|e| e.to_string())?;
    file.write_all(bytes).map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())
}

/// Publish immutable revision files *before* atomically selecting the new revision.
/// A crash before selection leaves the prior character usable and the new
/// revision safely retryable. No JSON/GLB active-pair rewrite is required.
fn save_revision(
    root: &Path,
    character: &BaCharacterV1,
    normalized: &str,
    glb: &[u8],
    hash: &str,
) -> Result<PathBuf, String> {
    let existing = read_selection(root)?;
    if let Some(old) = &existing {
        if old.character_id == character.character_id {
            if character.revision < old.revision {
                return Err("Refusing to overwrite newer character revision".into());
            }
            if character.revision == old.revision && old.content_hash != hash {
                return Err("Revision conflict: new avatar requires a higher revision".into());
            }
        }
    }

    let pointer = SelectedCharacter {
        character_id: character.character_id.clone(),
        revision: character.revision,
        content_hash: hash.to_string(),
    };
    let rev = revision_folder(root, &pointer)?;
    if rev.is_dir() {
        let old_json = fs::read_to_string(rev.join("character.json"))
            .map_err(|e| e.to_string())?;
        let old_glb = fs::read(rev.join("character.glb")).map_err(|e| e.to_string())?;
        if old_json != normalized || glb_hash(&old_glb) != hash {
            return Err("Revision already exists with different contents".into());
        }
    } else {
        let parent = rev.parent().ok_or("Invalid revisions path")?;
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        let stage = parent.join(format!(
            ".pending-r{}-{}-{}",
            character.revision,
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_err(|e| e.to_string())?
                .as_nanos()
        ));
        fs::create_dir(&stage).map_err(|e| e.to_string())?;
        let write_result = (|| {
            write_synced(&stage.join("character.json"), normalized.as_bytes())?;
            write_synced(&stage.join("character.glb"), glb)?;
            fs::rename(&stage, &rev).map_err(|e| e.to_string())?;
            Ok::<(), String>(())
        })();
        if write_result.is_err() {
            let _ = fs::remove_dir_all(&stage);
        }
        write_result?;
    }

    // Publish an immutable content-addressed local cache asset for same-host
    // multiplayer previews. A later authenticated character service will
    // distribute approved hashes across machines; this local cache is not that.
    let cache_dir = root.join("cache");
    fs::create_dir_all(&cache_dir).map_err(|e| e.to_string())?;
    let cache_path = cache_dir.join(format!("{hash}.glb"));
    if cache_path.exists() {
        let existing_bytes = fs::read(&cache_path).map_err(|e| e.to_string())?;
        if glb_hash(&existing_bytes) != hash || assert_glb(&existing_bytes).is_err() {
            return Err("Conflicting or corrupted avatar content cache".into());
        }
    } else {
        AtomicFile::new(&cache_path, DisallowOverwrite)
            .write(|f| {
                f.write_all(glb)?;
                f.sync_all()
            })
            .map_err(|e| format!("Could not publish immutable avatar cache: {e}"))?;
    }

    // This is the sole active-selection mutation. Atomic on Windows and POSIX.
    let data = serde_json::to_vec(&pointer).map_err(|e| e.to_string())?;
    AtomicFile::new(selection_path(root), AllowOverwrite)
        .write(|f| {
            f.write_all(&data)?;
            f.sync_all()
        })
        .map_err(|e| format!("Could not select saved character: {e}"))?;
    Ok(rev)
}

#[tauri::command]
fn save_draft(character_json: String, glb_base64: String) -> Result<serde_json::Value, String> {
    let _guard = SAVE_LOCK.lock().map_err(|_| "character save lock poisoned")?;
    let c = validate_character(&character_json)?;
    if glb_base64.len() > MAX_GLB * 4 / 3 + 8 {
        return Err("avatar upload too large".into());
    }
    let glb = B64.decode(glb_base64.as_bytes())
        .map_err(|e| format!("invalid avatar encoding: {e}"))?;
    assert_glb(&glb)?;
    let hash = glb_hash(&glb);

    let mut value: serde_json::Value =
        serde_json::from_str(&character_json).map_err(|e| e.to_string())?;
    if let Some(runtime) = value.get_mut("runtime") {
        runtime["format"] = serde_json::json!("glb");
        runtime["relativePath"] = serde_json::json!(
            format!("character://{}/revisions/r{}/character.glb", c.character_id, c.revision)
        );
        runtime["contentHash"] = serde_json::json!(hash);
    }
    let normalized = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
    let root = app_characters_dir()?;
    let rev = save_revision(&root, &c, &normalized, &glb, &hash)?;
    Ok(serde_json::json!({
        "ok": true, "prototype": true, "revision": c.revision,
        "json": rev.join("character.json").display().to_string(),
        "glb": rev.join("character.glb").display().to_string(),
        "contentHash": hash,
    }))
}

#[tauri::command]
fn get_draft() -> Result<Option<String>, String> {
    let root = app_characters_dir()?;
    Ok(read_selected_character(&root)?.map(|(_, json, _)| json))
}

/// Start the native Bevy executable with the *selected immutable revision*.
/// The selected revision is verified before game launch. This is a local preview,
/// not a Discord-authenticated or signed-production game session.
#[tauri::command]
async fn launch_game(app: tauri::AppHandle, multiplayer: bool) -> Result<serde_json::Value, String> {
    let root = app_characters_dir()?;
    let (selected, json, glb) = read_selected_character(&root)?
        .ok_or("Create and save your character in Studio before playing")?;
    let draft_path = revision_folder(&root, &selected)?.join("character.json");

    // Only the online game path requires a Discord-authenticated account.
    // Syncing happens before a ticket is issued; a rejected account/character
    // mapping cannot launch multiplayer. Local preview remains available.
    let game_ticket = if multiplayer {
        let bytes = fs::read(&glb).map_err(|e| format!("Cannot read saved avatar: {e}"))?;
        let synced = discord::sync_character(&json, &bytes).await?;
        if synced["characterId"].as_str() != Some(selected.character_id.as_str())
            || synced["contentHash"].as_str() != Some(selected.content_hash.as_str()) {
            return Err("Gateway returned a mismatched character sync receipt".into());
        }
        Some(discord::issue_game_ticket(&selected.character_id).await?)
    } else {
        None
    };

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
            .ok_or("Bevy game is not included with this launcher build")?;
        let dir = game.parent().ok_or("Invalid packaged game path")?.to_path_buf();
        (game, dir)
    };
    let _ = &app;

    #[cfg(debug_assertions)]
    if multiplayer && std::env::var_os("BA_ZONE_URL").is_none() {
        let address = std::net::SocketAddr::from(([127, 0, 0, 1], 8787));
        std::net::TcpStream::connect_timeout(&address, std::time::Duration::from_millis(350))
            .map_err(|_| "Local zone unavailable. Start authenticated ba_zone_server or configure BA_ZONE_URL for WSS".to_string())?;
    }

    if !binary.is_file() {
        return Err(format!("Bevy game executable not installed: {}", binary.display()));
    }
    #[cfg(not(debug_assertions))]
    verify_game_bundle(&working_dir)?;

    let mut command = Command::new(&binary);
    command.current_dir(&working_dir);
    command.env("BA_LAUNCH_MODE", "game");
    command.env("BA_CHARACTER_ROOT", &root);
    command.env("BA_CHARACTER_DRAFT_PATH", &draft_path);
    command.env("BA_CHARACTER_GLB_PATH", &glb);
    // Bevy's installed AssetServer reads the packaged resources, never the
    // original developer checkout from CARGO_MANIFEST_DIR.
    command.env("BA_GAME_ASSET_ROOT", working_dir.join("assets"));
    if let Some(ticket) = game_ticket {
        // One-use, short-lived zone ticket. No Discord token, no wallet signer,
        // and no long-lived game account token are passed to Bevy.
        command.env("BA_GAME_TICKET", ticket);
        command.env("BA_GATEWAY_PUBLIC_URL", discord::gateway_base()?);
    }
    if multiplayer {
        command.env("BA_ZONE", "1");
        if let Ok(url)=std::env::var("BA_ZONE_URL") {
            command.env("BA_ZONE_URL",url);
        } else if let Some(url)=option_env!("BA_ZONE_PUBLIC_URL") {
            command.env("BA_ZONE_URL",url);
        } else if !cfg!(debug_assertions) {
            return Err("Game server WSS URL is not configured for this release".into());
        }
    } else {
        command.env_remove("BA_ZONE");
        command.env_remove("BA_ZONE_URL");
    }
    let child = command.spawn().map_err(|e| format!("Failed to launch Bevy: {e}"))?;
    let pid = child.id();
    *running = Some(child);
    Ok(serde_json::json!({
        "pid": pid, "alreadyRunning": false,
        "mode": if multiplayer { "loopback-zone" } else { "local-preview" },
        "characterId": selected.character_id,
        "revision": selected.revision
    }))
}

fn main() {
    tauri::Builder::default()
        .setup(|_app| {
            discord::ensure_dev_gateway();
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            save_draft, get_draft, launch_game,
            discord::begin_discord_login, discord::await_discord_login,
            discord::discord_account, discord::discord_logout,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Black Anvil launcher");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_path_traversal_and_bad_glb() {
        assert!(assert_safe_character_id("../escape").is_err());
        assert!(assert_safe_character_id("a/b").is_err());
        assert!(assert_safe_character_id("aaa_bbb-001").is_ok());
        assert!(assert_glb(b"not gltf").is_err());
    }

    fn fixture_glb() -> Vec<u8> {
        let mut glb = b"glTF".to_vec();
        glb.extend_from_slice(&2u32.to_le_bytes()); // GLB v2
        glb.extend_from_slice(&20u32.to_le_bytes()); // total length
        glb.extend_from_slice(&0u32.to_le_bytes()); // empty JSON chunk (storage fixture only)
        glb.extend_from_slice(b"JSON");
        glb
    }

    #[test]
    fn selected_revision_is_atomic_and_idempotent() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos();
        let root = std::env::temp_dir().join(format!("ba-character-store-{}-{}", std::process::id(), stamp));
        fs::create_dir_all(&root).unwrap();

        let json = r#"{"schemaVersion":1,"revision":1,"characterId":"fixture-player","name":"Chrona","startingJob":"TimeMage","pack":"anata","manifestId":"anata/male","catalogVersion":"anata-2026-11","runtime":{"format":"glb","relativePath":"character://fixture-player/revisions/r1/character.glb"}}"#;
        let character = validate_character(json).unwrap();
        let glb = fixture_glb();
        let hash = glb_hash(&glb);
        save_revision(&root, &character, json, &glb, &hash).unwrap();
        save_revision(&root, &character, json, &glb, &hash).unwrap(); // exact retry
        let (selected, saved, _) = read_selected_character(&root).unwrap().unwrap();
        assert_eq!(selected.revision, 1);
        assert_eq!(selected.content_hash, hash);
        assert_eq!(saved, json);

        // An unpublished second revision must not change the selected snapshot.
        let pending = root.join("fixture-player/revisions/.pending-r2-incomplete");
        fs::create_dir_all(&pending).unwrap();
        fs::write(pending.join("character.json"), b"incomplete").unwrap();
        assert_eq!(read_selection(&root).unwrap().unwrap().revision, 1);

        // A different asset cannot silently overwrite the same revision.
        assert!(save_revision(&root, &character, "different", &glb, &hash).is_err());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn enforces_revision_and_schema_bounds() {
        let base = r#"{"schemaVersion":1,"revision":1,"characterId":"abc-123","name":"Chrona","startingJob":"TimeMage","pack":"anata","manifestId":"anata/male","catalogVersion":"1","runtime":{"format":"glb"}}"#;
        assert!(validate_character(base).is_ok());
        assert!(validate_character(&base.replace("\"revision\":1", "\"revision\":0")).is_err());
        assert!(validate_character(&base.replace("TimeMage", "UnknownJob")).is_err());
    }
}
