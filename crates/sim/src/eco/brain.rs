//! Modular utility AI.
//!
//! A species is given a set of behaviour modules (with weights) by its
//! genome. Every think, each module scores how much it wants to run from
//! the creature's needs and what it perceives; the best one runs and
//! produces an [`Intent`]: where to go, how, and what to do on arrival.
//! A shared steering layer turns intents into movement using pathfinding
//! that understands the species' locomotion.

use std::f32::consts::{FRAC_PI_2, PI};

use super::biome::Tree;
use super::creature::{Carcass, Creature, Egg, key_carcass, key_creature, key_plant};
use super::flora::{FloraSpecies, Plant};
use super::genome::{FleeStyle, Habitat, HuntStyle, Locomotion, Role, Species};
use super::math::{V2, v2};
use super::nav::{NavGrid, TILE, line_of_sight};
use super::physics::{self, Mode, MoveCmd};
use crate::material::{Kind, Material};
use crate::rng::Rng;
use crate::world::World;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BehaviorKind {
    Escape,
    Flee,
    Freeze,
    Shelter,
    Graze,
    Hunt,
    Ambush,
    SetTrap,
    Scavenge,
    Rest,
    BuildNest,
    Reproduce,
    Flock,
    Mob,
    Patrol,
    Investigate,
    Wander,
}

impl BehaviorKind {
    pub fn label(self) -> &'static str {
        match self {
            BehaviorKind::Escape => "Escape hazard",
            BehaviorKind::Flee => "Flee",
            BehaviorKind::Freeze => "Freeze",
            BehaviorKind::Shelter => "Shelter",
            BehaviorKind::Graze => "Graze",
            BehaviorKind::Hunt => "Hunt",
            BehaviorKind::Ambush => "Ambush",
            BehaviorKind::SetTrap => "Set trap",
            BehaviorKind::Scavenge => "Feed on carcass",
            BehaviorKind::Rest => "Rest",
            BehaviorKind::BuildNest => "Build shelter",
            BehaviorKind::Reproduce => "Reproduce",
            BehaviorKind::Flock => "Stay with group",
            BehaviorKind::Mob => "Mob attacker",
            BehaviorKind::Patrol => "Patrol territory",
            BehaviorKind::Investigate => "Investigate",
            BehaviorKind::Wander => "Wander",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum Action {
    #[default]
    None,
    Attack(u32),
    Spit(V2),
    EatPlant(u32),
    EatCarcass(u32),
    EatEgg(u32),
    Leap(V2),
    Dig(V2),
    PlaceNest,
    SpinWeb,
    Ink,
    PlayDead,
    Mate(u32),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Intent {
    pub goal: Option<V2>,
    /// Fraction of top speed; above 1 is a sprint.
    pub speed: f32,
    /// Steer straight at the goal without pathfinding.
    pub direct: bool,
    pub fly: bool,
    pub dig: bool,
    pub sneak: bool,
    pub action: Action,
}

/// A read-only view of another creature for perception.
#[derive(Clone, Copy, Debug)]
pub struct Snap {
    pub id: u32,
    pub species: usize,
    pub pos: V2,
    pub vel: V2,
    pub size: f32,
    pub health: f32,
    pub conspicuous: f32,
    pub burrowed: bool,
    pub webbed: bool,
    pub fleeing: bool,
    pub alarm: bool,
    pub mature: bool,
    pub ready: bool,
    pub playing_dead: bool,
    pub luring: bool,
    pub noise: f32,
    pub hunting: Option<u32>,
    pub juvenile: bool,
    pub home: Option<V2>,
}

pub struct Ctx<'a> {
    pub species: &'a [Species],
    pub flora: &'a [FloraSpecies],
    pub plants: &'a [Plant],
    pub carcasses: &'a [Carcass],
    pub eggs: &'a [Egg],
    pub snaps: &'a [Snap],
    pub trees: &'a [Tree],
    pub surface: &'a [i32],
    pub daylight: f32,
    pub dusk: f32,
    pub time: f32,
    pub gravity: f32,
    pub open_sky: bool,
    pub cave_ambient: f32,
    pub pop: [usize; 2],
    pub cap: [usize; 2],
}

impl Ctx<'_> {
    pub fn snap(&self, id: u32) -> Option<&Snap> {
        self.snaps
            .binary_search_by_key(&id, |s| s.id)
            .ok()
            .map(|i| &self.snaps[i])
    }

    pub fn plant(&self, id: u32) -> Option<&Plant> {
        self.plants.iter().find(|p| p.id == id && !p.dead)
    }

    /// Ambient light at a position, 0..1.
    pub fn light_at(&self, p: V2) -> f32 {
        let x = (p.x as usize).min(self.surface.len() - 1);
        if self.open_sky && p.y < self.surface[x] as f32 + 8.0 {
            0.12 + 0.88 * self.daylight
        } else {
            self.cave_ambient + 0.1
        }
    }
}

// ---------------------------------------------------------------------------
// Perception

fn relation(me: &Species, other: &Species) -> (bool, bool) {
    // (is threat to me, is prey for me)
    let threat = me.role == Role::Prey && other.role == Role::Predator;
    let prey = me.role == Role::Predator && other.role == Role::Prey;
    (threat, prey)
}

pub fn perceive(c: &Creature, sp: &Species, ctx: &Ctx, world: &World) -> super::creature::Perception {
    let mut p = super::creature::Perception::default();
    let light = if sp.traits.night_vision {
        1.0
    } else {
        ctx.light_at(c.pos)
    };
    p.light = light;
    let alert = if c.sleeping { 0.35 } else { 1.0 };
    let sight = sp.stats.sight * (0.35 + 0.65 * light) * alert;
    let hearing = sp.stats.hearing * alert;
    let me_scale = c.scale(sp);
    let mut best_prey: Option<(u32, V2, f32, f32)> = None;
    let mut kin_sum = V2::ZERO;
    let mut kin_n = 0u8;
    let mut best_mate: Option<(u32, V2, f32)> = None;
    let mut threat_d = f32::MAX;
    for s in ctx.snaps {
        if s.id == c.id {
            continue;
        }
        let d = s.pos.dist(c.pos);
        // Receptive kin call out to each other over long distances.
        if s.species == sp.idx && s.ready && d < 260.0 && best_mate.is_none_or(|m| d < m.2) {
            best_mate = Some((s.id, s.pos, d));
        }
        if d > sight.max(hearing).max(110.0) {
            continue;
        }
        let other = &ctx.species[s.species];
        let (is_threat, is_prey) = relation(sp, other);
        let glow = if other.traits.glow {
            1.0 + (1.0 - light)
        } else {
            1.0
        };
        let vis_range = sight * s.conspicuous * glow;
        let tremor = sp.traits.tremor_sense && d < hearing * 1.2 && s.vel.len() > 3.0;
        let seen = if s.burrowed {
            tremor
        } else {
            d < vis_range && line_of_sight(world, c.pos, s.pos)
        };
        let heard = d < hearing * s.noise || tremor;
        if s.species == sp.idx {
            if d < 70.0 {
                kin_sum += s.pos;
                kin_n += 1;
            }
            if s.fleeing && s.alarm && d < hearing * 2.0 && sp.role == Role::Prey {
                p.alarm = Some(s.pos - s.vel.norm() * 20.0);
            }
            if sp.traits.territorial
                && s.mature
                && let Some(home) = c.brain.home
                && s.pos.dist(home) < 60.0
                && seen
            {
                p.intruder = Some((s.id, s.pos));
            }
            continue;
        }
        if is_threat {
            if s.luring && !seen && d < sight * 1.4 {
                p.lure = Some((s.id, s.pos + v2(s.vel.x.signum() * 4.0, -3.0)));
            }
            if (seen || heard) && !s.playing_dead {
                p.threats += 1;
                if d < threat_d {
                    threat_d = d;
                    p.threat = Some((s.id, s.pos, d));
                }
            }
            // Kin under attack.
            if let Some(t) = s.hunting
                && ctx
                    .snap(t)
                    .is_some_and(|ts| ts.species == sp.idx && ts.pos.dist(c.pos) < 50.0)
                && seen
            {
                p.kin_attacker = Some((s.id, s.pos));
            }
        } else if is_prey {
            if s.webbed && d < 110.0 && sp.hunts(HuntStyle::WebTrap) {
                p.webbed_prey = Some((s.id, s.pos));
            }
            if !(seen || (heard && d < hearing)) || s.playing_dead || c.brain.ignored(key_creature(s.id)) {
                if heard && !s.burrowed {
                    p.noise = Some(s.pos);
                }
                continue;
            }
            let mut score = 1.0 / (d + 10.0);
            score *= 1.6 - s.health * 0.6;
            if s.juvenile {
                score *= 1.3;
            }
            if s.webbed {
                score *= 3.0;
            }
            // Packs focus one target.
            if sp.hunts(HuntStyle::Pack)
                && ctx
                    .snaps
                    .iter()
                    .any(|k| k.species == sp.idx && k.id != c.id && k.hunting == Some(s.id))
            {
                score *= 2.5;
            }
            // Don't bother with prey much bigger than us unless we're desperate.
            if s.size > sp.stats.size * me_scale * 1.6 && c.satiety > 0.25 && !sp.traits.venom {
                score *= 0.3;
            }
            if c.brain.target == Some(s.id) {
                score *= 1.5;
            }
            if best_prey.is_none_or(|b| score > b.3) {
                best_prey = Some((s.id, s.pos, d, score));
            }
        }
    }
    p.prey = best_prey.map(|(id, pos, d, _)| (id, pos, d));
    if kin_n > 0 {
        p.kin_centre = Some(kin_sum / kin_n as f32);
    }
    p.kin_count = kin_n;
    p.mate = best_mate.map(|(id, pos, _)| (id, pos));

    // Food.
    if sp.role == Role::Prey {
        let mut best: Option<(u32, V2, f32)> = None;
        for pl in ctx.plants {
            if pl.dead || !sp.food_flora.contains(&pl.flora) {
                continue;
            }
            let fl = &ctx.flora[pl.flora];
            if pl.food(fl) < 0.05 || c.brain.ignored(key_plant(pl.id)) {
                continue;
            }
            let fp = food_point(sp, pl, fl);
            let d = fp.dist(c.pos);
            if d > sight * 1.3 + 30.0 {
                continue;
            }
            // Avoid plants next to a known threat.
            if let Some((_, tp, _)) = p.threat
                && tp.dist(fp) < 30.0
            {
                continue;
            }
            let s = pl.food(fl) / (d + 15.0);
            if best.is_none_or(|b| s > b.2) {
                best = Some((pl.id, fp, s));
            }
        }
        p.plant = best.map(|(id, fp, _)| (id, fp));
    } else {
        let range = sp.stats.sight * if sp.traits.scavenger { 1.6 } else { 0.9 };
        p.carcass = ctx
            .carcasses
            .iter()
            .filter(|k| {
                k.meat > 0.3
                    && !c.brain.ignored(key_carcass(k.id))
                    && (k.species != sp.idx || c.satiety < 0.15)
            })
            .map(|k| (k.id, k.pos, k.pos.dist(c.pos)))
            .filter(|k| k.2 < range)
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(id, pos, _)| (id, pos));
        p.egg = ctx
            .eggs
            .iter()
            .filter(|e| e.species != sp.idx)
            .map(|e| (e.id, e.pos, e.pos.dist(c.pos)))
            .filter(|e| e.2 < sight * 0.6)
            .min_by(|a, b| a.2.total_cmp(&b.2))
            .map(|(id, pos, _)| (id, pos));
    }
    p
}

