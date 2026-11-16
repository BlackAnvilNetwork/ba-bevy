//! Load `assets/combat/kits.json` for shared hotbar / cast resolution.

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use serde::Deserialize;

use crate::{AbilityKind, AbilitySpec, JobId, AUTO_REACH};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AbilityDef {
    id: String,
    job: String,
    #[serde(default)]
    mp: i32,
    #[serde(default)]
    recast_ms: u32,
    #[serde(default)]
    category: String,
    duration_ms: Option<u32>,
    heal: Option<i32>,
    potency: Option<i32>,
    range: Option<i32>,
    atk_mul: Option<f32>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WeaponSkillDef {
    id: String,
    unlock_level: u32,
    ftp: f32,
    #[serde(default = "tp_default")]
    tp_cost: i32,
}

fn tp_default() -> i32 {
    1000
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct KitsFile {
    abilities: Vec<AbilityDef>,
    weapon_skills: Vec<WeaponSkillDef>,
    hotbar_hints: HashMap<String, Vec<String>>,
}

#[derive(Clone, Debug, Default)]
pub struct KitCatalog {
    abilities: HashMap<String, AbilityDef>,
    weapon_skills: HashMap<String, WeaponSkillDef>,
    hotbar_hints: HashMap<JobId, Vec<String>>,
}

static CATALOG: OnceLock<KitCatalog> = OnceLock::new();

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

fn job_slug(job: JobId) -> &'static str {
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

fn default_kits_path() -> PathBuf {
    let candidates = [
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/combat/kits.json"),
        PathBuf::from("assets/combat/kits.json"),
    ];
    for p in candidates {
        if p.is_file() {
            return p;
        }
    }
    PathBuf::from("assets/combat/kits.json")
}

impl KitCatalog {
    pub fn load_from(path: &Path) -> Result<Self, String> {
        let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
        let text = text.trim_start_matches('\u{feff}');
        let file: KitsFile =
            serde_json::from_str(text).map_err(|e| format!("parse {}: {e}", path.display()))?;

        let mut abilities = HashMap::new();
        for a in file.abilities {
            abilities.insert(a.id.clone(), a);
        }
        let mut weapon_skills = HashMap::new();
        for ws in file.weapon_skills {
            weapon_skills.insert(ws.id.clone(), ws);
        }
        let mut hotbar_hints = HashMap::new();
        for (slug, hints) in file.hotbar_hints {
            if let Some(job) = job_from_slug(&slug) {
                hotbar_hints.insert(job, hints);
            }
        }
        Ok(Self {
            abilities,
            weapon_skills,
            hotbar_hints,
        })
    }

    pub fn ability_count(&self) -> usize {
        self.abilities.len()
    }

    pub fn ws_count(&self) -> usize {
        self.weapon_skills.len()
    }

    /// Resolve a castable ability for this job (ability, WS, or none).
    pub fn resolve(&self, job: JobId, id: &str) -> Option<AbilitySpec> {
        if let Some(ws) = self.weapon_skills.get(id) {
            return Some(AbilitySpec {
                id: ws.id.clone(),
                potency: (ws.ftp * 100.0).round().max(1.0) as i32,
                mp: 0,
                range: AUTO_REACH + 0.4,
                kind: AbilityKind::WeaponSkill,
                recast: 1.0,
                atk_mul: None,
                duration: 0.0,
                tp_cost: ws.tp_cost,
            });
        }
        let a = self.abilities.get(id)?;
        // Job gate: allow if ability job matches or ability has no job (shouldn't happen).
        if let Some(aj) = job_from_slug(&a.job) {
            if aj != job {
                return None;
            }
        }
        let recast = (a.recast_ms as f32 / 1000.0).clamp(0.5, 300.0);
        // Cap absurd 2h timers for M2 zone playability (still honor long CD via clamp).
        let recast = recast.min(120.0);
        let duration = a.duration_ms.map(|ms| ms as f32 / 1000.0).unwrap_or(0.0);
        let range = a
            .range
            .map(|r| r as f32)
            .unwrap_or(AUTO_REACH + 0.4)
            .max(0.0);

        if let Some(heal) = a.heal {
            return Some(AbilitySpec {
                id: a.id.clone(),
                potency: heal,
                mp: a.mp,
                range: 0.0,
                kind: AbilityKind::Heal,
                recast: recast.min(30.0),
                atk_mul: None,
                duration: 0.0,
                tp_cost: 0,
            });
        }
        if let Some(mul) = a.atk_mul {
            return Some(AbilitySpec {
                id: a.id.clone(),
                potency: 0,
                mp: a.mp,
                range: 0.0,
                kind: AbilityKind::Buff,
                recast,
                atk_mul: Some(mul),
                duration: duration.max(5.0),
                tp_cost: 0,
            });
        }
        if let Some(pot) = a.potency {
            let kind = if a.category == "spell" || a.category == "magic" || a.category == "heal" {
                AbilityKind::Magic
            } else {
                AbilityKind::PhysicalJa
            };
            return Some(AbilitySpec {
                id: a.id.clone(),
                potency: pot,
                mp: a.mp,
                range,
                kind,
                recast: recast.min(60.0),
                atk_mul: None,
                duration: 0.0,
                tp_cost: 0,
            });
        }
        // Crit-storm style JA: treat as short atk buff so the hotbar slot does something.
        if a.category == "ja" && duration > 0.0 {
            return Some(AbilitySpec {
                id: a.id.clone(),
                potency: 0,
                mp: a.mp,
                range: 0.0,
                kind: AbilityKind::Buff,
                recast,
                atk_mul: Some(1.15),
                duration: duration.min(60.0),
                tp_cost: 0,
            });
        }
        None
    }

    pub fn hotbar(&self, job: JobId) -> Vec<AbilitySpec> {
        let hints = self
            .hotbar_hints
            .get(&job)
            .cloned()
            .unwrap_or_else(|| fallback_hints(job));
        let mut out = Vec::new();
        for id in hints {
            if let Some(spec) = self.resolve(job, &id) {
                out.push(spec);
            }
            if out.len() >= 8 {
                break;
            }
        }
        if out.is_empty() {
            // Last resort: any ability for this job that resolves.
            let slug = job_slug(job);
            for a in self.abilities.values() {
                if a.job == slug {
                    if let Some(spec) = self.resolve(job, &a.id) {
                        out.push(spec);
                    }
                }
                if out.len() >= 8 {
                    break;
                }
            }
        }
        out
    }
}

fn fallback_hints(job: JobId) -> Vec<String> {
    match job {
        JobId::Fighter => vec![
            "berserk".into(),
            "warcry".into(),
            "killing_storm".into(),
            "aggressor".into(),
            "ws_gs_hard_slash".into(),
        ],
        JobId::Knight => vec!["shield_bash".into(), "ws_sword_fast_blade".into()],
        JobId::TimeMage => vec!["cure".into(), "temporal_distortion".into()],
        _ => Vec::new(),
    }
}

/// Load once from default path (no-op if already loaded or file missing).
pub fn ensure_kits_loaded() -> bool {
    if CATALOG.get().is_some() {
        return true;
    }
    let path = default_kits_path();
    match KitCatalog::load_from(&path) {
        Ok(cat) => {
            let _ = CATALOG.set(cat);
            true
        }
        Err(_) => false,
    }
}

pub fn load_kits_from(path: &Path) -> Result<(), String> {
    let cat = KitCatalog::load_from(path)?;
    let _ = CATALOG.set(cat);
    Ok(())
}

pub fn kits() -> Option<&'static KitCatalog> {
    CATALOG.get()
}

pub fn kits_or_load() -> Option<&'static KitCatalog> {
    ensure_kits_loaded();
    CATALOG.get()
}
