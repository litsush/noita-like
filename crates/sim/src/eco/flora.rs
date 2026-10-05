//! Plant life: the food at the bottom of every food chain.

use super::biome::{Biome, BiomeKind, HEIGHT, WIDTH};
use super::math::{Rgb, V2, hsv, mix, to_hsv, v2};
use super::names;
use crate::material::Material;
use crate::rng::Rng;
use crate::world::World;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GrowthForm {
    Stalk,
    Bush,
    Tuft,
    Bulb,
    Vine,
    Kelp,
    Crystal,
    Cap,
    Pod,
    Reed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Substrate {
    /// On top of soil, growing up.
    Ground,
    /// Hanging from a ceiling or overhang.
    Ceiling,
    /// On the bottom of a body of water.
    Seabed,
    /// Hanging from tree canopies.
    Canopy,
    /// On rock near lava, heat-proof.
    LavaRock,
}

impl Substrate {
    pub fn label(self) -> &'static str {
        match self {
            Substrate::Ground => "grows on open ground",
            Substrate::Ceiling => "hangs from ceilings and overhangs",
            Substrate::Seabed => "grows underwater",
            Substrate::Canopy => "hangs from tree canopies",
            Substrate::LavaRock => "grows on hot rock beside lava",
        }
    }
}

#[derive(Clone, Debug)]
pub struct FloraSpecies {
    pub name: String,
    pub form: GrowthForm,
    pub substrate: Substrate,
    pub stem: Rgb,
    pub leaf: Rgb,
    pub fruit: Rgb,
    pub glow: Option<Rgb>,
    /// Height when fully grown, in cells.
    pub height: f32,
    pub growth_rate: f32,
    pub fruit_rate: f32,
    pub max_fruit: u8,
    /// Satiety gained per fruit.
    pub nutrition: f32,
    pub fireproof: bool,
    /// Seeds per minute when mature.
    pub spread: f32,
}

#[derive(Clone, Debug)]
pub struct Plant {
    pub id: u32,
    pub flora: usize,
    /// The open cell the plant grows from.
    pub x: i32,
    pub y: i32,
    /// -1 grows upwards, +1 hangs down.
    pub dir: i32,
    pub growth: f32,
    pub fruit: f32,
    /// Shape variation.
    pub seed: u32,
    pub dead: bool,
    pub check_timer: f32,
    pub seed_timer: f32,
    /// Recently chewed; shown by the renderer.
    pub bitten: f32,
}

impl Plant {
    pub fn base(&self) -> V2 {
        v2(
            self.x as f32 + 0.5,
            self.y as f32 + if self.dir < 0 { 1.0 } else { 0.0 },
        )
    }

    pub fn length(&self, fl: &FloraSpecies) -> f32 {
        (fl.height * (0.25 + 0.75 * self.growth)).max(2.0)
    }

    pub fn tip(&self, fl: &FloraSpecies) -> V2 {
        self.base() + v2(0.0, self.dir as f32 * self.length(fl))
    }

    /// Where an animal goes to eat.
    pub fn food_point(&self, fl: &FloraSpecies) -> V2 {
        self.base().lerp(self.tip(fl), 0.65)
    }

    /// Food left on the plant, in satiety units.
    pub fn food(&self, fl: &FloraSpecies) -> f32 {
        self.fruit.floor() * fl.nutrition + (self.growth - 0.2).max(0.0) * fl.nutrition * 0.8
    }

    /// Takes a bite. Returns satiety gained.
    pub fn eat(&mut self, fl: &FloraSpecies, bite: f32) -> f32 {
        self.bitten = 0.4;
        if self.fruit >= 1.0 {
            let n = bite.min(self.fruit.floor());
            self.fruit -= n;
            return n * fl.nutrition;
        }
        // Leaves can be grazed down to the stem, never past it.
        let take = (0.12 * bite).min(self.growth - 0.2).max(0.0);
        self.growth -= take;
        take * fl.nutrition * 2.0
    }
}

