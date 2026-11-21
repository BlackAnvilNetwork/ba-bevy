//! In-game note: typed update, place, facing, coordinates, and the pixel frame.
//! T opens the line. Enter writes `debug/inbox/<stamp>/` and queues it for this chat.
//! F3 shows the live readout. F8 saves the live view.
//!
//! Orbit capture is a one-shot the agent can fire while the game is running:
//! write `debug/inbox/orbit.request` (optional note in the file). The play
//! camera does not move. F7 is the same capture from the keyboard.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bevy::camera::RenderTarget;
use bevy::image::ImageSampler;
use bevy::input::keyboard::KeyboardInput;
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use serde::Serialize;

use crate::atmosphere::{self, StickAtmosphere};
use crate::camera_rig::{target_camera_transform, CameraMode, PlayerCamera};
use crate::data::GameData;
use crate::pixel_present::{PixelFrame, PIXEL_H, PIXEL_W};
use crate::player::{NoteComposer, Player};

#[derive(Resource, Default)]
pub struct DebugHud {
    pub visible: bool,
    pub notice: String,
    pub notice_until: f32,
}

#[derive(Component)]
pub(crate) struct DebugText;

#[derive(Component)]
pub(crate) struct NoteLine;

#[derive(Component)]
pub(crate) struct DebugOrbitCam;

#[derive(Resource, Clone)]
pub(crate) struct OrbitFrame(Handle<Image>);

#[derive(Resource)]
pub(crate) struct OrbitCapture {
    remaining: Vec<&'static str>,
    wait: u8,
    dir: PathBuf,
    inbox: bool,
    message: String,
    views: Vec<ShotView>,
    catalog: &'static [(&'static str, &'static str)],
}

impl Default for OrbitCapture {
    fn default() -> Self {
        Self {
            remaining: Vec::new(),
            wait: 0,
            dir: PathBuf::new(),
            inbox: false,
            message: String::new(),
            views: Vec::new(),
            catalog: ORBIT_VIEWS,
        }
    }
}

#[derive(Resource, Default)]
pub(crate) struct GripSweep {
    queue: Vec<String>,
    settle: u8,
    current: String,
}

#[derive(Clone, Serialize)]
pub(crate) struct ShotView {
    id: String,
    file: String,
    label: String,
}

const ORBIT_VIEWS: &[(&str, &str)] = &[
    ("fp", "first person"),
    ("3p_behind", "third person behind"),
    ("3p_front", "third person front"),
    ("3p_left", "third person left"),
    ("3p_right", "third person right"),
    ("3p_high", "high three-quarter"),
    ("3p_hands", "close on the hands"),
];

const GRIP_VIEWS: &[(&str, &str)] = &[
    ("fp_grip", "first person looking at the grip"),
    ("hands_front", "close three-quarter on both hands"),
    ("3p_front", "third person front"),
];

const GRIP_REQUEST: &str = "debug/inbox/grip.request";
const GRIP_RUN_ALL: &str = "debug/grip_estimates/run_all.request";
const GRIP_DIR: &str = "debug/grip_estimates";

#[derive(Serialize)]
struct ShotPackage {
    message: String,
    location: String,
    zone_id: String,
    direction: ShotDirection,
    coordinates: ShotCoordinates,
    screenshot: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    views: Vec<ShotView>,
}

#[derive(Serialize)]
struct ShotDirection {
    compass: String,
    yaw_degrees: f32,
    pitch_degrees: f32,
    forward: [f32; 3],
}

#[derive(Serialize)]
struct ShotCoordinates {
    x: f32,
    y: f32,
    z: f32,
}

