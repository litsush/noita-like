//! Body mods: the 53 sets, their tiers, and the stats they change.
//!
//! A player owns any number of sets but equips one per body slot. Systems
//! never look at sets directly: they read [`ModStats`], which
//! [`stats_for`] builds from what is equipped.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use super::items::Item;

pub const SETS: usize = 53;
pub const LEGENDARY: u8 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Slot {
    Head,
    Eyes,
    Torso,
    Back,
    Arms,
    Hands,
    Legs,
    Feet,
}

impl Slot {
    pub const ALL: [Slot; 8] = [
        Slot::Head,
        Slot::Eyes,
        Slot::Torso,
        Slot::Back,
        Slot::Arms,
        Slot::Hands,
        Slot::Legs,
        Slot::Feet,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Slot::Head => "Head",
            Slot::Eyes => "Eyes",
            Slot::Torso => "Torso",
            Slot::Back => "Back",
            Slot::Arms => "Arms",
            Slot::Hands => "Hands",
            Slot::Legs => "Legs",
            Slot::Feet => "Feet",
        }
    }
}

/// Set ids. The order matches the rows of `mods.png`.
pub mod set {
    pub const ROCKET_FEET: u8 = 0;
    pub const STICKY_SOLES: u8 = 1;
    pub const SPRING_HEELS: u8 = 2;
    pub const HUSTLE_TREADS: u8 = 3;
    pub const AQUA_FLIPPERS: u8 = 4;
    pub const STOMPERS: u8 = 5;
    pub const DASH_PISTONS: u8 = 6;
    pub const CARGO_PANTS: u8 = 7;
    pub const THERMAL_LEGGINGS: u8 = 8;
    pub const ROOT_WALKERS: u8 = 9;
    pub const SHOCK_ABSORBERS: u8 = 10;
    pub const JACKHAMMER_KNEES: u8 = 11;
    pub const O2_BACKPACK: u8 = 12;
    pub const GLIDE_WINGS: u8 = 13;
    pub const SHOULDER_TURRET: u8 = 14;
    pub const BATTERY_PACK: u8 = 15;
    pub const SPRINKLER_PACK: u8 = 16;
    pub const PACK_MULE: u8 = 17;
    pub const DRONE_DOCK: u8 = 18;
    pub const BEACON_RACK: u8 = 19;
    pub const PLATING: u8 = 20;
    pub const FILTER_LUNGS: u8 = 21;
    pub const MEDI_CORE: u8 = 22;
    pub const REACTOR_HEART: u8 = 23;
    pub const CAMO_SKIN: u8 = 24;
    pub const SYNTH_BELLY: u8 = 25;
    pub const BUDDY_BREATHER: u8 = 26;
    pub const ANTENNA_ARRAY: u8 = 27;
    pub const TRADE_CHIP: u8 = 28;
    pub const HIVE_MIND: u8 = 29;
    pub const BOTANISTS_BONNET: u8 = 30;
    pub const HEADLAMP: u8 = 31;
    pub const DOME_BRAIN: u8 = 32;
    pub const WEATHER_VANE: u8 = 33;
    pub const XRAY_SPECS: u8 = 34;
    pub const NIGHT_VISION: u8 = 35;
    pub const THREAT_LENS: u8 = 36;
    pub const APPRAISAL_MONOCLE: u8 = 37;
    pub const ZOOM_GOGGLES: u8 = 38;
    pub const GEO_VISOR: u8 = 39;
    pub const EXTENDO_ARMS: u8 = 40;
    pub const DRILL_ARMS: u8 = 41;
    pub const POWER_LIFTERS: u8 = 42;
    pub const SHIELD_ARM: u8 = 43;
    pub const FARM_HANDS: u8 = 44;
    pub const WELDER_ARMS: u8 = 45;
    pub const BOLT_ENHANCEMENTS: u8 = 46;
    pub const MIDAS_MITTS: u8 = 47;
    pub const CRYO_PALMS: u8 = 48;
    pub const BOOM_MITTS: u8 = 49;
    pub const GREEN_FINGERS: u8 = 50;
    pub const GRAPPLE_GLOVE: u8 = 51;
    pub const HEALING_HANDS: u8 = 52;
}

pub struct ModSet {
    pub name: &'static str,
    pub slot: Slot,
    /// (tier name, what it does) for tiers I, II, III and Legendary.
    pub tiers: [(&'static str, &'static str); 4],
}

const fn m(name: &'static str, slot: Slot, tiers: [(&'static str, &'static str); 4]) -> ModSet {
    ModSet { name, slot, tiers }
}

