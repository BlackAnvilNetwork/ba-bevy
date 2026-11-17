//! Authenticated, account-owned character store, immutable hash-addressed
//! avatars, single-use zone tickets and server-only persistent world state.

use std::path::{Path, PathBuf};
use std::fs;
use axum::{
    body::Body,
    extract::{Path as RoutePath, State},
    http::{header, HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    Json,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use uuid::Uuid;
use ba_protocol::{AppearanceWire, Pose, TraitRefWire};
use std::collections::BTreeMap;

use crate::{account_id_for_session, digest, now, random_secret, token_hash, require_internal, ApiError, ApiResult, Gateway, MAX_GLB_BYTES, MAX_CHARACTER_JSON};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CharacterUpload {
    character: Value,
    avatar_base64: String,
}

fn safe_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 &&
        id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
}

fn approved_character(value: &Value) -> ApiResult<(String,u64,String,String)> {
    let read = |key: &str| value.get(key).and_then(Value::as_str)
        .ok_or(ApiError::bad("invalid_character_definition"));
    let id = read("characterId")?;
    if !safe_id(id) { return Err(ApiError::bad("invalid_character_id")); }
    let revision = value.get("revision").and_then(Value::as_u64)
        .ok_or(ApiError::bad("invalid_revision"))?;
    if revision == 0 || revision > i64::MAX as u64 { return Err(ApiError::bad("invalid_revision")); }
    if value.get("schemaVersion").and_then(Value::as_u64) != Some(1) {
        return Err(ApiError::bad("unsupported_character_schema"));
    }
    let name = read("name")?;
    if name.trim().is_empty() || name.chars().count() > 24 {
        return Err(ApiError::bad("invalid_character_name"));
    }
    let job = read("startingJob")?;
    if !["TimeMage","Knight","Rogue","Cleric","Sorcerer","Fighter","BattleMage","Druid"].contains(&job) {
        return Err(ApiError::bad("unknown_starting_job"));
    }
    if read("pack")? != "anata" ||
        !["anata/male","anata/female"].contains(&read("manifestId")?) {
        return Err(ApiError::bad("unapproved_character_catalog"));
    }
    if read("catalogVersion")?.len() > 64 {
        return Err(ApiError::bad("invalid_catalog_version"));
    }
    // A studio upload cannot claim arbitrary client-trusted paths or remote URLs.
    let runtime = value.get("runtime").ok_or(ApiError::bad("missing_runtime"))?;
    if runtime.get("format").and_then(Value::as_str) != Some("glb") {
        return Err(ApiError::bad("unapproved_asset_format"));
    }
    for group in ["bodyTraits","starterEquipment"] {
        if let Some(traits) = value.get(group) {
            let entries = traits.as_object().ok_or(ApiError::bad("invalid_traits"))?;
            if entries.len() > 64 { return Err(ApiError::bad("too_many_traits")); }
            for (key,trait_value) in entries {
                if key.len()>64 || trait_value.get("id").and_then(Value::as_str)
                    .is_none_or(|id| id.is_empty() || id.len()>128)
                {
                    return Err(ApiError::bad("invalid_trait_ref"));
                }
            }
        }
    }
    Ok((id.to_string(),revision,name.to_string(),job.to_string()))
}

/// Validate self-contained GLB v2. Remote buffers/images and excessive model
/// complexity are rejected; a separate licensed manifest/catalog gate is still
/// needed before calling an asset genuinely approved for production.
fn validate_glb(bytes: &[u8]) -> ApiResult<()> {
    if bytes.len() < 24 || bytes.len() > MAX_GLB_BYTES || &bytes[0..4] != b"glTF" {
        return Err(ApiError::bad("invalid_glb"));
    }
    let u32_at = |at: usize| u32::from_le_bytes(bytes[at..at+4].try_into().unwrap()) as usize;
    if u32_at(4) != 2 || u32_at(8) != bytes.len() || &bytes[16..20] != b"JSON" {
        return Err(ApiError::bad("invalid_glb_header"));
    }
    let json_len = u32_at(12);
    if json_len == 0 || json_len > bytes.len()-20 {
        return Err(ApiError::bad("invalid_glb_json"));
    }
    let model: Value = serde_json::from_slice(&bytes[20..20+json_len])
        .map_err(|_| ApiError::bad("malformed_glb_scene"))?;
    let limits = [("nodes",2500),("meshes",400),("materials",256),("images",256),("animations",128),("skins",64)];
    for (name,max) in limits {
        if model.get(name).and_then(Value::as_array).is_some_and(|arr| arr.len()>max) {
            return Err(ApiError::bad("glb_complexity_budget_exceeded"));
        }
    }
    for key in ["buffers","images"] {
        if let Some(items) = model.get(key).and_then(Value::as_array) {
            for item in items {
                if item.get("uri").is_some() {
                    return Err(ApiError::bad("external_or_embedded_data_uri_not_allowed"));
                }
            }
        }
    }
    // JSON chunk padding + optional embedded binary chunk must stay within GLB.
    let padded_json_end = 20 + json_len;
    if padded_json_end < bytes.len() && bytes.len() - padded_json_end < 8 {
        return Err(ApiError::bad("invalid_glb_binary_chunk"));
    }
    Ok(())
}

fn store_verified_asset(root: &Path, hash: &str, bytes: &[u8]) -> ApiResult<PathBuf> {
    fs::create_dir_all(root).map_err(|_| ApiError::internal())?;
    let path = root.join(format!("{hash}.glb"));
    if path.exists() {
        let existing = fs::read(&path).map_err(|_| ApiError::internal())?;
        if digest(&existing) != hash { return Err(ApiError::conflict("asset_hash_collision")); }
        return Ok(path);
    }
    let stage = root.join(format!(".upload-{}",Uuid::new_v4()));
    let result = (|| {
        use std::io::Write;
        let mut f = fs::OpenOptions::new().write(true).create_new(true)
            .open(&stage).map_err(|_| ApiError::internal())?;
        f.write_all(bytes).map_err(|_| ApiError::internal())?;
        f.sync_all().map_err(|_| ApiError::internal())?;
        fs::rename(&stage,&path).map_err(|_| ApiError::internal())
    })();
    if result.is_err() { let _ = fs::remove_file(&stage); }
    result?;
    Ok(path)
}

pub(crate) async fn save_character(
    State(app): State<Gateway>, headers: HeaderMap,
    Json(upload): Json<CharacterUpload>,
) -> ApiResult<Json<Value>> {
    let account = account_id_for_session(&app,&headers)?;
    let raw_len = serde_json::to_vec(&upload.character).map_err(|_| ApiError::bad("invalid_character_json"))?.len();
    if raw_len > MAX_CHARACTER_JSON || upload.avatar_base64.len() > MAX_GLB_BYTES*4/3+8 {
        return Err(ApiError::bad("character_too_large"));
    }
    let (id,revision,name,job) = approved_character(&upload.character)?;
    let bytes = STANDARD.decode(upload.avatar_base64.as_bytes())
        .map_err(|_| ApiError::bad("invalid_avatar_encoding"))?;
    validate_glb(&bytes)?;
    let hash = digest(&bytes);
    let mut spec = upload.character;
    spec["runtime"] = json!({
        "format": "glb",
        "relativePath": format!("character://cache/{hash}.glb"),
        "contentHash": hash,
    });
    let normalized = serde_json::to_string(&spec).map_err(|_| ApiError::bad("invalid_character_definition"))?;

    {
        let db = app.db.lock().map_err(|_| ApiError::internal())?;
        let existing: Option<(String,i64,String,String)> = db.query_row(
            "SELECT id,revision,glb_hash,spec_json FROM characters WHERE account_id=?1",
            params![account], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)),
        ).optional()?;
        if let Some((old_id,old_rev,old_hash,old_spec)) = existing {
            if old_id != id { return Err(ApiError::conflict("one_character_per_discord_account")); }
            if revision < old_rev as u64 { return Err(ApiError::conflict("stale_character_revision")); }
            if revision == old_rev as u64 && (hash != old_hash || normalized != old_spec) {
                return Err(ApiError::conflict("character_revision_changed"));
            }
        }
    }
    store_verified_asset(&app.config.avatar_dir,&hash,&bytes)?;
    {
        let mut db = app.db.lock().map_err(|_| ApiError::internal())?;
        let tx = db.transaction()?;
        let result = tx.execute(
            "INSERT INTO characters(id,account_id,revision,spec_json,glb_hash,name,starting_job,updated_at)
             VALUES(?1,?2,?3,?4,?5,?6,?7,?8)
             ON CONFLICT(id) DO UPDATE SET
               revision=excluded.revision,spec_json=excluded.spec_json,
               glb_hash=excluded.glb_hash,name=excluded.name,
               updated_at=excluded.updated_at
             WHERE characters.account_id=excluded.account_id
               AND characters.revision<excluded.revision",
            params![id,account,revision as i64,normalized,hash,name,job,now()],
        )?;
        // The same revision is an idempotent success if it matches exactly.
        let current: (String,i64,String,String) = tx.query_row(
            "SELECT account_id,revision,glb_hash,spec_json FROM characters WHERE id=?1",
            params![id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)),
        )?;
        if current.0 != account || current.1 != revision as i64 || current.2 != hash || current.3 != normalized {
            return Err(ApiError::conflict("character_write_race"));
        }
        let _ = result;
        tx.execute(
            "INSERT OR IGNORE INTO avatars(hash,bytes,created_at) VALUES(?1,?2,?3)",
            params![hash,bytes.len() as i64,now()],
        )?;
        tx.commit()?;
    }
    Ok(Json(json!({"characterId":id,"revision":revision,"contentHash":hash,"status":"saved"})))
}

