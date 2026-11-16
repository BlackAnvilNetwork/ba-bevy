//! Standalone pixel-world demonstrator.
//!
//! Proves one scene can hold an SDF object (rocks + sword), procedural trees,
//! dynamic sunlight, and a destruction field — presented as a nearest-neighbor
//! pixel image. Run: `cargo run --bin pixel_clearing`

use ba_pixel_core::{palette, PixelMaterial};
use ba_pixel_geometry::{sd_rock, tree_mesh, voxel_shell, CpuMesh, V3};
use ba_pixel_sim::SwordDissolve;
use bevy::{
    asset::RenderAssetUsages,
    camera::RenderTarget,
    image::{Image, ImageSampler},
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
    render::render_resource::TextureFormat,
};

const PIXEL_W: u32 = 480;
const PIXEL_H: u32 = 270;

#[derive(Resource)]
struct Dissolve(SwordDissolve);

#[derive(Component)]
struct WorldCam;

#[derive(Component)]
struct PixelPresent;

#[derive(Component)]
struct SwordBody;

#[derive(Component)]
struct SwordAsh;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                title: "BA Pixel Clearing".into(),
                resolution: (1280, 720).into(),
                ..default()
            }),
            ..default()
        }))
        .insert_resource(ClearColor(Color::srgb(0.55, 0.62, 0.72)))
        .add_systems(Startup, setup)
        .add_systems(
            Update,
            (
                look_and_move,
                trigger_dissolve,
                tick_dissolve,
                sync_sword,
                fit_present,
            ),
        )
        .run();
}

fn setup(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let mut target = Image::new_target_texture(PIXEL_W, PIXEL_H, TextureFormat::Rgba8UnormSrgb, None);
    target.sampler = ImageSampler::nearest();
    let target = images.add(target);

    commands.spawn((
        WorldCam,
        Camera3d::default(),
        Camera {
            order: 0,
            clear_color: ClearColorConfig::Custom(Color::srgb(0.62, 0.72, 0.82)),
            ..default()
        },
        RenderTarget::Image(target.clone().into()),
        Msaa::Off,
        Transform::from_xyz(4.2, 2.1, 6.4).looking_at(Vec3::new(0.0, 1.0, 0.0), Vec3::Y),
    ));

    commands.spawn((
        Camera2d,
        Camera { order: 1, ..default() },
        Msaa::Off,
    ));
    commands.spawn((
        PixelPresent,
        Sprite {
            image: target,
            custom_size: Some(Vec2::new(1280.0, 720.0)),
            ..default()
        },
    ));

    commands.spawn((
        DirectionalLight {
            illuminance: 28_000.0,
            shadow_maps_enabled: true,
            ..default()
        },
        Transform::from_rotation(Quat::from_euler(EulerRot::XYZ, -0.85, 0.55, 0.0)),
    ));
    commands.insert_resource(GlobalAmbientLight {
        color: Color::srgb(0.65, 0.72, 0.85),
        brightness: 280.0,
        ..default()
    });

    let ground = palette::quantize([0.2, 0.24, 0.12], &palette::FOREST);
    commands.spawn((
        Mesh3d(meshes.add(Plane3d::new(Vec3::Y, Vec2::splat(18.0)))),
        MeshMaterial3d(materials.add(StandardMaterial {
            base_color: Color::srgb(ground[0], ground[1], ground[2]),
            perceptual_roughness: 0.95,
            ..default()
        })),
    ));

    spawn_rocks(&mut commands, &mut meshes, &mut materials);
    spawn_trees(&mut commands, &mut meshes, &mut materials);

    let sword = SwordDissolve::intact();
    let body = meshes.add(cpu_mesh(&sword.mesh));
    let ash = meshes.add(cpu_mesh(&sword.particle_mesh()));
    let iron = materials.add(mat_of(PixelMaterial::IRON));
    let ember = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.45, 0.12),
        emissive: LinearRgba::rgb(1.6, 0.4, 0.05),
        unlit: true,
        ..default()
    });
    commands.spawn((
        SwordBody,
        Mesh3d(body),
        MeshMaterial3d(iron),
        Transform::from_xyz(0.2, 0.0, 0.4),
    ));
    commands.spawn((
        SwordAsh,
        Mesh3d(ash),
        MeshMaterial3d(ember),
        Transform::from_xyz(0.2, 0.0, 0.4),
    ));
    commands.insert_resource(Dissolve(sword));

    commands.spawn((
        Text::new("WASD look-move · F disintegrate sword"),
        TextFont::from_font_size(18.0),
        TextColor(Color::srgb(0.95, 0.9, 0.75)),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(16),
            left: px(16),
            ..default()
        },
    ));
}

