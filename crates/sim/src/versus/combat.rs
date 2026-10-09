//! The fighting mind. Each individual sees what its senses allow, picks a
//! tactic, and keeps score of how every tactic has gone for it so far. The
//! smarter the species, the more that record steers its choices, and the
//! more it notices things like "biting that shell does nothing".

use crate::eco::brain::Intent;
use crate::eco::creature::Creature;
use crate::eco::math::{V2, v2};
use crate::eco::nav::line_of_sight;
use crate::material::Material;
use crate::rng::Rng;
use crate::world::World;

use super::design::{Loadout, WeaponKind};
use super::parts::Effects;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tactic {
    Rush,
    Flank,
    Kite,
    Harass,
    Ambush,
    Dive,
    Undermine,
    Hold,
    Retreat,
    Search,
    Forage,
    /// Keep away and out of sight: how the weaponless fight.
    Evade,
    /// Sit tight somewhere quiet until something turns up.
    Hide,
}

impl Tactic {
    pub const ALL: [Tactic; 13] = [
        Tactic::Rush,
        Tactic::Flank,
        Tactic::Kite,
        Tactic::Harass,
        Tactic::Ambush,
        Tactic::Dive,
        Tactic::Undermine,
        Tactic::Hold,
        Tactic::Retreat,
        Tactic::Search,
        Tactic::Forage,
        Tactic::Evade,
        Tactic::Hide,
    ];

    pub fn index(self) -> usize {
        Self::ALL.iter().position(|&t| t == self).unwrap()
    }

    pub fn from_index(i: usize) -> Tactic {
        Self::ALL[i.min(Self::ALL.len() - 1)]
    }

    pub fn label(self) -> &'static str {
        match self {
            Tactic::Rush => "charging in",
            Tactic::Flank => "flanking",
            Tactic::Kite => "keeping its distance",
            Tactic::Harass => "hit and run",
            Tactic::Ambush => "lying in wait",
            Tactic::Dive => "diving from above",
            Tactic::Undermine => "tunnelling underneath",
            Tactic::Hold => "holding ground",
            Tactic::Retreat => "retreating",
            Tactic::Search => "searching",
            Tactic::Forage => "foraging",
            Tactic::Evade => "evading",
            Tactic::Hide => "hiding",
        }
    }
}

/// Which part of an enemy to aim at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Focus {
    Any,
    Head,
    Limbs,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct TacticStat {
    pub uses: f32,
    pub dealt: f32,
    pub taken: f32,
}

/// What the brain wants done this tick besides moving.
#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum CombatAction {
    #[default]
    None,
    Strike(WeaponKind, u32),
    Spit(V2),
    Leap(V2),
    Ink,
    /// Eat the food source with this id (a plant, or a fallen fighter).
    Eat(u32),
}

/// Something to eat, as the arena lists it for hungry brains.
#[derive(Clone, Copy, Debug)]
pub struct Food {
    pub id: u32,
    pub pos: V2,
    /// Energy on offer.
    pub amount: f32,
    pub meat: bool,
}

#[derive(Clone, Debug)]
pub struct CombatBrain {
    pub tactic: Tactic,
    pub since: f32,
    pub think: f32,
    pub stage: u8,
    pub stage_t: f32,
    pub target: Option<u32>,
    pub last_seen: Option<(V2, f32)>,
    pub stats: Vec<TacticStat>,
    pub focus: Focus,
    /// Damage dealt and taken since the current tactic began.
    pub window_dealt: f32,
    pub window_taken: f32,
    /// Hits on armour that did little, and hits that went through.
    pub blocked: f32,
    pub landed: f32,
    /// Damage taken from ranged attacks, decaying.
    pub ranged_pain: f32,
    pub retreat_until: f32,
    /// Somewhere else to go for a moment when the direct way is blocked.
    pub detour: Option<(V2, f32)>,
    pub label: &'static str,
    pub action: CombatAction,
    /// Spotted the target before it spotted us: the first strike lands as
    /// an ambush. Set when the target is acquired, spent on the first blow.
    pub surprise: bool,
    pub surprise_until: f32,
    /// When the target was last in view, for drifting the search.
    pub lost_at: f32,
    /// The food we're heading for.
    pub food: Option<Food>,
}

impl CombatBrain {
    pub fn new(id: u32) -> CombatBrain {
        CombatBrain {
            tactic: Tactic::Search,
            since: 0.0,
            think: (id % 5) as f32 * 0.07,
            stage: 0,
            stage_t: 0.0,
            target: None,
            last_seen: None,
            stats: vec![TacticStat::default(); Tactic::ALL.len()],
            focus: Focus::Any,
            window_dealt: 0.0,
            window_taken: 0.0,
            blocked: 0.0,
            landed: 0.0,
            ranged_pain: 0.0,
            retreat_until: 0.0,
            detour: None,
            label: "",
            action: CombatAction::None,
            surprise: false,
            surprise_until: 0.0,
            lost_at: 0.0,
            food: None,
        }
    }

    pub fn dealt(&mut self, amount: f32, blocked_fraction: f32) {
        self.window_dealt += amount;
        if blocked_fraction > 0.35 {
            self.blocked += 1.0;
        } else {
            self.landed += 1.0;
        }
    }

    pub fn taken(&mut self, amount: f32, ranged: bool) {
        self.window_taken += amount;
        if ranged {
            self.ranged_pain += amount;
        }
    }

    fn set_tactic(&mut self, t: Tactic, now: f32) {
        if t != self.tactic {
            // Bank the record of the tactic we're leaving.
            let st = &mut self.stats[self.tactic.index()];
            st.uses += 1.0;
            st.dealt += self.window_dealt;
            st.taken += self.window_taken;
            self.window_dealt = 0.0;
            self.window_taken = 0.0;
            self.tactic = t;
            self.since = now;
            self.stage = 0;
            self.stage_t = 0.0;
        }
    }

