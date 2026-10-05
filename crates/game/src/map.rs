//! The map: a picture of the explored world with markers for domes, robots,
//! teammates and pins. Robot areas are marked here by dragging a box.
//!
//! Fog of war is kept per player and per world on the player's own machine;
//! what teammates see around themselves is revealed for everyone online.

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy_egui::{EguiContexts, EguiTextureHandle, egui};
use sbct_sim::Material;
use sbct_sim::colony::actions::Action as Act;
use sbct_sim::colony::geom::{Rect as CellRect, V2, v2};
use sbct_sim::colony::items::MachineKind;
use sbct_sim::colony::modfx::{PinOp, Teleport};
use sbct_sim::colony::robots::AreaOp;
use sbct_sim::colony::{Colony, EntKind, Id, Player, PlayerKey};

use crate::panels::{Open, Panels};
use crate::session::Session;
use crate::ui::{self, ACCENT, TEXT, TEXT_DIM, WARN, team_color};

/// Cells per map texel, and per fog cell.
const SCALE: usize = 4;
const FOG: usize = 8;
/// How far a player sees for the map, in cells.
const REVEAL: f32 = 110.0;

#[derive(Clone, Copy, PartialEq, Default)]
enum Mode {
    #[default]
    Browse,
    /// Dragging out a robot area.
    Area,
    /// Clicking to drop a pin.
    Pin,
    /// Clicking to send the free-flying arm.
    Arm,
}

#[derive(Resource, Default)]
pub struct MapView {
    image: Option<Handle<Image>>,
    texture: Option<egui::TextureId>,
    /// Size in texels.
    size: (usize, usize),
    fog: Vec<bool>,
    fog_size: (usize, usize),
    /// The world and player the fog belongs to.
    owner: Option<(u64, PlayerKey)>,
    fog_changed: bool,
    save_in: f32,
    /// Next texture row to refresh, and whether a full redraw is due.
    row: usize,
    redraw: bool,
    was_open: bool,
    /// View centre in cells and zoom in screen points per cell.
    center: Vec2,
    zoom: f32,
    mode: Mode,
    drag_from: Option<V2>,
    pending_area: Option<CellRect>,
    pending_pin: Option<V2>,
    name: String,
}

fn fog_path(owner: (u64, PlayerKey)) -> std::path::PathBuf {
    crate::settings::data_dir()
        .join("maps")
        .join(format!("{:016x}-{:016x}.fog", owner.0, owner.1))
}

impl MapView {
    fn load_fog(&mut self, owner: (u64, PlayerKey), w: usize, h: usize) {
        self.fog_size = (w.div_ceil(FOG), h.div_ceil(FOG));
        let n = self.fog_size.0 * self.fog_size.1;
        self.fog = vec![false; n];
        if let Ok(bytes) = std::fs::read(fog_path(owner))
            && bytes.len() == n.div_ceil(8)
        {
            for (i, seen) in self.fog.iter_mut().enumerate() {
                *seen = bytes[i / 8] & (1 << (i % 8)) != 0;
            }
        }
        self.owner = Some(owner);
        self.fog_changed = false;
        self.redraw = true;
    }

    fn save_fog(&mut self) {
        let Some(owner) = self.owner else { return };
        if !self.fog_changed {
            return;
        }
        let mut bytes = vec![0u8; self.fog.len().div_ceil(8)];
        for (i, seen) in self.fog.iter().enumerate() {
            if *seen {
                bytes[i / 8] |= 1 << (i % 8);
            }
        }
        let path = fog_path(owner);
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if std::fs::write(&path, bytes).is_ok() {
            self.fog_changed = false;
        }
    }

    pub fn seen(&self, p: V2) -> bool {
        let (x, y) = ((p.x as usize) / FOG, (p.y.max(0.0) as usize) / FOG);
        x < self.fog_size.0 && y < self.fog_size.1 && self.fog[y * self.fog_size.0 + x]
    }

