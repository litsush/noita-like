//! Coarse navigation grid and A* that understands each species' way of
//! moving: walkers jump and drop, climbers cling to any surface, fliers
//! and swimmers move freely in their medium, diggers tunnel.

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use super::math::{V2, v2};
use crate::material::{Kind, Material};
use crate::world::World;

pub const TILE: i32 = 4;

#[derive(Clone, Copy, Default)]
pub struct Tile {
    /// Blocking cells out of 16.
    pub solid: u8,
    /// Diggable cells out of 16.
    pub soil: u8,
    /// Trunk and canopy cells out of 16 (included in `solid`).
    pub tree: u8,
    /// Most common liquid and how many cells of it.
    pub liquid: Material,
    pub liquid_n: u8,
}

impl Tile {
    #[inline]
    pub fn open(&self) -> bool {
        self.solid <= 5
    }
    #[inline]
    pub fn wet(&self) -> bool {
        self.liquid_n >= 7
    }
}

/// How a species moves, as far as pathfinding cares.
#[derive(Clone, Copy, Debug, Default)]
pub struct Profile {
    pub walk: bool,
    pub climb: bool,
    pub fly: bool,
    pub dig: bool,
    pub swim_water: bool,
    pub swim_lava: bool,
    pub swim_acid: bool,
    pub lava_ok: bool,
    pub acid_ok: bool,
    /// Can't leave liquid at all.
    pub only_liquid: bool,
    pub jump_tiles: i32,
    pub drop_tiles: i32,
    pub height_tiles: i32,
    /// Tiles either side of the centre column the body also covers.
    pub half_width: i32,
}

pub struct NavGrid {
    pub w: i32,
    pub h: i32,
    pub tiles: Vec<Tile>,
    g: Vec<f32>,
    came: Vec<u32>,
    stamp: Vec<u32>,
    query: u32,
}

const NONE: u32 = u32::MAX;

impl NavGrid {
    pub fn new(world: &World) -> NavGrid {
        let w = world.width() as i32 / TILE;
        let h = world.height() as i32 / TILE;
        let n = (w * h) as usize;
        let mut g = NavGrid {
            w,
            h,
            tiles: vec![Tile::default(); n],
            g: vec![0.0; n],
            came: vec![NONE; n],
            stamp: vec![0; n],
            query: 0,
        };
        g.rebuild(world);
        g
    }

    pub fn rebuild(&mut self, world: &World) {
        for ty in 0..self.h {
            for tx in 0..self.w {
                let mut t = Tile::default();
                let mut water = 0u8;
                let mut lava = 0u8;
                let mut acid = 0u8;
                for dy in 0..TILE {
                    for dx in 0..TILE {
                        let m = world.material(tx * TILE + dx, ty * TILE + dy);
                        if m.is_solid_for_player() {
                            t.solid += 1;
                        }
                        if m.is_diggable() {
                            t.soil += 1;
                        }
                        if m.is_tree() {
                            t.tree += 1;
                        }
                        match m {
                            Material::Water | Material::Oil => water += 1,
                            Material::Lava => lava += 1,
                            Material::Acid => acid += 1,
                            _ => {}
                        }
                    }
                }
                let (liquid, n) = [
                    (Material::Water, water),
                    (Material::Lava, lava),
                    (Material::Acid, acid),
                ]
                .into_iter()
                .max_by_key(|(_, n)| *n)
                .unwrap();
                t.liquid = if n > 0 { liquid } else { Material::Empty };
                t.liquid_n = n;
                self.tiles[(ty * self.w + tx) as usize] = t;
            }
        }
    }

    #[inline]
    pub fn tile(&self, x: i32, y: i32) -> Option<&Tile> {
        (x >= 0 && y >= 0 && x < self.w && y < self.h).then(|| &self.tiles[(y * self.w + x) as usize])
    }

    /// Open for this mover: trees only get in the way of climbers.
    #[inline]
    fn tile_open(&self, p: &Profile, t: &Tile) -> bool {
        t.solid - if p.climb { 0 } else { t.tree } <= 5
    }

