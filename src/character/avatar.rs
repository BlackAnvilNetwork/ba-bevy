//! Spawn Studio-exported GLB under the player — wait for AssetEvent before claiming success.

use bevy::prelude::*;

use super::{active_glb_path, CharacterProfile, ACTIVE_GLB};

#[derive(Component)]
pub struct PlayerAvatarMesh;

#[derive(Component)]
pub struct HasPlayerAvatar;

#[derive(Component)]
pub struct PendingPlayerAvatar {
    pub handle: Handle<WorldAsset>,
}

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

/// Queue the active Studio GLB as a player child when a draft exists.
pub fn attach_player_avatar(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    profile: Res<CharacterProfile>,
    player: Query<
        Entity,
        (
            With<crate::player::Player>,
            Without<HasPlayerAvatar>,
            Without<PendingPlayerAvatar>,
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

    commands.entity(player_entity).add_child(avatar);
    commands
        .entity(player_entity)
        .insert(PendingPlayerAvatar { handle });
    info!("Queued Studio avatar load from {glb} (fallback path {ACTIVE_GLB})");
}

/// Promote Pending → HasPlayerAvatar only after the WorldAsset loads (or log failure).
pub fn resolve_pending_avatars(
    mut commands: Commands,
    mut events: MessageReader<AssetEvent<WorldAsset>>,
    pending: Query<(Entity, &PendingPlayerAvatar)>,
    assets: Res<Assets<WorldAsset>>,
) {
    let mut loaded = Vec::new();
    let mut failed = Vec::new();
    for ev in events.read() {
        match ev {
            AssetEvent::LoadedWithDependencies { id } | AssetEvent::Added { id } => {
                loaded.push(*id);
            }
            AssetEvent::Removed { id } => failed.push(*id),
            _ => {}
        }
    }

    for (entity, pending) in &pending {
        let id = pending.handle.id();
        if failed.iter().any(|f| *f == id) {
            warn!("Studio avatar asset failed/removed for {entity:?}");
            commands.entity(entity).remove::<PendingPlayerAvatar>();
            continue;
        }
        if loaded.iter().any(|l| *l == id) || assets.get(&pending.handle).is_some() {
            commands.entity(entity).remove::<PendingPlayerAvatar>();
            commands.entity(entity).insert(HasPlayerAvatar);
            info!("Studio avatar ready (AssetEvent) for {entity:?}");
        }
    }
}
