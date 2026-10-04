//! Casting: mana, cooldowns, and what every active scroll and fusion does.
//! Each effect is built from simulation operations (convert cells, fling
//! cells as particles, electrify, ignite, spawn liquids or gas), so spells
//! combine with the world, and each other, the way materials do.

use bevy::prelude::*;
use sbct_sim::material::{INDESTRUCTIBLE, MAX_CHARGE};
use sbct_sim::world::disc;
use sbct_sim::{Kind, Material, World};

use super::entities::{Bomb, BombKind, Projectile, Sun};
use super::input::PlayerInput;
use super::physics::default_solid;
use super::player::{REACH, RunPlayer};
use super::scrolls::{FusionId, School, ScrollId};
use super::{Run, Spell};
use crate::assets::GameAssets;
use crate::audio::Sfx;
use crate::fx::{Burst, Shake};
use crate::session::Session;

/// Mana regained per second.
pub const MANA_REGEN: f32 = 8.0;
/// How far targeted spells reach.
pub const SPELL_RANGE: f32 = 48.0;

/// What a spell does to creatures in an area.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Element {
    Fire,
    Frost,
    Electric,
    Acid,
    Steam,
    /// Stuns (petrify, burial).
    Stone,
    Blunt,
}

/// Damage to creatures in a radius, handled by the creature system.
#[derive(Message, Clone, Copy, Debug)]
pub struct Harm {
    pub at: Vec2,
    pub radius: f32,
    pub damage: f32,
    pub element: Element,
    /// Knockback speed away from `at`.
    pub push: f32,
}

/// Everything an effect may touch.
pub struct Ctx<'a, 'w, 's> {
    pub world: &'a mut World,
    pub commands: &'a mut Commands<'w, 's>,
    pub assets: &'a GameAssets,
    pub run: &'a mut Run,
    pub player: &'a mut RunPlayer,
    /// Messages produced by the effect; the calling system sends them.
    pub harm: Vec<Harm>,
    pub bursts: Vec<Burst>,
    pub sfx: Vec<Sfx>,
    pub shake: &'a mut Shake,
    /// Positions of nearby creatures (for chaining spells).
    pub creatures: &'a [Vec2],
}

impl Ctx<'_, '_, '_> {
    /// Sends the messages the effect produced.
    pub fn flush(
        self,
        harm: &mut MessageWriter<Harm>,
        bursts: &mut MessageWriter<Burst>,
        sfx: &mut MessageWriter<Sfx>,
    ) {
        harm.write_batch(self.harm);
        bursts.write_batch(self.bursts);
        sfx.write_batch(self.sfx);
    }

    fn rng(&mut self) -> f32 {
        self.run.rng.next_f32()
    }

    /// Replaces cells in a disc according to `f`; returns how many changed.
    fn convert(&mut self, at: Vec2, r: i32, f: impl Fn(Material) -> Option<Material>) -> usize {
        let mut n = 0;
        for (x, y) in disc(at.x as i32, at.y as i32, r) {
            let m = self.world.material(x, y);
            if m.props().hardness == INDESTRUCTIBLE && m != Material::Empty {
                continue;
            }
            if let Some(to) = f(m)
                && to != m
            {
                let life = if to == Material::Gravel {
                    0
                } else {
                    to.initial_life()
                };
                self.world.set_with_life(x, y, to, life);
                n += 1;
            }
        }
        n
    }

    /// Fills open cells (air, gas, fire) in a disc.
    fn fill(&mut self, at: Vec2, r: i32, mat: Material) -> usize {
        self.convert(at, r, |m| m.is_open().then_some(mat))
    }

    /// Places fungus and credits it to the Gardener achievement.
    fn grow(&mut self, x: i32, y: i32) {
        self.world.set(x, y, Material::Fungus);
        self.run.stats.fungus_grown += 1;
    }

    /// Cells along a line from `a` to `b`.
    fn line(a: Vec2, b: Vec2) -> Vec<(i32, i32)> {
        let n = a.distance(b).ceil().max(1.0) as i32;
        let mut cells: Vec<(i32, i32)> = (0..=n)
            .map(|i| a.lerp(b, i as f32 / n as f32))
            .map(|p| (p.x.floor() as i32, p.y.floor() as i32))
            .collect();
        cells.dedup();
        cells
    }

    /// Flings matching cells within `r` of `from` toward `dir` as particles.
    fn fling(&mut self, from: Vec2, r: i32, dir: Vec2, speed: f32, pred: impl Fn(Material) -> bool) -> usize {
        let mut n = 0;
        for (x, y) in disc(from.x as i32, from.y as i32, r) {
            let m = self.world.material(x, y);
            if m == Material::Empty || !pred(m) {
                continue;
            }
            let spread = (self.rng() - 0.5) * 0.6;
            let v = Vec2::from_angle(dir.to_angle() + spread) * speed * (0.7 + self.rng() * 0.6);
            self.world.set(x, y, Material::Empty);
            self.world
                .spawn_particle(x as f32 + 0.5, y as f32 + 0.5, v.x, v.y - 0.3, m);
            n += 1;
        }
        n
    }

    fn harm(&mut self, at: Vec2, radius: f32, damage: f32, element: Element, push: f32) {
        self.harm.push(Harm {
            at,
            radius,
            damage,
            element,
            push,
        });
    }

    fn burst(&mut self, at: Vec2, color: Color, count: u32, speed: f32) {
        self.bursts.push(
            Burst::new(at, color)
                .count(count)
                .speed(speed)
                .gravity(0.0)
                .life(0.5),
        );
    }

    fn projectile(&mut self, spell: Spell, from: Vec2, dir: Vec2, speed: f32, gravity: f32) {
        self.commands
            .spawn(Projectile::bundle(spell, from, dir * speed, gravity, self.assets));
    }
}

