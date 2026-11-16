use bevy::prelude::*;
use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

use crate::player::Player;
use crate::rpg::LevelUpFx;
use crate::scene::Warden;

/// Fired when the monologue ends — start ash dissolve.
#[derive(Message, Clone, Copy, Debug, Default)]
pub struct DeathDissolveRequest;

#[derive(Component)]
pub struct AshParticle {
    pub velocity: Vec3,
    pub life: f32,
}

#[derive(Component)]
pub struct Dissolving {
    pub timer: Timer,
}

/// The book left behind when the Warden turns to ash.
#[derive(Component)]
pub struct DroppedBook;

#[derive(Component)]
pub struct BookFall {
    pub velocity: Vec3,
    pub ground_y: f32,
}

pub fn start_dissolve(
    mut commands: Commands,
    mut events: MessageReader<DeathDissolveRequest>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    npc: Query<(Entity, &Transform), With<Warden>>,
) {
    for _ in events.read() {
        let Ok((entity, tf)) = npc.single() else {
            continue;
        };

        let origin = tf.translation;
        let ash_mat = materials.add(StandardMaterial {
            base_color: Color::srgb(1.0, 0.45, 0.12),
            emissive: LinearRgba::rgb(1.2, 0.35, 0.05),
            unlit: true,
            ..default()
        });
        let speck = meshes.add(Sphere::new(0.06));

        let mut rng = SmallRng::seed_from_u64(0x0BA5_u64);
        for _ in 0..140 {
            let dir = Vec3::new(
                rng.random_range(-1.0..1.0),
                rng.random_range(0.2..1.0),
                rng.random_range(-1.0..1.0),
            )
            .normalize_or_zero();
            let speed = rng.random_range(1.5..4.5);
            let life = rng.random_range(0.6..1.4);
            let offset = Vec3::new(
                rng.random_range(-0.25..0.25),
                rng.random_range(0.0..1.2),
                rng.random_range(-0.25..0.25),
            );

            commands.spawn((
                AshParticle {
                    velocity: dir * speed + Vec3::Y * 1.2,
                    life,
                },
                Mesh3d(speck.clone()),
                MeshMaterial3d(ash_mat.clone()),
                Transform::from_translation(origin + offset).with_scale(Vec3::splat(
                    rng.random_range(0.4..1.2),
                )),
            ));
        }

        spawn_dropped_book(&mut commands, &mut meshes, &mut materials, origin);

        // Hide / despawn NPC after a short beat so ash reads first
        commands.entity(entity).insert(Dissolving {
            timer: Timer::from_seconds(0.15, TimerMode::Once),
        });
    }
}

fn spawn_dropped_book(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    npc_origin: Vec3,
) {
    let cover = materials.add(StandardMaterial {
        base_color: Color::srgb(0.28, 0.1, 0.08),
        perceptual_roughness: 0.85,
        ..default()
    });
    let pages = materials.add(StandardMaterial {
        base_color: Color::srgb(0.86, 0.8, 0.68),
        perceptual_roughness: 0.95,
        ..default()
    });
    let spine = materials.add(StandardMaterial {
        base_color: Color::srgb(0.18, 0.06, 0.05),
        perceptual_roughness: 0.8,
        ..default()
    });

    let cover_mesh = meshes.add(Cuboid::new(0.28, 0.04, 0.38));
    let page_mesh = meshes.add(Cuboid::new(0.24, 0.05, 0.34));
    let spine_mesh = meshes.add(Cuboid::new(0.04, 0.09, 0.38));

    // Fall from chest height toward the ground beside his feet.
    let drop_pos = npc_origin + Vec3::new(0.35, 1.05, 0.1);
    let yaw = -0.55_f32;

    commands
        .spawn((
            DroppedBook,
            BookFall {
                velocity: Vec3::new(0.4, 0.2, -0.15),
                ground_y: 0.055,
            },
            Name::new("DroppedBook"),
            Transform::from_translation(drop_pos)
                .with_rotation(Quat::from_euler(EulerRot::YXZ, yaw, 0.12, 0.08)),
            Visibility::default(),
        ))
        .with_children(|book| {
            book.spawn((
                Name::new("Cover"),
                Mesh3d(cover_mesh),
                MeshMaterial3d(cover),
                Transform::from_xyz(0.0, 0.0, 0.0),
            ));
            book.spawn((
                Name::new("Pages"),
                Mesh3d(page_mesh),
                MeshMaterial3d(pages),
                Transform::from_xyz(0.01, 0.0, 0.0),
            ));
            book.spawn((
                Name::new("Spine"),
                Mesh3d(spine_mesh),
                MeshMaterial3d(spine),
                Transform::from_xyz(-0.12, 0.0, 0.0),
            ));
        });
}

pub fn tick_dissolve(
    time: Res<Time>,
    mut commands: Commands,
    mut dissolving: Query<(Entity, &mut Dissolving, &mut Visibility)>,
) {
    for (entity, mut state, mut visibility) in &mut dissolving {
        state.timer.tick(time.delta());
        if state.timer.just_finished() {
            *visibility = Visibility::Hidden;
            commands.entity(entity).despawn();
        }
    }
}

