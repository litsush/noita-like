//! The alien ecosystem sandbox: a generated biome with a predator and a
//! prey species living on the falling-sand world, viewed from outside.
//!
//! Controls: drag with right or middle mouse (or WASD) to pan, wheel to zoom,
//! click a creature to inspect it, Space to pause, 1-4 for speed, N for a new
//! scene, F to follow the selected creature, L for behaviour labels.

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::view::window::screenshot::{Screenshot, save_to_disk};
use bevy::window::PrimaryWindow;
use bevy_egui::{EguiContexts, EguiTextureHandle, egui};
use sbct_sim::eco::biome::{BiomeKind, HEIGHT, WIDTH};
use sbct_sim::eco::brain::BehaviorKind;
use sbct_sim::eco::genome::{Role, Species};
use sbct_sim::eco::math::v2;
use sbct_sim::eco::render::{DrawOpts, Renderer, portrait};
use sbct_sim::eco::{DT, Ecosystem};

use crate::AppState;
use crate::render::WorldCamera;

const SPEEDS: [f32; 4] = [1.0, 2.0, 4.0, 8.0];
const MAX_STEPS_PER_FRAME: u32 = 40;

#[derive(Component)]
pub struct EcoEntity;

#[derive(Resource)]
pub struct EcoScene {
    pub eco: Ecosystem,
    renderer: Renderer,
    image: Handle<Image>,
    portraits: Vec<(Handle<Image>, egui::Vec2, Option<egui::TextureId>)>,
    paused: bool,
    speed: usize,
    selected: Option<u32>,
    follow: bool,
    labels: bool,
    paths: bool,
    seed_text: String,
    biome_choice: Option<BiomeKind>,
    acc: f32,
    /// Screen pixels per cell.
    zoom: f32,
    /// Camera centre in cell coordinates.
    cam: Vec2,
    drag: Option<(Vec2, Vec2)>,
    sim_ms: f32,
    keyboard_busy: bool,
    pointer_busy: bool,
    show_log: bool,
    /// Species window tab: predator, prey, plants.
    tab: usize,
}

/// Dev options from the command line: `--ecosystem`, `--seed <n>`,
/// `--biome <name>`, `--speed <1-4>`, `--select`, `--screenshot <path>`,
/// `--after <secs>`, `--quit-after <secs>`.
#[derive(Resource, Default)]
pub struct EcoDevArgs {
    pub seed: Option<u64>,
    pub biome: Option<BiomeKind>,
    pub speed: Option<usize>,
    pub select: bool,
    pub select_prey: bool,
    pub screenshot: Option<String>,
    pub after: f32,
    pub quit_after: Option<f32>,
    pub zoom: Option<f32>,
    taken: bool,
}

impl EcoDevArgs {
    pub fn from_args() -> EcoDevArgs {
        let args: Vec<String> = std::env::args().skip(1).collect();
        let value = |flag: &str| {
            let i = args.iter().position(|a| a == flag)?;
            args.get(i + 1).filter(|v| !v.starts_with("--")).cloned()
        };
        let biome = value("--biome").and_then(|b| {
            let b = b.to_lowercase();
            BiomeKind::ALL.into_iter().find(|k| {
                k.title()
                    .to_lowercase()
                    .replace(' ', "")
                    .contains(&b.replace([' ', '-', '_'], ""))
            })
        });

        EcoDevArgs {
            seed: value("--seed").and_then(|s| s.parse().ok()),
            biome,
            speed: value("--speed")
                .and_then(|s| s.parse::<usize>().ok())
                .map(|s| s.clamp(1, 4) - 1),
            select: args.iter().any(|a| a == "--select"),
            select_prey: value("--select").is_some_and(|v| v == "prey"),
            screenshot: value("--screenshot"),
            after: value("--after").and_then(|s| s.parse().ok()).unwrap_or(5.0),
            quit_after: value("--quit-after").and_then(|s| s.parse().ok()),
            zoom: value("--zoom").and_then(|s| s.parse().ok()),
            taken: false,
        }
    }
}

fn random_seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64);
    sbct_sim::rng::hash2(nanos, 7, 3) % 1_000_000
}

