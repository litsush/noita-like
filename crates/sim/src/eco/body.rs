//! Procedural animation and the drawable pose of a creature.
//!
//! Legs plant their feet on real terrain and step in alternating gait
//! groups; long bodies follow the head like a rope; tentacles, tails and
//! antennae sway with springy chains; wings flap. The result is a list of
//! simple primitives that the renderer rasterises at world resolution.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use super::creature::{Creature, limb_instances};
use super::genome::{Crest, LimbKind, LimbTip, Locomotion, Mouth, Species, Teeth, WingKind};
use super::math::{V2, angle_diff, v2};
use crate::world::World;

/// How a primitive is coloured.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Paint {
    /// Body colour with the species' pattern.
    Skin,
    Base,
    Belly,
    Accent,
    Dark,
    Eye,
    Glow,
    /// Far-side limbs: shaded base.
    Far,
    /// Near-side limbs: a shade between base and accent.
    Limb,
    Fixed([u8; 3]),
}

#[derive(Clone, Debug)]
pub enum Prim {
    Ellipse {
        c: V2,
        rx: f32,
        ry: f32,
        angle: f32,
        paint: Paint,
        outline: bool,
    },
    Line {
        a: V2,
        b: V2,
        w0: f32,
        w1: f32,
        paint: Paint,
    },
    Tri {
        p: [V2; 3],
        paint: Paint,
    },
    Dot {
        p: V2,
        r: f32,
        paint: Paint,
    },
}

/// Body frame: local x forward (mirrored by facing), y down.
struct Frame {
    origin: V2,
    angle: f32,
    facing: f32,
}

impl Frame {
    #[inline]
    fn pt(&self, l: V2) -> V2 {
        self.origin + v2(l.x * self.facing, l.y).rot(self.angle)
    }
    #[inline]
    fn dir(&self, local_angle: f32) -> V2 {
        let d = V2::from_angle(local_angle);
        v2(d.x * self.facing, d.y).rot(self.angle)
    }
}

fn frame(c: &Creature) -> Frame {
    Frame {
        origin: c.pos,
        angle: c.anim.angle,
        facing: c.facing,
    }
}

/// Local-space centre of segment `i` (rigid bodies).
fn seg_local(sp: &Species, i: usize, s: f32) -> V2 {
    if sp.body.upright {
        v2(0.0, i as f32 * sp.body.spacing * s)
    } else {
        v2(-(i as f32) * sp.body.spacing * s, 0.0)
    }
}

fn head_local(sp: &Species, s: f32) -> V2 {
    let b = &sp.body;
    let s0 = b.segs[0];
    if b.upright {
        v2(s0.r * 0.15, -(s0.ry + b.neck + b.head_ry * 0.8) * s)
    } else {
        v2((s0.r + b.neck + b.head_r * 0.55) * s, -(b.neck * 0.5) * s)
    }
}

/// World position and body angle of a segment.
pub(crate) fn seg_world(c: &Creature, sp: &Species, i: usize) -> (V2, f32) {
    let f = frame(c);
    if sp.body.rope && i < c.anim.segs.len() {
        let p = c.anim.segs[i];
        let ahead = if i == 0 {
            f.pt(v2(1.0, 0.0))
        } else {
            c.anim.segs[i - 1]
        };
        let d = ahead - p;
        let a = if d.len() > 0.01 { d.angle() } else { f.angle };
        // Convert the travel direction into a body angle for our facing.
        let a = if c.facing > 0.0 { a } else { a + PI };
        (p, a)
    } else {
        (f.pt(seg_local(sp, i, c.scale(sp))), f.angle)
    }
}

/// World position of a limb's root.
pub(crate) fn limb_root(c: &Creature, sp: &Species, li: usize, far: bool) -> V2 {
    let l = &sp.body.limbs[li];
    let s = c.scale(sp);
    let f = frame(c);
    let (centre, r, ry) = if l.seg < 0 {
        let h = head_local(sp, s);
        (f.pt(h), sp.body.head_r * s, sp.body.head_ry * s)
    } else {
        let i = (l.seg as usize).min(sp.body.segs.len() - 1);
        let (p, _) = seg_world(c, sp, i);
        (p, sp.body.segs[i].r * s, sp.body.segs[i].ry * s)
    };
    let off = v2(l.angle.cos() * r * 0.75, l.angle.sin() * ry * 0.75);
    let off = v2(off.x * c.facing, off.y).rot(seg_angle_for(c, sp, l.seg));
    let depth = if far {
        v2(c.facing * 0.8, -0.6).rot(f.angle)
    } else {
        V2::ZERO
    };
    centre + off + depth
}

