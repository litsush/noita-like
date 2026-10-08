//! Body parts as separate hitboxes. A strike lands on whichever part is
//! nearest its contact point; limbs that run out of integrity are torn off
//! and the creature loses whatever they did for it.

use crate::eco::body::{self, limb_root, seg_world};
use crate::eco::creature::{Creature, limb_instances};
use crate::eco::genome::{LimbKind, LimbTip, Species};
use crate::eco::math::{V2, seg_dist, v2};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PartKind {
    Head,
    Body(u8),
    /// Limb instance index (see [`limb_instances`]).
    Limb(u8),
}

#[derive(Clone, Debug)]
pub struct PartDef {
    pub kind: PartKind,
    pub max_hp: f32,
    pub name: &'static str,
    /// Damage to this part also takes this fraction off overall health.
    pub to_health: f32,
    /// Whether armour plating covers it.
    pub armored: bool,
}

/// The parts of a species, in a fixed order: head, body segments, limbs.
pub fn layout(sp: &Species, max_health: f32) -> Vec<PartDef> {
    let b = &sp.body;
    let mut v = Vec::new();
    v.push(PartDef {
        kind: PartKind::Head,
        max_hp: max_health * 0.45,
        name: "head",
        to_health: 1.35,
        armored: false,
    });
    let n = b.segs.len().max(1) as f32;
    for i in 0..b.segs.len() {
        v.push(PartDef {
            kind: PartKind::Body(i as u8),
            max_hp: max_health * 0.7 / n.sqrt(),
            name: "body",
            to_health: 1.0,
            armored: true,
        });
    }
    for (k, (li, _)) in limb_instances(sp).into_iter().enumerate() {
        let l = &b.limbs[li];
        let (frac, name) = match l.kind {
            LimbKind::Leg => (0.24, "leg"),
            LimbKind::Arm => (0.2, "arm"),
            LimbKind::Tentacle => (0.14, "tentacle"),
            LimbKind::Wing => (0.15, "wing"),
            LimbKind::Tail => (0.26, "tail"),
            LimbKind::Antenna => (0.07, "feeler"),
            LimbKind::Fin => (0.12, "fin"),
            LimbKind::EyeStalk => (0.06, "eyestalk"),
        };
        v.push(PartDef {
            kind: PartKind::Limb(k as u8),
            max_hp: max_health * frac * (0.7 + l.width * 0.3),
            name,
            to_health: 0.55,
            armored: false,
        });
    }
    v
}

/// A contact: which part, how far the probe was from it, and the point on it.
#[derive(Clone, Copy, Debug)]
pub struct Contact {
    pub part: usize,
    pub dist: f32,
    pub at: V2,
}

/// The part of `c` nearest to `p`, if within `extra` cells of its surface.
pub fn nearest_part(c: &Creature, sp: &Species, defs: &[PartDef], p: V2, extra: f32) -> Option<Contact> {
    let s = c.scale(sp);
    let b = &sp.body;
    let mut best: Option<Contact> = None;
    let mut consider = |part: usize, d: f32, at: V2| {
        if d <= extra && best.is_none_or(|b| d < b.dist) {
            best = Some(Contact { part, dist: d, at });
        }
    };
    for (i, def) in defs.iter().enumerate() {
        match def.kind {
            PartKind::Head => {
                let h = body::head_world(c, sp);
                let r = if b.head_r > 0.0 {
                    b.head_r.max(b.head_ry) * s
                } else {
                    b.segs[0].r * s * 0.6
                };
                let d = (p.dist(h) - r).max(0.0);
                consider(i, d, h + (p - h).norm() * r);
            }
            PartKind::Body(k) => {
                let k = k as usize;
                if k >= b.segs.len() {
                    continue;
                }
                let (centre, _) = seg_world(c, sp, k);
                let r = b.segs[k].r.max(b.segs[k].ry) * s * 0.9;
                let d = (p.dist(centre) - r).max(0.0);
                consider(i, d, centre + (p - centre).norm() * r);
            }
            PartKind::Limb(k) => {
                let k = k as usize;
                if !c.has_limb(k) {
                    continue;
                }
                let insts = limb_instances(sp);
                let Some(&(li, far)) = insts.get(k) else { continue };
                let l = &b.limbs[li];
                let root = limb_root(c, sp, li, far);
                let w = l.width * s * 0.5 + 0.5;
                match l.kind {
                    LimbKind::Leg => {
                        let foot = c.anim.feet.get(k).map_or(root, |f| f.pos);
                        let d = (seg_dist(p, root, foot) - w).max(0.0);
                        consider(i, d, nearest_on_segment(p, root, foot));
                    }
                    LimbKind::Wing => {
                        let r = l.length * s * 0.45;
                        let centre = root + v2(0.0, -r * 0.5);
                        let d = (p.dist(centre) - r).max(0.0);
                        consider(i, d, centre + (p - centre).norm() * r);
                    }
                    _ => {
                        let Some(chain) = c.anim.chains.get(k) else {
                            continue;
                        };
                        if chain.len() < 2 {
                            continue;
                        }
                        for j in 1..chain.len() {
                            let d = (seg_dist(p, chain[j - 1], chain[j]) - w).max(0.0);
                            consider(i, d, nearest_on_segment(p, chain[j - 1], chain[j]));
                        }
                    }
                }
            }
        }
    }
    best
}