fn spawn_rocks(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let stone = materials.add(mat_of(PixelMaterial::STONE));
    for (origin, radius, seed, res) in [
        (V3::new(-2.4, 0.15, -1.2), 0.85, 1.7, [18, 16, 18]),
        (V3::new(2.1, 0.05, -2.4), 0.55, 4.2, [14, 12, 14]),
    ] {
        let extent = V3::new(radius * 2.4, radius * 2.2, radius * 2.4);
        let local = V3::new(-extent.x * 0.5, -radius * 0.15, -extent.z * 0.5);
        let shell = voxel_shell(
            |p| sd_rock(p, radius, seed),
            local,
            extent,
            res,
        );
        commands.spawn((
            Mesh3d(meshes.add(cpu_mesh(&shell))),
            MeshMaterial3d(stone.clone()),
            Transform::from_xyz(origin.x, origin.y, origin.z),
        ));
    }
}

fn spawn_trees(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let bark = materials.add(mat_of(PixelMaterial::BARK));
    let leaf = materials.add(mat_of(PixelMaterial::LEAF));
    for (i, pos) in [(-3.2, -3.5), (3.4, -1.2), (-1.5, 3.6)].into_iter().enumerate() {
        let (wood, leaves) = tree_mesh(i as u32, 20 + i as u32 * 11);
        commands.spawn((
            Mesh3d(meshes.add(cpu_mesh(&wood))),
            MeshMaterial3d(bark.clone()),
            Transform::from_xyz(pos.0, 0.0, pos.1),
        ));
        commands.spawn((
            Mesh3d(meshes.add(cpu_mesh(&leaves))),
            MeshMaterial3d(leaf.clone()),
            Transform::from_xyz(pos.0, 0.0, pos.1),
        ));
    }
}

fn mat_of(m: PixelMaterial) -> StandardMaterial {
    let c = palette::quantize(m.albedo, &palette::FOREST);
    StandardMaterial {
        base_color: Color::srgb(c[0], c[1], c[2]),
        emissive: LinearRgba::rgb(m.emission[0], m.emission[1], m.emission[2]),
        perceptual_roughness: m.roughness,
        ..default()
    }
}

fn cpu_mesh(cpu: &CpuMesh) -> Mesh {
    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList, RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD);
    if cpu.positions.is_empty() {
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0, 0.0, 0.0]; 3]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 3]);
        mesh.insert_indices(Indices::U32(vec![0, 1, 2]));
        return mesh;
    }
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, cpu.positions.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, cpu.normals.clone());
    mesh.insert_indices(Indices::U32(cpu.indices.clone()));
    mesh
}

fn trigger_dissolve(keys: Res<ButtonInput<KeyCode>>, mut sword: ResMut<Dissolve>) {
    if keys.just_pressed(KeyCode::KeyF) {
        sword.0.trigger();
    }
}

fn tick_dissolve(time: Res<Time>, mut sword: ResMut<Dissolve>) {
    sword.0.tick(time.delta_secs());
}

fn sync_sword(
    sword: Res<Dissolve>,
    mut meshes: ResMut<Assets<Mesh>>,
    body: Query<&Mesh3d, With<SwordBody>>,
    ash: Query<&Mesh3d, With<SwordAsh>>,
) {
    if !sword.is_changed() {
        return;
    }
    if let Ok(handle) = body.single() {
        if let Some(mut mesh) = meshes.get_mut(&handle.0) {
            *mesh = cpu_mesh(&sword.0.mesh);
        }
    }
    if let Ok(handle) = ash.single() {
        if let Some(mut mesh) = meshes.get_mut(&handle.0) {
            *mesh = cpu_mesh(&sword.0.particle_mesh());
        }
    }
}

fn fit_present(windows: Query<&Window>, mut sprites: Query<&mut Sprite, With<PixelPresent>>) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Ok(mut sprite) = sprites.single_mut() else {
        return;
    };
    sprite.custom_size = Some(Vec2::new(window.width(), window.height()));
}

fn look_and_move(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<bevy::input::mouse::AccumulatedMouseMotion>,
    mut cam: Query<&mut Transform, With<WorldCam>>,
) {
    let Ok(mut tf) = cam.single_mut() else {
        return;
    };
    let d = mouse.delta;
    if d != Vec2::ZERO {
        let (yaw, pitch, _) = tf.rotation.to_euler(EulerRot::YXZ);
        let yaw = yaw - d.x * 0.003;
        let pitch = (pitch - d.y * 0.002).clamp(-1.2, 1.2);
        tf.rotation = Quat::from_euler(EulerRot::YXZ, yaw, pitch, 0.0);
    }
    let mut dir = Vec3::ZERO;
    if keys.pressed(KeyCode::KeyW) {
        dir += *tf.forward();
    }
    if keys.pressed(KeyCode::KeyS) {
        dir -= *tf.forward();
    }
    if keys.pressed(KeyCode::KeyA) {
        dir -= *tf.right();
    }
    if keys.pressed(KeyCode::KeyD) {
        dir += *tf.right();
    }
    if dir != Vec3::ZERO {
        tf.translation += dir.normalize() * 4.5 * time.delta_secs();
    }
}
