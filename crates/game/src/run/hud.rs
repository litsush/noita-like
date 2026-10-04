//! In-run UI: HUD, popups, toasts, layer banners, pause menu and the
//! death/victory summary.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use sbct_sim::descent::{DESCENT_HEIGHT, Layer};

use super::entities::Prompt;
use super::items::{Active, icon};
use super::player::RunPlayer;
use super::save::{DigMode, SaveData};
use super::{Phase, Run, random_seed, start_run};
use crate::AppState;
use crate::audio::Sfx;
use crate::controls::Action;
use crate::render::{UiHasPointer, WorldCamera};
use crate::ui::{self, ACCENT, BORDER, DANGER, GOOD, TEXT_DIM, UiIcons};

#[derive(Resource, Default)]
pub struct Paused(pub bool);

const TOAST_SIZE: egui::Vec2 = egui::vec2(320.0, 88.0);
const TOAST_LIFE: f32 = 5.0;

/// Pause-menu sub-state: the settings page and its tab.
#[derive(Resource, Default)]
pub struct PauseMenu {
    pub settings_open: bool,
    pub tab: crate::settings::SettingsTab,
}

pub fn toggle_pause(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut paused: ResMut<Paused>,
    mut menu: ResMut<PauseMenu>,
    run: Res<Run>,
    save: Res<SaveData>,
    rebind: Res<crate::controls::Rebind>,
) {
    if rebind.blocking() || run.phase != Phase::Playing {
        return;
    }
    if save
        .bindings
        .just_pressed(crate::controls::Action::Pause, &keys, &mouse)
    {
        if menu.settings_open {
            menu.settings_open = false;
        } else {
            paused.0 = !paused.0;
        }
    }
}

pub fn loading_ui(mut contexts: EguiContexts, time: Res<Time<Real>>) -> Result {
    let ctx = contexts.ctx_mut()?;
    let mut root = ui::screen_ui(ctx);
    egui::CentralPanel::default().show(&mut root, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() / 2.0 - 60.0);
            let dots = ".".repeat((time.elapsed_secs() * 3.0) as usize % 4);
            ui.label(
                egui::RichText::new(format!("Forging a planet{dots}"))
                    .size(32.0)
                    .color(ACCENT),
            );
            ui.add_space(8.0);
            ui.label(egui::RichText::new("Layering crust, mantle and core").color(TEXT_DIM));
        });
    });
    Ok(())
}