fn seg_angle_for(c: &Creature, sp: &Species, seg: i32) -> f32 {
    if seg < 0 || !sp.body.rope {
        c.anim.angle
    } else {
        seg_world(c, sp, seg as usize).1
    }
}

fn solid(world: &World, climb: bool, p: V2) -> bool {
    world
        .material(p.x.floor() as i32, p.y.floor() as i32)
        .blocks_creature(climb)
}

/// Marches from `from` along `dir` up to `max` cells; returns the last open
/// point before solid ground.
fn raycast(world: &World, climb: bool, from: V2, dir: V2, max: f32) -> Option<V2> {
    let mut p = from;
    let steps = max.ceil() as i32;
    for _ in 0..steps {
        let n = p + dir;
        if solid(world, climb, n) {
            return Some(p);
        }
        p = n;
    }
    None
}

/// World position of the head centre (or the front segment if headless).
pub fn head_world(c: &Creature, sp: &Species) -> V2 {
    if sp.body.head_r > 0.0 {
        frame(c).pt(head_local(sp, c.scale(sp)))
    } else {
        seg_world(c, sp, 0).0
    }
}

/// Where the mouth is: the front of the head.
pub fn mouth_world(c: &Creature, sp: &Species) -> V2 {
    let f = frame(c);
    let s = c.scale(sp);
    let r = if sp.body.head_r > 0.0 {
        sp.body.head_r * s
    } else {
        sp.body.segs[0].r * s
    };
    head_world(c, sp) + f.dir(0.15) * (r * 0.9)
}

/// Unit vector the creature faces, in world space (follows body tilt).
pub fn forward(c: &Creature) -> V2 {
    frame(c).dir(0.0)
}

