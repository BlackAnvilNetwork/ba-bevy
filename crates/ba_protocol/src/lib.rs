//! Shared Black Anvil zone protocol (JSON over WebSocket).
//!
//! Auth is intentionally not wallet-based. Discord/session tickets land in M3.
//! This crate is a **dev/loopback prototype** until ticket verification lands.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PROTOCOL_VERSION: u32 = 4;
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


/**
 * Shared deterministic-ish movement kernel used by server authority and
 * client presentation prediction. These are dev-zone bounds until zones
 * provide versioned navigation/collision manifests.
 */
pub const WALK_SPEED: f32 = 3.5;
pub const SPRINT_SPEED: f32 = 6.5;
pub const DEV_ZONE_MIN_X: f32 = -20.0;
pub const DEV_ZONE_MAX_X: f32 = 20.0;
pub const DEV_ZONE_MIN_Z: f32 = -20.0;
pub const DEV_ZONE_MAX_Z: f32 = 20.0;
pub const PLAYER_HEIGHT: f32 = 1.65;
pub const TICK_DT: f32 = 1.0 / TICK_HZ as f32;

/// One authoritative movement sample, *not* a client position.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MoveAxes {
    pub forward: f32,
    pub strafe: f32,
    pub yaw: f32,
    pub sprint: bool,
}

impl MoveAxes {
    pub fn valid(self) -> bool {
        self.forward.is_finite()
            && self.strafe.is_finite()
            && self.yaw.is_finite()
            && self.forward.abs() <= 1.0
            && self.strafe.abs() <= 1.0
    }
}

/// Integrate exactly one bounded simulation step. The server remains authoritative;
/// the client uses this function only to predict and replay unacknowledged inputs.
pub fn integrate_movement(pose: Pose, axes: MoveAxes, dt: f32) -> Pose {
    if !pose.is_finite() || !axes.valid() || !dt.is_finite() || !(0.0..=TICK_DT).contains(&dt) {
        return pose;
    }
    let (s, c) = axes.yaw.sin_cos();
    let mut dx = -s * axes.forward + c * axes.strafe;
    let mut dz = -c * axes.forward - s * axes.strafe;
    let magnitude = (dx * dx + dz * dz).sqrt();
    if magnitude > 1.0 {
        dx /= magnitude;
        dz /= magnitude;
    }
    let speed = if axes.sprint { SPRINT_SPEED } else { WALK_SPEED };
    Pose {
        x: (pose.x + dx * speed * dt).clamp(DEV_ZONE_MIN_X, DEV_ZONE_MAX_X),
        y: PLAYER_HEIGHT,
        z: (pose.z + dz * speed * dt).clamp(DEV_ZONE_MIN_Z, DEV_ZONE_MAX_Z),
        yaw: axes.yaw,
    }
}

#[cfg(test)]
mod movement_tests {
    use super::*;

    fn origin() -> Pose { Pose::default() }
    fn axes(forward: f32, strafe: f32) -> MoveAxes {
        MoveAxes { forward, strafe, yaw: 0.0, sprint: true }
    }

    #[test]
    fn forward_and_diagonal_move_at_same_max_speed() {
        let forward = integrate_movement(origin(), axes(1.0, 0.0), TICK_DT);
        let diagonal = integrate_movement(origin(), axes(1.0, 1.0), TICK_DT);
        let d = (diagonal.x.powi(2) + diagonal.z.powi(2)).sqrt();
        assert!((forward.z.abs() - SPRINT_SPEED * TICK_DT).abs() < 0.00001);
        assert!((d - SPRINT_SPEED * TICK_DT).abs() < 0.00001);
    }

    #[test]
    fn invalid_inputs_do_not_advance_position() {
        for a in [axes(f32::NAN, 0.0), axes(2.0, 0.0), axes(0.0, f32::INFINITY)] {
            assert_eq!(integrate_movement(origin(), a, TICK_DT), origin());
        }
        assert_eq!(integrate_movement(origin(), axes(1.0, 0.0), 100.0), origin());
    }

    #[test]
    fn never_exits_zone_and_is_repeatable() {
        let sample = axes(1.0, 1.0);
        let mut a = origin();
        let mut b = origin();
        for _ in 0..1000 {
            a = integrate_movement(a, sample, TICK_DT);
            b = integrate_movement(b, sample, TICK_DT);
        }
        assert_eq!(a,b);
        assert!((DEV_ZONE_MIN_X..=DEV_ZONE_MAX_X).contains(&a.x));
        assert!((DEV_ZONE_MIN_Z..=DEV_ZONE_MAX_Z).contains(&a.z));
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
    /// Last movement input that the server applied during a simulation tick.
    pub last_input_seq: u64,
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
    /// Player intent, never an authoritative destination or position.
    MoveIntent {
        seq: u64,
        forward: f32,
        strafe: f32,
        yaw: f32,
        sprint: bool,
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
