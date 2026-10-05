//! What the Synthesizer can make, and the blueprints that unlock it.

use std::sync::LazyLock;

use serde::{Deserialize, Serialize};

use super::items::{DomeKind, Item, MachineKind};
use super::plants::{Product, Species};

/// A purchasable set of recipes.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Blueprint {
    Plumbing,
    Kitchen,
    WaterGen,
    Dark,
    Aqua,
    Barn,
    OxygenGenerator,
    Robot,
    Synthesizer,
    Splicer,
    DomeDome,
    Apartment,
    /// A body-mod set.
    Mod(u8),
}

impl Blueprint {
    /// Everything Earth sells except mod sets, in catalogue order.
    pub const BASE: [Blueprint; 12] = [
        Blueprint::Plumbing,
        Blueprint::Kitchen,
        Blueprint::Synthesizer,
        Blueprint::Dark,
        Blueprint::WaterGen,
        Blueprint::Barn,
        Blueprint::Aqua,
        Blueprint::OxygenGenerator,
        Blueprint::Robot,
        Blueprint::DomeDome,
        Blueprint::Apartment,
        Blueprint::Splicer,
    ];

    pub fn name(self) -> String {
        match self {
            Blueprint::Plumbing => "Plumbing (pipes, pump, tank)".into(),
            Blueprint::Kitchen => "Kitchen Dome".into(),
            Blueprint::WaterGen => "Water Generator Dome".into(),
            Blueprint::Dark => "Dark Dome".into(),
            Blueprint::Aqua => "Aqua Dome".into(),
            Blueprint::Barn => "Barn Dome".into(),
            Blueprint::OxygenGenerator => "Oxygen Generator".into(),
            Blueprint::Robot => "Robot".into(),
            Blueprint::Synthesizer => "Synthesizer".into(),
            Blueprint::Splicer => "Gene Splicer".into(),
            Blueprint::DomeDome => "Dome Dome".into(),
            Blueprint::Apartment => "Apartment Dome".into(),
            Blueprint::Mod(set) => match super::mods::MOD_SETS.get(set as usize) {
                Some(m) => format!("{} (body mod)", m.name),
                None => "Unknown mod".into(),
            },
        }
    }

    /// Price in credits. Mod sets are priced by the mod table.
    pub fn price(self) -> u32 {
        match self {
            Blueprint::Plumbing => 150,
            Blueprint::Kitchen => 400,
            Blueprint::WaterGen => 600,
            Blueprint::Dark => 500,
            Blueprint::Aqua => 800,
            Blueprint::Barn => 700,
            Blueprint::OxygenGenerator => 1200,
            Blueprint::Robot => 1500,
            Blueprint::Synthesizer => 500,
            Blueprint::Splicer => 5000,
            Blueprint::DomeDome => 3000,
            Blueprint::Apartment => 4000,
            Blueprint::Mod(set) => super::mods::blueprint_price(set),
        }
    }
}

/// One input of a recipe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ingredient {
    Item(Item),
    /// Any harvested food crop.
    AnyCrop,
}

impl Ingredient {
    pub fn accepts(self, item: Item, species: &[Species]) -> bool {
        match self {
            Ingredient::Item(i) => i == item,
            Ingredient::AnyCrop => matches!(item, Item::Crop(s)
                if species.get(s as usize).is_some_and(|sp| sp.yields(Product::Crop))),
        }
    }

    pub fn name(self, species: &[Species]) -> String {
        match self {
            Ingredient::Item(i) => i.name(species),
            Ingredient::AnyCrop => "any crop".into(),
        }
    }

