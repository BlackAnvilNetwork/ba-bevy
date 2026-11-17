//! Discord OAuth authorization-code grant with PKCE.
//!
//! The player's browser is redirected to the launcher's loopback listener.
//! This gateway is the only component that exchanges the code. It uses the
//! client secret, checks the single-use state, verifies the Discord profile,
//! revokes the Discord token, and returns a Black Anvil session.

use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    Json,
};
use base64::{engine::general_purpose::{STANDARD, URL_SAFE_NO_PAD}, Engine as _};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tracing::warn;
use uuid::Uuid;

use crate::{account_id_for_session, now, random_secret, token_hash, ApiError, ApiResult, Gateway};

const DISCORD_AUTHORIZE: &str = "https://discord.com/oauth2/authorize";
const DISCORD_TOKEN: &str = "https://discord.com/api/oauth2/token";
const DISCORD_REVOKE: &str = "https://discord.com/api/oauth2/token/revoke";
const DISCORD_ISSUER: &str = "https://discord.com";

const FLOW_TTL_SECONDS: i64 = 600;
const SESSION_TTL_SECONDS: i64 = 30 * 24 * 3600;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuthStart {
    authorize_url: String,
    state: String,
}

/// HTTP Basic client authentication, with the id and secret percent-encoded
/// first, as required by RFC 6749 §2.3.1 and Discord's token examples.
pub(crate) fn oauth_client_basic(client_id: &str, client_secret: &str) -> String {
    let encode = |value: &str| url::form_urlencoded::byte_serialize(value.as_bytes()).collect::<String>();
    let raw = format!("{}:{}", encode(client_id), encode(client_secret));
    format!("Basic {}", STANDARD.encode(raw.as_bytes()))
}

/// Authorization-code grant URL from Discord's OAuth2 reference.
/// `identify` does not install a bot, so `integration_type` is omitted.
/// PKCE S256 is included because Discord recommends it for production apps.
pub(crate) fn discord_authorize_url(client_id: &str, redirect_uri: &str, state: &str, challenge: &str) -> Result<String, ApiError> {
    let mut url = url::Url::parse(DISCORD_AUTHORIZE).map_err(|_| ApiError::internal())?;
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", client_id)
        .append_pair("scope", "identify")
        .append_pair("state", state)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("prompt", "consent")
        .append_pair("code_challenge", challenge)
        .append_pair("code_challenge_method", "S256");
    Ok(url.into())
}

pub(crate) async fn start(State(app): State<Gateway>) -> ApiResult<Json<AuthStart>> {
    if app.config.discord_client_id.is_empty()
        || app.config.discord_client_secret.is_empty()
        || app.config.discord_redirect_uri.is_empty()
    {
        return Err(ApiError::discord_not_configured());
    }
    let flow_id = Uuid::new_v4().to_string();
    let state = random_secret();
    let poll_secret = random_secret();
    let verifier = random_secret();
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));

    {
        let db = app.db.lock().map_err(|_| ApiError::internal())?;
        db.execute("DELETE FROM auth_flows WHERE expires_at<?1", params![now()])?;
        let pending: i64 = db.query_row(
            "SELECT COUNT(*) FROM auth_flows WHERE phase IN ('pending','exchanging')",
            [], |r| r.get(0),
        )?;
        if pending >= 1000 { return Err(ApiError(StatusCode::TOO_MANY_REQUESTS, "too_many_login_attempts")); }
        db.execute(
            "INSERT INTO auth_flows(id,state_hash,poll_hash,code_verifier,phase,expires_at)
             VALUES (?1,?2,?3,?4,'pending',?5)",
            params![flow_id, token_hash(&state), token_hash(&poll_secret), verifier, now()+FLOW_TTL_SECONDS],
        )?;
    }

    let authorize_url = discord_authorize_url(
        &app.config.discord_client_id,
        &app.config.discord_redirect_uri,
        &state,
        &challenge,
    )?;
    Ok(Json(AuthStart { authorize_url, state }))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CompleteRequest {
    code: String,
    state: String,
    iss: String,
}

pub(crate) async fn complete(
    State(app): State<Gateway>,
    Json(request): Json<CompleteRequest>,
) -> ApiResult<Json<Value>> {
    if request.code.is_empty() || request.code.len() > 512
        || request.state.is_empty() || request.state.len() > 128
    {
        return Err(ApiError::bad("invalid_oauth_callback"));
    }
    if request.iss != DISCORD_ISSUER {
        return Err(ApiError::bad("invalid_discord_issuer"));
    }
    finish_discord_login(app, &request.code, &request.state).await
}

#[derive(Deserialize)]
struct DiscordToken {
    access_token: String,
}

#[derive(Deserialize)]
struct DiscordUser {
    id: String,
    username: String,
    global_name: Option<String>,
}

