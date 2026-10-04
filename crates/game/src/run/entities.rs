//! Things in the world besides cells: chests, altars, shrines, the core,
//! thrown projectiles, blast charges, pocket suns and stuck torches.

use bevy::prelude::*;
use sbct_sim::descent::{Layer, SpawnKind};
use sbct_sim::material::MAX_CHARGE;
use sbct_sim::world::disc;
use sbct_sim::{Kind, Material, World};

use super::items::{Active, ItemId, icon};
use super::physics::to_world;
use super::player::RunPlayer;
use super::save::SaveData;
use super::{Phase, Run, RunSetup};
use crate::assets::{GameAssets, props_frame};
use crate::audio::Sfx;
use crate::fx::{Burst, HitStop, Shake};
use crate::render::InGameEntity;
use crate::session::Session;

const INTERACT_RANGE: f32 = 16.0;
const BLAST_RADIUS: i32 = 14;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PropKind {
    Chest { opened: bool },
    Altar { item: Option<ItemId> },
    Shrine { item: Option<ItemId>, price: u32 },
    Core,
}

/// A static interactable. `pos` is the bottom centre in cells.
#[derive(Component)]
pub struct Prop {
    pub kind: PropKind,
    pub pos: Vec2,
}

impl Prop {
    pub fn center(&self) -> Vec2 {
        let h = if self.kind == PropKind::Core { 24.0 } else { 8.0 };
        self.pos - Vec2::Y * h
    }
}

/// The floating item icon above an altar or shrine.
#[derive(Component)]
pub struct DisplayedItem(pub Entity);

/// Context prompt for the HUD ("F: Open chest").
#[derive(Resource, Default)]
pub struct Prompt(pub Option<(String, Vec2)>);

pub fn spawn_entities(
    mut commands: Commands,
    setup: Res<RunSetup>,
    assets: Res<GameAssets>,
    mut run: ResMut<Run>,
    save: Res<SaveData>,
) {
    commands.init_resource::<Prompt>();
    for s in &setup.spawns {
        let pos = Vec2::new(s.x as f32 + 0.5, s.y as f32 + 1.0);
        let kind = match s.kind {
            SpawnKind::Chest => PropKind::Chest { opened: false },
            SpawnKind::Altar => PropKind::Altar { item: run.roll_item(&save) },
            SpawnKind::Shrine => {
                let layer = Layer::at_depth(s.y);
                PropKind::Shrine { item: run.roll_item(&save), price: 30 + layer.index() as u32 * 30 }
            }
            SpawnKind::Core => PropKind::Core,
            SpawnKind::Creature(_) => continue,
        };
        let (sprite, z, offset) = match kind {
            PropKind::Chest { .. } => (assets.props.sprite(props_frame::CHEST_CLOSED), 3.0, 12.0),
            PropKind::Altar { .. } => (assets.props.sprite(props_frame::ALTAR), 3.0, 12.0),
            PropKind::Shrine { .. } => (assets.props.sprite(props_frame::SHRINE), 3.0, 12.0),
            PropKind::Core => (assets.core.sprite(0), 4.0, 24.0),
        };
        let entity = commands
            .spawn((
                Prop { kind, pos },
                InGameEntity,
                sprite,
                Transform::from_translation(to_world(pos - Vec2::Y * offset, z)),
            ))
            .id();
        if let PropKind::Altar { item: Some(item) } | PropKind::Shrine { item: Some(item), .. } = kind {
            let mut icon = assets.items.sprite(item.def().icon);
            icon.custom_size = Some(Vec2::splat(10.0));
            commands.spawn((
                DisplayedItem(entity),
                InGameEntity,
                icon,
                Transform::from_translation(to_world(pos - Vec2::Y * 30.0, 4.0)),
            ));
        }
    }
    // Clear the popup some starting items may have queued.
    run.popup = None;
}

pub fn update_charges(time: Res<Time>, mut run: ResMut<Run>) {
    let dt = time.delta_secs();
    let items = run.items.clone();
    for item in items {
        if let Some(Active::Charges { max, recharge }) = item.def().active
            && let Some(entry) = run.charges.get_mut(&item)
            && entry.0 < max
        {
            entry.1 += dt;
            if entry.1 >= recharge {
                entry.0 += 1;
                entry.1 = 0.0;
            }
        }
    }
}

