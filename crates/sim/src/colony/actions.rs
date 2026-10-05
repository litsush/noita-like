//! Everything a player can ask the colony to do. Clients send these to the
//! host; the host (or a singleplayer game) applies them with
//! [`Colony::apply`]. An `Err` is shown to the player as a toast.

use serde::{Deserialize, Serialize};

use super::geom::{V2, v2};
use super::inventory::{Inventory, move_between};
use super::{BOLT_COST, BOLT_SPEED, Bolt, Colony, EntKind, Fx, HOTBAR, Id, LOCKOUT_SECS, PlayerKey};
use crate::material::Material;
use crate::world::World;

/// An inventory a player can open.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Container {
    /// The player's own inventory.
    Me,
    /// A chest inside a dome.
    DomeChest(Id, u8),
    /// A chest or pylon standing on its own.
    Machine(Id),
    /// A pack, cache or pod.
    Crate(Id),
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum InvOp {
    /// Drag and drop (or a right-click split) between two slots.
    Move {
        from: (Container, u16),
        to: (Container, u16),
        count: u32,
    },
    /// Shift-click: send a stack wherever it fits in the other inventory.
    QuickMove {
        from: (Container, u16),
        to: Container,
    },
    Sort(Container),
    /// Move items the container already holds from the player into it.
    QuickStack(Container),
    DepositAll(Container),
    LootAll(Container),
    /// Quick stack into every chest in reach.
    QuickStackNearby,
    Trash(u16),
    /// Take back the last trashed stack.
    Untrash,
    Favorite(u16),
    /// Throw items on the ground.
    Drop {
        slot: u16,
        count: u32,
    },
    Select(u8),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum Action {
    /// Dig with the multitool at a cell.
    Dig {
        x: i32,
        y: i32,
    },
    /// Fire the laser from `from` along `dir`.
    Fire {
        from: V2,
        dir: V2,
        /// 0–1: how long the shot was charged (Bolt Enhancements).
        charge: f32,
    },
    /// Use the selected hotbar item at a point (place a block, …).
    Use {
        at: V2,
    },
    Inv(InvOp),
    /// Synthesize `count` batches of a recipe (an index into `recipes()`).
    Craft {
        recipe: u16,
        count: u16,
    },
    /// Sell items from the player's inventory to Earth.
    Sell {
        item: super::items::Item,
        count: u32,
    },
    Buy(super::economy::Offer),
    AssignWorker {
        worker: Id,
        job: Option<Id>,
    },
    /// Work a planter in a dome.
    Planter {
        dome: Id,
        slot: u8,
        op: super::farming::PlanterOp,
    },
    /// Harvest a plant growing outside a planter.
    Harvest {
        plant: Id,
    },
    /// Put the held animal or fish fry into the dome the player is in.
    Stock,
    /// Choose what a Dome Dome builds.
    SetTarget {
        dome: Id,
        target: Option<super::items::DomeKind>,
    },
    /// Pack a machine or dome back into an item.
    Dismantle {
        ent: Id,
    },
    Rename {
        ent: Id,
        name: String,
    },
    /// Lie down in a bed of the dome the player is in. Sets their spawn.
    Sleep,
    /// Lay a run of copper pipe between two tiles, or take it up.
    Pipe {
        from: super::water::Tile,
        to: super::water::Tile,
        remove: bool,
    },
    /// Call the colony ship down. Needs every readiness target met.
    CallShip,
    Robot {
        id: Id,
        op: super::robots::RobotOp,
    },
    Area(super::robots::AreaOp),
    /// Synthesize the next tier of a body-mod set.
    CraftMod {
        set: u8,
    },
    /// Wear a set in its slot, or clear the slot. At the Mod Bay.
    Equip {
        slot: super::mods::Slot,
        set: Option<u8>,
    },
    ArmTask {
        arm: u8,
        task: super::mods::ArmTask,
    },
    /// Send the free-flying arm somewhere, or call it back.
    SendArm {
        at: Option<V2>,
    },
    /// The player landed hard (Stompers).
    Stomp {
        speed: f32,
    },
    Teleport(super::modfx::Teleport),
    RainDance,
    Heal {
        target: PlayerKey,
    },
    /// Green Fingers on a planter or an outdoor plant.
    Tickle {
        planter: Option<(Id, u8)>,
        plant: Option<Id>,
    },
    /// The grapple hook landed here (Grapple Glove).
    Yank {
        at: V2,
    },
    Pin(super::modfx::PinOp),
    Splice {
        machine: Id,
        a: super::plants::SpeciesId,
        b: super::plants::SpeciesId,
    },
}

type Outcome = Result<(), &'static str>;

impl Colony {
    /// Applies a player's action. Host only.
    pub fn apply(&mut self, world: &mut World, key: PlayerKey, action: Action) -> Outcome {
        let Some(p) = self.players.get(&key) else {
            return Err("Unknown player");
        };
        if !p.alive() {
            return Err("You are blacked out");
        }
        match action {
            Action::Dig { x, y } => self.dig(world, key, x, y),
            Action::Fire { from, dir, charge } => self.fire(key, from, dir, charge),
            Action::Use { at } => self.use_item(world, key, at),
            Action::Inv(op) => self.inv_op(key, op),
            Action::Craft { recipe, count } => self.craft(key, recipe, count),
            Action::Sell { item, count } => self.sell(key, item, count),
            Action::Buy(offer) => self.buy(key, offer),
            Action::AssignWorker { worker, job } => self.assign_worker(worker, job),
            Action::Planter { dome, slot, op } => self.planter_op(key, dome, slot, op),
            Action::Harvest { plant } => self.harvest_plant(key, plant),
            Action::Stock => self.stock(key),
            Action::SetTarget { dome, target } => self.set_target(dome, target),
            Action::Dismantle { ent } => self.dismantle(world, key, ent),
            Action::Rename { ent, name } => self.rename(ent, &name),
            Action::Sleep => self.sleep(key),
            Action::Pipe { from, to, remove } => self.lay_pipe(key, from, to, remove),
            Action::CallShip => self.call_ship(key),
            Action::Robot { id, op } => self.robot_op(key, id, op),
            Action::Area(op) => self.area_op(op),
            Action::CraftMod { set } => self.craft_mod(key, set),
            Action::Equip { slot, set } => self.equip(key, slot, set),
            Action::ArmTask { arm, task } => self.set_arm_task(key, arm, task),
            Action::SendArm { at } => self.send_arm(key, at),
            Action::Stomp { speed } => self.stomp(world, key, speed),
            Action::Teleport(to) => self.teleport(key, to),
            Action::RainDance => self.rain_dance(key),
            Action::Heal { target } => self.heal(key, target),
            Action::Tickle { planter, plant } => self.tickle(key, planter, plant),
            Action::Yank { at } => self.yank(key, at),
            Action::Pin(op) => self.pin_op(key, op),
            Action::Splice { machine, a, b } => self.splice(key, machine, a, b),
        }
    }

    fn sleep(&mut self, key: PlayerKey) -> Outcome {
        let p = &self.players[&key];
        let dome = p
            .in_dome
            .ok_or("Beds are in the starter dome and Bedroom Domes")?;
        let (_, d) = self.dome(dome).ok_or("That dome is gone")?;
        if d.kind.beds() == 0 {
            return Err("There are no beds here");
        }
        let taken = self
            .online()
            .filter(|o| o.sleeping && o.in_dome == Some(dome))
            .count() as u32;
        if taken >= d.kind.beds() {
            return Err("Every bed is taken");
        }
        let night = self.clock.is_night();
        let p = self.players.get_mut(&key).unwrap();
        p.spawn = Some(dome);
        p.sleeping = night;
        p.touch_public();
        self.toast(
            Some(key),
            if night {
                "Sleeping. When everyone is in bed, the night passes."
            } else {
                "This is where you'll wake up now. Beds are for sleeping at night."
            },
            true,
        );
        Ok(())
    }

    fn in_reach(&self, key: PlayerKey, at: V2, extra: f32) -> bool {
        let p = &self.players[&key];
        p.center().distance(at) <= p.stats().reach + extra
    }

    fn dig(&mut self, world: &mut World, key: PlayerKey, x: i32, y: i32) -> Outcome {
        let at = v2(x as f32 + 0.5, y as f32 + 0.5);
        let p = self.players.get_mut(&key).unwrap();
        let stats = p.stats();
        if p.dig_wait > 0.0 {
            return Ok(());
        }
        if p.center().distance(at) > stats.reach + 4.0 {
            return Err("Out of reach");
        }
        p.dig_wait = 0.045;
        let power = (38.0 * stats.dig_power).min(250.0) as u8;
        let dug = world.dig(x, y, stats.dig_radius, power);
        if !dug.is_empty() {
            self.collect_dug(key, &dug, at);
        }
        Ok(())
    }

    fn fire(&mut self, key: PlayerKey, from: V2, dir: V2, charge: f32) -> Outcome {
        let crit_roll = self.rng.next_f32();
        let p = self.players.get_mut(&key).unwrap();
        let stats = p.stats();
        let charge = if stats.overcharge {
            charge.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let crit = if crit_roll < stats.crit {
            stats.crit_mult
        } else {
            1.0
        };
        let dir = dir.normalized();
        if dir == V2::ZERO || p.center().distance(from) > 30.0 {
            return Err("Bad shot");
        }
        if p.lockout {
            return Err("Battery recharging");
        }
        // A charged shot costs more; with too little battery it fires weaker.
        let charge = charge.min(((p.battery / (BOLT_COST * stats.bolt_cost)) - 1.0).max(0.0) / 1.5);
        let cost = BOLT_COST * stats.bolt_cost * (1.0 + 1.5 * charge);
        if p.battery < cost {
            return Err("Battery too low");
        }
        p.battery -= cost;
        p.battery_wait = 0.7;
        if p.battery < 1.0 && !stats.no_lockout {
            // Drained: a longer wait before it recharges.
            p.battery = 0.0;
            p.lockout = true;
            p.battery_wait = LOCKOUT_SECS;
        }
        for i in 0..stats.bolt_count {
            // Extra bolts fan out slightly.
            let spread = (i as f32 - (stats.bolt_count - 1) as f32 / 2.0) * 0.07;
            let (s, c) = spread.sin_cos();
            let d = v2(dir.x * c - dir.y * s, dir.x * s + dir.y * c);
            self.spawn(
                from,
                EntKind::Bolt(Bolt {
                    vel: d * BOLT_SPEED,
                    damage: 15.0 * stats.bolt_damage * (1.0 + 2.0 * charge) * crit,
                    size: stats.bolt_size * (1.0 + 0.8 * charge) * if crit > 1.0 { 1.25 } else { 1.0 },
                    owner: Some(key),
                    life: 1.3,
                    web: false,
                }),
            );
        }
        self.fx(Fx::BoltFire, from);
        Ok(())
    }

    fn use_item(&mut self, world: &mut World, key: PlayerKey, at: V2) -> Outcome {
        let p = &self.players[&key];
        if p.center().distance(at) > p.stats().place_reach {
            return Err("Out of reach");
        }
        let slot = p.selected as usize;
        let Some(stack) = p.held() else {
            return Ok(());
        };
        use super::items::Item;
        match stack.item {
            item if item.block().is_some() => self.place_block(world, key, slot, item.block().unwrap(), at),
            Item::DomeKit(kind) => self.place_dome(world, key, slot, kind, at),
            Item::Machine(kind) => self.place_machine(world, key, slot, kind, at),
            Item::Seed(species) => self.plant_outdoors(world, key, slot, species, at),
            Item::WateringCan => self.use_can(world, key, at),
            Item::RobotKit => self.deploy_robot(world, key, slot, at),
            Item::Cluckbug | Item::MilkGrub | Item::FishFry => self.stock(key),
            _ => Err("Can't use that here"),
        }
    }

    fn place_block(
        &mut self,
        world: &mut World,
        key: PlayerKey,
        slot: usize,
        block: Material,
        at: V2,
    ) -> Outcome {
        let (x, y) = at.cell();
        // Don't wall anyone in.
        let blocked = self.online().any(|p| {
            p.alive()
                && (p.pose.pos.x - at.x).abs() < 8.0
                && at.y > p.pose.pos.y - 26.0
                && at.y < p.pose.pos.y + 4.0
        });
        if blocked {
            return Ok(());
        }
        if self.dome_at(at).is_some() {
            return Err("Not inside a dome");
        }
        if world.paint_circle(x, y, 2, block) {
            let p = self.players.get_mut(&key).unwrap();
            if let Some(s) = &mut p.inv.slots[slot] {
                s.count -= 1;
                if s.count == 0 {
                    p.inv.slots[slot] = None;
                }
            }
            p.touch();
            self.fx(Fx::Place, at);
        }
        Ok(())
    }

    // ---- inventories -------------------------------------------------------

    /// Where a container is, for reach checks (`None` if it doesn't exist).
    fn container_pos(&self, key: PlayerKey, c: Container) -> Option<V2> {
        match c {
            Container::Me => Some(self.players[&key].center()),
            Container::DomeChest(id, i) => {
                let (e, d) = self.dome(id)?;
                d.chests.get(i as usize)?;
                d.kind
                    .fixtures()
                    .iter()
                    .find(|f| f.fixture == super::domes::Fixture::Chest(i))
                    .map(|f| super::domes::fixture_pos(e.pos, f))
            }
            Container::Machine(id) | Container::Crate(id) => {
                let e = self.ents.get(&id)?;
                matches!(
                    e.kind,
                    EntKind::Machine(_) | EntKind::Crate(_) | EntKind::Robot(_)
                )
                .then_some(e.pos)
            }
        }
    }

    pub(super) fn container_mut_for(&mut self, key: PlayerKey, c: Container) -> Option<&mut Inventory> {
        self.container_mut(key, c)
    }

    fn container_mut(&mut self, key: PlayerKey, c: Container) -> Option<&mut Inventory> {
        match c {
            Container::Me => self.players.get_mut(&key).map(|p| {
                p.touch();
                &mut p.inv
            }),
            Container::DomeChest(id, i) => self.dome_mut(id)?.chests.get_mut(i as usize),
            Container::Machine(id) | Container::Crate(id) => {
                self.touch(id);
                match &mut self.ents.get_mut(&id)?.kind {
                    EntKind::Machine(m) => Some(&mut m.inv),
                    EntKind::Crate(c) => Some(&mut c.inv),
                    EntKind::Robot(r) => Some(&mut r.inv),
                    _ => None,
                }
            }
        }
    }

    fn reachable(&self, key: PlayerKey, c: Container) -> Outcome {
        let pos = self.container_pos(key, c).ok_or("That container is gone")?;
        // Mods that open the colony stockpile from anywhere.
        if matches!(c, Container::DomeChest(..)) && self.players[&key].stats().remote_stockpile {
            return Ok(());
        }
        if self.in_reach(key, pos, 16.0) {
            Ok(())
        } else {
            Err("Too far away")
        }
    }

    /// Runs `f` on two different containers at once.
    fn with_two<R>(
        &mut self,
        key: PlayerKey,
        a: Container,
        b: Container,
        f: impl FnOnce(&mut Inventory, &mut Inventory) -> R,
    ) -> Result<R, &'static str> {
        self.reachable(key, a)?;
        self.reachable(key, b)?;
        if a == b {
            return Err("Same container");
        }
        let mut first = std::mem::take(self.container_mut(key, a).ok_or("That container is gone")?);
        let result = match self.container_mut(key, b) {
            Some(second) => Ok(f(&mut first, second)),
            None => Err("That container is gone"),
        };
        *self.container_mut(key, a).unwrap() = first;
        result
    }

    /// Removes emptied packs and pods.
    fn tidy_crate(&mut self, c: Container) {
        if let Container::Crate(id) = c
            && let Some(EntKind::Crate(k)) = self.ents.get(&id).map(|e| &e.kind)
            && k.inv.is_empty()
        {
            self.despawn(id);
        }
    }

    pub(super) fn apply_inv(&mut self, key: PlayerKey, op: InvOp) -> Outcome {
        self.inv_op(key, op)
    }

    fn inv_op(&mut self, key: PlayerKey, op: InvOp) -> Outcome {
        let mult = self.players[&key].stats().stack_mult;
        match op {
            InvOp::Move { from, to, count } => {
                let (fi, ti) = (from.1 as usize, to.1 as usize);
                if from.0 == to.0 {
                    self.reachable(key, from.0)?;
                    let inv = self.container_mut(key, from.0).ok_or("That container is gone")?;
                    inv.move_within(fi, ti, count, mult);
                } else {
                    self.with_two(key, from.0, to.0, |a, b| {
                        if fi < a.slots.len() && ti < b.slots.len() {
                            move_between(&mut a.slots[fi], &mut b.slots[ti], count, mult);
                        }
                    })?;
                    self.tidy_crate(from.0);
                }
            }
            InvOp::QuickMove { from, to } => {
                if from.0 == to {
                    // Within the player's own inventory: between hotbar and bag.
                    self.reachable(key, to)?;
                    let inv = self.container_mut(key, to).ok_or("That container is gone")?;
                    let slot = from.1 as usize;
                    let Some(stack) = inv.slots.get_mut(slot).and_then(|s| s.take()) else {
                        return Ok(());
                    };
                    let range = if slot < HOTBAR {
                        HOTBAR..inv.slots.len()
                    } else {
                        0..HOTBAR
                    };
                    let mut tmp = Inventory {
                        slots: vec![Some(stack)],
                    };
                    tmp.quick_move_range(0, inv, mult, range);
                    if let Some(left) = tmp.slots[0] {
                        inv.slots[slot] = Some(left);
                    }
                } else {
                    self.with_two(key, from.0, to, |a, b| {
                        // Into the player: fill the bag before the hotbar.
                        if to == Container::Me {
                            a.quick_move_range(from.1 as usize, b, mult, HOTBAR..b.slots.len());
                        }
                        a.quick_move(from.1 as usize, b, mult);
                    })?;
                    self.tidy_crate(from.0);
                }
            }
            InvOp::Sort(c) => {
                self.reachable(key, c)?;
                let skip = if c == Container::Me { HOTBAR } else { 0 };
                self.container_mut(key, c)
                    .ok_or("That container is gone")?
                    .sort(skip, mult);
            }
            InvOp::QuickStack(c) => {
                self.with_two(key, Container::Me, c, |me, chest| {
                    me.deposit_into(chest, HOTBAR, true, mult)
                })?;
            }
            InvOp::DepositAll(c) => {
                self.with_two(key, Container::Me, c, |me, chest| {
                    me.deposit_into(chest, HOTBAR, false, mult)
                })?;
            }
            InvOp::LootAll(c) => {
                self.with_two(key, c, Container::Me, |chest, me| {
                    for i in 0..chest.slots.len() {
                        chest.quick_move_range(i, me, mult, HOTBAR..me.slots.len());
                        chest.quick_move(i, me, mult);
                    }
                })?;
                self.tidy_crate(c);
            }
            InvOp::QuickStackNearby => {
                let mut moved = 0;
                for c in self.containers_near(key) {
                    if let Ok(n) = self.with_two(key, Container::Me, c, |me, chest| {
                        me.deposit_into(chest, HOTBAR, true, mult)
                    }) {
                        moved += n;
                    }
                }
                if moved == 0 {
                    return Err("Nothing to stack into the chests nearby");
                }
            }
            InvOp::Trash(slot) => {
                let p = self.players.get_mut(&key).unwrap();
                if let Some(s) = p.inv.slots.get_mut(slot as usize)
                    && s.is_some_and(|s| !s.fav)
                {
                    p.trash = s.take();
                    p.touch();
                }
            }
            InvOp::Untrash => {
                let p = self.players.get_mut(&key).unwrap();
                if let Some(s) = p.trash.take() {
                    let left = p.inv.add(s.item, s.count, mult);
                    if left > 0 {
                        p.trash = Some(super::inventory::Stack::new(s.item, left));
                    }
                    p.touch();
                }
            }
            InvOp::Favorite(slot) => {
                let p = self.players.get_mut(&key).unwrap();
                if let Some(Some(s)) = p.inv.slots.get_mut(slot as usize) {
                    s.fav = !s.fav;
                    p.touch();
                }
            }
            InvOp::Drop { slot, count } => {
                let p = self.players.get_mut(&key).unwrap();
                let at = p.center() + v2(p.pose.facing as f32 * 10.0, -4.0);
                let Some(Some(s)) = p.inv.slots.get_mut(slot as usize) else {
                    return Ok(());
                };
                let n = count.min(s.count);
                let item = s.item;
                s.count -= n;
                if s.count == 0 {
                    p.inv.slots[slot as usize] = None;
                }
                p.touch();
                let facing = p.pose.facing as f32;
                // A fresh drop (not merged) so it can be thrown.
                self.spawn(
                    at,
                    EntKind::Drop(super::Drop {
                        item,
                        count: n,
                        vel: v2(facing * 90.0, -60.0),
                        age: -1.2,
                    }),
                );
            }
            InvOp::Select(i) => {
                let p = self.players.get_mut(&key).unwrap();
                p.selected = i.min(HOTBAR as u8);
                p.touch_public();
            }
        }
        Ok(())
    }

    /// Chests (in domes and standing alone) within the player's reach.
    pub fn containers_near(&self, key: PlayerKey) -> Vec<Container> {
        let mut out = Vec::new();
        for e in self.ents.values() {
            match &e.kind {
                EntKind::Dome(d) => {
                    for i in 0..d.chests.len() as u8 {
                        out.push(Container::DomeChest(e.id, i));
                    }
                }
                EntKind::Machine(m) if m.kind == super::items::MachineKind::Chest => {
                    out.push(Container::Machine(e.id))
                }
                _ => {}
            }
        }
        out.retain(|c| self.reachable(key, *c).is_ok());
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colony::creatures::{Creature, CreatureKind};
    use crate::colony::items::Item;
    use crate::colony::testing::*;

    #[test]
    fn digging_needs_reach_and_gives_drops() {
        let (mut world, mut colony, key) = colony();
        let p = colony.players[&key].center();
        // Dig the ground outside the dome's foundation.
        let (x, y) = (colony.profile.spawn_x + 116, colony.profile.spawn_y() + 6);
        let _ = p;
        assert_eq!(
            colony.apply(&mut world, key, Action::Dig { x, y }),
            Err("Out of reach")
        );
        colony.players.get_mut(&key).unwrap().pose.pos = v2(x as f32, y as f32 - 6.0);
        let before = world.material(x, y);
        assert_ne!(before, Material::Empty);
        for _ in 0..120 {
            colony.apply(&mut world, key, Action::Dig { x, y }).unwrap();
            colony.step(&mut world, 1.0 / 60.0);
        }
        assert_eq!(world.material(x, y), Material::Empty);
        let p = &colony.players[&key];
        assert!(p.inv.count(Item::Dirt) + p.inv.count(Item::Sand) + p.inv.count(Item::Stone) > 0);
        // Dome plating can't be dug.
        let home = colony.ents[&colony.home].pos.cell();
        colony.players.get_mut(&key).unwrap().pose.pos = v2(home.0 as f32, home.1 as f32);
        for _ in 0..30 {
            colony
                .apply(
                    &mut world,
                    key,
                    Action::Dig {
                        x: home.0,
                        y: home.1 + 1,
                    },
                )
                .unwrap();
            colony.step(&mut world, 1.0 / 60.0);
        }
        assert_eq!(world.material(home.0, home.1 + 1), Material::Plating);
    }

    #[test]
    fn laser_drains_battery_locks_out_and_recharges() {
        let (mut world, mut colony, key) = colony();
        let from = colony.players[&key].center();
        let fire = |c: &mut Colony, w: &mut World| {
            c.apply(
                w,
                key,
                Action::Fire {
                    from,
                    dir: v2(1.0, 0.0),
                    charge: 0.0,
                },
            )
        };
        fire(&mut colony, &mut world).unwrap();
        assert_eq!(colony.players[&key].battery, 100.0 - BOLT_COST);
        assert!(colony.ents.values().any(|e| matches!(e.kind, EntKind::Bolt(_))));
        // A short pause, then it recharges by itself.
        run(&mut colony, &mut world, 0.5);
        assert_eq!(colony.players[&key].battery, 100.0 - BOLT_COST);
        run(&mut colony, &mut world, 1.5);
        assert_eq!(colony.players[&key].battery, 100.0);
        // Spam it dry.
        let mut shots = 0;
        while fire(&mut colony, &mut world).is_ok() {
            shots += 1;
        }
        assert_eq!(shots, 8);
        colony.players.get_mut(&key).unwrap().battery = BOLT_COST;
        fire(&mut colony, &mut world).unwrap();
        let p = &colony.players[&key];
        assert!(p.lockout && p.battery == 0.0);
        run(&mut colony, &mut world, 2.0);
        assert_eq!(fire(&mut colony, &mut world), Err("Battery recharging"));
        assert_eq!(colony.players[&key].battery, 0.0, "locked out for 2.5 s");
        run(&mut colony, &mut world, 1.5);
        assert!(!colony.players[&key].lockout && colony.players[&key].battery > 10.0);
    }

    #[test]
    fn bolts_hit_creatures_and_chip_terrain() {
        let (mut world, mut colony, key) = colony();
        let from = colony.players[&key].center();
        let target = colony.spawn(
            v2(from.x + 50.0, from.y + 11.0),
            EntKind::Creature(Creature::new(CreatureKind::Puffback)),
        );
        colony
            .apply(
                &mut world,
                key,
                Action::Fire {
                    from,
                    dir: v2(1.0, 0.0),
                    charge: 0.0,
                },
            )
            .unwrap();
        run(&mut colony, &mut world, 0.5);
        let EntKind::Creature(c) = &colony.ents[&target].kind else {
            panic!()
        };
        assert!(c.hp < CreatureKind::Puffback.max_hp());
        assert!(
            !colony.ents.values().any(|e| matches!(e.kind, EntKind::Bolt(_))),
            "bolt spent"
        );
        // Straight down into the ground outside.
        let out = v2(
            colony.profile.spawn_x as f32 + 116.0,
            colony.profile.spawn_y() as f32 - 12.0,
        );
        colony.players.get_mut(&key).unwrap().pose.pos = v2(out.x, out.y + 11.0);
        let (gx, gy) = (out.x as i32, colony.profile.spawn_y());
        assert!(world.is_solid(gx, gy));
        for _ in 0..4 {
            colony.players.get_mut(&key).unwrap().battery = 100.0;
            colony
                .apply(
                    &mut world,
                    key,
                    Action::Fire {
                        from: out,
                        dir: v2(0.0, 1.0),
                        charge: 0.0,
                    },
                )
                .unwrap();
            run(&mut colony, &mut world, 0.3);
        }
        assert!(!world.is_solid(gx, gy), "bolts chip the ground");
    }

    #[test]
    fn blocks_are_placed_from_the_hotbar() {
        let (mut world, mut colony, key) = colony();
        let out = v2(
            colony.profile.spawn_x as f32 + 200.0,
            colony.profile.surface_at(colony.profile.spawn_x + 200) as f32,
        );
        let p = colony.players.get_mut(&key).unwrap();
        p.pose.pos = out;
        p.inv.slots[0] = Some(crate::colony::inventory::Stack::new(Item::Stone, 2));
        let at = v2(out.x + 30.0, out.y - 30.0);
        colony.apply(&mut world, key, Action::Use { at }).unwrap();
        assert_eq!(world.material(at.x as i32, at.y as i32), Material::Stone);
        assert_eq!(colony.players[&key].inv.count(Item::Stone), 1);
        // Not on top of yourself, and not without the item.
        colony
            .apply(
                &mut world,
                key,
                Action::Use {
                    at: v2(out.x, out.y - 10.0),
                },
            )
            .unwrap();
        assert_eq!(colony.players[&key].inv.count(Item::Stone), 1);
        colony
            .apply(&mut world, key, Action::Inv(InvOp::Select(3)))
            .unwrap();
        colony.apply(&mut world, key, Action::Use { at }).unwrap();
        assert_eq!(colony.players[&key].inv.count(Item::Stone), 1);
    }

    #[test]
    fn chest_operations() {
        let (mut world, mut colony, key) = colony();
        let home = colony.home;
        let chest = Container::DomeChest(home, 0);
        // Stand by the chests.
        let pos = colony.container_pos(key, chest).unwrap();
        colony.players.get_mut(&key).unwrap().pose.pos = pos;
        let mut act = |c: &mut Colony, op| c.apply(&mut world, key, Action::Inv(op));

        // Shift-click the watering can out of the chest: lands in the bag.
        let can = colony.dome(home).unwrap().1.chests[0]
            .slots
            .iter()
            .position(|s| s.is_some_and(|s| s.item == Item::WateringCan))
            .unwrap() as u16;
        act(
            &mut colony,
            InvOp::QuickMove {
                from: (chest, can),
                to: Container::Me,
            },
        )
        .unwrap();
        let p = &colony.players[&key];
        assert_eq!(p.inv.count(Item::WateringCan), 1);
        assert!(
            p.inv.slots[..HOTBAR].iter().all(|s| s.is_none()),
            "bag first, not the hotbar"
        );

        // Drag half the seeds to hotbar slot 2.
        act(
            &mut colony,
            InvOp::Move {
                from: (chest, 0),
                to: (Container::Me, 2),
                count: 2,
            },
        )
        .unwrap();
        assert_eq!(colony.players[&key].inv.slots[2].unwrap().count, 2);
        assert_eq!(colony.dome(home).unwrap().1.chests[0].slots[0].unwrap().count, 2);

        // Deposit all leaves the hotbar alone; quick stack only matches.
        colony.players.get_mut(&key).unwrap().inv.slots[30] =
            Some(crate::colony::inventory::Stack::new(Item::Iron, 9));
        act(&mut colony, InvOp::QuickStack(chest)).unwrap();
        assert_eq!(
            colony.players[&key].inv.count(Item::Iron),
            9,
            "chest has no iron yet"
        );
        act(&mut colony, InvOp::DepositAll(chest)).unwrap();
        assert_eq!(colony.players[&key].inv.count(Item::Iron), 0);
        assert_eq!(colony.players[&key].inv.slots[2].unwrap().count, 2, "hotbar kept");
        colony.players.get_mut(&key).unwrap().inv.slots[30] =
            Some(crate::colony::inventory::Stack::new(Item::Iron, 5));
        act(&mut colony, InvOp::QuickStackNearby).unwrap();
        assert_eq!(colony.dome(home).unwrap().1.chests[0].count(Item::Iron), 14);

        // Favourites can't be trashed; trash keeps the last stack.
        colony.players.get_mut(&key).unwrap().inv.slots[5] =
            Some(crate::colony::inventory::Stack::new(Item::Dirt, 30));
        act(&mut colony, InvOp::Favorite(5)).unwrap();
        act(&mut colony, InvOp::Trash(5)).unwrap();
        assert_eq!(colony.players[&key].inv.count(Item::Dirt), 30);
        act(&mut colony, InvOp::Favorite(5)).unwrap();
        act(&mut colony, InvOp::Trash(5)).unwrap();
        assert_eq!(colony.players[&key].inv.count(Item::Dirt), 0);
        act(&mut colony, InvOp::Untrash).unwrap();
        assert_eq!(colony.players[&key].inv.count(Item::Dirt), 30);

        // Too far away.
        colony.players.get_mut(&key).unwrap().pose.pos = v2(pos.x + 400.0, pos.y);
        assert_eq!(act(&mut colony, InvOp::LootAll(chest)), Err("Too far away"));
    }

    #[test]
    fn dropped_items_can_be_picked_up_by_a_teammate() {
        let (mut world, mut colony, key) = colony();
        colony.join(2, "Grace".into());
        let pos = colony.players[&key].pose.pos;
        colony.players.get_mut(&2).unwrap().pose.pos = v2(pos.x + 60.0, pos.y);
        let p = colony.players.get_mut(&key).unwrap();
        p.pose.facing = 1;
        p.inv.slots[0] = Some(crate::colony::inventory::Stack::new(Item::Gold, 5));
        colony
            .apply(&mut world, key, Action::Inv(InvOp::Drop { slot: 0, count: 3 }))
            .unwrap();
        assert_eq!(colony.players[&key].inv.count(Item::Gold), 2);
        run(&mut colony, &mut world, 1.0);
        // Thrown toward Grace; it doesn't snap back to the thrower at once.
        colony.players.get_mut(&key).unwrap().pose.pos = v2(pos.x - 80.0, pos.y);
        run(&mut colony, &mut world, 3.0);
        assert_eq!(colony.players[&2].inv.count(Item::Gold), 3);
    }

    #[test]
    fn looting_a_pack_removes_it() {
        let (mut world, mut colony, key) = colony();
        colony.players.get_mut(&key).unwrap().inv.slots[20] =
            Some(crate::colony::inventory::Stack::new(Item::Iron, 40));
        colony.black_out(key);
        assert_eq!(colony.players[&key].inv.count(Item::Iron), 0);
        run(&mut colony, &mut world, 6.0);
        let pack = colony
            .ents
            .values()
            .find(|e| matches!(&e.kind, EntKind::Crate(c) if c.kind == crate::colony::CrateKind::Pack))
            .map(|e| (e.id, e.pos))
            .unwrap();
        colony.players.get_mut(&key).unwrap().pose.pos = pack.1;
        colony
            .apply(
                &mut world,
                key,
                Action::Inv(InvOp::LootAll(Container::Crate(pack.0))),
            )
            .unwrap();
        assert_eq!(colony.players[&key].inv.count(Item::Iron), 40);
        assert!(!colony.ents.contains_key(&pack.0));
    }
}
