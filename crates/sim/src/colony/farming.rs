//! Growing things and running the domes: planters, wild and outdoor plants,
//! the battery flowers that sprout at dawn, kitchens, barns, fish pools,
//! compost vats and Dome Domes, and the oxygen all of it gives off.

use serde::{Deserialize, Serialize};

use super::build::ground_below;
use super::domes::{Animal, AnimalKind, Fixture, fixture_pos, issue};
use super::geom::{V2, v2};
use super::items::{DomeKind, Item, MachineKind};
use super::plants::{Bed, Product, Species, SpeciesId, VOLTBLOOM};
use super::recipes::Ingredient;
use super::{Colony, DAY_SECS, EntKind, Fx, Id, Plant, PlayerKey, WeatherKind};
use crate::material::Material;
use crate::planetgen::Band;
use crate::rng::Rng;
use crate::world::World;

/// Oxygen units per day that raise the atmosphere by one percentage point per day.
pub const O2_PER_PERCENT: f32 = 450.0;
pub const CAN_LITRES: f32 = 10.0;
const FERTILIZER_SECS: f32 = 600.0;
/// Growth a harvested plant restarts from.
const REGROW: f32 = 0.35;
const COOK_SECS: f32 = 40.0;
const COMPOST_SECS: f32 = 30.0;
const WILD_FLOWERS: usize = 46;
pub const GENERATOR_O2: f32 = 40.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PlanterOp {
    /// Sow the seed in the player's hand.
    Plant,
    Harvest,
    /// Water from the player's watering can.
    Water,
    Fertilize,
    /// Dig the plant up and get a seed back.
    Uproot,
}

type Outcome = Result<(), &'static str>;

/// What a harvest of `species` yields.
pub fn harvest_items(sp: &Species, mult: f32, rng: &mut Rng) -> Vec<(Item, u32)> {
    let mut out: Vec<(Item, u32)> = Vec::new();
    fn add(out: &mut Vec<(Item, u32)>, item: Item, n: u32) {
        match out.iter_mut().find(|(i, _)| *i == item) {
            Some((_, c)) => *c += n,
            None => out.push((item, n)),
        }
    }
    let base = sp.harvest_count() as f32 * mult;
    for (product, share) in [(sp.product, 1.0), (sp.second, 0.6)] {
        let Some(p) = product else { continue };
        let n = ((base * share).round() as u32).max(1);
        match p {
            Product::Flower | Product::Crop => add(&mut out, Item::Crop(sp.id), n),
            Product::Wood => add(&mut out, Item::Wood, n * 2),
            Product::Fiber => add(&mut out, Item::Fiber, n * 2),
            Product::Fertilizer => add(&mut out, Item::Fertilizer, n),
            Product::Gel => add(&mut out, Item::BioGel, n),
            Product::Water => {}
        }
    }
    // Plants with nothing to pick give a cutting; others sometimes seed.
    if out.is_empty() || rng.next_f32() < 0.35 * mult {
        add(&mut out, Item::Seed(sp.id), 1);
    }
    out
}

impl Colony {
    fn near_fixture(&self, key: PlayerKey, dome: Id, f: Fixture) -> Outcome {
        let p = &self.players[&key];
        let (e, d) = self.dome(dome).ok_or("That dome is gone")?;
        let pl = d
            .kind
            .fixtures()
            .iter()
            .find(|pl| pl.fixture == f)
            .ok_or("No such fixture")?;
        if fixture_pos(e.pos, pl).distance(p.pose.pos) > p.stats().reach + 12.0 {
            return Err("Too far away");
        }
        Ok(())
    }

    pub(super) fn planter_op(&mut self, key: PlayerKey, dome: Id, slot: u8, op: PlanterOp) -> Outcome {
        self.near_fixture(key, dome, Fixture::Planter(slot))?;
        let (pos, kind) = {
            let (e, d) = self.dome(dome).unwrap();
            (e.pos, d.kind)
        };
        let planter = *self
            .dome(dome)
            .unwrap()
            .1
            .planters
            .get(slot as usize)
            .ok_or("No such planter")?;
        let at = v2(pos.x, pos.y - 10.0);
        match op {
            PlanterOp::Plant => {
                if planter.species.is_some() {
                    return Err("Something already grows here");
                }
                let p = &self.players[&key];
                let Some(Item::Seed(species)) = p.held().map(|s| s.item) else {
                    return Err("Hold a seed");
                };
                let sp = self.species.get(species as usize).ok_or("Unknown seed")?;
                if sp.fit(kind.bed()) <= 0.0 {
                    return Err(match kind.bed() {
                        Bed::Water => "Only aquatic plants grow in a tank",
                        Bed::Dark => "That needs light: plant it in a Green Dome",
                        _ if sp.habitat == super::plants::Habitat::Aquatic => {
                            "That is aquatic: plant it in an Aqua Dome"
                        }
                        _ => "That needs darkness: plant it in a Dark Dome",
                    });
                }
                let slot_index = p.selected as usize;
                self.consume_held(key, slot_index);
                let pl = &mut self.dome_mut(dome).unwrap().planters[slot as usize];
                pl.species = Some(species);
                pl.growth = 0.0;
                self.discover(species);
                self.fx(Fx::Sow, at);
            }
            PlanterOp::Harvest => {
                let species = planter.species.ok_or("Nothing is planted")?;
                if planter.growth < 1.0 {
                    return Err("Not ready yet");
                }
                let sp = self.species[species as usize].clone();
                let items = harvest_items(&sp, 1.0, &mut self.rng);
                for (item, n) in items {
                    self.give(key, item, n);
                }
                self.dome_mut(dome).unwrap().planters[slot as usize].growth = REGROW;
                self.fx(Fx::Harvest, at);
            }
            PlanterOp::Water => {
                let p = self.players.get_mut(&key).unwrap();
                if p.inv.count(Item::WateringCan) == 0 {
                    return Err("You need a Watering Can");
                }
                if p.can_water < 1.0 {
                    return Err("The can is empty: fill it at any water");
                }
                p.can_water -= 1.0;
                self.dome_mut(dome).unwrap().planters[slot as usize].moisture = 1.0;
                self.fx(Fx::Water, at);
            }
            PlanterOp::Fertilize => {
                if planter.fertilizer > FERTILIZER_SECS * 0.5 {
                    return Err("Already fertilized");
                }
                let p = self.players.get_mut(&key).unwrap();
                if !p.inv.remove(Item::Fertilizer, 1) {
                    return Err("You need Fertilizer");
                }
                p.touch();
                self.dome_mut(dome).unwrap().planters[slot as usize].fertilizer = FERTILIZER_SECS;
                self.fx(Fx::Fertilize, at);
            }
            PlanterOp::Uproot => {
                let species = planter.species.ok_or("Nothing is planted")?;
                let pl = &mut self.dome_mut(dome).unwrap().planters[slot as usize];
                pl.species = None;
                pl.growth = 0.0;
                self.give(key, Item::Seed(species), 1);
                self.fx(Fx::Harvest, at);
            }
        }
        Ok(())
    }