    #[inline]
    fn open_p(&self, p: &Profile, x: i32, y: i32) -> bool {
        self.tile(x, y).is_some_and(|t| self.tile_open(p, t))
    }

    fn harmful(&self, p: &Profile, t: &Tile) -> bool {
        t.liquid_n >= 4
            && match t.liquid {
                Material::Lava => !p.lava_ok,
                Material::Acid => !p.acid_ok,
                _ => false,
            }
    }

    fn swimmable(&self, p: &Profile, t: &Tile) -> bool {
        t.wet()
            && match t.liquid {
                Material::Water => p.swim_water,
                Material::Lava => p.swim_lava,
                Material::Acid => p.swim_acid,
                _ => false,
            }
    }

    /// Room for the whole body: its width around column `x` and its height above row `y`.
    fn headroom(&self, p: &Profile, x: i32, y: i32) -> bool {
        (-p.half_width..=p.half_width).all(|dx| {
            (0..p.height_tiles).all(|k| {
                k == 0
                    || self.open_p(p, x + dx, y - k)
                    || (p.dig && self.tile(x + dx, y - k).is_some_and(|t| t.soil >= 8))
            })
        })
    }

    /// Can move in any direction from/to here.
    fn free(&self, p: &Profile, x: i32, y: i32) -> bool {
        let Some(t) = self.tile(x, y) else { return false };
        if self.harmful(p, t) {
            return false;
        }
        if self.swimmable(p, t) {
            return true;
        }
        if p.only_liquid {
            return false;
        }
        if self.tile_open(p, t) {
            if p.fly {
                return p.half_width == 0 || self.headroom(p, x, y + p.height_tiles / 2);
            }
            if p.climb {
                return (-1..=1).any(|dy| (-1..=1).any(|dx| !self.open_p(p, x + dx, y + dy)));
            }
            return false;
        }
        // Diggers won't tunnel into the side of a harmful pool.
        p.dig
            && t.soil >= 8
            && [(1, 0), (-1, 0), (0, 1), (0, -1)]
                .iter()
                .all(|(dx, dy)| self.tile(x + dx, y + dy).is_none_or(|n| !self.harmful(p, n)))
    }

    /// A walker can stand here.
    fn standable(&self, p: &Profile, x: i32, y: i32) -> bool {
        if !p.walk || p.only_liquid {
            return false;
        }
        let Some(t) = self.tile(x, y) else { return false };
        if !self.tile_open(p, t) || self.harmful(p, t) {
            return false;
        }
        let below = self.tile(x, y + 1);
        let supported = below.is_none_or(|b| !self.tile_open(p, b) || b.wet());
        supported && self.headroom(p, x, y)
    }

    pub fn passable(&self, p: &Profile, x: i32, y: i32) -> bool {
        self.free(p, x, y) || self.standable(p, x, y)
    }

    fn cost(&self, p: &Profile, x: i32, y: i32) -> f32 {
        let t = self.tile(x, y).unwrap();
        if !self.tile_open(p, t) && !t.wet() {
            4.0 // digging is slow
        } else if t.wet() && !self.swimmable(p, t) {
            6.0 // wading
        } else {
            1.0
        }
    }

