//! The miner: movement, digging, ropes, torches and active items.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use sbct_sim::{Kind, Material, World};

use super::entities::{Bomb, Projectile, ProjectileKind, Sun};
use super::input::PlayerInput;
use super::items::{Active, ItemId};
use super::physics::{Body, SolidFn, default_solid, to_world};
use super::smart_dig;
use super::{Phase, Run, RunSetup};
use crate::assets::{GameAssets, player_anim};
use crate::audio::{Listener, Sfx};
use crate::fx::{Burst, Shake};
use crate::render::{InGameEntity, WorldCamera};
use crate::session::Session;

pub const HALF_WIDTH: f32 = 2.5;
pub const HEIGHT: f32 = 12.0;
const GRAVITY: f32 = 430.0;
const RUN_SPEED: f32 = 72.0;
const JUMP_SPEED: f32 = 170.0;
const MAX_FALL: f32 = 340.0;
const DASH_SPEED: f32 = 240.0;
const DASH_TIME: f32 = 0.14;
const DASH_COOLDOWN: f32 = 0.7;
const CLIMB_SPEED: f32 = 48.0;
const CLIMB_STAMINA: f32 = 0.9;
const ROPE_SPEED: f32 = 56.0;
/// How far from the player's centre the pick reaches, in cells.
pub const REACH: f32 = 22.0;
const DIG_INTERVAL: f32 = 0.065;
const DIG_RADIUS: i32 = 4;
const BASE_PICK_POWER: u8 = 42;

#[derive(Resource)]
pub struct RunPlayer {
    pub body: Body,
    pub hp: f32,
    pub max_hp: f32,
    /// Invulnerability after an impact.
    pub iframes: f32,
    /// Heat meter, 0–100. Damage at 100.
    pub heat: f32,
    /// Breath, 0–100. Drowning at 0.
    pub breath: f32,
    /// Seconds of burning left.
    pub burning: f32,
    pub stun: f32,
    pub crushed: bool,
    pub facing: f32,
    dash_time: f32,
    dash_cd: f32,
    dash_dir: f32,
    climb_stamina: f32,
    pub climbing: bool,
    pub on_rope: bool,
    coyote: f32,
    jump_buffer: f32,
    jump_cut: bool,
    dig_timer: f32,
    /// Recent dig direction for the animation.
    pub dig_anim: Option<(Vec2, f32)>,
    pub anim_time: f32,
    pub hurt_flash: f32,
    step_timer: f32,
    /// Achievement trackers.
    pub submerged_for: f32,
    pub heat_maxed_for: f32,
    pub was_burning: bool,
    /// Thermal Suit charge, 0–100.
    pub thermal: f32,
}

impl RunPlayer {
    pub fn solid_fn(run: &Run) -> SolidFn {
        if run.has(ItemId::HeavyBoots) {
            boots_solid
        } else {
            default_solid
        }
    }

    /// Where held items and projectiles come from.
    pub fn hand(&self) -> Vec2 {
        self.body.pos + Vec2::new(self.facing * 3.0, -7.0)
    }
}

/// Heavy Boots stand on molten metal but sink into sand.
fn boots_solid(m: Material) -> bool {
    m == Material::Metal || (m.is_solid_for_player() && m != Material::Sand)
}

#[derive(Component)]
pub struct PlayerSprite;

