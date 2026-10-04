//! Sound: one-shot effects with distance falloff and rate limiting, looping
//! ambience per layer, and music. Gameplay sends [`Sfx`] messages; nothing
//! else needs to know about audio.

use std::collections::HashMap;

use bevy::audio::Volume;
use bevy::prelude::*;

use crate::run::save::SaveData;

/// Play a sound. `pos` (cells) makes it positional: quieter with distance
/// from the listener, silent beyond [`HEARING_RANGE`].
#[derive(Message, Clone)]
pub struct Sfx {
    pub name: &'static str,
    pub pos: Option<Vec2>,
    pub volume: f32,
    /// Random pitch variation (0 = none, 0.1 = ±10%).
    pub pitch_var: f32,
}

impl Sfx {
    pub fn ui(name: &'static str) -> Sfx {
        Sfx { name, pos: None, volume: 1.0, pitch_var: 0.0 }
    }
    pub fn at(name: &'static str, pos: Vec2) -> Sfx {
        Sfx { name, pos: Some(pos), volume: 1.0, pitch_var: 0.08 }
    }
    pub fn volume(mut self, v: f32) -> Self {
        self.volume = v;
        self
    }
    pub fn pitch(mut self, p: f32) -> Self {
        self.pitch_var = p;
        self
    }
}

/// Where the ears are (the player, or the camera).
#[derive(Resource, Default)]
pub struct Listener(pub Vec2);

/// Which looping background track should be playing, if any.
#[derive(Resource, Default, PartialEq, Clone, Copy, Debug)]
pub enum MusicTrack {
    #[default]
    None,
    Menu,
    Ambient(usize),
}

pub const HEARING_RANGE: f32 = 220.0;

/// Base volume per sound, since every file is normalised to the same peak.
fn base_volume(name: &str) -> f32 {
    match name {
        "explosion" => 1.0,
        "explosion_small" | "collapse" | "rumble" | "layer_sting" | "victory" | "game_over" => 0.85,
        "item_get" | "achievement" | "death" | "chest_open" | "shrine_buy" => 0.7,
        "hurt" | "burn" | "sizzle" | "splash" | "acid_hiss" | "spark" => 0.55,
        n if n.starts_with("dig_") => 0.35,
        n if n.starts_with("step_") => 0.22,
        "ui_hover" => 0.25,
        "ui_click" => 0.4,
        "jump" | "land" | "dash" | "climb" => 0.35,
        _ => 0.5,
    }
}

/// Minimum seconds between plays of the same sound, so hundreds of cell
/// events don't become noise.
fn min_interval(name: &str) -> f32 {
    match name {
        "sizzle" | "acid_hiss" | "spark" | "splash" => 0.25,
        "explosion" | "explosion_small" => 0.08,
        "rumble" | "collapse" => 0.8,
        n if n.starts_with("dig_") => 0.09,
        n if n.starts_with("step_") => 0.05,
        "hurt" | "burn" => 0.3,
        "heartbeat" => 0.95,
        "gasp" => 1.5,
        "water_spray" => 0.9,
        "torch_ignite" => 0.15,
        "drown_bubble" => 0.4,
        "pickup" => 0.06,
        _ => 0.03,
    }
}

#[derive(Resource, Default)]
struct SfxState {
    handles: HashMap<&'static str, Handle<AudioSource>>,
    last_played: HashMap<&'static str, f32>,
    seed: u32,
}

#[derive(Component)]
struct MusicPlayer(MusicTrack);

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Sfx>()
            .init_resource::<Listener>()
            .init_resource::<MusicTrack>()
            .init_resource::<SfxState>()
            .add_systems(Update, (play_sfx, sync_music, apply_music_volume));
    }
}

fn play_sfx(
    mut commands: Commands,
    mut reader: MessageReader<Sfx>,
    mut state: ResMut<SfxState>,
    assets: Res<AssetServer>,
    listener: Res<Listener>,
    time: Res<Time<Real>>,
    save: Res<SaveData>,
) {
    let now = time.elapsed_secs();
    let master = save.settings.master_volume * save.settings.sfx_volume;
    for sfx in reader.read() {
        let mut volume = sfx.volume * base_volume(sfx.name) * master;
        if let Some(pos) = sfx.pos {
            let d = pos.distance(listener.0);
            if d > HEARING_RANGE {
                continue;
            }
            let falloff = 1.0 - d / HEARING_RANGE;
            volume *= falloff * falloff;
        }
        if volume < 0.01 {
            continue;
        }
        let last = state.last_played.get(sfx.name).copied().unwrap_or(f32::MIN);
        if now - last < min_interval(sfx.name) {
            continue;
        }
        state.last_played.insert(sfx.name, now);
        let handle = state
            .handles
            .entry(sfx.name)
            .or_insert_with(|| assets.load(format!("audio/{}.wav", sfx.name)))
            .clone();
        state.seed = state.seed.wrapping_mul(1_103_515_245).wrapping_add(12_345);
        let r = (state.seed >> 16) as f32 / 65_535.0 * 2.0 - 1.0;
        commands.spawn((
            AudioPlayer::new(handle),
            PlaybackSettings::DESPAWN
                .with_volume(Volume::Linear(volume))
                .with_speed(1.0 + r * sfx.pitch_var),
        ));
    }
}

fn track_file(track: MusicTrack) -> Option<String> {
    match track {
        MusicTrack::None => None,
        MusicTrack::Menu => Some("audio/music_menu.wav".into()),
        MusicTrack::Ambient(layer) => Some(format!("audio/amb_layer{}.wav", layer.min(4))),
    }
}

fn track_volume(track: MusicTrack, save: &SaveData) -> f32 {
    let s = &save.settings;
    match track {
        MusicTrack::Menu => s.master_volume * s.music_volume * 0.7,
        _ => s.master_volume * s.music_volume * 0.9,
    }
}

/// Swaps the looping track when the wanted one changes.
fn sync_music(
    mut commands: Commands,
    wanted: Res<MusicTrack>,
    players: Query<(Entity, &MusicPlayer)>,
    assets: Res<AssetServer>,
    save: Res<SaveData>,
) {
    let mut has_wanted = false;
    for (e, p) in &players {
        if p.0 == *wanted {
            has_wanted = true;
        } else {
            commands.entity(e).despawn();
        }
    }
    if !has_wanted && let Some(path) = track_file(*wanted) {
        commands.spawn((
            MusicPlayer(*wanted),
            AudioPlayer::new(assets.load(path)),
            PlaybackSettings::LOOP.with_volume(Volume::Linear(track_volume(*wanted, &save))),
        ));
    }
}

fn apply_music_volume(save: Res<SaveData>, mut sinks: Query<(&MusicPlayer, &mut AudioSink)>) {
    if !save.is_changed() {
        return;
    }
    for (p, mut sink) in &mut sinks {
        sink.set_volume(Volume::Linear(track_volume(p.0, &save)));
    }
}
