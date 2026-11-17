//! Bevy 0.20 glTF animation graph for procedural Studio Idle/Walk clips.
//! The rendered character follows the authoritative movement Transform.
//! Animation is presentation only; it cannot grant speed/combat outcomes.

use std::time::Duration;
use std::collections::HashMap;
use crate::camera_rig::{CameraMode, CameraView};
use super::PlayerAvatarMesh;
use bevy::prelude::*;

#[derive(Component)]
pub struct AvatarAnimationGraph {
    handle: Handle<AnimationGraph>,
    idle_clip: Handle<AnimationClip>,
    idle: AnimationNodeIndex,
    walk: AnimationNodeIndex,
}

#[derive(Debug,Clone,Copy,PartialEq,Eq)]
enum JointKind { UpperArm, LowerArm, UpperLeg }

/// Rest/bind rotation captured *before* Bevy applies the first animation.
/// The model is never replaced by a primitive viewmodel in first person.
#[derive(Component)]
pub(crate) struct AvatarJoint {
    root: Entity,
    rest: Quat,
    kind: JointKind,
    // +1 on the left, -1 on the right: rotate a standard +/-X T-pose arm
    // downward around Z in third person.
    sign: f32,
}

#[derive(Component,Default)]
pub(crate) struct AvatarMotion {
    previous: Vec3,
    initialized: bool,
    speed: f32,
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
    commands.entity(root).insert((
        AvatarAnimationGraph {
            handle, idle_clip, idle: nodes[0], walk: nodes[1],
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

fn simplified_bone_name(name: &str) -> String {
    name.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .flat_map(|c| c.to_lowercase())
        .collect()
}

/// Recognize VRoid J_Bip_L_UpperArm, Mixamo mixamorigLeftArm, VRM
/// leftUpperArm, and UpperArm_L. This used to recognize only one of these
/// naming systems; missing bones meant every old GLB stayed in a T-pose.
fn joint_from_name(name: &str) -> Option<(JointKind,f32)> {
    let key = simplified_bone_name(name);
    let part = if key.contains("lowerarm") || key.contains("forearm") {
        JointKind::LowerArm
    } else if key.contains("upperarm")
        || (key.ends_with("leftarm") || key.ends_with("rightarm"))
    {
        JointKind::UpperArm
    } else if key.contains("upperleg") || key.contains("thigh") || key.contains("upleg") {
        JointKind::UpperLeg
    } else {
        return None;
    };
    let part_key = match part {
        JointKind::UpperArm => "upperarm",
        JointKind::LowerArm => if key.contains("forearm") { "forearm" } else { "lowerarm" },
        JointKind::UpperLeg => "upperleg",
    };
    let is_left = key.contains("left") || key.contains(&format!("bipl{part_key}"))
        || key.ends_with(&format!("{part_key}l"))
        || (part==JointKind::UpperLeg && key.contains("biplthigh"));
    let is_right = key.contains("right") || key.contains(&format!("bipr{part_key}"))
        || key.ends_with(&format!("{part_key}r"))
        || (part==JointKind::UpperLeg && key.contains("biprthigh"));
    if is_left == is_right { return None; }
    Some((part, if is_left {1.0} else {-1.0}))
}

/// Traverse one full skinned scene once its nodes appear. Unlike an
/// event-only hook this also handles imported legacy GLBs which do not
/// contain an AnimationPlayer at all.
pub fn discover_avatar_bones(
    mut commands: Commands,
    roots: Query<Entity, With<AvatarAnimationGraph>>,
    children: Query<&Children>,
    bones: Query<(&Name,&Transform),Without<AvatarJoint>>,
) {
    for root in &roots {
        for node in children.iter_descendants(root) {
            let Ok((name,transform)) = bones.get(node) else {continue};
            let Some((kind,sign)) = joint_from_name(name.as_str()) else {continue};
            commands.entity(node).insert(AvatarJoint {
                root, rest:transform.rotation, kind, sign,
            });
        }
    }
}

/// Render-only posing layered after imported glTF animation evaluation.
///
/// BA's oldest character GLBs have no clips; newer ones can have partial
/// clips targeting the torso but not J_Bip arms. Pose the recognized upper
/// arms in both situations and give the legs a small walk cycle when the
/// character moves. It is the SAME actual skinned body in both cameras.
///
/// Runs AFTER AnimationSystems, before transform propagation, so an imported
/// animation cannot reset the arm pose to T-pose on the next frame.
pub fn pose_humanoid_bones(
    time: Res<Time>,
    view: Res<CameraView>,
    mut roots: Query<(Entity,&GlobalTransform,&mut AvatarMotion,Option<&PlayerAvatarMesh>)>,
    mut joints: Query<(&AvatarJoint,&mut Transform)>,
) {
    let dt=time.delta_secs().max(0.0001);
    let mut motion:HashMap<Entity,(f32,bool)> = HashMap::new();
    for (entity,world,mut state,local) in &mut roots {
        let here=world.translation();
        if state.initialized {
            state.speed=(here-state.previous).length()/dt;
        } else {
            state.speed=0.0;
            state.initialized=true;
        }
        state.previous=here;
        motion.insert(entity,(state.speed,local.is_some()));
    }
    let phase=time.elapsed_secs()*9.0;
    let breathe=(time.elapsed_secs()*1.8).sin()*0.025;
    for (joint,mut xf) in &mut joints {
        let Some(&(speed,is_local)) = motion.get(&joint.root) else {continue};
        let walking=speed>0.25;
        let sway=if walking {phase.sin()*0.25}else{0.0};
        match joint.kind {
            JointKind::UpperArm if is_local && view.mode==CameraMode::FirstPerson => {
                // Upper arms extend forward from the character's actual
                // shoulders. Pitch drives the real rig, not a white capsule.
                let reach=(1.27+view.pitch*0.3).clamp(0.80,1.5);
                xf.rotation=joint.rest
                    * Quat::from_rotation_y(-joint.sign*reach)
                    * Quat::from_rotation_z(-joint.sign*(0.30+breathe))
                    * Quat::from_rotation_x(joint.sign*sway*0.2);
            }
            JointKind::LowerArm if is_local && view.mode==CameraMode::FirstPerson => {
                // Bend the player's OWN skinned forearms inward toward the
                // middle of the view (no detached replacement viewmodel).
                xf.rotation=joint.rest
                    * Quat::from_rotation_y(-joint.sign*0.68)
                    * Quat::from_rotation_z(-joint.sign*0.10);
            }
            JointKind::LowerArm => {
                xf.rotation=joint.rest;
            }
            JointKind::UpperArm => {
                xf.rotation=joint.rest
                    * Quat::from_rotation_z(joint.sign*(1.00+breathe))
                    * Quat::from_rotation_x(-joint.sign*sway*0.7);
            }
            JointKind::UpperLeg => {
                xf.rotation=joint.rest
                    * Quat::from_rotation_x(joint.sign*sway);
            }
        }
    }
}

#[cfg(test)]
mod pose_tests {
    use super::*;

    #[test]
    fn recognizes_vrm_vroid_mixamo_and_suffix_arm_names() {
        for (left,right) in [
            ("leftUpperArm","rightUpperArm"),
            ("J_Bip_L_UpperArm","J_Bip_R_UpperArm"),
            ("mixamorigLeftArm","mixamorigRightArm"),
            ("UpperArm_L","UpperArm_R"),
        ] {
            assert_eq!(joint_from_name(left),Some((JointKind::UpperArm,1.0)), "{left}");
            assert_eq!(joint_from_name(right),Some((JointKind::UpperArm,-1.0)), "{right}");
        }
        assert_eq!(joint_from_name("J_Bip_L_UpperLeg"),Some((JointKind::UpperLeg,1.0)));
        assert_eq!(joint_from_name("J_Bip_R_UpperLeg"),Some((JointKind::UpperLeg,-1.0)));
        assert_eq!(joint_from_name("Head"),None);
        assert_eq!(joint_from_name("LeftForeArm"),Some((JointKind::LowerArm,1.0)));
        assert_eq!(joint_from_name("J_Bip_R_LowerArm"),Some((JointKind::LowerArm,-1.0)));
    }

    #[test]
    fn arms_down_rotate_bind_pose_toward_ground() {
        let left=Quat::from_rotation_z(1.0)*Vec3::NEG_X;
        let right=Quat::from_rotation_z(-1.0)*Vec3::X;
        assert!(left.y<0.0 && right.y<0.0);
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
