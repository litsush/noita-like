//! Screens for Alien Versus: the designer and lobby, the fight overlay, and
//! the results. Kept sparse on purpose: the creatures are the show.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use sbct_sim::eco::math::v2;
use sbct_sim::versus::ROUND_TIME;
use sbct_sim::versus::combat::Tactic;
use sbct_sim::versus::design::{Design, Group, Upgrade, build_species};

use crate::AppState;
use crate::render::WorldCamera;
use crate::session::EndSession;
use crate::steam::SteamClient;
use crate::versus::view::{VersusView, portrait_for};
use crate::versus::{Phase, Versus};

const GOLD: egui::Color32 = egui::Color32::from_rgb(230, 170, 70);
const DIM: egui::Color32 = egui::Color32::from_gray(150);

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
                    ui.label(egui::RichText::new("Joining the match…").size(22.0));
                    if ui.button("Cancel").clicked() {
                        commands.insert_resource(EndSession(None));
                    }
                });
            });
        }
        Phase::Lobby => lobby_ui(
            ctx,
            &mut versus,
            &mut commands,
            portraits.first().and_then(|p| p.1),
        ),
        Phase::Fight => fight_hud(ctx, &versus, cam, cam_tf, time.elapsed_secs()),
        Phase::Results => {
            fight_hud(ctx, &versus, cam, cam_tf, time.elapsed_secs());
            let replaying = versus.mirror.as_ref().is_some_and(|m| m.replaying);
            if replaying {
                if keys.just_pressed(KeyCode::Space) || ctx.input(|i| i.pointer.any_click()) {
                    versus.skip_replay();
                }
            } else {
                results_ui(ctx, &mut versus, &portraits);
            }
        }
    }

    if versus.lobby.is_some() && versus.is_authority() && versus.phase == Phase::Lobby {
        egui::Area::new(egui::Id::new("versus_invite"))
            .anchor(egui::Align2::RIGHT_TOP, [-12.0, 10.0])
            .show(ctx, |ui| {
                if let (Some(steam), Some(lobby)) = (&steam, versus.lobby) {
                    ui.horizontal(|ui| {
                        if ui.button("Invite friends").clicked() {
                            steam.open_invite_dialog(lobby);
                        }
                        if ui.button("Copy lobby ID").clicked() {
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
    portrait: Option<(egui::TextureId, egui::Vec2)>,
) {
    let spent = versus.design.cost();
    let left = versus.points.saturating_sub(spent);
    let ready = versus.ready;
    let mut root = screen_ui(ctx);

    egui::Panel::top("versus_top").show(&mut root, |ui| {
        ui.add_space(4.0);
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new("ALIEN VERSUS")
                    .size(22.0)
                    .strong()
                    .color(GOLD),
            );
            ui.separator();
            ui.label(egui::RichText::new(format!("Round {}", versus.round)).size(16.0));
            ui.separator();
            ui.label(
                egui::RichText::new(format!("{left}"))
                    .size(26.0)
                    .strong()
                    .color(if left == 0 {
                        DIM
                    } else {
                        egui::Color32::from_rgb(140, 230, 160)
                    }),
            );
            ui.label(egui::RichText::new(format!("points left of {}", versus.points)).color(DIM));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button("Leave").clicked() {
                    commands.insert_resource(EndSession(None));
                }
                ui.label(
                    egui::RichText::new(format!("{} · {}", versus.local_name, versus.describe())).color(DIM),
                );
            });
        });
        ui.add_space(4.0);
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
                            .desired_width(200.0)
                            .char_limit(28)
                            .font(egui::TextStyle::Heading),
                    );
                    if ui
                        .button("⟳")
                        .on_hover_text("New look (colours and shape)")
                        .clicked()
                    {
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
                .fill(egui::Color32::from_rgb(18, 20, 26))
                .corner_radius(6.0)
                .inner_margin(8)
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.set_height(180.0);
                    ui.centered_and_justified(|ui| {
                        if let Some((id, size)) = portrait {
                            let scale = (260.0 / size.x).min(160.0 / size.y).clamp(1.0, 8.0).floor();
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
                        .small()
                        .color(egui::Color32::from_gray(200)),
                );
            }
            ui.add_space(8.0);
            let count = lo.count as f32;
            stat(ui, "Health", sp.stats.max_health * count, 400.0);
            stat(ui, "Speed", sp.stats.speed, 60.0);
            let dmg = lo.weapons.iter().map(|w| w.damage / w.cooldown).sum::<f32>() * count
                + lo.spit_damage / lo.spit_cooldown * count;
            stat(ui, "Damage/s", dmg, 80.0);
            stat(ui, "Sight", lo.sight, 150.0);
            stat(ui, "Armour", lo.armor * 100.0, 60.0);
            ui.add_space(8.0);
            ui.weak("Every upgrade shows on the body.");
        });

    egui::Panel::right("versus_players")
        .exact_size(250.0)
        .resizable(false)
        .show(&mut root, |ui| {
            ui.add_space(8.0);
            ui.label(egui::RichText::new("Players").strong());
            ui.add_space(4.0);
            for s in &versus.slots {
                ui.horizontal(|ui| {
                    let me = s.id == versus.local_id;
                    let name = if me { format!("{} (you)", s.name) } else { s.name.clone() };
                    ui.label(egui::RichText::new(name).color(if me { GOLD } else { egui::Color32::WHITE }));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if s.wins > 0 {
                            ui.label(egui::RichText::new(format!("{}★", s.wins)).color(GOLD));
                        }
                        let (text, colour) = if s.spectator {
                            ("watching", DIM)
                        } else if s.ready {
                            ("ready", egui::Color32::from_rgb(140, 230, 160))
                        } else {
                            ("designing", DIM)
                        };
                        ui.label(egui::RichText::new(text).small().color(colour));
                    });
                });
            }
            if versus.slots.len() < 2 {
                ui.add_space(6.0);
                ui.weak("Waiting for an opponent to join.");
            }
            ui.add_space(16.0);
            let label = if ready { "Not ready" } else { "READY" };
            let fill = if ready {
                egui::Color32::from_rgb(90, 90, 100)
            } else {
                egui::Color32::from_rgb(60, 120, 70)
            };
            if ui
                .add_sized([230.0, 44.0], egui::Button::new(egui::RichText::new(label).size(20.0).strong()).fill(fill))
                .clicked()
            {
                if ready {
                    versus.unready();
                } else {
                    versus.set_ready();
                }
            }
            if ready {
                let waiting = versus.waiting_on();
                ui.add_space(4.0);
                if waiting.is_empty() {
                    ui.weak("Starting…");
                } else {
                    ui.weak(format!("Waiting for {}", waiting.join(", ")));
                }
            }
            ui.add_space(8.0);
            ui.add_enabled_ui(!ready, |ui| {
                if ui.add_sized([230.0, 30.0], egui::Button::new("Spend the rest at random")).clicked() {
                    versus.auto_fill();
                }
            });
            ui.add_space(12.0);
            ui.label(egui::RichText::new("How it works").small().strong());
            ui.label(
                egui::RichText::new(
                    "Spend points on your species. Both are dropped into a random arena and fight on their own. First to three round wins. After each round everyone gets more points to evolve; nothing can be taken away.",
                )
                .small()
                .color(egui::Color32::from_gray(180)),
            );
        });

    egui::CentralPanel::default().show(&mut root, |ui| {
        ui.add_enabled_ui(!ready, |ui| {
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.add_space(4.0);
                    for group in Group::ALL {
                        ui.label(
                            egui::RichText::new(group.label().to_uppercase())
                                .small()
                                .strong()
                                .color(GOLD),
                        );
                        ui.add_space(2.0);
                        for u in Upgrade::ALL {
                            if u.group() != group {
                                continue;
                            }
                            upgrade_row(ui, versus, u, left);
                        }
                        ui.add_space(10.0);
                    }
                });
        });
    });
}

