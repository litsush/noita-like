//! Sound: one-shot effects with distance falloff and rate limiting, two
//! looping ambience layers (place and weather), and music. Gameplay sends [`Sfx`] messages; nothing
//! else needs to know about audio.

use std::collections::HashMap;

use bevy::audio::Volume;
use bevy::prelude::*;

use crate::settings::Config;

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
        Sfx {
            name,
            pos: None,
            volume: 1.0,
            pitch_var: 0.0,
        }
    }
    pub fn at(name: &'static str, pos: Vec2) -> Sfx {
        Sfx {
            name,
            pos: Some(pos),
            volume: 1.0,
            pitch_var: 0.08,
        }
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

/// Which looping music should be playing, if any.
#[derive(Resource, Default, PartialEq, Clone, Copy, Debug)]
pub enum MusicTrack {
    #[default]
    None,
    Menu,
    Day,
    Night,
    Deep,
    Ending,
}

/// Which looping ambience should be playing, by file name.
#[derive(Resource, Default, PartialEq, Clone, Copy, Debug)]
pub struct Ambience(pub Option<&'static str>, pub Option<&'static str>);

pub const HEARING_RANGE: f32 = 220.0;

/// Base volume per sound, since every file is normalised to the same peak.
fn base_volume(name: &str) -> f32 {
    match name {
        "brood_roar" | "maw_erupt" | "dome_place" | "ship_land" | "readiness_all" => 0.9,
        "laser_big" | "stomp" | "fanfare_ready" | "unlock" | "blackout" | "respawn" | "drop_pod_land" => 0.75,
        "laser_fire" | "laser_hit" | "hurt" | "craft_big" | "thunder_1" | "thunder_2" => 0.6,
        "maw_rumble" | "skitter_lunge" | "skitter_die" | "web_spit" | "web_hit" | "leech_latch" => 0.6,
        "skitter_chitter" | "puff_grunt" | "moth_flutter" | "brood_spawn" | "creature_hit" => 0.45,
        "craft" | "sell" | "buy" | "toast" | "codex_new" | "plant_grow" | "day_start" | "night_start" => 0.55,
        n if n.starts_with("dig_") => 0.3,
        n if n.starts_with("step_") => 0.22,
        "pickup" | "drop" | "plant_harvest" | "plant_sow" | "water_pour" | "fertilize" | "pipe_place" => 0.4,
        "o2_low" | "error" | "laser_empty" => 0.5,
        "ui_hover" | "inv_move" => 0.8,
        "ui_click" | "ui_toggle" | "inv_open" | "inv_close" => 0.4,
        "jump" | "land" | "dash" | "swim_stroke" | "splash" => 0.35,
        n if n.ends_with("_loop") => 0.35,
        _ => 0.5,
    }
}

/// Minimum seconds between plays of the same sound, so bursts of events
/// don't become noise. Loops are retriggered while active; their length
/// keeps them from stacking.
fn min_interval(name: &str) -> f32 {
    match name {
        n if n.starts_with("dig_") => 0.09,
        n if n.starts_with("step_") => 0.05,
        "pickup" => 0.05,
        "hurt" | "creature_hit" | "laser_hit" => 0.12,
        "o2_low" => 1.4,
        "skitter_chitter" => 0.8,
        "gasp" | "shiver" | "cough" => 2.5,
        "rocket_loop" => 0.95,
        "glide_loop" | "robot_hover_loop" => 1.45,
        "sprinkler_loop" | "pylon_loop" | "pump_loop" | "water_flow_loop" => 1.95,
        "generator_loop" => 2.95,
        "dome_hum_loop" => 3.95,
        "splash" | "swim_stroke" => 0.3,
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

/// An ambience layer (0 = place, 1 = weather) playing a file.
#[derive(Component)]
struct AmbiencePlayer(usize, &'static str);

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Sfx>()
            .init_resource::<Listener>()
            .init_resource::<MusicTrack>()
            .init_resource::<Ambience>()
            .init_resource::<SfxState>()
            .add_systems(Update, (play_sfx, sync_music, sync_ambience, apply_music_volume));
    }
}

fn play_sfx(
    mut commands: Commands,
    mut reader: MessageReader<Sfx>,
    mut state: ResMut<SfxState>,
    assets: Res<AssetServer>,
    listener: Res<Listener>,
    time: Res<Time<Real>>,
    config: Res<Config>,
) {
    let now = time.elapsed_secs();
    let master = config.settings.master_volume * config.settings.sfx_volume;
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

fn track_file(track: MusicTrack) -> Option<&'static str> {
    Some(match track {
        MusicTrack::None => return None,
        MusicTrack::Menu => "audio/music_menu.wav",
        MusicTrack::Day => "audio/music_day.wav",
        MusicTrack::Night => "audio/music_night.wav",
        MusicTrack::Deep => "audio/music_deep.wav",
        MusicTrack::Ending => "audio/music_ending.wav",
    })
}

fn track_volume(track: MusicTrack, config: &Config) -> f32 {
    let s = &config.settings;
    match track {
        MusicTrack::Menu | MusicTrack::Ending => s.master_volume * s.music_volume * 0.8,
        _ => s.master_volume * s.music_volume * 0.55,
    }
}

fn ambience_volume(config: &Config) -> f32 {
    let s = &config.settings;
    s.master_volume * s.sfx_volume * 0.5
}

/// Swaps the two looping ambience layers when the wanted ones change.
fn sync_ambience(
    mut commands: Commands,
    wanted: Res<Ambience>,
    players: Query<(Entity, &AmbiencePlayer)>,
    assets: Res<AssetServer>,
    config: Res<Config>,
) {
    for (layer, want) in [wanted.0, wanted.1].into_iter().enumerate() {
        let mut has = false;
        for (e, p) in &players {
            if p.0 != layer {
                continue;
            }
            if Some(p.1) == want {
                has = true;
            } else {
                commands.entity(e).despawn();
            }
        }
        if !has && let Some(name) = want {
            commands.spawn((
                AmbiencePlayer(layer, name),
                AudioPlayer::new(assets.load(format!("audio/{name}.wav"))),
                PlaybackSettings::LOOP.with_volume(Volume::Linear(ambience_volume(&config))),
            ));
        }
    }
}

/// Swaps the looping track when the wanted one changes.
fn sync_music(
    mut commands: Commands,
    wanted: Res<MusicTrack>,
    players: Query<(Entity, &MusicPlayer)>,
    assets: Res<AssetServer>,
    config: Res<Config>,
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
            if *wanted == MusicTrack::Ending {
                PlaybackSettings::ONCE
            } else {
                PlaybackSettings::LOOP
            }
            .with_volume(Volume::Linear(track_volume(*wanted, &config))),
        ));
    }
}

