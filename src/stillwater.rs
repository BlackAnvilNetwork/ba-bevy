//! Stillwater landmarks and presentation of the authoritative marsh snapshot.
use crate::{forge_menu::GameFlow, net::ZoneNet, player::Player};
use ba_protocol::stillwater::*;
use bevy::prelude::*;

#[derive(Component)]
pub struct MarshHud;
#[derive(Component)]
pub struct Silt;
#[derive(Component)]
pub struct Actor(u32);
#[derive(Component)]
pub struct Cue(u32);
#[derive(Component)]
pub struct Parcel(usize, u8);
#[derive(Component)]
pub struct Label(usize);
#[derive(Resource)]
pub struct Paint {
    common: Handle<StandardMaterial>,
    gold: Handle<StandardMaterial>,
    red: Handle<StandardMaterial>,
    beams: [Handle<StandardMaterial>; 5],
    chime: Handle<AudioSource>,
}
#[derive(Default, Resource)]
pub struct DisplayState {
    inventory: bool,
    filter: bool,
    paused: bool,
    deaths: std::collections::HashMap<u32, f32>,
    landed: std::collections::HashSet<(i32, i32)>,
}

fn pigment(m: &mut Assets<StandardMaterial>, rgb: [f32; 3], glow: f32) -> Handle<StandardMaterial> {
    let [r, g, b] = rgb;
    m.add(StandardMaterial {
        base_color: Color::srgb(r, g, b),
        emissive: LinearRgba::rgb(r * glow, g * glow, b * glow),
        perceptual_roughness: 0.85,
        ..default()
    })
}
pub fn setup(
    mut c: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    assets: Res<AssetServer>,
    scans: Res<crate::megascans::MegascansRoot>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let cube = meshes.add(Cuboid::from_size(Vec3::ONE));
    let round = meshes.add(Sphere::new(1.).mesh().uv(10, 7));
    let water = pigment(&mut mats, [0.09, 0.18, 0.18], 0.13);
    let mud = pigment(&mut mats, [0.32, 0.29, 0.20], 0.12);
    let reed = pigment(&mut mats, [0.28, 0.39, 0.21], 0.12);
    let wood = pigment(&mut mats, [0.27, 0.18, 0.13], 0.15);
    let bone = pigment(&mut mats, [0.78, 0.75, 0.58], 0.25);
    let blue = pigment(&mut mats, [0.20, 0.75, 0.92], 2.);
    let gold = pigment(&mut mats, [1., 0.56, 0.12], 2.);
    let white = pigment(&mut mats, [0.94, 0.94, 0.87], 1.5);
    let red = pigment(&mut mats, [1., 0.12, 0.12], 2.);
    let rare = pigment(&mut mats, [0.24, 0.68, 1.], 2.5);
    let bank = scans.surface("bank", &assets, &mut mats, &mud);
    let masonry = scans.surface("gate", &assets, &mut mats, &mud);
    let mut prop =
        |mesh: &Handle<Mesh>, mat: &Handle<StandardMaterial>, x, y, z, w, h, d, roll: f32| {
            c.spawn((
                Silt,
                Mesh3d(mesh.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_xyz(x, y, z)
                    .with_scale(Vec3::new(w, h, d))
                    .with_rotation(Quat::from_rotation_z(roll)),
            ));
        };
    prop(&cube, &water, 300., -0.31, 0., 170., 0.4, 210., 0.);
    prop(&cube, &mud, 600., -0.16, 0., 40., 0.3, 40., 0.);
    for pt in [
        POST,
        SHRINE,
        BLIND,
        at(300., 0.),
        at(309., -42.),
        at(301., -61.),
        GATE,
    ] {
        prop(&round, &bank, pt.x, -0.13, pt.z, 6., 0.23, 5., 0.);
        prop(
            &cube,
            &wood,
            pt.x + 2.,
            1.65,
            pt.z + 1.,
            0.22,
            3.3,
            0.22,
            0.04,
        );
        prop(
            &cube,
            &blue,
            pt.x + 2.,
            3.42,
            pt.z + 1.,
            0.45,
            0.5,
            0.45,
            0.,
        );
    }
    for i in 0..145 {
        let x = 217. + ((i * 67 + 13) % 166) as f32;
        let z = -103. + ((i * 83 + 17) % 205) as f32;
        for j in 0..3 {
            prop(
                &cube,
                &reed,
                x + j as f32 * 0.23,
                0.46,
                z + j as f32 * 0.12,
                0.095,
                0.8 + 0.18 * j as f32,
                0.1,
                0.18 * j as f32 - 0.18,
            );
        }
    }
    // Each rib is a tapered arc that rises from either bank of the spine.
    for i in 0..9 {
        let z = 8. - i as f32 * 2.05;
        let height = 7. + (i as f32 * 0.4).sin() * 3.;
        prop(&round, &bone, 300., height, z, 0.75, 0.75, 1.5, 0.);
        for side in [-1., 1.] {
            for n in 0..6 {
                let pt = |j: f32| {
                    let a = j / 6. * 1.65;
                    Vec2::new(300. + side * 6. * a.sin(), height * a.cos())
                };
                let a = pt(n as f32);
                let b = pt(n as f32 + 1.);
                let mid = (a + b) / 2.;
                let v = b - a;
                prop(
                    &cube,
                    &bone,
                    mid.x,
                    mid.y,
                    z,
                    0.62,
                    v.length() + 0.15,
                    0.6,
                    -v.x.atan2(v.y),
                );
            }
        }
    }
    prop(&round, &bone, 300., 7.2, -11., 2.8, 1.8, 3.5, 0.);
    for side in [-1., 1.] {
        prop(
            &cube,
            &bone,
            300. + side * 3.,
            8.3,
            -11.,
            0.65,
            4.5,
            0.65,
            -side * 1.0,
        );
    }
    for z in [-98., -96.] {
        for x in [297., 303.] {
            prop(&cube, &masonry, x, 2.1, z, 1.1, 4.4, 1.2, 0.);
        }
        prop(&cube, &bone, 300., 4.5, z, 7.2, 0.9, 1.3, 0.);
    }
    for k in 0..13 {
        prop(
            &cube,
            &wood,
            304.,
            0.05,
            -45. - k as f32,
            2.4,
            0.12,
            0.88,
            0.,
        );
    }
    for pt in [POST, SHRINE, REST] {
        prop(&round, &gold, pt.x + 1.1, 0.2, pt.z, 0.45, 0.6, 0.45, 0.);
    }
    for (pt, mat) in NODES {
        prop(
            &round,
            if mat == "Stillwater Essence" {
                &blue
            } else if mat == "Cold Iron" {
                &white
            } else {
                &mud
            },
            pt.x,
            0.27,
            pt.z,
            0.65,
            0.42,
            0.52,
            0.,
        );
    }
    // Pell's overturned punt, the ferryman's boat and the dry causeway.
    for (x, z) in [(POST.x - 1.5, POST.z - 1.5), (GATE.x + 1.4, GATE.z - 1.1)] {
        prop(&round, &wood, x, 0.20, z, 0.85, 0.34, 2.35, 0.);
        prop(&cube, &mud, x, 0.48, z, 1.0, 0.12, 0.35, 0.);
    }
    for i in 0..8 {
        prop(
            &round,
            &bone,
            300.,
            -0.05,
            -97. - i as f32,
            1.3,
            0.12,
            0.55,
            0.,
        );
    }
    for pt in HERONS {
        prop(&round, &bone, pt.x, 0.8, pt.z, 0.25, 0.26, 0.46, 0.);
        prop(
            &cube,
            &bone,
            pt.x,
            1.13,
            pt.z - 0.25,
            0.12,
            0.72,
            0.13,
            0.12,
        );
        prop(&cube, &gold, pt.x, 1.45, pt.z - 0.43, 0.1, 0.1, 0.44, 0.);
    }
    for (x, z, path, size) in [
        (POST.x, POST.z - 1.5, "models/hooded_figure.glb", 1.),
        (
            SHRINE.x + 1.3,
            SHRINE.z + 0.7,
            "models/hooded_figure.glb",
            1.,
        ),
        (
            SHRINE.x,
            SHRINE.z,
            "kits/mini-dungeon/Models/GLB format/pot.glb",
            2.,
        ),
        (
            BLIND.x,
            BLIND.z,
            "kits/mini-dungeon/Models/GLB format/wood-structure.glb",
            2.,
        ),
        (GATE.x + 1.5, GATE.z, "models/hooded_figure.glb", 1.),
        (REST.x + 1.3, REST.z - 0.7, "models/hooded_figure.glb", 1.),
        (
            REST.x,
            REST.z,
            "kits/mini-dungeon/Models/GLB format/table.glb",
            2.,
        ),
    ] {
        c.spawn((
            Silt,
            WorldAssetRoot(assets.load(format!("{path}#Scene0"))),
            Transform::from_xyz(x, 0., z).with_scale(Vec3::splat(size)),
        ));
    }
    for (slot, sites) in [
        (
            "terrain",
            &[(294., 88.), (282., 64.), (249., -32.), (306., -46.)][..],
        ),
        ("rock", &[(282., 65.), (253., -29.), (303., -91.)][..]),
    ] {
        if let Some(scene) = scans.scene(slot, &assets) {
            for (i, &(x, z)) in sites.iter().enumerate() {
                c.spawn((
                    Silt,
                    WorldAssetRoot(scene.0.clone()),
                    Transform::from_xyz(x, -0.12, z)
                        .with_rotation(Quat::from_rotation_y(i as f32 * 1.7)),
                ));
            }
        }
    }
    for (x, z) in [
        (POST.x, POST.z),
        (SHRINE.x, SHRINE.z),
        (GATE.x, GATE.z),
        (REST.x, REST.z),
    ] {
        c.spawn((
            Silt,
            PointLight {
                color: Color::srgb(0.82, 0.90, 1.0),
                intensity: 52000.,
                range: 13.,
                ..default()
            },
            Transform::from_xyz(x + 1., 3.25, z),
        ));
    }
    for (i, (x, z)) in [
        (287., 84.),
        (306., 81.),
        (275., 63.),
        (318., 51.),
        (329., 17.),
        (258., -29.),
        (289., -55.),
        (310., -84.),
    ]
    .into_iter()
    .enumerate()
    {
        let asset = if i % 3 == 0 {
            "pine-fall-crooked"
        } else if i % 3 == 1 {
            "rocks"
        } else {
            "pine-crooked"
        };
        c.spawn((
            Silt,
            WorldAssetRoot(assets.load(format!(
                "kits/graveyard-kit/Models/GLB format/{asset}.glb#Scene0"
            ))),
            Transform::from_xyz(x, -0.13, z)
                .with_rotation(Quat::from_rotation_y(i as f32))
                .with_scale(Vec3::splat(2.2)),
        ));
    }
    // Authored Idle_Sword, Walk, Sword_Slash and Death in the shipped hooded figure.
    let clips = [9, 22, 21, 0].map(|index| {
        assets.load(GltfAssetLabel::Animation(index).from_asset("models/hooded_figure.glb"))
    });
    let (graph, nodes) = AnimationGraph::from_clips(clips);
    c.insert_resource(EddaClips {
        graph: graphs.add(graph),
        nodes,
    });
    for foe in FOES.iter() {
        let mut e = c.spawn((
            Actor(foe.id),
            Transform::from_xyz(foe.x, 0., foe.z),
            Visibility::Hidden,
        ));
        if foe.id == EDDA {
            e.insert(WorldAssetRoot(
                assets.load("models/hooded_figure.glb#Scene0"),
            ));
        } else {
            e.with_children(|root| {
                let (length, girth, height, color) = match foe.id {
                    OX => (2.3, 1.2, 1.7, &mud),
                    70000..=70001 | 70012..=70013 => (0.92, 0.38, 0.62, &wood),
                    70002..=70004 => (0.75, 0.85, 0.48, &reed),
                    70008..=70009 => (1.55, 0.23, 0.24, &bone),
                    _ => (0.62, 0.33, 0.4, &bone),
                };
                let mut part = |mesh: &Handle<Mesh>,
                                mat: &Handle<StandardMaterial>,
                                x,
                                y,
                                z,
                                w,
                                h,
                                d,
                                angle| {
                    root.spawn((
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(mat.clone()),
                        Transform::from_xyz(x, y, z)
                            .with_scale(Vec3::new(w, h, d))
                            .with_rotation(Quat::from_rotation_z(angle)),
                    ));
                };
                part(
                    &round,
                    color,
                    0.,
                    height * 0.8,
                    0.,
                    girth,
                    height * 0.58,
                    length,
                    0.,
                );
                part(
                    &round,
                    color,
                    0.,
                    height,
                    length * 0.84,
                    girth * 0.7,
                    height * 0.44,
                    length * 0.47,
                    0.,
                );
                for side in [-1., 1.] {
                    part(
                        &round,
                        &blue,
                        side * girth * 0.42,
                        height * 1.12,
                        length * 1.15,
                        0.09,
                        0.08,
                        0.1,
                        0.,
                    );
                    if foe.id == OX {
                        part(
                            &cube,
                            &bone,
                            side * 1.05,
                            2.1,
                            2.2,
                            0.22,
                            1.6,
                            0.3,
                            -side * 0.85,
                        );
                    }
                    if !matches!(foe.id, 70008..=70009) {
                        for z in [-0.6, 0.64] {
                            part(
                                &cube,
                                color,
                                side * girth * 0.72,
                                height * 0.35,
                                z * length,
                                0.17,
                                height * 0.7,
                                0.25,
                                side * 0.22,
                            );
                        }
                    }
                }
            });
        }
        c.spawn((Cue(foe.id), Transform::default(), Visibility::Hidden))
            .with_children(|root| {
                for i in 0..20 {
                    let a = i as f32 * std::f32::consts::TAU / 20.;
                    root.spawn((
                        Mesh3d(cube.clone()),
                        MeshMaterial3d(white.clone()),
                        Transform::from_xyz(a.cos(), 0., a.sin())
                            .with_scale(Vec3::new(0.07, 0.12, 0.07)),
                    ));
                }
            });
    }
    for i in 0..16 {
        for part in 0..3 {
            c.spawn((
                Parcel(i, part),
                Mesh3d(if part == 2 {
                    round.clone()
                } else {
                    cube.clone()
                }),
                MeshMaterial3d(gold.clone()),
                Transform::default(),
                Visibility::Hidden,
            ));
        }
        c.spawn((
            Label(i),
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            Text::new(""),
            TextFont::from_font_size(14.),
            TextColor(Color::WHITE),
            Visibility::Hidden,
        ));
    }
    c.spawn((
        MarshHud,
        Text::new(""),
        TextFont::from_font_size(16.),
        TextColor(Color::srgb(0.87, 0.95, 0.92)),
        Node {
            position_type: PositionType::Absolute,
            top: px(20),
            right: px(18),
            max_width: px(450),
            ..default()
        },
        Visibility::Hidden,
    ));
    c.spawn((
        ImpactText,
        Text::new(""),
        TextFont::from_font_size(26.),
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: percent(49),
            top: percent(45),
            ..default()
        },
        Visibility::Hidden,
    ));
    c.insert_resource(Paint {
        common: white.clone(),
        gold,
        red,
        beams: [
            white,
            rare,
            pigment(&mut mats, [1., 0.5, 0.08], 3.),
            pigment(&mut mats, [0.69, 0.28, 1.], 3.),
            pigment(&mut mats, [1., 0.15, 0.3], 4.),
        ],
        chime: assets.load("audio/cinder-land.ogg"),
    });
}

