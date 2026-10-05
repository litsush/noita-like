//! The local player: movement, aiming, the multitool and the laser. The
//! client owns its own movement; everything it does to the world goes
//! through [`Session::act`].

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use sbct_sim::colony::actions::{Action as Act, InvOp};
use sbct_sim::colony::body::{Body, SolidFn, player_solid};
use sbct_sim::colony::geom::{V2, v2};
use sbct_sim::colony::{HOTBAR, PLAYER_HALF_W, PLAYER_HEIGHT, Pose, pose_flag};
use sbct_sim::{Kind, Material};

use crate::assets::player_anim;
use crate::audio::{Listener, Sfx};
use crate::controls::Action;
use crate::fx::{Burst, Shake};
use crate::render::{PIXEL_SCALE, UiHasPointer, WorldCamera};
use crate::session::Session;
use crate::settings::Config;

const GRAVITY: f32 = 560.0;
const RUN_SPEED: f32 = 80.0;
const JUMP_SPEED: f32 = 222.0;
const MAX_FALL: f32 = 360.0;
const STEP_UP: i32 = 4;
const FIRE_INTERVAL: f32 = 0.2;
const DIG_INTERVAL: f32 = 0.05;
const USE_INTERVAL: f32 = 0.09;

#[derive(Resource)]
pub struct LocalPlayer {
    pub body: Body,
    pub facing: i8,
    pub anim: u8,
    /// Aim angle in radians (0 = right, positive = down).
    pub aim: f32,
    /// World position under the cursor.
    pub cursor: V2,
    /// Where the multitool is digging this frame, if it is.
    pub digging: Option<V2>,
    /// Seconds the tool pose is still held after the last use.
    tool_time: f32,
    fire_wait: f32,
    dig_wait: f32,
    use_wait: f32,
    coyote: f32,
    jump_buffer: f32,
    step_distance: f32,
    step_count: u32,
    /// The host's warp counter last seen; a change means we were moved.
    warp: u32,
    was_swimming: bool,
    // ---- body mods ----
    /// Air jumps used since leaving the ground.
    air_jumps: u8,
    /// Seconds of rocket thrust left this jump.
    pub thrust_left: f32,
    /// Dash charges ready (fractions are recharging).
    pub dash_charges: f32,
    dash_time: f32,
    dash_dir: f32,
    /// How long Down has been held on the ground (Kangaroo Court).
    jump_charge: f32,
    /// Mid ground-pound.
    pounding: bool,
    knee_wait: f32,
    knee_flip: bool,
    /// Where the grapple hook is set, while the line is out.
    pub grapple: Option<V2>,
    /// How long the fire button has been held (Overcharge).
    pub fire_held: f32,
    /// Pose flags from movement mods this frame.
    mod_flags: u8,
    /// Camera offset while scouting with Zoom Goggles.
    pub scout: Vec2,
}

pub fn spawn_local_player(mut commands: Commands, session: Res<Session>) {
    let (pos, warp) = session.player().map_or((V2::ZERO, 0), |p| (p.pose.pos, p.warp));
    commands.insert_resource(LocalPlayer {
        body: Body::new(pos, PLAYER_HALF_W, PLAYER_HEIGHT),
        facing: 1,
        anim: player_anim::IDLE,
        aim: 0.0,
        cursor: pos,
        digging: None,
        tool_time: 0.0,
        fire_wait: 0.0,
        dig_wait: 0.0,
        use_wait: 0.0,
        coyote: 0.0,
        jump_buffer: 0.0,
        step_distance: 0.0,
        step_count: 0,
        warp,
        was_swimming: false,
        air_jumps: 0,
        thrust_left: 0.0,
        dash_charges: 0.0,
        dash_time: 0.0,
        dash_dir: 1.0,
        jump_charge: 0.0,
        pounding: false,
        knee_wait: 0.0,
        knee_flip: false,
        grapple: None,
        fire_held: 0.0,
        mod_flags: 0,
        scout: Vec2::ZERO,
    });
}

