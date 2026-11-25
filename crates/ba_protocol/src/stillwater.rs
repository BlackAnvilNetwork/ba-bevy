//! World coordinates and owner-visible state for the opening marsh.
use crate::{ashworks::Construct, Pose};
use serde::{Deserialize, Serialize};

pub const POST: Pose = at(300., 93.);
pub const GATE: Pose = at(300., -98.);
pub const REST: Pose = at(600., 4.);
// Separated from Pell and Cael so one interaction always has one destination.
pub const GREYMERE_ROAD: Pose = at(17.5, -5.5);
pub const LANTERN_ROAD: Pose = at(300., 103.);
pub const SABLE_DOCK: Pose = at(600., 15.);
pub const TRAVEL_REACH: f32 = 4.5;

/// Cael's field forge at Sable Rest - anvil and fire for smithing.
pub const SABLE_FORGE: Pose = at(598., 1.5);
/// Reach for interacting with crafting stations.
pub const CRAFT_REACH: f32 = 5.0;

/// Crafting station types available in the world.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CraftingStation {
    /// Forge with anvil - metal, precious metal, stone work
    Forge,
    /// Workbench - wood, bone work
    Workbench,
    /// Loom - cloth, leather work
    Loom,
    /// Alchemy table - essence work
    Alchemy,
}

impl CraftingStation {
    pub fn name(&self) -> &'static str {
        match self {
            CraftingStation::Forge => "Forge",
            CraftingStation::Workbench => "Workbench",
            CraftingStation::Loom => "Loom",
            CraftingStation::Alchemy => "Alchemy Table",
        }
    }
}

/// Check which crafting station (if any) is within reach of the given position.
pub fn nearby_station(x: f32, z: f32) -> Option<CraftingStation> {
    let dist_to_forge = ((x - SABLE_FORGE.x).powi(2) + (z - SABLE_FORGE.z).powi(2)).sqrt();
    if dist_to_forge <= CRAFT_REACH {
        return Some(CraftingStation::Forge);
    }
    None
}
pub const SHRINE: Pose = at(246., -29.);
pub const BLIND: Pose = at(350., 33.);
pub const OX: u32 = 70010;
pub const EDDA: u32 = 70011;
pub const BOOK: &str = "Edda's Book";
pub const YOKE: &str = "Ox's Yoke";
pub const HERONS: [Pose; 3] = [at(273., 65.), at(320., -48.), at(307., -95.)];
pub const NODES: [(Pose, &str); 9] = [
    (at(285., 71.), "Stillwater Essence"),
    (at(280., 62.), "Cold Iron"),
    (at(284., 57.), "Peat"),
    (at(306., -38.), "Bone Splinters"),
    (at(309., -45.), "Peat"),
    (at(312., -53.), "Cold Iron"),
    (at(305., -60.), "Bone Splinters"),
    (at(299., -66.), "Peat"),
    (at(294., -71.), "Cold Iron"),
];
pub const fn at(x: f32, z: f32) -> Pose {
    Pose {
        x,
        y: 1.65,
        z,
        yaw: 0.,
    }
}
pub fn marsh(x: f32) -> bool {
    (210.0..390.0).contains(&x)
}
pub fn rest(x: f32) -> bool {
    x > 550.
}

/// Terrain height in metres. Courtyards stay flat; bowls rise at the rim so the
/// mesh edge is hidden but the sky stays open.
pub fn ground(x: f32, z: f32) -> f32 {
    if rest(x) {
        return sable_ground(x, z);
    }
    if x < 80. {
        return greymere_ground(x, z);
    }
    let in_skirt = (145.0..470.0).contains(&x);
    if !marsh(x) && !in_skirt {
        return 0.;
    }
    let hills = if marsh(x) {
        7.4 * (x * 0.016).sin() * (z * 0.012).cos()
            + 5.2 * ((x + 22.) * 0.025).sin()
            + 6.1 * ((z - 9.) * 0.019).cos()
            + 2.8 * ((x * 0.7 + z) * 0.037).sin()
            + 3.2
    } else {
        0.
    };
    let dx = x - 300.;
    let dz = z - 5.;
    let r = dx.hypot(dz);
    let ang = dz.atan2(dx);
    let wobble = 1. + 0.18 * (ang * 2.7).sin() + 0.12 * (ang * 5.1 + 1.3).cos();
    let rise = (r - 82. * wobble).max(0.);
    let wall = (rise * 0.48 + rise * rise * 0.015).min(38.);
    let mut h = hills + wall;
    for (px, pz) in [
        (POST.x, POST.z),
        (GATE.x, GATE.z),
        (SHRINE.x, SHRINE.z),
        (BLIND.x, BLIND.z),
        (LANTERN_ROAD.x, LANTERN_ROAD.z),
    ] {
        let d = (x - px).hypot(z - pz);
        if d < 18. {
            h *= (d / 18.).powi(2);
        }
    }
    h.max(0.)
}