fn new_image(images: &mut Assets<Image>, w: usize, h: usize, data: Vec<u8>) -> Handle<Image> {
    let mut image = Image::new(
        Extent3d {
            width: w as u32,
            height: h as u32,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    image.sampler = ImageSampler::nearest();
    images.add(image)
}

fn make_portraits(
    eco: &Ecosystem,
    images: &mut Assets<Image>,
) -> Vec<(Handle<Image>, egui::Vec2, Option<egui::TextureId>)> {
    eco.species
        .iter()
        .map(|sp| {
            let (w, h, data) = portrait(sp);
            let handle = new_image(images, w, h, data);
            (handle, egui::vec2(w as f32, h as f32), None)
        })
        .collect()
}

pub fn enter_ecosystem(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    dev: Option<Res<EcoDevArgs>>,
    mut clear: ResMut<ClearColor>,
) {
    let seed = dev.as_ref().and_then(|d| d.seed).unwrap_or_else(random_seed);
    let biome = dev.as_ref().and_then(|d| d.biome);
    let eco = Ecosystem::with_biome(seed, biome);
    let image = new_image(&mut images, WIDTH, HEIGHT, vec![0; WIDTH * HEIGHT * 4]);
    let portraits = make_portraits(&eco, &mut images);
    commands.spawn((
        EcoEntity,
        Sprite {
            image: image.clone(),
            custom_size: Some(Vec2::new(WIDTH as f32, HEIGHT as f32)),
            ..default()
        },
        Transform::from_xyz(WIDTH as f32 / 2.0, -(HEIGHT as f32) / 2.0, 0.0),
    ));
    clear.0 = Color::srgb(0.03, 0.03, 0.05);
    let selected = if dev.as_ref().is_some_and(|d| d.select) {
        let want = if dev.as_ref().is_some_and(|d| d.select_prey) {
            0
        } else {
            1
        };
        eco.creatures.iter().find(|c| c.species == want).map(|c| c.id)
    } else {
        None
    };
    commands.insert_resource(EcoScene {
        renderer: Renderer::new(WIDTH, HEIGHT),
        image,
        portraits,
        paused: false,
        speed: dev.as_ref().and_then(|d| d.speed).unwrap_or(0),
        selected,
        follow: selected.is_some() && dev.as_ref().is_some_and(|d| d.zoom.is_some()),
        labels: true,
        paths: false,
        seed_text: seed.to_string(),
        biome_choice: biome,
        acc: 0.0,
        zoom: dev.as_ref().and_then(|d| d.zoom).unwrap_or(2.4),
        cam: Vec2::new(WIDTH as f32 / 2.0, HEIGHT as f32 / 2.0),
        drag: None,
        sim_ms: 0.0,
        keyboard_busy: false,
        pointer_busy: false,
        show_log: true,
        tab: 0,
        eco,
    });
}

pub fn exit_ecosystem(
    mut commands: Commands,
    entities: Query<Entity, With<EcoEntity>>,
    mut clear: ResMut<ClearColor>,
    mut camera: Single<(&mut Transform, &mut Projection), With<WorldCamera>>,
) {
    for e in &entities {
        commands.entity(e).despawn();
    }
    commands.remove_resource::<EcoScene>();
    clear.0 = Color::srgb(0.45, 0.62, 0.85);
    let (ref mut tf, ref mut proj) = *camera;
    if let Projection::Orthographic(o) = &mut **proj {
        o.scale = 1.0 / crate::render::PIXEL_SCALE;
    }
    tf.translation = Vec3::ZERO;
}

fn regenerate(scene: &mut EcoScene, images: &mut Assets<Image>, seed: u64) {
    scene.eco = Ecosystem::with_biome(seed, scene.biome_choice);
    scene.portraits = make_portraits(&scene.eco, images);
    scene.seed_text = seed.to_string();
    scene.selected = None;
    scene.follow = false;
    scene.acc = 0.0;
}

fn cursor_cell(window: &Window, camera: &Camera, cam_tf: &GlobalTransform) -> Option<Vec2> {
    let pos = window.cursor_position()?;
    let w = camera.viewport_to_world_2d(cam_tf, pos).ok()?;
    Some(Vec2::new(w.x, -w.y))
}

pub fn eco_input(
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<MouseButton>>,
    mut wheel: MessageReader<MouseWheel>,
    time: Res<Time>,
    window: Single<&Window, With<PrimaryWindow>>,
    camera: Single<(&Camera, &GlobalTransform), With<WorldCamera>>,
    mut scene: ResMut<EcoScene>,
    mut images: ResMut<Assets<Image>>,
    mut commands: Commands,
) {
    let (cam, cam_tf) = *camera;
    let cursor = window.cursor_position();
    let cell = cursor_cell(&window, cam, cam_tf);

    if !scene.keyboard_busy {
        if keys.just_pressed(KeyCode::Space) {
            scene.paused = !scene.paused;
        }
        for (i, k) in [KeyCode::Digit1, KeyCode::Digit2, KeyCode::Digit3, KeyCode::Digit4]
            .into_iter()
            .enumerate()
        {
            if keys.just_pressed(k) {
                scene.speed = i;
                scene.paused = false;
            }
        }
        if keys.just_pressed(KeyCode::KeyN) {
            regenerate(&mut scene, &mut images, random_seed());
        }
        if keys.just_pressed(KeyCode::KeyF) {
            scene.follow = !scene.follow && scene.selected.is_some();
        }
        if keys.just_pressed(KeyCode::KeyL) {
            scene.labels = !scene.labels;
        }
        if keys.just_pressed(KeyCode::KeyP) {
            scene.paths = !scene.paths;
        }
        if keys.just_pressed(KeyCode::Escape) {
            if scene.selected.is_some() {
                scene.selected = None;
                scene.follow = false;
            } else {
                commands.set_state(AppState::Menu);
            }
        }
        let mut pan = Vec2::ZERO;
        if keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft) {
            pan.x -= 1.0;
        }
        if keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight) {
            pan.x += 1.0;
        }
        if keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp) {
            pan.y -= 1.0;
        }
        if keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown) {
            pan.y += 1.0;
        }
        if pan != Vec2::ZERO {
            let z = scene.zoom;
            scene.cam += pan * 300.0 / z * time.delta_secs();
            scene.follow = false;
        }
    }

    if !scene.pointer_busy {
        for ev in wheel.read() {
            let old = scene.zoom;
            let factor = if ev.y > 0.0 { 1.15 } else { 1.0 / 1.15 };
            scene.zoom = (scene.zoom * factor).clamp(1.0, 14.0);
            // Zoom around the cursor.
            if let Some(c) = cell
                && !scene.follow
            {
                let cam_pos = scene.cam;
                scene.cam = c + (cam_pos - c) * (old / scene.zoom);
            }
        }
        let dragging = mouse.pressed(MouseButton::Right) || mouse.pressed(MouseButton::Middle);
        if let Some(cur) = cursor {
            if dragging {
                match scene.drag {
                    None => scene.drag = Some((cur, scene.cam)),
                    Some((start, cam0)) => {
                        let z = scene.zoom;
                        scene.cam = cam0 - (cur - start) / z;
                        scene.follow = false;
                    }
                }
            } else {
                scene.drag = None;
            }
        }
        if mouse.just_pressed(MouseButton::Left)
            && let Some(c) = cell
        {
            let picked = scene.eco.creature_at(v2(c.x, c.y));
            scene.selected = picked;
            if picked.is_none() {
                scene.follow = false;
            }
        }
    } else {
        wheel.clear();
    }
}

