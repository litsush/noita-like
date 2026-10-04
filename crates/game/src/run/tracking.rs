//! Turns simulation events into consequences: explosion damage, sounds,
//! particles and achievement progress.

use bevy::prelude::*;
use sbct_sim::world::disc;
use sbct_sim::{Material, SimEvent};

use super::achievements::AchievementId;
use super::creatures::{Creature, blast_creatures};
use super::hazards::hurt;
use super::player::RunPlayer;
use super::save::SaveData;
use super::{DamageKind, Run, grant};
use crate::audio::Sfx;
use crate::fx::{Burst, HitStop, Shake};
use crate::session::Session;

/// Events this close to the player count towards achievements.
const NEAR: f32 = 200.0;

pub fn process_sim_events(
    mut session: ResMut<Session>,
    mut player: ResMut<RunPlayer>,
    mut run: ResMut<Run>,
    mut save: ResMut<SaveData>,
    mut creatures: Query<(Entity, &mut Creature)>,
    mut sfx: MessageWriter<Sfx>,
    mut bursts: MessageWriter<Burst>,
    mut shake: ResMut<Shake>,
    mut stop: ResMut<HitStop>,
) {
    let Some(world) = session.world.as_mut() else {
        return;
    };
    let events = world.take_events();
    let me = player.body.center();

    for event in events {
        match event {
            SimEvent::Explosion {
                x,
                y,
                radius,
                destroyed,
            } => {
                let at = Vec2::new(x as f32, y as f32);
                let d = at.distance(me);
                let reach = radius as f32 * 2.2;
                if d < reach {
                    let k = 1.0 - d / reach;
                    let knock = (me - at).normalize_or(Vec2::NEG_Y) * 260.0 * k + Vec2::Y * -80.0 * k;
                    hurt(
                        &mut player,
                        &mut run,
                        75.0 * k,
                        DamageKind::Explosion,
                        Some(knock),
                        &mut sfx,
                        &mut shake,
                    );
                }
                if run.influenced(at) {
                    run.stats.destroyed += destroyed;
                }
                if d < NEAR {
                    shake.add((1.0 - d / NEAR) * (radius as f32 / 12.0).min(1.0));
                    if radius >= 8 {
                        stop.0 = stop.0.max(0.05);
                    }
                }
                blast_creatures(&mut creatures, at, radius as f32);
                let name = if radius >= 8 {
                    "explosion"
                } else {
                    "explosion_small"
                };
                sfx.write(Sfx::at(name, at));
                bursts.write(
                    Burst::new(at, Color::srgb(1.0, 0.95, 0.7))
                        .count(30)
                        .speed(radius as f32 * 12.0)
                        .gravity(0.0)
                        .life(0.25)
                        .size(2.0),
                );
                bursts.write(
                    Burst::new(at, Color::srgba(0.3, 0.28, 0.28, 0.8))
                        .count(20)
                        .speed(40.0)
                        .gravity(-40.0)
                        .life(1.2)
                        .size(2.0),
                );

                // Volatile Core: blasts the player caused leave ore in the crater walls.
                if let Some(i) = run
                    .volatile_spots
                    .iter()
                    .position(|&(vx, vy)| (vx - x).abs() <= 2 && (vy - y).abs() <= 4)
                {
                    run.volatile_spots.swap_remove(i);
                    for (cx, cy) in disc(x, y, radius + 3) {
                        let m = world.material(cx, cy);
                        if m.is_solid_for_player() && m != Material::CoreShell && run.rng.chance(110) {
                            world.set(cx, cy, Material::ShardVein);
                        }
                    }
                }
            }
            SimEvent::Sizzle { x, y } => {
                let at = Vec2::new(x as f32, y as f32);
                sfx.write(Sfx::at("sizzle", at).volume(0.7));
                if run.rng.chance(60) {
                    bursts.write(
                        Burst::new(at, Color::srgba(0.85, 0.85, 0.9, 0.6))
                            .count(3)
                            .gravity(-60.0)
                            .speed(20.0),
                    );
                }
            }
            SimEvent::Dissolve { x, y } => {
                let at = Vec2::new(x as f32, y as f32);
                if run.influenced(at) {
                    run.stats.dissolved += 1;
                }
                sfx.write(Sfx::at("acid_hiss", at).volume(0.5));
            }
            SimEvent::ObsidianFormed { x, y } => {
                if run.influenced(Vec2::new(x as f32, y as f32)) {
                    run.stats.obsidian += 1;
                }
            }
            SimEvent::GasIgnited { x, y } => {
                let at = Vec2::new(x as f32, y as f32);
                if run.influenced(at) {
                    run.stats.gas_ignited += 1;
                }
                sfx.write(Sfx::at("torch_ignite", at).volume(0.6));
            }
            SimEvent::CollapseStarted { x, y } => {
                let at = Vec2::new(x as f32, y as f32);
                if run.influenced(at) {
                    run.stats.collapsed += 1;
                }
                if at.distance(me) < 60.0 {
                    shake.add(0.01);
                }
                sfx.write(Sfx::at("rumble", at));
            }
            SimEvent::Electrified { x, y } => {
                let at = Vec2::new(x as f32, y as f32);
                // Only water counts: molten metal sparks all by itself.
                if run.influenced(at) && world.material(x, y) == Material::Water {
                    run.stats.electrified += 1;
                }
                sfx.write(Sfx::at("spark", at).volume(0.5));
            }
            SimEvent::Ignited { x, y } => {
                sfx.write(Sfx::at("torch_ignite", Vec2::new(x as f32, y as f32)).volume(0.3));
            }
        }
    }

    let s = run.stats.clone();
    let checks = [
        (s.dissolved >= 500, AchievementId::Alchemist),
        (s.obsidian >= 60, AchievementId::ObsidianBridge),
        (s.gas_ignited >= 20, AchievementId::Pyromaniac),
        (s.electrified >= 200, AchievementId::Conductor),
        (s.collapsed >= 1000, AchievementId::CaveIn),
        (s.destroyed >= 1500, AchievementId::Demolitionist),
        (s.fungus_grown >= 300, AchievementId::Gardener),
        (s.remains_searched >= 5, AchievementId::GraveRobber),
    ];
    for (done, id) in checks {
        if done && run.is_playing() {
            grant(&mut save, &mut run, id, &mut sfx);
        }
    }
}
