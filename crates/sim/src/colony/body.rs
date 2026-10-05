//! Axis-aligned box movement against the cell grid, shared by players,
//! creatures and drops. A body's position is the centre of its feet.

use serde::{Deserialize, Serialize};

use super::geom::{V2, v2};
use crate::material::{Kind, Material};
use crate::world::World;

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Body {
    pub pos: V2,
    pub vel: V2,
    pub half_w: f32,
    pub height: f32,
    pub on_ground: bool,
    /// Pressing against a wall on this side (-1 left, 1 right, 0 none).
    pub wall: i8,
    /// Bumped its head this step.
    pub ceiling: bool,
}

/// What counts as solid for a body.
pub type SolidFn = fn(Material) -> bool;

pub fn player_solid(m: Material) -> bool {
    m.is_solid_for_player()
}

pub fn creature_solid(m: Material) -> bool {
    m.is_solid_for_creature()
}

pub struct MoveResult {
    /// Downward speed at the moment of landing, if the body landed this step.
    pub landed: Option<f32>,
}

impl Body {
    pub fn new(pos: V2, half_w: f32, height: f32) -> Body {
        Body {
            pos,
            vel: V2::ZERO,
            half_w,
            height,
            on_ground: false,
            wall: 0,
            ceiling: false,
        }
    }

    pub fn center(&self) -> V2 {
        v2(self.pos.x, self.pos.y - self.height / 2.0)
    }

    pub fn head(&self) -> V2 {
        v2(self.pos.x, self.pos.y - self.height + 3.0)
    }

    /// Cell ranges covered by the box at `pos`.
    pub fn cells_at(&self, pos: V2) -> (i32, i32, i32, i32) {
        (
            (pos.x - self.half_w).floor() as i32,
            (pos.x + self.half_w - 0.01).floor() as i32,
            (pos.y - self.height).floor() as i32,
            (pos.y - 0.01).floor() as i32,
        )
    }

    pub fn collides(&self, world: &World, pos: V2, solid: SolidFn) -> bool {
        let (x0, x1, y0, y1) = self.cells_at(pos);
        (y0..=y1).any(|y| (x0..=x1).any(|x| solid(world.material(x, y))))
    }

    /// Fraction of the body submerged in liquid, sampled down its centre line.
    pub fn submersion(&self, world: &World) -> f32 {
        let x = self.pos.x.floor() as i32;
        let (_, _, y0, y1) = self.cells_at(self.pos);
        let n = (y0..=y1)
            .filter(|&y| world.material(x, y).kind() == Kind::Liquid)
            .count();
        n as f32 / (y1 - y0 + 1) as f32
    }

    /// Counts cells of `mat` overlapping the body.
    pub fn touching(&self, world: &World, mat: Material) -> usize {
        let (x0, x1, y0, y1) = self.cells_at(self.pos);
        (y0..=y1)
            .flat_map(|y| (x0..=x1).map(move |x| (x, y)))
            .filter(|&(x, y)| world.material(x, y) == mat)
            .count()
    }

    /// Moves by `vel * dt`, one cell at a time, stepping up ledges of up to
    /// `step_up` cells when on the ground.
    pub fn step(&mut self, world: &World, dt: f32, step_up: i32, solid: SolidFn) -> MoveResult {
        let mut result = MoveResult { landed: None };
        self.wall = 0;
        self.ceiling = false;

        let dx = self.vel.x * dt;
        let steps = dx.abs().ceil() as i32;
        for _ in 0..steps {
            let inc = dx / steps as f32;
            let next = v2(self.pos.x + inc, self.pos.y);
            if !self.collides(world, next, solid) {
                self.pos = next;
                continue;
            }
            let climb = if self.on_ground {
                (1..=step_up).find(|&up| !self.collides(world, v2(next.x, next.y - up as f32), solid))
            } else {
                None
            };
            match climb {
                Some(up) => self.pos = v2(next.x, next.y - up as f32),
                None => {
                    self.wall = inc.signum() as i8;
                    self.vel.x = 0.0;
                    break;
                }
            }
        }

        let dy = self.vel.y * dt;
        let steps = dy.abs().ceil() as i32;
        let was_on_ground = self.on_ground;
        self.on_ground = false;
        for _ in 0..steps {
            let inc = dy / steps as f32;
            let next = v2(self.pos.x, self.pos.y + inc);
            if self.collides(world, next, solid) {
                if inc > 0.0 {
                    self.on_ground = true;
                    if !was_on_ground {
                        result.landed = Some(self.vel.y);
                    }
                } else {
                    self.ceiling = true;
                }
                self.vel.y = 0.0;
                break;
            }
            self.pos = next;
        }
        if steps == 0 && self.collides(world, v2(self.pos.x, self.pos.y + 1.0), solid) {
            self.on_ground = true;
        }

        let (w, h) = (world.width() as f32, world.height() as f32);
        self.pos.x = self.pos.x.clamp(self.half_w + 1.0, w - self.half_w - 1.0);
        self.pos.y = self.pos.y.clamp(self.height + 1.0, h - 1.0);
        result
    }