pub fn eco_step(time: Res<Time>, mut scene: ResMut<EcoScene>) {
    if scene.paused {
        return;
    }
    scene.acc += time.delta_secs().min(0.1) * SPEEDS[scene.speed];
    let start = std::time::Instant::now();
    let mut n = 0;
    while scene.acc >= DT && n < MAX_STEPS_PER_FRAME {
        scene.eco.step();
        scene.acc -= DT;
        n += 1;
    }
    if n == MAX_STEPS_PER_FRAME {
        scene.acc = 0.0;
    }
    if n > 0 {
        let ms = start.elapsed().as_secs_f32() * 1000.0 / n as f32;
        scene.sim_ms = scene.sim_ms * 0.9 + ms * 0.1;
    }
    if let Some(id) = scene.selected
        && scene.eco.creature(id).is_none()
    {
        scene.selected = None;
        scene.follow = false;
    }
}

pub fn eco_render(mut scene: ResMut<EcoScene>, mut images: ResMut<Assets<Image>>) {
    let scene = &mut *scene;
    let opts = DrawOpts {
        selected: scene.selected,
        show_paths: scene.paths,
    };
    scene.renderer.draw(&scene.eco, &opts);
    let Some(mut image) = images.get_mut(&scene.image) else {
        return;
    };
    if let Some(data) = image.data.as_mut() {
        data.copy_from_slice(&scene.renderer.rgba);
    }
}