pub fn spawn_player(
    mut commands: Commands,
    setup: Res<RunSetup>,
    assets: Res<GameAssets>,
    session: Res<Session>,
    start_layer: Option<Res<crate::dev::StartLayer>>,
) {
    let core = setup
        .spawns
        .iter()
        .find(|s| s.kind == sbct_sim::descent::SpawnKind::Core);
    let start = match (start_layer, &session.world, core) {
        // `--layer 5`: right next to the core, for testing the ending.
        (Some(layer), _, Some(core)) if layer.0 == 5 => Vec2::new(core.x as f32 - 12.0, core.y as f32 + 1.0),
        (Some(layer), Some(world), _) => dev_start(world, layer.0).unwrap_or(setup.start),
        _ => setup.start,
    };
    commands.insert_resource(RunPlayer {
        body: Body::new(start, HALF_WIDTH, HEIGHT),
        hp: 100.0,
        max_hp: 100.0,
        iframes: 0.0,
        heat: 0.0,
        breath: 100.0,
        burning: 0.0,
        stun: 0.0,
        crushed: false,
        facing: 1.0,
        dash_time: 0.0,
        dash_cd: 0.0,
        dash_dir: 1.0,
        climb_stamina: CLIMB_STAMINA,
        climbing: false,
        on_rope: false,
        coyote: 0.0,
        jump_buffer: 0.0,
        jump_cut: false,
        dig_timer: 0.0,
        dig_anim: None,
        anim_time: 0.0,
        hurt_flash: 0.0,
        step_timer: 0.0,
        submerged_for: 0.0,
        heat_maxed_for: 0.0,
        was_burning: false,
        thermal: 0.0,
    });
    commands.spawn((
        PlayerSprite,
        InGameEntity,
        assets.player.sprite(0),
        Transform::from_translation(to_world(start - Vec2::Y * 8.0, 5.0)),
    ));
}

/// An open spot with a floor near the top of a layer (the `--layer` dev flag).
fn dev_start(world: &World, layer: usize) -> Option<Vec2> {
    let layer = sbct_sim::descent::Layer::ALL.get(layer)?;
    let clear = |x: i32, y: i32| {
        (0..14).all(|dy| (-3..=3).all(|dx| world.material(x + dx, y - dy) == Material::Empty))
            && (-3..=3).all(|dx| world.is_solid(x + dx, y + 1))
    };
    (layer.top() + 20..layer.bottom()).find_map(|y| {
        (40..world.width() as i32 - 40)
            .step_by(3)
            .find(|&x| clear(x, y))
            .map(|x| Vec2::new(x as f32 + 0.5, y as f32 + 1.0))
    })
}

fn on_rope(world: &World, body: &Body) -> bool {
    let x = body.pos.x.floor() as i32;
    let c = body.center();
    [c.y, c.y + 3.0, c.y - 3.0]
        .iter()
        .any(|&y| world.material(x, y.floor() as i32).props().climbable)
}

/// Footstep sound family for the material underfoot.
fn step_sound(m: Material, n: u32) -> &'static str {
    let even = n.is_multiple_of(2);
    match m {
        Material::Sand | Material::Gravel | Material::Ash => {
            if even {
                "step_gravel_1"
            } else {
                "step_gravel_2"
            }
        }
        Material::Ferrite | Material::Metal | Material::Explosive => {
            if even {
                "step_metal_1"
            } else {
                "step_metal_2"
            }
        }
        Material::Stone | Material::Basalt | Material::Obsidian | Material::Brick | Material::Crystal => {
            if even {
                "step_hard_1"
            } else {
                "step_hard_2"
            }
        }
        _ => {
            if even {
                "step_soft_1"
            } else {
                "step_soft_2"
            }
        }
    }
}

