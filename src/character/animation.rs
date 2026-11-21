//! Bevy 0.20 glTF animation graph for procedural Studio Idle/Walk clips.
//! The rendered character follows the authoritative movement Transform.
//! Animation is presentation only; it cannot grant speed/combat outcomes.

use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::time::Duration;
#[cfg(not(test))]
use std::fs;
#[cfg(not(test))]
use std::time::{SystemTime, UNIX_EPOCH};

use serde::Deserialize;

use super::{EquipMode, EquipState, PlayerAvatarMesh};
use crate::camera_rig::{CameraMode, CameraView};
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
    Slash1HReturn,
    Slash1HSweep,
    SlashDw,
    SlashDwLeft,
    SlashDwReturn,
    SlashDwLeftReturn,
}

const MOTION_CLIP_COUNT: usize = 44;

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
    "", // WalkSlash1H archived; 1H uses Slash1H / Return / Sweep in place.
    "character/motion/crouch_to_crouch_walk.glb",
    "character/motion/crouch_walk_to_crouch.glb",
    "character/motion/idle_to_walk_back.glb",
    "character/motion/walk_back_to_idle.glb",
    "character/motion/idle_to_strafe_left.glb",
    "character/motion/strafe_left_to_idle.glb",
    "character/motion/idle_to_strafe_right.glb",
    "character/motion/strafe_right_to_idle.glb",
    "", // WalkSlash2H archived; moving 2H uses standing Slash2H.
    "character/motion/slash_1h_return.glb",
    "character/motion/slash_1h_sweep.glb",
    "character/motion/slash_dw.glb",
    "character/motion/slash_dw_left.glb",
    "character/motion/slash_dw_return.glb",
    "character/motion/slash_dw_left_return.glb",
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
    Shoulder,
    UpperArm,
    LowerArm,
    Hand,
    Finger,
    UpperLeg,
    LowerLeg,
    Hips,
    Head,
    Neck,
}

/// Rest/bind rotation captured *before* Bevy applies the first animation.
/// The model is never replaced by a primitive viewmodel in first person.
#[derive(Component, Clone, Copy)]
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
    /// Close the right hand around a one-hand grip. Presentation only.
    grip_right: bool,
    /// Close the left hand around a dual-wield offhand. Presentation only.
    grip_left: bool,
    /// Idle/rest/crouch: hold neck and head at bind so the face is level.
    level_head: bool,
    /// One-foot jump clip is playing. Presentation hop only.
    jumping: bool,
    jump_phase: f32,
    /// Rest clip is playing. Stow steel and plant a support hand.
    resting: bool,
    /// 0 = carry, 1 = planted. Eases across the 1.6 s sit / stand clips.
    rest_support: f32,
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

fn attack_clip(held: super::HeldWeapon) -> MotionClip {
    match held {
        super::HeldWeapon::Unarmed => MotionClip::Punch,
        super::HeldWeapon::OneHand => MotionClip::Slash1H,
        super::HeldWeapon::DualWield => MotionClip::SlashDw,
        super::HeldWeapon::GreatWeapon | super::HeldWeapon::PoleArm => MotionClip::Slash2H,
    }
}

fn attack_clip_for(held: super::HeldWeapon, moving: bool, graph: &AvatarAnimationGraph) -> MotionClip {
    attack_clip_arc(held, moving, graph, SlashArc::Diagonal, 0)
}

