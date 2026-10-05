//! Windows over the game: the inventory and containers, the Synthesizer,
//! the Comm. Terminal and the other things players open by interacting.
//! Also finds what the player can interact with and shows the prompt.

mod dome;
mod inventory;
mod synth;
mod terminal;

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use sbct_sim::colony::actions::{Action as Act, Container};
use sbct_sim::colony::domes::{Fixture, fixture_pos};
use sbct_sim::colony::farming::PlanterOp;
use sbct_sim::colony::geom::V2;
use sbct_sim::colony::items::MachineKind;
use sbct_sim::colony::{Colony, CrateKind, EntKind, Id, PLAYER_HEIGHT, Player};

use crate::audio::Sfx;
use crate::controls::{Action, Rebind};
use crate::hud::Toasts;
use crate::player::LocalPlayer;
use crate::render::{UiHasPointer, WorldCamera};
use crate::session::Session;
use crate::settings::{Config, SettingsTab};
use crate::ui::{self, ACCENT, UiIcons};

/// What is open besides the HUD.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Open {
    /// Just the player's inventory.
    Inventory,
    /// A chest, crate or pylon, next to the inventory.
    Container(Container),
    Synth,
    Terminal,
    /// A dome's status, planters and production.
    Dome(Id),
    /// A machine without an inventory of its own.
    Machine(Id),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TerminalTab {
    #[default]
    Sell,
    Buy,
    Blueprints,
    Workers,
    Colony,
}

/// Which windows are open over the game, and their transient state.
#[derive(Resource, Default)]
pub struct Panels {
    pub pause: bool,
    pub settings: bool,
    pub settings_tab: SettingsTab,
    /// A text field has keyboard focus, so keys shouldn't move the player.
    pub typing: bool,
    pub open: Option<Open>,
    /// The slot whose stack is being dragged.
    pub dragging: Option<(Container, u16)>,
    /// Filter typed into the inventory's search box.
    pub search: String,
    pub synth_filter: String,
    pub terminal_tab: TerminalTab,
    /// The name being typed in a rename box, and for which entity.
    pub rename: Option<(Id, String)>,
}

impl Panels {
    pub fn blocks_movement(&self) -> bool {
        self.pause || self.typing
    }

    pub fn close(&mut self) {
        self.open = None;
        self.dragging = None;
    }
}

/// Something in the world the player can use.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Target {
    Fixture(Id, Fixture),
    Machine(Id, MachineKind),
    Crate(Id, CrateKind),
    /// A plant growing outside a planter.
    Plant(Id),
}

/// The thing the Interact key would use right now, and where it is.
#[derive(Resource, Default)]
pub struct Focus(pub Option<(Target, V2)>);

impl Target {
    /// What the Interact key would do, for the prompt.
    fn label(&self, colony: &Colony, me: &Player) -> String {
        match *self {
            Target::Fixture(dome, f) => {
                let Some((_, d)) = colony.dome(dome) else {
                    return String::new();
                };
                match f {
                    Fixture::Chest(i) => d.kind.chest_label(i as usize).to_string(),
                    Fixture::Bunk(_) => {
                        if colony.clock.is_night() {
                            "Sleep".into()
                        } else {
                            "Set your spawn here".into()
                        }
                    }
                    Fixture::SuitRack => "Suit rack".into(),
                    Fixture::Synthesizer => "Synthesizer".into(),
                    Fixture::Terminal => "Comm. Terminal".into(),
                    Fixture::ModBay => "Mod Bay".into(),
                    Fixture::Planter(i) => match planter_action(colony, me, dome, i) {
                        Some((_, label)) => label,
                        None => planter_status(colony, dome, i),
                    },
                    Fixture::Cooker => "Cooker".into(),
                    Fixture::CompostVat => "Compost Vat".into(),
                    Fixture::AssemblyRing => "Assembly ring".into(),
                    Fixture::FishPool => "Fish pool".into(),
                    Fixture::Stall(_) => "Stall".into(),
                    Fixture::Condenser => "Condenser".into(),
                    Fixture::Berths => "Colonist berths".into(),
                }
            }
            Target::Machine(_, kind) => kind.name().to_string(),
            Target::Crate(_, CrateKind::Pack) => "Lost pack".into(),
            Target::Crate(_, CrateKind::Cache) => "Salvage cache".into(),
            Target::Crate(_, CrateKind::Pod) => "Drop pod".into(),
            Target::Plant(id) => {
                let Some(EntKind::Plant(p)) = colony.ents.get(&id).map(|e| &e.kind) else {
                    return String::new();
                };
                let name = colony
                    .species
                    .get(p.species as usize)
                    .map_or("Plant", |s| s.name.as_str());
                if p.growth >= 1.0 {
                    format!("Harvest {name}")
                } else {
                    format!("{name}: {:.0}% grown", p.growth * 100.0)
                }
            }
        }
    }
}

