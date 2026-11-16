//! Opening-slice shapes: well, broken wall, graves, and the Village Head.
//! Same SDF shell as the rocks. Placed by the Bevy scene, not by this crate.

use crate::extract::{voxel_shell, CpuMesh};
use crate::sdf::{sd_box, sd_cylinder_y, sd_sphere, smin, V3};

fn n3(p: V3, seed: f32) -> f32 {
    let s = (p.x * 4.1 + seed).sin() * (p.y * 3.7).cos() * (p.z * 5.3 + seed).sin();
    s * 0.5
}

pub fn well_stone() -> CpuMesh {
    let origin = V3::new(-1.7, 0.0, -1.7);
    let extent = V3::new(3.4, 1.4, 3.4);
    voxel_shell(
        |p| {
            let base = sd_cylinder_y(p.sub(V3::new(0.0, 0.55, 0.0)), 1.28, 0.52);
            let lip = sd_cylinder_y(p.sub(V3::new(0.0, 1.15, 0.0)), 1.48, 0.1);
            let bore = sd_cylinder_y(p.sub(V3::new(0.0, 0.75, 0.0)), 0.95, 0.85);
            smin(base, lip, 0.08).max(-bore) + n3(p, 2.0) * 0.04
        },
        origin,
        extent,
        [22, 12, 22],
    )
}

pub fn well_timber() -> CpuMesh {
    let origin = V3::new(-1.4, 1.05, -0.9);
    let extent = V3::new(2.8, 2.4, 1.8);
    voxel_shell(
        |p| {
            let post_l = sd_box(p.sub(V3::new(-0.95, 1.05, 0.0)), V3::new(0.1, 1.05, 0.1));
            let post_r = sd_box(p.sub(V3::new(0.95, 1.05, 0.0)), V3::new(0.1, 1.05, 0.1));
            let roof = sd_box(p.sub(V3::new(0.0, 2.15, 0.0)), V3::new(1.25, 0.08, 0.78));
            let axle = sd_cylinder_y(p.sub(V3::new(0.0, 1.7, 0.0)), 0.05, 0.08);
            // Axle should run along X; approximate with a thin box.
            let beam = sd_box(p.sub(V3::new(0.0, 1.7, 0.0)), V3::new(0.85, 0.05, 0.05));
            let bucket = sd_cylinder_y(p.sub(V3::new(0.15, 0.85, 0.15)), 0.16, 0.16);
            smin(smin(post_l, post_r, 0.02), smin(roof, smin(beam, bucket, 0.02), 0.04), 0.03)
                .min(axle)
        },
        origin,
        extent,
        [18, 16, 12],
    )
}

pub fn ruin_block(seed: f32, scale: V3) -> CpuMesh {
    let origin = V3::new(-scale.x, -scale.y, -scale.z);
    let extent = V3::new(scale.x * 2.0, scale.y * 2.0, scale.z * 2.0);
    voxel_shell(
        |p| sd_box(p, scale) + n3(p, seed) * 0.12,
        origin,
        extent,
        [8, 8, 6],
    )
}

pub fn gravestone(seed: f32) -> CpuMesh {
    let origin = V3::new(-0.32, 0.0, -0.12);
    let extent = V3::new(0.64, 1.15, 0.24);
    voxel_shell(
        |p| {
            let slab = sd_box(p.sub(V3::new(0.0, 0.5, 0.0)), V3::new(0.26, 0.5, 0.08));
            let cap = sd_sphere(p.sub(V3::new(0.0, 0.95, 0.0)), 0.22);
            smin(slab, cap, 0.06) + n3(p, seed) * 0.03
        },
        origin,
        extent,
        [6, 12, 4],
    )
}

/// Body in the Village Head's local space (parent sits at chest height).
pub fn elder_body() -> CpuMesh {
    let origin = V3::new(-0.32, -1.05, -0.22);
    let extent = V3::new(0.64, 1.35, 0.44);
    voxel_shell(
        |p| {
            let torso = sd_box(p.sub(V3::new(0.0, -0.15, 0.0)), V3::new(0.26, 0.38, 0.16));
            let leg_l = sd_box(p.sub(V3::new(-0.1, -0.72, 0.0)), V3::new(0.08, 0.32, 0.08));
            let leg_r = sd_box(p.sub(V3::new(0.1, -0.72, 0.0)), V3::new(0.08, 0.32, 0.08));
            let arm_l = sd_box(p.sub(V3::new(-0.34, -0.2, 0.0)), V3::new(0.07, 0.28, 0.07));
            let arm_r = sd_box(p.sub(V3::new(0.34, -0.2, 0.0)), V3::new(0.07, 0.28, 0.07));
            smin(smin(torso, smin(leg_l, leg_r, 0.04), 0.06), smin(arm_l, arm_r, 0.02), 0.05)
        },
        origin,
        extent,
        [10, 16, 8],
    )
}

pub fn elder_head() -> CpuMesh {
    let origin = V3::new(-0.28, 0.42, -0.28);
    let extent = V3::new(0.56, 0.56, 0.56);
    voxel_shell(
        |p| sd_sphere(p.sub(V3::new(0.0, 0.7, 0.0)), 0.2) + n3(p, 9.0) * 0.02,
        origin,
        extent,
        [8, 8, 8],
    )
}
