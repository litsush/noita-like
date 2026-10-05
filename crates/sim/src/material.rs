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
}

#[repr(u8)]
#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Default, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum Material {
    #[default]
    Empty = 0,
    Stone,
    Dirt,
    Grass,
    Wood,
    Leaves,
    Sand,
    Ash,
    Water,
    Lava,
    Fire,
    Smoke,
    Steam,
    Gravel,
    Basalt,
    Obsidian,
    Ice,
    Crystal,
    Fungus,
    Moss,
    Spore,
    IronOre,
    CopperOre,
    SiliconOre,
    GoldOre,
    TitaniumOre,
    XeniteOre,
    AuroriumOre,
    /// Dome shell: a membrane that stops cells and creatures but lets
    /// players and robots through.
    Glass,
    /// Dome floors and foundations.
    Plating,
    /// Walls of the old ruins.
    Ruin,
    /// The edge of the world.
    Bedrock,
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
    /// Heat given off (negative = cold). Warms or chills players nearby and melts ice.
    pub heat: i8,
    /// Light emitted, as RGB intensity.
    pub light: [u8; 3],
    /// Chance out of 255 per random tick to spread into a free neighbour.
    pub growth: u8,
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
    light: [0, 0, 0],
    growth: 0,
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

const fn ore(name: &'static str, color: [u8; 3], hardness: u8, light: [u8; 3]) -> Props {
    Props {
        name,
        color,
        variance: 14,
        hardness,
        light,
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
    solid("Stone", [92, 96, 108], 16, 40),
    solid("Dirt", [96, 72, 58], 16, 14),
    Props {
        flammability: 10,
        growth: 90,
        ..solid("Grass", [58, 132, 96], 22, 10)
    },
    Props {
        flammability: 18,
        ..solid("Wood", [104, 76, 60], 14, 24)
    },
    Props {
        flammability: 30,
        ..solid("Leaves", [46, 148, 122], 30, 5)
    },
    powder("Sand", [196, 178, 132], 22, 150, 6),
    powder("Ash", [90, 88, 86], 16, 90, 3),
    liquid("Water", [44, 112, 168], 10, 100, 6),
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
    powder("Gravel", [108, 104, 104], 34, 160, 10),
    Props {
        heat: 1,
        ..solid("Basalt", [62, 58, 70], 14, 60)
    },
    solid("Obsidian", [36, 28, 52], 12, 120),
    Props {
        heat: -2,
        ..solid("Ice", [170, 215, 240], 14, 20)
    },
    Props {
        light: [60, 150, 170],
        ..solid("Crystal", [96, 200, 214], 40, 70)
    },
    Props {
        flammability: 40,
        growth: 120,
        light: [70, 150, 60],
        ..solid("Fungus", [150, 214, 96], 30, 6)
    },
    Props {
        flammability: 14,
        ..solid("Moss", [64, 118, 74], 26, 6)
    },
    Props {
        flammability: 120,
        light: [40, 24, 50],
        ..gas("Spore Gas", [170, 110, 190], 16, 1)
    },
    ore("Iron Ore", [124, 132, 150], 48, [0, 0, 0]),
    ore("Copper Ore", [186, 112, 70], 46, [0, 0, 0]),
    ore("Silicon Ore", [198, 202, 214], 50, [10, 10, 14]),
    ore("Gold Ore", [222, 180, 70], 56, [24, 18, 4]),
    ore("Titanium Ore", [170, 196, 222], 80, [8, 12, 18]),
    ore("Xenite Ore", [214, 74, 190], 100, [90, 20, 80]),
    ore("Aurorium Ore", [90, 232, 210], 130, [40, 120, 110]),
    solid("Glass", [150, 196, 204], 6, INDESTRUCTIBLE),
    solid("Plating", [82, 90, 100], 8, INDESTRUCTIBLE),
    solid("Ruin", [104, 96, 92], 18, 70),
    solid("Bedrock", [26, 24, 34], 10, INDESTRUCTIBLE),
];

impl Material {
    pub const COUNT: usize = 32;

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

    /// Whether a player collides with this material. Dome glass is a
    /// membrane that players walk through, and foliage brushes aside.
    #[inline]
    pub fn is_solid_for_player(self) -> bool {
        matches!(self.kind(), Kind::Solid | Kind::Powder)
            && !matches!(self, Material::Glass | Material::Leaves)
    }

    /// Whether a creature collides with this material (dome glass keeps them out).
    #[inline]
    pub fn is_solid_for_creature(self) -> bool {
        matches!(self.kind(), Kind::Solid | Kind::Powder) && self != Material::Leaves
    }

    /// Whether flowing materials can occupy this cell (air, gas, fire).
    #[inline]
    pub fn is_open(self) -> bool {
        matches!(self.kind(), Kind::Empty | Kind::Gas | Kind::Fire)
    }

    pub fn is_ore(self) -> bool {
        (Material::IronOre as u8..=Material::AuroriumOre as u8).contains(&(self as u8))
    }

    /// Initial `life` for new cells. For gravel, [`GRAVEL_SETTLED`] means it
    /// stays put until something nearby changes.
    pub fn initial_life(self) -> u8 {
        match self {
            Material::Fire => 40,
            Material::Smoke => 120,
            Material::Steam => 160,
            Material::Gravel => GRAVEL_SETTLED,
            _ => 0,
        }
    }

    /// Gases with no lifetime (they never dissipate on their own).
    pub fn is_persistent_gas(self) -> bool {
        self == Material::Spore
    }
}

/// Gravel `life` value meaning "settled": wedged in place until disturbed.
pub const GRAVEL_SETTLED: u8 = 255;

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
        assert_eq!(Material::Bedrock.props().name, "Bedrock");
        assert_eq!(Material::Glass.props().hardness, INDESTRUCTIBLE);
        assert!(!Material::Glass.is_solid_for_player() && Material::Glass.is_solid_for_creature());
        assert!(Material::IronOre.is_ore() && Material::AuroriumOre.is_ore() && !Material::Glass.is_ore());
    }
}
