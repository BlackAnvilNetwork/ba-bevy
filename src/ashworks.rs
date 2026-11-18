//! Visible kiln constructs and their forge order in the online yard.
use crate::{
    cel::{CelMaterial, CelUniforms},
    forge_menu::GameFlow,
    net::ZoneNet,
    player::Player,
};
use ba_protocol::ashworks::{CONSTRUCTS, FORGE, PULSE_RADIUS, WARD_COST};
use bevy::prelude::*;

#[derive(Component)]
pub struct Body(usize);
#[derive(Component)]
pub struct Ember(usize, f32);
#[derive(Component)]
pub struct ForgeOrder;
#[derive(Component)]
pub struct Gallery;

pub fn gallery_visibility(
    net: Res<ZoneNet>,
    flow: Res<GameFlow>,
    mut gallery: Query<&mut Visibility, With<Gallery>>,
) {
    if let Ok(mut vis) = gallery.single_mut() {
        *vis = if net.online() && net.region_x<100. && !flow.in_menu() {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}

pub fn spawn(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<CelMaterial>>,
) {
    let cube = meshes.add(Cuboid::new(1., 1., 1.));
    let mut pigment = |r, g, b| {
        mats.add(CelMaterial {
            uniforms: CelUniforms {
                color: LinearRgba::new(r, g, b, 1.),
                light_dir: Vec4::new(-0.2, 0.9, 0.35, 0.),
            },
        })
    };
    let iron = pigment(0.16, 0.13, 0.20);
    let pale = pigment(0.95,0.95,0.95);
    let copper = pigment(0.47, 0.24, 0.12);
    let flame = pigment(1.0, 0.51, 0.08);
    // A roofless firing gallery follows the east road, leaving room to dodge
    // through either side. Coal-black brick, copper bands and ember lamps.
    commands
        .spawn((Gallery, Transform::default(), Visibility::Hidden))
        .with_children(|root| {
            for x in [16., 19., 25.] {
                for z in [-3.65, 0.65] {
                    for (y, w, h, d, material) in [
                        (0.15, 0.8, 0.3, 0.8, &iron),
                        (1.35, 0.48, 2.4, 0.48, &iron),
                        (2.45, 0.72, 0.18, 0.72, &copper),
                        (1.95, 0.51, 0.12, 0.51, &flame),
                    ] {
                        root.spawn((
                            Mesh3d(cube.clone()),
                            MeshMaterial3d(material.clone()),
                            Transform::from_xyz(x, y, z).with_scale(Vec3::new(w, h, d)),
                        ));
                    }
                }
                root.spawn((
                    Mesh3d(cube.clone()),
                    MeshMaterial3d(iron.clone()),
                    Transform::from_xyz(x, 2.75, -1.5).with_scale(Vec3::new(0.6, 0.36, 5.0)),
                ));
            }
        });
    for (i, spec) in CONSTRUCTS.iter().enumerate() {
        commands
            .spawn((Body(i), Transform::default(), Visibility::Hidden))
            .with_children(|root| {
                // Riveted firebox, chimney, two hammer fists, and a glowing grate.
                for (x, y, z, w, h, d, mat) in [
                    (0., 0.72, 0., 0.83, 0.94, 0.63, &iron),
                    (0., 1.29, 0., 0.48, 0.26, 0.46, &copper),
                    (-0.48, 0.62, 0., 0.29, 0.44, 0.39, &copper),
                    (0.48, 0.62, 0., 0.29, 0.44, 0.39, &copper),
                    (-0.23, 0.15, 0., 0.32, 0.30, 0.48, &iron),
                    (0.23, 0.15, 0., 0.32, 0.30, 0.48, &iron),
                    (0., 0.74, 0.32, 0.62, 0.16, 0.06, &flame),
                    (0., 0.53, 0.32, 0.50, 0.07, 0.06, &flame),
                ] {
                    root.spawn((
                        Mesh3d(cube.clone()),
                        MeshMaterial3d(mat.clone()),
                        Transform::from_xyz(x, y, z).with_scale(Vec3::new(w, h, d)),
                    ));
                }
            });
        for n in 0..20 {
            let angle = n as f32 * std::f32::consts::TAU / 20.;
            commands.spawn((
                Ember(i, angle),
                Mesh3d(cube.clone()),
                MeshMaterial3d(if i==0 { pale.clone() } else { flame.clone() }),
                Transform::from_xyz(spec.x, 0.1, spec.z),
                Visibility::Hidden,
            ));
        }
    }
    commands.spawn((
        ForgeOrder,
        Text::new(""),
        TextFont::from_font_size(16.),
        TextColor(Color::srgb(1., 0.83, 0.59)),
        Node {
            position_type: PositionType::Absolute,
            right: px(20),
            top: px(18),
            ..default()
        },
        Visibility::Hidden,
    ));
}

pub fn display(
    net: Res<ZoneNet>,
    flow: Res<GameFlow>,
    time: Res<Time>,
    player: Query<&Transform, (With<Player>, Without<Body>, Without<Ember>)>,
    mut bodies: Query<(&Body, &mut Transform, &mut Visibility), Without<Ember>>,
    mut sparks: Query<(&Ember, &mut Transform, &mut Visibility), Without<Body>>,
    mut order: Query<
        (&mut Text, &mut Visibility),
        (With<ForgeOrder>, Without<Ember>, Without<Body>),
    >,
) {
    let active = net.online() && net.region_x<100. && !flow.in_menu();
    let t = time.elapsed_secs();
    for (body, mut tf, mut vis) in &mut bodies {
        let spec = &CONSTRUCTS[body.0];
        let state = net.ashworks.constructs.iter().find(|s| s.id == spec.id);
        let Some(s) = state.filter(|s| active && s.hp > 0) else {
            *vis = Visibility::Hidden;
            continue;
        };
        *vis = Visibility::Visible;
        let size = if body.0 == 1 { 1.55 } else { 1.0 };
        let bounce = if s.winding_up {
            (t * 19.).sin() * 0.06
        } else {
            (t * 2.8).sin() * 0.025
        };
        *tf = Transform::from_xyz(spec.x, bounce.abs(), spec.z).with_scale(Vec3::splat(size));
        if s.winding_up {tf.scale*=Vec3::new(1.25,0.63,1.25);}
        if let Ok(p) = player.single() {
            tf.rotation =
                Quat::from_rotation_y((p.translation.x - spec.x).atan2(p.translation.z - spec.z));
        }
    }
    for (ember, mut tf, mut vis) in &mut sparks {
        let spec = &CONSTRUCTS[ember.0];
        if active
            && net
                .ashworks
                .constructs
                .iter()
                .any(|s| s.id == spec.id && s.winding_up)
        {
            *vis = Visibility::Visible;
            let (sin, cos) = ember.1.sin_cos();
            *tf = Transform::from_xyz(
                spec.x + cos * PULSE_RADIUS,
                0.12 + (t * 12. + ember.1).sin() * 0.07,
                spec.z + sin * PULSE_RADIUS,
            )
            .with_scale(Vec3::splat(0.14));
        } else {
            *vis = Visibility::Hidden;
        }
    }
    let Ok((mut text, mut vis)) = order.single_mut() else {
        return;
    };
    if !active {
        *vis = Visibility::Hidden;
        return;
    }
    *vis = Visibility::Visible;
    let Ok(p) = player.single() else { return };
    let pos = p.translation;
    let near_forge = Vec2::new(pos.x - FORGE.0, pos.z - FORGE.1).length() <= 2.5;
    let task = if net.ashworks.edge {
        "Tempered edge equipped · +25% auto-attack power"
    } else if net.ashworks.heart && net.ashworks.ward && net.ashworks.cinder >= WARD_COST {
        if near_forge {
            "G · Temper an edge with 1 kiln heart and 2 cinder"
        } else {
            "Return to the fire basket to temper your edge"
        }
    } else if net.ashworks.ward && net.ashworks.heart {
        "Gather 2 cinder to work the kiln heart"
    } else if net.ashworks.ward {
        "Kiln ward equipped · Hunt the Warden on the east road"
    } else if net.ashworks.cinder < WARD_COST {
        "Hunt the Clinker Imp by the well · F engage · 1–8 skills"
    } else if near_forge {
        "G · Hammer 2 raw cinder into a kiln ward"
    } else {
        "Bring 2 raw cinder to the fire basket beside the well"
    };
    let mut line = format!(
        "GREYMERE ASHWORKS\n{task}\nRaw cinder  {}",
        net.ashworks.cinder
    );
    for spec in &CONSTRUCTS {
        if Vec2::new(pos.x - spec.x, pos.z - spec.z).length() < 8. {
            if let Some(s) = net.ashworks.constructs.iter().find(|s| s.id == spec.id) {
                let tell = if s.winding_up {
                    " · Step outside the sparks!"
                } else if s.hp == 0 {
                    if spec.id == CONSTRUCTS[0].id {
                        " · On a union-mandated tea break."
                    } else {
                        " · Rekindling…"
                    }
                } else {
                    ""
                };
                line.push_str(&format!("\n{}  {}/{}{tell}", spec.name, s.hp, spec.hp));
            }
        }
    }
    if text.0 != line {
        **text = line;
    }
}
