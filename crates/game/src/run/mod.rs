//! "Descent to the Core": the singleplayer roguelike mode.
//!
//! A run is a [`Session`] with no network plus the resources here. The world
//! is generated on a background thread while a loading screen shows; then the
//! game spawns entities from the generator's spawn list and the player
//! digs down until they touch the core or die.

pub mod achievements;
pub mod creatures;
pub mod entities;
pub mod hazards;
pub mod hud;
pub mod input;
pub mod items;
pub mod physics;
pub mod player;
pub mod save;
pub mod tracking;

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;
use std::sync::mpsc::{Receiver, channel};

use bevy::prelude::*;
use sbct_sim::descent::{Descent, Layer, Spawn, generate_descent};
use sbct_sim::rng::Rng;

use crate::AppState;
use crate::audio::{MusicTrack, Sfx};
use crate::session::Session;
use achievements::{AchievementId, Loadout};
use items::{Active, ItemId};
use save::SaveData;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DamageKind {
    Fire,
    Lava,
    Metal,
    Heat,
    Acid,
    Explosion,
    Crushed,
    Drowned,
    Gas,
    Electric,
    Creature,
    Water,
}

impl DamageKind {
    pub fn death_text(self) -> &'static str {
        match self {
            DamageKind::Fire => "Burned to a crisp",
            DamageKind::Lava => "Melted in lava",
            DamageKind::Metal => "Swallowed by molten metal",
            DamageKind::Heat => "Cooked by the planet's heat",
            DamageKind::Acid => "Dissolved in acid",
            DamageKind::Explosion => "Blown to pieces",
            DamageKind::Crushed => "Crushed under falling rock",
            DamageKind::Drowned => "Drowned",
            DamageKind::Gas => "Choked on toxic gas",
            DamageKind::Electric => "Electrocuted",
            DamageKind::Creature => "Eaten by the locals",
            DamageKind::Water => "Scalded by water (Salamander Skin)",
        }
    }
}

const INFLUENCE_SECS: f32 = 15.0;
const INFLUENCE_RADIUS: f32 = 48.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Playing,
    /// Death slow-motion before the summary.
    Dying(f32),
    /// Victory sequence before the summary.
    Won(f32),
    /// Summary screen.
    Over,
}

/// Counters for achievements, cleared each run.
#[derive(Default, Clone)]
pub struct RunStats {
    pub dissolved: u32,
    pub obsidian: u32,
    pub gas_ignited: u32,
    pub electrified: u32,
    pub collapsed: u32,
    pub destroyed: u32,
    pub damage_taken: f32,
    pub cells_dug: u32,
    pub creatures_killed: u32,
}

pub struct Toast {
    pub achievement: AchievementId,
    pub time: f32,
}

#[derive(Resource)]
pub struct Run {
    pub seed: u64,
    pub loadout: Loadout,
    pub elapsed: f32,
    pub phase: Phase,
    pub won: bool,
    pub cause: Option<DamageKind>,
    pub layer: Layer,
    pub max_depth: i32,
    pub ore: u32,
    pub ropes: u32,
    pub torches: u32,
    pub items: Vec<ItemId>,
    /// Index into the player's active items.
    pub selected: usize,
    /// Charges and recharge timers for charge-based actives.
    pub charges: HashMap<ItemId, (u8, f32)>,
    /// Water Canister tank, 0–100.
    pub tank: f32,
    pub stats: RunStats,
    pub earned: Vec<AchievementId>,
    pub toasts: VecDeque<Toast>,
    /// Item just picked up, and how long its popup has shown.
    pub popup: Option<(ItemId, f32)>,
    /// Layer name banner and its age.
    pub banner: Option<(Layer, f32)>,
    pub rng: Rng,
    pub core: Vec2,
    /// Thermal Suit vent waiting to set the world alight.
    pub pending_fire_burst: Option<Vec2>,
    /// Player-caused blasts that should leave ore (Volatile Core).
    pub volatile_spots: Vec<(i32, i32)>,
    /// Where the player recently dug, threw or blew something up, with the
    /// time. Achievements only count sim events near these, so the planet
    /// doing its own thing doesn't hand them out.
    pub influence: VecDeque<(Vec2, f32)>,
    /// Largest damage taken this frame, so the cause of death is the worst hit.
    pub cause_amount: f32,
}

