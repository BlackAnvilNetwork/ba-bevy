//! In-game note: typed update, place, facing, coordinates, and the pixel frame.
//! T opens the line. Enter writes `debug/inbox/<stamp>/` and queues it for this chat.
//! F3 shows the live readout. F8 saves a shot without sending.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bevy::input::keyboard::KeyboardInput;
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::render::view::screenshot::{save_to_disk, Screenshot};
use serde::Serialize;

use crate::data::GameData;
use crate::pixel_present::PixelFrame;
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

#[derive(Serialize)]
struct ShotPackage {
    message: String,
    location: String,
    zone_id: String,
    direction: ShotDirection,
    coordinates: ShotCoordinates,
    screenshot: String,
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
            "{}\n{:0.2}, {:0.2}, {:0.2}\n{}  yaw {:0.0}°  pitch {:0.0}°\nT note · Enter sends it · F8 saves a shot",
            pose.location,
            pose.coordinates.x,
            pose.coordinates.y,
            pose.coordinates.z,
            pose.direction.compass,
            pose.direction.yaw_degrees,
            pose.direction.pitch_degrees,
        ));
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