    fn neighbours(&self, p: &Profile, x: i32, y: i32, out: &mut Vec<(i32, i32, f32)>) {
        out.clear();
        let here_free = self.free(p, x, y);
        for dy in -1..=1 {
            for dx in -1..=1 {
                if dx == 0 && dy == 0 {
                    continue;
                }
                let (nx, ny) = (x + dx, y + dy);
                if !self.passable(p, nx, ny) {
                    continue;
                }
                let both_free = here_free && self.free(p, nx, ny);
                let walk_step = dy == 0 && self.standable(p, nx, ny);
                let mixed = (here_free || self.free(p, nx, ny)) && (dx == 0 || dy == 0 || here_free);
                if both_free || walk_step || mixed {
                    // No cutting diagonal corners through solid tiles.
                    if dx != 0 && dy != 0 && !(self.passable(p, x + dx, y) || self.passable(p, x, y + dy)) {
                        continue;
                    }
                    let d = if dx != 0 && dy != 0 { 1.41 } else { 1.0 };
                    out.push((nx, ny, d * self.cost(p, nx, ny)));
                }
            }
        }
        if !p.walk || !self.standable(p, x, y) {
            return;
        }
        for dir in [-1, 1] {
            let nx = x + dir;
            // Jump up onto ledges, if the whole body fits through on the way.
            for k in 1..=p.jump_tiles {
                if !self.headroom(p, x, y - k) {
                    break;
                }
                if self.standable(p, nx, y - k) {
                    out.push((nx, y - k, 1.0 + k as f32 * 0.6));
                    break;
                }
            }
            // Drop off ledges.
            if self.open_p(p, nx, y) {
                for k in 1..=p.drop_tiles {
                    if !self.open_p(p, nx, y + k) && !self.tile(nx, y + k).is_some_and(|t| t.wet()) {
                        break;
                    }
                    if self.standable(p, nx, y + k) {
                        out.push((nx, y + k, 1.0 + k as f32 * 0.25));
                        break;
                    }
                }
            }
            // Leap across gaps.
            if p.jump_tiles >= 2
                && self.open_p(p, nx, y)
                && self.open_p(p, nx, y - 1)
                && !self.standable(p, nx, y)
            {
                for gap in 2..=(1 + p.jump_tiles / 2).min(4) {
                    let gx = x + dir * gap;
                    for gy in [y, y - 1, y + 1] {
                        if self.standable(p, gx, gy) {
                            out.push((gx, gy, gap as f32 * 1.3));
                        }
                    }
                }
            }
        }
    }

    /// Nearest passable tile to (x, y) within a small radius.
    pub fn nearest_passable(&self, p: &Profile, x: i32, y: i32, radius: i32) -> Option<(i32, i32)> {
        for r in 0..=radius {
            for dy in -r..=r {
                for dx in -r..=r {
                    if dx.abs() != r && dy.abs() != r {
                        continue;
                    }
                    if self.passable(p, x + dx, y + dy) {
                        return Some((x + dx, y + dy));
                    }
                }
            }
        }
        None
    }

    pub fn tile_of(pos: V2) -> (i32, i32) {
        (pos.x.floor() as i32 / TILE, pos.y.floor() as i32 / TILE)
    }

    pub fn centre(x: i32, y: i32) -> V2 {
        v2(
            (x * TILE) as f32 + TILE as f32 / 2.0,
            (y * TILE) as f32 + TILE as f32 / 2.0,
        )
    }

    /// A* from `from` to `to` in world coordinates. Returns waypoints (tile
    /// centres), ending as close to `to` as could be reached.
    pub fn find_path(&mut self, p: &Profile, from: V2, to: V2, max_expand: usize) -> Vec<V2> {
        let (sx, sy) = Self::tile_of(from);
        let Some((sx, sy)) = self.nearest_passable(p, sx, sy, 2) else {
            return Vec::new();
        };
        let (gx, gy) = Self::tile_of(to);
        let Some((gx, gy)) = self.nearest_passable(p, gx, gy, 3) else {
            return Vec::new();
        };
        self.query = self.query.wrapping_add(1);
        let q = self.query;
        let w = self.w;
        let idx = |x: i32, y: i32| (y * w + x) as usize;
        let h = |x: i32, y: i32| (((x - gx).pow(2) + (y - gy).pow(2)) as f32).sqrt();
        let mut open = BinaryHeap::new();
        let s = idx(sx, sy);
        self.stamp[s] = q;
        self.g[s] = 0.0;
        self.came[s] = NONE;
        open.push(Reverse(((h(sx, sy) * 100.0) as u32, s as u32)));
        let mut best = (h(sx, sy), s);
        let mut expanded = 0;
        let mut nbuf = Vec::with_capacity(16);
        while let Some(Reverse((_, cur))) = open.pop() {
            let cur = cur as usize;
            let (cx, cy) = ((cur as i32) % w, (cur as i32) / w);
            if cx == gx && cy == gy {
                best = (0.0, cur);
                break;
            }
            expanded += 1;
            if expanded > max_expand {
                break;
            }
            self.neighbours(p, cx, cy, &mut nbuf);
            let gc = self.g[cur];
            for &(nx, ny, c) in &nbuf {
                let n = idx(nx, ny);
                let ng = gc + c;
                if self.stamp[n] == q && self.g[n] <= ng {
                    continue;
                }
                self.stamp[n] = q;
                self.g[n] = ng;
                self.came[n] = cur as u32;
                let hn = h(nx, ny);
                if hn < best.0 {
                    best = (hn, n);
                }
                open.push(Reverse((((ng + hn * 1.2) * 100.0) as u32, n as u32)));
            }
        }
        let mut path = Vec::new();
        let mut n = best.1;
        while n != NONE as usize {
            let (x, y) = ((n as i32) % w, (n as i32) / w);
            path.push(Self::centre(x, y));
            if n == s {
                break;
            }
            n = self.came[n] as usize;
            if path.len() > 600 {
                break;
            }
        }
        path.reverse();
        // The first waypoint is where we already are.
        if path.len() > 1 {
            path.remove(0);
        }
        path
    }
}

