//! What body mods look like in the world beyond the sprite attachments:
//! drones, the free-flying arm, the raincloud, the grapple line, headlamps
//! and the X-ray sparkle. Everything here is drawn from replicated state,
//! so teammates see each other's gadgets.

use bevy::prelude::*;
use sbct_sim::colony::{EntKind, PlayerKey};

use crate::assets::GameAssets;
use crate::fx::Burst;
use crate::lighting::PointLights;
use crate::player::{LocalPlayer, hand_pos};
use crate::render::{InGameEntity, z};
use crate::session::Session;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Kind {
    Drone(u8),
    FreeArm,
    Cloud(u8),
    Grapple,
}

/// A gadget sprite belonging to a player.
#[derive(Component)]
pub struct Gadget {
    owner: PlayerKey,
    kind: Kind,
    /// Smoothed position in cells.
    pos: Vec2,
}

/// Mod set whose Legendary attachment stands in for the free arm.
const EXTENDO_ARMS: usize = 40;

pub fn sync_gadgets(
    mut commands: Commands,
    session: Res<Session>,
    local: Res<LocalPlayer>,
    assets: Res<GameAssets>,
    time: Res<Time>,
    mut bursts: MessageWriter<Burst>,
    mut gadgets: Query<(Entity, &mut Gadget, &mut Transform, &mut Sprite)>,
) {
    let Some(colony) = &session.colony else { return };
    let t = time.elapsed_secs();
    let dt = time.delta_secs();
    // What should exist.
    let mut wanted: Vec<(PlayerKey, Kind)> = Vec::new();
    let roster = session.roster();
    for public in &roster {
        if public.dead {
            continue;
        }
        let stats = public.stats();
        for i in 0..stats.drones {
            wanted.push((public.key, Kind::Drone(i)));
        }
        if public.arm.is_some() {
            wanted.push((public.key, Kind::FreeArm));
        }
        if stats.sprinkler >= 70.0 {
            for i in 0..3 {
                wanted.push((public.key, Kind::Cloud(i)));
            }
        }
    }
    if local.grapple.is_some() {
        wanted.push((session.me, Kind::Grapple));
    }
    for (entity, gadget, ..) in &gadgets {
        if !wanted.contains(&(gadget.owner, gadget.kind)) {
            commands.entity(entity).despawn();
        }
    }
    for &(owner, kind) in &wanted {
        if gadgets
            .iter()
            .any(|(_, g, ..)| g.owner == owner && g.kind == kind)
        {
            continue;
        }
        let Some(p) = colony.players.get(&owner) else {
            continue;
        };
        let at = Vec2::new(p.pose.pos.x, p.pose.pos.y - 12.0);
        let sprite = match kind {
            Kind::Drone(_) => {
                let mut s = assets.robot.sprite(0);
                s.custom_size = Some(Vec2::splat(10.0));
                s
            }
            Kind::FreeArm => assets.mods.sprite(EXTENDO_ARMS * 8 + 6),
            Kind::Cloud(i) => Sprite::from_color(
                [
                    Color::srgba(0.42, 0.5, 0.58, 0.82),
                    Color::srgba(0.56, 0.64, 0.72, 0.82),
                    Color::srgba(0.5, 0.58, 0.66, 0.82),
                ][i as usize],
                [Vec2::new(22.0, 4.0), Vec2::new(10.0, 4.0), Vec2::new(12.0, 5.0)][i as usize],
            ),
            Kind::Grapple => Sprite::from_color(Color::srgb(0.82, 0.8, 0.66), Vec2::new(1.0, 1.0)),
        };
        commands.spawn((
            Gadget { owner, kind, pos: at },
            InGameEntity,
            sprite,
            Transform::from_xyz(at.x, -at.y, z::PLAYERS + 0.6),
        ));
    }

    for (_, mut gadget, mut tf, mut sprite) in &mut gadgets {
        let Some(p) = colony.players.get(&gadget.owner) else {
            continue;
        };
        let feet = Vec2::new(p.pose.pos.x, p.pose.pos.y);
        let chest = feet - Vec2::Y * 12.0;
        match gadget.kind {
            Kind::Drone(i) => {
                // Circle the owner, each drone at its own height and phase.
                let a = t * 1.7 + i as f32 * 1.9;
                let target =
                    chest + Vec2::new(a.cos() * (18.0 + i as f32 * 5.0), -14.0 + (a * 1.3).sin() * 5.0);
                gadget.pos = gadget.pos.lerp(target, (6.0 * dt).min(1.0));
                if gadget.pos.distance(target) > 200.0 {
                    gadget.pos = target;
                }
                tf.translation = Vec3::new(gadget.pos.x, -gadget.pos.y, z::PLAYERS + 0.6);
                sprite.flip_x = a.sin() > 0.0;
                if let Some(atlas) = &mut sprite.texture_atlas {
                    atlas.index = (t * 8.0) as usize % 4;
                }
            }
            Kind::FreeArm => {
                let Some(target) = roster.iter().find(|r| r.key == gadget.owner).and_then(|r| r.arm) else {
                    continue;
                };
                // It flies out from its owner when sent.
                let target = Vec2::new(target.x, target.y - 8.0);
                let step = (260.0 * dt).min(gadget.pos.distance(target));
                let dir = (target - gadget.pos).normalize_or_zero();
                gadget.pos += dir * step;
                if gadget.pos.distance(target) > 900.0 {
                    gadget.pos = target;
                }
                let bob = (t * 3.0).sin() * 1.5;
                tf.translation = Vec3::new(gadget.pos.x, -gadget.pos.y + bob, z::PLAYERS + 0.6);
                if let Some(atlas) = &mut sprite.texture_atlas {
                    atlas.index = EXTENDO_ARMS * 8 + 6 + (t * 4.0) as usize % 2;
                }
            }
            Kind::Cloud(i) => {
                let offset = [
                    Vec2::new(0.0, -44.0),
                    Vec2::new(-5.0, -47.0),
                    Vec2::new(4.0, -48.0),
                ][i as usize];
                let drift = Vec2::new((t * 0.8 + i as f32).sin() * 2.0, 0.0);
                gadget.pos = gadget.pos.lerp(feet + offset + drift, (4.0 * dt).min(1.0));
                tf.translation = Vec3::new(gadget.pos.x, -gadget.pos.y, z::DOME_FRONT + 0.1);
                if i == 0 && (t * 20.0).fract() < dt * 20.0 {
                    let x = ((t * 37.0).sin() * 0.5) * 24.0;
                    bursts.write(
                        Burst::new(gadget.pos + Vec2::new(x, 4.0), Color::srgb(0.5, 0.72, 0.95))
                            .count(1)
                            .speed(70.0)
                            .dir(Vec2::Y)
                            .gravity(200.0)
                            .life(0.45),
                    );
                }
            }
            Kind::Grapple => {
                let Some(hook) = local.grapple else { continue };
                let from = hand_pos(p.pose.pos, p.pose.facing, p.pose.aim);
                let (a, b) = (Vec2::new(from.x, -from.y), Vec2::new(hook.x, -hook.y));
                let d = b - a;
                sprite.custom_size = Some(Vec2::new(d.length().max(1.0), 1.0));
                tf.translation = ((a + b) / 2.0).extend(z::BOLTS);
                tf.rotation = Quat::from_rotation_z(d.y.atan2(d.x));
            }
        }
    }

    // Sprinkler mist and the gadgets other players' movement mods show.
    for public in &roster {
        let Some(p) = colony.players.get(&public.key) else {
            continue;
        };
        let stats = public.stats();
        let feet = Vec2::new(p.pose.pos.x, p.pose.pos.y);
        if stats.sprinkler > 0.0 && stats.sprinkler < 70.0 && (t * 6.0).fract() < dt * 6.0 {
            bursts.write(
                Burst::new(feet - Vec2::Y * 20.0, Color::srgb(0.55, 0.78, 0.98))
                    .count(3)
                    .speed(stats.sprinkler * 1.6)
                    .gravity(120.0)
                    .life(0.5),
            );
        }
        // Teammates' rocket boots (our own are drawn by the controller).
        if public.key != session.me && p.pose.flags & sbct_sim::colony::pose_flag::THRUST != 0 {
            bursts.write(
                Burst::new(feet, Color::srgb(1.0, 0.7, 0.22))
                    .count(2)
                    .speed(70.0)
                    .dir(Vec2::Y)
                    .gravity(30.0)
                    .life(0.3),
            );
        }
    }
}

