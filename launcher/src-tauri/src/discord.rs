//! Native Discord login bridge. Browser OAuth is handled by ba_gateway; the
//! launcher keeps only a Black Anvil account session in the OS credential store.
//! No Discord access tokens, AA keys, or polling secrets enter the React UI.

use std::sync::Mutex;
use std::time::{Duration,Instant};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Deserialize;
use serde_json::{json,Value};

static LOGIN: Mutex<Option<PendingLogin>> = Mutex::new(None);

#[derive(Clone)]
struct PendingLogin {
    flow_id: String,
    poll_secret: String,
    started_at: Instant,
}

#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
struct StartResponse {
    authorize_url: String,
    flow_id: String,
    poll_secret: String,
}

pub(crate) fn gateway_base() -> Result<String,String> {
    let base=std::env::var("BA_GATEWAY_PUBLIC_URL")
        .or_else(|_| option_env!("BA_GATEWAY_PUBLIC_URL").map(str::to_owned)
            .ok_or(std::env::VarError::NotPresent))
        .unwrap_or_else(|_| "http://127.0.0.1:8790".into());
    if !cfg!(debug_assertions) && base=="http://127.0.0.1:8790" {
        return Err("Release missing BA_GATEWAY_PUBLIC_URL compiled configuration".into());
    }
    let url=reqwest::Url::parse(&base).map_err(|_|"Invalid BA_GATEWAY_PUBLIC_URL")?;
    let local=url.host_str().is_some_and(|h|h=="localhost" || h=="127.0.0.1" || h=="::1");
    if url.scheme()!="https" && !(url.scheme()=="http" && local) {
        return Err("Gateway must use HTTPS outside localhost".into());
    }
    if url.username()!="" || url.password().is_some() ||
        url.query().is_some() || url.fragment().is_some() || url.path()!="/" {
        return Err("Gateway URL must be an origin without credentials or a path".into());
    }
    Ok(base.trim_end_matches('/').to_string())
}

fn client()->Result<reqwest::Client,String> {
    reqwest::Client::builder().timeout(Duration::from_secs(25))
        .redirect(reqwest::redirect::Policy::none())
        .build().map_err(|_|"Unable to create secure HTTP client".to_string())
}

fn credential()->Result<keyring::Entry,String> {
    keyring::Entry::new("network.blackanvil.launcher","game-session")
        .map_err(|_|"Cannot access OS secure credential store".to_string())
}

fn session_token()->Result<String,String> {
    credential()?.get_password().map_err(|_|"Discord login required".to_string())
}

/// Starts login through the system browser (the React UI calls Tauri shell.open).
/// Poll secret remains in native memory and is never serialized to JavaScript.
#[tauri::command]
pub async fn begin_discord_login()->Result<Value,String> {
    let url=format!("{}/v1/auth/start",gateway_base()?);
    let resp=client()?.post(url).send().await
        .map_err(|_|"Discord gateway unavailable".to_string())?;
    if !resp.status().is_success() {return Err(format!("Gateway rejected sign-in ({})",resp.status()));}
    let start: StartResponse=resp.json().await.map_err(|_|"Invalid login response".to_string())?;
    if !start.authorize_url.starts_with("https://discord.com/oauth2/authorize?") {
        return Err("Untrusted Discord authorization URL".into());
    }
    *LOGIN.lock().map_err(|_|"Login lock poisoned")?=Some(PendingLogin {
        flow_id:start.flow_id,poll_secret:start.poll_secret,started_at:Instant::now(),
    });
    Ok(json!({"authorizationUrl":start.authorize_url,"status":"pending"}))
}

#[tauri::command]
pub async fn poll_discord_login()->Result<Value,String> {
    let pending=LOGIN.lock().map_err(|_|"Login lock poisoned")?.clone()
        .ok_or("No Discord login in progress")?;
    if pending.started_at.elapsed()>Duration::from_secs(600) {
        *LOGIN.lock().map_err(|_|"Login lock poisoned")?=None;
        return Err("Discord login expired. Try again.".into());
    }
    let url=format!("{}/v1/auth/poll",gateway_base()?);
    let resp=client()?.post(url)
        .json(&json!({"flowId":pending.flow_id,"pollSecret":pending.poll_secret}))
        .send().await.map_err(|_|"Discord gateway unavailable".to_string())?;
    if !resp.status().is_success() {return Err(format!("Discord login polling failed ({})",resp.status()));}
    let mut result: Value=resp.json().await.map_err(|_|"Invalid Discord login poll response".to_string())?;
    if result["status"]=="complete" {
        let token=result.get("sessionToken").and_then(Value::as_str)
            .ok_or("Missing game session")?;
        credential()?.set_password(token).map_err(|_|"Unable to store game session securely")?;
        *LOGIN.lock().map_err(|_|"Login lock poisoned")?=None;
        result.as_object_mut().unwrap().remove("sessionToken");
        result["connected"]=json!(true);
    }
    Ok(result)
}

