//! Copper pipes: drawing the tiles (with water visibly moving through
//! networks that flow), and laying or taking up runs by dragging.

use std::collections::{BTreeSet, HashMap};

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use sbct_sim::colony::actions::Action as Act;
use sbct_sim::colony::items::Item;
use sbct_sim::colony::water::{MAX_RUN, TILE, Tile, run, tile_center, tile_of};

use crate::audio::Sfx;
use crate::controls::Action;
use crate::player::LocalPlayer;
use crate::render::{InGameEntity, UiHasPointer, z};
use crate::session::Session;
use crate::settings::Config;

#[derive(Component)]
pub struct PipeSprite;

/// A tile of the run being dragged out.
#[derive(Component)]
pub struct PipeGhost;

#[derive(Resource)]
pub struct PipeView {
    sheet: Handle<Image>,
    layout: Handle<TextureAtlasLayout>,
    sprites: HashMap<Tile, Entity>,
    known: BTreeSet<Tile>,
    /// Tiles in networks where water is moving.
    flowing: BTreeSet<Tile>,
    refresh: f32,
    ghosts: Vec<Entity>,
}

/// The run being dragged: start tile and whether it removes pipe.
#[derive(Resource, Default)]
pub struct PipeDrag {
    pub from: Option<(Tile, bool)>,
    /// Tiles the release would lay or remove, and whether the player can afford it.
    pub preview: Option<(usize, bool)>,
}

const VARIANTS: usize = 16;
const FRAMES: usize = 2;

/// Draws the pipe atlas: one 4×4 tile per connection mask (bit 0 up, 1 right,
/// 2 down, 3 left), in two frames whose highlight alternates for flow.
fn atlas() -> Vec<u8> {
    let w = VARIANTS * TILE as usize;
    let h = FRAMES * TILE as usize;
    let mut px = vec![0u8; w * h * 4];
    let copper = [[150, 86, 54, 255], [206, 126, 74, 255], [240, 176, 120, 255]];
    let water = [96, 190, 236, 255];
    for frame in 0..FRAMES {
        for mask in 0..VARIANTS {
            let mut set = |x: usize, y: usize, c: [u8; 4]| {
                let o = ((frame * 4 + y) * w + mask * 4 + x) * 4;
                px[o..o + 4].copy_from_slice(&c);
            };
            // A 2×2 hub with arms toward each connected side.
            let arms = [
                (mask & 1 != 0, 0usize),
                (mask & 2 != 0, 1),
                (mask & 4 != 0, 2),
                (mask & 8 != 0, 3),
            ];
            for (y, x) in [(1, 1), (1, 2), (2, 1), (2, 2)] {
                set(
                    x,
                    y,
                    if (x + y) % 2 == frame {
                        copper[2]
                    } else {
                        copper[1]
                    },
                );
            }
            for (on, side) in arms {
                if !on {
                    continue;
                }
                let cells = match side {
                    0 => [(1, 0), (2, 0)],
                    1 => [(3, 1), (3, 2)],
                    2 => [(1, 3), (2, 3)],
                    _ => [(0, 1), (0, 2)],
                };
                set(cells[0].0, cells[0].1, copper[1]);
                set(cells[1].0, cells[1].1, copper[0]);
            }
            // An end cap when nothing connects: a darker rim.
            if mask == 0 {
                set(1, 1, copper[0]);
            }
            // The second frame's glint reads as water moving.
            if frame == 1 {
                set(1, 1, water);
            }
        }
    }
    px
}

