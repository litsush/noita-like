//! Credits and everything they move: synthesizing from the colony's
//! stockpile, selling to Earth, and buying seeds, animals, workers and
//! blueprints, which arrive by drop pod.

use serde::{Deserialize, Serialize};

use super::actions::Container;
use super::domes::Fixture;
use super::geom::{V2, v2};
use super::inventory::Inventory;
use super::items::{DomeKind, Item, MachineKind};
use super::plants::{self, SpeciesId};
use super::recipes::{Blueprint, Ingredient, recipes};
use super::{Colony, Crate, CrateKind, EntKind, Fx, Id, PlayerKey};
use crate::world::World;

/// Something Earth sells.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Offer {
    /// Three seeds of a base species.
    SeedPack(SpeciesId),
    Cluckbug,
    MilkGrub,
    /// Five fish fry.
    FishFry,
    Worker,
    Blueprint(Blueprint),
}

impl Offer {
    pub fn price(self) -> u32 {
        match self {
            Offer::SeedPack(s) => plants::seed_pack_price(s),
            Offer::Cluckbug => 300,
            Offer::MilkGrub => 500,
            Offer::FishFry => 250,
            Offer::Worker => 800,
            Offer::Blueprint(b) => b.price(),
        }
    }

    /// What arrives in the pod.
    fn goods(self) -> Option<(Item, u32)> {
        match self {
            Offer::SeedPack(s) => Some((Item::Seed(s), 3)),
            Offer::Cluckbug => Some((Item::Cluckbug, 1)),
            Offer::MilkGrub => Some((Item::MilkGrub, 1)),
            Offer::FishFry => Some((Item::FishFry, 5)),
            Offer::Worker | Offer::Blueprint(_) => None,
        }
    }
}

/// Goods on their way from Earth.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Delivery {
    pub secs: f32,
    pub item: Item,
    pub count: u32,
    pub buyer: PlayerKey,
}

/// A human worker living in the colony.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Worker {
    pub id: Id,
    pub name: String,
    /// The dome with their bed.
    pub home: Id,
    /// The dome they work in.
    pub job: Option<Id>,
    /// Went without food or water today.
    pub hungry: bool,
}

const WORKER_NAMES: [&str; 16] = [
    "Imani", "Soren", "Yuki", "Mateo", "Priya", "Tomas", "Nia", "Ezra", "Lucia", "Kofi", "Hana", "Otto",
    "Zara", "Milan", "Aiko", "Rafa",
];

pub const DELIVERY_SECS: f32 = 20.0;

impl Colony {
    /// Whether a player may use fixture `f` right now: they must be inside a
    /// dome that has it, close to it.
    fn at_fixture(&self, key: PlayerKey, f: Fixture) -> bool {
        let p = &self.players[&key];
        let Some((e, d)) = p.in_dome.and_then(|id| self.dome(id)) else {
            return false;
        };
        d.kind
            .fixtures()
            .iter()
            .filter(|pl| pl.fixture == f)
            .any(|pl| super::domes::fixture_pos(e.pos, pl).distance(p.pose.pos) <= p.stats().reach + 12.0)
    }

    /// A synthesizer the player can reach: the starter dome's, or a placed one.
    pub fn synthesizer_near(&self, key: PlayerKey) -> bool {
        if self.at_fixture(key, Fixture::Synthesizer) {
            return true;
        }
        let p = &self.players[&key];
        self.ents.values().any(|e| {
            matches!(&e.kind, EntKind::Machine(m) if m.kind == MachineKind::Synthesizer)
                && e.pos.distance(p.pose.pos) <= p.stats().reach + 12.0
        })
    }

    pub fn terminal_near(&self, key: PlayerKey) -> bool {
        self.at_fixture(key, Fixture::Terminal)
    }

    /// Everything a player's synthesizing can draw on: their inventory, the
    /// chests of the dome they are in, and every Storage Dome.
    pub fn stock_containers(&self, key: PlayerKey) -> Vec<Container> {
        let mut out = vec![Container::Me];
        let here = self.players[&key].in_dome;
        for (e, d) in self.domes() {
            if Some(e.id) == here || d.kind == DomeKind::Storage {
                out.extend((0..d.chests.len() as u8).map(|i| Container::DomeChest(e.id, i)));
            }
        }
        out
    }

