//! Health and environmental danger: fire, lava, molten metal, acid, heat,
//! drowning, toxic gas, electricity and being crushed. All of it comes from
//! the cells around the player, so every hazard is something the simulation
//! is actually doing.

use bevy::prelude::*;
use sbct_sim::descent::Layer;
use sbct_sim::{Kind, Material};

use super::achievements::AchievementId;
use super::items::ItemId;
use super::player::RunPlayer;
use super::save::SaveData;
use super::{DamageKind, Run, grant};
use crate::audio::Sfx;
use crate::fx::{Burst, HitStop, Shake};
use crate::session::Session;

/// Seconds of invulnerability after an impact.
const IFRAMES: f32 = 0.7;
const BURN_TIME: f32 = 3.0;

/// Applies damage. `impact` hits (explosions, shocks, bites) respect
/// invulnerability frames and knock back; continuous damage doesn't.
pub fn hurt(
    player: &mut RunPlayer,
    run: &mut Run,
    amount: f32,
    kind: DamageKind,
    impact: Option<Vec2>,
    sfx: &mut MessageWriter<Sfx>,
    shake: &mut Shake,
) {
    if !run.is_playing() || amount <= 0.0 {
        return;
    }
    if impact.is_some() && player.iframes > 0.0 {
        return;
    }
    let amount = if run.has(ItemId::GlassCannonPick) { amount * 2.0 } else { amount };
    player.hp -= amount;
    run.stats.damage_taken += amount;
    if amount >= run.cause_amount {
        run.cause_amount = amount;
        run.cause = Some(kind);
    }
    if let Some(knock) = impact {
        player.iframes = IFRAMES;
        player.body.vel += knock;
        player.hurt_flash = 0.45;
        shake.add(0.35 + (amount / 60.0).min(0.5));
        sfx.write(Sfx::at("hurt", player.body.pos));
    } else if player.hurt_flash <= 0.0 {
        player.hurt_flash = 0.2;
        let name = if matches!(kind, DamageKind::Fire | DamageKind::Lava | DamageKind::Metal | DamageKind::Heat) {
            "burn"
        } else {
            "hurt"
        };
        sfx.write(Sfx::at(name, player.body.pos).volume(0.6));
    }
}

/// Ambient heat added by depth, so the deep layers press on you.
fn layer_heat(layer: Layer) -> f32 {
    [0.0, 10.0, 16.0, 34.0, 44.0][layer.index()]
}