/// Where a creature of this species reaches a plant to eat.
pub fn food_point(sp: &Species, pl: &Plant, fl: &FloraSpecies) -> V2 {
    if sp.habitat == Habitat::Burrower {
        // Gnaw the roots from below.
        pl.base() + v2(0.0, 3.0)
    } else {
        pl.food_point(fl)
    }
}

// ---------------------------------------------------------------------------
// Scoring

fn hazard(c: &Creature, sp: &Species, world: &World) -> f32 {
    let m = world.material(c.pos.x as i32, c.pos.y as i32);
    let mut h: f32 = 0.0;
    if (m == Material::Lava && !sp.traits.lava_proof) || (m == Material::Acid && !sp.traits.acid_proof) {
        h = 1.6;
    }
    if c.status.burning > 0.0 {
        h = h.max(1.2);
    }
    if c.breath < 0.6 {
        h = h.max(1.0 + (0.6 - c.breath));
    }
    if sp.traits.gills && !c.in_liquid {
        h = h.max(1.3);
    }
    h
}

fn score(kind: BehaviorKind, c: &Creature, sp: &Species, ctx: &Ctx, world: &World) -> f32 {
    let p = &c.brain.perception;
    let pers = &sp.personality;
    let hunger = 1.0 - c.satiety;
    let alert = sp.activity.alertness(ctx.daylight, ctx.dusk);
    let threat_close = |k: f32| {
        p.threat
            .map(|(_, _, d)| (1.0 - d / (sp.stats.flee_distance * k)).clamp(0.0, 1.0))
            .unwrap_or(0.0)
    };
    let hurt_recently = c.hurt_timer > 0.0;
    match kind {
        BehaviorKind::Escape => hazard(c, sp, world),
        BehaviorKind::Flee => {
            if c.playing_dead > 0.0 {
                // Real danger (drowning, fire) still wins.
                return 1.1;
            }
            if sp.role == Role::Predator {
                return if c.health < c.max_health(sp) * 0.3 && hurt_recently {
                    1.2
                } else {
                    0.0
                };
            }
            let mut s = threat_close(1.5);
            if sp.traits.camouflage
                && p.threat
                    .is_some_and(|(_, _, d)| d > sp.stats.flee_distance * 0.45)
                && c.webbed <= 0.0
            {
                s *= 0.4;
            }
            if p.alarm.is_some() {
                s = s.max(0.55);
            }
            if hurt_recently {
                s = s.max(1.0);
            }
            s * (1.1 - pers.bravery * 0.5) * (1.0 + c.fear * 0.3)
        }
        BehaviorKind::Freeze => {
            if p.threat.is_some_and(|(_, _, d)| {
                d > sp.stats.flee_distance * 0.45 && d < sp.stats.flee_distance * 2.0
            }) {
                0.85
            } else {
                0.0
            }
        }
        BehaviorKind::Shelter => match c.brain.home {
            Some(h) if !c.brain.nest_plan.is_empty() && c.brain.nest_cells.is_empty() => {
                let _ = h;
                0.0
            }
            Some(h) => {
                let danger = threat_close(2.5).max(if p.alarm.is_some() { 0.5 } else { 0.0 });
                let near = (1.0 - h.dist(c.pos) / 150.0).clamp(0.0, 1.0);
                let sleepy = (1.0 - alert) * 0.7;
                danger.max(sleepy) * (0.5 + near * 0.6)
            }
            None => 0.0,
        },
        BehaviorKind::Graze => {
            if sp.role != Role::Prey || c.satiety > 0.95 {
                return 0.0;
            }
            let food = if p.plant.is_some() || p.lure.is_some() {
                1.0
            } else {
                0.45
            };
            hunger.powf(0.8) * food * (0.6 + alert * 0.4) * (1.0 - threat_close(1.2) * 0.8) + 0.05
        }
        BehaviorKind::Hunt => {
            if sp.role != Role::Predator {
                return 0.0;
            }
            if let Some((_, _)) = p.webbed_prey {
                return 1.3;
            }
            let Some((_, _, d)) = p.prey else {
                // Nothing in sight: hungry hunters go looking.
                let ambusher = sp.hunts(HuntStyle::Ambush)
                    || sp.hunts(HuntStyle::Lure)
                    || sp.hunts(HuntStyle::WebTrap)
                    || sp.hunts(HuntStyle::PitTrap);
                return if hunger > 0.4 && (!ambusher || c.satiety < 0.25) {
                    hunger * 0.75
                } else {
                    0.0
                };
            };
            // Well-fed predators only take what walks right up to them.
            if c.satiety > 0.9 || (c.satiety > 0.62 && d > 20.0) {
                return 0.0;
            }
            let ambusher =
                sp.hunts(HuntStyle::Ambush) || sp.hunts(HuntStyle::Lure) || sp.hunts(HuntStyle::PitTrap);
            let reach = if ambusher && !sp.hunts(HuntStyle::Chase) {
                if d < 18.0 + sp.stats.size * 3.0 { 1.3 } else { 0.15 }
            } else {
                1.1
            };
            let desperate = if c.satiety < 0.2 { 1.3 } else { 1.0 };
            hunger.powf(0.7) * reach * (1.0 - c.brain.aversion * 0.7) * desperate * (0.5 + alert * 0.5)
        }
        BehaviorKind::Ambush => {
            if c.satiety > 0.92 {
                return 0.0;
            }
            (0.3 + hunger * 0.5) * (0.6 + alert * 0.4)
        }
        BehaviorKind::SetTrap => {
            if c.satiety > 0.95 {
                return 0.1;
            }
            let have_trap = c.brain.trap.is_some();
            if have_trap { 0.35 + hunger * 0.35 } else { 0.55 }
        }
        BehaviorKind::Scavenge => {
            if p.carcass.is_some() && c.satiety < 0.9 {
                0.35 + hunger
            } else if p.egg.is_some() && c.satiety < 0.8 {
                0.3 + hunger * 0.7
            } else {
                0.0
            }
        }
        BehaviorKind::Rest => {
            let tired = (1.0 - c.energy).powf(1.5);
            // Hunger keeps animals up at night.
            let sleepy = (1.0 - alert) * 0.75 * (1.0 - hunger * hunger * 1.4).max(0.0);
            let unsafe_ = threat_close(1.5);
            (tired.max(sleepy) * (1.0 - unsafe_)).max(if c.sleeping { 0.25 } else { 0.0 })
        }
        BehaviorKind::BuildNest => {
            if c.satiety < 0.3 || p.threat.is_some() || !c.mature(sp) {
                return 0.0;
            }
            if c.brain.home.is_none() || !c.brain.nest_plan.is_empty() {
                0.5
            } else {
                0.0
            }
        }
        BehaviorKind::Reproduce => {
            let ready = c.mature(sp)
                && c.satiety > 0.6
                && c.energy > 0.3
                && c.mate_cd <= 0.0
                && c.pregnant.is_none()
                && ctx.pop[sp.role as usize] < ctx.cap[sp.role as usize];
            if !ready {
                return 0.0;
            }
            0.45 + if p.mate.is_some() { 0.35 } else { 0.0 }
        }
        BehaviorKind::Flock => match p.kin_centre {
            Some(k) if pers.sociality > 0.3 => {
                let d = k.dist(c.pos);
                (pers.sociality * 0.5 * ((d - 15.0) / 50.0).clamp(0.0, 1.0)).min(0.5)
            }
            _ => 0.0,
        },
        BehaviorKind::Mob => {
            if p.kin_attacker.is_some()
                && p.kin_count >= 2
                && c.health > c.max_health(sp) * 0.5
                && c.mature(sp)
            {
                0.6 + pers.aggression * 0.5
            } else {
                0.0
            }
        }
        BehaviorKind::Patrol => {
            if p.intruder.is_some() {
                0.5 + pers.aggression * 0.4
            } else if c.brain.home.is_some() {
                0.12
            } else {
                0.0
            }
        }
        BehaviorKind::Investigate => {
            if p.noise.is_some() || (p.lure.is_some() && sp.role == Role::Prey) {
                pers.curiosity
                    * if sp.role == Role::Predator {
                        0.4 + hunger * 0.5
                    } else {
                        0.5
                    }
            } else {
                0.0
            }
        }
        BehaviorKind::Wander => 0.12 + pers.curiosity * 0.08,
    }
}

