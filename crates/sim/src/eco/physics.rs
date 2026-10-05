//! Moving bodies through the cell world, and what the world does to them.

use super::creature::Creature;
use super::genome::{Habitat, HuntStyle, Locomotion, Species, SpitKind};
use super::math::{V2, approach, v2};
use super::nav::{Profile, TILE};
use crate::material::{Kind, Material};
use crate::rng::Rng;
use crate::world::World;

#[derive(Clone, Copy, Debug, Default)]
pub struct MoveCmd {
    /// Desired direction (normalised or zero).
    pub dir: V2,
    /// Desired speed in cells per second.
    pub speed: f32,
    pub jump: bool,
    pub fly: bool,
    pub dig: bool,
    pub climb: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Ground,
    Swim,
    Fly,
    Climb,
    Burrow,
}

pub struct Bounds {
    /// Trees are solid to it.
    pub climber: bool,
    pub hw: f32,
    pub top: f32,
    pub bottom: f32,
}

pub fn bounds(c: &Creature, sp: &Species) -> Bounds {
    let s = c.scale(sp);
    Bounds {
        climber: sp.traits.climb,
        hw: (sp.body.hw * s).max(1.0),
        top: (sp.body.top * s).max(1.0),
        bottom: (sp.body.stance * s).max(1.0),
    }
}

/// Pathfinding profile for a species at a given scale.
pub fn profile(sp: &Species, scale: f32, gravity: f32) -> Profile {
    let t = &sp.traits;
    let height = ((sp.body.top + sp.body.stance) * scale / TILE as f32).ceil() as i32;
    let jump_h = sp.stats.jump * sp.stats.jump / (2.0 * gravity.max(1.0)) * 0.85;
    let pure_swimmer = sp.loco == Locomotion::Swim && t.gills;
    Profile {
        walk: !pure_swimmer && !matches!(sp.loco, Locomotion::Fly | Locomotion::Float),
        climb: t.climb,
        fly: t.fly,
        dig: t.dig,
        swim_water: t.swim,
        swim_lava: t.swim && t.lava_proof,
        swim_acid: t.swim && t.acid_proof,
        lava_ok: t.lava_proof,
        acid_ok: t.acid_proof,
        only_liquid: pure_swimmer,
        jump_tiles: ((jump_h / TILE as f32) as i32).clamp(1, 8),
        drop_tiles: 12,
        height_tiles: height.clamp(1, 5),
        half_width: ((sp.body.hw * scale - 1.0) / TILE as f32).round().clamp(0.0, 2.0) as i32,
    }
}

#[inline]
fn blocks(m: Material, digging: bool, climber: bool) -> bool {
    m.blocks_creature(climber) && !(digging && m.is_diggable())
}

fn box_blocked(world: &World, p: V2, b: &Bounds, digging: bool) -> bool {
    let x0 = (p.x - b.hw).floor() as i32;
    let x1 = (p.x + b.hw - 0.01).floor() as i32;
    let y0 = (p.y - b.top).floor() as i32;
    let y1 = (p.y + b.bottom - 0.01).floor() as i32;
    (y0..=y1).any(|y| (x0..=x1).any(|x| blocks(world.material(x, y), digging, b.climber)))
}

/// Any solid within one cell of the box.
pub fn touching_solid(world: &World, p: V2, b: &Bounds) -> Option<V2> {
    let x0 = (p.x - b.hw).floor() as i32 - 1;
    let x1 = (p.x + b.hw - 0.01).floor() as i32 + 1;
    let y0 = (p.y - b.top).floor() as i32 - 1;
    let y1 = (p.y + b.bottom - 0.01).floor() as i32 + 1;
    // Prefer the floor, then walls, then the ceiling.
    if (x0 + 1..x1).any(|x| world.material(x, y1).blocks_creature(b.climber)) {
        return Some(v2(0.0, 1.0));
    }
    if (y0 + 1..y1).any(|y| world.material(x0, y).blocks_creature(b.climber)) {
        return Some(v2(-1.0, 0.0));
    }
    if (y0 + 1..y1).any(|y| world.material(x1, y).blocks_creature(b.climber)) {
        return Some(v2(1.0, 0.0));
    }
    if (x0 + 1..x1).any(|x| world.material(x, y0).blocks_creature(b.climber)) {
        return Some(v2(0.0, -1.0));
    }
    None
}

