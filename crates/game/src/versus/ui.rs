//! Screens for Alien Versus: the designer and lobby, the fight overlay, and
//! the results. Kept sparse on purpose: the creatures are the show.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use sbct_net::PeerId;
use sbct_sim::eco::math::v2;
use sbct_sim::versus::ROUND_TIME;
use sbct_sim::versus::combat::Tactic;
use sbct_sim::versus::design::{Design, Group, Upgrade, build_species};

use crate::AppState;
use crate::audio::UiSfx;
use crate::render::WorldCamera;
use crate::session::EndSession;
use crate::steam::SteamClient;
use crate::theme::{self, ACID, BLOOD, DIM, GOLD, INK, PANEL, RAISED, SHADOW, SKY};
use crate::versus::icons;
use crate::versus::session::Evolution;
use crate::versus::view::{VersusView, portrait_for};
use crate::versus::{Phase, Versus};

fn c32(c: [u8; 3]) -> egui::Color32 {
    egui::Color32::from_rgb(c[0], c[1], c[2])
}

fn screen_ui(ctx: &egui::Context) -> egui::Ui {
    egui::Ui::new(
        ctx.clone(),
        "versus_screen".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    )
}

pub fn ui(
    mut contexts: EguiContexts,
    mut commands: Commands,
    mut versus: ResMut<Versus>,
    mut view: ResMut<VersusView>,
    mut images: ResMut<Assets<Image>>,
    mut sfx: ResMut<UiSfx>,
    keys: Res<ButtonInput<KeyCode>>,
    steam: Option<Res<SteamClient>>,
    camera: Single<(&Camera, &GlobalTransform), With<WorldCamera>>,
    time: Res<Time>,
) -> Result {
    // Portraits first: they need the contexts before the frame's ui.
    let mut portraits: Vec<(u64, Option<(egui::TextureId, egui::Vec2)>)> = Vec::new();
    match versus.phase {
        Phase::Lobby | Phase::Connecting => {
            let d = versus.design.clone();
            let p = portrait_for(&mut view, &mut images, &mut contexts, 0, &d, 0, &[]);
            portraits.push((0, p));
        }
        Phase::Results => {
            let teams = versus.teams.clone();
            let all: Vec<(String, Design)> = teams.iter().map(|(_, n, d)| (n.clone(), d.clone())).collect();
            for (i, (pid, _, d)) in teams.iter().enumerate() {
                let p = portrait_for(
                    &mut view,
                    &mut images,
                    &mut contexts,
                    pid.wrapping_add(1),
                    d,
                    i,
                    &all,
                );
                portraits.push((pid.wrapping_add(1), p));
            }
        }
        Phase::Fight => {}
    }
    let ctx = contexts.ctx_mut()?;
    let (cam, cam_tf) = *camera;

    if !view.keyboard_busy && keys.just_pressed(KeyCode::Escape) {
        commands.insert_resource(EndSession(None));
    }

    match versus.phase {
        Phase::Connecting => {
            let mut root = screen_ui(ctx);
            egui::CentralPanel::default().show(&mut root, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(ui.available_height() / 2.0 - 40.0);
                    ui.label(theme::title("JOINING THE MATCH", 16.0, GOLD));
                    ui.add_space(12.0);
                    if theme::big_button(ui, "CANCEL", egui::vec2(200.0, 36.0), RAISED).clicked() {
                        commands.insert_resource(EndSession(None));
                    }
                });
            });
        }
        Phase::Lobby => lobby_ui(
            ctx,
            &mut versus,
            &mut commands,
            &mut sfx,
            portraits.first().and_then(|p| p.1),
        ),
        Phase::Fight => fight_hud(ctx, &versus, cam, cam_tf, time.elapsed_secs()),
        Phase::Results => {
            fight_hud(ctx, &versus, cam, cam_tf, time.elapsed_secs());
            results_ui(ctx, &mut versus, &mut sfx, &portraits);
        }
    }

    if versus.lobby.is_some() && versus.is_authority() && versus.phase == Phase::Lobby {
        egui::Area::new(egui::Id::new("versus_invite"))
            .anchor(egui::Align2::RIGHT_TOP, [-12.0, 10.0])
            .show(ctx, |ui| {
                if let (Some(steam), Some(lobby)) = (&steam, versus.lobby) {
                    ui.horizontal(|ui| {
                        if ui.button("Invite friends").clicked() {
                            sfx.0.push("ui_click");
                            steam.open_invite_dialog(lobby);
                        }
                        if ui.button("Copy lobby ID").clicked() {
                            sfx.0.push("ui_click");
                            ui.ctx().copy_text(lobby.to_string());
                        }
                    });
                }
            });
    }

    let _ = AppState::Versus;
    view.keyboard_busy = ctx.egui_wants_keyboard_input();
    view.pointer_busy = ctx.is_pointer_over_egui() || ctx.egui_wants_pointer_input();
    Ok(())
}

