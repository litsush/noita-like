use crate::color::cell_rgba;
use crate::material::{GRAVEL_SETTLED, INDESTRUCTIBLE, Kind, MAX_CHARGE, Material};
use crate::rng::{Rng, hash2};

/// Side length of a chunk in cells. Chunks are the unit of sleeping,
/// rendering updates, and network replication.
pub const CHUNK_SIZE: usize = 64;
/// Random cells visited per chunk per tick, sleeping or not, for slow
/// processes like fungus growth and melting.
const RANDOM_TICKS_PER_CHUNK: usize = 12;
const MAX_PARTICLES: usize = 6000;
/// Chain reactions carry over to later ticks beyond this.
const MAX_EXPLOSIONS_PER_TICK: usize = 8;
const PARTICLE_GRAVITY: f32 = 0.12;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Cell {
    pub mat: Material,
    /// Per-cell brightness variation so materials look grainy.
    pub shade: u8,
    /// Material-specific state: lifetime for fire and gases, electric charge
    /// for conductors, collapse countdown for gravel, generations for frost,
    /// burst timer for vents. Replicated with the chunk.
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
}

/// Something noteworthy happened in the simulation this tick. The game uses
/// these for sound, particles, damage and achievements.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimEvent {
    Explosion {
        x: i32,
        y: i32,
        radius: i32,
        destroyed: u32,
    },
    /// Water hit lava or molten metal.
    Sizzle {
        x: i32,
        y: i32,
    },
    /// Acid ate a cell.
    Dissolve {
        x: i32,
        y: i32,
    },
    ObsidianFormed {
        x: i32,
        y: i32,
    },
    GasIgnited {
        x: i32,
        y: i32,
    },
    /// Settled gravel was disturbed and will fall shortly.
    CollapseStarted {
        x: i32,
        y: i32,
    },
    /// A conductive cell became charged.
    Electrified {
        x: i32,
        y: i32,
    },
    /// Something caught fire.
    Ignited {
        x: i32,
        y: i32,
    },
}

