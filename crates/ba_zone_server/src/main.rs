//! Authoritative Greymere zone — move + real-time melee using `ba_combat`.
//!
//! **Dev/loopback prototype.** Default bind is `127.0.0.1` only. Discord ticket
//! admission is deferred (see #14/#15). Do not expose this process on a public NIC.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use ba_combat::{
    ensure_kits_loaded, find_ability, kits, resolve_ability_potency, resolve_auto_attack,
    starter_max_hp, starter_stats, swing_period, AbilityKind, AbilitySpec, CombatStats, JobId,
    SwingOutcome, TargetRef, AUTO_REACH,
};
use ba_protocol::{
    integrate_movement, AppearanceWire, ClientMessage, MoveAxes, PlayerState, Pose, ServerMessage,
    DEV_ZONE_MAX_X, DEV_ZONE_MAX_Z, DEV_ZONE_MIN_X, DEV_ZONE_MIN_Z,
    PROTOCOL_VERSION, SPRINT_SPEED, TICK_HZ,
};
use futures_util::{SinkExt, StreamExt};
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, RwLock};
use tokio_tungstenite::tungstenite::Message;
use tracing::{info, warn};

/// Input is applied at fixed simulation cadence, never when packets arrive.
const INPUT_TIMEOUT: Duration = Duration::from_millis(250);
const INTEREST_RADIUS_METERS: f32 = 96.0;
const MAX_MOVE_PACKETS_PER_SEC: u32 = 80;
const BOUNDS: (f32, f32, f32, f32) = (DEV_ZONE_MIN_X, DEV_ZONE_MAX_X, DEV_ZONE_MIN_Z, DEV_ZONE_MAX_Z);
const DT: f32 = 1.0 / TICK_HZ as f32;
const DEFAULT_SPAWN: Pose = Pose {
    x: 0.2,
    y: 1.65,
    z: 6.4,
    yaw: 0.0,
};

#[derive(Clone, Copy)]
struct MovementCommand {
    seq: u64,
    forward: f32,
    strafe: f32,
    yaw: f32,
    sprint: bool,
    received_at: Instant,
}

#[derive(Clone)]
struct Player {
    appearance: AppearanceWire,
    pose: Pose,
    movement: Option<MovementCommand>,
    last_input_seq: u64,
    job: JobId,
    stats: CombatStats,
    hp: i32,
    max_hp: i32,
    mp: i32,
    max_mp: i32,
    swing_cd: f32,
    engage_target: Option<u32>,
    recast_ready: HashMap<String, Instant>,
    tp: i32,
    atk_mul: f32,
    atk_mul_until: Option<Instant>,
    move_packets_window: u32,
    move_window_start: Instant,
}

struct World {
    players: HashMap<u32, Player>,
    tick: u64,
    rng: SmallRng,
}

type SharedWorld = Arc<RwLock<World>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "ba_zone_server=info".into()),
        )
        .init();

    if ensure_kits_loaded() {
        if let Some(cat) = kits() {
            info!(
                "Loaded kits.json ({} abilities, {} weapon skills)",
                cat.ability_count(),
                cat.ws_count()
            );
        }
    } else {
        warn!("kits.json missing — using legacy hotbar fallback");
    }

    let addr = std::env::var("BA_ZONE_BIND").unwrap_or_else(|_| "127.0.0.1:8787".into());
    // This developer server has NO account authentication. Never permit a
    // public bind, even through an override flag. Discord ticket admission
    // is a prerequisite for an Internet-facing game server.
    let parsed: SocketAddr = addr.parse().map_err(|_| "BA_ZONE_BIND must be a numeric loopback socket address")?;
    if !parsed.ip().is_loopback() {
        return Err("Unauthenticated zone server may only bind to loopback".into());
    }
    let listener = TcpListener::bind(&addr).await?;
    info!(
        "ba_zone_server listening on ws://{addr} (protocol v{PROTOCOL_VERSION}, loopback prototype)"
    );

    let world: SharedWorld = Arc::new(RwLock::new(World {
        players: HashMap::new(),
        tick: 0,
        rng: SmallRng::from_os_rng(),
    }));
    let (snap_tx, _) = broadcast::channel::<String>(256);
    let next_id = Arc::new(AtomicU32::new(1));

    {
        let world = world.clone();
        let snap_tx = snap_tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_secs_f32(DT));
            loop {
                interval.tick().await;
                let events = {
                    let mut w = world.write().await;
                    w.tick = w.tick.wrapping_add(1);
                    tick_movement(&mut w, Instant::now());
                    tick_combat(&mut w)
                };
                for msg in events {
                    if let Ok(json) = msg.to_json() {
                        let _ = snap_tx.send(json);
                    }
                }

            }
        });
    }

    loop {
        let (stream, peer) = listener.accept().await?;
        if !peer.ip().is_loopback() {
            warn!("rejecting non-loopback peer {peer}");
            continue;
        }
        let world = world.clone();
        let snap_tx = snap_tx.clone();
        let next_id = next_id.clone();
        tokio::spawn(async move {
            if let Err(err) = handle_connection(stream, peer, world, snap_tx, next_id).await {
                warn!("{peer}: {err}");
            }
        });
    }
}

