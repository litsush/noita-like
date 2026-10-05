//! Power. Charging Pylons hold battery flowers; pylons close to each other
//! link into a grid; every dome and machine near a pylon draws on the
//! grid's stored charge.

use std::collections::BTreeSet;

use super::domes::footprint;
use super::geom::V2;
use super::items::{DomeKind, Item, MachineKind};
use super::{Colony, DAY_SECS, EntKind, Id};

/// Pylons this close to each other share charge.
pub const LINK_RANGE: f32 = 60.0;
/// A pylon powers things within this many cells.
pub const SUPPLY_RANGE: f32 = 48.0;
/// Charge a pylon holds beyond the flowers in its rack.
pub const PYLON_BUFFER: f32 = 100.0;

/// One group of linked pylons and what it feeds, for the overlay.
#[derive(Clone, Debug, PartialEq)]
pub struct Grid {
    pub pylons: Vec<(Id, V2)>,
    pub consumers: Vec<Id>,
    /// Charge in the pylons' buffers plus their unspent flowers.
    pub stored: f32,
    pub drain_per_day: f32,
}

impl MachineKind {
    /// Charge used per day while running.
    pub fn power_per_day(self) -> f32 {
        match self {
            MachineKind::OxygenGenerator => 50.0,
            MachineKind::Splicer => 5.0,
            _ => 0.0,
        }
    }
}

impl Colony {
    /// Charge per day an entity draws, if it uses power at all.
    fn power_need(&self, id: Id) -> f32 {
        match self.ents.get(&id).map(|e| &e.kind) {
            Some(EntKind::Dome(d)) if d.kind != DomeKind::Starter => d.kind.power_per_day(),
            Some(EntKind::Machine(m)) => m.kind.power_per_day(),
            _ => 0.0,
        }
    }

    /// The pylon grids as they stand.
    pub fn grids(&self) -> Vec<Grid> {
        let pylons: Vec<(Id, V2)> = self
            .ents
            .values()
            .filter(|e| matches!(&e.kind, EntKind::Machine(m) if m.kind == MachineKind::Pylon))
            .map(|e| (e.id, e.pos))
            .collect();
        // Group pylons that are within link range of each other.
        let mut group: Vec<usize> = (0..pylons.len()).collect();
        fn find(g: &mut [usize], i: usize) -> usize {
            let mut r = i;
            while g[r] != r {
                r = g[r];
            }
            g[i] = r;
            r
        }
        for i in 0..pylons.len() {
            for j in i + 1..pylons.len() {
                if pylons[i].1.distance(pylons[j].1) <= LINK_RANGE {
                    let (a, b) = (find(&mut group, i), find(&mut group, j));
                    group[a] = b;
                }
            }
        }
        let mut grids: Vec<(usize, Grid)> = Vec::new();
        for (i, &pylon) in pylons.iter().enumerate() {
            let root = find(&mut group, i);
            let EntKind::Machine(m) = &self.ents[&pylon.0].kind else {
                continue;
            };
            let flowers: f32 = m
                .inv
                .stacks()
                .map(|s| s.item.charge(&self.species) * s.count as f32)
                .sum();
            let entry = match grids.iter_mut().find(|(r, _)| *r == root) {
                Some((_, g)) => g,
                None => {
                    grids.push((
                        root,
                        Grid {
                            pylons: Vec::new(),
                            consumers: Vec::new(),
                            stored: 0.0,
                            drain_per_day: 0.0,
                        },
                    ));
                    &mut grids.last_mut().unwrap().1
                }
            };
            entry.pylons.push(pylon);
            entry.stored += m.store + flowers;
        }
        let mut grids: Vec<Grid> = grids.into_iter().map(|(_, g)| g).collect();
        // Each consumer joins the first grid with a pylon in range.
        for e in self.ents.values() {
            let need = self.power_need(e.id);
            if need <= 0.0 {
                continue;
            }
            let near = |p: V2| match &e.kind {
                EntKind::Dome(d) => footprint(d.kind, e.pos).distance(p) <= SUPPLY_RANGE,
                _ => e.pos.distance(p) <= SUPPLY_RANGE,
            };
            if let Some(g) = grids.iter_mut().find(|g| g.pylons.iter().any(|(_, p)| near(*p))) {
                g.consumers.push(e.id);
                g.drain_per_day += need;
            }
        }
        grids
    }

