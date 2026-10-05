//! Everything behind and above the terrain: the sky and its day/night
//! colours, distant overgrown ruins and forest on the horizon, cave
//! backdrops with parallax, rain and drifting mist.

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;
use sbct_sim::colony::{DAY_SECS, DAYLIGHT_SECS, WeatherKind};
use sbct_sim::rng::{Rng, hash2};

use crate::assets::GameAssets;
use crate::audio::Sfx;
use crate::render::{InGameEntity, PIXEL_SCALE, WorldCamera, z};
use crate::session::Session;

const SKY_ROWS: usize = 64;
const RAIN_DROPS: usize = 220;

#[derive(Component)]
pub struct SkyQuad(Handle<Image>);

/// A tiling silhouette layer on the horizon.
#[derive(Component)]
pub struct Horizon {
    /// 0 = fixed to the screen, 1 = fixed to the world.
    factor: f32,
    /// Texture height in cells.
    height: f32,
}

/// A tiling cave backdrop shown between two rows of the world.
#[derive(Component)]
pub struct CaveLayer {
    top: f32,
    bottom: f32,
    factor: f32,
}

#[derive(Component)]
pub struct MistLayer {
    speed: f32,
    scale: f32,
    /// 0 is the layer itself; 1–4 are thinner and thinner strips under it,
    /// so the mist fades into the ground instead of ending on a line.
    strip: u8,
}

/// Height in cells of each fading strip under the mist.
const MIST_STRIP: f32 = 2.0;
const MIST_STRIPS: u8 = 12;

#[derive(Component)]
pub struct RainDrop {
    pos: Vec2,
    speed: f32,
}

#[derive(Resource)]
pub struct SkyState {
    rng: Rng,
    /// 0–1, eases toward the weather so rain and mist fade in and out.
    pub rain: f32,
    pub mist: f32,
    /// Row where the baked ground backdrop ends and cave layers begin.
    ground_bottom: f32,
}

type Rgb = [f32; 3];

fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// Sky colours (top, horizon) for a time of day.
pub fn sky_colors(time: f32) -> (Rgb, Rgb) {
    const NIGHT: (Rgb, Rgb) = ([0.02, 0.03, 0.08], [0.06, 0.10, 0.16]);
    const DAWN: (Rgb, Rgb) = ([0.20, 0.24, 0.40], [0.86, 0.52, 0.40]);
    const DAY: (Rgb, Rgb) = ([0.30, 0.50, 0.62], [0.62, 0.78, 0.74]);
    let blend = |a: (Rgb, Rgb), b: (Rgb, Rgb), t: f32| (mix(a.0, b.0, t), mix(a.1, b.1, t));
    let t = time.rem_euclid(DAY_SECS);
    let edge = 50.0;
    if t < edge {
        blend(DAWN, DAY, t / edge)
    } else if t < DAYLIGHT_SECS - edge {
        DAY
    } else if t < DAYLIGHT_SECS {
        blend(DAY, DAWN, (t - (DAYLIGHT_SECS - edge)) / edge)
    } else if t < DAYLIGHT_SECS + edge {
        blend(DAWN, NIGHT, (t - DAYLIGHT_SECS) / edge)
    } else if t > DAY_SECS - edge {
        blend(NIGHT, DAWN, (t - (DAY_SECS - edge)) / edge)
    } else {
        NIGHT
    }
}

