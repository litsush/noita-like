//! Windows over the game: the inventory and containers, the Synthesizer,
//! the Comm. Terminal and the other things players open by interacting.
//! Also finds what the player can interact with and shows the prompt.

mod dome;
mod inventory;
pub mod lab;
mod modbay;
pub mod robot;
mod synth;
mod terminal;

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use sbct_sim::colony::actions::{Action as Act, Container};
use sbct_sim::colony::domes::{Fixture, fixture_pos};
use sbct_sim::colony::farming::PlanterOp;
use sbct_sim::colony::geom::V2;
use sbct_sim::colony::items::MachineKind;
use sbct_sim::colony::mods::Slot;
use sbct_sim::colony::plants::SpeciesId;
use sbct_sim::colony::{Colony, CrateKind, EntKind, Id, PLAYER_HEIGHT, Player};

use crate::audio::Sfx;
use crate::controls::{Action, Rebind};
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
    /// A robot and its program.
    Robot(Id),
    Splicer(Id),
    ModBay,
    Map,
    Codex,
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
#[derive(Resource)]
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
    /// The robot program being edited.
    pub robot_draft: Option<robot::Draft>,
    /// The two parents chosen in the Gene Splicer.
    pub splice: (Option<SpeciesId>, Option<SpeciesId>),
    pub codex_tab: lab::CodexTab,
    pub codex_plant: Option<SpeciesId>,
    pub mod_slot: Slot,
}

