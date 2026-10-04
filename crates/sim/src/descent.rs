//! World generation for the "Descent to the Core" roguelike: a tall shaft of
//! layers that get more dangerous with depth, with hand-shaped set pieces
//! stamped in, ending in a core chamber. Deterministic from the seed.
//!
//! Generation works on a plain material grid first (fast, easy to stamp
//! into), then commits it to a [`World`]. Everything except the core shell
//! can be dug, so a route down always exists; worm tunnels add natural
//! routes, partly plugged with soft material so the way down isn't obvious.

use crate::material::Material;
use crate::rng::Rng;
use crate::world::{World, disc};
use crate::worldgen::fbm;

pub const DESCENT_WIDTH: usize = 512;
pub const DESCENT_HEIGHT: usize = 4096;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Layer {
    Crust,
    UpperMantle,
    DeepMantle,
    OuterCore,
    Core,
}

impl Layer {
    pub const ALL: [Layer; 5] = [
        Layer::Crust,
        Layer::UpperMantle,
        Layer::DeepMantle,
        Layer::OuterCore,
        Layer::Core,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn name(self) -> &'static str {
        [
            "The Whispering Crust",
            "The Drowned Halls",
            "The Fungal Abyss",
            "The Molten Sanctum",
            "The Heart of the World",
        ][self.index()]
    }

    /// First row of the layer.
    pub fn top(self) -> i32 {
        [0, 960, 1920, 2880, 3712][self.index()]
    }

    pub fn bottom(self) -> i32 {
        match self {
            Layer::Core => DESCENT_HEIGHT as i32,
            l => Layer::ALL[l.index() + 1].top(),
        }
    }

    pub fn at_depth(y: i32) -> Layer {
        Layer::ALL
            .into_iter()
            .rev()
            .find(|l| y >= l.top())
            .unwrap_or(Layer::Crust)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CreatureKind {
    /// Pale, eyeless cave crawler.
    Gnawling,
    /// A failed apprentice that still casts twisted spells.
    Hollowed,
    /// Burrows through rock toward the sound of digging.
    BlindWyrm,
    /// Fungus-ridden walker that bursts into spores.
    SporePuppet,
    SporeDrifter,
    /// Fire-immune spectre that leaves burning trails; water destroys it.
    CinderWraith,
    /// Hunts the brightest light nearby.
    Lightseeker,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpawnKind {
    Chest,
    /// A free item on a pedestal.
    Altar,
    /// Items bought with ore.
    Shrine,
    Creature(CreatureKind),
    /// A fallen apprentice: shards, maybe a scroll, and a journal page.
    Remains,
    /// The goal.
    Core,
}

/// Something the game should create as an entity. `(x, y)` is the open cell
/// resting on the floor (creatures: any open cell).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Spawn {
    pub kind: SpawnKind,
    pub x: i32,
    pub y: i32,
}

pub struct Descent {
    pub world: World,
    pub spawns: Vec<Spawn>,
    /// Where the player starts (feet position, in cells).
    pub start: (i32, i32),
    /// Centre-bottom of the core.
    pub core: (i32, i32),
}

// Cave mask values.
const SOLID: u8 = 0;
const OPEN: u8 = 1;
/// Part of a tunnel, refilled with soft material to hide the route.
const PLUG: u8 = 2;

struct Grid {
    w: i32,
    h: i32,
    m: Vec<Material>,
    cave: Vec<u8>,
}

impl Grid {
    fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.w && y < self.h
    }
    fn i(&self, x: i32, y: i32) -> usize {
        (y * self.w + x) as usize
    }
    fn get(&self, x: i32, y: i32) -> Material {
        if self.in_bounds(x, y) {
            self.m[self.i(x, y)]
        } else {
            Material::CoreShell
        }
    }
    fn set(&mut self, x: i32, y: i32, mat: Material) {
        if self.in_bounds(x, y) {
            let i = self.i(x, y);
            self.m[i] = mat;
        }
    }
    fn is_air(&self, x: i32, y: i32) -> bool {
        self.get(x, y) == Material::Empty
    }
    fn is_ground(&self, x: i32, y: i32) -> bool {
        self.get(x, y).is_solid_for_player()
    }
    fn fill_rect(&mut self, x0: i32, y0: i32, w: i32, h: i32, mat: Material) {
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                self.set(x, y, mat);
            }
        }
    }
    fn fill_ellipse(
        &mut self,
        cx: i32,
        cy: i32,
        rx: i32,
        ry: i32,
        mut f: impl FnMut(i32, i32, f32) -> Option<Material>,
    ) {
        for y in cy - ry..=cy + ry {
            for x in cx - rx..=cx + rx {
                let (dx, dy) = ((x - cx) as f32 / rx as f32, (y - cy) as f32 / ry as f32);
                let d = dx * dx + dy * dy;
                if d <= 1.0
                    && let Some(m) = f(x, y, d)
                {
                    self.set(x, y, m);
                }
            }
        }
    }
    /// An open cell with ground below and `clear` open cells above it.
    fn is_floor_spot(&self, x: i32, y: i32, clear: i32, half_w: i32) -> bool {
        (-half_w..=half_w).all(|dx| self.is_ground(x + dx, y + 1))
            && (0..clear).all(|dy| (-half_w..=half_w).all(|dx| self.is_air(x + dx, y - dy)))
    }
}