pub fn player_move(
    keys: Res<PlayerInput>,
    time: Res<Time>,
    mut player: ResMut<RunPlayer>,
    run: Res<Run>,
    mut session: ResMut<Session>,
    mut sfx: MessageWriter<Sfx>,
    mut bursts: MessageWriter<Burst>,
    mut shake: ResMut<Shake>,
    mut listener: ResMut<Listener>,
    paused: Res<super::hud::Paused>,
) {
    let Some(world) = session.world.as_mut() else {
        return;
    };
    let dt = time.delta_secs().min(1.0 / 30.0);
    if dt <= 0.0 {
        return;
    }
    let solid = RunPlayer::solid_fn(&run);
    let p = &mut *player;
    let alive = run.is_playing() && !paused.0;
    let ok = alive && p.stun <= 0.0;
    let (left, right, up, down) = (ok && keys.left, ok && keys.right, ok && keys.up, ok && keys.down);
    let (jump_pressed, dash_pressed) = (ok && keys.jump, ok && keys.dash);
    let dir = right as i32 - left as i32;

    p.stun = (p.stun - dt).max(0.0);
    p.dash_cd = (p.dash_cd - dt).max(0.0);
    p.coyote = if p.body.on_ground {
        0.1
    } else {
        (p.coyote - dt).max(0.0)
    };
    p.jump_buffer = if jump_pressed {
        0.12
    } else {
        (p.jump_buffer - dt).max(0.0)
    };
    if dir != 0 && p.dig_anim.is_none() {
        p.facing = dir as f32;
    }

    let submersion = p.body.submersion(world);
    let boots = run.has(ItemId::HeavyBoots);
    let swimming = submersion > 0.4 && !boots;
    p.on_rope = on_rope(world, &p.body);
    p.climbing = false;

    if dash_pressed && p.dash_cd <= 0.0 {
        p.dash_time = DASH_TIME;
        p.dash_cd = DASH_COOLDOWN;
        p.dash_dir = if dir != 0 { dir as f32 } else { p.facing };
        sfx.write(Sfx::at("dash", p.body.pos));
        bursts.write(
            Burst::new(p.body.center(), Color::srgba(0.9, 0.9, 1.0, 0.6))
                .count(10)
                .speed(30.0)
                .gravity(0.0)
                .dir(Vec2::new(-p.dash_dir, 0.0)),
        );
    }

    if p.dash_time > 0.0 {
        p.dash_time -= dt;
        p.body.vel = Vec2::new(p.dash_dir * DASH_SPEED, 0.0);
    } else if p.on_rope && (up || down || !p.body.on_ground) && !jump_pressed {
        p.climbing = true;
        p.body.vel.x = dir as f32 * 30.0;
        p.body.vel.y = (down as i32 - up as i32) as f32 * ROPE_SPEED;
    } else {
        let speed = if boots && submersion > 0.2 {
            28.0
        } else if swimming {
            42.0
        } else {
            RUN_SPEED
        };
        let target = dir as f32 * speed;
        let accel = if p.body.on_ground { 900.0 } else { 520.0 };
        let dv = (target - p.body.vel.x).clamp(-accel * dt, accel * dt);
        p.body.vel.x += dv;

        if swimming {
            p.body.vel.y += GRAVITY * 0.28 * dt;
            p.body.vel.y *= 1.0 - 4.0 * dt;
            if up {
                p.body.vel.y = p.body.vel.y.min(-60.0);
            } else if down {
                p.body.vel.y = p.body.vel.y.max(55.0);
            }
        } else if boots && submersion > 0.2 {
            p.body.vel.y = (p.body.vel.y + GRAVITY * 1.2 * dt).min(90.0);
        } else {
            p.body.vel.y = (p.body.vel.y + GRAVITY * dt).min(MAX_FALL);
        }

        // Wall climb (brief) and wall slide.
        let against_wall = !p.body.on_ground && p.body.wall != 0 && p.body.wall == dir;
        if against_wall && up && p.climb_stamina > 0.0 {
            p.body.vel.y = -CLIMB_SPEED;
            p.climb_stamina -= dt;
            p.climbing = true;
        } else if against_wall && p.body.vel.y > 50.0 {
            p.body.vel.y = 50.0;
        }

        if p.jump_buffer > 0.0 && !swimming {
            if p.coyote > 0.0 {
                p.body.vel.y = -JUMP_SPEED;
                p.jump_buffer = 0.0;
                p.coyote = 0.0;
                p.jump_cut = false;
                sfx.write(Sfx::at("jump", p.body.pos));
            } else if p.body.wall != 0 {
                p.body.vel = Vec2::new(-p.body.wall as f32 * 120.0, -JUMP_SPEED * 0.9);
                p.jump_buffer = 0.0;
                p.jump_cut = false;
                sfx.write(Sfx::at("jump", p.body.pos));
            }
        }
        // Short hop when the jump key is released early.
        if !up && !p.jump_cut && p.body.vel.y < -60.0 {
            p.body.vel.y *= 0.5;
            p.jump_cut = true;
        }
    }
    if p.on_rope && jump_pressed {
        p.body.vel.y = -JUMP_SPEED * 0.8;
        p.body.vel.x = dir as f32 * 90.0;
    }

    let step_up = if p.body.on_ground { 3 } else { 0 };
    let result = p.body.step(world, dt, step_up, solid);
    if p.body.on_ground {
        p.climb_stamina = CLIMB_STAMINA;
    }
    p.crushed = false;
    if !p.body.escape_overlap(world, solid) {
        // Buried: shove loose material out of the way rather than pinning
        // the player; only real weight (or solid rock) crushes.
        let mut cells = Vec::new();
        p.body.for_each_cell(|x, y| cells.push((x, y)));
        let (mut powder, mut hard) = (0, 0);
        let top = p.body.pos.y - p.body.height - 1.0;
        for (x, y) in cells {
            let m = world.material(x, y);
            if !solid(m) {
                continue;
            }
            if m.kind() == Kind::Powder {
                powder += 1;
                let side = if x as f32 >= p.body.pos.x { 1.0 } else { -1.0 };
                world.set(x, y, Material::Empty);
                world.spawn_particle(x as f32 + 0.5, top, side * 0.8, -1.2, m);
            } else {
                hard += 1;
            }
        }
        p.crushed = hard > 0 || powder > 10;
    }

    if let Some(speed) = result.landed
        && speed > 120.0
    {
        let under = world.material(p.body.pos.x as i32, p.body.pos.y as i32);
        sfx.write(Sfx::at("land", p.body.pos).volume((speed / 300.0).min(1.0)));
        let [r, g, b] = under.props().color;
        bursts.write(
            Burst::new(p.body.pos, Color::srgb_u8(r, g, b))
                .count(6 + (speed / 40.0) as u32)
                .speed(30.0)
                .dir(Vec2::NEG_Y)
                .life(0.4),
        );
        if speed > 260.0 && run.has(ItemId::SeismicStomp) {
            let (x, y) = (p.body.pos.x as i32, p.body.pos.y as i32);
            world.dig(x, y + 3, 7, 140);
            world.disturb(x, y, 24);
            shake.add(0.6);
            sfx.write(Sfx::at("explosion_small", p.body.pos));
        }
    }

    // Footsteps.
    if p.body.on_ground && p.body.vel.x.abs() > 10.0 {
        p.step_timer -= dt;
        if p.step_timer <= 0.0 {
            p.step_timer = 0.27;
            let under = world.material(p.body.pos.x as i32, p.body.pos.y as i32);
            let n = (p.body.pos.x as u32) / 4;
            sfx.write(Sfx::at(step_sound(under, n), p.body.pos).pitch(0.12));
        }
    } else if p.climbing {
        p.step_timer -= dt;
        if p.step_timer <= 0.0 {
            p.step_timer = 0.3;
            sfx.write(Sfx::at("climb", p.body.pos).volume(0.6));
        }
    }

    // Swimming refills the canister.
    if submersion > 0.3 {
        let _ = world;
    }
    listener.0 = p.body.center();
}

