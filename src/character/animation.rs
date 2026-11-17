//! Bevy 0.20 glTF animation graph for procedural Studio Idle/Walk clips.
//! The rendered character follows the authoritative movement Transform.
//! Animation is presentation only; it cannot grant speed/combat outcomes.

use std::time::Duration;
use bevy::prelude::*;

#[derive(Component)]
pub struct AvatarAnimationGraph {
    handle: Handle<AnimationGraph>,
    idle_clip: Handle<AnimationClip>,
    idle: AnimationNodeIndex,
    walk: AnimationNodeIndex,
}

#[derive(Component)]
pub(crate) struct LegacyRigPoseApplied;

#[derive(Component)]
pub(crate) struct LegacyArmPose {
    rest: Quat,
    direction: f32,
}

#[derive(Component)]
pub(crate) struct AvatarAnimationDrive {
    root: Entity,
    last_position: Vec3,
    initialized: bool,
    moving: bool,
}

/// Attach before a WorldAssetRoot finishes loading. The scene must contain
/// the two original Studio-generated clips labelled Animation(0)/(1).
/// Older static GLBs simply never create an AnimationPlayer and remain visible.
pub fn prepare_avatar_animation(
    commands: &mut Commands,
    asset_server: &AssetServer,
    graphs: &mut Assets<AnimationGraph>,
    root: Entity,
    glb_uri: &str,
) {
    let idle_clip: Handle<AnimationClip> =
        asset_server.load(GltfAssetLabel::Animation(0).from_asset(glb_uri.to_string()));
    let walk_clip: Handle<AnimationClip> =
        asset_server.load(GltfAssetLabel::Animation(1).from_asset(glb_uri.to_string()));
    let (graph, nodes) = AnimationGraph::from_clips([
        idle_clip.clone(),
        walk_clip,
    ]);
    let handle = graphs.add(graph);
    commands.entity(root).insert(AvatarAnimationGraph {
        handle, idle_clip, idle: nodes[0], walk: nodes[1],
    });
}

/// Explicitly bind every scene's AnimationPlayer once it exists. The glTF
/// importer can spawn the animated child AFTER the root scene-ready event on
/// some meshes: a one-off event observer may then miss the player, leaving
/// the skeleton in its T-pose forever. This polling system is idempotent and
/// runs only for newly spawned AnimationPlayers.
pub fn bind_avatar_animators(
    mut commands: Commands,
    roots: Query<(Entity, &AvatarAnimationGraph)>,
    children: Query<&Children>,
    mut players: Query<&mut AnimationPlayer, Without<AvatarAnimationDrive>>,
) {
    for (root, graph) in &roots {
        for descendant in children.iter_descendants(root) {
            if let Ok(mut player) = players.get_mut(descendant) {
                let mut transitions = AnimationTransitions::new();
                transitions.play(&mut player, graph.idle, Duration::ZERO).repeat();
                commands.entity(descendant).insert((
                    AnimationGraphHandle(graph.handle.clone()),
                    transitions,
                    AvatarAnimationDrive {
                        root,
                        last_position: Vec3::ZERO,
                        initialized: false,
                        moving: false,
                    },
                ));
                info!("Bound BA_Idle/BA_Walk animation to skinned avatar player {descendant:?}");
            }
        }
    }
}

/// Older locally saved GLBs may predate BA_Idle/BA_Walk (and Bevy won't
/// spawn an AnimationPlayer for them). Give such skeletons a relaxed arms-down
/// fallback instead of rendering a permanent T-pose. The newer clips take
/// precedence and are never mixed with this fallback.
pub fn pose_legacy_unanimated_avatars(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    roots: Query<(Entity,&AvatarAnimationGraph), Without<LegacyRigPoseApplied>>,
    children: Query<&Children>,
    names: Query<(&Name,&Transform)>,
) {
    use bevy::asset::LoadState;
    for (root,graph) in &roots {
        if !matches!(asset_server.load_state(graph.idle_clip.id()), LoadState::Failed(_)) {
            continue;
        }
        let mut posed=0_usize;
        for entity in children.iter_descendants(root) {
            let Ok((name,transform))=names.get(entity) else {continue};
            let simple: String=name.as_str().chars()
                .filter(|c|c.is_ascii_alphanumeric())
                .flat_map(|c|c.to_lowercase())
                .collect();
            let side = if simple.contains("leftupperarm") || simple.contains("leftarm") {
                Some(-1.0)
            } else if simple.contains("rightupperarm") || simple.contains("rightarm") {
                Some(1.0)
            } else {
                None
            };
            if let Some(direction)=side {
                commands.entity(entity).insert(LegacyArmPose {
                    rest:transform.rotation,
                    direction,
                });
                posed+=1;
            }
        }
        if posed>0 {
            commands.entity(root).insert(LegacyRigPoseApplied);
            warn!("Avatar GLB has no BA_Idle clip; applied animated legacy arms-down pose to {posed} bones. Re-save in Studio to embed animation clips.");
        }
    }
}

pub fn animate_legacy_arm_pose(
    time: Res<Time>,
    mut arms: Query<(&LegacyArmPose,&mut Transform)>,
) {
    let breathe=(time.elapsed_secs()*1.8).sin()*0.035;
    for (arm,mut transform) in &mut arms {
        transform.rotation=arm.rest*Quat::from_rotation_z(arm.direction*(1.0+breathe));
    }
}

/// Drive idle/walk from observed world displacement, NOT locally generated
/// input or animations on the authoritative server. Supports both player
/// avatar and streamed remote avatars without knowing their player IDs.
pub fn update_avatar_animation(
    time: Res<Time>,
    roots: Query<(&GlobalTransform, &AvatarAnimationGraph)>,
    mut players: Query<(
        &mut AnimationPlayer,
        &mut AnimationTransitions,
        &mut AvatarAnimationDrive,
    )>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 { return; }
    for (mut player, mut transitions, mut state) in &mut players {
        let Ok((transform, graph)) = roots.get(state.root) else {continue};
        let now = transform.translation();
        if !state.initialized {
            state.initialized = true;
            state.last_position = now;
            continue;
        }
        let speed = now.distance(state.last_position) / dt;
        state.last_position = now;
        // Hysteresis prevents standing animations from toggling on small
        // server reconciliation corrections / interpolation jitter.
        let moving = if state.moving { speed > 0.18 } else { speed > 0.32 };
        if moving != state.moving {
            state.moving = moving;
            transitions.play(
                &mut player,
                if moving { graph.walk } else { graph.idle },
                Duration::from_millis(170),
            ).repeat();
        }
    }
}
