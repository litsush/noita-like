//! Cosmetic feedback: particles that don't touch the simulation, screen
//! shake, and hit-stop. Gameplay code sends [`Burst`] messages and bumps
//! [`Shake`]; this module does the rest.

use bevy::prelude::*;
use sbct_sim::rng::Rng;

use crate::render::InGameEntity;

/// A puff of cosmetic particles at a point in cell coordinates.
#[derive(Message, Clone, Copy)]
pub struct Burst {
    pub pos: Vec2,
    pub color: Color,
    pub count: u32,
    /// Initial speed in cells per second.
    pub speed: f32,
    /// Gravity in cells/s² (negative floats upward, like embers or smoke).
    pub gravity: f32,
    pub life: f32,
    /// Preferred direction; zero for all around.
    pub dir: Vec2,
    pub size: f32,
}

impl Burst {
    pub fn new(pos: Vec2, color: Color) -> Burst {
        Burst {
            pos,
            color,
            count: 8,
            speed: 40.0,
            gravity: 160.0,
            life: 0.5,
            dir: Vec2::ZERO,
            size: 1.0,
        }
    }
    pub fn count(mut self, n: u32) -> Self {
        self.count = n;
        self
    }
    pub fn speed(mut self, s: f32) -> Self {
        self.speed = s;
        self
    }
    pub fn gravity(mut self, g: f32) -> Self {
        self.gravity = g;
        self
    }
    pub fn life(mut self, l: f32) -> Self {
        self.life = l;
        self
    }
    pub fn dir(mut self, d: Vec2) -> Self {
        self.dir = d;
        self
    }
    pub fn size(mut self, s: f32) -> Self {
        self.size = s;
        self
    }
}

#[derive(Component)]
struct FxParticle {
    vel: Vec2,
    gravity: f32,
    life: f32,
    max_life: f32,
}

/// Camera shake as "trauma" in [0, 1]; shake strength is trauma².
#[derive(Resource, Default)]
pub struct Shake {
    pub trauma: f32,
    pub enabled: bool,
}

impl Shake {
    pub fn add(&mut self, amount: f32) {
        self.trauma = (self.trauma + amount).min(1.0);
    }

    /// Current camera offset in cells.
    pub fn offset(&self, t: f32) -> Vec2 {
        if !self.enabled {
            return Vec2::ZERO;
        }
        let s = self.trauma * self.trauma * 6.0;
        Vec2::new(
            (t * 61.0).sin() + (t * 23.0).cos() * 0.5,
            (t * 53.0).cos() + (t * 37.0).sin() * 0.5,
        ) * s
    }
}

/// Brief freeze on big impacts: real seconds remaining.
#[derive(Resource, Default)]
pub struct HitStop(pub f32);

#[derive(Resource)]
struct FxRng(Rng);

const MAX_FX: usize = 1500;

pub struct FxPlugin;

impl Plugin for FxPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<Burst>()
            .insert_resource(Shake {
                trauma: 0.0,
                enabled: true,
            })
            .init_resource::<HitStop>()
            .insert_resource(FxRng(Rng::new(7)))
            .add_systems(Update, (spawn_bursts, update_particles, decay_shake));
    }
}

fn spawn_bursts(
    mut commands: Commands,
    mut bursts: MessageReader<Burst>,
    mut rng: ResMut<FxRng>,
    existing: Query<(), With<FxParticle>>,
) {
    let mut budget = MAX_FX.saturating_sub(existing.iter().count());
    for b in bursts.read() {
        for _ in 0..b.count {
            if budget == 0 {
                return;
            }
            budget -= 1;
            let r = &mut rng.0;
            let angle = if b.dir == Vec2::ZERO {
                r.next_f32() * std::f32::consts::TAU
            } else {
                b.dir.to_angle() + (r.next_f32() - 0.5) * 1.4
            };
            let speed = b.speed * (0.4 + r.next_f32() * 0.8);
            let vel = Vec2::from_angle(angle) * speed;
            let life = b.life * (0.6 + r.next_f32() * 0.7);
            let jitter = Vec2::new(r.next_f32() - 0.5, r.next_f32() - 0.5) * 2.0;
            commands.spawn((
                FxParticle {
                    vel,
                    gravity: b.gravity,
                    life,
                    max_life: life,
                },
                InGameEntity,
                Sprite::from_color(b.color, Vec2::splat(b.size)),
                Transform::from_xyz(b.pos.x + jitter.x, -(b.pos.y + jitter.y), 6.0),
            ));
        }
    }
}

fn update_particles(
    mut commands: Commands,
    time: Res<Time>,
    mut q: Query<(Entity, &mut FxParticle, &mut Transform, &mut Sprite)>,
) {
    let dt = time.delta_secs();
    for (e, mut p, mut tf, mut sprite) in &mut q {
        p.life -= dt;
        if p.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        p.vel.y += p.gravity * dt;
        p.vel *= 1.0 - 1.5 * dt;
        // Cell y grows downward; Bevy y grows upward.
        tf.translation.x += p.vel.x * dt;
        tf.translation.y -= p.vel.y * dt;
        sprite.color.set_alpha((p.life / p.max_life).clamp(0.0, 1.0));
    }
}

fn decay_shake(time: Res<Time<Real>>, mut shake: ResMut<Shake>, mut stop: ResMut<HitStop>) {
    shake.trauma = (shake.trauma - time.delta_secs() * 1.6).max(0.0);
    stop.0 = (stop.0 - time.delta_secs()).max(0.0);
}