    /// How well a tactic has gone, with a prior so untried ones look fine.
    fn learned(&self, t: Tactic, prior: f32) -> f32 {
        let st = &self.stats[t.index()];
        let (d, k) = if t == self.tactic {
            (st.dealt + self.window_dealt, st.taken + self.window_taken)
        } else {
            (st.dealt, st.taken)
        };
        ((d + prior) / (k + prior)).clamp(0.2, 4.0)
    }
}

/// What a fighter can see of another.
#[derive(Clone, Debug)]
pub struct EnemySnap {
    pub id: u32,
    pub team: usize,
    pub pos: V2,
    pub vel: V2,
    pub facing: f32,
    pub size: f32,
    pub health: f32,
    pub on_ground: bool,
    pub flying: bool,
    pub burrowed: bool,
    pub still: f32,
    pub camouflage: f32,
    pub armor: f32,
    pub ranged: bool,
    pub speed: f32,
    pub targeting: Option<u32>,
    pub stunned: bool,
    /// Best weak spot to aim at, if the brain is clever enough to use it.
    pub weak_spot: Option<V2>,
    pub head: V2,
    /// Who it has in view right now, if anyone.
    pub aware: Option<u32>,
    pub energy: f32,
    /// Every intact part: centre, kind (0 head, 1 body, 2 limb), health
    /// fraction, index into the species' part layout.
    pub parts: Vec<(V2, u8, f32, u8)>,
}

/// The thinker's own numbers that aren't on the creature.
#[derive(Clone, Copy, Debug)]
pub struct Me {
    pub max_health: f32,
    pub speed: f32,
    pub size: f32,
}

pub struct Ctx<'a> {
    pub world: &'a World,
    pub time: f32,
    pub light: f32,
    pub frenzy: bool,
    pub gravity: f32,
    /// Where the enemy teams started, to search towards.
    pub enemy_side: V2,
    pub centre: V2,
    /// Where the nearest living enemy actually is. A long search drifts
    /// towards it, so two animals never spend a whole round apart.
    pub enemy_hint: Option<V2>,
    pub food: &'a [Food],
}

fn away_from(c: &Creature, from: V2, dist: f32) -> V2 {
    let mut d = (c.pos - from).norm();
    if d.len() < 0.1 {
        d = v2(c.facing, 0.0);
    }
    c.pos + d * dist
}

fn intent(goal: Option<V2>, speed: f32) -> Intent {
    Intent {
        goal,
        speed,
        ..Default::default()
    }
}

/// Whether `me` notices `e` right now, and how clearly (0..1).
pub fn detect(
    me: &Creature,
    lo: &Loadout,
    eff: &Effects,
    e: &EnemySnap,
    ctx: &Ctx,
    frenzy: bool,
) -> Option<f32> {
    let d = e.pos.dist(me.pos);
    if frenzy && d < 220.0 {
        return Some(1.0);
    }
    if lo.echo > 0.0 && eff.ears_ok && d < lo.echo {
        return Some(1.0);
    }
    if lo.tremor && eff.feelers_ok && (e.on_ground || e.burrowed) && e.vel.len() > 3.0 && d < 75.0 {
        return Some(0.8);
    }
    // A nose finds them through walls and camouflage, roughly.
    if lo.nose > 0.0 && d < lo.nose {
        return Some(0.6);
    }
    if e.burrowed {
        return None;
    }
    let light = ctx.light + (1.0 - ctx.light) * lo.night;
    let sight = lo.sight * eff.sight * (0.3 + 0.7 * light);
    let stillness = (e.still / 1.5).clamp(0.0, 1.0);
    let camo = e.camouflage * (0.45 + 0.55 * stillness);
    let conspicuous = (e.size / 4.0).sqrt().clamp(0.6, 1.5) * (1.0 - camo) * (1.0 + e.vel.len() / 90.0);
    if d < sight * conspicuous && line_of_sight(ctx.world, me.pos, e.pos) {
        return Some(1.0);
    }
    if d < 4.0 + lo.size * 2.0 {
        // Touching range: nothing hides that close.
        return Some(0.6);
    }
    None
}

