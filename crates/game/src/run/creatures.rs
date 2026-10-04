//! Creatures of the deep. They matter less than the environment, and they
//! live in it too: they burn, drown, freeze, get crushed and dissolve.
//! The Hollow Stalker is the exception: it can't be killed, only escaped or
//! trapped using the world itself.

use bevy::prelude::*;
use sbct_sim::descent::{CreatureKind, Layer, SpawnKind};
use sbct_sim::material::INDESTRUCTIBLE;
use sbct_sim::world::disc;
use sbct_sim::{Kind, Material, World};

use super::achievements::AchievementId;
use super::entities::{LightOrb, ShardPickup};
use super::hazards::hurt;
use super::physics::{Body, default_solid, to_world};
use super::player::RunPlayer;
use super::save::SaveData;
use super::scrolls::ScrollId;
use super::spells::{Element, Harm};
use super::{DamageKind, Run, RunSetup, grant};
use crate::assets::GameAssets;
use crate::audio::Sfx;
use crate::fx::{Burst, Shake};
use crate::render::InGameEntity;
use crate::session::Session;

/// Creatures farther than this from the player are frozen (saves work).
const ACTIVE_RANGE: f32 = 300.0;

/// Every kind of creature, including the two the generator doesn't place.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Beast {
    Gnawling,
    Hollowed,
    BlindWyrm,
    SporePuppet,
    SporeDrifter,
    CinderWraith,
    Lightseeker,
    /// A chest with teeth.
    Mimic,
    /// Unkillable; appears once per run in the deep.
    Stalker,
}

impl From<CreatureKind> for Beast {
    fn from(k: CreatureKind) -> Beast {
        match k {
            CreatureKind::Gnawling => Beast::Gnawling,
            CreatureKind::Hollowed => Beast::Hollowed,
            CreatureKind::BlindWyrm => Beast::BlindWyrm,
            CreatureKind::SporePuppet => Beast::SporePuppet,
            CreatureKind::SporeDrifter => Beast::SporeDrifter,
            CreatureKind::CinderWraith => Beast::CinderWraith,
            CreatureKind::Lightseeker => Beast::Lightseeker,
        }
    }
}

impl Beast {
    /// (half width, height, hp)
    fn stats(self) -> (f32, f32, f32) {
        match self {
            Beast::Gnawling => (4.0, 6.0, 30.0),
            Beast::Hollowed => (2.5, 12.0, 45.0),
            Beast::BlindWyrm => (4.0, 8.0, 120.0),
            Beast::SporePuppet => (2.5, 11.0, 40.0),
            Beast::SporeDrifter => (3.0, 7.0, 15.0),
            Beast::CinderWraith => (3.0, 9.0, 40.0),
            Beast::Lightseeker => (3.0, 6.0, 25.0),
            Beast::Mimic => (5.0, 8.0, 60.0),
            Beast::Stalker => (3.0, 26.0, f32::INFINITY),
        }
    }

    /// Row in the creature sheet.
    fn row(self) -> usize {
        match self {
            Beast::Gnawling => 0,
            Beast::Hollowed => 1,
            Beast::BlindWyrm => 2,
            Beast::SporePuppet => 3,
            Beast::SporeDrifter => 4,
            Beast::CinderWraith => 5,
            Beast::Lightseeker => 6,
            Beast::Mimic => 7,
            Beast::Stalker => 0,
        }
    }

    fn flies(self) -> bool {
        matches!(
            self,
            Beast::SporeDrifter | Beast::CinderWraith | Beast::Lightseeker
        )
    }

    fn fire_proof(self) -> bool {
        matches!(self, Beast::CinderWraith | Beast::Stalker)
    }

    fn contact_damage(self) -> f32 {
        match self {
            Beast::Gnawling => 14.0,
            Beast::Hollowed => 12.0,
            Beast::BlindWyrm => 22.0,
            Beast::SporePuppet => 15.0,
            Beast::SporeDrifter => 12.0,
            Beast::CinderWraith => 16.0,
            Beast::Lightseeker => 12.0,
            Beast::Mimic => 18.0,
            Beast::Stalker => 45.0,
        }
    }
}

