//! Spawn Studio-exported GLB under the player.

use bevy::prelude::*;

use super::{active_glb_path, CharacterProfile, ACTIVE_GLB};

#[derive(Component)]
pub struct PlayerAvatarMesh;

#[derive(Component)]
pub struct HasPlayerAvatar;

/// Despawn prior Studio mesh when the player is ready for a fresh attach.
pub fn despawn_stale_avatars(
    mut commands: Commands,
    meshes: Query<Entity, With<PlayerAvatarMesh>>,
    waiting: Query<(), (With<crate::player::Player>, Without<HasPlayerAvatar>)>,
) {
    if waiting.is_empty() {
        return;
    }
    for entity in &meshes {
        commands.entity(entity).despawn();
    }
}

/// Attach the active Studio GLB as a player child when a draft exists.
pub fn attach_player_avatar(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    profile: Res<CharacterProfile>,
    player: Query<Entity, (With<crate::player::Player>, Without<HasPlayerAvatar>)>,
) {
    let Some(draft) = profile.draft.as_ref() else {
        return;
    };
    if !active_glb_path().is_file() {
        warn!(
            "BaCharacter draft present but GLB missing at {}",
            active_glb_path().display()
        );
        return;
    }

    let Ok(player_entity) = player.single() else {
        return;
    };

    let glb = draft.glb_asset_path();
    let handle: Handle<WorldAsset> =
        asset_server.load(GltfAssetLabel::Scene(0).from_asset(glb.to_string()));

    // Player transform sits at eye height (~1.65); drop the mesh to feet.
    let avatar = commands
        .spawn((
            PlayerAvatarMesh,
            Name::new(format!("Avatar:{}", draft.name)),
            WorldAssetRoot(handle),
            Transform::from_xyz(0.0, -1.65, 0.0),
            Visibility::default(),
        ))
        .id();

    commands.entity(player_entity).add_child(avatar);
    commands.entity(player_entity).insert(HasPlayerAvatar);
    info!("Attached Studio avatar from {glb} (fallback path {ACTIVE_GLB})");
}