/// Picks a target and a tactic. Called every think interval.
#[allow(clippy::too_many_arguments)]
pub fn think(
    c: &mut Creature,
    brain: &mut CombatBrain,
    lo: &Loadout,
    me: &Me,
    eff: &Effects,
    enemies: &[EnemySnap],
    allies: &[EnemySnap],
    ctx: &Ctx,
    rng: &mut Rng,
) {
    let now = ctx.time;
    let frenzy = ctx.frenzy;
    let intel = lo.intelligence;
    // What can we see?
    let mut seen: Vec<(EnemySnap, f32)> = enemies
        .iter()
        .filter_map(|e| detect(c, lo, eff, e, ctx, frenzy).map(|q| (e.clone(), q)))
        .collect();
    // Allies share what they see.
    if lo.pack > 0.0 {
        for a in allies {
            if let Some(t) = a.targeting
                && !seen.iter().any(|(e, _)| e.id == t)
                && let Some(e) = enemies.iter().find(|e| e.id == t)
            {
                seen.push((e.clone(), 0.5));
            }
        }
    }
    let health = c.health / me.max_health.max(1.0);
    // Choose a target: close, hurt, and (packs) the same one as friends.
    let pack_target = allies.iter().filter_map(|a| a.targeting).next();
    let mut best: Option<(u32, f32)> = None;
    for (e, q) in &seen {
        let d = e.pos.dist(c.pos);
        let mut score = q * 100.0 / (d + 20.0);
        score *= 1.0 + (1.0 - e.health) * (0.3 + intel * 0.7);
        if e.stunned {
            score *= 1.3;
        }
        if lo.pack > 0.0 && pack_target == Some(e.id) {
            score *= 1.0 + lo.pack * 1.5;
        }
        if brain.target == Some(e.id) {
            score *= 1.4;
        }
        if best.is_none_or(|b| score > b.1) {
            best = Some((e.id, score));
        }
    }
    match best {
        Some((id, _)) => {
            let e = &seen.iter().find(|(e, _)| e.id == id).unwrap().0;
            // Caught them unawares: the first blow is an ambush. Sharp eyes
            // earn this far more often than dull ones.
            let fresh = brain.target != Some(id) || now - brain.lost_at > 3.0;
            if fresh && e.aware != Some(c.id) && !frenzy {
                brain.surprise = true;
                brain.surprise_until = now + 6.0;
            }
            brain.target = Some(id);
            brain.last_seen = Some((e.pos, now));
            brain.lost_at = now;
        }
        None => {
            if brain.last_seen.is_some_and(|(_, t)| now - t > 7.0) {
                brain.last_seen = None;
            }
            // Keep the id so memory makes sense, but we don't know where it is.
        }
    }
    if brain.surprise
        && (now > brain.surprise_until
            || seen
                .iter()
                .any(|(e, _)| Some(e.id) == brain.target && e.aware == Some(c.id)))
    {
        brain.surprise = false;
    }
    brain.ranged_pain *= 0.9;

    // Hunger. A grazer with nothing in sight eats whenever it can; anything
    // starving goes for food unless the enemy is on top of it.
    let visible_now = best.is_some();
    let nearest_enemy = seen
        .iter()
        .map(|(e, _)| e.pos.dist(c.pos))
        .fold(f32::MAX, f32::min);
    let grazer = !lo.diet.eats_meat();
    let hungry = c.energy < 0.45 || (grazer && c.energy < 0.8 && !visible_now);
    if hungry && !(frenzy && c.energy > 0.3) {
        let safe = !visible_now || nearest_enemy > 45.0 || c.energy < 0.15;
        let food = ctx
            .food
            .iter()
            .filter(|f| {
                if f.meat {
                    lo.diet.eats_meat()
                } else {
                    lo.diet.eats_fruit()
                }
            })
            .filter(|f| f.amount > 0.02)
            .filter(|f| {
                // Smelled, seen, or remembered.
                let d = f.pos.dist(c.pos);
                d < lo.nose.max(55.0 + lo.sight * 0.5) || brain.food.is_some_and(|k| k.id == f.id)
            })
            .min_by(|a, b| a.pos.dist(c.pos).total_cmp(&b.pos.dist(c.pos)))
            .copied();
        if safe && let Some(f) = food {
            brain.food = Some(f);
            brain.set_tactic(Tactic::Forage, now);
            brain.label = "foraging";
            return;
        }
    }
    if brain.tactic == Tactic::Forage {
        let done = c.energy > 0.9
            || brain
                .food
                .is_none_or(|f| !ctx.food.iter().any(|g| g.id == f.id && g.amount > 0.02));
        let threatened = visible_now && nearest_enemy < 35.0 && c.energy > 0.15;
        if !(done || threatened || frenzy && c.energy > 0.3) {
            return;
        }
        brain.food = None;
    }

    // Nothing to fight with: live by not being caught. Outlast them.
    let can_fight = !lo.weapons.is_empty() || lo.spit.is_some();
    if !can_fight {
        let still_running = brain.tactic == Tactic::Evade && now - brain.lost_at < 6.0;
        let t = if visible_now || still_running {
            Tactic::Evade
        } else {
            Tactic::Hide
        };
        brain.set_tactic(t, now);
        return;
    }

    // Learn where to hit.
    if intel >= 0.25 && brain.blocked >= 3.0 && brain.blocked > brain.landed * 0.8 {
        brain.focus = if intel >= 0.5 && rng.coin() {
            Focus::Head
        } else {
            Focus::Limbs
        };
    } else if intel >= 0.75 && brain.target.is_some() {
        // Clever fighters go for whatever is already damaged.
        brain.focus = Focus::Limbs;
    }

    let has_melee = !lo.weapons.is_empty();
    let visible = best.is_some();
    let target = best.and_then(|(id, _)| seen.iter().find(|(e, _)| e.id == id).map(|(e, _)| e.clone()));

    // Retreat is a need, not a choice.
    let threshold = 0.12 + lo.caution * 0.3 - lo.aggression * 0.08;
    if !frenzy
        && health < threshold
        && (lo.regen > 0.0 || lo.ink || lo.caution > 0.3)
        && now > brain.retreat_until
    {
        brain.set_tactic(Tactic::Retreat, now);
        brain.retreat_until = now + 9.0;
        brain.label = "retreating";
        return;
    }
    if brain.tactic == Tactic::Retreat {
        let recovered = health > threshold + 0.25 || now > brain.retreat_until || frenzy;
        if !recovered {
            return;
        }
    }
    if !visible {
        if brain.tactic != Tactic::Ambush || brain.since + 25.0 < now {
            brain.set_tactic(Tactic::Search, now);
        }
        return;
    }
    let Some(t) = target else { return };

    // Reconsider the tactic every few seconds, sooner when smart or when
    // the current one is going badly.
    let going_badly = brain.window_taken > brain.window_dealt * 1.5 + 6.0;
    let period = 4.0 - 2.2 * intel;
    let due = now - brain.since > period || brain.tactic == Tactic::Search || (going_badly && intel > 0.2);
    if !due {
        return;
    }
    let d = t.pos.dist(c.pos);
    let my_speed = me.speed * eff.speed;
    let prior = 8.0 - 6.0 * intel;
    let learn_w = 0.25 + 0.75 * intel;
    let noise = (1.0 - intel) * 0.45;
    let aggression = if frenzy { 1.0 } else { lo.aggression };
    let unseen_by_them = t.targeting != Some(c.id);
    let mut scores: Vec<(Tactic, f32)> = Vec::new();
    let mut add = |tac: Tactic, base: f32| {
        if base <= 0.0 {
            return;
        }
        let l = brain.learned(tac, prior);
        let s = base * (1.0 - learn_w + learn_w * l) * (1.0 + rng.range(-noise, noise));
        scores.push((tac, s));
    };
    if has_melee {
        add(Tactic::Rush, 0.6 + aggression * 0.6);
        let faster = lo.sight > 0.0 && (my_speed > t.speed * 1.1 || lo.leap > 0.0 || lo.fly);
        add(Tactic::Harass, if faster { 0.5 + lo.caution * 0.4 } else { 0.15 });
        let allies_alive = !allies.is_empty();
        add(
            Tactic::Flank,
            if allies_alive {
                0.5 + lo.pack * 0.8
            } else if my_speed > t.speed {
                0.35
            } else {
                0.1
            },
        );
        add(
            Tactic::Hold,
            0.25 + lo.armor * 1.2 + lo.spines * 0.8 - aggression * 0.3,
        );
        if lo.camouflage > 0.0 && unseen_by_them && !frenzy {
            add(Tactic::Ambush, 0.7 + lo.camouflage * 0.6);
        }
        if lo.fly && eff.can_fly {
            add(Tactic::Dive, 0.6 + aggression * 0.3);
        }
        if lo.dig && t.on_ground && !t.flying {
            add(Tactic::Undermine, 0.6 + lo.camouflage * 0.3);
        }
    }
    if lo.spit.is_some() {
        let ranged_edge = if t.ranged { 0.5 } else { 0.9 };
        add(
            Tactic::Kite,
            ranged_edge + lo.caution * 0.4 + if has_melee { 0.0 } else { 1.0 },
        );
    }
    if scores.is_empty() {
        scores.push((Tactic::Rush, 1.0));
    }
    // Things noticed: being shot at from range means close the gap or hide.
    if brain.ranged_pain > 8.0 && intel > 0.2 {
        for (tac, s) in scores.iter_mut() {
            match tac {
                Tactic::Rush | Tactic::Dive | Tactic::Undermine | Tactic::Flank => *s *= 1.0 + intel,
                Tactic::Hold | Tactic::Ambush => *s *= 0.4,
                _ => {}
            }
        }
    }
    // Hysteresis so it doesn't flip-flop.
    for (tac, s) in scores.iter_mut() {
        if *tac == brain.tactic {
            *s *= 1.15;
        }
    }
    scores.sort_by(|a, b| b.1.total_cmp(&a.1));
    let chosen = scores[0].0;
    let _ = d;
    brain.set_tactic(chosen, now);
}