pub fn spawn_debug_hud(mut commands: Commands) {
    commands.init_resource::<DebugHud>();
    commands.init_resource::<NoteComposer>();
    commands.init_resource::<OrbitCapture>();
    commands.init_resource::<GripSweep>();
    commands
        .spawn((
            Node {
                position_type: PositionType::Absolute,
                top: px(16),
                right: px(18),
                max_width: px(360),
                ..default()
            },
            DebugText,
            Text::new(""),
            TextFont::from_font_size(15.0),
            TextColor(Color::srgb(0.78, 0.9, 0.72)),
        ));
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            bottom: px(108),
            left: px(24),
            right: px(24),
            ..default()
        },
        NoteLine,
        Text::new(""),
        TextFont::from_font_size(18.0),
        TextColor(Color::srgb(0.93, 0.86, 0.62)),
    ));
}

pub fn toggle_debug_hud(keys: Res<ButtonInput<KeyCode>>, mut hud: ResMut<DebugHud>) {
    if keys.just_pressed(KeyCode::F3) {
        hud.visible = !hud.visible;
    }
}

pub fn update_debug_hud(
    hud: Res<DebugHud>,
    time: Res<Time>,
    data: Res<GameData>,
    player: Query<&Transform, With<Player>>,
    mut text: Query<&mut Text, With<DebugText>>,
) {
    let Ok(mut text) = text.single_mut() else {
        return;
    };
    if !hud.visible && time.elapsed_secs() > hud.notice_until {
        **text = String::new().into();
        return;
    }
    let Ok(transform) = player.single() else {
        return;
    };
    let pose = describe(&data, transform);
    let mut lines = String::new();
    if hud.visible {
        lines.push_str(&format!(
            "{}\n{:0.2}, {:0.2}, {:0.2}\n{}  yaw {:0.0}°  pitch {:0.0}°\nT note · Enter sends · F8 live · orbit.request",
            pose.location,
            pose.coordinates.x,
            pose.coordinates.y,
            pose.coordinates.z,
            pose.direction.compass,
            pose.direction.yaw_degrees,
            pose.direction.pitch_degrees,
        ));
        if let Some(swing) = crate::character::swing_overlay_label() {
            lines.push_str(&format!(
                "\n2H swing {swing}  debug/swing_estimates/current.json"
            ));
        }
    }
    if time.elapsed_secs() <= hud.notice_until && !hud.notice.is_empty() {
        if !lines.is_empty() {
            lines.push('\n');
        }
        lines.push_str(&hud.notice);
    }
    **text = lines.into();
}

pub fn capture_debug_shot(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    data: Res<GameData>,
    frame: Option<Res<PixelFrame>>,
    player: Query<&Transform, With<Player>>,
    mut hud: ResMut<DebugHud>,
    mut auto_done: Local<bool>,
) {
    let auto = !*auto_done && std::env::var_os("BA_SHOT").is_some() && time.elapsed_secs() > 3.5;
    if !keys.just_pressed(KeyCode::F8) && !auto {
        return;
    }
    *auto_done = true;
    let Some(frame) = frame else {
        return;
    };
    let Ok(transform) = player.single() else {
        return;
    };

    let stamp = shot_stamp();
    let dir = PathBuf::from("debug").join("shots").join(&stamp);
    if let Err(err) = fs::create_dir_all(&dir) {
        hud.notice = format!("shot failed: {err}");
        hud.notice_until = time.elapsed_secs() + 4.0;
        return;
    }

    let mut pose = describe(&data, transform);
    pose.message.clear();
    pose.screenshot = "frame.png".into();
    let json_path = dir.join("shot.json");
    match serde_json::to_string_pretty(&pose) {
        Ok(body) => {
            if let Err(err) = fs::write(&json_path, body) {
                hud.notice = format!("shot failed: {err}");
                hud.notice_until = time.elapsed_secs() + 4.0;
                return;
            }
        }
        Err(err) => {
            hud.notice = format!("shot failed: {err}");
            hud.notice_until = time.elapsed_secs() + 4.0;
            return;
        }
    }

    let png = dir.join("frame.png");
    commands
        .spawn(Screenshot::image(frame.0.clone()))
        .observe(save_to_disk(png));
    hud.notice = format!("saved debug/shots/{stamp}");
    hud.notice_until = time.elapsed_secs() + 4.0;
    hud.visible = true;
    info!("debug shot {}", dir.display());
}

