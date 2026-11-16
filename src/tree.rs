//! Standing yard trees. Kenney's pines are the same stacked cone twice, with
//! black gaps between the discs. These overlap, lean, and differ per name.

use bevy::asset::RenderAssetUsages;
use bevy::mesh::{Indices, Mesh, PrimitiveTopology};
use bevy::prelude::*;

use crate::data::ZoneProp;

pub struct TreeMaterials {
    pub bark: Handle<StandardMaterial>,
    pub deep: Handle<StandardMaterial>,
    pub leaf: Handle<StandardMaterial>,
}

pub fn standing_tree(model: &str) -> bool {
    let file = model.rsplit(['/', '\\']).next().unwrap_or(model);
    matches!(
        file,
        "pine.glb"
            | "pine-crooked.glb"
            | "tree.glb"
            | "tree-crooked.glb"
            | "tree-high.glb"
            | "tree-high-crooked.glb"
    )
}

pub fn spawn_tree(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &TreeMaterials,
    prop: &ZoneProp,
    transform: Transform,
) {
    let file = prop
        .model
        .rsplit(['/', '\\'])
        .next()
        .unwrap_or(prop.model.as_str());
    let broadleaf = file.starts_with("tree");
    let crooked = file.contains("crooked");
    let (bark, deep, leaf) = build_tree(&prop.name, broadleaf, crooked);
    commands.spawn((
        Name::new(prop.name.clone()),
        Mesh3d(meshes.add(bark)),
        MeshMaterial3d(mats.bark.clone()),
        transform,
    ));
    commands.spawn((
        Name::new(format!("{} foliage", prop.name)),
        Mesh3d(meshes.add(deep)),
        MeshMaterial3d(mats.deep.clone()),
        transform,
    ));
    commands.spawn((
        Name::new(format!("{} tips", prop.name)),
        Mesh3d(meshes.add(leaf)),
        MeshMaterial3d(mats.leaf.clone()),
        transform,
    ));
}

fn build_tree(name: &str, broadleaf: bool, crooked: bool) -> (Mesh, Mesh, Mesh) {
    let mut seed = hash_name(name);
    let lean_amt = if crooked { 0.34 } else { 0.12 } + rnd(&mut seed) * 0.1;
    let lean_yaw = rnd(&mut seed) * std::f32::consts::TAU;
    let lean = Vec3::new(
        lean_yaw.cos() * lean_amt,
        1.0,
        lean_yaw.sin() * lean_amt,
    )
    .normalize();

    let mut bark = Builder::default();
    let mut deep = Builder::default();
    let mut leaf = Builder::default();

    if broadleaf {
        let height = 1.7 + rnd(&mut seed) * 0.55;
        let trunk_h = height * 0.42;
        curved_trunk(&mut bark, &mut seed, lean, trunk_h, 0.11, 0.05);
        let crown = lean * (trunk_h * 0.92);
        for i in 0..5 {
            let ang = rnd(&mut seed) * std::f32::consts::TAU;
            let dist = if i == 0 { 0.0 } else { 0.22 + rnd(&mut seed) * 0.28 };
            let lift = 0.15 + rnd(&mut seed) * 0.45;
            let center = crown + Vec3::new(ang.cos() * dist, lift, ang.sin() * dist);
            let radius = 0.38 + rnd(&mut seed) * 0.28;
            let target = if i % 2 == 0 { &mut leaf } else { &mut deep };
            target.blob(center, radius, 3, 5);
        }
    } else {
        let height = 2.3 + rnd(&mut seed) * 0.55;
        let width = 0.75 + rnd(&mut seed) * 0.55;
        curved_trunk(&mut bark, &mut seed, lean, height * 0.16, 0.2, 0.09);
        let clumps = 5;
        for i in 0..clumps {
            let t = i as f32 / (clumps as f32 - 1.0);
            let y = height * (0.22 + t * 0.74);
            let envelope = (1.0 - t * 0.78).max(0.22);
            let side = Vec3::new(
                (rnd(&mut seed) - 0.5) * 0.7 * envelope,
                0.0,
                (rnd(&mut seed) - 0.5) * 0.7 * envelope,
            );
            let center = lean * y + side;
            let rx = (0.42 + envelope * 0.5) * width;
            let ry = 0.38 + envelope * 0.22;
            let rz = rx * (0.78 + rnd(&mut seed) * 0.3);
            let target = if i % 2 == 0 { &mut leaf } else { &mut deep };
            target.ellipsoid(center, Vec3::new(rx, ry, rz), 5, 8);
        }
    }

    (bark.mesh(), deep.mesh(), leaf.mesh())
}