fn diggable(m: Material) -> bool {
    matches!(m.kind(), Kind::Solid | Kind::Powder) && m.props().hardness != INDESTRUCTIBLE
}

fn liquid(m: Material) -> bool {
    m.kind() == Kind::Liquid
}

fn school_color(s: Option<School>) -> Color {
    let [r, g, b] = s.map_or([220, 220, 230], |s| s.color());
    Color::srgb_u8(r, g, b)
}

/// Casts a spell from `tip` toward `aim`.
pub fn cast(spell: Spell, c: &mut Ctx, tip: Vec2, aim: Vec2) {
    let dir = (aim - tip).normalize_or(Vec2::new(c.player.facing, 0.0));
    let target = if aim.distance(tip) > SPELL_RANGE {
        tip + dir * SPELL_RANGE
    } else {
        aim
    };
    let feet = c.player.body.pos;
    let color = school_color(spell.school());
    c.burst(tip, color, 8, 40.0);
    match spell {
        Spell::Scroll(s) => match s {
            ScrollId::FireBolt => c.projectile(spell, tip, dir, 240.0, 0.0),
            ScrollId::FrostSeed => c.projectile(spell, tip, dir, 170.0, 260.0),
            ScrollId::IceLance => c.projectile(spell, tip, dir, 300.0, 0.0),
            ScrollId::SparkBolt => c.projectile(spell, tip, dir, 360.0, 0.0),
            ScrollId::AcidFlask => c.projectile(spell, tip, dir, 180.0, 260.0),
            ScrollId::SporeSowing => c.projectile(spell, tip, dir, 150.0, 200.0),
            ScrollId::PocketSun => {
                c.commands.spawn(Sun::bundle(target, c.assets));
            }
            ScrollId::AlchemistsCharge => {
                c.commands
                    .spawn(Bomb::bundle(target, BombKind::Blast(14), 2.5, c.assets));
            }
            ScrollId::FloodOrb => {
                c.fill(target, 6, Material::Water);
                c.burst(target, color, 20, 60.0);
            }
            ScrollId::StoneShape => {
                // A ledge, never inside the caster.
                let (x0, x1, y0, y1) = c.player.body.cells_at(c.player.body.pos);
                for dy in 0..2 {
                    for dx in -4..=4 {
                        let (x, y) = (target.x as i32 + dx, target.y as i32 + dy);
                        let inside = x >= x0 && x <= x1 && y >= y0 && y <= y1;
                        if !inside && c.world.material(x, y).is_open() {
                            c.world.set(x, y, Material::Stone);
                        }
                    }
                }
            }
            ScrollId::Gust => {
                for k in 1..5 {
                    let p = tip + dir * (k as f32 * 7.0);
                    c.fling(p, 4 + k, dir, 3.2, |m| {
                        matches!(m.kind(), Kind::Powder | Kind::Liquid | Kind::Gas | Kind::Fire)
                    });
                }
                c.harm(tip + dir * 15.0, 18.0, 6.0, Element::Blunt, 220.0);
            }
            ScrollId::Updraft => {
                c.player.body.vel.y = -270.0;
                c.fling(feet - Vec2::Y * 20.0, 10, Vec2::NEG_Y, 3.5, |m| {
                    m.kind() == Kind::Powder || liquid(m)
                });
                c.burst(feet, Color::srgba(0.85, 1.0, 0.9, 0.7), 20, 80.0);
            }
            ScrollId::ChainLightning => lightning(c, tip, target, false),
            ScrollId::Transmutation => {
                let n = c.convert(target, 6, |m| match m {
                    Material::Metal | Material::Ferrite | Material::Obsidian => Some(Material::ShardVein),
                    Material::Stone | Material::Basalt => Some(Material::Sand),
                    _ => None,
                });
                c.burst(target, Color::srgb(0.6, 1.0, 0.9), 10 + n.min(40) as u32, 50.0);
            }
            ScrollId::MycelialBridge => {
                let end = feet + (aim - feet).clamp_length_max(30.0);
                for (x, y) in Ctx::line(feet + Vec2::Y, end) {
                    for dy in 0..2 {
                        if c.world.material(x, y + dy).is_open() {
                            c.grow(x, y + dy);
                        }
                    }
                }
            }
            ScrollId::Blink => blink(c, aim),
            _ => {}
        },
        Spell::Fusion(f) => {
            c.sfx.push(Sfx::at("fusion_cast", tip));
            fusion(f, c, tip, dir, target);
        }
    }
}