/// Turns the current tactic into movement and an action for this tick.
#[allow(clippy::too_many_arguments)]
pub fn act(
    c: &mut Creature,
    brain: &mut CombatBrain,
    lo: &Loadout,
    me: &Me,
    eff: &Effects,
    target: Option<&EnemySnap>,
    allies: &[EnemySnap],
    cooldowns: &[f32],
    origins: &[(WeaponKind, V2)],
    spit_cd: f32,
    ctx: &Ctx,
    rng: &mut Rng,
) -> Intent {
    brain.action = CombatAction::None;
    brain.stage_t += super::DT;
    let s = me.size;
    let speed_mult = eff.speed;
    // Drowning, burning or dissolving comes before everything.
    if let Some(mut it) = escape_hazard(c, lo, eff, ctx) {
        brain.label = "escaping danger";
        it.speed *= speed_mult;
        return it;
    }
    let mut it = match brain.tactic {
        Tactic::Search => {
            brain.label = "searching";
            let goal = match brain.last_seen {
                Some((p, _)) if p.dist(c.pos) > 8.0 && c.brain.stuck < 3.0 => p,
                _ => {
                    // Sweep the arena: somewhere new, biased towards the
                    // enemy's side, that this body can actually stand in.
                    // The longer the search drags on, the more it homes in
                    // on where the enemy really is.
                    let arrived = c.brain.spot.is_none_or(|sp| sp.dist(c.pos) < 10.0);
                    if arrived || c.brain.timer > 10.0 || c.brain.stuck >= 3.0 {
                        let lost_for = ctx.time - brain.lost_at;
                        let instinct = ((lost_for - 8.0) / 30.0).clamp(0.0, 1.0);
                        let toward = ctx.enemy_hint.unwrap_or(ctx.enemy_side);
                        let mut spot = search_spot(ctx.world, rng, c.pos, toward.x > c.pos.x, lo);
                        if let Some(h) = ctx.enemy_hint {
                            // Lean the sweep towards them, then straight at them.
                            let lean = spot.lerp(h, instinct);
                            spot = if instinct >= 0.99 { h } else { lean };
                        }
                        c.brain.spot = Some(spot);
                        c.brain.timer = 0.0;
                        c.brain.stuck = 0.0;
                        c.brain.path.clear();
                        brain.last_seen = None;
                    }
                    c.brain.spot.unwrap()
                }
            };
            let mut it = intent(Some(goal), 0.85);
            it.fly = lo.fly && eff.can_fly;
            it.dig = lo.dig && c.burrowed;
            it.sneak = lo.camouflage > 0.0 && brain.last_seen.is_some();
            it
        }
        Tactic::Forage => {
            let Some(food) = brain.food else {
                brain.label = "looking for food";
                return intent(None, 0.0);
            };
            brain.label = if c.energy < 0.2 { "starving" } else { "foraging" };
            let reach = s * 1.1 + 4.0;
            let d = food.pos.dist(c.pos);
            if d < reach {
                brain.action = CombatAction::Eat(food.id);
                let mut it = intent(None, 0.0);
                it.sneak = true;
                if (c.facing > 0.0) != (food.pos.x > c.pos.x) && (food.pos.x - c.pos.x).abs() > 1.0 {
                    c.facing = (food.pos.x - c.pos.x).signum();
                }
                it
            } else {
                let mut it = intent(Some(food.pos), 0.9);
                it.fly = lo.fly && eff.can_fly && d > 20.0;
                it.sneak = lo.camouflage > 0.0;
                it.dig = lo.dig && c.burrowed;
                it
            }
        }
        Tactic::Hide => {
            // Somewhere out of the enemy's way; stay still so camouflage works.
            brain.label = "hiding";
            let threat = ctx.enemy_hint.unwrap_or(ctx.enemy_side);
            let exposed = c
                .brain
                .spot
                .is_none_or(|sp| ctx.time - brain.since > 25.0 && line_of_sight(ctx.world, threat, sp));
            if exposed || c.brain.stuck >= 3.0 {
                let mut best: Option<(V2, f32)> = None;
                for _ in 0..16 {
                    let p = search_spot(ctx.world, rng, c.pos, threat.x < c.pos.x, lo);
                    let d = p.dist(threat);
                    let hidden = !line_of_sight(ctx.world, threat, p);
                    let score = d + if hidden { 80.0 } else { 0.0 } - p.dist(c.pos) * 0.3;
                    if best.is_none_or(|b| score > b.1) {
                        best = Some((p, score));
                    }
                }
                c.brain.spot = best.map(|b| b.0);
                c.brain.stuck = 0.0;
                brain.since = ctx.time;
            }
            let spot = c.brain.spot.unwrap_or(c.pos);
            let mut it = if spot.dist(c.pos) > 6.0 {
                intent(Some(spot), 0.9)
            } else {
                intent(None, 0.0)
            };
            it.sneak = true;
            it.fly = lo.fly && eff.can_fly && spot.dist(c.pos) > 20.0;
            it.dig = lo.dig;
            it
        }
        Tactic::Evade => {
            brain.label = "evading";
            let from = target.map(|t| t.pos).or(brain.last_seen.map(|l| l.0));
            let Some(from) = from else {
                return intent(None, 0.0);
            };
            let d = from.dist(c.pos);
            if lo.ink && d < 30.0 && c.ink_cd <= 0.0 {
                brain.action = CombatAction::Ink;
            }
            if lo.leap > 0.0 && d < 25.0 && c.on_ground && c.leap_cd <= 0.0 {
                let dir = (c.pos - from).norm();
                brain.action = CombatAction::Leap((dir + v2(0.0, -0.6)).norm());
            }
            // Head for somewhere far, out of their sight, that we can stand in.
            let fresh = c
                .brain
                .spot
                .is_none_or(|sp| sp.dist(c.pos) < 8.0 || sp.dist(from) < d * 0.8 || c.brain.stuck >= 3.0);
            if fresh {
                let mut best: Option<(V2, f32)> = None;
                for _ in 0..14 {
                    let p = search_spot(ctx.world, rng, c.pos, from.x < c.pos.x, lo);
                    let far = p.dist(from);
                    let hidden = !line_of_sight(ctx.world, from, p);
                    let score = far + if hidden { 60.0 } else { 0.0 } - p.dist(c.pos) * 0.5;
                    if best.is_none_or(|b| score > b.1) {
                        best = Some((p, score));
                    }
                }
                c.brain.spot = Some(best.map_or_else(|| away_from(c, from, 70.0), |b| b.0));
                c.brain.stuck = 0.0;
            }
            let mut it = intent(c.brain.spot, 1.25);
            it.fly = lo.fly && eff.can_fly;
            it.dig = lo.dig;
            if d < 18.0 {
                // Too close for a plan: just get away.
                it.goal = Some(away_from(c, from, 40.0));
                it.direct = true;
            }
            it
        }
        Tactic::Retreat => {
            brain.label = "retreating";
            let from = target.map(|t| t.pos).or(brain.last_seen.map(|l| l.0));
            let Some(from) = from else {
                return intent(None, 0.0);
            };
            let d = from.dist(c.pos);
            if lo.ink && d < 30.0 && c.ink_cd <= 0.0 {
                brain.action = CombatAction::Ink;
            }
            // Break line of sight if we can.
            if c.brain
                .spot
                .is_none_or(|sp| sp.dist(c.pos) < 6.0 || line_of_sight(ctx.world, from, sp))
            {
                let mut best: Option<V2> = None;
                for _ in 0..12 {
                    let p = away_from(c, from, rng.range(35.0, 80.0))
                        + v2(rng.range(-20.0, 20.0), rng.range(-25.0, 10.0));
                    if ctx.world.in_bounds(p.x as i32, p.y as i32)
                        && ctx.world.material(p.x as i32, p.y as i32) == Material::Empty
                        && !line_of_sight(ctx.world, from, p)
                    {
                        best = Some(p);
                        break;
                    }
                }
                c.brain.spot = Some(best.unwrap_or_else(|| away_from(c, from, 60.0)));
            }
            let mut it = intent(c.brain.spot, 1.2);
            it.fly = lo.fly && eff.can_fly;
            it.dig = lo.dig;
            it
        }
        _ => {
            let Some(t) = target else {
                brain.label = "lost its target";
                return intent(brain.last_seen.map(|l| l.0), 0.8);
            };
            fight(
                c, brain, lo, eff, t, allies, cooldowns, origins, spit_cd, ctx, rng, s,
            )
        }
    };
    // Wedged on the way somewhere: go around for a few seconds.
    if let Some((spot, until)) = brain.detour {
        if ctx.time > until || c.brain.stuck >= 3.0 {
            brain.detour = None;
            c.brain.stuck = 0.0;
        } else if it.goal.is_some() && !matches!(brain.action, CombatAction::Strike(..)) {
            it.goal = Some(spot);
            it.direct = false;
        }
    } else if c.brain.stuck >= 3.0 && it.goal.is_some() {
        let toward = it.goal.unwrap_or(ctx.centre);
        let spot = search_spot(ctx.world, rng, c.pos, toward.x > c.pos.x, lo);
        brain.detour = Some((spot, ctx.time + 3.0));
        c.brain.stuck = 0.0;
        c.brain.path.clear();
        c.brain.repath = 0.0;
    }
    it.speed *= speed_mult;
    if lo.fly && !eff.can_fly {
        it.fly = false;
    }
    if !eff.can_jump
        && let CombatAction::Leap(_) = brain.action
    {
        brain.action = CombatAction::None;
    }
    it
}

