//! Procedural species. Everything about a species (shape, limbs, abilities,
//! stats, colours, behaviour weights) is decided here from the biome it
//! lives in, its place in the food chain and a random seed.

use std::f32::consts::{FRAC_PI_2, PI};

use super::biome::{Biome, BiomeKind};
use super::brain::BehaviorKind;
use super::flora::{FloraSpecies, GrowthForm, Substrate, make_flora};
use super::math::{Rgb, hsv, mix, scale, to_hsv};
use super::names;
use crate::material::Material;
use crate::rng::Rng;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Prey,
    Predator,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Habitat {
    Ground,
    Burrower,
    Arboreal,
    Aerial,
    Aquatic,
    Amphibious,
    LavaDweller,
    CaveClinger,
}

impl Habitat {
    pub const ALL: [Habitat; 8] = [
        Habitat::Ground,
        Habitat::Burrower,
        Habitat::Arboreal,
        Habitat::Aerial,
        Habitat::Aquatic,
        Habitat::Amphibious,
        Habitat::LavaDweller,
        Habitat::CaveClinger,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Habitat::Ground => "ground-dweller",
            Habitat::Burrower => "burrower",
            Habitat::Arboreal => "tree-dweller",
            Habitat::Aerial => "flier",
            Habitat::Aquatic => "swimmer",
            Habitat::Amphibious => "amphibian",
            Habitat::LavaDweller => "lava-dweller",
            Habitat::CaveClinger => "cave-clinger",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Locomotion {
    Walk,
    Hop,
    Slither,
    Fly,
    Float,
    Swim,
    Burrow,
    Climb,
}

impl Locomotion {
    pub fn label(self) -> &'static str {
        match self {
            Locomotion::Walk => "walks",
            Locomotion::Hop => "hops",
            Locomotion::Slither => "slithers",
            Locomotion::Fly => "flies",
            Locomotion::Float => "floats on a gas sac",
            Locomotion::Swim => "swims",
            Locomotion::Burrow => "tunnels",
            Locomotion::Climb => "climbs",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HuntStyle {
    Chase,
    Stalk,
    Ambush,
    WebTrap,
    PitTrap,
    Lure,
    Sniper,
    Pack,
}

impl HuntStyle {
    pub fn label(self) -> &'static str {
        match self {
            HuntStyle::Chase => "runs prey down",
            HuntStyle::Stalk => "stalks unseen, then pounces",
            HuntStyle::Ambush => "lies in wait and strikes",
            HuntStyle::WebTrap => "spins web traps",
            HuntStyle::PitTrap => "digs pitfall traps",
            HuntStyle::Lure => "dangles a glowing lure",
            HuntStyle::Sniper => "spits from range",
            HuntStyle::Pack => "hunts in coordinated packs",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FleeStyle {
    Run,
    Burrow,
    Fly,
    Dive,
    Shelter,
    Freeze,
    Ink,
    PlayDead,
    Leap,
    Mob,
}

impl FleeStyle {
    pub fn label(self) -> &'static str {
        match self {
            FleeStyle::Run => "bolts",
            FleeStyle::Burrow => "digs to safety",
            FleeStyle::Fly => "takes to the air",
            FleeStyle::Dive => "dives deep",
            FleeStyle::Shelter => "retreats into its shelter",
            FleeStyle::Freeze => "freezes and blends in",
            FleeStyle::Ink => "releases a blinding cloud",
            FleeStyle::PlayDead => "plays dead",
            FleeStyle::Leap => "leaps away",
            FleeStyle::Mob => "mobs attackers as a group",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpitKind {
    Acid,
    Fire,
    Venom,
    Web,
}

impl SpitKind {
    pub fn label(self) -> &'static str {
        match self {
            SpitKind::Acid => "acid",
            SpitKind::Fire => "fire",
            SpitKind::Venom => "venom",
            SpitKind::Web => "sticky silk",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Activity {
    Diurnal,
    Nocturnal,
    Crepuscular,
    Cathemeral,
}

impl Activity {
    pub fn label(self) -> &'static str {
        match self {
            Activity::Diurnal => "active by day",
            Activity::Nocturnal => "active at night",
            Activity::Crepuscular => "active at dawn and dusk",
            Activity::Cathemeral => "active at all hours",
        }
    }

    /// How awake the species wants to be at `daylight` (0 night, 1 noon)
    /// and `dusk` (1 at dawn/dusk).
    pub fn alertness(self, daylight: f32, dusk: f32) -> f32 {
        match self {
            Activity::Diurnal => 0.15 + 0.85 * daylight,
            Activity::Nocturnal => 1.0 - 0.85 * daylight,
            Activity::Crepuscular => 0.3 + 0.7 * dusk,
            Activity::Cathemeral => 0.8,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mouth {
    Mandibles,
    Beak,
    Maw,
    Proboscis,
    Sucker,
    Jaws,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LimbKind {
    Leg,
    Arm,
    Tentacle,
    Wing,
    Tail,
    Antenna,
    Fin,
    EyeStalk,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LimbTip {
    None,
    Claw,
    Pincer,
    Sucker,
    Stinger,
    Paddle,
    Club,
    Hook,
    Eye,
    Spike,
    /// Digits that can grip and punch.
    Hand,
}

/// What lines the jaws. Only drawn for `Mouth::Jaws` and `Mouth::Maw`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Teeth {
    #[default]
    None,
    /// Flat grinding teeth.
    Flat,
    /// Long fangs.
    Canines,
    /// Rows of triangular cutting teeth.
    Shark,
    /// Fine bristles that rake.
    Bristles,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pattern {
    Solid,
    Stripes,
    Spots,
    Bands,
    Gradient,
    Eyespots,
    Speckled,
    Chevron,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Crest {
    None,
    Horns,
    Crest,
    Frills,
    Spines,
    Sail,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WingKind {
    Membrane,
    Feather,
    Insect,
}

#[derive(Clone, Copy, Debug)]
pub struct Seg {
    /// Half length along the body.
    pub r: f32,
    /// Half height across it.
    pub ry: f32,
}

#[derive(Clone, Debug)]
pub struct Limb {
    pub kind: LimbKind,
    /// Body segment it grows from; -1 is the head.
    pub seg: i32,
    /// Attachment angle on the segment, facing right: 0 forward, PI/2 down.
    pub angle: f32,
    pub length: f32,
    pub joints: u8,
    pub width: f32,
    pub tip: LimbTip,
    /// Drawn twice (near and far side).
    pub paired: bool,
    /// Gait phase offset.
    pub phase: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Eye {
    pub angle: f32,
    pub dist: f32,
    pub size: f32,
}

#[derive(Clone, Debug)]
pub struct BodyPlan {
    /// Body axis is vertical (bipeds, floaters).
    pub upright: bool,
    pub segs: Vec<Seg>,
    pub spacing: f32,
    /// Segments follow the head's path like a rope instead of staying rigid.
    pub rope: bool,
    pub head_r: f32,
    pub head_ry: f32,
    pub neck: f32,
    pub eyes: Vec<Eye>,
    pub mouth: Mouth,
    pub teeth: Teeth,
    pub crest: Crest,
    /// Horn length relative to size (0 for none).
    pub horns: f32,
    pub shell: bool,
    /// Gas bladder radius (floaters).
    pub sac: f32,
    pub wing: WingKind,
    pub limbs: Vec<Limb>,
    /// Distance from body centre down to the ground when standing.
    pub stance: f32,
    /// Collision box relative to the body centre.
    pub hw: f32,
    pub top: f32,
}

impl BodyPlan {
    pub fn count(&self, kind: LimbKind) -> usize {
        self.limbs
            .iter()
            .filter(|l| l.kind == kind)
            .map(|l| if l.paired { 2 } else { 1 })
            .sum()
    }
    pub fn total_limbs(&self) -> usize {
        self.limbs
            .iter()
            .filter(|l| !matches!(l.kind, LimbKind::Antenna | LimbKind::EyeStalk))
            .map(|l| if l.paired { 2 } else { 1 })
            .sum()
    }
    pub fn length(&self) -> f32 {
        self.segs.len() as f32 * self.spacing + self.head_r * 2.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Colors {
    pub base: Rgb,
    pub belly: Rgb,
    pub accent: Rgb,
    pub eye: Rgb,
    pub pattern: Pattern,
    pub pattern_scale: f32,
    pub glow: Option<Rgb>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Traits {
    pub dig: bool,
    /// Moves through sand and soil without leaving tunnels.
    pub sand_swim: bool,
    pub fly: bool,
    pub swim: bool,
    pub lava_proof: bool,
    pub acid_proof: bool,
    pub climb: bool,
    pub leap: bool,
    pub bite: bool,
    pub claw: bool,
    pub venom: bool,
    pub constrict: bool,
    pub spit: Option<SpitKind>,
    /// Fraction of damage blocked.
    pub armor: f32,
    pub spines: bool,
    pub toxic: bool,
    pub ink: bool,
    pub glow: bool,
    pub camouflage: bool,
    pub build_shelter: bool,
    pub night_vision: bool,
    pub tremor_sense: bool,
    /// Gills only: suffocates in air.
    pub gills: bool,
    pub play_dead: bool,
    pub territorial: bool,
    pub mob: bool,
    pub alarm_call: bool,
    pub eggs: bool,
    pub scavenger: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Personality {
    pub aggression: f32,
    pub bravery: f32,
    pub curiosity: f32,
    pub sociality: f32,
    pub patience: f32,
    pub laziness: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Stats {
    /// Overall scale in cells.
    pub size: f32,
    pub max_health: f32,
    pub speed: f32,
    pub accel: f32,
    pub jump: f32,
    pub sight: f32,
    pub hearing: f32,
    pub damage: f32,
    pub attack_cooldown: f32,
    pub reach: f32,
    /// Satiety lost per second.
    pub metabolism: f32,
    pub lifespan: f32,
    pub maturity: f32,
    pub litter: (u8, u8),
    pub gestation: f32,
    pub spit_range: f32,
    /// How far it notices threats and starts fleeing.
    pub flee_distance: f32,
}

#[derive(Clone, Debug)]
pub struct Species {
    pub idx: usize,
    pub role: Role,
    pub name: String,
    pub common: String,
    pub habitat: Habitat,
    pub loco: Locomotion,
    pub activity: Activity,
    pub hunt: Vec<HuntStyle>,
    pub flee: Vec<FleeStyle>,
    pub traits: Traits,
    pub stats: Stats,
    pub body: BodyPlan,
    pub colors: Colors,
    pub personality: Personality,
    /// Behaviour modules this species has, with a weight scaling their utility.
    pub behaviors: Vec<(BehaviorKind, f32)>,
    /// Which plant species it can eat.
    pub food_flora: Vec<usize>,
    pub description: Vec<String>,
}

impl Species {
    pub fn has(&self, b: BehaviorKind) -> bool {
        self.behaviors.iter().any(|(k, _)| *k == b)
    }
    pub fn hunts(&self, h: HuntStyle) -> bool {
        self.hunt.contains(&h)
    }
    pub fn flees(&self, f: FleeStyle) -> bool {
        self.flee.contains(&f)
    }

    /// Whether it can be in this liquid without harm.
    pub fn liquid_ok(&self, m: Material) -> bool {
        match m {
            Material::Water | Material::Oil => true,
            Material::Lava => self.traits.lava_proof,
            Material::Acid => self.traits.acid_proof,
            _ => true,
        }
    }

    /// Whether it moves freely (swims) in this liquid.
    pub fn swims_in(&self, m: Material) -> bool {
        match m {
            Material::Water | Material::Oil => self.traits.swim,
            Material::Lava => self.traits.swim && self.traits.lava_proof,
            Material::Acid => self.traits.swim && self.traits.acid_proof,
            _ => false,
        }
    }

    /// Plain-language list of notable abilities for the UI.
    pub fn ability_list(&self) -> Vec<String> {
        let t = &self.traits;
        let mut v: Vec<String> = Vec::new();
        let add = |v: &mut Vec<String>, cond: bool, s: &str| {
            if cond {
                v.push(s.to_string());
            }
        };
        add(&mut v, t.fly, "flight");
        add(&mut v, t.swim, "swimming");
        add(&mut v, t.lava_proof, "heat-proof");
        add(&mut v, t.acid_proof, "acid-proof");
        add(&mut v, t.dig && !t.sand_swim, "tunnelling");
        add(&mut v, t.sand_swim, "sand-swimming");
        add(&mut v, t.climb, "climbing");
        add(&mut v, t.leap, "leaping");
        add(&mut v, t.bite, "bite");
        add(&mut v, t.claw, "claws");
        add(&mut v, t.venom, "venom");
        add(&mut v, t.constrict, "constriction");
        if let Some(s) = t.spit {
            v.push(format!("spits {}", s.label()));
        }
        add(&mut v, t.armor > 0.05, "armour");
        add(&mut v, t.spines, "spines");
        add(&mut v, t.toxic, "toxic flesh");
        add(&mut v, t.ink, "ink cloud");
        add(&mut v, t.glow, "bioluminescence");
        add(&mut v, t.camouflage, "camouflage");
        add(&mut v, t.build_shelter, "builds shelters");
        add(&mut v, t.night_vision, "night vision");
        add(&mut v, t.tremor_sense, "tremor sense");
        add(&mut v, t.gills, "gills");
        add(&mut v, t.play_dead, "plays dead");
        add(&mut v, t.territorial, "territorial");
        add(&mut v, t.mob, "mobbing");
        add(&mut v, t.alarm_call, "alarm calls");
        add(&mut v, t.eggs, "lays eggs");
        add(&mut v, t.scavenger, "scavenges");
        v
    }
}

// ---------------------------------------------------------------------------
// Habitat choice

fn habitat_weight(biome: &Biome, h: Habitat) -> f32 {
    let has_water = biome.water > 0.008;
    match h {
        Habitat::Ground => {
            if biome.kind == BiomeKind::SkyIsles {
                0.5
            } else {
                1.0
            }
        }
        Habitat::Burrower => (biome.soil * 3.5).clamp(0.1, 1.4),
        Habitat::Arboreal => {
            if biome.trees.len() >= 3 {
                0.7
            } else {
                0.0
            }
        }
        Habitat::Aerial => match biome.kind {
            BiomeKind::SkyIsles => 2.2,
            BiomeKind::SpireCanyon => 1.2,
            _ => 0.4 + biome.open_air * 0.6,
        },
        Habitat::Aquatic => {
            if has_water {
                (biome.water * 9.0).clamp(0.3, 2.2)
            } else {
                0.0
            }
        }
        Habitat::Amphibious => {
            if has_water || biome.acid > 0.008 {
                0.6
            } else {
                0.0
            }
        }
        Habitat::LavaDweller => {
            if biome.lava > 0.008 {
                (biome.lava * 40.0).clamp(0.4, 1.6)
            } else {
                0.0
            }
        }
        Habitat::CaveClinger => {
            if !biome.open_sky {
                1.3
            } else if biome.kind == BiomeKind::SpireCanyon {
                0.6
            } else {
                0.2
            }
        }
    }
}

/// How well a predator of habitat `pred` can reach prey of habitat `prey`.
fn compat(prey: Habitat, pred: Habitat) -> f32 {
    use Habitat::*;
    match (prey, pred) {
        (Ground, Ground) => 1.0,
        (Ground, Burrower) => 0.7,
        (Ground, Arboreal) => 0.5,
        (Ground, Aerial) => 0.8,
        (Ground, Amphibious) => 0.6,
        (Ground, CaveClinger) => 0.6,
        (Ground, LavaDweller) => 0.3,
        (Burrower, Burrower) => 1.2,
        (Burrower, Ground) => 0.6,
        (Arboreal, Arboreal) => 1.0,
        (Arboreal, Aerial) => 0.8,
        (Arboreal, CaveClinger) => 0.6,
        (Arboreal, Ground) => 0.3,
        (Aerial, Aerial) => 1.0,
        (Aerial, CaveClinger) => 0.7,
        (Aerial, Arboreal) => 0.7,
        (Aerial, Ground) => 0.3,
        (Aquatic, Aquatic) => 1.2,
        (Aquatic, Amphibious) => 0.8,
        (Aquatic, Aerial) => 0.6,
        (Amphibious, Aquatic) => 0.8,
        (Amphibious, Amphibious) => 1.0,
        (Amphibious, Ground) => 0.8,
        (Amphibious, Aerial) => 0.5,
        (LavaDweller, LavaDweller) => 1.5,
        (LavaDweller, Aerial) => 0.3,
        (CaveClinger, CaveClinger) => 1.0,
        (CaveClinger, Aerial) => 0.8,
        (CaveClinger, Arboreal) => 0.4,
        _ => 0.0,
    }
}

fn pick_habitat(rng: &mut Rng, biome: &Biome, prey: Option<&Species>) -> Habitat {
    let weights: Vec<f32> = Habitat::ALL
        .iter()
        .map(|&h| {
            let w = habitat_weight(biome, h);
            match prey {
                Some(p) => w.max(0.15 * (h == p.habitat) as u8 as f32) * compat(p.habitat, h),
                None => w,
            }
        })
        .collect();
    Habitat::ALL[rng.weighted(&weights)]
}

/// Plant substrates a herbivore of this habitat can feed from.
fn food_substrates(h: Habitat) -> &'static [Substrate] {
    use Substrate::*;
    match h {
        Habitat::Ground => &[Ground, LavaRock],
        Habitat::Burrower => &[Ground],
        Habitat::Arboreal => &[Canopy, Ground],
        Habitat::Aerial => &[Ground, Ceiling, Canopy, LavaRock],
        Habitat::Aquatic => &[Seabed],
        Habitat::Amphibious => &[Seabed, Ground],
        Habitat::LavaDweller => &[LavaRock],
        Habitat::CaveClinger => &[Ceiling, Canopy],
    }
}

// ---------------------------------------------------------------------------
// Bodies

pub(crate) fn seg(r: f32, ry: f32) -> Seg {
    Seg { r, ry }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn limb(
    kind: LimbKind,
    seg: i32,
    angle: f32,
    length: f32,
    joints: u8,
    width: f32,
    tip: LimbTip,
    paired: bool,
) -> Limb {
    Limb {
        kind,
        seg,
        angle,
        length,
        joints,
        width,
        tip,
        paired,
        phase: 0.0,
    }
}

/// Adds `pairs` leg pairs spread along the body.
pub(crate) fn add_legs(rng: &mut Rng, b: &mut BodyPlan, pairs: usize, length: f32, tip: LimbTip, width: f32) {
    let n = b.segs.len();
    for i in 0..pairs {
        let seg = if pairs == 1 {
            0
        } else if n >= pairs {
            i * (n - 1) / (pairs - 1).max(1)
        } else {
            (i * n / pairs).min(n - 1)
        };
        let lean = if pairs == 1 {
            0.0
        } else {
            0.5 - i as f32 / (pairs - 1) as f32
        };
        let mut l = limb(
            LimbKind::Leg,
            seg as i32,
            FRAC_PI_2 - lean * 0.6,
            length,
            2,
            width,
            tip,
            true,
        );
        l.phase = (i % 2) as f32 * 0.5 + rng.range(-0.05, 0.05);
        b.limbs.push(l);
    }
}

pub(crate) fn base_body(size: f32) -> BodyPlan {
    BodyPlan {
        upright: false,
        segs: vec![seg(size, size * 0.8)],
        spacing: size * 1.2,
        rope: false,
        head_r: size * 0.6,
        head_ry: size * 0.55,
        neck: 0.0,
        eyes: Vec::new(),
        mouth: Mouth::Jaws,
        teeth: Teeth::None,
        crest: Crest::None,
        horns: 0.0,
        shell: false,
        sac: 0.0,
        wing: WingKind::Membrane,
        limbs: Vec::new(),
        stance: size * 0.8,
        hw: size,
        top: size,
    }
}

pub(crate) fn tapering(n: usize, r0: f32, r1: f32, flat: f32) -> Vec<Seg> {
    (0..n)
        .map(|i| {
            let t = if n <= 1 { 0.0 } else { i as f32 / (n - 1) as f32 };
            let r = r0 + (r1 - r0) * t;
            seg(r, r * flat)
        })
        .collect()
}

fn build_body(
    rng: &mut Rng,
    loco: Locomotion,
    habitat: Habitat,
    role: Role,
    s: f32,
    t: &mut Traits,
) -> BodyPlan {
    let mut b = base_body(s);
    let pred = role == Role::Predator;
    match loco {
        Locomotion::Walk => {
            let pairs = [1, 2, 3, 4, 6][rng.weighted(&[0.25, 0.35, 0.25, 0.1, 0.05])];
            let upright = pairs == 1 && rng.prob(0.65);
            let leg_len = s * rng.range(1.2, 2.3) * if upright { 1.3 } else { 1.0 };
            if upright {
                b.upright = true;
                b.segs = vec![seg(s * 0.65, s * 1.0)];
                b.head_r = s * rng.range(0.45, 0.7);
                b.head_ry = b.head_r;
                b.neck = s * rng.range(0.0, 0.5);
            } else if pairs >= 4 {
                b.segs = tapering(pairs, s * 0.75, s * 0.55, rng.range(0.6, 0.9));
                b.spacing = s * 1.1;
                b.rope = true;
            } else if pairs == 3 {
                b.segs = vec![
                    seg(s * 0.7, s * 0.6),
                    seg(s * rng.range(0.9, 1.3), s * rng.range(0.7, 1.0)),
                ];
                b.spacing = s * 1.3;
            } else {
                let n = rng.int(1, 2) as usize;
                b.segs = tapering(n, s, s * 0.85, rng.range(0.55, 0.9));
                b.neck = s * rng.range(0.0, 0.9);
            }
            let tip = if t.dig || (t.claw && rng.coin()) {
                LimbTip::Claw
            } else {
                LimbTip::None
            };
            let tip = if habitat == Habitat::Amphibious {
                LimbTip::Paddle
            } else {
                tip
            };
            add_legs(rng, &mut b, pairs, leg_len, tip, if s > 5.0 { 2.0 } else { 1.0 });
            if upright && (t.claw || rng.prob(0.5)) {
                let tip = if t.claw {
                    *rng.pick(&[LimbTip::Claw, LimbTip::Pincer, LimbTip::Hook])
                } else {
                    LimbTip::None
                };
                b.limbs.push(limb(
                    LimbKind::Arm,
                    0,
                    -0.4,
                    s * rng.range(1.2, 2.0),
                    2,
                    1.0,
                    tip,
                    true,
                ));
            }
            b.stance = leg_len * 0.75;
        }
        Locomotion::Hop => {
            b.segs = vec![seg(s * 1.15, s * 1.05)];
            b.head_r = 0.0;
            b.head_ry = 0.0;
            if rng.coin() {
                let len = s * rng.range(1.3, 1.9);
                let mut l = limb(LimbKind::Leg, 0, 1.9, len, 2, 2.0, LimbTip::None, true);
                l.phase = 0.0;
                b.limbs.push(l);
                b.stance = s * 1.1;
            } else {
                b.stance = s * 1.05;
            }
            t.leap = true;
        }
        Locomotion::Slither => {
            let n = rng.int(5, 11) as usize;
            b.segs = tapering(n, s * 0.8, s * 0.3, 1.0);
            b.spacing = s * 0.9;
            b.rope = true;
            b.head_r = s * 0.8;
            b.head_ry = s * 0.65;
            b.stance = s * 0.8;
        }
        Locomotion::Fly => {
            let n = rng.int(1, 3) as usize;
            b.segs = tapering(n, s * 0.7, s * 0.45, 0.8);
            b.spacing = s * 1.0;
            b.wing = *rng.pick(&[WingKind::Membrane, WingKind::Feather, WingKind::Insect]);
            let wing_pairs = if b.wing == WingKind::Insect {
                rng.int(1, 2)
            } else {
                1
            };
            let wl = s * rng.range(2.0, 3.2);
            for i in 0..wing_pairs {
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
            let leg_pairs = rng.int(0, 2) as usize;
            if leg_pairs > 0 {
                add_legs(
                    rng,
                    &mut b,
                    leg_pairs,
                    s * 0.9,
                    if pred { LimbTip::Hook } else { LimbTip::None },
                    1.0,
                );
                b.stance = s * 0.8;
            } else {
                b.stance = s * 0.6;
            }
            if rng.prob(0.5) {
                let last = b.segs.len() as i32 - 1;
                b.limbs.push(limb(
                    LimbKind::Tail,
                    last,
                    PI,
                    s * rng.range(1.5, 4.0),
                    5,
                    1.0,
                    *rng.pick(&[LimbTip::None, LimbTip::Club, LimbTip::Spike]),
                    false,
                ));
            }
            b.mouth = *rng.pick(&[Mouth::Beak, Mouth::Jaws, Mouth::Proboscis]);
        }
        Locomotion::Float => {
            b.upright = true;
            b.sac = s * rng.range(1.1, 1.5);
            b.segs = vec![seg(s * 0.5, s * 0.45)];
            b.head_r = 0.0;
            b.head_ry = 0.0;
            let n = rng.int(3, 8);
            for i in 0..n {
                let a = FRAC_PI_2 + (i as f32 / (n - 1).max(1) as f32 - 0.5) * 1.2;
                let mut l = limb(
                    LimbKind::Tentacle,
                    0,
                    a,
                    s * rng.range(2.0, 4.0),
                    6,
                    1.0,
                    *rng.pick(&[LimbTip::None, LimbTip::Sucker, LimbTip::Stinger]),
                    false,
                );
                l.phase = rng.range(0.0, 1.0);
                b.limbs.push(l);
            }
            b.stance = s * 0.5;
            t.fly = true;
        }
        Locomotion::Swim => match rng.int(0, 3) {
            0 => {
                // Fish-like.
                let n = rng.int(2, 4) as usize;
                b.segs = tapering(n, s, s * 0.4, rng.range(0.6, 1.0));
                b.spacing = s * 0.9;
                let last = n as i32 - 1;
                b.limbs.push(limb(
                    LimbKind::Fin,
                    last,
                    PI,
                    s * 1.4,
                    2,
                    2.0,
                    LimbTip::Paddle,
                    false,
                ));
                b.limbs.push(limb(
                    LimbKind::Fin,
                    0,
                    -FRAC_PI_2 - 0.4,
                    s * 0.9,
                    2,
                    1.0,
                    LimbTip::None,
                    false,
                ));
                b.limbs.push(limb(
                    LimbKind::Fin,
                    0,
                    FRAC_PI_2 + 0.3,
                    s * 0.7,
                    2,
                    1.0,
                    LimbTip::None,
                    true,
                ));
            }
            1 => {
                // Eel-like.
                let n = rng.int(6, 10) as usize;
                b.segs = tapering(n, s * 0.7, s * 0.25, 1.0);
                b.spacing = s * 0.85;
                b.rope = true;
                b.head_r = s * 0.7;
                let last = n as i32 - 1;
                b.limbs.push(limb(
                    LimbKind::Fin,
                    last,
                    PI,
                    s * 1.0,
                    2,
                    1.0,
                    LimbTip::Paddle,
                    false,
                ));
            }
            2 => {
                // Squid-like, jets with tentacles trailing.
                b.segs = vec![seg(s * 1.2, s * 0.65)];
                b.head_r = s * 0.55;
                let n = rng.int(4, 8);
                for i in 0..n {
                    let mut l = limb(
                        LimbKind::Tentacle,
                        -1,
                        (i as f32 / (n - 1) as f32 - 0.5) * 0.9,
                        s * rng.range(1.8, 3.2),
                        6,
                        1.0,
                        *rng.pick(&[LimbTip::Sucker, LimbTip::None, LimbTip::Hook]),
                        false,
                    );
                    l.phase = rng.range(0.0, 1.0);
                    b.limbs.push(l);
                }
                t.ink = t.ink || rng.prob(0.5);
            }
            _ => {
                // Ray-like: flat, flapping fins.
                b.segs = vec![seg(s * 1.3, s * 0.4)];
                b.head_r = s * 0.4;
                b.limbs.push(limb(
                    LimbKind::Fin,
                    0,
                    -FRAC_PI_2,
                    s * 1.8,
                    3,
                    1.0,
                    LimbTip::None,
                    true,
                ));
                b.limbs.push(limb(
                    LimbKind::Tail,
                    0,
                    PI,
                    s * rng.range(2.0, 4.0),
                    5,
                    1.0,
                    if pred { LimbTip::Stinger } else { LimbTip::None },
                    false,
                ));
            }
        },
        Locomotion::Burrow => match rng.int(0, 2) {
            0 => {
                // Worm.
                let n = rng.int(6, 12) as usize;
                b.segs = tapering(n, s * 0.8, s * 0.55, 1.0);
                b.spacing = s * 0.85;
                b.rope = true;
                b.head_r = s * 0.85;
                b.mouth = Mouth::Maw;
                b.stance = s * 0.8;
            }
            1 => {
                // Mole-like with digging claws.
                b.segs = vec![seg(s * 1.1, s * 0.8)];
                b.head_r = s * 0.6;
                add_legs(rng, &mut b, 2, s * 1.0, LimbTip::Claw, 2.0);
                b.stance = s * 0.75;
                t.claw = true;
            }
            _ => {
                // Insect digger with mandibles.
                b.segs = vec![seg(s * 0.6, s * 0.55), seg(s * 0.9, s * 0.7)];
                b.spacing = s * 1.2;
                add_legs(rng, &mut b, 3, s * 1.3, LimbTip::Claw, 1.0);
                b.mouth = Mouth::Mandibles;
                b.stance = s * 0.9;
            }
        },
        Locomotion::Climb => match rng.int(0, 3) {
            0 => {
                // Spider-like.
                b.segs = vec![seg(s * 0.6, s * 0.55), seg(s * 1.0, s * 0.85)];
                b.spacing = s * 1.25;
                let len = s * rng.range(2.2, 3.4);
                add_legs(rng, &mut b, 4, len, LimbTip::Hook, 1.0);
                b.mouth = Mouth::Mandibles;
                b.head_r = s * 0.4;
                b.stance = s * 1.4;
            }
            1 => {
                // Long-armed brachiator with a grasping tail.
                b.upright = true;
                b.segs = vec![seg(s * 0.6, s * 0.95)];
                b.head_r = s * 0.55;
                b.head_ry = s * 0.5;
                add_legs(rng, &mut b, 1, s * 1.5, LimbTip::Hook, 1.0);
                b.limbs
                    .push(limb(LimbKind::Arm, 0, -0.6, s * 2.2, 2, 1.0, LimbTip::Hook, true));
                b.limbs.push(limb(
                    LimbKind::Tail,
                    0,
                    PI * 0.8,
                    s * 2.5,
                    6,
                    1.0,
                    LimbTip::Hook,
                    false,
                ));
                b.stance = s * 1.2;
            }
            2 => {
                // Octopoid that walks on its tentacles.
                b.segs = vec![seg(s * 1.0, s * 0.9)];
                b.head_r = 0.0;
                let n = rng.int(6, 8);
                for i in 0..n {
                    let mut l = limb(
                        LimbKind::Tentacle,
                        0,
                        FRAC_PI_2 + (i as f32 / (n - 1) as f32 - 0.5) * 1.6,
                        s * rng.range(1.6, 2.4),
                        5,
                        1.0,
                        LimbTip::Sucker,
                        false,
                    );
                    l.phase = i as f32 / n as f32;
                    b.limbs.push(l);
                }
                b.stance = s * 1.1;
            }
            _ => {
                // Flat gecko-like.
                b.segs = vec![seg(s * 0.9, s * 0.55), seg(s * 0.8, s * 0.5)];
                b.spacing = s * 1.1;
                add_legs(rng, &mut b, 2, s * 1.2, LimbTip::Sucker, 1.0);
                b.limbs.push(limb(
                    LimbKind::Tail,
                    1,
                    PI,
                    s * rng.range(1.5, 3.0),
                    5,
                    1.0,
                    LimbTip::None,
                    false,
                ));
                b.stance = s * 0.7;
            }
        },
    }

    // Features shared by every body plan, chosen for flavour.
    if b.head_r > 0.0 || b.segs.len() == 1 {
        let n_eyes = if t.tremor_sense && habitat == Habitat::Burrower && rng.prob(0.6) {
            0
        } else {
            [1, 2, 2, 3, 4, 6][rng.int(0, 5) as usize]
        };
        let big = t.night_vision;
        for i in 0..n_eyes {
            let spread = (i as f32 - (n_eyes - 1) as f32 / 2.0) * 0.45;
            b.eyes.push(Eye {
                angle: -0.5 + spread,
                dist: rng.range(0.35, 0.65),
                size: if big {
                    rng.range(1.2, 2.0)
                } else {
                    rng.range(0.6, 1.3)
                },
            });
        }
    }
    if !matches!(loco, Locomotion::Fly | Locomotion::Float | Locomotion::Burrow) {
        b.mouth = if pred {
            *rng.pick(&[Mouth::Jaws, Mouth::Mandibles, Mouth::Maw, Mouth::Beak])
        } else {
            *rng.pick(&[
                Mouth::Jaws,
                Mouth::Beak,
                Mouth::Proboscis,
                Mouth::Sucker,
                Mouth::Mandibles,
            ])
        };
    }
    if rng.prob(0.35) {
        b.limbs.push(limb(
            LimbKind::Antenna,
            -1,
            -1.2 + rng.range(-0.3, 0.3),
            s * rng.range(1.0, 2.2),
            4,
            1.0,
            *rng.pick(&[LimbTip::None, LimbTip::None, LimbTip::Club]),
            true,
        ));
    }
    if rng.prob(0.15) {
        let n = rng.int(1, 3);
        for i in 0..n {
            b.limbs.push(limb(
                LimbKind::EyeStalk,
                -1,
                -1.6 + i as f32 * 0.4,
                s * rng.range(0.8, 1.6),
                3,
                1.0,
                LimbTip::Eye,
                false,
            ));
        }
    }
    let has_tail = b.limbs.iter().any(|l| l.kind == LimbKind::Tail);
    if !has_tail && !b.rope && rng.prob(0.45) && b.sac == 0.0 {
        let last = b.segs.len() as i32 - 1;
        let tip = if t.venom && rng.coin() {
            LimbTip::Stinger
        } else {
            *rng.pick(&[LimbTip::None, LimbTip::None, LimbTip::Club, LimbTip::Spike])
        };
        b.limbs.push(limb(
            LimbKind::Tail,
            last,
            PI - 0.3,
            s * rng.range(1.2, 3.0),
            5,
            if s > 5.0 { 2.0 } else { 1.0 },
            tip,
            false,
        ));
    }
    if pred && t.claw && !b.limbs.iter().any(|l| l.kind == LimbKind::Arm) && rng.prob(0.4) {
        // Raptorial forelimbs.
        b.limbs.push(limb(
            LimbKind::Arm,
            0,
            0.2,
            s * rng.range(1.3, 2.0),
            2,
            1.0,
            *rng.pick(&[LimbTip::Claw, LimbTip::Pincer, LimbTip::Hook]),
            true,
        ));
    }
    if t.constrict && !b.limbs.iter().any(|l| l.kind == LimbKind::Tentacle) && !b.rope {
        for i in 0..2 {
            b.limbs.push(limb(
                LimbKind::Tentacle,
                -1,
                0.6 + i as f32 * 0.4,
                s * 1.8,
                5,
                1.0,
                LimbTip::Sucker,
                false,
            ));
        }
    }
    if rng.prob(0.06) {
        // Something unexplained growing from its back.
        let n = rng.int(2, 4);
        for i in 0..n {
            b.limbs.push(limb(
                LimbKind::Tentacle,
                0,
                -FRAC_PI_2 - 0.4 + i as f32 * 0.3,
                s * rng.range(1.0, 2.0),
                4,
                1.0,
                LimbTip::None,
                false,
            ));
        }
    }
    b.crest = if t.spines {
        Crest::Spines
    } else {
        *rng.pick(&[
            Crest::None,
            Crest::None,
            Crest::None,
            Crest::Horns,
            Crest::Crest,
            Crest::Frills,
            Crest::Sail,
        ])
    };
    if b.crest == Crest::Horns {
        b.horns = 1.0;
    }
    b.shell = t.armor > 0.2;

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
    b.top = b.top.max(b.sac * 1.6).clamp(1.0, 14.0);
    b.stance = b.stance.clamp(1.0, 14.0);
    b
}

// ---------------------------------------------------------------------------
// Colour

pub(crate) fn env_colour(biome: &Biome, habitat: Habitat, rng: &mut Rng) -> Rgb {
    let p = &biome.palette;
    match habitat {
        Habitat::Ground | Habitat::Amphibious => {
            let opts = [
                p.mat(Material::Grass),
                p.mat(Material::Dirt),
                p.mat(Material::Sand),
            ];
            match biome.kind {
                BiomeKind::DuneSea => p.mat(Material::Sand),
                BiomeKind::VolcanicRift => p.mat(Material::Ash),
                _ => *rng.pick(&opts),
            }
        }
        Habitat::Burrower => {
            if biome.kind == BiomeKind::DuneSea {
                p.mat(Material::Sand)
            } else {
                p.mat(Material::Dirt)
            }
        }
        Habitat::Arboreal => *rng.pick(&[p.mat(Material::Grass), p.mat(Material::Wood)]),
        Habitat::Aerial => p.sky_day[0],
        Habitat::Aquatic => p.mat(Material::Water),
        Habitat::LavaDweller => p.mat(Material::Stone),
        Habitat::CaveClinger => p.mat(Material::Stone),
    }
}

fn make_colors(rng: &mut Rng, biome: &Biome, habitat: Habitat, t: &Traits, role: Role) -> Colors {
    let env = env_colour(biome, habitat, rng);
    let (eh, es, ev) = to_hsv(env);
    let warning = t.toxic || (t.venom && role == Role::Prey) || (t.venom && rng.prob(0.3));
    let (base, accent, pattern) = if t.camouflage {
        let base = hsv(
            eh + rng.range(-15.0, 15.0),
            (es + rng.range(-0.1, 0.1)).clamp(0.15, 0.9),
            (ev + rng.range(0.0, 0.15)).clamp(0.42, 0.9),
        );
        let accent = scale(base, rng.range(0.65, 0.85));
        (
            base,
            accent,
            *rng.pick(&[
                Pattern::Speckled,
                Pattern::Spots,
                Pattern::Stripes,
                Pattern::Chevron,
            ]),
        )
    } else if warning {
        let base = hsv(
            eh + 180.0 + rng.range(-40.0, 40.0),
            rng.range(0.75, 0.95),
            rng.range(0.8, 1.0),
        );
        let accent = if rng.coin() {
            [20, 18, 24]
        } else {
            hsv(eh + 90.0, 0.9, 0.9)
        };
        (
            base,
            accent,
            *rng.pick(&[
                Pattern::Bands,
                Pattern::Stripes,
                Pattern::Spots,
                Pattern::Eyespots,
            ]),
        )
    } else {
        let h = rng.range(0.0, 360.0);
        let base = hsv(h, rng.range(0.35, 0.8), rng.range(0.55, 0.9));
        let accent = hsv(
            h + *rng.pick(&[30.0, 150.0, 180.0, 210.0, -30.0]),
            rng.range(0.4, 0.9),
            rng.range(0.4, 0.95),
        );
        let pattern = *rng.pick(&[
            Pattern::Solid,
            Pattern::Stripes,
            Pattern::Spots,
            Pattern::Bands,
            Pattern::Gradient,
            Pattern::Eyespots,
            Pattern::Speckled,
            Pattern::Chevron,
        ]);
        (base, accent, pattern)
    };
    let belly = if matches!(habitat, Habitat::Aerial | Habitat::Aquatic) || rng.coin() {
        mix(base, [235, 230, 220], rng.range(0.25, 0.5))
    } else {
        scale(base, 0.85)
    };
    let eye = if t.night_vision {
        hsv(rng.range(40.0, 180.0), 0.8, 1.0)
    } else {
        let bright = hsv(rng.range(0.0, 360.0), 0.9, 1.0);
        *rng.pick(&[[10, 10, 12], [240, 240, 230], bright])
    };
    let glow = t.glow.then(|| hsv(rng.range(0.0, 360.0), 0.7, 1.0));
    Colors {
        base,
        belly,
        accent,
        eye,
        pattern,
        pattern_scale: rng.range(1.5, 4.0),
        glow,
    }
}

// ---------------------------------------------------------------------------
// Species

fn loco_for(rng: &mut Rng, habitat: Habitat) -> Locomotion {
    use Locomotion::*;
    let (opts, w): (&[Locomotion], &[f32]) = match habitat {
        Habitat::Ground => (&[Walk, Hop, Slither], &[0.7, 0.15, 0.15]),
        Habitat::Burrower => (&[Burrow], &[1.0]),
        Habitat::Arboreal => (&[Climb], &[1.0]),
        Habitat::Aerial => (&[Fly, Float], &[0.8, 0.2]),
        Habitat::Aquatic => (&[Swim], &[1.0]),
        Habitat::Amphibious => (&[Walk, Slither], &[0.7, 0.3]),
        Habitat::LavaDweller => (&[Walk, Swim, Slither], &[0.4, 0.4, 0.2]),
        Habitat::CaveClinger => (&[Climb, Fly], &[0.7, 0.3]),
    };
    opts[rng.weighted(w)]
}

fn base_traits(rng: &mut Rng, habitat: Habitat, loco: Locomotion, biome: &Biome) -> Traits {
    let mut t = Traits::default();
    match habitat {
        Habitat::Burrower => {
            t.dig = true;
            t.sand_swim = biome.kind == BiomeKind::DuneSea || rng.prob(0.3);
            t.tremor_sense = rng.prob(0.7);
        }
        Habitat::Arboreal => {
            t.climb = true;
            t.leap = rng.prob(0.5);
        }
        Habitat::Aerial => t.fly = true,
        Habitat::Aquatic => {
            t.swim = true;
            t.gills = rng.prob(0.8);
        }
        Habitat::Amphibious => t.swim = true,
        Habitat::LavaDweller => {
            t.lava_proof = true;
            t.swim = true;
            t.glow = rng.prob(0.6);
        }
        Habitat::CaveClinger => {
            t.climb = loco == Locomotion::Climb;
            t.fly = loco == Locomotion::Fly;
            t.night_vision = rng.prob(0.6);
        }
        Habitat::Ground => {
            t.leap = rng.prob(0.25);
            t.swim = rng.prob(0.2);
            t.dig = rng.prob(0.15);
        }
    }
    if loco == Locomotion::Swim && habitat == Habitat::LavaDweller {
        t.gills = false;
    }
    if biome.acid > 0.005 && (t.swim || rng.prob(0.3)) {
        t.acid_proof = true;
    }
    if biome.kind == BiomeKind::VolcanicRift && rng.prob(0.5) {
        t.lava_proof = true;
    }
    if biome.darkness > 0.5 {
        t.night_vision = t.night_vision || rng.prob(0.6);
        t.glow = t.glow || rng.prob(0.4);
    }
    t
}

fn activity(rng: &mut Rng, biome: &Biome) -> Activity {
    if !biome.open_sky {
        return if rng.prob(0.8) {
            Activity::Cathemeral
        } else {
            Activity::Nocturnal
        };
    }
    *rng.pick(&[
        Activity::Diurnal,
        Activity::Diurnal,
        Activity::Nocturnal,
        Activity::Crepuscular,
        Activity::Cathemeral,
    ])
}

fn base_stats(rng: &mut Rng, loco: Locomotion, s: f32, role: Role, biome: &Biome) -> Stats {
    let speed = match loco {
        Locomotion::Walk => 30.0,
        Locomotion::Hop => 34.0,
        Locomotion::Slither => 24.0,
        Locomotion::Fly => 46.0,
        Locomotion::Float => 14.0,
        Locomotion::Swim => 34.0,
        Locomotion::Burrow => 22.0,
        Locomotion::Climb => 26.0,
    } * (s / 4.0).powf(0.3)
        * rng.range(0.85, 1.2);
    let g = 300.0 * biome.gravity;
    let jump_h = s * rng.range(2.5, 4.0) + 6.0;
    let pred = role == Role::Predator;
    Stats {
        size: s,
        max_health: 10.0 * s.powf(1.5),
        speed: if pred { speed * 1.1 } else { speed },
        accel: speed * rng.range(3.0, 6.0),
        jump: (2.0 * g * jump_h).sqrt(),
        sight: rng.range(70.0, 130.0),
        hearing: rng.range(30.0, 60.0),
        damage: if pred { 5.0 * s.powf(1.2) } else { 1.5 * s },
        attack_cooldown: rng.range(0.6, 1.2),
        reach: s * 0.8 + 2.0,
        metabolism: if pred {
            rng.range(0.0045, 0.007)
        } else {
            rng.range(0.006, 0.009)
        },
        lifespan: if pred {
            rng.range(500.0, 800.0)
        } else {
            rng.range(280.0, 480.0)
        },
        maturity: if pred {
            rng.range(70.0, 110.0)
        } else {
            rng.range(35.0, 60.0)
        },
        litter: if pred {
            (1, rng.int(1, 2) as u8)
        } else {
            (2, rng.int(3, 4) as u8)
        },
        gestation: rng.range(12.0, 25.0),
        spit_range: rng.range(45.0, 75.0),
        flee_distance: rng.range(35.0, 60.0),
    }
}

fn pick_hunt(
    rng: &mut Rng,
    habitat: Habitat,
    loco: Locomotion,
    prey: &Species,
    size_ratio: f32,
    biome: &Biome,
) -> Vec<HuntStyle> {
    use HuntStyle::*;
    let styles = [Chase, Stalk, Ambush, WebTrap, PitTrap, Lure, Sniper, Pack];
    let web_ok = !matches!(
        prey.habitat,
        Habitat::Aquatic | Habitat::Burrower | Habitat::LavaDweller
    );
    let pit_ok = matches!(prey.habitat, Habitat::Ground | Habitat::Amphibious) && biome.soil > 0.08;
    let w = |s: HuntStyle| -> f32 {
        match s {
            Chase => match loco {
                Locomotion::Walk | Locomotion::Fly | Locomotion::Swim | Locomotion::Hop => 1.0,
                _ => 0.35,
            },
            Stalk => match loco {
                Locomotion::Walk | Locomotion::Climb | Locomotion::Slither => 0.8,
                _ => 0.1,
            },
            Ambush => match habitat {
                Habitat::Burrower => 1.5,
                Habitat::Aquatic | Habitat::Amphibious | Habitat::CaveClinger | Habitat::LavaDweller => 1.0,
                _ => 0.45,
            },
            WebTrap if web_ok => match habitat {
                Habitat::CaveClinger | Habitat::Arboreal => 1.2,
                _ if loco == Locomotion::Climb => 1.0,
                _ if loco == Locomotion::Walk => 0.35,
                _ => 0.0,
            },
            PitTrap if pit_ok => match loco {
                Locomotion::Burrow => 0.9,
                Locomotion::Walk => 0.3,
                _ => 0.0,
            },
            Lure => {
                if biome.darkness > 0.5 || habitat == Habitat::Aquatic {
                    0.8
                } else {
                    0.12
                }
            }
            Sniper => 0.5,
            Pack => 0.4 + if size_ratio < 1.0 { 1.2 } else { 0.0 },
            _ => 0.0,
        }
    };
    let weights: Vec<f32> = styles.iter().map(|&s| w(s)).collect();
    let first = styles[rng.weighted(&weights)];
    let mut out = vec![first];
    if rng.prob(0.5) {
        let second: Vec<f32> = styles
            .iter()
            .map(|&s| {
                let clash = s == first
                    || matches!(
                        (first, s),
                        (WebTrap | PitTrap | Lure, Pack) | (Pack, WebTrap | PitTrap | Lure)
                    )
                    || matches!((first, s), (WebTrap, PitTrap) | (PitTrap, WebTrap));
                if clash { 0.0 } else { w(s) }
            })
            .collect();
        if second.iter().any(|&x| x > 0.0) {
            out.push(styles[rng.weighted(&second)]);
        }
    }
    out
}

fn pick_defenses(rng: &mut Rng, t: &mut Traits, habitat: Habitat, loco: Locomotion) -> Vec<FleeStyle> {
    let mut flee = Vec::new();
    if loco != Locomotion::Float {
        flee.push(FleeStyle::Run);
    }
    if t.fly {
        flee.push(FleeStyle::Fly);
    }
    if t.dig {
        flee.push(FleeStyle::Burrow);
    }
    if t.swim
        && matches!(
            habitat,
            Habitat::Amphibious | Habitat::Aquatic | Habitat::LavaDweller
        )
    {
        flee.push(FleeStyle::Dive);
    }
    // Extra defences, two or three of them.
    let options = [
        "camo", "shelter", "armor", "spines", "toxic", "ink", "dead", "leap", "mob", "herd",
    ];
    let weights = [
        1.0,
        if matches!(habitat, Habitat::Ground | Habitat::Amphibious) {
            1.0
        } else {
            0.0
        },
        0.7,
        0.6,
        0.6,
        if matches!(habitat, Habitat::Aquatic) {
            1.0
        } else {
            0.25
        },
        0.3,
        if matches!(loco, Locomotion::Walk | Locomotion::Hop | Locomotion::Climb) {
            0.6
        } else {
            0.0
        },
        0.4,
        0.8,
    ];
    let mut w = weights.to_vec();
    for _ in 0..rng.int(2, 3) {
        let i = rng.weighted(&w);
        w[i] = 0.0;
        match options[i] {
            "camo" => {
                t.camouflage = true;
                flee.push(FleeStyle::Freeze);
            }
            "shelter" => {
                t.build_shelter = true;
                flee.push(FleeStyle::Shelter);
            }
            "armor" => t.armor = rng.range(0.25, 0.5),
            "spines" => t.spines = true,
            "toxic" => t.toxic = true,
            "ink" => {
                t.ink = true;
                flee.push(FleeStyle::Ink);
            }
            "dead" => {
                t.play_dead = true;
                flee.push(FleeStyle::PlayDead);
            }
            "leap" => {
                t.leap = true;
                flee.push(FleeStyle::Leap);
            }
            "mob" => {
                t.mob = true;
                t.bite = true;
                flee.push(FleeStyle::Mob);
            }
            _ => t.alarm_call = true,
        }
    }
    flee.dedup();
    flee
}

fn common_name(rng: &mut Rng, sp: &Species) -> String {
    let colour = if let Some(g) = sp.colors.glow {
        if rng.coin() {
            "Lantern"
        } else {
            names::colour_word(g, rng)
        }
    } else {
        names::colour_word(sp.colors.base, rng)
    };
    let b = &sp.body;
    let t = &sp.traits;
    let mut features: Vec<&str> = Vec::new();
    let legs = b.count(LimbKind::Leg);
    if legs >= 8 {
        features.push("Many-legged");
    }
    if b.eyes.len() >= 4 {
        features.push("Many-eyed");
    }
    if b.eyes.is_empty() {
        features.push("Eyeless");
    }
    if b.sac > 0.0 {
        features.push("Bladder");
    }
    if b.count(LimbKind::Tentacle) >= 3 {
        features.push("Tendril");
    }
    if t.spines {
        features.push("Spine");
    }
    if t.armor > 0.2 {
        features.push("Shell");
    }
    if b.crest == Crest::Horns {
        features.push("Horned");
    }
    if b.crest == Crest::Sail {
        features.push("Sailback");
    }
    if b.limbs.iter().any(|l| l.tip == LimbTip::Pincer) {
        features.push("Pincer");
    }
    if b.limbs.iter().any(|l| l.tip == LimbTip::Hook) {
        features.push("Hook");
    }
    if b.limbs.iter().any(|l| l.kind == LimbKind::EyeStalk) {
        features.push("Stalk-eyed");
    }
    if t.venom {
        features.push("Venom");
    }
    let feature = if features.is_empty() {
        ""
    } else {
        *rng.pick(&features)
    };
    let noun = match sp.role {
        Role::Predator => match sp.hunt[0] {
            HuntStyle::Chase => *rng.pick(&["Runner", "Courser", "Snapper", "Harrier"]),
            HuntStyle::Stalk => *rng.pick(&["Stalker", "Prowler", "Creeper"]),
            HuntStyle::Ambush => *rng.pick(&["Lurker", "Ambusher", "Trapjaw"]),
            HuntStyle::WebTrap => *rng.pick(&["Weaver", "Spinner", "Silkmaw"]),
            HuntStyle::PitTrap => *rng.pick(&["Pitlord", "Sinkmaw", "Funneler"]),
            HuntStyle::Lure => *rng.pick(&["Angler", "Beckoner", "Lanternjaw"]),
            HuntStyle::Sniper => *rng.pick(&["Spitter", "Gobber", "Sprayer"]),
            HuntStyle::Pack => *rng.pick(&["Packhound", "Swarmer", "Wolfling"]),
        },
        Role::Prey => match sp.loco {
            Locomotion::Walk => *rng.pick(&["Grazer", "Skitter", "Trotter", "Nibbler"]),
            Locomotion::Hop => *rng.pick(&["Hopper", "Bouncer", "Springer"]),
            Locomotion::Slither => *rng.pick(&["Crawler", "Wriggler", "Slider"]),
            Locomotion::Fly => *rng.pick(&["Flitter", "Glider", "Darter"]),
            Locomotion::Float => *rng.pick(&["Drifter", "Bobber", "Floatling"]),
            Locomotion::Swim => *rng.pick(&["Shoaler", "Finling", "Swimmer"]),
            Locomotion::Burrow => *rng.pick(&["Delver", "Rootgnaw", "Mole"]),
            Locomotion::Climb => *rng.pick(&["Clamberer", "Clinger", "Scaler"]),
        },
    };
    if feature.is_empty() {
        format!("{colour} {noun}")
    } else {
        format!("{colour} {feature} {noun}")
    }
}

fn behaviours(sp: &Species) -> Vec<(BehaviorKind, f32)> {
    use BehaviorKind::*;
    let p = &sp.personality;
    let t = &sp.traits;
    let mut v = vec![
        (Escape, 1.0),
        (Wander, 0.6 + p.curiosity * 0.6),
        (Rest, 0.7 + p.laziness * 0.6),
        (Reproduce, 1.0),
    ];
    match sp.role {
        Role::Prey => {
            v.push((Graze, 1.0));
            v.push((Flee, 1.2 - p.bravery * 0.4));
            if t.camouflage {
                v.push((Freeze, 1.0));
            }
            if t.build_shelter {
                v.push((BuildNest, 0.8));
                v.push((Shelter, 1.0));
            }
            if p.sociality > 0.45 {
                v.push((Flock, p.sociality));
            }
            if t.mob {
                v.push((Mob, 0.6 + p.aggression));
            }
            if p.curiosity > 0.6 {
                v.push((Investigate, p.curiosity * 0.6));
            }
        }
        Role::Predator => {
            v.push((Hunt, 0.8 + p.aggression * 0.4));
            v.push((Flee, 0.5));
            if sp.hunts(HuntStyle::Ambush) || sp.hunts(HuntStyle::Lure) {
                v.push((Ambush, 0.6 + p.patience * 0.6));
            }
            if sp.hunts(HuntStyle::WebTrap) || sp.hunts(HuntStyle::PitTrap) {
                v.push((SetTrap, 0.7 + p.patience * 0.5));
            }
            if t.scavenger {
                v.push((Scavenge, 1.0));
            }
            if t.territorial {
                v.push((Patrol, 0.5 + p.aggression * 0.5));
            }
            if p.sociality > 0.6 {
                v.push((Flock, p.sociality * 0.6));
            }
            v.push((Investigate, 0.3 + p.curiosity * 0.6));
        }
    }
    v
}

fn describe(sp: &Species, biome: &Biome) -> Vec<String> {
    let b = &sp.body;
    let t = &sp.traits;
    let size = match sp.stats.size {
        s if s < 3.0 => "tiny",
        s if s < 4.5 => "small",
        s if s < 6.5 => "mid-sized",
        s if s < 9.0 => "large",
        _ => "huge",
    };
    let limbs = b.total_limbs();
    let limb_desc = if limbs == 0 {
        "limbless".to_string()
    } else {
        let mut parts = Vec::new();
        for (k, name) in [
            (LimbKind::Leg, "legs"),
            (LimbKind::Arm, "arms"),
            (LimbKind::Tentacle, "tentacles"),
            (LimbKind::Wing, "wings"),
            (LimbKind::Fin, "fins"),
            (LimbKind::Tail, "tails"),
        ] {
            let n = b.count(k);
            if n > 0 {
                let name = if n == 1 { name.trim_end_matches('s') } else { name };
                parts.push(format!("{n} {name}"));
            }
        }
        parts.join(", ")
    };
    let eyes = match b.eyes.len() + b.count(LimbKind::EyeStalk) {
        0 => "no eyes".to_string(),
        1 => "a single eye".to_string(),
        n => format!("{n} eyes"),
    };
    let mut out = vec![format!(
        "A {size} {} with {limb_desc} and {eyes}. It {}.",
        sp.habitat.label(),
        sp.loco.label()
    )];
    match sp.role {
        Role::Predator => {
            let styles: Vec<&str> = sp.hunt.iter().map(|h| h.label()).collect();
            let mut weapons = Vec::new();
            if t.bite {
                weapons.push("bites".to_string());
            }
            if t.claw {
                weapons.push("slashes with claws".to_string());
            }
            if t.venom {
                weapons.push("injects venom".to_string());
            }
            if t.constrict {
                weapons.push("constricts".to_string());
            }
            if let Some(s) = t.spit {
                weapons.push(format!("spits {}", s.label()));
            }
            out.push(format!(
                "A predator that {}. It {}.",
                styles.join(" and "),
                weapons.join(", ")
            ));
        }
        Role::Prey => {
            let flee: Vec<&str> = sp.flee.iter().map(|f| f.label()).collect();
            out.push(format!("A grazer. When threatened it {}.", flee.join(", ")));
        }
    }
    let mut extra = Vec::new();
    if t.armor > 0.05 {
        extra.push("Its back is plated");
    }
    if t.spines {
        extra.push("Spines punish anything that bites it");
    }
    if t.toxic {
        extra.push("Its flesh is toxic and its colours warn of it");
    }
    if t.camouflage {
        extra.push("Its colours match its surroundings");
    }
    if t.build_shelter {
        extra.push("It packs mud into domed shelters");
    }
    if t.glow {
        extra.push("It glows in the dark");
    }
    if t.territorial {
        extra.push("It drives rivals out of its territory");
    }
    if t.alarm_call {
        extra.push("It shrieks to warn its kin");
    }
    if t.scavenger {
        extra.push("It will eat carrion");
    }
    if !extra.is_empty() {
        out.push(format!("{}.", extra.join(". ")));
    }
    let social = match sp.personality.sociality {
        s if s > 0.7 => "lives in tight groups",
        s if s > 0.4 => "keeps loosely together",
        _ => "is solitary",
    };
    out.push(format!(
        "It {social}, is {}, and {} in the {} of {}.",
        sp.activity.label(),
        if t.eggs { "lays eggs" } else { "bears live young" },
        biome.describe_gravity(),
        biome.name
    ));
    out
}

fn finish(rng: &mut Rng, mut sp: Species, biome: &Biome) -> Species {
    sp.behaviors = behaviours(&sp);
    sp.common = common_name(rng, &sp);
    sp.description = describe(&sp, biome);
    sp
}

/// Makes sure some plant the prey can reach exists; returns the plants it eats.
fn ensure_food(
    rng: &mut Rng,
    biome: &Biome,
    habitat: Habitat,
    flora: &mut Vec<FloraSpecies>,
    lava_proof: bool,
) -> Vec<usize> {
    let subs = food_substrates(habitat);
    let ok =
        |f: &FloraSpecies| subs.contains(&f.substrate) && (lava_proof || f.substrate != Substrate::LavaRock);
    if !flora.iter().any(ok) {
        let (form, sub) = match subs[0] {
            Substrate::Seabed => (GrowthForm::Kelp, Substrate::Seabed),
            Substrate::LavaRock => (GrowthForm::Crystal, Substrate::LavaRock),
            Substrate::Ceiling => (GrowthForm::Vine, Substrate::Ceiling),
            Substrate::Canopy if !biome.trees.is_empty() => (GrowthForm::Pod, Substrate::Canopy),
            Substrate::Canopy => (GrowthForm::Vine, Substrate::Ceiling),
            Substrate::Ground => (GrowthForm::Bush, Substrate::Ground),
        };
        flora.push(make_flora(rng, biome, form, sub));
    }
    flora
        .iter()
        .enumerate()
        .filter(|(_, f)| subs.contains(&f.substrate) && (lava_proof || f.substrate != Substrate::LavaRock))
        .map(|(i, _)| i)
        .collect()
}

pub fn generate_prey(rng: &mut Rng, biome: &Biome, flora: &mut Vec<FloraSpecies>) -> Species {
    let habitat = pick_habitat(rng, biome, None);
    let loco = loco_for(rng, habitat);
    let mut traits = base_traits(rng, habitat, loco, biome);
    let s = match habitat {
        Habitat::Aerial | Habitat::CaveClinger => rng.range(2.0, 3.5),
        Habitat::Arboreal => rng.range(2.5, 4.0),
        _ => rng.range(2.5, 5.0),
    };
    let flee = pick_defenses(rng, &mut traits, habitat, loco);
    traits.eggs = rng.prob(if matches!(habitat, Habitat::Aquatic | Habitat::Aerial) {
        0.7
    } else {
        0.4
    });
    let personality = Personality {
        aggression: if traits.mob {
            rng.range(0.5, 0.8)
        } else {
            rng.range(0.0, 0.3)
        },
        bravery: rng.range(0.0, 0.5),
        curiosity: rng.range(0.2, 0.9),
        sociality: if traits.alarm_call {
            rng.range(0.6, 1.0)
        } else {
            rng.range(0.1, 0.9)
        },
        patience: rng.range(0.3, 0.7),
        laziness: rng.range(0.2, 0.7),
    };
    let food_flora = ensure_food(rng, biome, habitat, flora, traits.lava_proof);
    let body = build_body(rng, loco, habitat, Role::Prey, s, &mut traits);
    let colors = make_colors(rng, biome, habitat, &traits, Role::Prey);
    let mut stats = base_stats(rng, loco, s, Role::Prey, biome);
    if traits.armor > 0.0 {
        stats.speed *= 0.85;
    }
    if traits.leap {
        stats.jump *= 1.3;
    }
    if traits.night_vision {
        stats.sight *= 1.2;
    }
    let sp = Species {
        idx: 0,
        role: Role::Prey,
        name: names::binomial(rng),
        common: String::new(),
        habitat,
        loco,
        activity: activity(rng, biome),
        hunt: Vec::new(),
        flee,
        traits,
        stats,
        body,
        colors,
        personality,
        behaviors: Vec::new(),
        food_flora,
        description: Vec::new(),
    };
    finish(rng, sp, biome)
}

pub fn generate_predator(rng: &mut Rng, biome: &Biome, prey: &Species) -> Species {
    let habitat = pick_habitat(rng, biome, Some(prey));
    let loco = loco_for(rng, habitat);
    let mut traits = base_traits(rng, habitat, loco, biome);
    let ratio = rng.range(0.7, 2.4);
    let s = (prey.stats.size * ratio).clamp(2.5, 11.0);
    // Make sure it can actually get at its prey.
    match prey.habitat {
        Habitat::Burrower => traits.dig = true,
        Habitat::Aquatic => traits.swim = true,
        Habitat::LavaDweller => traits.lava_proof = true,
        _ => {}
    }
    if prey.traits.acid_proof && habitat == prey.habitat {
        traits.acid_proof = true;
    }
    let hunt = pick_hunt(rng, habitat, loco, prey, ratio, biome);
    traits.bite = true;
    traits.claw = rng.prob(0.45);
    traits.venom = rng.prob(if ratio < 1.0 { 0.7 } else { 0.3 });
    traits.constrict = matches!(loco, Locomotion::Slither) && rng.prob(0.5) || rng.prob(0.08);
    traits.scavenger = rng.prob(0.5);
    traits.territorial = !hunt.contains(&HuntStyle::Pack) && rng.prob(0.4);
    traits.camouflage = hunt
        .iter()
        .any(|h| matches!(h, HuntStyle::Stalk | HuntStyle::Ambush))
        || rng.prob(0.15);
    traits.armor = if rng.prob(0.2) { rng.range(0.15, 0.4) } else { 0.0 };
    traits.spines = rng.prob(0.1);
    if hunt.contains(&HuntStyle::Lure) {
        traits.glow = true;
    }
    traits.spit = if hunt.contains(&HuntStyle::Sniper) || rng.prob(0.15) {
        Some(if traits.lava_proof && biome.lava > 0.005 {
            SpitKind::Fire
        } else if hunt.contains(&HuntStyle::WebTrap) && rng.coin() {
            SpitKind::Web
        } else if biome.acid > 0.005 || rng.prob(0.4) {
            SpitKind::Acid
        } else {
            SpitKind::Venom
        })
    } else {
        None
    };
    if hunt.contains(&HuntStyle::WebTrap) {
        traits.climb = true;
    }
    if hunt.contains(&HuntStyle::PitTrap) {
        traits.dig = true;
    }
    if hunt.contains(&HuntStyle::Stalk) || traits.leap || rng.prob(0.3) {
        traits.leap = true;
    }
    traits.eggs = rng.prob(0.4);
    let pack = hunt.contains(&HuntStyle::Pack);
    let personality = Personality {
        aggression: rng.range(0.5, 1.0),
        bravery: rng.range(0.5, 1.0),
        curiosity: rng.range(0.2, 0.8),
        sociality: if pack {
            rng.range(0.75, 1.0)
        } else {
            rng.range(0.0, 0.4)
        },
        patience: if hunt.iter().any(|h| {
            matches!(
                h,
                HuntStyle::Ambush | HuntStyle::WebTrap | HuntStyle::PitTrap | HuntStyle::Lure
            )
        }) {
            rng.range(0.7, 1.0)
        } else {
            rng.range(0.2, 0.6)
        },
        laziness: rng.range(0.2, 0.7),
    };
    let body = build_body(rng, loco, habitat, Role::Predator, s, &mut traits);
    let colors = make_colors(rng, biome, habitat, &traits, Role::Predator);
    let mut stats = base_stats(rng, loco, s, Role::Predator, biome);
    if traits.venom {
        stats.damage *= 0.8;
    }
    if hunt.contains(&HuntStyle::Chase) {
        stats.speed *= 1.15;
    }
    if traits.night_vision {
        stats.sight *= 1.25;
    }
    if traits.leap {
        stats.jump *= 1.25;
    }
    // Packs of small hunters need to outrun what they eat.
    if pack {
        stats.speed = stats.speed.max(prey.stats.speed * 1.05);
    }
    let sp = Species {
        idx: 1,
        role: Role::Predator,
        name: names::binomial(rng),
        common: String::new(),
        habitat,
        loco,
        activity: activity(rng, biome),
        hunt,
        flee: vec![FleeStyle::Run],
        traits,
        stats,
        body,
        colors,
        personality,
        behaviors: Vec::new(),
        food_flora: Vec::new(),
        description: Vec::new(),
    };
    finish(rng, sp, biome)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eco::biome;
    use crate::eco::flora::biome_flora;

    #[test]
    fn species_are_varied_and_consistent() {
        let mut shapes = std::collections::HashSet::new();
        for seed in 0..60u64 {
            let (b, _) = biome::generate(seed, None);
            let mut rng = Rng::new(seed);
            let mut flora = biome_flora(&mut rng, &b);
            let prey = generate_prey(&mut rng, &b, &mut flora);
            let pred = generate_predator(&mut rng, &b, &prey);
            assert!(!prey.food_flora.is_empty(), "prey has nothing to eat");
            assert!(!pred.hunt.is_empty());
            assert!(pred.body.hw >= 1.0 && pred.body.top >= 1.0);
            if prey.habitat == Habitat::Aquatic {
                assert!(pred.traits.swim || pred.traits.fly);
            }
            shapes.insert((
                prey.loco as u8,
                prey.body.total_limbs(),
                pred.loco as u8,
                pred.body.total_limbs(),
            ));
        }
        assert!(shapes.len() > 25, "only {} distinct body combos", shapes.len());
    }
}
