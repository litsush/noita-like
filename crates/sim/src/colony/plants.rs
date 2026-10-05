//! Plant species, their genes, and gene splicing.
//!
//! Twenty base species ship with the game. The Gene Splicer crosses any two
//! species into a hybrid whose genes, products and looks come from both
//! parents, so hybrids are new plants and not recolours. Splicing is a pure
//! function of the two parents and the world seed: the same cross always
//! gives the same hybrid.

use serde::{Deserialize, Serialize};

use crate::rng::hash2;

pub type SpeciesId = u16;

/// Highest value of a gene.
pub const GENE_MAX: u8 = 10;
pub const BASE_SPECIES: usize = 20;
/// The battery flower.
pub const VOLTBLOOM: SpeciesId = 0;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Product {
    /// Battery flowers: charge for machines and robots.
    Flower,
    /// Food crops.
    Crop,
    Wood,
    Fiber,
    Fertilizer,
    Gel,
    /// Makes water instead of using it.
    Water,
}

impl Product {
    pub fn label(self) -> &'static str {
        match self {
            Product::Flower => "battery flowers",
            Product::Crop => "food",
            Product::Wood => "wood",
            Product::Fiber => "fiber",
            Product::Fertilizer => "fertilizer",
            Product::Gel => "bio-gel",
            Product::Water => "water",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Habitat {
    Land,
    Aquatic,
}

/// The numeric genes, each 0–10.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Genes {
    /// Oxygen released when mature.
    pub oxygen: u8,
    /// Speed from seed to maturity.
    pub growth: u8,
    /// Units of product per harvest.
    pub yield_: u8,
    /// Charge per harvested flower.
    pub power: u8,
    /// Food value of the crop.
    pub food: u8,
    /// Light needed: low grows only in the dark, high only in light.
    pub light: u8,
    /// Water used (0 = makes its own).
    pub thirst: u8,
    /// Survives outdoors and cold.
    pub hardy: u8,
}

impl Genes {
    pub const NAMES: [&'static str; 8] = [
        "Oxygen", "Growth", "Yield", "Power", "Food", "Light", "Thirst", "Hardy",
    ];

    pub fn as_array(&self) -> [u8; 8] {
        [
            self.oxygen,
            self.growth,
            self.yield_,
            self.power,
            self.food,
            self.light,
            self.thirst,
            self.hardy,
        ]
    }

    pub fn from_array(a: [u8; 8]) -> Genes {
        Genes {
            oxygen: a[0],
            growth: a[1],
            yield_: a[2],
            power: a[3],
            food: a[4],
            light: a[5],
            thirst: a[6],
            hardy: a[7],
        }
    }
}

/// Appearance genes that drive the procedural drawing.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Looks {
    /// 0–10: short ground cover to tall stalks.
    pub height: u8,
    /// 0–10: straight to strongly curved or drooping stems.
    pub curve: u8,
    /// Leaf style, see [`LEAF_STYLES`].
    pub leaf: u8,
    /// 0–10: sparse to bushy.
    pub leaves: u8,
    /// Bloom style, see [`BLOOM_STYLES`].
    pub bloom: u8,
    /// Hue (0–255 around the colour wheel) of the foliage and of the bloom.
    pub hue_leaf: u8,
    pub hue_bloom: u8,
    /// 0–10: how much it glows.
    pub glow: u8,
}

/// Leaf styles: blade, frond, round, needle, ribbon, cap (mushroom).
pub const LEAF_STYLES: u8 = 6;
/// Bloom styles: none, cell (battery), bell, star, pod, orb, spike.
pub const BLOOM_STYLES: u8 = 7;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Species {
    pub id: SpeciesId,
    pub name: String,
    /// Name parts used when naming hybrids ("Volt" + "bloom").
    pub head: String,
    pub tail: String,
    pub genes: Genes,
    pub looks: Looks,
    pub product: Option<Product>,
    /// Hybrids can carry a second product at reduced yield.
    pub second: Option<Product>,
    pub habitat: Habitat,
    /// Speeds up the other planters in its dome.
    pub pollinator: bool,
    pub parents: Option<(SpeciesId, SpeciesId)>,
    /// 0 for base species, parents' highest + 1 for hybrids.
    pub generation: u8,
}