#[rustfmt::skip]
pub static MOD_SETS: [ModSet; SETS] = [
    m("Rocket Feet", Slot::Feet, [
        ("Booster Thrust", "Double jump with a rocket burst."),
        ("Rocket Hover", "Hold jump after the second jump for 2 seconds of thrust."),
        ("Rocket Flight", "5 seconds of flight per jump."),
        ("Perma-thrust", "Infinite flight."),
    ]),
    m("Sticky Soles", Slot::Feet, [
        ("Gecko Grip", "Slide slowly down walls and wall-jump."),
        ("Wall Crawl", "Climb walls."),
        ("Cling Wrap", "Climb fast and hang without sliding."),
        ("Gravity Is A Suggestion", "Climb at running speed and never slip."),
    ]),
    m("Spring Heels", Slot::Feet, [
        ("Bounce", "+30% jump height."),
        ("Boing", "+60% jump height. Landing on a creature hurts it."),
        ("Kangaroo Court", "Hold down to charge a triple-height jump."),
        ("Moon Rules", "Low gravity all the time."),
    ]),
    m("Hustle Treads", Slot::Feet, [
        ("Brisk Walk", "+20% speed."),
        ("Jog Protocol", "+40% speed."),
        ("Zoomies", "+60% speed."),
        ("Late For Work", "Double speed, and you run across water."),
    ]),
    m("Aqua Flippers", Slot::Feet, [
        ("Doggy Paddle+", "Swim 50% faster."),
        ("Dolphin Kick", "Swim twice as fast."),
        ("Torpedo", "Triple swim speed. Water costs no extra oxygen."),
        ("Certified Fish", "Breathe underwater."),
    ]),
    m("Stompers", Slot::Feet, [
        ("Heavy Landing", "Landing from a height digs a small crater."),
        ("Crater Maker", "A bigger crater that hurts creatures."),
        ("Seismic Drop", "Press down in the air to ground-pound."),
        ("Tectonic Plates", "A huge ground-pound that leaves everything nearby reeling at half speed."),
    ]),
    m("Dash Pistons", Slot::Legs, [
        ("Sidestep", "A short dash."),
        ("Double Dash", "Two dash charges."),
        ("Phase Dash", "Dash through creatures unharmed."),
        ("Blink And You Miss Me", "Your dash passes through thin walls."),
    ]),
    m("Cargo Pants", Slot::Legs, [
        ("Deep Pockets", "+10 inventory slots."),
        ("Deeper Pockets", "+20 inventory slots."),
        ("Pocket Dimension", "+30 inventory slots."),
        ("Bag Of Holding My Beer", "Synthesize from, and open, the colony stockpile from anywhere."),
    ]),
    m("Thermal Leggings", Slot::Legs, [
        ("Warm Knees", "Cold drains you 50% slower."),
        ("Toasty", "Immune to cold."),
        ("Space Heater", "Warms teammates near you."),
        ("Walking Summer", "Plants near you ignore night and cold."),
    ]),
    m("Root Walkers", Slot::Legs, [
        ("Green Stride", "Grass grows where you walk."),
        ("Seed Trail", "Empty planters you pass are replanted from your seeds."),
        ("Harvest Stride", "Plants you pass are harvested."),
        ("Johnny Applesprint", "Plants you pass are harvested, replanted and watered."),
    ]),
    m("Shock Absorbers", Slot::Legs, [
        ("Brace", "Take 15% less damage."),
        ("Brace Harder", "Take 30% less damage and shrug off webs."),
        ("Braced For Impact", "Take 45% less damage."),
        ("Unbothered", "Take 60% less damage. Immune while standing still."),
    ]),
    m("Jackhammer Knees", Slot::Legs, [
        ("Kneel Drill", "Hold down to dig beneath you."),
        ("Power Squat", "Digs twice as fast."),
        ("Free Fall Drill", "Keeps digging while you fall."),
        ("Elevator Going Down", "Drills twice as deep with every stroke."),
    ]),
    m("O2 Backpack", Slot::Back, [
        ("Spare Lung", "+50% oxygen."),
        ("Scuba Flex", "+100% oxygen."),
        ("Tank Top", "+200% oxygen, and it refills twice as fast."),
        ("Photosynthesis, Baby", "No oxygen drain in daylight."),
    ]),
    m("Glide Wings", Slot::Back, [
        ("Flappy", "Hold jump in the air to fall slowly."),
        ("Wingsuit", "Glide forward fast."),
        ("Updraft", "Gain height while gliding in rain."),
        ("Technically Flying", "Glide without losing height."),
    ]),
    m("Shoulder Turret", Slot::Back, [
        ("Lil' Pew", "Fires by itself at the nearest predator."),
        ("Pew Pew", "Fires faster."),
        ("Pew Cubed", "Fires at three targets."),
        ("Pew Infinity", "Homing shots that never miss."),
    ]),
    m("Battery Pack", Slot::Back, [
        ("Extra Juice", "+50% battery."),
        ("Double A", "+100% battery."),
        ("Power Bank", "+200% battery and no lockout when drained."),
        ("Infinite Scroll", "The battery recharges three times as fast."),
    ]),
    m("Sprinkler Pack", Slot::Back, [
        ("Mist Me", "Waters plants close to you."),
        ("Drizzle", "Waters plants further away."),
        ("Monsoon", "Wider still, and plants near you grow 25% faster."),
        ("Personal Raincloud", "A cloud follows you. Plants under it grow twice as fast."),
    ]),
    m("Pack Mule Frame", Slot::Back, [
        ("Lint Roller", "Pickup radius ×2."),
        ("Vacuum", "Pickup radius ×3."),
        ("Hoover Dam", "Pickup radius ×5. Quick-stacks when you enter a dome."),
        ("Everything Comes To Me", "Pickup across the screen, through walls."),
    ]),
    m("Drone Dock", Slot::Back, [
        ("Buddy", "A drone follows you and fetches drops."),
        ("Buddy Pro", "It also mines ore near you."),
        ("Buddy Squad", "Two drones."),
        ("Union Busted", "Four drones."),
    ]),
    m("Beacon Rack", Slot::Back, [
        ("Ping", "Drop a waypoint all teammates see."),
        ("Recall", "Teleport home from the map (cooldown)."),
        ("Rally", "Teammates can teleport to you."),
        ("Group Chat", "Teleport to any teammate or dome."),
    ]),
    m("Plating", Slot::Torso, [
        ("Tin Vest", "+25 health."),
        ("Steel Hoodie", "+50 health."),
        ("Titanium Turtleneck", "+100 health."),
        ("Legally A Tank", "+200 health. Attackers take damage."),
    ]),
    m("Filter Lungs", Slot::Torso, [
        ("Sniff Test", "Spore gas hurts 50% less."),
        ("Nose Plugs", "Immune to spore gas."),
        ("Clean Air Act", "Clears gas around you."),
        ("Breathes In Menacingly", "Gas you clear becomes oxygen for the planet."),
    ]),
    m("Medi-Core", Slot::Torso, [
        ("Self-Care", "Slow health regeneration anywhere."),
        ("Group Hug", "Heals teammates near you."),
        ("Defib", "Revives downed teammates near you."),
        ("HMO Premium Plus", "The whole team regenerates anywhere."),
    ]),
    m("Reactor Heart", Slot::Torso, [
        ("Pocket Reactor", "Slowly charges pylons near you."),
        ("Walking Outlet", "You power machines and domes near you."),
        ("Grid Daddy", "A larger radius."),
        ("I Am The Power Plant", "A huge radius: whole bases run off you."),
    ]),
    m("Camo Skin", Slot::Torso, [
        ("Beige Mode", "Predators notice you from half as far."),
        ("Ghosting", "Unseen while standing still."),
        ("Left On Read", "Predators ignore you."),
        ("Do Not Disturb", "Predators flee from you."),
    ]),
    m("Synth Belly", Slot::Torso, [
        ("Snack Crafting", "Synthesize anywhere."),
        ("Bulk Order", "Recipes cost 15% less."),
        ("Supply Chain", "Recipes cost 30% less and draw on the stockpile from anywhere."),
        ("Free Shipping", "Half your crafts come out doubled."),
    ]),
    m("Buddy Breather", Slot::Torso, [
        ("Share Air", "Teammates near you use oxygen 50% slower."),
        ("Air Supply", "Teammates near you use no oxygen."),
        ("Lung Lease", "You refill their tanks."),
        ("Atmosphere Subscription", "You count as a dome: everyone near you breathes freely."),
    ]),
    m("Antenna Array", Slot::Head, [
        ("Bars", "The map reveals 50% further."),
        ("Full Bars", "The map reveals twice as far."),
        ("5G", "Ore veins are marked on the map."),
        ("Satellite Uplink", "The whole map is revealed."),
    ]),
    m("Trade Chip", Slot::Head, [
        ("Haggle", "Sell for 10% more."),
        ("Hard Bargain", "Sell for 20% more."),
        ("Remote Work", "Sell for 35% more, and trade with Earth from anywhere."),
        ("Insider Trading", "Sell for 75% more."),
    ]),
    m("Hive Mind", Slot::Head, [
        ("Middle Manager", "Robots near you work 25% faster."),
        ("Micromanager", "50% faster. Edit robot programs from the map."),
        ("Synergy", "Robots near you work twice as fast."),
        ("Hostile Takeover", "Every robot works twice as fast and uses no power."),
    ]),
    m("Botanist's Bonnet", Slot::Head, [
        ("Green Thumb", "+25% yield harvesting by hand."),
        ("Greener Thumb", "+50% yield. See plant genes when you point at a plant."),
        ("Greenest Thumb", "Double yield and extra seeds."),
        ("Photosynthesis Hat", "Hybrids you splice get +1 to every gene."),
    ]),
    m("Headlamp", Slot::Head, [
        ("Flashlight", "A light on your helmet."),
        ("High Beams", "A brighter, wider light."),
        ("Floodlight", "Lights the whole screen."),
        ("Second Sun", "Plants near you grow faster, even in caves."),
    ]),
    m("Dome Brain", Slot::Head, [
        ("Blueprint Memory", "Dome kits cost 10% less."),
        ("Efficient Layouts", "Dome kits cost 20% less."),
        ("Arcology", "Dome kits cost 30% less. Dome Domes near you build twice as fast."),
        ("Big Dome Energy", "Domes you place produce 25% more."),
    ]),
    m("Weather Vane", Slot::Head, [
        ("Forecast", "See the coming weather."),
        ("Rain Dance", "Call rain (long cooldown)."),
        ("Storm Chaser", "You move and recharge faster in rain."),
        ("Cloud Computing", "It always drizzles on the colony: domes top up their water."),
    ]),
    m("X-Ray Specs", Slot::Eyes, [
        ("Squint", "Ore near you sparkles through rock."),
        ("Stare", "Further, and shows creatures."),
        ("Glare", "Further still."),
        ("I Can See Your Skeleton", "Everything on screen."),
    ]),
    m("Night Vision", Slot::Eyes, [
        ("Carrot Diet", "Darkness is less dark."),
        ("Cat Mode", "Much less dark."),
        ("Owl Mode", "Nearly bright."),
        ("Dark Mode Disabled", "Nothing is ever dark."),
    ]),
    m("Threat Lens", Slot::Eyes, [
        ("Lucky Shot", "10% of bolts crit for double damage."),
        ("Weak Points", "20% crits. See creature health."),
        ("Bullseye", "30% crits, for triple damage."),
        ("Aimbot (Legal)", "Bolts curve toward predators."),
    ]),
    m("Appraisal Monocle", Slot::Eyes, [
        ("Finder's Fee", "Creatures drop 25% more."),
        ("Good Eye", "Creatures drop 50% more."),
        ("Connoisseur", "Creatures sometimes drop rare ore."),
        ("Loot Goblin", "Double drops."),
    ]),
    m("Zoom Goggles", Slot::Eyes, [
        ("Step Back", "See 15% further."),
        ("Wide Angle", "See 30% further."),
        ("Panorama", "See 50% further."),
        ("Eagle Eye", "Hold the scout key to send the camera off toward the cursor."),
    ]),
    m("Geo Visor", Slot::Eyes, [
        ("Long Reach Planning", "Place things 50% further away."),
        ("Site Survey", "Spots where the dome kit in your hand fits are marked on the ground."),
        ("Remote Build", "Place from three times as far."),
        ("Sim City Mode", "Place anywhere you can see."),
    ]),
    m("Extendo-Arms", Slot::Arms, [
        ("Arm++", "Longer arms: more reach for everything."),
        ("Arm++ Premium", "An extra arm that does a nearby task of your choice."),
        ("Arm++ Ultra", "A second automated arm, and more reach for all of you."),
        ("Armnipresence", "A free-flying arm you send anywhere on the revealed map."),
    ]),
    m("Drill Arms", Slot::Arms, [
        ("Dig Dug", "Dig 30% faster."),
        ("Bore Dom", "Dig 60% faster and wider."),
        ("Tunnel Vision", "Dig twice as fast, wider still."),
        ("Mine Craft", "Rock vaporizes instantly."),
    ]),
    m("Power Lifters", Slot::Arms, [
        ("Lift With Your Legs", "Stacks hold 50% more."),
        ("Two Trips", "Stacks hold double."),
        ("One Trip", "Stacks hold triple."),
        ("Do You Even Lift", "Stacks hold ten times as much."),
    ]),
    m("Shield Arm", Slot::Arms, [
        ("Parry", "A shield absorbs 20 damage, then recharges."),
        ("Bubble", "The shield absorbs 40."),
        ("Deflector", "The shield absorbs 70 and shrugs off webs."),
        ("Force Field Trip", "Teammates near you take half damage."),
    ]),
    m("Farm Hands", Slot::Arms, [
        ("Quick Pick", "Harvest a whole dome at once."),
        ("Quick Plant", "Plant a whole dome at once."),
        ("Quick Everything", "Also water and fertilize a whole dome at once."),
        ("Agricultural Revolution", "Works on every dome in view."),
    ]),
    m("Welder Arms", Slot::Arms, [
        ("Tack Weld", "Pipes go twice as far per item."),
        ("Pipe Dream", "Pipes go four times as far."),
        ("Hot Swap", "Dismantling puts the contents straight into your pack."),
        ("Unionized", "Machines you place run 25% faster."),
    ]),
    m("Bolt Enhancements", Slot::Hands, [
        ("Overcharge", "Hold fire to charge a bigger, stronger bolt that uses more battery."),
        ("Jack Up", "Bolts are larger and hit harder."),
        ("Rug Pull", "Every shot fires two bolts for the price of one."),
        ("Straight Up Scammed", "Bolts use 80% less energy."),
    ]),
    m("Midas Mitts", Slot::Hands, [
        ("Sticky Fingers", "+15% ore from digging."),
        ("Gold Digger", "+30% ore."),
        ("Fool's Gold", "+50% ore. Stone sometimes yields gold."),
        ("Everything I Touch", "Stone sometimes yields any ore."),
    ]),
    m("Cryo Palms", Slot::Hands, [
        ("Chill Pill", "Bolts slow creatures."),
        ("Brain Freeze", "Bolts freeze water into ice."),
        ("Ice Age", "Chilled creatures take 50% more damage."),
        ("Absolute Zero Chill", "A freezing aura around you."),
    ]),
    m("Boom Mitts", Slot::Hands, [
        ("Pop", "Bolts burst on impact, digging a little."),
        ("Bang", "Bigger bursts."),
        ("Kaboom", "Bigger still, and they set things alight."),
        ("Controlled Demolition", "Huge bursts."),
    ]),
    m("Green Fingers", Slot::Hands, [
        ("Tickle", "Touch a growing plant to speed it along (cooldown)."),
        ("Poke", "A shorter cooldown."),
        ("Coax", "Touched plants mature at once."),
        ("Miracle Grow", "Plants near you grow three times as fast."),
    ]),
    m("Grapple Glove", Slot::Hands, [
        ("Yoink", "Fire a short grapple line and pull yourself in."),
        ("Long Yoink", "A longer line that also pulls drops to you."),
        ("Swing State", "Keep your momentum when you let go."),
        ("Spider, Man", "Unlimited range."),
    ]),
    m("Healing Hands", Slot::Hands, [
        ("High Five", "Touch a teammate to heal them."),
        ("Fist Bump", "Touch a downed teammate to revive them."),
        ("Finger Guns", "Heal at range."),
        ("Thoughts And Prayers (Effective)", "Heal the whole team from anywhere."),
    ]),
];

