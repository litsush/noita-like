//! Draws the colony's entities (domes and their fixtures, machines, crates,
//! creatures, drops, bolts) from the replicated state. Nothing here changes
//! the colony; it only mirrors it into sprites.

use std::collections::HashMap;

use bevy::prelude::*;
use sbct_sim::colony::creatures::{CreatureKind, State};
use sbct_sim::colony::domes::{Fixture, fixture_pos};
use sbct_sim::colony::items::{DomeKind, Item, MachineKind};
use sbct_sim::colony::{CrateKind, Ent, EntKind, Id};

use crate::assets::{GameAssets, prop};
use crate::render::{InGameEntity, z};
use crate::session::Session;

/// The sprite (or sprite tree) standing in for a colony entity.
#[derive(Component)]
pub struct EntSprite {
    pub id: Id,
    /// Smoothed position in cells.
    shown: Vec2,
    /// When it appeared, to stagger animations.
    born: f32,
}

/// A prop that cycles through its four frames.
#[derive(Component)]
pub struct PropAnim {
    row: usize,
    fps: f32,
}

#[derive(Resource, Default)]
pub struct EntSprites(HashMap<Id, Entity>);

/// Sprite for a row of the props sheet, standing on `base` (cells).
fn prop_sprite(assets: &GameAssets, row: usize, frame: usize) -> Sprite {
    assets.props.sprite(row * 4 + frame)
}

/// Bevy position of a 32×32 prop whose base is at a cell position.
fn prop_at(x: f32, y: f32, zed: f32) -> Transform {
    Transform::from_xyz(x, -(y - 16.0), zed)
}

fn fixture_row(kind: DomeKind, f: Fixture) -> (usize, f32) {
    match f {
        Fixture::Bunk(_) => (prop::BUNK, 0.0),
        Fixture::Chest(_) => match kind {
            DomeKind::Starter | DomeKind::Storage => (prop::CHEST, 0.0),
            DomeKind::Barn => (prop::TROUGH, 0.0),
            _ => (prop::CRATE, 0.0),
        },
        Fixture::SuitRack => (prop::SUIT_RACK, 0.0),
        Fixture::Synthesizer => (prop::SYNTHESIZER, 5.0),
        Fixture::Terminal => (prop::TERMINAL, 4.0),
        Fixture::ModBay => (prop::MOD_BAY, 4.0),
        Fixture::Planter(_) => match kind {
            DomeKind::Dark => (prop::DARK_PLANTER, 0.0),
            DomeKind::Aqua => (prop::AQUA_PLANTER, 3.0),
            _ => (prop::PLANTER, 0.0),
        },
        Fixture::Cooker => (prop::COOKER, 5.0),
        Fixture::CompostVat => (prop::COMPOST_VAT, 3.0),
        Fixture::AssemblyRing => (prop::ASSEMBLY_RING, 6.0),
        Fixture::FishPool => (prop::FISH_POOL, 3.0),
        Fixture::Stall(_) => (prop::STALL, 0.0),
        Fixture::Condenser => (prop::CONDENSER, 5.0),
        Fixture::Berths => (prop::BERTH, 0.0),
    }
}

fn machine_row(kind: MachineKind) -> (usize, f32) {
    match kind {
        MachineKind::Chest => (prop::CHEST, 0.0),
        MachineKind::Lamp => (prop::LAMP, 3.0),
        MachineKind::Pylon => (prop::PYLON, 0.0),
        MachineKind::Pump => (prop::PUMP, 6.0),
        MachineKind::Tank => (prop::TANK, 0.0),
        MachineKind::OxygenGenerator => (prop::OXYGEN_GENERATOR, 6.0),
        MachineKind::Synthesizer => (prop::SYNTHESIZER, 5.0),
        MachineKind::Splicer => (prop::SPLICER, 4.0),
    }
}

/// (first atlas index, frames, fps) for a creature in a state.
fn creature_frames(kind: CreatureKind, state: State) -> (usize, usize, f32) {
    let at = |row: usize, col: usize| row * 8 + col;
    match (kind, state) {
        (_, State::Dying) if kind != CreatureKind::Skitter => (at(9, 0), 6, 12.0),
        (CreatureKind::Driftmoth, _) => (at(0, 0), 4, 10.0),
        (CreatureKind::Puffback, State::Chase) => (at(1, 0), 4, 6.0),
        (CreatureKind::Puffback, _) => (at(1, 4), 4, 3.0),
        (CreatureKind::Skitter, State::Dying) => (at(3, 0), 4, 8.0),
        (CreatureKind::Skitter, State::Attack) => (at(2, 4), 4, 12.0),
        (CreatureKind::Skitter, State::Chase) => (at(2, 0), 4, 14.0),
        (CreatureKind::Skitter, _) => (at(3, 4), 4, 4.0),
        (CreatureKind::Webspinner, State::Attack) => (at(4, 4), 4, 8.0),
        (CreatureKind::Webspinner, _) => (at(4, 0), 4, 6.0),
        (CreatureKind::GloomLeech, State::Latched) => (at(6, 4), 4, 8.0),
        (CreatureKind::GloomLeech, _) => (at(6, 0), 4, 8.0),
        (CreatureKind::BurrowMaw, State::Hidden) => (at(7, 0), 2, 2.0),
        (CreatureKind::BurrowMaw, State::Attack) => (at(7, 2), 6, 7.0),
        (CreatureKind::BurrowMaw, _) => (at(8, 0), 4, 6.0),
        (CreatureKind::BroodMother, State::Attack) => (4, 4, 5.0),
        (CreatureKind::BroodMother, _) => (0, 4, 4.0),
    }
}

