//! Handles to the generated art. Sheet layouts are documented at the top of
//! `tools/gen_sprites.py` (characters, creatures, mods) and
//! `tools/gen_art.py` (icons, domes, props, backgrounds, UI).

use std::collections::HashMap;
use std::path::PathBuf;

use bevy::prelude::*;
use sbct_sim::colony::items::DomeKind;

/// Asset folder path relative to Bevy's base path. Under `cargo run` the base
/// is `crates/game`; for a binary in `target/<profile>` it's that directory.
/// Both resolve to the repository's `assets/`.
pub const ASSET_ROOT: &str = "../../assets";

/// The same folder as a filesystem path, for things loaded outside Bevy's
/// asset server (fonts, anchor data).
pub fn assets_dir() -> PathBuf {
    let base = std::env::var_os("BEVY_ASSET_ROOT")
        .or_else(|| std::env::var_os("CARGO_MANIFEST_DIR"))
        .map(PathBuf::from)
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(PathBuf::from))
        })
        .unwrap_or_default();
    let dir = base.join(ASSET_ROOT);
    if dir.exists() {
        dir
    } else {
        PathBuf::from("assets")
    }
}

#[derive(Clone, Default)]
pub struct Sheet {
    pub image: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
}

impl Sheet {
    pub fn sprite(&self, index: usize) -> Sprite {
        Sprite::from_atlas_image(
            self.image.clone(),
            TextureAtlas {
                layout: self.layout.clone(),
                index,
            },
        )
    }
}

/// Player animation rows as (row, frames, frames per second). Columns per
/// row: 8. Frame size 24×32.
pub mod player_anim {
    pub const COLUMNS: usize = 8;
    pub const IDLE: u8 = 0;
    pub const RUN: u8 = 1;
    pub const JUMP: u8 = 2;
    pub const FALL: u8 = 3;
    pub const CLIMB: u8 = 4;
    pub const SWIM: u8 = 5;
    pub const REACH: u8 = 6;
    pub const REACH_UP: u8 = 7;
    pub const REACH_DOWN: u8 = 8;
    pub const HURT: u8 = 9;
    pub const BLACKOUT: u8 = 10;
    pub const SLEEP: u8 = 11;
    pub const FLY: u8 = 12;
    pub const GLIDE: u8 = 13;
    pub const DASH: u8 = 14;

    /// (sheet row, first column, frame count, frames per second, loops)
    pub fn frames(anim: u8) -> (usize, usize, usize, f32, bool) {
        match anim {
            RUN => (1, 0, 8, 12.0, true),
            JUMP => (2, 0, 2, 6.0, false),
            FALL => (2, 2, 2, 8.0, true),
            CLIMB => (3, 0, 4, 8.0, true),
            SWIM => (4, 0, 4, 6.0, true),
            REACH => (5, 0, 4, 12.0, false),
            REACH_UP => (6, 0, 4, 12.0, false),
            REACH_DOWN => (7, 0, 4, 12.0, false),
            HURT => (8, 0, 2, 10.0, false),
            BLACKOUT => (8, 2, 4, 6.0, false),
            SLEEP => (9, 0, 2, 1.0, true),
            FLY => (10, 0, 4, 10.0, true),
            GLIDE => (11, 0, 2, 6.0, true),
            DASH => (11, 2, 2, 12.0, true),
            _ => (0, 0, 4, 4.0, true),
        }
    }

    /// Atlas index of an animation at `time` seconds into it.
    pub fn index(anim: u8, time: f32) -> usize {
        let (row, first, count, fps, loops) = frames(anim);
        let f = (time * fps) as usize;
        let f = if loops { f % count } else { f.min(count - 1) };
        row * COLUMNS + first + f
    }
}

