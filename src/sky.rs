use bevy::{
    light::{NotShadowCaster, NotShadowReceiver},
    mesh::Indices,
    prelude::*,
    render::render_resource::{AsBindGroup, ShaderType},
    shader::ShaderRef,
};

use crate::atmosphere::StickAtmosphere;

const SHADER_PATH: &str = "shaders/stick_sky.wesl";

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct StickSkyMaterial {
    #[uniform(0)]
    pub uniforms: StickSkyUniforms,
}

#[derive(Clone, Copy, Debug, ShaderType)]
pub struct StickSkyUniforms {
    pub phase: f32,
    pub time: f32,
    pub cover: f32,
    pub dark: f32,
    pub haze: f32,
    pub veil: f32,
    pub _pad0: f32,
    pub _pad1: f32,
}

impl From<&StickAtmosphere> for StickSkyUniforms {
    fn from(a: &StickAtmosphere) -> Self {
        Self {
            phase: a.day_phase,
            time: a.time,
            cover: a.weather_cover,
            dark: a.weather_dark,
            haze: a.weather_haze,
            veil: a.weather_veil,
            _pad0: 0.0,
            _pad1: 0.0,
        }
    }
}

impl Material for StickSkyMaterial {
    fn fragment_shader() -> ShaderRef {
        SHADER_PATH.into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Opaque
    }

    fn depth_bias(&self) -> f32 {
        // Draw behind scene geometry.
        1_000.0
    }

    fn enable_prepass() -> bool {
        false
    }

    fn enable_shadows() -> bool {
        false
    }
}

#[derive(Component)]
pub struct StickSky;

pub fn spawn_sky_dome(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StickSkyMaterial>>,
    atmo: Res<StickAtmosphere>,
) {
    let mut sphere = Sphere::new(800.0).mesh().ico(4).expect("sky ico");
    // Flip winding so we see the inside.
    if let Some(indices) = sphere.indices_mut() {
        match indices {
            Indices::U16(v) => {
                for tri in v.chunks_exact_mut(3) {
                    tri.swap(0, 2);
                }
            }
            Indices::U32(v) => {
                for tri in v.chunks_exact_mut(3) {
                    tri.swap(0, 2);
                }
            }
        }
    }

    commands.spawn((
        StickSky,
        Name::new("StickSkyDome"),
        Mesh3d(meshes.add(sphere)),
        MeshMaterial3d(materials.add(StickSkyMaterial {
            uniforms: StickSkyUniforms::from(&*atmo),
        })),
        Transform::from_xyz(2.8, 1.65, 5.2),
        // No fog / shadows on sky
        NotShadowCaster,
        NotShadowReceiver,
    ));
}

pub fn follow_sky_to_player(
    player: Query<&Transform, (With<crate::player::Player>, Without<StickSky>)>,
    mut sky: Query<&mut Transform, With<StickSky>>,
) {
    let Ok(p) = player.single() else {
        return;
    };
    for mut tf in &mut sky {
        tf.translation = p.translation;
    }
}

pub fn sync_sky_uniforms(
    atmo: Res<StickAtmosphere>,
    mut materials: ResMut<Assets<StickSkyMaterial>>,
    skies: Query<&MeshMaterial3d<StickSkyMaterial>, With<StickSky>>,
) {
    if !atmo.is_changed() && !atmo.is_added() {
        // Still update time every frame
    }
    for handle in &skies {
        if let Some(mut mat) = materials.get_mut(&handle.0) {
            mat.uniforms = StickSkyUniforms::from(&*atmo);
        }
    }
}