pub(crate) async fn selected(
    State(app): State<Gateway>, headers: HeaderMap,
) -> ApiResult<Json<Value>> {
    let account=account_id_for_session(&app,&headers)?;
    let db=app.db.lock().map_err(|_| ApiError::internal())?;
    let row: Option<(String,String,i64,i64,i64)> = db.query_row(
        "SELECT spec_json,glb_hash,level,xp,state_revision FROM characters WHERE account_id=?1",
        params![account], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)),
    ).optional()?;
    let Some((spec,hash,level,xp,revision))=row else {
        return Ok(Json(json!({ "character": null })));
    };
    let character: Value=serde_json::from_str(&spec).map_err(|_| ApiError::internal())?;
    Ok(Json(json!({"character":character,"contentHash":hash,"level":level,"xp":xp,"stateRevision":revision})))
}

pub(crate) async fn download_asset(
    State(app): State<Gateway>, RoutePath(hash): RoutePath<String>,
) -> ApiResult<Response> {
    if hash.len()!=64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(ApiError::bad("invalid_asset_id"));
    }
    let db=app.db.lock().map_err(|_| ApiError::internal())?;
    let approved: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM avatars WHERE hash=?1)",params![hash],|r|r.get(0),
    )?;
    drop(db);
    if !approved { return Err(ApiError(StatusCode::NOT_FOUND,"asset_not_found")); }
    let bytes=fs::read(app.config.avatar_dir.join(format!("{hash}.glb")))
        .map_err(|_| ApiError::internal())?;
    if bytes.len()>MAX_GLB_BYTES || digest(&bytes)!=hash {
        return Err(ApiError::internal());
    }
    Ok((
        [(header::CONTENT_TYPE,"model/gltf-binary"),
         (header::CACHE_CONTROL,"public, max-age=31536000, immutable")],
        Body::from(bytes),
    ).into_response())
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IssueTicket {
    character_id: String,
}

