//! Draws plants: the ones in planters and the ones growing outside. Sprites
//! come from [`PlantArt`]; they sway in the wind and bend away from players
//! walking through them.

use std::collections::HashMap;

use bevy::prelude::*;
use sbct_sim::colony::domes::{Fixture, fixture_pos};
use sbct_sim::colony::items::DomeKind;
use sbct_sim::colony::plants::SpeciesId;
use sbct_sim::colony::{EntKind, Id, WeatherKind};

use crate::lighting::PointLights;
use crate::plantart::{FRAME_H, PlantArt, hsv, stage};
use crate::render::{InGameEntity, z};
use crate::session::Session;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum PlantKey {
    /// A stalk in a dome's planter: (dome, planter, stalk).
    Planter(Id, u8, u8),
    /// A plant growing outside a planter.
    Ent(Id),
}

#[derive(Component)]
pub struct PlantSprite {
    key: PlantKey,
    species: SpeciesId,
    /// Where it is rooted, in cells.
    base: Vec2,
    phase: f32,
    bend: f32,
    growth: f32,
}

#[derive(Resource, Default)]
pub struct PlantSprites(HashMap<PlantKey, Entity>);

/// Everything that should be drawn: key, species, growth, root position.
fn wanted(session: &Session) -> Vec<(PlantKey, SpeciesId, f32, Vec2)> {
    let mut out = Vec::new();
    let Some(colony) = &session.colony else { return out };
    for e in colony.ents.values() {
        match &e.kind {
            EntKind::Dome(d) => {
                for f in d.kind.fixtures() {
                    let Fixture::Planter(i) = f.fixture else { continue };
                    let Some(p) = d.planters.get(i as usize) else {
                        continue;
                    };
                    let Some(species) = p.species else { continue };
                    let at = fixture_pos(e.pos, f);
                    // Soil sits a little above the planter's base; tanks are deeper.
                    let (lift, stalks): (f32, &[f32]) = match d.kind {
                        DomeKind::Aqua => (4.0, &[-5.0, 5.0]),
                        DomeKind::Dark => (8.0, &[-6.0, 0.0, 6.0]),
                        _ => (8.0, &[-7.0, 0.0, 7.0]),
                    };
                    for (s, dx) in stalks.iter().enumerate() {
                        // Stalks in one planter are a little out of step.
                        let growth = (p.growth - s as f32 * 0.04).max(0.0);
                        let growth = if p.growth >= 1.0 { 1.0 } else { growth };
                        out.push((
                            PlantKey::Planter(e.id, i, s as u8),
                            species,
                            growth,
                            Vec2::new(at.x + dx, at.y - lift),
                        ));
                    }
                }
            }
            EntKind::Plant(p) => out.push((
                PlantKey::Ent(e.id),
                p.species,
                p.growth,
                Vec2::new(e.pos.x, e.pos.y),
            )),
            _ => {}
        }
    }
    out
}

pub fn sync_plants(
    mut commands: Commands,
    session: Res<Session>,
    mut sprites: ResMut<PlantSprites>,
    mut art: ResMut<PlantArt>,
    mut images: ResMut<Assets<Image>>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
    mut existing: Query<&mut PlantSprite>,
) {
    let Some(colony) = &session.colony else { return };
    let wanted = wanted(&session);
    let keys: std::collections::HashSet<PlantKey> = wanted.iter().map(|w| w.0).collect();
    sprites.0.retain(|key, entity| {
        let keep = keys.contains(key);
        if !keep {
            commands.entity(*entity).despawn();
        }
        keep
    });
    for (key, species, growth, base) in wanted {
        if let Some(&entity) = sprites.0.get(&key)
            && let Ok(mut sprite) = existing.get_mut(entity)
        {
            if sprite.species == species {
                sprite.growth = growth;
                sprite.base = base;
                continue;
            }
            // Replanted with something else.
            commands.entity(entity).despawn();
        }
        let Some(sp) = colony.species.get(species as usize) else {
            continue;
        };
        let phase = (sbct_sim::rng::hash2(species as u64, base.x as i32, base.y as i32) % 628) as f32 / 100.0;
        let entity = commands
            .spawn((
                PlantSprite {
                    key,
                    species,
                    base,
                    phase,
                    bend: 0.0,
                    growth,
                },
                InGameEntity,
                art.sprite(sp, growth, &mut images, &mut layouts),
                Transform::from_xyz(base.x, -base.y, z::PLANTS),
            ))
            .id();
        sprites.0.insert(key, entity);
    }
}

pub fn animate_plants(
    session: Res<Session>,
    time: Res<Time>,
    mut lights: ResMut<PointLights>,
    mut q: Query<(&mut PlantSprite, &mut Sprite, &mut Transform)>,
) {
    let Some(colony) = &session.colony else { return };
    let t = time.elapsed_secs();
    let dt = time.delta_secs();
    let players: Vec<Vec2> = colony
        .online()
        .filter(|p| p.alive())
        .map(|p| Vec2::new(p.pose.pos.x, p.pose.pos.y))
        .collect();
    let wind = match colony.weather.kind {
        WeatherKind::Rain => 2.2,
        WeatherKind::Mist => 0.6,
        WeatherKind::Clear => 1.0,
    };
    for (mut plant, mut sprite, mut tf) in &mut q {
        if let Some(atlas) = &mut sprite.texture_atlas {
            atlas.index = stage(plant.growth);
        }
        // Indoors there is only a gentle draught.
        let breeze = if matches!(plant.key, PlantKey::Planter(..)) {
            0.35
        } else {
            wind
        };
        let sway = (t * 1.4 + plant.phase).sin() * 0.035 * breeze
            + (t * 3.1 + plant.phase * 2.0).sin() * 0.012 * breeze;
        // Bend away from anyone brushing past.
        let mut push = 0.0;
        for p in &players {
            let d = plant.base - *p;
            if d.x.abs() < 9.0 && d.y > -4.0 && d.y < 26.0 {
                push = d.x.signum() * 0.4 * (1.0 - d.x.abs() / 9.0);
            }
        }
        plant.bend += (push - plant.bend) * (dt * 9.0).min(1.0);
        let angle = -(sway + plant.bend) * (0.4 + plant.growth * 0.6);
        // Rotate about the root, which is near the bottom of the frame.
        let pivot = FRAME_H as f32 / 2.0 - 2.0;
        let offset = Vec2::new(-angle.sin() * pivot, angle.cos() * pivot);
        tf.translation = Vec3::new(plant.base.x + offset.x, -plant.base.y + offset.y, z::PLANTS);
        tf.rotation = Quat::from_rotation_z(angle);

        // Mature glowing species light their surroundings.
        if plant.growth > 0.8
            && let Some(sp) = colony.species.get(plant.species as usize)
            && sp.looks.glow >= 5
        {
            let c = hsv(sp.looks.hue_bloom, 0.6, 1.0);
            let k = sp.looks.glow as f32 / 10.0 * 1.3;
            let height = 8.0 + sp.looks.height as f32 * 2.5;
            lights.0.push((
                Vec2::new(plant.base.x, plant.base.y - height),
                [
                    c[0] as f32 / 255.0 * k,
                    c[1] as f32 / 255.0 * k,
                    c[2] as f32 / 255.0 * k,
                ],
            ));
        }
    }
}

pub fn reset_plants(mut sprites: ResMut<PlantSprites>, mut art: ResMut<PlantArt>) {
    sprites.0.clear();
    art.clear();
}
