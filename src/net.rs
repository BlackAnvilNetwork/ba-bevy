//! Optional localhost zone client (dev prototype). Connects after Enter Yard when
//! `BA_ZONE=1` or `BA_ZONE_URL` is set (default `ws://127.0.0.1:8787`).
//!
//! Connection phases are explicit — never treat channel creation as Welcome.

use std::collections::{HashMap, VecDeque};
use std::fs;
use std::path::PathBuf;
use sha2::{Digest, Sha256};
use std::net::TcpStream;
use std::sync::mpsc::{self, Receiver, Sender, TryRecvError};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use bevy::prelude::*;
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{connect, Message};

use ba_protocol::{
    integrate_movement, AppearanceWire, ClientMessage, MoveAxes, PlayerState, Pose, ServerMessage,
    TraitRefWire, DEFAULT_WS_URL, PROTOCOL_VERSION, TICK_DT,
};

use crate::character::CharacterProfile;
use crate::forge_menu::{Appearance, CharacterDraft, GameFlow};
use crate::player::Player;
use crate::rpg::JobId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ZonePhase {
    #[default]
    Idle,
    Connecting,
    WelcomeVerified,
    Disconnected,
    Rejected,
}

#[derive(Resource, Default)]
pub struct ZoneNet {
    pub enabled: bool,
    pub phase: ZonePhase,
    pub local_player_id: Option<u32>,
    pub last_error: Option<String>,
    pub retry_at: Option<Instant>,
    pub connect_started_at: Option<Instant>,
    // One-time scoped character reconnect credential, rotated each Welcome.
    // It is not a Discord token or a wallet signer.
    pub reconnect_token: Option<String>,
    tx: Option<Sender<ClientMessage>>,
    rx: Option<Arc<Mutex<Receiver<ServerMessage>>>>,
}

impl ZoneNet {
    pub fn online(&self) -> bool {
        self.phase == ZonePhase::WelcomeVerified
    }
}

#[derive(Component)]
pub struct RemotePlayer {
    pub player_id: u32,
    pub target: Vec3,
    pub target_yaw: f32,
    pub avatar_hash: Option<String>,
    pub pending_avatar: Option<Handle<WorldAsset>>,
    pub has_avatar: bool,
}

#[derive(Component)]
pub(crate) struct RemoteAvatarPlaceholder;

const MAX_UNACKED_PREDICTIONS: usize = 128;

#[derive(Clone, Copy, Debug)]
struct PredictedInput {
    seq: u64,
    axes: MoveAxes,
}

/// Single source of truth for local speculative movement in zone mode.
/// Bevy's player Transform is presentation; only snapshots can reconcile it.
#[derive(Resource, Default, Debug)]
pub struct ZonePrediction {
    next_seq: u64,
    pending: VecDeque<PredictedInput>,
    pub predicted: Option<Pose>,
    last_server_tick: Option<u64>,
}

impl ZonePrediction {
    fn reset(&mut self, pose: Option<Pose>) {
        self.pending.clear();
        self.next_seq = 0;
        self.last_server_tick = None;
        self.predicted = pose;
    }

    fn send(&mut self, axes: MoveAxes) -> (u64, Pose) {
        self.next_seq = self.next_seq.saturating_add(1);
        let seq = self.next_seq;
        let base = self.predicted.unwrap_or_default();
        let next = integrate_movement(base, axes, TICK_DT);
        self.predicted = Some(next);
        self.pending.push_back(PredictedInput { seq, axes });
        while self.pending.len() > MAX_UNACKED_PREDICTIONS {
            self.pending.pop_front();
        }
        (seq, next)
    }

    fn reconcile(&mut self, authoritative: Pose, ack: u64, tick: u64) {
        if self.last_server_tick.is_some_and(|last| tick <= last) {
            return; // delayed/out-of-order snapshots cannot rewind local simulation
        }
        self.last_server_tick = Some(tick);
        self.pending.retain(|entry| entry.seq > ack);
        let mut predicted = authoritative;
        for input in &self.pending {
            predicted = integrate_movement(predicted, input.axes, TICK_DT);
        }
        self.predicted = Some(predicted);
    }
}

#[cfg(test)]
mod prediction_tests {
    use super::*;