pub fn eco_camera(
    mut scene: ResMut<EcoScene>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut camera: Single<(&mut Transform, &mut Projection), With<WorldCamera>>,
) {
    if scene.follow
        && let Some(c) = scene.selected.and_then(|id| scene.eco.creature(id))
    {
        let target = Vec2::new(c.pos.x, c.pos.y);
        scene.cam = scene.cam.lerp(target, 0.15);
    }
    let half = Vec2::new(window.width(), window.height()) / scene.zoom / 2.0;
    let (w, h) = (WIDTH as f32, HEIGHT as f32);
    let clamp = |v: f32, half: f32, size: f32| {
        if half * 2.0 >= size {
            size / 2.0
        } else {
            v.clamp(half, size - half)
        }
    };
    scene.cam.x = clamp(scene.cam.x, half.x, w);
    scene.cam.y = clamp(scene.cam.y, half.y, h);
    let (ref mut tf, ref mut proj) = *camera;
    if let Projection::Orthographic(o) = &mut **proj {
        o.scale = 1.0 / scene.zoom;
    }
    tf.translation.x = scene.cam.x;
    tf.translation.y = -scene.cam.y;
}

/// Screenshots and timed exit for unattended testing.
pub fn eco_dev(
    mut commands: Commands,
    time: Res<Time<Real>>,
    mut dev: ResMut<EcoDevArgs>,
    mut exit: MessageWriter<AppExit>,
) {
    let t = time.elapsed_secs();
    if let Some(path) = dev.screenshot.clone()
        && !dev.taken
        && t > dev.after
    {
        dev.taken = true;
        commands
            .spawn(Screenshot::primary_window())
            .observe(save_to_disk(path));
    }
    if dev.quit_after.is_some_and(|q| t > q) {
        exit.write(AppExit::Success);
    }
}

// ---------------------------------------------------------------------------
// UI

fn c32(c: [u8; 3]) -> egui::Color32 {
    egui::Color32::from_rgb(c[0], c[1], c[2])
}

const PREY_COLOUR: egui::Color32 = egui::Color32::from_rgb(120, 220, 140);
const PRED_COLOUR: egui::Color32 = egui::Color32::from_rgb(240, 110, 90);
const PLANT_COLOUR: egui::Color32 = egui::Color32::from_rgb(150, 160, 255);

fn role_colour(r: Role) -> egui::Color32 {
    match r {
        Role::Prey => PREY_COLOUR,
        Role::Predator => PRED_COLOUR,
    }
}

fn bar(ui: &mut egui::Ui, label: &str, v: f32, colour: egui::Color32) {
    ui.horizontal(|ui| {
        ui.add_sized([62.0, 14.0], egui::Label::new(egui::RichText::new(label).small()));
        let (rect, _) = ui.allocate_exact_size(egui::vec2(150.0, 10.0), egui::Sense::hover());
        let p = ui.painter();
        p.rect_filled(rect, 2.0, egui::Color32::from_gray(40));
        let mut fill = rect;
        fill.set_width(rect.width() * v.clamp(0.0, 1.0));
        p.rect_filled(fill, 2.0, colour);
        ui.label(egui::RichText::new(format!("{:.0}%", v * 100.0)).small());
    });
}

fn chips(ui: &mut egui::Ui, items: &[String], colour: egui::Color32) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(4.0, 3.0);
        for it in items {
            let text = egui::RichText::new(format!(" {it} "))
                .small()
                .color(egui::Color32::from_gray(235))
                .background_color(colour.gamma_multiply(0.3));
            ui.add(egui::Label::new(text).wrap_mode(egui::TextWrapMode::Extend));
        }
    });
}

