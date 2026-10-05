//! Per-cell colours. Pure functions so any renderer (or a test) can shade the world.
//! `frame` animates liquids, lava and glowing materials.

use crate::material::{Kind, Material};
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
        Material::Water => {
            let shimmer = wave(t * 0.05 + x as f32 * 0.35 - y as f32 * 0.2) * 10.0;
            r += shimmer as i32;
            g += shimmer as i32;
            b += shimmer as i32;
            a = 205;
        }
        Material::Fungus | Material::Crystal => {
            let pulse = wave(t * 0.03 + cell.shade as f32 * 0.1) * 16.0;
            r += pulse as i32;
            g += pulse as i32;
            b += pulse as i32;
        }
        Material::Gravel => {
            // Loose (disturbed) gravel is slightly lighter.
            if cell.life != crate::material::GRAVEL_SETTLED {
                r += 10;
                g += 8;
                b += 6;
            }
        }
        Material::Grass | Material::Moss | Material::Leaves => {
            // Speckles of lighter growth and the odd luminous bud.
            if cell.shade > 236 {
                r += 40;
                g += 60;
                b += 30;
            } else if cell.shade < 18 {
                r -= 14;
                g -= 22;
                b -= 10;
            }
        }
        Material::Wood => {
            // Vertical grain.
            if (x + (cell.shade as i32 >> 6)).rem_euclid(3) == 0 {
                r -= 14;
                g -= 12;
                b -= 10;
            }
        }
        Material::Ruin => {
            // Panel seams and streaks of rust.
            let seam = x.rem_euclid(12) == 0 || y.rem_euclid(9) == 0;
            if seam {
                r -= 28;
                g -= 28;
                b -= 26;
            } else if cell.shade > 200 {
                r += 38;
                g -= 4;
                b -= 22;
            } else if cell.shade < 30 {
                r -= 30;
                g += 14;
                b -= 6;
            }
        }
        Material::Plating => {
            let seam = x.rem_euclid(8) == 0;
            if seam {
                r -= 22;
                g -= 22;
                b -= 22;
            } else if y.rem_euclid(4) == 0 {
                r += 10;
                g += 10;
                b += 12;
            }
        }
        Material::Glass => {
            a = 150;
            if (x + y).rem_euclid(9) == 0 {
                r += 50;
                g += 46;
                b += 40;
                a = 210;
            }
        }
        Material::Ice => a = 235,
        m if m.is_ore() => {
            // Host rock with flecks of ore; richer ores glint.
            let fleck = cell.shade;
            if fleck < 120 {
                let host = if matches!(m, Material::XeniteOre | Material::AuroriumOre) {
                    [44, 40, 58]
                } else {
                    [86, 88, 100]
                };
                let k = (fleck as i32 % 13) - 6;
                r = host[0] + k;
                g = host[1] + k;
                b = host[2] + k;
            } else if fleck > 228 {
                // Only the luminous ores shimmer (see `World::chunk_animates`).
                let glow = matches!(m, Material::XeniteOre | Material::AuroriumOre);
                let glint = if glow {
                    (wave(t * 0.06 + fleck as f32) * 24.0) as i32
                } else {
                    0
                } + 36;
                r += glint;
                g += glint;
                b += glint;
            }
        }
        _ => {}
    }

    if mat.kind() == Kind::Gas {
        if mat.is_persistent_gas() {
            a = 80 + (wave(t * 0.04 + x as f32 * 0.1 + y as f32 * 0.07) * 24.0) as i32;
        } else {
            let max = mat.initial_life().max(1) as i32;
            a = 60 + cell.life as i32 * 140 / max;
        }
    }
    [clamp(r), clamp(g), clamp(b), clamp(a)]
}
