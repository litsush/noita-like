//! The inventory window: hotbar, bag, trash, and an open container beside
//! it. Drag (or click, then click) to move stacks, right-click to split,
//! Shift-click to send across, Alt-click to favourite.

use bevy_egui::egui;
use sbct_sim::colony::actions::{Action as Act, Container, InvOp};
use sbct_sim::colony::inventory::{Inventory, Stack};
use sbct_sim::colony::items::Item;
use sbct_sim::colony::{Colony, EntKind, HOTBAR, Player};

use super::Panels;
use crate::hud::slot;
use crate::ui::{self, ACCENT, TEXT_DIM, UiIcons};

#[derive(Clone, Copy)]
pub struct Mods {
    pub shift: bool,
    pub alt: bool,
}

const SLOT: f32 = 36.0;
const COLUMNS: usize = 10;
const PANEL_WIDTH: f32 = COLUMNS as f32 * (SLOT + 3.0);

pub fn container_inv<'a>(
    colony: &'a Colony,
    me: &'a Player,
    c: Container,
) -> Option<(&'a Inventory, String)> {
    match c {
        Container::Me => Some((&me.inv, "Inventory".into())),
        Container::DomeChest(id, i) => {
            let (_, d) = colony.dome(id)?;
            let label = d.kind.chest_label(i as usize);
            Some((d.chests.get(i as usize)?, format!("{}: {label}", d.name)))
        }
        Container::Machine(id) | Container::Crate(id) => match &colony.ents.get(&id)?.kind {
            EntKind::Machine(m) => Some((&m.inv, m.name.clone())),
            EntKind::Crate(c) => Some((
                &c.inv,
                match c.kind {
                    sbct_sim::colony::CrateKind::Pack => "Lost pack".into(),
                    sbct_sim::colony::CrateKind::Cache => "Salvage cache".into(),
                    sbct_sim::colony::CrateKind::Pod => "Drop pod".into(),
                },
            )),
            _ => None,
        },
    }
}

pub fn tooltip(ui: &mut egui::Ui, colony: &Colony, stack: Stack) {
    let item = stack.item;
    ui.label(egui::RichText::new(colony.item_name(item)).color(ACCENT));
    ui.label(item.describe(&colony.species));
    let value = item.value(&colony.species);
    if value > 0 {
        ui.label(egui::RichText::new(format!("Earth pays {value} cr each")).color(TEXT_DIM));
    }
    if let Item::Seed(s) | Item::Crop(s) = item
        && let Some(sp) = colony.species.get(s as usize)
    {
        let g = sp.genes;
        ui.label(
            egui::RichText::new(format!(
                "Oxygen {}  Growth {}  Yield {}  Power {}\nFood {}  Light {}  Thirst {}  Hardy {}",
                g.oxygen, g.growth, g.yield_, g.power, g.food, g.light, g.thirst, g.hardy
            ))
            .color(TEXT_DIM),
        );
    }
    if stack.fav {
        ui.label(
            egui::RichText::new("Favourite: skipped by sort, quick stack, deposit and trash").color(TEXT_DIM),
        );
    }
}