fn greymere_ground(x: f32, z: f32) -> f32 {
    let hills = 0.58 * (x * 0.19).sin() * (z * 0.17).cos()
        + 0.46 * ((x + 5.) * 0.27).sin()
        + 0.38 * ((z - 2.) * 0.23).cos();
    let dx = x - 8.;
    let dz = z - 1.2;
    let r = dx.hypot(dz);
    let ang = dz.atan2(dx);
    // Soft rim past the walkable rectangle so the floor stays flat and the
    // hills are a backdrop, not a climb.
    let wobble = 1. + 0.08 * (ang * 2.6).sin() + 0.05 * (ang * 4.9 + 0.7).cos();
    let rise = (r - 38. * wobble).max(0.);
    let wall = (rise * 0.45 + rise * rise * 0.028).min(14.);
    let mut h = hills.max(0.) + wall;
    for (px, pz, rad) in [
        (0., -1.45, 8.),
        (0.2, 6.4, 5.5),
        (GREYMERE_ROAD.x, GREYMERE_ROAD.z, 7.5),
        (1.45, 0.15, 4.5),
    ] {
        let d = (x - px).hypot(z - pz);
        if d < rad {
            h *= (d / rad).powi(2);
        }
    }
    h.max(0.)
}

fn sable_ground(x: f32, z: f32) -> f32 {
    let dx = x - 600.;
    let r = dx.hypot(z);
    let ang = z.atan2(dx);
    let wobble = 1. + 0.16 * (ang * 2.4).sin() + 0.1 * (ang * 5.2 + 0.4).cos();
    let rise = (r - 16. * wobble).max(0.);
    let mut wall = (rise * 1.05 + rise * rise * 0.055).min(14.);
    for (px, pz, rad) in [
        (600., 0., 14.),
        (REST.x, REST.z, 8.),
        (SABLE_DOCK.x, SABLE_DOCK.z, 7.),
    ] {
        let d = (x - px).hypot(z - pz);
        if d < rad {
            wall *= (d / rad).powi(2);
        }
    }
    wall.max(0.)
}

pub fn stand(x: f32, z: f32) -> f32 {
    crate::PLAYER_HEIGHT + ground(x, z)
}
pub fn bounds(x: f32) -> (f32, f32, f32, f32) {
    if marsh(x) {
        (198., 412., -148., 158.)
    } else if rest(x) {
        (580., 620., -20., 20.)
    } else {
        (
            crate::DEV_ZONE_MIN_X,
            crate::DEV_ZONE_MAX_X,
            crate::DEV_ZONE_MIN_Z,
            crate::DEV_ZONE_MAX_Z,
        )
    }
}

const fn foe(
    id: u32,
    name: &'static str,
    x: f32,
    z: f32,
    hp: i32,
    pulse: i32,
    xp: u64,
) -> Construct {
    Construct {
        id,
        name,
        x,
        z,
        hp,
        pulse,
        xp,
    }
}
pub const FOES: [Construct; 14] = [
    foe(70000, "Mire Hound", 294., 78., 52, 9, 105),
    foe(70001, "Mire Hound", 283., 72., 52, 9, 105),
    foe(70002, "Bog Toad", 343., 40., 60, 12, 110),
    foe(70003, "Bog Toad", 351., 38., 60, 12, 110),
    foe(70004, "Bog Toad", 354., 30., 60, 12, 110),
    foe(70005, "Bone-Rattler", 301., -44., 40, 7, 95),
    foe(70006, "Bone-Rattler", 304., -46., 40, 7, 95),
    foe(70007, "Bone-Rattler", 307., -44., 40, 7, 95),
    foe(70008, "Marsh Eel", 315., -56., 65, 14, 130),
    foe(70009, "Marsh Eel", 285., -62., 65, 14, 130),
    foe(OX, "Drowned Ox", 335., -4., 360, 30, 365),
    foe(EDDA, "Edda", 299., -87., 300, 25, 300),
    foe(70012, "Mire Hound", 330., 0., 45, 8, 25),
    foe(70013, "Mire Hound", 340., 0., 45, 8, 25),
];
#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
pub enum Tell {
    #[default]
    Normal,
    Heavy,
    Unblockable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Enemy {
    pub submerged: bool,
    pub grasp: bool,
    pub id: u32,
    pub x: f32,
    pub z: f32,
    pub yaw: f32,
    pub hp: i32,
    pub windup: f32,
    pub tell: Tell,
    pub radius: f32,
    pub line: bool,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Drop {
    pub x: f32,
    pub z: f32,
    pub items: Vec<String>,
    pub rare: bool,
    pub age: f32,
}
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Rarity {
    Common,
    Rare,
    Legendary,
    Ancient,
    Primal,
}
impl Drop {
    pub fn rarity(&self) -> Rarity {
        self.items
            .iter()
            .map(|item| match item.as_str() {
                "Primal Bog Amber" => Rarity::Primal,
                "Ancient Bog Amber" => Rarity::Ancient,
                "Golden Bog Amber" | "Star Iron" => Rarity::Legendary,
                _ if self.rare => Rarity::Rare,
                _ => Rarity::Common,
            })
            .max()
            .unwrap_or(Rarity::Common)
    }
}
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MarshView {
    pub enemies: Vec<Enemy>,
    pub drops: Vec<Drop>,
    pub inventory: std::collections::BTreeMap<String, u32>,
    pub rain: bool,
    pub roll_progress: f32,
    pub stamina: u8,
    pub rolling: bool,
    pub quest: String,
    pub notice: String,
    pub crossed: bool,
    pub dead: bool,
}
