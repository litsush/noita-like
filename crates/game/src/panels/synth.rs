//! The Synthesizer: recipes, what they need, and what the colony has.

use bevy_egui::egui;
use sbct_sim::colony::actions::Action as Act;
use sbct_sim::colony::items::Category;
use sbct_sim::colony::recipes::recipes;
use sbct_sim::colony::{Colony, Player};

use super::Panels;
use crate::hud::paint_item;
use crate::ui::{self, ACCENT, DANGER, GOOD, TEXT, TEXT_DIM, UiIcons, WARN};

pub fn window(
    ctx: &egui::Context,
    icons: &UiIcons,
    colony: &Colony,
    me: &Player,
    panels: &mut Panels,
    acts: &mut Vec<Act>,
    close: &mut bool,
) {
    let mut open_mods = false;
    egui::Area::new("synth".into())
        .anchor(egui::Align2::LEFT_CENTER, [14.0, -20.0])
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            ui::ornate(ui, |ui| {
                ui.set_width(430.0);
                ui.horizontal(|ui| {
                    ui.label(ui::title("Synthesizer", 24.0).color(ACCENT));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("x").clicked() {
                            *close = true;
                        }
                        if ui
                            .button("Body mods")
                            .on_hover_text("Synthesize body-mod tiers")
                            .clicked()
                        {
                            open_mods = true;
                        }
                        ui.add(
                            egui::TextEdit::singleline(&mut panels.synth_filter)
                                .desired_width(120.0)
                                .hint_text("search"),
                        );
                    });
                });
                ui.label(
                    egui::RichText::new("Uses your inventory, this dome's chests and every Storage Dome.")
                        .color(TEXT_DIM),
                );
                ui.add_space(4.0);
                let filter = panels.synth_filter.to_lowercase();
                egui::ScrollArea::vertical().max_height(420.0).show(ui, |ui| {
                    let mut section = "";
                    for (id, r) in recipes().iter().enumerate() {
                        let name = colony.item_name(r.output);
                        if !filter.is_empty() && !name.to_lowercase().contains(&filter) {
                            continue;
                        }
                        let heading = match r.output.category() {
                            Category::Building => {
                                if matches!(r.output, sbct_sim::colony::items::Item::DomeKit(_)) {
                                    "Domes"
                                } else {
                                    "Machines"
                                }
                            }
                            Category::Tool => "Tools",
                            _ => "Materials",
                        };
                        if heading != section {
                            section = heading;
                            ui.add_space(2.0);
                            ui.label(egui::RichText::new(heading).color(ACCENT));
                        }
                        let locked = r.blueprint.filter(|b| !colony.blueprints.contains(b));
                        // How many batches the stockpile covers.
                        let inputs = colony.recipe_cost(me.key, id);
                        let max = inputs
                            .iter()
                            .map(|&(ing, need)| colony.stock_count(me.key, ing) / need)
                            .min()
                            .unwrap_or(0);
                        ui::panel_frame()
                            .inner_margin(egui::Margin::same(6))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    let (rect, resp) =
                                        ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::hover());
                                    paint_item(ui.painter(), icons, colony, r.output, rect);
                                    resp.on_hover_text(r.output.describe(&colony.species));
                                    ui.vertical(|ui| {
                                        ui.spacing_mut().item_spacing.y = 2.0;
                                        let title = if r.count > 1 {
                                            format!("{name} x{}", r.count)
                                        } else {
                                            name.clone()
                                        };
                                        ui.label(egui::RichText::new(title).color(if locked.is_some() {
                                            TEXT_DIM
                                        } else {
                                            TEXT
                                        }));
                                        ui.horizontal_wrapped(|ui| {
                                            ui.spacing_mut().item_spacing.x = 4.0;
                                            for &(ing, need) in &inputs {
                                                let have = colony.stock_count(me.key, ing);
                                                ui::icon(ui, icons, ing.icon(), 16.0)
                                                    .on_hover_text(ing.name(&colony.species));
                                                ui.label(
                                                    egui::RichText::new(format!("{}/{need}", have.min(9999)))
                                                        .color(if have >= need { GOOD } else { DANGER }),
                                                );
                                                ui.add_space(4.0);
                                            }
                                        });
                                    });
                                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                        if let Some(b) = locked {
                                            ui.label(
                                                egui::RichText::new(format!("Blueprint: {} cr", b.price()))
                                                    .color(WARN),
                                            )
                                            .on_hover_text("Buy it from Earth at the Comm. Terminal.");
                                            return;
                                        }
                                        let mut craft = |ui: &mut egui::Ui, label: &str, n: u32| {
                                            if ui
                                                .add_enabled(max >= n && n > 0, egui::Button::new(label))
                                                .clicked()
                                            {
                                                acts.push(Act::Craft {
                                                    recipe: id as u16,
                                                    count: n as u16,
                                                });
                                            }
                                        };
                                        if max > 1 {
                                            craft(ui, &format!("x{}", max.min(99)), max.min(99));
                                        }
                                        craft(ui, "Make", 1);
                                    });
                                });
                            });
                    }
                });
            });
        });
    if open_mods {
        panels.open = Some(super::Open::ModBay);
    }
}
