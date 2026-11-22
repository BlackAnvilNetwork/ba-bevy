//! Loot beam rendering: custom material, layered geometry, and through-wall ghost.

use bevy::{
    asset::RenderAssetUsages,
    light::NotShadowCaster,
    prelude::*,
    render::render_resource::AsBindGroup,
    shader::ShaderRef,
};

use super::{
    grammar::LootGrammar,
    timeline::{beam_state, ground_glow_state, spark_positions, BeamState, GroundGlowState},
};

/// Marker for the core beam column entity.
#[derive(Component)]
pub struct BeamCore;

/// Marker for the soft halo around the beam.
#[derive(Component)]
pub struct BeamHalo;

/// Marker for the ground glow disc.
#[derive(Component)]
pub struct BeamGroundGlow;

/// Marker for the through-wall ghost pass.
#[derive(Component)]
pub struct BeamGhost;

/// Marker for a spark particle.
#[derive(Component)]
pub struct BeamSpark {
    pub index: usize,
}

/// Marker for the beam's point light.
#[derive(Component)]
pub struct BeamLight;

/// A complete loot beam entity with all its visual components.
#[derive(Component)]
pub struct LootBeamBundle {
    pub grammar: LootGrammar,
    pub spawn_time: f32,
}

/// Custom material for loot beams: unlit, additive, fog-exempt, HDR emissive.
#[derive(Asset, AsBindGroup, TypePath, Clone)]
pub struct LootBeamMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
    #[uniform(0)]
    pub hdr_multiplier: f32,
    #[uniform(0)]
    pub opacity: f32,
    #[uniform(0)]
    pub pulse_pos: f32,
    #[uniform(0)]
    pub pulse_active: f32,
    /// For the ghost pass: 1.0 for ghost, 0.0 for normal.
    #[uniform(0)]
    pub ghost_pass: f32,
}

impl Default for LootBeamMaterial {
    fn default() -> Self {
        Self {
            color: LinearRgba::WHITE,
            hdr_multiplier: 2.0,
            opacity: 1.0,
            pulse_pos: 0.0,
            pulse_active: 0.0,
            ghost_pass: 0.0,
        }
    }
}

impl Material for LootBeamMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/loot_beam.wgsl".into()
    }

    fn vertex_shader() -> ShaderRef {
        "shaders/loot_beam.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }
}

/// Material for the ground glow disc.
#[derive(Asset, AsBindGroup, TypePath, Clone)]
pub struct GroundGlowMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
    #[uniform(0)]
    pub intensity: f32,
}

impl Default for GroundGlowMaterial {
    fn default() -> Self {
        Self {
            color: LinearRgba::WHITE,
            intensity: 1.0,
        }
    }
}

impl Material for GroundGlowMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/ground_glow.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }
}

/// Material for spark particles.
#[derive(Asset, AsBindGroup, TypePath, Clone)]
pub struct SparkMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
    #[uniform(0)]
    pub intensity: f32,
}

impl Default for SparkMaterial {
    fn default() -> Self {
        Self {
            color: LinearRgba::WHITE,
            intensity: 3.0,
        }
    }
}

impl Material for SparkMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/spark.wgsl".into()
    }

    fn alpha_mode(&self) -> AlphaMode {
        AlphaMode::Add
    }
}

/// Resource holding shared beam meshes.
#[derive(Resource)]
pub struct BeamMeshes {
    /// Cylinder mesh for beam core.
    pub core_cylinder: Handle<Mesh>,
    /// Wider cylinder for halo.
    pub halo_cylinder: Handle<Mesh>,
    /// Disc mesh for ground glow.
    pub ground_disc: Handle<Mesh>,
    /// Small quad for sparks.
    pub spark_quad: Handle<Mesh>,
}

