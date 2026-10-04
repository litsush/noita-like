//! Things in the world besides cells: chests, altars, shrines, the core,
//! thrown projectiles, blast charges, pocket suns, light orbs and shards.

use bevy::prelude::*;
use sbct_sim::Material;
use sbct_sim::descent::{Layer, SpawnKind};
use sbct_sim::world::disc;

use super::Spell;
use super::achievements::AchievementId;
use super::physics::to_world;
use super::player::RunPlayer;
use super::save::SaveData;
use super::scrolls::ScrollId;
use super::spells::{self, Harm};
use super::{Phase, Run, RunSetup, grant};
use crate::assets::{GameAssets, props_frame};
use crate::audio::Sfx;
use crate::fx::{Burst, HitStop, Shake};
use crate::render::InGameEntity;
use crate::session::Session;

const INTERACT_RANGE: f32 = 16.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PropKind {
    /// About one chest in six is a mimic.
    Chest {
        opened: bool,
        mimic: bool,
    },
    /// A fallen apprentice, holding a journal page.
    Remains {
        searched: bool,
        journal: u16,
    },
    Altar {
        offer: Offer,
    },
    Shrine {
        offer: Offer,
        price: u32,
    },
    Core,
}

/// What an altar or shrine holds. Rolled lazily, the first time the player
/// comes near, so it matches their schools at that moment.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Offer {
    Unrolled,
    Scroll(ScrollId),
    Taken,
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
    // Hand out journal pages the player hasn't read first.
    let mut unread: Vec<u16> = (0..super::lore::JOURNALS.len() as u16)
        .filter(|i| !save.journals_read.contains(i))
        .collect();
    commands.init_resource::<Prompt>();
    for s in &setup.spawns {
        let pos = Vec2::new(s.x as f32 + 0.5, s.y as f32 + 1.0);
        let kind = match s.kind {
            SpawnKind::Chest => PropKind::Chest {
                opened: false,
                mimic: s.y > 300 && run.rng.chance(42),
            },
            SpawnKind::Remains => {
                let journal = if unread.is_empty() {
                    (run.rng.next_u64() % super::lore::JOURNALS.len() as u64) as u16
                } else {
                    unread.swap_remove((run.rng.next_u64() % unread.len() as u64) as usize)
                };
                PropKind::Remains {
                    searched: false,
                    journal,
                }
            }
            SpawnKind::Altar => PropKind::Altar {
                offer: Offer::Unrolled,
            },
            SpawnKind::Shrine => {
                let layer = Layer::at_depth(s.y);
                PropKind::Shrine {
                    offer: Offer::Unrolled,
                    price: 30 + layer.index() as u32 * 30,
                }
            }
            SpawnKind::Core => PropKind::Core,
            _ => continue,
        };
        let (sprite, z, offset) = match kind {
            PropKind::Chest { mimic, .. } => (
                assets.props.sprite(if mimic {
                    props_frame::MIMIC
                } else {
                    props_frame::CHEST_CLOSED
                }),
                3.0,
                12.0,
            ),
            PropKind::Remains { .. } => (assets.props.sprite(props_frame::REMAINS), 3.0, 12.0),
            PropKind::Altar { .. } => (assets.props.sprite(props_frame::ALTAR), 3.0, 12.0),
            PropKind::Shrine { .. } => (assets.props.sprite(props_frame::SHRINE), 3.0, 12.0),
            PropKind::Core => (assets.core.sprite(0), 4.0, 24.0),
        };
        commands.spawn((
            Prop { kind, pos },
            InGameEntity,
            sprite,
            Transform::from_translation(to_world(pos - Vec2::Y * offset, z)),
        ));
    }
    // Clear the popup a starting scroll may have queued.
    run.popup = None;
}