pub fn run_hud(
    mut contexts: EguiContexts,
    mut commands: Commands,
    mut run: ResMut<Run>,
    player: Res<RunPlayer>,
    icons: Option<Res<UiIcons>>,
    prompt: Res<Prompt>,
    mut paused: ResMut<Paused>,
    mut save: ResMut<SaveData>,
    mut pointer: ResMut<UiHasPointer>,
    camera: Single<(&Camera, &GlobalTransform), With<WorldCamera>>,
    real: Res<Time<Real>>,
    mut sfx: MessageWriter<Sfx>,
    mut shake: ResMut<crate::fx::Shake>,
    mut pause_menu: ResMut<PauseMenu>,
    mut rebind: ResMut<crate::controls::Rebind>,
) -> Result {
    let Some(icons) = icons else { return Ok(()) };
    let icons = *icons;
    let ctx = contexts.ctx_mut()?;
    let dt = real.delta_secs();
    let (cam, cam_tf) = *camera;
    shake.enabled = save.settings.screen_shake;

    // ---- top left: vitals ------------------------------------------------
    egui::Area::new("vitals".into())
        .anchor(egui::Align2::LEFT_TOP, [12.0, 12.0])
        .show(ctx, |ui| {
            ui::panel_frame().show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 2.0;
                    let hearts = (player.max_hp / 20.0) as usize;
                    for i in 0..hearts {
                        let hp = player.hp.max(0.0) - i as f32 * 20.0;
                        let index = if hp >= 15.0 {
                            icon::HEART_FULL
                        } else if hp >= 5.0 {
                            icon::HEART_HALF
                        } else {
                            icon::HEART_EMPTY
                        };
                        ui::icon(ui, &icons, index, 32.0);
                    }
                });
                ui.horizontal(|ui| {
                    ui::icon(ui, &icons, icon::THERMOMETER, 16.0);
                    let color = if player.heat > 80.0 {
                        DANGER
                    } else {
                        egui::Color32::from_rgb(240, 130, 60)
                    };
                    ui::meter(ui, player.heat / 100.0, color, 140.0);
                });
                if player.breath < 100.0 {
                    ui.horizontal(|ui| {
                        ui::icon(ui, &icons, icon::BUBBLE, 16.0);
                        let color = if player.breath < 30.0 {
                            DANGER
                        } else {
                            egui::Color32::from_rgb(110, 180, 240)
                        };
                        ui::meter(ui, player.breath / 100.0, color, 140.0);
                    });
                }
                if player.thermal > 0.0 {
                    ui.horizontal(|ui| {
                        ui::icon(ui, &icons, super::items::ItemId::ThermalSuit.def().icon, 16.0);
                        ui::meter(
                            ui,
                            player.thermal / 100.0,
                            egui::Color32::from_rgb(255, 180, 60),
                            140.0,
                        );
                    });
                }
                if player.burning > 0.0 {
                    ui.horizontal(|ui| {
                        ui::icon(ui, &icons, icon::FLAME, 16.0);
                        ui.colored_label(DANGER, "ON FIRE - find water!");
                    });
                }
                ui.separator();
                let b = &save.bindings;
                ui.horizontal(|ui| {
                    for (index, count, key) in [
                        (icon::TORCH, run.light_charges as u32, b.hint(Action::LightOrb)),
                        (icon::ORE, run.shards, String::new()),
                    ] {
                        ui::icon(ui, &icons, index, 24.0);
                        ui.label(egui::RichText::new(format!("{count}")).size(16.0));
                        if !key.is_empty() {
                            ui.label(egui::RichText::new(key).color(TEXT_DIM));
                        }
                        ui.add_space(6.0);
                    }
                });
                ui.horizontal(|ui| {
                    let smart = save.settings.dig_mode == DigMode::Smart;
                    ui::icon(
                        ui,
                        &icons,
                        if smart { icon::COMPASS } else { icon::PICKAXE },
                        16.0,
                    );
                    ui.label(
                        egui::RichText::new(if smart { "Smart dig" } else { "Cursor dig" }).color(if smart {
                            ACCENT
                        } else {
                            ui::TEXT
                        }),
                    );
                    ui.label(
                        egui::RichText::new(format!("{} to switch", b.hint(Action::ToggleDigMode)))
                            .color(TEXT_DIM),
                    );
                });
            });
        });

    // ---- bottom centre: items ----------------------------------------------
    if !run.items.is_empty() {
        let selected = run.selected_active();
        egui::Area::new("items".into())
            .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -12.0])
            .show(ctx, |ui| {
                ui::panel_frame().show(ui, |ui| {
                    ui.horizontal(|ui| {
                        for &item in &run.items {
                            let def = item.def();
                            let r = ui
                                .vertical(|ui| {
                                    ui::icon(ui, &icons, def.icon, 32.0);
                                    match def.active {
                                        Some(Active::Charges { max, recharge }) => {
                                            let (n, t) = run.charges.get(&item).copied().unwrap_or((0, 0.0));
                                            let text = if max > 1 {
                                                format!("{n}/{max}")
                                            } else if n > 0 {
                                                "ready".into()
                                            } else {
                                                "...".into()
                                            };
                                            ui.label(egui::RichText::new(text).size(16.0).color(if n > 0 {
                                                GOOD
                                            } else {
                                                TEXT_DIM
                                            }));
                                            if n < max {
                                                ui::meter(ui, t / recharge, BORDER, 32.0);
                                            }
                                        }
                                        Some(Active::Spray) => {
                                            ui::meter(
                                                ui,
                                                run.tank / 100.0,
                                                egui::Color32::from_rgb(90, 160, 240),
                                                32.0,
                                            );
                                        }
                                        None => {}
                                    }
                                })
                                .response;
                            if Some(item) == selected {
                                ui.painter().rect_stroke(
                                    r.rect.expand(3.0),
                                    0.0,
                                    egui::Stroke::new(2.0, ACCENT),
                                    egui::StrokeKind::Outside,
                                );
                            }
                            r.on_hover_ui(|ui| {
                                ui.label(egui::RichText::new(def.name).color(ACCENT));
                                ui.label(def.desc);
                            });
                        }
                    });
                    if run.actives().len() > 1 {
                        ui.label(
                            egui::RichText::new(format!(
                                "{}/{} or wheel: switch   {}: use",
                                save.bindings.hint(Action::PrevItem),
                                save.bindings.hint(Action::NextItem),
                                save.bindings.hint(Action::UseItem)
                            ))
                            .color(TEXT_DIM),
                        );
                    } else if !run.actives().is_empty() {
                        ui.label(
                            egui::RichText::new(format!("{}: use", save.bindings.hint(Action::UseItem)))
                                .color(TEXT_DIM),
                        );
                    }
                });
            });
    }

    // ---- right: depth gauge -------------------------------------------------
    let depth_bottom = egui::Area::new("depth".into())
        .anchor(egui::Align2::RIGHT_TOP, [-12.0, 12.0])
        .show(ctx, |ui| {
            ui::panel_frame().show(ui, |ui| {
                ui.label(egui::RichText::new(run.layer.name()).color(ACCENT));
                let depth = player.body.pos.y.max(0.0);
                let to_core = (run.core.y - depth).max(0.0);
                ui.label(format!("Depth {} m", depth as i32 / 4));
                ui.label(egui::RichText::new(format!("Core in {} m", to_core as i32 / 4)).color(TEXT_DIM));
                let (rect, _) = ui.allocate_exact_size(egui::vec2(150.0, 180.0), egui::Sense::hover());
                let p = ui.painter();
                let colors = [
                    egui::Color32::from_rgb(112, 82, 56),
                    egui::Color32::from_rgb(120, 60, 40),
                    egui::Color32::from_rgb(80, 50, 110),
                    egui::Color32::from_rgb(90, 90, 104),
                    egui::Color32::from_rgb(200, 70, 30),
                ];
                let bar = egui::Rect::from_min_size(rect.min, egui::vec2(14.0, rect.height()));
                let to_y = |y: f32| bar.top() + y / DESCENT_HEIGHT as f32 * bar.height();
                for (i, layer) in Layer::ALL.iter().enumerate() {
                    let r = egui::Rect::from_min_max(
                        egui::pos2(bar.left(), to_y(layer.top() as f32)),
                        egui::pos2(bar.right(), to_y(layer.bottom() as f32)),
                    );
                    let reached = *layer <= run.layer;
                    p.rect_filled(
                        r,
                        0.0,
                        if reached {
                            colors[i]
                        } else {
                            colors[i].gamma_multiply(0.35)
                        },
                    );
                    p.text(
                        egui::pos2(bar.right() + 8.0, r.center().y),
                        egui::Align2::LEFT_CENTER,
                        if reached { layer.name() } else { "???" },
                        egui::FontId::proportional(16.0),
                        if *layer == run.layer { ACCENT } else { TEXT_DIM },
                    );
                }
                p.rect_stroke(
                    bar,
                    0.0,
                    egui::Stroke::new(1.0, BORDER),
                    egui::StrokeKind::Outside,
                );
                let y = to_y(depth);
                p.line_segment(
                    [egui::pos2(bar.left() - 4.0, y), egui::pos2(bar.right() + 4.0, y)],
                    egui::Stroke::new(3.0, egui::Color32::WHITE),
                );
                ui.horizontal(|ui| {
                    ui::icon(ui, &icons, icon::HOURGLASS, 16.0);
                    ui.label(ui::clock(run.elapsed));
                });
                ui.horizontal(|ui| {
                    ui::icon(ui, &icons, icon::COMPASS, 16.0);
                    ui.label(egui::RichText::new(format!("Seed {}", run.seed)).color(TEXT_DIM));
                });
            });
        })
        .response
        .rect
        .bottom();

    // ---- prompt near the thing you can use --------------------------------
    if let Some((text, at)) = &prompt.0
        && let Ok(pos) = cam.world_to_viewport(cam_tf, Vec3::new(at.x, -at.y, 0.0))
    {
        egui::Area::new("prompt".into())
            .fixed_pos(egui::pos2(pos.x, pos.y))
            .pivot(egui::Align2::CENTER_BOTTOM)
            .show(ctx, |ui| {
                ui::panel_frame().show(ui, |ui| {
                    ui.label(egui::RichText::new(text).color(ACCENT));
                });
            });
    }

    // ---- layer banner --------------------------------------------------------
    if let Some((layer, t)) = run.banner {
        let alpha = if t < 0.5 {
            t / 0.5
        } else if t > 3.0 {
            (3.8 - t) / 0.8
        } else {
            1.0
        }
        .clamp(0.0, 1.0);
        egui::Area::new("banner".into())
            .anchor(egui::Align2::CENTER_TOP, [0.0, 90.0])
            .interactable(false)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    let sub =
                        ["Layer I", "Layer II", "Layer III", "Layer IV", "Journey's end"][layer.index()];
                    ui.label(
                        egui::RichText::new(sub)
                            .size(16.0)
                            .color(TEXT_DIM.gamma_multiply(alpha)),
                    );
                    ui.label(
                        egui::RichText::new(layer.name())
                            .size(48.0)
                            .color(ACCENT.gamma_multiply(alpha)),
                    );
                });
            });
        run.banner = (t < 3.8).then_some((layer, t + dt));
    }

    // ---- item popup ----------------------------------------------------------
    if let Some((item, t)) = run.popup {
        let def = item.def();
        egui::Area::new("popup".into())
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 120.0])
            .interactable(false)
            .show(ctx, |ui| {
                ui::panel_frame().show(ui, |ui| {
                    ui.set_max_width(420.0);
                    ui.horizontal(|ui| {
                        ui::icon(ui, &icons, def.icon, 48.0);
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(def.name).size(24.0).color(ACCENT));
                            ui.label(def.desc);
                            ui.label(egui::RichText::new(def.flavor).italics().color(TEXT_DIM));
                        });
                    });
                });
            });
        run.popup = (t < 4.5).then_some((item, t + dt));
    }

    // ---- achievement toasts ---------------------------------------------------
    // Fixed-size cards stacked down the right edge under the depth panel, so
    // they never cover the vitals, items, prompts or the item popup.
    let mut y = depth_bottom + 12.0;
    let bottom_limit = ctx.viewport_rect().bottom() - 150.0;
    let mut shown = 0;
    for (i, toast) in run.toasts.iter().enumerate() {
        // Out of room: the rest wait their turn.
        if i > 0 && y + TOAST_SIZE.y > bottom_limit {
            break;
        }
        shown += 1;
        let def = toast.achievement.def();
        let t = toast.time;
        let slide = if t < 0.3 {
            (1.0 - t / 0.3).powi(2) * (TOAST_SIZE.x + 24.0)
        } else {
            0.0
        };
        let alpha = if t > TOAST_LIFE - 0.6 {
            ((TOAST_LIFE - t) / 0.6).clamp(0.0, 1.0)
        } else {
            1.0
        };
        egui::Area::new(egui::Id::new(("toast", i)))
            .anchor(egui::Align2::RIGHT_TOP, [-12.0 + slide, y])
            .interactable(false)
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                ui.multiply_opacity(alpha);
                ui::panel_frame()
                    .stroke(egui::Stroke::new(2.0, ACCENT))
                    .show(ui, |ui| {
                        let inner = TOAST_SIZE - egui::vec2(20.0, 20.0);
                        ui.set_min_size(inner);
                        ui.set_max_size(inner);
                        ui.horizontal_top(|ui| {
                            ui::icon(ui, &icons, toast.achievement.icon(), 40.0);
                            ui.vertical(|ui| {
                                ui.set_max_width(inner.x - 52.0);
                                ui.label(egui::RichText::new(def.name).color(ACCENT));
                                ui.add(
                                    egui::Label::new(
                                        egui::RichText::new(format!(
                                            "Unlocked {}",
                                            toast.achievement.reward_name()
                                        ))
                                        .color(TEXT_DIM),
                                    )
                                    .wrap(),
                                );
                            });
                        });
                    });
            });
        y += TOAST_SIZE.y + 8.0;
    }
    for t in run.toasts.iter_mut().take(shown) {
        t.time += dt;
    }
    while run.toasts.front().is_some_and(|t| t.time > TOAST_LIFE) {
        run.toasts.pop_front();
    }

    // ---- low health / death vignette --------------------------------------------
    let danger = if matches!(run.phase, Phase::Dying(_)) {
        0.6
    } else {
        (1.0 - player.hp / 40.0).clamp(0.0, 0.4)
    };
    let flash = if player.hurt_flash > 0.3 { 0.25 } else { 0.0 };
    if danger + flash > 0.0 {
        ctx.layer_painter(egui::LayerId::background()).rect_filled(
            ctx.viewport_rect(),
            0.0,
            egui::Color32::from_rgba_unmultiplied(120, 0, 0, ((danger + flash) * 140.0) as u8),
        );
    }
    if let Phase::Won(t) = run.phase {
        let a = (t / 1.5).min(1.0);
        ctx.layer_painter(egui::LayerId::background()).rect_filled(
            ctx.viewport_rect(),
            0.0,
            egui::Color32::from_rgba_unmultiplied(255, 240, 210, (a * 200.0) as u8),
        );
    }

    // ---- pause menu ------------------------------------------------------------
    if paused.0 {
        egui::Area::new("pause".into())
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui::panel_frame().show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(egui::RichText::new("Paused").size(32.0).color(ACCENT));
                        ui.add_space(8.0);
                        if pause_menu.settings_open {
                            ui.set_max_width(560.0);
                            ui.set_max_height(ctx.viewport_rect().height() * 0.8);
                            let tab = &mut pause_menu.tab;
                            crate::settings::settings_ui(ui, &mut save, &mut rebind, tab);
                            ui.add_space(8.0);
                            if ui::menu_button(ui, "Back").clicked() {
                                pause_menu.settings_open = false;
                            }
                            return;
                        }
                        if ui::menu_button(ui, "Resume").clicked() {
                            paused.0 = false;
                        }
                        if ui::menu_button(ui, "Settings").clicked() {
                            pause_menu.settings_open = true;
                        }
                        ui.add_space(8.0);
                        if ui::menu_button(ui, "Abandon run").clicked() {
                            paused.0 = false;
                            run.phase = Phase::Over;
                            sfx.write(Sfx::ui("ui_click"));
                        }
                    });
                });
            });
    }

    // ---- summary -----------------------------------------------------------------
    if run.phase == Phase::Over {
        summary(ctx, &mut commands, &run, &save, &icons, &mut sfx);
    }

    pointer.0 = ctx.is_pointer_over_egui() || paused.0 || run.phase == Phase::Over;
    Ok(())
}