/// Advances all animation state by `dt`.
pub fn animate(c: &mut Creature, sp: &Species, world: &World, dt: f32, flying: bool) {
    let s = c.scale(sp);
    let climb = sp.traits.climb;
    let b = &sp.body;
    let a = &mut c.anim;
    a.t += dt;
    a.attack = (a.attack - dt).max(0.0);
    a.eat = (a.eat - dt).max(0.0);
    a.hurt = (a.hurt - dt).max(0.0);
    a.blink -= dt;
    if a.blink < -0.15 {
        a.blink = 2.0 + (c.id % 7) as f32 * 0.4;
    }
    let speed = c.vel.len();
    let stride = (b.stance.max(1.0) * s * 1.5).max(2.0);
    a.walk += c.vel.x.abs().max(if c.clinging { speed } else { 0.0 }) * dt / stride;
    let flap_rate = if flying {
        14.0 / (sp.stats.size * s).sqrt().max(1.0)
    } else {
        0.0
    };
    a.flap += dt * flap_rate;

    // Body tilt.
    let target = if c.clinging {
        a.surface.angle() - FRAC_PI_2
    } else if flying || (c.in_liquid && sp.traits.swim) || c.burrowed {
        if b.upright {
            (c.vel.x * 0.004).clamp(-0.3, 0.3)
        } else {
            let a = v2(c.vel.x.abs(), c.vel.y).angle() * c.facing;
            a.clamp(-0.8, 0.8) * if speed > 4.0 { 1.0 } else { 0.0 }
        }
    } else if c.playing_dead > 0.0 || c.dead.is_some() {
        PI * 0.95
    } else {
        0.0
    };
    a.angle += angle_diff(a.angle, target) * (dt * 6.0).min(1.0);
    if c.clinging {
        let fwd = v2(1.0, 0.0).rot(a.angle);
        let along = c.vel.dot(fwd);
        if along.abs() > 2.0 {
            c.facing = along.signum();
        }
    }

    // Rope bodies.
    let n = b.segs.len();
    if c.anim.segs.len() != n {
        c.anim.segs = vec![c.pos; n];
    }
    if b.rope {
        c.anim.segs[0] = c.pos;
        let spacing = b.spacing * s;
        let swim = (c.in_liquid && sp.traits.swim) || flying;
        for i in 1..n {
            let prev = c.anim.segs[i - 1];
            let mut p = c.anim.segs[i];
            if !swim && !c.burrowed && !c.clinging && !solid(world, climb, p + v2(0.0, 1.0)) {
                p.y += 0.6;
            }
            let mut d = p - prev;
            if d.len() < 0.01 {
                d = v2(-c.facing, 0.0);
            }
            let mut np = prev + d.norm() * spacing;
            if speed > 3.0 && (swim || sp.loco == Locomotion::Slither) {
                let wave = (c.anim.t * 7.0 - i as f32 * 0.9).sin() * 0.35 * spacing * (i as f32 / n as f32);
                np += d.norm().perp() * wave;
            }
            if !c.burrowed && solid(world, climb, np) && !solid(world, climb, np - v2(0.0, 1.5)) {
                np.y -= 1.0;
            }
            c.anim.segs[i] = np;
        }
    } else {
        for i in 0..n {
            c.anim.segs[i] = seg_world(c, sp, i).0;
        }
    }

    // Limbs.
    let insts = limb_instances(sp);
    if c.anim.feet.len() != insts.len() {
        super::creature::init_anim(c, sp);
    }
    let f = frame(c);
    let down = if c.clinging { c.anim.surface } else { v2(0.0, 1.0) };
    let grounded_body = (c.on_ground || c.clinging) && !flying && !(c.in_liquid && sp.traits.swim);
    let mut stepping = 0usize;
    let legs: usize = insts
        .iter()
        .enumerate()
        .filter(|(k, (li, _))| b.limbs[*li].kind == LimbKind::Leg && c.has_limb(*k))
        .count();
    stepping += insts
        .iter()
        .zip(&c.anim.feet)
        .filter(|((li, _), f)| f.t < 1.0 && b.limbs[*li].kind == LimbKind::Leg)
        .count();
    let reach = c.anim.reach;
    for (k, &(li, far)) in insts.iter().enumerate() {
        if !c.has_limb(k) {
            continue;
        }
        let l = &b.limbs[li];
        let root = limb_root(c, sp, li, far);
        let len = l.length * s;
        match l.kind {
            LimbKind::Leg => {
                let lead = c.vel * 0.12 + f.dir(0.0) * len * if far { 0.15 } else { -0.05 };
                let spread = f.dir(l.angle) * (len * 0.25);
                let probe = root + lead + v2(spread.x, 0.0);
                let ground = if grounded_body {
                    raycast(world, climb, probe, down, len * 1.4)
                } else {
                    None
                };
                let foot = &mut c.anim.feet[k];
                match ground {
                    Some(target) => {
                        if !foot.grounded {
                            foot.pos = target;
                            foot.from = target;
                            foot.to = target;
                            foot.t = 1.0;
                            foot.grounded = true;
                        }
                        if foot.t >= 1.0 {
                            let off = foot.pos.dist(target);
                            let gate = ((c.anim.walk + l.phase + if far { 0.5 } else { 0.0 }) % 1.0) < 0.5;
                            let can = stepping < (legs / 2).max(1);
                            if (off > len * 0.5 && gate && can)
                                || off > len * 1.1
                                || foot.pos.dist(root) > len * 1.05
                            {
                                foot.from = foot.pos;
                                foot.to = target;
                                foot.t = 0.0;
                                stepping += 1;
                            }
                        } else {
                            let rate = (speed / (len * 0.5)).clamp(4.0, 14.0);
                            foot.t = (foot.t + dt * rate).min(1.0);
                            let lift = -down * ((foot.t * PI).sin() * len * 0.3);
                            foot.to = target;
                            foot.pos = foot.from.lerp(foot.to, foot.t) + lift;
                        }
                    }
                    None => {
                        // Dangling or tucked.
                        let tuck = if flying { 0.55 } else { 0.85 };
                        let sway = (c.anim.t * 3.0 + l.phase * TAU).sin() * 0.25;
                        let dir = down.rot(sway - c.facing * if flying { 0.6 } else { 0.0 });
                        foot.pos = foot.pos.lerp(root + dir * len * tuck, 0.3);
                        foot.grounded = false;
                        foot.t = 1.0;
                    }
                }
            }
            LimbKind::Wing => {}
            _ => {
                let chain = &mut c.anim.chains[k];
                let joints = chain.len().max(2);
                let seg_len = len / (joints - 1) as f32;
                let mut base_dir = f.dir(l.angle);
                let in_liquid = c.in_liquid;
                let aimed = reach.filter(|(rk, _)| *rk == k).map(|(_, p)| p);
                let (mut amp, freq, mut droop) = match l.kind {
                    LimbKind::Tentacle => (0.45, 2.2, if in_liquid { 0.05 } else { 0.35 }),
                    LimbKind::Tail => (0.25, 3.0 + speed * 0.05, 0.12),
                    LimbKind::Antenna => (0.2, 4.0, -0.05),
                    LimbKind::Fin => (0.5, 8.0, 0.0),
                    LimbKind::EyeStalk => (0.15, 2.0, -0.1),
                    LimbKind::Arm => (0.15, 2.0, 0.4),
                    _ => (0.2, 2.0, 0.2),
                };
                if l.kind == LimbKind::Arm && c.anim.attack > 0.0 {
                    base_dir = f.dir(0.1);
                }
                if let Some(target) = aimed {
                    // Striking: the whole limb points at the target.
                    let d = target - root;
                    if d.len() > 0.5 {
                        base_dir = d.norm();
                    }
                    amp *= 0.2;
                    droop = 0.0;
                }
                if (in_liquid || flying)
                    && matches!(l.kind, LimbKind::Tentacle | LimbKind::Tail)
                    && speed > 4.0
                {
                    base_dir = base_dir.lerp(-c.vel.norm(), 0.5).norm();
                }
                chain[0] = root;
                let mut dir = base_dir;
                for i in 1..joints {
                    let t = i as f32 / (joints - 1) as f32;
                    let wave = (c.anim.t * freq + l.phase * TAU + i as f32 * 0.8).sin() * amp * t;
                    dir = dir.rot(wave * 0.6);
                    dir = dir.lerp(v2(0.0, 1.0), droop * t).norm();
                    let ideal = chain[i - 1] + dir * seg_len;
                    let mut p = chain[i].lerp(ideal, if aimed.is_some() { 0.8 } else { 0.45 });
                    let d = p - chain[i - 1];
                    p = chain[i - 1] + d.norm() * seg_len;
                    if solid(world, climb, p) {
                        p = chain[i - 1] + (dir - v2(0.0, 0.6)).norm() * seg_len;
                    }
                    chain[i] = p;
                }
            }
        }
    }
}