/// Shows the scroll an altar or shrine holds as a floating icon.
fn display(commands: &mut Commands, assets: &GameAssets, owner: Entity, pos: Vec2, icon: usize) {
    let mut sprite = assets.items.sprite(icon);
    sprite.custom_size = Some(Vec2::splat(10.0));
    commands.spawn((
        DisplayedItem(owner),
        InGameEntity,
        sprite,
        Transform::from_translation(to_world(pos - Vec2::Y * 30.0, 4.0)),
    ));
}

/// Rolls altars and shrines as the player approaches, and resolves the
/// attunement choice.
pub fn prepare_offers(
    mut commands: Commands,
    assets: Res<GameAssets>,
    player: Res<RunPlayer>,
    mut run: ResMut<Run>,
    save: Res<SaveData>,
    mut props: Query<(Entity, &mut Prop)>,
    displays: Query<(Entity, &DisplayedItem)>,
    mut sfx: MessageWriter<Sfx>,
    mut bursts: MessageWriter<Burst>,
) {
    // A choice made in the attunement UI.
    if let (Some(offer), Some(i)) = (run.attunement.clone(), run.attune_pick.take())
        && let Some(&sc) = offer.get(i)
    {
        run.attune(sc);
        sfx.write(Sfx::ui("attune"));
        if let Some(altar) = run.attune_altar.take()
            && let Ok((_, mut prop)) = props.get_mut(altar)
        {
            prop.kind = PropKind::Altar { offer: Offer::Taken };
            bursts.write(
                Burst::new(prop.center(), Color::srgb(0.9, 0.8, 1.0))
                    .count(30)
                    .speed(50.0)
                    .gravity(-30.0),
            );
            for (e, d) in &displays {
                if d.0 == altar {
                    commands.entity(e).despawn();
                }
            }
        }
    }

    let me = player.body.center();
    let attuned = run.second_school.is_some();
    for (entity, mut prop) in &mut props {
        if prop.center().distance(me) > 90.0 {
            continue;
        }
        let pos = prop.pos;
        match prop.kind {
            // Unattuned wizards treat any altar as an attunement altar.
            PropKind::Altar {
                offer: Offer::Unrolled,
            } if attuned => {
                let offer = run.roll_scroll(&save).map_or(Offer::Taken, Offer::Scroll);
                if let Offer::Scroll(sc) = offer {
                    display(&mut commands, &assets, entity, pos, sc.icon());
                }
                prop.kind = PropKind::Altar { offer };
            }
            PropKind::Shrine {
                offer: Offer::Unrolled,
                price,
            } => {
                let offer = run.roll_scroll(&save).map_or(Offer::Taken, Offer::Scroll);
                if let Offer::Scroll(sc) = offer {
                    display(&mut commands, &assets, entity, pos, sc.icon());
                }
                prop.kind = PropKind::Shrine { offer, price };
            }
            _ => {}
        }
    }
}

pub fn update_light_charges(time: Res<Time>, mut run: ResMut<Run>) {
    if run.light_charges < run.light_max {
        run.light_timer += time.delta_secs();
        if run.light_timer >= LIGHT_RECHARGE {
            run.light_timer = 0.0;
            run.light_charges += 1;
        }
    } else {
        run.light_timer = 0.0;
    }
}

/// Seconds for a light orb charge to come back.
pub const LIGHT_RECHARGE: f32 = 25.0;

