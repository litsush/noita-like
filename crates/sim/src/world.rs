use crate::material::{Kind, Material};
use crate::rng::{Rng, hash2};

/// Side length of a chunk in cells. Chunks are the unit of sleeping,
/// rendering updates, and network replication.
pub const CHUNK_SIZE: usize = 64;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cell {
    pub mat: Material,
    /// Per-cell brightness variation so materials look grainy.
    pub shade: u8,
    /// Remaining lifetime for fire/gases; unused (0) for everything else.
    pub life: u8,
    /// Tick parity marker so a cell that moved isn't updated twice in one step.
    clock: u8,
}

impl Cell {
    pub const EMPTY: Cell = Cell {
        mat: Material::Empty,
        shade: 0,
        life: 0,
        clock: 0,
    };

    pub fn new(mat: Material, shade: u8) -> Cell {
        Cell {
            mat,
            shade,
            life: mat.initial_life(),
            clock: 0,
        }
    }

    pub fn rgba(&self) -> [u8; 4] {
        let props = self.mat.props();
        if self.mat == Material::Empty {
            return [0, 0, 0, 0];
        }
        let v = props.variance as i32;
        let offset = if v == 0 {
            0
        } else {
            (self.shade as i32 * v) / 255 - v / 2
        };
        let [mut r, mut g, mut b] = props.color.map(|c| c as i32 + offset);
        let mut a = 255;
        match self.mat.kind() {
            Kind::Fire => {
                // Young fire is bright yellow, old fire is dark red.
                let t = self.life as i32 * 255 / Material::Fire.initial_life() as i32;
                g = 40 + t * 180 / 255 + offset / 2;
                b = 10 + t * 40 / 255;
                r = 200 + offset.abs();
            }
            Kind::Gas => {
                let max = self.mat.initial_life().max(1) as i32;
                a = 60 + self.life as i32 * 140 / max;
            }
            Kind::Liquid if self.mat != Material::Lava => a = 220,
            _ => {}
        }
        [clamp(r), clamp(g), clamp(b), clamp(a)]
    }
}

#[inline]
fn clamp(v: i32) -> u8 {
    v.clamp(0, 255) as u8
}

#[derive(Clone, Copy, Default)]
struct ChunkState {
    /// Simulated this tick.
    active: bool,
    /// Something changed in or next to this chunk; simulate next tick.
    active_next: bool,
    /// Needs its pixels re-uploaded.
    render_dirty: bool,
    /// Needs to be sent to remote peers.
    net_dirty: bool,
}

#[derive(Debug)]
pub struct DecodeError;

pub struct World {
    width: usize,
    height: usize,
    chunks_x: usize,
    chunks_y: usize,
    cells: Vec<Cell>,
    chunks: Vec<ChunkState>,
    rng: Rng,
    seed: u64,
    tick: u64,
    clock: u8,
}