fn summary(
    ctx: &egui::Context,
    commands: &mut Commands,
    run: &Run,
    save: &SaveData,
    icons: &UiIcons,
    sfx: &mut MessageWriter<Sfx>,
) {
    ctx.layer_painter(egui::LayerId::background()).rect_filled(
        ctx.viewport_rect(),
        0.0,
        egui::Color32::from_black_alpha(170),
    );
    egui::Area::new("summary".into())
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui::panel_frame().show(ui, |ui| {
                ui.set_width(520.0);
                ui.vertical_centered(|ui| {
                    let (title, color) = if run.won {
                        ("You reached the core", ACCENT)
                    } else {
                        ("Your descent has ended", DANGER)
                    };
                    ui.label(egui::RichText::new(title).size(32.0).color(color));
                    let cause = if run.won {
                        "The planet's heart is yours.".to_string()
                    } else {
                        run.cause
                            .map_or("Abandoned the run".to_string(), |c| c.death_text().to_string())
                    };
                    ui.horizontal(|ui| {
                        let w = 24.0 + cause.len() as f32 * 9.0;
                        ui.add_space((ui.available_width() - w).max(0.0) / 2.0);
                        ui::icon(ui, icons, if run.won { icon::TROPHY } else { icon::SKULL }, 16.0);
                        ui.label(egui::RichText::new(cause).color(TEXT_DIM));
                    });
                });
                ui.add_space(8.0);
                egui::Grid::new("stats")
                    .num_columns(2)
                    .spacing([24.0, 6.0])
                    .show(ui, |ui| {
                        let rows = [
                            (
                                "Deepest point",
                                format!(
                                    "{} m ({})",
                                    run.max_depth / 4,
                                    Layer::at_depth(run.max_depth).name()
                                ),
                            ),
                            ("Time", ui::clock(run.elapsed)),
                            ("Shards gathered", run.shards.to_string()),
                            ("Cells dug", run.stats.cells_dug.to_string()),
                            ("Seed", run.seed.to_string()),
                        ];
                        for (k, v) in rows {
                            ui.label(egui::RichText::new(k).color(TEXT_DIM));
                            ui.label(v);
                            ui.end_row();
                        }
                    });
                if !run.items.is_empty() {
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("Items").color(TEXT_DIM));
                    ui.horizontal_wrapped(|ui| {
                        for item in &run.items {
                            ui::icon(ui, icons, item.def().icon, 32.0).on_hover_text(item.def().name);
                        }
                    });
                }
                if !run.earned.is_empty() {
                    ui.add_space(6.0);
                    ui.label(egui::RichText::new("Achievements this run").color(TEXT_DIM));
                    for a in &run.earned {
                        ui.horizontal(|ui| {
                            ui::icon(ui, icons, a.icon(), 24.0);
                            ui.label(egui::RichText::new(a.def().name).color(ACCENT));
                            ui.label(
                                egui::RichText::new(format!("unlocked {}", a.reward_name())).color(TEXT_DIM),
                            );
                        });
                    }
                }
                ui.add_space(10.0);
                ui.vertical_centered(|ui| {
                    if ui::menu_button(ui, "New run").clicked() {
                        sfx.write(Sfx::ui("ui_click"));
                        let loadout = if save.loadout_unlocked(run.loadout) {
                            run.loadout
                        } else {
                            Default::default()
                        };
                        start_run(commands, random_seed(), loadout);
                    }
                    if ui::menu_button(ui, &format!("Retry seed {}", run.seed)).clicked() {
                        sfx.write(Sfx::ui("ui_click"));
                        start_run(commands, run.seed, run.loadout);
                    }
                    if ui::menu_button(ui, "Main menu").clicked() {
                        sfx.write(Sfx::ui("ui_click"));
                        commands.set_state(AppState::Menu);
                    }
                });
            });
        });
}