/// Create a vertical cylinder mesh centered at origin.
pub fn create_beam_cylinder(segments: u32, halo: bool) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    // Unit height cylinder (0 to 1 on Y axis)
    for i in 0..=segments {
        let angle = i as f32 * std::f32::consts::TAU / segments as f32;
        let (sin, cos) = angle.sin_cos();

        // Bottom vertex
        positions.push([cos * 0.5, 0.0, sin * 0.5]);
        normals.push([cos, 0.0, sin]);
        uvs.push([i as f32 / segments as f32, 0.0]);

        // Top vertex
        positions.push([cos * 0.5, 1.0, sin * 0.5]);
        normals.push([cos, 0.0, sin]);
        uvs.push([i as f32 / segments as f32, 1.0]);
    }

    // Indices for triangle strip
    for i in 0..segments {
        let base = i * 2;
        indices.extend_from_slice(&[base, base + 1, base + 2, base + 2, base + 1, base + 3]);
    }

    let mut mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::mesh::Indices::U32(indices));

    if halo {
        // Halo is wider, uses same mesh but scaled
    }

    mesh
}

/// Create a ground disc mesh with radial gradient UVs.
pub fn create_ground_disc(segments: u32) -> Mesh {
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uvs = Vec::new();
    let mut indices = Vec::new();

    // Center vertex
    positions.push([0.0, 0.0, 0.0]);
    normals.push([0.0, 1.0, 0.0]);
    uvs.push([0.5, 0.5]);

    // Edge vertices
    for i in 0..=segments {
        let angle = i as f32 * std::f32::consts::TAU / segments as f32;
        let (sin, cos) = angle.sin_cos();
        positions.push([cos * 0.5, 0.0, sin * 0.5]);
        normals.push([0.0, 1.0, 0.0]);
        uvs.push([0.5 + cos * 0.5, 0.5 + sin * 0.5]);
    }

    // Indices for fan
    for i in 0..segments {
        indices.extend_from_slice(&[0, i + 1, i + 2]);
    }

    let mut mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::mesh::Indices::U32(indices));

    mesh
}