/// Draws one inventory's slots and turns clicks and drags into operations.
#[allow(clippy::too_many_arguments)]
fn grid(
    ui: &mut egui::Ui,
    icons: &UiIcons,
    colony: &Colony,
    inv: &Inventory,
    range: std::ops::Range<usize>,
    this: Container,
    other: Option<Container>,
    selected: Option<usize>,
    panels: &mut Panels,
    mods: Mods,
    acts: &mut Vec<Act>,
) {
    let search = panels.search.to_lowercase();
    let pointer_released = ui.input(|i| i.pointer.any_released());
    let pointer_pos = ui.input(|i| i.pointer.interact_pos());
    egui::Grid::new(("grid", format!("{this:?}"), range.start))
        .spacing([3.0, 3.0])
        .show(ui, |ui| {
            for i in range.clone() {
                let stack = inv.slots.get(i).copied().flatten();
                let dragged = panels.dragging == Some((this, i as u16));
                let matches = search.is_empty()
                    || stack.is_some_and(|s| colony.item_name(s.item).to_lowercase().contains(&search));
                ui.scope(|ui| {
                    if !matches || dragged {
                        ui.set_opacity(0.3);
                    }
                    let r = slot(ui, icons, colony, stack, SLOT, selected == Some(i));
                    let here = (this, i as u16);

                    // Dropping a dragged stack on this slot.
                    let mut dropped = false;
                    if pointer_released
                        && let Some(from) = panels.dragging
                        && from != here
                        && pointer_pos.is_some_and(|p| r.rect.contains(p))
                    {
                        acts.push(Act::Inv(InvOp::Move {
                            from,
                            to: here,
                            count: u32::MAX,
                        }));
                        panels.dragging = None;
                        dropped = true;
                    }
                    if r.drag_started() && stack.is_some() && !mods.shift && !mods.alt {
                        panels.dragging = Some(here);
                    }
                    if r.clicked() && !dropped {
                        if let Some(from) = panels.dragging.take() {
                            // Click-then-click moving.
                            if from != here {
                                acts.push(Act::Inv(InvOp::Move {
                                    from,
                                    to: here,
                                    count: u32::MAX,
                                }));
                            }
                        } else if stack.is_some() {
                            if mods.alt && this == Container::Me {
                                acts.push(Act::Inv(InvOp::Favorite(i as u16)));
                            } else if mods.shift {
                                // To the other window, or between hotbar and bag.
                                acts.push(Act::Inv(InvOp::QuickMove {
                                    from: here,
                                    to: other.unwrap_or(this),
                                }));
                            } else {
                                panels.dragging = Some(here);
                            }
                        }
                    }
                    if r.secondary_clicked()
                        && let Some(s) = stack
                        && s.count > 1
                        && let Some(empty) = inv.slots.iter().position(|x| x.is_none())
                    {
                        // Split half into the first free slot.
                        acts.push(Act::Inv(InvOp::Move {
                            from: here,
                            to: (this, empty as u16),
                            count: s.count / 2,
                        }));
                    }
                    if let Some(s) = stack {
                        r.on_hover_ui(|ui| tooltip(ui, colony, s));
                    }
                });
                if (i - range.start + 1).is_multiple_of(COLUMNS) {
                    ui.end_row();
                }
            }
        });
}

fn icon_button(ui: &mut egui::Ui, icons: &UiIcons, icon: usize, tip: &str) -> bool {
    let (rect, r) = ui.allocate_exact_size(egui::vec2(26.0, 26.0), egui::Sense::click());
    let p = ui.painter();
    p.rect_filled(
        rect,
        0.0,
        if r.hovered() {
            egui::Color32::from_rgb(34, 62, 60)
        } else {
            egui::Color32::from_rgb(16, 27, 29)
        },
    );
    p.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0, if r.hovered() { ACCENT } else { ui::BORDER }),
        egui::StrokeKind::Inside,
    );
    ui::paint_icon(p, icons, icon, rect.shrink(5.0), egui::Color32::WHITE);
    r.on_hover_text(tip).clicked()
}

