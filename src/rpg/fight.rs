//! Real-time fighting surface — hotbar, TP, ability / WS resolution for all jobs.

use std::collections::HashMap;

use bevy::prelude::*;

use super::combat::{
    apply_magic_attack_bonus, hit_chance, melee_f_str, mitigate_magic_damage, physical_damage,
    crit_from_agi, WEAPON_TP_COST,
};
use super::grades::CombatStats;
use super::jobs::JobId;
use super::kits::{AbilityDef, CombatKits, WeaponSkillDef};
use super::progression::Progression;

pub const HOTBAR_SIZE: usize = 8;
pub const TP_MAX: f32 = 3000.0;
pub const AUTO_REACH: f32 = 2.9;
pub const SPELL_REACH: f32 = 14.0;

#[derive(Component, Clone, Debug)]
pub struct Fighter {
    pub swing_cd: f32,
    pub tp: f32,
    pub hotbar: Vec<String>,
    pub recast_until: HashMap<String, f32>,
    pub atk_mul: f32,
    pub atk_mul_until: f32,
    pub phys_dt: f32,
    pub phys_dt_until: f32,
    pub mag_dt: f32,
    pub mag_dt_until: f32,
    pub haste: f32,
    pub haste_until: f32,
    pub flux: bool,
    pub aether: bool,
    pub last_cast_label: String,
    pub last_cast_until: f32,
    pub resting: bool,
}

impl Default for Fighter {
    fn default() -> Self {
        Self {
            swing_cd: 0.0,
            tp: 0.0,
            hotbar: Vec::new(),
            recast_until: HashMap::new(),
            atk_mul: 1.0,
            atk_mul_until: 0.0,
            phys_dt: 1.0,
            phys_dt_until: 0.0,
            mag_dt: 1.0,
            mag_dt_until: 0.0,
            haste: 0.0,
            haste_until: 0.0,
            flux: false,
            aether: false,
            last_cast_label: String::new(),
            last_cast_until: 0.0,
            resting: false,
        }
    }
}

impl Fighter {
    pub fn tick_buffs(&mut self, now: f32) {
        if self.atk_mul_until > 0.0 && now >= self.atk_mul_until {
            self.atk_mul = 1.0;
            self.atk_mul_until = 0.0;
        }
        if self.phys_dt_until > 0.0 && now >= self.phys_dt_until {
            self.phys_dt = 1.0;
            self.phys_dt_until = 0.0;
        }
        if self.mag_dt_until > 0.0 && now >= self.mag_dt_until {
            self.mag_dt = 1.0;
            self.mag_dt_until = 0.0;
        }
        if self.haste_until > 0.0 && now >= self.haste_until {
            self.haste = 0.0;
            self.haste_until = 0.0;
        }
    }

    pub fn swing_period(&self) -> f32 {
        let haste = self.haste.clamp(0.0, 0.80);
        (0.72 * (1.0 - haste)).max(0.28)
    }

    pub fn add_tp(&mut self, amount: f32) {
        self.tp = (self.tp + amount).clamp(0.0, TP_MAX);
    }

    pub fn note_cast(&mut self, label: &str, now: f32) {
        self.last_cast_label = label.to_string();
        self.last_cast_until = now + 2.2;
        self.resting = false;
    }
}

