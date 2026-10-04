//! Axis-aligned box movement against the cell grid, shared by the player and
//! creatures. Positions are in cells with y growing downward; a body's
//! position is the centre of its feet.

use bevy::prelude::*;
use sbct_sim::{Kind, Material, World};

#[derive(Clone, Copy, Debug)]
pub struct Body {
    pub pos: Vec2,
    pub vel: Vec2,
    pub half_w: f32,
    pub height: f32,
    pub on_ground: bool,
    /// Pressing against a wall on this side (-1 left, 1 right, 0 none).
    pub wall: i32,
}

/// What counts as solid for a body (heavy boots stand on molten metal…).
pub type SolidFn = fn(Material) -> bool;

pub fn default_solid(m: Material) -> bool {
    m.is_solid_for_player()
}

pub struct MoveResult {
    /// Downward speed at the moment of landing, if the body landed this step.
    pub landed: Option<f32>,
}

impl Body {
    pub fn new(pos: Vec2, half_w: f32, height: f32) -> Body {
        Body { pos, vel: Vec2::ZERO, half_w, height, on_ground: false, wall: 0 }
    }

    pub fn center(&self) -> Vec2 {
        self.pos - Vec2::Y * self.height / 2.0
    }

    pub fn head(&self) -> Vec2 {
        self.pos - Vec2::Y * (self.height - 2.0)
    }

    /// Cell ranges covered by the box at `pos`.
    pub fn cells_at(&self, pos: Vec2) -> (i32, i32, i32, i32) {
        (
            (pos.x - self.half_w).floor() as i32,
            (pos.x + self.half_w - 0.01).floor() as i32,
            (pos.y - self.height).floor() as i32,
            (pos.y - 0.01).floor() as i32,
        )
    }

    pub fn collides(&self, world: &World, pos: Vec2, solid: SolidFn) -> bool {
        let (x0, x1, y0, y1) = self.cells_at(pos);
        (y0..=y1).any(|y| (x0..=x1).any(|x| solid(world.material(x, y))))
    }

    /// Visits every cell the body overlaps.
    pub fn for_each_cell(&self, mut f: impl FnMut(i32, i32)) {
        let (x0, x1, y0, y1) = self.cells_at(self.pos);
        for y in y0..=y1 {
            for x in x0..=x1 {
                f(x, y);
            }
        }
    }

    /// Fraction of the body submerged in liquid, sampled down its centre line.
    pub fn submersion(&self, world: &World) -> f32 {
        let x = self.pos.x.floor() as i32;
        let (_, _, y0, y1) = self.cells_at(self.pos);
        let n = (y0..=y1).filter(|&y| world.material(x, y).kind() == Kind::Liquid).count();
        n as f32 / (y1 - y0 + 1) as f32
    }

    /// Moves by `vel * dt`, one cell at a time, stepping up ledges of up to
    /// `step_up` cells when on the ground.
    pub fn step(&mut self, world: &World, dt: f32, step_up: i32, solid: SolidFn) -> MoveResult {
        let mut result = MoveResult { landed: None };
        self.wall = 0;

        let dx = self.vel.x * dt;
        let steps = dx.abs().ceil() as i32;
        for _ in 0..steps {
            let inc = dx / steps as f32;
            let next = self.pos + Vec2::X * inc;
            if !self.collides(world, next, solid) {
                self.pos = next;
                continue;
            }
            let climb = if self.on_ground {
                (1..=step_up).find(|&up| !self.collides(world, next - Vec2::Y * up as f32, solid))
            } else {
                None
            };
            match climb {
                Some(up) => self.pos = next - Vec2::Y * up as f32,
                None => {
                    self.wall = inc.signum() as i32;
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
            let next = self.pos + Vec2::Y * inc;
            if self.collides(world, next, solid) {
                if inc > 0.0 {
                    self.on_ground = true;
                    if !was_on_ground {
                        result.landed = Some(self.vel.y);
                    }
                }
                self.vel.y = 0.0;
                break;
            }
            self.pos = next;
        }
        if steps == 0 && self.collides(world, self.pos + Vec2::Y, solid) {
            self.on_ground = true;
        }

        let (w, h) = (world.width() as f32, world.height() as f32);
        self.pos.x = self.pos.x.clamp(self.half_w + 1.0, w - self.half_w - 1.0);
        self.pos.y = self.pos.y.clamp(self.height + 1.0, h - 1.0);
        result
    }

    /// If terrain ended up inside the body, push it up (or sideways) out of it.
    /// Returns false if it's stuck (being crushed).
    pub fn escape_overlap(&mut self, world: &World, solid: SolidFn) -> bool {
        if !self.collides(world, self.pos, solid) {
            return true;
        }
        for d in 1..=4 {
            for offset in [Vec2::new(0.0, -d as f32), Vec2::new(d as f32, 0.0), Vec2::new(-d as f32, 0.0)] {
                if !self.collides(world, self.pos + offset, solid) {
                    self.pos += offset;
                    return true;
                }
            }
        }
        false
    }
}

/// Cell coordinates to Bevy world space (y up), at depth `z`.
pub fn to_world(p: Vec2, z: f32) -> Vec3 {
    Vec3::new(p.x, -p.y, z)
}
