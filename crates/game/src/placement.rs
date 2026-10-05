//! The ghost shown while holding something that is placed in the world (a
//! dome kit or a machine): where it would go, and whether it can.

use bevy::prelude::*;
use sbct_sim::colony::domes;
use sbct_sim::colony::items::Item;

use crate::assets::GameAssets;
use crate::entities::machine_row;
use crate::player::LocalPlayer;
use crate::render::{InGameEntity, UiHasPointer, z};
use crate::session::Session;

#[derive(Component)]
pub struct Ghost;

/// What the ghost says, for the HUD.
#[derive(Resource, Default)]
pub struct Placement {
    pub active: bool,
    pub error: Option<&'static str>,
}

pub fn spawn_ghost(mut commands: Commands) {
    commands.spawn((
        Ghost,
        InGameEntity,
        Sprite::default(),
        Transform::from_xyz(0.0, 0.0, z::OVERLAY),
        Visibility::Hidden,
    ));
}

pub fn update_ghost(
    session: Res<Session>,
    local: Res<LocalPlayer>,
    assets: Res<GameAssets>,
    ui: Res<UiHasPointer>,
    mut placement: ResMut<Placement>,
    mut ghost: Single<(&mut Sprite, &mut Transform, &mut Visibility), With<Ghost>>,
) {
    let (sprite, tf, vis) = &mut *ghost;
    placement.active = false;
    placement.error = None;
    **vis = Visibility::Hidden;
    let (Some(world), Some(colony), Some(me)) = (&session.world, &session.colony, session.player()) else {
        return;
    };
    if ui.0 || !me.alive() {
        return;
    }
    let Some(item) = me.held().map(|s| s.item) else {
        return;
    };
    let at = local.cursor;
    let in_reach = me.center().distance(at) <= me.stats().place_reach;
    let (site, center, size) = match item {
        Item::DomeKit(kind) => {
            let (w, h) = kind.size();
            let site = colony.dome_site(world, kind, at);
            let (_, front) = assets.domes.get(&kind).cloned().unwrap_or_default();
            sprite.image = front;
            sprite.texture_atlas = None;
            sprite.custom_size = Some(Vec2::new(w as f32, h as f32));
            let pos = site.unwrap_or_else(|_| {
                // Show it resting on whatever is under the cursor anyway.
                let (x, y) = at.cell();
                let floor = sbct_sim::colony::build::ground_below(world, x, y - 20, 90).unwrap_or(y);
                sbct_sim::colony::geom::v2(x as f32, floor as f32)
            });
            let r = domes::footprint(kind, pos);
            (
                site.map(|_| ()),
                Vec2::new(pos.x.floor(), -(pos.y.floor() - h as f32 / 2.0)),
                r,
            )
        }
        Item::Machine(kind) => {
            let site = colony.machine_site(world, kind, at);
            let (row, _) = machine_row(kind);
            *sprite.as_mut() = assets.props.sprite(row * 4);
            let pos = site.unwrap_or(at);
            let r = sbct_sim::colony::build::machine_rect(kind, pos);
            (site.map(|_| ()), Vec2::new(pos.x, -(pos.y - 16.0)), r)
        }
        _ => return,
    };
    let _ = size;
    placement.active = true;
    let error = match site {
        Err(why) => Some(why),
        Ok(()) if !in_reach => Some("Out of reach"),
        Ok(()) => None,
    };
    placement.error = error;
    sprite.color = if error.is_some() {
        Color::srgba(1.0, 0.35, 0.3, 0.6)
    } else {
        Color::srgba(0.6, 1.0, 0.7, 0.7)
    };
    tf.translation = center.extend(z::OVERLAY);
    **vis = Visibility::Visible;
}
