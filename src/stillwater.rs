//! Stillwater landmarks and presentation of the authoritative marsh snapshot.
use crate::{forge_menu::GameFlow, net::ZoneNet, player::Player};
use ba_protocol::stillwater::*;
use bevy::{
    image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
    light::{NotShadowCaster, NotShadowReceiver},
    math::Affine2,
    prelude::*,
};

pub mod lantern_post;
pub mod sable_rest;

#[derive(Component)]
pub struct MarshHud;
#[derive(Component)]
pub struct MarshPrompt;
#[derive(Component)]
pub struct Silt;
/// Dark Mod leaf cards ship as Blend; clip them so foliage is solid.
#[derive(Component)]
pub struct ClipLeaves;
/// Cliff GLBs ship a shiny collision hull; dress the rock and hide the hull.
#[derive(Component)]
pub struct ClipCliffs;
#[derive(Component)]
pub struct Actor(u32);
#[derive(Component)]
pub struct Cue(u32);
#[derive(Component)]
pub struct Parcel(usize, u8);
#[derive(Component)]
pub struct Label(usize);
#[derive(Resource)]
pub struct Paint {
    common: Handle<StandardMaterial>,
    gold: Handle<StandardMaterial>,
    red: Handle<StandardMaterial>,
    beams: [Handle<StandardMaterial>; 5],
    chime: Handle<AudioSource>,
}
#[derive(Default, Resource)]
pub struct DisplayState {
    pub(crate) inventory: bool,
    last_notice: String,
    notice_until: f32,
    filter: bool,
    paused: bool,
    deaths: std::collections::HashMap<u32, f32>,
    landed: std::collections::HashSet<(i32, i32)>,
}

