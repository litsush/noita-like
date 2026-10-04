//! The settings page, shared by the main menu and the in-run pause menu:
//! audio and display, and the Controls page for rebinding.

use bevy_egui::egui;

use crate::controls::{Action, Bindings, Rebind, SLOTS};
use crate::run::save::{DigMode, SaveData, store};
use crate::ui::{self, ACCENT, DANGER, TEXT_DIM};

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    #[default]
    General,
    Controls,
}

pub fn settings_ui(ui: &mut egui::Ui, save: &mut SaveData, rebind: &mut Rebind, tab: &mut SettingsTab) {
    ui.horizontal(|ui| {
        ui.selectable_value(tab, SettingsTab::General, "Audio & display");
        ui.selectable_value(tab, SettingsTab::Controls, "Controls");
    });
    ui.add_space(8.0);
    match tab {
        SettingsTab::General => general(ui, save),
        SettingsTab::Controls => controls(ui, save, rebind),
    }
}

fn general(ui: &mut egui::Ui, save: &mut SaveData) {
    let s = &mut save.settings;
    let mut changed = false;
    egui::Grid::new("settings")
        .num_columns(2)
        .spacing([16.0, 12.0])
        .show(ui, |ui| {
            for (label, value) in [
                ("Master volume", &mut s.master_volume),
                ("Music", &mut s.music_volume),
                ("Effects", &mut s.sfx_volume),
            ] {
                ui.label(label);
                changed |= ui
                    .add(egui::Slider::new(value, 0.0..=1.0).show_value(false))
                    .changed();
                ui.end_row();
            }
            ui.label("UI scale");
            changed |= ui
                .add(
                    egui::Slider::new(&mut s.ui_scale, 0.75..=2.0)
                        .step_by(0.25)
                        .suffix("x"),
                )
                .changed();
            ui.end_row();
            ui.label("Screen shake");
            changed |= ui.checkbox(&mut s.screen_shake, "").changed();
            ui.end_row();
            ui.label("Dig mode");
            ui.horizontal(|ui| {
                changed |= ui
                    .selectable_value(&mut s.dig_mode, DigMode::Cursor, "Cursor")
                    .changed();
                changed |= ui
                    .selectable_value(&mut s.dig_mode, DigMode::Smart, "Smart")
                    .changed();
            });
            ui.end_row();
        });
    if changed {
        store(save);
    }
}

fn controls(ui: &mut egui::Ui, save: &mut SaveData, rebind: &mut Rebind) {
    if let Some((binding, target, other)) = rebind.conflict {
        ui::panel_frame()
            .stroke(egui::Stroke::new(2.0, DANGER))
            .show(ui, |ui| {
                ui.label(format!(
                    "{} is already bound to {}.",
                    binding.label(),
                    other.0.label()
                ));
                ui.horizontal(|ui| {
                    if ui.button("Swap").clicked() {
                        save.bindings.swap(target, other, binding);
                        store(save);
                        rebind.conflict = None;
                    }
                    if ui.button("Use for both").clicked() {
                        save.bindings.set(target.0, target.1, Some(binding));
                        store(save);
                        rebind.conflict = None;
                    }
                    if ui.button("Cancel").clicked() {
                        rebind.conflict = None;
                    }
                });
            });
        ui.add_space(6.0);
    }

    egui::ScrollArea::vertical()
        .max_height(ui.available_height() - 60.0)
        .show(ui, |ui| {
            let mut group = "";
            egui::Grid::new("bindings")
                .num_columns(1 + SLOTS + 1)
                .spacing([12.0, 6.0])
                .show(ui, |ui| {
                    for action in Action::ALL {
                        if action.group() != group {
                            group = action.group();
                            ui.label(egui::RichText::new(group).color(ACCENT));
                            ui.end_row();
                        }
                        ui.label(action.label());
                        let slots = save.bindings.slots(action);
                        for (slot, binding) in slots.iter().enumerate() {
                            let waiting = rebind.waiting == Some((action, slot));
                            let text = if waiting {
                                "press a key...".to_string()
                            } else {
                                binding.map_or("-".to_string(), |b| b.label())
                            };
                            let button = egui::Button::new(egui::RichText::new(text).color(if waiting {
                                ACCENT
                            } else {
                                ui::TEXT
                            }))
                            .min_size(egui::vec2(140.0, 26.0));
                            if ui
                                .add(button)
                                .on_hover_text("Click, then press a key or mouse button. Esc cancels.")
                                .clicked()
                            {
                                rebind.start(action, slot);
                            }
                        }
                        if slots[1].is_some()
                            && ui
                                .small_button("x")
                                .on_hover_text("Clear the alternate")
                                .clicked()
                        {
                            save.bindings.set(action, 1, None);
                            store(save);
                        }
                        ui.end_row();
                    }
                });
            ui.label(egui::RichText::new("The mouse wheel also switches items.").color(TEXT_DIM));
        });
    ui.add_space(6.0);
    if ui.button("Reset to defaults").clicked() {
        save.bindings = Bindings::default();
        rebind.waiting = None;
        rebind.conflict = None;
        store(save);
    }
}