/// A one-line description of a planter.
pub fn planter_status(colony: &Colony, dome: Id, slot: u8) -> String {
    let Some(p) = colony.dome(dome).and_then(|(_, d)| d.planters.get(slot as usize)) else {
        return String::new();
    };
    match p.species.and_then(|s| colony.species.get(s as usize)) {
        None => "Empty planter: hold a seed".into(),
        Some(sp) => {
            let water = if sp.thirst_per_day() == 0.0 {
                ""
            } else if p.moisture <= 0.0 {
                ", dry"
            } else if p.moisture < 0.3 {
                ", nearly dry"
            } else {
                ""
            };
            format!("{}: {:.0}% grown{water}", sp.name, p.growth.min(1.0) * 100.0)
        }
    }
}

/// What interacting with a planter would do given what the player holds.
pub fn planter_action(colony: &Colony, me: &Player, dome: Id, slot: u8) -> Option<(PlanterOp, String)> {
    use sbct_sim::colony::items::Item;
    let p = colony.dome(dome)?.1.planters.get(slot as usize)?;
    let held = me.held().map(|s| s.item);
    let sp = p.species.and_then(|s| colony.species.get(s as usize));
    match (sp, held) {
        (None, Some(Item::Seed(s))) => {
            let name = colony
                .species
                .get(s as usize)
                .map_or("seed", |sp| sp.name.as_str());
            Some((PlanterOp::Plant, format!("Plant {name}")))
        }
        (Some(sp), _) if p.growth >= 1.0 && sp.product.is_some() => {
            Some((PlanterOp::Harvest, format!("Harvest {}", sp.name)))
        }
        (Some(sp), Some(Item::WateringCan)) if p.moisture < 0.9 && sp.thirst_per_day() > 0.0 => {
            Some((PlanterOp::Water, format!("Water {}", sp.name)))
        }
        (Some(sp), Some(Item::Fertilizer)) if p.fertilizer <= 0.0 => {
            Some((PlanterOp::Fertilize, format!("Fertilize {}", sp.name)))
        }
        _ => None,
    }
}

/// Everything usable within the player's reach, with its position.
fn targets(colony: &Colony, me: &Player) -> Vec<(Target, V2)> {
    let reach = me.stats().reach;
    let at = me.center();
    let mut out = Vec::new();
    for e in colony.ents.values() {
        match &e.kind {
            EntKind::Dome(d) => {
                if Some(e.id) != me.in_dome {
                    continue;
                }
                for f in d.kind.fixtures() {
                    let mut p = fixture_pos(e.pos, f);
                    p.y -= 8.0;
                    out.push((Target::Fixture(e.id, f.fixture), p));
                }
            }
            EntKind::Machine(m) => out.push((
                Target::Machine(e.id, m.kind),
                V2 {
                    x: e.pos.x,
                    y: e.pos.y - 8.0,
                },
            )),
            EntKind::Crate(c) => out.push((
                Target::Crate(e.id, c.kind),
                V2 {
                    x: e.pos.x,
                    y: e.pos.y - 6.0,
                },
            )),
            EntKind::Plant(_) => out.push((
                Target::Plant(e.id),
                V2 {
                    x: e.pos.x,
                    y: e.pos.y - 8.0,
                },
            )),
            _ => {}
        }
    }
    out.retain(|(_, p)| p.distance(at) <= reach);
    out
}

