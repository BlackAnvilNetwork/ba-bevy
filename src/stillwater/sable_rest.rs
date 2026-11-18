//! Cael's working smithy and the fortified ferry courtyard.
use super::Silt;
use bevy::prelude::*;
use std::{
    collections::BTreeMap,
    f32::consts::{FRAC_PI_2, PI},
};

const V: fn(f32, f32, f32) -> Vec3 = Vec3::new;
type Surface = usize;

struct Yard {
    cube: Mesh,
    cobble: Mesh,
    materials: Vec<Handle<StandardMaterial>>,
    batches: BTreeMap<Surface, Mesh>,
    frame: Transform,
}

impl Yard {
    fn shape(&mut self, stone: bool, material: Surface, at: Vec3, size: Vec3, rotation: Quat) {
        let base = if stone { &self.cobble } else { &self.cube };
        let mesh = base.clone().transformed_by(
            self.frame.mul_transform(
                Transform::from_translation(at)
                    .with_scale(size)
                    .with_rotation(rotation),
            ),
        );
        match self.batches.entry(material) {
            std::collections::btree_map::Entry::Occupied(mut e) => {
                e.get_mut().merge(&mesh).unwrap()
            }
            std::collections::btree_map::Entry::Vacant(e) => {
                e.insert(mesh);
            }
        }
    }
    fn block(&mut self, m: Surface, at: Vec3, size: Vec3) {
        self.shape(false, m, at, size, Quat::IDENTITY);
    }
    fn beam(&mut self, m: Surface, a: Vec3, b: Vec3, w: f32, d: f32) {
        self.shape(
            false,
            m,
            (a + b) * 0.5,
            V(w, a.distance(b), d),
            Quat::from_rotation_arc(Vec3::Y, (b - a).normalize()),
        );
    }
}

fn noise(i: i32, j: i32) -> f32 {
    let mut h = (i as u32).wrapping_mul(0x9e3779b9) ^ (j as u32).wrapping_mul(0x85ebca6b);
    h ^= h >> 16;
    h = h.wrapping_mul(0x7feb352d);
    (h >> 8) as f32 / 16777216.
}

fn stonework(y: &mut Yard, x: f32, z: f32, width: f32, rows: usize, depth: f32, base: f32) {
    for row in 0..rows {
        let mut edge = -width * 0.5;
        for j in 0.. {
            let w = (0.68 + noise(row as i32, j) * 0.52).min(width * 0.5 - edge);
            if w <= 0.01 {
                break;
            }
            y.block(
                2 + (noise(j, row as i32) * 4.) as usize,
                V(x + edge + w * 0.5, base + row as f32 * 0.43 + 0.21, z),
                V(w - 0.012, 0.42, depth + 0.07 * noise(row as i32, j + 7)),
            );
            edge += w;
        }
    }
}

