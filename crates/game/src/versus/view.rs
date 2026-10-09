//! Drawing the fight: the mirror rendered to a texture, an automatic camera
//! that frames the action and punches in on big moments, and portraits for
//! the designer.

use bevy::asset::RenderAssetUsages;
use bevy::image::ImageSampler;
use bevy::input::mouse::MouseWheel;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::window::PrimaryWindow;
use bevy_egui::{EguiContexts, EguiTextureHandle, egui};
use sbct_sim::eco::biome::{HEIGHT, WIDTH};
use sbct_sim::eco::render::{DrawOpts, Renderer, portrait};
use sbct_sim::versus::design::{Design, build_teams};

use crate::render::{PIXEL_SCALE, WorldCamera};
use crate::versus::{Phase, Versus};

#[derive(Component)]
pub struct VersusEntity;

/// A portrait registered with egui.
pub struct Portrait {
    pub design: Design,
    pub handle: Handle<Image>,
    pub size: egui::Vec2,
    pub id: Option<egui::TextureId>,
}

#[derive(Resource)]
pub struct VersusView {
    renderer: Renderer,
    image: Option<Handle<Image>>,
    /// Camera centre in cells and screen pixels per cell.
    pub cam: Vec2,
    pub zoom: f32,
    /// Until this time (seconds of app time) the user's own zoom/pan wins.
    manual_until: f32,
    drag: Option<(Vec2, Vec2)>,
    shake_t: f32,
    /// Portraits keyed by (player, design).
    pub portraits: Vec<(u64, Portrait)>,
    pub keyboard_busy: bool,
    pub pointer_busy: bool,
}