/// Where a planter stands, which decides which species thrive in it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Bed {
    /// Green Dome and the starter dome: bright.
    Light,
    /// Dark Dome.
    Dark,
    /// Aqua Dome tank.
    Water,
    /// Planted outside.
    Outdoors,
}

/// Seconds from seed to maturity for a plant with growth 5 at full speed.
const BASE_GROW_SECS: f32 = 420.0;

impl Species {
    /// How well the species grows in a bed, 0–1 (0 = it won't grow there).
    pub fn fit(&self, bed: Bed) -> f32 {
        let light = self.genes.light;
        match (bed, self.habitat) {
            (Bed::Water, Habitat::Aquatic) => 1.0,
            (Bed::Water, Habitat::Land) | (_, Habitat::Aquatic) => 0.0,
            (Bed::Light, _) => match light {
                0..=1 => 0.0,
                2..=3 => 0.5,
                _ => 1.0,
            },
            (Bed::Dark, _) => match light {
                0..=3 => 1.0,
                4..=6 => 0.5,
                _ => 0.0,
            },
            (Bed::Outdoors, _) => {
                if light <= 1 {
                    0.0
                } else {
                    self.genes.hardy as f32 / 10.0
                }
            }
        }
    }

    /// Growth (0–1) gained per second at full speed.
    pub fn growth_rate(&self) -> f32 {
        (0.4 + self.genes.growth as f32 * 0.12) / BASE_GROW_SECS
    }

    /// Moisture (0–1) used per day.
    pub fn thirst_per_day(&self) -> f32 {
        if self.genes.thirst == 0 || self.habitat == Habitat::Aquatic {
            0.0
        } else {
            0.25 + self.genes.thirst as f32 * 0.11
        }
    }

    /// Oxygen units per day from one mature plant.
    pub fn oxygen_per_day(&self) -> f32 {
        self.genes.oxygen as f32
    }

    /// Units of the main product per harvest.
    pub fn harvest_count(&self) -> u32 {
        1 + self.genes.yield_ as u32 / 2
    }

    /// Charge in one harvested flower (0 if it isn't a battery plant).
    pub fn charge(&self) -> f32 {
        if self.yields(Product::Flower) {
            self.genes.power as f32 * 16.0 + 4.0
        } else {
            0.0
        }
    }

    /// Food value of one harvested crop.
    pub fn food_value(&self) -> f32 {
        if self.yields(Product::Crop) {
            self.genes.food as f32
        } else {
            0.0
        }
    }

    pub fn yields(&self, p: Product) -> bool {
        self.product == Some(p) || self.second == Some(p)
    }

    /// Litres of water a mature plant makes per day (Dew Bulb and its hybrids).
    pub fn water_per_day(&self) -> f32 {
        if self.yields(Product::Water) {
            4.0 + self.genes.yield_ as f32 * 3.0
        } else {
            0.0
        }
    }

    /// Sale value of one harvested crop or flower.
    pub fn value(&self) -> u32 {
        let g = &self.genes;
        2 + (g.food as u32 * 3 + g.power as u32 * 2) / 2
    }
}

struct Base {
    name: &'static str,
    head: &'static str,
    tail: &'static str,
    /// oxygen, growth, yield, power, food, light, thirst, hardy
    genes: [u8; 8],
    /// height, curve, leaf, leaves, bloom, hue_leaf, hue_bloom, glow
    looks: [u8; 8],
    product: Option<Product>,
    habitat: Habitat,
    pollinator: bool,
}

const fn base(
    name: &'static str,
    head: &'static str,
    tail: &'static str,
    genes: [u8; 8],
    looks: [u8; 8],
    product: Option<Product>,
) -> Base {
    Base {
        name,
        head,
        tail,
        genes,
        looks,
        product,
        habitat: Habitat::Land,
        pollinator: false,
    }
}

use Product::*;