/// One frame of a channelled spell (Tidecall).
pub fn channel(spell: Spell, c: &mut Ctx, tip: Vec2, aim: Vec2) {
    if spell == Spell::Scroll(ScrollId::Tidecall) {
        let dir = (aim - tip).normalize_or(Vec2::new(c.player.facing, 0.0));
        for i in 0..3 {
            let spread = (c.rng() - 0.5) * 0.5;
            let v = Vec2::from_angle(dir.to_angle() + spread) * (3.0 + i as f32 * 0.4);
            c.world.spawn_particle(tip.x, tip.y, v.x, v.y, Material::Water);
        }
    }
}

/// Lightning from `a` to `b`: charges conductors and ignites gas along the
/// path, then leaps between up to three creatures. Cryo Arc freezes instead.
fn lightning(c: &mut Ctx, a: Vec2, b: Vec2, freeze: bool) {
    let mut end = b;
    for (x, y) in Ctx::line(a, b) {
        let m = c.world.material(x, y);
        if freeze && liquid(m) {
            let to = if m == Material::Lava {
                Material::Obsidian
            } else {
                Material::Ice
            };
            for (fx, fy) in disc(x, y, 2) {
                if liquid(c.world.material(fx, fy)) {
                    c.world.set(fx, fy, to);
                }
            }
        }
        if m.props().conductive {
            c.world.electrify(x, y, MAX_CHARGE);
        }
        if m == Material::Gas {
            c.world.ignite(x, y);
        }
        if m.is_solid_for_player() && !freeze {
            end = Vec2::new(x as f32, y as f32);
            c.world.set(x, y - 1, Material::Spark);
            break;
        }
    }
    let element = if freeze { Element::Frost } else { Element::Electric };
    for (i, p) in Ctx::line(a, end).iter().enumerate().step_by(3) {
        if i % 2 == 0 {
            let jitter = Vec2::new(c.rng() - 0.5, c.rng() - 0.5) * 3.0;
            let col = if freeze {
                Color::srgb(0.75, 0.95, 1.0)
            } else {
                Color::srgb(0.85, 0.9, 1.0)
            };
            c.bursts.push(
                Burst::new(Vec2::new(p.0 as f32, p.1 as f32) + jitter, col)
                    .count(1)
                    .speed(5.0)
                    .gravity(0.0)
                    .life(0.25)
                    .size(1.5),
            );
        }
    }
    c.harm(end, 8.0, 30.0, element, 60.0);
    // Leap between nearby creatures.
    let mut from = end;
    let mut hit: Vec<Vec2> = Vec::new();
    for _ in 0..3 {
        let next = c
            .creatures
            .iter()
            .copied()
            .filter(|p| p.distance(from) < 40.0 && !hit.contains(p))
            .min_by(|x, y| x.distance(from).total_cmp(&y.distance(from)));
        let Some(next) = next else { break };
        c.harm(next, 6.0, 22.0, element, 40.0);
        for p in Ctx::line(from, next).iter().step_by(4) {
            c.bursts.push(
                Burst::new(Vec2::new(p.0 as f32, p.1 as f32), Color::srgb(0.85, 0.9, 1.0))
                    .count(1)
                    .speed(4.0)
                    .gravity(0.0)
                    .life(0.2),
            );
        }
        hit.push(next);
        from = next;
    }
    c.shake.add(0.15);
}

/// Steps toward the cursor to the farthest spot (up to 14 cells) the body fits.
fn blink(c: &mut Ctx, aim: Vec2) {
    let start = c.player.body.pos;
    let dir = (aim - c.player.body.center()).normalize_or(Vec2::new(c.player.facing, 0.0));
    for d in (2..=14).rev() {
        let p = start + dir * d as f32;
        if !c.player.body.collides(c.world, p, default_solid) {
            c.bursts.push(
                Burst::new(c.player.body.center(), Color::srgb(0.7, 0.6, 1.0))
                    .count(16)
                    .speed(50.0)
                    .gravity(0.0),
            );
            c.player.body.pos = p;
            c.player.body.vel = Vec2::ZERO;
            c.bursts.push(
                Burst::new(c.player.body.center(), Color::srgb(0.7, 0.6, 1.0))
                    .count(16)
                    .speed(50.0)
                    .gravity(0.0),
            );
            c.sfx.push(Sfx::at("blink", p));
            return;
        }
    }
}

