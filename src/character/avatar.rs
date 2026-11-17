//! Spawn Studio-exported GLB under the player — wait for AssetEvent before claiming success.

use bevy::prelude::*;

use super::{active_glb_path, CharacterProfile, ACTIVE_GLB};

#[derive(Component)]
pub struct PlayerAvatarMesh;

#[derive(Component)]
pub struct HasPlayerAvatar;

/// Prevent an invalid model from being re-queued every frame.
#[derive(Component)]
pub struct AvatarLoadFailed;

#[derive(Component)]
pub struct PendingPlayerAvatar {
    pub handle: Handle<WorldAsset>,
}

/// Queue the active Studio GLB as a player child when a draft exists.
pub fn attach_player_avatar(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    profile: Res<CharacterProfile>,
    player: Query<
        Entity,
        (
            With<crate::player::Player>,
            Without<HasPlayerAvatar>,
            Without<PendingPlayerAvatar>,
            Without<AvatarLoadFailed>,
        ),
    >,
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

    let avatar = commands
        .spawn((
            PlayerAvatarMesh,
            Name::new(format!("Avatar:{}", draft.name)),
            WorldAssetRoot(handle.clone()),
            Transform::from_xyz(0.0, -1.65, 0.0),
            Visibility::default(),
        ))
        .id();

    super::prepare_avatar_animation(
        &mut commands, &asset_server, &mut graphs, avatar, glb,
    );
    commands.entity(player_entity).add_child(avatar);
    commands
        .entity(player_entity)
        .insert(PendingPlayerAvatar { handle });
    info!("Queued Studio avatar load from {glb} (fallback path {ACTIVE_GLB})");
}

/// Promote only after the scene *and all dependencies* have loaded.
/// An Added event is not enough: skins/materials may still be pending.
pub fn resolve_pending_avatars(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    pending: Query<(Entity, &PendingPlayerAvatar)>,
) {
    use bevy::asset::LoadState;

    for (entity, waiting) in &pending {
        let id = waiting.handle.id();
        if asset_server.is_loaded_with_dependencies(id) {
            commands.entity(entity).remove::<PendingPlayerAvatar>();
            commands.entity(entity).insert(HasPlayerAvatar);
            info!("Studio avatar ready with dependencies for {entity:?}");
        } else if matches!(asset_server.load_state(id), LoadState::Failed(_)) {
            warn!("Studio avatar failed to load for {entity:?}; keeping explicit failure marker");
            commands.entity(entity).remove::<PendingPlayerAvatar>();
            commands.entity(entity).insert(AvatarLoadFailed);
        }
    }
}