/// Re-evaluates perception and picks a behaviour.
pub fn think(c: &mut Creature, sp: &Species, ctx: &Ctx, world: &World) {
    c.brain.perception = perceive(c, sp, ctx, world);
    if let Some((_, pos, _)) = c.brain.perception.prey {
        c.brain.last_prey = Some((pos, ctx.time));
    }
    let mut scores: Vec<(BehaviorKind, f32)> = sp
        .behaviors
        .iter()
        .map(|&(k, w)| {
            let mut s = score(k, c, sp, ctx, world) * w;
            if k == c.brain.current {
                s += 0.08; // hysteresis
            }
            (k, s)
        })
        .collect();
    scores.sort_by(|a, b| b.1.total_cmp(&a.1));
    let best = scores[0].0;
    if best != c.brain.current {
        enter(c, best);
    }
    c.brain.scores = scores;
    // Fear follows threats.
    if c.brain.perception.threat.is_some() {
        c.fear = (c.fear + 0.2).min(1.0);
    }
}

fn enter(c: &mut Creature, kind: BehaviorKind) {
    let b = &mut c.brain;
    let keep_spot = matches!(
        (b.current, kind),
        (BehaviorKind::Ambush, BehaviorKind::Hunt) | (BehaviorKind::Hunt, BehaviorKind::Ambush)
    );
    b.current = kind;
    b.stage = 0;
    b.timer = 0.0;
    b.path.clear();
    b.repath = 0.0;
    b.stuck = 0.0;
    if !keep_spot && !matches!(kind, BehaviorKind::SetTrap | BehaviorKind::BuildNest) {
        b.spot = None;
    }
    if kind != BehaviorKind::Hunt {
        b.target = None;
    }
    c.sleeping = false;
    c.luring = false;
}

// ---------------------------------------------------------------------------
// Acting

fn random_open_near(
    world: &World,
    rng: &mut Rng,
    around: V2,
    r: i32,
    pred: impl Fn(i32, i32) -> bool,
) -> Option<V2> {
    for _ in 0..30 {
        let x = around.x as i32 + rng.int(-r, r);
        let y = around.y as i32 + rng.int(-r, r);
        if world.in_bounds(x, y) && pred(x, y) {
            return Some(v2(x as f32 + 0.5, y as f32 + 0.5));
        }
    }
    None
}

/// Somewhere new to go that suits how this species lives.
pub fn wander_target(c: &Creature, sp: &Species, world: &World, ctx: &Ctx, rng: &mut Rng) -> V2 {
    let open = |x: i32, y: i32| world.material(x, y) == Material::Empty;
    let liquid_ok = |x: i32, y: i32| {
        let m = world.material(x, y);
        m.kind() == Kind::Liquid && sp.swims_in(m)
    };
    let home = c
        .brain
        .home
        .filter(|_| sp.traits.territorial || sp.traits.build_shelter);
    let around = home.unwrap_or(c.pos);
    let r = if home.is_some() { 50 } else { 80 };
    let found = match sp.habitat {
        Habitat::Aquatic => random_open_near(world, rng, around, r, liquid_ok),
        Habitat::LavaDweller if sp.loco == Locomotion::Swim => {
            random_open_near(world, rng, around, r, liquid_ok)
        }
        Habitat::Aerial => random_open_near(world, rng, around, r, |x, y| {
            open(x, y) && (y as usize) < ctx.surface[x as usize].max(10) as usize + 5 && open(x, y + 3)
        }),
        Habitat::Burrower => random_open_near(world, rng, around, r, |x, y| {
            world.material(x, y).is_diggable() && y < ctx.surface[x as usize] + 40
        }),
        Habitat::Arboreal if !ctx.trees.is_empty() && rng.prob(0.7) => {
            let t = rng.pick(ctx.trees);
            Some(v2(
                t.x as f32 + rng.range(-6.0, 6.0),
                (t.ground - t.height) as f32 + rng.range(-4.0, 6.0),
            ))
        }
        Habitat::CaveClinger | Habitat::Arboreal => random_open_near(world, rng, around, r, |x, y| {
            open(x, y) && world.material(x, y - 1).is_solid_for_player()
        }),
        _ => random_open_near(world, rng, around, r, |x, y| {
            open(x, y) && world.material(x, y + 1).blocks_creature(sp.traits.climb) && open(x, y - 2)
        }),
    };
    found.unwrap_or_else(|| c.pos + v2(rng.range(-30.0, 30.0), rng.range(-10.0, 10.0)))
}

