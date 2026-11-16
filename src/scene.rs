use bevy::prelude::*;

use crate::atmosphere::{self, StickAtmosphere};
use crate::data::{GameData, ZoneProp};
use crate::boss::WardenBody;
use crate::tree::{self, TreeMaterials};
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
    mut images: ResMut<Assets<Image>>,
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

    let dirt = materials.add(StandardMaterial {
        base_color: Color::WHITE,
        base_color_texture: Some(images.add(yard_ground_image())),
        perceptual_roughness: 1.0,
        metallic: 0.0,
        uv_transform: bevy::math::Affine2::from_scale(Vec2::splat(5.0)),
        ..default()
    });
    let chalk_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.78, 0.74, 0.62),
        perceptual_roughness: 1.0,
        ..default()
    });
    let trees = TreeMaterials {
        bark: materials.add(StandardMaterial {
            base_color: Color::srgb(0.3, 0.22, 0.15),
            perceptual_roughness: 1.0,
            ..default()
        }),
        deep: materials.add(StandardMaterial {
            base_color: Color::srgb(0.2, 0.4, 0.18),
            perceptual_roughness: 1.0,
            cull_mode: None,
            ..default()
        }),
        leaf: materials.add(StandardMaterial {
            base_color: Color::srgb(0.36, 0.58, 0.24),
            perceptual_roughness: 1.0,
            cull_mode: None,
            ..default()
        }),
    };

    let bounds = &data.zone.bounds;
    let center = Vec3::new(
        (bounds.min_x + bounds.max_x) * 0.5,
        0.0,
        (bounds.min_z + bounds.max_z) * 0.5,
    );
    let half = Vec2::new(
        (bounds.max_x - bounds.min_x) * 0.5 + 16.0,
        (bounds.max_z - bounds.min_z) * 0.5 + 16.0,
    );
    commands.spawn((
        Name::new("Ground"),
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, half))),
        MeshMaterial3d(dirt),
        Transform::from_translation(center),
    ));

    for prop in &data.zone.props {
        spawn_prop(
            &mut commands,
            &mut meshes,
            &mut materials,
            &chalk_mat,
            &trees,
            &assets,
            prop,
        );
    }

    atmosphere::spawn_sun(&mut commands, &atmo);
    atmosphere::spawn_ambient(&mut commands, &atmo);
    atmosphere::spawn_moon_fill(&mut commands);
    atmosphere::spawn_well_fire(&mut commands);
    spawn_fireflies(&mut commands, &mut meshes, &mut materials);
}

