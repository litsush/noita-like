//! Procedural plant sprites. Every species, including every hybrid the Gene
//! Splicer makes, is drawn from its appearance genes: stem height and curve,
//! leaf style and density, bloom style, two hues and glow. Hybrids therefore
//! look like their parents, and no two species look the same.

use std::collections::HashMap;

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use sbct_sim::colony::plants::{Looks, Species, SpeciesId};
use sbct_sim::rng::hash2;

pub const FRAME_W: usize = 32;
pub const FRAME_H: usize = 48;
/// Growth stages drawn per species, from sprout to mature.
pub const STAGES: usize = 6;

type Rgba = [u8; 4];

struct Canvas {
    px: Vec<Rgba>,
}

impl Canvas {
    fn new() -> Canvas {
        Canvas {
            px: vec![[0; 4]; FRAME_W * FRAME_H],
        }
    }

    fn set(&mut self, x: i32, y: i32, c: Rgba) {
        if x >= 0 && y >= 0 && (x as usize) < FRAME_W && (y as usize) < FRAME_H {
            self.px[y as usize * FRAME_W + x as usize] = c;
        }
    }

    fn get(&self, x: i32, y: i32) -> Rgba {
        if x >= 0 && y >= 0 && (x as usize) < FRAME_W && (y as usize) < FRAME_H {
            self.px[y as usize * FRAME_W + x as usize]
        } else {
            [0; 4]
        }
    }

    fn disc(&mut self, cx: f32, cy: f32, r: f32, c: Rgba) {
        let ri = r.ceil() as i32;
        for dy in -ri..=ri {
            for dx in -ri..=ri {
                if (dx * dx + dy * dy) as f32 <= r * r + 0.3 {
                    self.set(cx.round() as i32 + dx, cy.round() as i32 + dy, c);
                }
            }
        }
    }

    fn line(&mut self, x0: f32, y0: f32, x1: f32, y1: f32, c: Rgba) {
        let steps = (x1 - x0).abs().max((y1 - y0).abs()).ceil().max(1.0) as i32;
        for i in 0..=steps {
            let t = i as f32 / steps as f32;
            self.set(
                (x0 + (x1 - x0) * t).round() as i32,
                (y0 + (y1 - y0) * t).round() as i32,
                c,
            );
        }
    }

    /// A dark outline around everything drawn, as the rest of the art has.
    fn outline(&mut self, c: Rgba) {
        let src = self.px.clone();
        let at = |x: i32, y: i32| {
            x >= 0
                && y >= 0
                && (x as usize) < FRAME_W
                && (y as usize) < FRAME_H
                && src[y as usize * FRAME_W + x as usize][3] > 120
        };
        for y in 0..FRAME_H as i32 {
            for x in 0..FRAME_W as i32 {
                if !at(x, y) && (at(x + 1, y) || at(x - 1, y) || at(x, y + 1) || at(x, y - 1)) {
                    self.set(x, y, c);
                }
            }
        }
    }
}