/// Primitives to draw a creature, back to front.
pub fn pose(c: &Creature, sp: &Species) -> Vec<Prim> {
    let mut out = Vec::with_capacity(48);
    let s = c.scale(sp);
    let b = &sp.body;
    let f = frame(c);
    let insts = limb_instances(sp);
    let flying = c.flying;
    let squash = if c.anim.hurt > 0.0 { 0.85 } else { 1.0 };

    let limb = |out: &mut Vec<Prim>, k: usize, li: usize, far: bool| {
        let l = &b.limbs[li];
        let paint = if far { Paint::Far } else { Paint::Limb };
        let w = l.width * s.max(0.7);
        let root = limb_root(c, sp, li, far);
        let len = l.length * s;
        match l.kind {
            LimbKind::Leg => {
                let foot = c.anim.feet[k].pos;
                let d = foot - root;
                let dist = d.len().min(len * 0.999);
                // Two-bone IK: knee bends forward for bipeds, up for many-legged walkers.
                let half = len / 2.0;
                let h = (half * half - (dist / 2.0).powi(2)).max(0.0).sqrt();
                let bend = if b.upright || b.count(LimbKind::Leg) <= 4 {
                    c.facing
                } else {
                    -c.facing
                };
                let mid = root + d.norm() * (dist / 2.0);
                let mut perp = d.norm().perp() * bend;
                if b.count(LimbKind::Leg) > 4 && perp.y > 0.0 {
                    perp = -perp;
                }
                let knee = mid + perp * h;
                out.push(Prim::Line {
                    a: root,
                    b: knee,
                    w0: w + 0.5,
                    w1: w,
                    paint,
                });
                out.push(Prim::Line {
                    a: knee,
                    b: foot,
                    w0: w,
                    w1: (w - 0.4).max(0.6),
                    paint,
                });
                tip(out, l.tip, foot, (foot - knee).norm(), s, far);
            }
            LimbKind::Wing => {
                let flap = if flying { (c.anim.flap * TAU).sin() } else { -1.0 };
                let up = if flying {
                    -FRAC_PI_2 + flap * 0.9
                } else {
                    PI - 0.35
                };
                let tipp = root + f.dir(up + l.phase * 0.3) * len;
                let back = root + f.dir(PI - 0.2) * (len * 0.55) + v2(0.0, if flying { 0.0 } else { -1.0 });
                let wp = match (b.wing, far) {
                    (_, true) => Paint::Far,
                    (WingKind::Insect, false) => Paint::Fixed([200, 225, 235]),
                    (WingKind::Feather, false) => Paint::Accent,
                    (WingKind::Membrane, false) => Paint::Belly,
                };
                out.push(Prim::Tri {
                    p: [root, tipp, back],
                    paint: wp,
                });
                out.push(Prim::Line {
                    a: root,
                    b: tipp,
                    w0: 1.2,
                    w1: 0.7,
                    paint: if far { Paint::Far } else { Paint::Dark },
                });
                if b.wing == WingKind::Feather && !far {
                    for i in 1..4 {
                        let t = i as f32 / 4.0;
                        let a = root.lerp(tipp, t);
                        out.push(Prim::Line {
                            a,
                            b: a + f.dir(up + 1.6) * (len * 0.35),
                            w0: 1.0,
                            w1: 0.6,
                            paint: Paint::Accent,
                        });
                    }
                }
            }
            LimbKind::Fin => {
                let chain = &c.anim.chains[k];
                let end = *chain.last().unwrap_or(&root);
                let side = (end - root).perp().norm() * (len * 0.35);
                out.push(Prim::Tri {
                    p: [root, end + side, end - side],
                    paint: if far { Paint::Far } else { Paint::Accent },
                });
            }
            _ => {
                let chain = &c.anim.chains[k];
                let n = chain.len();
                for i in 1..n {
                    let t0 = (i - 1) as f32 / (n - 1) as f32;
                    let t1 = i as f32 / (n - 1) as f32;
                    let taper = |t: f32| (w + 0.6) * (1.0 - t * 0.6);
                    let p = if l.kind == LimbKind::Tail && !far {
                        Paint::Skin
                    } else {
                        paint
                    };
                    out.push(Prim::Line {
                        a: chain[i - 1],
                        b: chain[i],
                        w0: taper(t0),
                        w1: taper(t1),
                        paint: p,
                    });
                }
                if n >= 2 {
                    let dir = (chain[n - 1] - chain[n - 2]).norm();
                    tip(out, l.tip, chain[n - 1], dir, s, far);
                }
            }
        }
    };

    // Far side and things behind the body.
    for (k, &(li, far)) in insts.iter().enumerate() {
        let l = &b.limbs[li];
        let behind = far || matches!(l.kind, LimbKind::Tail) || (l.kind == LimbKind::Tentacle && b.sac > 0.0);
        if behind && c.has_limb(k) {
            limb(&mut out, k, li, far);
        }
    }

    // Gas sac.
    if b.sac > 0.0 {
        let c0 = f.pt(v2(0.0, -(b.segs[0].ry + b.sac * 0.8) * s));
        out.push(Prim::Ellipse {
            c: c0,
            rx: b.sac * s,
            ry: b.sac * s * 0.9,
            angle: f.angle,
            paint: Paint::Belly,
            outline: true,
        });
        out.push(Prim::Dot {
            p: c0 + v2(-b.sac * 0.35 * s, -b.sac * 0.35 * s),
            r: (b.sac * 0.25 * s).max(0.6),
            paint: Paint::Fixed([240, 240, 245]),
        });
    }

    // Body segments, tail end first.
    for i in (0..b.segs.len()).rev() {
        let sg = b.segs[i];
        let (p, a) = seg_world(c, sp, i);
        out.push(Prim::Ellipse {
            c: p,
            rx: sg.r * s * if b.upright { 1.0 } else { 1.05 },
            ry: sg.ry * s * squash,
            angle: a,
            paint: Paint::Skin,
            outline: true,
        });
    }
    if b.shell {
        let (p, a) = seg_world(c, sp, b.segs.len() / 2);
        let sg = b.segs[b.segs.len() / 2];
        let up = v2(0.0, -sg.ry * 0.35 * s).rot(a);
        out.push(Prim::Ellipse {
            c: p + up,
            rx: sg.r * s * 0.95,
            ry: sg.ry * s * 0.6,
            angle: a,
            paint: Paint::Accent,
            outline: true,
        });
    }
    // Crest along the back.
    if b.crest != Crest::None {
        for i in 0..b.segs.len() {
            let sg = b.segs[i];
            let (p, a) = seg_world(c, sp, i);
            let top = p + v2(0.0, -sg.ry * s).rot(a);
            let up = v2(0.0, -1.0).rot(a);
            match b.crest {
                Crest::Spines => {
                    for j in -1..=1 {
                        let base = top + v2(j as f32 * sg.r * 0.5 * s * c.facing, 0.0).rot(a);
                        out.push(Prim::Line {
                            a: base,
                            b: base + (up + v2(-c.facing * 0.4, 0.0)).norm() * (sg.ry * 0.9 * s + 1.5),
                            w0: 1.0,
                            w1: 0.5,
                            paint: Paint::Accent,
                        });
                    }
                }
                Crest::Sail if i == 0 || b.segs.len() == 1 => {
                    let back = top + v2(-sg.r * s * c.facing, 0.0).rot(a);
                    out.push(Prim::Tri {
                        p: [top, back, top.lerp(back, 0.4) + up * (sg.ry * 1.6 * s + 1.0)],
                        paint: Paint::Accent,
                    });
                }
                Crest::Frills if i == 0 => {
                    for j in 0..3 {
                        let base = top + v2(-(j as f32) * 1.2 * c.facing, 0.0);
                        out.push(Prim::Line {
                            a: base,
                            b: base + up * (2.0 + j as f32),
                            w0: 1.0,
                            w1: 0.6,
                            paint: Paint::Accent,
                        });
                    }
                }
                _ => {}
            }
        }
    }

    // Head.
    if b.head_r > 0.0 {
        let h = f.pt(head_local(sp, s));
        if b.neck > 0.2 && !b.upright {
            out.push(Prim::Line {
                a: seg_world(c, sp, 0).0,
                b: h,
                w0: b.head_r * s * 0.9,
                w1: b.head_r * s * 0.7,
                paint: Paint::Skin,
            });
        }
        out.push(Prim::Ellipse {
            c: h,
            rx: b.head_r * s,
            ry: b.head_ry * s * squash,
            angle: f.angle,
            paint: Paint::Skin,
            outline: true,
        });
        let fwd = f.dir(0.15);
        let mouth_at = h + fwd * (b.head_r * s * 0.9);
        let open = if c.anim.attack > 0.0 || c.anim.eat > 0.0 {
            1.0
        } else {
            0.0
        };
        match b.mouth {
            Mouth::Mandibles => {
                for side in [-1.0, 1.0] {
                    let base = mouth_at + v2(0.0, side * 0.8).rot(f.angle);
                    let a = 0.4 * side + open * 0.5 * side;
                    out.push(Prim::Line {
                        a: base,
                        b: base + f.dir(a) * (1.5 + s * 0.6),
                        w0: 1.0,
                        w1: 0.6,
                        paint: Paint::Dark,
                    });
                }
            }
            Mouth::Beak => {
                let up = v2(0.0, -1.0).rot(f.angle);
                out.push(Prim::Tri {
                    p: [
                        mouth_at + up * 1.0,
                        mouth_at - up * (0.6 + open),
                        mouth_at + fwd * (2.0 + s * 0.5),
                    ],
                    paint: Paint::Fixed([210, 190, 120]),
                });
            }
            Mouth::Maw => {
                out.push(Prim::Dot {
                    p: mouth_at,
                    r: (b.head_r * s * (0.35 + open * 0.2)).max(0.7),
                    paint: Paint::Fixed([40, 10, 16]),
                });
                for i in 0..3 {
                    let a = i as f32 / 3.0 * TAU + c.anim.t;
                    out.push(Prim::Dot {
                        p: mouth_at + V2::from_angle(a) * (b.head_r * s * 0.35),
                        r: 0.5,
                        paint: Paint::Fixed([235, 230, 210]),
                    });
                }
            }
            Mouth::Proboscis => {
                out.push(Prim::Line {
                    a: mouth_at,
                    b: mouth_at + f.dir(0.5 - open * 0.4) * (2.0 + s),
                    w0: 0.9,
                    w1: 0.6,
                    paint: Paint::Dark,
                });
            }
            Mouth::Sucker => {
                out.push(Prim::Dot {
                    p: mouth_at,
                    r: 0.9,
                    paint: Paint::Accent,
                });
            }
            Mouth::Jaws => {
                let down = v2(0.0, 1.0).rot(f.angle);
                let gape = open * (1.5 + s * 0.3);
                out.push(Prim::Line {
                    a: mouth_at - fwd * 1.5,
                    b: mouth_at + down * gape,
                    w0: 0.8,
                    w1: 0.8,
                    paint: Paint::Dark,
                });
                teeth(&mut out, b.teeth, mouth_at, fwd, down, gape, s);
            }
        }
        if b.mouth == Mouth::Maw {
            let down = v2(0.0, 1.0).rot(f.angle);
            teeth(&mut out, b.teeth, mouth_at, fwd, down, open * s * 0.3, s * 0.8);
        }
        if b.horns > 0.0 {
            let up = v2(0.0, -1.0).rot(f.angle);
            for side in [0.0, 1.0] {
                let base = h + up * (b.head_ry * s * 0.7) - fwd * (side * 1.0);
                out.push(Prim::Line {
                    a: base,
                    b: base + (up - fwd * 0.6).norm() * ((1.5 + s * 0.8) * b.horns),
                    w0: 1.0 + b.horns * 0.3,
                    w1: 0.5,
                    paint: Paint::Fixed([225, 215, 190]),
                });
            }
        }
    }

    // Eyes.
    let eye_host = if b.head_r > 0.0 {
        (f.pt(head_local(sp, s)), b.head_r * s)
    } else {
        (seg_world(c, sp, 0).0, b.segs[0].r * s)
    };
    let closed = c.sleeping || c.anim.blink < 0.0 || c.dead.is_some() || c.playing_dead > 0.0;
    for e in &b.eyes {
        let p = eye_host.0 + f.dir(e.angle) * (eye_host.1 * e.dist);
        if closed {
            out.push(Prim::Dot {
                p,
                r: 0.5,
                paint: Paint::Dark,
            });
        } else {
            let r = (e.size * s * 0.6).max(0.5);
            if r > 0.9 {
                out.push(Prim::Dot {
                    p,
                    r,
                    paint: Paint::Fixed([235, 235, 225]),
                });
            }
            out.push(Prim::Dot {
                p: p + f.dir(0.0) * (r * 0.3),
                r: (r * 0.6).max(0.5),
                paint: Paint::Eye,
            });
        }
    }

    // Near side.
    for (k, &(li, far)) in insts.iter().enumerate() {
        let l = &b.limbs[li];
        let behind = far || matches!(l.kind, LimbKind::Tail) || (l.kind == LimbKind::Tentacle && b.sac > 0.0);
        if !behind && c.has_limb(k) {
            limb(&mut out, k, li, far);
        }
    }

    // Bioluminescent spots and lures.
    if sp.colors.glow.is_some() {
        for i in 0..b.segs.len() {
            let (p, a) = seg_world(c, sp, i);
            let sg = b.segs[i];
            out.push(Prim::Dot {
                p: p + v2(0.0, sg.ry * 0.3 * s).rot(a),
                r: (sg.r * 0.25 * s).max(0.6),
                paint: Paint::Glow,
            });
        }
        if c.luring && b.head_r > 0.0 {
            let h = f.pt(head_local(sp, s));
            let stalk_top = h + f.dir(-1.2) * (b.head_r * s + 3.0);
            let lure = stalk_top + f.dir(0.3) * 3.0 + v2(0.0, (c.anim.t * 2.0).sin() * 0.8);
            out.push(Prim::Line {
                a: h,
                b: stalk_top,
                w0: 0.8,
                w1: 0.6,
                paint: Paint::Dark,
            });
            out.push(Prim::Line {
                a: stalk_top,
                b: lure,
                w0: 0.6,
                w1: 0.6,
                paint: Paint::Dark,
            });
            out.push(Prim::Dot {
                p: lure,
                r: 1.2,
                paint: Paint::Glow,
            });
        }
    }
    out
}