pub fn interact(
    mut commands: Commands,
    input: Res<super::input::PlayerInput>,
    mut player: ResMut<RunPlayer>,
    mut run: ResMut<Run>,
    save: Res<SaveData>,
    mut props: Query<(Entity, &mut Prop, &mut Sprite)>,
    displays: Query<(Entity, &DisplayedItem)>,
    mut prompt: ResMut<Prompt>,
    mut sfx: MessageWriter<Sfx>,
    mut bursts: MessageWriter<Burst>,
    paused: Res<super::hud::Paused>,
) {
    prompt.0 = None;
    if !run.is_playing() || paused.0 {
        return;
    }
    let c = player.body.center();
    let nearest = props
        .iter_mut()
        .filter(|(_, p, _)| p.center().distance(c) < INTERACT_RANGE + if p.kind == PropKind::Core { 14.0 } else { 0.0 })
        .min_by(|a, b| a.1.center().distance(c).total_cmp(&b.1.center().distance(c)));
    let Some((entity, mut prop, mut sprite)) = nearest else { return };

    let text = match prop.kind {
        PropKind::Chest { opened: false } => Some("F  Open chest".to_string()),
        PropKind::Altar { item: Some(item) } => Some(format!("F  Take {}", item.def().name)),
        PropKind::Shrine { item: Some(item), price } => Some(format!("F  Buy {} for {price} ore", item.def().name)),
        PropKind::Core => Some("F  Touch the core".to_string()),
        _ => None,
    };
    let Some(text) = text else { return };
    prompt.0 = Some((text, prop.pos - Vec2::Y * 30.0));
    if !input.interact {
        return;
    }

    let remove_display = |commands: &mut Commands| {
        for (e, d) in &displays {
            if d.0 == entity {
                commands.entity(e).despawn();
            }
        }
    };
    let at = prop.center();
    match prop.kind {
        PropKind::Chest { opened: false } => {
            prop.kind = PropKind::Chest { opened: true };
            if let Some(atlas) = &mut sprite.texture_atlas {
                atlas.index = props_frame::CHEST_OPEN;
            }
            sfx.write(Sfx::at("chest_open", at));
            bursts.write(Burst::new(at, Color::srgb(1.0, 0.85, 0.3)).count(16).speed(50.0).gravity(-20.0));
            let roll = run.rng.next_u8();
            match (roll, run.roll_item(&save)) {
                (0..150, Some(item)) => {
                    run.give(item);
                    sfx.write(Sfx::ui("item_get"));
                }
                (150..200, _) => {
                    run.ropes += 2;
                    run.torches += 2;
                }
                (200..230, _) => player.hp = (player.hp + 40.0).min(player.max_hp),
                _ => run.ore += 25,
            }
        }
        PropKind::Altar { item: Some(item) } => {
            prop.kind = PropKind::Altar { item: None };
            remove_display(&mut commands);
            run.give(item);
            sfx.write(Sfx::ui("item_get"));
            bursts.write(Burst::new(at, Color::srgb(0.6, 0.9, 1.0)).count(20).speed(40.0).gravity(-30.0));
        }
        PropKind::Shrine { item: Some(item), price } => {
            if run.ore >= price {
                run.ore -= price;
                prop.kind = PropKind::Shrine { item: None, price };
                remove_display(&mut commands);
                run.give(item);
                sfx.write(Sfx::at("shrine_buy", at));
                sfx.write(Sfx::ui("item_get"));
            } else {
                sfx.write(Sfx::ui("ui_click"));
            }
        }
        PropKind::Core => {
            run.phase = Phase::Won(0.0);
            bursts.write(Burst::new(at, Color::srgb(1.0, 0.9, 0.6)).count(80).speed(120.0).gravity(0.0).life(1.5));
        }
        _ => {}
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectileKind {
    Torch,
    AcidFlask,
    FrostSeed,
    Spores,
    SparkBolt,
}

#[derive(Component)]
pub struct Projectile {
    pub kind: ProjectileKind,
    pub pos: Vec2,
    pub vel: Vec2,
    pub age: f32,
}

impl Projectile {
    pub fn bundle(kind: ProjectileKind, pos: Vec2, vel: Vec2, assets: &GameAssets) -> impl Bundle {
        let sprite = match kind {
            ProjectileKind::SparkBolt => Sprite::from_color(Color::srgb(0.75, 0.9, 1.0), Vec2::splat(2.0)),
            k => {
                let index = match k {
                    ProjectileKind::Torch => icon::TORCH,
                    ProjectileKind::AcidFlask => ItemId::AcidFlask.def().icon,
                    ProjectileKind::FrostSeed => ItemId::FrostSeed.def().icon,
                    _ => ItemId::FungalSpores.def().icon,
                };
                let mut s = assets.items.sprite(index);
                s.custom_size = Some(Vec2::splat(7.0));
                s
            }
        };
        (Projectile { kind, pos, vel, age: 0.0 }, InGameEntity, sprite, Transform::from_translation(to_world(pos, 5.5)))
    }
}

/// A placed blast charge counting down.
#[derive(Component)]
pub struct Bomb {
    pub pos: Vec2,
    pub fuse: f32,
    vy: f32,
}

impl Bomb {
    pub fn bundle(pos: Vec2, assets: &GameAssets) -> impl Bundle {
        let mut s = assets.items.sprite(ItemId::BlastCharges.def().icon);
        s.custom_size = Some(Vec2::splat(8.0));
        (Bomb { pos, fuse: 2.5, vy: 0.0 }, InGameEntity, s, Transform::from_translation(to_world(pos, 5.0)))
    }
}

/// A tiny sun cooking its surroundings.
#[derive(Component)]
pub struct Sun {
    pub pos: Vec2,
    pub life: f32,
    timer: f32,
}

impl Sun {
    pub fn bundle(pos: Vec2, assets: &GameAssets) -> impl Bundle {
        let mut s = assets.items.sprite(ItemId::PocketSun.def().icon);
        s.custom_size = Some(Vec2::splat(12.0));
        (Sun { pos, life: 10.0, timer: 0.0 }, InGameEntity, s, Transform::from_translation(to_world(pos, 5.0)))
    }
}

/// A torch stuck where it landed. Lights the area and can ignite things.
#[derive(Component)]
pub struct Torch {
    pub pos: Vec2,
    pub life: f32,
    timer: f32,
}

pub fn update_projectiles(
    mut commands: Commands,
    time: Res<Time>,
    mut session: ResMut<Session>,
    mut run: ResMut<Run>,
    mut q: Query<(Entity, &mut Projectile, &mut Transform)>,
    assets: Res<GameAssets>,
    mut sfx: MessageWriter<Sfx>,
    mut bursts: MessageWriter<Burst>,
) {
    let Some(world) = session.world.as_mut() else { return };
    let dt = time.delta_secs();
    for (entity, mut p, mut tf) in &mut q {
        p.age += dt;
        let gravity = if p.kind == ProjectileKind::SparkBolt { 0.0 } else { 260.0 };
        p.vel.y += gravity * dt;
        let delta = p.vel * dt;
        let steps = delta.length().ceil().max(1.0) as i32;
        let mut hit = None;
        for _ in 0..steps {
            let next = p.pos + delta / steps as f32;
            let m = world.material(next.x.floor() as i32, next.y.floor() as i32);
            if !m.is_open() {
                hit = Some((next, m));
                break;
            }
            p.pos = next;
        }
        let expired = p.kind == ProjectileKind::SparkBolt && p.age > 0.5;
        tf.translation = to_world(p.pos, 5.5);
        tf.rotation = Quat::from_rotation_z(-p.age * 12.0);
        if hit.is_none() && !expired {
            continue;
        }
        commands.entity(entity).despawn();
        let (at, hit_mat) = hit.unwrap_or((p.pos, Material::Empty));
        run.influence(at);
        let (x, y) = (p.pos.x.floor() as i32, p.pos.y.floor() as i32);
        match p.kind {
            ProjectileKind::Torch => {
                if hit_mat == Material::Water || hit_mat == Material::Acid {
                    sfx.write(Sfx::at("sizzle", at).volume(0.5));
                } else {
                    sfx.write(Sfx::at("torch_ignite", at));
                    commands.spawn((
                        Torch { pos: p.pos, life: 150.0, timer: 0.0 },
                        InGameEntity,
                        assets.props.sprite(props_frame::TORCH),
                        Transform::from_translation(to_world(p.pos - Vec2::Y * 6.0, 4.5)),
                    ));
                }
            }
            ProjectileKind::AcidFlask => {
                for (cx, cy) in disc(x, y, 3) {
                    if world.material(cx, cy).is_open() {
                        world.set(cx, cy, Material::Acid);
                    }
                }
                for i in 0..20 {
                    let a = i as f32 / 20.0 * std::f32::consts::TAU;
                    world.spawn_particle(p.pos.x, p.pos.y - 1.0, a.cos() * 1.5, a.sin() * 1.5 - 1.0, Material::Acid);
                }
                sfx.write(Sfx::at("acid_hiss", at));
                bursts.write(Burst::new(at, Color::srgb(0.85, 0.95, 0.9)).count(10).speed(60.0));
            }
            ProjectileKind::FrostSeed => {
                for (cx, cy) in disc(x, y, 6) {
                    let m = world.material(cx, cy);
                    let near = (cx - x).abs() + (cy - y).abs() <= 3;
                    if m == Material::Water || m == Material::Lava || (near && m.is_open()) {
                        world.set_with_life(cx, cy, Material::Frost, 14);
                    }
                }
                sfx.write(Sfx::at("freeze", at));
                bursts.write(Burst::new(at, Color::srgb(0.8, 0.95, 1.0)).count(16).speed(50.0).gravity(20.0));
            }
            ProjectileKind::Spores => {
                for (cx, cy) in disc(x, y, 7) {
                    if world.material(cx, cy).is_open() && anchored(world, cx, cy) {
                        world.set(cx, cy, Material::Fungus);
                    }
                }
                sfx.write(Sfx::at("plant", at));
                bursts.write(Burst::new(at, Color::srgb(0.5, 0.9, 0.5)).count(14).speed(30.0).gravity(-10.0));
            }
            ProjectileKind::SparkBolt => {
                world.electrify(at.x.floor() as i32, at.y.floor() as i32, MAX_CHARGE);
                for (cx, cy) in disc(x, y, 2) {
                    if world.material(cx, cy).is_open() {
                        world.set(cx, cy, Material::Spark);
                    }
                }
                sfx.write(Sfx::at("spark", at));
                bursts.write(Burst::new(at, Color::srgb(0.7, 0.9, 1.0)).count(12).speed(80.0).gravity(0.0));
            }
        }
    }
}

fn anchored(world: &World, x: i32, y: i32) -> bool {
    [(0, 1), (0, -1), (1, 0), (-1, 0)].iter().any(|(dx, dy)| world.material(x + dx, y + dy).kind() == Kind::Solid)
}

/// A ring of flame (Thermal Suit vent).
pub fn fire_burst(bursts: &mut MessageWriter<Burst>, at: Vec2) {
    bursts.write(Burst::new(at, Color::srgb(1.0, 0.6, 0.15)).count(40).speed(110.0).gravity(-40.0).life(0.6).size(1.5));
}

pub fn update_bombs(
    mut commands: Commands,
    time: Res<Time>,
    mut session: ResMut<Session>,
    mut run: ResMut<Run>,
    mut q: Query<(Entity, &mut Bomb, &mut Transform, &mut Sprite)>,
    mut sfx: MessageWriter<Sfx>,
    mut shake: ResMut<Shake>,
    mut stop: ResMut<HitStop>,
) {
    let Some(world) = session.world.as_mut() else { return };
    let dt = time.delta_secs();

    if let Some(at) = run.pending_fire_burst.take() {
        for (x, y) in disc(at.x as i32, at.y as i32, 10) {
            if world.material(x, y).is_open() && run.rng.chance(70) {
                world.set(x, y, Material::Fire);
            }
        }
    }

    for (entity, mut bomb, mut tf, mut sprite) in &mut q {
        bomb.fuse -= dt;
        // Fall until resting on something.
        bomb.vy = (bomb.vy + 300.0 * dt).min(300.0);
        let next = bomb.pos + Vec2::Y * bomb.vy * dt;
        if world.material(next.x as i32, next.y as i32).is_open() {
            bomb.pos = next;
        } else {
            bomb.vy = 0.0;
        }
        tf.translation = to_world(bomb.pos - Vec2::Y * 3.0, 5.0);
        let blink = (bomb.fuse * if bomb.fuse < 0.8 { 16.0 } else { 6.0 }) as i32 % 2 == 0;
        sprite.color = if blink { Color::srgb(1.0, 0.4, 0.4) } else { Color::WHITE };
        if bomb.fuse <= 0.0 {
            commands.entity(entity).despawn();
            let (x, y) = (bomb.pos.x as i32, bomb.pos.y as i32 - 2);
            world.explode(x, y, BLAST_RADIUS);
            run.influence(bomb.pos);
            if run.has(ItemId::VolatileCore) {
                run.volatile_spots.push((x, y));
            }
            shake.add(0.9);
            stop.0 = 0.08;
            sfx.write(Sfx::at("explosion", bomb.pos));
        }
    }
}

pub fn update_suns(
    mut commands: Commands,
    time: Res<Time>,
    mut session: ResMut<Session>,
    mut q: Query<(Entity, &mut Sun, &mut Transform)>,
    mut bursts: MessageWriter<Burst>,
) {
    let Some(world) = session.world.as_mut() else { return };
    let dt = time.delta_secs();
    for (entity, mut sun, mut tf) in &mut q {
        sun.life -= dt;
        sun.timer -= dt;
        let pulse = 1.0 + (sun.life * 8.0).sin() * 0.12;
        tf.scale = Vec3::splat(pulse);
        tf.translation = to_world(sun.pos, 5.0);
        if sun.timer <= 0.0 {
            sun.timer = 0.2;
            world.cook(sun.pos.x as i32, sun.pos.y as i32, 9);
            bursts.write(Burst::new(sun.pos, Color::srgb(1.0, 0.9, 0.5)).count(2).speed(30.0).gravity(-20.0).life(0.4));
        }
        if sun.life <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

pub fn update_torches(
    mut commands: Commands,
    time: Res<Time>,
    mut session: ResMut<Session>,
    mut run: ResMut<Run>,
    mut q: Query<(Entity, &mut Torch)>,
    mut sfx: MessageWriter<Sfx>,
) {
    let Some(world) = session.world.as_mut() else { return };
    let dt = time.delta_secs();
    for (entity, mut torch) in &mut q {
        torch.life -= dt;
        torch.timer -= dt;
        let (x, y) = (torch.pos.x as i32, torch.pos.y as i32);
        let here = world.material(x, y);
        if torch.life <= 0.0 || matches!(here, Material::Water | Material::Acid) {
            if here == Material::Water {
                sfx.write(Sfx::at("sizzle", torch.pos).volume(0.4));
            }
            commands.entity(entity).despawn();
            continue;
        }
        if torch.timer <= 0.0 {
            torch.timer = 0.4;
            let (dx, dy) = ((run.rng.next_u8() % 5) as i32 - 2, (run.rng.next_u8() % 5) as i32 - 3);
            if world.material(x + dx, y + dy).props().flammability > 0 {
                world.ignite(x + dx, y + dy);
            }
        }
    }
}

pub fn animate_props(
    time: Res<Time>,
    mut torches: Query<&mut Sprite, With<Torch>>,
    mut props: Query<(&Prop, &mut Sprite), Without<Torch>>,
    mut displays: Query<(&DisplayedItem, &mut Transform)>,
    prop_pos: Query<&Prop>,
) {
    let t = time.elapsed_secs();
    let frame = (t * 10.0) as usize;
    for mut s in &mut torches {
        if let Some(atlas) = &mut s.texture_atlas {
            atlas.index = props_frame::TORCH + frame % 4;
        }
    }
    for (prop, mut s) in &mut props {
        if prop.kind == PropKind::Core
            && let Some(atlas) = &mut s.texture_atlas
        {
            atlas.index = (t * 8.0) as usize % 8;
        }
    }
    for (d, mut tf) in &mut displays {
        if let Ok(prop) = prop_pos.get(d.0) {
            let bob = (t * 2.5 + prop.pos.x).sin() * 1.5;
            tf.translation = to_world(prop.pos - Vec2::Y * (28.0 + bob), 4.0);
        }
    }
}
