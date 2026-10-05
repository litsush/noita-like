//! Alien ecosystem sandbox: a procedurally generated biome, its plants, a
//! prey species and a predator species living on the falling-sand world.
//! Deterministic for a given seed.

pub mod biome;
pub mod body;
pub mod brain;
pub mod creature;
pub mod flora;
pub mod genome;
pub mod math;
pub mod names;
pub mod nav;
pub mod physics;
pub mod render;

use biome::{Biome, HEIGHT, WIDTH, WeatherKind};
use brain::{Action, BehaviorKind, Ctx, Snap};
use creature::{Carcass, Creature, Egg};
use flora::{FloraSpecies, Plant};
use genome::{Habitat, Locomotion, Role, Species, SpitKind};
use math::{Rgb, V2, mix, scale, v2};
use nav::NavGrid;
use physics::Mode;

use crate::material::{Kind, Material};
use crate::rng::Rng;
use crate::world::World;

/// Seconds per simulation step.
pub const DT: f32 = 1.0 / 60.0;
/// Population limits keep the scene readable and the frame rate steady.
pub const PREY_CAP: usize = 36;
pub const PREDATOR_CAP: usize = 12;
pub const PLANT_CAP: usize = 170;

#[derive(Clone, Debug)]
pub struct Projectile {
    pub kind: SpitKind,
    pub pos: V2,
    pub vel: V2,
    pub owner: u32,
    pub owner_species: usize,
    pub life: f32,
    pub damage: f32,
}

#[derive(Clone, Debug)]
pub struct Particle {
    pub pos: V2,
    pub vel: V2,
    pub life: f32,
    pub max_life: f32,
    pub color: Rgb,
    pub glow: bool,
    pub gravity: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IconKind {
    Alert,
    Sleep,
    Heart,
    Skull,
    Food,
}

#[derive(Clone, Debug)]
pub struct Icon {
    pub pos: V2,
    pub kind: IconKind,
    pub life: f32,
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub time: f32,
    pub text: String,
}

#[derive(Clone, Debug)]
pub struct Weather {
    pub kind: WeatherKind,
    pub timer: f32,
    pub intensity: f32,
    pub wind: f32,
}

enum Event {
    Hit {
        from: u32,
        to: u32,
        damage: f32,
        venom: bool,
        constrict: bool,
    },
    EatPlant {
        who: u32,
        plant: u32,
    },
    EatCarcass {
        who: u32,
        carcass: u32,
    },
    EatEgg {
        who: u32,
        egg: u32,
    },
    Spit(Projectile),
    Mate {
        a: u32,
        b: u32,
    },
}

pub struct Ecosystem {
    pub seed: u64,
    pub biome: Biome,
    pub world: World,
    pub nav: NavGrid,
    pub flora: Vec<FloraSpecies>,
    pub plants: Vec<Plant>,
    pub species: Vec<Species>,
    pub creatures: Vec<Creature>,
    pub carcasses: Vec<Carcass>,
    pub eggs: Vec<Egg>,
    pub projectiles: Vec<Projectile>,
    pub particles: Vec<Particle>,
    pub icons: Vec<Icon>,
    pub weather: Weather,
    pub time: f32,
    pub tick: u64,
    /// Population samples every two seconds: prey, predators, plants.
    pub history: Vec<[u16; 3]>,
    pub log: Vec<LogEntry>,
    pub births: [u32; 2],
    pub deaths: [u32; 2],
    /// Deaths by species and cause.
    pub causes: Vec<(usize, &'static str, u32)>,
    /// Bring in a few newcomers when a species dies out.
    pub migrants: bool,
    extinct_timer: [f32; 2],
    reseed_timer: f32,
    rng: Rng,
    next_id: u32,
    liquid_count: usize,
    ash_count: usize,
}

impl Ecosystem {
    pub fn new(seed: u64) -> Ecosystem {
        Self::with_biome(seed, None)
    }

    pub fn with_biome(seed: u64, kind: Option<biome::BiomeKind>) -> Ecosystem {
        let (biome, mut world) = biome::generate(seed, kind);
        // Let loose sand and liquids settle before anything lives here.
        for _ in 0..90 {
            world.step();
        }
        let mut rng = Rng::new(seed ^ 0xEC0);
        let mut flora = flora::biome_flora(&mut rng, &biome);
        let mut prey = genome::generate_prey(&mut rng, &biome, &mut flora);
        prey.idx = 0;
        let mut pred = genome::generate_predator(&mut rng, &biome, &prey);
        pred.idx = 1;
        let nav = NavGrid::new(&world);
        let weather = Weather {
            kind: WeatherKind::Clear,
            timer: rng.range(20.0, 50.0),
            intensity: 0.0,
            wind: 0.0,
        };
        let day = biome.day_length;
        let mut eco = Ecosystem {
            seed,
            biome,
            world,
            nav,
            flora,
            plants: Vec::new(),
            species: vec![prey, pred],
            creatures: Vec::new(),
            carcasses: Vec::new(),
            eggs: Vec::new(),
            projectiles: Vec::new(),
            particles: Vec::new(),
            icons: Vec::new(),
            weather,
            time: day * 0.3,
            tick: 0,
            history: Vec::new(),
            log: Vec::new(),
            births: [0; 2],
            deaths: [0; 2],
            causes: Vec::new(),
            migrants: true,
            extinct_timer: [0.0; 2],
            reseed_timer: 0.0,
            rng,
            next_id: 1,
            liquid_count: 0,
            ash_count: 0,
        };
        eco.seed_plants();
        eco.spawn_initial();
        eco.count_cells();
        let names: Vec<String> = eco.species.iter().map(|s| s.common.clone()).collect();
        eco.note(format!("Arrived at {}.", eco.biome.name));
        eco.note(format!("{} and {} share this world.", names[0], names[1]));
        eco
    }

    fn note(&mut self, text: String) {
        self.log.push(LogEntry {
            time: self.time,
            text,
        });
        if self.log.len() > 80 {
            self.log.remove(0);
        }
    }

    // ---- time ------------------------------------------------------------

    /// Fraction of the day: 0 midnight, 0.25 dawn, 0.5 noon, 0.75 dusk.
    pub fn day_phase(&self) -> f32 {
        (self.time / self.biome.day_length).fract()
    }

    /// Sun elevation in [-1, 1].
    fn elevation(&self) -> f32 {
        -(self.day_phase() * std::f32::consts::TAU).cos()
    }

    pub fn daylight(&self) -> f32 {
        let e = self.elevation();
        ((e + 0.15) / 0.4).clamp(0.0, 1.0)
    }

    pub fn dusk(&self) -> f32 {
        (1.0 - self.elevation().abs() / 0.3).clamp(0.0, 1.0)
    }

    pub fn time_label(&self) -> &'static str {
        match self.day_phase() {
            p if p < 0.2 => "Night",
            p if p < 0.3 => "Dawn",
            p if p < 0.7 => "Day",
            p if p < 0.8 => "Dusk",
            _ => "Night",
        }
    }

    pub fn day_number(&self) -> u32 {
        (self.time / self.biome.day_length) as u32 + 1
    }

    // ---- queries ---------------------------------------------------------

    pub fn creature(&self, id: u32) -> Option<&Creature> {
        self.creatures
            .binary_search_by_key(&id, |c| c.id)
            .ok()
            .map(|i| &self.creatures[i])
    }

    fn index_of(&self, id: u32) -> Option<usize> {
        self.creatures.binary_search_by_key(&id, |c| c.id).ok()
    }

    /// The creature nearest `p`, if close enough to count as clicking it.
    pub fn creature_at(&self, p: V2) -> Option<u32> {
        self.creatures
            .iter()
            .filter(|c| c.alive())
            .map(|c| {
                let sp = &self.species[c.species];
                let r = sp.stats.size * c.scale(sp) * 1.5 + 4.0;
                (c.id, c.pos.dist(p) - r)
            })
            .filter(|(_, d)| *d < 0.0)
            .min_by(|a, b| a.1.total_cmp(&b.1))
            .map(|(id, _)| id)
    }

