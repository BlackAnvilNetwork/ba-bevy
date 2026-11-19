//! Bevy 0.20 glTF animation graph for procedural Studio Idle/Walk clips.
//! The rendered character follows the authoritative movement Transform.
//! Animation is presentation only; it cannot grant speed/combat outcomes.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration;

use super::PlayerAvatarMesh;
use crate::camera_rig::{CameraMode, CameraView};
use crate::rpg::JobId;
use bevy::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MotionClip {
    Idle = 0,
    Walk,
    Run,
    Crouch,
    CrouchWalk,
    Jump,
    Punch,
    Slash1H,
    Slash2H,
    Roll,
    WalkToRun,
    RunToWalk,
    WalkToJump,
    RunToJump,
    JumpToWalk,
    JumpToRun,
    WalkPunch,
    WalkJump,
    Rest,
    IdleToWalk,
    WalkToIdle,
    IdleToRest,
    RestToIdle,
    WalkBack,
    StrafeLeft,
    StrafeRight,
    IdleToCrouch,
    CrouchToIdle,
    WalkSlash1H,
    CrouchToCrouchWalk,
    CrouchWalkToCrouch,
    IdleToWalkBack,
    WalkBackToIdle,
    IdleToStrafeLeft,
    StrafeLeftToIdle,
    IdleToStrafeRight,
    StrafeRightToIdle,
    WalkSlash2H,
}

const MOTION_CLIP_COUNT: usize = 38;

const MOTION_ASSETS: [&str; MOTION_CLIP_COUNT] = [
    "character/motion/idle.glb",
    "character/motion/walk.glb",
    "character/motion/run.glb",
    "character/motion/crouch.glb",
    "character/motion/crouch_walk.glb",
    "character/motion/jump.glb",
    "character/motion/punch.glb",
    "character/motion/slash_1h.glb",
    "character/motion/slash_2h.glb",
    "character/motion/roll.glb",
    "character/motion/walk_to_run.glb",
    "character/motion/run_to_walk.glb",
    "character/motion/walk_to_jump.glb",
    "character/motion/run_to_jump.glb",
    "character/motion/jump_to_walk.glb",
    "character/motion/jump_to_run.glb",
    "character/motion/walk_punch.glb",
    "character/motion/walk_jump.glb",
    "character/motion/rest.glb",
    "character/motion/idle_to_walk.glb",
    "character/motion/walk_to_idle.glb",
    "character/motion/idle_to_rest.glb",
    "character/motion/rest_to_idle.glb",
    "character/motion/walk_back.glb",
    "character/motion/strafe_left.glb",
    "character/motion/strafe_right.glb",
    "character/motion/idle_to_crouch.glb",
    "character/motion/crouch_to_idle.glb",
    "character/motion/walk_slash_1h.glb",
    "character/motion/crouch_to_crouch_walk.glb",
    "character/motion/crouch_walk_to_crouch.glb",
    "character/motion/idle_to_walk_back.glb",
    "character/motion/walk_back_to_idle.glb",
    "character/motion/idle_to_strafe_left.glb",
    "character/motion/strafe_left_to_idle.glb",
    "character/motion/idle_to_strafe_right.glb",
    "character/motion/strafe_right_to_idle.glb",
    "character/motion/walk_slash_2h.glb",
];

#[derive(Component)]
pub struct AvatarAnimationGraph {
    handle: Handle<AnimationGraph>,
    node: [AnimationNodeIndex; MOTION_CLIP_COUNT],
    present: [bool; MOTION_CLIP_COUNT],
}

impl AvatarAnimationGraph {
    fn dummy() -> Self {
        Self {
            handle: default(),
            node: [AnimationNodeIndex::new(0); MOTION_CLIP_COUNT],
            present: [false; MOTION_CLIP_COUNT],
        }
    }

    fn node(&self, clip: MotionClip) -> AnimationNodeIndex {
        let i = clip as usize;
        if self.present[i] {
            self.node[i]
        } else if self.present[MotionClip::Walk as usize] {
            self.node[MotionClip::Walk as usize]
        } else {
            self.node[MotionClip::Idle as usize]
        }
    }

    fn has(&self, clip: MotionClip) -> bool {
        self.present[clip as usize]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum JointKind {
    UpperArm,
    LowerArm,
    Hand,
    Finger,
    UpperLeg,
    LowerLeg,
    Head,
    Neck,
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
    /// Bind-local finger flex in radians. Zero on arms/legs/wrists.
    curl: f32,
    /// Thumb opposition in radians. Applied around bind Y, signed by `sign`.
    oppose: f32,
    /// 0 thumb, 1 index … 4 little. Unused on arms/legs.
    digit: u8,
    /// 0 MCP / metacarpal, 1 PIP, 2 DIP.
    chain: u8,
}

#[derive(Component, Default)]
pub(crate) struct AvatarMotion {
    previous: Vec3,
    initialized: bool,
    speed: f32,
    phase: f32,
    facing: Quat,
    model_yaw: Option<Quat>,
    /// True when third person plays an authored walk clip instead of posing.
    authored_walk: bool,
    /// Close the fingers for unarmed punch clips. Presentation only.
    fist: bool,
    /// Idle/rest/crouch: hold neck and head at bind so the face is level.
    level_head: bool,
    /// One-foot jump clip is playing. Presentation hop only.
    jumping: bool,
    jump_phase: f32,
}

#[derive(Component)]
pub(crate) struct AvatarAnimationDrive {
    root: Entity,
    last_position: Vec3,
    initialized: bool,
    moving: bool,
    clip: MotionClip,
    oneshot: bool,
    last_swing: f32,
    queue: Vec<MotionClip>,
}

const AUTHORED_WALK: &str = "character/motion/walk.glb";

fn motion_asset_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets")
        .join(rel)
}

fn authored_walk_on_disk() -> bool {
    motion_asset_path(AUTHORED_WALK).is_file()
}

fn load_motion_clip(
    asset_server: &AssetServer,
    rel: &str,
    fallback: &Handle<AnimationClip>,
) -> (Handle<AnimationClip>, bool) {
    if rel.is_empty() {
        return (fallback.clone(), false);
    }
    if motion_asset_path(rel).is_file() {
        (
            asset_server.load(GltfAssetLabel::Animation(0).from_asset(rel.to_string())),
            true,
        )
    } else {
        (fallback.clone(), false)
    }
}

fn attack_clip(job: JobId) -> MotionClip {
    match job {
        JobId::Fighter => MotionClip::Slash2H,
        JobId::Knight | JobId::Rogue | JobId::Cleric | JobId::BattleMage => MotionClip::Slash1H,
        _ => MotionClip::Punch,
    }
}

fn attack_clip_for(job: JobId, moving: bool, graph: &AvatarAnimationGraph) -> MotionClip {
    let standing = attack_clip(job);
    if !moving {
        return standing;
    }
    match standing {
        MotionClip::Punch if graph.has(MotionClip::WalkPunch) => MotionClip::WalkPunch,
        MotionClip::Slash1H if graph.has(MotionClip::WalkSlash1H) => MotionClip::WalkSlash1H,
        MotionClip::Slash2H if graph.has(MotionClip::WalkSlash2H) => MotionClip::WalkSlash2H,
        _ => standing,
    }
}

fn is_looping(clip: MotionClip) -> bool {
    matches!(
        clip,
        MotionClip::Idle
            | MotionClip::Walk
            | MotionClip::Run
            | MotionClip::Crouch
            | MotionClip::CrouchWalk
            | MotionClip::Rest
            | MotionClip::WalkBack
            | MotionClip::StrafeLeft
            | MotionClip::StrafeRight
    )
}

/// Tide walk/run (and their gait bridges) actually move the legs. Walk-back and
/// strafe cycles on disk are near-idle, so third person must not skip posing
/// while those are active — otherwise A/S/D slide with a frozen stance.
struct StrideSync<'w> {
    motion: Mut<'w, AvatarMotion>,
    clip: MotionClip,
    moving: bool,
}

impl Drop for StrideSync<'_> {
    fn drop(&mut self) {
        self.motion.authored_walk = clip_owns_limbs(self.clip, self.moving);
        self.motion.fist = unarmed_fist(self.clip);
        self.motion.level_head = levels_head(self.clip);
        self.motion.jumping = jump_hop_clip(self.clip);
    }
}

fn clip_drives_stride(clip: MotionClip) -> bool {
    matches!(
        clip,
        MotionClip::Walk
            | MotionClip::Run
            | MotionClip::CrouchWalk
            | MotionClip::WalkPunch
            | MotionClip::WalkJump
            | MotionClip::WalkSlash1H
            | MotionClip::WalkSlash2H
            | MotionClip::IdleToWalk
            | MotionClip::WalkToIdle
            | MotionClip::WalkToRun
            | MotionClip::RunToWalk
            | MotionClip::WalkToJump
            | MotionClip::RunToJump
            | MotionClip::JumpToWalk
            | MotionClip::JumpToRun
            | MotionClip::CrouchToCrouchWalk
            | MotionClip::CrouchWalkToCrouch
    )
}

fn unarmed_fist(clip: MotionClip) -> bool {
    matches!(clip, MotionClip::Punch | MotionClip::WalkPunch)
}

/// Keep the boxing combo playing; retriggers must not rewind to the first jab.
fn keeps_unarmed_combo(clip: MotionClip, oneshot: bool) -> bool {
    oneshot && unarmed_fist(clip)
}

fn levels_head(clip: MotionClip) -> bool {
    matches!(clip, MotionClip::Idle | MotionClip::Rest | MotionClip::Crouch)
}

fn jump_hop_clip(clip: MotionClip) -> bool {
    matches!(clip, MotionClip::Jump | MotionClip::WalkJump)
}

