//! Turns what happened in the colony into what the player hears and sees:
//! sound effects, particle bursts, camera shake and toasts. Also picks the
//! ambience and music for where the player is.

use bevy::prelude::*;
use sbct_sim::Material;
use sbct_sim::colony::creatures::CreatureKind;
use sbct_sim::colony::geom::V2;
use sbct_sim::colony::{Event, Fx, WeatherKind};
use sbct_sim::planetgen::Band;

use crate::audio::{Ambience, MusicTrack, Sfx};
use crate::fx::{Burst, Shake};
use crate::hud::Toasts;
use crate::session::Session;

fn dig_sound(m: Material) -> &'static str {
    match m {
        m if m.is_ore() => "dig_ore",
        Material::Ruin | Material::Plating => "dig_metal",
        Material::Stone | Material::Basalt | Material::Obsidian | Material::Crystal | Material::Ice => {
            "dig_hard"
        }
        _ => "dig_soft",
    }
}

fn die_sound(kind: CreatureKind) -> &'static str {
    match kind {
        CreatureKind::Driftmoth => "moth_flutter",
        CreatureKind::Puffback => "puff_grunt",
        CreatureKind::BroodMother => "brood_roar",
        _ => "skitter_die",
    }
}

fn call_sound(kind: CreatureKind) -> &'static str {
    match kind {
        CreatureKind::Skitter => "skitter_lunge",
        CreatureKind::Webspinner => "web_spit",
        CreatureKind::GloomLeech => "leech_latch",
        CreatureKind::BurrowMaw => "maw_rumble",
        CreatureKind::BroodMother => "brood_spawn",
        CreatureKind::Driftmoth => "moth_flutter",
        CreatureKind::Puffback => "puff_grunt",
    }
}

pub fn present_events(
    mut session: ResMut<Session>,
    mut sfx: MessageWriter<Sfx>,
    mut bursts: MessageWriter<Burst>,
    mut shake: ResMut<Shake>,
    mut toasts: ResMut<Toasts>,
    mut thunder: Local<u32>,
) {
    let me = session.player().map(|p| p.center());
    for event in std::mem::take(&mut session.events) {
        match event {
            Event::Toast { text, good, .. } => {
                sfx.write(Sfx::ui(if good { "toast" } else { "error" }));
                toasts.push(text, good);
            }
            Event::Fx { fx, pos } => {
                let at = Vec2::new(pos.x, pos.y);
                let near_me = me.is_some_and(|m: V2| m.distance(pos) < 14.0);
                match fx {
                    Fx::Dig(m) => {
                        sfx.write(Sfx::at(dig_sound(m), at));
                        let [r, g, b] = m.props().color;
                        bursts.write(
                            Burst::new(at, Color::srgb_u8(r, g, b))
                                .count(5)
                                .speed(36.0)
                                .life(0.35),
                        );
                    }
                    Fx::BoltFire => {
                        sfx.write(Sfx::at("laser_fire", at));
                    }
                    Fx::BoltHit => {
                        sfx.write(Sfx::at("laser_hit", at));
                        bursts.write(
                            Burst::new(at, Color::srgb(0.6, 1.0, 0.95))
                                .count(8)
                                .speed(60.0)
                                .gravity(40.0)
                                .life(0.25),
                        );
                    }
                    Fx::Pickup => {
                        sfx.write(Sfx::at("pickup", at).pitch(0.25));
                    }
                    Fx::Hurt => {
                        sfx.write(Sfx::at("hurt", at));
                        if near_me {
                            shake.add(0.35);
                        }
                        bursts.write(Burst::new(at, Color::srgb(0.9, 0.3, 0.3)).count(6).speed(50.0));
                    }
                    Fx::Blackout => {
                        sfx.write(Sfx::at("blackout", at));
                    }
                    Fx::Respawn => {
                        sfx.write(Sfx::at("respawn", at));
                    }
                    Fx::CreatureHit => {
                        sfx.write(Sfx::at("creature_hit", at));
                        bursts.write(Burst::new(at, Color::srgb(0.7, 0.9, 0.4)).count(5).speed(45.0));
                    }
                    Fx::CreatureDie(kind) => {
                        sfx.write(Sfx::at(die_sound(kind), at));
                        bursts.write(
                            Burst::new(at, Color::srgb(0.6, 0.85, 0.35))
                                .count(16)
                                .speed(70.0)
                                .life(0.6),
                        );
                    }
                    Fx::CreatureCall(kind) => {
                        sfx.write(Sfx::at(call_sound(kind), at));
                        if kind == CreatureKind::BurrowMaw && me.is_some_and(|m| m.distance(pos) < 80.0) {
                            shake.add(0.3);
                        }
                    }
                    Fx::WebHit => {
                        sfx.write(Sfx::at("web_hit", at));
                    }
                    Fx::Place => {
                        sfx.write(Sfx::at("drop", at));
                    }
                    Fx::DomePlaced => {
                        sfx.write(Sfx::at("dome_place", at));
                        shake.add(0.25);
                    }
                    Fx::Craft => {
                        sfx.write(Sfx::at("craft", at));
                    }
                    Fx::Sell => {
                        sfx.write(Sfx::at("sell", at));
                    }
                    Fx::Buy => {
                        sfx.write(Sfx::at("buy", at));
                    }
                    Fx::Unlock => {
                        sfx.write(Sfx::ui("unlock"));
                    }
                    Fx::PodLand => {
                        sfx.write(Sfx::at("drop_pod_land", at));
                        shake.add(0.2);
                        bursts.write(
                            Burst::new(at, Color::srgb(0.7, 0.66, 0.6))
                                .count(20)
                                .speed(70.0)
                                .life(0.7),
                        );
                    }
                    Fx::Dawn => {
                        sfx.write(Sfx::ui("day_start"));
                    }
                    Fx::Dusk => {
                        sfx.write(Sfx::ui("night_start"));
                    }
                    Fx::Thunder => {
                        *thunder += 1;
                        sfx.write(
                            Sfx::ui(if thunder.is_multiple_of(2) {
                                "thunder_1"
                            } else {
                                "thunder_2"
                            })
                            .volume(0.7),
                        );
                    }
                }
            }
        }
    }
}

