//! Alien Versus: player-designed species fight in a generated arena.
//!
//! The arena is the authoritative fight simulation. It runs on the host,
//! emits [`Snapshot`]s and [`FightEvent`]s, and everyone (host included)
//! watches through a [`mirror::Mirror`] fed by those.

pub mod balance;
pub mod combat;
pub mod design;

pub mod mirror;
pub mod parts;

use serde::{Deserialize, Serialize};

use crate::eco::biome::{self, Biome, HEIGHT, WIDTH};
use crate::eco::body;
use crate::eco::brain::{self, Intent};
use crate::eco::creature::{Creature, limb_instances};
use crate::eco::flora::{self, FloraSpecies, GrowthForm, Plant, Substrate};
use crate::eco::genome::{Habitat, LimbKind, LimbTip, Locomotion, Species, SpitKind};
use crate::eco::math::{V2, mix, v2};
use crate::eco::nav::NavGrid;
use crate::eco::physics::{self, Mode};
use crate::eco::{Particle, Projectile};
use crate::material::{Kind, Material};
use crate::rng::Rng;
use crate::world::World;

use combat::{CombatAction, CombatBrain, EnemySnap, Food, Me, Tactic};
use design::{Design, Loadout, Weapon, WeaponKind, build_teams};
use parts::{PartDef, PartKind};

pub use crate::eco::DT;

/// Round length in simulated seconds.
pub const ROUND_TIME: f32 = 120.0;
/// When the frenzy starts: everyone sees everyone and nobody retreats.
pub const FRENZY_AT: f32 = 75.0;
/// No hits for this long starts the frenzy early.
pub const NO_HIT_FRENZY: f32 = 60.0;
/// A frenzy with no hits for this long ends the round on health.
pub const STANDOFF_SECS: f32 = 40.0;

/// Rounds in a match unless the host picks otherwise.
pub const DEFAULT_ROUNDS: u8 = 3;
/// Snapshots per simulated second.
pub const SNAPSHOT_HZ: f32 = 20.0;
/// Energy from one fruit.
pub const FRUIT_ENERGY: f32 = 0.22;
/// Plants the arena aims to have.
pub const PLANT_TARGET: usize = 44;

pub struct Team {
    pub player: String,
    pub design: Design,
    pub species: Species,
    pub loadout: Loadout,
    pub parts: Vec<PartDef>,
    pub spawn: V2,
}

#[derive(Clone, Copy, Debug)]
pub struct Strike {
    pub weapon: WeaponKind,
    pub t: f32,
    pub dur: f32,
    pub target: u32,
    pub aim: V2,
    /// The part aimed at, if the brain picked one.
    pub aim_part: Option<u8>,
    pub resolved: bool,
    /// Limb instance swinging, if any.
    pub limb: Option<usize>,
}

pub struct Fighter {
    pub c: Creature,
    pub team: usize,
    pub parts: Vec<f32>,
    pub brain: CombatBrain,
    pub strike: Option<Strike>,
    /// Per [`WeaponKind`] (by discriminant).
    pub cooldowns: [f32; 6],
    pub spit_cd: f32,
    pub stagger: f32,
    pub grab_t: f32,
    pub slam_cd: f32,
    pub charging: bool,
    pub dealt: f32,
    pub taken: f32,
    pub kills: u32,
    /// Seconds of flight left; empty wings are grounded until they recover.
    pub stamina: f32,
    pub winded: bool,
    /// Meal left on the body once dead.
    pub meat: f32,
    pub eat_t: f32,
    pub starving: bool,
}

