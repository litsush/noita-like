//! The look of every screen: pixel fonts at whole multiples of their grid,
//! square corners, two-pixel borders and one palette, so the interface sits
//! in the same world as the cells behind it.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

// Palette: night sky, bone text, amber, acid, blood.
pub const BG: egui::Color32 = egui::Color32::from_rgb(10, 12, 18);
pub const PANEL: egui::Color32 = egui::Color32::from_rgb(19, 23, 32);
pub const RAISED: egui::Color32 = egui::Color32::from_rgb(30, 37, 50);
pub const BORDER: egui::Color32 = egui::Color32::from_rgb(78, 94, 112);
pub const INK: egui::Color32 = egui::Color32::from_rgb(230, 222, 198);
pub const DIM: egui::Color32 = egui::Color32::from_rgb(132, 142, 154);
pub const GOLD: egui::Color32 = egui::Color32::from_rgb(242, 182, 72);
pub const ACID: egui::Color32 = egui::Color32::from_rgb(146, 226, 98);
pub const BLOOD: egui::Color32 = egui::Color32::from_rgb(255, 92, 80);
pub const SKY: egui::Color32 = egui::Color32::from_rgb(94, 184, 222);
pub const SHADOW: egui::Color32 = egui::Color32::from_rgb(4, 5, 8);

/// Font families beyond egui's defaults.
pub const PIXEL: &str = "pixel";
pub const SILK: &str = "silk";

pub fn pixel(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name(PIXEL.into()))
}

pub fn silk(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name(SILK.into()))
}

pub fn body(size: f32) -> egui::FontId {
    egui::FontId::proportional(size)
}

/// Installs the fonts and the style on an egui context.
pub fn apply(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        PIXEL.into(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../../../assets/fonts/PressStart2P-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        SILK.into(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../../../assets/fonts/Silkscreen-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        "vt".into(),
        std::sync::Arc::new(egui::FontData::from_static(include_bytes!(
            "../../../assets/fonts/VT323-Regular.ttf"
        ))),
    );
    fonts
        .families
        .insert(egui::FontFamily::Proportional, vec!["vt".into(), SILK.into()]);
    fonts
        .families
        .insert(egui::FontFamily::Monospace, vec!["vt".into()]);
    fonts.families.insert(
        egui::FontFamily::Name(PIXEL.into()),
        vec![PIXEL.into(), "vt".into()],
    );
    fonts.families.insert(
        egui::FontFamily::Name(SILK.into()),
        vec![SILK.into(), "vt".into()],
    );
    ctx.set_fonts(fonts);

    ctx.all_styles_mut(|style| {
        use egui::TextStyle::*;
        style.text_styles = [
            (Small, body(16.0)),
            (Body, body(20.0)),
            (Monospace, body(20.0)),
            (Button, silk(16.0)),
            (Heading, pixel(16.0)),
        ]
        .into();
        style.spacing.item_spacing = egui::vec2(8.0, 6.0);
        style.spacing.button_padding = egui::vec2(12.0, 6.0);
        style.spacing.interact_size = egui::vec2(40.0, 28.0);
        style.spacing.window_margin = egui::Margin::same(12);
        style.spacing.menu_margin = egui::Margin::same(8);
        style.spacing.indent = 16.0;
        style.spacing.slider_width = 160.0;
        style.spacing.text_edit_width = 200.0;

        let v = &mut style.visuals;
        *v = egui::Visuals::dark();
        v.override_text_color = Some(INK);
        v.weak_text_color = Some(DIM);
        v.panel_fill = PANEL;
        v.window_fill = BG;
        v.extreme_bg_color = SHADOW;
        v.faint_bg_color = RAISED;
        v.code_bg_color = SHADOW;
        v.hyperlink_color = SKY;
        v.window_shadow = egui::Shadow::NONE;
        v.popup_shadow = egui::Shadow::NONE;
        v.window_stroke = egui::Stroke::new(2.0, BORDER);
        v.window_corner_radius = egui::CornerRadius::ZERO;
        v.menu_corner_radius = egui::CornerRadius::ZERO;
        v.selection.bg_fill = GOLD.gamma_multiply(0.45);
        v.selection.stroke = egui::Stroke::new(2.0, GOLD);
        v.striped = false;
        v.slider_trailing_fill = true;
        let widget =
            |fill: egui::Color32, stroke: egui::Color32, text: egui::Color32| egui::style::WidgetVisuals {
                bg_fill: fill,
                weak_bg_fill: fill,
                bg_stroke: egui::Stroke::new(2.0, stroke),
                corner_radius: egui::CornerRadius::ZERO,
                fg_stroke: egui::Stroke::new(2.0, text),
                expansion: 0.0,
            };
        v.widgets.noninteractive = widget(PANEL, BORDER, INK);
        v.widgets.inactive = widget(RAISED, BORDER, INK);
        v.widgets.hovered = widget(RAISED, GOLD, INK);
        v.widgets.active = widget(GOLD, GOLD, SHADOW);
        v.widgets.open = widget(RAISED, GOLD, INK);
        v.widgets.noninteractive.bg_stroke = egui::Stroke::new(2.0, BORDER);
    });
    // Hard edges: no anti-aliased feathering on shapes.
    ctx.tessellation_options_mut(|t| {
        t.feathering = false;
    });
}

/// Whether the fonts are installed. New fonts only take effect on the
/// pass after they're set, so screens wait a frame before drawing.
#[derive(Resource, Default)]
pub struct ThemeReady(pub u8);

/// Applies the theme once the egui context exists.
pub fn setup(mut contexts: EguiContexts, mut state: ResMut<ThemeReady>) -> Result {
    match state.0 {
        0 => {
            let ctx = contexts.ctx_mut()?;
            apply(ctx);
            ctx.request_repaint();
            state.0 = 1;
        }
        1 => state.0 = 2,
        _ => {}
    }
    Ok(())
}

pub fn ready(state: Res<ThemeReady>) -> bool {
    state.0 >= 2
}

/// A label in the pixel display face.
pub fn title(text: &str, size: f32, colour: egui::Color32) -> egui::RichText {
    egui::RichText::new(text).font(pixel(size)).color(colour)
}

/// A two-tone bar: `frac` of it filled.
pub fn bar(ui: &mut egui::Ui, size: egui::Vec2, frac: f32, fill: egui::Color32, right_to_left: bool) {
    let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 0.0, SHADOW);
    p.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(2.0, BORDER),
        egui::StrokeKind::Inside,
    );
    let inner = rect.shrink(2.0);
    let w = (inner.width() * frac.clamp(0.0, 1.0)).round();
    let mut filled = inner;
    if right_to_left {
        filled.min.x = inner.max.x - w;
    } else {
        filled.set_width(w);
    }
    p.rect_filled(filled, 0.0, fill);
}

/// A square button of the given size with a pixel-face label.
pub fn big_button(ui: &mut egui::Ui, text: &str, size: egui::Vec2, fill: egui::Color32) -> egui::Response {
    ui.add_sized(
        size,
        egui::Button::new(egui::RichText::new(text).font(pixel(12.0)).color(INK)).fill(fill),
    )
}
