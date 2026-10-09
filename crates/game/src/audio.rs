//! Retro sound: every effect and track is a generated WAV under
//! `assets/audio` (see `tools/gen_audio.py`). Screens and the match queue
//! cues by name; this plays them and keeps the right music running.

use std::collections::HashMap;

use bevy::audio::{PlaybackSettings, Volume};
use bevy::prelude::*;

use crate::AppState;
use crate::versus::{Phase, Versus};

pub const EFFECTS: &[&str] = &[
    "ui_click",
    "ready",
    "join",
    "round_start",
    "hit",
    "hit_heavy",
    "blocked",
    "clip",
    "whoosh",
    "clash",
    "sever",
    "down",
    "finish",
    "slam",
    "spit",
    "grab",
    "ink",
    "poison",
    "eat",
    "eat_meat",
    "starving",
    "surprise",
    "frenzy",
    "round_won",
    "round_lost",
    "round_draw",
    "evolve",
];

pub const TRACKS: &[&str] = &["music_menu", "music_lobby", "music_fight"];

#[derive(Resource)]
pub struct Sounds {
    effects: HashMap<&'static str, Handle<AudioSource>>,
    tracks: HashMap<&'static str, Handle<AudioSource>>,
    /// The music entity and which track it is.
    music: Option<(Entity, &'static str)>,
    pub music_volume: f32,
    pub effect_volume: f32,
}

/// Effects screens ask for (button clicks and the like).
#[derive(Resource, Default)]
pub struct UiSfx(pub Vec<&'static str>);

#[derive(Component)]
struct Music;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiSfx>()
            .add_systems(Startup, load)
            .add_systems(Update, (play_queued, music));
    }
}

fn load(mut commands: Commands, assets: Res<AssetServer>) {
    let effects = EFFECTS
        .iter()
        .map(|n| (*n, assets.load(format!("audio/{n}.wav"))))
        .collect();
    let tracks = TRACKS
        .iter()
        .map(|n| (*n, assets.load(format!("audio/{n}.wav"))))
        .collect();
    commands.insert_resource(Sounds {
        effects,
        tracks,
        music: None,
        music_volume: 0.45,
        effect_volume: 0.8,
    });
}

impl Sounds {
    pub fn play(&self, commands: &mut Commands, name: &str, volume: f32) {
        if let Some(h) = self.effects.get(name) {
            commands.spawn((
                AudioPlayer::new(h.clone()),
                PlaybackSettings::DESPAWN.with_volume(Volume::Linear(volume * self.effect_volume)),
            ));
        } else {
            warn!("no sound named {name}");
        }
    }
}

/// Plays whatever the screens and the match queued this frame.
fn play_queued(
    mut commands: Commands,
    sounds: Option<Res<Sounds>>,
    mut ui: ResMut<UiSfx>,
    versus: Option<ResMut<Versus>>,
) {
    let Some(sounds) = sounds else { return };
    for name in ui.0.drain(..) {
        sounds.play(&mut commands, name, 0.7);
    }
    if let Some(mut v) = versus {
        // Many of the same cue in one frame is just one, louder.
        let mut cues: Vec<(&'static str, f32)> = Vec::new();
        for (name, vol) in v.sounds.drain(..) {
            match cues.iter_mut().find(|(n, _)| *n == name) {
                Some(c) => c.1 = (c.1 + 0.15).min(1.0),
                None => cues.push((name, vol)),
            }
        }
        for (name, vol) in cues {
            sounds.play(&mut commands, name, vol);
        }
    }
}

/// Keeps the track matching where the player is.
fn music(
    mut commands: Commands,
    sounds: Option<ResMut<Sounds>>,
    state: Res<State<AppState>>,
    versus: Option<Res<Versus>>,
) {
    let Some(mut sounds) = sounds else { return };
    let want: Option<&'static str> = match state.get() {
        AppState::Menu | AppState::Connecting => Some("music_menu"),
        AppState::Versus => match versus.map(|v| v.phase) {
            Some(Phase::Fight) => Some("music_fight"),
            Some(_) => Some("music_lobby"),
            None => Some("music_menu"),
        },
        AppState::Ecosystem | AppState::InGame => Some("music_lobby"),
    };
    if sounds.music.map(|(_, t)| t) == want {
        return;
    }
    if let Some((e, _)) = sounds.music.take() {
        commands.entity(e).despawn();
    }
    if let Some(track) = want
        && let Some(h) = sounds.tracks.get(track).cloned()
    {
        let e = commands
            .spawn((
                Music,
                AudioPlayer::new(h),
                PlaybackSettings::LOOP.with_volume(Volume::Linear(sounds.music_volume)),
            ))
            .id();
        sounds.music = Some((e, track));
    }
}