#[derive(Component)]
pub struct Creature {
    pub kind: Beast,
    pub body: Body,
    pub hp: f32,
    pub dir: f32,
    /// Seconds spent underwater (walkers drown).
    pub soaked: f32,
    pub anim: f32,
    pub hurt_flash: f32,
    /// General-purpose timer: casting, trails, hops, burrow decisions.
    pub timer: f32,
    /// Frozen or petrified: can't move or act.
    pub stun: f32,
    /// What last hurt it (for achievements like Overcharged).
    pub last_hit: Option<Element>,
    /// Where it's heading (wyrms, lightseekers).
    pub goal: Option<Vec2>,
    /// Wyrm body: past head positions.
    pub trail: Vec<Vec2>,
    /// The Hollowed's school of twisted magic.
    pub element: Element,
    /// Stalker: seconds trapped / out of reach, and retreat after a hit.
    pub trapped: f32,
    pub lost: f32,
    pub retreat: f32,
}

impl Creature {
    pub fn new(kind: Beast, pos: Vec2, seed: i32) -> Creature {
        let (half_w, h, hp) = kind.stats();
        let element = [Element::Fire, Element::Frost, Element::Acid][(seed.unsigned_abs() % 3) as usize];
        Creature {
            kind,
            body: Body::new(pos, half_w, h),
            hp,
            dir: if seed % 2 == 0 { 1.0 } else { -1.0 },
            soaked: 0.0,
            anim: 0.0,
            hurt_flash: 0.0,
            timer: 2.0 + (seed.unsigned_abs() % 30) as f32 / 10.0,
            stun: 0.0,
            last_hit: None,
            goal: None,
            trail: Vec::new(),
            element,
            trapped: 0.0,
            lost: 0.0,
            retreat: 0.0,
        }
    }
}

/// A wyrm's body segment (drawn along its trail).
#[derive(Component)]
pub struct WyrmSegment {
    owner: Entity,
    index: usize,
}

/// A twisted spell cast by a Hollowed Apprentice.
#[derive(Component)]
pub struct EnemyBolt {
    pos: Vec2,
    vel: Vec2,
    element: Element,
    age: f32,
}

/// How close (0–1) the Hollow Stalker is: drives the vignette, camera
/// wobble and whispers. Also tracks whether it has appeared this run.
#[derive(Resource, Default)]
pub struct StalkerState {
    pub near: f32,
    pub spawned: bool,
    /// Seconds spent in the deep layers (it comes after a while).
    pub deep_time: f32,
}

pub fn spawn_creature(
    commands: &mut Commands,
    assets: &GameAssets,
    kind: Beast,
    pos: Vec2,
    seed: i32,
) -> Entity {
    let sprite = if kind == Beast::Stalker {
        assets.stalker.sprite(6)
    } else {
        assets.creatures.sprite(kind.row() * 8)
    };
    let e = commands
        .spawn((
            Creature::new(kind, pos, seed),
            InGameEntity,
            sprite,
            Transform::from_translation(to_world(pos, 4.8)),
        ))
        .id();
    if kind == Beast::BlindWyrm {
        for index in 0..7 {
            let frame = if index == 6 { 6 } else { 4 + index % 2 };
            commands.spawn((
                WyrmSegment { owner: e, index },
                InGameEntity,
                assets.creatures.sprite(kind.row() * 8 + frame),
                Transform::from_translation(to_world(pos, 4.7)),
            ));
        }
    }
    e
}

pub fn spawn_creatures(mut commands: Commands, setup: Res<RunSetup>, assets: Res<GameAssets>) {
    commands.insert_resource(StalkerState::default());
    for s in &setup.spawns {
        let SpawnKind::Creature(kind) = s.kind else {
            continue;
        };
        let pos = Vec2::new(s.x as f32 + 0.5, s.y as f32 + 1.0);
        spawn_creature(&mut commands, &assets, kind.into(), pos, s.x + s.y);
    }
}

fn diggable(m: Material) -> bool {
    matches!(m.kind(), Kind::Solid | Kind::Powder) && m.props().hardness != INDESTRUCTIBLE
}