/// Create a small quad mesh for spark billboards.
pub fn create_spark_quad() -> Mesh {
    let size = 0.5;
    let positions = vec![
        [-size, -size, 0.0],
        [size, -size, 0.0],
        [size, size, 0.0],
        [-size, size, 0.0],
    ];
    let normals = vec![[0.0, 0.0, 1.0]; 4];
    let uvs = vec![[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];
    let indices = vec![0, 1, 2, 0, 2, 3];

    let mut mesh = Mesh::new(
        bevy::mesh::PrimitiveTopology::TriangleList,
        RenderAssetUsages::default(),
    );
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
    mesh.insert_indices(bevy::mesh::Indices::U32(indices));

    mesh
}

/// Initialize shared beam meshes.
pub fn setup_beam_meshes(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(BeamMeshes {
        core_cylinder: meshes.add(create_beam_cylinder(16, false)),
        halo_cylinder: meshes.add(create_beam_cylinder(12, true)),
        ground_disc: meshes.add(create_ground_disc(24)),
        spark_quad: meshes.add(create_spark_quad()),
    });
}

/// Spawn a complete loot beam with all visual layers.
pub fn spawn_beam(
    commands: &mut Commands,
    beam_meshes: &BeamMeshes,
    beam_materials: &mut Assets<LootBeamMaterial>,
    glow_materials: &mut Assets<GroundGlowMaterial>,
    spark_materials: &mut Assets<SparkMaterial>,
    grammar: LootGrammar,
    position: Vec3,
    spawn_time: f32,
) -> Entity {
    if !grammar.has_beam() {
        // Spawn an invisible marker for consistency
        return commands
            .spawn((
                LootBeamBundle { grammar, spawn_time },
                Transform::from_translation(position),
                Visibility::Hidden,
            ))
            .id();
    }

    let color = grammar.linear_color();
    let height = grammar.beam_height();
    let core_width = grammar.core_width();
    let glow_radius = grammar.ground_glow_radius();

    // Create materials
    let core_mat = beam_materials.add(LootBeamMaterial {
        color,
        hdr_multiplier: grammar.hdr_multiplier(),
        opacity: 1.0,
        ghost_pass: 0.0,
        ..default()
    });

    let ghost_mat = beam_materials.add(LootBeamMaterial {
        color,
        hdr_multiplier: grammar.hdr_multiplier() * 0.6,
        opacity: 0.6, // 60% opacity for ghost
        ghost_pass: 1.0,
        ..default()
    });

    let halo_mat = beam_materials.add(LootBeamMaterial {
        color,
        hdr_multiplier: grammar.hdr_multiplier() * 0.5,
        opacity: 0.3,
        ghost_pass: 0.0,
        ..default()
    });

    let glow_mat = glow_materials.add(GroundGlowMaterial {
        color,
        intensity: 1.0,
    });

    // Spawn parent entity
    let beam_entity = commands
        .spawn((
            LootBeamBundle { grammar, spawn_time },
            Transform::from_translation(position),
            Visibility::Inherited,
        ))
        .with_children(|parent| {
            // Core beam cylinder
            parent.spawn((
                BeamCore,
                Mesh3d(beam_meshes.core_cylinder.clone()),
                MeshMaterial3d(core_mat.clone()),
                Transform::from_scale(Vec3::new(core_width, height, core_width)),
                NotShadowCaster,
            ));

            // Ghost pass (through-wall) - same mesh, different material
            parent.spawn((
                BeamGhost,
                Mesh3d(beam_meshes.core_cylinder.clone()),
                MeshMaterial3d(ghost_mat),
                Transform::from_scale(Vec3::new(core_width * 1.1, height, core_width * 1.1)),
                NotShadowCaster,
            ));

            // Soft halo (3x wider)
            parent.spawn((
                BeamHalo,
                Mesh3d(beam_meshes.halo_cylinder.clone()),
                MeshMaterial3d(halo_mat),
                Transform::from_scale(Vec3::new(core_width * 4.0, height, core_width * 4.0)),
                NotShadowCaster,
            ));

            // Ground glow disc
            parent.spawn((
                BeamGroundGlow,
                Mesh3d(beam_meshes.ground_disc.clone()),
                MeshMaterial3d(glow_mat),
                Transform::from_xyz(0.0, 0.05, 0.0)
                    .with_scale(Vec3::new(glow_radius * 2.0, 1.0, glow_radius * 2.0)),
                NotShadowCaster,
            ));

            // Point light for ground illumination
            parent.spawn((
                BeamLight,
                PointLight {
                    color: grammar.color(),
                    intensity: grammar.light_intensity(),
                    range: glow_radius * 3.0,
                    shadow_maps_enabled: false,
                    ..default()
                },
                Transform::from_xyz(0.0, 0.5, 0.0),
            ));

            // Sparks for Legendary+
            if grammar.has_sparks() {
                let spark_color = grammar.linear_color();
                let spark_mat = spark_materials.add(SparkMaterial {
                    color: spark_color,
                    intensity: 4.0,
                });

                for i in 0..grammar.spark_count() {
                    parent.spawn((
                        BeamSpark { index: i },
                        Mesh3d(beam_meshes.spark_quad.clone()),
                        MeshMaterial3d(spark_mat.clone()),
                        Transform::from_scale(Vec3::splat(0.08)),
                        NotShadowCaster,
                        Visibility::Inherited,
                    ));
                }
            }
        })
        .id();

    beam_entity
}

/// Update beam visuals based on elapsed time.
pub fn update_beam_visuals(
    time: Res<Time>,
    mut beams: Query<(&LootBeamBundle, &Children)>,
    mut cores: Query<&mut Transform, (With<BeamCore>, Without<BeamHalo>, Without<BeamGroundGlow>)>,
    mut halos: Query<&mut Transform, (With<BeamHalo>, Without<BeamCore>, Without<BeamGroundGlow>)>,
    mut glows: Query<
        &mut Transform,
        (
            With<BeamGroundGlow>,
            Without<BeamCore>,
            Without<BeamHalo>,
        ),
    >,
    mut lights: Query<&mut PointLight, With<BeamLight>>,
    mut sparks: Query<(&BeamSpark, &mut Transform, &mut Visibility), Without<BeamCore>>,
    mut beam_mats: ResMut<Assets<LootBeamMaterial>>,
    mut glow_mats: ResMut<Assets<GroundGlowMaterial>>,
    core_handles: Query<&MeshMaterial3d<LootBeamMaterial>, With<BeamCore>>,
    glow_handles: Query<&MeshMaterial3d<GroundGlowMaterial>, With<BeamGroundGlow>>,
) {
    let now = time.elapsed_secs();

    for (bundle, children) in &beams {
        let elapsed = now - bundle.spawn_time;
        let state = beam_state(bundle.grammar, elapsed);
        let glow_state = ground_glow_state(bundle.grammar, elapsed);
        let sparks_positions = spark_positions(bundle.grammar, elapsed, bundle.grammar.spark_count());

        if !state.visible {
            continue;
        }

        for child in children.iter() {
            // Update core transform
            if let Ok(mut tf) = cores.get_mut(child) {
                let base_width = bundle.grammar.core_width();
                tf.scale = Vec3::new(
                    base_width * state.width_mult,
                    state.height,
                    base_width * state.width_mult,
                );
                tf.rotation = Quat::from_rotation_y(state.rotation);
            }

            // Update halo transform
            if let Ok(mut tf) = halos.get_mut(child) {
                let base_width = bundle.grammar.core_width() * 4.0;
                tf.scale = Vec3::new(
                    base_width * state.width_mult,
                    state.height,
                    base_width * state.width_mult,
                );
                tf.rotation = Quat::from_rotation_y(state.rotation);
            }

            // Update ground glow
            if let Ok(mut tf) = glows.get_mut(child) {
                let scale = glow_state.radius * 2.0 * glow_state.intensity;
                tf.scale = Vec3::new(scale, 1.0, scale);
            }

            // Update point light
            if let Ok(mut light) = lights.get_mut(child) {
                light.intensity = bundle.grammar.light_intensity() * glow_state.intensity;
            }

            // Update material uniforms
            if let Ok(handle) = core_handles.get(child) {
                if let Some(mut mat) = beam_mats.get_mut(&handle.0) {
                    mat.color = state.color;
                    mat.hdr_multiplier = state.hdr_mult;
                    mat.opacity = state.opacity;
                    if let Some(pos) = state.pulse_pos {
                        mat.pulse_pos = pos;
                        mat.pulse_active = 1.0;
                    } else {
                        mat.pulse_active = 0.0;
                    }
                }
            }

            if let Ok(handle) = glow_handles.get(child) {
                if let Some(mut mat) = glow_mats.get_mut(&handle.0) {
                    mat.color = glow_state.color;
                    mat.intensity = glow_state.intensity;
                }
            }

            // Update spark positions
            if let Ok((spark, mut tf, mut vis)) = sparks.get_mut(child) {
                if let Some((height_frac, angle)) = sparks_positions.get(spark.index) {
                    *vis = Visibility::Inherited;
                    let beam_height = bundle.grammar.beam_height();
                    let radius = bundle.grammar.core_width() * 2.0;
                    tf.translation = Vec3::new(
                        angle.cos() * radius,
                        height_frac * beam_height,
                        angle.sin() * radius,
                    );
                    // Fade out near top
                    let alpha = 1.0 - height_frac.powi(2);
                    tf.scale = Vec3::splat(0.08 * (0.5 + 0.5 * alpha));
                } else {
                    *vis = Visibility::Hidden;
                }
            }
        }
    }
}
