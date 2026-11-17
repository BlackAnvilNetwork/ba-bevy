//! Render-only first/third-person camera and viewmodel.
//!
//! The Player transform is always YAW ONLY (server movement authority).
//! Camera pitch is a local child transform. A third-person avatar never
//! inherits the camera's pitch, and the first-person camera never draws
//! the avatar's head/hair inside the lens.
//!
//! This is deliberately not an input/movement state machine: one explicit
//! camera mode, one equipped-item lookup, and a visual-only viewmodel.

use bevy::prelude::*;

use crate::character::{CharacterProfile, PlayerAvatarMesh};
use crate::forge_menu::GameFlow;
use crate::player::{NoteComposer, Player};
use crate::rpg::{CombatKits, Progression, WeaponFamily};

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

#[derive(Component)]
pub struct FirstPersonRig;

#[derive(Component)]
struct FirstPersonHand {
    left: bool,
    rest: Transform,
}

#[derive(Component)]
struct FirstPersonWeapon;

pub const THIRD_PERSON_DISTANCE: f32 = 3.5;
pub const THIRD_PERSON_HEIGHT: f32 = 0.45;

pub fn target_camera_transform(mode: CameraMode, pitch: f32) -> Transform {
    let pitch = pitch.clamp(-1.48, 1.48);
    match mode {
        CameraMode::FirstPerson => Transform {
            translation: Vec3::ZERO,
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
    // Both the conventional MMO V key and F5 work; no overlapping bindings
    // with WASD, chat or the E interact key.
    if keys.just_pressed(KeyCode::KeyV) || keys.just_pressed(KeyCode::F5) {
        view.mode = match view.mode {
            CameraMode::FirstPerson => CameraMode::ThirdPerson,
            CameraMode::ThirdPerson => CameraMode::FirstPerson,
        };
        info!("Camera now {:?}", view.mode);
    }
}

/// Camera mode only changes visual transforms/visibility, never authoritative
/// movement state. Hide the entire *local* skinned scene in first person,
/// not individual hair meshes (hair/head share the same GLB and may regenerate).
pub fn apply_camera_mode(
    time: Res<Time>,
    flow: Res<GameFlow>,
    view: Res<CameraView>,
    mut camera: Query<&mut Transform, With<PlayerCamera>>,
    mut avatars: Query<&mut Visibility, (With<PlayerAvatarMesh>, Without<FirstPersonRig>)>,
    mut viewmodels: Query<&mut Visibility, (With<FirstPersonRig>, Without<PlayerAvatarMesh>)>,
) {
    let Ok(mut camera) = camera.single_mut() else { return };
    let target = target_camera_transform(view.mode, view.pitch);
    // Smooth orbit transition without changing the player collider or pose.
    let blend = 1.0 - (-12.0 * time.delta_secs()).exp();
    camera.translation = camera.translation.lerp(target.translation, blend);
    camera.rotation = camera.rotation.slerp(target.rotation, blend);

    // A camera moving away from the eye must clear the skull before the
    // third-person mesh becomes visible; otherwise hair flashes across the
    // whole screen for a few frames during the transition.
    let third = view.mode == CameraMode::ThirdPerson
        && camera.translation.z > 1.15
        && !flow.in_menu();
    for mut visibility in &mut avatars {
        *visibility = if third { Visibility::Inherited } else { Visibility::Hidden };
    }
    for mut visibility in &mut viewmodels {
        *visibility = if !third && !flow.in_menu() {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

/// The only authoritative equipment here is the selected character's item
/// reference. Do not invent a default sword for an unarmed character. Future
/// server equipment snapshots can update this same reference explicitly.
pub fn equipped_weapon_name(profile: &CharacterProfile, combat_weapon: Option<WeaponFamily>) -> Option<String> {
    if let Some(draft)=profile.draft.as_ref() {
        if let Some((_,weapon)) = draft.starter_equipment.iter()
            .find(|(slot,_)|slot.eq_ignore_ascii_case("weapon"))
        {
            if weapon.id.trim().is_empty() { return None; } // explicitly unarmed
            return Some(weapon.name.clone().unwrap_or_else(||weapon.id.clone()));
        }
    }
    // Until the persistent item system exposes live equipment, the combat
    // kit's job weapon is the only actual weapon selection used to resolve
    // attacks. Use it here too rather than showing fists for a sword fighter.
    combat_weapon.map(|weapon|weapon.as_str().to_owned())
}

fn skin_color(profile: &CharacterProfile) -> Color {
    let hex = profile.draft.as_ref()
        .and_then(|d| d.body_traits.iter().find(|(key,_)|
            key.eq_ignore_ascii_case("body")).map(|(_,v)|v))
        .and_then(|body| body.colors.values().find(|v| v.len()==7 && v.starts_with('#')));
    if let Some(hex) = hex {
        if let (Ok(r),Ok(g),Ok(b))=(
            u8::from_str_radix(&hex[1..3],16),
            u8::from_str_radix(&hex[3..5],16),
            u8::from_str_radix(&hex[5..7],16),
        ) { return Color::srgb_u8(r,g,b); }
    }
    Color::srgb(0.64,0.43,0.30)
}

/// A small render-only viewmodel that cannot affect combat/world collision.
/// The rig follows the camera pitch and uses real equipment data for the
/// weapon visibility. The hands are a baseline until skinned arm meshes
/// become part of the first-person asset contract.
pub fn spawn_first_person_rig(
    mut commands: Commands,
    camera: Query<Entity, (With<PlayerCamera>, Without<FirstPersonRig>)>,
    existing: Query<Entity, With<FirstPersonRig>>,
    profile: Res<CharacterProfile>,
    kits: Option<Res<CombatKits>>,
    player: Query<&Progression, With<Player>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    if !existing.is_empty() { return; }
    let Ok(camera) = camera.single() else { return };

    let hand_mat = materials.add(StandardMaterial {
        base_color: skin_color(&profile),
        perceptual_roughness: 0.86,
        unlit: true,
        ..default()
    });
    let hand_mesh = meshes.add(Capsule3d::new(0.105,0.26));

    let rig = commands.spawn((
        FirstPersonRig,
        Name::new("FirstPersonHands"),
        Transform::default(),
        Visibility::default(),
    )).id();
    commands.entity(camera).add_child(rig);
    for (left, x) in [(true,-0.29),(false,0.29)] {
        let transform = Transform::from_xyz(x, -0.34, -0.65)
            .with_rotation(Quat::from_rotation_x(-0.58));
        let hand = commands.spawn((
            FirstPersonHand { left, rest: transform },
            Mesh3d(hand_mesh.clone()),
            MeshMaterial3d(hand_mat.clone()),
            transform,
            Visibility::default(),
            Name::new(if left { "LeftHand" } else { "RightHand" }),
        )).id();
        commands.entity(rig).add_child(hand);
    }

    let job_weapon = kits.as_ref()
        .and_then(|kits|player.single().ok().map(|p|kits.weapon_for(p.job)));
    if let Some(item) = equipped_weapon_name(&profile,job_weapon)
        .filter(|name|!name.to_ascii_lowercase().contains("knuckles"))
    {
        let weapon_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(0.74,0.82,0.87),
            metallic: 0.8,
            perceptual_roughness: 0.3,
            unlit: true,
            ..default()
        });
        let grip_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(0.19,0.12,0.10),
            unlit: true,
            ..default()
        });
        let weapon = commands.spawn((
            FirstPersonWeapon,
            Name::new(format!("Equipped view weapon: {item}")),
            Transform::from_xyz(0.28,-0.30,-0.75)
                .with_rotation(Quat::from_rotation_z(-0.25)),
            Visibility::default(),
        )).id();
        commands.entity(rig).add_child(weapon);
        let grip = commands.spawn((
            Mesh3d(meshes.add(Cylinder::new(0.028,0.35))),
            MeshMaterial3d(grip_mat),
            Transform::from_xyz(0.0,0.10,0.0),
            Visibility::default(),
        )).id();
        commands.entity(weapon).add_child(grip);
        // Weapon silhouettes: a longer blade for swords/spears, a short staff
        // for wands. No fake item in inventory; render only selected Weapon.
        let lower = item.to_ascii_lowercase();
        let (length,width) = if lower.contains("dagger") { (0.28,0.065) }
            else if lower.contains("staff") || lower.contains("spear") { (0.98,0.045) }
            else { (0.56,0.095) };
        let blade = commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(width,length,0.025))),
            MeshMaterial3d(weapon_mat),
            Transform::from_xyz(0.0,0.32+length*0.5,0.0),
            Visibility::default(),
        )).id();
        commands.entity(weapon).add_child(blade);
    }
}

