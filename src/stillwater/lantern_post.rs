//! The drowned abbey and the short plank causeway at Pell's landing.
use super::Silt;
use bevy::{prelude::*, render::render_resource::AsBindGroup, shader::ShaderRef};

// World X, world Z, and scale of the intact arches mirrored by the fen water.
const INTACT_ARCHES: [(f32, f32, f32); 4] = [
    (300., 65., 1.7),
    (301., 50., 1.67),
    (272., 48., 1.31),
    (281., 48., 1.30),
];

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct FenWater {
    #[uniform(0)]
    pub eye_time: Vec4,
    #[uniform(1)]
    arches: [Vec4; 4],
}
impl Material for FenWater {
    fn fragment_shader() -> ShaderRef {
        "shaders/fen_water.wesl".into()
    }
}

#[derive(Component)]
pub struct Flame {
    phase: f32,
    base: Vec3,
    scale: Vec3,
}

fn block(
    c: &mut Commands,
    mesh: &Handle<Mesh>,
    mat: &Handle<StandardMaterial>,
    pos: Vec3,
    size: Vec3,
    rot: Quat,
) {
    c.spawn((
        Silt,
        Mesh3d(mesh.clone()),
        MeshMaterial3d(mat.clone()),
        Transform::from_translation(pos)
            .with_scale(size)
            .with_rotation(rot),
    ));
}

fn kit(
    c: &mut Commands,
    assets: &AssetServer,
    kit: &str,
    name: &str,
    pos: Vec3,
    scale: f32,
    yaw: f32,
) {
    let format = if kit == "nature-kit" {
        "GLTF format"
    } else {
        "GLB format"
    };
    c.spawn((
        Silt,
        WorldAssetRoot(assets.load(format!("kits/{kit}/Models/{format}/{name}.glb#Scene0"))),
        Transform::from_translation(pos)
            .with_scale(Vec3::splat(scale))
            .with_rotation(Quat::from_rotation_y(yaw)),
    ));
}

fn hash(i: usize) -> f32 {
    let mut n = (i as u32).wrapping_mul(0x9e3779b9);
    n ^= n >> 16;
    n = n.wrapping_mul(0x85ebca6b);
    (n >> 8) as f32 / 16777216.
}

fn arch(
    c: &mut Commands,
    cube: &Handle<Mesh>,
    stone: &[Handle<StandardMaterial>],
    x: f32,
    z: f32,
    size: f32,
    broken: bool,
) {
    let place = |c: &mut Commands, pos: Vec3, dims: Vec3, angle: f32, index: usize| {
        block(
            c,
            cube,
            &stone[index % stone.len()],
            Vec3::new(x, 0., z) + pos * size,
            dims * size,
            Quat::from_rotation_z(angle),
        );
    };
    for (n, side) in [-1., 1.].into_iter().enumerate() {
        for row in 0..7 {
            let w = if row == 0 || row == 6 { 1.28 } else { 1. };
            for col in 0..2 {
                let offset = (col as f32 - 0.5) * 0.52 * w;
                place(
                    c,
                    Vec3::new(side * 3. + offset, row as f32 * 0.53 + 0.14, 0.),
                    Vec3::new(w * 0.51, 0.51, 1.3 + 0.04 * (row % 2) as f32),
                    0.,
                    n * 7 + row + col,
                );
            }
        }
        let point = |t: f32| Vec2::new(side * (3. - 3. * t * t), 3.59 + 4.4 * t - 0.85 * t * t);
        for j in 0..if broken && n == 0 { 3 } else { 10 } {
            let a = point(j as f32 / 10.);
            let b = point((j + 1) as f32 / 10.);
            let v = b - a;
            let p = (a + b) * 0.5;
            place(
                c,
                Vec3::new(p.x, p.y, 0.),
                Vec3::new(0.88, v.length() * 1.06, 1.26),
                -v.x.atan2(v.y),
                j + n * 7,
            );
            place(
                c,
                Vec3::new(p.x, p.y, -0.66),
                Vec3::new(0.5, v.length() * 1.03, 0.16),
                -v.x.atan2(v.y),
                j + n + 2,
            );
        }
    }
}

