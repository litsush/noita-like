//! In-game overlay: hotbar, player list, name tags, and the Esc menu.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use sbct_net::steam::SteamEvent;
use sbct_sim::Material;

use crate::player::{Avatar, HEIGHT, LocalPlayer};
use crate::render::{UiHasPointer, WorldCamera};
use crate::session::{EndSession, PendingJoin, Role, Session};
use crate::steam::{SteamClient, SteamInbox};

#[derive(Resource, Default)]
pub struct EscMenuOpen(pub bool);

pub fn toggle_esc_menu(keys: Res<ButtonInput<KeyCode>>, mut open: ResMut<EscMenuOpen>) {
    if keys.just_pressed(KeyCode::Escape) {
        open.0 = !open.0;
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

fn color32(m: Material) -> egui::Color32 {
    let [r, g, b] = m.props().color;
    egui::Color32::from_rgb(r, g, b)
}

pub fn hud_ui(
    mut contexts: EguiContexts,
    mut commands: Commands,
    session: Res<Session>,
    mut player: ResMut<LocalPlayer>,
    mut esc: ResMut<EscMenuOpen>,
    mut pointer: ResMut<UiHasPointer>,
    steam: Option<Res<SteamClient>>,
    camera: Single<(&Camera, &GlobalTransform), With<WorldCamera>>,
    avatars: Query<(&Avatar, &GlobalTransform)>,
    time: Res<Time>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let (cam, cam_tf) = *camera;

    // Name tags above other players.
    let painter = ctx.layer_painter(egui::LayerId::background());
    for (avatar, tf) in &avatars {
        let Some(p) = session.players.get(&avatar.0) else {
            continue;
        };
        let head = tf.translation() + Vec3::Y * (HEIGHT / 2.0 + 3.0);
        if let Ok(pos) = cam.world_to_viewport(cam_tf, head) {
            painter.text(
                egui::pos2(pos.x, pos.y),
                egui::Align2::CENTER_BOTTOM,
                &p.name,
                egui::FontId::proportional(14.0),
                egui::Color32::WHITE,
            );
        }
    }

    egui::Window::new("session")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::LEFT_TOP, [8.0, 8.0])
        .show(ctx, |ui| {
            ui.label(session.describe());
            ui.label(format!("{:.0} fps", 1.0 / time.delta_secs().max(1e-4)));
            ui.separator();
            ui.label(format!("{} (you)", session.local_name));
            let mut others: Vec<&str> = session.players.values().map(|p| p.name.as_str()).collect();
            others.sort();
            for name in others {
                ui.label(name);
            }
            if matches!(session.role, Role::Host)
                && let (Some(steam), Some(lobby)) = (&steam, session.lobby)
                && ui.button("Invite friends").clicked()
            {
                steam.open_invite_dialog(lobby);
            }
        });

    egui::Window::new("hotbar")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -8.0])
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                for (i, m) in Material::PLACEABLE.iter().enumerate() {
                    let selected = player.selected == i;
                    let [r, g, b] = m.props().color;
                    let dark = (r as u32 * 3 + g as u32 * 6 + b as u32) / 10 < 110;
                    let mut text =
                        egui::RichText::new(format!("{} {}", i + 1, m.props().name)).color(if dark {
                            egui::Color32::WHITE
                        } else {
                            egui::Color32::BLACK
                        });
                    if selected {
                        text = text.strong();
                    }
                    let stroke = if selected {
                        egui::Stroke::new(2.0, egui::Color32::WHITE)
                    } else {
                        egui::Stroke::new(1.0, egui::Color32::from_gray(20))
                    };
                    let button = egui::Button::new(text)
                        .fill(color32(*m))
                        .stroke(stroke)
                        .min_size(egui::vec2(64.0, 28.0));
                    if ui.add(button).clicked() {
                        player.selected = i;
                    }
                }
            });
            ui.weak("A/D move · W/Space jump · LMB dig · RMB place · 1-9/wheel select · Esc menu");
        });

    if esc.0 {
        egui::Window::new("Menu")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    let size = [200.0, 36.0];
                    if ui.add_sized(size, egui::Button::new("Resume")).clicked() {
                        esc.0 = false;
                    }
                    if let (Some(steam), Some(lobby)) = (&steam, session.lobby) {
                        if ui.add_sized(size, egui::Button::new("Invite friends")).clicked() {
                            steam.open_invite_dialog(lobby);
                        }
                        if ui.add_sized(size, egui::Button::new("Copy lobby ID")).clicked() {
                            ui.ctx().copy_text(lobby.to_string());
                        }
                    }
                    let leave = if session.is_authority() {
                        "Close world"
                    } else {
                        "Disconnect"
                    };
                    if ui.add_sized(size, egui::Button::new(leave)).clicked() {
                        esc.0 = false;
                        commands.insert_resource(EndSession(None));
                    }
                });
            });
    }

    pointer.0 = ctx.is_pointer_over_egui() || esc.0;
    Ok(())
}