// ---------------------------------------------------------------------------
// Designer

fn lobby_ui(
    ctx: &egui::Context,
    versus: &mut Versus,
    commands: &mut Commands,
    sfx: &mut UiSfx,
    portrait: Option<(egui::TextureId, egui::Vec2)>,
) {
    let spent = versus.design.cost();
    let left = versus.points.saturating_sub(spent);
    let ready = versus.ready;
    let mut root = screen_ui(ctx);

    egui::Panel::top("versus_top").show(&mut root, |ui| {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.label(theme::title("ALIEN VERSUS", 16.0, GOLD));
            ui.add_space(12.0);
            ui.label(theme::title(
                &format!("ROUND {} OF {}", versus.round, versus.settings.rounds),
                12.0,
                INK,
            ));
            ui.add_space(12.0);
            ui.label(theme::title(
                &format!("{left}"),
                24.0,
                if left == 0 { DIM } else { ACID },
            ));
            ui.label(egui::RichText::new(format!("points left of {}", versus.points)).color(DIM));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Leave").clicked() {
                    sfx.0.push("ui_click");
                    commands.insert_resource(EndSession(None));
                }
                ui.label(
                    egui::RichText::new(format!("{} · {}", versus.local_name, versus.describe())).color(DIM),
                );
            });
        });
        ui.add_space(6.0);
    });

    egui::Panel::left("versus_portrait")
        .exact_size(300.0)
        .resizable(false)
        .show(&mut root, |ui| {
            ui.add_space(8.0);
            ui.add_enabled_ui(!ready, |ui| {
                ui.horizontal(|ui| {
                    ui.add(
                        egui::TextEdit::singleline(&mut versus.design.name)
                            .desired_width(220.0)
                            .char_limit(28)
                            .font(theme::body(22.0)),
                    );
                    if ui
                        .button("⟳")
                        .on_hover_text("New look (colours and shape)")
                        .clicked()
                    {
                        sfx.0.push("ui_click");
                        versus.design.look_seed = versus
                            .design
                            .look_seed
                            .wrapping_mul(6364136223846793005)
                            .wrapping_add(1442695040888963407);
                        versus.design.name = versus.design.auto_name();
                    }
                });
            });
            ui.add_space(6.0);
            egui::Frame::new()
                .fill(SHADOW)
                .stroke(egui::Stroke::new(2.0, theme::BORDER))
                .inner_margin(8)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.set_height(190.0);
                    ui.centered_and_justified(|ui| {
                        if let Some((id, size)) = portrait {
                            let scale = (260.0 / size.x).min(170.0 / size.y).clamp(1.0, 8.0).floor();
                            ui.add(
                                egui::Image::new((id, size * scale))
                                    .texture_options(egui::TextureOptions::NEAREST),
                            );
                        }
                    });
                });
            ui.add_space(8.0);
            let (sp, lo) = build_species(&versus.design, 0, None);
            for line in &sp.description {
                ui.label(
                    egui::RichText::new(line)
                        .size(17.0)
                        .color(egui::Color32::from_gray(205)),
                );
            }
            ui.add_space(8.0);
            let count = lo.count as f32;
            stat(ui, "Health", sp.stats.max_health * count, 300.0, ACID);
            stat(ui, "Speed", sp.stats.speed, 60.0, SKY);
            let dmg = lo.weapons.iter().map(|w| w.damage / w.cooldown).sum::<f32>() * count
                + lo.spit_damage / lo.spit_cooldown * count;
            stat(ui, "Damage/s", dmg, 60.0, BLOOD);
            stat(ui, "Sight", lo.sight, 150.0, GOLD);
            stat(ui, "Armour", lo.armor * 100.0, 70.0, DIM);
            stat(ui, "Energy", 1.0 / lo.metabolism, 180.0, GOLD);
            if lo.stamina > 0.0 {
                stat(ui, "Flight", lo.stamina, 16.0, SKY);
            }
            ui.add_space(8.0);
            ui.label(
                egui::RichText::new("Every upgrade shows on the body.")
                    .small()
                    .color(DIM),
            );
            if let Some(why) = Upgrade::ALL.iter().find_map(|&u| {
                (versus.design.level(u) > 0)
                    .then(|| u.requires(&versus.design))
                    .flatten()
                    .map(|w| format!("{}: {w}", u.label()))
            }) {
                ui.label(egui::RichText::new(why).small().color(BLOOD));
            }
        });

    egui::Panel::right("versus_players")
        .exact_size(260.0)
        .resizable(false)
        .show(&mut root, |ui| {
            ui.add_space(8.0);
            ui.label(theme::title("PLAYERS", 12.0, GOLD));
            ui.add_space(4.0);
            for s in &versus.slots {
                ui.horizontal(|ui| {
                    let me = s.id == versus.local_id;
                    let name = if me { format!("{} (you)", s.name) } else { s.name.clone() };
                    ui.label(egui::RichText::new(name).color(if me { GOLD } else { INK }));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if s.wins > 0 {
                            ui.label(egui::RichText::new(format!("{}★", s.wins)).color(GOLD));
                        }
                        let (text, colour) = if s.spectator {
                            ("watching", DIM)
                        } else if s.ready {
                            ("ready", ACID)
                        } else {
                            ("designing", DIM)
                        };
                        ui.label(egui::RichText::new(text).small().color(colour));
                    });
                });
            }
            if versus.slots.len() < 2 {
                ui.add_space(6.0);
                ui.label(egui::RichText::new("Waiting for an opponent to join.").small().color(DIM));
            }
            ui.add_space(12.0);
            // Match length: the host's call.
            ui.label(theme::title("ROUNDS", 12.0, GOLD));
            ui.add_space(2.0);
            if versus.is_authority() {
                ui.horizontal(|ui| {
                    let current = versus.settings.rounds;
                    let mut pick: Option<u8> = None;
                    for n in [1u8, 3, 5, 7] {
                        if ui.selectable_label(n == current, format!(" {n} ")).clicked() {
                            pick = Some(n);
                        }
                    }
                    if let Some(n) = pick {
                        sfx.0.push("ui_click");
                        versus.set_rounds(n);
                    }
                });
                ui.label(
                    egui::RichText::new(format!(
                        "First to {} wins, or the lead after {} rounds.",
                        versus.wins_to_clinch(),
                        versus.settings.rounds
                    ))
                    .small()
                    .color(DIM),
                );
            } else {
                ui.label(
                    egui::RichText::new(format!(
                        "Best of {}: first to {} wins.",
                        versus.settings.rounds,
                        versus.wins_to_clinch()
                    ))
                    .color(INK),
                );
            }
            ui.add_space(14.0);
            let label = if ready { "NOT READY" } else { "READY" };
            let fill = if ready { RAISED } else { egui::Color32::from_rgb(46, 110, 60) };
            if theme::big_button(ui, label, egui::vec2(236.0, 44.0), fill).clicked() {
                if ready {
                    sfx.0.push("ui_click");
                    versus.unready();
                } else {
                    sfx.0.push("ready");
                    versus.set_ready();
                }
            }
            if ready {
                let waiting = versus.waiting_on();
                ui.add_space(4.0);
                let text = if waiting.is_empty() {
                    "Starting…".to_string()
                } else {
                    format!("Waiting for {}", waiting.join(", "))
                };
                ui.label(egui::RichText::new(text).small().color(DIM));
            }
            ui.add_space(8.0);
            ui.add_enabled_ui(!ready, |ui| {
                if ui
                    .add_sized([236.0, 30.0], egui::Button::new("Spend the rest at random"))
                    .clicked()
                {
                    sfx.0.push("ui_click");
                    versus.auto_fill();
                }
            });
            // What everyone grew last round.
            let round = versus.round;
            let recent: Vec<Evolution> = versus
                .evolutions
                .iter()
                .filter(|e| e.round + 1 == round && !e.gained.is_empty())
                .cloned()
                .collect();
            if !recent.is_empty() {
                ui.add_space(14.0);
                ui.label(theme::title("LAST ROUND'S EVOLUTIONS", 10.0, GOLD));
                ui.add_space(4.0);
                egui::ScrollArea::vertical()
                    .max_height(220.0)
                    .auto_shrink([false, true])
                    .show(ui, |ui| {
                        for e in &recent {
                            evolution_block(ui, e, e.player == versus.local_id);
                        }
                    });
            }
            ui.add_space(12.0);
            ui.label(theme::title("HOW IT WORKS", 10.0, GOLD));
            ui.label(
                egui::RichText::new(
                    "Spend points on your species. Both are dropped into a random arena and fight on their own. Everything burns energy: eat fruit or prey, or starve. After each round everyone gets more points to evolve; nothing can be taken away.",
                )
                .small()
                .color(egui::Color32::from_gray(185)),
            );
        });

    egui::CentralPanel::default().show(&mut root, |ui| {
        ui.add_enabled_ui(!ready, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add_space(4.0);
                    for group in Group::ALL {
                        ui.label(theme::title(&group.label().to_uppercase(), 10.0, GOLD));
                        ui.add_space(4.0);
                        for u in Upgrade::ALL {
                            if u.group() != group {
                                continue;
                            }
                            upgrade_row(ui, versus, sfx, u, left);
                        }
                        ui.add_space(12.0);
                    }
                });
        });
    });
}