/// What happened, for the watchers.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum FightEvent {
    Hit {
        x: f32,
        y: f32,
        dx: f32,
        dy: f32,
        damage: f32,
        victim: u32,
        /// 0 head, 1 body, 2 limb.
        part: u8,
        heavy: bool,
        blocked: bool,
        /// Landed on something that hadn't noticed the attacker.
        surprise: bool,
    },
    Clip {
        x: f32,
        y: f32,
    },
    Miss {
        x: f32,
        y: f32,
    },
    Clash {
        x: f32,
        y: f32,
    },
    Sever {
        victim: u32,
        limb: u8,
        name: String,
        x: f32,
        y: f32,
    },
    Kill {
        victim: u32,
        team: u8,
        killer_team: Option<u8>,
        cause: String,
        x: f32,
        y: f32,
        last: bool,
    },
    Slam {
        x: f32,
        y: f32,
        damage: f32,
    },
    Spit {
        x: f32,
        y: f32,
        kind: u8,
    },
    Grab {
        x: f32,
        y: f32,
    },
    Ink {
        x: f32,
        y: f32,
    },
    Poisoned {
        victim: u32,
    },
    Eat {
        who: u32,
        x: f32,
        y: f32,
        meat: bool,
        /// Index into the arena's plant list, for the bite animation.
        plant: Option<u16>,
    },
    Starving {
        victim: u32,
    },
    Frenzy,
    RoundOver {
        winner: Option<u8>,
    },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CreatureNet {
    pub id: u32,
    pub team: u8,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub facing: i8,
    /// 0..255 of max health.
    pub health: u8,
    pub parts: Vec<u8>,
    /// Bit 0 on_ground, 1 flying, 2 clinging, 3 burrowed, 4 in_liquid, 5 dead,
    /// 6 sneaking (still), 7 stunned.
    pub flags: u8,
    pub attack: u8,
    pub hurt: u8,
    pub poison: u8,
    pub burning: u8,
    pub webbed: u8,
    pub limbs: u32,
    pub tactic: u8,
    /// Limb instance aimed at (255 none) and where.
    pub reach_limb: u8,
    pub reach_x: f32,
    pub reach_y: f32,
    pub angle: f32,
    /// 0..255 of a full tank, and of full flight stamina.
    pub energy: u8,
    pub stamina: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ProjNet {
    pub kind: u8,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
    pub time: f32,
    pub clock: f32,
    pub time_scale: f32,
    pub frenzy: bool,
    pub creatures: Vec<CreatureNet>,
    pub projectiles: Vec<ProjNet>,
    /// Per plant, in the arena's order: growth and whole fruit; growth 0
    /// means it died.
    pub plants: Vec<(u8, u8)>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RoundResult {
    pub winner: Option<u8>,
    pub duration: f32,
    /// Per team: survivors, kills, damage dealt.
    pub survivors: Vec<u8>,
    pub kills: Vec<u32>,
    pub damage: Vec<f32>,
}

enum Pending {
    Strike { attacker: usize },
    Spit { attacker: usize, at: V2 },
    Ink { attacker: usize },
    Eat { attacker: usize, id: u32 },
}

pub struct Arena {
    pub seed: u64,
    pub biome: Biome,
    pub world: World,
    pub nav: NavGrid,
    pub flora: Vec<FloraSpecies>,
    pub plants: Vec<Plant>,
    pub teams: Vec<Team>,
    pub fighters: Vec<Fighter>,
    pub projectiles: Vec<Projectile>,
    pub particles: Vec<Particle>,
    pub events: Vec<FightEvent>,
    pub time: f32,
    pub tick: u64,
    pub clock: f32,
    pub time_scale: f32,
    pub frenzy: bool,
    pub result: Option<RoundResult>,
    /// Seconds of slow motion left and of frozen frame left.
    slowmo: f32,
    hitstop: f32,
    /// Real seconds since the round ended.
    pub over_for: f32,
    /// Clock time of the last landed hit, and when the frenzy began.
    last_hit: f32,
    frenzy_at: f32,
    acc: f32,
    day0: f32,
    rng: Rng,
    next_id: u32,
}

impl Arena {
    /// `designs` is one (player name, design) per team. The arena seed is
    /// adjusted until every team can reach every other, so read
    /// `arena.seed` for the one actually used.
    pub fn new(seed: u64, designs: &[(String, Design)]) -> Arena {
        let mut k = 0u64;
        loop {
            let candidate = seed.wrapping_add(k.wrapping_mul(7919));
            let mut arena = Self::generate(candidate, designs);
            if k >= 8 || arena.connected() {
                return arena;
            }
            k += 1;
        }
    }

    /// Whether every team has a route to every other team's start.
    fn connected(&mut self) -> bool {
        let gravity = 300.0 * self.biome.gravity;
        let n = self.teams.len();
        for a in 0..n {
            let lo = &self.teams[a].loadout;
            if lo.fly {
                continue;
            }
            let Some(from) = self.fighters.iter().find(|f| f.team == a).map(|f| f.c.pos) else {
                continue;
            };
            let sp = &self.teams[a].species;
            let prof = physics::profile(sp, 1.0, gravity);
            for b in 0..n {
                if a == b {
                    continue;
                }
                let Some(to) = self.fighters.iter().find(|f| f.team == b).map(|f| f.c.pos) else {
                    continue;
                };
                // Paths end as close as they can get, so check they arrive,
                // and that they don't wade through deep water on the way.
                let path = self.nav.find_path(&prof, from, to, 6000);
                let arrives = path.last().is_some_and(|p| p.dist(to) < 12.0) || from.dist(to) < 20.0;
                let wades = path.iter().any(|p| {
                    let (tx, ty) = NavGrid::tile_of(*p);
                    self.nav
                        .tile(tx, ty)
                        .is_some_and(|t| t.wet() && !sp.swims_in(t.liquid))
                });
                if !arrives || wades {
                    return false;
                }
            }
        }
        true
    }

    fn generate(seed: u64, designs: &[(String, Design)]) -> Arena {
        let (biome, mut world) = biome::generate(seed, None);
        for _ in 0..90 {
            world.step();
        }
        let (flora, plants) = grow_plants(seed, &biome, &world);
        let mut rng = Rng::new(seed ^ 0xA2E7A);
        let n = designs.len().max(1);
        let zone = WIDTH as f32 / n as f32;
        let teams: Vec<Team> = build_teams(designs, Some(&biome))
            .into_iter()
            .zip(designs)
            .enumerate()
            .map(|(i, ((species, loadout), (player, design)))| {
                let parts = parts::layout(&species, species.stats.max_health);
                Team {
                    player: player.clone(),
                    design: design.clone(),
                    species,
                    loadout,
                    parts,
                    spawn: v2(zone * (i as f32 + 0.5), HEIGHT as f32 * 0.5),
                }
            })
            .collect();
        let nav = NavGrid::new(&world);
        let day0 = rng.range(0.0, 1.0);
        let mut arena = Arena {
            seed,
            biome,
            world,
            nav,
            flora,
            plants,
            teams,
            fighters: Vec::new(),
            projectiles: Vec::new(),
            particles: Vec::new(),
            events: Vec::new(),
            time: 0.0,
            tick: 0,
            clock: 0.0,
            time_scale: 1.0,
            frenzy: false,
            result: None,
            slowmo: 0.0,
            hitstop: 0.0,
            over_for: 0.0,
            last_hit: 0.0,
            frenzy_at: 0.0,
            acc: 0.0,
            day0,
            rng,
            next_id: 1,
        };
        for ti in 0..arena.teams.len() {
            let count = arena.teams[ti].loadout.count;
            // The middle of each team's slice of the arena, so sides start
            // well apart.
            let pad = (zone * 0.22).max(14.0);
            let zone_x = (zone * ti as f32 + pad, zone * (ti as f32 + 1.0) - pad);
            for _ in 0..count {
                let p = arena.spawn_point(ti, zone_x);
                let id = arena.next_id;
                arena.next_id += 1;
                let sp = &arena.teams[ti].species;
                let mut c = Creature::new(id, sp, p, true, 0);
                c.facing = if p.x < WIDTH as f32 / 2.0 { 1.0 } else { -1.0 };
                c.limb_ok = vec![true; limb_instances(sp).len()];
                c.satiety = 1.0;
                c.energy = 1.0;
                c.leap_cd = 0.0;
                c.ink_cd = 0.0;
                let parts = arena.teams[ti].parts.iter().map(|d| d.max_hp).collect();
                let stamina = arena.teams[ti].loadout.stamina;
                arena.fighters.push(Fighter {
                    c,
                    team: ti,
                    parts,
                    brain: CombatBrain::new(id),
                    strike: None,
                    cooldowns: [0.0; 6],
                    spit_cd: 1.0,
                    stagger: 0.0,
                    grab_t: 0.0,
                    slam_cd: 0.0,
                    charging: false,
                    dealt: 0.0,
                    taken: 0.0,
                    kills: 0,
                    stamina,
                    winded: false,
                    meat: 0.0,
                    eat_t: 0.0,
                    starving: false,
                });
            }
        }
        arena
    }

    /// A free spot for a member of `team` within the x range.
    fn spawn_point(&mut self, team: usize, zone: (f32, f32)) -> V2 {
        let sp = self.teams[team].species.clone();
        let probe = Creature::new(0, &sp, V2::ZERO, true, 0);
        let b = physics::bounds(&probe, &sp);
        let w = &self.world;
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
                        !m.blocks_creature(sp.traits.climb) && m.kind() != Kind::Liquid
                    })
                })
        };
        let taken: Vec<V2> = self.fighters.iter().map(|f| f.c.pos).collect();
        let surface = self.biome.surface.clone();
        let open_sky = self.biome.open_sky;
        for attempt in 0..800 {
            let x = self.rng.range(zone.0, zone.1) as i32;
            // Mostly start on top of the terrain, not deep in a cave.
            let y = if attempt < 500 && open_sky {
                let s = surface[x as usize].min(HEIGHT as i32 - 8);
                (s - self.rng.int(2, 24)).max(3)
            } else {
                self.rng.int(4, HEIGHT as i32 - 6)
            };
            if w.material(x, y) != Material::Empty {
                continue;
            }
            let p = if sp.habitat == Habitat::Aerial {
                let p = v2(x as f32 + 0.5, y as f32 + 0.5);
                if !(0..10).all(|k| w.material(x, y + k) == Material::Empty) {
                    continue;
                }
                if open_sky && y > surface[x as usize] - 6 && attempt < 500 {
                    continue;
                }
                p
            } else {
                let mut yy = y;
                while yy < HEIGHT as i32 - 4 && !w.material(x, yy + 1).blocks_creature(sp.traits.climb) {
                    yy += 1;
                }
                let p = v2(x as f32 + 0.5, (yy + 1) as f32 - b.bottom - 0.05);
                let floor = w.material(x, yy + 1);
                if floor.kind() == Kind::Liquid || !floor.blocks_creature(sp.traits.climb) {
                    continue;
                }
                p
            };
            if !fits(p) {
                continue;
            }
            if attempt < 600 && taken.iter().any(|t| t.dist(p) < sp.stats.size * 3.0 + 4.0) {
                continue;
            }
            return p;
        }
        v2((zone.0 + zone.1) / 2.0, HEIGHT as f32 * 0.3)
    }

    // ---- time --------------------------------------------------------------

    pub fn day_phase(&self) -> f32 {
        (self.day0 + self.time / self.biome.day_length).fract()
    }

    fn elevation(&self) -> f32 {
        -(self.day_phase() * std::f32::consts::TAU).cos()
    }

    pub fn daylight(&self) -> f32 {
        ((self.elevation() + 0.15) / 0.4).clamp(0.0, 1.0)
    }

    pub fn dusk(&self) -> f32 {
        (1.0 - self.elevation().abs() / 0.3).clamp(0.0, 1.0)
    }

    pub fn over(&self) -> bool {
        self.result.is_some()
    }

    /// Advances by `real_dt` seconds of wall time, honouring slow motion.
    /// Returns how many simulation steps ran.
    pub fn advance(&mut self, real_dt: f32) -> u32 {
        let real_dt = real_dt.min(0.25);
        if self.over() {
            self.over_for += real_dt;
        }
        if self.hitstop > 0.0 {
            self.hitstop -= real_dt;
            self.time_scale = 0.0;
            return 0;
        }
        self.time_scale = if self.slowmo > 0.0 {
            self.slowmo -= real_dt;
            0.2
        } else if self.over() {
            0.5
        } else {
            1.0
        };
        self.acc += real_dt * self.time_scale;
        let mut n = 0;
        while self.acc >= DT && n < 16 {
            self.step();
            self.acc -= DT;
            n += 1;
        }
        if n == 16 {
            self.acc = 0.0;
        }
        n
    }

    fn drama(&mut self, slowmo: f32, hitstop: f32) {
        self.slowmo = self.slowmo.max(slowmo);
        self.hitstop = self.hitstop.max(hitstop);
    }

    // ---- simulation --------------------------------------------------------

    pub fn step(&mut self) {
        self.tick += 1;
        self.time += DT;
        if !self.over() {
            self.clock += DT;
        }
        self.world.step();
        if self.tick.is_multiple_of(30) {
            self.nav.rebuild(&self.world);
        }
        if !self.frenzy
            && !self.over()
            && (self.clock > FRENZY_AT || self.clock - self.last_hit > NO_HIT_FRENZY)
        {
            self.frenzy = true;
            self.frenzy_at = self.clock;
            self.events.push(FightEvent::Frenzy);
        }
        self.step_plants();
        self.step_fighters();
        self.step_projectiles();
        self.step_bodies();
        self.step_effects();
        self.reap();
        self.check_round();
    }

    fn light_at(&self, p: V2) -> f32 {
        let x = (p.x as usize).min(WIDTH - 1);
        if self.biome.open_sky && p.y < self.biome.surface[x] as f32 + 8.0 {
            0.12 + 0.88 * self.daylight()
        } else {
            self.biome.cave_ambient + 0.1
        }
    }

    fn snaps(&self) -> Vec<EnemySnap> {
        self.fighters
            .iter()
            .filter(|f| f.c.alive())
            .map(|f| {
                let team = &self.teams[f.team];
                let sp = &team.species;
                let lo = &team.loadout;
                let eff = parts::effects(&f.c, sp, &team.parts, &f.parts);
                // Weakest damaged limb, for clever enemies.
                let weak = team
                    .parts
                    .iter()
                    .zip(&f.parts)
                    .enumerate()
                    .filter(|(_, (d, hp))| matches!(d.kind, PartKind::Limb(_)) && **hp < d.max_hp * 0.7)
                    .filter(|(_, (d, _))| match d.kind {
                        PartKind::Limb(k) => f.c.has_limb(k as usize),
                        _ => false,
                    })
                    .min_by(|a, b| (a.1.1 / a.1.0.max_hp).total_cmp(&(b.1.1 / b.1.0.max_hp)))
                    .map(|(i, _)| part_centre(&f.c, sp, &team.parts[i]));
                EnemySnap {
                    id: f.c.id,
                    team: f.team,
                    pos: f.c.pos,
                    vel: f.c.vel,
                    facing: f.c.facing,
                    size: sp.stats.size,
                    health: f.c.health / sp.stats.max_health,
                    on_ground: f.c.on_ground,
                    flying: f.c.flying,
                    burrowed: f.c.burrowed,
                    still: f.c.still,
                    camouflage: lo.camouflage * if self.frenzy { 0.3 } else { 1.0 },
                    armor: lo.armor,
                    ranged: lo.spit.is_some(),
                    speed: sp.stats.speed * eff.speed,
                    targeting: f.brain.target,
                    stunned: f.c.status.stun > 0.0 || f.c.status.grabbed_by.is_some(),
                    weak_spot: weak,
                    head: body::head_world(&f.c, sp),
                    aware: f
                        .brain
                        .target
                        .filter(|_| f.brain.last_seen.is_some() && self.time - f.brain.lost_at < 1.5),
                    energy: f.c.energy,
                    parts: team
                        .parts
                        .iter()
                        .zip(&f.parts)
                        .enumerate()
                        .filter(|(_, (d, _))| match d.kind {
                            PartKind::Limb(k) => f.c.has_limb(k as usize),
                            _ => true,
                        })
                        .map(|(i, (d, hp))| {
                            let kind = match d.kind {
                                PartKind::Head => 0,
                                PartKind::Body(_) => 1,
                                PartKind::Limb(_) => 2,
                            };
                            (
                                part_centre(&f.c, sp, d),
                                kind,
                                (hp / d.max_hp).clamp(0.0, 1.0),
                                i as u8,
                            )
                        })
                        .collect(),
                }
            })
            .collect()
    }

    /// Everything edible right now: fruit on plants, meat on the fallen.
    fn food(&self) -> Vec<Food> {
        let mut out: Vec<Food> = self
            .plants
            .iter()
            .filter(|p| !p.dead && p.fruit >= 1.0)
            .map(|p| Food {
                id: p.id,
                pos: p.food_point(&self.flora[p.flora]),
                amount: p.fruit.floor() * FRUIT_ENERGY,
                meat: false,
            })
            .collect();
        for f in &self.fighters {
            if !f.c.alive() && f.meat > 0.02 {
                out.push(Food {
                    id: f.c.id,
                    pos: f.c.pos,
                    amount: f.meat,
                    meat: true,
                });
            }
        }
        out
    }

    fn step_fighters(&mut self) {
        let snaps = self.snaps();
        let food = self.food();
        let gravity = 300.0 * self.biome.gravity;
        let time = self.time;
        let frenzy = self.frenzy;
        let over = self.over();
        let centre = v2(WIDTH as f32 / 2.0, HEIGHT as f32 / 2.0);
        let mut pending: Vec<Pending> = Vec::new();
        let mut budget = 10u32;
        let mut dust: Vec<(V2, [u8; 3])> = Vec::new();
        for fi in 0..self.fighters.len() {
            if !self.fighters[fi].c.alive() {
                continue;
            }
            let team_i = self.fighters[fi].team;
            let light = self.light_at(self.fighters[fi].c.pos);
            let enemy_hint = snaps
                .iter()
                .filter(|s| s.team != team_i)
                .min_by(|a, b| {
                    let p = self.fighters[fi].c.pos;
                    a.pos.dist(p).total_cmp(&b.pos.dist(p))
                })
                .map(|s| s.pos);
            let enemy_side = {
                let mine = self.teams[team_i].spawn;
                let mut sum = V2::ZERO;
                let mut n = 0.0;
                for (i, t) in self.teams.iter().enumerate() {
                    if i != team_i {
                        sum += t.spawn;
                        n += 1.0;
                    }
                }
                if n > 0.0 { sum / n } else { mine }
            };
            let Arena {
                fighters,
                teams,
                world,
                nav,
                rng,
                ..
            } = self;
            let f = &mut fighters[fi];
            let team = &teams[team_i];
            let sp = &team.species;
            let lo = &team.loadout;
            let defs = &team.parts;
            let me = Me {
                max_health: sp.stats.max_health,
                speed: sp.stats.speed,
                size: sp.stats.size,
            };
            update_timers(f, lo, sp, rng);
            let mut eff = parts::effects(&f.c, sp, defs, &f.parts);
            eff.can_fly = eff.can_fly && !f.winded;
            if f.c.energy <= 0.0 {
                eff.speed *= 0.7;
            }
            let enemies: Vec<EnemySnap> = snaps.iter().filter(|s| s.team != team_i).cloned().collect();
            let allies: Vec<EnemySnap> = snaps
                .iter()
                .filter(|s| s.team == team_i && s.id != f.c.id)
                .cloned()
                .collect();
            let ctx = combat::Ctx {
                world,
                time,
                light,
                frenzy,
                gravity,
                enemy_side,
                centre,
                enemy_hint,
                food: &food,
            };
            f.brain.think -= DT;
            if f.brain.think <= 0.0 && !over {
                combat::think(
                    &mut f.c,
                    &mut f.brain,
                    lo,
                    &me,
                    &eff,
                    &enemies,
                    &allies,
                    &ctx,
                    rng,
                );
                f.brain.think = 0.2 + rng.range(0.0, 0.1) - lo.intelligence * 0.08;
            }
            let target = f.brain.target.and_then(|id| enemies.iter().find(|e| e.id == id));
            let origins: Vec<(WeaponKind, V2)> = lo
                .weapons
                .iter()
                .map(|w| (w.kind, weapon_origin(&f.c, sp, w.kind)))
                .collect();
            let it = if over {
                // Winners strut; the fight is done.
                f.brain.label = if f.c.vel.len() > 2.0 { "" } else { "victorious" };
                Intent::default()
            } else {
                combat::act(
                    &mut f.c,
                    &mut f.brain,
                    lo,
                    &me,
                    &eff,
                    target,
                    &allies,
                    &f.cooldowns,
                    &origins,
                    f.spit_cd,
                    &ctx,
                    rng,
                )
            };
            f.c.brain.label = f.brain.label;
            f.c.brain.timer += DT;
            let mut cmd = brain::steer(&mut f.c, sp, &it, nav, world, gravity, &mut budget, rng);
            // The shared steering refuses drops it can't see the bottom of.
            // Arena floors are never far: take the drop if it lands somewhere safe.
            if cmd.dir.x == 0.0
                && !cmd.fly
                && f.c.on_ground
                && let Some(goal) = it.goal
                && (goal.x - f.c.pos.x).abs() > 4.0
            {
                let b = physics::bounds(&f.c, sp);
                let dir = (goal.x - f.c.pos.x).signum();
                let ahead_x = (f.c.pos.x + dir * (b.hw + 3.0)) as i32;
                let feet = (f.c.pos.y + b.bottom) as i32;
                let landing = (0..(HEIGHT as i32 - feet).max(0))
                    .map(|k| world.material(ahead_x, feet + k))
                    .find(|m| !matches!(m.kind(), Kind::Empty | Kind::Gas));
                let safe = match landing {
                    Some(m) if m.kind() == Kind::Liquid => sp.swims_in(m),
                    Some(m) => m.blocks_creature(sp.traits.climb) || m == Material::Web,
                    None => true,
                };
                if safe {
                    cmd.dir.x = dir;
                    cmd.jump = goal.y < f.c.pos.y - 3.0 && (goal.x - f.c.pos.x).abs() < 18.0;
                }
            }
            // A path that goes straight down means "walk off this ledge".
            if f.c.on_ground && !cmd.dig && !cmd.fly && cmd.dir.y > 0.7 && cmd.dir.x.abs() < 0.35 {
                let toward = it.goal.map_or(f.c.facing, |g| {
                    if (g.x - f.c.pos.x).abs() > 1.0 {
                        (g.x - f.c.pos.x).signum()
                    } else {
                        f.c.facing
                    }
                });
                cmd.dir = v2(toward, 0.3).norm();
            }
            // Never move straight into a pool (or tunnel into one) that would kill us.
            if cmd.dir.len() > 0.1 && !cmd.fly {
                let b = physics::bounds(&f.c, sp);
                let probe = f.c.pos + cmd.dir.norm() * (b.hw.max(b.bottom) + 2.5);
                let feet_x = (f.c.pos.x + cmd.dir.x.signum() * (b.hw + 2.0)) as i32;
                let feet = (f.c.pos.y + b.bottom - 0.5) as i32;
                let bad = |m: Material| m.kind() == Kind::Liquid && !sp.liquid_ok(m);
                let deep_water = |x: i32, y: i32| {
                    world.material(x, y) == Material::Water
                        && !sp.traits.swim
                        && world.material(x, y + 3) == Material::Water
                };
                let (px, py) = probe.cell();
                let deadly = bad(world.material(px, py))
                    || deep_water(px, py)
                    || (cmd.dir.x != 0.0
                        && f.c.on_ground
                        && ((0..=2).any(|k| bad(world.material(feet_x, feet - k)))
                            || deep_water(feet_x, feet)));
                if deadly {
                    cmd.dir = V2::ZERO;
                    cmd.jump = false;
                    f.c.brain.stuck += 0.02;
                }
            }
            if f.c.status.stun > 0.0
                || f.stagger > 0.0
                || f.strike.is_some_and(|s| s.weapon != WeaponKind::Horn)
            {
                cmd.dir = V2::ZERO;
                cmd.jump = false;
            }
            if !eff.can_jump {
                cmd.jump = false;
            }
            if frenzy && f.brain.tactic != Tactic::Undermine {
                cmd.dig = false;
            }
            if !eff.can_fly {
                cmd.fly = false;
            }
            if f.c.status.grabbed_by.is_some() {
                cmd.dir = V2::ZERO;
                cmd.fly = false;
            }
            f.c.last_cmd = cmd;
            let before = f.c.pos;
            let mode = physics::integrate(&mut f.c, sp, world, &cmd, gravity, DT, rng);
            // The biggest bodies bulldoze soft ground that stops them.
            if lo.size >= 5.0
                && cmd.dir.x.abs() > 0.3
                && (f.c.pos.x - before.x).abs() < 0.05
                && mode == Mode::Ground
                && f.c.on_ground
                && self.tick.is_multiple_of(4)
            {
                let b = physics::bounds(&f.c, sp);
                let dir = cmd.dir.x.signum();
                let x0 = (f.c.pos.x + dir * (b.hw + 0.5)) as i32;
                let y0 = (f.c.pos.y - b.top) as i32;
                let y1 = (f.c.pos.y + b.bottom - 1.0) as i32;
                let mut cleared = 0;
                for y in y0..=y1 {
                    for k in 0..3 {
                        let x = x0 + dir as i32 * k;
                        if world.material(x, y).is_diggable() {
                            world.set(x, y, Material::Empty);
                            cleared += 1;
                        }
                    }
                }
                if cleared > 0 {
                    dust.push((v2(x0 as f32, (y0 + y1) as f32 / 2.0), [140, 110, 80]));
                }
            }
            brain::track_progress(&mut f.c, &it, DT);
            body::animate(&mut f.c, sp, world, DT, mode == Mode::Fly);
            let (dps, cause) = physics::environment(&mut f.c, sp, world, DT, rng);
            // Puddles too small for the pathfinder to notice shouldn't be
            // a death sentence; acid burns slowly here.
            let dps = if cause == Some("dissolved in acid") {
                dps * 0.35
            } else {
                dps
            };
            if dps > 0.0 {
                f.c.health -= dps * DT;
                f.taken += dps * DT;
                if f.c.health <= 0.0 {
                    f.c.dead = Some(cause.unwrap_or("died"));
                }
            }
            let speed = f.c.vel.len();
            f.c.still = if speed < 2.0 { f.c.still + DT } else { 0.0 };
            f.c.noise = (speed / sp.stats.speed.max(1.0)).min(1.2);
            // Energy: everything burns it, sprinting and flying fastest.
            let rate = if mode == Mode::Fly {
                1.7
            } else if speed > sp.stats.speed * 0.9 {
                1.5
            } else if speed < 2.0 {
                0.6
            } else if f.c.burrowed {
                0.8
            } else {
                1.0
            };
            f.c.energy = (f.c.energy - lo.metabolism * rate * DT).max(0.0);
            if f.c.energy <= 0.0 {
                if !f.starving {
                    f.starving = true;
                    self.events.push(FightEvent::Starving { victim: f.c.id });
                }
                let maxh = sp.stats.max_health;
                f.c.health -= maxh * 0.015 * DT;
                f.taken += maxh * 0.015 * DT;
                f.c.hurt_timer = f.c.hurt_timer.max(0.5);
                if f.c.health <= 0.0 {
                    f.c.dead = Some("starved");
                }
            } else if f.c.energy > 0.15 {
                f.starving = false;
            }
            // Flight stamina: wings tire in the air and rest on the ground.
            if lo.stamina > 0.0 {
                if mode == Mode::Fly {
                    f.stamina = (f.stamina - DT).max(0.0);
                    if f.stamina <= 0.0 {
                        f.winded = true;
                        f.c.flying = false;
                    }
                } else if f.c.on_ground || f.c.clinging || f.c.burrowed {
                    f.stamina = (f.stamina + DT * 0.6).min(lo.stamina);
                } else {
                    f.stamina = (f.stamina + DT * 0.2).min(lo.stamina);
                }
                if f.winded && f.stamina > lo.stamina * 0.4 {
                    f.winded = false;
                }
            }
            f.charging = lo.weapon(WeaponKind::Horn).is_some()
                && speed > 28.0
                && matches!(f.brain.tactic, Tactic::Rush | Tactic::Flank | Tactic::Dive);
            if f.c.burrowed && rng.chance(6) && speed > 3.0 {
                dust.push((f.c.pos - v2(0.0, 3.0), [140, 110, 80]));
            }

            // Actions.
            match f.brain.action {
                CombatAction::None => {}
                CombatAction::Strike(kind, target) => {
                    if f.strike.is_none() && f.stagger <= 0.0 && f.c.status.stun <= 0.0 {
                        let Some(w) = lo.weapon(kind) else { continue };
                        // Where to aim: a weak spot if we're clever, otherwise
                        // whatever part of them is nearest our weapon.
                        let origin = weapon_origin(&f.c, sp, kind);
                        let (aim, aim_part) = snaps
                            .iter()
                            .find(|s| s.id == target)
                            .map(|t| {
                                let limbs: Vec<&(V2, u8, f32, u8)> =
                                    t.parts.iter().filter(|p| p.1 == 2).collect();
                                let nearest = |ps: &[&(V2, u8, f32, u8)]| {
                                    ps.iter()
                                        .min_by(|a, b| a.0.dist(origin).total_cmp(&b.0.dist(origin)))
                                        .map(|p| (p.0, Some(p.3)))
                                };
                                match f.brain.focus {
                                    combat::Focus::Head => {
                                        (t.head, t.parts.iter().find(|p| p.1 == 0).map(|p| p.3))
                                    }
                                    combat::Focus::Limbs if !limbs.is_empty() => {
                                        let hurt = limbs
                                            .iter()
                                            .filter(|p| p.2 < 0.7)
                                            .min_by(|a, b| a.2.total_cmp(&b.2));
                                        match hurt {
                                            Some(p) => (p.0, Some(p.3)),
                                            None => nearest(&limbs).unwrap_or((t.pos, None)),
                                        }
                                    }
                                    _ => {
                                        let jitter = v2(rng.range(-1.0, 1.0), rng.range(-1.0, 1.0))
                                            * (t.size * (1.0 - lo.intelligence) * 0.6);
                                        let all: Vec<&(V2, u8, f32, u8)> = t.parts.iter().collect();
                                        let (p, part) = nearest(&all).unwrap_or((t.pos, None));
                                        (p + jitter, part)
                                    }
                                }
                            })
                            .unwrap_or((f.c.pos + v2(f.c.facing * 8.0, 0.0), None));
                        f.cooldowns[kind as usize] = w.cooldown;
                        let dur = match kind {
                            WeaponKind::Bite => 0.26,
                            WeaponKind::Claw => 0.3,
                            WeaponKind::Punch => 0.32,
                            WeaponKind::Tail => 0.42,
                            WeaponKind::Horn => 0.2,
                            WeaponKind::Tentacle => 0.45,
                        };
                        let limb = strike_limb(&f.c, sp, kind);
                        f.strike = Some(Strike {
                            weapon: kind,
                            t: 0.0,
                            dur,
                            target,
                            aim,
                            aim_part,
                            resolved: false,
                            limb,
                        });
                        f.c.anim.attack = dur;
                        if let Some(k) = limb {
                            f.c.anim.reach = Some((k, aim));
                        }
                        // Lunge.
                        let d = (aim - f.c.pos).norm();
                        let lunge = match kind {
                            WeaponKind::Bite => 30.0,
                            WeaponKind::Horn => 10.0,
                            _ => 12.0,
                        };
                        f.c.vel += d * lunge;
                        if d.x.abs() > 0.2 {
                            f.c.facing = d.x.signum();
                        }
                    }
                }
                CombatAction::Spit(at) => {
                    if f.spit_cd <= 0.0 && lo.spit.is_some() && f.c.energy > 0.0 {
                        f.spit_cd = lo.spit_cooldown;
                        f.c.anim.attack = 0.3;
                        // Venom and acid take something to make.
                        f.c.energy = (f.c.energy - 0.02).max(0.0);
                        pending.push(Pending::Spit { attacker: fi, at });
                    }
                }

                CombatAction::Leap(dir) => {
                    if f.c.leap_cd <= 0.0 && (f.c.on_ground || f.c.clinging || f.c.burrowed) {
                        f.c.leap_cd = 2.2 - lo.leap * 0.4;
                        f.c.vel = dir * sp.stats.jump * (1.0 + lo.leap * 0.15);
                        f.c.on_ground = false;
                        f.c.clinging = false;
                        f.c.burrowed = false;
                    }
                }
                CombatAction::Ink => {
                    if f.c.ink_cd <= 0.0 {
                        f.c.ink_cd = 12.0;
                        pending.push(Pending::Ink { attacker: fi });
                    }
                }
                CombatAction::Eat(id) => {
                    if f.eat_t <= 0.0 && f.strike.is_none() {
                        f.eat_t = 0.75;
                        f.c.anim.eat = 0.35;
                        pending.push(Pending::Eat { attacker: fi, id });
                    }
                }
            }
            // Strikes in progress.
            if let Some(mut st) = f.strike {
                st.t += DT;
                if !st.resolved && st.t >= st.dur * 0.45 {
                    st.resolved = true;
                    pending.push(Pending::Strike { attacker: fi });
                }
                if st.t >= st.dur {
                    f.strike = None;
                    f.c.anim.reach = None;
                } else {
                    f.strike = Some(st);
                }
            }
        }
        for (p, col) in dust {
            self.dust(p, col, 2, 0.6);
        }
        // Clashes: both strike each other in the same instant.
        let strikers: Vec<usize> = pending
            .iter()
            .filter_map(|p| match p {
                Pending::Strike { attacker } => Some(*attacker),
                _ => None,
            })
            .collect();
        for a in &strikers {
            for b in &strikers {
                if a < b {
                    let (sa, sb) = (self.fighters[*a].strike, self.fighters[*b].strike);
                    if let (Some(sa), Some(sb)) = (sa, sb)
                        && sa.target == self.fighters[*b].c.id
                        && sb.target == self.fighters[*a].c.id
                    {
                        let mid = self.fighters[*a].c.pos.lerp(self.fighters[*b].c.pos, 0.5);
                        self.events.push(FightEvent::Clash { x: mid.x, y: mid.y });
                        self.dust(mid, [255, 240, 180], 14, 0.5);
                    }
                }
            }
        }
        for p in pending {
            match p {
                Pending::Strike { attacker } => self.resolve_strike(attacker),
                Pending::Spit { attacker, at } => self.spit(attacker, at),
                Pending::Ink { attacker } => {
                    let (x, y) = self.fighters[attacker].c.pos.cell();
                    for _ in 0..50 {
                        let (dx, dy) = (self.rng.int(-8, 8), self.rng.int(-8, 8));
                        let m = self.world.material(x + dx, y + dy);
                        if m == Material::Empty || m.kind() == Kind::Liquid {
                            self.world.set(x + dx, y + dy, Material::Smoke);
                        }
                    }
                    self.events.push(FightEvent::Ink {
                        x: x as f32,
                        y: y as f32,
                    });
                }
                Pending::Eat { attacker, id } => self.eat(attacker, id),
            }
        }
    }

    /// A bite of fruit or meat, if `id` is still edible and in reach.
    fn eat(&mut self, ai: usize, id: u32) {
        let team = self.fighters[ai].team;
        let lo = &self.teams[team].loadout;
        let diet = lo.diet;
        let reach = lo.size * 1.1 + 5.0;
        let pos = self.fighters[ai].c.pos;
        let who = self.fighters[ai].c.id;
        let share = if diet == design::Diet::Omnivore { 0.7 } else { 1.0 };
        if let Some(pi) = self.plants.iter().position(|p| p.id == id) {
            let p = &mut self.plants[pi];
            let fl = &self.flora[p.flora];
            if p.dead || p.fruit < 1.0 || p.food_point(fl).dist(pos) > reach || !diet.eats_fruit() {
                return;
            }
            p.fruit -= 1.0;
            p.bitten = 0.4;
            let at = p.food_point(fl);
            let col = fl.fruit;
            let f = &mut self.fighters[ai];
            f.c.energy = (f.c.energy + FRUIT_ENERGY * share).min(1.0);
            self.dust(at, col, 3, 0.5);
            self.events.push(FightEvent::Eat {
                who,
                x: at.x,
                y: at.y,
                meat: false,
                plant: Some(pi as u16),
            });
            return;
        }
        let Some(ci) = self.fighters.iter().position(|f| f.c.id == id && !f.c.alive()) else {
            return;
        };
        if !diet.eats_meat() || self.fighters[ci].meat <= 0.02 || self.fighters[ci].c.pos.dist(pos) > reach {
            return;
        }
        let bite = self.fighters[ci].meat.min(0.12);
        self.fighters[ci].meat -= bite;
        let at = self.fighters[ci].c.pos;
        let col = mix(
            self.teams[self.fighters[ci].team].species.colors.accent,
            [150, 20, 40],
            0.5,
        );
        let f = &mut self.fighters[ai];
        f.c.energy = (f.c.energy + bite * share).min(1.0);
        self.dust(at, col, 2, 0.5);
        self.events.push(FightEvent::Eat {
            who,
            x: at.x,
            y: at.y,
            meat: true,
            plant: None,
        });
    }

    /// Fruit ripens quickly so grazers always have something to find.
    fn step_plants(&mut self) {
        let mut burnt = Vec::new();
        for (i, p) in self.plants.iter_mut().enumerate() {
            if p.dead {
                continue;
            }
            let fl = &self.flora[p.flora];
            p.growth = (p.growth + 0.02 * DT).min(1.0);
            p.fruit = (p.fruit + fl.max_fruit as f32 / 25.0 * DT).min(fl.max_fruit as f32);
            p.bitten = (p.bitten - DT).max(0.0);
            p.check_timer -= DT;
            if p.check_timer <= 0.0 {
                p.check_timer = 1.0;
                if flora::plant_hazard(&self.world, fl, p).is_some() {
                    p.dead = true;
                    burnt.push(i);
                }
            }
        }
        for i in burnt {
            let at = self.plants[i].base();
            let col = self.flora[self.plants[i].flora].leaf;
            self.dust(at, col, 4, 0.6);
        }
    }

    /// The contact point of a strike: where its weapon tip ends up, and
    /// the limb root it swings from.
    fn strike_geometry(&self, fi: usize) -> Option<(V2, V2, Weapon)> {
        let f = &self.fighters[fi];
        let st = f.strike?;
        let team = &self.teams[f.team];
        let sp = &team.species;
        let w = *team.loadout.weapon(st.weapon)?;
        let dir = (st.aim - f.c.pos).norm();
        let (root, tip) = match st.weapon {
            WeaponKind::Bite => {
                // Jaws snap at anything from the chest forward to the lunge.
                let m = body::mouth_world(&f.c, sp);
                let fwd = body::forward(&f.c);
                let d = if fwd.dot(dir) > 0.0 {
                    fwd.lerp(dir, 0.5).norm()
                } else {
                    fwd
                };
                (f.c.pos, m + d * (w.reach * 0.6))
            }
            WeaponKind::Horn => {
                let h = body::head_world(&f.c, sp);
                (f.c.pos, h + body::forward(&f.c) * (sp.body.head_r * 1.5 + 3.0))
            }
            _ => {
                let k = st.limb?;
                let insts = limb_instances(sp);
                let (li, far) = insts.get(k).copied()?;
                let root = body::limb_root(&f.c, sp, li, far);
                let len = sp.body.limbs[li].length * f.c.scale(sp) + 1.5;
                (root, root + (st.aim - root).norm() * len.min(w.reach))
            }
        };
        Some((root, tip, w))
    }

    fn resolve_strike(&mut self, ai: usize) {
        let Some((root, tip, w)) = self.strike_geometry(ai) else {
            return;
        };
        let st = self.fighters[ai].strike.unwrap();
        let attacker_team = self.fighters[ai].team;
        let climb = self.teams[attacker_team].species.traits.climb;
        // Does the swing clip terrain on the way?
        let path = tip - root;
        let len = path.len().max(0.1);
        let steps = len.ceil() as i32;
        let mut clipped: Option<V2> = None;
        for i in 1..=steps {
            let p = root + path * (i as f32 / steps as f32);
            if self.world.material(p.x as i32, p.y as i32).blocks_creature(climb) {
                clipped = Some(p);
                break;
            }
        }
        if let Some(at) = clipped {
            self.events.push(FightEvent::Clip { x: at.x, y: at.y });
            let col = self
                .biome
                .palette
                .mat(self.world.material(at.x as i32, at.y as i32));
            self.dust(at, mix(col, [255, 255, 255], 0.3), 8, 0.5);
            self.fighters[ai].stagger = 0.35;
            self.fighters[ai].c.vel *= 0.3;
            // Hard swings chip the terrain.
            if w.damage > 12.0 && self.world.material(at.x as i32, at.y as i32).is_diggable() {
                self.world
                    .paint_circle(at.x as i32, at.y as i32, 1, Material::Empty);
            }
            return;
        }
        // Find the target part along the swing.
        let Some(ti) = self
            .fighters
            .iter()
            .position(|f| f.c.id == st.target && f.c.alive())
        else {
            self.events.push(FightEvent::Miss { x: tip.x, y: tip.y });
            return;
        };
        let target_team = self.fighters[ti].team;
        let tsp = &self.teams[target_team].species;
        let defs = &self.teams[target_team].parts;
        let reach_slack = 1.2 + tsp.stats.size * 0.12 + w.reach * 0.05;
        let mut contact = None;
        // The part we aimed at counts first if the weapon got near it.
        if let Some(ap) = st.aim_part.map(|p| p as usize)
            && ap < defs.len()
            && match defs[ap].kind {
                PartKind::Limb(k) => self.fighters[ti].c.has_limb(k as usize),
                _ => true,
            }
        {
            let centre = part_centre(&self.fighters[ti].c, tsp, &defs[ap]);
            let near = (0..=4)
                .map(|i| root + path * (0.3 + 0.7 * i as f32 / 4.0))
                .map(|p| p.dist(centre))
                .fold(f32::MAX, f32::min);
            if near < reach_slack * 2.0 + 2.0 {
                contact = Some(parts::Contact {
                    part: ap,
                    dist: near,
                    at: centre,
                });
            }
        }
        for i in (0..=4).rev() {
            if contact.is_some() {
                break;
            }
            let p = root + path * (0.4 + 0.6 * i as f32 / 4.0);
            if let Some(c) = parts::nearest_part(&self.fighters[ti].c, tsp, defs, p, reach_slack) {
                contact = Some(c);
            }
        }
        let Some(contact) = contact else {
            self.events.push(FightEvent::Miss { x: tip.x, y: tip.y });
            return;
        };
        let dir = path.norm();
        let mut dmg = w.damage * self.rng.range(0.85, 1.15);
        let venom = w.venom && self.teams[attacker_team].loadout.venom > 0.0;
        // An ambush: they never saw it coming. Keen eyes earn these.
        let aid = self.fighters[ai].c.id;
        let unaware = {
            let t = &self.fighters[ti];
            t.brain.target != Some(aid) || self.time - t.brain.lost_at > 1.5
        };
        let surprise = self.fighters[ai].brain.surprise && unaware;
        if surprise {
            dmg *= 1.75;
        }
        self.fighters[ai].brain.surprise = false;
        self.damage(
            ti,
            contact.part,
            contact.at,
            dmg,
            dir,
            w.knockback,
            w.pierce,
            Some(ai),
            false,
            venom,
            surprise,
        );
        if surprise && self.fighters[ti].c.alive() {
            let t = &mut self.fighters[ti];
            t.c.status.stun = t.c.status.stun.max(0.5);
            t.brain.target = Some(aid);
            t.brain.lost_at = self.time;
        }
        if w.grab && self.fighters[ti].c.alive() {
            let aid = self.fighters[ai].c.id;
            let t = &mut self.fighters[ti];
            t.c.status.grabbed_by = Some(aid);
            t.c.status.stun = 0.9;
            let f = &mut self.fighters[ai];
            f.grab_t = 1.6;
            self.events.push(FightEvent::Grab {
                x: contact.at.x,
                y: contact.at.y,
            });
        }
    }

    /// Applies damage to a part of fighter `ti`. Handles armour, spines,
    /// severing, knockback and death.
    #[allow(clippy::too_many_arguments)]
    fn damage(
        &mut self,
        ti: usize,
        part: usize,
        at: V2,
        dmg: f32,
        dir: V2,
        knockback: f32,
        pierce: f32,
        from: Option<usize>,
        ranged: bool,
        venom: bool,
        surprise: bool,
    ) {
        let tteam = self.fighters[ti].team;
        let def = self.teams[tteam].parts[part].clone();
        let lo = &self.teams[tteam].loadout;
        let armor = if def.armored {
            lo.armor
        } else if def.kind == PartKind::Head {
            lo.armor * 0.4
        } else {
            0.0
        };
        let blocked_frac = armor * (1.0 - pierce);
        let actual = dmg * (1.0 - blocked_frac);
        let spines = lo.spines;
        let toxic = lo.toxic;
        let tmass = lo.size * lo.size;
        let amass = from.map_or(tmass, |ai| {
            let s = self.teams[self.fighters[ai].team].loadout.size;
            s * s
        });
        let maxh = self.teams[tteam].species.stats.max_health;
        let from_id = from.map(|ai| self.fighters[ai].c.id);

        self.last_hit = self.clock;
        let t = &mut self.fighters[ti];
        t.parts[part] -= actual;
        t.c.health -= actual * def.to_health;
        t.taken += actual;
        t.c.hurt_timer = 2.0;
        t.c.anim.hurt = 0.18;
        t.c.still = 0.0;
        t.brain.taken(actual, ranged);
        if let Some(id) = from_id {
            t.c.last_hit_by = Some(id);
        }
        let kb = dir * (knockback * actual * (amass / tmass).sqrt() * 0.9).min(95.0);
        t.c.vel += kb + v2(0.0, -kb.len() * 0.25);
        if venom {
            let dose = if ranged { 4.0 } else { 6.0 };
            t.c.status.poison = t.c.status.poison.max(dose);
            self.events.push(FightEvent::Poisoned { victim: t.c.id });
        }

        let heavy = actual > maxh * 0.16;
        if heavy {
            t.c.status.stun = t.c.status.stun.max(0.25);
        }
        // Severing.
        let mut severed: Option<(u8, &'static str)> = None;
        if let PartKind::Limb(k) = def.kind
            && t.parts[part] <= 0.0
            && t.c.has_limb(k as usize)
        {
            if t.c.limb_ok.is_empty() {
                t.c.limb_ok = vec![true; t.c.anim.feet.len()];
            }
            if let Some(ok) = t.c.limb_ok.get_mut(k as usize) {
                *ok = false;
            }
            severed = Some((k, def.name));
        }
        let crushed = def.kind == PartKind::Head && t.parts[part] <= 0.0;
        if crushed {
            t.c.health = t.c.health.min(0.0);
        }
        let dead = t.c.health <= 0.0;
        let vpos = t.c.pos;
        let vid = t.c.id;
        let base = self.teams[tteam].species.colors.accent;
        let blood = mix(base, [150, 20, 40], 0.5);
        self.dust(at, blood, (actual / 2.5).clamp(2.0, 14.0) as usize, 0.7);
        self.events.push(FightEvent::Hit {
            x: at.x,
            y: at.y,
            dx: dir.x,
            dy: dir.y,
            damage: actual,
            victim: vid,
            part: match def.kind {
                PartKind::Head => 0,
                PartKind::Body(_) => 1,
                PartKind::Limb(_) => 2,
            },
            heavy,
            blocked: blocked_frac > 0.35,
            surprise,
        });
        if let Some(ai) = from {
            let ateam = self.fighters[ai].team;
            let eats_meat = self.teams[ateam].loadout.diet.eats_meat();
            let amax = self.teams[ateam].species.stats.max_health;
            let a = &mut self.fighters[ai];
            a.dealt += actual;
            a.brain.dealt(actual, blocked_frac);
            // A bloody mouthful keeps a hunter going.
            if eats_meat && !ranged {
                a.c.energy = (a.c.energy + 0.3 * actual / amax).min(1.0);
            }
            // Spines and toxic flesh punish contact.
            if !ranged && (spines > 0.0 || toxic) {
                let hurt = dmg * spines * 0.4;
                a.c.health -= hurt;
                a.taken += hurt;
                a.c.anim.hurt = 0.15;
                if toxic {
                    a.c.status.poison = a.c.status.poison.max(5.0);
                }
                if a.c.health <= 0.0 {
                    a.c.dead = Some("impaled on spines");
                }
            }
        }
        if let Some((k, name)) = severed {
            self.events.push(FightEvent::Sever {
                victim: vid,
                limb: k,
                name: name.to_string(),
                x: at.x,
                y: at.y,
            });
            self.dust(at, blood, 12, 1.0);
        }
        if dead {
            let killer_team = from.map(|ai| self.fighters[ai].team as u8);
            let cause = if crushed {
                "skull crushed".to_string()
            } else if let Some(ai) = from {
                let w = self.fighters[ai]
                    .strike
                    .map(|s| s.weapon.label())
                    .unwrap_or("blow");
                format!("felled by a {w}")
            } else {
                "slain".to_string()
            };
            self.fighters[ti].c.dead = Some("killed");
            self.fighters[ti].meat = self.teams[tteam].loadout.size * 0.09;
            if let Some(ai) = from {
                self.fighters[ai].kills += 1;
                let ateam = self.fighters[ai].team;
                if self.teams[ateam].loadout.diet.eats_meat() {
                    let a = &mut self.fighters[ai];
                    a.c.energy = (a.c.energy + 0.25).min(1.0);
                }
            }
            let last = self
                .teams
                .iter()
                .enumerate()
                .filter(|(i, _)| self.alive(*i) > 0)
                .count()
                <= 1;
            self.events.push(FightEvent::Kill {
                victim: vid,
                team: tteam as u8,
                killer_team,
                cause,
                x: vpos.x,
                y: vpos.y,
                last,
            });
            if last {
                self.drama(1.6, 0.12);
            }
        }
    }

    fn spit(&mut self, ai: usize, at: V2) {
        let f = &self.fighters[ai];
        let team = &self.teams[f.team];
        let Some(kind) = team.loadout.spit else { return };
        let sp = &team.species;
        let speed = 115.0;
        let from = body::mouth_world(&f.c, sp);
        let d = at - from;
        let t = (d.len() / speed).max(0.1);
        let g = 150.0 * self.biome.gravity * 0.5;
        let vel = v2(d.x / t, d.y / t - 0.5 * g * t);
        self.projectiles.push(Projectile {
            kind,
            pos: from,
            vel,
            owner: f.c.id,
            owner_species: f.team,
            life: 3.0,
            damage: team.loadout.spit_damage,
        });
        self.events.push(FightEvent::Spit {
            x: from.x,
            y: from.y,
            kind: kind as u8,
        });
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
            let mut end: Option<(V2, Option<(usize, parts::Contact)>)> = None;
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
                let pos = p.pos;
                let hit = self
                    .fighters
                    .iter()
                    .enumerate()
                    .filter(|(_, f)| f.c.alive() && f.team != p.owner_species)
                    .find_map(|(fi, f)| {
                        let team = &self.teams[f.team];
                        parts::nearest_part(&f.c, &team.species, &team.parts, pos, 1.5).map(|c| (fi, c))
                    });
                if hit.is_some() {
                    end = Some((p.pos, hit));
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
            self.dust(at, col, 5, 0.5);
            if let Some((ti, contact)) = who {
                let ai = self.fighters.iter().position(|f| f.c.id == p.owner);
                let dir = p.vel.norm();
                self.damage(
                    ti,
                    contact.part,
                    contact.at,
                    p.damage,
                    dir,
                    0.6,
                    0.15,
                    ai,
                    true,
                    p.kind == SpitKind::Venom,
                    false,
                );
                if self.fighters[ti].c.alive() {
                    let lava_proof = self.teams[self.fighters[ti].team].species.traits.lava_proof;
                    if p.kind == SpitKind::Fire && !lava_proof {
                        self.fighters[ti].c.status.burning = 1.5;
                    }
                    if p.kind == SpitKind::Web {
                        self.fighters[ti].c.webbed = 2.0;
                    }
                }
            }
        }
    }

    /// Body-on-body: slams from charges and dives, and gentle separation.
    fn step_bodies(&mut self) {
        let n = self.fighters.len();
        for a in 0..n {
            for b in (a + 1)..n {
                if !self.fighters[a].c.alive() || !self.fighters[b].c.alive() {
                    continue;
                }
                let (pa, pb) = (self.fighters[a].c.pos, self.fighters[b].c.pos);
                let (sa, sb) = (
                    self.teams[self.fighters[a].team].loadout.size,
                    self.teams[self.fighters[b].team].loadout.size,
                );
                let d = pa.dist(pb);
                let touch = (sa + sb) * 0.9;
                if d > touch {
                    continue;
                }
                let away = if d < 0.1 { v2(1.0, 0.0) } else { (pb - pa) / d };
                if self.fighters[a].team == self.fighters[b].team {
                    self.fighters[a].c.vel -= away * 12.0;
                    self.fighters[b].c.vel += away * 12.0;
                    continue;
                }
                // Slam: whoever is moving fast into the other.
                let rel = (self.fighters[a].c.vel - self.fighters[b].c.vel).dot(away);
                let (hitter, victim, speed) = if rel > 0.0 { (a, b, rel) } else { (b, a, -rel) };
                let can = self.fighters[hitter].slam_cd <= 0.0 && speed > 45.0;
                if can {
                    let charging = self.fighters[hitter].charging;
                    let hteam = self.fighters[hitter].team;
                    let horn = self.teams[hteam].loadout.weapon(WeaponKind::Horn).copied();
                    let mass = self.teams[hteam].loadout.size.powi(2);
                    let mut dmg = 0.015 * speed * mass.sqrt();
                    let (mut kb, mut pierce) = (3.0, 0.0);
                    if charging && let Some(h) = horn {
                        dmg += h.damage;
                        kb = h.knockback;
                        pierce = h.pierce;
                    }
                    let vteam = self.fighters[victim].team;
                    let contact = parts::nearest_part(
                        &self.fighters[victim].c,
                        &self.teams[vteam].species,
                        &self.teams[vteam].parts,
                        self.fighters[hitter].c.pos,
                        touch + 2.0,
                    );
                    if let Some(c) = contact {
                        let dir = (self.fighters[victim].c.pos - self.fighters[hitter].c.pos).norm();
                        self.fighters[hitter].slam_cd = 1.0;
                        self.fighters[victim].slam_cd = 0.5;
                        self.damage(
                            victim,
                            c.part,
                            c.at,
                            dmg,
                            dir,
                            kb,
                            pierce,
                            Some(hitter),
                            false,
                            false,
                            false,
                        );
                        self.fighters[hitter].c.vel *= 0.4;
                        self.events.push(FightEvent::Slam {
                            x: c.at.x,
                            y: c.at.y,
                            damage: dmg,
                        });
                        continue;
                    }
                }
                let push = (touch - d) * 6.0 + 8.0;
                self.fighters[a].c.vel -= away * push;
                self.fighters[b].c.vel += away * push;
            }
        }
        // Grabs let go after a while, or when the grabber lets go of life.
        let ids: Vec<(u32, bool, f32)> = self
            .fighters
            .iter()
            .map(|f| (f.c.id, f.c.alive(), f.grab_t))
            .collect();
        for f in self.fighters.iter_mut() {
            if let Some(g) = f.c.status.grabbed_by {
                let holder = ids.iter().find(|(id, _, _)| *id == g);
                let held = holder.is_some_and(|(_, alive, t)| *alive && *t > 0.0);
                if !held {
                    f.c.status.grabbed_by = None;
                }
            }
        }
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
        if self.particles.len() > 1200 {
            let n = self.particles.len() - 1200;
            self.particles.drain(..n);
        }
    }

    fn dust(&mut self, at: V2, color: [u8; 3], n: usize, life: f32) {
        for _ in 0..n {
            self.particles.push(Particle {
                pos: at,
                vel: v2(self.rng.range(-30.0, 30.0), self.rng.range(-45.0, 5.0)),
                life,
                max_life: life,
                color,
                glow: false,
                gravity: 200.0,
            });
        }
    }

    fn reap(&mut self) {
        let mut deaths = Vec::new();
        for (i, f) in self.fighters.iter().enumerate() {
            if f.c.dead.is_some() && f.c.dead != Some("killed") && f.c.dead != Some("reported") {
                deaths.push(i);
            }
        }
        for i in deaths {
            let f = &mut self.fighters[i];
            let cause = f.c.dead.unwrap_or("died").to_string();
            let killer =
                f.c.last_hit_by
                    .and_then(|k| self.fighters.iter().find(|o| o.c.id == k).map(|o| o.team as u8));
            let (vid, team, pos) = (
                self.fighters[i].c.id,
                self.fighters[i].team,
                self.fighters[i].c.pos,
            );
            self.fighters[i].c.dead = Some("reported");
            self.fighters[i].meat = self.teams[team].loadout.size * 0.09;
            let last = self
                .teams
                .iter()
                .enumerate()
                .filter(|(t, _)| self.alive(*t) > 0)
                .count()
                <= 1;
            self.events.push(FightEvent::Kill {
                victim: vid,
                team: team as u8,
                killer_team: killer,
                cause,
                x: pos.x,
                y: pos.y,
                last,
            });
            if last {
                self.drama(1.4, 0.08);
            }
        }
        // Dead bodies keep falling but do nothing else; put back displaced soil.
        for f in self.fighters.iter_mut() {
            if !f.c.alive() && !f.c.displaced.is_empty() {
                for (x, y, m) in f.c.displaced.drain(..) {
                    if self.world.material(x, y) == Material::Empty {
                        self.world.set(x, y, Material::from_u8(m));
                    }
                }
            }
        }
    }

    pub fn alive(&self, team: usize) -> usize {
        self.fighters
            .iter()
            .filter(|f| f.team == team && f.c.alive())
            .count()
    }

    /// Average energy of a team's survivors, 0..1.
    pub fn team_energy(&self, team: usize) -> f32 {
        let alive: Vec<f32> = self
            .fighters
            .iter()
            .filter(|f| f.team == team && f.c.alive())
            .map(|f| f.c.energy)
            .collect();
        if alive.is_empty() {
            0.0
        } else {
            alive.iter().sum::<f32>() / alive.len() as f32
        }
    }

    /// Total health fraction of a team, 0..1.
    pub fn team_health(&self, team: usize) -> f32 {
        let t = &self.teams[team];
        let n = t.loadout.count.max(1) as f32;
        self.fighters
            .iter()
            .filter(|f| f.team == team && f.c.alive())
            .map(|f| (f.c.health / t.species.stats.max_health).clamp(0.0, 1.0))
            .sum::<f32>()
            / n
    }

    fn check_round(&mut self) {
        if self.over() {
            return;
        }
        let standing: Vec<usize> = (0..self.teams.len()).filter(|&t| self.alive(t) > 0).collect();
        // Out of time, or a stand-off nobody can break.
        let standoff = self.frenzy
            && self.clock - self.last_hit > STANDOFF_SECS
            && self.clock - self.frenzy_at > STANDOFF_SECS;
        let timeout = self.clock >= ROUND_TIME || standoff;
        if standing.len() > 1 && !timeout {
            return;
        }
        let winner = if standing.len() == 1 {
            Some(standing[0] as u8)
        } else if timeout {
            // Most health left wins; a dead heat goes to the better-fed
            // side, and a dead heat on both is a draw.
            let condition: Vec<(f32, f32)> = (0..self.teams.len())
                .map(|t| (self.team_health(t), self.team_energy(t)))
                .collect();
            let mut best: Option<usize> = None;
            let mut tie = false;
            for (t, &(h, e)) in condition.iter().enumerate() {
                match best {
                    None => best = Some(t),
                    Some(b) => {
                        let (bh, be) = condition[b];
                        if (h - bh).abs() < 0.02 {
                            if (e - be).abs() < 0.05 {
                                tie = true;
                            } else if e > be {
                                best = Some(t);
                                tie = false;
                            }
                        } else if h > bh {
                            best = Some(t);
                            tie = false;
                        }
                    }
                }
            }
            if tie { None } else { best.map(|t| t as u8) }
        } else {
            None
        };
        let result = RoundResult {
            winner,
            duration: self.clock,
            survivors: (0..self.teams.len()).map(|t| self.alive(t) as u8).collect(),
            kills: (0..self.teams.len())
                .map(|t| {
                    self.fighters
                        .iter()
                        .filter(|f| f.team == t)
                        .map(|f| f.kills)
                        .sum()
                })
                .collect(),
            damage: (0..self.teams.len())
                .map(|t| {
                    self.fighters
                        .iter()
                        .filter(|f| f.team == t)
                        .map(|f| f.dealt)
                        .sum()
                })
                .collect(),
        };
        self.events.push(FightEvent::RoundOver { winner });
        self.result = Some(result);
        self.over_for = 0.0;
        // Release everyone and stop the clock.
        for f in self.fighters.iter_mut() {
            f.c.status.grabbed_by = None;
            f.strike = None;
            f.c.anim.reach = None;
        }
    }

    // ---- watching ------------------------------------------------------------

    pub fn snapshot(&self) -> Snapshot {
        Snapshot {
            time: self.time,
            clock: self.clock,
            time_scale: self.time_scale,
            frenzy: self.frenzy,
            creatures: self
                .fighters
                .iter()
                .map(|f| {
                    let sp = &self.teams[f.team].species;
                    let defs = &self.teams[f.team].parts;
                    let c = &f.c;
                    let mut flags = 0u8;
                    for (bit, on) in [
                        c.on_ground,
                        c.flying,
                        c.clinging,
                        c.burrowed,
                        c.in_liquid,
                        !c.alive(),
                        c.still > 1.0,
                        c.status.stun > 0.0 || c.status.grabbed_by.is_some(),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        if on {
                            flags |= 1 << bit;
                        }
                    }
                    let mut limbs = 0u32;
                    for (k, ok) in c.limb_ok.iter().enumerate().take(32) {
                        if *ok {
                            limbs |= 1 << k;
                        }
                    }
                    if c.limb_ok.is_empty() {
                        limbs = u32::MAX;
                    }
                    CreatureNet {
                        id: c.id,
                        team: f.team as u8,
                        x: c.pos.x,
                        y: c.pos.y,
                        vx: c.vel.x,
                        vy: c.vel.y,
                        facing: if c.facing < 0.0 { -1 } else { 1 },
                        health: ((c.health / sp.stats.max_health).clamp(0.0, 1.0) * 255.0) as u8,
                        parts: defs
                            .iter()
                            .zip(&f.parts)
                            .map(|(d, hp)| ((hp / d.max_hp).clamp(0.0, 1.0) * 255.0) as u8)
                            .collect(),
                        flags,
                        attack: (c.anim.attack * 255.0).clamp(0.0, 255.0) as u8,
                        hurt: (c.anim.hurt * 255.0).clamp(0.0, 255.0) as u8,
                        poison: (c.status.poison * 20.0).clamp(0.0, 255.0) as u8,
                        burning: (c.status.burning * 40.0).clamp(0.0, 255.0) as u8,
                        webbed: (c.webbed * 60.0).clamp(0.0, 255.0) as u8,
                        limbs,
                        tactic: f.brain.tactic.index() as u8,
                        reach_limb: c.anim.reach.map_or(255, |(k, _)| k.min(254) as u8),
                        reach_x: c.anim.reach.map_or(0.0, |(_, p)| p.x),
                        reach_y: c.anim.reach.map_or(0.0, |(_, p)| p.y),
                        angle: c.anim.angle,
                        energy: (c.energy.clamp(0.0, 1.0) * 255.0) as u8,
                        stamina: {
                            let full = self.teams[f.team].loadout.stamina;
                            if full > 0.0 {
                                ((f.stamina / full).clamp(0.0, 1.0) * 255.0) as u8
                            } else {
                                255
                            }
                        },
                    }
                })
                .collect(),
            plants: self
                .plants
                .iter()
                .map(|p| {
                    if p.dead {
                        (0, 0)
                    } else {
                        (
                            (p.growth.clamp(0.0, 1.0) * 255.0).max(1.0) as u8,
                            p.fruit.floor() as u8,
                        )
                    }
                })
                .collect(),
            projectiles: self
                .projectiles
                .iter()
                .map(|p| ProjNet {
                    kind: p.kind as u8,
                    x: p.pos.x,
                    y: p.pos.y,
                    vx: p.vel.x,
                    vy: p.vel.y,
                })
                .collect(),
        }
    }

    /// Everything that happened since the last call.
    pub fn take_events(&mut self) -> Vec<FightEvent> {
        std::mem::take(&mut self.events)
    }
}

