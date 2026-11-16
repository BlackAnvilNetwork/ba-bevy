//! Stone well: a ring of blocks, a timber roof, and a crank that winds the bucket.

use std::f32::consts::TAU;

use bevy::ecs::hierarchy::ChildOf;
use bevy::mesh::CylinderMeshBuilder;
use bevy::prelude::*;

#[derive(Component)]
pub(crate) struct WellCrank;

#[derive(Component)]
pub(crate) struct WellWindlass;

#[derive(Component)]
pub(crate) struct WellRope;

#[derive(Component)]
pub(crate) struct WellBucket;

const AXLE_Y: f32 = 1.62;
const BUCKET_LOW: f32 = 0.78;
const BUCKET_HIGH: f32 = 1.34;

/// When `t > 0`, the bucket tips and dumps its water (Edda's light-hands beat).
#[derive(Resource, Default)]
pub struct WellPour {
    pub t: f32,
}

pub fn start_pour(pour: &mut WellPour) {
    pour.t = 2.2;
}

pub fn spawn_well(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    transform: Transform,
) {
    let stone = materials.add(StandardMaterial {
        base_color: Color::srgb(0.42, 0.4, 0.37),
        perceptual_roughness: 1.0,
        ..default()
    });
    let stone_dark = materials.add(StandardMaterial {
        base_color: Color::srgb(0.26, 0.25, 0.23),
        perceptual_roughness: 1.0,
        ..default()
    });
    let water = materials.add(StandardMaterial {
        base_color: Color::srgb(0.05, 0.11, 0.13),
        perceptual_roughness: 0.35,
        ..default()
    });
    let timber = materials.add(StandardMaterial {
        base_color: Color::srgb(0.34, 0.22, 0.13),
        perceptual_roughness: 1.0,
        ..default()
    });
    let slate = materials.add(StandardMaterial {
        base_color: Color::srgb(0.22, 0.24, 0.27),
        perceptual_roughness: 1.0,
        ..default()
    });
    let iron = materials.add(StandardMaterial {
        base_color: Color::srgb(0.16, 0.16, 0.18),
        perceptual_roughness: 0.55,
        metallic: 0.4,
        ..default()
    });
    let rope_mat = materials.add(StandardMaterial {
        base_color: Color::srgb(0.62, 0.52, 0.34),
        perceptual_roughness: 1.0,
        ..default()
    });

    let block = meshes.add(Cuboid::new(1.0, 1.0, 1.0));
    let rope_mesh = meshes.add(CylinderMeshBuilder::new(0.018, 1.0, 6));
    let windlass_mesh = meshes.add(CylinderMeshBuilder::new(0.07, 1.55, 8));
    let bucket_mesh = meshes.add(CylinderMeshBuilder::new(0.11, 0.22, 8));
    let grip_mesh = meshes.add(CylinderMeshBuilder::new(0.028, 0.16, 6));

    let root = commands
        .spawn((
            Name::new("Well"),
            transform,
            Visibility::default(),
        ))
        .id();

    let mut stone_at = |i: u32, radius: f32, y: f32, size: Vec3, dark: bool| {
        let n = unit(i);
        let angle = (i as f32 / 16.0) * TAU + (n - 0.5) * 0.08;
        let r = radius + (n - 0.5) * 0.04;
        let material = if dark { stone_dark.clone() } else { stone.clone() };
        commands.spawn((
            Name::new(format!("WellStone {i}")),
            Mesh3d(block.clone()),
            MeshMaterial3d(material),
            Transform::from_xyz(angle.cos() * r, y + (n - 0.5) * 0.04, angle.sin() * r)
                .with_rotation(Quat::from_rotation_y(angle))
                .with_scale(size * (0.92 + n * 0.16)),
            ChildOf(root),
        ));
    };
    for i in 0..16 {
        stone_at(i, 0.86, 0.26, Vec3::new(0.34, 0.5, 0.24), i % 3 == 0);
        stone_at(
            i.wrapping_add(7),
            0.78,
            0.58,
            Vec3::new(0.3, 0.28, 0.22),
            i % 2 == 0,
        );
    }

    commands.spawn((
        Name::new("WellWater"),
        Mesh3d(meshes.add(CylinderMeshBuilder::new(0.52, 0.08, 12))),
        MeshMaterial3d(water),
        Transform::from_xyz(0.0, 0.32, 0.0),
        ChildOf(root),
    ));

    for (x, name) in [(1.02, "WellPostEast"), (-1.02, "WellPostWest")] {
        commands.spawn((
            Name::new(name),
            Mesh3d(block.clone()),
            MeshMaterial3d(timber.clone()),
            Transform::from_xyz(x, 1.05, 0.0).with_scale(Vec3::new(0.12, 2.1, 0.12)),
            ChildOf(root),
        ));
    }
    commands.spawn((
        Name::new("WellBeam"),
        Mesh3d(block.clone()),
        MeshMaterial3d(timber.clone()),
        Transform::from_xyz(0.0, 2.02, 0.0).with_scale(Vec3::new(2.25, 0.1, 0.12)),
        ChildOf(root),
    ));
    for (x, tilt, name) in [
        (0.48, -0.55, "WellRoofEast"),
        (-0.48, 0.55, "WellRoofWest"),
    ] {
        commands.spawn((
            Name::new(name),
            Mesh3d(block.clone()),
            MeshMaterial3d(slate.clone()),
            Transform::from_xyz(x, 2.28, 0.0)
                .with_rotation(Quat::from_rotation_z(tilt))
                .with_scale(Vec3::new(1.15, 0.06, 1.35)),
            ChildOf(root),
        ));
    }

    commands.spawn((
        WellWindlass,
        Name::new("WellWindlass"),
        Mesh3d(windlass_mesh),
        MeshMaterial3d(timber.clone()),
        Transform::from_xyz(0.0, AXLE_Y, 0.0).with_rotation(Quat::from_rotation_z(std::f32::consts::FRAC_PI_2)),
        ChildOf(root),
    ));

    let crank = commands
        .spawn((
            WellCrank,
            Name::new("WellCrank"),
            Transform::from_xyz(0.86, AXLE_Y, 0.0),
            Visibility::default(),
            ChildOf(root),
        ))
        .id();
    commands.spawn((
        Name::new("WellCrankArm"),
        Mesh3d(block.clone()),
        MeshMaterial3d(iron.clone()),
        Transform::from_xyz(0.0, 0.16, 0.0).with_scale(Vec3::new(0.04, 0.32, 0.04)),
        ChildOf(crank),
    ));
    commands.spawn((
        Name::new("WellCrankGrip"),
        Mesh3d(grip_mesh),
        MeshMaterial3d(iron),
        Transform::from_xyz(0.0, 0.32, 0.0).with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
        ChildOf(crank),
    ));

    commands.spawn((
        WellRope,
        Name::new("WellRope"),
        Mesh3d(rope_mesh),
        MeshMaterial3d(rope_mat),
        Transform::from_xyz(0.0, 1.15, 0.0),
        ChildOf(root),
    ));
    commands.spawn((
        WellBucket,
        Name::new("WellBucket"),
        Mesh3d(bucket_mesh),
        MeshMaterial3d(timber),
        Transform::from_xyz(0.0, BUCKET_LOW, 0.0),
        ChildOf(root),
    ));
}

