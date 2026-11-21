use bevy::{
    image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
};

use ba_protocol::stillwater::{ground, GREYMERE_ROAD};
use crate::atmosphere::{self, StickAtmosphere};
use crate::boss::WardenBody;
use crate::data::{GameData, ZoneProp};
use crate::stillwater::{ClipCliffs, ClipLeaves};
use crate::well;

/// The yard's keeper. Talking to him starts the opening.
#[derive(Component)]
pub struct Warden;

/// Scene root marker for cleanup later.
#[derive(Component)]
pub struct OpeningScene;

pub fn spawn_opening_scene(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    assets: Res<AssetServer>,
    data: Res<GameData>,
    atmo: Res<StickAtmosphere>,
) {
    commands.spawn((
        OpeningScene,
        Name::new(data.scene.display_name.clone()),
        Transform::IDENTITY,
        Visibility::default(),
    ));

    let chalk_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.92, 0.88, 0.72),
        emissive: LinearRgba::new(0.35, 0.32, 0.22, 1.),
        perceptual_roughness: 1.0,
        ..default()
    });

    spawn_yard_ground(&mut commands, &mut meshes, &mut materials, &assets);

    for prop in &data.zone.props {
        spawn_prop(
            &mut commands,
            &mut meshes,
            &mut materials,
            &chalk_mat,
            &assets,
            prop,
        );
    }
    dress_yard(&mut commands, &mut meshes, &mut materials, &assets);

    atmosphere::spawn_sun(&mut commands, &atmo);
    atmosphere::spawn_ambient(&mut commands, &atmo);
    atmosphere::spawn_moon_fill(&mut commands, &atmo);
    atmosphere::spawn_well_fire(&mut commands);
    spawn_fireflies(&mut commands, &mut meshes, &mut materials);
}

fn spawn_yard_ground(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    assets: &AssetServer,
) {
    let grass = assets
        .load_builder()
        .with_settings(|s: &mut ImageLoaderSettings| {
            s.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::Repeat,
                address_mode_v: ImageAddressMode::Repeat,
                ..ImageSamplerDescriptor::linear()
            });
        })
        .load("darkmod/stone/dirt_leaves_roots_forest_floor_d.png");
    commands.spawn((
        Name::new("Ground"),
        Mesh3d(meshes.add(yard_floor())),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(0.52, 0.56, 0.38),
            base_color_texture: Some(grass),
            perceptual_roughness: 0.96,
            cull_mode: None,
            ..default()
        })),
        Transform::IDENTITY,
    ));
}

fn yard_floor() -> Mesh {
    let cols = 72usize;
    let rows = 64usize;
    let (x0, x1, z0, z1) = (-48., 62., -44., 50.);
    let mut positions = Vec::with_capacity((cols + 1) * (rows + 1));
    let mut uvs = Vec::with_capacity((cols + 1) * (rows + 1));
    for j in 0..=rows {
        let z = z0 + (z1 - z0) * j as f32 / rows as f32;
        for i in 0..=cols {
            let x = x0 + (x1 - x0) * i as f32 / cols as f32;
            positions.push([x, ground(x, z), z]);
            uvs.push([x * 0.11, z * 0.11]);
        }
    }
    let mut indices = Vec::with_capacity(cols * rows * 6);
    let stride = cols + 1;
    for j in 0..rows {
        for i in 0..cols {
            let a = (j * stride + i) as u32;
            let b = a + 1;
            let c = a + stride as u32;
            let d = c + 1;
            indices.extend_from_slice(&[a, c, b, b, c, d]);
        }
    }
    let mut normals = vec![[0., 1., 0.]; positions.len()];
    for tri in indices.chunks(3) {
        let a = Vec3::from(positions[tri[0] as usize]);
        let b = Vec3::from(positions[tri[1] as usize]);
        let c = Vec3::from(positions[tri[2] as usize]);
        let n = (b - a).cross(c - a);
        for idx in tri {
            let slot = &mut normals[*idx as usize];
            *slot = (Vec3::from(*slot) + n).to_array();
        }
    }
    for n in &mut normals {
        *n = Vec3::from(*n).normalize_or_zero().to_array();
    }
    let mut mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        bevy::asset::RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::mesh::Indices::U32(indices));
    mesh
}

