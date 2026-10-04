//! Smart dig (Tab): instead of digging at the cursor, dig whatever blocks
//! the player's collision box in the direction they're moving or aiming,
//! like Terraria's smart cursor. Holding dig plus a direction tunnels
//! smoothly without snagging on single-pixel lips.

use bevy::prelude::*;
use sbct_sim::material::INDESTRUCTIBLE;
use sbct_sim::{Kind, World};

use super::input::PlayerInput;
use super::physics::Body;
use super::player::RunPlayer;
use crate::session::Session;

/// How far ahead of the body smart dig clears, in cells.
pub const DEPTH: i32 = 3;

fn diggable(world: &World, x: i32, y: i32) -> bool {
    let m = world.material(x, y);
    matches!(m.kind(), Kind::Solid | Kind::Powder) && m.props().hardness != INDESTRUCTIBLE
}

/// Snaps a direction to -1, 0 or 1 per axis (diagonals allowed).
fn axes(dir: Vec2) -> (i32, i32) {
    let snap = |v: f32| {
        if v > 0.35 {
            1
        } else if v < -0.35 {
            -1
        } else {
            0
        }
    };
    let d = dir.normalize_or_zero();
    (snap(d.x), snap(d.y))
}

/// Cells smart dig would clear for `body` heading in `dir`, nearest first.
pub fn targets(world: &World, body: &Body, dir: Vec2) -> Vec<(i32, i32)> {
    let (x0, x1, y0, y1) = body.cells_at(body.pos);
    let (hx, vy) = axes(dir);
    let mut cells = Vec::new();
    let ahead_x = |hx: i32| {
        if hx > 0 {
            x1 + 1..=x1 + DEPTH
        } else {
            x0 - DEPTH..=x0 - 1
        }
    };
    let ahead_y = |vy: i32| {
        if vy > 0 {
            y1 + 1..=y1 + DEPTH
        } else {
            y0 - DEPTH..=y0 - 1
        }
    };
    if hx != 0 {
        // The full height of the body plus a row of headroom, so lips at the
        // feet and overhangs at head height are both cleared.
        for x in ahead_x(hx) {
            for y in y0 - 1..=y1 {
                cells.push((x, y));
            }
        }
    }
    if vy != 0 {
        for y in ahead_y(vy) {
            for x in x0 - 1..=x1 + 1 {
                cells.push((x, y));
            }
        }
    }
    if hx != 0 && vy != 0 {
        for x in ahead_x(hx) {
            for y in ahead_y(vy) {
                cells.push((x, y));
            }
        }
    }
    cells.sort_unstable();
    cells.dedup();
    cells.retain(|&(x, y)| diggable(world, x, y));
    let c = body.center();
    cells.sort_by(|a, b| {
        let da = Vec2::new(a.0 as f32 + 0.5, a.1 as f32 + 0.5).distance_squared(c);
        let db = Vec2::new(b.0 as f32 + 0.5, b.1 as f32 + 0.5).distance_squared(c);
        da.total_cmp(&db)
    });
    cells
}

/// Translucent fill over a targeted cell (drawn above the darkness overlay).
#[derive(Component)]
pub struct TargetFill;

