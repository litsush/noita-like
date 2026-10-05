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
//! * `--credits <n>` sets the colony's credits.
//! * `--open <inventory|chest|synth|terminal>` walks the player to that
//!   fixture in the starter dome and opens its window.
//! * `--overlay <water|power|both>` starts with overlays on; `--ship <secs>`
//!   starts with the colony ship that many seconds into its descent.
//! * `--setup <farm>` builds a demo base beside the starter dome (domes with
//!   crops at every stage, a pylon) and puts the player in it.
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
        app.add_systems(
            OnEnter(AppState::InGame),
            apply_world_flags.before(crate::player::spawn_local_player),
        );
    }
}

/// Applies the flags that change a freshly started world (hosts only).
fn apply_world_flags(
    mut session: ResMut<Session>,
    mut panels: ResMut<crate::panels::Panels>,
    mut overlays: ResMut<crate::overlays::Overlays>,
) {
    if let Some(o) = arg_value("--overlay") {
        overlays.water = o != "power";
        overlays.power = o != "water";
    }
    if !session.is_authority() {
        return;
    }
    let me = session.me;
    let session = &mut *session;
    let mut world = session.world.as_mut();
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
    if let Some(c) = arg_value("--credits").and_then(|s| s.parse::<i64>().ok()) {
        colony.credits = c;
    }
    if let Some(what) = arg_value("--open") {
        use crate::panels::Open;
        use sbct_sim::colony::actions::Container;
        use sbct_sim::colony::domes::{Fixture, fixture_pos};
        let home = colony.home;
        let (fixture, open) = match what.as_str() {
            "chest" => (Fixture::Chest(0), Open::Container(Container::DomeChest(home, 0))),
            "synth" => (Fixture::Synthesizer, Open::Synth),
            "terminal" => (Fixture::Terminal, Open::Terminal),
            _ => (Fixture::Bunk(3), Open::Inventory),
        };
        let pos = colony.ents.get(&home).and_then(|e| {
            let EntKindDome(d) = dome_of(e)?;
            d.fixtures()
                .iter()
                .find(|f| f.fixture == fixture)
                .map(|f| fixture_pos(e.pos, f))
        });
        if let (Some(pos), Some(p)) = (pos, colony.players.get_mut(&me)) {
            p.pose.pos = pos;
            p.in_dome = Some(home);
            p.warp += 1;
            panels.open = Some(open);
        }
    }
    if let Some(t) = arg_value("--ship").and_then(|s| s.parse::<f32>().ok()) {
        colony.ship = sbct_sim::colony::readiness::Ship::Landing(t);
    }
    if arg_value("--setup").as_deref() == Some("farm")
        && let Some(world) = world.as_mut()
    {
        setup_farm(colony, world, me);
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

struct EntKindDome(sbct_sim::colony::items::DomeKind);

fn dome_of(e: &sbct_sim::colony::Ent) -> Option<EntKindDome> {
    match &e.kind {
        sbct_sim::colony::EntKind::Dome(d) => Some(EntKindDome(d.kind)),
        _ => None,
    }
}

/// A small working base for screenshots and manual testing.
fn setup_farm(
    colony: &mut sbct_sim::colony::Colony,
    world: &mut sbct_sim::World,
    me: sbct_sim::colony::PlayerKey,
) {
    use sbct_sim::colony::domes::{Dome, build_cells};
    use sbct_sim::colony::items::{DomeKind, MachineKind};
    use sbct_sim::colony::{EntKind, Machine};
    let home = colony.ents[&colony.home].pos;
    let mut x = home.x + 108.0 + 30.0;
    let mut first = None;
    for (kind, species) in [
        (DomeKind::Green, vec![0u16, 2, 6, 10]),
        (DomeKind::Dark, vec![15, 16, 17, 15]),
        (DomeKind::Aqua, vec![18, 19]),
        (DomeKind::Kitchen, vec![]),
    ] {
        let (w, _) = kind.size();
        x += w as f32 / 2.0;
        let ground = sbct_sim::colony::build::ground_below(world, x as i32, home.y as i32 - 60, 140)
            .unwrap_or(home.y as i32);
        let pos = v2(x, ground as f32);
        build_cells(world, kind, pos);
        let mut dome = Dome::new(kind, format!("{} 1", kind.name()));
        for (i, s) in species.iter().enumerate() {
            dome.planters[i].species = Some(*s);
            dome.planters[i].growth = [1.0, 0.7, 0.4, 0.15][i % 4];
            dome.planters[i].moisture = 1.0;
        }
        dome.reserve = 20.0;
        let id = colony.spawn(pos, EntKind::Dome(dome));
        first.get_or_insert((id, pos));
        x += w as f32 / 2.0 + 14.0;
        if kind == DomeKind::Green {
            let mut pylon = Machine::new(MachineKind::Pylon, "Charging Pylon 1".into());
            pylon.inv.add(Item::Crop(0), 6, 1.0);
            colony.spawn(v2(x - 6.0, ground as f32), EntKind::Machine(pylon));
            // A buried main with a full tank at its head feeds every dome.
            use sbct_sim::colony::water::{run, tile_of};
            let from = tile_of(v2(home.x + 120.0, ground as f32 + 10.0));
            for t in run(from, (from.0 + 95, from.1)) {
                colony.pipes.insert(t);
            }
            let mut tank = Machine::new(MachineKind::Tank, "Water Tank 1".into());
            tank.store = 400.0;
            let tank_ground =
                sbct_sim::colony::build::ground_below(world, home.x as i32 + 124, home.y as i32 - 60, 140)
                    .unwrap_or(ground);
            colony.spawn(v2(home.x + 124.0, tank_ground as f32), EntKind::Machine(tank));
            for t in run(from, (from.0, tile_of(v2(0.0, tank_ground as f32)).1)) {
                colony.pipes.insert(t);
            }
        }
    }
    if let (Some((id, pos)), Some(p)) = (first, colony.players.get_mut(&me)) {
        p.pose.pos = v2(pos.x - 20.0, pos.y);
        p.in_dome = Some(id);
        p.warp += 1;
        p.inv.add(Item::WateringCan, 1, 1.0);
        p.inv.add(Item::Seed(3), 5, 1.0);
        p.inv.add(Item::Fertilizer, 5, 1.0);
        p.can_water = 10.0;
    }
}