pub fn generate_descent(seed: u64) -> Descent {
    let (w, h) = (DESCENT_WIDTH as i32, DESCENT_HEIGHT as i32);
    let mut g = Grid {
        w,
        h,
        m: vec![Material::Empty; (w * h) as usize],
        cave: vec![SOLID; (w * h) as usize],
    };
    let mut rng = Rng::new(seed ^ 0xD35C_E417);
    let mut spawns = Vec::new();

    let surface: Vec<i32> = (0..w)
        .map(|x| 70 + (fbm(seed ^ 11, x as f32 / 90.0, 0.0, 3) * 40.0) as i32)
        .collect();

    carve_caves(&mut g, seed, &surface);
    carve_worms(&mut g, &mut rng, &surface);
    fill_rock(&mut g, seed, &surface);
    fill_pockets(&mut g, seed);
    decorate(&mut g, seed, &mut rng, &surface);
    plant_trees(&mut g, &mut rng, &surface);

    let mut taken: Vec<(i32, i32, i32, i32)> = Vec::new();
    stamp_set_pieces(&mut g, &mut rng, &mut taken, &mut spawns);
    let core = build_core_chamber(&mut g, &mut rng, &mut spawns);
    scatter_chests(&mut g, &mut rng, &taken, &mut spawns);
    scatter_creatures(&g, &mut rng, &mut spawns);
    scatter_remains(&g, &mut rng, &taken, &mut spawns);

    let sx = w / 2;
    let sy = (0..h)
        .find(|&y| g.is_ground(sx, y))
        .unwrap_or(surface[sx as usize])
        - 1;
    // Clear a little landing spot in case a tree grew there.
    for y in sy - 16..=sy {
        for x in sx - 4..=sx + 4 {
            if g.get(x, y) == Material::Wood || g.get(x, y) == Material::Grass {
                g.set(x, y, Material::Empty);
            }
        }
    }

    let mut world = World::new(w as usize, h as usize, seed);
    for y in 0..h {
        for x in 0..w {
            let m = g.get(x, y);
            if m != Material::Empty {
                world.set(x, y, m);
            }
        }
    }
    world.settle_gravel();
    // Let loose sand and liquids find their rest before the player arrives.
    for _ in 0..90 {
        world.step();
    }
    world.settle_gravel();
    world.take_events();

    Descent {
        world,
        spawns,
        start: (sx, sy),
        core,
    }
}

fn cave_threshold(layer: Layer) -> f32 {
    match layer {
        Layer::Crust => 0.63,
        Layer::UpperMantle => 0.61,
        Layer::DeepMantle => 0.58,
        Layer::OuterCore => 0.61,
        Layer::Core => 0.64,
    }
}

fn carve_caves(g: &mut Grid, seed: u64, surface: &[i32]) {
    for y in 0..g.h {
        let layer = Layer::at_depth(y);
        let thr = cave_threshold(layer);
        for x in 0..g.w {
            let depth = y - surface[x as usize];
            if depth < 18 {
                continue;
            }
            let (fx, fy) = (x as f32, y as f32);
            let big = fbm(seed ^ 2, fx / 60.0, fy / 40.0, 4);
            let small = fbm(seed ^ 3, fx / 22.0, fy / 18.0, 3);
            if big > thr || small > 0.74 {
                let i = g.i(x, y);
                g.cave[i] = OPEN;
            }
        }
    }
}

/// Meandering tunnels from the top of each layer to the bottom. Some stretches
/// are plugged so they read as solid until dug.
fn carve_worms(g: &mut Grid, rng: &mut Rng, surface: &[i32]) {
    for layer in [
        Layer::Crust,
        Layer::UpperMantle,
        Layer::DeepMantle,
        Layer::OuterCore,
    ] {
        for _ in 0..2 {
            let mut x = 40.0 + rng.next_f32() * (g.w as f32 - 80.0);
            let mut y = layer.top().max(surface[x as usize] + 25) as f32;
            let mut angle: f32 = 0.0;
            let mut plug = false;
            let mut seg = 0;
            while (y as i32) < layer.bottom() + 20 && (y as i32) < g.h - 400 {
                angle = (angle + (rng.next_f32() - 0.5) * 0.5).clamp(-1.2, 1.2);
                x += angle.sin() * 1.6;
                y += 1.0 + rng.next_f32();
                if x < 20.0 || x > g.w as f32 - 20.0 {
                    angle = -angle;
                    x = x.clamp(20.0, g.w as f32 - 20.0);
                }
                seg += 1;
                if seg % 45 == 0 {
                    plug = rng.chance(150);
                }
                let r = 3 + (rng.next_u8() % 3) as i32;
                for (cx, cy) in disc(x as i32, y as i32, r) {
                    if g.in_bounds(cx, cy) {
                        let i = g.i(cx, cy);
                        g.cave[i] = if plug && g.cave[i] == SOLID {
                            PLUG
                        } else {
                            OPEN.max(g.cave[i])
                        };
                    }
                }
            }
        }
    }
}

/// The bulk rock of each layer, with caves left open and plugs filled softly.
fn fill_rock(g: &mut Grid, seed: u64, surface: &[i32]) {
    for y in 0..g.h {
        // Wobble the layer boundaries so they don't look ruled.
        for x in 0..g.w {
            let s = surface[x as usize];
            if y < s {
                continue;
            }
            let wobble = (fbm(seed ^ 7, x as f32 / 50.0, y as f32 / 50.0, 2) * 80.0 - 40.0) as i32;
            let layer = Layer::at_depth(y + wobble);
            let (fx, fy) = (x as f32, y as f32);
            let i = g.i(x, y);
            let cave = g.cave[i];
            if cave == OPEN {
                continue;
            }
            let depth = y - s;
            let n1 = fbm(seed ^ 4, fx / 28.0, fy / 28.0, 3);
            let n2 = fbm(seed ^ 5, fx / 40.0, fy / 24.0, 3);
            let m = if cave == PLUG {
                match layer {
                    Layer::Crust => {
                        if n1 > 0.5 {
                            Material::Sand
                        } else {
                            Material::Dirt
                        }
                    }
                    Layer::UpperMantle => Material::Gravel,
                    Layer::DeepMantle => {
                        if n1 > 0.55 {
                            Material::Fungus
                        } else {
                            Material::Gravel
                        }
                    }
                    _ => Material::Basalt,
                }
            } else {
                match layer {
                    Layer::Crust => {
                        if depth < 3 {
                            Material::Grass
                        } else if depth < 40 + (n2 * 60.0) as i32 {
                            if n1 > 0.7 { Material::Sand } else { Material::Dirt }
                        } else if n1 > 0.68 {
                            Material::Dirt
                        } else if n2 > 0.72 {
                            Material::Sand
                        } else {
                            Material::Stone
                        }
                    }
                    Layer::UpperMantle => {
                        if n1 > 0.7 {
                            Material::Gravel
                        } else if n2 > 0.55 {
                            Material::Basalt
                        } else {
                            Material::Stone
                        }
                    }
                    Layer::DeepMantle => {
                        if n1 > 0.74 {
                            Material::Gravel
                        } else if n2 > 0.6 {
                            Material::Stone
                        } else {
                            Material::Basalt
                        }
                    }
                    Layer::OuterCore => {
                        if n1 > 0.66 {
                            Material::Ferrite
                        } else if n2 > 0.7 {
                            Material::Obsidian
                        } else {
                            Material::Basalt
                        }
                    }
                    Layer::Core => {
                        if n1 > 0.6 {
                            Material::Obsidian
                        } else {
                            Material::Basalt
                        }
                    }
                }
            };
            g.m[i] = m;
        }
    }
}