fn fusion(f: FusionId, c: &mut Ctx, tip: Vec2, dir: Vec2, target: Vec2) {
    let feet = c.player.body.pos;
    match f {
        FusionId::SteamBurst => {
            c.convert(target, 8, |m| (m == Material::Water).then_some(Material::Steam));
            c.fill(target, 5, Material::Steam);
            c.harm(target, 9.0, 30.0, Element::Steam, 80.0);
        }
        FusionId::MagmaSurge => {
            c.convert(target, 5, |m| {
                (diggable(m) && m != Material::ShardVein).then_some(Material::Lava)
            });
            c.shake.add(0.2);
        }
        FusionId::ThermalShock => {
            c.convert(target, 7, |m| {
                matches!(
                    m,
                    Material::Stone
                        | Material::Basalt
                        | Material::Obsidian
                        | Material::Ice
                        | Material::Brick
                        | Material::Frost
                )
                .then_some(Material::Gravel)
            });
            c.fill(target, 3, Material::Steam);
            c.shake.add(0.3);
        }
        FusionId::Firestorm => c.projectile(Spell::Fusion(f), tip, dir, 110.0, 0.0),
        FusionId::PlasmaLance => {
            let end = tip + dir * 70.0;
            for (x, y) in Ctx::line(tip, end) {
                for (cx, cy) in disc(x, y, 1) {
                    let m = c.world.material(cx, cy);
                    if diggable(m) {
                        let to = if c.run.rng.chance(40) {
                            Material::Fire
                        } else {
                            Material::Empty
                        };
                        c.world.set(cx, cy, to);
                    } else if m.props().conductive {
                        c.world.electrify(cx, cy, MAX_CHARGE);
                    } else if m.props().flammability > 0 {
                        c.world.ignite(cx, cy);
                    }
                }
            }
            for k in 0..8 {
                c.harm(tip + dir * (k as f32 * 9.0), 6.0, 25.0, Element::Fire, 30.0);
            }
            c.shake.add(0.35);
        }
        FusionId::Napalm => c.projectile(Spell::Fusion(f), tip, dir, 170.0, 240.0),
        FusionId::SporeBomb => c.projectile(Spell::Fusion(f), tip, dir, 150.0, 220.0),
        FusionId::Mudslide => {
            let n = c.fling(target, 8, Vec2::new(dir.x, 0.6).normalize(), 2.2, |m| {
                matches!(
                    m,
                    Material::Dirt | Material::Sand | Material::Grass | Material::Gravel
                )
            });
            for _ in 0..n.min(60) / 2 {
                let v = Vec2::new(dir.x * 2.0 + (c.rng() - 0.5), -c.rng());
                c.world
                    .spawn_particle(target.x, target.y, v.x, v.y, Material::Water);
            }
            c.harm(target, 10.0, 12.0, Element::Blunt, 160.0);
        }
        FusionId::Glacier => {
            c.convert(target, 18, |m| match m {
                Material::Water | Material::Steam => Some(Material::Ice),
                _ => None,
            });
            c.convert(target, 6, |m| (m == Material::Lava).then_some(Material::Obsidian));
            c.harm(target, 18.0, 15.0, Element::Frost, 0.0);
        }
        FusionId::Typhoon => {
            let toward = (target - feet).normalize_or(dir);
            c.fling(feet, 16, toward, 3.6, liquid);
            c.harm(target, 14.0, 10.0, Element::Blunt, 200.0);
        }
        FusionId::StormFlood => {
            c.convert(target, 6, |m| m.is_open().then_some(Material::Water));
            for (x, y) in disc(target.x as i32, target.y as i32, 6) {
                if c.world.material(x, y) == Material::Water {
                    c.world.electrify(x, y, MAX_CHARGE);
                }
            }
            c.harm(target, 10.0, 25.0, Element::Electric, 40.0);
        }
        FusionId::AcidRain => {
            for i in 0..50 {
                let x = target.x + (c.rng() - 0.5) * 30.0;
                let y = target.y - 30.0 - i as f32 * 0.6;
                c.world.spawn_particle(x, y, 0.0, 1.0, Material::Acid);
            }
        }
        FusionId::SwampBloom => {
            let n = c.convert(target, 8, |m| (m == Material::Water).then_some(Material::Fungus));
            c.run.stats.fungus_grown += n as u32;
        }
        FusionId::FrozenEarth => {
            c.convert(target, 8, |m| {
                matches!(m, Material::Sand | Material::Gravel | Material::Ash).then_some(Material::Stone)
            });
        }
        FusionId::Sandstorm => {
            let toward = (target - feet).normalize_or(dir);
            c.fling(feet, 14, toward, 3.4, |m| {
                matches!(m, Material::Sand | Material::Gravel | Material::Ash)
            });
            c.harm(target, 12.0, 20.0, Element::Stone, 120.0);
        }
        FusionId::Railshot => {
            let end = tip + dir * 90.0;
            let mut last = end;
            for (x, y) in Ctx::line(tip, end) {
                for (cx, cy) in disc(x, y, 1) {
                    if diggable(c.world.material(cx, cy)) {
                        c.world.set(cx, cy, Material::Empty);
                    }
                }
                last = Vec2::new(x as f32, y as f32);
            }
            c.world.electrify(last.x as i32, last.y as i32, MAX_CHARGE);
            for k in 0..10 {
                c.harm(tip + dir * (k as f32 * 9.0), 5.0, 30.0, Element::Electric, 80.0);
            }
            c.shake.add(0.3);
        }
        FusionId::Petrify => {
            c.convert(target, 7, |m| liquid(m).then_some(Material::Stone));
            c.harm(target, 8.0, 10.0, Element::Stone, 0.0);
        }
        FusionId::RootLattice => {
            let end = feet + (target - feet).clamp_length_max(32.0);
            for (i, (x, y)) in Ctx::line(feet + Vec2::Y, end).into_iter().enumerate() {
                if c.world.material(x, y).is_open() {
                    c.world.set(x, y, Material::Wood);
                }
                // Rungs every few cells to climb on.
                if i % 5 == 0 {
                    for dx in -2..=2 {
                        if c.world.material(x + dx, y).is_open() {
                            c.world.set(x + dx, y, Material::Wood);
                        }
                    }
                }
            }
        }
        FusionId::Blizzard => {
            for k in 1..6 {
                let p = tip + dir * (k as f32 * 6.0);
                c.convert(p, 3 + k, |m| match m {
                    Material::Water | Material::Steam => Some(Material::Ice),
                    Material::Lava => Some(Material::Obsidian),
                    Material::Fire => Some(Material::Smoke),
                    _ => None,
                });
            }
            c.harm(tip + dir * 18.0, 16.0, 18.0, Element::Frost, 100.0);
        }
        FusionId::CryoArc => lightning(c, tip, target, true),
        FusionId::Shatter => {
            c.convert(target, 7, |m| {
                matches!(m, Material::Obsidian | Material::Crystal).then_some(Material::Gravel)
            });
        }
        FusionId::Rimebloom => {
            for (x, y) in disc(target.x as i32, target.y as i32, 10) {
                if c.world.material(x, y) == Material::Fungus {
                    c.world.set_with_life(x, y, Material::Frost, 10);
                }
            }
        }
        FusionId::Thunderstorm => {
            for _ in 0..4 {
                let x = target.x + (c.rng() - 0.5) * 40.0;
                let top = Vec2::new(x, target.y - 50.0);
                let mut hit = Vec2::new(x, target.y + 30.0);
                for y in top.y as i32..(target.y + 30.0) as i32 {
                    if !c.world.material(x as i32, y).is_open() {
                        hit = Vec2::new(x, y as f32);
                        break;
                    }
                }
                for p in Ctx::line(top, hit).iter().step_by(3) {
                    c.bursts.push(
                        Burst::new(Vec2::new(p.0 as f32, p.1 as f32), Color::srgb(0.9, 0.9, 1.0))
                            .count(1)
                            .speed(3.0)
                            .gravity(0.0)
                            .life(0.25),
                    );
                }
                c.fill(hit - Vec2::Y * 2.0, 2, Material::Spark);
                c.world.electrify(hit.x as i32, hit.y as i32, MAX_CHARGE);
                c.harm(hit, 7.0, 30.0, Element::Electric, 60.0);
            }
            c.shake.add(0.5);
        }
        FusionId::Miasma => {
            c.convert(target, 7, |m| (m == Material::Empty).then_some(Material::Gas));
        }
        FusionId::SporeGale => {
            for _ in 0..40 {
                let v = Vec2::from_angle(dir.to_angle() + (c.rng() - 0.5) * 0.8) * (1.5 + c.rng() * 2.0);
                c.world
                    .spawn_particle(tip.x, tip.y, v.x, v.y - 0.5, Material::Fungus);
            }
        }
        FusionId::Electrolysis => {
            c.convert(target, 7, |m| (m == Material::Water).then_some(Material::Gas));
            c.commands
                .spawn(Bomb::bundle(target, BombKind::Ignite, 0.6, c.assets));
        }
        FusionId::StormSpores => {
            for (x, y) in disc(target.x as i32, target.y as i32, 8) {
                if c.world.material(x, y) == Material::Fungus {
                    if c.world.material(x, y - 1).is_open() && c.run.rng.chance(60) {
                        c.world.set(x, y - 1, Material::Spark);
                    }
                    if c.run.rng.chance(40) {
                        c.world.ignite(x, y);
                    }
                }
            }
            c.harm(target, 9.0, 18.0, Element::Electric, 20.0);
        }
        FusionId::DecayBloom => {
            let mut rot = Vec::new();
            for (x, y) in disc(target.x as i32, target.y as i32, 7) {
                if matches!(
                    c.world.material(x, y),
                    Material::Wood | Material::Fungus | Material::Grass | Material::Dirt
                ) {
                    rot.push((x, y));
                }
            }
            for (x, y) in rot {
                let to = if c.run.rng.chance(80) {
                    Material::Acid
                } else {
                    Material::Empty
                };
                c.world.set(x, y, to);
            }
            c.harm(target, 8.0, 20.0, Element::Acid, 0.0);
        }
    }
}

