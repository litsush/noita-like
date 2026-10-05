//! Everything that can sit in an inventory slot.

use serde::{Deserialize, Serialize};

use super::plants::{Product, Species, SpeciesId, VOLTBLOOM};
use crate::material::Material;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum DomeKind {
    Starter,
    Green,
    Storage,
    Bedroom,
    Kitchen,
    Dark,
    DomeDome,
    Aqua,
    Apartment,
    Barn,
    WaterGen,
}

/// Machines that stand on their own outside (or inside) domes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum MachineKind {
    Chest,
    Lamp,
    Pylon,
    Pump,
    Tank,
    OxygenGenerator,
    Synthesizer,
    Splicer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Item {
    Stone,
    Dirt,
    Sand,
    Wood,
    Fiber,
    Iron,
    Copper,
    Silicon,
    Gold,
    Titanium,
    Xenite,
    Aurorium,
    Chitin,
    Silk,
    BioGel,
    Glass,
    Fertilizer,
    Meal,
    Egg,
    Milk,
    Fish,
    WateringCan,
    Pipe,
    Machine(MachineKind),
    RobotKit,
    DomeKit(DomeKind),
    Cluckbug,
    MilkGrub,
    FishFry,
    Seed(SpeciesId),
    /// The harvest of a species: food crops and battery flowers.
    Crop(SpeciesId),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Category {
    Resource,
    Food,
    Tool,
    Building,
    Animal,
    Seed,
    Crop,
}

/// Raw resources stack high; everything else stacks to this.
pub const STACK: u32 = 99;
pub const STACK_RAW: u32 = 999;

impl Item {
    /// Items with fixed names, for menus and tests.
    pub const FIXED: [Item; 23] = [
        Item::Stone,
        Item::Dirt,
        Item::Sand,
        Item::Wood,
        Item::Fiber,
        Item::Iron,
        Item::Copper,
        Item::Silicon,
        Item::Gold,
        Item::Titanium,
        Item::Xenite,
        Item::Aurorium,
        Item::Chitin,
        Item::Silk,
        Item::BioGel,
        Item::Glass,
        Item::Fertilizer,
        Item::Meal,
        Item::Egg,
        Item::Milk,
        Item::Fish,
        Item::WateringCan,
        Item::Pipe,
    ];

    pub fn category(self) -> Category {
        use Item::*;
        match self {
            Stone | Dirt | Sand | Wood | Fiber | Iron | Copper | Silicon | Gold | Titanium | Xenite
            | Aurorium | Chitin | Silk | BioGel | Glass | Fertilizer => Category::Resource,
            Meal | Egg | Milk | Fish => Category::Food,
            WateringCan => Category::Tool,
            Pipe | Machine(_) | RobotKit | DomeKit(_) => Category::Building,
            Cluckbug | MilkGrub | FishFry => Category::Animal,
            Seed(_) => Category::Seed,
            Crop(_) => Category::Crop,
        }
    }

    pub fn max_stack(self) -> u32 {
        match self {
            Item::WateringCan => 1,
            Item::Machine(_) | Item::RobotKit | Item::DomeKit(_) => 20,
            Item::Pipe => STACK_RAW,
            _ if self.category() == Category::Resource => STACK_RAW,
            _ => STACK,
        }
    }

    /// Name of the item. Seeds and crops are named after their species.
    pub fn name(self, species: &[Species]) -> String {
        use Item::*;
        let fixed = match self {
            Stone => "Stone",
            Dirt => "Dirt",
            Sand => "Sand",
            Wood => "Wood",
            Fiber => "Plant Fiber",
            Iron => "Iron",
            Copper => "Copper",
            Silicon => "Silicon",
            Gold => "Gold",
            Titanium => "Titanium",
            Xenite => "Xenite",
            Aurorium => "Aurorium",
            Chitin => "Chitin",
            Silk => "Silk",
            BioGel => "Bio-gel",
            Glass => "Glass",
            Fertilizer => "Fertilizer",
            Meal => "Meal",
            Egg => "Egg",
            Milk => "Milk",
            Fish => "Fish",
            WateringCan => "Watering Can",
            Pipe => "Copper Pipe",
            RobotKit => "Robot",
            Cluckbug => "Cluckbug",
            MilkGrub => "Milk Grub",
            FishFry => "Fish Fry",
            Machine(m) => m.name(),
            DomeKit(d) => return format!("{} kit", d.name()),
            Seed(s) => {
                return match species.get(s as usize) {
                    Some(sp) => format!("{} seed", sp.name),
                    None => "Unknown seed".into(),
                };
            }
            Crop(s) => {
                return match species.get(s as usize) {
                    Some(sp) if s == VOLTBLOOM => format!("Battery Flower ({})", sp.name),
                    Some(sp) if sp.yields(Product::Flower) => format!("{} flower", sp.name),
                    Some(sp) => sp.name.clone(),
                    None => "Unknown crop".into(),
                };
            }
        };
        fixed.to_string()
    }

    /// A line of help for the tooltip.
    pub fn describe(self, species: &[Species]) -> String {
        use Item::*;
        match self {
            Stone | Dirt | Sand => "Building block. Place it with the hotbar.".into(),
            Wood => "From trees and Ironbark. Building block and crafting material.".into(),
            Fiber => "From leaves, moss and Fiber Vine. Makes fertilizer.".into(),
            Iron | Copper | Silicon | Gold | Titanium | Xenite | Aurorium => {
                "Ore. Used by the Synthesizer; Earth pays for the surplus.".into()
            }
            Chitin | Silk | BioGel => "Dropped by creatures. Earth pays well for samples.".into(),
            Glass => "Synthesized from sand. Domes need a lot of it.".into(),
            Fertilizer => "Plants in a fertilized planter grow 50% faster.".into(),
            Meal => "Cooked in a Kitchen Dome. Feeds workers and counts as food output.".into(),
            Egg | Milk | Fish => "Food from the colony's animals.".into(),
            WateringCan => "Fill it at any water, then use it on a planter.".into(),
            Pipe => "Drag to lay copper pipe between water sources and domes.".into(),
            RobotKit => "Place it to deploy a robot, then give it a program.".into(),
            Cluckbug => "Put it in a Barn Dome. Lays eggs.".into(),
            MilkGrub => "Put it in a Barn Dome. Gives milk.".into(),
            FishFry => "Release into an Aqua Dome's fish pool.".into(),
            Machine(m) => m.describe().into(),
            DomeKit(d) => d.describe().into(),
            Seed(s) => match species.get(s as usize) {
                Some(sp) => format!("Plant it in a planter. Grows into {}.", sp.name),
                None => String::new(),
            },
            Crop(s) => match species.get(s as usize) {
                Some(sp) if sp.yields(Product::Flower) => {
                    format!("Holds {:.0} charge. Load it into a Charging Pylon.", sp.charge())
                }
                Some(sp) => format!("Food value {:.0}. Cook three crops into a Meal.", sp.food_value()),
                None => String::new(),
            },
        }
    }

    /// Credits Earth pays for one.
    pub fn value(self, species: &[Species]) -> u32 {
        use Item::*;
        match self {
            Stone | Dirt | Sand => 0,
            Wood | Fiber => 1,
            Iron => 4,
            Copper => 6,
            Silicon => 8,
            Gold => 30,
            Titanium => 55,
            Xenite => 140,
            Aurorium => 450,
            Chitin => 10,
            Silk => 12,
            BioGel => 15,
            Glass => 2,
            Fertilizer => 3,
            Meal => 15,
            Egg | Milk | Fish => 8,
            Crop(s) => species.get(s as usize).map_or(2, |sp| sp.value()),
            _ => 0,
        }
    }

    /// Food points when eaten or counted toward the colony's food output.
    pub fn food(self, species: &[Species]) -> f32 {
        match self {
            Item::Meal => 12.0,
            Item::Egg | Item::Milk => 3.0,
            Item::Fish => 4.0,
            Item::Crop(s) => species.get(s as usize).map_or(0.0, |sp| sp.food_value() * 0.5),
            _ => 0.0,
        }
    }

    /// Charge when loaded into a pylon.
    pub fn charge(self, species: &[Species]) -> f32 {
        match self {
            Item::Crop(s) => species.get(s as usize).map_or(0.0, |sp| sp.charge()),
            _ => 0.0,
        }
    }

    /// The terrain this item places, if it is a block.
    pub fn block(self) -> Option<Material> {
        match self {
            Item::Stone => Some(Material::Stone),
            Item::Dirt => Some(Material::Dirt),
            Item::Sand => Some(Material::Sand),
            Item::Wood => Some(Material::Wood),
            _ => None,
        }
    }

    /// Index into `icons.png` (see `tools/gen_art.py`). Seeds and crops share
    /// a neutral icon that the UI tints per species.
    pub fn icon(self) -> usize {
        use Item::*;
        match self {
            Stone => 0,
            Dirt => 1,
            Sand => 2,
            Wood => 3,
            Fiber => 4,
            Iron => 5,
            Copper => 6,
            Silicon => 7,
            Gold => 8,
            Titanium => 9,
            Xenite => 10,
            Aurorium => 11,
            Chitin => 12,
            Silk => 13,
            BioGel => 14,
            Glass => 15,
            Fertilizer => 16,
            Meal => 17,
            Egg => 18,
            Milk => 19,
            Fish => 20,
            WateringCan => 22,
            Pipe => 28,
            RobotKit => 30,
            Cluckbug => 43,
            MilkGrub => 44,
            FishFry => 45,
            Machine(m) => m.icon(),
            DomeKit(d) => d.icon(),
            Seed(_) => 46,
            Crop(VOLTBLOOM) => 92,
            Crop(_) => 47,
        }
    }
}

impl MachineKind {
    pub const ALL: [MachineKind; 8] = [
        MachineKind::Chest,
        MachineKind::Lamp,
        MachineKind::Pylon,
        MachineKind::Pump,
        MachineKind::Tank,
        MachineKind::OxygenGenerator,
        MachineKind::Synthesizer,
        MachineKind::Splicer,
    ];

    pub fn name(self) -> &'static str {
        match self {
            MachineKind::Chest => "Chest",
            MachineKind::Lamp => "Lamp",
            MachineKind::Pylon => "Charging Pylon",
            MachineKind::Pump => "Pump",
            MachineKind::Tank => "Water Tank",
            MachineKind::OxygenGenerator => "Oxygen Generator",
            MachineKind::Synthesizer => "Synthesizer",
            MachineKind::Splicer => "Gene Splicer",
        }
    }

    pub fn describe(self) -> &'static str {
        match self {
            MachineKind::Chest => "Holds 40 stacks.",
            MachineKind::Lamp => "Lights the dark.",
            MachineKind::Pylon => {
                "Holds battery flowers and powers everything within 48 cells. Pylons within 60 cells link up. Robots recharge here."
            }
            MachineKind::Pump => "Place it in a lake. Feeds 120 L/day into touching pipes.",
            MachineKind::Tank => "Buffers 500 L for its pipe network.",
            MachineKind::OxygenGenerator => {
                "Releases oxygen into the atmosphere while powered. Runs 50% faster with coolant water."
            }
            MachineKind::Synthesizer => "Turns materials into equipment, domes and body mods.",
            MachineKind::Splicer => "Crosses the seeds of two species into a new hybrid. Needs power.",
        }
    }

    pub fn icon(self) -> usize {
        match self {
            MachineKind::Chest => 24,
            MachineKind::Lamp => 23,
            MachineKind::Pylon => 25,
            MachineKind::Pump => 26,
            MachineKind::Tank => 27,
            MachineKind::OxygenGenerator => 29,
            MachineKind::Synthesizer => 31,
            MachineKind::Splicer => 32,
        }
    }

    /// Footprint in cells (width, height), standing on the ground.
    pub fn size(self) -> (i32, i32) {
        match self {
            MachineKind::Chest => (16, 12),
            MachineKind::Lamp => (6, 26),
            MachineKind::Pylon => (12, 30),
            MachineKind::Pump => (18, 16),
            MachineKind::Tank => (20, 26),
            MachineKind::OxygenGenerator => (24, 30),
            MachineKind::Synthesizer => (24, 24),
            MachineKind::Splicer => (22, 28),
        }
    }
}

