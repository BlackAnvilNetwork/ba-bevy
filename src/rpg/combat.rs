//! Port of ba-stick `packages/combat/src/index.ts` combat helpers.
//!
//! Canonical calculations: `src/rpg/COMBAT_CALCULATIONS.md` (§1–37). Prefer those
//! formulas when replacing the provisional helpers below.

use super::grades::CombatStats;
use super::jobs::JobId;

pub const SWING_BASE_MS: u32 = 1600;
pub const MAGIC_RESIST_FLOOR: f32 = 0.55;
pub const MAGIC_RESIST_K: f32 = 180.0;
pub const MAB_PCT_PER_POINT: f32 = 0.01;
pub const MAB_MAX_BONUS: f32 = 1.0;
pub const FREE_STAT_POINTS: u32 = 5;
pub const SKILL_POINTS_PER_LEVEL: u32 = 1;
pub const SKILL_PRESTIGE_CAP: u32 = 100;
/// Server level-up VFX window (`levelUpUntil = now + 2400`).
pub const LEVEL_UP_MS: u32 = 2400;

pub fn defense_from_vit(vit: i32) -> i32 {
    (vit as f32 * 1.5).floor() as i32
}

pub fn player_defense(vit: i32, gear_def: i32) -> i32 {
    defense_from_vit(vit) + gear_def.max(0)
}

pub fn magic_defense_from_mnd(mnd: i32) -> i32 {
    if mnd <= 0 {
        0
    } else {
        (mnd as f32 * 1.2).floor() as i32
    }
}

pub fn player_magic_defense(mnd: i32, gear_mdb: i32) -> i32 {
    magic_defense_from_mnd(mnd) + gear_mdb.max(0)
}

pub fn magic_resist_mul(mag_def: i32) -> f32 {
    let def = mag_def.max(0) as f32;
    if def <= 0.0 {
        return 1.0;
    }
    let shaved = def / (def + MAGIC_RESIST_K);
    (1.0 - shaved).max(MAGIC_RESIST_FLOOR)
}

pub fn mitigate_magic_damage(raw: i32, mag_def: i32) -> i32 {
    if raw <= 0 {
        return 0;
    }
    ((raw as f32) * magic_resist_mul(mag_def)).floor().max(1.0) as i32
}

pub fn magic_attack_mul(mab: i32) -> f32 {
    1.0 + ((mab.max(0) as f32) * MAB_PCT_PER_POINT).min(MAB_MAX_BONUS)
}

pub fn apply_magic_attack_bonus(dmg: i32, mab: i32) -> i32 {
    ((dmg as f32) * magic_attack_mul(mab)).floor().max(1.0) as i32
}

pub fn evasion_from_agi(agi: i32) -> i32 {
    agi / 2
}

pub fn crit_from_agi(agi: i32) -> f32 {
    agi.max(0) as f32 * 0.001
}

pub fn hit_chance(acc: i32, eva: i32) -> f32 {
    (0.75 + (acc - eva) as f32 / 200.0).clamp(0.2, 0.95)
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

/// Mob kill XP share (server `rewardMobKill`).
pub fn mob_kill_xp(mob_level: u32, player_level: u32) -> u32 {
    let pl = player_level.max(1);
    (200u32 * mob_level / pl).min(200)
}

/// Opening beat: defeating possessed Edda + claiming her ledger.
pub const WITNESS_ASH_XP: u32 = 500;
pub const BOOK_MEMORY_XP: u32 = 120;

/// Weapon skill TP cost (ba-stick `WEAPON_TP_COST`).
pub const WEAPON_TP_COST: i32 = 1000;
