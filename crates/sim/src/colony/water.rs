//! Water. Pumps in lakes and Water Generator Domes feed copper pipe
//! networks; domes and machines touching a network draw from it.
//!
//! Pipes sit on a grid of 4×4-cell tiles and may cross terrain. Tiles that
//! touch side to side form a network. Each second a network shares its
//! supply among its consumers in priority order; what is left over fills
//! tanks, and tanks cover shortfalls.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use super::build::machine_rect;
use super::domes::footprint;
use super::geom::{Rect, V2};
use super::items::{DomeKind, Item, MachineKind};
use super::{Colony, DAY_SECS, EntKind, Fx, Id, PlayerKey, WeatherKind};

/// Side of a pipe tile in cells.
pub const TILE: i32 = 4;
pub const PUMP_PER_DAY: f32 = 120.0;
pub const WATERGEN_PER_DAY: f32 = 90.0;
pub const TANK_LITRES: f32 = 500.0;
const COOLANT_PER_DAY: f32 = 10.0;
/// Longest run of pipe laid in one go.
pub const MAX_RUN: usize = 64;

pub type Tile = (i32, i32);

pub fn tile_of(p: V2) -> Tile {
    (
        (p.x / TILE as f32).floor() as i32,
        (p.y / TILE as f32).floor() as i32,
    )
}

/// Middle of a tile, in cells.
pub fn tile_center(t: Tile) -> V2 {
    V2 {
        x: (t.0 * TILE) as f32 + TILE as f32 / 2.0,
        y: (t.1 * TILE) as f32 + TILE as f32 / 2.0,
    }
}

/// Tiles from `a` to `b`: along x first, then y, so runs are tidy elbows.
pub fn run(a: Tile, b: Tile) -> Vec<Tile> {
    let mut out = vec![a];
    let mut cur = a;
    while cur.0 != b.0 {
        cur.0 += (b.0 - cur.0).signum();
        out.push(cur);
    }
    while cur.1 != b.1 {
        cur.1 += (b.1 - cur.1).signum();
        out.push(cur);
    }
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Problem {
    NoSource,
    /// Supply is less than what the consumers use.
    Insufficient,
    /// Connected to nothing that uses water.
    NoConsumer,
}

impl Problem {
    pub fn describe(self) -> &'static str {
        match self {
            Problem::NoSource => "No source: connect a Pump in a lake or a Water Generator Dome",
            Problem::Insufficient => "Not enough supply for everything connected",
            Problem::NoConsumer => "Nothing connected uses water",
        }
    }
}

/// One pipe network and what hangs off it.
#[derive(Clone, Debug, PartialEq)]
pub struct Network {
    pub tiles: Vec<Tile>,
    pub sources: Vec<Id>,
    pub consumers: Vec<Id>,
    pub tanks: Vec<Id>,
    /// Litres per day the sources give right now.
    pub supply: f32,
    /// Litres per day the consumers use when running.
    pub demand: f32,
    /// Pipe ends that lead nowhere.
    pub dead_ends: Vec<Tile>,
    pub problems: Vec<Problem>,
}

impl Network {
    pub fn surplus(&self) -> f32 {
        self.supply - self.demand
    }

    /// Whether water is moving (for the pipe animation).
    pub fn flowing(&self) -> bool {
        self.supply > 0.0 && (self.demand > 0.0 || !self.tanks.is_empty())
    }
}

/// How urgently a dome kind gets water when there isn't enough.
fn priority(kind: DomeKind) -> u8 {
    match kind {
        DomeKind::Bedroom | DomeKind::Apartment => 0,
        DomeKind::Green | DomeKind::Dark => 1,
        DomeKind::Barn | DomeKind::Aqua => 2,
        _ => 3,
    }
}

