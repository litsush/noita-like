//! What body mods do on the host: synthesizing and equipping them, the
//! abilities players trigger, and the things mods do by themselves (turrets,
//! drones, automated arms, auras that help plants and teammates).
//! Also map pins and the Gene Splicer, which share the same plumbing.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::body::{line_of_sight, player_solid};
use super::creatures::{self, State};
use super::domes::{Fixture, fixture_pos};
use super::farming::{REGROW, harvest_items};
use super::geom::{V2, v2};
use super::items::{DomeKind, Item, MachineKind};
use super::mods::{self, ArmTask, MOD_SETS, SETS, Slot};
use super::plants::{SpeciesId, splice};
use super::power::PYLON_BUFFER;
use super::recipes::{Blueprint, Ingredient};
use super::{BOLT_SPEED, Bolt, Colony, EntKind, Fx, INVENTORY, Id, PlayerKey, WeatherKind};
use crate::material::Material;
use crate::world::World;

type Outcome = Result<(), &'static str>;

/// Running state of a player's mods.
#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Gear {
    /// What each automated arm does.
    pub arm_tasks: [ArmTask; 2],
    /// Where the free-flying arm has been sent.
    pub free_arm: Option<V2>,
    pub shield: f32,
    /// Seconds until the shield starts recharging.
    pub shield_wait: f32,
    pub recall_wait: f32,
    pub rain_wait: f32,
    pub tickle_wait: f32,
    pub heal_wait: f32,
    pub turret_wait: f32,
    pub stomp_wait: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum Teleport {
    Home,
    Player(PlayerKey),
    Dome(Id),
}

/// A marker on the shared map.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Pin {
    pub id: u16,
    pub name: String,
    pub pos: V2,
    /// Team colour of whoever placed it.
    pub color: u8,
    /// Seconds left for a Beacon Rack ping; `None` for a lasting pin.
    pub ping: Option<f32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PinOp {
    Add {
        pos: V2,
        name: String,
    },
    Remove(u16),
    /// A Beacon Rack waypoint: everyone is told, and it fades by itself.
    Ping {
        pos: V2,
    },
}

/// Help a player is getting from teammates' mods right now.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Aid {
    /// 0 none, 1 half oxygen drain, 2 none, 3 refilled, 4 breathable.
    pub air: u8,
    pub warm: bool,
    pub heal: f32,
    pub shield: bool,
}

impl Colony {
    // ---- owning and equipping ----------------------------------------------

    /// Whether the colony can synthesize a set.
    pub fn knows_mod(&self, set: u8) -> bool {
        mods::STARTER_SETS.contains(&set) || self.blueprints.contains(&Blueprint::Mod(set))
    }

    /// What the next tier of a set would cost this player.
    pub fn mod_cost(&self, key: PlayerKey, tier: u8) -> Vec<(Item, u32)> {
        let discount = self.players.get(&key).map_or(0.0, |p| p.stats().craft_discount);
        mods::tier_cost(tier)
            .into_iter()
            .map(|(item, n)| (item, ((n as f32 * (1.0 - discount)).ceil() as u32).max(1)))
            .collect()
    }

    pub(super) fn craft_mod(&mut self, key: PlayerKey, set: u8) -> Outcome {
        if set as usize >= SETS {
            return Err("Unknown mod");
        }
        if !self.knows_mod(set) {
            return Err("Buy the blueprint from Earth first");
        }
        if !self.synthesizer_near(key) && !self.at_fixture(key, Fixture::ModBay) {
            return Err("Use a Synthesizer or the Mod Bay");
        }
        let owned = self.players[&key].mods.get(&set).copied().unwrap_or(0);
        if owned >= mods::LEGENDARY {
            return Err("Already Legendary");
        }
        let tier = owned + 1;
        let cost = self.mod_cost(key, tier);
        if cost
            .iter()
            .any(|&(item, n)| self.stock_count(key, Ingredient::Item(item)) < n)
        {
            return Err("Not enough materials");
        }
        for &(item, n) in &cost {
            self.stock_take(key, Ingredient::Item(item), n);
        }
        let info = &MOD_SETS[set as usize];
        let slot = info.slot as usize;
        let p = self.players.get_mut(&key).unwrap();
        p.mods.insert(set, tier);
        // Wear it if nothing else is in the slot.
        if p.equipped[slot].is_none() {
            p.equipped[slot] = Some(set);
        }
        let (name, at) = (p.name.clone(), p.center());
        self.refit(key);
        self.fx(Fx::ModCraft, at);
        self.toast(
            None,
            format!(
                "{name} synthesized {}: {}",
                info.name,
                info.tiers[tier as usize - 1].0
            ),
            true,
        );
        Ok(())
    }

    pub(super) fn equip(&mut self, key: PlayerKey, slot: Slot, set: Option<u8>) -> Outcome {
        if !self.at_fixture(key, Fixture::ModBay) {
            return Err("Mods are changed at the Mod Bay in the starter dome");
        }
        let p = self.players.get_mut(&key).unwrap();
        if let Some(set) = set {
            if !p.mods.contains_key(&set) {
                return Err("You haven't synthesized that set");
            }
            if MOD_SETS[set as usize].slot != slot {
                return Err("That set goes in another slot");
            }
        }
        p.equipped[slot as usize] = set;
        let at = p.center();
        self.refit(key);
        self.fx(Fx::ModEquip, at);
        Ok(())
    }

    /// Brings a player's inventory size and meters in line with their mods.
    pub(super) fn refit(&mut self, key: PlayerKey) {
        let Some(p) = self.players.get_mut(&key) else {
            return;
        };
        let stats = p.stats();
        let slots = INVENTORY + stats.extra_slots as usize;
        let spilled = if p.inv.slots.len() != slots {
            p.inv.resize(slots)
        } else {
            Vec::new()
        };
        p.hp = p.hp.min(stats.max_hp);
        p.o2 = p.o2.min(stats.o2_capacity);
        p.battery = p.battery.min(stats.battery_capacity);
        p.gear.shield = p.gear.shield.min(stats.shield);
        if stats.arms == 0 {
            p.gear.arm_tasks = [ArmTask::Off; 2];
        }
        if !stats.free_arm {
            p.gear.free_arm = None;
        }
        p.touch_public();
        let at = p.center();
        for s in spilled {
            self.drop_item(at, s.item, s.count);
        }
    }

    pub(super) fn set_arm_task(&mut self, key: PlayerKey, arm: u8, task: ArmTask) -> Outcome {
        let p = self.players.get_mut(&key).unwrap();
        if arm >= p.stats().arms {
            return Err("You don't have that arm");
        }
        p.gear.arm_tasks[arm as usize] = task;
        p.touch();
        Ok(())
    }

    pub(super) fn send_arm(&mut self, key: PlayerKey, at: Option<V2>) -> Outcome {
        let p = self.players.get_mut(&key).unwrap();
        if !p.stats().free_arm {
            return Err("That needs Armnipresence");
        }
        p.gear.free_arm = at;
        p.touch_public();
        Ok(())
    }

    // ---- abilities -----------------------------------------------------------