/// A projectile spell hit something (or ran out).
pub fn impact(spell: Spell, c: &mut Ctx, at: Vec2, hit: Material) {
    let school = spell.school();
    c.sfx.push(Sfx::at(impact_sound(school), at));
    let (x, y) = (at.x as i32, at.y as i32);
    match spell {
        Spell::Scroll(ScrollId::FireBolt) => {
            for (cx, cy) in disc(x, y, 3) {
                let m = c.world.material(cx, cy);
                if m.props().flammability > 0 {
                    c.world.ignite(cx, cy);
                } else if m.is_open() && c.run.rng.chance(120) {
                    c.world.set(cx, cy, Material::Fire);
                }
            }
            c.harm(at, 6.0, 28.0, Element::Fire, 60.0);
        }
        Spell::Scroll(ScrollId::IceLance) => {
            c.convert(at, 5, |m| match m {
                Material::Water | Material::Steam => Some(Material::Ice),
                Material::Lava => Some(Material::Obsidian),
                Material::Fire => Some(Material::Smoke),
                _ => None,
            });
            c.harm(at, 5.0, 32.0, Element::Frost, 80.0);
        }
        Spell::Scroll(ScrollId::FrostSeed) => {
            for (cx, cy) in disc(x, y, 6) {
                let m = c.world.material(cx, cy);
                let near = (cx - x).abs() + (cy - y).abs() <= 3;
                if m == Material::Water || m == Material::Lava || (near && m.is_open()) {
                    c.world.set_with_life(cx, cy, Material::Frost, 14);
                }
            }
            c.harm(at, 6.0, 12.0, Element::Frost, 0.0);
        }
        Spell::Scroll(ScrollId::SparkBolt) => {
            c.world.electrify(x, y, MAX_CHARGE);
            c.fill(at, 2, Material::Spark);
            c.harm(at, 5.0, 20.0, Element::Electric, 40.0);
        }
        Spell::Scroll(ScrollId::AcidFlask) => {
            c.fill(at, 3, Material::Acid);
            for i in 0..20 {
                let a = i as f32 / 20.0 * std::f32::consts::TAU;
                c.world.spawn_particle(
                    at.x,
                    at.y - 1.0,
                    a.cos() * 1.5,
                    a.sin() * 1.5 - 1.0,
                    Material::Acid,
                );
            }
            c.harm(at, 6.0, 25.0, Element::Acid, 0.0);
        }
        Spell::Scroll(ScrollId::SporeSowing) => {
            for (cx, cy) in disc(x, y, 7) {
                if c.world.material(cx, cy).is_open() && anchored(c.world, cx, cy) {
                    c.grow(cx, cy);
                }
            }
        }
        Spell::Fusion(FusionId::Napalm) => {
            c.fill(at, 5, Material::Oil);
            for (cx, cy) in disc(x, y - 3, 3) {
                if c.world.material(cx, cy).is_open() {
                    c.world.set(cx, cy, Material::Fire);
                }
            }
        }
        Spell::Fusion(FusionId::SporeBomb) => {
            c.fill(at, 6, Material::Gas);
            for (cx, cy) in disc(x, y, 7) {
                if c.world.material(cx, cy).is_open() && anchored(c.world, cx, cy) {
                    c.grow(cx, cy);
                }
            }
            c.commands
                .spawn(Bomb::bundle(at, BombKind::Ignite, 1.0, c.assets));
        }
        Spell::Fusion(FusionId::Firestorm) => {
            c.world.explode(x, y, 4);
        }
        _ => {}
    }
    let _ = hit;
    c.burst(at, school_color(school), 14, 60.0);
}

