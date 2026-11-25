//! Debug utilities for loot beam development.
//!
//! Provides a command to spawn a lineup of all beam tiers for visual comparison
//! and screenshot capture.

use bevy::prelude::*;

use super::{
    beam::{
        spawn_beam, BeamMeshes, GroundGlowMaterial, LootBeamMaterial, LootBeamBundle,
        SparkMaterial,
    },
    grammar::LootGrammar,
};

/// Resource to track debug lineup state.
#[derive(Resource, Default)]
pub struct LootDebugState {
    /// Whether the debug lineup is currently spawned.
    pub lineup_active: bool,
    /// Entities spawned for the lineup.
    pub lineup_entities: Vec<Entity>,
    /// Time the lineup was spawned (for consistent animation).
    pub lineup_spawn_time: f32,
}

/// Marker component for debug lineup beams.
#[derive(Component)]
pub struct DebugBeam {
    pub grammar: LootGrammar,
}

/// Marker for debug lineup label.
#[derive(Component)]
pub struct DebugBeamLabel;

/// System to handle debug lineup toggle with F9 key.
pub fn toggle_debug_lineup(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut commands: Commands,
    mut state: ResMut<LootDebugState>,
    beam_meshes: Option<Res<BeamMeshes>>,
    mut beam_mats: ResMut<Assets<LootBeamMaterial>>,
    mut glow_mats: ResMut<Assets<GroundGlowMaterial>>,
    mut spark_mats: ResMut<Assets<SparkMaterial>>,
    player: Query<&Transform, With<crate::player::Player>>,
) {
    // Also check for file-based request (like orbit.request)
    let file_request = check_lineup_request();

    if !keys.just_pressed(KeyCode::F9) && !file_request {
        return;
    }

    if state.lineup_active {
        // Despawn existing lineup
        despawn_lineup(&mut commands, &mut state);
        return;
    }

    let Some(beam_meshes) = beam_meshes else {
        warn!("Beam meshes not ready for debug lineup");
        return;
    };

    let Ok(player_tf) = player.single() else {
        warn!("No player for debug lineup");
        return;
    };

    let now = time.elapsed_secs();
    state.lineup_spawn_time = now;

    // Spawn beams in a line in front of the player
    let forward = player_tf.forward().as_vec3();
    let right = player_tf.right().as_vec3();
    let base_pos = player_tf.translation + forward * 8.0;

    let grammars = [
        LootGrammar::Rare,
        LootGrammar::Epic,
        LootGrammar::Legendary,
        LootGrammar::Mythic,
        LootGrammar::Quest,
    ];

    let spacing = 3.0;
    let start_offset = -((grammars.len() as f32 - 1.0) * spacing / 2.0);

    for (i, grammar) in grammars.iter().enumerate() {
        let offset = start_offset + i as f32 * spacing;
        let pos = base_pos + right * offset;
        // Place on ground (approximate)
        let ground_pos = Vec3::new(pos.x, 0.1, pos.z);

        let entity = spawn_beam(
            &mut commands,
            &beam_meshes,
            &mut beam_mats,
            &mut glow_mats,
            &mut spark_mats,
            *grammar,
            ground_pos,
            now,
        );

        // Add debug marker
        commands
            .entity(entity)
            .insert(DebugBeam { grammar: *grammar });

        state.lineup_entities.push(entity);
    }

    state.lineup_active = true;
    info!(
        "Debug loot lineup spawned at {:?} with {} beams",
        base_pos,
        grammars.len()
    );
}

/// Despawn the debug lineup.
fn despawn_lineup(commands: &mut Commands, state: &mut LootDebugState) {
    for entity in state.lineup_entities.drain(..) {
        // Despawn without recursive (children will be orphaned and cleaned up)
        commands.entity(entity).despawn();
    }
    state.lineup_active = false;
    info!("Debug loot lineup despawned");
}

/// Check for a file-based debug request.
fn check_lineup_request() -> bool {
    let path = std::path::Path::new("debug/inbox/loot_lineup.request");
    if path.exists() {
        let _ = std::fs::remove_file(path);
        true
    } else {
        false
    }
}