fn box_has_soil(world: &World, p: V2, b: &Bounds, margin: i32) -> bool {
    let x0 = (p.x - b.hw).floor() as i32 - margin;
    let x1 = (p.x + b.hw - 0.01).floor() as i32 + margin;
    let y0 = (p.y - b.top).floor() as i32 - margin;
    let y1 = (p.y + b.bottom - 0.01).floor() as i32 + margin;
    (y0..=y1).any(|y| (x0..=x1).any(|x| world.material(x, y).is_diggable()))
}

pub fn mode_of(c: &Creature, sp: &Species, world: &World, cmd: &MoveCmd) -> Mode {
    let b = bounds(c, sp);
    let centre = world.material(c.pos.x as i32, c.pos.y as i32);
    if centre.kind() == Kind::Liquid && sp.swims_in(centre) {
        return Mode::Swim;
    }
    let t = &sp.traits;
    if t.dig && (cmd.dig || c.burrowed) {
        let probe = c.pos + cmd.dir * (b.hw + 1.5);
        if box_has_soil(world, c.pos, &b, 0) || (cmd.dig && box_has_soil(world, probe, &b, 0)) {
            return Mode::Burrow;
        }
    }
    if sp.loco == Locomotion::Float || (t.fly && cmd.fly && (c.energy > 0.03 || c.in_liquid)) {
        return Mode::Fly;
    }
    if t.climb && (cmd.climb || c.clinging) && touching_solid(world, c.pos, &b).is_some() {
        return Mode::Climb;
    }
    Mode::Ground
}

