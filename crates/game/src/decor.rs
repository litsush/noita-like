//! Decorative foliage: grass tufts, ferns, reeds, hanging vines and cave
//! fungi dressed onto whatever terrain is on screen. Nothing here is game
//! state: each plant is chosen by a hash of the world seed and its cell, so
//! every player sees the same dressing, and it disappears when the ground
//! under it is dug away.

use bevy::prelude::*;
use sbct_sim::colony::geom::v2;
use sbct_sim::planetgen::Band;
use sbct_sim::rng::hash2;
use sbct_sim::{Kind, Material};

use crate::assets::GameAssets;
use crate::render::{InGameEntity, WorldCamera, z};
use crate::session::Session;

#[derive(Component)]
pub struct Decor;

const POOL: usize = 320;
/// Columns are considered every this many cells.
const STEP: i32 = 4;

fn soil(m: Material) -> bool {
    matches!(
        m,
        Material::Grass | Material::Dirt | Material::Moss | Material::Fungus | Material::Sand
    )
}

fn rock(m: Material) -> bool {
    matches!(m, Material::Stone | Material::Basalt | Material::Gravel) || soil(m)
}

pub fn dress_terrain(
    mut commands: Commands,
    session: Res<Session>,
    assets: Res<GameAssets>,
    time: Res<Time>,
    camera: Single<(&Transform, &Projection), With<WorldCamera>>,
    window: Single<&Window, With<bevy::window::PrimaryWindow>>,
    mut pool: Query<(&mut Sprite, &mut Transform, &mut Visibility), (With<Decor>, Without<WorldCamera>)>,
) {
    let (Some(world), Some(colony)) = (&session.world, &session.colony) else {
        return;
    };
    if pool.is_empty() {
        for _ in 0..POOL {
            commands.spawn((
                Decor,
                InGameEntity,
                assets.foliage.sprite(0),
                Transform::from_xyz(0.0, 0.0, z::PROPS - 0.5),
                Visibility::Hidden,
            ));
        }
        return;
    }
    let (cam_tf, projection) = *camera;
    let scale = match projection {
        Projection::Orthographic(o) => o.scale,
        _ => 1.0,
    };
    let half = Vec2::new(window.width(), window.height()) * scale / 2.0 + 12.0;
    let (cx, cy) = (cam_tf.translation.x, -cam_tf.translation.y);
    let (x0, x1) = ((cx - half.x) as i32, (cx + half.x) as i32);
    let (y0, y1) = ((cy - half.y) as i32, (cy + half.y) as i32);
    let seed = colony.seed ^ 0xF011A6E;
    let domes = colony.dome_rects();
    let t = time.elapsed_secs();
    let mut sprites = pool.iter_mut();
    let exhausted = std::cell::Cell::new(false);
    let mut place = |x: i32, y: i32, row: usize, col: usize, hang: bool| {
        let Some((mut sprite, mut tf, mut vis)) = sprites.next() else {
            exhausted.set(true);
            return;
        };
        if let Some(atlas) = &mut sprite.texture_atlas {
            atlas.index = row * 8 + col;
        }
        sprite.flip_x = hash2(seed, x, y) & 1 == 0;
        // Ground plants stand on the cell; hanging ones dangle from it.
        let (px, py) = (x as f32 + 0.5, if hang { y as f32 + 8.0 } else { y as f32 - 8.0 });
        let sway = (t * 1.3 + x as f32 * 0.37).sin() * 0.05;
        tf.translation = Vec3::new(px, -py, z::PROPS - 0.5);
        tf.rotation = Quat::from_rotation_z(if hang { -sway } else { sway });
        *vis = Visibility::Visible;
    };

    let mut x = x0.div_euclid(STEP) * STEP;
    'columns: while x <= x1 {
        let column = hash2(seed, x, 7) % 100;
        if column < 62 {
            let mut y = y0.max(2);
            while y <= y1 {
                let here = world.material(x, y);
                let above = world.material(x, y - 1);
                let h = hash2(seed, x, y) % 1000;
                let inside = domes.iter().any(|r| r.grown(2).contains(v2(x as f32, y as f32)));
                if inside {
                    y += 1;
                    continue;
                }
                let open = |m: Material| m == Material::Empty || m.kind() == Kind::Gas;
                // A floor: something to root in with air (or shallow water) over it.
                if rock(here) && (open(above) || above == Material::Water) && open(world.material(x, y - 6)) {
                    let band = colony.profile.band(x, y);
                    let surface = y <= colony.profile.surface_at(x) + 6;
                    let near_water = (-5..=5).any(|dx| world.material(x + dx, y - 1) == Material::Water);
                    let pick = (h / 8) as usize % 8;
                    if above == Material::Water {
                        // Lilies and reeds only in the shallows.
                        if open(world.material(x, y - 4)) && h < 420 {
                            place(x, y, 4, pick, false);
                        }
                    } else if near_water && soil(here) {
                        if h < 520 {
                            place(x, y, 4, [0, 1, 3, 4, 6][pick % 5], false);
                        }
                    } else if surface && soil(here) {
                        // Mostly grass, with the odd fern or flower.
                        if h < 560 {
                            place(x, y, 0, pick, false);
                        } else if h < 700 {
                            place(x, y, 1, pick, false);
                        }
                    } else if !surface && h < 150 {
                        let row = if matches!(band, Band::Deeps | Band::Abyss) {
                            5
                        } else {
                            3
                        };
                        place(x, y, row, pick, false);
                    }
                    if exhausted.get() {
                        break 'columns;
                    }
                    // Skip the body of this floor.
                    y += 3;
                    continue;
                }
                // A ceiling: vines and roots hang where there is room.
                if rock(above) && open(here) && open(world.material(x, y + 12)) && h < 60 {
                    place(x, y, 2, (h as usize / 7) % 8, true);
                    y += 12;
                    continue;
                }
                y += 1;
            }
        }
        x += STEP;
    }
    for (_, _, mut vis) in sprites {
        *vis = Visibility::Hidden;
    }
}
