//! Species design: a point-buy sheet of upgrades, and the deterministic
//! translation of that sheet into a drawable, fightable [`Species`] plus
//! the combat numbers ([`Loadout`]) the arena uses.
//!
//! Every upgrade shows on the body: legs are legs, intelligence swells the
//! head, armour grows a shell, echolocation grows ear-paddles, and so on.

use std::f32::consts::{FRAC_PI_2, PI};

use serde::{Deserialize, Serialize};

use crate::eco::biome::Biome;
use crate::eco::genome::{
    Activity, BodyPlan, Colors, Crest, Eye, FleeStyle, Habitat, HuntStyle, LimbKind, LimbTip, Locomotion,
    Mouth, Pattern, Personality, Role, Species, SpitKind, Stats, Teeth, Traits, WingKind, add_legs,
    base_body, env_colour, limb, seg, tapering,
};
use crate::eco::math::{Rgb, hsv, mix, scale, to_hsv};
use crate::eco::names;
use crate::rng::Rng;

/// Points a fresh species starts with.
pub const START_POINTS: u32 = 40;
/// Points gained after every round.
pub const ROUND_POINTS: u32 = 14;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Upgrade {
    // Body
    Size,
    Vitality,
    Regeneration,
    Armor,
    Spines,
    // Movement
    Legs,
    LongLegs,
    Wings,
    Leap,
    Burrow,
    Climb,
    Swim,
    // Senses
    Eyes,
    NightVision,
    Echolocation,
    Feelers,
    // Weapons
    Teeth,
    Claws,
    Arms,
    Tail,
    Horns,
    Venom,
    Spit,
    Tentacles,
    // Defence
    Camouflage,
    Ink,
    ToxicFlesh,
    // Mind
    Intelligence,
    Aggression,
    Caution,
    PackMind,
    // Numbers
    Individuals,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Group {
    Body,
    Movement,
    Senses,
    Weapons,
    Defence,
    Mind,
    Numbers,
}

impl Group {
    pub const ALL: [Group; 7] = [
        Group::Body,
        Group::Movement,
        Group::Senses,
        Group::Weapons,
        Group::Defence,
        Group::Mind,
        Group::Numbers,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Group::Body => "Body",
            Group::Movement => "Movement",
            Group::Senses => "Senses",
            Group::Weapons => "Weapons",
            Group::Defence => "Defence",
            Group::Mind => "Mind",
            Group::Numbers => "Numbers",
        }
    }
}