pub fn tick_well(
    time: Res<Time>,
    mut pour: ResMut<WellPour>,
    mut crank: Query<&mut Transform, (With<WellCrank>, Without<WellWindlass>, Without<WellRope>, Without<WellBucket>)>,
    mut windlass: Query<&mut Transform, (With<WellWindlass>, Without<WellCrank>, Without<WellRope>, Without<WellBucket>)>,
    mut rope: Query<&mut Transform, (With<WellRope>, Without<WellCrank>, Without<WellWindlass>, Without<WellBucket>)>,
    mut bucket: Query<&mut Transform, (With<WellBucket>, Without<WellCrank>, Without<WellWindlass>, Without<WellRope>)>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut splashed: Local<bool>,
) {
    if pour.t > 0.0 {
        pour.t = (pour.t - time.delta_secs()).max(0.0);
        let u = 1.0 - (pour.t / 2.2);
        let bucket_y = BUCKET_HIGH - u * 0.15;
        let tip = (u * 1.4).min(1.2);
        let rope_len = (AXLE_Y - 0.08 - (bucket_y + 0.12)).max(0.08);
        for mut transform in &mut crank {
            transform.rotation = Quat::from_rotation_x(tip);
        }
        for mut transform in &mut windlass {
            transform.rotation =
                Quat::from_rotation_x(tip) * Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
        }
        for mut transform in &mut rope {
            transform.translation.y = bucket_y + 0.12 + rope_len * 0.5;
            transform.scale.y = rope_len;
        }
        for mut transform in &mut bucket {
            transform.translation.y = bucket_y;
            transform.rotation = Quat::from_rotation_z(tip);
        }
        if !*splashed && u > 0.25 {
            *splashed = true;
            let drop = materials.add(StandardMaterial {
                base_color: Color::srgb(0.35, 0.55, 0.7),
                unlit: true,
                ..default()
            });
            let speck = meshes.add(Sphere::new(0.04));
            for i in 0..18 {
                let a = i as f32 * 0.35;
                commands.spawn((
                    Name::new("WellSplash"),
                    Mesh3d(speck.clone()),
                    MeshMaterial3d(drop.clone()),
                    Transform::from_xyz(a.cos() * 0.15, bucket_y - 0.1, a.sin() * 0.15),
                    WellSplash {
                        velocity: Vec3::new(a.cos() * 0.6, -0.4, a.sin() * 0.6),
                        life: 0.7,
                    },
                ));
            }
        }
        if pour.t <= 0.0 {
            *splashed = false;
        }
        return;
    }

    let lift = (time.elapsed_secs() * 0.45).sin() * 0.5 + 0.5;
    let spin = lift * TAU * 2.0;
    let bucket_y = BUCKET_LOW + (BUCKET_HIGH - BUCKET_LOW) * lift;
    let rope_len = (AXLE_Y - 0.08 - (bucket_y + 0.12)).max(0.08);

    for mut transform in &mut crank {
        transform.rotation = Quat::from_rotation_x(spin);
    }
    for mut transform in &mut windlass {
        transform.rotation =
            Quat::from_rotation_x(spin) * Quat::from_rotation_z(std::f32::consts::FRAC_PI_2);
    }
    for mut transform in &mut rope {
        transform.translation.y = bucket_y + 0.12 + rope_len * 0.5;
        transform.scale.y = rope_len;
    }
    for mut transform in &mut bucket {
        transform.translation.y = bucket_y;
        transform.rotation = Quat::IDENTITY;
    }
}

#[derive(Component)]
pub(crate) struct WellSplash {
    velocity: Vec3,
    life: f32,
}

pub fn tick_well_splash(
    time: Res<Time>,
    mut commands: Commands,
    mut drops: Query<(Entity, &mut WellSplash, &mut Transform)>,
) {
    let dt = time.delta_secs();
    for (entity, mut splash, mut tf) in &mut drops {
        splash.life -= dt;
        splash.velocity.y -= 4.5 * dt;
        tf.translation += splash.velocity * dt;
        if splash.life <= 0.0 || tf.translation.y < 0.05 {
            commands.entity(entity).despawn();
        }
    }
}

fn unit(i: u32) -> f32 {
    let mut n = i.wrapping_mul(747796405).wrapping_add(2891336453);
    n = (n ^ (n >> 13)).wrapping_mul(1274126177);
    (n >> 8) as f32 / 16_777_216.0
}
