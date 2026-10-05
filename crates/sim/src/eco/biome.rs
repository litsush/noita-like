//! Alien environments: terrain, palette and the affordances species
//! generation reads to decide what kinds of life make sense here.

use super::math::{Rgb, hsv, mix, scale};
use super::names;
use crate::material::{Kind, Material};
use crate::rng::Rng;
use crate::world::World;
use crate::worldgen::fbm;

pub const WIDTH: usize = 512;
pub const HEIGHT: usize = 320;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BiomeKind {
    Verdant,
    FungalHollow,
    VolcanicRift,
    Archipelago,
    AcidMarsh,
    DuneSea,
    SpireCanyon,
    SkyIsles,
}

impl BiomeKind {
    pub const ALL: [BiomeKind; 8] = [
        BiomeKind::Verdant,
        BiomeKind::FungalHollow,
        BiomeKind::VolcanicRift,
        BiomeKind::Archipelago,
        BiomeKind::AcidMarsh,
        BiomeKind::DuneSea,
        BiomeKind::SpireCanyon,
        BiomeKind::SkyIsles,
    ];

    pub fn title(self) -> &'static str {
        match self {
            BiomeKind::Verdant => "Verdant Highlands",
            BiomeKind::FungalHollow => "Fungal Hollow",
            BiomeKind::VolcanicRift => "Volcanic Rift",
            BiomeKind::Archipelago => "Tidal Archipelago",
            BiomeKind::AcidMarsh => "Acid Marsh",
            BiomeKind::DuneSea => "Dune Sea",
            BiomeKind::SpireCanyon => "Spire Canyon",
            BiomeKind::SkyIsles => "Sky Isles",
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            BiomeKind::Verdant => "Rolling hills under an open sky, strange groves and still ponds.",
            BiomeKind::FungalHollow => {
                "A sealed cavern lit only by what glows. Giant fungi hold up the dark."
            }
            BiomeKind::VolcanicRift => "Ash plains split by a lake of lava. Little survives the heat.",
            BiomeKind::Archipelago => "Islands scattered across a deep sea, kelp swaying below.",
            BiomeKind::AcidMarsh => "Flat mud flats pocked with stone-lined pools of acid.",
            BiomeKind::DuneSea => "Endless shifting sand around a single oasis.",
            BiomeKind::SpireCanyon => "Towering mesas over a river chasm, ledges and overhangs.",
            BiomeKind::SkyIsles => "Islands of rock drifting above a distant sea.",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WeatherKind {
    Clear,
    Rain,
    AshFall,
    AcidRain,
    Spores,
    Sandstorm,
}

impl WeatherKind {
    pub fn label(self) -> &'static str {
        match self {
            WeatherKind::Clear => "Clear",
            WeatherKind::Rain => "Rain",
            WeatherKind::AshFall => "Ash fall",
            WeatherKind::AcidRain => "Acid rain",
            WeatherKind::Spores => "Spore drift",
            WeatherKind::Sandstorm => "Sandstorm",
        }
    }
}

#[derive(Clone, Debug)]
pub struct Palette {
    /// Base colour per material.
    pub mats: [Rgb; Material::COUNT],
    pub sky_day: [Rgb; 2],
    pub sky_dusk: [Rgb; 2],
    pub sky_night: [Rgb; 2],
    /// Cave wall behind dug-out terrain.
    pub backdrop: Rgb,
    /// Distant silhouettes on the horizon.
    pub hills: Rgb,
    pub sun: Rgb,
    pub moons: Vec<(Rgb, f32)>,
}

impl Palette {
    pub fn mat(&self, m: Material) -> Rgb {
        self.mats[m as usize]
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Tree {
    pub x: i32,
    pub ground: i32,
    pub height: i32,
}

#[derive(Clone, Debug)]
pub struct Biome {
    pub kind: BiomeKind,
    pub name: String,
    pub palette: Palette,
    /// Whether the scene has a sky (false for sealed caverns).
    pub open_sky: bool,
    /// Ambient light deep underground.
    pub cave_ambient: f32,
    /// Strength of gravity relative to the home world.
    pub gravity: f32,
    pub day_length: f32,
    /// The liquid that defines this place.
    pub liquid: Material,
    pub weathers: Vec<WeatherKind>,
    pub trees: Vec<Tree>,
    /// Top of the original terrain per column (HEIGHT where there is none).
    pub surface: Vec<i32>,
    /// Fractions of the scene, measured after generation; species read these.
    pub water: f32,
    pub lava: f32,
    pub acid: f32,
    pub soil: f32,
    pub open_air: f32,
    pub darkness: f32,
    pub tree_cover: f32,
    /// Liquid cells above which rain stops.
    pub liquid_cap: usize,
    /// Distant horizon heights, for the backdrop.
    pub horizon: Vec<i32>,
}

impl Biome {
    pub fn describe_gravity(&self) -> &'static str {
        match self.gravity {
            g if g < 0.75 => "low gravity",
            g if g > 1.2 => "heavy gravity",
            _ => "normal gravity",
        }
    }
}

// ---------------------------------------------------------------------------
// Palettes

fn alien_hue(rng: &mut Rng, options: &[f32]) -> f32 {
    rng.pick(options) + rng.range(-12.0, 12.0)
}

fn make_palette(rng: &mut Rng, kind: BiomeKind) -> Palette {
    let mut mats = [[0u8; 3]; Material::COUNT];
    for (i, m) in mats.iter_mut().enumerate() {
        *m = Material::from_u8(i as u8).props().color;
    }
    let leaf_h = alien_hue(rng, &[150.0, 280.0, 320.0, 35.0, 190.0, 95.0, 0.0, 60.0]);
    let soil_h = alien_hue(rng, &[20.0, 300.0, 250.0, 10.0, 40.0, 340.0]);
    let stone_h = rng.range(0.0, 360.0);
    let water_h = alien_hue(rng, &[200.0, 170.0, 300.0, 330.0, 260.0, 140.0]);
    let sky_h = alien_hue(rng, &[200.0, 140.0, 30.0, 280.0, 320.0, 60.0, 180.0]);

    let mut leaf = hsv(leaf_h, rng.range(0.5, 0.75), rng.range(0.5, 0.7));
    let mut soil = hsv(soil_h, rng.range(0.3, 0.5), rng.range(0.3, 0.42));
    let mut stone = hsv(stone_h, rng.range(0.08, 0.25), rng.range(0.36, 0.5));
    let mut sand = hsv(
        alien_hue(rng, &[45.0, 30.0, 330.0, 15.0, 200.0]),
        rng.range(0.3, 0.5),
        rng.range(0.7, 0.85),
    );
    let mut wood = hsv(
        soil_h + rng.range(-30.0, 30.0),
        rng.range(0.35, 0.55),
        rng.range(0.3, 0.42),
    );
    let mut water = hsv(water_h, rng.range(0.5, 0.7), rng.range(0.55, 0.75));
    let lava = hsv(alien_hue(rng, &[18.0, 18.0, 5.0, 285.0, 190.0]), 0.9, 0.95);
    let acid = hsv(alien_hue(rng, &[95.0, 70.0, 160.0, 300.0]), 0.75, 0.85);
    let mut sky_day = [
        hsv(sky_h, rng.range(0.35, 0.6), rng.range(0.55, 0.75)),
        hsv(
            sky_h + rng.range(-40.0, 40.0),
            rng.range(0.2, 0.4),
            rng.range(0.8, 0.95),
        ),
    ];
    let mut backdrop_dim = 0.35;

    match kind {
        BiomeKind::FungalHollow => {
            leaf = hsv(alien_hue(rng, &[290.0, 180.0, 330.0, 50.0]), 0.6, 0.6);
            stone = hsv(stone_h, 0.2, 0.28);
            soil = hsv(soil_h, 0.35, 0.25);
            wood = hsv(alien_hue(rng, &[40.0, 300.0, 200.0]), 0.2, 0.7);
            backdrop_dim = 0.5;
        }
        BiomeKind::VolcanicRift => {
            stone = hsv(stone_h, 0.1, 0.18);
            soil = hsv(alien_hue(rng, &[15.0, 350.0, 30.0]), 0.4, 0.28);
            leaf = hsv(alien_hue(rng, &[30.0, 50.0, 0.0, 280.0]), 0.6, 0.55);
            sky_day = [
                hsv(alien_hue(rng, &[10.0, 25.0, 340.0]), 0.6, 0.35),
                hsv(alien_hue(rng, &[30.0, 40.0]), 0.55, 0.6),
            ];
        }
        BiomeKind::DuneSea => {
            sand = hsv(
                alien_hue(rng, &[35.0, 15.0, 330.0, 50.0, 280.0]),
                rng.range(0.35, 0.55),
                0.8,
            );
            sky_day[1] = mix(sky_day[1], sand, 0.4);
        }
        BiomeKind::AcidMarsh => {
            soil = hsv(alien_hue(rng, &[80.0, 60.0, 280.0, 30.0]), 0.3, 0.3);
            sky_day = [
                hsv(alien_hue(rng, &[90.0, 70.0, 180.0]), 0.35, 0.5),
                hsv(alien_hue(rng, &[60.0, 90.0]), 0.25, 0.75),
            ];
        }
        BiomeKind::Archipelago => {
            water = hsv(water_h, 0.65, 0.6);
            sand = hsv(alien_hue(rng, &[45.0, 320.0, 180.0, 10.0]), 0.3, 0.82);
        }
        _ => {}
    }

    mats[Material::Grass as usize] = leaf;
    mats[Material::Leaf as usize] = mix(leaf, hsv(rng.range(0.0, 360.0), 0.6, 0.7), 0.25);
    mats[Material::Dirt as usize] = soil;
    mats[Material::Stone as usize] = stone;
    mats[Material::Sand as usize] = sand;
    mats[Material::Wood as usize] = wood;
    mats[Material::Water as usize] = water;
    mats[Material::Lava as usize] = lava;
    mats[Material::Acid as usize] = acid;
    mats[Material::Nest as usize] = mix(soil, [150, 120, 90], 0.35);
    mats[Material::Ash as usize] = mix(stone, [90, 88, 86], 0.6);

    let sky_dusk = [
        mix(sky_day[0], hsv(sky_h + 150.0, 0.6, 0.35), 0.6),
        mix(sky_day[1], hsv(rng.range(0.0, 50.0), 0.7, 0.85), 0.6),
    ];
    let sky_night = [
        scale(hsv(sky_h + 40.0, 0.5, 0.25), 0.3),
        scale(hsv(sky_h + 20.0, 0.5, 0.35), 0.45),
    ];
    let n_moons = rng.int(0, 3);
    let moons = (0..n_moons)
        .map(|_| {
            (
                hsv(rng.range(0.0, 360.0), rng.range(0.1, 0.5), 0.9),
                rng.range(3.0, 11.0),
            )
        })
        .collect();
    Palette {
        mats,
        sky_day,
        sky_dusk,
        sky_night,
        backdrop: scale(mix(soil, stone, 0.5), backdrop_dim),
        hills: mix(sky_day[0], stone, 0.5),
        sun: hsv(rng.range(0.0, 70.0), rng.range(0.1, 0.5), 1.0),
        moons,
    }
}

// ---------------------------------------------------------------------------
// Terrain helpers

fn fill_column(world: &mut World, x: i32, from: i32, to: i32, mat: Material) {
    for y in from.max(0)..to.min(HEIGHT as i32) {
        world.set(x, y, mat);
    }
}

fn disc(world: &mut World, cx: i32, cy: i32, r: i32, mat: Material, only_empty: bool) {
    for y in cy - r..=cy + r {
        for x in cx - r..=cx + r {
            if (x - cx).pow(2) + (y - cy).pow(2) <= r * r && world.in_bounds(x, y) {
                if only_empty && world.material(x, y) != Material::Empty {
                    continue;
                }
                world.set(x, y, mat);
            }
        }
    }
}

fn ellipse(world: &mut World, cx: i32, cy: i32, rx: i32, ry: i32, mat: Material, only_empty: bool) {
    for y in cy - ry..=cy + ry {
        for x in cx - rx..=cx + rx {
            let (dx, dy) = ((x - cx) as f32 / rx as f32, (y - cy) as f32 / ry as f32);
            if dx * dx + dy * dy <= 1.0 && world.in_bounds(x, y) {
                if only_empty && world.material(x, y) != Material::Empty {
                    continue;
                }
                world.set(x, y, mat);
            }
        }
    }
}

/// Flood-fills liquid into empty cells reachable from (x, y) without
/// rising above `level`. Used to fill basins.
fn flood(world: &mut World, x: i32, y: i32, level: i32, mat: Material, max: usize) {
    let mut stack = vec![(x, y)];
    let mut n = 0;
    while let Some((x, y)) = stack.pop() {
        if n >= max || y < level || !world.in_bounds(x, y) || world.material(x, y) != Material::Empty {
            continue;
        }
        world.set(x, y, mat);
        n += 1;
        stack.extend([(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)]);
    }
}

#[derive(Clone, Copy)]
enum TreeStyle {
    Umbrella,
    Ball,
    Spire,
    Weeping,
    Mushroom,
    Coral,
}

fn grow_tree(world: &mut World, rng: &mut Rng, x: i32, ground: i32, style: TreeStyle, scale_f: f32) -> Tree {
    let height = ((22.0 + rng.range(0.0, 22.0)) * scale_f) as i32;
    let trunk_w = if scale_f > 1.3 { 2 } else { 1 };
    let top = ground - height;
    // Slightly wandering trunk.
    let mut tx = x as f32;
    let lean = rng.range(-0.25, 0.25);
    for y in (top..ground + 2).rev() {
        for dx in -trunk_w..=trunk_w {
            world.set(tx as i32 + dx, y, Material::Wood);
        }
        tx += lean * rng.range(0.0, 1.0);
    }
    let (cx, cy) = (tx as i32, top);
    let leaf = Material::Leaf;
    match style {
        TreeStyle::Umbrella => {
            let rx = (10.0 * scale_f + rng.range(0.0, 6.0)) as i32;
            ellipse(world, cx, cy, rx, (rx / 3).max(2), leaf, true);
        }
        TreeStyle::Ball => {
            let r = (7.0 * scale_f + rng.range(0.0, 4.0)) as i32;
            for _ in 0..4 {
                let ox = rng.int(-r / 2, r / 2);
                let oy = rng.int(-r / 2, r / 3);
                disc(world, cx + ox, cy + oy, r - 2, leaf, true);
            }
        }
        TreeStyle::Spire => {
            let tiers = rng.int(3, 5);
            for t in 0..tiers {
                let y = cy + t * 6;
                let w = 3 + t * 3;
                ellipse(world, cx, y, w, 2, leaf, true);
            }
        }
        TreeStyle::Weeping => {
            let r = (8.0 * scale_f) as i32 + 2;
            ellipse(world, cx, cy, r, r / 2 + 1, leaf, true);
            for i in 0..(r * 2) {
                if rng.chance(110) {
                    let sx = cx - r + i;
                    let len = rng.int(4, 12);
                    for y in cy..cy + len {
                        if world.material(sx, y) == Material::Empty {
                            world.set(sx, y, leaf);
                        }
                    }
                }
            }
        }
        TreeStyle::Mushroom => {
            let r = (9.0 * scale_f + rng.range(0.0, 8.0)) as i32;
            for y in cy - r / 2..=cy + 1 {
                for xx in cx - r..=cx + r {
                    let dx = (xx - cx) as f32 / r as f32;
                    let dome = (1.0 - dx * dx).max(0.0).sqrt() * (r as f32 / 2.0);
                    if (cy - y) as f32 <= dome && world.material(xx, y) == Material::Empty {
                        world.set(xx, y, leaf);
                    }
                }
            }
        }
        TreeStyle::Coral => {
            for _ in 0..rng.int(3, 6) {
                let mut p = (cx as f32, cy as f32 + 6.0);
                let dir = rng.range(-1.2, 1.2);
                for s in 0..rng.int(6, 14) {
                    p.0 += dir * 0.8 + rng.range(-0.4, 0.4);
                    p.1 -= 1.0;
                    world.set(p.0 as i32, p.1 as i32, Material::Wood);
                    if s > 3 && rng.chance(80) {
                        disc(world, p.0 as i32, p.1 as i32, 2, leaf, true);
                    }
                }
                disc(world, p.0 as i32, p.1 as i32, 3, leaf, true);
            }
        }
    }
    Tree {
        x: cx,
        ground,
        height,
    }
}

fn pick_style(rng: &mut Rng, kind: BiomeKind) -> TreeStyle {
    use TreeStyle::*;
    let options: &[TreeStyle] = match kind {
        BiomeKind::FungalHollow => &[Mushroom, Mushroom, Coral],
        BiomeKind::VolcanicRift => &[Coral, Spire],
        BiomeKind::DuneSea => &[Umbrella, Spire],
        BiomeKind::AcidMarsh => &[Weeping, Coral, Ball],
        _ => &[Umbrella, Ball, Spire, Weeping, Mushroom, Coral],
    };
    *rng.pick(options)
}

fn surface_of(world: &World, x: i32) -> i32 {
    (0..HEIGHT as i32)
        .find(|&y| world.material(x, y).blocks_creature(false) || world.material(x, y).kind() == Kind::Liquid)
        .unwrap_or(HEIGHT as i32)
}

/// Grows `count` trees on flat open ground.
fn plant_trees(world: &mut World, rng: &mut Rng, kind: BiomeKind, count: usize, scale_f: f32) -> Vec<Tree> {
    let style = pick_style(rng, kind);
    let second = pick_style(rng, kind);
    let mut trees: Vec<Tree> = Vec::new();
    for _ in 0..count * 8 {
        if trees.len() >= count {
            break;
        }
        let x = rng.int(12, WIDTH as i32 - 13);
        if trees.iter().any(|t| (t.x - x).abs() < 22) {
            continue;
        }
        let g = surface_of(world, x);
        if g >= HEIGHT as i32 - 4 || g < 30 {
            continue;
        }
        let m = world.material(x, g);
        if !matches!(m, Material::Grass | Material::Dirt | Material::Sand) {
            continue;
        }
        if (surface_of(world, x - 3) - g).abs() > 3 || (surface_of(world, x + 3) - g).abs() > 3 {
            continue;
        }
        let s = if rng.coin() { style } else { second };
        let sc = scale_f * rng.range(0.8, 1.25);
        trees.push(grow_tree(world, rng, x, g, s, sc));
    }
    trees
}

/// Hollows out noise caves below `min_depth` of the surface.
fn carve_caves(world: &mut World, seed: u64, surface: &[i32], min_depth: i32, threshold: f32) {
    for x in 0..WIDTH as i32 {
        for y in surface[x as usize] + min_depth..HEIGHT as i32 - 3 {
            let n = fbm(seed ^ 0x77, x as f32 / 55.0, y as f32 / 35.0, 4);
            if n > threshold {
                world.set(x, y, Material::Empty);
            }
        }
    }
}

/// A two-layer column: `top` material for `top_depth` cells, then `mid`
/// until `mid_depth`, then stone.
fn layered(world: &mut World, seed: u64, x: i32, surface: i32, top: Material, mid: Material, mid_depth: i32) {
    for y in surface.max(0)..HEIGHT as i32 {
        let d = y - surface;
        let wobble = (fbm(seed ^ 9, x as f32 / 20.0, y as f32 / 20.0, 2) * 10.0) as i32;
        let m = if d < 2 {
            top
        } else if d < mid_depth + wobble {
            mid
        } else {
            Material::Stone
        };
        world.set(x, y, m);
    }
}

// ---------------------------------------------------------------------------
// Generators

struct Gen {
    world: World,
    surface: Vec<i32>,
    trees: Vec<Tree>,
    open_sky: bool,
}

fn gen_verdant(rng: &mut Rng, seed: u64) -> Gen {
    let mut world = World::new(WIDTH, HEIGHT, seed);
    let h = HEIGHT as f32;
    let surface: Vec<i32> = (0..WIDTH)
        .map(|x| (h * 0.42 + fbm(seed, x as f32 / 120.0, 0.0, 4) * 90.0 - 30.0) as i32)
        .collect();
    for x in 0..WIDTH as i32 {
        layered(
            &mut world,
            seed,
            x,
            surface[x as usize],
            Material::Grass,
            Material::Dirt,
            30,
        );
    }
    carve_caves(&mut world, seed, &surface, 14, 0.64);
    // Ponds in the lowest dips.
    for _ in 0..rng.int(1, 3) {
        let x0 = rng.int(40, WIDTH as i32 - 41);
        let lowest = (x0 - 35..x0 + 35)
            .max_by_key(|&x| surface[x as usize])
            .unwrap_or(x0);
        let g = surface[lowest as usize];
        let (rx, ry) = (rng.int(14, 26), rng.int(6, 12));
        ellipse(&mut world, lowest, g, rx, ry, Material::Empty, false);
        flood(&mut world, lowest, g + ry - 1, g + 1, Material::Water, 4000);
    }
    let trees = {
        let n = rng.int(5, 9) as usize;
        plant_trees(&mut world, rng, BiomeKind::Verdant, n, 1.0)
    };
    Gen {
        world,
        surface,
        trees,
        open_sky: true,
    }
}

fn gen_fungal(rng: &mut Rng, seed: u64) -> Gen {
    let mut world = World::new(WIDTH, HEIGHT, seed);
    let h = HEIGHT as f32;
    let ceiling: Vec<i32> = (0..WIDTH)
        .map(|x| (h * 0.12 + fbm(seed ^ 1, x as f32 / 70.0, 0.0, 3) * 50.0) as i32)
        .collect();
    let floor: Vec<i32> = (0..WIDTH)
        .map(|x| (h * 0.62 + fbm(seed ^ 2, x as f32 / 90.0, 0.0, 4) * 70.0) as i32)
        .collect();
    for x in 0..WIDTH as i32 {
        let (c, f) = (ceiling[x as usize], floor[x as usize]);
        for y in 0..HEIGHT as i32 {
            let m = if y < c - 6 {
                Material::Stone
            } else if y < c {
                Material::Dirt
            } else if y < f {
                Material::Empty
            } else if y < f + 2 {
                Material::Grass
            } else if y < f + 22 {
                Material::Dirt
            } else {
                Material::Stone
            };
            if m != Material::Empty {
                world.set(x, y, m);
            }
        }
    }
    // Side walls so the hollow is sealed.
    for x in 0..6 {
        fill_column(&mut world, x, 0, HEIGHT as i32, Material::Stone);
        fill_column(
            &mut world,
            WIDTH as i32 - 1 - x,
            0,
            HEIGHT as i32,
            Material::Stone,
        );
    }
    // Stalactites and pillars.
    for _ in 0..rng.int(6, 12) {
        let x = rng.int(10, WIDTH as i32 - 11);
        let c = ceiling[x as usize];
        let len = rng.int(8, 30);
        for d in 0..len {
            let w = ((len - d) / 5).max(0);
            for dx in -w..=w {
                world.set(x + dx, c + d, Material::Stone);
            }
        }
    }
    carve_caves(&mut world, seed, &floor, 8, 0.66);
    // Underground pool.
    let x0 = rng.int(60, WIDTH as i32 - 61);
    let lowest = (x0 - 50..x0 + 50)
        .max_by_key(|&x| floor[x as usize])
        .unwrap_or(x0);
    let g = floor[lowest as usize];
    ellipse(
        &mut world,
        lowest,
        g,
        rng.int(18, 30),
        rng.int(7, 12),
        Material::Empty,
        false,
    );
    flood(&mut world, lowest, g + 6, g + 1, Material::Water, 5000);
    let mut trees = {
        let n = rng.int(5, 8) as usize;
        plant_trees(&mut world, rng, BiomeKind::FungalHollow, n, 1.4)
    };
    // Mushrooms can't poke through the ceiling.
    trees.retain(|t| t.ground - t.height > ceiling[t.x as usize]);
    Gen {
        world,
        surface: ceiling,
        trees,
        open_sky: false,
    }
}

fn gen_volcanic(rng: &mut Rng, seed: u64) -> Gen {
    let mut world = World::new(WIDTH, HEIGHT, seed);
    let h = HEIGHT as f32;
    let rift = rng.range(0.3, 0.7) * WIDTH as f32;
    let surface: Vec<i32> = (0..WIDTH)
        .map(|x| {
            let ridged = 1.0 - (fbm(seed, x as f32 / 50.0, 0.0, 4) * 2.0 - 1.0).abs();
            let valley = (-(x as f32 - rift).powi(2) / (2.0 * 45.0f32.powi(2))).exp() * 70.0;
            (h * 0.38 + ridged * 45.0 + valley) as i32
        })
        .collect();
    for x in 0..WIDTH as i32 {
        layered(
            &mut world,
            seed,
            x,
            surface[x as usize],
            Material::Ash,
            Material::Dirt,
            10,
        );
    }
    carve_caves(&mut world, seed, &surface, 18, 0.66);
    // Lava lake in the rift and pockets in the deep caves.
    let g = surface[rift as usize];
    ellipse(&mut world, rift as i32, g, 40, 16, Material::Empty, false);
    flood(&mut world, rift as i32, g + 10, g - 2, Material::Lava, 9000);
    for _ in 0..rng.int(2, 5) {
        let x = rng.int(20, WIDTH as i32 - 21);
        let y = rng.int(HEIGHT as i32 * 3 / 4, HEIGHT as i32 - 12);
        if world.material(x, y) == Material::Empty {
            flood(&mut world, x, y, y - 4, Material::Lava, 600);
        }
    }
    let trees = {
        let n = rng.int(2, 4) as usize;
        plant_trees(&mut world, rng, BiomeKind::VolcanicRift, n, 0.9)
    };
    Gen {
        world,
        surface,
        trees,
        open_sky: true,
    }
}

fn gen_archipelago(rng: &mut Rng, seed: u64) -> Gen {
    let mut world = World::new(WIDTH, HEIGHT, seed);
    let h = HEIGHT as f32;
    let sea = (h * 0.5) as i32;
    let n_islands = rng.int(2, 4);
    let centres: Vec<f32> = (0..n_islands)
        .map(|i| (i as f32 + 0.5 + rng.range(-0.2, 0.2)) * WIDTH as f32 / n_islands as f32)
        .collect();
    let widths: Vec<f32> = (0..n_islands).map(|_| rng.range(30.0, 55.0)).collect();
    let surface: Vec<i32> = (0..WIDTH)
        .map(|x| {
            let bed = h * 0.82 + fbm(seed ^ 3, x as f32 / 60.0, 0.0, 3) * 30.0;
            let island = centres
                .iter()
                .zip(&widths)
                .map(|(c, w)| (1.0 - ((x as f32 - c) / w).powi(2)).max(0.0))
                .fold(0.0f32, f32::max);
            (bed - island * (h * 0.42 + fbm(seed ^ 4, x as f32 / 25.0, 0.0, 2) * 20.0)) as i32
        })
        .collect();
    for x in 0..WIDTH as i32 {
        let s = surface[x as usize];
        let top = if s < sea - 2 {
            Material::Grass
        } else {
            Material::Sand
        };
        layered(
            &mut world,
            seed,
            x,
            s,
            top,
            if s < sea - 3 {
                Material::Dirt
            } else {
                Material::Sand
            },
            16,
        );
    }
    carve_caves(&mut world, seed, &surface, 16, 0.7);
    for x in 0..WIDTH as i32 {
        for y in sea..HEIGHT as i32 {
            if world.material(x, y) == Material::Empty && y < surface[x as usize] + 1 {
                world.set(x, y, Material::Water);
            }
        }
    }
    let trees = {
        let n = rng.int(3, 6) as usize;
        plant_trees(&mut world, rng, BiomeKind::Archipelago, n, 0.9)
    };
    Gen {
        world,
        surface,
        trees,
        open_sky: true,
    }
}

fn gen_acid_marsh(rng: &mut Rng, seed: u64) -> Gen {
    let mut world = World::new(WIDTH, HEIGHT, seed);
    let h = HEIGHT as f32;
    let surface: Vec<i32> = (0..WIDTH)
        .map(|x| (h * 0.55 + fbm(seed, x as f32 / 80.0, 0.0, 3) * 30.0) as i32)
        .collect();
    for x in 0..WIDTH as i32 {
        layered(
            &mut world,
            seed,
            x,
            surface[x as usize],
            Material::Grass,
            Material::Dirt,
            40,
        );
    }
    carve_caves(&mut world, seed, &surface, 22, 0.68);
    for _ in 0..rng.int(3, 5) {
        let x = rng.int(30, WIDTH as i32 - 31);
        let g = surface[x as usize];
        let (rx, ry) = (rng.int(10, 22), rng.int(5, 9));
        // Stone lining first so the acid has something to sit in.
        ellipse(&mut world, x, g, rx + 3, ry + 3, Material::Stone, false);
        ellipse(&mut world, x, g, rx, ry, Material::Empty, false);
        for xx in x - rx - 3..=x + rx + 3 {
            for yy in g - ry - 4..g {
                if world.material(xx, yy) == Material::Stone {
                    world.set(xx, yy, Material::Empty);
                }
            }
        }
        flood(&mut world, x, g + ry - 1, g + 1, Material::Acid, 3000);
    }
    // A couple of fresh water pools too.
    for _ in 0..rng.int(0, 2) {
        let x = rng.int(30, WIDTH as i32 - 31);
        let g = surface[x as usize];
        if world.material(x, g) != Material::Grass {
            continue;
        }
        ellipse(
            &mut world,
            x,
            g,
            rng.int(8, 14),
            rng.int(4, 6),
            Material::Empty,
            false,
        );
        flood(&mut world, x, g + 3, g + 1, Material::Water, 1500);
    }
    let trees = {
        let n = rng.int(3, 6) as usize;
        plant_trees(&mut world, rng, BiomeKind::AcidMarsh, n, 0.8)
    };
    Gen {
        world,
        surface,
        trees,
        open_sky: true,
    }
}

fn gen_dunes(rng: &mut Rng, seed: u64) -> Gen {
    let mut world = World::new(WIDTH, HEIGHT, seed);
    let h = HEIGHT as f32;
    let phase = rng.range(0.0, 6.0);
    let surface: Vec<i32> = (0..WIDTH)
        .map(|x| {
            let dune = ((x as f32 / 70.0 + phase).sin() * 0.5 + 0.5) * 35.0;
            (h * 0.45 + dune + fbm(seed, x as f32 / 140.0, 0.0, 3) * 40.0) as i32
        })
        .collect();
    for x in 0..WIDTH as i32 {
        layered(
            &mut world,
            seed,
            x,
            surface[x as usize],
            Material::Sand,
            Material::Sand,
            55,
        );
    }
    carve_caves(&mut world, seed, &surface, 50, 0.67);
    // Rock arches.
    for _ in 0..rng.int(1, 3) {
        let x = rng.int(40, WIDTH as i32 - 41);
        let g = surface[x as usize];
        let (rx, ry) = (rng.int(16, 28), rng.int(14, 24));
        for yy in g - ry..g {
            for xx in x - rx..=x + rx {
                let d = ((xx - x) as f32 / rx as f32).powi(2) + ((yy - g) as f32 / ry as f32).powi(2);
                let inner =
                    ((xx - x) as f32 / (rx - 7) as f32).powi(2) + ((yy - g) as f32 / (ry - 7) as f32).powi(2);
                if d <= 1.0 && inner > 1.0 {
                    world.set(xx, yy, Material::Stone);
                }
            }
        }
    }
    // The oasis.
    let ox = rng.int(80, WIDTH as i32 - 81);
    let g = surface[ox as usize];
    ellipse(&mut world, ox, g + 2, 24, 9, Material::Stone, false);
    ellipse(&mut world, ox, g + 2, 20, 7, Material::Empty, false);
    for xx in ox - 26..=ox + 26 {
        for yy in g - 10..g + 1 {
            if world.material(xx, yy) == Material::Stone {
                world.set(xx, yy, Material::Empty);
            }
        }
    }
    flood(&mut world, ox, g + 6, g + 2, Material::Water, 2000);
    for dx in [-30, -24, 24, 30] {
        let x = ox + dx;
        let gg = surface_of(&world, x);
        world.set(x, gg, Material::Grass);
    }
    let mut trees = Vec::new();
    for dx in [-28, 28] {
        let x = ox + dx;
        let gg = surface_of(&world, x);
        trees.push(grow_tree(&mut world, rng, x, gg, TreeStyle::Umbrella, 0.9));
    }
    Gen {
        world,
        surface,
        trees,
        open_sky: true,
    }
}

fn gen_canyon(rng: &mut Rng, seed: u64) -> Gen {
    let mut world = World::new(WIDTH, HEIGHT, seed);
    let h = HEIGHT as f32;
    let n_mesas = rng.int(3, 5);
    let surface: Vec<i32> = (0..WIDTH)
        .map(|x| {
            let t = x as f32 / WIDTH as f32 * n_mesas as f32;
            let cell = t.fract();
            // Flat tops with steep sides.
            let mesa = ((cell - 0.5).abs() * 2.0).powf(4.0);
            let top = h * 0.18 + fbm(seed ^ (t as u64), t, 0.0, 2) * 40.0;
            let bottom = h * 0.85;
            (top + (bottom - top) * mesa + fbm(seed, x as f32 / 12.0, 0.0, 2) * 6.0) as i32
        })
        .collect();
    for x in 0..WIDTH as i32 {
        layered(
            &mut world,
            seed,
            x,
            surface[x as usize],
            Material::Grass,
            Material::Dirt,
            8,
        );
    }
    // Ledges on the cliff faces.
    for _ in 0..rng.int(8, 16) {
        let x = rng.int(10, WIDTH as i32 - 11);
        let y = rng.int(surface[x as usize] + 10, HEIGHT as i32 - 30);
        let dir = if rng.coin() { 1 } else { -1 };
        let len = rng.int(6, 16);
        for i in 0..len {
            for dy in 0..3 {
                let xx = x + i * dir;
                if world.in_bounds(xx, y + dy) && world.material(xx, y + dy) == Material::Empty {
                    world.set(
                        xx,
                        y + dy,
                        if dy == 0 { Material::Grass } else { Material::Stone },
                    );
                }
            }
        }
    }
    carve_caves(&mut world, seed, &surface, 12, 0.7);
    // River along the chasm floors.
    let river = (h * 0.83) as i32;
    for x in 0..WIDTH as i32 {
        for y in river..HEIGHT as i32 {
            if world.material(x, y) == Material::Empty && y > surface[x as usize] - 6 {
                world.set(x, y, Material::Water);
            }
        }
    }
    let trees = {
        let n = rng.int(3, 6) as usize;
        plant_trees(&mut world, rng, BiomeKind::SpireCanyon, n, 0.8)
    };
    Gen {
        world,
        surface,
        trees,
        open_sky: true,
    }
}

fn gen_sky_isles(rng: &mut Rng, seed: u64) -> Gen {
    let mut world = World::new(WIDTH, HEIGHT, seed);
    let h = HEIGHT as i32;
    let sea_mat = if rng.prob(0.3) {
        Material::Lava
    } else {
        Material::Water
    };
    let sea = h - rng.int(22, 34);
    for x in 0..WIDTH as i32 {
        let floor = h - 8 + (fbm(seed, x as f32 / 30.0, 0.0, 2) * 6.0) as i32;
        fill_column(&mut world, x, floor, h, Material::Stone);
        for y in sea..floor {
            world.set(x, y, sea_mat);
        }
    }
    // Sea stacks and beaches, so whatever falls has somewhere to climb out.
    let stacks = rng.int(2, 3);
    for i in 0..=stacks {
        let cx = (i as f32 / stacks as f32 * (WIDTH as f32 - 1.0)) as i32 + rng.int(-10, 10);
        let half = rng.int(8, 16);
        let top = sea - rng.int(4, 12);
        for x in (cx - half).max(0)..(cx + half).min(WIDTH as i32) {
            let edge = ((x - cx).abs() as f32 / half as f32).powi(2);
            let t = top + (edge * 8.0) as i32;
            for y in t..h {
                let m = if y < t + 2 {
                    Material::Grass
                } else {
                    Material::Stone
                };
                world.set(x, y, m);
            }
        }
    }
    let mut trees = Vec::new();
    let n = rng.int(6, 9);
    for i in 0..n {
        let cx = ((i as f32 + 0.5) / n as f32 * WIDTH as f32 + rng.range(-15.0, 15.0)) as i32;
        let cy = rng.int(70, sea - 50);
        let rx = rng.int(18, 34);
        let depth = rng.int(14, 30);
        for x in cx - rx..=cx + rx {
            let t = (x - cx) as f32 / rx as f32;
            let top_off = (fbm(seed ^ i as u64, x as f32 / 10.0, 0.0, 2) * 5.0) as i32;
            let top = cy - 4 + top_off;
            let bottom = cy + ((1.0 - t * t).max(0.0).sqrt() * depth as f32) as i32;
            for y in top..bottom {
                let d = y - top;
                let m = if d < 2 {
                    Material::Grass
                } else if d < 7 {
                    Material::Dirt
                } else {
                    Material::Stone
                };
                world.set(x, y, m);
            }
        }
        if rng.prob(0.75) {
            let tx = cx + rng.int(-rx / 2, rx / 2);
            let g = surface_of(&world, tx);
            let style = pick_style(rng, BiomeKind::SkyIsles);
            trees.push(grow_tree(&mut world, rng, tx, g, style, 0.75));
        }
        if rng.prob(0.3) {
            // A little pond on top.
            let px = cx + rng.int(-rx / 3, rx / 3);
            let g = surface_of(&world, px);
            ellipse(&mut world, px, g, 6, 3, Material::Empty, false);
            flood(&mut world, px, g + 2, g, Material::Water, 200);
        }
    }
    // Open sky all the way down to the sea; the islands float in it.
    let surface = vec![sea; WIDTH];
    Gen {
        world,
        surface,
        trees,
        open_sky: true,
    }
}

// ---------------------------------------------------------------------------

/// Generates a biome of the given kind (or a random one) for `seed`.
pub fn generate(seed: u64, kind: Option<BiomeKind>) -> (Biome, World) {
    let mut rng = Rng::new(seed ^ 0xB10E);
    let kind = kind.unwrap_or_else(|| *rng.pick(&BiomeKind::ALL));
    let palette = make_palette(&mut rng, kind);
    let g = match kind {
        BiomeKind::Verdant => gen_verdant(&mut rng, seed),
        BiomeKind::FungalHollow => gen_fungal(&mut rng, seed),
        BiomeKind::VolcanicRift => gen_volcanic(&mut rng, seed),
        BiomeKind::Archipelago => gen_archipelago(&mut rng, seed),
        BiomeKind::AcidMarsh => gen_acid_marsh(&mut rng, seed),
        BiomeKind::DuneSea => gen_dunes(&mut rng, seed),
        BiomeKind::SpireCanyon => gen_canyon(&mut rng, seed),
        BiomeKind::SkyIsles => gen_sky_isles(&mut rng, seed),
    };
    let world = g.world;

    // Measure what was made.
    let total = (WIDTH * HEIGHT) as f32;
    let mut counts = [0usize; Material::COUNT];
    for c in world.cells() {
        counts[c.mat as usize] += 1;
    }
    let frac = |m: Material| counts[m as usize] as f32 / total;
    let soil = frac(Material::Dirt) + frac(Material::Sand) + frac(Material::Grass) * 0.5;
    let open_air = frac(Material::Empty);
    let water = frac(Material::Water);
    let lava = frac(Material::Lava);
    let acid = frac(Material::Acid);
    let liquid = if lava > water && lava > acid {
        Material::Lava
    } else if acid > water {
        Material::Acid
    } else {
        Material::Water
    };
    let weathers = match kind {
        BiomeKind::Verdant | BiomeKind::Archipelago | BiomeKind::SpireCanyon => {
            vec![WeatherKind::Clear, WeatherKind::Clear, WeatherKind::Rain]
        }
        BiomeKind::FungalHollow => vec![WeatherKind::Clear, WeatherKind::Spores],
        BiomeKind::VolcanicRift => vec![WeatherKind::Clear, WeatherKind::AshFall],
        BiomeKind::AcidMarsh => vec![WeatherKind::Clear, WeatherKind::AcidRain, WeatherKind::Rain],
        BiomeKind::DuneSea => vec![WeatherKind::Clear, WeatherKind::Clear, WeatherKind::Sandstorm],
        BiomeKind::SkyIsles => vec![WeatherKind::Clear, WeatherKind::Rain],
    };
    let liquid_cap = counts[Material::Water as usize] + counts[Material::Acid as usize] + 1500;
    let horizon_seed = seed ^ 0x40;
    let horizon = (0..WIDTH)
        .map(|x| (HEIGHT as f32 * 0.25 + fbm(horizon_seed, x as f32 / 60.0, 0.0, 3) * 70.0) as i32)
        .collect();
    let biome = Biome {
        kind,
        name: format!("{} of {}", kind.title(), names::planet(&mut rng)),
        palette,
        open_sky: g.open_sky,
        cave_ambient: match kind {
            BiomeKind::FungalHollow => 0.42,
            _ => 0.06,
        },
        gravity: if rng.prob(0.35) {
            rng.range(0.55, 0.8)
        } else if rng.prob(0.2) {
            rng.range(1.15, 1.35)
        } else {
            1.0
        },
        day_length: rng.range(100.0, 180.0),
        liquid,
        weathers,
        tree_cover: g.trees.len() as f32 / 8.0,
        trees: g.trees,
        surface: g.surface,
        water,
        lava,
        acid,
        soil,
        open_air,
        darkness: if g.open_sky { 0.2 } else { 0.9 },
        liquid_cap,
        horizon,
    };
    (biome, world)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_biome_generates() {
        for (i, kind) in BiomeKind::ALL.iter().enumerate() {
            let (b, w) = generate(100 + i as u64, Some(*kind));
            assert_eq!(b.kind, *kind);
            assert!(b.open_air > 0.15, "{kind:?} has too little air: {}", b.open_air);
            assert!(b.open_air < 0.9, "{kind:?} has no ground");
            assert_eq!(w.width(), WIDTH);
        }
    }

    #[test]
    fn biome_generation_is_deterministic() {
        let (a, wa) = generate(7, None);
        let (b, wb) = generate(7, None);
        assert_eq!(a.kind, b.kind);
        assert_eq!(a.name, b.name);
        assert_eq!(wa.encode_chunk(3, 2), wb.encode_chunk(3, 2));
    }
}