fn away_from(c: &Creature, from: V2, dist: f32) -> V2 {
    let mut d = (c.pos - from).norm();
    if d.len() < 0.1 {
        d = v2(c.facing, 0.0);
    }
    c.pos + d * dist
}

/// The nearest cell matching `pred`, scanning outward in rings.
fn nearest_cell(
    world: &World,
    from: V2,
    max_r: i32,
    step: i32,
    pred: impl Fn(i32, i32) -> bool,
) -> Option<V2> {
    let (cx, cy) = from.cell();
    let mut r = 0;
    while r <= max_r {
        let mut best: Option<(i32, i32)> = None;
        let mut bd = i32::MAX;
        for dy in (-r..=r).step_by(step as usize) {
            for dx in (-r..=r).step_by(step as usize) {
                if dx.abs() != r && dy.abs() != r {
                    continue;
                }
                let (x, y) = (cx + dx, cy + dy);
                if world.in_bounds(x, y) && pred(x, y) {
                    let d = dx * dx + dy * dy;
                    if d < bd {
                        bd = d;
                        best = Some((x, y));
                    }
                }
            }
        }
        if let Some((x, y)) = best {
            return Some(v2(x as f32 + 0.5, y as f32 + 0.5));
        }
        r += step;
    }
    None
}

fn intent(goal: Option<V2>, speed: f32) -> Intent {
    Intent {
        goal,
        speed,
        ..Default::default()
    }
}

pub fn act(c: &mut Creature, sp: &Species, ctx: &Ctx, world: &World, rng: &mut Rng) -> Intent {
    let kind = c.brain.current;
    c.brain.timer += super::DT;
    let p = c.brain.perception.clone();
    let s = c.scale(sp);
    let reach = sp.stats.reach * s;
    let mut it = match kind {
        BehaviorKind::Escape => act_escape(c, sp, world),
        BehaviorKind::Flee => act_flee(c, sp, rng),
        BehaviorKind::Freeze => {
            c.brain.label = "freezing in place";
            intent(None, 0.0)
        }
        BehaviorKind::Shelter => {
            let home = c.brain.home.unwrap_or(c.pos);
            c.brain.label = "sheltering";
            if home.dist(c.pos) < 4.0 {
                c.sleeping = sp.activity.alertness(ctx.daylight, ctx.dusk) < 0.5;
                intent(None, 0.0)
            } else {
                intent(Some(home), 1.1)
            }
        }
        BehaviorKind::Graze => act_graze(c, sp, ctx, world, rng, reach),
        BehaviorKind::Hunt => act_hunt(c, sp, ctx, world, rng, reach),
        BehaviorKind::Ambush => act_ambush(c, sp, ctx, world, rng),
        BehaviorKind::SetTrap => act_trap(c, sp, ctx, world, rng),
        BehaviorKind::Scavenge => {
            if let Some((id, pos)) = p.carcass {
                c.brain.label = "feeding on a carcass";
                let mut it = intent(Some(pos), 0.9);
                it.fly = sp.traits.fly;
                if pos.dist(c.pos) < reach + 3.0 {
                    it.goal = None;
                    it.action = Action::EatCarcass(id);
                }
                give_up_if_stuck(c, key_carcass(id), ctx.time);
                it
            } else if let Some((id, pos)) = p.egg {
                c.brain.label = "raiding eggs";
                let mut it = intent(Some(pos), 0.9);
                if pos.dist(c.pos) < reach + 3.0 {
                    it.action = Action::EatEgg(id);
                }
                it
            } else {
                intent(None, 0.0)
            }
        }
        BehaviorKind::Rest => act_rest(c, sp, ctx, world, rng),
        BehaviorKind::BuildNest => act_build(c, sp, world, rng),
        BehaviorKind::Reproduce => {
            if let Some((id, pos)) = p.mate {
                c.brain.label = "courting";
                let mut it = intent(Some(pos), 0.7);
                it.fly = sp.traits.fly;
                if pos.dist(c.pos) < reach + sp.stats.size * 2.0 + 3.0 {
                    it.goal = None;
                    it.action = Action::Mate(id);
                }
                it
            } else {
                c.brain.label = "looking for a mate";
                wander(c, sp, ctx, world, rng, 0.6)
            }
        }
        BehaviorKind::Flock => {
            c.brain.label = "rejoining the group";
            let mut it = intent(p.kin_centre, 0.75);
            it.fly = sp.traits.fly;
            it
        }
        BehaviorKind::Mob => {
            if let Some((id, pos)) = p.kin_attacker {
                c.brain.label = "mobbing a predator";
                let mut it = intent(Some(pos), 1.15);
                it.fly = sp.traits.fly;
                if pos.dist(c.pos) < reach + 6.0 {
                    it.action = Action::Attack(id);
                }
                it
            } else {
                intent(None, 0.0)
            }
        }
        BehaviorKind::Patrol => {
            if let Some((id, pos)) = p.intruder {
                c.brain.label = "driving off a rival";
                let mut it = intent(Some(pos), 1.0);
                if pos.dist(c.pos) < reach + 4.0 {
                    it.action = Action::Attack(id);
                }
                it
            } else {
                c.brain.label = "patrolling its territory";
                wander(c, sp, ctx, world, rng, 0.5)
            }
        }
        BehaviorKind::Investigate => {
            let target = if sp.role == Role::Prey {
                p.lure.map(|l| l.1).or(p.noise)
            } else {
                p.noise
            };
            c.brain.label = if p.lure.is_some() && sp.role == Role::Prey {
                "drawn to a strange light"
            } else {
                "investigating a sound"
            };
            let mut it = intent(target, if sp.role == Role::Predator { 0.6 } else { 0.5 });
            it.sneak = sp.role == Role::Predator;
            it.fly = sp.traits.fly;
            it
        }
        BehaviorKind::Wander => {
            c.brain.label = "wandering";
            wander(c, sp, ctx, world, rng, 0.5)
        }
    };
    if sp.loco == Locomotion::Float {
        it.fly = true;
    }
    it
}

fn give_up_if_stuck(c: &mut Creature, key: u64, now: f32) {
    if c.brain.stuck >= 3.0 {
        c.brain.ignore.push((key, now + 25.0));
        c.brain.stuck = 0.0;
        c.brain.path.clear();
    }
}

fn wander(c: &mut Creature, sp: &Species, ctx: &Ctx, world: &World, rng: &mut Rng, speed: f32) -> Intent {
    let arrived = c.brain.spot.is_none_or(|s| s.dist(c.pos) < 6.0);
    if arrived || c.brain.timer > 14.0 || c.brain.stuck >= 3.0 {
        c.brain.spot = Some(wander_target(c, sp, world, ctx, rng));
        c.brain.timer = 0.0;
        c.brain.stuck = 0.0;
        c.brain.path.clear();
    }
    let mut it = intent(c.brain.spot, speed);
    it.fly = sp.traits.fly && (sp.habitat == Habitat::Aerial || rng.prob(0.02) || c.flying);
    it.dig = sp.habitat == Habitat::Burrower;
    it
}

fn act_escape(c: &mut Creature, sp: &Species, world: &World) -> Intent {
    c.brain.label = "escaping danger";
    let t = &sp.traits;
    let goal = if t.fly && c.in_liquid {
        Some(c.pos - v2(0.0, 30.0))
    } else if c.in_liquid && t.swim && !t.gills && c.breath < 0.6 {
        // Swim straight up for air.
        let (x, mut y) = c.pos.cell();
        while y > 1 && world.material(x, y).kind() == Kind::Liquid {
            y -= 1;
        }
        Some(v2(x as f32 + 0.5, y as f32 - 2.0))
    } else if t.gills && !c.in_liquid {
        nearest_cell(world, c.pos, 80, 2, |x, y| {
            let m = world.material(x, y);
            m.kind() == Kind::Liquid && sp.swims_in(m)
        })
    } else if c.status.burning > 0.0 {
        nearest_cell(world, c.pos, 50, 2, |x, y| {
            world.material(x, y) == Material::Water
        })
        .or_else(|| Some(away_from(c, c.pos + v2(c.facing, 0.0), 30.0)))
    } else {
        // Out of the liquid: the nearest dry spot to stand on.
        nearest_cell(world, c.pos, 60, 2, |x, y| {
            world.material(x, y) == Material::Empty
                && world.material(x, y - 3) == Material::Empty
                && world.material(x, y + 1).blocks_creature(sp.traits.climb)
        })
    };
    let mut it = intent(goal, 1.2);
    it.fly = t.fly;
    it
}