impl Run {
    pub fn new(seed: u64, loadout: Loadout, save: &SaveData, core: Vec2) -> Run {
        let mut run = Run {
            seed,
            loadout,
            elapsed: 0.0,
            phase: Phase::Playing,
            won: false,
            cause: None,
            layer: Layer::Crust,
            max_depth: 0,
            ore: 0,
            ropes: 4,
            torches: 5,
            items: Vec::new(),
            selected: 0,
            charges: HashMap::new(),
            tank: 100.0,
            stats: RunStats::default(),
            earned: Vec::new(),
            toasts: VecDeque::new(),
            popup: None,
            banner: Some((Layer::Crust, 0.0)),
            rng: Rng::new(seed ^ 0x17E5),
            core,
            pending_fire_burst: None,
            volatile_spots: Vec::new(),
            influence: VecDeque::new(),
            cause_amount: 0.0,
        };
        match loadout {
            Loadout::Standard => {}
            Loadout::Gifted => {
                let pool = save.unlocked_items();
                let pick = pool[(run.rng.next_u64() % pool.len() as u64) as usize];
                run.give(pick);
            }
            Loadout::Excavator => {
                run.ropes = 8;
                run.torches = 8;
                run.give(ItemId::CrumblingPick);
                run.give(ItemId::BlastCharges);
            }
        }
        run.popup = None;
        run
    }

    pub fn has(&self, item: ItemId) -> bool {
        self.items.contains(&item)
    }

    /// Adds an item; a new pick replaces the old one.
    pub fn give(&mut self, item: ItemId) {
        if self.has(item) {
            return;
        }
        if item.def().is_pick {
            self.items.retain(|i| !i.def().is_pick);
        }
        self.items.push(item);
        if let Some(Active::Charges { max, .. }) = item.def().active {
            self.charges.insert(item, (max, 0.0));
        }
        self.popup = Some((item, 0.0));
    }

    pub fn actives(&self) -> Vec<ItemId> {
        self.items.iter().copied().filter(|i| i.def().active.is_some()).collect()
    }

    pub fn selected_active(&self) -> Option<ItemId> {
        let actives = self.actives();
        (!actives.is_empty()).then(|| actives[self.selected % actives.len()])
    }

    /// A random unlocked item the player doesn't have yet.
    pub fn roll_item(&mut self, save: &SaveData) -> Option<ItemId> {
        let pool: Vec<ItemId> = save.unlocked_items().into_iter().filter(|i| !self.has(*i)).collect();
        (!pool.is_empty()).then(|| pool[(self.rng.next_u64() % pool.len() as u64) as usize])
    }

    /// Records a player action at `at` for achievement attribution.
    pub fn influence(&mut self, at: Vec2) {
        let now = self.elapsed;
        if self.influence.back().is_some_and(|(p, t)| p.distance(at) < 8.0 && now - t < 0.5) {
            return;
        }
        self.influence.push_back((at, now));
        while self.influence.front().is_some_and(|(_, t)| now - t > INFLUENCE_SECS) {
            self.influence.pop_front();
        }
    }

    /// Whether a sim event at `at` can be credited to the player.
    pub fn influenced(&self, at: Vec2) -> bool {
        self.influence.iter().any(|(p, t)| p.distance(at) < INFLUENCE_RADIUS && self.elapsed - t <= INFLUENCE_SECS)
    }

    pub fn is_playing(&self) -> bool {
        self.phase == Phase::Playing
    }
}

/// Grants an achievement (once ever), saving immediately so it survives a quit.
pub fn grant(save: &mut SaveData, run: &mut Run, id: AchievementId, sfx: &mut MessageWriter<Sfx>) {
    if run.earned.contains(&id) {
        return;
    }
    run.earned.push(id);
    if save.grant(id) {
        run.toasts.push_back(Toast { achievement: id, time: 0.0 });
        sfx.write(Sfx::ui("achievement"));
        save::store(save);
    }
}

/// World generation in progress.
#[derive(Resource)]
pub struct PendingRun {
    rx: Mutex<Receiver<Descent>>,
    seed: u64,
    loadout: Loadout,
}