pub fn tier_label(tier: u8) -> &'static str {
    match tier {
        1 => "I",
        2 => "II",
        3 => "III",
        _ => "Legendary",
    }
}

/// Sets every colony can synthesize without a blueprint.
pub const STARTER_SETS: [u8; 6] = [
    set::ROCKET_FEET,
    set::EXTENDO_ARMS,
    set::BOLT_ENHANCEMENTS,
    set::O2_BACKPACK,
    set::DRILL_ARMS,
    set::PLATING,
];

/// Price of a set's blueprint in credits (0 for starter sets).
pub fn blueprint_price(set: u8) -> u32 {
    if STARTER_SETS.contains(&set) || set as usize >= SETS {
        return 0;
    }
    // Spread between 600 and 2400 so there is always something affordable.
    600 + (set as u32 * 7 % 10) * 200
}

/// What synthesizing a tier (1–4) of any set costs.
pub fn tier_cost(tier: u8) -> Vec<(Item, u32)> {
    match tier {
        1 => vec![(Item::Iron, 12), (Item::Copper, 8), (Item::Silicon, 4)],
        2 => vec![
            (Item::Iron, 24),
            (Item::Copper, 16),
            (Item::Silicon, 10),
            (Item::Gold, 6),
        ],
        3 => vec![
            (Item::Iron, 40),
            (Item::Silicon, 20),
            (Item::Gold, 12),
            (Item::Titanium, 12),
            (Item::Xenite, 4),
        ],
        _ => vec![
            (Item::Gold, 20),
            (Item::Titanium, 30),
            (Item::Xenite, 16),
            (Item::Aurorium, 8),
        ],
    }
}