fn stat(ui: &mut egui::Ui, label: &str, v: f32, max: f32) {
    ui.horizontal(|ui| {
        ui.add_sized([68.0, 14.0], egui::Label::new(egui::RichText::new(label).small()));
        let (rect, _) = ui.allocate_exact_size(egui::vec2(150.0, 9.0), egui::Sense::hover());
        let p = ui.painter();
        p.rect_filled(rect, 2.0, egui::Color32::from_gray(40));
        let mut fill = rect;
        fill.set_width(rect.width() * (v / max).clamp(0.0, 1.0));
        p.rect_filled(fill, 2.0, GOLD.gamma_multiply(0.8));
        ui.label(egui::RichText::new(format!("{v:.0}")).small().color(DIM));
    });
}

fn upgrade_row(ui: &mut egui::Ui, versus: &mut Versus, u: Upgrade, left: u32) {
    let level = versus.design.level(u);
    let floor = versus.base.as_ref().map_or(0, |b| b.level(u));
    let blocked = u.requires(&versus.design);
    let row_colour = if blocked.is_some() {
        DIM
    } else if level > 0 {
        egui::Color32::WHITE
    } else {
        egui::Color32::from_gray(200)
    };
    ui.horizontal(|ui| {
        let label = ui.add_sized(
            [130.0, 18.0],
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
                    let cost = u.price(lv) as i64 - u.price(level) as i64;
                    let affordable = cost <= left as i64;
                    let allowed = lv >= floor && (affordable || lv == level) && blocked.is_none();
                    let text = if lv == level || cost <= 0 {
                        name.to_string()
                    } else {
                        format!("{name} {cost}")
                    };
                    let selected = lv == level;
                    let resp = ui
                        .add_enabled_ui(allowed, |ui| ui.selectable_label(selected, text))
                        .inner;
                    if resp.clicked() && !selected {
                        versus.design.set(u, lv);
                    }
                }
            }
            None => {
                let can_down = level > floor;
                if ui
                    .add_enabled(can_down, egui::Button::new("−").min_size(egui::vec2(22.0, 18.0)))
                    .clicked()
                {
                    versus.design.set(u, level - 1);
                }
                for i in 0..u.max() {
                    let on = i < level;
                    let locked = i < floor;
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(12.0, 12.0), egui::Sense::hover());
                    let colour = if on && locked {
                        GOLD.gamma_multiply(0.6)
                    } else if on {
                        GOLD
                    } else {
                        egui::Color32::from_gray(50)
                    };
                    ui.painter().circle_filled(rect.center(), 5.0, colour);
                }
                let next_cost = if level < u.max() {
                    u.price(level + 1) - u.price(level)
                } else {
                    0
                };
                let can_up = level < u.max() && next_cost <= left && blocked.is_none();
                let plus = if level < u.max() {
                    format!("+ {next_cost}")
                } else {
                    "max".into()
                };
                if ui
                    .add_enabled(can_up, egui::Button::new(plus).min_size(egui::vec2(44.0, 18.0)))
                    .clicked()
                {
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

    // Letterbox and flash for the dramatic bits.
    let slow = mirror.time_scale < 0.9 || mirror.replaying;
    if slow {
        let h = screen.height() * 0.09;
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
    // Speed lines towards the focus point.
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

    // Team cards.
    let n = versus.teams.len().max(1);
    for (i, (pid, player, design)) in versus.teams.iter().enumerate() {
        let colour = c32(mirror.team_colour_of(i));
        let health = mirror.team_health(i);
        let alive = mirror.alive(i);
        let total = mirror.teams.get(i).map_or(1, |t| t.loadout.count);
        let anchor = if n == 2 {
            if i == 0 {
                egui::Align2::LEFT_TOP
            } else {
                egui::Align2::RIGHT_TOP
            }
        } else {
            egui::Align2::LEFT_TOP
        };
        let offset = if n == 2 {
            [if i == 0 { 14.0 } else { -14.0 }, 14.0]
        } else {
            [14.0, 14.0 + i as f32 * 64.0]
        };
        egui::Area::new(egui::Id::new(("team_card", i)))
            .anchor(anchor, offset)
            .interactable(false)
            .show(ctx, |ui| {
                let right = n == 2 && i == 1;
                let layout = if right {
                    egui::Layout::top_down(egui::Align::RIGHT)
                } else {
                    egui::Layout::top_down(egui::Align::LEFT)
                };
                ui.with_layout(layout, |ui| {
                    let me = *pid == versus.local_id;
                    ui.label(
                        egui::RichText::new(format!("{}{}", design.name, if me { "  (you)" } else { "" }))
                            .size(18.0)
                            .strong()
                            .color(colour),
                    );
                    ui.label(egui::RichText::new(player).small().color(DIM));
                    let (rect, _) = ui.allocate_exact_size(egui::vec2(220.0, 10.0), egui::Sense::hover());
                    let p = ui.painter();
                    p.rect_filled(rect, 3.0, egui::Color32::from_black_alpha(160));
                    let mut fill = rect;
                    let w = rect.width() * health.clamp(0.0, 1.0);
                    if right {
                        fill.min.x = rect.max.x - w;
                    } else {
                        fill.set_width(w);
                    }
                    p.rect_filled(fill, 3.0, colour);
                    ui.horizontal(|ui| {
                        for k in 0..total {
                            let (r, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                            let on = k < alive;
                            ui.painter().circle_filled(
                                r.center(),
                                4.0,
                                if on { colour } else { egui::Color32::from_gray(60) },
                            );
                        }
                    });
                });
            });
    }

    // Clock and round.
    egui::Area::new(egui::Id::new("versus_clock"))
        .anchor(egui::Align2::CENTER_TOP, [0.0, 10.0])
        .interactable(false)
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new(format!("ROUND {}", versus.round))
                        .small()
                        .strong()
                        .color(GOLD),
                );
                let left = (ROUND_TIME - mirror.clock).max(0.0);
                let text = if mirror.replaying {
                    "INSTANT REPLAY".to_string()
                } else if mirror.frenzy {
                    "FRENZY".to_string()
                } else {
                    format!("{:.0}", left)
                };
                let colour = if mirror.frenzy || mirror.replaying {
                    egui::Color32::from_rgb(255, 200, 60)
                } else {
                    egui::Color32::WHITE
                };
                ui.label(egui::RichText::new(text).size(22.0).strong().color(colour));
                let mut names: Vec<String> = Vec::new();
                for (i, c) in mirror.creatures.iter().enumerate() {
                    if c.alive()
                        && let Some(t) = mirror.tactics.get(i)
                    {
                        let team = mirror.teams.get(c.species).map_or("", |t| t.design.name.as_str());
                        if !names.iter().any(|n| n.starts_with(team)) && *t != Tactic::Search {
                            names.push(format!("{team}: {}", t.label()));
                        }
                    }
                }
            });
        });

    // Callouts.
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
                    egui::FontId::proportional(15.0),
                    colour,
                );
            }
            None => {
                let pop = 1.0 + (1.0 - t).powi(3) * 0.4;
                let size = 46.0 * pop;
                let pos = egui::pos2(screen.center().x, screen.center().y - screen.height() * 0.22);
                painter.text(
                    pos + egui::vec2(3.0, 3.0),
                    egui::Align2::CENTER_CENTER,
                    &c.text,
                    egui::FontId::proportional(size),
                    egui::Color32::from_black_alpha((alpha * 200.0) as u8),
                );
                painter.text(
                    pos,
                    egui::Align2::CENTER_CENTER,
                    &c.text,
                    egui::FontId::proportional(size),
                    colour,
                );
            }
        }
    }
    // What each creature is doing, floating above it.
    let font = egui::FontId::proportional(11.0);
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
        painter.text(
            egui::pos2(p.x, p.y),
            egui::Align2::CENTER_BOTTOM,
            t.label(),
            font.clone(),
            colour,
        );
    }
    if mirror.replaying {
        egui::Area::new(egui::Id::new("versus_skip"))
            .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -screen.height() * 0.09 - 8.0])
            .interactable(false)
            .show(ctx, |ui| {
                ui.label(egui::RichText::new("click or Space to skip").small().color(DIM));
            });
    }
}