fn act_flee(c: &mut Creature, sp: &Species, rng: &mut Rng) -> Intent {
    let p = &c.brain.perception;
    if c.playing_dead > 0.0 {
        c.brain.label = "playing dead";
        return intent(None, 0.0);
    }
    let threat = p
        .threat
        .map(|t| t.1)
        .or(p.alarm)
        .unwrap_or(c.pos + v2(c.facing * 10.0, 0.0));
    let d = threat.dist(c.pos);
    let t = &sp.traits;
    let mut it = intent(Some(away_from(c, threat, 60.0)), 1.25);
    c.brain.label = "fleeing";
    c.noise = c.noise.max(if t.alarm_call { 1.0 } else { 0.5 });

    if t.ink && d < 28.0 && c.ink_cd <= 0.0 {
        it.action = Action::Ink;
    } else if t.play_dead && d < 14.0 && c.on_ground && c.playing_dead <= 0.0 && rng.prob(0.02) {
        it.action = Action::PlayDead;
        return it;
    }
    if let Some(home) = c.brain.home
        && sp.flees(FleeStyle::Shelter)
        && home.dist(c.pos) < 90.0
        && (home - c.pos).dot(threat - c.pos) < (home.dist(c.pos) * d * 0.5)
    {
        c.brain.label = "running for shelter";
        it.goal = Some(home);
        return it;
    }
    if sp.flees(FleeStyle::Burrow) {
        c.brain.label = "burrowing to safety";
        it.dig = true;
        let down = away_from(c, threat, 25.0) + v2(0.0, 20.0);
        it.goal = Some(down);
        return it;
    }
    if sp.flees(FleeStyle::Fly) {
        c.brain.label = "taking flight";
        it.fly = true;
        let up = away_from(c, threat, 50.0) + v2(0.0, -35.0);
        it.goal = Some(v2(up.x, up.y.max(8.0)));
        it.direct = true;
        return it;
    }
    if sp.flees(FleeStyle::Dive) && c.in_liquid {
        c.brain.label = "diving deep";
        it.goal = Some(away_from(c, threat, 30.0) + v2(0.0, 25.0));
        return it;
    }
    if sp.flees(FleeStyle::Leap) && c.on_ground && d < 22.0 && c.leap_cd <= 0.0 {
        let dir = ((c.pos - threat).norm() + v2(0.0, -0.8)).norm();
        it.action = Action::Leap(dir);
    }
    it
}

fn act_graze(c: &mut Creature, sp: &Species, ctx: &Ctx, world: &World, rng: &mut Rng, reach: f32) -> Intent {
    let p = c.brain.perception.clone();
    let lure_first = p.lure.is_some() && (p.plant.is_none() || rng.prob(sp.personality.curiosity * 0.02));
    if lure_first && let Some((_, pos)) = p.lure {
        c.brain.label = "drawn to a strange light";
        let mut it = intent(Some(pos), 0.6);
        it.fly = sp.traits.fly;
        return it;
    }
    let Some((id, fp)) = p.plant else {
        c.brain.label = "foraging";
        return wander(c, sp, ctx, world, rng, 0.55);
    };
    c.brain.label = "grazing";
    let mut it = intent(Some(fp), 0.65);
    it.fly = sp.traits.fly;
    it.dig = sp.habitat == Habitat::Burrower;
    let body = physics::bounds(c, sp);
    if fp.dist(c.pos) < reach + body.hw.max(body.top) + 2.0 {
        it.goal = None;
        it.action = Action::EatPlant(id);
    }
    give_up_if_stuck(c, key_plant(id), ctx.time);
    it
}

fn act_hunt(c: &mut Creature, sp: &Species, ctx: &Ctx, world: &World, rng: &mut Rng, reach: f32) -> Intent {
    let p = c.brain.perception.clone();
    let target = p
        .webbed_prey
        .map(|(id, _)| id)
        .or(p.prey.map(|(id, _, _)| id))
        .or(c.brain.target);
    let Some(tid) = target else {
        return search(c, sp, ctx, world, rng);
    };
    let Some(t) = ctx.snap(tid).copied() else {
        c.brain.target = None;
        return intent(None, 0.0);
    };
    if c.brain.target != Some(tid) {
        c.brain.target = Some(tid);
        c.brain.stage = 0;
        c.brain.timer = 0.0;
    }
    let d = t.pos.dist(c.pos);
    // Chases are short bursts; a long one means the prey got away.
    let patient = sp.hunts(HuntStyle::Sniper) || sp.hunts(HuntStyle::Stalk) || t.webbed;
    let limit = if patient {
        35.0
    } else {
        14.0 + sp.personality.aggression * 8.0
    };
    if d > sp.stats.sight * 1.5 || (c.brain.timer > limit && d > 12.0) {
        c.brain.ignore.push((key_creature(tid), ctx.time + 15.0));
        c.brain.target = None;
        return intent(None, 0.0);
    }
    // Can't follow prey into a medium we can't enter.
    let tm = world.material(t.pos.x as i32, t.pos.y as i32);
    if tm.kind() == Kind::Liquid && !sp.liquid_ok(tm) {
        c.brain.ignore.push((key_creature(tid), ctx.time + 10.0));
        return intent(None, 0.0);
    }
    let t_size = t.size;
    let strike = reach + t_size * 0.8 + 2.0;
    let traits = &sp.traits;
    let mut it = intent(Some(t.pos + t.vel * 0.25), 1.2);
    it.fly = traits.fly;
    it.dig = traits.dig && t.burrowed;
    c.brain.label = "chasing prey";

    // Ranged first.
    if let Some(_kind) = traits.spit
        && d > 12.0
        && d < sp.stats.spit_range
        && c.spit_cd <= 0.0
        && !t.webbed
        && line_of_sight(world, c.pos, t.pos)
    {
        it.action = Action::Spit(t.pos + t.vel * (d / 110.0));
    }
    // Snipers soften prey up from range for a while, then close in.
    let sniper = sp.hunts(HuntStyle::Sniper) && t.health > 0.45 && !t.webbed && c.brain.timer < 9.0;
    if sniper && d < sp.stats.spit_range * 0.9 {
        c.brain.label = "spitting at prey";
        let keep = if d < sp.stats.spit_range * 0.4 {
            away_from(c, t.pos, 20.0)
        } else {
            c.pos
        };
        it.goal = Some(keep);
        it.speed = 0.8;
        return it;
    }
    let stalker = sp.hunts(HuntStyle::Stalk) && !t.fleeing && !t.webbed;
    let pounce = sp.stats.size * c.scale(sp) * 3.0 + 18.0;
    if stalker && d > pounce {
        c.brain.label = "stalking";
        it.sneak = true;
        it.speed = 0.4;
        return it;
    }
    if (stalker || traits.leap)
        && d < pounce
        && d > strike
        && c.on_ground
        && c.leap_cd <= 0.0
        && t.pos.y > c.pos.y - 30.0
    {
        c.brain.label = "pouncing";
        let dir = (t.pos - c.pos).norm();
        it.action = Action::Leap((dir + v2(0.0, -0.45)).norm());
    }
    if sp.hunts(HuntStyle::Pack) && d > strike * 2.5 {
        // Flank: come at the prey from the other side to the pack.
        c.brain.label = "flanking with the pack";
        let side = if c.id.is_multiple_of(2) { 1.0 } else { -1.0 };
        let perp = (t.pos - c.pos).norm().perp() * side * 16.0;
        it.goal = Some(t.pos + t.vel * 0.4 + perp);
    }
    if traits.fly && d > strike * 2.0 && !t.webbed {
        // Get above, then dive.
        c.brain.label = "circling above prey";
        it.goal = Some(t.pos + v2(0.0, -20.0));
        it.direct = true;
        if (c.pos.x - t.pos.x).abs() < 12.0 && c.pos.y < t.pos.y {
            c.brain.label = "diving at prey";
            it.goal = Some(t.pos);
            it.speed = 1.5;
        }
    }
    if d < strike {
        it.action = Action::Attack(tid);
        c.brain.label = "attacking";
        if traits.constrict {
            c.brain.label = "constricting";
        }
    }
    if c.brain.stuck >= 4.0 {
        c.brain.ignore.push((key_creature(tid), ctx.time + 12.0));
        c.brain.stuck = 0.0;
    }
    it
}