pub(crate) async fn issue_ticket(
    State(app): State<Gateway>,headers: HeaderMap,Json(data): Json<IssueTicket>,
) -> ApiResult<Json<Value>> {
    let account=account_id_for_session(&app,&headers)?;
    if !safe_id(&data.character_id) { return Err(ApiError::bad("invalid_character_id")); }
    let db=app.db.lock().map_err(|_| ApiError::internal())?;
    let exists: bool=db.query_row(
        "SELECT EXISTS(SELECT 1 FROM characters WHERE id=?1 AND account_id=?2)",
        params![data.character_id,account],|r|r.get(0),
    )?;
    if !exists { return Err(ApiError::forbidden()); }
    let secret=random_secret();
    db.execute(
        "INSERT INTO game_tickets(ticket_hash,account_id,character_id,expires_at)
         VALUES(?1,?2,?3,?4)",
        params![token_hash(&secret),account,data.character_id,now()+90],
    )?;
    Ok(Json(json!({"ticket":secret,"expiresIn":90,"characterId":data.character_id})))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RedeemTicket { ticket: String }

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReconnectRequest { reconnect_token: String }

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AdmittedProfile {
    account_id: String,
    character_id: String,
    appearance: AppearanceWire,
    pose: Pose,
    hp: i32,
    xp: u64,
    level: u32,
    inventory: Value,
    state_revision: u64,
    reconnect_token: String,
}

fn appearance_from_spec(spec: &Value, hash: &str) -> AppearanceWire {
    let traits = |key: &str| -> BTreeMap<String,TraitRefWire> {
        let mut out=BTreeMap::new();
        if let Some(values)=spec.get(key).and_then(Value::as_object) {
            for (name,value) in values {
                if let Some(id)=value.get("id").and_then(Value::as_str) {
                    out.insert(name.clone(),TraitRefWire {
                        id: id.to_string(), name: value.get("name").and_then(Value::as_str).map(str::to_string),
                    });
                }
            }
        }
        out
    };
    AppearanceWire {
        character_id: spec["characterId"].as_str().unwrap_or_default().to_string(),
        name: spec["name"].as_str().unwrap_or_default().to_string(),
        starting_job: spec["startingJob"].as_str().unwrap_or_default().to_string(),
        manifest_id: spec["manifestId"].as_str().unwrap_or_default().to_string(),
        body_traits: traits("bodyTraits"),
        starter_equipment: traits("starterEquipment"),
        glb_content_hash: Some(hash.to_string()),
    }
}

fn profile_for_character(
    db: &rusqlite::Connection, account: &str, character: &str, reconnect_token: String,
) -> ApiResult<AdmittedProfile> {
    let row: (String,String,String,f32,f32,f32,f32,i32,i64,i64,String,i64)=db.query_row(
        "SELECT account_id,id,spec_json,px,py,pz,yaw,hp,xp,level,inventory_json,state_revision
         FROM characters WHERE id=?1 AND account_id=?2",
        params![character,account],
        |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?,r.get(8)?,r.get(9)?,r.get(10)?,r.get(11)?)),
    )?;
    let hash: String=db.query_row(
        "SELECT glb_hash FROM characters WHERE id=?1",
        params![character],|r|r.get(0),
    )?;
    let spec: Value=serde_json::from_str(&row.2).map_err(|_| ApiError::internal())?;
    let inventory: Value=serde_json::from_str(&row.10).map_err(|_| ApiError::internal())?;
    let pose=Pose{x:row.3,y:row.4,z:row.5,yaw:row.6};
    Ok(AdmittedProfile {
        account_id:row.0,character_id:row.1,appearance:appearance_from_spec(&spec,&hash),
        pose:if pose.is_finite(){pose}else{Pose::default()},
        hp:row.7,xp:row.8.max(0) as u64,level:row.9.max(1) as u32,
        inventory,state_revision:row.11.max(0) as u64,reconnect_token,
    })
}

