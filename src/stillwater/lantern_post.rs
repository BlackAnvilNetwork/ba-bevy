//! The drowned abbey and the short plank causeway at Pell's landing.
use super::{ClipCliffs, ClipLeaves, Silt};
use ba_protocol::stillwater::ground;
use bevy::{
    image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
};

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

fn relic(
    c: &mut Commands,
    assets: &AssetServer,
    path: &str,
    pos: Vec3,
    scale: f32,
    yaw: f32,
) {
    c.spawn((
        Silt,
        ClipCliffs,
        WorldAssetRoot(assets.load(format!("{path}#Scene0"))),
        Transform::from_translation(pos)
            .with_scale(Vec3::splat(scale))
            .with_rotation(Quat::from_rotation_y(yaw)),
    ));
}

fn ruined_keep(ashlar: &mut Ashlar, x: f32, z: f32, size: f32, seed: usize) {
    let y0 = ground(x, z);
    let unit = size * 0.32;
    let stories = 6 + seed % 3;
    for story in 0..stories {
        let y = y0 + story as f32 * unit * 1.02;
        let w = unit * (4.6 - story as f32 * 0.28);
        for (i, (dx, dz, yaw)) in [
            (0., 0.5, 0.),
            (0., -0.5, 0.),
            (0.5, 0., std::f32::consts::FRAC_PI_2),
            (-0.5, 0., std::f32::consts::FRAC_PI_2),
        ]
        .into_iter()
        .enumerate()
        {
            if hash(seed + story * 11 + i) < 0.1 + story as f32 * 0.09 {
                continue;
            }
            ashlar.place(
                Vec3::new(x + dx * w, y + unit * 0.52, z + dz * w),
                Vec3::new(w + unit * 0.14, unit, unit * 0.46),
                Quat::from_rotation_y(yaw),
                seed + story * 17 + i,
            );
        }
    }
    for k in 0..12 {
        let a = hash(seed + 80 + k) * std::f32::consts::TAU;
        let r = unit * (2.1 + hash(seed + 90 + k) * 2.8);
        ashlar.place(
            Vec3::new(
                x + a.cos() * r,
                y0 + hash(seed + k) * 0.42,
                z + a.sin() * r,
            ),
            Vec3::new(
                unit * (0.45 + hash(k + 3) * 0.7),
                unit * (0.22 + hash(k + 5) * 0.4),
                unit * (0.4 + hash(k + 8) * 0.55),
            ),
            Quat::from_rotation_y(a) * Quat::from_rotation_z((hash(k + 11) - 0.5) * 0.9),
            seed + 110 + k,
        );
    }
}

fn hash(i: usize) -> f32 {
    let mut n = (i as u32).wrapping_mul(0x9e3779b9);
    n ^= n >> 16;
    n = n.wrapping_mul(0x85ebca6b);
    (n >> 8) as f32 / 16777216.
}

fn world_uvs(mesh: &mut Mesh, scale: f32) {
    let Some(uvs) = mesh.attribute(Mesh::ATTRIBUTE_POSITION).and_then(|a| {
        a.as_float3().map(|pos| {
            let (mut min_x, mut max_x) = (f32::MAX, f32::MIN);
            let (mut min_z, mut max_z) = (f32::MAX, f32::MIN);
            for p in pos {
                min_x = min_x.min(p[0]);
                max_x = max_x.max(p[0]);
                min_z = min_z.min(p[2]);
                max_z = max_z.max(p[2]);
            }
            let along_x = (max_x - min_x) >= (max_z - min_z);
            pos.iter()
                .map(|p| {
                    if along_x {
                        [p[0] * scale, p[1] * scale]
                    } else {
                        [p[2] * scale, p[1] * scale]
                    }
                })
                .collect::<Vec<[f32; 2]>>()
        })
    }) else {
        return;
    };
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
}

struct Ashlar {
    cube: Mesh,
    batch: Option<Mesh>,
    mat: Handle<StandardMaterial>,
}

