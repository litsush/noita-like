//! Planet generation: a rolling surface with lakes, alien trees and ruins
//! over contiguous cave networks in three depth bands, with ore veins that
//! get richer with depth. Deterministic for a given seed.
//!
//! The surface line and lake basins come from [`profile`], a cheap pure
//! function of the seed, so clients can rebuild it without the cell data.

use crate::material::Material;
use crate::rng::{Rng, hash2};
use crate::world::{World, disc};

pub const WORLD_WIDTH: usize = 5120;
pub const WORLD_HEIGHT: usize = 1536;
/// Half-width of the levelled ground the starter dome stands on.
pub const PLATEAU_HALF: i32 = 120;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Band {
    Sky,
    Shallows,
    Deeps,
    Abyss,
}

#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Lake {
    pub x0: i32,
    pub x1: i32,
    /// Row of the water surface.
    pub level: i32,
}

/// The shape of the land before caves: ground height per column, lakes and
/// the depth bands.
#[derive(Clone, Debug, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct Profile {
    pub width: i32,
    pub height: i32,
    /// First ground row of each column.
    pub surface: Vec<i32>,
    pub lakes: Vec<Lake>,
    /// Column of the middle of the starter plateau.
    pub spawn_x: i32,
    /// First row of the Deeps and of the Abyss.
    pub deeps_y: i32,
    pub abyss_y: i32,
}

impl Profile {
    pub fn surface_at(&self, x: i32) -> i32 {
        self.surface[x.clamp(0, self.width - 1) as usize]
    }

    /// Ground row at the spawn plateau.
    pub fn spawn_y(&self) -> i32 {
        self.surface_at(self.spawn_x)
    }

    pub fn band(&self, x: i32, y: i32) -> Band {
        if y < self.surface_at(x) {
            Band::Sky
        } else if y < self.deeps_y {
            Band::Shallows
        } else if y < self.abyss_y {
            Band::Deeps
        } else {
            Band::Abyss
        }
    }

    pub fn lake_at(&self, x: i32) -> Option<&Lake> {
        self.lakes.iter().find(|l| x >= l.x0 && x <= l.x1)
    }

