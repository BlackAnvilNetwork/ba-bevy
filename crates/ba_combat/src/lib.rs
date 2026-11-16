//! Pure combat helpers shared by the zone server and Bevy.
//! Formulas mirrored from `src/rpg/combat.rs` + `src/rpg/fight.rs`
//! (see `COMBAT_CALCULATIONS.md`).

mod kits;

pub use kits::{ensure_kits_loaded, kits, kits_or_load, load_kits_from, KitCatalog};

use rand::Rng;
use serde::{Deserialize, Serialize};

/// Melee reach from `src/rpg/fight.rs`.
pub const AUTO_REACH: f32 = 2.9;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "PascalCase")]
pub enum JobId {
    #[default]
    TimeMage,
    Knight,
    Rogue,
    Cleric,
    Sorcerer,
    Fighter,
    BattleMage,
    Druid,
}

impl JobId {
    pub fn parse(s: &str) -> Self {
        match s {
            "Knight" => Self::Knight,
            "Rogue" => Self::Rogue,
            "Cleric" => Self::Cleric,
            "Sorcerer" => Self::Sorcerer,
            "Fighter" => Self::Fighter,
            "BattleMage" => Self::BattleMage,
            "Druid" => Self::Druid,
            _ => Self::TimeMage,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CombatStats {
    pub str: i32,
    pub dex: i32,
    pub vit: i32,
    pub agi: i32,
    pub int: i32,
    pub mnd: i32,
}

/// Level-1 main-job sheet for M2 until full grade curves live in this crate.
pub fn starter_stats(job: JobId) -> CombatStats {
    match job {
        JobId::TimeMage => CombatStats {
            str: 8,
            dex: 9,
            vit: 9,
            agi: 10,
            int: 12,
            mnd: 13,
        },
        JobId::Knight => CombatStats {
            str: 12,
            dex: 9,
            vit: 13,
            agi: 8,
            int: 8,
            mnd: 10,
        },
        JobId::Rogue => CombatStats {
            str: 10,
            dex: 13,
            vit: 9,
            agi: 12,
            int: 8,
            mnd: 8,
        },
        JobId::Cleric => CombatStats {
            str: 9,
            dex: 8,
            vit: 11,
            agi: 8,
            int: 10,
            mnd: 13,
        },
        JobId::Sorcerer => CombatStats {
            str: 8,
            dex: 9,
            vit: 8,
            agi: 10,
            int: 13,
            mnd: 11,
        },
        JobId::Fighter => CombatStats {
            str: 13,
            dex: 10,
            vit: 11,
            agi: 9,
            int: 8,
            mnd: 8,
        },
        JobId::BattleMage => CombatStats {
            str: 11,
            dex: 10,
            vit: 10,
            agi: 9,
            int: 12,
            mnd: 10,
        },
        JobId::Druid => CombatStats {
            str: 10,
            dex: 9,
            vit: 11,
            agi: 10,
            int: 11,
            mnd: 12,
        },
    }
}

pub fn starter_max_hp(job: JobId) -> i32 {
    let vit = starter_stats(job).vit;
    120 + vit * 8
}

pub fn defense_from_vit(vit: i32) -> i32 {
    (vit as f32 * 1.5).floor() as i32
}

pub fn evasion_from_agi(agi: i32) -> i32 {
    agi / 2
}

pub fn hit_chance(acc: i32, eva: i32) -> f32 {
    (0.75 + (acc - eva) as f32 / 200.0).clamp(0.2, 0.95)
}

pub fn crit_from_agi(agi: i32) -> f32 {
    agi.max(0) as f32 * 0.001
}

pub fn f_str(attacker_str: i32, target_vit: i32) -> i32 {
    (attacker_str - target_vit).clamp(-10, 20)
}

pub fn physical_damage(att: i32, def: i32, fstr: i32, crit: bool) -> i32 {
    let ratio = (att as f32 / (def.max(1) as f32)).clamp(0.1, 2.5);
    let mut dmg = ((8 + fstr) as f32 * ratio).floor() as i32;
    if crit {
        dmg = (dmg as f32 * 1.25).floor() as i32;
    }
    dmg.max(1)
}

pub fn melee_f_str(job: JobId, stats: &CombatStats, target_vit: i32, flux: bool) -> i32 {
    match job {
        JobId::TimeMage if flux => f_str(stats.mnd, target_vit),
        JobId::Rogue => f_str(stats.str + stats.dex / 4, target_vit),
        JobId::Sorcerer | JobId::Cleric => f_str(stats.str, target_vit).max(-1),
        _ => f_str(stats.str, target_vit),
    }
}

/// Matches `Fighter::swing_period` in `src/rpg/fight.rs`.
pub fn swing_period(haste: f32) -> f32 {
    let haste = haste.clamp(0.0, 0.80);
    (0.72 * (1.0 - haste)).max(0.28)
}

#[derive(Clone, Copy, Debug)]
pub struct TargetRef {
    pub vit: i32,
    pub defense: i32,
    pub evasion: i32,
    pub distance: f32,
}

impl TargetRef {
    pub fn from_defender(stats: &CombatStats, distance: f32) -> Self {
        Self {
            vit: stats.vit,
            defense: defense_from_vit(stats.vit),
            evasion: evasion_from_agi(stats.agi),
            distance,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SwingOutcome {
    OutOfRange,
    Miss,
    Hit { damage: i32, crit: bool },
}

/// Mirror of `src/rpg/fight.rs::resolve_auto_attack` (no aether/flux yet on server).
pub fn resolve_auto_attack(
    job: JobId,
    stats: &CombatStats,
    atk_mul: f32,
    target: TargetRef,
    rng_hit: f32,
    rng_crit: f32,
) -> SwingOutcome {
    if target.distance > AUTO_REACH {
        return SwingOutcome::OutOfRange;
    }
    let acc = stats.dex + 12;
    if rng_hit >= hit_chance(acc, target.evasion) {
        return SwingOutcome::Miss;
    }
    let crit_bonus = if job == JobId::Rogue { 0.08 } else { 0.0 };
    let crit = rng_crit < crit_from_agi(stats.agi) + crit_bonus;
    let fstr = melee_f_str(job, stats, target.vit, false);
    let atk = ((stats.str + 14) as f32 * atk_mul).round() as i32;
    let damage = physical_damage(atk, target.defense, fstr, crit);
    SwingOutcome::Hit { damage, crit }
}

pub fn roll_auto_attack(
    job: JobId,
    stats: &CombatStats,
    target: TargetRef,
    rng: &mut impl Rng,
) -> SwingOutcome {
    resolve_auto_attack(
        job,
        stats,
        1.0,
        target,
        rng.random::<f32>(),
        rng.random::<f32>(),
    )
}

#[derive(Clone, Debug)]
pub struct AbilitySpec {
    pub id: String,
    pub potency: i32,
    pub mp: i32,
    pub range: f32,
    pub kind: AbilityKind,
    pub recast: f32,
    pub atk_mul: Option<f32>,
    pub duration: f32,
    pub tp_cost: i32,
}

impl AbilitySpec {
    fn legacy(
        id: &str,
        potency: i32,
        mp: i32,
        range: f32,
        kind: AbilityKind,
        recast: f32,
    ) -> Self {
        Self {
            id: id.to_string(),
            potency,
            mp,
            range,
            kind,
            recast,
            atk_mul: None,
            duration: 0.0,
            tp_cost: 0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AbilityKind {
    PhysicalJa,
    Magic,
    Heal,
    Buff,
    WeaponSkill,
}

/// Prefer kits.json hotbar; fall back to compact legacy list.
pub fn starter_hotbar(job: JobId) -> Vec<AbilitySpec> {
    if let Some(cat) = kits_or_load() {
        let bar = cat.hotbar(job);
        if !bar.is_empty() {
            return bar;
        }
    }
    legacy_hotbar(job)
}

fn legacy_hotbar(job: JobId) -> Vec<AbilitySpec> {
    match job {
        JobId::Fighter => vec![
            AbilitySpec::legacy("berserk", 0, 0, 0.0, AbilityKind::Buff, 60.0),
            AbilitySpec::legacy(
                "killing_storm",
                120,
                0,
                AUTO_REACH + 0.4,
                AbilityKind::PhysicalJa,
                8.0,
            ),
        ],
        JobId::TimeMage => vec![
            AbilitySpec::legacy("temporal_distortion", 95, 28, 12.0, AbilityKind::Magic, 6.0),
            AbilitySpec::legacy("cure", 40, 12, 0.0, AbilityKind::Heal, 3.0),
        ],
        JobId::Knight => vec![AbilitySpec::legacy(
            "shield_bash",
            70,
            0,
            AUTO_REACH + 0.4,
            AbilityKind::PhysicalJa,
            5.0,
        )],
        JobId::Rogue => vec![AbilitySpec::legacy(
            "backblade",
            90,
            0,
            AUTO_REACH + 0.4,
            AbilityKind::PhysicalJa,
            6.0,
        )],
        JobId::Cleric => vec![
            AbilitySpec::legacy("cl_banish", 80, 18, 12.0, AbilityKind::Magic, 4.0),
            AbilitySpec::legacy("cl_cure", 45, 12, 0.0, AbilityKind::Heal, 3.0),
        ],
        JobId::Sorcerer => vec![AbilitySpec::legacy(
            "sc_ember_lance",
            100,
            22,
            14.0,
            AbilityKind::Magic,
            4.0,
        )],
        JobId::BattleMage => vec![AbilitySpec::legacy(
            "arc_slash",
            85,
            10,
            AUTO_REACH + 0.4,
            AbilityKind::PhysicalJa,
            5.0,
        )],
        JobId::Druid => vec![AbilitySpec::legacy(
            "dr_sour_hum",
            75,
            16,
            12.0,
            AbilityKind::Magic,
            5.0,
        )],
    }
}

pub fn find_ability(job: JobId, id: &str) -> Option<AbilitySpec> {
    if let Some(cat) = kits_or_load() {
        if let Some(spec) = cat.resolve(job, id) {
            return Some(spec);
        }
    }
    legacy_hotbar(job).into_iter().find(|a| a.id == id)
}

/// Mirrors elemental / physical JA / heal potency math from `fight.rs`.
pub fn resolve_ability_potency(
    stats: &CombatStats,
    spec: &AbilitySpec,
    target: Option<TargetRef>,
) -> SwingOutcome {
    match spec.kind {
        AbilityKind::Heal => {
            let amount = spec.potency + stats.mnd / 2 + stats.vit / 4;
            SwingOutcome::Hit {
                damage: -amount, // negative = heal on server
                crit: false,
            }
        }
        AbilityKind::PhysicalJa => {
            let Some(t) = target else {
                return SwingOutcome::OutOfRange;
            };
            if t.distance > spec.range.max(AUTO_REACH) {
                return SwingOutcome::OutOfRange;
            }
            let base = spec.potency as f32 + stats.str as f32 * 0.8 + stats.vit as f32 * 0.3;
            let damage = base.floor().max(1.0) as i32;
            SwingOutcome::Hit {
                damage,
                crit: false,
            }
        }
        AbilityKind::Magic => {
            let Some(t) = target else {
                return SwingOutcome::OutOfRange;
            };
            if t.distance > spec.range {
                return SwingOutcome::OutOfRange;
            }
            let base =
                spec.potency as f32 * 0.52 + stats.int as f32 * 2.85 + stats.mnd as f32 * 0.35;
            let raw = base.floor().max(1.0) as i32;
            // Simplified MDB resist (combat.rs mitigate_magic_damage floor path)
            let mag_def = (t.vit as f32 * 1.2).floor() as i32;
            let shaved = mag_def.max(0) as f32 / (mag_def.max(0) as f32 + 180.0);
            let mul = (1.0 - shaved).max(0.55);
            let damage = ((raw as f32) * mul).floor().max(1.0) as i32;
            SwingOutcome::Hit {
                damage,
                crit: false,
            }
        }
        AbilityKind::WeaponSkill => {
            let Some(t) = target else {
                return SwingOutcome::OutOfRange;
            };
            if t.distance > spec.range.max(AUTO_REACH) {
                return SwingOutcome::OutOfRange;
            }
            // ftp×100 stored as potency; scale like a heavy physical JA.
            let base = spec.potency as f32 * 0.85 + stats.str as f32 * 1.1 + stats.vit as f32 * 0.25;
            let damage = base.floor().max(1.0) as i32;
            SwingOutcome::Hit {
                damage,
                crit: false,
            }
        }
        AbilityKind::Buff => SwingOutcome::Hit {
            damage: 0,
            crit: false,
        },
    }
}
