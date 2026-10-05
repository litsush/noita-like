//! Lighting: caves are dark and the surface follows the sun. A half-resolution
//! light map around the camera is seeded from daylight above the ground,
//! glowing materials, dome interiors, lamps, suit lights and laser bolts, then
//! spread by directional sweeps where rock blocks light far more than air.
//! The result is drawn as a smooth darkness overlay tinted by the light.

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use sbct_sim::colony::geom::V2;
use sbct_sim::colony::items::{DomeKind, MachineKind};
use sbct_sim::colony::{EntKind, WeatherKind, domes};
use sbct_sim::planetgen::Band;
use sbct_sim::{Kind, Material, World};

use crate::render::{InGameEntity, WorldCamera, z};
use crate::session::Session;

/// Cells per light texel.
const SCALE: i32 = 2;
/// Light map size in texels (covers more than a 1080p view at 3x).
const W: usize = 360;
const H: usize = 220;
const AIR: f32 = 0.9;
const LIQUID: f32 = 0.8;
const SOLID: f32 = 0.45;

#[derive(Resource)]
pub struct LightMap {
    image: Handle<Image>,
    light: Vec<[f32; 3]>,
    decay: Vec<f32>,
}

/// Extra lights for this frame, added by other systems (in cell coordinates).
#[derive(Resource, Default)]
pub struct PointLights(pub Vec<(Vec2, [f32; 3])>);

#[derive(Component)]
pub struct LightOverlay;

