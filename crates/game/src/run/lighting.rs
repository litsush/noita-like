//! Lighting: deep caves are dark. A half-resolution light map around the
//! camera is seeded from glowing materials, daylight above the surface, the
//! headlamp, torches, pocket suns and the core, then spread by directional
//! sweeps (Terraria style) where rock blocks light far more than air. The
//! result is drawn as a smooth darkness overlay tinted by the light colour.

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;
use sbct_sim::descent::Layer;
use sbct_sim::{Kind, Material, World};

use super::creatures::Creature;
use super::entities::{LightOrb, Projectile, Prop, PropKind, ShardPickup, Sun};
use super::player::RunPlayer;
use super::scrolls::ScrollId;
use super::{Phase, Run};
use crate::render::{InGameEntity, PIXEL_SCALE, WorldCamera};
use crate::session::Session;

/// Cells per light texel.
const SCALE: i32 = 2;
/// Light map size in texels (covers more than a 1080p view at 3x).
const W: usize = 360;
const H: usize = 220;
const AIR: f32 = 0.9;
const LIQUID: f32 = 0.78;
const SOLID: f32 = 0.42;

/// Top of the original ground per column; everything above gets daylight.
#[derive(Resource)]
pub struct SurfaceLine(pub Vec<i32>);

#[derive(Resource)]
pub struct LightMap {
    image: Handle<Image>,
    light: Vec<[f32; 3]>,
    decay: Vec<f32>,
}

#[derive(Component)]
pub struct LightOverlay;

pub fn setup_lighting(mut commands: Commands, session: Res<Session>, mut images: ResMut<Assets<Image>>) {
    let Some(world) = &session.world else { return };
    let surface = (0..world.width() as i32)
        .map(|x| {
            (0..world.height() as i32)
                .find(|&y| {
                    matches!(
                        world.material(x, y),
                        Material::Dirt | Material::Stone | Material::Sand | Material::Gravel
                    )
                })
                .unwrap_or(0)
        })
        .collect();
    commands.insert_resource(SurfaceLine(surface));

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
        Transform::from_xyz(0.0, 0.0, 8.0),
    ));
    commands.insert_resource(LightMap {
        image: handle,
        light: vec![[0.0; 3]; W * H],
        decay: vec![AIR; W * H],
    });
}

fn ambient(layer: Layer) -> f32 {
    [0.10, 0.04, 0.025, 0.04, 0.06][layer.index()]
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
            Kind::Solid | Kind::Powder => SOLID,
            Kind::Liquid => LIQUID,
            _ => AIR,
        };
        decay = decay.min(d);
    }
    (glow, decay)
}

pub fn update_lighting(
    session: Res<Session>,
    run: Res<Run>,
    player: Res<RunPlayer>,
    surface: Res<SurfaceLine>,
    mut map: ResMut<LightMap>,
    mut images: ResMut<Assets<Image>>,
    camera: Single<&Transform, (With<WorldCamera>, Without<LightOverlay>)>,
    mut overlay: Single<&mut Transform, With<LightOverlay>>,
    window: Single<&Window, With<PrimaryWindow>>,
    orbs: Query<&LightOrb>,
    shards: Query<&ShardPickup>,
    suns: Query<&Sun>,
    props: Query<&Prop>,
    projectiles: Query<&Projectile>,
    creatures: Query<&Creature>,
) {
    let Some(world) = &session.world else { return };
    let cam = Vec2::new(camera.translation.x, -camera.translation.y);
    let view = Vec2::new(window.width(), window.height()) / PIXEL_SCALE;
    let _ = view;
    let origin = IVec2::new(
        ((cam.x as i32 - (W as i32 * SCALE) / 2) / SCALE) * SCALE,
        ((cam.y as i32 - (H as i32 * SCALE) / 2) / SCALE) * SCALE,
    );
    let to_texel = |p: Vec2| -> Option<usize> {
        let t = ((p.as_ivec2() - origin) / SCALE).as_uvec2();
        let (x, y) = (t.x as usize, t.y as usize);
        (p.x >= origin.x as f32 && p.y >= origin.y as f32 && x < W && y < H).then_some(y * W + x)
    };

    let map = &mut *map;
    // Seed: daylight, glowing cells.
    for ty in 0..H {
        for tx in 0..W {
            let (cx, cy) = (origin.x + tx as i32 * SCALE, origin.y + ty as i32 * SCALE);
            let i = ty * W + tx;
            let sky = cx >= 0
                && (cx as usize) < surface.0.len()
                && cy < surface.0[cx as usize]
                && world.material(cx, cy).is_open();
            let (glow, decay) = texel_material_info(world, cx, cy);
            map.light[i] = if sky { [1.2, 1.2, 1.15] } else { glow };
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
    let dying = matches!(run.phase, Phase::Dying(_)) || (run.phase == Phase::Over && !run.won);
    if let Some((_, to)) = player.beam {
        add(to, [0.9, 0.8, 1.3]);
    }
    if !dying {
        let lamp = if run.has(ScrollId::Beacon) {
            7.0
        } else if run.staff_dimmed {
            0.55
        } else {
            2.4
        };
        add(
            player.body.head() + Vec2::new(player.facing * 2.0, 0.0),
            [lamp, lamp * 0.95, lamp * 0.82],
        );
        add(player.body.center(), [0.9, 0.85, 0.75]);
    }
    for o in &orbs {
        let b = o.brightness();
        add(o.pos, [2.2 * b, 2.1 * b, 1.8 * b]);
    }
    for s in &shards {
        add(s.pos, [0.25, 0.7, 0.8]);
    }
    for s in &suns {
        add(s.pos, [4.0, 3.6, 2.4]);
    }
    for p in &projectiles {
        let [r, g, b] = p.spell.school().map_or([200, 200, 220], |s| s.color());
        add(p.pos, [r as f32 / 160.0, g as f32 / 160.0, b as f32 / 160.0]);
    }
    for prop in &props {
        if prop.kind == PropKind::Core {
            add(prop.center(), [6.0, 3.5, 1.5]);
        }
    }
    for c in &creatures {
        use super::creatures::Beast;
        let glow = match c.kind {
            Beast::CinderWraith => [1.0, 0.45, 0.1],
            Beast::SporeDrifter | Beast::SporePuppet => [0.3, 0.9, 0.4],
            Beast::Lightseeker => [0.25, 0.25, 0.35],
            Beast::Hollowed => [0.3, 0.2, 0.45],
            _ => continue,
        };
        add(c.body.center(), glow);
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

    // Write the overlay.
    let amb = ambient(run.layer);
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
        let k = 0.18 * (1.0 - dark);
        data[i * 4] = (tint[0] * k * 255.0) as u8;
        data[i * 4 + 1] = (tint[1] * k * 255.0) as u8;
        data[i * 4 + 2] = (tint[2] * k * 255.0) as u8;
        data[i * 4 + 3] = (dark * 255.0) as u8;
    }
    let size = Vec2::new((W as i32 * SCALE) as f32, (H as i32 * SCALE) as f32);
    let center = origin.as_vec2() + size / 2.0;
    overlay.translation = Vec3::new(center.x, -center.y, 8.0);
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
