//! A watcher's copy of the arena, driven by snapshots and events from the
//! authoritative [`super::Arena`]. It smooths positions between snapshots,
//! runs the procedural animation and particles locally, and knows what to
//! shout about and what to play a sound for.

use crate::eco::biome::{self, Biome};
use crate::eco::body::{self, Paint, Prim};
use crate::eco::creature::{Creature, init_anim, limb_instances};
use crate::eco::flora::{FloraSpecies, Plant};
use crate::eco::genome::{LimbKind, Species, SpitKind};
use crate::eco::math::{Rgb, V2, mix, scale, v2};
use crate::eco::nav::NavGrid;
use crate::eco::render::Scene;
use crate::eco::{Particle, Projectile};
use crate::material::Material;
use crate::rng::Rng;
use crate::world::{DecodeError, World};

use super::combat::Tactic;
use super::design::{Design, Loadout, build_teams};
use super::parts::{self, PartDef};
use super::{FightEvent, Snapshot, grow_plants};

pub struct WatchedTeam {
    pub player: String,
    pub design: Design,
    pub species: Species,
    pub loadout: Loadout,
    pub parts: Vec<PartDef>,
}

/// A severed limb tumbling through the air.
#[derive(Clone, Debug)]
pub struct Gib {
    /// Segments relative to the centre, with widths.
    pub segs: Vec<(V2, V2, f32)>,
    pub colour: Rgb,
    pub pos: V2,
    pub vel: V2,
    pub angle: f32,
    pub spin: f32,
    pub life: f32,
    pub settled: bool,
}

#[derive(Clone, Debug)]
pub struct Callout {
    pub text: String,
    /// World position; `None` means the centre of the screen.
    pub pos: Option<V2>,
    pub life: f32,
    pub max_life: f32,
    pub colour: Rgb,
    pub big: bool,
}