    fn container(&self, key: PlayerKey, c: Container) -> Option<&Inventory> {
        match c {
            Container::Me => self.players.get(&key).map(|p| &p.inv),
            Container::DomeChest(id, i) => self.dome(id)?.1.chests.get(i as usize),
            Container::Machine(id) | Container::Crate(id) => match &self.ents.get(&id)?.kind {
                EntKind::Machine(m) => Some(&m.inv),
                EntKind::Crate(c) => Some(&c.inv),
                _ => None,
            },
        }
    }

    /// How much of an ingredient the player's stockpile holds.
    pub fn stock_count(&self, key: PlayerKey, ing: Ingredient) -> u32 {
        self.stock_containers(key)
            .into_iter()
            .filter_map(|c| self.container(key, c))
            .flat_map(|inv| inv.stacks())
            .filter(|s| ing.accepts(s.item, &self.species))
            .map(|s| s.count)
            .sum()
    }

    /// Removes `count` of an ingredient from the stockpile (the player's own
    /// inventory first). Call only after checking [`Colony::stock_count`].
    fn stock_take(&mut self, key: PlayerKey, ing: Ingredient, count: u32) {
        let mut left = count;
        let species = self.species.clone();
        for c in self.stock_containers(key) {
            if left == 0 {
                break;
            }
            let Some(inv) = self.container_mut_for(key, c) else {
                continue;
            };
            for slot in inv.slots.iter_mut() {
                if let Some(s) = slot
                    && ing.accepts(s.item, &species)
                {
                    let n = s.count.min(left);
                    s.count -= n;
                    left -= n;
                    if s.count == 0 {
                        *slot = None;
                    }
                    if left == 0 {
                        break;
                    }
                }
            }
        }
    }

    /// Gives items to a player, dropping at their feet what doesn't fit.
    pub fn give(&mut self, key: PlayerKey, item: Item, count: u32) {
        let Some(p) = self.players.get_mut(&key) else {
            return;
        };
        let mult = p.stats().stack_mult;
        let left = p.inv.add(item, count, mult);
        p.touch();
        let at = p.center();
        if left > 0 {
            self.drop_item(at, item, left);
        }
    }

