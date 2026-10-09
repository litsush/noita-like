//! Alien Versus: design a species, drop it into a generated arena against
//! the other players' species, watch them fight, evolve, repeat.
//!
//! The host runs the arena and streams snapshots; everyone, host included,
//! watches through a mirror of it. See `sbct_sim::versus`.

pub mod icons;
pub mod session;
pub mod ui;
pub mod view;

use bevy::prelude::*;
use bevy_egui::EguiPrimaryContextPass;

pub use session::{Phase, Versus};

use crate::AppState;

pub struct VersusPlugin;

impl Plugin for VersusPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<view::VersusView>()
            .add_systems(
                Update,
                session::receive
                    .run_if(resource_exists::<Versus>)
                    .before(crate::session::end_session),
            )
            .add_systems(OnEnter(AppState::Versus), view::enter)
            .add_systems(OnExit(AppState::Versus), view::exit)
            .add_systems(
                Update,
                (session::update, view::update, view::camera)
                    .chain()
                    .after(session::receive)
                    .before(crate::session::end_session)
                    .run_if(in_state(AppState::Versus).and_then(resource_exists::<Versus>)),
            )
            .add_systems(
                EguiPrimaryContextPass,
                ui::ui.after(crate::theme::setup).run_if(
                    in_state(AppState::Versus)
                        .and_then(resource_exists::<Versus>)
                        .and_then(crate::theme::ready),
                ),
            );
    }
}
