//! Draws every player: body, suit, team-colour accents and one mod
//! attachment per body slot, all following the pose their client sends.

use std::collections::HashMap;

use bevy::prelude::*;
use sbct_sim::colony::{PLAYER_HEIGHT, PlayerKey, PublicPlayer, pose_flag};

use crate::assets::{GameAssets, PlayerAnchors, player_anim};
use crate::player::{LocalPlayer, hand_pos};
use crate::render::{InGameEntity, z};
use crate::session::Session;
use crate::ui::TEAM;

/// Body slot of the back, whose attachment is drawn behind the body.
const BACK_SLOT: usize = 3;

#[derive(Component)]
pub struct Avatar {
    pub key: PlayerKey,
    anim: u8,
    anim_time: f32,
    /// Smoothed position (cells).
    pos: Vec2,
}

#[derive(Component, Clone, Copy, PartialEq, Eq)]
pub enum Layer {
    Body,
    Suit,
    AccentBody,
    AccentSuit,
    Mod(usize),
}

/// The multitool's beam for a player who is digging.
#[derive(Component)]
pub struct DigBeam {
    key: PlayerKey,
    /// 0–1: how far an Extendo-Arm has stretched toward its target.
    ext: f32,
    /// Where it last reached, so it can be seen drawing back.
    last: Vec2,
}

/// Body slot and set of the Extendo-Arms.
const ARMS_SLOT: usize = 4;
const EXTENDO_ARMS: u8 = 40;

fn team(color: u8) -> Color {
    let [r, g, b] = TEAM[color as usize % TEAM.len()];
    Color::srgb_u8(r, g, b)
}

/// Keeps one avatar per online player.
pub fn sync_avatars(
    mut commands: Commands,
    session: Res<Session>,
    assets: Res<GameAssets>,
    avatars: Query<(Entity, &Avatar)>,
    beams: Query<(Entity, &DigBeam)>,
) {
    let roster = session.roster();
    for (entity, avatar) in &avatars {
        if !roster.iter().any(|p| p.key == avatar.key) {
            commands.entity(entity).despawn();
        }
    }
    for (entity, beam) in &beams {
        if !roster.iter().any(|p| p.key == beam.key) {
            commands.entity(entity).despawn();
        }
    }
    let Some(colony) = &session.colony else { return };
    for public in &roster {
        if avatars.iter().any(|(_, a)| a.key == public.key) {
            continue;
        }
        let pos = colony
            .players
            .get(&public.key)
            .map_or(Vec2::ZERO, |p| Vec2::new(p.pose.pos.x, p.pose.pos.y));
        commands
            .spawn((
                Avatar {
                    key: public.key,
                    anim: player_anim::IDLE,
                    anim_time: 0.0,
                    pos,
                },
                InGameEntity,
                Transform::from_xyz(pos.x, -pos.y, z::PLAYERS),
                Visibility::Visible,
            ))
            .with_children(|parent| {
                let layers = [
                    (Layer::Body, &assets.player_body, 0.0),
                    (Layer::Suit, &assets.player_suit, 0.01),
                    (Layer::AccentBody, &assets.player_accent_body, 0.02),
                    (Layer::AccentSuit, &assets.player_accent_suit, 0.03),
                ];
                for (layer, sheet, dz) in layers {
                    parent.spawn((layer, sheet.sprite(0), Transform::from_xyz(0.0, 0.0, dz)));
                }
                for slot in 0..8 {
                    let dz = if slot == BACK_SLOT {
                        -0.05
                    } else {
                        0.04 + slot as f32 * 0.005
                    };
                    parent.spawn((
                        Layer::Mod(slot),
                        assets.mods.sprite(0),
                        Transform::from_xyz(0.0, 0.0, dz),
                        Visibility::Hidden,
                    ));
                }
            });
        commands.spawn((
            DigBeam {
                key: public.key,
                ext: 0.0,
                last: Vec2::ZERO,
            },
            InGameEntity,
            Sprite::from_color(Color::srgba(0.5, 1.0, 0.9, 0.8), Vec2::new(1.0, 1.5)),
            Transform::from_xyz(0.0, 0.0, z::BOLTS),
            Visibility::Hidden,
        ));
    }
}