/// Liquid pools on cave floors and gas trapped under cave ceilings.
fn fill_pockets(g: &mut Grid, seed: u64) {
    for x in 0..g.w {
        // Scan bottom-up, tracking how far above the cave floor we are.
        let mut above_floor = i32::MAX;
        for y in (0..g.h).rev() {
            if !g.is_air(x, y) || g.cave[g.i(x, y)] != OPEN {
                above_floor = i32::MAX;
                continue;
            }
            above_floor = if g.is_ground(x, y + 1) {
                0
            } else {
                above_floor.saturating_add(1)
            };
            let layer = Layer::at_depth(y);
            let (fx, fy) = (x as f32, y as f32);
            let pool = fbm(seed ^ 20, fx / 70.0, fy / 50.0, 2);
            let gas = fbm(seed ^ 21, fx / 45.0, fy / 45.0, 2);
            let liquid = match layer {
                Layer::Crust if pool > 0.56 && above_floor <= 14 => Some(Material::Water),
                Layer::Crust if pool < 0.22 && above_floor <= 6 => Some(Material::Oil),
                // The Drowned Halls: flooded caverns, lava only in the rare deep pocket.
                Layer::UpperMantle if pool > 0.47 && above_floor <= 18 => Some(Material::Water),
                Layer::UpperMantle if pool < 0.14 && above_floor <= 6 => Some(Material::Lava),
                Layer::DeepMantle if pool > 0.57 && above_floor <= 12 => Some(Material::Acid),
                Layer::DeepMantle if pool < 0.22 && above_floor <= 8 => Some(Material::Water),
                Layer::OuterCore if pool > 0.5 && above_floor <= 12 => Some(Material::Metal),
                Layer::OuterCore if pool < 0.25 && above_floor <= 8 => Some(Material::Lava),
                Layer::Core if pool > 0.55 && above_floor <= 10 => Some(Material::Lava),
                _ => None,
            };
            if let Some(m) = liquid {
                g.set(x, y, m);
            } else if matches!(layer, Layer::UpperMantle | Layer::DeepMantle) && gas > 0.66 && pool < 0.45 {
                g.set(x, y, Material::Gas);
            }
        }
    }
}

/// Shard veins, crystals, fungus, vents, roots, unstable ceilings and fractures.
fn decorate(g: &mut Grid, seed: u64, rng: &mut Rng, surface: &[i32]) {
    for y in 1..g.h - 1 {
        let layer = Layer::at_depth(y);
        for x in 1..g.w - 1 {
            let (fx, fy) = (x as f32, y as f32);
            let m = g.get(x, y);
            if m.is_solid_for_player() && m != Material::Grass {
                let ore = fbm(seed ^ 30, fx / 9.0, fy / 9.0, 2);
                let rich = [0.80, 0.78, 0.76, 0.75, 0.9][layer.index()];
                if ore > rich {
                    g.set(x, y, Material::ShardVein);
                    continue;
                }
                // Gravel ceilings over caves in the mantle: cave-ins waiting to happen.
                if layer == Layer::UpperMantle
                    && g.is_air(x, y + 1)
                    && fbm(seed ^ 31, fx / 30.0, fy / 30.0, 2) > 0.52
                {
                    for k in 0..(3 + rng.next_u8() % 5) as i32 {
                        if g.get(x, y - k).is_solid_for_player() {
                            g.set(x, y - k, Material::Gravel);
                        }
                    }
                }
                continue;
            }
            if !g.is_air(x, y) {
                continue;
            }
            let floor = g.is_ground(x, y + 1);
            let wall = g.is_ground(x - 1, y) || g.is_ground(x + 1, y) || g.is_ground(x, y - 1);
            match layer {
                Layer::UpperMantle if floor && rng.chance(3) && g.get(x, y + 1) != Material::Gravel => {
                    g.set(x, y + 1, Material::Vent);
                }
                Layer::DeepMantle => {
                    let glow = fbm(seed ^ 32, fx / 35.0, fy / 35.0, 2);
                    // The Fungal Abyss is carpeted in glowing (flammable) growth.
                    if (floor || wall) && glow > 0.42 && rng.chance(210) {
                        g.set(x, y, Material::Fungus);
                    } else if wall && glow < 0.4 && rng.chance(28) {
                        grow_crystal(g, rng, x, y);
                    }
                }
                Layer::OuterCore if wall && rng.chance(3) => grow_crystal(g, rng, x, y),
                _ => {}
            }
        }
    }

    // Pressure fractures: thin gravel-filled cracks in the deep mantle.
    for _ in 0..60 {
        let mut x = rng.next_f32() * g.w as f32;
        let mut y = (Layer::DeepMantle.top() as f32) + rng.next_f32() * 900.0;
        let mut a = rng.next_f32() * std::f32::consts::TAU;
        for _ in 0..(30 + rng.next_u8() as i32 / 2) {
            a += (rng.next_f32() - 0.5) * 0.6;
            x += a.cos();
            y += a.sin();
            let (ix, iy) = (x as i32, y as i32);
            if g.get(ix, iy).is_solid_for_player() {
                g.set(ix, iy, Material::Gravel);
            }
        }
    }

    // Roots growing down from the surface.
    for _ in 0..40 {
        let mut x = 10.0 + rng.next_f32() * (g.w as f32 - 20.0);
        let mut y = surface[x as usize] as f32 + 3.0;
        let mut a = std::f32::consts::FRAC_PI_2;
        for _ in 0..(20 + rng.next_u8() as i32 / 3) {
            a = (a + (rng.next_f32() - 0.5) * 0.7).clamp(0.5, 2.6);
            x += a.cos();
            y += a.sin();
            let (ix, iy) = (x as i32, y as i32);
            if matches!(g.get(ix, iy), Material::Dirt | Material::Stone | Material::Sand) {
                g.set(ix, iy, Material::Wood);
            }
        }
    }
}