    #[test]
    fn ack_prunes_history_and_replays_only_unacknowledged_input() {
        let spawn = Pose::default();
        let mut p = ZonePrediction::default();
        p.reset(Some(spawn));
        let forward = MoveAxes { forward: 1.0, strafe: 0.0, yaw: 0.0, sprint: false };
        for _ in 0..4 {
            p.send(forward);
        }
        let server = integrate_movement(integrate_movement(spawn, forward, TICK_DT), forward, TICK_DT);
        p.reconcile(server, 2, 50);
        assert_eq!(p.pending.len(), 2);
        assert_eq!(p.pending.front().unwrap().seq, 3);
        let expected = integrate_movement(integrate_movement(server, forward, TICK_DT), forward, TICK_DT);
        assert_eq!(p.predicted, Some(expected));
        p.reconcile(spawn, 0, 49); // old snapshot must not rewind prediction
        assert_eq!(p.predicted, Some(expected));
    }

    #[test]
    fn reconnect_clears_input_history_and_uses_new_spawn() {
        let mut p = ZonePrediction::default();
        p.reset(Some(Pose::default()));
        let input = MoveAxes { forward: 1.0, strafe: 0.0, yaw: 0.0, sprint: true };
        p.send(input);
        let new_spawn = Pose { x: 9.0, ..Pose::default() };
        p.reset(Some(new_spawn));
        assert!(p.pending.is_empty());
        assert_eq!(p.next_seq, 0);
        assert_eq!(p.predicted, Some(new_spawn));
    }
}


pub fn maybe_enable_zone_net(mut net: ResMut<ZoneNet>) {
    if net.enabled {
        return;
    }
    if std::env::var_os("BA_ZONE").is_some() || std::env::var_os("BA_ZONE_URL").is_some() {
        net.enabled = true;
        let url = std::env::var("BA_ZONE_URL").unwrap_or_else(|_| DEFAULT_WS_URL.to_string());
        info!("Zone net enabled → {url} (dev loopback prototype)");
    }
}

pub fn connect_zone_on_enter(
    flow: Res<GameFlow>,
    draft: Res<CharacterDraft>,
    profile: Res<CharacterProfile>,
    player: Query<&Transform, With<Player>>,
    mut net: ResMut<ZoneNet>,
    mut prediction: ResMut<ZonePrediction>,
) {
    if !net.enabled || flow.in_menu() {
        return;
    }
    // Welcome timeout / rejected → schedule retry instead of staying stuck.
    if net.phase == ZonePhase::Connecting {
        if let Some(started) = net.connect_started_at {
            if started.elapsed() > Duration::from_secs(8) {
                warn!("Zone Welcome timeout — resetting to Disconnected");
                net.phase = ZonePhase::Disconnected;
                prediction.reset(None);
                net.tx = None;
                net.rx = None;
                net.connect_started_at = None;
                net.retry_at = Some(Instant::now() + Duration::from_secs(3));
                net.last_error = Some("welcome_timeout".into());
            }
        }
        return;
    }
    if net.phase == ZonePhase::Rejected {
        // Admission failure is a terminal result: a fresh Discord-authorized
        // game ticket must come from the launcher. No silent retry loop.
        return;
    }
    if net.phase == ZonePhase::WelcomeVerified {
        return;
    }
    match net.phase {
        ZonePhase::Idle | ZonePhase::Disconnected => {}
        _ => return,
    }
    if let Some(at) = net.retry_at {
        if Instant::now() < at {
            return;
        }
    }
    let Ok(transform) = player.single() else {
        return;
    };
    let appearance = appearance_from_local(&draft, &profile);
    let (_, yaw, _) = transform.rotation.to_euler(EulerRot::YXZ);
    let pose = Pose {
        x: transform.translation.x,
        y: transform.translation.y,
        z: transform.translation.z,
        yaw,
    };

    let url = std::env::var("BA_ZONE_URL").unwrap_or_else(|_| DEFAULT_WS_URL.to_string());
    let reconnect_token = net.reconnect_token.clone();
    let ticket = if reconnect_token.is_some() {
        None
    } else {
        std::env::var("BA_GAME_TICKET").ok()
    };
    let (out_tx, out_rx) = mpsc::channel::<ClientMessage>();
    let (in_tx, in_rx) = mpsc::channel::<ServerMessage>();
    let url_thread = url.clone();

    thread::Builder::new()
        .name("ba-zone-ws".into())
        .spawn(move || {
            if let Err(err) = zone_ws_thread(&url_thread, appearance, pose, ticket, reconnect_token, out_rx, in_tx) {
                eprintln!("zone ws ended: {err}");
            }
        })
        .ok();

    net.tx = Some(out_tx);
    net.rx = Some(Arc::new(Mutex::new(in_rx)));
    net.phase = ZonePhase::Connecting;
    net.connect_started_at = Some(Instant::now());
    net.last_error = None;
    info!("Zone ConnectRequested → {url}");
}

