//! Port of ba-stick `packages/combat/src/jobs.ts`.

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
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
    pub const ALL: [JobId; 8] = [
        JobId::TimeMage,
        JobId::Knight,
        JobId::Rogue,
        JobId::Cleric,
        JobId::Sorcerer,
        JobId::Fighter,
        JobId::BattleMage,
        JobId::Druid,
    ];

    pub fn display_name(self) -> &'static str {
        match self {
            JobId::TimeMage => "Time Mage",
            JobId::Knight => "Knight",
            JobId::Rogue => "Rogue",
            JobId::Cleric => "Cleric",
            JobId::Sorcerer => "Sorcerer",
            JobId::Fighter => "Fighter",
            JobId::BattleMage => "Battle Mage",
            JobId::Druid => "Druid",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            JobId::TimeMage => "Tempo, Flux, and chronomancy — haste the party, enfeeble the hall.",
            JobId::Knight => "Shield tank — Provoke, Flash, Bulwark. Hold the line.",
            JobId::Rogue => "Ghost Step to slip blows; Backblade for crit openers.",
            JobId::Cleric => "Sacred Mercy, Ashmend, and Sunrise Recall.",
            JobId::Sorcerer => "Violet neve and elemental lances — Arcane Flood.",
            JobId::Fighter => "Two-hander physical DPS — Berserk, Killing Storm, Warcry.",
            JobId::BattleMage => "Hybrid blade and en-spells — teal runes between casts.",
            JobId::Druid => "Ring-Walker — Stick Bear, Stick Deer, Stick Lion, Racerunner.",
        }
    }
}

/// Main job level required before a support job can be set (ba-stick dual-job).
pub const SUBJOB_UNLOCK_LEVEL: u32 = 10;

pub fn subjob_level(main_level: u32) -> u32 {
    (main_level.max(1) / 2).max(1)
}

pub fn support_job_active(main: JobId, level: u32, sub: Option<JobId>) -> bool {
    match sub {
        Some(s) if s != main && s != JobId::Druid && level >= SUBJOB_UNLOCK_LEVEL => true,
        _ => false,
    }
}