/// Tide owns the body except dead walk-back/strafe (and idle while sliding).
fn clip_owns_limbs(clip: MotionClip, moving: bool) -> bool {
    if moving
        && !clip_drives_stride(clip)
        && matches!(
            clip,
            MotionClip::Idle
                | MotionClip::WalkBack
                | MotionClip::StrafeLeft
                | MotionClip::StrafeRight
        )
    {
        false
    } else {
        true
    }
}

fn pose_finger(joint: &AvatarJoint, fist: bool) -> Quat {
    let (curl_n, oppose_n) = if fist {
        fist_angles(joint.digit, joint.chain)
    } else {
        (joint.curl, joint.oppose)
    };
    let hinge = joint.axis.cross(Vec3::NEG_Y).normalize_or_zero();
    let curl = Quat::from_axis_angle(hinge, curl_n);
    let oppose = Quat::from_axis_angle(Vec3::Y, joint.sign * oppose_n);
    joint.rest * oppose * curl
}

/// Closed-fist flex per joint. PIP locks the hand shut; thumb wraps across.
fn fist_angles(digit: u8, chain: u8) -> (f32, f32) {
    if digit == 0 {
        return match chain {
            0 => (0.55, 1.15),
            1 => (1.05, 0.22),
            _ => (1.20, 0.0),
        };
    }
    let tight = (digit as f32 - 1.0) * 0.05;
    match chain {
        0 => (1.42 + tight, 0.0),
        1 => (1.82 + tight, 0.0),
        _ => (1.38 + tight, 0.0),
    }
}

fn clip_playback_speed(clip: MotionClip) -> f32 {
    if clip == MotionClip::Rest {
        0.2
    } else {
        1.0
    }
}

fn play_motion(
    player: &mut AnimationPlayer,
    transitions: &mut AnimationTransitions,
    graph: &AvatarAnimationGraph,
    clip: MotionClip,
    blend_ms: u64,
) {
    let active = transitions.play(player, graph.node(clip), Duration::from_millis(blend_ms));
    active.set_speed(clip_playback_speed(clip));
    if is_looping(clip) {
        active.repeat();
    }
}