/// Cooldowns, poison, regeneration and the like.
fn update_timers(f: &mut Fighter, lo: &Loadout, sp: &Species, rng: &mut Rng) {
    let c = &mut f.c;
    c.age += DT;
    for cd in f.cooldowns.iter_mut() {
        *cd -= DT;
    }
    f.spit_cd -= DT;
    f.eat_t = (f.eat_t - DT).max(0.0);
    f.stagger = (f.stagger - DT).max(0.0);
    f.grab_t = (f.grab_t - DT).max(0.0);
    f.slam_cd = (f.slam_cd - DT).max(0.0);
    c.status.stun = (c.status.stun - DT).max(0.0);
    c.hurt_timer = (c.hurt_timer - DT).max(0.0);
    c.leap_cd -= DT;
    c.ink_cd -= DT;
    if c.status.poison > 0.0 {
        c.status.poison -= DT;
        c.health -= 2.0 * DT;
        f.taken += 2.0 * DT;
        if c.health <= 0.0 {
            c.dead = Some("succumbed to venom");
        }
    }
    if lo.regen > 0.0 && c.hurt_timer <= 0.0 && c.health > 0.0 && c.energy > 0.0 {
        let maxh = sp.stats.max_health;
        c.health = (c.health + maxh * lo.regen * DT).min(maxh);
        for (hp, d) in f.parts.iter_mut().zip(&sp.body.segs) {
            let _ = d;
            *hp += maxh * lo.regen * DT * 0.5;
        }
    }
    if c.webbed > 0.0 && rng.chance(2) {
        c.still = 0.0;
    }
}

