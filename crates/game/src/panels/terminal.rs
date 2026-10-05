//! The Comm. Terminal: sell to Earth, buy seeds, animals, workers and
//! blueprints.

use bevy_egui::egui;
use sbct_sim::colony::actions::Action as Act;
use sbct_sim::colony::economy::Offer;
use sbct_sim::colony::items::{Category, Item};
use sbct_sim::colony::plants::BASE_SPECIES;
use sbct_sim::colony::recipes::Blueprint;
use sbct_sim::colony::{Colony, Player};

use super::{Panels, TerminalTab};
use crate::hud::paint_item;
use crate::ui::{self, ACCENT, GOOD, TEXT_DIM, UiIcons, WARN};

fn row(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui)) {
    ui::panel_frame()
        .inner_margin(egui::Margin::same(6))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(add);
        });
}

fn buy_button(ui: &mut egui::Ui, colony: &Colony, offer: Offer, acts: &mut Vec<Act>) {
    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
        let price = offer.price();
        let afford = colony.credits >= price as i64;
        if ui
            .add_enabled(afford, egui::Button::new(format!("Buy  {price} cr")))
            .clicked()
        {
            acts.push(Act::Buy(offer));
        }
    });
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
    egui::Area::new("terminal".into())
        .anchor(egui::Align2::CENTER_CENTER, [0.0, -10.0])
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            ui::ornate(ui, |ui| {
                ui.set_width(520.0);
                ui.horizontal(|ui| {
                    ui.label(ui::title("Comm. Terminal", 24.0).color(ACCENT));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.small_button("x").clicked() {
                            *close = true;
                        }
                        ui.label(format!("{} cr", colony.credits));
                        ui::icon(ui, icons, 50, 16.0);
                    });
                });
                ui.horizontal(|ui| {
                    for (tab, label) in [
                        (TerminalTab::Sell, "Sell to Earth"),
                        (TerminalTab::Buy, "Order goods"),
                        (TerminalTab::Blueprints, "Blueprints"),
                        (TerminalTab::Workers, "Workers"),
                        (TerminalTab::Colony, "Colony"),
                    ] {
                        ui.selectable_value(&mut panels.terminal_tab, tab, label);
                    }
                });
                ui.add_space(4.0);
                egui::ScrollArea::vertical()
                    .max_height(400.0)
                    .show(ui, |ui| match panels.terminal_tab {
                        TerminalTab::Sell => sell(ui, icons, colony, me, acts),
                        TerminalTab::Buy => buy(ui, icons, colony, acts),
                        TerminalTab::Blueprints => blueprints(ui, icons, colony, acts),
                        TerminalTab::Workers => workers(ui, icons, colony, acts),
                        TerminalTab::Colony => readiness(ui, icons, colony, acts),
                    });
            });
        });
}

fn sell(ui: &mut egui::Ui, icons: &UiIcons, colony: &Colony, me: &Player, acts: &mut Vec<Act>) {
    // One row per kind of item the player carries that Earth wants.
    let mut items: Vec<(Item, u32)> = Vec::new();
    for s in me.inv.stacks() {
        if s.item.value(&colony.species) == 0 || s.fav {
            continue;
        }
        match items.iter_mut().find(|(i, _)| *i == s.item) {
            Some((_, n)) => *n += s.count,
            None => items.push((s.item, s.count)),
        }
    }
    items.sort_by_key(|(i, _)| std::cmp::Reverse(i.value(&colony.species)));
    if items.is_empty() {
        ui.label(
            egui::RichText::new("Nothing to sell. Earth pays for ores, creature samples, crops and meals.")
                .color(TEXT_DIM),
        );
        return;
    }
    let minerals: Vec<&(Item, u32)> = items
        .iter()
        .filter(|(i, _)| {
            matches!(
                i,
                Item::Iron
                    | Item::Copper
                    | Item::Silicon
                    | Item::Gold
                    | Item::Titanium
                    | Item::Xenite
                    | Item::Aurorium
            )
        })
        .collect();
    let total: u32 = minerals.iter().map(|(i, n)| i.value(&colony.species) * n).sum();
    if total > 0 && ui.button(format!("Sell all ores  (+{total} cr)")).clicked() {
        for (item, count) in &minerals {
            acts.push(Act::Sell {
                item: *item,
                count: *count,
            });
        }
    }
    ui.label(egui::RichText::new("Favourited stacks are never listed here.").color(TEXT_DIM));
    for (item, count) in items {
        let value = item.value(&colony.species);
        row(ui, |ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
            paint_item(ui.painter(), icons, colony, item, rect);
            ui.label(format!("{} x{count}", colony.item_name(item)));
            ui.label(egui::RichText::new(format!("{value} cr each")).color(TEXT_DIM));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(format!("All  +{}", value * count)).clicked() {
                    acts.push(Act::Sell { item, count });
                }
                if count >= 10 && ui.button("10").clicked() {
                    acts.push(Act::Sell { item, count: 10 });
                }
                if ui.button("1").clicked() {
                    acts.push(Act::Sell { item, count: 1 });
                }
            });
        });
    }
    let _ = Category::Resource;
}

