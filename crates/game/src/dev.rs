//! Developer flags for unattended play-testing:
//!
//! * `--autoplay` lets a bot play runs (see `run::input::bot_input`).
//! * `--screenshot <dir>` saves a screenshot every few seconds.
//! * `--quit-after <secs>` exits after that many seconds.
//! * `--give <all|ItemName>` starts runs with items (e.g. `--give SparkRod`).
//! * `--window <WxH>` sets the window size; `--ui-scale <x>` the UI scale.
//! * `--test-toasts` shows a few achievement toasts at the start of a run.
//! * `--fps` logs frame rate once a second.
//! * `--layer <0-5>` starts runs in a deeper layer (5 = beside the core).
//! * `--menu-screen <new-run|unlocks|items|settings|controls|multiplayer>` opens a menu page.
//!
//! Set `SBCT_SAVE_DIR` when testing so bot achievements don't touch your save.

use bevy::prelude::*;
use bevy::render::view::screenshot::{Screenshot, save_to_disk};

#[derive(Resource)]
struct Screenshots {
    dir: String,
    every: f32,
    next: f32,
    count: u32,
}

#[derive(Resource)]
struct QuitAfter(f32);

/// Items granted at the start of a run.
#[derive(Resource)]
pub struct GiveItems(pub Vec<crate::run::items::ItemId>);

/// Start runs in this layer instead of on the surface.
#[derive(Resource)]
pub struct StartLayer(pub usize);

/// Window size from `--window 1920x1080`, if given.
pub fn window_size() -> Option<(u32, u32)> {
    let v = arg_value("--window")?;
    let (w, h) = v.split_once('x')?;
    Some((w.parse().ok()?, h.parse().ok()?))
}

fn arg_value(flag: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    args.iter()
        .position(|a| a == flag)
        .and_then(|i| args.get(i + 1).cloned())
}

pub struct DevPlugin;

impl Plugin for DevPlugin {
    fn build(&self, app: &mut App) {
        if std::env::args().any(|a| a == "--autoplay") {
            app.insert_resource(crate::run::input::Autoplay::default());
        }
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
        if let Some(which) = arg_value("--give") {
            use crate::run::items::ItemId;
            let names: Vec<&str> = which.split(',').collect();
            let items = ItemId::ALL
                .into_iter()
                .filter(|i| names.contains(&"all") || names.contains(&format!("{i:?}").as_str()))
                .collect();
            app.insert_resource(GiveItems(items));
        }
        if let Some(scale) = arg_value("--ui-scale").and_then(|s| s.parse::<f32>().ok()) {
            app.add_systems(Startup, move |mut save: ResMut<crate::run::save::SaveData>| {
                save.settings.ui_scale = scale
            });
        }
        if std::env::args().any(|a| a == "--test-toasts") {
            app.add_systems(
                OnEnter(crate::AppState::Run),
                test_toasts.after(crate::run::RunSetupDone),
            );
        }
        if std::env::args().any(|a| a == "--fps") {
            app.add_plugins((
                bevy::diagnostic::FrameTimeDiagnosticsPlugin::default(),
                bevy::diagnostic::LogDiagnosticsPlugin::default(),
            ));
        }
        if let Some(layer) = arg_value("--layer").and_then(|s| s.parse().ok()) {
            app.insert_resource(StartLayer(layer));
        }
        if let Some(secs) = arg_value("--quit-after").and_then(|s| s.parse().ok()) {
            app.insert_resource(QuitAfter(secs))
                .add_systems(Update, quit_after);
        }
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

fn test_toasts(mut run: ResMut<crate::run::Run>) {
    use crate::run::achievements::AchievementId::*;
    for a in [ObsidianBridge, Untouched, Demolitionist] {
        run.toasts.push_back(crate::run::Toast {
            achievement: a,
            time: 0.0,
        });
    }
}