fn place_sound(in_dome: bool, underground: bool, night: bool, band: Band) -> &'static str {
    if in_dome {
        "amb_dome"
    } else if !underground {
        if night {
            "amb_surface_night"
        } else {
            "amb_surface_day"
        }
    } else {
        match band {
            Band::Abyss => "amb_abyss",
            Band::Deeps => "amb_deep",
            _ => "amb_cave",
        }
    }
}

fn weather_sound(kind: WeatherKind) -> Option<&'static str> {
    match kind {
        WeatherKind::Rain => Some("amb_rain"),
        WeatherKind::Mist => Some("amb_mist"),
        WeatherKind::Clear => None,
    }
}

/// Picks the ambience layers and music for where this player is, and nags
/// when the suit is nearly out of air.
pub fn soundscape(
    session: Res<Session>,
    mut ambience: ResMut<Ambience>,
    mut music: ResMut<MusicTrack>,
    mut sfx: MessageWriter<Sfx>,
    time: Res<Time>,
    mut beep: Local<f32>,
) {
    let (Some(colony), Some(me)) = (&session.colony, session.player()) else {
        return;
    };
    let (x, y) = me.center().cell();
    let band = colony.profile.band(x, y);
    let underground = y > colony.profile.surface_at(x) + 24;
    let night = colony.clock.is_night();
    let place = Some(place_sound(me.in_dome.is_some(), underground, night, band));
    let weather = if underground {
        None
    } else {
        weather_sound(colony.weather.kind)
    };
    let wanted = Ambience(place, weather);
    if *ambience != wanted {
        *ambience = wanted;
    }
    let track = if underground && band != Band::Shallows {
        MusicTrack::Deep
    } else if night {
        MusicTrack::Night
    } else {
        MusicTrack::Day
    };
    if *music != track && *music != MusicTrack::Ending {
        *music = track;
    }

    *beep -= time.delta_secs();
    if me.alive() && me.suit && me.o2 < me.stats().o2_capacity * 0.25 && *beep <= 0.0 {
        *beep = if me.o2 < 8.0 { 0.7 } else { 1.5 };
        sfx.write(Sfx::ui("o2_low"));
    }
}