impl Default for Panels {
    fn default() -> Panels {
        Panels {
            pause: false,
            settings: false,
            settings_tab: SettingsTab::default(),
            typing: false,
            open: None,
            dragging: None,
            search: String::new(),
            synth_filter: String::new(),
            terminal_tab: TerminalTab::default(),
            rename: None,
            robot_draft: None,
            splice: (None, None),
            codex_tab: lab::CodexTab::default(),
            codex_plant: None,
            mod_slot: Slot::Feet,
        }
    }
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
    Robot(Id),
    /// A teammate Healing Hands can help.
    Teammate(sbct_sim::colony::PlayerKey),
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
                    Fixture::SuitRack => "Suit rack: your suit seals by itself outside".into(),
                    Fixture::Synthesizer => "Synthesizer".into(),
                    Fixture::Terminal => "Comm. Terminal".into(),
                    Fixture::ModBay => "Mod Bay".into(),
                    Fixture::Planter(i) => match planter_action(colony, me, dome, i) {
                        Some((_, label)) => label,
                        None if can_tickle(colony, me, dome, i) => {
                            format!("Tickle: {}", planter_status(colony, dome, i))
                        }
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
            Target::Teammate(key) => match colony.players.get(&key) {
                Some(p) if p.dead.is_some() => format!("Revive {}", p.name),
                Some(p) => format!("Heal {}", p.name),
                None => String::new(),
            },
            Target::Robot(id) => match colony.ents.get(&id).map(|e| &e.kind) {
                Some(EntKind::Robot(r)) => format!("{}: program", r.name),
                _ => String::new(),
            },
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
                } else if me.stats().tickle > 0 {
                    format!("Tickle {name} ({:.0}% grown)", p.growth * 100.0)
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

/// Whether Green Fingers would do something to a planter.
fn can_tickle(colony: &Colony, me: &Player, dome: Id, slot: u8) -> bool {
    me.stats().tickle > 0
        && colony
            .dome(dome)
            .and_then(|(_, d)| d.planters.get(slot as usize))
            .is_some_and(|p| p.species.is_some() && p.growth < 1.0)
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
            EntKind::Robot(_) => out.push((Target::Robot(e.id), e.pos)),
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
    // Healing Hands: hurt or downed teammates, further away at higher tiers.
    let hands = me.stats().heal_touch;
    if hands > 0 {
        let range = if hands >= 3 { 180.0 } else { reach };
        for p in colony.online() {
            let needs = p.dead.is_some() && hands >= 2 || p.alive() && p.hp < p.stats().max_hp - 1.0;
            if p.key != me.key && needs && p.center().distance(at) <= range {
                out.push((Target::Teammate(p.key), p.center()));
            }
        }
    }
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
            Open::Robot(id) => me.stats().remote_robots || crate_near(colony, me, Container::Machine(id)),
            Open::Splicer(id) => crate_near(colony, me, Container::Machine(id)),
            Open::ModBay => colony.synthesizer_near(me.key) || me.in_dome == Some(colony.home),
            Open::Map | Open::Codex => true,
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
    for (action, view) in [(Action::Map, Open::Map), (Action::Codex, Open::Codex)] {
        if b.just_pressed(action, &keys, &mouse) {
            if panels.open == Some(view) {
                panels.close();
                sfx.write(Sfx::ui("inv_close"));
            } else {
                panels.open = Some(view);
                sfx.write(Sfx::ui("inv_open"));
            }
            return;
        }
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
        Target::Fixture(_, Fixture::SuitRack) => None,
        Target::Fixture(_, Fixture::Bunk(_)) => {
            act = Some(Act::Sleep);
            None
        }
        Target::Fixture(_, Fixture::ModBay) => Some(Open::ModBay),
        Target::Fixture(dome, Fixture::Planter(i)) => match planter_action(colony, me, dome, i) {
            Some((op, _)) => {
                act = Some(Act::Planter { dome, slot: i, op });
                None
            }
            None if can_tickle(colony, me, dome, i) => {
                act = Some(Act::Tickle {
                    planter: Some((dome, i)),
                    plant: None,
                });
                None
            }
            None => Some(Open::Dome(dome)),
        },
        Target::Fixture(dome, _) => Some(Open::Dome(dome)),
        Target::Machine(id, MachineKind::Chest | MachineKind::Pylon) => {
            Some(Open::Container(Container::Machine(id)))
        }
        Target::Machine(_, MachineKind::Synthesizer) => Some(Open::Synth),
        Target::Machine(id, MachineKind::Splicer) => Some(Open::Splicer(id)),
        Target::Machine(id, _) => Some(Open::Machine(id)),
        Target::Crate(id, _) => Some(Open::Container(Container::Crate(id))),
        Target::Robot(id) => Some(Open::Robot(id)),
        Target::Teammate(key) => {
            act = Some(Act::Heal { target: key });
            None
        }
        Target::Plant(id) => {
            let ripe =
                matches!(colony.ents.get(&id).map(|e| &e.kind), Some(EntKind::Plant(p)) if p.growth >= 1.0);
            act = Some(if !ripe && me.stats().tickle > 0 {
                Act::Tickle {
                    planter: None,
                    plant: Some(id),
                }
            } else {
                Act::Harvest { plant: id }
            });
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

/// What some mods draw over the world: creature health (Threat Lens),
/// plant genes under the cursor (Botanist's Bonnet) and where the held
/// dome kit fits (Geo Visor).
fn mod_overlays(
    ctx: &egui::Context,
    colony: &Colony,
    me: &Player,
    world: Option<&sbct_sim::World>,
    cursor: V2,
    to_screen: &dyn Fn(V2) -> Option<egui::Pos2>,
) {
    let stats = me.stats();
    let painter = ctx.layer_painter(egui::LayerId::background());
    if stats.see_health {
        for e in colony.ents.values() {
            let EntKind::Creature(c) = &e.kind else { continue };
            if e.pos.distance(me.center()) > 260.0 || c.hp <= 0.0 {
                continue;
            }
            let top = V2 {
                x: e.pos.x,
                y: e.pos.y - c.kind.size().1 - 5.0,
            };
            let Some(at) = to_screen(top) else { continue };
            let bar = egui::Rect::from_center_size(at, egui::vec2(28.0, 4.0));
            painter.rect_filled(bar, 0.0, egui::Color32::from_black_alpha(170));
            let mut fill = bar;
            fill.set_width(bar.width() * (c.hp / c.kind.max_hp()).clamp(0.0, 1.0));
            painter.rect_filled(fill, 0.0, if c.kind.predator() { ui::DANGER } else { ui::GOOD });
        }
    }
    if stats.see_genes {
        // The plant or planter under the cursor.
        let mut species = colony
            .ents
            .values()
            .filter_map(|e| match e.kind {
                EntKind::Plant(p) if e.pos.distance(cursor) < 12.0 => Some((p.species, p.growth)),
                _ => None,
            })
            .next();
        if species.is_none() {
            species = colony.domes().find_map(|(e, d)| {
                d.kind.fixtures().iter().find_map(|f| match f.fixture {
                    Fixture::Planter(i)
                        if fixture_pos(e.pos, f).distance(V2 {
                            x: cursor.x,
                            y: cursor.y + 8.0,
                        }) < 12.0 =>
                    {
                        let p = d.planters.get(i as usize)?;
                        Some((p.species?, p.growth))
                    }
                    _ => None,
                })
            });
        }
        if let Some((id, growth)) = species
            && let Some(sp) = colony.species.get(id as usize)
            && let Some(at) = to_screen(V2 {
                x: cursor.x,
                y: cursor.y - 22.0,
            })
        {
            let g = sp.genes;
            let text = format!(
                "{}  {:.0}% grown\nOxygen {}  Growth {}  Yield {}  Power {}\nFood {}  Light {}  Thirst {}  Hardy {}",
                sp.name,
                growth.min(1.0) * 100.0,
                g.oxygen,
                g.growth,
                g.yield_,
                g.power,
                g.food,
                g.light,
                g.thirst,
                g.hardy
            );
            let galley = painter.layout_no_wrap(text, egui::FontId::proportional(13.0), ui::TEXT);
            let rect = egui::Rect::from_center_size(
                at - egui::vec2(0.0, 24.0),
                galley.size() + egui::vec2(12.0, 8.0),
            );
            painter.rect_filled(rect, 2.0, egui::Color32::from_black_alpha(215));
            painter.rect_stroke(
                rect,
                2.0,
                egui::Stroke::new(1.0, ui::BORDER),
                egui::StrokeKind::Inside,
            );
            painter.galley(rect.min + egui::vec2(6.0, 4.0), galley, ui::TEXT);
        }
    }
    if stats.site_survey
        && let Some(world) = world
        && let Some(sbct_sim::colony::items::Item::DomeKit(kind)) = me.held().map(|s| s.item)
    {
        let mut dx = -320.0;
        while dx <= 320.0 {
            let probe = V2 {
                x: cursor.x + dx,
                y: cursor.y,
            };
            if let Ok(site) = colony.dome_site(world, kind, probe)
                && let Some(at) = to_screen(site)
            {
                painter.line_segment(
                    [at + egui::vec2(-5.0, 0.0), at + egui::vec2(5.0, 0.0)],
                    egui::Stroke::new(3.0, ui::GOOD),
                );
                painter.line_segment([at, at - egui::vec2(0.0, 7.0)], egui::Stroke::new(2.0, ui::GOOD));
            }
            dx += 16.0;
        }
    }
}

#[allow(clippy::too_many_arguments)]
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
    local: Res<LocalPlayer>,
    mut map: ResMut<crate::map::MapView>,
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

    // What each robot is up to.
    if !panels.pause {
        let to_screen = |p: V2| {
            cam.world_to_viewport(cam_tf, Vec3::new(p.x, -p.y, 0.0))
                .ok()
                .map(|s| egui::pos2(s.x / zoom, s.y / zoom))
        };
        robot::bubbles(ctx, colony, local.cursor, me.center(), &to_screen);
        crate::map::ping_markers(ctx, colony, &to_screen);
        mod_overlays(ctx, colony, me, session.world.as_ref(), local.cursor, &to_screen);
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
        Some(Open::Robot(id)) => {
            robot::window(ctx, &icons, colony, id, &mut panels, &mut acts, &mut close);
        }
        Some(Open::Splicer(id)) => {
            lab::splicer_window(ctx, &icons, colony, me, id, &mut panels, &mut acts, &mut close);
        }
        Some(Open::ModBay) => {
            modbay::window(ctx, &icons, colony, me, &mut panels, &mut acts, &mut close);
        }
        Some(Open::Codex) => {
            lab::codex_window(ctx, &icons, colony, me, &mut panels, &mut close);
        }
        Some(Open::Map) => {
            crate::map::window(ctx, &mut map, colony, me, &mut panels, &mut acts, &mut close);
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