pub fn impact_time(
    net: Res<ZoneNet>,
    mut clock: ResMut<Time<Virtual>>,
    mut display: ResMut<DisplayState>,
) {
    let stop = net.online()
        && net
            .impact
            .is_some_and(|(t, big, _)| t.elapsed().as_millis() < if big { 110 } else { 50 });
    if stop && !clock.is_paused() {
        clock.pause();
        display.paused = true;
    }
    if !stop && display.paused {
        clock.unpause();
        display.paused = false;
    }
}
pub fn camera_feedback(
    net: Res<ZoneNet>,
    mut camera: Query<&mut Transform, With<crate::camera_rig::PlayerCamera>>,
) {
    if !net.online() {
        return;
    }
    let Ok(mut tf) = camera.single_mut() else {
        return;
    };
    let age = net
        .impact
        .map(|(t, _, hurt)| (t.elapsed().as_secs_f32(), hurt));
    let tilt = if net.marsh.rolling {
        -0.28
    } else if let Some((dt, hurt)) = age {
        let sign = if hurt { -1. } else { 0.45 };
        sign * (1. - dt / 0.25).clamp(0., 1.) * 0.08
    } else {
        0.
    };
    let (pitch, _, _) = tf.rotation.to_euler(EulerRot::XYZ);
    tf.rotation = Quat::from_rotation_x(pitch) * Quat::from_rotation_z(tilt);
}