fn describe(data: &GameData, transform: &Transform) -> ShotPackage {
    let position = transform.translation;
    let (yaw, pitch, _) = transform.rotation.to_euler(EulerRot::YXZ);
    let forward = transform.forward().as_vec3();
    let place = nearest_place(data, position);
    ShotPackage {
        message: String::new(),
        location: format!("{} — {}", data.zone.display_name, place),
        zone_id: data.zone.zone_id.clone(),
        direction: ShotDirection {
            compass: compass(forward).into(),
            yaw_degrees: yaw.to_degrees(),
            pitch_degrees: pitch.to_degrees(),
            forward: [forward.x, forward.y, forward.z],
        },
        coordinates: ShotCoordinates {
            x: position.x,
            y: position.y,
            z: position.z,
        },
        screenshot: String::new(),
        views: Vec::new(),
    }
}

fn nearest_place(data: &GameData, position: Vec3) -> String {
    for exit in &data.zone.exits {
        if position.x >= exit.min[0]
            && position.x <= exit.max[0]
            && position.z >= exit.min[1]
            && position.z <= exit.max[1]
        {
            return "chalk line".into();
        }
    }
    let mut on_path = false;
    let mut best_name: Option<&str> = None;
    let mut best_dist = 6.0_f32;
    for prop in &data.zone.props {
        if prop.model.starts_with("primitive:") {
            continue;
        }
        let dx = position.x - prop.at[0];
        let dz = position.z - prop.at[2];
        let dist = (dx * dx + dz * dz).sqrt();
        if prop.name.starts_with("Path") && dist < 1.4 {
            on_path = true;
            continue;
        }
        if !is_landmark(&prop.name) {
            continue;
        }
        if dist < best_dist {
            best_dist = dist;
            best_name = Some(prop.name.as_str());
        }
    }
    match best_name {
        Some(name) if best_dist < 3.0 => format!("by {name}"),
        _ if on_path => "on the path".into(),
        Some(name) => format!("near {name}"),
        None => "open ground".into(),
    }
}

fn is_landmark(name: &str) -> bool {
    !name.starts_with("Path")
        && !name.starts_with("Cloister")
        && !name.starts_with("Line")
        && !name.starts_with("Chalk")
        && !name.starts_with("Rear")
        && !name.starts_with("Fallen")
        && !name.starts_with("Pine")
        && !name.starts_with("Tree")
        && !name.starts_with("GraveFence")
}

fn compass(forward: Vec3) -> &'static str {
    // North is -Z, east is +X, matching the yard map.
    let degrees = forward.x.atan2(-forward.z).to_degrees();
    let index = ((degrees + 22.5).rem_euclid(360.0) / 45.0) as usize % 8;
    ["N", "NE", "E", "SE", "S", "SW", "W", "NW"][index]
}