fn buy(ui: &mut egui::Ui, icons: &UiIcons, colony: &Colony, acts: &mut Vec<Act>) {
    ui.label(egui::RichText::new("Orders arrive by drop pod beside the starter dome.").color(TEXT_DIM));
    ui.label(egui::RichText::new("Animals").color(ACCENT));
    for (offer, item, text) in [
        (
            Offer::Cluckbug,
            Item::Cluckbug,
            "Cluckbug: lays eggs in a Barn Dome",
        ),
        (
            Offer::MilkGrub,
            Item::MilkGrub,
            "Milk Grub: gives milk in a Barn Dome",
        ),
        (
            Offer::FishFry,
            Item::FishFry,
            "Fish fry x5: stock an Aqua Dome's pool",
        ),
    ] {
        row(ui, |ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
            paint_item(ui.painter(), icons, colony, item, rect);
            ui.label(text);
            buy_button(ui, colony, offer, acts);
        });
    }
    ui.label(egui::RichText::new("Seed packs (3 seeds)").color(ACCENT));
    for id in 0..BASE_SPECIES as u16 {
        let Some(sp) = colony.species.get(id as usize) else {
            continue;
        };
        row(ui, |ui| {
            let (rect, resp) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
            paint_item(ui.painter(), icons, colony, Item::Seed(id), rect);
            resp.on_hover_ui(|ui| {
                super::inventory::tooltip(
                    ui,
                    colony,
                    sbct_sim::colony::inventory::Stack::new(Item::Seed(id), 1),
                )
            });
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                ui.label(&sp.name);
                let what = match sp.product {
                    Some(p) => format!("yields {}", p.label()),
                    None => "oxygen plant".into(),
                };
                let whereabouts = match sp.habitat {
                    sbct_sim::colony::plants::Habitat::Aquatic => "Aqua Dome",
                    _ if sp.genes.light <= 3 => "Dark Dome",
                    _ => "Green Dome",
                };
                ui.label(egui::RichText::new(format!("{what}  ·  {whereabouts}")).color(TEXT_DIM));
            });
            buy_button(ui, colony, Offer::SeedPack(id), acts);
        });
    }
}

fn blueprints(ui: &mut egui::Ui, icons: &UiIcons, colony: &Colony, acts: &mut Vec<Act>) {
    ui.label(
        egui::RichText::new("A blueprint teaches every Synthesizer in the colony new recipes.")
            .color(TEXT_DIM),
    );
    for b in Blueprint::BASE {
        row(ui, |ui| {
            ui::icon(ui, icons, 63, 24.0);
            ui.label(b.name());
            if colony.blueprints.contains(&b) {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new("Owned").color(GOOD));
                });
            } else {
                buy_button(ui, colony, Offer::Blueprint(b), acts);
            }
        });
    }
    ui.add_space(6.0);
    ui.label(egui::RichText::new("Body mods").color(ACCENT));
    ui.label(
        egui::RichText::new(
            "A blueprint lets everyone synthesize that set's tiers at a Synthesizer or the Mod Bay. Six \
             basic sets are already known.",
        )
        .color(TEXT_DIM),
    );
    for (id, set) in sbct_sim::colony::mods::MOD_SETS.iter().enumerate() {
        let b = Blueprint::Mod(id as u8);
        if b.price() == 0 {
            continue;
        }
        row(ui, |ui| {
            ui::mod_icon(ui, icons, id, 24.0, egui::Color32::WHITE);
            ui.label(format!("{} ({})", set.name, set.slot.name()))
                .on_hover_text(
                    set.tiers
                        .iter()
                        .map(|(name, text)| format!("{name}: {text}"))
                        .collect::<Vec<_>>()
                        .join("\n"),
                );
            if colony.blueprints.contains(&b) {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new("Owned").color(GOOD));
                });
            } else {
                buy_button(ui, colony, Offer::Blueprint(b), acts);
            }
        });
    }
}