/// Hunting with nothing in sight: return to where prey was last seen, then
/// check the places prey feed.
fn search(c: &mut Creature, sp: &Species, ctx: &Ctx, world: &World, rng: &mut Rng) -> Intent {
    if let Some((pos, when)) = c.brain.last_prey
        && ctx.time - when < 15.0
        && pos.dist(c.pos) > 6.0
    {
        c.brain.label = "tracking prey";
        let mut it = intent(Some(pos), 0.8);
        it.fly = sp.traits.fly;
        it.dig = sp.traits.dig && sp.habitat == Habitat::Burrower;
        return it;
    }
    c.brain.label = "searching for prey";
    let arrived = c.brain.spot.is_none_or(|s| s.dist(c.pos) < 8.0);
    if arrived || c.brain.timer > 15.0 || c.brain.stuck >= 3.0 {
        c.brain.spot = Some(ambush_spot(c, sp, ctx, world, rng) + v2(rng.range(-15.0, 15.0), 0.0));
        c.brain.timer = 0.0;
        c.brain.stuck = 0.0;
    }
    let mut it = intent(c.brain.spot, 0.65);
    it.fly = sp.traits.fly;
    it.dig = sp.habitat == Habitat::Burrower;
    it
}

fn act_ambush(c: &mut Creature, sp: &Species, ctx: &Ctx, world: &World, rng: &mut Rng) -> Intent {
    let lure = sp.hunts(HuntStyle::Lure);
    if c.brain.spot.is_none() {
        c.brain.spot = Some(ambush_spot(c, sp, ctx, world, rng));
        c.brain.stage = 0;
        c.brain.timer = 0.0;
    }
    let spot = c.brain.spot.unwrap();
    let patience = 30.0 + sp.personality.patience * 70.0;
    if c.brain.timer > patience {
        c.brain.spot = None;
        return intent(None, 0.0);
    }
    if c.brain.stage == 0 {
        c.brain.label = "moving to an ambush spot";
        let mut it = intent(Some(spot), 0.6);
        it.sneak = true;
        it.fly = sp.traits.fly;
        it.dig = sp.habitat == Habitat::Burrower;
        if spot.dist(c.pos) < 5.0 || c.brain.stuck >= 3.0 {
            c.brain.stage = 1;
            c.brain.timer = 0.0;
        }
        return it;
    }
    c.luring = lure;
    c.brain.label = if lure {
        "dangling its lure"
    } else if c.burrowed {
        "buried, waiting"
    } else {
        "lying in ambush"
    };
    let mut it = intent(None, 0.0);
    it.dig = sp.habitat == Habitat::Burrower;
    it
}

fn ambush_spot(c: &Creature, sp: &Species, ctx: &Ctx, world: &World, rng: &mut Rng) -> V2 {
    // Prey gather at food, so wait near plants they eat.
    let prey_sp = ctx.species.iter().find(|s| s.role == Role::Prey);
    let candidates: Vec<&Plant> = ctx
        .plants
        .iter()
        .filter(|p| !p.dead && prey_sp.is_some_and(|ps| ps.food_flora.contains(&p.flora)))
        .filter(|p| p.base().dist(c.pos) < 140.0)
        .collect();
    if !candidates.is_empty() {
        let pl = *rng.pick(&candidates);
        let fl = &ctx.flora[pl.flora];
        let base = pl.base();
        return match sp.habitat {
            Habitat::Burrower => base + v2(rng.range(-6.0, 6.0), sp.stats.size + 3.0),
            Habitat::CaveClinger => nearest_cell(world, pl.tip(fl), 40, 2, |x, y| {
                world.material(x, y) == Material::Empty && world.material(x, y - 1).is_solid_for_player()
            })
            .unwrap_or(base),
            Habitat::Aerial => base + v2(rng.range(-10.0, 10.0), -25.0),
            _ => base + v2(rng.range(-10.0, 10.0), -2.0),
        };
    }
    wander_target(c, sp, world, ctx, rng)
}

fn act_trap(c: &mut Creature, sp: &Species, ctx: &Ctx, world: &World, rng: &mut Rng) -> Intent {
    let web = sp.hunts(HuntStyle::WebTrap);
    // Check on an existing trap.
    if let Some(trap) = c.brain.trap {
        let want = if web { Material::Web } else { Material::Empty };
        let intact = c
            .brain
            .trap_cells
            .iter()
            .filter(|&&(x, y)| world.material(x, y) == want)
            .count();
        if intact * 3 < c.brain.trap_cells.len() {
            c.brain.trap = None;
            c.brain.trap_cells.clear();
            c.brain.stage = 0;
        } else {
            c.brain.label = if web {
                "waiting by its web"
            } else {
                "waiting at the bottom of its pit"
            };
            let wait = if web {
                c.brain.spot.unwrap_or(trap)
            } else {
                trap + v2(0.0, 2.0)
            };
            let mut it = intent(Some(wait), 0.5);
            it.dig = !web;
            if wait.dist(c.pos) < 4.0 {
                it.goal = None;
            }
            return it;
        }
    }
    match c.brain.stage {
        0 => {
            c.brain.label = if web {
                "looking for a place to spin"
            } else {
                "looking for soft ground"
            };
            let site = if web {
                web_site(c, ctx, world, rng)
            } else {
                pit_site(c, ctx, world, rng)
            };
            match site {
                Some((anchor, cells)) => {
                    c.brain.spot = Some(anchor);
                    c.brain.trap_cells = cells;
                    c.brain.stage = 1;
                    c.brain.timer = 0.0;
                }
                None => return wander(c, sp, ctx, world, rng, 0.5),
            }
            intent(None, 0.0)
        }
        1 => {
            c.brain.label = "heading to its trap site";
            let spot = c.brain.spot.unwrap_or(c.pos);
            if spot.dist(c.pos) < 6.0 {
                c.brain.stage = 2;
            }
            if c.brain.stuck >= 4.0 || c.brain.timer > 25.0 {
                c.brain.stage = 0;
                c.brain.trap_cells.clear();
            }
            intent(Some(spot), 0.7)
        }
        _ => {
            c.brain.label = if web { "spinning a web" } else { "digging a pit" };
            let mut it = intent(None, 0.0);
            it.action = if web {
                Action::SpinWeb
            } else {
                Action::Dig(c.brain.spot.unwrap_or(c.pos))
            };
            if c.brain.trap_cells.is_empty() {
                c.brain.stage = 0;
            }
            it
        }
    }
}

