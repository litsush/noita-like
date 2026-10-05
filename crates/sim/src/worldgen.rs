//! Terrain generation. Deterministic for a given seed.

use crate::material::Material;
use crate::rng::{Rng, hash2};
use crate::world::World;

fn lattice(seed: u64, x: i32, y: i32) -> f32 {
    (hash2(seed, x, y) >> 40) as f32 / (1u64 << 24) as f32
}

/// Smooth value noise in [0, 1].
fn value_noise(seed: u64, x: f32, y: f32) -> f32 {
    let (x0, y0) = (x.floor() as i32, y.floor() as i32);
    let (fx, fy) = (x - x0 as f32, y - y0 as f32);
    let (sx, sy) = (fx * fx * (3.0 - 2.0 * fx), fy * fy * (3.0 - 2.0 * fy));
    let a = lattice(seed, x0, y0);
    let b = lattice(seed, x0 + 1, y0);
    let c = lattice(seed, x0, y0 + 1);
    let d = lattice(seed, x0 + 1, y0 + 1);
    let top = a + (b - a) * sx;
    let bottom = c + (d - c) * sx;
    top + (bottom - top) * sy
}

/// Fractal noise in roughly [0, 1].
pub fn fbm(seed: u64, x: f32, y: f32, octaves: u32) -> f32 {
    let (mut sum, mut amp, mut freq, mut norm) = (0.0, 1.0, 1.0, 0.0);
    for o in 0..octaves {
        sum += value_noise(seed.wrapping_add(o as u64 * 7919), x * freq, y * freq) * amp;
        norm += amp;
        amp *= 0.5;
        freq *= 2.0;
    }
    sum / norm
}

/// Generates a Terraria-style world: hilly surface with trees, a dirt layer,
/// stone below, and caves containing water, oil, and (deep down) lava.
pub fn generate(width: usize, height: usize, seed: u64) -> World {
    let mut world = World::new(width, height, seed);
    let (w, h) = (world.width() as i32, world.height() as i32);
    let mut rng = Rng::new(seed ^ 0xA5A5);

    let surface: Vec<i32> = (0..w)
        .map(|x| {
            let n = fbm(seed, x as f32 / 180.0, 0.0, 4);
            (h as f32 * 0.22 + n * 110.0) as i32
        })
        .collect();

    for x in 0..w {
        let top = surface[x as usize];
        let dirt_depth = 18 + (fbm(seed ^ 1, x as f32 / 40.0, 0.0, 2) * 20.0) as i32;
        for y in top..h {
            let depth = y - top;
            let (fx, fy) = (x as f32, y as f32);

            // Caves widen with depth and never breach the first few pixels of surface.
            let cave = fbm(seed ^ 2, fx / 70.0, fy / 45.0, 4);
            let cave_threshold = 0.66 - (depth as f32 / h as f32) * 0.12;
            if depth > 12 && cave > cave_threshold {
                let pocket = fbm(seed ^ 3, fx / 90.0, fy / 90.0, 2);
                let deep = y > h * 3 / 4;
                if deep && pocket > 0.55 {
                    world.set(x, y, Material::Lava);
                } else if !deep && pocket > 0.66 {
                    world.set(x, y, Material::Water);
                } else if pocket < 0.28 {
                    world.set(x, y, Material::Oil);
                }
                continue;
            }

            let mat = if depth < 2 {
                Material::Grass
            } else if depth < dirt_depth {
                if fbm(seed ^ 4, fx / 25.0, fy / 25.0, 2) > 0.68 {
                    Material::Sand
                } else {
                    Material::Dirt
                }
            } else if fbm(seed ^ 5, fx / 30.0, fy / 30.0, 3) > 0.7 {
                Material::Dirt
            } else {
                Material::Stone
            };
            world.set(x, y, mat);
        }
    }

    // Trees.
    let mut x = 20 + (rng.next_u8() % 40) as i32;
    while x < w - 20 {
        let base = surface[x as usize];
        let flat =
            (surface[(x - 3) as usize] - base).abs() < 4 && (surface[(x + 3) as usize] - base).abs() < 4;
        if flat {
            grow_tree(&mut world, &mut rng, x, base);
        }
        x += 35 + (rng.next_u8() % 60) as i32;
    }

    world
}

fn grow_tree(world: &mut World, rng: &mut Rng, x: i32, ground: i32) {
    let height = 22 + (rng.next_u8() % 18) as i32;
    for y in ground - height..ground + 2 {
        for dx in -1..=1 {
            world.set(x + dx, y, Material::Wood);
        }
    }
    let (cx, cy) = (x, ground - height);
    let r = 8 + (rng.next_u8() % 5) as i32;
    for y in cy - r..=cy + r {
        for lx in cx - r..=cx + r {
            let (dx, dy) = (lx - cx, y - cy);
            let jitter = (rng.next_u8() % 5) as i32;
            if dx * dx + dy * dy <= (r - 2 + jitter / 2).pow(2) && world.material(lx, y) == Material::Empty {
                world.set(lx, y, Material::Grass);
            }
        }
    }
}

/// Finds a spawn point: the first open air above ground near column `x`.
pub fn find_spawn(world: &World, x: i32) -> (i32, i32) {
    for y in 0..world.height() as i32 {
        if world.is_solid(x, y) {
            return (x, (y - 12).max(0));
        }
    }
    (x, 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generation_is_deterministic() {
        let a = generate(256, 256, 42);
        let b = generate(256, 256, 42);
        for cy in 0..a.chunks_y() {
            for cx in 0..a.chunks_x() {
                assert_eq!(a.encode_chunk(cx, cy), b.encode_chunk(cx, cy));
            }
        }
    }

    #[test]
    fn noise_in_range() {
        for i in 0..1000 {
            let v = fbm(9, i as f32 * 0.37, i as f32 * 0.11, 4);
            assert!((0.0..=1.0).contains(&v));
        }
    }
}