fn yard_ground_image() -> Image {
    const N: usize = 64;
    const PALETTE: [[u8; 4]; 6] = [
        [10, 22, 16, 255],
        [24, 64, 30, 255],
        [48, 112, 42, 255],
        [132, 168, 62, 255],
        [72, 48, 28, 255],
        [28, 36, 34, 255],
    ];
    let mut data = vec![0u8; N * N * 4];
    for y in 0..N {
        for x in 0..N {
            let cell = hash2((x / 8) as i32, (y / 8) as i32);
            let grain = hash2(x as i32, y as i32);
            let mut index = (cell % 4) as usize;
            if grain % 11 == 0 {
                index = 4;
            } else if grain % 9 == 0 {
                index = 5;
            } else if grain % 5 == 0 {
                index = 3;
            }
            let px = &PALETTE[index];
            let i = (y * N + x) * 4;
            data[i..i + 4].copy_from_slice(px);
        }
    }
    let mut image = Image::new(
        bevy::render::render_resource::Extent3d {
            width: N as u32,
            height: N as u32,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        data,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::MAIN_WORLD | bevy::asset::RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = bevy::image::ImageSampler::nearest();
    image
}

fn hash2(x: i32, y: i32) -> u32 {
    let mut n = (x as u32).wrapping_mul(374761393) ^ (y as u32).wrapping_mul(668265263);
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    n ^ (n >> 16)
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
        commands.spawn((
            Name::new("Firefly"),
            Firefly {
                origin: Vec3::new(x, y, z),
                phase,
            },
            Mesh3d(mesh.clone()),
            MeshMaterial3d(if is_warm { warm.clone() } else { cool.clone() }),
            Transform::from_xyz(x, y, z),
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
    trees: &TreeMaterials,
    assets: &AssetServer,
    prop: &ZoneProp,
) {
    let transform = Transform::from_translation(Vec3::from_array(prop.at))
        .with_rotation(Quat::from_rotation_y(prop.rot_y))
        .with_scale(Vec3::splat(prop.scale));

    if tree::standing_tree(&prop.model) {
        tree::spawn_tree(commands, meshes, trees, prop, transform);
        return;
    }

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

    let scene = assets.load(format!("{}#Scene0", prop.model));
    let mut entity = commands.spawn((
        Name::new(prop.name.clone()),
        WorldAssetRoot(scene),
        transform,
    ));
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
    if prop.model.contains("crypt-large.glb") && prop.name.starts_with("House") {
        finish_king_mausoleum(commands, meshes, materials, prop);
    }
}

/// Roof, floor, steps, pillars, and pediment for a crypt-large king house.
fn finish_king_mausoleum(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    prop: &ZoneProp,
) {
    let s = prop.scale;
    let slate = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.22, 0.26),
        perceptual_roughness: 1.0,
        ..default()
    });
    let timber = materials.add(StandardMaterial {
        base_color: Color::srgb(0.34, 0.23, 0.14),
        perceptual_roughness: 1.0,
        ..default()
    });
    let stone = materials.add(StandardMaterial {
        base_color: Color::srgb(0.4, 0.39, 0.37),
        perceptual_roughness: 1.0,
        ..default()
    });
    let stone_dark = materials.add(StandardMaterial {
        base_color: Color::srgb(0.28, 0.27, 0.26),
        perceptual_roughness: 1.0,
        ..default()
    });
    let block = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let origin = Vec3::from_array(prop.at);
    let yaw = Quat::from_rotation_y(prop.rot_y);
    let place = |local: Vec3, rotation: Quat, scale: Vec3| {
        Transform::from_translation(origin + yaw * local)
            .with_rotation(yaw * rotation)
            .with_scale(scale)
    };
    let mut part = |name: &str, mat: Handle<StandardMaterial>, local: Vec3, rotation: Quat, scale: Vec3| {
        commands.spawn((
            Name::new(format!("{} {name}", prop.name)),
            Mesh3d(block.clone()),
            MeshMaterial3d(mat),
            place(local, rotation, scale),
        ));
    };

    // Floor slab inside the crypt cell.
    part(
        "Floor",
        stone_dark.clone(),
        Vec3::new(0.0, 0.04, 0.05),
        Quat::IDENTITY,
        Vec3::new(s * 1.7, s * 0.08, s * 2.0),
    );

    // Steps out toward the path (+Z).
    for (i, (z, y, depth)) in [
        (1.35, 0.08, 0.38),
        (1.62, 0.04, 0.38),
    ]
    .into_iter()
    .enumerate()
    {
        part(
            &format!("Step{i}"),
            stone.clone(),
            Vec3::new(0.0, y * s, z * s),
            Quat::IDENTITY,
            Vec3::new(s * 1.35, s * 0.1, s * depth),
        );
    }

    // Door pillars flanking the arch.
    for (side, label) in [(1.0_f32, "PillarE"), (-1.0_f32, "PillarW")] {
        part(
            label,
            stone.clone(),
            Vec3::new(side * 0.72 * s, 0.55 * s, 1.05 * s),
            Quat::IDENTITY,
            Vec3::new(s * 0.22, s * 1.15, s * 0.22),
        );
        part(
            &format!("{label}Cap"),
            stone_dark.clone(),
            Vec3::new(side * 0.72 * s, 1.15 * s, 1.05 * s),
            Quat::IDENTITY,
            Vec3::new(s * 0.3, s * 0.12, s * 0.3),
        );
    }

    // Front corner urns.
    for (side, label) in [(1.0_f32, "UrnE"), (-1.0_f32, "UrnW")] {
        part(
            label,
            stone_dark.clone(),
            Vec3::new(side * 1.05 * s, 0.22 * s, 1.25 * s),
            Quat::IDENTITY,
            Vec3::new(s * 0.22, s * 0.4, s * 0.22),
        );
        part(
            &format!("{label}Bowl"),
            stone.clone(),
            Vec3::new(side * 1.05 * s, 0.42 * s, 1.25 * s),
            Quat::IDENTITY,
            Vec3::new(s * 0.28, s * 0.1, s * 0.28),
        );
    }

    // Roof slopes with overhang, ridge, eaves, and a south pediment.
    let top_y = 0.98 * s;
    let tilt = 0.5_f32;
    for (side, label) in [(1.0_f32, "RoofE"), (-1.0_f32, "RoofW")] {
        part(
            label,
            slate.clone(),
            Vec3::new(side * 0.62 * s, top_y + 0.34 * s, 0.0),
            Quat::from_rotation_z(-side * tilt),
            Vec3::new(s * 1.45, s * 0.11, s * 2.85),
        );
    }
    part(
        "Ridge",
        timber.clone(),
        Vec3::new(0.0, top_y + 0.62 * s, 0.0),
        Quat::IDENTITY,
        Vec3::new(s * 0.16, s * 0.12, s * 2.7),
    );
    for (z, label) in [(1.35_f32, "EaveS"), (-1.35_f32, "EaveN")] {
        part(
            label,
            timber.clone(),
            Vec3::new(0.0, top_y + 0.12 * s, z * s),
            Quat::IDENTITY,
            Vec3::new(s * 2.15, s * 0.1, s * 0.12),
        );
    }
    // Pediment board facing the path.
    part(
        "Pediment",
        stone_dark,
        Vec3::new(0.0, top_y + 0.28 * s, 1.22 * s),
        Quat::IDENTITY,
        Vec3::new(s * 1.9, s * 0.55, s * 0.1),
    );
    part(
        "PedimentPeak",
        slate,
        Vec3::new(0.0, top_y + 0.55 * s, 1.28 * s),
        Quat::IDENTITY,
        Vec3::new(s * 0.55, s * 0.35, s * 0.1),
    );
    // Lintel over the arch.
    part(
        "Lintel",
        timber,
        Vec3::new(0.0, 1.05 * s, 1.12 * s),
        Quat::IDENTITY,
        Vec3::new(s * 1.5, s * 0.14, s * 0.18),
    );
}