fn workers(ui: &mut egui::Ui, icons: &UiIcons, colony: &Colony, acts: &mut Vec<Act>) {
    ui.label(
        egui::RichText::new(
            "A worker needs a bed in a Bedroom or Apartment Dome, and eats 2 food and drinks 4 L a day from the stockpile. Assigned to a dome, they raise its output by 50% and harvest and replant its planters. Two per dome.",
        )
        .color(TEXT_DIM),
    );
    let beds: u32 = colony.free_worker_beds().iter().map(|b| b.1).sum();
    row(ui, |ui| {
        ui::icon(ui, icons, 62, 24.0);
        ui.label(format!("Hire a worker  ({beds} free beds)"));
        if beds == 0 {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(egui::RichText::new("No free bed").color(WARN));
            });
        } else {
            buy_button(ui, colony, Offer::Worker, acts);
        }
    });
    let jobs: Vec<(u32, String)> = colony
        .domes()
        .filter(|(_, d)| d.kind.employs())
        .map(|(e, d)| (e.id, d.name.clone()))
        .collect();
    for w in &colony.workers {
        row(ui, |ui| {
            ui::icon(ui, icons, 62, 24.0);
            let mut name = egui::RichText::new(&w.name);
            if w.hungry {
                name = name.color(WARN);
            }
            ui.label(name).on_hover_text(if w.hungry {
                "Hungry or thirsty: not working until the stockpile has food and water"
            } else {
                "Fed and working"
            });
            let current = w
                .job
                .and_then(|j| jobs.iter().find(|(id, _)| *id == j))
                .map_or("Unassigned".to_string(), |(_, n)| n.clone());
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                egui::ComboBox::from_id_salt(("job", w.id))
                    .selected_text(current)
                    .show_ui(ui, |ui| {
                        if ui.selectable_label(w.job.is_none(), "Unassigned").clicked() {
                            acts.push(Act::AssignWorker {
                                worker: w.id,
                                job: None,
                            });
                        }
                        for (id, name) in &jobs {
                            if ui.selectable_label(w.job == Some(*id), name).clicked() {
                                acts.push(Act::AssignWorker {
                                    worker: w.id,
                                    job: Some(*id),
                                });
                            }
                        }
                    });
            });
        });
    }
}

fn readiness(ui: &mut egui::Ui, icons: &UiIcons, colony: &Colony, acts: &mut Vec<Act>) {
    use sbct_sim::colony::readiness::{COLONISTS_PER_APARTMENT, Readiness, Ship, TARGETS};
    ui.label(
        egui::RichText::new(
            "Earth will send the colony ship when the planet can support its passengers: housing, air, food and water.",
        )
        .color(TEXT_DIM),
    );
    let r = colony.readiness;
    let (values, targets, met) = (r.values(), TARGETS.values(), r.met());
    let how = [
        "Build Apartment Domes and give each power and water.",
        "Grow plants in domes (Kelp Ribbon and Breathfern give the most) and run Oxygen Generators.",
        "Cook Meals in Kitchen Domes; keep Barns and Aqua Domes producing.",
        "Pumps and Water Generators must supply more than the colony uses.",
    ];
    for i in 0..4 {
        row(ui, |ui| {
            ui::icon(ui, icons, [56, 51, 55, 54][i], 24.0);
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.horizontal(|ui| {
                    ui.label(Readiness::LABELS[i]);
                    ui.label(
                        egui::RichText::new(format!("{:.1} of {:.0}", values[i], targets[i]))
                            .color(if met[i] { GOOD } else { WARN }),
                    );
                });
                ui::meter(
                    ui,
                    values[i] / targets[i],
                    if met[i] { GOOD } else { ACCENT },
                    260.0,
                );
                if !met[i] {
                    ui.label(egui::RichText::new(how[i]).color(TEXT_DIM));
                }
            });
        });
    }
    ui.add_space(6.0);
    match colony.ship {
        Ship::Away => {
            let ready = r.all_met();
            ui.vertical_centered(|ui| {
                if ui
                    .add_enabled(
                        ready,
                        egui::Button::new(egui::RichText::new("Call the ship").size(24.0))
                            .min_size(egui::vec2(260.0, 44.0)),
                    )
                    .on_disabled_hover_text("Every target must be met first.")
                    .clicked()
                {
                    acts.push(Act::CallShip);
                }
                if ready {
                    ui.label(
                        egui::RichText::new(format!(
                            "{} colonists are waiting in orbit.",
                            r.apartments as u32 * COLONISTS_PER_APARTMENT
                        ))
                        .color(GOOD),
                    );
                }
            });
        }
        Ship::Landing(_) => {
            ui.label(egui::RichText::new("The ship is on its way down. Go outside and watch!").color(GOOD));
        }
        Ship::Landed(day) => {
            ui.label(egui::RichText::new(format!("The colony was established on day {day}.")).color(GOOD));
        }
    }
}