/// What the generator decided, consumed when entering the run.
#[derive(Resource)]
pub struct RunSetup {
    pub spawns: Vec<Spawn>,
    pub start: Vec2,
}

/// Starts generating a run; the loading screen shows until it's ready.
pub fn start_run(commands: &mut Commands, seed: u64, loadout: Loadout) {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let _ = tx.send(generate_descent(seed));
    });
    commands.insert_resource(PendingRun { rx: Mutex::new(rx), seed, loadout });
    commands.set_state(AppState::Loading);
}

pub fn random_seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_nanos() as u64);
    sbct_sim::rng::hash2(nanos, 3, 9) % 100_000_000
}

fn finish_loading(mut commands: Commands, pending: Option<Res<PendingRun>>, save: Res<SaveData>) {
    let Some(pending) = pending else { return };
    let Ok(descent) = pending.rx.lock().unwrap().try_recv() else { return };
    let start = Vec2::new(descent.start.0 as f32 + 0.5, descent.start.1 as f32 + 1.0);
    let core = Vec2::new(descent.core.0 as f32, descent.core.1 as f32);
    commands.insert_resource(Session::offline(descent.world, start, "Miner".into()));
    commands.insert_resource(RunSetup { spawns: descent.spawns, start });
    commands.insert_resource(Run::new(pending.seed, pending.loadout, &save, core));
    commands.remove_resource::<PendingRun>();
    commands.set_state(AppState::Run);
}

fn on_enter_run(mut save: ResMut<SaveData>, mut music: ResMut<MusicTrack>, mut run: ResMut<Run>, start: Option<Res<crate::dev::StartLayer>>) {
    if let Some(layer) = start.and_then(|l| Layer::ALL.get(l.0).copied()) {
        run.layer = layer;
        run.banner = Some((layer, 0.0));
    }
    save.stats.runs += 1;
    save::store(&save);
    *music = MusicTrack::Ambient(0);
}

fn on_exit_run(mut commands: Commands, mut time: ResMut<Time<Virtual>>) {
    commands.remove_resource::<Run>();
    commands.remove_resource::<RunSetup>();
    commands.remove_resource::<player::RunPlayer>();
    commands.remove_resource::<Session>();
    time.set_relative_speed(1.0);
}

/// Layer banners, depth records and depth achievements.
fn track_depth(
    mut run: ResMut<Run>,
    player: Res<player::RunPlayer>,
    mut save: ResMut<SaveData>,
    mut sfx: MessageWriter<Sfx>,
    mut music: ResMut<MusicTrack>,
    time: Res<Time>,
) {
    if !run.is_playing() {
        return;
    }
    run.elapsed += time.delta_secs();
    let y = player.body.pos.y as i32;
    run.max_depth = run.max_depth.max(y);
    let layer = Layer::at_depth(y);
    if layer > run.layer {
        run.layer = layer;
        run.banner = Some((layer, 0.0));
        sfx.write(Sfx::ui("layer_sting"));
        *music = MusicTrack::Ambient(layer.index());
        if layer >= Layer::UpperMantle {
            if run.stats.damage_taken <= 0.0 {
                grant(&mut save, &mut run, AchievementId::Untouched, &mut sfx);
            }
            if run.elapsed < 240.0 {
                grant(&mut save, &mut run, AchievementId::SpeedDigger, &mut sfx);
            }
        }
        if layer >= Layer::DeepMantle {
            grant(&mut save, &mut run, AchievementId::DeepDiver, &mut sfx);
        }
    }
}