/// Attach before a WorldAssetRoot finishes loading. Idle comes from the
/// character GLB Animation(0). Other clips prefer retargeted Tide files under
/// `character/motion/` so third person can stop posing the limbs.
/// Older static GLBs simply never create an AnimationPlayer and remain visible.
pub fn prepare_avatar_animation(
    commands: &mut Commands,
    asset_server: &AssetServer,
    graphs: &mut Assets<AnimationGraph>,
    root: Entity,
    glb_uri: &str,
) {
    let studio_idle: Handle<AnimationClip> =
        asset_server.load(GltfAssetLabel::Animation(0).from_asset(glb_uri.to_string()));
    let (idle_clip, idle_authored) =
        load_motion_clip(asset_server, MOTION_ASSETS[0], &studio_idle);
    let authored = authored_walk_on_disk();
    let walk_clip: Handle<AnimationClip> = if authored {
        info!("Third-person motion uses authored clips under character/motion/");
        asset_server.load(GltfAssetLabel::Animation(0).from_asset(AUTHORED_WALK.to_string()))
    } else {
        asset_server.load(GltfAssetLabel::Animation(1).from_asset(glb_uri.to_string()))
    };
    let mut clips = vec![idle_clip, walk_clip.clone()];
    let mut present = [false; MOTION_CLIP_COUNT];
    present[MotionClip::Idle as usize] = true;
    present[MotionClip::Walk as usize] = true;
    if idle_authored {
        info!("Idle uses authored rest clip {}", MOTION_ASSETS[0]);
    }
    for i in 2..MOTION_CLIP_COUNT {
        let (clip, found) = load_motion_clip(asset_server, MOTION_ASSETS[i], &walk_clip);
        clips.push(clip);
        present[i] = found;
    }
    let (graph, nodes) = AnimationGraph::from_clips(clips);
    let mut node = [AnimationNodeIndex::new(0); MOTION_CLIP_COUNT];
    for (i, n) in nodes.into_iter().enumerate() {
        node[i] = n;
    }
    let handle = graphs.add(graph);
    commands.entity(root).insert((
        AvatarAnimationGraph {
            handle,
            node,
            present,
        },
        AvatarMotion {
            authored_walk: authored,
            ..default()
        },
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
                    .play(&mut player, graph.node(MotionClip::Idle), Duration::ZERO)
                    .repeat();
                commands.entity(descendant).insert((
                    AnimationGraphHandle(graph.handle.clone()),
                    transitions,
                    AvatarAnimationDrive {
                        root,
                        last_position: Vec3::ZERO,
                        initialized: false,
                        moving: false,
                        clip: MotionClip::Idle,
                        oneshot: false,
                        last_swing: -1.0,
                        queue: Vec::new(),
                    },
                ));
                info!("Bound authored motion graph to skinned avatar player {descendant:?}");
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
    let part = if key == "head" || key.ends_with("head") && !key.contains("hand") {
        JointKind::Head
    } else if key.contains("neck") {
        JointKind::Neck
    } else if key.contains("lowerarm") || key.contains("forearm") || key.ends_with("elbow") {
        JointKind::LowerArm
    } else if key.contains("upperarm") || (key.ends_with("leftarm") || key.ends_with("rightarm")) {
        JointKind::UpperArm
    } else if key.contains("thumb")
        || key.contains("index")
        || key.contains("middle")
        || key.contains("ring")
        || key.contains("pinky")
        || key.contains("little")
    {
        JointKind::Finger
    } else if key.contains("hand") || key.contains("wrist") {
        JointKind::Hand
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
        JointKind::Hand => {
            if key.contains("wrist") {
                "wrist"
            } else {
                "hand"
            }
        }
        JointKind::Finger => {
            if key.contains("thumb") {
                "thumb"
            } else if key.contains("index") {
                "index"
            } else if key.contains("middle") {
                "middle"
            } else if key.contains("ring") {
                "ring"
            } else if key.contains("pinky") {
                "pinky"
            } else {
                "little"
            }
        }
        JointKind::UpperLeg => "upperleg",
        JointKind::LowerLeg => "lowerleg",
        JointKind::Head => "head",
        JointKind::Neck => "neck",
    };
    if matches!(part, JointKind::Head | JointKind::Neck) {
        return Some((part, 0.0));
    }
    let is_left = key.contains("left")
        || key.contains(&format!("bipl{part_key}"))
        || key.ends_with(&format!("{part_key}l"))
        || (part == JointKind::UpperLeg && key.contains("biplthigh"))
        || (part == JointKind::Finger && key.ends_with('l'));
    let is_right = key.contains("right")
        || key.contains(&format!("bipr{part_key}"))
        || key.ends_with(&format!("{part_key}r"))
        || (part == JointKind::UpperLeg && key.contains("biprthigh"))
        || (part == JointKind::Finger && key.ends_with('r'));
    if is_left == is_right {
        return None;
    }
    Some((part, if is_left { 1.0 } else { -1.0 }))
}

fn finger_layout(name: &str) -> (u8, u8) {
    let key = simplified_bone_name(name);
    if key.contains("thumb") {
        let chain = if key.contains("distal") || key.contains("thumb2") {
            2
        } else if key.contains("proximal") || key.contains("thumb1") {
            1
        } else {
            0
        };
        return (0, chain);
    }
    let digit: u8 = if key.contains("index") {
        1
    } else if key.contains("middle") {
        2
    } else if key.contains("ring") {
        3
    } else {
        4
    };
    let chain: u8 = if key.contains("distal") || key.contains("finger3") {
        2
    } else if key.contains("intermediate") || key.contains("finger2") {
        1
    } else if key.contains('3') {
        2
    } else if key.contains('2') {
        1
    } else {
        0
    };
    (digit, chain)
}

/// Bind-local flex for a finger bone. MCP < PIP, DIP smallest; ring/little
/// curl more than the index so a relaxed hand isn't a rake or a fist.
fn finger_curl(name: &str) -> (f32, f32, u8, u8) {
    let (digit, chain) = finger_layout(name);
    let digit_i = digit as usize;
    let chain_i = chain as usize;
    const CURL: [[f32; 3]; 5] = [
        [0.20, 0.36, 0.22],
        [0.30, 0.46, 0.24],
        [0.34, 0.52, 0.28],
        [0.40, 0.58, 0.32],
        [0.46, 0.62, 0.34],
    ];
    let oppose = if digit == 0 && chain == 0 { 0.28 } else { 0. };
    (CURL[digit_i][chain_i], oppose, digit, chain)
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
                JointKind::Hand => &["index", "middle", "finger"],
                JointKind::Finger => &[
                    "intermediate",
                    "distal",
                    "proximal",
                    "finger2",
                    "finger3",
                    "thumb1",
                    "thumb2",
                ],
                JointKind::UpperLeg => &["lowerleg", "shin", "calf", "knee", "leg"],
                JointKind::LowerLeg => &["foot", "ankle"],
                JointKind::Neck => &["head"],
                JointKind::Head => &["eye", "jaw"],
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
                .unwrap_or(match kind {
                    JointKind::UpperLeg | JointKind::LowerLeg => Vec3::NEG_Y,
                    // Chrona / VRM fingers extend along +X (left) / -X (right).
                    // Distal bones have no child, so the arm fallback (-X on the
                    // left) curled fingertips open and the punch never read as a fist.
                    JointKind::Finger => sign * Vec3::X,
                    JointKind::Head | JointKind::Neck => Vec3::Y,
                    _ => -sign * Vec3::X,
                });
            let (curl, oppose, digit, chain) = if kind == JointKind::Finger {
                finger_curl(name.as_str())
            } else {
                (0., 0., 0, 0)
            };
            commands.entity(node).insert(AvatarJoint {
                axis,
                root,
                rest: transform.rotation,
                kind,
                sign,
                curl,
                oppose,
                digit,
                chain,
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
    pub fn started_at(&self) -> Option<f32> {
        self.started
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

/// Unarmed: right jab then left cross on the same 0.48s swing. Armed: right only.
fn swing_arm_amount(swing: &SwingPose, now: f32, side: f32, fist: bool) -> f32 {
    if fist {
        let delayed = if side > 0. { now } else { now - 0.16 };
        swing.amount(delayed)
    } else if side > 0. {
        swing.amount(now)
    } else {
        0.
    }
}

fn model_correction(left_minus_right: Vec3) -> Option<Quat> {
    let lateral = Vec3::new(left_minus_right.x, 0., left_minus_right.z).try_normalize()?;
    let forward = lateral.cross(Vec3::Y);
    Some(Quat::from_rotation_y(-(-forward.x).atan2(-forward.z)))
}

fn wrap01(t: f32) -> f32 {
    t.rem_euclid(1.)
}

/// Dive hop, then tuck near the ground. Presentation only; combat roll is unchanged.
fn roll_root(progress: f32) -> (Vec3, Quat) {
    let t = progress.clamp(0., 1.);
    let hop = if t < 0.22 {
        0.36 * (t / 0.22)
    } else if t < 0.58 {
        0.36 * (1.0 - (t - 0.22) / 0.36)
    } else {
        0.0
    };
    let spin = Quat::from_rotation_x(-std::f32::consts::TAU * t);
    (Vec3::Y * (-1.65 + hop), spin)
}

/// Upward one-foot takeoff. Space jump is presentation only.
fn jump_hop(phase: f32) -> f32 {
    (phase.clamp(0., 1.) * std::f32::consts::PI).sin() * 0.82
}

/// Free leg tucks at the peak. Right takeoff stays on the authored clip.
fn jump_tuck_leg(joint: &AvatarJoint, phase: f32) -> Quat {
    let k = (phase.clamp(0., 1.) * std::f32::consts::PI).sin();
    let hinge = joint.axis.cross(Vec3::NEG_Z).normalize_or_zero();
    let hinge = if hinge.length_squared() < 0.01 {
        Vec3::X
    } else {
        hinge
    };
    let flex = if joint.kind == JointKind::LowerLeg {
        2.05
    } else {
        1.15
    };
    // Hip flexes the thigh forward; the shin must rotate the other way or the
    // knee hyperextends and the shin folds past the joint.
    let signed = if joint.kind == JointKind::LowerLeg {
        k * flex
    } else {
        -k * flex
    };
    joint.rest * Quat::from_axis_angle(hinge, signed)
}

/// Smooth keyframes around a 0..1 loop. Used for a walk cycle, not a sine swing.
fn cycle_pose(u: f32, keys: &[(f32, Vec3)]) -> Vec3 {
    let u = wrap01(u);
    let n = keys.len();
    for i in 0..n {
        let (t0, a) = keys[i];
        let (mut t1, b) = keys[(i + 1) % n];
        let mut sample = u;
        if t1 <= t0 {
            t1 += 1.;
            if sample < t0 {
                sample += 1.;
            }
        }
        if sample >= t0 && sample <= t1 {
            let s = ((sample - t0) / (t1 - t0).max(1e-4)).clamp(0., 1.);
            let s = s * s * (3. - 2. * s);
            return a.lerp(b, s);
        }
    }
    keys[0].1
}

fn walk_blend(amp: f32) -> f32 {
    (amp / 0.32).clamp(0., 1.)
}

/// One leg through contact → stance → toe-off → swing → reach.
fn walk_leg(lower: bool, u: f32, side: f32, amp: f32) -> Vec3 {
    let stand = if lower {
        Vec3::new(0., -1., 0.)
    } else {
        Vec3::new(side * 0.04, -1., 0.)
    };
    let keys: [(f32, Vec3); 6] = if lower {
        [
            (0.00, Vec3::new(0., -1.00, 0.08)),
            (0.16, Vec3::new(0., -1.00, 0.02)),
            (0.32, Vec3::new(0., -1.00, 0.06)),
            (0.48, Vec3::new(0., -0.94, 0.40)),
            (0.66, Vec3::new(0., -0.55, 1.18)),
            (0.86, Vec3::new(0., -0.90, 0.42)),
        ]
    } else {
        [
            (0.00, Vec3::new(side * 0.04, -1.00, 0.42)),
            (0.16, Vec3::new(side * 0.04, -1.00, 0.10)),
            (0.32, Vec3::new(side * 0.04, -1.00, -0.18)),
            (0.48, Vec3::new(side * 0.04, -0.96, -0.58)),
            (0.66, Vec3::new(side * 0.04, -0.68, 0.14)),
            (0.86, Vec3::new(side * 0.04, -0.86, 0.54)),
        ]
    };
    stand.lerp(cycle_pose(u, &keys), walk_blend(amp))
}

/// Opposite arm to the same-side leg, elbows kept bent.
fn walk_arm(lower: bool, u: f32, side: f32, amp: f32, hit: f32, breath: f32) -> Vec3 {
    let stand = if lower {
        Vec3::new(side * 0.08, -0.48 + breath, -0.72)
    } else {
        Vec3::new(side * 0.26, -0.78 + breath, 0.)
    };
    let keys: [(f32, Vec3); 5] = if lower {
        [
            (0.00, Vec3::new(side * 0.08, -0.50 + breath, -0.70)),
            (0.22, Vec3::new(side * 0.08, -0.48 + breath, -0.74)),
            (0.48, Vec3::new(side * 0.06, -0.40 + breath, -0.86)),
            (0.70, Vec3::new(side * 0.08, -0.46 + breath, -0.76)),
            (0.90, Vec3::new(side * 0.08, -0.50 + breath, -0.72)),
        ]
    } else {
        [
            (0.00, Vec3::new(side * 0.26, -0.78 + breath, -0.46)),
            (0.22, Vec3::new(side * 0.26, -0.80 + breath, -0.12)),
            (0.48, Vec3::new(side * 0.24, -0.74 + breath, 0.40)),
            (0.70, Vec3::new(side * 0.26, -0.78 + breath, 0.08)),
            (0.90, Vec3::new(side * 0.26, -0.78 + breath, -0.28)),
        ]
    };
    let mut pose = stand.lerp(cycle_pose(u, &keys), walk_blend(amp));
    if !lower {
        pose += Vec3::new(-side * hit * 0.48, hit * 0.48, -hit * 1.1);
    } else {
        pose += Vec3::new(-side * hit * 0.38, hit * 0.80, -hit * 0.75);
    }
    pose
}

fn fight_body_facing(
    net: Option<&crate::net::ZoneNet>,
    swing: &SwingPose,
    now: f32,
    here: Vec3,
) -> Option<Quat> {
    let net = net.filter(|n| n.online())?;
    if !swing
        .started_at()
        .is_some_and(|start| now - start < 1.2)
    {
        return None;
    }
    let id = net.engage?;
    let (x, z) = crate::net::living_foe_xz(net, id)?;
    let dx = x - here.x;
    let dz = z - here.z;
    if dx * dx + dz * dz > 144.0 {
        return None;
    }
    crate::net::combat_yaw(here.x, here.z, x, z).map(Quat::from_rotation_y)
}

pub fn pose_humanoid_bones(
    zone: Option<Res<crate::net::ZoneNet>>,
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
        let fight_face = if local.is_some() && !first {
            fight_body_facing(zone.as_deref(), &swing, time.elapsed_secs(), here)
        } else {
            None
        };
        if let Some(want) = fight_face {
            // Body yaws toward the foe. Player/camera yaw stays on the mouse.
            state.facing = state.facing.slerp(want, 1. - (-18. * dt).exp());
        } else if !state.initialized || first {
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
        if local.is_some() {
            let progress = zone
                .as_ref()
                .filter(|z| z.online() && z.marsh.rolling)
                .map(|z| z.marsh.roll_progress);
            let (offset, tumble) = roll_root(progress.unwrap_or(0.0));
            visual.translation = offset;
            visual.rotation = tumble * visual.rotation;
        }
        if state.jumping {
            state.jump_phase = (state.jump_phase + dt / 1.13).min(1.0);
            visual.translation.y += jump_hop(state.jump_phase);
        } else {
            state.jump_phase = 0.0;
        }
        // GlobalTransforms still describe the previous frame. Express aim in
        // that frame too, cancelling movement of the actor and camera.
        let frame = global.rotation() * correction.inverse();
        let amp = (state.speed * 0.13).min(0.82);
        if !first && !state.authored_walk {
            let cadence = (state.speed * 0.22).min(1.);
            visual.translation.y += -(state.phase * 2.).cos() * 0.022 * cadence;
            visual.rotation *= Quat::from_rotation_y((-state.phase).sin() * 0.10 * cadence);
        }
        frames.insert(
            entity,
            (
                frame,
                state.phase,
                amp,
                first,
                local.is_some(),
                state.authored_walk,
                state.fist,
                state.level_head,
                state.jumping,
                state.jump_phase,
            ),
        );
    }
    for (joint, parent, _, mut tf) in &mut joints {
        let Some(&(
            frame,
            phase,
            amp,
            first,
            local,
            authored_walk,
            fist,
            level_head,
            jumping,
            jump_phase,
        )) = frames.get(&joint.root)
        else {
            continue;
        };
        if jumping && matches!(joint.kind, JointKind::UpperLeg | JointKind::LowerLeg) {
            if joint.sign > 0. {
                tf.rotation = jump_tuck_leg(joint, jump_phase);
            }
            continue;
        }
        let head_joint = matches!(joint.kind, JointKind::Head | JointKind::Neck);
        // Authored boxing owns the limbs. First person still poses idle/walk,
        // but not during punch — that was replacing the clip with a limp swing.
        let clip_limb = authored_walk
            && joint.kind != JointKind::Finger
            && !(level_head && head_joint);
        if clip_limb && (!first || fist) {
            continue;
        }
        let Ok(parent_world) = world.get(parent.parent()) else {
            continue;
        };
        let side = -joint.sign;
        let stride = joint.sign * phase.sin() * amp;
        let step = wrap01(
            phase / std::f32::consts::TAU + if joint.sign < 0. { 0.5 } else { 0. },
        );
        let hit = if local {
            swing_arm_amount(&swing, time.elapsed_secs(), side, fist)
        } else {
            0.
        };
        let breath = (time.elapsed_secs() * 2.).sin() * 0.025;
        if matches!(joint.kind, JointKind::Head | JointKind::Neck) {
            if level_head {
                tf.rotation = joint.rest;
            }
            continue;
        }
        if joint.kind == JointKind::Hand {
            if first {
                let n = if joint.axis.x.abs() < 0.9 {
                    Vec3::X
                } else {
                    Vec3::Y
                };
                let flex_axis = joint.axis.cross(n).normalize_or_zero();
                let flex = Quat::from_axis_angle(flex_axis, 0.18);
                let roll = Quat::from_axis_angle(joint.axis, side * 0.22);
                tf.rotation = joint.rest * roll * flex;
            } else {
                tf.rotation = joint.rest;
            }
            continue;
        }
        if joint.kind == JointKind::Finger {
            if first || fist {
                tf.rotation = pose_finger(joint, fist);
            } else if authored_walk {
                continue;
            } else {
                tf.rotation = joint.rest;
            }
            continue;
        }
        let direction = match joint.kind {
            JointKind::UpperLeg if first => Vec3::new(side * 0.04, -1., stride),
            JointKind::LowerLeg if first => {
                Vec3::new(0., -1., stride + (-stride).max(0.) * 1.65)
            }
            JointKind::UpperLeg => walk_leg(false, step, side, amp),
            JointKind::LowerLeg => walk_leg(true, step, side, amp),
            // Upper arm hangs, forearm comes forward — that angle is the elbow.
            JointKind::UpperArm if first => Vec3::new(
                side * (0.20 - hit * 0.48),
                -0.68 + view.pitch * 0.18 + hit * 0.30 + breath,
                -0.52 + stride * 0.28,
            ),
            JointKind::LowerArm if first => Vec3::new(
                -side * (0.04 + hit * 0.30),
                0.20 + view.pitch * 0.22,
                -0.96 - stride * 0.10,
            ),
            JointKind::UpperArm => walk_arm(false, step, side, amp, hit, breath),
            JointKind::LowerArm => walk_arm(true, step, side, amp, hit, breath),
            JointKind::Hand | JointKind::Finger | JointKind::Head | JointKind::Neck => Vec3::NEG_Z,
        };
        let goal = parent_world.rotation().inverse() * frame * direction.normalize();
        let mut rot = Quat::from_rotation_arc(joint.rest * joint.axis, goal) * joint.rest;
        if first && joint.kind == JointKind::LowerArm {
            rot *= Quat::from_axis_angle(joint.axis, side * 0.45);
        }
        tf.rotation = rot;
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
        assert_eq!(joint_from_name("Head"), Some((JointKind::Head, 0.0)));
        assert_eq!(joint_from_name("neck"), Some((JointKind::Neck, 0.0)));
        assert!(levels_head(MotionClip::Idle));
        assert!(levels_head(MotionClip::Rest));
        assert!(!levels_head(MotionClip::Walk));
        assert_eq!(
            joint_from_name("LeftForeArm"),
            Some((JointKind::LowerArm, 1.0))
        );
        assert_eq!(
            joint_from_name("J_Bip_R_LowerArm"),
            Some((JointKind::LowerArm, -1.0))
        );
        assert_eq!(
            joint_from_name("LeftHand"),
            Some((JointKind::Hand, 1.0))
        );
        assert_eq!(
            joint_from_name("J_Bip_R_Hand"),
            Some((JointKind::Hand, -1.0))
        );
        assert_eq!(
            joint_from_name("index.proximal.L"),
            Some((JointKind::Finger, 1.0))
        );
        assert_eq!(
            joint_from_name("IndexFinger1_R"),
            Some((JointKind::Finger, -1.0))
        );
        assert_eq!(
            joint_from_name("Thumb0_L"),
            Some((JointKind::Finger, 1.0))
        );
        assert_eq!(
            joint_from_name("little.distal.R"),
            Some((JointKind::Finger, -1.0))
        );
        assert_eq!(
            joint_from_name("LeftHandIndex1"),
            Some((JointKind::Finger, 1.0))
        );
    }

    #[test]
    fn arms_down_rotate_bind_pose_toward_ground() {
        let left = Quat::from_rotation_z(1.0) * Vec3::NEG_X;
        let right = Quat::from_rotation_z(-1.0) * Vec3::X;
        assert!(left.y < 0.0 && right.y < 0.0);
    }

    #[test]
    fn third_person_walk_plants_then_lifts_the_knee() {
        let contact = walk_leg(false, 0.0, 1.0, 0.8);
        let toe_off = walk_leg(false, 0.48, 1.0, 0.8);
        assert!(contact.z > 0.2, "heel strike reaches forward: {contact:?}");
        assert!(toe_off.z < -0.2, "toe-off trails behind: {toe_off:?}");
        let knee_swing = walk_leg(true, 0.66, 1.0, 0.8);
        let knee_stance = walk_leg(true, 0.16, 1.0, 0.8);
        assert!(
            knee_swing.z > knee_stance.z + 0.4,
            "swing knee comes forward of the planted leg: swing {knee_swing:?} stance {knee_stance:?}"
        );
    }

    #[test]
    fn fight_body_facing_is_off_without_a_swing() {
        let swing = SwingPose::default();
        assert!(fight_body_facing(None, &swing, 1.0, Vec3::ZERO).is_none());
    }

    #[test]
    fn attack_clip_follows_job_weapon() {
        assert_eq!(attack_clip(JobId::TimeMage), MotionClip::Punch);
        assert_eq!(attack_clip(JobId::Knight), MotionClip::Slash1H);
        assert_eq!(attack_clip(JobId::Fighter), MotionClip::Slash2H);
        assert_eq!(bridge_between(MotionClip::Walk, MotionClip::Run), None);
        assert_eq!(
            bridge_between(MotionClip::Run, MotionClip::Walk),
            Some(MotionClip::RunToWalk)
        );
        assert_eq!(
            loco_clip(
                false,
                true,
                false,
                false,
                None,
                true,
                &AvatarAnimationGraph::dummy()
            ),
            MotionClip::Idle
        );
    }

    #[test]
    fn rest_hotkey_uses_the_rest_clip() {
        let mut graph = AvatarAnimationGraph::dummy();
        graph.present[MotionClip::Rest as usize] = true;
        assert_eq!(
            loco_clip(false, false, false, true, None, true, &graph),
            MotionClip::Rest
        );
        assert_eq!(
            loco_clip(true, false, false, true, Some(MoveDir::Forward), true, &graph),
            MotionClip::Walk
        );
        assert!(is_looping(MotionClip::Rest));
        assert!((clip_playback_speed(MotionClip::Rest) - 0.2).abs() < 0.001);
        assert!((clip_playback_speed(MotionClip::Idle) - 1.0).abs() < 0.001);
        assert!((clip_playback_speed(MotionClip::IdleToRest) - 1.0).abs() < 0.001);
    }

    #[test]
    fn run_gait_plays_start_and_stop_only_on_edges() {
        assert_eq!(bridge_between(MotionClip::Idle, MotionClip::Run), None);
        assert_eq!(bridge_between(MotionClip::WalkToRun, MotionClip::Run), None);
        assert_eq!(bridge_between(MotionClip::Run, MotionClip::Run), None);
        assert_eq!(
            bridge_between(MotionClip::Run, MotionClip::Idle),
            Some(MotionClip::RunToWalk)
        );
        assert!(is_looping(MotionClip::Run));
        assert!(!is_looping(MotionClip::WalkToRun));
        assert!(run_gait_bridge_matches(MotionClip::WalkToRun, MotionClip::Run));
        assert!(!run_gait_bridge_matches(
            MotionClip::WalkToRun,
            MotionClip::Walk
        ));
        assert!(run_gait_bridge_matches(
            MotionClip::RunToWalk,
            MotionClip::Idle
        ));
        assert_eq!(bridge_between(MotionClip::Idle, MotionClip::Walk), None);
        assert_eq!(
            bridge_between(MotionClip::Walk, MotionClip::Idle),
            Some(MotionClip::WalkToIdle)
        );
        assert_eq!(
            bridge_between(MotionClip::Jump, MotionClip::Run),
            Some(MotionClip::JumpToRun)
        );
        assert_eq!(
            bridge_between(MotionClip::Idle, MotionClip::Rest),
            Some(MotionClip::IdleToRest)
        );
        assert_eq!(
            bridge_between(MotionClip::Rest, MotionClip::Idle),
            Some(MotionClip::RestToIdle)
        );
    }

    #[test]
    fn idle_jump_does_not_queue_a_walk() {
        let mut graph = AvatarAnimationGraph::dummy();
        graph.present[MotionClip::Jump as usize] = true;
        graph.present[MotionClip::WalkToJump as usize] = true;
        graph.present[MotionClip::JumpToWalk as usize] = true;
        assert_eq!(
            jump_sequence(MotionClip::Idle, &graph),
            vec![MotionClip::Jump]
        );
        graph.present[MotionClip::WalkToJump as usize] = true;
        assert_eq!(
            jump_sequence(MotionClip::Walk, &graph),
            vec![MotionClip::Jump]
        );
        graph.present[MotionClip::WalkJump as usize] = true;
        assert_eq!(
            jump_sequence(MotionClip::Walk, &graph),
            vec![MotionClip::Jump]
        );
        graph.present[MotionClip::RunToJump as usize] = true;
        graph.present[MotionClip::JumpToRun as usize] = true;
        assert_eq!(
            jump_sequence(MotionClip::Run, &graph),
            vec![MotionClip::Jump]
        );
        assert_eq!(
            bridge_between(MotionClip::Jump, MotionClip::Run),
            Some(MotionClip::JumpToRun)
        );
    }

    #[test]
    fn walking_punch_uses_the_walk_punch_clip() {
        let mut graph = AvatarAnimationGraph::dummy();
        graph.present[MotionClip::WalkPunch as usize] = true;
        assert_eq!(
            attack_clip_for(JobId::TimeMage, true, &graph),
            MotionClip::WalkPunch
        );
        assert_eq!(
            attack_clip_for(JobId::TimeMage, false, &graph),
            MotionClip::Punch
        );
        graph.present[MotionClip::WalkSlash1H as usize] = true;
        assert_eq!(
            attack_clip_for(JobId::Knight, true, &graph),
            MotionClip::WalkSlash1H
        );
        assert_eq!(
            attack_clip_for(JobId::Knight, false, &graph),
            MotionClip::Slash1H
        );
        graph.present[MotionClip::WalkSlash2H as usize] = true;
        assert_eq!(
            attack_clip_for(JobId::Fighter, true, &graph),
            MotionClip::WalkSlash2H
        );
        assert_eq!(
            attack_clip_for(JobId::Fighter, false, &graph),
            MotionClip::Slash2H
        );
    }

    #[test]
    fn wasd_picks_backpedal_and_strafe_clips() {
        let mut graph = AvatarAnimationGraph::dummy();
        graph.present[MotionClip::WalkBack as usize] = true;
        graph.present[MotionClip::StrafeLeft as usize] = true;
        graph.present[MotionClip::StrafeRight as usize] = true;
        graph.present[MotionClip::Run as usize] = true;
        assert_eq!(
            loco_clip(true, false, false, false, Some(MoveDir::Back), false, &graph),
            MotionClip::Walk
        );
        assert_eq!(
            loco_clip(true, false, false, false, Some(MoveDir::Left), false, &graph),
            MotionClip::Walk
        );
        assert_eq!(
            loco_clip(true, true, false, false, Some(MoveDir::Back), false, &graph),
            MotionClip::Run
        );
        assert_eq!(
            loco_clip(true, true, false, false, Some(MoveDir::Forward), false, &graph),
            MotionClip::Run
        );
        assert_eq!(
            loco_clip(true, false, false, false, Some(MoveDir::Back), true, &graph),
            MotionClip::Walk
        );
        assert_eq!(
            loco_clip(true, false, false, false, Some(MoveDir::Left), true, &graph),
            MotionClip::Walk
        );
        assert_eq!(
            loco_clip(true, false, false, false, Some(MoveDir::Right), true, &graph),
            MotionClip::Walk
        );
        assert_eq!(
            loco_clip(true, true, false, false, Some(MoveDir::Left), true, &graph),
            MotionClip::Run
        );
        assert_eq!(bridge_between(MotionClip::StrafeLeft, MotionClip::Walk), None);
        assert_eq!(bridge_between(MotionClip::WalkBack, MotionClip::Walk), None);
        assert!(clip_drives_stride(MotionClip::Walk));
        assert!(clip_drives_stride(MotionClip::Run));
        assert!(!clip_drives_stride(MotionClip::WalkBack));
        assert!(!clip_drives_stride(MotionClip::StrafeLeft));
        assert!(!clip_drives_stride(MotionClip::Idle));
        assert!(unarmed_fist(MotionClip::Punch));
        assert!(unarmed_fist(MotionClip::WalkPunch));
        assert!(!unarmed_fist(MotionClip::Slash1H));
        assert!(!unarmed_fist(MotionClip::Idle));
        assert!(keeps_unarmed_combo(MotionClip::Punch, true));
        assert!(keeps_unarmed_combo(MotionClip::WalkPunch, true));
        assert!(!keeps_unarmed_combo(MotionClip::Punch, false));
        assert!(!keeps_unarmed_combo(MotionClip::Slash1H, true));
        assert!(jump_hop_clip(MotionClip::Jump));
        assert!(!jump_hop_clip(MotionClip::Idle));
        assert!((jump_hop(0.0) - 0.0).abs() < 0.001);
        assert!((jump_hop(1.0) - 0.0).abs() < 0.001);
        assert!(
            jump_hop(0.5) > 0.7,
            "one-foot jump must leave the ground, got {}",
            jump_hop(0.5)
        );
        let rest = Quat::IDENTITY;
        let thigh = AvatarJoint {
            root: Entity::PLACEHOLDER,
            rest,
            axis: Vec3::NEG_Y,
            kind: JointKind::UpperLeg,
            sign: 1.0,
            curl: 0.,
            oppose: 0.,
            digit: 0,
            chain: 0,
        };
        let shin = AvatarJoint {
            root: Entity::PLACEHOLDER,
            rest,
            axis: Vec3::NEG_Y,
            kind: JointKind::LowerLeg,
            sign: 1.0,
            curl: 0.,
            oppose: 0.,
            digit: 0,
            chain: 0,
        };
        assert!(rest.angle_between(jump_tuck_leg(&thigh, 0.5)) > 0.8);
        assert!(
            rest.angle_between(jump_tuck_leg(&shin, 0.5))
                > rest.angle_between(jump_tuck_leg(&thigh, 0.5)),
            "the free knee must tuck harder than the thigh"
        );
        assert!(rest.angle_between(jump_tuck_leg(&shin, 0.0)) < 0.05);
        let thigh_dir = jump_tuck_leg(&thigh, 0.5) * Vec3::NEG_Y;
        let shin_dir = jump_tuck_leg(&shin, 0.5) * Vec3::NEG_Y;
        assert!(
            thigh_dir.z > 0.3,
            "free thigh must lift in front of the body, got {thigh_dir:?}"
        );
        assert!(
            shin_dir.z * thigh_dir.z < 0.0,
            "shin must fold at the knee, not past it, thigh {thigh_dir:?} shin {shin_dir:?}"
        );
        let rest_y = roll_root(0.0).0.y;
        let takeoff_y = roll_root(0.18).0.y;
        let mid_y = roll_root(0.5).0.y;
        assert!((rest_y + 1.65).abs() < 0.001);
        assert!((roll_root(1.0).0.y + 1.65).abs() < 0.001);
        assert!(
            takeoff_y > rest_y + 0.15 && takeoff_y < rest_y + 0.55,
            "dive roll hops, it does not vault, got {takeoff_y}"
        );
        assert!(
            mid_y < takeoff_y && mid_y < rest_y + 0.25,
            "after the hop the tuck stays near the ground, got {mid_y}"
        );
        assert!(clip_owns_limbs(MotionClip::Punch, false));
        assert!(clip_owns_limbs(MotionClip::Walk, true));
        assert!(!clip_owns_limbs(MotionClip::Idle, true));
        assert!(!clip_owns_limbs(MotionClip::StrafeLeft, true));
        let rest = Quat::IDENTITY;
        let finger = AvatarJoint {
            root: Entity::PLACEHOLDER,
            rest,
            axis: Vec3::Z,
            kind: JointKind::Finger,
            sign: 1.0,
            curl: 0.30,
            oppose: 0.0,
            digit: 1,
            chain: 1,
        };
        assert!(
            rest.angle_between(pose_finger(&finger, true))
                > rest.angle_between(pose_finger(&finger, false)) + 1.0,
            "unarmed fist must curl past the relaxed hand"
        );
        assert!(fist_angles(1, 1).0 > 1.7, "PIP must lock the fist shut");
        assert_eq!(finger_layout("leftIndexDistal"), (1, 2));
        assert_eq!(finger_layout("leftThumbMetacarpal"), (0, 0));
        assert_eq!(finger_layout("leftThumbProximal"), (0, 1));
        assert_eq!(
            bridge_between(MotionClip::Idle, MotionClip::Crouch),
            Some(MotionClip::IdleToCrouch)
        );
        assert_eq!(
            bridge_between(MotionClip::Crouch, MotionClip::Idle),
            Some(MotionClip::CrouchToIdle)
        );
        assert_eq!(
            bridge_between(MotionClip::Idle, MotionClip::WalkBack),
            Some(MotionClip::IdleToWalkBack)
        );
        assert_eq!(
            bridge_between(MotionClip::WalkBack, MotionClip::Idle),
            Some(MotionClip::WalkBackToIdle)
        );
        assert_eq!(
            bridge_between(MotionClip::Idle, MotionClip::StrafeLeft),
            Some(MotionClip::IdleToStrafeLeft)
        );
        assert_eq!(
            bridge_between(MotionClip::StrafeLeft, MotionClip::Idle),
            Some(MotionClip::StrafeLeftToIdle)
        );
        assert_eq!(
            bridge_between(MotionClip::Idle, MotionClip::StrafeRight),
            Some(MotionClip::IdleToStrafeRight)
        );
        assert_eq!(
            bridge_between(MotionClip::Crouch, MotionClip::CrouchWalk),
            Some(MotionClip::CrouchToCrouchWalk)
        );
        assert_eq!(
            bridge_between(MotionClip::IdleToCrouch, MotionClip::CrouchWalk),
            Some(MotionClip::CrouchToCrouchWalk)
        );
        assert_eq!(
            bridge_between(MotionClip::CrouchWalk, MotionClip::Crouch),
            Some(MotionClip::CrouchWalkToCrouch)
        );
        assert_eq!(
            bridge_between(MotionClip::CrouchWalk, MotionClip::Idle),
            Some(MotionClip::CrouchWalkToCrouch)
        );
        assert_eq!(
            bridge_between(MotionClip::CrouchWalkToCrouch, MotionClip::Idle),
            Some(MotionClip::CrouchToIdle)
        );
    }

    #[test]
    fn authored_motion_clips_are_on_disk() {
        for rel in MOTION_ASSETS.iter().filter(|rel| !rel.is_empty()) {
            assert!(
                motion_asset_path(rel).is_file(),
                "missing authored clip assets/{rel}"
            );
        }
    }

    #[test]
    fn authored_walk_clip_targets_the_exported_legs() {
        assert!(authored_walk_on_disk(), "assets/{AUTHORED_WALK} missing");
        let glb = std::fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("assets")
                .join(AUTHORED_WALK),
        )
        .unwrap();
        let len = u32::from_le_bytes(glb[12..16].try_into().unwrap()) as usize;
        let doc: serde_json::Value = serde_json::from_slice(&glb[20..20 + len]).unwrap();
        let nodes = doc["nodes"].as_array().unwrap();
        let names: Vec<&str> = nodes
            .iter()
            .filter_map(|n| n["name"].as_str())
            .collect();
        for bone in [
            "hips",
            "leftUpperLeg",
            "leftLowerLeg",
            "leftFoot",
            "rightUpperLeg",
            "rightLowerLeg",
            "rightFoot",
            "leftUpperArm",
            "rightUpperArm",
        ] {
            assert!(names.contains(&bone), "retargeted clip missing {bone}");
        }
        let channels = doc["animations"][0]["channels"].as_array().unwrap();
        assert!(
            channels.len() > 20,
            "authored walk should drive the full retargeted body, got {}",
            channels.len()
        );
    }

    #[test]
    fn third_person_authored_walk_does_not_overwrite_limbs() {
        let mut app = App::new();
        app.add_plugins(TransformPlugin)
            .insert_resource(Time::<()>::default())
            .init_resource::<CameraView>()
            .init_resource::<SwingPose>()
            .add_systems(
                PostUpdate,
                pose_humanoid_bones.before(bevy::transform::TransformSystems::Propagate),
            );
        app.world_mut().resource_mut::<CameraView>().mode = CameraMode::ThirdPerson;
        let world = app.world_mut();
        let actor = world.spawn(Transform::default()).id();
        let root = world
            .spawn((
                Transform::default(),
                AvatarMotion {
                    authored_walk: true,
                    ..default()
                },
                AvatarAnimationGraph::dummy(),
                ChildOf(actor),
            ))
            .id();
        let rest = Quat::from_rotation_x(0.4);
        let leg = world
            .spawn((
                Name::new("leftUpperLeg"),
                Transform {
                    rotation: rest,
                    ..default()
                },
                AvatarJoint {
                    root,
                    rest,
                    axis: Vec3::NEG_Y,
                    kind: JointKind::UpperLeg,
                    sign: 1.0,
                    curl: 0.,
                    oppose: 0.,
                    digit: 0,
                    chain: 0,
                },
                ChildOf(root),
            ))
            .id();
        for _ in 0..8 {
            app.world_mut()
                .get_mut::<Transform>(actor)
                .unwrap()
                .translation
                .z -= 0.08;
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(Duration::from_secs_f32(1. / 60.));
            app.update();
        }
        let posed = app.world().get::<Transform>(leg).unwrap().rotation;
        assert!(
            rest.angle_between(posed) < 0.001,
            "authored 3P walk must leave the clip on the armature, got {posed:?} vs rest {rest:?}"
        );
    }

    #[test]
    fn third_person_unarmed_punch_closes_the_fingers() {
        let mut app = App::new();
        app.add_plugins(TransformPlugin)
            .insert_resource(Time::<()>::default())
            .init_resource::<CameraView>()
            .init_resource::<SwingPose>()
            .add_systems(
                PostUpdate,
                pose_humanoid_bones.before(bevy::transform::TransformSystems::Propagate),
            );
        app.world_mut().resource_mut::<CameraView>().mode = CameraMode::ThirdPerson;
        let world = app.world_mut();
        let actor = world.spawn(Transform::default()).id();
        let root = world
            .spawn((
                Transform::default(),
                PlayerAvatarMesh,
                AvatarMotion {
                    authored_walk: true,
                    fist: true,
                    ..default()
                },
                AvatarAnimationGraph::dummy(),
                ChildOf(actor),
            ))
            .id();
        let rest = Quat::IDENTITY;
        let finger = world
            .spawn((
                Name::new("leftIndexProximal"),
                Transform {
                    rotation: rest,
                    ..default()
                },
                AvatarJoint {
                    root,
                    rest,
                    axis: Vec3::Z,
                    kind: JointKind::Finger,
                    sign: 1.0,
                    curl: 0.30,
                    oppose: 0.0,
                    digit: 1,
                    chain: 1,
                },
                ChildOf(root),
            ))
            .id();
        for _ in 0..4 {
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(Duration::from_secs_f32(1. / 60.));
            app.update();
        }
        let posed = app.world().get::<Transform>(finger).unwrap().rotation;
        assert!(
            rest.angle_between(posed) > 1.2,
            "hand-to-hand punch must close the fist, got {posed:?} vs rest {rest:?}"
        );
    }

    #[test]
    fn idle_and_rest_hold_the_head_level() {
        let mut app = App::new();
        app.add_plugins(TransformPlugin)
            .insert_resource(Time::<()>::default())
            .init_resource::<CameraView>()
            .init_resource::<SwingPose>()
            .add_systems(
                PostUpdate,
                pose_humanoid_bones.before(bevy::transform::TransformSystems::Propagate),
            );
        app.world_mut().resource_mut::<CameraView>().mode = CameraMode::ThirdPerson;
        let world = app.world_mut();
        let actor = world.spawn(Transform::default()).id();
        let root = world
            .spawn((
                Transform::default(),
                PlayerAvatarMesh,
                AvatarMotion {
                    authored_walk: true,
                    level_head: true,
                    ..default()
                },
                AvatarAnimationGraph::dummy(),
                ChildOf(actor),
            ))
            .id();
        let rest = Quat::IDENTITY;
        let tilted = Quat::from_rotation_x(0.55);
        let head = world
            .spawn((
                Name::new("head"),
                Transform {
                    rotation: tilted,
                    ..default()
                },
                AvatarJoint {
                    root,
                    rest,
                    axis: Vec3::Y,
                    kind: JointKind::Head,
                    sign: 0.0,
                    curl: 0.,
                    oppose: 0.,
                    digit: 0,
                    chain: 0,
                },
                ChildOf(root),
            ))
            .id();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(1. / 60.));
        app.update();
        let posed = app.world().get::<Transform>(head).unwrap().rotation;
        assert!(
            rest.angle_between(posed) < 0.001,
            "idle/rest must return the head to bind, got {posed:?}"
        );
    }

    #[test]
    fn unarmed_swing_throws_the_left_cross_after_the_right_jab() {
        let mut swing = SwingPose::default();
        swing.strike(1.0);
        assert!(
            swing_arm_amount(&swing, 1.12, 1.0, true).abs() > 0.05,
            "right jab must start immediately"
        );
        assert!(
            swing_arm_amount(&swing, 1.05, -1.0, true).abs() < 0.001,
            "left cross must wait for the jab"
        );
        assert!(
            swing_arm_amount(&swing, 1.32, -1.0, true).abs() > 0.05,
            "left cross must fire on the same swing clock"
        );
        assert_eq!(
            swing_arm_amount(&swing, 1.32, -1.0, false),
            0.0,
            "armed swings stay on the right hand"
        );
    }

    #[test]
    fn first_person_unarmed_punch_throws_both_arms() {
        let mut app = App::new();
        app.add_plugins(TransformPlugin)
            .insert_resource(Time::<()>::default())
            .init_resource::<CameraView>()
            .init_resource::<SwingPose>()
            .add_systems(
                PostUpdate,
                pose_humanoid_bones.before(bevy::transform::TransformSystems::Propagate),
            );
        app.world_mut().resource_mut::<CameraView>().mode = CameraMode::FirstPerson;
        let world = app.world_mut();
        let actor = world.spawn(Transform::default()).id();
        let root = world
            .spawn((
                Transform::default(),
                PlayerAvatarMesh,
                AvatarMotion {
                    fist: true,
                    ..default()
                },
                AvatarAnimationGraph::dummy(),
                ChildOf(actor),
            ))
            .id();
        let rest = Quat::IDENTITY;
        let spawn_arm = |world: &mut World, name: &str, sign: f32| {
            world
                .spawn((
                    Name::new(name.to_string()),
                    Transform {
                        rotation: rest,
                        ..default()
                    },
                    AvatarJoint {
                        root,
                        rest,
                        axis: Vec3::NEG_X * sign,
                        kind: JointKind::UpperArm,
                        sign,
                        curl: 0.,
                        oppose: 0.,
                        digit: 0,
                        chain: 0,
                    },
                    ChildOf(root),
                ))
                .id()
        };
        let left = spawn_arm(world, "leftUpperArm", 1.0);
        let right = spawn_arm(world, "rightUpperArm", -1.0);
        for _ in 0..4 {
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(Duration::from_secs_f32(1. / 60.));
            app.update();
        }
        let now = app.world().resource::<Time>().elapsed_secs();
        app.world_mut().resource_mut::<SwingPose>().strike(now);
        for _ in 0..20 {
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(Duration::from_secs_f32(1. / 60.));
            app.update();
        }
        let left_rot = app.world().get::<Transform>(left).unwrap().rotation;
        let right_rot = app.world().get::<Transform>(right).unwrap().rotation;
        assert!(
            rest.angle_between(right_rot) > 0.15,
            "right jab must leave bind, got {right_rot:?}"
        );
        assert!(
            rest.angle_between(left_rot) > 0.15,
            "left cross must leave bind, got {left_rot:?}"
        );
    }

    #[test]
    fn authored_punch_clip_keeps_the_boxing_arms() {
        let mut app = App::new();
        app.add_plugins(TransformPlugin)
            .insert_resource(Time::<()>::default())
            .init_resource::<CameraView>()
            .init_resource::<SwingPose>()
            .add_systems(
                PostUpdate,
                pose_humanoid_bones.before(bevy::transform::TransformSystems::Propagate),
            );
        app.world_mut().resource_mut::<CameraView>().mode = CameraMode::ThirdPerson;
        let world = app.world_mut();
        let actor = world.spawn(Transform::default()).id();
        let root = world
            .spawn((
                Transform::default(),
                PlayerAvatarMesh,
                AvatarMotion {
                    authored_walk: true,
                    fist: true,
                    ..default()
                },
                AvatarAnimationGraph::dummy(),
                ChildOf(actor),
            ))
            .id();
        let rest = Quat::IDENTITY;
        let tilted = Quat::from_rotation_x(0.4);
        let arm = world
            .spawn((
                Name::new("leftUpperArm"),
                Transform {
                    rotation: tilted,
                    ..default()
                },
                AvatarJoint {
                    root,
                    rest,
                    axis: Vec3::X,
                    kind: JointKind::UpperArm,
                    sign: 1.0,
                    curl: 0.,
                    oppose: 0.,
                    digit: 0,
                    chain: 0,
                },
                ChildOf(root),
            ))
            .id();
        let now = 0.2;
        app.world_mut().resource_mut::<Time>().advance_by(Duration::from_secs_f32(now));
        app.world_mut().resource_mut::<SwingPose>().strike(now);
        for _ in 0..8 {
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(Duration::from_secs_f32(1. / 60.));
            app.update();
        }
        let posed = app.world().get::<Transform>(arm).unwrap().rotation;
        assert!(
            tilted.angle_between(posed) < 0.001,
            "authored boxing must keep the clip on the arms, got {posed:?}"
        );
    }

    #[test]
    fn jump_tucks_the_free_left_leg_and_leaves_the_takeoff_leg() {
        let mut app = App::new();
        app.add_plugins(TransformPlugin)
            .insert_resource(Time::<()>::default())
            .init_resource::<CameraView>()
            .init_resource::<SwingPose>()
            .add_systems(
                PostUpdate,
                pose_humanoid_bones.before(bevy::transform::TransformSystems::Propagate),
            );
        app.world_mut().resource_mut::<CameraView>().mode = CameraMode::ThirdPerson;
        let world = app.world_mut();
        let actor = world.spawn(Transform::default()).id();
        let root = world
            .spawn((
                Transform::default(),
                PlayerAvatarMesh,
                AvatarMotion {
                    authored_walk: true,
                    jumping: true,
                    jump_phase: 0.5,
                    ..default()
                },
                AvatarAnimationGraph::dummy(),
                ChildOf(actor),
            ))
            .id();
        let rest = Quat::IDENTITY;
        let spawn_leg = |world: &mut World, name: &str, sign: f32, kind: JointKind| {
            world
                .spawn((
                    Name::new(name.to_string()),
                    Transform {
                        rotation: rest,
                        ..default()
                    },
                    AvatarJoint {
                        root,
                        rest,
                        axis: Vec3::NEG_Y,
                        kind,
                        sign,
                        curl: 0.,
                        oppose: 0.,
                        digit: 0,
                        chain: 0,
                    },
                    ChildOf(root),
                ))
                .id()
        };
        let left = spawn_leg(world, "leftLowerLeg", 1.0, JointKind::LowerLeg);
        let right = spawn_leg(world, "rightLowerLeg", -1.0, JointKind::LowerLeg);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(1. / 60.));
        app.update();
        let left_rot = app.world().get::<Transform>(left).unwrap().rotation;
        let right_rot = app.world().get::<Transform>(right).unwrap().rotation;
        assert!(
            rest.angle_between(left_rot) > 1.2,
            "free left knee must tuck, got {left_rot:?}"
        );
        assert!(
            rest.angle_between(right_rot) < 0.001,
            "right takeoff must stay on the clip, got {right_rot:?}"
        );
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MoveDir {
    Forward,
    Back,
    Left,
    Right,
}

fn dominant_move_dir(forward: f32, right: f32) -> Option<MoveDir> {
    if forward.abs() < 0.01 && right.abs() < 0.01 {
        return None;
    }
    if forward.abs() >= right.abs() {
        if forward > 0.0 {
            Some(MoveDir::Forward)
        } else {
            Some(MoveDir::Back)
        }
    } else if right < 0.0 {
        Some(MoveDir::Left)
    } else {
        Some(MoveDir::Right)
    }
}

fn move_dir_from_keys(keys: &ButtonInput<KeyCode>) -> Option<MoveDir> {
    let forward = keys.pressed(KeyCode::KeyW) as i8 - keys.pressed(KeyCode::KeyS) as i8;
    let right = keys.pressed(KeyCode::KeyD) as i8 - keys.pressed(KeyCode::KeyA) as i8;
    dominant_move_dir(forward as f32, right as f32)
}

fn walk_clip(dir: MoveDir, graph: &AvatarAnimationGraph) -> MotionClip {
    match dir {
        MoveDir::Back if graph.has(MotionClip::WalkBack) => MotionClip::WalkBack,
        MoveDir::Left if graph.has(MotionClip::StrafeLeft) => MotionClip::StrafeLeft,
        MoveDir::Right if graph.has(MotionClip::StrafeRight) => MotionClip::StrafeRight,
        _ => MotionClip::Walk,
    }
}

fn loco_clip(
    moving: bool,
    running: bool,
    crouching: bool,
    resting: bool,
    dir: Option<MoveDir>,
    face_velocity: bool,
    graph: &AvatarAnimationGraph,
) -> MotionClip {
    if resting && !moving {
        if graph.has(MotionClip::Rest) {
            return MotionClip::Rest;
        }
        return MotionClip::Idle;
    }
    let dir = dir.unwrap_or(MoveDir::Forward);
    if crouching {
        if moving && graph.has(MotionClip::CrouchWalk) {
            MotionClip::CrouchWalk
        } else if graph.has(MotionClip::Crouch) {
            MotionClip::Crouch
        } else if moving {
            walk_clip(dir, graph)
        } else {
            MotionClip::Idle
        }
    } else if moving {
        // Walk-back / strafe files are near-idle (sub-2cm foot travel). W, A, S,
        // and D all use the same walk/run cycles; third person yaws the visual
        // to the move heading so the stride still reads as locomotion.
        let _ = (dir, face_velocity);
        if running && graph.has(MotionClip::Run) {
            MotionClip::Run
        } else {
            MotionClip::Walk
        }
    } else {
        MotionClip::Idle
    }
}

fn jump_sequence(_from: MotionClip, graph: &AvatarAnimationGraph) -> Vec<MotionClip> {
    // Take off on Jump immediately. WalkJump / WalkToJump / RunToJump start with
    // a long stride prefix, and queuing them in front of Jump meant a run
    // oneshot finished, cleared the leftover Jump, and never left the ground.
    if graph.has(MotionClip::Jump) {
        vec![MotionClip::Jump]
    } else if graph.has(MotionClip::WalkJump) {
        vec![MotionClip::WalkJump]
    } else {
        vec![]
    }
}

fn is_run_gait_bridge(clip: MotionClip) -> bool {
    matches!(
        clip,
        MotionClip::WalkToRun
            | MotionClip::RunToWalk
            | MotionClip::IdleToWalk
            | MotionClip::WalkToIdle
            | MotionClip::IdleToRest
            | MotionClip::RestToIdle
            | MotionClip::IdleToCrouch
            | MotionClip::CrouchToIdle
            | MotionClip::CrouchToCrouchWalk
            | MotionClip::CrouchWalkToCrouch
            | MotionClip::IdleToWalkBack
            | MotionClip::WalkBackToIdle
            | MotionClip::IdleToStrafeLeft
            | MotionClip::StrafeLeftToIdle
            | MotionClip::IdleToStrafeRight
            | MotionClip::StrafeRightToIdle
    )
}

fn run_gait_bridge_matches(bridge: MotionClip, want: MotionClip) -> bool {
    match bridge {
        MotionClip::WalkToRun => want == MotionClip::Run,
        MotionClip::RunToWalk => {
            want == MotionClip::Walk || want == MotionClip::Idle || want == MotionClip::WalkBack
        }
        MotionClip::IdleToWalk => want == MotionClip::Walk,
        MotionClip::WalkToIdle => want == MotionClip::Idle,
        MotionClip::IdleToRest => want == MotionClip::Rest,
        MotionClip::RestToIdle => {
            want == MotionClip::Idle || want == MotionClip::Walk || want == MotionClip::WalkBack
        }
        MotionClip::IdleToCrouch => want == MotionClip::Crouch || want == MotionClip::CrouchWalk,
        MotionClip::CrouchToIdle => want == MotionClip::Idle || want == MotionClip::Walk,
        MotionClip::CrouchToCrouchWalk => want == MotionClip::CrouchWalk,
        MotionClip::CrouchWalkToCrouch => want == MotionClip::Crouch || want == MotionClip::Idle,
        MotionClip::IdleToWalkBack => want == MotionClip::WalkBack || want == MotionClip::Walk,
        MotionClip::WalkBackToIdle => want == MotionClip::Idle || want == MotionClip::Walk,
        MotionClip::IdleToStrafeLeft => want == MotionClip::StrafeLeft || want == MotionClip::Walk,
        MotionClip::StrafeLeftToIdle => want == MotionClip::Idle || want == MotionClip::Walk,
        MotionClip::IdleToStrafeRight => want == MotionClip::StrafeRight || want == MotionClip::Walk,
        MotionClip::StrafeRightToIdle => want == MotionClip::Idle || want == MotionClip::Walk,
        _ => true,
    }
}

fn bridge_between(from: MotionClip, to: MotionClip) -> Option<MotionClip> {
    match (from, to) {
        // Idle→walk and walk→run connects are ~1s oneshots. Play the looping
        // gait immediately; do not regenerate or retune those clips.
        (
            MotionClip::Run | MotionClip::WalkToRun,
            MotionClip::Walk | MotionClip::Idle | MotionClip::CrouchWalk,
        ) => Some(MotionClip::RunToWalk),
        (
            MotionClip::Idle | MotionClip::WalkToIdle | MotionClip::WalkBackToIdle,
            MotionClip::WalkBack,
        ) => Some(MotionClip::IdleToWalkBack),
        (
            MotionClip::Idle | MotionClip::WalkToIdle | MotionClip::StrafeLeftToIdle,
            MotionClip::StrafeLeft,
        ) => Some(MotionClip::IdleToStrafeLeft),
        (
            MotionClip::Idle | MotionClip::WalkToIdle | MotionClip::StrafeRightToIdle,
            MotionClip::StrafeRight,
        ) => Some(MotionClip::IdleToStrafeRight),
        (
            MotionClip::Walk | MotionClip::IdleToWalk | MotionClip::RunToWalk,
            MotionClip::Idle,
        ) => Some(MotionClip::WalkToIdle),
        (MotionClip::WalkBack | MotionClip::IdleToWalkBack, MotionClip::Idle) => {
            Some(MotionClip::WalkBackToIdle)
        }
        (MotionClip::StrafeLeft | MotionClip::IdleToStrafeLeft, MotionClip::Idle) => {
            Some(MotionClip::StrafeLeftToIdle)
        }
        (MotionClip::StrafeRight | MotionClip::IdleToStrafeRight, MotionClip::Idle) => {
            Some(MotionClip::StrafeRightToIdle)
        }
        (MotionClip::Idle | MotionClip::WalkToIdle, MotionClip::Rest) => {
            Some(MotionClip::IdleToRest)
        }
        (
            MotionClip::Idle | MotionClip::WalkToIdle | MotionClip::Walk,
            MotionClip::Crouch | MotionClip::CrouchWalk,
        ) => Some(MotionClip::IdleToCrouch),
        (MotionClip::Crouch | MotionClip::IdleToCrouch, MotionClip::CrouchWalk) => {
            Some(MotionClip::CrouchToCrouchWalk)
        }
        (
            MotionClip::CrouchWalk | MotionClip::CrouchToCrouchWalk,
            MotionClip::Crouch | MotionClip::Idle,
        ) => Some(MotionClip::CrouchWalkToCrouch),
        (
            MotionClip::Crouch | MotionClip::IdleToCrouch | MotionClip::CrouchWalkToCrouch,
            MotionClip::Idle,
        ) => Some(MotionClip::CrouchToIdle),
        (
            MotionClip::Rest | MotionClip::IdleToRest,
            MotionClip::Idle | MotionClip::Walk,
        ) => Some(MotionClip::RestToIdle),
        (
            MotionClip::Jump
            | MotionClip::WalkToJump
            | MotionClip::RunToJump
            | MotionClip::JumpToWalk
            | MotionClip::WalkJump,
            MotionClip::Run,
        ) => Some(MotionClip::JumpToRun),
        (
            MotionClip::Jump | MotionClip::WalkToJump | MotionClip::RunToJump | MotionClip::WalkJump,
            MotionClip::Walk | MotionClip::Idle,
        ) => Some(MotionClip::JumpToWalk),
        _ => None,
    }
}

fn wants_move(keys: &ButtonInput<KeyCode>) -> bool {
    keys.pressed(KeyCode::KeyW)
        || keys.pressed(KeyCode::KeyA)
        || keys.pressed(KeyCode::KeyS)
        || keys.pressed(KeyCode::KeyD)
}

fn wants_sprint(keys: &ButtonInput<KeyCode>) -> bool {
    keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight)
}

/// Drive idle/walk/run and authored one-shots from observed displacement plus
/// local input. Presentation only; it cannot grant speed/combat outcomes.
pub fn update_avatar_animation(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    swing: Res<SwingPose>,
    view: Res<CameraView>,
    parents: Query<&ChildOf>,
    jobs: Query<&crate::rpg::Progression>,
    fighters: Query<&crate::rpg::Fighter>,
    mut roots: Query<(
        &GlobalTransform,
        &AvatarAnimationGraph,
        Option<&PlayerAvatarMesh>,
        &mut AvatarMotion,
    )>,
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
        let Ok((transform, graph, local, motion)) = roots.get_mut(state.root) else {
            continue;
        };
        let mut sync = StrideSync {
            motion,
            clip: state.clip,
            moving: state.moving,
        };
        let now = transform.translation();
        if !state.initialized {
            state.initialized = true;
            state.last_position = now;
            continue;
        }
        let delta = now - state.last_position;
        let speed = delta.length() / dt;
        state.last_position = now;
        // Local idle follows WASD so zone jitter / sprint-key leftovers cannot
        // keep a run→walk cycle playing while standing still. Remotes still
        // use displacement hysteresis.
        state.moving = if local.is_some() {
            wants_move(&keys)
        } else if state.moving {
            speed > 0.18
        } else {
            speed > 0.32
        };
        sync.moving = state.moving;
        let dir = if local.is_some() {
            move_dir_from_keys(&keys)
        } else {
            let rot = transform.rotation();
            let fwd = {
                let v = rot * Vec3::NEG_Z;
                Vec3::new(v.x, 0.0, v.z).normalize_or_zero()
            };
            let right = {
                let v = rot * Vec3::X;
                Vec3::new(v.x, 0.0, v.z).normalize_or_zero()
            };
            let planar = Vec3::new(delta.x, 0.0, delta.z);
            dominant_move_dir(planar.dot(fwd), planar.dot(right))
        };
        let running = state.moving
            && if local.is_some() {
                wants_sprint(&keys)
            } else {
                speed > 5.0
            };
        let crouching = local.is_some() && keys.pressed(KeyCode::KeyC);
        let parent = parents.get(state.root).ok().map(|p| p.parent());
        let resting = parent
            .and_then(|p| fighters.get(p).ok())
            .map(|f| f.resting)
            .unwrap_or(false);
        let face_velocity = view.mode != CameraMode::FirstPerson;
        let want = loco_clip(
            state.moving,
            running,
            crouching,
            resting,
            dir,
            face_velocity,
            graph,
        );
        let job = parent
            .and_then(|p| jobs.get(p).ok())
            .map(|p| p.job)
            .unwrap_or(JobId::TimeMage);

        if local.is_some() {
            if let Some(at) = swing.started_at() {
                if at != state.last_swing {
                    state.last_swing = at;
                    if !keeps_unarmed_combo(state.clip, state.oneshot) {
                        let moving = state.moving;
                        state.queue.clear();
                        state.queue.push(attack_clip_for(job, moving, graph));
                        state.oneshot = false;
                    }
                }
            }
            if keys.just_pressed(KeyCode::ShiftLeft) && graph.has(MotionClip::Roll) {
                state.queue.clear();
                state.queue.push(MotionClip::Roll);
                state.oneshot = false;
            }
            if keys.just_pressed(KeyCode::Space)
                && (graph.has(MotionClip::Jump) || graph.has(MotionClip::WalkJump))
            {
                state.queue = jump_sequence(state.clip, graph);
                state.oneshot = false;
            }
        }

        if !state.queue.is_empty() && !state.oneshot {
            let next = state.queue.remove(0);
            play_motion(&mut player, &mut transitions, graph, next, 90);
            state.clip = next;
            sync.clip = next;
            state.oneshot = !is_looping(next);
            if state.oneshot {
                continue;
            }
        }
        if state.oneshot {
            if is_run_gait_bridge(state.clip) && !run_gait_bridge_matches(state.clip, want) {
                state.oneshot = false;
                state.queue.clear();
            } else if !player.all_finished() {
                continue;
            } else {
                state.oneshot = false;
                if state.queue.last() != Some(&want) {
                    state.queue.clear();
                } else {
                    let next = state.queue.remove(0);
                    play_motion(&mut player, &mut transitions, graph, next, 80);
                    state.clip = next;
                    sync.clip = next;
                    state.oneshot = !is_looping(next);
                    if state.oneshot {
                        continue;
                    }
                }
            }
        }
        if want == state.clip {
            continue;
        }
        if let Some(bridge) = bridge_between(state.clip, want) {
            if graph.has(bridge) {
                play_motion(&mut player, &mut transitions, graph, bridge, 80);
                state.clip = bridge;
                sync.clip = bridge;
                state.oneshot = true;
                state.queue.clear();
                if let Some(second) = bridge_between(bridge, want) {
                    if graph.has(second) && second != bridge {
                        state.queue.push(second);
                    }
                }
                state.queue.push(want);
                continue;
            }
        }
        let blend = if is_looping(want) { 80 } else { 170 };
        play_motion(&mut player, &mut transitions, graph, want, blend);
        state.clip = want;
        sync.clip = want;
        state.oneshot = !is_looping(want);
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
                AvatarAnimationGraph::dummy(),
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
        let finger = bone(&["indexproximalr", "indexfinger1r"]);
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
        let finger_joint = app.world().get::<AvatarJoint>(finger).unwrap();
        let finger_pose = app.world().get::<Transform>(finger).unwrap().rotation;
        assert!(
            finger_joint.rest.angle_between(finger_pose) > 0.2,
            "{body} first-person fingers must curl off bind: rest {:?} pose {:?}",
            finger_joint.rest,
            finger_pose
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
