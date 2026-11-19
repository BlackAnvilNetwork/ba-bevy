//! Character Studio bakes an all-black ORM atlas onto every exported GLB.
//! Bevy multiplies roughness by that green channel, so a roughnessFactor of 1
//! still shades as chrome on every player and remote character.

use bevy::prelude::*;
use std::collections::HashSet;

const SKIN_ROUGHNESS: f32 = 0.86;
const SKIN_REFLECTANCE: f32 = 0.32;
const DEAD_ORM: u32 = 12;

pub fn matte_character_material(mat: &mut StandardMaterial) {
    mat.metallic = 0.0;
    mat.perceptual_roughness = SKIN_ROUGHNESS;
    mat.metallic_roughness_texture = None;
    mat.reflectance = SKIN_REFLECTANCE;
}

/// Packed ORM is occlusion/roughness/metallic in RGB. A placeholder atlas is
/// ~black in G and B, which collapses PBR roughness to a mirror.
pub fn packed_orm_is_dead(data: &[u8]) -> bool {
    if data.len() < 4 || data.len() % 4 != 0 {
        return false;
    }
    let mut n = 0u32;
    let mut g = 0u32;
    let mut b = 0u32;
    for px in data.chunks_exact(4).step_by(19) {
        n += 1;
        g += px[1] as u32;
        b += px[2] as u32;
    }
    n > 0 && g / n < DEAD_ORM && b / n < DEAD_ORM
}

fn orm_map_is_dead(img: &Image) -> bool {
    img.data.as_ref().is_some_and(|data| packed_orm_is_dead(data))
}

/// Any Studio character GLB — local, remote, or a later revision — ships the
/// same dead ORM. Inspect the map rather than a single Chrona path.
pub fn matte_avatar_materials(
    mut materials: ResMut<Assets<StandardMaterial>>,
    images: Res<Assets<Image>>,
    mut seen: Local<HashSet<AssetId<StandardMaterial>>>,
) {
    for (id, mut mat) in materials.iter_mut() {
        if seen.contains(&id) {
            continue;
        }
        let Some(handle) = mat.metallic_roughness_texture.clone() else {
            seen.insert(id);
            continue;
        };
        let Some(img) = images.get(&handle) else {
            continue;
        };
        if img.data.is_none() {
            continue;
        }
        if orm_map_is_dead(img) {
            matte_character_material(&mut mat);
        }
        seen.insert(id);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn black_orm_map_is_stripped_so_skin_stays_matte() {
        let mut mat = StandardMaterial {
            metallic: 0.0,
            perceptual_roughness: 1.0,
            metallic_roughness_texture: Some(Handle::default()),
            reflectance: 0.5,
            ..default()
        };
        matte_character_material(&mut mat);
        assert_eq!(mat.metallic, 0.0);
        assert!(
            mat.perceptual_roughness >= 0.8,
            "skin/cloth must stay rough, got {}",
            mat.perceptual_roughness
        );
        assert!(
            mat.metallic_roughness_texture.is_none(),
            "a black ORM map would multiply roughness back to zero"
        );
        assert!(
            mat.reflectance < 0.5,
            "default reflectance reads as plastic under moonlight"
        );
    }

    #[test]
    fn packed_orm_detects_studio_placeholder_and_keeps_real_maps() {
        let dead = vec![0u8, 0, 0, 255].repeat(64);
        assert!(packed_orm_is_dead(&dead));
        let cloth = vec![255u8, 220, 0, 255].repeat(64);
        assert!(!packed_orm_is_dead(&cloth));
        let metal = vec![255u8, 40, 230, 255].repeat(64);
        assert!(!packed_orm_is_dead(&metal));
    }
}