impl DomeKind {
    /// Kits a player can synthesize (everything but the starter dome).
    pub const KITS: [DomeKind; 10] = [
        DomeKind::Green,
        DomeKind::Storage,
        DomeKind::Bedroom,
        DomeKind::Kitchen,
        DomeKind::Dark,
        DomeKind::DomeDome,
        DomeKind::Aqua,
        DomeKind::Apartment,
        DomeKind::Barn,
        DomeKind::WaterGen,
    ];

    pub fn name(self) -> &'static str {
        match self {
            DomeKind::Starter => "Starter Biodome",
            DomeKind::Green => "Green Dome",
            DomeKind::Storage => "Storage Dome",
            DomeKind::Bedroom => "Bedroom Dome",
            DomeKind::Kitchen => "Kitchen Dome",
            DomeKind::Dark => "Dark Dome",
            DomeKind::DomeDome => "Dome Dome",
            DomeKind::Aqua => "Aqua Dome",
            DomeKind::Apartment => "Apartment Dome",
            DomeKind::Barn => "Barn Dome",
            DomeKind::WaterGen => "Water Generator Dome",
        }
    }

    pub fn describe(self) -> &'static str {
        match self {
            DomeKind::Starter => "Home. Beds, storage, the Synthesizer, the Comm. Terminal and the Mod Bay.",
            DomeKind::Green => {
                "Four planters for land crops, including battery flowers. Plants release oxygen. Needs water: by hand, or sprinklers on a pipe network."
            }
            DomeKind::Storage => {
                "Six chests in the colony stockpile, which synthesizers and robots draw from."
            }
            DomeKind::Bedroom => {
                "Four beds. Sleep through the night and set your spawn. Houses four workers."
            }
            DomeKind::Kitchen => "Cooks any three crops into a Meal. Needs power and water.",
            DomeKind::Dark => {
                "Four planters for mushrooms and other dark species, and a Compost Vat that makes fertilizer."
            }
            DomeKind::DomeDome => {
                "Builds a dome kit of your choice for free. It just takes time. Needs power."
            }
            DomeKind::Aqua => "Two aquatic planters (lots of oxygen) and a fish pool (food). Needs water.",
            DomeKind::Apartment => {
                "Housing for 20 colonists and 6 workers. Counts toward readiness when it has power and water."
            }
            DomeKind::Barn => {
                "Four stalls for animals from Earth. They need water and crops as feed, and give eggs, milk and fertilizer."
            }
            DomeKind::WaterGen => {
                "Condenses 90 L of water a day into its pipe network (more in rain). Needs power."
            }
        }
    }

    /// File stem of the dome's sprites.
    pub fn sprite(self) -> &'static str {
        match self {
            DomeKind::Starter => "starter",
            DomeKind::Green => "green",
            DomeKind::Storage => "storage",
            DomeKind::Bedroom => "bedroom",
            DomeKind::Kitchen => "kitchen",
            DomeKind::Dark => "dark",
            DomeKind::DomeDome => "domedome",
            DomeKind::Aqua => "aqua",
            DomeKind::Apartment => "apartment",
            DomeKind::Barn => "barn",
            DomeKind::WaterGen => "watergen",
        }
    }

    pub fn icon(self) -> usize {
        match self {
            DomeKind::Starter => 93,
            DomeKind::Green => 33,
            DomeKind::Storage => 34,
            DomeKind::Bedroom => 35,
            DomeKind::Kitchen => 36,
            DomeKind::Dark => 37,
            DomeKind::DomeDome => 38,
            DomeKind::Aqua => 39,
            DomeKind::Apartment => 40,
            DomeKind::Barn => 41,
            DomeKind::WaterGen => 42,
        }
    }

    /// Interior size in cells (width, height); see the dome geometry in
    /// `colony::domes`.
    pub fn size(self) -> (i32, i32) {
        match self {
            DomeKind::Starter => (216, 80),
            DomeKind::Green | DomeKind::DomeDome | DomeKind::Aqua | DomeKind::Barn | DomeKind::Dark => {
                (104, 52)
            }
            DomeKind::Apartment => (136, 64),
            _ => (72, 44),
        }
    }
}