/// The cursor clamped to the pick's reach.
fn aim(player: &RunPlayer, cursor: Vec2) -> Vec2 {
    let c = player.body.center();
    let d = cursor - c;
    if d.length() > REACH {
        c + d.normalize() * REACH
    } else {
        cursor
    }
}

fn dig_sound(m: Material) -> &'static str {
    match m {
        Material::Crystal | Material::Ice | Material::Frost | Material::Obsidian => "dig_crystal",
        Material::Ferrite | Material::Explosive => "dig_metal",
        Material::Stone | Material::Basalt | Material::Brick | Material::Ore | Material::Vent => "dig_hard",
        _ => "dig_soft",
    }
}

pub fn player_dig(
    input: Res<PlayerInput>,
    time: Res<Time>,
    mut player: ResMut<RunPlayer>,
    mut run: ResMut<Run>,
    mut session: ResMut<Session>,
    mut sfx: MessageWriter<Sfx>,
    mut bursts: MessageWriter<Burst>,
    paused: Res<super::hud::Paused>,
) {
    let dt = time.delta_secs();
    if let Some((d, t)) = player.dig_anim {
        player.dig_anim = (t > 0.0).then_some((d, t - dt));
    }
    if !run.is_playing() || paused.0 || !input.dig || player.stun > 0.0 {
        return;
    }
    let Some(world) = session.world.as_mut() else {
        return;
    };
    let power = if run.has(ItemId::GlassCannonPick) {
        235
    } else {
        BASE_PICK_POWER
    };
    // Smart dig clears what blocks the body; cursor dig carves a disc at the aim.
    let (target, cells) = if let Some(dir) = input.dig_dir {
        let cells = smart_dig::targets(world, &player.body, dir);
        let Some(&(x, y)) = cells.first() else { return };
        (Vec2::new(x as f32 + 0.5, y as f32 + 0.5), cells)
    } else {
        let Some(cursor) = input.aim else { return };
        let t = aim(&player, cursor);
        (
            t,
            sbct_sim::world::disc(t.x.floor() as i32, t.y.floor() as i32, DIG_RADIUS).collect(),
        )
    };
    let dir = (target - player.body.center()).normalize_or_zero();
    player.dig_anim = Some((dir, 0.25));
    if dir.x.abs() > 0.2 {
        player.facing = dir.x.signum();
    }

    player.dig_timer -= dt;
    if player.dig_timer > 0.0 {
        return;
    }
    player.dig_timer = DIG_INTERVAL;
    let dug = world.dig_cells(&cells, power);
    let Some(&(_, _, first)) = dug.first() else { return };
    run.influence(target);

    for &(x, y, m) in &dug {
        run.stats.cells_dug += 1;
        if m.props().value > 0 {
            run.ore += m.props().value as u32;
            sfx.write(Sfx::at("pickup", target).volume(0.5).pitch(0.15));
            bursts.write(
                Burst::new(target, Color::srgb(1.0, 0.85, 0.3))
                    .count(2)
                    .speed(20.0)
                    .gravity(-20.0),
            );
            continue;
        }
        let rocky = matches!(
            m,
            Material::Stone | Material::Basalt | Material::Brick | Material::Obsidian | Material::Ferrite
        );
        if rocky && run.has(ItemId::CrumblingPick) && run.rng.chance(180) {
            let rubble = if run.rng.coin() {
                Material::Gravel
            } else {
                Material::Sand
            };
            world.set_with_life(x, y, rubble, 0);
        } else if rocky && run.has(ItemId::MagmaPick) && run.rng.chance(60) {
            world.set(x, y, Material::Lava);
        }
    }
    let [r, g, b] = first.props().color;
    bursts.write(
        Burst::new(target, Color::srgb_u8(r, g, b))
            .count(4 + dug.len().min(10) as u32)
            .speed(45.0)
            .dir(-dir)
            .life(0.5),
    );
    sfx.write(Sfx::at(dig_sound(first), target).pitch(0.15));
}