fn job_wire(job: JobId) -> String {
    match job {
        JobId::TimeMage => "TimeMage",
        JobId::Knight => "Knight",
        JobId::Rogue => "Rogue",
        JobId::Cleric => "Cleric",
        JobId::Sorcerer => "Sorcerer",
        JobId::Fighter => "Fighter",
        JobId::BattleMage => "BattleMage",
        JobId::Druid => "Druid",
    }
    .into()
}

fn appearance_from_local(draft: &CharacterDraft, profile: &CharacterProfile) -> AppearanceWire {
    if let Some(ba) = profile.draft.as_ref() {
        let body = ba
            .body_traits
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    TraitRefWire {
                        id: v.id.clone(),
                        name: v.name.clone(),
                    },
                )
            })
            .collect();
        let equip = ba
            .starter_equipment
            .iter()
            .map(|(k, v)| {
                (
                    k.clone(),
                    TraitRefWire {
                        id: v.id.clone(),
                        name: v.name.clone(),
                    },
                )
            })
            .collect();
        return AppearanceWire {
            character_id: ba.character_id.clone(),
            name: ba.name.clone(),
            starting_job: ba.starting_job.clone(),
            manifest_id: ba.manifest_id.clone(),
            body_traits: body,
            starter_equipment: equip,
            // A content hash, never a file path. Receiver verifies bytes before
            // permitting the renderer to load a cached model.
            glb_content_hash: ba.runtime.content_hash.clone().filter(|hash| {
                hash.len() == 64 && hash.chars().all(|c| c.is_ascii_hexdigit())
            }),
        };
    }
    AppearanceWire {
        character_id: format!("local-{}", draft.name),
        name: draft.name.clone(),
        starting_job: job_wire(draft.job),
        manifest_id: if matches!(draft.appearance, Appearance::Female) {
            "anata/female".into()
        } else {
            "anata/male".into()
        },
        body_traits: Default::default(),
        starter_equipment: Default::default(),
        glb_content_hash: None,
    }
}

fn zone_ws_thread(
    url: &str,
    appearance: AppearanceWire,
    pose: Pose,
    ticket: Option<String>,
    reconnect_token: Option<String>,
    out_rx: Receiver<ClientMessage>,
    in_tx: Sender<ServerMessage>,
) -> Result<(), Box<dyn std::error::Error>> {
    if url.starts_with("ws://") &&
        !(url.starts_with("ws://127.0.0.1:") || url.starts_with("ws://localhost:")
            || url.starts_with("ws://[::1]:"))
    {
        return Err("Unencrypted WebSocket connections are allowed only on loopback".into());
    }
    let (mut socket, _resp) = connect(url)?;
    if let MaybeTlsStream::Plain(stream) = socket.get_mut() {
        stream.set_read_timeout(Some(Duration::from_millis(50)))?;
        stream.set_nonblocking(false)?;
    }
    let hello = ClientMessage::Hello {
        protocol: PROTOCOL_VERSION,
        appearance,
        pose,
        ticket,
        reconnect_token,
    };
    socket.send(Message::Text(hello.to_json()?.into()))?;

    loop {
        while let Ok(msg) = out_rx.try_recv() {
            socket.send(Message::Text(msg.to_json()?.into()))?;
        }

        match socket.read() {
            Ok(Message::Text(text)) => {
                if let Ok(server_msg) = ServerMessage::from_json(&text) {
                    let _ = in_tx.send(server_msg);
                }
            }
            Ok(Message::Ping(p)) => {
                socket.send(Message::Pong(p))?;
            }
            Ok(Message::Close(_)) => break,
            Ok(_) => {}
            Err(tungstenite::Error::Io(err))
                if err.kind() == std::io::ErrorKind::WouldBlock
                    || err.kind() == std::io::ErrorKind::TimedOut =>
            {
                thread::sleep(Duration::from_millis(5));
            }
            Err(err) => return Err(err.into()),
        }
    }
    Ok(())
}

