//! Shared egui look: the generated pixel fonts, a sharp-cornered dark theme
//! with mint accents, and helpers to draw sprite-sheet icons.

use std::sync::Arc;

use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiTextureHandle, egui};

use crate::assets::{GameAssets, assets_dir};

// The plant-punk palette: deep teal-black panels, pale lichen text, mint
// growth for accents, rust and amber for warnings.
pub const ACCENT: egui::Color32 = egui::Color32::from_rgb(138, 228, 168);
pub const ACCENT_DIM: egui::Color32 = egui::Color32::from_rgb(46, 104, 88);
pub const TEXT: egui::Color32 = egui::Color32::from_rgb(220, 228, 214);
pub const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(128, 146, 140);
pub const PANEL: egui::Color32 = egui::Color32::from_rgba_premultiplied(11, 19, 21, 238);
pub const BORDER: egui::Color32 = egui::Color32::from_rgb(70, 98, 94);
/// The inner rule of the double border.
pub const RULE: egui::Color32 = egui::Color32::from_rgb(34, 54, 54);
pub const DANGER: egui::Color32 = egui::Color32::from_rgb(236, 96, 80);
pub const GOOD: egui::Color32 = egui::Color32::from_rgb(150, 232, 120);
pub const WARN: egui::Color32 = egui::Color32::from_rgb(240, 184, 84);
pub const WATER: egui::Color32 = egui::Color32::from_rgb(96, 176, 232);
pub const POWER: egui::Color32 = egui::Color32::from_rgb(250, 226, 110);

/// Team colours, indexed by `Player::color`.
pub const TEAM: [[u8; 3]; 8] = [
    [240, 132, 84],
    [96, 200, 236],
    [176, 232, 104],
    [236, 196, 84],
    [214, 124, 220],
    [108, 232, 190],
    [240, 110, 134],
    [150, 150, 244],
];

pub fn team_color(index: u8) -> egui::Color32 {
    let [r, g, b] = TEAM[index as usize % TEAM.len()];
    egui::Color32::from_rgb(r, g, b)
}

/// egui texture ids for sprite sheets drawn in the UI.
#[derive(Resource, Clone, Copy)]
#[allow(dead_code)] // The mod sheet is used once body mods land.
pub struct UiIcons {
    pub items: egui::TextureId,
    /// One 16×16 icon per body-mod set, 16 per row.
    pub mods: egui::TextureId,
    /// The title backdrop.
    pub menu: egui::TextureId,
}

/// The ornate display face for titles and headings (falls back to the
/// pixel font if the file is missing).
pub fn title_font(size: f32) -> egui::FontId {
    // `set_fonts` only applies from the next frame, so the first frame
    // must not name the custom family yet.
    if TITLE_FONT_READY.load(std::sync::atomic::Ordering::Relaxed) {
        egui::FontId::new(size, egui::FontFamily::Name("title".into()))
    } else {
        egui::FontId::proportional(size)
    }
}

static TITLE_FONT_READY: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// A heading in the title face.
pub fn title(text: impl Into<String>, size: f32) -> egui::RichText {
    egui::RichText::new(text).font(title_font(size))
}

/// Sets up fonts, theme and icon textures once.
pub fn setup_ui(
    mut contexts: EguiContexts,
    assets: Res<GameAssets>,
    mut commands: Commands,
    mut done: Local<bool>,
) -> Result {
    if *done {
        // Fonts installed last frame are live now.
        TITLE_FONT_READY.store(true, std::sync::atomic::Ordering::Relaxed);
        return Ok(());
    }
    let items = contexts.add_image(EguiTextureHandle::Strong(assets.icons.clone()));
    let mods = contexts.add_image(EguiTextureHandle::Strong(assets.mod_icons.clone()));
    let ornaments = contexts.add_image(EguiTextureHandle::Strong(assets.ui.clone()));
    let ctx = contexts.ctx_mut()?;
    install_font(ctx);
    ctx.all_styles_mut(apply_theme);
    ctx.data_mut(|d| d.insert_temp(egui::Id::new("ornaments"), ornaments));
    let menu = contexts.add_image(EguiTextureHandle::Strong(assets.menu.clone()));
    commands.insert_resource(UiIcons { items, mods, menu });
    *done = true;
    Ok(())
}

