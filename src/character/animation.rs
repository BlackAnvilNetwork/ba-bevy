//! Bevy 0.20 glTF animation graph for procedural Studio Idle/Walk clips.
//! The rendered character follows the authoritative movement Transform.
//! Animation is presentation only; it cannot grant speed/combat outcomes.

use std::collections::HashMap;
use std::time::Duration;

use super::PlayerAvatarMesh;
use crate::camera_rig::{CameraMode, CameraView};
use bevy::prelude::*;

#[derive(Component)]
pub struct AvatarAnimationGraph {
    handle: Handle<AnimationGraph>,

    idle: AnimationNodeIndex,
    walk: AnimationNodeIndex,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JointKind {
    UpperArm,
    LowerArm,
    UpperLeg,
    LowerLeg,
}

/// Rest/bind rotation captured *before* Bevy applies the first animation.
/// The model is never replaced by a primitive viewmodel in first person.
#[derive(Component)]
pub(crate) struct AvatarJoint {
    root: Entity,
    rest: Quat,
    axis: Vec3,
    kind: JointKind,
    // +1 on the left, -1 on the right: rotate a standard +/-X T-pose arm
    // downward around Z in third person.
    sign: f32,
}

#[derive(Component, Default)]
pub(crate) struct AvatarMotion {
    previous: Vec3,
    initialized: bool,
    speed: f32,
    phase: f32,
    facing: Quat,
    model_yaw: Option<Quat>,
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
    let (graph, nodes) = AnimationGraph::from_clips([idle_clip, walk_clip]);
    let handle = graphs.add(graph);
    commands.entity(root).insert((
        AvatarAnimationGraph {
            handle,
            idle: nodes[0],
            walk: nodes[1],
        },
        AvatarMotion::default(),
    ));
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
                transitions
                    .play(&mut player, graph.idle, Duration::ZERO)
                    .repeat();
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

fn simplified_bone_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Recognize VRoid J_Bip_L_UpperArm, Mixamo mixamorigLeftArm, VRM
/// leftUpperArm, and UpperArm_L. This used to recognize only one of these
/// naming systems; missing bones meant every old GLB stayed in a T-pose.
fn joint_from_name(name: &str) -> Option<(JointKind, f32)> {
    let key = simplified_bone_name(name);
    let part = if key.contains("lowerarm") || key.contains("forearm") || key.ends_with("elbow") {
        JointKind::LowerArm
    } else if key.contains("upperarm") || (key.ends_with("leftarm") || key.ends_with("rightarm")) {
        JointKind::UpperArm
    } else if key.contains("lowerleg")
        || key.contains("calf")
        || key.contains("shin")
        || key.ends_with("knee")
        || key.ends_with("mixamorigleftleg")
        || key.ends_with("mixamorigrightleg")
    {
        JointKind::LowerLeg
    } else if key.contains("upperleg")
        || key.contains("thigh")
        || key.contains("upleg")
        || matches!(key.as_str(), "leftleg" | "rightleg")
    {
        JointKind::UpperLeg
    } else {
        return None;
    };
    let part_key = match part {
        JointKind::UpperArm => "upperarm",
        JointKind::LowerArm => {
            if key.contains("forearm") {
                "forearm"
            } else {
                "lowerarm"
            }
        }
        JointKind::UpperLeg => "upperleg",
        JointKind::LowerLeg => "lowerleg",
    };
    let is_left = key.contains("left")
        || key.contains(&format!("bipl{part_key}"))
        || key.ends_with(&format!("{part_key}l"))
        || (part == JointKind::UpperLeg && key.contains("biplthigh"));
    let is_right = key.contains("right")
        || key.contains(&format!("bipr{part_key}"))
        || key.ends_with(&format!("{part_key}r"))
        || (part == JointKind::UpperLeg && key.contains("biprthigh"));
    if is_left == is_right {
        return None;
    }
    Some((part, if is_left { 1.0 } else { -1.0 }))
}

/// Traverse one full skinned scene once its nodes appear. Unlike an
/// event-only hook this also handles imported legacy GLBs which do not
/// contain an AnimationPlayer at all.
pub fn discover_avatar_bones(
    mut commands: Commands,
    roots: Query<(Entity, Option<&PlayerAvatarMesh>), With<AvatarAnimationGraph>>,
    children: Query<&Children>,
    bones: Query<(&Name, &Transform), Without<AvatarJoint>>,
    nodes: Query<(&Name, &Transform)>,
    eyes: Query<(), With<LocalEye>>,
) {
    for (root, local) in &roots {
        for node in children.iter_descendants(root) {
            let Ok((name, transform)) = bones.get(node) else {
                continue;
            };
            if local.is_some()
                && matches!(
                    simplified_bone_name(name.as_str()).as_str(),
                    "eyeleft"
                        | "eyeright"
                        | "eyel"
                        | "eyer"
                        | "lefteye"
                        | "righteye"
                        | "jadjlfaceeye"
                        | "jadjrfaceeye"
                )
                && eyes.get(node).is_err()
            {
                commands.entity(node).insert(LocalEye);
            }
            let Some((kind, sign)) = joint_from_name(name.as_str()) else {
                continue;
            };
            let next = match kind {
                JointKind::UpperArm => &["lowerarm", "forearm", "elbow"][..],
                JointKind::LowerArm => &["hand", "wrist"],
                JointKind::UpperLeg => &["lowerleg", "shin", "calf", "knee", "leg"],
                JointKind::LowerLeg => &["foot", "ankle"],
            };
            let axis = children
                .get(node)
                .ok()
                .and_then(|kids| {
                    kids.iter().find_map(|e| {
                        let (name, tf) = nodes.get(e).ok()?;
                        next.iter()
                            .any(|part| simplified_bone_name(name.as_str()).contains(part))
                            .then(|| tf.translation.try_normalize())
                            .flatten()
                    })
                })
                .unwrap_or(
                    if matches!(kind, JointKind::UpperLeg | JointKind::LowerLeg) {
                        Vec3::NEG_Y
                    } else {
                        -sign * Vec3::X
                    },
                );
            commands.entity(node).insert(AvatarJoint {
                axis,
                root,
                rest: transform.rotation,
                kind,
                sign,
            });
        }
    }
}

#[derive(Component)]
pub struct LocalEye;

/// Visual swing clock. Local boss strikes and server-resolved zone swings
/// start the same motion, including a miss. Camera cuts do not reset it.
#[derive(Resource, Default)]
pub struct SwingPose {
    started: Option<f32>,
}
impl SwingPose {
    pub fn strike(&mut self, now: f32) {
        self.started = Some(now);
    }
    pub fn preview(&mut self, now: f32) {
        if self.started.is_none_or(|t| now - t >= 0.48) {
            self.strike(now);
        }
    }
    fn amount(&self, now: f32) -> f32 {
        let t = self
            .started
            .map_or(1., |start| ((now - start) / 0.48).clamp(0., 1.));
        // Draw back, cut quickly across the body, then recover.
        if t < 0.28 {
            -0.6 * (t / 0.28)
        } else if t < 0.55 {
            -0.6 + 1.6 * ((t - 0.28) / 0.27)
        } else {
            (1. - t) / 0.45
        }
    }
}

fn model_correction(left_minus_right: Vec3) -> Option<Quat> {
    let lateral = Vec3::new(left_minus_right.x, 0., left_minus_right.z).try_normalize()?;
    let forward = lateral.cross(Vec3::Y);
    Some(Quat::from_rotation_y(-(-forward.x).atan2(-forward.z)))
}

pub fn pose_humanoid_bones(
    time: Res<Time>,
    view: Res<CameraView>,
    swing: Res<SwingPose>,
    mut roots: Query<
        (
            Entity,
            &ChildOf,
            &GlobalTransform,
            &mut Transform,
            &mut AvatarMotion,
            Option<&PlayerAvatarMesh>,
        ),
        Without<AvatarJoint>,
    >,
    actors: Query<&Transform, (Without<AvatarMotion>, Without<AvatarJoint>)>,
    world: Query<&GlobalTransform>,
    mut joints: Query<
        (&AvatarJoint, &ChildOf, &GlobalTransform, &mut Transform),
        Without<AvatarMotion>,
    >,
) {
    let dt = time.delta_secs().max(0.0001);
    let mut frames = HashMap::new();
    for (entity, parent, global, mut visual, mut state, local) in &mut roots {
        let Ok(actor) = actors.get(parent.parent()) else {
            continue;
        };
        if state.model_yaw.is_none() {
            let mut shoulders = [None, None];
            for (joint, _, tf, _) in joints.iter() {
                if joint.root == entity && joint.kind == JointKind::UpperArm {
                    shoulders[usize::from(joint.sign < 0.)] = Some(tf.translation());
                }
            }
            if let [Some(left), Some(right)] = shoulders {
                state.model_yaw = model_correction(global.rotation().inverse() * (left - right));
            }
        }
        let here = actor.translation;
        let step = (here - state.previous) * Vec3::new(1., 0., 1.);
        let first = local.is_some() && view.mode == CameraMode::FirstPerson;
        if !state.initialized || first {
            state.facing = actor.rotation;
        } else if step.length() > dt * 0.12 {
            let heading = Quat::from_rotation_y((-step.x).atan2(-step.z));
            state.facing = state.facing.slerp(heading, 1. - (-16. * dt).exp());
        }
        let speed = if state.initialized {
            step.length() / dt
        } else {
            0.
        };
        state.speed += (speed.min(7.) - state.speed) * (1. - (-15. * dt).exp());
        state.phase = (state.phase + state.speed * dt * 2.8) % std::f32::consts::TAU;
        state.previous = here;
        state.initialized = true;
        let correction = state
            .model_yaw
            .unwrap_or(Quat::from_rotation_y(std::f32::consts::PI));
        visual.rotation = actor.rotation.inverse() * state.facing * correction;
        // GlobalTransforms still describe the previous frame. Express aim in
        // that frame too, cancelling movement of the actor and camera.
        let frame = global.rotation() * correction.inverse();
        frames.insert(
            entity,
            (
                frame,
                state.phase.sin() * (state.speed * 0.11).min(0.73),
                first,
                local.is_some(),
            ),
        );
    }
    for (joint, parent, _, mut tf) in &mut joints {
        let Some(&(frame, gait, first, local)) = frames.get(&joint.root) else {
            continue;
        };
        let Ok(parent_world) = world.get(parent.parent()) else {
            continue;
        };
        let side = -joint.sign;
        let stride = joint.sign * gait;
        let hit = if local && side > 0. {
            swing.amount(time.elapsed_secs())
        } else {
            0.
        };
        let breath = (time.elapsed_secs() * 2.).sin() * 0.025;
        let direction = match joint.kind {
            JointKind::UpperLeg => Vec3::new(side * 0.04, -1., stride),
            JointKind::LowerLeg => Vec3::new(0., -1., stride + (-stride).max(0.) * 1.65),
            JointKind::UpperArm if first => Vec3::new(
                side * 0.34 - hit * 0.65,
                -0.45 + view.pitch * 0.35 + hit * 0.38,
                -0.82 + stride * 0.10,
            ),
            JointKind::LowerArm if first => {
                Vec3::new(-side * 0.20 - hit * 0.35, 0.12 + view.pitch * 0.55, -1.)
            }
            JointKind::UpperArm => Vec3::new(
                side * 0.32 - hit * 0.48,
                -0.92 + hit * 0.48 + breath,
                -stride * 0.68 - hit * 1.1,
            ),
            JointKind::LowerArm => Vec3::new(
                side * 0.12 - hit * 0.38,
                -0.88 + hit * 0.80,
                -0.36 - stride * 0.45 - hit * 0.75,
            ),
        };
        let goal = parent_world.rotation().inverse() * frame * direction.normalize();
        tf.rotation = Quat::from_rotation_arc(joint.rest * joint.axis, goal) * joint.rest;
    }
}

#[cfg(test)]
mod pose_tests {
    use super::*;