/// Line of sight between two points through the cell world. Solids and
/// thick smoke block it; liquids don't.
pub fn line_of_sight(world: &World, a: V2, b: V2) -> bool {
    let d = b - a;
    let steps = d.x.abs().max(d.y.abs()).ceil() as i32;
    if steps == 0 {
        return true;
    }
    let inc = d / steps as f32;
    let mut p = a;
    let mut smoke = 0;
    for _ in 0..steps {
        p += inc;
        let m = world.material(p.x as i32, p.y as i32);
        if m.blocks_creature(false) && m != Material::Nest {
            return false;
        }
        // Nest walls are porous enough to peek through only at the edges.
        if m == Material::Nest {
            smoke += 3;
        }
        // Canopy is cover, not a wall.
        if m == Material::Leaf || m == Material::Smoke || m.kind() == Kind::Gas && m != Material::Steam {
            smoke += 1;
        }
        if smoke > 6 {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    fn flat_world() -> World {
        let mut w = World::new(128, 64, 1);
        for x in 0..128 {
            for y in 48..64 {
                w.set(x, y, Material::Stone);
            }
        }
        w
    }

    #[test]
    fn walker_crosses_flat_ground_and_climbs_step() {
        let mut w = flat_world();
        // A 6-cell step halfway.
        for x in 64..128 {
            for y in 42..48 {
                w.set(x, y, Material::Stone);
            }
        }
        let mut nav = NavGrid::new(&w);
        let p = Profile {
            walk: true,
            jump_tiles: 3,
            drop_tiles: 8,
            height_tiles: 2,
            ..Default::default()
        };
        let path = nav.find_path(&p, v2(10.0, 44.0), v2(110.0, 38.0), 5000);
        let last = *path.last().unwrap();
        assert!((last.x - 110.0).abs() < 6.0, "path ended at {last:?}");
    }

    #[test]
    fn swimmer_stays_in_water() {
        let mut w = flat_world();
        for x in 0..128 {
            for y in 30..48 {
                w.set(x, y, Material::Water);
            }
        }
        let mut nav = NavGrid::new(&w);
        let p = Profile {
            swim_water: true,
            only_liquid: true,
            ..Default::default()
        };
        let path = nav.find_path(&p, v2(10.0, 40.0), v2(100.0, 5.0), 5000);
        assert!(path.iter().all(|p| p.y >= 28.0), "swimmer left the water");
    }

    #[test]
    fn sight_is_blocked_by_rock() {
        let w = flat_world();
        assert!(line_of_sight(&w, v2(5.0, 20.0), v2(100.0, 20.0)));
        assert!(!line_of_sight(&w, v2(5.0, 20.0), v2(100.0, 60.0)));
    }
}