#[tauri::command]
pub async fn discord_account()->Result<Value,String> {
    let token=match session_token() {
        Ok(v)=>v,
        Err(_)=>return Ok(json!({"connected":false})),
    };
    let url=format!("{}/v1/me",gateway_base()?);
    let resp=client()?.get(url).bearer_auth(token).send().await
        .map_err(|_|"Account service unavailable; local preview is still available".to_string())?;
    if resp.status()==reqwest::StatusCode::UNAUTHORIZED {
        return Ok(json!({"connected":false,"reason":"session_expired"}));
    }
    if !resp.status().is_success() {return Err("Account lookup failed".into());}
    let mut data:Value=resp.json().await.map_err(|_|"Invalid account response")?;
    data["connected"]=json!(true);
    Ok(data)
}

#[tauri::command]
pub async fn discord_logout()->Result<Value,String> {
    if let Ok(token)=session_token() {
        let url=format!("{}/v1/auth/logout",gateway_base()?);
        // Best effort server revocation; do not retain local credential on error.
        let _=client()?.post(url).bearer_auth(token).send().await;
    }
    credential()?.delete_credential().map_err(|_|"Unable to remove OS session credential".to_string())?;
    Ok(json!({"connected":false}))
}

/// Uploads exactly the versioned, hash-validated local revision selected by
/// Tauri's native save operation. The gateway enforces account ownership and
/// substitutes an approved immutable model hash into the canonical record.
pub(crate) async fn sync_character(json:&str,glb:&[u8])->Result<Value,String> {
    let token=session_token()?;
    let definition: Value=serde_json::from_str(json).map_err(|_|"Invalid local character JSON")?;
    let url=format!("{}/v1/characters",gateway_base()?);
    let res=client()?.post(url).bearer_auth(token)
        .json(&json!({
            "character":definition,
            "avatarBase64":STANDARD.encode(glb),
        })).send().await.map_err(|_|"Could not sync character to gateway".to_string())?;
    if !res.status().is_success() {
        return Err(format!("Account-owned character sync rejected ({})",res.status()));
    }
    res.json().await.map_err(|_|"Invalid character sync receipt".into())
}

/// Recover a finalized character on a different machine using the verified
/// Discord account session. The returned model is SHA-256 checked before any
/// local Bevy scene can load it; no local file-path claims are trusted.
pub(crate) async fn retrieve_character()->Result<Option<(Value,Vec<u8>)>,String> {
    let token=session_token()?;
    let base=gateway_base()?;
    let response=client()?.get(format!("{base}/v1/characters/selected"))
        .bearer_auth(&token).send().await
        .map_err(|_|"Unable to access saved Black Anvil character".to_string())?;
    if !response.status().is_success() {
        return Err(format!("Account-owned character lookup rejected ({})",response.status()));
    }
    let data:Value=response.json().await.map_err(|_|"Malformed character restore response")?;
    let Some(definition)=data.get("character").filter(|v|!v.is_null()) else {
        return Ok(None);
    };
    let id=definition.get("characterId").and_then(Value::as_str)
        .ok_or("Character ID missing from trusted response")?;
    if id.is_empty() || id.len()>64 || !id.chars().all(|c| c.is_ascii_alphanumeric()||c=='-'||c=='_') {
        return Err("Gateway character ID unsafe".into());
    }
    let hash=data.get("contentHash").and_then(Value::as_str)
        .ok_or("Character model content hash missing")?;
    if hash.len()!=64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Character model hash malformed".into());
    }
    let response=client()?.get(format!("{base}/v1/assets/{hash}")).send().await
        .map_err(|_|"Avatar model download unavailable".to_string())?;
    if !response.status().is_success() {
        return Err(format!("Character model unavailable ({})",response.status()));
    }
    if response.content_length().is_some_and(|v|v>32*1024*1024) {
        return Err("Character model exceeds 32MiB limit".into());
    }
    let bytes=response.bytes().await.map_err(|_|"Could not retrieve saved avatar model")?;
    if bytes.len()>32*1024*1024 {
        return Err("Character model exceeds 32MiB limit".into());
    }
    let actual=format!("{:x}",sha2::Sha256::digest(&bytes));
    if !actual.eq_ignore_ascii_case(hash) {
        return Err("Saved avatar content hash mismatch; refusing restore".into());
    }
    Ok(Some((definition.clone(),bytes.to_vec())))
}

pub(crate) async fn issue_game_ticket(character_id:&str)->Result<String,String> {
    let token=session_token()?;
    let url=format!("{}/v1/tickets",gateway_base()?);
    let res=client()?.post(url).bearer_auth(token)
        .json(&json!({"characterId":character_id}))
        .send().await.map_err(|_|"Unable to request game admission ticket".to_string())?;
    if !res.status().is_success() {return Err(format!("Game ticket denied ({})",res.status()));}
    let data:Value=res.json().await.map_err(|_|"Malformed game ticket response")?;
    let ticket=data.get("ticket").and_then(Value::as_str).ok_or("Missing game ticket")?;
    if ticket.len()<32 {return Err("Short game admission credential".into());}
    Ok(ticket.to_string())
}
