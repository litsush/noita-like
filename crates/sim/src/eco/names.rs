//! Alien-sounding names built from syllables.

use crate::rng::Rng;

const ONSETS: &[&str] = &[
    "k", "v", "x", "z", "th", "kr", "vr", "sk", "q", "gl", "ny", "ss", "dr", "ph", "y", "m", "n", "t", "r",
    "sh", "zh", "kh", "tl", "gw", "h", "b", "l",
];
const VOWELS: &[&str] = &[
    "a", "e", "i", "o", "u", "y", "aa", "ei", "ou", "ae", "io", "ü", "ö", "ai",
];
const CODAS: &[&str] = &[
    "", "", "", "x", "th", "n", "r", "l", "k", "s", "sh", "m", "q", "rk", "nd", "z", "ss", "lth",
];

fn syllable(rng: &mut Rng) -> String {
    format!("{}{}{}", rng.pick(ONSETS), rng.pick(VOWELS), rng.pick(CODAS))
}

fn capitalise(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// A word of 2-3 syllables, capitalised.
pub fn word(rng: &mut Rng) -> String {
    let n = rng.int(2, 3);
    let mut s = String::new();
    for i in 0..n {
        let syl = syllable(rng);
        // Occasional apostrophes and hyphens for flavour.
        if i > 0 && rng.chance(18) {
            s.push('\'');
        }
        s.push_str(&syl);
    }
    capitalise(&s)
}

/// "Genus species" in the usual style.
pub fn binomial(rng: &mut Rng) -> String {
    let genus = word(rng);
    let mut epithet = word(rng).to_lowercase();
    let endings = ["ii", "ensis", "ax", "ora", "ix", "us", "oid", "a", "ae"];
    if rng.coin() {
        epithet.push_str(rng.pick(&endings));
    }
    format!("{genus} {epithet}")
}

/// A planet designation, e.g. "Vreth-7b".
pub fn planet(rng: &mut Rng) -> String {
    let letters = ["b", "c", "d", "e", "f"];
    format!("{}-{}{}", word(rng), rng.int(2, 19), rng.pick(&letters))
}

/// A broad colour word for a hue, for common names.
pub fn colour_word(rgb: [u8; 3], rng: &mut Rng) -> &'static str {
    let (h, s, v) = super::math::to_hsv(rgb);
    if v < 0.25 {
        return rng.pick(&["Ink", "Shadow", "Soot", "Umbral"]);
    }
    if s < 0.18 {
        return if v > 0.7 {
            rng.pick(&["Pale", "Ghost", "Bone", "Ashen"])
        } else {
            rng.pick(&["Grey", "Slate", "Dust", "Ashen"])
        };
    }
    let options: &[&str] = match h as i32 {
        0..=14 | 345..=360 => &["Crimson", "Blood", "Rust", "Scarlet"],
        15..=40 => &["Ember", "Amber", "Copper", "Rust"],
        41..=65 => &["Gold", "Sulphur", "Ochre", "Saffron"],
        66..=100 => &["Lime", "Bile", "Moss", "Chartreuse"],
        101..=160 => &["Jade", "Verdant", "Emerald", "Moss"],
        161..=200 => &["Teal", "Cyan", "Glacier", "Tide"],
        201..=250 => &["Azure", "Cobalt", "Sapphire", "Deep"],
        251..=290 => &["Violet", "Indigo", "Dusk", "Amethyst"],
        _ => &["Magenta", "Orchid", "Fuchsia", "Rose"],
    };
    rng.pick(options)
}
