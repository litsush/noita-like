//! Developer flags for unattended play-testing:
//!
//! * `--screenshot <dir>` saves a screenshot every few seconds.
//! * `--quit-after <secs>` exits after that many seconds.
//! * `--window <WxH>` sets the window size; `--ui-scale <x>` the UI scale.
//! * `--fps` logs frame rate once a second.
//! * `--time <secs>` sets the time of day when a world starts (0 = dawn,
//!   480 = dusk); `--weather <clear|mist|rain>` sets the weather.
//! * `--goto <x,y>` moves the player there when a world starts.
//! * `--give <Item:count,...>` puts items in the player's inventory
//!   (`Iron:50,Gold:10`).
//! * `--o2 <percent>` sets the atmosphere's oxygen.
//!
//! Set `SBCT_SAVE_DIR` when testing so nothing touches your real saves.

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};
use sbct_sim::colony::WeatherKind;
use sbct_sim::colony::geom::v2;
use sbct_sim::colony::items::Item;

use crate::AppState;
use crate::session::Session;

#[derive(Resource)]
struct Screenshots {
    dir: String,
    every: f32,
    next: f32,
    count: u32,
}

#[derive(Resource)]
struct QuitAfter(f32);

/// Window size from `--window 1920x1080`, if given.
pub fn window_size() -> Option<(u32, u32)> {
    let v = arg_value("--window")?;
    let (w, h) = v.split_once('x')?;
    Some((w.parse().ok()?, h.parse().ok()?))
}

pub fn arg_value(flag: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1).cloned())
}

pub fn has_flag(flag: &str) -> bool {
    std::env::args().any(|a| a == flag)
}

pub struct DevPlugin;

impl Plugin for DevPlugin {
    fn build(&self, app: &mut App) {
        if let Some(dir) = arg_value("--screenshot") {
            let _ = std::fs::create_dir_all(&dir);
            app.insert_resource(Screenshots {
                dir,
                every: 4.0,
                next: 3.0,
                count: 0,
            })
            .add_systems(Update, take_screenshots);
        }
        if let Some(scale) = arg_value("--ui-scale").and_then(|s| s.parse::<f32>().ok()) {
            app.add_systems(Startup, move |mut config: ResMut<crate::settings::Config>| {
                config.settings.ui_scale = scale;
                config.ephemeral = true;
            });
        }
        if has_flag("--fps") {
            app.add_plugins((
                bevy::diagnostic::FrameTimeDiagnosticsPlugin::default(),
                bevy::diagnostic::LogDiagnosticsPlugin::default(),
            ));
        }
        if let Some(secs) = arg_value("--quit-after").and_then(|s| s.parse().ok()) {
            app.insert_resource(QuitAfter(secs))
                .add_systems(Update, quit_after);
        }
        app.add_systems(OnEnter(AppState::InGame), apply_world_flags);
    }
}

/// Applies the flags that change a freshly started world (hosts only).
fn apply_world_flags(mut session: ResMut<Session>) {
    if !session.is_authority() {
        return;
    }
    let me = session.me;
    let Some(colony) = &mut session.colony else { return };
    if let Some(t) = arg_value("--time").and_then(|s| s.parse::<f32>().ok()) {
        colony.clock.time = t;
    }
    if let Some(w) = arg_value("--weather") {
        colony.weather.kind = match w.as_str() {
            "rain" => WeatherKind::Rain,
            "mist" => WeatherKind::Mist,
            _ => WeatherKind::Clear,
        };
        colony.weather.remaining = 600.0;
    }
    if let Some(o2) = arg_value("--o2").and_then(|s| s.parse::<f32>().ok()) {
        colony.atmosphere.o2 = o2;
    }
    if let Some(list) = arg_value("--give") {
        let species = colony.species.clone();
        for part in list.split(',') {
            let (name, count) = part.split_once(':').unwrap_or((part, "1"));
            let count = count.parse().unwrap_or(1);
            let wanted = name.to_lowercase().replace(['_', ' ', '-'], "");
            let item = Item::FIXED
                .into_iter()
                .find(|i| i.name(&species).to_lowercase().replace([' ', '-'], "") == wanted);
            match (item, colony.players.get_mut(&me)) {
                (Some(item), Some(p)) => {
                    p.inv.add(item, count, 1.0);
                    p.touch();
                }
                _ => warn!("--give: unknown item {name}"),
            }
        }
    }
    if let Some(at) = arg_value("--goto")
        && let Some((x, y)) = at.split_once(',')
        && let (Ok(x), Ok(y)) = (x.parse::<f32>(), y.parse::<f32>())
        && let Some(p) = colony.players.get_mut(&me)
    {
        p.pose.pos = v2(x, y);
        p.warp += 1;
    }
}

fn take_screenshots(mut commands: Commands, time: Res<Time<Real>>, mut shots: ResMut<Screenshots>) {
    if time.elapsed_secs() < shots.next {
        return;
    }
    shots.next += shots.every;
    let path = format!("{}/shot{:03}.png", shots.dir, shots.count);
    shots.count += 1;
    commands
        .spawn(Screenshot::primary_window())
        .observe(save_to_disk(path));
}

fn quit_after(time: Res<Time<Real>>, limit: Res<QuitAfter>, mut exit: MessageWriter<AppExit>) {
    if time.elapsed_secs() > limit.0 {
        exit.write(AppExit::Success);
    }
}
