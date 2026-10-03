//! Draws the cell world as a single nearest-filtered texture, re-uploading
//! only the chunks that changed.

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use sbct_sim::Material;
use sbct_sim::rng::hash2;

use crate::session::Session;

/// Screen pixels per cell.
pub const PIXEL_SCALE: f32 = 3.0;

#[derive(Component)]
pub struct WorldCamera;

/// Everything spawned for a game session; despawned on leaving.
#[derive(Component)]
pub struct InGameEntity;

#[derive(Resource)]
pub struct WorldTexture(pub Handle<Image>);

/// Whether egui is under the mouse, so clicks on UI don't dig.
#[derive(Resource, Default)]
pub struct UiHasPointer(pub bool);

pub fn spawn_camera(mut commands: Commands) {
    commands.spawn((
        Camera2d,
        WorldCamera,
        Projection::Orthographic(OrthographicProjection {
            scale: 1.0 / PIXEL_SCALE,
            ..OrthographicProjection::default_2d()
        }),
    ));
}

pub fn spawn_world_view(mut commands: Commands, session: Res<Session>, mut images: ResMut<Assets<Image>>) {
    let Some(world) = &session.world else { return };
    let (w, h) = (world.width() as u32, world.height() as u32);
    let mut image = Image::new_fill(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 0],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::nearest();
    let handle = images.add(image);

    // Dark backdrop behind everything below the original surface, so dug-out
    // areas and caves read as underground rather than sky.
    let mut backdrop = vec![0u8; (w * h * 4) as usize];
    for x in 0..w as i32 {
        let surface = (0..h as i32)
            .find(|&y| {
                matches!(
                    world.material(x, y),
                    Material::Dirt | Material::Stone | Material::Sand
                )
            })
            .unwrap_or(h as i32);
        for y in (surface + 2).max(0)..h as i32 {
            let n = (hash2(world.seed(), x / 2, y / 2) % 10) as u8;
            let i = ((y as u32 * w + x as u32) * 4) as usize;
            backdrop[i..i + 4].copy_from_slice(&[30 + n, 24 + n, 22 + n, 255]);
        }
    }
    let mut backdrop = Image::new(
        Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        backdrop,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    backdrop.sampler = ImageSampler::nearest();
    let (wf, hf) = (w as f32, h as f32);
    commands.spawn((
        InGameEntity,
        Sprite {
            image: images.add(backdrop),
            custom_size: Some(Vec2::new(wf, hf)),
            ..default()
        },
        Transform::from_xyz(wf / 2.0, -hf / 2.0, -1.0),
    ));
    commands.spawn((
        InGameEntity,
        Sprite {
            image: handle.clone(),
            custom_size: Some(Vec2::new(wf, hf)),
            ..default()
        },
        Transform::from_xyz(wf / 2.0, -hf / 2.0, 0.0),
    ));
    commands.insert_resource(WorldTexture(handle));
}

pub fn upload_dirty_chunks(
    mut session: ResMut<Session>,
    texture: Res<WorldTexture>,
    mut images: ResMut<Assets<Image>>,
) {
    let Some(world) = &mut session.world else { return };
    let dirty = world.take_render_dirty();
    if dirty.is_empty() {
        return;
    }
    let Some(mut image) = images.get_mut(&texture.0) else {
        return;
    };
    let Some(data) = image.data.as_mut() else { return };
    for (cx, cy) in dirty {
        world.write_chunk_rgba(cx, cy, data);
    }
}

pub fn cleanup_world_view(mut commands: Commands, entities: Query<Entity, With<InGameEntity>>) {
    for e in &entities {
        commands.entity(e).despawn();
    }
    commands.remove_resource::<WorldTexture>();
}