fn tile_rect(r: Rect) -> Rect {
    Rect {
        x0: r.x0.div_euclid(TILE) - 1,
        y0: r.y0.div_euclid(TILE) - 1,
        x1: (r.x1 - 1).div_euclid(TILE) + 2,
        y1: (r.y1 - 1).div_euclid(TILE) + 2,
    }
}

impl Colony {
    /// The pipe networks as they stand. Pure: clients call it for the overlay.
    pub fn networks(&self) -> Vec<Network> {
        // Flood fill the tiles into connected groups.
        let mut group: BTreeMap<Tile, usize> = BTreeMap::new();
        let mut nets: Vec<Network> = Vec::new();
        for &start in &self.pipes {
            if group.contains_key(&start) {
                continue;
            }
            let index = nets.len();
            let mut tiles = Vec::new();
            let mut stack = vec![start];
            group.insert(start, index);
            while let Some(t) = stack.pop() {
                tiles.push(t);
                for n in [(t.0 + 1, t.1), (t.0 - 1, t.1), (t.0, t.1 + 1), (t.0, t.1 - 1)] {
                    if self.pipes.contains(&n) && !group.contains_key(&n) {
                        group.insert(n, index);
                        stack.push(n);
                    }
                }
            }
            nets.push(Network {
                tiles,
                sources: Vec::new(),
                consumers: Vec::new(),
                tanks: Vec::new(),
                supply: 0.0,
                demand: 0.0,
                dead_ends: Vec::new(),
                problems: Vec::new(),
            });
        }

        // Attach everything whose footprint touches a tile.
        let rain = if self.weather.kind == WeatherKind::Rain {
            1.5
        } else {
            1.0
        };
        let mut touched: BTreeSet<Tile> = BTreeSet::new();
        for e in self.ents.values() {
            let rect = match &e.kind {
                EntKind::Dome(d) if d.kind.water_per_day() > 0.0 || d.kind == DomeKind::WaterGen => {
                    footprint(d.kind, e.pos)
                }
                EntKind::Machine(m)
                    if matches!(
                        m.kind,
                        MachineKind::Pump | MachineKind::Tank | MachineKind::OxygenGenerator
                    ) =>
                {
                    machine_rect(m.kind, e.pos)
                }
                _ => continue,
            };
            let tr = tile_rect(rect);
            let mut joined: Option<usize> = None;
            for ty in tr.y0..tr.y1 {
                for tx in tr.x0..tr.x1 {
                    if let Some(&g) = group.get(&(tx, ty)) {
                        touched.insert((tx, ty));
                        joined.get_or_insert(g);
                    }
                }
            }
            let Some(g) = joined else { continue };
            let net = &mut nets[g];
            match &e.kind {
                EntKind::Dome(d) if d.kind == DomeKind::WaterGen => {
                    net.sources.push(e.id);
                    if d.powered {
                        net.supply +=
                            WATERGEN_PER_DAY * rain * d.boost * (1.0 + 0.5 * self.workers_in(e.id) as f32);
                    }
                }
                EntKind::Dome(d) => {
                    net.consumers.push(e.id);
                    net.demand += d.kind.water_per_day();
                }
                EntKind::Machine(m) => match m.kind {
                    MachineKind::Pump => {
                        net.sources.push(e.id);
                        if m.on {
                            net.supply += PUMP_PER_DAY * m.boost;
                        }
                    }
                    MachineKind::Tank => net.tanks.push(e.id),
                    _ => {
                        net.consumers.push(e.id);
                        net.demand += COOLANT_PER_DAY;
                    }
                },
                _ => {}
            }
        }

        for net in &mut nets {
            for &t in &net.tiles {
                let neighbours = [(t.0 + 1, t.1), (t.0 - 1, t.1), (t.0, t.1 + 1), (t.0, t.1 - 1)]
                    .iter()
                    .filter(|n| self.pipes.contains(n))
                    .count();
                if neighbours <= 1 && !touched.contains(&t) && net.tiles.len() > 1 {
                    net.dead_ends.push(t);
                }
            }
            if net.sources.is_empty() {
                net.problems.push(Problem::NoSource);
            } else if net.supply < net.demand {
                net.problems.push(Problem::Insufficient);
            }
            if net.consumers.is_empty() && net.tanks.is_empty() {
                net.problems.push(Problem::NoConsumer);
            }
        }
        nets
    }

