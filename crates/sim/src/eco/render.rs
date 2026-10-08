//! CPU renderer for the ecosystem: composes sky, terrain, plants, creatures
//! and effects into one RGBA image at world resolution, then lights it with
//! the time of day, cave darkness and anything that glows.

use std::f32::consts::{PI, TAU};

use super::biome::Biome;
use super::body::{self, Paint, Prim};
use super::creature::{Carcass, Creature, Egg};
use super::flora::{FloraSpecies, GrowthForm, Plant};
use super::genome::{Colors, Pattern, Species, SpitKind};
use super::math::{Rgb, V2, mix, scale, v2};
use super::nav::{NavGrid, TILE};
use super::{Ecosystem, Icon, IconKind, Particle, Projectile};
use crate::material::{Kind, Material};
use crate::rng::hash2;
use crate::world::World;

#[derive(Clone, Debug, Default)]
pub struct DrawOpts {
    pub selected: Option<u32>,
    pub show_paths: bool,
}

/// Everything the renderer needs for one frame. The ecosystem sandbox and
/// the versus arena both build one of these.
pub struct Scene<'a> {
    pub seed: u64,
    pub biome: &'a Biome,
    pub world: &'a World,
    pub nav: &'a NavGrid,
    pub time: f32,
    pub tick: u64,
    /// Fraction of the day: 0 midnight, 0.25 dawn, 0.5 noon, 0.75 dusk.
    pub day_phase: f32,
    pub daylight: f32,
    pub dusk: f32,
    pub flora: &'a [FloraSpecies],
    pub plants: &'a [Plant],
    pub carcasses: &'a [Carcass],
    pub eggs: &'a [Egg],
    pub species: &'a [Species],
    pub creatures: &'a [Creature],
    pub projectiles: &'a [Projectile],
    pub particles: &'a [Particle],
    pub icons: &'a [Icon],
    /// Extra primitives drawn over the creatures (severed limbs, strike
    /// trails), with their colour and whether they glow.
    pub extras: &'a [(Prim, Rgb, bool)],
}

impl Scene<'_> {
    fn creature(&self, id: u32) -> Option<&Creature> {
        self.creatures.iter().find(|c| c.id == id)
    }
}

pub struct Renderer {
    pub w: usize,
    pub h: usize,
    pub rgba: Vec<u8>,
    col: Vec<Rgb>,
    sky: Vec<bool>,
    emissive: Vec<(u32, Rgb)>,
    emit: Vec<[f32; 3]>,
    emit_tmp: Vec<[f32; 3]>,
    amb: Vec<f32>,
    tw: usize,
    th: usize,
}

struct Target<'a> {
    w: i32,
    h: i32,
    col: &'a mut [Rgb],
    alpha: Option<&'a mut [u8]>,
    emissive: &'a mut Vec<(u32, Rgb)>,
    emit: &'a mut [[f32; 3]],
    tw: usize,
}

impl Target<'_> {
    #[inline]
    fn plot(&mut self, x: i32, y: i32, c: Rgb, glow: bool) {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            return;
        }
        let i = (y * self.w + x) as usize;
        if glow {
            self.emissive.push((i as u32, c));
            let t = (y as usize / TILE as usize) * self.tw + x as usize / TILE as usize;
            if let Some(e) = self.emit.get_mut(t) {
                for k in 0..3 {
                    e[k] += c[k] as f32 / 255.0 * 0.05;
                }
            }
        } else {
            self.col[i] = c;
        }
        if let Some(a) = self.alpha.as_deref_mut() {
            a[i] = 255;
        }
    }
}

fn blend(a: Rgb, b: Rgb, t: f32) -> Rgb {
    mix(a, b, t)
}

fn cell_colour(pal: &[Rgb], c: &crate::world::Cell) -> Rgb {
    let props = c.mat.props();
    let v = props.variance as i32;
    let off = if v == 0 {
        0
    } else {
        (c.shade as i32 * v) / 255 - v / 2
    };
    let base = pal[c.mat as usize];
    base.map(|b| (b as i32 + off).clamp(0, 255) as u8)
}

impl Renderer {
    pub fn new(w: usize, h: usize) -> Renderer {
        let tw = w / TILE as usize;
        let th = h / TILE as usize;
        Renderer {
            w,
            h,
            rgba: vec![0; w * h * 4],
            col: vec![[0; 3]; w * h],
            sky: vec![false; w * h],
            emissive: Vec::new(),
            emit: vec![[0.0; 3]; tw * th],
            emit_tmp: vec![[0.0; 3]; tw * th],
            amb: vec![0.0; tw * th],
            tw,
            th,
        }
    }

    pub fn draw(&mut self, eco: &Ecosystem, opts: &DrawOpts) {
        let scene = Scene {
            seed: eco.seed,
            biome: &eco.biome,
            world: &eco.world,
            nav: &eco.nav,
            time: eco.time,
            tick: eco.tick,
            day_phase: eco.day_phase(),
            daylight: eco.daylight(),
            dusk: eco.dusk(),
            flora: &eco.flora,
            plants: &eco.plants,
            carcasses: &eco.carcasses,
            eggs: &eco.eggs,
            species: &eco.species,
            creatures: &eco.creatures,
            projectiles: &eco.projectiles,
            particles: &eco.particles,
            icons: &eco.icons,
            extras: &[],
        };
        self.draw_scene(&scene, opts);
    }