pub fn type_note(
    mut keys_in: MessageReader<KeyboardInput>,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    data: Res<GameData>,
    frame: Option<Res<PixelFrame>>,
    player: Query<&Transform, With<Player>>,
    mut composer: ResMut<NoteComposer>,
    mut hud: ResMut<DebugHud>,
    mut line: Query<&mut Text, With<NoteLine>>,
    mut commands: Commands,
    mut auto_sent: Local<bool>,
) {
    let mut opened = false;
    if !composer.active && keys.just_pressed(KeyCode::KeyT) {
        composer.active = true;
        composer.text.clear();
        opened = true;
    }

    if !*auto_sent && time.elapsed_secs() > 4.5 {
        *auto_sent = true;
        if let Some(text) = std::env::var("BA_NOTE").ok().filter(|s| !s.trim().is_empty()) {
            composer.text = text;
            send_note(
                &mut commands,
                &frame,
                &player,
                &data,
                &mut composer,
                &mut hud,
                time.elapsed_secs(),
            );
        }
    }

    if composer.active {
        let mut submit = false;
        let mut cancel = false;
        for msg in keys_in.read() {
            if msg.state != ButtonState::Pressed {
                continue;
            }
            if opened && msg.key_code == KeyCode::KeyT {
                continue;
            }
            match msg.key_code {
                KeyCode::Enter | KeyCode::NumpadEnter => submit = true,
                KeyCode::F7 | KeyCode::F8 | KeyCode::F3 => {}
                KeyCode::Escape => cancel = true,
                KeyCode::Backspace => {
                    composer.text.pop();
                }
                _ => {
                    if let Some(text) = &msg.text {
                        let piece = text.as_str();
                        if !piece.is_empty() && piece.chars().all(|c| !c.is_control()) {
                            for ch in piece.chars() {
                                if composer.text.chars().count() >= 240 {
                                    break;
                                }
                                composer.text.push(ch);
                            }
                        }
                    }
                }
            }
        }
        if cancel {
            composer.active = false;
            composer.text.clear();
        } else if submit {
            send_note(
                &mut commands,
                &frame,
                &player,
                &data,
                &mut composer,
                &mut hud,
                time.elapsed_secs(),
            );
        }
    }

    let Ok(mut text) = line.single_mut() else {
        return;
    };
    if composer.active {
        **text = format!(
            "> {}_    Enter sends · Esc cancels",
            composer.text
        )
        .into();
    } else if time.elapsed_secs() <= composer.sent_until {
        **text = "sent. keep walking.".into();
    } else {
        **text = String::new().into();
    }
}

fn send_note(
    commands: &mut Commands,
    frame: &Option<Res<PixelFrame>>,
    player: &Query<&Transform, With<Player>>,
    data: &GameData,
    composer: &mut NoteComposer,
    hud: &mut DebugHud,
    now: f32,
) {
    let message = {
        let trimmed = composer.text.trim();
        if trimmed.is_empty() {
            "(picture only)".to_string()
        } else {
            trimmed.to_string()
        }
    };
    composer.active = false;
    composer.text.clear();
    composer.sent_until = now + 3.0;

    let Some(frame) = frame else {
        hud.notice = "note failed: no picture yet".into();
        hud.notice_until = now + 4.0;
        return;
    };
    let Ok(transform) = player.single() else {
        hud.notice = "note failed: no player".into();
        hud.notice_until = now + 4.0;
        return;
    };

    let stamp = shot_stamp();
    let dir = PathBuf::from("debug").join("inbox").join(&stamp);
    if let Err(err) = fs::create_dir_all(&dir) {
        hud.notice = format!("note failed: {err}");
        hud.notice_until = now + 4.0;
        return;
    }

    let mut pose = describe(data, transform);
    pose.message = message.clone();
    pose.screenshot = "frame.png".into();
    let body = match serde_json::to_string_pretty(&pose) {
        Ok(body) => body,
        Err(err) => {
            hud.notice = format!("note failed: {err}");
            hud.notice_until = now + 4.0;
            return;
        }
    };
    if let Err(err) = fs::write(dir.join("shot.json"), &body) {
        hud.notice = format!("note failed: {err}");
        hud.notice_until = now + 4.0;
        return;
    }
    let note_body = format!(
        "{message}\n\n{}\n{:0.2}, {:0.2}, {:0.2}\n{}  yaw {:0.0}°  pitch {:0.0}°\n",
        pose.location,
        pose.coordinates.x,
        pose.coordinates.y,
        pose.coordinates.z,
        pose.direction.compass,
        pose.direction.yaw_degrees,
        pose.direction.pitch_degrees,
    );
    if let Err(err) = fs::write(dir.join("note.txt"), note_body) {
        hud.notice = format!("note failed: {err}");
        hud.notice_until = now + 4.0;
        return;
    }

    let png = dir.join("frame.png");
    commands
        .spawn(Screenshot::image(frame.0.clone()))
        .observe(save_to_disk(png));
    let queued = dir.clone();
    thread::spawn(move || enqueue_when_ready(queued));

    hud.notice = format!("sent {stamp}");
    hud.notice_until = now + 4.0;
    hud.visible = true;
    info!("note queued {}", dir.display());
}

