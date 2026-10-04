//! "Descent to the Core": the singleplayer roguelike mode.
//!
//! A run is a [`Session`] with no network plus the resources here. The world
//! is generated on a background thread while a loading screen shows; then the
//! game spawns entities from the generator's spawn list and the player
//! digs down until they touch the core or die.

pub mod achievements;
pub mod background;
pub mod creatures;
pub mod entities;
pub mod hazards;
pub mod hud;
pub mod input;
pub mod juice;
pub mod lighting;
pub mod lore;
pub mod physics;
pub mod player;
pub mod save;
pub mod scrolls;
pub mod smart_dig;
pub mod spells;
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
use achievements::{AchievementId, StartChoice};
use save::SaveData;
use scrolls::{FusionId, School, ScrollId};

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
    /// Seconds spent in darkness in a row (Lightless).
    pub dark_time: f32,
    /// Fungus cells grown by the player's magic (Gardener).
    pub fungus_grown: u32,
    /// Fallen apprentices searched (Grave Robber).
    pub remains_searched: u32,
}

pub struct Toast {
    pub achievement: AchievementId,
    pub time: f32,
}

#[derive(Resource)]
pub struct Run {
    pub seed: u64,
    pub start_choice: StartChoice,
    pub elapsed: f32,
    pub phase: Phase,
    pub won: bool,
    pub cause: Option<DamageKind>,
    pub layer: Layer,
    pub max_depth: i32,
    /// Aether shards: the currency for shrines.
    pub shards: u32,
    /// Light orb charges and the recharge timer for the next one.
    pub light_charges: u8,
    pub light_max: u8,
    pub light_timer: f32,
    /// The staff's glow is dimmed (hides you from lightseekers).
    pub staff_dimmed: bool,
    pub first_school: School,
    /// Bound at the first altar (or at the start for Twin-Souled).
    pub second_school: Option<School>,
    /// Learned scrolls, in order.
    pub scrolls: Vec<ScrollId>,
    /// The pair's fusion, once awakened.
    pub fusion: Option<FusionId>,
    /// Index into the castable spells.
    pub selected: usize,
    pub mana: f32,
    pub max_mana: f32,
    /// Seconds until each spell can be cast again.
    pub cooldowns: HashMap<Spell, f32>,
    pub stats: RunStats,
    pub earned: Vec<AchievementId>,
    pub toasts: VecDeque<Toast>,
    /// Something to announce in the centre popup, and how long it has shown.
    pub popup: Option<(Popup, f32)>,
    /// Layer name banner and its age.
    pub banner: Option<(Layer, f32)>,
    /// The attunement altar's offer while its choice is open, the altar it
    /// came from, and the choice once made in the UI.
    pub attunement: Option<Vec<ScrollId>>,
    pub attune_altar: Option<Entity>,
    pub attune_pick: Option<usize>,
    /// A journal page being read.
    pub journal: Option<u16>,
    pub rng: Rng,
    pub core: Vec2,
    /// Ember Heart vent waiting to set the world alight.
    pub pending_fire_burst: Option<Vec2>,
    /// Player-caused blasts that should leave shard veins (Volatile Core).
    pub volatile_spots: Vec<(i32, i32)>,
    /// Where the player recently dug, cast or blew something up, with the
    /// time. Achievements only count sim events near these, so the planet
    /// doing its own thing doesn't hand them out.
    pub influence: VecDeque<(Vec2, f32)>,
    /// Largest damage taken this frame, so the cause of death is the worst hit.
    pub cause_amount: f32,
}

/// Anything castable: a scroll's active spell or the awakened fusion.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Spell {
    Scroll(ScrollId),
    Fusion(FusionId),
}