/// Advances position and velocity by `dt`. Digging changes the world.
pub fn integrate(
    c: &mut Creature,
    sp: &Species,
    world: &mut World,
    cmd: &MoveCmd,
    gravity: f32,
    dt: f32,
    rng: &mut Rng,
) -> Mode {
    let b = bounds(c, sp);
    let mode = mode_of(c, sp, world, cmd);
    if c.status.grabbed_by.is_some() {
        c.vel = V2::ZERO;
        return mode;
    }
    let accel = sp.stats.accel * if c.status.stun > 0.0 { 0.2 } else { 1.0 };
    let target = cmd.dir * cmd.speed;
    let centre = world.material(c.pos.x as i32, c.pos.y as i32);
    let wading = centre.kind() == Kind::Liquid && mode != Mode::Swim;

    match mode {
        Mode::Swim => {
            c.vel.x = approach(c.vel.x, target.x, accel * dt);
            c.vel.y = approach(c.vel.y, target.y, accel * dt);
            c.vel *= 0.985;
            // Gills that can walk on land hop out when asked to jump.
            if cmd.jump && sp.traits.swim && !sp.traits.gills {
                c.vel.y = c.vel.y.min(-sp.stats.jump * 0.5);
            }
        }
        Mode::Fly => {
            let float = sp.loco == Locomotion::Float;
            c.vel.x = approach(c.vel.x, target.x, accel * dt * if float { 0.4 } else { 1.0 });
            c.vel.y = approach(c.vel.y, target.y, accel * dt * if float { 0.4 } else { 1.0 });
            c.vel.y += gravity * dt * 0.08;
            c.energy -= dt * if float { 0.0 } else { 0.0025 };
        }
        Mode::Climb => {
            c.vel.x = approach(c.vel.x, target.x * 0.85, accel * dt);
            c.vel.y = approach(c.vel.y, target.y * 0.85, accel * dt);
        }
        Mode::Burrow => {
            let inside = box_has_soil(world, c.pos, &b, 0);
            c.vel.x = approach(c.vel.x, target.x * 0.6, accel * dt);
            if inside {
                c.vel.y = approach(c.vel.y, target.y * 0.6, accel * dt);
            } else {
                c.vel.y += gravity * dt;
            }
        }
        Mode::Ground => {
            let hop = sp.loco == Locomotion::Hop;
            let want = if cmd.dir.x.abs() > 0.1 {
                cmd.dir.x.signum() * cmd.speed * cmd.dir.x.abs().max(0.4)
            } else {
                0.0
            };
            if !hop || !c.on_ground {
                let a = if c.on_ground || wading { accel } else { accel * 0.4 };
                c.vel.x = approach(c.vel.x, want, a * dt);
            } else {
                c.vel.x = approach(c.vel.x, 0.0, accel * 2.0 * dt);
            }
            if wading {
                c.vel.y += gravity * dt * 0.3;
                c.vel *= 0.94;
                // Paddle towards the surface when trying to get out.
                if cmd.dir.y < -0.3 || cmd.jump {
                    c.vel.y = approach(c.vel.y, -30.0, accel * dt);
                }
            } else {
                c.vel.y += gravity * dt;
            }
            c.vel.y = c.vel.y.min(330.0);
            if c.on_ground && c.status.stun <= 0.0 {
                if cmd.jump {
                    c.vel.y = -sp.stats.jump;
                    c.on_ground = false;
                } else if hop && want != 0.0 {
                    c.vel.y = -sp.stats.jump * 0.55;
                    c.vel.x = want * 1.1;
                    c.on_ground = false;
                }
            }
            if wading && cmd.jump {
                c.vel.y = c.vel.y.min(-40.0);
            }
        }
    }
    if c.webbed > 0.0 {
        c.vel *= 0.05;
    }

    let digging = mode == Mode::Burrow;
    let step_up = if mode == Mode::Ground || mode == Mode::Swim {
        ((b.bottom * 0.6) as i32 + 1).clamp(2, 6)
    } else {
        1
    };

    // Horizontal.
    let dx = c.vel.x * dt;
    let n = (dx.abs() * 2.0).ceil().max(1.0) as i32;
    for _ in 0..n {
        let inc = dx / n as f32;
        let next = c.pos + v2(inc, 0.0);
        if !box_blocked(world, next, &b, digging) {
            c.pos = next;
            continue;
        }
        let climb = (1..=step_up).find(|&up| !box_blocked(world, next - v2(0.0, up as f32), &b, digging));
        match climb {
            Some(up) if c.on_ground || mode != Mode::Ground => {
                c.pos = next - v2(0.0, up as f32);
            }
            _ => {
                c.vel.x = 0.0;
                break;
            }
        }
    }
    // Vertical.
    let dy = c.vel.y * dt;
    let n = (dy.abs() * 2.0).ceil().max(1.0) as i32;
    c.on_ground = false;
    for _ in 0..n {
        let inc = dy / n as f32;
        let next = c.pos + v2(0.0, inc);
        if box_blocked(world, next, &b, digging) {
            if inc > 0.0 {
                c.on_ground = true;
            }
            c.vel.y = 0.0;
            break;
        }
        c.pos = next;
    }
    if !c.on_ground && box_blocked(world, c.pos + v2(0.0, 1.0), &b, false) {
        c.on_ground = true;
    }
    // Stuck inside terrain (sand fell on us): climb out.
    if box_blocked(world, c.pos, &b, digging) {
        c.pos.y -= 1.0;
    }

    if digging {
        dig_box(c, sp, world, &b, rng);
    }
    restore_displaced(c, world, &b);

    c.clinging = mode == Mode::Climb;
    if mode == Mode::Climb
        && let Some(s) = touching_solid(world, c.pos, &b)
    {
        c.anim.surface = c.anim.surface.lerp(s, 0.2).norm();
    } else if mode != Mode::Climb {
        c.anim.surface = c.anim.surface.lerp(v2(0.0, 1.0), 0.1).norm();
    }
    c.flying = mode == Mode::Fly;
    c.burrowed = digging && box_has_soil(world, c.pos, &b, 0);
    c.in_liquid = world.material(c.pos.x as i32, c.pos.y as i32).kind() == Kind::Liquid;

    let (w, h) = (world.width() as f32, world.height() as f32);
    c.pos.x = c.pos.x.clamp(b.hw + 1.0, w - b.hw - 1.0);
    c.pos.y = c.pos.y.clamp(b.top + 1.0, h - b.bottom - 1.0);
    if c.vel.x.abs() > 2.0 && mode != Mode::Climb {
        c.facing = c.vel.x.signum();
    }
    mode
}

/// Removes soil the body overlaps. Sand-swimmers remember it to put back.
fn dig_box(c: &mut Creature, sp: &Species, world: &mut World, b: &Bounds, rng: &mut Rng) {
    let x0 = (c.pos.x - b.hw).floor() as i32;
    let x1 = (c.pos.x + b.hw - 0.01).floor() as i32;
    let y0 = (c.pos.y - b.top).floor() as i32;
    let y1 = (c.pos.y + b.bottom - 0.01).floor() as i32;
    for y in y0..=y1 {
        for x in x0..=x1 {
            let m = world.material(x, y);
            if !m.is_diggable() {
                continue;
            }
            if sp.traits.sand_swim {
                c.displaced.push((x, y, m as u8));
            } else if c.carry_dirt < 400 && rng.chance(120) {
                c.carry_dirt += 1;
            }
            world.set(x, y, Material::Empty);
        }
    }
}