#[allow(clippy::too_many_arguments)]
pub fn window(
    ctx: &egui::Context,
    icons: &UiIcons,
    colony: &Colony,
    me: &Player,
    container: Option<Container>,
    panels: &mut Panels,
    mods: Mods,
    acts: &mut Vec<Act>,
    close: &mut bool,
) {
    egui::Area::new("inventory".into())
        .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -96.0])
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            ui.horizontal_top(|ui| {
                // The open container, to the left.
                if let Some(c) = container
                    && let Some((inv, title)) = container_inv(colony, me, c)
                {
                    ui::ornate(ui, |ui| {
                      ui.vertical(|ui| {
                        ui.set_width(PANEL_WIDTH);
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(title).color(ACCENT));
                            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                if icon_button(ui, icons, 74, "Sort") {
                                    acts.push(Act::Inv(InvOp::Sort(c)));
                                }
                                if icon_button(ui, icons, 108, "Take everything") {
                                    acts.push(Act::Inv(InvOp::LootAll(c)));
                                }
                                if icon_button(ui, icons, 76, "Deposit everything from your bag") {
                                    acts.push(Act::Inv(InvOp::DepositAll(c)));
                                }
                                if icon_button(ui, icons, 75, "Quick stack: deposit what this already holds") {
                                    acts.push(Act::Inv(InvOp::QuickStack(c)));
                                }
                            });
                        });
                        egui::ScrollArea::vertical().max_height(SLOT * 6.5).show(ui, |ui| {
                            grid(
                                ui,
                                icons,
                                colony,
                                inv,
                                0..inv.slots.len(),
                                c,
                                Some(Container::Me),
                                None,
                                panels,
                                mods,
                                acts,
                            );
                        });
                      });
                    });
                }

                ui::ornate(ui, |ui| {
                  ui.vertical(|ui| {
                    ui.set_width(PANEL_WIDTH);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new("Inventory").color(ACCENT));
                        ui::icon(ui, icons, 77, 16.0);
                        ui.add(
                            egui::TextEdit::singleline(&mut panels.search)
                                .desired_width(110.0)
                                .hint_text("search"),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("x").on_hover_text("Close").clicked() {
                                *close = true;
                            }
                            if icon_button(ui, icons, 74, "Sort the bag") {
                                acts.push(Act::Inv(InvOp::Sort(Container::Me)));
                            }
                            if icon_button(ui, icons, 75, "Quick stack to nearby chests") {
                                acts.push(Act::Inv(InvOp::QuickStackNearby));
                            }
                            // Trash: drop a stack here, or click to take the last one back.
                            let (rect, r) = ui.allocate_exact_size(egui::vec2(26.0, 26.0), egui::Sense::click());
                            let p = ui.painter();
                            p.rect_filled(rect, 0.0, egui::Color32::from_rgb(16, 27, 29));
                            p.rect_stroke(rect, 0.0, egui::Stroke::new(1.0, ui::BORDER), egui::StrokeKind::Inside);
                            match me.trash {
                                Some(s) => crate::hud::paint_item(p, icons, colony, s.item, rect.shrink(4.0)),
                                None => ui::paint_icon(p, icons, 59, rect.shrink(5.0), egui::Color32::WHITE),
                            }
                            let released = ui.input(|i| i.pointer.any_released());
                            let over = ui.input(|i| i.pointer.interact_pos()).is_some_and(|p| rect.contains(p));
                            if let Some((Container::Me, i)) = panels.dragging
                                && over
                                && (released || r.clicked())
                            {
                                acts.push(Act::Inv(InvOp::Trash(i)));
                                panels.dragging = None;
                            } else if r.clicked() && me.trash.is_some() {
                                acts.push(Act::Inv(InvOp::Untrash));
                            }
                            r.on_hover_text("Trash: drop a stack here. Click to take the last one back.");
                        });
                    });
                    let selected = (me.selected as usize) < HOTBAR;
                    grid(
                        ui,
                        icons,
                        colony,
                        &me.inv,
                        HOTBAR..me.inv.slots.len(),
                        Container::Me,
                        container,
                        None,
                        panels,
                        mods,
                        acts,
                    );
                    ui.add_space(4.0);
                    grid(
                        ui,
                        icons,
                        colony,
                        &me.inv,
                        0..HOTBAR,
                        Container::Me,
                        container,
                        selected.then_some(me.selected as usize),
                        panels,
                        mods,
                        acts,
                    );
                    ui.label(
                        egui::RichText::new("Drag to move  ·  Right-click splits  ·  Shift-click sends  ·  Alt-click favourites")
                            .color(TEXT_DIM),
                    );
                  });
                });
            });
        });

    // A dragged stack follows the pointer; letting go over nothing cancels.
    if let Some((c, i)) = panels.dragging {
        let stack =
            container_inv(colony, me, c).and_then(|(inv, _)| inv.slots.get(i as usize).copied().flatten());
        match (stack, ctx.pointer_latest_pos()) {
            (Some(s), Some(pos)) => {
                let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Tooltip, "drag".into()));
                let rect = egui::Rect::from_center_size(pos, egui::vec2(28.0, 28.0));
                crate::hud::paint_item(&painter, icons, colony, s.item, rect);
            }
            _ => panels.dragging = None,
        }
        if ctx.input(|i| i.pointer.any_released()) && !ctx.is_pointer_over_egui() {
            // Released over the world: throw it out.
            if c == Container::Me {
                acts.push(Act::Inv(InvOp::Drop {
                    slot: i,
                    count: u32::MAX,
                }));
            }
            panels.dragging = None;
        }
        if ctx.input(|i| i.pointer.secondary_clicked()) {
            panels.dragging = None;
        }
    }
}
