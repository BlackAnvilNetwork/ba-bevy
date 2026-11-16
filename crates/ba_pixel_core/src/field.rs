/// One simulated pixel fragment. Stored in a buffer, never as its own entity.
#[derive(Clone, Copy, Debug)]
pub struct PixelParticle {
    pub position: [f32; 3],
    pub velocity: [f32; 3],
    pub life: f32,
    pub size: f32,
    pub albedo: [f32; 3],
    pub emission: [f32; 3],
}

impl PixelParticle {
    pub fn step(&mut self, dt: f32, gravity: f32, drag: f32) {
        self.velocity[1] -= gravity * dt;
        let keep = (1.0 - drag * dt).max(0.0);
        self.velocity[0] *= keep;
        self.velocity[1] *= keep;
        self.velocity[2] *= keep;
        self.position[0] += self.velocity[0] * dt;
        self.position[1] += self.velocity[1] * dt;
        self.position[2] += self.velocity[2] * dt;
        self.life -= dt;
    }
}