#[rustfmt::skip]
static BASES: [Base; BASE_SPECIES] = [
    //                                              oxy gro yld pow fod lgt thr hdy    hgt crv leaf lvs blm hueL hueB glow
    base("Voltbloom", "Volt", "bloom",             [2, 6, 4, 6, 0, 6, 4, 6],          [5, 2, 0, 4, 1, 110, 130, 7], Some(Flower)),
    base("Breathfern", "Breath", "fern",           [7, 7, 0, 0, 0, 5, 5, 4],          [6, 5, 1, 9, 0, 118, 118, 1], None),
    base("Ration Root", "Ration", "root",          [2, 5, 5, 0, 4, 6, 5, 4],          [3, 1, 2, 5, 4, 92, 24, 0], Some(Crop)),
    base("Amber Wheat", "Amber", "wheat",          [2, 8, 4, 0, 3, 7, 4, 3],          [6, 1, 0, 6, 6, 60, 34, 0], Some(Crop)),
    base("Sugar Reed", "Sugar", "reed",            [3, 6, 4, 0, 5, 6, 8, 2],          [8, 2, 4, 5, 6, 100, 220, 0], Some(Crop)),
    base("Pepper Pod", "Pepper", "pod",            [1, 4, 2, 0, 7, 8, 5, 2],          [4, 3, 2, 6, 4, 104, 4, 1], Some(Crop)),
    base("Starfruit Tree", "Star", "fruit",        [4, 2, 5, 0, 8, 7, 6, 5],          [9, 3, 2, 8, 3, 124, 40, 2], Some(Crop)),
    base("Ironbark Sapling", "Iron", "bark",       [3, 3, 6, 0, 0, 5, 3, 7],          [10, 1, 3, 6, 0, 138, 138, 0], Some(Wood)),
    base("Fiber Vine", "Fiber", "vine",            [2, 6, 6, 0, 0, 5, 4, 5],          [7, 9, 4, 7, 0, 96, 96, 0], Some(Fiber)),
    base("Lung Moss", "Lung", "moss",              [5, 3, 0, 0, 0, 3, 2, 6],          [1, 4, 2, 10, 0, 128, 128, 2], None),
    base("Sunspire", "Sun", "spire",               [1, 2, 3, 9, 0, 9, 5, 4],          [9, 0, 3, 3, 6, 70, 36, 8], Some(Flower)),
    base("Dew Bulb", "Dew", "bulb",                [2, 4, 4, 0, 0, 4, 0, 5],          [3, 4, 2, 4, 5, 140, 150, 3], Some(Water)),
    base("Thornshield", "Thorn", "shield",         [3, 4, 0, 0, 0, 5, 2, 10],         [5, 2, 3, 8, 0, 150, 150, 0], None),
    Base { pollinator: true, ..base("Honeycup", "Honey", "cup", [2, 5, 3, 0, 3, 7, 4, 4], [4, 4, 2, 5, 2, 100, 30, 3], Some(Crop)) },
    base("Gel Cactus", "Gel", "cactus",            [1, 3, 4, 0, 0, 7, 1, 8],          [5, 0, 3, 2, 5, 112, 96, 1], Some(Gel)),
    base("Glowcap", "Glow", "cap",                 [1, 5, 4, 0, 4, 1, 4, 4],          [3, 2, 5, 3, 0, 132, 132, 9], Some(Crop)),
    base("Sporebloom", "Spore", "bloom",           [1, 5, 6, 0, 0, 1, 5, 4],          [4, 5, 5, 5, 5, 200, 216, 5], Some(Fertilizer)),
    base("Night Orchid", "Night", "orchid",        [1, 4, 3, 5, 0, 0, 4, 3],          [5, 6, 0, 4, 3, 176, 196, 8], Some(Flower)),
    Base { habitat: Habitat::Aquatic, ..base("Kelp Ribbon", "Kelp", "ribbon", [9, 5, 0, 0, 0, 5, 0, 3], [9, 8, 4, 8, 0, 122, 122, 1], None) },
    Base { habitat: Habitat::Aquatic, ..base("Bubble Lotus", "Bubble", "lotus", [5, 4, 4, 0, 5, 6, 0, 3], [3, 3, 2, 4, 2, 130, 236, 4], Some(Crop)) },
];

fn looks_from(a: [u8; 8]) -> Looks {
    Looks {
        height: a[0],
        curve: a[1],
        leaf: a[2],
        leaves: a[3],
        bloom: a[4],
        hue_leaf: a[5],
        hue_bloom: a[6],
        glow: a[7],
    }
}