#[derive(Clone, Debug)]
pub enum CastOutcome {
    None,
    Miss,
    Damage { amount: i32, kind: DamageKind },
    Heal { amount: i32 },
    Buff { label: String },
    Fail { reason: &'static str },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamageKind {
    Physical,
    Magic,
}

#[derive(Clone, Copy, Debug)]
pub struct TargetRef {
    pub vit: i32,
    pub defense: i32,
    pub magic_defense: i32,
    pub evasion: i32,
    pub distance: f32,
}

pub fn rebuild_hotbar(fighter: &mut Fighter, kits: &CombatKits, job: JobId, level: u32) {
    fighter.hotbar = kits.hotbar_for(job, level);
    fighter.recast_until.clear();
}

pub fn ensure_fighter(
    mut commands: Commands,
    kits: Res<CombatKits>,
    q: Query<(Entity, &Progression), (With<crate::player::Player>, Without<Fighter>)>,
) {
    for (e, prog) in &q {
        let mut f = Fighter::default();
        rebuild_hotbar(&mut f, &kits, prog.job, prog.level);
        commands.entity(e).insert(f);
    }
}

pub fn sync_hotbar_on_level(
    kits: Res<CombatKits>,
    mut q: Query<(&Progression, &mut Fighter), Changed<Progression>>,
) {
    for (prog, mut fighter) in &mut q {
        rebuild_hotbar(&mut fighter, &kits, prog.job, prog.level);
    }
}

pub fn cycle_job(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    note: Res<crate::player::NoteComposer>,
    kits: Res<CombatKits>,
    mut q: Query<(&mut Progression, &mut Fighter), With<crate::player::Player>>,
) {
    if note.active {
        return;
    }
    let Ok((mut prog, mut fighter)) = q.single_mut() else {
        return;
    };
    let dir = if keys.just_pressed(KeyCode::BracketRight) {
        1isize
    } else if keys.just_pressed(KeyCode::BracketLeft) {
        -1isize
    } else {
        return;
    };
    let all = JobId::ALL;
    let idx = all.iter().position(|j| *j == prog.job).unwrap_or(0) as isize;
    let next = (idx + dir).rem_euclid(all.len() as isize) as usize;
    prog.job = all[next];
    prog.sync_sheet();
    prog.hp = prog.vitals.max_hp;
    prog.mp = prog.vitals.max_mp;
    rebuild_hotbar(&mut fighter, &kits, prog.job, prog.level);
    fighter.flux = false;
    fighter.aether = false;
    fighter.note_cast(
        &format!("Job: {}", prog.job.display_name()),
        time.elapsed_secs(),
    );
}

fn slot_key(slot: usize) -> Option<KeyCode> {
    Some(match slot {
        0 => KeyCode::Digit1,
        1 => KeyCode::Digit2,
        2 => KeyCode::Digit3,
        3 => KeyCode::Digit4,
        4 => KeyCode::Digit5,
        5 => KeyCode::Digit6,
        6 => KeyCode::Digit7,
        7 => KeyCode::Digit8,
        _ => return None,
    })
}

pub fn hotbar_slot_pressed(keys: &ButtonInput<KeyCode>) -> Option<usize> {
    for i in 0..HOTBAR_SIZE {
        if let Some(k) = slot_key(i) {
            if keys.just_pressed(k) {
                return Some(i);
            }
        }
    }
    None
}

pub fn resolve_auto_attack(
    job: JobId,
    stats: &CombatStats,
    fighter: &Fighter,
    target: TargetRef,
    rng_hit: f32,
    rng_crit: f32,
) -> CastOutcome {
    if target.distance > AUTO_REACH {
        return CastOutcome::Fail {
            reason: "Out of range",
        };
    }
    if fighter.aether {
        return CastOutcome::Fail {
            reason: "Aether suppresses melee",
        };
    }
    let acc = stats.dex + 12;
    if rng_hit >= hit_chance(acc, target.evasion) {
        return CastOutcome::Miss;
    }
    let crit = rng_crit < crit_from_agi(stats.agi) + if job == JobId::Rogue { 0.08 } else { 0.0 };
    let fstr = melee_f_str(job, stats, target.vit, fighter.flux);
    let atk = ((stats.str + 14) as f32 * fighter.atk_mul).round() as i32;
    let dmg = physical_damage(atk, target.defense, fstr, crit);
    CastOutcome::Damage {
        amount: dmg.max(1),
        kind: DamageKind::Physical,
    }
}

fn tp_bonus_mul(tp: f32) -> f32 {
    // ba-stick weaponTpBonusMul: 0.25 at 1000 → 1.0 at 3000
    let t = ((tp - WEAPON_TP_COST as f32) / (TP_MAX - WEAPON_TP_COST as f32)).clamp(0.0, 1.0);
    0.25 + t * 0.75
}

pub fn resolve_weapon_skill(
    job: JobId,
    stats: &CombatStats,
    fighter: &Fighter,
    ws: &WeaponSkillDef,
    target: TargetRef,
    rng_hit: f32,
) -> CastOutcome {
    if fighter.tp < ws.tp_cost as f32 {
        return CastOutcome::Fail {
            reason: "Not enough TP",
        };
    }
    if target.distance > AUTO_REACH + 0.4 {
        return CastOutcome::Fail {
            reason: "Out of range",
        };
    }
    let acc = stats.dex + 18;
    if rng_hit >= hit_chance(acc, target.evasion) {
        return CastOutcome::Miss;
    }
    let fstr = melee_f_str(job, stats, target.vit, fighter.flux);
    let atk = ((stats.str + 18) as f32 * fighter.atk_mul).round() as i32;
    let base = physical_damage(atk, target.defense, fstr, false);
    let mul = ws.ftp * (1.0 + tp_bonus_mul(fighter.tp));
    let total = ((base as f32) * mul * ws.hits.max(1) as f32).floor().max(1.0) as i32;
    CastOutcome::Damage {
        amount: total,
        kind: DamageKind::Physical,
    }
}

fn elemental_damage(potency: i32, stats: &CombatStats, aether: bool) -> i32 {
    let mut base =
        potency as f32 * 0.52 + stats.int as f32 * 2.85 + stats.mnd as f32 * 0.35;
    if aether {
        base *= 1.15;
    }
    apply_magic_attack_bonus(base.floor().max(1.0) as i32, stats.int / 2)
}

fn divine_damage(potency: i32, stats: &CombatStats) -> i32 {
    let base = potency as f32 * 0.4 + stats.mnd as f32 * 2.5 + stats.str as f32 * 0.2;
    apply_magic_attack_bonus(base.floor().max(1.0) as i32, stats.mnd / 3)
}

fn physical_ja_damage(potency: i32, stats: &CombatStats, fighter: &Fighter) -> i32 {
    let base = potency as f32 + stats.str as f32 * 0.8 + stats.vit as f32 * 0.3;
    ((base * fighter.atk_mul).floor().max(1.0) as i32).max(1)
}

pub fn resolve_ability(
    job: JobId,
    stats: &CombatStats,
    prog_mp: i32,
    fighter: &Fighter,
    def: &AbilityDef,
    target: Option<TargetRef>,
    now: f32,
) -> CastOutcome {
    let _ = job; // hotbar already job-gated
    if let Some(until) = fighter.recast_until.get(&def.id) {
        if now < *until {
            return CastOutcome::Fail {
                reason: "On cooldown",
            };
        }
    }
    if def.mp > 0 && prog_mp < def.mp {
        return CastOutcome::Fail {
            reason: "Not enough MP",
        };
    }

    let needs_enemy = def.needs_target.unwrap_or(false);
    let ally = def.ally_target.unwrap_or(false);
    let reach = def.range.unwrap_or(if needs_enemy { 12 } else { 0 }) as f32;

    if needs_enemy {
        let Some(t) = target else {
            return CastOutcome::Fail {
                reason: "No target",
            };
        };
        if t.distance > reach.max(AUTO_REACH) {
            return CastOutcome::Fail {
                reason: "Out of range",
            };
        }
    }

    // Stance toggles
    if def.id == "flux" || def.id == "aether" {
        return CastOutcome::Buff {
            label: def.label.clone(),
        };
    }

    if def.id == "rest" {
        return CastOutcome::Buff {
            label: "Rest".into(),
        };
    }

    if let Some(heal) = def.heal {
        let amount = heal + stats.mnd / 2 + stats.vit / 4;
        return CastOutcome::Heal { amount };
    }

    if let Some(pot) = def.potency {
        let Some(t) = target else {
            if needs_enemy || !ally {
                return CastOutcome::Fail {
                    reason: "No target",
                };
            }
            // Self-buff with potency unused — treat as buff
            return CastOutcome::Buff {
                label: def.label.clone(),
            };
        };
        let raw = match def.category.as_str() {
            "elemental" | "nuke" | "time" | "dark" => {
                elemental_damage(pot, stats, fighter.aether)
            }
            "divine" => divine_damage(pot, stats),
            "ja" | "enspell" => physical_ja_damage(pot, stats, fighter),
            _ if needs_enemy => {
                if matches!(
                    def.category.as_str(),
                    "enfeeble" | "enhance" | "utility" | "stance" | "travel"
                ) {
                    elemental_damage(pot, stats, fighter.aether)
                } else {
                    physical_ja_damage(pot, stats, fighter)
                }
            }
            _ => {
                return CastOutcome::Buff {
                    label: def.label.clone(),
                };
            }
        };
        let amount = match def.category.as_str() {
            "elemental" | "nuke" | "time" | "divine" | "enfeeble" | "dark" => {
                mitigate_magic_damage(raw, t.magic_defense)
            }
            _ => raw,
        };
        let kind = match def.category.as_str() {
            "ja" | "enspell" => DamageKind::Physical,
            _ => DamageKind::Magic,
        };
        return CastOutcome::Damage { amount, kind };
    }

    // Buffs / enfeebles without potency
    CastOutcome::Buff {
        label: def.label.clone(),
    }
}

pub fn apply_ability_side_effects(
    fighter: &mut Fighter,
    prog: &mut Progression,
    def: &AbilityDef,
    outcome: &CastOutcome,
    now: f32,
) {
    match outcome {
        CastOutcome::Fail { .. } | CastOutcome::None | CastOutcome::Miss => return,
        _ => {}
    }

    if def.mp > 0 {
        prog.mp = (prog.mp - def.mp).max(0);
    }
    if def.recast_ms > 0 {
        fighter
            .recast_until
            .insert(def.id.clone(), now + def.recast_ms as f32 / 1000.0);
    }

    if def.id == "flux" {
        fighter.flux = !fighter.flux;
        if fighter.flux {
            fighter.aether = false;
        }
    } else if def.id == "aether" {
        fighter.aether = !fighter.aether;
        if fighter.aether {
            fighter.flux = false;
        }
    } else if def.id == "rest" {
        fighter.resting = true;
    }

    if let Some(ms) = def.duration_ms {
        let until = now + ms as f32 / 1000.0;
        if let Some(v) = def.atk_mul {
            fighter.atk_mul = v;
            fighter.atk_mul_until = until;
        }
        if let Some(v) = def.phys_dt {
            fighter.phys_dt = v;
            fighter.phys_dt_until = until;
        }
        if let Some(v) = def.mag_dt {
            fighter.mag_dt = v;
            fighter.mag_dt_until = until;
        }
        if let Some(v) = def.haste {
            fighter.haste = v;
            fighter.haste_until = until;
        }
    }

    // Common JA atk buffs without explicit atkMul field in extract
    match def.id.as_str() {
        "berserk" | "blood_rage" | "brazen_rush" | "killing_storm" => {
            fighter.atk_mul = 1.35;
            fighter.atk_mul_until = now + def.duration_ms.unwrap_or(60_000) as f32 / 1000.0;
        }
        "warcry" | "aggressor" => {
            fighter.atk_mul = 1.2;
            fighter.atk_mul_until = now + def.duration_ms.unwrap_or(60_000) as f32 / 1000.0;
        }
        "defender" | "bulwark" | "sentinel" | "ghost_step" => {
            if fighter.phys_dt >= 1.0 {
                fighter.phys_dt = 0.55;
                fighter.phys_dt_until = now + def.duration_ms.unwrap_or(30_000) as f32 / 1000.0;
            }
        }
        "haste" | "hastega" | "quicken" | "quicken_ii" | "tempo" | "cl_haste" | "bm_haste" => {
            fighter.haste = def.haste.unwrap_or(0.15);
            fighter.haste_until = now + def.duration_ms.unwrap_or(180_000) as f32 / 1000.0;
        }
        _ => {}
    }

    fighter.note_cast(&def.label, now);
}

pub fn apply_ws_side_effects(fighter: &mut Fighter, ws: &WeaponSkillDef, now: f32) {
    fighter.tp = (fighter.tp - ws.tp_cost as f32).max(0.0);
    fighter.note_cast(&ws.label, now);
}

pub fn tick_rest(prog: &mut Progression, fighter: &mut Fighter, dt: f32) {
    if !fighter.resting {
        return;
    }
    let hp_gain = (prog.vitals.max_hp as f32 * 0.08 * dt).max(1.0 * dt);
    let mp_gain = (prog.vitals.max_mp as f32 * 0.10 * dt).max(0.5 * dt);
    prog.hp = ((prog.hp as f32) + hp_gain)
        .min(prog.vitals.max_hp as f32)
        .round() as i32;
    if prog.vitals.max_mp > 0 {
        prog.mp = ((prog.mp as f32) + mp_gain)
            .min(prog.vitals.max_mp as f32)
            .round() as i32;
    }
}

pub fn mitigate_incoming_physical(raw: i32, fighter: &Fighter) -> i32 {
    ((raw as f32) * fighter.phys_dt.clamp(0.15, 1.0))
        .floor()
        .max(1.0) as i32
}

/// Approximate delay-based TP (§14) for a standard 1H/staff swing (~180–240 delay).
pub fn tp_from_auto_hit(weapon_delay: f32) -> f32 {
    let d = weapon_delay;
    let base = if d <= 180.0 {
        5.0 + (d - 180.0) * 1.5 / 180.0
    } else if d <= 450.0 {
        5.0 + (d - 180.0) * 6.5 / 270.0
    } else {
        11.5
    };
    base.max(4.0)
}

pub fn format_hotbar_line(kits: &CombatKits, fighter: &Fighter, now: f32) -> String {
    let mut parts = Vec::new();
    for (i, id) in fighter.hotbar.iter().enumerate() {
        let label = kits.label_of(id);
        let cd = fighter
            .recast_until
            .get(id)
            .map(|u| (*u - now).max(0.0))
            .unwrap_or(0.0);
        if cd > 0.05 {
            parts.push(format!("{}:{}({:.0}s)", i + 1, short_label(label), cd));
        } else {
            parts.push(format!("{}:{}", i + 1, short_label(label)));
        }
    }
    if parts.is_empty() {
        "Hotbar empty".into()
    } else {
        parts.join("  ")
    }
}

fn short_label(label: &str) -> String {
    let t = label.trim();
    if t.chars().count() <= 10 {
        t.to_string()
    } else {
        t.chars().take(9).collect::<String>() + "…"
    }
}
