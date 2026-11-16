//! One destruction behavior on the shared pixel field.
//! The sword stays an SDF; lost cells become particles in a buffer.

use ba_pixel_core::{PixelMaterial, PixelParticle};
use ba_pixel_geometry::{solid_centers, voxel_shell, CpuMesh, V3, sd_sword};

const RES: [i32; 3] = [10, 36, 6];
const ORIGIN: V3 = V3 { x: -0.22, y: -0.2, z: -0.08 };
const EXTENT: V3 = V3 { x: 0.44, y: 1.5, z: 0.16 };

#[derive(Clone, Debug)]
pub struct SwordDissolve {
    pub eat: f32,
    pub rate: f32,
    pub playing: bool,
    pub mesh: CpuMesh,
    pub particles: Vec<PixelParticle>,
    prev: Vec<[f32; 3]>,
}

impl SwordDissolve {
    pub fn intact() -> Self {
        let mut s = Self {
            eat: 0.0,
            rate: 0.22,
            playing: false,
            mesh: CpuMesh::default(),
            particles: Vec::new(),
            prev: Vec::new(),
        };
        s.rebuild(true);
        s
    }

    pub fn trigger(&mut self) {
        self.playing = true;
    }

    pub fn tick(&mut self, dt: f32) {
        if self.playing && self.eat < 1.05 {
            self.eat = (self.eat + self.rate * dt).min(1.05);
            self.rebuild(false);
        }
        let mut i = 0;
        while i < self.particles.len() {
            self.particles[i].step(dt, 2.4, 0.6);
            if self.particles[i].life <= 0.0 || self.particles[i].position[1] < -0.05 {
                self.particles.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    fn rebuild(&mut self, first: bool) {
        let eat = self.eat;
        let sdf = |p: V3| sd_sword(p, eat);
        self.mesh = voxel_shell(sdf, ORIGIN, EXTENT, RES);
        let now = solid_centers(sdf, ORIGIN, EXTENT, RES);
        if !first {
            let lost: Vec<[f32; 3]> = self
                .prev
                .iter()
                .copied()
                .filter(|p| {
                    !now.iter()
                        .any(|q| (q[0] - p[0]).abs() < 0.01 && (q[1] - p[1]).abs() < 0.01)
                })
                .collect();
            for p in lost {
                self.spawn(p);
            }
        }
        self.prev = now;
    }

    fn spawn(&mut self, at: [f32; 3]) {
        let mat = PixelMaterial::EMBER;
        let jitter = |k: f32| (at[1] * 13.1 + k).sin() * 0.4;
        self.particles.push(PixelParticle {
            position: at,
            velocity: [jitter(1.0), 1.4 + jitter(2.0).abs(), jitter(3.0)],
            life: 0.9,
            size: 0.035,
            albedo: mat.albedo,
            emission: mat.emission,
        });
    }

    /// One combined mesh of the live fragments (a single draw, not one entity each).
    pub fn particle_mesh(&self) -> CpuMesh {
        let mut mesh = CpuMesh::default();
        for p in &self.particles {
            let s = p.size * p.life.clamp(0.2, 1.0);
            let [x, y, z] = p.position;
            let quad = [
                [x - s, y, z],
                [x, y + s, z],
                [x + s, y, z],
                [x, y - s * 0.2, z],
            ];
            mesh.push_quad(quad, [0.0, 0.0, 1.0]);
        }
        mesh
    }
}