fn enqueue_when_ready(dir: PathBuf) {
    let png = dir.join("frame.png");
    for _ in 0..80 {
        if let Ok(meta) = png.metadata() {
            if meta.len() > 32 {
                break;
            }
        }
        thread::sleep(Duration::from_millis(50));
    }
    let queue = PathBuf::from("debug").join("inbox").join("queue.txt");
    if let Some(parent) = queue.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut file) = OpenOptions::new().create(true).append(true).open(queue) {
        let _ = writeln!(file, "{}", dir.display());
    }
}

pub fn spawn_orbit_camera(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    atmo: Res<StickAtmosphere>,
    player: Query<Entity, With<Player>>,
    existing: Query<(), With<DebugOrbitCam>>,
) {
    if !existing.is_empty() {
        return;
    }
    let Ok(player) = player.single() else {
        return;
    };
    let mut target =
        Image::new_target_texture(PIXEL_W, PIXEL_H, TextureFormat::Rgba8UnormSrgb, None);
    target.sampler = ImageSampler::nearest();
    let handle = images.add(target);
    let cam = commands
        .spawn((
            DebugOrbitCam,
            atmosphere::camera_bundle(&atmo),
            orbit_camera_local("3p_behind"),
        ))
        .id();
    commands.entity(cam).insert((
        Msaa::Off,
        RenderTarget::Image(handle.clone().into()),
        Camera {
            is_active: false,
            order: -1,
            clear_color: ClearColorConfig::Custom(atmosphere::hex_rgb(atmosphere::SKY_TOP)),
            ..default()
        },
    ));
    commands.entity(player).add_child(cam);
    commands.insert_resource(OrbitFrame(handle));
}

/// Player-local camera for a named orbit view. The play camera is not moved.
pub(crate) fn orbit_camera_local(id: &str) -> Transform {
    let chest = Vec3::new(0.0, -0.45, 0.0);
    match id {
        // Look down the real arms so a grip check is visible without a keypress.
        "fp" => target_camera_transform(CameraMode::FirstPerson, -1.12),
        "3p_behind" => look_from(Vec3::new(0.0, 0.55, 3.4), chest),
        "3p_front" => look_from(Vec3::new(0.0, 0.45, -3.4), chest),
        "3p_left" => look_from(Vec3::new(-3.2, 0.45, 0.4), chest),
        "3p_right" => look_from(Vec3::new(3.2, 0.45, 0.4), chest),
        "3p_high" => look_from(Vec3::new(2.2, 2.4, 2.8), chest),
        "3p_hands" => look_from(Vec3::new(-1.5, 0.10, -0.5), Vec3::new(-0.08, -0.48, 0.05)),
        // Same eye point as play first-person, aimed at the 2H hands instead
        // of a steep look-down that only hits forest floor.
        "fp_grip" => look_from(Vec3::new(0.0, -0.12, -0.24), Vec3::new(-0.08, -0.48, -0.62)),
        "hands_front" => look_from(Vec3::new(-1.45, 0.08, -0.55), Vec3::new(-0.08, -0.48, 0.08)),
        _ => look_from(Vec3::new(0.0, 0.55, 3.4), chest),
    }
}

fn look_from(eye: Vec3, at: Vec3) -> Transform {
    Transform::from_translation(eye).looking_at(at, Vec3::Y)
}

pub(crate) const ORBIT_REQUEST: &str = "debug/inbox/orbit.request";

fn take_request_file(path: &str) -> Option<String> {
    let path = PathBuf::from(path);
    let body = fs::read_to_string(&path).ok()?;
    let _ = fs::remove_file(&path);
    Some(body)
}