pub fn tick_book_fall(
    time: Res<Time>,
    mut commands: Commands,
    mut books: Query<(Entity, &mut BookFall, &mut Transform), With<DroppedBook>>,
) {
    let dt = time.delta_secs();
    for (entity, mut fall, mut tf) in &mut books {
        fall.velocity.y -= 9.5 * dt;
        tf.translation += fall.velocity * dt;
        // Soft tumble while falling
        tf.rotate_local_x(fall.velocity.y.abs().min(4.0) * 0.15 * dt);
        tf.rotate_local_z(0.35 * dt);

        if tf.translation.y <= fall.ground_y {
            tf.translation.y = fall.ground_y;
            // Settle flat-ish on the dirt
            let (_, yaw, _) = tf.rotation.to_euler(EulerRot::YXZ);
            tf.rotation = Quat::from_euler(EulerRot::YXZ, yaw, 0.0, 0.05);
            commands.entity(entity).remove::<BookFall>();
        }
    }
}

pub fn tick_ash(
    time: Res<Time>,
    mut commands: Commands,
    mut particles: Query<(Entity, &mut AshParticle, &mut Transform)>,
) {
    let dt = time.delta_secs();
    for (entity, mut ash, mut tf) in &mut particles {
        ash.life -= dt;
        ash.velocity += Vec3::new(0.0, 1.8, 0.0) * dt;
        ash.velocity *= 1.0 - 1.2 * dt;
        tf.translation += ash.velocity * dt;
        let s = (ash.life * 1.2).clamp(0.05, 1.2);
        tf.scale = Vec3::splat(s * 0.08 / 0.06);
        if ash.life <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

#[derive(Component)]
pub struct LevelUpParticle {
    pub velocity: Vec3,
    pub life: f32,
    pub spin: f32,
}

#[derive(Component)]
pub struct LevelUpRing {
    pub birth: f32,
    pub phase: f32,
}

/// ba-stick `level_up` overlay: rising rings + sparkles for ~2.4s around the player.
pub fn spawn_level_up_vfx(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    added: Query<&Transform, (With<Player>, Added<LevelUpFx>)>,
) {
    let Ok(tf) = added.single() else {
        return;
    };
    let origin = tf.translation - Vec3::Y * 1.55;

    let glow = materials.add(StandardMaterial {
        base_color: Color::srgb(1.0, 0.92, 0.45),
        emissive: LinearRgba::rgb(2.4, 1.8, 0.35),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let cool = materials.add(StandardMaterial {
        base_color: Color::srgb(0.55, 0.85, 1.0),
        emissive: LinearRgba::rgb(0.4, 1.2, 2.0),
        unlit: true,
        alpha_mode: AlphaMode::Blend,
        ..default()
    });
    let ring_mesh = meshes.add(Torus::new(0.55, 0.04));
    let speck = meshes.add(Sphere::new(0.05));

    for k in 0..3 {
        commands.spawn((
            LevelUpRing {
                birth: 0.0,
                phase: k as f32 / 3.0,
            },
            Mesh3d(ring_mesh.clone()),
            MeshMaterial3d(glow.clone()),
            Transform::from_translation(origin + Vec3::Y * (0.2 + k as f32 * 0.15))
                .with_rotation(Quat::from_rotation_x(std::f32::consts::FRAC_PI_2)),
            Name::new("LevelUpRing"),
        ));
    }

    let mut rng = SmallRng::seed_from_u64(0x1EEF_u64);
    for i in 0..28u32 {
        let a = rng.random_range(0.0..std::f32::consts::TAU);
        let rr = rng.random_range(0.2..0.75);
        let mat = if i % 3 == 0 {
            cool.clone()
        } else {
            glow.clone()
        };
        commands.spawn((
            LevelUpParticle {
                velocity: Vec3::new(a.cos() * 0.35, rng.random_range(1.6..3.4), a.sin() * 0.35),
                life: rng.random_range(0.9..2.2),
                spin: rng.random_range(-4.0..4.0),
            },
            Mesh3d(speck.clone()),
            MeshMaterial3d(mat),
            Transform::from_translation(origin + Vec3::new(a.cos() * rr, 0.1, a.sin() * rr)),
            Name::new("LevelUpSpark"),
        ));
    }
}

pub fn tick_level_up_fx(
    time: Res<Time>,
    mut commands: Commands,
    mut player_fx: Query<(Entity, &mut LevelUpFx), With<Player>>,
    mut rings: Query<(Entity, &mut LevelUpRing, &mut Transform)>,
    mut sparks: Query<(Entity, &mut LevelUpParticle, &mut Transform), Without<LevelUpRing>>,
    player: Query<&Transform, (With<Player>, Without<LevelUpRing>, Without<LevelUpParticle>)>,
) {
    let dt = time.delta_secs();
    let player_pos = player
        .single()
        .ok()
        .map(|t| t.translation - Vec3::Y * 1.55);

    for (entity, mut fx) in &mut player_fx {
        fx.timer.tick(time.delta());
        if fx.timer.is_finished() {
            commands.entity(entity).remove::<LevelUpFx>();
        }
    }

    for (entity, mut ring, mut tf) in &mut rings {
        ring.birth += dt;
        let prog = (ring.birth / 2.4 + ring.phase) % 1.0;
        let h = prog * 2.4;
        let r = (0.7 - h * 0.18).max(0.15);
        if let Some(origin) = player_pos {
            tf.translation = origin + Vec3::Y * h;
        }
        tf.scale = Vec3::new(r / 0.55, r / 0.55, 1.0);
        tf.rotate_local_z(dt * 1.2);
        if ring.birth > 2.4 {
            commands.entity(entity).despawn();
        }
    }

    for (entity, mut spark, mut tf) in &mut sparks {
        spark.life -= dt;
        spark.velocity.y += 0.6 * dt;
        tf.translation += spark.velocity * dt;
        tf.rotate_local_y(spark.spin * dt);
        let s = spark.life.clamp(0.05, 1.0) * 0.9;
        tf.scale = Vec3::splat(s);
        if spark.life <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}
