//! Private authenticated gateway interface for the authoritative zone.
//! Game tickets are issued *only* after Discord account authentication.
//! The zone never trusts a client's appearance, job, account or spawn in
//! authenticated mode. Private HTTP is confined to loopback in the MVP.

use std::time::Duration;
use ba_protocol::{AppearanceWire,Pose};
use reqwest::Client;
use serde::{Deserialize,Serialize};
use serde_json::{json,Value};

#[derive(Clone)]
pub enum AdmissionMode {
    Authenticated(GatewayClient),
    /// Explicit localhost smoke/developer mode. Never internet facing.
    Development,
}

#[derive(Clone)]
pub struct GatewayClient {
    base: String,
    internal: String,
    http: Client,
}

#[derive(Debug,Clone,Deserialize)]
#[serde(rename_all="camelCase")]
pub struct Admitted {
    pub account_id: String,
    pub character_id: String,
    pub appearance: AppearanceWire,
    pub pose: Pose,
    pub hp: i32,
    pub xp: u64,
    pub level: u32,
    pub inventory: Value,
    pub state_revision: u64,
    pub reconnect_token: String,
}

#[derive(Debug,Clone,Serialize)]
#[serde(rename_all="camelCase")]
pub struct WorldSave {
    pub account_id: String,
    pub character_id: String,
    pub state_revision: u64,
    pub pose: Pose,
    pub hp: i32,
    pub xp: u64,
    pub level: u32,
    pub inventory: Value,
}

impl AdmissionMode {
    pub fn from_env() -> Result<Self,String> {
        let internal = std::env::var("BA_INTERNAL_TOKEN").unwrap_or_default();
        let gateway = std::env::var("BA_GATEWAY_INTERNAL_URL").unwrap_or_default();
        if !internal.is_empty() || !gateway.is_empty() {
            if internal.len()<32 {
                return Err("BA_INTERNAL_TOKEN must have at least 32 characters".into());
            }
            let base = if gateway.is_empty() { "http://127.0.0.1:8790".to_string() }
                else { gateway.trim_end_matches('/').to_string() };
            let url = reqwest::Url::parse(&base).map_err(|_| "Invalid BA_GATEWAY_INTERNAL_URL")?;
            if url.scheme()!="https" && !(url.scheme()=="http" && 
                    url.host_str().is_some_and(|host| host=="127.0.0.1" || host=="localhost" || host=="::1")) {
                return Err("Gateway internal connection requires HTTPS or loopback HTTP".into());
            }
            let http = Client::builder().timeout(Duration::from_secs(8))
                .redirect(reqwest::redirect::Policy::none())
                .build().map_err(|e| e.to_string())?;
            return Ok(Self::Authenticated(GatewayClient{base,internal,http}));
        }
        if std::env::var("BA_DEV_ALLOW_UNAUTH").as_deref()==Ok("1") {
            return Ok(Self::Development);
        }
        Err("No gateway credentials. Configure Discord gateway or set BA_DEV_ALLOW_UNAUTH=1 for localhost-only testing".into())
    }

    pub fn gateway(&self) -> Option<&GatewayClient> {
        if let Self::Authenticated(value)=self {Some(value)} else {None}
    }

    pub fn dev(&self) -> bool {matches!(self, Self::Development)}
}

impl GatewayClient {
    async fn request(
        &self, route:&str, body:Value,
    ) -> Result<reqwest::Response,String> {
        let response=self.http.post(format!("{}{}",self.base,route))
            .header("x-ba-internal-token",&self.internal)
            .json(&body).send().await
            .map_err(|_| "gateway_unreachable".to_string())?;
        if !response.status().is_success() {
            return Err(format!("gateway_rejected_{}",response.status().as_u16()));
        }
        Ok(response)
    }

    pub async fn admit(
        &self, ticket: Option<&str>, reconnect: Option<&str>,
    ) -> Result<Admitted,String> {
        let (route,body)=if let Some(key)=reconnect {
            ("/v1/internal/reconnect",json!({"reconnectToken":key}))
        } else if let Some(key)=ticket {
            ("/v1/internal/redeem",json!({"ticket":key}))
        } else {
            return Err("missing_game_ticket".into());
        };
        let response=self.request(route,body).await?;
        response.json::<Admitted>().await.map_err(|_| "malformed_gateway_admission".into())
    }

    pub async fn save(&self,state: WorldSave) -> Result<(),String> {
        self.request("/v1/internal/state",serde_json::to_value(state)
            .map_err(|_| "invalid_state".to_string())?).await?;
        Ok(())
    }
}
