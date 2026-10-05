//! Body mods. This file holds the stats that mods change; the 53 sets and
//! their tiers are filled in by the body-mod milestone.

use serde::{Deserialize, Serialize};

/// Everything about a player that equipped mods can change. Systems read
/// these instead of constants.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ModStats {
    pub max_hp: f32,
    /// Multiplier on damage taken.
    pub damage_taken: f32,
    pub o2_capacity: f32,
    /// Multiplier on suit oxygen drain.
    pub o2_drain: f32,
    pub o2_refill: f32,
    pub battery_capacity: f32,
    pub battery_regen: f32,
    /// The battery doesn't lock out when drained.
    pub no_lockout: bool,
    /// 0–1 fraction of cold ignored.
    pub cold_resist: f32,
    /// 0–1 fraction of spore gas damage ignored.
    pub toxin_resist: f32,
    /// Health regenerated per second anywhere.
    pub regen: f32,
    /// How far the player can dig, place and interact, in cells.
    pub reach: f32,
    pub dig_power: f32,
    pub dig_radius: i32,
    /// Multiplier on items from dug ore.
    pub ore_yield: f32,
    pub bolt_damage: f32,
    pub bolt_cost: f32,
    pub bolt_count: u32,
    pub bolt_size: f32,
    /// Pickup radius in cells.
    pub magnet: f32,
    pub extra_slots: u32,
    pub stack_mult: f32,
}

impl Default for ModStats {
    fn default() -> ModStats {
        ModStats {
            max_hp: 100.0,
            damage_taken: 1.0,
            o2_capacity: 100.0,
            o2_drain: 1.0,
            o2_refill: 1.0,
            battery_capacity: 100.0,
            battery_regen: 1.0,
            no_lockout: false,
            cold_resist: 0.0,
            toxin_resist: 0.0,
            regen: 0.0,
            reach: 64.0,
            dig_power: 1.0,
            dig_radius: 4,
            ore_yield: 1.0,
            bolt_damage: 1.0,
            bolt_cost: 1.0,
            bolt_count: 1,
            bolt_size: 1.0,
            magnet: 30.0,
            extra_slots: 0,
            stack_mult: 1.0,
        }
    }
}