    /// Fresh-water surplus across all networks, in litres per day.
    pub fn water_surplus(&self) -> f32 {
        self.networks().iter().map(|n| n.surplus().max(0.0)).sum()
    }

    /// Moves water for `dt` seconds.
    pub(super) fn step_water(&mut self, dt: f32) {
        for net in self.networks() {
            let mut available = net.supply / DAY_SECS * dt;
            // Consumers in priority order.
            let mut order: Vec<(u8, Id)> = net
                .consumers
                .iter()
                .map(|&id| match &self.ents[&id].kind {
                    EntKind::Dome(d) => (priority(d.kind), id),
                    _ => (4, id),
                })
                .collect();
            order.sort();
            let want = |c: &Colony, id: Id| -> f32 {
                match &c.ents[&id].kind {
                    // Refill reserves faster than they drain, up to the cap.
                    EntKind::Dome(d) => (d.reserve_cap() - d.reserve)
                        .max(0.0)
                        .min(d.kind.water_per_day() / DAY_SECS * dt * 4.0),
                    _ => COOLANT_PER_DAY / DAY_SECS * dt,
                }
            };
            let total_want: f32 = order.iter().map(|&(_, id)| want(self, id)).sum();
            // Tanks make up a shortfall.
            if total_want > available {
                for &t in &net.tanks {
                    if let Some(EntKind::Machine(m)) = self.ents.get_mut(&t).map(|e| &mut e.kind) {
                        let take = m.store.min(total_want - available);
                        m.store -= take;
                        available += take;
                    }
                }
            }
            for (_, id) in order {
                let w = want(self, id);
                let give = w.min(available);
                available -= give;
                match self.ents.get_mut(&id).map(|e| &mut e.kind) {
                    Some(EntKind::Dome(d)) => d.reserve += give,
                    // Coolant is a flag on the machine: cooled or not.
                    Some(EntKind::Machine(m)) => {
                        m.store = if give >= w * 0.99 && w > 0.0 { 1.0 } else { 0.0 }
                    }
                    _ => {}
                }
            }
            // The rest goes into tanks.
            for &t in &net.tanks {
                if let Some(EntKind::Machine(m)) = self.ents.get_mut(&t).map(|e| &mut e.kind) {
                    let room = (TANK_LITRES - m.store).max(0.0);
                    let put = room.min(available);
                    m.store += put;
                    available -= put;
                }
            }
        }
        // Coolant flags on machines with no network at all.
        let piped: BTreeSet<Id> = self.networks().iter().flat_map(|n| n.consumers.clone()).collect();
        for e in self.ents.values_mut() {
            if let EntKind::Machine(m) = &mut e.kind
                && m.kind == MachineKind::OxygenGenerator
                && !piped.contains(&e.id)
            {
                m.store = 0.0;
            }
        }
    }