impl Ashlar {
    fn place(&mut self, pos: Vec3, size: Vec3, rot: Quat, seed: usize) {
        let j = hash(seed);
        let k = hash(seed + 19);
        let n = hash(seed + 41);
        let wobble = size.length() * 0.045;
        let pos = pos
            + Vec3::new((j - 0.5) * wobble, (k - 0.5) * wobble * 0.7, (n - 0.5) * wobble);
        let size = size
            * Vec3::new(0.86 + j * 0.28, 0.88 + k * 0.24, 0.86 + n * 0.28);
        let rot = rot
            * Quat::from_rotation_y((j - 0.5) * 0.18)
            * Quat::from_rotation_x((k - 0.5) * 0.12);
        let mut mesh = self.cube.clone().transformed_by(
            Transform::from_translation(pos)
                .with_scale(size)
                .with_rotation(rot),
        );
        world_uvs(&mut mesh, 0.28);
        match &mut self.batch {
            Some(b) => {
                let _ = b.merge(&mesh);
            }
            None => self.batch = Some(mesh),
        }
    }

    fn finish(self, c: &mut Commands, meshes: &mut Assets<Mesh>) {
        let Some(mesh) = self.batch else {
            return;
        };
        c.spawn((
            Silt,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(self.mat),
            Transform::IDENTITY,
        ));
    }
}