/// What an automated arm (Extendo-Arms II+) does.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ArmTask {
    #[default]
    Off,
    /// Mine ore nearby.
    Mine,
    /// Water plants nearby.
    Water,
    /// Harvest ripe plants nearby.
    Harvest,
    /// Pick up drops nearby.
    Collect,
    /// Help Dome Domes nearby build.
    Build,
}

impl ArmTask {
    pub const ALL: [ArmTask; 6] = [
        ArmTask::Off,
        ArmTask::Mine,
        ArmTask::Water,
        ArmTask::Harvest,
        ArmTask::Collect,
        ArmTask::Build,
    ];

    pub fn name(self) -> &'static str {
        match self {
            ArmTask::Off => "Rest",
            ArmTask::Mine => "Mine nearby ore",
            ArmTask::Water => "Water nearby plants",
            ArmTask::Harvest => "Harvest nearby plants",
            ArmTask::Collect => "Pick up nearby drops",
            ArmTask::Build => "Help Dome Domes build",
        }
    }
}

/// Everything about a player that equipped mods can change. Systems read
/// these instead of constants.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ModStats {
    // ---- movement (applied by the player's own client) ----
    pub speed: f32,
    pub jump: f32,
    pub air_jumps: u8,
    /// Seconds of rocket thrust after the last air jump.
    pub thrust_secs: f32,
    /// 0 none, 1 slide and wall-jump, 2 climb, 3 climb fast and hang, 4 run up walls.
    pub wall: u8,
    pub charge_jump: bool,
    /// Landing on a creature hurts it (and bounces the player).
    pub head_stomp: bool,
    pub gravity: f32,
    pub water_run: bool,
    pub swim: f32,
    /// 0 none, 1 crater, 2 bigger and hurts, 3 ground-pound, 4 huge and stuns.
    pub stomp: u8,
    pub dashes: u8,
    pub dash_phase: bool,
    pub dash_walls: bool,
    /// 0 none, 1 slow fall, 2 fast forward, 3 rises in rain, 4 level flight.
    pub glide: u8,
    /// 0 none, 1 dig below, 2 faster, 3 while falling, 4 plunge.
    pub knee_drill: u8,
    /// Grapple range in cells (0 = none).
    pub grapple: f32,
    pub grapple_pull: bool,
    pub grapple_swing: bool,

    // ---- vitals ----
    pub max_hp: f32,
    /// Multiplier on damage taken.
    pub damage_taken: f32,
    pub still_immune: bool,
    pub o2_capacity: f32,
    /// Multiplier on suit oxygen drain.
    pub o2_drain: f32,
    pub o2_refill: f32,
    pub no_drain_daylight: bool,
    pub water_o2_free: bool,
    pub gills: bool,
    pub battery_capacity: f32,
    pub battery_regen: f32,
    /// The battery doesn't lock out when drained.
    pub no_lockout: bool,
    /// 0–1 fraction of cold ignored.
    pub cold_resist: f32,
    /// 0–1 fraction of spore gas damage ignored.
    pub toxin_resist: f32,
    /// Health regenerated per second anywhere.
    pub regen: f32,
    /// Damage a recharging shield absorbs.
    pub shield: f32,
    pub web_immune: bool,
    /// Damage dealt back to attackers.
    pub thorns: f32,
    /// 0 normal, 1 half range, 2 unseen when still, 3 ignored, 4 feared.
    pub stealth: u8,

    // ---- tools ----
    /// How far the player can dig and interact, in cells.
    pub reach: f32,
    /// How far the player can place things, in cells.
    pub place_reach: f32,
    pub site_survey: bool,
    pub dig_power: f32,
    pub dig_radius: i32,
    /// Multiplier on items from dug ore.
    pub ore_yield: f32,
    /// 0 none, 1 stone sometimes yields gold, 2 any ore.
    pub lucky_stone: u8,
    pub overcharge: bool,
    pub bolt_damage: f32,
    pub bolt_cost: f32,
    pub bolt_count: u32,
    pub bolt_size: f32,
    pub crit: f32,
    pub crit_mult: f32,
    pub homing: bool,
    /// 0 none, 1 bolts slow, 2 freeze water, 3 chilled take more, 4 aura.
    pub cryo: u8,
    /// Radius of the burst a bolt makes on impact.
    pub boom: i32,
    pub boom_fire: bool,
    /// Pickup radius in cells.
    pub magnet: f32,
    pub magnet_walls: bool,
    pub dome_quick_stack: bool,
    pub extra_slots: u32,
    pub stack_mult: f32,

    // ---- economy and building ----
    pub sell_bonus: f32,
    pub remote_terminal: bool,
    pub remote_synth: bool,
    pub remote_stockpile: bool,
    /// Fraction off recipe costs.
    pub craft_discount: f32,
    /// Chance a craft comes out doubled.
    pub craft_double: f32,
    /// Fraction off dome kit costs.
    pub dome_discount: f32,
    pub dome_dome_speed: f32,
    /// Production multiplier of domes this player places.
    pub dome_boost: f32,
    /// Pipe tiles laid per pipe item.
    pub pipe_per_item: u32,
    pub pack_contents: bool,
    pub machine_boost: f32,

    // ---- farming ----
    pub harvest_mult: f32,
    pub extra_seeds: bool,
    pub see_genes: bool,
    pub splice_bonus: u8,
    /// 0 none, 1 harvest a dome, 2 plant, 3 water and fertilize, 4 every dome in view.
    pub farm_hands: u8,
    /// 0 none; cooldown gets shorter, 3 matures at once.
    pub tickle: u8,
    /// 0 none, 1 grass, 2 replant, 3 harvest, 4 everything.
    pub root_walk: u8,
    /// Radius watered around the player.
    pub sprinkler: f32,
    /// Growth multiplier for plants near the player (1 = none).
    pub grow_aura: f32,

    // ---- helping teammates ----
    pub warm_aura: bool,
    pub heal_aura: f32,
    pub revive_aura: bool,
    pub team_regen: f32,
    /// 0 none, 1 half drain, 2 no drain, 3 refill, 4 counts as a dome.
    pub air_aura: u8,
    pub shield_aura: bool,
    /// 0 none, 1 clears gas, 2 and turns it into oxygen.
    pub gas_clear: u8,
    /// 0 none, 1 charges pylons, 2–4 powers things in a growing radius.
    pub reactor: u8,
    /// 0 none, 1 touch, 2 revive, 3 ranged, 4 whole team.
    pub heal_touch: u8,

    // ---- robots, drones and arms ----
    /// Speed multiplier for robots near the player.
    pub robot_speed: f32,
    pub robot_all: bool,
    /// Robots can be programmed from anywhere.
    pub remote_robots: bool,
    pub drones: u8,
    pub drone_mines: bool,
    /// 0 none, 1 slow, 2 fast, 3 three targets, 4 never misses.
    pub turret: u8,
    pub arms: u8,
    pub free_arm: bool,

    // ---- senses (applied by the player's own client) ----
    pub map_reveal: f32,
    pub map_ore: bool,
    pub map_all: bool,
    pub xray: f32,
    pub xray_creatures: bool,
    /// 0–1: how much of the darkness is lifted.
    pub night_vision: f32,
    pub see_health: bool,
    pub zoom: f32,
    pub scout_cam: bool,
    /// 0 none, 1 lamp, 2 bright, 3 whole screen.
    pub headlamp: u8,
    pub forecast: bool,
    pub rain_dance: bool,
    pub rain_boost: bool,
    pub drizzle: bool,
    pub loot: f32,
    pub rare_loot: bool,
    /// 0 none, 1 ping, 2 recall, 3 rally point, 4 teleport anywhere.
    pub beacon: u8,
}