fn install_font(ctx: &egui::Context) {
    let path = assets_dir().join("fonts/pixel.ttf");
    let Ok(bytes) = std::fs::read(&path) else {
        warn!("pixel font missing at {}; using egui's default", path.display());
        return;
    };
    let mut fonts = egui::FontDefinitions::default();
    fonts
        .font_data
        .insert("pixel".into(), Arc::new(egui::FontData::from_owned(bytes)));
    for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family)
            .or_default()
            .insert(0, "pixel".into());
    }
    // The ornate title face, with the pixel font as fallback for missing glyphs.
    let title = assets_dir().join("fonts/title.ttf");
    let mut title_family = vec!["pixel".to_string()];
    if let Ok(bytes) = std::fs::read(&title) {
        fonts
            .font_data
            .insert("title".into(), Arc::new(egui::FontData::from_owned(bytes)));
        title_family.insert(0, "title".into());
    }
    fonts
        .families
        .insert(egui::FontFamily::Name("title".into()), title_family);
    ctx.set_fonts(fonts);
}

fn apply_theme(style: &mut egui::Style) {
    use egui::{CornerRadius, FontFamily, FontId, Stroke, TextStyle};
    // The pixel font is crisp at multiples of 8 points.
    style.text_styles = [
        (TextStyle::Small, FontId::new(16.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(16.0, FontFamily::Proportional)),
        (TextStyle::Button, FontId::new(16.0, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(16.0, FontFamily::Monospace)),
        (TextStyle::Heading, FontId::new(32.0, FontFamily::Proportional)),
    ]
    .into();
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 6.0);
    style.spacing.window_margin = egui::Margin::same(12);
    style.spacing.slider_width = 200.0;

    let v = &mut style.visuals;
    *v = egui::Visuals::dark();
    v.override_text_color = Some(TEXT);
    v.window_fill = PANEL;
    v.panel_fill = egui::Color32::from_rgb(9, 15, 17);
    v.window_stroke = Stroke::new(2.0, BORDER);
    v.window_corner_radius = CornerRadius::ZERO;
    v.menu_corner_radius = CornerRadius::ZERO;
    v.window_shadow = egui::Shadow {
        offset: [4, 4],
        blur: 0,
        spread: 0,
        color: egui::Color32::from_black_alpha(120),
    };
    v.popup_shadow = v.window_shadow;
    v.selection.bg_fill = ACCENT_DIM;
    v.selection.stroke = Stroke::new(2.0, ACCENT);
    v.extreme_bg_color = egui::Color32::from_rgb(6, 11, 12);
    for (w, fill, stroke) in [
        (
            &mut v.widgets.noninteractive,
            egui::Color32::from_rgb(16, 27, 29),
            RULE,
        ),
        (
            &mut v.widgets.inactive,
            egui::Color32::from_rgb(22, 38, 40),
            BORDER,
        ),
        (
            &mut v.widgets.hovered,
            egui::Color32::from_rgb(34, 62, 60),
            ACCENT,
        ),
        (&mut v.widgets.active, ACCENT_DIM, ACCENT),
        (
            &mut v.widgets.open,
            egui::Color32::from_rgb(28, 50, 50),
            ACCENT_DIM,
        ),
    ] {
        w.bg_fill = fill;
        w.weak_bg_fill = fill;
        w.bg_stroke = Stroke::new(2.0, stroke);
        w.corner_radius = CornerRadius::ZERO;
        w.fg_stroke = Stroke::new(1.0, TEXT);
        w.expansion = 0.0;
    }
}

/// UV rectangle of icon `index` in the 16-column, 8-row icon sheet.
pub fn icon_uv(index: usize) -> egui::Rect {
    let (cols, rows) = (16.0, 8.0);
    let (x, y) = ((index % 16) as f32, (index / 16) as f32);
    egui::Rect::from_min_max(
        egui::pos2(x / cols, y / rows),
        egui::pos2((x + 1.0) / cols, (y + 1.0) / rows),
    )
}

/// An item-sheet icon as a widget.
pub fn icon(ui: &mut egui::Ui, icons: &UiIcons, index: usize, size: f32) -> egui::Response {
    ui.add(
        egui::Image::new(egui::load::SizedTexture::new(icons.items, egui::vec2(size, size)))
            .uv(icon_uv(index)),
    )
}

/// A big menu button with consistent sizing. Hover and click sounds are
/// played by [`ui_sounds`].
pub fn menu_button(ui: &mut egui::Ui, text: &str) -> egui::Response {
    let r = ui.add_sized(
        [300.0, 40.0],
        egui::Button::new(egui::RichText::new(text).size(16.0)),
    );
    if r.hovered() {
        // An accent underline on hover.
        let rule = r.rect.shrink2(egui::vec2(40.0, 6.0));
        ui.painter().line_segment(
            [rule.left_bottom(), rule.right_bottom()],
            egui::Stroke::new(1.0, ACCENT),
        );
        ui.ctx()
            .data_mut(|d| d.insert_temp(egui::Id::new("ui_hovered"), Some(r.id)));
    }
    if r.clicked() {
        ui.ctx()
            .data_mut(|d| d.insert_temp(egui::Id::new("ui_clicked"), true));
    }
    r
}

/// Plays hover/click sounds for [`menu_button`]s, and draws the fade-in
/// that smooths every screen change.
pub fn ui_sounds_and_fade(
    mut contexts: EguiContexts,
    mut sfx: MessageWriter<crate::audio::Sfx>,
    mut last: Local<Option<egui::Id>>,
    state: Res<State<crate::AppState>>,
    mut fade: Local<f32>,
    time: Res<Time<Real>>,
    config: Res<crate::settings::Config>,
    window: Single<&Window, With<bevy::window::PrimaryWindow>>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    // The HUD needs about 1000×620 logical points; small windows scale the
    // UI down rather than letting panels overlap.
    let fit = (window.width() / 1000.0).min(window.height() / 620.0);
    let scale = config.settings.ui_scale.min(fit).clamp(0.5, 3.0);
    if (ctx.zoom_factor() - scale).abs() > 0.001 {
        ctx.set_zoom_factor(scale);
    }
    let (hovered, clicked) = ctx.data_mut(|d| {
        (
            d.remove_temp::<Option<egui::Id>>(egui::Id::new("ui_hovered"))
                .flatten(),
            d.remove_temp::<bool>(egui::Id::new("ui_clicked")),
        )
    });
    if hovered.is_some() && hovered != *last {
        sfx.write(crate::audio::Sfx::ui("ui_hover"));
    }
    *last = hovered;
    if clicked == Some(true) {
        sfx.write(crate::audio::Sfx::ui("ui_click"));
    }

    if state.is_changed() {
        *fade = 1.0;
    }
    if *fade > 0.0 {
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Foreground, egui::Id::new("fade")));
        painter.rect_filled(
            ctx.viewport_rect(),
            0.0,
            egui::Color32::from_black_alpha((*fade * 255.0) as u8),
        );
        *fade = (*fade - time.delta_secs() * 2.5).max(0.0);
    }
    Ok(())
}