    /// Records a species in the codex the first time the colony grows it.
    pub fn discover(&mut self, species: SpeciesId) {
        if self.codex.plants.insert(species) {
            self.meta_rev += 1;
            let name = self
                .species
                .get(species as usize)
                .map_or(String::new(), |s| s.name.clone());
            self.fx(Fx::Discover, V2::ZERO);
            self.toast(None, format!("Codex: {name}"), true);
        }
    }

    /// Harvests a wild or outdoor plant.
    pub(super) fn harvest_plant(&mut self, key: PlayerKey, id: Id) -> Outcome {
        let e = self.ents.get(&id).ok_or("It's gone")?;
        let EntKind::Plant(plant) = e.kind else {
            return Err("That isn't a plant");
        };
        let p = &self.players[&key];
        if e.pos.distance(p.pose.pos) > p.stats().reach + 10.0 {
            return Err("Too far away");
        }
        if plant.growth < 1.0 {
            return Err("Not ready yet");
        }
        let at = v2(e.pos.x, e.pos.y - 6.0);
        let sp = self.species[plant.species as usize].clone();
        let mut items = harvest_items(&sp, 1.0, &mut self.rng);
        // Wild plants always give a seed the first time the colony meets them.
        if !self.codex.plants.contains(&plant.species)
            && !items.iter().any(|(i, _)| matches!(i, Item::Seed(_)))
        {
            items.push((Item::Seed(plant.species), 1));
        }
        for (item, n) in items {
            self.give(key, item, n);
        }
        self.discover(plant.species);
        if plant.wild && plant.species == VOLTBLOOM {
            // Wild battery flowers are picked whole; new ones come at dawn.
            self.despawn(id);
        } else if let Some(EntKind::Plant(pl)) = self.ents.get_mut(&id).map(|e| &mut e.kind) {
            pl.growth = REGROW;
            self.touch(id);
        }
        self.fx(Fx::Harvest, at);
        Ok(())
    }

    /// The watering can: fills at water, waters a planter or plant it is aimed at.
    pub(super) fn use_can(&mut self, world: &World, key: PlayerKey, at: V2) -> Outcome {
        let (x, y) = at.cell();
        let near_water =
            (-2..=2).any(|dy| (-2..=2).any(|dx| world.material(x + dx, y + dy) == Material::Water));
        if near_water {
            let p = self.players.get_mut(&key).unwrap();
            if p.can_water >= CAN_LITRES {
                return Ok(());
            }
            p.can_water = CAN_LITRES;
            self.fx(Fx::Water, at);
            return Ok(());
        }
        // A planter under the aim?
        let hit = self.domes().find_map(|(e, d)| {
            d.kind.fixtures().iter().find_map(|f| match f.fixture {
                Fixture::Planter(i) if fixture_pos(e.pos, f).distance(v2(at.x, at.y + 6.0)) < 16.0 => {
                    Some((e.id, i))
                }
                _ => None,
            })
        });
        if let Some((dome, slot)) = hit {
            return self.planter_op(key, dome, slot, PlanterOp::Water);
        }
        // Otherwise top up the dome we're standing in.
        let p = &self.players[&key];
        if let Some(dome) = p.in_dome
            && let Some((_, d)) = self.dome(dome)
            && d.kind.water_per_day() > 0.0
            && !matches!(d.kind, DomeKind::Green | DomeKind::Dark)
        {
            if p.can_water < 1.0 {
                return Err("The can is empty: fill it at any water");
            }
            let cap = d.kind.water_per_day();
            let pour = p.can_water.min((cap - d.reserve).max(0.0));
            if pour <= 0.0 {
                return Err("It has all the water it can hold");
            }
            self.players.get_mut(&key).unwrap().can_water -= pour;
            self.dome_mut(dome).unwrap().reserve += pour;
            self.fx(Fx::Water, at);
            return Ok(());
        }
        Err("Aim at water to fill the can, or at a planter to water it")
    }

    /// Plants a seed in open ground.
    pub(super) fn plant_outdoors(
        &mut self,
        world: &World,
        key: PlayerKey,
        slot: usize,
        species: SpeciesId,
        at: V2,
    ) -> Outcome {
        let (x, y) = at.cell();
        if self.dome_at(at).is_some() {
            return Err("Use a planter inside domes");
        }
        let ground = ground_below(world, x, y - 4, 40).ok_or("Aim at the ground")?;
        if !matches!(
            world.material(x, ground),
            Material::Dirt | Material::Grass | Material::Moss | Material::Sand
        ) {
            return Err("It needs soil");
        }
        let pos = v2(x as f32 + 0.5, ground as f32);
        let crowded = self
            .ents
            .values()
            .any(|e| matches!(e.kind, EntKind::Plant(_)) && e.pos.distance(pos) < 10.0);
        if crowded {
            return Err("Too close to another plant");
        }
        let sp = self.species.get(species as usize).ok_or("Unknown seed")?;
        if sp.fit(Bed::Outdoors) <= 0.0 {
            return Err("That won't survive outside");
        }
        self.spawn(
            pos,
            EntKind::Plant(Plant {
                species,
                growth: 0.0,
                wild: false,
            }),
        );
        self.consume_held(key, slot);
        self.discover(species);
        self.fx(Fx::Sow, pos);
        Ok(())
    }