fn take_orbit_request() -> Option<String> {
    take_request_file(ORBIT_REQUEST)
}

fn grip_estimate_ids() -> Vec<String> {
    (1..=20)
        .map(|i| format!("{i:02}"))
        .filter(|id| PathBuf::from(GRIP_DIR).join(format!("{id}.json")).is_file())
        .collect()
}

fn activate_grip_estimate(id: &str) -> bool {
    let src = PathBuf::from(GRIP_DIR).join(format!("{id}.json"));
    let dst = PathBuf::from(GRIP_DIR).join("current.json");
    fs::copy(src, dst).is_ok()
}

fn begin_orbit(
    capture: &mut OrbitCapture,
    inbox: bool,
    message: String,
    catalog: &'static [(&'static str, &'static str)],
    dir: PathBuf,
) -> bool {
    if !capture.remaining.is_empty() {
        return false;
    }
    if fs::create_dir_all(&dir).is_err() {
        return false;
    }
    capture.remaining = catalog.iter().map(|(id, _)| *id).collect();
    capture.wait = 3;
    capture.dir = dir;
    capture.inbox = inbox;
    capture.message = message;
    capture.views.clear();
    capture.catalog = catalog;
    true
}

fn begin_orbit_stamp(
    capture: &mut OrbitCapture,
    inbox: bool,
    message: String,
    catalog: &'static [(&'static str, &'static str)],
) -> bool {
    let stamp = shot_stamp();
    let folder = if inbox { "inbox" } else { "shots" };
    let dir = PathBuf::from("debug").join(folder).join(&stamp);
    begin_orbit(capture, inbox, message, catalog, dir)
}

