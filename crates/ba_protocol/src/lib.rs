//! Shared Black Anvil zone protocol (JSON over WebSocket).
//!
//! Auth is intentionally not wallet-based. Discord/session tickets land in M3.
//! This crate is a **dev/loopback prototype** until ticket verification lands.

pub mod ashworks;
pub mod stillwater;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const PROTOCOL_VERSION: u32 = 5;
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
/// Extra planar speed while chaining Space ghost-steps. Still below a second sprint.
pub const GHOST_STEP_BONUS: f32 = 1.2;
/// Second Space inside this window starts/refreshes the skip. Seconds.
pub const GHOST_CHAIN_WINDOW: f32 = 0.9;
/// How long a chained tap keeps the bonus. Seconds.
pub const GHOST_BOOST_SECS: f32 = 0.4;
pub const DEV_ZONE_MIN_X: f32 = -16.0;
pub const DEV_ZONE_MAX_X: f32 = 32.0;
pub const DEV_ZONE_MIN_Z: f32 = -14.0;
pub const DEV_ZONE_MAX_Z: f32 = 20.0;
pub const PLAYER_HEIGHT: f32 = 1.65;
/// Studio GLBs are foot-origin. Seat them this far above `ground()` so soles
/// rest on the dirt (bind mesh is ~1cm below the toe bones; the forest-floor
/// albedo reads as volume). Player eye height stays `PLAYER_HEIGHT`.
pub const AVATAR_GROUND_CLEARANCE: f32 = 0.05;
pub const TICK_DT: f32 = 1.0 / TICK_HZ as f32;

/// Local Y of a foot-origin avatar parented to a `stand()` actor.
#[inline]
pub fn avatar_local_y() -> f32 {
    -PLAYER_HEIGHT + AVATAR_GROUND_CLEARANCE
}

/// One authoritative movement sample, *not* a client position.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct MoveAxes {
    pub forward: f32,
    pub strafe: f32,
    pub yaw: f32,
    pub sprint: bool,
    /// Chained Space ghost-step. Presentation jump is unchanged; this only adds planar speed.
    pub ghost: bool,
}

/// Client-side chain of Space taps. Server trusts the resulting `MoveAxes.ghost` flag.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub struct GhostStepChain {
    last_tap: f32,
    boost_until: f32,
}

impl GhostStepChain {
    pub fn on_jump(&mut self, now: f32) {
        if now.is_finite() && self.last_tap > 0.0 && now - self.last_tap <= GHOST_CHAIN_WINDOW {
            self.boost_until = now + GHOST_BOOST_SECS;
        }
        if now.is_finite() {
            self.last_tap = now;
        }
    }

    pub fn active(self, now: f32) -> bool {
        now.is_finite() && now < self.boost_until
    }
}

pub fn movement_speed(sprint: bool, ghost: bool) -> f32 {
    let base = if sprint { SPRINT_SPEED } else { WALK_SPEED };
    if ghost {
        base + GHOST_STEP_BONUS
    } else {
        base
    }
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
    if axes.ghost && magnitude <= 1e-5 {
        dx = -s;
        dz = -c;
    } else if magnitude > 1.0 {
        dx /= magnitude;
        dz /= magnitude;
    }
    let speed = movement_speed(axes.sprint, axes.ghost);
    let (x0,x1,z0,z1)=stillwater::bounds(pose.x);
    let x = (pose.x + dx * speed * dt).clamp(x0, x1);
    let z = (pose.z + dz * speed * dt).clamp(z0, z1);
    Pose {
        x,
        y: stillwater::stand(x, z),
        z,
        yaw: axes.yaw,
    }
}

#[cfg(test)]
mod movement_tests {
    use super::*;

