//! The robot window: what the robot is doing and why, and the sentence
//! builder that programs it from drop-down word blocks.

use std::hash::Hash;

use bevy_egui::egui;
use sbct_sim::colony::actions::{Action as Act, Container};
use sbct_sim::colony::items::{Item, MachineKind};
use sbct_sim::colony::robots::{
    Clause, Cond, InvState, MAX_SENTENCES, MAX_STEPS, Place, ROBOT_CHARGE, Robot, RobotOp, Sentence, Step,
    TimeState, What, Where,
};
use sbct_sim::colony::{Colony, EntKind, Id};

use super::{Open, Panels};
use crate::ui::{self, ACCENT, DANGER, GOOD, TEXT, TEXT_DIM, UiIcons, WARN};

/// A program being edited, until it is applied.
pub struct Draft {
    pub robot: Id,
    pub program: Vec<Sentence>,
}

/// A drop-down word block. Returns true if the choice changed.
fn pick<T: PartialEq + Clone>(
    ui: &mut egui::Ui,
    id: impl Hash + std::fmt::Debug,
    current: &mut T,
    options: &[(T, String)],
    width: f32,
) -> bool {
    let label = options
        .iter()
        .find(|(v, _)| v == current)
        .map_or("(gone)".to_string(), |(_, l)| l.clone());
    let mut changed = false;
    egui::ComboBox::from_id_salt(id)
        .selected_text(egui::RichText::new(label).color(ACCENT))
        .width(width)
        .show_ui(ui, |ui| {
            for (value, label) in options {
                if ui.selectable_label(value == current, label).clicked() && value != current {
                    *current = value.clone();
                    changed = true;
                }
            }
        });
    changed
}

fn word(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).color(TEXT));
}

fn keyword(ui: &mut egui::Ui, text: &str) {
    ui.label(egui::RichText::new(text).color(WARN).strong());
}

/// The lists the drop-downs choose from.
struct Words {
    places: Vec<(Place, String)>,
    domes: Vec<(Place, String)>,
    areas: Vec<(u8, String)>,
    crops: Vec<(What, String)>,
    seeds: Vec<(What, String)>,
    ores: Vec<(What, String)>,
    items: Vec<(What, String)>,
    stock: Vec<(Item, String)>,
}

impl Words {
    fn new(colony: &Colony) -> Words {
        let mut places = vec![
            (Place::NearestDome, "the nearest dome".to_string()),
            (Place::NearestStorage, "the nearest storage".to_string()),
            (Place::NearestPylon, "the nearest pylon".to_string()),
        ];
        let mut domes = vec![(Place::NearestDome, "the nearest dome".to_string())];
        for e in colony.ents.values() {
            match &e.kind {
                EntKind::Dome(d) => {
                    places.push((Place::Ent(e.id), d.name.clone()));
                    domes.push((Place::Ent(e.id), d.name.clone()));
                }
                EntKind::Machine(m) if matches!(m.kind, MachineKind::Chest | MachineKind::Pylon) => {
                    places.push((Place::Ent(e.id), m.name.clone()));
                }
                _ => {}
            }
        }
        let mut areas: Vec<(u8, String)> = colony.areas.iter().map(|a| (a.id, a.name.clone())).collect();
        if areas.is_empty() {
            areas.push((0, "(mark an area on the map first)".to_string()));
        }
        let known = |pick: &dyn Fn(u16) -> Item| -> Vec<(What, String)> {
            colony
                .codex
                .plants
                .iter()
                .map(|&s| pick(s))
                .map(|i| (What::Item(i), colony.item_name(i).to_lowercase()))
                .collect()
        };
        let mut crops = vec![(What::Anything, "anything".to_string())];
        crops.extend(
            colony
                .codex
                .plants
                .iter()
                .filter(|&&s| {
                    colony
                        .species
                        .get(s as usize)
                        .is_some_and(|sp| sp.product.is_some())
                })
                .map(|&s| {
                    (
                        What::Item(Item::Crop(s)),
                        colony.species[s as usize].name.to_lowercase(),
                    )
                }),
        );
        let mut seeds = vec![(What::Anything, "any seed".to_string())];
        seeds.extend(known(&Item::Seed));
        let mut ores = vec![(What::Anything, "any ore".to_string())];
        let ore_items = [
            Item::Iron,
            Item::Copper,
            Item::Silicon,
            Item::Gold,
            Item::Titanium,
            Item::Xenite,
            Item::Aurorium,
        ];
        ores.extend(
            ore_items
                .iter()
                .map(|&i| (What::Item(i), colony.item_name(i).to_lowercase())),
        );
        let mut stock: Vec<(Item, String)> = Item::FIXED
            .iter()
            .map(|&i| (i, colony.item_name(i).to_lowercase()))
            .collect();
        for &s in &colony.codex.plants {
            for i in [Item::Crop(s), Item::Seed(s)] {
                stock.push((i, colony.item_name(i).to_lowercase()));
            }
        }
        let mut items = vec![(What::Anything, "everything".to_string())];
        items.extend(stock.iter().map(|(i, l)| (What::Item(*i), l.clone())));
        Words {
            places,
            domes,
            areas,
            crops,
            seeds,
            ores,
            items,
            stock,
        }
    }

