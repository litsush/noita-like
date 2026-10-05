//! The Mod Bay: every body-mod set by slot, what each tier does, what the
//! next tier costs, and what is equipped.

use bevy_egui::egui;
use sbct_sim::colony::actions::Action as Act;
use sbct_sim::colony::domes::Fixture;
use sbct_sim::colony::mods::{ArmTask, LEGENDARY, MOD_SETS, Slot, blueprint_price, tier_label};
use sbct_sim::colony::recipes::Ingredient;
use sbct_sim::colony::{Colony, Player};

use super::Panels;
use crate::ui::{self, ACCENT, DANGER, GOOD, TEXT, TEXT_DIM, UiIcons, WARN};

const LEGEND: egui::Color32 = egui::Color32::from_rgb(255, 214, 110);

/// Whether the player is standing at a Mod Bay (where sets are swapped).
fn at_bay(colony: &Colony, me: &Player) -> bool {
    let Some((e, d)) = me.in_dome.and_then(|id| colony.dome(id)) else {
        return false;
    };
    d.kind
        .fixtures()
        .iter()
        .filter(|f| f.fixture == Fixture::ModBay)
        .any(|f| {
            sbct_sim::colony::domes::fixture_pos(e.pos, f).distance(me.pose.pos) <= me.stats().reach + 12.0
        })
}

