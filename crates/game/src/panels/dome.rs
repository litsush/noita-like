//! The dome and machine windows: what it is, whether it is running and why
//! not, its planters and production, and the controls it has.

use bevy_egui::egui;
use sbct_sim::colony::actions::Action as Act;
use sbct_sim::colony::domes::{AnimalKind, issue};
use sbct_sim::colony::farming::PlanterOp;
use sbct_sim::colony::items::{DomeKind, Item, MachineKind};
use sbct_sim::colony::power::PYLON_BUFFER;
use sbct_sim::colony::{Colony, EntKind, Id, Player};

use super::{Panels, planter_status};
use crate::ui::{self, ACCENT, DANGER, GOOD, TEXT_DIM, UiIcons, WARN};

/// A name with a pencil: click to rename.
fn name_row(
    ui: &mut egui::Ui,
    panels: &mut Panels,
    id: Id,
    name: &str,
    acts: &mut Vec<Act>,
    close: &mut bool,
) {
    ui.horizontal(|ui| {
        let editing = matches!(&panels.rename, Some((e, _)) if *e == id);
        if editing {
            let (_, text) = panels.rename.as_mut().unwrap();
            let r = ui.add(
                egui::TextEdit::singleline(text)
                    .desired_width(200.0)
                    .char_limit(28),
            );
            let enter = r.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
            if ui.button("Save").clicked() || enter {
                let (_, text) = panels.rename.take().unwrap();
                acts.push(Act::Rename { ent: id, name: text });
            }
        } else {
            ui.label(ui::title(name, 24.0).color(ACCENT));
            if ui.small_button("rename").clicked() {
                panels.rename = Some((id, name.to_string()));
            }
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("x").clicked() {
                *close = true;
            }
        });
    });
}

fn issues(ui: &mut egui::Ui, icons: &UiIcons, flags: u8) {
    for text in issue::describe(flags) {
        ui.horizontal(|ui| {
            ui::icon(ui, icons, 78, 16.0);
            ui.label(egui::RichText::new(text).color(WARN));
        });
    }
}

fn progress(ui: &mut egui::Ui, label: &str, value: f32) {
    ui.horizontal(|ui| {
        ui.label(label);
        ui::meter(ui, value, ACCENT, 180.0);
        ui.label(egui::RichText::new(format!("{:.0}%", value * 100.0)).color(TEXT_DIM));
    });
}