    pub fn population(&self, role: Role) -> usize {
        self.creatures
            .iter()
            .filter(|c| c.alive() && self.species[c.species].role == role)
            .count()
    }

    // ---- spawning ----------------------------------------------------------

    fn alloc_id(&mut self) -> u32 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn seed_plants(&mut self) {
        for fi in 0..self.flora.len() {
            let n = match self.flora[fi].substrate {
                flora::Substrate::Ground => self.rng.int(30, 45),
                _ => self.rng.int(16, 28),
            };
            for _ in 0..n {
                if let Some((x, y, dir)) =
                    flora::find_spot(&self.world, &self.flora[fi], &mut self.rng, None, 8)
                {
                    if self
                        .plants
                        .iter()
                        .any(|p| (p.x - x).abs() < 3 && (p.y - y).abs() < 3)
                    {
                        continue;
                    }
                    let id = self.alloc_id();
                    let growth = self.rng.range(0.4, 1.0);
                    let fruit = self.rng.range(0.0, self.flora[fi].max_fruit as f32);
                    self.plants.push(Plant {
                        id,
                        flora: fi,
                        x,
                        y,
                        dir,
                        growth,
                        fruit,
                        seed: self.rng.next_u64() as u32,
                        dead: false,
                        check_timer: self.rng.range(0.0, 1.0),
                        seed_timer: self.rng.range(10.0, 40.0),
                        bitten: 0.0,
                    });
                }
            }
        }
    }

    /// A free spot where a creature of this species would naturally be.
    fn spawn_point(&mut self, sp_idx: usize, around: Option<V2>) -> Option<(V2, bool)> {
        let sp = self.species[sp_idx].clone();
        let probe = Creature::new(0, &sp, V2::ZERO, true, 0);
        let b = physics::bounds(&probe, &sp);
        let w = &self.world;
        let rng = &mut self.rng;
        let surf = &self.biome.surface;
        let fits = |p: V2| {
            let x0 = (p.x - b.hw).floor() as i32;
            let x1 = (p.x + b.hw).floor() as i32;
            let y0 = (p.y - b.top).floor() as i32;
            let y1 = (p.y + b.bottom - 0.01).floor() as i32;
            x0 > 2
                && x1 < WIDTH as i32 - 2
                && y0 > 2
                && y1 < HEIGHT as i32 - 2
                && (y0..=y1).all(|y| {
                    (x0..=x1).all(|x| {
                        let m = w.material(x, y);
                        !m.blocks_creature(sp.traits.climb) && (m.kind() != Kind::Liquid || sp.liquid_ok(m))
                    })
                })
        };
        for attempt in 0..600 {
            let x = match around {
                Some(a) if attempt < 300 => a.x as i32 + rng.int(-45, 45),
                _ => rng.int(8, WIDTH as i32 - 9),
            };
            if x < 4 || x >= WIDTH as i32 - 4 {
                continue;
            }
            let col: Vec<i32> = (4..HEIGHT as i32 - 4).collect();
            let y = *rng.pick(&col);
            let m = w.material(x, y);
            let p = v2(x as f32 + 0.5, y as f32 + 0.5);
            match sp.habitat {
                Habitat::Aquatic => {
                    if m.kind() == Kind::Liquid
                        && sp.swims_in(m)
                        && sp.swims_in(w.material(x, y - (b.top as i32 + 2)))
                        && fits(p)
                    {
                        return Some((p, false));
                    }
                }
                Habitat::LavaDweller if sp.loco == Locomotion::Swim => {
                    if m == Material::Lava && fits(p) {
                        return Some((p, false));
                    }
                }
                Habitat::Burrower => {
                    let depth = y - surf[x as usize];
                    if m.is_diggable() && (4..30).contains(&depth) {
                        return Some((p, true));
                    }
                }
                Habitat::Aerial => {
                    if m == Material::Empty
                        && fits(p)
                        && (0..12).all(|k| w.material(x, y + k) == Material::Empty)
                    {
                        return Some((p, false));
                    }
                }
                Habitat::CaveClinger => {
                    let ceiling = w.material(x, (y as f32 - b.top) as i32 - 1).is_solid_for_player();
                    if m == Material::Empty && ceiling && fits(p) {
                        return Some((p, false));
                    }
                }
                _ => {
                    // Standing on solid ground: drop down the column to the floor.
                    let (x, y) = match (sp.habitat, ctx_tree(&self.biome.trees, rng)) {
                        (Habitat::Arboreal, Some((tx, ty))) if attempt < 400 => (tx, ty),
                        _ => (x, y),
                    };
                    if w.material(x, y) != Material::Empty {
                        continue;
                    }
                    let mut yy = y;
                    while yy < HEIGHT as i32 - 4 && !w.material(x, yy + 1).blocks_creature(sp.traits.climb) {
                        yy += 1;
                    }
                    let p = v2(x as f32 + 0.5, (yy + 1) as f32 - b.bottom - 0.05);
                    let here = w.material(x, p.y as i32);
                    if here.kind() == Kind::Liquid && !sp.swims_in(here) {
                        continue;
                    }
                    if fits(p) {
                        return Some((p, false));
                    }
                }
            }
        }
        None
    }

    fn spawn(&mut self, sp_idx: usize, pos: V2, adult: bool, generation: u32, burrowed: bool) -> u32 {
        let id = self.alloc_id();
        let sp = &self.species[sp_idx];
        let mut c = Creature::new(id, sp, pos, adult, generation);
        c.burrowed = burrowed;
        c.age += self.rng.range(0.0, sp.stats.maturity * 0.5);
        c.satiety = self.rng.range(0.55, 0.85);
        c.mate_cd = self.rng.range(10.0, 40.0);
        self.creatures.push(c);
        id
    }

    fn spawn_initial(&mut self) {
        let prey_n = 10;
        let pred_n = self.rng.int(3, 5) as usize;
        let social = self.species[0].personality.sociality > 0.5;
        let mut centre: Option<V2> = None;
        for _ in 0..prey_n {
            let around = if social { centre } else { None };
            if let Some((p, burrowed)) = self.spawn_point(0, around) {
                centre.get_or_insert(p);
                self.spawn(0, p, true, 0, burrowed);
            }
        }
        let prey_centre = centre.unwrap_or(v2(WIDTH as f32 / 2.0, HEIGHT as f32 / 2.0));
        let pack = self.species[1].personality.sociality > 0.6;
        let mut pcentre: Option<V2> = None;
        for _ in 0..pred_n {
            let mut chosen = None;
            for _ in 0..6 {
                let around = if pack { pcentre } else { None };
                if let Some((p, b)) = self.spawn_point(1, around) {
                    chosen = Some((p, b));
                    if p.dist(prey_centre) > 90.0 {
                        break;
                    }
                }
            }
            if let Some((p, b)) = chosen {
                pcentre.get_or_insert(p);
                self.spawn(1, p, true, 0, b);
            }
        }
    }

    fn count_cells(&mut self) {
        let mut liquid = 0;
        let mut ash = 0;
        for c in self.world.cells() {
            match c.mat {
                Material::Water | Material::Acid => liquid += 1,
                Material::Ash => ash += 1,
                _ => {}
            }
        }
        self.liquid_count = liquid;
        self.ash_count = ash;
    }

    // ---- simulation --------------------------------------------------------

    pub fn step(&mut self) {
        self.tick += 1;
        self.time += DT;
        self.world.step();
        if self.tick.is_multiple_of(30) {
            self.nav.rebuild(&self.world);
        }
        if self.tick.is_multiple_of(300) {
            self.count_cells();
        }
        self.step_weather();
        self.step_plants();
        self.step_creatures();
        self.step_projectiles();
        self.step_eggs_and_carcasses();
        self.step_effects();
        if self.tick.is_multiple_of(120) {
            let prey = self.population(Role::Prey) as u16;
            let pred = self.population(Role::Predator) as u16;
            self.history.push([prey, pred, self.plants.len() as u16]);
            if self.history.len() > 400 {
                self.history.remove(0);
            }
        }
        self.step_migration();
    }