fn snapshot_message_for(w: &World, viewer_id: u32) -> ServerMessage {
    let origin = w.players.get(&viewer_id).map(|p| p.pose);
    let players: Vec<PlayerState> = w.players.iter().filter_map(|(id, p)| {
        if let Some(origin) = origin {
            if *id != viewer_id && dist_xz(origin, p.pose) > INTEREST_RADIUS_METERS {
                return None;
            }
        } else {
            return None;
        }
        Some(PlayerState {
            player_id: *id,
            pose: p.pose,
            appearance: p.appearance.clone(),
            hp: p.hp,
            max_hp: p.max_hp,
            last_input_seq: p.last_input_seq,
        })
    }).collect();
    ServerMessage::Snapshot { tick: w.tick, players }
}

/// Integrate player intent exactly once per authoritative zone tick.
fn tick_movement(w: &mut World, now: Instant) {
    for player in w.players.values_mut() {
        let Some(cmd) = player.movement else { continue };
        if player.hp <= 0 || now.duration_since(cmd.received_at) > INPUT_TIMEOUT {
            continue;
        }
        player.pose = integrate_movement(player.pose, MoveAxes { forward: cmd.forward, strafe: cmd.strafe, yaw: cmd.yaw, sprint: cmd.sprint }, DT);
        player.last_input_seq = cmd.seq;
    }
}

fn tick_combat(w: &mut World) -> Vec<ServerMessage> {
    let now = Instant::now();
    let ids: Vec<u32> = w.players.keys().copied().collect();
    for id in &ids {
        if let Some(p) = w.players.get_mut(id) {
            p.swing_cd = (p.swing_cd - DT).max(0.0);
            if p.atk_mul_until.is_some_and(|until| now >= until) {
                p.atk_mul = 1.0;
                p.atk_mul_until = None;
            }
        }
    }

    let mut events = Vec::new();
    for attacker_id in ids {
        let Some(engage) = w.players.get(&attacker_id).and_then(|p| p.engage_target) else {
            continue;
        };
        if w.players
            .get(&attacker_id)
            .map(|p| p.swing_cd > 0.0)
            .unwrap_or(true)
        {
            continue;
        }
        if w.players.get(&engage).map(|p| p.hp <= 0).unwrap_or(true) {
            if let Some(p) = w.players.get_mut(&attacker_id) {
                p.engage_target = None;
            }
            continue;
        }

        let (pose_a, job, stats, atk_mul) = {
            let a = match w.players.get(&attacker_id) {
                Some(p) => p,
                None => continue,
            };
            (a.pose, a.job, a.stats, a.atk_mul)
        };
        let (pose_t, def_stats) = {
            let t = match w.players.get(&engage) {
                Some(p) => p,
                None => continue,
            };
            (t.pose, t.stats)
        };
        let dist = dist_xz(pose_a, pose_t);
        if dist > AUTO_REACH {
            continue;
        }

        let target = TargetRef::from_defender(&def_stats, dist);
        let outcome = resolve_auto_attack(
            job,
            &stats,
            atk_mul,
            target,
            w.rng.random::<f32>(),
            w.rng.random::<f32>(),
        );
        let period = swing_period(0.0);
        if let Some(a) = w.players.get_mut(&attacker_id) {
            a.swing_cd = period;
        }

        let (damage, hit, crit, target_hp) = match outcome {
            SwingOutcome::OutOfRange => continue,
            SwingOutcome::Miss => {
                let hp = w.players.get(&engage).map(|p| p.hp).unwrap_or(0);
                (0, false, false, hp)
            }
            SwingOutcome::Hit { damage, crit } => {
                if let Some(a) = w.players.get_mut(&attacker_id) {
                    a.tp = (a.tp + if crit { 150 } else { 100 }).min(3000);
                }
                let hp = if let Some(t) = w.players.get_mut(&engage) {
                    t.hp = (t.hp - damage).max(0);
                    t.hp
                } else {
                    0
                };
                (damage, true, crit, hp)
            }
        };

        events.push(ServerMessage::Damage {
            tick: w.tick,
            attacker_id,
            target_id: engage,
            damage,
            hit,
            crit,
            target_hp,
        });

        if target_hp <= 0 {
            if let Some(a) = w.players.get_mut(&attacker_id) {
                a.engage_target = None;
            }
        }
    }
    events
}

