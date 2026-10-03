/// How a material behaves in the simulation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Empty,
    /// Never moves on its own (terrain, placed blocks).
    Solid,
    /// Falls and piles up (sand, ash).
    Powder,
    /// Falls and spreads sideways.
    Liquid,
    /// Rises and dissipates.
    Gas,
    /// Burns, spreads to flammable neighbours, then dies.
    Fire,
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
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
}

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
}

const fn p(
    name: &'static str,
    kind: Kind,
    density: u8,
    flammability: u8,
    dispersion: u8,
    color: [u8; 3],
    variance: u8,
) -> Props {
    Props {
        name,
        kind,
        density,
        flammability,
        dispersion,
        color,
        variance,
    }
}

static PROPS: [Props; Material::COUNT] = [
    p("Empty", Kind::Empty, 0, 0, 0, [0, 0, 0], 0),
    p("Stone", Kind::Solid, 255, 0, 0, [110, 108, 116], 18),
    p("Dirt", Kind::Solid, 255, 0, 0, [112, 78, 52], 16),
    p("Grass", Kind::Solid, 255, 12, 0, [70, 140, 58], 20),
    p("Wood", Kind::Solid, 255, 20, 0, [120, 84, 46], 14),
    p("Sand", Kind::Powder, 150, 0, 0, [214, 190, 120], 22),
    p("Ash", Kind::Powder, 90, 0, 0, [90, 88, 86], 16),
    p("Water", Kind::Liquid, 100, 0, 6, [48, 110, 210], 10),
    p("Oil", Kind::Liquid, 80, 60, 4, [60, 44, 30], 8),
    p("Lava", Kind::Liquid, 140, 0, 2, [230, 90, 20], 30),
    p("Fire", Kind::Fire, 1, 0, 0, [255, 140, 30], 60),
    p("Smoke", Kind::Gas, 2, 0, 0, [60, 60, 64], 12),
    p("Steam", Kind::Gas, 3, 0, 0, [200, 210, 220], 12),
];

impl Material {
    pub const COUNT: usize = 13;

    /// Materials a player can place, in hotbar order.
    pub const PLACEABLE: [Material; 9] = [
        Material::Sand,
        Material::Water,
        Material::Stone,
        Material::Wood,
        Material::Oil,
        Material::Fire,
        Material::Lava,
        Material::Dirt,
        Material::Steam,
    ];

    pub fn from_u8(v: u8) -> Material {
        use Material::*;
        match v {
            1 => Stone,
            2 => Dirt,
            3 => Grass,
            4 => Wood,
            5 => Sand,
            6 => Ash,
            7 => Water,
            8 => Oil,
            9 => Lava,
            10 => Fire,
            11 => Smoke,
            12 => Steam,
            _ => Empty,
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

    /// Initial `life` for materials that expire.
    pub fn initial_life(self) -> u8 {
        match self {
            Material::Fire => 40,
            Material::Smoke => 120,
            Material::Steam => 160,
            _ => 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_u8_roundtrips() {
        for i in 0..Material::COUNT as u8 {
            assert_eq!(Material::from_u8(i) as u8, i);
        }
    }
}
