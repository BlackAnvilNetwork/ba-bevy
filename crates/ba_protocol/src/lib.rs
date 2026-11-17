//! Shared Black Anvil zone protocol (JSON over WebSocket).
//!
//! Auth is intentionally not wallet-based. Discord/session tickets land in M3.
//! This crate is a **dev/loopback prototype** until ticket verification lands.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PROTOCOL_VERSION: u32 = 3;
pub const DEFAULT_WS_URL: &str = "ws://127.0.0.1:8787";
pub const TICK_HZ: u32 = 20;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TraitRefWire {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

/// Compact appearance payload for replication (not the full BaCharacterV1 file).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AppearanceWire {
    pub character_id: String,
    pub name: String,
    pub starting_job: String,
    pub manifest_id: String,
    #[serde(default)]
    pub body_traits: BTreeMap<String, TraitRefWire>,
    #[serde(default)]
    pub starter_equipment: BTreeMap<String, TraitRefWire>,
    /// Optional content hash (sha256 hex) for future signed asset cache lookup.
    /// Do not treat as a filesystem path — clients must never load untrusted paths.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub glb_content_hash: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Pose {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub yaw: f32,
}

impl Default for Pose {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 1.65,
            z: 0.0,
            yaw: 0.0,
        }
    }
}

impl Pose {
    pub fn is_finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite() && self.yaw.is_finite()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    pub player_id: u32,
    pub pose: Pose,
    pub appearance: AppearanceWire,
    pub hp: i32,
    pub max_hp: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    Hello {
        protocol: u32,
        appearance: AppearanceWire,
        /// Hint only — server chooses spawn for admission.
        pose: Pose,
    },
    Move {
        pose: Pose,
    },
    /// Engage sticky target for real-time auto-attacks. `target_id: 0` = nearest.
    Attack {
        target_id: u32,
    },
    /// Hotbar ability / WS by id (server validates job + range + resources).
    Cast {
        ability_id: String,
    },
    Ping {
        client_time_ms: u64,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    Welcome {
        protocol: u32,
        player_id: u32,
        tick: u64,
        tick_hz: u32,
        spawn: Pose,
    },
    Snapshot {
        tick: u64,
        players: Vec<PlayerState>,
    },
    Damage {
        tick: u64,
        attacker_id: u32,
        target_id: u32,
        damage: i32,
        hit: bool,
        crit: bool,
        target_hp: i32,
    },
    Heal {
        tick: u64,
        caster_id: u32,
        target_id: u32,
        amount: i32,
        target_hp: i32,
    },
    BuffApplied {
        tick: u64,
        caster_id: u32,
        ability_id: String,
        atk_mul: Option<f32>,
        duration_s: f32,
    },
    CastRejected {
        tick: u64,
        ability_id: String,
        reason: String,
    },
    Pong {
        client_time_ms: u64,
        server_time_ms: u64,
    },
    Reject {
        reason: String,
    },
}

impl ClientMessage {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}

impl ServerMessage {
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    pub fn from_json(s: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(s)
    }
}