    fn step_weather(&mut self) {
        self.weather.timer -= DT;
        if self.weather.timer <= 0.0 {
            let kinds = self.biome.weathers.clone();
            let kind = *self.rng.pick(&kinds);
            let changed = kind != self.weather.kind;
            self.weather = Weather {
                kind,
                timer: self.rng.range(35.0, 90.0),
                intensity: self.rng.range(0.3, 1.0),
                wind: if kind == WeatherKind::Sandstorm {
                    self.rng.range(25.0, 50.0) * if self.rng.coin() { 1.0 } else { -1.0 }
                } else {
                    self.rng.range(-6.0, 6.0)
                },
            };
            if changed && kind != WeatherKind::Clear {
                self.note(format!("{} begins.", kind.label()));
            }
        }
        if !self.biome.open_sky {
            if self.weather.kind == WeatherKind::Spores && self.rng.prob(self.weather.intensity * 0.5) {
                let x = self.rng.range(0.0, WIDTH as f32);
                let y = self.rng.range(0.0, HEIGHT as f32);
                let c = self
                    .flora
                    .first()
                    .map_or([200, 200, 255], |f| f.glow.unwrap_or(f.fruit));
                self.particles.push(Particle {
                    pos: v2(x, y),
                    vel: v2(self.rng.range(-3.0, 3.0), self.rng.range(-4.0, 1.0)),
                    life: 6.0,
                    max_life: 6.0,
                    color: c,
                    glow: true,
                    gravity: 0.0,
                });
            }
            return;
        }
        let n = (self.weather.intensity
            * if self.weather.kind == WeatherKind::AcidRain {
                1.5
            } else {
                3.0
            }) as usize;
        let mat = match self.weather.kind {
            WeatherKind::Rain => Some(Material::Water),
            WeatherKind::AcidRain => Some(Material::Acid),
            WeatherKind::AshFall => Some(Material::Ash),
            _ => None,
        };
        if let Some(mat) = mat {
            let room = match mat {
                Material::Ash => self.ash_count < 9000,
                _ => self.liquid_count < self.biome.liquid_cap,
            };
            if room {
                for _ in 0..n {
                    let x = self.rng.int(1, WIDTH as i32 - 2);
                    if self.world.material(x, 1) == Material::Empty {
                        self.world.set(x, 1, mat);
                        if mat == Material::Ash {
                            self.ash_count += 1;
                        } else {
                            self.liquid_count += 1;
                        }
                    }
                }
            }
            // Streaks for the eye.
            if mat != Material::Ash {
                let col = self.biome.palette.mat(mat);
                for _ in 0..n * 2 {
                    self.particles.push(Particle {
                        pos: v2(self.rng.range(0.0, WIDTH as f32), 0.0),
                        vel: v2(self.weather.wind, 160.0),
                        life: 2.0,
                        max_life: 2.0,
                        color: mix(col, [255, 255, 255], 0.3),
                        glow: false,
                        gravity: 0.0,
                    });
                }
            }
        }
        if self.weather.kind == WeatherKind::Sandstorm {
            let col = self.biome.palette.mat(Material::Sand);
            for _ in 0..(self.weather.intensity * 6.0) as usize {
                let x = if self.weather.wind > 0.0 {
                    0.0
                } else {
                    WIDTH as f32
                };
                self.particles.push(Particle {
                    pos: v2(x, self.rng.range(0.0, HEIGHT as f32 * 0.7)),
                    vel: v2(self.weather.wind * 4.0, self.rng.range(-5.0, 10.0)),
                    life: 4.0,
                    max_life: 4.0,
                    color: col,
                    glow: false,
                    gravity: 5.0,
                });
            }
        }
        // Too much liquid: the sun takes some back.
        if self.tick.is_multiple_of(60) && self.liquid_count > self.biome.liquid_cap {
            for _ in 0..40 {
                let x = self.rng.int(1, WIDTH as i32 - 2);
                let surface = (0..HEIGHT as i32).find(|&y| self.world.material(x, y) != Material::Empty);
                if let Some(y) = surface
                    && matches!(self.world.material(x, y), Material::Water | Material::Acid)
                {
                    self.world.set(x, y, Material::Steam);
                    self.liquid_count -= 1;
                }
            }
        }
    }

    fn step_plants(&mut self) {
        let daylight = self.daylight();
        let light = if self.biome.open_sky {
            0.3 + 0.7 * daylight
        } else {
            1.0
        };
        let mut new_plants = Vec::new();
        let count = self.plants.len();
        for p in self.plants.iter_mut() {
            let fl = &self.flora[p.flora];
            p.growth = (p.growth + fl.growth_rate * DT * light * (1.0 - p.growth * 0.5)).min(1.0);
            if p.growth > 0.6 {
                p.fruit = (p.fruit + fl.fruit_rate * DT * light).min(fl.max_fruit as f32);
            }
            p.bitten = (p.bitten - DT).max(0.0);
            p.check_timer -= DT;
            if p.check_timer <= 0.0 {
                p.check_timer = 0.8 + self.rng.range(0.0, 0.6);
                let hazard = flora::plant_hazard(&self.world, fl, p);
                let burning = hazard == Some(flora::Hazard::Burnt);
                if hazard.is_some() || flora::valid_spot(&self.world, fl, p.x, p.y).is_none() {
                    p.dead = true;
                    let base = p.base();
                    if burning
                        && !fl.fireproof
                        && self.world.material(base.x as i32, base.y as i32 - 1) == Material::Empty
                    {
                        self.world.set(base.x as i32, base.y as i32 - 1, Material::Fire);
                    }
                    continue;
                }
            }
            p.seed_timer -= DT;
            if p.seed_timer <= 0.0 {
                p.seed_timer = 60.0 / fl.spread * self.rng.range(0.7, 1.3);
                if p.growth > 0.7
                    && count + new_plants.len() < PLANT_CAP
                    && let Some((x, y, dir)) =
                        flora::find_spot(&self.world, fl, &mut self.rng, Some((p.x, p.y)), 3)
                {
                    new_plants.push((p.flora, x, y, dir));
                }
            }
        }
        self.plants.retain(|p| !p.dead);
        // Wind-blown seeds keep every plant species going.
        self.reseed_timer -= DT;
        if self.reseed_timer <= 0.0 {
            let sparse = self.plants.len() < 60;
            self.reseed_timer = if sparse { 1.5 } else { 6.0 };
            for fi in 0..self.flora.len() {
                let n = self.plants.iter().filter(|p| p.flora == fi).count();
                if (n < 6 || sparse)
                    && self.plants.len() + new_plants.len() < PLANT_CAP
                    && let Some((x, y, dir)) =
                        flora::find_spot(&self.world, &self.flora[fi], &mut self.rng, None, 6)
                {
                    new_plants.push((fi, x, y, dir));
                }
            }
        }
        for (fi, x, y, dir) in new_plants {
            if self
                .plants
                .iter()
                .any(|p| (p.x - x).abs() < 3 && (p.y - y).abs() < 3)
            {
                continue;
            }
            let id = self.alloc_id();
            let seed = self.rng.next_u64() as u32;
            self.plants.push(Plant {
                id,
                flora: fi,
                x,
                y,
                dir,
                growth: 0.05,
                fruit: 0.0,
                seed,
                dead: false,
                check_timer: 1.0,
                seed_timer: 30.0,
                bitten: 0.0,
            });
        }
    }

