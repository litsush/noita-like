//! The in-game overlay: vitals, clock and atmosphere, hotbar, the roster,
//! name tags, toasts, the blackout screen and the pause menu.

use std::collections::VecDeque;

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use sbct_net::steam::SteamEvent;
use sbct_sim::colony::items::Item;
use sbct_sim::colony::{Colony, HOTBAR, PLAYER_HEIGHT, WeatherKind};

use crate::controls::{Action, Rebind};
use crate::panels::Panels;
use crate::render::{UiHasPointer, WorldCamera};
use crate::session::{EndSession, PendingJoin, Role, Session};
use crate::settings::{Config, settings_ui};
use crate::steam::{SteamClient, SteamInbox};
use crate::ui::{self, ACCENT, DANGER, GOOD, TEXT, TEXT_DIM, UiIcons, WARN};

pub struct Toast {
    pub text: String,
    pub good: bool,
    pub age: f32,
}

#[derive(Resource, Default)]
pub struct Toasts(pub VecDeque<Toast>);

impl Toasts {
    pub fn push(&mut self, text: impl Into<String>, good: bool) {
        let text = text.into();
        // The same message again just refreshes it.
        if let Some(t) = self.0.iter_mut().find(|t| t.text == text) {
            t.age = t.age.min(0.3);
            return;
        }
        self.0.push_back(Toast { text, good, age: 0.0 });
        while self.0.len() > 5 {
            self.0.pop_front();
        }
    }
}

const TOAST_SECS: f32 = 4.5;

pub fn toggle_pause(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    config: Res<Config>,
    rebind: Res<Rebind>,
    mut panels: ResMut<Panels>,
) {
    if rebind.blocking() {
        return;
    }
    if config.bindings.just_pressed(Action::Pause, &keys, &mouse) {
        if panels.settings {
            panels.settings = false;
        } else if panels.open.is_some() {
            panels.close();
        } else {
            panels.pause = !panels.pause;
        }
    }
}

/// Accepting a Steam invite while in a game leaves it and joins the new lobby.
pub fn ingame_steam_events(
    mut commands: Commands,
    mut inbox: ResMut<SteamInbox>,
    steam: Option<Res<SteamClient>>,
) {
    if steam.is_none() {
        return;
    }
    for event in std::mem::take(&mut inbox.0) {
        if let SteamEvent::JoinRequested(lobby) = event {
            commands.insert_resource(EndSession(None));
            commands.insert_resource(PendingJoin(lobby));
        }
    }
}

/// Paints an item's icon; seeds and crops are tinted by their species.
pub fn paint_item(painter: &egui::Painter, icons: &UiIcons, colony: &Colony, item: Item, rect: egui::Rect) {
    let tint = match item {
        Item::Seed(s) | Item::Crop(s) if item.icon() != 92 => {
            colony.species.get(s as usize).map_or(egui::Color32::WHITE, |sp| {
                ui::hue_color(sp.looks.hue_bloom, 0.55, 0.95)
            })
        }
        _ => egui::Color32::WHITE,
    };
    ui::paint_icon(painter, icons, item.icon(), rect, tint);
}

/// Draws a slot frame with an optional stack in it. Returns the response.
pub fn slot(
    ui: &mut egui::Ui,
    icons: &UiIcons,
    colony: &Colony,
    stack: Option<sbct_sim::colony::inventory::Stack>,
    size: f32,
    selected: bool,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::click_and_drag());
    let p = ui.painter();
    p.rect_filled(rect, 0.0, egui::Color32::from_rgb(8, 14, 16));
    let border = if selected {
        ACCENT
    } else if response.hovered() {
        ui::TEXT_DIM
    } else {
        ui::BORDER
    };
    p.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(if selected { 2.0 } else { 1.0 }, border),
        egui::StrokeKind::Inside,
    );
    if let Some(s) = stack {
        paint_item(p, icons, colony, s.item, rect.shrink(size * 0.12));
        if s.count > 1 {
            let text = if s.count >= 1000 {
                format!("{}k", s.count / 1000)
            } else {
                s.count.to_string()
            };
            let pos = rect.right_bottom() - egui::vec2(2.0, 0.0);
            p.text(
                pos + egui::vec2(1.0, 1.0),
                egui::Align2::RIGHT_BOTTOM,
                &text,
                egui::FontId::proportional(16.0),
                egui::Color32::BLACK,
            );
            p.text(
                pos,
                egui::Align2::RIGHT_BOTTOM,
                &text,
                egui::FontId::proportional(16.0),
                TEXT,
            );
        }
        if s.fav {
            ui::paint_icon(
                p,
                icons,
                60,
                egui::Rect::from_min_size(rect.min, egui::vec2(10.0, 10.0)),
                egui::Color32::WHITE,
            );
        }
    }
    response
}

