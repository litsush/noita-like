//! Shared egui look: the generated pixel font, a sharp-cornered dark theme
//! with warm accents, and helpers to draw sprite-sheet icons.

use std::sync::Arc;

use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiTextureHandle, egui};

use crate::assets::{GameAssets, assets_dir};

pub const ACCENT: egui::Color32 = egui::Color32::from_rgb(240, 168, 64);
pub const ACCENT_DIM: egui::Color32 = egui::Color32::from_rgb(150, 96, 40);
pub const TEXT: egui::Color32 = egui::Color32::from_rgb(232, 224, 214);
pub const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(150, 140, 150);
pub const PANEL: egui::Color32 = egui::Color32::from_rgba_premultiplied(18, 14, 24, 236);
pub const BORDER: egui::Color32 = egui::Color32::from_rgb(78, 60, 92);
pub const DANGER: egui::Color32 = egui::Color32::from_rgb(232, 80, 72);
pub const GOOD: egui::Color32 = egui::Color32::from_rgb(120, 220, 140);

/// egui texture ids for sprite sheets drawn in the UI.
#[derive(Resource, Clone, Copy)]
pub struct UiIcons {
    pub items: egui::TextureId,
    /// The 8-frame core animation strip.
    pub core: egui::TextureId,
}

/// Sets up fonts, theme and icon textures once.
pub fn setup_ui(
    mut contexts: EguiContexts,
    assets: Res<GameAssets>,
    mut commands: Commands,
    mut done: Local<bool>,
) -> Result {
    if *done {
        return Ok(());
    }
    let items = contexts.add_image(EguiTextureHandle::Strong(assets.items.image.clone()));
    let core = contexts.add_image(EguiTextureHandle::Strong(assets.core.image.clone()));
    let ctx = contexts.ctx_mut()?;
    install_font(ctx);
    ctx.all_styles_mut(apply_theme);
    commands.insert_resource(UiIcons { items, core });
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
    v.panel_fill = egui::Color32::from_rgb(14, 11, 18);
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
    v.extreme_bg_color = egui::Color32::from_rgb(10, 8, 14);
    for (w, fill, stroke) in [
        (
            &mut v.widgets.noninteractive,
            egui::Color32::from_rgb(28, 22, 36),
            BORDER,
        ),
        (
            &mut v.widgets.inactive,
            egui::Color32::from_rgb(44, 34, 56),
            BORDER,
        ),
        (
            &mut v.widgets.hovered,
            egui::Color32::from_rgb(74, 54, 86),
            ACCENT_DIM,
        ),
        (&mut v.widgets.active, ACCENT_DIM, ACCENT),
        (
            &mut v.widgets.open,
            egui::Color32::from_rgb(60, 44, 72),
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
fn icon_uv(index: usize) -> egui::Rect {
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

/// An icon tinted (e.g. black for locked silhouettes).
pub fn icon_tinted(
    ui: &mut egui::Ui,
    icons: &UiIcons,
    index: usize,
    size: f32,
    tint: egui::Color32,
) -> egui::Response {
    ui.add(
        egui::Image::new(egui::load::SizedTexture::new(icons.items, egui::vec2(size, size)))
            .uv(icon_uv(index))
            .tint(tint),
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
    save: Res<crate::run::save::SaveData>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let scale = save.settings.ui_scale.clamp(0.5, 3.0);
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
    p.rect_filled(rect, 0.0, egui::Color32::from_rgb(10, 8, 14));
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

/// A full-screen root `Ui` for panels that cover the window.
pub fn screen_ui(ctx: &egui::Context) -> egui::Ui {
    egui::Ui::new(
        ctx.clone(),
        "screen".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    )
}

/// Format seconds as m:ss.
pub fn clock(secs: f32) -> String {
    let s = secs.max(0.0) as u32;
    format!("{}:{:02}", s / 60, s % 60)
}