fn draft_job(profile: &CharacterProfile, draft: &CharacterDraft) -> JobId {
    profile
        .draft
        .as_ref()
        .map(|d| d.job_id())
        .unwrap_or(draft.job)
}

fn zone_hotbar_press(keys: &ButtonInput<KeyCode>, job: JobId) -> Option<String> {
    use ba_combat::starter_hotbar;
    let bar = starter_hotbar(match job {
        JobId::TimeMage => ba_combat::JobId::TimeMage,
        JobId::Knight => ba_combat::JobId::Knight,
        JobId::Rogue => ba_combat::JobId::Rogue,
        JobId::Cleric => ba_combat::JobId::Cleric,
        JobId::Sorcerer => ba_combat::JobId::Sorcerer,
        JobId::Fighter => ba_combat::JobId::Fighter,
        JobId::BattleMage => ba_combat::JobId::BattleMage,
        JobId::Druid => ba_combat::JobId::Druid,
    });
    let slot = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
    ]
    .iter()
    .position(|k| keys.just_pressed(*k))?;
    bar.get(slot).map(|a| a.id.clone())
}

pub fn send_local_move(
    flow: Res<GameFlow>,
    net: Res<ZoneNet>,
    draft: Res<CharacterDraft>,
    profile: Res<CharacterProfile>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    player: Query<&Transform, With<Player>>,
    time: Res<Time>,
    mut elapsed: Local<f32>,
    mut prediction: ResMut<ZonePrediction>,
) {
    if !net.online() || flow.in_menu() {
        *elapsed = 0.0;
        return;
    }
    let Some(tx) = net.tx.as_ref() else { return };
    if mouse.just_pressed(MouseButton::Left) || keys.just_pressed(KeyCode::KeyF) {
        let _ = tx.send(ClientMessage::Attack { target_id: 0 });
    }
    if let Some(ability_id) = zone_hotbar_press(&keys, draft_job(&profile, &draft)) {
        let _ = tx.send(ClientMessage::Cast { ability_id });
    }
    // One command per local tick; the server simulates at its own fixed tick.
    *elapsed = (*elapsed + time.delta_secs()).min(TICK_DT * 1.5);
    if *elapsed < TICK_DT {
        return;
    }
    *elapsed -= TICK_DT;

    let Ok(transform) = player.single() else { return };
    let (_, yaw, _) = transform.rotation.to_euler(EulerRot::YXZ);
    let axes = MoveAxes {
        forward: (keys.pressed(KeyCode::KeyW) as i8 - keys.pressed(KeyCode::KeyS) as i8) as f32,
        strafe: (keys.pressed(KeyCode::KeyD) as i8 - keys.pressed(KeyCode::KeyA) as i8) as f32,
        yaw,
        sprint: keys.pressed(KeyCode::ShiftLeft),
    };
    let seq = prediction.next_seq.saturating_add(1);
    if tx.send(ClientMessage::MoveIntent {
        seq, forward: axes.forward, strafe: axes.strafe, yaw: axes.yaw, sprint: axes.sprint,
    }).is_ok() {
        prediction.send(axes);
    }
}

const MAX_CACHED_AVATAR_SIZE: u64 = 32 * 1024 * 1024;

fn verified_cache_uri(hash: &str) -> Option<String> {
    // No filesystem path supplied by the peer ever reaches Bevy's loader.
    if hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return None;
    }
    let root = PathBuf::from(std::env::var_os("BA_CHARACTER_ROOT")?);
    let path = root.join("cache").join(format!("{hash}.glb"));
    let size = fs::metadata(&path).ok()?.len();
    if size < 20 || size > MAX_CACHED_AVATAR_SIZE {
        return None;
    }
    let content = fs::read(&path).ok()?;
    if content.len() < 20 || &content[0..4] != b"glTF" {
        return None;
    }
    if u32::from_le_bytes(content[4..8].try_into().ok()?) != 2
        || u32::from_le_bytes(content[8..12].try_into().ok()?) as usize != content.len()
    {
        return None;
    }
    let actual = format!("{:x}", Sha256::digest(&content));
    if !actual.eq_ignore_ascii_case(hash) {
        warn!("Rejecting modified remote avatar cache: {hash}");
        return None;
    }
    Some(format!("character://cache/{hash}.glb"))
}

