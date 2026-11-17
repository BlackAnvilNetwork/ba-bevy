use std::f32::consts::FRAC_PI_2;

use bevy::{
    input::mouse::AccumulatedMouseMotion,
    prelude::*,
    window::{CursorGrabMode, CursorOptions},
};

use crate::atmosphere::{self, StickAtmosphere};
use crate::data::GameData;
use crate::camera_rig::{CameraView,PlayerCamera};
use crate::rpg::Progression;

#[derive(Component)]
pub struct Player;

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
    commands.spawn((
        Player,
        Progression::default(),
        CameraSensitivity::default(),
        Name::new("Player"),
        transform,
        Visibility::default(),
        children![(
            atmosphere::camera_bundle(&atmo),
            PlayerCamera,
            Transform::from_rotation(Quat::from_rotation_x(pitch)),
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

pub fn move_player(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    data: Res<GameData>,
    note: Res<NoteComposer>,
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

    if direction != Vec3::ZERO {
        if let Some(mut f) = fighter {
            f.resting = false;
        }
        let speed = if keys.pressed(KeyCode::ShiftLeft) {
            6.5
        } else {
            3.5
        };
        transform.translation += direction.normalize() * speed * time.delta_secs();
        transform.translation.y = 1.65;
        let bounds = &data.zone.bounds;
        transform.translation.x = transform.translation.x.clamp(bounds.min_x, bounds.max_x);
        transform.translation.z = transform.translation.z.clamp(bounds.min_z, bounds.max_z);
    }
}