    /// Lays (or with `remove`, takes up) a run of pipe.
    pub(super) fn lay_pipe(
        &mut self,
        key: PlayerKey,
        from: Tile,
        to: Tile,
        remove: bool,
    ) -> Result<(), &'static str> {
        let p = &self.players[&key];
        let reach = p.stats().reach + 12.0;
        if tile_center(from).distance(p.center()) > reach {
            return Err("Start the pipe within reach");
        }
        let tiles = run(from, to);
        if tiles.len() > MAX_RUN {
            return Err("That run is too long: lay it in sections");
        }
        let (w, h) = (self.profile.width / TILE, self.profile.height / TILE);
        if tiles
            .iter()
            .any(|t| t.0 < 1 || t.1 < 1 || t.0 >= w - 1 || t.1 >= h - 1)
        {
            return Err("Out of bounds");
        }
        let at = tile_center(to);
        if remove {
            let taken: Vec<Tile> = tiles.into_iter().filter(|t| self.pipes.contains(t)).collect();
            if taken.is_empty() {
                return Ok(());
            }
            for t in &taken {
                self.pipes.remove(t);
                self.pipe_changes.push((*t, false));
            }
            self.give(key, Item::Pipe, taken.len() as u32);
        } else {
            let new: Vec<Tile> = tiles.into_iter().filter(|t| !self.pipes.contains(t)).collect();
            if new.is_empty() {
                return Ok(());
            }
            let p = self.players.get_mut(&key).unwrap();
            if !p.inv.remove(Item::Pipe, new.len() as u32) {
                return Err("Not enough Copper Pipe");
            }
            p.touch_public();
            for t in new {
                self.pipes.insert(t);
                self.pipe_changes.push((t, true));
            }
        }
        self.fx(Fx::PipePlace, at);
        Ok(())
    }

    /// Pipe tiles added (true) and removed (false) since the last call.
    pub fn take_pipe_changes(&mut self) -> Vec<(Tile, bool)> {
        std::mem::take(&mut self.pipe_changes)
    }

    pub fn apply_pipe_changes(&mut self, changes: &[(Tile, bool)]) {
        for &(t, on) in changes {
            if on {
                self.pipes.insert(t);
            } else {
                self.pipes.remove(&t);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colony::Machine;
    use crate::colony::actions::Action;
    use crate::colony::domes::Dome;
    use crate::colony::geom::v2;
    use crate::colony::inventory::Stack;
    use crate::colony::testing::colony;

    /// A pump at x=1000 and a dome at x=1200 on a line at y=300 (tile row 74).
    fn rig(colony: &mut Colony, dome: DomeKind) -> (Id, Id) {
        let mut pump = Machine::new(MachineKind::Pump, "Pump".into());
        pump.on = true;
        let pump = colony.spawn(v2(1000.0, 300.0), EntKind::Machine(pump));
        let dome = colony.spawn(v2(1200.0, 300.0), EntKind::Dome(Dome::new(dome, "Dome".into())));
        (pump, dome)
    }

    fn pipe(colony: &mut Colony, from: Tile, to: Tile) {
        for t in run(from, to) {
            colony.pipes.insert(t);
        }
    }

    #[test]
    fn runs_are_elbows() {
        assert_eq!(run((0, 0), (2, 1)), vec![(0, 0), (1, 0), (2, 0), (2, 1)]);
        assert_eq!(run((3, 3), (3, 3)), vec![(3, 3)]);
        assert_eq!(tile_of(v2(9.0, 4.0)), (2, 1));
    }

    #[test]
    fn a_pump_fills_a_dome_through_pipes() {
        let (_, mut colony, _) = colony();
        let (pump, dome) = rig(&mut colony, DomeKind::Green);
        // Pipe from the pump to the dome's edge, along the ground line.
        pipe(&mut colony, (251, 74), (288, 74));
        let nets = colony.networks();
        assert_eq!(nets.len(), 1);
        let n = &nets[0];
        assert_eq!(n.sources, vec![pump]);
        assert_eq!(n.consumers, vec![dome]);
        assert_eq!(n.supply, PUMP_PER_DAY);
        assert_eq!(n.demand, 30.0);
        assert!(n.problems.is_empty() && n.dead_ends.is_empty(), "{n:?}");
        assert!(n.flowing());
        assert_eq!(colony.water_surplus(), 90.0);
        for _ in 0..200 {
            colony.step_water(1.0);
        }
        let d = colony.dome(dome).unwrap().1;
        assert!(
            (d.reserve - d.reserve_cap()).abs() < 0.01,
            "reserve fills to its cap: {}",
            d.reserve
        );
    }

    #[test]
    fn problems_are_reported() {
        let (_, mut colony, _) = colony();
        let (pump, dome) = rig(&mut colony, DomeKind::Aqua);
        // A stub that reaches nothing.
        pipe(&mut colony, (100, 20), (110, 20));
        // The dome's pipe stops short of the pump.
        pipe(&mut colony, (262, 74), (288, 74));
        let nets = colony.networks();
        assert_eq!(nets.len(), 2);
        let stub = nets.iter().find(|n| n.consumers.is_empty()).unwrap();
        assert!(stub.problems.contains(&Problem::NoSource) && stub.problems.contains(&Problem::NoConsumer));
        assert_eq!(stub.dead_ends.len(), 2);
        let dry = nets.iter().find(|n| n.consumers == vec![dome]).unwrap();
        assert_eq!(dry.problems, vec![Problem::NoSource]);
        assert_eq!(dry.dead_ends, vec![(262, 74)]);
        // Connect it, then switch the pump off (lake drained).
        pipe(&mut colony, (251, 74), (262, 74));
        if let EntKind::Machine(m) = &mut colony.ents.get_mut(&pump).unwrap().kind {
            m.on = false;
        }
        let nets = colony.networks();
        let n = nets.iter().find(|n| n.consumers == vec![dome]).unwrap();
        assert_eq!(n.problems, vec![Problem::Insufficient]);
        assert!(!n.flowing());
    }

    #[test]
    fn short_supply_goes_by_priority_and_tanks_buffer() {
        let (_, mut colony, _) = colony();
        let (pump, aqua) = rig(&mut colony, DomeKind::Aqua);
        let apartment = colony.spawn(
            v2(1400.0, 300.0),
            EntKind::Dome(Dome::new(DomeKind::Apartment, "Apt".into())),
        );
        let barn = colony.spawn(
            v2(1600.0, 300.0),
            EntKind::Dome(Dome::new(DomeKind::Barn, "Barn".into())),
        );
        pipe(&mut colony, (251, 74), (390, 74));
        let nets = colony.networks();
        assert_eq!(nets[0].consumers.len(), 3);
        assert_eq!(nets[0].demand, 60.0 + 40.0 + 20.0);
        assert!(nets[0].problems.is_empty(), "120 L/day just covers it");
        // Halve the pump: the apartment drinks first.
        if let EntKind::Machine(m) = &mut colony.ents.get_mut(&pump).unwrap().kind {
            m.boost = 0.25;
        }
        assert_eq!(colony.networks()[0].problems, vec![Problem::Insufficient]);
        for _ in 0..300 {
            colony.step_water(1.0);
        }
        let r = |c: &Colony, id| c.dome(id).unwrap().1.reserve;
        assert!(
            r(&colony, apartment) > r(&colony, aqua) * 3.0,
            "apartment {} vs aqua {}",
            r(&colony, apartment),
            r(&colony, aqua)
        );
        let _ = barn;

        // A full tank covers the shortfall for a while.
        let mut tank = Machine::new(MachineKind::Tank, "Tank".into());
        tank.store = TANK_LITRES;
        let tank = colony.spawn(v2(1100.0, 300.0), EntKind::Machine(tank));
        assert_eq!(colony.networks()[0].tanks, vec![tank]);
        for id in [aqua, apartment, barn] {
            colony.dome_mut(id).unwrap().reserve = 0.0;
        }
        for _ in 0..300 {
            colony.step_water(1.0);
        }
        assert!(
            r(&colony, aqua) > 5.0,
            "the tank feeds the aqua dome: {}",
            r(&colony, aqua)
        );
        let EntKind::Machine(m) = &colony.ents[&tank].kind else {
            panic!()
        };
        assert!(m.store < TANK_LITRES);
        // With the pump back at full power and reserves full, the tank refills.
        if let EntKind::Machine(m) = &mut colony.ents.get_mut(&pump).unwrap().kind {
            m.boost = 4.0;
        }
        for _ in 0..3000 {
            colony.step_water(1.0);
        }
        let EntKind::Machine(m) = &colony.ents[&tank].kind else {
            panic!()
        };
        assert!(m.store > TANK_LITRES * 0.9);
    }

    #[test]
    fn water_generators_supply_and_generators_get_coolant() {
        let (_, mut colony, _) = colony();
        let mut wg = Dome::new(DomeKind::WaterGen, "WG".into());
        wg.powered = true;
        let wg = colony.spawn(v2(1000.0, 300.0), EntKind::Dome(wg));
        let o2 = colony.spawn(
            v2(1100.0, 300.0),
            EntKind::Machine(Machine::new(MachineKind::OxygenGenerator, "O2".into())),
        );
        pipe(&mut colony, (258, 74), (273, 74));
        let nets = colony.networks();
        assert_eq!(nets[0].sources, vec![wg]);
        assert_eq!(nets[0].consumers, vec![o2]);
        assert_eq!(nets[0].supply, WATERGEN_PER_DAY);
        colony.weather.kind = WeatherKind::Rain;
        assert_eq!(colony.networks()[0].supply, WATERGEN_PER_DAY * 1.5);
        colony.step_water(1.0);
        let EntKind::Machine(m) = &colony.ents[&o2].kind else {
            panic!()
        };
        assert_eq!(m.store, 1.0, "cooled");
        colony.pipes.clear();
        colony.step_water(1.0);
        let EntKind::Machine(m) = &colony.ents[&o2].kind else {
            panic!()
        };
        assert_eq!(m.store, 0.0);
    }

    #[test]
    fn laying_pipe_uses_items_and_taking_it_up_returns_them() {
        let (mut world, mut colony, key) = colony();
        let pos = colony.players[&key].center();
        let from = tile_of(pos);
        let to = (from.0 + 9, from.1 + 2);
        colony.players.get_mut(&key).unwrap().inv.slots[0] = Some(Stack::new(Item::Pipe, 10));
        assert_eq!(
            colony.apply(
                &mut world,
                key,
                Action::Pipe {
                    from,
                    to,
                    remove: false
                }
            ),
            Err("Not enough Copper Pipe")
        );
        colony.players.get_mut(&key).unwrap().inv.slots[0] = Some(Stack::new(Item::Pipe, 20));
        colony
            .apply(
                &mut world,
                key,
                Action::Pipe {
                    from,
                    to,
                    remove: false,
                },
            )
            .unwrap();
        assert_eq!(colony.pipes.len(), 12);
        assert_eq!(colony.players[&key].inv.count(Item::Pipe), 8);
        // Overlapping runs only pay for new tiles.
        colony
            .apply(
                &mut world,
                key,
                Action::Pipe {
                    from,
                    to: (from.0 + 10, from.1),
                    remove: false,
                },
            )
            .unwrap();
        assert_eq!(colony.players[&key].inv.count(Item::Pipe), 7);
        let changes = colony.take_pipe_changes();
        assert_eq!(changes.len(), 13);
        let mut replica = BTreeSet::new();
        std::mem::swap(&mut replica, &mut colony.pipes);
        colony.apply_pipe_changes(&changes);
        assert_eq!(colony.pipes, replica);
        colony
            .apply(
                &mut world,
                key,
                Action::Pipe {
                    from,
                    to,
                    remove: true,
                },
            )
            .unwrap();
        assert_eq!(colony.pipes.len(), 1);
        assert_eq!(colony.players[&key].inv.count(Item::Pipe), 19);
        assert_eq!(
            colony.apply(
                &mut world,
                key,
                Action::Pipe {
                    from: (from.0 + 60, from.1),
                    to,
                    remove: false
                }
            ),
            Err("Start the pipe within reach")
        );
    }
}