/// Spawn UI labels for the debug lineup showing tier names.
pub fn spawn_debug_labels(
    mut commands: Commands,
    state: Res<LootDebugState>,
    beams: Query<(&DebugBeam, &Transform), Added<DebugBeam>>,
    cam: Query<(&Camera, &GlobalTransform), With<crate::camera_rig::PlayerCamera>>,
    windows: Query<&Window>,
) {
    if !state.lineup_active {
        return;
    }

    let Ok((camera, cam_tf)) = cam.single() else {
        return;
    };

    let Ok(window) = windows.single() else {
        return;
    };

    for (debug_beam, tf) in &beams {
        // Project beam position to screen
        let world_pos = tf.translation + Vec3::Y * (debug_beam.grammar.beam_height() + 0.5);
        let Ok(viewport_pos) = camera.world_to_viewport(cam_tf, world_pos) else {
            continue;
        };

        let label = match debug_beam.grammar {
            LootGrammar::None => "None",
            LootGrammar::Rare => "Rare (2m)",
            LootGrammar::Epic => "Epic (3.5m)",
            LootGrammar::Legendary => "Legendary (6m)",
            LootGrammar::Mythic => "Mythic (10m)",
            LootGrammar::Quest => "Quest (4m)",
        };

        let color = debug_beam.grammar.color();

        commands.spawn((
            DebugBeamLabel,
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(viewport_pos.x / window.width() * window.width()),
                top: Val::Px(viewport_pos.y / window.height() * window.height()),
                ..default()
            },
            Text::new(label),
            TextFont::from_font_size(14.0),
            TextColor(color),
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        ));
    }
}

/// Update debug label positions as camera moves.
pub fn update_debug_labels(
    state: Res<LootDebugState>,
    beams: Query<(&DebugBeam, &GlobalTransform)>,
    mut labels: Query<(&mut Node, &mut Visibility), With<DebugBeamLabel>>,
    cam: Query<(&Camera, &GlobalTransform), With<crate::camera_rig::PlayerCamera>>,
    windows: Query<&Window>,
) {
    if !state.lineup_active {
        for (_, mut vis) in &mut labels {
            *vis = Visibility::Hidden;
        }
        return;
    }

    let Ok((camera, cam_tf)) = cam.single() else {
        return;
    };

    let Ok(window) = windows.single() else {
        return;
    };

    // This is a simplified version - in production we'd match labels to beams
    for (debug_beam, beam_tf) in &beams {
        let world_pos = beam_tf.translation() + Vec3::Y * (debug_beam.grammar.beam_height() + 0.5);
        let Ok(_viewport_pos) = camera.world_to_viewport(cam_tf, world_pos) else {
            continue;
        };
        // Label position update would go here
    }
}

/// Clean up debug labels when lineup is despawned.
pub fn cleanup_debug_labels(
    mut commands: Commands,
    state: Res<LootDebugState>,
    labels: Query<Entity, With<DebugBeamLabel>>,
) {
    if state.lineup_active {
        return;
    }

    for entity in &labels {
        commands.entity(entity).despawn();
    }
}

/// Debug HUD showing current beam state.
pub fn debug_beam_hud(
    state: Res<LootDebugState>,
    time: Res<Time>,
    mut hud: ResMut<crate::debug_shot::DebugHud>,
) {
    if !state.lineup_active {
        return;
    }

    let elapsed = time.elapsed_secs() - state.lineup_spawn_time;
    let phase = if elapsed < 0.65 {
        "Hidden"
    } else if elapsed < 0.75 {
        "Erupting"
    } else if elapsed < 0.85 {
        "Settling"
    } else {
        "Idle"
    };

    hud.notice = format!("Loot Lineup: T+{:.2}s Phase: {}", elapsed, phase);
    hud.notice_until = time.elapsed_secs() + 0.5;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grammar_names_are_readable() {
        let grammars = [
            LootGrammar::None,
            LootGrammar::Rare,
            LootGrammar::Epic,
            LootGrammar::Legendary,
            LootGrammar::Mythic,
            LootGrammar::Quest,
        ];

        for grammar in grammars {
            // Just verify we can format them
            let _ = format!("{:?}", grammar);
        }
    }
}