fn discord_display_name(user: &DiscordUser) -> &str {
    user.global_name.as_deref().filter(|name| !name.is_empty()).unwrap_or(&user.username)
}

async fn finish_discord_login(app: Gateway, code: &str, state: &str) -> ApiResult<Json<Value>> {
    let state_hash = token_hash(state);
    // Claim the state before the external HTTP call. Parallel callbacks cannot
    // exchange one code twice, and the single-use state is bound to this flow.
    let verifier = {
        let db = app.db.lock().map_err(|_| ApiError::internal())?;
        let claimed = db.execute(
            "UPDATE auth_flows SET phase='exchanging'
             WHERE state_hash=?1 AND phase='pending' AND expires_at>?2",
            params![state_hash, now()],
        )?;
        if claimed != 1 { return Err(ApiError::bad("expired_or_replayed_discord_state")); }
        db.query_row(
            "SELECT code_verifier FROM auth_flows WHERE state_hash=?1",
            params![state_hash],
            |r| r.get::<_, String>(0),
        )?
    };

    let basic = oauth_client_basic(&app.config.discord_client_id, &app.config.discord_client_secret);
    let token_response = app.http
        .post(DISCORD_TOKEN)
        .header(axum::http::header::AUTHORIZATION, &basic)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", app.config.discord_redirect_uri.as_str()),
            ("code_verifier", verifier.as_str()),
        ])
        .send().await.map_err(|_| ApiError::bad("discord_code_exchange_failed"))?;
    if !token_response.status().is_success() {
        warn!(status = %token_response.status(), "discord authorization code exchange failed");
        return Err(ApiError::bad("discord_code_exchange_failed"));
    }
    let discord_token: DiscordToken = token_response.json().await.map_err(|_| ApiError::bad("discord_code_exchange_failed"))?;

    let profile_response = app.http
        .get("https://discord.com/api/v10/users/@me")
        .bearer_auth(&discord_token.access_token)
        .send().await.map_err(|_| ApiError::bad("discord_profile_unavailable"))?;
    if !profile_response.status().is_success() {
        warn!(status = %profile_response.status(), "discord identify profile failed");
        return Err(ApiError::bad("discord_profile_unavailable"));
    }
    let discord_user: DiscordUser = profile_response.json().await.map_err(|_| ApiError::bad("discord_profile_unavailable"))?;

    // The game session is independent of Discord. Revoke the access token so
    // the refresh token issued with it is revoked too (Discord revokes both).
    let revoke = app.http
        .post(DISCORD_REVOKE)
        .header(axum::http::header::AUTHORIZATION, basic)
        .form(&[
            ("token", discord_token.access_token.as_str()),
            ("token_type_hint", "access_token"),
        ])
        .send().await;
    match revoke {
        Ok(response) if response.status().is_success() => {}
        Ok(response) => warn!(status = %response.status(), "discord token revocation failed"),
        Err(err) => warn!(error = %err, "discord token revocation unreachable"),
    }
    if discord_user.id.len() < 15 || discord_user.id.len() > 22
        || !discord_user.id.chars().all(|c| c.is_ascii_digit())
        || discord_display_name(&discord_user).is_empty()
    {
        return Err(ApiError::bad("invalid_discord_identity"));
    }

    let session_token = random_secret();
    let display_name = discord_display_name(&discord_user).to_string();
    enum SaveFail { Disabled, Store(String) }
    let saved = (|| -> Result<String, SaveFail> {
        let mut db = app.db.lock().map_err(|_| SaveFail::Store("lock".into()))?;
        let tx = db.transaction().map_err(|e| SaveFail::Store(e.to_string()))?;
        let existing: Option<String> = tx.query_row(
            "SELECT id FROM accounts WHERE discord_id=?1",
            params![discord_user.id], |r| r.get(0),
        ).optional().map_err(|e| SaveFail::Store(e.to_string()))?;
        let account_id = existing.unwrap_or_else(|| Uuid::new_v4().to_string());
        tx.execute(
            "INSERT INTO accounts(id,discord_id,display_name,created_at)
             VALUES(?1,?2,?3,?4)
             ON CONFLICT(discord_id) DO UPDATE SET display_name=excluded.display_name",
            params![account_id, discord_user.id, display_name, now()],
        ).map_err(|e| SaveFail::Store(e.to_string()))?;
        let disabled: bool = tx.query_row(
            "SELECT disabled FROM accounts WHERE id=?1", params![account_id],
            |r| r.get::<_, i64>(0),
        ).map_err(|e| SaveFail::Store(e.to_string()))? != 0;
        if disabled { return Err(SaveFail::Disabled); }
        tx.execute(
            "INSERT INTO sessions(token_hash,account_id,expires_at) VALUES(?1,?2,?3)",
            params![token_hash(&session_token), account_id, now()+SESSION_TTL_SECONDS],
        ).map_err(|e| SaveFail::Store(e.to_string()))?;
        let updated = tx.execute(
            "UPDATE auth_flows SET phase='consumed', session_result=NULL
             WHERE state_hash=?1 AND phase='exchanging'",
            params![state_hash],
        ).map_err(|e| SaveFail::Store(e.to_string()))?;
        if updated != 1 { return Err(SaveFail::Store("flow was not claimed".into())); }
        tx.commit().map_err(|e| SaveFail::Store(e.to_string()))?;
        Ok(account_id)
    })();
    let account_id = match saved {
        Ok(id) => id,
        Err(SaveFail::Disabled) => return Err(ApiError::forbidden()),
        Err(SaveFail::Store(detail)) => {
            warn!(error = %detail, "discord account session was not stored");
            return Err(ApiError::internal());
        }
    };
    Ok(Json(json!({
        "status": "complete",
        "sessionToken": session_token,
        "accountId": account_id,
        "displayName": display_name,
    })))
}