    fn reveal(&mut self, at: V2, radius: f32) {
        let (fw, fh) = self.fog_size;
        let r = (radius / FOG as f32).ceil() as i32;
        let (cx, cy) = ((at.x / FOG as f32) as i32, (at.y / FOG as f32) as i32);
        for dy in -r..=r {
            for dx in -r..=r {
                let (x, y) = (cx + dx, cy + dy);
                if dx * dx + dy * dy > r * r || x < 0 || y < 0 || x >= fw as i32 || y >= fh as i32 {
                    continue;
                }
                let i = y as usize * fw + x as usize;
                if !self.fog[i] {
                    self.fog[i] = true;
                    self.fog_changed = true;
                }
            }
        }
    }
}

/// Reveals the map around everyone online and keeps the fog file fresh.
pub fn explore(mut map: ResMut<MapView>, session: Res<Session>, time: Res<Time>) {
    let (Some(colony), Some(world), Some(me)) = (&session.colony, &session.world, session.player()) else {
        return;
    };
    let owner = (colony.seed, me.key);
    if map.owner != Some(owner) {
        map.save_fog();
        map.load_fog(owner, world.width(), world.height());
        map.center = Vec2::new(me.pose.pos.x, me.pose.pos.y);
        map.zoom = 0.0;
    }
    let stats = me.stats();
    if stats.map_all {
        if map.fog.iter().any(|s| !s) {
            map.fog.fill(true);
            map.fog_changed = true;
            map.redraw = true;
        }
    } else {
        let radius = REVEAL * stats.map_reveal;
        for p in colony.online() {
            let r = if p.key == me.key { radius } else { REVEAL };
            map.reveal(p.center(), r);
        }
        // The free arm sees for its owner.
        if let Some(arm) = me.gear.free_arm {
            map.reveal(arm, 50.0);
        }
    }
    map.save_in -= time.delta_secs();
    if map.save_in <= 0.0 {
        map.save_in = 45.0;
        map.save_fog();
    }
}

pub fn reset_map(mut map: ResMut<MapView>) {
    map.save_fog();
    map.owner = None;
    map.fog.clear();
    map.mode = Mode::Browse;
    map.pending_area = None;
    map.pending_pin = None;
}

fn texel(colony: &Colony, world: &sbct_sim::World, x: i32, y: i32, ore: bool) -> [u8; 4] {
    let m = world.material(x, y);
    let [r, g, b] = match m {
        Material::Empty | Material::Smoke | Material::Steam | Material::Spore => {
            if y < colony.profile.surface_at(x) {
                [44, 66, 76]
            } else {
                [20, 24, 28]
            }
        }
        m if m.is_ore() => {
            if ore {
                // Antenna Array 5G: veins stand out.
                let [r, g, b] = m.props().color;
                [r.saturating_add(70), g.saturating_add(70), b.saturating_add(70)]
            } else {
                // Without it, ore reads as the rock around it.
                Material::Stone.props().color.map(|c| (c as f32 * 0.72) as u8)
            }
        }
        m => m.props().color.map(|c| (c as f32 * 0.72) as u8),
    };
    [r, g, b, 255]
}

