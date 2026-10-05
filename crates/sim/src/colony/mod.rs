//! The colony: every rule of the terraforming game that the host runs and
//! replicates. Plain data and functions with no engine types, so it runs
//! headless in tests and identically for a host and for singleplayer.
//!
//! The host calls [`Colony::step`] every tick and [`Colony::apply`] for each
//! player action. Clients hold a replica that they only ever update from the
//! host's messages.

pub mod actions;
pub mod body;
pub mod creatures;
pub mod domes;
pub mod geom;
pub mod inventory;
pub mod items;
pub mod mods;
pub mod plants;
pub mod save;

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::material::{Kind, Material};
use crate::planetgen::{Band, Feature, FeatureKind, Planet, Profile};
use crate::rng::Rng;
use crate::world::World;
use body::Body;
use creatures::Creature;
use domes::Dome;
use geom::{Rect, V2, v2};
use inventory::Inventory;
use items::{DomeKind, Item, MachineKind};
use mods::ModStats;
use plants::{Species, SpeciesId};

pub type Id = u32;
/// A persistent player identity (the Steam id, or a random id kept in the
/// client's settings), so characters survive leaving and rejoining.
pub type PlayerKey = u64;

/// Seconds in a full day, and how much of it is daylight.
pub const DAY_SECS: f32 = 720.0;
pub const DAYLIGHT_SECS: f32 = 480.0;
/// Atmospheric oxygen (percent) at the start, and where suits come off.
pub const O2_START: f32 = 3.0;
pub const O2_BREATHABLE: f32 = 16.0;
pub const PLAYER_HALF_W: f32 = 4.0;
pub const PLAYER_HEIGHT: f32 = 22.0;
pub const HOTBAR: usize = 10;
pub const INVENTORY: usize = 50;
pub const BOLT_COST: f32 = 12.0;
pub const BOLT_SPEED: f32 = 330.0;
const LOCKOUT_SECS: f32 = 2.5;
const RESPAWN_SECS: f32 = 5.0;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Clock {
    pub day: u32,
    /// Seconds since dawn.
    pub time: f32,
}

impl Clock {
    pub fn is_night(&self) -> bool {
        self.time >= DAYLIGHT_SECS
    }

    /// Sunlight 0–1, with soft dawn and dusk.
    pub fn daylight(&self) -> f32 {
        let edge = 40.0;
        let t = self.time;
        if t < edge {
            0.25 + 0.75 * t / edge
        } else if t < DAYLIGHT_SECS - edge {
            1.0
        } else if t < DAYLIGHT_SECS {
            0.25 + 0.75 * (DAYLIGHT_SECS - t) / edge
        } else {
            // Moonlight, darkest at midnight.
            let n = (t - DAYLIGHT_SECS) / (DAY_SECS - DAYLIGHT_SECS);
            0.12 + 0.13 * (2.0 * n - 1.0).abs()
        }
    }

