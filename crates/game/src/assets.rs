//! Handles to the generated art (see `tools/gen_art.py` for sheet layouts).

use std::path::PathBuf;

use bevy::prelude::*;

/// Asset folder path relative to Bevy's base path. Under `cargo run` the base
/// is `crates/game`; for a binary in `target/<profile>` it's that directory.
/// Both resolve to the repository's `assets/`.
pub const ASSET_ROOT: &str = "../../assets";

/// The same folder as a filesystem path, for things loaded outside Bevy's
/// asset server (the egui font).
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

#[derive(Clone)]
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

#[derive(Resource, Clone)]
pub struct GameAssets {
    /// 16×16 frames, 8 per row; rows are animations (see [`player_anim`]).
    pub player: Sheet,
    /// 16×16 icons, 8 per row.
    pub items: Sheet,
    /// 16×24 frames: torch 0–3, chest closed 4, chest open 5, altar 6, shrine 7.
    pub props: Sheet,
    /// 48×48 frames, 8 total.
    pub core: Sheet,
    /// 16×16 frames, 4 per row: crawler, magma slug, spore drifter.
    pub creatures: Sheet,
    /// Per layer: (far, near) tileable 256×256 backgrounds.
    pub backgrounds: Vec<(Handle<Image>, Handle<Image>)>,
}

/// Player animation rows and frame counts.
pub mod player_anim {
    pub const IDLE: (usize, usize) = (0, 6);
    pub const RUN: (usize, usize) = (1, 6);
    pub const JUMP: (usize, usize) = (2, 4);
    pub const FALL: (usize, usize) = (3, 4);
    pub const CLIMB: (usize, usize) = (4, 6);
    pub const DIG_SIDE: (usize, usize) = (5, 6);
    pub const DIG_DOWN: (usize, usize) = (6, 6);
    pub const DIG_UP: (usize, usize) = (7, 6);
    pub const HURT: (usize, usize) = (8, 4);
    pub const DEATH: (usize, usize) = (9, 8);
    pub const COLUMNS: usize = 8;
}

pub mod props_frame {
    pub const TORCH: usize = 0;
    pub const CHEST_CLOSED: usize = 4;
    pub const CHEST_OPEN: usize = 5;
    pub const ALTAR: usize = 6;
    pub const SHRINE: usize = 7;
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
    let assets = GameAssets {
        player: sheet("sprites/player.png", UVec2::splat(16), 8, 10),
        items: sheet("sprites/items.png", UVec2::splat(16), 8, 5),
        props: sheet("sprites/props.png", UVec2::new(16, 24), 8, 1),
        core: sheet("sprites/core.png", UVec2::splat(48), 8, 1),
        creatures: sheet("sprites/creatures.png", UVec2::splat(16), 4, 3),
        backgrounds: (0..5)
            .map(|i| {
                (
                    repeating(&server, format!("backgrounds/layer{i}_far.png")),
                    repeating(&server, format!("backgrounds/layer{i}_near.png")),
                )
            })
            .collect(),
    };
    commands.insert_resource(assets);
}

/// Loads a texture that tiles (repeat addressing) for the parallax layers.
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
