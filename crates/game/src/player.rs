//! Local player movement/tools and drawing of all players.

use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use sbct_net::PeerId;
use sbct_net::protocol::{Edit, PlayerState};
use sbct_sim::{Kind, Material, World};

use crate::render::{InGameEntity, UiHasPointer, WorldCamera};
use crate::session::Session;

pub const HALF_WIDTH: f32 = 3.0;
pub const HEIGHT: f32 = 12.0;
const GRAVITY: f32 = 420.0;
const MOVE_SPEED: f32 = 75.0;
const JUMP_SPEED: f32 = 170.0;
const MAX_FALL: f32 = 320.0;
const MAX_STEP_UP: i32 = 3;
const DIG_RADIUS: u8 = 5;
const PLACE_RADIUS: u8 = 3;
const EDIT_INTERVAL: f32 = 1.0 / 30.0;

#[derive(Resource)]
pub struct LocalPlayer {
    pub state: PlayerState,
    pub on_ground: bool,
    /// Index into `Material::PLACEABLE`.
    pub selected: usize,
    edit_timer: f32,
}

pub fn spawn_local_player(mut commands: Commands, session: Res<Session>) {
    commands.insert_resource(LocalPlayer {
        state: PlayerState {
            x: session.spawn.x,
            y: session.spawn.y,
            ..default()
        },
        on_ground: false,
        selected: 0,
        edit_timer: 0.0,
    });
}

fn collides(world: &World, x: f32, y: f32) -> bool {
    let x0 = (x - HALF_WIDTH).floor() as i32;
    let x1 = (x + HALF_WIDTH - 0.01).floor() as i32;
    let y0 = (y - HEIGHT).floor() as i32;
    let y1 = (y - 0.01).floor() as i32;
    (y0..=y1).any(|cy| (x0..=x1).any(|cx| world.is_solid(cx, cy)))
}

fn in_liquid(world: &World, x: f32, y: f32) -> bool {
    world.material(x as i32, (y - HEIGHT * 0.5) as i32).kind() == Kind::Liquid
}

pub fn move_player(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    session: Res<Session>,
    mut player: ResMut<LocalPlayer>,
) {
    let Some(world) = &session.world else { return };
    let dt = time.delta_secs().min(1.0 / 30.0);
    let p = &mut *player;
    let s = &mut p.state;

    // If terrain appeared inside us (sand fell, host update), climb out.
    if collides(world, s.x, s.y) {
        s.y -= 1.0;
        s.vy = 0.0;
        return;
    }

    let swimming = in_liquid(world, s.x, s.y);
    let dir = keys.pressed(KeyCode::KeyD) as i32 - keys.pressed(KeyCode::KeyA) as i32;
    s.vx = dir as f32 * MOVE_SPEED * if swimming { 0.6 } else { 1.0 };

    let jump = keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::Space);
    if swimming {
        s.vy += GRAVITY * 0.25 * dt;
        s.vy *= 0.92;
        if jump {
            s.vy = s.vy.min(-60.0);
        }
    } else {
        s.vy = (s.vy + GRAVITY * dt).min(MAX_FALL);
        if jump && p.on_ground {
            s.vy = -JUMP_SPEED;
        }
    }

    // Horizontal, one cell at a time, stepping up small ledges.
    let dx = s.vx * dt;
    let steps = dx.abs().ceil() as i32;
    for _ in 0..steps {
        let inc = dx / steps as f32;
        if !collides(world, s.x + inc, s.y) {
            s.x += inc;
            continue;
        }
        let climb = (1..=MAX_STEP_UP).find(|&up| !collides(world, s.x + inc, s.y - up as f32));
        match climb {
            Some(up) if p.on_ground || swimming => {
                s.x += inc;
                s.y -= up as f32;
            }
            _ => {
                s.vx = 0.0;
                break;
            }
        }
    }

    // Vertical.
    let dy = s.vy * dt;
    let steps = dy.abs().ceil() as i32;
    p.on_ground = false;
    for _ in 0..steps {
        let inc = dy / steps as f32;
        if collides(world, s.x, s.y + inc) {
            if inc > 0.0 {
                p.on_ground = true;
            }
            s.vy = 0.0;
            break;
        }
        s.y += inc;
    }
    if steps == 0 && collides(world, s.x, s.y + 1.0) {
        p.on_ground = true;
    }

    let (w, h) = (world.width() as f32, world.height() as f32);
    s.x = s.x.clamp(HALF_WIDTH, w - HALF_WIDTH);
    s.y = s.y.clamp(HEIGHT, h);
}

