//! World coordinates and owner-visible state for the opening marsh.
use crate::{ashworks::Construct, Pose};
use serde::{Deserialize, Serialize};

pub const POST: Pose = at(300., 93.);
pub const GATE: Pose = at(300., -98.);
pub const REST: Pose = at(600., 4.);
// Separated from Pell and Cael so one interaction always has one destination.
pub const GREYMERE_ROAD: Pose = at(28., -1.5);
pub const LANTERN_ROAD: Pose = at(300., 103.);
pub const SABLE_DOCK: Pose = at(600., 15.);
pub const TRAVEL_REACH: f32 = 4.5;
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
pub fn bounds(x: f32) -> (f32, f32, f32, f32) {
    if marsh(x) {
        (215., 385., -105., 105.)
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