/// A horizontal span between two solid walls near prey food: the web line.
fn web_site(c: &Creature, ctx: &Ctx, world: &World, rng: &mut Rng) -> Option<(V2, Vec<(i32, i32)>)> {
    let prey_sp = ctx.species.iter().find(|s| s.role == Role::Prey)?;
    for _ in 0..40 {
        let origin = if rng.prob(0.7) {
            let foods: Vec<&Plant> = ctx
                .plants
                .iter()
                .filter(|p| !p.dead && prey_sp.food_flora.contains(&p.flora) && p.base().dist(c.pos) < 160.0)
                .collect();
            if foods.is_empty() {
                c.pos
            } else {
                rng.pick(&foods).base()
            }
        } else {
            c.pos
        };
        let x = origin.x as i32 + rng.int(-20, 20);
        let y = origin.y as i32 - rng.int(2, 20);
        if world.material(x, y) != Material::Empty {
            continue;
        }
        let horizontal = rng.coin();
        let (dx, dy) = if horizontal { (1, 0) } else { (0, 1) };
        let mut a = (x, y);
        let mut b = (x, y);
        let mut ok = true;
        for _ in 0..30 {
            if world.material(a.0 - dx, a.1 - dy) != Material::Empty {
                break;
            }
            a = (a.0 - dx, a.1 - dy);
        }
        for _ in 0..30 {
            if world.material(b.0 + dx, b.1 + dy) != Material::Empty {
                break;
            }
            b = (b.0 + dx, b.1 + dy);
        }
        let span = (b.0 - a.0) + (b.1 - a.1);
        if !(6..=50).contains(&span) {
            ok = false;
        }
        if !world.material(a.0 - dx, a.1 - dy).is_solid_for_player()
            || !world.material(b.0 + dx, b.1 + dy).is_solid_for_player()
        {
            ok = false;
        }
        if !ok {
            continue;
        }
        let mut cells = Vec::new();
        let n = span.max(1);
        // Main strand with a slight sag, plus a zig-zag of support threads.
        for i in 0..=n {
            let t = i as f32 / n as f32;
            let sag = ((t * PI).sin() * span as f32 * 0.08) as i32;
            let (cx, cy) = (a.0 + dx * i + dy * sag, a.1 + dy * i + dx * sag);
            cells.push((cx, cy));
        }
        for k in 0..3 {
            let off = (k + 1) * 3;
            for i in (0..=n).step_by(2) {
                let wob = if i % 4 == 0 { 1 } else { 0 };
                let (cx, cy) = (a.0 + dx * i + dy * (off + wob), a.1 + dy * i + dx * (off + wob));
                if world.material(cx, cy) == Material::Empty {
                    cells.push((cx, cy));
                }
            }
        }
        cells.retain(|&(x, y)| world.material(x, y) == Material::Empty);
        let anchor = v2(a.0 as f32 + 0.5, a.1 as f32 + 0.5);
        return Some((anchor, cells));
    }
    None
}

/// A cone of cells to dig out of soft ground near prey food.
fn pit_site(c: &Creature, ctx: &Ctx, world: &World, rng: &mut Rng) -> Option<(V2, Vec<(i32, i32)>)> {
    let prey_sp = ctx.species.iter().find(|s| s.role == Role::Prey)?;
    let s = ctx.species[c.species].stats.size;
    for _ in 0..30 {
        let foods: Vec<&Plant> = ctx
            .plants
            .iter()
            .filter(|p| !p.dead && prey_sp.food_flora.contains(&p.flora) && p.base().dist(c.pos) < 180.0)
            .collect();
        let origin = if foods.is_empty() {
            c.pos
        } else {
            rng.pick(&foods).base()
        };
        let x = origin.x as i32 + rng.int(-25, 25);
        // Find the ground surface in this column near the origin.
        let mut y = origin.y as i32 - 20;
        while y < origin.y as i32 + 30 && !world.material(x, y + 1).is_solid_for_player() {
            y += 1;
        }
        if !world.material(x, y + 1).is_diggable() || world.material(x, y) != Material::Empty {
            continue;
        }
        let half = (s * 1.6 + 6.0) as i32;
        let depth = (s * 1.5 + 6.0) as i32;
        let mut cells = Vec::new();
        for dy in 1..=depth {
            let w = half * (depth - dy + 1) / depth;
            for dx in -w..=w {
                if world.material(x + dx, y + dy).is_diggable() {
                    cells.push((x + dx, y + dy));
                }
            }
        }
        if cells.len() < 20 {
            continue;
        }
        let anchor = v2(x as f32 + 0.5, y as f32);
        return Some((anchor, cells));
    }
    None
}

fn act_rest(c: &mut Creature, sp: &Species, ctx: &Ctx, world: &World, rng: &mut Rng) -> Intent {
    let spot = if let Some(h) = c.brain.home {
        h
    } else {
        if c.brain.spot.is_none() {
            c.brain.spot = Some(match sp.habitat {
                Habitat::Aerial | Habitat::Arboreal if !ctx.trees.is_empty() => {
                    let t = ctx
                        .trees
                        .iter()
                        .min_by(|a, b| {
                            let da = v2(a.x as f32, a.ground as f32).dist(c.pos);
                            let db = v2(b.x as f32, b.ground as f32).dist(c.pos);
                            da.total_cmp(&db)
                        })
                        .unwrap();
                    v2(
                        t.x as f32 + rng.range(-4.0, 4.0),
                        (t.ground - t.height) as f32 - 4.0,
                    )
                }
                Habitat::Burrower => c.pos + v2(0.0, 6.0),
                Habitat::Aerial | Habitat::CaveClinger if sp.loco != Locomotion::Float => {
                    // A perch: a ledge, or a ceiling for clingers.
                    let clinger = sp.traits.climb;
                    nearest_cell(world, c.pos + v2(0.0, 8.0), 90, 2, |x, y| {
                        world.material(x, y) == Material::Empty
                            && if clinger {
                                world.material(x, y - 1).is_solid_for_player()
                            } else {
                                world.material(x, y + 1).blocks_creature(false)
                                    && world.material(x, y - 4) == Material::Empty
                            }
                    })
                    .map(|p| p - v2(0.0, 2.0))
                    .unwrap_or(c.pos)
                }
                _ => c.pos,
            });
        }
        c.brain.spot.unwrap()
    };
    let settled = c.on_ground
        || c.clinging
        || c.burrowed
        || (c.in_liquid && sp.traits.swim)
        || sp.loco == Locomotion::Float;
    if (spot.dist(c.pos) < 6.0 || c.brain.stuck >= 3.0 || c.sleeping || c.brain.timer > 15.0) && settled {
        c.sleeping = true;
        c.brain.label = "sleeping";
        let mut it = intent(None, 0.0);
        it.dig = sp.habitat == Habitat::Burrower;
        it
    } else {
        c.brain.label = "heading home to rest";
        let mut it = intent(Some(spot), 0.6);
        it.fly = sp.traits.fly;
        it.dig = sp.habitat == Habitat::Burrower;
        it
    }
}

/// Plans a domed mud shelter around `floor` with a doorway on one side.
fn plan_nest(c: &Creature, sp: &Species, world: &World, floor: V2, door_right: bool) -> Vec<(i32, i32)> {
    let b = physics::bounds(c, sp);
    let inner = (b.top + b.bottom).max(b.hw * 2.0) + 1.5;
    let outer = inner + 2.0;
    let door_h = (b.top + b.bottom) + 1.0;
    let (cx, cy) = (floor.x, floor.y);
    let mut cells = Vec::new();
    let r = outer.ceil() as i32;
    for dy in -r..=0 {
        for dx in -r..=r {
            let d = ((dx * dx + dy * dy) as f32).sqrt();
            if d <= inner || d > outer {
                continue;
            }
            let (x, y) = (cx as i32 + dx, cy as i32 + dy);
            let doorway = (dx > 0) == door_right && (-dy as f32) < door_h && dx.abs() as f32 > inner * 0.5;
            if doorway {
                continue;
            }
            if world.material(x, y) == Material::Empty {
                cells.push((x, y));
            }
        }
    }
    // Build from the ground up.
    cells.sort_by_key(|&(_, y)| std::cmp::Reverse(y));
    cells
}