    /// A Stompers landing: digs a crater under the player.
    pub(super) fn stomp(&mut self, world: &mut World, key: PlayerKey, speed: f32) -> Outcome {
        let p = self.players.get_mut(&key).unwrap();
        let tier = p.stats().stomp;
        if tier == 0 || p.gear.stomp_wait > 0.0 || speed < 150.0 {
            return Ok(());
        }
        p.gear.stomp_wait = 0.4;
        let pos = p.pose.pos;
        let force = (speed / 300.0).clamp(0.6, 1.5);
        let radius = ([3.0, 5.0, 6.0, 9.0][tier as usize - 1] * force) as i32;
        let (x, y) = pos.cell();
        // Rock breaks by chance, so hit it a few times for a proper crater.
        for _ in 0..4 {
            let dug = world.dig(x, y + 2, radius, 200);
            self.collect_dug(key, &dug, pos);
        }
        if tier >= 2 {
            let damage = [0.0, 15.0, 25.0, 40.0][tier as usize - 1] * force;
            self.damage_all(pos, radius as f32 * 3.0, damage, Some(key));
        }
        if tier >= 4 {
            self.chill_near(pos, 70.0, 4.0);
        }
        self.fx(Fx::Stomp, pos);
        Ok(())
    }

    /// Hurts every creature within `radius`.
    fn damage_all(&mut self, at: V2, radius: f32, damage: f32, by: Option<PlayerKey>) {
        let hit: Vec<V2> = self
            .ents
            .values()
            .filter(|e| matches!(&e.kind, EntKind::Creature(c) if c.state != State::Dying))
            .map(|e| v2(e.pos.x, e.pos.y - 4.0))
            .filter(|p| p.distance(at) < radius)
            .collect();
        for p in hit {
            creatures::damage_at(self, p, 2.0, damage, by);
        }
    }

    /// Slows every creature within `radius` for `secs`.
    fn chill_near(&mut self, at: V2, radius: f32, secs: f32) {
        for e in self.ents.values_mut() {
            if let EntKind::Creature(c) = &mut e.kind
                && e.pos.distance(at) < radius
            {
                c.chilled = c.chilled.max(secs);
            }
        }
    }

    pub(super) fn teleport(&mut self, key: PlayerKey, to: Teleport) -> Outcome {
        let p = &self.players[&key];
        let tier = p.stats().beacon;
        if p.gear.recall_wait > 0.0 {
            return Err("The beacon is recharging");
        }
        let at = match to {
            Teleport::Home => {
                if tier < 2 {
                    return Err("That needs a Beacon Rack with Recall");
                }
                self.spawn_point(Some(key))
            }
            Teleport::Player(other) => {
                let o = self.players.get(&other).filter(|o| o.online && o.alive());
                let o = o.ok_or("They aren't around")?;
                if tier < 4 && o.stats().beacon < 3 {
                    return Err("They need a Beacon Rack with Rally, or you need Group Chat");
                }
                o.pose.pos
            }
            Teleport::Dome(id) => {
                if tier < 4 {
                    return Err("That needs Group Chat");
                }
                let (e, _) = self.dome(id).ok_or("That dome is gone")?;
                v2(e.pos.x, e.pos.y - 1.0)
            }
        };
        let p = self.players.get_mut(&key).unwrap();
        let from = p.center();
        p.pose.pos = at;
        p.pose.vel = V2::ZERO;
        p.warp += 1;
        p.gear.recall_wait = if tier >= 4 { 20.0 } else { 120.0 };
        p.touch_public();
        self.fx(Fx::Teleport, from);
        self.fx(Fx::Teleport, at);
        Ok(())
    }

    pub(super) fn rain_dance(&mut self, key: PlayerKey) -> Outcome {
        let p = self.players.get_mut(&key).unwrap();
        if !p.stats().rain_dance {
            return Err("That needs a Weather Vane with Rain Dance");
        }
        if p.gear.rain_wait > 0.0 {
            return Err("The clouds need a rest");
        }
        p.gear.rain_wait = 600.0;
        p.touch();
        let name = p.name.clone();
        self.weather.kind = WeatherKind::Rain;
        self.weather.remaining = 150.0;
        self.fx(Fx::Thunder, V2::ZERO);
        self.toast(None, format!("{name} danced. It rains."), true);
        Ok(())
    }

    /// Brings a blacked-out player round where they lie.
    fn revive(&mut self, key: PlayerKey) {
        let Some(p) = self.players.get_mut(&key) else {
            return;
        };
        if p.dead.is_none() {
            return;
        }
        let stats = p.stats();
        p.dead = None;
        p.hp = stats.max_hp * 0.4;
        p.o2 = p.o2.max(stats.o2_capacity * 0.5);
        p.warmth = p.warmth.max(50.0);
        p.touch_public();
        let (at, name) = (p.center(), p.name.clone());
        self.fx(Fx::Respawn, at);
        self.toast(None, format!("{name} was revived"), true);
    }

    pub(super) fn heal(&mut self, key: PlayerKey, target: PlayerKey) -> Outcome {
        let p = &self.players[&key];
        let (tier, reach, from) = (p.stats().heal_touch, p.stats().reach, p.center());
        if tier == 0 {
            return Err("That needs Healing Hands");
        }
        if p.gear.heal_wait > 0.0 {
            return Err("Your hands are recharging");
        }
        let range = match tier {
            1 | 2 => reach + 12.0,
            3 => 180.0,
            _ => f32::MAX,
        };
        let targets: Vec<PlayerKey> = if tier >= 4 {
            self.online().map(|o| o.key).collect()
        } else {
            vec![target]
        };
        let mut helped = false;
        for t in targets {
            let Some(o) = self.players.get_mut(&t).filter(|o| o.online) else {
                continue;
            };
            if o.center().distance(from) > range {
                continue;
            }
            if o.dead.is_some() {
                if tier >= 2 {
                    self.revive(t);
                    helped = true;
                }
            } else if t != key || tier >= 4 {
                let max = o.stats().max_hp;
                if o.hp < max {
                    o.hp = (o.hp + 35.0).min(max);
                    let at = o.center();
                    self.fx(Fx::Heal, at);
                    helped = true;
                }
            }
        }
        if !helped {
            return Err(if tier >= 2 {
                "Nobody in reach needs help"
            } else {
                "Nobody in reach is hurt"
            });
        }
        let p = self.players.get_mut(&key).unwrap();
        p.gear.heal_wait = 6.0;
        Ok(())
    }

