use std::f32::consts::FRAC_PI_2;

use bevy::{
    input::mouse::AccumulatedMouseMotion,
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};

use crate::atmosphere::{self, StickAtmosphere};
use crate::data::GameData;
use crate::camera_rig::{CameraView,PlayerCamera};
use ba_protocol::{
    stillwater::{self, GATE, GREYMERE_ROAD, LANTERN_ROAD, POST, REST, SABLE_DOCK, TRAVEL_REACH},
    GhostStepChain, GHOST_STEP_BONUS, SPRINT_SPEED, WALK_SPEED,
};
use crate::character::CharacterProfile;
use crate::forge_menu::{Appearance, CharacterAppearance, CharacterName};
use crate::rpg::Progression;

#[derive(Component)]
pub struct Player;

/// Chained Space ghost-steps. The first tap is animation-only; repeats skip a little farther.
#[derive(Resource, Default)]
pub struct GhostStep(pub GhostStepChain);

/// Open while a note to the session is being typed. Movement and talk pause.
#[derive(Resource, Default)]
pub struct NoteComposer {
    pub active: bool,
    pub text: String,
    pub sent_until: f32,
}

#[derive(Component, Deref, DerefMut)]
pub struct CameraSensitivity(Vec2);

impl Default for CameraSensitivity {
    fn default() -> Self {
        Self(Vec2::new(0.0028, 0.0020))
    }
}

pub fn spawn_player(
    mut commands: Commands,
    atmo: Res<StickAtmosphere>,
    data: Res<GameData>,
    profile: Res<CharacterProfile>,
    mut view: ResMut<CameraView>,
) {
    let at = Vec3::from_array(data.zone.spawn.at);
    let look = Vec3::from_array(data.zone.spawn.look_at);
    let mut transform = Transform::from_translation(at).looking_at(look, Vec3::Y);
    if let Some(pose) = std::env::var("BA_POSE").ok() {
        let nums: Vec<f32> = pose
            .split(',')
            .filter_map(|part| part.trim().parse().ok())
            .collect();
        if nums.len() == 5 {
            transform = Transform::from_translation(Vec3::new(nums[0], nums[1], nums[2]))
                .with_rotation(Quat::from_euler(
                    EulerRot::YXZ,
                    nums[3].to_radians(),
                    nums[4].to_radians(),
                    0.0,
                ));
        }
    }
    // The networked player and third-person rig own yaw only. Pitch belongs
    // to the child camera, so the body never bends when the mouse looks up.
    let (yaw,pitch,_) = transform.rotation.to_euler(EulerRot::YXZ);
    view.pitch = pitch;
    transform.rotation=Quat::from_rotation_y(yaw);
    // The launcher skips the old Forge menu. Initialize progression from the
    // SAME account-selected character used by the avatar/weapon pipeline,
    // otherwise a saved Knight spawned with Time Mage stats and a staff.
    let mut progression = Progression::default();
    let (name, appearance) = if let Some(draft) = profile.draft.as_ref() {
        progression.job = draft.job_id();
        progression.sync_sheet();
        progression.hp = progression.vitals.max_hp;
        progression.mp = progression.vitals.max_mp;
        (draft.name.clone(), if draft.is_female() {
            Appearance::Female
        } else {
            Appearance::Male
        })
    } else {
        ("Wanderer".to_string(),Appearance::Male)
    };
    commands.spawn((
        Player,
        progression,
        CharacterName(name.clone()),
        CharacterAppearance(appearance),
        CameraSensitivity::default(),
        Name::new(format!("Player: {name}")),
        transform,
        Visibility::default(),
        children![(
            atmosphere::camera_bundle(&atmo),
            PlayerCamera,
            crate::camera_rig::target_camera_transform(view.mode, pitch),
        )],
    ));
}

pub fn grab_cursor(
    mut cursor_options: Single<&mut CursorOptions>,
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    note: Res<NoteComposer>,
) {
    if note.active {
        return;
    }
    if mouse.just_pressed(MouseButton::Left) {
        cursor_options.visible = false;
        cursor_options.grab_mode = CursorGrabMode::Locked;
    }
    if keys.just_pressed(KeyCode::Escape) {
        cursor_options.visible = true;
        cursor_options.grab_mode = CursorGrabMode::None;
    }
}

