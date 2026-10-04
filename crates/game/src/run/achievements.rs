//! Achievements: each teaches a piece of the physics and unlocks a school,
//! a scroll or a starting choice for later runs.

use serde::{Deserialize, Serialize};

use super::scrolls::{School, ScrollId};

/// Saved by name (`format!("{:?}")`); see [`AchievementId::from_name`] for
/// how older names map onto current ones.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum AchievementId {
    FloodSurvivor,
    Pyromaniac,
    Alchemist,
    ObsidianBridge,
    Untouched,
    RiteComplete,
    DeepDiver,
    Firewalker,
    HeatStroke,
    Demolitionist,
    SpeedDigger,
    Conductor,
    CaveIn,
    LongFall,
    Lightless,
    GraveRobber,
    FusionAdept,
    LoreKeeper,
    StalkersShadow,
    Gardener,
    Overcharged,
    TwinMastery,
}

/// What an achievement unlocks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reward {
    School(School),
    Scroll(ScrollId),
    Start(StartChoice),
}

/// How a run begins (replaces the old loadouts; old saved values fall back
/// to the default).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum StartChoice {
    #[default]
    Initiate,
    /// Begins with a random scroll of the first school.
    Prodigy,
    /// Extra shards and an extra light-orb charge.
    Scavenger,
    /// A first-school scroll and Blink.
    ArchmagesHeir,
    /// Chooses both schools at the start.
    TwinSouled,
}

impl StartChoice {
    pub const ALL: [StartChoice; 5] = [
        StartChoice::Initiate,
        StartChoice::Prodigy,
        StartChoice::Scavenger,
        StartChoice::ArchmagesHeir,
        StartChoice::TwinSouled,
    ];

    pub fn name(self) -> &'static str {
        match self {
            StartChoice::Initiate => "Initiate",
            StartChoice::Prodigy => "Prodigy",
            StartChoice::Scavenger => "Scavenger",
            StartChoice::ArchmagesHeir => "Archmage's Heir",
            StartChoice::TwinSouled => "Twin-Souled",
        }
    }

    pub fn desc(self) -> &'static str {
        match self {
            StartChoice::Initiate => "A staff, three light orbs and your wits.",
            StartChoice::Prodigy => "Begin knowing a scroll of your first school.",
            StartChoice::Scavenger => "Begin with 40 shards and four light orbs.",
            StartChoice::ArchmagesHeir => "Begin with a first-school scroll and Blink.",
            StartChoice::TwinSouled => "Choose both of your schools before descending.",
        }
    }

    pub fn icon(self) -> usize {
        101 + self as usize
    }

    pub fn unlock(self) -> Option<AchievementId> {
        match self {
            StartChoice::Initiate => None,
            StartChoice::Prodigy => Some(AchievementId::Untouched),
            StartChoice::Scavenger => Some(AchievementId::GraveRobber),
            StartChoice::ArchmagesHeir => Some(AchievementId::RiteComplete),
            StartChoice::TwinSouled => Some(AchievementId::TwinMastery),
        }
    }
}

pub struct AchievementDef {
    pub id: AchievementId,
    pub name: &'static str,
    /// Shown when locked.
    pub hint: &'static str,
    pub reward: Reward,
}

use AchievementId as A;

const fn ach(id: AchievementId, name: &'static str, hint: &'static str, reward: Reward) -> AchievementDef {
    AchievementDef {
        id,
        name,
        hint,
        reward,
    }
}