/// A framed panel: double border with bracketed, overgrown corners.
pub fn ornate<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> egui::InnerResponse<R> {
    let r = panel_frame().show(ui, add);
    decorate(ui.painter(), ui.ctx(), r.response.rect);
    r
}

/// Paints the inner rule and corner ornaments around a panel rectangle.
pub fn decorate(painter: &egui::Painter, ctx: &egui::Context, rect: egui::Rect) {
    painter.rect_stroke(
        rect.shrink(4.0),
        0.0,
        egui::Stroke::new(1.0, RULE),
        egui::StrokeKind::Inside,
    );
    let Some(tex) = ctx.data(|d| d.get_temp::<egui::TextureId>(egui::Id::new("ornaments"))) else {
        return;
    };
    // The corner piece is the top-left 16x16 of the 96x48 sheet; mirror it.
    let (u, v) = (16.0 / 96.0, 16.0 / 48.0);
    let size = egui::vec2(16.0, 16.0);
    let corners = [
        (
            rect.left_top(),
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(u, v)),
        ),
        (
            rect.right_top() - egui::vec2(size.x, 0.0),
            egui::Rect::from_min_max(egui::pos2(u, 0.0), egui::pos2(0.0, v)),
        ),
        (
            rect.left_bottom() - egui::vec2(0.0, size.y),
            egui::Rect::from_min_max(egui::pos2(0.0, v), egui::pos2(u, 0.0)),
        ),
        (
            rect.right_bottom() - size,
            egui::Rect::from_min_max(egui::pos2(u, v), egui::pos2(0.0, 0.0)),
        ),
    ];
    for (at, uv) in corners {
        painter.image(tex, egui::Rect::from_min_size(at, size), uv, egui::Color32::WHITE);
    }
}