pub fn player_actions(
    mut commands: Commands,
    input: Res<PlayerInput>,
    time: Res<Time>,
    player: ResMut<RunPlayer>,
    mut run: ResMut<Run>,
    mut session: ResMut<Session>,
    mut sfx: MessageWriter<Sfx>,
    assets: Res<GameAssets>,
    paused: Res<super::hud::Paused>,
) {
    if !run.is_playing() || paused.0 || player.stun > 0.0 {
        return;
    }
    let Some(world) = session.world.as_mut() else {
        return;
    };
    let dt = time.delta_secs();

    // Cycle actives.
    let n = run.actives().len().max(1);
    if input.cycle > 0 {
        run.selected = (run.selected + 1) % n;
    }
    if input.cycle < 0 {
        run.selected = (run.selected + n - 1) % n;
    }

    // Swimming refills the water canister.
    if player.body.submersion(world) > 0.3 && run.has(ItemId::WaterCanister) {
        run.tank = (run.tank + 50.0 * dt).min(100.0);
    }

    let cursor = input.aim.unwrap_or(player.body.center());
    let hand = player.hand();
    let toward = (cursor - hand).normalize_or(Vec2::new(player.facing, 0.0));

    if input.rope && run.ropes > 0 {
        run.ropes -= 1;
        throw_rope(world, &player);
        sfx.write(Sfx::at("place_rope", player.body.pos));
    }

    if input.torch && run.torches > 0 {
        run.torches -= 1;
        commands.spawn(Projectile::bundle(
            ProjectileKind::Torch,
            hand,
            toward * 170.0 + Vec2::Y * -40.0,
            &assets,
        ));
        sfx.write(Sfx::at("throw", hand));
    }

    let Some(item) = run.selected_active() else { return };
    match item.def().active {
        Some(Active::Spray) => {
            if input.use_held && run.tank > 0.0 {
                let reach = hand + toward * 30.0;
                run.influence(reach);
                run.tank = (run.tank - 22.0 * dt).max(0.0);
                for i in 0..3 {
                    let spread = (run.rng.next_f32() - 0.5) * 0.5;
                    let v = Vec2::from_angle(toward.to_angle() + spread) * (3.0 + i as f32 * 0.4);
                    world.spawn_particle(hand.x, hand.y, v.x, v.y, Material::Water);
                }
                sfx.write(Sfx::at("water_spray", hand).volume(0.6));
            }
        }
        Some(Active::Charges { .. }) => {
            if !input.use_pressed {
                return;
            }
            let Some(entry) = run.charges.get_mut(&item) else {
                return;
            };
            if entry.0 == 0 {
                return;
            }
            entry.0 -= 1;
            let target = aim(&player, cursor);
            run.influence(target);
            match item {
                ItemId::AcidFlask => {
                    commands.spawn(Projectile::bundle(
                        ProjectileKind::AcidFlask,
                        hand,
                        toward * 180.0,
                        &assets,
                    ));
                    sfx.write(Sfx::at("throw", hand));
                }
                ItemId::FrostSeed => {
                    commands.spawn(Projectile::bundle(
                        ProjectileKind::FrostSeed,
                        hand,
                        toward * 170.0,
                        &assets,
                    ));
                    sfx.write(Sfx::at("throw", hand));
                }
                ItemId::FungalSpores => {
                    commands.spawn(Projectile::bundle(
                        ProjectileKind::Spores,
                        hand,
                        toward * 150.0,
                        &assets,
                    ));
                    sfx.write(Sfx::at("throw", hand));
                }
                ItemId::SparkRod => {
                    commands.spawn(Projectile::bundle(
                        ProjectileKind::SparkBolt,
                        hand,
                        toward * 360.0,
                        &assets,
                    ));
                    sfx.write(Sfx::at("spark", hand));
                }
                ItemId::BlastCharges => {
                    commands.spawn(Bomb::bundle(target, &assets));
                    sfx.write(Sfx::at("place_rope", target).pitch(0.3));
                }
                ItemId::PocketSun => {
                    commands.spawn(Sun::bundle(target, &assets));
                    sfx.write(Sfx::at("torch_ignite", target));
                }
                _ => {}
            }
        }
        None => {}
    }
}