    /// Green Fingers: hurries a plant along.
    pub(super) fn tickle(&mut self, key: PlayerKey, planter: Option<(Id, u8)>, plant: Option<Id>) -> Outcome {
        let p = &self.players[&key];
        let (tier, reach, from) = (p.stats().tickle, p.stats().reach, p.center());
        if tier == 0 {
            return Err("That needs Green Fingers");
        }
        if p.gear.tickle_wait > 0.0 {
            return Err("Your fingers are tired");
        }
        let grow = |g: &mut f32| *g = if tier >= 3 { 1.0 } else { (*g + 0.25).min(1.0) };
        let at;
        if let Some((dome, slot)) = planter {
            let (e, d) = self.dome(dome).ok_or("That dome is gone")?;
            let f = d
                .kind
                .fixtures()
                .iter()
                .find(|f| f.fixture == Fixture::Planter(slot));
            at = fixture_pos(e.pos, f.ok_or("No such planter")?);
            if at.distance(from) > reach + 16.0 {
                return Err("Too far away");
            }
            let pl = self.dome_mut(dome).unwrap().planters.get_mut(slot as usize);
            let pl = pl.filter(|pl| pl.species.is_some() && pl.growth < 1.0);
            grow(&mut pl.ok_or("Nothing here is still growing")?.growth);
        } else if let Some(id) = plant {
            let e = self.ents.get_mut(&id).ok_or("It's gone")?;
            at = e.pos;
            if at.distance(from) > reach + 16.0 {
                return Err("Too far away");
            }
            let EntKind::Plant(pl) = &mut e.kind else {
                return Err("That isn't a plant");
            };
            if pl.growth >= 1.0 {
                return Err("It's fully grown");
            }
            grow(&mut pl.growth);
            self.touch(id);
        } else {
            return Err("Point at a plant");
        }
        let p = self.players.get_mut(&key).unwrap();
        p.gear.tickle_wait = [45.0, 20.0, 20.0][tier as usize - 1];
        self.fx(Fx::Fertilize, at);
        Ok(())
    }

    /// Grapple Glove II: drops near the hook come back with it.
    pub(super) fn yank(&mut self, key: PlayerKey, at: V2) -> Outcome {
        let p = &self.players[&key];
        let stats = p.stats();
        let to = p.center();
        if !stats.grapple_pull || at.distance(to) > stats.grapple + 20.0 {
            return Ok(());
        }
        let ids: Vec<Id> = self
            .ents
            .values()
            .filter(|e| matches!(e.kind, EntKind::Drop(_)) && e.pos.distance(at) < 16.0)
            .map(|e| e.id)
            .collect();
        for id in ids {
            if let Some(e) = self.ents.get_mut(&id) {
                e.pos = to;
            }
            self.touch(id);
        }
        Ok(())
    }

    // ---- pins ------------------------------------------------------------------

    pub(super) fn pin_op(&mut self, key: PlayerKey, op: PinOp) -> Outcome {
        let p = &self.players[&key];
        let (color, who, beacon) = (p.color, p.name.clone(), p.stats().beacon);
        match op {
            PinOp::Add { pos, name } => {
                if self.pins.iter().filter(|p| p.ping.is_none()).count() >= 40 {
                    return Err("Too many pins: remove one first");
                }
                let name: String = name.chars().filter(|c| !c.is_control()).take(24).collect();
                let id = self.next_pin();
                let name = if name.trim().is_empty() {
                    format!("Pin {}", id + 1)
                } else {
                    name
                };
                self.pins.push(Pin {
                    id,
                    name,
                    pos,
                    color,
                    ping: None,
                });
            }
            PinOp::Remove(id) => self.pins.retain(|p| p.id != id),
            PinOp::Ping { pos } => {
                if beacon == 0 {
                    return Err("That needs a Beacon Rack");
                }
                // One ping per player at a time.
                let name = format!("{who}'s ping");
                self.pins.retain(|p| p.ping.is_none() || p.name != name);
                let id = self.next_pin();
                self.pins.push(Pin {
                    id,
                    name,
                    pos,
                    color,
                    ping: Some(120.0),
                });
                self.fx(Fx::Ping, pos);
                self.toast(None, format!("{who} pinged a spot on the map"), true);
            }
        }
        self.meta_rev += 1;
        Ok(())
    }

    fn next_pin(&self) -> u16 {
        (0..u16::MAX)
            .find(|i| self.pins.iter().all(|p| p.id != *i))
            .unwrap_or(0)
    }

    // ---- gene splicing -----------------------------------------------------------

    /// The hybrid two species would make, if the colony has already made it.
    pub fn hybrid_of(&self, a: SpeciesId, b: SpeciesId) -> Option<SpeciesId> {
        let pair = (a.min(b), a.max(b));
        self.species
            .iter()
            .find(|s| s.parents == Some(pair))
            .map(|s| s.id)
    }

    pub(super) fn splice(&mut self, key: PlayerKey, machine: Id, a: SpeciesId, b: SpeciesId) -> Outcome {
        let e = self.ents.get(&machine).ok_or("That machine is gone")?;
        let EntKind::Machine(m) = &e.kind else {
            return Err("That isn't a Gene Splicer");
        };
        if m.kind != MachineKind::Splicer {
            return Err("That isn't a Gene Splicer");
        }
        let p = &self.players[&key];
        if e.pos.distance(p.pose.pos) > p.stats().reach + 16.0 {
            return Err("Too far away");
        }
        if !m.on {
            return Err("The Gene Splicer has no power");
        }
        if a == b {
            return Err("Choose two different species");
        }
        let (Some(sa), Some(sb)) = (self.species.get(a as usize), self.species.get(b as usize)) else {
            return Err("Unknown species");
        };
        if p.inv.count(Item::Seed(a)) == 0 || p.inv.count(Item::Seed(b)) == 0 {
            return Err("You need a seed of each species");
        }
        let bonus = p.stats().splice_bonus;
        let at = e.pos;
        let id = match self.hybrid_of(a, b) {
            Some(id) => id,
            None => {
                if self.species.len() >= 4000 {
                    return Err("The gene bank is full");
                }
                let mut child = splice(self.seed, sa, sb, bonus, &self.species);
                child.id = self.species.len() as SpeciesId;
                let id = child.id;
                self.toast(None, format!("New hybrid: {}", child.name), true);
                self.species.push(child);
                self.meta_rev += 1;
                id
            }
        };
        let p = self.players.get_mut(&key).unwrap();
        p.inv.take(Item::Seed(a), 1);
        p.inv.take(Item::Seed(b), 1);
        p.touch();
        self.give(key, Item::Seed(id), 2);
        self.discover(id);
        self.fx(Fx::Splice, at);
        Ok(())
    }

    // ---- what mods do by themselves --------------------------------------------

    /// Help each online player is getting from teammates.
    pub(super) fn aid(&self) -> BTreeMap<PlayerKey, Aid> {
        let givers: Vec<(PlayerKey, V2, mods::ModStats)> = self
            .online()
            .filter(|p| p.alive())
            .map(|p| (p.key, p.center(), p.stats()))
            .collect();
        let team_regen: f32 = givers.iter().map(|g| g.2.team_regen).fold(0.0, f32::max);
        self.online()
            .map(|p| {
                let mut aid = Aid {
                    heal: team_regen,
                    ..Aid::default()
                };
                for (key, at, s) in &givers {
                    let d = at.distance(p.center());
                    let me = *key == p.key;
                    // "Atmosphere Subscription" covers its wearer too.
                    if d < 56.0 && s.air_aura > 0 && (!me || s.air_aura >= 4) {
                        aid.air = aid.air.max(s.air_aura);
                    }
                    if me {
                        continue;
                    }
                    aid.warm |= s.warm_aura && d < 60.0;
                    if d < 60.0 {
                        aid.heal += s.heal_aura;
                    }
                    aid.shield |= s.shield_aura && d < 56.0;
                }
                (p.key, aid)
            })
            .collect()
    }

