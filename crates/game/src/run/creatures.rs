//! Simple creatures. They matter less than the environment and they live in
//! it too: they burn, drown, dissolve, get crushed and get blown up.

use bevy::prelude::*;
use sbct_sim::descent::{CreatureKind, SpawnKind};
use sbct_sim::{Kind, Material};

use super::hazards::hurt;
use super::items::ItemId;
use super::physics::{Body, default_solid, to_world};
use super::player::RunPlayer;
use super::{DamageKind, Run, RunSetup};
use crate::assets::GameAssets;
use crate::audio::Sfx;
use crate::fx::{Burst, Shake};
use crate::render::InGameEntity;
use crate::session::Session;

/// Creatures farther than this from the player are frozen (saves work).
const ACTIVE_RANGE: f32 = 280.0;

#[derive(Component)]
pub struct Creature {
    pub kind: CreatureKind,
    pub body: Body,
    pub hp: f32,
    pub dir: f32,
    /// Seconds spent underwater (crawlers drown).
    pub soaked: f32,
    pub anim: f32,
    pub hurt_flash: f32,
    /// Magma slugs leave burning trails.
    pub trail: f32,
}

pub fn spawn_creatures(mut commands: Commands, setup: Res<RunSetup>, assets: Res<GameAssets>) {
    for s in &setup.spawns {
        let SpawnKind::Creature(kind) = s.kind else { continue };
        let (half_w, h, hp) = match kind {
            CreatureKind::Crawler => (4.0, 6.0, 30.0),
            CreatureKind::MagmaSlug => (5.0, 5.0, 45.0),
            CreatureKind::SporeDrifter => (3.0, 7.0, 15.0),
        };
        let pos = Vec2::new(s.x as f32 + 0.5, s.y as f32 + 1.0);
        let dir = if s.x % 2 == 0 { 1.0 } else { -1.0 };
        commands.spawn((
            Creature { kind, body: Body::new(pos, half_w, h), hp, dir, soaked: 0.0, anim: 0.0, hurt_flash: 0.0, trail: 0.0 },
            InGameEntity,
            assets.creatures.sprite(row(kind) * 4),
            Transform::from_translation(to_world(pos - Vec2::Y * 8.0, 4.8)),
        ));
    }
}

fn row(kind: CreatureKind) -> usize {
    match kind {
        CreatureKind::Crawler => 0,
        CreatureKind::MagmaSlug => 1,
        CreatureKind::SporeDrifter => 2,
    }
}