pub fn capture_orbit_shots(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    data: Res<GameData>,
    frame: Option<Res<OrbitFrame>>,
    player: Query<&Transform, (With<Player>, Without<DebugOrbitCam>, Without<PlayerCamera>)>,
    mut hud: ResMut<DebugHud>,
    mut composer: ResMut<NoteComposer>,
    mut capture: ResMut<OrbitCapture>,
    mut sweep: ResMut<GripSweep>,
    mut orbit: Query<
        (&mut Transform, &mut Camera, &mut Projection),
        (With<DebugOrbitCam>, Without<Player>, Without<PlayerCamera>),
    >,
    mut auto_done: Local<bool>,
) {
    let auto = !*auto_done && std::env::var_os("BA_ORBIT_SHOT").is_some() && time.elapsed_secs() > 4.0;
    if capture.remaining.is_empty() {
        if take_request_file(GRIP_RUN_ALL).is_some() {
            sweep.queue = grip_estimate_ids();
            sweep.settle = 0;
            sweep.current.clear();
            hud.notice = format!("grip sweep {} estimates", sweep.queue.len());
            hud.notice_until = time.elapsed_secs() + 8.0;
            hud.visible = true;
        }
        if sweep.settle > 0 {
            sweep.settle -= 1;
            hud.notice = format!("grip {} settling", sweep.current);
            hud.notice_until = time.elapsed_secs() + 4.0;
            return;
        }
        if !sweep.current.is_empty() && sweep.settle == 0 {
            let id = sweep.current.clone();
            let dir = PathBuf::from(GRIP_DIR).join("shots").join(&id);
            if begin_orbit(&mut capture, false, id.clone(), GRIP_VIEWS, dir) {
                hud.notice = format!("grip {id} capturing");
                hud.notice_until = time.elapsed_secs() + 6.0;
                hud.visible = true;
            } else {
                sweep.current.clear();
                return;
            }
        } else if !sweep.queue.is_empty() {
            let id = sweep.queue.remove(0);
            if activate_grip_estimate(&id) {
                sweep.current = id.clone();
                sweep.settle = 18;
                hud.notice = format!("grip {id} loaded");
                hud.notice_until = time.elapsed_secs() + 4.0;
            } else {
                hud.notice = format!("grip {id} missing");
                hud.notice_until = time.elapsed_secs() + 4.0;
            }
            return;
        }

        if !capture.remaining.is_empty() {
            // sweep just started a capture
        } else if let Some(body) = take_request_file(GRIP_REQUEST) {
            let text = body.trim();
            let message = if text.is_empty() {
                "(grip)".into()
            } else {
                text.to_string()
            };
            if !begin_orbit_stamp(&mut capture, true, message, GRIP_VIEWS) {
                hud.notice = "grip failed: could not create folder".into();
                hud.notice_until = time.elapsed_secs() + 4.0;
                return;
            }
            hud.notice = "grip: posing…".into();
            hud.notice_until = time.elapsed_secs() + 8.0;
            hud.visible = true;
        } else {
            let requested = take_orbit_request();
            let f7 = keys.just_pressed(KeyCode::F7);
            if requested.is_none() && !f7 && !auto {
                return;
            }
            if auto {
                *auto_done = true;
            }
            let from_file = requested.is_some();
            let inbox = from_file || composer.active;
            let message = if let Some(body) = requested {
                let text = body.trim().to_string();
                if text.is_empty() {
                    "(orbit)".into()
                } else {
                    text
                }
            } else if composer.active {
                let text = composer.text.trim().to_string();
                composer.active = false;
                composer.text.clear();
                composer.sent_until = time.elapsed_secs() + 3.0;
                if text.is_empty() {
                    "(orbit)".into()
                } else {
                    text
                }
            } else {
                String::new()
            };
            if !begin_orbit_stamp(&mut capture, inbox, message, ORBIT_VIEWS) {
                hud.notice = "orbit failed: could not create folder".into();
                hud.notice_until = time.elapsed_secs() + 4.0;
                return;
            }
            hud.notice = "orbit: posing…".into();
            hud.notice_until = time.elapsed_secs() + 8.0;
            hud.visible = true;
        }
    }

    let Some(frame) = frame else {
        return;
    };
    let Ok((mut tf, mut cam, mut projection)) = orbit.single_mut() else {
        return;
    };
    let Some(&id) = capture.remaining.first() else {
        cam.is_active = false;
        return;
    };
    *tf = orbit_camera_local(id);
    cam.is_active = true;
    if let Projection::Perspective(ref mut perspective) = *projection {
        let first = id == "fp" || id == "fp_grip";
        let hands = id == "3p_hands" || id == "hands_front";
        perspective.near = if first || hands { 0.02 } else { 0.1 };
        perspective.fov = if first {
            70.0_f32.to_radians()
        } else if hands {
            42.0_f32.to_radians()
        } else {
            75.0_f32.to_radians()
        };
    }
    if capture.wait > 0 {
        capture.wait -= 1;
        let done = capture.views.len();
        let total = capture.catalog.len();
        hud.notice = format!("orbit {done}/{total} {id}");
        hud.notice_until = time.elapsed_secs() + 6.0;
        return;
    }

    let file = format!("{id}.png");
    let png = capture.dir.join(&file);
    commands
        .spawn(Screenshot::image(frame.0.clone()))
        .observe(save_to_disk(png));
    if id == "3p_behind" {
        commands
            .spawn(Screenshot::image(frame.0.clone()))
            .observe(save_to_disk(capture.dir.join("frame.png")));
    }
    let label = capture
        .catalog
        .iter()
        .find(|(view, _)| *view == id)
        .map(|(_, label)| *label)
        .unwrap_or(id);
    capture.views.push(ShotView {
        id: id.into(),
        file,
        label: label.into(),
    });
    capture.remaining.remove(0);
    capture.wait = 2;

    if !capture.remaining.is_empty() {
        return;
    }

    cam.is_active = false;
    let Ok(transform) = player.single() else {
        hud.notice = "orbit failed: no player".into();
        hud.notice_until = time.elapsed_secs() + 4.0;
        return;
    };
    let mut pose = describe(&data, transform);
    pose.message = capture.message.clone();
    pose.screenshot = "frame.png".into();
    pose.views = capture.views.clone();
    let stamp = capture
        .dir
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("orbit")
        .to_string();
    if let Ok(body) = serde_json::to_string_pretty(&pose) {
        let _ = fs::write(capture.dir.join("shot.json"), body);
    }
    if capture.inbox {
        let note_body = format!(
            "{}\n\n{}\n{:0.2}, {:0.2}, {:0.2}\n{}  yaw {:0.0}°  pitch {:0.0}°\norbit: {}\n",
            pose.message,
            pose.location,
            pose.coordinates.x,
            pose.coordinates.y,
            pose.coordinates.z,
            pose.direction.compass,
            pose.direction.yaw_degrees,
            pose.direction.pitch_degrees,
            pose.views
                .iter()
                .map(|v| v.id.as_str())
                .collect::<Vec<_>>()
                .join(", "),
        );
        let _ = fs::write(capture.dir.join("note.txt"), note_body);
        let queued = capture.dir.clone();
        thread::spawn(move || enqueue_when_ready(queued));
        hud.notice = format!("sent orbit {stamp}");
    } else if !sweep.current.is_empty() {
        hud.notice = format!("grip {} saved", sweep.current);
        info!("grip estimate shot {}", capture.dir.display());
        sweep.current.clear();
        if sweep.queue.is_empty() {
            let _ = fs::write(PathBuf::from(GRIP_DIR).join("sweep.done"), "ok");
            hud.notice = "grip sweep done".into();
        }
    } else {
        hud.notice = format!("saved debug/shots/{stamp} ({} views)", pose.views.len());
    }
    hud.notice_until = time.elapsed_secs() + 5.0;
    hud.visible = true;
    info!("orbit shot {}", capture.dir.display());
    capture.remaining.clear();
    capture.views.clear();
}