    /// Entities a Reactor Heart is powering.
    pub(super) fn reactor_powered(&self) -> Vec<Id> {
        let hearts: Vec<(V2, f32)> = self
            .online()
            .filter(|p| p.alive())
            .filter_map(|p| {
                let r = match p.stats().reactor {
                    2 => 70.0,
                    3 => 140.0,
                    4 => 320.0,
                    _ => return None,
                };
                Some((p.center(), r))
            })
            .collect();
        if hearts.is_empty() {
            return Vec::new();
        }
        self.ents
            .values()
            .filter(|e| matches!(e.kind, EntKind::Dome(_) | EntKind::Machine(_)))
            .filter(|e| hearts.iter().any(|(at, r)| e.pos.distance(*at) < *r))
            .map(|e| e.id)
            .collect()
    }

    /// How much faster a robot at `pos` works thanks to Hive Minds, and
    /// whether it runs for free.
    pub(super) fn robot_boost(&self, pos: V2) -> (f32, bool) {
        let mut best = (1.0f32, false);
        for p in self.online().filter(|p| p.alive()) {
            let s = p.stats();
            if s.robot_all {
                return (2.0, true);
            }
            if s.robot_speed > 1.0 && p.center().distance(pos) < 90.0 {
                best.0 = best.0.max(s.robot_speed);
            }
        }
        best
    }

    /// Planters within `radius` of a point: (dome, slot, position).
    fn planters_near(&self, at: V2, radius: f32) -> Vec<(Id, u8, V2)> {
        let mut out = Vec::new();
        for (e, d) in self.domes() {
            if d.planters.is_empty() || e.pos.distance(at) > radius + 140.0 {
                continue;
            }
            for f in d.kind.fixtures() {
                if let Fixture::Planter(i) = f.fixture {
                    let p = fixture_pos(e.pos, f);
                    if p.distance(at) <= radius {
                        out.push((e.id, i, p));
                    }
                }
            }
        }
        out
    }

    /// Harvests one ripe planter or plant near a point for a player.
    fn auto_harvest(&mut self, key: PlayerKey, at: V2, radius: f32) -> bool {
        for (dome, slot, pos) in self.planters_near(at, radius) {
            let pl = self.dome(dome).unwrap().1.planters[slot as usize];
            let Some(sp) = pl.species.and_then(|s| self.species.get(s as usize)) else {
                continue;
            };
            if pl.growth < 1.0 || sp.product.is_none() {
                continue;
            }
            let sp = sp.clone();
            for (item, n) in harvest_items(&sp, 1.0, &mut self.rng) {
                self.give(key, item, n);
            }
            self.dome_mut(dome).unwrap().planters[slot as usize].growth = REGROW;
            self.fx(Fx::Harvest, pos);
            return true;
        }
        let plant = self
            .ents
            .values()
            .find(|e| matches!(e.kind, EntKind::Plant(p) if p.growth >= 1.0) && e.pos.distance(at) <= radius)
            .map(|e| e.id);
        match plant {
            Some(id) => self.harvest_plant_for(key, id, 1.0).is_ok(),
            None => false,
        }
    }

    /// Waters every thirsty planter near a point. Returns how many.
    fn auto_water(&mut self, at: V2, radius: f32) -> usize {
        let mut n = 0;
        for (dome, slot, pos) in self.planters_near(at, radius) {
            let pl = self.dome(dome).unwrap().1.planters[slot as usize];
            let thirsty = pl
                .species
                .and_then(|s| self.species.get(s as usize))
                .is_some_and(|s| s.thirst_per_day() > 0.0);
            if thirsty && pl.moisture < 0.7 {
                self.dome_mut(dome).unwrap().planters[slot as usize].moisture = 1.0;
                self.fx(Fx::Water, pos);
                n += 1;
            }
        }
        n
    }

    /// Sows the player's seeds into empty planters near a point.
    fn auto_plant(&mut self, key: PlayerKey, at: V2, radius: f32) {
        for (dome, slot, pos) in self.planters_near(at, radius) {
            let (_, d) = self.dome(dome).unwrap();
            if d.planters[slot as usize].species.is_some() {
                continue;
            }
            let bed = d.kind.bed();
            let seed = self.players[&key].inv.stacks().find_map(|s| match s.item {
                Item::Seed(sp) if self.species.get(sp as usize).is_some_and(|x| x.fit(bed) > 0.0) => Some(sp),
                _ => None,
            });
            let Some(sp) = seed else { continue };
            let p = self.players.get_mut(&key).unwrap();
            p.inv.take(Item::Seed(sp), 1);
            p.touch();
            let pl = &mut self.dome_mut(dome).unwrap().planters[slot as usize];
            pl.species = Some(sp);
            pl.growth = 0.0;
            self.discover(sp);
            self.fx(Fx::Sow, pos);
        }
    }

    /// Picks up to `max` drops near a point straight into a player's pack.
    fn auto_collect(&mut self, key: PlayerKey, at: V2, radius: f32, max: usize) -> usize {
        let drops: Vec<(Id, Item, u32)> = self
            .ents
            .values()
            .filter(|e| e.pos.distance(at) <= radius)
            .filter_map(|e| match &e.kind {
                EntKind::Drop(d) if d.age > 0.6 => Some((e.id, d.item, d.count)),
                _ => None,
            })
            .take(max)
            .collect();
        let n = drops.len();
        for (id, item, count) in drops {
            let p = self.players.get_mut(&key).unwrap();
            let mult = p.stats().stack_mult;
            let left = p.inv.add(item, count, mult);
            p.touch();
            let to = p.center();
            if left == 0 {
                self.despawn(id);
                self.fx(Fx::Pickup, to);
            } else if let Some(EntKind::Drop(d)) = self.ents.get_mut(&id).map(|e| &mut e.kind) {
                d.count = left;
                self.touch(id);
            }
        }
        n
    }

    /// Digs at the nearest ore within `radius` of a point for a player.
    fn auto_mine(&mut self, world: &mut World, key: PlayerKey, at: V2, radius: i32) -> bool {
        let (cx, cy) = at.cell();
        let mut best: Option<((i32, i32), i32)> = None;
        let mut y = cy - radius;
        while y <= cy + radius {
            let mut x = cx - radius;
            while x <= cx + radius {
                let d = (x - cx) * (x - cx) + (y - cy) * (y - cy);
                if d <= radius * radius && world.material(x, y).is_ore() && best.is_none_or(|(_, b)| d < b) {
                    best = Some(((x, y), d));
                }
                x += 2;
            }
            y += 2;
        }
        let Some(((x, y), _)) = best else { return false };
        let dug = world.dig(x, y, 3, 110);
        self.collect_dug(key, &dug, v2(x as f32, y as f32));
        true
    }