    fn snapshots(&self) -> Vec<Snap> {
        self.creatures
            .iter()
            .filter(|c| c.alive())
            .map(|c| {
                let sp = &self.species[c.species];
                let s = c.scale(sp);
                let speed = c.vel.len();
                let mut k = (sp.stats.size * s / 4.0).sqrt().clamp(0.5, 1.6);
                if c.still > 1.0 {
                    k *= if sp.traits.camouflage { 0.2 } else { 0.6 };
                } else if sp.traits.camouflage {
                    k *= 0.7;
                }
                if c.sheltered {
                    k *= 0.3;
                }
                if c.luring {
                    k *= 0.15;
                }
                k *= 1.0 + speed / 80.0;
                let fleeing = c.brain.current == BehaviorKind::Flee;
                Snap {
                    id: c.id,
                    species: c.species,
                    pos: c.pos,
                    vel: c.vel,
                    size: sp.stats.size * s,
                    health: c.health / c.max_health(sp),
                    conspicuous: if c.burrowed { 0.0 } else { k },
                    burrowed: c.burrowed,
                    webbed: c.webbed > 0.3,
                    fleeing,
                    alarm: fleeing && sp.traits.alarm_call,
                    mature: c.mature(sp),
                    ready: c.mature(sp) && c.mate_cd <= 0.0 && c.pregnant.is_none() && c.satiety > 0.5,
                    playing_dead: c.playing_dead > 0.0,
                    luring: c.luring,
                    noise: c.noise,
                    hunting: if matches!(c.brain.current, BehaviorKind::Hunt | BehaviorKind::Mob) {
                        c.brain.target.or(c.brain.perception.kin_attacker.map(|k| k.0))
                    } else {
                        None
                    },
                    juvenile: !c.mature(sp),
                    home: c.brain.home,
                }
            })
            .collect()
    }

    fn step_creatures(&mut self) {
        let snaps = self.snapshots();
        let pop = [self.population(Role::Prey), self.population(Role::Predator)];
        let daylight = self.daylight();
        let dusk = self.dusk();
        let gravity = 300.0 * self.biome.gravity;
        let wind = if self.weather.kind == WeatherKind::Sandstorm {
            self.weather.wind
        } else {
            0.0
        };
        let time = self.time;
        let mut events = Vec::new();
        let mut icons = Vec::new();
        let mut particles = Vec::new();
        {
            let Ecosystem {
                creatures,
                species,
                flora,
                plants,
                carcasses,
                eggs,
                world,
                nav,
                rng,
                biome,
                ..
            } = self;
            let ctx = Ctx {
                species,
                flora,
                plants,
                carcasses,
                eggs,
                snaps: &snaps,
                trees: &biome.trees,
                surface: &biome.surface,
                daylight,
                dusk,
                time,
                gravity,
                open_sky: biome.open_sky,
                cave_ambient: biome.cave_ambient,
                pop,
                cap: [PREY_CAP, PREDATOR_CAP],
            };
            let mut budget = 10u32;
            for c in creatures.iter_mut() {
                if !c.alive() {
                    continue;
                }
                let sp = &species[c.species];
                update_needs(c, sp, rng);
                if c.status
                    .grabbed_by
                    .is_some_and(|g| ctx.snap(g).is_none_or(|s| s.pos.dist(c.pos) > 16.0))
                {
                    c.status.grabbed_by = None;
                }

                c.brain.think -= DT;
                if c.brain.think <= 0.0 {
                    let was = c.brain.current;
                    brain::think(c, sp, &ctx, world);
                    c.brain.think = 0.18 + rng.range(0.0, 0.1);
                    c.brain.ignore.retain(|(_, until)| *until > time);
                    if c.brain.current != was {
                        match c.brain.current {
                            BehaviorKind::Flee if sp.role == Role::Prey => {
                                icons.push((c.pos, IconKind::Alert))
                            }
                            BehaviorKind::Hunt => icons.push((c.pos, IconKind::Alert)),
                            BehaviorKind::Rest => icons.push((c.pos, IconKind::Sleep)),
                            _ => {}
                        }
                    }
                }
                let it = brain::act(c, sp, &ctx, world, rng);
                let mut cmd = brain::steer(c, sp, &it, nav, world, gravity, &mut budget, rng);
                if c.playing_dead > 0.0 || c.status.stun > 0.0 {
                    cmd.dir = V2::ZERO;
                }
                c.last_cmd = cmd;
                let mode = physics::integrate(c, sp, world, &cmd, gravity, DT, rng);
                if mode == Mode::Fly && wind != 0.0 {
                    c.vel.x += wind * DT * 2.0;
                }
                brain::track_progress(c, &it, DT);
                body::animate(c, sp, world, DT, mode == Mode::Fly);
                let (dps, cause) = physics::environment(c, sp, world, DT, rng);
                if dps > 0.0 {
                    c.health -= dps * DT;
                    if c.health <= 0.0 {
                        c.dead = Some(cause.unwrap_or("died"));
                    }
                }
                // Noise and stillness.
                let speed = c.vel.len();
                c.still = if speed < 2.0 { c.still + DT } else { 0.0 };
                c.noise = (speed / sp.stats.speed.max(1.0)).min(1.2)
                    * if it.sneak { 0.25 } else { 1.0 }
                    * (sp.stats.size / 4.0).sqrt();
                c.sheltered =
                    c.brain.home.is_some_and(|h| h.dist(c.pos) < 5.0) && !c.brain.nest_cells.is_empty();
                if c.burrowed && rng.chance(4) && speed > 3.0 {
                    let col = world.material(c.pos.x as i32, c.pos.y as i32 - 3);
                    if col.kind() != Kind::Empty {
                        particles.push((c.pos - v2(0.0, 3.0), [140, 110, 80]));
                    }
                }
                c.brain.action = it.action;
                resolve_action(
                    c,
                    sp,
                    it.action,
                    &ctx,
                    world,
                    rng,
                    gravity,
                    &mut events,
                    &mut icons,
                );
            }
        }
        for (p, k) in icons {
            self.icons.push(Icon {
                pos: p,
                kind: k,
                life: 1.2,
            });
        }
        for (p, col) in particles {
            self.dust(p, col, 2);
        }
        self.apply_events(events);
        self.reap();
    }

    fn apply_events(&mut self, events: Vec<Event>) {
        for e in events {
            match e {
                Event::Hit {
                    from,
                    to,
                    damage,
                    venom,
                    constrict,
                } => self.hit(from, to, damage, venom, constrict),
                Event::EatPlant { who, plant } => {
                    let Some(ci) = self.index_of(who) else { continue };
                    let Some(pl) = self.plants.iter_mut().find(|p| p.id == plant) else {
                        continue;
                    };
                    let fl = &self.flora[pl.flora];
                    let gained = pl.eat(fl, 1.0);
                    if pl.growth < 0.06 {
                        pl.dead = true;
                    }
                    let c = &mut self.creatures[ci];
                    c.satiety = (c.satiety + gained).min(1.0);
                    c.anim.eat = 0.35;
                    if gained > 0.1 && self.rng.chance(40) {
                        let p = c.pos - v2(0.0, 4.0);
                        self.icons.push(Icon {
                            pos: p,
                            kind: IconKind::Food,
                            life: 0.8,
                        });
                    }
                }
                Event::EatCarcass { who, carcass } => {
                    let Some(ci) = self.index_of(who) else { continue };
                    let Some(k) = self.carcasses.iter_mut().find(|k| k.id == carcass) else {
                        continue;
                    };
                    let sp = &self.species[self.creatures[ci].species];
                    let bite = (0.06 * sp.stats.size / 4.0).min(k.meat);
                    k.meat -= bite;
                    let toxic = k.toxic;
                    let victim = k.species;
                    let pos = k.pos;
                    let c = &mut self.creatures[ci];
                    c.satiety = (c.satiety + bite * 0.9).min(1.0);
                    c.anim.eat = 0.35;
                    if toxic && c.status.sick <= 0.0 {
                        c.status.sick = 12.0;
                        c.brain.aversion = (c.brain.aversion + 0.3).min(0.9);
                        let name = self.species[victim].common.clone();
                        let me = self.species[c.species].common.clone();
                        self.note(format!(
                            "A {me} fell sick after eating a {name}, and learned to avoid them."
                        ));
                    }
                    let blood = scale(self.species[victim].colors.accent, 0.6);
                    self.dust(pos, blood, 1);
                }
                Event::EatEgg { who, egg } => {
                    let Some(ci) = self.index_of(who) else { continue };
                    if let Some(i) = self.eggs.iter().position(|e| e.id == egg) {
                        let e = self.eggs.remove(i);
                        self.creatures[ci].satiety = (self.creatures[ci].satiety + 0.22).min(1.0);
                        self.dust(e.pos, e.color, 4);
                    }
                }
                Event::Spit(p) => self.projectiles.push(p),
                Event::Mate { a, b } => {
                    let (Some(ia), Some(ib)) = (self.index_of(a), self.index_of(b)) else {
                        continue;
                    };
                    let receptive = |c: &Creature| {
                        c.alive()
                            && c.pregnant.is_none()
                            && c.mate_cd <= 2.0
                            && c.mature(&self.species[c.species])
                    };
                    if !receptive(&self.creatures[ia]) || !receptive(&self.creatures[ib]) {
                        continue;
                    }
                    let cd = if sp_is_prey(&self.species, self.creatures[ia].species) {
                        self.rng.range(25.0, 45.0)
                    } else {
                        self.rng.range(40.0, 70.0)
                    };
                    let sp = self.species[self.creatures[ia].species].clone();
                    self.creatures[ia].mate_cd = cd;
                    self.creatures[ib].mate_cd = cd;
                    let mother = if a < b { ia } else { ib };
                    self.creatures[mother].pregnant = Some(sp.stats.gestation);
                    self.creatures[mother].satiety -= 0.2;
                    let p = self.creatures[ia].pos.lerp(self.creatures[ib].pos, 0.5) - v2(0.0, 5.0);
                    self.icons.push(Icon {
                        pos: p,
                        kind: IconKind::Heart,
                        life: 1.5,
                    });
                }
            }
        }
    }

