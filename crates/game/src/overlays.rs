//! The Water and Power overlays: each network or grid outlined in its own
//! colour, labelled with its numbers, and its problems marked.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use sbct_sim::colony::domes::{footprint, issue};
use sbct_sim::colony::power::{LINK_RANGE, SUPPLY_RANGE};
use sbct_sim::colony::water::{TILE, tile_center};
use sbct_sim::colony::{EntKind, Id};

use crate::controls::Action;
use crate::panels::Panels;
use crate::render::WorldCamera;
use crate::session::Session;
use crate::settings::Config;
use crate::ui::{self, DANGER, GOOD, UiIcons, WARN};

#[derive(Resource, Default)]
pub struct Overlays {
    pub water: bool,
    pub power: bool,
}

pub fn toggle_overlays(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    config: Res<Config>,
    panels: Res<Panels>,
    mut overlays: ResMut<Overlays>,
) {
    if panels.blocks_movement() {
        return;
    }
    if config.bindings.just_pressed(Action::WaterOverlay, &keys, &mouse) {
        overlays.water = !overlays.water;
    }
    if config.bindings.just_pressed(Action::PowerOverlay, &keys, &mouse) {
        overlays.power = !overlays.power;
    }
}

fn hue(i: usize) -> Color {
    Color::hsl((i as f32 * 67.0 + 190.0) % 360.0, 0.75, 0.6)
}

fn egui_hue(i: usize) -> egui::Color32 {
    let c = hue(i).to_srgba();
    egui::Color32::from_rgb(
        (c.red * 255.0) as u8,
        (c.green * 255.0) as u8,
        (c.blue * 255.0) as u8,
    )
}

/// Lines and rings drawn in the world.
pub fn draw_overlay_shapes(
    session: Res<Session>,
    overlays: Res<Overlays>,
    mut gizmos: Gizmos,
    time: Res<Time>,
) {
    let Some(colony) = &session.colony else { return };
    let at = |e: Id| colony.ents.get(&e).map(|e| Vec2::new(e.pos.x, -e.pos.y + 8.0));
    if overlays.water {
        for (i, net) in colony.networks().iter().enumerate() {
            let color = hue(i);
            for &t in &net.tiles {
                let c = tile_center(t);
                gizmos.rect_2d(Vec2::new(c.x, -c.y), Vec2::splat(TILE as f32 + 1.0), color);
            }
            for &t in &net.dead_ends {
                let c = tile_center(t);
                gizmos.circle_2d(Vec2::new(c.x, -c.y), 5.0, Color::srgb(1.0, 0.7, 0.2));
            }
            // Tie each source and consumer to the network's colour.
            for &id in net.sources.iter().chain(&net.consumers).chain(&net.tanks) {
                if let Some(e) = colony.ents.get(&id) {
                    let (center, size) = match &e.kind {
                        EntKind::Dome(d) => {
                            let r = footprint(d.kind, e.pos);
                            (r.center(), Vec2::new((r.x1 - r.x0) as f32, (r.y1 - r.y0) as f32))
                        }
                        _ => (
                            sbct_sim::colony::geom::v2(e.pos.x, e.pos.y - 12.0),
                            Vec2::new(28.0, 28.0),
                        ),
                    };
                    gizmos.rect_2d(Vec2::new(center.x, -center.y), size + 4.0, color);
                }
            }
        }
    }
    if overlays.power {
        let pulse = 0.6 + 0.4 * (time.elapsed_secs() * 3.0).sin().abs();
        for (i, grid) in colony.grids().iter().enumerate() {
            let color = hue(i + 3);
            for (a, (_, pa)) in grid.pylons.iter().enumerate() {
                let pa2 = Vec2::new(pa.x, -pa.y + 16.0);
                gizmos.circle_2d(pa2, SUPPLY_RANGE, color.with_alpha(0.5));
                for (_, pb) in grid.pylons.iter().skip(a + 1) {
                    if pa.distance(*pb) <= LINK_RANGE {
                        gizmos.line_2d(pa2, Vec2::new(pb.x, -pb.y + 16.0), color);
                    }
                }
            }
            for &c in &grid.consumers {
                if let (Some(p), Some((_, first))) = (at(c), grid.pylons.first()) {
                    let nearest = grid
                        .pylons
                        .iter()
                        .map(|(_, q)| Vec2::new(q.x, -q.y + 16.0))
                        .min_by(|a, b| a.distance(p).total_cmp(&b.distance(p)))
                        .unwrap_or(Vec2::new(first.x, -first.y));
                    gizmos.line_2d(p, nearest, color.with_alpha(0.35 * pulse));
                }
            }
        }
    }
}