fn spawn_fireflies(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let mesh = meshes.add(Sphere::new(0.045));
    let warm = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.82, 0.35),
        emissive: LinearRgba::rgb(4.0, 1.8, 0.25),
        unlit: true,
        ..default()
    });
    let cool = materials.add(StandardMaterial {
        base_color: Color::srgb(0.65, 1.0, 0.55),
        emissive: LinearRgba::rgb(0.4, 2.2, 0.7),
        unlit: true,
        ..default()
    });
    let spots = [
        (1.35, 0.85, -0.7, 0.2, true),
        (0.7, 1.3, -1.1, 1.4, true),
        (1.8, 0.55, -0.4, 2.2, true),
        (-2.4, 1.6, 1.2, 0.6, false),
        (-6.5, 2.1, 2.4, 1.1, false),
        (6.8, 1.4, 1.6, 2.6, false),
        (4.2, 1.9, -2.2, 0.9, false),
        (-4.0, 1.2, -3.4, 1.8, true),
        (9.2, 1.7, -1.2, 3.1, false),
        (-8.6, 2.4, -1.5, 0.4, false),
        (2.8, 0.9, 2.6, 2.0, true),
        (-1.2, 1.5, 3.4, 1.5, false),
        (7.4, 0.8, 3.1, 0.8, true),
        (-9.5, 1.8, 4.2, 2.8, false),
        (11.0, 1.3, 2.2, 1.2, false),
        (0.4, 1.1, -3.6, 2.4, true),
    ];
    for (x, y, z, phase, is_warm) in spots {
        let origin = Vec3::new(x, y + ground(x, z), z);
        commands.spawn((
            Name::new("Firefly"),
            Firefly { origin, phase },
            Mesh3d(mesh.clone()),
            MeshMaterial3d(if is_warm { warm.clone() } else { cool.clone() }),
            Transform::from_translation(origin),
        ));
    }
}

#[derive(Component)]
pub(crate) struct Firefly {
    origin: Vec3,
    phase: f32,
}

