//! The apprentice: movement, the dig spell, light orbs and spells.

use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use sbct_sim::{Kind, Material, World};

use super::entities::{LightOrb, ShardPickup};
use super::input::PlayerInput;
use super::physics::{Body, SolidFn, default_solid, to_world};
use super::scrolls::{School, ScrollId};
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
    /// The dig beam this frame: staff tip to target.
    pub beam: Option<(Vec2, Vec2)>,
    /// Seconds of the casting animation left.
    pub cast_anim: f32,
    pub levitating: bool,
    /// Height where the current fall began, and the length of the last one.
    pub fall_start: Option<f32>,
    pub last_fall: f32,
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
    pub fn new(start: Vec2) -> RunPlayer {
        RunPlayer {
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
            beam: None,
            cast_anim: 0.0,
            levitating: false,
            fall_start: None,
            last_fall: 0.0,
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
        }
    }

    pub fn solid_fn(run: &Run) -> SolidFn {
        if run.has(ScrollId::IronSoles) {
            boots_solid
        } else {
            default_solid
        }
    }

    /// The staff's glowing tip when aiming in `dir`.
    pub fn staff_tip(&self, dir: Vec2) -> Vec2 {
        self.body.center() + Vec2::new(0.0, -2.0) + dir.normalize_or_zero() * 7.0
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
pub struct PlayerSprite {
    /// 0 = robe (first school's colour), 1 = trim (second school's), 2 = base.
    layer: u8,
}

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
    commands.insert_resource(RunPlayer::new(start));
    for (layer, sheet, z) in [
        (0, &assets.wizard_robe, 5.0),
        (1, &assets.wizard_trim, 5.01),
        (2, &assets.player, 5.02),
    ] {
        commands.spawn((
            PlayerSprite { layer },
            InGameEntity,
            sheet.sprite(0),
            Transform::from_translation(to_world(start - Vec2::Y * 8.0, z)),
        ));
    }
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
    mut run: ResMut<Run>,
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
    let boots = run.has(ScrollId::IronSoles);
    let undertow = run.has(ScrollId::Undertow);
    let swimming = submersion > 0.4 && !boots;
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
    } else {
        let speed = if boots && submersion > 0.2 {
            28.0
        } else if swimming {
            if undertow { 80.0 } else { 42.0 }
        } else {
            RUN_SPEED
        };
        let target = dir as f32 * speed;
        let accel = if p.body.on_ground { 900.0 } else { 520.0 };
        let dv = (target - p.body.vel.x).clamp(-accel * dt, accel * dt);
        p.body.vel.x += dv;

        if swimming && undertow {
            // Water never drags you down: you steer freely.
            p.body.vel.y *= 1.0 - 5.0 * dt;
            p.body.vel.y += (down as i32 - up as i32) as f32 * 600.0 * dt;
            p.body.vel.y = p.body.vel.y.clamp(-90.0, 90.0);
        } else if swimming {
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
        // Earthen Grip: earth and stone hold you without tiring.
        let earthen = run.has(ScrollId::EarthenGrip) && {
            let wx = (p.body.pos.x + p.body.wall as f32 * (p.body.half_w + 1.0)) as i32;
            let m = world.material(wx, (p.body.pos.y - 4.0) as i32);
            matches!(
                m,
                Material::Dirt
                    | Material::Stone
                    | Material::Basalt
                    | Material::Gravel
                    | Material::Sand
                    | Material::Grass
            )
        };
        if against_wall && up && (p.climb_stamina > 0.0 || earthen) {
            p.body.vel.y = -CLIMB_SPEED;
            if !earthen {
                p.climb_stamina -= dt;
            }
            p.climbing = true;
        } else if against_wall && earthen {
            // Cling without sliding.
            p.body.vel.y = p.body.vel.y.clamp(-CLIMB_SPEED, 0.0);
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
                p.climb_stamina = (p.climb_stamina + CLIMB_STAMINA * 0.5).min(CLIMB_STAMINA);
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

    // Levitate: hold jump in the air to float, paid in mana.
    p.levitating = false;
    if run.has(ScrollId::Levitate)
        && !p.body.on_ground
        && up
        && p.dash_time <= 0.0
        && !swimming
        && run.mana > 1.0
    {
        p.body.vel.y = (p.body.vel.y - 900.0 * dt).max(-55.0);
        run.mana -= 14.0 * dt;
        // Rate-limited to its own length, so it loops while held.
        sfx.write(Sfx::at("levitate_loop", p.body.pos).volume(0.5));
        p.levitating = true;
    }

    let step_up = if p.body.on_ground { 3 } else { 0 };
    let result = p.body.step(world, dt, step_up, solid);
    if p.body.on_ground {
        p.climb_stamina = CLIMB_STAMINA;
    }
    // Frozen Path: liquids under your feet freeze as you walk.
    if run.has(ScrollId::FrozenPath) {
        for dx in -3..=3 {
            let (x, y) = (p.body.pos.x as i32 + dx, p.body.pos.y as i32);
            match world.material(x, y) {
                Material::Water | Material::Acid => world.set(x, y, Material::Ice),
                Material::Lava | Material::Metal => world.set(x, y, Material::Obsidian),
                _ => {}
            }
        }
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

    if p.body.on_ground {
        if let Some(start) = p.fall_start.take() {
            p.last_fall = p.body.pos.y - start;
        }
    } else if p.fall_start.is_none() || p.body.vel.y < 0.0 {
        // Measure from the top of a fall, not the start of a jump.
        if p.body.vel.y <= 0.0 {
            p.fall_start = Some(p.body.pos.y);
        }
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
        if speed > 260.0 && run.has(ScrollId::SeismicStomp) {
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
        Material::Crystal | Material::Ice | Material::Frost | Material::Obsidian | Material::ShardVein => {
            "dig_beam_crystal"
        }
        Material::Ferrite | Material::Explosive => "dig_beam_metal",
        Material::Stone | Material::Basalt | Material::Brick | Material::Vent => "dig_beam_hard",
        _ => "dig_beam_soft",
    }
}

/// The crackling beam from the staff to the dig target.
#[derive(Component)]
pub struct DigBeam;

pub fn draw_dig_beam(
    mut commands: Commands,
    player: Res<RunPlayer>,
    mut beam: Query<(&mut Transform, &mut Sprite, &mut Visibility), With<DigBeam>>,
    time: Res<Time<Real>>,
    mut bursts: MessageWriter<Burst>,
) {
    let Ok((mut tf, mut sprite, mut vis)) = beam.single_mut() else {
        commands.spawn((
            DigBeam,
            InGameEntity,
            Sprite::from_color(Color::WHITE, Vec2::ONE),
            Transform::default(),
            Visibility::Hidden,
        ));
        return;
    };
    let Some((from, to)) = player.beam else {
        *vis = Visibility::Hidden;
        return;
    };
    let d = to - from;
    let t = time.elapsed_secs();
    let flicker = 0.75 + 0.25 * (t * 40.0).sin().abs();
    *vis = Visibility::Visible;
    sprite.custom_size = Some(Vec2::new(d.length().max(1.0), 1.0 + flicker));
    sprite.color = Color::srgba(0.85, 0.75, 1.0, 0.85 * flicker);
    let mid = from + d / 2.0;
    tf.translation = Vec3::new(mid.x, -mid.y, 5.6);
    tf.rotation = Quat::from_rotation_z(-d.to_angle());
    if (t * 30.0).fract() < 0.5 {
        bursts.write(
            Burst::new(to, Color::srgb(0.8, 0.7, 1.0))
                .count(1)
                .speed(25.0)
                .gravity(0.0)
                .life(0.25),
        );
    }
}

pub fn player_dig(
    mut commands: Commands,
    assets: Res<GameAssets>,
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
    player.beam = None;
    if !run.is_playing() || paused.0 || !input.dig || player.stun > 0.0 {
        return;
    }
    let Some(world) = session.world.as_mut() else {
        return;
    };
    let variant = run.dig_variant();
    let power = if variant == Some(ScrollId::GlassFocus) {
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
    player.beam = Some((player.staff_tip(dir), target));
    if dir.x.abs() > 0.2 {
        player.facing = dir.x.signum();
    }

    player.dig_timer -= dt;
    if player.dig_timer > 0.0 {
        return;
    }
    player.dig_timer = DIG_INTERVAL;
    // Rot Touch eats organic matter instantly; Arc Drill bites through metal.
    let power = match variant {
        Some(ScrollId::RotTouch) => {
            for &(x, y) in &cells {
                if matches!(
                    world.material(x, y),
                    Material::Wood | Material::Fungus | Material::Grass | Material::Dirt
                ) {
                    world.set(x, y, Material::Empty);
                }
            }
            power
        }
        Some(ScrollId::ArcDrill) => {
            for &(x, y) in &cells {
                if world.material(x, y).props().conductive {
                    world.electrify(x, y, 12);
                }
            }
            if cells
                .iter()
                .any(|&(x, y)| matches!(world.material(x, y), Material::Ferrite))
            {
                160
            } else {
                power
            }
        }
        _ => power,
    };
    let mut rot_edges = Vec::new();
    let dug = world.dig_cells(&cells, power);
    let mut shards = Vec::new();
    let Some(&(_, _, first)) = dug.first() else { return };
    run.influence(target);

    for &(x, y, m) in &dug {
        run.stats.cells_dug += 1;
        if m.props().value > 0 {
            shards.push(Vec2::new(x as f32 + 0.5, y as f32 + 0.5));
            continue;
        }
        let rocky = matches!(
            m,
            Material::Stone | Material::Basalt | Material::Brick | Material::Obsidian | Material::Ferrite
        );
        match variant {
            Some(ScrollId::CrumblingTouch) if rocky && run.rng.chance(180) => {
                let rubble = if run.rng.coin() {
                    Material::Gravel
                } else {
                    Material::Sand
                };
                world.set_with_life(x, y, rubble, 0);
            }
            Some(ScrollId::MagmaBore) if rocky && run.rng.chance(60) => world.set(x, y, Material::Lava),
            Some(ScrollId::RotTouch)
                if matches!(
                    m,
                    Material::Wood | Material::Fungus | Material::Grass | Material::Dirt
                ) && run.rng.chance(40) =>
            {
                rot_edges.push((x, y));
            }
            _ => {}
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
    if !shards.is_empty() {
        sfx.write(Sfx::at("shard_vein", target).pitch(0.1));
    }
    // Rot leaves glowing fungus along the edges it eats.
    for (x, y) in rot_edges {
        for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
            if world.material(x + dx, y + dy).is_solid_for_player() && world.material(x, y).is_open() {
                world.set(x, y, Material::Fungus);
                break;
            }
        }
    }
    for at in shards {
        let v = Vec2::new(run.rng.next_f32() - 0.5, -run.rng.next_f32()) * 60.0;
        commands.spawn(ShardPickup::bundle(at, v, 1, &assets));
    }
}

/// Light orbs and the staff glow (spells are cast in `spells::cast_spells`).
pub fn player_actions(
    mut commands: Commands,
    input: Res<PlayerInput>,
    player: Res<RunPlayer>,
    mut run: ResMut<Run>,
    mut sfx: MessageWriter<Sfx>,
    assets: Res<GameAssets>,
    paused: Res<super::hud::Paused>,
) {
    if !run.is_playing() || paused.0 || player.stun > 0.0 {
        return;
    }
    let cursor = input.aim.unwrap_or(player.body.center());
    let hand = player.hand();
    let toward = (cursor - hand).normalize_or(Vec2::new(player.facing, 0.0));

    if input.toggle_light {
        run.staff_dimmed = !run.staff_dimmed;
        sfx.write(Sfx::ui("ui_toggle"));
    }

    if input.light_orb && run.light_charges > 0 {
        run.light_charges -= 1;
        commands.spawn(LightOrb::bundle(hand, toward * 130.0, &assets));
        sfx.write(Sfx::at("cast_light", hand));
    }
}

pub fn animate_player(
    time: Res<Time>,
    mut player: ResMut<RunPlayer>,
    run: Res<Run>,
    mut q: Query<(&PlayerSprite, &mut Sprite, &mut Transform)>,
) {
    let dt = time.delta_secs();
    player.anim_time += dt;
    player.hurt_flash = (player.hurt_flash - dt).max(0.0);
    if player.cast_anim > 0.0 {
        player.cast_anim -= dt;
    }
    let p = &*player;

    let dead = matches!(run.phase, Phase::Dying(_)) || (run.phase == Phase::Over && !run.won);
    let (row, frames, fps, looping) = if dead {
        (player_anim::DEATH.0, player_anim::DEATH.1, 6.0, false)
    } else if p.hurt_flash > 0.25 {
        (player_anim::HURT.0, player_anim::HURT.1, 14.0, true)
    } else if p.cast_anim > 0.0 {
        (player_anim::CAST.0, player_anim::CAST.1, 18.0, false)
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
    let t = if p.cast_anim > 0.0 && !dead {
        0.35 - p.cast_anim
    } else {
        t
    };
    let frame = if p.cast_anim > 0.0 && !dead {
        ((t * fps) as usize).min(frames - 1)
    } else {
        frame
    };

    // Flicker while invulnerable, glow orange while burning.
    let flicker = p.iframes > 0.0 && (p.anim_time * 20.0) as i32 % 2 == 0;
    let status = if p.burning > 0.0 {
        Some(Color::srgb(1.0, 0.7, 0.5))
    } else if p.hurt_flash > 0.0 {
        Some(Color::srgb(1.0, 0.5, 0.5))
    } else {
        None
    };
    let tint = |s: School| {
        let [r, g, b] = s.color();
        Color::srgb_u8(r, g, b)
    };
    let pos = to_world(p.body.pos - Vec2::Y * 8.0, 5.0).round();
    for (layer, mut sprite, mut tf) in &mut q {
        if let Some(atlas) = &mut sprite.texture_atlas {
            atlas.index = row * player_anim::COLUMNS + frame;
        }
        sprite.flip_x = p.facing < 0.0;
        let base = match layer.layer {
            0 => tint(run.first_school),
            // Until attuned, the trim is a pale echo of the first school.
            1 => run
                .second_school
                .map_or_else(|| tint(run.first_school).mix(&Color::WHITE, 0.6), tint),
            _ => Color::WHITE,
        };
        sprite.color = status.unwrap_or(base);
        sprite.color.set_alpha(if flicker { 0.4 } else { 1.0 });
        tf.translation = pos + Vec3::Z * layer.layer as f32 * 0.01;
    }
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
    stalker: Option<Res<super::creatures::StalkerState>>,
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
    let t = time.elapsed_secs();
    let near = stalker.map_or(0.0, |s| s.near);
    let wobble = Vec2::new((t * 1.7).sin(), (t * 2.3).cos()) * near * near * 2.5;
    let pos = Vec2::new(clamp(current.x, half.x, w), clamp(current.y, half.y, h)) + shake.offset(t) + wobble;
    tf.translation.x = pos.x;
    tf.translation.y = -pos.y;
}