    /// Puts the animal or fish fry in the player's hand into the dome they
    /// are standing in.
    pub(super) fn stock(&mut self, key: PlayerKey) -> Outcome {
        let p = &self.players[&key];
        let dome = p.in_dome.ok_or("Stand in a Barn or Aqua Dome")?;
        let slot = p.selected as usize;
        let held = p.held().map(|s| s.item).ok_or("Hold an animal")?;
        let kind = self.dome(dome).ok_or("That dome is gone")?.1.kind;
        match (held, kind) {
            (Item::Cluckbug | Item::MilkGrub, DomeKind::Barn) => {
                let d = self.dome_mut(dome).unwrap();
                if d.animals.len() >= 4 {
                    return Err("All four stalls are taken");
                }
                d.animals.push(Animal {
                    kind: if held == Item::Cluckbug {
                        AnimalKind::Cluckbug
                    } else {
                        AnimalKind::MilkGrub
                    },
                    progress: 0.0,
                    fed: false,
                });
            }
            (Item::FishFry, DomeKind::Aqua) => {
                let d = self.dome_mut(dome).unwrap();
                if d.fish >= 12 {
                    return Err("The pool is full");
                }
                d.fish += 1;
            }
            (Item::Cluckbug | Item::MilkGrub, _) => return Err("Animals live in a Barn Dome"),
            (Item::FishFry, _) => return Err("Fish live in an Aqua Dome"),
            _ => return Err("Hold an animal"),
        }
        self.consume_held(key, slot);
        let at = self.players[&key].center();
        self.fx(Fx::Place, at);
        Ok(())
    }

    pub(super) fn set_target(&mut self, dome: Id, target: Option<DomeKind>) -> Outcome {
        let (_, d) = self.dome(dome).ok_or("That dome is gone")?;
        if d.kind != DomeKind::DomeDome {
            return Err("Only a Dome Dome builds domes");
        }
        if target == Some(DomeKind::Starter) {
            return Err("It can't build that");
        }
        let d = self.dome_mut(dome).unwrap();
        if d.target != target {
            d.target = target;
            d.progress = 0.0;
        }
        Ok(())
    }

    // ---- wild flora --------------------------------------------------------

    /// Scatters the planet's native plants at founding.
    pub(super) fn seed_wild_flora(&mut self, world: &World) {
        let w = self.profile.width;
        // Surface plants, by name so the table can be reordered safely.
        let surface = [
            "Breathfern",
            "Ration Root",
            "Amber Wheat",
            "Fiber Vine",
            "Lung Moss",
            "Thornshield",
            "Ironbark Sapling",
        ];
        let cave = ["Glowcap", "Sporebloom", "Night Orchid"];
        let id_of = |c: &Colony, name: &str| c.species.iter().find(|s| s.name == name).map(|s| s.id);
        let mut x = 60;
        while x < w - 60 {
            x += self.rng.range(50, 130);
            let name = surface[self.rng.range(0, surface.len() as i32) as usize];
            if let (Some(species), Some(pos)) = (id_of(self, name), self.surface_spot(world, x)) {
                self.spawn(
                    pos,
                    EntKind::Plant(Plant {
                        species,
                        growth: 1.0,
                        wild: true,
                    }),
                );
            }
        }
        // Reeds on the shores, kelp and lotus in the lakes.
        for lake in self.profile.lakes.clone() {
            for (name, dx) in [("Sugar Reed", -14), ("Sugar Reed", 14)] {
                let x = if dx < 0 { lake.x0 + dx } else { lake.x1 + dx };
                if let (Some(species), Some(pos)) = (id_of(self, name), self.surface_spot(world, x)) {
                    self.spawn(
                        pos,
                        EntKind::Plant(Plant {
                            species,
                            growth: 1.0,
                            wild: true,
                        }),
                    );
                }
            }
            for (name, t) in [
                ("Kelp Ribbon", 0.35),
                ("Bubble Lotus", 0.6),
                ("Kelp Ribbon", 0.75),
            ] {
                let x = lake.x0 + ((lake.x1 - lake.x0) as f32 * t) as i32;
                let bed = self.profile.surface_at(x);
                if let Some(species) = id_of(self, name) {
                    self.spawn(
                        v2(x as f32, bed as f32),
                        EntKind::Plant(Plant {
                            species,
                            growth: 1.0,
                            wild: true,
                        }),
                    );
                }
            }
        }
        // Dark species on cave floors.
        let mut placed = 0;
        for _ in 0..4000 {
            if placed >= (w / 90) as usize {
                break;
            }
            let x = self.rng.range(30, w - 30);
            let y = self
                .rng
                .range(self.profile.surface_at(x) + 60, self.profile.abyss_y);
            if world.material(x, y) != Material::Empty || self.profile.band(x, y) == Band::Abyss {
                continue;
            }
            let Some(floor) = ground_below(world, x, y, 30) else {
                continue;
            };
            if !(1..=10).all(|d| world.material(x, floor - d).is_open()) {
                continue;
            }
            let name = cave[self.rng.range(0, cave.len() as i32) as usize];
            if let Some(species) = id_of(self, name) {
                self.spawn(
                    v2(x as f32, floor as f32),
                    EntKind::Plant(Plant {
                        species,
                        growth: 1.0,
                        wild: true,
                    }),
                );
                placed += 1;
            }
        }
        self.spawn_dawn_flowers(world);
    }

    /// A spot on the open surface near column `x` where a plant can stand.
    fn surface_spot(&self, world: &World, x: i32) -> Option<V2> {
        if x < 10 || x >= self.profile.width - 10 || self.profile.lake_at(x).is_some() {
            return None;
        }
        let top = self.profile.surface_at(x);
        let ground = ground_below(world, x, top - 30, 60)?;
        if !matches!(
            world.material(x, ground),
            Material::Grass | Material::Dirt | Material::Moss | Material::Sand
        ) {
            return None;
        }
        // Open air above, and not inside or under a dome.
        if !(1..=24).all(|d| world.material(x, ground - d).is_open()) {
            return None;
        }
        let pos = v2(x as f32 + 0.5, ground as f32);
        if self
            .dome_rects()
            .iter()
            .any(|r| r.grown(12).contains(v2(pos.x, pos.y - 4.0)))
        {
            return None;
        }
        let crowded = self
            .ents
            .values()
            .any(|e| matches!(e.kind, EntKind::Plant(_)) && e.pos.distance(pos) < 26.0);
        (!crowded).then_some(pos)
    }

    /// Battery flowers sprout across the surface each dawn.
    pub(super) fn spawn_dawn_flowers(&mut self, world: &World) {
        let wild = self
            .ents
            .values()
            .filter(|e| matches!(e.kind, EntKind::Plant(p) if p.wild && p.species == VOLTBLOOM))
            .count();
        let want = (WILD_FLOWERS * self.profile.width as usize / 5120).max(6);
        let mut new = 0;
        for _ in 0..want * 12 {
            if wild + new >= want {
                break;
            }
            let x = self.rng.range(20, self.profile.width - 20);
            if let Some(pos) = self.surface_spot(world, x) {
                self.spawn(
                    pos,
                    EntKind::Plant(Plant {
                        species: VOLTBLOOM,
                        growth: 1.0,
                        wild: true,
                    }),
                );
                self.fx(Fx::FlowerPop, pos);
                new += 1;
            }
        }
    }