fn apply_music_volume(config: Res<Config>, mut sinks: Query<(&MusicPlayer, &mut AudioSink)>) {
    if !config.is_changed() {
        return;
    }
    for (p, mut sink) in &mut sinks {
        sink.set_volume(Volume::Linear(track_volume(p.0, &config)));
    }
}

#[cfg(test)]
mod tests {
    /// String literals on `Sfx::` and `Ambience(` lines and inside
    /// `fn *_sound` helpers must all name a file in `assets/audio`.
    #[test]
    fn every_sound_named_in_code_exists() {
        let src = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let audio = crate::assets::assets_dir().join("audio");
        let mut missing = Vec::new();
        let mut checked = 0;
        let mut files = vec![src];
        while let Some(path) = files.pop() {
            if path.is_dir() {
                files.extend(std::fs::read_dir(&path).unwrap().map(|e| e.unwrap().path()));
                continue;
            }
            if path.extension().is_none_or(|e| e != "rs") || path.ends_with("audio.rs") {
                continue;
            }
            let text = std::fs::read_to_string(&path).unwrap();
            let mut in_helper = false;
            for line in text.lines() {
                if line.starts_with("fn ") || line.starts_with("pub fn ") {
                    in_helper = line.contains("_sound(");
                }
                if !(in_helper || line.contains("Sfx::") || line.contains("Ambience(")) {
                    continue;
                }
                for (i, lit) in line.split('"').enumerate() {
                    let looks_like_sound = i % 2 == 1
                        && !lit.is_empty()
                        && lit
                            .chars()
                            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_');
                    if looks_like_sound {
                        checked += 1;
                        if !audio.join(format!("{lit}.wav")).exists() {
                            missing.push(format!("{lit} ({})", path.display()));
                        }
                    }
                }
            }
        }
        assert!(checked > 30, "only found {checked} sound names");
        assert!(missing.is_empty(), "missing sounds: {missing:?}");
    }
}
