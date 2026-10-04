//! Draws the cell world as a grid of nearest-filtered texture tiles,
//! re-shading only chunks that changed (plus visible animated chunks every
//! few frames for liquid shimmer and glow) and re-uploading only their tiles.

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use sbct_sim::rng::hash2;
use sbct_sim::{CHUNK_SIZE, Material};

use crate::session::Session;

/// Screen pixels per cell.
pub const PIXEL_SCALE: f32 = 3.0;
/// Side of a world texture tile in cells (a multiple of the chunk size).
const TILE: usize = 256;
/// Visible animated chunks are re-shaded every this many frames.
const ANIMATE_EVERY: u32 = 4;

#[derive(Component)]
pub struct WorldCamera;

/// Everything spawned for a game session; despawned on leaving.
#[derive(Component)]
pub struct InGameEntity;

#[derive(Resource)]
pub struct WorldTiles {
    tiles_x: usize,
    handles: Vec<Handle<Image>>,
    frame: u32,
    /// Chunks that changed while off screen; shaded once they're visible.
    pending: std::collections::HashSet<(usize, usize)>,
}

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
    let (tiles_x, tiles_y) = (world.width().div_ceil(TILE), world.height().div_ceil(TILE));
    let mut handles = Vec::with_capacity(tiles_x * tiles_y);
    for ty in 0..tiles_y {
        for tx in 0..tiles_x {
            let mut image = Image::new_fill(
                Extent3d {
                    width: TILE as u32,
                    height: TILE as u32,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                &[0, 0, 0, 0],
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
            );
            image.sampler = ImageSampler::nearest();
            let handle = images.add(image);
            let t = TILE as f32;
            commands.spawn((
                InGameEntity,
                Sprite {
                    image: handle.clone(),
                    custom_size: Some(Vec2::splat(t)),
                    ..default()
                },
                Transform::from_xyz(tx as f32 * t + t / 2.0, -(ty as f32 * t + t / 2.0), 0.0),
            ));
            handles.push(handle);
        }
    }
    commands.insert_resource(WorldTiles {
        tiles_x,
        handles,
        frame: 0,
        pending: Default::default(),
    });

    if session.backdrop {
        spawn_surface_backdrop(&mut commands, world, &mut images);
    }
}

/// Dark backdrop behind everything below the original surface, so dug-out
/// areas and caves read as underground rather than sky (sandbox worlds).
fn spawn_surface_backdrop(commands: &mut Commands, world: &sbct_sim::World, images: &mut Assets<Image>) {
    let (w, h) = (world.width() as u32, world.height() as u32);
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
}

pub fn upload_dirty_chunks(
    mut session: ResMut<Session>,
    mut tiles: ResMut<WorldTiles>,
    mut images: ResMut<Assets<Image>>,
    camera: Single<(&Transform, &Projection), With<WorldCamera>>,
    window: Single<&Window, With<bevy::window::PrimaryWindow>>,
) {
    let Some(world) = &mut session.world else { return };
    tiles.frame = tiles.frame.wrapping_add(1);
    let frame = tiles.frame;
    let tiles = &mut *tiles;
    tiles.pending.extend(world.take_render_dirty());

    // Only shade what's on screen (plus a margin); the rest waits.
    let (tf, projection) = *camera;
    let scale = match projection {
        Projection::Orthographic(o) => o.scale,
        _ => 1.0,
    };
    let half = Vec2::new(window.width(), window.height()) * scale / 2.0 + 16.0;
    let cs = CHUNK_SIZE as f32;
    let (cx0, cx1) = ((tf.translation.x - half.x) / cs, (tf.translation.x + half.x) / cs);
    let (cy0, cy1) = (
        (-tf.translation.y - half.y) / cs,
        (-tf.translation.y + half.y) / cs,
    );
    let clamp = |v: f32, n: usize| (v.max(0.0) as usize).min(n - 1);
    let (cx0, cx1) = (clamp(cx0, world.chunks_x()), clamp(cx1, world.chunks_x()));
    let (cy0, cy1) = (clamp(cy0, world.chunks_y()), clamp(cy1, world.chunks_y()));
    let visible = |&(cx, cy): &(usize, usize)| cx >= cx0 && cx <= cx1 && cy >= cy0 && cy <= cy1;
    let mut dirty: Vec<(usize, usize)> = tiles.pending.iter().copied().filter(visible).collect();
    for c in &dirty {
        tiles.pending.remove(c);
    }
    if frame.is_multiple_of(ANIMATE_EVERY) {
        for cy in cy0..=cy1 {
            for cx in cx0..=cx1 {
                if world.chunk_animates(cx, cy) {
                    dirty.push((cx, cy));
                }
            }
        }
        dirty.sort_unstable();
        dirty.dedup();
    }

    let per_tile = TILE / CHUNK_SIZE;
    for (cx, cy) in dirty {
        let (tx, ty) = (cx / per_tile, cy / per_tile);
        let Some(handle) = tiles.handles.get(ty * tiles.tiles_x + tx) else {
            continue;
        };
        let Some(mut image) = images.get_mut(handle) else {
            continue;
        };
        let Some(data) = image.data.as_mut() else { continue };
        world.write_chunk_rgba(cx, cy, data, TILE, (tx * TILE, ty * TILE), frame);
    }
}

pub fn cleanup_world_view(mut commands: Commands, entities: Query<Entity, With<InGameEntity>>) {
    for e in &entities {
        commands.entity(e).despawn();
    }
    commands.remove_resource::<WorldTiles>();
}

/// Sprite pool for cells in flight (explosion debris, sprays).
#[derive(Resource, Default)]
pub struct ParticlePool(Vec<Entity>);

#[derive(Component)]
pub struct SimParticleSprite;

pub fn draw_sim_particles(
    mut commands: Commands,
    session: Res<Session>,
    mut pool: ResMut<ParticlePool>,
    mut sprites: Query<(&mut Transform, &mut Sprite, &mut Visibility), With<SimParticleSprite>>,
) {
    let Some(world) = &session.world else { return };
    let particles = world.particles();
    while pool.0.len() < particles.len().min(4000) {
        let e = commands
            .spawn((
                SimParticleSprite,
                InGameEntity,
                Sprite::from_color(Color::WHITE, Vec2::ONE),
                Transform::default(),
                Visibility::Hidden,
            ))
            .id();
        pool.0.push(e);
    }
    let mut used = 0;
    for (p, &e) in particles.iter().zip(pool.0.iter()) {
        let Ok((mut tf, mut sprite, mut vis)) = sprites.get_mut(e) else {
            continue;
        };
        let cell = sbct_sim::Cell::new(p.mat, 128);
        let [r, g, b, a] = sbct_sim::color::cell_rgba(cell, p.x as i32, p.y as i32, 0);
        sprite.color = Color::srgba_u8(r, g, b, a.max(200));
        tf.translation = Vec3::new(p.x, -p.y, 1.0);
        *vis = Visibility::Visible;
        used += 1;
    }
    for &e in pool.0.iter().skip(used) {
        if let Ok((_, _, mut vis)) = sprites.get_mut(e) {
            *vis = Visibility::Hidden;
        }
    }
}

pub fn reset_particle_pool(mut pool: ResMut<ParticlePool>) {
    pool.0.clear();
}