pub fn interact(
    mut commands: Commands,
    assets: Res<GameAssets>,
    input: Res<super::input::PlayerInput>,
    mut player: ResMut<RunPlayer>,
    mut run: ResMut<Run>,
    mut save: ResMut<SaveData>,
    mut shake: ResMut<Shake>,
    mut props: Query<(Entity, &mut Prop, &mut Sprite)>,
    displays: Query<(Entity, &DisplayedItem)>,
    mut prompt: ResMut<Prompt>,
    mut sfx: MessageWriter<Sfx>,
    mut bursts: MessageWriter<Burst>,
    paused: Res<super::hud::Paused>,
) {
    prompt.0 = None;
    if !run.is_playing() || paused.0 || run.attunement.is_some() || run.journal.is_some() {
        return;
    }
    let c = player.body.center();
    let nearest = props
        .iter_mut()
        .filter(|(_, p, _)| {
            p.center().distance(c) < INTERACT_RANGE + if p.kind == PropKind::Core { 14.0 } else { 0.0 }
        })
        .min_by(|a, b| a.1.center().distance(c).total_cmp(&b.1.center().distance(c)));
    let Some((entity, mut prop, mut sprite)) = nearest else {
        return;
    };

    let text = match prop.kind {
        PropKind::Chest { opened: false, .. } => Some("F  Open chest".to_string()),
        PropKind::Remains { searched: false, .. } => Some("F  Search the fallen apprentice".to_string()),
        PropKind::Altar {
            offer: Offer::Unrolled,
        } if run.second_school.is_none() => Some("F  Attune to a second school".to_string()),
        PropKind::Altar {
            offer: Offer::Scroll(sc),
        } => Some(format!("F  Take {}", sc.def().name)),
        PropKind::Shrine {
            offer: Offer::Scroll(sc),
            price,
        } => Some(format!("F  Buy {} for {price} shards", sc.def().name)),
        PropKind::Core => Some("F  Touch the heart of the world".to_string()),
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
        PropKind::Chest {
            opened: false,
            mimic: true,
        } => {
            // It was never a chest.
            commands.entity(entity).despawn();
            super::creatures::spawn_creature(
                &mut commands,
                &assets,
                super::creatures::Beast::Mimic,
                prop.pos,
                7,
            );
            let knock = Vec2::new(player.facing * -80.0, -90.0);
            super::hazards::hurt(
                &mut player,
                &mut run,
                15.0,
                super::DamageKind::Creature,
                Some(knock),
                &mut sfx,
                &mut shake,
            );
            sfx.write(Sfx::at("mimic_bite", at));
            sfx.write(Sfx::ui("sting_1"));
        }
        PropKind::Remains {
            searched: false,
            journal,
        } => {
            prop.kind = PropKind::Remains {
                searched: true,
                journal,
            };
            if let Some(atlas) = &mut sprite.texture_atlas {
                atlas.index = props_frame::REMAINS_SEARCHED;
            }
            run.stats.remains_searched += 1;
            sfx.write(Sfx::at("remains_search", at));
            let n = 2 + run.rng.next_u8() % 4;
            for i in 0..n {
                let v = Vec2::new(i as f32 - n as f32 / 2.0, -2.5) * 20.0;
                commands.spawn(ShardPickup::bundle(at, v, 4, &assets));
            }
            // Fallen apprentices studied every school; foreign scrolls crumble.
            if run.rng.chance(115)
                && let Some(sc) = run.roll_any_scroll(&save)
            {
                run.take_scroll(sc);
            }
            run.journal = Some(journal);
            sfx.write(Sfx::ui("journal_open"));
            if save.journals_read.insert(journal) {
                super::save::store(&save);
            }
            if save.journals_read.len() >= 10 {
                super::grant(&mut save, &mut run, AchievementId::LoreKeeper, &mut sfx);
            }
        }
        PropKind::Chest { opened: false, .. } => {
            prop.kind = PropKind::Chest {
                opened: true,
                mimic: false,
            };
            if let Some(atlas) = &mut sprite.texture_atlas {
                atlas.index = props_frame::CHEST_OPEN;
            }
            sfx.write(Sfx::at("chest_open", at));
            bursts.write(
                Burst::new(at, Color::srgb(1.0, 0.85, 0.3))
                    .count(16)
                    .speed(50.0)
                    .gravity(-20.0),
            );
            let roll = run.rng.next_u8();
            match (roll, run.roll_scroll(&save)) {
                (0..140, Some(sc)) => {
                    run.take_scroll(sc);
                    sfx.write(Sfx::ui("scroll_learned"));
                }
                (140..180, _) => run.light_charges = run.light_max,
                (180..215, _) => player.hp = (player.hp + 40.0).min(player.max_hp),
                _ => {
                    for i in 0..5 {
                        let v = Vec2::new(i as f32 - 2.0, -2.5) * 20.0;
                        commands.spawn(ShardPickup::bundle(at, v, 5, &assets));
                    }
                }
            }
        }
        PropKind::Altar {
            offer: Offer::Unrolled,
        } => {
            run.attunement = Some(run.attunement_offer(&save));
            run.attune_altar = Some(entity);
            sfx.write(Sfx::ui("ui_click"));
        }
        PropKind::Altar {
            offer: Offer::Scroll(sc),
        } => {
            prop.kind = PropKind::Altar { offer: Offer::Taken };
            remove_display(&mut commands);
            run.take_scroll(sc);
            sfx.write(Sfx::ui("scroll_learned"));
            bursts.write(
                Burst::new(at, Color::srgb(0.6, 0.9, 1.0))
                    .count(20)
                    .speed(40.0)
                    .gravity(-30.0),
            );
        }
        PropKind::Shrine {
            offer: Offer::Scroll(sc),
            price,
        } => {
            if run.shards >= price {
                run.shards -= price;
                prop.kind = PropKind::Shrine {
                    offer: Offer::Taken,
                    price,
                };
                remove_display(&mut commands);
                run.take_scroll(sc);
                sfx.write(Sfx::at("shrine_buy", at));
                sfx.write(Sfx::ui("scroll_learned"));
            } else {
                sfx.write(Sfx::ui("ui_click"));
            }
        }
        PropKind::Core => {
            run.phase = Phase::Won(0.0);
            bursts.write(
                Burst::new(at, Color::srgb(1.0, 0.9, 0.6))
                    .count(80)
                    .speed(120.0)
                    .gravity(0.0)
                    .life(1.5),
            );
        }
        _ => {}
    }
}