    /// Mod effects that run every tick: cooldowns, shields and turrets.
    pub(super) fn step_mods(&mut self, world: &World, dt: f32) {
        let keys: Vec<PlayerKey> = self.online().filter(|p| p.alive()).map(|p| p.key).collect();
        for key in keys {
            let p = self.players.get_mut(&key).unwrap();
            let stats = p.stats();
            let g = &mut p.gear;
            for wait in [
                &mut g.recall_wait,
                &mut g.rain_wait,
                &mut g.tickle_wait,
                &mut g.heal_wait,
                &mut g.turret_wait,
                &mut g.stomp_wait,
                &mut g.shield_wait,
            ] {
                *wait = (*wait - dt).max(0.0);
            }
            if g.shield_wait <= 0.0 && g.shield < stats.shield {
                g.shield = (g.shield + stats.shield / 3.0 * dt).min(stats.shield);
            }
            if stats.turret > 0 && g.turret_wait <= 0.0 && p.in_dome.is_none() {
                let from = v2(p.pose.pos.x, p.pose.pos.y - 20.0);
                let mut targets: Vec<(f32, V2)> = self
                    .ents
                    .values()
                    .filter_map(|e| match &e.kind {
                        EntKind::Creature(c)
                            if c.kind.predator() && !matches!(c.state, State::Dying | State::Hidden) =>
                        {
                            let at = v2(e.pos.x, e.pos.y - c.kind.size().1 / 2.0);
                            Some((at.distance(from), at))
                        }
                        _ => None,
                    })
                    .filter(|(d, at)| *d < 130.0 && line_of_sight(world, from, *at, player_solid))
                    .collect();
                targets.sort_by(|a, b| a.0.total_cmp(&b.0));
                targets.truncate(if stats.turret >= 3 { 3 } else { 1 });
                if !targets.is_empty() {
                    self.players.get_mut(&key).unwrap().gear.turret_wait =
                        [1.4, 0.7, 0.7, 0.5][stats.turret as usize - 1];
                    self.fx(Fx::BoltFire, from);
                }
                for (_, at) in targets {
                    if stats.turret >= 4 {
                        // Never misses: the shot lands at once.
                        creatures::damage_at(self, at, 3.0, 12.0, Some(key));
                        self.fx(Fx::BoltHit, at);
                    } else {
                        self.spawn(
                            from,
                            EntKind::Bolt(Bolt {
                                vel: (at - from).normalized() * BOLT_SPEED,
                                damage: 9.0,
                                size: 0.8,
                                owner: Some(key),
                                life: 0.8,
                                web: false,
                            }),
                        );
                    }
                }
            }
        }
        // Pings fade.
        let before = self.pins.len();
        for pin in &mut self.pins {
            if let Some(t) = &mut pin.ping {
                *t -= dt;
            }
        }
        self.pins.retain(|p| p.ping.is_none_or(|t| t > 0.0));
        if self.pins.len() != before {
            self.meta_rev += 1;
        }
    }

    /// Mod effects that run once a second.
    pub(super) fn step_mods_second(&mut self, world: &mut World) {
        let keys: Vec<PlayerKey> = self.online().filter(|p| p.alive()).map(|p| p.key).collect();
        let mut drizzle = false;
        for key in keys {
            let p = &self.players[&key];
            let (stats, at, feet, gear, in_dome) = (p.stats(), p.center(), p.pose.pos, p.gear, p.in_dome);
            drizzle |= stats.drizzle;

            // Root Walkers.
            if stats.root_walk >= 1 {
                let (x, y) = feet.cell();
                for dx in -5..=5 {
                    for dy in 0..=3 {
                        if world.material(x + dx, y + dy) == Material::Dirt
                            && !player_solid(world.material(x + dx, y + dy - 1))
                        {
                            world.set(x + dx, y + dy, Material::Grass);
                        }
                    }
                }
            }
            if stats.root_walk >= 3 {
                while self.auto_harvest(key, at, 20.0) {}
            }
            if stats.root_walk >= 2 {
                self.auto_plant(key, at, 20.0);
            }
            if stats.root_walk >= 4 {
                self.auto_water(at, 20.0);
            }

            // Sprinkler Pack.
            if stats.sprinkler > 0.0 {
                self.auto_water(at, stats.sprinkler);
            }

            // Plants near the player grow faster.
            if stats.grow_aura > 1.0 {
                let extra = stats.grow_aura - 1.0;
                for (dome, slot, _) in self.planters_near(at, 60.0) {
                    let (_, d) = self.dome(dome).unwrap();
                    let pl = d.planters[slot as usize];
                    let Some(sp) = pl.species.and_then(|s| self.species.get(s as usize)) else {
                        continue;
                    };
                    if pl.moisture <= 0.0 || pl.growth >= 1.0 {
                        continue;
                    }
                    let add = sp.growth_rate() * sp.fit(d.kind.bed()) * extra;
                    let pl = &mut self.dome_mut(dome).unwrap().planters[slot as usize];
                    pl.growth = (pl.growth + add).min(1.0);
                }
                let species = &self.species;
                let mut grown = Vec::new();
                for e in self.ents.values_mut() {
                    if let EntKind::Plant(pl) = &mut e.kind
                        && pl.growth < 1.0
                        && e.pos.distance(at) < 60.0
                        && let Some(sp) = species.get(pl.species as usize)
                    {
                        let before = (pl.growth * 8.0) as u32;
                        pl.growth = (pl.growth + sp.growth_rate() * extra).min(1.0);
                        if (pl.growth * 8.0) as u32 != before {
                            grown.push(e.id);
                        }
                    }
                }
                for id in grown {
                    self.touch(id);
                }
            }

            // Filter Lungs clear spore gas.
            if stats.gas_clear > 0 {
                let (x, y) = at.cell();
                let mut cleared = 0;
                for dy in -16..=16 {
                    for dx in -16..=16 {
                        if dx * dx + dy * dy <= 256 && world.material(x + dx, y + dy) == Material::Spore {
                            world.set(x + dx, y + dy, Material::Empty);
                            cleared += 1;
                        }
                    }
                }
                if stats.gas_clear >= 2 {
                    self.atmosphere.o2 = (self.atmosphere.o2 + cleared as f32 * 0.00002).min(21.0);
                }
            }

            // Reactor Heart charges pylons.
            if stats.reactor >= 1 {
                let near: Vec<Id> = self
                    .ents
                    .values()
                    .filter(|e| {
                        matches!(&e.kind, EntKind::Machine(m) if m.kind == MachineKind::Pylon)
                            && e.pos.distance(at) < 70.0
                    })
                    .map(|e| e.id)
                    .collect();
                for id in near {
                    if let Some(EntKind::Machine(m)) = self.ents.get_mut(&id).map(|e| &mut e.kind)
                        && m.store < PYLON_BUFFER
                    {
                        m.store = (m.store + 0.4).min(PYLON_BUFFER);
                    }
                }
            }

            // Medi-Core revives teammates who fell nearby.
            if stats.revive_aura {
                let down: Vec<PlayerKey> = self
                    .online()
                    .filter(|o| o.dead.is_some() && o.center().distance(at) < 50.0)
                    .map(|o| o.key)
                    .collect();
                for k in down {
                    self.revive(k);
                }
            }

            // Drones fetch drops and mine ore.
            for _ in 0..stats.drones {
                if self.auto_collect(key, at, 120.0, 2) == 0 && stats.drone_mines {
                    self.auto_mine(world, key, at, 40);
                }
            }

            // Automated arms.
            let reach = stats.reach;
            for task in gear.arm_tasks.iter().take(stats.arms as usize) {
                self.arm_work(world, key, *task, at, reach);
            }
            // The free arm does a bit of everything where it was sent.
            if let Some(spot) = gear.free_arm.filter(|_| stats.free_arm) {
                for task in [ArmTask::Collect, ArmTask::Harvest, ArmTask::Water, ArmTask::Mine] {
                    self.arm_work(world, key, task, spot, 40.0);
                }
            }

            // Dome Brain hurries Dome Domes along.
            if stats.dome_dome_speed > 1.0 {
                let near: Vec<Id> = self
                    .domes()
                    .filter(|(e, d)| d.kind == DomeKind::DomeDome && e.pos.distance(at) < 120.0)
                    .map(|(e, _)| e.id)
                    .collect();
                for id in near {
                    self.hurry_dome_dome(id, stats.dome_dome_speed - 1.0);
                }
            }

            // Cryo Palms' aura, and predators fleeing Camo Skin.
            if stats.cryo >= 4 {
                self.chill_near(at, 48.0, 2.0);
            }
            if stats.stealth >= 4 {
                for e in self.ents.values_mut() {
                    if let EntKind::Creature(c) = &mut e.kind
                        && c.kind.predator()
                        && e.pos.distance(at) < 70.0
                    {
                        c.target = None;
                        c.vel.x = (e.pos.x - at.x).signum() * 70.0;
                        c.facing = if e.pos.x >= at.x { 1 } else { -1 };
                    }
                }
            }

            // Pack Mule quick-stacks when its wearer steps into a dome.
            let _ = in_dome;
        }

        // Cloud Computing: every dome's water tops itself up.
        if drizzle {
            let ids: Vec<Id> = self.domes().map(|(e, _)| e.id).collect();
            for id in ids {
                let d = self.dome_mut(id).unwrap();
                if d.kind.water_per_day() > 0.0 {
                    let cap = d.reserve_cap();
                    d.reserve = (d.reserve + d.kind.water_per_day() / super::DAY_SECS * 1.5).min(cap);
                }
            }
        }
    }