    fn hit(&mut self, from: u32, to: u32, damage: f32, venom: bool, constrict: bool) {
        let (Some(fi), Some(ti)) = (self.index_of(from), self.index_of(to)) else {
            return;
        };
        let tsp = self.species[self.creatures[ti].species].clone();
        let t = &mut self.creatures[ti];
        if !t.alive() {
            return;
        }
        let dmg = damage * (1.0 - tsp.traits.armor);
        t.health -= dmg;
        t.hurt_timer = 2.5;
        t.anim.hurt = 0.2;
        t.last_hit_by = Some(from);
        t.sleeping = false;
        t.playing_dead = 0.0;
        t.brain.think = 0.0;
        if venom {
            t.status.poison = t.status.poison.max(6.0);
        }
        if constrict {
            t.status.grabbed_by = Some(from);
            t.status.stun = 0.6;
        }
        let pos = t.pos;
        let dead = t.health <= 0.0;
        if dead {
            t.dead = Some("killed");
        }
        let blood = mix(tsp.colors.accent, [150, 20, 40], 0.4);
        self.dust(pos, blood, (dmg / 3.0).clamp(2.0, 10.0) as usize);
        if tsp.traits.spines {
            let a = &mut self.creatures[fi];
            a.health -= damage * 0.3;
            a.hurt_timer = 2.0;
            a.anim.hurt = 0.2;
            if a.health <= 0.0 {
                a.dead = Some("impaled on spines");
            }
        }
        if dead {
            self.creatures[fi].kills += 1;
            let killer = self.species[self.creatures[fi].species].common.clone();
            if self.rng.chance(120) || self.creatures[fi].kills == 1 || cfg!(test) {
                let how = if self.creatures[fi].brain.current == BehaviorKind::Mob {
                    "was mobbed and killed by"
                } else {
                    "was caught by"
                };
                self.note(format!("A {} {how} a {killer}.", tsp.common));
            }
        }
    }

    /// Removes the dead, leaving carcasses, and handles births.
    fn reap(&mut self) {
        let mut newborns: Vec<(usize, V2, u32, bool)> = Vec::new();
        let mut new_eggs: Vec<(usize, V2, u32)> = Vec::new();
        for c in self.creatures.iter_mut() {
            let sp = &self.species[c.species];
            if c.alive() && c.age > sp.stats.lifespan {
                c.dead = Some("old age");
            }
            if let Some(t) = c.pregnant
                && c.alive()
            {
                let t = t - DT;
                if t <= 0.0 {
                    c.pregnant = None;
                    let n = self.rng.int(sp.stats.litter.0 as i32, sp.stats.litter.1 as i32);
                    for _ in 0..n {
                        if sp.traits.eggs {
                            new_eggs.push((c.species, c.pos, c.generation + 1));
                        } else {
                            newborns.push((c.species, c.pos, c.generation + 1, c.burrowed));
                        }
                    }
                } else {
                    c.pregnant = Some(t);
                }
            }
        }
        let mut dead_notes = Vec::new();
        let mut i = 0;
        while i < self.creatures.len() {
            if self.creatures[i].alive() {
                i += 1;
                continue;
            }
            let c = self.creatures.remove(i);
            let sp = &self.species[c.species];
            self.deaths[sp.role as usize] += 1;
            let cause = c.dead.unwrap_or("died");
            match self
                .causes
                .iter_mut()
                .find(|(s, k, _)| *s == c.species && *k == cause)
            {
                Some(e) => e.2 += 1,
                None => self.causes.push((c.species, cause, 1)),
            }
            if let Some(cause) = c.dead
                && cause != "killed"
                && (self.rng.chance(90) || cfg!(test))
            {
                dead_notes.push(format!("A {} {cause}.", sp.common));
            }
            let s = c.scale(sp);
            let meat = sp.stats.size * s * 0.4;
            self.icons.push(Icon {
                pos: c.pos - v2(0.0, 6.0),
                kind: IconKind::Skull,
                life: 1.5,
            });
            let id = self.next_id;
            self.next_id += 1;
            self.carcasses.push(Carcass {
                id,
                species: c.species,
                pos: c.pos,
                vel: c.vel * 0.3,
                meat,
                max_meat: meat,
                age: 0.0,
                scale: s,
                facing: c.facing,
                toxic: sp.traits.toxic,
            });
            // Put back anything a sand-swimmer was holding open.
            for (x, y, m) in c.displaced {
                if self.world.material(x, y) == Material::Empty {
                    self.world.set(x, y, Material::from_u8(m));
                }
            }
        }
        for n in dead_notes {
            self.note(n);
        }
        for (si, pos, generation, burrowed) in newborns {
            let role = self.species[si].role;
            if self.population(role) >= [PREY_CAP, PREDATOR_CAP][role as usize] {
                break;
            }
            let id = self.alloc_id();
            let sp = &self.species[si];
            let mut c = Creature::new(
                id,
                sp,
                pos + v2(self.rng.range(-3.0, 3.0), 0.0),
                false,
                generation,
            );
            c.burrowed = burrowed;
            c.mate_cd = sp.stats.maturity + 20.0;
            self.creatures.push(c);
            self.births[role as usize] += 1;
        }
        for (si, pos, generation) in new_eggs {
            let id = self.alloc_id();
            let sp = &self.species[si];
            let color = mix(sp.colors.belly, [240, 235, 220], 0.4);
            self.eggs.push(Egg {
                id,
                species: si,
                pos: pos + v2(self.rng.range(-4.0, 4.0), 0.0),
                vel: V2::ZERO,
                timer: self.rng.range(18.0, 28.0),
                generation,
                color,
            });
        }
    }