fn spawn_remote_player(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    asset_server: &AssetServer,
    graphs: &mut Assets<AnimationGraph>,
    state: &PlayerState,
) {
    let hash = state.appearance.glb_content_hash.clone();
    if let Some(hash) = hash.as_deref() {
        crate::avatar_delivery::prefetch(hash);
    }
    let uri = hash.as_deref().and_then(verified_cache_uri);
    let handle: Option<Handle<WorldAsset>> = uri.as_ref().map(|uri|
        asset_server.load(GltfAssetLabel::Scene(0).from_asset(uri.clone()))
    );
    let root = commands.spawn((
        RemotePlayer {
            player_id: state.player_id,
            target: Vec3::new(state.pose.x,state.pose.y,state.pose.z),
            target_yaw: state.pose.yaw,
            avatar_hash: hash,
            pending_avatar: handle.clone(),
            has_avatar: false,
        },
        Name::new(format!("Remote:{}",state.appearance.name)),
        Transform::from_translation(Vec3::new(state.pose.x,state.pose.y,state.pose.z))
            .with_rotation(Quat::from_euler(EulerRot::YXZ,state.pose.yaw,0.0,0.0)),
        Visibility::default(),
    )).id();

    // Leave capsule visible until all glTF dependencies have loaded. A failed
    // download/model does not make other players invisible.
    let mesh = meshes.add(Capsule3d::new(0.35,1.0));
    let mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.75,0.55,0.35), ..default()
    });
    let capsule = commands.spawn((
        RemoteAvatarPlaceholder,
        Name::new(format!("RemoteCapsule:{}",state.appearance.name)),
        Mesh3d(mesh), MeshMaterial3d(mat), Transform::default(),
        Visibility::default(),
    )).id();
    commands.entity(root).add_child(capsule);

    if let Some(handle) = handle {
        let scene = commands.spawn((
            Name::new(format!("RemoteAvatar:{}",state.appearance.name)),
            WorldAssetRoot(handle),
            Transform::from_xyz(0.0,-1.65,0.0),
            Visibility::default(),
        )).id();
        crate::character::prepare_avatar_animation(commands, asset_server, graphs, scene, &uri.expect("verified avatar URI present with handle"));
        commands.entity(root).add_child(scene);
    }
}

/// Once the cross-machine download completes, queue its verified GLB without
/// recreating player entities. Remove the fallback only after the scene AND
/// its dependencies are loaded by Bevy.
pub fn refresh_remote_avatar_models(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut remotes: Query<(Entity,&mut RemotePlayer,&Children)>,
    placeholders: Query<(),With<RemoteAvatarPlaceholder>>,
) {
    use bevy::asset::LoadState;
    for (entity,mut remote,children) in &mut remotes {
        if remote.has_avatar {continue;}
        if let Some(handle) = remote.pending_avatar.as_ref() {
            if asset_server.is_loaded_with_dependencies(handle.id()) {
                for child in children.iter() {
                    if placeholders.get(child).is_ok() {
                        commands.entity(child).despawn();
                    }
                }
                remote.pending_avatar=None;
                remote.has_avatar=true;
            } else if matches!(asset_server.load_state(handle.id()),LoadState::Failed(_)) {
                // Explicit fallback; no silent infinite retries in renderer.
                warn!("Remote avatar scene load failed; retaining capsule");
                remote.pending_avatar=None;
                remote.avatar_hash=None;
            }
            continue;
        }
        if let Some(uri) = remote.avatar_hash.as_deref().and_then(verified_cache_uri) {
            let handle: Handle<WorldAsset>=asset_server.load(GltfAssetLabel::Scene(0).from_asset(uri.clone()));
            let scene=commands.spawn((
                Name::new("RemoteVerifiedAvatar"),
                WorldAssetRoot(handle.clone()),
                Transform::from_xyz(0.0,-1.65,0.0),
                Visibility::default(),
            )).id();
            crate::character::prepare_avatar_animation(
                &mut commands, &asset_server, &mut graphs, scene, &uri,
            );
            commands.entity(entity).add_child(scene);
            remote.pending_avatar=Some(handle);
        }
    }
}