pub fn build(
    mut c: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut waters: ResMut<Assets<FenWater>>,
    assets: Res<AssetServer>,
) {
    let cube = meshes.add(Cuboid::from_size(Vec3::ONE));
    let rock = meshes.add(Sphere::new(1.).mesh().ico(1).unwrap());
    let rod = meshes.add(Cylinder::new(0.5, 1.).mesh().resolution(8));
    let tip = meshes.add(Cone::new(0.5, 1.).mesh().resolution(7));
    let v = Vec3::new;
    let id = Quat::IDENTITY;
    let mut surface = |name: &str, color: Color, glow: f32| {
        mats.add(StandardMaterial {
            base_color: color,
            base_color_texture: Some(assets.load(format!("stillwater/{name}.png"))),
            emissive_texture: Some(assets.load(format!("stillwater/{name}.png"))),
            emissive: LinearRgba::rgb(glow * 10., glow * 9.1, glow * 8.6),
            perceptual_roughness: 0.88,
            ..default()
        })
    };
    let stones: Vec<_> = [0.55, 0.70, 0.82, 0.62, 0.94]
        .map(|s| {
            surface(
                "limestone",
                Color::linear_rgb(s * 0.48, s * 1.04, s * 1.65),
                0.035 + s * 0.041,
            )
        })
        .into();
    let woods: Vec<_> = [0.42, 0.58, 0.79, 0.97]
        .map(|s| {
            surface(
                "oak",
                Color::srgb(s, s * 0.93, s * 0.86),
                0.018 + s * s * 0.072,
            )
        })
        .into();
    let dirt = surface("earth", Color::srgb(0.84, 0.8, 0.73), 0.036);
    let iron = super::pigment(&mut mats, [0.16, 0.19, 0.24], 0.18);
    let glass = super::pigment(&mut mats, [0.03, 0.62, 0.79], 1.9);
    let wick = super::pigment(&mut mats, [0.52, 0.98, 1.], 3.8);
    let amber = super::pigment(&mut mats, [1., 0.29, 0.032], 2.5);
    let honey = super::pigment(&mut mats, [1., 0.68, 0.10], 3.2);
    c.spawn((
        Silt,
        Mesh3d(meshes.add(Plane3d::default().mesh().size(170., 210.))),
        MeshMaterial3d(waters.add(FenWater {
            eye_time: Vec4::new(300., 2., 96., 0.),
            arches: INTACT_ARCHES.map(|(x, z, scale)| Vec4::new(x, z, scale, 0.)),
        })),
        Transform::from_xyz(300., -0.105, 0.),
    ));
    block(
        &mut c,
        &rock,
        &dirt,
        v(300., -0.26, 93.0),
        v(6.2, 0.48, 4.7),
        id,
    );
    // Pier boards sit at foot height along the existing northbound route.
    for j in 0..67 {
        let z = 92.1 - j as f32 * 0.38;
        block(
            &mut c,
            &cube,
            &woods[j % 4],
            v(
                300. + (hash(j) - 0.5) * 0.11,
                0.19 + hash(j + 17) * 0.026,
                z,
            ),
            v(2.28 + hash(j + 2) * 0.36, 0.16, 0.319),
            Quat::from_rotation_y((hash(j + 9) - 0.5) * 0.044),
        );
        if j % 6 == 0 {
            for side in [-1., 1.] {
                let h = 0.65 + hash(j + 5) * 0.37;
                block(
                    &mut c,
                    &rod,
                    &woods[(j + 2) % 4],
                    v(300. + side * 1.27, h * 0.5 - 0.22, z),
                    v(0.15, h, 0.16),
                    Quat::from_rotation_z(side * 0.045),
                );
            }
        }
    }
    let broken_arches = [
        (261., 46., 1.48, true),
        (321., 44., 0.96, true),
        (332., 40., 1.22, true),
    ];
    for (x, z, s, broken) in INTACT_ARCHES
        .map(|(x, z, s)| (x, z, s, false))
        .into_iter()
        .chain(broken_arches)
    {
        arch(&mut c, &cube, &stones, x, z, s, broken);
        // The outer arcade still carries portions of the original wall above its vaults.
        if x < 290. || x > 310. {
            for column in 0..11 {
                let dx = (column as f32 - 5.) * 0.84;
                let t = (1. - dx.abs() / 3.).clamp(0., 1.).sqrt();
                let sill = if dx.abs() > 3.45 {
                    -0.1
                } else {
                    3.59 + 4.4 * t - 0.85 * t * t
                };
                let crown = 7.15 + hash(column + x as usize) * 0.72
                    - if broken {
                        (dx + 2.0).max(0.) * 0.95
                    } else {
                        0.
                    };
                if crown > sill {
                    block(
                        &mut c,
                        &cube,
                        &stones[column % stones.len()],
                        v(x + dx * s, (sill + crown) * 0.5 * s, z - 0.16 * s),
                        v(0.85, crown - sill, 1.04) * s,
                        id,
                    );
                }
            }
        }
    }
    for (x, z, s) in [(302., 13., 5.1), (268., 29., 3.1), (339., 20., 2.7)] {
        kit(
            &mut c,
            &assets,
            "castle-kit",
            "tower-square",
            v(x, -0.12, z),
            s,
            0.,
        );
        kit(
            &mut c,
            &assets,
            "castle-kit",
            "tower-square-mid-open",
            v(x, s * 1.25, z),
            s,
            0.,
        );
        kit(
            &mut c,
            &assets,
            "castle-kit",
            "tower-square-top-roof-high",
            v(x, s * 2.17, z),
            s,
            0.,
        );
    }
    // Fallen nave walls step down from the belfry to the flooded transept.
    for (x, z, width, height, yaw) in [
        (295.8, 15., 5.8, 4.5, -0.12),
        (291.5, 17., 3.4, 2.9, -0.36),
        (308.2, 14., 7.2, 4.1, 0.19),
        (313.2, 16., 3.8, 2.5, 0.46),
    ] {
        block(
            &mut c,
            &cube,
            &stones[0],
            v(x, height * 0.5, z),
            v(width, height, 3.2),
            Quat::from_rotation_y(yaw),
        );
    }
    // Grazing light picks out the ashlar joints and broken outer buttresses.
    for (x, z, side) in [(300., 65., 1.), (281., 48., -1.), (321., 44., 1.)] {
        c.spawn((
            Silt,
            PointLight {
                color: Color::srgb(1., 0.82, 0.66),
                intensity: 1_850_000.,
                range: 29.,
                ..default()
            },
            Transform::from_xyz(x - 7., 8., z + 8.),
        ));
        for row in 0..8 {
            for col in 0..if row < 4 { 3 } else { 2 } {
                block(
                    &mut c,
                    &cube,
                    &stones[(row + col * 3) % stones.len()],
                    v(
                        x + side * (5.9 + col as f32 * 0.85),
                        row as f32 * 0.76,
                        z - 0.3,
                    ),
                    v(0.82, 0.72, 1.65),
                    Quat::from_rotation_z((hash(row + col) - 0.5) * 0.07),
                );
            }
        }
    }
    // Rush clumps have narrow ochre leaves above their darker flooded stems.
    let rush = [
        super::pigment(&mut mats, [0.35, 0.31, 0.14], 0.20),
        super::pigment(&mut mats, [0.23, 0.24, 0.13], 0.13),
    ];
    for i in 0..98 {
        let side = if i % 2 == 0 { -1. } else { 1. };
        let root = v(
            300. + side * (1.75 + hash(i + 83) * 11.),
            0.,
            94. - hash(i + 39) * 25.,
        );
        for j in 0..5 {
            let k = i * 5 + j;
            let h = 0.45 + hash(k + 77) * 0.79;
            block(
                &mut c,
                &tip,
                &rush[j % 2],
                root + v((hash(k + 3) - 0.5) * 0.65, h * 0.38, hash(k + 7) * 0.43),
                v(0.082, h, 0.05),
                Quat::from_rotation_z((hash(k + 1) - 0.5) * 0.69),
            );
        }
    }
    // Iron-bound gallows lamp, its core suspended clear of the timber.
    let post = v(302.58, 0., 93.10);
    for (offset, size, roll) in [
        (v(0., 1.4, 0.), v(0.23, 2.8, 0.24), 0.),
        (v(-0.20, 2.76, 0.), v(1.95, 0.21, 0.25), 0.),
        (v(-0.31, 2.40, 0.), v(0.14, 0.89, 0.17), 0.74),
    ] {
        block(
            &mut c,
            &cube,
            &woods[2],
            post + offset * 1.32,
            size * 1.32,
            Quat::from_rotation_z(roll),
        );
    }
    for y in [0.38, 1.68, 2.7] {
        block(
            &mut c,
            &cube,
            &iron,
            post + v(0., y * 1.32, 0.),
            1.55 * v(0.35, 0.14, 0.365),
            id,
        );
    }
    let lamp = post + v(-1.29, 2.64, 0.);
    block(
        &mut c,
        &rod,
        &iron,
        lamp + 1.55 * v(0., 0.48, 0.),
        1.55 * v(0.035, 0.38, 0.035),
        id,
    );
    for y in [-0.25, 0., 0.24] {
        block(
            &mut c,
            &rod,
            &iron,
            lamp + 1.55 * v(0., y, 0.),
            1.55 * v(0.39, 0.046, 0.39),
            id,
        );
    }
    for i in 0..6 {
        let a = i as f32 * std::f32::consts::TAU / 6.;
        block(
            &mut c,
            &rod,
            &iron,
            lamp + 1.55 * v(a.cos() * 0.177, 0., a.sin() * 0.177),
            1.55 * v(0.032, 0.51, 0.032),
            id,
        );
    }
    block(
        &mut c,
        &tip,
        &iron,
        lamp + 1.55 * v(0., 0.31, 0.),
        1.55 * v(0.44, 0.16, 0.44),
        id,
    );
    block(&mut c, &rock, &glass, lamp, v(0.16, 0.32, 0.16), id);
    block(
        &mut c,
        &rock,
        &wick,
        lamp + 1.55 * v(0., 0.018, 0.105),
        1.55 * v(0.055, 0.12, 0.052),
        id,
    );
    kit(
        &mut c,
        &assets,
        "castle-kit",
        "flag-banner-short",
        post + v(0.65, 2.12, 0.),
        1.88,
        1.57,
    );
    for (name, x, z, size, yaw) in [
        ("gravestone-bevel", 302.9, 92.5, 1.8, -0.21),
        ("gravestone-cross", 303.65, 91.55, 1.45, 0.31),
        ("rocks", 303.4, 92.9, 1.3, 0.7),
        ("fire-basket", 301.86, 93.25, 1.56, 0.2),
        ("bench-damaged", 297.7, 90.9, 1.32, -0.5),
    ] {
        kit(
            &mut c,
            &assets,
            "graveyard-kit",
            name,
            v(x, 0.08, z),
            size,
            yaw,
        );
    }
    kit(
        &mut c,
        &assets,
        "nature-kit",
        "canoe",
        v(296.75, 0.13, 91.4),
        3.4,
        0.83,
    );
    kit(
        &mut c,
        &assets,
        "nature-kit",
        "canoe_paddle",
        v(297.5, 0.14, 92.3),
        2.2,
        -0.61,
    );
    let staff = v(298.55, 0.06, 92.05);
    for (dx, y, length, tilt) in [
        (0., 0.73, 1.48, -0.09),
        (0.14, 1.55, 0.27, -0.66),
        (0.27, 1.64, 0.17, 1.47),
    ] {
        block(
            &mut c,
            &rod,
            &woods[2],
            staff + v(dx, y, 0.),
            v(0.058, length, 0.064),
            Quat::from_rotation_z(tilt),
        );
    }
    let marker = v(302.82, 0.76, 93.25);
    block(
        &mut c,
        &cube,
        &stones[3],
        marker,
        v(0.56, 1.62, 0.21),
        Quat::from_rotation_z(-0.25) * Quat::from_rotation_y(-0.25),
    );
    for i in 0..8 {
        let a = i as f32 * 2.4;
        let r = 0.12 * hash(i + 1);
        let base = v(
            301.86 + r * a.cos(),
            0.73 + hash(i) * 0.12,
            93.25 + r * a.sin(),
        );
        let scale = v(0.10 + hash(i) * 0.08, 0.27 + hash(i + 6) * 0.17, 0.15);
        c.spawn((
            Silt,
            Flame {
                base,
                phase: a,
                scale,
            },
            Mesh3d(tip.clone()),
            MeshMaterial3d(if i % 3 == 0 {
                honey.clone()
            } else {
                amber.clone()
            }),
            Transform::from_translation(base).with_scale(scale),
        ));
    }
    for (pos, color, intensity) in [
        (v(301.86, 1.06, 93.25), Color::srgb(1., 0.55, 0.24), 88000.),
        (lamp, Color::srgb(0.27, 0.79, 1.), 48000.),
    ] {
        c.spawn((
            Silt,
            PointLight {
                color,
                intensity,
                range: 10.,
                ..default()
            },
            Transform::from_translation(pos),
        ));
    }
    for i in 0..92 {
        let side = if i % 2 == 0 { -1. } else { 1. };
        let z = 95. - hash(i + 11) * 22.;
        let x = 300. + side * (2.35 + hash(i + 700) * 7.);
        let h = if (x - 300.).abs() < 5. && z > 89. {
            0.055
        } else {
            -0.19
        };
        let name = if i % 11 == 0 {
            "rock_smallFlatA"
        } else if i % 7 == 0 {
            "grass_leafsLarge"
        } else {
            "grass_large"
        };
        kit(
            &mut c,
            &assets,
            "nature-kit",
            name,
            v(x, h, z),
            1.2 + hash(i + 30) * 1.7,
            hash(i + 3) * 6.,
        );
    }
    for i in 0..36 {
        let x = 258. + hash(i + 24) * 83.;
        let z = 16. + hash(i + 291) * 49.;
        if (x - 300.).abs() < 5. {
            continue;
        }
        let name = if i % 3 == 0 {
            "gravestone-broken"
        } else if i % 3 == 1 {
            "stone-wall-damaged"
        } else {
            "pine-crooked"
        };
        kit(
            &mut c,
            &assets,
            "graveyard-kit",
            name,
            v(x, -0.28, z),
            1.8 + hash(i + 77) * 1.7,
            hash(i + 5) * 6.,
        );
    }
}

pub fn flicker(
    time: Res<Time>,
    camera: Query<&GlobalTransform, With<Camera3d>>,
    mut waters: ResMut<Assets<FenWater>>,
    mut flames: Query<(&Flame, &mut Transform)>,
) {
    let tick = (time.elapsed_secs() * 12.).floor();
    if let Ok(eye) = camera.single() {
        for (_, water) in waters.iter_mut() {
            water.eye_time = eye.translation().extend(time.elapsed_secs());
        }
    }
    for (flame, mut tf) in &mut flames {
        let pulse = (tick * 1.15 + flame.phase).sin();
        tf.scale = flame.scale * Vec3::new(1. - pulse * 0.12, 1. + pulse * 0.26, 1. - pulse * 0.12);
        tf.translation = flame.base + Vec3::new(pulse * 0.016, pulse * 0.026, 0.);
    }
}