    fn step_projectiles(&mut self) {
        let g = 150.0 * self.biome.gravity;
        let mut hits = Vec::new();
        let mut i = 0;
        while i < self.projectiles.len() {
            let p = &mut self.projectiles[i];
            p.life -= DT;
            p.vel.y += g * DT;
            let steps = (p.vel.len() * DT).ceil().max(1.0) as i32;
            let inc = p.vel * DT / steps as f32;
            let mut end: Option<(V2, Option<u32>)> = None;
            for _ in 0..steps {
                p.pos += inc;
                if self
                    .world
                    .material(p.pos.x as i32, p.pos.y as i32)
                    .is_solid_for_player()
                {
                    end = Some((p.pos - inc, None));
                    break;
                }
                let hit = self.creatures.iter().find(|c| {
                    c.alive()
                        && c.id != p.owner
                        && c.species != p.owner_species
                        && c.pos.dist(p.pos)
                            < self.species[c.species].stats.size * c.scale(&self.species[c.species]) + 1.5
                });
                if let Some(c) = hit {
                    end = Some((p.pos, Some(c.id)));
                    break;
                }
            }
            if p.life <= 0.0 && end.is_none() {
                end = Some((p.pos, None));
            }
            match end {
                Some((at, who)) => {
                    let p = self.projectiles.swap_remove(i);
                    hits.push((p, at, who));
                }
                None => i += 1,
            }
        }
        for (p, at, who) in hits {
            let (x, y) = at.cell();
            let (mat, r) = match p.kind {
                SpitKind::Acid => (Some(Material::Acid), 1),
                SpitKind::Fire => (Some(Material::Fire), 2),
                SpitKind::Web => (Some(Material::Web), 2),
                SpitKind::Venom => (None, 0),
            };
            if let Some(m) = mat {
                self.world.paint_circle(x, y, r, m);
            }
            let col = match p.kind {
                SpitKind::Acid => self.biome.palette.mat(Material::Acid),
                SpitKind::Fire => [255, 150, 40],
                SpitKind::Web => [230, 230, 240],
                SpitKind::Venom => [180, 60, 220],
            };
            self.dust(at, col, 5);
            if let Some(id) = who {
                self.hit(p.owner, id, p.damage, p.kind == SpitKind::Venom, false);
                if let Some(i) = self.index_of(id) {
                    if p.kind == SpitKind::Fire && !self.species[self.creatures[i].species].traits.lava_proof
                    {
                        self.creatures[i].status.burning = 3.0;
                    }
                    if p.kind == SpitKind::Venom {
                        self.creatures[i].status.poison = 8.0;
                    }
                }
            }
        }
    }

    fn step_eggs_and_carcasses(&mut self) {
        let g = 300.0 * self.biome.gravity;
        let fall = |world: &World, pos: &mut V2, vel: &mut V2| {
            let below = world.material(pos.x as i32, (pos.y + 1.0) as i32);
            if below.blocks_creature(false) {
                vel.y = 0.0;
                vel.x *= 0.8;
            } else if below.kind() == Kind::Liquid {
                vel.y = (vel.y + g * 0.1 * DT).min(15.0);
            } else {
                vel.y = (vel.y + g * DT).min(200.0);
            }
            let steps = (vel.len() * DT).ceil().max(1.0) as i32;
            for _ in 0..steps {
                let next = *pos + *vel * (DT / steps as f32);
                if world
                    .material(next.x as i32, next.y as i32)
                    .blocks_creature(false)
                {
                    vel.y = 0.0;
                    break;
                }
                *pos = next;
            }
        };
        let mut hatch = Vec::new();
        for e in self.eggs.iter_mut() {
            fall(&self.world, &mut e.pos, &mut e.vel);
            e.timer -= DT;
            if e.timer <= 0.0 {
                hatch.push((e.species, e.pos, e.generation));
            }
        }
        let world = &self.world;
        let species = &self.species;
        self.eggs.retain(|e| {
            let m = world.material(e.pos.x as i32, e.pos.y as i32);
            let cooked =
                (m == Material::Lava || m == Material::Fire) && !species[e.species].traits.lava_proof;
            e.timer > 0.0 && !cooked && m != Material::Acid
        });
        for (si, pos, generation) in hatch {
            let role = self.species[si].role;
            if self.population(role) >= [PREY_CAP, PREDATOR_CAP][role as usize] {
                continue;
            }
            let id = self.alloc_id();
            let sp = &self.species[si];
            let c = Creature::new(id, sp, pos - v2(0.0, 2.0), false, generation);
            self.creatures.push(c);
            self.births[role as usize] += 1;
        }
        for k in self.carcasses.iter_mut() {
            fall(&self.world, &mut k.pos, &mut k.vel);
            k.age += DT;
            k.meat -= DT * 0.004 * k.max_meat;
        }
        self.carcasses.retain(|k| k.meat > 0.05 && k.age < 150.0);
    }

    fn step_effects(&mut self) {
        for p in self.particles.iter_mut() {
            p.vel.y += p.gravity * DT;
            p.pos += p.vel * DT;
            p.life -= DT;
        }
        let world = &self.world;
        self.particles.retain(|p| {
            p.life > 0.0
                && world.in_bounds(p.pos.x as i32, p.pos.y as i32)
                && !world
                    .material(p.pos.x as i32, p.pos.y as i32)
                    .is_solid_for_player()
        });
        if self.particles.len() > 1500 {
            let n = self.particles.len() - 1500;
            self.particles.drain(..n);
        }
        for i in self.icons.iter_mut() {
            i.life -= DT;
            i.pos.y -= DT * 6.0;
        }
        self.icons.retain(|i| i.life > 0.0);
    }

    fn dust(&mut self, at: V2, color: Rgb, n: usize) {
        for _ in 0..n {
            self.particles.push(Particle {
                pos: at,
                vel: v2(self.rng.range(-25.0, 25.0), self.rng.range(-40.0, 5.0)),
                life: 0.6,
                max_life: 0.6,
                color,
                glow: false,
                gravity: 200.0,
            });
        }
    }