/// One species' gains: "+Wings: Membrane · Size 1→2".
fn evolution_block(ui: &mut egui::Ui, e: &Evolution, mine: bool) {
    ui.label(egui::RichText::new(&e.species).color(if mine { GOLD } else { INK }));
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(6.0, 4.0);
        for &(u, from, to) in &e.gained {
            icons::draw(ui, u, 2.0, true);
            let text = match u.choices() {
                Some(names) => format!("{}: {}", u.label(), names[to as usize]),
                None if from == 0 => format!("{} {to}", u.label()),
                None => format!("{} {from}→{to}", u.label()),
            };
            ui.label(egui::RichText::new(text).small().color(ACID));
        }
    });
    ui.add_space(4.0);
}

fn stat(ui: &mut egui::Ui, label: &str, v: f32, max: f32, colour: egui::Color32) {
    ui.horizontal(|ui| {
        ui.add_sized([70.0, 16.0], egui::Label::new(egui::RichText::new(label).small()));
        theme::bar(ui, egui::vec2(140.0, 12.0), v / max, colour, false);
        ui.label(egui::RichText::new(format!("{v:.0}")).small().color(DIM));
    });
}

fn upgrade_row(ui: &mut egui::Ui, versus: &mut Versus, sfx: &mut UiSfx, u: Upgrade, left: u32) {
    let level = versus.design.level(u);
    let floor = versus.base.as_ref().map_or(0, |b| b.level(u));
    let blocked = u.requires(&versus.design);
    let row_colour = if blocked.is_some() {
        BLOOD.gamma_multiply(0.8)
    } else if level > 0 {
        INK
    } else {
        egui::Color32::from_gray(190)
    };
    ui.horizontal(|ui| {
        icons::draw(ui, u, 3.0, level > 0);
        let label = ui.add_sized(
            [140.0, 24.0],
            egui::Label::new(egui::RichText::new(u.label()).color(row_colour)).halign(egui::Align::LEFT),
        );
        let mut hint = u.hint().to_string();
        if let Some(why) = blocked {
            hint = format!("{hint}\n({why})");
        }
        label.on_hover_text(hint);
        match u.choices() {
            Some(names) => {
                for (i, name) in names.iter().enumerate() {
                    let lv = i as u8;
                    let cost = versus.design.delta(u, lv);
                    let affordable = cost <= left as i64;
                    let allowed = lv >= floor && (affordable || lv == level);
                    let text = if lv == level || cost <= 0 {
                        name.to_string()
                    } else {
                        format!("{name} +{cost}")
                    };

                    let selected = lv == level;
                    let resp = ui
                        .add_enabled_ui(allowed, |ui| ui.selectable_label(selected, text))
                        .inner;
                    if resp.clicked() && !selected {
                        sfx.0.push("ui_click");
                        versus.design.set(u, lv);
                    }
                }
            }
            None => {
                let can_down = level > floor;
                if ui
                    .add_enabled(can_down, egui::Button::new("−").min_size(egui::vec2(26.0, 22.0)))
                    .clicked()
                {
                    sfx.0.push("ui_click");
                    versus.design.set(u, level - 1);
                }
                for i in 0..u.max() {
                    let on = i < level;
                    let locked = i < floor;
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
                    let colour = if on && locked {
                        GOLD.gamma_multiply(0.55)
                    } else if on {
                        GOLD
                    } else {
                        RAISED
                    };
                    ui.painter().rect_filled(rect.shrink(2.0), 0.0, colour);
                    ui.painter().rect_stroke(
                        rect.shrink(2.0),
                        0.0,
                        egui::Stroke::new(1.0, theme::BORDER),
                        egui::StrokeKind::Inside,
                    );
                }
                let next_cost = if level < u.max() {
                    versus.design.delta(u, level + 1)
                } else {
                    0
                };
                let can_up = level < u.max() && next_cost <= left as i64;
                let plus = if level < u.max() {
                    format!("+{next_cost}")
                } else {
                    "max".into()
                };
                if ui
                    .add_enabled(can_up, egui::Button::new(plus).min_size(egui::vec2(50.0, 22.0)))
                    .clicked()
                {
                    sfx.0.push("ui_click");
                    versus.design.set(u, level + 1);
                }
            }
        }
    });
}