/// Out of a liquid or fire: the nearest dry, open spot to stand on.
fn escape_hazard(c: &Creature, lo: &Loadout, eff: &Effects, ctx: &Ctx) -> Option<Intent> {
    let world = ctx.world;
    // The body centre and the feet: a shallow pool burns from below.
    let feet = c.pos + v2(0.0, lo.size * 0.9);
    let bad = |m: Material| {
        m == Material::Lava
            || m == Material::Acid
            || (matches!(m.kind(), crate::material::Kind::Liquid) && !lo.swim)
    };
    let harmful = [c.pos, feet, feet - v2(0.0, 1.0)]
        .iter()
        .any(|p| bad(world.material(p.x as i32, p.y as i32)));
    let drowning = c.in_liquid && c.breath < 0.7 && !lo.swim;

    if !(harmful || drowning || c.status.burning > 0.0) {
        return None;
    }
    let (cx, cy) = c.pos.cell();
    let mut best: Option<(V2, i32)> = None;
    for dy in -70..=10 {
        for dx in -70..=70 {
            let (x, y) = (cx + dx, cy + dy);
            if !world.in_bounds(x, y) {
                continue;
            }
            let dry = world.material(x, y) == Material::Empty
                && world.material(x, y - 3) == Material::Empty
                && world.material(x, y + 1).blocks_creature(lo.climb);
            if dry {
                // Prefer up and out over sideways.
                let d = dx * dx + dy * dy * if dy < 0 { 1 } else { 4 };
                if best.is_none_or(|b| d < b.1) {
                    best = Some((v2(x as f32 + 0.5, y as f32 - 1.0), d));
                }
            }
        }
    }
    let goal = best.map(|b| b.0).unwrap_or(c.pos - v2(0.0, 30.0));
    let mut it = intent(Some(goal), 1.2);
    it.fly = lo.fly && eff.can_fly;
    it.direct = c.in_liquid;
    Some(it)
}