/// A rope anchors to the ceiling above (or 48 cells up) and hangs to the floor.
fn throw_rope(world: &mut World, player: &RunPlayer) {
    let x = player.body.pos.x.floor() as i32;
    let head = (player.body.pos.y - HEIGHT) as i32;
    let mut top = head;
    for y in (head - 48..head).rev() {
        if !world.material(x, y).is_open() {
            break;
        }
        top = y;
    }
    let mut y = top;
    while y < top + 72 && world.material(x, y).is_open() {
        world.set(x, y, Material::Rope);
        y += 1;
    }
}

pub fn animate_player(
    time: Res<Time>,
    mut player: ResMut<RunPlayer>,
    run: Res<Run>,
    mut q: Query<(&mut Sprite, &mut Transform), With<PlayerSprite>>,
) {
    let Ok((mut sprite, mut tf)) = q.single_mut() else {
        return;
    };
    let dt = time.delta_secs();
    player.anim_time += dt;
    player.hurt_flash = (player.hurt_flash - dt).max(0.0);
    let p = &*player;

    let dead = matches!(run.phase, Phase::Dying(_)) || (run.phase == Phase::Over && !run.won);
    let (row, frames, fps, looping) = if dead {
        (player_anim::DEATH.0, player_anim::DEATH.1, 6.0, false)
    } else if p.hurt_flash > 0.25 {
        (player_anim::HURT.0, player_anim::HURT.1, 14.0, true)
    } else if let Some((d, _)) = p.dig_anim {
        let anim = if d.y > 0.6 {
            player_anim::DIG_DOWN
        } else if d.y < -0.6 {
            player_anim::DIG_UP
        } else {
            player_anim::DIG_SIDE
        };
        (anim.0, anim.1, 16.0, true)
    } else if p.climbing {
        let moving = p.body.vel.y.abs() > 5.0;
        (
            player_anim::CLIMB.0,
            player_anim::CLIMB.1,
            if moving { 10.0 } else { 0.0 },
            true,
        )
    } else if !p.body.on_ground && p.body.vel.y < 0.0 {
        (player_anim::JUMP.0, player_anim::JUMP.1, 10.0, false)
    } else if !p.body.on_ground {
        (player_anim::FALL.0, player_anim::FALL.1, 10.0, true)
    } else if p.body.vel.x.abs() > 8.0 {
        (player_anim::RUN.0, player_anim::RUN.1, 12.0, true)
    } else {
        (player_anim::IDLE.0, player_anim::IDLE.1, 6.0, true)
    };
    let t = if dead {
        run_time_since_death(&run)
    } else {
        p.anim_time
    };
    let mut frame = (t * fps) as usize;
    frame = if looping {
        frame % frames
    } else {
        frame.min(frames - 1)
    };
    if let Some(atlas) = &mut sprite.texture_atlas {
        atlas.index = row * player_anim::COLUMNS + frame;
    }
    sprite.flip_x = p.facing < 0.0;

    // Flicker while invulnerable, glow orange while burning.
    let flicker = p.iframes > 0.0 && (p.anim_time * 20.0) as i32 % 2 == 0;
    sprite.color = if p.burning > 0.0 {
        Color::srgb(1.0, 0.7, 0.5)
    } else if p.hurt_flash > 0.0 {
        Color::srgb(1.0, 0.5, 0.5)
    } else {
        Color::WHITE
    };
    sprite.color.set_alpha(if flicker { 0.4 } else { 1.0 });
    tf.translation = to_world(p.body.pos - Vec2::Y * 8.0, 5.0).round();
}