fn spawn_ent(commands: &mut Commands, assets: &GameAssets, e: &Ent, now: f32) -> Entity {
    let at = Vec2::new(e.pos.x, e.pos.y);
    let tag = EntSprite {
        id: e.id,
        shown: at,
        born: now,
    };
    match &e.kind {
        EntKind::Dome(d) => {
            let (w, h) = d.kind.size();
            let (back, front) = assets.domes.get(&d.kind).cloned().unwrap_or_default();
            let size = Some(Vec2::new(w as f32, h as f32));
            let (cx, fy) = (e.pos.x.floor(), e.pos.y.floor());
            commands
                .spawn((tag, InGameEntity, Transform::default(), Visibility::Visible))
                .with_children(|parent| {
                    parent.spawn((
                        Sprite {
                            image: back,
                            custom_size: size,
                            ..default()
                        },
                        Transform::from_xyz(cx, -(fy - h as f32 / 2.0), z::DOME_BACK),
                    ));
                    parent.spawn((
                        Sprite {
                            image: front,
                            custom_size: size,
                            ..default()
                        },
                        Transform::from_xyz(cx, -(fy - h as f32 / 2.0), z::DOME_FRONT),
                    ));
                    for f in d.kind.fixtures() {
                        let p = fixture_pos(e.pos, f);
                        let (row, fps) = fixture_row(d.kind, f.fixture);
                        let mut fixture =
                            parent.spawn((prop_sprite(assets, row, 0), prop_at(p.x, p.y, z::PROPS)));
                        if fps > 0.0 {
                            fixture.insert(PropAnim { row, fps });
                        }
                        // The colonists' berths fill the back wall.
                        if f.fixture == Fixture::Berths {
                            for dx in [-44.0, -22.0, 22.0, 44.0] {
                                parent.spawn((prop_sprite(assets, row, 1), prop_at(p.x + dx, p.y, z::PROPS)));
                            }
                        }
                    }
                })
                .id()
        }
        EntKind::Machine(m) => {
            let (row, _) = machine_row(m.kind);
            commands
                .spawn((
                    tag,
                    InGameEntity,
                    prop_sprite(assets, row, 0),
                    prop_at(at.x, at.y, z::PROPS),
                ))
                .id()
        }
        EntKind::Crate(c) => {
            let row = match c.kind {
                CrateKind::Pack => prop::PACK,
                CrateKind::Cache => prop::CHEST,
                CrateKind::Pod => prop::POD,
            };
            commands
                .spawn((
                    tag,
                    InGameEntity,
                    prop_sprite(assets, row, 0),
                    prop_at(at.x, at.y, z::PROPS),
                ))
                .id()
        }
        EntKind::Creature(c) => {
            let sprite = if c.kind == CreatureKind::BroodMother {
                assets.brood_mother.sprite(0)
            } else {
                assets.creatures.sprite(0)
            };
            commands
                .spawn((
                    tag,
                    InGameEntity,
                    sprite,
                    Transform::from_xyz(at.x, -at.y, z::CREATURES),
                ))
                .id()
        }
        EntKind::Drop(d) => {
            let mut sprite = assets.icon_sheet.sprite(d.item.icon());
            sprite.custom_size = Some(Vec2::splat(9.0));
            commands
                .spawn((
                    tag,
                    InGameEntity,
                    sprite,
                    Transform::from_xyz(at.x, -at.y, z::DROPS),
                ))
                .id()
        }
        EntKind::Bolt(b) => {
            let sprite = if b.web {
                assets.creatures.sprite(5 * 8)
            } else {
                assets.fx.sprite(if b.size > 1.3 { 4 } else { 0 })
            };
            commands
                .spawn((
                    tag,
                    InGameEntity,
                    sprite,
                    Transform::from_xyz(at.x, -at.y, z::BOLTS),
                ))
                .id()
        }
        EntKind::Plant(_) => commands
            .spawn((
                tag,
                InGameEntity,
                Transform::from_xyz(at.x, -at.y, z::PLANTS),
                Visibility::Hidden,
            ))
            .id(),
    }
}

/// Spawns sprites for new entities and removes those of entities that are gone.
pub fn sync_entities(
    mut commands: Commands,
    session: Res<Session>,
    assets: Res<GameAssets>,
    time: Res<Time>,
    mut sprites: ResMut<EntSprites>,
) {
    let Some(colony) = &session.colony else { return };
    sprites.0.retain(|id, entity| {
        let keep = colony.ents.contains_key(id);
        if !keep {
            commands.entity(*entity).despawn();
        }
        keep
    });
    for e in colony.ents.values() {
        sprites
            .0
            .entry(e.id)
            .or_insert_with(|| spawn_ent(&mut commands, &assets, e, time.elapsed_secs()));
    }
}

