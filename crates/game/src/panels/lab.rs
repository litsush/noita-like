//! The Gene Splicer window and the Codex (plants, creatures, recipes).

use bevy_egui::egui;
use sbct_sim::colony::actions::Action as Act;
use sbct_sim::colony::creatures::CreatureKind;
use sbct_sim::colony::items::{Item, MachineKind};
use sbct_sim::colony::plants::{BASE_SPECIES, Bed, Habitat, Product, Species, SpeciesId, splice};
use sbct_sim::colony::recipes::recipes;
use sbct_sim::colony::{Colony, EntKind, Id, Player};

use super::Panels;
use crate::hud::paint_item;
use crate::ui::{self, ACCENT, DANGER, GOOD, TEXT, TEXT_DIM, UiIcons, WARN};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum CodexTab {
    #[default]
    Plants,
    Creatures,
    Recipes,
}

const GENES: [&str; 8] = [
    "Oxygen", "Growth", "Yield", "Power", "Food", "Light", "Thirst", "Hardy",
];

fn product_name(p: Product) -> &'static str {
    match p {
        Product::Flower => "battery flowers",
        Product::Crop => "food",
        Product::Wood => "wood",
        Product::Fiber => "fiber",
        Product::Fertilizer => "fertilizer",
        Product::Gel => "bio-gel",
        Product::Water => "water",
    }
}

/// Where a species grows, in words.
fn grows_in(sp: &Species) -> String {
    let mut places = Vec::new();
    for (bed, name) in [
        (Bed::Light, "Green Dome"),
        (Bed::Dark, "Dark Dome"),
        (Bed::Water, "Aqua Dome"),
        (Bed::Outdoors, "outdoors"),
    ] {
        let fit = sp.fit(bed);
        if fit >= 1.0 {
            places.push(name.to_string());
        } else if fit > 0.0 {
            places.push(format!("{name} ({:.0}% speed)", fit * 100.0));
        }
    }
    if places.is_empty() {
        "nowhere yet".into()
    } else {
        places.join(", ")
    }
}

/// A swatch in the plant's colours, standing in for a picture.
fn swatch(ui: &mut egui::Ui, sp: &Species, size: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, 3.0, egui::Color32::from_rgb(8, 14, 16));
    let leaf = ui::hue_color(sp.looks.hue_leaf, 0.55, 0.75);
    let bloom = ui::hue_color(sp.looks.hue_bloom, 0.6, 0.95);
    // A stem as tall as the plant is, with a bloom on top if it has one.
    let h = size * (0.3 + sp.looks.height as f32 * 0.055);
    let base = rect.center_bottom() - egui::vec2(0.0, 3.0);
    let lean = (sp.looks.curve as f32 - 3.0) * 0.6;
    let top = base + egui::vec2(lean, -h);
    p.line_segment([base, top], egui::Stroke::new(2.0, leaf));
    for i in 0..(sp.looks.leaves as usize / 2 + 1).min(5) {
        let t = (i as f32 + 1.0) / 6.0;
        let at = base + (top - base) * t;
        let side = if i % 2 == 0 { -1.0 } else { 1.0 };
        p.line_segment(
            [at, at + egui::vec2(side * size * 0.2, -size * 0.06)],
            egui::Stroke::new(2.0, leaf),
        );
    }
    if sp.looks.bloom != 0 {
        p.circle_filled(top, size * 0.11, bloom);
    }
}

/// Gene bars for a species, optionally beside its parents' values.
fn genes_ui(ui: &mut egui::Ui, sp: &Species, parents: Option<(&Species, &Species)>) {
    let g = sp.genes.as_array();
    egui::Grid::new(("genes", sp.id, parents.is_some()))
        .spacing([8.0, 2.0])
        .show(ui, |ui| {
            for (i, name) in GENES.iter().enumerate() {
                ui.label(egui::RichText::new(*name).color(TEXT_DIM));
                ui::meter(ui, g[i] as f32 / 10.0, ACCENT, 90.0);
                ui.label(format!("{}", g[i]));
                if let Some((a, b)) = parents {
                    let (ga, gb) = (a.genes.as_array()[i], b.genes.as_array()[i]);
                    let best = ga.max(gb);
                    let color = if g[i] > best {
                        GOOD
                    } else if g[i] < ga.min(gb) {
                        DANGER
                    } else {
                        TEXT_DIM
                    };
                    ui.label(egui::RichText::new(format!("parents {ga} / {gb}")).color(color));
                }
                ui.end_row();
            }
        });
}