fn tip(out: &mut Vec<Prim>, kind: LimbTip, at: V2, dir: V2, s: f32, far: bool) {
    let paint = if far { Paint::Far } else { Paint::Dark };
    let k = (s * 0.8 + 0.8).min(3.0);
    match kind {
        LimbTip::Claw | LimbTip::Hook => {
            out.push(Prim::Line {
                a: at,
                b: at + dir.rot(0.9) * k,
                w0: 0.8,
                w1: 0.5,
                paint,
            });
        }
        LimbTip::Hand => {
            for a in [-0.55, 0.0, 0.55] {
                out.push(Prim::Line {
                    a: at,
                    b: at + dir.rot(a) * (k * 0.9),
                    w0: 0.8,
                    w1: 0.5,
                    paint,
                });
            }
        }
        LimbTip::Pincer => {
            out.push(Prim::Line {
                a: at,
                b: at + dir.rot(0.5) * k * 1.3,
                w0: 1.0,
                w1: 0.6,
                paint,
            });
            out.push(Prim::Line {
                a: at,
                b: at + dir.rot(-0.5) * k * 1.3,
                w0: 1.0,
                w1: 0.6,
                paint,
            });
        }
        LimbTip::Stinger | LimbTip::Spike => {
            out.push(Prim::Line {
                a: at,
                b: at + dir * k * 1.3,
                w0: 1.0,
                w1: 0.4,
                paint: if far { Paint::Far } else { Paint::Accent },
            });
        }
        LimbTip::Club => out.push(Prim::Dot {
            p: at,
            r: k * 0.7,
            paint: if far { Paint::Far } else { Paint::Accent },
        }),
        LimbTip::Sucker => out.push(Prim::Dot {
            p: at,
            r: 0.6,
            paint: if far { Paint::Far } else { Paint::Belly },
        }),
        LimbTip::Paddle => out.push(Prim::Tri {
            p: [at, at + dir.rot(0.7) * k, at + dir.rot(-0.7) * k],
            paint,
        }),
        LimbTip::Eye => {
            out.push(Prim::Dot {
                p: at,
                r: 1.0,
                paint: Paint::Fixed([235, 235, 225]),
            });
            out.push(Prim::Dot {
                p: at + dir * 0.3,
                r: 0.6,
                paint: Paint::Eye,
            });
        }
        LimbTip::None => {}
    }
}

