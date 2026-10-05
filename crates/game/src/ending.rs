//! The colony ship: its descent when called, the touchdown, the colonists
//! stepping out, and the banner. Afterwards the ship stays where it landed.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use sbct_sim::colony::readiness::{COLONISTS_PER_APARTMENT, LANDING_SECS, Ship};
use sbct_sim::rng::Rng;

use crate::assets::GameAssets;
use crate::audio::{MusicTrack, Sfx};
use crate::fx::{Burst, Shake};
use crate::lighting::PointLights;
use crate::render::{InGameEntity, z};
use crate::session::Session;
use crate::ui::{self, ACCENT, TEXT};

#[derive(Component)]
pub struct ShipSprite;

#[derive(Component)]
pub struct ShipFlame(f32);

/// A colonist walking from the ramp toward the base.
#[derive(Component)]
pub struct Colonist {
    variant: usize,
    speed: f32,
    stop_x: f32,
    born: f32,
}

#[derive(Resource)]
pub struct Ending {
    rng: Rng,
    /// Seconds since touchdown was first seen by this client.
    since_landing: Option<f32>,
    colonists_out: u32,
    spawn_wait: f32,
    engine_wait: f32,
    was: Ship,
}

impl Default for Ending {
    fn default() -> Self {
        Ending {
            rng: Rng::new(4242),
            since_landing: None,
            colonists_out: 0,
            spawn_wait: 0.0,
            engine_wait: 0.0,
            was: Ship::Away,
        }
    }
}

/// How high above the ground the ship is, `t` seconds after the call.
fn altitude(t: f32) -> f32 {
    let left = (1.0 - t / LANDING_SECS).clamp(0.0, 1.0);
    // Fast at first, feathering down at the end.
    900.0 * left * left
}