fn form_noun(form: GrowthForm, rng: &mut Rng) -> &'static str {
    let options: &[&str] = match form {
        GrowthForm::Stalk => &["stalk", "spire", "wand"],
        GrowthForm::Bush => &["bramble", "thicket", "puffbush"],
        GrowthForm::Tuft => &["tuft", "frond", "whisker-grass"],
        GrowthForm::Bulb => &["bulbwort", "gourd", "bladder"],
        GrowthForm::Vine => &["lantern vine", "drip-vine", "hangroot"],
        GrowthForm::Kelp => &["kelp", "ribbonweed", "streamer"],
        GrowthForm::Crystal => &["emberglass", "heatbloom", "slagflower"],
        GrowthForm::Cap => &["cap", "shelf", "parasol"],
        GrowthForm::Pod => &["pod", "seedsack", "dangle-fruit"],
        GrowthForm::Reed => &["reed", "rush", "spindle"],
    };
    rng.pick(options)
}

pub fn make_flora(rng: &mut Rng, biome: &Biome, form: GrowthForm, substrate: Substrate) -> FloraSpecies {
    let leaf_base = biome.palette.mat(Material::Grass);
    let (lh, _, _) = to_hsv(leaf_base);
    let leaf = hsv(
        lh + rng.range(-40.0, 40.0),
        rng.range(0.45, 0.8),
        rng.range(0.5, 0.8),
    );
    let stem = mix(leaf, biome.palette.mat(Material::Wood), rng.range(0.3, 0.7));
    let fruit = hsv(
        lh + 180.0 + rng.range(-60.0, 60.0),
        rng.range(0.6, 0.95),
        rng.range(0.75, 1.0),
    );
    let dark = biome.darkness > 0.5;
    let glow = if form == GrowthForm::Crystal || rng.prob(if dark { 0.7 } else { 0.12 }) {
        Some(hsv(rng.range(0.0, 360.0), 0.6, 1.0))
    } else {
        None
    };
    let height = match form {
        GrowthForm::Stalk => rng.range(10.0, 20.0),
        GrowthForm::Bush => rng.range(5.0, 9.0),
        GrowthForm::Tuft => rng.range(3.0, 6.0),
        GrowthForm::Bulb => rng.range(5.0, 9.0),
        GrowthForm::Vine => rng.range(10.0, 26.0),
        GrowthForm::Kelp => rng.range(14.0, 32.0),
        GrowthForm::Crystal => rng.range(5.0, 11.0),
        GrowthForm::Cap => rng.range(6.0, 14.0),
        GrowthForm::Pod => rng.range(4.0, 8.0),
        GrowthForm::Reed => rng.range(12.0, 22.0),
    };
    FloraSpecies {
        name: format!("{} {}", names::word(rng), form_noun(form, rng)),
        form,
        substrate,
        stem,
        leaf,
        fruit,
        glow,
        height,
        growth_rate: rng.range(0.025, 0.06),
        fruit_rate: rng.range(0.04, 0.12),
        max_fruit: rng.int(2, 5) as u8,
        nutrition: rng.range(0.12, 0.22),
        fireproof: matches!(form, GrowthForm::Crystal) || substrate == Substrate::LavaRock,
        spread: rng.range(1.5, 4.0),
    }
}

/// The plants a biome starts with.
pub fn biome_flora(rng: &mut Rng, biome: &Biome) -> Vec<FloraSpecies> {
    use GrowthForm::*;
    use Substrate::*;
    let ground_forms = [Stalk, Bush, Tuft, Bulb, Cap, Reed];
    let mut picks: Vec<(GrowthForm, Substrate)> = match biome.kind {
        BiomeKind::Verdant => vec![(*rng.pick(&[Stalk, Bush, Bulb]), Ground), (Tuft, Ground)],
        BiomeKind::FungalHollow => vec![(Cap, Ground), (Vine, Ceiling), (Tuft, Ground)],
        BiomeKind::VolcanicRift => vec![(Crystal, LavaRock), (*rng.pick(&[Tuft, Bulb]), Ground)],
        BiomeKind::Archipelago => vec![(Kelp, Seabed), (*rng.pick(&ground_forms), Ground)],
        BiomeKind::AcidMarsh => vec![(Reed, Ground), (*rng.pick(&[Bulb, Cap, Bush]), Ground)],
        BiomeKind::DuneSea => vec![(Bulb, Ground), (Tuft, Ground)],
        BiomeKind::SpireCanyon => vec![(Vine, Ceiling), (*rng.pick(&[Bush, Stalk, Tuft]), Ground)],
        BiomeKind::SkyIsles => vec![(*rng.pick(&ground_forms), Ground), (Vine, Ceiling)],
    };
    if !biome.trees.is_empty() && rng.prob(0.6) {
        picks.push((Pod, Canopy));
    }
    if biome.water > 0.02 && biome.kind != BiomeKind::Archipelago && rng.prob(0.5) {
        picks.push((Kelp, Seabed));
    }
    picks.iter().map(|&(f, s)| make_flora(rng, biome, f, s)).collect()
}

