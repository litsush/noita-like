//! Renders a generated descent to PNGs, one per layer, for eyeballing world
//! generation without running the game:
//!
//!     cargo run -p sbct_sim --release --example descent_map -- <seed> <out-dir>

use sbct_sim::color::cell_rgba;
use sbct_sim::descent::{Layer, SpawnKind, generate_descent};

fn main() {
    let mut args = std::env::args().skip(1);
    let seed: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(1);
    let out = args.next().unwrap_or_else(|| ".".into());
    let start = std::time::Instant::now();
    let d = generate_descent(seed);
    eprintln!("generated seed {seed} in {:?}", start.elapsed());

    let w = d.world.width();
    for layer in Layer::ALL {
        let (y0, y1) = (layer.top() as usize, layer.bottom() as usize);
        let mut rgba = Vec::with_capacity(w * (y1 - y0) * 4);
        for y in y0..y1 {
            for x in 0..w {
                let c = d.world.get(x as i32, y as i32).unwrap();
                let [r, g, b, a] = cell_rgba(c, x as i32, y as i32, 0);
                // Composite over a dark backdrop so caves are visible.
                let mix = |c: u8, bg: u8| ((c as u32 * a as u32 + bg as u32 * (255 - a as u32)) / 255) as u8;
                rgba.extend_from_slice(&[mix(r, 18), mix(g, 16), mix(b, 22), 255]);
            }
        }
        for s in &d.spawns {
            if (y0..y1).contains(&(s.y as usize)) {
                let color = match s.kind {
                    SpawnKind::Chest => [255, 220, 0, 255],
                    SpawnKind::Altar => [0, 255, 255, 255],
                    SpawnKind::Shrine => [255, 0, 255, 255],
                    SpawnKind::Core => [255, 255, 255, 255],
                    SpawnKind::Creature(_) => [255, 60, 60, 255],
                };
                for dy in -3..=0 {
                    for dx in -2..=2 {
                        let (x, y) = (s.x + dx, s.y + dy - y0 as i32);
                        if x >= 0 && (x as usize) < w && y >= 0 && (y as usize) < y1 - y0 {
                            let i = (y as usize * w + x as usize) * 4;
                            rgba[i..i + 4].copy_from_slice(&color);
                        }
                    }
                }
            }
        }
        let path = format!("{out}/layer{}.png", layer.index());
        std::fs::write(&path, png(w as u32, (y1 - y0) as u32, &rgba)).unwrap();
        eprintln!("wrote {path}");
    }
}

/// Minimal PNG encoder (stored deflate blocks) so the example has no dependencies.
fn png(w: u32, h: u32, rgba: &[u8]) -> Vec<u8> {
    fn crc32(data: &[u8]) -> u32 {
        let mut c = 0xFFFF_FFFFu32;
        for &b in data {
            c ^= b as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 { 0xEDB8_8320 ^ (c >> 1) } else { c >> 1 };
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