fn forest_floor() -> Mesh {
    let cols = 72usize;
    let rows = 88usize;
    let (x0, x1, z0, z1) = (145., 470., -230., 240.);
    let mut positions = Vec::with_capacity((cols + 1) * (rows + 1));
    let mut uvs = Vec::with_capacity((cols + 1) * (rows + 1));
    for j in 0..=rows {
        let z = z0 + (z1 - z0) * j as f32 / rows as f32;
        for i in 0..=cols {
            let x = x0 + (x1 - x0) * i as f32 / cols as f32;
            positions.push([x, ground(x, z), z]);
            uvs.push([x * 0.09, z * 0.09]);
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
        let a = Vec3::from(positions[tri[0] as usize]);
        let b = Vec3::from(positions[tri[1] as usize]);
        let c = Vec3::from(positions[tri[2] as usize]);
        let n = (b - a).cross(c - a);
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

fn plant_forest(
    c: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    assets: &AssetServer,
) {
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
        Mesh3d(meshes.add(forest_floor())),
        MeshMaterial3d(mats.add(StandardMaterial {
            base_color: Color::srgb(0.55, 0.62, 0.42),
            base_color_texture: Some(grass),
            perceptual_roughness: 0.96,
            cull_mode: None,
            ..default()
        })),
        Transform::IDENTITY,
    ));
    // Alpha-masked Dark Mod keepers. Birches repeat so they stay the majority.
    const GROVE: &[(&str, f32)] = &[
        ("birch_02", 0.034),
        ("birch_03", 0.036),
        ("birch_04", 0.038),
        ("birch_05", 0.034),
        ("birch_06", 0.040),
        ("birch_03", 0.035),
        ("birch_04", 0.037),
        ("birch_05", 0.033),
        ("tree_dm", 0.038),
        ("tree_dm01", 0.036),
        ("tree_dm02", 0.036),
        ("tree_dm_small01", 0.044),
        ("tree_dm_small02", 0.044),
        ("tree_02", 0.032),
        ("tree_02_large", 0.026),
        ("tree_02_small", 0.038),
        ("tree_04_large", 0.026),
        ("tree_04_small", 0.036),
        ("tree_05_large", 0.026),
        ("tree_05_small", 0.036),
        ("tree_06_large", 0.026),
        ("tree_06_small", 0.036),
        ("tree_old_01", 0.030),
        ("tree_old_02", 0.030),
        ("spruce_01a", 0.032),
        ("spruce_01c", 0.032),
        ("tree_07_young", 0.044),
    ];
    for gx in 0..24 {
        for gz in 0..32 {
            let i = gx * 32 + gz;
            let jitter = {
                let mut h = (i as u32).wrapping_mul(0x9e3779b9);
                h ^= h >> 16;
                h = h.wrapping_mul(0x7feb352d);
                (h >> 8) as f32 / 16777216.
            };
            let sway = {
                let mut h = ((i + 91) as u32).wrapping_mul(0x85ebca6b);
                h ^= h >> 13;
                (h >> 8) as f32 / 16777216.
            };
            if jitter < 0.05 {
                continue;
            }
            let x = 168. + gx as f32 * 11.2 + (jitter - 0.5) * 5.2;
            let z = -172. + gz as f32 * 12. + (sway - 0.5) * 5.;
            if (x - POST.x).hypot(z - POST.z) < 14.
                || (x - GATE.x).hypot(z - GATE.z) < 12.
                || (x - SHRINE.x).hypot(z - SHRINE.z) < 10.
                || (x - 300.).abs() < 5.5
                || x < 210.
            {
                continue;
            }
            let spin = {
                let mut h = ((i + 211) as u32).wrapping_mul(0xc2b2ae35);
                h ^= h >> 16;
                (h >> 8) as f32 / 16777216.
            };
            let (name, base) = GROVE[(i * 17 + 3) % GROVE.len()];
            let scale = base * (0.82 + jitter * 0.38);
            let yaw = spin * std::f32::consts::TAU;
            let tilt = (sway - 0.5) * 0.14;
            let roll = (jitter - 0.5) * 0.11;
            c.spawn((
                Silt,
                ClipLeaves,
                WorldAssetRoot(
                    assets.load(format!("darkmod/trees/{name}.glb#Scene0")),
                ),
                Transform::from_xyz(x, ground(x, z) - scale * 16., z)
                    .with_scale(Vec3::new(
                        scale * if spin > 0.5 { -1. } else { 1. },
                        scale,
                        scale,
                    ))
                    .with_rotation(Quat::from_euler(EulerRot::YXZ, yaw, tilt, roll)),
            ));
        }
    }
    ring_cliffs(c, assets);
}

fn ring_cliffs(c: &mut Commands, assets: &AssetServer) {
    const ROCKS: &[&str] = &[
        "cliffrock_01",
        "cliffrock_02",
        "cliffrock_03",
        "cliffrock_04",
        "cliffrock_05",
        "cliffrock_06",
        "cliffrock_07",
        "cliffrock_08",
    ];
    let mut k = 0usize;
    // Outcrops on the outer bowl. These GLBs are huge; keep them small and far
    // so the forest is not inside a hollow cliff.
    for i in 0..22 {
        let n = {
            let mut h = (i as u32).wrapping_mul(0x9e3779b9);
            h ^= h >> 16;
            (h >> 8) as f32 / 16777216.
        };
        let a = i as f32 / 22. * std::f32::consts::TAU + (n - 0.5) * 0.2;
        let radius = 158. + n * 16.;
        let x = 300. + a.cos() * radius;
        let z = 5. + a.sin() * radius;
        if x < 222. {
            continue;
        }
        let scale = 0.038 + n * 0.034;
        c.spawn((
            Silt,
            ClipCliffs,
            WorldAssetRoot(
                assets.load(format!("darkmod/rocks/{}.glb#Scene0", ROCKS[k % ROCKS.len()])),
            ),
            Transform::from_xyz(x, ground(x, z) - 1.1, z)
                .with_scale(Vec3::splat(scale))
                .with_rotation(Quat::from_rotation_y(a + std::f32::consts::PI)),
            NotShadowCaster,
            NotShadowReceiver,
        ));
        k += 1;
    }
}

pub fn silt_visibility(
    player: Query<&Transform, With<Player>>,
    mut silt: Query<&mut Visibility, With<Silt>>,
) {
    let x = player.single().map(|t| t.translation.x).unwrap_or(0.);
    let vis = if marsh(x) || rest(x) {
        Visibility::Visible
    } else {
        Visibility::Hidden
    };
    for mut v in &mut silt {
        *v = vis;
    }
}

/// Dark Mod GLBs ship metallic 0 / roughness 0, which reads as chrome.
pub fn fix_zero_roughness(mut materials: ResMut<Assets<StandardMaterial>>) {
    for (_, mat) in materials.iter_mut() {
        if mat.metallic <= 0.02 && mat.perceptual_roughness <= 0.08 {
            mat.metallic = 0.0;
            mat.perceptual_roughness = 0.9;
        }
    }
}

fn texture_is_cutout(img: &Image) -> bool {
    let Some(data) = img.data.as_ref() else {
        return false;
    };
    if data.len() < 4 {
        return false;
    }
    let stride = if data.len() % 4 == 0 { 4 } else { 1 };
    if stride != 4 {
        return false;
    }
    let mut cut = 0u32;
    let mut n = 0u32;
    for px in data.chunks_exact(4).step_by(19) {
        n += 1;
        if px[3] < 115 {
            cut += 1;
        }
    }
    n > 0 && cut as f32 / n as f32 > 0.12
}

fn hull_name(name: &str) -> bool {
    ["collision", "clip", "shadow", "nodraw", "playerclip", "monsterclip"]
        .iter()
        .any(|k| name.contains(k))
}

pub fn dress_darkmod(
    roots: Query<Entity, Or<(With<ClipLeaves>, With<ClipCliffs>)>>,
    children: Query<&Children>,
    mesh_mats: Query<&MeshMaterial3d<StandardMaterial>>,
    names: Query<&Name>,
    images: Res<Assets<Image>>,
    assets: Res<AssetServer>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut commands: Commands,
    mut seen: Local<std::collections::HashSet<Entity>>,
) {
    let masonry = assets.load("darkmod/stone/castle_blocks.jpg");
    for root in &roots {
        let mut stack = vec![root];
        let mut ents = Vec::new();
        while let Some(entity) = stack.pop() {
            if let Ok(ch) = children.get(entity) {
                stack.extend(ch.iter());
            }
            ents.push(entity);
        }
        let mut textured = false;
        let mut hulls = Vec::new();
        let mut bare = Vec::new();
        let mut paint = Vec::new();
        for &entity in &ents {
            commands
                .entity(entity)
                .insert((NotShadowCaster, NotShadowReceiver));
            if seen.contains(&entity) {
                if mesh_mats.get(entity).ok().and_then(|h| materials.get(&h.0)).is_some_and(|m| {
                    m.base_color_texture.is_some()
                }) {
                    textured = true;
                }
                continue;
            }
            let named = names
                .get(entity)
                .map(|n| n.as_str().to_lowercase())
                .unwrap_or_default();
            let Ok(handle) = mesh_mats.get(entity) else {
                continue;
            };
            let Some(mat) = materials.get(&handle.0) else {
                continue;
            };
            let tex = mat.base_color_texture.clone();
            let path = tex
                .as_ref()
                .and_then(|t| assets.get_path(t))
                .map(|p| p.to_string().to_lowercase())
                .unwrap_or_default();
            if hull_name(&named) || hull_name(&path) {
                hulls.push(entity);
                continue;
            }
            if tex.is_none() {
                bare.push(entity);
                continue;
            }
            textured = true;
            paint.push((entity, handle.0.clone(), path, tex.unwrap()));
        }
        for entity in hulls {
            commands.entity(entity).insert(Visibility::Hidden);
            seen.insert(entity);
        }
        if textured {
            for entity in bare {
                commands.entity(entity).insert(Visibility::Hidden);
                seen.insert(entity);
            }
        } else {
            for entity in bare {
                let Ok(handle) = mesh_mats.get(entity) else {
                    continue;
                };
                let Some(mut mat) = materials.get_mut(&handle.0) else {
                    continue;
                };
                mat.base_color_texture = Some(masonry.clone());
                mat.base_color = Color::srgb(0.84, 0.82, 0.78);
                mat.alpha_mode = AlphaMode::Opaque;
                mat.metallic = 0.0;
                mat.perceptual_roughness = 0.93;
                mat.occlusion_texture = None;
                mat.cull_mode = None;
                mat.double_sided = true;
                mat.unlit = false;
                seen.insert(entity);
            }
        }
        for (entity, handle, path, tex) in paint {
            let Some(mut mat) = materials.get_mut(&handle) else {
                continue;
            };
            mat.metallic = 0.0;
            mat.occlusion_texture = None;
            mat.cull_mode = None;
            mat.double_sided = true;
            if mat.perceptual_roughness <= 0.08 {
                mat.perceptual_roughness = 0.9;
            }
            let bark = path.contains("stem")
                || path.contains("trunk")
                || path.contains("bark")
                || path.contains("wood");
            let mut pending = false;
            let foliage = !bark
                && (path.contains("leaf")
                    || path.contains("foliage")
                    || path.contains("branch")
                    || path.contains("needl")
                    || match images.get(&tex) {
                        Some(img) => texture_is_cutout(img),
                        None => {
                            pending = true;
                            false
                        }
                    });
            if foliage {
                mat.alpha_mode = AlphaMode::Mask(0.45);
            } else {
                mat.alpha_mode = AlphaMode::Opaque;
            }
            if !pending {
                seen.insert(entity);
            }
        }
    }
}

fn pigment(m: &mut Assets<StandardMaterial>, rgb: [f32; 3], glow: f32) -> Handle<StandardMaterial> {
    let [r, g, b] = rgb;
    m.add(StandardMaterial {
        base_color: Color::srgb(r, g, b),
        emissive: LinearRgba::rgb(r * glow, g * glow, b * glow),
        perceptual_roughness: 0.85,
        ..default()
    })
}

fn repeat_tex(assets: &AssetServer, path: &'static str) -> Handle<Image> {
    assets
        .load_builder()
        .with_settings(|s: &mut ImageLoaderSettings| {
            s.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::Repeat,
                address_mode_v: ImageAddressMode::Repeat,
                ..ImageSamplerDescriptor::linear()
            });
        })
        .load(path)
}