fn is_soil(m: Material) -> bool {
    matches!(
        m,
        Material::Dirt | Material::Grass | Material::Sand | Material::Ash | Material::Nest
    )
}

/// Whether a plant of `fl` can grow from the open cell (x, y). Returns the
/// growth direction.
pub fn valid_spot(world: &World, fl: &FloraSpecies, x: i32, y: i32) -> Option<i32> {
    if !world.in_bounds(x, y) || y < 2 || y >= HEIGHT as i32 - 2 {
        return None;
    }
    let here = world.material(x, y);
    let below = world.material(x, y + 1);
    let above = world.material(x, y - 1);
    let open = here == Material::Empty;
    match fl.substrate {
        Substrate::Ground => {
            (open && is_soil(below) && world.material(x, y - 1) == Material::Empty).then_some(-1)
        }
        Substrate::Ceiling => (open
            && matches!(above, Material::Stone | Material::Dirt | Material::Grass)
            && world.material(x, y + 1) == Material::Empty)
            .then_some(1),
        Substrate::Seabed => (here == Material::Water
            && below.is_solid_for_player()
            && world.material(x, y - 3) == Material::Water)
            .then_some(-1),
        Substrate::Canopy => {
            (open && above == Material::Leaf && world.material(x, y + 1) == Material::Empty).then_some(1)
        }
        Substrate::LavaRock => {
            if !(open && below.is_solid_for_player()) {
                return None;
            }
            let near_lava = (-14..=14).step_by(2).any(|dx| {
                (-6..=10)
                    .step_by(2)
                    .any(|dy| world.material(x + dx, y + dy) == Material::Lava)
            });
            near_lava.then_some(-1)
        }
    }
}

/// Finds a random valid spot near `around` (or anywhere when None).
pub fn find_spot(
    world: &World,
    fl: &FloraSpecies,
    rng: &mut Rng,
    around: Option<(i32, i32)>,
    tries: usize,
) -> Option<(i32, i32, i32)> {
    for _ in 0..tries {
        let x = match around {
            Some((ax, _)) => ax + rng.int(-40, 40),
            None => rng.int(4, WIDTH as i32 - 5),
        };
        if x < 2 || x >= WIDTH as i32 - 2 {
            continue;
        }
        // Scan the column for candidate cells; pick one of them at random.
        let (y0, y1) = match around {
            Some((_, ay)) => ((ay - 40).max(2), (ay + 40).min(HEIGHT as i32 - 3)),
            None => (2, HEIGHT as i32 - 3),
        };
        let mut found = Vec::new();
        for y in y0..y1 {
            if let Some(dir) = valid_spot(world, fl, x, y) {
                found.push((x, y, dir));
            }
        }
        if !found.is_empty() {
            return Some(*rng.pick(&found));
        }
    }
    None
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hazard {
    Burnt,
    Dissolved,
    Buried,
}

/// Whatever is killing the plant, if anything.
pub fn plant_hazard(world: &World, fl: &FloraSpecies, p: &Plant) -> Option<Hazard> {
    let base = p.base();
    let tip = p.tip(fl);
    for i in 0..4 {
        let q = base.lerp(tip, i as f32 / 3.0);
        let (x, y) = q.cell();
        for (dx, dy) in [(0, 0), (1, 0), (-1, 0)] {
            let m = world.material(x + dx, y + dy);
            match m {
                Material::Fire | Material::Lava if !fl.fireproof => return Some(Hazard::Burnt),
                Material::Acid => return Some(Hazard::Dissolved),
                _ => {}
            }
            // Buried by falling sand or a built wall.
            if i == 0 && dx == 0 && m == Material::Sand && fl.substrate != Substrate::Seabed {
                return Some(Hazard::Buried);
            }
        }
    }
    None
}
