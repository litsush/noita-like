/// How a material behaves in the simulation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Empty,
    /// Never moves on its own (terrain, placed blocks).
    Solid,
    /// Falls and piles up (sand, ash, gravel).
    Powder,
    /// Falls and spreads sideways.
    Liquid,
    /// Rises; most gases dissipate.
    Gas,
    /// Burns, spreads to flammable neighbours, then dies.
    Fire,
    /// Short-lived electricity that charges conductive neighbours.
    Energy,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub enum Material {
    #[default]
    Empty = 0,
    Stone,
    Dirt,
    Grass,
    Wood,
    Sand,
    Ash,
    Water,
    Oil,
    Lava,
    Fire,
    Smoke,
    Steam,
    Gravel,
    Crystal,
    Fungus,
    Acid,
    Gas,
    Obsidian,
    Ice,
    Metal,
    Explosive,
    ShardVein,
    CoreShell,
    Spark,
    Basalt,
    Ferrite,
    Brick,
    Vent,
    Frost,
}

/// Hardness at which a material can't be dug or blown up.
pub const INDESTRUCTIBLE: u8 = 255;

pub struct Props {
    pub name: &'static str,
    pub kind: Kind,
    /// Heavier materials sink through lighter liquids and gases.
    pub density: u8,
    /// Chance out of 255 per neighbouring fire per tick to ignite.
    pub flammability: u8,
    /// How far a liquid spreads sideways per tick.
    pub dispersion: u8,
    pub color: [u8; 3],
    /// Max random brightness variation applied per cell.
    pub variance: u8,
    /// Resistance to digging and explosions; [`INDESTRUCTIBLE`] can't be removed.
    pub hardness: u8,
    /// Heat given off (negative = cold). Drives the player's heat meter and melting.
    pub heat: i8,
    /// Carries electric charge (stored in the cell's `life`).
    pub conductive: bool,
    /// Chance out of 255 per tick to eat a neighbouring solid.
    pub corrosive: u8,
    /// Immune to acid.
    pub acid_resistant: bool,
    /// Blast radius when ignited; 0 if not explosive.
    pub explosive: u8,
    /// Light emitted, as RGB intensity.
    pub light: [u8; 3],
    /// Chance out of 255 per random tick to spread into a free neighbour.
    pub growth: u8,
    /// Aether shards dropped when dug.
    pub value: u8,
}

const BASE: Props = Props {
    name: "",
    kind: Kind::Solid,
    density: 255,
    flammability: 0,
    dispersion: 0,
    color: [0, 0, 0],
    variance: 0,
    hardness: 30,
    heat: 0,
    conductive: false,
    corrosive: 0,
    acid_resistant: false,
    explosive: 0,
    light: [0, 0, 0],
    growth: 0,
    value: 0,
};

const fn solid(name: &'static str, color: [u8; 3], variance: u8, hardness: u8) -> Props {
    Props {
        name,
        color,
        variance,
        hardness,
        ..BASE
    }
}

const fn powder(name: &'static str, color: [u8; 3], variance: u8, density: u8, hardness: u8) -> Props {
    Props {
        name,
        kind: Kind::Powder,
        color,
        variance,
        density,
        hardness,
        ..BASE
    }
}

const fn liquid(name: &'static str, color: [u8; 3], variance: u8, density: u8, dispersion: u8) -> Props {
    Props {
        name,
        kind: Kind::Liquid,
        color,
        variance,
        density,
        dispersion,
        hardness: 0,
        ..BASE
    }
}

const fn gas(name: &'static str, color: [u8; 3], variance: u8, density: u8) -> Props {
    Props {
        name,
        kind: Kind::Gas,
        color,
        variance,
        density,
        hardness: 0,
        ..BASE
    }
}

