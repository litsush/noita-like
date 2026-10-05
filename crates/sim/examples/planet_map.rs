//! Renders a generated planet to a PNG for eyeballing world generation
//! without running the game:
//!
//!     cargo run -p sbct_sim --release --example planet_map -- <seed> <out.png> [scale-down]

use sbct_sim::color::cell_rgba;
use sbct_sim::planetgen::{FeatureKind, generate};

fn main() {
    let mut args = std::env::args().skip(1);
    let seed: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(1);
    let out = args.next().unwrap_or_else(|| "planet.png".into());
    let step: usize = args.next().and_then(|s| s.parse().ok()).unwrap_or(1).max(1);
    let start = std::time::Instant::now();
    let planet = generate(seed);
    eprintln!("generated seed {seed} in {:?}", start.elapsed());

    let (w, h) = (planet.world.width() / step, planet.world.height() / step);
    let mut rgba = Vec::with_capacity(w * h * 4);
    for y in 0..h {
        for x in 0..w {
            let (wx, wy) = ((x * step) as i32, (y * step) as i32);
            let c = planet.world.get(wx, wy).unwrap();
            let [r, g, b, a] = cell_rgba(c, wx, wy, 0);
            // Composite over sky or a dark backdrop so caves are visible.
            let bg = if wy < planet.profile.surface_at(wx) {
                [96, 124, 150]
            } else {
                [18, 16, 22]
            };
            let mix = |c: u8, bg: u8| ((c as u32 * a as u32 + bg as u32 * (255 - a as u32)) / 255) as u8;
            rgba.extend_from_slice(&[mix(r, bg[0]), mix(g, bg[1]), mix(b, bg[2]), 255]);
        }
    }
    for f in &planet.features {
        let color = match f.kind {
            FeatureKind::Vault => [255, 220, 0, 255],
            FeatureKind::Lair => [255, 60, 60, 255],
            FeatureKind::Ruin => [255, 255, 255, 255],
            FeatureKind::Entrance => [0, 255, 255, 255],
        };
        for dy in -4..=0 {
            for dx in -2..=2 {
                let (x, y) = (f.x / step as i32 + dx, f.y / step as i32 + dy);
                if x >= 0 && (x as usize) < w && y >= 0 && (y as usize) < h {
                    let i = (y as usize * w + x as usize) * 4;
                    rgba[i..i + 4].copy_from_slice(&color);
                }
            }
        }
    }
    std::fs::write(&out, png(w as u32, h as u32, &rgba)).unwrap();
    eprintln!("wrote {out}");
}

/// Minimal PNG encoder (stored deflate blocks) so the example has no dependencies.
fn png(w: u32, h: u32, rgba: &[u8]) -> Vec<u8> {
    fn crc32(data: &[u8]) -> u32 {
        let mut c = 0xFFFF_FFFFu32;
        for &b in data {
            c ^= b as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 {
                    0xEDB8_8320 ^ (c >> 1)
                } else {
                    c >> 1
                };
            }
        }
        !c
    }
    fn chunk(out: &mut Vec<u8>, kind: &[u8], data: &[u8]) {
        out.extend_from_slice(&(data.len() as u32).to_be_bytes());
        let mut body = kind.to_vec();
        body.extend_from_slice(data);
        out.extend_from_slice(&body);
        out.extend_from_slice(&crc32(&body).to_be_bytes());
    }
    let mut raw = Vec::with_capacity(rgba.len() + h as usize);
    for row in rgba.chunks(w as usize * 4) {
        raw.push(0);
        raw.extend_from_slice(row);
    }
    let mut z = vec![0x78, 0x01];
    for (i, block) in raw.chunks(65535).enumerate() {
        let last = (i + 1) * 65535 >= raw.len();
        z.push(last as u8);
        z.extend_from_slice(&(block.len() as u16).to_le_bytes());
        z.extend_from_slice(&(!(block.len() as u16)).to_le_bytes());
        z.extend_from_slice(block);
    }
    let (mut a, mut b) = (1u32, 0u32);
    for &x in &raw {
        a = (a + x as u32) % 65521;
        b = (b + a) % 65521;
    }
    z.extend_from_slice(&((b << 16) | a).to_be_bytes());

    let mut out = b"\x89PNG\r\n\x1a\n".to_vec();
    let mut ihdr = Vec::new();
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);
    chunk(&mut out, b"IDAT", &z);
    chunk(&mut out, b"IEND", &[]);
    out
}
