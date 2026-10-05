//! The planet's fauna: harmless surface life and the predators of the caves.
//! All of it runs on the host; clients only draw what they are sent.

use serde::{Deserialize, Serialize};

use super::body::{self, Body, creature_solid, line_of_sight};
use super::geom::{V2, v2};
use super::items::Item;
use super::{Bolt, Colony, Ent, EntKind, Fx, Id, PlayerKey};
use crate::material::Kind;
use crate::planetgen::{Band, FeatureKind};
use crate::world::World;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum CreatureKind {
    Driftmoth,
    Puffback,
    Skitter,
    Webspinner,
    GloomLeech,
    BurrowMaw,
    BroodMother,
}

impl CreatureKind {
    pub const ALL: [CreatureKind; 7] = [
        CreatureKind::Driftmoth,
        CreatureKind::Puffback,
        CreatureKind::Skitter,
        CreatureKind::Webspinner,
        CreatureKind::GloomLeech,
        CreatureKind::BurrowMaw,
        CreatureKind::BroodMother,
    ];

    pub fn name(self) -> &'static str {
        match self {
            CreatureKind::Driftmoth => "Driftmoth",
            CreatureKind::Puffback => "Puffback",
            CreatureKind::Skitter => "Skitter",
            CreatureKind::Webspinner => "Webspinner",
            CreatureKind::GloomLeech => "Gloom Leech",
            CreatureKind::BurrowMaw => "Burrow Maw",
            CreatureKind::BroodMother => "Brood Mother",
        }
    }

    pub fn describe(self) -> &'static str {
        match self {
            CreatureKind::Driftmoth => "Harmless. Drawn to plants and lights.",
            CreatureKind::Puffback => "A harmless grazer with a garden on its back. Drops bio-gel.",
            CreatureKind::Skitter => "Climbs walls and ceilings, then lunges. Comes to the surface at night.",
            CreatureKind::Webspinner => "Keeps its distance and spits webs that slow you.",
            CreatureKind::GloomLeech => {
                "Latches on and drains your oxygen. Shoot it or it lets go when full."
            }
            CreatureKind::BurrowMaw => {
                "Hides in the ground and erupts under you. Watch for the feeler and the rumble."
            }
            CreatureKind::BroodMother => "Guards the aurorium of the Abyss and never stops laying.",
        }
    }

    pub fn predator(self) -> bool {
        !matches!(self, CreatureKind::Driftmoth | CreatureKind::Puffback)
    }

    pub fn max_hp(self) -> f32 {
        match self {
            CreatureKind::Driftmoth => 5.0,
            CreatureKind::Puffback => 40.0,
            CreatureKind::Skitter => 30.0,
            CreatureKind::Webspinner => 45.0,
            CreatureKind::GloomLeech => 20.0,
            CreatureKind::BurrowMaw => 80.0,
            CreatureKind::BroodMother => 420.0,
        }
    }

    /// (half width, height) of the collision box.
    pub fn size(self) -> (f32, f32) {
        match self {
            CreatureKind::Driftmoth => (3.0, 6.0),
            CreatureKind::Puffback => (7.0, 10.0),
            CreatureKind::Skitter => (7.0, 9.0),
            CreatureKind::Webspinner => (7.0, 14.0),
            CreatureKind::GloomLeech => (5.0, 6.0),
            CreatureKind::BurrowMaw => (8.0, 20.0),
            CreatureKind::BroodMother => (24.0, 36.0),
        }
    }

    fn flies(self) -> bool {
        matches!(self, CreatureKind::Driftmoth | CreatureKind::GloomLeech)
    }

    fn contact_damage(self) -> f32 {
        match self {
            CreatureKind::Skitter => 10.0,
            CreatureKind::Webspinner => 8.0,
            CreatureKind::BroodMother => 24.0,
            _ => 0.0,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum State {
    Idle,
    Chase,
    /// Winding up or mid-attack (a skitter's lunge, a maw erupting).
    Attack,
    /// A burrow maw under the ground.
    Hidden,
    /// A leech on a player.
    Latched,
    Dying,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Creature {
    pub kind: CreatureKind,
    pub hp: f32,
    pub vel: V2,
    pub facing: i8,
    pub state: State,
    /// Counts down within the current state.
    pub timer: f32,
    /// Seconds until it may hurt someone again.
    pub cooldown: f32,
    pub target: Option<PlayerKey>,
    /// Flashes briefly when hit.
    pub hit_flash: f32,
    pub on_ground: bool,
    /// Seconds of cold left: it moves and thinks at half speed.
    pub chilled: f32,
}

impl Creature {
    pub fn new(kind: CreatureKind) -> Creature {
        Creature {
            kind,
            hp: kind.max_hp(),
            vel: V2::ZERO,
            facing: 1,
            state: if kind == CreatureKind::BurrowMaw {
                State::Hidden
            } else {
                State::Idle
            },
            timer: 0.0,
            cooldown: 0.0,
            target: None,
            hit_flash: 0.0,
            on_ground: false,
            chilled: 0.0,
        }
    }

    /// State and facing packed for the motion stream.
    pub fn anim(&self) -> u8 {
        let state = match self.state {
            State::Idle => 0,
            State::Chase => 1,
            State::Attack => 2,
            State::Hidden => 3,
            State::Latched => 4,
            State::Dying => 5,
        };
        state | if self.facing < 0 { 0x80 } else { 0 } | if self.hit_flash > 0.0 { 0x40 } else { 0 }
    }

    pub fn set_anim(&mut self, a: u8) {
        self.state = match a & 0x0F {
            1 => State::Chase,
            2 => State::Attack,
            3 => State::Hidden,
            4 => State::Latched,
            5 => State::Dying,
            _ => State::Idle,
        };
        self.facing = if a & 0x80 != 0 { -1 } else { 1 };
        self.hit_flash = if a & 0x40 != 0 { 0.1 } else { 0.0 };
    }
}

const ACTIVE_RANGE: f32 = 340.0;
const DESPAWN_RANGE: f32 = 460.0;

/// Damages the first creature within `radius` of `at`. Returns true on a hit.
pub fn damage_at(colony: &mut Colony, at: V2, radius: f32, damage: f32, by: Option<PlayerKey>) -> bool {
    let cryo = by
        .and_then(|k| colony.players.get(&k))
        .map_or(0, |p| p.stats().cryo);
    let hit = colony.ents.values_mut().find_map(|e| {
        let EntKind::Creature(c) = &mut e.kind else {
            return None;
        };
        if c.state == State::Dying || c.state == State::Hidden {
            return None;
        }
        let (hw, h) = c.kind.size();
        let center = v2(e.pos.x, e.pos.y - h / 2.0);
        (center.distance(at) < radius + hw.max(h / 2.0)).then(|| {
            // Cryo Palms: chilled creatures are brittle, and hits chill.
            c.hp -= damage * if cryo >= 3 && c.chilled > 0.0 { 1.5 } else { 1.0 };
            if cryo >= 1 {
                c.chilled = 4.0;
            }
            c.hit_flash = 0.15;
            if c.target.is_none() {
                c.target = by;
            }
            // Knock it back a little.
            c.vel += (center - at).normalized() * 40.0;
            (e.id, center, c.hp <= 0.0, c.kind)
        })
    });
    let Some((id, center, dead, kind)) = hit else {
        return false;
    };
    colony.fx(Fx::CreatureHit, center);
    if dead {
        kill(colony, id, center, kind, by);
    }
    true
}

fn kill(colony: &mut Colony, id: Id, at: V2, kind: CreatureKind, by: Option<PlayerKey>) {
    if let Some(Ent {
        kind: EntKind::Creature(c),
        ..
    }) = colony.ents.get_mut(&id)
    {
        c.state = State::Dying;
        c.timer = 0.5;
        c.vel = V2::ZERO;
    }
    colony.touch(id);
    colony.fx(Fx::CreatureDie(kind), at);
    let r = colony.rng.next_u8();
    let drops: &[(Item, u32)] = match kind {
        CreatureKind::Driftmoth => &[],
        CreatureKind::Puffback => &[(Item::BioGel, 2), (Item::Fiber, 3)],
        CreatureKind::Skitter if r < 128 => &[(Item::Chitin, 1), (Item::Silk, 1)],
        CreatureKind::Skitter => &[(Item::Chitin, 1)],
        CreatureKind::Webspinner => &[(Item::Silk, 3)],
        CreatureKind::GloomLeech => &[(Item::BioGel, 2)],
        CreatureKind::BurrowMaw => &[(Item::Chitin, 4), (Item::Xenite, 2)],
        CreatureKind::BroodMother => &[(Item::Aurorium, 6), (Item::Chitin, 8), (Item::Silk, 6)],
    };
    // Appraisal Monocle: the killer's eye for loot.
    let (loot, rare) = by
        .and_then(|k| colony.players.get(&k))
        .map_or((1.0, false), |p| (p.stats().loot, p.stats().rare_loot));
    for &(item, count) in drops {
        let n = count as f32 * loot;
        let extra = (colony.rng.next_f32() < n.fract()) as u32;
        colony.drop_item(at, item, n as u32 + extra);
    }
    if rare && kind.predator() && colony.rng.next_f32() < 0.25 {
        let item = if colony.rng.next_f32() < 0.5 {
            Item::Gold
        } else {
            Item::Xenite
        };
        colony.drop_item(at, item, 1);
    }
}

pub fn step(colony: &mut Colony, world: &mut World, dt: f32) {
    // Camo Skin: (key, where, can't be hunted, noticed from half as far).
    let players: Vec<(PlayerKey, V2, bool, bool)> = colony
        .online()
        .filter(|p| p.alive())
        .map(|p| {
            let stealth = p.stats().stealth;
            let unseen = stealth >= 3 || (stealth == 2 && p.pose.vel.length() < 2.0);
            (p.key, p.center(), p.in_dome.is_some() || unseen, stealth >= 1)
        })
        .collect();
    let ids: Vec<Id> = colony
        .ents
        .values()
        .filter(|e| matches!(e.kind, EntKind::Creature(_)))
        .map(|e| e.id)
        .collect();
    for id in ids {
        let Some(Ent {
            pos,
            kind: EntKind::Creature(c),
            ..
        }) = colony.ents.get(&id).cloned()
        else {
            continue;
        };
        let nearest = players
            .iter()
            .map(|&(k, p, safe, half)| {
                let d = p.distance(pos);
                (k, p, safe || (half && d > 75.0), d)
            })
            .min_by(|a, b| a.3.total_cmp(&b.3));
        let dist = nearest.map_or(f32::MAX, |n| n.3);
        if dist > DESPAWN_RANGE && c.kind != CreatureKind::BroodMother {
            colony.despawn(id);
            continue;
        }
        if dist > ACTIVE_RANGE {
            continue;
        }
        let mut c = c;
        let mut pos = pos;
        let before = (c.state, c.facing, c.hit_flash > 0.0);
        c.chilled = (c.chilled - dt).max(0.0);
        let dt = if c.chilled > 0.0 { dt * 0.5 } else { dt };
        let alive = think(colony, world, id, &mut c, &mut pos, nearest, dt);
        if !alive {
            colony.despawn(id);
            continue;
        }
        let changed = before != (c.state, c.facing, c.hit_flash > 0.0);
        if let Some(e) = colony.ents.get_mut(&id) {
            e.pos = pos;
            // Health may have changed underneath us (a bolt hit this tick).
            if let EntKind::Creature(live) = &e.kind {
                c.hp = c.hp.min(live.hp);
                if live.state == State::Dying && c.state != State::Dying {
                    c.state = State::Dying;
                    c.timer = live.timer;
                }
            }
            e.kind = EntKind::Creature(c);
        }
        if changed {
            colony.touch(id);
        }
    }
}

/// One creature's behaviour for a tick. Returns false when it should vanish.
fn think(
    colony: &mut Colony,
    world: &mut World,
    id: Id,
    c: &mut Creature,
    pos: &mut V2,
    nearest: Option<(PlayerKey, V2, bool, f32)>,
    dt: f32,
) -> bool {
    c.timer -= dt;
    c.cooldown = (c.cooldown - dt).max(0.0);
    c.hit_flash = (c.hit_flash - dt).max(0.0);
    if c.state == State::Dying {
        return c.timer > 0.0;
    }
    let (hw, h) = c.kind.size();
    let center = v2(pos.x, pos.y - h / 2.0);
    // Predators notice players they can reach; nobody hunts inside a dome.
    let prey = nearest.filter(|&(_, p, safe, d)| {
        c.kind.predator()
            && !safe
            && d < 150.0
            && (d < 50.0 || line_of_sight(world, center, p, creature_solid))
    });
    let mut body = Body::new(*pos, hw, h);
    body.vel = c.vel;
    body.on_ground = c.on_ground;

    match c.kind {
        CreatureKind::Driftmoth => {
            if c.timer <= 0.0 {
                c.timer = 0.6 + colony.rng.next_f32();
                let a = colony.rng.next_f32() * std::f32::consts::TAU;
                c.vel = v2(a.cos() * 26.0, a.sin() * 16.0 - 4.0);
            }
            body.vel = c.vel;
        }
        CreatureKind::Puffback => {
            if c.timer <= 0.0 {
                c.timer = 2.0 + colony.rng.next_f32() * 3.0;
                c.state = if colony.rng.coin() {
                    State::Idle
                } else {
                    State::Chase
                };
                c.facing = if colony.rng.coin() { 1 } else { -1 };
            }
            // Shy: trots away from players that come close.
            if let Some((_, p, _, d)) = nearest
                && d < 40.0
            {
                c.state = State::Chase;
                c.facing = if p.x < pos.x { 1 } else { -1 };
            }
            body.vel.x = if c.state == State::Chase {
                c.facing as f32 * 22.0
            } else {
                0.0
            };
            body.vel.y = (body.vel.y + 400.0 * dt).min(260.0);
        }
        CreatureKind::Skitter | CreatureKind::BroodMother => {
            let speed = if c.kind == CreatureKind::Skitter {
                62.0
            } else {
                20.0
            };
            match (c.state, prey) {
                (State::Attack, _) => {
                    // Mid-lunge: ballistic until the timer runs out.
                    body.vel.y = (body.vel.y + 400.0 * dt).min(260.0);
                    if c.timer <= 0.0 {
                        c.state = State::Chase;
                    }
                }
                (_, Some((key, p, _, d))) => {
                    c.state = State::Chase;
                    c.target = Some(key);
                    c.facing = if p.x > pos.x { 1 } else { -1 };
                    body.vel.x = c.facing as f32 * speed;
                    body.vel.y = (body.vel.y + 400.0 * dt).min(260.0);
                    if c.kind == CreatureKind::Skitter {
                        if d < 44.0 && c.cooldown <= 0.0 && body.on_ground {
                            // Lunge.
                            c.state = State::Attack;
                            c.timer = 0.45;
                            let to = (p - center).normalized();
                            body.vel = v2(to.x * 170.0, to.y * 120.0 - 90.0);
                            colony.fx(Fx::CreatureCall(c.kind), center);
                        }
                    } else if c.timer <= 0.0 {
                        // The Brood Mother lays while she has someone to chase.
                        c.timer = 6.0;
                        let brood = count_near(colony, *pos, 260.0, |k| k == CreatureKind::Skitter);
                        if brood < 6 {
                            colony.fx(Fx::CreatureCall(c.kind), center);
                            let at = v2(pos.x - c.facing as f32 * 20.0, pos.y - 6.0);
                            colony.spawn(at, EntKind::Creature(Creature::new(CreatureKind::Skitter)));
                        }
                    }
                }
                _ => {
                    // Wander.
                    if c.timer <= 0.0 {
                        c.timer = 1.5 + colony.rng.next_f32() * 2.5;
                        c.state = if colony.rng.chance(150) {
                            State::Chase
                        } else {
                            State::Idle
                        };
                        c.facing = if colony.rng.coin() { 1 } else { -1 };
                    }
                    body.vel.x = if c.state == State::Chase {
                        c.facing as f32 * speed * 0.4
                    } else {
                        0.0
                    };
                    body.vel.y = (body.vel.y + 400.0 * dt).min(260.0);
                }
            }
        }
        CreatureKind::Webspinner => {
            body.vel.y = (body.vel.y + 400.0 * dt).min(260.0);
            match prey {
                Some((key, p, _, d)) => {
                    c.target = Some(key);
                    c.facing = if p.x > pos.x { 1 } else { -1 };
                    // Hold at spitting distance.
                    let want = if d < 60.0 {
                        -1.0
                    } else if d > 100.0 {
                        1.0
                    } else {
                        0.0
                    };
                    body.vel.x = c.facing as f32 * want * 34.0;
                    if c.state == State::Attack {
                        if c.timer <= 0.0 {
                            c.state = State::Chase;
                            let from = v2(pos.x + c.facing as f32 * 6.0, pos.y - h + 2.0);
                            let dir = (p - from).normalized();
                            colony.spawn(
                                from,
                                EntKind::Bolt(Bolt {
                                    vel: dir * 150.0,
                                    damage: 6.0,
                                    size: 1.0,
                                    owner: None,
                                    life: 2.0,
                                    web: true,
                                }),
                            );
                        }
                    } else if c.cooldown <= 0.0 && d < 130.0 {
                        c.state = State::Attack;
                        c.timer = 0.5;
                        c.cooldown = 2.6;
                        colony.fx(Fx::CreatureCall(c.kind), center);
                    } else {
                        c.state = State::Chase;
                    }
                }
                None => {
                    c.state = State::Idle;
                    body.vel.x = 0.0;
                }
            }
        }
        CreatureKind::GloomLeech => {
            if c.state == State::Latched {
                // Ride the player, draining their tank, then drop off.
                let host = c
                    .target
                    .and_then(|k| colony.players.get(&k))
                    .filter(|p| p.alive() && p.in_dome.is_none());
                match host {
                    Some(p) if c.timer > 0.0 => {
                        let key = p.key;
                        *pos = v2(p.pose.pos.x, p.pose.pos.y - 8.0);
                        let p = colony.players.get_mut(&key).unwrap();
                        p.o2 = (p.o2 - 7.0 * dt).max(0.0);
                        colony.hurt(key, 2.5 * dt);
                        c.vel = V2::ZERO;
                        return true;
                    }
                    _ => {
                        c.state = State::Idle;
                        c.cooldown = 4.0;
                        c.target = None;
                    }
                }
            }
            let in_water = world.material(center.x as i32, center.y as i32).kind() == Kind::Liquid;
            let speed = if in_water { 58.0 } else { 30.0 };
            match prey {
                Some((key, p, _, d)) if c.cooldown <= 0.0 => {
                    c.state = State::Chase;
                    let to = (p - center).normalized();
                    c.vel = c.vel.lerp(to * speed, (dt * 3.0).min(1.0));
                    c.facing = if to.x >= 0.0 { 1 } else { -1 };
                    if d < 9.0 {
                        c.state = State::Latched;
                        c.target = Some(key);
                        c.timer = 5.0;
                        colony.fx(Fx::CreatureCall(c.kind), center);
                    }
                }
                _ => {
                    c.state = State::Idle;
                    if c.timer <= 0.0 {
                        c.timer = 1.0 + colony.rng.next_f32() * 2.0;
                        let a = colony.rng.next_f32() * std::f32::consts::TAU;
                        c.vel = v2(a.cos() * 14.0, a.sin() * 8.0);
                    }
                }
            }
            body.vel = c.vel;
        }
        CreatureKind::BurrowMaw => {
            c.vel = V2::ZERO;
            match c.state {
                State::Hidden => {
                    // Someone walking overhead sets it off.
                    let above = nearest.filter(|&(_, p, safe, _)| {
                        !safe && (p.x - pos.x).abs() < 26.0 && p.y < pos.y + 6.0 && p.y > pos.y - 50.0
                    });
                    if let Some((key, _, _, _)) = above
                        && c.cooldown <= 0.0
                    {
                        c.state = State::Attack;
                        c.timer = 0.9;
                        c.target = Some(key);
                        colony.fx(Fx::CreatureCall(c.kind), center);
                    }
                }
                State::Attack => {
                    if c.timer <= 0.0 {
                        // Erupt: bite whoever is still standing there.
                        c.state = State::Chase;
                        c.timer = 3.5;
                        let victims: Vec<PlayerKey> = colony
                            .online()
                            .filter(|p| {
                                p.alive()
                                    && (p.pose.pos.x - pos.x).abs() < 16.0
                                    && (p.pose.pos.y - pos.y).abs() < 30.0
                            })
                            .map(|p| p.key)
                            .collect();
                        for key in victims {
                            colony.hurt(key, 30.0);
                        }
                        // Bursting out throws up the ground above it.
                        for (x, y, m) in world.dig(pos.x as i32, pos.y as i32 - 8, 7, 200) {
                            world.spawn_particle(x as f32, y as f32, (x as f32 - pos.x) * 0.15, -2.0, m);
                        }
                    }
                }
                _ => {
                    // Exposed for a while, then it sinks back and waits.
                    if c.timer <= 0.0 {
                        c.state = State::Hidden;
                        c.cooldown = 4.0;
                    }
                }
            }
            colony.touch(id);
            return true;
        }
    }

    // Move.
    if c.kind.flies() {
        body.step(world, dt, 0, creature_solid);
        if body.wall != 0 || body.ceiling || body.on_ground {
            c.timer = 0.0;
        }
    } else {
        if !body.escape_overlap(world, creature_solid) {
            body.pos.y -= 2.0;
        }
        body.step(world, dt, 4, creature_solid);
        // Skitters go straight up walls after their prey.
        if body.wall != 0 && c.kind == CreatureKind::Skitter && c.state == State::Chase && prey.is_some() {
            body.vel.y = -70.0;
        } else if body.wall != 0 && c.state == State::Chase && prey.is_none() {
            c.facing = -c.facing;
        }
    }
    *pos = body.pos;
    c.vel = body.vel;
    c.on_ground = body.on_ground;

    // Touching a player hurts.
    let touch = c.kind.contact_damage();
    if touch > 0.0
        && c.cooldown <= 0.0
        && let Some((key, p, safe, _)) = nearest
        && !safe
        && (p.x - pos.x).abs() < hw + 5.0
        && (p.y - (pos.y - h / 2.0)).abs() < h / 2.0 + 10.0
    {
        colony.hurt(key, touch);
        c.cooldown = 1.1;
    }
    let _ = body::player_solid;
    true
}

fn count_near(colony: &Colony, at: V2, radius: f32, pick: impl Fn(CreatureKind) -> bool) -> usize {
    colony
        .ents
        .values()
        .filter(|e| {
            matches!(&e.kind, EntKind::Creature(c) if pick(c.kind) && c.state != State::Dying)
                && e.pos.distance(at) < radius
        })
        .count()
}

/// A free spot for a creature of this size near `at`, standing on ground
/// (or anywhere open for fliers).
fn find_spot(
    colony: &mut Colony,
    world: &World,
    around: V2,
    kind: CreatureKind,
    lo: f32,
    hi: f32,
) -> Option<V2> {
    let (hw, h) = kind.size();
    for _ in 0..24 {
        let a = colony.rng.next_f32() * std::f32::consts::TAU;
        let r = lo + colony.rng.next_f32() * (hi - lo);
        let mut p = v2(around.x + a.cos() * r, around.y + a.sin() * r * 0.6);
        let body = Body::new(p, hw, h);
        if body.collides(world, p, creature_solid) {
            continue;
        }
        if !kind.flies() {
            // Drop to the floor.
            let mut fell = false;
            for _ in 0..80 {
                if body.collides(world, v2(p.x, p.y + 1.0), creature_solid) {
                    fell = true;
                    break;
                }
                p.y += 1.0;
            }
            if !fell {
                continue;
            }
        }
        if colony.dome_at(v2(p.x, p.y - 2.0)).is_some() {
            continue;
        }
        if kind == CreatureKind::GloomLeech
            && world.material(p.x as i32, p.y as i32 - 2).kind() != Kind::Liquid
            && colony.rng.coin()
        {
            continue;
        }
        return Some(p);
    }
    None
}

/// Keeps the area around each player populated according to where they are.
pub fn spawn_tick(colony: &mut Colony, world: &World) {
    let players: Vec<V2> = colony
        .online()
        .filter(|p| p.alive())
        .map(|p| p.center())
        .collect();
    let night = colony.clock.is_night();
    for at in players {
        let (x, y) = at.cell();
        let band = colony.profile.band(x, y + 20);
        let underground = y > colony.profile.surface_at(x) + 30;
        let (cap, table): (usize, &[CreatureKind]) = match (band, underground) {
            (Band::Abyss, _) => (
                5,
                &[
                    CreatureKind::BurrowMaw,
                    CreatureKind::Skitter,
                    CreatureKind::BurrowMaw,
                    CreatureKind::GloomLeech,
                ],
            ),
            (Band::Deeps, _) => (
                5,
                &[
                    CreatureKind::Webspinner,
                    CreatureKind::GloomLeech,
                    CreatureKind::Skitter,
                    CreatureKind::Webspinner,
                ],
            ),
            (_, true) => (3, &[CreatureKind::Skitter]),
            _ if night => (2, &[CreatureKind::Skitter]),
            _ => (0, &[]),
        };
        let predators = count_near(colony, at, 300.0, |k| k.predator());
        if predators < cap && !table.is_empty() && colony.rng.chance(150) {
            let kind = table[colony.rng.range(0, table.len() as i32) as usize];
            if let Some(p) = find_spot(colony, world, at, kind, 170.0, 280.0) {
                colony.spawn(p, EntKind::Creature(Creature::new(kind)));
            }
        }
        // Gentle life on the surface by day.
        if !underground && !night {
            let fauna = count_near(colony, at, 300.0, |k| !k.predator());
            if fauna < 5 && colony.rng.chance(120) {
                let kind = if colony.rng.chance(170) {
                    CreatureKind::Driftmoth
                } else {
                    CreatureKind::Puffback
                };
                let ground = v2(at.x, colony.profile.surface_at(x) as f32 - 30.0);
                if let Some(p) = find_spot(colony, world, ground, kind, 120.0, 260.0) {
                    colony.spawn(p, EntKind::Creature(Creature::new(kind)));
                }
            }
        }
    }
    // Each lair's Brood Mother appears when someone first comes near.
    for f in colony.features.clone() {
        if f.kind != FeatureKind::Lair {
            continue;
        }
        let lair = v2(f.x as f32, f.y as f32);
        let near = colony.online().any(|p| p.center().distance(lair) < 320.0);
        let present = count_near(colony, lair, 400.0, |k| k == CreatureKind::BroodMother) > 0;
        if near && !present && !colony.cleared_lairs.contains(&(f.x, f.y)) {
            colony.cleared_lairs.push((f.x, f.y));
            colony.spawn(lair, EntKind::Creature(Creature::new(CreatureKind::BroodMother)));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colony::testing::*;
    use crate::material::Material;

    /// A flat arena far from the dome with the player standing in it.
    fn arena(colony: &mut Colony, world: &mut World, key: PlayerKey) -> V2 {
        let x0 = colony.profile.spawn_x + 400;
        let y = 300;
        for cy in y - 120..y + 40 {
            for cx in x0 - 200..x0 + 200 {
                world.set(cx, cy, if cy >= y { Material::Stone } else { Material::Empty });
            }
        }
        let at = v2(x0 as f32, y as f32);
        colony.players.get_mut(&key).unwrap().pose.pos = at;
        // Underground as far as the colony is concerned: no day fauna.
        for s in colony.profile.surface.iter_mut() {
            *s = 100;
        }
        at
    }

    fn creature(colony: &Colony, id: Id) -> Option<(&Ent, &Creature)> {
        let e = colony.ents.get(&id)?;
        match &e.kind {
            EntKind::Creature(c) => Some((e, c)),
            _ => None,
        }
    }

    #[test]
    fn skitters_chase_lunge_and_die_to_bolts() {
        let (mut world, mut colony, key) = colony();
        let at = arena(&mut colony, &mut world, key);
        let id = colony.spawn(
            v2(at.x + 100.0, at.y),
            EntKind::Creature(Creature::new(CreatureKind::Skitter)),
        );
        let hp0 = colony.players[&key].hp;
        for _ in 0..(8.0 * 60.0) as usize {
            colony.step(&mut world, 1.0 / 60.0);
            // Keep the spawner from adding more.
            colony.timers.spawner = 0.0;
        }
        let (e, _) = creature(&colony, id).unwrap();
        assert!(e.pos.distance(at) < 60.0, "it closed in: {:?}", e.pos);
        assert!(colony.players[&key].hp < hp0, "and bit the player");
        // Two bolts kill it and it drops chitin.
        let c = creature(&colony, id).unwrap().0.pos + v2(0.0, -4.0);
        assert!(damage_at(&mut colony, c, 5.0, 20.0, Some(key)));
        assert!(damage_at(&mut colony, c, 5.0, 20.0, Some(key)));
        assert_eq!(creature(&colony, id).unwrap().1.state, State::Dying);
        run(&mut colony, &mut world, 1.0);
        assert!(creature(&colony, id).is_none());
        let has_chitin = colony.players[&key].inv.count(Item::Chitin) > 0
            || colony
                .ents
                .values()
                .any(|e| matches!(&e.kind, EntKind::Drop(d) if d.item == Item::Chitin));
        assert!(has_chitin);
    }

    #[test]
    fn domes_keep_predators_out() {
        let (mut world, mut colony, key) = colony();
        let home = colony.ents[&colony.home].pos;
        // A skitter right outside the starter dome, player inside.
        colony.players.get_mut(&key).unwrap().pose.pos = v2(home.x + 60.0, home.y);
        let id = colony.spawn(
            v2(home.x + 140.0, home.y),
            EntKind::Creature(Creature::new(CreatureKind::Skitter)),
        );
        colony.clock.time = 100.0;
        for _ in 0..(10.0 * 60.0) as usize {
            colony.step(&mut world, 1.0 / 60.0);
            colony.timers.spawner = 0.0;
        }
        assert_eq!(colony.players[&key].hp, 100.0);
        if let Some((e, _)) = creature(&colony, id) {
            assert!(
                colony.dome_at(v2(e.pos.x, e.pos.y - 4.0)).is_none(),
                "it stayed outside"
            );
        }
    }

    #[test]
    fn webs_slow_and_leeches_drain_oxygen() {
        let (mut world, mut colony, key) = colony();
        let at = arena(&mut colony, &mut world, key);
        colony.spawn(
            v2(at.x + 90.0, at.y),
            EntKind::Creature(Creature::new(CreatureKind::Webspinner)),
        );
        let mut slowed = false;
        for _ in 0..(12.0 * 60.0) as usize {
            colony.step(&mut world, 1.0 / 60.0);
            colony.timers.spawner = 0.0;
            slowed |= colony.players[&key].slowed > 0.0;
        }
        assert!(slowed, "a web should have landed");

        let (mut world, mut colony, key) = colony_pair();
        let at = arena(&mut colony, &mut world, key);
        let id = colony.spawn(
            v2(at.x + 30.0, at.y - 12.0),
            EntKind::Creature(Creature::new(CreatureKind::GloomLeech)),
        );
        let mut latched = false;
        for _ in 0..(6.0 * 60.0) as usize {
            colony.step(&mut world, 1.0 / 60.0);
            colony.timers.spawner = 0.0;
            latched |= creature(&colony, id).is_some_and(|(_, c)| c.state == State::Latched);
        }
        assert!(latched);
        assert!(colony.players[&key].o2 < 80.0, "o2 {}", colony.players[&key].o2);
    }

    fn colony_pair() -> (World, Colony, PlayerKey) {
        colony()
    }

    #[test]
    fn burrow_maw_erupts_under_players() {
        let (mut world, mut colony, key) = colony();
        let at = arena(&mut colony, &mut world, key);
        // Buried just under the floor the player stands on.
        let id = colony.spawn(
            v2(at.x + 4.0, at.y + 14.0),
            EntKind::Creature(Creature::new(CreatureKind::BurrowMaw)),
        );
        assert_eq!(creature(&colony, id).unwrap().1.state, State::Hidden);
        assert!(
            !damage_at(&mut colony, v2(at.x + 4.0, at.y + 6.0), 6.0, 50.0, None),
            "safe while hidden"
        );
        let mut called = false;
        for _ in 0..(2.0 * 60.0) as usize {
            colony.step(&mut world, 1.0 / 60.0);
            colony.timers.spawner = 0.0;
            called |= colony.take_events().iter().any(|e| {
                matches!(
                    e,
                    crate::colony::Event::Fx {
                        fx: Fx::CreatureCall(CreatureKind::BurrowMaw),
                        ..
                    }
                )
            });
        }
        assert!(called, "it warns before erupting");
        assert!(colony.players[&key].hp <= 70.0);
        assert_eq!(
            creature(&colony, id).unwrap().1.state,
            State::Chase,
            "exposed after erupting"
        );
    }

    #[test]
    fn spawner_respects_depth_and_caps() {
        let (mut world, mut colony, key) = colony();
        let at = arena(&mut colony, &mut world, key);
        let _ = at;
        for _ in 0..200 {
            spawn_tick(&mut colony, &world);
        }
        let predators = count_near(&colony, colony.players[&key].center(), 400.0, |k| k.predator());
        assert!((1..=3).contains(&predators), "shallows cap is 3, got {predators}");
        assert_eq!(
            count_near(&colony, colony.players[&key].center(), 4000.0, |k| k
                == CreatureKind::BroodMother),
            0
        );
        assert_eq!(
            count_near(&colony, colony.players[&key].center(), 400.0, |k| !k.predator()),
            0,
            "no day fauna underground"
        );
    }
}