    pub(super) fn craft(&mut self, key: PlayerKey, recipe: u16, count: u16) -> Result<(), &'static str> {
        let r = recipes().get(recipe as usize).ok_or("Unknown recipe")?;
        if r.blueprint.is_some_and(|b| !self.blueprints.contains(&b)) {
            return Err("Buy the blueprint from Earth first");
        }
        if !self.synthesizer_near(key) {
            return Err("Use a Synthesizer");
        }
        let n = count.clamp(1, 99) as u32;
        if r.inputs
            .iter()
            .any(|&(ing, need)| self.stock_count(key, ing) < need * n)
        {
            return Err("Not enough materials");
        }
        for &(ing, need) in &r.inputs {
            self.stock_take(key, ing, need * n);
        }
        self.give(key, r.output, r.count * n);
        let at = self.players[&key].center();
        self.fx(Fx::Craft, at);
        Ok(())
    }

    pub(super) fn sell(&mut self, key: PlayerKey, item: Item, count: u32) -> Result<(), &'static str> {
        if !self.terminal_near(key) {
            return Err("Use the Comm. Terminal");
        }
        let value = item.value(&self.species);
        if value == 0 {
            return Err("Earth doesn't want that");
        }
        let p = self.players.get_mut(&key).unwrap();
        let sold = p.inv.take(item, count);
        if sold == 0 {
            return Err("You don't have any");
        }
        p.touch();
        let at = p.center();
        self.credits += (value * sold) as i64;
        self.stats.earned += (value * sold) as u64;
        self.fx(Fx::Sell, at);
        Ok(())
    }

    /// Free worker beds: (dome, beds free).
    pub fn free_worker_beds(&self) -> Vec<(Id, u32)> {
        self.domes()
            .filter_map(|(e, d)| {
                let used = self.workers.iter().filter(|w| w.home == e.id).count() as u32;
                (d.kind.worker_beds() > used).then(|| (e.id, d.kind.worker_beds() - used))
            })
            .collect()
    }

    pub(super) fn buy(&mut self, key: PlayerKey, offer: Offer) -> Result<(), &'static str> {
        if !self.terminal_near(key) {
            return Err("Use the Comm. Terminal");
        }
        let price = offer.price() as i64;
        if price == 0 {
            return Err("Earth doesn't sell that");
        }
        match offer {
            Offer::Blueprint(b) if self.blueprints.contains(&b) => {
                return Err("The colony already has that blueprint");
            }
            Offer::SeedPack(s) if s as usize >= plants::BASE_SPECIES => {
                return Err("Earth doesn't have that seed");
            }
            Offer::Worker if self.free_worker_beds().is_empty() => {
                return Err("No free bed: build a Bedroom or Apartment Dome first");
            }
            _ => {}
        }
        if self.credits < price {
            return Err("Not enough credits");
        }
        self.credits -= price;
        let at = self.players[&key].center();
        self.fx(Fx::Buy, at);
        match offer {
            Offer::Blueprint(b) => {
                self.blueprints.insert(b);
                self.meta_rev += 1;
                self.fx(Fx::Unlock, at);
                self.toast(None, format!("New blueprint: {}", b.name()), true);
            }
            Offer::Worker => {
                let (home, _) = self.free_worker_beds()[0];
                let id = self.next_id;
                self.next_id += 1;
                let name = WORKER_NAMES[(id as usize + self.workers.len()) % WORKER_NAMES.len()].to_string();
                self.toast(
                    None,
                    format!("{name} arrived from Earth. Assign them to a dome."),
                    true,
                );
                self.workers.push(Worker {
                    id,
                    name,
                    home,
                    job: None,
                    hungry: false,
                });
                self.meta_rev += 1;
            }
            _ => {
                let (item, count) = offer.goods().unwrap();
                self.deliveries.push(Delivery {
                    secs: DELIVERY_SECS,
                    item,
                    count,
                    buyer: key,
                });
                self.toast(Some(key), "Ordered. A drop pod is on its way.", true);
            }
        }
        Ok(())
    }

    pub(super) fn assign_worker(&mut self, worker: Id, job: Option<Id>) -> Result<(), &'static str> {
        if let Some(dome) = job {
            let (_, d) = self.dome(dome).ok_or("That dome is gone")?;
            if !d.kind.employs() {
                return Err("Workers can't help in that dome");
            }
            if self
                .workers
                .iter()
                .filter(|w| w.job == Some(dome) && w.id != worker)
                .count()
                >= 2
            {
                return Err("Two workers per dome at most");
            }
        }
        let w = self
            .workers
            .iter_mut()
            .find(|w| w.id == worker)
            .ok_or("Unknown worker")?;
        w.job = job;
        self.meta_rev += 1;
        Ok(())
    }

    /// Workers assigned to a dome who are fed.
    pub fn workers_in(&self, dome: Id) -> usize {
        self.workers
            .iter()
            .filter(|w| w.job == Some(dome) && !w.hungry)
            .count()
    }

    /// Lands pods whose time has come, beside the starter dome.
    pub(super) fn step_deliveries(&mut self, world: &World, dt: f32) {
        if self.deliveries.is_empty() {
            return;
        }
        for d in &mut self.deliveries {
            d.secs -= dt;
        }
        let due: Vec<Delivery> = self
            .deliveries
            .iter()
            .filter(|d| d.secs <= 0.0)
            .cloned()
            .collect();
        self.deliveries.retain(|d| d.secs > 0.0);
        if due.is_empty() {
            return;
        }
        let at = self.pod_site(world);
        // Everything landing together shares a pod.
        let existing = self.ents.values().find_map(|e| match &e.kind {
            EntKind::Crate(c) if c.kind == CrateKind::Pod && e.pos.distance(at) < 6.0 => Some(e.id),
            _ => None,
        });
        let id = existing.unwrap_or_else(|| {
            self.spawn(
                at,
                EntKind::Crate(Crate {
                    kind: CrateKind::Pod,
                    inv: Inventory::new(16),
                    owner: None,
                }),
            )
        });
        for d in due {
            if let Some(EntKind::Crate(c)) = self.ents.get_mut(&id).map(|e| &mut e.kind) {
                c.inv.add(d.item, d.count, 10.0);
            }
            self.toast(
                Some(d.buyer),
                "Your order has landed beside the starter dome.",
                true,
            );
        }
        self.touch(id);
        self.fx(Fx::PodLand, at);
    }

    /// The ground just outside the starter dome's right-hand airlock.
    fn pod_site(&self, world: &World) -> V2 {
        let home = self.ents.get(&self.home).map_or(V2::ZERO, |e| e.pos);
        let x = home.x + DomeKind::Starter.size().0 as f32 / 2.0 + 26.0;
        let mut y = home.y - 30.0;
        for _ in 0..120 {
            if world.material(x as i32, y as i32).is_solid_for_player() {
                break;
            }
            y += 1.0;
        }
        v2(x, y)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colony::actions::Action;
    use crate::colony::inventory::Stack;
    use crate::colony::testing::*;

    fn at_fixture(colony: &mut Colony, key: PlayerKey, f: Fixture) {
        let home = colony.home;
        let e = &colony.ents[&home];
        let pl = DomeKind::Starter
            .fixtures()
            .iter()
            .find(|p| p.fixture == f)
            .unwrap();
        let pos = super::super::domes::fixture_pos(e.pos, pl);
        let p = colony.players.get_mut(&key).unwrap();
        p.pose.pos = pos;
        p.in_dome = Some(home);
    }

    fn recipe_for(item: Item) -> u16 {
        recipes().iter().position(|r| r.output == item).unwrap() as u16
    }

    #[test]
    fn crafting_draws_on_inventory_and_dome_chests() {
        let (mut world, mut colony, key) = colony();
        let home = colony.home;
        let glass = recipe_for(Item::Glass);
        colony.players.get_mut(&key).unwrap().inv.slots[20] = Some(Stack::new(Item::Sand, 4));
        // Not at a synthesizer.
        at_fixture(&mut colony, key, Fixture::SuitRack);
        assert_eq!(
            colony.apply(
                &mut world,
                key,
                Action::Craft {
                    recipe: glass,
                    count: 1
                }
            ),
            Err("Use a Synthesizer")
        );
        at_fixture(&mut colony, key, Fixture::Synthesizer);
        colony
            .apply(
                &mut world,
                key,
                Action::Craft {
                    recipe: glass,
                    count: 1,
                },
            )
            .unwrap();
        assert_eq!(colony.players[&key].inv.count(Item::Glass), 2);
        assert_eq!(colony.players[&key].inv.count(Item::Sand), 1);
        // Two more batches need 6 sand: 1 in the bag, the rest from the dome's chest.
        assert_eq!(
            colony.apply(
                &mut world,
                key,
                Action::Craft {
                    recipe: glass,
                    count: 2
                }
            ),
            Err("Not enough materials")
        );
        colony.dome_mut(home).unwrap().chests[1].add(Item::Sand, 10, 1.0);
        assert_eq!(colony.stock_count(key, Ingredient::Item(Item::Sand)), 11);
        colony
            .apply(
                &mut world,
                key,
                Action::Craft {
                    recipe: glass,
                    count: 2,
                },
            )
            .unwrap();
        assert_eq!(colony.players[&key].inv.count(Item::Glass), 6);
        assert_eq!(
            colony.players[&key].inv.count(Item::Sand),
            0,
            "own inventory is used first"
        );
        assert_eq!(colony.dome(home).unwrap().1.chests[1].count(Item::Sand), 5);
    }

    #[test]
    fn blueprints_gate_recipes_and_are_bought_with_credits() {
        let (mut world, mut colony, key) = colony();
        let pipe = recipe_for(Item::Pipe);
        colony.players.get_mut(&key).unwrap().inv.slots[20] = Some(Stack::new(Item::Copper, 8));
        at_fixture(&mut colony, key, Fixture::Synthesizer);
        assert_eq!(
            colony.apply(
                &mut world,
                key,
                Action::Craft {
                    recipe: pipe,
                    count: 1
                }
            ),
            Err("Buy the blueprint from Earth first")
        );
        at_fixture(&mut colony, key, Fixture::Terminal);
        let offer = Offer::Blueprint(Blueprint::Plumbing);
        assert_eq!(
            colony.apply(&mut world, key, Action::Buy(offer)),
            Err("Not enough credits")
        );
        // Sell ore to afford it.
        colony.players.get_mut(&key).unwrap().inv.slots[21] = Some(Stack::new(Item::Gold, 10));
        colony
            .apply(
                &mut world,
                key,
                Action::Sell {
                    item: Item::Gold,
                    count: 5,
                },
            )
            .unwrap();
        assert_eq!(colony.credits, 150);
        assert_eq!(colony.players[&key].inv.count(Item::Gold), 5);
        assert_eq!(
            colony.apply(
                &mut world,
                key,
                Action::Sell {
                    item: Item::Stone,
                    count: 1
                }
            ),
            Err("Earth doesn't want that")
        );
        let rev = colony.meta_rev;
        colony.apply(&mut world, key, Action::Buy(offer)).unwrap();
        assert_eq!(colony.credits, 0);
        assert!(colony.meta_rev > rev);
        assert_eq!(
            colony.apply(&mut world, key, Action::Buy(offer)),
            Err("The colony already has that blueprint")
        );
        at_fixture(&mut colony, key, Fixture::Synthesizer);
        colony
            .apply(
                &mut world,
                key,
                Action::Craft {
                    recipe: pipe,
                    count: 2,
                },
            )
            .unwrap();
        assert_eq!(colony.players[&key].inv.count(Item::Pipe), 16);
    }

    #[test]
    fn goods_arrive_by_drop_pod() {
        let (mut world, mut colony, key) = colony();
        colony.credits = 1000;
        at_fixture(&mut colony, key, Fixture::Terminal);
        colony
            .apply(&mut world, key, Action::Buy(Offer::SeedPack(3)))
            .unwrap();
        colony
            .apply(&mut world, key, Action::Buy(Offer::Cluckbug))
            .unwrap();
        assert_eq!(colony.credits, 1000 - 40 - 300);
        assert_eq!(colony.deliveries.len(), 2);
        run(&mut colony, &mut world, DELIVERY_SECS + 1.5);
        let pods: Vec<_> = colony
            .ents
            .values()
            .filter_map(|e| match &e.kind {
                EntKind::Crate(c) if c.kind == CrateKind::Pod => Some((e.pos, c)),
                _ => None,
            })
            .collect();
        assert_eq!(pods.len(), 1, "one pod for both orders");
        let (pos, pod) = pods[0];
        assert_eq!(pod.inv.count(Item::Seed(3)), 3);
        assert_eq!(pod.inv.count(Item::Cluckbug), 1);
        assert!(colony.dome_at(pos).is_none(), "lands outside");
        assert!(world.is_solid(pos.x as i32, pos.y as i32 + 1) || world.is_solid(pos.x as i32, pos.y as i32));
    }

    #[test]
    fn workers_need_beds_and_take_jobs() {
        let (mut world, mut colony, key) = colony();
        colony.credits = 5000;
        at_fixture(&mut colony, key, Fixture::Terminal);
        assert_eq!(
            colony.apply(&mut world, key, Action::Buy(Offer::Worker)),
            Err("No free bed: build a Bedroom or Apartment Dome first")
        );
        let home_pos = colony.ents[&colony.home].pos;
        let bedroom = colony.spawn(
            v2(home_pos.x + 300.0, home_pos.y),
            EntKind::Dome(super::super::domes::Dome::new(
                DomeKind::Bedroom,
                "Bedroom 1".into(),
            )),
        );
        let green = colony.spawn(
            v2(home_pos.x + 500.0, home_pos.y),
            EntKind::Dome(super::super::domes::Dome::new(DomeKind::Green, "Green 1".into())),
        );
        for _ in 0..4 {
            colony.apply(&mut world, key, Action::Buy(Offer::Worker)).unwrap();
        }
        assert!(
            colony.apply(&mut world, key, Action::Buy(Offer::Worker)).is_err(),
            "four beds"
        );
        let ids: Vec<Id> = colony.workers.iter().map(|w| w.id).collect();
        assert!(colony.workers.iter().all(|w| w.home == bedroom));
        colony
            .apply(
                &mut world,
                key,
                Action::AssignWorker {
                    worker: ids[0],
                    job: Some(green),
                },
            )
            .unwrap();
        colony
            .apply(
                &mut world,
                key,
                Action::AssignWorker {
                    worker: ids[1],
                    job: Some(green),
                },
            )
            .unwrap();
        assert_eq!(
            colony.apply(
                &mut world,
                key,
                Action::AssignWorker {
                    worker: ids[2],
                    job: Some(green)
                }
            ),
            Err("Two workers per dome at most")
        );
        assert_eq!(
            colony.apply(
                &mut world,
                key,
                Action::AssignWorker {
                    worker: ids[2],
                    job: Some(bedroom)
                }
            ),
            Err("Workers can't help in that dome")
        );
        assert_eq!(colony.workers_in(green), 2);
        colony
            .apply(
                &mut world,
                key,
                Action::AssignWorker {
                    worker: ids[0],
                    job: None,
                },
            )
            .unwrap();
        assert_eq!(colony.workers_in(green), 1);
    }
}
