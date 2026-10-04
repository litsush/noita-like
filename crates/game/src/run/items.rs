//! Run items. Every item changes how you interact with materials rather
//! than tweaking numbers. Effects live where they apply (digging, hazards,
//! actives); this module is the catalogue.

use serde::{Deserialize, Serialize};

use super::achievements::AchievementId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ItemId {
    CrumblingPick,
    MagmaPick,
    GlassCannonPick,
    WaterCanister,
    AcidFlask,
    BlastCharges,
    SalamanderSkin,
    GillMask,
    HeavyBoots,
    ThermalSuit,
    FrostSeed,
    FungalSpores,
    PocketSun,
    BeaconHeart,
    VolatileCore,
    SparkRod,
    SeismicStomp,
}

/// How an active item is used (right mouse button).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Active {
    /// Hold to spray from a tank that refills in water.
    Spray,
    /// Discrete uses that recharge over time.
    Charges { max: u8, recharge: f32 },
}

pub struct ItemDef {
    pub id: ItemId,
    pub name: &'static str,
    pub desc: &'static str,
    pub flavor: &'static str,
    /// Index into the item icon sheet.
    pub icon: usize,
    pub active: Option<Active>,
    /// Achievement that unlocks it; `None` means available from the start.
    pub unlock: Option<AchievementId>,
    /// Pick upgrades replace each other.
    pub is_pick: bool,
}

const fn def(
    id: ItemId,
    name: &'static str,
    desc: &'static str,
    flavor: &'static str,
    icon: usize,
    active: Option<Active>,
    unlock: Option<AchievementId>,
) -> ItemDef {
    ItemDef { id, name, desc, flavor, icon, active, unlock, is_pick: false }
}

use AchievementId as A;
use ItemId as I;

pub static ITEMS: [ItemDef; 17] = [
    ItemDef {
        is_pick: true,
        ..def(
            I::CrumblingPick,
            "Crumbling Pick",
            "Dug rock crumbles into sand and gravel instead of vanishing.",
            "Nothing is destroyed. Only rearranged.",
            0,
            None,
            None,
        )
    },
    ItemDef {
        is_pick: true,
        ..def(
            I::MagmaPick,
            "Magma Pick",
            "Rock you dig sometimes melts into lava.",
            "Forged in the vent where the mantle sings.",
            1,
            None,
            Some(A::DeepDiver),
        )
    },
    ItemDef {
        is_pick: true,
        ..def(
            I::GlassCannonPick,
            "Glass Cannon Pick",
            "Dig through almost anything instantly, but you take double damage.",
            "Sharp enough to cut the planet. And you.",
            2,
            None,
            Some(A::SpeedDigger),
        )
    },
    def(
        I::WaterCanister,
        "Water Canister",
        "Hold right mouse to spray water. Cools lava, douses fire, carries current. Refills when you swim.",
        "Half full. Always half full.",
        3,
        Some(Active::Spray),
        None,
    ),
    def(
        I::AcidFlask,
        "Acid Flask",
        "Throw a flask of acid that eats through rock (but not obsidian or crystal).",
        "Do not drink. Do not hold for long.",
        4,
        Some(Active::Charges { max: 3, recharge: 22.0 }),
        Some(A::Alchemist),
    ),
    def(
        I::BlastCharges,
        "Blast Charges",
        "Place a charge that detonates after a short fuse, blasting a crater.",
        "Mining, but faster and louder.",
        5,
        Some(Active::Charges { max: 3, recharge: 28.0 }),
        Some(A::Pyromaniac),
    ),
    def(
        I::SalamanderSkin,
        "Salamander Skin",
        "Fire and lava can't hurt you. Water burns like acid.",
        "Wear it and the magma feels like a warm bath.",
        6,
        None,
        Some(A::Firewalker),
    ),
    def(
        I::GillMask,
        "Gill Mask",
        "Breathe underwater and in toxic gas.",
        "Smells like a fish market. Keeps you alive.",
        7,
        None,
        Some(A::FloodSurvivor),
    ),
    def(
        I::HeavyBoots,
        "Heavy Boots",
        "Walk on molten metal. You sink fast in water and wade through sand.",
        "Every step is a decision.",
        8,
        None,
        None,
    ),
    def(
        I::ThermalSuit,
        "Thermal Suit",
        "Heat no longer hurts. It charges the suit, which vents it as a burst of fire when full.",
        "Insulation that remembers.",
        9,
        None,
        Some(A::HeatStroke),
    ),
    def(
        I::FrostSeed,
        "Frost Seed",
        "Throw a seed that grows ice through water and turns lava into obsidian.",
        "Cold enough to freeze a river mid-sentence.",
        10,
        Some(Active::Charges { max: 2, recharge: 18.0 }),
        None,
    ),
    def(
        I::FungalSpores,
        "Fungal Spores",
        "Throw spores that grow glowing, flammable fungus over rock.",
        "Light for the dark. Fuel for the fire.",
        11,
        Some(Active::Charges { max: 3, recharge: 14.0 }),
        None,
    ),
    def(
        I::PocketSun,
        "Pocket Sun",
        "Place a tiny sun that cooks lava into obsidian, boils water and lights the dark.",
        "Handle by the corona.",
        12,
        Some(Active::Charges { max: 1, recharge: 30.0 }),
        Some(A::ObsidianBridge),
    ),
    def(
        I::BeaconHeart,
        "Beacon Heart",
        "Your lamp shines far brighter and farther, but creatures can see you coming.",
        "Hope is visible from a long way off.",
        13,
        None,
        None,
    ),
    def(
        I::VolatileCore,
        "Volatile Core",
        "Explosions you cause leave veins of ore behind.",
        "Destruction is just mining with enthusiasm.",
        14,
        None,
        Some(A::Demolitionist),
    ),
    def(
        I::SparkRod,
        "Spark Rod",
        "Shoot sparks that electrify water and metal and ignite gas.",
        "Point away from face.",
        15,
        Some(Active::Charges { max: 1, recharge: 1.2 }),
        Some(A::Conductor),
    ),
    def(
        I::SeismicStomp,
        "Seismic Stomp",
        "Hard landings shatter the ground beneath you and shake gravel loose nearby.",
        "Land like you mean it.",
        16,
        None,
        Some(A::CaveIn),
    ),
];