pub fn window(
    ctx: &egui::Context,
    icons: &UiIcons,
    colony: &Colony,
    me: &Player,
    panels: &mut Panels,
    acts: &mut Vec<Act>,
    close: &mut bool,
) {
    let bay = at_bay(colony, me);
    let stats = me.stats();
    egui::Area::new("modbay".into())
        .anchor(egui::Align2::CENTER_CENTER, [0.0, -10.0])
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            ui::ornate(ui, |ui| {
                ui.set_width(900.0);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(ui::title("Body Mods", 26.0).color(ACCENT));
                        ui.label(
                            egui::RichText::new(if bay {
                                "One set per body slot. Swap them here."
                            } else {
                                "Synthesize tiers here. Swap sets at the Mod Bay in the starter dome."
                            })
                            .color(TEXT_DIM),
                        );
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("x").clicked() {
                                *close = true;
                            }
                        });
                    });
                    // Slot tabs, each showing what is worn there.
                    ui.horizontal(|ui| {
                        for slot in Slot::ALL {
                            let worn = me.equipped[slot as usize];
                            let selected = panels.mod_slot == slot;
                            let frame = egui::Frame::new()
                                .fill(if selected {
                                    egui::Color32::from_rgb(24, 52, 46)
                                } else {
                                    egui::Color32::from_rgb(12, 22, 24)
                                })
                                .stroke(egui::Stroke::new(1.0, if selected { ACCENT } else { ui::RULE }))
                                .inner_margin(5.0);
                            let r = frame
                                .show(ui, |ui| {
                                    ui.set_width(96.0);
                                    ui.vertical_centered(|ui| {
                                        ui.label(egui::RichText::new(slot.name()).color(if selected {
                                            ACCENT
                                        } else {
                                            TEXT
                                        }));
                                        match worn {
                                            Some(set) => {
                                                let tier = me.mods.get(&set).copied().unwrap_or(1);
                                                ui::mod_icon(
                                                    ui,
                                                    icons,
                                                    set as usize,
                                                    32.0,
                                                    egui::Color32::WHITE,
                                                );
                                                ui.label(
                                                    egui::RichText::new(if tier >= LEGENDARY {
                                                        "Legend"
                                                    } else {
                                                        tier_label(tier)
                                                    })
                                                    .color(if tier >= LEGENDARY { LEGEND } else { GOOD })
                                                    .small(),
                                                );
                                            }
                                            None => {
                                                ui.add_space(32.0);
                                                ui.label(
                                                    egui::RichText::new("empty").color(TEXT_DIM).small(),
                                                );
                                            }
                                        }
                                    });
                                })
                                .response
                                .interact(egui::Sense::click());
                            if r.clicked() {
                                panels.mod_slot = slot;
                            }
                        }
                    });
                    ui.add_space(4.0);

                    // Automated arms get their orders here.
                    if stats.arms > 0 {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new("Automated arms:").color(ACCENT));
                            for arm in 0..stats.arms {
                                let current = me.gear.arm_tasks[arm as usize];
                                egui::ComboBox::from_id_salt(("arm", arm))
                                    .selected_text(format!("Arm {}: {}", arm + 1, current.name()))
                                    .width(230.0)
                                    .show_ui(ui, |ui| {
                                        for task in ArmTask::ALL {
                                            if ui.selectable_label(task == current, task.name()).clicked()
                                                && task != current
                                            {
                                                acts.push(Act::ArmTask { arm, task });
                                            }
                                        }
                                    });
                            }
                            if stats.free_arm {
                                ui.label(
                                    egui::RichText::new("Send the free arm from the map.").color(TEXT_DIM),
                                );
                            }
                        });
                        ui.add_space(4.0);
                    }

                    let slot = panels.mod_slot;
                    egui::ScrollArea::vertical()
                        .max_height(440.0)
                        .min_scrolled_height(440.0)
                        .show(ui, |ui| {
                            ui.set_width(884.0);
                            for (id, set) in MOD_SETS.iter().enumerate().filter(|(_, s)| s.slot == slot) {
                                let id = id as u8;
                                let owned = me.mods.get(&id).copied().unwrap_or(0);
                                let worn = me.equipped[slot as usize] == Some(id);
                                let known = colony.knows_mod(id);
                                ui::panel_frame()
                                    .inner_margin(egui::Margin::same(8))
                                    .show(ui, |ui| {
                                        ui.set_width(860.0);
                                        ui.horizontal(|ui| {
                                            let tint = if owned > 0 {
                                                egui::Color32::WHITE
                                            } else {
                                                egui::Color32::from_gray(110)
                                            };
                                            ui::mod_icon(ui, icons, id as usize, 48.0, tint);
                                            ui.vertical(|ui| {
                                                ui.set_width(540.0);
                                                ui.horizontal(|ui| {
                                                    ui.label(ui::title(set.name, 20.0).color(ACCENT));
                                                    if worn {
                                                        ui.label(
                                                            egui::RichText::new("EQUIPPED")
                                                                .color(GOOD)
                                                                .small(),
                                                        );
                                                    }
                                                });
                                                for (t, (name, text)) in set.tiers.iter().enumerate() {
                                                    let tier = t as u8 + 1;
                                                    let have = owned >= tier;
                                                    let color = if !have {
                                                        TEXT_DIM
                                                    } else if tier >= LEGENDARY {
                                                        LEGEND
                                                    } else {
                                                        TEXT
                                                    };
                                                    ui.horizontal_wrapped(|ui| {
                                                        ui.spacing_mut().item_spacing.x = 5.0;
                                                        ui.label(
                                                            egui::RichText::new(if have {
                                                                "■"
                                                            } else {
                                                                "□"
                                                            })
                                                            .color(color),
                                                        );
                                                        ui.label(
                                                            egui::RichText::new(format!(
                                                                "{}  {name}:",
                                                                tier_label(tier)
                                                            ))
                                                            .color(color)
                                                            .strong(),
                                                        );
                                                        ui.label(egui::RichText::new(*text).color(color));
                                                    });
                                                }
                                            });
                                            ui.vertical(|ui| {
                                                ui.set_width(236.0);
                                                if !known {
                                                    ui.label(
                                                        egui::RichText::new(format!(
                                                            "Blueprint: {} cr",
                                                            blueprint_price(id)
                                                        ))
                                                        .color(WARN),
                                                    );
                                                    ui.label(
                                                        egui::RichText::new("Buy it at the Comm. Terminal.")
                                                            .color(TEXT_DIM)
                                                            .small(),
                                                    );
                                                } else if owned < LEGENDARY {
                                                    let next = owned + 1;
                                                    let cost = colony.mod_cost(me.key, next);
                                                    let mut enough = true;
                                                    ui.horizontal_wrapped(|ui| {
                                                        ui.spacing_mut().item_spacing.x = 3.0;
                                                        for &(item, need) in &cost {
                                                            let have = colony
                                                                .stock_count(me.key, Ingredient::Item(item));
                                                            enough &= have >= need;
                                                            ui::icon(ui, icons, item.icon(), 16.0)
                                                                .on_hover_text(colony.item_name(item));
                                                            ui.label(
                                                                egui::RichText::new(format!(
                                                                    "{}/{need}",
                                                                    have.min(999)
                                                                ))
                                                                .color(if have >= need {
                                                                    GOOD
                                                                } else {
                                                                    DANGER
                                                                })
                                                                .small(),
                                                            );
                                                        }
                                                    });
                                                    let label = format!(
                                                        "Synthesize {}: {}",
                                                        tier_label(next),
                                                        set.tiers[next as usize - 1].0
                                                    );
                                                    if ui
                                                        .add_enabled(enough, egui::Button::new(label))
                                                        .clicked()
                                                    {
                                                        acts.push(Act::CraftMod { set: id });
                                                    }
                                                } else {
                                                    ui.label(egui::RichText::new("Legendary").color(LEGEND));
                                                }
                                                if owned > 0 {
                                                    let (label, set) = if worn {
                                                        ("Take off", None)
                                                    } else {
                                                        ("Equip", Some(id))
                                                    };
                                                    let r = ui.add_enabled(bay, egui::Button::new(label));
                                                    if r.clicked() {
                                                        acts.push(Act::Equip { slot, set });
                                                    }
                                                    if !bay {
                                                        r.on_disabled_hover_text(
                                                    "Sets are swapped at the Mod Bay in the starter dome.",
                                                );
                                                    }
                                                }
                                            });
                                        });
                                    });
                                ui.add_space(3.0);
                            }
                        });
                });
            });
        });
}
