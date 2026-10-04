//! Achievements: each teaches a piece of the physics and unlocks something.

use serde::{Deserialize, Serialize};

use super::items::{ItemId, icon};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum AchievementId {
    FloodSurvivor,
    Pyromaniac,
    Alchemist,
    ObsidianBridge,
    Untouched,
    CoreBreaker,
    DeepDiver,
    Firewalker,
    HeatStroke,
    Demolitionist,
    SpeedDigger,
    Conductor,
    CaveIn,
}

/// What an achievement unlocks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reward {
    Item(ItemId),
    Loadout(Loadout),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum Loadout {
    #[default]
    Standard,
    /// Starts with a random unlocked item.
    Gifted,
    /// Extra ropes and torches, plus a Crumbling Pick and Blast Charges.
    Excavator,
}

impl Loadout {
    pub const ALL: [Loadout; 3] = [Loadout::Standard, Loadout::Gifted, Loadout::Excavator];

    pub fn name(self) -> &'static str {
        match self {
            Loadout::Standard => "Standard",
            Loadout::Gifted => "Gifted",
            Loadout::Excavator => "Excavator",
        }
    }

    pub fn desc(self) -> &'static str {
        match self {
            Loadout::Standard => "3 light orbs.",
            Loadout::Gifted => "3 light orbs and a random unlocked item.",
            Loadout::Excavator => "4 light orbs, Crumbling Pick and Blast Charges.",
        }
    }

    pub fn unlock(self) -> Option<AchievementId> {
        match self {
            Loadout::Standard => None,
            Loadout::Gifted => Some(AchievementId::Untouched),
            Loadout::Excavator => Some(AchievementId::CoreBreaker),
        }
    }
}

pub struct AchievementDef {
    /// Must match the table position; checked in tests.
    #[cfg_attr(not(test), allow(dead_code))]
    pub id: AchievementId,
    pub name: &'static str,
    /// Shown when locked.
    pub hint: &'static str,
    pub reward: Reward,
}

use AchievementId as A;

pub static ACHIEVEMENTS: [AchievementDef; 13] = [
    AchievementDef {
        id: A::FloodSurvivor,
        name: "Flood Survivor",
        hint: "Stay underwater for 6 seconds and come up alive.",
        reward: Reward::Item(ItemId::GillMask),
    },
    AchievementDef {
        id: A::Pyromaniac,
        name: "Pyromaniac",
        hint: "Ignite a pocket of gas.",
        reward: Reward::Item(ItemId::BlastCharges),
    },
    AchievementDef {
        id: A::Alchemist,
        name: "Alchemist",
        hint: "Watch acid dissolve 500 cells in one run.",
        reward: Reward::Item(ItemId::AcidFlask),
    },
    AchievementDef {
        id: A::ObsidianBridge,
        name: "Obsidian Bridge",
        hint: "Turn 60 cells of lava into obsidian in one run.",
        reward: Reward::Item(ItemId::PocketSun),
    },
    AchievementDef {
        id: A::Untouched,
        name: "Untouched",
        hint: "Reach the Upper Mantle without taking damage.",
        reward: Reward::Loadout(Loadout::Gifted),
    },
    AchievementDef {
        id: A::CoreBreaker,
        name: "Core Breaker",
        hint: "Touch the core.",
        reward: Reward::Loadout(Loadout::Excavator),
    },
    AchievementDef {
        id: A::DeepDiver,
        name: "Deep Diver",
        hint: "Reach the Deep Mantle.",
        reward: Reward::Item(ItemId::MagmaPick),
    },
    AchievementDef {
        id: A::Firewalker,
        name: "Firewalker",
        hint: "Catch fire and live through it.",
        reward: Reward::Item(ItemId::SalamanderSkin),
    },
    AchievementDef {
        id: A::HeatStroke,
        name: "Heat Stroke",
        hint: "Max out your heat meter for 3 seconds and cool down alive.",
        reward: Reward::Item(ItemId::ThermalSuit),
    },
    AchievementDef {
        id: A::Demolitionist,
        name: "Demolitionist",
        hint: "Destroy 1500 cells with explosions in one run.",
        reward: Reward::Item(ItemId::VolatileCore),
    },
    AchievementDef {
        id: A::SpeedDigger,
        name: "Speed Digger",
        hint: "Reach the Upper Mantle within 4 minutes.",
        reward: Reward::Item(ItemId::GlassCannonPick),
    },
    AchievementDef {
        id: A::Conductor,
        name: "Conductor",
        hint: "Electrify 200 cells of water or metal in one run.",
        reward: Reward::Item(ItemId::SparkRod),
    },
    AchievementDef {
        id: A::CaveIn,
        name: "Cave-In",
        hint: "Bring down 1000 cells of gravel in one run.",
        reward: Reward::Item(ItemId::SeismicStomp),
    },
];

impl AchievementId {
    pub const ALL: [AchievementId; 13] = [
        A::FloodSurvivor,
        A::Pyromaniac,
        A::Alchemist,
        A::ObsidianBridge,
        A::Untouched,
        A::CoreBreaker,
        A::DeepDiver,
        A::Firewalker,
        A::HeatStroke,
        A::Demolitionist,
        A::SpeedDigger,
        A::Conductor,
        A::CaveIn,
    ];

    pub fn def(self) -> &'static AchievementDef {
        &ACHIEVEMENTS[self as usize]
    }

    /// Icon shown for the achievement: its reward's icon.
    pub fn icon(self) -> usize {
        match self.def().reward {
            Reward::Item(item) => item.def().icon,
            Reward::Loadout(_) => icon::TROPHY,
        }
    }

    pub fn reward_name(self) -> &'static str {
        match self.def().reward {
            Reward::Item(item) => item.def().name,
            Reward::Loadout(l) => match l {
                Loadout::Gifted => "Gifted loadout",
                Loadout::Excavator => "Excavator loadout",
                Loadout::Standard => "Standard loadout",
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_matches_ids_and_unlocks_are_consistent() {
        for id in AchievementId::ALL {
            assert_eq!(id.def().id, id);
            if let Reward::Item(item) = id.def().reward {
                assert_eq!(item.def().unlock, Some(id), "{:?} unlock mismatch", item);
            }
        }
        // Every locked item is unlocked by exactly the achievement that rewards it.
        for item in ItemId::ALL {
            if let Some(a) = item.def().unlock {
                assert_eq!(a.def().reward, Reward::Item(item));
            }
        }
        assert!(ACHIEVEMENTS.len() >= 12);
    }
}
