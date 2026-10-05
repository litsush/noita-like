//! The client's own configuration (volumes, UI scale, key bindings, player
//! name and identity), stored as JSON in the platform data directory, and
//! the settings page shared by the main menu and the pause menu.

use std::path::PathBuf;

use bevy::prelude::*;
use bevy_egui::egui;
use serde::{Deserialize, Serialize};

use crate::controls::{Action, Bindings, Rebind, SLOTS};
use crate::ui::{self, ACCENT, DANGER, TEXT_DIM};

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(default)]
pub struct Settings {
    pub master_volume: f32,
    pub music_volume: f32,
    pub sfx_volume: f32,
    pub screen_shake: bool,
    /// UI zoom (egui zoom factor).
    pub ui_scale: f32,
    /// Show name tags above teammates.
    pub name_tags: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            master_volume: 0.8,
            music_volume: 0.6,
            sfx_volume: 0.8,
            screen_shake: true,
            ui_scale: 1.0,
            name_tags: true,
        }
    }
}

#[derive(Resource, Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct Config {
    pub settings: Settings,
    pub bindings: Bindings,
    pub name: String,
    /// This player's persistent identity on LAN hosts (Steam uses the Steam
    /// id). Generated on first run.
    pub key: u64,
    pub last_address: String,
    /// Don't write to disk (set by dev flags that override the identity).
    #[serde(skip)]
    pub ephemeral: bool,
}

/// Where the config and saved worlds live. `SBCT_SAVE_DIR` overrides it
/// (handy for testing).
pub fn data_dir() -> PathBuf {
    if let Ok(dir) = std::env::var("SBCT_SAVE_DIR") {
        return PathBuf::from(dir);
    }
    let env = |k: &str| std::env::var_os(k).map(PathBuf::from);
    let base = if cfg!(windows) {
        env("APPDATA")
    } else if cfg!(target_os = "macos") {
        env("HOME").map(|h| h.join("Library/Application Support"))
    } else {
        env("XDG_DATA_HOME").or_else(|| env("HOME").map(|h| h.join(".local/share")))
    };
    base.unwrap_or_else(|| PathBuf::from("."))
        .join("sbct-terraforming")
}

impl Config {
    pub fn load() -> Config {
        let path = data_dir().join("config.json");
        let mut config = match std::fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str(&text).unwrap_or_else(|e| {
                warn!(
                    "config {} is corrupt ({e}); backing it up and starting fresh",
                    path.display()
                );
                let _ = std::fs::rename(&path, path.with_extension("json.bak"));
                Config::default()
            }),
            Err(_) => Config::default(),
        };
        if config.key == 0 {
            use std::time::{SystemTime, UNIX_EPOCH};
            let nanos = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(1, |d| d.as_nanos() as u64);
            config.key = sbct_sim::rng::hash2(nanos, std::process::id() as i32, 7) | 1;
            config.store();
        }
        if config.name.trim().is_empty() {
            config.name = "Terraformer".into();
        }
        config
    }

    /// Writes atomically (temp file + rename) so a crash mid-write can't corrupt it.
    pub fn store(&self) {
        if self.ephemeral {
            return;
        }
        let dir = data_dir();
        let result = (|| -> std::io::Result<()> {
            std::fs::create_dir_all(&dir)?;
            let tmp = dir.join("config.json.tmp");
            std::fs::write(
                &tmp,
                serde_json::to_string_pretty(self).expect("config serializes"),
            )?;
            std::fs::rename(tmp, dir.join("config.json"))
        })();
        if let Err(e) = result {
            warn!("couldn't write config in {}: {e}", dir.display());
        }
    }
}

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum SettingsTab {
    #[default]
    General,
    Controls,
}

pub fn settings_ui(ui: &mut egui::Ui, config: &mut Config, rebind: &mut Rebind, tab: &mut SettingsTab) {
    ui.horizontal(|ui| {
        ui.selectable_value(tab, SettingsTab::General, "Audio & display");
        ui.selectable_value(tab, SettingsTab::Controls, "Controls");
    });
    ui.add_space(8.0);
    match tab {
        SettingsTab::General => general(ui, config),
        SettingsTab::Controls => controls(ui, config, rebind),
    }
}

fn general(ui: &mut egui::Ui, config: &mut Config) {
    let s = &mut config.settings;
    let mut changed = false;
    egui::Grid::new("settings")
        .num_columns(2)
        .spacing([16.0, 12.0])
        .show(ui, |ui| {
            for (label, value) in [
                ("Master volume", &mut s.master_volume),
                ("Music", &mut s.music_volume),
                ("Effects and ambience", &mut s.sfx_volume),
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
            ui.label("Name tags");
            changed |= ui.checkbox(&mut s.name_tags, "").changed();
            ui.end_row();
        });
    if changed {
        config.store();
    }
}

fn controls(ui: &mut egui::Ui, config: &mut Config, rebind: &mut Rebind) {
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
                        config.bindings.swap(target, other, binding);
                        config.store();
                        rebind.conflict = None;
                    }
                    if ui.button("Use for both").clicked() {
                        config.bindings.set(target.0, target.1, Some(binding));
                        config.store();
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
        .max_height((ui.available_height() - 60.0).max(120.0))
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
                        let slots = config.bindings.slots(action);
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
                            config.bindings.set(action, 1, None);
                            config.store();
                        }
                        ui.end_row();
                    }
                });
            ui.label(
                egui::RichText::new(
                    "The mouse wheel also changes the hotbar slot. In the inventory: drag to move, right-click to split, Shift-click to send across, Alt-click to favourite.",
                )
                .color(TEXT_DIM),
            );
        });
    ui.add_space(6.0);
    if ui.button("Reset to defaults").clicked() {
        config.bindings = Bindings::default();
        rebind.waiting = None;
        rebind.conflict = None;
        config.store();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_roundtrips_and_tolerates_old_files() {
        let mut c = Config {
            name: "Ada".into(),
            key: 42,
            ..Config::default()
        };
        c.settings.ui_scale = 1.5;
        c.bindings.set(Action::Jump, 1, None);
        let json = serde_json::to_string(&c).unwrap();
        let back: Config = serde_json::from_str(&json).unwrap();
        assert_eq!(back, c);
        // Missing and unknown fields are fine.
        let old: Config =
            serde_json::from_str(r#"{"name":"Bo","future":1,"settings":{"music_volume":0.1}}"#).unwrap();
        assert_eq!(old.name, "Bo");
        assert_eq!(old.settings.music_volume, 0.1);
        assert_eq!(old.settings.sfx_volume, Settings::default().sfx_volume);
        assert_eq!(old.bindings, Bindings::default());
    }
}