/// A spell in flight. On impact (or when it fizzles) its effect comes from
/// [`spells::impact`].
#[derive(Component)]
pub struct Projectile {
    pub spell: Spell,
    pub pos: Vec2,
    pub vel: Vec2,
    pub gravity: f32,
    pub age: f32,
}

impl Projectile {
    pub fn bundle(spell: Spell, pos: Vec2, vel: Vec2, gravity: f32, assets: &GameAssets) -> impl Bundle {
        let row = spell.school().map_or(8, |s| s.index());
        let mut sprite = assets.spells.sprite(row * 8);
        if matches!(spell, Spell::Fusion(_)) {
            sprite.custom_size = Some(Vec2::splat(12.0));
        }
        (
            Projectile {
                spell,
                pos,
                vel,
                gravity,
                age: 0.0,
            },
            InGameEntity,
            sprite,
            Transform::from_translation(to_world(pos, 5.5)),
        )
    }
}

/// A placed blast charge counting down.
#[derive(Component)]
pub struct Bomb {
    pub pos: Vec2,
    pub fuse: f32,
    pub kind: BombKind,
    vy: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BombKind {
    /// Explodes with this radius.
    Blast(i32),
    /// Just ignites what's there (spore bombs, electrolysis sparks).
    Ignite,
}

impl Bomb {
    pub fn bundle(pos: Vec2, kind: BombKind, fuse: f32, assets: &GameAssets) -> impl Bundle {
        let mut s = assets.items.sprite(ScrollId::AlchemistsCharge.icon());
        s.custom_size = Some(Vec2::splat(if kind == BombKind::Ignite { 5.0 } else { 8.0 }));
        if kind == BombKind::Ignite {
            s.color = Color::srgba(1.0, 0.8, 0.5, 0.0);
        }
        (
            Bomb {
                pos,
                fuse,
                kind,
                vy: 0.0,
            },
            InGameEntity,
            s,
            Transform::from_translation(to_world(pos, 5.0)),
        )
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
        let mut s = assets.items.sprite(ScrollId::PocketSun.icon());
        s.custom_size = Some(Vec2::splat(12.0));
        (
            Sun {
                pos,
                life: 10.0,
                timer: 0.0,
            },
            InGameEntity,
            s,
            Transform::from_translation(to_world(pos, 5.0)),
        )
    }
}

/// A conjured ball of light: flies toward the cursor, sticks or drifts, and
/// fades. With Pyromancy it ignites flammables it touches.
#[derive(Component)]
pub struct LightOrb {
    pub pos: Vec2,
    vel: Vec2,
    pub life: f32,
    stuck: bool,
    timer: f32,
    /// Random flicker dips (horror).
    pub flicker: f32,
}

pub const LIGHT_ORB_LIFE: f32 = 60.0;

impl LightOrb {
    pub fn bundle(pos: Vec2, vel: Vec2, assets: &GameAssets) -> impl Bundle {
        (
            LightOrb {
                pos,
                vel,
                life: LIGHT_ORB_LIFE,
                stuck: false,
                timer: 0.0,
                flicker: 1.0,
            },
            InGameEntity,
            assets.props.sprite(props_frame::LIGHT_ORB),
            Transform::from_translation(to_world(pos - Vec2::Y * 0.0, 5.5)),
        )
    }