    /// Hour of a 24-hour clock where dawn is 06:00.
    pub fn hour(&self) -> f32 {
        (6.0 + self.time / DAY_SECS * 24.0) % 24.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeatherKind {
    Clear,
    Mist,
    Rain,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Weather {
    pub kind: WeatherKind,
    /// Seconds until it changes.
    pub remaining: f32,
    pub next: WeatherKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Atmosphere {
    /// Percent oxygen.
    pub o2: f32,
    /// Change in percentage points per day at the current output.
    pub rate: f32,
}

/// What the client sends about its own character each tick; the host relays
/// it to everyone. Movement is the one thing clients own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Pose {
    pub pos: V2,
    pub vel: V2,
    /// -1 or 1.
    pub facing: i8,
    /// Animation row (see the client's `player_anim`).
    pub anim: u8,
    /// Aim angle in radians (0 = right, positive = down).
    pub aim: f32,
    /// Bit flags, see [`pose_flag`].
    pub flags: u8,
}

pub mod pose_flag {
    pub const ON_GROUND: u8 = 1;
    pub const THRUST: u8 = 2;
    pub const SWIMMING: u8 = 4;
    pub const TOOL: u8 = 8;
    pub const DASH: u8 = 16;
    pub const GLIDE: u8 = 32;
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Player {
    pub key: PlayerKey,
    pub name: String,
    /// Index into the team colour palette.
    pub color: u8,
    #[serde(skip)]
    pub online: bool,
    pub pose: Pose,
    pub hp: f32,
    pub o2: f32,
    pub battery: f32,
    /// Seconds until the battery starts recharging again.
    pub battery_wait: f32,
    /// The battery was drained and is locked out.
    pub lockout: bool,
    pub warmth: f32,
    /// Seconds of web slow left.
    pub slowed: f32,
    /// Counts down to respawn while blacked out.
    pub dead: Option<f32>,
    pub suit: bool,
    /// The dome the player is inside, if any.
    pub in_dome: Option<Id>,
    /// Hotbar is slots `0..HOTBAR`.
    pub inv: Inventory,
    pub trash: Option<inventory::Stack>,
    pub selected: u8,
    /// Litres in the watering can.
    pub can_water: f32,
    /// Highest tier owned of each mod set (set id -> 1..=4).
    pub mods: BTreeMap<u8, u8>,
    /// The set equipped in each of the eight body slots.
    pub equipped: [Option<u8>; 8],
    /// The dome whose bed the player wakes in.
    pub spawn: Option<Id>,
    /// Bumped whenever the host moves the player (respawn, recall); the
    /// client then jumps to `pose.pos`.
    pub warp: u32,
    /// Cells dug per material that haven't added up to an item yet.
    #[serde(skip)]
    pub dig_tally: BTreeMap<u8, f32>,
    #[serde(skip)]
    pub dig_wait: f32,
    /// Bumped on any change the owner must see (inventory, mods).
    #[serde(skip)]
    pub rev: u32,
    /// Bumped on any change other players must see (name, mods shown, suit).
    #[serde(skip)]
    pub public_rev: u32,
}

impl Player {
    pub fn new(key: PlayerKey, name: String, color: u8, at: V2) -> Player {
        let stats = ModStats::default();
        Player {
            key,
            name,
            color,
            online: false,
            pose: Pose {
                pos: at,
                facing: 1,
                ..Pose::default()
            },
            hp: stats.max_hp,
            o2: stats.o2_capacity,
            battery: stats.battery_capacity,
            battery_wait: 0.0,
            lockout: false,
            warmth: 100.0,
            slowed: 0.0,
            dead: None,
            suit: false,
            in_dome: None,
            inv: Inventory::new(INVENTORY),
            trash: None,
            selected: 0,
            can_water: 0.0,
            mods: BTreeMap::new(),
            equipped: [None; 8],
            spawn: None,
            warp: 0,
            dig_tally: BTreeMap::new(),
            dig_wait: 0.0,
            rev: 1,
            public_rev: 1,
        }
    }

    /// The player's stats with their equipped mods applied.
    pub fn stats(&self) -> ModStats {
        ModStats::default()
    }

    pub fn body(&self) -> Body {
        Body::new(self.pose.pos, PLAYER_HALF_W, PLAYER_HEIGHT)
    }

    pub fn center(&self) -> V2 {
        v2(self.pose.pos.x, self.pose.pos.y - PLAYER_HEIGHT / 2.0)
    }

    pub fn alive(&self) -> bool {
        self.dead.is_none()
    }

    /// The stack in the selected hotbar slot. Selecting slot `HOTBAR` puts
    /// everything away (the bare multitool).
    pub fn held(&self) -> Option<inventory::Stack> {
        if (self.selected as usize) < HOTBAR {
            self.inv.slots[self.selected as usize]
        } else {
            None
        }
    }

    pub fn touch(&mut self) {
        self.rev = self.rev.wrapping_add(1);
    }

    pub fn touch_public(&mut self) {
        self.rev = self.rev.wrapping_add(1);
        self.public_rev = self.public_rev.wrapping_add(1);
    }

    /// What other players need to draw this one.
    pub fn public(&self) -> PublicPlayer {
        PublicPlayer {
            key: self.key,
            name: self.name.clone(),
            color: self.color,
            suit: self.suit,
            dead: self.dead.is_some(),
            mods: self
                .equipped
                .map(|set| set.map(|s| (s, self.mods.get(&s).copied().unwrap_or(1)))),
            held: self.held().map(|s| s.item),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PublicPlayer {
    pub key: PlayerKey,
    pub name: String,
    pub color: u8,
    pub suit: bool,
    pub dead: bool,
    /// Per body slot: the equipped set and its tier.
    pub mods: [Option<(u8, u8)>; 8],
    pub held: Option<Item>,
}

/// Fast-changing numbers, sent to their owner several times a second.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Vitals {
    pub hp: f32,
    pub o2: f32,
    pub battery: f32,
    pub lockout: bool,
    pub warmth: f32,
    pub slowed: f32,
    pub dead: Option<f32>,
    pub suit: bool,
    pub in_dome: Option<Id>,
    pub can_water: f32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Machine {
    pub kind: MachineKind,
    pub name: String,
    /// Chest contents, or a pylon's battery flowers.
    pub inv: Inventory,
    /// Charge stored (pylon) or litres held (tank).
    pub store: f32,
    pub on: bool,
    pub progress: f32,
    pub issues: u8,
    pub boost: f32,
}

/// A loose container in the world.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CrateKind {
    /// What a player dropped when they blacked out.
    Pack,
    /// Salvage in a ruin vault.
    Cache,
    /// A delivery from Earth.
    Pod,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Crate {
    pub kind: CrateKind,
    pub inv: Inventory,
    pub owner: Option<PlayerKey>,
}

/// A plant growing outside a planter (wild, or planted outdoors).
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Plant {
    pub species: SpeciesId,
    pub growth: f32,
    pub wild: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Drop {
    pub item: Item,
    pub count: u32,
    pub vel: V2,
    pub age: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Bolt {
    pub vel: V2,
    pub damage: f32,
    /// 1 = normal.
    pub size: f32,
    pub owner: Option<PlayerKey>,
    pub life: f32,
    /// A webspinner's web rather than a laser bolt.
    pub web: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum EntKind {
    Dome(Dome),
    Machine(Machine),
    Crate(Crate),
    Plant(Plant),
    Creature(Creature),
    Drop(Drop),
    Bolt(Bolt),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Ent {
    pub id: Id,
    /// Domes: middle of the floor. Others: middle of the base.
    pub pos: V2,
    pub kind: EntKind,
}

impl Ent {
    /// Entities whose position changes every tick and is sent separately.
    pub fn moves(&self) -> bool {
        matches!(
            self.kind,
            EntKind::Creature(_) | EntKind::Drop(_) | EntKind::Bolt(_)
        )
    }

    pub fn velocity(&self) -> V2 {
        match &self.kind {
            EntKind::Creature(c) => c.vel,
            EntKind::Drop(d) => d.vel,
            EntKind::Bolt(b) => b.vel,
            _ => V2::ZERO,
        }
    }
}

/// Position update for a moving entity.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Motion {
    pub id: Id,
    pub pos: V2,
    pub vel: V2,
    /// Entity-specific animation state (a creature's state and facing).
    pub anim: u8,
}

/// One-shot effects for sound and particles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Fx {
    /// A cell of this material was dug.
    Dig(Material),
    BoltFire,
    BoltHit,
    Pickup,
    Hurt,
    Blackout,
    Respawn,
    CreatureHit,
    CreatureDie(creatures::CreatureKind),
    CreatureCall(creatures::CreatureKind),
    WebHit,
    Place,
    DomePlaced,
    Dawn,
    Dusk,
    Thunder,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Event {
    Fx {
        fx: Fx,
        pos: V2,
    },
    /// A message for one player, or for everyone.
    Toast {
        to: Option<PlayerKey>,
        text: String,
        good: bool,
    },
}

/// Small, frequently sent colony-wide state.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Globals {
    pub clock: Clock,
    pub weather: Weather,
    pub atmosphere: Atmosphere,
    pub credits: i64,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Colony {
    pub seed: u64,
    pub profile: Profile,
    pub features: Vec<Feature>,
    pub clock: Clock,
    pub weather: Weather,
    pub atmosphere: Atmosphere,
    pub credits: i64,
    pub species: Vec<Species>,
    pub next_id: Id,
    pub ents: BTreeMap<Id, Ent>,
    pub players: BTreeMap<PlayerKey, Player>,
    /// The starter dome.
    pub home: Id,
    /// Lairs whose Brood Mother has already appeared.
    pub cleared_lairs: Vec<(i32, i32)>,
    pub rng: Rng,
    /// Accumulators for work done less often than every tick.
    #[serde(skip)]
    timers: Timers,
    #[serde(skip)]
    dirty: BTreeSet<Id>,
    #[serde(skip)]
    removed: Vec<Id>,
    #[serde(skip)]
    events: Vec<Event>,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct Timers {
    second: f32,
    spawner: f32,
}

impl Colony {
    /// Founds a colony on a freshly generated planet: builds the starter
    /// dome on the plateau and stocks its chests.
    pub fn found(planet: &mut Planet) -> Colony {
        let seed = planet.world.seed();
        let profile = planet.profile.clone();
        let mut colony = Colony {
            seed,
            features: planet.features.clone(),
            clock: Clock { day: 1, time: 20.0 },
            weather: Weather {
                kind: WeatherKind::Mist,
                remaining: 90.0,
                next: WeatherKind::Clear,
            },
            atmosphere: Atmosphere {
                o2: O2_START,
                rate: 0.0,
            },
            credits: 0,
            species: plants::base_species(),
            next_id: 1,
            ents: BTreeMap::new(),
            players: BTreeMap::new(),
            home: 0,
            cleared_lairs: Vec::new(),
            rng: Rng::new(seed ^ 0xC0107),
            timers: Timers::default(),
            dirty: BTreeSet::new(),
            removed: Vec::new(),
            events: Vec::new(),
            profile,
        };
        let pos = v2(colony.profile.spawn_x as f32, colony.profile.spawn_y() as f32);
        domes::build_cells(&mut planet.world, DomeKind::Starter, pos);
        let mut dome = Dome::new(DomeKind::Starter, "Starter Biodome".into());
        dome.powered = true;
        dome.chests[0].add(Item::Seed(plants::VOLTBLOOM), 4, 1.0);
        dome.chests[0].add(Item::Seed(2), 4, 1.0);
        dome.chests[0].add(Item::Seed(1), 2, 1.0);
        dome.chests[0].add(Item::WateringCan, 1, 1.0);
        dome.chests[1].add(Item::Meal, 8, 1.0);
        dome.chests[1].add(Item::Machine(MachineKind::Lamp), 4, 1.0);
        colony.home = colony.spawn(pos, EntKind::Dome(dome));
        colony.place_caches();
        colony
    }

    /// Salvage waiting in the ruin vaults.
    fn place_caches(&mut self) {
        for f in self.features.clone() {
            if f.kind != FeatureKind::Vault {
                continue;
            }
            let mut inv = Inventory::new(12);
            let deep = self.profile.band(f.x, f.y) != Band::Shallows;
            inv.add(Item::Iron, self.rng.range(15, 40) as u32, 1.0);
            inv.add(Item::Silicon, self.rng.range(8, 20) as u32, 1.0);
            inv.add(Item::Copper, self.rng.range(8, 24) as u32, 1.0);
            if deep {
                inv.add(Item::Gold, self.rng.range(6, 16) as u32, 1.0);
                inv.add(Item::Titanium, self.rng.range(3, 9) as u32, 1.0);
            }
            let seed = self.rng.range(0, plants::BASE_SPECIES as i32) as SpeciesId;
            inv.add(Item::Seed(seed), 2, 1.0);
            self.spawn(
                v2(f.x as f32, f.y as f32),
                EntKind::Crate(Crate {
                    kind: CrateKind::Cache,
                    inv,
                    owner: None,
                }),
            );
        }
    }

    // ---- entities ----------------------------------------------------------

    pub fn spawn(&mut self, pos: V2, kind: EntKind) -> Id {
        let id = self.next_id;
        self.next_id += 1;
        self.ents.insert(id, Ent { id, pos, kind });
        self.dirty.insert(id);
        id
    }

    pub fn despawn(&mut self, id: Id) {
        if self.ents.remove(&id).is_some() {
            self.dirty.remove(&id);
            self.removed.push(id);
        }
    }

    /// Marks an entity as changed so it is sent to clients.
    pub fn touch(&mut self, id: Id) {
        self.dirty.insert(id);
    }

    pub fn dome(&self, id: Id) -> Option<(&Ent, &Dome)> {
        let e = self.ents.get(&id)?;
        match &e.kind {
            EntKind::Dome(d) => Some((e, d)),
            _ => None,
        }
    }

    pub fn dome_mut(&mut self, id: Id) -> Option<&mut Dome> {
        self.dirty.insert(id);
        match &mut self.ents.get_mut(&id)?.kind {
            EntKind::Dome(d) => Some(d),
            _ => None,
        }
    }

    pub fn domes(&self) -> impl Iterator<Item = (&Ent, &Dome)> {
        self.ents.values().filter_map(|e| match &e.kind {
            EntKind::Dome(d) => Some((e, d)),
            _ => None,
        })
    }

    pub fn dome_rects(&self) -> Vec<Rect> {
        self.domes()
            .map(|(e, d)| domes::footprint(d.kind, e.pos))
            .collect()
    }

    /// The dome containing a point, if any.
    pub fn dome_at(&self, p: V2) -> Option<Id> {
        self.domes()
            .find(|(e, d)| domes::contains(d.kind, e.pos, p))
            .map(|(e, _)| e.id)
    }

    pub fn item_name(&self, item: Item) -> String {
        item.name(&self.species)
    }

    // ---- players -----------------------------------------------------------

    /// Where new and respawning players appear: inside their spawn dome.
    pub fn spawn_point(&self, key: Option<PlayerKey>) -> V2 {
        let dome = key
            .and_then(|k| self.players.get(&k))
            .and_then(|p| p.spawn)
            .filter(|id| self.dome(*id).is_some())
            .unwrap_or(self.home);
        match self.ents.get(&dome) {
            Some(e) => v2(e.pos.x - 20.0, e.pos.y),
            None => v2(self.profile.spawn_x as f32, self.profile.spawn_y() as f32),
        }
    }

    /// A player connects. Returning players get their character back.
    pub fn join(&mut self, key: PlayerKey, name: String) -> &Player {
        if !self.players.contains_key(&key) {
            // Give each new player the least used colour.
            let color = (0..8u8)
                .min_by_key(|c| self.players.values().filter(|p| p.color == *c).count())
                .unwrap_or(0);
            // Stand side by side rather than inside each other.
            let mut at = self.spawn_point(None);
            at.x += (color as f32 - 3.5) * 12.0;
            let mut p = Player::new(key, name.clone(), color, at);
            p.spawn = Some(self.home);
            self.players.insert(key, p);
        }
        let p = self.players.get_mut(&key).unwrap();
        p.name = name;
        p.online = true;
        p.touch_public();
        p
    }

    pub fn leave(&mut self, key: PlayerKey) {
        if let Some(p) = self.players.get_mut(&key) {
            p.online = false;
            p.touch_public();
        }
    }

    pub fn online(&self) -> impl Iterator<Item = &Player> {
        self.players.values().filter(|p| p.online)
    }

    /// Updates a player's pose from their client.
    pub fn set_pose(&mut self, key: PlayerKey, pose: Pose) {
        if let Some(p) = self.players.get_mut(&key)
            && p.alive()
        {
            p.pose = pose;
        }
    }

    pub fn vitals(&self, key: PlayerKey) -> Option<Vitals> {
        let p = self.players.get(&key)?;
        Some(Vitals {
            hp: p.hp,
            o2: p.o2,
            battery: p.battery,
            lockout: p.lockout,
            warmth: p.warmth,
            slowed: p.slowed,
            dead: p.dead,
            suit: p.suit,
            in_dome: p.in_dome,
            can_water: p.can_water,
        })
    }

    pub fn apply_vitals(&mut self, key: PlayerKey, v: Vitals) {
        if let Some(p) = self.players.get_mut(&key) {
            p.hp = v.hp;
            p.o2 = v.o2;
            p.battery = v.battery;
            p.lockout = v.lockout;
            p.warmth = v.warmth;
            p.slowed = v.slowed;
            p.dead = v.dead;
            p.suit = v.suit;
            p.in_dome = v.in_dome;
            p.can_water = v.can_water;
        }
    }

    /// Damages a player (after their mods' reduction).
    pub fn hurt(&mut self, key: PlayerKey, amount: f32) {
        let Some(p) = self.players.get_mut(&key) else {
            return;
        };
        if !p.alive() || amount <= 0.0 {
            return;
        }
        p.hp -= amount * p.stats().damage_taken;
        let pos = p.center();
        if amount >= 4.0 {
            self.events.push(Event::Fx { fx: Fx::Hurt, pos });
        }
    }

    pub fn toast(&mut self, to: Option<PlayerKey>, text: impl Into<String>, good: bool) {
        self.events.push(Event::Toast {
            to,
            text: text.into(),
            good,
        });
    }

    pub fn fx(&mut self, fx: Fx, pos: V2) {
        self.events.push(Event::Fx { fx, pos });
    }

    // ---- replication -------------------------------------------------------

    pub fn globals(&self) -> Globals {
        Globals {
            clock: self.clock,
            weather: self.weather,
            atmosphere: self.atmosphere,
            credits: self.credits,
        }
    }

    pub fn apply_globals(&mut self, g: Globals) {
        self.clock = g.clock;
        self.weather = g.weather;
        self.atmosphere = g.atmosphere;
        self.credits = g.credits;
    }

    /// Changed entities (as snapshots) and removed ids since the last call.
    pub fn take_changes(&mut self) -> (Vec<Ent>, Vec<Id>) {
        let upserts = std::mem::take(&mut self.dirty)
            .into_iter()
            .filter_map(|id| self.ents.get(&id).cloned())
            .collect();
        (upserts, std::mem::take(&mut self.removed))
    }

    pub fn apply_changes(&mut self, upserts: Vec<Ent>, removed: Vec<Id>) {
        for e in upserts {
            self.next_id = self.next_id.max(e.id + 1);
            self.ents.insert(e.id, e);
        }
        for id in removed {
            self.ents.remove(&id);
        }
    }

    /// Motion of moving entities within `radius` of a point.
    pub fn motion_near(&self, at: V2, radius: f32) -> Vec<Motion> {
        self.ents
            .values()
            .filter(|e| e.moves() && e.pos.distance(at) < radius)
            .map(|e| Motion {
                id: e.id,
                pos: e.pos,
                vel: e.velocity(),
                anim: match &e.kind {
                    EntKind::Creature(c) => c.anim(),
                    _ => 0,
                },
            })
            .collect()
    }

    pub fn apply_motion(&mut self, motion: &[Motion]) {
        for m in motion {
            if let Some(e) = self.ents.get_mut(&m.id) {
                e.pos = m.pos;
                match &mut e.kind {
                    EntKind::Creature(c) => {
                        c.vel = m.vel;
                        c.set_anim(m.anim);
                    }
                    EntKind::Drop(d) => d.vel = m.vel,
                    EntKind::Bolt(b) => b.vel = m.vel,
                    _ => {}
                }
            }
        }
    }

    pub fn take_events(&mut self) -> Vec<Event> {
        std::mem::take(&mut self.events)
    }

    // ---- simulation --------------------------------------------------------

    /// Advances the colony by `dt` seconds. Host only.
    pub fn step(&mut self, world: &mut World, dt: f32) {
        self.step_clock(dt);
        self.step_players(world, dt);
        self.step_bolts(world, dt);
        self.step_drops(world, dt);
        creatures::step(self, world, dt);

        self.timers.second += dt;
        if self.timers.second >= 1.0 {
            self.timers.second -= 1.0;
            self.step_second(world);
        }
        self.timers.spawner += dt;
        if self.timers.spawner >= 2.0 {
            self.timers.spawner = 0.0;
            creatures::spawn_tick(self, world);
        }
    }

    fn step_clock(&mut self, dt: f32) {
        let was_night = self.clock.is_night();
        self.clock.time += dt;
        if self.clock.time >= DAY_SECS {
            self.clock.time -= DAY_SECS;
            self.clock.day += 1;
            self.fx(Fx::Dawn, V2::ZERO);
            self.toast(None, format!("Day {}", self.clock.day), true);
        }
        if !was_night && self.clock.is_night() {
            self.fx(Fx::Dusk, V2::ZERO);
        }

        self.weather.remaining -= dt;
        if self.weather.remaining <= 0.0 {
            self.weather.kind = self.weather.next;
            self.weather.remaining = 120.0 + self.rng.next_f32() * 240.0;
            self.weather.next = match self.rng.next_u8() % 10 {
                0..=4 => WeatherKind::Clear,
                5..=6 => WeatherKind::Mist,
                _ => WeatherKind::Rain,
            };
        }
        if self.weather.kind == WeatherKind::Rain && self.rng.next_f32() < dt / 25.0 {
            self.fx(Fx::Thunder, V2::ZERO);
        }
    }

    /// Whether a point is under open sky (for rain, cold and daylight).
    pub fn outdoors(&self, world: &World, p: V2) -> bool {
        let (x, y) = p.cell();
        y < self.profile.surface_at(x) + 6
            && (1..40).all(|d| !world.material(x, y - d * 3).is_solid_for_player())
    }

    fn step_players(&mut self, world: &World, dt: f32) {
        let breathable_air = self.atmosphere.o2 >= O2_BREATHABLE;
        // The air thickens as the planet greens, so suits last longer.
        let thin = 1.0 - 0.8 * ((self.atmosphere.o2 - O2_START) / (O2_BREATHABLE - O2_START)).clamp(0.0, 1.0);
        let night = self.clock.is_night();
        let keys: Vec<PlayerKey> = self.online().map(|p| p.key).collect();
        for key in keys {
            let (center, head) = {
                let p = &self.players[&key];
                (p.center(), p.body().head())
            };
            let in_dome = self.dome_at(center);
            let outdoors = in_dome.is_none() && self.outdoors(world, center);
            let frost = Body::new(
                self.players[&key].pose.pos,
                PLAYER_HALF_W + 3.0,
                PLAYER_HEIGHT + 3.0,
            )
            .touching(world, Material::Ice)
                > 0;
            let (hx, hy) = head.cell();
            let head_in = world.material(hx, hy);
            let spores = self.players[&key].body().touching(world, Material::Spore);
            let hot = Body::new(
                self.players[&key].pose.pos,
                PLAYER_HALF_W + 1.0,
                PLAYER_HEIGHT + 1.0,
            )
            .touching(world, Material::Lava)
                + self.players[&key].body().touching(world, Material::Fire);

            let p = self.players.get_mut(&key).unwrap();
            let stats = p.stats();
            if let Some(t) = &mut p.dead {
                *t -= dt;
                if *t <= 0.0 {
                    self.respawn(key);
                }
                continue;
            }
            if p.in_dome != in_dome {
                p.in_dome = in_dome;
            }
            let suit = in_dome.is_none() && !breathable_air;
            if p.suit != suit {
                p.suit = suit;
                p.touch_public();
            }

            // Oxygen.
            let underwater = head_in.kind() == Kind::Liquid;
            if in_dome.is_some() || (breathable_air && !underwater) {
                p.o2 = (p.o2 + 25.0 * stats.o2_refill * dt).min(stats.o2_capacity);
            } else {
                let rate = 100.0 / 180.0 * thin * stats.o2_drain * if underwater { 1.6 } else { 1.0 };
                p.o2 = (p.o2 - rate * dt).max(0.0);
            }
            let mut damage = 0.0;
            if p.o2 <= 0.0 {
                damage += 9.0 * dt;
            }

            // Cold: nights under open sky, and frost caves.
            let cold = (outdoors && night) || frost;
            if cold {
                p.warmth = (p.warmth - 100.0 / 150.0 * (1.0 - stats.cold_resist) * dt).max(0.0);
            } else {
                p.warmth = (p.warmth + 100.0 / 20.0 * dt).min(100.0);
            }
            if p.warmth <= 0.0 {
                damage += 3.0 * dt;
            }

            // Hazards in the cells around the player.
            if spores > 0 {
                damage += 7.0 * (1.0 - stats.toxin_resist) * dt;
            }
            if hot > 0 {
                damage += 30.0 * dt;
            }

            // Health.
            let regen = stats.regen + if in_dome.is_some() { 4.0 } else { 0.0 };
            p.hp = (p.hp + regen * dt).min(stats.max_hp);
            p.slowed = (p.slowed - dt).max(0.0);

            // Laser battery: recharges after a pause, longer if drained.
            p.battery_wait = (p.battery_wait - dt).max(0.0);
            if p.battery_wait <= 0.0 {
                p.lockout = false;
                p.battery = (p.battery + 30.0 * stats.battery_regen * dt).min(stats.battery_capacity);
            }
            p.dig_wait = (p.dig_wait - dt).max(0.0);

            if damage > 0.0 {
                p.hp -= damage * stats.damage_taken;
            }
            if p.hp <= 0.0 {
                self.black_out(key);
            }
        }
    }

    /// A player's health ran out: they drop their raw resources in a pack
    /// and wake in their bed a few seconds later.
    fn black_out(&mut self, key: PlayerKey) {
        let Some(p) = self.players.get_mut(&key) else {
            return;
        };
        p.hp = 0.0;
        p.dead = Some(RESPAWN_SECS);
        p.touch_public();
        let pos = p.pose.pos;
        let mut pack = Inventory::new(INVENTORY);
        for slot in p.inv.slots.iter_mut().skip(HOTBAR) {
            if let Some(s) = slot
                && !s.fav
                && s.item.category() == items::Category::Resource
            {
                pack.add(s.item, s.count, 10.0);
                *slot = None;
            }
        }
        let name = p.name.clone();
        self.fx(Fx::Blackout, pos);
        self.toast(None, format!("{name} blacked out"), false);
        if !pack.is_empty() {
            self.spawn(
                pos,
                EntKind::Crate(Crate {
                    kind: CrateKind::Pack,
                    inv: pack,
                    owner: Some(key),
                }),
            );
            self.toast(Some(key), "Your resources are in a pack where you fell", false);
        }
    }

    fn respawn(&mut self, key: PlayerKey) {
        let at = self.spawn_point(Some(key));
        let Some(p) = self.players.get_mut(&key) else {
            return;
        };
        let stats = p.stats();
        p.dead = None;
        p.hp = stats.max_hp * 0.6;
        p.o2 = stats.o2_capacity;
        p.warmth = 100.0;
        p.pose.pos = at;
        p.pose.vel = V2::ZERO;
        p.warp += 1;
        p.touch_public();
        self.fx(Fx::Respawn, at);
    }

    fn step_bolts(&mut self, world: &mut World, dt: f32) {
        let ids: Vec<Id> = self
            .ents
            .values()
            .filter(|e| matches!(e.kind, EntKind::Bolt(_)))
            .map(|e| e.id)
            .collect();
        for id in ids {
            let Some(Ent {
                pos,
                kind: EntKind::Bolt(bolt),
                ..
            }) = self.ents.get(&id).cloned()
            else {
                continue;
            };
            let mut bolt = bolt;
            let mut pos = pos;
            bolt.life -= dt;
            let travel = bolt.vel.length() * dt;
            let steps = (travel / 2.0).ceil().max(1.0) as i32;
            let step = bolt.vel * (dt / steps as f32);
            let mut hit = bolt.life <= 0.0;
            for _ in 0..steps {
                pos += step;
                let (x, y) = pos.cell();
                if world.material(x, y).is_solid_for_creature() {
                    hit = true;
                    if !bolt.web {
                        // Laser bolts chip the terrain; the owner gets the bits.
                        let dug = world.dig(x, y, (2.0 * bolt.size) as i32, 70);
                        if let Some(owner) = bolt.owner {
                            self.collect_dug(owner, &dug, pos);
                        }
                    }
                    break;
                }
                if bolt.web {
                    // Webs stick to players.
                    let target = self
                        .online()
                        .find(|p| p.alive() && p.center().distance(pos) < 9.0)
                        .map(|p| p.key);
                    if let Some(key) = target {
                        self.hurt(key, bolt.damage);
                        if let Some(p) = self.players.get_mut(&key) {
                            p.slowed = 3.0;
                        }
                        self.fx(Fx::WebHit, pos);
                        hit = true;
                        break;
                    }
                } else if creatures::damage_at(self, pos, 5.0 * bolt.size, bolt.damage, bolt.owner) {
                    hit = true;
                    break;
                }
            }
            if hit {
                if !bolt.web {
                    self.fx(Fx::BoltHit, pos);
                }
                self.despawn(id);
            } else if let Some(e) = self.ents.get_mut(&id) {
                e.pos = pos;
                e.kind = EntKind::Bolt(bolt);
            }
        }
    }

    /// Turns dug cells into item drops for a player, a fraction at a time.
    pub fn collect_dug(&mut self, key: PlayerKey, dug: &[(i32, i32, Material)], at: V2) {
        let Some(p) = self.players.get_mut(&key) else {
            return;
        };
        let ore_yield = p.stats().ore_yield;
        let mut ready: Vec<(Item, u32)> = Vec::new();
        for &(_, _, m) in dug {
            let Some((item, per)) = items::drop_for(m) else {
                continue;
            };
            let gain = if m.is_ore() { ore_yield } else { 1.0 } / per as f32;
            let t = p.dig_tally.entry(m as u8).or_insert(0.0);
            *t += gain;
            if *t >= 1.0 {
                let n = t.floor();
                *t -= n;
                match ready.iter_mut().find(|(i, _)| *i == item) {
                    Some((_, c)) => *c += n as u32,
                    None => ready.push((item, n as u32)),
                }
            }
        }
        if let Some(&(_, _, m)) = dug.first() {
            self.fx(Fx::Dig(m), at);
        }
        for (item, count) in ready {
            self.drop_item(at, item, count);
        }
    }

    /// Drops items into the world, merging with a nearby drop of the same item.
    pub fn drop_item(&mut self, at: V2, item: Item, count: u32) {
        if count == 0 {
            return;
        }
        let near = self.ents.values_mut().find_map(|e| match &mut e.kind {
            EntKind::Drop(d) if d.item == item && e.pos.distance(at) < 10.0 && d.count < 500 => {
                Some((e.id, d))
            }
            _ => None,
        });
        if let Some((id, d)) = near {
            d.count += count;
            d.age = 0.0;
            self.dirty.insert(id);
            return;
        }
        let vel = v2(
            (self.rng.next_f32() - 0.5) * 50.0,
            -30.0 - self.rng.next_f32() * 30.0,
        );
        self.spawn(
            at,
            EntKind::Drop(Drop {
                item,
                count,
                vel,
                age: 0.0,
            }),
        );
    }

    fn step_drops(&mut self, world: &World, dt: f32) {
        let targets: Vec<(PlayerKey, V2, f32, f32)> = self
            .online()
            .filter(|p| p.alive())
            .map(|p| {
                let s = p.stats();
                (p.key, p.center(), s.magnet, s.stack_mult)
            })
            .collect();
        let ids: Vec<Id> = self
            .ents
            .values()
            .filter(|e| matches!(e.kind, EntKind::Drop(_)))
            .map(|e| e.id)
            .collect();
        for id in ids {
            let Some(e) = self.ents.get_mut(&id) else { continue };
            let EntKind::Drop(d) = &mut e.kind else { continue };
            d.age += dt;
            if d.age > 600.0 {
                self.despawn(id);
                continue;
            }
            // Fly to the nearest player in range (after a moment, so thrown
            // items don't snap straight back).
            let pull = targets
                .iter()
                .filter(|(_, c, magnet, _)| d.age > 0.4 && c.distance(e.pos) < *magnet)
                .min_by(|a, b| a.1.distance(e.pos).total_cmp(&b.1.distance(e.pos)));
            if let Some(&(key, center, _, mult)) = pull {
                let to = center - e.pos;
                if to.length() < 7.0 {
                    let (item, count) = (d.item, d.count);
                    let p = self.players.get_mut(&key).unwrap();
                    let left = p.inv.add(item, count, mult);
                    if left < count {
                        p.touch();
                        let pos = e.pos;
                        if left == 0 {
                            self.despawn(id);
                        } else if let Some(Ent {
                            kind: EntKind::Drop(d),
                            ..
                        }) = self.ents.get_mut(&id)
                        {
                            d.count = left;
                            self.dirty.insert(id);
                        }
                        self.fx(Fx::Pickup, pos);
                        continue;
                    }
                    // Inventory full: stop chasing.
                    d.vel = V2::ZERO;
                } else {
                    d.vel = d.vel.lerp(to.normalized() * 190.0, (dt * 10.0).min(1.0));
                    e.pos += d.vel * dt;
                    continue;
                }
            }
            // Otherwise fall and settle.
            d.vel.y = (d.vel.y + 300.0 * dt).min(240.0);
            d.vel.x *= 1.0 - (2.0 * dt).min(1.0);
            let mut body = Body::new(e.pos, 2.0, 4.0);
            body.vel = d.vel;
            if !body.escape_overlap(world, body::player_solid) {
                body.pos.y -= 4.0;
            }
            body.step(world, dt, 0, body::player_solid);
            if body.on_ground {
                body.vel = V2::ZERO;
            }
            e.pos = body.pos;
            d.vel = body.vel;
        }
    }

    /// Work done once a second.
    fn step_second(&mut self, world: &mut World) {
        // The greener the air, the faster grass creeps over bare dirt.
        let t = ((self.atmosphere.o2 - O2_START) / (O2_BREATHABLE - O2_START)).clamp(0.0, 1.0);
        world.set_fertility((6.0 + t * 200.0) as u8);
    }
}

#[cfg(test)]
pub(crate) mod testing {
    use super::*;
    use crate::planetgen;

    /// A small planet with a founded colony and one online player.
    pub fn colony() -> (World, Colony, PlayerKey) {
        let mut planet = planetgen::generate_sized(2560, 1024, 9);
        let mut colony = Colony::found(&mut planet);
        colony.join(1, "Ada".into());
        (planet.world, colony, 1)
    }

    pub fn run(colony: &mut Colony, world: &mut World, secs: f32) {
        let ticks = (secs * 60.0) as usize;
        for _ in 0..ticks {
            colony.step(world, 1.0 / 60.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::testing::*;
    use super::*;

    #[test]
    fn founding_builds_a_stocked_starter_dome() {
        let (world, colony, key) = colony();
        let (e, d) = colony.dome(colony.home).unwrap();
        assert_eq!(d.kind, DomeKind::Starter);
        assert!(d.chests[0].count(Item::Seed(plants::VOLTBLOOM)) > 0);
        let (x, y) = e.pos.cell();
        assert_eq!(world.material(x, y), Material::Plating);
        assert_eq!(world.material(x, y - 10), Material::Empty);
        // The player starts inside, standing on the floor.
        let p = &colony.players[&key];
        assert_eq!(colony.dome_at(p.center()), Some(colony.home));
        assert!(!p.body().collides(&world, p.pose.pos, body::player_solid));
        assert!(
            colony
                .ents
                .values()
                .any(|e| matches!(&e.kind, EntKind::Crate(c) if c.kind == CrateKind::Cache))
        );
    }

    #[test]
    fn day_turns_to_night_and_weather_changes() {
        let (mut world, mut colony, _) = colony();
        assert!(!colony.clock.is_night());
        let bright = colony.clock.daylight();
        colony.clock.time = DAYLIGHT_SECS - 1.0;
        run(&mut colony, &mut world, 2.0);
        assert!(colony.clock.is_night());
        assert!(colony.clock.daylight() < bright);
        assert!(
            colony
                .take_events()
                .iter()
                .any(|e| matches!(e, Event::Fx { fx: Fx::Dusk, .. }))
        );
        colony.clock.time = DAY_SECS - 0.5;
        run(&mut colony, &mut world, 1.0);
        assert_eq!(colony.clock.day, 2);
        let first = colony.weather.kind;
        colony.weather.next = WeatherKind::Rain;
        colony.weather.remaining = 0.1;
        run(&mut colony, &mut world, 0.5);
        assert_eq!(colony.weather.kind, WeatherKind::Rain);
        let _ = first;
    }

    #[test]
    fn suit_oxygen_drains_outside_and_refills_in_the_dome() {
        let (mut world, mut colony, key) = colony();
        run(&mut colony, &mut world, 1.0);
        assert!(!colony.players[&key].suit, "no suit inside the dome");
        assert_eq!(colony.players[&key].o2, 100.0);
        // Walk out onto the surface, away from the dome.
        let outside = v2(colony.profile.spawn_x as f32 + 200.0, 0.0);
        let ground = colony.profile.surface_at(outside.x as i32) as f32;
        colony.players.get_mut(&key).unwrap().pose.pos = v2(outside.x, ground);
        run(&mut colony, &mut world, 30.0);
        let p = &colony.players[&key];
        assert!(p.suit && p.in_dome.is_none());
        assert!(p.o2 < 90.0 && p.o2 > 70.0, "about 3 minutes of air: {}", p.o2);
        // Out of air: health drains and the player blacks out, then wakes at home.
        colony.players.get_mut(&key).unwrap().o2 = 0.0;
        run(&mut colony, &mut world, 13.0);
        assert!(colony.players[&key].dead.is_some());
        run(&mut colony, &mut world, RESPAWN_SECS + 0.5);
        let p = &colony.players[&key];
        assert!(p.alive() && p.warp == 1);
        assert_eq!(colony.dome_at(p.center()), Some(colony.home));
        run(&mut colony, &mut world, 5.0);
        assert_eq!(colony.players[&key].o2, 100.0);
        // A greener atmosphere drains the tank more slowly.
        colony.atmosphere.o2 = 12.0;
        colony.players.get_mut(&key).unwrap().pose.pos = v2(outside.x, ground);
        run(&mut colony, &mut world, 30.0);
        assert!(colony.players[&key].o2 > 92.0);
        colony.atmosphere.o2 = 17.0;
        run(&mut colony, &mut world, 5.0);
        assert!(!colony.players[&key].suit, "breathable air: the suit comes off");
        assert_eq!(colony.players[&key].o2, 100.0);
    }

    #[test]
    fn dug_ore_becomes_drops_that_fly_to_the_player() {
        let (mut world, mut colony, key) = colony();
        let at = colony.players[&key].center() + v2(20.0, 0.0);
        let dug: Vec<_> = (0..12).map(|i| (i, 0, Material::IronOre)).collect();
        colony.collect_dug(key, &dug, at);
        let drops: u32 = colony
            .ents
            .values()
            .filter_map(|e| match &e.kind {
                EntKind::Drop(d) if d.item == Item::Iron => Some(d.count),
                _ => None,
            })
            .sum();
        assert_eq!(drops, 2, "12 cells at 5 per item");
        run(&mut colony, &mut world, 3.0);
        assert_eq!(colony.players[&key].inv.count(Item::Iron), 2);
        assert!(!colony.ents.values().any(|e| matches!(e.kind, EntKind::Drop(_))));
        // The remainder carries over.
        colony.collect_dug(key, &dug[..3], at);
        run(&mut colony, &mut world, 3.0);
        assert_eq!(colony.players[&key].inv.count(Item::Iron), 3);
    }

    #[test]
    fn players_keep_their_character_when_they_rejoin() {
        let (_, mut colony, key) = colony();
        colony.players.get_mut(&key).unwrap().inv.add(Item::Gold, 7, 1.0);
        colony.leave(key);
        assert_eq!(colony.online().count(), 0);
        colony.join(2, "Grace".into());
        assert_ne!(colony.players[&2].color, colony.players[&key].color);
        colony.join(key, "Ada L.".into());
        let p = &colony.players[&key];
        assert_eq!(p.inv.count(Item::Gold), 7);
        assert_eq!(p.name, "Ada L.");
        assert_eq!(colony.online().count(), 2);
    }

    #[test]
    fn replication_roundtrips() {
        let (mut world, mut host, key) = colony();
        let bytes = bincode::serialize(&host).unwrap();
        let mut client: Colony = bincode::deserialize(&bytes).unwrap();
        assert_eq!(client.ents, host.ents);
        host.take_changes();
        host.drop_item(v2(100.0, 100.0), Item::Wood, 3);
        let home = host.home;
        host.dome_mut(home).unwrap().chests[0].add(Item::Gold, 1, 1.0);
        run(&mut host, &mut world, 0.5);
        let (up, rm) = host.take_changes();
        assert_eq!(up.len(), 2);
        client.apply_changes(up, rm);
        let motion = host.motion_near(v2(100.0, 100.0), 500.0);
        assert!(!motion.is_empty());
        client.apply_motion(&motion);
        client.apply_globals(host.globals());
        assert_eq!(client.ents, host.ents);
        assert_eq!(client.clock, host.clock);
        // Removal.
        let drop_id = host
            .ents
            .values()
            .find(|e| matches!(e.kind, EntKind::Drop(_)))
            .unwrap()
            .id;
        host.despawn(drop_id);
        let (up, rm) = host.take_changes();
        client.apply_changes(up, rm);
        assert_eq!(client.ents, host.ents);
        client.apply_vitals(key, host.vitals(key).unwrap());
    }
}
