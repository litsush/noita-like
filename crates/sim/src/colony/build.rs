//! Placing things in the world: dome kits and machines, and taking them
//! back down.

use super::domes::{self, Dome};
use super::geom::{Rect, V2, v2};
use super::items::{DomeKind, Item, MachineKind};
use super::{Colony, EntKind, Fx, Id, Machine, PlayerKey};
use crate::material::{Kind, Material};
use crate::world::World;

/// First solid row at or below `(x, y)`, within `max` cells.
pub fn ground_below(world: &World, x: i32, y: i32, max: i32) -> Option<i32> {
    (y..y + max).find(|&cy| world.material(x, cy).is_solid_for_player())
}

/// Bounding box of a machine standing at `pos`.
pub fn machine_rect(kind: MachineKind, pos: V2) -> Rect {
    let (w, h) = kind.size();
    let (x, y) = pos.cell();
    Rect {
        x0: x - w / 2,
        y0: y - h,
        x1: x + w / 2,
        y1: y,
    }
}

impl Colony {
    /// Where a dome kit aimed at `at` would stand, or why it can't.
    /// Clients call this too, to draw the placement preview.
    pub fn dome_site(&self, world: &World, kind: DomeKind, at: V2) -> Result<V2, &'static str> {
        let (x, y) = at.cell();
        let floor = ground_below(world, x, y - 20, 90).ok_or("Aim at the ground")?;
        let pos = v2(x as f32, floor as f32);
        match domes::placement_error(world, kind, pos, &self.dome_rects()) {
            Some(why) => Err(why),
            None => {
                // Machines and people's belongings can't be built over.
                let r = domes::footprint(kind, pos);
                let blocked = self.ents.values().any(|e| match &e.kind {
                    EntKind::Machine(m) => machine_rect(m.kind, e.pos).overlaps(&r),
                    EntKind::Crate(_) => r.contains(e.pos),
                    _ => false,
                });
                if blocked {
                    Err("Something is in the way")
                } else {
                    Ok(pos)
                }
            }
        }
    }

    /// Where a machine aimed at `at` would stand, or why it can't.
    pub fn machine_site(&self, world: &World, kind: MachineKind, at: V2) -> Result<V2, &'static str> {
        let (x, y) = at.cell();
        let floor = ground_below(world, x, y - 6, 60).ok_or("Aim at the ground")?;
        let pos = v2(x as f32, floor as f32);
        let r = machine_rect(kind, pos);
        // A few cells of slope under the base are fine.
        for cy in r.y0..r.y1 - 4 {
            for cx in r.x0..r.x1 {
                if world.material(cx, cy).is_solid_for_player() {
                    return Err("Not enough room");
                }
            }
        }
        let clash = self.ents.values().any(|e| match &e.kind {
            EntKind::Machine(m) => machine_rect(m.kind, e.pos).overlaps(&r),
            EntKind::Dome(d) => {
                // Inside a dome is fine, but not on top of its fixtures or through its shell.
                let f = domes::footprint(d.kind, e.pos);
                let inside = f.x0 + 4 <= r.x0 && r.x1 <= f.x1 - 4 && f.overlaps(&r);
                (f.overlaps(&r) && !inside)
                    || (inside
                        && d.kind.fixtures().iter().any(|fx| {
                            (domes::fixture_pos(e.pos, fx).x - pos.x).abs() < (r.x1 - r.x0) as f32 / 2.0 + 8.0
                        }))
            }
            _ => false,
        });
        if clash {
            return Err("Something is in the way");
        }
        Ok(pos)
    }

    fn numbered_name(&self, base: &str, count: usize) -> String {
        format!("{base} {}", count + 1)
    }

    pub(super) fn place_dome(
        &mut self,
        world: &mut World,
        key: PlayerKey,
        slot: usize,
        kind: DomeKind,
        at: V2,
    ) -> Result<(), &'static str> {
        let pos = self.dome_site(world, kind, at)?;
        // Nobody gets entombed in the foundation.
        let r = domes::footprint(kind, pos);
        domes::build_cells(world, kind, pos);
        // Plants and loose items under the new dome are cleared away.
        let buried: Vec<Id> = self
            .ents
            .values()
            .filter(|e| {
                matches!(e.kind, EntKind::Plant(_) | EntKind::Drop(_))
                    && r.contains(v2(e.pos.x, e.pos.y - 2.0))
            })
            .map(|e| e.id)
            .collect();
        for id in buried {
            self.despawn(id);
        }
        let n = self.domes().filter(|(_, d)| d.kind == kind).count();
        let mut dome = Dome::new(kind, self.numbered_name(kind.name(), n));
        // Big Dome Energy.
        dome.boost = self.players[&key].stats().dome_boost;
        let id = self.spawn(pos, EntKind::Dome(dome));
        self.consume_held(key, slot);
        self.fx(Fx::DomePlaced, r.center());
        self.toast(None, format!("{} built", self.dome(id).unwrap().1.name), true);
        Ok(())
    }

    pub(super) fn place_machine(
        &mut self,
        world: &mut World,
        key: PlayerKey,
        slot: usize,
        kind: MachineKind,
        at: V2,
    ) -> Result<(), &'static str> {
        let pos = self.machine_site(world, kind, at)?;
        let n = self
            .ents
            .values()
            .filter(|e| matches!(&e.kind, EntKind::Machine(m) if m.kind == kind))
            .count();
        let mut machine = Machine::new(kind, self.numbered_name(kind.name(), n));
        // Unionized.
        machine.boost = self.players[&key].stats().machine_boost;
        self.spawn(pos, EntKind::Machine(machine));
        self.consume_held(key, slot);
        self.fx(Fx::Place, pos);
        Ok(())
    }

    /// Takes one item from a hotbar slot.
    pub(super) fn consume_held(&mut self, key: PlayerKey, slot: usize) {
        if let Some(p) = self.players.get_mut(&key) {
            if let Some(s) = &mut p.inv.slots[slot] {
                s.count -= 1;
                if s.count == 0 {
                    p.inv.slots[slot] = None;
                }
            }
            p.touch_public();
        }
    }

    /// Packs a machine or dome back into an item. Contents are dropped.
    pub(super) fn dismantle(
        &mut self,
        world: &mut World,
        key: PlayerKey,
        id: Id,
    ) -> Result<(), &'static str> {
        let e = self.ents.get(&id).ok_or("It's already gone")?;
        let p = &self.players[&key];
        let pos = e.pos;
        let (item, spill): (Item, Vec<super::inventory::Stack>) = match &e.kind {
            EntKind::Machine(m) => {
                if pos.distance(p.pose.pos) > p.stats().reach + 16.0 {
                    return Err("Too far away");
                }
                (Item::Machine(m.kind), m.inv.stacks().copied().collect())
            }
            EntKind::Dome(d) => {
                if d.kind == DomeKind::Starter {
                    return Err("The starter dome stays");
                }
                if p.in_dome != Some(id) {
                    return Err("Stand inside the dome to pack it up");
                }
                if self.online().any(|o| o.key != key && o.in_dome == Some(id)) {
                    return Err("Someone else is inside");
                }
                let mut spill: Vec<_> = d.chests.iter().flat_map(|c| c.stacks().copied()).collect();
                for pl in &d.planters {
                    if let Some(s) = pl.species {
                        spill.push(super::inventory::Stack::new(Item::Seed(s), 1));
                    }
                }
                (Item::DomeKit(d.kind), spill)
            }
            _ => return Err("That can't be packed up"),
        };
        if let EntKind::Dome(d) = &e.kind {
            domes::clear_cells(world, d.kind, pos);
        }
        self.despawn(id);
        for w in &mut self.workers {
            if w.job == Some(id) {
                w.job = None;
            }
        }
        self.workers.retain(|w| w.home != id);
        self.meta_rev += 1;
        let at = v2(pos.x, pos.y - 6.0);
        // Hot Swap: the contents go straight into the pack.
        let pack = self.players[&key].stats().pack_contents;
        for s in spill {
            if pack {
                self.give(key, s.item, s.count);
            } else {
                self.drop_item(at, s.item, s.count);
            }
        }
        self.give(key, item, 1);
        self.fx(Fx::Place, at);
        Ok(())
    }

    pub(super) fn rename(&mut self, id: Id, name: &str) -> Result<(), &'static str> {
        let name: String = name.chars().filter(|c| !c.is_control()).take(28).collect();
        if name.trim().is_empty() {
            return Err("Give it a name");
        }
        self.touch(id);
        match self.ents.get_mut(&id).map(|e| &mut e.kind) {
            Some(EntKind::Dome(d)) => d.name = name,
            Some(EntKind::Machine(m)) => m.name = name,
            Some(EntKind::Robot(r)) => r.name = name,
            _ => return Err("That can't be renamed"),
        }
        Ok(())
    }

    /// Whether a point is in water (for the pump and the watering can).
    pub fn in_water(world: &World, p: V2) -> bool {
        let (x, y) = p.cell();
        world.material(x, y) == Material::Water
    }

    /// Fraction of a machine's footprint that is under liquid.
    pub fn submerged(world: &World, kind: MachineKind, pos: V2) -> f32 {
        let r = machine_rect(kind, pos);
        let mut wet = 0;
        let mut total = 0;
        for y in r.y0..r.y1 {
            for x in r.x0..r.x1 {
                total += 1;
                if world.material(x, y).kind() == Kind::Liquid {
                    wet += 1;
                }
            }
        }
        wet as f32 / total.max(1) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colony::actions::{Action, InvOp};
    use crate::colony::inventory::Stack;
    use crate::colony::testing::*;

    /// Puts the player on open ground with `item` in hand; returns a point to aim at.
    pub fn outside_with(colony: &mut Colony, key: PlayerKey, item: Item, dx: i32) -> V2 {
        let x = colony.profile.spawn_x + dx;
        let y = colony.profile.surface_at(x);
        let p = colony.players.get_mut(&key).unwrap();
        p.pose.pos = v2(x as f32, y as f32);
        p.in_dome = None;
        p.inv.slots[0] = Some(Stack::new(item, 1));
        p.selected = 0;
        v2(x as f32 + 40.0, y as f32 - 4.0)
    }

    #[test]
    fn dome_kits_become_domes() {
        let (mut world, mut colony, key) = colony();
        let aim = outside_with(&mut colony, key, Item::DomeKit(DomeKind::Green), 300);
        colony.apply(&mut world, key, Action::Use { at: aim }).unwrap();
        let (id, (x, y)) = {
            let (e, d) = colony.domes().find(|(_, d)| d.kind == DomeKind::Green).unwrap();
            assert_eq!(d.name, "Green Dome 1");
            assert_eq!(d.planters.len(), 4);
            (e.id, e.pos.cell())
        };
        assert_eq!(world.material(x, y), Material::Plating);
        assert_eq!(world.material(x, y - 20), Material::Empty);
        assert!(colony.players[&key].held().is_none(), "kit used up");
        assert!(
            colony.dome_at(v2(x as f32, y as f32 - 10.0)).is_some(),
            "breathable inside"
        );
        // A second one right on top is refused and keeps the kit.
        colony.players.get_mut(&key).unwrap().inv.slots[0] =
            Some(Stack::new(Item::DomeKit(DomeKind::Storage), 1));
        assert!(colony.apply(&mut world, key, Action::Use { at: aim }).is_err());
        assert!(colony.players[&key].held().is_some());
        // Packing it up from inside returns the kit and removes the shell.
        let p = colony.players.get_mut(&key).unwrap();
        p.inv.slots[0] = None;
        p.pose.pos = v2(x as f32, y as f32);
        p.in_dome = Some(id);
        colony
            .apply(&mut world, key, Action::Dismantle { ent: id })
            .unwrap();
        assert_eq!(colony.players[&key].inv.count(Item::DomeKit(DomeKind::Green)), 1);
        assert_eq!(world.material(x, y), Material::Empty);
        let home = colony.home;
        colony.players.get_mut(&key).unwrap().in_dome = Some(home);
        assert_eq!(
            colony.apply(&mut world, key, Action::Dismantle { ent: home }),
            Err("The starter dome stays")
        );
    }

    #[test]
    fn machines_stand_on_the_ground_and_can_be_renamed() {
        let (mut world, mut colony, key) = colony();
        let aim = outside_with(&mut colony, key, Item::Machine(MachineKind::Chest), 260);
        // Clear any tree out of the way.
        for y in aim.y as i32 - 60..aim.y as i32 - 2 {
            for x in aim.x as i32 - 60..aim.x as i32 + 30 {
                world.set(x, y, Material::Empty);
            }
        }
        colony.apply(&mut world, key, Action::Use { at: aim }).unwrap();
        let chest = colony
            .ents
            .values()
            .find(|e| matches!(&e.kind, EntKind::Machine(m) if m.kind == MachineKind::Chest))
            .unwrap();
        let (id, pos) = (chest.id, chest.pos);
        assert!(world.is_solid(pos.x as i32, pos.y as i32));
        assert!(!world.is_solid(pos.x as i32, pos.y as i32 - 1));
        // Another machine can't overlap it.
        colony.players.get_mut(&key).unwrap().inv.slots[0] =
            Some(Stack::new(Item::Machine(MachineKind::Lamp), 1));
        assert_eq!(
            colony.apply(
                &mut world,
                key,
                Action::Use {
                    at: v2(pos.x + 4.0, pos.y - 4.0)
                }
            ),
            Err("Something is in the way")
        );
        colony
            .apply(
                &mut world,
                key,
                Action::Use {
                    at: v2(pos.x - 20.0, pos.y - 4.0),
                },
            )
            .unwrap();
        colony
            .apply(
                &mut world,
                key,
                Action::Rename {
                    ent: id,
                    name: "Ore box".into(),
                },
            )
            .unwrap();
        let EntKind::Machine(m) = &colony.ents[&id].kind else {
            panic!()
        };
        assert_eq!(m.name, "Ore box");
        // Fill it, then pack it up: the contents spill and the chest comes back.
        colony.players.get_mut(&key).unwrap().inv.slots[20] = Some(Stack::new(Item::Iron, 5));
        colony
            .apply(
                &mut world,
                key,
                Action::Inv(InvOp::DepositAll(crate::colony::actions::Container::Machine(id))),
            )
            .unwrap();
        colony
            .apply(&mut world, key, Action::Dismantle { ent: id })
            .unwrap();
        assert!(!colony.ents.contains_key(&id));
        colony.players.get_mut(&key).unwrap().pose.pos = pos;
        run(&mut colony, &mut world, 3.0);
        let p = &colony.players[&key];
        assert_eq!(p.inv.count(Item::Machine(MachineKind::Chest)), 1);
        assert_eq!(p.inv.count(Item::Iron), 5, "spilled iron was picked back up");
    }
}