fn arch(ashlar: &mut Ashlar, x: f32, z: f32, size: f32, broken: bool) {
    let origin = Vec3::new(x, ground(x, z), z);
    let seed = (x * 13. + z * 17.) as usize;
    let place = |ashlar: &mut Ashlar, pos: Vec3, dims: Vec3, angle: f32, index: usize| {
        ashlar.place(
            origin + pos * size,
            dims * size,
            Quat::from_rotation_z(angle),
            seed + index,
        );
    };
    for (n, side) in [-1., 1.].into_iter().enumerate() {
        for row in 0..7 {
            let w = if row == 0 || row == 6 { 1.28 } else { 1. };
            for col in 0..2 {
                let offset = (col as f32 - 0.5) * 0.52 * w;
                place(
                    ashlar,
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
                ashlar,
                Vec3::new(p.x, p.y, 0.),
                Vec3::new(0.88, v.length() * 1.06, 1.26),
                -v.x.atan2(v.y),
                j + n * 7,
            );
            place(
                ashlar,
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
    let cube_mesh: Mesh = Cuboid::from_size(Vec3::ONE).into();
    let cube = meshes.add(cube_mesh.clone());
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
    let castle = assets
        .load_builder()
        .with_settings(|s: &mut ImageLoaderSettings| {
            s.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::Repeat,
                address_mode_v: ImageAddressMode::Repeat,
                ..ImageSamplerDescriptor::linear()
            });
        })
        .load("darkmod/stone/castle_blocks.jpg");
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
    let stone_mat = mats.add(StandardMaterial {
        base_color: Color::srgb(0.84, 0.82, 0.78),
        base_color_texture: Some(castle),
        perceptual_roughness: 0.93,
        ..default()
    });
    let mut ashlar = Ashlar {
        cube: cube_mesh.clone(),
        batch: None,
        mat: stone_mat.clone(),
    };
    let iron = super::pigment(&mut mats, [0.16, 0.19, 0.24], 0.18);
    let glass = super::pigment(&mut mats, [0.03, 0.62, 0.79], 1.9);
    let wick = super::pigment(&mut mats, [0.52, 0.98, 1.], 3.8);
    let amber = super::pigment(&mut mats, [1., 0.29, 0.032], 2.5);
    let honey = super::pigment(&mut mats, [1., 0.68, 0.10], 3.2);
    let _ = waters.add(FenWater {
        eye_time: Vec4::new(300., 2., 96., 0.),
        arches: INTACT_ARCHES.map(|(x, z, scale)| Vec4::new(x, z, scale, 0.)),
    });
    block(
        &mut c,
        &rock,
        &dirt,
        v(300., ground(300., 93.) - 0.26, 93.0),
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
                ground(300., z) + 0.19 + hash(j + 17) * 0.026,
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
                    v(300. + side * 1.27, ground(300., z) + h * 0.5 - 0.22, z),
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
        arch(&mut ashlar, x, z, s, broken);
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
                    ashlar.place(
                        v(x + dx * s, (sill + crown) * 0.5 * s, z - 0.16 * s),
                        v(0.85, crown - sill, 1.04) * s,
                        id,
                        column + x as usize,
                    );
                }
            }
        }
    }
    for (i, (x, z, s)) in [(302., 13., 5.1), (268., 29., 3.1), (339., 20., 2.7)]
        .into_iter()
        .enumerate()
    {
        let x = x + (hash(i + 3) - 0.5) * 1.6;
        let z = z + (hash(i + 11) - 0.5) * 1.4;
        ruined_keep(&mut ashlar, x, z, s, i * 91 + 4);
        relic(
            &mut c,
            &assets,
            "darkmod/arches/stone_arch01.glb",
            v(
                x + (hash(i + 21) - 0.5) * 2.4,
                ground(x, z) - 0.2,
                z + (hash(i + 29) - 0.5) * 2.2,
            ),
            0.014 + hash(i + 7) * 0.008,
            hash(i + 5) * std::f32::consts::TAU,
        );
    }
    // Fallen nave walls step down from the belfry to the flooded transept.
    for (x, z, width, height, yaw) in [
        (295.8, 15., 5.8, 4.5, -0.12),
        (291.5, 17., 3.4, 2.9, -0.36),
        (308.2, 14., 7.2, 4.1, 0.19),
        (313.2, 16., 3.8, 2.5, 0.46),
    ] {
        ashlar.place(
            v(x, height * 0.5, z),
            v(width, height, 3.2),
            Quat::from_rotation_y(yaw),
            (x + z) as usize,
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
                ashlar.place(
                    v(
                        x + side * (5.9 + col as f32 * 0.85),
                        row as f32 * 0.76,
                        z - 0.3,
                    ),
                    v(0.82, 0.72, 1.65),
                    Quat::from_rotation_z((hash(row + col) - 0.5) * 0.07),
                    row * 3 + col + x as usize,
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
    block(
        &mut c,
        &cube,
        &woods[1],
        post + v(0.62, 2.55, 0.),
        v(0.04, 1.15, 0.62),
        Quat::from_rotation_y(1.52) * Quat::from_rotation_z(0.18),
    );
    for (i, (path, x, z, size, yaw)) in [
        ("darkmod/graveyard/grave_simple_1.glb", 302.9, 92.5, 0.034, -0.21),
        ("darkmod/graveyard/grave_simple_3.glb", 303.65, 91.55, 0.03, 0.31),
        ("darkmod/graveyard/grave_rect_ruined.glb", 303.4, 92.9, 0.032, 0.7),
        ("darkmod/graveyard/urn.glb", 301.86, 93.25, 0.028, 0.2),
    ]
    .into_iter()
    .enumerate()
    {
        relic(
            &mut c,
            &assets,
            path,
            v(
                x + (hash(i + 40) - 0.5) * 0.28,
                ground(x, z) - 0.08,
                z + (hash(i + 51) - 0.5) * 0.22,
            ),
            size * (0.9 + hash(i + 8) * 0.22),
            yaw + (hash(i + 17) - 0.5) * 0.4,
        );
    }
    // Damaged bench from stacked oak, not a Kenney stamp.
    let bench = v(297.7, ground(297.7, 90.9) + 0.22, 90.9);
    block(&mut c, &cube, &woods[0], bench, v(1.35, 0.09, 0.38), Quat::from_rotation_y(-0.5));
    for side in [-0.52, 0.52] {
        block(
            &mut c,
            &cube,
            &woods[2],
            bench + v(side, -0.18, 0.12),
            v(0.08, 0.34, 0.08),
            Quat::IDENTITY,
        );
        block(
            &mut c,
            &cube,
            &woods[2],
            bench + v(side, -0.18, -0.12),
            v(0.08, 0.34, 0.08),
            Quat::IDENTITY,
        );
    }
    let hull = v(296.75, 0.16, 91.4);
    block(
        &mut c,
        &cube,
        &woods[3],
        hull,
        v(2.15, 0.16, 0.62),
        Quat::from_rotation_y(0.83),
    );
    block(
        &mut c,
        &cube,
        &woods[1],
        hull + v(0., 0.14, 0.),
        v(2.05, 0.08, 0.48),
        Quat::from_rotation_y(0.83),
    );
    block(
        &mut c,
        &rod,
        &woods[2],
        v(297.5, 0.22, 92.3),
        v(0.04, 1.15, 0.04),
        Quat::from_rotation_z(-0.61) * Quat::from_rotation_x(1.1),
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
    ashlar.place(
        marker,
        v(0.56, 1.62, 0.21),
        Quat::from_rotation_z(-0.25) * Quat::from_rotation_y(-0.25),
        93,
    );
    ashlar.finish(&mut c, &mut meshes);
    let mut rubble = Ashlar {
        cube: cube_mesh,
        batch: None,
        mat: stone_mat,
    };
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
    const GRAVES: [&str; 7] = [
        "darkmod/graveyard/grave_simple_1.glb",
        "darkmod/graveyard/grave_simple_2.glb",
        "darkmod/graveyard/grave_simple_4.glb",
        "darkmod/graveyard/grave_simple_5.glb",
        "darkmod/graveyard/grave_rect_ruined.glb",
        "darkmod/graveyard/grave_big_1.glb",
        "darkmod/graveyard/grave_wall_1.glb",
    ];
    for i in 0..36 {
        let x = 258. + hash(i + 24) * 83. + (hash(i + 61) - 0.5) * 2.4;
        let z = 16. + hash(i + 291) * 49. + (hash(i + 73) - 0.5) * 2.1;
        if (x - 300.).abs() < 5. {
            continue;
        }
        if i % 3 == 0 {
            relic(
                &mut c,
                &assets,
                GRAVES[i % GRAVES.len()],
                v(x, ground(x, z) - 0.12, z),
                0.028 + hash(i + 77) * 0.016,
                hash(i + 5) * 6.,
            );
        } else if i % 3 == 1 {
            rubble.place(
                v(x, ground(x, z) + 0.55, z),
                v(
                    1.4 + hash(i + 4) * 1.1,
                    0.7 + hash(i + 9) * 1.3,
                    0.42 + hash(i + 13) * 0.3,
                ),
                Quat::from_rotation_y(hash(i + 5) * 6.)
                    * Quat::from_rotation_z((hash(i + 19) - 0.5) * 0.22),
                i * 13 + 8,
            );
        } else {
            let birch = ["birch_02", "birch_03", "birch_04", "birch_05", "birch_06"][i % 5];
            c.spawn((
                Silt,
                ClipLeaves,
                WorldAssetRoot(assets.load(format!("darkmod/trees/{birch}.glb#Scene0"))),
                Transform::from_translation(v(x, ground(x, z) - 0.55, z))
                    .with_scale(Vec3::new(
                        if hash(i + 11) > 0.5 { -1. } else { 1. }
                            * (0.032 + hash(i + 77) * 0.016),
                        0.032 + hash(i + 77) * 0.016,
                        0.032 + hash(i + 77) * 0.016,
                    ))
                    .with_rotation(Quat::from_euler(
                        EulerRot::YXZ,
                        hash(i + 5) * 6.,
                        (hash(i + 19) - 0.5) * 0.12,
                        (hash(i + 31) - 0.5) * 0.1,
                    )),
            ));
        }
    }
    rubble.finish(&mut c, &mut meshes);
}

pub fn flicker(
    time: Res<Time>,
    camera: Query<&GlobalTransform, With<crate::camera_rig::PlayerCamera>>,
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