fn species_summary(ui: &mut egui::Ui, colony: &Colony, sp: &Species) {
    let mut makes: Vec<String> = Vec::new();
    for p in [sp.product, sp.second].into_iter().flatten() {
        makes.push(product_name(p).to_string());
    }
    if sp.genes.oxygen >= 5 {
        makes.push("lots of oxygen".into());
    }
    ui.label(format!(
        "Gives: {}",
        if makes.is_empty() {
            "oxygen".to_string()
        } else {
            makes.join(", ")
        }
    ));
    ui.label(format!("Grows in: {}", grows_in(sp)));
    if sp.habitat == Habitat::Aquatic {
        ui.label("Aquatic.");
    }
    if sp.pollinator {
        ui.label("Pollinator: speeds up the other planters in its dome.");
    }
    if let Some((a, b)) = sp.parents {
        let name = |id: SpeciesId| {
            colony
                .species
                .get(id as usize)
                .map_or("?".to_string(), |s| s.name.clone())
        };
        ui.label(
            egui::RichText::new(format!(
                "Hybrid of {} and {} (generation {})",
                name(a),
                name(b),
                sp.generation
            ))
            .color(TEXT_DIM),
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub fn splicer_window(
    ctx: &egui::Context,
    icons: &UiIcons,
    colony: &Colony,
    me: &Player,
    id: Id,
    panels: &mut Panels,
    acts: &mut Vec<Act>,
    close: &mut bool,
) {
    let Some(EntKind::Machine(m)) = colony.ents.get(&id).map(|e| &e.kind) else {
        *close = true;
        return;
    };
    if m.kind != MachineKind::Splicer {
        *close = true;
        return;
    }
    // Seeds the player carries, by species.
    let mut seeds: Vec<(SpeciesId, u32)> = Vec::new();
    for s in me.inv.stacks() {
        if let Item::Seed(sp) = s.item {
            match seeds.iter_mut().find(|(i, _)| *i == sp) {
                Some((_, n)) => *n += s.count,
                None => seeds.push((sp, s.count)),
            }
        }
    }
    seeds.sort();
    let have = |s: Option<SpeciesId>| s.filter(|s| seeds.iter().any(|(i, _)| i == s));
    panels.splice = (have(panels.splice.0), have(panels.splice.1));
    egui::Area::new("splicer".into())
        .anchor(egui::Align2::CENTER_CENTER, [0.0, -20.0])
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            ui::ornate(ui, |ui| {
                ui.set_width(640.0);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(ui::title(&m.name, 24.0).color(ACCENT));
                        ui.label(if m.on {
                            egui::RichText::new("Powered").color(GOOD)
                        } else {
                            egui::RichText::new("No power: put a Charging Pylon with battery flowers nearby")
                                .color(DANGER)
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("x").clicked() {
                                *close = true;
                            }
                        });
                    });
                    ui.label(
                        egui::RichText::new(
                            "Pick two species you carry seeds of. One seed of each goes in; two seeds of their \
                             hybrid come out. The same parents always make the same hybrid.",
                        )
                        .color(TEXT_DIM),
                    );
                    ui.add_space(4.0);
                    if seeds.len() < 2 {
                        ui.label(egui::RichText::new("You need seeds of two different species.").color(WARN));
                        return;
                    }
                    let picker = |ui: &mut egui::Ui, which: usize, pick: &mut Option<SpeciesId>| {
                        ui.vertical(|ui| {
                            ui.set_width(200.0);
                            ui.label(egui::RichText::new(format!("Parent {}", which + 1)).color(TEXT_DIM));
                            egui::ScrollArea::vertical()
                                .id_salt(("parent", which))
                                .max_height(300.0)
                                .min_scrolled_height(300.0)
                                .show(ui, |ui| {
                                    for &(s, n) in &seeds {
                                        let Some(sp) = colony.species.get(s as usize) else {
                                            continue;
                                        };
                                        ui.horizontal(|ui| {
                                            let (rect, _) =
                                                ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::hover());
                                            paint_item(ui.painter(), icons, colony, Item::Seed(s), rect);
                                            if ui
                                                .selectable_label(*pick == Some(s), format!("{}  x{n}", sp.name))
                                                .clicked()
                                            {
                                                *pick = Some(s);
                                            }
                                        });
                                    }
                                });
                        });
                    };
                    ui.horizontal(|ui| {
                        let (mut a, mut b) = panels.splice;
                        picker(ui, 0, &mut a);
                        picker(ui, 1, &mut b);
                        panels.splice = (a, b);
                        ui.vertical(|ui| {
                            ui.set_width(220.0);
                            let (Some(a), Some(b)) = panels.splice else {
                                ui.label(egui::RichText::new("Choose both parents.").color(TEXT_DIM));
                                return;
                            };
                            if a == b {
                                ui.label(egui::RichText::new("Choose two different species.").color(WARN));
                                return;
                            }
                            let (sa, sb) = (&colony.species[a as usize], &colony.species[b as usize]);
                            // The colony may already know this cross; otherwise work it out.
                            let known = colony.hybrid_of(a, b);
                            let child = match known {
                                Some(id) => colony.species[id as usize].clone(),
                                None => splice(colony.seed, sa, sb, me.stats().splice_bonus, &colony.species),
                            };
                            ui.horizontal(|ui| {
                                swatch(ui, sa, 40.0);
                                ui.label("+");
                                swatch(ui, sb, 40.0);
                                ui.label("=");
                                swatch(ui, &child, 48.0);
                            });
                            ui.label(egui::RichText::new(&child.name).color(ACCENT).strong());
                            if known.is_none() {
                                ui.label(egui::RichText::new("A new species").color(GOOD));
                            }
                            species_summary(ui, colony, &child);
                            genes_ui(ui, &child, Some((sa, sb)));
                            ui.add_space(4.0);
                            if ui
                                .add_enabled(m.on, egui::Button::new(egui::RichText::new("Splice").color(ACCENT)))
                                .clicked()
                            {
                                acts.push(Act::Splice { machine: id, a, b });
                            }
                        });
                    });
                });
            });
        });
}