/// A divider flourish, centred in the available width.
pub fn flourish(ui: &mut egui::Ui) {
    let Some(tex) = ui
        .ctx()
        .data(|d| d.get_temp::<egui::TextureId>(egui::Id::new("ornaments")))
    else {
        ui.separator();
        return;
    };
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width().min(256.0), 16.0),
        egui::Sense::hover(),
    );
    let uv = egui::Rect::from_min_max(egui::pos2(32.0 / 96.0, 0.0), egui::pos2(64.0 / 96.0, 8.0 / 48.0));
    let r = egui::Rect::from_center_size(rect.center(), egui::vec2(96.0, 24.0));
    ui.painter().image(tex, r, uv, egui::Color32::WHITE);
}

/// A framed panel in the theme's style.
pub fn panel_frame() -> egui::Frame {
    egui::Frame::new()
        .fill(PANEL)
        .stroke(egui::Stroke::new(2.0, BORDER))
        .inner_margin(egui::Margin::same(10))
        .shadow(egui::Shadow {
            offset: [4, 4],
            blur: 0,
            spread: 0,
            color: egui::Color32::from_black_alpha(120),
        })
}

/// A labelled meter bar.
pub fn meter(ui: &mut egui::Ui, value: f32, color: egui::Color32, width: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 10.0), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 0.0, egui::Color32::from_rgb(6, 11, 12));
    let mut fill = rect.shrink(2.0);
    fill.set_width(fill.width() * value.clamp(0.0, 1.0));
    p.rect_filled(fill, 0.0, color);
    p.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0, BORDER),
        egui::StrokeKind::Inside,
    );
}

/// A body-mod set's icon from the 16-column, 4-row sheet.
#[allow(dead_code)] // Used once body mods land.
pub fn mod_icon(
    ui: &mut egui::Ui,
    icons: &UiIcons,
    set: usize,
    size: f32,
    tint: egui::Color32,
) -> egui::Response {
    let (x, y) = ((set % 16) as f32, (set / 16) as f32);
    let uv = egui::Rect::from_min_max(
        egui::pos2(x / 16.0, y / 4.0),
        egui::pos2((x + 1.0) / 16.0, (y + 1.0) / 4.0),
    );
    ui.add(
        egui::Image::new(egui::load::SizedTexture::new(icons.mods, egui::vec2(size, size)))
            .uv(uv)
            .tint(tint),
    )
}

/// Paints an item-sheet icon into a rectangle.
pub fn paint_icon(
    painter: &egui::Painter,
    icons: &UiIcons,
    index: usize,
    rect: egui::Rect,
    tint: egui::Color32,
) {
    painter.image(icons.items, rect, icon_uv(index), tint);
}

/// A colour for a hue on the 0–255 wheel, used to tint seeds and crops.
pub fn hue_color(hue: u8, sat: f32, val: f32) -> egui::Color32 {
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
    egui::Color32::from_rgb(
        ((r + m) * 255.0) as u8,
        ((g + m) * 255.0) as u8,
        ((b + m) * 255.0) as u8,
    )
}