    fn arm_work(&mut self, world: &mut World, key: PlayerKey, task: ArmTask, at: V2, reach: f32) {
        match task {
            ArmTask::Off => {}
            ArmTask::Mine => {
                if self.auto_mine(world, key, at, reach as i32) {
                    self.auto_collect(key, at, reach + 12.0, 4);
                }
            }
            ArmTask::Water => {
                self.auto_water(at, reach);
            }
            ArmTask::Harvest => {
                self.auto_harvest(key, at, reach);
            }
            ArmTask::Collect => {
                self.auto_collect(key, at, reach + 12.0, 4);
            }
            ArmTask::Build => {
                let near: Vec<Id> = self
                    .domes()
                    .filter(|(e, d)| d.kind == DomeKind::DomeDome && e.pos.distance(at) < reach + 80.0)
                    .map(|(e, _)| e.id)
                    .collect();
                for id in near {
                    self.hurry_dome_dome(id, 0.5);
                }
            }
        }
    }

    /// Adds `extra` seconds' worth of work to a running Dome Dome.
    fn hurry_dome_dome(&mut self, id: Id, extra: f32) {
        let Some(d) = self.dome_mut(id) else { return };
        if let Some(target) = d.target
            && d.powered
            && d.progress > 0.0
        {
            let days = match target.size().0 {
                0..=80 => 1.0,
                81..=110 => 2.0,
                _ => 3.0,
            };
            d.progress = (d.progress + extra / (days * super::DAY_SECS)).min(0.999);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colony::actions::Action;
    use crate::colony::creatures::{Creature, CreatureKind};
    use crate::colony::mods::set;
    use crate::colony::testing::colony;
    use crate::colony::{Machine, pose_flag};

    fn stock(colony: &mut Colony, key: PlayerKey) {
        let p = colony.players.get_mut(&key).unwrap();
        for item in [
            Item::Iron,
            Item::Copper,
            Item::Silicon,
            Item::Gold,
            Item::Titanium,
            Item::Xenite,
            Item::Aurorium,
        ] {
            p.inv.add(item, 400, 1.0);
        }
    }

    /// Stands a player at the starter dome's Mod Bay.
    fn at_mod_bay(colony: &mut Colony, key: PlayerKey) {
        let home = colony.home;
        let (e, d) = colony.dome(home).unwrap();
        let f = d
            .kind
            .fixtures()
            .iter()
            .find(|f| f.fixture == Fixture::ModBay)
            .unwrap();
        let at = fixture_pos(e.pos, f);
        let p = colony.players.get_mut(&key).unwrap();
        p.pose.pos = at;
        p.in_dome = Some(home);
    }

    /// Gives a player a set at a tier, equipped.
    fn wear(colony: &mut Colony, key: PlayerKey, set: u8, tier: u8) {
        let p = colony.players.get_mut(&key).unwrap();
        p.mods.insert(set, tier);
        p.equipped[MOD_SETS[set as usize].slot as usize] = Some(set);
        colony.refit(key);
    }

    #[test]
    fn mods_are_synthesized_tier_by_tier_and_equipped_at_the_mod_bay() {
        let (mut world, mut colony, key) = colony();
        at_mod_bay(&mut colony, key);
        // No materials yet.
        assert_eq!(
            colony.apply(
                &mut world,
                key,
                Action::CraftMod {
                    set: set::ROCKET_FEET
                }
            ),
            Err("Not enough materials")
        );
        stock(&mut colony, key);
        // A set Earth hasn't sent the blueprint for.
        assert_eq!(
            colony.apply(
                &mut world,
                key,
                Action::CraftMod {
                    set: set::GLIDE_WINGS
                }
            ),
            Err("Buy the blueprint from Earth first")
        );
        colony.blueprints.insert(Blueprint::Mod(set::GLIDE_WINGS));
        colony
            .apply(
                &mut world,
                key,
                Action::CraftMod {
                    set: set::GLIDE_WINGS,
                },
            )
            .unwrap();
        let iron = colony.players[&key].inv.count(Item::Iron);
        for tier in 1..=4 {
            colony
                .apply(
                    &mut world,
                    key,
                    Action::CraftMod {
                        set: set::ROCKET_FEET,
                    },
                )
                .unwrap();
            assert_eq!(colony.players[&key].mods[&set::ROCKET_FEET], tier);
        }
        assert_eq!(colony.players[&key].inv.count(Item::Iron), iron - 12 - 24 - 40);
        assert_eq!(
            colony.apply(
                &mut world,
                key,
                Action::CraftMod {
                    set: set::ROCKET_FEET
                }
            ),
            Err("Already Legendary")
        );
        // First set in a slot is worn at once and shows on the sprite.
        let p = &colony.players[&key];
        assert_eq!(p.equipped[Slot::Feet as usize], Some(set::ROCKET_FEET));
        assert!(p.stats().thrust_secs.is_infinite());
        assert_eq!(p.public().mods[Slot::Feet as usize], Some((set::ROCKET_FEET, 4)));
        assert_eq!(p.public().mods[Slot::Back as usize], Some((set::GLIDE_WINGS, 1)));
        // One set per slot: swapping happens at the Mod Bay.
        colony
            .apply(
                &mut world,
                key,
                Action::CraftMod {
                    set: set::SPRING_HEELS,
                },
            )
            .unwrap_err();
        colony.blueprints.insert(Blueprint::Mod(set::SPRING_HEELS));
        colony
            .apply(
                &mut world,
                key,
                Action::CraftMod {
                    set: set::SPRING_HEELS,
                },
            )
            .unwrap();
        assert_eq!(
            colony.players[&key].equipped[Slot::Feet as usize],
            Some(set::ROCKET_FEET)
        );
        let equip = Action::Equip {
            slot: Slot::Feet,
            set: Some(set::SPRING_HEELS),
        };
        colony.apply(&mut world, key, equip.clone()).unwrap();
        let s = colony.players[&key].stats();
        assert!(s.jump > 1.0 && s.air_jumps == 0);
        assert_eq!(
            colony.apply(
                &mut world,
                key,
                Action::Equip {
                    slot: Slot::Head,
                    set: Some(set::SPRING_HEELS)
                }
            ),
            Err("That set goes in another slot")
        );
        // Away from the Mod Bay it can't be changed.
        colony.players.get_mut(&key).unwrap().pose.pos.x += 400.0;
        colony.players.get_mut(&key).unwrap().in_dome = None;
        assert!(colony.apply(&mut world, key, equip).is_err());
    }

    #[test]
    fn cargo_pants_add_slots_and_taking_them_off_spills_the_extra() {
        let (_, mut colony, key) = colony();
        wear(&mut colony, key, set::CARGO_PANTS, 2);
        let p = colony.players.get_mut(&key).unwrap();
        assert_eq!(p.inv.slots.len(), INVENTORY + 20);
        p.inv.slots[INVENTORY + 5] = Some(crate::colony::inventory::Stack::new(Item::Gold, 7));
        p.equipped[Slot::Legs as usize] = None;
        colony.refit(key);
        assert_eq!(colony.players[&key].inv.slots.len(), INVENTORY);
        let dropped = colony
            .ents
            .values()
            .any(|e| matches!(&e.kind, EntKind::Drop(d) if d.item == Item::Gold && d.count == 7));
        assert!(dropped || colony.players[&key].inv.count(Item::Gold) == 7);
    }

    #[test]
    fn survival_mods_change_what_hurts() {
        let (mut world, mut colony, key) = colony();
        // Plating and shock absorbers.
        wear(&mut colony, key, set::PLATING, 2);
        wear(&mut colony, key, set::SHOCK_ABSORBERS, 1);
        let p = colony.players.get_mut(&key).unwrap();
        assert_eq!(p.stats().max_hp, 150.0);
        p.hp = 150.0;
        colony.hurt(key, 20.0);
        assert!((colony.players[&key].hp - 133.0).abs() < 0.01);
        // A shield soaks damage first, then recharges.
        wear(&mut colony, key, set::SHIELD_ARM, 1);
        colony.players.get_mut(&key).unwrap().gear.shield = 20.0;
        colony.hurt(key, 10.0);
        assert!((colony.players[&key].hp - 133.0).abs() < 0.01);
        assert!((colony.players[&key].gear.shield - 11.5).abs() < 0.01);
        colony.hurt(key, 20.0);
        let hp = colony.players[&key].hp;
        assert!(hp < 133.0 && hp > 120.0, "{hp}");
        for _ in 0..600 {
            colony.step_mods(&world, 1.0 / 60.0);
        }
        assert!(colony.players[&key].gear.shield > 10.0, "recharges after a pause");
        // Phase Dash: nothing lands mid-dash.
        wear(&mut colony, key, set::DASH_PISTONS, 3);
        colony.players.get_mut(&key).unwrap().pose.flags |= pose_flag::DASH;
        let hp = colony.players[&key].hp;
        colony.players.get_mut(&key).unwrap().gear.shield = 0.0;
        colony.hurt(key, 30.0);
        assert_eq!(colony.players[&key].hp, hp);
        let _ = &mut world;
    }

    #[test]
    fn turret_and_stomp_fight_for_you() {
        let (mut world, mut colony, key) = colony();
        let p = colony.players.get_mut(&key).unwrap();
        // Out in the open, away from the dome.
        p.pose.pos = v2(p.pose.pos.x + 260.0, 200.0);
        p.in_dome = None;
        let at = p.pose.pos;
        for y in 100..230 {
            for x in (at.x as i32 - 150)..(at.x as i32 + 150) {
                world.set(x, y, if y > 200 { Material::Stone } else { Material::Empty });
            }
        }
        wear(&mut colony, key, set::SHOULDER_TURRET, 4);
        let skitter = colony.spawn(
            v2(at.x + 60.0, 200.0),
            EntKind::Creature(Creature::new(CreatureKind::Skitter)),
        );
        for _ in 0..240 {
            colony.step_mods(&world, 1.0 / 60.0);
        }
        let EntKind::Creature(c) = &colony.ents[&skitter].kind else {
            panic!()
        };
        assert_eq!(c.state, State::Dying, "Pew Infinity never misses");

        wear(&mut colony, key, set::STOMPERS, 2);
        let maw = colony.spawn(
            v2(at.x + 8.0, 200.0),
            EntKind::Creature(Creature::new(CreatureKind::Skitter)),
        );
        colony
            .apply(&mut world, key, Action::Stomp { speed: 320.0 })
            .unwrap();
        let (x, y) = at.cell();
        let dug = (1..6)
            .filter(|dy| world.material(x, y + dy) == Material::Empty)
            .count();
        assert!(dug >= 3, "a crater: {dug} cells deep");
        let EntKind::Creature(c) = &colony.ents[&maw].kind else {
            panic!()
        };
        assert!(c.hp < CreatureKind::Skitter.max_hp());
    }

    #[test]
    fn helper_mods_reach_teammates() {
        let (mut world, mut colony, key) = colony();
        colony.join(2, "Bo".into());
        let at = colony.players[&key].pose.pos;
        colony.players.get_mut(&2).unwrap().pose.pos = at;
        // Buddy Breather, then Medi-Core in the same slot.
        wear(&mut colony, key, set::BUDDY_BREATHER, 2);
        let aid = colony.aid();
        assert_eq!(aid[&2].air, 2);
        assert_eq!(aid[&key].air, 0, "Air Supply is for teammates");
        wear(&mut colony, key, set::MEDI_CORE, 3);
        let aid = colony.aid();
        assert_eq!(aid[&2].air, 0, "one set per slot");
        assert!(aid[&2].heal > 0.0);
        // Defib brings a fallen teammate round where they lie.
        colony.players.get_mut(&2).unwrap().dead = Some(5.0);
        colony.step_mods_second(&mut world);
        assert!(colony.players[&2].alive());
        // Healing Hands.
        wear(&mut colony, key, set::HEALING_HANDS, 1);
        colony.players.get_mut(&2).unwrap().hp = 20.0;
        colony.apply(&mut world, key, Action::Heal { target: 2 }).unwrap();
        assert_eq!(colony.players[&2].hp, 55.0);
        assert_eq!(
            colony.apply(&mut world, key, Action::Heal { target: 2 }),
            Err("Your hands are recharging")
        );
        // Rally: Bo can jump to Ada once she carries the beacon.
        colony.players.get_mut(&2).unwrap().pose.pos.x += 500.0;
        assert!(
            colony
                .apply(&mut world, 2, Action::Teleport(Teleport::Player(key)))
                .is_err()
        );
        wear(&mut colony, key, set::BEACON_RACK, 3);
        colony
            .apply(&mut world, 2, Action::Teleport(Teleport::Player(key)))
            .unwrap();
        assert_eq!(colony.players[&2].pose.pos, at);
        // Ada herself can recall home, then has to wait.
        colony.players.get_mut(&key).unwrap().pose.pos.x += 500.0;
        colony
            .apply(&mut world, key, Action::Teleport(Teleport::Home))
            .unwrap();
        assert!(colony.players[&key].pose.pos.distance(at) < 120.0);
        assert_eq!(
            colony.apply(&mut world, key, Action::Teleport(Teleport::Home)),
            Err("The beacon is recharging")
        );
    }

    #[test]
    fn arms_drones_and_sprinklers_do_chores() {
        let (mut world, mut colony, key) = colony();
        let at = colony.players[&key].center();
        // A dome of ripe, drying planters: the pack waters them.
        let home = colony.spawn(
            v2(at.x + 400.0, at.y),
            EntKind::Dome(crate::colony::domes::Dome::new(
                DomeKind::Green,
                "Green Dome 1".into(),
            )),
        );
        for pl in colony.dome_mut(home).unwrap().planters.iter_mut() {
            pl.species = Some(2);
            pl.growth = 1.0;
            pl.moisture = 0.1;
        }
        wear(&mut colony, key, set::SPRINKLER_PACK, 4);
        colony.players.get_mut(&key).unwrap().pose.pos = colony.ents[&home].pos;
        colony.step_mods_second(&mut world);
        let watered = colony
            .dome(home)
            .unwrap()
            .1
            .planters
            .iter()
            .filter(|p| p.moisture == 1.0)
            .count();
        assert!(watered > 0);
        // An arm set to harvest picks what is ripe near its owner.
        wear(&mut colony, key, set::EXTENDO_ARMS, 2);
        colony
            .apply(
                &mut world,
                key,
                Action::ArmTask {
                    arm: 0,
                    task: ArmTask::Harvest,
                },
            )
            .unwrap();
        assert!(
            colony
                .apply(
                    &mut world,
                    key,
                    Action::ArmTask {
                        arm: 1,
                        task: ArmTask::Mine
                    }
                )
                .is_err(),
            "tier II has one arm"
        );
        let before = colony.players[&key].inv.count(Item::Crop(2));
        colony.step_mods_second(&mut world);
        assert!(colony.players[&key].inv.count(Item::Crop(2)) > before);
        // A drone fetches a far-off drop.
        wear(&mut colony, key, set::DRONE_DOCK, 1);
        let me = colony.players[&key].center();
        colony.drop_item(v2(me.x + 90.0, me.y), Item::Xenite, 3);
        for e in colony.ents.values_mut() {
            if let EntKind::Drop(d) = &mut e.kind {
                d.age = 2.0;
            }
        }
        colony.step_mods_second(&mut world);
        assert_eq!(colony.players[&key].inv.count(Item::Xenite), 3);
        let _ = at;
    }

    #[test]
    fn pins_and_pings() {
        let (mut world, mut colony, key) = colony();
        let add = PinOp::Add {
            pos: v2(100.0, 100.0),
            name: "Iron vein".into(),
        };
        colony.apply(&mut world, key, Action::Pin(add)).unwrap();
        assert_eq!(colony.pins[0].name, "Iron vein");
        assert!(
            colony
                .apply(&mut world, key, Action::Pin(PinOp::Ping { pos: v2(5.0, 5.0) }))
                .is_err()
        );
        wear(&mut colony, key, set::BEACON_RACK, 1);
        colony
            .apply(&mut world, key, Action::Pin(PinOp::Ping { pos: v2(5.0, 5.0) }))
            .unwrap();
        colony
            .apply(&mut world, key, Action::Pin(PinOp::Ping { pos: v2(9.0, 9.0) }))
            .unwrap();
        assert_eq!(colony.pins.len(), 2, "a new ping replaces the old one");
        for _ in 0..130 {
            colony.step_mods(&world, 1.0);
        }
        assert_eq!(colony.pins.len(), 1, "pings fade, pins stay");
        colony
            .apply(&mut world, key, Action::Pin(PinOp::Remove(colony.pins[0].id)))
            .unwrap();
        assert!(colony.pins.is_empty());
    }

    #[test]
    fn splicing_makes_a_named_hybrid_once_and_gives_seeds() {
        let (mut world, mut colony, key) = colony();
        let at = colony.players[&key].pose.pos;
        let mut splicer = Machine::new(MachineKind::Splicer, "Gene Splicer 1".into());
        let id = colony.spawn(at, EntKind::Machine(splicer.clone()));
        let p = colony.players.get_mut(&key).unwrap();
        p.inv.add(Item::Seed(0), 3, 1.0);
        p.inv.add(Item::Seed(1), 3, 1.0);
        let act = Action::Splice {
            machine: id,
            a: 0,
            b: 1,
        };
        assert_eq!(
            colony.apply(&mut world, key, act.clone()),
            Err("The Gene Splicer has no power")
        );
        splicer.on = true;
        colony.ents.get_mut(&id).unwrap().kind = EntKind::Machine(splicer);
        let n = colony.species.len();
        colony.apply(&mut world, key, act).unwrap();
        assert_eq!(colony.species.len(), n + 1);
        let hybrid = colony.species[n].clone();
        assert_eq!(hybrid.parents, Some((0, 1)));
        assert!(!hybrid.name.is_empty());
        let p = &colony.players[&key];
        assert_eq!(p.inv.count(Item::Seed(n as SpeciesId)), 2);
        assert_eq!(p.inv.count(Item::Seed(0)), 2);
        // The other way round is the same plant.
        colony
            .apply(
                &mut world,
                key,
                Action::Splice {
                    machine: id,
                    a: 1,
                    b: 0,
                },
            )
            .unwrap();
        assert_eq!(colony.species.len(), n + 1);
        assert_eq!(colony.players[&key].inv.count(Item::Seed(n as SpeciesId)), 4);
        assert!(colony.codex.plants.contains(&(n as SpeciesId)));
        // Hybrids can be crossed again.
        colony
            .players
            .get_mut(&key)
            .unwrap()
            .inv
            .add(Item::Seed(4), 1, 1.0);
        colony
            .apply(
                &mut world,
                key,
                Action::Splice {
                    machine: id,
                    a: n as SpeciesId,
                    b: 4,
                },
            )
            .unwrap();
        assert_eq!(colony.species[n + 1].generation, 2);
    }
}
