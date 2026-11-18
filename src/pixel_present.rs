//! Low-resolution world camera, nearest-neighbor present.
//! The 3D opening (well, ruins, old man, sky) renders into a pixel target.

use bevy::{
    asset::RenderAssetUsages,
    camera::RenderTarget,
    ecs::hierarchy::ChildOf,
    image::{Image, ImageSampler},
    mesh::{Indices, PrimitiveTopology},
    prelude::*,
    render::render_resource::TextureFormat,
    ui::UiTargetCamera,
};

use ba_pixel_geometry::CpuMesh;

const PIXEL_W: u32 = 768;
const PIXEL_H: u32 = 432;

#[derive(Component)]
pub struct PixelPresentCam;

#[derive(Component)]
pub(crate) struct PixelPresentSprite;

pub fn cpu_to_mesh(cpu: &CpuMesh) -> Mesh {
    let mut mesh = Mesh::new(
        PrimitiveTopology::TriangleList,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    if cpu.positions.is_empty() {
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec![[0.0, 0.0, 0.0]; 3]);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, vec![[0.0, 1.0, 0.0]; 3]);
        mesh.insert_indices(Indices::U32(vec![0, 1, 2]));
        return mesh;
    }
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, cpu.positions.clone());
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, cpu.normals.clone());
    mesh.insert_indices(Indices::U32(cpu.indices.clone()));
    mesh
}

pub fn attach_pixel_camera(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    world_cam: Query<Entity, With<Camera3d>>,
) {
    let Ok(cam) = world_cam.single() else {
        return;
    };
    let mut target = Image::new_target_texture(PIXEL_W, PIXEL_H, TextureFormat::Rgba8UnormSrgb, None);
    target.sampler = ImageSampler::nearest();
    let target = images.add(target);

    commands.entity(cam).insert((Msaa::Off, RenderTarget::Image(target.clone().into())));
    commands.insert_resource(PixelFrame(target.clone()));

    let present = commands
        .spawn((
            PixelPresentCam,
            Camera2d,
            Camera { order: 1, ..default() },
            Msaa::Off,
        ))
        .id();

    commands.spawn((
        PixelPresentSprite,
        Sprite {
            image: target,
            custom_size: Some(Vec2::new(1280.0, 720.0)),
            ..default()
        },
    ));

    // UI roots created in spawn_hud need this camera or they draw into the offscreen view.
    commands.insert_resource(PixelUiCamera(present));
}

#[derive(Resource)]
pub(crate) struct PixelUiCamera(Entity);

/// The low-resolution image the world camera renders into.
#[derive(Resource, Clone)]
pub struct PixelFrame(pub Handle<Image>);

pub fn point_ui_at_present(
    cam: Res<PixelUiCamera>,
    mut commands: Commands,
    roots: Query<Entity, (With<Node>, Without<ChildOf>, Without<UiTargetCamera>)>,
) {
    for entity in &roots {
        commands.entity(entity).insert(UiTargetCamera(cam.0));
    }
}

pub fn fit_pixel_present(
    windows: Query<&Window>,
    mut sprites: Query<&mut Sprite, With<PixelPresentSprite>>,
) {
    let Ok(window) = windows.single() else {
        return;
    };
    let Ok(mut sprite) = sprites.single_mut() else {
        return;
    };
    sprite.custom_size = Some(Vec2::new(window.width(), window.height()));
}
