//! Controlled palette steps. Lighting quantizes into these instead of a continuous grade.

/// Stick-adjacent dusk / forest swatches (sRGB 0..1).
pub const FOREST: [[f32; 3]; 8] = [
    [0.08, 0.07, 0.06],
    [0.16, 0.12, 0.08],
    [0.22, 0.28, 0.14],
    [0.34, 0.42, 0.18],
    [0.45, 0.32, 0.18],
    [0.72, 0.55, 0.28],
    [0.92, 0.72, 0.38],
    [0.96, 0.90, 0.72],
];

pub const EMBER: [[f32; 3]; 4] = [
    [0.35, 0.08, 0.02],
    [0.85, 0.28, 0.05],
    [1.0, 0.55, 0.12],
    [1.0, 0.86, 0.45],
];

/// Snap a linear-ish rgb sample to the nearest palette entry.
pub fn quantize(rgb: [f32; 3], palette: &[[f32; 3]]) -> [f32; 3] {
    let mut best = palette[0];
    let mut best_d = f32::MAX;
    for swatch in palette {
        let d = (rgb[0] - swatch[0]).powi(2)
            + (rgb[1] - swatch[1]).powi(2)
            + (rgb[2] - swatch[2]).powi(2);
        if d < best_d {
            best_d = d;
            best = *swatch;
        }
    }
    best
}

/// Bayer threshold in -0.5..0.5 for ordered dither at a pixel.
pub fn bayer4(x: u32, y: u32) -> f32 {
    const BAYER4: [f32; 16] = [
        0., 8., 2., 10., 12., 4., 14., 6., 3., 11., 1., 9., 15., 7., 13., 5.,
    ];
    (BAYER4[((y & 3) * 4 + (x & 3)) as usize] + 0.5) / 16.0 - 0.5
}