/// Headlamps: light thrown ahead of anyone wearing one.
pub fn mod_lights(session: Res<Session>, mut lights: ResMut<PointLights>) {
    let Some(colony) = &session.colony else { return };
    for public in session.roster() {
        let lamp = public.stats().headlamp;
        if lamp == 0 || public.dead {
            continue;
        }
        let Some(p) = colony.players.get(&public.key) else {
            continue;
        };
        let head = p.body().head();
        let dir = Vec2::new(p.pose.aim.cos(), p.pose.aim.sin());
        let (reach, power) = match lamp {
            1 => (3, 2.6),
            2 => (6, 3.4),
            _ => (9, 4.2),
        };
        for i in 1..=reach {
            let at = Vec2::new(head.x, head.y) + dir * (i as f32 * 12.0);
            lights.0.push((at, [power, power, power * 0.86]));
        }
    }
}

/// A sparkle over ore (or a marker over a creature) seen through rock.
#[derive(Component)]
pub struct XraySpark;

#[derive(Resource, Default)]
pub struct Xray {
    /// (cell position, colour, is a creature).
    marks: Vec<(Vec2, [u8; 3], bool)>,
    rescan_in: f32,
}

const SPARKS: usize = 220;

pub fn xray(
    mut commands: Commands,
    session: Res<Session>,
    time: Res<Time>,
    mut state: ResMut<Xray>,
    mut sparks: Query<(&mut Sprite, &mut Transform, &mut Visibility), With<XraySpark>>,
) {
    let (Some(world), Some(colony), Some(me)) = (&session.world, &session.colony, session.player()) else {
        return;
    };
    let stats = me.stats();
    if stats.xray <= 0.0 || !me.alive() {
        for (_, _, mut vis) in &mut sparks {
            *vis = Visibility::Hidden;
        }
        state.marks.clear();
        return;
    }
    if sparks.is_empty() {
        for _ in 0..SPARKS {
            commands.spawn((
                XraySpark,
                InGameEntity,
                Sprite::from_color(Color::WHITE, Vec2::splat(1.4)),
                Transform::from_xyz(0.0, 0.0, z::OVERLAY),
                Visibility::Hidden,
            ));
        }
        return;
    }
    state.rescan_in -= time.delta_secs();
    if state.rescan_in <= 0.0 {
        state.rescan_in = 0.4;
        state.marks.clear();
        let at = me.center();
        // "Everything on screen" still only needs a screen's worth.
        let radius = stats.xray.min(190.0) as i32;
        let (cx, cy) = at.cell();
        let mut y = cy - radius;
        while y <= cy + radius {
            let mut x = cx - radius;
            while x <= cx + radius {
                let d2 = (x - cx) * (x - cx) + (y - cy) * (y - cy);
                let m = world.material(x, y);
                // A stable scatter of the ore cells, so the sparkle doesn't crawl.
                if d2 <= radius * radius && m.is_ore() && sbct_sim::rng::hash2(11, x, y).is_multiple_of(7) {
                    state
                        .marks
                        .push((Vec2::new(x as f32 + 0.5, y as f32 + 0.5), m.props().color, false));
                }
                x += 2;
            }
            y += 2;
        }
        // Nearest first, so the pool goes to what is closest.
        state.marks.sort_by(|a, b| {
            a.0.distance_squared(Vec2::new(at.x, at.y))
                .total_cmp(&b.0.distance_squared(Vec2::new(at.x, at.y)))
        });
        state.marks.truncate(SPARKS - 30);
        if stats.xray_creatures {
            for e in colony.ents.values() {
                if let EntKind::Creature(c) = &e.kind
                    && e.pos.distance(at) < stats.xray.min(260.0)
                {
                    let color = if c.kind.predator() {
                        [255, 90, 80]
                    } else {
                        [140, 230, 160]
                    };
                    let h = c.kind.size().1;
                    state
                        .marks
                        .push((Vec2::new(e.pos.x, e.pos.y - h / 2.0), color, true));
                }
            }
        }
    }
    let t = time.elapsed_secs();
    let mut marks = state.marks.iter();
    for (i, (mut sprite, mut tf, mut vis)) in sparks.iter_mut().enumerate() {
        let Some(&(pos, [r, g, b], creature)) = marks.next() else {
            *vis = Visibility::Hidden;
            continue;
        };
        *vis = Visibility::Visible;
        let twinkle = 0.45 + 0.55 * (t * 5.0 + i as f32 * 1.7).sin().abs();
        if creature {
            sprite.custom_size = Some(Vec2::splat(4.0));
            sprite.color = Color::srgba_u8(r, g, b, 200);
            tf.rotation = Quat::from_rotation_z(std::f32::consts::FRAC_PI_4);
        } else {
            sprite.custom_size = Some(Vec2::splat(1.4));
            let lift = |c: u8| (c as f32 / 255.0 * 0.6 + 0.4).min(1.0);
            sprite.color = Color::srgba(lift(r), lift(g), lift(b), twinkle);
            tf.rotation = Quat::IDENTITY;
        }
        tf.translation = Vec3::new(pos.x, -pos.y, z::OVERLAY);
    }
}

pub fn reset_xray(mut state: ResMut<Xray>) {
    state.marks.clear();
}
