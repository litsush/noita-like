//! A tiny 2D vector, so the rules don't depend on an engine's math types.
//! Positions are in cells with y growing downward.

use serde::{Deserialize, Serialize};
use std::ops::{Add, AddAssign, Mul, Sub};

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct V2 {
    pub x: f32,
    pub y: f32,
}

pub const fn v2(x: f32, y: f32) -> V2 {
    V2 { x, y }
}

impl V2 {
    pub const ZERO: V2 = v2(0.0, 0.0);

    pub fn length(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }

    pub fn distance(self, o: V2) -> f32 {
        (self - o).length()
    }

    /// Unit vector, or zero for a zero vector.
    pub fn normalized(self) -> V2 {
        let l = self.length();
        if l > 1e-6 {
            v2(self.x / l, self.y / l)
        } else {
            V2::ZERO
        }
    }

    pub fn cell(self) -> (i32, i32) {
        (self.x.floor() as i32, self.y.floor() as i32)
    }

    pub fn lerp(self, o: V2, t: f32) -> V2 {
        self + (o - self) * t
    }
}

impl Add for V2 {
    type Output = V2;
    fn add(self, o: V2) -> V2 {
        v2(self.x + o.x, self.y + o.y)
    }
}

impl AddAssign for V2 {
    fn add_assign(&mut self, o: V2) {
        *self = *self + o;
    }
}

impl Sub for V2 {
    type Output = V2;
    fn sub(self, o: V2) -> V2 {
        v2(self.x - o.x, self.y - o.y)
    }
}

impl Mul<f32> for V2 {
    type Output = V2;
    fn mul(self, k: f32) -> V2 {
        v2(self.x * k, self.y * k)
    }
}

/// An axis-aligned box in cells; `x1`/`y1` are exclusive.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct Rect {
    pub x0: i32,
    pub y0: i32,
    pub x1: i32,
    pub y1: i32,
}

impl Rect {
    pub fn new(x0: i32, y0: i32, x1: i32, y1: i32) -> Rect {
        Rect {
            x0: x0.min(x1),
            y0: y0.min(y1),
            x1: x0.max(x1),
            y1: y0.max(y1),
        }
    }

    pub fn contains(&self, p: V2) -> bool {
        p.x >= self.x0 as f32 && p.x < self.x1 as f32 && p.y >= self.y0 as f32 && p.y < self.y1 as f32
    }

    pub fn center(&self) -> V2 {
        v2((self.x0 + self.x1) as f32 / 2.0, (self.y0 + self.y1) as f32 / 2.0)
    }

    pub fn grown(&self, by: i32) -> Rect {
        Rect {
            x0: self.x0 - by,
            y0: self.y0 - by,
            x1: self.x1 + by,
            y1: self.y1 + by,
        }
    }

    pub fn overlaps(&self, o: &Rect) -> bool {
        self.x0 < o.x1 && o.x0 < self.x1 && self.y0 < o.y1 && o.y0 < self.y1
    }

    /// Distance from a point to the box (0 inside).
    pub fn distance(&self, p: V2) -> f32 {
        let dx = (self.x0 as f32 - p.x).max(p.x - self.x1 as f32).max(0.0);
        let dy = (self.y0 as f32 - p.y).max(p.y - self.y1 as f32).max(0.0);
        (dx * dx + dy * dy).sqrt()
    }
}