fn grow_crystal(g: &mut Grid, rng: &mut Rng, x: i32, y: i32) {
    // Grow away from the nearest wall.
    let (dx, dy) = if g.is_ground(x - 1, y) {
        (1, -((rng.next_u8() % 2) as i32))
    } else if g.is_ground(x + 1, y) {
        (-1, -((rng.next_u8() % 2) as i32))
    } else if g.is_ground(x, y - 1) {
        ((rng.next_u8() % 3) as i32 - 1, 1)
    } else {
        ((rng.next_u8() % 3) as i32 - 1, -1)
    };
    let len = 3 + (rng.next_u8() % 7) as i32;
    for k in 0..len {
        let (cx, cy) = (x + dx * k, y + dy * k);
        if !g.is_air(cx, cy) {
            break;
        }
        g.set(cx, cy, Material::Crystal);
        if k < len / 2 {
            g.set(cx + dy.abs(), cy + dx.abs(), Material::Crystal);
        }
    }
}

fn plant_trees(g: &mut Grid, rng: &mut Rng, surface: &[i32]) {
    let mut x = 15 + (rng.next_u8() % 30) as i32;
    while x < g.w - 15 {
        // Keep the start area clear.
        if (x - g.w / 2).abs() > 24 {
            let base = surface[x as usize];
            let height = 18 + (rng.next_u8() % 14) as i32;
            for y in base - height..base + 2 {
                for dx in -1..=1 {
                    g.set(x + dx, y, Material::Wood);
                }
            }
            let (cx, cy, r) = (x, base - height, 6 + (rng.next_u8() % 4) as i32);
            g.fill_ellipse(cx, cy, r, r - 1, |lx, ly, d| {
                (d < 0.85 || (lx + ly) % 3 != 0).then_some(Material::Grass)
            });
        }
        x += 30 + (rng.next_u8() % 50) as i32;
    }
}

// ---- set pieces ----------------------------------------------------------

/// Finds a free rectangle inside a layer, away from other set pieces.
fn find_spot(
    rng: &mut Rng,
    layer: Layer,
    w: i32,
    h: i32,
    taken: &mut Vec<(i32, i32, i32, i32)>,
) -> Option<(i32, i32)> {
    let (top, bottom) = (layer.top().max(200) + 40, layer.bottom() - h - 40);
    for _ in 0..80 {
        let x = 12 + (rng.next_u64() % (DESCENT_WIDTH as u64 - w as u64 - 24)) as i32;
        let y = top + (rng.next_u64() % (bottom - top).max(1) as u64) as i32;
        let pad = 24;
        let clear = taken.iter().all(|&(tx, ty, tw, th)| {
            x + w + pad < tx || tx + tw + pad < x || y + h + pad < ty || ty + th + pad < y
        });
        if clear {
            taken.push((x, y, w, h));
            return Some((x, y));
        }
    }
    None
}

/// Draws a set piece with its top-left corner at (x, y).
type Stamp = fn(&mut Grid, &mut Rng, i32, i32, &mut Vec<Spawn>);

fn stamp_set_pieces(
    g: &mut Grid,
    rng: &mut Rng,
    taken: &mut Vec<(i32, i32, i32, i32)>,
    spawns: &mut Vec<Spawn>,
) {
    use Layer::*;
    attunement_chamber(g, rng, taken, spawns);
    let plan: [(Layer, Stamp, i32, i32); 17] = [
        (Crust, flooded_ruin, 64, 40),
        (Crust, abandoned_mine, 110, 18),
        (Crust, ritual_circle, 36, 36),
        (UpperMantle, drowned_hall, 130, 60),
        (UpperMantle, drowned_hall, 130, 60),
        (UpperMantle, magma_chamber, 150, 70),
        (UpperMantle, shrine_room, 40, 30),
        (UpperMantle, gas_chamber, 56, 34),
        (DeepMantle, crystal_cavern, 110, 60),
        (DeepMantle, mushroom_grove, 110, 70),
        (DeepMantle, mushroom_grove, 110, 70),
        (DeepMantle, gas_chamber, 56, 34),
        (DeepMantle, ritual_circle, 36, 36),
        (OuterCore, foundry, 120, 50),
        (OuterCore, gas_chamber, 56, 34),
        (OuterCore, ritual_circle, 36, 36),
        (OuterCore, shrine_room, 40, 30),
    ];
    for (layer, stamp, w, h) in plan {
        if let Some((x, y)) = find_spot(rng, layer, w, h, taken) {
            stamp(g, rng, x, y, spawns);
        }
    }
}

