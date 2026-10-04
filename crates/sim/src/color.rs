//! Per-cell colours. Pure functions so any renderer (or a test) can shade the world.
//! `frame` animates liquids, lava, glowing materials and electric charge.

use crate::material::{Kind, MAX_CHARGE, Material};
use crate::rng::hash2;
use crate::world::Cell;

#[inline]
fn clamp(v: i32) -> u8 {
    v.clamp(0, 255) as u8
}

/// Cheap periodic wave in [-1, 1].
#[inline]
fn wave(t: f32) -> f32 {
    t.sin()
}

pub fn cell_rgba(cell: Cell, x: i32, y: i32, frame: u32) -> [u8; 4] {
    let mat = cell.mat;
    if mat == Material::Empty {
        return [0, 0, 0, 0];
    }
    let props = mat.props();
    let v = props.variance as i32;
    let offset = if v == 0 {
        0
    } else {
        (cell.shade as i32 * v) / 255 - v / 2
    };
    let [mut r, mut g, mut b] = props.color.map(|c| c as i32 + offset);
    let mut a = 255;
    let t = frame as f32;

    match mat {
        Material::Fire => {
            // Young fire is bright yellow, old fire is dark red; flicker per frame.
            let life = cell.life as i32 * 255 / Material::Fire.initial_life() as i32;
            let flick = (hash2(frame as u64 / 3, x, y) % 40) as i32;
            r = 210 + flick / 2;
            g = 40 + life * 180 / 255 + flick - 20;
            b = 10 + life * 40 / 255;
        }
        Material::Lava => {
            let glow = wave(t * 0.07 + x as f32 * 0.21 + y as f32 * 0.13) * 22.0
                + (hash2(frame as u64 / 6, x, y) % 18) as f32;
            r += glow as i32;
            g += (glow * 0.8) as i32;
        }
        Material::Water | Material::Acid | Material::Oil => {
            let shimmer = wave(t * 0.05 + x as f32 * 0.35 - y as f32 * 0.2) * 10.0;
            r += shimmer as i32;
            g += shimmer as i32;
            b += shimmer as i32;
            if mat != Material::Oil {
                a = 215;
            }
        }
        Material::Metal => {
            let sheen = wave(t * 0.04 + x as f32 * 0.25) * 18.0;
            r += sheen as i32;
            g += sheen as i32;
            b += (sheen * 0.7) as i32;
        }
        Material::Fungus | Material::Crystal | Material::Frost => {
            let pulse = wave(t * 0.03 + cell.shade as f32 * 0.1) * 16.0;
            r += pulse as i32;
            g += pulse as i32;
            b += pulse as i32;
        }
        Material::Spark => {
            let flick = (hash2(frame as u64, x, y) % 60) as i32;
            r = 160 + flick;
            g = 200 + flick / 2;
            b = 255;
        }
        Material::Gravel => {
            // Loose (disturbed) gravel is slightly lighter.
            if cell.life != crate::material::GRAVEL_SETTLED {
                r += 10;
                g += 8;
                b += 6;
            }
        }
        Material::Brick => {
            // Mortar lines every 4 rows, staggered joints every 8 columns.
            let row = y.rem_euclid(4) == 0;
            let joint = (x + if y.rem_euclid(8) < 4 { 0 } else { 4 }).rem_euclid(8) == 0;
            if row || joint {
                r = 96;
                g = 88;
                b = 84;
            }
        }
        Material::Explosive => {
            // Crate: dark edges and a pale band.
            let (lx, ly) = (x.rem_euclid(6), y.rem_euclid(6));
            if lx == 0 || ly == 0 {
                r = 90;
                g = 30;
                b = 26;
            } else if ly == 3 {
                r = 230;
                g = 210;
                b = 150;
            }
        }
        Material::Ore => {
            // Gold flecks.
            if cell.shade > 215 {
                r = 245;
                g = 200;
                b = 70;
            } else if cell.shade > 200 {
                r = 210;
                g = 160;
                b = 60;
            }
        }
        Material::Ferrite => {
            if (x + y * 2).rem_euclid(7) == 0 {
                r += 30;
                g += 30;
                b += 36;
            }
        }
        Material::Vent => {
            if x.rem_euclid(3) == 1 {
                r -= 30;
                g -= 26;
                b -= 26;
            }
        }
        Material::Rope => {
            if y.rem_euclid(3) == 0 {
                r -= 30;
                g -= 26;
                b -= 20;
            }
        }
        Material::Ice => a = 235,
        _ => {}
    }

    match mat.kind() {
        Kind::Gas => {
            if mat.is_persistent_gas() {
                a = 70 + (wave(t * 0.04 + x as f32 * 0.1 + y as f32 * 0.07) * 20.0) as i32;
            } else {
                let max = mat.initial_life().max(1) as i32;
                a = 60 + cell.life as i32 * 140 / max;
            }
        }
        Kind::Liquid | Kind::Solid if mat.props().conductive && cell.life > 0 => {
            // Electric charge: flash towards blue-white.
            let k = cell.life as i32 * 255 / MAX_CHARGE as i32;
            let flick = (hash2(frame as u64 / 2, x, y) % 2) as i32;
            let mix = (k * (1 + flick)) / 2;
            r += (200 - r) * mix / 255;
            g += (230 - g) * mix / 255;
            b += (255 - b) * mix / 255;
            a = 255;
        }
        _ => {}
    }
    [clamp(r), clamp(g), clamp(b), clamp(a)]
}
