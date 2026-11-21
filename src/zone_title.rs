//! Gold place name at the top of the screen when the ground under you changes.
use crate::forge_menu::GameFlow;
use crate::net::ZoneNet;
use crate::player::Player;
use crate::stillwater::place_name;
use bevy::prelude::*;

const HOLD: f32 = 4.;
const FADE: f32 = 0.7;

#[derive(Component)]
pub(crate) struct ZoneTitle;

#[derive(Resource, Default)]
pub(crate) struct ZoneAnnounce {
    shown: Option<&'static str>,
    until: f32,
    pending: Option<&'static str>,
    pending_at: f32,
    last: Option<Vec2>,
}

pub fn spawn(mut commands: Commands, assets: Res<AssetServer>) {
    commands.spawn((
        ZoneTitle,
        Text::new(""),
        TextFont::from_font_size(52.0)
            .with_font(assets.load("fonts/Saltblackcombat-Regular.ttf")),
        TextLayout::justify(Justify::Center),
        TextColor(Color::srgb(0.96, 0.78, 0.34)),
        TextShadow {
            offset: Vec2::new(0., 3.),
            color: Color::srgba(0.07, 0.04, 0.02, 0.9),
        },
        Node {
            position_type: PositionType::Absolute,
            top: px(36.),
            width: percent(100.),
            justify_content: JustifyContent::Center,
            ..default()
        },
        ZIndex(30),
        Visibility::Hidden,
    ));
    commands.insert_resource(ZoneAnnounce::default());
}

pub fn tick(
    time: Res<Time>,
    flow: Res<GameFlow>,
    net: Res<ZoneNet>,
    player: Query<&Transform, With<Player>>,
    mut announce: ResMut<ZoneAnnounce>,
    mut title: Query<(&mut Text, &mut TextColor, &mut TextShadow, &mut Visibility), With<ZoneTitle>>,
) {
    let Ok((mut text, mut color, mut shadow, mut vis)) = title.single_mut() else {
        return;
    };
    if flow.in_menu() {
        *announce = ZoneAnnounce::default();
        *vis = Visibility::Hidden;
        return;
    }
    // Online play stands in the yard until the server sends a welcome. Hold
    // the title until that placement, so it names the zone you arrive in.
    if net.enabled && !net.online() {
        announce.shown = None;
        announce.pending = None;
        announce.last = None;
        *vis = Visibility::Hidden;
        return;
    }
    let Ok(tf) = player.single() else {
        return;
    };
    let here = Vec2::new(tf.translation.x, tf.translation.z);
    let now = time.elapsed_secs();
    let name = titled(place_name(here.x, here.y));
    let jumped = announce
        .last
        .map(|p| p.distance(here) > 18.)
        .unwrap_or(true);
    announce.last = Some(here);

    if announce.shown != Some(name) && (jumped || settled(&announce, name, now)) {
        announce.shown = Some(name);
        announce.pending = None;
        announce.until = now + HOLD;
        **text = name.to_string();
    } else if announce.shown != Some(name) {
        if announce.pending != Some(name) {
            announce.pending = Some(name);
            announce.pending_at = now;
        }
    } else {
        announce.pending = None;
    }

    let left = announce.until - now;
    if announce.shown.is_none() || left <= 0. {
        *vis = Visibility::Hidden;
        return;
    }
    *vis = Visibility::Visible;
    let alpha = (left / FADE).clamp(0., 1.);
    color.0 = Color::srgba(0.96, 0.78, 0.34, alpha);
    shadow.color = Color::srgba(0.07, 0.04, 0.02, alpha * 0.9);
}

fn settled(announce: &ZoneAnnounce, name: &'static str, now: f32) -> bool {
    announce.pending == Some(name) && now - announce.pending_at >= 0.3
}

fn titled(name: &'static str) -> &'static str {
    match name {
        "GREYMERE" => "Greymere Yard",
        "THE LANTERN POST" => "The Lantern Post",
        "THE DROWNED GATE" => "The Drowned Gate",
        "HUNTER'S BLIND" => "Hunter's Blind",
        "FEN SHRINE" => "Fen Shrine",
        "THE BONE ISLANDS" => "The Bone Islands",
        "THE SUNKEN OX" => "The Sunken Ox",
        "THE SHALLOWS" => "The Shallows",
        "THE SABLE REST" => "The Sable Rest",
        other => other,
    }
}