fn curved_trunk(bark: &mut Builder, seed: &mut u32, lean: Vec3, height: f32, r0: f32, r1: f32) {
    let mid = lean * (height * 0.5)
        + Vec3::new((rnd(seed) - 0.5) * 0.08, 0.0, (rnd(seed) - 0.5) * 0.08);
    bark.frustum(Vec3::ZERO, mid, r0, (r0 + r1) * 0.5, 6);
    bark.frustum(mid, lean * height, (r0 + r1) * 0.5, r1, 6);
}

fn hash_name(name: &str) -> u32 {
    let mut n = 2166136261u32;
    for b in name.bytes() {
        n ^= b as u32;
        n = n.wrapping_mul(16777619);
    }
    n
}

fn rnd(state: &mut u32) -> f32 {
    *state = state.wrapping_mul(1664525).wrapping_add(1013904223);
    (*state >> 8) as f32 / 16_777_216.0
}

#[derive(Default)]
struct Builder {
    pos: Vec<[f32; 3]>,
    nrm: Vec<[f32; 3]>,
    idx: Vec<u32>,
}

impl Builder {
    fn tri(&mut self, a: Vec3, b: Vec3, c: Vec3) {
        let n = (b - a).cross(c - a).normalize_or_zero();
        if n == Vec3::ZERO {
            return;
        }
        let base = self.pos.len() as u32;
        self.pos.push(a.to_array());
        self.pos.push(b.to_array());
        self.pos.push(c.to_array());
        let n = n.to_array();
        self.nrm.push(n);
        self.nrm.push(n);
        self.nrm.push(n);
        self.idx.extend([base, base + 1, base + 2]);
    }

    fn cone(&mut self, apex: Vec3, base: Vec3, radius: f32, sides: u32) {
        let (u, v) = basis(apex - base);
        let ring: Vec<Vec3> = (0..sides)
            .map(|i| {
                let a = i as f32 / sides as f32 * std::f32::consts::TAU;
                base + (u * a.cos() + v * a.sin()) * radius
            })
            .collect();
        for i in 0..sides as usize {
            let j = (i + 1) % sides as usize;
            self.tri(apex, ring[i], ring[j]);
        }
    }

    fn frustum(&mut self, bottom: Vec3, top: Vec3, r0: f32, r1: f32, sides: u32) {
        let (u, v) = basis(top - bottom);
        let ring = |center: Vec3, radius: f32| -> Vec<Vec3> {
            (0..sides)
                .map(|i| {
                    let a = i as f32 / sides as f32 * std::f32::consts::TAU;
                    center + (u * a.cos() + v * a.sin()) * radius
                })
                .collect()
        };
        let low = ring(bottom, r0);
        let high = ring(top, r1);
        for i in 0..sides as usize {
            let j = (i + 1) % sides as usize;
            self.tri(low[i], high[i], high[j]);
            self.tri(low[i], high[j], low[j]);
        }
    }

    fn blob(&mut self, center: Vec3, radius: f32, lat_steps: u32, lon_steps: u32) {
        self.ellipsoid(center, Vec3::splat(radius), lat_steps, lon_steps);
    }

    fn ellipsoid(&mut self, center: Vec3, radii: Vec3, lat_steps: u32, lon_steps: u32) {
        let mut rings: Vec<Vec<Vec3>> = Vec::new();
        for lat in 0..=lat_steps {
            let v = lat as f32 / lat_steps as f32;
            let phi = v * std::f32::consts::PI;
            let mut ring = Vec::new();
            for lon in 0..lon_steps {
                let u = lon as f32 / lon_steps as f32;
                let theta = u * std::f32::consts::TAU;
                ring.push(
                    center
                        + Vec3::new(
                            phi.sin() * theta.cos() * radii.x,
                            phi.cos() * radii.y,
                            phi.sin() * theta.sin() * radii.z,
                        ),
                );
            }
            rings.push(ring);
        }
        for lat in 0..lat_steps as usize {
            for lon in 0..lon_steps as usize {
                let lon2 = (lon + 1) % lon_steps as usize;
                let a = rings[lat][lon];
                let b = rings[lat][lon2];
                let c = rings[lat + 1][lon];
                let d = rings[lat + 1][lon2];
                self.tri(a, c, b);
                self.tri(b, c, d);
            }
        }
    }

    fn mesh(self) -> Mesh {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.pos);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.nrm);
        mesh.insert_indices(Indices::U32(self.idx));
        mesh
    }
}

fn basis(axis: Vec3) -> (Vec3, Vec3) {
    let axis = axis.normalize_or_zero();
    let helper = if axis.dot(Vec3::Y).abs() > 0.85 {
        Vec3::X
    } else {
        Vec3::Y
    };
    let u = axis.cross(helper).normalize_or_zero();
    let v = axis.cross(u).normalize_or_zero();
    (u, v)
}
