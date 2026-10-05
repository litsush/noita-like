//! Robots and the sentence builder.
//!
//! A robot's program is a short list of plain sentences built from word
//! blocks. Every tick the interpreter checks the sentences from the top: the
//! first one whose condition holds and whose current step can run is the
//! active one, so sentences higher up interrupt those below. A sentence that
//! can't run is skipped and remembers why, so the editor can show the reason.

use std::collections::{BTreeMap, BinaryHeap};

use serde::{Deserialize, Serialize};

use super::body::{line_of_sight, player_solid};
use super::creatures;
use super::domes::{Fixture, fixture_pos};
use super::farming::harvest_items;
use super::geom::{Rect, V2, v2};
use super::inventory::Inventory;
use super::items::{self, DomeKind, Item, MachineKind};
use super::plants::Product;
use super::{Bolt, Colony, EntKind, Fx, Id, PlayerKey, WeatherKind};
use crate::material::Material;
use crate::world::World;

pub const MAX_SENTENCES: usize = 8;
pub const MAX_STEPS: usize = 4;
pub const ROBOT_SLOTS: usize = 12;
pub const ROBOT_CHARGE: f32 = 100.0;
const SPEED: f32 = 62.0;
/// Charge used per second while working, hovering idle, and tunnelling.
const DRAIN_WORK: f32 = ROBOT_CHARGE / 900.0;
const DRAIN_IDLE: f32 = ROBOT_CHARGE / 3600.0;
const ARRIVE: f32 = 12.0;
const NODE: i32 = 8;

/// A named box on the map that robots can be sent to work in.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Area {
    pub id: u8,
    pub name: String,
    pub rect: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Place {
    /// A named dome or machine.
    Ent(Id),
    NearestDome,
    NearestStorage,
    NearestPylon,
}