/// Outlines and tints the cells smart dig will clear next. Both draw above
/// the darkness overlay, so the selection reads against any material, unlit.
pub fn highlight_targets(
    mut commands: Commands,
    input: Res<PlayerInput>,
    player: Res<RunPlayer>,
    session: Res<Session>,
    mut gizmos: Gizmos,
    time: Res<Time<Real>>,
    mut fills: Query<(&mut Transform, &mut Sprite, &mut Visibility), With<TargetFill>>,
    mut pool: Local<Vec<Entity>>,
) {
    let cells = match (input.dig_dir, &session.world) {
        (Some(dir), Some(world)) => targets(world, &player.body, dir),
        _ => Vec::new(),
    };
    let pulse = 0.5 + 0.5 * (time.elapsed_secs() * 6.0).sin().abs();
    let strength = if input.dig { 1.0 } else { 0.55 + 0.25 * pulse };

    while pool.len() < cells.len() {
        let e = commands
            .spawn((
                TargetFill,
                crate::render::InGameEntity,
                Sprite::from_color(Color::WHITE, Vec2::ONE),
                Transform::default(),
                Visibility::Hidden,
            ))
            .id();
        pool.push(e);
    }
    for (i, &e) in pool.iter().enumerate() {
        let Ok((mut tf, mut sprite, mut vis)) = fills.get_mut(e) else {
            continue;
        };
        match cells.get(i) {
            Some(&(x, y)) => {
                tf.translation = Vec3::new(x as f32 + 0.5, -(y as f32 + 0.5), 9.0);
                sprite.color = Color::srgba(1.0, 0.95, 0.7, 0.28 * strength);
                *vis = Visibility::Visible;
            }
            None => *vis = Visibility::Hidden,
        }
    }

    let set: std::collections::HashSet<(i32, i32)> = cells.iter().copied().collect();
    let color = Color::srgba(1.0, 1.0, 1.0, strength);
    // Only the outer edges of the selection, so it reads as one shape.
    for &(x, y) in &cells {
        let (fx, fy) = (x as f32, -(y as f32));
        let edges = [
            ((0, -1), Vec2::new(fx, fy), Vec2::new(fx + 1.0, fy)),
            ((0, 1), Vec2::new(fx, fy - 1.0), Vec2::new(fx + 1.0, fy - 1.0)),
            ((-1, 0), Vec2::new(fx, fy), Vec2::new(fx, fy - 1.0)),
            ((1, 0), Vec2::new(fx + 1.0, fy), Vec2::new(fx + 1.0, fy - 1.0)),
        ];
        for ((dx, dy), a, b) in edges {
            if !set.contains(&(x + dx, y + dy)) {
                gizmos.line_2d(a, b, color);
            }
        }
    }
}

/// Thicker gizmo lines for the smart-dig outline.
pub fn configure_gizmos(mut store: ResMut<GizmoConfigStore>) {
    let (config, _) = store.config_mut::<DefaultGizmoConfigGroup>();
    config.line.width = 3.0;
}

#[cfg(test)]
mod tests {
    use super::*;
    use sbct_sim::Material;

    fn world_with_wall() -> (World, Body) {
        let mut w = World::new(64, 64, 1);
        for x in 0..64 {
            for y in 40..64 {
                w.set(x, y, Material::Stone);
            }
        }
        // A wall to the right and a one-pixel lip at foot height.
        for y in 20..40 {
            w.set(40, y, Material::Dirt);
        }
        w.set(36, 39, Material::Sand);
        let body = Body::new(Vec2::new(32.5, 40.0), 2.5, 12.0);
        (w, body)
    }

    #[test]
    fn moving_right_clears_lips_and_walls_ahead() {
        let (w, body) = world_with_wall();
        let t = targets(&w, &body, Vec2::X);
        assert!(t.contains(&(36, 39)), "foot-height lip is targeted: {t:?}");
        assert!(t.iter().all(|&(x, _)| x > 34 && x <= 34 + DEPTH + 1));
        assert!(t.iter().all(|&(_, y)| (27..40).contains(&y)));
        assert!(!t.contains(&(31, 41)), "nothing behind or below");
    }

    #[test]
    fn holding_down_digs_straight_down() {
        let (w, body) = world_with_wall();
        let t = targets(&w, &body, Vec2::Y);
        assert!(!t.is_empty());
        assert!(
            t.iter()
                .all(|&(x, y)| (40..43).contains(&y) && (29..=36).contains(&x)),
            "{t:?}"
        );
        // Nearest first.
        let first = t[0];
        assert_eq!(first.1, 40);
    }

    #[test]
    fn indestructible_and_empty_cells_are_skipped() {
        let mut w = World::new(64, 64, 2);
        for x in 0..64 {
            w.set(x, 41, Material::CoreShell);
        }
        let body = Body::new(Vec2::new(32.5, 41.0), 2.5, 12.0);
        assert!(targets(&w, &body, Vec2::Y).is_empty());
        assert!(
            targets(&w, &body, Vec2::NEG_X).is_empty(),
            "open air needs no digging"
        );
    }
}