pub fn animate_first_person_hands(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mode: Res<CameraView>,
    flow: Res<GameFlow>,
    mut hands: Query<(&FirstPersonHand,&mut Transform)>,
) {
    if mode.mode != CameraMode::FirstPerson || flow.in_menu() { return; }
    let walking = [KeyCode::KeyW,KeyCode::KeyA,KeyCode::KeyS,KeyCode::KeyD]
        .iter().any(|key|keys.pressed(*key));
    let amplitude=if walking {0.025}else{0.005};
    let speed=if walking {10.0}else{2.0};
    let phase=time.elapsed_secs()*speed;
    for (hand,mut pose) in &mut hands {
        let sign=if hand.left {-1.0}else{1.0};
        *pose=hand.rest;
        pose.translation.y+=amplitude*(phase+sign).sin();
        pose.translation.x+=0.006*(phase+sign).cos();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn first_person_camera_does_not_embed_behind_character() {
        let pose=target_camera_transform(CameraMode::FirstPerson,0.0);
        assert_eq!(pose.translation,Vec3::ZERO);
        let back=target_camera_transform(CameraMode::ThirdPerson,0.0);
        assert!(back.translation.z>3.0);
        assert!(back.translation.y>0.0);
    }
    #[test]
    fn camera_pitch_is_bounded_and_does_not_change_player_pose() {
        let a=target_camera_transform(CameraMode::FirstPerson,500.0);
        let b=target_camera_transform(CameraMode::FirstPerson,1.48);
        assert_eq!(a.rotation,b.rotation);
    }
    #[test]
    fn unarmed_character_does_not_get_a_fake_sword() {
        assert_eq!(equipped_weapon_name(&CharacterProfile::default(),None),None);
        assert_eq!(equipped_weapon_name(&CharacterProfile::default(),Some(WeaponFamily::Sword)),Some("sword".to_owned()));
        assert_eq!(equipped_weapon_name(&CharacterProfile::default(),Some(WeaponFamily::Knuckles)),Some("knuckles".to_owned()));
    }
}