// ---------------------------------------------------------------------------
// Fight

fn fight_hud(ctx: &egui::Context, versus: &Versus, cam: &Camera, cam_tf: &GlobalTransform, now: f32) {
    let Some(mirror) = &versus.mirror else { return };
    let screen = ctx.content_rect();
    let painter = ctx.layer_painter(egui::LayerId::new(
        egui::Order::Foreground,
        egui::Id::new("versus_fx"),
    ));
    // Effects go under the HUD so the cards stay readable.
    let fx = ctx.layer_painter(egui::LayerId::background());

    // The finishing blow: letterbox, flash, speed lines. Nothing else
    // slows the world down.
    let slow = mirror.time_scale < 0.9 && versus.phase == Phase::Fight;
    if slow {
        let h = (screen.height() * 0.09).round();
        fx.rect_filled(
            egui::Rect::from_min_size(screen.min, egui::vec2(screen.width(), h)),
            0.0,
            egui::Color32::BLACK,
        );
        fx.rect_filled(
            egui::Rect::from_min_size(
                egui::pos2(screen.min.x, screen.max.y - h),
                egui::vec2(screen.width(), h),
            ),
            0.0,
            egui::Color32::BLACK,
        );
    }
    if mirror.flash > 0.0 {
        fx.rect_filled(
            screen,
            0.0,
            egui::Color32::from_white_alpha((mirror.flash * 140.0) as u8),
        );
    }
    if let Some((p, t)) = mirror.focus
        && let Ok(sp) = cam.world_to_viewport(cam_tf, Vec3::new(p.x, -p.y, 0.0))
    {
        let centre = egui::pos2(sp.x, sp.y);
        let a = (t * 1.2).clamp(0.0, 1.0);
        let n = 28;
        for i in 0..n {
            let ang = i as f32 / n as f32 * std::f32::consts::TAU + now * 0.3;
            let dir = egui::vec2(ang.cos(), ang.sin());
            let far = centre + dir * 1400.0;
            let near = centre + dir * (120.0 + ((i * 7) % 5) as f32 * 30.0);
            fx.line_segment(
                [near, far],
                egui::Stroke::new(
                    2.0 + (i % 3) as f32,
                    egui::Color32::from_white_alpha((a * 60.0) as u8),
                ),
            );
        }
    }

    // Team cards: name, health, energy, who's still up.
    let n = versus.teams.len().max(1);
    for (i, (pid, player, design)) in versus.teams.iter().enumerate() {
        let colour = c32(mirror.team_colour_of(i));
        let health = mirror.team_health(i);
        let energy = mirror.team_energy(i);
        let alive = mirror.alive(i);
        let total = mirror.teams.get(i).map_or(1, |t| t.loadout.count);
        let right = n == 2 && i == 1;
        let anchor = if right {
            egui::Align2::RIGHT_TOP
        } else {
            egui::Align2::LEFT_TOP
        };
        let offset = if n == 2 {
            [if right { -14.0 } else { 14.0 }, 14.0]
        } else {
            [14.0, 14.0 + i as f32 * 78.0]
        };
        egui::Area::new(egui::Id::new(("team_card", i)))
            .anchor(anchor, offset)
            .interactable(false)
            .show(ctx, |ui| {
                let layout = if right {
                    egui::Layout::top_down(egui::Align::RIGHT)
                } else {
                    egui::Layout::top_down(egui::Align::LEFT)
                };
                egui::Frame::new()
                    .fill(SHADOW.gamma_multiply(0.85))
                    .stroke(egui::Stroke::new(2.0, colour))
                    .inner_margin(8)
                    .show(ui, |ui| {
                        ui.set_max_width(250.0);
                        ui.with_layout(layout, |ui| {
                            let me = *pid == versus.local_id;
                            ui.label(theme::title(
                                &format!("{}{}", design.name.to_uppercase(), if me { " (YOU)" } else { "" }),
                                12.0,
                                colour,
                            ));
                            ui.label(egui::RichText::new(player).small().color(DIM));
                            theme::bar(ui, egui::vec2(230.0, 14.0), health, colour, right);
                            theme::bar(
                                ui,
                                egui::vec2(230.0, 8.0),
                                energy,
                                if energy < 0.2 { BLOOD } else { GOLD },
                                right,
                            );
                            ui.horizontal(|ui| {
                                for k in 0..total {
                                    let (r, _) =
                                        ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                                    let on = k < alive;
                                    ui.painter().rect_filled(
                                        r.shrink(1.0),
                                        0.0,
                                        if on { colour } else { RAISED },
                                    );
                                }
                            });
                        });
                    });
            });
    }

    // Clock and round.
    egui::Area::new(egui::Id::new("versus_clock"))
        .anchor(egui::Align2::CENTER_TOP, [0.0, 10.0])
        .interactable(false)
        .show(ctx, |ui| {
            ui.set_width(320.0);
            ui.vertical_centered(|ui| {
                ui.label(theme::title(
                    &format!("ROUND {} / {}", versus.round, versus.settings.rounds),
                    10.0,
                    GOLD,
                ));
                let left = (ROUND_TIME - mirror.clock).max(0.0);
                let text = if mirror.frenzy {
                    "FRENZY".to_string()
                } else {
                    format!("{:.0}", left)
                };
                let colour = if mirror.frenzy { GOLD } else { INK };
                ui.label(theme::title(&text, 24.0, colour));
            });
        });

    // Callouts and labels only while the fight is on; the results panel
    // wants a quiet backdrop.
    if versus.phase != Phase::Fight {
        return;
    }
    for c in &mirror.callouts {
        let t = (c.life / c.max_life).clamp(0.0, 1.0);
        let alpha = (t * 2.5).clamp(0.0, 1.0);
        let colour = egui::Color32::from_rgba_unmultiplied(
            c.colour[0],
            c.colour[1],
            c.colour[2],
            (alpha * 255.0) as u8,
        );
        match c.pos {
            Some(p) => {
                let Ok(sp) = cam.world_to_viewport(cam_tf, Vec3::new(p.x, -p.y, 0.0)) else {
                    continue;
                };
                painter.text(
                    egui::pos2(sp.x, sp.y - 10.0),
                    egui::Align2::CENTER_BOTTOM,
                    &c.text,
                    theme::silk(16.0),
                    colour,
                );
            }
            None => {
                let pop = 1.0 + (1.0 - t).powi(3) * 0.4;
                let size = (32.0 * pop / 8.0).round() * 8.0;
                let pos = egui::pos2(screen.center().x, screen.center().y - screen.height() * 0.22);
                painter.text(
                    pos + egui::vec2(4.0, 4.0),
                    egui::Align2::CENTER_CENTER,
                    &c.text,
                    theme::pixel(size),
                    egui::Color32::from_black_alpha((alpha * 220.0) as u8),
                );
                painter.text(
                    pos,
                    egui::Align2::CENTER_CENTER,
                    &c.text,
                    theme::pixel(size),
                    colour,
                );
            }
        }
    }
    // What each creature is doing, floating above it.
    let font = theme::silk(12.0);
    for (i, c) in mirror.creatures.iter().enumerate() {
        if !c.alive() {
            continue;
        }
        let Some(t) = mirror.tactics.get(i) else { continue };
        let sp = &mirror.species[c.species];
        let top = c.pos - v2(0.0, sp.body.top + sp.body.stance * 0.2 + 5.0);
        let Ok(p) = cam.world_to_viewport(cam_tf, Vec3::new(top.x, -top.y, 0.0)) else {
            continue;
        };
        let colour = c32(mirror.team_colour_of(c.species)).gamma_multiply(0.9);
        let starving = mirror.energy.get(i).is_some_and(|e| *e <= 0.0);
        let text = if starving { "starving" } else { t.label() };
        painter.text(
            egui::pos2(p.x, p.y),
            egui::Align2::CENTER_BOTTOM,
            text,
            font.clone(),
            if starving { BLOOD } else { colour },
        );
        if *t == Tactic::Forage || *t == Tactic::Hide {
            continue;
        }
    }
}