pub fn animate_avatars(
    time: Res<Time>,
    session: Res<Session>,
    local: Res<LocalPlayer>,
    anchors: Res<PlayerAnchors>,
    mut avatars: Query<(&mut Avatar, &mut Transform, &Children), Without<Layer>>,
    mut layers: Query<(&Layer, &mut Sprite, &mut Transform, &mut Visibility), Without<Avatar>>,
    mut beams: Query<
        (&mut DigBeam, &mut Sprite, &mut Transform, &mut Visibility),
        (Without<Avatar>, Without<Layer>),
    >,
) {
    let Some(colony) = &session.colony else { return };
    let Some(world) = &session.world else { return };
    let roster: HashMap<PlayerKey, PublicPlayer> = session.roster().into_iter().map(|p| (p.key, p)).collect();
    let dt = time.delta_secs();
    let t = time.elapsed_secs();

    for (mut avatar, mut tf, children) in &mut avatars {
        let (Some(player), Some(public)) = (colony.players.get(&avatar.key), roster.get(&avatar.key)) else {
            continue;
        };
        let mine = avatar.key == session.me;
        let pose = player.pose;
        let target = Vec2::new(pose.pos.x, pose.pos.y);
        // Teammates arrive at 20 Hz; smooth them out.
        avatar.pos = if mine || avatar.pos.distance(target) > 120.0 {
            target
        } else {
            avatar.pos.lerp(target, 1.0 - (-18.0 * dt).exp())
        };
        let anim = if public.dead && !mine {
            player_anim::BLACKOUT
        } else {
            pose.anim
        };
        if anim != avatar.anim {
            avatar.anim = anim;
            avatar.anim_time = 0.0;
        }
        // Running animates with ground speed.
        let rate = if anim == player_anim::RUN {
            (pose.vel.x.abs() / 80.0).clamp(0.4, 1.6)
        } else {
            1.0
        };
        avatar.anim_time += dt * rate;
        let index = player_anim::index(avatar.anim, avatar.anim_time);
        let facing = if pose.facing < 0 { -1.0 } else { 1.0 };
        // The sprite is 32 tall with feet on its bottom row.
        tf.translation = Vec3::new(
            avatar.pos.x,
            -(avatar.pos.y - 16.0),
            z::PLAYERS + if mine { 0.3 } else { 0.0 },
        );
        tf.scale = Vec3::new(facing, 1.0, 1.0);

        let suit = public.suit;
        let color = team(public.color);
        for child in children.iter() {
            let Ok((layer, mut sprite, mut ltf, mut vis)) = layers.get_mut(child) else {
                continue;
            };
            let show = match layer {
                Layer::Body => true,
                Layer::Suit => suit,
                Layer::AccentBody => !suit,
                Layer::AccentSuit => suit,
                Layer::Mod(slot) => {
                    match (public.mods[*slot], anchors.offset(*slot, index)) {
                        (Some((set, tier)), Some(offset)) => {
                            // Columns: tier × 2 animation frames.
                            let frame = ((t * 4.0) as usize) % 2;
                            if let Some(atlas) = &mut sprite.texture_atlas {
                                atlas.index = set as usize * 8 + (tier.clamp(1, 4) as usize - 1) * 2 + frame;
                            }
                            ltf.translation.x = offset.x;
                            ltf.translation.y = offset.y;
                            true
                        }
                        _ => false,
                    }
                }
            };
            *vis = if show {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            if !matches!(layer, Layer::Mod(_)) {
                if let Some(atlas) = &mut sprite.texture_atlas {
                    atlas.index = index;
                }
                if matches!(layer, Layer::AccentBody | Layer::AccentSuit) {
                    sprite.color = color;
                }
            }
        }
    }

    // Dig beams: from the hand to where the tool bites.
    for (mut beam, mut sprite, mut tf, mut vis) in &mut beams {
        let Some(player) = colony.players.get(&beam.key) else {
            *vis = Visibility::Hidden;
            continue;
        };
        let pose = player.pose;
        let from = hand_pos(pose.pos, pose.facing, pose.aim);
        let to = if beam.key == session.me {
            local.digging
        } else if pose.flags & pose_flag::TOOL != 0 {
            // We only know the aim: march along it to the first solid cell.
            let reach = player.stats().reach;
            let dir = sbct_sim::colony::geom::v2(pose.aim.cos(), pose.aim.sin());
            let mut end = from;
            let mut d = 0.0;
            while d < reach {
                end = from + dir * d;
                let (x, y) = end.cell();
                if world.material(x, y).is_solid_for_player() {
                    break;
                }
                d += 2.0;
            }
            Some(end)
        } else {
            None
        };
        let extendo = roster
            .get(&beam.key)
            .is_some_and(|p| matches!(p.mods[ARMS_SLOT], Some((EXTENDO_ARMS, _))));
        if extendo {
            // The arm shoots out to where it works and draws back after.
            if let Some(to) = to {
                beam.last = Vec2::new(to.x, to.y);
                beam.ext = (beam.ext + dt * 9.0).min(1.0);
            } else {
                beam.ext = (beam.ext - dt * 7.0).max(0.0);
            }
            if beam.ext <= 0.0 {
                *vis = Visibility::Hidden;
                continue;
            }
            let shoulder = Vec2::new(pose.pos.x + pose.facing as f32 * 2.0, pose.pos.y - 14.0);
            let tip = shoulder.lerp(beam.last, beam.ext);
            let (a, b) = (Vec2::new(shoulder.x, -shoulder.y), Vec2::new(tip.x, -tip.y));
            let d = b - a;
            *vis = Visibility::Visible;
            sprite.custom_size = Some(Vec2::new(d.length().max(1.0), 2.0));
            // Banded like a telescoping arm.
            let band = 0.72 + 0.1 * ((d.length() * 0.5 + t * 6.0).sin());
            sprite.color = Color::srgb(band, band + 0.04, band + 0.08);
            tf.translation = ((a + b) / 2.0).extend(z::PLAYERS + 0.5);
            tf.rotation = Quat::from_rotation_z(d.y.atan2(d.x));
            continue;
        }
        let Some(to) = to else {
            *vis = Visibility::Hidden;
            continue;
        };
        let (a, b) = (Vec2::new(from.x, -from.y), Vec2::new(to.x, -to.y));
        let d = b - a;
        *vis = Visibility::Visible;
        let flicker = 0.6 + 0.4 * ((t * 40.0).sin() * 0.5 + 0.5);
        sprite.custom_size = Some(Vec2::new(d.length().max(1.0), 1.0 + flicker));
        sprite.color = Color::srgba(0.55, 1.0, 0.9, 0.55 + 0.3 * flicker);
        tf.translation = ((a + b) / 2.0).extend(z::BOLTS);
        tf.rotation = Quat::from_rotation_z(d.y.atan2(d.x));
        let _ = PLAYER_HEIGHT;
    }
}