/// Advances death/victory sequences and records the result.
fn update_phase(
    mut run: ResMut<Run>,
    player: Res<player::RunPlayer>,
    real: Res<Time<Real>>,
    mut save: ResMut<SaveData>,
    mut sfx: MessageWriter<Sfx>,
    mut music: ResMut<MusicTrack>,
) {
    let dt = real.delta_secs();
    match run.phase {
        Phase::Playing => {
            if player.hp <= 0.0 {
                run.phase = Phase::Dying(0.0);
                sfx.write(Sfx::ui("death"));
            }
        }
        Phase::Dying(t) => {
            if t > 2.0 {
                run.phase = Phase::Over;
                sfx.write(Sfx::ui("game_over"));
                *music = MusicTrack::None;
                record_result(&mut save, &run);
            } else {
                run.phase = Phase::Dying(t + dt);
            }
        }
        Phase::Won(t) => {
            if t == 0.0 {
                run.won = true;
                grant(&mut save, &mut run, AchievementId::CoreBreaker, &mut sfx);
                sfx.write(Sfx::ui("victory"));
                *music = MusicTrack::None;
            }
            if t > 3.5 {
                run.phase = Phase::Over;
                record_result(&mut save, &run);
            } else {
                run.phase = Phase::Won(t + dt.max(1e-4));
            }
        }
        Phase::Over => {}
    }
}

fn record_result(save: &mut SaveData, run: &Run) {
    save.stats.best_depth = save.stats.best_depth.max(run.max_depth);
    if run.won {
        save.stats.wins += 1;
        save.stats.best_time = Some(save.stats.best_time.map_or(run.elapsed, |t| t.min(run.elapsed)));
    }
    save::store(save);
}

/// Slow motion while dying, brief freezes on big hits.
fn apply_time_scale(
    run: Res<Run>,
    stop: Res<crate::fx::HitStop>,
    mut time: ResMut<Time<Virtual>>,
    paused: Res<hud::Paused>,
) {
    let speed = if paused.0 || run.phase == Phase::Over {
        0.0
    } else if stop.0 > 0.0 {
        0.05
    } else {
        match run.phase {
            Phase::Dying(t) => (0.2 + t * 0.2).min(1.0),
            Phase::Won(_) => 0.5,
            _ => 1.0,
        }
    };
    if (time.relative_speed() - speed).abs() > f32::EPSILON {
        time.set_relative_speed(speed);
    }
}

fn running(run: Option<Res<Run>>, player: Option<Res<player::RunPlayer>>) -> bool {
    run.is_some() && player.is_some()
}

pub struct RunPlugin;

impl Plugin for RunPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(save::load())
            .init_resource::<hud::Paused>()
            .init_resource::<input::PlayerInput>()
            .add_systems(Update, finish_loading.run_if(in_state(AppState::Loading)))
            .add_systems(EguiLoading, hud::loading_ui.run_if(in_state(AppState::Loading)))
            .add_systems(
                OnEnter(AppState::Run),
                (
                    on_enter_run,
                    crate::render::spawn_world_view,
                    player::spawn_player,
                    entities::spawn_entities,
                    creatures::spawn_creatures,
                )
                    .chain(),
            )
            .add_systems(OnExit(AppState::Run), (crate::render::cleanup_world_view, on_exit_run))
            .add_systems(
                FixedUpdate,
                crate::session::step_world.run_if(in_state(AppState::Run).and_then(resource_exists::<Session>)),
            )
            .add_systems(
                Update,
                (
                    hud::toggle_pause,
                    input::read_input.run_if(not(resource_exists::<input::Autoplay>)),
                    input::bot_input.run_if(resource_exists::<input::Autoplay>),
                    player::player_move,
                    player::player_dig,
                    player::player_actions,
                    entities::update_charges,
                    entities::interact,
                    entities::update_projectiles,
                    entities::update_bombs,
                    entities::update_suns,
                    entities::update_torches,
                    tracking::process_sim_events,
                    hazards::environment,
                    creatures::update_creatures,
                    track_depth,
                    update_phase,
                    apply_time_scale,
                )
                    .chain()
                    .run_if(in_state(AppState::Run).and_then(running)),
            )
            .add_systems(
                Update,
                (
                    player::animate_player,
                    entities::animate_props,
                    creatures::animate_creatures,
                    player::follow_camera,
                    crate::render::upload_dirty_chunks,
                )
                    .chain()
                    .after(apply_time_scale)
                    .run_if(in_state(AppState::Run).and_then(running)),
            )
            .add_systems(
                bevy_egui::EguiPrimaryContextPass,
                hud::run_hud.run_if(in_state(AppState::Run).and_then(running)),
            );
    }
}

/// The egui pass schedule, named for readability above.
use bevy_egui::EguiPrimaryContextPass as EguiLoading;