// ---------------------------------------------------------------------------
// Results

fn results_ui(
    ctx: &egui::Context,
    versus: &mut Versus,
    sfx: &mut UiSfx,
    portraits: &[(u64, Option<(egui::TextureId, egui::Vec2)>)],
) {
    let Some(result) = versus.result.clone() else {
        return;
    };
    let teams = versus.teams.clone();
    let evolutions: Vec<(PeerId, Evolution)> = teams
        .iter()
        .filter_map(|(pid, _, _)| versus.latest_evolution(*pid).map(|e| (*pid, e.clone())))
        .collect();
    egui::Window::new("versus_results")
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(
            egui::Frame::window(&ctx.global_style())
                .fill(PANEL.gamma_multiply(0.97))
                .inner_margin(18),
        )
        .show(ctx, |ui| {
            ui.set_width(560.0);
            ui.vertical_centered(|ui| {
                let headline = match result.winner.and_then(|w| teams.get(w as usize)) {
                    Some((pid, _, d)) => {
                        let who = if *pid == versus.local_id {
                            "YOU WIN THE ROUND"
                        } else {
                            "ROUND LOST"
                        };
                        format!("{who}: {}", d.name.to_uppercase())
                    }
                    None => "DRAW".to_string(),
                };
                let colour = result
                    .winner
                    .and_then(|w| versus.mirror.as_ref().map(|m| c32(m.team_colour_of(w as usize))))
                    .unwrap_or(GOLD);
                ui.label(theme::title(&headline, 16.0, colour));
                ui.label(
                    egui::RichText::new(format!(
                        "Round {} of {} · {:.0}s",
                        versus.round, versus.settings.rounds, result.duration
                    ))
                    .color(DIM),
                );
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    let w = ui.available_width() / teams.len().max(1) as f32;
                    for (i, (pid, player, design)) in teams.iter().enumerate() {
                        ui.allocate_ui(egui::vec2(w, 200.0), |ui| {
                            ui.vertical_centered(|ui| {
                                if let Some((_, Some((id, size)))) =
                                    portraits.iter().find(|(k, _)| *k == pid.wrapping_add(1))
                                {
                                    let scale = (w / size.x * 0.8).min(90.0 / size.y).clamp(1.0, 6.0).floor();
                                    ui.add(
                                        egui::Image::new((*id, *size * scale))
                                            .texture_options(egui::TextureOptions::NEAREST),
                                    );
                                }
                                let tc = versus.mirror.as_ref().map_or(GOLD, |m| c32(m.team_colour_of(i)));
                                ui.label(egui::RichText::new(&design.name).size(22.0).color(tc));
                                ui.label(egui::RichText::new(player).small().color(DIM));
                                let wins = versus.wins.get(i).copied().unwrap_or(0);
                                ui.label(theme::title(&"*".repeat(wins as usize), 12.0, GOLD));
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} kills · {:.0} damage · {} left",
                                        result.kills.get(i).copied().unwrap_or(0),
                                        result.damage.get(i).copied().unwrap_or(0.0),
                                        result.survivors.get(i).copied().unwrap_or(0)
                                    ))
                                    .small()
                                    .color(DIM),
                                );
                                if let Some((_, e)) = evolutions.iter().find(|(p, _)| p == pid)
                                    && !e.gained.is_empty()
                                {
                                    ui.add_space(4.0);
                                    ui.label(egui::RichText::new("evolved this round").small().color(ACID));
                                    ui.horizontal_wrapped(|ui| {
                                        ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                                        for &(u, _, _) in &e.gained {
                                            icons::draw(ui, u, 2.0, true).on_hover_text(u.label());
                                        }
                                    });
                                }
                            });
                        });
                    }
                });
                ui.add_space(12.0);
                if let Some(winner) = versus.match_winner {
                    let name = versus
                        .slots
                        .iter()
                        .find(|s| s.id == winner)
                        .map_or("Someone".to_string(), |s| s.name.clone());
                    let text = if winner == versus.local_id {
                        "YOU WIN THE MATCH".to_string()
                    } else {
                        format!("{} WINS THE MATCH", name.to_uppercase())
                    };
                    ui.label(theme::title(&text, 16.0, GOLD));
                    ui.add_space(6.0);
                    if versus.is_authority() {
                        if theme::big_button(ui, "REMATCH", egui::vec2(240.0, 40.0), RAISED).clicked() {
                            sfx.0.push("ui_click");
                            versus.leave_results();
                        }
                    } else {
                        ui.label(egui::RichText::new("Rematch (host decides)").small().color(DIM));
                    }
                } else if theme::big_button(
                    ui,
                    "EVOLVE",
                    egui::vec2(240.0, 40.0),
                    egui::Color32::from_rgb(46, 110, 60),
                )
                .clicked()
                {
                    sfx.0.push("ui_click");
                    versus.leave_results();
                }
                ui.add_space(4.0);
                ui.label(
                    egui::RichText::new("Watch the arena behind this panel as long as you like.")
                        .small()
                        .color(DIM),
                );
            });
        });
}