/// Brick ruin half-full of water, with wooden floors and a chest.
fn flooded_ruin(g: &mut Grid, rng: &mut Rng, x: i32, y: i32, spawns: &mut Vec<Spawn>) {
    let (w, h) = (64, 40);
    g.fill_rect(x, y, w, h, Material::Brick);
    g.fill_rect(x + 3, y + 3, w - 6, h - 6, Material::Empty);
    let water_top = y + 12 + (rng.next_u8() % 8) as i32;
    for yy in water_top..y + h - 3 {
        for xx in x + 3..x + w - 3 {
            g.set(xx, yy, Material::Water);
        }
    }
    // Wooden floors with gaps.
    for (fy, gap) in [(y + 14, x + 10), (y + 26, x + w - 22)] {
        for xx in x + 3..x + w - 3 {
            if !(gap..gap + 10).contains(&xx) {
                g.set(xx, fy, Material::Wood);
            }
        }
    }
    // Cracked wall: water will burst out when dug.
    let side = if rng.coin() { x } else { x + w - 3 };
    g.fill_rect(side, y + h - 14, 3, 6, Material::Gravel);
    spawns.push(Spawn {
        kind: SpawnKind::Chest,
        x: x + w / 2,
        y: y + h - 4,
    });
}

/// Horizontal timbered tunnel with explosive crates and an altar.
fn abandoned_mine(g: &mut Grid, rng: &mut Rng, x: i32, y: i32, spawns: &mut Vec<Spawn>) {
    let (w, h) = (110, 18);
    g.fill_rect(x, y, w, h, Material::Empty);
    g.fill_rect(x, y + h - 2, w, 2, Material::Wood);
    for px in (x + 4..x + w - 4).step_by(14) {
        g.fill_rect(px, y, 2, h - 2, Material::Wood);
        g.fill_rect(px - 3, y, 8, 2, Material::Wood);
    }
    for _ in 0..4 {
        let cx = x + 10 + (rng.next_u64() % (w as u64 - 30)) as i32;
        g.fill_rect(cx, y + h - 8, 6, 6, Material::Explosive);
    }
    let altar_left = rng.coin();
    let ax = if altar_left { x + 8 } else { x + w - 9 };
    spawns.push(Spawn {
        kind: SpawnKind::Altar,
        x: ax,
        y: y + h - 3,
    });
    spawns.push(Spawn {
        kind: SpawnKind::Chest,
        x: if altar_left { x + w - 10 } else { x + 10 },
        y: y + h - 3,
    });
}

/// Sealed pocket of explosive gas around a chest. One spark and it goes up.
fn gas_chamber(g: &mut Grid, rng: &mut Rng, x: i32, y: i32, spawns: &mut Vec<Spawn>) {
    let (w, h) = (56, 34);
    let (cx, cy) = (x + w / 2, y + h / 2);
    g.fill_ellipse(cx, cy, w / 2, h / 2, |_, _, d| {
        Some(if d > 0.78 { Material::Stone } else { Material::Gas })
    });
    // Flat floor for the chest.
    let floor = cy + h / 2 - 6;
    g.fill_rect(cx - 10, floor + 1, 20, 3, Material::Stone);
    for xx in cx - 10..cx + 10 {
        for yy in floor - 8..=floor {
            g.set(xx, yy, Material::Gas);
        }
    }
    for _ in 0..6 {
        let ox = cx - w / 3 + (rng.next_u8() as i32 % (w * 2 / 3));
        g.set(ox, cy - h / 2 + 5, Material::Crystal);
        g.set(ox, cy - h / 2 + 6, Material::Crystal);
    }
    if rng.coin() {
        g.fill_rect(cx + 12, floor - 5, 5, 6, Material::Explosive);
    }
    spawns.push(Spawn {
        kind: SpawnKind::Chest,
        x: cx,
        y: floor,
    });
}

/// Huge cavern over a lava lake, crossed by a rickety bridge, altar on a ledge.
fn magma_chamber(g: &mut Grid, rng: &mut Rng, x: i32, y: i32, spawns: &mut Vec<Spawn>) {
    let (w, h) = (150, 70);
    let (cx, cy) = (x + w / 2, y + h / 2);
    let lava_line = cy + h / 6;
    g.fill_ellipse(cx, cy, w / 2, h / 2, |_, yy, d| {
        Some(if d > 0.9 {
            Material::Basalt
        } else if yy > lava_line {
            Material::Lava
        } else {
            Material::Empty
        })
    });
    // The bridge: planks with gravel patches that give way.
    let by = cy - 2;
    for xx in x + 4..x + w - 4 {
        if g.get(xx, by) == Material::Empty || g.get(xx, by) == Material::Basalt {
            let m = if rng.chance(50) {
                Material::Gravel
            } else {
                Material::Wood
            };
            g.set(xx, by, m);
            g.set(xx, by + 1, Material::Wood);
        }
    }
    // Ledge with the altar at one end.
    let left = rng.coin();
    let lx = if left { x + 14 } else { x + w - 34 };
    g.fill_rect(lx, by - 1, 20, 1, Material::Basalt);
    for xx in lx..lx + 20 {
        for yy in by - 14..by - 1 {
            g.set(xx, yy, Material::Empty);
        }
    }
    spawns.push(Spawn {
        kind: SpawnKind::Altar,
        x: lx + 10,
        y: by - 2,
    });
    // Vents puff steam from the lake shore.
    for vx in [x + 20, x + w - 20] {
        g.set(vx, lava_line, Material::Vent);
    }
}

/// Glittering cave of crystals and glowing fungus, with a shrine and an altar.
fn crystal_cavern(g: &mut Grid, rng: &mut Rng, x: i32, y: i32, spawns: &mut Vec<Spawn>) {
    let (w, h) = (110, 60);
    let (cx, cy) = (x + w / 2, y + h / 2);
    g.fill_ellipse(cx, cy, w / 2, h / 2, |_, _, d| {
        Some(if d > 0.85 {
            Material::Stone
        } else {
            Material::Empty
        })
    });
    let floor = cy + h / 2 - 8;
    g.fill_rect(x + 8, floor + 1, w - 16, 6, Material::Stone);
    for xx in x + 8..x + w - 8 {
        for yy in floor - 30..=floor {
            if g.get(xx, yy) == Material::Stone && yy < floor {
                continue;
            }
            g.set(xx, yy, Material::Empty);
        }
        if rng.chance(140) {
            g.set(xx, floor, Material::Fungus);
        }
    }
    for _ in 0..40 {
        let px = x + 6 + (rng.next_u64() % (w as u64 - 12)) as i32;
        let py = y + 4 + (rng.next_u64() % (h as u64 - 8)) as i32;
        if g.is_air(px, py) && (g.is_ground(px, py - 1) || g.is_ground(px - 1, py) || g.is_ground(px + 1, py))
        {
            grow_crystal(g, rng, px, py);
        }
    }
    for xx in cx - 30..cx + 30 {
        for yy in floor - 24..=floor {
            if g.get(xx, yy) == Material::Crystal || g.get(xx, yy) == Material::Fungus {
                g.set(xx, yy, Material::Empty);
            }
        }
    }
    spawns.push(Spawn {
        kind: SpawnKind::Shrine,
        x: cx - 14,
        y: floor,
    });
    spawns.push(Spawn {
        kind: SpawnKind::Altar,
        x: cx + 14,
        y: floor,
    });
}