static PROPS: [Props; Material::COUNT] = [
    Props {
        name: "Empty",
        kind: Kind::Empty,
        density: 0,
        hardness: 0,
        ..BASE
    },
    solid("Stone", [110, 108, 116], 18, 40),
    solid("Dirt", [112, 78, 52], 16, 14),
    Props {
        flammability: 12,
        ..solid("Grass", [70, 140, 58], 20, 10)
    },
    Props {
        flammability: 20,
        ..solid("Wood", [120, 84, 46], 14, 24)
    },
    powder("Sand", [214, 190, 120], 22, 150, 6),
    powder("Ash", [90, 88, 86], 16, 90, 3),
    Props {
        conductive: true,
        ..liquid("Water", [48, 110, 210], 10, 100, 6)
    },
    Props {
        flammability: 60,
        ..liquid("Oil", [60, 44, 30], 8, 80, 4)
    },
    Props {
        heat: 4,
        light: [255, 120, 30],
        ..liquid("Lava", [230, 90, 20], 30, 140, 2)
    },
    Props {
        kind: Kind::Fire,
        density: 1,
        hardness: 0,
        heat: 3,
        light: [255, 150, 50],
        ..solid("Fire", [255, 140, 30], 60, 0)
    },
    gas("Smoke", [60, 60, 64], 12, 2),
    Props {
        heat: 1,
        ..gas("Steam", [200, 210, 220], 12, 3)
    },
    powder("Gravel", [118, 110, 102], 34, 160, 10),
    Props {
        acid_resistant: true,
        light: [120, 60, 200],
        ..solid("Crystal", [170, 96, 230], 40, 70)
    },
    Props {
        flammability: 50,
        growth: 120,
        light: [40, 160, 80],
        ..solid("Fungus", [90, 200, 120], 30, 6)
    },
    Props {
        corrosive: 50,
        light: [30, 70, 10],
        ..liquid("Acid", [120, 230, 60], 20, 105, 4)
    },
    Props {
        flammability: 255,
        ..gas("Gas", [150, 170, 90], 16, 1)
    },
    Props {
        acid_resistant: true,
        ..solid("Obsidian", [40, 28, 54], 12, 120)
    },
    Props {
        heat: -2,
        ..solid("Ice", [170, 215, 240], 14, 20)
    },
    Props {
        heat: 4,
        conductive: true,
        light: [160, 90, 40],
        ..liquid("Metal", [244, 178, 96], 24, 200, 1)
    },
    Props {
        flammability: 200,
        explosive: 9,
        ..solid("Explosive", [180, 46, 38], 20, 18)
    },
    Props {
        value: 1,
        light: [20, 90, 110],
        ..solid("Shard Vein", [52, 58, 74], 16, 45)
    },
    Props {
        acid_resistant: true,
        ..solid("Core Shell", [70, 22, 26], 12, INDESTRUCTIBLE)
    },
    Props {
        kind: Kind::Energy,
        density: 0,
        hardness: 0,
        light: [140, 200, 255],
        ..solid("Spark", [190, 225, 255], 40, 0)
    },
    Props {
        heat: 1,
        ..solid("Basalt", [72, 58, 56], 14, 55)
    },
    Props {
        heat: 1,
        conductive: true,
        ..solid("Ferrite", [92, 92, 104], 20, 80)
    },
    solid("Brick", [138, 82, 62], 14, 45),
    Props {
        heat: 1,
        ..solid("Vent", [84, 72, 70], 10, 60)
    },
    Props {
        heat: -3,
        light: [60, 110, 140],
        ..solid("Frost", [210, 242, 255], 10, 20)
    },
];

impl Material {
    pub const COUNT: usize = 30;

    /// Materials a sandbox player can place, in hotbar order.
    pub const PLACEABLE: [Material; 9] = [
        Material::Sand,
        Material::Water,
        Material::Stone,
        Material::Wood,
        Material::Oil,
        Material::Fire,
        Material::Lava,
        Material::Acid,
        Material::Gas,
    ];

    pub fn from_u8(v: u8) -> Material {
        if (v as usize) < Self::COUNT {
            // SAFETY: Material is repr(u8) with contiguous discriminants 0..COUNT.
            unsafe { std::mem::transmute::<u8, Material>(v) }
        } else {
            Material::Empty
        }
    }

    #[inline]
    pub fn props(self) -> &'static Props {
        &PROPS[self as usize]
    }

    #[inline]
    pub fn kind(self) -> Kind {
        self.props().kind
    }

    /// Whether a player collides with this material.
    #[inline]
    pub fn is_solid_for_player(self) -> bool {
        matches!(self.kind(), Kind::Solid | Kind::Powder)
    }

    /// Whether flowing materials can occupy this cell (air, gas, fire, sparks).
    #[inline]
    pub fn is_open(self) -> bool {
        matches!(self.kind(), Kind::Empty | Kind::Gas | Kind::Fire | Kind::Energy)
    }

    /// Initial `life` for new cells. For gravel, [`GRAVEL_SETTLED`] means it
    /// stays put until something nearby changes.
    pub fn initial_life(self) -> u8 {
        match self {
            Material::Fire => 40,
            Material::Smoke => 120,
            Material::Steam => 160,
            Material::Spark => 10,
            Material::Gravel => GRAVEL_SETTLED,
            Material::Frost => 12,
            _ => 0,
        }
    }

    /// Gases with no lifetime (they never dissipate on their own).
    pub fn is_persistent_gas(self) -> bool {
        self == Material::Gas
    }
}

/// Gravel `life` value meaning "settled": wedged in place until disturbed.
pub const GRAVEL_SETTLED: u8 = 255;
/// Charge given to a conductor touched by a spark; decays by one per cell travelled.
pub const MAX_CHARGE: u8 = 24;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_u8_roundtrips() {
        for i in 0..Material::COUNT as u8 {
            assert_eq!(Material::from_u8(i) as u8, i);
        }
        assert_eq!(Material::from_u8(200), Material::Empty);
    }

    #[test]
    fn props_match_variants() {
        assert_eq!(Material::Frost.props().name, "Frost");
        assert_eq!(Material::CoreShell.props().hardness, INDESTRUCTIBLE);
        assert!(Material::ShardVein.props().value > 0 && Material::ShardVein.is_solid_for_player());
    }
}
