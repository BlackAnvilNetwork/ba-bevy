//! Three mathematically different trees. Trunks and limbs are tapered prisms.

use crate::extract::CpuMesh;

struct Node {
    origin: [f32; 3],
    dir: [f32; 3],
    length: f32,
    radius: f32,
    depth: u32,
}

fn add(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] + b[0], a[1] + b[1], a[2] + b[2]]
}
fn mul(a: [f32; 3], s: f32) -> [f32; 3] {
    [a[0] * s, a[1] * s, a[2] * s]
}
fn norm(a: [f32; 3]) -> [f32; 3] {
    let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt().max(1e-5);
    mul(a, 1.0 / l)
}
fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}
fn hash(i: u32) -> f32 {
    let mut n = i.wrapping_mul(0x27d4_eb2d) ^ 0x1656_67b1;
    n ^= n >> 15;
    n = n.wrapping_mul(0x85eb_ca6b);
    (n & 0xffff) as f32 / 65536.0
}

fn prism(mesh: &mut CpuMesh, a: [f32; 3], b: [f32; 3], ra: f32, rb: f32, sides: u32) {
    let axis = norm([b[0] - a[0], b[1] - a[1], b[2] - a[2]]);
    let helper = if axis[1].abs() > 0.9 { [1.0, 0.0, 0.0] } else { [0.0, 1.0, 0.0] };
    let t = norm(cross(axis, helper));
    let bt = norm(cross(axis, t));
    let sides = sides.max(3);
    let mut ring_a = Vec::with_capacity(sides as usize);
    let mut ring_b = Vec::with_capacity(sides as usize);
    for i in 0..sides {
        let ang = i as f32 / sides as f32 * std::f32::consts::TAU;
        let (s, c) = ang.sin_cos();
        let off = add(mul(t, c), mul(bt, s));
        ring_a.push(add(a, mul(off, ra)));
        ring_b.push(add(b, mul(off, rb)));
    }
    for i in 0..sides as usize {
        let j = (i + 1) % sides as usize;
        let n = norm(cross(
            [ring_a[j][0] - ring_a[i][0], ring_a[j][1] - ring_a[i][1], ring_a[j][2] - ring_a[i][2]],
            [ring_b[i][0] - ring_a[i][0], ring_b[i][1] - ring_a[i][1], ring_b[i][2] - ring_a[i][2]],
        ));
        mesh.push_quad([ring_a[i], ring_a[j], ring_b[j], ring_b[i]], n);
    }
}

fn leaves(mesh: &mut CpuMesh, at: [f32; 3], radius: f32, seed: u32) {
    for k in 0..5 {
        let a = hash(seed + k * 3) * std::f32::consts::TAU;
        let e = (hash(seed + k * 5) - 0.2) * 0.8;
        let dir = norm([a.cos(), 0.35 + e, a.sin()]);
        let tip = add(at, mul(dir, radius));
        prism(mesh, at, tip, radius * 0.22, radius * 0.05, 4);
    }
}

/// `kind` 0 pine, 1 oak, 2 willow — different branching, same mesh path.
pub fn tree_mesh(kind: u32, seed: u32) -> (CpuMesh, CpuMesh) {
    let mut wood = CpuMesh::default();
    let mut leaf = CpuMesh::default();
    let (max_depth, spread, up_bias, leaf_r) = match kind % 3 {
        0 => (4, 0.35, 0.82, 0.28),
        1 => (3, 0.85, 0.45, 0.42),
        _ => (4, 0.7, 0.15, 0.22),
    };
    let mut stack = vec![Node {
        origin: [0.0, 0.0, 0.0],
        dir: [0.0, 1.0, 0.0],
        length: 1.6,
        radius: 0.16,
        depth: 0,
    }];
    let mut id = seed;
    while let Some(n) = stack.pop() {
        let end = add(n.origin, mul(n.dir, n.length));
        prism(&mut wood, n.origin, end, n.radius, n.radius * 0.62, 6);
        if n.depth >= max_depth {
            leaves(&mut leaf, end, leaf_r * (1.0 + hash(id)), id);
            continue;
        }
        let children = if kind % 3 == 0 { 3 } else { 2 };
        for c in 0..children {
            id = id.wrapping_add(17 + c);
            let yaw = hash(id) * std::f32::consts::TAU;
            let tilt = spread * (0.45 + hash(id.wrapping_add(9)));
            let dir = norm([
                yaw.cos() * tilt,
                up_bias + (1.0 - up_bias) * hash(id.wrapping_add(4)),
                yaw.sin() * tilt,
            ]);
            // Willow droops on later depths.
            let dir = if kind % 3 == 2 && n.depth > 0 {
                norm([dir[0], dir[1] - 0.55, dir[2]])
            } else {
                dir
            };
            stack.push(Node {
                origin: end,
                dir,
                length: n.length * (0.62 + 0.1 * hash(id.wrapping_add(2))),
                radius: n.radius * 0.58,
                depth: n.depth + 1,
            });
        }
    }
    (wood, leaf)
}