/// Small carved room holding a shrine.
fn shrine_room(g: &mut Grid, _rng: &mut Rng, x: i32, y: i32, spawns: &mut Vec<Spawn>) {
    let (w, h) = (40, 30);
    g.fill_rect(x, y, w, h, Material::Brick);
    g.fill_rect(x + 3, y + 3, w - 6, h - 6, Material::Empty);
    g.fill_rect(x + 3, y + h - 6, w - 6, 3, Material::Brick);
    spawns.push(Spawn {
        kind: SpawnKind::Shrine,
        x: x + w / 2,
        y: y + h - 7,
    });
}

/// Ferrite halls with molten metal channels, a shrine and an altar.
fn foundry(g: &mut Grid, rng: &mut Rng, x: i32, y: i32, spawns: &mut Vec<Spawn>) {
    let (w, h) = (120, 50);
    g.fill_rect(x, y, w, h, Material::Ferrite);
    g.fill_rect(x + 4, y + 4, w - 8, h - 8, Material::Empty);
    let floor = y + h - 10;
    g.fill_rect(x + 4, floor + 1, w - 8, 5, Material::Ferrite);
    // Metal channels in the floor.
    for cx in [x + 30, x + 60, x + 90] {
        g.fill_rect(cx - 5, floor + 1, 10, 4, Material::Metal);
    }
    // Pillars.
    for px in (x + 15..x + w - 10).step_by(25) {
        g.fill_rect(px, y + 4, 3, floor - y - 4, Material::Ferrite);
    }
    if rng.coin() {
        g.fill_rect(x + w - 20, floor - 6, 6, 6, Material::Explosive);
    }
    spawns.push(Spawn {
        kind: SpawnKind::Shrine,
        x: x + 22,
        y: floor,
    });
    spawns.push(Spawn {
        kind: SpawnKind::Altar,
        x: x + w - 34,
        y: floor,
    });
}

/// A small brick sanctum early in the crust, near the middle of the shaft,
/// holding the attunement altar most apprentices find first.
fn attunement_chamber(
    g: &mut Grid,
    rng: &mut Rng,
    taken: &mut Vec<(i32, i32, i32, i32)>,
    spawns: &mut Vec<Spawn>,
) {
    let (w, h) = (54, 30);
    for _ in 0..200 {
        let x = g.w / 2 - w / 2 + (rng.next_u8() as i32 % 160) - 80;
        let y = 170 + (rng.next_u8() as i32 % 80);
        let pad = 16;
        let clear = taken.iter().all(|&(tx, ty, tw, th)| {
            x + w + pad < tx || tx + tw + pad < x || y + h + pad < ty || ty + th + pad < y
        });
        if !clear {
            continue;
        }
        taken.push((x, y, w, h));
        g.fill_rect(x, y, w, h, Material::Brick);
        g.fill_rect(x + 3, y + 3, w - 6, h - 6, Material::Empty);
        g.fill_rect(x + 3, y + h - 6, w - 6, 3, Material::Brick);
        // Crystal candles either side of the altar.
        for cx in [x + 12, x + w - 13] {
            g.fill_rect(cx, y + h - 9, 1, 3, Material::Crystal);
        }
        // A doorway in the roof so it can be found from above.
        g.fill_rect(x + w / 2 - 4, y, 8, 3, Material::Empty);
        spawns.push(Spawn {
            kind: SpawnKind::Altar,
            x: x + w / 2,
            y: y + h - 7,
        });
        return;
    }
}

/// Vast flooded halls of arches and pillars.
fn drowned_hall(g: &mut Grid, rng: &mut Rng, x: i32, y: i32, spawns: &mut Vec<Spawn>) {
    let (w, h) = (130, 60);
    g.fill_rect(x, y, w, h, Material::Brick);
    g.fill_rect(x + 4, y + 4, w - 8, h - 8, Material::Empty);
    let water_top = y + 20 + (rng.next_u8() % 12) as i32;
    for px in (x + 18..x + w - 10).step_by(22) {
        g.fill_rect(px, y + 4, 4, h - 8, Material::Brick);
        // Arched openings between the pillars.
        g.fill_rect(px, y + 16, 4, 12, Material::Empty);
    }
    for yy in water_top..y + h - 4 {
        for xx in x + 4..x + w - 4 {
            if g.get(xx, yy) == Material::Empty {
                g.set(xx, yy, Material::Water);
            }
        }
    }
    // A shelf above the water where an apprentice drowned waiting.
    g.fill_rect(x + w - 30, water_top - 1, 24, 2, Material::Brick);
    spawns.push(Spawn {
        kind: SpawnKind::Remains,
        x: x + w - 18,
        y: water_top - 2,
    });
    if rng.coin() {
        spawns.push(Spawn {
            kind: SpawnKind::Chest,
            x: x + 30,
            y: y + h - 5,
        });
    }
}