/// The twenty base species, in id order.
pub fn base_species() -> Vec<Species> {
    BASES
        .iter()
        .enumerate()
        .map(|(i, b)| Species {
            id: i as SpeciesId,
            name: b.name.to_string(),
            head: b.head.to_string(),
            tail: b.tail.to_string(),
            genes: Genes::from_array(b.genes),
            looks: looks_from(b.looks),
            product: b.product,
            second: None,
            habitat: b.habitat,
            pollinator: b.pollinator,
            parents: None,
            generation: 0,
        })
        .collect()
}

/// Price of a pack of three seeds from Earth.
pub fn seed_pack_price(id: SpeciesId) -> u32 {
    const PRICES: [u32; BASE_SPECIES] = [
        120, 90, 40, 40, 70, 160, 260, 110, 80, 100, 400, 220, 90, 180, 150, 120, 140, 300, 280, 240,
    ];
    PRICES.get(id as usize).copied().unwrap_or(0)
}

/// Blends two hues the short way around the colour wheel.
fn mix_hue(a: u8, b: u8) -> u8 {
    let d = b.wrapping_sub(a) as i8;
    a.wrapping_add((d / 2) as u8)
}

/// Crosses two species. `existing` is used only to keep names unique.
/// `bonus` is added to every numeric gene (the Photosynthesis Hat mod).
/// The result's `id` is left as `SpeciesId::MAX` for the caller to assign.
pub fn splice(seed: u64, a: &Species, b: &Species, bonus: u8, existing: &[Species]) -> Species {
    // Order the parents so A×B and B×A are the same plant.
    let (a, b) = if a.id <= b.id { (a, b) } else { (b, a) };
    let h = hash2(seed ^ 0x5EED_1E55, a.id as i32, b.id as i32);
    let bits = |shift: u32, modulo: u64| (h >> shift) % modulo;

    let (ga, gb) = (a.genes.as_array(), b.genes.as_array());
    let mut genes = [0u8; 8];
    for i in 0..8 {
        let r = hash2(h, i as i32, 7);
        let inherited = match r % 3 {
            0 => ga[i] as i32,
            1 => gb[i] as i32,
            _ => (ga[i] as i32 + gb[i] as i32 + 1) / 2,
        };
        let mutation = ((r >> 8) % 3) as i32 - 1;
        genes[i] = (inherited + mutation + bonus as i32).clamp(0, GENE_MAX as i32) as u8;
    }
    // Light need is the plain average, so land × dark crosses are shade plants.
    genes[5] = (ga[5] + gb[5]).div_ceil(2);
    // Hybrid vigour on one gene (not light, where "more" isn't "better").
    let vigour = [0, 1, 2, 3, 4, 7][bits(20, 6) as usize];
    genes[vigour] = (genes[vigour] + 2).min(GENE_MAX);

    let (first, other) = if bits(24, 2) == 0 {
        (a.product, b.product)
    } else {
        (b.product, a.product)
    };
    let product = first.or(other);
    let second = (bits(26, 4) == 0 && other.is_some() && other != product)
        .then_some(other)
        .flatten();
    // A plant that makes water doesn't need watering.
    if product == Some(Water) || second == Some(Water) {
        genes[6] = 0;
    }
    // A battery plant needs some power gene, a crop some food.
    let needs = |p: Product| product == Some(p) || second == Some(p);
    if needs(Flower) {
        genes[3] = genes[3].max(2);
    }
    if needs(Crop) {
        genes[4] = genes[4].max(2);
    }

    let (la, lb) = (a.looks, b.looks);
    let avg = |x: u8, y: u8, salt: i32| {
        let m = (hash2(h, salt, 11) % 3) as i32 - 1;
        ((x as i32 + y as i32 + 1) / 2 + m).clamp(0, 10) as u8
    };
    let pick = |x: u8, y: u8, salt: u32| if bits(salt, 2) == 0 { x } else { y };
    let looks = Looks {
        height: avg(la.height, lb.height, 1),
        curve: avg(la.curve, lb.curve, 2),
        leaf: pick(la.leaf, lb.leaf, 30),
        leaves: avg(la.leaves, lb.leaves, 3),
        // The bloom comes from whichever parent has one, preferring the
        // parent that didn't supply the leaves.
        bloom: {
            let (x, y) = if bits(30, 2) == 0 {
                (lb.bloom, la.bloom)
            } else {
                (la.bloom, lb.bloom)
            };
            if x != 0 { x } else { y }
        },
        hue_leaf: mix_hue(la.hue_leaf, lb.hue_leaf),
        hue_bloom: pick(la.hue_bloom, lb.hue_bloom, 31),
        glow: avg(la.glow, lb.glow, 4).max(la.glow.min(lb.glow)),
    };

    let genes = Genes::from_array(genes);
    let mut child = Species {
        id: SpeciesId::MAX,
        name: String::new(),
        head: a.head.clone(),
        tail: b.tail.clone(),
        genes,
        looks,
        product,
        second,
        habitat: if a.habitat == Habitat::Aquatic && b.habitat == Habitat::Aquatic {
            Habitat::Aquatic
        } else {
            Habitat::Land
        },
        pollinator: a.pollinator || b.pollinator,
        parents: Some((a.id, b.id)),
        generation: a.generation.max(b.generation) + 1,
    };
    if bits(34, 2) == 1 && a.tail != b.tail {
        child.head = b.head.clone();
        child.tail = a.tail.clone();
    }
    child.name = hybrid_name(&child, existing);
    child
}