/// Keeps the map texture up to date while the map is open.
pub fn refresh(
    mut map: ResMut<MapView>,
    panels: Res<Panels>,
    session: Res<Session>,
    mut images: ResMut<Assets<Image>>,
) {
    let open = panels.open == Some(Open::Map);
    let (Some(colony), Some(world), Some(me)) = (&session.colony, &session.world, session.player()) else {
        return;
    };
    if !open {
        map.was_open = false;
        return;
    }
    let size = (world.width() / SCALE, world.height() / SCALE);
    if map.image.is_none() || map.size != size {
        let mut image = Image::new_fill(
            Extent3d {
                width: size.0 as u32,
                height: size.1 as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            &[6, 10, 12, 255],
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
        );
        image.sampler = ImageSampler::nearest();
        map.image = Some(images.add(image));
        map.texture = None;
        map.size = size;
        map.redraw = true;
    }
    // Everything when the map opens; after that a band of rows per frame.
    let rows = if map.redraw || !map.was_open { size.1 } else { 24 };
    if !map.was_open {
        map.center = Vec2::new(me.pose.pos.x, me.pose.pos.y);
    }
    map.was_open = true;
    map.redraw = false;
    let ore = me.stats().map_ore;
    let handle = map.image.clone().unwrap();
    let Some(mut image) = images.get_mut(&handle) else {
        return;
    };
    let Some(data) = image.data.as_mut() else { return };
    for _ in 0..rows {
        let ty = map.row;
        map.row = (map.row + 1) % size.1;
        for tx in 0..size.0 {
            let (x, y) = ((tx * SCALE + SCALE / 2) as i32, (ty * SCALE + SCALE / 2) as i32);
            let px = if map.seen(v2(x as f32, y as f32)) {
                texel(colony, world, x, y, ore)
            } else {
                [6, 10, 12, 255]
            };
            let i = (ty * size.0 + tx) * 4;
            data[i..i + 4].copy_from_slice(&px);
        }
    }
}

/// Registers the map texture with egui once it exists.
pub fn prepare(mut map: ResMut<MapView>, mut contexts: EguiContexts) {
    if map.texture.is_none()
        && let Some(handle) = map.image.clone()
    {
        map.texture = Some(contexts.add_image(EguiTextureHandle::Strong(handle)));
    }
}

fn label(painter: &egui::Painter, at: egui::Pos2, text: &str, color: egui::Color32) {
    let galley = painter.layout_no_wrap(text.to_string(), egui::FontId::proportional(13.0), color);
    let rect = egui::Rect::from_min_size(
        at + egui::vec2(-galley.size().x / 2.0, 6.0),
        galley.size() + egui::vec2(6.0, 2.0),
    );
    painter.rect_filled(rect, 2.0, egui::Color32::from_black_alpha(170));
    painter.galley(rect.min + egui::vec2(3.0, 1.0), galley, color);
}

/// The map window. Returns actions for the host.
pub fn window(
    ctx: &egui::Context,
    map: &mut MapView,
    colony: &Colony,
    me: &Player,
    panels: &mut Panels,
    acts: &mut Vec<Act>,
    close: &mut bool,
) {
    let (w, h) = (colony.profile.width as f32, colony.profile.height as f32);
    let stats = me.stats();
    let screen = ctx.content_rect();
    let mut open_robot = None;
    egui::Area::new("map".into())
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            ui::ornate(ui, |ui| {
                let size = egui::vec2(
                    (screen.width() - 60.0).min(1500.0),
                    (screen.height() - 70.0).min(900.0),
                );
                ui.set_width(size.x);
                ui.set_height(size.y);
                ui.horizontal(|ui| {
                    // ---- the map itself ----
                    let map_size = egui::vec2(size.x - 270.0, size.y);
                    let (rect, response) = ui.allocate_exact_size(map_size, egui::Sense::click_and_drag());
                    if map.zoom <= 0.0 {
                        map.zoom = (map_size.x / 900.0).max(0.6);
                    }
                    let min_zoom = (map_size.x / w).min(map_size.y / h);
                    // Zoom about the cursor.
                    if let Some(pointer) = response.hover_pos() {
                        let scroll = ui.input(|i| i.smooth_scroll_delta.y);
                        if scroll != 0.0 {
                            let before = map.center + (pointer - rect.center()).to_bevy() / map.zoom;
                            map.zoom = (map.zoom * (scroll * 0.004).exp()).clamp(min_zoom, 6.0);
                            map.center = before - (pointer - rect.center()).to_bevy() / map.zoom;
                        }
                    }
                    let panning = response.dragged_by(egui::PointerButton::Secondary)
                        || response.dragged_by(egui::PointerButton::Middle)
                        || (map.mode == Mode::Browse && response.dragged_by(egui::PointerButton::Primary));
                    if panning {
                        map.center -= response.drag_delta().to_bevy() / map.zoom;
                    }
                    map.center = map.center.clamp(Vec2::ZERO, Vec2::new(w, h));
                    let (center, zoom) = (map.center, map.zoom);
                    let to_screen = |p: V2| rect.center() + egui::vec2(p.x - center.x, p.y - center.y) * zoom;
                    let to_cell = |p: egui::Pos2| {
                        let d = (p - rect.center()) / zoom;
                        v2(center.x + d.x, center.y + d.y)
                    };
                    let painter = ui.painter().with_clip_rect(rect);
                    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(6, 10, 12));
                    if let Some(texture) = map.texture {
                        let world_rect =
                            egui::Rect::from_min_max(to_screen(v2(0.0, 0.0)), to_screen(v2(w, h)));
                        painter.image(
                            texture,
                            world_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }

                    // Robot areas.
                    for area in &colony.areas {
                        let r = egui::Rect::from_min_max(
                            to_screen(v2(area.rect.x0 as f32, area.rect.y0 as f32)),
                            to_screen(v2(area.rect.x1 as f32, area.rect.y1 as f32)),
                        );
                        painter.rect_filled(r, 0.0, egui::Color32::from_rgba_unmultiplied(240, 184, 84, 28));
                        painter.rect_stroke(r, 0.0, egui::Stroke::new(1.5, WARN), egui::StrokeKind::Inside);
                        painter.text(
                            r.min + egui::vec2(4.0, 2.0),
                            egui::Align2::LEFT_TOP,
                            &area.name,
                            egui::FontId::proportional(13.0),
                            WARN,
                        );
                    }
                    // Domes and machines the colony has built are always known.
                    let hover = response.hover_pos();
                    let near = |at: egui::Pos2| hover.is_some_and(|h| h.distance(at) < 12.0);
                    for e in colony.ents.values() {
                        match &e.kind {
                            EntKind::Dome(d) => {
                                let (dw, dh) = d.kind.size();
                                let r = egui::Rect::from_min_max(
                                    to_screen(v2(e.pos.x - dw as f32 / 2.0, e.pos.y - dh as f32)),
                                    to_screen(e.pos + v2(dw as f32 / 2.0, 0.0)),
                                );
                                let color = egui::Color32::from_rgb(120, 220, 190);
                                painter.rect_filled(
                                    r,
                                    3.0,
                                    egui::Color32::from_rgba_unmultiplied(120, 220, 190, 30),
                                );
                                painter.rect_stroke(
                                    r,
                                    3.0,
                                    egui::Stroke::new(1.0, color),
                                    egui::StrokeKind::Inside,
                                );
                                if zoom > 0.7 || near(r.center()) {
                                    label(&painter, r.center_bottom(), &d.name, color);
                                }
                            }
                            EntKind::Machine(m) => {
                                let at = to_screen(e.pos);
                                let color = match m.kind {
                                    MachineKind::Pylon => ui::POWER,
                                    MachineKind::Pump | MachineKind::Tank => ui::WATER,
                                    _ => TEXT_DIM,
                                };
                                painter.rect_filled(
                                    egui::Rect::from_center_size(at, egui::vec2(4.0, 4.0)),
                                    0.0,
                                    color,
                                );
                                if near(at) {
                                    label(&painter, at, &m.name, color);
                                }
                            }
                            EntKind::Robot(r) => {
                                let at = to_screen(e.pos);
                                let color = team_color(r.color);
                                painter.add(egui::Shape::convex_polygon(
                                    vec![
                                        at + egui::vec2(0.0, -5.0),
                                        at + egui::vec2(5.0, 0.0),
                                        at + egui::vec2(0.0, 5.0),
                                        at + egui::vec2(-5.0, 0.0),
                                    ],
                                    color,
                                    egui::Stroke::new(1.0, egui::Color32::BLACK),
                                ));
                                if zoom > 1.6 || near(at) {
                                    label(&painter, at, &format!("{}: {}", r.name, r.status), color);
                                }
                            }
                            _ => {}
                        }
                    }
                    // Pins and pings.
                    let t = ui.input(|i| i.time) as f32;
                    for pin in &colony.pins {
                        let at = to_screen(pin.pos);
                        let color = team_color(pin.color);
                        if pin.ping.is_some() {
                            let pulse = 6.0 + 6.0 * (t * 3.0).sin().abs();
                            painter.circle_stroke(at, pulse, egui::Stroke::new(2.0, color));
                        }
                        painter
                            .line_segment([at, at + egui::vec2(0.0, -12.0)], egui::Stroke::new(2.0, color));
                        painter.add(egui::Shape::convex_polygon(
                            vec![
                                at + egui::vec2(0.0, -12.0),
                                at + egui::vec2(9.0, -9.0),
                                at + egui::vec2(0.0, -6.0),
                            ],
                            color,
                            egui::Stroke::NONE,
                        ));
                        label(&painter, at, &pin.name, color);
                    }
                    // Everyone online.
                    for p in colony.online() {
                        let at = to_screen(p.center());
                        let color = team_color(p.color);
                        painter.circle_filled(at, 5.0, color);
                        painter.circle_stroke(at, 5.0, egui::Stroke::new(1.5, egui::Color32::BLACK));
                        if p.key == me.key {
                            painter.circle_stroke(at, 8.0 + (t * 4.0).sin(), egui::Stroke::new(1.0, color));
                        }
                        label(&painter, at, &p.name, color);
                    }
                    if let Some(arm) = me.gear.free_arm {
                        let at = to_screen(arm);
                        painter.circle_stroke(at, 6.0, egui::Stroke::new(2.0, ACCENT));
                        label(&painter, at, "Your arm", ACCENT);
                    }

                    // Tools.
                    let pointer_cell = response.interact_pointer_pos().or(hover).map(to_cell);
                    match map.mode {
                        Mode::Area => {
                            if response.drag_started_by(egui::PointerButton::Primary) {
                                map.drag_from = pointer_cell;
                            }
                            if let (Some(from), Some(to)) = (map.drag_from, pointer_cell) {
                                let r = egui::Rect::from_two_pos(to_screen(from), to_screen(to));
                                painter.rect_stroke(
                                    r,
                                    0.0,
                                    egui::Stroke::new(2.0, WARN),
                                    egui::StrokeKind::Inside,
                                );
                                if response.drag_stopped_by(egui::PointerButton::Primary) {
                                    map.pending_area = Some(CellRect::new(
                                        from.x as i32,
                                        from.y as i32,
                                        to.x as i32,
                                        to.y as i32,
                                    ));
                                    map.drag_from = None;
                                    map.name.clear();
                                }
                            }
                        }
                        Mode::Pin => {
                            if response.clicked()
                                && let Some(at) = pointer_cell
                            {
                                map.pending_pin = Some(at);
                                map.name.clear();
                            }
                        }
                        Mode::Arm => {
                            if response.clicked()
                                && let Some(at) = pointer_cell
                                && map.seen(at)
                            {
                                acts.push(Act::SendArm { at: Some(at) });
                                map.mode = Mode::Browse;
                            }
                        }
                        Mode::Browse => {}
                    }
                    if let Some(r) = map.pending_area {
                        let r = egui::Rect::from_min_max(
                            to_screen(v2(r.x0 as f32, r.y0 as f32)),
                            to_screen(v2(r.x1 as f32, r.y1 as f32)),
                        );
                        painter.rect_stroke(r, 0.0, egui::Stroke::new(2.0, ACCENT), egui::StrokeKind::Inside);
                    }
                    if let Some(p) = map.pending_pin {
                        painter.circle_stroke(to_screen(p), 6.0, egui::Stroke::new(2.0, ACCENT));
                    }
                    let hint = match map.mode {
                        Mode::Browse => "Drag to pan, scroll to zoom",
                        Mode::Area => "Drag a box over the place robots should work",
                        Mode::Pin => "Click where the pin goes",
                        Mode::Arm => "Click an explored spot to send your arm there",
                    };
                    painter.text(
                        rect.left_bottom() + egui::vec2(8.0, -6.0),
                        egui::Align2::LEFT_BOTTOM,
                        hint,
                        egui::FontId::proportional(14.0),
                        TEXT,
                    );

                    // ---- the side panel ----
                    ui.vertical(|ui| {
                        ui.set_width(250.0);
                        ui.horizontal(|ui| {
                            ui.label(ui::title("Map", 26.0).color(ACCENT));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("x").clicked() {
                                    *close = true;
                                }
                            });
                        });
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("Find me").clicked() {
                                map.center = Vec2::new(me.pose.pos.x, me.pose.pos.y);
                            }
                            let mut tool = |ui: &mut egui::Ui, mode: Mode, text: &str, tip: &str| {
                                if ui
                                    .selectable_label(map.mode == mode, text)
                                    .on_hover_text(tip)
                                    .clicked()
                                {
                                    map.mode = if map.mode == mode { Mode::Browse } else { mode };
                                    map.pending_area = None;
                                    map.pending_pin = None;
                                }
                            };
                            tool(ui, Mode::Pin, "+ Pin", "Mark a spot for the whole team");
                            tool(
                                ui,
                                Mode::Area,
                                "+ Area",
                                "Drag a box that robots can be told to mine, scavenge or guard",
                            );
                            if stats.free_arm {
                                tool(
                                    ui,
                                    Mode::Arm,
                                    "Send arm",
                                    "Send your free-flying arm to work somewhere",
                                );
                                if me.gear.free_arm.is_some() && ui.button("Recall arm").clicked() {
                                    acts.push(Act::SendArm { at: None });
                                }
                            }
                            if stats.beacon >= 2 && ui.button("Recall home").clicked() {
                                acts.push(Act::Teleport(Teleport::Home));
                                *close = true;
                            }
                        });
                        // Naming a new area or pin.
                        if map.pending_area.is_some() || map.pending_pin.is_some() {
                            ui.add_space(4.0);
                            let what = if map.pending_area.is_some() { "area" } else { "pin" };
                            ui.label(format!("Name the {what}:"));
                            let r = ui.add(
                                egui::TextEdit::singleline(&mut map.name)
                                    .desired_width(230.0)
                                    .char_limit(24),
                            );
                            r.request_focus();
                            let enter = ui.input(|i| i.key_pressed(egui::Key::Enter));
                            ui.horizontal(|ui| {
                                if ui.button("Save").clicked() || enter {
                                    let name = std::mem::take(&mut map.name);
                                    if let Some(rect) = map.pending_area.take() {
                                        acts.push(Act::Area(AreaOp::Add { name, rect }));
                                    } else if let Some(pos) = map.pending_pin.take() {
                                        acts.push(Act::Pin(PinOp::Add { pos, name }));
                                    }
                                    map.mode = Mode::Browse;
                                }
                                if ui.button("Cancel").clicked() {
                                    map.pending_area = None;
                                    map.pending_pin = None;
                                }
                            });
                        }
                        ui.add_space(4.0);
                        egui::ScrollArea::vertical().show(ui, |ui| {
                            ui.set_width(244.0);
                            let mut go = None;
                            let section = |ui: &mut egui::Ui, text: &str| {
                                ui.add_space(4.0);
                                ui.label(egui::RichText::new(text).color(TEXT_DIM));
                            };
                            section(ui, "TEAM");
                            for p in colony.online() {
                                ui.horizontal(|ui| {
                                    if ui
                                        .link(egui::RichText::new(&p.name).color(team_color(p.color)))
                                        .clicked()
                                    {
                                        go = Some(p.center());
                                    }
                                    let can = p.key != me.key && (stats.beacon >= 4 || p.stats().beacon >= 3);
                                    if can && ui.small_button("teleport").clicked() {
                                        acts.push(Act::Teleport(Teleport::Player(p.key)));
                                        *close = true;
                                    }
                                });
                            }
                            section(ui, "DOMES");
                            for (e, d) in colony.domes() {
                                ui.horizontal(|ui| {
                                    if ui.link(&d.name).clicked() {
                                        go = Some(e.pos);
                                    }
                                    if stats.beacon >= 4 && ui.small_button("teleport").clicked() {
                                        acts.push(Act::Teleport(Teleport::Dome(e.id)));
                                        *close = true;
                                    }
                                });
                            }
                            let robots: Vec<(Id, V2, String, u8)> = colony
                                .ents
                                .values()
                                .filter_map(|e| match &e.kind {
                                    EntKind::Robot(r) => {
                                        Some((e.id, e.pos, format!("{}: {}", r.name, r.status), r.color))
                                    }
                                    _ => None,
                                })
                                .collect();
                            if !robots.is_empty() {
                                section(ui, "ROBOTS");
                            }
                            for (id, pos, text, color) in robots {
                                ui.horizontal_wrapped(|ui| {
                                    if ui
                                        .link(egui::RichText::new(text).color(team_color(color)))
                                        .clicked()
                                    {
                                        go = Some(pos);
                                    }
                                    if stats.remote_robots && ui.small_button("program").clicked() {
                                        open_robot = Some(id);
                                    }
                                });
                            }
                            if !colony.areas.is_empty() {
                                section(ui, "ROBOT AREAS");
                            }
                            for area in &colony.areas {
                                ui.horizontal(|ui| {
                                    if ui.link(egui::RichText::new(&area.name).color(WARN)).clicked() {
                                        go = Some(area.rect.center());
                                    }
                                    if ui.small_button("remove").clicked() {
                                        acts.push(Act::Area(AreaOp::Remove(area.id)));
                                    }
                                });
                            }
                            if !colony.pins.is_empty() {
                                section(ui, "PINS");
                            }
                            for pin in &colony.pins {
                                ui.horizontal(|ui| {
                                    if ui
                                        .link(egui::RichText::new(&pin.name).color(team_color(pin.color)))
                                        .clicked()
                                    {
                                        go = Some(pin.pos);
                                    }
                                    if pin.ping.is_none() && ui.small_button("remove").clicked() {
                                        acts.push(Act::Pin(PinOp::Remove(pin.id)));
                                    }
                                });
                            }
                            if let Some(p) = go {
                                map.center = Vec2::new(p.x, p.y);
                            }
                        });
                    });
                });
            });
        });
    if let Some(id) = open_robot {
        panels.open = Some(Open::Robot(id));
    }
}