fn attack_clip_arc(
    held: super::HeldWeapon,
    moving: bool,
    graph: &AvatarAnimationGraph,
    arc: SlashArc,
    combo: u8,
) -> MotionClip {
    if held == super::HeldWeapon::DualWield {
        return if combo % 2 == 1 && graph.has(MotionClip::SlashDwLeft) {
            MotionClip::SlashDwLeft
        } else {
            MotionClip::SlashDw
        };
    }
    if held.is_two_handed() {
        let standing = attack_clip(held);
        if moving && graph.has(MotionClip::WalkSlash2H) {
            return MotionClip::WalkSlash2H;
        }
        return standing;
    }
    if held == super::HeldWeapon::OneHand {
        let standing = match arc {
            SlashArc::Return if graph.has(MotionClip::Slash1HReturn) => MotionClip::Slash1HReturn,
            SlashArc::Sweep if graph.has(MotionClip::Slash1HSweep) => MotionClip::Slash1HSweep,
            _ => MotionClip::Slash1H,
        };
        // Standing Tide 1H oneshots play in place; root motion is the controller.
        return standing;
    }
    let standing = attack_clip(held);
    if !moving {
        return standing;
    }
    match standing {
        MotionClip::Punch if graph.has(MotionClip::WalkPunch) => MotionClip::WalkPunch,
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
        self.motion.grip_right = one_hand_grip(self.clip) || dual_right_grip(self.clip);
        self.motion.grip_left = dual_left_grip(self.clip);
        self.motion.level_head = levels_head(self.clip);
        self.motion.jumping = jump_hop_clip(self.clip);
        self.motion.resting = rest_pose(self.clip);
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

fn one_hand_grip(clip: MotionClip) -> bool {
    matches!(
        clip,
        MotionClip::Slash1H | MotionClip::Slash1HReturn | MotionClip::Slash1HSweep
    )
}

fn dual_right_grip(clip: MotionClip) -> bool {
    matches!(clip, MotionClip::SlashDw | MotionClip::SlashDwReturn)
}

fn dual_left_grip(clip: MotionClip) -> bool {
    matches!(clip, MotionClip::SlashDwLeft | MotionClip::SlashDwLeftReturn)
}


/// Keep the boxing combo playing; retriggers must not rewind to the first jab.
fn keeps_unarmed_combo(clip: MotionClip, oneshot: bool) -> bool {
    oneshot && unarmed_fist(clip)
}

fn levels_head(clip: MotionClip) -> bool {
    matches!(
        clip,
        MotionClip::Idle
            | MotionClip::Rest
            | MotionClip::Crouch
            | MotionClip::Slash1H
            | MotionClip::Slash1HReturn
            | MotionClip::Slash1HSweep
            | MotionClip::SlashDw
            | MotionClip::SlashDwLeft
            | MotionClip::SlashDwReturn
            | MotionClip::SlashDwLeftReturn
            | MotionClip::Slash2H
            | MotionClip::WalkSlash2H
    )
}

fn jump_hop_clip(clip: MotionClip) -> bool {
    matches!(clip, MotionClip::Jump | MotionClip::WalkJump)
}

fn rest_pose(clip: MotionClip) -> bool {
    matches!(clip, MotionClip::Rest | MotionClip::IdleToRest)
}

/// Matches idle_to_rest / rest_to_idle (1.63 s). Arms follow the sit, not the 80 ms graph blend.
const REST_SUPPORT_SECS: f32 = 1.6;

fn tick_rest_support(current: f32, sitting: bool, dt: f32) -> f32 {
    let want = if sitting { 1. } else { 0. };
    let step = (dt / REST_SUPPORT_SECS).max(0.);
    if want > current {
        (current + step).min(1.)
    } else {
        (current - step).max(0.)
    }
}

fn rest_support_blend(support: f32) -> f32 {
    let t = support.clamp(0., 1.);
    t * t * (3. - 2. * t)
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

#[derive(Clone, Copy)]
enum FingerPose {
    Relaxed,
    Fist,
    Grip,
}

fn pose_finger(joint: &AvatarJoint, pose: FingerPose) -> Quat {
    let (curl_n, oppose_n) = match pose {
        FingerPose::Relaxed => (joint.curl, joint.oppose),
        FingerPose::Fist => fist_angles(joint.digit, joint.chain),
        FingerPose::Grip => grip_angles(joint.digit, joint.chain),
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

/// Wrap a short-sword hilt (~3cm). Boxing PIP (1.82 rad) folds the finger
/// through the grip and reads as hyperextension from first person.
fn grip_angles(digit: u8, chain: u8) -> (f32, f32) {
    if digit == 0 {
        return match chain {
            0 => (0.62, 0.52),
            1 => (0.70, 0.10),
            _ => (0.48, 0.0),
        };
    }
    let wrap = (digit as f32 - 1.0) * 0.04;
    match chain {
        0 => (0.78 + wrap, 0.0),
        1 => (0.92 + wrap, 0.0),
        _ => (0.50 + wrap, 0.0),
    }
}

fn clip_playback_speed(clip: MotionClip) -> f32 {
    if clip == MotionClip::Rest {
        0.2
    } else {
        1.0
    }
}

/// Instant starts (idle→walk/strafe, jump takeoff, chop chain). MotionBricks
/// connect is ≥0.6 s and would lag the 0.48 s combat clock.
const RESPONSIVE_BLEND_MS: u64 = 120;
/// Into a settling MotionBricks clip (walk→idle, rest, crouch).
const SETTLING_BLEND_MS: u64 = 80;
/// Chop-to-chop and other oneshot queues. 1H Tide cuts start at wind and
/// end at follow-through; Bevy blends 80 ms between them.
const ONESHOT_BLEND_MS: u64 = 80;

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
    } else if key == "hips" || key.ends_with("hips") || key.contains("pelvis") {
        JointKind::Hips
    } else if key.contains("lowerarm") || key.contains("forearm") || key.ends_with("elbow") {
        JointKind::LowerArm
    } else if key.contains("upperarm") || (key.ends_with("leftarm") || key.ends_with("rightarm")) {
        JointKind::UpperArm
    } else if key.contains("shoulder") {
        JointKind::Shoulder
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
        JointKind::Shoulder => "shoulder",
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
        JointKind::Hips => "hips",
        JointKind::Head => "head",
        JointKind::Neck => "neck",
    };
    if matches!(part, JointKind::Head | JointKind::Neck | JointKind::Hips) {
        return Some((part, 0.0));
    }
    let is_left = key.contains("left")
        || key.contains(&format!("bipl{part_key}"))
        || key.ends_with(&format!("{part_key}l"))
        || (part == JointKind::UpperLeg && key.contains("biplthigh"))
        // Mixamo `index.proximal.L` → `indexproximall`. Do not treat the
        // trailing L in VRM `rightIndexProximal` as a left-side marker.
        || (part == JointKind::Finger && key.ends_with('l') && !key.contains("right"));
    let is_right = key.contains("right")
        || key.contains(&format!("bipr{part_key}"))
        || key.ends_with(&format!("{part_key}r"))
        || (part == JointKind::UpperLeg && key.contains("biprthigh"))
        || (part == JointKind::Finger && key.ends_with('r') && !key.contains("left"));
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
                JointKind::Shoulder => &["upperarm", "arm"],
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
                JointKind::Hips => &["spine", "upperleg", "thigh"],
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
                    JointKind::Hips => Vec3::Y,
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
pub(crate) struct HeldWeaponMesh;

#[derive(Component)]
pub(crate) struct HeldDarkmodSword;

#[derive(Component, Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum HeldMeshKind {
    ShortSword,
    LongSword,
    Staff,
}

const DARKMOD_SHORTSWORD: &str = "darkmod/weapons/shortsword.glb";
const DARKMOD_LONGSWORD: &str = "darkmod/weapons/longsword01.glb";
const DARKMOD_STAFF: &str = "darkmod/weapons/quarterstaff.glb";
/// Dark Mod inches → metres. 35-unit blade becomes ~0.88 m, readable in 3P.
const DARKMOD_SHORTSWORD_SCALE: f32 = 0.025;
/// longsword01 is ~51 units along −Z. 0.034 → ~1.73 m, a two-hander.
const DARKMOD_LONGSWORD_SCALE: f32 = 0.034;
/// quarterstaff is ~90 units along Z. 0.020 → ~1.80 m. Origin sits 18.6
/// units from the −Z end, so grip_pommel slides the shaft center into the palm.
const DARKMOD_STAFF_SCALE: f32 = 0.020;

fn darkmod_shortsword_on_disk() -> bool {
    motion_asset_path(DARKMOD_SHORTSWORD).is_file()
}

fn darkmod_weapon_on_disk(rel: &str) -> bool {
    motion_asset_path(rel).is_file()
}

fn held_mesh_kind(mode: EquipMode) -> Option<HeldMeshKind> {
    match mode {
        EquipMode::Unarmed => None,
        EquipMode::OneHand | EquipMode::DualWield => Some(HeldMeshKind::ShortSword),
        EquipMode::GreatWeapon => Some(HeldMeshKind::LongSword),
        EquipMode::PoleArm => Some(HeldMeshKind::Staff),
    }
}

fn held_mesh_path(kind: HeldMeshKind) -> &'static str {
    match kind {
        HeldMeshKind::ShortSword => DARKMOD_SHORTSWORD,
        HeldMeshKind::LongSword => DARKMOD_LONGSWORD,
        HeldMeshKind::Staff => DARKMOD_STAFF,
    }
}

fn held_mesh_scale(kind: HeldMeshKind) -> f32 {
    match kind {
        HeldMeshKind::ShortSword => DARKMOD_SHORTSWORD_SCALE,
        HeldMeshKind::LongSword => DARKMOD_LONGSWORD_SCALE,
        HeldMeshKind::Staff => DARKMOD_STAFF_SCALE,
    }
}

#[cfg(not(test))]
const GRIP_CURRENT: &str = "debug/grip_estimates/current.json";
#[cfg(not(test))]
const SWING_CURRENT: &str = "debug/swing_estimates/current.json";
#[cfg(not(test))]
const STAFF_GRIP_CURRENT: &str = "debug/staff_estimates/grip.json";
#[cfg(not(test))]
const STAFF_CAST_GRIP_CURRENT: &str = "debug/staff_estimates/cast_grip.json";
#[cfg(not(test))]
const STAFF_SWING_CURRENT: &str = "debug/staff_estimates/current.json";

/// Hot-loaded 2H grip. `debug/grip_estimates/*.json` are throwaway estimates.
#[derive(Clone, Debug, Deserialize)]
struct TwoHandGrip {
    #[serde(default)]
    id: String,
    right_upper: [f32; 3],
    right_lower: [f32; 3],
    right_hand: [f32; 3],
    /// Chest-space aim for the left fist. Same meaning as `right_hand`.
    /// Zero keeps the old “aim along the forearm” IK hand.
    #[serde(default)]
    left_hand: [f32; 3],
    palm_along: f32,
    palm_in: f32,
    /// Slide the cylinder across the palm (finger axis × into-palm).
    /// This is the missing parenting axis: along the handle's side, not
    /// along the fingers or into the cup.
    #[serde(default)]
    palm_side: f32,
    grip_pommel: f32,
    left_pommel: f32,
    left_x: f32,
    left_y: f32,
    left_wrist_deg: f32,
    /// Extra twist around the finger axis. Positive is counter-clockwise
    /// looking from the wrist toward the fingers; negative is clockwise.
    #[serde(default)]
    right_wrist_deg: f32,
    pole: [f32; 3],
}

impl Default for TwoHandGrip {
    fn default() -> Self {
        Self {
            id: "2h-grip".into(),
            right_upper: [0.0, -0.15, 1.0],
            right_lower: [0.12, 0.05, 0.96],
            right_hand: [0.0, 0.4, 0.8],
            left_hand: [0.0, 0.4, 0.8],
            palm_along: 0.01,
            palm_in: 0.052,
            palm_side: -0.02,
            grip_pommel: 0.0,
            left_pommel: 0.10,
            left_x: -0.015,
            left_y: 0.0,
            left_wrist_deg: -105.0,
            right_wrist_deg: 50.0,
            pole: [0.55, -0.18, 0.22],
        }
    }
}

fn grip_now() -> TwoHandGrip {
    #[cfg(test)]
    {
        TwoHandGrip::default()
    }
    #[cfg(not(test))]
    {
        load_grip_file()
    }
}

fn default_staff_grip() -> TwoHandGrip {
    TwoHandGrip {
        id: "staff-idle-grip".into(),
        right_upper: [0.20, -0.55, 0.70],
        right_lower: [0.12, -0.28, 0.88],
        right_hand: [0.10, -0.20, 0.90],
        left_hand: [0.08, -0.14, 0.88],
        palm_along: 0.01,
        palm_in: 0.052,
        palm_side: -0.02,
        // Shaft center: quarterstaff origin is 18.6 units from −Z, 71.6 from +Z.
        grip_pommel: 0.53,
        left_pommel: 0.16,
        left_x: -0.02,
        left_y: 0.0,
        left_wrist_deg: -105.0,
        right_wrist_deg: 50.0,
        pole: [0.50, -0.28, 0.18],
    }
}

fn default_staff_cast_grip() -> TwoHandGrip {
    TwoHandGrip {
        id: "staff-cast-grip".into(),
        right_upper: [0.0, -0.08, 1.0],
        right_lower: [0.10, 0.08, 0.96],
        right_hand: [0.0, 0.5, 0.85],
        left_hand: [0.0, 0.5, 0.85],
        palm_along: 0.01,
        palm_in: 0.052,
        palm_side: -0.02,
        grip_pommel: 0.53,
        left_pommel: -0.22,
        left_x: -0.015,
        left_y: 0.0,
        left_wrist_deg: -105.0,
        right_wrist_deg: 50.0,
        pole: [0.55, -0.18, 0.22],
    }
}

fn staff_grip_now() -> TwoHandGrip {
    #[cfg(test)]
    {
        default_staff_grip()
    }
    #[cfg(not(test))]
    {
        load_staff_grip_file()
    }
}

fn staff_cast_grip_now() -> TwoHandGrip {
    #[cfg(test)]
    {
        default_staff_cast_grip()
    }
    #[cfg(not(test))]
    {
        load_staff_cast_grip_file()
    }
}

fn staff_showing_cast() -> bool {
    staff_estimate().show.eq_ignore_ascii_case("cast")
}

fn grip_for(kind: HeldMeshKind) -> TwoHandGrip {
    match kind {
        HeldMeshKind::Staff if staff_showing_cast() => staff_cast_grip_now(),
        HeldMeshKind::Staff => staff_grip_now(),
        _ => grip_now(),
    }
}

#[cfg(not(test))]
fn load_grip_file() -> TwoHandGrip {
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<(u64, TwoHandGrip)>> = OnceLock::new();
    let mtime = fs::metadata(GRIP_CURRENT)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let mut slot = CACHE
        .get_or_init(|| Mutex::new((0, TwoHandGrip::default())))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if slot.0 != mtime {
        slot.0 = mtime;
        if let Ok(body) = fs::read_to_string(GRIP_CURRENT) {
            if let Ok(g) = serde_json::from_str::<TwoHandGrip>(&body) {
                slot.1 = g;
            }
        }
    }
    slot.1.clone()
}

#[cfg(not(test))]
fn load_staff_grip_file() -> TwoHandGrip {
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<(u64, TwoHandGrip)>> = OnceLock::new();
    let mtime = fs::metadata(STAFF_GRIP_CURRENT)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let mut slot = CACHE
        .get_or_init(|| Mutex::new((0, default_staff_grip())))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if slot.0 != mtime {
        slot.0 = mtime;
        if let Ok(body) = fs::read_to_string(STAFF_GRIP_CURRENT) {
            if let Ok(g) = serde_json::from_str::<TwoHandGrip>(&body) {
                slot.1 = g;
            }
        }
    }
    slot.1.clone()
}

#[cfg(not(test))]
fn load_staff_cast_grip_file() -> TwoHandGrip {
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<(u64, TwoHandGrip)>> = OnceLock::new();
    let mtime = fs::metadata(STAFF_CAST_GRIP_CURRENT)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let mut slot = CACHE
        .get_or_init(|| Mutex::new((0, default_staff_cast_grip())))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if slot.0 != mtime {
        slot.0 = mtime;
        if let Ok(body) = fs::read_to_string(STAFF_CAST_GRIP_CURRENT) {
            if let Ok(g) = serde_json::from_str::<TwoHandGrip>(&body) {
                slot.1 = g;
            }
        }
    }
    slot.1.clone()
}

fn grip_vec(v: [f32; 3]) -> Vec3 {
    Vec3::new(v[0], v[1], v[2])
}

/// Handle + tip in chest space. The 2H attack interpolates these so the
/// steel itself passes through the locked straight-out plant.
#[derive(Clone, Copy, Debug, Deserialize)]
struct SwordLoc {
    handle: [f32; 3],
    tip: [f32; 3],
}

fn default_wind_loc() -> SwordLoc {
    SwordLoc {
        handle: [-0.20, 0.18, 0.36],
        tip: [-0.34, 0.96, 0.22],
    }
}

fn default_plant_loc() -> SwordLoc {
    SwordLoc {
        handle: [-0.16, 0.12, 0.40],
        tip: [-0.16, 0.12, 1.40],
    }
}

fn default_follow_loc() -> SwordLoc {
    SwordLoc {
        handle: [0.22, -0.10, 0.40],
        tip: [0.48, -0.26, 0.82],
    }
}

fn default_idle_loc() -> SwordLoc {
    SwordLoc {
        handle: [-0.08, -0.06, 0.34],
        tip: [-0.10, 0.78, 0.26],
    }
}

fn default_return_wind_loc() -> SwordLoc {
    SwordLoc {
        handle: [0.22, 0.28, 0.26],
        tip: [0.38, 1.00, -0.06],
    }
}

fn default_return_plant_loc() -> SwordLoc {
    SwordLoc {
        handle: [0.16, 0.12, 0.40],
        tip: [0.16, 0.12, 1.40],
    }
}

fn default_return_follow_loc() -> SwordLoc {
    SwordLoc {
        handle: [-0.18, -0.16, 0.32],
        tip: [-0.38, -0.32, 0.68],
    }
}

fn default_staff_wind_loc() -> SwordLoc {
    SwordLoc {
        handle: [-0.12, 0.32, 0.16],
        tip: [-0.16, 1.28, -0.18],
    }
}

fn default_staff_plant_loc() -> SwordLoc {
    SwordLoc {
        handle: [-0.10, 0.10, 0.42],
        tip: [-0.10, 0.10, 1.55],
    }
}

fn default_staff_follow_loc() -> SwordLoc {
    SwordLoc {
        handle: [0.08, -0.20, 0.36],
        tip: [0.14, -0.78, 0.68],
    }
}

fn default_staff_idle_loc() -> SwordLoc {
    SwordLoc {
        handle: [-0.22, -0.26, 0.16],
        tip: [-0.20, 0.68, 0.08],
    }
}

fn default_staff_cast_loc() -> SwordLoc {
    SwordLoc {
        handle: [-0.10, -0.04, 0.30],
        tip: [-0.08, 1.10, 0.22],
    }
}

fn default_cast_loc() -> SwordLoc {
    default_idle_loc()
}

fn default_staff_return_wind_loc() -> SwordLoc {
    SwordLoc {
        handle: [0.12, 0.32, 0.16],
        tip: [0.16, 1.28, -0.18],
    }
}

fn default_staff_return_plant_loc() -> SwordLoc {
    SwordLoc {
        handle: [0.10, 0.10, 0.42],
        tip: [0.10, 0.10, 1.55],
    }
}

fn default_staff_return_follow_loc() -> SwordLoc {
    SwordLoc {
        handle: [-0.08, -0.20, 0.36],
        tip: [-0.14, -0.78, 0.68],
    }
}

#[derive(Clone, Debug, Deserialize)]
struct SwingEstimate {
    #[serde(default)]
    show: String,
    #[serde(default = "default_wind_loc")]
    wind: SwordLoc,
    start: SwordLoc,
    end: SwordLoc,
    #[serde(default = "default_idle_loc")]
    idle: SwordLoc,
    #[serde(default = "default_return_wind_loc")]
    return_wind: SwordLoc,
    #[serde(default = "default_return_plant_loc")]
    return_start: SwordLoc,
    #[serde(default = "default_return_follow_loc")]
    return_end: SwordLoc,
    #[serde(default = "default_cast_loc")]
    cast: SwordLoc,
}

impl Default for SwingEstimate {
    fn default() -> Self {
        Self {
            show: "off".into(),
            wind: default_wind_loc(),
            start: default_plant_loc(),
            end: default_follow_loc(),
            idle: default_idle_loc(),
            return_wind: default_return_wind_loc(),
            return_start: default_return_plant_loc(),
            return_end: default_return_follow_loc(),
            cast: default_cast_loc(),
        }
    }
}

fn default_staff_estimate() -> SwingEstimate {
    SwingEstimate {
        show: "off".into(),
        wind: default_staff_wind_loc(),
        start: default_staff_plant_loc(),
        end: default_staff_follow_loc(),
        idle: default_staff_idle_loc(),
        return_wind: default_staff_return_wind_loc(),
        return_start: default_staff_return_plant_loc(),
        return_end: default_staff_return_follow_loc(),
        cast: default_staff_cast_loc(),
    }
}

fn lerp_arr(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

fn lerp_loc(a: SwordLoc, b: SwordLoc, t: f32) -> SwordLoc {
    SwordLoc {
        handle: lerp_arr(a.handle, b.handle, t),
        tip: lerp_arr(a.tip, b.tip, t),
    }
}

fn ease_in(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    u * u
}

fn ease_out(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    1.0 - (1.0 - u) * (1.0 - u)
}

fn smoothstep(u: f32) -> f32 {
    let u = u.clamp(0.0, 1.0);
    u * u * (3.0 - 2.0 * u)
}

/// 0..1 athletic drive: small prep, peak through the strike, ease out on recover.
fn two_hand_cut_drive(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    if t <= 0.42 {
        ease_in(t / 0.42)
    } else if t <= 0.68 {
        1.0
    } else {
        1.0 - smoothstep((t - 0.68) / 0.32)
    }
}

fn two_hand_arc(swing: &SwingPose) -> SlashArc {
    if swing.combo() % 2 == 1 {
        SlashArc::Return
    } else {
        SlashArc::Diagonal
    }
}

/// Pelvis yaw into the cut. Multiplied onto the idle/walk hip so the
/// stance turns as a unit and the feet stay at idle width.
fn two_hand_cut_hip(t: f32, arc: SlashArc) -> Quat {
    let drive = two_hand_cut_drive(t);
    let coil = 1.0 - drive;
    let mut yaw = -0.22 * coil + 0.42 * drive;
    if arc == SlashArc::Return {
        yaw = -yaw;
    }
    Quat::from_axis_angle(Vec3::Y, yaw)
}

/// 2H attack is IK. Playing Slash2H would replace idle/walk legs with Tide's
/// collapsed stance.
fn two_hand_keeps_loco_clip(held: super::HeldWeapon) -> bool {
    held.is_two_handed()
}

/// Clavicles coil then open into the cut. Return flips the yaw. Root stays put.
fn two_hand_cut_shoulder(joint: &AvatarJoint, t: f32, arc: SlashArc) -> Quat {
    let drive = two_hand_cut_drive(t);
    let coil = 1.0 - drive;
    let right = joint.sign < 0.0;
    let mut yaw = if right {
        -0.12 * coil + 0.18 * drive
    } else {
        0.08 * coil + 0.14 * drive
    };
    if arc == SlashArc::Return {
        yaw = -yaw;
    }
    let pitch = -0.05 * coil - 0.07 * drive;
    joint.rest * Quat::from_axis_angle(Vec3::Y, yaw) * Quat::from_axis_angle(Vec3::X, pitch)
}


fn two_hand_cut_keys(arc: SlashArc, pole: bool) -> (SwordLoc, SwordLoc, SwordLoc, SwordLoc) {
    if pole {
        let s = staff_estimate();
        return match arc {
            SlashArc::Return => (s.return_wind, s.return_start, s.return_end, s.idle),
            _ => (s.wind, s.start, s.end, s.idle),
        };
    }
    match arc {
        SlashArc::Return => (
            default_return_wind_loc(),
            default_return_plant_loc(),
            default_return_follow_loc(),
            default_idle_loc(),
        ),
        _ => (
            default_wind_loc(),
            default_plant_loc(),
            default_follow_loc(),
            default_idle_loc(),
        ),
    }
}

/// 0.48s cut: small coil → locked straight-out plant at 0.42 → low guard → wrap.
/// Great-weapon diagonal and return are baked. Staff hot-loads `debug/staff_estimates`.
fn two_hand_cut_at(t: f32, arc: SlashArc) -> SwordLoc {
    two_hand_cut_at_for(t, arc, false)
}

fn two_hand_cut_at_for(t: f32, arc: SlashArc, pole: bool) -> SwordLoc {
    let (wind, plant, follow, idle) = two_hand_cut_keys(arc, pole);
    let t = t.clamp(0.0, 1.0);
    if t <= 0.42 {
        lerp_loc(wind, plant, ease_in(t / 0.42))
    } else if t <= 0.68 {
        lerp_loc(plant, follow, ease_out((t - 0.42) / 0.26))
    } else {
        lerp_loc(follow, idle, smoothstep((t - 0.68) / 0.32))
    }
}

fn staff_estimate() -> SwingEstimate {
    #[cfg(test)]
    {
        default_staff_estimate()
    }
    #[cfg(not(test))]
    {
        load_staff_swing_file().unwrap_or_else(default_staff_estimate)
    }
}

fn two_hand_sword_loc(cutting: bool, t: f32, arc: SlashArc, pole: bool) -> Option<SwordLoc> {
    if cutting {
        Some(two_hand_cut_at_for(t, arc, pole))
    } else if pole {
        let s = staff_estimate();
        loc_from_show(&s).or(Some(s.idle))
    } else {
        swing_shown()
    }
}

fn loc_from_show(s: &SwingEstimate) -> Option<SwordLoc> {
    match s.show.to_ascii_lowercase().as_str() {
        "wind" => Some(s.wind),
        "start" => Some(s.start),
        "end" => Some(s.end),
        "idle" => Some(s.idle),
        "cast" => Some(s.cast),
        "return_wind" => Some(s.return_wind),
        "return_start" => Some(s.return_start),
        "return_end" => Some(s.return_end),
        _ => None,
    }
}

fn swing_shown() -> Option<SwordLoc> {
    #[cfg(test)]
    {
        None
    }
    #[cfg(not(test))]
    {
        loc_from_show(&load_swing_file()?)
    }
}

fn overlay_label_from(s: &SwingEstimate) -> Option<String> {
    match s.show.to_ascii_lowercase().as_str() {
        "wind" => Some("wind".into()),
        "start" => Some("start".into()),
        "end" => Some("end".into()),
        "idle" => Some("idle".into()),
        "cast" => Some("cast".into()),
        "return_wind" => Some("return wind".into()),
        "return_start" => Some("return start".into()),
        "return_end" => Some("return end".into()),
        _ => None,
    }
}

/// F3 label. `None` when the overlay is off or the file is missing.
pub fn swing_overlay_label() -> Option<String> {
    #[cfg(test)]
    {
        None
    }
    #[cfg(not(test))]
    {
        overlay_label_from(&staff_estimate())
            .or_else(|| load_swing_file().and_then(|s| overlay_label_from(&s)))
    }
}

#[cfg(not(test))]
fn load_swing_file() -> Option<SwingEstimate> {
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<(u64, Option<SwingEstimate>)>> = OnceLock::new();
    let mtime = fs::metadata(SWING_CURRENT)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let mut slot = CACHE
        .get_or_init(|| Mutex::new((0, None)))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if slot.0 != mtime {
        slot.0 = mtime;
        slot.1 = fs::read_to_string(SWING_CURRENT)
            .ok()
            .and_then(|body| serde_json::from_str::<SwingEstimate>(&body).ok());
    }
    slot.1.clone()
}

#[cfg(not(test))]
fn load_staff_swing_file() -> Option<SwingEstimate> {
    use std::sync::{Mutex, OnceLock};
    static CACHE: OnceLock<Mutex<(u64, Option<SwingEstimate>)>> = OnceLock::new();
    let mtime = fs::metadata(STAFF_SWING_CURRENT)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let mut slot = CACHE
        .get_or_init(|| Mutex::new((0, None)))
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    if slot.0 != mtime {
        slot.0 = mtime;
        slot.1 = fs::read_to_string(STAFF_SWING_CURRENT)
            .ok()
            .and_then(|body| serde_json::from_str::<SwingEstimate>(&body).ok());
    }
    slot.1.clone()
}

/// Chrona / VRM T-pose has identity hand rotation; fingers continue along
/// ±X (right = -X), not +Y. Place the grip on that child axis so the blade
/// leaves the fist instead of sitting above the wrist.
fn held_sword_grip(along: Vec3, blade: Vec3) -> Transform {
    let along = along.normalize_or(Vec3::NEG_X);
    let blade = blade.normalize_or(Vec3::Y);
    let rot = Quat::from_rotation_arc(Vec3::Y, blade);
    // Grip mesh is centered at local +Y 0.05. Put that point in the palm so
    // the handle occupies the fist instead of sprouting from the wrist.
    let fist = along * 0.07 + Vec3::NEG_Y * 0.028;
    Transform::from_translation(fist - rot * Vec3::new(0.0, 0.05, 0.0)).with_rotation(rot)
}

/// Dark Mod shortsword is authored along −Z (tip at −30). Local +X is the
/// wide face (~9 units) and +Y is the thin edge (~1.5). Hilt stays in the
/// fist (`along`). `blade` is the tip direction: idle passes chest-up so
/// the rest pose is a hip-held sword, not a cane; authored slashes pass
/// the finger axis so the cut orients the steel. Dual wield then yaws that
/// tip slightly toward the foe.
/// Roll the thin edge onto the side plane so the crossguard stays the same
/// in first and third person (first-person hilt is the correct one).

fn held_weapon_tf(along: Vec3, blade: Vec3, kind: HeldMeshKind) -> Transform {
    let path = held_mesh_path(kind);
    if !darkmod_weapon_on_disk(path) && !darkmod_shortsword_on_disk() {
        return held_sword_grip(along, blade);
    }
    let along = along.normalize_or(Vec3::NEG_X);
    let dir = blade.normalize_or(Vec3::Y);
    let mut rot = Quat::from_rotation_arc(Vec3::NEG_Z, dir);
    let edge = rot * Vec3::Y;
    let stand = (Vec3::Y - dir * dir.dot(Vec3::Y)).normalize_or(Vec3::X);
    rot = Quat::from_rotation_arc(edge, stand) * rot;
    let scale = held_mesh_scale(kind);
    // Guard sits at mesh origin; the wrap-able grip is toward +Z (pommel).
    // Slide the origin so that cylinder occupies the palm, not the fingertips.
    let g = grip_for(kind);
    let palm = match kind {
        HeldMeshKind::LongSword | HeldMeshKind::Staff => {
            let side = along.cross(Vec3::NEG_Y).normalize_or(Vec3::Z);
            along * g.palm_along + Vec3::NEG_Y * g.palm_in + side * g.palm_side
        }
        HeldMeshKind::ShortSword => along * 0.07 + Vec3::NEG_Y * 0.028,
    };
    let grip = match kind {
        HeldMeshKind::LongSword | HeldMeshKind::Staff => rot * Vec3::Z * g.grip_pommel,
        HeldMeshKind::ShortSword => Vec3::ZERO,
    };
    Transform::from_translation(palm - grip)
        .with_rotation(rot)
        .with_scale(Vec3::splat(scale))
}

/// Chest-space point the left palm must reach: 9 cm toward the pommel from
/// the right-hand seating, then out toward the character's left so the
/// off-hand sits on the handle instead of crossing the right fist.
fn two_hand_left_target(
    hand_pos: Vec3,
    hand_rot: Quat,
    along: Vec3,
    blade: Vec3,
    kind: HeldMeshKind,
) -> Vec3 {
    let g = grip_for(kind);
    let weapon = held_weapon_tf(along, blade, kind);
    let pommel = weapon.rotation * Vec3::Z;
    let right_grip = hand_pos + hand_rot * weapon.translation;
    right_grip + hand_rot * pommel * g.left_pommel + Vec3::X * g.left_x + Vec3::Y * g.left_y
}

/// Analytical two-bone IK. `forearm_offset` / `hand_offset` are bind-local
/// child translations. Returns parent-space rotations for upper then lower.
fn two_bone_ik(
    upper: &AvatarJoint,
    lower: &AvatarJoint,
    forearm_offset: Vec3,
    hand_offset: Vec3,
    parent_rot: Quat,
    upper_origin: Vec3,
    target: Vec3,
    pole: Vec3,
) -> (Quat, Quat) {
    let upper_len = forearm_offset.length().max(0.05);
    let lower_len = hand_offset.length().max(0.05);
    let to_target = target - upper_origin;
    let reach = (upper_len + lower_len) * 0.995;
    let dist = to_target.length().clamp(0.05, reach);
    let dir = to_target.normalize_or(Vec3::Z);
    let cos_sh = ((upper_len * upper_len + dist * dist - lower_len * lower_len)
        / (2.0 * upper_len * dist))
        .clamp(-1.0, 1.0);
    let shoulder_off = cos_sh.acos();
    let pole_vec = pole - upper_origin;
    let mut plane = dir.cross(pole_vec);
    if plane.length_squared() < 1e-6 {
        plane = dir.any_orthonormal_vector();
    }
    let elbow_dir = Quat::from_axis_angle(plane.normalize(), shoulder_off) * dir;
    let upper_rot = aim_bone(upper, parent_rot.inverse() * elbow_dir);
    let upper_world = parent_rot * upper_rot;
    let elbow = upper_origin + upper_world * forearm_offset;
    let to_hand = (target - elbow).normalize_or(dir);
    let lower_rot = aim_bone(lower, upper_world.inverse() * to_hand);
    (upper_rot, lower_rot)
}

fn ik_hand_pos(
    upper_rot: Quat,
    lower_rot: Quat,
    parent_rot: Quat,
    upper_origin: Vec3,
    forearm_offset: Vec3,
    hand_offset: Vec3,
) -> Vec3 {
    let upper_world = parent_rot * upper_rot;
    upper_origin + upper_world * forearm_offset + upper_world * lower_rot * hand_offset
}

fn chain_entity(
    links: &[(Entity, i8, u8, Entity)],
    root: Entity,
    side: i8,
    rank: u8,
) -> Option<Entity> {
    links
        .iter()
        .find(|&&(r, s, k, _)| r == root && s == side && k == rank)
        .map(|&(_, _, _, entity)| entity)
}

fn left_two_hand_rots(
    upper: &AvatarJoint,
    lower: &AvatarJoint,
    hand: &AvatarJoint,
    forearm_offset: Vec3,
    hand_offset: Vec3,
    parent_rot: Quat,
    upper_origin: Vec3,
    target: Vec3,
    blade_chest: Vec3,
    kind: HeldMeshKind,
) -> (Quat, Quat, Quat) {
    let g = grip_for(kind);
    let pole = grip_vec(g.pole);
    let (upper_rot, lower_rot) = two_bone_ik(
        upper,
        lower,
        forearm_offset,
        hand_offset,
        parent_rot,
        upper_origin,
        target,
        pole,
    );
    let lower_world = parent_rot * upper_rot * lower_rot;
    let forearm_dir = lower_world * (lower.rest * lower.axis);
    let hand_aim = grip_vec(g.left_hand);
    let hand_goal = if hand_aim.length_squared() > 0.01 {
        hand_aim
    } else {
        forearm_dir
    };
    let mut hand_rot = aim_bone(hand, lower_world.inverse() * hand_goal);
    let blade_local = (lower_world * hand_rot).inverse() * blade_chest;
    hand_rot *= sword_wrist_roll(hand.axis, blade_local);
    hand_rot *= Quat::from_axis_angle(hand.axis, g.left_wrist_deg.to_radians());
    (upper_rot, lower_rot, hand_rot)
}

fn right_two_hand_swing_rots(
    upper: &AvatarJoint,
    lower: &AvatarJoint,
    hand: &AvatarJoint,
    forearm_offset: Vec3,
    hand_offset: Vec3,
    parent_rot: Quat,
    upper_origin: Vec3,
    target: Vec3,
    blade_chest: Vec3,
    kind: HeldMeshKind,
) -> (Quat, Quat, Quat) {
    let g = grip_for(kind);
    let pole = Vec3::new(-g.pole[0], g.pole[1], g.pole[2]);
    let (upper_rot, lower_rot) = two_bone_ik(
        upper,
        lower,
        forearm_offset,
        hand_offset,
        parent_rot,
        upper_origin,
        target,
        pole,
    );
    let lower_world = parent_rot * upper_rot * lower_rot;
    let forearm_dir = lower_world * (lower.rest * lower.axis);
    let hand_aim = grip_vec(g.right_hand);
    let hand_goal = if hand_aim.length_squared() > 0.01 {
        hand_aim
    } else {
        forearm_dir
    };
    let mut hand_rot = aim_bone(hand, lower_world.inverse() * hand_goal);
    let blade_local = (lower_world * hand_rot).inverse() * blade_chest;
    hand_rot *= sword_wrist_roll(hand.axis, blade_local);
    hand_rot *= Quat::from_axis_angle(hand.axis, g.right_wrist_deg.to_radians());
    (upper_rot, lower_rot, hand_rot)
}

/// Dual-wield: the steel attacks the foe (chest +Z). Keep the clip's cut
/// angle, but never let the tip come back at the body.
fn dual_blade_toward_foe(blade: Vec3, from_chest: Quat) -> Vec3 {
    let blade = blade.normalize_or(Vec3::Y);
    let mut world = from_chest * blade;
    if world.z < 0.0 {
        world.z = -world.z;
    }
    world = Vec3::new(world.x, world.y, world.z.max(0.42)).normalize_or(Vec3::Z);
    from_chest.inverse() * world
}

/// Slight wrist roll so the palm cups a chest-up hilt. The first pass used
/// `along × blade` and opened the hand the wrong way; wrap is the reverse.
fn sword_wrist_roll(along: Vec3, blade: Vec3) -> Quat {
    let along = along.normalize_or(Vec3::NEG_X);
    let blade = blade.normalize_or(Vec3::Y);
    let palm = (Vec3::NEG_Y - along * along.dot(Vec3::NEG_Y)).normalize_or_zero();
    let wrap = blade.cross(along).normalize_or_zero();
    if palm.length_squared() < 1e-4 || wrap.length_squared() < 1e-4 {
        return Quat::IDENTITY;
    }
    let (axis, angle) = Quat::from_rotation_arc(palm, wrap).to_axis_angle();
    Quat::from_axis_angle(axis, angle.min(0.62))
}

fn aim_bone(joint: &AvatarJoint, to: Vec3) -> Quat {
    let from = (joint.rest * joint.axis).normalize_or(Vec3::NEG_X);
    Quat::from_rotation_arc(from, to.normalize_or(Vec3::Z)) * joint.rest
}

/// Chest-space directions for the right-arm chain. Negative X is out to the
/// character's right; negative Y is down. Forearm/hand stay forward-down so
/// a 56cm blade cannot rise through the skull. Idle carry leaves the clavicle
/// at rest — aiming `Shoulder` like an upper arm hunches the socket.
/// Left arm reaches behind and down so the palm plants while sitting.
/// Chest space: +X left, -Y down, -Z back.
fn rest_support_goal(kind: JointKind) -> Vec3 {
    match kind {
        JointKind::Shoulder => Vec3::new(0.88, -0.10, -0.46),
        JointKind::UpperArm => Vec3::new(0.36, -0.46, -0.81),
        JointKind::LowerArm => Vec3::new(0.12, -0.74, -0.66),
        JointKind::Hand => Vec3::new(0.06, -0.90, -0.43),
        _ => Vec3::NEG_Z,
    }
    .normalize()
}

fn sword_carry_goal(kind: JointKind) -> Vec3 {
    sword_carry_goal_side(kind, -1.0)
}

fn sword_carry_goal_side(kind: JointKind, sign: f32) -> Vec3 {
    let goal = match kind {
        JointKind::Shoulder => Vec3::NEG_X,
        JointKind::UpperArm => Vec3::new(-0.38, -0.88, 0.28),
        JointKind::LowerArm => Vec3::new(-0.08, -0.32, 0.94),
        JointKind::Hand => Vec3::new(-0.06, -0.28, 0.955),
        _ => Vec3::Z,
    }
    .normalize();
    if sign > 0.0 {
        Vec3::new(-goal.x, goal.y, goal.z).normalize()
    } else {
        goal
    }
}

/// Shared 2H grip. Right chain keeps a 1H-style fist (that seating already
/// puts steel in the palm). Left forearm reaches across onto the same
/// handle, a palm-width lower toward the pommel.
fn great_weapon_carry_goal(kind: JointKind, sign: f32, mesh: HeldMeshKind) -> Vec3 {
    let g = grip_for(mesh);
    if sign < 0.0 {
        return match kind {
            JointKind::Shoulder => Vec3::NEG_X,
            JointKind::UpperArm => grip_vec(g.right_upper),
            JointKind::LowerArm => grip_vec(g.right_lower),
            JointKind::Hand => grip_vec(g.right_hand),
            _ => Vec3::Z,
        }
        .normalize();
    }
    match kind {
        JointKind::Shoulder => Vec3::X,
        JointKind::UpperArm => Vec3::new(-0.52, -0.52, 0.68),
        JointKind::LowerArm => Vec3::new(-0.58, -0.28, 0.76),
        JointKind::Hand => {
            let aim = grip_vec(g.left_hand);
            if aim.length_squared() > 0.01 {
                aim
            } else {
                Vec3::new(-0.06, -0.40, 0.915)
            }
        }
        _ => Vec3::Z,
    }
    .normalize()
}

fn great_weapon_slash_goal(kind: JointKind, sign: f32, t: f32, arc: SlashArc) -> Vec3 {
    let base = sword_slash_goal(kind, t, arc);
    if sign < 0.0 {
        return base;
    }
    (base + Vec3::new(0.0, -0.24, 0.0)).normalize_or(base)
}

fn great_weapon_goal(
    kind: JointKind,
    sign: f32,
    attacking: bool,
    t: f32,
    arc: SlashArc,
    mesh: HeldMeshKind,
) -> Vec3 {
    if attacking {
        great_weapon_slash_goal(kind, sign, t, arc)
    } else {
        great_weapon_carry_goal(kind, sign, mesh)
    }
}

fn chest_goal_for_side(goal: Vec3, sign: f32) -> Vec3 {
    if sign > 0.0 {
        Vec3::new(-goal.x, goal.y, goal.z).normalize_or(goal)
    } else {
        goal
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SlashArc {
    Diagonal = 0,
    Return = 1,
    Sweep = 2,
}

const SLASH_ARC_COUNT: u8 = 3;

fn slash_arc_from(index: u8) -> SlashArc {
    match index % SLASH_ARC_COUNT {
        1 => SlashArc::Return,
        2 => SlashArc::Sweep,
        _ => SlashArc::Diagonal,
    }
}

fn slash_hit(kind: JointKind) -> Vec3 {
    match kind {
        JointKind::Shoulder => Vec3::new(-0.10, -0.30, 0.95),
        JointKind::UpperArm => Vec3::new(0.04, -0.22, 0.97),
        JointKind::LowerArm => Vec3::new(0.02, -0.18, 0.98),
        JointKind::Hand => Vec3::new(0.02, -0.22, 0.975),
        _ => Vec3::Z,
    }
}

/// Fallback IK when a Tide 1H oneshot is not playing (`grip_right` is
/// false). Live 1H attacks use Slash1H / Return / Sweep; those skip this
/// path and seat the blade on the posed fingers instead. y stays negative
/// so a missing-clip fallback cannot lift the blade through the skull.
fn slash_keyframes(kind: JointKind, arc: SlashArc) -> (Vec3, Vec3, Vec3) {
    let hit = slash_hit(kind);
    let (wind, follow) = match (kind, arc) {
        (JointKind::Shoulder, SlashArc::Diagonal) => (
            Vec3::new(-0.82, -0.42, 0.32),
            Vec3::new(0.42, -0.48, 0.62),
        ),
        (JointKind::Shoulder, SlashArc::Return) => (
            Vec3::new(0.42, -0.48, 0.62),
            Vec3::new(-0.78, -0.45, 0.35),
        ),
        (JointKind::Shoulder, SlashArc::Sweep) => (
            Vec3::new(-0.85, -0.32, 0.38),
            Vec3::new(0.58, -0.36, 0.58),
        ),
        (JointKind::UpperArm, SlashArc::Diagonal) => (
            Vec3::new(-0.78, -0.38, 0.42),
            Vec3::new(0.55, -0.40, 0.62),
        ),
        (JointKind::UpperArm, SlashArc::Return) => (
            Vec3::new(0.55, -0.40, 0.62),
            Vec3::new(-0.72, -0.42, 0.42),
        ),
        (JointKind::UpperArm, SlashArc::Sweep) => (
            Vec3::new(-0.80, -0.28, 0.48),
            Vec3::new(0.62, -0.32, 0.60),
        ),
        (JointKind::LowerArm, SlashArc::Diagonal) => (
            Vec3::new(-0.58, -0.26, 0.76),
            Vec3::new(0.50, -0.28, 0.80),
        ),
        (JointKind::LowerArm, SlashArc::Return) => (
            Vec3::new(0.50, -0.28, 0.80),
            Vec3::new(-0.55, -0.28, 0.76),
        ),
        (JointKind::LowerArm, SlashArc::Sweep) => (
            Vec3::new(-0.62, -0.22, 0.74),
            Vec3::new(0.62, -0.24, 0.74),
        ),
        (JointKind::Hand, SlashArc::Diagonal) => (
            Vec3::new(-0.72, -0.28, 0.62),
            Vec3::new(0.58, -0.30, 0.74),
        ),
        (JointKind::Hand, SlashArc::Return) => (
            Vec3::new(0.58, -0.30, 0.74),
            Vec3::new(-0.70, -0.28, 0.64),
        ),
        (JointKind::Hand, SlashArc::Sweep) => (
            Vec3::new(-0.78, -0.24, 0.58),
            Vec3::new(0.72, -0.26, 0.62),
        ),
        _ => (Vec3::Z, Vec3::Z),
    };
    (wind, hit, follow)
}

fn sword_slash_goal(kind: JointKind, t: f32, arc: SlashArc) -> Vec3 {
    let t = t.clamp(0., 1.);
    let (wind, hit, follow) = slash_keyframes(kind, arc);
    let to = if t < 0.45 {
        wind.lerp(hit, t / 0.45)
    } else {
        hit.lerp(follow, (t - 0.45) / 0.55)
    };
    to.normalize()
}

fn sword_carry_rotation(joint: &AvatarJoint) -> Quat {
    aim_bone(joint, sword_carry_goal(joint.kind))
}

fn sword_slash_rotation(joint: &AvatarJoint, t: f32) -> Quat {
    sword_slash_rotation_arc(joint, t, SlashArc::Diagonal)
}

fn sword_slash_rotation_arc(joint: &AvatarJoint, t: f32, arc: SlashArc) -> Quat {
    aim_bone(joint, sword_slash_goal(joint.kind, t, arc))
}

fn sword_chain_rank(kind: JointKind) -> Option<u8> {
    match kind {
        JointKind::Shoulder => Some(0),
        JointKind::UpperArm => Some(1),
        JointKind::LowerArm => Some(2),
        JointKind::Hand => Some(3),
        _ => None,
    }
}

fn swing_clock(swing: &SwingPose, now: f32) -> f32 {
    swing
        .started_at()
        .map_or(1., |start| ((now - start) / 0.48).clamp(0., 1.))
}

fn spawn_held_sword(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    hand: Entity,
    along: Vec3,
    name: &str,
    kind: HeldMeshKind,
) {
    let grip = held_weapon_tf(along, Vec3::Y, kind);
    let path = held_mesh_path(kind);
    let path = if darkmod_weapon_on_disk(path) {
        path
    } else if darkmod_shortsword_on_disk() {
        DARKMOD_SHORTSWORD
    } else {
        ""
    };
    if !path.is_empty() {
        let scene: Handle<WorldAsset> = asset_server
            .load(GltfAssetLabel::Scene(0).from_asset(path.to_string()));
        let root = commands
            .spawn((
                HeldWeaponMesh,
                HeldDarkmodSword,
                kind,
                Name::new(name.to_string()),
                WorldAssetRoot(scene),
                grip,
                Visibility::Visible,
            ))
            .id();
        commands.entity(hand).add_child(root);
        return;
    }
    let cube = meshes.add(Cuboid::new(1., 1., 1.));
    let wood = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.12, 0.08),
        perceptual_roughness: 0.86,
        metallic: 0.0,
        reflectance: 0.22,
        ..default()
    });
    let steel = materials.add(StandardMaterial {
        base_color: Color::srgb(0.62, 0.64, 0.68),
        perceptual_roughness: 0.45,
        metallic: 0.32,
        reflectance: 0.38,
        ..default()
    });
    let root = commands
        .spawn((
            HeldWeaponMesh,
            kind,
            Name::new(name.to_string()),
            grip,
            Visibility::Visible,
        ))
        .id();
    commands.entity(hand).add_child(root);
    commands.entity(root).with_children(|c| {
        c.spawn((
            Mesh3d(cube.clone()),
            MeshMaterial3d(wood),
            Transform::from_xyz(0.0, 0.05, 0.0).with_scale(Vec3::new(0.026, 0.11, 0.026)),
            Visibility::Visible,
        ));
        c.spawn((
            Mesh3d(cube.clone()),
            MeshMaterial3d(steel.clone()),
            Transform::from_xyz(0.0, 0.12, 0.0).with_scale(Vec3::new(0.09, 0.016, 0.032)),
            Visibility::Visible,
        ));
        c.spawn((
            Mesh3d(cube),
            MeshMaterial3d(steel),
            Transform::from_xyz(0.0, 0.42, 0.0).with_scale(Vec3::new(0.032, 0.56, 0.012)),
            Visibility::Visible,
        ));
    });
}

/// Keep parented swords in sync with [`EquipState`]. Unarmed despawns both.
pub fn attach_held_weapon(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    equip: Option<Res<EquipState>>,
    hands: Query<(Entity, &AvatarJoint, Option<&Name>)>,
    local_roots: Query<(), With<PlayerAvatarMesh>>,
    existing: Query<(Entity, &ChildOf, Option<&HeldMeshKind>), With<HeldWeaponMesh>>,
) {
    let mode = equip.map(|e| e.mode).unwrap_or(EquipMode::Unarmed);
    let want_kind = held_mesh_kind(mode);
    let mut have_right = false;
    let mut have_left = false;
    let mut extras = Vec::new();
    for (entity, parent, have_kind) in &existing {
        let Ok((_, joint, _)) = hands.get(parent.parent()) else {
            extras.push(entity);
            continue;
        };
        let right = joint.sign < 0.0;
        let keep = if right {
            mode.right_weapon()
        } else {
            mode.left_weapon()
        } && have_kind.copied() == want_kind;
        if keep {
            if right {
                have_right = true;
            } else {
                have_left = true;
            }
        } else {
            extras.push(entity);
        }
    }
    for entity in extras {
        commands.entity(entity).despawn();
    }
    let mut targets: Vec<(Entity, Vec3, bool, String)> = Vec::new();
    for (hand, joint, hand_name) in &hands {
        if joint.kind != JointKind::Hand || local_roots.get(joint.root).is_err() {
            continue;
        }
        let right = joint.sign < 0.0;
        let want = if right {
            mode.right_weapon() && !have_right
        } else {
            mode.left_weapon() && !have_left
        };
        if !want {
            continue;
        }
        let label = hand_name
            .map(Name::as_str)
            .unwrap_or(if right { "right hand" } else { "left hand" })
            .to_string();
        targets.push((hand, joint.axis, right, label));
    }
    let Some(kind) = want_kind else {
        return;
    };
    for (hand, along, right, label) in targets {
        info!("Parented held sword to {label} along {along:?} ({kind:?})");
        spawn_held_sword(
            &mut commands,
            &mut meshes,
            &mut materials,
            &asset_server,
            hand,
            along,
            if right {
                "HeldSword"
            } else {
                "HeldSwordOffhand"
            },
            kind,
        );
    }
}

fn resting_sword_visibility(resting: bool) -> Visibility {
    if resting {
        Visibility::Hidden
    } else {
        Visibility::Visible
    }
}

/// Rest puts the steel away. R again or WASD brings it back.
pub fn stow_held_swords_while_resting(
    fighters: Query<&crate::rpg::Fighter, With<crate::player::Player>>,
    mut swords: Query<&mut Visibility, With<HeldWeaponMesh>>,
) {
    let Ok(fighter) = fighters.single() else {
        return;
    };
    let want = resting_sword_visibility(fighter.resting);
    for mut vis in &mut swords {
        if *vis != want {
            *vis = want;
        }
    }
}

pub fn hide_held_weapon_hulls(
    roots: Query<Entity, With<HeldWeaponMesh>>,
    children: Query<&Children>,
    names: Query<&Name>,
    mut vis: Query<&mut Visibility>,
) {
    for root in &roots {
        let mut stack = vec![root];
        while let Some(entity) = stack.pop() {
            if let Ok(ch) = children.get(entity) {
                stack.extend(ch.iter());
            }
            let named = names
                .get(entity)
                .map(|n| n.as_str().to_lowercase())
                .unwrap_or_default();
            if named.contains("collision") || named.contains("clip") {
                if let Ok(mut v) = vis.get_mut(entity) {
                    *v = Visibility::Hidden;
                }
            }
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
    slash: u8,
}
impl SwingPose {
    pub fn strike(&mut self, now: f32) {
        if self.started.is_some() {
            self.slash = self.slash.wrapping_add(1);
        }
        self.started = Some(now);
    }
    fn slash_arc(&self) -> SlashArc {
        slash_arc_from(self.slash)
    }
    fn combo(&self) -> u8 {
        self.slash
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
    (Vec3::Y * (ba_protocol::avatar_local_y() + hop), spin)
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
    profile: Option<Res<super::CharacterProfile>>,
    equip: Option<Res<EquipState>>,
    mut roots: Query<
        (
            Entity,
            &ChildOf,
            &GlobalTransform,
            &mut Transform,
            &mut AvatarMotion,
            Option<&PlayerAvatarMesh>,
        ),
        (Without<AvatarJoint>, Without<HeldWeaponMesh>),
    >,
    actors: Query<
        &Transform,
        (
            Without<AvatarMotion>,
            Without<AvatarJoint>,
            Without<HeldWeaponMesh>,
        ),
    >,
    world: Query<&GlobalTransform>,
    mut joints: Query<
        (Entity, &AvatarJoint, &ChildOf, &GlobalTransform, &mut Transform),
        (Without<AvatarMotion>, Without<HeldWeaponMesh>),
    >,
    mut swords: Query<
        (&ChildOf, &mut Transform, Has<HeldDarkmodSword>),
        (With<HeldWeaponMesh>, Without<AvatarMotion>, Without<AvatarJoint>),
    >,
) {
    let dt = time.delta_secs().max(0.0001);
    let mode = equip.as_deref().map(|e| e.mode).unwrap_or_else(|| {
        profile
            .as_deref()
            .and_then(|p| p.draft.as_ref())
            .map(|d| EquipMode::from_held(d.held_weapon()))
            .unwrap_or(EquipMode::Unarmed)
    });
    let two_hand = mode.two_handed();
    let pole = mode == EquipMode::PoleArm;
    let two_kind = held_mesh_kind(mode).unwrap_or(HeldMeshKind::LongSword);
    let mut frames = HashMap::new();
    for (entity, parent, global, mut visual, mut state, local) in &mut roots {
        let Ok(actor) = actors.get(parent.parent()) else {
            continue;
        };
        if state.model_yaw.is_none() {
            let mut shoulders = [None, None];
            for (_, joint, _, tf, _) in joints.iter() {
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
        state.rest_support = tick_rest_support(state.rest_support, state.resting, dt);
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
                state.grip_right,
                state.grip_left,
                state.level_head,
                state.jumping,
                state.jump_phase,
                state.rest_support,
            ),
        );
    }
    let sword_parents: HashSet<Entity> = swords.iter().map(|(parent, _, _)| parent.parent()).collect();
    let armed_roots: HashSet<Entity> = joints
        .iter()
        .filter_map(|(entity, joint, _, _, _)| {
            sword_parents.contains(&entity).then_some(joint.root)
        })
        .collect();
    let dual = swords.iter().count() >= 2 || mode.left_weapon();
    let armed = !armed_roots.is_empty() || mode.right_weapon();
    for (_entity, joint, parent, _, mut tf) in &mut joints {
        let Some(&(
            frame,
            phase,
            amp,
            first,
            local,
            authored_walk,
            fist,
            grip_right,
            grip_left,
            level_head,
            jumping,
            jump_phase,
            rest_support,
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
        let cut_t = swing_clock(&swing, time.elapsed_secs());
        let arc = two_hand_arc(&swing);
        // First person still poses tagged joints; aiming hips at −Z folded
        // the whole mesh. Idle keeps the clip; a 2H cut yaws that pelvis.
        if joint.kind == JointKind::Hips {
            if two_hand && cut_t < 1.0 {
                tf.rotation = tf.rotation * two_hand_cut_hip(cut_t, arc);
            }
            continue;
        }
        // Do not re-aim thighs in world space after the hip yaws — that
        // adducts idle A-stance into a narrow stand.
        if two_hand
            && cut_t < 1.0
            && matches!(joint.kind, JointKind::UpperLeg | JointKind::LowerLeg)
        {
            continue;
        }
        let head_joint = matches!(joint.kind, JointKind::Head | JointKind::Neck);
        let armed_here = armed_roots.contains(&joint.root) || (armed && local);
        // Armed 1H: do not use the unarmed FP reach. Left arm stays on the
        // idle clip; the right arm is posed by the sword chain below.
        let first = first
            && !(armed_here
                && matches!(
                    joint.kind,
                    JointKind::Shoulder
                        | JointKind::UpperArm
                        | JointKind::LowerArm
                        | JointKind::Hand
                ));
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
            if first && fist {
                let n = if joint.axis.x.abs() < 0.9 {
                    Vec3::X
                } else {
                    Vec3::Y
                };
                let flex_axis = joint.axis.cross(n).normalize_or_zero();
                let flex = Quat::from_axis_angle(flex_axis, 0.18);
                let roll = Quat::from_axis_angle(joint.axis, side * 0.22);
                tf.rotation = joint.rest * roll * flex;
                continue;
            }
            tf.rotation = joint.rest;
            continue;
        }
        // Unarmed FP idle: clavicle stays at bind so the shoulders are not
        // rolled forward. Upper arm hangs; forearm gets the slight elbow.
        if first && !fist && !armed_here && joint.kind == JointKind::Shoulder {
            tf.rotation = joint.rest;
            continue;
        }
        if joint.kind == JointKind::Finger {
            let pose = if rest_support > 0.45 {
                if joint.sign > 0.0 {
                    Some(FingerPose::Relaxed)
                } else {
                    None
                }
            } else if fist {
                Some(FingerPose::Fist)
            } else if (joint.sign < 0.0 && (grip_right || armed))
                || (joint.sign > 0.0 && (grip_left || dual || two_hand))
            {
                Some(FingerPose::Grip)
            } else if authored_walk {
                None
            } else {
                tf.rotation = joint.rest;
                continue;
            };
            if let Some(pose) = pose {
                tf.rotation = pose_finger(joint, pose);
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
            JointKind::UpperArm if first && !fist => Vec3::new(
                side * 0.22,
                -0.96 + breath,
                0.04,
            ),
            JointKind::LowerArm if first && !fist => Vec3::new(
                side * 0.16,
                -0.82,
                -0.32,
            ),
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
            JointKind::Hand
            | JointKind::Finger
            | JointKind::Head
            | JointKind::Neck
            | JointKind::Hips
            | JointKind::Shoulder => Vec3::NEG_Z,
        };
        let goal = parent_world.rotation().inverse() * frame * direction.normalize();
        let mut rot = Quat::from_rotation_arc(joint.rest * joint.axis, goal) * joint.rest;
        if first && fist && joint.kind == JointKind::LowerArm {
            rot *= Quat::from_axis_angle(joint.axis, side * 0.45);
        }
        tf.rotation = rot;
    }
    // Pose each armed arm chain in parent order. Dual wield runs both sides
    // independently so left carry does not eat the right from_chest. Great
    // weapons share one grip: left chain reaches the same handle, lower.
    // Chest goals convert into each bone's parent frame so a hanging upper
    // arm does not turn forearm +Z into world up (blade through the skull).
    let mut links: Vec<(Entity, i8, u8, Entity)> = Vec::new();
    for (entity, joint, _, _, _) in joints.iter() {
        let Some(rank) = sword_chain_rank(joint.kind) else {
            continue;
        };
        if joint.sign >= 0.0 && !dual && !two_hand {
            continue;
        }
        let Some(&(_, _, _, _, local, _, _, _, _, _, _, _, _)) = frames.get(&joint.root) else {
            continue;
        };
        if !(armed_roots.contains(&joint.root) || (armed && local)) {
            continue;
        }
        let side = if joint.sign < 0.0 { 0i8 } else { 1 };
        links.push((joint.root, side, rank, entity));
    }
    links.sort_unstable_by_key(|&(root, side, rank, _)| (root, side, rank));
    let bind_pos: HashMap<Entity, Vec3> = joints
        .iter()
        .map(|(entity, _, _, _, tf)| (entity, tf.translation))
        .collect();
    let joint_of: HashMap<Entity, AvatarJoint> = joints
        .iter()
        .map(|(entity, joint, _, _, _)| (entity, *joint))
        .collect();
    let now = time.elapsed_secs();
    let mut from_chest = Quat::IDENTITY;
    let mut chain_pos = Vec3::ZERO;
    let mut current_root = Entity::PLACEHOLDER;
    let mut current_side = i8::MIN;
    let mut left_pending: HashMap<Entity, Quat> = HashMap::new();
    let mut right_grip: Option<(Vec3, Quat, Vec3, Vec3)> = None;
    for (root, side, _, entity) in links.iter().copied() {
        if root != current_root || side != current_side {
            from_chest = Quat::IDENTITY;
            chain_pos = Vec3::ZERO;
            current_root = root;
            current_side = side;
        }
        let local_pos = bind_pos.get(&entity).copied().unwrap_or(Vec3::ZERO);
        let origin = chain_pos + from_chest * local_pos;
        let mut sword_seat = None;
        let t = swing_clock(&swing, now);
        let cutting = two_hand && t < 1.0;
        let arc = two_hand_arc(&swing);
        {
            let Ok((_, joint, _, _, mut tf)) = joints.get_mut(entity) else {
                continue;
            };
            let authored = frames.get(&root).is_some_and(|f| {
                if joint.sign < 0.0 {
                    f.7
                } else {
                    f.8
                }
            });
            if authored {
                // Tide owns the swinging arm. Accumulate so dual-wield can yaw
                // the tip toward the foe without dropping a high chamber.
                from_chest *= tf.rotation;
                if joint.kind == JointKind::Hand {
                    let blade = if dual {
                        dual_blade_toward_foe(joint.axis, from_chest)
                    } else {
                        joint.axis
                    };
                    sword_seat = Some((joint.axis, blade));
                }
            } else if let Some(&pending) = left_pending.get(&entity) {
                let clip = tf.rotation;
                let k = frames
                    .get(&root)
                    .map(|f| rest_support_blend(f.12))
                    .unwrap_or(0.);
                tf.rotation = pending.slerp(clip, k);
                from_chest *= tf.rotation;
                if two_hand && side == 0 && joint.kind == JointKind::Hand {
                    if let Some(pose) = two_hand_sword_loc(cutting, t, arc, pole) {
                        let blade_chest =
                            (grip_vec(pose.tip) - grip_vec(pose.handle)).normalize_or(Vec3::Y);
                        let blade_local = from_chest.inverse() * blade_chest;
                        sword_seat = Some((joint.axis, blade_local));
                    }
                }
            } else {
                let clip = tf.rotation;
                let k = frames
                    .get(&root)
                    .map(|f| rest_support_blend(f.12))
                    .unwrap_or(0.);
                let t = swing_clock(&swing, now);
                let this_attacks = t < 1.0
                    && if dual {
                        (swing.combo() % 2 == 1) == (joint.sign > 0.0)
                    } else {
                        true
                    };
                if joint.kind == JointKind::Shoulder && two_hand {
                    let posed = if cutting {
                        two_hand_cut_shoulder(joint, t, arc)
                    } else {
                        joint.rest
                    };
                    tf.rotation = posed.slerp(clip, k);
                    from_chest *= tf.rotation;
                } else if joint.kind == JointKind::Shoulder && !this_attacks {
                    tf.rotation = joint.rest.slerp(clip, k);
                    from_chest *= tf.rotation;
                } else if two_hand
                    && side == 0
                    && joint.kind == JointKind::UpperArm
                    && two_hand_sword_loc(cutting, t, arc, pole).is_some()
                {
                    let upper_j = *joint;
                    let solved = two_hand_sword_loc(cutting, t, arc, pole).and_then(|pose| {
                        let lower_e = chain_entity(&links, root, side, 2)?;
                        let hand_e = chain_entity(&links, root, side, 3)?;
                        let lower_j = *joint_of.get(&lower_e)?;
                        let hand_j = *joint_of.get(&hand_e)?;
                        let forearm_off = *bind_pos.get(&lower_e)?;
                        let hand_off = *bind_pos.get(&hand_e)?;
                        let target = grip_vec(pose.handle);
                        let blade_chest =
                            (grip_vec(pose.tip) - target).normalize_or(Vec3::Y);
                        let (upper_rot, lower_rot, hand_rot) = right_two_hand_swing_rots(
                            &upper_j,
                            &lower_j,
                            &hand_j,
                            forearm_off,
                            hand_off,
                            from_chest,
                            origin,
                            target,
                            blade_chest,
                            two_kind,
                        );
                        Some((upper_rot, lower_e, lower_rot, hand_e, hand_rot))
                    });
                    if let Some((upper_rot, lower_e, lower_rot, hand_e, hand_rot)) = solved {
                        tf.rotation = upper_rot.slerp(clip, k);
                        left_pending.insert(lower_e, lower_rot);
                        left_pending.insert(hand_e, hand_rot);
                        from_chest *= tf.rotation;
                    } else {
                        tf.rotation = joint.rest.slerp(clip, k);
                        from_chest *= tf.rotation;
                    }
                } else if two_hand && side == 1 && joint.kind == JointKind::UpperArm {
                    let upper_j = *joint;
                    let solved = right_grip.and_then(|(rh_pos, rh_rot, along, blade)| {
                        let lower_e = chain_entity(&links, root, side, 2)?;
                        let hand_e = chain_entity(&links, root, side, 3)?;
                        let lower_j = *joint_of.get(&lower_e)?;
                        let hand_j = *joint_of.get(&hand_e)?;
                        let forearm_off = *bind_pos.get(&lower_e)?;
                        let hand_off = *bind_pos.get(&hand_e)?;
                        let target = two_hand_left_target(rh_pos, rh_rot, along, blade, two_kind);
                        let weapon = held_weapon_tf(along, blade, two_kind);
                        let blade_chest = rh_rot * weapon.rotation * Vec3::NEG_Z;
                        let (upper_rot, lower_rot, hand_rot) = left_two_hand_rots(
                            &upper_j,
                            &lower_j,
                            &hand_j,
                            forearm_off,
                            hand_off,
                            from_chest,
                            origin,
                            target,
                            blade_chest,
                            two_kind,
                        );
                        Some((upper_rot, lower_e, lower_rot, hand_e, hand_rot))
                    });
                    if let Some((upper_rot, lower_e, lower_rot, hand_e, hand_rot)) = solved {
                        tf.rotation = upper_rot.slerp(clip, k);
                        left_pending.insert(lower_e, lower_rot);
                        left_pending.insert(hand_e, hand_rot);
                        from_chest *= tf.rotation;
                    } else {
                        tf.rotation = joint.rest.slerp(clip, k);
                        from_chest *= tf.rotation;
                    }
                } else {
                    let chest_goal = if two_hand {
                        great_weapon_goal(
                            joint.kind,
                            joint.sign,
                            this_attacks,
                            t,
                            swing.slash_arc(),
                            two_kind,
                        )
                    } else if this_attacks {
                        chest_goal_for_side(
                            sword_slash_goal(joint.kind, t, swing.slash_arc()),
                            joint.sign,
                        )
                    } else {
                        sword_carry_goal_side(joint.kind, joint.sign)
                    };
                    let mut rot = aim_bone(joint, from_chest.inverse() * chest_goal);
                    if joint.kind == JointKind::Hand {
                        let blade = (from_chest * rot).inverse() * Vec3::Y;
                        let roll = sword_wrist_roll(joint.axis, blade);
                        rot *= roll;
                        if two_hand && side == 0 {
                            rot *= Quat::from_axis_angle(
                                joint.axis,
                                grip_for(two_kind).right_wrist_deg.to_radians(),
                            );
                        }
                    }
                    tf.rotation = rot.slerp(clip, k);
                    from_chest *= tf.rotation;
                    if joint.kind == JointKind::Hand {
                        let blade = from_chest.inverse() * Vec3::Y;
                        let blade = if dual {
                            dual_blade_toward_foe(blade, from_chest)
                        } else {
                            blade
                        };
                        sword_seat = Some((joint.axis, blade));
                    }
                }
            }
            if two_hand && side == 0 && joint.kind == JointKind::Hand {
                if let Some((along, blade)) = sword_seat {
                    right_grip = Some((origin, from_chest, along, blade));
                }
            }
        }
        chain_pos = origin;
        if let Some((along, blade)) = sword_seat {
            let kind = held_mesh_kind(mode).unwrap_or(HeldMeshKind::ShortSword);
            for (parent, mut st, darkmod) in &mut swords {
                if parent.parent() == entity {
                    *st = if darkmod {
                        held_weapon_tf(along, blade, kind)
                    } else {
                        held_sword_grip(along, blade)
                    };
                }
            }
        }
    }
    // Sit: Tide keeps the body. Left hand plants behind so the rest has a
    // support arm instead of a stowed-sword carry.
    let mut support: Vec<(Entity, u8, Entity)> = Vec::new();
    for (entity, joint, _, _, _) in joints.iter() {
        if joint.sign <= 0.0 {
            continue;
        }
        let Some(rank) = sword_chain_rank(joint.kind) else {
            continue;
        };
        if !frames.get(&joint.root).is_some_and(|f| f.12 > 0.001) {
            continue;
        }
        support.push((joint.root, rank, entity));
    }
    support.sort_unstable_by_key(|&(root, rank, _)| (root, rank));
    let mut from_chest = Quat::IDENTITY;
    let mut current_root = Entity::PLACEHOLDER;
    for (root, _, entity) in support {
        if root != current_root {
            from_chest = Quat::IDENTITY;
            current_root = root;
        }
        let k = frames
            .get(&root)
            .map(|f| rest_support_blend(f.12))
            .unwrap_or(0.);
        let Ok((_, joint, _, _, mut tf)) = joints.get_mut(entity) else {
            continue;
        };
        let planted = aim_bone(joint, from_chest.inverse() * rest_support_goal(joint.kind));
        let rot = tf.rotation.slerp(planted, k);
        tf.rotation = rot;
        from_chest *= rot;
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
        assert_eq!(joint_from_name("hips"), Some((JointKind::Hips, 0.0)));
        assert_eq!(joint_from_name("J_Bip_C_Hips"), Some((JointKind::Hips, 0.0)));
        assert!(levels_head(MotionClip::Idle));
        assert!(levels_head(MotionClip::Rest));
        assert!(levels_head(MotionClip::Slash1H));
        assert!(levels_head(MotionClip::Slash1HReturn));
        assert!(levels_head(MotionClip::Slash1HSweep));
        assert!(levels_head(MotionClip::SlashDw));
        assert!(levels_head(MotionClip::SlashDwLeft));
        assert!(levels_head(MotionClip::SlashDwReturn));
        assert!(levels_head(MotionClip::SlashDwLeftReturn));
        assert!(levels_head(MotionClip::Slash2H));
        assert!(levels_head(MotionClip::WalkSlash2H));
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
        assert_eq!(
            joint_from_name("rightIndexProximal"),
            Some((JointKind::Finger, -1.0)),
            "VRM proximal must not be dropped because the word ends in L"
        );
        assert_eq!(
            joint_from_name("rightIndexDistal"),
            Some((JointKind::Finger, -1.0))
        );
        assert_eq!(
            joint_from_name("leftIndexProximal"),
            Some((JointKind::Finger, 1.0))
        );
        assert_eq!(
            joint_from_name("rightThumbMetacarpal"),
            Some((JointKind::Finger, -1.0))
        );
        assert_eq!(
            joint_from_name("rightShoulder"),
            Some((JointKind::Shoulder, -1.0))
        );
        assert_eq!(
            joint_from_name("leftShoulder"),
            Some((JointKind::Shoulder, 1.0))
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
    fn held_sword_follows_the_vrm_finger_axis() {
        let along = Vec3::new(-0.97, -0.03, 0.23).normalize();
        let grip = held_sword_grip(along, Vec3::Y);
        let blade = grip.rotation * Vec3::Y;
        let handle = grip.translation + grip.rotation * Vec3::new(0.0, 0.05, 0.0);
        assert!(
            handle.dot(along) > 0.04,
            "handle must sit along the fingers, got {handle}"
        );
        assert!(
            handle.y < -0.015,
            "handle must sit in the VRM palm hollow (-Y), got {handle}"
        );
        assert!(
            blade.dot(Vec3::Y) > 0.95,
            "blade must stand up from the fist on hand-local +Y, got {blade:?}"
        );
    }

    #[test]
    fn darkmod_shortsword_maps_neg_z_onto_the_blade() {
        let along = Vec3::new(-0.97, -0.03, 0.23).normalize();
        if darkmod_shortsword_on_disk() {
            let slash = held_weapon_tf(along, along, HeldMeshKind::ShortSword);
            let slash_dir = slash.rotation * Vec3::NEG_Z;
            assert!(
                slash_dir.dot(along) > 0.95,
                "slash seating must follow the fingers, got {slash_dir:?} vs {along:?}"
            );
            let idle = held_weapon_tf(along, Vec3::Y, HeldMeshKind::ShortSword);
            let idle_dir = idle.rotation * Vec3::NEG_Z;
            assert!(
                idle_dir.dot(Vec3::Y) > 0.95,
                "idle seating must stand the blade up from the hip, got {idle_dir:?}"
            );
            let thin = idle.rotation * Vec3::Y;
            assert!(
                thin.dot(Vec3::X).abs() > 0.65,
                "hilt roll must keep the thin edge sideways in both cameras, got {thin:?}"
            );
            assert!(
                (slash.scale.x - DARKMOD_SHORTSWORD_SCALE).abs() < 0.001,
                "Dark Mod inches must shrink to metres, got {}",
                slash.scale.x
            );
            let great = held_weapon_tf(along, Vec3::Y, HeldMeshKind::LongSword);
            assert!(
                great.scale.x > slash.scale.x + 0.005,
                "great weapon must be longer than a shortsword, got {} vs {}",
                great.scale.x,
                slash.scale.x
            );
        } else {
            let blade = held_weapon_tf(along, Vec3::Y, HeldMeshKind::ShortSword).rotation * Vec3::Y;
            assert!(blade.dot(Vec3::Y) > 0.95);
        }
    }

    #[test]
    fn sword_carry_aims_the_vrm_right_arm_forward_of_the_ribs() {
        let rest = Quat::IDENTITY;
        let upper = AvatarJoint {
            root: Entity::PLACEHOLDER,
            rest,
            axis: Vec3::NEG_X,
            kind: JointKind::UpperArm,
            sign: -1.0,
            curl: 0.,
            oppose: 0.,
            digit: 0,
            chain: 0,
        };
        let aimed = sword_carry_rotation(&upper) * upper.axis;
        assert!(
            rest.angle_between(sword_carry_rotation(&upper)) > 0.8,
            "carry must leave T-pose, got {}",
            rest.angle_between(sword_carry_rotation(&upper))
        );
        assert!(
            aimed.z > 0.18,
            "right upper arm must still reach forward of the hip, got {aimed:?}"
        );
        assert!(
            aimed.y < -0.55,
            "idle carry must hang below the head, got {aimed:?}"
        );
        assert!(
            aimed.x < -0.20,
            "idle elbow must sit out beside the hip, not tucked into the ribs, got {aimed:?}"
        );
        let hand = AvatarJoint {
            kind: JointKind::Hand,
            ..upper
        };
        let blade = sword_carry_rotation(&hand) * hand.axis;
        assert!(
            blade.y < -0.18 && blade.z > 0.70,
            "idle wrist must stay in line with the forearm, blade away from the skull, got {blade:?}"
        );
    }

    fn test_joint(kind: JointKind, sign: f32, axis: Vec3) -> AvatarJoint {
        AvatarJoint {
            root: Entity::PLACEHOLDER,
            rest: Quat::IDENTITY,
            axis,
            kind,
            sign,
            curl: 0.,
            oppose: 0.,
            digit: 0,
            chain: 0,
        }
    }

    #[test]
    fn great_weapon_handle_sits_in_the_right_palm() {
        let along = Vec3::NEG_X;
        let g = TwoHandGrip::default();
        let tf = held_weapon_tf(along, Vec3::Y, HeldMeshKind::LongSword);
        let side = along.cross(Vec3::NEG_Y).normalize_or(Vec3::Z);
        let palm = along * g.palm_along + Vec3::NEG_Y * g.palm_in + side * g.palm_side;
        let grip = tf.translation + tf.rotation * Vec3::Z * g.grip_pommel;
        assert!(
            grip.distance(palm) < 0.02,
            "longsword grip must occupy the palm, grip {grip} palm {palm}"
        );
        assert!(
            tf.translation.dot(along) < 0.02,
            "guard must not sit out at the fingertips, got {}",
            tf.translation.dot(along)
        );
        let short = held_weapon_tf(along, Vec3::Y, HeldMeshKind::ShortSword);
        assert!(
            tf.translation.length() + 0.02 < short.translation.length()
                || tf.translation.dot(along) < short.translation.dot(along),
            "2H seating is closer to the wrist than 1H, 2h {} 1h {}",
            tf.translation,
            short.translation
        );
    }

    #[test]
    fn two_bone_ik_puts_the_left_palm_on_the_handle() {
        let upper = test_joint(JointKind::UpperArm, 1.0, Vec3::X);
        let lower = test_joint(JointKind::LowerArm, 1.0, Vec3::X);
        let forearm = Vec3::X * 0.28;
        let hand = Vec3::X * 0.25;
        let origin = Vec3::new(0.18, 0.05, 0.02);
        let along = Vec3::NEG_X;
        let right_hand = Vec3::new(-0.12, -0.18, 0.30);
        let right_rot = Quat::IDENTITY;
        let target = two_hand_left_target(right_hand, right_rot, along, Vec3::Y, HeldMeshKind::LongSword);
        let pole = Vec3::new(0.64, -0.16, 0.22);
        let (u, l) = two_bone_ik(
            &upper,
            &lower,
            forearm,
            hand,
            Quat::IDENTITY,
            origin,
            target,
            pole,
        );
        let reached = ik_hand_pos(u, l, Quat::IDENTITY, origin, forearm, hand);
        assert!(
            reached.distance(target) < 0.04,
            "left palm must land on the handle, reached {reached} target {target} err {}",
            reached.distance(target)
        );
        let elbow = origin + u * forearm;
        assert!(
            elbow.x > 0.02,
            "left elbow stays on the left while reaching across, elbow {elbow}"
        );
        assert!(
            reached.distance(right_hand) > 0.04,
            "left palm is not occupying the right fist, reached {reached} right {right_hand}"
        );
    }

    #[test]
    fn great_weapon_left_target_is_on_the_same_handle() {
        let along = Vec3::NEG_X;
        let right_pos = Vec3::new(-0.10, -0.16, 0.32);
        let left = two_hand_left_target(right_pos, Quat::IDENTITY, along, Vec3::Y, HeldMeshKind::LongSword);
        let weapon = held_weapon_tf(along, Vec3::Y, HeldMeshKind::LongSword);
        let right_grip = right_pos + weapon.translation;
        let span = left.distance(right_grip);
        assert!(
            (0.05..0.22).contains(&span),
            "hands stack a palm-width apart on the grip, got {span}"
        );
        let mirrored = sword_carry_goal_side(JointKind::Hand, 1.0);
        assert!(
            mirrored.x > 0.0,
            "1H off-hand is still a mirrored hip carry, got {mirrored:?}"
        );
        assert!(
            (TwoHandGrip::default().left_wrist_deg + 105.0).abs() < 0.01,
            "default left 2H wrist twist is -105 degrees"
        );
        assert!(
            (TwoHandGrip::default().right_wrist_deg - 50.0).abs() < 0.01,
            "default right 2H wrist twist is 50 degrees"
        );
    }

    #[test]
    fn two_hand_cut_passes_through_straight_out_plant() {
        let wind = two_hand_cut_at(0.0, SlashArc::Diagonal);
        let prep = two_hand_cut_at(0.20, SlashArc::Diagonal);
        let plant = two_hand_cut_at(0.42, SlashArc::Diagonal);
        let follow = two_hand_cut_at(0.68, SlashArc::Diagonal);
        let recover = two_hand_cut_at(1.0, SlashArc::Diagonal);
        let wind_tip = grip_vec(wind.tip);
        let prep_tip = grip_vec(prep.tip);
        let plant_tip = grip_vec(plant.tip);
        let follow_handle = grip_vec(follow.handle);
        let follow_tip = grip_vec(follow.tip);
        assert!(
            wind_tip.y > grip_vec(wind.handle).y + 0.40,
            "opens from a high-right chamber, wind tip {wind_tip:?}"
        );
        assert!(
            (prep_tip - wind_tip).length() < (prep_tip - plant_tip).length(),
            "early frames stay in the small coil, prep tip {prep_tip:?}"
        );
        assert!(
            (plant_tip - grip_vec(default_plant_loc().tip)).length() < 0.01,
            "contact is the locked straight-out plant, got {plant_tip:?}"
        );
        assert!(
            plant_tip.z > wind_tip.z + 0.60,
            "blade must swing forward through the plant, wind z {} plant z {}",
            wind_tip.z,
            plant_tip.z
        );
        assert!(
            follow_handle.x > 0.0 && follow_handle.y < grip_vec(plant.handle).y - 0.08,
            "follow-through is a low left guard, handle {follow_handle:?}"
        );
        assert!(
            follow_tip.y < plant_tip.y - 0.20
                && follow_tip.y > -0.45
                && follow_tip.z > 0.25,
            "diagonal finishes down-left in front, not in the dirt, tip {follow_tip:?}"
        );
        assert!(
            grip_vec(recover.tip).y > grip_vec(recover.handle).y + 0.40,
            "recovers with the blade up at idle wrap, recover {:?}",
            recover.tip
        );
        let rest = Quat::IDENTITY;
        let shoulder = AvatarJoint {
            root: Entity::PLACEHOLDER,
            rest,
            axis: Vec3::NEG_X,
            kind: JointKind::Shoulder,
            sign: -1.0,
            curl: 0.,
            oppose: 0.,
            digit: 0,
            chain: 0,
        };
        let coil = two_hand_cut_shoulder(&shoulder, 0.0, SlashArc::Diagonal);
        let strike = two_hand_cut_shoulder(&shoulder, 0.42, SlashArc::Diagonal);
        assert!(
            rest.angle_between(coil) > 0.08,
            "right clavicle coils before the cut"
        );
        assert!(
            coil.angle_between(strike) > 0.12,
            "shoulders open through the strike instead of a locked torso"
        );
        let hip_coil = two_hand_cut_hip(0.0, SlashArc::Diagonal).to_euler(EulerRot::YXZ).0;
        let hip_strike = two_hand_cut_hip(0.42, SlashArc::Diagonal).to_euler(EulerRot::YXZ).0;
        assert!(
            hip_coil < -0.16 && hip_strike > 0.30,
            "hips pivot into the cut, coil {hip_coil} strike {hip_strike}"
        );
        assert!(two_hand_keeps_loco_clip(crate::character::HeldWeapon::GreatWeapon));
        assert!(two_hand_keeps_loco_clip(crate::character::HeldWeapon::PoleArm));
        assert!(!two_hand_keeps_loco_clip(crate::character::HeldWeapon::OneHand));
        assert_eq!(joint_from_name("hips"), Some((JointKind::Hips, 0.0)));
        assert_eq!(joint_from_name("J_Bip_C_Hips"), Some((JointKind::Hips, 0.0)));
        let ret_wind = two_hand_cut_at(0.0, SlashArc::Return);
        let ret_plant = two_hand_cut_at(0.42, SlashArc::Return);
        let ret_follow = two_hand_cut_at(0.68, SlashArc::Return);
        assert!(
            grip_vec(ret_wind.tip).x > 0.10
                && grip_vec(ret_wind.tip).y > grip_vec(ret_wind.handle).y + 0.40
                && grip_vec(ret_wind.tip).z < grip_vec(ret_wind.handle).z + 0.05,
            "return chambers beside the left shoulder, blade back and up, wind {:?}",
            ret_wind.tip
        );
        assert!(
            (grip_vec(ret_plant.tip) - grip_vec(default_return_plant_loc().tip)).length() < 0.01,
            "return contact is the mirrored straight-out plant, got {:?}",
            ret_plant.tip
        );
        assert!(
            grip_vec(ret_follow.handle).x < 0.0
                && grip_vec(ret_follow.tip).y < grip_vec(ret_plant.tip).y - 0.20,
            "return follow-through is a low right guard, handle {:?} tip {:?}",
            ret_follow.handle,
            ret_follow.tip
        );
        let mut combo = SwingPose::default();
        combo.strike(0.0);
        assert_eq!(two_hand_arc(&combo), SlashArc::Diagonal);
        combo.strike(0.5);
        assert_eq!(two_hand_arc(&combo), SlashArc::Return);
        assert_eq!(swing_overlay_label(), None);
        let baked_return = two_hand_cut_at(0.0, SlashArc::Return);
        assert_eq!(baked_return.handle, default_return_wind_loc().handle);
        assert_eq!(baked_return.tip, default_return_wind_loc().tip);
    }

    #[test]
    fn pole_arm_uses_the_quarterstaff_mesh() {
        assert_eq!(
            held_mesh_kind(EquipMode::PoleArm),
            Some(HeldMeshKind::Staff)
        );
        assert_eq!(
            held_mesh_kind(EquipMode::GreatWeapon),
            Some(HeldMeshKind::LongSword)
        );
        assert_eq!(held_mesh_scale(HeldMeshKind::Staff), DARKMOD_STAFF_SCALE);
        let g = default_staff_grip();
        let cast = default_staff_cast_grip();
        assert!(
            g.right_hand[1] < cast.right_hand[1] - 0.40,
            "staff idle hands sit lower than the spell-cast hold, idle {} cast {}",
            g.right_hand[1],
            cast.right_hand[1]
        );
        assert!(
            cast.left_pommel.abs() > TwoHandGrip::default().left_pommel + 0.08,
            "cast hands sit farther apart on the shaft, left_pommel {}",
            cast.left_pommel
        );
        assert!(
            g.grip_pommel > 0.4,
            "staff origin must slide so the shaft center sits in the palm, got {}",
            g.grip_pommel
        );
        if darkmod_weapon_on_disk(DARKMOD_STAFF) {
            let staff = held_weapon_tf(Vec3::NEG_X, Vec3::Y, HeldMeshKind::Staff);
            assert!((staff.scale.x - DARKMOD_STAFF_SCALE).abs() < 0.001);
            let tip = staff.rotation * Vec3::NEG_Z;
            assert!(
                tip.dot(Vec3::Y) > 0.95,
                "idle staff must stand the shaft up, got {tip:?}"
            );
        }
    }

    #[test]
    fn staff_idle_is_hip_held_and_cast_keeps_the_high_front() {
        let idle = default_staff_idle_loc();
        let cast = default_staff_cast_loc();
        assert!(
            idle.handle[1] < -0.18 && idle.handle[1] < cast.handle[1] - 0.12,
            "staff idle handle sits at the hip, idle {:?} cast {:?}",
            idle.handle,
            cast.handle
        );
        assert!(
            cast.tip[1] > idle.tip[1] + 0.25,
            "spell-cast keeps the high-front shaft, idle tip {:?} cast tip {:?}",
            idle.tip,
            cast.tip
        );
        let planted = two_hand_sword_loc(false, 1.0, SlashArc::Diagonal, true).unwrap();
        assert_eq!(planted.handle, idle.handle);
        assert_eq!(planted.tip, idle.tip);
    }

    #[test]
    fn staff_cut_is_an_overhead_smash_not_the_greatsword_diagonal() {
        let plant = two_hand_cut_at_for(0.42, SlashArc::Diagonal, true);
        assert!(
            (grip_vec(plant.tip) - grip_vec(default_staff_plant_loc().tip)).length() < 0.01,
            "staff contact is the straight-out plant, got {:?}",
            plant.tip
        );
        let wind = two_hand_cut_at_for(0.0, SlashArc::Diagonal, true);
        assert!(
            grip_vec(wind.tip).y > grip_vec(wind.handle).y + 0.70
                && grip_vec(wind.tip).z < grip_vec(wind.handle).z,
            "staff chambers high and back, wind {:?}",
            wind.tip
        );
        let great = two_hand_cut_at(0.0, SlashArc::Diagonal);
        assert!(
            grip_vec(wind.tip).distance(grip_vec(great.tip)) > 0.20,
            "staff wind must not reuse the baked greatsword chamber"
        );
        let follow = two_hand_cut_at_for(0.68, SlashArc::Diagonal, true);
        assert!(
            grip_vec(follow.tip).y < -0.50,
            "staff follow-through smashes down, tip {:?}",
            follow.tip
        );
    }

    #[test]
    fn sword_slash_stays_in_front_of_the_ribs() {
        let rest = Quat::IDENTITY;
        let hand = AvatarJoint {
            root: Entity::PLACEHOLDER,
            rest,
            axis: Vec3::NEG_X,
            kind: JointKind::Hand,
            sign: -1.0,
            curl: 0.,
            oppose: 0.,
            digit: 0,
            chain: 0,
        };
        for t in [0.0, 0.35, 0.7, 1.0] {
            let aimed = sword_slash_rotation(&hand, t) * hand.axis;
            let min_z = if t < 0.2 {
                0.32
            } else if t > 0.85 {
                0.50
            } else {
                0.70
            };
            assert!(
                aimed.z > min_z,
                "slash at {t} must stay in front of the hip, got {aimed:?}"
            );
            assert!(
                aimed.y < 0.05,
                "slash at {t} must not raise the blade through the skull, got {aimed:?}"
            );
        }
        let start = sword_slash_rotation(&hand, 0.0) * hand.axis;
        let end = sword_slash_rotation(&hand, 1.0) * hand.axis;
        assert!(
            end.x > start.x + 0.35,
            "cut must travel from a right-side wind-up across to the left, start {start:?} end {end:?}"
        );
        let upper = AvatarJoint {
            kind: JointKind::UpperArm,
            ..hand
        };
        let lower = AvatarJoint {
            kind: JointKind::LowerArm,
            ..hand
        };
        let u = sword_slash_rotation(&upper, 0.45) * upper.axis;
        let l = sword_slash_rotation(&lower, 0.45) * lower.axis;
        assert!(
            u.dot(l) > 0.90,
            "contact must straighten the elbow so the cut reaches out, upper {u:?} lower {l:?}"
        );
        assert!(
            l.z > 0.90,
            "extended forearm must reach forward, got {l:?}"
        );
        let mut swing = SwingPose::default();
        swing.strike(1.0);
        assert_eq!(swing.slash_arc(), SlashArc::Diagonal);
        swing.strike(1.5);
        assert_eq!(swing.slash_arc(), SlashArc::Return);
        swing.strike(2.0);
        assert_eq!(swing.slash_arc(), SlashArc::Sweep);
        swing.strike(2.5);
        assert_eq!(swing.slash_arc(), SlashArc::Diagonal);
        for arc in [SlashArc::Return, SlashArc::Sweep] {
            for t in [0.0, 0.35, 0.7, 1.0] {
                let aimed = sword_slash_rotation_arc(&hand, t, arc) * hand.axis;
                let min_z = if t < 0.2 {
                    0.32
                } else if t > 0.85 {
                    0.50
                } else {
                    0.70
                };
                assert!(
                    aimed.z > min_z,
                    "{arc:?} slash at {t} must stay in front of the hip, got {aimed:?}"
                );
                assert!(
                    aimed.y < 0.05,
                    "{arc:?} slash at {t} must not raise the blade through the skull, got {aimed:?}"
                );
            }
        }
        let ret_start = sword_slash_rotation_arc(&hand, 0.0, SlashArc::Return) * hand.axis;
        let ret_end = sword_slash_rotation_arc(&hand, 1.0, SlashArc::Return) * hand.axis;
        assert!(
            ret_end.x < ret_start.x - 0.35,
            "return cut must travel left to right, start {ret_start:?} end {ret_end:?}"
        );
        let sweep_start = sword_slash_rotation_arc(&hand, 0.0, SlashArc::Sweep) * hand.axis;
        let sweep_end = sword_slash_rotation_arc(&hand, 1.0, SlashArc::Sweep) * hand.axis;
        assert!(
            sweep_end.x > sweep_start.x + 0.40,
            "sweep must cut a wider horizontal path, start {sweep_start:?} end {sweep_end:?}"
        );
    }

    #[test]
    fn third_person_held_sword_carries_the_right_arm() {
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
                    ..default()
                },
                AvatarAnimationGraph::dummy(),
                ChildOf(actor),
            ))
            .id();
        let rest = Quat::IDENTITY;
        let shoulder = world
            .spawn((
                Name::new("rightShoulder"),
                Transform {
                    rotation: rest,
                    ..default()
                },
                AvatarJoint {
                    root,
                    rest,
                    axis: Vec3::NEG_X,
                    kind: JointKind::Shoulder,
                    sign: -1.0,
                    curl: 0.,
                    oppose: 0.,
                    digit: 0,
                    chain: 0,
                },
                ChildOf(root),
            ))
            .id();
        let arm = world
            .spawn((
                Name::new("rightUpperArm"),
                Transform {
                    rotation: rest,
                    ..default()
                },
                AvatarJoint {
                    root,
                    rest,
                    axis: Vec3::NEG_X,
                    kind: JointKind::UpperArm,
                    sign: -1.0,
                    curl: 0.,
                    oppose: 0.,
                    digit: 0,
                    chain: 0,
                },
                ChildOf(root),
            ))
            .id();
        let hand = world
            .spawn((
                Name::new("rightHand"),
                Transform::default(),
                AvatarJoint {
                    root,
                    rest,
                    axis: Vec3::NEG_X,
                    kind: JointKind::Hand,
                    sign: -1.0,
                    curl: 0.,
                    oppose: 0.,
                    digit: 0,
                    chain: 0,
                },
                ChildOf(arm),
            ))
            .id();
        let sword = world
            .spawn((HeldWeaponMesh, Transform::default(), ChildOf(hand)))
            .id();
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(1. / 60.));
        app.update();
        let clavicle = app.world().get::<Transform>(shoulder).unwrap().rotation;
        assert!(
            rest.angle_between(clavicle) < 0.05,
            "idle carry must leave the clavicle at rest, got {clavicle:?}"
        );
        let posed = app.world().get::<Transform>(arm).unwrap().rotation;
        let along = posed * Vec3::NEG_X;
        assert!(
            rest.angle_between(posed) > 0.8,
            "held sword must lift the hanging idle arm, got {posed:?}"
        );
        assert!(
            along.y < -0.50 && along.z > 0.18,
            "idle fist must hang at the hip, not the head, arm along {along:?}"
        );
        let hand_rot = app.world().get::<Transform>(hand).unwrap().rotation;
        let fingers = posed * hand_rot * Vec3::NEG_X;
        assert!(
            fingers.y < -0.20 && fingers.z > 0.50,
            "chained forearm must keep the fist below the skull, got {fingers:?}"
        );
        let grip = app.world().get::<Transform>(sword).unwrap();
        let world_blade = posed * hand_rot * grip.rotation * Vec3::Y;
        assert!(
            world_blade.dot(Vec3::Y) > 0.85,
            "cuboid blade must stand chest-up out of the posed fist, got {world_blade:?}"
        );
        let roll = sword_wrist_roll(Vec3::NEG_X, Vec3::Y);
        let (_, angle) = roll.to_axis_angle();
        assert!(
            angle > 0.12 && angle <= 0.62,
            "wrist roll must cup the hilt without flipping the hand, got {angle}"
        );
        assert!(
            (roll * Vec3::NEG_X).dot(Vec3::NEG_X) > 0.98,
            "wrist roll must not redirect the fingers"
        );
        assert!(
            (roll * Vec3::NEG_Y).z > 0.12,
            "palm must cup toward the hilt, not open the other way, got {:?}",
            roll * Vec3::NEG_Y
        );
    }

    #[test]
    fn dual_blade_leans_toward_the_foe_without_hanging() {
        let leaned = dual_blade_toward_foe(Vec3::Y, Quat::IDENTITY);
        assert!(
            leaned.z > 0.30,
            "dual-wield tip must attack the foe, got {leaned:?}"
        );
        assert!(
            leaned.y > 0.75,
            "lean must not hang the chamber tip, got {leaned:?}"
        );
        let back = dual_blade_toward_foe(Vec3::NEG_Z, Quat::IDENTITY);
        assert!(
            back.z > 0.30,
            "a backward blade must be turned onto the foe, got {back:?}"
        );
        assert!(
            back.z > 0.0 && Vec3::NEG_Z.dot(back) < 0.0,
            "tip must not come back at the body, got {back:?}"
        );
    }

    #[test]
    fn attack_clip_follows_job_weapon() {
        assert_eq!(attack_clip(crate::character::HeldWeapon::Unarmed), MotionClip::Punch);
        assert_eq!(attack_clip(crate::character::HeldWeapon::OneHand), MotionClip::Slash1H);
        assert_eq!(attack_clip(crate::character::HeldWeapon::GreatWeapon), MotionClip::Slash2H);
        assert_eq!(attack_clip(crate::character::HeldWeapon::PoleArm), MotionClip::Slash2H);
        assert_eq!(attack_clip(crate::character::HeldWeapon::DualWield), MotionClip::SlashDw);
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
        assert_eq!(resting_sword_visibility(true), Visibility::Hidden);
        assert_eq!(resting_sword_visibility(false), Visibility::Visible);
        assert!(rest_pose(MotionClip::Rest));
        assert!(rest_pose(MotionClip::IdleToRest));
        assert!(!rest_pose(MotionClip::RestToIdle));
        assert!((tick_rest_support(0., true, REST_SUPPORT_SECS) - 1.).abs() < 0.001);
        assert!(tick_rest_support(0., true, 0.35) < 0.25);
        assert!((tick_rest_support(1., false, REST_SUPPORT_SECS)).abs() < 0.001);
        let plant = rest_support_goal(JointKind::Hand);
        assert!(
            plant.x > 0.0 && plant.y < -0.6 && plant.z < 0.0,
            "rest support must plant the left hand behind, got {plant:?}"
        );
        let upper = rest_support_goal(JointKind::UpperArm);
        assert!(
            upper.z < -0.5 && upper.y < 0.0,
            "support upper arm reaches back, got {upper:?}"
        );
    }

    #[test]
    fn rest_plants_the_left_hand_behind_for_support() {
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
                    resting: true,
                    rest_support: 1.0,
                    ..default()
                },
                AvatarAnimationGraph::dummy(),
                ChildOf(actor),
            ))
            .id();
        let rest = Quat::IDENTITY;
        let arm = world
            .spawn((
                Name::new("leftUpperArm"),
                Transform {
                    rotation: rest,
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
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(1. / 60.));
        app.update();
        let posed = app.world().get::<Transform>(arm).unwrap().rotation;
        let dir = posed * Vec3::X;
        assert!(
            dir.z < -0.4 && dir.y < 0.0,
            "left rest support must reach behind, got {dir:?}"
        );
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
        assert_eq!(bridge_between(MotionClip::Idle, MotionClip::StrafeLeft), None);
        assert_eq!(bridge_between(MotionClip::Idle, MotionClip::WalkBack), None);
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
        assert_eq!(bridge_between(MotionClip::Idle, MotionClip::StrafeRight), None);
        assert!(
            RESPONSIVE_BLEND_MS >= 100 && RESPONSIVE_BLEND_MS <= 200,
            "idle→walk/strafe should blend in 0.1–0.2 s, got {RESPONSIVE_BLEND_MS}"
        );
        assert!(
            ONESHOT_BLEND_MS >= 50 && ONESHOT_BLEND_MS <= 100,
            "chop chain should blend in 0.05–0.1 s, got {ONESHOT_BLEND_MS}"
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
            attack_clip_for(crate::character::HeldWeapon::Unarmed, true, &graph),
            MotionClip::WalkPunch
        );
        assert_eq!(
            attack_clip_for(crate::character::HeldWeapon::Unarmed, false, &graph),
            MotionClip::Punch
        );
        graph.present[MotionClip::WalkSlash1H as usize] = true;
        assert_eq!(
            attack_clip_for(crate::character::HeldWeapon::OneHand, true, &graph),
            MotionClip::Slash1H
        );
        assert_eq!(
            attack_clip_for(crate::character::HeldWeapon::OneHand, false, &graph),
            MotionClip::Slash1H
        );
        graph.present[MotionClip::Slash1HReturn as usize] = true;
        graph.present[MotionClip::Slash1HSweep as usize] = true;
        assert_eq!(
            attack_clip_arc(
                crate::character::HeldWeapon::OneHand,
                false,
                &graph,
                SlashArc::Return,
                1
            ),
            MotionClip::Slash1HReturn
        );
        assert_eq!(
            attack_clip_arc(
                crate::character::HeldWeapon::OneHand,
                false,
                &graph,
                SlashArc::Sweep,
                2
            ),
            MotionClip::Slash1HSweep
        );
        assert_eq!(
            attack_clip_arc(
                crate::character::HeldWeapon::OneHand,
                true,
                &graph,
                SlashArc::Return,
                1
            ),
            MotionClip::Slash1HReturn
        );
        assert_eq!(
            attack_clip_arc(
                crate::character::HeldWeapon::OneHand,
                true,
                &graph,
                SlashArc::Sweep,
                2
            ),
            MotionClip::Slash1HSweep
        );
        graph.present[MotionClip::SlashDw as usize] = true;
        graph.present[MotionClip::SlashDwLeft as usize] = true;
        assert_eq!(
            attack_clip_arc(
                crate::character::HeldWeapon::DualWield,
                false,
                &graph,
                SlashArc::Diagonal,
                0
            ),
            MotionClip::SlashDw
        );
        assert_eq!(
            attack_clip_arc(
                crate::character::HeldWeapon::DualWield,
                false,
                &graph,
                SlashArc::Return,
                1
            ),
            MotionClip::SlashDwLeft
        );
        assert_eq!(
            attack_clip_arc(
                crate::character::HeldWeapon::DualWield,
                true,
                &graph,
                SlashArc::Sweep,
                2
            ),
            MotionClip::SlashDw
        );
        graph.present[MotionClip::WalkSlash2H as usize] = true;
        assert_eq!(
            attack_clip_for(crate::character::HeldWeapon::GreatWeapon, true, &graph),
            MotionClip::WalkSlash2H
        );
        assert_eq!(
            attack_clip_for(crate::character::HeldWeapon::GreatWeapon, false, &graph),
            MotionClip::Slash2H
        );
        assert_eq!(
            attack_clip_for(crate::character::HeldWeapon::PoleArm, false, &graph),
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
        assert!(one_hand_grip(MotionClip::Slash1H));
        assert!(one_hand_grip(MotionClip::Slash1HReturn));
        assert!(one_hand_grip(MotionClip::Slash1HSweep));
        assert!(!one_hand_grip(MotionClip::WalkSlash1H));
        assert!(!one_hand_grip(MotionClip::Punch));
        assert!(dual_right_grip(MotionClip::SlashDw));
        assert!(dual_right_grip(MotionClip::SlashDwReturn));
        assert!(!dual_right_grip(MotionClip::SlashDwLeft));
        assert!(dual_left_grip(MotionClip::SlashDwLeft));
        assert!(dual_left_grip(MotionClip::SlashDwLeftReturn));
        assert!(!dual_left_grip(MotionClip::SlashDw));
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
        let seated = ba_protocol::avatar_local_y();
        assert!((rest_y - seated).abs() < 0.001);
        assert!((roll_root(1.0).0.y - seated).abs() < 0.001);
        assert!(
            seated > -ba_protocol::PLAYER_HEIGHT,
            "soles must sit on the height-field, not in it"
        );
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
            rest.angle_between(pose_finger(&finger, FingerPose::Fist))
                > rest.angle_between(pose_finger(&finger, FingerPose::Relaxed)) + 1.0,
            "unarmed fist must curl past the relaxed hand"
        );
        assert!(fist_angles(1, 1).0 > 1.7, "PIP must lock the fist shut");
        assert!(
            grip_angles(1, 1).0 > 0.7 && grip_angles(1, 1).0 < 1.15,
            "sword PIP must wrap a hilt, not fold through it, got {}",
            grip_angles(1, 1).0
        );
        assert!(
            grip_angles(0, 0).1 < 0.8,
            "sword thumb must sit on the grip, not flop backward, got {}",
            grip_angles(0, 0).1
        );
        assert!(
            rest.angle_between(pose_finger(&finger, FingerPose::Grip))
                > rest.angle_between(pose_finger(&finger, FingerPose::Relaxed)) + 0.25
                && rest.angle_between(pose_finger(&finger, FingerPose::Fist))
                    > rest.angle_between(pose_finger(&finger, FingerPose::Grip)) + 0.4,
            "hilt wrap is tighter than idle and looser than a punch"
        );
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
        assert_eq!(bridge_between(MotionClip::Idle, MotionClip::WalkBack), None);
        assert_eq!(
            bridge_between(MotionClip::WalkBack, MotionClip::Idle),
            Some(MotionClip::WalkBackToIdle)
        );
        assert_eq!(bridge_between(MotionClip::Idle, MotionClip::StrafeLeft), None);
        assert_eq!(
            bridge_between(MotionClip::StrafeLeft, MotionClip::Idle),
            Some(MotionClip::StrafeLeftToIdle)
        );
        assert_eq!(bridge_between(MotionClip::Idle, MotionClip::StrafeRight), None);
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
    fn first_person_held_sword_closes_the_right_fingers() {
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
                    authored_walk: true,
                    ..default()
                },
                AvatarAnimationGraph::dummy(),
                ChildOf(actor),
            ))
            .id();
        let rest = Quat::IDENTITY;
        let hand = world
            .spawn((
                Name::new("rightHand"),
                Transform::default(),
                AvatarJoint {
                    root,
                    rest,
                    axis: Vec3::NEG_X,
                    kind: JointKind::Hand,
                    sign: -1.0,
                    curl: 0.,
                    oppose: 0.,
                    digit: 0,
                    chain: 0,
                },
                ChildOf(root),
            ))
            .id();
        let finger = world
            .spawn((
                Name::new("rightIndexProximal"),
                Transform {
                    rotation: rest,
                    ..default()
                },
                AvatarJoint {
                    root,
                    rest,
                    axis: Vec3::NEG_X,
                    kind: JointKind::Finger,
                    sign: -1.0,
                    curl: 0.30,
                    oppose: 0.0,
                    digit: 1,
                    chain: 0,
                },
                ChildOf(hand),
            ))
            .id();
        world.spawn((HeldWeaponMesh, Transform::default(), ChildOf(hand)));
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(1. / 60.));
        app.update();
        let posed = app.world().get::<Transform>(finger).unwrap().rotation;
        let bent = rest.angle_between(posed);
        assert!(
            bent > 0.5 && bent < 1.15,
            "right fingers must wrap the hilt, not crush through it, got {bent} rad"
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
    fn first_person_idle_does_not_aim_the_hips() {
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
                    authored_walk: true,
                    ..default()
                },
                AvatarAnimationGraph::dummy(),
                ChildOf(actor),
            ))
            .id();
        let clip = Quat::from_rotation_x(0.35);
        let hips = world
            .spawn((
                Name::new("hips"),
                Transform {
                    rotation: clip,
                    ..default()
                },
                AvatarJoint {
                    root,
                    rest: Quat::IDENTITY,
                    axis: Vec3::Y,
                    kind: JointKind::Hips,
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
        let posed = app.world().get::<Transform>(hips).unwrap().rotation;
        assert!(
            clip.angle_between(posed) < 0.001,
            "first person must not fold the pelvis toward -Z, got {posed:?}"
        );
    }

    #[test]
    fn two_hand_cut_keeps_idle_leg_width() {
        let mut app = App::new();
        app.add_plugins(TransformPlugin)
            .insert_resource(Time::<()>::default())
            .init_resource::<CameraView>()
            .init_resource::<SwingPose>()
            .insert_resource(EquipState::from_loadout(
                crate::character::HeldWeapon::GreatWeapon,
            ))
            .add_systems(
                PostUpdate,
                pose_humanoid_bones.before(bevy::transform::TransformSystems::Propagate),
            );
        app.world_mut().resource_mut::<CameraView>().mode = CameraMode::FirstPerson;
        app.world_mut().resource_mut::<SwingPose>().strike(0.0);
        let world = app.world_mut();
        let actor = world.spawn(Transform::default()).id();
        let root = world
            .spawn((
                Transform::default(),
                PlayerAvatarMesh,
                AvatarMotion {
                    authored_walk: true,
                    ..default()
                },
                AvatarAnimationGraph::dummy(),
                ChildOf(actor),
            ))
            .id();
        let idle = Quat::from_rotation_x(0.18) * Quat::from_rotation_z(0.22);
        let spawn_leg = |world: &mut World, name: &str, sign: f32| {
            world
                .spawn((
                    Name::new(name.to_string()),
                    Transform {
                        rotation: idle,
                        ..default()
                    },
                    AvatarJoint {
                        root,
                        rest: Quat::IDENTITY,
                        axis: Vec3::NEG_Y,
                        kind: JointKind::UpperLeg,
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
        let left = spawn_leg(world, "leftUpperLeg", 1.0);
        let right = spawn_leg(world, "rightUpperLeg", -1.0);
        app.world_mut()
            .resource_mut::<Time>()
            .advance_by(Duration::from_secs_f32(1. / 60.));
        app.update();
        let left_posed = app.world().get::<Transform>(left).unwrap().rotation;
        let right_posed = app.world().get::<Transform>(right).unwrap().rotation;
        assert!(
            idle.angle_between(left_posed) < 0.001,
            "2H cut must keep idle left stance, got {left_posed:?}"
        );
        assert!(
            idle.angle_between(right_posed) < 0.001,
            "2H cut must keep idle right stance, got {right_posed:?}"
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
    fn first_person_unarmed_idle_keeps_hands_low_and_apart() {
        let mut app = App::new();
        app.add_plugins(TransformPlugin)
            .insert_resource(Time::<()>::default())
            .init_resource::<CameraView>()
            .init_resource::<SwingPose>()
            .init_resource::<super::super::EquipState>()
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
                AvatarMotion::default(),
                AvatarAnimationGraph::dummy(),
                ChildOf(actor),
            ))
            .id();
        let rest = Quat::IDENTITY;
        let spawn = |world: &mut World, name: &str, kind: JointKind, sign: f32| {
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
        let shoulder = spawn(world, "rightShoulder", JointKind::Shoulder, -1.0);
        let upper = spawn(world, "rightUpperArm", JointKind::UpperArm, -1.0);
        let lower = spawn(world, "rightLowerArm", JointKind::LowerArm, -1.0);
        for _ in 0..8 {
            app.world_mut()
                .resource_mut::<Time>()
                .advance_by(Duration::from_secs_f32(1. / 60.));
            app.update();
        }
        let along = |e| {
            let axis = app.world().get::<AvatarJoint>(e).unwrap().axis;
            app.world().get::<Transform>(e).unwrap().rotation * axis
        };
        let rot = |e| app.world().get::<Transform>(e).unwrap().rotation;
        assert!(
            rest.angle_between(rot(shoulder)) < 0.05,
            "unarmed idle shoulder must stay at bind, got {:?}",
            rot(shoulder)
        );
        let upper_dir = along(upper);
        let lower_dir = along(lower);
        assert!(
            upper_dir.y < -0.85 && upper_dir.z.abs() < 0.20,
            "unarmed idle upper arm must hang, not reach, got {upper_dir:?}"
        );
        let elbow = upper_dir.angle_between(lower_dir);
        assert!(
            elbow > 0.18 && elbow < 0.85,
            "unarmed idle elbow must be a slight bend, got {elbow} upper {upper_dir:?} lower {lower_dir:?}"
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
        // Responsive: idle→walk/strafe/walk-back and walk→run start the loop
        // immediately with RESPONSIVE_BLEND_MS. Settling clips (walk→idle,
        // rest, crouch, run→walk) keep MotionBricks.
        (
            MotionClip::Run | MotionClip::WalkToRun,
            MotionClip::Walk | MotionClip::Idle | MotionClip::CrouchWalk,
        ) => Some(MotionClip::RunToWalk),
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
    profile: Res<super::CharacterProfile>,
    equip: Option<Res<EquipState>>,
    parents: Query<&ChildOf>,
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
        let held = if local.is_some() {
            equip
                .as_deref()
                .map(|e| e.held())
                .or_else(|| profile.draft.as_ref().map(|d| d.held_weapon()))
                .unwrap_or(super::HeldWeapon::Unarmed)
        } else {
            super::HeldWeapon::Unarmed
        };

        if local.is_some() {
            if let Some(at) = swing.started_at() {
                if at != state.last_swing {
                    state.last_swing = at;
                    if !two_hand_keeps_loco_clip(held)
                        && !keeps_unarmed_combo(state.clip, state.oneshot)
                    {
                        let moving = state.moving;
                        state.queue.clear();
                        state.queue.push(attack_clip_arc(
                            held,
                            moving,
                            graph,
                            swing.slash_arc(),
                            swing.combo(),
                        ));
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
            play_motion(&mut player, &mut transitions, graph, next, ONESHOT_BLEND_MS);
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
                    play_motion(
                        &mut player,
                        &mut transitions,
                        graph,
                        next,
                        if is_looping(next) {
                            RESPONSIVE_BLEND_MS
                        } else {
                            ONESHOT_BLEND_MS
                        },
                    );
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
                play_motion(&mut player, &mut transitions, graph, bridge, SETTLING_BLEND_MS);
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
        let blend = if is_looping(want) {
            RESPONSIVE_BLEND_MS
        } else {
            ONESHOT_BLEND_MS
        };
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
            reach.y < -0.05,
            "{body} first-person idle hand must hang below the shoulder: {reach:?}"
        );
        assert!(
            reach.z > -0.55,
            "{body} first-person idle must not ram the hand into the lens: {reach:?}"
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
