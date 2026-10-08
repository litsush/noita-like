//! Per-individual state: needs, status effects, memory and animation.

use super::brain::{Action, BehaviorKind};
use super::genome::{LimbKind, Species};
use super::math::{Rgb, V2, v2};
use super::physics::MoveCmd;

#[derive(Clone, Copy, Debug, Default)]
pub struct Status {
    pub poison: f32,
    pub burning: f32,
    /// Ate something toxic.
    pub sick: f32,
    pub stun: f32,
    /// Held by a constrictor.
    pub grabbed_by: Option<u32>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Foot {
    pub pos: V2,
    pub from: V2,
    pub to: V2,
    /// Step progress; >= 1 means planted.
    pub t: f32,
    pub grounded: bool,
}

#[derive(Clone, Debug, Default)]
pub struct Anim {
    pub t: f32,
    /// Advances with distance travelled; drives gaits.
    pub walk: f32,
    pub flap: f32,
    /// Body tilt.
    pub angle: f32,
    pub segs: Vec<V2>,
    pub feet: Vec<Foot>,
    pub chains: Vec<Vec<V2>>,
    pub attack: f32,
    pub eat: f32,
    pub hurt: f32,
    pub blink: f32,
    /// Which surface a clinging creature is stuck to (unit vector).
    pub surface: V2,
    /// A limb instance to aim at a world point (striking with an arm or tail).
    pub reach: Option<(usize, V2)>,
}

/// What a creature has noticed this think.
#[derive(Clone, Debug, Default)]
pub struct Perception {
    pub threat: Option<(u32, V2, f32)>,
    pub threats: u8,
    pub prey: Option<(u32, V2, f32)>,
    pub kin_centre: Option<V2>,
    pub kin_count: u8,
    pub mate: Option<(u32, V2)>,
    pub plant: Option<(u32, V2)>,
    pub carcass: Option<(u32, V2)>,
    pub egg: Option<(u32, V2)>,
    /// A kin being attacked, and by whom.
    pub kin_attacker: Option<(u32, V2)>,
    pub intruder: Option<(u32, V2)>,
    pub alarm: Option<V2>,
    pub lure: Option<(u32, V2)>,
    pub webbed_prey: Option<(u32, V2)>,
    pub noise: Option<V2>,
    pub light: f32,
}

#[derive(Clone, Debug)]
pub struct Brain {
    pub current: BehaviorKind,
    pub scores: Vec<(BehaviorKind, f32)>,
    pub think: f32,
    pub target: Option<u32>,
    pub spot: Option<V2>,
    pub stage: u8,
    pub timer: f32,
    pub path: Vec<V2>,
    pub path_goal: V2,
    pub repath: f32,
    pub stuck: f32,
    pub last_pos: V2,
    pub stuck_check: f32,
    pub home: Option<V2>,
    /// Shelter wall cells still to place (or to repair).
    pub nest_plan: Vec<(i32, i32)>,
    pub nest_cells: Vec<(i32, i32)>,
    pub trap: Option<V2>,
    pub trap_cells: Vec<(i32, i32)>,
    /// Targets that turned out unreachable: (key, forget at time).
    pub ignore: Vec<(u64, f32)>,
    pub aversion: f32,
    /// Where prey was last seen, and when.
    pub last_prey: Option<(V2, f32)>,
    pub label: &'static str,
    pub perception: Perception,
    pub action: Action,
}

impl Brain {
    pub fn new() -> Brain {
        Brain {
            current: BehaviorKind::Wander,
            scores: Vec::new(),
            think: 0.0,
            target: None,
            spot: None,
            stage: 0,
            timer: 0.0,
            path: Vec::new(),
            path_goal: V2::ZERO,
            repath: 0.0,
            stuck: 0.0,
            last_pos: V2::ZERO,
            stuck_check: 0.0,
            home: None,
            nest_plan: Vec::new(),
            nest_cells: Vec::new(),
            trap: None,
            trap_cells: Vec::new(),
            ignore: Vec::new(),
            aversion: 0.0,
            last_prey: None,
            label: "",
            perception: Perception::default(),
            action: Action::None,
        }
    }