// ---------------------------------------------------------------------------
// Results

fn results_ui(
    ctx: &egui::Context,
    versus: &mut Versus,
    portraits: &[(u64, Option<(egui::TextureId, egui::Vec2)>)],
) {
    let Some(result) = versus.result.clone() else {
        return;
    };
    let teams = versus.teams.clone();
    egui::Window::new("versus_results")
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .frame(
            egui::Frame::window(&ctx.global_style())
                .fill(egui::Color32::from_rgba_unmultiplied(12, 12, 18, 235))
                .inner_margin(18),
        )
        .show(ctx, |ui| {
            ui.set_width(520.0);
            ui.vertical_centered(|ui| {
                let headline = match result.winner.and_then(|w| teams.get(w as usize)) {
                    Some((pid, _, d)) => {
                        let who = if *pid == versus.local_id {
                            "You win the round"
                        } else {
                            "Round lost"
                        };
                        format!("{who}: {} prevails", d.name)
                    }
                    None => "Draw".to_string(),
                };
                let colour = result
                    .winner
                    .and_then(|w| versus.mirror.as_ref().map(|m| c32(m.team_colour_of(w as usize))))
                    .unwrap_or(GOLD);
                ui.label(egui::RichText::new(headline).size(24.0).strong().color(colour));
                ui.label(
                    egui::RichText::new(format!("Round {} · {:.0}s", versus.round, result.duration))
                        .color(DIM),
                );
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    let w = ui.available_width() / teams.len().max(1) as f32;
                    for (i, (pid, player, design)) in teams.iter().enumerate() {
                        ui.allocate_ui(egui::vec2(w, 160.0), |ui| {
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
                                ui.label(egui::RichText::new(&design.name).strong().color(tc));
                                ui.label(egui::RichText::new(player).small().color(DIM));
                                let wins = versus.wins.get(i).copied().unwrap_or(0);
                                ui.label(egui::RichText::new("★".repeat(wins as usize)).color(GOLD));
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
                        format!("{name} wins the match")
                    };
                    ui.label(egui::RichText::new(text).size(20.0).strong().color(GOLD));
                    ui.add_space(6.0);
                    let label = if versus.is_authority() {
                        "Rematch"
                    } else {
                        "Rematch (host decides)"
                    };
                    if versus.is_authority()
                        && ui
                            .add_sized(
                                [220.0, 40.0],
                                egui::Button::new(egui::RichText::new(label).size(18.0)),
                            )
                            .clicked()
                    {
                        versus.leave_results();
                    }
                    if !versus.is_authority() {
                        ui.weak(label);
                    }
                } else if ui
                    .add_sized(
                        [240.0, 40.0],
                        egui::Button::new(egui::RichText::new("Evolve").size(18.0).strong())
                            .fill(egui::Color32::from_rgb(60, 120, 70)),
                    )
                    .clicked()
                {
                    versus.leave_results();
                }
                ui.add_space(4.0);
                ui.weak("Watch the arena behind this panel as long as you like.");
            });
        });
}