pub fn look_player(
    mouse: Res<AccumulatedMouseMotion>,
    cursor: Single<&CursorOptions>,
    note: Res<NoteComposer>,
    mut view: ResMut<CameraView>,
    player: Single<(&mut Transform, &CameraSensitivity), With<Player>>,
) {
    if note.active || cursor.grab_mode == CursorGrabMode::None {
        return;
    }
    let (mut transform, sensitivity) = player.into_inner();
    let delta = mouse.delta;
    if delta == Vec2::ZERO {
        return;
    }
    let delta_yaw = -delta.x * sensitivity.x;
    let delta_pitch = -delta.y * sensitivity.y;
    let (yaw, _, _) = transform.rotation.to_euler(EulerRot::YXZ);
    const PITCH_LIMIT: f32 = FRAC_PI_2 - 0.08;
    view.pitch = (view.pitch + delta_pitch).clamp(-PITCH_LIMIT,PITCH_LIMIT);
    transform.rotation = Quat::from_rotation_y(yaw + delta_yaw);
}

pub fn note_ghost_step(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    note: Res<NoteComposer>,
    mut ghost: ResMut<GhostStep>,
) {
    if note.active {
        return;
    }
    if keys.just_pressed(KeyCode::Space) {
        ghost.0.on_jump(time.elapsed_secs());
    }
}

pub fn move_player(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    note: Res<NoteComposer>,
    ghost: Res<GhostStep>,
    mut player: Query<(&mut Transform, Option<&mut crate::rpg::Fighter>), With<Player>>,
) {
    if note.active {
        return;
    }
    let Ok((mut transform, fighter)) = player.single_mut() else {
        return;
    };
    let mut direction = Vec3::ZERO;
    let forward = Vec3::new(transform.forward().x, 0.0, transform.forward().z).normalize_or_zero();
    let right = Vec3::new(transform.right().x, 0.0, transform.right().z).normalize_or_zero();

    if keys.pressed(KeyCode::KeyW) {
        direction += forward;
    }
    if keys.pressed(KeyCode::KeyS) {
        direction -= forward;
    }
    if keys.pressed(KeyCode::KeyA) {
        direction -= right;
    }
    if keys.pressed(KeyCode::KeyD) {
        direction += right;
    }

    let ghosting = ghost.0.active(time.elapsed_secs());
    if ghosting && direction == Vec3::ZERO {
        direction = forward;
    }

    if direction != Vec3::ZERO {
        if let Some(mut f) = fighter {
            f.resting = false;
        }
        // Instant target speed (no ramp). Walk↔run MotionBricks must stay short
        // (~0.6 s, --tokens 6) or the in-place gait slides under this snap.
        let mut speed = if keys.pressed(KeyCode::KeyC) {
            1.6
        } else if keys.pressed(KeyCode::ControlLeft) || keys.pressed(KeyCode::ControlRight) {
            SPRINT_SPEED
        } else {
            WALK_SPEED
        };
        if ghosting {
            speed += GHOST_STEP_BONUS;
        }
        transform.translation += direction.normalize() * speed * time.delta_secs();
        let (min_x, max_x, min_z, max_z) = stillwater::bounds(transform.translation.x);
        transform.translation.x = transform.translation.x.clamp(min_x, max_x);
        transform.translation.z = transform.translation.z.clamp(min_z, max_z);
        transform.translation.y = stillwater::stand(transform.translation.x, transform.translation.z);
        walk_zone_line(&mut transform);
    }
}

fn walk_zone_line(transform: &mut Transform) {
    let reach = |pose: ba_protocol::Pose| {
        Vec2::new(transform.translation.x - pose.x, transform.translation.z - pose.z).length()
            < TRAVEL_REACH
    };
    let dest = if reach(GREYMERE_ROAD) {
        Some(POST)
    } else if reach(LANTERN_ROAD) {
        Some(stillwater::at(GREYMERE_ROAD.x, GREYMERE_ROAD.z + 6.))
    } else if reach(SABLE_DOCK) {
        Some(stillwater::at(GATE.x, GATE.z - 6.))
    } else if reach(GATE) {
        Some(REST)
    } else {
        None
    };
    if let Some(pose) = dest {
        transform.translation = Vec3::new(pose.x, stillwater::stand(pose.x, pose.z), pose.z);
    }
}
