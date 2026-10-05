//! The local player: movement, aiming, the multitool and the laser. The
//! client owns its own movement; everything it does to the world goes
//! through [`Session::act`].

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use sbct_sim::colony::actions::{Action as Act, InvOp};
use sbct_sim::colony::body::{Body, player_solid};
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
    let speed = RUN_SPEED * if swimming { 0.6 } else { 1.0 } * if slowed { 0.45 } else { 1.0 };
    let target = dir as f32 * speed;
    let accel = if p.body.on_ground { 900.0 } else { 560.0 };
    p.body.vel.x += (target - p.body.vel.x).clamp(-accel * dt, accel * dt);
    if dir != 0 && p.tool_time <= 0.0 {
        p.facing = dir as i8;
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
    if swimming {
        p.body.vel.y += GRAVITY * 0.25 * dt;
        p.body.vel.y *= 1.0 - (3.5 * dt).min(1.0);
        if held(Action::Jump) {
            p.body.vel.y = p.body.vel.y.min(-64.0);
        } else if held(Action::Down) {
            p.body.vel.y = p.body.vel.y.max(58.0);
        }
    } else {
        p.body.vel.y = (p.body.vel.y + GRAVITY * dt).min(MAX_FALL);
        if p.jump_buffer > 0.0 && p.coyote > 0.0 {
            p.body.vel.y = -JUMP_SPEED * if slowed { 0.75 } else { 1.0 };
            p.jump_buffer = 0.0;
            p.coyote = 0.0;
            sfx.write(Sfx::at("jump", Vec2::new(p.body.pos.x, p.body.pos.y)));
        }
        // Letting go early makes a shorter hop.
        if !held(Action::Jump) && p.body.vel.y < -90.0 {
            p.body.vel.y += GRAVITY * 1.4 * dt;
        }
    }
    if !alive {
        p.body.vel.x = 0.0;
    }

    let step_up = if p.body.on_ground { STEP_UP } else { 0 };
    let result = p.body.step(world, dt, step_up, player_solid);
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

    let mut flags = 0;
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
    let pose = Pose {
        pos: p.body.pos,
        vel: p.body.vel,
        facing: p.facing,
        anim: p.anim,
        aim: p.aim,
        flags,
    };
    session.set_pose(pose);
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
    if typing || ui.0 {
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

    // Laser.
    if b.pressed(Action::Fire, &keys, &mouse) && p.fire_wait <= 0.0 {
        p.fire_wait = FIRE_INTERVAL;
        p.tool_time = 0.3;
        let cost = sbct_sim::colony::BOLT_COST * stats.bolt_cost;
        if lockout || battery < cost {
            sfx.write(Sfx::ui("laser_empty"));
            p.fire_wait = 0.5;
        } else {
            let dir = (p.cursor - hand).normalized();
            session.act(Act::Fire { from: hand, dir });
        }
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
        p.tool_time = 0.25;
        let usable = held_item.is_some_and(|i| i.block().is_some());
        if usable {
            if p.use_wait <= 0.0 {
                p.use_wait = USE_INTERVAL;
                session.act(Act::Use { at: target });
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
    let target = Vec2::new(b.pos.x, b.pos.y - PLAYER_HEIGHT * 0.8) + look;
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