/// Per-step effect while a projectile flies (Firestorm burns its path).
pub fn trail(spell: Spell, world: &mut World, at: Vec2) {
    if spell == Spell::Fusion(FusionId::Firestorm) {
        for (x, y) in disc(at.x as i32, at.y as i32, 3) {
            let m = world.material(x, y);
            if m.props().flammability > 0 {
                world.ignite(x, y);
            } else if m == Material::Empty && (x + y) % 3 == 0 {
                world.set(x, y, Material::Fire);
            }
        }
    }
}

fn anchored(world: &World, x: i32, y: i32) -> bool {
    [(0, 1), (0, -1), (1, 0), (-1, 0)]
        .iter()
        .any(|(dx, dy)| world.material(x + dx, y + dy).kind() == Kind::Solid)
}

pub fn cast_sound(s: Option<School>) -> &'static str {
    match s {
        Some(School::Pyromancy) => "cast_pyro",
        Some(School::Hydromancy) => "cast_hydro",
        Some(School::Terramancy) => "cast_terra",
        Some(School::Cryomancy) => "cast_cryo",
        Some(School::Aeromancy) => "cast_aero",
        Some(School::Fulmancy) => "cast_fulm",
        Some(School::Alchemy) => "cast_alch",
        Some(School::Mycomancy) => "cast_myco",
        None => "cast_light",
    }
}