/// A random open spot across the arena this body can reach: on the ground
/// for walkers, in the air for fliers, in soil for diggers.
fn search_spot(world: &World, rng: &mut Rng, from: V2, toward_right: bool, lo: &Loadout) -> V2 {
    let w = world.width() as i32;
    let h = world.height() as i32;
    for attempt in 0..40 {
        // Mostly far away, in the direction the enemy started.
        let x = if attempt < 25 {
            let dir = if toward_right { 1.0 } else { -1.0 };
            (from.x + dir * rng.range(40.0, 160.0)) as i32
        } else {
            rng.int(8, w - 9)
        };
        if x < 6 || x >= w - 6 {
            continue;
        }
        let y = rng.int(6, h - 8);
        let m = world.material(x, y);
        if lo.fly {
            if m == Material::Empty && (0..6).all(|k| world.material(x, y + k) == Material::Empty) {
                return v2(x as f32 + 0.5, y as f32 + 0.5);
            }
            continue;
        }
        if lo.dig && attempt % 2 == 0 && m.is_diggable() {
            return v2(x as f32 + 0.5, y as f32 + 0.5);
        }
        if m != Material::Empty {
            continue;
        }
        // Drop to the floor.
        let mut yy = y;
        while yy < h - 4 && !world.material(x, yy + 1).is_solid_for_player() {
            yy += 1;
        }
        let floor = world.material(x, yy + 1);
        if floor.is_solid_for_player() && world.material(x, yy - 3) == Material::Empty {
            return v2(x as f32 + 0.5, yy as f32 - 2.0);
        }
    }
    v2(w as f32 / 2.0, h as f32 / 2.0)
}

/// How far a weapon's origin is from the nearest part of the target.
fn weapon_gap(origins: &[(WeaponKind, V2)], kind: WeaponKind, t: &EnemySnap, fallback: V2) -> f32 {
    let origin = origins
        .iter()
        .find(|(k, _)| *k == kind)
        .map_or(fallback, |(_, p)| *p);
    let nearest = t.parts.iter().map(|p| p.0.dist(origin)).fold(f32::MAX, f32::min);
    if nearest == f32::MAX {
        t.pos.dist(origin)
    } else {
        (nearest - t.size * 0.25).max(0.0)
    }
}