/// What digging a material gives: the item and how many cells make one.
pub fn drop_for(m: Material) -> Option<(Item, u32)> {
    use Material::*;
    Some(match m {
        Stone | Basalt | Obsidian | Gravel => (Item::Stone, 12),
        Dirt | Grass => (Item::Dirt, 12),
        Sand => (Item::Sand, 10),
        Wood => (Item::Wood, 8),
        Leaves | Moss | Fungus => (Item::Fiber, 14),
        Crystal => (Item::Silicon, 14),
        Ruin => (Item::Iron, 22),
        IronOre => (Item::Iron, 5),
        CopperOre => (Item::Copper, 5),
        SiliconOre => (Item::Silicon, 5),
        GoldOre => (Item::Gold, 6),
        TitaniumOre => (Item::Titanium, 6),
        XeniteOre => (Item::Xenite, 7),
        AuroriumOre => (Item::Aurorium, 8),
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colony::plants::base_species;

    #[test]
    fn names_and_icons() {
        let sp = base_species();
        for item in Item::FIXED {
            assert!(!item.name(&sp).is_empty());
            assert!(!item.describe(&sp).is_empty());
            assert!(item.icon() < 128);
        }
        assert_eq!(Item::Seed(1).name(&sp), "Breathfern seed");
        assert_eq!(Item::Crop(2).name(&sp), "Ration Root");
        assert!(Item::Crop(VOLTBLOOM).name(&sp).starts_with("Battery Flower"));
        assert_eq!(Item::Crop(VOLTBLOOM).charge(&sp), 100.0);
        assert!(Item::Crop(2).food(&sp) > 0.0);
        assert_eq!(Item::DomeKit(DomeKind::Green).name(&sp), "Green Dome kit");
        for d in DomeKind::KITS {
            assert!(!d.describe().is_empty());
        }
        // Icons are distinct for fixed items.
        let mut icons: Vec<usize> = Item::FIXED.iter().map(|i| i.icon()).collect();
        icons.extend(MachineKind::ALL.iter().map(|m| m.icon()));
        icons.extend(DomeKind::KITS.iter().map(|d| d.icon()));
        let n = icons.len();
        icons.sort_unstable();
        icons.dedup();
        assert_eq!(icons.len(), n);
    }

    #[test]
    fn ores_drop_their_item() {
        assert_eq!(drop_for(Material::IronOre), Some((Item::Iron, 5)));
        assert_eq!(drop_for(Material::Water), None);
        assert_eq!(drop_for(Material::Glass), None);
        for i in 0..Material::COUNT as u8 {
            let m = Material::from_u8(i);
            if m.is_ore() {
                assert!(drop_for(m).is_some());
            }
        }
        // Richer ores are worth more.
        let sp = base_species();
        assert!(Item::Aurorium.value(&sp) > Item::Xenite.value(&sp));
        assert!(Item::Xenite.value(&sp) > Item::Gold.value(&sp));
    }
}