/// A specific kind of thing, or any.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum What {
    Anything,
    Item(Item),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InvState {
    Full,
    Empty,
    NotEmpty,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeState {
    Day,
    Night,
    Raining,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Cond {
    Battery {
        below: bool,
        percent: u8,
    },
    Inventory(InvState),
    Time(TimeState),
    Predator {
        nearby: bool,
    },
    /// "[place] has [less than / more than] [n] [item]".
    Stock {
        place: Place,
        less: bool,
        count: u32,
        item: Item,
    },
}

/// Where harvesting happens: a dome, or an area outdoors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Where {
    Place(Place),
    Area(u8),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Step {
    GoTo(Place),
    FleeTo(Place),
    ChargeAt(Place),
    Harvest { what: What, at: Where },
    Plant { seed: What, at: Place },
    Water(Place),
    Fertilize(Place),
    Mine { ore: What, area: u8 },
    Scavenge { area: u8 },
    Store { what: What, at: Place },
    Take { what: What, from: Place },
    Guard { area: u8 },
    Wait { secs: u16 },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Clause {
    pub step: Step,
    /// "UNTIL …": ends this step early and moves on to the next.
    pub until: Option<Cond>,
}

impl From<Step> for Clause {
    fn from(step: Step) -> Clause {
        Clause { step, until: None }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Sentence {
    /// "WHEN …": the sentence only runs while this holds.
    pub when: Option<Cond>,
    pub steps: Vec<Clause>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RobotAnim {
    Idle,
    Work,
    Carry,
    Charge,
    Dead,
    Alarm,
}

/// Progress on the step being carried out. Thrown away when the robot
/// moves on to another step.
#[derive(Clone, Debug, Default, PartialEq)]
struct Scratch {
    work: f32,
    /// The step has done something since it started.
    did: bool,
    waited: f32,
    path: Vec<V2>,
    path_goal: Option<V2>,
    path_age: f32,
    /// A cell the step has picked to work on.
    focus: Option<(i32, i32)>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Robot {
    pub name: String,
    pub inv: Inventory,
    pub charge: f32,
    pub vel: V2,
    pub program: Vec<Sentence>,
    pub paused: bool,
    /// The sentence being carried out and the step within it.
    pub active: Option<u8>,
    pub step: u8,
    /// What it is doing, in words.
    pub status: String,
    /// Why each sentence can't run, if it can't.
    pub blocked: Vec<Option<String>>,
    pub anim: RobotAnim,
    pub facing: i8,
    /// Colour band, to tell robots apart.
    pub color: u8,
    #[serde(skip)]
    scratch: Scratch,
    #[serde(skip)]
    tally: BTreeMap<u8, f32>,
    #[serde(skip)]
    cooldown: f32,
}

impl Robot {
    pub fn new(name: String, color: u8) -> Robot {
        Robot {
            name,
            inv: Inventory::new(ROBOT_SLOTS),
            charge: ROBOT_CHARGE,
            vel: V2::ZERO,
            program: Vec::new(),
            paused: false,
            active: None,
            step: 0,
            status: "No program yet".into(),
            blocked: Vec::new(),
            anim: RobotAnim::Idle,
            facing: 1,
            color,
            scratch: Scratch::default(),
            tally: BTreeMap::new(),
            cooldown: 0.0,
        }
    }

    /// Animation and facing packed for the motion stream.
    pub fn anim_byte(&self) -> u8 {
        let a = match self.anim {
            RobotAnim::Idle => 0,
            RobotAnim::Work => 1,
            RobotAnim::Carry => 2,
            RobotAnim::Charge => 3,
            RobotAnim::Dead => 4,
            RobotAnim::Alarm => 5,
        };
        a | if self.facing < 0 { 0x80 } else { 0 }
    }

    pub fn set_anim_byte(&mut self, a: u8) {
        self.anim = match a & 0x0F {
            1 => RobotAnim::Work,
            2 => RobotAnim::Carry,
            3 => RobotAnim::Charge,
            4 => RobotAnim::Dead,
            5 => RobotAnim::Alarm,
            _ => RobotAnim::Idle,
        };
        self.facing = if a & 0x80 != 0 { -1 } else { 1 };
    }

    fn reset_step(&mut self) {
        self.scratch = Scratch::default();
    }

    /// What clients need to see change.
    fn display_key(&self) -> (Option<u8>, u8, String, Vec<Option<String>>, u8, u32) {
        (
            self.active,
            self.step,
            self.status.clone(),
            self.blocked.clone(),
            (self.charge / 5.0) as u8,
            self.inv.stacks().map(|s| s.count).sum(),
        )
    }
}

// ---- sentences as text -----------------------------------------------------

impl What {
    fn text(self, colony: &Colony, any: &str) -> String {
        match self {
            What::Anything => any.to_string(),
            What::Item(i) => colony.item_name(i).to_lowercase(),
        }
    }

    fn matches(self, item: Item) -> bool {
        match self {
            What::Anything => true,
            What::Item(i) => i == item,
        }
    }
}

impl Place {
    pub fn text(self, colony: &Colony) -> String {
        match self {
            Place::Ent(id) => match colony.ents.get(&id).map(|e| &e.kind) {
                Some(EntKind::Dome(d)) => d.name.clone(),
                Some(EntKind::Machine(m)) => m.name.clone(),
                _ => "(gone)".into(),
            },
            Place::NearestDome => "the nearest dome".into(),
            Place::NearestStorage => "the nearest storage".into(),
            Place::NearestPylon => "the nearest pylon".into(),
        }
    }
}

fn area_name(colony: &Colony, id: u8) -> String {
    colony
        .areas
        .iter()
        .find(|a| a.id == id)
        .map_or("(deleted area)".to_string(), |a| a.name.clone())
}

impl Cond {
    pub fn text(&self, colony: &Colony) -> String {
        match *self {
            Cond::Battery { below, percent } => {
                format!("battery is {} {percent}%", if below { "below" } else { "above" })
            }
            Cond::Inventory(s) => format!(
                "inventory is {}",
                match s {
                    InvState::Full => "full",
                    InvState::Empty => "empty",
                    InvState::NotEmpty => "not empty",
                }
            ),
            Cond::Time(t) => format!(
                "it is {}",
                match t {
                    TimeState::Day => "day",
                    TimeState::Night => "night",
                    TimeState::Raining => "raining",
                }
            ),
            Cond::Predator { nearby } => {
                format!("a predator is {}", if nearby { "nearby" } else { "not nearby" })
            }
            Cond::Stock {
                place,
                less,
                count,
                item,
            } => format!(
                "{} has {} {count} {}",
                place.text(colony),
                if less { "less than" } else { "more than" },
                colony.item_name(item).to_lowercase()
            ),
        }
    }
}

impl Step {
    pub fn text(&self, colony: &Colony) -> String {
        match *self {
            Step::GoTo(p) => format!("go to {}", p.text(colony)),
            Step::FleeTo(p) => format!("flee to {}", p.text(colony)),
            Step::ChargeAt(p) => format!("charge at {}", p.text(colony)),
            Step::Harvest { what, at } => format!(
                "harvest {} in {}",
                what.text(colony, "anything"),
                match at {
                    Where::Place(p) => p.text(colony),
                    Where::Area(a) => area_name(colony, a),
                }
            ),
            Step::Plant { seed, at } => {
                format!("plant {} in {}", seed.text(colony, "any seed"), at.text(colony))
            }
            Step::Water(p) => format!("water plants in {}", p.text(colony)),
            Step::Fertilize(p) => format!("fertilize plants in {}", p.text(colony)),
            Step::Mine { ore, area } => format!(
                "mine {} in {}",
                ore.text(colony, "any ore"),
                area_name(colony, area)
            ),
            Step::Scavenge { area } => format!("scavenge in {}", area_name(colony, area)),
            Step::Store { what, at } => {
                format!("store {} in {}", what.text(colony, "everything"), at.text(colony))
            }
            Step::Take { what, from } => format!(
                "take {} from {}",
                what.text(colony, "anything"),
                from.text(colony)
            ),
            Step::Guard { area } => format!("guard {}", area_name(colony, area)),
            Step::Wait { secs } => format!("wait {secs} seconds"),
        }
    }
}

impl Sentence {
    /// The sentence as the player reads it.
    pub fn text(&self, colony: &Colony) -> String {
        let mut s = String::new();
        if let Some(c) = &self.when {
            s.push_str(&format!("WHEN {}, ", c.text(colony)));
        }
        let steps: Vec<String> = self
            .steps
            .iter()
            .map(|c| match &c.until {
                Some(u) => format!("{} UNTIL {}", c.step.text(colony), u.text(colony)),
                None => c.step.text(colony),
            })
            .collect();
        s.push_str(&steps.join(", then "));
        s.push('.');
        // Capitalise the first letter.
        let mut chars = s.chars();
        match chars.next() {
            Some(f) => f.to_uppercase().collect::<String>() + chars.as_str(),
            None => s,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum RobotOp {
    SetProgram(Vec<Sentence>),
    Pause(bool),
    /// Pack the robot back into a kit; its cargo is dropped.
    PickUp,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum AreaOp {
    Add { name: String, rect: Rect },
    Remove(u8),
}

/// What running a step did this tick.
enum Outcome {
    Working(String),
    Done,
    Blocked(String),
}

use Outcome::{Blocked, Done, Working};

// ---- path finding ------------------------------------------------------------

fn node_open(world: &World, n: (i32, i32)) -> bool {
    let (x0, y0) = (n.0 * NODE, n.1 * NODE);
    if x0 < NODE
        || y0 < NODE
        || x0 + NODE >= world.width() as i32 - NODE
        || y0 + NODE >= world.height() as i32 - NODE
    {
        return false;
    }
    // The middle of the block must be clear enough for a small drone.
    (1..NODE - 1).step_by(2).all(|dy| {
        (1..NODE - 1)
            .step_by(2)
            .all(|dx| !player_solid(world.material(x0 + dx, y0 + dy)))
    })
}

fn node_center(n: (i32, i32)) -> V2 {
    v2(
        (n.0 * NODE) as f32 + NODE as f32 / 2.0,
        (n.1 * NODE) as f32 + NODE as f32 / 2.0,
    )
}

/// A* over 8-cell blocks of open space. Returns waypoints, or `None` if no
/// route was found within the search budget.
pub fn find_path(world: &World, from: V2, to: V2) -> Option<Vec<V2>> {
    let node = |p: V2| ((p.x as i32).div_euclid(NODE), (p.y as i32).div_euclid(NODE));
    let (start, goal) = (node(from), node(to));
    if start == goal {
        return Some(vec![to]);
    }
    let h = |n: (i32, i32)| {
        let (dx, dy) = ((n.0 - goal.0).abs(), (n.1 - goal.1).abs());
        (dx.max(dy) * 10 + dx.min(dy) * 4) as u32
    };
    let mut open: BinaryHeap<(std::cmp::Reverse<u32>, (i32, i32))> = BinaryHeap::new();
    let mut best: BTreeMap<(i32, i32), (u32, (i32, i32))> = BTreeMap::new();
    let mut passable: BTreeMap<(i32, i32), bool> = BTreeMap::new();
    let mut can =
        |n: (i32, i32)| n == start || n == goal || *passable.entry(n).or_insert_with(|| node_open(world, n));
    best.insert(start, (0, start));
    open.push((std::cmp::Reverse(h(start)), start));
    let mut budget = 5000;
    while let Some((_, n)) = open.pop() {
        if n == goal {
            let mut path = vec![to];
            let mut cur = goal;
            while cur != start {
                cur = best[&cur].1;
                if cur != start {
                    path.push(node_center(cur));
                }
            }
            path.reverse();
            return Some(path);
        }
        budget -= 1;
        if budget == 0 {
            return None;
        }
        let g = best[&n].0;
        for (dx, dy) in [
            (1, 0),
            (-1, 0),
            (0, 1),
            (0, -1),
            (1, 1),
            (1, -1),
            (-1, 1),
            (-1, -1),
        ] {
            let m = (n.0 + dx, n.1 + dy);
            if !can(m) {
                continue;
            }
            // No cutting corners through rock.
            if dx != 0 && dy != 0 && (!can((n.0 + dx, n.1)) || !can((n.0, n.1 + dy))) {
                continue;
            }
            let cost = g + if dx != 0 && dy != 0 { 14 } else { 10 };
            if best.get(&m).is_none_or(|&(old, _)| cost < old) {
                best.insert(m, (cost, n));
                open.push((std::cmp::Reverse(cost + h(m)), m));
            }
        }
    }
    None
}

// ---- the interpreter -----------------------------------------------------------

impl Colony {
    fn place_target(&self, from: V2, place: Place) -> Result<(Id, V2), String> {
        let spot = |c: &Colony, id: Id| -> Option<V2> {
            let e = c.ents.get(&id)?;
            match &e.kind {
                // Hover inside the dome, above the floor.
                EntKind::Dome(_) => Some(v2(e.pos.x, e.pos.y - 16.0)),
                EntKind::Machine(_) => Some(v2(e.pos.x, e.pos.y - 14.0)),
                _ => None,
            }
        };
        let nearest = |c: &Colony, pick: &dyn Fn(&EntKind) -> bool| -> Option<(Id, V2)> {
            c.ents
                .values()
                .filter(|e| pick(&e.kind))
                .filter_map(|e| Some((e.id, spot(c, e.id)?)))
                .min_by(|a, b| a.1.distance(from).total_cmp(&b.1.distance(from)))
        };
        match place {
            Place::Ent(id) => spot(self, id)
                .map(|p| (id, p))
                .ok_or_else(|| "That place is gone".into()),
            Place::NearestDome => {
                nearest(self, &|k| matches!(k, EntKind::Dome(_))).ok_or_else(|| "There are no domes".into())
            }
            Place::NearestStorage => nearest(self, &|k| match k {
                EntKind::Dome(d) => matches!(d.kind, DomeKind::Storage | DomeKind::Starter),
                EntKind::Machine(m) => m.kind == MachineKind::Chest,
                _ => false,
            })
            .ok_or_else(|| "There is no storage".into()),
            Place::NearestPylon => nearest(
                self,
                &|k| matches!(k, EntKind::Machine(m) if m.kind == MachineKind::Pylon),
            )
            .ok_or_else(|| "There are no Charging Pylons".into()),
        }
    }

    /// Chests of a place: a dome's, or a chest machine's.
    fn place_chests(&mut self, id: Id) -> Vec<&mut Inventory> {
        self.touch(id);
        match self.ents.get_mut(&id).map(|e| &mut e.kind) {
            Some(EntKind::Dome(d)) => d.chests.iter_mut().collect(),
            Some(EntKind::Machine(m)) if matches!(m.kind, MachineKind::Chest | MachineKind::Pylon) => {
                vec![&mut m.inv]
            }
            _ => Vec::new(),
        }
    }

    fn place_count(&self, id: Id, item: Item) -> u32 {
        match self.ents.get(&id).map(|e| &e.kind) {
            Some(EntKind::Dome(d)) => d.chests.iter().map(|c| c.count(item)).sum(),
            Some(EntKind::Machine(m)) => m.inv.count(item),
            _ => 0,
        }
    }

    fn eval(&self, robot: &Robot, pos: V2, cond: &Cond) -> bool {
        match *cond {
            Cond::Battery { below, percent } => {
                let pct = robot.charge / ROBOT_CHARGE * 100.0;
                if below {
                    pct < percent as f32
                } else {
                    pct > percent as f32
                }
            }
            Cond::Inventory(s) => match s {
                InvState::Full => robot.inv.free_slots() == 0,
                InvState::Empty => robot.inv.is_empty(),
                InvState::NotEmpty => !robot.inv.is_empty(),
            },
            Cond::Time(t) => match t {
                TimeState::Day => !self.clock.is_night(),
                TimeState::Night => self.clock.is_night(),
                TimeState::Raining => self.weather.kind == WeatherKind::Rain,
            },
            Cond::Predator { nearby } => {
                let near = self.ents.values().any(|e| {
                    matches!(&e.kind, EntKind::Creature(c) if c.kind.predator() && c.state != creatures::State::Dying)
                        && e.pos.distance(pos) < 90.0
                });
                near == nearby
            }
            Cond::Stock {
                place,
                less,
                count,
                item,
            } => match self.place_target(pos, place) {
                Ok((id, _)) => {
                    let n = self.place_count(id, item);
                    if less { n < count } else { n > count }
                }
                Err(_) => false,
            },
        }
    }

    pub(super) fn deploy_robot(
        &mut self,
        world: &World,
        key: PlayerKey,
        slot: usize,
        at: V2,
    ) -> Result<(), &'static str> {
        let (x, y) = at.cell();
        if player_solid(world.material(x, y)) {
            return Err("Not enough room");
        }
        let n = self
            .ents
            .values()
            .filter(|e| matches!(e.kind, EntKind::Robot(_)))
            .count();
        let robot = Robot::new(format!("Robot {}", n + 1), (n % 8) as u8);
        self.spawn(at, EntKind::Robot(robot));
        self.consume_held(key, slot);
        self.fx(Fx::RobotOk, at);
        Ok(())
    }

    pub(super) fn robot_op(&mut self, key: PlayerKey, id: Id, op: RobotOp) -> Result<(), &'static str> {
        let e = self.ents.get(&id).ok_or("That robot is gone")?;
        let EntKind::Robot(_) = &e.kind else {
            return Err("That isn't a robot");
        };
        let pos = e.pos;
        let p = &self.players[&key];
        if pos.distance(p.center()) > p.stats().reach + 24.0 && !p.stats().remote_robots {
            return Err("Too far away");
        }
        match op {
            RobotOp::SetProgram(program) => {
                if program.len() > MAX_SENTENCES {
                    return Err("At most 8 sentences");
                }
                if program
                    .iter()
                    .any(|s| s.steps.is_empty() || s.steps.len() > MAX_STEPS)
                {
                    return Err("Each sentence needs one to four steps");
                }
                let Some(EntKind::Robot(r)) = self.ents.get_mut(&id).map(|e| &mut e.kind) else {
                    unreachable!()
                };
                r.blocked = vec![None; program.len()];
                r.program = program;
                r.active = None;
                r.step = 0;
                r.scratch.focus = None;
                r.scratch.path.clear();
                self.touch(id);
                self.fx(Fx::RobotOk, pos);
            }
            RobotOp::Pause(paused) => {
                if let Some(EntKind::Robot(r)) = self.ents.get_mut(&id).map(|e| &mut e.kind) {
                    r.paused = paused;
                }
                self.touch(id);
            }
            RobotOp::PickUp => {
                let Some(Ent_ { cargo }) = self.ents.get(&id).map(|e| match &e.kind {
                    EntKind::Robot(r) => Ent_ {
                        cargo: r.inv.stacks().copied().collect::<Vec<_>>(),
                    },
                    _ => Ent_ { cargo: Vec::new() },
                }) else {
                    unreachable!()
                };
                self.despawn(id);
                for s in cargo {
                    self.drop_item(pos, s.item, s.count);
                }
                self.give(key, Item::RobotKit, 1);
            }
        }
        Ok(())
    }

    pub(super) fn area_op(&mut self, op: AreaOp) -> Result<(), &'static str> {
        match op {
            AreaOp::Add { name, rect } => {
                if self.areas.len() >= 24 {
                    return Err("Too many areas: remove one first");
                }
                let name: String = name.chars().filter(|c| !c.is_control()).take(24).collect();
                let id = (0..=255u8)
                    .find(|i| self.areas.iter().all(|a| a.id != *i))
                    .ok_or("Too many areas")?;
                let rect = Rect::new(rect.x0, rect.y0, rect.x1, rect.y1);
                if rect.x1 - rect.x0 < 8 || rect.y1 - rect.y0 < 8 {
                    return Err("Drag a bigger box");
                }
                let name = if name.trim().is_empty() {
                    format!("Area {}", id + 1)
                } else {
                    name
                };
                self.areas.push(Area { id, name, rect });
            }
            AreaOp::Remove(id) => self.areas.retain(|a| a.id != id),
        }
        self.meta_rev += 1;
        Ok(())
    }

    /// Runs every robot for `dt` seconds.
    pub(super) fn step_robots(&mut self, world: &mut World, dt: f32) {
        let ids: Vec<Id> = self
            .ents
            .values()
            .filter(|e| matches!(e.kind, EntKind::Robot(_)))
            .map(|e| e.id)
            .collect();
        for id in ids {
            let Some(e) = self.ents.get_mut(&id) else { continue };
            let pos = e.pos;
            // Take the robot out while it runs so it can use the rest of the colony.
            let EntKind::Robot(robot) = &mut e.kind else {
                continue;
            };
            let mut robot = std::mem::replace(robot, Robot::new(String::new(), 0));
            let before = robot.display_key();
            let mut pos = pos;
            // Hive Mind: robots near (or, at Legendary, anywhere) work faster.
            let (speed, free) = self.robot_boost(pos);
            let charge = robot.charge;
            self.run_robot(world, &mut robot, &mut pos, dt * speed);
            if free {
                robot.charge = robot.charge.max(charge);
            }
            let changed = robot.display_key() != before;
            if let Some(e) = self.ents.get_mut(&id) {
                e.pos = pos;
                e.kind = EntKind::Robot(robot);
            }
            if changed {
                self.touch(id);
            }
        }
    }

    fn run_robot(&mut self, world: &mut World, r: &mut Robot, pos: &mut V2, dt: f32) {
        r.cooldown = (r.cooldown - dt).max(0.0);
        r.scratch.path_age += dt;
        r.vel = V2::ZERO;
        if r.blocked.len() != r.program.len() {
            r.blocked = vec![None; r.program.len()];
        }
        if r.paused {
            r.status = "Paused".into();
            r.anim = RobotAnim::Idle;
            return;
        }
        if r.charge <= 0.0 {
            r.charge = 0.0;
            r.anim = RobotAnim::Dead;
            r.status = "Out of power: carry a battery flower to it".into();
            r.active = None;
            return;
        }
        // Predators gnaw on robots, draining them.
        let bitten = self.ents.values().any(|e| {
            matches!(&e.kind, EntKind::Creature(c) if c.kind.predator() && !matches!(c.state, creatures::State::Dying | creatures::State::Hidden))
                && e.pos.distance(*pos) < 14.0
        });
        if bitten {
            r.charge = (r.charge - 4.0 * dt).max(0.0);
        }
        if r.program.is_empty() {
            r.status = "No program yet".into();
            r.anim = RobotAnim::Idle;
            r.charge -= DRAIN_IDLE * dt;
            return;
        }

        for i in 0..r.program.len() {
            let sentence = r.program[i].clone();
            let was_active = r.active == Some(i as u8);
            let mut index = if was_active {
                (r.step as usize).min(sentence.steps.len() - 1)
            } else {
                0
            };
            // A robot that has started charging or fleeing finishes, even
            // though its battery is no longer "below 20%" or the predator
            // is out of sight.
            let charging =
                was_active && matches!(sentence.steps[index].step, Step::ChargeAt(_) | Step::FleeTo(_));
            if let Some(c) = &sentence.when
                && !charging
                && !self.eval(r, *pos, c)
            {
                r.blocked[i] = None;
                continue;
            }
            // A different sentence taking over starts from its first step.
            // Its progress is put back if it turns out it can't run.
            let saved = (!was_active).then(|| std::mem::take(&mut r.scratch));
            // UNTIL ends a step early. If that runs off the end of the
            // sentence there is nothing for it to do right now.
            while index < sentence.steps.len()
                && sentence.steps[index]
                    .until
                    .as_ref()
                    .is_some_and(|c| self.eval(r, *pos, c))
            {
                index += 1;
                r.reset_step();
            }
            if index == sentence.steps.len() {
                if let Some(saved) = saved {
                    r.scratch = saved;
                }
                r.blocked[i] = None;
                if was_active {
                    r.active = None;
                    r.step = 0;
                }
                continue;
            }
            let step = sentence.steps[index].step;
            match self.run_step(world, r, pos, &step, dt) {
                Working(status) => {
                    r.active = Some(i as u8);
                    r.step = index as u8;
                    r.blocked[i] = None;
                    r.status = status;
                    r.charge -= DRAIN_WORK * dt;
                    return;
                }
                Done => {
                    // After the last step the sentence starts over, and
                    // its WHEN is asked again.
                    let next = (index + 1) % sentence.steps.len();
                    r.active = (next != 0).then_some(i as u8);
                    r.step = next as u8;
                    r.blocked[i] = None;
                    r.reset_step();
                    r.status = format!("Finished: {}", step.text(self));
                    return;
                }
                Blocked(reason) => {
                    if let Some(saved) = saved {
                        r.scratch = saved;
                    }
                    r.blocked[i] = Some(reason);
                    if was_active {
                        r.active = None;
                        r.step = 0;
                    }
                }
            }
        }
        r.active = None;
        r.anim = RobotAnim::Idle;
        r.charge -= DRAIN_IDLE * dt;
        r.status = match r.blocked.iter().flatten().next() {
            Some(reason) => format!("Waiting: {reason}"),
            None => "Idle: no sentence applies right now".into(),
        };
    }

    /// Flies toward `goal`. Returns true on arrival.
    fn fly(&mut self, world: &mut World, r: &mut Robot, pos: &mut V2, goal: V2, speed: f32, dt: f32) -> bool {
        let dist = pos.distance(goal);
        if dist <= ARRIVE {
            r.scratch.path.clear();
            return true;
        }
        r.anim = if r.inv.is_empty() {
            RobotAnim::Idle
        } else {
            RobotAnim::Carry
        };
        // Straight there when nothing is in the way; otherwise plan a route.
        let direct = line_of_sight(world, *pos, goal, player_solid);
        if !direct {
            let stale = r.scratch.path_goal.is_none_or(|g| g.distance(goal) > 16.0)
                || r.scratch.path_age > 3.0
                || r.scratch.path.is_empty();
            if stale {
                r.scratch.path = find_path(world, *pos, goal).unwrap_or_default();
                r.scratch.path_goal = Some(goal);
                r.scratch.path_age = 0.0;
            }
        } else {
            r.scratch.path.clear();
        }
        // Skip waypoints we can already see past.
        while r.scratch.path.len() > 1 && line_of_sight(world, *pos, r.scratch.path[1], player_solid) {
            r.scratch.path.remove(0);
        }
        let next = r.scratch.path.first().copied().unwrap_or(goal);
        if pos.distance(next) < 4.0 && !r.scratch.path.is_empty() {
            r.scratch.path.remove(0);
        }
        let dir = (next - *pos).normalized();
        let blocked = !direct && r.scratch.path.is_empty();
        let v = if blocked {
            // No route: drill straight through.
            let (x, y) = (*pos + dir * 5.0).cell();
            world.dig(x, y, 5, 90);
            r.anim = RobotAnim::Work;
            speed * 0.22
        } else {
            speed
        };
        r.vel = dir * v;
        *pos += r.vel * dt;
        if dir.x.abs() > 0.2 {
            r.facing = dir.x.signum() as i8;
        }
        false
    }

    fn run_step(&mut self, world: &mut World, r: &mut Robot, pos: &mut V2, step: &Step, dt: f32) -> Outcome {
        let here = *pos;
        match *step {
            Step::GoTo(place) => match self.place_target(here, place) {
                Err(why) => Blocked(why),
                Ok((_, at)) => {
                    if self.fly(world, r, pos, at, SPEED, dt) {
                        Done
                    } else {
                        Working(format!("Going to {}", place.text(self)))
                    }
                }
            },
            Step::FleeTo(place) => match self.place_target(here, place) {
                Err(why) => Blocked(why),
                Ok((_, at)) => {
                    r.anim = RobotAnim::Alarm;
                    if self.fly(world, r, pos, at, SPEED * 1.6, dt) {
                        Done
                    } else {
                        r.anim = RobotAnim::Alarm;
                        Working(format!("Fleeing to {}", place.text(self)))
                    }
                }
            },
            Step::ChargeAt(place) => {
                let (id, at) = match self.place_target(here, place) {
                    Ok(t) => t,
                    Err(why) => return Blocked(why),
                };
                if !matches!(self.ents.get(&id).map(|e| &e.kind), Some(EntKind::Machine(m)) if m.kind == MachineKind::Pylon)
                {
                    return Blocked(format!("{} is not a Charging Pylon", place.text(self)));
                }
                if r.charge >= ROBOT_CHARGE - 0.5 {
                    return Done;
                }
                if !self.fly(world, r, pos, at, SPEED, dt) {
                    return Working(format!("Going to charge at {}", place.text(self)));
                }
                // Draw from the pylon, loading a flower if its buffer is dry.
                let species = self.species.clone();
                let Some(EntKind::Machine(m)) = self.ents.get_mut(&id).map(|e| &mut e.kind) else {
                    return Blocked("That pylon is gone".into());
                };
                let flower = m
                    .inv
                    .stacks()
                    .find(|s| s.item.charge(&species) > 0.0)
                    .map(|s| s.item);
                if m.store < 1.0
                    && let Some(flower) = flower
                {
                    m.inv.take(flower, 1);
                    m.store += flower.charge(&species);
                }
                if m.store < 0.5 {
                    return Blocked(format!("{} has no charge: load battery flowers", m.name));
                }
                let take = (30.0 * dt).min(m.store).min(ROBOT_CHARGE - r.charge);
                m.store -= take;
                r.charge += take + DRAIN_WORK * dt;
                r.anim = RobotAnim::Charge;
                Working(format!("Charging at {}", place.text(self)))
            }
            Step::Wait { secs } => {
                r.scratch.waited += dt;
                r.anim = RobotAnim::Idle;
                if r.scratch.waited >= secs as f32 {
                    Done
                } else {
                    Working(format!("Waiting ({:.0} s left)", secs as f32 - r.scratch.waited))
                }
            }
            Step::Store { what, at } => {
                if !r.inv.stacks().any(|s| what.matches(s.item)) {
                    return Blocked("Nothing on board to store".into());
                }
                let (id, spot) = match self.place_target(here, at) {
                    Ok(t) => t,
                    Err(why) => return Blocked(why),
                };
                if !self.fly(world, r, pos, spot, SPEED, dt) {
                    return Working(format!("Carrying cargo to {}", at.text(self)));
                }
                let name = at.text(self);
                let mut chests = self.place_chests(id);
                if chests.is_empty() {
                    return Blocked(format!("{name} has no chests"));
                }
                let mut moved = false;
                for slot in r.inv.slots.iter_mut() {
                    let Some(s) = slot else { continue };
                    if !what.matches(s.item) {
                        continue;
                    }
                    for chest in chests.iter_mut() {
                        let left = chest.add(s.item, s.count, 1.0);
                        moved |= left < s.count;
                        s.count = left;
                        if left == 0 {
                            break;
                        }
                    }
                    if s.count == 0 {
                        *slot = None;
                    }
                }
                if r.inv.stacks().any(|s| what.matches(s.item)) {
                    if moved {
                        Working(format!("Unloading at {name}"))
                    } else {
                        Blocked(format!("{name} is full"))
                    }
                } else {
                    self.events.push(super::Event::Fx {
                        fx: Fx::RobotOk,
                        pos: here,
                    });
                    Done
                }
            }
            Step::Take { what, from } => {
                let (id, spot) = match self.place_target(here, from) {
                    Ok(t) => t,
                    Err(why) => return Blocked(why),
                };
                let name = from.text(self);
                let available = self
                    .place_chests(id)
                    .iter()
                    .any(|c| c.stacks().any(|s| what.matches(s.item)));
                if !available {
                    return Blocked(format!("{name} has no {}", what.text(self, "items")));
                }
                if r.inv.free_slots() == 0 {
                    return Done;
                }
                if !self.fly(world, r, pos, spot, SPEED, dt) {
                    return Working(format!("Going to {name} for {}", what.text(self, "items")));
                }
                // One stack per visit.
                let mut chests = self.place_chests(id);
                for chest in chests.iter_mut() {
                    let pick = chest.stacks().find(|s| what.matches(s.item)).copied();
                    if let Some(s) = pick {
                        let n = s.count.min(s.item.max_stack().min(99));
                        let left = r.inv.add(s.item, n, 1.0);
                        chest.take(s.item, n - left);
                        break;
                    }
                }
                Done
            }
            Step::Harvest { what, at } => self.step_harvest(world, r, pos, what, at, dt),
            Step::Plant { seed, at } => self.step_planters(world, r, pos, at, dt, PlanterJob::Plant(seed)),
            Step::Water(at) => self.step_planters(world, r, pos, at, dt, PlanterJob::Water),
            Step::Fertilize(at) => self.step_planters(world, r, pos, at, dt, PlanterJob::Fertilize),
            Step::Mine { ore, area } => self.step_mine(world, r, pos, ore, area, dt),
            Step::Scavenge { area } => self.step_scavenge(world, r, pos, area, dt),
            Step::Guard { area } => self.step_guard(world, r, pos, area, dt),
        }
    }

    fn area_rect(&self, id: u8) -> Result<Rect, String> {
        self.areas
            .iter()
            .find(|a| a.id == id)
            .map(|a| a.rect)
            .ok_or_else(|| "That area was deleted".into())
    }

    fn step_harvest(
        &mut self,
        world: &mut World,
        r: &mut Robot,
        pos: &mut V2,
        what: What,
        at: Where,
        dt: f32,
    ) -> Outcome {
        if r.inv.free_slots() == 0 {
            return Done;
        }
        let wants = |c: &Colony, species: u16| -> bool {
            match what {
                What::Anything => true,
                What::Item(Item::Crop(s) | Item::Seed(s)) => s == species,
                What::Item(item) => c.species.get(species as usize).is_some_and(|sp| {
                    [
                        (Product::Wood, Item::Wood),
                        (Product::Fiber, Item::Fiber),
                        (Product::Fertilizer, Item::Fertilizer),
                        (Product::Gel, Item::BioGel),
                    ]
                    .iter()
                    .any(|&(p, i)| i == item && sp.yields(p))
                }),
            }
        };
        // Find something ripe: (where to hover, planter or plant).
        enum Target {
            Planter(Id, u8),
            Plant(Id),
        }
        let here = *pos;
        let (label, found): (String, Option<(V2, Target)>) = match at {
            Where::Place(place) => {
                let (id, _) = match self.place_target(here, place) {
                    Ok(t) => t,
                    Err(why) => return Blocked(why),
                };
                let Some((e, d)) = self.dome(id) else {
                    return Blocked(format!("{} is not a dome", place.text(self)));
                };
                let found = d.kind.fixtures().iter().find_map(|f| match f.fixture {
                    Fixture::Planter(i) => {
                        let p = d.planters.get(i as usize)?;
                        let species = p.species?;
                        let ripe = p.growth >= 1.0 && self.species.get(species as usize)?.product.is_some();
                        (ripe && wants(self, species)).then(|| {
                            let fp = fixture_pos(e.pos, f);
                            (v2(fp.x, fp.y - 16.0), Target::Planter(id, i))
                        })
                    }
                    _ => None,
                });
                (place.text(self), found)
            }
            Where::Area(a) => {
                let rect = match self.area_rect(a) {
                    Ok(r) => r,
                    Err(why) => return Blocked(why),
                };
                let found = self
                    .ents
                    .values()
                    .filter(|e| {
                        matches!(e.kind, EntKind::Plant(p) if p.growth >= 1.0 && wants(self, p.species))
                            && rect.contains(e.pos)
                    })
                    .min_by(|a, b| a.pos.distance(here).total_cmp(&b.pos.distance(here)))
                    .map(|e| (v2(e.pos.x, e.pos.y - 12.0), Target::Plant(e.id)));
                (area_name(self, a), found)
            }
        };
        let Some((spot, target)) = found else {
            return if r.inv.is_empty() {
                Blocked(format!("Nothing is ripe in {label}"))
            } else {
                Done
            };
        };
        if !self.fly(world, r, pos, spot, SPEED, dt) {
            return Working(format!("Going to harvest in {label}"));
        }
        r.anim = RobotAnim::Work;
        r.scratch.work += dt;
        if r.scratch.work < 1.2 {
            return Working(format!("Harvesting {} in {label}", what.text(self, "crops")));
        }
        r.scratch.work = 0.0;
        let species_id = match target {
            Target::Planter(dome, slot) => {
                let d = self.dome_mut(dome).unwrap();
                let s = d.planters[slot as usize].species.unwrap();
                d.planters[slot as usize].growth = 0.35;
                s
            }
            Target::Plant(id) => {
                let Some(EntKind::Plant(p)) = self.ents.get_mut(&id).map(|e| &mut e.kind) else {
                    return Working("Harvesting".into());
                };
                let (s, wild) = (p.species, p.wild);
                if wild && s == super::plants::VOLTBLOOM {
                    self.despawn(id);
                } else {
                    p.growth = 0.35;
                    self.touch(id);
                }
                s
            }
        };
        let sp = self.species[species_id as usize].clone();
        for (item, n) in harvest_items(&sp, 1.0, &mut self.rng) {
            let left = r.inv.add(item, n, 1.0);
            if left > 0 {
                self.drop_item(*pos, item, left);
            }
        }
        self.discover(species_id);
        self.fx(Fx::Harvest, spot);
        Working(format!("Harvesting in {label}"))
    }

    fn step_planters(
        &mut self,
        world: &mut World,
        r: &mut Robot,
        pos: &mut V2,
        at: Place,
        dt: f32,
        job: PlanterJob,
    ) -> Outcome {
        let here = *pos;
        let (id, _) = match self.place_target(here, at) {
            Ok(t) => t,
            Err(why) => return Blocked(why),
        };
        let label = at.text(self);
        let Some((e, d)) = self.dome(id) else {
            return Blocked(format!("{label} is not a dome"));
        };
        if d.planters.is_empty() {
            return Blocked(format!("{label} has no planters"));
        }
        let bed = d.kind.bed();
        // What the robot would sow, if sowing.
        let seed = match job {
            PlanterJob::Plant(what) => {
                let seed = r.inv.stacks().find_map(|s| match s.item {
                    Item::Seed(sp)
                        if what.matches(s.item)
                            && self.species.get(sp as usize).is_some_and(|x| x.fit(bed) > 0.0) =>
                    {
                        Some(sp)
                    }
                    _ => None,
                });
                if seed.is_none() {
                    return Blocked(format!("No seeds on board that grow in {label}"));
                }
                seed
            }
            PlanterJob::Fertilize if r.inv.count(Item::Fertilizer) == 0 => {
                return Blocked("No fertilizer on board".into());
            }
            _ => None,
        };
        let found = d.kind.fixtures().iter().find_map(|f| match f.fixture {
            Fixture::Planter(i) => {
                let p = d.planters.get(i as usize)?;
                let sp = p.species.and_then(|s| self.species.get(s as usize));
                let needs = match job {
                    PlanterJob::Plant(_) => p.species.is_none(),
                    PlanterJob::Water => sp.is_some_and(|s| s.thirst_per_day() > 0.0) && p.moisture < 0.6,
                    PlanterJob::Fertilize => p.species.is_some() && p.fertilizer <= 0.0,
                };
                needs.then(|| {
                    let fp = fixture_pos(e.pos, f);
                    (v2(fp.x, fp.y - 16.0), i)
                })
            }
            _ => None,
        });
        let verb = match job {
            PlanterJob::Plant(_) => "Planting",
            PlanterJob::Water => "Watering",
            PlanterJob::Fertilize => "Fertilizing",
        };
        let Some((spot, slot)) = found else {
            // Nothing needs doing: that is this step finished.
            return if r.scratch.did {
                Done
            } else {
                Blocked(match job {
                    PlanterJob::Plant(_) => format!("No empty planters in {label}"),
                    PlanterJob::Water => format!("Nothing in {label} needs water"),
                    PlanterJob::Fertilize => format!("Everything in {label} is fertilized"),
                })
            };
        };
        if !self.fly(world, r, pos, spot, SPEED, dt) {
            return Working(format!("Going to {label}"));
        }
        r.anim = RobotAnim::Work;
        r.scratch.work += dt;
        if r.scratch.work < 1.0 {
            return Working(format!("{verb} in {label}"));
        }
        r.scratch.work = 0.0;
        r.scratch.did = true;
        let pl = &mut self.dome_mut(id).unwrap().planters[slot as usize];
        match job {
            PlanterJob::Plant(_) => {
                let sp = seed.unwrap();
                pl.species = Some(sp);
                pl.growth = 0.0;
                r.inv.take(Item::Seed(sp), 1);
                self.discover(sp);
                self.fx(Fx::Sow, spot);
            }
            PlanterJob::Water => {
                pl.moisture = 1.0;
                self.fx(Fx::Water, spot);
            }
            PlanterJob::Fertilize => {
                pl.fertilizer = 600.0;
                r.inv.take(Item::Fertilizer, 1);
                self.fx(Fx::Fertilize, spot);
            }
        }
        Working(format!("{verb} in {label}"))
    }

    fn step_mine(
        &mut self,
        world: &mut World,
        r: &mut Robot,
        pos: &mut V2,
        ore: What,
        area: u8,
        dt: f32,
    ) -> Outcome {
        if r.inv.free_slots() == 0 {
            return Done;
        }
        let rect = match self.area_rect(area) {
            Ok(r) => r,
            Err(why) => return Blocked(why),
        };
        let label = area_name(self, area);
        let is_target = |m: Material| -> bool {
            m.is_ore()
                && match ore {
                    What::Anything => true,
                    What::Item(item) => items::drop_for(m).is_some_and(|(i, _)| i == item),
                }
        };
        // Keep working the same spot until it is exhausted.
        let still_there = r
            .scratch
            .focus
            .is_some_and(|(x, y)| is_target(world.material(x, y)));
        if !still_there {
            r.scratch.focus = None;
            let here = *pos;
            let mut best: Option<((i32, i32), f32)> = None;
            let mut y = rect.y0;
            while y < rect.y1 {
                let mut x = rect.x0;
                while x < rect.x1 {
                    if is_target(world.material(x, y)) {
                        let d = v2(x as f32, y as f32).distance(here);
                        if best.is_none_or(|(_, bd)| d < bd) {
                            best = Some(((x, y), d));
                        }
                    }
                    x += 2;
                }
                y += 2;
            }
            r.scratch.focus = best.map(|b| b.0);
        }
        let Some((tx, ty)) = r.scratch.focus else {
            return if r.inv.is_empty() {
                Blocked(format!("No {} left in {label}", ore.text(self, "ore")))
            } else {
                Done
            };
        };
        let target = v2(tx as f32 + 0.5, ty as f32 + 0.5);
        if pos.distance(target) > ARRIVE + 4.0 {
            self.fly(world, r, pos, target, SPEED, dt);
            return Working(format!("Going to mine {} in {label}", ore.text(self, "ore")));
        }
        r.anim = RobotAnim::Work;
        r.scratch.work += dt;
        if r.scratch.work >= 0.12 {
            r.scratch.work = 0.0;
            for (_, _, m) in world.dig(tx, ty, 3, 55) {
                let Some((item, per)) = items::drop_for(m) else {
                    continue;
                };
                let t = r.tally.entry(m as u8).or_insert(0.0);
                *t += 1.0 / per as f32;
                if *t >= 1.0 {
                    *t -= 1.0;
                    // Rubble is left behind; only ore is kept.
                    if m.is_ore() && r.inv.add(item, 1, 1.0) > 0 {
                        self.drop_item(*pos, item, 1);
                    }
                }
            }
        }
        Working(format!("Mining {} in {label}", ore.text(self, "ore")))
    }

    fn step_scavenge(
        &mut self,
        world: &mut World,
        r: &mut Robot,
        pos: &mut V2,
        area: u8,
        dt: f32,
    ) -> Outcome {
        if r.inv.free_slots() == 0 {
            return Done;
        }
        let rect = match self.area_rect(area) {
            Ok(r) => r,
            Err(why) => return Blocked(why),
        };
        let label = area_name(self, area);
        let here = *pos;
        let drop = self
            .ents
            .values()
            .filter(|e| matches!(e.kind, EntKind::Drop(_)) && rect.contains(e.pos))
            .min_by(|a, b| a.pos.distance(here).total_cmp(&b.pos.distance(here)))
            .map(|e| (e.id, e.pos));
        if let Some((id, at)) = drop {
            if !self.fly(world, r, pos, v2(at.x, at.y - 4.0), SPEED, dt) {
                return Working(format!("Collecting drops in {label}"));
            }
            if let Some(EntKind::Drop(d)) = self.ents.get(&id).map(|e| &e.kind) {
                let (item, count) = (d.item, d.count);
                let left = r.inv.add(item, count, 1.0);
                if left == 0 {
                    self.despawn(id);
                } else if let Some(EntKind::Drop(d)) = self.ents.get_mut(&id).map(|e| &mut e.kind) {
                    d.count = left;
                    self.touch(id);
                }
                self.fx(Fx::Pickup, at);
            }
            return Working(format!("Collecting drops in {label}"));
        }
        // No drops: pick whatever wild plants are ripe.
        match self.step_harvest(world, r, pos, What::Anything, Where::Area(area), dt) {
            Blocked(_) => Blocked(format!("Nothing to collect in {label}")),
            other => other,
        }
    }

    fn step_guard(&mut self, world: &mut World, r: &mut Robot, pos: &mut V2, area: u8, dt: f32) -> Outcome {
        let rect = match self.area_rect(area) {
            Ok(r) => r,
            Err(why) => return Blocked(why),
        };
        let label = area_name(self, area);
        let here = *pos;
        let threat = self
            .ents
            .values()
            .filter(|e| {
                matches!(&e.kind, EntKind::Creature(c) if c.kind.predator() && !matches!(c.state, creatures::State::Dying | creatures::State::Hidden))
                    && rect.grown(40).contains(e.pos)
            })
            .min_by(|a, b| a.pos.distance(here).total_cmp(&b.pos.distance(here)))
            .map(|e| v2(e.pos.x, e.pos.y - 5.0));
        match threat {
            Some(at) if at.distance(here) < 110.0 && line_of_sight(world, here, at, player_solid) => {
                r.anim = RobotAnim::Alarm;
                r.facing = if at.x >= here.x { 1 } else { -1 };
                if r.cooldown <= 0.0 {
                    r.cooldown = 0.8;
                    r.charge -= 1.0;
                    let dir = (at - here).normalized();
                    self.spawn(
                        here + dir * 6.0,
                        EntKind::Bolt(Bolt {
                            vel: dir * super::BOLT_SPEED,
                            damage: 10.0,
                            size: 0.8,
                            owner: None,
                            life: 1.0,
                            web: false,
                        }),
                    );
                    self.fx(Fx::BoltFire, here);
                }
                Working(format!("Defending {label}"))
            }
            Some(at) => {
                self.fly(world, r, pos, at, SPEED, dt);
                Working(format!("Chasing a predator near {label}"))
            }
            None => {
                // Patrol: drift between the ends of the area's top edge.
                let c = rect.center();
                let side = if (self.clock.time / 12.0) as i32 % 2 == 0 {
                    0.3
                } else {
                    0.7
                };
                let goal = v2(rect.x0 as f32 + (rect.x1 - rect.x0) as f32 * side, c.y);
                self.fly(world, r, pos, goal, SPEED * 0.5, dt);
                Working(format!("Guarding {label}"))
            }
        }
    }
}

#[derive(Clone, Copy)]
enum PlanterJob {
    Plant(What),
    Water,
    Fertilize,
}

/// Cargo taken out of a robot being packed up.
struct Ent_ {
    cargo: Vec<super::inventory::Stack>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colony::Machine;
    use crate::colony::actions::Action;
    use crate::colony::creatures::{Creature, CreatureKind};
    use crate::colony::domes::Dome;
    use crate::colony::plants::VOLTBLOOM;
    use crate::colony::testing::colony;

    /// A flat arena in open air: ground at y=400 from x=800 to 1400.
    fn arena(world: &mut World) {
        for y in 250..460 {
            for x in 760..1440 {
                world.set(
                    x,
                    y,
                    if y >= 400 {
                        Material::Stone
                    } else {
                        Material::Empty
                    },
                );
            }
        }
    }

    fn robot(colony: &mut Colony, at: V2, program: Vec<S>) -> Id {
        let program: Vec<Sentence> = program.into_iter().map(Sentence::from).collect();
        let mut r = Robot::new("Robot 1".into(), 0);
        r.blocked = vec![None; program.len()];
        r.program = program;
        colony.spawn(at, EntKind::Robot(r))
    }

    fn get(colony: &Colony, id: Id) -> (&Robot, V2) {
        let e = &colony.ents[&id];
        match &e.kind {
            EntKind::Robot(r) => (r, e.pos),
            _ => panic!(),
        }
    }

    fn run(colony: &mut Colony, world: &mut World, secs: f32) {
        for _ in 0..(secs * 30.0) as usize {
            colony.step_robots(world, 1.0 / 30.0);
        }
    }

    /// A sentence written the short way: UNTIL belongs to the first step.
    #[derive(Clone)]
    struct S {
        when: Option<Cond>,
        steps: Vec<Step>,
        until: Option<Cond>,
    }

    impl From<S> for Sentence {
        fn from(s: S) -> Sentence {
            let mut steps: Vec<Clause> = s.steps.into_iter().map(Clause::from).collect();
            if let Some(first) = steps.first_mut() {
                first.until = s.until;
            }
            Sentence { when: s.when, steps }
        }
    }

    impl S {
        fn text(&self, colony: &Colony) -> String {
            Sentence::from(self.clone()).text(colony)
        }
    }

    fn sentence(steps: Vec<Step>) -> S {
        S {
            when: None,
            steps,
            until: None,
        }
    }

    fn green(colony: &mut Colony, x: f32) -> Id {
        let mut d = Dome::new(DomeKind::Green, "Green Dome 2".into());
        for p in d.planters.iter_mut() {
            p.species = Some(VOLTBLOOM);
            p.growth = 1.0;
            p.moisture = 1.0;
        }
        colony.spawn(v2(x, 400.0), EntKind::Dome(d))
    }

    #[test]
    fn sentences_read_as_plain_english() {
        let (_, mut colony, _) = colony();
        let dome = green(&mut colony, 900.0);
        colony
            .area_op(AreaOp::Add {
                name: "North vein".into(),
                rect: Rect::new(0, 0, 100, 100),
            })
            .unwrap();
        let s = S {
            when: Some(Cond::Battery {
                below: true,
                percent: 20,
            }),
            steps: vec![Step::ChargeAt(Place::NearestPylon)],
            until: None,
        };
        assert_eq!(
            s.text(&colony),
            "WHEN battery is below 20%, charge at the nearest pylon."
        );
        let s = sentence(vec![
            Step::Harvest {
                what: What::Item(Item::Crop(VOLTBLOOM)),
                at: Where::Place(Place::Ent(dome)),
            },
            Step::Store {
                what: What::Anything,
                at: Place::NearestStorage,
            },
        ]);
        assert_eq!(
            s.text(&colony),
            "Harvest battery flower (voltbloom) in Green Dome 2, then store everything in the nearest storage."
        );
        let s = S {
            when: None,
            steps: vec![Step::Mine {
                ore: What::Item(Item::Iron),
                area: 0,
            }],
            until: Some(Cond::Inventory(InvState::Full)),
        };
        assert_eq!(
            s.text(&colony),
            "Mine iron in North vein UNTIL inventory is full."
        );
        let s = S {
            when: Some(Cond::Predator { nearby: true }),
            steps: vec![Step::FleeTo(Place::NearestDome)],
            until: None,
        };
        assert_eq!(
            s.text(&colony),
            "WHEN a predator is nearby, flee to the nearest dome."
        );
    }

    #[test]
    fn harvests_then_stores_and_reports_when_nothing_is_ripe() {
        let (mut world, mut colony, _) = colony();
        arena(&mut world);
        let dome = green(&mut colony, 1000.0);
        let chest = colony.spawn(
            v2(1200.0, 400.0),
            EntKind::Machine(Machine::new(MachineKind::Chest, "Chest 1".into())),
        );
        let id = robot(
            &mut colony,
            v2(1100.0, 380.0),
            vec![sentence(vec![
                Step::Harvest {
                    what: What::Anything,
                    at: Where::Place(Place::Ent(dome)),
                },
                Step::Store {
                    what: What::Anything,
                    at: Place::Ent(chest),
                },
            ])],
        );
        run(&mut colony, &mut world, 3.0);
        assert!(
            get(&colony, id).0.status.to_lowercase().contains("harvest"),
            "{}",
            get(&colony, id).0.status
        );
        run(&mut colony, &mut world, 40.0);
        let EntKind::Machine(m) = &colony.ents[&chest].kind else {
            panic!()
        };
        assert!(
            m.inv.count(Item::Crop(VOLTBLOOM)) >= 12,
            "four planters of three flowers each: {:?}",
            m.inv.count(Item::Crop(VOLTBLOOM))
        );
        assert!(
            colony
                .dome(dome)
                .unwrap()
                .1
                .planters
                .iter()
                .all(|p| p.growth < 1.0)
        );
        let (r, _) = get(&colony, id);
        assert!(r.inv.is_empty());
        assert_eq!(r.active, None);
        assert_eq!(r.blocked[0].as_deref(), Some("Nothing is ripe in Green Dome 2"));
        assert!(r.status.starts_with("Waiting"));
        // When the crop ripens again, it goes back to work by itself.
        colony.dome_mut(dome).unwrap().planters[0].growth = 1.0;
        run(&mut colony, &mut world, 20.0);
        let EntKind::Machine(m) = &colony.ents[&chest].kind else {
            panic!()
        };
        assert!(m.inv.count(Item::Crop(VOLTBLOOM)) >= 15);
    }

    #[test]
    fn higher_sentences_interrupt_and_low_battery_sends_it_to_charge() {
        let (mut world, mut colony, _) = colony();
        arena(&mut world);
        let mut pylon = Machine::new(MachineKind::Pylon, "Charging Station 1".into());
        pylon.inv.add(Item::Crop(VOLTBLOOM), 2, 1.0);
        let pylon = colony.spawn(v2(900.0, 400.0), EntKind::Machine(pylon));
        let id = robot(
            &mut colony,
            v2(1300.0, 380.0),
            vec![
                S {
                    when: Some(Cond::Battery {
                        below: true,
                        percent: 20,
                    }),
                    steps: vec![Step::ChargeAt(Place::Ent(pylon))],
                    until: None,
                },
                sentence(vec![Step::Wait { secs: 600 }]),
            ],
        );
        run(&mut colony, &mut world, 1.0);
        assert_eq!(get(&colony, id).0.active, Some(1), "waiting, battery is fine");
        if let EntKind::Robot(r) = &mut colony.ents.get_mut(&id).unwrap().kind {
            r.charge = 12.0;
        }
        run(&mut colony, &mut world, 1.0);
        let (r, _) = get(&colony, id);
        assert_eq!(r.active, Some(0));
        assert!(r.status.contains("charge"), "{}", r.status);
        run(&mut colony, &mut world, 20.0);
        let (r, pos) = get(&colony, id);
        assert!(r.charge > 95.0, "charged to {}", r.charge);
        assert!(pos.distance(v2(900.0, 386.0)) < 20.0);
        assert_eq!(r.active, Some(1), "back to its routine");
        // An empty pylon is reported.
        if let EntKind::Machine(m) = &mut colony.ents.get_mut(&pylon).unwrap().kind {
            m.store = 0.0;
            m.inv = Inventory::new(20);
        }
        if let EntKind::Robot(r) = &mut colony.ents.get_mut(&id).unwrap().kind {
            r.charge = 10.0;
        }
        run(&mut colony, &mut world, 2.0);
        let (r, _) = get(&colony, id);
        assert!(r.blocked[0].as_deref().unwrap().contains("no charge"));
        // With nothing left it stops dead.
        if let EntKind::Robot(r) = &mut colony.ents.get_mut(&id).unwrap().kind {
            r.charge = 0.01;
        }
        run(&mut colony, &mut world, 20.0);
        assert_eq!(get(&colony, id).0.anim, RobotAnim::Dead);
    }

    #[test]
    fn mines_ore_in_an_area_until_full_then_returns() {
        let (mut world, mut colony, _) = colony();
        arena(&mut world);
        // A vein under the floor, behind rock.
        for y in 410..430 {
            for x in 1000..1040 {
                world.set(x, y, Material::IronOre);
            }
        }
        colony
            .area_op(AreaOp::Add {
                name: "Vein".into(),
                rect: Rect::new(980, 400, 1060, 440),
            })
            .unwrap();
        let chest = colony.spawn(
            v2(1300.0, 400.0),
            EntKind::Machine(Machine::new(MachineKind::Chest, "Chest 1".into())),
        );
        let id = robot(
            &mut colony,
            v2(900.0, 380.0),
            vec![S {
                when: None,
                steps: vec![
                    Step::Mine {
                        ore: What::Item(Item::Iron),
                        area: 0,
                    },
                    Step::Store {
                        what: What::Anything,
                        at: Place::Ent(chest),
                    },
                ],
                until: None,
            }],
        );
        run(&mut colony, &mut world, 240.0);
        let EntKind::Machine(m) = &colony.ents[&chest].kind else {
            panic!()
        };
        assert!(m.inv.count(Item::Iron) > 100, "mined {}", m.inv.count(Item::Iron));
        let left = (410..430)
            .flat_map(|y| (1000..1040).map(move |x| (x, y)))
            .filter(|&(x, y)| world.material(x, y) == Material::IronOre)
            .count();
        assert!(left < 100, "{left} ore cells left");
        let (r, _) = get(&colony, id);
        assert!(
            r.blocked[0]
                .as_deref()
                .is_some_and(|b| b.contains("No iron left in Vein")),
            "{:?} {}",
            r.blocked,
            r.status
        );
    }

    #[test]
    fn flees_from_predators_and_guards_shoot_them() {
        let (mut world, mut colony, _) = colony();
        arena(&mut world);
        let dome = green(&mut colony, 900.0);
        colony
            .area_op(AreaOp::Add {
                name: "Yard".into(),
                rect: Rect::new(1100, 300, 1400, 400),
            })
            .unwrap();
        let runner = robot(
            &mut colony,
            v2(1200.0, 380.0),
            vec![
                S {
                    when: Some(Cond::Predator { nearby: true }),
                    steps: vec![Step::FleeTo(Place::Ent(dome))],
                    until: None,
                },
                sentence(vec![Step::Wait { secs: 999 }]),
            ],
        );
        let guard = robot(
            &mut colony,
            v2(1150.0, 360.0),
            vec![sentence(vec![Step::Guard { area: 0 }])],
        );
        run(&mut colony, &mut world, 1.0);
        assert_eq!(get(&colony, runner).0.active, Some(1));
        assert!(get(&colony, guard).0.status.contains("Guarding"));
        let skitter = colony.spawn(
            v2(1250.0, 399.0),
            EntKind::Creature(Creature::new(CreatureKind::Skitter)),
        );
        run(&mut colony, &mut world, 0.5);
        let (r, pos) = get(&colony, runner);
        assert_eq!(r.active, Some(0), "{:?} {} {:?}", r.blocked, r.status, pos);
        assert!(r.status.starts_with("Fleeing"), "{}", r.status);
        assert!(pos.x < 1180.0, "fled toward the dome: {pos:?}");
        run(&mut colony, &mut world, 4.0);
        assert!(
            get(&colony, runner).1.distance(v2(900.0, 384.0)) < 20.0,
            "all the way"
        );
        for _ in 0..(8.0 * 30.0) as usize {
            colony.step_robots(&mut world, 1.0 / 30.0);
            colony.step_bolts(&mut world, 1.0 / 30.0);
        }
        let dead = !matches!(colony.ents.get(&skitter).map(|e| &e.kind), Some(EntKind::Creature(c)) if c.state != creatures::State::Dying);
        assert!(dead, "the guard shot it");
    }

    #[test]
    fn stock_conditions_drive_logistics() {
        let (mut world, mut colony, _) = colony();
        arena(&mut world);
        let mut store = Machine::new(MachineKind::Chest, "Seed store".into());
        store.inv.add(Item::Crop(VOLTBLOOM), 30, 1.0);
        let store = colony.spawn(v2(900.0, 400.0), EntKind::Machine(store));
        let pylon = colony.spawn(
            v2(1300.0, 400.0),
            EntKind::Machine(Machine::new(MachineKind::Pylon, "Pylon A".into())),
        );
        // "WHEN Pylon A has less than 5 flowers, take flowers from the store, then store them in Pylon A."
        let id = robot(
            &mut colony,
            v2(1100.0, 380.0),
            vec![S {
                when: Some(Cond::Stock {
                    place: Place::Ent(pylon),
                    less: true,
                    count: 5,
                    item: Item::Crop(VOLTBLOOM),
                }),
                steps: vec![
                    Step::Take {
                        what: What::Item(Item::Crop(VOLTBLOOM)),
                        from: Place::Ent(store),
                    },
                    Step::Store {
                        what: What::Item(Item::Crop(VOLTBLOOM)),
                        at: Place::Ent(pylon),
                    },
                ],
                until: None,
            }],
        );
        run(&mut colony, &mut world, 30.0);
        let EntKind::Machine(m) = &colony.ents[&pylon].kind else {
            panic!()
        };
        assert_eq!(m.inv.count(Item::Crop(VOLTBLOOM)), 30, "one stack carried over");
        let (r, _) = get(&colony, id);
        assert_eq!(r.active, None, "the condition no longer holds");
        assert!(r.status.starts_with("Idle"), "{}", r.status);
    }

    #[test]
    fn plants_waters_and_fertilizes() {
        let (mut world, mut colony, _) = colony();
        arena(&mut world);
        let dome = colony.spawn(
            v2(1000.0, 400.0),
            EntKind::Dome(Dome::new(DomeKind::Green, "Green Dome 1".into())),
        );
        let id = robot(
            &mut colony,
            v2(1100.0, 380.0),
            vec![
                sentence(vec![Step::Plant {
                    seed: What::Anything,
                    at: Place::Ent(dome),
                }]),
                sentence(vec![Step::Water(Place::Ent(dome))]),
                sentence(vec![Step::Fertilize(Place::Ent(dome))]),
            ],
        );
        run(&mut colony, &mut world, 1.0);
        assert!(
            get(&colony, id).0.blocked[0]
                .as_deref()
                .unwrap()
                .contains("No seeds on board")
        );
        if let EntKind::Robot(r) = &mut colony.ents.get_mut(&id).unwrap().kind {
            r.inv.add(Item::Seed(2), 3, 1.0);
            r.inv.add(Item::Seed(15), 2, 1.0);
            r.inv.add(Item::Fertilizer, 2, 1.0);
        }
        run(&mut colony, &mut world, 40.0);
        let d = colony.dome(dome).unwrap().1;
        let planted = d.planters.iter().filter(|p| p.species == Some(2)).count();
        assert_eq!(planted, 3, "glowcap seeds don't go in a Green Dome");
        assert!(
            d.planters
                .iter()
                .filter(|p| p.species.is_some())
                .all(|p| p.moisture == 1.0),
            "watered"
        );
        assert_eq!(
            d.planters.iter().filter(|p| p.fertilizer > 0.0).count(),
            2,
            "two bags of fertilizer"
        );
        let (r, _) = get(&colony, id);
        assert_eq!(r.inv.count(Item::Seed(15)), 2);
        assert_eq!(r.inv.count(Item::Fertilizer), 0);
    }

    #[test]
    fn paths_go_around_walls_and_drill_when_sealed_in() {
        let (mut world, mut colony, _) = colony();
        arena(&mut world);
        // A wall with a gap at the top.
        for y in 300..400 {
            for x in 1090..1100 {
                world.set(x, y, Material::Stone);
            }
        }
        let path = find_path(&world, v2(1000.0, 380.0), v2(1200.0, 380.0)).expect("a way over the wall");
        assert!(path.iter().any(|p| p.y < 300.0), "goes over the top");
        let chest = colony.spawn(
            v2(1200.0, 400.0),
            EntKind::Machine(Machine::new(MachineKind::Chest, "Chest 1".into())),
        );
        let id = robot(
            &mut colony,
            v2(1000.0, 380.0),
            vec![sentence(vec![Step::GoTo(Place::Ent(chest))])],
        );
        run(&mut colony, &mut world, 12.0);
        assert!(get(&colony, id).1.distance(v2(1200.0, 386.0)) < 20.0);
        // Seal the robot in a box: it drills out.
        let id2 = robot(
            &mut colony,
            v2(860.0, 380.0),
            vec![sentence(vec![Step::GoTo(Place::Ent(chest))])],
        );
        for y in 360..400 {
            for x in 840..880 {
                if !(850..870).contains(&x) || !(370..392).contains(&y) {
                    world.set(x, y, Material::Dirt);
                }
            }
        }
        run(&mut colony, &mut world, 40.0);
        assert!(
            get(&colony, id2).1.distance(v2(1200.0, 386.0)) < 20.0,
            "{:?}",
            get(&colony, id2).1
        );
    }

    #[test]
    fn robots_are_deployed_programmed_and_packed_up_by_players() {
        let (mut world, mut colony, key) = colony();
        let p = colony.players.get_mut(&key).unwrap();
        p.inv.slots[0] = Some(crate::colony::inventory::Stack::new(Item::RobotKit, 1));
        p.selected = 0;
        let at = p.center() + v2(20.0, -10.0);
        colony.apply(&mut world, key, Action::Use { at }).unwrap();
        let id = colony
            .ents
            .values()
            .find(|e| matches!(e.kind, EntKind::Robot(_)))
            .unwrap()
            .id;
        let program: Vec<Sentence> = vec![sentence(vec![Step::Wait { secs: 5 }]).into()];
        colony
            .apply(
                &mut world,
                key,
                Action::Robot {
                    id,
                    op: RobotOp::SetProgram(program.clone()),
                },
            )
            .unwrap();
        assert_eq!(get(&colony, id).0.program, program);
        assert_eq!(
            colony.apply(
                &mut world,
                key,
                Action::Robot {
                    id,
                    op: RobotOp::SetProgram(vec![sentence(vec![]).into()])
                }
            ),
            Err("Each sentence needs one to four steps")
        );
        colony
            .apply(
                &mut world,
                key,
                Action::Robot {
                    id,
                    op: RobotOp::Pause(true),
                },
            )
            .unwrap();
        colony.step_robots(&mut world, 0.1);
        assert_eq!(get(&colony, id).0.status, "Paused");
        // It can be opened like a chest.
        colony
            .apply(
                &mut world,
                key,
                Action::Inv(crate::colony::actions::InvOp::DepositAll(
                    crate::colony::actions::Container::Machine(id),
                )),
            )
            .unwrap();
        colony
            .apply(
                &mut world,
                key,
                Action::Robot {
                    id,
                    op: RobotOp::PickUp,
                },
            )
            .unwrap();
        assert!(!colony.ents.contains_key(&id));
        assert_eq!(colony.players[&key].inv.count(Item::RobotKit), 1);
        // Areas.
        colony
            .apply(
                &mut world,
                key,
                Action::Area(AreaOp::Add {
                    name: "".into(),
                    rect: Rect::new(10, 10, 60, 60),
                }),
            )
            .unwrap();
        assert_eq!(colony.areas[0].name, "Area 1");
        assert_eq!(
            colony.apply(
                &mut world,
                key,
                Action::Area(AreaOp::Add {
                    name: "x".into(),
                    rect: Rect::new(0, 0, 2, 2)
                })
            ),
            Err("Drag a bigger box")
        );
        colony
            .apply(&mut world, key, Action::Area(AreaOp::Remove(0)))
            .unwrap();
        assert!(colony.areas.is_empty());
    }
}
