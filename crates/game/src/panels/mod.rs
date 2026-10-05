//! Windows over the game: the inventory and containers, the Synthesizer,
//! the Comm. Terminal and the other things players open by interacting.
//! Also finds what the player can interact with and shows the prompt.

mod inventory;
mod synth;
mod terminal;

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use sbct_sim::colony::actions::{Action as Act, Container};
use sbct_sim::colony::domes::{Fixture, fixture_pos};
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
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum TerminalTab {
    #[default]
    Sell,
    Buy,
    Blueprints,
    Workers,
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
}

/// The thing the Interact key would use right now, and where it is.
#[derive(Resource, Default)]
pub struct Focus(pub Option<(Target, V2)>);

impl Target {
    fn label(&self, colony: &Colony) -> String {
        match *self {
            Target::Fixture(dome, f) => {
                let kind = colony.dome(dome).map(|(_, d)| d.kind);
                match f {
                    Fixture::Chest(i) => kind.map_or("Chest", |k| k.chest_label(i as usize)).to_string(),
                    Fixture::Bunk(_) => "Bunk".into(),
                    Fixture::SuitRack => "Suit rack".into(),
                    Fixture::Synthesizer => "Synthesizer".into(),
                    Fixture::Terminal => "Comm. Terminal".into(),
                    Fixture::ModBay => "Mod Bay".into(),
                    Fixture::Planter(_) => "Planter".into(),
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
        }
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
    let open = match target {
        Target::Fixture(dome, Fixture::Chest(i)) => Some(Open::Container(Container::DomeChest(dome, i))),
        Target::Fixture(_, Fixture::Synthesizer) => Some(Open::Synth),
        Target::Fixture(_, Fixture::Terminal) => Some(Open::Terminal),
        Target::Fixture(_, Fixture::SuitRack) => {
            toasts.push("Your suit seals by itself when you step outside.", true);
            None
        }
        Target::Fixture(..) => {
            toasts.push("Nothing to do here yet.", false);
            None
        }
        Target::Machine(id, MachineKind::Chest | MachineKind::Pylon) => {
            Some(Open::Container(Container::Machine(id)))
        }
        Target::Machine(_, MachineKind::Synthesizer) => Some(Open::Synth),
        Target::Machine(..) => None,
        Target::Crate(id, _) => Some(Open::Container(Container::Crate(id))),
    };
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
                target.label(colony)
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