/// A cavern of giant mushrooms: glowing, flammable, magnificent.
fn mushroom_grove(g: &mut Grid, rng: &mut Rng, x: i32, y: i32, spawns: &mut Vec<Spawn>) {
    let (w, h) = (110, 70);
    let (cx, cy) = (x + w / 2, y + h / 2);
    g.fill_ellipse(cx, cy, w / 2, h / 2, |_, _, d| {
        Some(if d > 0.86 {
            Material::Basalt
        } else {
            Material::Empty
        })
    });
    let floor = cy + h / 2 - 8;
    g.fill_rect(x + 8, floor + 1, w - 16, 6, Material::Basalt);
    for i in 0..4 {
        let mx = x + 18 + i * 24 + (rng.next_u8() % 6) as i32;
        let height = 18 + (rng.next_u8() % 16) as i32;
        g.fill_rect(mx - 1, floor - height, 3, height + 1, Material::Wood);
        let r = 7 + (rng.next_u8() % 5) as i32;
        g.fill_ellipse(mx, floor - height, r, r / 2 + 1, |_, yy, _| {
            (yy <= floor - height).then_some(Material::Fungus)
        });
    }
    for xx in x + 10..x + w - 10 {
        if rng.chance(90) {
            g.set(xx, floor, Material::Fungus);
        }
    }
    spawns.push(Spawn {
        kind: SpawnKind::Remains,
        x: cx + 6,
        y: floor - 1,
    });
}

/// A ring of brick and crystal where an apprentice died mid-ritual.
fn ritual_circle(g: &mut Grid, _rng: &mut Rng, x: i32, y: i32, spawns: &mut Vec<Spawn>) {
    let (cx, cy, r) = (x + 18, y + 18, 16);
    g.fill_ellipse(cx, cy, r, r, |_, _, d| {
        Some(if d > 0.8 { Material::Brick } else { Material::Empty })
    });
    let floor = cy + 6;
    g.fill_rect(cx - 10, floor + 1, 21, 3, Material::Obsidian);
    for k in 0..5 {
        let a = k as f32 / 5.0 * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        let (px, py) = (
            cx + (a.cos() * (r - 4) as f32) as i32,
            cy + (a.sin() * (r - 4) as f32) as i32,
        );
        g.set(px, py, Material::Crystal);
        g.set(px, py + 1, Material::Crystal);
    }
    spawns.push(Spawn {
        kind: SpawnKind::Remains,
        x: cx,
        y: floor,
    });
}

/// The final chamber: a core-shell sphere with a single opening on top.
fn build_core_chamber(g: &mut Grid, rng: &mut Rng, spawns: &mut Vec<Spawn>) -> (i32, i32) {
    let (cx, cy, rx, ry) = (g.w / 2, 3910, 190, 150);
    let gap_x = cx + (rng.next_u8() as i32 % 120) - 60;
    let floor = cy + ry / 2;
    g.fill_ellipse(cx, cy, rx, ry, |x, y, d| {
        let shell = d > 0.86;
        let opening = y < cy && (x - gap_x).abs() < 14;
        Some(if shell && !opening {
            Material::CoreShell
        } else if y > floor {
            Material::Obsidian
        } else {
            Material::Empty
        })
    });
    // Lava moats either side of the core plinth.
    for side in [-1, 1] {
        let mx = cx + side * 90;
        g.fill_rect(mx - 25, floor - 1, 50, 8, Material::Lava);
    }
    g.fill_rect(cx - 30, floor - 2, 60, 3, Material::Obsidian);
    // Bedrock floor across the bottom of the world.
    g.fill_rect(0, g.h - 6, g.w, 6, Material::CoreShell);
    let core = (cx, floor - 3);
    spawns.push(Spawn {
        kind: SpawnKind::Core,
        x: core.0,
        y: core.1,
    });
    core
}

fn scatter_chests(g: &mut Grid, rng: &mut Rng, taken: &[(i32, i32, i32, i32)], spawns: &mut Vec<Spawn>) {
    const PER_LAYER: usize = 5;
    for layer in [
        Layer::Crust,
        Layer::UpperMantle,
        Layer::DeepMantle,
        Layer::OuterCore,
    ] {
        let top = layer.top().max(150);
        let mut placed = 0;
        for attempt in 0..6000 {
            if placed >= PER_LAYER {
                break;
            }
            let x = 12 + (rng.next_u64() % (g.w as u64 - 24)) as i32;
            let y = top + (rng.next_u64() % (layer.bottom() - top - 20) as u64) as i32;
            let inside = taken
                .iter()
                .any(|&(tx, ty, tw, th)| x >= tx - 8 && x < tx + tw + 8 && y >= ty - 8 && y < ty + th + 8);
            if inside {
                continue;
            }
            if g.is_floor_spot(x, y, 9, 3) {
                spawns.push(Spawn {
                    kind: SpawnKind::Chest,
                    x,
                    y,
                });
                placed += 1;
            } else if attempt > 3000
                && g.get(x, y).is_solid_for_player()
                && g.get(x, y) != Material::CoreShell
            {
                // Not enough natural ledges: hollow out a buried niche instead.
                g.fill_rect(x - 7, y - 9, 15, 10, Material::Empty);
                g.fill_rect(x - 7, y + 1, 15, 2, Material::Stone);
                spawns.push(Spawn {
                    kind: SpawnKind::Chest,
                    x,
                    y,
                });
                placed += 1;
            }
        }
    }
}

/// Fallen apprentices lie where they gave up, a couple per layer.
fn scatter_remains(g: &Grid, rng: &mut Rng, taken: &[(i32, i32, i32, i32)], spawns: &mut Vec<Spawn>) {
    for layer in [
        Layer::Crust,
        Layer::UpperMantle,
        Layer::DeepMantle,
        Layer::OuterCore,
    ] {
        let top = layer.top().max(160);
        let mut placed = 0;
        for _ in 0..4000 {
            if placed >= 2 {
                break;
            }
            let x = 12 + (rng.next_u64() % (g.w as u64 - 24)) as i32;
            let y = top + (rng.next_u64() % (layer.bottom() - top - 20) as u64) as i32;
            let inside = taken
                .iter()
                .any(|&(tx, ty, tw, th)| x >= tx && x < tx + tw && y >= ty && y < ty + th);
            if !inside && g.is_floor_spot(x, y, 8, 4) {
                spawns.push(Spawn {
                    kind: SpawnKind::Remains,
                    x,
                    y,
                });
                placed += 1;
            }
        }
    }
}

