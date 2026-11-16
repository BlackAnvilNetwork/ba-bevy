//! Flat cel shading for human-proportioned figures.
//! The pixel camera still breaks the bands into pixels.

use bevy::ecs::hierarchy::ChildOf;
use bevy::prelude::*;
use bevy::render::render_resource::{AsBindGroup, ShaderType};
use bevy::shader::ShaderRef;

use crate::scene::Warden;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct CelMaterial {
    #[uniform(0)]
    pub uniforms: CelUniforms,
}

#[derive(Clone, Copy, Debug, ShaderType)]
pub struct CelUniforms {
    pub color: LinearRgba,
    pub light_dir: Vec4,
}

impl Material for CelMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/cel_figure.wesl".into()
    }
}

/// Swap a figure's smooth materials for cel bands once the glTF has spawned them.
pub fn cel_shade_figures(
    mut commands: Commands,
    figures: Query<Entity, With<Warden>>,
    parents: Query<&ChildOf>,
    meshes: Query<(Entity, &MeshMaterial3d<StandardMaterial>)>,
    standard: Res<Assets<StandardMaterial>>,
    mut cel: ResMut<Assets<CelMaterial>>,
) {
    let light = Vec3::new(-0.2, 0.9, 0.35).normalize();
    let light_dir = Vec4::new(light.x, light.y, light.z, 0.0);
    let mut converted = 0;

    for (entity, material) in &meshes {
        if !under_figure(entity, &parents, &figures) {
            continue;
        }
        let Some(source) = standard.get(material) else {
            continue;
        };
        let color = LinearRgba::from(source.base_color);
        let handle = cel.add(CelMaterial {
            uniforms: CelUniforms { color, light_dir },
        });
        commands
            .entity(entity)
            .remove::<MeshMaterial3d<StandardMaterial>>()
            .insert(MeshMaterial3d(handle));
        converted += 1;
    }
    if converted > 0 {
        info!("cel shaded {converted} figure parts");
    }
}

fn under_figure(
    mut entity: Entity,
    parents: &Query<&ChildOf>,
    figures: &Query<Entity, With<Warden>>,
) -> bool {
    if figures.get(entity).is_ok() {
        return true;
    }
    while let Ok(child_of) = parents.get(entity) {
        entity = child_of.parent();
        if figures.get(entity).is_ok() {
            return true;
        }
    }
    false
}
