//! Schools of magic, spell scrolls and fusions. Every scroll changes how
//! the apprentice interacts with materials; effects live where they apply
//! (spells.rs for casts, hazards/player for passives and dig variants).

use super::achievements::AchievementId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum School {
    Pyromancy,
    Hydromancy,
    Terramancy,
    Cryomancy,
    Aeromancy,
    Fulmancy,
    Alchemy,
    Mycomancy,
}

impl School {
    pub const ALL: [School; 8] = [
        School::Pyromancy,
        School::Hydromancy,
        School::Terramancy,
        School::Cryomancy,
        School::Aeromancy,
        School::Fulmancy,
        School::Alchemy,
        School::Mycomancy,
    ];

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn name(self) -> &'static str {
        [
            "Pyromancy",
            "Hydromancy",
            "Terramancy",
            "Cryomancy",
            "Aeromancy",
            "Fulmancy",
            "Alchemy",
            "Mycomancy",
        ][self.index()]
    }

    pub fn id(self) -> &'static str {
        ["pyro", "hydro", "terra", "cryo", "aero", "fulm", "alch", "myco"][self.index()]
    }

    pub fn from_name(s: &str) -> Option<School> {
        School::ALL
            .into_iter()
            .find(|x| x.name().eq_ignore_ascii_case(s) || x.id() == s)
    }

    pub fn desc(self) -> &'static str {
        match self {
            School::Pyromancy => "Fire, lava and heat. Burn your way down; mind the smoke.",
            School::Hydromancy => "Water and steam. Flood, cool, drown, swim.",
            School::Terramancy => "Stone, sand and gravel. Shape the cave around you.",
            School::Cryomancy => "Ice and frost. Freeze rivers, harden lava.",
            School::Aeromancy => "Wind and gas. Push the world, rise above it.",
            School::Fulmancy => "Lightning. Conducts through water and metal.",
            School::Alchemy => "Acid, explosives and transmutation.",
            School::Mycomancy => "Fungus and rot. Grow light, grow fuel.",
        }
    }

    /// Icon index of the school's sigil in the icon sheet.
    pub fn sigil(self) -> usize {
        self.index()
    }

    /// Robe/accent tint (sRGB 0–255).
    pub fn color(self) -> [u8; 3] {
        match self {
            School::Pyromancy => [222, 92, 52],
            School::Hydromancy => [70, 128, 222],
            School::Terramancy => [170, 128, 78],
            School::Cryomancy => [150, 216, 236],
            School::Aeromancy => [184, 226, 196],
            School::Fulmancy => [226, 206, 92],
            School::Alchemy => [136, 212, 76],
            School::Mycomancy => [96, 196, 168],
        }
    }

    /// The achievement that unlocks this school; `None` = available from the start.
    pub fn unlock(self) -> Option<AchievementId> {
        match self {
            School::Cryomancy => Some(AchievementId::ObsidianBridge),
            School::Aeromancy => Some(AchievementId::LongFall),
            School::Fulmancy => Some(AchievementId::Conductor),
            School::Alchemy => Some(AchievementId::Alchemist),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ScrollId {
    FireBolt,
    MagmaBore,
    SalamanderWard,
    EmberHeart,
    PocketSun,
    Tidecall,
    FloodOrb,
    GillsOfTheDeep,
    Undertow,
    CrumblingTouch,
    StoneShape,
    EarthenGrip,
    SeismicStomp,
    FrostSeed,
    IceLance,
    FrozenPath,
    RimeShell,
    Gust,
    Updraft,
    Levitate,
    AirBubble,
    SparkBolt,
    ChainLightning,
    ArcDrill,
    Grounded,
    AcidFlask,
    AlchemistsCharge,
    Transmutation,
    VolatileCore,
    SporeSowing,
    RotTouch,
    MycelialBridge,
    Symbiosis,
    Beacon,
    GlassFocus,
    IronSoles,
    Blink,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    /// Cast with the use button: mana cost and cooldown (seconds).
    Active { cost: f32, cooldown: f32 },
    /// Held: mana per second.
    Channel { cost_per_sec: f32 },
    /// Always on.
    Passive,
    /// Changes the dig spell. Only the newest dig scroll is in effect.
    Dig,
}

pub struct ScrollDef {
    pub id: ScrollId,
    pub name: &'static str,
    pub desc: &'static str,
    pub flavor: &'static str,
    /// `None` = neutral (available to every wizard).
    pub school: Option<School>,
    pub kind: Kind,
    /// Extra achievement needed beyond the school being unlocked.
    pub unlock: Option<AchievementId>,
}

const fn active(cost: f32, cooldown: f32) -> Kind {
    Kind::Active { cost, cooldown }
}

use AchievementId as A;
use School::*;
use ScrollId as S;

const fn sc(
    id: ScrollId,
    name: &'static str,
    school: Option<School>,
    kind: Kind,
    unlock: Option<AchievementId>,
    desc: &'static str,
    flavor: &'static str,
) -> ScrollDef {
    ScrollDef {
        id,
        name,
        desc,
        flavor,
        school,
        kind,
        unlock,
    }
}

pub static SCROLLS: [ScrollDef; 37] = [
    sc(
        S::FireBolt,
        "Fire Bolt",
        Some(Pyromancy),
        active(12.0, 0.5),
        None,
        "Hurl a bolt of fire that ignites whatever it strikes.",
        "The first spell every apprentice learns. The last some ever cast.",
    ),
    sc(
        S::MagmaBore,
        "Magma Bore",
        Some(Pyromancy),
        Kind::Dig,
        Some(A::DeepDiver),
        "Rock struck by your dig spell sometimes melts into lava.",
        "Dig hot enough and the stone remembers it was once a river.",
    ),
    sc(
        S::SalamanderWard,
        "Salamander Ward",
        Some(Pyromancy),
        Kind::Passive,
        Some(A::Firewalker),
        "Fire and lava cannot hurt you. Water scalds like acid.",
        "The salamander's bargain: every flame, and no rain.",
    ),
    sc(
        S::EmberHeart,
        "Ember Heart",
        Some(Pyromancy),
        Kind::Passive,
        Some(A::HeatStroke),
        "Heat no longer hurts. It gathers in you and bursts out as fire when full.",
        "A hearth where the heart should be.",
    ),
    sc(
        S::PocketSun,
        "Pocket Sun",
        Some(Pyromancy),
        active(35.0, 8.0),
        Some(A::LoreKeeper),
        "Conjure a tiny sun that cooks lava into obsidian, boils water and lights the dark.",
        "Hold it by the corona.",
    ),
    sc(
        S::Tidecall,
        "Tidecall",
        Some(Hydromancy),
        Kind::Channel { cost_per_sec: 18.0 },
        None,
        "Hold to pour a stream of water. Cools lava, douses fire, carries current.",
        "The deep remembers the sea.",
    ),
    sc(
        S::FloodOrb,
        "Flood Orb",
        Some(Hydromancy),
        active(30.0, 4.0),
        None,
        "A sphere of water bursts at the target.",
        "A cistern's worth, folded small.",
    ),
    sc(
        S::GillsOfTheDeep,
        "Gills of the Deep",
        Some(Hydromancy),
        Kind::Passive,
        Some(A::FloodSurvivor),
        "Breathe underwater and in toxic gas.",
        "You stopped needing air somewhere in the Drowned Halls.",
    ),
    sc(
        S::Undertow,
        "Undertow",
        Some(Hydromancy),
        Kind::Passive,
        None,
        "Swim swiftly and dash underwater. Water never drags you down.",
        "Currents part for you.",
    ),
    sc(
        S::CrumblingTouch,
        "Crumbling Touch",
        Some(Terramancy),
        Kind::Dig,
        None,
        "Rock you dig crumbles into sand and gravel instead of vanishing.",
        "Nothing is destroyed. Only rearranged.",
    ),
    sc(
        S::StoneShape,
        "Stone Shape",
        Some(Terramancy),
        active(20.0, 1.0),
        None,
        "Raise a ledge of stone at the target: a step, a bridge, a wall.",
        "The earth lends a hand. It wants it back.",
    ),
    sc(
        S::EarthenGrip,
        "Earthen Grip",
        Some(Terramancy),
        Kind::Passive,
        None,
        "Cling to and climb earth and stone walls without tiring.",
        "Stone holds the faithful.",
    ),
    sc(
        S::SeismicStomp,
        "Seismic Stomp",
        Some(Terramancy),
        Kind::Passive,
        Some(A::CaveIn),
        "Hard landings shatter the ground and shake loose gravel down.",
        "Land like you mean it.",
    ),
    sc(
        S::FrostSeed,
        "Frost Seed",
        Some(Cryomancy),
        active(20.0, 3.0),
        None,
        "Throw a seed that grows ice through water and hardens lava into obsidian.",
        "Cold enough to freeze a river mid-sentence.",
    ),
    sc(
        S::IceLance,
        "Ice Lance",
        Some(Cryomancy),
        active(15.0, 0.8),
        None,
        "A shard of ice that freezes liquids where it strikes and wounds creatures.",
        "Winter, sharpened.",
    ),
    sc(
        S::FrozenPath,
        "Frozen Path",
        Some(Cryomancy),
        Kind::Passive,
        None,
        "Liquids beneath your feet freeze: water to ice, lava to obsidian.",
        "Walk on the river. Walk on the fire.",
    ),
    sc(
        S::RimeShell,
        "Rime Shell",
        Some(Cryomancy),
        Kind::Passive,
        Some(A::StalkersShadow),
        "Lava that touches you hardens to obsidian. Flames gutter out against you.",
        "A coat of frost no fire can find.",
    ),
    sc(
        S::Gust,
        "Gust",
        Some(Aeromancy),
        active(15.0, 0.6),
        None,
        "A cone of wind flings sand, water, gas and creatures away.",
        "The cave exhales.",
    ),
    sc(
        S::Updraft,
        "Updraft",
        Some(Aeromancy),
        active(22.0, 2.0),
        None,
        "A rising column of air lifts you and loose rubble upward.",
        "Fall upward, for once.",
    ),
    sc(
        S::Levitate,
        "Levitate",
        Some(Aeromancy),
        Kind::Passive,
        None,
        "Hold jump in the air to float. Drains mana.",
        "Your feet forget the ground.",
    ),
    sc(
        S::AirBubble,
        "Air Bubble",
        Some(Aeromancy),
        Kind::Passive,
        None,
        "Gas and smoke are blown from your face. Underwater, your breath lasts twice as long.",
        "A pocket of sky, worn like a hood.",
    ),
    sc(
        S::SparkBolt,
        "Spark Bolt",
        Some(Fulmancy),
        active(10.0, 0.6),
        None,
        "Shoot a spark that electrifies water and metal and ignites gas.",
        "Point away from face.",
    ),
    sc(
        S::ChainLightning,
        "Chain Lightning",
        Some(Fulmancy),
        active(25.0, 1.2),
        Some(A::Overcharged),
        "Lightning arcs to the target, charging conductors along its path and leaping between creatures.",
        "It always finds the shortest way down.",
    ),
    sc(
        S::ArcDrill,
        "Arc Drill",
        Some(Fulmancy),
        Kind::Dig,
        None,
        "Your dig spell bites through metal and electrifies conductive cells it touches.",
        "Stone screams in a voltage only the dead can hear.",
    ),
    sc(
        S::Grounded,
        "Grounded",
        Some(Fulmancy),
        Kind::Passive,
        None,
        "Electricity cannot hurt you. Touching charged cells restores mana.",
        "Let it pass through. Keep a little.",
    ),
    sc(
        S::AcidFlask,
        "Acid Flask",
        Some(Alchemy),
        active(20.0, 2.0),
        None,
        "Throw a flask of acid that eats rock (but not obsidian or crystal).",
        "Do not drink. Do not hold for long.",
    ),
    sc(
        S::AlchemistsCharge,
        "Alchemist's Charge",
        Some(Alchemy),
        active(30.0, 4.0),
        Some(A::Pyromaniac),
        "Place a charge that detonates after a short fuse, blasting a crater.",
        "Mining, but faster and louder.",
    ),
    sc(
        S::Transmutation,
        "Transmutation",
        Some(Alchemy),
        active(35.0, 6.0),
        None,
        "Metal, ferrite and obsidian at the target turn into shard veins; stone becomes sand.",
        "Lead to gold was always the easy part.",
    ),
    sc(
        S::VolatileCore,
        "Volatile Core",
        Some(Alchemy),
        Kind::Passive,
        Some(A::Demolitionist),
        "Explosions you cause leave veins of aether shards behind.",
        "Destruction is just mining with enthusiasm.",
    ),
    sc(
        S::SporeSowing,
        "Spore Sowing",
        Some(Mycomancy),
        active(15.0, 1.5),
        None,
        "Throw spores that grow glowing, flammable fungus over rock.",
        "Light for the dark. Fuel for the fire.",
    ),
    sc(
        S::RotTouch,
        "Rot Touch",
        Some(Mycomancy),
        Kind::Dig,
        None,
        "Wood, fungus, grass and dirt rot away at a touch, leaving glowing fungus at the edges.",
        "Everything returns to the mycelium.",
    ),
    sc(
        S::MycelialBridge,
        "Mycelial Bridge",
        Some(Mycomancy),
        active(20.0, 2.0),
        Some(A::Gardener),
        "Grow a bridge of fungus from your feet toward the cursor.",
        "The network will carry you.",
    ),
    sc(
        S::Symbiosis,
        "Symbiosis",
        Some(Mycomancy),
        Kind::Passive,
        Some(A::FusionAdept),
        "Touching fungus heals you and restores mana.",
        "It feeds you. You feed it. Don't ask how.",
    ),
    sc(
        S::Beacon,
        "Beacon",
        None,
        Kind::Passive,
        None,
        "Your glow reveals far more of the cave, but lightseekers flock to it.",
        "Hope is visible from a long way off.",
    ),
    sc(
        S::GlassFocus,
        "Glass Focus",
        None,
        Kind::Dig,
        Some(A::SpeedDigger),
        "Your dig spell cuts almost instantly, but you take double damage.",
        "Sharp enough to cut the planet. And you.",
    ),
    sc(
        S::IronSoles,
        "Iron Soles",
        None,
        Kind::Passive,
        None,
        "Walk on molten metal. You sink fast in water and wade through sand.",
        "Every step is a decision.",
    ),
    sc(
        S::Blink,
        "Blink",
        None,
        active(20.0, 1.5),
        Some(A::Lightless),
        "Step up to 14 cells toward the cursor, even through rock.",
        "The dark has doors, if you know where to knock.",
    ),
];

impl ScrollId {
    pub const COUNT: usize = 37;

    pub fn all() -> impl Iterator<Item = ScrollId> {
        SCROLLS.iter().map(|d| d.id)
    }

    pub fn def(self) -> &'static ScrollDef {
        &SCROLLS[self as usize]
    }

    /// Icon index in the icon sheet.
    pub fn icon(self) -> usize {
        8 + self as usize
    }

    pub fn is_active(self) -> bool {
        matches!(self.def().kind, Kind::Active { .. } | Kind::Channel { .. })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum FusionId {
    SteamBurst,
    MagmaSurge,
    ThermalShock,
    Firestorm,
    PlasmaLance,
    Napalm,
    SporeBomb,
    Mudslide,
    Glacier,
    Typhoon,
    StormFlood,
    AcidRain,
    SwampBloom,
    FrozenEarth,
    Sandstorm,
    Railshot,
    Petrify,
    RootLattice,
    Blizzard,
    CryoArc,
    Shatter,
    Rimebloom,
    Thunderstorm,
    Miasma,
    SporeGale,
    Electrolysis,
    StormSpores,
    DecayBloom,
}

pub struct FusionDef {
    pub id: FusionId,
    pub name: &'static str,
    pub desc: &'static str,
    pub schools: (School, School),
    pub cost: f32,
    pub cooldown: f32,
}

use FusionId as F;

const fn fu(
    id: FusionId,
    name: &'static str,
    a: School,
    b: School,
    cost: f32,
    cooldown: f32,
    desc: &'static str,
) -> FusionDef {
    FusionDef {
        id,
        name,
        desc,
        schools: (a, b),
        cost,
        cooldown,
    }
}

pub static FUSIONS: [FusionDef; 28] = [
    fu(
        F::SteamBurst,
        "Steam Burst",
        Pyromancy,
        Hydromancy,
        30.0,
        4.0,
        "Scalding steam floods the target and water there flashes to steam.",
    ),
    fu(
        F::MagmaSurge,
        "Magma Surge",
        Pyromancy,
        Terramancy,
        35.0,
        5.0,
        "Rock at the target melts into a pool of lava.",
    ),
    fu(
        F::ThermalShock,
        "Thermal Shock",
        Pyromancy,
        Cryomancy,
        30.0,
        4.0,
        "Stone, basalt, obsidian and ice at the target shatter into gravel.",
    ),
    fu(
        F::Firestorm,
        "Firestorm",
        Pyromancy,
        Aeromancy,
        35.0,
        5.0,
        "A whirl of fire sweeps forward, igniting everything it passes.",
    ),
    fu(
        F::PlasmaLance,
        "Plasma Lance",
        Pyromancy,
        Fulmancy,
        40.0,
        5.0,
        "A beam vaporises a straight tunnel, igniting and charging what it touches.",
    ),
    fu(
        F::Napalm,
        "Napalm",
        Pyromancy,
        Alchemy,
        30.0,
        4.0,
        "A flask of burning oil splashes over the target.",
    ),
    fu(
        F::SporeBomb,
        "Spore Bomb",
        Pyromancy,
        Mycomancy,
        30.0,
        4.0,
        "A spore pod blooms gas and fungus, then ignites.",
    ),
    fu(
        F::Mudslide,
        "Mudslide",
        Hydromancy,
        Terramancy,
        30.0,
        4.0,
        "Dirt and sand at the target slump into a flowing slurry.",
    ),
    fu(
        F::Glacier,
        "Glacier",
        Hydromancy,
        Cryomancy,
        35.0,
        6.0,
        "All water in a wide radius flash-freezes to ice.",
    ),
    fu(
        F::Typhoon,
        "Typhoon",
        Hydromancy,
        Aeromancy,
        30.0,
        4.0,
        "Liquid around you is torn up and hurled toward the cursor.",
    ),
    fu(
        F::StormFlood,
        "Storm Flood",
        Hydromancy,
        Fulmancy,
        35.0,
        5.0,
        "An electrified wave of water bursts at the target.",
    ),
    fu(
        F::AcidRain,
        "Acid Rain",
        Hydromancy,
        Alchemy,
        30.0,
        5.0,
        "Acid falls from above over the target.",
    ),
    fu(
        F::SwampBloom,
        "Swamp Bloom",
        Hydromancy,
        Mycomancy,
        25.0,
        3.0,
        "Water at the target turns into fungus mats you can stand on.",
    ),
    fu(
        F::FrozenEarth,
        "Frozen Earth",
        Terramancy,
        Cryomancy,
        25.0,
        3.0,
        "Loose sand and gravel at the target freeze solid, halting cave-ins.",
    ),
    fu(
        F::Sandstorm,
        "Sandstorm",
        Terramancy,
        Aeromancy,
        30.0,
        4.0,
        "Sand, gravel and ash around you are flung at the cursor, burying creatures.",
    ),
    fu(
        F::Railshot,
        "Railshot",
        Terramancy,
        Fulmancy,
        35.0,
        4.0,
        "A charged ferrite slug bores straight through rock.",
    ),
    fu(
        F::Petrify,
        "Petrify",
        Terramancy,
        Alchemy,
        30.0,
        5.0,
        "Liquids at the target turn to stone; creatures there are stunned.",
    ),
    fu(
        F::RootLattice,
        "Root Lattice",
        Terramancy,
        Mycomancy,
        25.0,
        3.0,
        "Roots grow from your feet toward the cursor: a lattice to climb.",
    ),
    fu(
        F::Blizzard,
        "Blizzard",
        Cryomancy,
        Aeromancy,
        30.0,
        4.0,
        "A cone of frost freezes liquids, smothers fire and chills creatures.",
    ),
    fu(
        F::CryoArc,
        "Cryo Arc",
        Cryomancy,
        Fulmancy,
        30.0,
        3.0,
        "Lightning that freezes liquids along its path and stuns creatures.",
    ),
    fu(
        F::Shatter,
        "Shatter",
        Cryomancy,
        Alchemy,
        30.0,
        4.0,
        "Obsidian and crystal at the target turn brittle and crumble.",
    ),
    fu(
        F::Rimebloom,
        "Rimebloom",
        Cryomancy,
        Mycomancy,
        25.0,
        3.0,
        "Fungus at the target turns to frost that creeps through water.",
    ),
    fu(
        F::Thunderstorm,
        "Thunderstorm",
        Aeromancy,
        Fulmancy,
        40.0,
        6.0,
        "Lightning strikes down around the target, igniting gas.",
    ),
    fu(
        F::Miasma,
        "Miasma",
        Aeromancy,
        Alchemy,
        25.0,
        4.0,
        "A cloud of toxic, flammable gas billows at the target.",
    ),
    fu(
        F::SporeGale,
        "Spore Gale",
        Aeromancy,
        Mycomancy,
        25.0,
        3.0,
        "A gust of spores plants fungus wherever it lands.",
    ),
    fu(
        F::Electrolysis,
        "Electrolysis",
        Fulmancy,
        Alchemy,
        30.0,
        4.0,
        "Water at the target splits into explosive gas and is sparked.",
    ),
    fu(
        F::StormSpores,
        "Storm Spores",
        Fulmancy,
        Mycomancy,
        25.0,
        3.0,
        "Fungus at the target crackles with sparks and catches fire.",
    ),
    fu(
        F::DecayBloom,
        "Decay Bloom",
        Alchemy,
        Mycomancy,
        25.0,
        3.0,
        "Organic matter at the target rots into acid and spores.",
    ),
];

impl FusionId {
    pub const COUNT: usize = 28;

    pub fn all() -> impl Iterator<Item = FusionId> {
        FUSIONS.iter().map(|d| d.id)
    }

    pub fn def(self) -> &'static FusionDef {
        &FUSIONS[self as usize]
    }

    pub fn icon(self) -> usize {
        45 + self as usize
    }

    /// The fusion of two different schools (order doesn't matter).
    pub fn for_pair(a: School, b: School) -> Option<FusionId> {
        if a == b {
            return None;
        }
        FusionId::all().find(|f| {
            let (x, y) = f.def().schools;
            (x, y) == (a, b) || (x, y) == (b, a)
        })
    }

    pub fn name_id(self) -> String {
        format!("{self:?}")
    }

    pub fn from_name(s: &str) -> Option<FusionId> {
        FusionId::all().find(|f| f.name_id() == s)
    }
}

impl ScrollId {
    pub fn name_id(self) -> String {
        format!("{self:?}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_match_ids() {
        for (i, d) in SCROLLS.iter().enumerate() {
            assert_eq!(d.id as usize, i, "{}", d.name);
        }
        for (i, d) in FUSIONS.iter().enumerate() {
            assert_eq!(d.id as usize, i, "{}", d.name);
        }
    }

    #[test]
    fn every_pair_has_exactly_one_fusion() {
        let mut seen = std::collections::HashSet::new();
        for (i, &a) in School::ALL.iter().enumerate() {
            for &b in &School::ALL[i + 1..] {
                let f = FusionId::for_pair(a, b).unwrap_or_else(|| panic!("no fusion for {a:?}+{b:?}"));
                assert_eq!(FusionId::for_pair(b, a), Some(f));
                assert!(seen.insert(f), "{f:?} used twice");
            }
            assert_eq!(FusionId::for_pair(a, a), None);
        }
        assert_eq!(seen.len(), 28);
    }

    #[test]
    fn every_school_has_at_least_four_scrolls() {
        for s in School::ALL {
            let n = SCROLLS.iter().filter(|d| d.school == Some(s)).count();
            assert!(n >= 4, "{s:?} has {n}");
        }
        assert!(SCROLLS.iter().filter(|d| d.school.is_none()).count() >= 3);
        let starting = School::ALL.iter().filter(|s| s.unlock().is_none()).count();
        assert!((3..=4).contains(&starting));
    }
}

/// Indices of non-scroll icons in the icon sheet (16 columns × 8 rows).
pub mod icon {
    pub const SHARD: usize = 73;
    pub const LIGHT_ORB: usize = 74;
    pub const CHEST_CLOSED: usize = 75;
    pub const ALTAR: usize = 77;
    pub const SHRINE: usize = 78;
    pub const REMAINS: usize = 79;
    pub const JOURNAL: usize = 80;
    pub const HEART_FULL: usize = 81;
    pub const HEART_HALF: usize = 82;
    pub const HEART_EMPTY: usize = 83;
    pub const LOCK: usize = 84;
    pub const FLAME: usize = 85;
    pub const BUBBLE: usize = 86;
    pub const THERMOMETER: usize = 87;
    pub const SKULL: usize = 88;
    pub const TROPHY: usize = 89;
    pub const STAFF: usize = 90;
    pub const UNKNOWN: usize = 91;
    pub const COMPASS: usize = 92;
    pub const HOURGLASS: usize = 93;
    pub const MANA: usize = 94;
    pub const EYE_CLOSED: usize = 95;
    pub const SMART_DIG: usize = 96;
    pub const CURSOR_DIG: usize = 97;
    pub const MIMIC: usize = 98;
    pub const STALKER: usize = 99;
    pub const RUNE: usize = 100;
}