fn nearest_on_segment(p: V2, a: V2, b: V2) -> V2 {
    let ab = b - a;
    let t = if ab.len2() < 1e-6 {
        0.0
    } else {
        ((p - a).dot(ab) / ab.len2()).clamp(0.0, 1.0)
    };
    a + ab * t
}

/// What the body can still do given its parts.
#[derive(Clone, Copy, Debug)]
pub struct Effects {
    pub speed: f32,
    pub can_jump: bool,
    pub can_fly: bool,
    pub sight: f32,
    pub claw_ok: bool,
    pub punch_ok: bool,
    pub tail_ok: bool,
    pub tentacle_ok: bool,
    pub feelers_ok: bool,
    pub ears_ok: bool,
    /// Legs lost out of legs had.
    pub legs_lost: usize,
    pub legs_total: usize,
}

pub fn effects(c: &Creature, sp: &Species, defs: &[PartDef], hp: &[f32]) -> Effects {
    let b = &sp.body;
    let insts = limb_instances(sp);
    let mut legs_total = 0;
    let mut legs_ok = 0;
    let mut wings_total = 0;
    let mut wings_ok = 0;
    let (mut claw_ok, mut punch_ok, mut tail_ok, mut tentacle_ok, mut feelers_ok, mut ears_ok) =
        (false, false, false, false, false, false);
    for (k, &(li, _)) in insts.iter().enumerate() {
        let l = &b.limbs[li];
        let ok = c.has_limb(k);
        match l.kind {
            LimbKind::Leg => {
                legs_total += 1;
                if ok {
                    legs_ok += 1;
                    if l.tip == LimbTip::Claw {
                        claw_ok = true;
                    }
                }
            }
            LimbKind::Wing => {
                wings_total += 1;
                if ok {
                    wings_ok += 1;
                }
            }
            LimbKind::Arm if ok => {
                punch_ok = true;
                if l.tip == LimbTip::Claw {
                    claw_ok = true;
                }
            }
            LimbKind::Tail if ok => tail_ok = true,
            LimbKind::Tentacle if ok => tentacle_ok = true,
            LimbKind::Antenna if ok => {
                if l.tip == LimbTip::Paddle {
                    ears_ok = true;
                } else {
                    feelers_ok = true;
                }
            }
            _ => {}
        }
    }
    let speed = if legs_total == 0 {
        1.0
    } else {
        0.3 + 0.7 * legs_ok as f32 / legs_total as f32
    };
    let head_frac = defs
        .iter()
        .zip(hp)
        .find(|(d, _)| d.kind == PartKind::Head)
        .map_or(1.0, |(d, h)| (h / d.max_hp).clamp(0.0, 1.0));
    Effects {
        speed,
        can_jump: legs_total == 0 || legs_ok * 2 >= legs_total,
        can_fly: wings_total == 0 || wings_ok == wings_total || (wings_total >= 4 && wings_ok >= 3),
        sight: 0.35 + 0.65 * head_frac,
        claw_ok,
        punch_ok,
        tail_ok,
        tentacle_ok,
        feelers_ok,
        ears_ok,
        legs_lost: legs_total - legs_ok,
        legs_total,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eco::creature::init_anim;
    use crate::eco::math::v2;
    use crate::versus::design::{Design, Upgrade, build_species};

    #[test]
    fn parts_cover_head_body_and_limbs() {
        let mut d = Design::new(3);
        d.set(Upgrade::Tail, 2);
        let (sp, _) = build_species(&d, 0, None);
        let defs = layout(&sp, sp.stats.max_health);
        let limbs = limb_instances(&sp).len();
        assert_eq!(defs.len(), 1 + sp.body.segs.len() + limbs);
        let mut c = Creature::new(1, &sp, v2(100.0, 100.0), true, 0);
        init_anim(&mut c, &sp);
        let head = body::head_world(&c, &sp);
        let hit = nearest_part(&c, &sp, &defs, head, 2.0).expect("head is a part");
        assert_eq!(defs[hit.part].kind, PartKind::Head);
        assert!(nearest_part(&c, &sp, &defs, v2(0.0, 0.0), 2.0).is_none());
        let hp: Vec<f32> = defs.iter().map(|d| d.max_hp).collect();
        let e = effects(&c, &sp, &defs, &hp);
        assert_eq!(e.legs_total, 4);
        assert!(e.tail_ok && e.can_jump);
        // Tear off two legs.
        c.limb_ok = vec![true; limbs];
        let mut torn = 0;
        for (k, (li, _)) in limb_instances(&sp).into_iter().enumerate() {
            if sp.body.limbs[li].kind == LimbKind::Leg && torn < 2 {
                c.limb_ok[k] = false;
                torn += 1;
            }
        }
        let e = effects(&c, &sp, &defs, &hp);
        assert_eq!(e.legs_lost, 2);
        assert!(e.speed < 1.0);
    }
}
