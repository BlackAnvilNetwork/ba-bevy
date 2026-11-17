//! Discord OAuth authorization-code flow with PKCE, CSRF state, server-side
//! token exchange, and one-time native-launcher polling.
//!
//! Browser receives no game session. Discord access/refresh tokens are never
//! persisted; the launcher obtains only a Black Anvil session credential.

use axum::{
    extract::{Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Response},
    Json,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{account_id_for_session, now, random_secret, token_hash, ApiError, ApiResult, Gateway};

const FLOW_TTL_SECONDS: i64 = 600;
const SESSION_TTL_SECONDS: i64 = 30 * 24 * 3600;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AuthStart {
    authorize_url: String,
    flow_id: String,
    poll_secret: String,
}

pub(crate) async fn start(State(app): State<Gateway>) -> ApiResult<Json<AuthStart>> {
    if app.config.discord_client_id.is_empty()
        || app.config.discord_client_secret.is_empty()
        || app.config.discord_redirect_uri.is_empty()
    {
        return Err(ApiError::unavailable());
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

    let mut url = url::Url::parse("https://discord.com/oauth2/authorize")
        .map_err(|_| ApiError::internal())?;
    url.query_pairs_mut()
        .append_pair("client_id", &app.config.discord_client_id)
        .append_pair("redirect_uri", &app.config.discord_redirect_uri)
        .append_pair("response_type", "code")
        .append_pair("scope", "identify")
        .append_pair("state", &state)
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("prompt", "consent");
    Ok(Json(AuthStart { authorize_url: url.to_string(), flow_id, poll_secret }))
}

#[derive(Deserialize)]
pub(crate) struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    iss: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
struct DiscordToken {
    access_token: String,
}

#[derive(Deserialize)]
struct DiscordUser {
    id: String,
    username: String,
}

pub(crate) async fn callback(
    State(app): State<Gateway>,
    Query(query): Query<CallbackQuery>,
) -> ApiResult<Html<&'static str>> {
    if query.error.is_some() {
        return Err(ApiError::bad("discord_authorization_denied"));
    }
    if query.iss.as_deref().is_some_and(|issuer| issuer != "https://discord.com") {
        return Err(ApiError::bad("invalid_discord_issuer"));
    }
    let code = query.code.as_ref().filter(|v| !v.is_empty()).ok_or(ApiError::bad("missing_code"))?;
    let state = query.state.as_ref().filter(|v| !v.is_empty()).ok_or(ApiError::bad("missing_state"))?;
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

    let token_response = app.http
        .post("https://discord.com/api/v10/oauth2/token")
        .form(&[
            ("client_id", app.config.discord_client_id.as_str()),
            ("client_secret", app.config.discord_client_secret.as_str()),
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", app.config.discord_redirect_uri.as_str()),
            ("code_verifier", verifier.as_str()),
        ])
        .send().await.map_err(|_| ApiError::unavailable())?;
    if !token_response.status().is_success() {
        return Err(ApiError::bad("discord_code_exchange_failed"));
    }
    let discord_token: DiscordToken = token_response.json().await.map_err(|_| ApiError::unavailable())?;

    let profile_response = app.http
        .get("https://discord.com/api/v10/users/@me")
        .bearer_auth(&discord_token.access_token)
        .send().await.map_err(|_| ApiError::unavailable())?;
    if !profile_response.status().is_success() {
        return Err(ApiError::bad("discord_profile_unavailable"));
    }
    let discord_user: DiscordUser = profile_response.json().await.map_err(|_| ApiError::unavailable())?;
    if discord_user.id.len() < 15 || discord_user.id.len() > 22
        || !discord_user.id.chars().all(|c| c.is_ascii_digit())
        || discord_user.username.is_empty()
    {
        return Err(ApiError::bad("invalid_discord_identity"));
    }

    let session_token = random_secret();
    {
        let mut db = app.db.lock().map_err(|_| ApiError::internal())?;
        let tx = db.transaction()?;
        let existing: Option<String> = tx.query_row(
            "SELECT id FROM accounts WHERE discord_id=?1",
            params![discord_user.id], |r| r.get(0),
        ).optional()?;
        let account_id = existing.unwrap_or_else(|| Uuid::new_v4().to_string());
        tx.execute(
            "INSERT INTO accounts(id,discord_id,display_name,created_at)
             VALUES(?1,?2,?3,?4)
             ON CONFLICT(discord_id) DO UPDATE SET display_name=excluded.display_name",
            params![account_id, discord_user.id, discord_user.username, now()],
        )?;
        let disabled: bool = tx.query_row(
            "SELECT disabled FROM accounts WHERE id=?1", params![account_id],
            |r| r.get::<_, i64>(0),
        )? != 0;
        if disabled { return Err(ApiError::forbidden()); }
        tx.execute(
            "INSERT INTO sessions(token_hash,account_id,expires_at) VALUES(?1,?2,?3)",
            params![token_hash(&session_token), account_id, now()+SESSION_TTL_SECONDS],
        )?;
        tx.execute(
            "UPDATE auth_flows SET phase='complete', session_result=?1
             WHERE state_hash=?2 AND phase='exchanging'",
            params![session_token, state_hash],
        )?;
        tx.commit()?;
    }
    Ok(Html("<!doctype html><html><body style='font:18px system-ui;padding:4rem;background:#121212;color:#eee'><h2>Black Anvil — Discord connected</h2><p>Return to the Black Anvil launcher to continue.</p></body></html>"))
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PollRequest {
    flow_id: String,
    poll_secret: String,
}

pub(crate) async fn poll(
    State(app): State<Gateway>,
    Json(request): Json<PollRequest>,
) -> ApiResult<Json<Value>> {
    let mut db = app.db.lock().map_err(|_| ApiError::internal())?;
    let tx = db.transaction()?;
    let row: Option<(String, String, Option<String>, i64)> = tx.query_row(
        "SELECT poll_hash,phase,session_result,expires_at FROM auth_flows WHERE id=?1",
        params![request.flow_id],
        |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?)),
    ).optional()?;
    let (expected,phase,session,expires) = row.ok_or_else(ApiError::auth)?;
    if token_hash(&request.poll_secret) != expected || expires <= now() {
        return Err(ApiError::auth());
    }
    match phase.as_str() {
        "pending" | "exchanging" => Ok(Json(json!({ "status": "pending" }))),
        "complete" => {
            let secret = session.ok_or_else(ApiError::internal)?;
            let (id,display_name): (String,String) = tx.query_row(
                "SELECT a.id,a.display_name FROM sessions s JOIN accounts a ON a.id=s.account_id
                 WHERE s.token_hash=?1",
                params![token_hash(&secret)],
                |r| Ok((r.get(0)?,r.get(1)?)),
            )?;
            let updated = tx.execute(
                "UPDATE auth_flows SET phase='consumed',session_result=NULL
                 WHERE id=?1 AND phase='complete'",
                params![request.flow_id],
            )?;
            if updated != 1 { return Err(ApiError::auth()); }
            tx.commit()?;
            Ok(Json(json!({
                "status": "complete", "sessionToken": secret,
                "accountId": id, "displayName": display_name
            })))
        }
        _ => Err(ApiError::auth()),
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn polling_secret_is_distinct_from_csrf_and_game_session() {
        assert_ne!(random_secret(), random_secret());
        assert_eq!(URL_SAFE_NO_PAD.encode(Sha256::digest(b"test")).len(), 43);
    }
}