/// Picks the interaction target: what the cursor points at if it points at
/// something usable, otherwise whatever is closest to the player.
pub fn find_focus(session: Res<Session>, local: Res<LocalPlayer>, mut focus: ResMut<Focus>) {
    focus.0 = None;
    let (Some(colony), Some(me)) = (&session.colony, session.player()) else {
        return;
    };
    if !me.alive() {
        return;
    }
    let all = targets(colony, me);
    let near = |p: V2, to: V2| p.distance(to);
    let by_cursor = all
        .iter()
        .filter(|(_, p)| near(*p, local.cursor) < 14.0)
        .min_by(|a, b| near(a.1, local.cursor).total_cmp(&near(b.1, local.cursor)));
    let at = me.center();
    let by_player = all
        .iter()
        .filter(|(_, p)| near(*p, at) < 22.0)
        .min_by(|a, b| near(a.1, at).total_cmp(&near(b.1, at)));
    focus.0 = by_cursor.or(by_player).copied();
}

/// Handles the Interact and Inventory keys.
pub fn interact(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    config: Res<Config>,
    rebind: Res<Rebind>,
    focus: Res<Focus>,
    mut session: ResMut<Session>,
    mut panels: ResMut<Panels>,
    mut toasts: ResMut<Toasts>,
    mut sfx: MessageWriter<Sfx>,
) {
    if rebind.blocking() || panels.pause {
        return;
    }
    let b = &config.bindings;
    // Walking away from what is open closes it.
    if let (Some(open), Some(colony), Some(me)) = (panels.open, &session.colony, session.player()) {
        let still_there = match open {
            Open::Inventory => me.alive(),
            Open::Container(c) => colony.containers_near(me.key).contains(&c) || crate_near(colony, me, c),
            Open::Synth => colony.synthesizer_near(me.key),
            Open::Terminal => colony.terminal_near(me.key),
            Open::Dome(id) => me.in_dome == Some(id),
            Open::Machine(id) => crate_near(colony, me, Container::Machine(id)),
        };
        if !still_there {
            panels.close();
            sfx.write(Sfx::ui("inv_close"));
        }
    }
    if panels.typing {
        return;
    }
    if b.just_pressed(Action::Inventory, &keys, &mouse) {
        if panels.open.is_some() {
            panels.close();
            sfx.write(Sfx::ui("inv_close"));
        } else {
            panels.open = Some(Open::Inventory);
            sfx.write(Sfx::ui("inv_open"));
        }
        return;
    }
    if b.just_pressed(Action::QuickStack, &keys, &mouse) {
        session.act(Act::Inv(sbct_sim::colony::actions::InvOp::QuickStackNearby));
        return;
    }
    if !b.just_pressed(Action::Interact, &keys, &mouse) {
        return;
    }
    if panels.open.is_some() {
        panels.close();
        sfx.write(Sfx::ui("inv_close"));
        return;
    }
    let Some((target, _)) = focus.0 else { return };
    let (Some(colony), Some(me)) = (&session.colony, session.player()) else {
        return;
    };
    let mut act = None;
    let open = match target {
        Target::Fixture(dome, Fixture::Chest(i)) => Some(Open::Container(Container::DomeChest(dome, i))),
        Target::Fixture(_, Fixture::Synthesizer) => Some(Open::Synth),
        Target::Fixture(_, Fixture::Terminal) => Some(Open::Terminal),
        Target::Fixture(_, Fixture::SuitRack) => {
            toasts.push("Your suit seals by itself when you step outside.", true);
            None
        }
        Target::Fixture(_, Fixture::Bunk(_)) => {
            act = Some(Act::Sleep);
            None
        }
        Target::Fixture(_, Fixture::ModBay) => {
            toasts.push("The Mod Bay isn't wired up yet.", false);
            None
        }
        Target::Fixture(dome, Fixture::Planter(i)) => match planter_action(colony, me, dome, i) {
            Some((op, _)) => {
                act = Some(Act::Planter { dome, slot: i, op });
                None
            }
            None => Some(Open::Dome(dome)),
        },
        Target::Fixture(dome, _) => Some(Open::Dome(dome)),
        Target::Machine(id, MachineKind::Chest | MachineKind::Pylon) => {
            Some(Open::Container(Container::Machine(id)))
        }
        Target::Machine(_, MachineKind::Synthesizer) => Some(Open::Synth),
        Target::Machine(id, _) => Some(Open::Machine(id)),
        Target::Crate(id, _) => Some(Open::Container(Container::Crate(id))),
        Target::Plant(id) => {
            act = Some(Act::Harvest { plant: id });
            None
        }
    };
    if let Some(act) = act {
        session.act(act);
    }
    if let Some(open) = open {
        panels.open = Some(open);
        sfx.write(Sfx::ui("inv_open"));
    }
}

