//! Native Discord login. The system browser returns to a loopback listener in
//! this process. The gateway exchanges the code with the client secret and
//! this launcher stores only the Black Anvil session.
//! Public settings are compiled from launcher/config.toml.

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::Mutex;
use std::time::{Duration,Instant};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Deserialize;
use serde_json::{json,Value};

static LOGIN: Mutex<Option<PendingLogin>> = Mutex::new(None);
static GATEWAY: Mutex<Option<Child>> = Mutex::new(None);

/// In a debug launcher, start the local identity gateway if port 8790 is closed.
/// Settings come from `gateway.local.env` at the repo root. A release build
/// never spawns a gateway; it uses the compiled public URL.
pub(crate) fn ensure_dev_gateway() {
    if !cfg!(debug_assertions) { return; }
    let address = std::net::SocketAddr::from(([127, 0, 0, 1], 8790));
    if std::net::TcpStream::connect_timeout(&address, Duration::from_millis(250)).is_ok() {
        return;
    }
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let Some(repo) = manifest.parent().and_then(|parent| parent.parent()).map(|path| path.to_path_buf()) else { return };
    let binary = repo.join("target").join("debug").join(if cfg!(windows) { "ba_gateway.exe" } else { "ba_gateway" });
    if !binary.is_file() {
        eprintln!("Discord gateway binary is not built: {}", binary.display());
        return;
    }
    match Command::new(&binary).current_dir(&repo).spawn() {
        Ok(child) => {
            if let Ok(mut slot) = GATEWAY.lock() { *slot = Some(child); }
            let deadline = Instant::now() + Duration::from_secs(8);
            while Instant::now() < deadline {
                if std::net::TcpStream::connect_timeout(&address, Duration::from_millis(200)).is_ok() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(150));
            }
            eprintln!("Discord gateway did not accept connections on 127.0.0.1:8790");
        }
        Err(err) => eprintln!("Could not start Discord gateway: {err}"),
    }
}

struct PendingLogin {
    state: String,
    listener: TcpListener,
    started_at: Instant,
}

struct LauncherPublicConfig {
    discord_client_id: String,
    discord_redirect_uri: String,
    gateway_url: String,
}

#[derive(Deserialize)]
#[serde(rename_all="camelCase")]
struct StartResponse {
    authorize_url: String,
    state: String,
}

fn parse_public_config(text: &str) -> Result<LauncherPublicConfig, String> {
    let mut discord_client_id = String::new();
    let mut discord_redirect_uri = None;
    let mut gateway_url = None;
    for raw in text.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        let Some((key, value)) = line.split_once('=') else { continue };
        let value = value.trim().trim_matches('"').trim_matches('\'');
        match key.trim() {
            "discord_client_id" => discord_client_id = value.to_string(),
            "discord_redirect_uri" => discord_redirect_uri = Some(value.to_string()),
            "gateway_url" => gateway_url = Some(value.to_string()),
            _ => {}
        }
    }
    Ok(LauncherPublicConfig {
        discord_client_id,
        discord_redirect_uri: discord_redirect_uri.ok_or("launcher/config.toml is missing discord_redirect_uri")?,
        gateway_url: gateway_url.ok_or("launcher/config.toml is missing gateway_url")?,
    })
}

fn public_config() -> Result<LauncherPublicConfig, String> {
    parse_public_config(include_str!("../../config.toml"))
}

pub(crate) fn loopback_callback_addr(redirect: &str) -> Result<SocketAddr, String> {
    let url = reqwest::Url::parse(redirect).map_err(|_| "Invalid discord_redirect_uri".to_string())?;
    if url.scheme() != "http" || url.host_str() != Some("127.0.0.1") || url.path() != "/callback" {
        return Err("discord_redirect_uri must be http://127.0.0.1:<port>/callback".into());
    }
    let port = url.port().ok_or("discord_redirect_uri is missing a port")?;
    Ok(SocketAddr::from(([127, 0, 0, 1], port)))
}

fn callback_query(request: &str) -> Result<Vec<(String, String)>, String> {
    let mut parts = request.lines().next().unwrap_or("").split_whitespace();
    let method = parts.next().unwrap_or("");
    let target = parts.next().unwrap_or("");
    if method != "GET" { return Err("Discord callback must be a GET".into()); }
    let url = reqwest::Url::parse(&format!("http://127.0.0.1{target}"))
        .map_err(|_| "Malformed Discord callback".to_string())?;
    if url.path() != "/callback" { return Err("Discord callback arrived on an unexpected path".into()); }
    Ok(url.query_pairs().map(|(key, value)| (key.into_owned(), value.into_owned())).collect())
}

fn write_html(stream: &mut TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\nContent-Length: {}\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes());
}