pub fn animate(
    net: Res<ZoneNet>,
    flow: Res<GameFlow>,
    time: Res<Time>,
    paint: Res<Paint>,
    mut display: ResMut<DisplayState>,
    mut actors: Query<(&Actor, &mut Transform, &mut Visibility), Without<Cue>>,
    mut cues: Query<(&Cue, &Children, &mut Transform, &mut Visibility), Without<Actor>>,
    mut materials: Query<&mut MeshMaterial3d<StandardMaterial>>,
) {
    let active = net.online() && !flow.in_menu() && marsh(net.region_x);
    for (actor, mut tf, mut vis) in &mut actors {
        let Some(e) = net.marsh.enemies.iter().find(|e| active && e.id == actor.0) else {
            *vis = Visibility::Hidden;
            continue;
        };
        let elapsed = time.elapsed_secs();
        let death = if e.hp == 0 {
            elapsed - *display.deaths.entry(e.id).or_insert(elapsed)
        } else {
            display.deaths.remove(&e.id);
            0.
        };
        *vis = if death > 0.65 {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
        let collapse = (death / 0.65).clamp(0., 1.);
        let crouch = e.windup * 0.32;
        *tf = Transform::from_xyz(
            e.x,
            if e.submerged { -0.45 } else { 0.03 } + (elapsed * 3. + e.x).sin().abs() * 0.045
                - collapse * 0.4,
            e.z,
        )
        .with_rotation(
            Quat::from_rotation_y(e.yaw) * Quat::from_rotation_x(-0.32 * e.windup + collapse * 0.9),
        )
        .with_scale(Vec3::new(
            1. + crouch,
            1. - crouch - collapse * 0.75,
            1. + crouch,
        ));
    }
    for (cue, children, mut tf, mut vis) in &mut cues {
        let Some(e) = net
            .marsh
            .enemies
            .iter()
            .find(|e| active && e.id == cue.0 && e.windup > 0. && e.hp > 0)
        else {
            *vis = Visibility::Hidden;
            continue;
        };
        *vis = Visibility::Visible;
        let (sin, cos) = e.yaw.sin_cos();
        let offset = if e.line { e.radius * 0.5 } else { 0. };
        *tf = Transform::from_xyz(e.x + sin * offset, 0.16, e.z + cos * offset)
            .with_rotation(Quat::from_rotation_y(e.yaw))
            .with_scale(if e.line {
                Vec3::new(1.2, 1. + e.windup * 2., offset)
            } else {
                Vec3::new(e.radius, 1. + e.windup * 2., e.radius)
            });
        let color = match e.tell {
            Tell::Normal => &paint.common,
            Tell::Heavy => &paint.gold,
            Tell::Unblockable => &paint.red,
        };
        for child in children.iter() {
            if let Ok(mut mat) = materials.get_mut(child) {
                if mat.0 != *color {
                    mat.0 = color.clone();
                }
            }
        }
    }
}

pub fn loot(
    mut c: Commands,
    net: Res<ZoneNet>,
    paint: Res<Paint>,
    mut display: ResMut<DisplayState>,
    flow: Res<GameFlow>,
    time: Res<Time>,
    cam: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    windows: Query<&Window>,
    mut parcels: Query<
        (
            &Parcel,
            &mut Transform,
            &mut MeshMaterial3d<StandardMaterial>,
            &mut Visibility,
        ),
        Without<Label>,
    >,
    mut labels: Query<
        (
            &Label,
            &mut Node,
            &mut Text,
            &mut TextColor,
            &mut Visibility,
        ),
        Without<Parcel>,
    >,
) {
    let active = net.online() && !flow.in_menu();
    let t = time.elapsed_secs();
    let visible = |i: usize| {
        net.marsh
            .drops
            .get(i)
            .filter(|d| active && (!display.filter || d.rare) && d.age >= 0.65)
    };
    for (parcel, mut tf, mut mat, mut vis) in &mut parcels {
        let Some(d) = visible(parcel.0) else {
            *vis = Visibility::Hidden;
            continue;
        };
        let reveal = ((d.age - 0.65) / 0.3).clamp(0., 1.);
        let size = match parcel.1 {
            0 => Vec3::splat(0.34),
            1 if d.rare => Vec3::new(0.16, (8. + 2. * d.rarity() as usize as f32) * reveal, 0.16),
            2 if d.age < 1.1 => Vec3::new(reveal, 0.13, reveal),
            _ => {
                *vis = Visibility::Hidden;
                continue;
            }
        };
        let y = if parcel.1 == 1 {
            size.y / 2.
        } else if parcel.1 == 2 {
            0.12
        } else {
            0.45 + (t * 3.).sin() * 0.07
        };
        *tf = Transform::from_xyz(d.x, y, d.z)
            .with_rotation(Quat::from_rotation_y(t * 1.8))
            .with_scale(size);
        *vis = Visibility::Visible;
        mat.0 = paint.beams[d.rarity() as usize].clone();
    }
    for (label, mut node, mut text, mut color, mut vis) in &mut labels {
        *vis = Visibility::Hidden;
        let Some(d) = visible(label.0) else { continue };
        let (Ok((cam, xf)), Ok(w)) = (cam.single(), windows.single()) else {
            continue;
        };
        let Ok(pt) = cam.world_to_viewport(xf, Vec3::new(d.x, 1.1, d.z)) else {
            continue;
        };
        let Some(size) = cam.logical_viewport_size() else {
            continue;
        };
        if pt.x < 0. || pt.y < 0. || pt.x > size.x || pt.y > size.y {
            continue;
        }
        node.left = px(pt.x / size.x * w.width());
        node.top = px(pt.y / size.y * w.height());
        **text = d
            .items
            .iter()
            .map(|s| s.trim_start_matches("ashworks:").replace('_', " "))
            .collect::<Vec<_>>()
            .join(" · ");
        color.0 = match d.rarity() {
            Rarity::Common => Color::WHITE,
            Rarity::Rare => Color::srgb(0.42, 0.83, 1.),
            Rarity::Legendary => Color::srgb(1., 0.65, 0.23),
            Rarity::Ancient => Color::srgb(0.83, 0.58, 1.),
            Rarity::Primal => Color::srgb(1., 0.34, 0.47),
        };
        *vis = Visibility::Visible;
    }
    let mut seen = std::collections::HashSet::new();
    for d in &net.marsh.drops {
        if active && d.age >= 0.65 {
            let key = (d.x.to_bits() as i32, d.z.to_bits() as i32);
            seen.insert(key);
            if d.age < 1.3 && !display.landed.contains(&key) {
                c.spawn((
                    AudioPlayer::new(paint.chime.clone()),
                    PlaybackSettings::DESPAWN,
                ));
            }
        }
    }
    display.landed = seen;
}

pub fn journal(
    net: Res<ZoneNet>,
    flow: Res<GameFlow>,
    keys: Res<ButtonInput<KeyCode>>,
    mut display: ResMut<DisplayState>,
    player: Query<&Transform, With<Player>>,
    mut hud: Query<(&mut Text, &mut Visibility), With<MarshHud>>,
) {
    let Ok((mut text, mut vis)) = hud.single_mut() else {
        return;
    };
    if !net.online() || flow.in_menu() {
        *vis = Visibility::Hidden;
        return;
    }
    if keys.just_pressed(KeyCode::KeyI) {
        display.inventory = !display.inventory
    }
    if keys.just_pressed(KeyCode::KeyL) {
        display.filter = !display.filter
    }
    let m = &net.marsh;
    if !marsh(net.region_x) && !rest(net.region_x) && m.drops.is_empty() && !display.inventory {
        *vis = Visibility::Hidden;
        return;
    }
    *vis = Visibility::Visible;
    let Ok(tf) = player.single() else { return };
    let at = Vec2::new(tf.translation.x, tf.translation.z);
    let range = |p: ba_protocol::Pose| at.distance(Vec2::new(p.x, p.z));
    let (name, mut action) = if rest(net.region_x) {
        (
            "THE SABLE REST",
            "E at Cael's field forge · 4 Cold Iron + 1 Peat",
        )
    } else if !marsh(net.region_x) {
        ("GREYMERE", "E · Collect materials")
    } else if range(GATE) < 13. {
        (
            "THE DROWNED GATE",
            "Defeat Edda, pick up her book, then E at the ward stone",
        )
    } else if range(POST) < 10. {
        (
            "THE LANTERN POST",
            "E · Talk to Old Pell. Hunt west into the Shallows",
        )
    } else if range(BLIND) < 10. {
        ("HUNTER'S BLIND", "E · Search under the hide rack")
    } else if range(SHRINE) < 10. {
        ("FEN SHRINE", "E · Rest by Pell's fire")
    } else if tf.translation.z < -36. {
        (
            "THE BONE ISLANDS",
            "Gather from the banks. Follow the lanterns north to Edda",
        )
    } else if tf.translation.z < 20. {
        (
            "THE SUNKEN OX",
            "The Ox walks east of the ribs. Bring its yoke north",
        )
    } else {
        (
            "THE SHALLOWS",
            "Hunt the reeds, then head north to the great ribs",
        )
    };
    if HERONS.iter().any(|&p| range(p) < 4.5) {
        action = "E · Listen to the heron"
    }
    if NODES.iter().any(|&(p, _)| range(p) < 4.5) {
        action = "E · Gather materials"
    }
    if m.drops
        .iter()
        .any(|d| at.distance(Vec2::new(d.x, d.z)) < 3.6 && d.age >= 0.75)
    {
        action = "E · Pick up fallen materials"
    }
    let mut lines = format!(
        "{name}\n{action}\nStamina {} · Shift dodge · Ctrl sprint · Q heal\nI pack · L {} drops",
        m.stamina,
        if display.filter { "rare" } else { "all" }
    );
    if m.rain && marsh(net.region_x) {
        lines.push_str("\nWarm rain · Essence finds doubled");
    }
    if m.dead {
        lines.push_str("\nThe water holds you…");
    } else if !m.notice.is_empty() {
        lines.push('\n');
        lines.push_str(&m.notice);
    }
    if marsh(net.region_x) {
        lines.push_str(&format!("\n{}", m.quest));
    }
    if display.inventory {
        for (name, n) in &m.inventory {
            lines.push_str(&format!("\n{name} ×{n}"));
        }
    }
    if **text != lines {
        **text = lines;
    }
}

#[derive(Resource)]
pub struct EddaClips {
    graph: Handle<AnimationGraph>,
    nodes: Vec<AnimationNodeIndex>,
}

pub fn edda_animation(
    mut commands: Commands,
    clips: Res<EddaClips>,
    net: Res<ZoneNet>,
    time: Res<Time>,
    actors: Query<(Entity, &Actor)>,
    children: Query<&Children>,
    mut players: Query<(&mut AnimationPlayer, Option<&mut AnimationTransitions>)>,
    mut last: Local<Option<(usize, Vec2, f32, f32)>>,
) {
    let Some(enemy) = net.marsh.enemies.iter().find(|e| e.id == EDDA) else {
        *last = None;
        return;
    };
    let Some((root, _)) = actors.iter().find(|(_, actor)| actor.0 == EDDA) else {
        return;
    };
    let pos = Vec2::new(enemy.x, enemy.z);
    let (previous, old_pos, until, release) = last.unwrap_or((usize::MAX, pos, 0., 0.));
    let now = time.elapsed_secs();
    let moving_until = if pos.distance_squared(old_pos) > 0.001 {
        now + 0.18
    } else {
        until
    };
    let release = if enemy.windup > 0. {
        now + 0.23
    } else {
        release
    };
    let pose = if enemy.hp == 0 {
        3
    } else if enemy.windup > 0. || now < release {
        2
    } else if now < moving_until {
        1
    } else {
        0
    };
    for id in children.iter_descendants(root) {
        let Ok((mut player, transitions)) = players.get_mut(id) else {
            continue;
        };
        let Some(mut transitions) = transitions else {
            commands.entity(id).insert((
                AnimationGraphHandle(clips.graph.clone()),
                AnimationTransitions::new(),
            ));
            continue;
        };
        if pose != previous {
            let anim = transitions.play(
                &mut player,
                clips.nodes[pose],
                std::time::Duration::from_millis(110),
            );
            if pose < 2 {
                anim.repeat();
            } else if pose == 2 {
                anim.set_speed(0.68);
            }
        }
        *last = Some((pose, pos, moving_until, release));
    }
}
pub fn mist(
    net: Res<ZoneNet>,
    time: Res<Time>,
    mut cameras: Query<&mut bevy::pbr::DistanceFog, With<Camera3d>>,
    mut original: Local<Option<bevy::pbr::DistanceFog>>,
) {
    let Ok(mut fog) = cameras.single_mut() else {
        return;
    };
    if net.online() && marsh(net.region_x) {
        if original.is_none() {
            *original = Some(fog.clone());
        }
        fog.color = Color::srgba(0.57, 0.61, 0.54, 0.86);
        fog.falloff = bevy::pbr::FogFalloff::Linear {
            start: if net.marsh.rain { 8. } else { 20. },
            end: 128. + (time.elapsed_secs() * 0.055).sin() * 17.,
        };
    } else if let Some(old) = original.take() {
        *fog = old;
    }
}

#[derive(Component)]
pub struct ImpactText;
pub fn impact_mark(
    net: Res<ZoneNet>,
    mut query: Query<(&mut Text, &mut TextColor, &mut Visibility), With<ImpactText>>,
) {
    let Ok((mut text, mut color, mut vis)) = query.single_mut() else {
        return;
    };
    let Some((when, big, hurt)) = net
        .impact
        .filter(|(t, _, _)| net.online() && t.elapsed().as_secs_f32() < 0.52)
    else {
        *vis = Visibility::Hidden;
        return;
    };
    *vis = Visibility::Visible;
    color.0 = if hurt {
        Color::srgb(1., 0.28, 0.22)
    } else if big {
        Color::srgb(1., 0.75, 0.19)
    } else {
        Color::WHITE
    };
    **text = if when.elapsed().as_millis() < if big { 110 } else { 50 } {
        "✦".into()
    } else if hurt {
        format!("−{}", net.last_amount)
    } else {
        net.last_amount.to_string()
    };
}