    fn first_area(&self) -> u8 {
        self.areas[0].0
    }
}

#[derive(Clone, Copy, PartialEq)]
enum CondKind {
    Battery,
    Inventory,
    Time,
    Predator,
    Stock,
}

fn cond_kind(c: &Cond) -> CondKind {
    match c {
        Cond::Battery { .. } => CondKind::Battery,
        Cond::Inventory(_) => CondKind::Inventory,
        Cond::Time(_) => CondKind::Time,
        Cond::Predator { .. } => CondKind::Predator,
        Cond::Stock { .. } => CondKind::Stock,
    }
}

fn default_cond(kind: CondKind) -> Cond {
    match kind {
        CondKind::Battery => Cond::Battery {
            below: true,
            percent: 20,
        },
        CondKind::Inventory => Cond::Inventory(InvState::Full),
        CondKind::Time => Cond::Time(TimeState::Night),
        CondKind::Predator => Cond::Predator { nearby: true },
        CondKind::Stock => Cond::Stock {
            place: Place::NearestPylon,
            less: true,
            count: 5,
            item: Item::Crop(0),
        },
    }
}

fn cond_ui(ui: &mut egui::Ui, id: (usize, usize, u8), cond: &mut Cond, words: &Words) {
    let mut kind = cond_kind(cond);
    let kinds = [
        (CondKind::Battery, "battery".to_string()),
        (CondKind::Inventory, "inventory".to_string()),
        (CondKind::Time, "it".to_string()),
        (CondKind::Predator, "a predator".to_string()),
        (CondKind::Stock, "a place".to_string()),
    ];
    if pick(ui, (id, "ck"), &mut kind, &kinds, 86.0) {
        *cond = default_cond(kind);
    }
    match cond {
        Cond::Battery { below, percent } => {
            word(ui, "is");
            let opts = [(true, "below".to_string()), (false, "above".to_string())];
            pick(ui, (id, "cb"), below, &opts, 62.0);
            let pcts: Vec<(u8, String)> = [10u8, 20, 30, 50, 70, 90]
                .iter()
                .map(|p| (*p, format!("{p}%")))
                .collect();
            pick(ui, (id, "cp"), percent, &pcts, 52.0);
        }
        Cond::Inventory(state) => {
            word(ui, "is");
            let opts = [
                (InvState::Full, "full".to_string()),
                (InvState::Empty, "empty".to_string()),
                (InvState::NotEmpty, "not empty".to_string()),
            ];
            pick(ui, (id, "ci"), state, &opts, 84.0);
        }
        Cond::Time(t) => {
            word(ui, "is");
            let opts = [
                (TimeState::Day, "day".to_string()),
                (TimeState::Night, "night".to_string()),
                (TimeState::Raining, "raining".to_string()),
            ];
            pick(ui, (id, "ct"), t, &opts, 74.0);
        }
        Cond::Predator { nearby } => {
            word(ui, "is");
            let opts = [(true, "nearby".to_string()), (false, "not nearby".to_string())];
            pick(ui, (id, "cn"), nearby, &opts, 90.0);
        }
        Cond::Stock {
            place,
            less,
            count,
            item,
        } => {
            pick(ui, (id, "csp"), place, &words.places, 150.0);
            word(ui, "has");
            let opts = [(true, "less than".to_string()), (false, "more than".to_string())];
            pick(ui, (id, "csl"), less, &opts, 84.0);
            let counts: Vec<(u32, String)> = [1u32, 5, 10, 20, 50, 100, 200]
                .iter()
                .map(|n| (*n, n.to_string()))
                .collect();
            pick(ui, (id, "csc"), count, &counts, 48.0);
            pick(ui, (id, "csi"), item, &words.stock, 150.0);
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum StepKind {
    GoTo,
    FleeTo,
    ChargeAt,
    Harvest,
    Plant,
    Water,
    Fertilize,
    Mine,
    Scavenge,
    Store,
    Take,
    Guard,
    Wait,
}

fn step_kind(s: &Step) -> StepKind {
    match s {
        Step::GoTo(_) => StepKind::GoTo,
        Step::FleeTo(_) => StepKind::FleeTo,
        Step::ChargeAt(_) => StepKind::ChargeAt,
        Step::Harvest { .. } => StepKind::Harvest,
        Step::Plant { .. } => StepKind::Plant,
        Step::Water(_) => StepKind::Water,
        Step::Fertilize(_) => StepKind::Fertilize,
        Step::Mine { .. } => StepKind::Mine,
        Step::Scavenge { .. } => StepKind::Scavenge,
        Step::Store { .. } => StepKind::Store,
        Step::Take { .. } => StepKind::Take,
        Step::Guard { .. } => StepKind::Guard,
        Step::Wait { .. } => StepKind::Wait,
    }
}

fn default_step(kind: StepKind, words: &Words) -> Step {
    let area = words.first_area();
    match kind {
        StepKind::GoTo => Step::GoTo(Place::NearestDome),
        StepKind::FleeTo => Step::FleeTo(Place::NearestDome),
        StepKind::ChargeAt => Step::ChargeAt(Place::NearestPylon),
        StepKind::Harvest => Step::Harvest {
            what: What::Anything,
            at: Where::Place(Place::NearestDome),
        },
        StepKind::Plant => Step::Plant {
            seed: What::Anything,
            at: Place::NearestDome,
        },
        StepKind::Water => Step::Water(Place::NearestDome),
        StepKind::Fertilize => Step::Fertilize(Place::NearestDome),
        StepKind::Mine => Step::Mine {
            ore: What::Anything,
            area,
        },
        StepKind::Scavenge => Step::Scavenge { area },
        StepKind::Store => Step::Store {
            what: What::Anything,
            at: Place::NearestStorage,
        },
        StepKind::Take => Step::Take {
            what: What::Anything,
            from: Place::NearestStorage,
        },
        StepKind::Guard => Step::Guard { area },
        StepKind::Wait => Step::Wait { secs: 30 },
    }
}

fn step_ui(ui: &mut egui::Ui, id: (usize, usize, u8), step: &mut Step, words: &Words) {
    let mut kind = step_kind(step);
    let kinds = [
        (StepKind::GoTo, "go to"),
        (StepKind::FleeTo, "flee to"),
        (StepKind::ChargeAt, "charge at"),
        (StepKind::Harvest, "harvest"),
        (StepKind::Plant, "plant"),
        (StepKind::Water, "water"),
        (StepKind::Fertilize, "fertilize"),
        (StepKind::Mine, "mine"),
        (StepKind::Scavenge, "scavenge"),
        (StepKind::Store, "store"),
        (StepKind::Take, "take"),
        (StepKind::Guard, "guard"),
        (StepKind::Wait, "wait"),
    ]
    .map(|(k, l)| (k, l.to_string()));
    if pick(ui, (id, "sk"), &mut kind, &kinds, 82.0) {
        *step = default_step(kind, words);
    }
    match step {
        Step::GoTo(p) | Step::FleeTo(p) | Step::ChargeAt(p) => {
            pick(ui, (id, "sp"), p, &words.places, 160.0);
        }
        Step::Harvest { what, at } => {
            pick(ui, (id, "sw"), what, &words.crops, 130.0);
            word(ui, "in");
            // A dome, or an area outdoors.
            let mut options: Vec<(Where, String)> = words
                .domes
                .iter()
                .map(|(p, l)| (Where::Place(*p), l.clone()))
                .collect();
            options.extend(
                words
                    .areas
                    .iter()
                    .map(|(a, l)| (Where::Area(*a), format!("area: {l}"))),
            );
            pick(ui, (id, "sa"), at, &options, 170.0);
        }
        Step::Plant { seed, at } => {
            pick(ui, (id, "sw"), seed, &words.seeds, 130.0);
            word(ui, "in");
            pick(ui, (id, "sp"), at, &words.domes, 160.0);
        }
        Step::Water(p) | Step::Fertilize(p) => {
            word(ui, "plants in");
            pick(ui, (id, "sp"), p, &words.domes, 160.0);
        }
        Step::Mine { ore, area } => {
            pick(ui, (id, "sw"), ore, &words.ores, 100.0);
            word(ui, "in");
            pick(ui, (id, "sa"), area, &words.areas, 160.0);
        }
        Step::Scavenge { area } => {
            word(ui, "in");
            pick(ui, (id, "sa"), area, &words.areas, 160.0);
        }
        Step::Guard { area } => {
            pick(ui, (id, "sa"), area, &words.areas, 160.0);
        }
        Step::Store { what, at } => {
            pick(ui, (id, "sw"), what, &words.items, 130.0);
            word(ui, "in");
            pick(ui, (id, "sp"), at, &words.places, 160.0);
        }
        Step::Take { what, from } => {
            pick(ui, (id, "sw"), what, &words.items, 130.0);
            word(ui, "from");
            pick(ui, (id, "sp"), from, &words.places, 160.0);
        }
        Step::Wait { secs } => {
            let opts: Vec<(u16, String)> = [5u16, 10, 30, 60, 120, 300, 600]
                .iter()
                .map(|s| (*s, format!("{s} seconds")))
                .collect();
            pick(ui, (id, "ss"), secs, &opts, 110.0);
        }
    }
}

/// Ready-made sentences for the "Add" menu.
fn templates(words: &Words) -> Vec<(&'static str, Sentence)> {
    let one = |step: Step| vec![Clause::from(step)];
    let area = words.first_area();
    vec![
        (
            "Blank sentence",
            Sentence {
                when: None,
                steps: one(Step::Wait { secs: 30 }),
            },
        ),
        (
            "Charge when the battery is low",
            Sentence {
                when: Some(Cond::Battery {
                    below: true,
                    percent: 20,
                }),
                steps: one(Step::ChargeAt(Place::NearestPylon)),
            },
        ),
        (
            "Flee from predators",
            Sentence {
                when: Some(Cond::Predator { nearby: true }),
                steps: one(Step::FleeTo(Place::NearestDome)),
            },
        ),
        (
            "Harvest, then store",
            Sentence {
                when: None,
                steps: vec![
                    Step::Harvest {
                        what: What::Anything,
                        at: Where::Place(Place::NearestDome),
                    }
                    .into(),
                    Step::Store {
                        what: What::Anything,
                        at: Place::NearestStorage,
                    }
                    .into(),
                ],
            },
        ),
        (
            "Mine until full, then store",
            Sentence {
                when: None,
                steps: vec![
                    Clause {
                        step: Step::Mine {
                            ore: What::Anything,
                            area,
                        },
                        until: Some(Cond::Inventory(InvState::Full)),
                    },
                    Step::Store {
                        what: What::Anything,
                        at: Place::NearestStorage,
                    }
                    .into(),
                ],
            },
        ),
        (
            "Scavenge an area, then store",
            Sentence {
                when: None,
                steps: vec![
                    Step::Scavenge { area }.into(),
                    Step::Store {
                        what: What::Anything,
                        at: Place::NearestStorage,
                    }
                    .into(),
                ],
            },
        ),
        (
            "Keep a pylon stocked with flowers",
            Sentence {
                when: Some(Cond::Stock {
                    place: Place::NearestPylon,
                    less: true,
                    count: 5,
                    item: Item::Crop(0),
                }),
                steps: vec![
                    Step::Take {
                        what: What::Item(Item::Crop(0)),
                        from: Place::NearestStorage,
                    }
                    .into(),
                    Step::Store {
                        what: What::Item(Item::Crop(0)),
                        at: Place::NearestPylon,
                    }
                    .into(),
                ],
            },
        ),
        (
            "Water the plants",
            Sentence {
                when: None,
                steps: one(Step::Water(Place::NearestDome)),
            },
        ),
        (
            "Replant from seeds on board",
            Sentence {
                when: None,
                steps: one(Step::Plant {
                    seed: What::Anything,
                    at: Place::NearestDome,
                }),
            },
        ),
        (
            "Guard an area",
            Sentence {
                when: None,
                steps: one(Step::Guard { area }),
            },
        ),
    ]
}

#[allow(clippy::too_many_arguments)]
pub fn window(
    ctx: &egui::Context,
    icons: &UiIcons,
    colony: &Colony,
    id: Id,
    panels: &mut Panels,
    acts: &mut Vec<Act>,
    close: &mut bool,
) {
    let Some(EntKind::Robot(robot)) = colony.ents.get(&id).map(|e| &e.kind) else {
        *close = true;
        return;
    };
    // Start a draft from the robot's program the first time it is opened.
    if panels.robot_draft.as_ref().is_none_or(|d| d.robot != id) {
        panels.robot_draft = Some(Draft {
            robot: id,
            program: robot.program.clone(),
        });
    }
    let words = Words::new(colony);
    let mut open_cargo = false;
    egui::Area::new("robot".into())
        .anchor(egui::Align2::CENTER_CENTER, [0.0, -10.0])
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            ui::ornate(ui, |ui| {
                ui.set_width(880.0);
                ui.vertical(|ui| {
                    header(ui, icons, robot, id, panels, acts, close, &mut open_cargo);
                    ui.add_space(4.0);
                    program(ui, colony, robot, id, panels, acts, &words);
                });
            });
        });
    if open_cargo {
        panels.open = Some(Open::Container(Container::Machine(id)));
    }
}

#[allow(clippy::too_many_arguments)]
fn header(
    ui: &mut egui::Ui,
    icons: &UiIcons,
    robot: &Robot,
    id: Id,
    panels: &mut Panels,
    acts: &mut Vec<Act>,
    close: &mut bool,
    open_cargo: &mut bool,
) {
    ui.horizontal(|ui| {
        let editing = matches!(&panels.rename, Some((e, _)) if *e == id);
        if editing {
            let (_, text) = panels.rename.as_mut().unwrap();
            ui.add(
                egui::TextEdit::singleline(text)
                    .desired_width(180.0)
                    .char_limit(28),
            );
            if ui.button("Save").clicked() {
                let (_, text) = panels.rename.take().unwrap();
                acts.push(Act::Rename { ent: id, name: text });
            }
        } else {
            ui.label(ui::title(&robot.name, 24.0).color(ACCENT));
            if ui.small_button("rename").clicked() {
                panels.rename = Some((id, robot.name.clone()));
            }
        }
        ui.add_space(12.0);
        ui::icon(ui, icons, 57, 16.0);
        let charge = robot.charge / ROBOT_CHARGE;
        ui::meter(ui, charge, if charge < 0.2 { DANGER } else { ui::POWER }, 110.0);
        ui.label(egui::RichText::new(format!("{:.0}%", charge * 100.0)).color(TEXT_DIM));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui.small_button("x").clicked() {
                *close = true;
            }
            if ui
                .button("Pick up")
                .on_hover_text("Pack the robot back into a kit. Its cargo is dropped.")
                .clicked()
            {
                acts.push(Act::Robot {
                    id,
                    op: RobotOp::PickUp,
                });
                *close = true;
            }
            let cargo: u32 = robot.inv.stacks().map(|s| s.count).sum();
            if ui.button(format!("Cargo ({cargo})")).clicked() {
                *open_cargo = true;
            }
            let label = if robot.paused { "Resume" } else { "Pause" };
            if ui.button(label).clicked() {
                acts.push(Act::Robot {
                    id,
                    op: RobotOp::Pause(!robot.paused),
                });
            }
        });
    });
    // What it is doing, and which sentence is the reason.
    ui.horizontal(|ui| {
        let color = if robot.active.is_some() { GOOD } else { WARN };
        let why = match robot.active {
            Some(i) => format!("{}  (sentence {})", robot.status, i + 1),
            None => robot.status.clone(),
        };
        ui.label(egui::RichText::new(why).color(color));
    });
}

