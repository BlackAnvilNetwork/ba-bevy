//! Black Anvil identity gateway.
//!
//! A player's **only game login** is Discord OAuth. Game/character authorization
//! is an independent account database, not a Discord username and not a wallet.
//! The zone redeems single-use character-bound tickets using a private service
//! credential. No client may supply authoritative account/character identity.

mod auth;
mod characters;
mod store;

use std::net::{IpAddr, SocketAddr};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH, Duration};

use axum::{
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::RngCore;
use rusqlite::{Connection, OptionalExtension};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tracing::{error, info};

pub(crate) const MAX_GLB_BYTES: usize = 32 * 1024 * 1024;
pub(crate) const MAX_CHARACTER_JSON: usize = 128 * 1024;

#[derive(Clone)]
pub(crate) struct Settings {
    bind: SocketAddr,
    gateway_public_url: String,
    discord_client_id: String,
    discord_client_secret: String,
    discord_redirect_uri: String,
    internal_token: String,
    avatar_dir: PathBuf,
    db_path: PathBuf,
}

impl Settings {
    fn from_env() -> Result<Self, String> {
        let bind: SocketAddr = std::env::var("BA_GATEWAY_BIND")
            .unwrap_or_else(|_| "127.0.0.1:8790".into())
            .parse().map_err(|_| "Invalid BA_GATEWAY_BIND")?;
        let gateway_public_url = std::env::var("BA_GATEWAY_PUBLIC_URL")
            .unwrap_or_else(|_| "http://127.0.0.1:8790".into())
            .trim_end_matches('/').to_string();
        if !bind.ip().is_loopback() && !gateway_public_url.starts_with("https://") {
            return Err("Non-loopback gateway requires an HTTPS public URL and a TLS reverse proxy".into());
        }
        let internal_token = std::env::var("BA_INTERNAL_TOKEN").unwrap_or_default();
        if internal_token.len() < 32 {
            return Err("Set a random BA_INTERNAL_TOKEN with at least 32 characters".into());
        }
        Ok(Self {
            bind,
            gateway_public_url,
            discord_client_id: std::env::var("BA_DISCORD_CLIENT_ID").unwrap_or_default(),
            discord_client_secret: std::env::var("BA_DISCORD_CLIENT_SECRET").unwrap_or_default(),
            discord_redirect_uri: std::env::var("BA_DISCORD_REDIRECT_URI").unwrap_or_default(),
            internal_token,
            avatar_dir: PathBuf::from(std::env::var("BA_AVATAR_DIR").unwrap_or_else(|_| "ba-avatars".into())),
            db_path: PathBuf::from(std::env::var("BA_GATEWAY_DB").unwrap_or_else(|_| "ba-gateway.sqlite".into())),
        })
    }
}

#[derive(Clone)]
pub(crate) struct Gateway {
    config: Arc<Settings>,
    db: Arc<Mutex<Connection>>,
    http: reqwest::Client,
}

#[derive(Debug)]
pub(crate) struct ApiError(StatusCode, &'static str);

impl ApiError {
    fn bad(message: &'static str) -> Self { Self(StatusCode::BAD_REQUEST, message) }
    fn auth() -> Self { Self(StatusCode::UNAUTHORIZED, "unauthorized") }
    fn forbidden() -> Self { Self(StatusCode::FORBIDDEN, "forbidden") }
    fn conflict(message: &'static str) -> Self { Self(StatusCode::CONFLICT, message) }
    fn unavailable() -> Self { Self(StatusCode::SERVICE_UNAVAILABLE, "service_unavailable") }
    fn internal() -> Self { Self(StatusCode::INTERNAL_SERVER_ERROR, "internal_error") }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        (self.0, Json(json!({ "error": self.1 }))).into_response()
    }
}

impl From<rusqlite::Error> for ApiError {
    fn from(e: rusqlite::Error) -> Self {
        error!("gateway database operation failed: {e}");
        ApiError::internal()
    }
}

pub(crate) type ApiResult<T> = Result<T, ApiError>;

pub(crate) fn now() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs() as i64
}

pub(crate) fn random_secret() -> String {
    let mut bytes = [0u8; 32];
    rand::rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub(crate) fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(crate) fn token_hash(secret: &str) -> String {
    digest(secret.as_bytes())
}

pub(crate) fn account_id_for_session(gateway: &Gateway, headers: &HeaderMap) -> ApiResult<String> {
    let raw = headers.get(axum::http::header::AUTHORIZATION)
        .and_then(|h| h.to_str().ok()).ok_or_else(ApiError::auth)?;
    let token = raw.strip_prefix("Bearer ").ok_or_else(ApiError::auth)?;
    if token.len() < 32 || token.len() > 256 { return Err(ApiError::auth()); }
    let hash = token_hash(token);
    let db = gateway.db.lock().map_err(|_| ApiError::internal())?;
    let id = db.query_row(
        "SELECT s.account_id FROM sessions s JOIN accounts a ON a.id=s.account_id
         WHERE s.token_hash=?1 AND s.expires_at>?2 AND s.revoked=0 AND a.disabled=0",
        rusqlite::params![hash, now()],
        |row| row.get::<_, String>(0),
    ).optional()?;
    id.ok_or_else(ApiError::auth)
}

pub(crate) fn require_internal(gateway: &Gateway, headers: &HeaderMap) -> ApiResult<()> {
    use sha2::Digest;
    let supplied = headers.get("x-ba-internal-token").and_then(|v| v.to_str().ok())
        .ok_or_else(ApiError::auth)?;
    if supplied.len() < 32 || supplied.len() > 256 { return Err(ApiError::auth()); }
    // Constant-time comparison of fixed-length hashes instead of direct secret strings.
    use subtle::ConstantTimeEq;
    let expected = Sha256::digest(gateway.config.internal_token.as_bytes());
    let actual = Sha256::digest(supplied.as_bytes());
    if bool::from(expected.as_slice().ct_eq(actual.as_slice())) { Ok(()) } else { Err(ApiError::auth()) }
}

fn router(app: Gateway) -> Router {
    Router::new()
        .route("/health", get(|| async { Json(json!({ "status": "ok" })) }))
        .route("/v1/auth/start", post(auth::start))
        .route("/v1/auth/callback", get(auth::callback))
        .route("/v1/auth/poll", post(auth::poll))
        .route("/v1/me", get(auth::me))
        .route("/v1/auth/logout", post(auth::logout))
        .route("/v1/characters", post(characters::save_character))
        .route("/v1/characters/selected", get(characters::selected))
        .route("/v1/assets/{hash}", get(characters::download_asset))
        .route("/v1/tickets", post(characters::issue_ticket))
        .route("/v1/internal/redeem", post(characters::redeem_ticket))
        .route("/v1/internal/reconnect", post(characters::reconnect))
        .route("/v1/internal/state", post(characters::save_world_state))
        .layer(axum::extract::DefaultBodyLimit::max(MAX_GLB_BYTES * 4 / 3 + MAX_CHARACTER_JSON + 65536))
        .with_state(app)
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt().with_env_filter(
        tracing_subscriber::EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "ba_gateway=info,tower_http=warn".into())
    ).init();

    let config = Settings::from_env()?;
    if let Some(parent) = config.db_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::create_dir_all(&config.avatar_dir)?;
    let db = Connection::open(&config.db_path)?;
    store::initialize(&db)?;
    let http = reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .redirect(reqwest::redirect::Policy::none())
        .build()?;
    let addr = config.bind;
    let app = Gateway { config: Arc::new(config), db: Arc::new(Mutex::new(db)), http };
    let listener = tokio::net::TcpListener::bind(addr).await?;
    info!("Black Anvil gateway listening at {addr} (Discord OAuth server)");
    axum::serve(listener, router(app)).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn random_credentials_are_opaque_and_hashed() {
        let a = random_secret();
        let b = random_secret();
        assert_ne!(a,b);
        assert_eq!(a.len(),43);
        assert_ne!(token_hash(&a),a);
    }
}
