//! Port of ba-stick `packages/combat/src/grades.ts` + dual-job attribute merge.

use super::jobs::{support_job_active, subjob_level, JobId};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CombatStats {
    pub str: i32,
    pub dex: i32,
    pub vit: i32,
    pub agi: i32,
    pub int: i32,
    pub mnd: i32,
}

impl CombatStats {
    pub fn add(self, o: Self) -> Self {
        Self {
            str: self.str + o.str,
            dex: self.dex + o.dex,
            vit: self.vit + o.vit,
            agi: self.agi + o.agi,
            int: self.int + o.int,
            mnd: self.mnd + o.mnd,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Vitals {
    pub max_hp: i32,
    pub max_mp: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grade {
    APlus,
    A,
    BPlus,
    B,
    CPlus,
    C,
    D,
    E,
    F,
    None,
}

struct JobGrades {
    hp: Grade,
    mp: Grade,
    str: Grade,
    dex: Grade,
    vit: Grade,
    agi: Grade,
    int: Grade,
    mnd: Grade,
}

fn attr_growth(g: Grade) -> f32 {
    match g {
        Grade::APlus => 0.58,
        Grade::A => 0.52,
        Grade::BPlus => 0.46,
        Grade::B => 0.42,
        Grade::CPlus => 0.36,
        Grade::C => 0.32,
        Grade::D => 0.24,
        Grade::E => 0.16,
        Grade::F => 0.1,
        Grade::None => 0.0,
    }
}

fn attr_base(g: Grade) -> i32 {
    match g {
        Grade::APlus => 14,
        Grade::A => 13,
        Grade::BPlus => 12,
        Grade::B => 11,
        Grade::CPlus => 10,
        Grade::C => 9,
        Grade::D => 8,
        Grade::E => 6,
        Grade::F => 5,
        Grade::None => 0,
    }
}

fn hp_growth(g: Grade) -> f32 {
    match g {
        Grade::APlus => 9.0,
        Grade::A => 8.0,
        Grade::BPlus => 7.0,
        Grade::B => 6.2,
        Grade::CPlus => 5.5,
        Grade::C => 5.0,
        Grade::D => 4.0,
        Grade::E => 3.0,
        Grade::F => 2.2,
        Grade::None => 0.0,
    }
}

fn hp_base(g: Grade) -> i32 {
    match g {
        Grade::APlus => 150,
        Grade::A => 140,
        Grade::BPlus => 130,
        Grade::B => 125,
        Grade::CPlus => 122,
        Grade::C => 120,
        Grade::D => 110,
        Grade::E => 100,
        Grade::F => 90,
        Grade::None => 100,
    }
}

fn mp_growth(g: Grade) -> f32 {
    match g {
        Grade::APlus => 6.0,
        Grade::A => 5.2,
        Grade::BPlus => 4.6,
        Grade::B => 4.2,
        Grade::CPlus => 3.8,
        Grade::C => 3.5,
        Grade::D => 2.4,
        Grade::E => 1.5,
        Grade::F => 0.8,
        Grade::None => 0.0,
    }
}

fn mp_base(g: Grade) -> i32 {
    match g {
        Grade::APlus => 110,
        Grade::A => 100,
        Grade::BPlus => 90,
        Grade::B => 85,
        Grade::CPlus => 82,
        Grade::C => 80,
        Grade::D => 60,
        Grade::E => 40,
        Grade::F => 25,
        Grade::None => 0,
    }
}

fn job_grades(job: JobId) -> JobGrades {
    match job {
        JobId::TimeMage => JobGrades {
            hp: Grade::C,
            mp: Grade::C,
            str: Grade::E,
            dex: Grade::C,
            vit: Grade::D,
            agi: Grade::B,
            int: Grade::B,
            mnd: Grade::B,
        },
        JobId::Knight => JobGrades {
            hp: Grade::A,
            mp: Grade::E,
            str: Grade::B,
            dex: Grade::D,
            vit: Grade::A,
            agi: Grade::E,
            int: Grade::E,
            mnd: Grade::C,
        },
        JobId::Rogue => JobGrades {
            hp: Grade::D,
            mp: Grade::None,
            str: Grade::C,
            dex: Grade::A,
            vit: Grade::D,
            agi: Grade::A,
            int: Grade::D,
            mnd: Grade::E,
        },
        JobId::Cleric => JobGrades {
            hp: Grade::E,
            mp: Grade::B,
            str: Grade::D,
            dex: Grade::E,
            vit: Grade::C,
            agi: Grade::D,
            int: Grade::D,
            mnd: Grade::A,
        },
        JobId::Sorcerer => JobGrades {
            hp: Grade::F,
            mp: Grade::A,
            str: Grade::E,
            dex: Grade::C,
            vit: Grade::E,
            agi: Grade::C,
            int: Grade::A,
            mnd: Grade::D,
        },
        JobId::Fighter => JobGrades {
            hp: Grade::B,
            mp: Grade::None,
            str: Grade::A,
            dex: Grade::B,
            vit: Grade::C,
            agi: Grade::C,
            int: Grade::E,
            mnd: Grade::E,
        },
        JobId::BattleMage => JobGrades {
            hp: Grade::D,
            mp: Grade::C,
            str: Grade::C,
            dex: Grade::C,
            vit: Grade::D,
            agi: Grade::D,
            int: Grade::B,
            mnd: Grade::C,
        },
        JobId::Druid => JobGrades {
            hp: Grade::CPlus,
            mp: Grade::B,
            str: Grade::D,
            dex: Grade::C,
            vit: Grade::C,
            agi: Grade::C,
            int: Grade::C,
            mnd: Grade::B,
        },
    }
}

fn l1_stats(job: JobId) -> CombatStats {
    match job {
        JobId::TimeMage => CombatStats {
            str: 6,
            dex: 10,
            vit: 8,
            agi: 12,
            int: 11,
            mnd: 14,
        },
        JobId::Knight => CombatStats {
            str: 11,
            dex: 7,
            vit: 14,
            agi: 5,
            int: 4,
            mnd: 9,
        },
        JobId::Rogue => CombatStats {
            str: 8,
            dex: 14,
            vit: 7,
            agi: 14,
            int: 6,
            mnd: 5,
        },
        JobId::Sorcerer => CombatStats {
            str: 5,
            dex: 9,
            vit: 6,
            agi: 9,
            int: 14,
            mnd: 7,
        },
        JobId::Fighter => CombatStats {
            str: 14,
            dex: 10,
            vit: 10,
            agi: 8,
            int: 5,
            mnd: 5,
        },
        JobId::BattleMage => CombatStats {
            str: 9,
            dex: 9,
            vit: 8,
            agi: 8,
            int: 11,
            mnd: 10,
        },
        JobId::Cleric => CombatStats {
            str: 7,
            dex: 6,
            vit: 10,
            agi: 6,
            int: 7,
            mnd: 15,
        },
        JobId::Druid => CombatStats {
            str: 8,
            dex: 9,
            vit: 11,
            agi: 10,
            int: 9,
            mnd: 12,
        },
    }
}

fn l1_vitals(job: JobId) -> Vitals {
    match job {
        JobId::TimeMage => Vitals {
            max_hp: 120,
            max_mp: 80,
        },
        JobId::Knight => Vitals {
            max_hp: 160,
            max_mp: 40,
        },
        JobId::Rogue => Vitals {
            max_hp: 100,
            max_mp: 0,
        },
        JobId::Sorcerer => Vitals {
            max_hp: 88,
            max_mp: 100,
        },
        JobId::Fighter => Vitals {
            max_hp: 140,
            max_mp: 0,
        },
        JobId::BattleMage => Vitals {
            max_hp: 115,
            max_mp: 72,
        },
        JobId::Cleric => Vitals {
            max_hp: 95,
            max_mp: 95,
        },
        JobId::Druid => Vitals {
            max_hp: 122,
            max_mp: 88,
        },
    }
}

fn clamp_level(level: u32) -> u32 {
    level.clamp(1, 75)
}

fn attr_at(grade: Grade, level: u32, seed: i32) -> i32 {
    if grade == Grade::None {
        return 0;
    }
    let base = if seed > 0 { seed } else { attr_base(grade) };
    ((base as f32) + (level.saturating_sub(1) as f32) * attr_growth(grade)).floor() as i32
}

pub fn job_stats_at_level(job: JobId, level: u32) -> CombatStats {
    let lv = clamp_level(level);
    let g = job_grades(job);
    let seed = l1_stats(job);
    CombatStats {
        str: attr_at(g.str, lv, seed.str).max(1),
        dex: attr_at(g.dex, lv, seed.dex).max(1),
        vit: attr_at(g.vit, lv, seed.vit).max(1),
        agi: attr_at(g.agi, lv, seed.agi).max(1),
        int: attr_at(g.int, lv, seed.int).max(1),
        mnd: attr_at(g.mnd, lv, seed.mnd).max(1),
    }
}

pub fn job_vitals_at_level(job: JobId, level: u32) -> Vitals {
    let lv = clamp_level(level);
    let g = job_grades(job);
    let seed = l1_vitals(job);

    let max_hp = if g.hp == Grade::None {
        100
    } else {
        let base = if seed.max_hp > 0 {
            seed.max_hp
        } else {
            hp_base(g.hp)
        };
        (base as f32 + (lv.saturating_sub(1) as f32) * hp_growth(g.hp)).floor() as i32
    };

    let max_mp = if g.mp == Grade::None {
        0
    } else {
        let base = if seed.max_mp > 0 {
            seed.max_mp
        } else {
            mp_base(g.mp)
        };
        (base as f32 + (lv.saturating_sub(1) as f32) * mp_growth(g.mp)).floor() as i32
    };

    Vitals { max_hp, max_mp }
}

/// Main at full level + support growth at half level (ba-stick `combinedJobStats`).
pub fn combined_job_stats(main: JobId, level: u32, sub: Option<JobId>) -> CombatStats {
    let main_stats = job_stats_at_level(main, level);
    if !support_job_active(main, level, sub) {
        return main_stats;
    }
    let sub = sub.unwrap();
    let sub_now = job_stats_at_level(sub, subjob_level(level));
    let sub_base = job_stats_at_level(sub, 1);
    main_stats.add(CombatStats {
        str: sub_now.str - sub_base.str,
        dex: sub_now.dex - sub_base.dex,
        vit: sub_now.vit - sub_base.vit,
        agi: sub_now.agi - sub_base.agi,
        int: sub_now.int - sub_base.int,
        mnd: sub_now.mnd - sub_base.mnd,
    })
}

pub fn combined_job_vitals(main: JobId, level: u32, sub: Option<JobId>) -> Vitals {
    let mut v = job_vitals_at_level(main, level);
    if !support_job_active(main, level, sub) {
        return v;
    }
    let sub = sub.unwrap();
    let now = job_vitals_at_level(sub, subjob_level(level));
    let base = job_vitals_at_level(sub, 1);
    v.max_hp += now.max_hp - base.max_hp;
    v.max_mp += now.max_mp - base.max_mp;
    v
}