fn shot_stamp() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let day = secs / 86_400;
    let clock = secs % 86_400;
    format!(
        "{day}-{:02}{:02}{:02}",
        clock / 3600,
        (clock / 60) % 60,
        clock % 60
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn orbit_request_is_a_file_not_a_key() {
        assert_eq!(ORBIT_REQUEST, "debug/inbox/orbit.request");
        assert!(
            !ORBIT_REQUEST.contains("shots"),
            "agent-triggered orbit must land where the inbox watcher already looks"
        );
    }

    #[test]
    fn orbit_views_cover_first_person_and_a_ring_of_third_person() {
        let ids: Vec<&str> = ORBIT_VIEWS.iter().map(|(id, _)| *id).collect();
        assert_eq!(
            ids,
            vec![
                "fp", "3p_behind", "3p_front", "3p_left", "3p_right", "3p_high", "3p_hands"
            ]
        );
        let fp = orbit_camera_local("fp");
        let behind = orbit_camera_local("3p_behind");
        let front = orbit_camera_local("3p_front");
        let left = orbit_camera_local("3p_left");
        let right = orbit_camera_local("3p_right");
        let high = orbit_camera_local("3p_high");
        let hands = orbit_camera_local("3p_hands");
        assert!(
            fp.translation.z < -0.20,
            "first person stays on the face, got {}",
            fp.translation
        );
        assert!(
            behind.translation.z > 3.0,
            "behind should sit on +Z, got {}",
            behind.translation
        );
        assert!(
            front.translation.z < -3.0,
            "front should sit ahead of the player, got {}",
            front.translation
        );
        assert!(
            left.translation.x < -3.0 && right.translation.x > 3.0,
            "left/right should flank the player, left {} right {}",
            left.translation,
            right.translation
        );
        assert!(
            high.translation.y > behind.translation.y + 1.0,
            "high three-quarter must look down from above, high {} behind {}",
            high.translation,
            behind.translation
        );
        assert!(
            hands.translation.length() < behind.translation.length(),
            "hands close-up must be nearer than the full-body behind shot"
        );
    }
}