#[allow(clippy::too_many_arguments)]
pub fn dome_window(
    ctx: &egui::Context,
    icons: &UiIcons,
    colony: &Colony,
    me: &Player,
    id: Id,
    panels: &mut Panels,
    acts: &mut Vec<Act>,
    close: &mut bool,
) {
    let Some((_, d)) = colony.dome(id) else {
        *close = true;
        return;
    };
    egui::Area::new("dome".into())
        .anchor(egui::Align2::CENTER_CENTER, [0.0, -30.0])
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            ui::ornate(ui, |ui| {
                ui.set_width(460.0);
                name_row(ui, panels, id, &d.name, acts, close);
                ui.label(egui::RichText::new(d.kind.describe()).color(TEXT_DIM));
                ui.add_space(4.0);

                // Supply.
                ui.horizontal(|ui| {
                    if d.kind.power_per_day() > 0.0 && d.kind != DomeKind::Starter {
                        ui::icon(ui, icons, 57, 16.0);
                        ui.label(
                            egui::RichText::new(if d.powered { "Powered" } else { "No power" })
                                .color(if d.powered { GOOD } else { DANGER }),
                        )
                        .on_hover_text(format!("Uses {:.0} charge a day", d.kind.power_per_day()));
                        ui.add_space(10.0);
                    }
                    if d.kind.water_per_day() > 0.0 {
                        ui::icon(ui, icons, 54, 16.0);
                        ui.label(format!("{:.0} L", d.reserve)).on_hover_text(format!(
                            "Water on hand. Uses {:.0} L a day. Pipes refill it; a watering can tops it up.",
                            d.kind.water_per_day()
                        ));
                        ui.add_space(10.0);
                    }
                    let workers: Vec<&str> = colony
                        .workers
                        .iter()
                        .filter(|w| w.job == Some(id))
                        .map(|w| w.name.as_str())
                        .collect();
                    if !workers.is_empty() {
                        ui::icon(ui, icons, 62, 16.0);
                        ui.label(workers.join(", "));
                    }
                });
                issues(ui, icons, d.issues);

                // Planters.
                for (i, p) in d.planters.iter().enumerate() {
                    ui::panel_frame()
                        .inner_margin(egui::Margin::same(6))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                let sp = p.species.and_then(|s| colony.species.get(s as usize));
                                let (rect, resp) =
                                    ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
                                if let Some(s) = p.species {
                                    crate::hud::paint_item(ui.painter(), icons, colony, Item::Seed(s), rect);
                                    resp.on_hover_ui(|ui| {
                                        super::inventory::tooltip(
                                            ui,
                                            colony,
                                            sbct_sim::colony::inventory::Stack::new(Item::Seed(s), 1),
                                        )
                                    });
                                }
                                ui.vertical(|ui| {
                                    ui.spacing_mut().item_spacing.y = 2.0;
                                    ui.label(planter_status(colony, id, i as u8));
                                    if sp.is_some() {
                                        ui.horizontal(|ui| {
                                            ui::meter(ui, p.growth, GOOD, 90.0);
                                            ui::icon(ui, icons, 54, 12.0);
                                            ui::meter(ui, p.moisture, ui::WATER, 50.0);
                                            if p.fertilizer > 0.0 {
                                                ui::icon(ui, icons, 16, 12.0)
                                                    .on_hover_text("Fertilized: +50% growth");
                                            }
                                        });
                                    }
                                });
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    let mut op = |ui: &mut egui::Ui, label: &str, op: PlanterOp, on: bool| {
                                        if ui.add_enabled(on, egui::Button::new(label)).clicked() {
                                            acts.push(Act::Planter {
                                                dome: id,
                                                slot: i as u8,
                                                op,
                                            });
                                        }
                                    };
                                    match sp {
                                        None => {
                                            let seed =
                                                matches!(me.held().map(|s| s.item), Some(Item::Seed(_)));
                                            op(ui, "Plant held seed", PlanterOp::Plant, seed);
                                        }
                                        Some(sp) => {
                                            op(ui, "Uproot", PlanterOp::Uproot, true);
                                            op(
                                                ui,
                                                "Fertilize",
                                                PlanterOp::Fertilize,
                                                me.inv.count(Item::Fertilizer) > 0,
                                            );
                                            if sp.thirst_per_day() > 0.0 {
                                                op(ui, "Water", PlanterOp::Water, me.can_water >= 1.0);
                                            }
                                            op(ui, "Harvest", PlanterOp::Harvest, p.growth >= 1.0);
                                        }
                                    }
                                });
                            });
                        });
                }

                // Production specific to the dome type.
                match d.kind {
                    DomeKind::Kitchen => progress(ui, "Cooking", d.progress),
                    DomeKind::Dark => progress(ui, "Composting", d.progress),
                    DomeKind::Aqua => {
                        ui.label(format!("Fish in the pool: {} of 12", d.fish));
                        if d.fish < 2 {
                            ui.label(
                                egui::RichText::new("Release at least two fish fry to start a shoal.")
                                    .color(TEXT_DIM),
                            );
                        } else {
                            progress(ui, "Next fish", d.progress);
                        }
                    }
                    DomeKind::Barn => {
                        if d.animals.is_empty() {
                            ui.label(
                                egui::RichText::new("No animals. Order some from Earth and use them here.")
                                    .color(TEXT_DIM),
                            );
                        }
                        for a in &d.animals {
                            let (name, product) = match a.kind {
                                AnimalKind::Cluckbug => ("Cluckbug", "eggs"),
                                AnimalKind::MilkGrub => ("Milk Grub", "milk"),
                            };
                            ui.horizontal(|ui| {
                                ui.label(name);
                                ui::meter(ui, a.progress, ACCENT, 120.0);
                                ui.label(
                                    egui::RichText::new(if a.fed { product } else { "hungry or thirsty" })
                                        .color(if a.fed { TEXT_DIM } else { WARN }),
                                );
                            });
                        }
                    }
                    DomeKind::DomeDome => {
                        ui.horizontal(|ui| {
                            ui.label("Building");
                            let current = d.target.map_or("nothing", |k| k.name());
                            egui::ComboBox::from_id_salt("target")
                                .selected_text(current)
                                .show_ui(ui, |ui| {
                                    for k in DomeKind::KITS {
                                        if ui.selectable_label(d.target == Some(k), k.name()).clicked() {
                                            acts.push(Act::SetTarget {
                                                dome: id,
                                                target: Some(k),
                                            });
                                        }
                                    }
                                });
                        });
                        if d.target.is_some() {
                            progress(ui, "Progress", d.progress);
                        }
                    }
                    DomeKind::Apartment => {
                        let ready = d.powered && d.watered;
                        ui.label(
                            egui::RichText::new(if ready {
                                "Ready for 20 colonists"
                            } else {
                                "Not ready: needs power and water"
                            })
                            .color(if ready { GOOD } else { WARN }),
                        );
                    }
                    _ => {}
                }

                if d.kind != DomeKind::Starter {
                    ui.add_space(4.0);
                    if ui
                        .button("Pack up this dome")
                        .on_hover_text("Returns the kit. Everything inside is dropped.")
                        .clicked()
                    {
                        acts.push(Act::Dismantle { ent: id });
                        *close = true;
                    }
                }
            });
        });
}

pub fn machine_window(
    ctx: &egui::Context,
    icons: &UiIcons,
    colony: &Colony,
    id: Id,
    panels: &mut Panels,
    acts: &mut Vec<Act>,
    close: &mut bool,
) {
    let Some(EntKind::Machine(m)) = colony.ents.get(&id).map(|e| &e.kind) else {
        *close = true;
        return;
    };
    egui::Area::new("machine".into())
        .anchor(egui::Align2::CENTER_CENTER, [0.0, -30.0])
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            ui::ornate(ui, |ui| {
                ui.set_width(400.0);
                name_row(ui, panels, id, &m.name, acts, close);
                ui.label(egui::RichText::new(m.kind.describe()).color(TEXT_DIM));
                ui.add_space(4.0);
                match m.kind {
                    MachineKind::OxygenGenerator | MachineKind::Splicer | MachineKind::Pump => {
                        ui.label(
                            egui::RichText::new(if m.on { "Running" } else { "Stopped" }).color(if m.on {
                                GOOD
                            } else {
                                DANGER
                            }),
                        );
                    }
                    MachineKind::Tank => {
                        ui.horizontal(|ui| {
                            ui::icon(ui, icons, 54, 16.0);
                            ui::meter(ui, m.store / 500.0, ui::WATER, 180.0);
                            ui.label(format!("{:.0} / 500 L", m.store));
                        });
                    }
                    MachineKind::Pylon => {
                        ui.label(format!("Buffer {:.0} / {PYLON_BUFFER:.0}", m.store));
                    }
                    _ => {}
                }
                issues(ui, icons, m.issues);
                ui.add_space(4.0);
                if ui
                    .button("Pack up")
                    .on_hover_text("Returns it to your inventory.")
                    .clicked()
                {
                    acts.push(Act::Dismantle { ent: id });
                    *close = true;
                }
            });
        });
}