fn species_card(
    ui: &mut egui::Ui,
    sp: &Species,
    tex: Option<(egui::TextureId, egui::Vec2)>,
    population: usize,
    births: u32,
    deaths: u32,
) {
    let colour = role_colour(sp.role);
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(&sp.common).size(17.0).strong().color(colour));
    });
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(&sp.name).italics().weak());
        ui.label(
            egui::RichText::new(match sp.role {
                Role::Prey => "· prey",
                Role::Predator => "· predator",
            })
            .color(colour),
        );
    });
    if let Some((id, size)) = tex {
        let scale = (270.0 / size.x).min(120.0 / size.y).clamp(1.0, 8.0).floor();
        egui::Frame::new()
            .fill(egui::Color32::from_rgb(22, 24, 30))
            .corner_radius(4.0)
            .inner_margin(4)
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.vertical_centered(|ui| {
                    ui.add(
                        egui::Image::new((id, size * scale)).texture_options(egui::TextureOptions::NEAREST),
                    );
                });
            });
    }
    ui.label(format!("Alive: {population}   born: {births}   died: {deaths}"));
    for line in &sp.description {
        ui.label(egui::RichText::new(line).small());
    }
    let mut facts = vec![
        sp.habitat.label().to_string(),
        sp.loco.label().to_string(),
        sp.activity.label().to_string(),
        format!("size {:.1}", sp.stats.size),
        format!("speed {:.0}", sp.stats.speed),
    ];
    facts.extend(sp.ability_list());
    chips(ui, &facts, colour);
    ui.add_space(2.0);
    ui.label(egui::RichText::new("Behaviour modules").small().strong());
    let mods: Vec<String> = sp
        .behaviors
        .iter()
        .filter(|(k, _)| *k != BehaviorKind::Escape)
        .map(|(k, w)| format!("{} ×{:.1}", k.label(), w))
        .collect();
    chips(ui, &mods, egui::Color32::from_rgb(170, 170, 210));
}

fn population_graph(ui: &mut egui::Ui, eco: &Ecosystem) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 90.0), egui::Sense::hover());
    let p = ui.painter_at(rect);
    p.rect_filled(rect, 3.0, egui::Color32::from_rgb(18, 20, 26));
    let hist = &eco.history;
    if hist.len() < 2 {
        p.text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            "collecting…",
            egui::FontId::proportional(12.0),
            egui::Color32::GRAY,
        );
        return;
    }
    let max_creatures = hist.iter().map(|h| h[0].max(h[1])).max().unwrap_or(1).max(4) as f32;
    let max_plants = hist.iter().map(|h| h[2]).max().unwrap_or(1).max(4) as f32;
    let n = hist.len();
    let line = |k: usize, max: f32, colour: egui::Color32| {
        let pts: Vec<egui::Pos2> = hist
            .iter()
            .enumerate()
            .map(|(i, h)| {
                let x = rect.left() + i as f32 / (n - 1) as f32 * rect.width();
                let y = rect.bottom() - 4.0 - h[k] as f32 / max * (rect.height() - 8.0);
                egui::pos2(x, y)
            })
            .collect();
        p.add(egui::Shape::line(pts, egui::Stroke::new(1.5, colour)));
    };
    line(2, max_plants, PLANT_COLOUR.gamma_multiply(0.6));
    line(0, max_creatures, PREY_COLOUR);
    line(1, max_creatures, PRED_COLOUR);
}

