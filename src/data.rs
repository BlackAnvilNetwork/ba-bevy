use bevy::prelude::*;
use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

#[derive(Resource, Debug, Clone)]
pub struct GameData {
    pub world: WorldData,
    pub zone: ZoneData,
    pub scene: SceneData,
    pub npc: NpcData,
    pub cutscene: CutsceneData,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct WorldData {
    #[serde(rename = "worldId")]
    pub world_id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    #[serde(rename = "startupZoneId")]
    pub startup_zone_id: String,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ZoneData {
    #[serde(rename = "zoneId")]
    pub zone_id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub role: Option<String>,
    #[serde(default)]
    pub bounds: ZoneBounds,
    #[serde(default)]
    pub spawn: ZoneSpawn,
    #[serde(default)]
    pub exits: Vec<ZoneExit>,
    #[serde(default)]
    pub props: Vec<ZoneProp>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ZoneBounds {
    #[serde(rename = "minX")]
    pub min_x: f32,
    #[serde(rename = "maxX")]
    pub max_x: f32,
    #[serde(rename = "minZ")]
    pub min_z: f32,
    #[serde(rename = "maxZ")]
    pub max_z: f32,
}

impl Default for ZoneBounds {
    fn default() -> Self {
        Self {
            min_x: -16.0,
            max_x: 32.0,
            min_z: -14.0,
            max_z: 20.0,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct ZoneSpawn {
    pub at: [f32; 3],
    #[serde(rename = "lookAt")]
    pub look_at: [f32; 3],
}

impl Default for ZoneSpawn {
    fn default() -> Self {
        Self {
            at: [0.2, 1.65, 6.4],
            look_at: [0.0, 1.2, -1.4],
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct ZoneExit {
    pub id: String,
    #[serde(rename = "toZoneId")]
    pub to_zone_id: String,
    pub hint: String,
    /// XZ minimum of the crossing rect.
    pub min: [f32; 2],
    /// XZ maximum of the crossing rect.
    pub max: [f32; 2],
}

#[derive(Debug, Clone, Deserialize)]
pub struct ZoneProp {
    pub model: String,
    pub name: String,
    pub at: [f32; 3],
    #[serde(default, rename = "rotY")]
    pub rot_y: f32,
    #[serde(default = "default_scale")]
    pub scale: f32,
    /// Full size for `primitive:` props. Ignored for glTF.
    #[serde(default = "default_size")]
    pub size: [f32; 3],
    #[serde(default)]
    pub role: Option<String>,
}

fn default_scale() -> f32 {
    1.0
}

fn default_size() -> [f32; 3] {
    [1.0, 1.0, 1.0]
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct SceneData {
    #[serde(rename = "sceneId")]
    pub scene_id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub mood: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct NpcData {
    #[serde(rename = "npcId")]
    pub npc_id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub role: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct CutsceneData {
    #[serde(rename = "cutsceneId")]
    pub cutscene_id: String,
    #[serde(rename = "displayName")]
    pub display_name: String,
    pub beats: Vec<CutsceneBeat>,
    #[serde(default, rename = "bookBeats")]
    pub book_beats: Vec<CutsceneBeat>,
}

#[derive(Debug, Clone, Deserialize)]
#[allow(dead_code)]
pub struct CutsceneBeat {
    pub id: String,
    pub tone: Option<String>,
    pub speaker: String,
    pub line: String,
    /// Optional stage cue: laugh | lean | pour.
    #[serde(default)]
    pub stage: Option<String>,
}

fn data_dir() -> PathBuf {
    // Prefer CWD/assets/data (cargo run from crate root), fall back to relative to exe.
    let candidates = [
        PathBuf::from("assets/data"),
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/data"),
    ];
    for path in candidates {
        if path.exists() {
            return path;
        }
    }
    PathBuf::from("assets/data")
}

fn load_json<T: for<'de> Deserialize<'de>>(rel: &str) -> T {
    let path = data_dir().join(rel);
    let text = fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
    serde_json::from_str(&text)
        .unwrap_or_else(|e| panic!("failed to parse {}: {e}", path.display()))
}

pub fn load_game_data(mut commands: Commands) {
    let data = GameData {
        world: load_json("World/World_BlackAnvil.json"),
        zone: load_json("Zones/Zone_GreymereYard.json"),
        scene: load_json("Scenes/Scene_GreymereYard.json"),
        npc: load_json("NPCs/NPC_Warden.json"),
        cutscene: load_json("Cinematics/Cutscene_WardenAtTheWell.json"),
    };
    info!(
        "Loaded {} — {} ({})",
        data.world.display_name, data.zone.display_name, data.scene.mood.as_deref().unwrap_or("—")
    );
    commands.insert_resource(data);
}