/// Best weapon that is ready and within reach of the target.
fn ready_weapon<'a>(
    lo: &'a Loadout,
    eff: &Effects,
    cooldowns: &[f32],
    origins: &[(WeaponKind, V2)],
    t: &EnemySnap,
    me: V2,
    charging: bool,
) -> Option<&'a super::design::Weapon> {
    lo.weapons
        .iter()
        .filter(|w| cooldowns.get(w.kind as usize).copied().unwrap_or(0.0) <= 0.0)
        .filter(|w| match w.kind {
            WeaponKind::Claw => eff.claw_ok,
            WeaponKind::Punch => eff.punch_ok,
            WeaponKind::Tail => eff.tail_ok,
            WeaponKind::Tentacle => eff.tentacle_ok,
            WeaponKind::Horn => charging,
            WeaponKind::Bite => true,
        })
        .filter(|w| weapon_gap(origins, w.kind, t, me) <= w.reach)
        .max_by(|a, b| a.damage.total_cmp(&b.damage))
}

#[allow(clippy::too_many_arguments)]
fn fight(
    c: &mut Creature,
    brain: &mut CombatBrain,
    lo: &Loadout,
    eff: &Effects,
    t: &EnemySnap,
    allies: &[EnemySnap],
    cooldowns: &[f32],
    origins: &[(WeaponKind, V2)],
    spit_cd: f32,
    ctx: &Ctx,
    rng: &mut Rng,
    s: f32,
) -> Intent {
    let d = t.pos.dist(c.pos);
    let mut lead = t.pos + t.vel * 0.2;
    // Packs fan out: each member takes its own side and distance so they
    // don't pile up on one spot, and nobody crowds a packmate.
    let mut mates: Vec<u32> = allies
        .iter()
        .filter(|a| a.targeting == Some(t.id))
        .map(|a| a.id)
        .collect();
    mates.push(c.id);
    mates.sort_unstable();
    let slot = mates.iter().position(|&id| id == c.id).unwrap_or(0);
    if mates.len() > 1 {
        let side = if slot % 2 == 0 { 1.0 } else { -1.0 };
        let ring = (slot / 2) as f32;
        let off = v2(side * (s * 1.6 + 3.0 + ring * 7.0), -ring * 5.0);
        lead = t.pos + t.vel * 0.2 + off;
    }
    let sep = s * 2.4 + 5.0;
    let crowd: V2 = allies
        .iter()
        .filter(|a| a.pos.dist(c.pos) < sep)
        .map(|a| {
            let away = c.pos - a.pos;
            if away.len() < 0.1 {
                v2(if c.id.is_multiple_of(2) { 1.0 } else { -1.0 }, 0.0)
            } else {
                away.norm() * (sep - a.pos.dist(c.pos))
            }
        })
        .fold(V2::ZERO, |acc, v| acc + v);
    if crowd.len() > 0.5 {
        lead += crowd;
    }
    // Reach measured from the body centre: the weapon's own offset plus its reach.
    let longest = lo
        .weapons
        .iter()
        .map(|w| {
            let off = origins
                .iter()
                .find(|(k, _)| *k == w.kind)
                .map_or(0.0, |(_, p)| p.dist(c.pos));
            w.reach + off
        })
        .fold(s * 1.5, f32::max);
    // Close enough that some weapon can land.
    let in_range = lo
        .weapons
        .iter()
        .any(|w| weapon_gap(origins, w.kind, t, c.pos) <= w.reach * 0.7);
    let pounce = s * 3.0 + 18.0;
    let can_fly = lo.fly && eff.can_fly;
    let melee_now = |brain: &mut CombatBrain, charging: bool| -> bool {
        if let Some(w) = ready_weapon(lo, eff, cooldowns, origins, t, c.pos, charging) {
            brain.action = CombatAction::Strike(w.kind, t.id);
            true
        } else {
            false
        }
    };
    let spit_now = |brain: &mut CombatBrain| -> bool {
        if lo.spit.is_some()
            && spit_cd <= 0.0
            && d > 10.0
            && d < lo.spit_range
            && line_of_sight(ctx.world, c.pos, t.pos)
        {
            brain.action = CombatAction::Spit(t.pos + t.vel * (d / 110.0));
            true
        } else {
            false
        }
    };
    let leap_now = |brain: &mut CombatBrain, c: &Creature| {
        if lo.leap > 0.0
            && c.on_ground
            && c.leap_cd <= 0.0
            && d < pounce
            && d > longest * 0.8
            && t.pos.y > c.pos.y - 30.0
        {
            let dir = (t.pos - c.pos).norm();
            brain.action = CombatAction::Leap((dir + v2(0.0, -0.45)).norm());
        }
    };

    match brain.tactic {
        Tactic::Rush => {
            brain.label = "charging in";
            let mut it = intent(Some(lead), 1.2);
            it.fly = can_fly && (t.flying || d > 20.0);
            it.dig = lo.dig && c.burrowed;
            let charging = lo.weapon(WeaponKind::Horn).is_some() && c.vel.len() > 25.0;
            if !melee_now(brain, charging) {
                leap_now(brain, c);
                if brain.action == CombatAction::None && d > longest * 1.5 {
                    spit_now(brain);
                }
            }
            if in_range {
                // In the thick of it: stand and swing rather than push past.
                it.goal = None;
            }
            it
        }
        Tactic::Harass => {
            // Stage 0: close in and strike; stage 1: back off for a moment.
            if brain.stage == 0 {
                brain.label = "darting in";
                let mut it = intent(Some(lead), 1.3);
                it.fly = can_fly;
                if melee_now(brain, false) {
                    brain.stage = 1;
                    brain.stage_t = 0.0;
                } else {
                    leap_now(brain, c);
                }
                it
            } else {
                brain.label = "backing off";
                let mut it = intent(Some(away_from(c, t.pos, 40.0)), 1.2);
                it.fly = can_fly;
                it.direct = true;
                if brain.stage_t > 1.3 || d > 45.0 {
                    brain.stage = 0;
                    brain.stage_t = 0.0;
                }
                spit_now(brain);
                it
            }
        }
        Tactic::Flank => {
            brain.label = "flanking";
            // Come at it from the side nobody else is on.
            let ally_side = allies
                .iter()
                .filter(|a| a.targeting == Some(t.id))
                .map(|a| (a.pos.x - t.pos.x).signum())
                .sum::<f32>();
            let side = if ally_side.abs() > 0.1 {
                -ally_side.signum()
            } else if c.id.is_multiple_of(2) {
                1.0
            } else {
                -1.0
            };
            let behind = t.pos + v2(side * (longest * 0.8 + 6.0), -2.0);
            let mut it = intent(Some(if d < longest * 1.6 { lead } else { behind }), 1.15);
            it.fly = can_fly;
            if !melee_now(brain, false) {
                leap_now(brain, c);
            }
            it
        }
        Tactic::Kite => {
            brain.label = "keeping its distance";
            let near = lo.spit_range * 0.4;
            let far = lo.spit_range * 0.85;
            let goal = if d < near {
                away_from(c, t.pos, 30.0)
            } else if d > far || !line_of_sight(ctx.world, c.pos, t.pos) {
                lead
            } else {
                c.pos
            };
            let mut it = intent(Some(goal), 1.0);
            it.fly = can_fly;
            if !spit_now(brain) && d < longest {
                melee_now(brain, false);
            }
            it
        }
        Tactic::Ambush => {
            // Freeze until the target is close, then pounce.
            let seen = t.targeting == Some(c.id);
            if d < pounce || seen || ctx.frenzy {
                brain.label = "springing the ambush";
                let it = intent(Some(lead), 1.3);
                if !melee_now(brain, false) {
                    leap_now(brain, c);
                }
                if seen && brain.stage_t > 2.0 {
                    brain.set_tactic(Tactic::Rush, ctx.time);
                }
                it
            } else {
                brain.label = "lying in wait";
                let mut it = intent(None, 0.0);
                it.sneak = true;
                it.dig = lo.dig;
                it
            }
        }
        Tactic::Dive => {
            if brain.stage == 0 {
                brain.label = "climbing above its target";
                let above = t.pos + v2((c.pos.x - t.pos.x).signum() * 8.0, -28.0);
                let mut it = intent(Some(above), 1.1);
                it.fly = true;
                it.direct = true;
                if (c.pos.x - t.pos.x).abs() < 14.0 && c.pos.y < t.pos.y - 16.0 {
                    brain.stage = 1;
                    brain.stage_t = 0.0;
                }
                it
            } else {
                brain.label = "diving";
                let mut it = intent(Some(lead), 1.6);
                it.fly = true;
                it.direct = true;
                if melee_now(brain, false) || brain.stage_t > 2.5 {
                    brain.stage = 0;
                    brain.stage_t = 0.0;
                }
                it
            }
        }
        Tactic::Undermine => {
            let under = t.pos + v2(0.0, 10.0);
            if brain.stage == 0 {
                brain.label = "tunnelling underneath";
                let mut it = intent(Some(under), 1.0);
                it.dig = true;
                it.sneak = true;
                if under.dist(c.pos) < 9.0 || (c.burrowed && d < 12.0) {
                    brain.stage = 1;
                    brain.stage_t = 0.0;
                }
                it
            } else {
                brain.label = "bursting out";
                let mut it = intent(Some(t.pos), 1.4);
                it.dig = true;
                if brain.action == CombatAction::None && c.leap_cd <= 0.0 && brain.stage_t < 0.3 {
                    brain.action = CombatAction::Leap((t.pos - c.pos).norm() + v2(0.0, -0.3));
                }
                melee_now(brain, false);
                if brain.stage_t > 2.0 {
                    brain.stage = 0;
                }
                it
            }
        }
        Tactic::Hold => {
            brain.label = "holding ground";
            // Stand with something solid at the back, face the enemy, strike what comes.
            if c.brain.spot.is_none() {
                let mut best = c.pos;
                for _ in 0..10 {
                    let p = c.pos + v2(rng.range(-25.0, 25.0), 0.0);
                    let (x, y) = p.cell();
                    let backed = ctx.world.material(x - 4, y).is_solid_for_player()
                        || ctx.world.material(x + 4, y).is_solid_for_player();
                    if ctx.world.material(x, y) == Material::Empty && backed {
                        best = p;
                        break;
                    }
                }
                c.brain.spot = Some(best);
            }
            let spot = c.brain.spot.unwrap();
            let mut it = if spot.dist(c.pos) > 5.0 && d > longest {
                intent(Some(spot), 0.8)
            } else {
                intent(None, 0.0)
            };
            if !in_range && d < longest * 1.3 {
                // Step in for the hit.
                it.goal = Some(lead);
                it.speed = 1.0;
            }
            if (c.facing > 0.0) != (t.pos.x > c.pos.x) && d < 30.0 {
                c.facing = (t.pos.x - c.pos.x).signum();
            }
            if !melee_now(brain, false) {
                spit_now(brain);
            }
            it
        }
        Tactic::Search | Tactic::Retreat | Tactic::Forage | Tactic::Evade | Tactic::Hide => intent(None, 0.0),
    }
}