/// Rows of `props.png` (32×32 cells, 4 frames each).
#[allow(dead_code)]
pub mod prop {
    pub const BED: usize = 0;
    pub const BED_OCCUPIED: usize = 1;
    pub const CHEST: usize = 2;
    pub const SUIT_RACK: usize = 3;
    pub const SYNTHESIZER: usize = 4;
    pub const TERMINAL: usize = 5;
    pub const MOD_BAY: usize = 6;
    pub const PLANTER: usize = 7;
    pub const DARK_PLANTER: usize = 8;
    pub const AQUA_PLANTER: usize = 9;
    pub const COOKER: usize = 10;
    pub const COMPOST_VAT: usize = 11;
    pub const ASSEMBLY_RING: usize = 12;
    pub const FISH_POOL: usize = 13;
    pub const STALL: usize = 14;
    pub const TROUGH: usize = 15;
    pub const CONDENSER: usize = 16;
    pub const BERTH: usize = 17;
    pub const SPRINKLER: usize = 18;
    pub const TAP: usize = 19;
    pub const LAMP: usize = 20;
    pub const PYLON: usize = 21;
    pub const PUMP: usize = 22;
    pub const TANK: usize = 23;
    pub const OXYGEN_GENERATOR: usize = 24;
    pub const SPLICER: usize = 25;
    pub const POD: usize = 26;
    pub const PACK: usize = 27;
    pub const PIN: usize = 28;
    pub const CRATE: usize = 29;
    pub const BUNK: usize = 30;
    pub const KIT: usize = 31;
}

/// Per-frame attachment points on the player sprite, from
/// `player_anchors.json`. Coordinates are pixels in the 24×32 frame, facing
/// right.
#[derive(Resource, Default, Clone)]
pub struct PlayerAnchors {
    /// `slots[slot][atlas index]`, in body-slot order (head, eyes, torso,
    /// back, arms, hands, legs, feet).
    pub slots: [Vec<Option<(f32, f32)>>; 8],
}