    pub fn icon(self) -> usize {
        match self {
            Ingredient::Item(i) => i.icon(),
            Ingredient::AnyCrop => 47,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Recipe {
    pub inputs: Vec<(Ingredient, u32)>,
    pub output: Item,
    pub count: u32,
    /// `None` for recipes known from the start.
    pub blueprint: Option<Blueprint>,
}

fn recipe(output: Item, count: u32, blueprint: Option<Blueprint>, inputs: &[(Item, u32)]) -> Recipe {
    Recipe {
        inputs: inputs.iter().map(|&(i, n)| (Ingredient::Item(i), n)).collect(),
        output,
        count,
        blueprint,
    }
}

static RECIPES: LazyLock<Vec<Recipe>> = LazyLock::new(|| {
    use Item::*;
    let machine = Item::Machine;
    let dome = Item::DomeKit;
    let mut v = vec![
        recipe(Glass, 2, None, &[(Sand, 3)]),
        recipe(Fertilizer, 2, None, &[(Fiber, 2)]),
        Recipe {
            inputs: vec![(Ingredient::AnyCrop, 3)],
            output: Fertilizer,
            count: 2,
            blueprint: None,
        },
        recipe(machine(MachineKind::Chest), 1, None, &[(Wood, 12), (Iron, 4)]),
        recipe(WateringCan, 1, None, &[(Iron, 6), (Copper, 2)]),
        recipe(machine(MachineKind::Lamp), 1, None, &[(Iron, 2), (Silicon, 2)]),
        recipe(
            machine(MachineKind::Pylon),
            1,
            None,
            &[(Iron, 12), (Copper, 8), (Silicon, 4)],
        ),
        recipe(
            dome(DomeKind::Green),
            1,
            None,
            &[(Glass, 40), (Iron, 30), (Dirt, 40), (Fertilizer, 10), (Wood, 20)],
        ),
        recipe(
            dome(DomeKind::Storage),
            1,
            None,
            &[(Glass, 15), (Iron, 40), (Wood, 20)],
        ),
        recipe(
            dome(DomeKind::Bedroom),
            1,
            None,
            &[(Glass, 20), (Iron, 25), (Wood, 30), (Fiber, 10)],
        ),
    ];
    let plumbing = Some(Blueprint::Plumbing);
    v.extend([
        recipe(Pipe, 8, plumbing, &[(Copper, 4)]),
        recipe(
            machine(MachineKind::Pump),
            1,
            plumbing,
            &[(Iron, 15), (Copper, 15)],
        ),
        recipe(
            machine(MachineKind::Tank),
            1,
            plumbing,
            &[(Iron, 20), (Glass, 10)],
        ),
        recipe(
            dome(DomeKind::Kitchen),
            1,
            Some(Blueprint::Kitchen),
            &[(Glass, 20), (Iron, 30), (Copper, 20), (Silicon, 10)],
        ),
        recipe(
            dome(DomeKind::WaterGen),
            1,
            Some(Blueprint::WaterGen),
            &[(Glass, 20), (Iron, 40), (Copper, 40), (Silicon, 20)],
        ),
        recipe(
            dome(DomeKind::Dark),
            1,
            Some(Blueprint::Dark),
            &[(Stone, 60), (Iron, 30), (Silicon, 10)],
        ),
        recipe(
            dome(DomeKind::Aqua),
            1,
            Some(Blueprint::Aqua),
            &[(Glass, 60), (Iron, 30), (Copper, 20), (Sand, 40)],
        ),
        recipe(
            dome(DomeKind::Barn),
            1,
            Some(Blueprint::Barn),
            &[(Glass, 20), (Iron, 30), (Wood, 60), (Fiber, 20)],
        ),
        recipe(
            machine(MachineKind::OxygenGenerator),
            1,
            Some(Blueprint::OxygenGenerator),
            &[(Iron, 40), (Silicon, 25), (Crop(super::plants::VOLTBLOOM), 10)],
        ),
        recipe(
            RobotKit,
            1,
            Some(Blueprint::Robot),
            &[(Iron, 30), (Silicon, 20), (Gold, 8)],
        ),
        recipe(
            machine(MachineKind::Synthesizer),
            1,
            Some(Blueprint::Synthesizer),
            &[(Iron, 30), (Copper, 15), (Silicon, 15)],
        ),
        recipe(
            machine(MachineKind::Splicer),
            1,
            Some(Blueprint::Splicer),
            &[(Iron, 40), (Gold, 20), (Silicon, 40), (Xenite, 4)],
        ),
        recipe(
            dome(DomeKind::DomeDome),
            1,
            Some(Blueprint::DomeDome),
            &[(Glass, 40), (Iron, 60), (Gold, 20), (Silicon, 40), (Titanium, 10)],
        ),
        recipe(
            dome(DomeKind::Apartment),
            1,
            Some(Blueprint::Apartment),
            &[(Glass, 80), (Iron, 80), (Copper, 40), (Titanium, 20), (Wood, 40)],
        ),
    ]);
    v
});

/// Every recipe, in menu order. A recipe's id is its index.
pub fn recipes() -> &'static [Recipe] {
    &RECIPES
}

/// The recipe that makes a dome kit (for cost discounts and the Dome Dome).
pub fn dome_recipe(kind: DomeKind) -> Option<&'static Recipe> {
    RECIPES.iter().find(|r| r.output == Item::DomeKit(kind))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colony::plants::base_species;

    #[test]
    fn every_buildable_thing_has_a_recipe() {
        let all = recipes();
        for d in DomeKind::KITS {
            assert!(dome_recipe(d).is_some(), "{d:?}");
        }
        for m in MachineKind::ALL {
            assert!(all.iter().any(|r| r.output == Item::Machine(m)), "{m:?}");
        }
        for item in [
            Item::Glass,
            Item::Fertilizer,
            Item::Pipe,
            Item::RobotKit,
            Item::WateringCan,
        ] {
            assert!(all.iter().any(|r| r.output == item), "{item:?}");
        }
        // The start recipes are enough for a first Green Dome.
        let start: Vec<_> = all.iter().filter(|r| r.blueprint.is_none()).collect();
        assert!(start.iter().any(|r| r.output == Item::DomeKit(DomeKind::Green)));
        assert!(
            start
                .iter()
                .any(|r| r.output == Item::Machine(MachineKind::Pylon))
        );
        // Every blueprint Earth sells unlocks something.
        for b in Blueprint::BASE {
            assert!(
                all.iter().any(|r| r.blueprint == Some(b)),
                "{b:?} unlocks nothing"
            );
            assert!(b.price() > 0);
        }
        for r in all {
            assert!(!r.inputs.is_empty() && r.count > 0);
        }
    }

    #[test]
    fn any_crop_accepts_food_but_not_flowers() {
        let sp = base_species();
        let any = Ingredient::AnyCrop;
        assert!(any.accepts(Item::Crop(2), &sp), "ration root");
        assert!(!any.accepts(Item::Crop(0), &sp), "battery flowers aren't food");
        assert!(!any.accepts(Item::Seed(2), &sp));
        assert!(!any.accepts(Item::Iron, &sp));
        assert!(Ingredient::Item(Item::Iron).accepts(Item::Iron, &sp));
    }
}