fn program(
    ui: &mut egui::Ui,
    colony: &Colony,
    robot: &Robot,
    id: Id,
    panels: &mut Panels,
    acts: &mut Vec<Act>,
    words: &Words,
) {
    let draft = panels.robot_draft.as_mut().unwrap();
    let applied = draft.program == robot.program;
    ui.label(
        egui::RichText::new(
            "Sentences are tried from the top every moment: the first one that can run is what the robot \
             does, so put the urgent ones first.",
        )
        .color(TEXT_DIM),
    );
    ui.add_space(2.0);
    let mut remove = None;
    let mut swap = None;
    let count = draft.program.len();
    egui::ScrollArea::vertical()
        .max_height(430.0)
        .min_scrolled_height(300.0)
        .show(ui, |ui| {
            ui.set_width(864.0);
            for (i, sentence) in draft.program.iter_mut().enumerate() {
                // Green: this is what the robot is doing. Amber: it can't run.
                let active = applied && robot.active == Some(i as u8);
                let blocked = if applied {
                    robot.blocked.get(i).cloned().flatten()
                } else {
                    None
                };
                let (fill, edge) = if active {
                    (egui::Color32::from_rgb(22, 58, 38), GOOD)
                } else if blocked.is_some() {
                    (egui::Color32::from_rgb(64, 46, 18), WARN)
                } else {
                    (egui::Color32::from_rgb(18, 30, 32), ui::RULE)
                };
                egui::Frame::new()
                    .fill(fill)
                    .stroke(egui::Stroke::new(1.0, edge))
                    .inner_margin(6.0)
                    .show(ui, |ui| {
                        ui.set_width(846.0);
                        ui.vertical(|ui| {
                            ui.set_min_width(846.0);
                            ui.horizontal_wrapped(|ui| {
                                ui.label(egui::RichText::new(format!("{}.", i + 1)).color(TEXT_DIM));
                                match &mut sentence.when {
                                    Some(cond) => {
                                        keyword(ui, "WHEN");
                                        cond_ui(ui, (i, 99, 0), cond, words);
                                        if ui
                                            .small_button("x")
                                            .on_hover_text("Remove the condition")
                                            .clicked()
                                        {
                                            sentence.when = None;
                                        }
                                        word(ui, ",");
                                    }
                                    None => {
                                        if ui
                                            .small_button("+ WHEN")
                                            .on_hover_text("Only run this sentence while something is true")
                                            .clicked()
                                        {
                                            sentence.when = Some(default_cond(CondKind::Battery));
                                        }
                                    }
                                }
                                let steps = sentence.steps.len();
                                let mut drop_step = None;
                                for (j, clause) in sentence.steps.iter_mut().enumerate() {
                                    if j > 0 {
                                        keyword(ui, ", then");
                                    }
                                    step_ui(ui, (i, j, 1), &mut clause.step, words);
                                    match &mut clause.until {
                                        Some(cond) => {
                                            keyword(ui, "UNTIL");
                                            cond_ui(ui, (i, j, 2), cond, words);
                                            if ui.small_button("x").on_hover_text("Remove UNTIL").clicked() {
                                                clause.until = None;
                                            }
                                        }
                                        None => {
                                            if ui
                                                .small_button("+ UNTIL")
                                                .on_hover_text(
                                                    "End this step early when something becomes true",
                                                )
                                                .clicked()
                                            {
                                                clause.until = Some(default_cond(CondKind::Inventory));
                                            }
                                        }
                                    }
                                    if steps > 1
                                        && ui.small_button("-").on_hover_text("Remove this step").clicked()
                                    {
                                        drop_step = Some(j);
                                    }
                                }
                                if let Some(j) = drop_step {
                                    sentence.steps.remove(j);
                                }
                                if sentence.steps.len() < MAX_STEPS
                                    && ui
                                        .small_button("+ then")
                                        .on_hover_text("Add another step")
                                        .clicked()
                                {
                                    sentence.steps.push(default_step(StepKind::Store, words).into());
                                }
                            });
                            ui.horizontal(|ui| {
                                // The sentence as plain words, and why it can't run.
                                ui.label(
                                    egui::RichText::new(sentence.text(colony))
                                        .color(TEXT_DIM)
                                        .italics(),
                                );
                                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                    if ui.small_button("remove").clicked() {
                                        remove = Some(i);
                                    }
                                    if i + 1 < count && ui.small_button("down").clicked() {
                                        swap = Some((i, i + 1));
                                    }
                                    if i > 0 && ui.small_button("up").clicked() {
                                        swap = Some((i, i - 1));
                                    }
                                });
                            });
                            if active {
                                ui.label(
                                    egui::RichText::new(format!("Running: {}", robot.status)).color(GOOD),
                                );
                            }
                            if let Some(reason) = blocked {
                                ui.label(egui::RichText::new(format!("Can't run: {reason}")).color(WARN));
                            }
                        });
                    });
                ui.add_space(3.0);
            }
            if draft.program.is_empty() {
                ui.label(
                    egui::RichText::new(
                        "No sentences yet. Add one below: \"Harvest, then store\" is a good start.",
                    )
                    .color(TEXT_DIM),
                );
            }
        });
    if let Some(i) = remove {
        draft.program.remove(i);
    }
    if let Some((a, b)) = swap {
        draft.program.swap(a, b);
    }
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        ui.add_enabled_ui(draft.program.len() < MAX_SENTENCES, |ui| {
            ui.menu_button("+ Add a sentence", |ui| {
                for (label, sentence) in templates(words) {
                    if ui.button(label).clicked() {
                        draft.program.push(sentence);
                        ui.close();
                    }
                }
            });
        });
        ui.label(egui::RichText::new(format!("{}/{MAX_SENTENCES}", draft.program.len())).color(TEXT_DIM));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let changed = draft.program != robot.program;
            if ui
                .add_enabled(
                    changed,
                    egui::Button::new(egui::RichText::new("Apply program").color(ACCENT)),
                )
                .clicked()
            {
                acts.push(Act::Robot {
                    id,
                    op: RobotOp::SetProgram(draft.program.clone()),
                });
            }
            if ui.add_enabled(changed, egui::Button::new("Revert")).clicked() {
                draft.program = robot.program.clone();
            }
            if changed {
                ui.label(egui::RichText::new("Not applied yet").color(WARN));
            }
        });
    });
}