impl World {
    /// Creates an empty world. Dimensions are rounded up to whole chunks.
    pub fn new(width: usize, height: usize, seed: u64) -> World {
        let chunks_x = width.div_ceil(CHUNK_SIZE);
        let chunks_y = height.div_ceil(CHUNK_SIZE);
        let (width, height) = (chunks_x * CHUNK_SIZE, chunks_y * CHUNK_SIZE);
        World {
            width,
            height,
            chunks_x,
            chunks_y,
            cells: vec![Cell::EMPTY; width * height],
            chunks: vec![
                ChunkState {
                    active: false,
                    active_next: true,
                    render_dirty: true,
                    net_dirty: true
                };
                chunks_x * chunks_y
            ],
            rng: Rng::new(seed),
            seed,
            tick: 0,
            clock: 0,
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }
    pub fn height(&self) -> usize {
        self.height
    }
    pub fn chunks_x(&self) -> usize {
        self.chunks_x
    }
    pub fn chunks_y(&self) -> usize {
        self.chunks_y
    }
    pub fn seed(&self) -> u64 {
        self.seed
    }
    pub fn tick(&self) -> u64 {
        self.tick
    }

    #[inline]
    pub fn in_bounds(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && (x as usize) < self.width && (y as usize) < self.height
    }

    #[inline]
    fn idx(&self, x: i32, y: i32) -> usize {
        y as usize * self.width + x as usize
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32) -> Option<Cell> {
        self.in_bounds(x, y).then(|| self.cells[self.idx(x, y)])
    }

    /// Material at a position; out of bounds reads as stone (the world is walled).
    #[inline]
    pub fn material(&self, x: i32, y: i32) -> Material {
        self.get(x, y).map_or(Material::Stone, |c| c.mat)
    }

    #[inline]
    pub fn is_solid(&self, x: i32, y: i32) -> bool {
        self.material(x, y).is_solid_for_player()
    }

    /// Places a fresh cell of `mat`, giving it a random shade.
    pub fn set(&mut self, x: i32, y: i32, mat: Material) {
        if !self.in_bounds(x, y) {
            return;
        }
        let shade = self.rng.next_u8();
        let i = self.idx(x, y);
        self.cells[i] = Cell::new(mat, shade);
        self.mark_changed(x, y);
    }

    /// Fills a disc. `Material::Empty` digs everything; other materials only
    /// fill empty or gaseous cells so painting doesn't overwrite terrain.
    /// Returns true if anything changed.
    pub fn paint_circle(&mut self, cx: i32, cy: i32, radius: i32, mat: Material) -> bool {
        let mut changed = false;
        for y in cy - radius..=cy + radius {
            for x in cx - radius..=cx + radius {
                let (dx, dy) = (x - cx, y - cy);
                if dx * dx + dy * dy > radius * radius || !self.in_bounds(x, y) {
                    continue;
                }
                let cur = self.material(x, y);
                let replace = if mat == Material::Empty {
                    cur != Material::Empty
                } else {
                    matches!(cur.kind(), Kind::Empty | Kind::Gas) && cur != mat
                };
                if replace {
                    self.set(x, y, mat);
                    changed = true;
                }
            }
        }
        changed
    }

    /// Wakes every chunk touching the 3x3 neighbourhood of (x, y) and flags
    /// the owning chunk for rendering and replication.
    fn mark_changed(&mut self, x: i32, y: i32) {
        let cs = CHUNK_SIZE as i32;
        let cx0 = ((x - 1).max(0) / cs) as usize;
        let cy0 = ((y - 1).max(0) / cs) as usize;
        let cx1 = (((x + 1) / cs) as usize).min(self.chunks_x - 1);
        let cy1 = (((y + 1) / cs) as usize).min(self.chunks_y - 1);
        for cy in cy0..=cy1 {
            for cx in cx0..=cx1 {
                self.chunks[cy * self.chunks_x + cx].active_next = true;
            }
        }
        let c = &mut self.chunks[(y / cs) as usize * self.chunks_x + (x / cs) as usize];
        c.render_dirty = true;
        c.net_dirty = true;
    }

    /// Advances the simulation by one tick.
    pub fn step(&mut self) {
        self.tick += 1;
        self.clock = self.clock.wrapping_add(1);
        for c in &mut self.chunks {
            c.active = std::mem::take(&mut c.active_next);
        }

        // Bottom-up so falling cells move once per tick; alternate horizontal
        // direction per row and tick to avoid a directional bias.
        for y in (0..self.height as i32).rev() {
            let cy = y as usize / CHUNK_SIZE;
            let ltr = (self.tick as i32 + y) & 1 == 0;
            for i in 0..self.chunks_x {
                let cx = if ltr { i } else { self.chunks_x - 1 - i };
                if !self.chunks[cy * self.chunks_x + cx].active {
                    continue;
                }
                let x0 = (cx * CHUNK_SIZE) as i32;
                for j in 0..CHUNK_SIZE as i32 {
                    let x = if ltr {
                        x0 + j
                    } else {
                        x0 + CHUNK_SIZE as i32 - 1 - j
                    };
                    self.update_cell(x, y);
                }
            }
        }
    }

    fn update_cell(&mut self, x: i32, y: i32) {
        let i = self.idx(x, y);
        let cell = self.cells[i];
        if cell.clock == self.clock {
            return;
        }
        match cell.mat.kind() {
            Kind::Empty | Kind::Solid => {}
            Kind::Powder => self.update_powder(x, y),
            Kind::Liquid => self.update_liquid(x, y, cell),
            Kind::Gas => self.update_gas(x, y, cell),
            Kind::Fire => self.update_fire(x, y, cell),
        }
    }

    /// Can a cell of `src` move into a cell occupied by `dst`?
    #[inline]
    fn can_displace(src: Material, dst: Material) -> bool {
        match dst.kind() {
            Kind::Empty => true,
            Kind::Liquid | Kind::Gas | Kind::Fire => dst.props().density < src.props().density,
            Kind::Solid | Kind::Powder => false,
        }
    }

    fn try_move(&mut self, x: i32, y: i32, nx: i32, ny: i32) -> bool {
        if !self.in_bounds(nx, ny) {
            return false;
        }
        let (a, b) = (self.idx(x, y), self.idx(nx, ny));
        if !Self::can_displace(self.cells[a].mat, self.cells[b].mat) {
            return false;
        }
        self.cells.swap(a, b);
        self.cells[a].clock = self.clock;
        self.cells[b].clock = self.clock;
        self.mark_changed(x, y);
        self.mark_changed(nx, ny);
        true
    }

    fn update_powder(&mut self, x: i32, y: i32) {
        if self.try_move(x, y, x, y + 1) {
            return;
        }
        let d = if self.rng.coin() { 1 } else { -1 };
        let _ = self.try_move(x, y, x + d, y + 1) || self.try_move(x, y, x - d, y + 1);
    }

    fn update_liquid(&mut self, x: i32, y: i32, cell: Cell) {
        if cell.mat == Material::Lava && self.react_hot(x, y, 12) {
            return;
        }
        if cell.mat == Material::Acid && self.react_acid(x, y) {
            return;
        }
        if self.try_move(x, y, x, y + 1) {
            return;
        }
        let d = if self.rng.coin() { 1 } else { -1 };
        if self.try_move(x, y, x + d, y + 1) || self.try_move(x, y, x - d, y + 1) {
            return;
        }
        let reach = cell.mat.props().dispersion as i32;
        for dir in [d, -d] {
            let mut target = None;
            for k in 1..=reach {
                let nx = x + dir * k;
                if !self.in_bounds(nx, y) || !Self::can_displace(cell.mat, self.material(nx, y)) {
                    break;
                }
                target = Some(nx);
                // Stop at a ledge so the liquid falls over it next tick.
                if Self::can_displace(cell.mat, self.material(nx, y + 1)) {
                    break;
                }
            }
            if let Some(nx) = target {
                self.try_move(x, y, nx, y);
                return;
            }
        }
    }

    fn update_gas(&mut self, x: i32, y: i32, mut cell: Cell) {
        let i = self.idx(x, y);
        if cell.life <= 1 {
            // Some steam condenses back into water.
            let next = if cell.mat == Material::Steam && self.rng.chance(40) {
                Material::Water
            } else {
                Material::Empty
            };
            self.set(x, y, next);
            return;
        }
        cell.life -= 1;
        self.cells[i].life = cell.life;
        self.mark_changed(x, y);

        let d = if self.rng.coin() { 1 } else { -1 };
        if self.rng.chance(200) && self.try_move(x, y, x, y - 1) {
            return;
        }
        let _ = self.try_move(x, y, x + d, y - 1)
            || self.try_move(x, y, x - d, y - 1)
            || self.try_move(x, y, x + d, y);
    }

    fn update_fire(&mut self, x: i32, y: i32, cell: Cell) {
        let i = self.idx(x, y);
        let burn = 1 + (self.rng.next_u8() & 1);
        if cell.life <= burn {
            let r = self.rng.next_u8();
            let next = match r {
                0..40 => Material::Smoke,
                40..52 => Material::Ash,
                _ => Material::Empty,
            };
            self.set(x, y, next);
            return;
        }
        self.cells[i].life = cell.life - burn;
        self.mark_changed(x, y);

        if self.react_hot(x, y, 0) {
            return;
        }
        // Flicker upwards.
        if self.rng.chance(120) {
            let d = (self.rng.next_u8() % 3) as i32 - 1;
            if self.material(x + d, y - 1) == Material::Empty {
                self.try_move(x, y, x + d, y - 1);
            }
        }
    }

    /// Shared behaviour for fire and lava: ignite flammable neighbours and
    /// react with water. `extra_ignite` boosts ignition odds (lava is hotter).
    /// Returns true if this cell was consumed.
    fn react_hot(&mut self, x: i32, y: i32, extra_ignite: u8) -> bool {
        let is_lava = self.material(x, y) == Material::Lava;
        for (dx, dy) in NEIGHBOURS {
            let (nx, ny) = (x + dx, y + dy);
            let n = self.material(nx, ny);
            if !self.in_bounds(nx, ny) {
                continue;
            }
            if n == Material::Water || n == Material::Acid {
                if is_lava {
                    self.set(x, y, Material::Stone);
                    self.set(nx, ny, Material::Steam);
                } else {
                    self.set(x, y, Material::Steam);
                }
                return true;
            }
            let flam = n.props().flammability;
            if flam > 0 && self.rng.chance(flam.saturating_add(extra_ignite)) {
                self.set(nx, ny, Material::Fire);
            }
        }
        if is_lava && self.rng.chance(2) && self.material(x, y - 1) == Material::Empty {
            self.set(x, y - 1, Material::Smoke);
        }
        false
    }

    /// Acid slowly dissolves soft and organic neighbours, using itself up.
    /// Stone resists it. Returns true if this cell was consumed.
    fn react_acid(&mut self, x: i32, y: i32) -> bool {
        for (dx, dy) in [(0, 1), (-1, 0), (1, 0), (0, -1)] {
            let (nx, ny) = (x + dx, y + dy);
            let n = self.material(nx, ny);
            let eats = n.is_diggable() || matches!(n, Material::Wood | Material::Web);
            if eats && self.in_bounds(nx, ny) && self.rng.chance(3) {
                self.set(nx, ny, Material::Smoke);
                if self.rng.coin() {
                    self.set(x, y, Material::Empty);
                    return true;
                }
            }
        }
        false
    }

    /// All cells, row-major.
    pub fn cells(&self) -> &[Cell] {
        &self.cells
    }

    // ---- change tracking -------------------------------------------------

    /// Returns and clears the chunks whose pixels changed since last call.
    pub fn take_render_dirty(&mut self) -> Vec<(usize, usize)> {
        let cx_n = self.chunks_x;
        self.chunks
            .iter_mut()
            .enumerate()
            .filter_map(|(i, c)| std::mem::take(&mut c.render_dirty).then_some((i % cx_n, i / cx_n)))
            .collect()
    }

    /// Returns and clears the chunks that changed since last replication.
    pub fn take_net_dirty(&mut self) -> Vec<(usize, usize)> {
        let cx_n = self.chunks_x;
        self.chunks
            .iter_mut()
            .enumerate()
            .filter_map(|(i, c)| std::mem::take(&mut c.net_dirty).then_some((i % cx_n, i / cx_n)))
            .collect()
    }

    /// Writes a chunk's pixels into an RGBA8 buffer covering the whole world.
    pub fn write_chunk_rgba(&self, cx: usize, cy: usize, buf: &mut [u8]) {
        for y in cy * CHUNK_SIZE..(cy + 1) * CHUNK_SIZE {
            for x in cx * CHUNK_SIZE..(cx + 1) * CHUNK_SIZE {
                let i = y * self.width + x;
                buf[i * 4..i * 4 + 4].copy_from_slice(&self.cells[i].rgba());
            }
        }
    }

    // ---- replication -----------------------------------------------------

    /// Run-length encodes a chunk as `[run, material, life]` triples.
    /// Shade is cosmetic and not sent.
    pub fn encode_chunk(&self, cx: usize, cy: usize) -> Vec<u8> {
        let mut out = Vec::with_capacity(256);
        let mut run: Option<(u8, u8, u8)> = None;
        for y in cy * CHUNK_SIZE..(cy + 1) * CHUNK_SIZE {
            for x in cx * CHUNK_SIZE..(cx + 1) * CHUNK_SIZE {
                let c = self.cells[y * self.width + x];
                let (m, l) = (c.mat as u8, c.life);
                run = match run {
                    Some((n, rm, rl)) if rm == m && rl == l && n < u8::MAX => Some((n + 1, m, l)),
                    Some((n, rm, rl)) => {
                        out.extend_from_slice(&[n, rm, rl]);
                        Some((1, m, l))
                    }
                    None => Some((1, m, l)),
                };
            }
        }
        if let Some((n, m, l)) = run {
            out.extend_from_slice(&[n, m, l]);
        }
        out
    }

    /// Replaces a chunk with data from [`World::encode_chunk`]. Cells whose
    /// material didn't change keep their local shade.
    pub fn decode_chunk(&mut self, cx: usize, cy: usize, data: &[u8]) -> Result<(), DecodeError> {
        if cx >= self.chunks_x || cy >= self.chunks_y || !data.len().is_multiple_of(3) {
            return Err(DecodeError);
        }
        let (runs, _) = data.as_chunks::<3>();
        let total: usize = runs.iter().map(|r| r[0] as usize).sum();
        if total != CHUNK_SIZE * CHUNK_SIZE {
            return Err(DecodeError);
        }
        let mut n = 0;
        for run in runs {
            let (mat, life) = (Material::from_u8(run[1]), run[2]);
            for _ in 0..run[0] {
                let x = cx * CHUNK_SIZE + n % CHUNK_SIZE;
                let y = cy * CHUNK_SIZE + n / CHUNK_SIZE;
                let cell = &mut self.cells[y * self.width + x];
                if cell.mat != mat {
                    cell.mat = mat;
                    cell.shade = hash2(self.seed, x as i32, y as i32) as u8;
                }
                cell.life = life;
                n += 1;
            }
        }
        self.chunks[cy * self.chunks_x + cx].render_dirty = true;
        Ok(())
    }
}

const NEIGHBOURS: [(i32, i32); 8] = [
    (-1, -1),
    (0, -1),
    (1, -1),
    (-1, 0),
    (1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sand_falls_and_piles() {
        let mut w = World::new(64, 64, 1);
        w.set(10, 0, Material::Sand);
        for _ in 0..100 {
            w.step();
        }
        assert_eq!(w.material(10, 63), Material::Sand);
        assert_eq!(w.material(10, 0), Material::Empty);
    }

    #[test]
    fn water_spreads_flat() {
        let mut w = World::new(64, 64, 2);
        w.paint_circle(32, 10, 3, Material::Water);
        for _ in 0..400 {
            w.step();
        }
        // All water should have settled into the bottom row(s).
        for y in 0..60 {
            for x in 0..64 {
                assert_ne!(w.material(x, y), Material::Water, "water floating at {x},{y}");
            }
        }
    }

    #[test]
    fn chunks_fall_asleep() {
        let mut w = World::new(128, 128, 3);
        w.set(5, 120, Material::Stone);
        for _ in 0..3 {
            w.step();
        }
        w.take_net_dirty();
        w.step();
        assert!(w.take_net_dirty().is_empty());
    }

    #[test]
    fn chunk_codec_roundtrip() {
        let mut a = World::new(128, 64, 4);
        a.paint_circle(70, 30, 10, Material::Sand);
        a.set(65, 5, Material::Fire);
        let mut b = World::new(128, 64, 4);
        for cy in 0..a.chunks_y() {
            for cx in 0..a.chunks_x() {
                b.decode_chunk(cx, cy, &a.encode_chunk(cx, cy)).unwrap();
            }
        }
        for y in 0..64 {
            for x in 0..128 {
                let (ca, cb) = (a.get(x, y).unwrap(), b.get(x, y).unwrap());
                assert_eq!((ca.mat, ca.life), (cb.mat, cb.life));
            }
        }
    }

    #[test]
    fn decode_rejects_garbage() {
        let mut w = World::new(64, 64, 5);
        assert!(w.decode_chunk(0, 0, &[1, 2]).is_err());
        assert!(w.decode_chunk(0, 0, &[10, 1, 0]).is_err());
        assert!(w.decode_chunk(9, 0, &[]).is_err());
    }
}