fn world_uvs(mesh: &mut Mesh, scale: f32) {
    let Some(uvs) = mesh.attribute(Mesh::ATTRIBUTE_POSITION).and_then(|a| {
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
                        [p[0] * scale, p[1] * scale]
                    } else {
                        [p[2] * scale, p[1] * scale]
                    }
                })
                .collect::<Vec<[f32; 2]>>()
        })
    }) else {
        return;
    };
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
}
pub fn setup(
    mut c: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    assets: Res<AssetServer>,
    scans: Res<crate::megascans::MegascansRoot>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
) {
    let cube = meshes.add(Cuboid::from_size(Vec3::ONE));
    let round = meshes.add(Sphere::new(1.).mesh().uv(10, 7));

    let mud = pigment(&mut mats, [0.32, 0.29, 0.20], 0.12);
    let reed = pigment(&mut mats, [0.28, 0.39, 0.21], 0.12);
    let wood = pigment(&mut mats, [0.27, 0.18, 0.13], 0.15);
    let bone = mats.add(StandardMaterial {
        base_color: Color::srgb(0.94, 0.88, 0.74),
        base_color_texture: Some(repeat_tex(&assets, "darkmod/stone/bone.jpg")),
        perceptual_roughness: 0.88,
        uv_transform: Affine2::from_scale(Vec2::splat(1.8)),
        ..default()
    });
    let blue = pigment(&mut mats, [0.20, 0.75, 0.92], 2.);
    let gold = pigment(&mut mats, [1., 0.56, 0.12], 2.);
    let white = pigment(&mut mats, [0.94, 0.94, 0.87], 1.5);
    let red = pigment(&mut mats, [1., 0.12, 0.12], 2.);
    let rare = pigment(&mut mats, [0.24, 0.68, 1.], 2.5);
    let bank = scans.surface("bank", &assets, &mut mats, &mud);
    let masonry = scans.surface("gate", &assets, &mut mats, &mud);
    plant_forest(&mut c, &mut meshes, &mut mats, &assets);
    let mut prop =
        |mesh: &Handle<Mesh>, mat: &Handle<StandardMaterial>, x, y, z, w, h, d, roll: f32| {
            c.spawn((
                Silt,
                Mesh3d(mesh.clone()),
                MeshMaterial3d(mat.clone()),
                Transform::from_xyz(x, y, z)
                    .with_scale(Vec3::new(w, h, d))
                    .with_rotation(Quat::from_rotation_z(roll)),
            ));
        };

    // Sable Rest ground is the bowl mesh in sable_rest::build, not a flat pad.
    for pt in [
        POST,
        SHRINE,
        BLIND,
        at(300., 0.),
        at(309., -42.),
        at(301., -61.),
        GATE,
    ] {
        prop(&round, &bank, pt.x, -0.13, pt.z, 6., 0.23, 5., 0.);
        if pt.z == POST.z {
            continue;
        }
        prop(
            &cube,
            &wood,
            pt.x + 2.,
            1.65,
            pt.z + 1.,
            0.22,
            3.3,
            0.22,
            0.04,
        );
        prop(
            &cube,
            &blue,
            pt.x + 2.,
            3.42,
            pt.z + 1.,
            0.45,
            0.5,
            0.45,
            0.,
        );
    }
    for i in 0..145 {
        let x = 217. + ((i * 67 + 13) % 166) as f32;
        let z = -103. + ((i * 83 + 17) % 205) as f32;
        for j in 0..3 {
            prop(
                &cube,
                &reed,
                x + j as f32 * 0.23,
                0.46,
                z + j as f32 * 0.12,
                0.095,
                0.8 + 0.18 * j as f32,
                0.1,
                0.18 * j as f32 - 0.18,
            );
        }
    }
    struct Ossuary {
        cube: Mesh,
        round: Mesh,
        batch: Option<Mesh>,
    }
    impl Ossuary {
        fn add(&mut self, sphere: bool, x: f32, y: f32, z: f32, w: f32, h: f32, d: f32, roll: f32) {
            let mut mesh = (if sphere { &self.round } else { &self.cube })
                .clone()
                .transformed_by(
                    Transform::from_xyz(x, y, z)
                        .with_scale(Vec3::new(w, h, d))
                        .with_rotation(Quat::from_rotation_z(roll)),
                );
            world_uvs(&mut mesh, 0.22);
            match &mut self.batch {
                Some(b) => {
                    let _ = b.merge(&mesh);
                }
                None => self.batch = Some(mesh),
            }
        }
    }
    let mut ossuary = Ossuary {
        cube: Cuboid::from_size(Vec3::ONE).into(),
        round: Sphere::new(1.).mesh().uv(10, 7),
        batch: None,
    };
    // Each rib is a tapered arc that rises from either bank of the spine.
    for i in 0..9 {
        let z = 8. - i as f32 * 2.05;
        let height = 7. + (i as f32 * 0.4).sin() * 3.;
        ossuary.add(true, 300., height, z, 0.75, 0.75, 1.5, 0.);
        for side in [-1., 1.] {
            for n in 0..6 {
                let pt = |j: f32| {
                    let a = j / 6. * 1.65;
                    Vec2::new(300. + side * 6. * a.sin(), height * a.cos())
                };
                let a = pt(n as f32);
                let b = pt(n as f32 + 1.);
                let mid = (a + b) / 2.;
                let v = b - a;
                ossuary.add(
                    false,
                    mid.x,
                    mid.y,
                    z,
                    0.62,
                    v.length() + 0.15,
                    0.6,
                    -v.x.atan2(v.y),
                );
            }
        }
    }
    ossuary.add(true, 300., 7.2, -11., 2.8, 1.8, 3.5, 0.);
    for side in [-1., 1.] {
        ossuary.add(false, 300. + side * 3., 8.3, -11., 0.65, 4.5, 0.65, -side * 1.0);
    }
    let ossuary_mesh = ossuary.batch.map(|mesh| meshes.add(mesh));
    if let Some(ref handle) = ossuary_mesh {
        prop(handle, &bone, 0., 0., 0., 1., 1., 1., 0.);
    }
    for z in [-98., -96.] {
        for x in [297., 303.] {
            prop(&cube, &masonry, x, 2.1, z, 1.1, 4.4, 1.2, 0.);
        }
        prop(&cube, &bone, 300., 4.5, z, 7.2, 0.9, 1.3, 0.);
    }
    for k in 0..13 {
        prop(
            &cube,
            &wood,
            304.,
            0.05,
            -45. - k as f32,
            2.4,
            0.12,
            0.88,
            0.,
        );
    }
    prop(&round, &gold, SHRINE.x + 1.1, 0.2, SHRINE.z, 0.45, 0.6, 0.45, 0.);
    for (pt, mat) in NODES {
        prop(
            &round,
            if mat == "Stillwater Essence" {
                &blue
            } else if mat == "Cold Iron" {
                &white
            } else {
                &mud
            },
            pt.x,
            0.27,
            pt.z,
            0.65,
            0.42,
            0.52,
            0.,
        );
    }
    // The ferryman's boat and the dry causeway at the gate.
    let (x, z) = (GATE.x + 1.4, GATE.z - 1.1);
    prop(&round, &wood, x, 0.20, z, 0.85, 0.34, 2.35, 0.);
    prop(&cube, &mud, x, 0.48, z, 1.0, 0.12, 0.35, 0.);
    for i in 0..8 {
        prop(
            &round,
            &bone,
            300.,
            -0.05,
            -97. - i as f32,
            1.3,
            0.12,
            0.55,
            0.,
        );
    }
    for pt in HERONS {
        prop(&round, &bone, pt.x, 0.8, pt.z, 0.25, 0.26, 0.46, 0.);
        prop(
            &cube,
            &bone,
            pt.x,
            1.13,
            pt.z - 0.25,
            0.12,
            0.72,
            0.13,
            0.12,
        );
        prop(&cube, &gold, pt.x, 1.45, pt.z - 0.43, 0.1, 0.1, 0.44, 0.);
    }
    // Blue twin lanterns distinguish travel points from the single hub lanterns.
    for site in [LANTERN_ROAD, GATE] {
        for side in [-1., 1.] {
            let x = site.x + side * 1.65;
            prop(&cube, &wood, x, 1.15, site.z, 0.44, 2.3, 0.44, 0.);
            prop(&cube, &blue, x, 2.5, site.z, 0.48, 0.5, 0.48, 0.);
        }
    }
    // Shrine urn, hunter's lean-to, and rest table — generative, not Kenney.
    prop(&round, &mud, SHRINE.x, 0.22, SHRINE.z, 0.32, 0.18, 0.32, 0.);
    prop(&round, &wood, SHRINE.x, 0.48, SHRINE.z, 0.28, 0.34, 0.28, 0.);
    prop(&round, &gold, SHRINE.x, 0.72, SHRINE.z, 0.16, 0.12, 0.16, 0.);
    for (dx, dz, yaw) in [(0., 0.9, 0.), (0.85, 0., 1.2), (-0.7, 0.4, -0.7)] {
        prop(
            &cube,
            &wood,
            BLIND.x + dx,
            0.85,
            BLIND.z + dz,
            1.35,
            1.55,
            0.12,
            yaw,
        );
    }
    prop(&cube, &wood, BLIND.x, 1.55, BLIND.z, 1.7, 0.1, 1.4, 0.18);
    prop(&cube, &wood, REST.x - 2.8, 0.72, REST.z - 1.1, 1.45, 0.08, 0.72, 0.);
    for side in [-0.58, 0.58] {
        prop(
            &cube,
            &wood,
            REST.x - 2.8 + side,
            0.36,
            REST.z - 1.1 + 0.24,
            0.08,
            0.72,
            0.08,
            0.,
        );
        prop(
            &cube,
            &wood,
            REST.x - 2.8 + side,
            0.36,
            REST.z - 1.1 - 0.24,
            0.08,
            0.72,
            0.08,
            0.,
        );
    }
    for (x, z) in [
        (POST.x - 1.8, POST.z - 1.2),
        (SHRINE.x + 1.3, SHRINE.z + 0.7),
        (GATE.x + 1.5, GATE.z),
        (REST.x - 0.9, REST.z - 1.9),
    ] {
        c.spawn((
            Silt,
            WorldAssetRoot(assets.load("models/hooded_figure.glb#Scene0")),
            Transform::from_xyz(x, 0., z),
        ));
    }
    for (slot, sites) in [
        (
            "terrain",
            &[(294., 88.), (282., 64.), (249., -32.), (306., -46.)][..],
        ),
        ("rock", &[(282., 65.), (253., -29.), (303., -91.)][..]),
    ] {
        if let Some(scene) = scans.scene(slot, &assets) {
            for (i, &(x, z)) in sites.iter().enumerate() {
                c.spawn((
                    Silt,
                    WorldAssetRoot(scene.0.clone()),
                    Transform::from_xyz(x, ground(x, z) - 0.12, z)
                        .with_rotation(Quat::from_rotation_y(i as f32 * 1.7)),
                ));
            }
        }
    }
    for (x, z) in [
        (POST.x, POST.z),
        (SHRINE.x, SHRINE.z),
        (GATE.x, GATE.z),
        (REST.x, REST.z),
    ] {
        c.spawn((
            Silt,
            PointLight {
                color: Color::srgb(0.82, 0.90, 1.0),
                intensity: 52000.,
                range: 13.,
                ..default()
            },
            Transform::from_xyz(x + 1., 3.25, z),
        ));
    }
    for (i, (x, z)) in [
        (287., 84.),
        (306., 81.),
        (275., 63.),
        (318., 51.),
        (329., 17.),
        (258., -29.),
        (289., -55.),
        (310., -84.),
    ]
    .into_iter()
    .enumerate()
    {
        let birch = ["birch_03", "birch_04", "birch_05", "birch_06"][i % 4];
        c.spawn((
            Silt,
            ClipLeaves,
            WorldAssetRoot(assets.load(format!("darkmod/trees/{birch}.glb#Scene0"))),
            Transform::from_xyz(x, ground(x, z) - 0.13, z)
                .with_rotation(Quat::from_rotation_y(i as f32))
                .with_scale(Vec3::splat(0.034 + (i % 3) as f32 * 0.006)),
        ));
    }
    // Authored Idle_Sword, Walk, Sword_Slash and Death in the shipped hooded figure.
    let clips = [9, 22, 21, 0].map(|index| {
        assets.load(GltfAssetLabel::Animation(index).from_asset("models/hooded_figure.glb"))
    });
    let (graph, nodes) = AnimationGraph::from_clips(clips);
    c.insert_resource(EddaClips {
        graph: graphs.add(graph),
        nodes,
    });
    for foe in FOES.iter() {
        let mut e = c.spawn((
            Actor(foe.id),
            Transform::from_xyz(foe.x, 0., foe.z),
            Visibility::Hidden,
        ));
        if foe.id == EDDA {
            e.insert(WorldAssetRoot(
                assets.load("models/hooded_figure.glb#Scene0"),
            ));
        } else {
            e.with_children(|root| {
                let (length, girth, height, color) = match foe.id {
                    OX => (2.3, 1.2, 1.7, &bone),
                    70000..=70001 | 70012..=70013 => (0.92, 0.38, 0.62, &wood),
                    70002..=70004 => (0.75, 0.85, 0.48, &reed),
                    70008..=70009 => (1.55, 0.23, 0.24, &bone),
                    _ => (0.62, 0.33, 0.4, &bone),
                };
                let mut part = |mesh: &Handle<Mesh>,
                                mat: &Handle<StandardMaterial>,
                                x,
                                y,
                                z,
                                w,
                                h,
                                d,
                                angle| {
                    root.spawn((
                        Mesh3d(mesh.clone()),
                        MeshMaterial3d(mat.clone()),
                        Transform::from_xyz(x, y, z)
                            .with_scale(Vec3::new(w, h, d))
                            .with_rotation(Quat::from_rotation_z(angle)),
                    ));
                };
                part(
                    &round,
                    color,
                    0.,
                    height * 0.8,
                    0.,
                    girth,
                    height * 0.58,
                    length,
                    0.,
                );
                part(
                    &round,
                    color,
                    0.,
                    height,
                    length * 0.84,
                    girth * 0.7,
                    height * 0.44,
                    length * 0.47,
                    0.,
                );
                for side in [-1., 1.] {
                    part(
                        &round,
                        &blue,
                        side * girth * 0.42,
                        height * 1.12,
                        length * 1.15,
                        0.09,
                        0.08,
                        0.1,
                        0.,
                    );
                    if foe.id == OX {
                        part(
                            &cube,
                            &bone,
                            side * 1.05,
                            2.1,
                            2.2,
                            0.22,
                            1.6,
                            0.3,
                            -side * 0.85,
                        );
                    }
                    if !matches!(foe.id, 70008..=70009) {
                        for z in [-0.6, 0.64] {
                            part(
                                &cube,
                                color,
                                side * girth * 0.72,
                                height * 0.35,
                                z * length,
                                0.17,
                                height * 0.7,
                                0.25,
                                side * 0.22,
                            );
                        }
                    }
                }
            });
        }
        c.spawn((Cue(foe.id), Transform::default(), Visibility::Hidden))
            .with_children(|root| {
                for i in 0..20 {
                    let a = i as f32 * std::f32::consts::TAU / 20.;
                    root.spawn((
                        Mesh3d(cube.clone()),
                        MeshMaterial3d(white.clone()),
                        Transform::from_xyz(a.cos(), 0., a.sin())
                            .with_scale(Vec3::new(0.07, 0.12, 0.07)),
                    ));
                }
            });
    }
    for i in 0..16 {
        for part in 0..3 {
            c.spawn((
                Parcel(i, part),
                Mesh3d(if part == 2 {
                    round.clone()
                } else {
                    cube.clone()
                }),
                MeshMaterial3d(gold.clone()),
                Transform::default(),
                Visibility::Hidden,
            ));
        }
        c.spawn((
            Label(i),
            Node {
                position_type: PositionType::Absolute,
                ..default()
            },
            Text::new(""),
            TextFont::from_font_size(14.),
            TextColor(Color::WHITE),
            Visibility::Hidden,
        ));
    }
    c.spawn((
        MarshHud,
        Text::new(""),
        TextFont::from_font_size(15.),
        TextColor(Color::srgb(0.96, 0.89, 0.74)),
        BackgroundColor(Color::srgba(0.066, 0.065, 0.085, 0.83)),
        BorderColor::all(Color::srgb(0.55, 0.45, 0.33)),
        Node {
            position_type: PositionType::Absolute,
            top: px(20),
            right: px(18),
            max_width: px(372),
            border: UiRect::right(px(2)),
            padding: UiRect::axes(px(12), px(8)),
            ..default()
        },
        Visibility::Hidden,
    ));
    c.spawn((
        MarshPrompt,
        Text::new(""),
        TextFont::from_font_size(16.),
        TextLayout::justify(Justify::Center),
        TextColor(Color::srgb(0.98, 0.91, 0.77)),
        BackgroundColor(Color::srgba(0.065, 0.061, 0.075, 0.83)),
        BorderColor::all(Color::srgba(0.65, 0.52, 0.36, 0.8)),
        Node {
            position_type: PositionType::Absolute,
            bottom: px(23),
            left: percent(32.),
            width: percent(36.),
            padding: UiRect::axes(px(10), px(8)),
            border: UiRect::bottom(px(1)),
            ..default()
        },
        Visibility::Hidden,
    ));
    c.spawn((
        ImpactText,
        Text::new(""),
        TextFont::from_font_size(26.),
        TextColor(Color::WHITE),
        Node {
            position_type: PositionType::Absolute,
            left: percent(49),
            top: percent(45),
            ..default()
        },
        Visibility::Hidden,
    ));
    c.insert_resource(Paint {
        common: white.clone(),
        gold,
        red,
        beams: [
            white,
            rare,
            pigment(&mut mats, [1., 0.5, 0.08], 3.),
            pigment(&mut mats, [0.69, 0.28, 1.], 3.),
            pigment(&mut mats, [1., 0.15, 0.3], 4.),
        ],
        chime: assets.load("audio/cinder-land.ogg"),
    });
}