    pub fn draw_scene(&mut self, eco: &Scene, opts: &DrawOpts) {
        let (w, h) = (self.w, self.h);
        self.emissive.clear();
        for e in self.emit.iter_mut() {
            *e = [0.0; 3];
        }
        let daylight = eco.daylight;
        let dusk = eco.dusk;
        let biome = eco.biome;
        let pal = &biome.palette;
        let phase = eco.day_phase;

        // ---- background --------------------------------------------------
        let sky_top = blend(
            blend(pal.sky_night[0], pal.sky_day[0], daylight),
            pal.sky_dusk[0],
            dusk * 0.7,
        );
        let sky_bot = blend(
            blend(pal.sky_night[1], pal.sky_day[1], daylight),
            pal.sky_dusk[1],
            dusk * 0.8,
        );
        let hills = scale(blend(pal.hills, pal.sky_night[1], 1.0 - daylight), 0.85);
        let far_hills = blend(hills, sky_bot, 0.45);
        for y in 0..h {
            let t = y as f32 / h as f32;
            let sky = blend(sky_top, sky_bot, t);
            for x in 0..w {
                let i = y * w + x;
                let below_surface = !biome.open_sky || (y as i32) > biome.surface[x] + 2;
                if below_surface {
                    let n = (hash2(eco.seed, x as i32 / 3, y as i32 / 3) % 12) as u8;
                    self.col[i] = pal.backdrop.map(|c| c.saturating_add(n));
                    self.sky[i] = false;
                } else {
                    let horizon = biome.horizon[x];
                    let far = horizon - 30 + ((x as f32 / 23.0).sin() * 8.0) as i32;
                    self.col[i] = if y as i32 > horizon {
                        hills
                    } else if y as i32 > far {
                        far_hills
                    } else {
                        sky
                    };
                    self.sky[i] = true;
                }
            }
        }
        if biome.open_sky {
            // Stars.
            let star = (1.0 - daylight * 1.5).clamp(0.0, 1.0);
            if star > 0.0 {
                for k in 0..260u64 {
                    let hsh = hash2(eco.seed ^ 0x57A2, k as i32, 3);
                    let (x, y) = ((hsh % w as u64) as usize, ((hsh >> 20) % (h as u64 / 2)) as usize);
                    let i = y * w + x;
                    if self.sky[i] {
                        let tw = 0.6 + 0.4 * ((eco.time * 2.0 + k as f32).sin());
                        self.col[i] = blend(self.col[i], [255, 250, 235], star * tw);
                    }
                }
            }
            // Sun and moons travel the sky.
            let arc = |p: f32| -> V2 {
                let t = (p - 0.25).rem_euclid(1.0) * 2.0;
                v2(t * w as f32, h as f32 * 0.55 - (t * PI).sin() * h as f32 * 0.48)
            };
            let sun = arc(phase);
            self.disc_sky(sun, 7.0, pal.sun, 0.9);
            for (k, (c, r)) in pal.moons.iter().enumerate() {
                let m = arc(phase + 0.5 + k as f32 * 0.13);
                self.disc_sky(m, *r * 0.6, *c, 0.8);
            }
        }

        // ---- terrain -------------------------------------------------------
        let tw = self.tw;
        let cells = eco.world.cells();
        let lava = pal.mat(Material::Lava);
        for (i, c) in cells.iter().enumerate() {
            if c.mat == Material::Empty {
                continue;
            }
            let x = i % w;
            let y = i / w;
            let t = (y / TILE as usize) * tw + x / TILE as usize;
            let mut rgb = cell_colour(&pal.mats, c);
            match c.mat.kind() {
                Kind::Fire => {
                    let life = c.life as f32 / Material::Fire.initial_life() as f32;
                    let fire = [255, (90.0 + life * 150.0) as u8, (20.0 + life * 40.0) as u8];
                    self.emissive.push((i as u32, fire));
                    let e = &mut self.emit[t];
                    e[0] += 0.09;
                    e[1] += 0.05;
                    e[2] += 0.01;
                    continue;
                }
                Kind::Gas => {
                    let a = (c.life as f32 / c.mat.initial_life().max(1) as f32) * 0.6;
                    rgb = blend(self.col[i], rgb, a);
                }
                Kind::Liquid if c.mat == Material::Lava => {
                    let flicker =
                        ((hash2(c.shade as u64, x as i32, (eco.tick / 8) as i32) % 40) as i32 - 20) as f32;
                    let l = lava.map(|v| (v as f32 + flicker).clamp(0.0, 255.0) as u8);
                    self.emissive.push((i as u32, l));
                    let e = &mut self.emit[t];
                    e[0] += lava[0] as f32 / 255.0 * 0.06;
                    e[1] += lava[1] as f32 / 255.0 * 0.06;
                    e[2] += lava[2] as f32 / 255.0 * 0.06;
                    continue;
                }
                Kind::Liquid => {
                    rgb = blend(self.col[i], rgb, 0.82);
                    if c.mat == Material::Acid {
                        let e = &mut self.emit[t];
                        for k in 0..3 {
                            e[k] += rgb[k] as f32 / 255.0 * 0.006;
                        }
                    }
                }
                _ if c.mat == Material::Web => rgb = blend(self.col[i], rgb, 0.7),
                _ => {}
            }
            self.col[i] = rgb;
            self.sky[i] = false;
        }

        let mut target = Target {
            w: w as i32,
            h: h as i32,
            col: &mut self.col,
            alpha: None,
            emissive: &mut self.emissive,
            emit: &mut self.emit,
            tw,
        };

        // ---- life ----------------------------------------------------------
        for p in eco.plants {
            draw_plant(&mut target, p, &eco.flora[p.flora], eco.time, eco.world);
        }
        for k in eco.carcasses {
            let sp = &eco.species[k.species];
            let s = k.scale * sp.stats.size;
            let rot = (k.age / 60.0).min(1.0);
            let flesh = blend(sp.colors.base, [70, 50, 45], rot * 0.7);
            let frac = k.meat / k.max_meat.max(0.01);
            let len = s * 1.3 + sp.body.segs.len() as f32 * 0.6;
            raster_ellipse(
                &mut target,
                k.pos - v2(0.0, s * 0.3),
                len * frac.max(0.4),
                (s * 0.45).max(1.0),
                0.0,
                |_, _, edge| (if edge { scale(flesh, 0.5) } else { flesh }, false),
            );
            if frac < 0.7 {
                for r in 0..(len as i32 / 2).max(2) {
                    let x = k.pos.x - len * 0.7 + r as f32 * 2.0;
                    raster_line(
                        &mut target,
                        v2(x, k.pos.y - s * 0.7),
                        v2(x + 0.5, k.pos.y),
                        0.8,
                        0.8,
                        [225, 220, 205],
                        false,
                    );
                }
            }
        }
        for e in eco.eggs {
            let glow = eco.species[e.species].colors.glow;
            raster_ellipse(&mut target, e.pos - v2(0.0, 1.5), 1.3, 1.8, 0.0, |u, v, edge| {
                let speck = hash2(e.id as u64, (u * 2.0) as i32, (v * 2.0) as i32).is_multiple_of(5);
                (
                    if edge {
                        scale(e.color, 0.6)
                    } else if speck {
                        glow.unwrap_or(scale(e.color, 0.7))
                    } else {
                        e.color
                    },
                    speck && glow.is_some(),
                )
            });
        }
        for c in eco.creatures {
            let sp = &eco.species[c.species];
            draw_creature(&mut target, c, sp);
        }
        for (prim, col, glow) in eco.extras {
            draw_prim(&mut target, prim, *col, *glow);
        }
        for p in eco.projectiles {
            let (col, glow) = match p.kind {
                SpitKind::Acid => (pal.mat(Material::Acid), true),
                SpitKind::Fire => ([255, 170, 50], true),
                SpitKind::Venom => ([190, 70, 230], true),
                SpitKind::Web => ([235, 235, 245], false),
            };
            raster_dot(&mut target, p.pos, 1.3, col, glow);
            raster_line(&mut target, p.pos, p.pos - p.vel * 0.02, 1.0, 0.6, col, glow);
        }
        for p in eco.particles {
            let a = (p.life / p.max_life).clamp(0.0, 1.0);
            let (x, y) = p.pos.cell();
            if p.glow {
                target.plot(x, y, scale(p.color, a), true);
            } else if (x as usize) < w && (y as usize) < h && x >= 0 && y >= 0 {
                let i = y as usize * w + x as usize;
                target.col[i] = blend(target.col[i], p.color, a);
            }
        }

        // ---- lighting ------------------------------------------------------
        self.compute_light(eco, daylight);
        let night_tint = [0.45, 0.55, 1.0];
        let dusk_tint = [1.0, 0.72, 0.55];
        let mut tint = [0.0f32; 3];
        for k in 0..3 {
            let base = night_tint[k] + (1.0 - night_tint[k]) * daylight;
            tint[k] = base + (dusk_tint[k] - base) * dusk * 0.6;
        }
        let (tw, th) = (self.tw, self.th);
        let light_at = |amb: &[f32], emit: &[[f32; 3]], x: usize, y: usize| -> [f32; 3] {
            let fx = (x as f32 + 0.5) / TILE as f32 - 0.5;
            let fy = (y as f32 + 0.5) / TILE as f32 - 0.5;
            let x0 = (fx.floor().max(0.0) as usize).min(tw - 1);
            let y0 = (fy.floor().max(0.0) as usize).min(th - 1);
            let x1 = (x0 + 1).min(tw - 1);
            let y1 = (y0 + 1).min(th - 1);
            let (ax, ay) = ((fx - x0 as f32).clamp(0.0, 1.0), (fy - y0 as f32).clamp(0.0, 1.0));
            let s = |i: usize| -> [f32; 3] {
                let a = amb[i];
                let e = emit[i];
                [a * tint[0] + e[0], a * tint[1] + e[1], a * tint[2] + e[2]]
            };
            let (a, b, c, d) = (s(y0 * tw + x0), s(y0 * tw + x1), s(y1 * tw + x0), s(y1 * tw + x1));
            let mut out = [0.0; 3];
            for k in 0..3 {
                let top = a[k] + (b[k] - a[k]) * ax;
                let bot = c[k] + (d[k] - c[k]) * ax;
                out[k] = top + (bot - top) * ay;
            }
            out
        };
        for y in 0..h {
            for x in 0..w {
                let i = y * w + x;
                let c = self.col[i];
                let rgb = if self.sky[i] {
                    c
                } else {
                    let l = light_at(&self.amb, &self.emit, x, y);
                    [
                        (c[0] as f32 * l[0].min(1.5)).min(255.0) as u8,
                        (c[1] as f32 * l[1].min(1.5)).min(255.0) as u8,
                        (c[2] as f32 * l[2].min(1.5)).min(255.0) as u8,
                    ]
                };
                self.rgba[i * 4..i * 4 + 4].copy_from_slice(&[rgb[0], rgb[1], rgb[2], 255]);
            }
        }
        for &(i, c) in &self.emissive {
            let i = i as usize * 4;
            self.rgba[i..i + 3].copy_from_slice(&c);
        }

        // ---- overlays ------------------------------------------------------
        for icon in eco.icons {
            self.icon(icon.pos, icon.kind, (icon.life * 2.0).min(1.0));
        }
        if let Some(id) = opts.selected
            && let Some(c) = eco.creature(id)
        {
            let sp = &eco.species[c.species];
            let r = sp.stats.size * c.scale(sp) * 1.6 + 5.0;
            let steps = (r * 6.0) as i32;
            for k in 0..steps {
                if (k / 3) % 2 == 0 {
                    let a = k as f32 / steps as f32 * TAU + eco.time;
                    let p = c.pos + V2::from_angle(a) * r;
                    self.put(p, [255, 255, 255]);
                }
            }
            if opts.show_paths {
                for w in &c.brain.path {
                    self.put(*w, [255, 230, 60]);
                }
                if let Some(s) = c.brain.spot {
                    self.put(s, [80, 255, 120]);
                    self.put(s + v2(1.0, 0.0), [80, 255, 120]);
                }
            }
        }
    }