pub fn apply_zone_snapshots(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    mut net: ResMut<ZoneNet>,
    mut remotes: Query<(Entity, &mut RemotePlayer, &Transform)>,
    mut local_prog: Query<&mut crate::rpg::Progression, With<Player>>,
    mut local_xf: Query<&mut Transform, (With<Player>, Without<RemotePlayer>)>,
    mut prediction: ResMut<ZonePrediction>,
    time: Res<Time>,
) {
    let Some(rx) = net.rx.clone() else {
        return;
    };
    let Ok(rx) = rx.lock() else {
        return;
    };

    let mut latest: Option<Vec<PlayerState>> = None;
    let mut local_hp: Option<(i32, i32)> = None;
    let mut local_progression: Option<(u32,u64)> = None;
    let mut local_authoritative: Option<(Pose, u64, u64)> = None;
    let mut spawn: Option<Pose> = None;
    loop {
        match rx.try_recv() {
            Ok(ServerMessage::Welcome {
                player_id, spawn: s, reconnect_token, ..
            }) => {
                prediction.reset(Some(s));
                net.reconnect_token = reconnect_token;
                net.local_player_id = Some(player_id);
                net.phase = ZonePhase::WelcomeVerified;
                net.connect_started_at = None;
                spawn = Some(s);
                info!("Zone WelcomeVerified player_id={player_id}");
            }
            Ok(ServerMessage::Snapshot { tick, players }) => {
                if let Some(id) = net.local_player_id {
                    if let Some(me) = players.iter().find(|p| p.player_id == id) {
                        local_hp = Some((me.hp, me.max_hp));
                        local_progression = Some((me.level, me.xp));
                        local_authoritative = Some((me.pose, me.last_input_seq, tick));
                    }
                }
                latest = Some(players);
            }
            Ok(ServerMessage::Damage {
                attacker_id,
                target_id,
                damage,
                hit,
                crit,
                target_hp,
                ..
            }) => {
                if hit {
                    info!(
                        "Zone hit {attacker_id}->{target_id} dmg={damage}{} hp={target_hp}",
                        if crit { " CRIT" } else { "" }
                    );
                } else {
                    info!("Zone miss {attacker_id}->{target_id}");
                }
                if Some(target_id) == net.local_player_id {
                    if let Ok(mut prog) = local_prog.single_mut() {
                        prog.hp = target_hp;
                    }
                }
            }
            Ok(ServerMessage::Heal {
                target_id,
                amount,
                target_hp,
                ..
            }) => {
                info!("Zone heal ->{target_id} +{amount} hp={target_hp}");
                if Some(target_id) == net.local_player_id {
                    if let Ok(mut prog) = local_prog.single_mut() {
                        prog.hp = target_hp;
                    }
                }
            }
            Ok(ServerMessage::BuffApplied {
                caster_id,
                ability_id,
                atk_mul,
                duration_s,
                ..
            }) => {
                info!("Zone buff {ability_id} on {caster_id} mul={atk_mul:?} {duration_s}s");
            }
            Ok(ServerMessage::CastRejected {
                ability_id,
                reason,
                ..
            }) => {
                warn!("Zone cast rejected {ability_id}: {reason}");
            }
            Ok(ServerMessage::Reject { reason }) => {
                warn!("Zone rejected: {reason}");
                net.phase = ZonePhase::Rejected;
                prediction.reset(None);
                net.reconnect_token = None;
                net.connect_started_at = None;
                net.tx = None;
                net.rx = None;
                net.local_player_id = None;
                net.last_error = Some(reason);
                net.retry_at = Some(Instant::now() + Duration::from_secs(5));
            }
            Ok(_) => {}
            Err(TryRecvError::Empty) => break,
            Err(TryRecvError::Disconnected) => {
                net.phase = ZonePhase::Disconnected;
                prediction.reset(None);
                net.tx = None;
                net.rx = None;
                net.local_player_id = None;
                net.retry_at = Some(Instant::now() + Duration::from_secs(3));
                warn!("Zone disconnected; retry in 3s");
                break;
            }
        }
    }
    drop(rx);

    if let Some(s) = spawn {
        if let Ok(mut xf) = local_xf.single_mut() {
            xf.translation = Vec3::new(s.x, s.y, s.z);
            xf.rotation = Quat::from_euler(EulerRot::YXZ, s.yaw, 0.0, 0.0);
        }
    }

    if let Some((server_pose, ack, tick)) = local_authoritative {
        prediction.reconcile(server_pose, ack, tick);
    }

    // The player Transform is a presentation-only view. Server snapshots
    // reconcile the predicted pose; rendering eases to it instead of snapping
    // back to the previous server pose each packet.
    if let Some(target) = prediction.predicted {
        if let Ok(mut transform) = local_xf.single_mut() {
            let goal = Vec3::new(target.x, target.y, target.z);
            let delta = transform.translation.distance(goal);
            if delta > 2.5 {
                transform.translation = goal;
            } else if delta > 0.001 {
                let alpha = 1.0 - (-20.0 * time.delta_secs()).exp();
                transform.translation = transform.translation.lerp(goal, alpha);
            }
        }
    }

    if let Some((level,xp)) = local_progression {
        if let Ok(mut prog) = local_prog.single_mut() {
            let level = level.max(1);
            let changed = prog.level != level;
            prog.level = level;
            prog.xp = xp.min(u32::MAX as u64) as u32;
            if changed { prog.sync_sheet(); }
        }
    }
    if let Some((hp, max_hp)) = local_hp {
        if let Ok(mut prog) = local_prog.single_mut() {
            prog.hp = hp;
            prog.vitals.max_hp = max_hp;
        }
    }

    let Some(players) = latest else {
        return;
    };
    let local_id = net.local_player_id;

    let mut seen = HashMap::new();
    for state in &players {
        if Some(state.player_id) == local_id {
            continue;
        }
        seen.insert(state.player_id, state.clone());
    }

    for (entity, mut remote, _transform) in &mut remotes {
        if let Some(state) = seen.remove(&remote.player_id) {
            if remote.avatar_hash != state.appearance.glb_content_hash {
                commands.entity(entity).despawn();
                // Rebuild changed appearance with a fresh fallback/model.
                seen.insert(state.player_id,state);
                continue;
            }
            remote.target = Vec3::new(state.pose.x, state.pose.y, state.pose.z);
            remote.target_yaw = state.pose.yaw;
            if let Some(hash)=remote.avatar_hash.as_deref() {
                crate::avatar_delivery::prefetch(hash);
            }
        } else {
            commands.entity(entity).despawn();
        }
    }

    // Remote avatars use hash-verified local cache assets when present;
    // otherwise show a deliberately safe capsule placeholder.
    for (_id, state) in seen {
        spawn_remote_player(&mut commands, &mut meshes, &mut materials, &asset_server, &mut graphs, &state);
    }
}