/// Mouse position in cell coordinates.
fn cursor_cell(window: &Window, camera: &Camera, camera_transform: &GlobalTransform) -> Option<IVec2> {
    let pos = window.cursor_position()?;
    let world = camera.viewport_to_world_2d(camera_transform, pos).ok()?;
    Some(IVec2::new(world.x.floor() as i32, (-world.y).floor() as i32))
}

pub fn use_tools(
    mouse: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut wheel: MessageReader<MouseWheel>,
    time: Res<Time>,
    ui: Res<UiHasPointer>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<WorldCamera>>,
    mut player: ResMut<LocalPlayer>,
    mut session: ResMut<Session>,
) {
    let n = Material::PLACEABLE.len();
    for (i, key) in [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
        KeyCode::Digit7,
        KeyCode::Digit8,
        KeyCode::Digit9,
    ]
    .into_iter()
    .enumerate()
    .take(n)
    {
        if keys.just_pressed(key) {
            player.selected = i;
        }
    }
    for ev in wheel.read() {
        if ev.y > 0.0 {
            player.selected = (player.selected + n - 1) % n;
        } else if ev.y < 0.0 {
            player.selected = (player.selected + 1) % n;
        }
    }

    player.edit_timer -= time.delta_secs();
    if ui.0 || player.edit_timer > 0.0 {
        return;
    }
    let (dig, place) = (
        mouse.pressed(MouseButton::Left),
        mouse.pressed(MouseButton::Right),
    );
    if !dig && !place {
        return;
    }
    let (cam, cam_tf) = *camera;
    let Some(cell) = cursor_cell(&window, cam, cam_tf) else {
        return;
    };
    player.edit_timer = EDIT_INTERVAL;
    let (radius, material) = if dig {
        (DIG_RADIUS, Material::Empty)
    } else {
        (PLACE_RADIUS, Material::PLACEABLE[player.selected])
    };
    session.edit(Edit {
        x: cell.x,
        y: cell.y,
        radius,
        material: material as u8,
    });
}

pub fn follow_camera(
    player: Res<LocalPlayer>,
    session: Res<Session>,
    mut camera: Single<(&mut Transform, &Projection), With<WorldCamera>>,
    window: Single<&Window, With<PrimaryWindow>>,
) {
    let Some(world) = &session.world else { return };
    let (ref mut tf, projection) = *camera;
    let scale = match projection {
        Projection::Orthographic(o) => o.scale,
        _ => 1.0,
    };
    let half = Vec2::new(window.width(), window.height()) * scale / 2.0;
    let (w, h) = (world.width() as f32, world.height() as f32);
    let target = Vec2::new(player.state.x, player.state.y - HEIGHT / 2.0);
    let x = if half.x * 2.0 >= w {
        w / 2.0
    } else {
        target.x.clamp(half.x, w - half.x)
    };
    let y = if half.y * 2.0 >= h {
        h / 2.0
    } else {
        target.y.clamp(half.y, h - half.y)
    };
    tf.translation.x = x;
    tf.translation.y = -y;
}

#[derive(Component)]
pub struct Avatar(pub PeerId);

fn player_color(id: PeerId) -> Color {
    let hue = (sbct_sim::rng::hash2(id, 1, 2) % 360) as f32;
    Color::hsl(hue, 0.7, 0.6)
}

fn to_world(state: &PlayerState) -> Vec3 {
    Vec3::new(state.x, -(state.y - HEIGHT / 2.0), 5.0)
}

/// Keeps one sprite per player (local + remote) in sync with their state.
pub fn sync_avatars(
    mut commands: Commands,
    session: Res<Session>,
    player: Res<LocalPlayer>,
    mut avatars: Query<(Entity, &Avatar, &mut Transform)>,
) {
    let mut wanted: Vec<(PeerId, PlayerState)> =
        session.players.iter().map(|(&id, p)| (id, p.state)).collect();
    wanted.push((session.local_id, player.state));

    for (entity, avatar, mut tf) in &mut avatars {
        match wanted.iter().position(|(id, _)| *id == avatar.0) {
            Some(i) => {
                let (_, state) = wanted.swap_remove(i);
                let target = to_world(&state);
                // Remote players arrive at 20 Hz; smooth them out.
                tf.translation = if avatar.0 == session.local_id {
                    target
                } else {
                    tf.translation.lerp(target, 0.35)
                };
            }
            None => commands.entity(entity).despawn(),
        }
    }
    for (id, state) in wanted {
        commands.spawn((
            Avatar(id),
            InGameEntity,
            Sprite::from_color(player_color(id), Vec2::new(HALF_WIDTH * 2.0, HEIGHT)),
            Transform::from_translation(to_world(&state)),
        ));
    }
}