impl ItemId {
    pub const ALL: [ItemId; 17] = [
        I::CrumblingPick,
        I::MagmaPick,
        I::GlassCannonPick,
        I::WaterCanister,
        I::AcidFlask,
        I::BlastCharges,
        I::SalamanderSkin,
        I::GillMask,
        I::HeavyBoots,
        I::ThermalSuit,
        I::FrostSeed,
        I::FungalSpores,
        I::PocketSun,
        I::BeaconHeart,
        I::VolatileCore,
        I::SparkRod,
        I::SeismicStomp,
    ];

    pub fn def(self) -> &'static ItemDef {
        &ITEMS[self as usize]
    }
}

/// Icon indices for non-item things in the item sheet.
pub mod icon {
    pub const CHEST_CLOSED: usize = 17;
    pub const ROPE: usize = 21;
    pub const TORCH: usize = 22;
    pub const ORE: usize = 23;
    pub const GEM: usize = 24;
    pub const HEART_FULL: usize = 25;
    pub const HEART_HALF: usize = 26;
    pub const HEART_EMPTY: usize = 27;
    pub const LOCK: usize = 28;
    pub const FLAME: usize = 29;
    pub const BUBBLE: usize = 30;
    pub const THERMOMETER: usize = 31;
    pub const SKULL: usize = 32;
    pub const TROPHY: usize = 33;
    pub const PICKAXE: usize = 34;
    pub const UNKNOWN: usize = 35;
    pub const COMPASS: usize = 36;
    pub const HOURGLASS: usize = 37;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_matches_ids() {
        for id in ItemId::ALL {
            assert_eq!(id.def().id, id);
        }
        assert!(ITEMS.iter().filter(|d| d.unlock.is_none()).count() >= 6);
        assert!(ITEMS.len() >= 15);
    }
}
