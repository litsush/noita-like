//! Player intent for this frame, from the keyboard and mouse or from the
//! autoplay bot (`--autoplay`), which digs toward the core so whole runs
//! can be exercised and screenshotted without a human.

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use sbct_sim::Material;

use super::entities::{Prop, PropKind};
use super::player::RunPlayer;
use super::save::{DigMode, SaveData, store};
use super::{Phase, Run};
use crate::audio::Sfx;
use crate::controls::{Action, Binding};
use crate::render::{UiHasPointer, WorldCamera};
use crate::session::Session;

#[derive(Resource, Default, Clone, Debug)]
pub struct PlayerInput {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    pub jump: bool,
    pub dash: bool,
    /// Digging at `aim`.
    pub dig: bool,
    /// Using the selected active item at `aim` (held / just pressed).
    pub use_held: bool,
    pub use_pressed: bool,
    pub light_orb: bool,
    pub toggle_light: bool,
    pub interact: bool,
    /// Cycle actives: +1 / -1.
    pub cycle: i32,
    /// Where the player is pointing, in cells.
    pub aim: Option<Vec2>,
    /// Smart dig direction; `Some` means dig what blocks the body that way
    /// instead of digging at `aim`.
    pub dig_dir: Option<Vec2>,
}

/// Present when the bot is playing.
#[derive(Resource, Default)]
pub struct Autoplay {
    stuck_for: f32,
    last_pos: Vec2,
    wiggle: f32,
    timer: f32,
}

pub fn read_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut wheel: MessageReader<MouseWheel>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<WorldCamera>>,
    ui: Res<UiHasPointer>,
    mut input: ResMut<PlayerInput>,
    mut save: ResMut<SaveData>,
    player: Res<RunPlayer>,
    paused: Res<super::hud::Paused>,
    mut sfx: MessageWriter<Sfx>,
) {
    let scroll: f32 = wheel.read().map(|w| w.y).sum();
    if paused.0 {
        *input = PlayerInput::default();
        return;
    }
    let bindings = save.bindings.clone();
    let b = &bindings;
    let held = |a: Action| b.pressed(a, &keys, &mouse);
    let just = |a: Action| b.just_pressed(a, &keys, &mouse);
    let (cam, tf) = *camera;
    let aim = window
        .cursor_position()
        .and_then(|p| cam.viewport_to_world_2d(tf, p).ok())
        .map(|w| Vec2::new(w.x, -w.y));
    // Mouse buttons over UI panels belong to the UI.
    let mouse_ok = !ui.0;
    let gated = |a: Action, v: bool| {
        let on_mouse = b
            .slots(a)
            .iter()
            .flatten()
            .all(|s| matches!(s, Binding::Mouse(_)));
        v && (mouse_ok || !on_mouse)
    };

    if just(Action::ToggleDigMode) {
        save.settings.dig_mode = match save.settings.dig_mode {
            DigMode::Cursor => DigMode::Smart,
            DigMode::Smart => DigMode::Cursor,
        };
        store(&save);
        sfx.write(Sfx::ui("ui_click"));
    }

    let (left, right, up, down) = (
        held(Action::MoveLeft),
        held(Action::MoveRight),
        held(Action::Jump),
        held(Action::Down),
    );
    let dig_dir = (save.settings.dig_mode == DigMode::Smart).then(|| {
        // Movement keys take priority; otherwise dig toward the cursor.
        let keys_dir = Vec2::new(
            (right as i32 - left as i32) as f32,
            (down as i32 - up as i32) as f32,
        );
        if keys_dir != Vec2::ZERO {
            keys_dir
        } else {
            aim.map_or(Vec2::new(player.facing, 0.0), |a| a - player.body.center())
        }
    });

    *input = PlayerInput {
        left,
        right,
        up,
        down,
        jump: just(Action::Jump),
        dash: just(Action::Dash),
        dig: gated(Action::Dig, held(Action::Dig)),
        use_held: gated(Action::UseItem, held(Action::UseItem)),
        use_pressed: gated(Action::UseItem, just(Action::UseItem)),
        light_orb: just(Action::LightOrb),
        toggle_light: just(Action::ToggleStaffLight),
        interact: just(Action::Interact),
        cycle: if just(Action::NextItem) || scroll < 0.0 {
            1
        } else if just(Action::PrevItem) || scroll > 0.0 {
            -1
        } else {
            0
        },
        aim,
        dig_dir,
    };
}