/// Where the hand holding the multitool is.
pub fn hand_pos(pos: V2, facing: i8, aim: f32) -> V2 {
    let shoulder = v2(pos.x + facing as f32 * 2.0, pos.y - 14.0);
    v2(shoulder.x + aim.cos() * 8.0, shoulder.y + aim.sin() * 8.0)
}

fn step_sound(m: Material, n: u32) -> &'static str {
    let even = n.is_multiple_of(2);
    match m {
        Material::Plating | Material::Ruin | Material::Glass => {
            if even {
                "step_metal_1"
            } else {
                "step_metal_2"
            }
        }
        Material::Stone
        | Material::Basalt
        | Material::Obsidian
        | Material::Crystal
        | Material::Ice
        | Material::Bedrock => {
            if even {
                "step_hard_1"
            } else {
                "step_hard_2"
            }
        }
        m if m.is_ore() => {
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

pub fn move_player(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    config: Res<Config>,
    time: Res<Time>,
    mut session: ResMut<Session>,
    mut player: ResMut<LocalPlayer>,
    mut listener: ResMut<Listener>,
    mut sfx: MessageWriter<Sfx>,
    mut bursts: MessageWriter<Burst>,
    ui: Res<UiHasPointer>,
    panels: Res<crate::panels::Panels>,
) {
    let dt = time.delta_secs().min(1.0 / 30.0);
    let p = &mut *player;
    let Some(me) = session.player() else { return };
    let (alive, slowed, warp, pos) = (me.alive(), me.slowed > 0.0, me.warp, me.pose.pos);
    let sleeping = me.sleeping;
    let stats = me.stats();
    let slowed = slowed && !stats.web_immune;
    let raining = session
        .colony
        .as_ref()
        .is_some_and(|c| c.weather.kind == sbct_sim::colony::WeatherKind::Rain);
    let mut acts: Vec<Act> = Vec::new();
    // The host moved us (respawn, recall).
    if warp != p.warp {
        p.warp = warp;
        p.body.pos = pos;
        p.body.vel = V2::ZERO;
    }
    let Some(world) = &session.world else { return };
    let b = &config.bindings;
    let typing = panels.blocks_movement();
    let held = |a: Action| alive && !typing && b.pressed(a, &keys, &mouse);
    let pressed = |a: Action| alive && !typing && b.just_pressed(a, &keys, &mouse);

    // If terrain appeared inside us (sand fell, a dome was built), get out.
    if !p.body.escape_overlap(world, player_solid) {
        p.body.pos.y -= 1.0;
    }

    let swimming = p.body.submersion(world) > 0.45;
    let dir = held(Action::MoveRight) as i32 - held(Action::MoveLeft) as i32;
    let feet = Vec2::new(p.body.pos.x, p.body.pos.y);
    p.mod_flags = 0;

    // Late For Work: at a run, water is as good as ground.
    let water_run = stats.water_run && dir != 0 && !held(Action::Down) && p.body.submersion(world) <= 0.0;
    let solid: SolidFn = if water_run { solid_or_water } else { player_solid };

    let storm = if raining && stats.rain_boost { 1.25 } else { 1.0 };
    let speed = RUN_SPEED
        * stats.speed
        * storm
        * if swimming { 0.6 * stats.swim } else { 1.0 }
        * if slowed { 0.45 } else { 1.0 };
    let target = dir as f32 * speed;
    let accel = if p.body.on_ground { 900.0 } else { 560.0 } * stats.speed.max(1.0);
    p.body.vel.x += (target - p.body.vel.x).clamp(-accel * dt, accel * dt);
    if dir != 0 && p.tool_time <= 0.0 {
        p.facing = dir as i8;
    }

    if p.body.on_ground || swimming {
        p.air_jumps = 0;
        p.thrust_left = 0.0;
    }
    p.coyote = if p.body.on_ground {
        0.1
    } else {
        (p.coyote - dt).max(0.0)
    };
    p.jump_buffer = if pressed(Action::Jump) {
        0.12
    } else {
        (p.jump_buffer - dt).max(0.0)
    };

    // Sticky Soles: which side has a wall to hold on to.
    let wall_side = if stats.wall > 0 && !p.body.on_ground && !swimming {
        [-1.0f32, 1.0].into_iter().find(|side| {
            p.body.collides(
                world,
                v2(p.body.pos.x + side * 1.5, p.body.pos.y - 4.0),
                player_solid,
            )
        })
    } else {
        None
    };
    let clinging = wall_side.is_some_and(|side| stats.wall >= 3 || dir as f32 == side);

    let gravity = GRAVITY * stats.gravity;
    let mut gliding = false;
    if swimming {
        p.pounding = false;
        p.body.vel.y += gravity * 0.25 * dt;
        p.body.vel.y *= 1.0 - (3.5 * dt).min(1.0);
        if held(Action::Jump) {
            p.body.vel.y = p.body.vel.y.min(-64.0 * stats.swim);
        } else if held(Action::Down) {
            p.body.vel.y = p.body.vel.y.max(58.0 * stats.swim);
        }
    } else {
        let max_fall = if p.pounding { MAX_FALL * 1.7 } else { MAX_FALL };
        p.body.vel.y = (p.body.vel.y + gravity * dt).min(max_fall);

        // Kangaroo Court: crouch to wind up a much higher jump.
        if stats.charge_jump && p.body.on_ground && held(Action::Down) {
            p.jump_charge = (p.jump_charge + dt).min(0.8);
        } else if !p.body.on_ground || !stats.charge_jump {
            p.jump_charge = 0.0;
        }

        if p.jump_buffer > 0.0 && p.coyote > 0.0 {
            let wound = if p.jump_charge >= 0.5 { 1.75 } else { 1.0 };
            p.body.vel.y = -JUMP_SPEED * stats.jump * wound * if slowed { 0.75 } else { 1.0 };
            p.jump_buffer = 0.0;
            p.coyote = 0.0;
            p.jump_charge = 0.0;
            sfx.write(Sfx::at("jump", feet).pitch(if wound > 1.0 { -0.2 } else { 0.0 }));
        } else if let Some(side) = wall_side.filter(|_| clinging) {
            // On a wall: slide, hang or climb, and kick off it.
            let (slide, climb) = match stats.wall {
                1 => (34.0, 0.0),
                2 => (18.0, 46.0),
                3 => (0.0, 84.0),
                _ => (0.0, speed.max(RUN_SPEED)),
            };
            p.body.vel.y = p.body.vel.y.min(slide);
            if climb > 0.0 && held(Action::Jump) && dir as f32 != -side {
                p.body.vel.y = -climb;
            } else if climb > 0.0 && held(Action::Down) {
                p.body.vel.y = climb;
            }
            let kick = if stats.wall == 1 {
                pressed(Action::Jump)
            } else {
                pressed(Action::Jump) && dir as f32 == -side
            };
            if kick {
                p.body.vel.x = -side * 130.0;
                p.body.vel.y = -JUMP_SPEED * 0.9 * stats.jump;
                p.jump_buffer = 0.0;
                sfx.write(Sfx::at("jump", feet));
            }
            p.air_jumps = 0;
        } else if pressed(Action::Jump) && p.coyote <= 0.0 && p.air_jumps < stats.air_jumps {
            // Rocket Feet: a second jump on a burst of flame.
            p.air_jumps += 1;
            p.jump_buffer = 0.0;
            p.body.vel.y = -JUMP_SPEED * 0.95 * stats.jump;
            p.thrust_left = stats.thrust_secs;
            p.pounding = false;
            sfx.write(Sfx::at("rocket_burst", feet));
            bursts.write(
                Burst::new(feet, Color::srgb(1.0, 0.72, 0.25))
                    .count(18)
                    .speed(90.0)
                    .dir(Vec2::Y)
                    .gravity(60.0)
                    .life(0.45),
            );
            bursts.write(
                Burst::new(feet, Color::srgb(0.75, 0.75, 0.78))
                    .count(8)
                    .speed(40.0)
                    .life(0.7),
            );
        } else if held(Action::Jump)
            && !p.body.on_ground
            && stats.air_jumps > 0
            && p.air_jumps >= stats.air_jumps
            && p.thrust_left > 0.0
        {
            // Rocket Hover / Flight: keep the boots lit.
            p.thrust_left -= dt;
            p.body.vel.y = (p.body.vel.y - (gravity + 520.0) * dt).max(-150.0);
            p.mod_flags |= pose_flag::THRUST;
            p.pounding = false;
        } else if stats.glide > 0 && held(Action::Jump) && !p.body.on_ground && p.body.vel.y > 0.0 {
            // Glide Wings.
            gliding = true;
            let sink = match stats.glide {
                1 => 42.0,
                2 => 30.0,
                3 if raining => -34.0,
                3 => 26.0,
                _ => 0.0,
            };
            p.body.vel.y = if sink < 0.0 {
                (p.body.vel.y - 500.0 * dt).max(sink)
            } else {
                p.body.vel.y.min(sink)
            };
            if stats.glide >= 2 && dir != 0 {
                let fast = speed * 1.7;
                p.body.vel.x += (dir as f32 * fast - p.body.vel.x).clamp(-400.0 * dt, 400.0 * dt);
            }
            p.mod_flags |= pose_flag::GLIDE;
        }
        // Letting go early makes a shorter hop.
        if !held(Action::Jump) && p.body.vel.y < -90.0 && p.mod_flags & pose_flag::THRUST == 0 {
            p.body.vel.y += gravity * 1.4 * dt;
        }

        // Stompers III: slam straight down.
        if stats.stomp >= 3 && pressed(Action::Down) && !p.body.on_ground && p.coyote <= 0.0 {
            p.pounding = true;
            p.body.vel = v2(0.0, MAX_FALL * 1.2);
            sfx.write(Sfx::at("dash", feet).pitch(-0.3));
        }
        if p.pounding {
            p.body.vel.x = 0.0;
        }
    }

    // Spring Heels: bounce off a creature you come down on.
    if stats.head_stomp
        && p.body.vel.y > 90.0
        && let Some(colony) = &session.colony
    {
        let feet = p.body.pos;
        let hit = colony.ents.values().any(|e| match &e.kind {
            sbct_sim::colony::EntKind::Creature(c) => {
                let (hw, h) = c.kind.size();
                v2(e.pos.x, e.pos.y - h / 2.0).distance(feet) < 7.0 + hw.max(h / 2.0)
            }
            _ => false,
        });
        if hit {
            p.body.vel.y = -JUMP_SPEED * 0.85;
            p.air_jumps = 0;
            sfx.write(Sfx::at("jump", feet_v(feet)).pitch(0.3));
        }
    }

    // Dash Pistons.
    p.dash_charges = (p.dash_charges + dt / 1.1).min(stats.dashes as f32);
    if pressed(Action::Dash) && stats.dashes > 0 && p.dash_charges >= 1.0 && p.dash_time <= 0.0 {
        p.dash_charges -= 1.0;
        p.dash_time = 0.16;
        p.dash_dir = if dir != 0 { dir as f32 } else { p.facing as f32 };
        sfx.write(Sfx::at("dash", feet));
        bursts.write(
            Burst::new(feet - Vec2::Y * 10.0, Color::srgb(0.7, 0.95, 1.0))
                .count(10)
                .speed(50.0)
                .dir(Vec2::new(-p.dash_dir, 0.0))
                .life(0.3),
        );
        // Blink And You Miss Me: a thin wall ahead is no obstacle.
        if stats.dash_walls {
            let ahead = |d: f32| v2(p.body.pos.x + p.dash_dir * d, p.body.pos.y);
            let blocked = (4..=20)
                .step_by(4)
                .any(|d| p.body.collides(world, ahead(d as f32), player_solid));
            if blocked {
                let gap = (12..=44)
                    .step_by(4)
                    .map(|d| ahead(d as f32))
                    .find(|at| !p.body.collides(world, *at, player_solid));
                if let Some(at) = gap {
                    p.body.pos = at;
                }
            }
        }
    }
    if p.dash_time > 0.0 {
        p.dash_time -= dt;
        p.body.vel = v2(p.dash_dir * 330.0, 0.0);
        p.mod_flags |= pose_flag::DASH;
    }

    // Grapple Glove: fire a line at the cursor and reel in.
    if pressed(Action::Ability) && stats.grapple > 0.0 {
        let from = hand_pos(p.body.pos, p.facing, p.aim);
        let aim = (p.cursor - from).normalized();
        let mut d = 4.0;
        p.grapple = None;
        while d <= stats.grapple {
            let at = from + aim * d;
            let (x, y) = at.cell();
            if player_solid(world.material(x, y)) {
                p.grapple = Some(at);
                break;
            }
            d += 2.0;
        }
        sfx.write(Sfx::at("grapple_fire", feet));
        if let Some(at) = p.grapple {
            sfx.write(Sfx::at("grapple_hit", Vec2::new(at.x, at.y)));
            if stats.grapple_pull {
                acts.push(Act::Yank { at });
            }
        }
    }
    if let Some(hook) = p.grapple {
        let chest = v2(p.body.pos.x, p.body.pos.y - 12.0);
        let to = hook - chest;
        if !held(Action::Ability) || to.length() < 9.0 || stats.grapple <= 0.0 {
            p.grapple = None;
            if !stats.grapple_swing {
                p.body.vel = p.body.vel * 0.35;
            }
        } else {
            let pull = to.normalized() * 300.0;
            let k = (10.0 * dt).min(1.0);
            p.body.vel = p.body.vel.lerp(pull, k);
            p.air_jumps = 0;
        }
    }

    // Zoom Goggles' Eagle Eye: hold to send the camera off toward the cursor.
    if stats.scout_cam && held(Action::Scout) {
        let from = v2(p.body.pos.x, p.body.pos.y - 12.0) + v2(p.scout.x, p.scout.y);
        let to = p.cursor - from;
        if to.length() > 24.0 {
            let d = to.normalized() * (260.0 * dt);
            p.scout = (p.scout + Vec2::new(d.x, d.y)).clamp_length_max(520.0);
        }
    } else {
        p.scout = p.scout.lerp(Vec2::ZERO, (8.0 * dt).min(1.0));
    }

    // Jackhammer Knees: crouch to drill straight down.
    p.knee_wait = (p.knee_wait - dt).max(0.0);
    let kneeling = stats.knee_drill > 0
        && held(Action::Down)
        && !swimming
        && (p.body.on_ground || stats.knee_drill >= 3);
    if kneeling && p.knee_wait <= 0.0 {
        p.knee_wait = if stats.knee_drill >= 2 { 0.05 } else { 0.1 };
        p.knee_flip = !p.knee_flip;
        let deep = if stats.knee_drill >= 4 && p.knee_flip {
            8
        } else {
            3
        };
        let side = if p.knee_flip { 2 } else { -2 };
        acts.push(Act::Dig {
            x: p.body.pos.x as i32 + side,
            y: p.body.pos.y as i32 + deep,
        });
    }
    if !alive {
        p.body.vel.x = 0.0;
        p.grapple = None;
    }

    let step_up = if p.body.on_ground { STEP_UP } else { 0 };
    let result = p.body.step(world, dt, step_up, solid);
    if let Some(speed) = result.landed {
        // Stompers: a hard landing leaves a crater.
        if stats.stomp > 0 && (speed > 230.0 || p.pounding) {
            acts.push(Act::Stomp {
                speed: if p.pounding { speed.max(420.0) } else { speed },
            });
        }
        p.pounding = false;
    }
    if gliding && p.step_count.is_multiple_of(1) && (time.elapsed_secs() * 3.0).fract() < dt * 3.0 {
        sfx.write(Sfx::at("glide_loop", feet).volume(0.35));
    }
    if p.mod_flags & pose_flag::THRUST != 0 {
        if (time.elapsed_secs() * 5.0).fract() < dt * 5.0 {
            sfx.write(Sfx::at("rocket_loop", feet).volume(0.5));
        }
        bursts.write(
            Burst::new(
                feet,
                Color::srgb(1.0, 0.6 + 0.3 * (time.elapsed_secs() * 31.0).sin().abs(), 0.2),
            )
            .count(2)
            .speed(70.0)
            .dir(Vec2::Y)
            .gravity(30.0)
            .life(0.3),
        );
    }
    let here = Vec2::new(p.body.pos.x, p.body.pos.y);
    let under = world.material(p.body.pos.x as i32, p.body.pos.y as i32 + 1);
    if let Some(speed) = result.landed
        && speed > 150.0
    {
        sfx.write(Sfx::at("land", here).volume((speed / 300.0).min(1.0)));
        let [r, g, bl] = under.props().color;
        bursts.write(
            Burst::new(here, Color::srgb_u8(r, g, bl))
                .count(6)
                .speed(30.0)
                .life(0.3),
        );
    }
    if p.body.on_ground && p.body.vel.x.abs() > 10.0 {
        p.step_distance += p.body.vel.x.abs() * dt;
        if p.step_distance > 15.0 {
            p.step_distance = 0.0;
            p.step_count += 1;
            let wet = world
                .material(p.body.pos.x as i32, p.body.pos.y as i32 - 2)
                .kind()
                == Kind::Liquid;
            let name = if wet {
                if p.step_count.is_multiple_of(2) {
                    "step_wet_1"
                } else {
                    "step_wet_2"
                }
            } else {
                step_sound(under, p.step_count)
            };
            sfx.write(Sfx::at(name, here));
        }
    }
    if swimming && !p.was_swimming && p.body.vel.y > 60.0 {
        sfx.write(Sfx::at("splash", here));
        bursts.write(
            Burst::new(here - Vec2::Y * 8.0, Color::srgb(0.5, 0.75, 0.9))
                .count(14)
                .speed(60.0)
                .dir(Vec2::NEG_Y)
                .life(0.5),
        );
    }
    p.was_swimming = swimming;

    // Aim at the cursor.
    let shoulder = v2(p.body.pos.x, p.body.pos.y - 14.0);
    let to = p.cursor - shoulder;
    p.aim = to.y.atan2(to.x);
    p.tool_time = (p.tool_time - dt).max(0.0);
    if p.tool_time > 0.0 && !ui.0 {
        p.facing = if to.x >= 0.0 { 1 } else { -1 };
    }

    // Pick the animation.
    let up = -to.normalized().y;
    p.anim = if !alive {
        player_anim::BLACKOUT
    } else if p.tool_time > 0.0 {
        if up > 0.55 {
            player_anim::REACH_UP
        } else if up < -0.55 {
            player_anim::REACH_DOWN
        } else {
            player_anim::REACH
        }
    } else if swimming {
        player_anim::SWIM
    } else if !p.body.on_ground {
        if p.body.vel.y < 0.0 {
            player_anim::JUMP
        } else {
            player_anim::FALL
        }
    } else if p.body.vel.x.abs() > 8.0 {
        player_anim::RUN
    } else {
        player_anim::IDLE
    };

    let mut flags = p.mod_flags;
    if p.body.on_ground {
        flags |= pose_flag::ON_GROUND;
    }
    if swimming {
        flags |= pose_flag::SWIMMING;
    }
    if p.digging.is_some() {
        flags |= pose_flag::TOOL;
    }
    listener.0 = Vec2::new(p.body.pos.x, p.body.pos.y - PLAYER_HEIGHT / 2.0);
    if sleeping {
        p.anim = player_anim::SLEEP;
    }
    let pose = Pose {
        pos: p.body.pos,
        vel: p.body.vel,
        facing: p.facing,
        anim: p.anim,
        aim: p.aim,
        flags,
    };
    session.set_pose(pose);
    for act in acts {
        session.act(act);
    }
}

fn feet_v(p: V2) -> Vec2 {
    Vec2::new(p.x, p.y)
}

fn solid_or_water(m: Material) -> bool {
    player_solid(m) || m == Material::Water
}

/// Mouse position in cell coordinates.
fn cursor_world(window: &Window, camera: &Camera, camera_transform: &GlobalTransform) -> Option<V2> {
    let pos = window.cursor_position()?;
    let world = camera.viewport_to_world_2d(camera_transform, pos).ok()?;
    Some(v2(world.x, -world.y))
}

pub fn use_tools(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut wheel: MessageReader<MouseWheel>,
    config: Res<Config>,
    time: Res<Time>,
    ui: Res<UiHasPointer>,
    panels: Res<crate::panels::Panels>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<WorldCamera>>,
    mut player: ResMut<LocalPlayer>,
    mut session: ResMut<Session>,
    mut sfx: MessageWriter<Sfx>,
) {
    let dt = time.delta_secs();
    let (cam, cam_tf) = *camera;
    if let Some(c) = cursor_world(&window, cam, cam_tf) {
        player.cursor = c;
    }
    let p = &mut *player;
    p.fire_wait = (p.fire_wait - dt).max(0.0);
    p.dig_wait = (p.dig_wait - dt).max(0.0);
    p.use_wait = (p.use_wait - dt).max(0.0);
    p.digging = None;
    let Some(me) = session.player() else { return };
    if !me.alive() {
        return;
    }
    let stats = me.stats();
    let selected = me.selected as usize;
    let held_item = me.inv.slots.get(selected).copied().flatten().map(|s| s.item);
    let (battery, lockout) = (me.battery, me.lockout);
    let b = &config.bindings;
    let typing = panels.blocks_movement();

    // Hotbar selection: number keys, wheel, next/previous, and "put away".
    let mut select = None;
    if !typing {
        for a in Action::ALL {
            if let Some(slot) = a.hotbar_slot()
                && b.just_pressed(a, &keys, &mouse)
            {
                select = Some(slot as usize);
            }
        }
        let mut step = b.just_pressed(Action::NextItem, &keys, &mouse) as i32
            - b.just_pressed(Action::PrevItem, &keys, &mouse) as i32;
        if !ui.0 {
            for ev in wheel.read() {
                step -= ev.y.signum() as i32;
            }
        }
        if step != 0 {
            // The slot after the last is the bare multitool.
            select = Some((selected as i32 + step).rem_euclid(HOTBAR as i32 + 1) as usize);
        }
        if b.just_pressed(Action::Multitool, &keys, &mouse) {
            select = Some(HOTBAR);
        }
    }
    if let Some(slot) = select
        && slot != selected
    {
        session.act(Act::Inv(InvOp::Select(slot as u8)));
        sfx.write(Sfx::ui("inv_move"));
    }
    // Pipe in hand: both buttons drag out runs instead (see `pipes.rs`).
    if typing || ui.0 || held_item == Some(sbct_sim::colony::items::Item::Pipe) {
        return;
    }
    let center = v2(p.body.pos.x, p.body.pos.y - PLAYER_HEIGHT / 2.0);
    let hand = hand_pos(p.body.pos, p.facing, p.aim);

    if b.just_pressed(Action::DropItem, &keys, &mouse) && held_item.is_some() {
        session.act(Act::Inv(InvOp::Drop {
            slot: selected as u16,
            count: if keys.pressed(KeyCode::ControlLeft) {
                u32::MAX
            } else {
                1
            },
        }));
        sfx.write(Sfx::ui("drop"));
    }

    // Laser. With Overcharge, holding the button charges a bigger shot
    // that goes off on release; a tap fires at once.
    let firing = b.pressed(Action::Fire, &keys, &mouse);
    let cost = sbct_sim::colony::BOLT_COST * stats.bolt_cost;
    let mut shoot = None;
    if stats.overcharge {
        if firing {
            if p.fire_held == 0.0 && p.fire_wait <= 0.0 {
                shoot = Some(0.0);
                p.fire_held = 0.001;
            } else if p.fire_held > 0.0 {
                p.fire_held += dt;
                p.tool_time = 0.3;
                if p.fire_held > 0.3 && p.fire_held - dt <= 0.3 {
                    sfx.write(Sfx::ui("laser_charge"));
                }
            }
        } else {
            if p.fire_held > 0.45 {
                shoot = Some(((p.fire_held - 0.3) / 0.9).clamp(0.0, 1.0));
            }
            p.fire_held = 0.0;
        }
    } else if firing && p.fire_wait <= 0.0 {
        shoot = Some(0.0);
    }
    if let Some(charge) = shoot {
        p.fire_wait = FIRE_INTERVAL;
        p.tool_time = 0.3;
        if lockout || battery < cost {
            sfx.write(Sfx::ui("laser_empty"));
            p.fire_wait = 0.5;
        } else {
            let dir = (p.cursor - hand).normalized();
            if charge > 0.3 {
                sfx.write(Sfx::ui("laser_big"));
            }
            session.act(Act::Fire {
                from: hand,
                dir,
                charge,
            });
        }
    }

    // One-key abilities.
    if b.just_pressed(Action::Ping, &keys, &mouse) && stats.beacon > 0 {
        session.act(Act::Pin(sbct_sim::colony::modfx::PinOp::Ping { pos: center }));
    }
    if b.just_pressed(Action::RainDance, &keys, &mouse) && stats.rain_dance {
        session.act(Act::RainDance);
    }

    // Primary: use the held item if it does something, otherwise dig.
    if b.pressed(Action::Primary, &keys, &mouse) {
        // Clamp the target to reach.
        let to = p.cursor - center;
        let target = if to.length() > stats.reach {
            center + to.normalized() * stats.reach
        } else {
            p.cursor
        };
        let place_at = if to.length() > stats.place_reach {
            center + to.normalized() * stats.place_reach
        } else {
            p.cursor
        };
        p.tool_time = 0.25;
        use sbct_sim::colony::items::Item;
        let block = held_item.is_some_and(|i| i.block().is_some());
        // Things that are placed or applied once per click.
        let once = matches!(
            held_item,
            Some(
                Item::DomeKit(_)
                    | Item::Machine(_)
                    | Item::Seed(_)
                    | Item::WateringCan
                    | Item::Cluckbug
                    | Item::MilkGrub
                    | Item::FishFry
                    | Item::RobotKit
            )
        );
        if block {
            if p.use_wait <= 0.0 {
                p.use_wait = USE_INTERVAL;
                session.act(Act::Use { at: place_at });
            }
        } else if once {
            if b.just_pressed(Action::Primary, &keys, &mouse) {
                // Kits and machines go where the preview shows, even past arm's reach of the cursor clamp.
                let at = if matches!(held_item, Some(Item::DomeKit(_) | Item::Machine(_))) {
                    p.cursor
                } else {
                    target
                };
                session.act(Act::Use { at });
            }
        } else {
            p.digging = Some(target);
            if p.dig_wait <= 0.0 {
                p.dig_wait = DIG_INTERVAL;
                let (x, y) = target.cell();
                session.act(Act::Dig { x, y });
            }
        }
    }
}

/// Smooth follow with look-ahead in the direction of travel, plus shake.
pub fn follow_camera(
    time: Res<Time<Real>>,
    player: Res<LocalPlayer>,
    session: Res<Session>,
    shake: Res<Shake>,
    mut camera: Single<(&mut Transform, &mut Projection), With<WorldCamera>>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut smoothed: Local<Option<Vec2>>,
) {
    let Some(world) = &session.world else { return };
    let (ref mut tf, ref mut projection) = *camera;
    // Whole-number zoom (crisp pixels): bigger windows see more, up to a point.
    let pixels = (window.height() / 300.0)
        .floor()
        .clamp(2.0, 6.0)
        .max(PIXEL_SCALE.min(3.0));
    let stats = session.player().map(|p| p.stats()).unwrap_or_default();
    // Zoom Goggles show more of the world.
    let pixels = pixels / stats.zoom;
    if let Projection::Orthographic(o) = &mut **projection {
        o.scale = 1.0 / pixels;
    }
    let scale = 1.0 / pixels;
    let dt = time.delta_secs().min(0.1);
    let b = &player.body;
    let look = Vec2::new(
        (b.vel.x * 0.25).clamp(-40.0, 40.0),
        (b.vel.y * 0.2).clamp(-20.0, 50.0),
    );
    let target = Vec2::new(b.pos.x, b.pos.y - PLAYER_HEIGHT * 0.8) + look + player.scout;
    let current = smoothed.get_or_insert(target);
    // Snap after a warp instead of gliding across the world.
    if current.distance(target) > 400.0 {
        *current = target;
    }
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
    let pos = Vec2::new(clamp(current.x, half.x, w), clamp(current.y, half.y, h)) + shake.offset(t);
    // Whole screen pixels, so sprites and cells don't shimmer.
    tf.translation.x = (pos.x * pixels).round() / pixels;
    tf.translation.y = -(pos.y * pixels).round() / pixels;
}
