//! Player progression — XP curve, level-up, sheet sync (ba-stick server tryLevelUp).

use bevy::prelude::*;

use super::combat::{
    player_defense, player_magic_defense, FREE_STAT_POINTS, LEVEL_UP_MS, SKILL_POINTS_PER_LEVEL,
    SKILL_PRESTIGE_CAP,
};
use super::grades::{combined_job_stats, combined_job_vitals, CombatStats, Vitals};
use super::jobs::JobId;

pub use ba_combat::experience::{xp_to_next_level, MAX_LEVEL};

#[derive(Component, Clone, Debug)]
pub struct Progression {
    pub job: JobId,
    pub subjob: Option<JobId>,
    pub level: u32,
    pub xp: u32,
    pub skill_points: u32,
    pub skill_prestige: u32,
    pub free_stat_points: u32,
    pub free_stats: CombatStats,
    pub stats: CombatStats,
    pub vitals: Vitals,
    pub hp: i32,
    pub mp: i32,
}

impl Default for Progression {
    fn default() -> Self {
        let job = JobId::TimeMage;
        let vitals = combined_job_vitals(job, 1, None);
        Self {
            job,
            subjob: None,
            level: 1,
            xp: 0,
            skill_points: SKILL_POINTS_PER_LEVEL,
            skill_prestige: 0,
            free_stat_points: FREE_STAT_POINTS,
            free_stats: CombatStats::default(),
            stats: combined_job_stats(job, 1, None),
            vitals,
            hp: vitals.max_hp,
            mp: vitals.max_mp,
        }
    }
}

impl Progression {
    pub fn sync_sheet(&mut self) {
        let mut stats = combined_job_stats(self.job, self.level, self.subjob);
        stats = stats.add(self.free_stats);
        self.stats = stats;
        let vitals = combined_job_vitals(self.job, self.level, self.subjob);
        let hp_ratio = if self.vitals.max_hp > 0 {
            self.hp as f32 / self.vitals.max_hp as f32
        } else {
            1.0
        };
        let mp_ratio = if self.vitals.max_mp > 0 {
            self.mp as f32 / self.vitals.max_mp as f32
        } else {
            1.0
        };
        self.vitals = vitals;
        self.hp = ((vitals.max_hp as f32) * hp_ratio)
            .round()
            .clamp(1.0, vitals.max_hp as f32) as i32;
        self.mp = if vitals.max_mp == 0 {
            0
        } else {
            ((vitals.max_mp as f32) * mp_ratio)
                .round()
                .clamp(0.0, vitals.max_mp as f32) as i32
        };
    }

    pub fn defense(&self) -> i32 {
        player_defense(self.stats.vit, 0)
    }

    pub fn magic_defense(&self) -> i32 {
        player_magic_defense(self.stats.mnd, 0)
    }

    pub fn xp_need(&self) -> u32 {
        xp_to_next_level(self.level)
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LevelUpResult {
    pub levels_gained: u32,
    pub prestige_echoes: u32,
}

impl LevelUpResult {
    pub fn leveled(&self) -> bool {
        self.levels_gained > 0 || self.prestige_echoes > 0
    }
}

pub fn try_level_up(p: &mut Progression) -> LevelUpResult {
    let mut result = LevelUpResult::default();
    let before = p.level;
    while p.level < MAX_LEVEL {
        let need = xp_to_next_level(p.level);
        if p.xp < need {
            break;
        }
        p.xp -= need;
        p.level += 1;
        result.levels_gained += 1;
    }
    if result.levels_gained > 0 {
        p.skill_points += result.levels_gained * SKILL_POINTS_PER_LEVEL;
        p.sync_sheet();
        p.hp = p.vitals.max_hp;
        p.mp = p.vitals.max_mp;
        let _ = before;
    }
    while p.level >= MAX_LEVEL && p.skill_prestige < SKILL_PRESTIGE_CAP {
        let need = xp_to_next_level(MAX_LEVEL);
        if p.xp < need {
            break;
        }
        p.xp -= need;
        p.skill_prestige += 1;
        p.skill_points += 1;
        result.prestige_echoes += 1;
    }
    result
}

/// Active level-up FX window on the player (ba-stick 2.4s).
#[derive(Component, Clone, Debug)]
pub struct LevelUpFx {
    pub timer: Timer,
}

impl Default for LevelUpFx {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(LEVEL_UP_MS as f32 / 1000.0, TimerMode::Once),
        }
    }
}

#[derive(Message, Clone, Copy, Debug)]
pub struct GrantXp {
    pub amount: u32,
    pub reason: XpReason,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum XpReason {
    WitnessAsh,
    BookMemory,
}

#[derive(Message, Clone, Copy, Debug, Default)]
pub struct PlayerLeveledUp {
    pub new_level: u32,
    pub levels_gained: u32,
}

pub fn apply_xp_grants(
    mut commands: Commands,
    mut grants: MessageReader<GrantXp>,
    mut leveled: MessageWriter<PlayerLeveledUp>,
    mut player: Query<(Entity, &mut Progression), With<crate::player::Player>>,
) {
    let Ok((entity, mut prog)) = player.single_mut() else {
        return;
    };
    let mut any_level = LevelUpResult::default();
    for grant in grants.read() {
        prog.xp = prog.xp.saturating_add(grant.amount);
        let r = try_level_up(&mut prog);
        any_level.levels_gained += r.levels_gained;
        any_level.prestige_echoes += r.prestige_echoes;
    }
    if any_level.leveled() {
        commands.entity(entity).insert(LevelUpFx::default());
        leveled.write(PlayerLeveledUp {
            new_level: prog.level,
            levels_gained: any_level.levels_gained,
        });
    }
}