impl Upgrade {
    pub const ALL: [Upgrade; 32] = [
        Upgrade::Size,
        Upgrade::Vitality,
        Upgrade::Regeneration,
        Upgrade::Armor,
        Upgrade::Spines,
        Upgrade::Legs,
        Upgrade::LongLegs,
        Upgrade::Wings,
        Upgrade::Leap,
        Upgrade::Burrow,
        Upgrade::Climb,
        Upgrade::Swim,
        Upgrade::Eyes,
        Upgrade::NightVision,
        Upgrade::Echolocation,
        Upgrade::Feelers,
        Upgrade::Teeth,
        Upgrade::Claws,
        Upgrade::Arms,
        Upgrade::Tail,
        Upgrade::Horns,
        Upgrade::Venom,
        Upgrade::Spit,
        Upgrade::Tentacles,
        Upgrade::Camouflage,
        Upgrade::Ink,
        Upgrade::ToxicFlesh,
        Upgrade::Intelligence,
        Upgrade::Aggression,
        Upgrade::Caution,
        Upgrade::PackMind,
        Upgrade::Individuals,
    ];
    pub const COUNT: usize = Self::ALL.len();

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|&u| u == self).unwrap()
    }

    pub fn group(self) -> Group {
        use Upgrade::*;
        match self {
            Size | Vitality | Regeneration | Armor | Spines => Group::Body,
            Legs | LongLegs | Wings | Leap | Burrow | Climb | Swim => Group::Movement,
            Eyes | NightVision | Echolocation | Feelers => Group::Senses,
            Teeth | Claws | Arms | Tail | Horns | Venom | Spit | Tentacles => Group::Weapons,
            Camouflage | Ink | ToxicFlesh => Group::Defence,
            Intelligence | Aggression | Caution | PackMind => Group::Mind,
            Individuals => Group::Numbers,
        }
    }

    pub fn label(self) -> &'static str {
        use Upgrade::*;
        match self {
            Size => "Size",
            Vitality => "Vitality",
            Regeneration => "Regeneration",
            Armor => "Armour plating",
            Spines => "Spines",
            Legs => "Legs",
            LongLegs => "Long legs",
            Wings => "Wings",
            Leap => "Spring legs",
            Burrow => "Digging claws",
            Climb => "Grip pads",
            Swim => "Fins",
            Eyes => "Eyes",
            NightVision => "Big eyes",
            Echolocation => "Echolocation",
            Feelers => "Feelers",
            Teeth => "Teeth",
            Claws => "Claws",
            Arms => "Arms",
            Tail => "Tail",
            Horns => "Horns",
            Venom => "Venom",
            Spit => "Spit",
            Tentacles => "Tentacles",
            Camouflage => "Camouflage",
            Ink => "Ink cloud",
            ToxicFlesh => "Toxic flesh",
            Intelligence => "Intelligence",
            Aggression => "Aggression",
            Caution => "Caution",
            PackMind => "Pack mind",
            Individuals => "Individuals",
        }
    }

    /// One line on what it does.
    pub fn hint(self) -> &'static str {
        use Upgrade::*;
        match self {
            Size => "Bigger body: more health and harder hits, slower and easier to see.",
            Vitality => "Tougher flesh: more health on every part.",
            Regeneration => "Wounds close over time. Lost limbs never regrow.",
            Armor => "A shell over the back and body. Blocks damage there, not on the head or limbs.",
            Spines => "Anything that bites or punches you gets hurt back.",
            Legs => {
                "No legs slithers. Two run upright, four run fastest, six and eight are stable and climb well."
            }
            LongLegs => "Faster and higher-jumping, but easier to knock over.",
            Wings => "Insect wings hover, membrane wings turn tight, feathers are fastest.",
            Leap => "Pounces across gaps and onto enemies.",
            Burrow => "Tunnels through soil. Level 2 swims through sand without a trace.",
            Climb => "Clings to walls and ceilings.",
            Swim => "Moves freely in water. Level 2 shrugs off acid too.",
            Eyes => "Each eye adds sight range. None at all means hunting by feel.",
            NightVision => "Large eyes see in the dark.",
            Echolocation => "Senses enemies through walls and camouflage, up close.",
            Feelers => "Feels footsteps through the ground, even when burrowed.",
            Teeth => {
                "Bristles rake fast, flat teeth grind, a beak pierces armour, mandibles hold, canines and shark teeth tear."
            }
            Claws => "Slashing claws on the front legs or hands.",
            Arms => "Arms with gripping hands: punches that knock enemies back.",
            Tail => "A whip stings, a club knocks flat, a spike pierces, a stinger injects venom.",
            Horns => "Charge headlong and gore.",
            Venom => "Bites and stingers poison, dealing damage over time.",
            Spit => {
                "Ranged attack. Web entangles, venom poisons, acid burns through soil, fire sets things alight."
            }
            Tentacles => "Grasping tentacles that hold enemies still.",
            Camouflage => "Blends into the arena. Hard to see while still, invisible to the eyeless.",
            Ink => "Dumps a blinding cloud when hurt.",
            ToxicFlesh => "Poisons anything that bites you.",
            Intelligence => "A bigger brain learns which tactics work and targets weak spots.",
            Aggression => "How readily it attacks rather than circling.",
            Caution => "Retreats and recovers when badly hurt.",
            PackMind => "Individuals coordinate: flank and focus the same target.",
            Individuals => "More of you. Each one is a little smaller.",
        }
    }

    /// Highest level.
    pub fn max(self) -> u8 {
        self.prices().len() as u8 - 1
    }

    /// Total points to be at each level (index 0 is free).
    fn prices(self) -> &'static [u32] {
        use Upgrade::*;
        match self {
            Size => &[0, 4, 8, 12, 16],
            Vitality => &[0, 3, 6, 9, 12],
            Regeneration => &[0, 4, 8, 12],
            Armor => &[0, 4, 8, 12],
            Spines => &[0, 3, 6],
            Legs => &[0, 3, 5, 7, 9],
            LongLegs => &[0, 2, 4, 6],
            Wings => &[0, 5, 6, 7],
            Leap => &[0, 3, 6],
            Burrow => &[0, 4, 8],
            Climb => &[0, 3],
            Swim => &[0, 2, 4],
            Eyes => &[0, 1, 2, 3, 4, 6],
            NightVision => &[0, 3, 6],
            Echolocation => &[0, 6],
            Feelers => &[0, 3],
            Teeth => &[0, 2, 3, 4, 4, 5, 6],
            Claws => &[0, 3, 6, 9],
            Arms => &[0, 4, 8],
            Tail => &[0, 2, 4, 5, 6],
            Horns => &[0, 3, 6],
            Venom => &[0, 4, 8],
            Spit => &[0, 4, 5, 6, 7],
            Tentacles => &[0, 4, 8],
            Camouflage => &[0, 4, 8],
            Ink => &[0, 3],
            ToxicFlesh => &[0, 4],
            Intelligence => &[0, 3, 6, 9, 12],
            Aggression => &[0, 1, 2, 3, 4],
            Caution => &[0, 1, 2, 3],
            PackMind => &[0, 3, 6],
            Individuals => &[0, 6, 13, 21, 30, 40],
        }
    }

    pub fn price(self, level: u8) -> u32 {
        self.prices()[(level as usize).min(self.prices().len() - 1)]
    }

    /// Names for each level when the upgrade is a choice rather than a
    /// ladder; `None` for plain levels.
    pub fn choices(self) -> Option<&'static [&'static str]> {
        use Upgrade::*;
        match self {
            Legs => Some(&["None", "Two", "Four", "Six", "Eight"]),
            Wings => Some(&["None", "Insect", "Membrane", "Feather"]),
            Eyes => Some(&["0", "1", "2", "3", "4", "6"]),
            Teeth => Some(&[
                "None",
                "Bristles",
                "Flat",
                "Beak",
                "Mandibles",
                "Canines",
                "Shark",
            ]),
            Tail => Some(&["None", "Whip", "Club", "Spike", "Stinger"]),
            Spit => Some(&["None", "Web", "Venom", "Acid", "Fire"]),
            _ => None,
        }
    }

    /// Why the upgrade does nothing with this design, if so.
    pub fn requires(self, d: &Design) -> Option<&'static str> {
        use Upgrade::*;
        match self {
            LongLegs | Leap if d.level(Legs) == 0 => Some("needs legs"),
            Claws if d.level(Legs) == 0 && d.level(Arms) == 0 => Some("needs legs or arms"),
            Venom if d.level(Teeth) == 0 && d.level(Tail) != 4 && d.level(Spit) == 0 => {
                Some("needs teeth, a stinger or spit")
            }
            PackMind if d.level(Individuals) == 0 => Some("needs more than one individual"),
            NightVision if d.level(Eyes) == 0 => Some("needs eyes"),
            _ => None,
        }
    }
}

/// A player's species sheet.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Design {
    pub name: String,
    /// Decides colours and the small shape choices that aren't upgrades.
    pub look_seed: u64,
    pub levels: Vec<u8>,
}

impl Design {
    pub fn new(look_seed: u64) -> Design {
        let mut d = Design {
            name: String::new(),
            look_seed,
            levels: vec![0; Upgrade::COUNT],
        };
        // A plain starting animal: four legs, two eyes, flat teeth.
        d.set(Upgrade::Legs, 2);
        d.set(Upgrade::Eyes, 2);
        d.set(Upgrade::Teeth, 2);
        d.set(Upgrade::Aggression, 2);
        d.name = d.auto_name();
        d
    }

    pub fn level(&self, u: Upgrade) -> u8 {
        self.levels.get(u.index()).copied().unwrap_or(0)
    }

    pub fn set(&mut self, u: Upgrade, level: u8) {
        if self.levels.len() < Upgrade::COUNT {
            self.levels.resize(Upgrade::COUNT, 0);
        }
        self.levels[u.index()] = level.min(u.max());
    }

    /// Total points spent.
    pub fn cost(&self) -> u32 {
        Upgrade::ALL.iter().map(|&u| u.price(self.level(u))).sum()
    }