// A pegged oak truss under individually coursed overlapping slate.
fn hall(y: &mut Yard, x: f32, z: f32, w: f32, d: f32, closed: bool) {
    let eave = if closed { 4.3 } else { 3.28 };
    let rise = w * 0.39;
    let front = z + d * 0.5;
    if closed {
        stonework(y, x, front - 0.3, w, 3, 0.58, 0.);
        y.block(13, V(x, 2.82, front - 0.35), V(w - 0.14, 3., 0.45));
        y.block(13, V(x, 2.23, z), V(w - 0.22, 4.23, d - 0.58));
        for j in -2..=2 {
            let xx = x + j as f32 * w / 5.;
            y.block(0, V(xx, 2.72, front), V(0.17, 3.25, 0.24));
            y.beam(
                1,
                V(xx, 3.0, front),
                V(xx + w / 6., 4.18, front),
                0.14,
                0.18,
            );
        }
        for h in [1.28, 2.9, 4.22] {
            y.block(0, V(x, h, front), V(w, 0.18, 0.23));
        }
    } else {
        stonework(y, x, z - d * 0.5, w, 6, 0.5, 0.);
    }
    for zz in [front, z, z - d * 0.5] {
        for side in [-1., 1.] {
            let xx = x + side * (w * 0.5 - 0.14);
            y.block(0, V(xx, eave * 0.5, zz), V(0.31, eave, 0.34));
            y.block(3, V(xx, 0.25, zz), V(0.54, 0.5, 0.55));
            y.beam(
                1,
                V(xx, eave - 0.92, zz),
                V(xx - side * 1.02, eave, zz),
                0.19,
                0.20,
            );
            y.beam(
                0,
                V(xx + side * 0.36, eave - 0.14, zz),
                V(x, eave + rise, zz),
                0.26,
                0.34,
            );
        }
        y.block(0, V(x, eave, zz), V(w, 0.23, 0.28));
        y.block(1, V(x, eave + rise * 0.5, zz), V(0.19, rise, 0.22));
    }
    let half = w * 0.5 + 0.4;
    for side in [-1., 1.] {
        let a = V(x, eave + rise + 0.15, z);
        let b = V(x + side * half, eave - 0.10, z);
        let delta = b - a;
        let rot = Quat::from_rotation_z(delta.y.atan2(half) * side);
        y.shape(
            false,
            7,
            (a + b) * 0.5,
            V(delta.length(), 0.14, d + 1.2),
            rot,
        );
        for row in 0..11 {
            for col in 0..17 {
                let u = (row as f32 + 0.5) / 11.;
                let offset = if row % 2 == 0 { 0. } else { 0.28 };
                let p = a
                    + delta * u
                    + V(
                        0.,
                        0.10 + noise(row, col) * 0.03,
                        (col as f32 - 8. + offset) * (d + 1.15) / 17.,
                    );
                y.shape(
                    false,
                    6 + (noise(row + 44, col + 13) * 4.) as usize,
                    p,
                    V(delta.length() / 11. * 1.05, 0.10, (d + 1.15) / 17. * 0.98),
                    rot,
                );
            }
        }
    }
    y.block(7, V(x, eave + rise + 0.23, z), V(0.3, 0.25, d + 1.32));
}

fn lamp(y: &mut Yard, c: &mut Commands, p: Vec3, blue: bool) {
    y.block(if blue { 15 } else { 10 }, p, V(0.26, 0.40, 0.26));
    for h in [-0.24, 0.24] {
        y.block(12, p + Vec3::Y * h, V(0.42, 0.085, 0.42));
    }
    for x in [-0.14, 0.14] {
        for z in [-0.14, 0.14] {
            y.block(12, p + V(x, 0., z), V(0.028, 0.47, 0.028));
        }
    }
    y.block(12, p + Vec3::Y * 0.38, V(0.044, 0.26, 0.044));
    c.spawn((
        Silt,
        PointLight {
            color: if blue {
                Color::srgb(0.22, 0.63, 1.)
            } else {
                Color::srgb(1., 0.57, 0.22)
            },
            intensity: if blue { 23000. } else { 52000. },
            range: 9.,
            ..default()
        },
        Transform::from_translation(p + V(600., 0., 0.3)),
    ));
}

fn kit(c: &mut Commands, a: &AssetServer, pack: &str, name: &str, p: Vec3, scale: f32, yaw: f32) {
    let fmt = if pack == "nature-kit" {
        "GLTF format"
    } else {
        "GLB format"
    };
    c.spawn((
        Silt,
        WorldAssetRoot(a.load(format!("kits/{pack}/Models/{fmt}/{name}.glb#Scene0"))),
        Transform::from_translation(p + V(600., 0., 0.))
            .with_scale(Vec3::splat(scale))
            .with_rotation(Quat::from_rotation_y(yaw)),
    ));
}

#[derive(Component)]
pub struct Cinder {
    origin: Vec3,
    phase: f32,
}

pub fn drift(time: Res<Time>, mut motes: Query<(&Cinder, &mut Transform)>) {
    for (c, mut t) in &mut motes {
        let life = (time.elapsed_secs() * 0.38 + c.phase).fract();
        t.translation = c.origin
            + V(
                life * 0.47 + (life * 7. + c.phase * 12.).sin() * 0.14,
                life * 2.6,
                life * 0.17,
            );
        t.scale = Vec3::splat(0.025 * (1. - life) + 0.008);
    }
}