fn restore_displaced(c: &mut Creature, world: &mut World, b: &Bounds) {
    if c.displaced.is_empty() {
        return;
    }
    let (px, py) = (c.pos.x, c.pos.y);
    let keep = |x: i32, y: i32| {
        x as f32 >= px - b.hw - 1.0
            && x as f32 <= px + b.hw + 1.0
            && y as f32 >= py - b.top - 1.0
            && y as f32 <= py + b.bottom + 1.0
    };
    let mut i = 0;
    while i < c.displaced.len() {
        let (x, y, m) = c.displaced[i];
        if keep(x, y) {
            i += 1;
            continue;
        }
        if world.material(x, y) == Material::Empty {
            world.set(x, y, Material::from_u8(m));
        }
        c.displaced.swap_remove(i);
    }
    if c.displaced.len() > 600 {
        c.displaced.drain(..300);
    }
}

/// Damage per second and other effects from the cells around a creature.
pub fn environment(
    c: &mut Creature,
    sp: &Species,
    world: &mut World,
    dt: f32,
    rng: &mut Rng,
) -> (f32, Option<&'static str>) {
    let b = bounds(c, sp);
    let t = &sp.traits;
    let mut dps = 0.0;
    let mut cause = None;
    let samples = [
        c.pos,
        c.pos + v2(0.0, b.bottom - 0.5),
        c.pos - v2(0.0, b.top - 0.5),
        c.pos + v2(b.hw - 0.5, 0.0),
        c.pos - v2(b.hw - 0.5, 0.0),
    ];
    let mut web = 0;
    let mut lava = false;
    let mut acid = 0;
    let mut fire = false;
    for p in samples {
        match world.material(p.x as i32, p.y as i32) {
            Material::Lava => lava = true,
            Material::Acid => acid += 1,
            Material::Fire => fire = true,
            Material::Web => web += 1,
            _ => {}
        }
    }
    if lava && !t.lava_proof {
        dps += 60.0;
        c.status.burning = 3.0;
        cause = Some("burned in lava");
    }
    if acid > 0 && !t.acid_proof {
        dps += 5.0 * acid as f32;
        cause = Some("dissolved in acid");
    }
    if fire && !t.lava_proof {
        c.status.burning = c.status.burning.max(2.0);
    }
    if c.status.burning > 0.0 {
        c.status.burning -= dt;
        if c.in_liquid && !lava {
            c.status.burning = 0.0;
        } else {
            dps += 7.0;
            cause = cause.or(Some("burned"));
            if rng.chance(30) {
                world.paint_circle(c.pos.x as i32, (c.pos.y - b.top) as i32 - 1, 0, Material::Fire);
            }
        }
    }
    // Breathing.
    let head = c.pos - v2(0.0, b.top * 0.6);
    let head_wet = world.material(head.x as i32, head.y as i32).kind() == Kind::Liquid;
    let adapted = matches!(sp.habitat, Habitat::Aquatic | Habitat::LavaDweller) && t.swim;
    if adapted && !t.gills {
        // Breathes through its skin: fine in its liquid and in air.
        c.breath = (c.breath + dt / 2.0).min(1.0);
    } else if t.gills {
        if !c.in_liquid {
            c.breath -= dt / 7.0;
        } else {
            c.breath = (c.breath + dt / 2.0).min(1.0);
        }
    } else if head_wet && !t.swim {
        c.breath -= dt / 9.0;
    } else if head_wet {
        c.breath -= dt / 40.0;
    } else {
        c.breath = (c.breath + dt / 2.0).min(1.0);
    }
    if c.breath < 0.0 {
        c.breath = 0.0;
        dps += 12.0;
        cause = Some(if t.gills { "suffocated in air" } else { "drowned" });
    }
    // Webs.
    let immune = sp.hunts(HuntStyle::WebTrap) || t.spit == Some(SpitKind::Web);
    if web > 0 && !immune {
        c.webbed = (c.webbed + dt * 2.0).min(3.0);
        // Struggle: tear at the silk.
        let strength = sp.stats.size * c.scale(sp);
        if rng.prob(dt * strength * 0.35) {
            let p = samples[rng.int(0, 4) as usize];
            for dy in -1..=1 {
                for dx in -1..=1 {
                    if world.material(p.x as i32 + dx, p.y as i32 + dy) == Material::Web {
                        world.set(p.x as i32 + dx, p.y as i32 + dy, Material::Empty);
                    }
                }
            }
        }
    } else {
        c.webbed = (c.webbed - dt * 3.0).max(0.0);
    }
    (dps, cause)
}