    /// Valid for the wire: every level in range.
    pub fn sanitize(&mut self) {
        if self.levels.len() != Upgrade::COUNT {
            self.levels.resize(Upgrade::COUNT, 0);
        }
        for u in Upgrade::ALL {
            let l = self.level(u).min(u.max());
            self.levels[u.index()] = l;
        }
        self.name = self.name.chars().filter(|c| !c.is_control()).take(28).collect();
        if self.name.trim().is_empty() {
            self.name = self.auto_name();
        }
    }

    /// Whether `other` only adds to this design (evolution never takes away).
    pub fn evolves_from(&self, base: &Design) -> bool {
        Upgrade::ALL.iter().all(|&u| self.level(u) >= base.level(u))
    }

    pub fn count(&self) -> usize {
        self.level(Upgrade::Individuals) as usize + 1
    }

    /// A name from what it is, e.g. "Horned Shark-toothed Strider".
    pub fn auto_name(&self) -> String {
        let mut rng = Rng::new(self.look_seed ^ 0x3A3E);
        let colour = names::colour_word(look_colour(self), &mut rng);
        let mut features: Vec<&str> = Vec::new();
        use Upgrade::*;
        if self.level(Horns) > 0 {
            features.push("Horned");
        }
        if self.level(Armor) > 0 {
            features.push("Shell");
        }
        if self.level(Spines) > 0 {
            features.push("Spine");
        }
        match self.level(Teeth) {
            1 => features.push("Bristle"),
            3 => features.push("Beak"),
            4 => features.push("Mandible"),
            5 => features.push("Fang"),
            6 => features.push("Sawtooth"),
            _ => {}
        }
        if self.level(Eyes) >= 4 {
            features.push("Many-eyed");
        }
        if self.level(Eyes) == 0 {
            features.push("Eyeless");
        }
        if self.level(Tentacles) > 0 {
            features.push("Tendril");
        }
        if self.level(Venom) > 0 {
            features.push("Venom");
        }
        if self.level(Intelligence) >= 3 {
            features.push("Big-brained");
        }
        if self.level(Claws) >= 2 {
            features.push("Talon");
        }
        let feature = if features.is_empty() {
            ""
        } else {
            *rng.pick(&features)
        };
        let noun = if self.level(Wings) > 0 {
            *rng.pick(&["Flitter", "Harrier", "Skyjaw", "Darter"])
        } else if self.level(Burrow) > 0 {
            *rng.pick(&["Delver", "Sinkmaw", "Mole"])
        } else if self.level(Legs) == 0 {
            *rng.pick(&["Coiler", "Wriggler", "Slider"])
        } else if self.level(Legs) == 1 {
            *rng.pick(&["Strider", "Stalker", "Prowler"])
        } else if self.level(Legs) >= 3 {
            *rng.pick(&["Skitter", "Crawler", "Weaver"])
        } else {
            *rng.pick(&["Runner", "Courser", "Snapper", "Brute"])
        };
        if feature.is_empty() {
            format!("{colour} {noun}")
        } else {
            format!("{colour} {feature} {noun}")
        }
    }

    /// Spends `budget` points at random on top of `base`, leaning towards
    /// coherent builds. Used for practice opponents and auto-fill.
    pub fn random(rng: &mut Rng, budget: u32, base: Option<&Design>) -> Design {
        let mut d = base.cloned().unwrap_or_else(|| Design::new(rng.next_u64()));
        if base.is_none() {
            d.look_seed = rng.next_u64();
            // Start from nothing so builds vary.
            for u in Upgrade::ALL {
                d.set(u, 0);
            }
            d.set(Upgrade::Legs, rng.weighted(&[0.15, 0.3, 0.35, 0.1, 0.1]) as u8);
            d.set(
                Upgrade::Eyes,
                rng.weighted(&[0.05, 0.1, 0.5, 0.15, 0.1, 0.1]) as u8,
            );
            d.set(Upgrade::Aggression, rng.int(1, 3) as u8);
        }
        let total = budget + base.map_or(0, |b| b.cost());
        // Weighted wishes: a build theme plus a few extras.
        let themes: [&[Upgrade]; 6] = [
            &[
                Upgrade::Teeth,
                Upgrade::Size,
                Upgrade::Vitality,
                Upgrade::Armor,
                Upgrade::Horns,
            ],
            &[
                Upgrade::Wings,
                Upgrade::Teeth,
                Upgrade::Claws,
                Upgrade::NightVision,
                Upgrade::Leap,
            ],
            &[
                Upgrade::Spit,
                Upgrade::LongLegs,
                Upgrade::Caution,
                Upgrade::Intelligence,
                Upgrade::Eyes,
            ],
            &[
                Upgrade::Individuals,
                Upgrade::PackMind,
                Upgrade::Teeth,
                Upgrade::Claws,
                Upgrade::Aggression,
            ],
            &[
                Upgrade::Burrow,
                Upgrade::Feelers,
                Upgrade::Tentacles,
                Upgrade::Venom,
                Upgrade::Camouflage,
            ],
            &[
                Upgrade::Arms,
                Upgrade::Claws,
                Upgrade::Intelligence,
                Upgrade::Tail,
                Upgrade::Spines,
            ],
        ];
        let theme = themes[rng.int(0, themes.len() as i32 - 1) as usize];
        let mut tries = 0;
        while d.cost() < total && tries < 400 {
            tries += 1;
            let u = if rng.prob(0.6) {
                theme[rng.int(0, theme.len() as i32 - 1) as usize]
            } else {
                Upgrade::ALL[rng.int(0, Upgrade::COUNT as i32 - 1) as usize]
            };
            let l = d.level(u);
            if l >= u.max() || u.requires(&d).is_some() {
                continue;
            }
            // Choices jump straight to a random option.
            let next = if u.choices().is_some() && l == 0 {
                rng.int(1, u.max() as i32) as u8
            } else {
                l + 1
            };
            let delta = u.price(next) - u.price(l);
            if d.cost() + delta <= total {
                d.set(u, next);
            }
        }
        if base.is_none() {
            d.name = d.auto_name();
        }
        d
    }
}

/// Base hue for naming and colouring, from the look seed.
fn look_colour(d: &Design) -> Rgb {
    let mut rng = Rng::new(d.look_seed ^ 0xC01);
    let warning = d.level(Upgrade::Venom) > 0 || d.level(Upgrade::ToxicFlesh) > 0;
    if warning {
        hsv(*rng.pick(&[0.0, 50.0, 300.0, 30.0]), 0.9, 0.95)
    } else {
        hsv(rng.range(0.0, 360.0), rng.range(0.35, 0.8), rng.range(0.5, 0.9))
    }
}

