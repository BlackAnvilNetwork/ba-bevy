//! Pixel-quantized shell of an SDF. Exposed faces become quads so the object
//! reads as clusters, while the field itself stays analytic.

use crate::sdf::V3;

#[derive(Clone, Debug, Default)]
pub struct CpuMesh {
    pub positions: Vec<[f32; 3]>,
    pub normals: Vec<[f32; 3]>,
    pub indices: Vec<u32>,
}

impl CpuMesh {
    pub fn push_quad(&mut self, pts: [[f32; 3]; 4], normal: [f32; 3]) {
        let base = self.positions.len() as u32;
        for p in pts {
            self.positions.push(p);
            self.normals.push(normal);
        }
        self.indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    pub fn append(&mut self, other: &CpuMesh) {
        let base = self.positions.len() as u32;
        self.positions.extend_from_slice(&other.positions);
        self.normals.extend_from_slice(&other.normals);
        self.indices.extend(other.indices.iter().map(|i| i + base));
    }
}

const FACES: [(V3, [f32; 3]); 6] = [
    (V3 { x: 1.0, y: 0.0, z: 0.0 }, [1.0, 0.0, 0.0]),
    (V3 { x: -1.0, y: 0.0, z: 0.0 }, [-1.0, 0.0, 0.0]),
    (V3 { x: 0.0, y: 1.0, z: 0.0 }, [0.0, 1.0, 0.0]),
    (V3 { x: 0.0, y: -1.0, z: 0.0 }, [0.0, -1.0, 0.0]),
    (V3 { x: 0.0, y: 0.0, z: 1.0 }, [0.0, 0.0, 1.0]),
    (V3 { x: 0.0, y: 0.0, z: -1.0 }, [0.0, 0.0, -1.0]),
];

/// Cells whose SDF is negative, with a face wherever a neighbor is outside.
pub fn voxel_shell(
    sdf: impl Fn(V3) -> f32,
    origin: V3,
    extent: V3,
    res: [i32; 3],
) -> CpuMesh {
    let nx = res[0].max(2);
    let ny = res[1].max(2);
    let nz = res[2].max(2);
    let dx = extent.x / nx as f32;
    let dy = extent.y / ny as f32;
    let dz = extent.z / nz as f32;
    let mut solid = vec![false; (nx * ny * nz) as usize];
    let idx = |x: i32, y: i32, z: i32| (x + y * nx + z * nx * ny) as usize;
    for z in 0..nz {
        for y in 0..ny {
            for x in 0..nx {
                let p = V3::new(
                    origin.x + (x as f32 + 0.5) * dx,
                    origin.y + (y as f32 + 0.5) * dy,
                    origin.z + (z as f32 + 0.5) * dz,
                );
                solid[idx(x, y, z)] = sdf(p) < 0.0;
            }
        }
    }

    let mut mesh = CpuMesh::default();
    for z in 0..nz {
        for y in 0..ny {
            for x in 0..nx {
                if !solid[idx(x, y, z)] {
                    continue;
                }
                let x0 = origin.x + x as f32 * dx;
                let y0 = origin.y + y as f32 * dy;
                let z0 = origin.z + z as f32 * dz;
                let x1 = x0 + dx;
                let y1 = y0 + dy;
                let z1 = z0 + dz;
                for (dir, normal) in FACES {
                    let nxb = x + dir.x as i32;
                    let nyb = y + dir.y as i32;
                    let nzb = z + dir.z as i32;
                    let exposed = nxb < 0
                        || nyb < 0
                        || nzb < 0
                        || nxb >= nx
                        || nyb >= ny
                        || nzb >= nz
                        || !solid[idx(nxb, nyb, nzb)];
                    if !exposed {
                        continue;
                    }
                    let quad = face_quad(x0, y0, z0, x1, y1, z1, normal);
                    mesh.push_quad(quad, normal);
                }
            }
        }
    }
    mesh
}

fn face_quad(x0: f32, y0: f32, z0: f32, x1: f32, y1: f32, z1: f32, n: [f32; 3]) -> [[f32; 3]; 4] {
    match n {
        [1.0, 0.0, 0.0] => [[x1, y0, z0], [x1, y1, z0], [x1, y1, z1], [x1, y0, z1]],
        [-1.0, 0.0, 0.0] => [[x0, y0, z1], [x0, y1, z1], [x0, y1, z0], [x0, y0, z0]],
        [0.0, 1.0, 0.0] => [[x0, y1, z1], [x1, y1, z1], [x1, y1, z0], [x0, y1, z0]],
        [0.0, -1.0, 0.0] => [[x0, y0, z0], [x1, y0, z0], [x1, y0, z1], [x0, y0, z1]],
        [0.0, 0.0, 1.0] => [[x0, y0, z1], [x1, y0, z1], [x1, y1, z1], [x0, y1, z1]],
        _ => [[x1, y0, z0], [x0, y0, z0], [x0, y1, z0], [x1, y1, z0]],
    }
}

/// Solid cell centers, used to spawn pixel fragments when the field changes.
pub fn solid_centers(
    sdf: impl Fn(V3) -> f32,
    origin: V3,
    extent: V3,
    res: [i32; 3],
) -> Vec<[f32; 3]> {
    let nx = res[0].max(2);
    let ny = res[1].max(2);
    let nz = res[2].max(2);
    let dx = extent.x / nx as f32;
    let dy = extent.y / ny as f32;
    let dz = extent.z / nz as f32;
    let mut out = Vec::new();
    for z in 0..nz {
        for y in 0..ny {
            for x in 0..nx {
                let p = V3::new(
                    origin.x + (x as f32 + 0.5) * dx,
                    origin.y + (y as f32 + 0.5) * dy,
                    origin.z + (z as f32 + 0.5) * dz,
                );
                if sdf(p) < 0.0 {
                    out.push([p.x, p.y, p.z]);
                }
            }
        }
    }
    out
}