pub fn eco_ui(
    mut contexts: EguiContexts,
    mut scene: ResMut<EcoScene>,
    mut images: ResMut<Assets<Image>>,
    mut commands: Commands,
    camera: Single<(&Camera, &GlobalTransform), With<WorldCamera>>,
) -> Result {
    // Register portrait textures with egui once.
    for i in 0..scene.portraits.len() {
        if scene.portraits[i].2.is_none() {
            let id = contexts.add_image(EguiTextureHandle::Strong(scene.portraits[i].0.clone()));
            scene.portraits[i].2 = Some(id);
        }
    }
    let ctx = contexts.ctx_mut()?;
    let scene = &mut *scene;
    let mut regen: Option<u64> = None;

    // Behaviour labels above creatures, skipping any that would overlap.
    if scene.labels {
        let (cam, cam_tf) = *camera;
        let painter = ctx.layer_painter(egui::LayerId::background());
        let font = egui::FontId::proportional(11.0);
        let mut placed: Vec<egui::Rect> = Vec::new();
        let mut order: Vec<&sbct_sim::eco::creature::Creature> = scene.eco.creatures.iter().collect();
        // The selected creature always gets its label.
        order.sort_by_key(|c| Some(c.id) != scene.selected);
        for c in order {
            let sp = &scene.eco.species[c.species];
            let top = c.pos - v2(0.0, sp.body.top * c.scale(sp) + 4.0);
            let Ok(p) = cam.world_to_viewport(cam_tf, Vec3::new(top.x, -top.y, 0.0)) else {
                continue;
            };
            let selected = Some(c.id) == scene.selected;
            let colour = role_colour(sp.role).gamma_multiply(if selected { 1.0 } else { 0.8 });
            let galley = painter.layout_no_wrap(c.brain.label.to_string(), font.clone(), colour);
            let rect = egui::Align2::CENTER_BOTTOM
                .anchor_size(egui::pos2(p.x, p.y), galley.size())
                .expand(2.0);
            if !selected && placed.iter().any(|r| r.intersects(rect)) {
                continue;
            }
            placed.push(rect);
            painter.rect_filled(rect, 3.0, egui::Color32::from_black_alpha(110));
            painter.galley(rect.min + egui::vec2(2.0, 2.0), galley, colour);
        }
    }

    let screen_w = ctx.content_rect().width();
    egui::Window::new("eco_top")
        .title_bar(false)
        .resizable(false)
        .anchor(egui::Align2::LEFT_TOP, [8.0, 8.0])
        .show(ctx, |ui| {
            ui.set_width(screen_w - 30.0);
            ui.horizontal_wrapped(|ui| {
                ui.label(
                    egui::RichText::new("Alien Ecosystem")
                        .strong()
                        .color(egui::Color32::from_rgb(230, 170, 70)),
                );
                ui.separator();
                ui.label(egui::RichText::new(&scene.eco.biome.name).strong());
                ui.label(format!(
                    "Day {} · {} · {} · {}",
                    scene.eco.day_number(),
                    scene.eco.time_label(),
                    scene.eco.weather.kind.label(),
                    scene.eco.biome.describe_gravity()
                ));
                ui.separator();
                let pause_label = if scene.paused { "▶ Play" } else { "⏸ Pause" };
                if ui.button(pause_label).clicked() {
                    scene.paused = !scene.paused;
                }
                for (i, s) in SPEEDS.iter().enumerate() {
                    if ui
                        .selectable_label(scene.speed == i && !scene.paused, format!("{s}×"))
                        .clicked()
                    {
                        scene.speed = i;
                        scene.paused = false;
                    }
                }
                ui.separator();
                ui.label("Seed");
                ui.add(egui::TextEdit::singleline(&mut scene.seed_text).desired_width(70.0));
                egui::ComboBox::from_id_salt("biome")
                    .selected_text(scene.biome_choice.map_or("Any biome", |b| b.title()))
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut scene.biome_choice, None, "Any biome");
                        for b in BiomeKind::ALL {
                            ui.selectable_value(&mut scene.biome_choice, Some(b), b.title());
                        }
                    });
                if ui.button("Load seed").clicked() {
                    regen = Some(scene.seed_text.trim().parse().unwrap_or_else(|_| {
                        sbct_sim::rng::hash2(0, scene.seed_text.len() as i32, 1) % 1_000_000
                    }));
                }
                if ui
                    .add(
                        egui::Button::new(egui::RichText::new("⟳ New Scene").strong())
                            .fill(egui::Color32::from_rgb(70, 110, 60)),
                    )
                    .clicked()
                {
                    regen = Some(random_seed());
                }
                ui.separator();
                ui.checkbox(&mut scene.labels, "Labels");
                ui.checkbox(&mut scene.paths, "Paths");
                ui.checkbox(&mut scene.show_log, "Log");
                ui.checkbox(&mut scene.eco.migrants, "Migrants");
                ui.separator();
                if ui.button("Main menu").clicked() {
                    commands.set_state(AppState::Menu);
                }
            });
        });

    let screen_h = ctx.content_rect().height();
    egui::Window::new("Species")
        .anchor(egui::Align2::LEFT_TOP, [8.0, 56.0])
        .default_width(300.0)
        .max_height(screen_h - 260.0)
        .collapsible(true)
        .resizable(false)
        .show(ctx, |ui| {
            ui.set_width(300.0);
            ui.label(egui::RichText::new(scene.eco.biome.kind.blurb()).italics());
            ui.horizontal(|ui| {
                ui.selectable_value(
                    &mut scene.tab,
                    0,
                    egui::RichText::new("Predator").color(PRED_COLOUR),
                );
                ui.selectable_value(&mut scene.tab, 1, egui::RichText::new("Prey").color(PREY_COLOUR));
                ui.selectable_value(
                    &mut scene.tab,
                    2,
                    egui::RichText::new("Plants").color(PLANT_COLOUR),
                );
            });
            ui.separator();
            egui::ScrollArea::vertical().show(ui, |ui| {
                if scene.tab < 2 {
                    let i = 1 - scene.tab;
                    let sp = &scene.eco.species[i];
                    let tex = scene
                        .portraits
                        .get(i)
                        .and_then(|(_, size, id)| id.map(|id| (id, *size)));
                    let pop = scene.eco.population(sp.role);
                    let r = sp.role as usize;
                    species_card(ui, sp, tex, pop, scene.eco.births[r], scene.eco.deaths[r]);
                    return;
                }
                for (fi, f) in scene.eco.flora.iter().enumerate() {
                    let n = scene.eco.plants.iter().filter(|p| p.flora == fi).count();
                    ui.horizontal(|ui| {
                        let (r, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                        ui.painter().rect_filled(r, 2.0, c32(f.leaf));
                        let (r, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
                        ui.painter()
                            .circle_filled(r.center(), 4.0, c32(f.glow.unwrap_or(f.fruit)));
                        ui.label(format!("{} ({n})", f.name));
                    });
                    ui.label(
                        egui::RichText::new(format!(
                            "   {}{}",
                            f.substrate.label(),
                            if f.glow.is_some() { ", glows" } else { "" }
                        ))
                        .small()
                        .weak(),
                    );
                }
            });
        });

    egui::Window::new("Population")
        .anchor(egui::Align2::RIGHT_BOTTOM, [-8.0, -8.0])
        .default_width(300.0)
        .resizable(false)
        .collapsible(true)
        .show(ctx, |ui| {
            ui.set_width(300.0);
            let prey = scene.eco.population(Role::Prey);
            let pred = scene.eco.population(Role::Predator);
            ui.horizontal(|ui| {
                ui.colored_label(PREY_COLOUR, format!("prey {prey}"));
                ui.colored_label(PRED_COLOUR, format!("predators {pred}"));
                ui.colored_label(PLANT_COLOUR, format!("plants {}", scene.eco.plants.len()));
                ui.weak(format!("eggs {}", scene.eco.eggs.len()));
            });
            population_graph(ui, &scene.eco);
            let mut causes: Vec<String> = scene
                .eco
                .causes
                .iter()
                .map(|(s, k, n)| format!("{n} {} {k}", if *s == 0 { "prey" } else { "predators" }))
                .collect();
            causes.sort();
            if !causes.is_empty() {
                ui.label(egui::RichText::new(causes.join(" · ")).small().weak());
            }
            ui.label(
                egui::RichText::new(format!("sim {:.2} ms/step", scene.sim_ms))
                    .small()
                    .weak(),
            );
        });

    if scene.show_log {
        egui::Window::new("Field notes")
            .anchor(egui::Align2::LEFT_BOTTOM, [316.0, -8.0])
            .default_width(360.0)
            .default_height(140.0)
            .collapsible(true)
            .show(ctx, |ui| {
                egui::ScrollArea::vertical()
                    .stick_to_bottom(true)
                    .max_height(140.0)
                    .show(ui, |ui| {
                        for e in &scene.eco.log {
                            let day = (e.time / scene.eco.biome.day_length) as u32 + 1;
                            ui.label(egui::RichText::new(format!("[day {day}] {}", e.text)).small());
                        }
                    });
            });
    }

    if let Some(id) = scene.selected
        && let Some(c) = scene.eco.creature(id)
    {
        let sp = &scene.eco.species[c.species];
        let mut open = true;
        let mut follow = scene.follow;
        egui::Window::new(format!("{} #{}", sp.common, c.id))
            .id(egui::Id::new("inspector"))
            .open(&mut open)
            .anchor(egui::Align2::RIGHT_TOP, [-8.0, 40.0])
            .default_width(290.0)
            .show(ctx, |ui| {
                let stage = if c.mature(sp) { "adult" } else { "juvenile" };
                ui.label(format!(
                    "{stage}, generation {}, age {:.0}s, {} kills",
                    c.generation, c.age, c.kills
                ));
                ui.label(
                    egui::RichText::new(format!("Now: {}", c.brain.label))
                        .strong()
                        .color(role_colour(sp.role)),
                );
                let maxh = c.max_health(sp);
                bar(
                    ui,
                    "health",
                    c.health / maxh,
                    egui::Color32::from_rgb(220, 80, 80),
                );
                bar(ui, "satiety", c.satiety, egui::Color32::from_rgb(220, 180, 80));
                bar(ui, "energy", c.energy, egui::Color32::from_rgb(90, 160, 230));
                bar(ui, "fear", c.fear, egui::Color32::from_rgb(200, 120, 220));
                if c.breath < 0.99 {
                    bar(ui, "breath", c.breath, egui::Color32::from_rgb(120, 220, 230));
                }
                let mut state = Vec::new();
                for (on, s) in [
                    (c.flying, "flying"),
                    (c.clinging, "clinging"),
                    (c.burrowed, "burrowed"),
                    (c.in_liquid, "in liquid"),
                    (c.on_ground, "on ground"),
                    (c.sleeping, "asleep"),
                    (c.sheltered, "sheltered"),
                    (c.webbed > 0.0, "entangled"),
                    (c.status.poison > 0.0, "poisoned"),
                    (c.status.burning > 0.0, "burning"),
                    (c.status.sick > 0.0, "sick"),
                    (c.status.grabbed_by.is_some(), "constricted"),
                    (c.playing_dead > 0.0, "playing dead"),
                    (c.luring, "luring"),
                    (
                        c.pregnant.is_some(),
                        if sp.traits.eggs {
                            "carrying eggs"
                        } else {
                            "pregnant"
                        },
                    ),
                    (c.still > 1.0 && sp.traits.camouflage, "camouflaged"),
                ] {
                    if on {
                        state.push(s.to_string());
                    }
                }
                chips(ui, &state, role_colour(sp.role));
                ui.separator();
                ui.label(
                    egui::RichText::new("Utility scores (highest runs)")
                        .small()
                        .strong(),
                );
                let max = c.brain.scores.first().map_or(1.0, |s| s.1.max(0.01));
                for (k, s) in c.brain.scores.iter().take(8) {
                    let colour = if *k == c.brain.current {
                        role_colour(sp.role)
                    } else {
                        egui::Color32::from_gray(110)
                    };
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [110.0, 14.0],
                            egui::Label::new(egui::RichText::new(k.label()).small()),
                        );
                        let (rect, _) = ui.allocate_exact_size(egui::vec2(110.0, 9.0), egui::Sense::hover());
                        let mut fill = rect;
                        fill.set_width(rect.width() * (s / max).clamp(0.0, 1.0));
                        ui.painter().rect_filled(fill, 2.0, colour);
                        ui.label(egui::RichText::new(format!("{s:.2}")).small());
                    });
                }
                ui.separator();
                let p = &c.brain.perception;
                let fmt = |o: Option<f32>| o.map_or("none".to_string(), |d| format!("{d:.0} cells"));
                ui.label(egui::RichText::new("Senses").small().strong());
                ui.label(
                    egui::RichText::new(format!(
                        "threat {} · prey {} · kin nearby {} · light {:.0}%",
                        fmt(p.threat.map(|t| t.2)),
                        fmt(p.prey.map(|t| t.2)),
                        p.kin_count,
                        p.light * 100.0
                    ))
                    .small(),
                );
                let food = p
                    .plant
                    .map(|(_, pos)| pos.dist(c.pos))
                    .or(p.carcass.map(|(_, pos)| pos.dist(c.pos)));
                ui.label(
                    egui::RichText::new(format!(
                        "food {} · home {} · mate {}",
                        fmt(food),
                        c.brain
                            .home
                            .map_or("none".into(), |h| format!("{:.0} cells", h.dist(c.pos))),
                        if p.mate.is_some() { "calling" } else { "none" }
                    ))
                    .small(),
                );
                ui.horizontal(|ui| {
                    ui.checkbox(&mut follow, "Follow (F)");
                });
            });
        scene.follow = follow;
        if !open {
            scene.selected = None;
            scene.follow = false;
        }
    }

    egui::Area::new(egui::Id::new("eco_help"))
        .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -6.0])
        .interactable(false)
        .show(ctx, |ui| {
            ui.label(
                egui::RichText::new("Click a creature to inspect · right-drag / WASD pan · wheel zoom · Space pause · 1-4 speed · N new scene · L labels · Esc back")
                    .small()
                    .color(egui::Color32::from_gray(170)),
            );
        });

    scene.keyboard_busy = ctx.egui_wants_keyboard_input();
    scene.pointer_busy = ctx.is_pointer_over_egui() || ctx.egui_wants_pointer_input();

    if let Some(seed) = regen {
        regenerate(scene, &mut images, seed);
    }
    Ok(())
}
