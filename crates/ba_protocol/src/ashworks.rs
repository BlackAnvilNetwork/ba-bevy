//! Shared Greymere encounter landmarks and server-to-client presentation.
use serde::{Deserialize, Serialize};

pub const FORGE: (f32, f32) = (-2.05, 0.35);
pub const WARD_COST: u32 = 2;

pub struct Construct {
    pub id: u32,
    pub name: &'static str,
    pub x: f32,
    pub z: f32,
    pub hp: i32,
    pub pulse: i32,
    pub xp: u64,
}
pub const CONSTRUCTS: [Construct; 2] = [
    Construct {
        id: u32::MAX,
        name: "Clinker Imp",
        x: 1.6,
        z: 4.8,
        hp: 90,
        pulse: 8,
        xp: 125,
    },
    Construct {
        id: u32::MAX - 1,
        name: "Kiln Warden",
        x: 22.0,
        z: -1.5,
        hp: 340,
        pulse: 32,
        xp: 300,
    },
];
pub const PULSE_RADIUS: f32 = 2.8;
pub const WINDUP_TICKS: u64 = 20;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConstructState {
    pub id: u32,
    pub hp: i32,
    pub winding_up: bool,
}

#[derive(Clone, Debug, PartialEq, Default, Serialize, Deserialize)]
pub struct AshworksView {
    pub constructs: Vec<ConstructState>,
    pub cinder: u32,
    pub ward: bool,
    pub heart: bool,
    pub edge: bool,
}