    /// Draws power for `dt` seconds and returns who has it.
    pub(super) fn step_power(&mut self, dt: f32) -> BTreeSet<Id> {
        let mut powered = BTreeSet::new();
        for grid in self.grids() {
            let need = grid.drain_per_day / DAY_SECS * dt;
            if need <= 0.0 {
                continue;
            }
            // Keep each pylon's buffer topped up from its flowers.
            let mut available = 0.0;
            for &(id, _) in &grid.pylons {
                let species = self.species.clone();
                let Some(EntKind::Machine(m)) = self.ents.get_mut(&id).map(|e| &mut e.kind) else {
                    continue;
                };
                let mut loaded = false;
                while m.store < need.max(1.0) {
                    let flower = m
                        .inv
                        .stacks()
                        .find(|s| s.item.charge(&species) > 0.0)
                        .map(|s| s.item);
                    let Some(flower) = flower else { break };
                    m.inv.take(flower, 1);
                    m.store += flower.charge(&species);
                    loaded = true;
                }
                available += m.store;
                if loaded {
                    self.touch(id);
                }
            }
            if available < need {
                continue;
            }
            let mut left = need;
            for &(id, _) in &grid.pylons {
                if let Some(EntKind::Machine(m)) = self.ents.get_mut(&id).map(|e| &mut e.kind) {
                    let take = m.store.min(left);
                    m.store -= take;
                    left -= take;
                }
            }
            self.stats.power_used += need;
            powered.extend(grid.consumers);
        }
        powered
    }

    /// Whether a pylon accepts an item in its rack.
    pub fn is_fuel(&self, item: Item) -> bool {
        item.charge(&self.species) > 0.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colony::Machine;
    use crate::colony::domes::Dome;
    use crate::colony::geom::v2;
    use crate::colony::plants::VOLTBLOOM;
    use crate::colony::testing::*;

    fn pylon(colony: &mut Colony, at: V2, flowers: u32) -> Id {
        let mut m = Machine::new(MachineKind::Pylon, "Pylon".into());
        m.inv.add(Item::Crop(VOLTBLOOM), flowers, 1.0);
        colony.spawn(at, EntKind::Machine(m))
    }

    #[test]
    fn pylons_link_and_power_what_is_near() {
        let (_, mut colony, _) = colony();
        let base = v2(1000.0, 300.0);
        let a = pylon(&mut colony, base, 2);
        let b = pylon(&mut colony, v2(base.x + 50.0, base.y), 0);
        let far = pylon(&mut colony, v2(base.x + 400.0, base.y), 1);
        let kitchen = colony.spawn(
            v2(base.x + 110.0, base.y),
            EntKind::Dome(Dome::new(DomeKind::Kitchen, "Kitchen".into())),
        );
        let gen_far = colony.spawn(
            v2(base.x + 900.0, base.y),
            EntKind::Machine(Machine::new(MachineKind::OxygenGenerator, "O2".into())),
        );
        let grids = colony.grids();
        assert_eq!(grids.len(), 2);
        let main = grids.iter().find(|g| g.pylons.len() == 2).unwrap();
        assert!(main.pylons.iter().any(|p| p.0 == a) && main.pylons.iter().any(|p| p.0 == b));
        assert_eq!(main.stored, 200.0);
        assert_eq!(
            main.consumers,
            vec![kitchen],
            "the kitchen is in range of pylon b only"
        );
        assert_eq!(main.drain_per_day, 20.0);
        let lone = grids.iter().find(|g| g.pylons[0].0 == far).unwrap();
        assert!(lone.consumers.is_empty());

        let powered = colony.step_power(1.0);
        assert!(powered.contains(&kitchen));
        assert!(!powered.contains(&gen_far), "nothing in range of the generator");
        // A flower was loaded into the buffer and a little charge was used.
        let after = colony.grids();
        let main = after.iter().find(|g| g.pylons.len() == 2).unwrap();
        assert!((main.stored - (200.0 - 20.0 / DAY_SECS)).abs() < 0.01);
    }

    #[test]
    fn an_empty_grid_powers_nothing() {
        let (_, mut colony, _) = colony();
        let base = v2(1000.0, 300.0);
        pylon(&mut colony, base, 0);
        let kitchen = colony.spawn(
            v2(base.x + 60.0, base.y),
            EntKind::Dome(Dome::new(DomeKind::Kitchen, "Kitchen".into())),
        );
        assert!(!colony.step_power(1.0).contains(&kitchen));
        // One flower runs a kitchen for five days.
        let p = pylon(&mut colony, v2(base.x + 20.0, base.y), 1);
        let mut days = 0.0;
        while colony.step_power(60.0).contains(&kitchen) {
            days += 60.0 / DAY_SECS;
            assert!(days < 10.0);
        }
        assert!((days - 5.0).abs() < 0.2, "ran {days} days");
        let _ = p;
    }
}