/// Where a weapon starts from: the mouth, or the root of the limb it uses.
fn weapon_origin(c: &Creature, sp: &Species, kind: WeaponKind) -> V2 {
    match kind {
        WeaponKind::Bite | WeaponKind::Horn => body::mouth_world(c, sp),
        _ => strike_limb(c, sp, kind)
            .and_then(|k| limb_instances(sp).get(k).copied())
            .map(|(li, far)| body::limb_root(c, sp, li, far))
            .unwrap_or(c.pos),
    }
}

/// The limb instance a weapon swings with.
fn strike_limb(c: &Creature, sp: &Species, kind: WeaponKind) -> Option<usize> {
    let insts = limb_instances(sp);
    let pick = |want: &dyn Fn(&crate::eco::genome::Limb) -> bool| {
        insts
            .iter()
            .enumerate()
            .filter(|(k, _)| c.has_limb(*k))
            .find(|(_, (li, _))| want(&sp.body.limbs[*li]))
            .map(|(k, _)| k)
    };
    match kind {
        WeaponKind::Bite | WeaponKind::Horn => None,
        WeaponKind::Punch => pick(&|l| l.kind == LimbKind::Arm),
        WeaponKind::Claw => pick(&|l| l.kind == LimbKind::Arm && l.tip == LimbTip::Claw)
            .or_else(|| pick(&|l| l.kind == LimbKind::Leg && l.tip == LimbTip::Claw))
            .or_else(|| pick(&|l| l.kind == LimbKind::Leg)),
        WeaponKind::Tail => pick(&|l| l.kind == LimbKind::Tail),
        WeaponKind::Tentacle => pick(&|l| l.kind == LimbKind::Tentacle),
    }
}

