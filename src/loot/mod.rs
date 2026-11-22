//! Loot beam presentation system.
//!
//! This module implements the visual "grammar" of loot drops: layered beams with
//! per-tier colors, heights, and effects. The spec defines five tiers that get beams:
//!
//! - **Rare**: Cold blue (#3a7bd5), 2m tall
//! - **Epic**: Deep violet (#8e44ad), 3.5m tall  
//! - **Legendary**: Forge orange (#e67e22), 6m tall, rising sparks
//! - **Mythic**: Blood red (#c0392b), 10m tall, embers + slow rotation
//! - **Quest**: Aged gold (#d4af37), 4m tall, solid non-shimmering pillar
//!
//! Common items and materials (Bog Amber, Cold Iron, etc.) have no beam.
//!
//! Each beam consists of:
//! - Core column: thin additive cylinder, HDR-boosted for bloom
//! - Soft halo: wider Gaussian falloff for distance readability
//! - Ground glow: disc with radial gradient + point light
//! - Through-wall ghost: 60% opacity pass visible through geometry
//! - Sparks (Legendary+): CPU-pooled rising billboards
//!
//! Animation timeline (from T+0 death):
//! - T+0.65: Beam erupts (oversaturated white shoot-up)
//! - T+0.75: Settles to true color with pulse traveling up
//! - T+0.85+: Idle shimmer/oscillation (Quest is static)

pub mod beam;
pub mod debug;
pub mod grammar;
pub mod timeline;

use bevy::prelude::*;

use beam::{
    setup_beam_meshes, update_beam_visuals, GroundGlowMaterial, LootBeamMaterial, SparkMaterial,
};
use debug::{cleanup_debug_labels, debug_beam_hud, toggle_debug_lineup, LootDebugState};

/// Plugin that sets up the loot beam presentation system.
pub struct LootBeamPlugin;

impl Plugin for LootBeamPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            MaterialPlugin::<LootBeamMaterial>::default(),
            MaterialPlugin::<GroundGlowMaterial>::default(),
            MaterialPlugin::<SparkMaterial>::default(),
        ))
        .init_resource::<LootDebugState>()
        .add_systems(Startup, setup_beam_meshes)
        .add_systems(
            Update,
            (
                update_beam_visuals,
                toggle_debug_lineup,
                cleanup_debug_labels,
                debug_beam_hud,
            ),
        );
    }
}

// Re-export commonly used types
pub use beam::{spawn_beam, BeamMeshes, LootBeamBundle};
pub use grammar::{wire_rarity_to_grammar, LootGrammar};
pub use timeline::{beam_phase, beam_state, BeamPhase, BeamState};