pub fn setup_lighting(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let mut image = Image::new_fill(
        Extent3d {
            width: W as u32,
            height: H as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        &[0, 0, 0, 255],
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::linear();
    let handle = images.add(image);
    commands.spawn((
        LightOverlay,
        InGameEntity,
        Sprite {
            image: handle.clone(),
            custom_size: Some(Vec2::new((W as i32 * SCALE) as f32, (H as i32 * SCALE) as f32)),
            ..default()
        },
        Transform::from_xyz(0.0, 0.0, z::LIGHT),
    ));
    commands.insert_resource(LightMap {
        image: handle,
        light: vec![[0.0; 3]; W * H],
        decay: vec![AIR; W * H],
    });
}

fn emission(m: Material) -> [f32; 3] {
    let l = m.props().light;
    [l[0] as f32 / 330.0, l[1] as f32 / 330.0, l[2] as f32 / 330.0]
}

fn texel_material_info(world: &World, cx: i32, cy: i32) -> ([f32; 3], f32) {
    let mut glow = [0.0f32; 3];
    let mut decay = AIR;
    for (dx, dy) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
        let m = world.material(cx + dx, cy + dy);
        let e = emission(m);
        for c in 0..3 {
            glow[c] = glow[c].max(e[c]);
        }
        let d = match m.kind() {
            // Glass lets light through.
            Kind::Solid if m == Material::Glass => AIR,
            Kind::Solid | Kind::Powder => SOLID,
            Kind::Liquid => LIQUID,
            _ => AIR,
        };
        decay = decay.min(d);
    }
    (glow, decay)
}

/// The colour of the sky's light for a time of day.
pub fn sun_color(daylight: f32, time: f32) -> [f32; 3] {
    use sbct_sim::colony::DAYLIGHT_SECS;
    // Warm at dawn and dusk, cool at night.
    let edge = (time.min((DAYLIGHT_SECS - time).abs()) / 60.0).clamp(0.0, 1.0);
    if time < DAYLIGHT_SECS {
        let warm = 1.0 - edge;
        [
            daylight * 1.2,
            daylight * (1.2 - 0.25 * warm),
            daylight * (1.15 - 0.5 * warm),
        ]
    } else {
        [daylight * 0.75, daylight * 0.95, daylight * 1.5]
    }
}

pub fn update_lighting(
    session: Res<Session>,
    mut map: ResMut<LightMap>,
    mut images: ResMut<Assets<Image>>,
    mut points: ResMut<PointLights>,
    camera: Single<&Transform, (With<WorldCamera>, Without<LightOverlay>)>,
    mut overlay: Single<&mut Transform, With<LightOverlay>>,
) {
    let (Some(world), Some(colony)) = (&session.world, &session.colony) else {
        return;
    };
    let cam = Vec2::new(camera.translation.x, -camera.translation.y);
    let origin = IVec2::new(
        ((cam.x as i32 - (W as i32 * SCALE) / 2) / SCALE) * SCALE,
        ((cam.y as i32 - (H as i32 * SCALE) / 2) / SCALE) * SCALE,
    );
    let to_texel = |p: Vec2| -> Option<usize> {
        let t = ((p.as_ivec2() - origin) / SCALE).as_uvec2();
        let (x, y) = (t.x as usize, t.y as usize);
        (p.x >= origin.x as f32 && p.y >= origin.y as f32 && x < W && y < H).then_some(y * W + x)
    };

    let weather_dim = match colony.weather.kind {
        WeatherKind::Clear => 1.0,
        WeatherKind::Mist => 0.8,
        WeatherKind::Rain => 0.68,
    };
    let sun = sun_color(colony.clock.daylight() * weather_dim, colony.clock.time);
    // Domes in view light their interiors.
    let view = sbct_sim::colony::geom::Rect::new(
        origin.x,
        origin.y,
        origin.x + W as i32 * SCALE,
        origin.y + H as i32 * SCALE,
    );
    let lit_domes: Vec<(DomeKind, V2)> = colony
        .domes()
        .filter(|(e, d)| domes::footprint(d.kind, e.pos).overlaps(&view))
        .map(|(e, d)| (d.kind, e.pos))
        .collect();

    let map = &mut *map;
    for ty in 0..H {
        for tx in 0..W {
            let (cx, cy) = (origin.x + tx as i32 * SCALE, origin.y + ty as i32 * SCALE);
            let i = ty * W + tx;
            let sky = cy < colony.profile.surface_at(cx) && world.material(cx, cy).is_open();
            let (glow, decay) = texel_material_info(world, cx, cy);
            let mut light = if sky { sun } else { glow };
            let p = V2 {
                x: cx as f32 + 0.5,
                y: cy as f32 + 0.5,
            };
            for &(kind, pos) in &lit_domes {
                if domes::contains(kind, pos, p) {
                    let lamp = if kind == DomeKind::Dark {
                        [0.3, 0.2, 0.42]
                    } else {
                        [1.0, 0.94, 0.82]
                    };
                    for c in 0..3 {
                        light[c] = light[c].max(lamp[c]);
                    }
                }
            }
            map.light[i] = light;
            map.decay[i] = decay;
        }
    }

    // Point lights.
    let mut add = |p: Vec2, c: [f32; 3]| {
        if let Some(i) = to_texel(p) {
            for (l, c) in map.light[i].iter_mut().zip(c) {
                *l = l.max(c);
            }
        }
    };
    for p in colony.online() {
        if p.alive() {
            let head = p.body().head();
            let lamp = if p.suit { 2.2 } else { 1.2 };
            add(
                Vec2::new(head.x + p.pose.facing as f32 * 2.0, head.y),
                [lamp * 0.9, lamp, lamp],
            );
            add(Vec2::new(p.center().x, p.center().y), [0.8, 0.85, 0.85]);
        }
    }
    for e in colony.ents.values() {
        let at = Vec2::new(e.pos.x, e.pos.y);
        match &e.kind {
            EntKind::Bolt(b) if !b.web => add(at, [0.6, 1.5, 1.7]),
            EntKind::Machine(m) => match m.kind {
                MachineKind::Lamp => add(at + Vec2::new(0.0, -22.0), [3.2, 2.9, 2.2]),
                MachineKind::Pylon if m.store > 0.0 => add(at + Vec2::new(0.0, -20.0), [0.9, 1.5, 0.7]),
                _ => {}
            },
            EntKind::Drop(_) => add(at, [0.25, 0.3, 0.28]),
            EntKind::Creature(c) => {
                use sbct_sim::colony::creatures::CreatureKind::*;
                let glow = match c.kind {
                    Driftmoth => [0.3, 0.7, 0.6],
                    Webspinner => [0.25, 0.5, 0.15],
                    GloomLeech => [0.15, 0.3, 0.45],
                    BroodMother => [0.5, 0.2, 0.4],
                    _ => continue,
                };
                add(at + Vec2::new(0.0, -5.0), glow);
            }
            _ => {}
        }
    }
    for (p, c) in points.0.drain(..) {
        add(p, c);
    }

    // Spread: two rounds of four directional sweeps.
    for _ in 0..2 {
        for ty in 0..H {
            let row = ty * W;
            for tx in 1..W {
                spread(map, row + tx - 1, row + tx);
            }
            for tx in (0..W - 1).rev() {
                spread(map, row + tx + 1, row + tx);
            }
        }
        for tx in 0..W {
            for ty in 1..H {
                spread(map, (ty - 1) * W + tx, ty * W + tx);
            }
            for ty in (0..H - 1).rev() {
                spread(map, (ty + 1) * W + tx, ty * W + tx);
            }
        }
    }

    // Write the overlay. The ambient floor depends on how deep the camera is.
    let amb: f32 = match colony.profile.band(cam.x as i32, cam.y as i32) {
        Band::Sky | Band::Shallows => 0.07,
        Band::Deeps => 0.035,
        Band::Abyss => 0.045,
    };
    // Night Vision lifts the darkness; a Floodlight washes the screen.
    let stats = session.player().map(|p| p.stats()).unwrap_or_default();
    let amb = amb
        .max(stats.night_vision * 0.9)
        .max(if stats.headlamp >= 3 { 0.7 } else { 0.0 });
    let Some(mut image) = images.get_mut(&map.image) else {
        return;
    };
    let Some(data) = image.data.as_mut() else { return };
    for (i, l) in map.light.iter().enumerate() {
        let lum = l[0].max(l[1]).max(l[2]).max(amb);
        let dark = (1.0 - lum.min(1.0)).powf(1.25);
        let tint = if lum > 0.01 {
            [l[0] / lum, l[1] / lum, l[2] / lum]
        } else {
            [0.0; 3]
        };
        let k = 0.16 * (1.0 - dark);
        data[i * 4] = (tint[0] * k * 255.0) as u8;
        data[i * 4 + 1] = (tint[1] * k * 255.0) as u8;
        data[i * 4 + 2] = (tint[2] * k * 255.0) as u8;
        data[i * 4 + 3] = (dark * 255.0) as u8;
    }
    let size = Vec2::new((W as i32 * SCALE) as f32, (H as i32 * SCALE) as f32);
    let center = origin.as_vec2() + size / 2.0;
    overlay.translation = Vec3::new(center.x, -center.y, z::LIGHT);
}

#[inline]
fn spread(map: &mut LightMap, from: usize, to: usize) {
    let d = map.decay[to];
    let src = map.light[from];
    let dst = &mut map.light[to];
    for k in 0..3 {
        let v = src[k] * d;
        if v > dst[k] {
            dst[k] = v;
        }
    }
}