pub fn spawn_sky(
    mut commands: Commands,
    session: Res<Session>,
    assets: Res<GameAssets>,
    mut images: ResMut<Assets<Image>>,
) {
    let (Some(world), Some(colony)) = (&session.world, &session.colony) else {
        return;
    };
    let profile = &colony.profile;

    // The sky: a vertical gradient that fills the view.
    let mut gradient = Image::new_fill(
        Extent3d {
            width: 1,
            height: SKY_ROWS as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    gradient.sampler = ImageSampler::linear();
    let gradient = images.add(gradient);
    commands.spawn((
        SkyQuad(gradient.clone()),
        InGameEntity,
        Sprite {
            image: gradient,
            custom_size: Some(Vec2::ONE),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, z::SKY),
    ));

    for (image, factor, zed) in [
        (&assets.surface_far, 0.12, z::HORIZON_FAR),
        (&assets.surface_mid, 0.3, z::HORIZON_MID),
    ] {
        commands.spawn((
            Horizon {
                factor,
                height: 256.0,
            },
            InGameEntity,
            Sprite {
                image: image.clone(),
                custom_size: Some(Vec2::ONE),
                rect: Some(Rect::new(0.0, 0.0, 1.0, 1.0)),
                ..default()
            },
            Transform::from_xyz(0.0, 0.0, zed),
        ));
    }

    // Dark rock behind everything below the original surface, so dug-out
    // ground reads as underground and hides the horizon. Baked for the rows
    // the surface line passes through; cave layers take over below.
    let width = profile.width as usize;
    let top = profile.surface.iter().copied().min().unwrap_or(0).max(0) as usize;
    let bottom = (profile.surface.iter().copied().max().unwrap_or(0) as usize + 48).min(world.height());
    let rows = bottom - top;
    let mut data = vec![0u8; width * rows * 4];
    for x in 0..width {
        let surface = profile.surface[x].max(0) as usize;
        for y in surface.max(top)..bottom {
            let n = (hash2(world.seed(), x as i32 / 3, y as i32 / 3) % 9) as u8;
            let i = ((y - top) * width + x) * 4;
            data[i..i + 4].copy_from_slice(&[24 + n, 27 + n, 32 + n, 255]);
        }
    }
    let mut ground = Image::new(
        Extent3d {
            width: width as u32,
            height: rows as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    ground.sampler = ImageSampler::nearest();
    commands.spawn((
        InGameEntity,
        Sprite {
            image: images.add(ground),
            custom_size: Some(Vec2::new(width as f32, rows as f32)),
            ..default()
        },
        Transform::from_xyz(
            width as f32 / 2.0,
            -(top as f32 + rows as f32 / 2.0),
            z::GROUND_BACK,
        ),
    ));

    let bands = [
        (bottom as f32, profile.deeps_y as f32),
        (profile.deeps_y as f32, profile.abyss_y as f32),
        (profile.abyss_y as f32, profile.height as f32),
    ];
    for ((top, bottom), (far, near)) in bands.into_iter().zip(&assets.cave_layers) {
        for (image, factor, zed) in [(far, 0.3, z::CAVE_FAR), (near, 0.55, z::CAVE_NEAR)] {
            commands.spawn((
                CaveLayer { top, bottom, factor },
                InGameEntity,
                Sprite {
                    image: image.clone(),
                    custom_size: Some(Vec2::ONE),
                    rect: Some(Rect::new(0.0, 0.0, 1.0, 1.0)),
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, zed),
                Visibility::Hidden,
            ));
        }
    }

    for (speed, scale) in [(5.0, 1.0), (-3.0, 1.7)] {
        for strip in 0..=MIST_STRIPS {
            commands.spawn((
                MistLayer { speed, scale, strip },
                InGameEntity,
                Sprite {
                    image: assets.mist.clone(),
                    custom_size: Some(Vec2::ONE),
                    rect: Some(Rect::new(0.0, 0.0, 1.0, 1.0)),
                    color: Color::srgba(0.8, 0.9, 0.88, 0.0),
                    ..default()
                },
                Transform::from_xyz(0.0, 0.0, z::MIST),
            ));
        }
    }
    for _ in 0..RAIN_DROPS {
        commands.spawn((
            RainDrop {
                pos: Vec2::ZERO,
                speed: 0.0,
            },
            InGameEntity,
            Sprite::from_color(Color::srgba(0.7, 0.85, 0.95, 0.5), Vec2::new(1.0, 7.0)),
            Transform::from_xyz(0.0, 0.0, z::RAIN),
            Visibility::Hidden,
        ));
    }
    commands.insert_resource(SkyState {
        rng: Rng::new(99),
        rain: 0.0,
        mist: 0.0,
        ground_bottom: bottom as f32,
    });
}

fn view(camera: &Transform, window: &Window, projection: &Projection) -> (Vec2, Vec2) {
    let scale = match projection {
        Projection::Orthographic(o) => o.scale,
        _ => 1.0 / PIXEL_SCALE,
    };
    let cam = Vec2::new(camera.translation.x, -camera.translation.y);
    let half = Vec2::new(window.width(), window.height()) * scale / 2.0 + 4.0;
    (cam, half)
}

pub fn update_sky(
    session: Res<Session>,
    time: Res<Time>,
    mut state: ResMut<SkyState>,
    mut images: ResMut<Assets<Image>>,
    camera: Single<(&Transform, &Projection), With<WorldCamera>>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut sky: Single<
        (&SkyQuad, &mut Sprite, &mut Transform),
        (
            Without<WorldCamera>,
            Without<Horizon>,
            Without<CaveLayer>,
            Without<MistLayer>,
            Without<RainDrop>,
        ),
    >,
    mut horizons: Query<
        (&Horizon, &mut Sprite, &mut Transform),
        (
            Without<WorldCamera>,
            Without<SkyQuad>,
            Without<CaveLayer>,
            Without<MistLayer>,
            Without<RainDrop>,
        ),
    >,
    mut caves: Query<
        (&CaveLayer, &mut Sprite, &mut Transform, &mut Visibility),
        (
            Without<WorldCamera>,
            Without<SkyQuad>,
            Without<Horizon>,
            Without<MistLayer>,
            Without<RainDrop>,
        ),
    >,
    mut mists: Query<
        (&MistLayer, &mut Sprite, &mut Transform),
        (
            Without<WorldCamera>,
            Without<SkyQuad>,
            Without<Horizon>,
            Without<CaveLayer>,
            Without<RainDrop>,
        ),
    >,
    mut rain: Query<
        (&mut RainDrop, &mut Transform, &mut Visibility),
        (
            Without<WorldCamera>,
            Without<SkyQuad>,
            Without<Horizon>,
            Without<CaveLayer>,
            Without<MistLayer>,
        ),
    >,
    mut sfx: MessageWriter<Sfx>,
) {
    let (Some(world), Some(colony)) = (&session.world, &session.colony) else {
        return;
    };
    let dt = time.delta_secs();
    let (cam, half) = view(camera.0, &window, camera.1);
    let weather = colony.weather.kind;
    let ease = |v: &mut f32, on: bool| *v = (*v + if on { dt } else { -dt } * 0.25).clamp(0.0, 1.0);
    ease(&mut state.rain, weather == WeatherKind::Rain);
    ease(&mut state.mist, weather != WeatherKind::Clear);

    // Sky gradient, greyed by the weather.
    let (top, horizon) = sky_colors(colony.clock.time);
    let grey = |c: Rgb| {
        let l = (c[0] + c[1] + c[2]) / 3.0;
        mix(c, [l * 0.8, l * 0.85, l * 0.9], state.mist * 0.7)
    };
    let (top, horizon) = (grey(top), grey(horizon));
    let (quad, sprite, tf) = &mut *sky;
    if let Some(mut image) = images.get_mut(&quad.0)
        && let Some(data) = image.data.as_mut()
    {
        for row in 0..SKY_ROWS {
            let c = mix(top, horizon, (row as f32 / (SKY_ROWS - 1) as f32).powf(1.6));
            let px = [
                (c[0] * 255.0) as u8,
                (c[1] * 255.0) as u8,
                (c[2] * 255.0) as u8,
                255,
            ];
            data[row * 4..row * 4 + 4].copy_from_slice(&px);
        }
    }
    sprite.custom_size = Some(half * 2.0);
    tf.translation.x = cam.x;
    tf.translation.y = -cam.y;

    // Horizon silhouettes: tinted by the light, hazier in mist.
    let light = colony.clock.daylight();
    let base_y = colony.profile.spawn_y() as f32;
    for (h, mut sprite, mut tf) in &mut horizons {
        let haze = mix(horizon, [light * 0.35, light * 0.5, light * 0.5], 0.55 - h.factor);
        sprite.color = Color::srgb(haze[0].min(1.0), haze[1].min(1.0), haze[2].min(1.0));
        // Scroll sideways slower than the world; sit on the horizon line.
        let width = half.x * 2.0;
        let u = (cam.x * h.factor).rem_euclid(512.0);
        sprite.custom_size = Some(Vec2::new(width, h.height));
        sprite.rect = Some(Rect::new(u, 0.0, u + width, h.height));
        let bottom = base_y + 70.0 + (cam.y - base_y) * (1.0 - h.factor) * 0.7 - 40.0 * h.factor;
        tf.translation.x = cam.x;
        tf.translation.y = -(bottom - h.height / 2.0);
    }

    // Cave backdrops: each shows only within its band.
    let (view_top, view_bottom) = (cam.y - half.y, cam.y + half.y);
    for (layer, mut sprite, mut tf, mut vis) in &mut caves {
        let band_top = view_top.max(layer.top);
        let band_bottom = view_bottom.min(layer.bottom);
        if band_bottom <= band_top {
            *vis = Visibility::Hidden;
            continue;
        }
        *vis = Visibility::Visible;
        let size = Vec2::new(half.x * 2.0, band_bottom - band_top);
        let left = cam.x - half.x;
        let u = (left - cam.x * (1.0 - layer.factor)).rem_euclid(256.0);
        let v = (band_top - cam.y * (1.0 - layer.factor)).rem_euclid(256.0);
        sprite.custom_size = Some(size);
        sprite.rect = Some(Rect::new(u, v, u + size.x, v + size.y));
        tf.translation.x = cam.x;
        tf.translation.y = -(band_top + size.y / 2.0);
    }

    // Mist drifts over the surface and thins out underground.
    let depth_fade = 1.0 - ((cam.y - state.ground_bottom) / 200.0).clamp(0.0, 1.0);
    let t = time.elapsed_secs();
    for (m, mut sprite, mut tf) in &mut mists {
        let alpha = state.mist * depth_fade * 0.5 + state.rain * depth_fade * 0.12;
        // Mist hangs over the ground: the layer stops a little below the
        // surface and thins out over a few strips.
        let floor = (cam.y + half.y).min(colony.profile.surface_at(cam.x as i32) as f32 + 12.0);
        let (top, bottom, fade) = if m.strip == 0 {
            (cam.y - half.y, floor, 1.0)
        } else {
            let top = floor + (m.strip - 1) as f32 * MIST_STRIP;
            (
                top,
                top + MIST_STRIP,
                1.0 - m.strip as f32 / (MIST_STRIPS + 1) as f32,
            )
        };
        sprite.color = Color::srgba(0.82, 0.92, 0.9, alpha * fade);
        let size = Vec2::new(half.x * 2.0, (bottom - top).max(1.0));
        let u = (cam.x * 0.8 + t * m.speed).rem_euclid(256.0 * m.scale) / m.scale;
        let v = (top * 0.8).rem_euclid(256.0 * m.scale) / m.scale;
        sprite.custom_size = Some(size);
        sprite.rect = Some(Rect::new(u, v, u + size.x / m.scale, v + size.y / m.scale));
        tf.translation.x = cam.x;
        tf.translation.y = -(top + size.y / 2.0);
    }

    // Rain falls from the top of the view under open sky and stops at the
    // first thing it hits.
    let active = (state.rain * RAIN_DROPS as f32) as usize;
    let mut splashes = 0;
    for (i, (mut drop, mut tf, mut vis)) in rain.iter_mut().enumerate() {
        if i >= active {
            *vis = Visibility::Hidden;
            drop.speed = 0.0;
            continue;
        }
        let respawn = drop.speed == 0.0
            || (drop.pos.x - cam.x).abs() > half.x + 30.0
            || drop.pos.y > cam.y + half.y + 10.0
            || !world.material(drop.pos.x as i32, drop.pos.y as i32).is_open()
            || colony
                .dome_at(sbct_sim::colony::geom::v2(drop.pos.x, drop.pos.y))
                .is_some();
        if respawn {
            if drop.speed > 0.0 && (drop.pos.y - cam.y).abs() < half.y {
                splashes += 1;
            }
            let rng = &mut state.rng;
            let x = cam.x + (rng.next_f32() * 2.0 - 1.0) * (half.x + 20.0);
            let surface = colony.profile.surface_at(x as i32) as f32;
            let top = (cam.y - half.y - rng.next_f32() * 60.0).min(surface - 4.0);
            drop.pos = Vec2::new(x, top);
            drop.speed = 300.0 + rng.next_f32() * 120.0;
            // Nothing falls where the view is entirely underground.
            if top > cam.y + half.y {
                *vis = Visibility::Hidden;
                drop.speed = 0.0;
                continue;
            }
        }
        let fall = Vec2::new(-drop.speed * 0.12, drop.speed) * dt;
        drop.pos += fall;
        *vis = Visibility::Visible;
        tf.translation = Vec3::new(drop.pos.x, -drop.pos.y, z::RAIN);
        tf.rotation = Quat::from_rotation_z(-0.12);
    }
    let _ = (splashes, &mut sfx);
}