pub fn update_ship(
    mut commands: Commands,
    session: Res<Session>,
    assets: Res<GameAssets>,
    time: Res<Time>,
    mut ending: ResMut<Ending>,
    mut music: ResMut<MusicTrack>,
    mut sfx: MessageWriter<Sfx>,
    mut bursts: MessageWriter<Burst>,
    mut shake: ResMut<Shake>,
    mut lights: ResMut<PointLights>,
    mut ship: Query<&mut Transform, (With<ShipSprite>, Without<ShipFlame>, Without<Colonist>)>,
    mut flames: Query<
        (&ShipFlame, &mut Transform, &mut Sprite, &mut Visibility),
        (Without<ShipSprite>, Without<Colonist>),
    >,
    mut colonists: Query<(&Colonist, &mut Transform, &mut Sprite), (Without<ShipSprite>, Without<ShipFlame>)>,
) {
    let Some(colony) = &session.colony else { return };
    let dt = time.delta_secs();
    let t = time.elapsed_secs();
    let site = colony.landing_site();
    let ground = Vec2::new(site.x, -site.y);
    let state = colony.ship;
    if state == Ship::Away {
        return;
    }

    // The ship sprite appears as soon as it is called (or was already down).
    if ship.is_empty() {
        commands.spawn((
            ShipSprite,
            InGameEntity,
            Sprite::from_image(assets.ship.clone()),
            Transform::from_xyz(ground.x, ground.y + 2000.0, z::PLAYERS - 0.5),
        ));
        // One flame under each of the three engine bells.
        for dx in [-34.0, 0.0, 32.0] {
            commands.spawn((
                ShipFlame(dx),
                InGameEntity,
                assets.ship_flame.sprite(0),
                Transform::from_xyz(0.0, 0.0, z::PLAYERS - 0.6),
                Visibility::Hidden,
            ));
        }
        if matches!(state, Ship::Landing(_)) {
            *music = MusicTrack::Ending;
        }
        return;
    }

    let height = match state {
        Ship::Landing(elapsed) => altitude(elapsed),
        _ => 0.0,
    };
    // The art is 96 tall with its feet on the bottom row.
    let center = Vec2::new(ground.x, ground.y + 48.0 + height);
    for mut tf in &mut ship {
        tf.translation = center.extend(z::PLAYERS - 0.5);
    }
    let burning = matches!(state, Ship::Landing(_));
    for (flame, mut tf, mut sprite, mut vis) in &mut flames {
        *vis = if burning {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
        // Engine mouths end 16 px above the feet; the flame hangs below.
        tf.translation = Vec3::new(
            center.x + flame.0,
            center.y - 48.0 + 16.0 - 24.0,
            z::PLAYERS - 0.6,
        );
        if let Some(atlas) = &mut sprite.texture_atlas {
            atlas.index = (t * 14.0) as usize % 4;
        }
    }
    if burning {
        lights
            .0
            .push((Vec2::new(center.x, -(center.y - 60.0)), [3.5, 2.6, 1.6]));
        ending.engine_wait -= dt;
        if ending.engine_wait <= 0.0 {
            ending.engine_wait = 2.9;
            sfx.write(Sfx::at("ship_engine_loop", Vec2::new(site.x, site.y - height)).volume(1.0));
        }
        // The exhaust kicks up dust once it is close.
        if height < 160.0 {
            shake.add(0.03);
            bursts.write(
                Burst::new(
                    Vec2::new(site.x + (ending.rng.next_f32() - 0.5) * 120.0, site.y),
                    Color::srgb(0.75, 0.7, 0.62),
                )
                .count(3)
                .speed(90.0)
                .life(0.8),
            );
        }
    }

    // Touchdown.
    let landed = matches!(state, Ship::Landed(_));
    if landed && matches!(ending.was, Ship::Landing(_)) {
        ending.since_landing = Some(0.0);
        shake.add(0.6);
        sfx.write(Sfx::at("ship_land", Vec2::new(site.x, site.y)));
        bursts.write(
            Burst::new(Vec2::new(site.x, site.y), Color::srgb(0.8, 0.76, 0.68))
                .count(60)
                .speed(140.0)
                .life(1.2),
        );
    }
    ending.was = state;

    // Colonists file out of the ramp on the right and walk toward the base.
    if let Some(since) = &mut ending.since_landing {
        *since += dt;
        let since = *since;
        // Hand the music back once the ending theme has played out.
        if since > 40.0 && *music == MusicTrack::Ending {
            *music = MusicTrack::Day;
        }
        let total = (colony.readiness.apartments as u32 * COLONISTS_PER_APARTMENT).clamp(12, 40);
        ending.spawn_wait -= dt;
        if since > 2.5 && ending.colonists_out < total && ending.spawn_wait <= 0.0 {
            ending.spawn_wait = 0.45;
            ending.colonists_out += 1;
            let variant = (ending.rng.next_u8() % 2) as usize;
            let stop_x = site.x + 110.0 + ending.rng.next_f32() * 150.0;
            let speed = 20.0 + ending.rng.next_f32() * 10.0;
            commands.spawn((
                Colonist {
                    variant,
                    speed,
                    stop_x,
                    born: t,
                },
                InGameEntity,
                assets.workers.sprite((4 + variant) * 8),
                Transform::from_xyz(site.x + 86.0, ground.y + 12.0, z::PLAYERS - 0.2),
            ));
        }
        // Fireworks for the first while.
        if since < 20.0 && ending.rng.chance(6) {
            let at = Vec2::new(
                site.x + (ending.rng.next_f32() - 0.3) * 400.0,
                site.y - 120.0 - ending.rng.next_f32() * 120.0,
            );
            let hue = ending.rng.next_f32() * 360.0;
            bursts.write(
                Burst::new(at, Color::hsl(hue, 0.9, 0.7))
                    .count(26)
                    .speed(80.0)
                    .gravity(40.0)
                    .life(1.3),
            );
            lights.0.push((at, [1.5, 1.5, 1.5]));
        }
    }
    let Some(world) = &session.world else { return };
    for (c, mut tf, mut sprite) in &mut colonists {
        let age = t - c.born;
        let walking = tf.translation.x < c.stop_x;
        if walking {
            tf.translation.x += c.speed * dt;
        }
        // Follow the ground.
        let x = tf.translation.x as i32;
        let top = colony.profile.surface_at(x);
        let floor = sbct_sim::colony::build::ground_below(world, x, top - 30, 80).unwrap_or(top);
        tf.translation.y = -(floor as f32) + 12.0;
        if let Some(atlas) = &mut sprite.texture_atlas {
            let row = (4 + c.variant) * 8;
            atlas.index = if walking {
                row + (age * 7.0) as usize % 4
            } else {
                // Arrived: wave and cheer.
                row + 4 + (age * 5.0) as usize % 4
            };
        }
    }
}

/// The banner shown for a while after touchdown.
pub fn ending_banner(mut contexts: EguiContexts, session: Res<Session>, ending: Res<Ending>) -> Result {
    let Some(since) = ending.since_landing else {
        return Ok(());
    };
    if !(1.0..16.0).contains(&since) {
        return Ok(());
    }
    let ctx = contexts.ctx_mut()?;
    let Some(colony) = &session.colony else {
        return Ok(());
    };
    let Ship::Landed(day) = colony.ship else {
        return Ok(());
    };
    let fade = ((since - 1.0) / 1.5).clamp(0.0, 1.0) * ((16.0 - since) / 2.0).clamp(0.0, 1.0);
    egui::Area::new("ending".into())
        .anchor(egui::Align2::CENTER_TOP, [0.0, 90.0])
        .interactable(false)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            ui.set_opacity(fade);
            ui::ornate(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(ui::title("THE COLONY IS ESTABLISHED", 40.0).color(ACCENT));
                    ui::flourish(ui);
                    ui.label(
                        egui::RichText::new(format!(
                            "Day {day}. {:.1}% oxygen, {} species in the codex, {} credits earned.",
                            colony.atmosphere.o2,
                            colony.codex.plants.len(),
                            colony.stats.earned
                        ))
                        .color(TEXT),
                    );
                    ui.label(egui::RichText::new("The planet is yours to keep growing.").color(ui::TEXT_DIM));
                });
            });
        });
    Ok(())
}

pub fn reset_ending(mut ending: ResMut<Ending>) {
    *ending = Ending::default();
}