fn accept_loopback_callback(listener: TcpListener, expected_state: &str) -> Result<(String, String), String> {
    listener.set_nonblocking(true).map_err(|err| err.to_string())?;
    let deadline = Instant::now() + Duration::from_secs(180);
    let (mut stream, peer) = loop {
        match listener.accept() {
            Ok(pair) => break pair,
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline { return Err("Discord sign-in timed out. Try again.".into()); }
                std::thread::sleep(Duration::from_millis(40));
            }
            Err(err) => return Err(format!("Discord callback listener failed: {err}")),
        }
    };
    if !peer.ip().is_loopback() { return Err("Discord callback came from a non-local address.".into()); }
    stream.set_nonblocking(false).map_err(|err| err.to_string())?;
    stream.set_read_timeout(Some(Duration::from_secs(5))).map_err(|err| err.to_string())?;
    let mut buf = [0u8; 8192];
    let mut collected = Vec::new();
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                collected.extend_from_slice(&buf[..n]);
                if collected.len() > 8192 { return Err("OAuth callback request is too large.".into()); }
                if collected.windows(4).any(|window| window == b"\r\n\r\n") { break; }
            }
            Err(err) => return Err(format!("Could not read the Discord callback: {err}")),
        }
    }
    let request = String::from_utf8(collected).map_err(|_| "Discord callback was not valid text.".to_string())?;
    let pairs = callback_query(&request)?;
    let value = |name: &str| pairs.iter().find(|(key, _)| key == name).map(|(_, item)| item.clone());
    let declined = "<!doctype html><html><body style=\"font:18px system-ui;padding:4rem;background:#121212;color:#eee\"><h2>Discord sign-in stopped</h2><p>Return to the Black Anvil launcher.</p></body></html>";
    let ready = "<!doctype html><html><body style=\"font:18px system-ui;padding:4rem;background:#121212;color:#eee\"><h2>Black Anvil</h2><p>Return to the Black Anvil launcher.</p></body></html>";
    if value("state").as_deref() != Some(expected_state) {
        write_html(&mut stream, "400 Bad Request", declined);
        return Err("Discord returned a sign-in state this launcher did not start.".into());
    }
    if value("iss").as_deref() != Some("https://discord.com") {
        write_html(&mut stream, "400 Bad Request", declined);
        return Err("Discord did not identify itself as the authorization server.".into());
    }
    if value("error").is_some() {
        write_html(&mut stream, "200 OK", declined);
        return Err("Discord authorization was declined.".into());
    }
    let code = value("code").filter(|code| !code.is_empty() && code.len() <= 512)
        .ok_or("Missing Discord authorization code.")?;
    write_html(&mut stream, "200 OK", ready);
    Ok((code, expected_state.to_string()))
}

fn discord_authorize_matches(config: &LauncherPublicConfig, authorize_url: &str, state: &str) -> Result<(), String> {
    let url = reqwest::Url::parse(authorize_url).map_err(|_| "Invalid Discord authorization URL".to_string())?;
    if url.scheme() != "https" || url.host_str() != Some("discord.com") || url.path() != "/oauth2/authorize" {
        return Err("Untrusted Discord authorization URL".into());
    }
    let pairs: Vec<(String, String)> = url.query_pairs().map(|(key, value)| (key.into_owned(), value.into_owned())).collect();
    let get = |name: &str| pairs.iter().find(|(key, _)| key == name).map(|(_, value)| value.as_str());
    if get("response_type") != Some("code") || get("scope") != Some("identify") || get("code_challenge_method") != Some("S256") {
        return Err("Discord authorization URL is missing the authorization-code grant.".into());
    }
    if get("client_id") != Some(config.discord_client_id.as_str()) {
        return Err("Discord application id does not match launcher/config.toml.".into());
    }
    if get("redirect_uri") != Some(config.discord_redirect_uri.as_str()) {
        return Err("Discord redirect does not match launcher/config.toml. Register http://127.0.0.1:17384/callback.".into());
    }
    if get("state") != Some(state) {
        return Err("Discord login state did not match the gateway response.".into());
    }
    Ok(())
}