fn rotate_reconnect(db:&rusqlite::Connection,account:&str,character:&str) -> ApiResult<String> {
    let token=random_secret();
    db.execute(
        "INSERT INTO reconnect_tokens(token_hash,account_id,character_id,expires_at)
         VALUES(?1,?2,?3,?4)",
        params![token_hash(&token),account,character,now()+3600],
    )?;
    Ok(token)
}

pub(crate) async fn redeem_ticket(
    State(app): State<Gateway>, headers: HeaderMap,
    Json(request): Json<RedeemTicket>,
) -> ApiResult<Json<Value>> {
    require_internal(&app,&headers)?;
    if request.ticket.len()<32 || request.ticket.len()>256 {return Err(ApiError::auth());}
    let mut db=app.db.lock().map_err(|_| ApiError::internal())?;
    let tx=db.transaction()?;
    let data: Option<(String,String)>=tx.query_row(
        "SELECT account_id,character_id FROM game_tickets
         WHERE ticket_hash=?1 AND redeemed_at IS NULL AND expires_at>?2",
        params![token_hash(&request.ticket),now()],
        |r|Ok((r.get(0)?,r.get(1)?)),
    ).optional()?;
    let (account,character)=data.ok_or_else(ApiError::auth)?;
    if tx.execute(
        "UPDATE game_tickets SET redeemed_at=?2 WHERE ticket_hash=?1 AND redeemed_at IS NULL",
        params![token_hash(&request.ticket),now()],
    )? != 1 { return Err(ApiError::auth()); }
    let reconnect=rotate_reconnect(&tx,&account,&character)?;
    let profile=profile_for_character(&tx,&account,&character,reconnect)?;
    tx.commit()?;
    Ok(Json(serde_json::to_value(profile).map_err(|_| ApiError::internal())?))
}

