//! In-run UI: HUD, popups, toasts, layer banners, pause menu and the
//! death/victory summary.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use sbct_sim::descent::{DESCENT_HEIGHT, Layer};

use super::entities::Prompt;
use super::player::RunPlayer;
use super::save::{DigMode, SaveData};
use super::scrolls::{self, FusionId, ScrollId, icon};
use super::{Phase, Popup, Run, RunConfig, Spell, random_seed, start_run};
use crate::AppState;
use crate::audio::Sfx;
use crate::controls::Action;
use crate::render::{UiHasPointer, WorldCamera};
use crate::ui::{self, ACCENT, BORDER, DANGER, TEXT_DIM, UiIcons};

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
    mut run: ResMut<Run>,
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
        if run.attunement.is_some() {
            run.attunement = None;
            run.attune_altar = None;
        } else if run.journal.is_some() {
            run.journal = None;
        } else if menu.settings_open {
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
    stalker: Option<Res<super::creatures::StalkerState>>,
) -> Result {
    let stalker = stalker.map_or(super::creatures::StalkerState::default(), |s| {
        super::creatures::StalkerState {
            near: s.near,
            ..Default::default()
        }
    });
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
            ui::ornate(ui, |ui| {
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
                    ui::icon(ui, &icons, icon::MANA, 16.0);
                    ui::meter(
                        ui,
                        run.mana / run.max_mana,
                        egui::Color32::from_rgb(150, 110, 240),
                        140.0,
                    );
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
                        ui::icon(ui, &icons, ScrollId::EmberHeart.icon(), 16.0);
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
                    ui::icon(ui, &icons, icon::LIGHT_ORB, 24.0);
                    ui.label(
                        egui::RichText::new(format!("{}/{}", run.light_charges, run.light_max)).size(16.0),
                    );
                    ui.label(egui::RichText::new(b.hint(Action::LightOrb)).color(TEXT_DIM));
                    ui.add_space(6.0);
                    ui::icon(ui, &icons, icon::SHARD, 24.0);
                    ui.label(egui::RichText::new(run.shards.to_string()).size(16.0));
                    if run.staff_dimmed {
                        ui.add_space(6.0);
                        ui::icon(ui, &icons, icon::EYE_CLOSED, 24.0).on_hover_text("Staff dimmed");
                    }
                });
                ui.horizontal(|ui| {
                    let smart = save.settings.dig_mode == DigMode::Smart;
                    ui::icon(
                        ui,
                        &icons,
                        if smart { icon::SMART_DIG } else { icon::CURSOR_DIG },
                        16.0,
                    );
                    let label = egui::RichText::new(if smart { "Smart dig" } else { "Cursor dig" });
                    ui.label(if smart { label.color(ACCENT) } else { label });
                    if let Some(v) = run.dig_variant() {
                        ui::icon(ui, &icons, v.icon(), 16.0).on_hover_text(v.def().name);
                    }
                    ui.label(
                        egui::RichText::new(format!("{} to switch", b.hint(Action::ToggleDigMode)))
                            .color(TEXT_DIM),
                    );
                });
            });
        });

    // ---- bottom centre: schools and spells ----------------------------------
    egui::Area::new("spells".into())
        .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -12.0])
        .show(ctx, |ui| {
            ui::ornate(ui, |ui| {
                ui.horizontal(|ui| {
                    for school in run.schools() {
                        ui::icon(ui, &icons, school.sigil(), 32.0).on_hover_text(school.name());
                    }
                    if run.second_school.is_none() {
                        ui::icon_tinted(ui, &icons, icon::UNKNOWN, 32.0, egui::Color32::from_gray(120))
                            .on_hover_text("Find an altar to bind a second school");
                    }
                    ui.separator();
                    let selected = run.selected_spell();
                    let spells = run.spells();
                    if spells.is_empty() {
                        ui.label(
                            egui::RichText::new(
                                "No spells yet: find scrolls in chests, on altars and at shrines.",
                            )
                            .color(TEXT_DIM),
                        );
                    }
                    for spell in spells {
                        let (cost, cooldown) = spell.cost();
                        let cd = run.cooldowns.get(&spell).copied().unwrap_or(0.0);
                        let fusion = matches!(spell, Spell::Fusion(_));
                        let r = ui
                            .vertical(|ui| {
                                let resp = ui::icon(ui, &icons, spell.icon(), 32.0);
                                let rect = resp.rect;
                                if cd > 0.0 && cooldown > 0.0 {
                                    let mut shade = rect;
                                    shade.set_top(
                                        rect.bottom() - rect.height() * (cd / cooldown).clamp(0.0, 1.0),
                                    );
                                    ui.painter().rect_filled(
                                        shade,
                                        0.0,
                                        egui::Color32::from_black_alpha(160),
                                    );
                                }
                                let afford = run.mana >= cost;
                                let label = if spell.channelled() {
                                    format!("{cost:.0}/s")
                                } else {
                                    format!("{cost:.0}")
                                };
                                ui.label(egui::RichText::new(label).size(16.0).color(if afford {
                                    egui::Color32::from_rgb(180, 150, 255)
                                } else {
                                    DANGER
                                }));
                            })
                            .response;
                        let color = if fusion {
                            egui::Color32::from_rgb(255, 215, 120)
                        } else {
                            ACCENT
                        };
                        if Some(spell) == selected {
                            ui.painter().rect_stroke(
                                r.rect.expand(3.0),
                                0.0,
                                egui::Stroke::new(2.0, color),
                                egui::StrokeKind::Outside,
                            );
                        }
                        r.on_hover_ui(|ui| {
                            ui.label(egui::RichText::new(spell.name()).color(color));
                            ui.label(spell.desc());
                        });
                    }
                    let passives: Vec<ScrollId> =
                        run.scrolls.iter().copied().filter(|s| !s.is_active()).collect();
                    if !passives.is_empty() {
                        ui.separator();
                        ui.spacing_mut().item_spacing.x = 2.0;
                        for p in passives {
                            let active_dig =
                                p.def().kind != scrolls::Kind::Dig || run.dig_variant() == Some(p);
                            let tint = if active_dig {
                                egui::Color32::WHITE
                            } else {
                                egui::Color32::from_gray(90)
                            };
                            ui::icon_tinted(ui, &icons, p.icon(), 24.0, tint).on_hover_ui(|ui| {
                                ui.label(egui::RichText::new(p.def().name).color(ACCENT));
                                ui.label(p.def().desc);
                            });
                        }
                    }
                });
                if run.spells().len() > 1 {
                    let b = &save.bindings;
                    ui.label(
                        egui::RichText::new(format!(
                            "{}/{} or wheel: switch   {}: cast",
                            b.hint(Action::PrevItem),
                            b.hint(Action::NextItem),
                            b.hint(Action::UseItem)
                        ))
                        .color(TEXT_DIM),
                    );
                }
            });
        });

    // ---- right: depth gauge -------------------------------------------------
    let depth_bottom = egui::Area::new("depth".into())
        .anchor(egui::Align2::RIGHT_TOP, [-12.0, 12.0])
        .show(ctx, |ui| {
            ui::ornate(ui, |ui| {
                ui.label(egui::RichText::new(run.layer.name()).color(ACCENT));
                let depth = player.body.pos.y.max(0.0);
                let to_core = (run.core.y - depth).max(0.0);
                ui.label(format!("Depth {} m", depth as i32 / 4));
                ui.label(egui::RichText::new(format!("Core in {} m", to_core as i32 / 4)).color(TEXT_DIM));
                let (rect, _) = ui.allocate_exact_size(egui::vec2(230.0, 180.0), egui::Sense::hover());
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
                ui::ornate(ui, |ui| {
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
                    let sub = [
                        "The first descent",
                        "The second descent",
                        "The third descent",
                        "The fourth descent",
                        "The final trial",
                    ][layer.index()];
                    ui.label(
                        egui::RichText::new(sub)
                            .size(16.0)
                            .color(TEXT_DIM.gamma_multiply(alpha)),
                    );
                    ui.label(ui::title(layer.name(), 48.0).color(ACCENT.gamma_multiply(alpha)));
                });
            });
        run.banner = (t < 3.8).then_some((layer, t + dt));
    }

    // ---- popup: scroll learned, shattered, fusion, attunement --------------
    if let Some((p, t)) = run.popup {
        let (icon_index, title, body, flavor) = match p {
            Popup::Learned(sc) => (
                sc.icon(),
                sc.def().name.to_string(),
                sc.def().desc.to_string(),
                sc.def().flavor.to_string(),
            ),
            Popup::Shattered(sc) => (
                icon::SHARD,
                format!("{} crumbles", sc.def().name),
                format!(
                    "Not of your schools. It breaks into {} shards.",
                    super::SHATTER_SHARDS
                ),
                String::new(),
            ),
            Popup::Fusion(f) => (
                f.icon(),
                format!("Fusion awakened: {}", f.def().name),
                f.def().desc.to_string(),
                format!(
                    "{} and {} entwine.",
                    f.def().schools.0.name(),
                    f.def().schools.1.name()
                ),
            ),
            Popup::Attuned(school) => (
                school.sigil(),
                format!("Attuned to {}", school.name()),
                school.desc().to_string(),
                "Two schools now bind your staff. Scrolls of others will crumble in your hands.".to_string(),
            ),
        };
        egui::Area::new("popup".into())
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 120.0])
            .interactable(false)
            .show(ctx, |ui| {
                ui::ornate(ui, |ui| {
                    ui.set_max_width(440.0);
                    ui.horizontal(|ui| {
                        ui::icon(ui, &icons, icon_index, 48.0);
                        ui.vertical(|ui| {
                            ui.label(ui::title(title, 24.0).color(ACCENT));
                            ui.label(body);
                            if !flavor.is_empty() {
                                ui.label(egui::RichText::new(flavor).italics().color(TEXT_DIM));
                            }
                        });
                    });
                });
            });
        run.popup = (t < 4.5).then_some((p, t + dt));
    }

    // ---- journal page -----------------------------------------------------------
    if let Some(id) = run.journal {
        let page = &super::lore::JOURNALS[id as usize % super::lore::JOURNALS.len()];
        let mut close = false;
        egui::Area::new("journal".into())
            .anchor(egui::Align2::CENTER_CENTER, [0.0, -20.0])
            .order(egui::Order::Foreground)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(egui::Color32::from_rgb(42, 34, 28))
                    .stroke(egui::Stroke::new(2.0, egui::Color32::from_rgb(120, 96, 64)))
                    .inner_margin(egui::Margin::same(18))
                    .shadow(egui::Shadow {
                        offset: [6, 6],
                        blur: 0,
                        spread: 0,
                        color: egui::Color32::from_black_alpha(160),
                    })
                    .show(ui, |ui| {
                        ui.set_width(420.0);
                        ui.horizontal(|ui| {
                            ui::icon(ui, &icons, icon::JOURNAL, 32.0);
                            ui.label(
                                egui::RichText::new(format!("Journal page {}", id + 1))
                                    .color(egui::Color32::from_rgb(200, 170, 120)),
                            );
                        });
                        ui.add_space(6.0);
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(page.text)
                                    .size(16.0)
                                    .color(egui::Color32::from_rgb(226, 210, 180)),
                            )
                            .wrap(),
                        );
                        ui.add_space(8.0);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Min), |ui| {
                            ui.label(
                                egui::RichText::new(format!("- {}", page.author))
                                    .italics()
                                    .color(egui::Color32::from_rgb(170, 140, 100)),
                            );
                        });
                        ui.add_space(6.0);
                        ui.vertical_centered(|ui| {
                            if ui.button("Close").clicked() {
                                close = true;
                            }
                        });
                    });
            });
        if close {
            run.journal = None;
        }
    }

    // ---- attunement choice ----------------------------------------------------
    if let Some(offer) = run.attunement.clone() {
        let mut pick = None;
        egui::Area::new("attune".into()).anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0]).order(egui::Order::Foreground).show(ctx, |ui| {
            ui::panel_frame().stroke(egui::Stroke::new(2.0, ACCENT)).show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.label(ui::title("The altar offers a second attunement", 24.0).color(ACCENT));
                    ui.label(egui::RichText::new(format!("You are a student of {}. Choose one scroll; its school binds to your staff for this descent.", run.first_school.name())).color(TEXT_DIM));
                });
                ui.add_space(8.0);
                ui.horizontal_top(|ui| {
                    for (i, sc) in offer.iter().enumerate() {
                        let school = sc.def().school;
                        let r = ui::panel_frame()
                            .show(ui, |ui| {
                                ui.set_width(200.0);
                                ui.set_min_height(190.0);
                                ui.vertical_centered(|ui| {
                                    ui.horizontal(|ui| {
                                        if let Some(sch) = school {
                                            ui::icon(ui, &icons, sch.sigil(), 32.0);
                                        }
                                        ui::icon(ui, &icons, sc.icon(), 32.0);
                                    });
                                    ui.label(egui::RichText::new(school.map_or("", |s| s.name())).color(TEXT_DIM));
                                    ui.label(egui::RichText::new(sc.def().name).color(ACCENT));
                                    ui.add(egui::Label::new(sc.def().desc).wrap());
                                    if let (Some(sch), true) = (school, true)
                                        && let Some(f) = FusionId::for_pair(run.first_school, sch)
                                    {
                                        ui.label(egui::RichText::new(format!("Fusion: {}", if save.fusions_discovered.contains(&f) { f.def().name } else { "???" })).color(egui::Color32::from_rgb(255, 215, 120)));
                                    }
                                    ui.label(egui::RichText::new(format!("[{}]", i + 1)).color(TEXT_DIM));
                                });
                            })
                            .response
                            .interact(egui::Sense::click());
                        if r.clicked() {
                            pick = Some(i);
                        }
                    }
                });
                if offer.is_empty() {
                    ui.label("The altar is silent: no other school is open to you yet.");
                }
                ui.vertical_centered(|ui| {
                    ui.label(egui::RichText::new("Esc to decide later").color(TEXT_DIM));
                });
            });
        });
        for (i, key) in [egui::Key::Num1, egui::Key::Num2, egui::Key::Num3]
            .into_iter()
            .enumerate()
        {
            if ctx.input(|inp| inp.key_pressed(key)) && i < offer.len() {
                pick = Some(i);
            }
        }
        if let Some(i) = pick {
            run.attune_pick = Some(i);
        }
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

    // ---- dread: a vignette that tightens with depth, wounds and the Stalker ----
    {
        let depth = run.layer.index() as f32 / 4.0;
        let strength =
            (0.35 + depth * 0.25 + stalker.near * 0.5 + (1.0 - player.hp / player.max_hp) * 0.2).min(1.0);
        let rect = ctx.viewport_rect();
        let painter = ctx.layer_painter(egui::LayerId::background());
        let bands = 10;
        let band = rect.width().min(rect.height()) * 0.22 / bands as f32;
        for i in 0..bands {
            let a = strength * (1.0 - i as f32 / bands as f32).powf(1.6) * 70.0;
            let r = rect.shrink(band * (i as f32 + 0.5));
            let tint = if stalker.near > 0.2 {
                egui::Color32::from_rgba_unmultiplied(20, 0, 30, a as u8)
            } else {
                egui::Color32::from_black_alpha(a as u8)
            };
            painter.rect_stroke(r, 0.0, egui::Stroke::new(band, tint), egui::StrokeKind::Middle);
        }
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
                ui::ornate(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(ui::title("Paused", 32.0).color(ACCENT));
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
            ui::ornate(ui, |ui| {
                ui.set_width(520.0);
                ui.vertical_centered(|ui| {
                    let (title, color) = if run.won {
                        ("The rite is complete", ACCENT)
                    } else {
                        ("Another apprentice lost", DANGER)
                    };
                    ui.label(ui::title(title, 32.0).color(color));
                    let cause = if run.won {
                        "The heart of the world has heard you. Your training is done.".to_string()
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
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    for school in run.schools() {
                        ui::icon(ui, icons, school.sigil(), 24.0);
                        ui.label(egui::RichText::new(school.name()).color(ACCENT));
                    }
                    if let Some(f) = run.fusion {
                        ui.label(egui::RichText::new("·").color(TEXT_DIM));
                        ui::icon(ui, icons, f.icon(), 24.0);
                        ui.label(
                            egui::RichText::new(f.def().name).color(egui::Color32::from_rgb(255, 215, 120)),
                        );
                    }
                });
                if !run.scrolls.is_empty() {
                    ui.label(egui::RichText::new("Scrolls").color(TEXT_DIM));
                    ui.horizontal_wrapped(|ui| {
                        for sc in &run.scrolls {
                            ui::icon(ui, icons, sc.icon(), 32.0).on_hover_text(sc.def().name);
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
                    let config = |seed| RunConfig {
                        seed,
                        choice: if save.start_unlocked(run.start_choice) {
                            run.start_choice
                        } else {
                            Default::default()
                        },
                        first: run.first_school,
                        second: (run.start_choice == super::achievements::StartChoice::TwinSouled)
                            .then_some(run.second_school)
                            .flatten(),
                    };
                    if ui::menu_button(ui, "Descend again").clicked() {
                        start_run(commands, config(random_seed()));
                    }
                    if ui::menu_button(ui, &format!("Retry seed {}", run.seed)).clicked() {
                        start_run(commands, config(run.seed));
                    }
                    if ui::menu_button(ui, "Main menu").clicked() {
                        sfx.write(Sfx::ui("ui_click"));
                        commands.set_state(AppState::Menu);
                    }
                });
            });
        });
}