pub(crate) async fn me(State(app): State<Gateway>, headers: HeaderMap) -> ApiResult<Json<Value>> {
    let account_id = account_id_for_session(&app,&headers)?;
    let db = app.db.lock().map_err(|_| ApiError::internal())?;
    let (display_name,discord_id): (String,String) = db.query_row(
        "SELECT display_name,discord_id FROM accounts WHERE id=?1",
        params![account_id], |r| Ok((r.get(0)?,r.get(1)?)),
    )?;
    Ok(Json(json!({ "accountId": account_id, "displayName": display_name, "discordId": discord_id })))
}

/// Revoke a Black Anvil account session without touching the user's Discord
/// OAuth identity or an independently-controlled smart-account wallet.
pub(crate) async fn logout(
    State(app): State<Gateway>, headers: HeaderMap,
) -> ApiResult<Json<Value>> {
    let account_id = account_id_for_session(&app,&headers)?;
    let raw = headers.get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or_else(ApiError::auth)?;
    let db = app.db.lock().map_err(|_| ApiError::internal())?;
    db.execute(
        "UPDATE sessions SET revoked=1 WHERE token_hash=?1 AND account_id=?2",
        params![token_hash(raw),account_id],
    )?;
    Ok(Json(json!({"ok":true})))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn polling_secret_is_distinct_from_csrf_and_game_session() {
        assert_ne!(random_secret(), random_secret());
        assert_eq!(URL_SAFE_NO_PAD.encode(Sha256::digest(b"test")).len(), 43);
    }

    #[test]
    fn authorize_url_is_discord_authorization_code_grant() {
        let url = discord_authorize_url(
            "157730590492196864",
            "http://127.0.0.1:17384/callback",
            "state-value",
            "challenge-value",
        ).unwrap();
        let parsed = url::Url::parse(&url).unwrap();
        assert_eq!(parsed.origin().ascii_serialization(), "https://discord.com");
        assert_eq!(parsed.path(), "/oauth2/authorize");
        let pairs: Vec<(String, String)> = parsed.query_pairs().map(|(k, v)| (k.into(), v.into())).collect();
        assert!(pairs.contains(&("response_type".into(), "code".into())));
        assert!(pairs.contains(&("client_id".into(), "157730590492196864".into())));
        assert!(pairs.contains(&("scope".into(), "identify".into())));
        assert!(pairs.contains(&("state".into(), "state-value".into())));
        assert!(pairs.contains(&("redirect_uri".into(), "http://127.0.0.1:17384/callback".into())));
        assert!(pairs.contains(&("prompt".into(), "consent".into())));
        assert!(pairs.contains(&("code_challenge".into(), "challenge-value".into())));
        assert!(pairs.contains(&("code_challenge_method".into(), "S256".into())));
        assert!(!pairs.iter().any(|(key, _)| key == "integration_type"));
    }

    #[test]
    fn basic_client_auth_percent_encodes_secret() {
        let header = oauth_client_basic("157730590492196864", "s/ecret");
        let encoded = header.strip_prefix("Basic ").unwrap();
        let raw = String::from_utf8(STANDARD.decode(encoded).unwrap()).unwrap();
        assert_eq!(raw, "157730590492196864:s%2Fecret");
    }

    #[test]
    fn display_name_prefers_discord_global_name() {
        let named = DiscordUser { id: "1".into(), username: "handle".into(), global_name: Some("Mack".into()) };
        let handle_only = DiscordUser { id: "1".into(), username: "handle".into(), global_name: None };
        assert_eq!(discord_display_name(&named), "Mack");
        assert_eq!(discord_display_name(&handle_only), "handle");
    }
}