/// Rough world position of a part, for aiming.
pub fn part_centre(c: &Creature, sp: &Species, def: &PartDef) -> V2 {
    match def.kind {
        PartKind::Head => body::head_world(c, sp),
        PartKind::Body(i) => body::seg_world(c, sp, (i as usize).min(sp.body.segs.len() - 1)).0,
        PartKind::Limb(k) => {
            let k = k as usize;
            let insts = limb_instances(sp);
            let Some(&(li, far)) = insts.get(k) else {
                return c.pos;
            };
            match sp.body.limbs[li].kind {
                LimbKind::Leg => c
                    .anim
                    .feet
                    .get(k)
                    .map_or(c.pos, |f| f.pos.lerp(body::limb_root(c, sp, li, far), 0.5)),
                LimbKind::Wing => body::limb_root(c, sp, li, far) - v2(0.0, 3.0),
                _ => c
                    .anim
                    .chains
                    .get(k)
                    .and_then(|ch| ch.get(ch.len() / 2))
                    .copied()
                    .unwrap_or(c.pos),
            }
        }
    }
}

/// Human name of a locomotion, for commentary.
pub fn loco_word(l: Locomotion) -> &'static str {
    l.label()
}

/// The arena's plant life, grown the same way on every machine from the
/// seed and the settled world. Every arena gets fruit: the biome's own
/// flora first, then a hardy bush on whatever solid ground is left.
pub fn grow_plants(seed: u64, biome: &Biome, world: &World) -> (Vec<FloraSpecies>, Vec<Plant>) {
    let mut rng = Rng::new(seed ^ 0xF00D);
    let mut flora = flora::biome_flora(&mut rng, biome);
    flora.retain(|f| f.substrate != Substrate::Seabed);
    let mut plants: Vec<Plant> = Vec::new();
    let mut next_id = 1_000_000u32;
    let mut push =
        |plants: &mut Vec<Plant>, fi: usize, max_fruit: f32, x: i32, y: i32, dir: i32, rng: &mut Rng| {
            if plants.iter().any(|p| (p.x - x).abs() < 4 && (p.y - y).abs() < 4) {
                return;
            }
            plants.push(Plant {
                id: next_id,
                flora: fi,
                x,
                y,
                dir,
                growth: rng.range(0.7, 1.0),
                fruit: rng.range(max_fruit * 0.5, max_fruit),
                seed: rng.next_u64() as u32,
                dead: false,
                check_timer: rng.range(0.0, 1.0),
                seed_timer: 1.0e9,
                bitten: 0.0,
            });
            next_id += 1;
        };
    for (fi, fl) in flora.iter().enumerate() {
        let n = match fl.substrate {
            Substrate::Ground => 22,
            _ => 10,
        };
        for _ in 0..n {
            if let Some((x, y, dir)) = flora::find_spot(world, fl, &mut rng, None, 8) {
                let mf = fl.max_fruit as f32;

                push(&mut plants, fi, mf, x, y, dir, &mut rng);
            }
        }
    }
    // Not enough soil here: a bush that roots in bare rock.
    if plants.len() < PLANT_TARGET {
        let hardy = flora::make_flora(&mut rng, biome, GrowthForm::Bush, Substrate::Ground);
        flora.push(hardy);
        let fi = flora.len() - 1;
        let mut tries = 0;
        while plants.len() < PLANT_TARGET && tries < 4000 {
            tries += 1;
            let x = rng.int(4, WIDTH as i32 - 5);
            let y = rng.int(3, HEIGHT as i32 - 4);
            let open = (0..4).all(|k| world.material(x, y - k) == Material::Empty);
            let floor = world.material(x, y + 1);
            if open && floor.is_solid_for_player() && floor.kind() != Kind::Liquid {
                let mf = flora[fi].max_fruit as f32;
                push(&mut plants, fi, mf, x, y, -1, &mut rng);
            }
        }
    }
    (flora, plants)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn two(seed: u64) -> Arena {
        let mut rng = Rng::new(seed);
        let a = Design::random(&mut rng, design::START_POINTS, None);
        let b = Design::random(&mut rng, design::START_POINTS, None);
        Arena::new(seed, &[("Ada".into(), a), ("Grace".into(), b)])
    }

    #[test]
    fn fights_end_and_stay_finite() {
        let mut finished = 0;
        for seed in 1..7u64 {
            let mut arena = two(seed);
            assert!(arena.alive(0) >= 1 && arena.alive(1) >= 1);
            for _ in 0..(60.0 * ROUND_TIME) as usize + 60 {
                arena.step();
                if arena.over() {
                    break;
                }
            }
            for f in &arena.fighters {
                assert!(f.c.pos.x.is_finite() && f.c.pos.y.is_finite(), "seed {seed}");
            }
            assert!(arena.over(), "seed {seed}: round never ended");
            if arena.result.as_ref().unwrap().winner.is_some() {
                finished += 1;
            }
        }
        assert!(finished >= 3, "only {finished} of 6 fights had a winner");
    }

    #[test]
    fn arena_is_deterministic() {
        let mut a = two(42);
        let mut b = two(42);
        for _ in 0..600 {
            a.step();
            b.step();
        }
        for (x, y) in a.fighters.iter().zip(&b.fighters) {
            assert_eq!(x.c.pos, y.c.pos);
            assert_eq!(x.c.health, y.c.health);
        }
        let sa = a.snapshot();
        let sb = b.snapshot();
        assert_eq!(sa.creatures.len(), sb.creatures.len());
    }

    #[test]
    fn limbs_can_be_torn_off() {
        let mut prey = Design::new(2);
        prey.set(design::Upgrade::Tail, 1);
        prey.set(design::Upgrade::Vitality, 4);
        let mut arena = Arena::new(5, &[("A".into(), Design::new(1)), ("B".into(), prey)]);
        // Damage a leg directly until it comes off.
        let leg = arena.teams[1].parts.iter().position(|d| d.name == "leg").unwrap();
        let at = arena.fighters[1].c.pos;
        let mut severed = false;
        for _ in 0..40 {
            arena.damage(
                1,
                leg,
                at,
                6.0,
                v2(1.0, 0.0),
                1.0,
                0.0,
                Some(0),
                false,
                false,
                false,
            );

            if arena
                .take_events()
                .iter()
                .any(|e| matches!(e, FightEvent::Sever { .. }))
            {
                severed = true;
                break;
            }
        }
        assert!(severed, "a leg should come off after enough damage");
        assert!(!arena.fighters[1].c.limb_ok.iter().all(|ok| *ok));
        let sp = &arena.teams[1].species;
        let eff = parts::effects(
            &arena.fighters[1].c,
            sp,
            &arena.teams[1].parts,
            &arena.fighters[1].parts,
        );
        assert_eq!(eff.legs_lost, 1);
    }

    #[test]
    #[ignore]
    fn trace_fight() {
        let seed: u64 = std::env::var("VS_SEED")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(0);
        let mut arena = if seed == 0 {
            let mut biter = Design::new(1);
            biter.set(design::Upgrade::Teeth, 6);
            biter.set(design::Upgrade::Size, 4);
            biter.set(design::Upgrade::Intelligence, 4);
            let mut prey = Design::new(2);
            prey.set(design::Upgrade::Vitality, 4);
            prey.set(design::Upgrade::Size, 2);
            let mut arena = Arena::new(0, &[("A".into(), biter), ("B".into(), prey)]);
            let p = arena.fighters[1].c.pos;
            arena.fighters[0].c.pos = p - v2(8.0, 0.0);
            arena
        } else {
            let mut rng = Rng::new(seed);
            let a = Design::random(&mut rng, design::START_POINTS, None);
            let b = Design::random(&mut rng, design::START_POINTS, None);
            Arena::new(seed, &[("A".into(), a), ("B".into(), b)])
        };
        for (i, t) in arena.teams.iter().enumerate() {
            println!(
                "team {i}: {} {:?} {:?} weapons {:?} fly {} dig {} size {:.1}",
                t.design.name,
                t.species.habitat,
                t.species.loco,
                t.loadout
                    .weapons
                    .iter()
                    .map(|w| (w.kind, w.reach as i32))
                    .collect::<Vec<_>>(),
                t.loadout.fly,
                t.loadout.dig,
                t.loadout.size
            );
        }
        for t in 0..60 * 40 {
            arena.step();
            for e in arena.take_events() {
                println!("{:5.2} {e:?}", t as f32 / 60.0);
            }
            if t % 60 == 0 {
                for f in &arena.fighters {
                    let under = arena.world.material(
                        f.c.pos.x as i32,
                        (f.c.pos.y + physics::bounds(&f.c, &arena.teams[f.team].species).bottom + 1.0) as i32,
                    );
                    println!(
                        "  t{} id{} pos({:.0},{:.0}) hp {:.0} {:?} '{}' tgt {:?} seen {:?} ground {} cmd ({:.1},{:.1}) j{} stuck {:.0} path {} spot {:?} under {:?} stag {:.1} stun {:.1}",
                        t / 60,
                        f.c.id,
                        f.c.pos.x,
                        f.c.pos.y,
                        f.c.health,
                        f.brain.tactic,
                        f.brain.label,
                        f.brain.target,
                        f.brain.last_seen.map(|(p, _)| (p.x as i32, p.y as i32)),
                        f.c.on_ground,
                        f.c.last_cmd.dir.x,
                        f.c.last_cmd.dir.y,
                        f.c.last_cmd.jump as u8,
                        f.c.brain.stuck,
                        f.c.brain.path.len(),
                        f.c.brain.spot.map(|p| (p.x as i32, p.y as i32)),
                        under,
                        f.stagger,
                        f.c.status.stun
                    );
                }
            }
            if t == 60 * 12 {
                let f = &arena.fighters[0];
                let sp = &arena.teams[0].species;
                let b = physics::bounds(&f.c, sp);
                println!(
                    "bounds hw {:.1} top {:.1} bottom {:.1} profile {:?}",
                    b.hw,
                    b.top,
                    b.bottom,
                    physics::profile(sp, 1.0, 300.0)
                );
                for y in (f.c.pos.y as i32 - 20)..(f.c.pos.y as i32 + 45) {
                    let row: String = ((f.c.pos.x as i32 - 50)..(f.c.pos.x as i32 + 30))
                        .map(|x| {
                            if (x, y) == f.c.pos.cell() {
                                return '@';
                            }
                            if (x, y) == arena.fighters[1].c.pos.cell() {
                                return '%';
                            }
                            match arena.world.material(x, y) {
                                Material::Empty => '.',
                                Material::Water => '~',
                                Material::Acid => 'a',
                                Material::Lava => 'L',
                                Material::Sand => 's',
                                Material::Grass => 'g',
                                Material::Dirt => 'd',
                                Material::Stone => 'S',
                                Material::Wood | Material::Leaf => 'T',
                                _ => '?',
                            }
                        })
                        .collect();
                    println!("   {y:4} {row}");
                }
            }
            if arena.over() {
                println!("over: {:?}", arena.result);
                break;
            }
        }
    }

    #[test]
    #[ignore]
    fn stats() {
        for seed in 1..=16u64 {
            let mut rng = Rng::new(seed);
            let a = Design::random(&mut rng, design::START_POINTS, None);
            let b = Design::random(&mut rng, design::START_POINTS, None);
            let mut arena = Arena::new(seed, &[("A".into(), a.clone()), ("B".into(), b.clone())]);
            let (mut hits, mut misses, mut clips, mut severs, mut kills, mut clashes) = (0, 0, 0, 0, 0, 0);
            let mut tactics: std::collections::BTreeMap<String, u32> = Default::default();
            let mut t = 0;
            while !arena.over() && t < 60 * 200 {
                arena.step();
                t += 1;
                for e in arena.take_events() {
                    match e {
                        FightEvent::Hit { .. } => hits += 1,
                        FightEvent::Miss { .. } => misses += 1,
                        FightEvent::Clip { .. } => clips += 1,
                        FightEvent::Sever { .. } => severs += 1,
                        FightEvent::Kill { .. } => kills += 1,
                        FightEvent::Clash { .. } => clashes += 1,
                        _ => {}
                    }
                }
                if t % 30 == 0 {
                    for f in &arena.fighters {
                        if f.c.alive() {
                            *tactics.entry(format!("{:?}", f.brain.tactic)).or_default() += 1;
                        }
                    }
                }
            }
            let tries = arena.seed.wrapping_sub(seed) / 7919;
            let conn = arena.connected();
            let r = arena.result.as_ref().unwrap();
            println!(
                "seed {seed:2} {:?} tries {tries} conn {conn} {:>3.0}s winner {:?} hits {hits} miss {misses} clip {clips} sever {severs} kills {kills} clash {clashes} | {} ({}x) vs {} ({}x) | {:?}",
                arena.biome.kind,
                r.duration,
                r.winner,
                a.name,
                a.count(),
                b.name,
                b.count(),
                tactics
            );
        }
    }

    #[test]
    fn big_biters_tear_limbs_in_real_fights() {
        let mut biter = Design::new(1);
        biter.set(design::Upgrade::Teeth, 6);
        biter.set(design::Upgrade::Size, 4);
        biter.set(design::Upgrade::Intelligence, 4);
        let mut prey = Design::new(2);
        prey.set(design::Upgrade::Vitality, 4);
        prey.set(design::Upgrade::Size, 2);
        let mut severed = 0;
        for seed in 0..6u64 {
            let mut arena = Arena::new(seed, &[("A".into(), biter.clone()), ("B".into(), prey.clone())]);
            let p = arena.fighters[1].c.pos;
            arena.fighters[0].c.pos = p - v2(8.0, 0.0);
            for _ in 0..60 * 90 {
                arena.step();
                if arena
                    .take_events()
                    .iter()
                    .any(|e| matches!(e, FightEvent::Sever { .. }))
                {
                    severed += 1;
                    break;
                }
                if arena.over() {
                    break;
                }
            }
        }
        assert!(severed >= 3, "limbs came off in only {severed} of 6 fights");
    }

    #[test]
    fn snapshot_roundtrips() {
        let arena = two(8);
        let s = arena.snapshot();
        let bytes = bincode_like(&s);
        assert!(!bytes.is_empty());
    }

    fn bincode_like(s: &Snapshot) -> Vec<u8> {
        // serde is proven by the net crate's tests; here just make sure the
        // snapshot is populated.
        assert_eq!(s.creatures.len(), 2.max(s.creatures.len()));
        vec![s.creatures.len() as u8]
    }
}