impl Default for VersusView {
    fn default() -> Self {
        VersusView {
            renderer: Renderer::new(WIDTH, HEIGHT),
            image: None,
            cam: Vec2::new(WIDTH as f32 / 2.0, HEIGHT as f32 / 2.0),
            zoom: 2.5,
            manual_until: 0.0,
            drag: None,
            shake_t: 0.0,
            portraits: Vec::new(),
            keyboard_busy: false,
            pointer_busy: false,
        }
    }
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

pub fn enter(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut view: ResMut<VersusView>,
    mut clear: ResMut<ClearColor>,
) {
    let image = new_image(&mut images, WIDTH, HEIGHT, vec![0; WIDTH * HEIGHT * 4]);
    commands.spawn((
        VersusEntity,
        Sprite {
            image: image.clone(),
            custom_size: Some(Vec2::new(WIDTH as f32, HEIGHT as f32)),
            ..default()
        },
        Transform::from_xyz(WIDTH as f32 / 2.0, -(HEIGHT as f32) / 2.0, 0.0),
    ));
    view.image = Some(image);
    view.cam = Vec2::new(WIDTH as f32 / 2.0, HEIGHT as f32 / 2.0);
    view.zoom = 2.5;
    clear.0 = Color::srgb(0.02, 0.02, 0.04);
}

pub fn exit(
    mut commands: Commands,
    entities: Query<Entity, With<VersusEntity>>,
    mut view: ResMut<VersusView>,
    mut clear: ResMut<ClearColor>,
    mut camera: Single<(&mut Transform, &mut Projection), With<WorldCamera>>,
) {
    for e in &entities {
        commands.entity(e).despawn();
    }
    view.image = None;
    view.portraits.clear();
    clear.0 = Color::srgb(0.45, 0.62, 0.85);
    let (ref mut tf, ref mut proj) = *camera;
    if let Projection::Orthographic(o) = &mut **proj {
        o.scale = 1.0 / PIXEL_SCALE;
    }
    tf.translation = Vec3::ZERO;
}

/// Renders the mirror into the arena texture.
pub fn update(
    mut view: ResMut<VersusView>,
    versus: Res<Versus>,
    mut images: ResMut<Assets<Image>>,
    mut sprites: Query<&mut Visibility, With<VersusEntity>>,
) {
    let fighting = matches!(versus.phase, Phase::Fight | Phase::Results) && versus.mirror.is_some();
    for mut v in &mut sprites {
        *v = if fighting {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !fighting {
        return;
    }
    let Some(mirror) = &versus.mirror else { return };
    let view = &mut *view;
    view.renderer.draw_scene(&mirror.scene(), &DrawOpts::default());
    let Some(handle) = &view.image else { return };
    let Some(mut image) = images.get_mut(handle) else {
        return;
    };
    if let Some(data) = image.data.as_mut() {
        data.copy_from_slice(&view.renderer.rgba);
    }
}

/// The director: frames everyone alive, leans into whatever just happened,
/// shakes on impacts. The wheel and right-drag take over for a few seconds.
pub fn camera(
    mut view: ResMut<VersusView>,
    versus: Res<Versus>,
    time: Res<Time>,
    mut wheel: MessageReader<MouseWheel>,
    mouse: Res<ButtonInput<MouseButton>>,
    window: Single<&Window, With<PrimaryWindow>>,
    mut camera: Single<(&mut Transform, &mut Projection), With<WorldCamera>>,
) {
    let now = time.elapsed_secs();
    let dt = time.delta_secs().min(0.1);
    let (ww, wh) = (window.width(), window.height());
    let fighting = matches!(versus.phase, Phase::Fight | Phase::Results) && versus.mirror.is_some();
    if !fighting {
        wheel.clear();
        return;
    }
    if !view.pointer_busy {
        for ev in wheel.read() {
            let factor = if ev.y > 0.0 { 1.15 } else { 1.0 / 1.15 };
            view.zoom = (view.zoom * factor).clamp(1.2, 10.0);
            view.manual_until = now + 4.0;
        }
        let cursor = window.cursor_position();
        if let Some(cur) = cursor {
            if mouse.pressed(MouseButton::Right) || mouse.pressed(MouseButton::Middle) {
                match view.drag {
                    None => view.drag = Some((cur, view.cam)),
                    Some((start, cam0)) => {
                        let z = view.zoom;
                        view.cam = cam0 - (cur - start) / z;
                        view.manual_until = now + 4.0;
                    }
                }
            } else {
                view.drag = None;
            }
        }
    } else {
        wheel.clear();
    }
    let mirror = versus.mirror.as_ref().unwrap();
    if now > view.manual_until {
        let (target, zoom) = match mirror.bounds() {
            Some((lo, hi)) => {
                let mut centre = Vec2::new((lo.x + hi.x) / 2.0, (lo.y + hi.y) / 2.0 - 6.0);
                let bw = (hi.x - lo.x) + 110.0;
                let bh = (hi.y - lo.y) + 80.0;

                let mut zoom = (ww / bw).min(wh / bh).clamp(1.6, 5.0);
                if let Some((p, _)) = mirror.focus {
                    centre = centre.lerp(Vec2::new(p.x, p.y), 0.65);
                    zoom *= 1.35;
                }
                (centre, zoom)
            }
            None => (view.cam, view.zoom),
        };
        let k = (dt * 3.5).min(1.0);
        view.cam = view.cam.lerp(target, k);
        view.zoom += (zoom - view.zoom) * (dt * 2.5).min(1.0);
    }
    // Keep the arena on screen.
    let half = Vec2::new(ww, wh) / view.zoom / 2.0;
    let clamp = |v: f32, half: f32, size: f32| {
        if half * 2.0 >= size {
            size / 2.0
        } else {
            v.clamp(half, size - half)
        }
    };
    view.cam.x = clamp(view.cam.x, half.x, WIDTH as f32);
    view.cam.y = clamp(view.cam.y, half.y, HEIGHT as f32);
    // Shake.
    view.shake_t += dt * 40.0;
    let s = mirror.shake * 2.5;
    let shake = Vec2::new((view.shake_t * 1.3).sin() * s, (view.shake_t * 1.7).cos() * s);
    let (ref mut tf, ref mut proj) = *camera;
    if let Projection::Orthographic(o) = &mut **proj {
        o.scale = 1.0 / view.zoom;
    }
    tf.translation.x = view.cam.x + shake.x;
    tf.translation.y = -(view.cam.y + shake.y);
}

/// A portrait texture for a design, made on demand and cached.
pub fn portrait_for(
    view: &mut VersusView,
    images: &mut Assets<Image>,
    contexts: &mut EguiContexts,
    key: u64,
    design: &Design,
    team: usize,
    others: &[(String, Design)],
) -> Option<(egui::TextureId, egui::Vec2)> {
    let stale = view
        .portraits
        .iter()
        .position(|(k, p)| *k == key && p.design != *design);
    if let Some(i) = stale {
        view.portraits.remove(i);
    }
    if !view.portraits.iter().any(|(k, _)| *k == key) {
        // Later teams may have been recoloured to stand out; mirror that.
        let mut designs: Vec<(String, Design)> = others.to_vec();
        designs.truncate(team);
        designs.push((String::new(), design.clone()));
        let (species, _) = build_teams(&designs, None).pop().unwrap();
        let (w, h, data) = portrait(&species);
        let handle = new_image(images, w, h, data);
        view.portraits.push((
            key,
            Portrait {
                design: design.clone(),
                handle,
                size: egui::vec2(w as f32, h as f32),
                id: None,
            },
        ));
    }
    let (_, p) = view.portraits.iter_mut().find(|(k, _)| *k == key)?;
    if p.id.is_none() {
        p.id = Some(contexts.add_image(EguiTextureHandle::Strong(p.handle.clone())));
    }
    p.id.map(|id| (id, p.size))
}