pub(crate) async fn reconnect(
    State(app): State<Gateway>,headers: HeaderMap,
    Json(request):Json<ReconnectRequest>,
) -> ApiResult<Json<Value>> {
    require_internal(&app,&headers)?;
    if request.reconnect_token.len()<32 || request.reconnect_token.len()>256{return Err(ApiError::auth());}
    let mut db=app.db.lock().map_err(|_| ApiError::internal())?;
    let tx=db.transaction()?;
    let identity: Option<(String,String)>=tx.query_row(
        "SELECT account_id,character_id FROM reconnect_tokens
         WHERE token_hash=?1 AND consumed_at IS NULL AND expires_at>?2",
        params![token_hash(&request.reconnect_token),now()],
        |r|Ok((r.get(0)?,r.get(1)?)),
    ).optional()?;
    let (account,character)=identity.ok_or_else(ApiError::auth)?;
    if tx.execute(
        "UPDATE reconnect_tokens SET consumed_at=?2
         WHERE token_hash=?1 AND consumed_at IS NULL",
        params![token_hash(&request.reconnect_token),now()],
    )? != 1 { return Err(ApiError::auth()); }
    let next=rotate_reconnect(&tx,&account,&character)?;
    let profile=profile_for_character(&tx,&account,&character,next)?;
    tx.commit()?;
    Ok(Json(serde_json::to_value(profile).map_err(|_| ApiError::internal())?))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveWorldState {
    account_id: String,
    character_id: String,
    state_revision: u64,
    pose: Pose,
    hp: i32,
    xp: u64,
    level: u32,
    inventory: Value,
}

/// Only the trusted zone service can advance world state revisions. Client
/// requests cannot call this endpoint without the private service credential.
pub(crate) async fn save_world_state(
    State(app): State<Gateway>,headers: HeaderMap,
    Json(next): Json<SaveWorldState>,
) -> ApiResult<Json<Value>> {
    require_internal(&app,&headers)?;
    if !safe_id(&next.character_id) || !next.pose.is_finite()
        || !(1..=1000).contains(&next.level) || next.hp<0
        || next.state_revision==0 || next.state_revision>i64::MAX as u64
        || next.xp>i64::MAX as u64
    {
        return Err(ApiError::bad("invalid_authoritative_state"));
    }
    let json=serde_json::to_string(&next.inventory).map_err(|_| ApiError::bad("invalid_inventory"))?;
    if json.len()>MAX_CHARACTER_JSON { return Err(ApiError::bad("inventory_too_large")); }
    let db=app.db.lock().map_err(|_| ApiError::internal())?;
    let updated=db.execute(
        "UPDATE characters SET px=?1,py=?2,pz=?3,yaw=?4,hp=?5,xp=?6,level=?7,
            inventory_json=?8,state_revision=?9,updated_at=?10
         WHERE id=?11 AND account_id=?12 AND state_revision<?9",
        params![next.pose.x,next.pose.y,next.pose.z,next.pose.yaw,next.hp,next.xp as i64,
                next.level as i64,json,next.state_revision as i64,now(),
                next.character_id,next.account_id],
    )?;
    if updated != 1 {
        return Err(ApiError::conflict("stale_or_unauthorized_world_revision"));
    }
    Ok(Json(json!({"ok":true,"stateRevision":next.state_revision})))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn malformed_external_glb_is_rejected() {
        assert!(validate_glb(b"not a glb").is_err());
        let mut bytes=b"glTF".to_vec();
        bytes.extend_from_slice(&2u32.to_le_bytes());
        bytes.extend_from_slice(&24u32.to_le_bytes());
        bytes.extend_from_slice(&4u32.to_le_bytes());
        bytes.extend_from_slice(b"JSON");
        bytes.extend_from_slice(b"null");
        assert!(validate_glb(&bytes).is_ok());
        let mut bad=bytes.clone();
        bad[0]=b'X';
        assert!(validate_glb(&bad).is_err());
    }
    #[test]
    fn unauthorized_trait_and_path_names_cannot_be_uploaded() {
        let mut spec=json!({
            "characterId":"../../secret", "revision":1,"schemaVersion":1,
            "name":"Chrona","startingJob":"TimeMage","pack":"anata",
            "manifestId":"anata/male","catalogVersion":"anata-2026-11",
            "runtime":{"format":"glb"}
        });
        assert!(approved_character(&spec).is_err());
        spec["characterId"]=json!("safe-uuid");
        assert!(approved_character(&spec).is_ok());
    }
}