impl PlayerAnchors {
    pub const SLOT_NAMES: [&'static str; 8] =
        ["head", "eyes", "torso", "back", "arms", "hands", "legs", "feet"];

    fn load() -> PlayerAnchors {
        let path = assets_dir().join("sprites/player_anchors.json");
        let mut out = PlayerAnchors::default();
        let Ok(text) = std::fs::read_to_string(&path) else {
            warn!("missing {}", path.display());
            return out;
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
            warn!("bad anchors file {}", path.display());
            return out;
        };
        for (i, name) in Self::SLOT_NAMES.iter().enumerate() {
            let Some(rows) = json["anchors"][name].as_array() else {
                continue;
            };
            for row in rows {
                let cols = row.as_array().cloned().unwrap_or_default();
                for c in 0..player_anim::COLUMNS {
                    let p = cols.get(c).and_then(|v| {
                        let a = v.as_array()?;
                        Some((a.first()?.as_f64()? as f32, a.get(1)?.as_f64()? as f32))
                    });
                    out.slots[i].push(p);
                }
            }
        }
        out
    }

    /// Offset of a slot's anchor from the sprite's centre (y up), facing right.
    pub fn offset(&self, slot: usize, index: usize) -> Option<Vec2> {
        let (x, y) = self.slots[slot].get(index).copied().flatten()?;
        Some(Vec2::new(x - 12.0, 16.0 - y))
    }
}

#[derive(Resource, Clone, Default)]
#[allow(dead_code)] // Every generated sheet is listed; some are used by later milestones.
pub struct GameAssets {
    /// 24×32 frames, 8×12: the four player layers share one layout.
    pub player_body: Sheet,
    pub player_suit: Sheet,
    pub player_accent_body: Sheet,
    pub player_accent_suit: Sheet,
    /// 16×16 cells, 8 columns (tier × 2 frames) × 53 rows (mod sets).
    pub mods: Sheet,
    /// 16×16 per mod set, 16 columns.
    pub mod_icons: Handle<Image>,
    /// 16×16 icons, 16 columns × 8 rows.
    pub icons: Handle<Image>,
    pub icon_sheet: Sheet,
    /// 16×16 frames, 8×5.
    pub robot: Sheet,
    pub robot_accent: Sheet,
    /// 24×24 frames, 8×12.
    pub creatures: Sheet,
    /// 64×48 frames, 4×3.
    pub brood_mother: Sheet,
    /// 16×24 frames, 8×6.
    pub workers: Sheet,
    /// 16×16 frames, 8×3.
    pub animals: Sheet,
    /// 32×32 cells, 4 frames per row, 32 rows.
    pub props: Sheet,
    /// 16×16 cells, 8×8.
    pub fx: Sheet,
    /// 16×16 cells, 8×6.
    pub foliage: Sheet,
    /// Dome sprites: (back, front) per kind.
    pub domes: HashMap<DomeKind, (Handle<Image>, Handle<Image>)>,
    pub ui: Handle<Image>,
    pub ship: Handle<Image>,
    pub ship_flame: Sheet,
    pub surface_far: Handle<Image>,
    pub surface_mid: Handle<Image>,
    /// (far, near) per underground band: cave, deep, abyss.
    pub cave_layers: Vec<(Handle<Image>, Handle<Image>)>,
    pub mist: Handle<Image>,
    pub menu: Handle<Image>,
}

pub fn load_assets(
    mut commands: Commands,
    server: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let mut sheet = |path: &str, size: UVec2, cols: u32, rows: u32| Sheet {
        image: server.load(path.to_string()),
        layout: layouts.add(TextureAtlasLayout::from_grid(size, cols, rows, None, None)),
    };
    let player = UVec2::new(24, 32);
    let icon_sheet = sheet("sprites/icons.png", UVec2::splat(16), 16, 8);
    let assets = GameAssets {
        player_body: sheet("sprites/player_body.png", player, 8, 12),
        player_suit: sheet("sprites/player_suit.png", player, 8, 12),
        player_accent_body: sheet("sprites/player_accent_body.png", player, 8, 12),
        player_accent_suit: sheet("sprites/player_accent_suit.png", player, 8, 12),
        mods: sheet("sprites/mods.png", UVec2::splat(16), 8, 53),
        mod_icons: server.load("sprites/mod_icons.png"),
        icons: icon_sheet.image.clone(),
        icon_sheet,
        robot: sheet("sprites/robot.png", UVec2::splat(16), 8, 5),
        robot_accent: sheet("sprites/robot_accent.png", UVec2::splat(16), 8, 5),
        creatures: sheet("sprites/creatures.png", UVec2::splat(24), 8, 12),
        brood_mother: sheet("sprites/brood_mother.png", UVec2::new(64, 48), 4, 3),
        workers: sheet("sprites/workers.png", UVec2::new(16, 24), 8, 6),
        animals: sheet("sprites/animals.png", UVec2::splat(16), 8, 3),
        props: sheet("sprites/props.png", UVec2::splat(32), 4, 32),
        fx: sheet("sprites/fx.png", UVec2::splat(16), 8, 8),
        foliage: sheet("sprites/foliage.png", UVec2::splat(16), 8, 6),
        domes: [DomeKind::Starter]
            .into_iter()
            .chain(DomeKind::KITS)
            .map(|k| {
                (
                    k,
                    (
                        server.load(format!("sprites/domes/{}_back.png", k.sprite())),
                        server.load(format!("sprites/domes/{}_front.png", k.sprite())),
                    ),
                )
            })
            .collect(),
        ui: server.load("sprites/ui.png"),
        ship: server.load("sprites/ship.png"),
        ship_flame: sheet("sprites/ship_flame.png", UVec2::new(64, 48), 4, 1),
        surface_far: repeating(&server, "backgrounds/surface_far.png".into()),
        surface_mid: repeating(&server, "backgrounds/surface_mid.png".into()),
        cave_layers: ["cave", "deep", "abyss"]
            .iter()
            .map(|n| {
                (
                    repeating(&server, format!("backgrounds/{n}_far.png")),
                    repeating(&server, format!("backgrounds/{n}_near.png")),
                )
            })
            .collect(),
        mist: repeating(&server, "backgrounds/mist.png".into()),
        menu: server.load("backgrounds/menu.png"),
    };
    commands.insert_resource(assets);
    commands.insert_resource(PlayerAnchors::load());
}

/// Loads a texture that tiles (repeat addressing) for parallax layers.
fn repeating(server: &AssetServer, path: String) -> Handle<Image> {
    use bevy::image::{ImageAddressMode, ImageLoaderSettings, ImageSampler, ImageSamplerDescriptor};
    server
        .load_builder()
        .with_settings(|s: &mut ImageLoaderSettings| {
            s.sampler = ImageSampler::Descriptor(ImageSamplerDescriptor {
                address_mode_u: ImageAddressMode::Repeat,
                address_mode_v: ImageAddressMode::Repeat,
                ..ImageSamplerDescriptor::nearest()
            });
        })
        .load(path)
}