// ---------------------------------------------------------------------------
// Combat numbers

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum WeaponKind {
    Bite,
    Claw,
    Punch,
    Tail,
    Horn,
    Tentacle,
}

impl WeaponKind {
    pub fn label(self) -> &'static str {
        match self {
            WeaponKind::Bite => "bite",
            WeaponKind::Claw => "claw",
            WeaponKind::Punch => "punch",
            WeaponKind::Tail => "tail",
            WeaponKind::Horn => "gore",
            WeaponKind::Tentacle => "grab",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Weapon {
    pub kind: WeaponKind,
    pub damage: f32,
    pub cooldown: f32,
    /// How far from the body centre the strike reaches.
    pub reach: f32,
    /// Impulse on the target per point of damage.
    pub knockback: f32,
    /// Fraction of armour ignored.
    pub pierce: f32,
    pub venom: bool,
    pub grab: bool,
    /// Needs the body to be moving fast (charges).
    pub charge: bool,
}

/// Combat-relevant numbers derived from a design.
#[derive(Clone, Debug)]
pub struct Loadout {
    pub count: usize,
    pub size: f32,
    pub regen: f32,
    pub armor: f32,
    pub spines: f32,
    pub weapons: Vec<Weapon>,
    pub venom: f32,
    pub spit: Option<SpitKind>,
    pub spit_damage: f32,
    pub spit_cooldown: f32,
    pub spit_range: f32,
    /// 0..1
    pub intelligence: f32,
    pub aggression: f32,
    pub caution: f32,
    pub pack: f32,
    pub camouflage: f32,
    pub ink: bool,
    pub toxic: bool,
    pub sight: f32,
    pub night: f32,
    /// Echolocation range, 0 for none.
    pub echo: f32,
    pub tremor: bool,
    pub fly: bool,
    pub dig: bool,
    pub climb: bool,
    pub swim: bool,
    pub leap: f32,
    /// Legs standing on, for stability.
    pub legs: usize,
}

impl Loadout {
    pub fn weapon(&self, kind: WeaponKind) -> Option<&Weapon> {
        self.weapons.iter().find(|w| w.kind == kind)
    }
}

/// Builds every team's species, nudging later teams' colours away from
/// earlier ones so the sides can be told apart at a glance.
pub fn build_teams(designs: &[(String, Design)], biome: Option<&Biome>) -> Vec<(Species, Loadout)> {
    let mut out: Vec<(Species, Loadout)> = Vec::new();
    for (i, (_, d)) in designs.iter().enumerate() {
        let (mut sp, lo) = build_species(d, i, biome);
        let (h, _, _) = to_hsv(sp.colors.base);
        let clash = out.iter().any(|(o, _)| {
            let (oh, _, _) = to_hsv(o.colors.base);
            let diff = (h - oh).rem_euclid(360.0);
            diff.min(360.0 - diff) < 45.0
        });
        if clash {
            let shift = 150.0 + i as f32 * 40.0;
            let rotate = |c: Rgb| {
                let (h, s, v) = to_hsv(c);
                hsv(h + shift, s, v)
            };
            sp.colors.base = rotate(sp.colors.base);
            sp.colors.accent = rotate(sp.colors.accent);
            sp.colors.belly = rotate(sp.colors.belly);
        }
        out.push((sp, lo));
    }
    out
}

/// The mouth the teeth choice implies.
fn mouth_for(teeth: u8) -> (Mouth, Teeth) {
    match teeth {
        0 => (Mouth::Sucker, Teeth::None),
        1 => (Mouth::Jaws, Teeth::Bristles),
        2 => (Mouth::Jaws, Teeth::Flat),
        3 => (Mouth::Beak, Teeth::None),
        4 => (Mouth::Mandibles, Teeth::None),
        5 => (Mouth::Jaws, Teeth::Canines),
        _ => (Mouth::Maw, Teeth::Shark),
    }
}

/// Builds the species for team `team` from a design. `biome` colours
/// camouflaged species to match the arena; `None` uses a neutral palette.
pub fn build_species(d: &Design, team: usize, biome: Option<&Biome>) -> (Species, Loadout) {
    use Upgrade::*;
    let mut rng = Rng::new(d.look_seed ^ 0x5EED);
    let lv = |u: Upgrade| d.level(u) as f32;
    let count = d.count();
    // More mouths, each a little smaller.
    let s = (2.8 + lv(Size) * 1.1) * (1.0 - 0.06 * lv(Individuals));
    let legs_choice = d.level(Legs) as usize;
    let leg_count = [0usize, 2, 4, 6, 8][legs_choice];
    let wings = d.level(Wings);
    let (mouth, teeth) = mouth_for(d.level(Teeth));

    let mut t = Traits {
        fly: wings > 0,
        dig: d.level(Burrow) > 0,
        sand_swim: d.level(Burrow) >= 2,
        climb: d.level(Climb) > 0 || leg_count >= 6 && rng.prob(0.5),
        leap: d.level(Leap) > 0,
        swim: d.level(Swim) > 0,
        acid_proof: d.level(Swim) >= 2 || d.level(Spit) == 3,
        lava_proof: d.level(Spit) == 4,
        bite: d.level(Teeth) > 0,
        claw: d.level(Claws) > 0,
        venom: d.level(Venom) > 0,
        constrict: d.level(Tentacles) > 0,
        spit: match d.level(Spit) {
            1 => Some(SpitKind::Web),
            2 => Some(SpitKind::Venom),
            3 => Some(SpitKind::Acid),
            4 => Some(SpitKind::Fire),
            _ => None,
        },
        armor: lv(Armor) * 0.18,
        spines: d.level(Spines) > 0,
        toxic: d.level(ToxicFlesh) > 0,
        ink: d.level(Ink) > 0,
        camouflage: d.level(Camouflage) > 0,
        night_vision: d.level(NightVision) > 0,
        tremor_sense: d.level(Feelers) > 0,
        glow: d.level(Spit) == 4,
        ..Traits::default()
    };

    let loco = if leg_count == 0 {
        if wings > 0 {
            Locomotion::Fly
        } else if t.dig {
            Locomotion::Burrow
        } else {
            Locomotion::Slither
        }
    } else if d.level(Leap) >= 2 && leg_count == 2 {
        Locomotion::Hop
    } else {
        Locomotion::Walk
    };
    let habitat = if loco == Locomotion::Fly {
        Habitat::Aerial
    } else if loco == Locomotion::Burrow {
        Habitat::Burrower
    } else {
        Habitat::Ground
    };

    let body = build_body(d, &mut rng, s, leg_count, loco, mouth, teeth, &mut t);
    let colors = make_colors(d, &mut rng, biome, habitat, &t);

    // Stats.
    let base_speed = match leg_count {
        0 => {
            if loco == Locomotion::Fly {
                40.0
            } else {
                24.0
            }
        }
        2 => 31.0,
        4 => 35.0,
        6 => 30.0,
        _ => 27.0,
    };
    let mut speed =
        base_speed * (1.0 + 0.15 * lv(LongLegs)) * (1.0 - 0.06 * lv(Armor)) * (s / 4.0).powf(0.25);
    if wings > 0 {
        speed = speed.max(36.0 + 4.0 * wings as f32);
    }
    let g = 300.0;
    let jump_h = (s * 2.2 + 5.0) * (1.0 + 0.25 * lv(LongLegs)) * (1.0 + 0.35 * lv(Leap));
    let max_health = 10.0 * s.powf(1.5) * 1.22f32.powf(lv(Vitality));
    let eyes = [0usize, 1, 2, 3, 4, 6][d.level(Eyes) as usize];
    let sight = (28.0 + eyes as f32 * 22.0).min(150.0) * (1.0 + 0.2 * lv(NightVision));
    let stats = Stats {
        size: s,
        max_health,
        speed,
        accel: speed * 4.5,
        jump: (2.0 * g * jump_h).sqrt(),
        sight,
        hearing: 40.0 + 20.0 * lv(Feelers),
        damage: 4.0 * s.powf(1.2),
        attack_cooldown: 0.8,
        reach: s * 0.8 + 2.0,
        metabolism: 0.0,
        lifespan: 1.0e9,
        maturity: 1.0,
        litter: (0, 0),
        gestation: 1.0e9,
        spit_range: 55.0 + 10.0 * lv(Eyes),
        flee_distance: 40.0,
    };

    // Weapons.
    let mut weapons = Vec::new();
    let dmg = s.powf(1.15);
    let teeth_lv = d.level(Teeth);
    if teeth_lv > 0 {
        let (mult, cd, pierce, grab) = match teeth_lv {
            1 => (0.9, 0.35, 0.0, false),
            2 => (1.6, 0.8, 0.0, false),
            3 => (2.1, 0.8, 0.6, false),
            4 => (2.0, 0.9, 0.1, true),
            5 => (2.7, 0.85, 0.15, false),
            _ => (3.2, 0.95, 0.1, false),
        };
        weapons.push(Weapon {
            kind: WeaponKind::Bite,
            damage: dmg * mult,
            cooldown: cd,
            reach: s * 0.9 + 3.0,
            knockback: 1.2,
            pierce,
            venom: t.venom,
            grab,
            charge: false,
        });
    }
    if d.level(Claws) > 0 && (leg_count > 0 || d.level(Arms) > 0) {
        weapons.push(Weapon {
            kind: WeaponKind::Claw,
            damage: dmg * (1.0 + 0.9 * lv(Claws)),
            cooldown: 0.55,
            reach: s * 1.3 + 3.0,
            knockback: 1.6,
            pierce: 0.2,
            venom: false,
            grab: false,
            charge: false,
        });
    }
    if d.level(Arms) > 0 {
        weapons.push(Weapon {
            kind: WeaponKind::Punch,
            damage: dmg * (1.3 + 1.0 * lv(Arms)),
            cooldown: 0.7,
            reach: s * (1.3 + 0.4 * lv(Arms)) + 3.0,
            knockback: 5.0,
            pierce: 0.0,
            venom: false,
            grab: false,
            charge: false,
        });
    }
    match d.level(Tail) {
        0 => {}
        tail => {
            let (mult, kb, pierce, venom) = match tail {
                1 => (0.8, 2.0, 0.0, false),
                2 => (1.9, 7.0, 0.0, false),
                3 => (2.4, 2.0, 0.5, false),
                _ => (1.4, 1.5, 0.3, true),
            };
            weapons.push(Weapon {
                kind: WeaponKind::Tail,
                damage: dmg * mult,
                cooldown: 1.1,
                reach: s * 2.2 + 4.0,
                knockback: kb,
                pierce,
                venom,
                grab: false,
                charge: false,
            });
        }
    }
    if d.level(Horns) > 0 {
        weapons.push(Weapon {
            kind: WeaponKind::Horn,
            damage: dmg * (2.0 + 1.2 * lv(Horns)),
            cooldown: 2.2,
            reach: s * 0.8 + 3.0,
            knockback: 6.0,
            pierce: 0.4,
            venom: false,
            grab: false,
            charge: true,
        });
    }
    if d.level(Tentacles) > 0 {
        weapons.push(Weapon {
            kind: WeaponKind::Tentacle,
            damage: dmg * 0.6 * lv(Tentacles),
            cooldown: 1.4,
            reach: s * 2.4 + 4.0,
            knockback: 0.3,
            pierce: 0.0,
            venom: false,
            grab: true,
            charge: false,
        });
    }

    let spit_dmg = match t.spit {
        Some(SpitKind::Web) => dmg * 0.3,
        Some(SpitKind::Venom) => dmg * 0.8,
        Some(SpitKind::Acid) => dmg * 1.0,
        Some(SpitKind::Fire) => dmg * 0.9,
        None => 0.0,
    };

    let loadout = Loadout {
        count,
        size: s,
        regen: lv(Regeneration) * 0.012,
        armor: t.armor,
        spines: lv(Spines) * 0.35,
        weapons,
        venom: lv(Venom),
        spit: t.spit,
        spit_damage: spit_dmg,
        spit_cooldown: 2.2,
        spit_range: stats.spit_range,
        intelligence: lv(Intelligence) / 4.0,
        aggression: 0.15 + lv(Aggression) * 0.2,
        caution: lv(Caution) / 3.0,
        pack: lv(PackMind) / 2.0,
        camouflage: lv(Camouflage) / 2.0,
        ink: t.ink,
        toxic: t.toxic,
        sight,
        night: lv(NightVision) / 2.0,
        echo: if d.level(Echolocation) > 0 { 85.0 } else { 0.0 },
        tremor: t.tremor_sense,
        fly: t.fly,
        dig: t.dig,
        climb: t.climb,
        swim: t.swim,
        leap: lv(Leap),
        legs: leg_count,
    };

    let personality = Personality {
        aggression: loadout.aggression,
        bravery: 1.0 - loadout.caution * 0.7,
        curiosity: 0.5,
        sociality: loadout.pack,
        patience: 0.4 + loadout.intelligence * 0.4,
        laziness: 0.0,
    };
    let mut sp = Species {
        idx: team,
        role: Role::Predator,
        name: names::binomial(&mut rng),
        common: d.name.clone(),
        habitat,
        loco,
        activity: Activity::Cathemeral,
        hunt: vec![HuntStyle::Chase],
        flee: vec![FleeStyle::Run],
        traits: t,
        stats,
        body,
        colors,
        personality,
        behaviors: Vec::new(),
        food_flora: Vec::new(),
        description: Vec::new(),
    };
    sp.description = describe(d, &sp, &loadout);
    (sp, loadout)
}

#[allow(clippy::too_many_arguments)]
fn build_body(
    d: &Design,
    rng: &mut Rng,
    s: f32,
    legs: usize,
    loco: Locomotion,
    mouth: Mouth,
    teeth: Teeth,
    t: &mut Traits,
) -> BodyPlan {
    use Upgrade::*;
    let lv = |u: Upgrade| d.level(u) as f32;
    let mut b = base_body(s);
    b.mouth = mouth;
    b.teeth = teeth;
    let leg_len = s * (1.1 + 0.3 * lv(LongLegs) + 0.1 * lv(Leap)) * if legs == 2 { 1.35 } else { 1.0 };
    let leg_tip = if d.level(Burrow) > 0 {
        LimbTip::Claw
    } else if d.level(Climb) > 0 {
        LimbTip::Sucker
    } else if d.level(Claws) > 0 && d.level(Arms) == 0 {
        LimbTip::Claw
    } else if d.level(Swim) > 0 {
        LimbTip::Paddle
    } else {
        LimbTip::None
    };
    let leg_w = if s > 5.0 { 2.0 } else { 1.0 } + lv(Size) * 0.15 + lv(Leap) * 0.35;
    match legs {
        0 => {
            if loco == Locomotion::Fly {
                let n = rng.int(1, 2) as usize;
                b.segs = tapering(n, s * 0.75, s * 0.5, 0.8);
                b.spacing = s * 1.0;
                b.stance = s * 0.6;
            } else {
                let n = rng.int(6, 9) as usize;
                b.segs = tapering(n, s * 0.8, s * 0.3, 1.0);
                b.spacing = s * 0.9;
                b.rope = true;
                b.head_r = s * 0.8;
                b.head_ry = s * 0.65;
                b.stance = s * 0.8;
            }
        }
        2 => {
            // Upright biped, or a bird-like runner.
            let upright = rng.prob(0.55) || d.level(Arms) > 0;
            if upright {
                b.upright = true;
                b.segs = vec![seg(s * 0.65, s * 1.0)];
                b.head_r = s * rng.range(0.45, 0.6);
                b.head_ry = b.head_r;
                b.neck = s * rng.range(0.0, 0.4);
            } else {
                b.segs = vec![seg(s * 1.0, s * 0.8)];
                b.neck = s * rng.range(0.3, 0.9);
            }
            add_legs(rng, &mut b, 1, leg_len, leg_tip, leg_w);
            b.stance = leg_len * 0.78;
        }
        4 => {
            let n = rng.int(1, 2) as usize;
            b.segs = tapering(n, s, s * 0.85, rng.range(0.6, 0.9));
            b.neck = s * rng.range(0.1, 0.8);
            add_legs(rng, &mut b, 2, leg_len, leg_tip, leg_w);
            b.stance = leg_len * 0.75;
        }
        6 => {
            b.segs = vec![seg(s * 0.7, s * 0.6), seg(s * rng.range(0.9, 1.2), s * 0.8)];
            b.spacing = s * 1.3;
            add_legs(rng, &mut b, 3, leg_len, leg_tip, leg_w * 0.8);
            b.stance = leg_len * 0.7;
        }
        _ => {
            b.segs = vec![seg(s * 0.6, s * 0.55), seg(s * 1.0, s * 0.85)];
            b.spacing = s * 1.25;
            add_legs(rng, &mut b, 4, leg_len * 1.15, leg_tip, leg_w * 0.8);
            b.head_r = s * 0.45;
            b.stance = leg_len * 0.75;
        }
    }

    // Arms with hands (claws if clawed).
    if d.level(Arms) > 0 {
        let tip = if d.level(Claws) > 0 {
            LimbTip::Claw
        } else {
            LimbTip::Hand
        };
        let len = s * (1.3 + 0.4 * lv(Arms));
        b.limbs.push(limb(
            LimbKind::Arm,
            0,
            if b.upright { -0.4 } else { 0.1 },
            len,
            2,
            1.0 + 0.4 * lv(Arms),
            tip,
            true,
        ));
    }
    // Wings.
    if d.level(Wings) > 0 {
        b.wing = match d.level(Wings) {
            1 => WingKind::Insect,
            2 => WingKind::Membrane,
            _ => WingKind::Feather,
        };
        let pairs = if b.wing == WingKind::Insect { 2 } else { 1 };
        let wl = s * 2.4 + lv(Size) * 0.3;
        for i in 0..pairs {
            let mut w = limb(
                LimbKind::Wing,
                0,
                -FRAC_PI_2 + i as f32 * 0.5,
                wl * (1.0 - i as f32 * 0.25),
                3,
                1.0,
                LimbTip::None,
                true,
            );
            w.phase = i as f32 * 0.3;
            b.limbs.push(w);
        }
    }
    // Tail.
    let last = b.segs.len() as i32 - 1;
    let tail_tip = match d.level(Tail) {
        0 => None,
        1 => Some(LimbTip::None),
        2 => Some(LimbTip::Club),
        3 => Some(LimbTip::Spike),
        _ => Some(LimbTip::Stinger),
    };
    if let Some(tip) = tail_tip {
        b.limbs.push(limb(
            LimbKind::Tail,
            last,
            PI - 0.3,
            s * (2.0 + 0.4 * lv(Tail)),
            5,
            if s > 5.0 { 2.0 } else { 1.2 },
            tip,
            false,
        ));
    } else if !b.rope && rng.prob(0.4) {
        // A plain balancing tail.
        b.limbs.push(limb(
            LimbKind::Tail,
            last,
            PI - 0.3,
            s * 1.3,
            4,
            1.0,
            LimbTip::None,
            false,
        ));
    }
    // Tentacles from the head.
    if d.level(Tentacles) > 0 {
        let n = 2 * d.level(Tentacles) as i32;
        for i in 0..n {
            let mut l = limb(
                LimbKind::Tentacle,
                -1,
                0.5 + i as f32 * 0.35,
                s * 2.2,
                5,
                1.0,
                LimbTip::Sucker,
                false,
            );
            l.phase = i as f32 * 0.25;
            b.limbs.push(l);
        }
    }
    // A dorsal fin for swimmers, a tail fin for the legless.
    if d.level(Swim) > 0 {
        b.limbs.push(limb(
            LimbKind::Fin,
            0,
            -FRAC_PI_2 - 0.4,
            s * (0.8 + 0.3 * lv(Swim)),
            2,
            1.0,
            LimbTip::None,
            false,
        ));
        if legs == 0 {
            b.limbs.push(limb(
                LimbKind::Fin,
                last,
                PI,
                s * 1.2,
                2,
                2.0,
                LimbTip::Paddle,
                false,
            ));
        }
    }
    // Stockier with vitality; a throat pouch for spitters.
    let stock = 1.0 + 0.07 * lv(Vitality);
    for sg in b.segs.iter_mut() {
        sg.r *= 1.0 + 0.03 * lv(Vitality);
        sg.ry *= stock;
    }
    if d.level(Spit) > 0 {
        b.neck = b.neck.max(s * 0.35);
    }
    // Feelers: thin antennae. Echolocation: broad ear paddles.
    if d.level(Feelers) > 0 {
        b.limbs.push(limb(
            LimbKind::Antenna,
            -1,
            -1.2,
            s * 1.6,
            4,
            1.0,
            LimbTip::None,
            true,
        ));
    }
    if d.level(Echolocation) > 0 {
        b.limbs.push(limb(
            LimbKind::Antenna,
            -1,
            -1.5,
            s * 0.9,
            2,
            1.4,
            LimbTip::Paddle,
            true,
        ));
    }
    // Eyes.
    let n_eyes = [0usize, 1, 2, 3, 4, 6][d.level(Eyes) as usize];
    let big = 0.9 + 0.5 * lv(NightVision);
    for i in 0..n_eyes {
        let spread = (i as f32 - (n_eyes as f32 - 1.0) / 2.0) * 0.42;
        b.eyes.push(Eye {
            angle: -0.5 + spread,
            dist: rng.range(0.4, 0.6),
            size: big * rng.range(0.9, 1.15),
        });
    }
    // The head grows with the brain.
    let brain = 1.0 + 0.16 * lv(Intelligence);
    b.head_r *= brain;
    b.head_ry *= brain;
    if b.head_r == 0.0 && d.level(Intelligence) > 0 {
        b.head_r = s * 0.5 * brain;
        b.head_ry = s * 0.45 * brain;
    }
    // Crest and horns.
    b.horns = match d.level(Horns) {
        0 => 0.0,
        1 => 1.0,
        _ => 1.6,
    };
    b.crest = if d.level(Spines) > 0 {
        Crest::Spines
    } else if b.horns > 0.0 {
        Crest::Horns
    } else if d.level(Regeneration) > 0 {
        // Regenerative frills, like an axolotl.
        Crest::Frills
    } else {
        *rng.pick(&[Crest::None, Crest::None, Crest::Crest, Crest::Sail])
    };
    b.shell = d.level(Armor) > 0;
    t.spines = d.level(Spines) > 0;

    // Collision box: the front segment plus the head, never the trailing body.
    let s0 = b.segs[0];
    b.hw = if b.upright {
        s0.r.max(b.head_r)
    } else {
        (s0.r + b.head_r * 0.5).max(1.0)
    };
    b.hw = b.hw.clamp(1.0, 8.0);
    b.top = if b.upright {
        s0.ry + b.head_ry * 1.6 + b.neck
    } else {
        s0.ry.max(b.head_ry)
    };
    b.top = b.top.clamp(1.0, 14.0);
    b.stance = b.stance.clamp(1.0, 14.0);
    b
}

fn make_colors(d: &Design, rng: &mut Rng, biome: Option<&Biome>, habitat: Habitat, t: &Traits) -> Colors {
    let look = look_colour(d);
    let warning = t.toxic || t.venom;
    let (base, accent, pattern) = if t.camouflage {
        let env = match biome {
            Some(b) => env_colour(b, habitat, rng),
            None => [110, 100, 70],
        };
        let (eh, es, ev) = to_hsv(env);
        let base = hsv(
            eh + rng.range(-12.0, 12.0),
            (es + rng.range(-0.1, 0.1)).clamp(0.15, 0.9),
            (ev + rng.range(0.0, 0.12)).clamp(0.4, 0.9),
        );
        (
            base,
            scale(base, 0.75),
            *rng.pick(&[
                Pattern::Speckled,
                Pattern::Spots,
                Pattern::Stripes,
                Pattern::Chevron,
            ]),
        )
    } else if warning {
        let accent = if rng.coin() {
            [20, 18, 24]
        } else {
            let (h, _, _) = to_hsv(look);
            hsv(h + 90.0, 0.9, 0.9)
        };
        (
            look,
            accent,
            *rng.pick(&[
                Pattern::Bands,
                Pattern::Stripes,
                Pattern::Spots,
                Pattern::Eyespots,
            ]),
        )
    } else {
        let (h, _, _) = to_hsv(look);
        let accent = hsv(
            h + *rng.pick(&[30.0, 150.0, 180.0, 210.0, -30.0]),
            rng.range(0.4, 0.9),
            rng.range(0.4, 0.95),
        );
        (
            look,
            accent,
            *rng.pick(&[
                Pattern::Solid,
                Pattern::Stripes,
                Pattern::Spots,
                Pattern::Bands,
                Pattern::Gradient,
                Pattern::Speckled,
                Pattern::Chevron,
            ]),
        )
    };
    use Upgrade::*;
    let lv = |u: Upgrade| d.level(u) as f32;
    // Cautious animals keep a pale underside; regenerators a pink one.
    let mut belly = if rng.coin() || d.level(Caution) > 0 {
        mix(base, [235, 230, 220], rng.range(0.25, 0.5) + 0.12 * lv(Caution))
    } else {
        scale(base, 0.85)
    };
    if d.level(Regeneration) > 0 {
        belly = mix(belly, [255, 170, 180], 0.2 * lv(Regeneration));
    }
    let mut eye = if t.night_vision {
        hsv(rng.range(40.0, 180.0), 0.8, 1.0)
    } else {
        let bright = hsv(rng.range(0.0, 360.0), 0.9, 1.0);
        *rng.pick(&[[10, 10, 12], [240, 240, 230], bright])
    };
    // Aggression shows in the eyes.
    if d.level(Aggression) >= 3 {
        eye = mix(eye, [255, 40, 30], 0.3 * (lv(Aggression) - 2.0));
    }
    // Ink sacs darken the markings; pack hunters wear stripes.
    let accent = if t.ink {
        mix(accent, [20, 18, 30], 0.6)
    } else {
        accent
    };
    let pattern = if d.level(PackMind) > 0 {
        Pattern::Stripes
    } else {
        pattern
    };
    let glow = match t.spit {
        Some(SpitKind::Fire) => Some([255, 160, 60]),
        Some(SpitKind::Acid) => Some([150, 255, 80]),
        Some(SpitKind::Venom) => Some([200, 90, 255]),
        Some(SpitKind::Web) => Some([235, 235, 245]),
        None => None,
    };
    Colors {
        base,
        belly,
        accent,
        eye,
        pattern,
        pattern_scale: rng.range(1.5, 4.0) * if d.level(PackMind) > 0 { 0.6 } else { 1.0 },
        glow,
    }
}

fn describe(d: &Design, sp: &Species, l: &Loadout) -> Vec<String> {
    use Upgrade::*;
    let size = match sp.stats.size {
        s if s < 3.5 => "small",
        s if s < 5.0 => "mid-sized",
        s if s < 6.5 => "large",
        _ => "huge",
    };
    let legs = [
        "limbless",
        "two-legged",
        "four-legged",
        "six-legged",
        "eight-legged",
    ][d.level(Legs) as usize];
    let mut how = vec![sp.loco.label().to_string()];
    if l.fly && sp.loco != Locomotion::Fly {
        how.push("flies".into());
    }
    if l.dig {
        how.push("tunnels".into());
    }
    if l.climb {
        how.push("climbs".into());
    }
    if l.swim {
        how.push("swims".into());
    }
    let mut out = vec![format!("A {size}, {legs} creature that {}.", how.join(", "))];
    let weapons: Vec<String> = l.weapons.iter().map(|w| w.kind.label().to_string()).collect();
    let mut fight = if weapons.is_empty() {
        "It has no natural weapons".to_string()
    } else {
        format!("It fights with {}", weapons.join(", "))
    };
    if let Some(s) = l.spit {
        fight.push_str(&format!(" and spits {}", s.label()));
    }
    out.push(format!("{fight}."));
    let mut extra = Vec::new();
    if l.armor > 0.0 {
        extra.push("Plated back");
    }
    if l.spines > 0.0 {
        extra.push("spined");
    }
    if l.camouflage > 0.0 {
        extra.push("camouflaged");
    }
    if l.echo > 0.0 {
        extra.push("echolocates");
    }
    if l.tremor {
        extra.push("feels footsteps");
    }
    if l.night > 0.0 {
        extra.push("sees in the dark");
    }
    if l.regen > 0.0 {
        extra.push("heals");
    }
    if l.toxic {
        extra.push("toxic to bite");
    }
    if !extra.is_empty() {
        out.push(format!("{}.", extra.join(", ")));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_design_fits_the_budget() {
        let d = Design::new(5);
        assert!(d.cost() <= START_POINTS, "{}", d.cost());
        assert!(!d.name.is_empty());
    }

    #[test]
    fn random_designs_spend_within_budget_and_build() {
        let mut rng = Rng::new(9);
        for _ in 0..60 {
            let d = Design::random(&mut rng, START_POINTS, None);
            assert!(d.cost() <= START_POINTS, "{} > {}", d.cost(), START_POINTS);
            let (sp, l) = build_species(&d, 0, None);
            assert!(sp.body.hw >= 1.0 && sp.body.top >= 1.0);
            assert_eq!(l.count, d.count());
            let evolved = Design::random(&mut rng, ROUND_POINTS, Some(&d));
            assert!(evolved.evolves_from(&d));
            assert!(evolved.cost() <= START_POINTS + ROUND_POINTS);
        }
    }

    #[test]
    fn every_upgrade_shows_on_the_body() {
        let mut base = Design::new(1);
        for u in Upgrade::ALL {
            base.set(u, 0);
        }
        base.set(Upgrade::Legs, 1);
        let (plain, _) = build_species(&base, 0, None);
        for u in Upgrade::ALL {
            let mut d = base.clone();
            d.set(u, u.max());
            if u == Upgrade::NightVision {
                d.set(Upgrade::Eyes, 2);
            }
            let (sp, _) = build_species(&d, 0, None);
            let changed = sp.body.limbs.len() != plain.body.limbs.len()
                || sp.body.eyes.len() != plain.body.eyes.len()
                || sp
                    .body
                    .eyes
                    .iter()
                    .zip(&plain.body.eyes)
                    .any(|(a, b)| a.size != b.size)
                || sp.body.head_r != plain.body.head_r
                || sp.body.neck != plain.body.neck
                || sp.body.mouth != plain.body.mouth
                || sp.body.teeth != plain.body.teeth
                || sp.body.crest != plain.body.crest
                || sp.body.shell != plain.body.shell
                || sp.body.horns != plain.body.horns
                || sp
                    .body
                    .segs
                    .iter()
                    .zip(&plain.body.segs)
                    .any(|(a, b)| a.ry != b.ry)
                || sp.colors != plain.colors
                || sp.stats.size != plain.stats.size
                || sp
                    .body
                    .limbs
                    .iter()
                    .zip(&plain.body.limbs)
                    .any(|(a, b)| a.tip != b.tip || a.length != b.length || a.width != b.width);
            assert!(changed, "{u:?} is invisible on the body");
        }
    }
}