pub fn build(
    mut c: Commands,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    let mut materials = Vec::new();
    let mut surface = |tex: &str, color: [f32; 3], glow: f32| {
        let img = (!tex.is_empty()).then(|| assets.load(format!("stillwater/{tex}.png")));
        let [r, g, b] = color;
        materials.push(mats.add(StandardMaterial {
            base_color: Color::srgb(r, g, b),
            base_color_texture: img.clone(),
            emissive_texture: img,
            emissive: LinearRgba::rgb(r * glow, g * glow, b * glow),
            perceptual_roughness: 0.83,
            ..default()
        }));
    };
    surface("oak", [0.88, 0.66, 0.49], 0.23);
    surface("oak", [1., 0.82, 0.60], 0.30);
    for k in [0.60, 0.79, 0.67, 0.88] {
        surface("limestone", [k * 0.94, k * 0.91, k], 0.34);
    }
    for k in [0.34, 0.43, 0.53, 0.39] {
        surface("limestone", [k * 0.86, k * 0.91, k * 1.32], 0.27);
    }
    surface("", [1., 0.47, 0.085], 3.0);
    surface("", [1., 0.19, 0.018], 2.5);
    surface("", [0.13, 0.13, 0.16], 0.07);
    surface("limestone", [1., 0.81, 0.66], 0.24);
    surface("", [0.44, 0.10, 0.13], 0.11);
    surface("", [0.08, 0.55, 1.], 3.);
    surface("earth", [0.63, 0.53, 0.39], 0.24);
    surface("", [1., 0.62, 0.13], 5.);
    surface("limestone", [0.90, 0.22, 0.035], 1.6);
    surface("", [0.25, 0.28, 0.095], 0.14);
    surface("", [0.35, 0.32, 0.12], 0.17);
    surface("", [0.74, 0.48, 0.22], 0.25);
    let mut y = Yard {
        cube: Cuboid::from_size(Vec3::ONE).into(),
        cobble: Cylinder::new(0.5, 1.).mesh().resolution(8).build(),
        materials,
        batches: BTreeMap::new(),
        frame: Transform::IDENTITY,
    };
    for row in -6..7 {
        for col in -6..7 {
            y.block(
                16,
                V(col as f32 * 3.2, 0.007, row as f32 * 3.2),
                V(3.2, 0.016, 3.2),
            );
        }
    }
    // Irregular flagstones taper into the grass along the courtyard margins.
    for row in -27..33 {
        for col in -18..19 {
            let z = row as f32 * 0.59;
            let x = (col as f32 + 0.49 * (row % 2) as f32) * 0.88;
            let full =
                x.abs() < 3.5 + (z * 0.35).cos() * 0.7 || (z < 5. && z > -11. && x.abs() < 8.);
            if !full && noise(row + 7, col + 13) > 0.21 {
                continue;
            }
            let n = noise(row, col);
            y.shape(
                true,
                2 + (n * 4.) as usize,
                V(x + 0.048 * n, 0.042, z + 0.035 * n),
                V(0.86 + 0.06 * n, 0.049 + n * 0.016, 0.59 + 0.035 * n),
                Quat::from_rotation_y((noise(col, row) - 0.5) * 0.16),
            );
        }
    }
    // The smithy sits obliquely on the old wall line, exposing its open side.
    let pivot = V(-5.7, 0., 2.95);
    let angle = Quat::from_rotation_y(-0.34);
    y.frame = Transform::from_translation(pivot - angle * pivot).with_rotation(angle);
    hall(&mut y, -5.7, -0.65, 7.4, 7.2, false);
    y.frame = Transform::IDENTITY;
    hall(&mut y, 8.9, -1.9, 6.8, 7.9, true);
    // Low curtain returns enclose the courtyard beyond the smithy and inn.
    for side in [-1.0, 1.0] {
        for bay in 0..6 {
            let z = -8.8 + bay as f32 * 3.15;
            for course in 0..8 {
                for j in 0..4 {
                    let offset = (course % 2) as f32 * 0.38;
                    y.block(
                        2 + (course + j) % 4,
                        V(
                            side * 16.5,
                            course as f32 * 0.40 + 0.2,
                            z + j as f32 * 0.79 + offset,
                        ),
                        V(0.75, 0.39, 0.78),
                    );
                }
            }
            y.block(3, V(side * 16.5, 1.72, z), V(1.16, 3.44, 0.72));
            y.block(4, V(side * 16.5, 3.49, z), V(1.23, 0.17, 0.85));
            y.block(2, V(side * 16.5, 3.43, z + 1.6), V(0.79, 0.50, 0.91));
        }
    }
    // An arched fire mouth, hood and brick flue behind the open colonnade.
    let fx = -5.45;
    let fz = -1.1;
    for side in [-1., 1.] {
        stonework(&mut y, fx + side * 1.05, fz, 0.58, 4, 1.35, 0.);
    }
    for j in 0..11 {
        let ang = (j as f32 + 0.5) * PI / 11.;
        y.shape(
            false,
            2 + j % 4,
            V(fx + 1.04 * ang.cos(), 1.46 + 1.04 * ang.sin(), fz + 0.06),
            V(0.48, 0.38, 1.39),
            Quat::from_rotation_z(ang),
        );
    }
    stonework(&mut y, fx, fz - 0.45, 1.40, 13, 0.97, 2.3);
    y.block(3, V(fx, 7.89, fz - 0.45), V(1.66, 0.25, 1.2));
    y.block(12, V(fx, 1.14, fz - 0.54), V(1.8, 2., 0.17));
    y.block(3, V(fx, 0.43, fz + 0.3), V(2.85, 0.8, 1.55));
    // Hot brick backing follows the vaulted opening behind the bed of coke.
    for row in 0..5 {
        let half_width = (1. - ((row as f32 - 1.) * 0.24).powi(2)).sqrt() * 0.86;
        for brick in 0..5 {
            y.block(
                18,
                V(
                    fx + (brick as f32 - 2.) * half_width * 0.4,
                    0.95 + row as f32 * 0.245,
                    fz - 0.12,
                ),
                V(half_width * 0.385, 0.232, 0.25),
            );
        }
    }
    for j in 0..9 {
        let n = noise(50, j);
        let x = fx + (j as f32 - 4.) * 0.16;
        y.beam(
            17,
            V(x, 0.94, fz + 0.41),
            V(x + 0.09, 1.29 + n * 0.40, fz + 0.36),
            0.09 + n * 0.06,
            0.13,
        );
    }
    for j in 0..12 {
        let n = noise(j, 33);
        y.shape(
            true,
            if j % 3 == 0 { 11 } else { 10 },
            V(fx + (j as f32 - 5.5) * 0.13, 0.89 + n * 0.16, fz + 0.40),
            V(0.25, 0.19 + n * 0.4, 0.33),
            Quat::from_rotation_z(n - 0.5),
        );
    }
    c.spawn((
        Silt,
        PointLight {
            color: Color::srgb(1., 0.38, 0.095),
            intensity: 340000.,
            range: 15.,
            ..default()
        },
        Transform::from_xyz(600. + fx, 1.5, fz + 1.25),
    ));
    // Warm light under the awning reaches the approach and catches the paving.
    for (x, h, z, lumens, tint) in [
        (-5.1, 2.8, 3.75, 360000., Color::srgb(1., 0.63, 0.30)),
        (8.1, 3.25, 3.85, 160000., Color::srgb(1., 0.73, 0.46)),
        (1.1, 6.8, 5.2, 260000., Color::srgb(0.74, 0.72, 1.)),
    ] {
        c.spawn((
            Silt,
            PointLight {
                color: tint,
                intensity: lumens,
                range: 16.,
                ..default()
            },
            Transform::from_xyz(600. + x, h, z),
        ));
    }
    let spark = meshes.add(Cuboid::from_size(Vec3::ONE));
    for j in 0..34 {
        let origin = V(
            600. + fx + noise(j, 78) * 1.5 - 0.75,
            0.99,
            fz + 0.58 + noise(28, j) * 0.4,
        );
        c.spawn((
            Silt,
            Cinder {
                origin,
                phase: noise(j, 37),
            },
            Mesh3d(spark.clone()),
            MeshMaterial3d(y.materials[10].clone()),
            Transform::from_translation(origin).with_scale(Vec3::splat(0.03)),
        ));
    }
    // Wrought iron anvil on its oak stump beside Cael.
    y.shape(
        true,
        0,
        V(-1.55, 0.36, 2.5),
        V(0.91, 0.70, 0.84),
        Quat::IDENTITY,
    );
    for (h, w, d) in [(0.76, 0.78, 0.49), (0.90, 0.47, 0.34), (1.05, 1.03, 0.46)] {
        y.block(12, V(-1.55, h, 2.5), V(w, 0.16, d));
    }
    for (xx, zz) in [(-8.9, 2.01), (-2.35, 4.24), (5.85, 2.29)] {
        lamp(&mut y, &mut c, V(xx, 2.71, zz), false);
    }
    // Inn shutters, leaded lights and studded entry.
    let front = 2.15;
    for xx in [6.95, 10.68] {
        y.block(0, V(xx, 3.20, front), V(1.27, 1.35, 0.21));
        y.block(10, V(xx, 3.20, front + 0.13), V(0.98, 1.04, 0.09));
        for s in [-1., 1.] {
            y.beam(
                0,
                V(xx - s * 0.45, 2.70, front + 0.22),
                V(xx + s * 0.45, 3.70, front + 0.22),
                0.07,
                0.065,
            );
        }
        y.block(0, V(xx, 2.60, front + 0.18), V(1.40, 0.14, 0.44));
    }
    for j in 0..6 {
        y.block(
            j % 2,
            V(8.62 + j as f32 * 0.18, 1.15, front + 0.05),
            V(0.176, 2.26, 0.22),
        );
    }
    for h in [0.65, 1.72] {
        y.block(12, V(9.07, h, front + 0.19), V(1.07, 0.09, 0.07));
    }
    for (xx, zz, hh) in [(-5.7, 3.24, 3.48), (11.4, 2.34, 3.91)] {
        y.block(14, V(xx, hh, zz), V(1.05, 1.45, 0.08));
        y.block(1, V(xx, hh + 0.76, zz), V(1.34, 0.095, 0.11));
        for side in [-1., 1.] {
            y.block(
                21,
                V(xx + side * 0.48, hh, zz + 0.047),
                V(0.025, 1.40, 0.018),
            );
            y.beam(
                21,
                V(xx + side * 0.48, hh - 0.67, zz + 0.05),
                V(xx, hh - 0.49, zz + 0.05),
                0.032,
                0.018,
            );
        }
        // A small heraldic anvil worked in brass.
        y.block(13, V(xx, hh + 0.14, zz + 0.055), V(0.68, 0.12, 0.04));
        y.block(13, V(xx, hh, zz + 0.055), V(0.25, 0.19, 0.04));
        y.block(13, V(xx, hh - 0.12, zz + 0.055), V(0.46, 0.08, 0.04));
    }
    // Curtain wall with a recessed plank gate, crenels and roofed watchtowers.
    let gz = -11.;
    for (xx, w) in [(-8.35, 21.3), (12.25, 14.)] {
        stonework(&mut y, xx, gz, w, 10, 1.08, 0.);
    }
    for j in -18..20 {
        y.block(
            2 + (j + 19) as usize % 4,
            V(j as f32, 4.52, gz),
            V(0.66, 0.62, 1.1),
        );
    }
    for j in 0..13 {
        y.block(
            j % 2,
            V(2.33 + j as f32 * 0.232, 1.72, gz),
            V(0.226, 3.46, 0.43),
        );
    }
    for h in [0.7, 2.18] {
        y.block(12, V(3.72, h, gz + 0.26), V(2.93, 0.13, 0.11));
    }
    for j in 0..13 {
        let a = (j as f32 + 0.5) * PI / 13.;
        y.shape(
            false,
            2 + j % 4,
            V(3.74 + 1.64 * a.cos(), 2.42 + 1.64 * a.sin(), gz + 0.05),
            V(0.48, 0.43, 1.28),
            Quat::from_rotation_z(a),
        );
    }
    for xx in [1.26, 6.22] {
        lamp(&mut y, &mut c, V(xx, 2.56, gz + 0.89), false);
    }
    for (xx, zz, h) in [(2.6, -21., 11.2), (13., -26., 7.5), (-16., -20., 7.)] {
        y.block(3, V(xx, h * 0.5, zz), V(2.9, h, 2.9));
        for s in [-1., 1.] {
            y.block(4, V(xx + s * 1.32, h * 0.5, zz + 1.42), V(0.26, h, 0.23));
        }
        y.block(1, V(xx, h + 0.1, zz), V(3.3, 0.28, 3.3));
        y.block(0, V(xx, h + 0.74, zz), V(2.92, 1.20, 2.92));
        for i in -1..2 {
            y.block(
                10,
                V(xx + i as f32 * 0.76, h + 0.72, zz + 1.47),
                V(0.20, 0.52, 0.06),
            );
        }
        for i in 0..9 {
            let w = 3.9 - i as f32 * 0.40;
            y.block(7, V(xx, h + 1.44 + i as f32 * 0.18, zz), V(w, 0.21, w));
        }
        y.block(12, V(xx, h + 3.34, zz), V(0.09, 1., 0.09));
    }
    for j in 0..58 {
        let x = j as f32 * 1.05 - 30.;
        let n = noise(j, 81);
        kit(
            &mut c,
            &assets,
            "nature-kit",
            "tree_pineTallA",
            V(x, 0., -18. - noise(32, j) * 11.),
            4. + n * 3.4,
            n * 6.,
        );
    }
    for (x, z) in [
        (-7.9, 4.2),
        (-6.92, 4.33),
        (-8.5, 3.4),
        (-3.6, -2.0),
        (6.9, 3.05),
        (7.85, 2.96),
        (0.8, -9.8),
    ] {
        kit(
            &mut c,
            &assets,
            "mini-dungeon",
            "barrel",
            V(x, 0.06, z),
            2.1,
            0.32 * x,
        );
    }
    for (x, z) in [(-4., 0.7), (-7.1, -0.4), (-3.4, -2.9)] {
        kit(
            &mut c,
            &assets,
            "mini-dungeon",
            "table",
            V(x, 0.06, z),
            2.3,
            0.,
        );
        kit(
            &mut c,
            &assets,
            "mini-dungeon",
            "pot",
            V(x + 0.37, 0.79, z),
            0.61,
            0.36,
        );
    }
    kit(
        &mut c,
        &assets,
        "fantasy-town-kit",
        "cart",
        V(6.6, 0.06, -3.1),
        2.65,
        -0.5,
    );
    kit(
        &mut c,
        &assets,
        "fantasy-town-kit",
        "wheel",
        V(-9.83, 0.06, 2.8),
        3.,
        FRAC_PI_2,
    );
    for j in 0..7 {
        kit(
            &mut c,
            &assets,
            "mini-dungeon",
            "weapon-sword",
            V(-8. + j as f32 * 0.35, 1.84, -0.37),
            1.65,
            0.14 * j as f32,
        );
    }
    y.block(0, V(-6.95, 1.8, -0.53), V(2.75, 0.16, 0.21));
    // Split firewood stacked under the smithy eaves.
    for row in 0..4 {
        for log in 0..(7 - row) {
            let x = -9.2 + (log as f32 + row as f32 * 0.5) * 0.27;
            y.shape(
                true,
                (row + log) % 2,
                V(x, 0.18 + row as f32 * 0.23, 2.22),
                V(0.29, 0.88 + noise(row as i32, log as i32) * 0.18, 0.28),
                Quat::from_rotation_x(FRAC_PI_2),
            );
        }
    }
    // The ferry's chained lanterns bracket its walkable landing.
    for (x, z) in [(-1.95, 15.), (1.95, 15.), (3.55, 11.1)] {
        stonework(&mut y, x, z, 0.84, 2, 0.82, 0.);
        y.block(0, V(x, 1.47, z), V(0.21, 1.6, 0.21));
        y.block(1, V(x - 0.23, 2.22, z), V(0.69, 0.14, 0.18));
        lamp(&mut y, &mut c, V(x - 0.46, 1.70, z), true);
    }
    // Small warm grass tussocks gather between the loose edge stones.
    for tuft in 0..1450 {
        let x = noise(tuft, 91) * 31. - 15.5;
        let z = noise(107, tuft) * 26. - 9.;
        if x.abs() < 3.9 + 0.7 * (z * 0.35).cos() || (z < 4.7 && z > -9. && x.abs() < 11.) {
            continue;
        }
        for blade in 0..7 {
            let dx = noise(tuft, blade + 25) - 0.5;
            let dz = noise(blade + 45, tuft) - 0.5;
            let foot = V(x + dx * 0.33, 0.04, z + dz * 0.33);
            y.beam(
                19 + (blade % 2) as usize,
                foot,
                foot + V(dx * 0.26, 0.12 + noise(tuft, blade) * 0.17, dz * 0.26),
                0.025,
                0.016,
            );
        }
    }
    for j in 0..95 {
        let x = noise(j, 14) * 35. - 17.5;
        let z = noise(6, j) * 33. - 15.;
        if x.abs() < 3.5 || (z < 5. && z > -10. && x.abs() < 9.) {
            continue;
        }
        kit(
            &mut c,
            &assets,
            "nature-kit",
            "grass",
            V(x, 0.06, z),
            0.50 + noise(j, 7) * 0.55,
            noise(j, 15) * 6.,
        );
    }
    for (mat, mesh) in y.batches {
        c.spawn((
            Silt,
            Mesh3d(meshes.add(mesh)),
            MeshMaterial3d(y.materials[mat].clone()),
            Transform::from_xyz(600., 0., 0.),
        ));
    }
}