pub fn reset_entities(mut sprites: ResMut<EntSprites>) {
    sprites.0.clear();
}

/// Clients move things along between the host's updates.
pub fn extrapolate(mut session: ResMut<Session>, time: Res<Time>) {
    if session.is_authority() {
        return;
    }
    let dt = time.delta_secs();
    if let Some(colony) = &mut session.colony {
        for e in colony.ents.values_mut() {
            if e.moves() {
                e.pos += e.velocity() * dt;
            }
        }
    }
}

pub fn animate_entities(
    session: Res<Session>,
    time: Res<Time>,
    mut q: Query<(&mut EntSprite, &mut Transform, Option<&mut Sprite>)>,
    mut props: Query<(&PropAnim, &mut Sprite), Without<EntSprite>>,
) {
    let Some(colony) = &session.colony else { return };
    let t = time.elapsed_secs();
    let dt = time.delta_secs();
    for (anim, mut sprite) in &mut props {
        if let Some(atlas) = &mut sprite.texture_atlas {
            atlas.index = anim.row * 4 + (t * anim.fps) as usize % 4;
        }
    }
    for (mut tag, mut tf, sprite) in &mut q {
        let Some(e) = colony.ents.get(&tag.id) else {
            continue;
        };
        let target = Vec2::new(e.pos.x, e.pos.y);
        tag.shown = if tag.shown.distance(target) > 60.0 {
            target
        } else {
            tag.shown.lerp(target, 1.0 - (-22.0 * dt).exp())
        };
        let age = t - tag.born;
        let Some(mut sprite) = sprite else { continue };
        match &e.kind {
            EntKind::Creature(c) => {
                let (first, frames, fps) = creature_frames(c.kind, c.state);
                let (_, h) = c.kind.size();
                if let Some(atlas) = &mut sprite.texture_atlas {
                    let f = (age * fps) as usize;
                    // One-shot animations hold their last frame.
                    let once = matches!(c.state, State::Dying)
                        || (c.kind == CreatureKind::BurrowMaw && c.state == State::Attack);
                    atlas.index = first + if once { f.min(frames - 1) } else { f % frames };
                }
                let flying = matches!(c.kind, CreatureKind::Driftmoth | CreatureKind::GloomLeech);
                let lift = if c.kind == CreatureKind::BroodMother {
                    24.0
                } else if flying {
                    h / 2.0
                } else {
                    12.0
                };
                tf.translation = Vec3::new(tag.shown.x, -(tag.shown.y - lift), z::CREATURES);
                sprite.flip_x = c.facing < 0;
                sprite.color = if c.hit_flash > 0.0 {
                    Color::srgb(3.0, 3.0, 3.0)
                } else {
                    Color::WHITE
                };
            }
            EntKind::Drop(d) => {
                let bob = (t * 3.0 + tag.id as f32).sin() * 1.2;
                tf.translation = Vec3::new(tag.shown.x, -(tag.shown.y - 5.0) + bob, z::DROPS);
                if let Some(atlas) = &mut sprite.texture_atlas {
                    atlas.index = d.item.icon();
                }
                if let Item::Seed(s) | Item::Crop(s) = d.item
                    && let Some(sp) = colony.species.get(s as usize)
                    && d.item.icon() != 92
                {
                    let c = crate::ui::hue_color(sp.looks.hue_bloom, 0.6, 0.95);
                    sprite.color = Color::srgb_u8(c.r(), c.g(), c.b());
                }
            }
            EntKind::Bolt(b) => {
                tf.translation = Vec3::new(tag.shown.x, -tag.shown.y, z::BOLTS);
                tf.rotation = Quat::from_rotation_z(-b.vel.y.atan2(b.vel.x));
                tf.scale = Vec3::splat(b.size.clamp(0.8, 2.0));
                if let Some(atlas) = &mut sprite.texture_atlas {
                    let base = if b.web {
                        5 * 8
                    } else if b.size > 1.3 {
                        4
                    } else {
                        0
                    };
                    atlas.index = base + (age * 16.0) as usize % 4;
                }
            }
            EntKind::Machine(m) => {
                // Pylons and tanks show how full they are; others animate
                // only while running.
                let (row, fps) = machine_row(m.kind);
                let frame = match m.kind {
                    MachineKind::Pylon => {
                        let n = m.inv.stacks().map(|s| s.count).sum::<u32>();
                        (n.min(20) as usize * 3).div_ceil(20)
                    }
                    MachineKind::Tank => ((m.store / 500.0) * 3.0).round().clamp(0.0, 3.0) as usize,
                    MachineKind::Chest => 0,
                    _ if m.on => (t * fps) as usize % 4,
                    _ => 0,
                };
                if let Some(atlas) = &mut sprite.texture_atlas {
                    atlas.index = row * 4 + frame;
                }
            }
            _ => {}
        }
    }
}
