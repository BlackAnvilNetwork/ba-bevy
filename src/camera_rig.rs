//! One customized, skinned character for both first and third person.
//!
//! No capsules, duplicated facsimile arms or fabricated sword. The camera
//! moves just ahead of the actual avatar's face and the rig's real arms are
//! posed for first person in character::animation. The full GLB (including
//! authored clothing and equipment) stays the only rendered local player.

use bevy::prelude::*;
use crate::character::PlayerAvatarMesh;
use crate::forge_menu::GameFlow;
use crate::player::NoteComposer;

#[derive(Component)]
pub struct PlayerCamera;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum CameraMode {
    #[default]
    FirstPerson,
    ThirdPerson,
}

#[derive(Resource, Default, Debug)]
pub struct CameraView {
    pub mode: CameraMode,
    pub pitch: f32,
}

// The player origin is at standing eye height (1.65m). Looking from slightly
// forward of the face keeps nose/hair behind the view frustum without hiding
// the customized model or replacing its genuine arms.
pub const FIRST_PERSON_FACE_CLEARANCE: f32 = 0.42;
pub const THIRD_PERSON_DISTANCE: f32 = 3.5;
pub const THIRD_PERSON_HEIGHT: f32 = 0.45;

pub fn target_camera_transform(mode: CameraMode, pitch: f32) -> Transform {
    let pitch = pitch.clamp(-1.48, 1.48);
    match mode {
        CameraMode::FirstPerson => Transform {
            translation: Vec3::new(0.0, -0.18, -FIRST_PERSON_FACE_CLEARANCE),
            rotation: Quat::from_rotation_x(pitch),
            ..default()
        },
        CameraMode::ThirdPerson => Transform {
            translation: Vec3::new(0.0, THIRD_PERSON_HEIGHT, THIRD_PERSON_DISTANCE),
            rotation: Quat::from_rotation_x(pitch - 0.10),
            ..default()
        },
    }
}

pub fn toggle_camera_mode(
    keys: Res<ButtonInput<KeyCode>>,
    flow: Res<GameFlow>,
    note: Res<NoteComposer>,
    mut view: ResMut<CameraView>,
) {
    if flow.in_menu() || note.active {
        return;
    }
    if keys.just_pressed(KeyCode::KeyV) || keys.just_pressed(KeyCode::F5) {
        view.mode = match view.mode {
            CameraMode::FirstPerson => CameraMode::ThirdPerson,
            CameraMode::ThirdPerson => CameraMode::FirstPerson,
        };
        info!("Camera now {:?} — using the same skinned player GLB", view.mode);
    }
}

pub fn apply_camera_mode(
    time: Res<Time>,
    view: Res<CameraView>,
    mut camera: Query<(&mut Transform,&mut Projection), With<PlayerCamera>>,
    mut avatars: Query<&mut Visibility, With<PlayerAvatarMesh>>,
) {
    let Ok((mut camera,mut projection)) = camera.single_mut() else { return };
    let target = target_camera_transform(view.mode, view.pitch);
    if let Projection::Perspective(ref mut perspective) = *projection {
        // Fingers on the actual skinned model can be closer to the eyes than
        // Bevy's default 10cm clipping plane. Do not clip the real hands.
        perspective.near = if view.mode == CameraMode::FirstPerson { 0.025 } else { 0.1 };
    }

    // Never interpolate camera translation THROUGH the avatar's head. That
    // caused the original hair-on-screen glitch at mode changes. A camera
    // cut is safer than passing through geometry; pitch can still smooth.
    camera.translation = target.translation;
    let blend = 1.0 - (-18.0 * time.delta_secs()).exp();
    camera.rotation = camera.rotation.slerp(target.rotation, blend);

    // Exactly one real GLB is visible in both modes; the bone animation
    // system adjusts the upper arms for the first-person composition.
    for mut visibility in &mut avatars {
        if *visibility != Visibility::Inherited {
            *visibility = Visibility::Inherited;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn first_person_camera_clears_hair_without_hiding_real_avatar() {
        let fp = target_camera_transform(CameraMode::FirstPerson, 0.0);
        assert!(fp.translation.z < -0.35);
        assert!(fp.translation.y < 0.0);
        let tp = target_camera_transform(CameraMode::ThirdPerson, 0.0);
        assert!(tp.translation.z > 3.0);
        assert!(tp.translation.y > 0.0);
    }
    #[test]
    fn pitch_clamps_without_touching_player_yaw() {
        let high = target_camera_transform(CameraMode::FirstPerson, 500.0);
        let limited = target_camera_transform(CameraMode::FirstPerson, 1.48);
        assert_eq!(high.rotation, limited.rotation);
    }
}