/// A sound the watcher should play: its name and a volume.
pub type SoundCue = (&'static str, f32);

pub struct Mirror {
    pub seed: u64,
    pub biome: Biome,
    pub world: World,
    pub nav: NavGrid,
    pub flora: Vec<FloraSpecies>,
    pub plants: Vec<Plant>,
    /// The living plants, for drawing.
    drawn_plants: Vec<Plant>,
    pub teams: Vec<WatchedTeam>,
    pub species: Vec<Species>,
    pub creatures: Vec<Creature>,
    /// Per creature, matching `creatures`: health fraction, part fractions,
    /// tactic, energy, flight stamina.
    pub health: Vec<f32>,
    pub parts: Vec<Vec<u8>>,
    pub tactics: Vec<Tactic>,
    pub energy: Vec<f32>,
    pub stamina: Vec<f32>,
    targets: Vec<(V2, V2)>,
    pub projectiles: Vec<Projectile>,
    pub particles: Vec<Particle>,
    pub gibs: Vec<Gib>,
    pub extras: Vec<(Prim, Rgb, bool)>,
    pub callouts: Vec<Callout>,
    pub sounds: Vec<SoundCue>,
    pub time: f32,
    pub clock: f32,
    pub time_scale: f32,
    pub frenzy: bool,
    /// Camera cues: shake strength, white flash, a point worth looking at.
    pub shake: f32,
    pub flash: f32,
    pub focus: Option<(V2, f32)>,
    pub finished: bool,
    fresh: bool,
    rng: Rng,
    nav_timer: f32,
}

impl Mirror {
    pub fn new(seed: u64, designs: &[(String, Design)]) -> Mirror {
        let (biome, mut world) = biome::generate(seed, None);
        for _ in 0..90 {
            world.step();
        }
        let (flora, plants) = grow_plants(seed, &biome, &world);
        let teams: Vec<WatchedTeam> = build_teams(designs, Some(&biome))
            .into_iter()
            .zip(designs)
            .map(|((species, loadout), (player, design))| {
                let parts = parts::layout(&species, species.stats.max_health);
                WatchedTeam {
                    player: player.clone(),
                    design: design.clone(),
                    species,
                    loadout,
                    parts,
                }
            })
            .collect();
        let species = teams.iter().map(|t| t.species.clone()).collect();
        let nav = NavGrid::new(&world);
        Mirror {
            seed,
            biome,
            world,
            nav,
            drawn_plants: plants.clone(),
            flora,
            plants,
            teams,
            species,
            creatures: Vec::new(),
            health: Vec::new(),
            parts: Vec::new(),
            tactics: Vec::new(),
            energy: Vec::new(),
            stamina: Vec::new(),
            targets: Vec::new(),
            projectiles: Vec::new(),
            particles: Vec::new(),
            gibs: Vec::new(),
            extras: Vec::new(),
            callouts: Vec::new(),
            sounds: Vec::new(),
            time: 0.0,
            clock: 0.0,
            time_scale: 1.0,
            frenzy: false,
            shake: 0.0,
            flash: 0.0,
            focus: None,
            finished: false,
            fresh: true,
            rng: Rng::new(seed ^ 0x717),
            nav_timer: 0.0,
        }
    }

    pub fn apply_chunk(&mut self, cx: usize, cy: usize, data: &[u8]) -> Result<(), DecodeError> {
        self.world.decode_chunk(cx, cy, data)
    }

    pub fn day_phase(&self) -> f32 {
        // Matches the arena: it starts the day at a seeded phase.
        let mut rng = Rng::new(self.seed ^ 0xA2E7A);
        let day0 = rng.range(0.0, 1.0);
        (day0 + self.time / self.biome.day_length).fract()
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

    /// Feeds a snapshot from the host.
    pub fn apply(&mut self, snap: &Snapshot) {
        self.time = snap.time;
        self.clock = snap.clock;
        self.time_scale = snap.time_scale;
        self.frenzy = snap.frenzy;
        // Match creatures by id; make new ones as needed.
        let fresh = self.fresh;
        for n in &snap.creatures {
            let idx = match self.creatures.iter().position(|c| c.id == n.id) {
                Some(i) => i,
                None => {
                    let sp = &self.species[(n.team as usize).min(self.species.len() - 1)];
                    let mut c = Creature::new(n.id, sp, v2(n.x, n.y), true, 0);
                    c.facing = n.facing as f32;
                    init_anim(&mut c, sp);
                    self.creatures.push(c);
                    self.health.push(1.0);
                    self.parts.push(Vec::new());
                    self.tactics.push(Tactic::Search);
                    self.energy.push(1.0);
                    self.stamina.push(1.0);
                    self.targets.push((v2(n.x, n.y), V2::ZERO));
                    self.creatures.len() - 1
                }
            };
            let c = &mut self.creatures[idx];
            self.targets[idx] = (v2(n.x, n.y), v2(n.vx, n.vy));
            if fresh || c.pos.dist(v2(n.x, n.y)) > 40.0 {
                c.pos = v2(n.x, n.y);
            }
            c.vel = v2(n.vx, n.vy);
            c.facing = if n.facing < 0 { -1.0 } else { 1.0 };
            c.on_ground = n.flags & 1 != 0;
            c.flying = n.flags & 2 != 0;
            c.clinging = n.flags & 4 != 0;
            c.burrowed = n.flags & 8 != 0;
            c.in_liquid = n.flags & 16 != 0;
            let dead = n.flags & 32 != 0;
            c.dead = if dead { Some("killed") } else { None };
            c.still = if n.flags & 64 != 0 { 2.0 } else { 0.0 };
            c.status.stun = if n.flags & 128 != 0 { 1.0 } else { 0.0 };
            c.anim.attack = c.anim.attack.max(n.attack as f32 / 255.0);
            c.anim.hurt = c.anim.hurt.max(n.hurt as f32 / 255.0);
            c.status.poison = n.poison as f32 / 20.0;
            c.status.burning = n.burning as f32 / 40.0;
            c.webbed = n.webbed as f32 / 60.0;
            c.health = n.health as f32 / 255.0 * self.species[n.team as usize].stats.max_health;
            c.energy = n.energy as f32 / 255.0;
            let limbs = limb_instances(&self.species[n.team as usize]).len();
            c.limb_ok = (0..limbs).map(|k| n.limbs & (1 << k.min(31)) != 0).collect();
            c.anim.reach = if n.reach_limb == 255 {
                None
            } else {
                Some((n.reach_limb as usize, v2(n.reach_x, n.reach_y)))
            };
            self.health[idx] = n.health as f32 / 255.0;
            self.parts[idx] = n.parts.clone();
            self.tactics[idx] = Tactic::from_index(n.tactic as usize);
            self.energy[idx] = n.energy as f32 / 255.0;
            self.stamina[idx] = n.stamina as f32 / 255.0;
        }
        self.projectiles = snap
            .projectiles
            .iter()
            .map(|p| Projectile {
                kind: match p.kind {
                    0 => SpitKind::Acid,
                    1 => SpitKind::Fire,
                    2 => SpitKind::Venom,
                    _ => SpitKind::Web,
                },
                pos: v2(p.x, p.y),
                vel: v2(p.vx, p.vy),
                owner: 0,
                owner_species: 0,
                life: 1.0,
                damage: 0.0,
            })
            .collect();
        let mut plants_changed = false;
        for (p, (growth, fruit)) in self.plants.iter_mut().zip(&snap.plants) {
            let dead = *growth == 0;
            if dead != p.dead {
                plants_changed = true;
            }
            p.dead = dead;
            p.growth = *growth as f32 / 255.0;
            p.fruit = *fruit as f32;
        }
        if plants_changed || fresh {
            self.drawn_plants = self.plants.iter().filter(|p| !p.dead).cloned().collect();
        }
        self.fresh = false;
    }

    fn creature_index(&self, id: u32) -> Option<usize> {
        self.creatures.iter().position(|c| c.id == id)
    }

    fn team_colour(&self, team: usize) -> Rgb {
        self.species.get(team).map_or([200, 200, 200], |s| s.colors.base)
    }

    fn blood_of(&self, victim: u32) -> Rgb {
        self.creature_index(victim)
            .map(|i| {
                mix(
                    self.species[self.creatures[i].species].colors.accent,
                    [150, 20, 40],
                    0.5,
                )
            })
            .unwrap_or([150, 30, 40])
    }

    /// Reacts to something that happened: particles, gibs, callouts, cues.
    /// Only the finishing blow gets the full treatment.
    pub fn apply_event(&mut self, ev: &FightEvent) {
        match ev {
            FightEvent::Hit {
                x,
                y,
                dx,
                dy,
                damage,
                victim,
                heavy,
                blocked,
                surprise,
                ..
            } => {
                let at = v2(*x, *y);
                let colour = self.blood_of(*victim);
                let n = (*damage / 2.0).clamp(3.0, 12.0) as usize;
                let dir = v2(*dx, *dy);
                if *blocked {
                    self.spark(at, 5, [255, 240, 200]);
                    self.callout("blocked", Some(at), [220, 220, 230], false, 0.7);
                    self.sound("blocked", 0.7);
                } else {
                    self.burst(at, dir, n, colour, 0.6);
                    self.sound(if *heavy { "hit_heavy" } else { "hit" }, 0.8);
                }
                if *surprise {
                    self.callout("AMBUSH", Some(at), [255, 230, 120], false, 1.1);
                    self.sound("surprise", 0.9);
                } else if *heavy {
                    self.callout(&format!("{:.0}", damage), Some(at), [255, 230, 120], false, 0.9);
                }
            }
            FightEvent::Clip { x, y } => {
                let at = v2(*x, *y);
                self.spark(at, 8, [255, 220, 150]);
                self.callout("clipped the terrain", Some(at), [255, 200, 120], false, 1.0);
                self.sound("clip", 0.6);
            }
            FightEvent::Miss { .. } => {
                self.sound("whoosh", 0.35);
            }
            FightEvent::Clash { x, y } => {
                let at = v2(*x, *y);
                self.spark(at, 14, [255, 250, 220]);
                self.callout("clash", Some(at), [255, 240, 200], false, 0.9);
                self.sound("clash", 0.8);
            }
            FightEvent::Sever {
                victim,
                limb,
                name,
                x,
                y,
            } => {
                // Plain: the limb falls off and that's that.
                let at = v2(*x, *y);
                if let Some(i) = self.creature_index(*victim) {
                    self.spawn_gib(i, *limb as usize, at);
                    if let Some(ok) = self.creatures[i].limb_ok.get_mut(*limb as usize) {
                        *ok = false;
                    }
                    let colour = self.blood_of(*victim);
                    self.burst(at, v2(0.0, -1.0), 10, colour, 0.8);
                }
                self.callout(&format!("{name} torn off"), Some(at), [255, 160, 140], false, 1.0);
                self.sound("sever", 0.7);
            }
            FightEvent::Kill {
                victim,
                x,
                y,
                last,
                cause,
                ..
            } => {
                let at = v2(*x, *y);
                let colour = self.blood_of(*victim);
                if *last {
                    // The one moment that gets the works.
                    self.burst(at, v2(0.0, -1.0), 36, colour, 1.3);
                    self.shake = self.shake.max(1.0);
                    self.flash = self.flash.max(0.6);
                    self.focus = Some((at, 2.2));
                    self.callout("FINISHING BLOW", None, [255, 90, 80], true, 2.6);
                    self.sound("finish", 1.0);
                    self.finished = true;
                } else {
                    self.burst(at, v2(0.0, -1.0), 14, colour, 0.9);
                    let text = if cause.starts_with("felled") {
                        "down".to_string()
                    } else {
                        cause.clone()
                    };
                    self.callout(&text, Some(at), [255, 150, 120], false, 1.2);
                    self.sound("down", 0.8);
                }
            }
            FightEvent::Slam { x, y, damage } => {
                let at = v2(*x, *y);
                self.spark(at, 6, [230, 220, 200]);
                self.burst(at, v2(0.0, -1.0), 6, [120, 100, 80], 0.5);
                if *damage > 8.0 {
                    self.callout("slam", Some(at), [255, 230, 160], false, 0.8);
                }
                self.sound("slam", 0.7);
            }
            FightEvent::Spit { .. } => {
                self.sound("spit", 0.5);
            }
            FightEvent::Grab { x, y } => {
                self.callout("grabbed", Some(v2(*x, *y)), [220, 200, 255], false, 0.9);
                self.sound("grab", 0.6);
            }
            FightEvent::Ink { x, y } => {
                self.burst(v2(*x, *y), v2(0.0, -1.0), 20, [40, 40, 50], 1.0);
                self.sound("ink", 0.6);
            }
            FightEvent::Poisoned { victim } => {
                if let Some(i) = self.creature_index(*victim) {
                    let p = self.creatures[i].pos;
                    self.callout("poisoned", Some(p), [150, 230, 100], false, 0.9);
                    self.sound("poison", 0.5);
                }
            }
            FightEvent::Eat {
                x, y, meat, plant, ..
            } => {
                let at = v2(*x, *y);
                let colour = match plant {
                    Some(pi) => self
                        .plants
                        .get(*pi as usize)
                        .map_or([200, 120, 120], |p| self.flora[p.flora].fruit),
                    None => [150, 40, 50],
                };
                if let Some(p) = plant.and_then(|pi| self.plants.get_mut(pi as usize)) {
                    p.bitten = 0.4;
                    p.fruit = (p.fruit - 1.0).max(0.0);
                }
                self.burst(at, v2(0.0, -1.0), 3, colour, 0.5);
                self.sound(if *meat { "eat_meat" } else { "eat" }, 0.5);
            }
            FightEvent::Starving { victim } => {
                if let Some(i) = self.creature_index(*victim) {
                    let p = self.creatures[i].pos;
                    self.callout("starving", Some(p), [255, 190, 90], false, 1.3);
                    self.sound("starving", 0.6);
                }
            }
            FightEvent::Frenzy => {
                self.callout("FRENZY", None, [255, 200, 60], true, 2.0);
                self.sound("frenzy", 0.9);
            }
            FightEvent::RoundOver { .. } => {}
        }
    }

    fn sound(&mut self, name: &'static str, volume: f32) {
        self.sounds.push((name, volume));
    }

    /// Hands over the sounds queued since the last call.
    pub fn take_sounds(&mut self) -> Vec<SoundCue> {
        std::mem::take(&mut self.sounds)
    }

    fn callout(&mut self, text: &str, pos: Option<V2>, colour: Rgb, big: bool, life: f32) {
        if big && self.callouts.iter().any(|c| c.big && c.text == text) {
            return;
        }
        // The same words near the same spot just get refreshed, not stacked.
        if let Some(p) = pos
            && let Some(c) = self
                .callouts
                .iter_mut()
                .find(|c| c.text == text && c.pos.is_some_and(|q| q.dist(p) < 14.0))
        {
            c.life = life;
            c.max_life = life;
            c.pos = Some(p);
            return;
        }
        self.callouts.push(Callout {
            text: text.to_string(),
            pos,
            life,
            max_life: life,
            colour,
            big,
        });
    }

    fn burst(&mut self, at: V2, dir: V2, n: usize, colour: Rgb, life: f32) {
        for _ in 0..n {
            let spread = v2(self.rng.range(-1.0, 1.0), self.rng.range(-1.0, 1.0));
            let v = (dir * 1.5 + spread).norm() * self.rng.range(20.0, 70.0);
            self.particles.push(Particle {
                pos: at,
                vel: v,
                life: life * self.rng.range(0.6, 1.0),
                max_life: life,
                color: colour,
                glow: false,
                gravity: 220.0,
            });
        }
    }

    fn spark(&mut self, at: V2, n: usize, colour: Rgb) {
        for _ in 0..n {
            let a = self.rng.range(0.0, std::f32::consts::TAU);
            self.particles.push(Particle {
                pos: at,
                vel: V2::from_angle(a) * self.rng.range(40.0, 110.0),
                life: 0.35,
                max_life: 0.35,
                color: colour,
                glow: true,
                gravity: 120.0,
            });
        }
    }

    /// Makes a tumbling copy of limb instance `k` of creature `i`.
    fn spawn_gib(&mut self, i: usize, k: usize, at: V2) {
        let c = &self.creatures[i];
        let sp = &self.species[c.species];
        let insts = limb_instances(sp);
        let Some(&(li, _)) = insts.get(k) else { return };
        let l = &sp.body.limbs[li];
        let w = l.width * c.scale(sp).max(0.7);
        let mut pts: Vec<V2> = match l.kind {
            LimbKind::Leg => {
                let foot = c.anim.feet.get(k).map_or(at, |f| f.pos);
                vec![at, foot]
            }
            LimbKind::Wing => vec![at, at + v2(-c.facing * l.length * 0.6, -l.length * 0.5)],
            _ => c
                .anim
                .chains
                .get(k)
                .cloned()
                .unwrap_or_else(|| vec![at, at + v2(0.0, 3.0)]),
        };
        if pts.len() < 2 {
            pts.push(at + v2(1.0, 1.0));
        }
        let centre = pts.iter().fold(V2::ZERO, |a, p| a + *p) / pts.len() as f32;
        let segs = pts
            .windows(2)
            .map(|w2| (w2[0] - centre, w2[1] - centre, w))
            .collect();
        let colour = mix(sp.colors.base, sp.colors.accent, 0.35);
        self.gibs.push(Gib {
            segs,
            colour,
            pos: centre,
            vel: v2(self.rng.range(-30.0, 30.0), self.rng.range(-70.0, -30.0)),
            angle: 0.0,
            spin: self.rng.range(-8.0, 8.0),
            life: 14.0,
            settled: false,
        });
    }

    /// Advances presentation by `real_dt` seconds.
    pub fn advance(&mut self, real_dt: f32) {
        let real_dt = real_dt.min(0.1);
        let dt = real_dt * self.time_scale.max(0.0);
        self.nav_timer += real_dt;
        if self.nav_timer > 0.5 {
            self.nav_timer = 0.0;
            self.nav.rebuild(&self.world);
        }
        // Smooth towards the host's positions.
        let k = 1.0 - (-real_dt * 16.0).exp();
        for (i, c) in self.creatures.iter_mut().enumerate() {
            let (tp, tv) = self.targets[i];
            c.pos = c.pos.lerp(tp, k);
            c.vel = tv;
            let sp = &self.species[c.species];
            let flying = c.flying;
            body::animate(c, sp, &self.world, dt.max(1e-4), flying);
            // Fade the host's attack/hurt timers like the arena would.
            c.anim.attack = (c.anim.attack - dt).max(0.0);
            c.anim.hurt = (c.anim.hurt - dt).max(0.0);
            c.anim.eat = (c.anim.eat - dt).max(0.0);
        }
        for p in self.drawn_plants.iter_mut() {
            p.bitten = (p.bitten - dt).max(0.0);
            if let Some(src) = self.plants.iter().find(|q| q.id == p.id) {
                p.fruit = src.fruit;
                p.growth = src.growth;
                p.bitten = p.bitten.max(src.bitten);
            }
        }
        for p in self.plants.iter_mut() {
            p.bitten = (p.bitten - dt).max(0.0);
        }
        // Particles and gibs.
        for p in self.particles.iter_mut() {
            p.vel.y += p.gravity * dt;
            p.pos += p.vel * dt;
            p.life -= dt;
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
        let g = 300.0 * self.biome.gravity;
        for gib in self.gibs.iter_mut() {
            gib.life -= real_dt;
            if gib.settled {
                continue;
            }
            gib.vel.y += g * dt;
            let next = gib.pos + gib.vel * dt;
            let below = world.material(next.x as i32, (next.y + 1.5) as i32);
            if below.is_solid_for_player()
                || world.material(next.x as i32, next.y as i32).is_solid_for_player()
            {
                gib.vel = V2::ZERO;
                gib.spin = 0.0;
                gib.settled = true;
            } else {
                gib.pos = next;
                gib.angle += gib.spin * dt;
                gib.spin *= 0.995;
            }
        }
        self.gibs
            .retain(|g| g.life > 0.0 && world.in_bounds(g.pos.x as i32, g.pos.y as i32));
        // Callouts and camera cues run on real time.
        for c in self.callouts.iter_mut() {
            c.life -= real_dt;
            if let Some(p) = &mut c.pos {
                p.y -= real_dt * 8.0;
            }
        }
        self.callouts.retain(|c| c.life > 0.0);
        self.shake = (self.shake - real_dt * 2.2).max(0.0);
        self.flash = (self.flash - real_dt * 2.5).max(0.0);
        if let Some((_, t)) = &mut self.focus {
            *t -= real_dt;
            if *t <= 0.0 {
                self.focus = None;
            }
        }
        self.time += dt;
        // Extras: gibs as primitives.
        self.extras.clear();
        for gib in &self.gibs {
            let fade = (gib.life / 3.0).clamp(0.0, 1.0);
            let col = scale(gib.colour, 0.5 + 0.5 * fade);
            for (a, b, w) in &gib.segs {
                self.extras.push((
                    Prim::Line {
                        a: gib.pos + a.rot(gib.angle),
                        b: gib.pos + b.rot(gib.angle),
                        w0: *w + 0.6,
                        w1: *w,
                        paint: Paint::Fixed(col),
                    },
                    col,
                    false,
                ));
            }
        }
    }

    pub fn alive(&self, team: usize) -> usize {
        self.creatures
            .iter()
            .filter(|c| c.species == team && c.alive())
            .count()
    }

    /// Team health as a fraction of what it started with.
    pub fn team_health(&self, team: usize) -> f32 {
        let n = self.teams.get(team).map_or(1, |t| t.loadout.count.max(1)) as f32;
        self.creatures
            .iter()
            .zip(&self.health)
            .filter(|(c, _)| c.species == team && c.alive())
            .map(|(_, h)| *h)
            .sum::<f32>()
            / n
    }

    /// Average energy of a team's survivors, 0..1.
    pub fn team_energy(&self, team: usize) -> f32 {
        let alive: Vec<f32> = self
            .creatures
            .iter()
            .zip(&self.energy)
            .filter(|(c, _)| c.species == team && c.alive())
            .map(|(_, e)| *e)
            .collect();
        if alive.is_empty() {
            0.0
        } else {
            alive.iter().sum::<f32>() / alive.len() as f32
        }
    }

    pub fn scene(&self) -> Scene<'_> {
        Scene {
            seed: self.seed,
            biome: &self.biome,
            world: &self.world,
            nav: &self.nav,
            time: self.time,
            tick: (self.time * 60.0) as u64,
            day_phase: self.day_phase(),
            daylight: self.daylight(),
            dusk: self.dusk(),
            flora: &self.flora,
            plants: &self.drawn_plants,
            carcasses: &[],
            eggs: &[],
            species: &self.species,
            creatures: &self.creatures,
            projectiles: &self.projectiles,
            particles: &self.particles,
            icons: &[],
            extras: &self.extras,
        }
    }

    /// Centre of everything alive, and the box around it, for the camera.
    pub fn bounds(&self) -> Option<(V2, V2)> {
        let alive: Vec<&Creature> = self.creatures.iter().filter(|c| c.alive()).collect();
        if alive.is_empty() {
            return None;
        }
        let mut lo = v2(f32::MAX, f32::MAX);
        let mut hi = v2(f32::MIN, f32::MIN);
        for c in alive {
            lo = v2(lo.x.min(c.pos.x), lo.y.min(c.pos.y));
            hi = v2(hi.x.max(c.pos.x), hi.y.max(c.pos.y));
        }
        Some((lo, hi))
    }

    pub fn team_colour_of(&self, team: usize) -> Rgb {
        self.team_colour(team)
    }

    pub fn material_colour(&self, m: Material) -> Rgb {
        self.biome.palette.mat(m)
    }
}
