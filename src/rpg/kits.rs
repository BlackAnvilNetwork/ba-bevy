//! Ability / spell / weapon-skill catalogs ported from ba-stick kits.
//! Source JSON: `assets/combat/kits.json` (regenerate via `scripts/extract-ba-stick-kits.ps1`).

use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use bevy::prelude::*;
use serde::Deserialize;

use super::jobs::JobId;

#[derive(Resource, Debug, Clone)]
pub struct CombatKits {
    pub abilities: HashMap<String, AbilityDef>,
    pub weapon_skills: HashMap<String, WeaponSkillDef>,
    pub by_job: HashMap<JobId, Vec<String>>,
    pub job_weapons: HashMap<JobId, WeaponFamily>,
    pub hotbar_hints: HashMap<JobId, Vec<String>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WeaponFamily {
    Sword,
    Greatsword,
    Dagger,
    Axe,
    Staff,
    Club,
    Knuckles,
}

impl WeaponFamily {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Sword => "sword",
            Self::Greatsword => "greatsword",
            Self::Dagger => "dagger",
            Self::Axe => "axe",
            Self::Staff => "staff",
            Self::Club => "club",
            Self::Knuckles => "knuckles",
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AbilityDef {
    pub id: String,
    pub job: String,
    pub label: String,
    #[serde(default)]
    pub glyph: String,
    #[serde(default = "one")]
    pub unlock_level: u32,
    #[serde(default)]
    pub mp: i32,
    #[serde(default)]
    pub recast_ms: u32,
    #[serde(default)]
    pub category: String,
    pub duration_ms: Option<u32>,
    pub heal: Option<i32>,
    pub potency: Option<i32>,
    pub range: Option<i32>,
    pub aoe: Option<i32>,
    pub needs_target: Option<bool>,
    pub ally_target: Option<bool>,
    pub phys_dt: Option<f32>,
    pub mag_dt: Option<f32>,
    pub haste: Option<f32>,
    pub atk_mul: Option<f32>,
    #[serde(default)]
    pub blurb: String,
}

fn one() -> u32 {
    1
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeaponSkillDef {
    pub id: String,
    pub weapon: WeaponFamily,
    pub unlock_level: u32,
    pub label: String,
    #[serde(default)]
    pub glyph: String,
    pub hits: u32,
    pub ftp: f32,
    #[serde(default)]
    pub blurb: String,
    #[serde(default = "tp_cost_default")]
    pub tp_cost: i32,
}

fn tp_cost_default() -> i32 {
    1000
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct KitsFile {
    abilities: Vec<AbilityDef>,
    weapon_skills: Vec<WeaponSkillDef>,
    job_weapons: HashMap<String, WeaponFamily>,
    hotbar_hints: HashMap<String, Vec<String>>,
}

fn assets_combat_dir() -> PathBuf {
    let candidates = [
        PathBuf::from("assets/combat"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/combat"),
    ];
    for path in candidates {
        if path.exists() {
            return path;
        }
    }
    PathBuf::from("assets/combat")
}

fn job_from_slug(slug: &str) -> Option<JobId> {
    match slug {
        "time_mage" => Some(JobId::TimeMage),
        "knight" => Some(JobId::Knight),
        "rogue" => Some(JobId::Rogue),
        "cleric" => Some(JobId::Cleric),
        "sorcerer" => Some(JobId::Sorcerer),
        "fighter" => Some(JobId::Fighter),
        "battle_mage" => Some(JobId::BattleMage),
        "druid" => Some(JobId::Druid),
        _ => None,
    }
}

pub fn job_slug(job: JobId) -> &'static str {
    match job {
        JobId::TimeMage => "time_mage",
        JobId::Knight => "knight",
        JobId::Rogue => "rogue",
        JobId::Cleric => "cleric",
        JobId::Sorcerer => "sorcerer",
        JobId::Fighter => "fighter",
        JobId::BattleMage => "battle_mage",
        JobId::Druid => "druid",
    }
}

impl CombatKits {
    pub fn load() -> Self {
        let path = assets_combat_dir().join("kits.json");
        let text = fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        // PowerShell UTF-8 exports sometimes prepend a BOM.
        let text = text.trim_start_matches('\u{feff}');
        let file: KitsFile = serde_json::from_str(text)
            .unwrap_or_else(|e| panic!("failed to parse {}: {e}", path.display()));

        let mut abilities = HashMap::new();
        let mut by_job: HashMap<JobId, Vec<String>> = HashMap::new();
        for a in file.abilities {
            if let Some(job) = job_from_slug(&a.job) {
                by_job.entry(job).or_default().push(a.id.clone());
            }
            abilities.insert(a.id.clone(), a);
        }

        let mut weapon_skills = HashMap::new();
        for ws in file.weapon_skills {
            weapon_skills.insert(ws.id.clone(), ws);
        }

        let mut job_weapons = HashMap::new();
        for (slug, fam) in file.job_weapons {
            if let Some(job) = job_from_slug(&slug) {
                job_weapons.insert(job, fam);
            }
        }

        let mut hotbar_hints = HashMap::new();
        for (slug, hints) in file.hotbar_hints {
            if let Some(job) = job_from_slug(&slug) {
                hotbar_hints.insert(job, hints);
            }
        }

        info!(
            "Loaded combat kits: {} abilities, {} weapon skills",
            abilities.len(),
            weapon_skills.len()
        );

        Self {
            abilities,
            weapon_skills,
            by_job,
            job_weapons,
            hotbar_hints,
        }
    }

    pub fn weapon_for(&self, job: JobId) -> WeaponFamily {
        self.job_weapons
            .get(&job)
            .copied()
            .unwrap_or(WeaponFamily::Staff)
    }

    pub fn ability(&self, id: &str) -> Option<&AbilityDef> {
        self.abilities.get(id)
    }

    pub fn weapon_skill(&self, id: &str) -> Option<&WeaponSkillDef> {
        self.weapon_skills.get(id)
    }

    /// Build a compact hotbar: preferred hints first (if unlocked), then fill with other unlocked.
    pub fn hotbar_for(&self, job: JobId, level: u32) -> Vec<String> {
        let weapon = self.weapon_for(job);
        let mut out = Vec::new();
        let mut used = std::collections::HashSet::new();

        let hints = self.hotbar_hints.get(&job).cloned().unwrap_or_default();
        for id in hints {
            if used.contains(&id) {
                continue;
            }
            if self.is_unlocked(job, level, &id, weapon) {
                used.insert(id.clone());
                out.push(id);
            }
            if out.len() >= 8 {
                return out;
            }
        }

        // Prefer offensive / JA / heals over rest.
        let mut candidates: Vec<(u32, String)> = Vec::new();
        if let Some(ids) = self.by_job.get(&job) {
            for id in ids {
                if used.contains(id) {
                    continue;
                }
                let Some(def) = self.abilities.get(id) else {
                    continue;
                };
                if def.unlock_level > level {
                    continue;
                }
                let rank = match def.category.as_str() {
                    "ja" => 0,
                    "elemental" | "divine" | "nuke" | "time" => 1,
                    "heal" => 2,
                    "enspell" | "enhance" | "stance" => 3,
                    "enfeeble" => 4,
                    _ => 5,
                };
                candidates.push((rank * 1000 + def.unlock_level, id.clone()));
            }
        }
        for (id, ws) in &self.weapon_skills {
            if ws.weapon != weapon || ws.unlock_level > level || used.contains(id) {
                continue;
            }
            candidates.push((10 + ws.unlock_level, id.clone()));
        }
        candidates.sort_by_key(|(k, _)| *k);
        for (_, id) in candidates {
            if out.len() >= 8 {
                break;
            }
            if used.insert(id.clone()) {
                out.push(id);
            }
        }
        out
    }

    fn is_unlocked(&self, job: JobId, level: u32, id: &str, weapon: WeaponFamily) -> bool {
        if let Some(a) = self.abilities.get(id) {
            let job_ok = a.job == job_slug(job)
                || id == "rest"
                || self
                    .by_job
                    .get(&job)
                    .is_some_and(|ids| ids.iter().any(|x| x == id));
            return job_ok && a.unlock_level <= level;
        }
        if let Some(ws) = self.weapon_skills.get(id) {
            return ws.weapon == weapon && ws.unlock_level <= level;
        }
        false
    }

    pub fn label_of<'a>(&'a self, id: &'a str) -> &'a str {
        if let Some(a) = self.abilities.get(id) {
            return &a.label;
        }
        if let Some(ws) = self.weapon_skills.get(id) {
            return &ws.label;
        }
        id
    }

    pub fn blurb_of<'a>(&'a self, id: &'a str) -> &'a str {
        if let Some(a) = self.abilities.get(id) {
            return &a.blurb;
        }
        if let Some(ws) = self.weapon_skills.get(id) {
            return &ws.blurb;
        }
        ""
    }
}

pub fn load_combat_kits(mut commands: Commands) {
    commands.insert_resource(CombatKits::load());
}