pub fn codex_window(
    ctx: &egui::Context,
    icons: &UiIcons,
    colony: &Colony,
    me: &Player,
    panels: &mut Panels,
    close: &mut bool,
) {
    egui::Area::new("codex".into())
        .anchor(egui::Align2::CENTER_CENTER, [0.0, -10.0])
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            ui::ornate(ui, |ui| {
                ui.set_width(760.0);
                ui.vertical(|ui| {
                    ui.horizontal(|ui| {
                        ui.label(ui::title("Codex", 26.0).color(ACCENT));
                        ui.add_space(10.0);
                        for (tab, label) in [
                            (CodexTab::Plants, "Plants"),
                            (CodexTab::Creatures, "Creatures"),
                            (CodexTab::Recipes, "Recipes"),
                        ] {
                            ui.selectable_value(&mut panels.codex_tab, tab, label);
                        }
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui.small_button("x").clicked() {
                                *close = true;
                            }
                        });
                    });
                    ui.add_space(4.0);
                    match panels.codex_tab {
                        CodexTab::Plants => plants(ui, icons, colony, panels),
                        CodexTab::Creatures => creatures(ui, colony),
                        CodexTab::Recipes => recipe_list(ui, icons, colony, me),
                    }
                });
            });
        });
}

fn plants(ui: &mut egui::Ui, icons: &UiIcons, colony: &Colony, panels: &mut Panels) {
    let known: Vec<&Species> = colony
        .codex
        .plants
        .iter()
        .filter_map(|&s| colony.species.get(s as usize))
        .collect();
    let base_known = known.iter().filter(|s| (s.id as usize) < BASE_SPECIES).count();
    ui.label(
        egui::RichText::new(format!(
            "{base_known} of {BASE_SPECIES} native and Earth species grown, {} hybrids. A species is recorded \
             when the colony first grows or harvests it.",
            known.len() - base_known
        ))
        .color(TEXT_DIM),
    );
    if known.is_empty() {
        return;
    }
    if panels
        .codex_plant
        .is_none_or(|s| !colony.codex.plants.contains(&s))
    {
        panels.codex_plant = Some(known[0].id);
    }
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            ui.set_width(240.0);
            egui::ScrollArea::vertical()
                .id_salt("codex plants")
                .max_height(430.0)
                .min_scrolled_height(430.0)
                .show(ui, |ui| {
                    for sp in &known {
                        ui.horizontal(|ui| {
                            let (rect, _) =
                                ui.allocate_exact_size(egui::vec2(20.0, 20.0), egui::Sense::hover());
                            paint_item(ui.painter(), icons, colony, Item::Seed(sp.id), rect);
                            if ui
                                .selectable_label(panels.codex_plant == Some(sp.id), &sp.name)
                                .clicked()
                            {
                                panels.codex_plant = Some(sp.id);
                            }
                        });
                    }
                });
        });
        ui.vertical(|ui| {
            ui.set_width(480.0);
            let Some(sp) = panels.codex_plant.and_then(|s| colony.species.get(s as usize)) else {
                return;
            };
            ui.horizontal(|ui| {
                swatch(ui, sp, 64.0);
                ui.vertical(|ui| {
                    ui.label(ui::title(&sp.name, 22.0).color(ACCENT));
                    species_summary(ui, colony, sp);
                });
            });
            ui.add_space(6.0);
            let parents = sp
                .parents
                .and_then(|(a, b)| Some((colony.species.get(a as usize)?, colony.species.get(b as usize)?)));
            genes_ui(ui, sp, parents);
            ui.add_space(6.0);
            let mut facts = vec![match sp.product {
                Some(_) => format!("A harvest gives {} unit(s).", sp.harvest_count()),
                None => "Nothing to harvest: it is grown for its oxygen.".to_string(),
            }];
            if sp.yields(Product::Flower) {
                facts.push(format!("Each battery flower holds {:.0} charge.", sp.charge()));
            }
            if sp.yields(Product::Crop) {
                facts.push(format!("Each crop is worth {:.1} food.", sp.food_value()));
            }
            facts.push(if sp.thirst_per_day() > 0.0 {
                format!("Drinks {:.0}% of a planter a day.", sp.thirst_per_day() * 100.0)
            } else {
                "Never needs watering.".to_string()
            });
            facts.push(format!(
                "Releases {:.0} oxygen a day when mature.",
                sp.oxygen_per_day()
            ));
            ui.label(egui::RichText::new(facts.join(" ")).color(TEXT_DIM));
        });
    });
}