pub fn setup_pipes(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let mut image = Image::new(
        Extent3d {
            width: (VARIANTS * TILE as usize) as u32,
            height: (FRAMES * TILE as usize) as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        atlas(),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::nearest();
    commands.insert_resource(PipeView {
        sheet: images.add(image),
        layout: layouts.add(TextureAtlasLayout::from_grid(
            UVec2::splat(TILE as u32),
            VARIANTS as u32,
            FRAMES as u32,
            None,
            None,
        )),
        sprites: HashMap::new(),
        known: BTreeSet::new(),
        flowing: BTreeSet::new(),
        refresh: 0.0,
        ghosts: Vec::new(),
    });
    commands.insert_resource(PipeDrag::default());
}

fn mask(pipes: &BTreeSet<Tile>, t: Tile) -> usize {
    (pipes.contains(&(t.0, t.1 - 1)) as usize)
        | (pipes.contains(&(t.0 + 1, t.1)) as usize) << 1
        | (pipes.contains(&(t.0, t.1 + 1)) as usize) << 2
        | (pipes.contains(&(t.0 - 1, t.1)) as usize) << 3
}

fn tile_transform(t: Tile, zed: f32) -> Transform {
    let c = tile_center(t);
    Transform::from_xyz(c.x, -c.y, zed)
}

pub fn sync_pipes(
    mut commands: Commands,
    session: Res<Session>,
    time: Res<Time>,
    mut view: ResMut<PipeView>,
    mut sprites: Query<&mut Sprite, With<PipeSprite>>,
) {
    let Some(colony) = &session.colony else { return };
    let view = &mut *view;
    if colony.pipes != view.known {
        // Add and remove sprites, then fix the joints of everything nearby.
        let mut touched: BTreeSet<Tile> = BTreeSet::new();
        for &t in colony.pipes.symmetric_difference(&view.known) {
            touched.extend([t, (t.0 + 1, t.1), (t.0 - 1, t.1), (t.0, t.1 + 1), (t.0, t.1 - 1)]);
        }
        for &t in view.known.difference(&colony.pipes) {
            if let Some(e) = view.sprites.remove(&t) {
                commands.entity(e).despawn();
            }
        }
        for &t in colony.pipes.difference(&view.known) {
            let sprite = Sprite::from_atlas_image(
                view.sheet.clone(),
                TextureAtlas {
                    layout: view.layout.clone(),
                    index: mask(&colony.pipes, t),
                },
            );
            let e = commands
                .spawn((
                    PipeSprite,
                    InGameEntity,
                    sprite,
                    tile_transform(t, z::PROPS + 0.2),
                ))
                .id();
            view.sprites.insert(t, e);
        }
        view.known = colony.pipes.clone();
        view.refresh = 0.0;
        for t in touched {
            if let Some(&e) = view.sprites.get(&t)
                && let Ok(mut sprite) = sprites.get_mut(e)
                && let Some(atlas) = &mut sprite.texture_atlas
            {
                atlas.index = mask(&colony.pipes, t);
            }
        }
    }

    // Which networks are moving water, checked a couple of times a second.
    view.refresh -= time.delta_secs();
    if view.refresh <= 0.0 {
        view.refresh = 0.5;
        view.flowing = colony
            .networks()
            .into_iter()
            .filter(|n| n.flowing())
            .flat_map(|n| n.tiles)
            .collect();
    }
    let beat = (time.elapsed_secs() * 5.0) as i32;
    for (&t, &e) in &view.sprites {
        let Ok(mut sprite) = sprites.get_mut(e) else {
            continue;
        };
        let Some(atlas) = &mut sprite.texture_atlas else {
            continue;
        };
        let base = atlas.index % VARIANTS;
        // A glint travels along flowing pipes.
        let lit = view.flowing.contains(&t) && (t.0 + t.1 - beat).rem_euclid(4) == 0;
        atlas.index = base + if lit { VARIANTS } else { 0 };
    }
}

/// Dragging with pipe in hand lays a run; dragging with the laser button
/// takes pipe up.
pub fn drag_pipes(
    mut commands: Commands,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    config: Res<Config>,
    ui: Res<UiHasPointer>,
    local: Res<LocalPlayer>,
    mut session: ResMut<Session>,
    mut drag: ResMut<PipeDrag>,
    mut view: ResMut<PipeView>,
    mut sfx: MessageWriter<Sfx>,
) {
    for e in view.ghosts.drain(..) {
        commands.entity(e).despawn();
    }
    drag.preview = None;
    let Some(me) = session.player() else { return };
    let holding = me.held().is_some_and(|s| s.item == Item::Pipe) && me.alive();
    if !holding {
        drag.from = None;
        return;
    }
    let have = me.inv.count(Item::Pipe) as usize;
    let b = &config.bindings;
    let cursor = tile_of(local.cursor);
    if !ui.0 {
        if b.just_pressed(Action::Primary, &keys, &mouse) {
            drag.from = Some((cursor, false));
        } else if b.just_pressed(Action::Fire, &keys, &mouse) {
            drag.from = Some((cursor, true));
        }
    }
    let Some((from, remove)) = drag.from else { return };
    let Some(colony) = &session.colony else { return };
    let tiles = run(from, cursor);
    let count = tiles
        .iter()
        .filter(|t| colony.pipes.contains(t) == remove)
        .count();
    let ok = tiles.len() <= MAX_RUN && (remove || count <= have);
    drag.preview = Some((count, ok));
    for &t in tiles.iter().take(MAX_RUN + 8) {
        let color = match (remove, ok) {
            (true, _) => Color::srgba(1.0, 0.4, 0.3, 0.55),
            (false, true) => Color::srgba(0.5, 1.0, 0.7, 0.6),
            (false, false) => Color::srgba(1.0, 0.75, 0.3, 0.5),
        };
        let e = commands
            .spawn((
                PipeGhost,
                InGameEntity,
                Sprite::from_color(color, Vec2::splat(TILE as f32 - 1.0)),
                tile_transform(t, z::OVERLAY),
            ))
            .id();
        view.ghosts.push(e);
    }
    let released = if remove {
        !b.pressed(Action::Fire, &keys, &mouse)
    } else {
        !b.pressed(Action::Primary, &keys, &mouse)
    };
    if released {
        drag.from = None;
        if count > 0 {
            session.act(Act::Pipe {
                from,
                to: cursor,
                remove,
            });
            sfx.write(Sfx::ui("pipe_place"));
        }
    }
}

pub fn reset_pipes(mut view: ResMut<PipeView>) {
    view.sprites.clear();
    view.known.clear();
    view.ghosts.clear();
}
