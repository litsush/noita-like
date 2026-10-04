//! Ambient life and feedback: lava embers, smoke over fires, acid fizz,
//! splashes, fire crackle and the core's hum.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use sbct_sim::Material;

use super::Run;
use super::entities::{Prop, PropKind};
use super::player::RunPlayer;
use crate::audio::Sfx;
use crate::fx::Burst;
use crate::render::{PIXEL_SCALE, WorldCamera};
use crate::session::Session;

/// Cells sampled per frame for ambient effects.
const SAMPLES: usize = 60;

pub fn ambient_effects(
    session: Res<Session>,
    mut run: ResMut<Run>,
    player: Res<RunPlayer>,
    camera: Single<&Transform, With<WorldCamera>>,
    window: Single<&Window, With<PrimaryWindow>>,
    props: Query<&Prop>,
    mut bursts: MessageWriter<Burst>,
    mut sfx: MessageWriter<Sfx>,
    mut was_submerged: Local<f32>,
    mut crackle: Local<f32>,
    mut hum: Local<f32>,
    time: Res<Time>,
    mut dread: Local<Dread>,
    mut music: ResMut<crate::audio::MusicTrack>,
    stalker: Option<Res<super::creatures::StalkerState>>,
) {
    let Some(world) = &session.world else { return };
    let dt = time.delta_secs();
    let cam = Vec2::new(camera.translation.x, -camera.translation.y);
    let half = Vec2::new(window.width(), window.height()) / PIXEL_SCALE / 2.0;

    let mut fires = 0;
    let mut nearest_fire = None::<Vec2>;
    for _ in 0..SAMPLES {
        let x = (cam.x - half.x + run.rng.next_f32() * half.x * 2.0) as i32;
        let y = (cam.y - half.y + run.rng.next_f32() * half.y * 2.0) as i32;
        let at = Vec2::new(x as f32 + 0.5, y as f32);
        let above = world.material(x, y - 1);
        match world.material(x, y) {
            Material::Lava if above.is_open() && run.rng.chance(60) => {
                bursts.write(
                    Burst::new(at, Color::srgb(1.0, 0.55, 0.15))
                        .count(1)
                        .speed(15.0)
                        .gravity(-45.0)
                        .life(0.9),
                );
            }
            Material::Fire => {
                fires += 1;
                if nearest_fire.is_none_or(|n| n.distance(player.body.pos) > at.distance(player.body.pos)) {
                    nearest_fire = Some(at);
                }
                if above.is_open() && run.rng.chance(50) {
                    bursts.write(
                        Burst::new(at, Color::srgba(0.25, 0.23, 0.23, 0.6))
                            .count(1)
                            .speed(8.0)
                            .gravity(-30.0)
                            .life(1.4)
                            .size(2.0),
                    );
                }
            }
            Material::Acid if above.is_open() && run.rng.chance(30) => {
                bursts.write(
                    Burst::new(at, Color::srgba(0.6, 1.0, 0.3, 0.8))
                        .count(1)
                        .speed(6.0)
                        .gravity(-25.0)
                        .life(0.6),
                );
            }
            Material::Metal if above.is_open() && run.rng.chance(20) => {
                bursts.write(
                    Burst::new(at, Color::srgb(1.0, 0.85, 0.5))
                        .count(1)
                        .speed(25.0)
                        .gravity(60.0)
                        .life(0.4),
                );
            }
            _ => {}
        }
    }

    // Fire crackle loop, retriggered while flames are on screen.
    *crackle -= dt;
    if fires > 0 && *crackle <= 0.0 {
        *crackle = 2.9;
        if let Some(at) = nearest_fire {
            sfx.write(
                Sfx::at("fire_loop", at)
                    .volume((fires as f32 / 6.0).min(1.0))
                    .pitch(0.0),
            );
        }
    }

    // The core hums as you get close.
    *hum -= dt;
    if *hum <= 0.0 {
        *hum = 3.9;
        if let Some(core) = props.iter().find(|p| p.kind == PropKind::Core) {
            let d = core.center().distance(player.body.center());
            if d < 260.0 {
                sfx.write(Sfx::at("core_hum", core.center()).volume(1.4).pitch(0.0));
            }
        }
    }

    // Unexplained sounds, now and then, somewhere out in the dark.
    dread.whisper -= dt;
    if dread.whisper <= 0.0 {
        dread.whisper = 35.0 + run.rng.next_f32() * 50.0;
        if player.body.pos.y > 200.0 && run.is_playing() {
            let names = [
                "whisper_1",
                "whisper_2",
                "whisper_3",
                "whisper_4",
                "distant_steps",
            ];
            let name = names[(run.rng.next_u8() % names.len() as u8) as usize];
            let a = run.rng.next_f32() * std::f32::consts::TAU;
            let at = player.body.center() + Vec2::from_angle(a) * (50.0 + run.rng.next_f32() * 60.0);
            sfx.write(Sfx::at(name, at).volume(0.6).pitch(0.0));
        }
    }
    // Moments of silence: the ambience drops out, then creeps back.
    dread.silence -= dt;
    if dread.silence <= 0.0 && run.is_playing() {
        if dread.silent {
            dread.silent = false;
            dread.silence = 70.0 + run.rng.next_f32() * 60.0;
            *music = crate::audio::MusicTrack::Ambient(run.layer.index());
            sfx.write(Sfx::ui("sting_3").volume(0.5));
        } else if run.layer.index() > 0 {
            dread.silent = true;
            dread.silence = 10.0 + run.rng.next_f32() * 8.0;
            *music = crate::audio::MusicTrack::None;
        } else {
            dread.silence = 30.0;
        }
    }
    // The Stalker keeps its own company.
    if stalker.is_some_and(|s| s.near > 0.3) && dread.silent {
        dread.silent = false;
        *music = crate::audio::MusicTrack::Ambient(run.layer.index());
    }

    // Splash on entering liquid at speed.
    let sub = player.body.submersion(world);
    if *was_submerged < 0.1 && sub > 0.25 && player.body.vel.y > 40.0 {
        let liquid = world.material(player.body.pos.x as i32, player.body.pos.y as i32);
        let [r, g, b] = liquid.props().color;
        bursts.write(
            Burst::new(player.body.pos - Vec2::Y * 6.0, Color::srgb_u8(r, g, b))
                .count(16)
                .speed(60.0)
                .dir(Vec2::NEG_Y)
                .life(0.6),
        );
        sfx.write(Sfx::at("splash", player.body.pos));
    }
    *was_submerged = sub;
}

/// Timers for the occasional unexplained sound and the stretches of silence.
pub struct Dread {
    whisper: f32,
    silence: f32,
    silent: bool,
}

impl Default for Dread {
    fn default() -> Self {
        Dread {
            whisper: 25.0,
            silence: 90.0,
            silent: false,
        }
    }
}