fn weather_icon(kind: WeatherKind) -> usize {
    match kind {
        WeatherKind::Clear => 80,
        WeatherKind::Mist => 81,
        WeatherKind::Rain => 82,
    }
}

fn vitals_row(ui: &mut egui::Ui, icons: &UiIcons, icon: usize, value: f32, max: f32, color: egui::Color32) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        ui::icon(ui, icons, icon, 16.0);
        ui::meter(ui, value / max.max(1.0), color, 150.0);
        ui.label(egui::RichText::new(format!("{:.0}", value.max(0.0))).color(TEXT_DIM));
    });
}

pub fn hud_ui(
    mut contexts: EguiContexts,
    mut commands: Commands,
    mut session: ResMut<Session>,
    mut panels: ResMut<Panels>,
    mut toasts: ResMut<Toasts>,
    mut pointer: ResMut<UiHasPointer>,
    mut config: ResMut<Config>,
    mut rebind: ResMut<Rebind>,
    icons: Option<Res<UiIcons>>,
    steam: Option<Res<SteamClient>>,
    placement: Res<crate::placement::Placement>,
    camera: Single<(&Camera, &GlobalTransform), With<WorldCamera>>,
    time: Res<Time<Real>>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let Some(icons) = icons else { return Ok(()) };
    let (cam, cam_tf) = *camera;
    let dt = time.delta_secs();
    let s = &*session;
    let (Some(colony), Some(me)) = (&s.colony, s.player()) else {
        return Ok(());
    };
    let stats = me.stats();
    let zoom = ctx.zoom_factor();

    // Name tags above other players.
    if config.settings.name_tags {
        let painter = ctx.layer_painter(egui::LayerId::background());
        for p in colony.online() {
            if p.key == s.me {
                continue;
            }
            let head = Vec3::new(p.pose.pos.x, -(p.pose.pos.y - PLAYER_HEIGHT - 9.0), 0.0);
            if let Ok(pos) = cam.world_to_viewport(cam_tf, head) {
                let at = egui::pos2(pos.x / zoom, pos.y / zoom);
                let color = ui::team_color(p.color);
                painter.text(
                    at + egui::vec2(1.0, 1.0),
                    egui::Align2::CENTER_BOTTOM,
                    &p.name,
                    egui::FontId::proportional(16.0),
                    egui::Color32::BLACK,
                );
                painter.text(
                    at,
                    egui::Align2::CENTER_BOTTOM,
                    &p.name,
                    egui::FontId::proportional(16.0),
                    color,
                );
                // A thin health bar under the name.
                let bar = egui::Rect::from_center_size(at + egui::vec2(0.0, 3.0), egui::vec2(30.0, 3.0));
                painter.rect_filled(bar, 0.0, egui::Color32::from_black_alpha(160));
                let mut fill = bar;
                fill.set_width(bar.width() * (p.hp / 100.0).clamp(0.0, 1.0));
                painter.rect_filled(fill, 0.0, if p.alive() { GOOD } else { DANGER });
            }
        }
    }

    // Vitals.
    egui::Area::new("vitals".into())
        .anchor(egui::Align2::LEFT_TOP, [10.0, 10.0])
        .show(ctx, |ui| {
            ui::ornate(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 4.0;
                vitals_row(ui, &icons, 53, me.hp, stats.max_hp, DANGER);
                let o2_color = if me.o2 < stats.o2_capacity * 0.25 {
                    WARN
                } else {
                    ui::WATER
                };
                vitals_row(ui, &icons, 51, me.o2, stats.o2_capacity, o2_color);
                let battery_color = if me.lockout { DANGER } else { ui::POWER };
                vitals_row(ui, &icons, 52, me.battery, stats.battery_capacity, battery_color);
                if me.warmth < 99.0 {
                    vitals_row(
                        ui,
                        &icons,
                        89,
                        me.warmth,
                        100.0,
                        egui::Color32::from_rgb(170, 210, 240),
                    );
                }
                let status = if me.in_dome.is_some() {
                    ("Breathable air", GOOD)
                } else if me.suit {
                    ("Suit sealed", TEXT_DIM)
                } else {
                    ("Open air", GOOD)
                };
                ui.label(egui::RichText::new(status.0).color(status.1));
                if me.lockout {
                    ui.label(egui::RichText::new("Laser drained: recharging").color(DANGER));
                }
                if me.slowed > 0.0 {
                    ui.label(egui::RichText::new("Webbed").color(WARN));
                }
            });
        });

    // Clock, weather and atmosphere.
    egui::Area::new("clock".into())
        .anchor(egui::Align2::CENTER_TOP, [0.0, 10.0])
        .show(ctx, |ui| {
            ui::ornate(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    let hour = colony.clock.hour();
                    ui::icon(ui, &icons, if colony.clock.is_night() { 84 } else { 83 }, 16.0);
                    ui.label(format!(
                        "Day {}  {:02}:{:02}",
                        colony.clock.day,
                        hour as u32,
                        ((hour.fract()) * 60.0) as u32 / 10 * 10
                    ));
                    ui::icon(ui, &icons, weather_icon(colony.weather.kind), 16.0);
                    ui.add_space(8.0);
                    ui::icon(ui, &icons, 51, 16.0);
                    let rate = colony.atmosphere.rate;
                    ui.label(format!("{:.2}%", colony.atmosphere.o2)).on_hover_text(
                        "Oxygen in the atmosphere. At 16% you can breathe outside; the colonists need 19%.",
                    );
                    let (arrow, color) = if rate > 0.0005 {
                        (105, GOOD)
                    } else if rate < -0.0005 {
                        (106, DANGER)
                    } else {
                        (107, TEXT_DIM)
                    };
                    ui::icon(ui, &icons, arrow, 16.0);
                    ui.label(egui::RichText::new(format!("{rate:+.2}/day")).color(color));
                    ui.add_space(8.0);
                    ui::icon(ui, &icons, 50, 16.0);
                    ui.label(format!("{}", colony.credits));
                });
            });
        });

    // Roster.
    egui::Area::new("roster".into())
        .anchor(egui::Align2::RIGHT_TOP, [-10.0, 10.0])
        .show(ctx, |ui| {
            ui::ornate(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 2.0;
                ui.label(egui::RichText::new(s.describe()).color(TEXT_DIM));
                let mut players: Vec<_> = colony.online().collect();
                players.sort_by_key(|p| (p.key != s.me, p.name.clone()));
                for p in players {
                    ui.horizontal(|ui| {
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
                        ui.painter().rect_filled(rect, 0.0, ui::team_color(p.color));
                        let mut text = egui::RichText::new(&p.name);
                        if !p.alive() {
                            text = text.color(DANGER).strikethrough();
                        }
                        ui.label(text);
                    });
                }
            });
        });

    // Hotbar.
    let mut select = None;
    egui::Area::new("hotbar".into())
        .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -10.0])
        .show(ctx, |ui| {
            ui::ornate(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 3.0;
                    for i in 0..HOTBAR {
                        let r = slot(
                            ui,
                            &icons,
                            colony,
                            me.inv.slots[i],
                            40.0,
                            me.selected as usize == i,
                        );
                        ui.painter().text(
                            r.rect.left_top() + egui::vec2(3.0, 1.0),
                            egui::Align2::LEFT_TOP,
                            ((i + 1) % 10).to_string(),
                            egui::FontId::proportional(8.0),
                            TEXT_DIM,
                        );
                        if r.clicked() {
                            select = Some(i);
                        }
                        if let Some(stack) = me.inv.slots[i] {
                            r.on_hover_text(colony.item_name(stack.item));
                        }
                    }
                    ui.add_space(6.0);
                    // The bare multitool.
                    let (rect, r) = ui.allocate_exact_size(egui::vec2(40.0, 40.0), egui::Sense::click());
                    let on = me.selected as usize >= HOTBAR;
                    ui.painter()
                        .rect_filled(rect, 0.0, egui::Color32::from_rgb(8, 14, 16));
                    ui.painter().rect_stroke(
                        rect,
                        0.0,
                        egui::Stroke::new(if on { 2.0 } else { 1.0 }, if on { ACCENT } else { ui::BORDER }),
                        egui::StrokeKind::Inside,
                    );
                    ui::paint_icon(ui.painter(), &icons, 48, rect.shrink(5.0), egui::Color32::WHITE);
                    if r.clicked() {
                        select = Some(HOTBAR);
                    }
                    r.on_hover_text(format!(
                        "Multitool: digs. {} puts your item away.",
                        config.bindings.hint(Action::Multitool)
                    ));
                });
                if let Some(why) = placement.error {
                    ui.label(egui::RichText::new(format!("Can't place here: {why}")).color(DANGER));
                } else if placement.active {
                    ui.label(egui::RichText::new("Click to place").color(GOOD));
                }
                let held = me.held().map(|st| colony.item_name(st.item));
                let hint = match held {
                    Some(name) => format!("{name}  ·  {} to use", config.bindings.hint(Action::Primary)),
                    None => format!(
                        "{} dig  ·  {} laser",
                        config.bindings.hint(Action::Primary),
                        config.bindings.hint(Action::Fire)
                    ),
                };
                ui.label(egui::RichText::new(hint).color(TEXT_DIM));
            });
        });

    // Toasts, newest at the bottom.
    egui::Area::new("toasts".into())
        .anchor(egui::Align2::RIGHT_BOTTOM, [-10.0, -10.0])
        .interactable(false)
        .show(ctx, |ui| {
            ui.spacing_mut().item_spacing.y = 4.0;
            for t in toasts.0.iter() {
                let fade = ((TOAST_SECS - t.age) / 0.6).clamp(0.0, 1.0) * (t.age / 0.15).clamp(0.0, 1.0);
                let color = if t.good { ACCENT } else { WARN };
                ui.scope(|ui| {
                    ui.set_opacity(fade);
                    ui::panel_frame()
                        .stroke(egui::Stroke::new(1.0, color))
                        .show(ui, |ui| {
                            ui.set_max_width(320.0);
                            ui.label(egui::RichText::new(&t.text).color(TEXT));
                        });
                });
            }
        });
    for t in toasts.0.iter_mut() {
        t.age += dt;
    }
    toasts.0.retain(|t| t.age < TOAST_SECS);

    // Blackout.
    if let Some(left) = me.dead {
        let painter = ctx.layer_painter(egui::LayerId::new(egui::Order::Middle, "blackout".into()));
        painter.rect_filled(ctx.viewport_rect(), 0.0, egui::Color32::from_black_alpha(200));
        painter.text(
            ctx.viewport_rect().center(),
            egui::Align2::CENTER_CENTER,
            format!(
                "You blacked out. Waking in your bed in {:.0}…",
                left.max(0.0).ceil()
            ),
            ui::title_font(32.0),
            TEXT,
        );
    }

    // Pause menu.
    let mut act = None;
    if panels.pause {
        egui::Area::new("pause".into())
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui::ornate(ui, |ui| {
                    if panels.settings {
                        ui.set_width(560.0);
                        ui.set_height(430.0);
                        settings_ui(ui, &mut config, &mut rebind, &mut panels.settings_tab);
                        ui.add_space(6.0);
                        if ui.button("Back").clicked() {
                            panels.settings = false;
                        }
                        return;
                    }
                    ui.vertical_centered(|ui| {
                        ui.label(ui::title("Paused", 32.0).color(ACCENT));
                        ui.label(egui::RichText::new("The colony keeps running.").color(TEXT_DIM));
                        ui.add_space(6.0);
                        if ui::menu_button(ui, "Resume").clicked() {
                            panels.pause = false;
                        }
                        if ui::menu_button(ui, "Settings").clicked() {
                            panels.settings = true;
                        }
                        if let (Some(steam), Some(lobby)) = (&steam, s.lobby) {
                            if ui::menu_button(ui, "Invite friends").clicked() {
                                steam.open_invite_dialog(lobby);
                            }
                            if ui::menu_button(ui, "Copy lobby ID").clicked() {
                                ui.ctx().copy_text(lobby.to_string());
                            }
                        }
                        if s.is_authority() && ui::menu_button(ui, "Save now").clicked() {
                            act = Some(PauseAction::Save);
                        }
                        let leave = match s.role {
                            Role::Client { .. } => "Disconnect",
                            _ => "Save and quit to menu",
                        };
                        if ui::menu_button(ui, leave).clicked() {
                            act = Some(PauseAction::Leave);
                        }
                    });
                });
            });
    }

    panels.typing = ctx.egui_wants_keyboard_input();
    pointer.0 = ctx.is_pointer_over_egui() || panels.pause;

    if let Some(slot) = select {
        session.act(sbct_sim::colony::actions::Action::Inv(
            sbct_sim::colony::actions::InvOp::Select(slot as u8),
        ));
    }
    match act {
        Some(PauseAction::Save) => match session.save() {
            Ok(()) => toasts.push("World saved", true),
            Err(e) => toasts.push(e, false),
        },
        Some(PauseAction::Leave) => {
            panels.pause = false;
            commands.insert_resource(EndSession(None));
        }
        None => {}
    }
    Ok(())
}

enum PauseAction {
    Save,
    Leave,
}
