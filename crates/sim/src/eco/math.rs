//! A tiny 2D vector so the simulation stays free of engine dependencies.
//! World space is in cells with +y pointing down.

use std::ops::{Add, AddAssign, Div, Mul, MulAssign, Neg, Sub, SubAssign};

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct V2 {
    pub x: f32,
    pub y: f32,
}

#[inline]
pub const fn v2(x: f32, y: f32) -> V2 {
    V2 { x, y }
}

impl V2 {
    pub const ZERO: V2 = v2(0.0, 0.0);

    #[inline]
    pub fn len(self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
    #[inline]
    pub fn len2(self) -> f32 {
        self.x * self.x + self.y * self.y
    }
    #[inline]
    pub fn dist(self, o: V2) -> f32 {
        (self - o).len()
    }
    #[inline]
    pub fn norm(self) -> V2 {
        let l = self.len();
        if l < 1e-6 { V2::ZERO } else { self / l }
    }
    #[inline]
    pub fn dot(self, o: V2) -> f32 {
        self.x * o.x + self.y * o.y
    }
    #[inline]
    pub fn lerp(self, o: V2, t: f32) -> V2 {
        self + (o - self) * t
    }
    #[inline]
    pub fn angle(self) -> f32 {
        self.y.atan2(self.x)
    }
    #[inline]
    pub fn from_angle(a: f32) -> V2 {
        v2(a.cos(), a.sin())
    }
    #[inline]
    pub fn rot(self, a: f32) -> V2 {
        let (s, c) = a.sin_cos();
        v2(self.x * c - self.y * s, self.x * s + self.y * c)
    }
    #[inline]
    pub fn perp(self) -> V2 {
        v2(-self.y, self.x)
    }
    /// Shortens the vector to at most `max`.
    #[inline]
    pub fn clamp_len(self, max: f32) -> V2 {
        let l = self.len();
        if l > max && l > 0.0 {
            self * (max / l)
        } else {
            self
        }
    }
    #[inline]
    pub fn cell(self) -> (i32, i32) {
        (self.x.floor() as i32, self.y.floor() as i32)
    }
}

impl Add for V2 {
    type Output = V2;
    #[inline]
    fn add(self, o: V2) -> V2 {
        v2(self.x + o.x, self.y + o.y)
    }
}
impl Sub for V2 {
    type Output = V2;
    #[inline]
    fn sub(self, o: V2) -> V2 {
        v2(self.x - o.x, self.y - o.y)
    }
}
impl Mul<f32> for V2 {
    type Output = V2;
    #[inline]
    fn mul(self, s: f32) -> V2 {
        v2(self.x * s, self.y * s)
    }
}
impl Div<f32> for V2 {
    type Output = V2;
    #[inline]
    fn div(self, s: f32) -> V2 {
        v2(self.x / s, self.y / s)
    }
}
impl Neg for V2 {
    type Output = V2;
    #[inline]
    fn neg(self) -> V2 {
        v2(-self.x, -self.y)
    }
}
impl AddAssign for V2 {
    #[inline]
    fn add_assign(&mut self, o: V2) {
        self.x += o.x;
        self.y += o.y;
    }
}
impl SubAssign for V2 {
    #[inline]
    fn sub_assign(&mut self, o: V2) {
        self.x -= o.x;
        self.y -= o.y;
    }
}
impl MulAssign<f32> for V2 {
    #[inline]
    fn mul_assign(&mut self, s: f32) {
        self.x *= s;
        self.y *= s;
    }
}

/// Smallest signed difference between two angles, in (-PI, PI].
pub fn angle_diff(a: f32, b: f32) -> f32 {
    let mut d = (b - a) % std::f32::consts::TAU;
    if d > std::f32::consts::PI {
        d -= std::f32::consts::TAU;
    } else if d < -std::f32::consts::PI {
        d += std::f32::consts::TAU;
    }
    d
}

/// Moves `a` towards `b` by at most `max`.
pub fn approach(a: f32, b: f32, max: f32) -> f32 {
    if (b - a).abs() <= max {
        b
    } else {
        a + max * (b - a).signum()
    }
}

/// Distance from `p` to the segment `a`-`b`.
pub fn seg_dist(p: V2, a: V2, b: V2) -> f32 {
    let ab = b - a;
    let t = if ab.len2() < 1e-6 {
        0.0
    } else {
        ((p - a).dot(ab) / ab.len2()).clamp(0.0, 1.0)
    };
    p.dist(a + ab * t)
}

/// RGB colour helpers used by generation and drawing.
pub type Rgb = [u8; 3];

pub fn hsv(h: f32, s: f32, v: f32) -> Rgb {
    let h = h.rem_euclid(360.0) / 60.0;
    let c = v * s;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as i32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = v - c;
    [
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    ]
}

/// Hue in degrees, saturation and value of an RGB colour.
pub fn to_hsv(c: Rgb) -> (f32, f32, f32) {
    let [r, g, b] = c.map(|v| v as f32 / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d < 1e-5 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / d).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / d + 2.0)
    } else {
        60.0 * ((r - g) / d + 4.0)
    };
    let s = if max <= 0.0 { 0.0 } else { d / max };
    (h, s, max)
}

pub fn mix(a: Rgb, b: Rgb, t: f32) -> Rgb {
    let t = t.clamp(0.0, 1.0);
    [0, 1, 2].map(|i| (a[i] as f32 + (b[i] as f32 - a[i] as f32) * t) as u8)
}

pub fn scale(c: Rgb, f: f32) -> Rgb {
    c.map(|v| (v as f32 * f).clamp(0.0, 255.0) as u8)
}