    #[test]
    fn recognizes_vrm_vroid_mixamo_and_suffix_arm_names() {
        for (left, right) in [
            ("leftUpperArm", "rightUpperArm"),
            ("J_Bip_L_UpperArm", "J_Bip_R_UpperArm"),
            ("mixamorigLeftArm", "mixamorigRightArm"),
            ("UpperArm_L", "UpperArm_R"),
        ] {
            assert_eq!(
                joint_from_name(left),
                Some((JointKind::UpperArm, 1.0)),
                "{left}"
            );
            assert_eq!(
                joint_from_name(right),
                Some((JointKind::UpperArm, -1.0)),
                "{right}"
            );
        }
        assert_eq!(
            joint_from_name("J_Bip_L_UpperLeg"),
            Some((JointKind::UpperLeg, 1.0))
        );
        assert_eq!(
            joint_from_name("J_Bip_R_UpperLeg"),
            Some((JointKind::UpperLeg, -1.0))
        );
        assert_eq!(joint_from_name("Head"), None);
        assert_eq!(
            joint_from_name("LeftForeArm"),
            Some((JointKind::LowerArm, 1.0))
        );
        assert_eq!(
            joint_from_name("J_Bip_R_LowerArm"),
            Some((JointKind::LowerArm, -1.0))
        );
    }