    /// Brightness multiplier: fades over its life and flickers.
    pub fn brightness(&self) -> f32 {
        (self.life / LIGHT_ORB_LIFE).sqrt().max(0.15) * self.flicker
    }
}

/// A glowing aether shard on the ground. Pulled toward you when close.
#[derive(Component)]
pub struct ShardPickup {
    pub pos: Vec2,
    vel: Vec2,
    value: u32,
    age: f32,
}

impl ShardPickup {
    pub fn bundle(pos: Vec2, vel: Vec2, value: u32, assets: &GameAssets) -> impl Bundle {
        (
            ShardPickup {
                pos,
                vel,
                value,
                age: 0.0,
            },
            InGameEntity,
            assets.props.sprite(props_frame::SHARD),
            Transform::from_translation(to_world(pos, 5.2)),
        )
    }
}

pub fn update_projectiles(
    mut commands: Commands,
    time: Res<Time>,
    mut session: ResMut<Session>,
    mut run: ResMut<Run>,
    mut player: ResMut<RunPlayer>,
    mut q: Query<(Entity, &mut Projectile, &mut Transform, &mut Sprite)>,
    creatures: Query<&super::creatures::Creature>,
    assets: Res<GameAssets>,
    mut sfx: MessageWriter<Sfx>,
    mut bursts: MessageWriter<Burst>,
    mut harm: MessageWriter<Harm>,
    mut shake: ResMut<Shake>,
) {
    let Some(world) = session.world.as_mut() else {
        return;
    };
    let dt = time.delta_secs();
    let positions: Vec<Vec2> = creatures.iter().map(|c| c.body.center()).collect();
    let mut impacts = Vec::new();
    for (entity, mut p, mut tf, mut sprite) in &mut q {
        p.age += dt;
        p.vel.y += p.gravity * dt;
        let delta = p.vel * dt;
        let steps = delta.length().ceil().max(1.0) as i32;
        let mut hit = None;
        for _ in 0..steps {
            let next = p.pos + delta / steps as f32;
            let m = world.material(next.x.floor() as i32, next.y.floor() as i32);
            // Creatures stop bolts too.
            let struck = positions.iter().any(|c| c.distance(next) < 5.0);
            if !m.is_open() || struck {
                hit = Some((next, m));
                break;
            }
            p.pos = next;
            spells::trail(p.spell, world, p.pos);
        }
        let expired = p.age > 2.5;
        tf.translation = to_world(p.pos, 5.5);
        tf.rotation = Quat::from_rotation_z(-p.vel.to_angle());
        if let Some(atlas) = &mut sprite.texture_atlas {
            let row = p.spell.school().map_or(8, |s| s.index());
            atlas.index = row * 8 + (p.age * 12.0) as usize % 4;
        }
        if hit.is_none() && !expired {
            continue;
        }
        commands.entity(entity).despawn();
        let (at, hit_mat) = hit.unwrap_or((p.pos, Material::Empty));
        impacts.push((p.spell, p.pos.lerp(at, 0.5), hit_mat));
    }
    for (spell, at, hit_mat) in impacts {
        run.influence(at);
        let mut c = spells::Ctx {
            world,
            commands: &mut commands,
            assets: &assets,
            run: &mut run,
            player: &mut player,
            harm: Vec::new(),
            bursts: Vec::new(),
            sfx: Vec::new(),
            shake: &mut shake,
            creatures: &positions,
        };
        spells::impact(spell, &mut c, at, hit_mat);
        c.flush(&mut harm, &mut bursts, &mut sfx);
    }
}

/// A ring of flame (Thermal Suit vent).
pub fn fire_burst(bursts: &mut MessageWriter<Burst>, at: Vec2) {
    bursts.write(
        Burst::new(at, Color::srgb(1.0, 0.6, 0.15))
            .count(40)
            .speed(110.0)
            .gravity(-40.0)
            .life(0.6)
            .size(1.5),
    );
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
    let Some(world) = session.world.as_mut() else {
        return;
    };
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
        sprite.color = if blink {
            Color::srgb(1.0, 0.4, 0.4)
        } else {
            Color::WHITE
        };
        if bomb.fuse <= 0.0 {
            commands.entity(entity).despawn();
            let (x, y) = (bomb.pos.x as i32, bomb.pos.y as i32 - 2);
            run.influence(bomb.pos);
            match bomb.kind {
                BombKind::Blast(r) => {
                    world.explode(x, y, r);
                    if run.has(ScrollId::VolatileCore) {
                        run.volatile_spots.push((x, y));
                    }
                    shake.add(0.9);
                    stop.0 = 0.08;
                    sfx.write(Sfx::at("explosion", bomb.pos));
                }
                BombKind::Ignite => {
                    for (cx, cy) in disc(x, y + 2, 3) {
                        world.ignite(cx, cy);
                        if world.material(cx, cy).is_open() && (cx + cy) % 2 == 0 {
                            world.set(cx, cy, Material::Fire);
                        }
                    }
                    sfx.write(Sfx::at("impact_pyro", bomb.pos));
                }
            }
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
    let Some(world) = session.world.as_mut() else {
        return;
    };
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
            bursts.write(
                Burst::new(sun.pos, Color::srgb(1.0, 0.9, 0.5))
                    .count(2)
                    .speed(30.0)
                    .gravity(-20.0)
                    .life(0.4),
            );
        }
        if sun.life <= 0.0 {
            commands.entity(entity).despawn();
        }
    }
}