/// Speech bubbles over robots: what each one is doing.
pub fn bubbles(
    ctx: &egui::Context,
    colony: &Colony,
    cursor: sbct_sim::colony::geom::V2,
    me: sbct_sim::colony::geom::V2,
    to_screen: &dyn Fn(sbct_sim::colony::geom::V2) -> Option<egui::Pos2>,
) {
    let painter = ctx.layer_painter(egui::LayerId::background());
    for e in colony.ents.values() {
        let EntKind::Robot(r) = &e.kind else { continue };
        let stuck = r.active.is_none() && !r.paused;
        // Always shown when it needs attention; otherwise when you are near or pointing at it.
        let near = e.pos.distance(cursor) < 26.0 || e.pos.distance(me) < 70.0;
        if !near && !(stuck && e.pos.distance(me) < 260.0) {
            continue;
        }
        let at = sbct_sim::colony::geom::V2 {
            x: e.pos.x,
            y: e.pos.y - 15.0,
        };
        let Some(pos) = to_screen(at) else { continue };
        let text = match r.active {
            Some(i) if near => format!("{}: {} (sentence {})", r.name, r.status, i + 1),
            Some(_) => r.status.clone(),
            None if near => format!("{}: {}", r.name, r.status),
            None => format!("! {}", r.status),
        };
        let color = if stuck { WARN } else { TEXT };
        let galley = painter.layout_no_wrap(text, egui::FontId::proportional(13.0), color);
        let rect = egui::Rect::from_center_size(pos, galley.size() + egui::vec2(10.0, 4.0));
        painter.rect_filled(rect, 3.0, egui::Color32::from_black_alpha(185));
        painter.rect_stroke(
            rect,
            3.0,
            egui::Stroke::new(1.0, if stuck { WARN } else { ui::BORDER }),
            egui::StrokeKind::Inside,
        );
        // The bubble's tail.
        painter.add(egui::Shape::convex_polygon(
            vec![
                egui::pos2(pos.x - 4.0, rect.max.y - 0.5),
                egui::pos2(pos.x + 4.0, rect.max.y - 0.5),
                egui::pos2(pos.x, rect.max.y + 5.0),
            ],
            egui::Color32::from_black_alpha(185),
            egui::Stroke::NONE,
        ));
        painter.galley(rect.min + egui::vec2(5.0, 2.0), galley, color);
    }
}