fn scatter_creatures(g: &Grid, rng: &mut Rng, spawns: &mut Vec<Spawn>) {
    use CreatureKind::*;
    let plan = [
        (Layer::Crust, Gnawling, 8),
        (Layer::Crust, Hollowed, 3),
        (Layer::UpperMantle, Hollowed, 4),
        (Layer::UpperMantle, BlindWyrm, 2),
        (Layer::UpperMantle, Lightseeker, 4),
        (Layer::UpperMantle, Gnawling, 4),
        (Layer::DeepMantle, SporePuppet, 8),
        (Layer::DeepMantle, SporeDrifter, 8),
        (Layer::DeepMantle, Lightseeker, 4),
        (Layer::DeepMantle, Hollowed, 3),
        (Layer::OuterCore, CinderWraith, 8),
        (Layer::OuterCore, Lightseeker, 4),
        (Layer::OuterCore, Hollowed, 3),
    ];
    for (layer, kind, n) in plan {
        let mut placed = 0;
        for _ in 0..6000 {
            if placed >= n {
                break;
            }
            let x = 10 + (rng.next_u64() % (g.w as u64 - 20)) as i32;
            let y = layer.top().max(200)
                + (rng.next_u64() % (layer.bottom() - layer.top().max(200)) as u64) as i32;
            let ok = match kind {
                SporeDrifter | Lightseeker | CinderWraith => {
                    (-4..=4).all(|d| g.is_air(x + d, y) && g.is_air(x, y + d))
                }
                // Wyrms start buried in rock.
                BlindWyrm => (-3..=3).all(|d| g.is_ground(x + d, y) && g.is_ground(x, y + d)),
                _ => g.is_floor_spot(x, y, 8, 5),
            };
            if ok {
                spawns.push(Spawn {
                    kind: SpawnKind::Creature(kind),
                    x,
                    y,
                });
                placed += 1;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    #[test]
    fn layers_are_ordered() {
        assert_eq!(Layer::at_depth(0), Layer::Crust);
        assert_eq!(Layer::at_depth(1000), Layer::UpperMantle);
        assert_eq!(Layer::at_depth(4095), Layer::Core);
        for w in Layer::ALL.windows(2) {
            assert_eq!(w[0].bottom(), w[1].top());
        }
    }

    #[test]
    fn descent_is_deterministic_and_complete() {
        let a = generate_descent(1234);
        let b = generate_descent(1234);
        assert_eq!(a.spawns, b.spawns);
        for cy in (0..a.world.chunks_y()).step_by(7) {
            for cx in 0..a.world.chunks_x() {
                assert_eq!(a.world.encode_chunk(cx, cy), b.world.encode_chunk(cx, cy));
            }
        }

        let count = |k: SpawnKind| a.spawns.iter().filter(|s| s.kind == k).count();
        assert_eq!(count(SpawnKind::Core), 1);
        assert!(
            count(SpawnKind::Altar) >= 4,
            "altars: {}",
            count(SpawnKind::Altar)
        );
        assert!(
            count(SpawnKind::Shrine) >= 3,
            "shrines: {}",
            count(SpawnKind::Shrine)
        );
        assert!(
            count(SpawnKind::Chest) >= 12,
            "chests: {}",
            count(SpawnKind::Chest)
        );
        assert!(
            count(SpawnKind::Remains) >= 6,
            "remains: {}",
            count(SpawnKind::Remains)
        );
        // The attunement altar sits early, near the middle of the shaft.
        assert!(
            a.spawns
                .iter()
                .any(|s| s.kind == SpawnKind::Altar && s.y < 300 && (s.x - 256).abs() < 120)
        );

        let w = &a.world;
        let has = |layer: Layer, m: Material| {
            (layer.top()..layer.bottom()).any(|y| (0..w.width() as i32).any(|x| w.material(x, y) == m))
        };
        assert!(has(Layer::Crust, Material::Water));
        assert!(has(Layer::UpperMantle, Material::Lava));
        assert!(has(Layer::UpperMantle, Material::Vent));
        assert!(has(Layer::UpperMantle, Material::Gas));
        assert!(has(Layer::DeepMantle, Material::Acid));
        assert!(has(Layer::DeepMantle, Material::Crystal));
        assert!(has(Layer::DeepMantle, Material::Fungus));
        assert!(has(Layer::OuterCore, Material::Metal));
        assert!(has(Layer::Core, Material::CoreShell));
        assert!(has(Layer::Crust, Material::ShardVein));
    }

    /// Everything but the core shell can be dug, so the core is reachable iff
    /// a path of non-shell cells connects the start to it.
    #[test]
    fn core_is_reachable_from_the_start() {
        for seed in [1, 99, 4242] {
            let d = generate_descent(seed);
            let w = &d.world;
            let (wd, ht) = (w.width() as i32, w.height() as i32);
            let mut seen = vec![false; (wd * ht) as usize];
            let mut queue = VecDeque::from([d.start]);
            seen[(d.start.1 * wd + d.start.0) as usize] = true;
            let mut reached = false;
            while let Some((x, y)) = queue.pop_front() {
                if (x, y) == d.core {
                    reached = true;
                    break;
                }
                for (dx, dy) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                    let (nx, ny) = (x + dx, y + dy);
                    if nx < 0 || ny < 0 || nx >= wd || ny >= ht {
                        continue;
                    }
                    let i = (ny * wd + nx) as usize;
                    if !seen[i] && w.material(nx, ny) != Material::CoreShell {
                        seen[i] = true;
                        queue.push_back((nx, ny));
                    }
                }
            }
            assert!(reached, "seed {seed}: core unreachable");
        }
    }
}