    fn step_migration(&mut self) {
        if !self.migrants {
            return;
        }
        for si in 0..2 {
            let alive =
                self.creatures.iter().any(|c| c.species == si) || self.eggs.iter().any(|e| e.species == si);
            if alive {
                self.extinct_timer[si] = 0.0;
                continue;
            }
            self.extinct_timer[si] += DT;
            if self.extinct_timer[si] > 30.0 {
                self.extinct_timer[si] = 0.0;
                let n = if si == 0 { 5 } else { 2 };
                let mut around = None;
                for _ in 0..n {
                    if let Some((p, b)) = self.spawn_point(si, around) {
                        around.get_or_insert(p);
                        self.spawn(si, p, true, 0, b);
                    }
                }
                let name = self.species[si].common.clone();
                self.note(format!("A few {name} migrated in from elsewhere."));
            }
        }
    }
}

fn sp_is_prey(species: &[Species], i: usize) -> bool {
    species[i].role == Role::Prey
}

/// A random spot just above a tree's canopy.
fn ctx_tree(trees: &[biome::Tree], rng: &mut Rng) -> Option<(i32, i32)> {
    if trees.is_empty() || rng.prob(0.3) {
        return None;
    }
    let t = rng.pick(trees);
    Some((t.x + rng.int(-8, 8), t.ground - t.height - rng.int(8, 20)))
}

/// Hunger, tiredness, ageing, status effects and cooldowns.
fn update_needs(c: &mut Creature, sp: &Species, rng: &mut Rng) {
    c.age += DT;
    let sprint = c.vel.len() > sp.stats.speed * 1.05;
    let rate = if c.sleeping {
        0.5
    } else if sprint {
        1.6
    } else {
        1.0
    };
    c.satiety -= sp.stats.metabolism * rate * DT;
    if c.sleeping {
        c.energy = (c.energy + DT / 25.0).min(1.0);
    } else {
        c.energy -= DT / if sprint { 60.0 } else { 260.0 };
    }
    c.energy = c.energy.clamp(0.0, 1.0);
    let maxh = c.max_health(sp);
    if c.satiety <= 0.0 {
        c.satiety = 0.0;
        c.health -= maxh * 0.02 * DT;
        if c.health <= 0.0 {
            c.dead = Some("starved");
        }
    } else if c.satiety > 0.45 && c.hurt_timer <= 0.0 && c.status.poison <= 0.0 {
        c.health = (c.health + maxh * 0.012 * DT).min(maxh);
    }
    if c.status.poison > 0.0 {
        c.status.poison -= DT;
        c.health -= 2.5 * DT;
        if c.health <= 0.0 {
            c.dead = Some("succumbed to venom");
        }
    }
    c.status.sick = (c.status.sick - DT).max(0.0);
    c.status.stun = (c.status.stun - DT).max(0.0);
    c.brain.aversion = (c.brain.aversion - DT * 0.002).max(0.0);
    c.hurt_timer = (c.hurt_timer - DT).max(0.0);
    c.fear = (c.fear - DT * 0.15).max(0.0);
    c.playing_dead = (c.playing_dead - DT).max(0.0);
    c.attack_cd -= DT;
    c.spit_cd -= DT;
    c.leap_cd -= DT;
    c.ink_cd -= DT;
    c.mate_cd -= DT;
    if c.still > 3.0 && c.webbed > 0.0 && rng.chance(2) {
        c.still = 0.0;
    }
}

/// Carries out the physical part of an action this tick.
#[allow(clippy::too_many_arguments)]
fn resolve_action(
    c: &mut Creature,
    sp: &Species,
    action: Action,
    ctx: &Ctx,
    world: &mut World,
    rng: &mut Rng,
    gravity: f32,
    events: &mut Vec<Event>,
    icons: &mut Vec<(V2, IconKind)>,
) {
    let s = c.scale(sp);
    match action {
        Action::None => {}
        Action::Attack(id) => {
            let Some(t) = ctx.snap(id) else { return };
            let d = t.pos.dist(c.pos);
            let reach = sp.stats.reach * s + t.size * 0.8 + 3.0;
            if d <= reach && c.attack_cd <= 0.0 {
                c.attack_cd = sp.stats.attack_cooldown;
                c.anim.attack = 0.25;
                c.vel += (t.pos - c.pos).norm() * 25.0;
                let mut damage = sp.stats.damage * s;
                if sp.role == Role::Prey {
                    damage *= 0.6;
                }
                if c.brain.current == BehaviorKind::Patrol {
                    damage *= 0.3;
                }
                events.push(Event::Hit {
                    from: c.id,
                    to: id,
                    damage,
                    venom: sp.traits.venom && sp.role == Role::Predator,
                    constrict: sp.traits.constrict && sp.role == Role::Predator,
                });
            }
        }
        Action::Spit(target) => {
            if c.spit_cd > 0.0 {
                return;
            }
            let Some(kind) = sp.traits.spit else { return };
            c.spit_cd = rng.range(2.5, 4.0);
            c.anim.attack = 0.3;
            let speed = 110.0;
            let from = c.pos - v2(0.0, physics::bounds(c, sp).top * 0.5);
            let d = target - from;
            let t = (d.len() / speed).max(0.1);
            let g = gravity * 0.5;
            let vel = v2(d.x / t, d.y / t - 0.5 * g * t);
            events.push(Event::Spit(Projectile {
                kind,
                pos: from,
                vel,
                owner: c.id,
                owner_species: c.species,
                life: 3.0,
                damage: sp.stats.damage * 0.4 * s,
            }));
        }
        Action::EatPlant(id) => {
            if c.attack_cd <= 0.0 && c.satiety < 0.98 {
                c.attack_cd = 0.6;
                events.push(Event::EatPlant { who: c.id, plant: id });
            }
        }
        Action::EatCarcass(id) => {
            if c.attack_cd <= 0.0 && c.satiety < 0.98 {
                c.attack_cd = 0.4;
                events.push(Event::EatCarcass {
                    who: c.id,
                    carcass: id,
                });
            }
        }
        Action::EatEgg(id) => {
            if c.attack_cd <= 0.0 {
                c.attack_cd = 0.5;
                events.push(Event::EatEgg { who: c.id, egg: id });
            }
        }
        Action::Leap(dir) => {
            if c.leap_cd <= 0.0 && (c.on_ground || c.clinging) {
                c.leap_cd = 2.5;
                c.vel = dir * sp.stats.jump * 1.1;
                c.on_ground = false;
                c.clinging = false;
            }
        }
        Action::Dig(at) => {
            if c.brain.current == BehaviorKind::SetTrap {
                // Excavate the pit a few cells at a time.
                let mut dug = 0;
                for &(x, y) in &c.brain.trap_cells {
                    if world.material(x, y).is_diggable() {
                        world.set(x, y, Material::Empty);
                        dug += 1;
                        if dug >= 3 {
                            break;
                        }
                    }
                }
                if dug == 0 || c.brain.timer > 30.0 {
                    c.brain.trap = Some(at);
                    c.brain
                        .trap_cells
                        .retain(|&(x, y)| world.material(x, y) == Material::Empty);
                }
            } else if c.attack_cd <= 0.0 {
                c.attack_cd = 0.3;
                let (x, y) = at.cell();
                let mut n = 0;
                for dy in -1..=1 {
                    for dx in -1..=1 {
                        if world.material(x + dx, y + dy).is_diggable() {
                            world.set(x + dx, y + dy, Material::Empty);
                            n += 1;
                        }
                    }
                }
                c.carry_dirt = (c.carry_dirt + n).min(40);
                c.anim.eat = 0.3;
                if n == 0 {
                    c.brain.spot = None;
                }
            }
        }
        Action::PlaceNest => {
            let b = physics::bounds(c, sp);
            let mut placed = 0;
            let mut tries = c.brain.nest_plan.len();
            while placed < 2 && c.carry_dirt > 0 && tries > 0 {
                tries -= 1;
                let Some((x, y)) = c.brain.nest_plan.first().copied() else {
                    break;
                };
                c.brain.nest_plan.remove(0);
                let inside_me = (x as f32) >= c.pos.x - b.hw - 1.0
                    && (x as f32) <= c.pos.x + b.hw + 1.0
                    && (y as f32) >= c.pos.y - b.top - 1.0
                    && (y as f32) <= c.pos.y + b.bottom + 1.0;
                if inside_me {
                    c.brain.nest_plan.push((x, y));
                    continue;
                }
                if world.material(x, y) == Material::Empty {
                    world.set(x, y, Material::Nest);
                    c.carry_dirt -= 1;
                    placed += 1;
                }
            }
            c.anim.eat = 0.2;
        }
        Action::SpinWeb => {
            let mut spun = 0;
            for &(x, y) in &c.brain.trap_cells {
                if world.material(x, y) == Material::Empty {
                    world.set(x, y, Material::Web);
                    spun += 1;
                    if spun >= 2 {
                        break;
                    }
                }
            }
            if spun == 0 {
                let n = c.brain.trap_cells.len().max(1);
                let (sx, sy) = c
                    .brain
                    .trap_cells
                    .iter()
                    .fold((0, 0), |a, &(x, y)| (a.0 + x, a.1 + y));
                c.brain.trap = Some(v2(sx as f32 / n as f32, sy as f32 / n as f32));
            }
        }
        Action::Ink => {
            if c.ink_cd <= 0.0 {
                c.ink_cd = 14.0;
                let (x, y) = c.pos.cell();
                for _ in 0..40 {
                    let (dx, dy) = (rng.int(-7, 7), rng.int(-7, 7));
                    let m = world.material(x + dx, y + dy);
                    if m == Material::Empty || m.kind() == Kind::Liquid {
                        world.set(x + dx, y + dy, Material::Smoke);
                    }
                }
            }
        }
        Action::PlayDead => {
            c.playing_dead = rng.range(5.0, 9.0);
        }
        Action::Mate(id) => {
            if c.mate_cd <= 0.0 && c.pregnant.is_none() {
                events.push(Event::Mate { a: c.id, b: id });
                c.mate_cd = 2.0;
                icons.push((c.pos - v2(0.0, 6.0), IconKind::Heart));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecosystem_runs_and_stays_sane() {
        for seed in [1u64, 2, 3, 4, 5, 6] {
            let mut eco = Ecosystem::new(seed);
            assert!(
                eco.population(Role::Prey) >= 5,
                "seed {seed}: too few prey spawned"
            );
            assert!(
                eco.population(Role::Predator) >= 2,
                "seed {seed}: too few predators"
            );
            assert!(!eco.plants.is_empty());
            for _ in 0..60 * 20 {
                eco.step();
            }
            for c in &eco.creatures {
                assert!(c.pos.x.is_finite() && c.pos.y.is_finite());
                assert!(c.pos.x >= 0.0 && c.pos.x <= WIDTH as f32);
            }
        }
    }

    #[test]
    #[ignore]
    fn dump() {
        for seed in 0..30u64 {
            let mut eco = Ecosystem::new(seed);
            let s0 = (eco.population(Role::Prey), eco.population(Role::Predator));
            let mut per_min = Vec::new();
            for m in 0..5 {
                for _ in 0..60 * 60 {
                    eco.step();
                }
                per_min.push(format!(
                    "{m}:{}/{}",
                    eco.population(Role::Prey),
                    eco.population(Role::Predator)
                ));
            }
            println!(
                "   minutes {}  births {:?} deaths {:?} {:?}",
                per_min.join(" "),
                eco.births,
                eco.deaths,
                eco.causes
            );
            let p = &eco.species[0];
            let q = &eco.species[1];
            println!(
                "{seed:2} {:?} prey {:?}/{:?} pred {:?}/{:?} {:?} start {:?} after60s {} {} plants {} | {} | {}",
                eco.biome.kind,
                p.habitat,
                p.loco,
                q.habitat,
                q.loco,
                q.hunt,
                s0,
                eco.population(Role::Prey),
                eco.population(Role::Predator),
                eco.plants.len(),
                p.common,
                q.common
            );
            let mut beh: Vec<String> = eco
                .creatures
                .iter()
                .map(|c| {
                    if c.brain.current == BehaviorKind::Escape {
                        format!(
                            "{}:escape(breath {:.1} burn {:.1} liquid {} at {:?})",
                            c.species,
                            c.breath,
                            c.status.burning,
                            c.in_liquid,
                            eco.world.material(c.pos.x as i32, c.pos.y as i32)
                        )
                    } else {
                        format!("{}:{}", c.species, c.brain.label)
                    }
                })
                .collect();
            beh.sort();
            beh.dedup();
            println!("     {}", beh.join(", "));
            if eco.population(Role::Prey) < 3 {
                for l in &eco.log {
                    println!("       log: {}", l.text);
                }
            }
        }
    }

    #[test]
    #[ignore]
    fn coverage() {
        let mut seen: Vec<(String, u32)> = Vec::new();
        let (mut webs, mut nests, mut homes) = (0, 0, 0);
        for seed in 100..140u64 {
            let mut eco = Ecosystem::new(seed);
            for t in 0..60 * 180 {
                eco.step();
                if t % 30 == 0 {
                    for c in &eco.creatures {
                        let key = format!(
                            "{}: {}",
                            if c.species == 0 { "prey" } else { "pred" },
                            c.brain.label
                        );
                        match seen.iter_mut().find(|(k, _)| *k == key) {
                            Some(e) => e.1 += 1,
                            None => seen.push((key, 1)),
                        }
                    }
                }
            }
            let cells = eco.world.cells();
            webs += cells.iter().filter(|c| c.mat == Material::Web).count();
            nests += cells.iter().filter(|c| c.mat == Material::Nest).count();
            homes += eco
                .creatures
                .iter()
                .filter(|c| !c.brain.nest_cells.is_empty())
                .count();
        }
        seen.sort();
        for (k, n) in &seen {
            println!("{n:7} {k}");
        }
        println!("web cells {webs}, nest cells {nests}, creatures with shelters {homes}");
    }

    #[test]
    #[ignore]
    fn trace() {
        let seed: u64 = std::env::var("ECO_SEED")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(12);
        let mut eco = Ecosystem::new(seed);
        let want: usize = std::env::var("ECO_SP")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(1);
        let id = eco
            .creatures
            .iter()
            .find(|c| c.species == want)
            .map(|c| c.id)
            .unwrap();
        let frames_at = std::env::var("ECO_FRAMES")
            .ok()
            .and_then(|s| s.parse::<i32>().ok());
        for t in 0..90 {
            for f in 0..60 {
                eco.step();
                if frames_at == Some(t)
                    && f % 3 == 0
                    && let Some(c) = eco.creature(id)
                {
                    println!(
                        "     f{f:2} pos ({:.2},{:.2}) vel ({:.1},{:.1}) ground {} cmd ({:.2},{:.2}) j{} mode fly {} cling {} burrow {} liquid {} grabbed {:?} stun {:.1}",
                        c.pos.x,
                        c.pos.y,
                        c.vel.x,
                        c.vel.y,
                        c.on_ground,
                        c.last_cmd.dir.x,
                        c.last_cmd.dir.y,
                        c.last_cmd.jump as u8,
                        c.flying,
                        c.clinging,
                        c.burrowed,
                        c.in_liquid,
                        c.status.grabbed_by,
                        c.status.stun
                    );
                }
            }
            let Some(c) = eco.creature(id) else {
                println!("dead");
                break;
            };
            let p = &c.brain.perception;
            let tgt = c
                .brain
                .target
                .and_then(|t| eco.creature(t))
                .map(|t| t.pos.dist(c.pos));
            if std::env::var("ECO_MAP").ok().and_then(|s| s.parse::<i32>().ok()) == Some(t) {
                let sp = &eco.species[c.species];
                let b = physics::bounds(c, sp);
                println!(
                    "   bounds hw {:.1} top {:.1} bottom {:.1} webbed {:.1} in_liquid {}",
                    b.hw, b.top, b.bottom, c.webbed, c.in_liquid
                );
                for y in (c.pos.y as i32 - 25)..(c.pos.y as i32 + 20) {
                    let row: String = ((c.pos.x as i32 - 30)..(c.pos.x as i32 + 30))
                        .map(|x| {
                            if (x, y) == c.pos.cell() {
                                return '@';
                            }
                            match eco.world.material(x, y) {
                                Material::Empty => '.',
                                Material::Water => '~',
                                Material::Acid => 'a',
                                Material::Web => '#',
                                Material::Sand => 's',
                                Material::Grass => 'g',
                                Material::Dirt => 'd',
                                Material::Stone => 'S',
                                Material::Nest => 'N',
                                Material::Wood => 'w',
                                Material::Smoke | Material::Steam => ',',
                                _ => '?',
                            }
                        })
                        .collect();
                    println!("   {y:4} {row}");
                }
            }
            if t % 10 == 9 {
                let sp = &eco.species[c.species];
                let prof = physics::profile(sp, c.scale(sp), 300.0 * eco.biome.gravity);
                let (tx, ty) = NavGrid::tile_of(c.pos);
                let b = physics::bounds(c, sp);
                println!(
                    "   nav: tile ({tx},{ty}) passable {} profile {:?} bounds hw {:.1} top {:.1} bottom {:.1} near {:?}",
                    eco.nav.passable(&prof, tx, ty),
                    prof,
                    b.hw,
                    b.top,
                    b.bottom,
                    eco.nav.nearest_passable(&prof, tx, ty, 2)
                );
            }
            println!(
                "{t:3}s {:<28} cmd ({:.1},{:.1}) j{} wp {:?} sat {:.2} en {:.2} pos ({:.0},{:.0}) vel ({:.0},{:.0}) prey {:?} tgt_d {:?} stuck {:.0} path {} ground {} fly {} cling {} top3 {:?}",
                c.brain.label,
                c.last_cmd.dir.x,
                c.last_cmd.dir.y,
                c.last_cmd.jump as u8,
                c.brain.path.first().map(|w| (w.x as i32, w.y as i32)),
                c.satiety,
                c.energy,
                c.pos.x,
                c.pos.y,
                c.vel.x,
                c.vel.y,
                p.prey.map(|x| x.2 as i32),
                tgt.map(|d| d as i32),
                c.brain.stuck,
                c.brain.path.len(),
                c.on_ground,
                c.flying,
                c.clinging,
                c.brain
                    .scores
                    .iter()
                    .take(3)
                    .map(|(k, s)| format!("{k:?}={s:.2}"))
                    .collect::<Vec<_>>()
            );
        }
    }

    #[test]
    fn ecosystem_is_deterministic() {
        let mut a = Ecosystem::new(77);
        let mut b = Ecosystem::new(77);
        for _ in 0..300 {
            a.step();
            b.step();
        }
        assert_eq!(a.creatures.len(), b.creatures.len());
        for (x, y) in a.creatures.iter().zip(&b.creatures) {
            assert_eq!(x.pos, y.pos);
        }
    }
}
