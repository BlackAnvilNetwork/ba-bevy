//! Minimal SDF graph: primitives the gear tool already uses (sphere, box, cylinder, smin).

#[derive(Clone, Copy, Debug)]
pub struct V3 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
}

impl V3 {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0, z: 0.0 };

    pub fn new(x: f32, y: f32, z: f32) -> Self {
        Self { x, y, z }
    }

    pub fn sub(self, o: Self) -> Self {
        Self::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }

    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y + self.z * self.z).sqrt()
    }
}

pub fn smin(a: f32, b: f32, k: f32) -> f32 {
    if k <= 1e-6 {
        return a.min(b);
    }
    let h = (0.5 + 0.5 * (b - a) / k).clamp(0.0, 1.0);
    b + (a - b) * h - k * h * (1.0 - h)
}

pub fn sd_sphere(p: V3, r: f32) -> f32 {
    p.length() - r
}

pub fn sd_box(p: V3, b: V3) -> f32 {
    let qx = p.x.abs() - b.x;
    let qy = p.y.abs() - b.y;
    let qz = p.z.abs() - b.z;
    let ox = qx.max(0.0);
    let oy = qy.max(0.0);
    let oz = qz.max(0.0);
    let outside = (ox * ox + oy * oy + oz * oz).sqrt();
    let inside = qx.max(qy).max(qz).min(0.0);
    outside + inside
}

pub fn sd_cylinder_y(p: V3, r: f32, h: f32) -> f32 {
    let dx = (p.x * p.x + p.z * p.z).sqrt() - r;
    let dy = p.y.abs() - h;
    let ox = dx.max(0.0);
    let oy = dy.max(0.0);
    (ox * ox + oy * oy).sqrt() + dx.max(dy).min(0.0)
}

fn hash3(ix: i32, iy: i32, iz: i32) -> f32 {
    let mut n = (ix as u32).wrapping_mul(0x27d4_eb2d)
        ^ (iy as u32).wrapping_mul(0x1656_67b1)
        ^ (iz as u32).wrapping_mul(0x85eb_ca6b);
    n ^= n >> 15;
    n = n.wrapping_mul(0x85eb_ca6b);
    n ^= n >> 13;
    (n & 0xffff) as f32 / 65536.0
}

fn value_noise(p: V3) -> f32 {
    let ix = p.x.floor() as i32;
    let iy = p.y.floor() as i32;
    let iz = p.z.floor() as i32;
    let fx = p.x - ix as f32;
    let fy = p.y - iy as f32;
    let fz = p.z - iz as f32;
    let u = fx * fx * (3.0 - 2.0 * fx);
    let v = fy * fy * (3.0 - 2.0 * fy);
    let w = fz * fz * (3.0 - 2.0 * fz);
    let lerp = |a: f32, b: f32, t: f32| a + (b - a) * t;
    let n000 = hash3(ix, iy, iz);
    let n100 = hash3(ix + 1, iy, iz);
    let n010 = hash3(ix, iy + 1, iz);
    let n110 = hash3(ix + 1, iy + 1, iz);
    let n001 = hash3(ix, iy, iz + 1);
    let n101 = hash3(ix + 1, iy, iz + 1);
    let n011 = hash3(ix, iy + 1, iz + 1);
    let n111 = hash3(ix + 1, iy + 1, iz + 1);
    let x00 = lerp(n000, n100, u);
    let x10 = lerp(n010, n110, u);
    let x01 = lerp(n001, n101, u);
    let x11 = lerp(n011, n111, u);
    lerp(lerp(x00, x10, v), lerp(x01, x11, v), w)
}

/// Boulder: sphere with ridged noise. Same idea as gear `RidgedNoise`, smaller graph.
pub fn sd_rock(p: V3, radius: f32, seed: f32) -> f32 {
    let n = value_noise(V3::new(p.x * 2.4 + seed, p.y * 2.4, p.z * 2.4));
    sd_sphere(p, radius) + (n - 0.5) * radius * 0.55
}

/// Blade + grip. `eat` raises a cutting plane from the tip downward (0 intact .. 1 gone).
pub fn sd_sword(p: V3, eat: f32) -> f32 {
    let blade = sd_box(p.sub(V3::new(0.0, 0.62, 0.0)), V3::new(0.045, 0.52, 0.012));
    let fuller = sd_box(p.sub(V3::new(0.0, 0.62, 0.0)), V3::new(0.012, 0.46, 0.02));
    let blade = blade.max(-fuller);
    let guard = sd_box(p.sub(V3::new(0.0, 0.12, 0.0)), V3::new(0.16, 0.02, 0.03));
    let grip = sd_cylinder_y(p.sub(V3::new(0.0, 0.0, 0.0)), 0.028, 0.11);
    let pommel = sd_sphere(p.sub(V3::new(0.0, -0.13, 0.0)), 0.045);
    let body = smin(smin(blade, guard, 0.02), smin(grip, pommel, 0.03), 0.02);
    // Tip is near y=1.14. eat=0 keeps the half-space above the pommel; eat=1 consumes the blade.
    let clip = (1.2 - eat * 1.45) - p.y;
    body.max(-clip)
}