impl Spell {
    pub fn name(self) -> &'static str {
        match self {
            Spell::Scroll(s) => s.def().name,
            Spell::Fusion(f) => f.def().name,
        }
    }

    pub fn icon(self) -> usize {
        match self {
            Spell::Scroll(s) => s.icon(),
            Spell::Fusion(f) => f.icon(),
        }
    }

    pub fn desc(self) -> &'static str {
        match self {
            Spell::Scroll(s) => s.def().desc,
            Spell::Fusion(f) => f.def().desc,
        }
    }

    /// Mana cost and cooldown (channelled spells: cost per second, no cooldown).
    pub fn cost(self) -> (f32, f32) {
        match self {
            Spell::Scroll(s) => match s.def().kind {
                scrolls::Kind::Active { cost, cooldown } => (cost, cooldown),
                scrolls::Kind::Channel { cost_per_sec } => (cost_per_sec, 0.0),
                _ => (0.0, 0.0),
            },
            Spell::Fusion(f) => (f.def().cost, f.def().cooldown),
        }
    }

    pub fn channelled(self) -> bool {
        matches!(self, Spell::Scroll(s) if matches!(s.def().kind, scrolls::Kind::Channel { .. }))
    }

    /// The school whose colours and sounds the spell uses.
    pub fn school(self) -> Option<School> {
        match self {
            Spell::Scroll(s) => s.def().school,
            Spell::Fusion(f) => Some(f.def().schools.0),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Popup {
    Learned(ScrollId),
    /// An off-school scroll broken down into shards.
    Shattered(ScrollId),
    Fusion(FusionId),
    Attuned(School),
}

pub const SHATTER_SHARDS: u32 = 15;

impl Run {
    pub fn new(
        seed: u64,
        choice: StartChoice,
        first: School,
        second: Option<School>,
        save: &SaveData,
        core: Vec2,
    ) -> Run {
        let mut run = Run {
            seed,
            start_choice: choice,
            elapsed: 0.0,
            phase: Phase::Playing,
            won: false,
            cause: None,
            layer: Layer::Crust,
            max_depth: 0,
            shards: 0,
            light_charges: 3,
            light_max: 3,
            light_timer: 0.0,
            staff_dimmed: false,
            first_school: first,
            second_school: second.filter(|&s| s != first),
            scrolls: Vec::new(),
            fusion: None,
            selected: 0,
            mana: 100.0,
            max_mana: 100.0,
            cooldowns: HashMap::new(),
            stats: RunStats::default(),
            earned: Vec::new(),
            toasts: VecDeque::new(),
            popup: None,
            banner: Some((Layer::Crust, 0.0)),
            attunement: None,
            attune_altar: None,
            attune_pick: None,
            journal: None,
            rng: Rng::new(seed ^ 0x17E5),
            core,
            pending_fire_burst: None,
            volatile_spots: Vec::new(),
            influence: VecDeque::new(),
            cause_amount: 0.0,
        };
        let first_pool: Vec<ScrollId> = save
            .scrolls_for(&[first])
            .into_iter()
            .filter(|s| s.def().school == Some(first))
            .collect();
        match choice {
            StartChoice::Initiate | StartChoice::TwinSouled => {}
            StartChoice::Prodigy => {
                if let Some(sc) = run.pick(&first_pool) {
                    run.learn(sc);
                }
            }
            StartChoice::Scavenger => {
                run.shards = 40;
                run.light_max = 4;
                run.light_charges = 4;
            }
            StartChoice::ArchmagesHeir => {
                if let Some(sc) = run.pick(&first_pool) {
                    run.learn(sc);
                }
                run.learn(ScrollId::Blink);
            }
        }
        run.popup = None;
        run
    }

    fn pick(&mut self, pool: &[ScrollId]) -> Option<ScrollId> {
        let pool: Vec<ScrollId> = pool.iter().copied().filter(|s| !self.has(*s)).collect();
        (!pool.is_empty()).then(|| pool[(self.rng.next_u64() % pool.len() as u64) as usize])
    }

    pub fn has(&self, sc: ScrollId) -> bool {
        self.scrolls.contains(&sc)
    }

    /// Knows at least one scroll of the school.
    pub fn knows(&self, school: School) -> bool {
        self.scrolls.iter().any(|s| s.def().school == Some(school))
    }

    /// The wizard's schools (one until attuned).
    pub fn schools(&self) -> Vec<School> {
        std::iter::once(self.first_school)
            .chain(self.second_school)
            .collect()
    }

    pub fn in_my_schools(&self, sc: ScrollId) -> bool {
        sc.def().school.is_none_or(|s| self.schools().contains(&s))
    }

    /// The dig variant in effect: the newest dig scroll learned.
    pub fn dig_variant(&self) -> Option<ScrollId> {
        self.scrolls
            .iter()
            .rev()
            .copied()
            .find(|s| s.def().kind == scrolls::Kind::Dig)
    }

    /// Learns a scroll (a new dig scroll supersedes older ones).
    pub fn learn(&mut self, sc: ScrollId) {
        if self.has(sc) {
            return;
        }
        self.scrolls.push(sc);
        self.popup = Some((Popup::Learned(sc), 0.0));
    }

    /// Takes a scroll found in the world: learned if it belongs to the
    /// wizard's schools, otherwise broken down into shards.
    pub fn take_scroll(&mut self, sc: ScrollId) -> bool {
        if self.in_my_schools(sc) {
            self.learn(sc);
            true
        } else {
            self.shards += SHATTER_SHARDS;
            self.popup = Some((Popup::Shattered(sc), 0.0));
            false
        }
    }

    /// The pair's fusion, if both schools are known and it hasn't awakened yet.
    pub fn fusion_ready(&self) -> Option<FusionId> {
        let second = self.second_school?;
        (self.fusion.is_none() && self.knows(self.first_school) && self.knows(second))
            .then(|| FusionId::for_pair(self.first_school, second))
            .flatten()
    }

    /// Castable spells: active scrolls, then the fusion.
    pub fn spells(&self) -> Vec<Spell> {
        self.scrolls
            .iter()
            .copied()
            .filter(|s| s.is_active())
            .map(Spell::Scroll)
            .chain(self.fusion.map(Spell::Fusion))
            .collect()
    }

    pub fn selected_spell(&self) -> Option<Spell> {
        let spells = self.spells();
        (!spells.is_empty()).then(|| spells[self.selected % spells.len()])
    }

    /// A random scroll for a chest, altar or shrine: from the wizard's
    /// schools (or neutral) that they don't know yet.
    pub fn roll_scroll(&mut self, save: &SaveData) -> Option<ScrollId> {
        let pool = save.scrolls_for(&self.schools());
        self.pick(&pool)
    }

    /// A random unlocked scroll from any school (fallen apprentices carry these).
    pub fn roll_any_scroll(&mut self, save: &SaveData) -> Option<ScrollId> {
        let pool = save.scrolls_for(&School::ALL);
        self.pick(&pool)
    }

    /// The attunement altar's offer: one scroll each from up to three other
    /// unlocked schools.
    pub fn attunement_offer(&mut self, save: &SaveData) -> Vec<ScrollId> {
        let mut others: Vec<School> = save
            .unlocked_schools()
            .into_iter()
            .filter(|&s| s != self.first_school)
            .collect();
        let mut offer = Vec::new();
        while offer.len() < 3 && !others.is_empty() {
            let school = others.swap_remove((self.rng.next_u64() % others.len() as u64) as usize);
            let pool: Vec<ScrollId> = save
                .scrolls_for(&[school])
                .into_iter()
                .filter(|s| s.def().school == Some(school))
                .collect();
            if let Some(sc) = self.pick(&pool) {
                offer.push(sc);
            }
        }
        offer
    }

    /// Binds the second school by taking one of the offered scrolls.
    pub fn attune(&mut self, sc: ScrollId) {
        if let Some(school) = sc.def().school {
            self.second_school = Some(school);
            self.learn(sc);
            self.popup = Some((Popup::Attuned(school), 0.0));
        }
        self.attunement = None;
    }

    /// Records a player action at `at` for achievement attribution.
    pub fn influence(&mut self, at: Vec2) {
        let now = self.elapsed;
        if self
            .influence
            .back()
            .is_some_and(|(p, t)| p.distance(at) < 8.0 && now - t < 0.5)
        {
            return;
        }
        self.influence.push_back((at, now));
        while self
            .influence
            .front()
            .is_some_and(|(_, t)| now - t > INFLUENCE_SECS)
        {
            self.influence.pop_front();
        }
    }

    /// Whether a sim event at `at` can be credited to the player.
    pub fn influenced(&self, at: Vec2) -> bool {
        self.influence
            .iter()
            .any(|(p, t)| p.distance(at) < INFLUENCE_RADIUS && self.elapsed - t <= INFLUENCE_SECS)
    }

    /// Light orbs set flammables alight once you know any Pyromancy.
    pub fn orbs_ignite(&self) -> bool {
        self.knows(School::Pyromancy)
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
        run.toasts.push_back(Toast {
            achievement: id,
            time: 0.0,
        });
        sfx.write(Sfx::ui("achievement"));
        save::store(save);
    }
}

/// System set marking the end of run setup (for things that need the run).
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub struct RunSetupDone;

/// Everything chosen on the wizard screen.
#[derive(Clone, Copy, Debug)]
pub struct RunConfig {
    pub seed: u64,
    pub choice: StartChoice,
    pub first: School,
    /// Only for Twin-Souled.
    pub second: Option<School>,
}

/// World generation in progress.
#[derive(Resource)]
pub struct PendingRun {
    rx: Mutex<Receiver<Descent>>,
    config: RunConfig,
}

/// What the generator decided, consumed when entering the run.
#[derive(Resource)]
pub struct RunSetup {
    pub spawns: Vec<Spawn>,
    pub start: Vec2,
    pub config: RunConfig,
}

/// Starts generating a run; the loading screen shows until it's ready.
pub fn start_run(commands: &mut Commands, config: RunConfig) {
    let (tx, rx) = channel();
    let seed = config.seed;
    std::thread::spawn(move || {
        let _ = tx.send(generate_descent(seed));
    });
    commands.insert_resource(PendingRun {
        rx: Mutex::new(rx),
        config,
    });
    commands.set_state(AppState::Loading);
}

pub fn random_seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64);
    sbct_sim::rng::hash2(nanos, 3, 9) % 100_000_000
}