fn act_build(c: &mut Creature, sp: &Species, world: &World, rng: &mut Rng) -> Intent {
    if c.brain.home.is_none() {
        // Pick a site: flat dry ground right here, if possible.
        let b = physics::bounds(c, sp);
        let floor = v2(c.pos.x, c.pos.y + b.bottom - 0.5);
        let flat = c.on_ground
            && !c.in_liquid
            && (-4..=4).all(|dx| {
                world
                    .material(floor.x as i32 + dx, floor.y as i32 + 1)
                    .blocks_creature(false)
            });
        if !flat {
            c.brain.label = "looking for a nest site";
            let mut it = intent(c.brain.spot, 0.5);
            if c.brain.spot.is_none_or(|s| s.dist(c.pos) < 5.0) || c.brain.stuck >= 2.0 {
                c.brain.spot = Some(c.pos + v2(rng.range(-40.0, 40.0), 0.0));
                c.brain.stuck = 0.0;
            }
            it.goal = c.brain.spot;
            return it;
        }
        c.brain.home = Some(v2(c.pos.x, c.pos.y));
        c.brain.nest_plan = plan_nest(c, sp, world, floor, rng.coin());
        c.brain.nest_cells = c.brain.nest_plan.clone();
    }
    let home = c.brain.home.unwrap();
    if c.carry_dirt == 0 {
        c.brain.label = "gathering mud";
        // Dig a little soil from nearby, away from the shelter itself.
        if c.brain
            .spot
            .is_none_or(|s| s.dist(home) < 10.0 || !world.material(s.x as i32, s.y as i32).is_diggable())
        {
            c.brain.spot = nearest_cell(world, home + v2(rng.range(-25.0, 25.0), 0.0), 30, 2, |x, y| {
                world.material(x, y).is_diggable()
                    && world.material(x, y - 1) == Material::Empty
                    && (x as f32 - home.x).abs() > 9.0
            });
        }
        let Some(spot) = c.brain.spot else {
            c.brain.home = None;
            return intent(None, 0.0);
        };
        let mut it = intent(Some(spot - v2(0.0, 2.0)), 0.7);
        if spot.dist(c.pos) < physics::bounds(c, sp).bottom + 5.0 {
            it.action = Action::Dig(spot);
        }
        return it;
    }
    c.brain.label = "building a shelter";
    let mut it = intent(Some(home), 0.6);
    if home.dist(c.pos) < 14.0 {
        it.action = Action::PlaceNest;
    }
    it
}

// ---------------------------------------------------------------------------
// Steering

/// Turns an intent into a movement command, pathfinding when needed.
#[allow(clippy::too_many_arguments)]
pub fn steer(
    c: &mut Creature,
    sp: &Species,
    it: &Intent,
    nav: &mut NavGrid,
    world: &World,
    gravity: f32,
    budget: &mut u32,
    rng: &mut Rng,
) -> MoveCmd {
    let s = c.scale(sp);
    let fatigue = if c.energy < 0.15 { 0.6 } else { 1.0 };
    let base_speed = sp.stats.speed * s.powf(0.3) * fatigue;
    let speed = base_speed * it.speed * if c.status.poison > 0.0 { 0.6 } else { 1.0 };
    let mut cmd = MoveCmd {
        dir: V2::ZERO,
        speed,
        jump: false,
        fly: it.fly,
        dig: it.dig,
        climb: sp.traits.climb,
    };
    let Some(goal) = it.goal else {
        cmd.fly = it.fly && c.flying;
        return cmd;
    };
    let to_goal = goal - c.pos;
    let dist = to_goal.len();
    if dist < 1.5 {
        return cmd;
    }
    let b = physics::bounds(c, sp);
    let free_mover = c.flying || (c.in_liquid && sp.traits.swim) || c.burrowed || c.clinging;
    let direct = it.direct
        || (dist < 16.0 && (free_mover || (to_goal.y.abs() < 6.0 && line_of_sight(world, c.pos, goal))));
    // Walkers navigate from their feet, everything else from the body.
    let anchor = if free_mover {
        c.pos
    } else {
        c.pos + v2(0.0, b.bottom - 1.0)
    };

    let waypoint = if direct {
        c.brain.path.clear();
        goal
    } else {
        let stale = c.brain.path.is_empty() || c.brain.path_goal.dist(goal) > 10.0 || c.brain.repath <= 0.0;
        if stale && *budget > 0 {
            *budget -= 1;
            let prof = physics::profile(sp, s, gravity);
            c.brain.path = nav.find_path(&prof, anchor, goal, 2200);
            c.brain.path_goal = goal;
            c.brain.repath = 2.0 + rng.range(0.0, 1.5);
        }
        // Drop waypoints we've reached.
        while let Some(&w) = c.brain.path.first() {
            let d = w - anchor;
            let reached = if free_mover {
                d.len() < TILE as f32 * 1.2
            } else {
                d.x.abs() < (b.hw * 0.5 + 1.0).max(3.0) && d.y.abs() < TILE as f32 * 1.6
            };
            if reached {
                c.brain.path.remove(0);
            } else {
                break;
            }
        }
        c.brain.path.first().copied().unwrap_or(goal)
    };

    let d = waypoint - if c.brain.path.is_empty() { c.pos } else { anchor };
    cmd.dir = d.norm();
    // Wedged somewhere: wriggle in a different direction each second.
    if c.brain.stuck >= 2.0 {
        let turn = [FRAC_PI_2, -FRAC_PI_2, PI, 0.6][c.brain.stuck as usize % 4];
        cmd.dir = cmd.dir.rot(turn);
        cmd.jump = true;
    }
    let mode_ground = !free_mover;
    if mode_ground && c.on_ground && cmd.dir.x != 0.0 {
        let above = d.y < -3.0 && d.x.abs() < 18.0;
        let blocked = c.vel.x.abs() < 1.0 && c.brain.stuck > 0.5;
        let ahead_x = (c.pos.x + cmd.dir.x.signum() * (b.hw + 3.0)) as i32;
        let feet = (c.pos.y + b.bottom) as i32;
        // A gap ahead with the waypoint across it.
        let gap = d.x.abs() > 6.0
            && d.y < 4.0
            && !(0..14).any(|k| world.material(ahead_x, feet + k).blocks_creature(sp.traits.climb));
        // Never walk off a deadly drop or into something that would kill us,
        // unless the planned path says the drop is fine.
        let planned_drop = !c.brain.path.is_empty() && d.y > 3.0;
        let landing = (0..64)
            .map(|k| world.material(ahead_x, feet + k))
            .find(|m| !matches!(m.kind(), Kind::Empty | Kind::Gas));
        let deadly = match landing {
            None => true,
            Some(m) if m.kind() == Kind::Liquid => !sp.swims_in(m),
            Some(m) => !m.blocks_creature(sp.traits.climb) && m != Material::Web,
        };
        if deadly && !planned_drop && !(gap && sp.traits.leap) {
            cmd.dir.x = 0.0;
            cmd.jump = above;
        } else {
            cmd.jump = above || blocked || gap;
        }
    }
    if sp.traits.fly && (d.y < -16.0 || it.fly) {
        cmd.fly = true;
    }
    if sp.traits.dig && !cmd.dig {
        let wm = world.material(waypoint.x as i32, waypoint.y as i32);
        cmd.dig = wm.is_diggable() || c.burrowed && wm.is_solid_for_player();
    }
    if it.sneak {
        cmd.speed = cmd.speed.min(base_speed * 0.45);
    }
    cmd
}

/// Updates stuck detection once per second.
pub fn track_progress(c: &mut Creature, it: &Intent, dt: f32) {
    c.brain.stuck_check += dt;
    c.brain.repath -= dt;
    if c.brain.stuck_check < 1.0 {
        return;
    }
    c.brain.stuck_check = 0.0;
    let moved = c.pos.dist(c.brain.last_pos);
    c.brain.last_pos = c.pos;
    let wants = it.goal.is_some_and(|g| g.dist(c.pos) > 4.0);
    if wants && moved < 1.5 && c.webbed <= 0.0 {
        c.brain.stuck += 1.0;
        if c.brain.stuck >= 2.0 {
            c.brain.path.clear();
            c.brain.repath = 0.0;
        }
    } else {
        c.brain.stuck = (c.brain.stuck - 1.0).max(0.0);
    }
}

pub fn mode_label(m: Mode) -> &'static str {
    match m {
        Mode::Ground => "on foot",
        Mode::Swim => "swimming",
        Mode::Fly => "flying",
        Mode::Climb => "climbing",
        Mode::Burrow => "burrowing",
    }
}