pub fn update_light_orbs(
    mut commands: Commands,
    time: Res<Time>,
    mut session: ResMut<Session>,
    mut run: ResMut<Run>,
    player: Res<RunPlayer>,
    mut save: ResMut<SaveData>,
    mut q: Query<(Entity, &mut LightOrb, &mut Transform)>,
    mut sfx: MessageWriter<Sfx>,
) {
    let Some(world) = session.world.as_mut() else {
        return;
    };
    let dt = time.delta_secs();
    let t = time.elapsed_secs();
    let ignites = run.orbs_ignite();
    // Lightless: staff dimmed, no orbs, underground.
    if run.staff_dimmed && q.is_empty() && player.body.pos.y > 150.0 && run.is_playing() {
        run.stats.dark_time += dt;
        if run.stats.dark_time >= 60.0 {
            grant(&mut save, &mut run, AchievementId::Lightless, &mut sfx);
        }
    } else {
        run.stats.dark_time = 0.0;
    }
    for (entity, mut orb, mut tf) in &mut q {
        orb.life -= dt;
        orb.timer -= dt;
        if orb.life <= 0.0 {
            sfx.write(Sfx::at("orb_fade", orb.pos).volume(0.6));
            commands.entity(entity).despawn();
            continue;
        }
        if !orb.stuck {
            orb.vel *= 1.0 - 2.5 * dt;
            let next = orb.pos + orb.vel * dt;
            if world.material(next.x as i32, next.y as i32).is_solid_for_player() {
                orb.stuck = true;
            } else {
                orb.pos = next;
            }
            if orb.vel.length() < 6.0 {
                orb.stuck = true;
            }
        }
        // Drift and bob gently once it settles.
        let bob = Vec2::new(
            (t * 0.7 + orb.pos.x).sin() * 1.5,
            (t * 1.3 + orb.pos.y).sin() * 1.0,
        );
        // Now and then the light gutters.
        orb.flicker = if run.rng.chance(4) {
            0.35
        } else {
            (orb.flicker + dt * 3.0).min(1.0)
        };
        tf.translation = to_world(orb.pos + bob, 5.5);
        if ignites && orb.timer <= 0.0 {
            orb.timer = 0.5;
            let (x, y) = (orb.pos.x as i32, orb.pos.y as i32);
            for (cx, cy) in disc(x, y, 3) {
                if world.material(cx, cy).props().flammability > 0 && run.rng.chance(60) {
                    world.ignite(cx, cy);
                }
            }
        }
    }
}