fn finish_loading(
    mut commands: Commands,
    pending: Option<Res<PendingRun>>,
    save: Res<SaveData>,
    dev_schools: Option<Res<crate::dev::DevSchools>>,
) {
    let Some(pending) = pending else { return };
    let Ok(descent) = pending.rx.lock().unwrap().try_recv() else {
        return;
    };
    let start = Vec2::new(descent.start.0 as f32 + 0.5, descent.start.1 as f32 + 1.0);
    let core = Vec2::new(descent.core.0 as f32, descent.core.1 as f32);
    commands.insert_resource(Session::offline(descent.world, start, "Miner".into()));
    let mut c = pending.config;
    if let Some(dev) = &dev_schools {
        c.first = dev.0;
        c.second = dev.1;
    }
    commands.insert_resource(RunSetup {
        spawns: descent.spawns,
        start,
        config: c,
    });
    commands.insert_resource(Run::new(c.seed, c.choice, c.first, c.second, &save, core));
    commands.remove_resource::<PendingRun>();
    commands.set_state(AppState::Run);
}

fn on_enter_run(
    mut save: ResMut<SaveData>,
    mut music: ResMut<MusicTrack>,
    mut run: ResMut<Run>,
    start: Option<Res<crate::dev::StartLayer>>,
    give: Option<Res<crate::dev::GiveItems>>,
) {
    if let Some(give) = give {
        for &sc in &give.0 {
            run.learn(sc);
        }
        run.popup = None;
    }
    if let Some(layer) = start.and_then(|l| Layer::ALL.get(l.0.min(4)).copied()) {
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

/// Only the area around the player is simulated (like Noita); the rest of
/// the planet waits, keeping its pending changes.
fn update_sim_region(mut session: ResMut<Session>, player: Res<player::RunPlayer>) {
    if let Some(world) = &mut session.world {
        let p = player.body.pos;
        world.set_active_region(Some(&[(p.x as i32, p.y as i32)]), SIM_RADIUS_CHUNKS);
    }
}

/// Chunks simulated around the player in each direction (64 cells each).
const SIM_RADIUS_CHUNKS: usize = 4;

/// Awakens the pair's fusion once a scroll of each school is known.
fn awaken_fusion(mut run: ResMut<Run>, mut save: ResMut<SaveData>, mut sfx: MessageWriter<Sfx>) {
    let Some(f) = run.fusion_ready() else { return };
    run.fusion = Some(f);
    run.popup = Some((Popup::Fusion(f), 0.0));
    sfx.write(Sfx::ui("fusion_awaken"));
    if save.fusions_discovered.insert(f) {
        save::store(&save);
    }
    grant(&mut save, &mut run, AchievementId::FusionAdept, &mut sfx);
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
                grant(&mut save, &mut run, AchievementId::RiteComplete, &mut sfx);
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
            .add_message::<spells::Harm>()
            .init_resource::<hud::Paused>()
            .init_resource::<hud::PauseMenu>()
            .init_resource::<input::PlayerInput>()
            .add_systems(Startup, smart_dig::configure_gizmos)
            .add_systems(Update, finish_loading.run_if(in_state(AppState::Loading)))
            .add_systems(EguiLoading, hud::loading_ui.run_if(in_state(AppState::Loading)))
            .add_systems(
                OnEnter(AppState::Run),
                (
                    on_enter_run,
                    crate::render::spawn_world_view,
                    lighting::setup_lighting,
                    background::spawn_backgrounds,
                    player::spawn_player,
                    entities::spawn_entities,
                    creatures::spawn_creatures,
                )
                    .chain()
                    .before(RunSetupDone),
            )
            .add_systems(
                OnExit(AppState::Run),
                (
                    crate::render::cleanup_world_view,
                    crate::render::reset_particle_pool,
                    on_exit_run,
                ),
            )
            .add_systems(
                FixedUpdate,
                crate::session::step_world
                    .run_if(in_state(AppState::Run).and_then(resource_exists::<Session>)),
            )
            .add_systems(
                Update,
                (
                    (
                        hud::toggle_pause,
                        update_sim_region,
                        input::read_input.run_if(not(resource_exists::<input::Autoplay>)),
                        input::bot_input.run_if(resource_exists::<input::Autoplay>),
                        player::player_move,
                        player::player_dig,
                        player::player_actions,
                    )
                        .chain(),
                    (
                        spells::cast_spells,
                        awaken_fusion,
                        entities::update_light_charges,
                        entities::prepare_offers,
                        entities::interact,
                        entities::update_projectiles,
                        entities::update_bombs,
                        entities::update_suns,
                        entities::update_light_orbs,
                        entities::update_shards,
                    )
                        .chain(),
                    (
                        tracking::process_sim_events,
                        hazards::environment,
                        creatures::update_creatures,
                        creatures::update_enemy_bolts,
                        creatures::stalker_director,
                        track_depth,
                        update_phase,
                        apply_time_scale,
                    )
                        .chain(),
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
                    player::draw_dig_beam,
                    smart_dig::highlight_targets,
                    crate::render::upload_dirty_chunks,
                    crate::render::draw_sim_particles,
                    background::update_backgrounds,
                    lighting::update_lighting,
                    juice::ambient_effects,
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