fn hybrid_name(s: &Species, existing: &[Species]) -> String {
    let g = &s.genes;
    let prefix = [
        (g.oxygen, "Verdant"),
        (g.power, "Radiant"),
        (g.food, "Hearty"),
        (g.yield_, "Bountiful"),
        (g.growth, "Swift"),
        (g.hardy, "Stubborn"),
        (g.thirst, "Thirsty"),
    ]
    .iter()
    .filter(|(v, _)| *v >= 9)
    .max_by_key(|(v, _)| *v)
    .map(|(_, p)| *p);
    let core = format!("{}{}", s.head, s.tail);
    let base = match prefix {
        Some(p) => format!("{p} {core}"),
        None => core,
    };
    let taken = |n: &str| existing.iter().any(|e| e.name == n);
    if !taken(&base) {
        return base;
    }
    for numeral in ["II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X"] {
        let n = format!("{base} {numeral}");
        if !taken(&n) {
            return n;
        }
    }
    format!("{base} {}", existing.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base_table_is_sane() {
        let all = base_species();
        assert_eq!(all.len(), BASE_SPECIES);
        assert!((15..=20).contains(&all.len()));
        for (i, s) in all.iter().enumerate() {
            assert_eq!(s.id as usize, i);
            assert!(s.genes.as_array().iter().all(|&g| g <= GENE_MAX), "{}", s.name);
            assert!(
                s.looks.leaf < LEAF_STYLES && s.looks.bloom < BLOOM_STYLES,
                "{}",
                s.name
            );
            assert!(seed_pack_price(s.id) > 0);
            // Every species grows somewhere.
            assert!(
                [Bed::Light, Bed::Dark, Bed::Water]
                    .iter()
                    .any(|&b| s.fit(b) > 0.0),
                "{} grows nowhere",
                s.name
            );
        }
        let names: std::collections::HashSet<_> = all.iter().map(|s| &s.name).collect();
        assert_eq!(names.len(), all.len());
        assert_eq!(all[VOLTBLOOM as usize].name, "Voltbloom");
        assert!((all[VOLTBLOOM as usize].charge() - 100.0).abs() < 0.1);
        assert!(all.iter().filter(|s| s.habitat == Habitat::Aquatic).count() >= 2);
        assert!(
            all.iter()
                .filter(|s| s.fit(Bed::Dark) == 1.0 && s.fit(Bed::Light) == 0.0)
                .count()
                >= 3
        );
    }

    #[test]
    fn splicing_is_deterministic_and_symmetric() {
        let all = base_species();
        let x = splice(9, &all[0], &all[1], 0, &all);
        let y = splice(9, &all[1], &all[0], 0, &all);
        assert_eq!(x, y);
        assert_eq!(x.parents, Some((0, 1)));
        assert_eq!(x.generation, 1);
        let other_world = splice(10, &all[0], &all[1], 0, &all);
        assert!(x.genes != other_world.genes || x.looks != other_world.looks || x.name != other_world.name);
    }

    #[test]
    fn hybrids_inherit_from_both_parents() {
        let all = base_species();
        let mut differs_from_both = 0;
        let mut dual = 0;
        let mut total = 0;
        for i in 0..all.len() {
            for j in i + 1..all.len() {
                let (a, b) = (&all[i], &all[j]);
                let c = splice(3, a, b, 0, &all);
                total += 1;
                let (ga, gb, gc) = (a.genes.as_array(), b.genes.as_array(), c.genes.as_array());
                for k in 0..8 {
                    let (lo, hi) = (ga[k].min(gb[k]) as i32, ga[k].max(gb[k]) as i32);
                    // Inherited value ± mutation, plus vigour on one gene.
                    assert!(
                        (gc[k] as i32) >= lo - 1 && (gc[k] as i32) <= hi + 3 || k == 6,
                        "{} x {} gene {k}: {} not near {lo}..{hi}",
                        a.name,
                        b.name,
                        gc[k]
                    );
                }
                assert_eq!(gc[5], (ga[5] + gb[5]).div_ceil(2), "light is the average");
                if gc != ga && gc != gb {
                    differs_from_both += 1;
                }
                // Products come from the parents.
                for p in [c.product, c.second].into_iter().flatten() {
                    assert!(a.product == Some(p) || b.product == Some(p));
                }
                if a.product.is_some() || b.product.is_some() {
                    assert!(c.product.is_some());
                }
                if c.second.is_some() {
                    dual += 1;
                    assert_ne!(c.second, c.product);
                }
                assert_eq!(
                    c.habitat == Habitat::Aquatic,
                    a.habitat == Habitat::Aquatic && b.habitat == Habitat::Aquatic
                );
                assert!(!c.name.is_empty() && c.name != a.name && c.name != b.name);
                assert!(c.name.contains(&c.head) && c.name.to_lowercase().contains(&c.tail));
            }
        }
        assert!(differs_from_both > total * 9 / 10);
        assert!(dual > 5, "some hybrids yield two products ({dual})");
    }

    #[test]
    fn names_stay_unique_and_hybrids_splice_again() {
        let mut all = base_species();
        let mut child = splice(1, &all[0], &all[1], 0, &all);
        child.id = all.len() as SpeciesId;
        let first_name = child.name.clone();
        all.push(child.clone());
        // Same name parts again must not collide.
        let again = hybrid_name(&child, &all);
        assert_ne!(again, first_name);
        // Second generation.
        let mut grandchild = splice(1, &child, &all[2], 0, &all);
        grandchild.id = all.len() as SpeciesId;
        assert_eq!(grandchild.generation, 2);
        assert_eq!(grandchild.parents, Some((2, child.id)));
    }

    #[test]
    fn bonus_raises_genes() {
        let all = base_species();
        let plain = splice(4, &all[2], &all[3], 0, &all);
        let boosted = splice(4, &all[2], &all[3], 1, &all);
        let (p, b) = (plain.genes.as_array(), boosted.genes.as_array());
        assert!((0..8).filter(|&k| k != 5).all(|k| b[k] >= p[k]));
        assert!((0..8).any(|k| b[k] > p[k]));
    }

    #[test]
    fn beds_decide_what_grows() {
        let all = base_species();
        let by = |n: &str| all.iter().find(|s| s.name == n).unwrap();
        assert_eq!(by("Glowcap").fit(Bed::Light), 0.0);
        assert_eq!(by("Glowcap").fit(Bed::Dark), 1.0);
        assert_eq!(by("Amber Wheat").fit(Bed::Dark), 0.0);
        assert_eq!(by("Kelp Ribbon").fit(Bed::Water), 1.0);
        assert_eq!(by("Kelp Ribbon").fit(Bed::Light), 0.0);
        assert_eq!(by("Thornshield").fit(Bed::Outdoors), 1.0);
        assert!(by("Pepper Pod").fit(Bed::Outdoors) < 0.3);
        // A land × dark cross is a shade plant that grows in both domes.
        let shade = splice(2, by("Amber Wheat"), by("Glowcap"), 0, &all);
        assert!(shade.fit(Bed::Light) > 0.0 && shade.fit(Bed::Dark) > 0.0);
        assert_eq!(by("Dew Bulb").thirst_per_day(), 0.0);
        assert!(by("Dew Bulb").water_per_day() > 0.0);
        assert!(by("Sugar Reed").thirst_per_day() > by("Lung Moss").thirst_per_day());
    }
}