fn run_time_since_death(run: &Run) -> f32 {
    match run.phase {
        Phase::Dying(t) => t,
        _ => 10.0,
    }
}

/// Smooth follow with look-ahead in the direction of travel, plus shake.
pub fn follow_camera(
    time: Res<Time<Real>>,
    player: Res<RunPlayer>,
    session: Res<Session>,
    shake: Res<Shake>,
    mut camera: Single<(&mut Transform, &mut Projection), With<WorldCamera>>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut smoothed: Local<Option<Vec2>>,
) {
    let Some(world) = &session.world else { return };
    let (ref mut tf, ref mut projection) = *camera;
    // Whole-number zoom (crisp pixels) that's never narrower than the world.
    let pixels = (window.width() / world.width() as f32)
        .ceil()
        .max(crate::render::PIXEL_SCALE);
    if let Projection::Orthographic(o) = &mut **projection {
        o.scale = 1.0 / pixels;
    }
    let scale = 1.0 / pixels;
    let dt = time.delta_secs().min(0.1);
    let look = Vec2::new(
        (player.body.vel.x * 0.25).clamp(-40.0, 40.0),
        (player.body.vel.y * 0.3).clamp(-20.0, 60.0),
    );
    let target = player.body.center() + look;
    let current = smoothed.get_or_insert(target);
    *current = current.lerp(target, 1.0 - (-6.0 * dt).exp());

    let half = Vec2::new(window.width(), window.height()) * scale / 2.0;
    let (w, h) = (world.width() as f32, world.height() as f32);
    let clamp = |v: f32, half: f32, max: f32| {
        if half * 2.0 >= max {
            max / 2.0
        } else {
            v.clamp(half, max - half)
        }
    };
    let pos = Vec2::new(clamp(current.x, half.x, w), clamp(current.y, half.y, h))
        + shake.offset(time.elapsed_secs());
    tf.translation.x = pos.x;
    tf.translation.y = -pos.y;
}
