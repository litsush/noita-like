//! Parallax backgrounds: each layer has a far and a near tileable texture,
//! shown only within that layer's depth band and scrolled slower than the
//! world. Tiling comes from a repeat sampler and a sprite rect that reaches
//! past the texture's edges. A sky mask covers them above the surface.

use bevy::asset::RenderAssetUsages;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;
use sbct_sim::descent::Layer;

use super::lighting::SurfaceLine;
use crate::assets::GameAssets;
use crate::render::{InGameEntity, PIXEL_SCALE, WorldCamera};

#[derive(Component)]
pub struct Parallax {
    layer: Layer,
    /// 0 = fixed to the screen, 1 = fixed to the world.
    factor: f32,
}

pub fn spawn_backgrounds(
    mut commands: Commands,
    assets: Res<GameAssets>,
    surface: Res<SurfaceLine>,
    mut images: ResMut<Assets<Image>>,
) {
    for layer in Layer::ALL {
        let (far, near) = &assets.backgrounds[layer.index()];
        for (image, factor, z) in [(far, 0.25, -6.0), (near, 0.5, -5.0)] {
            commands.spawn((
                Parallax { layer, factor },
                InGameEntity,
                Sprite { image: image.clone(), custom_size: Some(Vec2::ONE), rect: Some(Rect::new(0.0, 0.0, 1.0, 1.0)), ..default() },
                Transform::from_xyz(0.0, 0.0, z),
                Visibility::Hidden,
            ));
        }
    }

    // Sky above the original surface, hiding the crust backgrounds there.
    let width = surface.0.len();
    let height = surface.0.iter().copied().max().unwrap_or(0).max(1) as usize + 8;
    let mut data = vec![0u8; width * height * 4];
    for (x, &top) in surface.0.iter().enumerate() {
        for y in 0..(top.max(0) as usize + 3).min(height) {
            let t = y as f32 / height as f32;
            let c = [(118.0 + 40.0 * t) as u8, (160.0 + 30.0 * t) as u8, (218.0 + 10.0 * t) as u8, 255];
            data[(y * width + x) * 4..][..4].copy_from_slice(&c);
        }
    }
    let sky = Image::new(
        Extent3d { width: width as u32, height: height as u32, depth_or_array_layers: 1 },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    let (w, h) = (width as f32, height as f32);
    commands.spawn((
        InGameEntity,
        Sprite { image: images.add(sky), custom_size: Some(Vec2::new(w, h)), ..default() },
        Transform::from_xyz(w / 2.0, -h / 2.0, -4.0),
    ));
}

pub fn update_backgrounds(
    camera: Single<&Transform, (With<WorldCamera>, Without<Parallax>)>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut q: Query<(&Parallax, &mut Sprite, &mut Transform, &mut Visibility)>,
) {
    let cam = Vec2::new(camera.translation.x, -camera.translation.y);
    let half = Vec2::new(window.width(), window.height()) / PIXEL_SCALE / 2.0 + 4.0;
    let (top, bottom) = (cam.y - half.y, cam.y + half.y);
    for (p, mut sprite, mut tf, mut vis) in &mut q {
        let band_top = top.max(p.layer.top() as f32);
        let band_bottom = bottom.min(p.layer.bottom() as f32);
        if band_bottom <= band_top {
            *vis = Visibility::Hidden;
            continue;
        }
        *vis = Visibility::Visible;
        let size = Vec2::new(half.x * 2.0, band_bottom - band_top);
        let left = cam.x - half.x;
        // Texture coordinates scroll at `factor` of the world's speed.
        let u = (left - cam.x * (1.0 - p.factor)).rem_euclid(256.0);
        let v = (band_top - cam.y * (1.0 - p.factor)).rem_euclid(256.0);
        sprite.custom_size = Some(size);
        sprite.rect = Some(Rect::new(u, v, u + size.x, v + size.y));
        tf.translation.x = cam.x;
        tf.translation.y = -(band_top + size.y / 2.0);
    }
}