    /// 0 at the surface, 1 at the bottom of the world.
    pub fn depth_fraction(&self, y: i32) -> f32 {
        let top = self.spawn_y();
        ((y - top) as f32 / (self.height - top) as f32).clamp(0.0, 1.0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum FeatureKind {
    /// A sealed room of the old ruins holding a cache.
    Vault,
    /// A Brood Mother's cavern, studded with aurorium.
    Lair,
    /// A ruined structure on the surface.
    Ruin,
    /// Where a cave shaft opens onto the surface.
    Entrance,
}

/// Something world generation placed. `(x, y)` is the middle of its floor.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Feature {
    pub kind: FeatureKind,
    pub x: i32,
    pub y: i32,
}

pub struct Planet {
    pub world: World,
    pub profile: Profile,
    pub features: Vec<Feature>,
}

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

fn smoothstep(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// The land's outline for a seed. Cheap (one pass over the columns).
pub fn profile(seed: u64, width: usize, height: usize) -> Profile {
    let (w, h) = (width as i32, height as i32);
    let base = h as f32 * 0.21;
    let mut surface: Vec<i32> = (0..w)
        .map(|x| {
            let hills = fbm(seed ^ 0x51, x as f32 / 420.0, 0.0, 3) * 110.0 - 55.0;
            let bumps = fbm(seed ^ 0x52, x as f32 / 90.0, 0.0, 2) * 24.0 - 12.0;
            (base + hills + bumps) as i32
        })
        .collect();

    // Level the ground for the starter dome.
    let spawn_x = w / 2;
    let plateau = surface[spawn_x as usize];
    for x in 0..w {
        let d = (x - spawn_x).abs() - PLATEAU_HALF;
        let t = smoothstep(d as f32 / 70.0);
        let s = &mut surface[x as usize];
        *s = (plateau as f32 + (*s - plateau) as f32 * t).round() as i32;
    }

    // Lakes: one per slot of land, kept clear of the plateau.
    let mut rng = Rng::new(seed ^ 0x1A4E);
    let mut lakes = Vec::new();
    let slots = (w / 900).max(2);
    for i in 0..slots {
        let slot_w = w / slots;
        let half = 70 + (rng.next_u8() % 60) as i32;
        let cx = i * slot_w + half + 30 + (rng.next_u64() % (slot_w - 2 * half - 60).max(1) as u64) as i32;
        if (cx - spawn_x).abs() < PLATEAU_HALF + half + 160 {
            continue;
        }
        let (x0, x1) = ((cx - half).max(8), (cx + half).min(w - 9));
        let rim = surface[x0 as usize].max(surface[x1 as usize]);
        let depth = 26.0 + (rng.next_u8() % 20) as f32;
        for x in x0..=x1 {
            let t = (x - cx) as f32 / half as f32;
            let bowl = depth * (1.0 - t * t).max(0.0).powf(0.8);
            let s = &mut surface[x as usize];
            *s = (*s).max(rim) + bowl as i32;
        }
        // Ease the banks so higher ground slopes down to the rim.
        for d in 1..=60 {
            for x in [x0 - d, x1 + d] {
                if x >= 0 && x < w {
                    let s = &mut surface[x as usize];
                    *s = (*s).max(rim - (d as f32 * 0.6) as i32);
                }
            }
        }
        lakes.push(Lake {
            x0,
            x1,
            level: rim + 2,
        });
    }

    let top = plateau;
    let under = h - top;
    Profile {
        width: w,
        height: h,
        surface,
        lakes,
        spawn_x,
        deeps_y: top + (under as f32 * 0.30) as i32,
        abyss_y: top + (under as f32 * 0.65) as i32,
    }
}

pub fn generate(seed: u64) -> Planet {
    generate_sized(WORLD_WIDTH, WORLD_HEIGHT, seed)
}

pub fn generate_sized(width: usize, height: usize, seed: u64) -> Planet {
    let mut world = World::new(width, height, seed);
    let profile = profile(seed, world.width(), world.height());
    let mut rng = Rng::new(seed ^ 0x9E0);
    let mut features = Vec::new();

    fill_rock(&mut world, &profile, seed);
    carve_entrances(&mut world, &profile, &mut rng, &mut features);
    stamp_vaults(&mut world, &profile, &mut rng, &mut features);
    stamp_lairs(&mut world, &profile, &mut rng, &mut features);
    dress_caves(&mut world, &profile, seed);
    fill_lakes(&mut world, &profile);
    grow_trees(&mut world, &profile, &mut rng);
    stamp_ruins(&mut world, &profile, &mut rng, &mut features);
    seal_edges(&mut world);
    world.settle_gravel();
    world.take_events();

    Planet {
        world,
        profile,
        features,
    }
}

/// Is this cell hollowed out by the cave noise?
fn is_cave(seed: u64, p: &Profile, x: i32, y: i32) -> bool {
    let top = p.surface_at(x);
    let depth = y - top;
    // Keep the ground under the plateau and the lake beds intact.
    let guard = if (x - p.spawn_x).abs() < PLATEAU_HALF + 60 {
        70
    } else if p.lake_at(x).is_some() {
        46
    } else {
        16
    };
    if depth < guard {
        return false;
    }
    let (fx, fy) = (x as f32, y as f32);
    let deep = p.depth_fraction(y);
    // Winding tunnels: thin bands around the 0.5 contour of two noises.
    let a = fbm(seed ^ 0xC1, fx / 190.0, fy / 120.0, 3);
    if (a - 0.5).abs() < 0.030 + deep * 0.012 {
        return true;
    }
    let b = fbm(seed ^ 0xC2, fx / 150.0, fy / 210.0, 3);
    if (b - 0.5).abs() < 0.022 + deep * 0.012 {
        return true;
    }
    // Open caverns, more of them further down.
    let c = fbm(seed ^ 0xC3, fx / 240.0, fy / 150.0, 3);
    c > 0.715 - deep * 0.07
}

/// Which ore, if any, a rock cell holds.
fn ore_at(seed: u64, p: &Profile, x: i32, y: i32) -> Option<Material> {
    let (fx, fy) = (x as f32, y as f32);
    let vein = fbm(seed ^ 0x0E1, fx / 15.0, fy / 15.0, 2);
    let deep = p.depth_fraction(y);
    if vein < 0.735 - deep * 0.03 {
        return None;
    }
    // A slow-changing value picks the kind, so veins of one ore cluster.
    let kind = fbm(seed ^ 0x0E2, fx / 110.0, fy / 110.0, 2);
    let k = ((kind - 0.25) / 0.5).clamp(0.0, 0.999);
    let table: &[Material] = match p.band(x, y) {
        Band::Sky => return None,
        Band::Shallows => &[
            Material::IronOre,
            Material::CopperOre,
            Material::IronOre,
            Material::SiliconOre,
            Material::CopperOre,
        ],
        Band::Deeps => &[
            Material::IronOre,
            Material::GoldOre,
            Material::CopperOre,
            Material::TitaniumOre,
            Material::SiliconOre,
            Material::GoldOre,
            Material::TitaniumOre,
        ],
        Band::Abyss => &[
            Material::TitaniumOre,
            Material::XeniteOre,
            Material::GoldOre,
            Material::XeniteOre,
            Material::IronOre,
            Material::XeniteOre,
        ],
    };
    Some(table[(k * table.len() as f32) as usize])
}

fn fill_rock(world: &mut World, p: &Profile, seed: u64) {
    let (w, h) = (p.width, p.height);
    for x in 0..w {
        let top = p.surface_at(x);
        let dirt_depth = 22 + (fbm(seed ^ 1, x as f32 / 40.0, 0.0, 2) * 22.0) as i32;
        let grassy = fbm(seed ^ 0x6A, x as f32 / 55.0, 0.0, 2) > 0.42;
        for y in top..h {
            if is_cave(seed, p, x, y) {
                continue;
            }
            let depth = y - top;
            let (fx, fy) = (x as f32, y as f32);
            let mat = if depth < dirt_depth {
                if depth < 3 && grassy && p.lake_at(x).is_none() {
                    Material::Grass
                } else if fbm(seed ^ 4, fx / 25.0, fy / 25.0, 2) > 0.7 {
                    Material::Sand
                } else {
                    Material::Dirt
                }
            } else if let Some(ore) = ore_at(seed, p, x, y) {
                ore
            } else {
                let blend = fbm(seed ^ 5, fx / 60.0, fy / 34.0, 3);
                match p.band(x, y) {
                    Band::Shallows | Band::Sky => {
                        if blend > 0.72 {
                            Material::Dirt
                        } else if blend < 0.24 {
                            Material::Gravel
                        } else {
                            Material::Stone
                        }
                    }
                    Band::Deeps => {
                        // Stone gives way to basalt with depth.
                        let t = (y - p.deeps_y) as f32 / (p.abyss_y - p.deeps_y) as f32;
                        if blend < 0.2 {
                            Material::Gravel
                        } else if blend > 0.86 - t * 0.5 {
                            Material::Basalt
                        } else {
                            Material::Stone
                        }
                    }
                    Band::Abyss => {
                        if blend > 0.78 {
                            Material::Obsidian
                        } else if blend < 0.16 {
                            Material::Stone
                        } else {
                            Material::Basalt
                        }
                    }
                }
            };
            world.set(x, y, mat);
        }
    }
}

fn carve_disc(world: &mut World, cx: i32, cy: i32, r: i32) {
    for (x, y) in disc(cx, cy, r) {
        if world.in_bounds(x, y) && world.material(x, y) != Material::Empty {
            world.set(x, y, Material::Empty);
        }
    }
}

fn clear_of_landmarks(p: &Profile, x: i32, margin: i32) -> bool {
    (x - p.spawn_x).abs() > PLATEAU_HALF + margin
        && p.lakes
            .iter()
            .all(|l| x < l.x0 - margin / 2 || x > l.x1 + margin / 2)
        && x > 40
        && x < p.width - 40
}

/// Sloping shafts from the surface down into the cave network.
fn carve_entrances(world: &mut World, p: &Profile, rng: &mut Rng, features: &mut Vec<Feature>) {
    let count = (p.width / 420).max(2);
    for i in 0..count {
        let slot = p.width / count;
        let mut x = i * slot + 60 + (rng.next_u64() % (slot - 120).max(1) as u64) as i32;
        if !clear_of_landmarks(p, x, 90) {
            // Nudge it away from the plateau rather than losing the entrance.
            x += if x < p.spawn_x { -260 } else { 260 };
            if !clear_of_landmarks(p, x, 90) {
                continue;
            }
        }
        let top = p.surface_at(x);
        features.push(Feature {
            kind: FeatureKind::Entrance,
            x,
            y: top,
        });
        let dir = if rng.coin() { 1.0 } else { -1.0 };
        let (mut fx, mut fy) = (x as f32, top as f32 - 4.0);
        let mut angle: f32 = 0.95 + rng.next_f32() * 0.35;
        let max_depth = 190 + (rng.next_u8() % 120) as i32;
        let mut open_run = 0;
        while (fy as i32) < top + max_depth && (fy as i32) < p.height - 30 {
            let r = 9 + (rng.next_u8() % 4) as i32;
            // Stop once the shaft has broken into an existing cave.
            let ahead = world.material(
                (fx + dir * angle.cos() * (r + 6) as f32) as i32,
                (fy + angle.sin() * (r + 6) as f32) as i32,
            );
            if ahead == Material::Empty && fy as i32 > top + 40 {
                open_run += 1;
                if open_run > 6 {
                    break;
                }
            } else {
                open_run = 0;
            }
            carve_disc(world, fx as i32, fy as i32, r);
            angle = (angle + (rng.next_f32() - 0.5) * 0.25).clamp(0.55, 1.35);
            fx += dir * angle.cos() * 4.0;
            fy += angle.sin() * 4.0;
        }
    }
}

fn fill_rect(world: &mut World, x0: i32, y0: i32, x1: i32, y1: i32, mat: Material) {
    for y in y0..=y1 {
        for x in x0..=x1 {
            if world.in_bounds(x, y) {
                world.set(x, y, mat);
            }
        }
    }
}

/// Sealed rooms of the old ruins, each holding a cache.
fn stamp_vaults(world: &mut World, p: &Profile, rng: &mut Rng, features: &mut Vec<Feature>) {
    let count = (p.width / 640).max(1);
    for i in 0..count {
        let slot = p.width / count;
        let x = i * slot + 80 + (rng.next_u64() % (slot - 160).max(1) as u64) as i32;
        // Alternate between the Shallows and the Deeps.
        let (lo, hi) = if i % 2 == 0 {
            (p.surface_at(x) + 110, p.deeps_y - 50)
        } else {
            (p.deeps_y + 30, p.abyss_y - 60)
        };
        if hi <= lo {
            continue;
        }
        let y = lo + (rng.next_u64() % (hi - lo) as u64) as i32;
        let (hw, hh) = (36, 34);
        fill_rect(world, x - hw - 3, y - hh - 3, x + hw + 3, y + 3, Material::Ruin);
        fill_rect(world, x - hw, y - hh, x + hw, y, Material::Empty);
        // A pillar and a broken doorway on one side.
        fill_rect(world, x - 2, y - hh, x + 2, y - hh + 8, Material::Ruin);
        let side = if rng.coin() { 1 } else { -1 };
        fill_rect(
            world,
            x + side * hw + side.min(0) * 3,
            y - 26,
            x + side * hw + side.max(0) * 3,
            y,
            Material::Empty,
        );
        features.push(Feature {
            kind: FeatureKind::Vault,
            x,
            y,
        });
    }
}

/// Brood Mother caverns in the Abyss with aurorium in the walls.
fn stamp_lairs(world: &mut World, p: &Profile, rng: &mut Rng, features: &mut Vec<Feature>) {
    let count = (p.width / 1700).max(1);
    for i in 0..count {
        let slot = p.width / count;
        let x = i * slot + 160 + (rng.next_u64() % (slot - 320).max(1) as u64) as i32;
        let lo = p.abyss_y + 80;
        let hi = p.height - 110;
        if hi <= lo {
            continue;
        }
        let y = lo + (rng.next_u64() % (hi - lo) as u64) as i32;
        let (a, b) = (72.0f32, 40.0f32);
        for cy in -(b as i32)..=0 {
            for cx in -(a as i32)..=a as i32 {
                let d = (cx as f32 / a).powi(2) + (cy as f32 / b).powi(2);
                if d <= 1.0 {
                    world.set(x + cx, y + cy, Material::Empty);
                }
            }
        }
        fill_rect(world, x - a as i32, y + 1, x + a as i32, y + 5, Material::Basalt);
        for _ in 0..12 {
            let t = rng.next_f32() * std::f32::consts::PI;
            let (ox, oy) = ((t.cos() * (a + 4.0)) as i32, (-t.sin() * (b + 4.0)) as i32);
            for (cx, cy) in disc(x + ox, y + oy, 3 + (rng.next_u8() % 3) as i32) {
                if world.in_bounds(cx, cy) && world.material(cx, cy) != Material::Empty {
                    world.set(cx, cy, Material::AuroriumOre);
                }
            }
        }
        features.push(Feature {
            kind: FeatureKind::Lair,
            x,
            y,
        });
    }
}

/// Moss, fungus, crystals, pools, gas and ice along the cave walls.
fn dress_caves(world: &mut World, p: &Profile, seed: u64) {
    let (w, h) = (p.width, p.height);
    let mut rng = Rng::new(seed ^ 0xD2E55);
    for x in 1..w - 1 {
        let top = p.surface_at(x);
        for y in (top + 12).max(1)..h - 1 {
            let m = world.material(x, y);
            let band = p.band(x, y);
            let (fx, fy) = (x as f32, y as f32);
            if m == Material::Empty {
                let floor = world.material(x, y + 1).is_solid_for_player();
                let ceiling = world.material(x, y - 1).is_solid_for_player();
                if floor && x % 2 == 0 {
                    let wet = fbm(seed ^ 0x77, fx / 130.0, fy / 130.0, 2);
                    let pool = match band {
                        Band::Shallows | Band::Deeps if wet > 0.6 => Some(Material::Water),
                        Band::Abyss if wet > 0.56 => Some(Material::Lava),
                        _ => None,
                    };
                    if let Some(liquid) = pool {
                        fill_pool(world, x, y, liquid, 3 + ((wet - 0.55) * 70.0) as i32);
                    }
                } else if ceiling && band == Band::Deeps {
                    let spores = fbm(seed ^ 0x78, fx / 150.0, fy / 150.0, 2);
                    if spores > 0.69 {
                        for dy in 0..(6 + ((spores - 0.69) * 90.0) as i32).min(16) {
                            if world.material(x, y + dy) != Material::Empty {
                                break;
                            }
                            world.set(x, y + dy, Material::Spore);
                        }
                    }
                }
                continue;
            }
            if !matches!(m, Material::Stone | Material::Dirt | Material::Basalt) {
                continue;
            }
            let exposed = [(0, -1), (0, 1), (-1, 0), (1, 0)]
                .iter()
                .any(|(dx, dy)| world.material(x + dx, y + dy) == Material::Empty);
            if !exposed {
                continue;
            }
            let roll = rng.next_u8();
            let frost = fbm(seed ^ 0x79, fx / 260.0, fy / 200.0, 2) > 0.7;
            let lining = match band {
                _ if frost && band != Band::Abyss => (roll < 190).then_some(Material::Ice),
                Band::Shallows => (roll < 96).then_some(Material::Moss),
                Band::Deeps => {
                    let crystals = fbm(seed ^ 0x7A, fx / 70.0, fy / 70.0, 2) > 0.66;
                    if crystals && roll < 120 {
                        Some(Material::Crystal)
                    } else if roll < 14 {
                        Some(Material::Fungus)
                    } else if roll < 50 {
                        Some(Material::Moss)
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some(l) = lining {
                world.set(x, y, l);
            }
        }
    }
}

/// Fills a basin with liquid from its floor at (x, y) upward, one level
/// row at a time, stopping when the liquid would spill.
fn fill_pool(world: &mut World, x: i32, y: i32, liquid: Material, max_depth: i32) {
    const REACH: i32 = 70;
    let holds = |world: &World, cx: i32, cy: i32| {
        let below = world.material(cx, cy + 1);
        below.is_solid_for_player() || below == liquid
    };
    for row in 0..max_depth.min(14) {
        let cy = y - row;
        if world.material(x, cy) != Material::Empty {
            return;
        }
        let mut span = (x, x);
        for dir in [-1, 1] {
            let mut cx = x;
            loop {
                if !holds(world, cx, cy) || (cx - x).abs() > REACH {
                    return;
                }
                let next = world.material(cx + dir, cy);
                if next != Material::Empty {
                    if next.is_solid_for_player() || next == liquid {
                        break;
                    }
                    return;
                }
                cx += dir;
            }
            if dir < 0 { span.0 = cx } else { span.1 = cx }
        }
        for cx in span.0..=span.1 {
            world.set(cx, cy, liquid);
        }
    }
}

fn fill_lakes(world: &mut World, p: &Profile) {
    for lake in &p.lakes {
        for x in lake.x0..=lake.x1 {
            let top = p.surface_at(x);
            for y in lake.level..top {
                world.set(x, y, Material::Water);
            }
            // Sandy bed.
            for y in top..top + 5 {
                if world.material(x, y) != Material::Empty {
                    world.set(x, y, Material::Sand);
                }
            }
        }
    }
}

fn grow_trees(world: &mut World, p: &Profile, rng: &mut Rng) {
    let mut x = 30 + (rng.next_u8() % 40) as i32;
    while x < p.width - 30 {
        let ground = p.surface_at(x);
        let flat = (p.surface_at(x - 5) - ground).abs() < 6 && (p.surface_at(x + 5) - ground).abs() < 6;
        let grounded = world.material(x, ground).is_solid_for_player();
        if flat && grounded && clear_of_landmarks(p, x, 40) {
            grow_tree(world, rng, x, ground);
        }
        x += 44 + (rng.next_u8() % 90) as i32;
    }
}

fn leaf(world: &mut World, x: i32, y: i32) {
    if world.in_bounds(x, y) && world.material(x, y) == Material::Empty {
        world.set(x, y, Material::Leaves);
    }
}

/// An alien tree: a leaning trunk with one of three canopies.
fn grow_tree(world: &mut World, rng: &mut Rng, x: i32, ground: i32) {
    let height = 34 + (rng.next_u8() % 40) as i32;
    let lean = (rng.next_f32() - 0.5) * 0.5;
    let half = 1 + (rng.next_u8() % 2) as i32;
    let mut top = (x, ground);
    for i in -3..height {
        let cx = x + (lean * i.max(0) as f32) as i32;
        for dx in -half..=half {
            let (tx, ty) = (cx + dx, ground - i);
            if world.in_bounds(tx, ty) && world.material(tx, ty).is_open() {
                world.set(tx, ty, Material::Wood);
            }
        }
        top = (cx, ground - i);
    }
    let (cx, cy) = top;
    match rng.next_u8() % 3 {
        0 => {
            // Umbrella: a wide flat dome with a ragged underside.
            let (a, b) = (16 + (rng.next_u8() % 10) as i32, 9 + (rng.next_u8() % 5) as i32);
            for dy in -b..=2 {
                for dx in -a..=a {
                    let d = (dx as f32 / a as f32).powi(2) + (dy as f32 / b as f32).powi(2);
                    if d <= 1.0 && (dy < 0 || rng.chance(110)) {
                        leaf(world, cx + dx, cy + dy);
                    }
                }
            }
        }
        1 => {
            // Bulbs: a cluster of round pods.
            for _ in 0..3 + rng.next_u8() % 3 {
                let (ox, oy) = ((rng.next_u8() % 25) as i32 - 12, -((rng.next_u8() % 14) as i32));
                let r = 6 + (rng.next_u8() % 5) as i32;
                for (lx, ly) in disc(cx + ox, cy + oy, r) {
                    leaf(world, lx, ly);
                }
            }
        }
        _ => {
            // Weeping: a small crown with long hanging fronds.
            let r = 8 + (rng.next_u8() % 4) as i32;
            for (lx, ly) in disc(cx, cy, r) {
                leaf(world, lx, ly);
            }
            for dx in (-r - 4..=r + 4).step_by(3) {
                let len = 8 + (rng.next_u8() % 22) as i32;
                for dy in 0..len {
                    leaf(world, cx + dx, cy + dy - 2);
                }
            }
        }
    }
}

/// Overgrown ruins on the surface: towers, arches and halls.
fn stamp_ruins(world: &mut World, p: &Profile, rng: &mut Rng, features: &mut Vec<Feature>) {
    let count = (p.width / 520).max(1);
    for i in 0..count {
        let slot = p.width / count;
        let x = i * slot + 70 + (rng.next_u64() % (slot - 140).max(1) as u64) as i32;
        if !clear_of_landmarks(p, x, 110) {
            continue;
        }
        let ground = p.surface_at(x);
        let (x0, x1, top) = match rng.next_u8() % 3 {
            0 => ruin_tower(world, rng, x, ground),
            1 => ruin_arch(world, rng, x, ground),
            _ => ruin_hall(world, rng, x, ground),
        };
        overgrow(world, rng, x0, x1, top, ground + 12);
        features.push(Feature {
            kind: FeatureKind::Ruin,
            x,
            y: ground,
        });
    }
}

fn ruin(world: &mut World, x: i32, y: i32) {
    if world.in_bounds(x, y) {
        world.set(x, y, Material::Ruin);
    }
}

fn ruin_tower(world: &mut World, rng: &mut Rng, x: i32, ground: i32) -> (i32, i32, i32) {
    let half = 18 + (rng.next_u8() % 8) as i32;
    let height = 60 + (rng.next_u8() % 60) as i32;
    for side in [-1, 1] {
        // Each wall crumbles at a different height.
        let wall_top = height - (rng.next_u8() % 30) as i32;
        for i in -12..wall_top {
            let ragged = if i > wall_top - 6 {
                (rng.next_u8() % 3) as i32
            } else {
                0
            };
            for t in 0..4 - ragged {
                ruin(world, x + side * (half - t), ground - i);
            }
            // Windows.
            if i > 10 && i % 30 > 14 && i % 30 < 24 {
                for t in 0..4 {
                    let wx = x + side * (half - t);
                    if world.in_bounds(wx, ground - i) {
                        world.set(wx, ground - i, Material::Empty);
                    }
                }
            }
        }
    }
    // Floors with holes.
    let mut level = 30;
    while level < height - 12 {
        let gap = x - half + 6 + (rng.next_u64() % (half as u64 * 2 - 24)) as i32;
        for fx in x - half..=x + half {
            if (fx - gap).abs() > 6 {
                for t in 0..3 {
                    ruin(world, fx, ground - level + t);
                }
            }
        }
        level += 30;
    }
    (x - half, x + half, ground - height)
}

fn ruin_arch(world: &mut World, rng: &mut Rng, x: i32, ground: i32) -> (i32, i32, i32) {
    let span = 26 + (rng.next_u8() % 14) as i32;
    let height = 44 + (rng.next_u8() % 20) as i32;
    for side in [-1, 1] {
        for i in -10..height {
            for t in 0..7 {
                ruin(world, x + side * (span + t), ground - i);
            }
        }
    }
    // A curved lintel, broken near one end.
    let broken = if rng.coin() { 1 } else { -1 };
    for dx in -span - 6..=span + 6 {
        let t = dx as f32 / (span + 6) as f32;
        let rise = ((1.0 - t * t).max(0.0).sqrt() * 14.0) as i32;
        if dx * broken > span / 2 && rng.chance(150) {
            continue;
        }
        for k in 0..6 {
            ruin(world, x + dx, ground - height - rise + k);
        }
    }
    (x - span - 7, x + span + 7, ground - height - 14)
}

fn ruin_hall(world: &mut World, rng: &mut Rng, x: i32, ground: i32) -> (i32, i32, i32) {
    let half = 50 + (rng.next_u8() % 24) as i32;
    let height = 34 + (rng.next_u8() % 8) as i32;
    for side in [-1, 1] {
        for i in -10..height {
            // Doorways at ground level on both ends.
            if (0..26).contains(&i) {
                continue;
            }
            for t in 0..4 {
                ruin(world, x + side * (half - t), ground - i);
            }
        }
    }
    for dx in -half..=half {
        // The roof has fallen in here and there.
        let hole = fbm(rng.next_u64() & 0xFF, dx as f32 / 9.0, 0.0, 1) > 0.78;
        if hole && rng.chance(200) {
            continue;
        }
        for k in 0..4 {
            ruin(world, x + dx, ground - height - k);
        }
        // Columns.
        if dx % 28 == 0 && dx.abs() < half - 6 {
            for i in 0..height {
                for t in -1..=1 {
                    ruin(world, x + dx + t, ground - i);
                }
            }
        }
    }
    (x - half, x + half, ground - height - 4)
}

/// Moss on top of ruin walls and strands of leaves hanging beneath them.
fn overgrow(world: &mut World, rng: &mut Rng, x0: i32, x1: i32, y0: i32, y1: i32) {
    for x in x0..=x1 {
        for y in y0..=y1 {
            if world.material(x, y) != Material::Ruin {
                continue;
            }
            if world.material(x, y - 1) == Material::Empty && rng.chance(150) {
                world.set(x, y - 1, Material::Moss);
                if rng.chance(70) {
                    leaf(world, x, y - 2);
                }
            }
            if world.material(x, y + 1) == Material::Empty && rng.chance(40) {
                for k in 1..3 + (rng.next_u8() % 12) as i32 {
                    if world.material(x, y + k) != Material::Empty {
                        break;
                    }
                    world.set(x, y + k, Material::Leaves);
                }
            }
        }
    }
}

fn seal_edges(world: &mut World) {
    let (w, h) = (world.width() as i32, world.height() as i32);
    for y in 0..h {
        for t in 0..4 {
            world.set(t, y, Material::Bedrock);
            world.set(w - 1 - t, y, Material::Bedrock);
        }
    }
    for x in 0..w {
        for t in 0..6 {
            world.set(x, h - 1 - t, Material::Bedrock);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small(seed: u64) -> Planet {
        generate_sized(2560, 1024, seed)
    }

    fn count(world: &World, mat: Material) -> usize {
        (0..world.height() as i32)
            .flat_map(|y| (0..world.width() as i32).map(move |x| (x, y)))
            .filter(|&(x, y)| world.material(x, y) == mat)
            .count()
    }

    #[test]
    fn generation_is_deterministic() {
        let (a, b) = (small(42), small(42));
        for cy in 0..a.world.chunks_y() {
            for cx in 0..a.world.chunks_x() {
                assert_eq!(a.world.encode_chunk(cx, cy), b.world.encode_chunk(cx, cy));
            }
        }
        assert_eq!(a.features, b.features);
        let c = small(43);
        assert_ne!(a.profile.surface, c.profile.surface);
    }

    #[test]
    fn has_a_flat_spawn_plateau_and_lakes() {
        let p = small(7);
        let s = &p.profile;
        let y = s.spawn_y();
        for x in s.spawn_x - PLATEAU_HALF..=s.spawn_x + PLATEAU_HALF {
            assert_eq!(s.surface_at(x), y, "plateau should be level at {x}");
            assert!(p.world.is_solid(x, y), "ground under the plateau at {x}");
            assert!(!p.world.is_solid(x, y - 1));
            // No caves right under the dome.
            assert!(p.world.is_solid(x, y + 40));
        }
        assert!(!s.lakes.is_empty());
        for lake in &s.lakes {
            let mid = (lake.x0 + lake.x1) / 2;
            assert_eq!(p.world.material(mid, lake.level + 3), Material::Water);
            assert!(count(&p.world, Material::Water) > 500);
        }
    }

    #[test]
    fn ores_get_richer_with_depth() {
        let p = small(11);
        let s = &p.profile;
        let mut by_band = std::collections::HashMap::new();
        for y in 0..s.height {
            for x in 0..s.width {
                let m = p.world.material(x, y);
                if m.is_ore() {
                    *by_band.entry((s.band(x, y), m)).or_insert(0usize) += 1;
                }
            }
        }
        let get = |b, m| by_band.get(&(b, m)).copied().unwrap_or(0);
        assert!(get(Band::Shallows, Material::IronOre) > 2000);
        assert!(get(Band::Shallows, Material::CopperOre) > 1000);
        assert!(get(Band::Shallows, Material::SiliconOre) > 500);
        assert_eq!(get(Band::Shallows, Material::XeniteOre), 0);
        assert!(get(Band::Deeps, Material::GoldOre) > 500);
        assert!(get(Band::Deeps, Material::TitaniumOre) > 500);
        assert!(get(Band::Abyss, Material::XeniteOre) > 500);
        assert!(count(&p.world, Material::AuroriumOre) > 50, "lairs hold aurorium");
    }

    #[test]
    fn caves_connect_to_the_surface() {
        let p = small(5);
        let s = &p.profile;
        let entrances: Vec<_> = p
            .features
            .iter()
            .filter(|f| f.kind == FeatureKind::Entrance)
            .collect();
        assert!(!entrances.is_empty());
        // Flood the open air from the sky and see how deep it reaches.
        let (w, h) = (s.width as usize, s.height as usize);
        let mut seen = vec![false; w * h];
        let mut stack = vec![(s.spawn_x, 5)];
        let mut deepest = 0;
        let mut open = 0usize;
        while let Some((x, y)) = stack.pop() {
            if !p.world.in_bounds(x, y) || seen[y as usize * w + x as usize] {
                continue;
            }
            if !p.world.material(x, y).is_open() {
                continue;
            }
            seen[y as usize * w + x as usize] = true;
            if y > s.surface_at(x) {
                open += 1;
            }
            deepest = deepest.max(y);
            stack.extend([(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]);
        }
        assert!(
            deepest > s.deeps_y,
            "walkable air should reach the Deeps without digging (reached {deepest}, deeps at {})",
            s.deeps_y
        );
        assert!(open > 20_000, "a real network, not one shaft: {open} cells");
    }

    #[test]
    fn features_are_placed() {
        let p = small(3);
        let kinds = |k| p.features.iter().filter(|f| f.kind == k).count();
        assert!(kinds(FeatureKind::Vault) >= 1);
        assert!(kinds(FeatureKind::Lair) >= 1);
        assert!(kinds(FeatureKind::Ruin) >= 1);
        assert!(count(&p.world, Material::Ruin) > 1000);
        assert!(count(&p.world, Material::Leaves) > 1000, "trees and overgrowth");
        // The world is walled.
        assert_eq!(p.world.material(0, 100), Material::Bedrock);
        assert_eq!(p.world.material(100, p.profile.height - 1), Material::Bedrock);
    }

    #[test]
    fn noise_in_range() {
        for i in 0..1000 {
            let v = fbm(9, i as f32 * 0.37, i as f32 * 0.11, 4);
            assert!((0.0..=1.0).contains(&v));
        }
    }
}