pub fn impact_time(
    net: Res<ZoneNet>,
    mut clock: ResMut<Time<Virtual>>,
    mut display: ResMut<DisplayState>,
) {
    let stop = net.online()
        && net
            .impact
            .is_some_and(|(t, big, _)| t.elapsed().as_millis() < if big { 110 } else { 50 });
    if stop && !clock.is_paused() {
        clock.pause();
        display.paused = true;
    }
    if !stop && display.paused {
        clock.unpause();
        display.paused = false;
    }
}
pub fn camera_feedback(
    net: Res<ZoneNet>,
    mut camera: Query<&mut Transform, With<crate::camera_rig::PlayerCamera>>,
) {
    if !net.online() {
        return;
    }
    let Ok(mut tf) = camera.single_mut() else {
        return;
    };
    let age = net
        .impact
        .map(|(t, _, hurt)| (t.elapsed().as_secs_f32(), hurt));
    let tilt = if net.marsh.rolling {
        -0.28
    } else if let Some((dt, hurt)) = age {
        let sign = if hurt { -1. } else { 0.45 };
        sign * (1. - dt / 0.25).clamp(0., 1.) * 0.08
    } else {
        0.
    };
    let (pitch, _, _) = tf.rotation.to_euler(EulerRot::XYZ);
    tf.rotation = Quat::from_rotation_x(pitch) * Quat::from_rotation_z(tilt);
}