/// A cell in ballistic flight (explosion debris, sprays). It lands back into
/// the grid when it hits something.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub mat: Material,
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
    events: Vec<SimEvent>,
    particles: Vec<Particle>,
    pending_explosions: Vec<(i32, i32, i32)>,
    /// Chunk rectangle (inclusive) to simulate; `None` simulates everything.
    region: Option<(usize, usize, usize, usize)>,
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
const ADJACENT: [(i32, i32); 4] = [(0, -1), (-1, 0), (1, 0), (0, 1)];

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
            events: Vec::new(),
            particles: Vec::new(),
            pending_explosions: Vec::new(),
            region: None,
        }
    }

    /// Limits simulation to chunks within `radius` chunks of the given cells
    /// (e.g. around each player). Chunks outside keep their pending wake-ups
    /// and resume when the region reaches them. `None` simulates everything.
    pub fn set_active_region(&mut self, centers: Option<&[(i32, i32)]>, radius: usize) {
        self.region = centers.and_then(|c| {
            let cs = CHUNK_SIZE as i32;
            let xs = c
                .iter()
                .map(|p| (p.0 / cs).clamp(0, self.chunks_x as i32 - 1) as usize);
            let ys = c
                .iter()
                .map(|p| (p.1 / cs).clamp(0, self.chunks_y as i32 - 1) as usize);
            let (x0, x1) = (xs.clone().min()?, xs.max()?);
            let (y0, y1) = (ys.clone().min()?, ys.max()?);
            Some((
                x0.saturating_sub(radius),
                y0.saturating_sub(radius),
                (x1 + radius).min(self.chunks_x - 1),
                (y1 + radius).min(self.chunks_y - 1),
            ))
        });
    }

    #[inline]
    fn in_region(&self, cx: usize, cy: usize) -> bool {
        self.region
            .is_none_or(|(x0, y0, x1, y1)| cx >= x0 && cx <= x1 && cy >= y0 && cy <= y1)
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
    pub fn particles(&self) -> &[Particle] {
        &self.particles
    }

    /// Returns and clears this tick's (and earlier undrained) events.
    pub fn take_events(&mut self) -> Vec<SimEvent> {
        std::mem::take(&mut self.events)
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

    /// Material at a position; out of bounds reads as core shell (the world is walled).
    #[inline]
    pub fn material(&self, x: i32, y: i32) -> Material {
        self.get(x, y).map_or(Material::CoreShell, |c| c.mat)
    }

    #[inline]
    pub fn is_solid(&self, x: i32, y: i32) -> bool {
        self.material(x, y).is_solid_for_player()
    }

    /// Places a fresh cell of `mat`, giving it a random shade.
    pub fn set(&mut self, x: i32, y: i32, mat: Material) {
        let life = mat.initial_life();
        self.set_with_life(x, y, mat, life);
    }

    pub fn set_with_life(&mut self, x: i32, y: i32, mat: Material, life: u8) {
        if !self.in_bounds(x, y) {
            return;
        }
        let shade = self.rng.next_u8();
        let i = self.idx(x, y);
        self.cells[i] = Cell {
            mat,
            shade,
            life,
            clock: self.clock,
        };
        self.mark_changed(x, y);
    }

    /// Fills a disc. `Material::Empty` digs everything; other materials only
    /// fill open cells so painting doesn't overwrite terrain.
    /// Returns true if anything changed.
    pub fn paint_circle(&mut self, cx: i32, cy: i32, radius: i32, mat: Material) -> bool {
        let mut changed = false;
        for (x, y) in disc(cx, cy, radius) {
            if !self.in_bounds(x, y) {
                continue;
            }
            let cur = self.material(x, y);
            let replace = if mat == Material::Empty {
                cur != Material::Empty && cur.props().hardness != INDESTRUCTIBLE
            } else {
                cur.is_open() && cur != mat
            };
            if replace {
                self.set(x, y, mat);
                changed = true;
            }
        }
        changed
    }

    /// Digs a disc with a pick of the given `power`. Harder materials are less
    /// likely to break per call; liquids, gases and indestructible cells are
    /// skipped. Returns what was removed.
    pub fn dig(&mut self, cx: i32, cy: i32, radius: i32, power: u8) -> Vec<(i32, i32, Material)> {
        let mut out = Vec::new();
        for (x, y) in disc(cx, cy, radius) {
            let m = self.material(x, y);
            if !matches!(m.kind(), Kind::Solid | Kind::Powder) {
                continue;
            }
            let hardness = m.props().hardness;
            if hardness == INDESTRUCTIBLE {
                continue;
            }
            let p = power as f32 / (power as f32 + hardness as f32 * 1.5);
            if self.rng.next_f32() < p {
                self.set(x, y, Material::Empty);
                out.push((x, y, m));
            }
        }
        out
    }

    /// Intense heat in a disc (the Pocket Sun): lava cooks into obsidian,
    /// water boils, ice melts and flammables ignite.
    pub fn cook(&mut self, cx: i32, cy: i32, radius: i32) {
        for (x, y) in disc(cx, cy, radius) {
            match self.material(x, y) {
                Material::Lava => {
                    self.set(x, y, Material::Obsidian);
                    self.events.push(SimEvent::ObsidianFormed { x, y });
                }
                Material::Water => {
                    if self.rng.chance(60) {
                        self.set(x, y, Material::Steam);
                        self.events.push(SimEvent::Sizzle { x, y });
                    }
                }
                Material::Ice | Material::Frost => self.set(x, y, Material::Water),
                m if m.props().flammability > 0 && self.rng.chance(m.props().flammability / 4 + 1) => {
                    self.ignite(x, y);
                }
                _ => {}
            }
        }
    }

    /// Shakes settled gravel loose within `radius` (it will start to fall).
    pub fn disturb(&mut self, cx: i32, cy: i32, radius: i32) {
        for (x, y) in disc(cx, cy, radius) {
            if let Some(c) = self.get(x, y)
                && c.mat == Material::Gravel
                && c.life == GRAVEL_SETTLED
            {
                let i = self.idx(x, y);
                self.cells[i].life = 4 + self.rng.next_u8() % 30;
                self.touch(x, y);
                self.events.push(SimEvent::CollapseStarted { x, y });
            }
        }
    }

    /// Charges a conductive cell (and anything conductive next to it).
    pub fn electrify(&mut self, x: i32, y: i32, charge: u8) {
        for (cx, cy) in disc(x, y, 1) {
            if let Some(c) = self.get(cx, cy)
                && c.mat.props().conductive
                && c.life < charge
            {
                let i = self.idx(cx, cy);
                self.cells[i].life = charge;
                self.touch(cx, cy);
                self.events.push(SimEvent::Electrified { x: cx, y: cy });
            }
        }
    }

    /// Total heat of materials within `radius` (negative near ice).
    pub fn heat_near(&self, cx: i32, cy: i32, radius: i32) -> i32 {
        disc(cx, cy, radius)
            .map(|(x, y)| self.get(x, y).map_or(0, |c| c.mat.props().heat as i32))
            .sum()
    }

    /// Wakes and flags the chunk for a change that doesn't alter the material
    /// (lifetimes, charge, countdowns).
    #[inline]
    fn touch(&mut self, x: i32, y: i32) {
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

    /// A cell's material changed: wake the area and disturb settled gravel
    /// next to it, which is what starts cave-ins.
    fn mark_changed(&mut self, x: i32, y: i32) {
        self.touch(x, y);
        for (dx, dy) in ADJACENT {
            let (nx, ny) = (x + dx, y + dy);
            if !self.in_bounds(nx, ny) {
                continue;
            }
            let i = self.idx(nx, ny);
            let c = &mut self.cells[i];
            if c.mat == Material::Gravel && c.life == GRAVEL_SETTLED {
                c.life = 6 + self.rng.next_u8() % 18;
                self.touch(nx, ny);
                self.events.push(SimEvent::CollapseStarted { x: nx, y: ny });
            }
        }
    }

    /// Marks all gravel as settled. World generation calls this last so
    /// pre-placed gravel waits for the player to disturb it.
    pub fn settle_gravel(&mut self) {
        for c in &mut self.cells {
            if c.mat == Material::Gravel {
                c.life = GRAVEL_SETTLED;
            }
        }
    }

    /// Advances the simulation by one tick.
    pub fn step(&mut self) {
        self.tick += 1;
        self.clock = self.clock.wrapping_add(1);
        for i in 0..self.chunks.len() {
            let (cx, cy) = (i % self.chunks_x, i / self.chunks_x);
            let inside = self.in_region(cx, cy);
            let c = &mut self.chunks[i];
            c.active = inside && std::mem::take(&mut c.active_next);
        }
        let (y_top, y_bottom) = match self.region {
            Some((_, y0, _, y1)) => (y0 * CHUNK_SIZE, (y1 + 1) * CHUNK_SIZE),
            None => (0, self.height),
        };

        // Bottom-up so falling cells move once per tick; alternate horizontal
        // direction per row and tick to avoid a directional bias.
        for y in (y_top as i32..y_bottom as i32).rev() {
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

        self.random_ticks();
        self.step_particles();
        self.process_explosions();
    }

    fn update_cell(&mut self, x: i32, y: i32) {
        let cell = self.cells[self.idx(x, y)];
        if cell.clock == self.clock {
            return;
        }
        match cell.mat.kind() {
            Kind::Empty => {}
            Kind::Solid => {
                if cell.life > 0 {
                    self.update_active_solid(x, y, cell);
                }
            }
            Kind::Powder => self.update_powder(x, y, cell),
            Kind::Liquid => self.update_liquid(x, y, cell),
            Kind::Gas => self.update_gas(x, y, cell),
            Kind::Fire => self.update_fire(x, y, cell),
            Kind::Energy => self.update_spark(x, y, cell),
        }
    }

    /// Can a cell of `src` move into a cell occupied by `dst`?
    #[inline]
    fn can_displace(src: Material, dst: Material) -> bool {
        match dst.kind() {
            Kind::Empty => true,
            Kind::Liquid | Kind::Gas | Kind::Fire | Kind::Energy => dst.props().density < src.props().density,
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

    #[inline]
    fn coin_dir(&mut self) -> i32 {
        if self.rng.coin() { 1 } else { -1 }
    }

    fn update_powder(&mut self, x: i32, y: i32, cell: Cell) {
        if cell.mat == Material::Gravel {
            if cell.life == GRAVEL_SETTLED {
                return;
            }
            if cell.life > 0 {
                // Disturbed: creak for a moment before letting go.
                let i = self.idx(x, y);
                self.cells[i].life -= 1;
                self.touch(x, y);
                return;
            }
        }
        if self.try_move(x, y, x, y + 1) {
            return;
        }
        let d = self.coin_dir();
        let _ = self.try_move(x, y, x + d, y + 1) || self.try_move(x, y, x - d, y + 1);
    }

    fn update_liquid(&mut self, x: i32, y: i32, cell: Cell) {
        let props = cell.mat.props();
        if props.conductive && cell.life > 0 {
            self.propagate_charge(x, y, cell);
        }
        if props.heat > 0 && self.react_hot(x, y) {
            return;
        }
        if props.corrosive > 0 && self.corrode(x, y, props.corrosive) {
            return;
        }
        if self.try_move(x, y, x, y + 1) {
            return;
        }
        let d = self.coin_dir();
        if self.try_move(x, y, x + d, y + 1) || self.try_move(x, y, x - d, y + 1) {
            return;
        }
        let reach = props.dispersion as i32;
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
        if !cell.mat.is_persistent_gas() {
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
            let i = self.idx(x, y);
            self.cells[i].life = cell.life;
            self.touch(x, y);
        } else if !self.rng.chance(110) {
            // Heavy gas drifts lazily. It only keeps its chunk awake while
            // something actually moves, so settled pockets go to sleep.
            if self.material(x, y - 1).is_open() && self.material(x, y - 1) != Material::Gas {
                self.touch(x, y);
            }
            return;
        }

        let d = self.coin_dir();
        if self.rng.chance(200) && self.try_move(x, y, x, y - 1) {
            return;
        }
        let _ = self.try_move(x, y, x + d, y - 1)
            || self.try_move(x, y, x - d, y - 1)
            || self.try_move(x, y, x + d, y);
    }

    fn update_fire(&mut self, x: i32, y: i32, cell: Cell) {
        let burn = 1 + (self.rng.next_u8() & 1);
        if cell.life <= burn {
            let next = match self.rng.next_u8() {
                0..40 => Material::Smoke,
                40..52 => Material::Ash,
                _ => Material::Empty,
            };
            self.set(x, y, next);
            return;
        }
        let i = self.idx(x, y);
        self.cells[i].life = cell.life - burn;
        self.touch(x, y);

        if self.react_hot(x, y) {
            return;
        }
        // Fire with nothing left to burn drifts upward and dies; fire on fuel stays put.
        let fuelled = NEIGHBOURS
            .iter()
            .any(|(dx, dy)| self.material(x + dx, y + dy).props().flammability > 0);
        if !fuelled && self.rng.chance(120) {
            let d = (self.rng.next_u8() % 3) as i32 - 1;
            if self.material(x + d, y - 1) == Material::Empty {
                self.try_move(x, y, x + d, y - 1);
            }
        }
    }

    fn update_spark(&mut self, x: i32, y: i32, cell: Cell) {
        if cell.life <= 1 {
            self.set(x, y, Material::Empty);
            return;
        }
        let i = self.idx(x, y);
        self.cells[i].life = cell.life - 1;
        self.touch(x, y);

        for (dx, dy) in NEIGHBOURS {
            let (nx, ny) = (x + dx, y + dy);
            let Some(n) = self.get(nx, ny) else { continue };
            let props = n.mat.props();
            if props.conductive {
                if n.life < MAX_CHARGE {
                    let j = self.idx(nx, ny);
                    self.cells[j].life = MAX_CHARGE;
                    self.touch(nx, ny);
                    self.events.push(SimEvent::Electrified { x: nx, y: ny });
                }
                self.set(x, y, Material::Empty);
                return;
            }
            if props.flammability > 0 && self.rng.chance(props.flammability / 2 + 8) {
                self.ignite(nx, ny);
            }
        }
        // Skitter, mostly downward.
        let dx = (self.rng.next_u8() % 3) as i32 - 1;
        let dy = if self.rng.chance(170) { 1 } else { -1 };
        if self.material(x + dx, y + dy) == Material::Empty {
            self.try_move(x, y, x + dx, y + dy);
        }
    }

    /// Solids with a non-zero `life` have something going on: vent bursts,
    /// spreading frost, or electric charge.
    fn update_active_solid(&mut self, x: i32, y: i32, cell: Cell) {
        let i = self.idx(x, y);
        match cell.mat {
            Material::Vent => {
                if self.material(x, y - 1).is_open() && self.rng.chance(180) {
                    self.set(x, y - 1, Material::Steam);
                }
                self.cells[i].life -= 1;
                self.touch(x, y);
            }
            Material::Frost => {
                for (dx, dy) in ADJACENT {
                    let (nx, ny) = (x + dx, y + dy);
                    match self.material(nx, ny) {
                        Material::Water if cell.life > 1 => {
                            self.set_with_life(nx, ny, Material::Frost, cell.life - 1)
                        }
                        Material::Water | Material::Steam => self.set(nx, ny, Material::Ice),
                        Material::Lava => {
                            self.set(nx, ny, Material::Obsidian);
                            self.events.push(SimEvent::ObsidianFormed { x: nx, y: ny });
                        }
                        Material::Empty if cell.life > 4 && self.rng.chance(70) => {
                            self.set(nx, ny, Material::Ice)
                        }
                        Material::Fire => self.set(nx, ny, Material::Smoke),
                        _ => {}
                    }
                }
                self.set(x, y, Material::Ice);
            }
            m if m.props().conductive => self.propagate_charge(x, y, cell),
            // Stray state on inert solids means nothing.
            _ => self.cells[i].life = 0,
        }
    }

    /// Spreads electric charge to conductive neighbours (one less each hop)
    /// and lets it decay here. Charge ignites gas it touches.
    fn propagate_charge(&mut self, x: i32, y: i32, cell: Cell) {
        let charge = cell.life;
        for (dx, dy) in ADJACENT {
            let (nx, ny) = (x + dx, y + dy);
            let Some(n) = self.get(nx, ny) else { continue };
            if n.mat.props().conductive && n.life + 1 < charge {
                let j = self.idx(nx, ny);
                self.cells[j].life = charge - 1;
                self.touch(nx, ny);
                self.events.push(SimEvent::Electrified { x: nx, y: ny });
            } else if n.mat == Material::Gas && self.rng.chance(60) {
                self.ignite(nx, ny);
            }
        }
        let i = self.idx(x, y);
        self.cells[i].life = charge - 1;
        self.touch(x, y);
    }

    /// Sets a cell alight. Explosives detonate; gas flashes and sometimes blasts.
    pub fn ignite(&mut self, x: i32, y: i32) {
        let m = self.material(x, y);
        let props = m.props();
        if props.explosive > 0 {
            self.set(x, y, Material::Fire);
            self.pending_explosions.push((x, y, props.explosive as i32));
        } else if m == Material::Gas {
            let life = 25 + self.rng.next_u8() % 20;
            self.set_with_life(x, y, Material::Fire, life);
            self.events.push(SimEvent::GasIgnited { x, y });
            if self.rng.chance(14) {
                self.pending_explosions.push((x, y, 4));
            }
        } else if props.flammability > 0 {
            self.set(x, y, Material::Fire);
            self.events.push(SimEvent::Ignited { x, y });
        }
    }

    /// Shared behaviour for hot cells (fire, lava, molten metal): react with
    /// water and ice, and ignite flammable neighbours.
    /// Returns true if this cell was consumed or transformed.
    fn react_hot(&mut self, x: i32, y: i32) -> bool {
        let me = self.material(x, y);
        for (dx, dy) in NEIGHBOURS {
            let (nx, ny) = (x + dx, y + dy);
            if !self.in_bounds(nx, ny) {
                continue;
            }
            let n = self.material(nx, ny);
            match (me, n) {
                (Material::Lava, Material::Water) => {
                    self.set(x, y, Material::Obsidian);
                    self.set(nx, ny, Material::Steam);
                    self.events.push(SimEvent::Sizzle { x, y });
                    self.events.push(SimEvent::ObsidianFormed { x, y });
                    return true;
                }
                (Material::Metal, Material::Water) => {
                    self.set(nx, ny, Material::Steam);
                    self.events.push(SimEvent::Sizzle { x, y });
                    if self.rng.chance(64) {
                        self.set(x, y, Material::Ferrite);
                    }
                    return true;
                }
                (Material::Fire, Material::Water) => {
                    self.set(x, y, Material::Steam);
                    return true;
                }
                (Material::Lava, Material::Ice | Material::Frost) => {
                    self.set(x, y, Material::Obsidian);
                    self.set(nx, ny, Material::Water);
                    self.events.push(SimEvent::ObsidianFormed { x, y });
                    return true;
                }
                (_, Material::Ice | Material::Frost) => {
                    if self.rng.chance(30) {
                        self.set(nx, ny, Material::Water);
                    }
                }
                _ => {
                    let flam = n.props().flammability;
                    let bonus = if me == Material::Fire { 0 } else { 12 };
                    if flam > 0 && self.rng.chance(flam.saturating_add(bonus)) {
                        self.ignite(nx, ny);
                    }
                }
            }
        }
        if me == Material::Lava && self.rng.chance(2) && self.material(x, y - 1) == Material::Empty {
            self.set(x, y - 1, Material::Smoke);
        }
        false
    }

    /// Acid eats a neighbouring solid (and is usually used up doing it).
    /// Returns true if this cell changed.
    fn corrode(&mut self, x: i32, y: i32, strength: u8) -> bool {
        let start = (self.rng.next_u8() % 4) as usize;
        for k in 0..4 {
            let (dx, dy) = ADJACENT[(start + k) % 4];
            let (nx, ny) = (x + dx, y + dy);
            if !self.in_bounds(nx, ny) {
                continue;
            }
            let n = self.material(nx, ny);
            let props = n.props();
            if n == Material::Water {
                if self.rng.chance(3) {
                    self.set(x, y, Material::Water);
                    return true;
                }
                continue;
            }
            if !matches!(props.kind, Kind::Solid | Kind::Powder)
                || props.acid_resistant
                || props.hardness == INDESTRUCTIBLE
            {
                continue;
            }
            let p = strength as u32 * (300 - props.hardness as u32) / 300;
            if self.rng.chance(p.min(255) as u8) {
                let fumes = if self.rng.chance(50) {
                    Material::Smoke
                } else {
                    Material::Empty
                };
                self.set(nx, ny, fumes);
                self.events.push(SimEvent::Dissolve { x: nx, y: ny });
                if self.rng.coin() {
                    self.set(x, y, Material::Empty);
                    return true;
                }
            }
        }
        false
    }

    // ---- slow processes --------------------------------------------------

    fn random_ticks(&mut self) {
        for cy in 0..self.chunks_y {
            for cx in 0..self.chunks_x {
                if !self.in_region(cx, cy) {
                    continue;
                }
                for _ in 0..RANDOM_TICKS_PER_CHUNK {
                    let r = self.rng.next_u64();
                    let x = (cx * CHUNK_SIZE) as i32 + (r & 63) as i32;
                    let y = (cy * CHUNK_SIZE) as i32 + ((r >> 8) & 63) as i32;
                    self.random_tick(x, y);
                }
            }
        }
    }

    fn random_tick(&mut self, x: i32, y: i32) {
        let cell = self.cells[self.idx(x, y)];
        match cell.mat {
            Material::Fungus => {
                let (dx, dy) = NEIGHBOURS[(self.rng.next_u8() % 8) as usize];
                let (nx, ny) = (x + dx, y + dy);
                if self.material(nx, ny) == Material::Empty
                    && self.rng.chance(Material::Fungus.props().growth)
                    && self.is_anchored(nx, ny)
                    && self.count_adjacent(nx, ny, Material::Fungus) <= 1
                {
                    self.set(nx, ny, Material::Fungus);
                }
            }
            Material::Ice => {
                if ADJACENT
                    .iter()
                    .any(|(dx, dy)| self.material(x + dx, y + dy).props().heat > 0)
                {
                    self.set(x, y, Material::Water);
                }
            }
            Material::Stone | Material::Basalt => {
                // Heat cracks rock next to lava.
                if self.rng.chance(60) && self.count_adjacent(x, y, Material::Lava) > 0 {
                    self.set(x, y, Material::Gravel);
                }
            }
            Material::Vent => {
                if cell.life == 0 && self.rng.chance(90) {
                    let i = self.idx(x, y);
                    self.cells[i].life = 30 + self.rng.next_u8() % 30;
                    self.touch(x, y);
                }
            }
            Material::Metal if self.material(x, y - 1) == Material::Empty && self.rng.chance(40) => {
                self.set(x, y - 1, Material::Spark);
            }
            _ => {}
        }
    }

    /// Next to a solid that isn't fungus (so fungus hugs surfaces).
    fn is_anchored(&self, x: i32, y: i32) -> bool {
        ADJACENT.iter().any(|(dx, dy)| {
            let m = self.material(x + dx, y + dy);
            m.kind() == Kind::Solid && m != Material::Fungus
        })
    }

    fn count_adjacent(&self, x: i32, y: i32, mat: Material) -> usize {
        ADJACENT
            .iter()
            .filter(|(dx, dy)| self.material(x + dx, y + dy) == mat)
            .count()
    }

    // ---- particles -------------------------------------------------------

    /// Launches a cell of `mat` from (x, y). Dropped if too many are in flight.
    pub fn spawn_particle(&mut self, x: f32, y: f32, vx: f32, vy: f32, mat: Material) {
        if self.particles.len() < MAX_PARTICLES && self.in_bounds(x as i32, y as i32) {
            self.particles.push(Particle { x, y, vx, vy, mat });
        }
    }

    fn step_particles(&mut self) {
        let mut i = 0;
        while i < self.particles.len() {
            let mut p = self.particles[i];
            p.vy = (p.vy + PARTICLE_GRAVITY).min(5.0);
            let steps = p.vx.abs().max(p.vy.abs()).ceil().max(1.0) as i32;
            let (sx, sy) = (p.vx / steps as f32, p.vy / steps as f32);
            let mut done = None;
            for _ in 0..steps {
                let (nx, ny) = (p.x + sx, p.y + sy);
                let (cx, cy) = (nx.floor() as i32, ny.floor() as i32);
                if !self.in_bounds(cx, cy) {
                    done = Some(false);
                    break;
                }
                if !self.material(cx, cy).is_open() {
                    done = Some(true);
                    break;
                }
                p.x = nx;
                p.y = ny;
            }
            match done {
                Some(landed) => {
                    if landed {
                        let (x, y) = (p.x.floor() as i32, p.y.floor() as i32);
                        if self.material(x, y).is_open() {
                            self.set(x, y, p.mat);
                        }
                    }
                    self.particles.swap_remove(i);
                }
                None => {
                    self.particles[i] = p;
                    i += 1;
                }
            }
        }
    }

    // ---- explosions ------------------------------------------------------

    /// Queues an explosion; it happens at the end of the current/next tick.
    pub fn explode(&mut self, x: i32, y: i32, radius: i32) {
        self.pending_explosions.push((x, y, radius));
    }

    fn process_explosions(&mut self) {
        let batch: Vec<_> = {
            let n = self.pending_explosions.len().min(MAX_EXPLOSIONS_PER_TICK);
            self.pending_explosions.drain(..n).collect()
        };
        for (x, y, r) in batch {
            self.detonate(x, y, r);
        }
    }

    fn detonate(&mut self, cx: i32, cy: i32, r: i32) {
        let mut destroyed = 0;
        let rf = r.max(1) as f32;
        for (x, y) in disc(cx, cy, r) {
            let Some(cell) = self.get(x, y) else { continue };
            let m = cell.mat;
            let props = m.props();
            if m == Material::Empty || props.hardness == INDESTRUCTIBLE {
                continue;
            }
            let (dx, dy) = ((x - cx) as f32, (y - cy) as f32);
            let d = (dx * dx + dy * dy).sqrt() / rf;
            let (nx, ny) = if d > 0.0 {
                (dx / (d * rf), dy / (d * rf))
            } else {
                (0.0, -1.0)
            };
            let speed = 1.2 + self.rng.next_f32() * 2.5;
            let fling = |w: &mut World, mat| {
                w.spawn_particle(x as f32 + 0.5, y as f32 + 0.5, nx * speed, ny * speed - 0.8, mat)
            };

            if props.explosive > 0 && (x, y) != (cx, cy) {
                self.ignite(x, y);
                continue;
            }
            match props.kind {
                Kind::Liquid => {
                    if self.rng.chance(90) {
                        self.set(x, y, Material::Empty);
                        fling(self, m);
                    }
                }
                Kind::Gas if m == Material::Gas => self.ignite(x, y),
                Kind::Solid | Kind::Powder => {
                    let strength = (1.0 - d * d * 0.8) * 1.4 - props.hardness as f32 / 255.0 * 0.9;
                    if self.rng.next_f32() >= strength {
                        continue;
                    }
                    destroyed += 1;
                    let roll = self.rng.next_u8();
                    let next = if roll < 30 {
                        let debris = match (props.kind, props.flammability > 0) {
                            (Kind::Powder, _) => m,
                            (_, true) => Material::Ash,
                            _ => Material::Gravel,
                        };
                        self.set(x, y, Material::Empty);
                        fling(self, debris);
                        continue;
                    } else if d < 0.5 && roll < 90 {
                        Material::Fire
                    } else if roll < 110 {
                        Material::Smoke
                    } else {
                        Material::Empty
                    };
                    self.set(x, y, next);
                }
                _ => {}
            }
        }
        // Embers.
        for _ in 0..r * 2 {
            let a = self.rng.next_f32() * std::f32::consts::TAU;
            let s = 1.0 + self.rng.next_f32() * 2.5;
            self.spawn_particle(
                cx as f32,
                cy as f32,
                a.cos() * s,
                a.sin() * s - 1.0,
                Material::Fire,
            );
        }
        self.events.push(SimEvent::Explosion {
            x: cx,
            y: cy,
            radius: r,
            destroyed,
        });
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

    /// Writes a chunk's pixels into an RGBA8 buffer `buf_width` pixels wide
    /// whose top-left corner is world cell `origin`. The chunk must lie inside it.
    pub fn write_chunk_rgba(
        &self,
        cx: usize,
        cy: usize,
        buf: &mut [u8],
        buf_width: usize,
        origin: (usize, usize),
        frame: u32,
    ) {
        for y in cy * CHUNK_SIZE..(cy + 1) * CHUNK_SIZE {
            let row = (y - origin.1) * buf_width;
            for x in cx * CHUNK_SIZE..(cx + 1) * CHUNK_SIZE {
                let o = (row + x - origin.0) * 4;
                let rgba = cell_rgba(self.cells[y * self.width + x], x as i32, y as i32, frame);
                buf[o..o + 4].copy_from_slice(&rgba);
            }
        }
    }

    /// Whether a chunk holds anything whose colour animates over time.
    pub fn chunk_animates(&self, cx: usize, cy: usize) -> bool {
        (cy * CHUNK_SIZE..(cy + 1) * CHUNK_SIZE).any(|y| {
            let row = &self.cells[y * self.width + cx * CHUNK_SIZE..y * self.width + (cx + 1) * CHUNK_SIZE];
            row.iter().any(|c| {
                matches!(
                    c.mat,
                    Material::Water
                        | Material::Lava
                        | Material::Acid
                        | Material::Metal
                        | Material::Fungus
                        | Material::Crystal
                        | Material::Gas
                        | Material::Frost
                        | Material::Oil
                )
            })
        })
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

/// Cells of a filled circle.
pub fn disc(cx: i32, cy: i32, r: i32) -> impl Iterator<Item = (i32, i32)> {
    (cy - r..=cy + r).flat_map(move |y| {
        (cx - r..=cx + r).filter_map(move |x| {
            let (dx, dy) = (x - cx, y - cy);
            (dx * dx + dy * dy <= r * r).then_some((x, y))
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(w: &mut World, ticks: usize) {
        for _ in 0..ticks {
            w.step();
        }
    }

    fn count(w: &World, mat: Material) -> usize {
        (0..w.height() as i32)
            .flat_map(|y| (0..w.width() as i32).map(move |x| (x, y)))
            .filter(|&(x, y)| w.material(x, y) == mat)
            .count()
    }

    fn fill(w: &mut World, x0: i32, y0: i32, x1: i32, y1: i32, mat: Material) {
        for y in y0..=y1 {
            for x in x0..=x1 {
                w.set(x, y, mat);
            }
        }
    }

    #[test]
    fn sand_falls_and_piles() {
        let mut w = World::new(64, 64, 1);
        w.set(10, 0, Material::Sand);
        run(&mut w, 100);
        assert_eq!(w.material(10, 63), Material::Sand);
        assert_eq!(w.material(10, 0), Material::Empty);
    }

    #[test]
    fn water_spreads_flat() {
        let mut w = World::new(64, 64, 2);
        w.paint_circle(32, 10, 3, Material::Water);
        run(&mut w, 400);
        for y in 0..60 {
            for x in 0..64 {
                assert_ne!(w.material(x, y), Material::Water, "water floating at {x},{y}");
            }
        }
    }

    #[test]
    fn heavy_sinks_through_lighter_liquid() {
        let mut w = World::new(64, 64, 3);
        fill(&mut w, 0, 50, 63, 63, Material::Water);
        w.set(20, 10, Material::Sand);
        w.set(40, 40, Material::Oil);
        w.set(40, 63, Material::Oil);
        run(&mut w, 200);
        assert_eq!(
            w.material(20, 63),
            Material::Sand,
            "sand should sink to the bottom"
        );
        let oil_on_top =
            (0..64).any(|x| w.material(x, 49) == Material::Oil || w.material(x, 50) == Material::Oil);
        assert!(oil_on_top, "oil should float on water");
    }

    #[test]
    fn water_and_lava_make_obsidian_and_steam() {
        let mut w = World::new(64, 64, 4);
        fill(&mut w, 10, 60, 20, 63, Material::Lava);
        fill(&mut w, 10, 50, 20, 52, Material::Water);
        run(&mut w, 60);
        let events = w.take_events();
        assert!(count(&w, Material::Obsidian) > 0);
        assert!(events.iter().any(|e| matches!(e, SimEvent::Sizzle { .. })));
        assert!(
            events
                .iter()
                .any(|e| matches!(e, SimEvent::ObsidianFormed { .. }))
        );
    }

    #[test]
    fn fire_ignites_flammables() {
        let mut w = World::new(64, 64, 5);
        fill(&mut w, 10, 50, 30, 63, Material::Wood);
        // Oil in a basin, so it can't spread away from the flame.
        fill(&mut w, 39, 58, 39, 63, Material::Stone);
        fill(&mut w, 51, 58, 51, 63, Material::Stone);
        fill(&mut w, 40, 60, 50, 63, Material::Oil);
        w.set(10, 49, Material::Fire);
        w.set(45, 59, Material::Fire);
        run(&mut w, 300);
        assert!(count(&w, Material::Wood) < 21 * 14, "wood should burn");
        assert!(count(&w, Material::Oil) < 11 * 4, "oil should burn");
    }

    #[test]
    fn ignited_gas_explodes() {
        let mut w = World::new(128, 128, 6);
        fill(&mut w, 0, 100, 127, 127, Material::Stone);
        fill(&mut w, 30, 60, 90, 99, Material::Gas);
        w.set(60, 99, Material::Fire);
        run(&mut w, 200);
        let events = w.take_events();
        assert!(events.iter().any(|e| matches!(e, SimEvent::GasIgnited { .. })));
        assert!(
            events.iter().any(|e| matches!(e, SimEvent::Explosion { .. })),
            "gas should blast"
        );
        assert!(count(&w, Material::Gas) < 100);
    }

    #[test]
    fn explosion_destroys_terrain_but_not_core_shell() {
        let mut w = World::new(64, 64, 7);
        fill(&mut w, 0, 0, 63, 63, Material::Stone);
        fill(&mut w, 30, 30, 34, 34, Material::CoreShell);
        w.explode(20, 20, 10);
        w.step();
        assert!(count(&w, Material::Stone) < 64 * 64 - 25 - 150);
        assert_eq!(count(&w, Material::CoreShell), 25);
        let destroyed = w.take_events().iter().find_map(|e| match e {
            SimEvent::Explosion { destroyed, .. } => Some(*destroyed),
            _ => None,
        });
        assert!(destroyed.unwrap() > 100);
    }

    #[test]
    fn explosives_chain() {
        let mut w = World::new(128, 64, 8);
        fill(&mut w, 0, 60, 127, 63, Material::Stone);
        fill(&mut w, 20, 55, 25, 59, Material::Explosive);
        fill(&mut w, 30, 55, 35, 59, Material::Explosive);
        w.set(20, 54, Material::Fire);
        run(&mut w, 120);
        assert_eq!(count(&w, Material::Explosive), 0);
        let blasts = w
            .take_events()
            .iter()
            .filter(|e| matches!(e, SimEvent::Explosion { .. }))
            .count();
        assert!(blasts >= 2);
    }

    #[test]
    fn acid_dissolves_solids_but_not_obsidian() {
        let mut w = World::new(64, 64, 9);
        fill(&mut w, 0, 40, 63, 63, Material::Obsidian);
        fill(&mut w, 20, 40, 40, 50, Material::Stone);
        fill(&mut w, 25, 30, 35, 39, Material::Acid);
        run(&mut w, 600);
        assert!(count(&w, Material::Stone) < 21 * 11, "acid should eat stone");
        assert_eq!(
            count(&w, Material::Obsidian),
            64 * 24 - 21 * 11,
            "obsidian is acid-proof"
        );
        assert!(count(&w, Material::Acid) < 110, "acid is consumed");
        assert!(
            w.take_events()
                .iter()
                .any(|e| matches!(e, SimEvent::Dissolve { .. }))
        );
    }

    #[test]
    fn heat_melts_ice() {
        let mut w = World::new(64, 64, 10);
        fill(&mut w, 0, 62, 63, 63, Material::Stone);
        fill(&mut w, 20, 55, 30, 61, Material::Ice);
        w.set(25, 54, Material::Fire);
        fill(&mut w, 31, 58, 33, 61, Material::Lava);
        run(&mut w, 200);
        assert!(count(&w, Material::Ice) < 77);
        assert!(
            count(&w, Material::Water) + count(&w, Material::Steam) > 0 || count(&w, Material::Obsidian) > 0
        );
    }

    #[test]
    fn sparks_electrify_water() {
        let mut w = World::new(64, 64, 11);
        fill(&mut w, 0, 50, 63, 63, Material::Water);
        w.set(10, 49, Material::Spark);
        run(&mut w, 6);
        let charged = (0..64)
            .flat_map(|x| (50..64).map(move |y| (x, y)))
            .filter(|&(x, y)| w.get(x, y).unwrap().life > 0)
            .count();
        assert!(charged > 10, "charge should spread through water, got {charged}");
        run(&mut w, 80);
        assert!((0..64).all(|x| w.get(x, 55).unwrap().life == 0), "charge decays");
        assert!(
            w.take_events()
                .iter()
                .any(|e| matches!(e, SimEvent::Electrified { .. }))
        );
    }

    #[test]
    fn settled_gravel_holds_until_disturbed() {
        let mut w = World::new(64, 64, 12);
        fill(&mut w, 0, 20, 63, 29, Material::Gravel);
        w.settle_gravel();
        w.take_events();
        run(&mut w, 100);
        assert_eq!(count(&w, Material::Gravel), 64 * 10);
        assert_eq!(w.material(30, 20), Material::Gravel, "ceiling holds");

        w.set(30, 29, Material::Empty);
        run(&mut w, 1500);
        assert!(
            w.take_events()
                .iter()
                .any(|e| matches!(e, SimEvent::CollapseStarted { .. }))
        );
        assert!(
            (0..64).filter(|&x| w.material(x, 63) == Material::Gravel).count() > 40,
            "disturbed gravel should collapse to the floor"
        );
    }

    #[test]
    fn frost_freezes_water() {
        let mut w = World::new(64, 64, 13);
        fill(&mut w, 0, 50, 63, 63, Material::Water);
        w.set(30, 49, Material::Frost);
        run(&mut w, 60);
        assert!(count(&w, Material::Ice) > 15);
    }

    #[test]
    fn digging_respects_hardness() {
        let mut w = World::new(64, 64, 14);
        fill(&mut w, 0, 0, 30, 63, Material::Dirt);
        fill(&mut w, 31, 0, 63, 63, Material::Obsidian);
        fill(&mut w, 0, 0, 5, 5, Material::CoreShell);
        let mut dirt = 0;
        let mut obsidian = 0;
        for i in 0..20 {
            dirt += w.dig(15, 10 + i * 2, 3, 40).len();
            obsidian += w.dig(48, 10 + i * 2, 3, 40).len();
            w.dig(2, 2, 3, 255);
        }
        assert!(dirt > obsidian * 2, "dirt {dirt} vs obsidian {obsidian}");
        assert_eq!(count(&w, Material::CoreShell), 36);
    }

    #[test]
    fn fungus_grows_over_time() {
        let mut w = World::new(64, 64, 15);
        fill(&mut w, 0, 60, 63, 63, Material::Stone);
        for x in [8, 24, 40, 56] {
            w.set(x, 59, Material::Fungus);
        }
        run(&mut w, 4000);
        assert!(
            count(&w, Material::Fungus) > 6,
            "got {}",
            count(&w, Material::Fungus)
        );
    }

    #[test]
    fn particles_land_back_in_the_world() {
        let mut w = World::new(64, 64, 16);
        fill(&mut w, 0, 60, 63, 63, Material::Stone);
        w.spawn_particle(10.0, 10.0, 1.0, -1.0, Material::Sand);
        run(&mut w, 200);
        assert!(w.particles().is_empty());
        assert_eq!(count(&w, Material::Sand), 1);
    }

    #[test]
    fn region_limits_simulation_but_keeps_wakeups() {
        let mut w = World::new(64, 512, 20);
        w.set(10, 10, Material::Sand);
        w.set(10, 400, Material::Sand);
        w.set_active_region(Some(&[(10, 10)]), 1);
        run(&mut w, 100);
        assert_eq!(w.material(10, 400), Material::Sand, "far sand is frozen");
        assert_ne!(w.material(10, 10), Material::Sand, "near sand fell");
        w.set_active_region(Some(&[(10, 400)]), 1);
        run(&mut w, 200);
        assert_eq!(
            w.material(10, 511),
            Material::Sand,
            "far sand resumes when the region arrives"
        );
    }

    #[test]
    fn settled_gas_lets_chunks_sleep() {
        let mut w = World::new(64, 64, 21);
        fill(&mut w, 0, 0, 63, 63, Material::Stone);
        fill(&mut w, 10, 10, 40, 30, Material::Gas);
        run(&mut w, 200);
        w.take_net_dirty();
        run(&mut w, 5);
        assert!(w.take_net_dirty().is_empty(), "sealed gas pocket should sleep");
    }

    #[test]
    fn chunks_fall_asleep() {
        let mut w = World::new(128, 128, 17);
        w.set(5, 120, Material::Stone);
        run(&mut w, 3);
        w.take_net_dirty();
        w.step();
        assert!(w.take_net_dirty().is_empty());
    }

    #[test]
    fn chunk_codec_roundtrip() {
        let mut a = World::new(128, 64, 18);
        a.paint_circle(70, 30, 10, Material::Sand);
        a.set(65, 5, Material::Fire);
        let mut b = World::new(128, 64, 18);
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
        let mut w = World::new(64, 64, 19);
        assert!(w.decode_chunk(0, 0, &[1, 2]).is_err());
        assert!(w.decode_chunk(0, 0, &[10, 1, 0]).is_err());
        assert!(w.decode_chunk(9, 0, &[]).is_err());
    }
}