fn label(painter: &egui::Painter, at: egui::Pos2, lines: &[(String, egui::Color32)]) {
    let font = egui::FontId::proportional(16.0);
    let galleys: Vec<_> = lines
        .iter()
        .map(|(t, c)| painter.layout_no_wrap(t.clone(), font.clone(), *c))
        .collect();
    let w = galleys.iter().map(|g| g.size().x).fold(0.0, f32::max);
    let h: f32 = galleys.iter().map(|g| g.size().y).sum();
    let rect = egui::Rect::from_center_size(at, egui::vec2(w + 12.0, h + 8.0));
    painter.rect_filled(rect, 0.0, egui::Color32::from_black_alpha(215));
    painter.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0, ui::BORDER),
        egui::StrokeKind::Inside,
    );
    let mut y = rect.min.y + 4.0;
    for (g, (_, c)) in galleys.into_iter().zip(lines) {
        let height = g.size().y;
        painter.galley(egui::pos2(rect.min.x + 6.0, y), g, *c);
        y += height;
    }
}

/// Labels drawn over the world for the active overlays.
pub fn overlay_labels(
    mut contexts: EguiContexts,
    session: Res<Session>,
    overlays: Res<Overlays>,
    config: Res<Config>,
    icons: Option<Res<UiIcons>>,
    camera: Single<(&Camera, &GlobalTransform), With<WorldCamera>>,
) -> Result {
    if !overlays.water && !overlays.power {
        return Ok(());
    }
    let ctx = contexts.ctx_mut()?;
    let Some(colony) = &session.colony else {
        return Ok(());
    };
    let (cam, cam_tf) = *camera;
    let zoom = ctx.zoom_factor();
    let painter = ctx.layer_painter(egui::LayerId::background());
    let to_screen = |x: f32, y: f32| {
        cam.world_to_viewport(cam_tf, Vec3::new(x, -y, 0.0))
            .ok()
            .map(|p| egui::pos2(p.x / zoom, p.y / zoom))
    };

    if overlays.water {
        for (i, net) in colony.networks().iter().enumerate() {
            // Label above the network's middle tile.
            let mid = tile_center(net.tiles[net.tiles.len() / 2]);
            let Some(at) = to_screen(mid.x, mid.y - 14.0) else {
                continue;
            };
            let mut lines = vec![(
                format!("Network {}: {:.0} of {:.0} L/day", i + 1, net.supply, net.demand),
                egui_hue(i),
            )];
            if net.surplus() > 0.0 {
                lines.push((format!("Surplus {:.0} L/day", net.surplus()), GOOD));
            }
            for p in &net.problems {
                lines.push((p.describe().to_string(), WARN));
            }
            if !net.dead_ends.is_empty() {
                lines.push((format!("{} dead ends", net.dead_ends.len()), WARN));
            }
            label(&painter, at, &lines);
        }
        // Anything that wants water and isn't getting it.
        for (e, d) in colony.domes() {
            if d.kind.water_per_day() > 0.0
                && d.issues & issue::NO_WATER != 0
                && let Some(at) = to_screen(e.pos.x, e.pos.y - d.kind.size().1 as f32 - 8.0)
            {
                label(&painter, at, &[(format!("{}: no water", d.name), DANGER)]);
            }
        }
    }
    if overlays.power {
        for (i, grid) in colony.grids().iter().enumerate() {
            let Some((_, p)) = grid.pylons.first() else {
                continue;
            };
            let Some(at) = to_screen(p.x, p.y - 44.0) else {
                continue;
            };
            let days = if grid.drain_per_day > 0.0 {
                format!("  ({:.1} days left)", grid.stored / grid.drain_per_day)
            } else {
                String::new()
            };
            let mut lines = vec![(
                format!(
                    "Grid {}: {:.0} charge, -{:.0}/day{days}",
                    i + 1,
                    grid.stored,
                    grid.drain_per_day
                ),
                egui_hue(i + 3),
            )];
            if grid.stored < 1.0 && grid.drain_per_day > 0.0 {
                lines.push(("Out of charge: load battery flowers".into(), DANGER));
            }
            label(&painter, at, &lines);
        }
        for e in colony.ents.values() {
            let (flags, name, top) = match &e.kind {
                EntKind::Dome(d) => (d.issues, d.name.as_str(), d.kind.size().1 as f32 + 8.0),
                EntKind::Machine(m) => (m.issues, m.name.as_str(), 36.0),
                _ => continue,
            };
            if flags & issue::NO_POWER != 0
                && let Some(at) = to_screen(e.pos.x, e.pos.y - top)
            {
                label(&painter, at, &[(format!("{name}: no power"), DANGER)]);
            }
        }
    }

    // A reminder of what is on and how to turn it off.
    egui::Area::new("overlay_legend".into())
        .anchor(egui::Align2::LEFT_BOTTOM, [10.0, -10.0])
        .interactable(false)
        .show(ctx, |ui| {
            ui::panel_frame().show(ui, |ui| {
                if let Some(icons) = &icons {
                    if overlays.water {
                        ui.horizontal(|ui| {
                            ui::icon(ui, icons, 94, 16.0);
                            ui.label(format!(
                                "Water overlay  [{}]",
                                config.bindings.hint(Action::WaterOverlay)
                            ));
                        });
                    }
                    if overlays.power {
                        ui.horizontal(|ui| {
                            ui::icon(ui, icons, 95, 16.0);
                            ui.label(format!(
                                "Power overlay  [{}]",
                                config.bindings.hint(Action::PowerOverlay)
                            ));
                        });
                    }
                }
            });
        });
    Ok(())
}