pub fn tick_fireflies(time: Res<Time>, mut motes: Query<(&Firefly, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (mote, mut transform) in &mut motes {
        let bob = (t * 1.4 + mote.phase).sin();
        transform.translation = mote.origin
            + Vec3::new(
                (t * 0.7 + mote.phase).sin() * 0.18,
                bob * 0.1,
                (t * 0.5 + mote.phase * 1.7).cos() * 0.18,
            );
    }
}

fn spawn_prop(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    chalk_mat: &Handle<StandardMaterial>,
    assets: &AssetServer,
    prop: &ZoneProp,
) {
    if prop.model.starts_with("kits/") {
        return;
    }
    let y = prop.at[1] + ground(prop.at[0], prop.at[2]);
    let transform = Transform::from_translation(Vec3::new(prop.at[0], y, prop.at[2]))
        .with_rotation(Quat::from_rotation_y(prop.rot_y))
        .with_scale(Vec3::splat(prop.scale));

    if let Some(kind) = prop.model.strip_prefix("primitive:") {
        if kind == "well" {
            well::spawn_well(commands, meshes, materials, transform);
            return;
        }
        let mesh = match kind {
            "chalk" => meshes.add(Cuboid::from_size(Vec3::from_array(prop.size))),
            _ => return,
        };
        commands.spawn((
            Name::new(prop.name.clone()),
            Mesh3d(mesh),
            MeshMaterial3d(chalk_mat.clone()),
            transform,
        ));
        return;
    }

    let model = prop.model.as_str();
    let foliage = model.contains("/trees/");
    let scene = assets.load(format!("{model}#Scene0"));
    let mut entity = commands.spawn((
        Name::new(prop.name.clone()),
        WorldAssetRoot(scene),
        transform,
    ));
    if foliage {
        entity.insert(ClipLeaves);
    } else if model.starts_with("darkmod/") {
        entity.insert(ClipCliffs);
    }
    if prop.role.as_deref() == Some("warden") {
        entity.insert((
            Warden,
            WardenBody {
                home: transform,
                laugh_until: 0.0,
                lean_until: 0.0,
                lean_dir: Vec3::NEG_Z,
            },
        ));
    }
}

fn yard_hash(i: usize) -> f32 {
    let mut n = (i as u32).wrapping_mul(0x9e3779b9);
    n ^= n >> 16;
    n = n.wrapping_mul(0x85ebca6b);
    (n >> 8) as f32 / 16_777_216.
}

fn relic(
    commands: &mut Commands,
    assets: &AssetServer,
    path: &str,
    pos: Vec3,
    scale: f32,
    yaw: f32,
    foliage: bool,
) {
    let mut e = commands.spawn((
        Name::new(path.to_string()),
        WorldAssetRoot(assets.load(format!("{path}#Scene0"))),
        Transform::from_translation(pos)
            .with_scale(Vec3::splat(scale))
            .with_rotation(Quat::from_rotation_y(yaw)),
    ));
    if foliage {
        e.insert(ClipLeaves);
    } else {
        e.insert(ClipCliffs);
    }
}

struct Ashlar {
    cube: Mesh,
    batch: Option<Mesh>,
    mat: Handle<StandardMaterial>,
}

impl Ashlar {
    fn place(&mut self, pos: Vec3, size: Vec3, rot: Quat, seed: usize) {
        let j = yard_hash(seed);
        let k = yard_hash(seed + 19);
        let n = yard_hash(seed + 41);
        let wobble = size.length() * 0.04;
        let pos = pos
            + Vec3::new((j - 0.5) * wobble, (k - 0.5) * wobble * 0.55, (n - 0.5) * wobble);
        let size = size * Vec3::new(0.88 + j * 0.24, 0.9 + k * 0.2, 0.88 + n * 0.24);
        let rot = rot
            * Quat::from_rotation_y((j - 0.5) * 0.16)
            * Quat::from_rotation_z((k - 0.5) * 0.1);
        let mut mesh = self.cube.clone().transformed_by(
            Transform::from_translation(pos)
                .with_scale(size)
                .with_rotation(rot),
        );
        if let Some(uvs) = mesh.attribute(Mesh::ATTRIBUTE_POSITION).and_then(|a| {
            a.as_float3().map(|pts| {
                pts.iter()
                    .map(|p| [p[0] * 0.28, p[1] * 0.28])
                    .collect::<Vec<[f32; 2]>>()
            })
        }) {
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        }
        match &mut self.batch {
            Some(b) => {
                let _ = b.merge(&mesh);
            }
            None => self.batch = Some(mesh),
        }
    }

    fn finish(self, commands: &mut Commands, meshes: &mut Assets<Mesh>) {
        let Some(mesh) = self.batch else {
            return;
        };
        commands.spawn((
            Name::new("YardAshlar"),
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(self.mat),
            Transform::IDENTITY,
        ));
    }
}

fn crypt(ashlar: &mut Ashlar, x: f32, z: f32, seed: usize) {
    let y0 = ground(x, z);
    let w = 1.55 + yard_hash(seed) * 0.4;
    let d = 1.9 + yard_hash(seed + 3) * 0.35;
    let h = 1.35 + yard_hash(seed + 5) * 0.4;
    ashlar.place(
        Vec3::new(x, y0 + h * 0.5, z),
        Vec3::new(w, h, d),
        Quat::from_rotation_y((yard_hash(seed + 7) - 0.5) * 0.35),
        seed,
    );
    ashlar.place(
        Vec3::new(x, y0 + h + 0.12, z),
        Vec3::new(w + 0.35, 0.22, d + 0.28),
        Quat::from_rotation_y((yard_hash(seed + 9) - 0.5) * 0.2),
        seed + 11,
    );
}

fn dress_yard(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    assets: &AssetServer,
) {
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
    let mut ashlar = Ashlar {
        cube: Cuboid::from_size(Vec3::ONE).into(),
        batch: None,
        mat: materials.add(StandardMaterial {
            base_color: Color::srgb(0.84, 0.82, 0.78),
            base_color_texture: Some(castle),
            perceptual_roughness: 0.93,
            ..default()
        }),
    };
    // Broken cloister — gaps and jitter, not a Kenney stamp ring.
    for i in 0..14 {
        if yard_hash(i + 4) < 0.22 {
            continue;
        }
        let t = i as f32 / 14.;
        let x = -9.5 + t * 18. + (yard_hash(i) - 0.5) * 1.1;
        let z = -6.8 + (yard_hash(i + 8) - 0.5) * 0.8;
        ashlar.place(
            Vec3::new(x, ground(x, z) + 0.55 + yard_hash(i + 2) * 0.7, z),
            Vec3::new(
                1.15 + yard_hash(i + 6) * 0.7,
                1.05 + yard_hash(i + 9) * 1.1,
                0.42,
            ),
            Quat::from_rotation_y((yard_hash(i + 11) - 0.5) * 0.28),
            i * 17,
        );
    }
    for i in 0..8 {
        if yard_hash(i + 40) < 0.28 {
            continue;
        }
        let z = -5.4 + i as f32 * 1.35 + (yard_hash(i + 44) - 0.5) * 0.6;
        let x = -10.2 + (yard_hash(i + 48) - 0.5) * 0.7;
        ashlar.place(
            Vec3::new(x, ground(x, z) + 0.5 + yard_hash(i + 50) * 0.55, z),
            Vec3::new(0.4, 0.95 + yard_hash(i + 52) * 0.9, 1.2),
            Quat::from_rotation_y(1.52 + (yard_hash(i + 55) - 0.5) * 0.2),
            i * 23 + 9,
        );
    }
    crypt(&mut ashlar, -5.2, -4.6, 3);
    crypt(&mut ashlar, 2.8, -5.3, 8);
    crypt(&mut ashlar, 9.4, -4.4, 14);
    ashlar.finish(commands, meshes);

    const GRAVES: [&str; 6] = [
        "darkmod/graveyard/grave_simple_1.glb",
        "darkmod/graveyard/grave_simple_2.glb",
        "darkmod/graveyard/grave_simple_4.glb",
        "darkmod/graveyard/grave_simple_5.glb",
        "darkmod/graveyard/grave_rect_ruined.glb",
        "darkmod/graveyard/grave_rectangle.glb",
    ];
    for i in 0..10 {
        let a = yard_hash(i + 70) * std::f32::consts::TAU;
        let r = 4.2 + yard_hash(i + 80) * 3.4;
        let x = a.cos() * r + (yard_hash(i + 90) - 0.5) * 0.6;
        let z = a.sin() * r - 1.4 + (yard_hash(i + 100) - 0.5) * 0.6;
        if z > 3.5
            || ((x - GREYMERE_ROAD.x).abs() < 5. && (z - GREYMERE_ROAD.z).abs() < 4.)
        {
            continue;
        }
        relic(
            commands,
            assets,
            GRAVES[i % GRAVES.len()],
            Vec3::new(x, ground(x, z) - 0.04, z),
            0.012 + yard_hash(i + 110) * 0.006,
            a + std::f32::consts::PI + (yard_hash(i + 120) - 0.5) * 0.5,
            false,
        );
    }
    relic(
        commands,
        assets,
        "darkmod/graveyard/sarcophagus_ruined.glb",
        Vec3::new(-5.2, ground(-5.2, -3.8) - 0.02, -3.8),
        0.014,
        0.25,
        false,
    );
    relic(
        commands,
        assets,
        "darkmod/graveyard/sarcophagus_closed.glb",
        Vec3::new(2.8, ground(2.8, -4.4) - 0.02, -4.4),
        0.013,
        -0.15,
        false,
    );
    relic(
        commands,
        assets,
        "darkmod/graveyard/urn.glb",
        Vec3::new(-4.1, ground(-4.1, -3.2), -3.2),
        0.015,
        0.8,
        false,
    );

    const TREES: [&str; 8] = [
        "birch_02",
        "birch_03",
        "birch_04",
        "birch_05",
        "spruce_01a",
        "spruce_01c",
        "spruce_01e",
        "pine_12",
    ];
    for i in 0..18 {
        let a = i as f32 / 18. * std::f32::consts::TAU + (yard_hash(i + 130) - 0.5) * 0.35;
        let r = 34. + yard_hash(i + 140) * 6.;
        let x = 8.5 + a.cos() * r;
        let z = 1.2 + a.sin() * r;
        if (x - GREYMERE_ROAD.x).abs() < 6. && (z - GREYMERE_ROAD.z).abs() < 6. {
            continue;
        }
        let scale = 0.022 + yard_hash(i + 150) * 0.012;
        relic(
            commands,
            assets,
            &format!("darkmod/trees/{}.glb", TREES[i % TREES.len()]),
            Vec3::new(x, ground(x, z) - scale * 14., z),
            scale,
            yard_hash(i + 160) * std::f32::consts::TAU,
            true,
        );
    }

    const ROCKS: [&str; 8] = [
        "cliffrock_01",
        "cliffrock_02",
        "cliffrock_03",
        "cliffrock_04",
        "cliffrock_05",
        "cliffrock_06",
        "cliffrock_07",
        "cliffrock_08",
    ];
    for i in 0..12 {
        let a = i as f32 / 12. * std::f32::consts::TAU + (yard_hash(i + 170) - 0.5) * 0.28;
        let r = 40. + yard_hash(i + 180) * 5.;
        let x = 8.5 + a.cos() * r;
        let z = 1.2 + a.sin() * r;
        if (x - GREYMERE_ROAD.x).abs() < 6. && (z - GREYMERE_ROAD.z).abs() < 6. {
            continue;
        }
        relic(
            commands,
            assets,
            &format!("darkmod/rocks/{}.glb", ROCKS[i % ROCKS.len()]),
            Vec3::new(x, ground(x, z) - 0.6, z),
            0.012 + yard_hash(i + 190) * 0.01,
            a + std::f32::consts::PI,
            false,
        );
    }
}