pub(crate) fn gateway_base() -> Result<String,String> {
    let base = public_config()?.gateway_url.trim_end_matches('/').to_string();
    if !cfg!(debug_assertions) && (base.starts_with("http://127.0.0.1") || base.starts_with("http://localhost")) {
        return Err("Release launcher/config.toml still points at the local gateway".into());
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

/// Binds the loopback redirect, then returns Discord's authorize URL.
/// The authorization code never enters the React UI.
#[tauri::command]
pub async fn begin_discord_login()->Result<Value,String> {
    ensure_dev_gateway();
    let config = public_config()?;
    if config.discord_client_id.is_empty() {
        return Err("Set discord_client_id in launcher/config.toml to the Discord application id.".into());
    }
    let address = loopback_callback_addr(&config.discord_redirect_uri)?;
    let url=format!("{}/v1/auth/start",gateway_base()?);
    let resp=client()?.post(url).send().await
        .map_err(|_|"Discord gateway unavailable. The local gateway is not listening on 127.0.0.1:8790.".to_string())?;
    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        if body.contains("discord_application_not_configured") {
            let redirect = serde_json::from_str::<Value>(&body).ok()
                .and_then(|value| value.get("redirectUri").and_then(Value::as_str).map(str::to_string))
                .unwrap_or_else(|| "http://127.0.0.1:17384/callback".into());
            return Err(format!(
                "Discord application is not configured on the gateway. Register redirect {redirect} and set the gateway client id and client secret."
            ));
        }
        return Err(format!("Gateway rejected sign-in ({status})"));
    }
    let start: StartResponse=resp.json().await.map_err(|_|"Invalid login response".to_string())?;
    discord_authorize_matches(&config, &start.authorize_url, &start.state)?;
    if let Ok(mut slot) = LOGIN.lock() { *slot = None; }
    let listener = TcpListener::bind(address).map_err(|_| {
        format!("OAuth callback port {} is already in use.", address.port())
    })?;
    *LOGIN.lock().map_err(|_|"Login lock poisoned")?=Some(PendingLogin {
        state: start.state, listener, started_at: Instant::now(),
    });
    Ok(json!({"authorizationUrl":start.authorize_url,"status":"pending"}))
}

#[tauri::command]
pub async fn await_discord_login()->Result<Value,String> {
    let pending=LOGIN.lock().map_err(|_|"Login lock poisoned")?.take()
        .ok_or("No Discord login in progress")?;
    if pending.started_at.elapsed() > Duration::from_secs(180) {
        return Err("Discord sign-in timed out. Try again.".into());
    }
    let expected = pending.state;
    let listener = pending.listener;
    let (code, state) = tauri::async_runtime::spawn_blocking(move || {
        accept_loopback_callback(listener, &expected)
    }).await.map_err(|_| "Discord callback waiter stopped".to_string())??;
    let url=format!("{}/v1/auth/complete",gateway_base()?);
    let resp=client()?.post(url).json(&json!({"code":code,"state":state,"iss":"https://discord.com"}))
        .send().await.map_err(|_|"Discord gateway unavailable during code exchange".to_string())?;
    if !resp.status().is_success() {
        let body = resp.text().await.unwrap_or_default();
        if body.contains("expired_or_replayed_discord_state") {
            return Err("This Discord sign-in expired. Try again.".into());
        }
        if body.contains("discord_code_exchange_failed") {
            return Err("Discord rejected the authorization code. Try again.".into());
        }
        return Err("Discord sign-in could not be completed.".into());
    }
    let mut result: Value=resp.json().await.map_err(|_|"Invalid Discord login response".to_string())?;
    let token=result.get("sessionToken").and_then(Value::as_str)
        .ok_or("Missing game session")?;
    credential()?.set_password(token).map_err(|_|"Unable to store game session securely")?;
    result.as_object_mut().unwrap().remove("sessionToken");
    result["connected"]=json!(true);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn public_config_keeps_the_loopback_redirect() {
        let config = parse_public_config(include_str!("../../config.toml")).unwrap();
        assert_eq!(config.discord_redirect_uri, "http://127.0.0.1:17384/callback");
        assert!(config.gateway_url.starts_with("http://127.0.0.1:8790"));
        assert_eq!(loopback_callback_addr(&config.discord_redirect_uri).unwrap().port(), 17384);
        assert!(loopback_callback_addr("http://localhost:17384/callback").is_err());
        assert!(loopback_callback_addr("https://127.0.0.1:17384/callback").is_err());
    }

    #[test]
    fn callback_requires_discord_issuer_and_code() {
        let request = "GET /callback?code=abc&state=state-value&iss=https%3A%2F%2Fdiscord.com HTTP/1.1\r\nHost: 127.0.0.1:17384\r\n\r\n";
        let pairs = callback_query(request).unwrap();
        assert!(pairs.contains(&("code".into(), "abc".into())));
        assert!(pairs.contains(&("iss".into(), "https://discord.com".into())));
        assert!(callback_query("GET /v1/auth/callback?code=abc HTTP/1.1\r\n\r\n").is_err());
        assert!(callback_query("POST /callback?code=abc HTTP/1.1\r\n\r\n").is_err());
    }

    #[test]
    fn authorize_url_must_match_compiled_public_settings() {
        let config = LauncherPublicConfig {
            discord_client_id: "157730590492196864".into(),
            discord_redirect_uri: "http://127.0.0.1:17384/callback".into(),
            gateway_url: "http://127.0.0.1:8790".into(),
        };
        let url = "https://discord.com/oauth2/authorize?response_type=code&client_id=157730590492196864&scope=identify&state=state-value&redirect_uri=http%3A%2F%2F127.0.0.1%3A17384%2Fcallback&prompt=consent&code_challenge=abc&code_challenge_method=S256";
        assert!(discord_authorize_matches(&config, url, "state-value").is_ok());
        assert!(discord_authorize_matches(&config, url, "other-state").is_err());
    }
}