pub fn animate(
    net: Res<ZoneNet>,
    flow: Res<GameFlow>,
    time: Res<Time>,
    paint: Res<Paint>,
    mut display: ResMut<DisplayState>,
    mut actors: Query<(&Actor, &mut Transform, &mut Visibility), Without<Cue>>,
    mut cues: Query<(&Cue, &Children, &mut Transform, &mut Visibility), Without<Actor>>,
    mut materials: Query<&mut MeshMaterial3d<StandardMaterial>>,
) {
    let active = net.online() && !flow.in_menu() && marsh(net.region_x);
    for (actor, mut tf, mut vis) in &mut actors {
        let Some(e) = net.marsh.enemies.iter().find(|e| active && e.id == actor.0) else {
            *vis = Visibility::Hidden;
            continue;
        };
        let elapsed = time.elapsed_secs();
        let death = if e.hp == 0 {
            elapsed - *display.deaths.entry(e.id).or_insert(elapsed)
        } else {
            display.deaths.remove(&e.id);
            0.
        };
        *vis = if death > 0.65 {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
        let collapse = (death / 0.65).clamp(0., 1.);
        let crouch = e.windup * 0.32;
        *tf = Transform::from_xyz(
            e.x,
            ground(e.x, e.z)
                + if e.submerged { -0.45 } else { 0.03 }
                + (elapsed * 3. + e.x).sin().abs() * 0.045
                - collapse * 0.4,
            e.z,
        )
        .with_rotation(
            Quat::from_rotation_y(e.yaw) * Quat::from_rotation_x(-0.32 * e.windup + collapse * 0.9),
        )
        .with_scale(Vec3::new(
            1. + crouch,
            1. - crouch - collapse * 0.75,
            1. + crouch,
        ));
    }
    for (cue, children, mut tf, mut vis) in &mut cues {
        let Some(e) = net
            .marsh
            .enemies
            .iter()
            .find(|e| active && e.id == cue.0 && e.windup > 0. && e.hp > 0)
        else {
            *vis = Visibility::Hidden;
            continue;
        };
        *vis = Visibility::Visible;
        let (sin, cos) = e.yaw.sin_cos();
        let offset = if e.line { e.radius * 0.5 } else { 0. };
        *tf = Transform::from_xyz(e.x + sin * offset, 0.16, e.z + cos * offset)
            .with_rotation(Quat::from_rotation_y(e.yaw))
            .with_scale(if e.line {
                Vec3::new(1.2, 1. + e.windup * 2., offset)
            } else {
                Vec3::new(e.radius, 1. + e.windup * 2., e.radius)
            });
        let color = match e.tell {
            Tell::Normal => &paint.common,
            Tell::Heavy => &paint.gold,
            Tell::Unblockable => &paint.red,
        };
        for child in children.iter() {
            if let Ok(mut mat) = materials.get_mut(child) {
                if mat.0 != *color {
                    mat.0 = color.clone();
                }
            }
        }
    }
}

pub fn loot(
    mut c: Commands,
    net: Res<ZoneNet>,
    paint: Res<Paint>,
    mut display: ResMut<DisplayState>,
    flow: Res<GameFlow>,
    time: Res<Time>,
    cam: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    windows: Query<&Window>,
    mut parcels: Query<
        (
            &Parcel,
            &mut Transform,
            &mut MeshMaterial3d<StandardMaterial>,
            &mut Visibility,
        ),
        Without<Label>,
    >,
    mut labels: Query<
        (
            &Label,
            &mut Node,
            &mut Text,
            &mut TextColor,
            &mut Visibility,
        ),
        Without<Parcel>,
    >,
) {
    let active = net.online() && !flow.in_menu();
    let t = time.elapsed_secs();
    let visible = |i: usize| {
        net.marsh
            .drops
            .get(i)
            .filter(|d| active && (!display.filter || d.rare) && d.age >= 0.65)
    };
    for (parcel, mut tf, mut mat, mut vis) in &mut parcels {
        let Some(d) = visible(parcel.0) else {
            *vis = Visibility::Hidden;
            continue;
        };
        let reveal = ((d.age - 0.65) / 0.3).clamp(0., 1.);
        let size = match parcel.1 {
            0 => Vec3::splat(0.34),
            1 if d.rare => Vec3::new(0.16, (8. + 2. * d.rarity() as usize as f32) * reveal, 0.16),
            2 if d.age < 1.1 => Vec3::new(reveal, 0.13, reveal),
            _ => {
                *vis = Visibility::Hidden;
                continue;
            }
        };
        let y = if parcel.1 == 1 {
            size.y / 2.
        } else if parcel.1 == 2 {
            0.12
        } else {
            0.45 + (t * 3.).sin() * 0.07
        };
        *tf = Transform::from_xyz(d.x, y, d.z)
            .with_rotation(Quat::from_rotation_y(t * 1.8))
            .with_scale(size);
        *vis = Visibility::Visible;
        mat.0 = paint.beams[d.rarity() as usize].clone();
    }
    for (label, mut node, mut text, mut color, mut vis) in &mut labels {
        *vis = Visibility::Hidden;
        let Some(d) = visible(label.0) else { continue };
        let (Ok((cam, xf)), Ok(w)) = (cam.single(), windows.single()) else {
            continue;
        };
        let Ok(pt) = cam.world_to_viewport(xf, Vec3::new(d.x, 1.1, d.z)) else {
            continue;
        };
        let Some(size) = cam.logical_viewport_size() else {
            continue;
        };
        if pt.x < 0. || pt.y < 0. || pt.x > size.x || pt.y > size.y {
            continue;
        }
        node.left = px(pt.x / size.x * w.width());
        node.top = px(pt.y / size.y * w.height());
        **text = d
            .items
            .iter()
            .map(|s| s.trim_start_matches("ashworks:").replace('_', " "))
            .collect::<Vec<_>>()
            .join(" · ");
        color.0 = match d.rarity() {
            Rarity::Common => Color::WHITE,
            Rarity::Rare => Color::srgb(0.42, 0.83, 1.),
            Rarity::Legendary => Color::srgb(1., 0.65, 0.23),
            Rarity::Ancient => Color::srgb(0.83, 0.58, 1.),
            Rarity::Primal => Color::srgb(1., 0.34, 0.47),
        };
        *vis = Visibility::Visible;
    }
    let mut seen = std::collections::HashSet::new();
    for d in &net.marsh.drops {
        if active && d.age >= 0.65 {
            let key = (d.x.to_bits() as i32, d.z.to_bits() as i32);
            seen.insert(key);
            if d.age < 1.3 && !display.landed.contains(&key) {
                c.spawn((
                    AudioPlayer::new(paint.chime.clone()),
                    PlaybackSettings::DESPAWN,
                ));
            }
        }
    }
    display.landed = seen;
}

pub fn journal(
    net: Res<ZoneNet>,
    flow: Res<GameFlow>,
    keys: Res<ButtonInput<KeyCode>>,
    mut display: ResMut<DisplayState>,
    player: Query<&Transform, With<Player>>,
    doors: Query<&sable_rest::InnDoor>,
    time: Res<Time>,
    mut prompt: Query<(&mut Text, &mut Visibility), (With<MarshPrompt>, Without<MarshHud>)>,
    mut hud: Query<(&mut Text, &mut Visibility, &mut Node), With<MarshHud>>,
) {
    let Ok((mut text, mut vis, mut node)) = hud.single_mut() else {
        return;
    };
    let Ok((mut prompt_text, mut prompt_vis)) = prompt.single_mut() else {
        return;
    };
    if !net.online() || flow.in_menu() {
        *prompt_vis = Visibility::Hidden;
        *vis = Visibility::Hidden;
        return;
    }
    if keys.just_pressed(KeyCode::KeyI) {
        display.inventory = !display.inventory
    }
    if keys.just_pressed(KeyCode::KeyL) {
        display.filter = !display.filter
    }
    let m = &net.marsh;
    // Leave room for the Ashworks order and its east-road directions.
    let top = px(if marsh(net.region_x) || rest(net.region_x) {
        20.
    } else {
        175.
    });
    if node.top != top {
        node.top = top;
    }
    if !marsh(net.region_x) && !rest(net.region_x) && m.drops.is_empty() && !display.inventory {
        *vis = Visibility::Hidden;
        *prompt_vis = Visibility::Hidden;
        return;
    }
    *vis = Visibility::Visible;
    let Ok(tf) = player.single() else { return };
    let at = Vec2::new(tf.translation.x, tf.translation.z);
    let range = |p: ba_protocol::Pose| at.distance(Vec2::new(p.x, p.z));
    let (name, mut action) = if rest(net.region_x) {
        (
            "THE SABLE REST",
            "Cael’s forge ahead · Return ferry at the blue lanterns to the south",
        )
    } else if !marsh(net.region_x) {
        ("GREYMERE", "E · Collect materials")
    } else if range(GATE) < 13. {
        (
            "THE DROWNED GATE",
            "Sable Rest ferry · Approach the twin blue lanterns at the ward stone",
        )
    } else if range(POST) < 10. {
        (
            "THE LANTERN POST",
            if range(POST) < 4.5 {
                "E   Speak to Pell"
            } else {
                "Approach Pell at the blue lantern"
            },
        )
    } else if range(BLIND) < 10. {
        ("HUNTER'S BLIND", "E · Search under the hide rack")
    } else if range(SHRINE) < 10. {
        ("FEN SHRINE", "E · Rest by Pell's fire")
    } else if tf.translation.z < -36. {
        (
            "THE BONE ISLANDS",
            "Gather from the banks. Follow the lanterns north to Edda",
        )
    } else if tf.translation.z < 20. {
        (
            "THE SUNKEN OX",
            "The Ox walks east of the ribs. Bring its yoke north",
        )
    } else {
        (
            "THE SHALLOWS",
            "Hunt the reeds, then head north to the great ribs",
        )
    };
    if range(LANTERN_ROAD) < TRAVEL_REACH {
        action = "E · Travel to Greymere";
    } else if range(SABLE_DOCK) < TRAVEL_REACH {
        action = "E · Take the return ferry to Stillwater Boneyard";
    } else if range(GATE) < TRAVEL_REACH {
        action = "E · Cross the Drowned Gate to Sable Rest";
    } else if range(REST) < 4.5 {
        action = "E · Forge Cael’s Iron Edge · 4 Cold Iron + 1 Peat";
    }
    if rest(net.region_x)
        && at.distance(Vec2::new(608.41, 2.18)) < 2.6
    {
        if let Ok(door) = doors.single() {
            action = if door.open {
                "E · Close the inn door"
            } else {
                "E · Open the inn door"
            };
        }
    }
    if HERONS.iter().any(|&p| range(p) < 4.5) {
        action = "E · Listen to the heron"
    }
    if NODES.iter().any(|&(p, _)| range(p) < 4.5) {
        action = "E · Gather materials"
    }
    if m.drops
        .iter()
        .any(|d| at.distance(Vec2::new(d.x, d.z)) < 3.6 && d.age >= 0.75)
    {
        action = "E · Pick up fallen materials"
    }
    if display.last_notice != m.notice {
        display.last_notice = m.notice.clone();
        display.notice_until = time.elapsed_secs() + 9.;
    }
    let mut caption = action.replace(" · ", "   ");
    if m.dead {
        caption = "The water holds you…".into();
    } else if !display.inventory && !m.notice.is_empty() && time.elapsed_secs() < display.notice_until {
        caption = format!("{}\n{}", m.notice.replace(" · ", " / "), caption);
    }
    if **prompt_text != caption {
        **prompt_text = caption;
    }
    *prompt_vis = Visibility::Visible;
    let mut lines = if marsh(net.region_x) {
        m.quest
            .split_once(" · ")
            .map_or(m.quest.as_str(), |(_, objective)| objective)
            .to_owned()
    } else {
        name.to_owned()
    };
    if m.rain && marsh(net.region_x) {
        lines.push_str("\nWarm rain / Double essence");
    }
    if m.stamina < 100 {
        lines.push_str(&format!("\nStamina {}", m.stamina));
    }
    if !display.inventory && range(POST) < 10. {
        lines.push_str("\nI   Pack & skills");
    }
    if display.inventory {
        lines.push_str(&format!(
            "\n{name}\nShift roll / Space jump / Ctrl sprint / C crouch / R rest / Q heal\nI pack / L {} drops",
            if display.filter { "rare" } else { "all" }
        ));
        if range(POST) < 12. {
            lines.push_str("\nGreymere passage: twin blue lanterns south of Pell");
        }
        if !m.notice.is_empty() {
            lines.push_str(&format!("\n{}", m.notice.replace(" · ", " / ")));
        }
        for (name, n) in &m.inventory {
            lines.push_str(&format!("\n{name} x{n}"));
        }
    }
    if **text != lines {
        **text = lines;
    }
}

#[derive(Resource)]
pub struct EddaClips {
    graph: Handle<AnimationGraph>,
    nodes: Vec<AnimationNodeIndex>,
}

pub fn edda_animation(
    mut commands: Commands,
    clips: Res<EddaClips>,
    net: Res<ZoneNet>,
    time: Res<Time>,
    actors: Query<(Entity, &Actor)>,
    children: Query<&Children>,
    mut players: Query<(&mut AnimationPlayer, Option<&mut AnimationTransitions>)>,
    mut last: Local<Option<(usize, Vec2, f32, f32)>>,
) {
    let Some(enemy) = net.marsh.enemies.iter().find(|e| e.id == EDDA) else {
        *last = None;
        return;
    };
    let Some((root, _)) = actors.iter().find(|(_, actor)| actor.0 == EDDA) else {
        return;
    };
    let pos = Vec2::new(enemy.x, enemy.z);
    let (previous, old_pos, until, release) = last.unwrap_or((usize::MAX, pos, 0., 0.));
    let now = time.elapsed_secs();
    let moving_until = if pos.distance_squared(old_pos) > 0.001 {
        now + 0.18
    } else {
        until
    };
    let release = if enemy.windup > 0. {
        now + 0.23
    } else {
        release
    };
    let pose = if enemy.hp == 0 {
        3
    } else if enemy.windup > 0. || now < release {
        2
    } else if now < moving_until {
        1
    } else {
        0
    };
    for id in children.iter_descendants(root) {
        let Ok((mut player, transitions)) = players.get_mut(id) else {
            continue;
        };
        let Some(mut transitions) = transitions else {
            commands.entity(id).insert((
                AnimationGraphHandle(clips.graph.clone()),
                AnimationTransitions::new(),
            ));
            continue;
        };
        if pose != previous {
            let anim = transitions.play(
                &mut player,
                clips.nodes[pose],
                std::time::Duration::from_millis(110),
            );
            if pose < 2 {
                anim.repeat();
            } else if pose == 2 {
                anim.set_speed(0.68);
            }
        }
        *last = Some((pose, pos, moving_until, release));
    }
}
pub fn mist(
    atmo: Res<crate::atmosphere::StickAtmosphere>,
    mut cameras: Query<&mut bevy::pbr::DistanceFog, With<Camera3d>>,
) {
    let Ok(mut fog) = cameras.single_mut() else {
        return;
    };
    let reach = crate::atmosphere::fog_visibility(&atmo);
    *fog = crate::atmosphere::distance_fog(&atmo, reach);
}

#[derive(Component)]
pub struct ImpactText;
pub fn impact_mark(
    net: Res<ZoneNet>,
    mut query: Query<(&mut Text, &mut TextColor, &mut Visibility), With<ImpactText>>,
) {
    let Ok((mut text, mut color, mut vis)) = query.single_mut() else {
        return;
    };
    let Some((when, big, hurt)) = net
        .impact
        .filter(|(t, _, _)| net.online() && t.elapsed().as_secs_f32() < 0.52)
    else {
        *vis = Visibility::Hidden;
        return;
    };
    *vis = Visibility::Visible;
    color.0 = if hurt {
        Color::srgb(1., 0.28, 0.22)
    } else if big {
        Color::srgb(1., 0.75, 0.19)
    } else {
        Color::WHITE
    };
    **text = if when.elapsed().as_millis() < if big { 110 } else { 50 } {
        "✦".into()
    } else if hurt {
        format!("−{}", net.last_amount)
    } else {
        net.last_amount.to_string()
    };
}