pub fn update_creatures(
    mut commands: Commands,
    assets: Res<GameAssets>,
    time: Res<Time>,
    mut session: ResMut<Session>,
    mut player: ResMut<RunPlayer>,
    mut run: ResMut<Run>,
    mut q: Query<(Entity, &mut Creature)>,
    mut orbs: Query<&mut LightOrb>,
    segments: Query<(Entity, &WyrmSegment)>,
    mut sfx: MessageWriter<Sfx>,
    mut shake: ResMut<Shake>,
    mut bursts: MessageWriter<Burst>,
    mut harms: MessageReader<Harm>,
    mut save: ResMut<SaveData>,
    mut stalker: ResMut<StalkerState>,
) {
    let harms: Vec<Harm> = harms.read().copied().collect();
    let Some(world) = session.world.as_mut() else {
        return;
    };
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    let target = player.body.center();
    let beacon = run.has(ScrollId::Beacon);
    let aggro = if beacon { 170.0 } else { 80.0 };
    // Recent digging is noise the wyrms hear.
    let noise = run
        .influence
        .back()
        .filter(|(_, t)| run.elapsed - t < 2.0)
        .map(|(p, _)| *p);
    stalker.near = 0.0;

    for (entity, mut c) in &mut q {
        let dist = c.body.center().distance(target);
        if dist > ACTIVE_RANGE && c.kind != Beast::Stalker {
            continue;
        }
        c.anim += dt;
        c.hurt_flash = (c.hurt_flash - dt).max(0.0);
        c.timer -= dt;
        c.retreat = (c.retreat - dt).max(0.0);

        // Spells.
        for h in &harms {
            let d = c.body.center().distance(h.at);
            if d > h.radius + c.body.half_w {
                continue;
            }
            let immune = c.kind == Beast::Stalker || (c.kind.fire_proof() && h.element == Element::Fire);
            if !immune {
                c.hp -= h.damage * (1.0 - 0.5 * d / (h.radius + 1.0));
                c.last_hit = Some(h.element);
                if c.kind == Beast::SporePuppet && c.hurt_flash <= 0.0 {
                    sfx.write(Sfx::at("puppet_squelch", c.body.center()));
                }
                c.hurt_flash = 0.25;
            }
            let push = (c.body.center() - h.at).normalize_or_zero() * h.push;
            c.body.vel += push;
            match h.element {
                Element::Frost => c.stun = c.stun.max(1.5),
                Element::Stone => c.stun = c.stun.max(3.0),
                Element::Electric => c.stun = c.stun.max(0.4),
                _ => {}
            }
        }
        c.stun = (c.stun - dt).max(0.0);

        // The environment, from the same cells that hurt the player.
        let (mut hot, mut acid, mut water, mut solid) = (0, 0, 0, 0);
        c.body.for_each_cell(|x, y| match world.material(x, y) {
            m if m.kind() == Kind::Fire || m == Material::Lava || m == Material::Metal => hot += 1,
            Material::Acid => acid += 1,
            Material::Water => water += 1,
            m if m.is_solid_for_player() => solid += 1,
            _ => {}
        });
        if c.kind != Beast::Stalker {
            let mut damage = 0.0;
            if hot > 0 && !c.kind.fire_proof() {
                damage += 40.0 * dt;
                c.last_hit = Some(Element::Fire);
            }
            if acid > 0 {
                damage += 50.0 * dt;
                c.last_hit = Some(Element::Acid);
            }
            if c.kind == Beast::CinderWraith && water > 0 {
                damage += 70.0 * dt;
                if run.rng.chance(40) {
                    bursts.write(
                        Burst::new(c.body.center(), Color::srgba(0.85, 0.85, 0.9, 0.7))
                            .count(3)
                            .gravity(-60.0),
                    );
                }
            }
            if c.body.submersion(world) > 0.6 && !c.kind.flies() && c.kind != Beast::BlindWyrm {
                c.soaked += dt;
                if c.soaked > 4.0 {
                    damage += 30.0 * dt;
                }
            } else {
                c.soaked = 0.0;
            }
            if c.kind != Beast::BlindWyrm && !c.body.escape_overlap(world, default_solid) {
                damage += 80.0 * dt;
            }
            if damage > 0.0 {
                c.hp -= damage;
                c.hurt_flash = 0.15;
            }
        }

        if c.stun > 0.0 {
            if !c.kind.flies() && c.kind != Beast::BlindWyrm {
                c.body.vel.x *= 0.8;
                c.body.vel.y = (c.body.vel.y + 400.0 * dt).min(300.0);
                c.body.step(world, dt, 0, default_solid);
            }
        } else {
            behave(
                &mut c,
                world,
                &mut run,
                target,
                dist,
                aggro,
                beacon,
                noise,
                &mut orbs,
                &mut commands,
                &assets,
                &mut sfx,
                &mut bursts,
                dt,
            );
        }

        // The Stalker's own rules: escape or trap it.
        if c.kind == Beast::Stalker {
            stalker.near = (1.0 - dist / 160.0).clamp(0.0, 1.0);
            let iced = disc(c.body.center().x as i32, c.body.center().y as i32, 6)
                .filter(|&(x, y)| matches!(world.material(x, y), Material::Ice | Material::Frost))
                .count();
            let trapped = solid > 12 || iced > 40;
            c.trapped = if trapped { c.trapped + dt } else { 0.0 };
            c.lost = if dist > 260.0 { c.lost + dt } else { 0.0 };
            if c.trapped > 2.5 || c.lost > 15.0 {
                commands.entity(entity).despawn();
                bursts.write(
                    Burst::new(c.body.center(), Color::srgba(0.2, 0.1, 0.3, 0.8))
                        .count(30)
                        .speed(30.0)
                        .gravity(40.0)
                        .life(1.5),
                );
                sfx.write(Sfx::at("sting_2", c.body.center()));
                grant(&mut save, &mut run, AchievementId::StalkersShadow, &mut sfx);
                continue;
            }
            if run.rng.chance(2) && dist < 140.0 {
                sfx.write(Sfx::at("stalker_breath", c.body.center()).volume(0.8).pitch(0.0));
            }
        }

        // Touching the player.
        let touching = c.body.center().distance(target) < c.body.half_w + 5.0
            || (c.kind == Beast::BlindWyrm && c.trail.iter().any(|p| p.distance(target) < 6.0));
        if touching && run.is_playing() && c.stun <= 0.0 && c.retreat <= 0.0 {
            let knock = Vec2::new((target.x - c.body.pos.x).signum() * 90.0, -80.0);
            hurt(
                &mut player,
                &mut run,
                c.kind.contact_damage(),
                DamageKind::Creature,
                Some(knock),
                &mut sfx,
                &mut shake,
            );
            match c.kind {
                Beast::SporeDrifter => c.hp = 0.0,
                Beast::CinderWraith => {
                    if !run.has(ScrollId::SalamanderWard) {
                        player.burning = player.burning.max(1.5);
                    }
                }
                Beast::Mimic => {
                    sfx.write(Sfx::at("mimic_bite", c.body.center()));
                }
                Beast::Stalker => {
                    c.retreat = 1.5;
                    shake.add(0.6);
                }
                _ => {}
            }
        }

        if c.hp <= 0.0 {
            die(
                entity,
                &c,
                world,
                &mut run,
                &mut save,
                &mut commands,
                &assets,
                &segments,
                &mut sfx,
                &mut bursts,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn behave(
    c: &mut Creature,
    world: &mut World,
    run: &mut Run,
    target: Vec2,
    dist: f32,
    aggro: f32,
    beacon: bool,
    noise: Option<Vec2>,
    orbs: &mut Query<&mut LightOrb>,
    commands: &mut Commands,
    assets: &GameAssets,
    sfx: &mut MessageWriter<Sfx>,
    bursts: &mut MessageWriter<Burst>,
    dt: f32,
) {
    match c.kind {
        Beast::Gnawling | Beast::Hollowed | Beast::SporePuppet | Beast::Mimic => {
            let speed = match c.kind {
                Beast::Gnawling => 22.0,
                Beast::Hollowed => 14.0,
                Beast::SporePuppet => 12.0,
                _ => 26.0,
            };
            let chasing = dist < aggro * 0.7 || c.kind == Beast::Mimic;
            if chasing && (target.y - c.body.pos.y).abs() < 24.0 {
                c.dir = (target.x - c.body.pos.x).signum();
            }
            let ahead = Vec2::new(c.body.pos.x + c.dir * (c.body.half_w + 1.0), c.body.pos.y + 1.0);
            let ledge = c.body.on_ground && !world.is_solid(ahead.x as i32, ahead.y as i32) && !chasing;
            if c.body.wall != 0 || ledge {
                c.dir = -c.dir;
            }
            c.body.vel.x = c.dir * speed;
            if c.kind == Beast::Mimic && c.body.on_ground && c.timer <= 0.0 {
                c.timer = 0.9;
                c.body.vel.y = -140.0;
            }
            c.body.vel.y = (c.body.vel.y + 400.0 * dt).min(300.0);
            c.body.step(world, dt, 2, default_solid);
            if c.kind == Beast::Hollowed {
                if c.timer <= 0.0 && dist < 90.0 {
                    c.timer = 3.5 + run.rng.next_f32() * 2.0;
                    let from = c.body.center() - Vec2::Y * 3.0;
                    let vel = (target - from).normalize_or_zero() * 110.0;
                    commands.spawn((
                        EnemyBolt {
                            pos: from,
                            vel,
                            element: c.element,
                            age: 0.0,
                        },
                        InGameEntity,
                        assets.spells.sprite(element_row(c.element) * 8),
                        Transform::from_translation(to_world(from, 5.4)),
                    ));
                    sfx.write(Sfx::at("hollowed_cast", from));
                } else if run.rng.chance(1) {
                    sfx.write(Sfx::at("hollowed_moan", c.body.center()).volume(0.7));
                }
            }
        }
        Beast::SporeDrifter | Beast::CinderWraith => {
            let to = target - c.body.center();
            let bob = (c.anim * 3.0).sin() * 12.0;
            let speed = if c.kind == Beast::CinderWraith { 20.0 } else { 26.0 };
            c.body.vel = if to.length() < aggro {
                to.normalize_or_zero() * speed + Vec2::Y * bob
            } else {
                Vec2::new(c.dir * 8.0, bob)
            };
            c.body.step(world, dt, 0, default_solid);
            if c.body.wall != 0 {
                c.dir = -c.dir;
            }
            if c.kind == Beast::CinderWraith && c.timer <= 0.0 {
                c.timer = 1.2;
                let (x, y) = (c.body.pos.x as i32, c.body.pos.y as i32);
                if world.material(x, y).is_open() {
                    world.set(x, y, Material::Fire);
                }
            }
        }
        Beast::Lightseeker => {
            // Hunt the brightest light: an orb if any is near, else the
            // wizard, unless their staff is dimmed.
            if c.timer <= 0.0 {
                c.timer = 0.5;
                let here = c.body.center();
                let orb = orbs
                    .iter()
                    .map(|o| o.pos)
                    .filter(|p| p.distance(here) < 160.0)
                    .min_by(|a, b| a.distance(here).total_cmp(&b.distance(here)));
                let sight = if beacon {
                    220.0
                } else if run.staff_dimmed {
                    25.0
                } else {
                    110.0
                };
                c.goal = orb.or((dist < sight).then_some(target));
            }
            let here = c.body.center();
            let goal = c
                .goal
                .unwrap_or(here + Vec2::new(c.dir * 20.0, (c.anim * 2.0).sin() * 10.0));
            let to = goal - here;
            c.body.vel = to.normalize_or_zero() * 48.0 + Vec2::new(0.0, (c.anim * 9.0).sin() * 20.0);
            c.body.step(world, dt, 0, default_solid);
            if c.body.wall != 0 {
                c.dir = -c.dir;
            }
            // Feeding on a light orb drains it.
            for mut o in orbs.iter_mut() {
                if o.pos.distance(here) < 6.0 && o.life > 0.5 {
                    o.life = 0.5;
                    sfx.write(Sfx::at("lightseeker_shriek", here));
                }
            }
            if run.rng.chance(6) && dist < 120.0 {
                sfx.write(Sfx::at("lightseeker_flutter", here).volume(0.6).pitch(0.0));
            }
        }
        Beast::BlindWyrm => {
            // Hears digging; otherwise wanders through the rock.
            if c.timer <= 0.0 {
                c.timer = 3.0;
                c.goal = match noise {
                    Some(p) if p.distance(c.body.pos) < 180.0 => Some(p),
                    _ => Some(
                        c.body.pos + Vec2::new(run.rng.next_f32() - 0.5, run.rng.next_f32() - 0.5) * 120.0,
                    ),
                };
            }
            if let Some(p) = noise
                && p.distance(c.body.pos) < 180.0
            {
                c.goal = Some(p);
            }
            let goal = c.goal.unwrap_or(c.body.pos);
            let to = goal - c.body.pos;
            let speed = if to.length() < 4.0 { 0.0 } else { 32.0 };
            let step = to.normalize_or_zero() * speed * dt;
            c.body.pos += step;
            c.body.pos.x = c.body.pos.x.clamp(8.0, world.width() as f32 - 8.0);
            c.body.pos.y = c.body.pos.y.clamp(200.0, world.height() as f32 - 120.0);
            // Eat a tunnel.
            let (x, y) = (c.body.pos.x as i32, (c.body.pos.y - 4.0) as i32);
            let mut ate = false;
            for (cx, cy) in disc(x, y, 3) {
                if diggable(world.material(cx, cy)) {
                    world.set(cx, cy, Material::Empty);
                    ate = true;
                }
            }
            if ate && run.rng.chance(10) {
                sfx.write(Sfx::at("wyrm_burrow", c.body.pos).volume(0.8).pitch(0.0));
            }
            if c.trail.last().is_none_or(|p| p.distance(c.body.pos) > 4.0) {
                c.trail.push(c.body.pos);
                if c.trail.len() > 8 {
                    c.trail.remove(0);
                }
            }
            if speed > 0.0 {
                c.dir = to.x.signum();
            }
            if dist < 30.0 && run.rng.chance(4) {
                sfx.write(Sfx::at("wyrm_screech", c.body.pos));
            }
        }
        Beast::Stalker => {
            // Walks (and climbs) toward you. Never digs; won't enter water.
            let back_off = c.retreat > 0.0;
            c.dir = (target.x - c.body.pos.x).signum() * if back_off { -1.0 } else { 1.0 };
            let ahead_x = (c.body.pos.x + c.dir * (c.body.half_w + 2.0)) as i32;
            let water_ahead = (0..c.body.height as i32)
                .step_by(3)
                .any(|dy| world.material(ahead_x, c.body.pos.y as i32 - dy).kind() == Kind::Liquid);
            c.body.vel.x = if water_ahead { 0.0 } else { c.dir * 52.0 };
            if c.body.wall != 0 && !water_ahead {
                c.body.vel.y = -42.0;
            } else {
                c.body.vel.y = (c.body.vel.y + 400.0 * dt).min(260.0);
            }
            c.body.step(world, dt, 4, default_solid);
            if c.timer <= 0.0 && c.body.vel.x.abs() > 1.0 {
                c.timer = 0.55;
                sfx.write(Sfx::at("stalker_step", c.body.pos).volume(0.7));
            }
            let _ = bursts;
        }
    }
}

fn element_row(e: Element) -> usize {
    match e {
        Element::Fire => 0,
        Element::Frost => 3,
        _ => 6,
    }
}

#[allow(clippy::too_many_arguments)]
fn die(
    entity: Entity,
    c: &Creature,
    world: &mut World,
    run: &mut Run,
    save: &mut SaveData,
    commands: &mut Commands,
    assets: &GameAssets,
    segments: &Query<(Entity, &WyrmSegment)>,
    sfx: &mut MessageWriter<Sfx>,
    bursts: &mut MessageWriter<Burst>,
) {
    commands.entity(entity).despawn();
    for (e, s) in segments.iter() {
        if s.owner == entity {
            commands.entity(e).despawn();
        }
    }
    run.stats.creatures_killed += 1;
    if c.last_hit == Some(Element::Electric) {
        grant(save, run, AchievementId::Overcharged, sfx);
    }
    let at = c.body.center();
    let (x, y) = (at.x as i32, at.y as i32);
    let mut shards = 3;
    match c.kind {
        Beast::SporeDrifter | Beast::SporePuppet => {
            let grow = if c.last_hit == Some(Element::Fire) {
                Material::Fire
            } else {
                Material::Fungus
            };
            for (cx, cy) in disc(x, y, 4) {
                if world.material(cx, cy).is_open() && run.rng.chance(90) {
                    world.set(cx, cy, grow);
                }
            }
            bursts.write(
                Burst::new(at, Color::srgb(0.6, 1.0, 0.6))
                    .count(20)
                    .speed(40.0)
                    .gravity(-10.0),
            );
            sfx.write(Sfx::at("puppet_burst", at));
        }
        Beast::CinderWraith => {
            if world.material(x, y).is_open() {
                world.set(x, y, Material::Obsidian);
            }
            bursts.write(Burst::new(at, Color::srgb(1.0, 0.5, 0.1)).count(16).speed(40.0));
            sfx.write(Sfx::at("wraith_hiss", at));
        }
        Beast::Mimic => {
            shards = 20;
            sfx.write(Sfx::at("mimic_bite", at).pitch(0.3));
        }
        Beast::BlindWyrm => {
            shards = 12;
            sfx.write(Sfx::at("wyrm_screech", at));
        }
        _ => {
            bursts.write(Burst::new(at, Color::srgb(0.5, 0.45, 0.5)).count(12).speed(40.0));
            sfx.write(Sfx::at("hurt", at).volume(0.4).pitch(0.3));
        }
    }
    for i in 0..shards {
        let v = Vec2::new((i % 5) as f32 - 2.0, -2.0) * 25.0;
        commands.spawn(ShardPickup::bundle(at, v, 1, assets));
    }
}

/// Twisted bolts from the Hollowed: they hit cells and the wizard.
pub fn update_enemy_bolts(
    mut commands: Commands,
    time: Res<Time>,
    mut session: ResMut<Session>,
    mut player: ResMut<RunPlayer>,
    mut run: ResMut<Run>,
    mut q: Query<(Entity, &mut EnemyBolt, &mut Transform, &mut Sprite)>,
    mut sfx: MessageWriter<Sfx>,
    mut shake: ResMut<Shake>,
    mut bursts: MessageWriter<Burst>,
) {
    let Some(world) = session.world.as_mut() else {
        return;
    };
    let dt = time.delta_secs();
    let me = player.body.center();
    for (e, mut b, mut tf, mut sprite) in &mut q {
        b.age += dt;
        let next = b.pos + b.vel * dt;
        let hit_wall = !world.material(next.x as i32, next.y as i32).is_open();
        let hit_me = next.distance(me) < 5.0;
        b.pos = next;
        tf.translation = to_world(b.pos, 5.4);
        tf.rotation = Quat::from_rotation_z(-b.vel.to_angle());
        if let Some(atlas) = &mut sprite.texture_atlas {
            atlas.index = element_row(b.element) * 8 + (b.age * 12.0) as usize % 4;
        }
        if !(hit_wall || hit_me || b.age > 3.0) {
            continue;
        }
        commands.entity(e).despawn();
        let (x, y) = (b.pos.x as i32, b.pos.y as i32);
        let (mat, kind, color) = match b.element {
            Element::Fire => (Material::Fire, DamageKind::Fire, Color::srgb(1.0, 0.5, 0.2)),
            Element::Frost => (Material::Ice, DamageKind::Creature, Color::srgb(0.7, 0.9, 1.0)),
            _ => (Material::Acid, DamageKind::Acid, Color::srgb(0.6, 1.0, 0.3)),
        };
        for (cx, cy) in disc(x, y, 2) {
            if world.material(cx, cy).is_open() && run.rng.chance(160) {
                world.set(cx, cy, mat);
            }
        }
        if b.pos.distance(me) < 9.0 {
            let knock = b.vel.normalize_or_zero() * 60.0;
            hurt(
                &mut player,
                &mut run,
                14.0,
                kind,
                Some(knock),
                &mut sfx,
                &mut shake,
            );
            if b.element == Element::Frost {
                player.stun = player.stun.max(0.4);
            }
        }
        bursts.write(Burst::new(b.pos, color).count(10).speed(40.0));
        sfx.write(Sfx::at(super::spells::impact_sound(None), b.pos).volume(0.6));
    }
}

/// Brings the Hollow Stalker once per run, in the deep, out of sight.
pub fn stalker_director(
    mut commands: Commands,
    time: Res<Time>,
    assets: Res<GameAssets>,
    session: Res<Session>,
    player: Res<RunPlayer>,
    mut run: ResMut<Run>,
    mut state: ResMut<StalkerState>,
    mut sfx: MessageWriter<Sfx>,
) {
    let Some(world) = &session.world else { return };
    if state.spawned || !run.is_playing() || run.layer < Layer::DeepMantle || run.layer == Layer::Core {
        return;
    }
    state.deep_time += time.delta_secs();
    if state.deep_time < 45.0 || !run.rng.chance(2) {
        return;
    }
    // A standing spot 90–140 cells away.
    let me = player.body.pos;
    for _ in 0..60 {
        let a = run.rng.next_f32() * std::f32::consts::TAU;
        let d = 90.0 + run.rng.next_f32() * 50.0;
        let p = me + Vec2::from_angle(a) * d;
        let (x, y) = (p.x as i32, p.y as i32);
        let fits = (0..26).all(|dy| (-3..=3).all(|dx| world.material(x + dx, y - dy).is_open()))
            && (-3..=3).all(|dx| world.is_solid(x + dx, y + 1));
        if fits {
            spawn_creature(
                &mut commands,
                &assets,
                Beast::Stalker,
                Vec2::new(x as f32 + 0.5, y as f32 + 1.0),
                x + y,
            );
            state.spawned = true;
            sfx.write(Sfx::ui("stalker_appear"));
            return;
        }
    }
}

/// Damage creatures caught in an explosion (the Stalker shrugs it off).
pub fn blast_creatures(q: &mut Query<(Entity, &mut Creature)>, at: Vec2, radius: f32) {
    for (_, mut c) in q.iter_mut() {
        let d = c.body.center().distance(at);
        if d < radius * 2.0 {
            if c.kind != Beast::Stalker {
                c.hp -= 80.0 * (1.0 - d / (radius * 2.0));
                c.hurt_flash = 0.3;
            }
            let push = (c.body.center() - at).normalize_or_zero() * 150.0;
            c.body.vel += push;
        }
    }
}

pub fn animate_creatures(
    mut q: Query<(&Creature, &mut Sprite, &mut Transform), Without<WyrmSegment>>,
    mut segments: Query<(&WyrmSegment, &mut Transform), Without<Creature>>,
    creatures: Query<&Creature>,
) {
    for (c, mut sprite, mut tf) in &mut q {
        if let Some(atlas) = &mut sprite.texture_atlas {
            atlas.index = match c.kind {
                Beast::Stalker => {
                    if c.body.vel.x.abs() > 1.0 {
                        (c.anim * 8.0) as usize % 6
                    } else {
                        6 + (c.anim * 2.0) as usize % 2
                    }
                }
                Beast::Hollowed if c.timer > 3.0 => c.kind.row() * 8 + 4 + (c.anim * 10.0) as usize % 4,
                Beast::Mimic if c.timer > 0.6 => c.kind.row() * 8 + 4 + (c.anim * 12.0) as usize % 4,
                _ => c.kind.row() * 8 + (c.anim * 8.0) as usize % 4,
            };
        }
        sprite.flip_x = c.dir < 0.0;
        sprite.color = if c.hurt_flash > 0.0 {
            Color::srgb(1.0, 0.4, 0.4)
        } else if c.stun > 0.0 {
            Color::srgb(0.7, 0.85, 1.0)
        } else {
            Color::WHITE
        };
        let offset = if c.kind.flies() {
            c.body.height / 2.0
        } else if c.kind == Beast::Stalker {
            16.0
        } else {
            8.0
        };
        let rot = if c.kind == Beast::BlindWyrm {
            let d = c.goal.map_or(Vec2::X, |g| g - c.body.pos);
            Quat::from_rotation_z(-d.to_angle())
        } else {
            Quat::IDENTITY
        };
        tf.translation = to_world(c.body.pos - Vec2::Y * offset, 4.8).round();
        tf.rotation = rot;
    }
    for (s, mut tf) in &mut segments {
        let Ok(c) = creatures.get(s.owner) else { continue };
        let n = c.trail.len();
        if n == 0 {
            continue;
        }
        let i = n.saturating_sub(2 + s.index).min(n - 1);
        tf.translation = to_world(c.trail[i] - Vec2::Y * 4.0, 4.7).round();
    }
}