fn crate_near(colony: &Colony, me: &Player, c: Container) -> bool {
    match c {
        Container::Crate(id) | Container::Machine(id) => colony
            .ents
            .get(&id)
            .is_some_and(|e| e.pos.distance(me.pose.pos) <= me.stats().reach + 16.0),
        _ => false,
    }
}

/// Draws the interaction prompt and whichever panels are open.
pub fn panels_ui(
    mut contexts: EguiContexts,
    mut session: ResMut<Session>,
    mut panels: ResMut<Panels>,
    mut pointer: ResMut<UiHasPointer>,
    mut sfx: MessageWriter<Sfx>,
    focus: Res<Focus>,
    config: Res<Config>,
    icons: Option<Res<UiIcons>>,
    camera: Single<(&Camera, &GlobalTransform), With<WorldCamera>>,
    keys: Res<ButtonInput<KeyCode>>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let Some(icons) = icons else { return Ok(()) };
    let (cam, cam_tf) = *camera;
    let me_key = session.me;
    let (Some(colony), Some(me)) = (&session.colony, session.player()) else {
        return Ok(());
    };
    let zoom = ctx.zoom_factor();
    let mut acts: Vec<Act> = Vec::new();

    // The prompt over whatever the Interact key would use.
    if panels.open.is_none()
        && !panels.pause
        && let Some((target, at)) = focus.0
    {
        let world = Vec3::new(at.x, -(at.y - 14.0), 0.0);
        if let Ok(pos) = cam.world_to_viewport(cam_tf, world) {
            let painter = ctx.layer_painter(egui::LayerId::background());
            let text = format!(
                "[{}] {}",
                config.bindings.hint(Action::Interact),
                target.label(colony, me)
            );
            let at = egui::pos2(pos.x / zoom, pos.y / zoom);
            let font = egui::FontId::proportional(16.0);
            let galley = painter.layout_no_wrap(text, font, ACCENT);
            let rect = egui::Rect::from_center_size(at, galley.size() + egui::vec2(10.0, 4.0));
            painter.rect_filled(rect, 0.0, egui::Color32::from_black_alpha(190));
            painter.rect_stroke(
                rect,
                0.0,
                egui::Stroke::new(1.0, ui::BORDER),
                egui::StrokeKind::Inside,
            );
            painter.galley(rect.min + egui::vec2(5.0, 2.0), galley, ACCENT);
        }
    }

    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let alt = keys.pressed(KeyCode::AltLeft) || keys.pressed(KeyCode::AltRight);
    let mods = inventory::Mods { shift, alt };
    let mut close = false;
    match panels.open {
        Some(Open::Inventory) => {
            inventory::window(
                ctx,
                &icons,
                colony,
                me,
                None,
                &mut panels,
                mods,
                &mut acts,
                &mut close,
            );
        }
        Some(Open::Container(c)) => {
            inventory::window(
                ctx,
                &icons,
                colony,
                me,
                Some(c),
                &mut panels,
                mods,
                &mut acts,
                &mut close,
            );
        }
        Some(Open::Synth) => {
            synth::window(ctx, &icons, colony, me, &mut panels, &mut acts, &mut close);
            inventory::window(
                ctx,
                &icons,
                colony,
                me,
                None,
                &mut panels,
                mods,
                &mut acts,
                &mut close,
            );
        }
        Some(Open::Terminal) => {
            terminal::window(ctx, &icons, colony, me, &mut panels, &mut acts, &mut close);
        }
        Some(Open::Dome(id)) => {
            dome::dome_window(ctx, &icons, colony, me, id, &mut panels, &mut acts, &mut close);
        }
        Some(Open::Machine(id)) => {
            dome::machine_window(ctx, &icons, colony, id, &mut panels, &mut acts, &mut close);
        }
        None => {}
    }
    if close {
        panels.close();
        sfx.write(Sfx::ui("inv_close"));
    }
    if panels.open.is_some() {
        pointer.0 |= ctx.is_pointer_over_egui();
        panels.typing |= ctx.egui_wants_keyboard_input();
    }
    let _ = (me_key, PLAYER_HEIGHT);
    for act in acts {
        match &act {
            Act::Craft { .. } => {
                sfx.write(Sfx::ui("ui_click"));
            }
            Act::Inv(_) => {
                sfx.write(Sfx::ui("inv_move"));
            }
            _ => {}
        }
        session.act(act);
    }
    Ok(())
}