    // ---- production --------------------------------------------------------

    /// Everything the colony does by itself, once a second.
    pub(super) fn step_production(&mut self, world: &mut World, dt: f32) {
        let powered = self.step_power(dt);
        let raining = self.weather.kind == WeatherKind::Rain;
        let daylight = !self.clock.is_night();
        let mut oxygen = 0.0;
        let species = self.species.clone();

        let ids: Vec<Id> = self.ents.keys().copied().collect();
        for id in ids {
            let workers = self.workers_in(id) as f32;
            let has_power = powered.contains(&id);
            let Some(e) = self.ents.get_mut(&id) else { continue };
            match &mut e.kind {
                EntKind::Dome(d) => {
                    let before = d.display_key();
                    let boost = d.boost * (1.0 + 0.5 * workers);
                    let needs_power = d.kind.power_per_day() > 0.0 && d.kind != DomeKind::Starter;
                    d.powered = !needs_power || has_power;
                    let mut issues = 0;
                    if !d.powered {
                        issues |= issue::NO_POWER;
                    }

                    // Water: the reserve covers this dome's own use.
                    let use_per_sec = d.kind.water_per_day() / DAY_SECS;
                    let sprinklers = matches!(d.kind, DomeKind::Green | DomeKind::Dark);
                    let dew: f32 = d
                        .planters
                        .iter()
                        .filter_map(|p| {
                            Some(species.get(p.species? as usize)?.water_per_day() * p.growth.min(1.0))
                        })
                        .sum();
                    d.reserve = (d.reserve + dew / DAY_SECS * dt).min(d.kind.water_per_day().max(1.0));
                    if use_per_sec > 0.0 {
                        let any_planted = d.planters.iter().any(|p| p.species.is_some());
                        let draws = !sprinklers || any_planted;
                        d.watered = d.reserve > 0.0;
                        if d.watered && draws {
                            d.reserve = (d.reserve - use_per_sec * dt).max(0.0);
                        }
                        if !d.watered && !sprinklers {
                            issues |= issue::NO_WATER;
                        }
                    } else {
                        d.watered = true;
                    }

                    // Planters.
                    let bed = d.kind.bed();
                    let pollinated = d.planters.iter().any(|p| {
                        p.growth >= 1.0
                            && p.species
                                .and_then(|s| species.get(s as usize))
                                .is_some_and(|s| s.pollinator)
                    });
                    let mut ripe = Vec::new();
                    let mut dry = false;
                    for (i, p) in d.planters.iter_mut().enumerate() {
                        let Some(sp) = p.species.and_then(|s| species.get(s as usize)) else {
                            continue;
                        };
                        if sprinklers && d.watered
                            || bed == Bed::Water && d.watered
                            || sp.thirst_per_day() == 0.0
                        {
                            p.moisture = 1.0;
                        } else if bed == Bed::Water {
                            p.moisture = 0.0;
                        } else {
                            p.moisture = (p.moisture - sp.thirst_per_day() / DAY_SECS * dt).max(0.0);
                        }
                        let fed = if p.fertilizer > 0.0 { 1.5 } else { 1.0 };
                        p.fertilizer = (p.fertilizer - dt).max(0.0);
                        if p.moisture > 0.0 {
                            let pollen = if pollinated && !sp.pollinator { 1.25 } else { 1.0 };
                            let rate = sp.growth_rate() * sp.fit(bed) * fed * boost * pollen;
                            p.growth = (p.growth + rate * dt).min(1.0);
                        } else {
                            dry = true;
                        }
                        let vigour = if p.moisture > 0.0 { 1.0 } else { 0.3 };
                        oxygen += sp.oxygen_per_day() * p.growth * p.growth * vigour * d.boost;
                        if p.growth >= 1.0 {
                            ripe.push(i);
                        }
                    }
                    if dry {
                        issues |= issue::NO_WATER;
                    }
                    // Workers pick what is ripe into the dome's chest.
                    if workers > 0.0 {
                        for i in ripe {
                            let sp = &species[d.planters[i].species.unwrap() as usize];
                            if sp.product.is_none() {
                                continue;
                            }
                            let items = harvest_items(sp, 1.0, &mut self.rng);
                            let Some(chest) = d.chests.last_mut() else { break };
                            if items.iter().all(|&(item, n)| chest.room_for(item, 1.0) >= n) {
                                for (item, n) in items {
                                    chest.add(item, n, 1.0);
                                }
                                d.planters[i].growth = REGROW;
                            } else {
                                issues |= issue::OUTPUT_FULL;
                            }
                        }
                    }

                    let running = d.powered && d.watered;
                    match d.kind {
                        DomeKind::Kitchen => {
                            let crops = |inv: &super::inventory::Inventory| -> u32 {
                                inv.stacks()
                                    .filter(|s| Ingredient::AnyCrop.accepts(s.item, &species))
                                    .map(|s| s.count)
                                    .sum()
                            };
                            if crops(&d.chests[0]) < 3 {
                                issues |= issue::NO_INPUT;
                                d.progress = 0.0;
                            } else if d.chests[1].room_for(Item::Meal, 1.0) == 0 {
                                issues |= issue::OUTPUT_FULL;
                            } else if running {
                                d.progress += dt / COOK_SECS * boost;
                                if d.progress >= 1.0 {
                                    d.progress = 0.0;
                                    let mut left = 3;
                                    for slot in d.chests[0].slots.iter_mut() {
                                        if let Some(s) = slot
                                            && left > 0
                                            && Ingredient::AnyCrop.accepts(s.item, &species)
                                        {
                                            let n = s.count.min(left);
                                            s.count -= n;
                                            left -= n;
                                            if s.count == 0 {
                                                *slot = None;
                                            }
                                        }
                                    }
                                    d.chests[1].add(Item::Meal, 1, 1.0);
                                    self.stats.food_today += Item::Meal.food(&species);
                                }
                            }
                        }
                        DomeKind::Dark => {
                            let matter =
                                |i: Item| i == Item::Fiber || Ingredient::AnyCrop.accepts(i, &species);
                            let have: u32 = d.chests[0]
                                .stacks()
                                .filter(|s| matter(s.item))
                                .map(|s| s.count)
                                .sum();
                            if have >= 2 && d.chests[1].room_for(Item::Fertilizer, 1.0) > 0 {
                                d.progress += dt / COMPOST_SECS * boost;
                                if d.progress >= 1.0 {
                                    d.progress = 0.0;
                                    let mut left = 2;
                                    for slot in d.chests[0].slots.iter_mut() {
                                        if let Some(s) = slot
                                            && left > 0
                                            && matter(s.item)
                                        {
                                            let n = s.count.min(left);
                                            s.count -= n;
                                            left -= n;
                                            if s.count == 0 {
                                                *slot = None;
                                            }
                                        }
                                    }
                                    d.chests[1].add(Item::Fertilizer, 1, 1.0);
                                }
                            } else {
                                d.progress = 0.0;
                            }
                        }
                        DomeKind::DomeDome => match d.target {
                            None => issues |= issue::NO_TARGET,
                            Some(target) => {
                                if d.chests[0].room_for(Item::DomeKit(target), 1.0) == 0 {
                                    issues |= issue::OUTPUT_FULL;
                                } else if d.powered {
                                    // Bigger domes take longer: one to three days.
                                    let days = match target.size().0 {
                                        0..=80 => 1.0,
                                        81..=110 => 2.0,
                                        _ => 3.0,
                                    };
                                    d.progress += dt / (days * DAY_SECS) * boost;
                                    if d.progress >= 1.0 {
                                        d.progress = 0.0;
                                        d.chests[0].add(Item::DomeKit(target), 1, 1.0);
                                        self.events.push(super::Event::Toast {
                                            to: None,
                                            text: format!("{} finished a {} kit", d.name, target.name()),
                                            good: true,
                                        });
                                    }
                                }
                            }
                        },
                        DomeKind::Aqua => {
                            if d.fish >= 2 && d.watered {
                                // More fish breed faster; the surplus is caught.
                                d.progress += dt / (DAY_SECS * 0.5) * (d.fish as f32 / 4.0) * boost;
                                if d.progress >= 1.0 {
                                    d.progress = 0.0;
                                    if d.fish < 12 {
                                        d.fish += 1;
                                    } else if d.chests[0].add(Item::Fish, 1, 1.0) == 0 {
                                        self.stats.food_today += Item::Fish.food(&species);
                                    } else {
                                        issues |= issue::OUTPUT_FULL;
                                    }
                                }
                            }
                        }
                        DomeKind::Barn => {
                            let feed = d.chests[0]
                                .stacks()
                                .find(|s| Ingredient::AnyCrop.accepts(s.item, &species))
                                .map(|s| s.item);
                            let (trough, out) = d.chests.split_at_mut(1);
                            if !d.animals.is_empty() && feed.is_none() {
                                issues |= issue::NO_INPUT;
                            }
                            for a in d.animals.iter_mut() {
                                a.fed = feed.is_some() && d.watered;
                                if !a.fed {
                                    continue;
                                }
                                a.progress += dt / (DAY_SECS * 0.4) * boost;
                                if a.progress >= 1.0 {
                                    a.progress = 0.0;
                                    let product = match a.kind {
                                        AnimalKind::Cluckbug => Item::Egg,
                                        AnimalKind::MilkGrub => Item::Milk,
                                    };
                                    if let Some(f) = feed {
                                        trough[0].take(f, 1);
                                    }
                                    if out[0].add(product, 2, 1.0) == 0 {
                                        self.stats.food_today += product.food(&species) * 2.0;
                                    } else {
                                        issues |= issue::OUTPUT_FULL;
                                    }
                                    out[0].add(Item::Fertilizer, 1, 1.0);
                                }
                            }
                        }
                        _ => {}
                    }
                    d.issues = issues;
                    if d.display_key() != before {
                        self.dirty.insert(id);
                    }
                }
                EntKind::Machine(m) => {
                    let before = m.display_key();
                    match m.kind {
                        MachineKind::OxygenGenerator => {
                            m.on = has_power;
                            m.issues = if has_power { 0 } else { issue::NO_POWER };
                            if m.on {
                                // Coolant (set by the water step) makes it run faster.
                                let cooled = if m.store > 0.0 { 1.5 } else { 1.0 };
                                oxygen += GENERATOR_O2 * cooled * m.boost;
                            }
                        }
                        MachineKind::Splicer => {
                            m.on = has_power;
                            m.issues = if has_power { 0 } else { issue::NO_POWER };
                        }
                        MachineKind::Pump => {
                            let wet = Colony::submerged(world, m.kind, e.pos) > 0.25;
                            m.on = wet;
                            m.issues = if wet { 0 } else { issue::NOT_SUBMERGED };
                        }
                        MachineKind::Pylon | MachineKind::Lamp | MachineKind::Synthesizer => m.on = true,
                        MachineKind::Chest | MachineKind::Tank => {}
                    }
                    if m.display_key() != before {
                        self.dirty.insert(id);
                    }
                }
                EntKind::Plant(p) => {
                    let Some(sp) = species.get(p.species as usize) else {
                        continue;
                    };
                    let before = (p.growth * 8.0) as u32;
                    let rate = if p.wild {
                        sp.growth_rate() * 0.5
                    } else if daylight {
                        sp.growth_rate() * sp.fit(Bed::Outdoors) * if raining { 1.3 } else { 1.0 }
                    } else {
                        0.0
                    };
                    p.growth = (p.growth + rate * dt).min(1.0);
                    if !p.wild {
                        oxygen += sp.oxygen_per_day() * p.growth * p.growth * 0.5;
                    }
                    if (p.growth * 8.0) as u32 != before {
                        self.dirty.insert(id);
                    }
                }
                _ => {}
            }
        }

        self.atmosphere.rate = oxygen / O2_PER_PERCENT;
        self.atmosphere.o2 = (self.atmosphere.o2 + self.atmosphere.rate / DAY_SECS * dt).min(21.0);
    }