const SHARD_MAGNET: f32 = 26.0;

pub fn update_shards(
    mut commands: Commands,
    time: Res<Time>,
    session: Res<Session>,
    player: Res<RunPlayer>,
    mut run: ResMut<Run>,
    mut q: Query<(Entity, &mut ShardPickup, &mut Transform)>,
    mut sfx: MessageWriter<Sfx>,
    mut bursts: MessageWriter<Burst>,
) {
    let Some(world) = &session.world else {
        return;
    };
    let dt = time.delta_secs();
    let me = player.body.center();
    for (entity, mut s, mut tf) in &mut q {
        s.age += dt;
        let to = me - s.pos;
        let d = to.length();
        if d < 4.0 && s.age > 0.3 {
            run.shards += s.value;
            commands.entity(entity).despawn();
            sfx.write(Sfx::at("shard_pickup", s.pos).pitch(0.2));
            bursts.write(
                Burst::new(s.pos, Color::srgb(0.5, 0.95, 1.0))
                    .count(4)
                    .speed(25.0)
                    .gravity(-20.0)
                    .life(0.3),
            );
            continue;
        }
        if d < SHARD_MAGNET && s.age > 0.3 && run.is_playing() {
            // Pulled in faster the closer it gets.
            s.vel = to / d * (60.0 + (SHARD_MAGNET - d) * 12.0);
            let step = s.vel * dt;
            s.pos += step;
        } else {
            s.vel.y = (s.vel.y + 300.0 * dt).min(200.0);
            s.vel.x *= 1.0 - 3.0 * dt;
            let next = s.pos + s.vel * dt;
            if world.material(next.x as i32, next.y as i32).is_open() {
                s.pos = next;
            } else {
                s.vel = Vec2::ZERO;
            }
        }
        let glint = 1.0 + (time.elapsed_secs() * 5.0 + s.pos.x).sin() * 0.1;
        tf.translation = to_world(s.pos, 5.2);
        tf.scale = Vec3::splat(glint);
    }
}

pub fn animate_props(
    time: Res<Time>,
    mut orbs: Query<&mut Sprite, With<LightOrb>>,
    mut props: Query<(&Prop, &mut Sprite), Without<LightOrb>>,
    mut displays: Query<(&DisplayedItem, &mut Transform)>,
    prop_pos: Query<&Prop>,
) {
    let t = time.elapsed_secs();
    let frame = (t * 10.0) as usize;
    for mut s in &mut orbs {
        if let Some(atlas) = &mut s.texture_atlas {
            atlas.index = props_frame::LIGHT_ORB + frame % 4;
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