impl Default for ModStats {
    fn default() -> ModStats {
        ModStats {
            speed: 1.0,
            jump: 1.0,
            air_jumps: 0,
            thrust_secs: 0.0,
            wall: 0,
            charge_jump: false,
            head_stomp: false,
            gravity: 1.0,
            water_run: false,
            swim: 1.0,
            stomp: 0,
            dashes: 0,
            dash_phase: false,
            dash_walls: false,
            glide: 0,
            knee_drill: 0,
            grapple: 0.0,
            grapple_pull: false,
            grapple_swing: false,
            max_hp: 100.0,
            damage_taken: 1.0,
            still_immune: false,
            o2_capacity: 100.0,
            o2_drain: 1.0,
            o2_refill: 1.0,
            no_drain_daylight: false,
            water_o2_free: false,
            gills: false,
            battery_capacity: 100.0,
            battery_regen: 1.0,
            no_lockout: false,
            cold_resist: 0.0,
            toxin_resist: 0.0,
            regen: 0.0,
            shield: 0.0,
            web_immune: false,
            thorns: 0.0,
            stealth: 0,
            reach: 64.0,
            place_reach: 64.0,
            site_survey: false,
            dig_power: 1.0,
            dig_radius: 4,
            ore_yield: 1.0,
            lucky_stone: 0,
            overcharge: false,
            bolt_damage: 1.0,
            bolt_cost: 1.0,
            bolt_count: 1,
            bolt_size: 1.0,
            crit: 0.0,
            crit_mult: 2.0,
            homing: false,
            cryo: 0,
            boom: 0,
            boom_fire: false,
            magnet: 30.0,
            magnet_walls: false,
            dome_quick_stack: false,
            extra_slots: 0,
            stack_mult: 1.0,
            sell_bonus: 0.0,
            remote_terminal: false,
            remote_synth: false,
            remote_stockpile: false,
            craft_discount: 0.0,
            craft_double: 0.0,
            dome_discount: 0.0,
            dome_dome_speed: 1.0,
            dome_boost: 1.0,
            pipe_per_item: 1,
            pack_contents: false,
            machine_boost: 1.0,
            harvest_mult: 1.0,
            extra_seeds: false,
            see_genes: false,
            splice_bonus: 0,
            farm_hands: 0,
            tickle: 0,
            root_walk: 0,
            sprinkler: 0.0,
            grow_aura: 1.0,
            warm_aura: false,
            heal_aura: 0.0,
            revive_aura: false,
            team_regen: 0.0,
            air_aura: 0,
            shield_aura: false,
            gas_clear: 0,
            reactor: 0,
            heal_touch: 0,
            robot_speed: 1.0,
            robot_all: false,
            remote_robots: false,
            drones: 0,
            drone_mines: false,
            turret: 0,
            arms: 0,
            free_arm: false,
            map_reveal: 1.0,
            map_ore: false,
            map_all: false,
            xray: 0.0,
            xray_creatures: false,
            night_vision: 0.0,
            see_health: false,
            zoom: 1.0,
            scout_cam: false,
            headlamp: 0,
            forecast: false,
            rain_dance: false,
            rain_boost: false,
            drizzle: false,
            loot: 1.0,
            rare_loot: false,
            beacon: 0,
        }
    }
}