/// Beacon pings shown in the world: a marker where they are, or an arrow
/// at the edge of the screen pointing the way.
pub fn ping_markers(ctx: &egui::Context, colony: &Colony, to_screen: &dyn Fn(V2) -> Option<egui::Pos2>) {
    let screen = ctx.content_rect().shrink(40.0);
    let painter = ctx.layer_painter(egui::LayerId::background());
    let t = ctx.input(|i| i.time) as f32;
    for pin in colony.pins.iter().filter(|p| p.ping.is_some()) {
        let Some(at) = to_screen(pin.pos) else { continue };
        let color = team_color(pin.color);
        let clamped = egui::pos2(
            at.x.clamp(screen.min.x, screen.max.x),
            at.y.clamp(screen.min.y, screen.max.y),
        );
        let pulse = 7.0 + 5.0 * (t * 3.0).sin().abs();
        painter.circle_stroke(clamped, pulse, egui::Stroke::new(2.0, color));
        painter.circle_filled(clamped, 3.0, color);
        if clamped != at {
            // Off screen: point toward it.
            let dir = (at - clamped).normalized();
            painter.line_segment(
                [clamped + dir * 12.0, clamped + dir * 24.0],
                egui::Stroke::new(3.0, color),
            );
        }
        label(&painter, clamped, &pin.name, color);
    }
}

trait ToBevy {
    fn to_bevy(self) -> Vec2;
}

impl ToBevy for egui::Vec2 {
    fn to_bevy(self) -> Vec2 {
        Vec2::new(self.x, self.y)
    }
}