pub fn environment(
    time: Res<Time>,
    mut player: ResMut<RunPlayer>,
    mut run: ResMut<Run>,
    session: Res<Session>,
    mut save: ResMut<SaveData>,
    mut sfx: MessageWriter<Sfx>,
    mut shake: ResMut<Shake>,
    mut stop: ResMut<HitStop>,
    mut bursts: MessageWriter<Burst>,
) {
    let Some(world) = &session.world else { return };
    let dt = time.delta_secs();
    if dt <= 0.0 || !run.is_playing() {
        return;
    }
    let p = &mut *player;
    p.iframes = (p.iframes - dt).max(0.0);
    run.cause_amount = 0.0;

    let salamander = run.has(ItemId::SalamanderSkin);
    let gills = run.has(ItemId::GillMask);
    let thermal = run.has(ItemId::ThermalSuit);

    // What are we touching?
    let (mut fire, mut lava, mut metal, mut acid, mut water, mut charged, mut spark) = (0, 0, 0, 0, 0, 0, 0);
    p.body.for_each_cell(|x, y| {
        let Some(c) = world.get(x, y) else { return };
        match c.mat {
            Material::Fire => fire += 1,
            Material::Lava => lava += 1,
            Material::Metal => metal += 1,
            Material::Acid => acid += 1,
            Material::Water => water += 1,
            Material::Spark => spark += 1,
            _ => {}
        }
        if c.mat.props().conductive && c.life > 2 {
            charged += 1;
        }
    });
    // Standing on (not just in) molten metal or lava also burns.
    let under = world.material(p.body.pos.x as i32, p.body.pos.y as i32);
    if under == Material::Metal && !run.has(ItemId::HeavyBoots) {
        metal += 1;
    }
    let head = world.material(p.body.head().x as i32, p.body.head().y as i32);

    // Continuous damage per second.
    macro_rules! damage {
        ($amount:expr, $kind:expr) => {
            hurt(p, &mut run, $amount * dt, $kind, None, &mut sfx, &mut shake)
        };
    }

    if !salamander {
        if fire > 0 {
            damage!(10.0, DamageKind::Fire);
            p.burning = BURN_TIME;
        }
        if lava > 0 {
            damage!(45.0, DamageKind::Lava);
            p.burning = BURN_TIME;
        }
        if metal > 0 {
            damage!(40.0, DamageKind::Metal);
            p.burning = BURN_TIME;
        }
    } else if water > 0 {
        damage!(12.0, DamageKind::Water);
    }
    if acid > 0 {
        damage!(28.0, DamageKind::Acid);
        if run.rng.chance(40) {
            bursts.write(Burst::new(p.body.pos, Color::srgb(0.5, 0.9, 0.2)).count(2).gravity(-30.0).speed(15.0));
        }
    }
    if charged > 0 || spark > 0 {
        let knock = Vec2::new(-p.facing * 40.0, -60.0);
        hurt(p, &mut run, if charged > 0 { 22.0 } else { 8.0 }, DamageKind::Electric, Some(knock), &mut sfx, &mut shake);
        if p.stun <= 0.0 && charged > 0 {
            p.stun = 0.35;
            stop.0 = 0.06;
            bursts.write(Burst::new(p.body.center(), Color::srgb(0.7, 0.9, 1.0)).count(14).speed(70.0).gravity(0.0));
            sfx.write(Sfx::at("spark", p.body.pos));
        }
    }

    // Burning: put it out in water, or tough it out.
    if p.burning > 0.0 {
        if p.body.submersion(world) > 0.25 {
            p.burning = 0.0;
            sfx.write(Sfx::at("sizzle", p.body.pos).volume(0.6));
            bursts.write(Burst::new(p.body.center(), Color::srgba(0.8, 0.8, 0.85, 0.7)).count(10).gravity(-60.0));
        } else if !salamander {
            p.burning -= dt;
            damage!(7.0, DamageKind::Fire);
            if run.rng.chance(120) {
                bursts.write(
                    Burst::new(p.body.center(), Color::srgb(1.0, 0.55, 0.15)).count(2).gravity(-80.0).speed(20.0).life(0.4),
                );
            }
        } else {
            p.burning = 0.0;
        }
        p.was_burning = true;
    } else if p.was_burning {
        p.was_burning = false;
        if p.hp > 0.0 {
            grant(&mut save, &mut run, AchievementId::Firewalker, &mut sfx);
        }
    }

    // Heat: hot materials nearby (solids count less), plus the depth itself.
    let c = p.body.center();
    let mut heat = layer_heat(run.layer);
    for (x, y) in sbct_sim::world::disc(c.x as i32, c.y as i32, 9) {
        let Some(cell) = world.get(x, y) else { continue };
        let h = cell.mat.props().heat as f32;
        heat += match cell.mat.kind() {
            Kind::Solid => h * 0.12,
            Kind::Liquid if cell.mat == Material::Water => -0.6,
            _ => h,
        };
    }
    if p.body.submersion(world) > 0.5 {
        heat -= 60.0;
    }
    let delta = if heat > 40.0 { (heat - 40.0) * 0.25 } else { -15.0 - (40.0 - heat) * 0.2 };
    if thermal && delta > 0.0 {
        // The suit soaks it up instead, then vents.
        p.thermal += delta * dt;
        p.heat = (p.heat - 15.0 * dt).max(0.0);
        if p.thermal >= 100.0 {
            p.thermal = 0.0;
            super::entities::fire_burst(&mut bursts, p.body.center());
            run.pending_fire_burst = Some(p.body.center());
            sfx.write(Sfx::at("explosion_small", p.body.pos));
        }
    } else {
        p.heat = (p.heat + delta * dt).clamp(0.0, 100.0);
    }
    if p.heat >= 100.0 {
        damage!(10.0, DamageKind::Heat);
        p.heat_maxed_for += dt;
    } else if p.heat < 50.0 && p.heat_maxed_for >= 3.0 && p.hp > 0.0 {
        p.heat_maxed_for = 0.0;
        grant(&mut save, &mut run, AchievementId::HeatStroke, &mut sfx);
    }

    // Breath: underwater or in toxic gas.
    let underwater = head.kind() == Kind::Liquid;
    let in_gas = head == Material::Gas;
    if (underwater || in_gas) && !gills {
        p.breath = (p.breath - if in_gas { 25.0 } else { 10.0 } * dt).max(0.0);
        if in_gas {
            damage!(5.0, DamageKind::Gas);
        }
        if p.breath <= 0.0 {
            damage!(15.0, if in_gas { DamageKind::Gas } else { DamageKind::Drowned });
        }
        if underwater && run.rng.chance(12) {
            sfx.write(Sfx::at("drown_bubble", p.body.pos).volume(0.4));
            bursts.write(Burst::new(p.body.head(), Color::srgba(0.8, 0.9, 1.0, 0.7)).count(1).gravity(-50.0).speed(5.0));
        }
    } else {
        if p.breath < 40.0 && !underwater {
            sfx.write(Sfx::at("gasp", p.body.pos));
        }
        p.breath = (p.breath + 40.0 * dt).min(100.0);
    }
    if underwater {
        p.submerged_for += dt;
    } else {
        if p.submerged_for >= 6.0 && p.hp > 0.0 {
            grant(&mut save, &mut run, AchievementId::FloodSurvivor, &mut sfx);
        }
        p.submerged_for = 0.0;
    }

    if p.crushed {
        damage!(40.0, DamageKind::Crushed);
    }

    if p.hp > 0.0 && p.hp < p.max_hp * 0.25 && run.rng.chance(3) {
        sfx.write(Sfx::ui("heartbeat").volume(0.5));
    }
}