fn creatures(ui: &mut egui::Ui, colony: &Colony) {
    ui.label(
        egui::RichText::new(format!(
            "{} of {} creatures met.",
            colony.codex.creatures.len(),
            CreatureKind::ALL.len()
        ))
        .color(TEXT_DIM),
    );
    egui::ScrollArea::vertical()
        .max_height(440.0)
        .min_scrolled_height(440.0)
        .show(ui, |ui| {
            ui.set_width(740.0);
            for kind in CreatureKind::ALL {
                ui::panel_frame()
                    .inner_margin(egui::Margin::same(6))
                    .show(ui, |ui| {
                        ui.set_width(720.0);
                        if colony.codex.creatures.contains(&kind) {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(kind.name()).color(ACCENT).strong());
                                ui.label(
                                    egui::RichText::new(if kind.predator() {
                                        format!("Predator, {:.0} health", kind.max_hp())
                                    } else {
                                        format!("Harmless, {:.0} health", kind.max_hp())
                                    })
                                    .color(if kind.predator() {
                                        WARN
                                    } else {
                                        GOOD
                                    }),
                                );
                            });
                            ui.label(egui::RichText::new(kind.describe()).color(TEXT));
                        } else {
                            ui.label(egui::RichText::new("???  Not met yet.").color(TEXT_DIM));
                        }
                    });
                ui.add_space(2.0);
            }
        });
}

fn recipe_list(ui: &mut egui::Ui, icons: &UiIcons, colony: &Colony, me: &Player) {
    ui.label(
        egui::RichText::new(
            "Everything a Synthesizer can make. Blueprints come from Earth at the Comm. Terminal.",
        )
        .color(TEXT_DIM),
    );
    egui::ScrollArea::vertical()
        .max_height(440.0)
        .min_scrolled_height(440.0)
        .show(ui, |ui| {
            ui.set_width(740.0);
            for (i, r) in recipes().iter().enumerate() {
                ui.horizontal(|ui| {
                    let (rect, resp) = ui.allocate_exact_size(egui::vec2(24.0, 24.0), egui::Sense::hover());
                    paint_item(ui.painter(), icons, colony, r.output, rect);
                    resp.on_hover_text(r.output.describe(&colony.species));
                    let name = colony.item_name(r.output);
                    let title = if r.count > 1 {
                        format!("{name} x{}", r.count)
                    } else {
                        name
                    };
                    ui.add_sized(
                        [210.0, 20.0],
                        egui::Label::new(egui::RichText::new(title).color(TEXT)).truncate(),
                    );
                    for (ing, need) in colony.recipe_cost(me.key, i) {
                        ui::icon(ui, icons, ing.icon(), 16.0).on_hover_text(ing.name(&colony.species));
                        ui.label(format!("{need}"));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        match r.blueprint {
                            None => {
                                ui.label(egui::RichText::new("Known").color(GOOD));
                            }
                            Some(b) if colony.blueprints.contains(&b) => {
                                ui.label(egui::RichText::new("Blueprint owned").color(GOOD));
                            }
                            Some(b) => {
                                ui.label(
                                    egui::RichText::new(format!("Blueprint: {} cr", b.price())).color(WARN),
                                );
                            }
                        }
                    });
                });
            }
        });
}
