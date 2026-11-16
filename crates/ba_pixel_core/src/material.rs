/// Surface description shared by SDF shells, meshes, and particle clusters.
#[derive(Clone, Copy, Debug)]
pub struct PixelMaterial {
    pub albedo: [f32; 3],
    pub emission: [f32; 3],
    /// 0 smooth metal .. 1 chalk.
    pub roughness: f32,
}

impl PixelMaterial {
    pub const BARK: Self = Self {
        albedo: [0.28, 0.16, 0.08],
        emission: [0.0, 0.0, 0.0],
        roughness: 0.92,
    };
    pub const LEAF: Self = Self {
        albedo: [0.18, 0.32, 0.12],
        emission: [0.0, 0.0, 0.0],
        roughness: 0.85,
    };
    pub const STONE: Self = Self {
        albedo: [0.34, 0.32, 0.30],
        emission: [0.0, 0.0, 0.0],
        roughness: 0.95,
    };
    pub const IRON: Self = Self {
        albedo: [0.55, 0.56, 0.58],
        emission: [0.02, 0.02, 0.03],
        roughness: 0.35,
    };
    pub const EMBER: Self = Self {
        albedo: [1.0, 0.45, 0.12],
        emission: [1.4, 0.35, 0.05],
        roughness: 1.0,
    };
}