pub fn impact_sound(s: Option<School>) -> &'static str {
    match s {
        Some(School::Pyromancy) => "impact_pyro",
        Some(School::Hydromancy) => "impact_hydro",
        Some(School::Terramancy) => "impact_terra",
        Some(School::Cryomancy) => "impact_cryo",
        Some(School::Aeromancy) => "impact_aero",
        Some(School::Fulmancy) => "impact_fulm",
        Some(School::Alchemy) => "impact_alch",
        Some(School::Mycomancy) => "impact_myco",
        None => "impact_terra",
    }
}

/// Reads the cast input, spends mana, runs cooldowns and casts.
pub fn cast_spells(
    mut commands: Commands,
    input: Res<PlayerInput>,
    time: Res<Time>,
    mut player: ResMut<RunPlayer>,
    mut run: ResMut<Run>,
    mut session: ResMut<Session>,
    assets: Res<GameAssets>,
    mut harm: MessageWriter<Harm>,
    mut bursts: MessageWriter<Burst>,
    mut sfx: MessageWriter<Sfx>,
    mut shake: ResMut<Shake>,
    creatures: Query<&super::creatures::Creature>,
    paused: Res<super::hud::Paused>,
) {
    let dt = time.delta_secs();
    run.mana = (run.mana + MANA_REGEN * dt).min(run.max_mana);
    for cd in run.cooldowns.values_mut() {
        *cd = (*cd - dt).max(0.0);
    }
    let n = run.spells().len().max(1);
    if input.cycle > 0 {
        run.selected = (run.selected + 1) % n;
    }
    if input.cycle < 0 {
        run.selected = (run.selected + n - 1) % n;
    }
    if !run.is_playing() || paused.0 || player.stun > 0.0 || run.attunement.is_some() {
        return;
    }
    let Some(spell) = run.selected_spell() else { return };
    let Some(world) = session.world.as_mut() else {
        return;
    };
    let aim = input
        .aim
        .unwrap_or(player.body.center() + Vec2::new(player.facing * 20.0, 0.0));
    let tip = player.staff_tip(aim - player.body.center());
    let positions: Vec<Vec2> = creatures.iter().map(|c| c.body.center()).collect();
    let (cost, cooldown) = spell.cost();

    if spell.channelled() {
        if !input.use_held || run.mana < cost * dt {
            return;
        }
        run.mana -= cost * dt;
        run.influence(tip + (aim - tip).clamp_length_max(30.0));
        player.cast_anim = 0.2;
        let mut c = Ctx {
            world,
            commands: &mut commands,
            assets: &assets,
            run: &mut run,
            player: &mut player,
            harm: Vec::new(),
            bursts: Vec::new(),
            sfx: vec![Sfx::at("tidecall_loop", tip).volume(0.6)],
            shake: &mut shake,
            creatures: &positions,
        };
        channel(spell, &mut c, tip, aim);
        c.flush(&mut harm, &mut bursts, &mut sfx);
        return;
    }

    if !input.use_pressed {
        return;
    }
    let ready = run.cooldowns.get(&spell).copied().unwrap_or(0.0) <= 0.0;
    if !ready || run.mana < cost {
        sfx.write(Sfx::ui("ui_click").volume(0.5));
        return;
    }
    run.mana -= cost;
    run.cooldowns.insert(spell, cooldown);
    let reach = aim.distance(tip).min(SPELL_RANGE.max(REACH));
    run.influence(tip + (aim - tip).normalize_or_zero() * reach);
    player.cast_anim = 0.35;
    sfx.write(Sfx::at(cast_sound(spell.school()), tip));
    let mut c = Ctx {
        world,
        commands: &mut commands,
        assets: &assets,
        run: &mut run,
        player: &mut player,
        harm: Vec::new(),
        bursts: Vec::new(),
        sfx: Vec::new(),
        shake: &mut shake,
        creatures: &positions,
    };
    cast(spell, &mut c, tip, aim);
    c.flush(&mut harm, &mut bursts, &mut sfx);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::achievements::StartChoice;
    use crate::run::save::SaveData;
    use bevy::ecs::world::CommandQueue;

    /// A test cave: stone floor, a water pool, a lava pool, sand, gravel,
    /// fungus, wood, metal and gas, all around the target point.
    fn cave() -> World {
        let mut w = World::new(192, 128, 1);
        let mut put = |x0: i32, y0: i32, x1: i32, y1: i32, m: Material| {
            for y in y0..=y1 {
                for x in x0..=x1 {
                    w.set(x, y, m);
                }
            }
        };
        put(0, 90, 191, 127, Material::Stone);
        put(60, 80, 75, 89, Material::Water);
        put(80, 80, 95, 89, Material::Lava);
        put(100, 84, 110, 89, Material::Sand);
        put(112, 84, 118, 89, Material::Gravel);
        put(120, 86, 126, 89, Material::Fungus);
        put(128, 80, 132, 89, Material::Wood);
        put(134, 86, 140, 89, Material::Metal);
        put(142, 76, 150, 85, Material::Gas);
        put(54, 70, 58, 89, Material::Obsidian);
        put(152, 84, 160, 89, Material::Dirt);
        w
    }

    /// Casts `spell` at `target` and reports whether anything happened: cells
    /// changed, entities spawned, creatures harmed or the caster moved.
    fn effect(spell: Spell, target: Vec2) -> bool {
        let mut world = cave();
        let before: Vec<Material> = (0..128)
            .flat_map(|y| (0..192).map(move |x| (x, y)))
            .map(|(x, y)| world.material(x, y))
            .collect();
        let mut ecs = bevy::ecs::world::World::new();
        let mut queue = CommandQueue::default();
        let mut commands = Commands::new(&mut queue, &ecs);
        let assets = GameAssets::placeholder();
        let save = SaveData::default();
        let mut run = Run::new(
            1,
            StartChoice::Initiate,
            School::Pyromancy,
            Some(School::Hydromancy),
            &save,
            Vec2::ZERO,
        );
        let start = Vec2::new(96.0, 70.0);
        let mut player = RunPlayer::new(start);
        let mut shake = Shake::default();
        let creatures = [target + Vec2::new(4.0, -3.0), target + Vec2::new(-20.0, -10.0)];
        let mut c = Ctx {
            world: &mut world,
            commands: &mut commands,
            assets: &assets,
            run: &mut run,
            player: &mut player,
            harm: Vec::new(),
            bursts: Vec::new(),
            sfx: Vec::new(),
            shake: &mut shake,
            creatures: &creatures,
        };
        let tip = c.player.body.center();
        cast(spell, &mut c, tip, target);
        let harmed = !c.harm.is_empty();
        let moved = c.player.body.pos != start || c.player.body.vel != Vec2::ZERO;
        drop(c);
        queue.apply(&mut ecs);
        let spawned = ecs.iter_entities().count() > 0;
        for _ in 0..3 {
            world.step();
        }
        let changed = (0..128)
            .flat_map(|y| (0..192).map(move |x| (x, y)))
            .zip(before)
            .any(|((x, y), m)| world.material(x, y) != m)
            || !world.particles().is_empty();
        changed || spawned || harmed || moved
    }

    #[test]
    fn every_fusion_does_something() {
        // Aim each at the part of the cave it's meant for.
        for f in FusionId::all() {
            let target = match f {
                FusionId::SteamBurst
                | FusionId::Glacier
                | FusionId::StormFlood
                | FusionId::SwampBloom
                | FusionId::Electrolysis => Vec2::new(67.0, 84.0),
                FusionId::MagmaSurge | FusionId::ThermalShock => Vec2::new(96.0, 92.0),
                FusionId::Mudslide | FusionId::FrozenEarth | FusionId::Sandstorm => Vec2::new(108.0, 86.0),
                FusionId::Petrify => Vec2::new(87.0, 84.0),
                FusionId::Shatter => Vec2::new(56.0, 80.0),
                FusionId::Rimebloom | FusionId::StormSpores => Vec2::new(123.0, 87.0),
                FusionId::DecayBloom => Vec2::new(130.0, 85.0),
                FusionId::Miasma | FusionId::Thunderstorm => Vec2::new(110.0, 75.0),
                _ => Vec2::new(130.0, 84.0),
            };
            assert!(effect(Spell::Fusion(f), target), "{f:?} had no effect");
        }
    }

    #[test]
    fn every_active_scroll_does_something() {
        for s in ScrollId::all().filter(|s| s.is_active() && !Spell::Scroll(*s).channelled()) {
            let target = match s {
                ScrollId::Transmutation => Vec2::new(137.0, 88.0),
                ScrollId::Blink => Vec2::new(110.0, 70.0),
                _ => Vec2::new(110.0, 82.0),
            };
            assert!(effect(Spell::Scroll(s), target), "{s:?} had no effect");
        }
    }

    #[test]
    fn every_fusion_and_active_has_a_cost() {
        for f in FusionId::all() {
            let (cost, cd) = Spell::Fusion(f).cost();
            assert!(cost > 0.0 && cd > 0.0, "{f:?}");
        }
        for s in ScrollId::all().filter(|s| s.is_active()) {
            assert!(Spell::Scroll(s).cost().0 > 0.0, "{s:?}");
        }
    }

    #[test]
    fn line_is_contiguous() {
        let cells = Ctx::line(Vec2::new(0.5, 0.5), Vec2::new(10.5, 4.5));
        assert_eq!(cells.first(), Some(&(0, 0)));
        assert_eq!(cells.last(), Some(&(10, 4)));
        for w in cells.windows(2) {
            assert!((w[0].0 - w[1].0).abs() <= 1 && (w[0].1 - w[1].1).abs() <= 1);
        }
    }
}
