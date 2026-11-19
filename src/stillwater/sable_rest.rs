//! Cael's working smithy and the fortified ferry courtyard.
use super::{ClipCliffs, ClipLeaves, Silt};
use ba_protocol::stillwater::ground;
use bevy::{
    image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
    prelude::*,
};
use std::{
    collections::BTreeMap,
    f32::consts::{FRAC_PI_2, PI, TAU},
};

const V: fn(f32, f32, f32) -> Vec3 = Vec3::new;
type Surface = usize;
const DOOR_HINGE: Vec3 = Vec3::new(608.41, 1.16, 2.18);
const DOOR_WIDTH: f32 = 1.32;

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

    /// Vertical masonry: world UVs so a castle albedo tiles across the wall,
    /// not as a stamped glyph on every block.
    fn masonry(&mut self, material: Surface, at: Vec3, size: Vec3) {
        let mut mesh = self.cube.clone().transformed_by(
            self.frame.mul_transform(Transform::from_translation(at).with_scale(size)),
        );
        if let Some(uvs) = mesh.attribute(Mesh::ATTRIBUTE_POSITION).and_then(|a| {
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
                            [p[0] * 0.28, p[1] * 0.28]
                        } else {
                            [p[2] * 0.28, p[1] * 0.28]
                        }
                    })
                    .collect::<Vec<[f32; 2]>>()
            })
        }) {
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        }
        match self.batches.entry(material) {
            std::collections::btree_map::Entry::Occupied(mut e) => {
                e.get_mut().merge(&mesh).unwrap()
            }
            std::collections::btree_map::Entry::Vacant(e) => {
                e.insert(mesh);
            }
        }
    }

    /// Courtyard sett: cube with world-XZ UVs so Dark Mod cobble reads as one bed.
    fn sett(&mut self, material: Surface, at: Vec3, size: Vec3, yaw: f32) {
        let mut mesh = self.cube.clone().transformed_by(
            self.frame.mul_transform(
                Transform::from_translation(at)
                    .with_scale(size)
                    .with_rotation(Quat::from_rotation_y(yaw)),
            ),
        );
        if let Some(uvs) = mesh.attribute(Mesh::ATTRIBUTE_POSITION).and_then(|a| {
            a.as_float3().map(|pos| {
                pos.iter()
                    .map(|p| [p[0] * 0.42, p[2] * 0.42])
                    .collect::<Vec<[f32; 2]>>()
            })
        }) {
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        }
        match self.batches.entry(material) {
            std::collections::btree_map::Entry::Occupied(mut e) => {
                e.get_mut().merge(&mesh).unwrap()
            }
            std::collections::btree_map::Entry::Vacant(e) => {
                e.insert(mesh);
            }
        }
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

fn stonework(
    y: &mut Yard,
    x: f32,
    z: f32,
    width: f32,
    rows: usize,
    depth: f32,
    base: f32,
    mat: Surface,
) {
    for row in 0..rows {
        let mut edge = -width * 0.5;
        for j in 0.. {
            let w = (0.68 + noise(row as i32, j) * 0.52).min(width * 0.5 - edge);
            if w <= 0.01 {
                break;
            }
            y.masonry(
                mat,
                V(x + edge + w * 0.5, base + row as f32 * 0.43 + 0.21, z),
                V(w - 0.012, 0.42, depth + 0.07 * noise(row as i32, j + 7)),
            );
            edge += w;
        }
    }
}

// A pegged oak truss under individually coursed overlapping slate.
fn hall(
    y: &mut Yard,
    x: f32,
    z: f32,
    w: f32,
    d: f32,
    closed: bool,
    stone: Surface,
    plaster: Surface,
) {
    let eave = if closed { 4.3 } else { 3.28 };
    let rise = w * 0.39;
    let front = z + d * 0.5;
    if closed {
        let door_w = 1.32;
        let door_h = 2.28;
        let door_x = x + 0.17;
        let half = w * 0.5;
        let left_span = (door_x - door_w * 0.5) - (x - half);
        let right_span = (x + half) - (door_x + door_w * 0.5);
        stonework(
            y,
            x - half + left_span * 0.5,
            front - 0.3,
            left_span,
            3,
            0.58,
            0.,
            stone,
        );
        stonework(
            y,
            x + half - right_span * 0.5,
            front - 0.3,
            right_span,
            3,
            0.58,
            0.,
            stone,
        );
        y.block(
            plaster,
            V(x - half + left_span * 0.5, 2.55, front - 0.28),
            V(left_span - 0.04, 3.5, 0.42),
        );
        y.block(
            plaster,
            V(x + half - right_span * 0.5, 2.55, front - 0.28),
            V(right_span - 0.04, 3.5, 0.42),
        );
        y.block(
            plaster,
            V(door_x, door_h + 1.15, front - 0.28),
            V(door_w + 0.16, 2.3, 0.42),
        );
        y.block(
            plaster,
            V(x, 2.45, z - d * 0.5 + 0.22),
            V(w - 0.18, 4.7, 0.40),
        );
        y.block(
            plaster,
            V(x - half + 0.18, 2.45, z),
            V(0.38, 4.7, d - 0.55),
        );
        y.block(
            plaster,
            V(x + half - 0.18, 2.45, z),
            V(0.38, 4.7, d - 0.55),
        );
        y.block(0, V(x, 0.05, z), V(w - 0.55, 0.08, d - 0.6));
        y.block(0, V(x, 4.05, z), V(w - 0.4, 0.10, d - 0.5));
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
        stonework(y, x, z - d * 0.5, w, 6, 0.5, 0., stone);
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

fn grove(c: &mut Commands, a: &AssetServer, name: &str, p: Vec3, scale: f32, yaw: f32) {
    let mirror = if (yaw * 7.).fract() > 0.5 { -1. } else { 1. };
    let wx = p.x + 600.;
    let wz = p.z;
    c.spawn((
        Silt,
        ClipLeaves,
        WorldAssetRoot(a.load(format!("darkmod/trees/{name}.glb#Scene0"))),
        Transform::from_translation(V(wx, ground(wx, wz) - scale * 14., wz))
            .with_scale(Vec3::new(scale * mirror, scale, scale))
            .with_rotation(Quat::from_euler(
                EulerRot::YXZ,
                yaw,
                (scale - 0.03) * 1.4,
                (yaw - 1.) * 0.04,
            )),
    ));
}

fn outcrop(c: &mut Commands, a: &AssetServer, name: &str, p: Vec3, scale: f32, yaw: f32) {
    let wx = p.x + 600.;
    let wz = p.z;
    c.spawn((
        Silt,
        ClipCliffs,
        WorldAssetRoot(a.load(format!("darkmod/rocks/{name}.glb#Scene0"))),
        Transform::from_translation(V(wx, ground(wx, wz) - 0.7, wz))
            .with_scale(Vec3::splat(scale))
            .with_rotation(Quat::from_rotation_y(yaw)),
    ));
}

fn rest_floor() -> Mesh {
    let cols = 48usize;
    let rows = 48usize;
    let (x0, x1, z0, z1) = (555., 645., -48., 48.);
    let mut positions = Vec::with_capacity((cols + 1) * (rows + 1));
    let mut uvs = Vec::with_capacity((cols + 1) * (rows + 1));
    for j in 0..=rows {
        let z = z0 + (z1 - z0) * j as f32 / rows as f32;
        for i in 0..=cols {
            let x = x0 + (x1 - x0) * i as f32 / cols as f32;
            positions.push([x, ground(x, z), z]);
            uvs.push([x * 0.1, z * 0.1]);
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
        let pa = Vec3::from(positions[tri[0] as usize]);
        let pb = Vec3::from(positions[tri[1] as usize]);
        let pc = Vec3::from(positions[tri[2] as usize]);
        let n = (pb - pa).cross(pc - pa);
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

fn oak_table(y: &mut Yard, x: f32, z: f32, yaw: f32) {
    let rot = Quat::from_rotation_y(yaw);
    y.shape(false, 0, V(x, 0.72, z), V(1.42, 0.08, 0.74), rot);
    for (dx, dz) in [(-0.58, -0.28), (0.58, -0.28), (-0.58, 0.28), (0.58, 0.28)] {
        y.shape(
            false,
            1,
            V(x, 0.36, z) + rot * V(dx, 0., dz),
            V(0.08, 0.72, 0.08),
            rot,
        );
    }
}

fn oak_cask(y: &mut Yard, x: f32, z: f32, seed: i32) {
    let n = noise(seed, 3);
    for i in 0..5 {
        let r = 0.28 + if i == 0 || i == 4 { -0.04 } else { 0.03 } + n * 0.05;
        y.block(
            i % 2,
            V(x, 0.12 + i as f32 * 0.16, z),
            V(r * 2., 0.16, r * 2.),
        );
    }
}

fn wagon(y: &mut Yard, x: f32, z: f32) {
    y.block(0, V(x, 0.58, z), V(2.05, 0.12, 1.02));
    y.block(1, V(x, 0.88, z + 0.46), V(1.9, 0.48, 0.08));
    y.block(1, V(x, 0.88, z - 0.46), V(1.9, 0.48, 0.08));
    y.block(0, V(x - 0.92, 0.82, z), V(0.08, 0.38, 0.92));
    for (dx, dz) in [(-0.72, 0.58), (0.72, 0.58), (-0.72, -0.58), (0.72, -0.58)] {
        y.shape(
            false,
            0,
            V(x + dx, 0.36, z + dz),
            V(0.12, 0.7, 0.7),
            Quat::from_rotation_z(FRAC_PI_2),
        );
    }
}

#[derive(Component)]
pub struct Cinder {
    origin: Vec3,
    phase: f32,
}

#[derive(Component)]
pub struct InnDoor {
    pub open: bool,
    angle: f32,
}

#[derive(Component, Clone, Copy)]
pub struct WallBox {
    min: Vec2,
    max: Vec2,
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
    let mut tile = |path: &'static str, tint: [f32; 3]| {
        let img = assets
            .load_builder()
            .with_settings(|s: &mut ImageLoaderSettings| {
                s.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                    address_mode_u: ImageAddressMode::Repeat,
                    address_mode_v: ImageAddressMode::Repeat,
                    ..ImageSamplerDescriptor::linear()
                });
            })
            .load(path);
        let [r, g, b] = tint;
        materials.push(mats.add(StandardMaterial {
            base_color: Color::srgb(r, g, b),
            base_color_texture: Some(img),
            perceptual_roughness: 0.93,
            ..default()
        }));
        materials.len() - 1
    };
    let cobble_mat = tile(
        "darkmod/stone/cobblestones_irregular.png",
        [0.78, 0.74, 0.68],
    );
    let castle_mat = tile("darkmod/stone/castle_blocks.jpg", [0.84, 0.82, 0.78]);
    let plaster_mat = tile("darkmod/stone/inn_plaster.jpg", [0.94, 0.90, 0.82]);
    let door_mat = mats.add(StandardMaterial {
        base_color: Color::srgb(0.72, 0.55, 0.38),
        base_color_texture: Some(assets.load("darkmod/stone/inn_door.jpg")),
        perceptual_roughness: 0.88,
        ..default()
    });
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
    c.spawn((
        Silt,
        Mesh3d(meshes.add(rest_floor())),
        MeshMaterial3d(mats.add(StandardMaterial {
            base_color: Color::srgb(0.48, 0.52, 0.36),
            base_color_texture: Some(grass),
            perceptual_roughness: 0.96,
            cull_mode: None,
            ..default()
        })),
        Transform::IDENTITY,
    ));
    let mut y = Yard {
        cube: Cuboid::from_size(Vec3::ONE).into(),
        cobble: Cylinder::new(0.5, 1.).mesh().resolution(5).build(),
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
    // Dark Mod cobble bed: mixed-size setts, world UVs, no stamped octagon grid.
    y.sett(cobble_mat, V(0., 0.02, 1.2), V(22., 0.03, 28.), 0.07);
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
            let m = noise(col, row);
            if n > 0.18 && full {
                continue;
            }
            y.sett(
                cobble_mat,
                V(
                    x + (n - 0.5) * 0.34,
                    0.034 + n * 0.022,
                    z + (m - 0.5) * 0.32,
                ),
                V(0.48 + n * 0.62, 0.036 + m * 0.028, 0.34 + n * 0.46),
                m * TAU,
            );
        }
    }
    for j in 0..14 {
        let n = noise(j, 41);
        let m = noise(17, j);
        c.spawn((
            Silt,
            WorldAssetRoot(assets.load(format!(
                "darkmod/stones/cobblestone0{}.glb#Scene0",
                1 + j % 2
            ))),
            Transform::from_xyz(
                600. + (n - 0.5) * 16.,
                -0.12,
                (m - 0.5) * 18.,
            )
            .with_scale(Vec3::splat(0.055 + n * 0.04))
            .with_rotation(Quat::from_rotation_y(m * TAU)),
        ));
    }
    // The smithy sits obliquely on the old wall line, exposing its open side.
    let pivot = V(-5.7, 0., 2.95);
    let angle = Quat::from_rotation_y(-0.34);
    y.frame = Transform::from_translation(pivot - angle * pivot).with_rotation(angle);
    hall(&mut y, -5.7, -0.65, 7.4, 7.2, false, castle_mat, plaster_mat);
    y.frame = Transform::IDENTITY;
    hall(&mut y, 8.9, -1.9, 6.8, 7.9, true, castle_mat, plaster_mat);
    // Low curtain returns enclose the courtyard beyond the smithy and inn.
    for side in [-1.0, 1.0] {
        for bay in 0..6 {
            let z = -8.8 + bay as f32 * 3.15;
            for course in 0..8 {
                for j in 0..4 {
                    let offset = (course % 2) as f32 * 0.38;
                    y.masonry(
                        castle_mat,
                        V(
                            side * 16.5,
                            course as f32 * 0.40 + 0.2,
                            z + j as f32 * 0.79 + offset,
                        ),
                        V(0.75, 0.39, 0.78),
                    );
                }
            }
            y.masonry(castle_mat, V(side * 16.5, 1.72, z), V(1.16, 3.44, 0.72));
            y.masonry(castle_mat, V(side * 16.5, 3.49, z), V(1.23, 0.17, 0.85));
            y.masonry(castle_mat, V(side * 16.5, 3.43, z + 1.6), V(0.79, 0.50, 0.91));
        }
    }
    // An arched fire mouth, hood and brick flue behind the open colonnade.
    let fx = -5.45;
    let fz = -1.1;
    for side in [-1., 1.] {
        stonework(&mut y, fx + side * 1.05, fz, 0.58, 4, 1.35, 0., castle_mat);
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
    stonework(&mut y, fx, fz - 0.45, 1.40, 13, 0.97, 2.3, castle_mat);
    y.masonry(castle_mat, V(fx, 7.89, fz - 0.45), V(1.66, 0.25, 1.2));
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
    let door_mesh = meshes.add(Cuboid::from_size(Vec3::new(1.32, 2.28, 0.12)));
    c.spawn((
        Silt,
        InnDoor {
            open: false,
            angle: 0.,
        },
        Transform::from_xyz(DOOR_HINGE.x, DOOR_HINGE.y, DOOR_HINGE.z),
        Visibility::default(),
        children![(
            Mesh3d(door_mesh),
            MeshMaterial3d(door_mat),
            Transform::from_xyz(DOOR_WIDTH * 0.5, 0., 0.),
        )],
    ));
    c.spawn((
        Silt,
        PointLight {
            color: Color::srgb(1., 0.72, 0.42),
            intensity: 42000.,
            range: 8.,
            ..default()
        },
        Transform::from_xyz(608.9, 2.35, -1.7),
    ));
    for (x, z) in [(7.35, -2.55), (10.55, -3.35)] {
        oak_table(&mut y, x, z, 0.2);
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
        stonework(&mut y, xx, gz, w, 10, 1.08, 0., castle_mat);
    }
    for j in -18..20 {
        y.masonry(
            castle_mat,
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
    for j in -10..14 {
        if noise(j, 61) < 0.18 {
            continue;
        }
        y.masonry(
            castle_mat,
            V(
                -19.4 + (noise(j, 4) - 0.5) * 0.5,
                1.55 + noise(j, 8) * 0.4,
                j as f32 * 1.15,
            ),
            V(
                0.92 + noise(j, 11) * 0.28,
                3.05 + noise(j, 15) * 1.1,
                1.08,
            ),
        );
    }
    for (xx, zz, h) in [
        (2.6, -21., 11.2),
        (13., -26., 7.5),
        (-16., -20., 7.),
        (-22., 7., 8.4),
        (-18., 14., 6.8),
    ] {
        y.masonry(castle_mat, V(xx, h * 0.5, zz), V(2.9, h, 2.9));
        for s in [-1., 1.] {
            y.masonry(
                castle_mat,
                V(xx + s * 1.32, h * 0.5, zz + 1.42),
                V(0.26, h, 0.23),
            );
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
    const PINES: [&str; 8] = [
        "spruce_01a",
        "spruce_01b",
        "spruce_01c",
        "spruce_01d",
        "spruce_01e",
        "pine_12",
        "pine_20",
        "pine_30",
    ];
    for j in 0..52 {
        let n = noise(j, 81);
        let a = j as f32 / 52. * TAU + (n - 0.5) * 0.28;
        let r = 18.5 + noise(j, 3) * 6.2;
        let x = a.cos() * r;
        let z = a.sin() * r;
        if z > 9. && x.abs() < 7. {
            continue;
        }
        grove(
            &mut c,
            &assets,
            PINES[j as usize % PINES.len()],
            V(x, 0., z),
            0.028 + n * 0.018,
            n * 6. + noise(j, 9) * 1.4,
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
    for j in 0..18 {
        let n = noise(j, 21);
        let a = j as f32 / 18. * TAU + (n - 0.5) * 0.3;
        let r = 22. + noise(j, 7) * 5.;
        let x = a.cos() * r;
        let z = a.sin() * r;
        if z > 10. && x.abs() < 8. {
            continue;
        }
        outcrop(
            &mut c,
            &assets,
            ROCKS[j as usize % ROCKS.len()],
            V(x, 0., z),
            0.022 + n * 0.02,
            a + PI,
        );
    }
    for (i, (x, z)) in [
        (-7.9, 4.2),
        (-6.92, 4.33),
        (-8.5, 3.4),
        (-3.6, -2.0),
        (6.9, 3.05),
        (7.85, 2.96),
        (0.8, -9.8),
    ]
    .into_iter()
    .enumerate()
    {
        oak_cask(&mut y, x + (noise(i as i32, 2) - 0.5) * 0.2, z, i as i32);
    }
    for (x, z) in [(-4., 0.7), (-7.1, -0.4), (-3.4, -2.9)] {
        oak_table(&mut y, x, z, 0.);
        y.block(1, V(x + 0.37, 0.86, z), V(0.22, 0.18, 0.22));
        y.block(0, V(x + 0.37, 1.02, z), V(0.16, 0.16, 0.16));
    }
    wagon(&mut y, 6.6, -3.1);
    y.shape(
        false,
        0,
        V(-9.83, 0.38, 2.8),
        V(0.12, 0.74, 0.74),
        Quat::from_rotation_z(FRAC_PI_2),
    );
    for j in 0..7 {
        let x = -8. + j as f32 * 0.35 + (noise(j, 5) - 0.5) * 0.08;
        y.block(12, V(x, 1.84, -0.37), V(0.04, 0.04, 0.72));
        y.block(10, V(x, 1.84, -0.72), V(0.12, 0.04, 0.08));
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
        stonework(&mut y, x, z, 0.84, 2, 0.82, 0., castle_mat);
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
    for (minx, maxx, minz, maxz) in [
        (605.35, 605.88, -5.95, 2.20),
        (611.92, 612.45, -5.95, 2.20),
        (605.35, 612.45, -6.15, -5.50),
        (605.35, 608.41, 1.88, 2.38),
        (609.73, 612.45, 1.88, 2.38),
    ] {
        c.spawn((
            Silt,
            WallBox {
                min: Vec2::new(minx, minz),
                max: Vec2::new(maxx, maxz),
            },
        ));
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

pub fn swing_door(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    note: Res<crate::player::NoteComposer>,
    player: Query<&Transform, With<crate::player::Player>>,
    mut doors: Query<(&mut InnDoor, &mut Transform), Without<crate::player::Player>>,
) {
    if note.active {
        return;
    }
    let Ok(player) = player.single() else {
        return;
    };
    for (mut door, mut tf) in &mut doors {
        let near = player.translation.xz().distance(DOOR_HINGE.xz()) < 2.6;
        if near && keys.just_pressed(KeyCode::KeyE) {
            door.open = !door.open;
        }
        let target = if door.open { -1.45 } else { 0. };
        door.angle += (target - door.angle) * (1. - (-time.delta_secs() * 8.).exp());
        tf.rotation = Quat::from_rotation_y(door.angle);
    }
}

pub fn block_player(
    walls: Query<&WallBox>,
    doors: Query<&InnDoor>,
    mut player: Query<&mut Transform, With<crate::player::Player>>,
) {
    let Ok(mut tf) = player.single_mut() else {
        return;
    };
    let p = &mut tf.translation;
    if (p.x - 609.).abs() > 20. {
        return;
    }
    let radius = 0.38;
    let mut push = |min: Vec2, max: Vec2| {
        let cx = p.x.clamp(min.x, max.x);
        let cz = p.z.clamp(min.y, max.y);
        let dx = p.x - cx;
        let dz = p.z - cz;
        let d2 = dx * dx + dz * dz;
        if d2 >= radius * radius {
            return;
        }
        if d2 < 1e-6 {
            let left = p.x - min.x;
            let right = max.x - p.x;
            let south = p.z - min.y;
            let north = max.y - p.z;
            let m = left.min(right).min(south).min(north);
            if m == left {
                p.x = min.x - radius;
            } else if m == right {
                p.x = max.x + radius;
            } else if m == south {
                p.z = min.y - radius;
            } else {
                p.z = max.y + radius;
            }
        } else {
            let d = d2.sqrt();
            let k = (radius - d) / d;
            p.x += dx * k;
            p.z += dz * k;
        }
    };
    for wall in &walls {
        push(wall.min, wall.max);
    }
    if doors.iter().any(|d| !d.open) {
        push(
            Vec2::new(DOOR_HINGE.x, DOOR_HINGE.z - 0.16),
            Vec2::new(DOOR_HINGE.x + DOOR_WIDTH, DOOR_HINGE.z + 0.16),
        );
    }
}