    #[test]
    fn arms_down_rotate_bind_pose_toward_ground() {
        let left = Quat::from_rotation_z(1.0) * Vec3::NEG_X;
        let right = Quat::from_rotation_z(-1.0) * Vec3::X;
        assert!(left.y < 0.0 && right.y < 0.0);
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
    if dt <= 0.0 {
        return;
    }
    for (mut player, mut transitions, mut state) in &mut players {
        let Ok((transform, graph)) = roots.get(state.root) else {
            continue;
        };
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
        let moving = if state.moving {
            speed > 0.18
        } else {
            speed > 0.32
        };
        if moving != state.moving {
            state.moving = moving;
            transitions
                .play(
                    &mut player,
                    if moving { graph.walk } else { graph.idle },
                    Duration::from_millis(170),
                )
                .repeat();
        }
    }
}

#[cfg(test)]
mod playable_rig {
    use super::*;
    use serde_json::Value;
    use std::path::Path;

    // Use the actual shipped joint hierarchy and bind transforms, including the
    // female elbow/knee names. No renderer or GPU is needed for pose assertions.
    fn play(body: &str) {
        let file = Path::new(env!("CARGO_MANIFEST_DIR")).join(format!(
            "character-studio/public/loot-assets/anata/{body}/Body/{body}.vrm"
        ));
        let glb = std::fs::read(file).unwrap();
        let len = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
        let doc: Value = serde_json::from_slice(&glb[20..20 + len]).unwrap();
        let nodes = doc["nodes"].as_array().unwrap();
        let mut app = App::new();
        app.add_plugins(TransformPlugin)
            .insert_resource(Time::<()>::default())
            .init_resource::<CameraView>()
            .init_resource::<SwingPose>()
            .add_systems(Update, discover_avatar_bones)
            .add_systems(
                PostUpdate,
                pose_humanoid_bones.before(bevy::transform::TransformSystems::Propagate),
            );
        let world = app.world_mut();
        let actor = world.spawn(Transform::default()).id();
        let root = world
            .spawn((
                Transform::default(),
                PlayerAvatarMesh,
                AvatarMotion::default(),
                AvatarAnimationGraph {
                    handle: default(),
                    idle: AnimationNodeIndex::new(0),
                    walk: AnimationNodeIndex::new(1),
                },
                ChildOf(actor),
            ))
            .id();
        let entities: Vec<_> = nodes
            .iter()
            .map(|node| {
                let mut tf = Transform::default();
                if let Some(v) = node["translation"].as_array() {
                    tf.translation =
                        Vec3::from_array(std::array::from_fn(|i| v[i].as_f64().unwrap() as f32));
                }
                if let Some(v) = node["rotation"].as_array() {
                    tf.rotation =
                        Quat::from_array(std::array::from_fn(|i| v[i].as_f64().unwrap() as f32));
                }
                if let Some(v) = node["scale"].as_array() {
                    tf.scale =
                        Vec3::from_array(std::array::from_fn(|i| v[i].as_f64().unwrap() as f32));
                }
                world
                    .spawn((
                        Name::new(node["name"].as_str().unwrap_or("").to_owned()),
                        tf,
                        ChildOf(root),
                    ))
                    .id()
            })
            .collect();
        for (i, node) in nodes.iter().enumerate() {
            for child in node["children"].as_array().into_iter().flatten() {
                world
                    .entity_mut(entities[child.as_u64().unwrap() as usize])
                    .insert(ChildOf(entities[i]));
            }
        }
        let bone = |names: &[&str]| {
            entities[nodes
                .iter()
                .position(|n| {
                    names.contains(&simplified_bone_name(n["name"].as_str().unwrap_or("")).as_str())
                })
                .unwrap()]
        };
        let hand = bone(&["handr", "rightwrist"]);
        let shoulder = bone(&["upperarmr", "rightarm"]);
        let leg = bone(&["upperlegr", "rightleg"]);
        let pos = |a: &App, e| a.world().get::<GlobalTransform>(e).unwrap().translation();
        let tick = |a: &mut App| {
            a.world_mut()
                .resource_mut::<Time>()
                .advance_by(Duration::from_secs_f32(1. / 60.));
            a.update();
        };
        for _ in 0..8 {
            tick(&mut app);
        }
        let reach = pos(&app, hand) - pos(&app, shoulder);
        assert!(
            reach.z < -0.15,
            "{body} first-person hand must extend towards view: {reach:?}"
        );
        app.world_mut().resource_mut::<CameraView>().mode = CameraMode::ThirdPerson;
        for _ in 0..5 {
            tick(&mut app);
        }
        let rest = pos(&app, hand);
        app.world_mut()
            .get_mut::<Transform>(actor)
            .unwrap()
            .rotation = Quat::from_rotation_y(1.8);
        for _ in 0..5 {
            tick(&mut app);
        }
        assert!(
            rest.distance(pos(&app, hand)) < 0.04,
            "{body} follows an idle camera orbit"
        );
        let idle_leg = app.world().get::<Transform>(leg).unwrap().rotation;
        for _ in 0..8 {
            app.world_mut()
                .get_mut::<Transform>(actor)
                .unwrap()
                .translation
                .z -= 0.08;
            tick(&mut app);
        }
        assert!(idle_leg.angle_between(app.world().get::<Transform>(leg).unwrap().rotation) > 0.1);
        for _ in 0..30 {
            tick(&mut app);
        }
        let before = pos(&app, hand);
        let now = app.world().resource::<Time>().elapsed_secs();
        app.world_mut().resource_mut::<SwingPose>().strike(now);
        for _ in 0..15 {
            tick(&mut app);
        }
        assert!(
            before.distance(pos(&app, hand)) > 0.14,
            "{body} strike leaves the hand still"
        );
    }
    #[test]
    fn male_and_female_starter_rigs_move_in_both_views() {
        play("male");
        play("female");
    }
}