    fn put(&mut self, p: V2, c: Rgb) {
        let (x, y) = p.cell();
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            let i = (y as usize * self.w + x as usize) * 4;
            self.rgba[i..i + 3].copy_from_slice(&c);
        }
    }

    fn disc_sky(&mut self, c: V2, r: f32, col: Rgb, halo: f32) {
        let rr = (r * 3.0) as i32;
        for dy in -rr..=rr {
            for dx in -rr..=rr {
                let (x, y) = (c.x as i32 + dx, c.y as i32 + dy);
                if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h {
                    continue;
                }
                let i = y as usize * self.w + x as usize;
                if !self.sky[i] {
                    continue;
                }
                let d = ((dx * dx + dy * dy) as f32).sqrt();
                if d <= r {
                    self.col[i] = col;
                } else {
                    let a = (1.0 - (d - r) / (r * 2.0)).clamp(0.0, 1.0) * 0.35 * halo;
                    self.col[i] = blend(self.col[i], col, a);
                }
            }
        }
    }

    fn compute_light(&mut self, eco: &Scene, daylight: f32) {
        let (tw, th) = (self.tw, self.th);
        let sky = 0.16 + 0.84 * daylight;
        let cave = eco.biome.cave_ambient;
        // Terrain never goes fully black; it just dims with depth and night.
        let floor = if eco.biome.open_sky {
            0.3 + 0.12 * daylight
        } else {
            0.0
        };
        for tx in 0..tw {
            let mut l: f32 = if eco.biome.open_sky { 1.0 } else { 0.0 };
            for ty in 0..th {
                let t = eco.nav.tile(tx as i32, ty as i32).copied().unwrap_or_default();
                self.amb[ty * tw + tx] = (l * sky).max(floor) + cave;
                if !t.open() {
                    l *= 0.55;
                } else if t.wet() {
                    l *= 0.88;
                } else {
                    l *= 0.995;
                }
            }
        }
        // Soften light falling into cave mouths.
        let mut tmp = self.amb.clone();
        for ty in 0..th {
            for tx in 0..tw {
                let mut s = 0.0;
                let mut n = 0.0;
                for dx in -2i32..=2 {
                    let x = tx as i32 + dx;
                    if x >= 0 && (x as usize) < tw {
                        s += self.amb[ty * tw + x as usize];
                        n += 1.0;
                    }
                }
                tmp[ty * tw + tx] = s / n;
            }
        }
        self.amb = tmp;
        // Spread emitted light.
        for _ in 0..2 {
            for ty in 0..th {
                for tx in 0..tw {
                    let mut s = [0.0; 3];
                    for dx in -3i32..=3 {
                        let x = tx as i32 + dx;
                        if x >= 0 && (x as usize) < tw {
                            let e = self.emit[ty * tw + x as usize];
                            for k in 0..3 {
                                s[k] += e[k];
                            }
                        }
                    }
                    self.emit_tmp[ty * tw + tx] = s.map(|v| v / 4.0);
                }
            }
            for ty in 0..th {
                for tx in 0..tw {
                    let mut s = [0.0; 3];
                    for dy in -3i32..=3 {
                        let y = ty as i32 + dy;
                        if y >= 0 && (y as usize) < th {
                            let e = self.emit_tmp[y as usize * tw + tx];
                            for k in 0..3 {
                                s[k] += e[k];
                            }
                        }
                    }
                    self.emit[ty * tw + tx] = s.map(|v| (v / 4.0).min(1.2));
                }
            }
        }
    }

    fn icon(&mut self, at: V2, kind: IconKind, a: f32) {
        let (rows, col): (&[u8], Rgb) = match kind {
            IconKind::Alert => (&[0b00100, 0b00100, 0b00100, 0b00000, 0b00100], [255, 225, 60]),
            IconKind::Sleep => (&[0b11110, 0b00100, 0b01000, 0b11110, 0b00000], [160, 200, 255]),
            IconKind::Heart => (&[0b01010, 0b11111, 0b11111, 0b01110, 0b00100], [255, 110, 160]),
            IconKind::Skull => (&[0b01110, 0b10101, 0b11111, 0b01110, 0b01010], [235, 235, 225]),
            IconKind::Food => (&[0b00100, 0b01110, 0b11111, 0b01110, 0b00000], [120, 230, 110]),
        };
        let (ox, oy) = (at.x as i32 - 2, at.y as i32 - 8);
        for (ry, bits) in rows.iter().enumerate() {
            for rx in 0..5 {
                if bits & (0b10000 >> rx) != 0 {
                    let (x, y) = (ox + rx, oy + ry as i32);
                    if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
                        let i = (y as usize * self.w + x as usize) * 4;
                        let cur = [self.rgba[i], self.rgba[i + 1], self.rgba[i + 2]];
                        self.rgba[i..i + 3].copy_from_slice(&blend(cur, col, a));
                    }
                }
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Rasterisation

fn raster_ellipse(
    t: &mut Target,
    c: V2,
    rx: f32,
    ry: f32,
    angle: f32,
    mut paint: impl FnMut(f32, f32, bool) -> (Rgb, bool),
) {
    let rx = rx.max(0.6);
    let ry = ry.max(0.6);
    let r = rx.max(ry) + 1.0;
    let (sa, ca) = (-angle).sin_cos();
    let min_r = rx.min(ry);
    let edge_t = if min_r < 1.6 {
        2.0
    } else {
        ((min_r - 1.0) / min_r).powi(2)
    };
    for y in (c.y - r).floor() as i32..=(c.y + r).ceil() as i32 {
        for x in (c.x - r).floor() as i32..=(c.x + r).ceil() as i32 {
            let (dx, dy) = (x as f32 + 0.5 - c.x, y as f32 + 0.5 - c.y);
            let u = dx * ca - dy * sa;
            let v = dx * sa + dy * ca;
            let d = (u / rx).powi(2) + (v / ry).powi(2);
            if d <= 1.0 {
                let (col, glow) = paint(u, v, d > edge_t);
                t.plot(x, y, col, glow);
            }
        }
    }
}

fn raster_line(t: &mut Target, a: V2, b: V2, w0: f32, w1: f32, col: Rgb, glow: bool) {
    let r = w0.max(w1) / 2.0 + 0.5;
    let (x0, x1) = (
        (a.x.min(b.x) - r).floor() as i32,
        (a.x.max(b.x) + r).ceil() as i32,
    );
    let (y0, y1) = (
        (a.y.min(b.y) - r).floor() as i32,
        (a.y.max(b.y) + r).ceil() as i32,
    );
    let ab = b - a;
    let l2 = ab.len2().max(1e-4);
    if (x1 - x0) * (y1 - y0) > 4000 {
        return;
    }
    for y in y0..=y1 {
        for x in x0..=x1 {
            let p = v2(x as f32 + 0.5, y as f32 + 0.5);
            let tt = ((p - a).dot(ab) / l2).clamp(0.0, 1.0);
            let d = p.dist(a + ab * tt);
            let w = (w0 + (w1 - w0) * tt) / 2.0;
            if d <= w.max(0.5) {
                t.plot(x, y, col, glow);
            }
        }
    }
}

fn raster_tri(t: &mut Target, p: [V2; 3], col: Rgb, glow: bool) {
    let x0 = p.iter().map(|q| q.x).fold(f32::MAX, f32::min).floor() as i32;
    let x1 = p.iter().map(|q| q.x).fold(f32::MIN, f32::max).ceil() as i32;
    let y0 = p.iter().map(|q| q.y).fold(f32::MAX, f32::min).floor() as i32;
    let y1 = p.iter().map(|q| q.y).fold(f32::MIN, f32::max).ceil() as i32;
    if (x1 - x0) * (y1 - y0) > 4000 {
        return;
    }
    let edge = |a: V2, b: V2, c: V2| (b.x - a.x) * (c.y - a.y) - (b.y - a.y) * (c.x - a.x);
    let area = edge(p[0], p[1], p[2]);
    if area.abs() < 0.3 {
        raster_line(t, p[0], p[1], 1.0, 1.0, col, glow);
        return;
    }
    for y in y0..=y1 {
        for x in x0..=x1 {
            let q = v2(x as f32 + 0.5, y as f32 + 0.5);
            let w0 = edge(p[1], p[2], q) / area;
            let w1 = edge(p[2], p[0], q) / area;
            let w2 = edge(p[0], p[1], q) / area;
            if w0 >= -0.02 && w1 >= -0.02 && w2 >= -0.02 {
                t.plot(x, y, col, glow);
            }
        }
    }
}

fn raster_dot(t: &mut Target, p: V2, r: f32, col: Rgb, glow: bool) {
    if r < 0.8 {
        let (x, y) = p.cell();
        t.plot(x, y, col, glow);
        return;
    }
    let ri = r.ceil() as i32;
    for dy in -ri..=ri {
        for dx in -ri..=ri {
            let q = v2(p.x.floor() + dx as f32 + 0.5, p.y.floor() + dy as f32 + 0.5);
            if q.dist(p) <= r {
                t.plot(q.x as i32, q.y as i32, col, glow);
            }
        }
    }
}

fn pattern(c: &Colors, u: f32, v: f32, rx: f32, ry: f32, seed: u64) -> Rgb {
    let belly = v > ry * 0.35;
    let k = c.pattern_scale;
    let accent = match c.pattern {
        Pattern::Solid => false,
        Pattern::Stripes => ((u / k).floor() as i32).rem_euclid(2) == 0 && !belly,
        Pattern::Bands => ((u / (k * 2.0)).floor() as i32).rem_euclid(2) == 0 && v < 0.0,
        Pattern::Spots => {
            let (cx, cy) = ((u / k).floor() as i32, (v / k).floor() as i32);
            let h = hash2(seed, cx, cy);
            let du = u - (cx as f32 + 0.5) * k;
            let dv = v - (cy as f32 + 0.5) * k;
            h.is_multiple_of(3) && du * du + dv * dv < (k * 0.35).powi(2) * 1.5 && !belly
        }
        Pattern::Gradient => return mix(c.base, c.accent, (u / rx * 0.5 + 0.5).clamp(0.0, 1.0)),
        Pattern::Eyespots => {
            let e = v2(u.abs() - rx * 0.4, v + ry * 0.2);
            let d = e.len();
            if d < ry * 0.18 {
                return [15, 15, 20];
            }
            d < ry * 0.38
        }
        Pattern::Speckled => hash2(seed, (u * 2.0) as i32, (v * 2.0) as i32).is_multiple_of(4),
        Pattern::Chevron => (((u + v.abs()) / k).floor() as i32).rem_euclid(2) == 0 && !belly,
    };
    if accent {
        c.accent
    } else if belly {
        c.belly
    } else {
        c.base
    }
}

/// Draws one primitive in a flat colour.
fn draw_prim(t: &mut Target, prim: &Prim, col: Rgb, glow: bool) {
    match *prim {
        Prim::Ellipse { c, rx, ry, angle, .. } => raster_ellipse(t, c, rx, ry, angle, |_, _, _| (col, glow)),
        Prim::Line { a, b, w0, w1, .. } => raster_line(t, a, b, w0, w1, col, glow),
        Prim::Tri { p, .. } => raster_tri(t, p, col, glow),
        Prim::Dot { p, r, .. } => raster_dot(t, p, r, col, glow),
    }
}

fn draw_creature(t: &mut Target, c: &Creature, sp: &Species) {
    let prims = body::pose(c, sp);
    let colors = &sp.colors;
    let hurt = c.anim.hurt > 0.0;
    let dark = scale(colors.base, 0.5);
    let far = scale(colors.base, 0.74);
    let limb = mix(colors.base, colors.accent, 0.35);
    let poisoned = c.status.poison > 0.0;
    let adjust = |rgb: Rgb| -> Rgb {
        let mut rgb = rgb;
        if poisoned {
            rgb = mix(rgb, [120, 200, 80], 0.25);
        }
        if hurt { mix(rgb, [255, 255, 255], 0.6) } else { rgb }
    };
    let resolve = |p: Paint| -> (Rgb, bool) {
        match p {
            Paint::Skin | Paint::Base => (adjust(colors.base), false),
            Paint::Belly => (adjust(colors.belly), false),
            Paint::Accent => (adjust(colors.accent), false),
            Paint::Dark => (dark, false),
            Paint::Eye => (colors.eye, colors.glow.is_some() || sp.traits.night_vision),
            Paint::Glow => (colors.glow.unwrap_or(colors.accent), true),
            Paint::Far => (adjust(far), false),
            Paint::Limb => (adjust(limb), false),
            Paint::Fixed(c) => (c, false),
        }
    };
    let seed = c.species as u64 * 977 + 13;
    for prim in &prims {
        match *prim {
            Prim::Ellipse {
                c: ctr,
                rx,
                ry,
                angle,
                paint,
                outline,
            } => {
                if paint == Paint::Skin {
                    raster_ellipse(t, ctr, rx, ry, angle, |u, v, edge| {
                        if edge && outline {
                            (dark, false)
                        } else {
                            let mut col = pattern(colors, u * c.facing, v, rx, ry, seed);
                            if v < -ry * 0.55 {
                                col = scale(col, 1.12);
                            }
                            (adjust(col), false)
                        }
                    });
                } else {
                    let (col, glow) = resolve(paint);
                    raster_ellipse(t, ctr, rx, ry, angle, |_, _, edge| {
                        if edge && outline {
                            (dark, false)
                        } else {
                            (col, glow)
                        }
                    });
                }
            }
            Prim::Line { a, b, w0, w1, paint } => {
                let (col, glow) = resolve(paint);
                // Limbs get a dark outline so they read against the body.
                if matches!(paint, Paint::Limb | Paint::Skin | Paint::Far) {
                    raster_line(t, a, b, w0 + 1.0, w1 + 1.0, dark, false);
                }
                raster_line(t, a, b, w0, w1, col, glow);
            }
            Prim::Tri { p, paint } => {
                let (col, glow) = resolve(paint);
                raster_tri(t, p, col, glow);
            }
            Prim::Dot { p, r, paint } => {
                let (col, glow) = resolve(paint);
                raster_dot(t, p, r, col, glow);
            }
        }
    }
}

fn draw_plant(t: &mut Target, p: &Plant, fl: &FloraSpecies, time: f32, world: &World) {
    let base = p.base();
    let len = p.length(fl);
    let dir = p.dir as f32;
    let wet = world.material(base.x as i32, base.y as i32 - 1).kind() == Kind::Liquid;
    let sway_amp = if wet { 0.25 } else { 0.07 };
    let sway = (time * if wet { 1.2 } else { 1.7 } + (p.seed % 100) as f32 * 0.1).sin() * sway_amp;
    let tipd = v2(0.0, dir).rot(sway * -dir);
    let tip = base + tipd * len;
    let stem = if p.bitten > 0.0 {
        mix(fl.stem, [255, 255, 255], 0.3)
    } else {
        fl.stem
    };
    let leaf = fl.leaf;
    let glow = fl.glow.is_some();
    let gcol = fl.glow.unwrap_or(fl.fruit);
    let fruit_n = p.fruit.floor() as i32;
    let h = |k: u32| hash2(p.seed as u64, k as i32, 7);
    match fl.form {
        GrowthForm::Stalk => {
            raster_line(t, base, tip, 1.4, 0.8, stem, false);
            let mut k = 2.0;
            let mut side = 1.0;
            while k < len - 1.0 {
                let a = base + tipd * k;
                raster_line(
                    t,
                    a,
                    a + v2(side * 2.5, -(-1.5 * -dir)).rot(sway),
                    1.0,
                    0.6,
                    leaf,
                    false,
                );
                side = -side;
                k += 3.0;
            }
            for i in 0..fruit_n {
                let q = tip + v2((i as f32 - fruit_n as f32 / 2.0) * 1.6, 0.6 * i as f32 % 2.0);
                raster_dot(t, q, 1.0, if glow { gcol } else { fl.fruit }, glow);
            }
        }
        GrowthForm::Bush => {
            let r = (len * 0.55).max(1.5);
            for k in 0..5u32 {
                let o = v2(
                    ((h(k) % 100) as f32 / 50.0 - 1.0) * r * 0.7,
                    -(((h(k + 9) % 100) as f32 / 100.0) * r),
                );
                raster_dot(
                    t,
                    base + o + v2(sway * 3.0, -r * 0.5),
                    r * 0.6,
                    if k % 2 == 0 { leaf } else { scale(leaf, 0.82) },
                    false,
                );
            }
            for i in 0..fruit_n {
                let o = v2(
                    ((h(i as u32 + 20) % 100) as f32 / 50.0 - 1.0) * r,
                    -(((h(i as u32 + 30) % 100) as f32 / 100.0) * r * 1.4),
                );
                raster_dot(t, base + o, 0.9, if glow { gcol } else { fl.fruit }, glow);
            }
        }
        GrowthForm::Tuft => {
            for k in 0..5 {
                let a = (k as f32 - 2.0) * 0.3 + sway;
                let end = base + v2(0.0, -len).rot(a);
                raster_line(
                    t,
                    base,
                    end,
                    1.0,
                    0.6,
                    if k % 2 == 0 { leaf } else { stem },
                    false,
                );
                if k < fruit_n {
                    raster_dot(t, end, 0.7, if glow { gcol } else { fl.fruit }, glow);
                }
            }
        }
        GrowthForm::Bulb => {
            raster_line(t, base, base + tipd * (len * 0.4), 1.2, 1.0, stem, false);
            let r = len * 0.3 + p.fruit * 0.4;
            raster_ellipse(
                t,
                base + tipd * (len * 0.4 + r * 0.8),
                r * 0.85,
                r,
                0.0,
                |u, v, edge| {
                    let ribs = ((u * 1.3).floor() as i32).rem_euclid(2) == 0;
                    (
                        if edge {
                            scale(leaf, 0.6)
                        } else if ribs {
                            leaf
                        } else {
                            mix(leaf, fl.fruit, (p.fruit / fl.max_fruit as f32) * (0.6 - v * 0.05))
                        },
                        glow && !edge && !ribs,
                    )
                },
            );
        }
        GrowthForm::Vine | GrowthForm::Kelp => {
            let n = (len / 2.0).max(2.0) as i32;
            let mut prev = base;
            for i in 1..=n {
                let f = i as f32 / n as f32;
                let wave = (time * if wet { 1.3 } else { 2.0 } + f * 4.0 + (p.seed % 50) as f32).sin()
                    * f
                    * if wet { 3.0 } else { 1.5 };
                let q = base + v2(wave, dir * len * f);
                raster_line(t, prev, q, 1.2, 1.0, if i % 3 == 0 { leaf } else { stem }, false);
                if fl.form == GrowthForm::Kelp && i % 2 == 0 {
                    raster_line(
                        t,
                        q,
                        q + v2(2.0 * if i % 4 == 0 { 1.0 } else { -1.0 }, -1.0),
                        1.0,
                        0.6,
                        leaf,
                        false,
                    );
                }
                if (i * 2 % n.max(1)) < fruit_n.max(0) * 2 && i > n / 3 {
                    raster_dot(t, q, 0.9, if glow { gcol } else { fl.fruit }, glow);
                }
                prev = q;
            }
        }
        GrowthForm::Crystal => {
            for k in 0..4u32 {
                let a = (k as f32 - 1.5) * 0.35 + ((h(k) % 10) as f32 - 5.0) * 0.03;
                let l = len * (0.5 + (h(k + 4) % 50) as f32 / 100.0);
                let d = v2(0.0, -1.0).rot(a);
                let side = d.perp() * 1.2;
                raster_tri(
                    t,
                    [base + side, base - side, base + d * l],
                    if k % 2 == 0 { leaf } else { gcol },
                    k % 2 == 1 || glow,
                );
            }
        }
        GrowthForm::Cap => {
            raster_line(t, base, tip, 1.6, 1.4, stem, false);
            let r = len * 0.45;
            raster_ellipse(t, tip, r, r * 0.45, sway, |u, v, edge| {
                if v > r * 0.1 {
                    (scale(leaf, 0.7), false)
                } else if edge {
                    (scale(leaf, 0.6), false)
                } else {
                    let spot = hash2(p.seed as u64, (u * 0.6) as i32, (v * 0.8) as i32).is_multiple_of(4);
                    (if spot { gcol } else { leaf }, spot && glow)
                }
            });
        }
        GrowthForm::Pod => {
            raster_line(t, base, base + tipd * (len * 0.4), 1.0, 0.8, stem, false);
            let r = (len * 0.25 + p.fruit * 0.3).max(1.0);
            raster_ellipse(
                t,
                base + tipd * (len * 0.4 + r),
                r * 0.75,
                r,
                sway,
                |_, _, edge| {
                    (
                        if edge {
                            scale(fl.fruit, 0.6)
                        } else {
                            mix(leaf, fl.fruit, p.fruit / fl.max_fruit as f32)
                        },
                        glow && !edge,
                    )
                },
            );
        }
        GrowthForm::Reed => {
            for k in 0..3 {
                let off = v2((k as f32 - 1.0) * 1.5, 0.0);
                let l = len * (0.75 + 0.12 * k as f32);
                let end = base + off + v2(0.0, -l).rot(sway * (1.0 + k as f32 * 0.3));
                raster_line(t, base + off, end, 1.0, 0.8, stem, false);
                if k < fruit_n.max(1) {
                    raster_ellipse(t, end, 0.9, 2.2, sway, |_, _, _| {
                        (if glow { gcol } else { fl.fruit }, glow)
                    });
                }
            }
        }
    }
}

/// A standalone picture of a species for the UI, RGBA with a transparent background.
pub fn portrait(sp: &Species) -> (usize, usize, Vec<u8>) {
    let len = sp.body.length() * 1.2 + sp.body.limbs.iter().map(|l| l.length).fold(0.0, f32::max) * 2.0;
    let w = ((len + 24.0) as usize).clamp(48, 220) & !3;
    let tall = (sp.body.top + sp.body.stance + sp.body.sac * 2.5) * 1.3 + sp.stats.size * 4.0;
    let h = ((tall + 20.0) as usize).clamp(40, 160) & !3;
    let ww = w.div_ceil(64) * 64;
    let hh = h.div_ceil(64) * 64;
    let mut world = World::new(ww, hh, 1);
    let floor = h as i32 - 6;
    for x in 0..w as i32 {
        for y in floor..hh as i32 {
            world.set(x, y, Material::Stone);
        }
    }
    let mut c = Creature::new(1, sp, V2::ZERO, true, 0);
    let b = super::physics::bounds(&c, sp);
    let airborne = sp.traits.fly && sp.loco != super::genome::Locomotion::Walk
        || sp.loco == super::genome::Locomotion::Swim;
    let body_len = sp.body.segs.len() as f32 * sp.body.spacing;
    c.pos = v2(
        (w as f32 / 2.0 + body_len * 0.4).min(w as f32 - b.hw - 4.0),
        if airborne {
            h as f32 * 0.5
        } else {
            floor as f32 - b.bottom - 0.1
        },
    );
    c.facing = 1.0;
    c.on_ground = !airborne;
    c.flying = airborne && sp.traits.fly;
    c.in_liquid = sp.loco == super::genome::Locomotion::Swim;
    super::creature::init_anim(&mut c, sp);
    for i in 0..90 {
        c.anim.flap = 0.22;
        let flying = c.flying;
        body::animate(&mut c, sp, &world, 1.0 / 60.0, flying);
        // Lay rope bodies out straight behind the head.
        if i < 2 {
            for (k, s) in c.anim.segs.iter_mut().enumerate() {
                *s = c.pos - v2(k as f32 * sp.body.spacing, 0.0);
            }
        }
        c.anim.blink = 1.0;
    }
    let mut col = vec![[0u8; 3]; ww * hh];
    let mut alpha = vec![0u8; ww * hh];
    let mut emissive = Vec::new();
    let mut emit = vec![[0.0; 3]; 1];
    let mut t = Target {
        w: ww as i32,
        h: hh as i32,
        col: &mut col,
        alpha: Some(&mut alpha),
        emissive: &mut emissive,
        emit: &mut emit,
        tw: 0,
    };
    draw_creature(&mut t, &c, sp);
    for &(i, c) in &emissive {
        col[i as usize] = c;
    }
    // Crop to the creature with a small margin.
    let (mut x0, mut y0, mut x1, mut y1) = (w, h, 0, 0);
    for y in 0..h {
        for x in 0..w {
            if alpha[y * ww + x] > 0 {
                x0 = x0.min(x);
                y0 = y0.min(y);
                x1 = x1.max(x);
                y1 = y1.max(y);
            }
        }
    }
    if x1 < x0 {
        return (1, 1, vec![0; 4]);
    }
    let (x0, y0) = (x0.saturating_sub(2), y0.saturating_sub(2));
    let (x1, y1) = ((x1 + 2).min(w - 1), (y1 + 2).min(h - 1));
    let (cw, ch) = (x1 - x0 + 1, y1 - y0 + 1);
    let mut out = vec![0u8; cw * ch * 4];
    for y in 0..ch {
        for x in 0..cw {
            let i = (y + y0) * ww + x + x0;
            let o = (y * cw + x) * 4;
            let [r, g, b] = col[i];
            out[o..o + 4].copy_from_slice(&[r, g, b, alpha[i]]);
        }
    }
    (cw, ch, out)
}