/// A simple digger: head for the core's column, dig down, climb out of
/// trouble, grab anything nearby.
pub fn bot_input(
    mut bot: ResMut<Autoplay>,
    mut input: ResMut<PlayerInput>,
    player: Res<RunPlayer>,
    run: Res<Run>,
    session: Res<Session>,
    props: Query<&Prop>,
    time: Res<Time>,
) {
    let Some(world) = &session.world else { return };
    let dt = time.delta_secs();
    let p = &player.body;
    let c = p.center();
    let mut i = PlayerInput::default();
    bot.timer += dt;

    if run.phase != Phase::Playing {
        *input = i;
        return;
    }

    // Progress check: if we haven't moved, try something else.
    if p.pos.distance(bot.last_pos) < 0.5 {
        bot.stuck_for += dt;
    } else {
        bot.stuck_for = 0.0;
    }
    bot.last_pos = p.pos;
    if bot.stuck_for > 2.0 {
        bot.wiggle = 1.5;
        bot.stuck_for = 0.0;
    }
    bot.wiggle = (bot.wiggle - dt).max(0.0);

    let target_x = run.core.x;
    let dx = target_x - p.pos.x;
    let want = if dx.abs() > 30.0 { dx.signum() } else { 0.0 };
    let wiggle_dir = if (bot.timer * 0.7).sin() > 0.0 { 1.0 } else { -1.0 };
    let dir = if bot.wiggle > 0.0 { wiggle_dir } else { want };
    i.left = dir < 0.0;
    i.right = dir > 0.0;

    // Danger: liquid at head or burning -> go up and sideways.
    let head = world.material(p.head().x as i32, p.head().y as i32);
    let feet = world.material(p.pos.x as i32, p.pos.y as i32 - 1);
    let in_liquid = head.kind() == sbct_sim::Kind::Liquid || feet.kind() == sbct_sim::Kind::Liquid;
    // Surface for air; otherwise swim down to dig the pool's floor.
    let harmful =
        matches!(head, Material::Lava | Material::Acid) || matches!(feet, Material::Lava | Material::Acid);
    if player.breath < 60.0 || harmful {
        i.up = true;
    } else if in_liquid {
        i.down = true;
    }

    // Dig: below us mostly, toward the target when far off sideways.
    let below = Vec2::new(p.pos.x + dir * 2.0, p.pos.y + 2.0);
    let side = Vec2::new(c.x + dir * 8.0, c.y);
    let blocked_side = dir != 0.0 && p.wall != 0;
    let hazard_below = (1..14).any(|d| {
        matches!(
            world.material(c.x as i32, c.y as i32 + d),
            Material::Lava | Material::Acid | Material::Metal
        )
    });
    i.aim = Some(if blocked_side || hazard_below { side } else { below });
    // Smart dig: tunnel toward where we want to go.
    i.dig_dir = Some(if blocked_side || hazard_below {
        Vec2::new(dir, 0.0)
    } else {
        Vec2::new(dir * 0.3, 1.0)
    });
    i.dig = true;
    if blocked_side && p.on_ground {
        i.jump = (bot.timer * 3.0) as i32 % 2 == 0;
        i.up = true;
    }

    // Grab chests, altars and the core.
    let near = props.iter().any(|pr| {
        pr.center().distance(c) < 18.0
            && matches!(
                pr.kind,
                PropKind::Chest { opened: false, .. }
                    | PropKind::Remains { searched: false, .. }
                    | PropKind::Altar {
                        offer: super::entities::Offer::Scroll(_) | super::entities::Offer::Unrolled
                    }
                    | PropKind::Core
            )
    });
    i.interact = near && (bot.timer * 4.0) as i32 % 2 == 0;

    // Occasionally use whatever active is selected, and light the way.
    i.use_pressed = (bot.timer * 0.5).fract() < dt * 0.5;
    i.use_held = (bot.timer * 0.25).fract() < 0.2;
    i.cycle = if (bot.timer * 0.5 + 0.5).fract() < dt * 0.5 {
        1
    } else {
        0
    };
    i.light_orb = run.light_charges > 1 && (bot.timer * 0.05).fract() < dt * 0.05 && run.layer.index() > 0;
    *input = i;
}