/// HSV to RGB with hue on the 0–255 wheel.
pub fn hsv(hue: u8, sat: f32, val: f32) -> [u8; 3] {
    let h = hue as f32 / 255.0 * 6.0;
    let c = val * sat;
    let x = c * (1.0 - (h % 2.0 - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = val - c;
    [
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    ]
}

fn rgba(c: [u8; 3]) -> Rgba {
    [c[0], c[1], c[2], 255]
}

/// Draws one growth stage (0 = sprout … 1 = mature) of a plant standing on
/// the bottom-centre of the frame.
fn draw(looks: &Looks, growth: f32, seed: u64) -> Canvas {
    let mut cv = Canvas::new();
    let glow = looks.glow as f32 / 10.0;
    // Muted, slightly teal greens; glowing plants are more saturated.
    let leaf_dark = rgba(hsv(looks.hue_leaf, 0.55 + glow * 0.2, 0.34 + glow * 0.1));
    let leaf = rgba(hsv(looks.hue_leaf, 0.5 + glow * 0.25, 0.56 + glow * 0.16));
    let leaf_light = rgba(hsv(
        looks.hue_leaf.wrapping_sub(10),
        0.4 + glow * 0.3,
        0.78 + glow * 0.2,
    ));
    let bloom_dark = rgba(hsv(looks.hue_bloom, 0.7, 0.55));
    let bloom = rgba(hsv(looks.hue_bloom, 0.62, 0.9));
    let bloom_light = rgba(hsv(looks.hue_bloom, 0.25, 1.0));
    let mushroom = looks.leaf == 5;
    let stem_col = if mushroom { [206, 196, 176, 255] } else { leaf_dark };

    let base_x = FRAME_W as f32 / 2.0 - 0.5;
    let base_y = FRAME_H as f32 - 2.0;
    let full = 7.0 + looks.height as f32 * 3.3;
    let h = (full * (0.2 + 0.8 * growth)).min(FRAME_H as f32 - 9.0);
    let lean = (looks.curve as f32 - 3.0) * 0.9 * growth;
    let side = if hash2(seed, 1, 1) & 1 == 0 { 1.0 } else { -1.0 };
    // The stem bows sideways and (for strongly curved ones) droops at the tip.
    let point = |t: f32| -> (f32, f32) {
        let droop = if looks.curve > 6 {
            (t - 0.7).max(0.0) * (looks.curve as f32 - 6.0) * 4.0
        } else {
            0.0
        };
        (base_x + side * lean * t * t, base_y - h * t + droop)
    };

    if looks.leaf == 4 {
        // Ribbons: several wavy strands rising from the base (kelp, vines).
        let strands = 2 + (looks.leaves as usize * growth.max(0.3) as usize + looks.leaves as usize) / 4;
        for s in 0..strands.min(5) {
            let off = (s as f32 - (strands as f32 - 1.0) / 2.0) * 3.0;
            let len = h * (0.7 + (hash2(seed, s as i32, 2) % 30) as f32 / 100.0);
            let phase = (hash2(seed, s as i32, 3) % 60) as f32 / 10.0;
            let mut prev = (base_x + off, base_y);
            for i in 1..=len as i32 {
                let t = i as f32 / len;
                let x = base_x
                    + off
                    + (t * 5.0 + phase).sin() * (1.5 + looks.curve as f32 * 0.25) * t
                    + side * lean * t;
                let y = base_y - i as f32;
                cv.line(prev.0, prev.1, x, y, if i % 5 < 3 { leaf } else { leaf_dark });
                if i % 6 == 0 && looks.leaves > 4 {
                    cv.set(x.round() as i32 + 1, y as i32, leaf_light);
                }
                prev = (x, y);
            }
        }
    } else {
        // Stem.
        let thick = looks.height > 7 && growth > 0.5;
        let mut prev = point(0.0);
        for i in 1..=h.ceil() as i32 {
            let p = point(i as f32 / h.max(1.0));
            cv.line(prev.0, prev.1, p.0, p.1, stem_col);
            if thick && i < (h * 0.6) as i32 {
                cv.set(p.0.round() as i32 + 1, p.1.round() as i32, stem_col);
            }
            prev = p;
        }
        // Leaves along the stem.
        let count = if mushroom {
            0
        } else {
            (1.0 + looks.leaves as f32 * 0.8 * growth) as i32
        };
        for i in 0..count {
            let t = 0.2 + 0.75 * (i as f32 + 0.5) / count as f32;
            let (x, y) = point(t);
            let dir = if i % 2 == 0 { 1.0 } else { -1.0 };
            let size = (2.0 + looks.leaves as f32 * 0.25 + (1.0 - t) * 2.5) * (0.5 + 0.5 * growth);
            match looks.leaf {
                0 => {
                    // Blade: a straight leaf angled upward.
                    cv.line(x, y, x + dir * size * 1.4, y - size * 0.9, leaf);
                    cv.set(
                        (x + dir * size * 1.4).round() as i32,
                        (y - size * 0.9).round() as i32,
                        leaf_light,
                    );
                }
                1 => {
                    // Frond: an arc that droops, with leaflets.
                    let mut px = (x, y);
                    let n = (size * 2.0) as i32;
                    for k in 1..=n {
                        let u = k as f32 / n as f32;
                        let fx = x + dir * size * 1.7 * u;
                        let fy = y - size * 0.9 * (u * (1.6 - u * 1.5));
                        cv.line(px.0, px.1, fx, fy, leaf);
                        if k % 2 == 0 {
                            cv.set(fx.round() as i32, fy.round() as i32 - 1, leaf_light);
                            cv.set(fx.round() as i32, fy.round() as i32 + 1, leaf_dark);
                        }
                        px = (fx, fy);
                    }
                }
                2 => {
                    // Round: a soft blob on a short stalk.
                    let (cx, cy) = (x + dir * (size + 1.0), y - 1.0);
                    cv.line(x, y, cx, cy, leaf_dark);
                    cv.disc(cx, cy, size * 0.55, leaf);
                    cv.set(cx.round() as i32, cy.round() as i32 - 1, leaf_light);
                }
                _ => {
                    // Needle: short spikes.
                    cv.line(x, y, x + dir * size, y - 1.0, leaf);
                    cv.line(x, y + 1.0, x + dir * size * 0.7, y + 2.0, leaf_dark);
                }
            }
        }
    }

    let (tx, ty) = point(1.0);
    if mushroom {
        // The cap grows with the plant and carries the glow spots.
        let r = (2.0 + looks.leaves as f32 * 0.5) * (0.4 + 0.6 * growth);
        for dy in -(r as i32)..=1 {
            let w = (r * r - (dy as f32).powi(2) * 1.6).max(0.0).sqrt() + 1.0;
            for dx in -(w as i32)..=w as i32 {
                let c = if dy == -(r as i32) || dx == -(w as i32) {
                    leaf_light
                } else if dy >= 0 {
                    leaf_dark
                } else {
                    leaf
                };
                cv.set(tx.round() as i32 + dx, ty.round() as i32 + dy, c);
            }
        }
        if looks.glow > 4 {
            for k in 0..3 {
                let dx = (hash2(seed, k, 9) % (2 * r as u64 + 1)) as i32 - r as i32;
                cv.set(
                    tx.round() as i32 + dx,
                    ty.round() as i32 - 1 - (k % 2),
                    bloom_light,
                );
            }
        }
    }

    // The bloom opens over the last third of growth.
    let open = ((growth - 0.6) / 0.4).clamp(0.0, 1.0);
    if open > 0.0 && looks.bloom != 0 {
        let s = 1.0 + open * 2.5;
        let (bx, by) = (tx.round(), ty.round());
        match looks.bloom {
            1 => {
                // Cell: a little battery whose charge bars fill as it ripens.
                let (w, hgt) = ((1.0 + s * 0.6) as i32, (2.0 + s * 1.3) as i32);
                for dy in -hgt..=0 {
                    for dx in -w..=w {
                        let edge = dx.abs() == w || dy == -hgt || dy == 0;
                        let bars = (-dy) as f32 <= open * hgt as f32 && dy % 2 != 0;
                        let c = if edge {
                            bloom_dark
                        } else if bars {
                            bloom_light
                        } else {
                            [30, 38, 44, 255]
                        };
                        cv.set(bx as i32 + dx, by as i32 + dy, c);
                    }
                }
                cv.set(bx as i32, by as i32 - hgt - 1, bloom);
            }
            2 => {
                // Bell: a hanging cup.
                for dy in 0..=(s as i32 + 1) {
                    let w = 1 + dy / 2;
                    for dx in -w..=w {
                        cv.set(
                            bx as i32 + dx,
                            by as i32 + dy - 1,
                            if dx == -w { bloom_light } else { bloom },
                        );
                    }
                }
                cv.set(bx as i32, by as i32 + s as i32 + 1, bloom_light);
            }
            3 => {
                // Star: petals around a bright centre.
                let r = s + 0.5;
                for k in 0..5 {
                    let a = k as f32 * std::f32::consts::TAU / 5.0 - 1.57;
                    cv.line(bx, by - 1.0, bx + a.cos() * r, by - 1.0 + a.sin() * r, bloom);
                }
                cv.disc(bx, by - 1.0, 1.0, bloom_light);
            }
            4 => {
                // Pod: plump fruit hanging off the tip.
                cv.disc(bx + side * 2.0, by + 1.0, s * 0.8, bloom);
                cv.set((bx + side * 2.0) as i32 - 1, by as i32, bloom_light);
                cv.disc(bx - side * 2.0, by + 3.0, s * 0.55, bloom_dark);
            }
            5 => {
                // Orb: a glowing bulb.
                cv.disc(bx, by - s * 0.6, s * 0.9, bloom);
                cv.disc(bx - 1.0, by - s * 0.6 - 1.0, s * 0.3, bloom_light);
            }
            _ => {
                // Spike: an ear of grain or a flower spire.
                let len = (s * 2.2) as i32;
                for i in 0..len {
                    let y = by as i32 - i;
                    cv.set(bx as i32, y, bloom);
                    if i % 2 == 0 {
                        cv.set(bx as i32 - 1, y, bloom_dark);
                        cv.set(bx as i32 + 1, y, bloom_light);
                    }
                }
            }
        }
    }

    cv.outline([18, 16, 26, 255]);
    // A soft halo around glowing blooms and caps.
    if looks.glow >= 6 && growth > 0.5 {
        let halo = hsv(if mushroom { looks.hue_leaf } else { looks.hue_bloom }, 0.5, 1.0);
        let a = (40.0 + glow * 50.0) as u8;
        let (cx, cy) = (tx.round() as i32, ty.round() as i32);
        for dy in -7..=5 {
            for dx in -7..=7 {
                if dx * dx + dy * dy <= 46 && cv.get(cx + dx, cy + dy)[3] == 0 {
                    cv.set(
                        cx + dx,
                        cy + dy,
                        [
                            halo[0],
                            halo[1],
                            halo[2],
                            a / (1 + (dx * dx + dy * dy) as u8 / 16),
                        ],
                    );
                }
            }
        }
    }
    cv
}

/// All growth stages of a species side by side, as RGBA bytes
/// (`STAGES * FRAME_W` by `FRAME_H`).
pub fn sheet(looks: &Looks, seed: u64) -> Vec<u8> {
    let w = STAGES * FRAME_W;
    let mut out = vec![0u8; w * FRAME_H * 4];
    for stage in 0..STAGES {
        let growth = if stage == STAGES - 1 {
            1.0
        } else {
            0.08 + stage as f32 * 0.19
        };
        let cv = draw(looks, growth, seed);
        for y in 0..FRAME_H {
            for x in 0..FRAME_W {
                let o = (y * w + stage * FRAME_W + x) * 4;
                out[o..o + 4].copy_from_slice(&cv.px[y * FRAME_W + x]);
            }
        }
    }
    out
}

/// The stage to show for a growth value.
pub fn stage(growth: f32) -> usize {
    if growth >= 1.0 {
        STAGES - 1
    } else {
        ((growth * (STAGES - 1) as f32) as usize).min(STAGES - 2)
    }
}

/// Generated plant sheets, by species, made on first use.
#[derive(Resource, Default)]
pub struct PlantArt {
    sheets: HashMap<SpeciesId, Handle<Image>>,
    layout: Option<Handle<TextureAtlasLayout>>,
}

impl PlantArt {
    /// A sprite of `species` at a growth stage.
    pub fn sprite(
        &mut self,
        species: &Species,
        growth: f32,
        images: &mut Assets<Image>,
        layouts: &mut Assets<TextureAtlasLayout>,
    ) -> Sprite {
        let layout = self
            .layout
            .get_or_insert_with(|| {
                layouts.add(TextureAtlasLayout::from_grid(
                    UVec2::new(FRAME_W as u32, FRAME_H as u32),
                    STAGES as u32,
                    1,
                    None,
                    None,
                ))
            })
            .clone();
        let image = self
            .sheets
            .entry(species.id)
            .or_insert_with(|| {
                let mut image = Image::new(
                    Extent3d {
                        width: (STAGES * FRAME_W) as u32,
                        height: FRAME_H as u32,
                        depth_or_array_layers: 1,
                    },
                    TextureDimension::D2,
                    sheet(&species.looks, species.id as u64 * 7919 + 13),
                    TextureFormat::Rgba8UnormSrgb,
                    RenderAssetUsages::RENDER_WORLD,
                );
                image.sampler = ImageSampler::nearest();
                images.add(image)
            })
            .clone();
        Sprite::from_atlas_image(
            image,
            TextureAtlas {
                layout,
                index: stage(growth),
            },
        )
    }

    /// Forgets every sheet (species ids restart in a new world).
    pub fn clear(&mut self) {
        self.sheets.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sbct_sim::colony::plants::{base_species, splice};

    fn opaque(sheet: &[u8], stage: usize) -> usize {
        let w = STAGES * FRAME_W;
        (0..FRAME_H)
            .flat_map(|y| (0..FRAME_W).map(move |x| (x, y)))
            .filter(|&(x, y)| sheet[(y * w + stage * FRAME_W + x) * 4 + 3] > 200)
            .count()
    }

    #[test]
    fn every_species_draws_and_grows() {
        let all = base_species();
        let mut seen = std::collections::HashSet::new();
        for sp in &all {
            let s = sheet(&sp.looks, sp.id as u64);
            assert_eq!(s.len(), STAGES * FRAME_W * FRAME_H * 4);
            let (sprout, mature) = (opaque(&s, 0), opaque(&s, STAGES - 1));
            assert!(sprout > 3, "{} sprout is empty", sp.name);
            assert!(
                mature > sprout * 2,
                "{} doesn't visibly grow ({sprout} -> {mature})",
                sp.name
            );
            // Nothing is cut off: the outer columns and top row stay clear.
            let w = STAGES * FRAME_W;
            for stage in 0..STAGES {
                for y in 0..FRAME_H {
                    for x in [0, FRAME_W - 1] {
                        assert!(
                            s[(y * w + stage * FRAME_W + x) * 4 + 3] < 200,
                            "{} touches the frame edge",
                            sp.name
                        );
                    }
                }
                for x in 0..FRAME_W {
                    assert!(
                        s[(stage * FRAME_W + x) * 4 + 3] < 200,
                        "{} touches the top",
                        sp.name
                    );
                }
            }
            assert!(seen.insert(s), "{} looks identical to another species", sp.name);
        }
    }

    #[test]
    fn hybrids_get_their_own_look() {
        let all = base_species();
        let child = splice(5, &all[0], &all[1], 0, &all);
        let (a, b, c) = (
            sheet(&all[0].looks, 1),
            sheet(&all[1].looks, 1),
            sheet(&child.looks, 1),
        );
        assert!(c != a && c != b);
        assert!(opaque(&c, STAGES - 1) > 20);
    }

    #[test]
    fn stages_follow_growth() {
        assert_eq!(stage(0.0), 0);
        assert_eq!(stage(0.99), STAGES - 2);
        assert_eq!(stage(1.0), STAGES - 1);
    }
}