    fn origin() -> Pose { Pose::default() }
    fn axes(forward: f32, strafe: f32) -> MoveAxes {
        MoveAxes { forward, strafe, yaw: 0.0, sprint: true, ghost: false }
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
    fn movement_bounds_match_actual_greymere_zone_data() {
        // A shared hardcoded preview bounds constant is temporary. This guard
        // ensures the server/client kernel stays aligned with the real zone
        // until authoritative zone descriptors are sent in Welcome.
        let raw = include_str!("../../../assets/data/Zones/Zone_GreymereYard.json");
        let zone: serde_json::Value = serde_json::from_str(raw).unwrap();
        let bounds = &zone["bounds"];
        assert_eq!(bounds["minX"].as_f64().unwrap() as f32, DEV_ZONE_MIN_X);
        assert_eq!(bounds["maxX"].as_f64().unwrap() as f32, DEV_ZONE_MAX_X);
        assert_eq!(bounds["minZ"].as_f64().unwrap() as f32, DEV_ZONE_MIN_Z);
        assert_eq!(bounds["maxZ"].as_f64().unwrap() as f32, DEV_ZONE_MAX_Z);
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

    #[test]
    fn avatar_seats_soles_on_the_height_field() {
        assert!(avatar_local_y() > -PLAYER_HEIGHT);
        assert!((avatar_local_y() + PLAYER_HEIGHT - AVATAR_GROUND_CLEARANCE).abs() < 1e-6);
    }

    #[test]
    fn first_jump_does_not_boost_and_a_chain_does() {
        let mut chain = GhostStepChain::default();
        chain.on_jump(1.0);
        assert!(!chain.active(1.0));
        chain.on_jump(1.4);
        assert!(chain.active(1.4));
        assert!(chain.active(1.7));
        assert!(!chain.active(1.4 + GHOST_BOOST_SECS + 0.01));
        chain.on_jump(3.0);
        assert!(!chain.active(3.0));
    }

    #[test]
    fn chained_ghost_step_moves_standing_and_walks_slightly_faster() {
        let standing = MoveAxes { ghost: true, ..MoveAxes::default() };
        let hopped = integrate_movement(origin(), standing, TICK_DT);
        let d = (hopped.x.powi(2) + hopped.z.powi(2)).sqrt();
        assert!((d - movement_speed(false, true) * TICK_DT).abs() < 0.00001);

        let walk = MoveAxes { forward: 1.0, ..MoveAxes::default() };
        let ghost_walk = MoveAxes { forward: 1.0, ghost: true, ..MoveAxes::default() };
        let a = integrate_movement(origin(), walk, TICK_DT);
        let b = integrate_movement(origin(), ghost_walk, TICK_DT);
        let da = (a.x.powi(2) + a.z.powi(2)).sqrt();
        let db = (b.x.powi(2) + b.z.powi(2)).sqrt();
        assert!(db > da + 0.02);
        assert!(db < SPRINT_SPEED * TICK_DT);
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
    /// Server-owned character progression. Never accepted from clients.
    #[serde(default)]
    pub level: u32,
    #[serde(default)]
    pub xp: u64,
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
        /// Single-use game ticket issued for a Discord-owned character.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ticket: Option<String>,
        /// Scoped, rotating reconnect credential from prior Welcome.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reconnect_token: Option<String>,
    },
    /// Player intent, never an authoritative destination or position.
    MoveIntent {
        seq: u64,
        forward: f32,
        strafe: f32,
        yaw: f32,
        sprint: bool,
        #[serde(default)]
        ghost: bool,
    },
    /// Engage sticky target for real-time auto-attacks. `target_id: 0` = nearest.
    Attack {
        target_id: u32,
    },
    /// Hotbar ability / WS by id (server validates job + range + resources).
    Cast {
        ability_id: String,
    },
    Interact,
    Drink,
    Dodge { forward:f32, strafe:f32, yaw:f32 },
    Forge { recipe: ashworks::Recipe },
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
        /// One-hour scoped game reconnect token, never a Discord or AA key.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        reconnect_token: Option<String>,
    },
    Snapshot {
        tick: u64,
        players: Vec<PlayerState>,
        #[serde(default)]
        ashworks: ashworks::AshworksView,
        #[serde(default)]
        marsh: stillwater::MarshView,
    },
    Progression {
        tick: u64,
        character_id: String,
        xp: u64,
        level: u32,
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

#[cfg(test)]
mod session_protocol_tests {
    use super::*;

    #[test]
    fn ticket_and_reconnect_are_explicit_and_optional() {
        let hello=ClientMessage::Hello {
            protocol: PROTOCOL_VERSION, appearance: AppearanceWire {
                character_id:"test".into(),name:"Test".into(),starting_job:"Fighter".into(),
                manifest_id:"anata/male".into(),body_traits:Default::default(),
                starter_equipment:Default::default(),glb_content_hash:None,
            }, pose: Pose::default(),ticket:Some("single-use-secret".into()),reconnect_token:None,
        };
        let encoded=hello.to_json().unwrap();
        let recovered=ClientMessage::from_json(&encoded).unwrap();
        assert_eq!(hello,recovered);
        assert!(!encoded.contains("reconnect_token"));
    }
}