async fn handle_connection(
    stream: TcpStream,
    peer: SocketAddr,
    world: SharedWorld,
    snap_tx: broadcast::Sender<String>,
    next_id: Arc<AtomicU32>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let ws = tokio_tungstenite::accept_async(stream).await?;
    let (mut write, mut read) = ws.split();
    let mut snap_rx = snap_tx.subscribe();
    let mut player_id: Option<u32> = None;
    // Per-viewer snapshots: remote actors outside AOI are never replicated.
    let mut snapshot_interval = tokio::time::interval(Duration::from_secs_f32(DT));
    snapshot_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    info!("{peer}: connected");

    loop {
        tokio::select! {
            _ = snapshot_interval.tick(), if player_id.is_some() => {
                if let Some(viewer_id) = player_id {
                    let snap = {
                        let w = world.read().await;
                        snapshot_message_for(&w, viewer_id)
                    };
                    write.send(Message::Text(snap.to_json()?.into())).await?;
                }
            }
            incoming = read.next() => {
                match incoming {
                    Some(Ok(Message::Text(text))) => {
                        match ClientMessage::from_json(&text) {
                            Ok(ClientMessage::Hello { protocol, appearance, pose: _client_pose }) => {
                                if player_id.is_some() {
                                    let reject = ServerMessage::Reject {
                                        reason: "duplicate hello".into(),
                                    };
                                    write.send(Message::Text(reject.to_json()?.into())).await?;
                                    continue;
                                }
                                if protocol != PROTOCOL_VERSION {
                                    let reject = ServerMessage::Reject {
                                        reason: format!(
                                            "protocol {protocol} != server {PROTOCOL_VERSION}"
                                        ),
                                    };
                                    write.send(Message::Text(reject.to_json()?.into())).await?;
                                    break;
                                }
                                if appearance.name.trim().is_empty()
                                    || !safe_character_id(&appearance.character_id)
                                {
                                    let reject = ServerMessage::Reject {
                                        reason: "name and safe characterId required".into(),
                                    };
                                    write.send(Message::Text(reject.to_json()?.into())).await?;
                                    break;
                                }
                                // Sanitize appearance: drop any path-like fields (wire uses hash only).
                                let mut appearance = appearance;
                                appearance.glb_content_hash = appearance
                                    .glb_content_hash
                                    .filter(|h| h.len() == 64 && h.chars().all(|c| c.is_ascii_hexdigit()));

                                let job = JobId::parse(&appearance.starting_job);
                                let stats = starter_stats(job);
                                let max_hp = starter_max_hp(job);
                                let max_mp = 80 + stats.mnd * 4 + stats.int * 2;
                                let id = next_id.fetch_add(1, Ordering::Relaxed);
                                // Server-chosen spawn (ignore client pose for admission).
                                let pose = DEFAULT_SPAWN;
                                {
                                    let mut w = world.write().await;
                                    w.players.insert(
                                        id,
                                        Player {
                                            appearance: appearance.clone(),
                                            pose,
                                            movement: None,
                                            last_input_seq: 0,
                                            job,
                                            stats,
                                            hp: max_hp,
                                            max_hp,
                                            mp: max_mp,
                                            max_mp,
                                            swing_cd: 0.0,
                                            engage_target: None,
                                            recast_ready: HashMap::new(),
                                            tp: 0,
                                            atk_mul: 1.0,
                                            atk_mul_until: None,
                                            move_packets_window: 0,
                                            move_window_start: Instant::now(),
                                        },
                                    );
                                }
                                player_id = Some(id);
                                let tick = world.read().await.tick;
                                let welcome = ServerMessage::Welcome {
                                    protocol: PROTOCOL_VERSION,
                                    player_id: id,
                                    tick,
                                    tick_hz: TICK_HZ,
                                    spawn: pose,
                                };
                                write.send(Message::Text(welcome.to_json()?.into())).await?;
                                info!("{peer}: hello as {} ({job:?}) id={id}", appearance.name);
                            }
                            Ok(ClientMessage::MoveIntent { seq, forward, strafe, yaw, sprint }) => {
                                let Some(id) = player_id else { continue };
                                if !forward.is_finite() || !strafe.is_finite() || !yaw.is_finite()
                                    || forward.abs() > 1.0 || strafe.abs() > 1.0
                                {
                                    continue;
                                }
                                let now = Instant::now();
                                let mut w = world.write().await;
                                let Some(p) = w.players.get_mut(&id) else { continue };
                                if now.duration_since(p.move_window_start) >= Duration::from_secs(1) {
                                    p.move_window_start = now;
                                    p.move_packets_window = 0;
                                }
                                if p.move_packets_window >= MAX_MOVE_PACKETS_PER_SEC {
                                    continue;
                                }
                                p.move_packets_window += 1;
                                if seq <= p.last_input_seq
                                    || p.movement.as_ref().is_some_and(|previous| seq <= previous.seq)
                                {
                                    continue; // reject duplicate / out-of-order input
                                }
                                // Keep only the most recent validated input; no movement here.
                                p.movement = Some(MovementCommand {
                                    seq, forward, strafe, yaw, sprint, received_at: now,
                                });
                            }
                            Ok(ClientMessage::Attack { target_id }) => {
                                let Some(attacker_id) = player_id else { continue };
                                let mut w = world.write().await;
                                let pose = match w.players.get(&attacker_id) {
                                    Some(p) => p.pose,
                                    None => continue,
                                };
                                let target = if target_id == 0 {
                                    nearest_other(&w.players, attacker_id, pose)
                                } else if w.players.contains_key(&target_id) {
                                    Some(target_id)
                                } else {
                                    None
                                };
                                if let Some(tid) = target {
                                    if let Some(p) = w.players.get_mut(&attacker_id) {
                                        p.engage_target = Some(tid);
                                    }
                                    info!("{peer}: engage -> {tid}");
                                }
                            }
                            Ok(ClientMessage::Cast { ability_id }) => {
                                let Some(caster_id) = player_id else { continue };
                                if let Some(msg) = resolve_cast(&world, caster_id, &ability_id).await
                                {
                                    if let Ok(json) = msg.to_json() {
                                        let _ = snap_tx.send(json);
                                    }
                                }
                            }
                            Ok(ClientMessage::Ping { client_time_ms }) => {
                                let now = SystemTime::now()
                                    .duration_since(UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_millis() as u64;
                                let pong = ServerMessage::Pong {
                                    client_time_ms,
                                    server_time_ms: now,
                                };
                                write.send(Message::Text(pong.to_json()?.into())).await?;
                            }
                            Err(err) => warn!("{peer}: bad client message: {err}"),
                        }
                    }
                    Some(Ok(Message::Ping(p))) => {
                        write.send(Message::Pong(p)).await?;
                    }
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(err)) => {
                        warn!("{peer}: ws error {err}");
                        break;
                    }
                }
            }
            snap = snap_rx.recv() => {
                match snap {
                    Ok(json) => {
                        if player_id.is_some() {
                            if write.send(Message::Text(json.into())).await.is_err() {
                                break;
                            }
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
        }
    }

    if let Some(id) = player_id {
        world.write().await.players.remove(&id);
        info!("{peer}: left (id={id})");
    }
    Ok(())
}

async fn resolve_cast(
    world: &SharedWorld,
    caster_id: u32,
    ability_id: &str,
) -> Option<ServerMessage> {
    let mut w = world.write().await;
    let now = Instant::now();
    let tick = w.tick;

    let (job, stats, mp, tp, pose, engage) = {
        let p = w.players.get(&caster_id)?;
        (p.job, p.stats, p.mp, p.tp, p.pose, p.engage_target)
    };
    let Some(spec) = find_ability(job, ability_id) else {
        return Some(ServerMessage::CastRejected {
            tick,
            ability_id: ability_id.to_string(),
            reason: "unknown_ability".into(),
        });
    };
    if let Some(ready) = w.players.get(&caster_id)?.recast_ready.get(ability_id) {
        if *ready > now {
            return Some(ServerMessage::CastRejected {
                tick,
                ability_id: spec.id.clone(),
                reason: "recast".into(),
            });
        }
    }
    if spec.mp > 0 && mp < spec.mp {
        return Some(ServerMessage::CastRejected {
            tick,
            ability_id: spec.id.clone(),
            reason: "mp".into(),
        });
    }
    if spec.kind == AbilityKind::WeaponSkill && tp < spec.tp_cost {
        return Some(ServerMessage::CastRejected {
            tick,
            ability_id: spec.id.clone(),
            reason: "tp".into(),
        });
    }

    if spec.kind == AbilityKind::Buff {
        {
            let p = w.players.get_mut(&caster_id)?;
            if let Some(mul) = spec.atk_mul {
                p.atk_mul = mul;
                p.atk_mul_until = Some(now + Duration::from_secs_f32(spec.duration.max(1.0)));
            }
            if spec.mp > 0 {
                p.mp = (p.mp - spec.mp).max(0);
            }
            p.recast_ready
                .insert(spec.id.clone(), now + Duration::from_secs_f32(spec.recast));
        }
        info!("cast buff {} on {caster_id} atk_mul={:?}", spec.id, spec.atk_mul);
        return Some(ServerMessage::BuffApplied {
            tick,
            caster_id,
            ability_id: spec.id.clone(),
            atk_mul: spec.atk_mul,
            duration_s: spec.duration,
        });
    }

    if matches!(spec.kind, AbilityKind::Heal) {
        let outcome = resolve_ability_potency(&stats, &spec, None);
        return apply_ability_outcome(&mut w, caster_id, caster_id, &spec, outcome, now, tick);
    }

    let Some(tid) = engage.or_else(|| nearest_other(&w.players, caster_id, pose)) else {
        return Some(ServerMessage::CastRejected {
            tick,
            ability_id: spec.id.clone(),
            reason: "no_target".into(),
        });
    };
    let tpose = w.players.get(&tid)?.pose;
    let dist = dist_xz(pose, tpose);
    let tstats = w.players.get(&tid)?.stats;
    let outcome =
        resolve_ability_potency(&stats, &spec, Some(TargetRef::from_defender(&tstats, dist)));
    if matches!(outcome, SwingOutcome::OutOfRange) {
        return Some(ServerMessage::CastRejected {
            tick,
            ability_id: spec.id.clone(),
            reason: "out_of_range".into(),
        });
    }
    apply_ability_outcome(&mut w, caster_id, tid, &spec, outcome, now, tick)
}

fn apply_ability_outcome(
    w: &mut World,
    caster_id: u32,
    target_id: u32,
    spec: &AbilitySpec,
    outcome: SwingOutcome,
    now: Instant,
    tick: u64,
) -> Option<ServerMessage> {
    // Range already validated by resolve_cast for damaging casts.
    if matches!(outcome, SwingOutcome::OutOfRange) {
        return Some(ServerMessage::CastRejected {
            tick,
            ability_id: spec.id.clone(),
            reason: "out_of_range".into(),
        });
    }

    {
        let p = w.players.get_mut(&caster_id)?;
        if spec.mp > 0 {
            p.mp = (p.mp - spec.mp).max(0);
        }
        if spec.kind == AbilityKind::WeaponSkill && spec.tp_cost > 0 {
            p.tp = (p.tp - spec.tp_cost).max(0);
        }
        p.recast_ready
            .insert(spec.id.clone(), now + Duration::from_secs_f32(spec.recast));
    }

    match outcome {
        SwingOutcome::OutOfRange => unreachable!(),
        SwingOutcome::Miss => Some(ServerMessage::Damage {
            tick,
            attacker_id: caster_id,
            target_id,
            damage: 0,
            hit: false,
            crit: false,
            target_hp: w.players.get(&target_id).map(|p| p.hp).unwrap_or(0),
        }),
        SwingOutcome::Hit { damage, crit: _ } if damage < 0 => {
            let amount = (-damage).max(0);
            let hp = if let Some(t) = w.players.get_mut(&target_id) {
                t.hp = (t.hp + amount).min(t.max_hp);
                t.hp
            } else {
                0
            };
            Some(ServerMessage::Heal {
                tick,
                caster_id,
                target_id,
                amount,
                target_hp: hp,
            })
        }
        SwingOutcome::Hit { damage, crit } => {
            let hp = if let Some(t) = w.players.get_mut(&target_id) {
                t.hp = (t.hp - damage).max(0);
                t.hp
            } else {
                0
            };
            Some(ServerMessage::Damage {
                tick,
                attacker_id: caster_id,
                target_id,
                damage,
                hit: true,
                crit,
                target_hp: hp,
            })
        }
    }
}

fn nearest_other(players: &HashMap<u32, Player>, self_id: u32, pose: Pose) -> Option<u32> {
    let mut best: Option<(u32, f32)> = None;
    for (id, p) in players {
        if *id == self_id || p.hp <= 0 {
            continue;
        }
        let d = dist_xz(pose, p.pose);
        if best.map(|(_, bd)| d < bd).unwrap_or(true) {
            best = Some((*id, d));
        }
    }
    best.map(|(id, _)| id)
}

fn safe_character_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 64
        && id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
        && id != "."
        && id != ".."
}

fn dist_xz(a: Pose, b: Pose) -> f32 {
    let dx = a.x - b.x;
    let dz = a.z - b.z;
    (dx * dx + dz * dz).sqrt()
}

fn clamp_pose(mut pose: Pose) -> Pose {
    if !pose.is_finite() {
        return DEFAULT_SPAWN;
    }
    let (min_x, max_x, min_z, max_z) = BOUNDS;
    pose.x = pose.x.clamp(min_x, max_x);
    pose.z = pose.z.clamp(min_z, max_z);
    pose.y = 1.65;
    pose
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_command(now: Instant, seq: u64) -> MovementCommand {
        MovementCommand { seq, forward: 1.0, strafe: 0.0, yaw: 0.0, sprint: true, received_at: now }
    }

    #[test]
    fn movement_is_bound_to_tick_not_packet_count() {
        let now = Instant::now();
        let cmd = test_command(now, 5000);
        let a = integrate_movement(DEFAULT_SPAWN, MoveAxes { forward: cmd.forward, strafe: cmd.strafe, yaw: cmd.yaw, sprint: cmd.sprint }, DT);
        assert!((a.z - DEFAULT_SPAWN.z).abs() <= SPRINT_SPEED * DT + 0.0001);
        let b = integrate_movement(a, MoveAxes { forward: cmd.forward, strafe: cmd.strafe, yaw: cmd.yaw, sprint: cmd.sprint }, DT);
        assert!((b.z - DEFAULT_SPAWN.z).abs() <= SPRINT_SPEED * DT * 2.0 + 0.0001);
        // 5000 inbound packets still correspond to just one latest input per tick.
    }

    #[test]
    fn interest_filter_rejects_out_of_range_players() {
        let origin = DEFAULT_SPAWN;
        let near = Pose { x: 20.0, ..origin };
        let far = Pose { x: INTEREST_RADIUS_METERS + 0.01, ..origin };
        assert!(dist_xz(origin, near) < INTEREST_RADIUS_METERS);
        assert!(dist_xz(origin, far) > INTEREST_RADIUS_METERS);
    }

    #[test]
    fn diagonal_input_is_normalized() {
        let mut cmd = test_command(Instant::now(), 1);
        cmd.strafe = 1.0;
        let next = integrate_movement(DEFAULT_SPAWN, MoveAxes { forward: cmd.forward, strafe: cmd.strafe, yaw: cmd.yaw, sprint: cmd.sprint }, DT);
        let d = ((next.x - DEFAULT_SPAWN.x).powi(2) + (next.z - DEFAULT_SPAWN.z).powi(2)).sqrt();
        assert!(d <= SPRINT_SPEED * DT + 0.0001);
    }

    #[test]
    fn rejects_non_finite_pose() {
        let bad = Pose {
            x: f32::NAN,
            y: 1.65,
            z: 0.0,
            yaw: 0.0,
        };
        assert!(!bad.is_finite());
        let clamped = clamp_pose(bad);
        assert!(clamped.is_finite());
    }
}