    /// Start-of-day bookkeeping: yesterday's food total, workers' meals.
    pub(super) fn dawn(&mut self, world: &World) {
        self.stats.food_yesterday = self.stats.food_today;
        self.stats.food_today = 0.0;
        self.spawn_dawn_flowers(world);
        self.feed_workers();
    }

    /// Each worker eats 2 food from the stockpile and needs water at home.
    fn feed_workers(&mut self) {
        if self.workers.is_empty() {
            return;
        }
        let species = self.species.clone();
        let mut changed = false;
        for i in 0..self.workers.len() {
            let home = self.workers[i].home;
            let watered = self.dome(home).is_some_and(|(_, d)| d.watered);
            let mut need = 2.0;
            // Eat from Storage Domes and the starter dome.
            let larders: Vec<Id> = self
                .domes()
                .filter(|(_, d)| matches!(d.kind, DomeKind::Storage | DomeKind::Starter | DomeKind::Kitchen))
                .map(|(e, _)| e.id)
                .collect();
            'eat: for dome in larders {
                let Some(d) = self.dome_mut(dome) else { continue };
                for chest in d.chests.iter_mut() {
                    for slot in chest.slots.iter_mut() {
                        if let Some(s) = slot {
                            let each = s.item.food(&species);
                            if each <= 0.0 {
                                continue;
                            }
                            while s.count > 0 && need > 0.0 {
                                s.count -= 1;
                                need -= each;
                            }
                            if s.count == 0 {
                                *slot = None;
                            }
                            if need <= 0.0 {
                                break 'eat;
                            }
                        }
                    }
                }
            }
            let hungry = need > 0.0 || !watered;
            if self.workers[i].hungry != hungry {
                self.workers[i].hungry = hungry;
                changed = true;
            }
        }
        if changed {
            self.meta_rev += 1;
        }
        if self.workers.iter().any(|w| w.hungry) {
            self.toast(
                None,
                "Some workers went without food or water and have stopped working",
                false,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colony::actions::Action;
    use crate::colony::domes::Dome;
    use crate::colony::inventory::Stack;
    use crate::colony::testing::*;

    /// A Green Dome beside the starter dome with the player standing at planter 0.
    fn green(colony: &mut Colony, key: PlayerKey, kind: DomeKind) -> Id {
        let home = colony.ents[&colony.home].pos;
        let pos = v2(home.x + 300.0, home.y);
        let id = colony.spawn(pos, EntKind::Dome(Dome::new(kind, format!("{} 1", kind.name()))));
        let f = kind
            .fixtures()
            .iter()
            .find(|f| matches!(f.fixture, Fixture::Planter(0)));
        let p = colony.players.get_mut(&key).unwrap();
        p.pose.pos = f.map_or(pos, |f| fixture_pos(pos, f));
        p.in_dome = Some(id);
        id
    }

    fn hold(colony: &mut Colony, key: PlayerKey, item: Item, n: u32) {
        let p = colony.players.get_mut(&key).unwrap();
        p.inv.slots[0] = Some(Stack::new(item, n));
        p.selected = 0;
    }

    /// Runs only the once-a-second work, quickly.
    fn tick(colony: &mut Colony, world: &mut World, secs: usize) {
        for _ in 0..secs {
            colony.step_production(world, 1.0);
        }
    }

    #[test]
    fn planting_watering_growing_harvesting() {
        let (mut world, mut colony, key) = colony();
        let dome = green(&mut colony, key, DomeKind::Green);
        let op = |c: &mut Colony, w: &mut World, op| c.apply(w, key, Action::Planter { dome, slot: 0, op });
        assert_eq!(op(&mut colony, &mut world, PlanterOp::Plant), Err("Hold a seed"));
        hold(&mut colony, key, Item::Seed(16), 1);
        assert!(
            op(&mut colony, &mut world, PlanterOp::Plant)
                .unwrap_err()
                .contains("Dark Dome"),
            "sporebloom needs dark"
        );
        hold(&mut colony, key, Item::Seed(VOLTBLOOM), 2);
        op(&mut colony, &mut world, PlanterOp::Plant).unwrap();
        assert_eq!(colony.players[&key].inv.count(Item::Seed(VOLTBLOOM)), 1);
        assert!(colony.codex.plants.contains(&VOLTBLOOM));
        assert_eq!(
            op(&mut colony, &mut world, PlanterOp::Plant),
            Err("Something already grows here")
        );

        // Dry soil: nothing grows.
        tick(&mut colony, &mut world, 60);
        assert_eq!(colony.dome(dome).unwrap().1.planters[0].growth, 0.0);
        assert!(colony.dome(dome).unwrap().1.issues & issue::NO_WATER != 0);
        assert_eq!(
            op(&mut colony, &mut world, PlanterOp::Water),
            Err("You need a Watering Can")
        );
        colony.players.get_mut(&key).unwrap().inv.slots[20] = Some(Stack::new(Item::WateringCan, 1));
        assert!(
            op(&mut colony, &mut world, PlanterOp::Water)
                .unwrap_err()
                .contains("empty")
        );
        colony.players.get_mut(&key).unwrap().can_water = CAN_LITRES;
        op(&mut colony, &mut world, PlanterOp::Water).unwrap();
        assert_eq!(colony.players[&key].can_water, CAN_LITRES - 1.0);

        tick(&mut colony, &mut world, 120);
        let g1 = colony.dome(dome).unwrap().1.planters[0].growth;
        assert!(g1 > 0.1 && g1 < 1.0, "growing: {g1}");
        assert_eq!(
            op(&mut colony, &mut world, PlanterOp::Harvest),
            Err("Not ready yet")
        );
        // Fertilizer speeds it up by half.
        hold(&mut colony, key, Item::Fertilizer, 1);
        op(&mut colony, &mut world, PlanterOp::Fertilize).unwrap();
        tick(&mut colony, &mut world, 120);
        let g2 = colony.dome(dome).unwrap().1.planters[0].growth;
        assert!(
            ((g2 - g1) / g1 - 1.5).abs() < 0.05,
            "fertilized growth {g1} -> {g2}"
        );

        // When the soil dries out, growth stops until it is watered again.
        colony.dome_mut(dome).unwrap().planters[0].moisture = 0.02;
        tick(&mut colony, &mut world, 60);
        let stalled = colony.dome(dome).unwrap().1.planters[0];
        assert_eq!(stalled.moisture, 0.0);
        assert!(stalled.growth < 1.0);
        tick(&mut colony, &mut world, 200);
        assert_eq!(colony.dome(dome).unwrap().1.planters[0].growth, stalled.growth);
        op(&mut colony, &mut world, PlanterOp::Water).unwrap();
        tick(&mut colony, &mut world, 500);
        assert_eq!(colony.dome(dome).unwrap().1.planters[0].growth, 1.0);
        assert!(colony.atmosphere.rate > 0.0, "mature plants release oxygen");
        op(&mut colony, &mut world, PlanterOp::Harvest).unwrap();
        assert_eq!(colony.players[&key].inv.count(Item::Crop(VOLTBLOOM)), 3);
        assert_eq!(
            colony.dome(dome).unwrap().1.planters[0].growth,
            REGROW,
            "it regrows"
        );
        op(&mut colony, &mut world, PlanterOp::Uproot).unwrap();
        assert!(colony.dome(dome).unwrap().1.planters[0].species.is_none());
    }

    #[test]
    fn harvest_yields_follow_the_species() {
        let sp = crate::colony::plants::base_species();
        let mut rng = Rng::new(1);
        let by = |n: &str| sp.iter().find(|s| s.name == n).unwrap();
        let items = harvest_items(by("Ironbark Sapling"), 1.0, &mut rng);
        assert!(items.contains(&(Item::Wood, 8)));
        let fern = harvest_items(by("Breathfern"), 1.0, &mut rng);
        assert_eq!(
            fern,
            vec![(Item::Seed(by("Breathfern").id), 1)],
            "oxygen plants give cuttings"
        );
        let reed = by("Sugar Reed");
        let many: u32 = (0..100)
            .map(|_| {
                harvest_items(reed, 1.0, &mut rng)
                    .iter()
                    .filter(|(i, _)| matches!(i, Item::Seed(_)))
                    .count() as u32
            })
            .sum();
        assert!(
            (15..60).contains(&many),
            "seeds drop about a third of the time: {many}"
        );
        // A hybrid with two products gives both.
        let hybrid = (0..sp.len())
            .flat_map(|i| (i + 1..sp.len()).map(move |j| (i, j)))
            .map(|(i, j)| crate::colony::plants::splice(3, &sp[i], &sp[j], 0, &sp))
            .find(|h| {
                h.second.is_some() && h.product != Some(Product::Water) && h.second != Some(Product::Water)
            })
            .unwrap();
        let mut h = hybrid.clone();
        h.id = 30;
        let got = harvest_items(&h, 1.0, &mut rng);
        assert!(
            got.iter()
                .filter(|(i, _)| !matches!(i, Item::Seed(_)))
                .map(|(_, n)| n)
                .sum::<u32>()
                >= 2
        );
    }

    #[test]
    fn sprinklers_and_workers_automate_a_green_dome() {
        let (mut world, mut colony, key) = colony();
        let dome = green(&mut colony, key, DomeKind::Green);
        hold(&mut colony, key, Item::Seed(2), 4);
        for slot in 0..4 {
            colony
                .apply(
                    &mut world,
                    key,
                    Action::Planter {
                        dome,
                        slot,
                        op: PlanterOp::Plant,
                    },
                )
                .unwrap_or(());
            // Planters further away are out of reach; plant them directly.
            let pl = &mut colony.dome_mut(dome).unwrap().planters[slot as usize];
            pl.species = Some(2);
        }
        // Water in the dome's reserve runs the sprinklers.
        colony.dome_mut(dome).unwrap().reserve = 15.0;
        tick(&mut colony, &mut world, 10);
        let d = colony.dome(dome).unwrap().1;
        assert!(d.planters.iter().all(|p| p.moisture == 1.0));
        assert!(d.reserve < 15.0, "sprinklers use water");
        // A worker picks ripe crops into the chest and the plants regrow.
        let w = colony.next_id;
        colony.workers.push(crate::colony::economy::Worker {
            id: w,
            name: "Imani".into(),
            home: colony.home,
            job: Some(dome),
            hungry: false,
        });
        colony.dome_mut(dome).unwrap().reserve = 30.0;
        for pl in colony.dome_mut(dome).unwrap().planters.iter_mut() {
            pl.growth = 0.99;
        }
        tick(&mut colony, &mut world, 30);
        let d = colony.dome(dome).unwrap().1;
        assert!(d.chests[0].count(Item::Crop(2)) >= 12, "picked into the chest");
        assert!(d.planters.iter().all(|p| p.growth < 1.0));
        // Workers also make it grow faster.
        let with = d.planters[0].growth;
        colony.workers.clear();
        colony.dome_mut(dome).unwrap().planters[0].growth = REGROW;
        tick(&mut colony, &mut world, 30);
        let without = colony.dome(dome).unwrap().1.planters[0].growth;
        assert!(with - REGROW > (without - REGROW) * 1.2, "{with} vs {without}");
    }

    #[test]
    fn kitchen_cooks_when_powered_and_watered() {
        let (mut world, mut colony, key) = colony();
        let dome = green(&mut colony, key, DomeKind::Kitchen);
        colony.dome_mut(dome).unwrap().chests[0].add(Item::Crop(2), 7, 1.0);
        tick(&mut colony, &mut world, 60);
        let d = colony.dome(dome).unwrap().1;
        assert!(d.issues & issue::NO_POWER != 0 && d.issues & issue::NO_WATER != 0);
        assert_eq!(d.chests[1].count(Item::Meal), 0);
        // A pylon with flowers nearby, and water by hand.
        let pos = colony.ents[&dome].pos;
        let mut pylon = crate::colony::Machine::new(MachineKind::Pylon, "Pylon".into());
        pylon.inv.add(Item::Crop(VOLTBLOOM), 2, 1.0);
        colony.spawn(v2(pos.x + 50.0, pos.y), EntKind::Machine(pylon));
        let p = colony.players.get_mut(&key).unwrap();
        p.inv.slots[0] = Some(Stack::new(Item::WateringCan, 1));
        p.selected = 0;
        p.can_water = CAN_LITRES;
        colony
            .apply(
                &mut world,
                key,
                Action::Use {
                    at: v2(pos.x, pos.y - 10.0),
                },
            )
            .unwrap();
        assert!(colony.dome(dome).unwrap().1.reserve > 0.0);
        tick(&mut colony, &mut world, 100);
        let d = colony.dome(dome).unwrap().1;
        assert_eq!(d.issues & (issue::NO_POWER | issue::NO_WATER), 0);
        assert_eq!(d.chests[1].count(Item::Meal), 2);
        assert_eq!(d.chests[0].count(Item::Crop(2)), 1);
        assert_eq!(colony.stats.food_today, 24.0);
        // The crate runs out.
        tick(&mut colony, &mut world, 10);
        assert!(colony.dome(dome).unwrap().1.issues & issue::NO_INPUT != 0);
    }

    #[test]
    fn barn_aqua_compost_and_dome_dome() {
        let (mut world, mut colony, key) = colony();
        // Barn: a fed, watered cluckbug lays eggs.
        let barn = green(&mut colony, key, DomeKind::Barn);
        hold(&mut colony, key, Item::Cluckbug, 1);
        colony.apply(&mut world, key, Action::Stock).unwrap();
        assert_eq!(colony.dome(barn).unwrap().1.animals.len(), 1);
        tick(&mut colony, &mut world, 400);
        assert_eq!(
            colony.dome(barn).unwrap().1.chests[1].count(Item::Egg),
            0,
            "unfed"
        );
        let d = colony.dome_mut(barn).unwrap();
        d.chests[0].add(Item::Crop(2), 5, 1.0);
        d.reserve = 20.0;
        tick(&mut colony, &mut world, 400);
        let d = colony.dome(barn).unwrap().1;
        assert_eq!(d.chests[1].count(Item::Egg), 2);
        assert_eq!(d.chests[1].count(Item::Fertilizer), 1);
        assert_eq!(d.chests[0].count(Item::Crop(2)), 4);
        colony.despawn(barn);

        // Aqua: fish breed to twelve, then the surplus is caught.
        let aqua = green(&mut colony, key, DomeKind::Aqua);
        hold(&mut colony, key, Item::FishFry, 5);
        for _ in 0..5 {
            colony.apply(&mut world, key, Action::Stock).unwrap();
        }
        colony.dome_mut(aqua).unwrap().reserve = 60.0;
        for _ in 0..40 {
            colony.dome_mut(aqua).unwrap().reserve = 60.0;
            tick(&mut colony, &mut world, 200);
        }
        let d = colony.dome(aqua).unwrap().1;
        assert_eq!(d.fish, 12);
        assert!(d.chests[0].count(Item::Fish) > 5);
        colony.despawn(aqua);

        // Dark Dome: the vat composts plant matter.
        let dark = green(&mut colony, key, DomeKind::Dark);
        colony.dome_mut(dark).unwrap().chests[0].add(Item::Fiber, 6, 1.0);
        tick(&mut colony, &mut world, 100);
        assert_eq!(colony.dome(dark).unwrap().1.chests[1].count(Item::Fertilizer), 3);
        colony.despawn(dark);

        // Dome Dome: builds the chosen kit when powered.
        let dd = green(&mut colony, key, DomeKind::DomeDome);
        tick(&mut colony, &mut world, 5);
        assert!(colony.dome(dd).unwrap().1.issues & issue::NO_TARGET != 0);
        colony
            .apply(
                &mut world,
                key,
                Action::SetTarget {
                    dome: dd,
                    target: Some(DomeKind::Storage),
                },
            )
            .unwrap();
        let pos = colony.ents[&dd].pos;
        let mut pylon = crate::colony::Machine::new(MachineKind::Pylon, "Pylon".into());
        pylon.inv.add(Item::Crop(VOLTBLOOM), 5, 1.0);
        colony.spawn(v2(pos.x + 60.0, pos.y), EntKind::Machine(pylon));
        tick(&mut colony, &mut world, DAY_SECS as usize + 5);
        assert_eq!(
            colony.dome(dd).unwrap().1.chests[0].count(Item::DomeKit(DomeKind::Storage)),
            1
        );
    }

    #[test]
    fn battery_flowers_sprout_at_dawn_and_wild_plants_give_seeds() {
        let (mut world, mut colony, key) = colony();
        let flowers = |c: &Colony| {
            c.ents
                .values()
                .filter(|e| matches!(e.kind, EntKind::Plant(p) if p.wild && p.species == VOLTBLOOM))
                .count()
        };
        let start = flowers(&colony);
        assert!(start >= 10, "flowers on day one: {start}");
        assert!(
            colony
                .ents
                .values()
                .any(|e| matches!(e.kind, EntKind::Plant(p) if p.species != VOLTBLOOM))
        );
        // Pick one.
        let (id, pos) = colony
            .ents
            .values()
            .find(|e| matches!(e.kind, EntKind::Plant(p) if p.wild && p.species == VOLTBLOOM))
            .map(|e| (e.id, e.pos))
            .unwrap();
        let p = colony.players.get_mut(&key).unwrap();
        p.pose.pos = pos;
        p.in_dome = None;
        colony
            .apply(&mut world, key, Action::Harvest { plant: id })
            .unwrap();
        assert!(colony.players[&key].inv.count(Item::Crop(VOLTBLOOM)) >= 3);
        assert!(
            colony.players[&key].inv.count(Item::Seed(VOLTBLOOM)) >= 1,
            "first find gives a seed"
        );
        assert_eq!(flowers(&colony), start - 1);
        // Next dawn they are back.
        colony.clock.time = DAY_SECS - 0.5;
        run(&mut colony, &mut world, 1.0);
        assert_eq!(flowers(&colony), start);
        // Seeds can be planted outside, but tender ones refuse.
        let x = colony.profile.spawn_x + 200;
        let ground = colony.profile.surface_at(x) as f32;
        colony.players.get_mut(&key).unwrap().pose.pos = v2(x as f32, ground);
        hold(&mut colony, key, Item::Seed(12), 1);
        let planted = colony.apply(
            &mut world,
            key,
            Action::Use {
                at: v2(x as f32 + 20.0, ground - 6.0),
            },
        );
        if planted.is_ok() {
            assert!(
                colony
                    .ents
                    .values()
                    .any(|e| matches!(e.kind, EntKind::Plant(p) if !p.wild))
            );
        }
        hold(&mut colony, key, Item::Seed(18), 1);
        assert_eq!(
            colony.apply(
                &mut world,
                key,
                Action::Use {
                    at: v2(x as f32 + 60.0, ground - 6.0)
                }
            ),
            Err("That won't survive outside")
        );
    }

    #[test]
    fn hungry_workers_stop_working() {
        let (mut world, mut colony, _) = colony();
        let home = colony.home;
        colony.workers.push(crate::colony::economy::Worker {
            id: 999,
            name: "Soren".into(),
            home,
            job: None,
            hungry: false,
        });
        // The starter chest holds 8 meals: two days for one worker... and none after.
        colony.dawn(&world);
        assert!(!colony.workers[0].hungry);
        let meals = colony.dome(home).unwrap().1.chests[1].count(Item::Meal);
        assert_eq!(meals, 7, "a 12-point meal covers the day's 2 food");
        colony.dome_mut(home).unwrap().chests[1].take(Item::Meal, 99);
        colony.dawn(&world);
        assert!(colony.workers[0].hungry);
        assert_eq!(colony.workers_in(home), 0);
        let _ = &mut world;
    }
}