/// Picks the value for a tier from a list of per-tier values.
fn by<T: Copy>(tier: u8, values: [T; 4]) -> T {
    values[(tier.clamp(1, 4) - 1) as usize]
}

/// Applies one equipped set at a tier (1–4) to the stats.
fn apply(s: &mut ModStats, id: u8, t: u8) {
    use set::*;
    match id {
        ROCKET_FEET => {
            s.air_jumps = 1;
            s.thrust_secs = by(t, [0.0, 2.0, 5.0, f32::INFINITY]);
        }
        STICKY_SOLES => s.wall = t,
        SPRING_HEELS => {
            s.jump *= by(t, [1.14, 1.265, 1.265, 1.265]);
            s.head_stomp = t >= 2;
            s.charge_jump = t >= 3;
            if t >= 4 {
                s.gravity *= 0.5;
            }
        }
        HUSTLE_TREADS => {
            s.speed *= by(t, [1.2, 1.4, 1.6, 2.0]);
            s.water_run = t >= 4;
        }
        AQUA_FLIPPERS => {
            s.swim *= by(t, [1.5, 2.0, 3.0, 3.0]);
            s.water_o2_free = t >= 3;
            s.gills = t >= 4;
        }
        STOMPERS => s.stomp = t,
        DASH_PISTONS => {
            s.dashes = by(t, [1, 2, 2, 2]);
            s.dash_phase = t >= 3;
            s.dash_walls = t >= 4;
        }
        CARGO_PANTS => {
            s.extra_slots += by(t, [10, 20, 30, 30]);
            s.remote_stockpile |= t >= 4;
        }
        THERMAL_LEGGINGS => {
            s.cold_resist = by(t, [0.5, 1.0, 1.0, 1.0]);
            s.warm_aura = t >= 3;
            if t >= 4 {
                s.grow_aura = s.grow_aura.max(1.5);
            }
        }
        ROOT_WALKERS => s.root_walk = t,
        SHOCK_ABSORBERS => {
            s.damage_taken *= by(t, [0.85, 0.7, 0.55, 0.4]);
            s.web_immune |= t >= 2;
            s.still_immune = t >= 4;
        }
        JACKHAMMER_KNEES => s.knee_drill = t,
        O2_BACKPACK => {
            s.o2_capacity *= by(t, [1.5, 2.0, 3.0, 3.0]);
            if t >= 3 {
                s.o2_refill *= 2.0;
            }
            s.no_drain_daylight = t >= 4;
        }
        GLIDE_WINGS => s.glide = t,
        SHOULDER_TURRET => s.turret = t,
        BATTERY_PACK => {
            s.battery_capacity *= by(t, [1.5, 2.0, 3.0, 3.0]);
            s.no_lockout = t >= 3;
            if t >= 4 {
                s.battery_regen *= 3.0;
            }
        }
        SPRINKLER_PACK => {
            s.sprinkler = by(t, [22.0, 38.0, 54.0, 70.0]);
            s.grow_aura = s.grow_aura.max(by(t, [1.0, 1.0, 1.25, 2.0]));
        }
        PACK_MULE => {
            s.magnet *= by(t, [2.0, 3.0, 5.0, 14.0]);
            s.dome_quick_stack = t >= 3;
            s.magnet_walls = t >= 4;
        }
        DRONE_DOCK => {
            s.drones = by(t, [1, 1, 2, 4]);
            s.drone_mines = t >= 2;
        }
        BEACON_RACK => s.beacon = t,
        PLATING => {
            s.max_hp += by(t, [25.0, 50.0, 100.0, 200.0]);
            if t >= 4 {
                s.thorns = 12.0;
            }
        }
        FILTER_LUNGS => {
            s.toxin_resist = by(t, [0.5, 1.0, 1.0, 1.0]);
            s.gas_clear = by(t, [0, 0, 1, 2]);
        }
        MEDI_CORE => {
            s.regen += 1.0;
            s.heal_aura = by(t, [0.0, 3.0, 3.0, 3.0]);
            s.revive_aura = t >= 3;
            s.team_regen = by(t, [0.0, 0.0, 0.0, 1.5]);
        }
        REACTOR_HEART => s.reactor = t,
        CAMO_SKIN => s.stealth = t,
        SYNTH_BELLY => {
            s.remote_synth = true;
            s.craft_discount = by(t, [0.0, 0.15, 0.3, 0.3]);
            s.remote_stockpile |= t >= 3;
            s.craft_double = by(t, [0.0, 0.0, 0.0, 0.5]);
        }
        BUDDY_BREATHER => s.air_aura = t,
        ANTENNA_ARRAY => {
            s.map_reveal = by(t, [1.5, 2.0, 2.0, 2.0]);
            s.map_ore = t >= 3;
            s.map_all = t >= 4;
        }
        TRADE_CHIP => {
            s.sell_bonus = by(t, [0.10, 0.20, 0.35, 0.75]);
            s.remote_terminal = t >= 3;
        }
        HIVE_MIND => {
            s.robot_speed = by(t, [1.25, 1.5, 2.0, 2.0]);
            s.remote_robots = t >= 2;
            s.robot_all = t >= 4;
        }
        BOTANISTS_BONNET => {
            s.harvest_mult = by(t, [1.25, 1.5, 2.0, 2.0]);
            s.see_genes = t >= 2;
            s.extra_seeds = t >= 3;
            s.splice_bonus = by(t, [0, 0, 0, 1]);
        }
        HEADLAMP => {
            s.headlamp = by(t, [1, 2, 3, 3]);
            if t >= 4 {
                s.grow_aura = s.grow_aura.max(1.5);
            }
        }
        DOME_BRAIN => {
            s.dome_discount = by(t, [0.1, 0.2, 0.3, 0.3]);
            s.dome_dome_speed = by(t, [1.0, 1.0, 2.0, 2.0]);
            s.dome_boost = by(t, [1.0, 1.0, 1.0, 1.25]);
        }
        WEATHER_VANE => {
            s.forecast = true;
            s.rain_dance = t >= 2;
            s.rain_boost = t >= 3;
            s.drizzle = t >= 4;
        }
        XRAY_SPECS => {
            s.xray = by(t, [40.0, 70.0, 100.0, 400.0]);
            s.xray_creatures = t >= 2;
        }
        NIGHT_VISION => s.night_vision = by(t, [0.25, 0.5, 0.75, 1.0]),
        THREAT_LENS => {
            s.crit = by(t, [0.1, 0.2, 0.3, 0.3]);
            s.crit_mult = by(t, [2.0, 2.0, 3.0, 3.0]);
            s.see_health = t >= 2;
            s.homing |= t >= 4;
        }
        APPRAISAL_MONOCLE => {
            s.loot = by(t, [1.25, 1.5, 1.5, 2.0]);
            s.rare_loot = t >= 3;
        }
        ZOOM_GOGGLES => {
            s.zoom = by(t, [1.15, 1.3, 1.5, 1.5]);
            s.scout_cam = t >= 4;
        }
        GEO_VISOR => {
            s.place_reach *= by(t, [1.5, 1.5, 3.0, 12.0]);
            s.site_survey = t >= 2;
        }
        EXTENDO_ARMS => {
            let more = by(t, [1.4, 1.4, 1.8, 1.8]);
            s.reach *= more;
            s.place_reach *= more;
            s.arms = by(t, [0, 1, 2, 2]);
            s.free_arm = t >= 4;
        }
        DRILL_ARMS => {
            s.dig_power *= by(t, [1.3, 1.6, 2.0, 7.0]);
            s.dig_radius += by(t, [0, 1, 2, 3]);
        }
        POWER_LIFTERS => s.stack_mult *= by(t, [1.5, 2.0, 3.0, 10.0]),
        SHIELD_ARM => {
            s.shield = by(t, [20.0, 40.0, 70.0, 70.0]);
            s.web_immune |= t >= 3;
            s.shield_aura = t >= 4;
        }
        FARM_HANDS => s.farm_hands = t,
        WELDER_ARMS => {
            s.pipe_per_item = by(t, [2, 4, 4, 4]);
            s.pack_contents = t >= 3;
            s.machine_boost = by(t, [1.0, 1.0, 1.0, 1.25]);
        }
        BOLT_ENHANCEMENTS => {
            s.overcharge = true;
            if t >= 2 {
                s.bolt_damage *= 1.5;
                s.bolt_size *= 1.4;
            }
            if t >= 3 {
                s.bolt_count *= 2;
            }
            if t >= 4 {
                s.bolt_cost *= 0.2;
            }
        }
        MIDAS_MITTS => {
            s.ore_yield *= by(t, [1.15, 1.3, 1.5, 1.5]);
            s.lucky_stone = by(t, [0, 0, 1, 2]);
        }
        CRYO_PALMS => s.cryo = t,
        BOOM_MITTS => {
            s.boom = by(t, [3, 5, 7, 11]);
            s.boom_fire = t == 3;
        }
        GREEN_FINGERS => {
            s.tickle = by(t, [1, 2, 3, 3]);
            if t >= 4 {
                s.grow_aura = s.grow_aura.max(3.0);
            }
        }
        GRAPPLE_GLOVE => {
            s.grapple = by(t, [60.0, 110.0, 110.0, 1000.0]);
            s.grapple_pull = t >= 2;
            s.grapple_swing = t >= 3;
        }
        HEALING_HANDS => s.heal_touch = t,
        _ => {}
    }
}