pub fn update_creatures(
    mut commands: Commands,
    time: Res<Time>,
    mut session: ResMut<Session>,
    mut player: ResMut<RunPlayer>,
    mut run: ResMut<Run>,
    mut q: Query<(Entity, &mut Creature)>,
    mut sfx: MessageWriter<Sfx>,
    mut shake: ResMut<Shake>,
    mut bursts: MessageWriter<Burst>,
) {
    let Some(world) = session.world.as_mut() else { return };
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    let target = player.body.center();
    let aggro = if run.has(ItemId::BeaconHeart) { 170.0 } else { 70.0 };

    for (entity, mut c) in &mut q {
        if c.body.center().distance(target) > ACTIVE_RANGE {
            continue;
        }
        c.anim += dt;
        c.hurt_flash = (c.hurt_flash - dt).max(0.0);
        let fire_immune = c.kind == CreatureKind::MagmaSlug;

        // Environment damage, from the same cells that hurt the player.
        let (mut hot, mut acid, mut water) = (0, 0, 0);
        c.body.for_each_cell(|x, y| match world.material(x, y) {
            m if m.kind() == Kind::Fire || m == Material::Lava || m == Material::Metal => hot += 1,
            Material::Acid => acid += 1,
            Material::Water => water += 1,
            _ => {}
        });
        let mut damage = 0.0;
        if hot > 0 && !fire_immune {
            damage += 40.0 * dt;
        }
        if acid > 0 {
            damage += 50.0 * dt;
        }
        if c.kind == CreatureKind::MagmaSlug && water > 0 {
            damage += 60.0 * dt;
            if run.rng.chance(40) {
                bursts.write(Burst::new(c.body.center(), Color::srgba(0.85, 0.85, 0.9, 0.7)).count(3).gravity(-60.0));
            }
        }
        if c.body.submersion(world) > 0.6 && c.kind == CreatureKind::Crawler {
            c.soaked += dt;
            if c.soaked > 4.0 {
                damage += 30.0 * dt;
            }
        } else {
            c.soaked = 0.0;
        }
        if !c.body.escape_overlap(world, default_solid) {
            damage += 80.0 * dt;
        }
        if damage > 0.0 {
            c.hp -= damage;
            c.hurt_flash = 0.15;
        }

        // Movement.
        match c.kind {
            CreatureKind::Crawler | CreatureKind::MagmaSlug => {
                let speed = if c.kind == CreatureKind::Crawler { 22.0 } else { 11.0 };
                let chasing = c.body.center().distance(target) < aggro * 0.6;
                if chasing && (target.y - c.body.pos.y).abs() < 20.0 {
                    c.dir = (target.x - c.body.pos.x).signum();
                }
                // Turn at walls and ledges.
                let ahead = Vec2::new(c.body.pos.x + c.dir * (c.body.half_w + 1.0), c.body.pos.y + 1.0);
                let ledge = c.body.on_ground && !world.is_solid(ahead.x as i32, ahead.y as i32) && !chasing;
                if c.body.wall != 0 || ledge {
                    c.dir = -c.dir;
                }
                c.body.vel.x = c.dir * speed;
                c.body.vel.y = (c.body.vel.y + 400.0 * dt).min(300.0);
                c.body.step(world, dt, 2, default_solid);
                if c.kind == CreatureKind::MagmaSlug {
                    c.trail -= dt;
                    if c.trail <= 0.0 {
                        c.trail = 1.4;
                        let (x, y) = (c.body.pos.x as i32, c.body.pos.y as i32);
                        if world.material(x, y - 1) == Material::Empty {
                            world.set(x, y - 1, Material::Fire);
                        }
                    }
                }
            }
            CreatureKind::SporeDrifter => {
                let to = target - c.body.center();
                let bob = (c.anim * 3.0).sin() * 12.0;
                c.body.vel = if to.length() < aggro {
                    to.normalize_or_zero() * 26.0 + Vec2::Y * bob
                } else {
                    Vec2::new(c.dir * 8.0, bob)
                };
                c.body.step(world, dt, 0, default_solid);
                if c.body.wall != 0 {
                    c.dir = -c.dir;
                }
            }
        }

        // Touching the player.
        let touching = c.body.center().distance(target) < c.body.half_w + 5.0;
        if touching && run.is_playing() {
            let knock = Vec2::new((target.x - c.body.pos.x).signum() * 90.0, -80.0);
            match c.kind {
                CreatureKind::SporeDrifter => {
                    hurt(&mut player, &mut run, 12.0, DamageKind::Creature, Some(knock), &mut sfx, &mut shake);
                    c.hp = 0.0;
                }
                CreatureKind::MagmaSlug => {
                    hurt(&mut player, &mut run, 18.0, DamageKind::Creature, Some(knock), &mut sfx, &mut shake);
                    if !run.has(ItemId::SalamanderSkin) {
                        player.burning = player.burning.max(1.5);
                    }
                }
                CreatureKind::Crawler => {
                    hurt(&mut player, &mut run, 14.0, DamageKind::Creature, Some(knock), &mut sfx, &mut shake);
                }
            }
        }

        if c.hp <= 0.0 {
            commands.entity(entity).despawn();
            run.stats.creatures_killed += 1;
            let at = c.body.center();
            let (x, y) = (at.x as i32, at.y as i32);
            match c.kind {
                // Drifters burst into spores (or flame, if burning).
                CreatureKind::SporeDrifter => {
                    let grow = if hot > 0 { Material::Fire } else { Material::Fungus };
                    for (cx, cy) in sbct_sim::world::disc(x, y, 4) {
                        if world.material(cx, cy).is_open() && run.rng.chance(90) {
                            world.set(cx, cy, grow);
                        }
                    }
                    bursts.write(Burst::new(at, Color::srgb(0.6, 1.0, 0.6)).count(20).speed(40.0).gravity(-10.0));
                    sfx.write(Sfx::at("plant", at));
                }
                CreatureKind::MagmaSlug => {
                    if world.material(x, y).is_open() {
                        world.set(x, y, Material::Obsidian);
                    }
                    bursts.write(Burst::new(at, Color::srgb(1.0, 0.5, 0.1)).count(16).speed(40.0));
                    sfx.write(Sfx::at("sizzle", at));
                }
                CreatureKind::Crawler => {
                    bursts.write(Burst::new(at, Color::srgb(0.5, 0.4, 0.35)).count(12).speed(40.0));
                    sfx.write(Sfx::at("hurt", at).volume(0.4).pitch(0.3));
                }
            }
            run.ore += 3;
        }
    }
}

/// Damage creatures caught in an explosion.
pub fn blast_creatures(q: &mut Query<(Entity, &mut Creature)>, at: Vec2, radius: f32) {
    for (_, mut c) in q.iter_mut() {
        let d = c.body.center().distance(at);
        if d < radius * 2.0 {
            c.hp -= 80.0 * (1.0 - d / (radius * 2.0));
            let push = (c.body.center() - at).normalize_or_zero() * 150.0;
            c.body.vel += push;
            c.hurt_flash = 0.3;
        }
    }
}

pub fn animate_creatures(mut q: Query<(&Creature, &mut Sprite, &mut Transform)>) {
    for (c, mut sprite, mut tf) in &mut q {
        let frame = (c.anim * 8.0) as usize % 4;
        if let Some(atlas) = &mut sprite.texture_atlas {
            atlas.index = row(c.kind) * 4 + frame;
        }
        sprite.flip_x = c.dir < 0.0;
        sprite.color = if c.hurt_flash > 0.0 { Color::srgb(1.0, 0.4, 0.4) } else { Color::WHITE };
        let offset = if c.kind == CreatureKind::SporeDrifter { c.body.height / 2.0 } else { 8.0 };
        tf.translation = to_world(c.body.pos - Vec2::Y * offset, 4.8).round();
    }
}