    pub fn ignored(&self, key: u64) -> bool {
        self.ignore.iter().any(|(k, _)| *k == key)
    }
}

impl Default for Brain {
    fn default() -> Self {
        Brain::new()
    }
}

/// Keys for the ignore list.
pub fn key_creature(id: u32) -> u64 {
    id as u64
}
pub fn key_plant(id: u32) -> u64 {
    (1u64 << 32) | id as u64
}
pub fn key_carcass(id: u32) -> u64 {
    (2u64 << 32) | id as u64
}

#[derive(Clone, Debug)]
pub struct Creature {
    pub id: u32,
    pub species: usize,
    pub pos: V2,
    pub vel: V2,
    pub facing: f32,
    pub age: f32,
    pub health: f32,
    pub satiety: f32,
    pub energy: f32,
    pub fear: f32,
    pub breath: f32,
    pub generation: u32,
    pub on_ground: bool,
    pub in_liquid: bool,
    pub clinging: bool,
    pub flying: bool,
    pub burrowed: bool,
    pub webbed: f32,
    pub sheltered: bool,
    pub sleeping: bool,
    pub playing_dead: f32,
    pub still: f32,
    pub luring: bool,
    pub status: Status,
    pub attack_cd: f32,
    pub spit_cd: f32,
    pub leap_cd: f32,
    pub ink_cd: f32,
    pub pregnant: Option<f32>,
    pub mate_cd: f32,
    pub carry_dirt: u16,
    /// Soil displaced by sand-swimming, put back once passed.
    pub displaced: Vec<(i32, i32, u8)>,
    pub brain: Brain,
    pub anim: Anim,
    pub dead: Option<&'static str>,
    pub last_hit_by: Option<u32>,
    pub hurt_timer: f32,
    /// How much noise it is making (0 silent, 1 loud).
    pub noise: f32,
    pub kills: u32,
    /// Last movement command, for the inspector.
    pub last_cmd: MoveCmd,
    /// Per limb instance: false once the limb has been torn off. Empty
    /// means every limb is intact.
    pub limb_ok: Vec<bool>,
}

impl Creature {
    pub fn new(id: u32, species: &Species, pos: V2, adult: bool, generation: u32) -> Creature {
        let age = if adult { species.stats.maturity * 1.2 } else { 0.0 };
        let mut c = Creature {
            id,
            species: species.idx,
            pos,
            vel: V2::ZERO,
            facing: if id.is_multiple_of(2) { 1.0 } else { -1.0 },
            age,
            health: 0.0,
            satiety: 0.75,
            energy: 0.9,
            fear: 0.0,
            breath: 1.0,
            generation,
            on_ground: false,
            in_liquid: false,
            clinging: false,
            flying: false,
            burrowed: false,
            webbed: 0.0,
            sheltered: false,
            sleeping: false,
            playing_dead: 0.0,
            still: 0.0,
            luring: false,
            status: Status::default(),
            attack_cd: 0.0,
            spit_cd: 0.0,
            leap_cd: 0.0,
            ink_cd: 0.0,
            pregnant: None,
            mate_cd: 20.0,
            carry_dirt: 0,
            displaced: Vec::new(),
            brain: Brain::new(),
            anim: Anim::default(),
            dead: None,
            last_hit_by: None,
            hurt_timer: 0.0,
            noise: 0.0,
            kills: 0,
            last_cmd: MoveCmd::default(),
            limb_ok: Vec::new(),
        };
        c.health = c.max_health(species);
        c.brain.last_pos = pos;
        c.brain.think = (id % 7) as f32 * 0.03;
        init_anim(&mut c, species);
        c
    }

    /// Juveniles are drawn and collide smaller.
    pub fn scale(&self, sp: &Species) -> f32 {
        0.55 + 0.45 * (self.age / sp.stats.maturity).min(1.0)
    }

    pub fn mature(&self, sp: &Species) -> bool {
        self.age >= sp.stats.maturity
    }

    pub fn max_health(&self, sp: &Species) -> f32 {
        sp.stats.max_health * self.scale(sp)
    }

    pub fn alive(&self) -> bool {
        self.dead.is_none()
    }

    /// Whether limb instance `k` (see [`limb_instances`]) is still attached.
    #[inline]
    pub fn has_limb(&self, k: usize) -> bool {
        self.limb_ok.get(k).copied().unwrap_or(true)
    }
}

/// Number of drawn instances of each limb gene (paired limbs twice).
pub fn limb_instances(sp: &Species) -> Vec<(usize, bool)> {
    let mut v = Vec::new();
    for (i, l) in sp.body.limbs.iter().enumerate() {
        v.push((i, false));
        if l.paired {
            v.push((i, true));
        }
    }
    v
}

pub fn init_anim(c: &mut Creature, sp: &Species) {
    let s = c.scale(sp);
    let b = &sp.body;
    c.anim.segs = (0..b.segs.len())
        .map(|i| c.pos - v2(c.facing * b.spacing * s * i as f32, 0.0))
        .collect();
    c.anim.feet.clear();
    c.anim.chains.clear();
    for (li, _far) in limb_instances(sp) {
        let l = &b.limbs[li];
        if l.kind == LimbKind::Leg {
            let p = c.pos + v2(0.0, b.stance * s);
            c.anim.feet.push(Foot {
                pos: p,
                from: p,
                to: p,
                t: 1.0,
                grounded: false,
            });
            c.anim.chains.push(Vec::new());
        } else {
            c.anim.feet.push(Foot::default());
            let n = l.joints.max(2) as usize + 1;
            c.anim.chains.push(vec![c.pos; n]);
        }
    }
    c.anim.surface = v2(0.0, 1.0);
}

#[derive(Clone, Debug)]
pub struct Carcass {
    pub id: u32,
    pub species: usize,
    pub pos: V2,
    pub vel: V2,
    pub meat: f32,
    pub max_meat: f32,
    pub age: f32,
    pub scale: f32,
    pub facing: f32,
    pub toxic: bool,
}

#[derive(Clone, Debug)]
pub struct Egg {
    pub id: u32,
    pub species: usize,
    pub pos: V2,
    pub vel: V2,
    pub timer: f32,
    pub generation: u32,
    pub color: Rgb,
}