pub static ACHIEVEMENTS: [AchievementDef; 22] = [
    ach(
        A::FloodSurvivor,
        "Flood Survivor",
        "Stay underwater for 6 seconds and come up alive.",
        Reward::Scroll(ScrollId::GillsOfTheDeep),
    ),
    ach(
        A::Pyromaniac,
        "Pyromaniac",
        "Ignite a pocket of gas.",
        Reward::Scroll(ScrollId::AlchemistsCharge),
    ),
    ach(
        A::Alchemist,
        "Alchemist",
        "Watch acid dissolve 500 cells in one run.",
        Reward::School(School::Alchemy),
    ),
    ach(
        A::ObsidianBridge,
        "Obsidian Bridge",
        "Turn 60 cells of lava into obsidian in one run.",
        Reward::School(School::Cryomancy),
    ),
    ach(
        A::Untouched,
        "Untouched",
        "Reach the Drowned Halls without taking damage.",
        Reward::Start(StartChoice::Prodigy),
    ),
    ach(
        A::RiteComplete,
        "Rite Complete",
        "Touch the heart of the world.",
        Reward::Start(StartChoice::ArchmagesHeir),
    ),
    ach(
        A::DeepDiver,
        "Deep Diver",
        "Reach the Fungal Abyss.",
        Reward::Scroll(ScrollId::MagmaBore),
    ),
    ach(
        A::Firewalker,
        "Firewalker",
        "Catch fire and live through it.",
        Reward::Scroll(ScrollId::SalamanderWard),
    ),
    ach(
        A::HeatStroke,
        "Heat Stroke",
        "Max out your heat for 3 seconds and cool down alive.",
        Reward::Scroll(ScrollId::EmberHeart),
    ),
    ach(
        A::Demolitionist,
        "Demolitionist",
        "Destroy 1500 cells with explosions in one run.",
        Reward::Scroll(ScrollId::VolatileCore),
    ),
    ach(
        A::SpeedDigger,
        "Speed Digger",
        "Reach the Drowned Halls within 4 minutes.",
        Reward::Scroll(ScrollId::GlassFocus),
    ),
    ach(
        A::Conductor,
        "Conductor",
        "Electrify 200 cells of water in one run.",
        Reward::School(School::Fulmancy),
    ),
    ach(
        A::CaveIn,
        "Cave-In",
        "Bring down 1000 cells of gravel in one run.",
        Reward::Scroll(ScrollId::SeismicStomp),
    ),
    ach(
        A::LongFall,
        "Long Fall",
        "Fall 150 cells in one drop and survive.",
        Reward::School(School::Aeromancy),
    ),
    ach(
        A::Lightless,
        "Lightless",
        "Spend 60 seconds in darkness without casting a light orb.",
        Reward::Scroll(ScrollId::Blink),
    ),
    ach(
        A::GraveRobber,
        "Grave Robber",
        "Search 5 fallen apprentices in one run.",
        Reward::Start(StartChoice::Scavenger),
    ),
    ach(
        A::FusionAdept,
        "Fusion Adept",
        "Awaken a fusion of two schools.",
        Reward::Scroll(ScrollId::Symbiosis),
    ),
    ach(
        A::LoreKeeper,
        "Lore Keeper",
        "Read 10 different journal pages.",
        Reward::Scroll(ScrollId::PocketSun),
    ),
    ach(
        A::StalkersShadow,
        "Stalker's Shadow",
        "Escape or trap the Hollow Stalker.",
        Reward::Scroll(ScrollId::RimeShell),
    ),
    ach(
        A::Gardener,
        "Gardener",
        "Grow 300 cells of fungus in one run.",
        Reward::Scroll(ScrollId::MycelialBridge),
    ),
    ach(
        A::Overcharged,
        "Overcharged",
        "Kill a creature with electricity.",
        Reward::Scroll(ScrollId::ChainLightning),
    ),
    ach(
        A::TwinMastery,
        "Twin Mastery",
        "Complete the rite with your fusion awakened.",
        Reward::Start(StartChoice::TwinSouled),
    ),
];

impl AchievementId {
    pub fn all() -> impl Iterator<Item = AchievementId> {
        ACHIEVEMENTS.iter().map(|d| d.id)
    }

    pub fn def(self) -> &'static AchievementDef {
        &ACHIEVEMENTS[self as usize]
    }

    pub fn name_id(self) -> String {
        format!("{self:?}")
    }

    /// Parses a saved name, mapping names from older versions.
    pub fn from_name(s: &str) -> Option<AchievementId> {
        let s = match s {
            "CoreBreaker" => "RiteComplete",
            other => other,
        };
        AchievementId::all().find(|a| a.name_id() == s)
    }

    /// Icon shown for the achievement: its reward's icon.
    pub fn icon(self) -> usize {
        match self.def().reward {
            Reward::School(s) => s.sigil(),
            Reward::Scroll(sc) => sc.icon(),
            Reward::Start(c) => c.icon(),
        }
    }

    pub fn reward_name(self) -> String {
        match self.def().reward {
            Reward::School(s) => format!("the {} school", s.name()),
            Reward::Scroll(sc) => sc.def().name.to_string(),
            Reward::Start(c) => format!("the {} start", c.name()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_matches_ids_and_unlocks_are_consistent() {
        for (i, d) in ACHIEVEMENTS.iter().enumerate() {
            assert_eq!(d.id as usize, i, "{}", d.name);
        }
        assert!(ACHIEVEMENTS.len() >= 20);
        for sc in ScrollId::all() {
            if let Some(a) = sc.def().unlock {
                assert_eq!(a.def().reward, Reward::Scroll(sc), "{sc:?}");
            }
        }
        for s in School::ALL {
            if let Some(a) = s.unlock() {
                assert_eq!(a.def().reward, Reward::School(s), "{s:?}");
            }
        }
        for c in StartChoice::ALL {
            if let Some(a) = c.unlock() {
                assert_eq!(a.def().reward, Reward::Start(c), "{c:?}");
            }
        }
    }

    #[test]
    fn old_names_map_forward() {
        assert_eq!(
            AchievementId::from_name("CoreBreaker"),
            Some(AchievementId::RiteComplete)
        );
        assert_eq!(
            AchievementId::from_name("Pyromaniac"),
            Some(AchievementId::Pyromaniac)
        );
        assert_eq!(AchievementId::from_name("Nonsense"), None);
    }
}
