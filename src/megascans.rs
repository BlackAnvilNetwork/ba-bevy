//! Optional local Fab kit. Every path is anchored to the game's installed asset root.
use bevy::{
    image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor},
    math::Affine2,
    prelude::*,
};
use std::path::PathBuf;

#[derive(Resource)]
pub struct MegascansRoot(pub PathBuf);

impl MegascansRoot {
    pub fn surface(
        &self,
        slot: &str,
        assets: &AssetServer,
        materials: &mut Assets<StandardMaterial>,
        fallback: &Handle<StandardMaterial>,
    ) -> Handle<StandardMaterial> {
        let dir = self.0.join(slot);
        if !["base.png", "normal.png", "metal-rough.png"]
            .iter()
            .all(|file| dir.join(file).is_file())
        {
            return fallback.clone();
        }
        let map = |name, srgb| {
            assets
                .load_builder()
                .with_settings(move |s: &mut ImageLoaderSettings| {
                    s.is_srgb = srgb;
                    s.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                        address_mode_u: ImageAddressMode::Repeat,
                        address_mode_v: ImageAddressMode::Repeat,
                        ..ImageSamplerDescriptor::linear()
                    });
                })
                .load(format!("megascans/{slot}/{name}"))
        };
        materials.add(StandardMaterial {
            base_color_texture: Some(map("base.png", true)),
            normal_map_texture: Some(map("normal.png", false)),
            metallic_roughness_texture: Some(map("metal-rough.png", false)),
            perceptual_roughness: 1.0,
            uv_transform: Affine2::from_scale(Vec2::splat(3.0)),
            ..default()
        })
    }

    pub fn scene(&self, slot: &str, assets: &AssetServer) -> Option<WorldAssetRoot> {
        self.0
            .join(slot)
            .join("scene.gltf")
            .is_file()
            .then(|| WorldAssetRoot(assets.load(format!("megascans/{slot}/scene.gltf#Scene0"))))
    }
}