    /// If terrain ended up inside the body, push it up (or sideways) out of it.
    /// Returns false if it's stuck.
    pub fn escape_overlap(&mut self, world: &World, solid: SolidFn) -> bool {
        if !self.collides(world, self.pos, solid) {
            return true;
        }
        for d in 1..=6 {
            for offset in [v2(0.0, -(d as f32)), v2(d as f32, 0.0), v2(-(d as f32), 0.0)] {
                if !self.collides(world, self.pos + offset, solid) {
                    self.pos += offset;
                    return true;
                }
            }
        }
        false
    }
}

/// True if a straight line between two points crosses no solid cell.
pub fn line_of_sight(world: &World, a: V2, b: V2, solid: SolidFn) -> bool {
    let d = b - a;
    let steps = (d.length() / 2.0).ceil().max(1.0) as i32;
    (0..=steps).all(|i| {
        let p = a + d * (i as f32 / steps as f32);
        let (x, y) = p.cell();
        !solid(world.material(x, y))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn floor_world() -> World {
        let mut w = World::new(128, 128, 1);
        for y in 100..128 {
            for x in 0..128 {
                w.set(x, y, Material::Stone);
            }
        }
        w
    }

    #[test]
    fn falls_lands_and_steps_up() {
        let mut w = floor_world();
        let mut b = Body::new(v2(20.0, 60.0), 4.0, 22.0);
        let mut landed = false;
        for _ in 0..200 {
            b.vel.y = (b.vel.y + 400.0 / 60.0).min(300.0);
            landed |= b.step(&w, 1.0 / 60.0, 3, player_solid).landed.is_some();
        }
        assert!(landed && b.on_ground);
        assert!((b.pos.y - 100.0).abs() < 0.1);
        // A 3-cell ledge is climbed, a wall is not.
        for y in 97..100 {
            for x in 40..128 {
                w.set(x, y, Material::Stone);
            }
        }
        for y in 60..97 {
            for x in 80..84 {
                w.set(x, y, Material::Stone);
            }
        }
        for _ in 0..200 {
            b.vel.x = 60.0;
            b.vel.y += 400.0 / 60.0;
            b.step(&w, 1.0 / 60.0, 3, player_solid);
        }
        assert!((b.pos.y - 97.0).abs() < 0.1, "stepped onto the ledge");
        assert!(
            b.pos.x < 80.0 && b.wall == 1,
            "stopped by the wall at {}",
            b.pos.x
        );
    }

    #[test]
    fn glass_stops_creatures_but_not_players() {
        let mut w = floor_world();
        for y in 60..100 {
            w.set(50, y, Material::Glass);
            w.set(51, y, Material::Glass);
        }
        let mut player = Body::new(v2(30.0, 100.0), 4.0, 22.0);
        let mut creature = Body::new(v2(30.0, 100.0), 6.0, 10.0);
        for _ in 0..120 {
            player.vel = v2(60.0, 0.0);
            creature.vel = v2(60.0, 0.0);
            player.step(&w, 1.0 / 60.0, 3, player_solid);
            creature.step(&w, 1.0 / 60.0, 3, creature_solid);
        }
        assert!(player.pos.x > 100.0);
        assert!(creature.pos.x < 50.0);
        assert!(line_of_sight(&w, v2(30.0, 90.0), v2(100.0, 90.0), player_solid));
        assert!(!line_of_sight(
            &w,
            v2(30.0, 90.0),
            v2(100.0, 90.0),
            creature_solid
        ));
    }
}