/// Presentation interpolation for other players. Snap on genuine warps;
/// otherwise smooth from the latest server target without predicting AI.
pub fn interpolate_remote_players(
    time: Res<Time>,
    mut remotes: Query<(&RemotePlayer, &mut Transform)>,
) {
    let alpha = 1.0 - (-14.0 * time.delta_secs()).exp();
    for (remote, mut transform) in &mut remotes {
        let delta = transform.translation.distance(remote.target);
        transform.translation = if delta > 4.0 {
            remote.target
        } else {
            transform.translation.lerp(remote.target, alpha)
        };
        let target_rot = Quat::from_euler(EulerRot::YXZ, remote.target_yaw, 0.0, 0.0);
        transform.rotation = transform.rotation.slerp(target_rot, alpha);
    }
}


#[cfg(test)]
mod avatar_cache_tests {
    use super::*;

    #[test]
    fn invalid_or_injected_hash_cannot_form_asset_path() {
        assert!(verified_cache_uri("../../../../outside").is_none());
        assert!(verified_cache_uri("not-a-hash").is_none());
        assert!(verified_cache_uri(&"0".repeat(64)).is_none());
    }
}

/// When zone combat is verified, skip local boss simulation (dual-authority).
pub fn zone_online(net: Res<ZoneNet>) -> bool {
    net.online()
}

#[allow(dead_code)]
fn _tcp_ty(_: &TcpStream) {}