/// Teeth along the upper jaw, growing with the gape.
fn teeth(out: &mut Vec<Prim>, kind: Teeth, mouth_at: V2, fwd: V2, down: V2, gape: f32, s: f32) {
    let ivory = Paint::Fixed([240, 236, 220]);
    let jaw = mouth_at - fwd * 1.5;
    let n = (s * 0.8).clamp(2.0, 6.0) as i32;
    match kind {
        Teeth::None => {}
        Teeth::Flat => {
            for i in 0..n {
                let t = i as f32 / n as f32;
                let p = jaw + fwd * (t * 2.0);
                out.push(Prim::Dot {
                    p: p + down * 0.4,
                    r: 0.45,
                    paint: ivory,
                });
            }
        }
        Teeth::Canines => {
            let len = (0.9 + s * 0.35).max(1.0);
            for (k, t) in [0.25, 0.75].into_iter().enumerate() {
                let p = jaw + fwd * (t * 2.2);
                out.push(Prim::Line {
                    a: p,
                    b: p + down * (len * if k == 0 { 1.0 } else { 0.8 }),
                    w0: 0.9,
                    w1: 0.3,
                    paint: ivory,
                });
                if gape > 0.5 {
                    // Lower fangs show when the mouth is open.
                    let q = p + down * gape;
                    out.push(Prim::Line {
                        a: q,
                        b: q - down * (len * 0.7),
                        w0: 0.8,
                        w1: 0.3,
                        paint: ivory,
                    });
                }
            }
        }
        Teeth::Shark => {
            let len = (0.5 + s * 0.22).max(0.8);
            for i in 0..n + 1 {
                let t = i as f32 / n as f32;
                let p = jaw + fwd * (t * 2.6);
                out.push(Prim::Tri {
                    p: [p - fwd * 0.4, p + fwd * 0.4, p + down * len],
                    paint: ivory,
                });
                if gape > 0.5 {
                    let q = p + down * gape;
                    out.push(Prim::Tri {
                        p: [q - fwd * 0.4, q + fwd * 0.4, q - down * len * 0.8],
                        paint: ivory,
                    });
                }
            }
        }
        Teeth::Bristles => {
            for i in 0..n * 2 {
                let t = i as f32 / (n * 2) as f32;
                let p = jaw + fwd * (t * 2.6);
                out.push(Prim::Line {
                    a: p,
                    b: p + down * (0.6 + s * 0.15) + fwd * 0.2,
                    w0: 0.4,
                    w1: 0.3,
                    paint: ivory,
                });
            }
        }
    }
}