/// The stats of a player with these sets equipped.
pub fn stats_for(equipped: &[Option<u8>; 8], owned: &BTreeMap<u8, u8>) -> ModStats {
    let mut s = ModStats::default();
    for id in equipped.iter().flatten() {
        if let Some(&tier) = owned.get(id) {
            apply(&mut s, *id, tier);
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn there_are_53_sets_with_four_named_tiers_across_eight_slots() {
        assert_eq!(MOD_SETS.len(), 53);
        let mut names = std::collections::BTreeSet::new();
        for s in &MOD_SETS {
            assert!(names.insert(s.name), "duplicate set {}", s.name);
            for (name, text) in s.tiers {
                assert!(!name.is_empty() && !text.is_empty());
                assert!(names.insert(name), "duplicate tier name {name}");
            }
        }
        for slot in Slot::ALL {
            let n = MOD_SETS.iter().filter(|s| s.slot == slot).count();
            assert!((6..=8).contains(&n), "{slot:?} has {n} sets");
        }
        assert_eq!(MOD_SETS[set::ROCKET_FEET as usize].name, "Rocket Feet");
        assert_eq!(MOD_SETS[set::EXTENDO_ARMS as usize].name, "Extendo-Arms");
        assert_eq!(
            MOD_SETS[set::BOLT_ENHANCEMENTS as usize].name,
            "Bolt Enhancements"
        );
        assert_eq!(MOD_SETS[set::HEALING_HANDS as usize].name, "Healing Hands");
    }

    #[test]
    fn every_tier_of_every_set_changes_something() {
        let base = ModStats::default();
        for id in 0..SETS as u8 {
            let mut prev = base;
            for tier in 1..=4 {
                let mut s = base;
                apply(&mut s, id, tier);
                assert_ne!(
                    s, prev,
                    "{} tier {tier} changes nothing",
                    MOD_SETS[id as usize].name
                );
                prev = s;
            }
        }
    }

    #[test]
    fn only_equipped_sets_count_and_they_combine() {
        let mut owned = BTreeMap::new();
        owned.insert(set::ROCKET_FEET, 2);
        owned.insert(set::BOLT_ENHANCEMENTS, 4);
        owned.insert(set::PLATING, 3);
        let mut equipped = [None; 8];
        assert_eq!(stats_for(&equipped, &owned), ModStats::default());
        equipped[Slot::Feet as usize] = Some(set::ROCKET_FEET);
        equipped[Slot::Hands as usize] = Some(set::BOLT_ENHANCEMENTS);
        let s = stats_for(&equipped, &owned);
        assert_eq!((s.air_jumps, s.thrust_secs), (1, 2.0));
        assert_eq!(s.bolt_count, 2);
        assert!((s.bolt_cost - 0.2).abs() < 1e-6 && s.overcharge);
        assert_eq!(s.max_hp, 100.0, "plating is owned but not equipped");
    }

    #[test]
    fn example_sets_match_the_brief() {
        let stats = |id, tier| {
            let mut s = ModStats::default();
            apply(&mut s, id, tier);
            s
        };
        // Rocket Feet: double jump, 2 s hover, 5 s flight, infinite flight.
        assert_eq!(stats(set::ROCKET_FEET, 1).thrust_secs, 0.0);
        assert_eq!(stats(set::ROCKET_FEET, 3).thrust_secs, 5.0);
        assert!(stats(set::ROCKET_FEET, 4).thrust_secs.is_infinite());
        // Extendo-Arms: reach, one arm, two arms and more reach, a free arm.
        assert!(stats(set::EXTENDO_ARMS, 1).reach > 64.0);
        assert_eq!(stats(set::EXTENDO_ARMS, 2).arms, 1);
        assert_eq!(stats(set::EXTENDO_ARMS, 3).arms, 2);
        assert!(stats(set::EXTENDO_ARMS, 3).reach > stats(set::EXTENDO_ARMS, 2).reach);
        assert!(stats(set::EXTENDO_ARMS, 4).free_arm);
        // Blueprints: starter sets are free, the rest cost credits.
        assert_eq!(blueprint_price(set::ROCKET_FEET), 0);
        assert!((0..SETS as u8).filter(|s| blueprint_price(*s) > 0).count() == 47);
    }
}
