mod eco_view;
mod hud;
mod menu;
mod player;
mod render;
mod session;
mod steam;

use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    Menu,
    Connecting,
    InGame,
    /// The alien ecosystem sandbox.
    Ecosystem,
}

fn main() {
    let eco_dev = eco_view::EcoDevArgs::from_args();
    let mut app = App::new();
    app.add_plugins(DefaultPlugins.set(WindowPlugin {
        primary_window: Some(Window {
            title: "SBCT".into(),
            resolution: (1280, 800).into(),
            ..default()
        }),
        ..default()
    }))
    .add_plugins(EguiPlugin::default())
    .add_plugins(steam::SteamPlugin)
    .init_state::<AppState>()
    .insert_resource(ClearColor(Color::srgb(0.45, 0.62, 0.85)))
    .insert_resource(Time::<Fixed>::from_hz(60.0))
    .init_resource::<render::UiHasPointer>()
    .init_resource::<hud::EscMenuOpen>()
    .add_systems(Startup, (render::spawn_camera, menu::setup_menu))
    // Session lifecycle, in any state.
    .add_systems(
        Update,
        (
            session::receive.run_if(resource_exists::<session::Session>),
            session::end_session.run_if(resource_exists::<session::EndSession>),
        )
            .chain(),
    )
    // Menu.
    .add_systems(Update, menu::menu_steam_events.run_if(in_state(AppState::Menu)))
    .add_systems(
        EguiPrimaryContextPass,
        menu::menu_ui.run_if(in_state(AppState::Menu)),
    )
    // Connecting.
    .add_systems(
        Update,
        menu::connecting_update
            .after(session::receive)
            .run_if(in_state(AppState::Connecting)),
    )
    .add_systems(
        EguiPrimaryContextPass,
        menu::connecting_ui.run_if(in_state(AppState::Connecting)),
    )
    // In game.
    .add_systems(
        OnEnter(AppState::InGame),
        (player::spawn_local_player, render::spawn_world_view),
    )
    .add_systems(OnExit(AppState::InGame), render::cleanup_world_view)
    .add_systems(
        FixedUpdate,
        session::step_world.run_if(in_state(AppState::InGame).and_then(resource_exists::<session::Session>)),
    )
    .add_systems(
        Update,
        (
            hud::toggle_esc_menu,
            hud::ingame_steam_events,
            player::use_tools,
            player::move_player,
            session::send,
            player::sync_avatars,
            player::follow_camera,
            render::upload_dirty_chunks,
        )
            .chain()
            .after(session::receive)
            .before(session::end_session)
            .run_if(
                in_state(AppState::InGame)
                    .and_then(resource_exists::<session::Session>)
                    .and_then(resource_exists::<player::LocalPlayer>),
            ),
    )
    .add_systems(
        EguiPrimaryContextPass,
        hud::hud_ui.run_if(
            in_state(AppState::InGame)
                .and_then(resource_exists::<session::Session>)
                .and_then(resource_exists::<player::LocalPlayer>),
        ),
    )
    // Alien ecosystem.
    .add_systems(OnEnter(AppState::Ecosystem), eco_view::enter_ecosystem)
    .add_systems(OnExit(AppState::Ecosystem), eco_view::exit_ecosystem)
    .add_systems(
        Update,
        (
            eco_view::eco_input,
            eco_view::eco_step,
            eco_view::eco_render,
            eco_view::eco_camera,
        )
            .chain()
            .run_if(in_state(AppState::Ecosystem).and_then(resource_exists::<eco_view::EcoScene>)),
    )
    .add_systems(
        EguiPrimaryContextPass,
        eco_view::eco_ui
            .run_if(in_state(AppState::Ecosystem).and_then(resource_exists::<eco_view::EcoScene>)),
    );
    if eco_dev.screenshot.is_some() || eco_dev.quit_after.is_some() {
        app.add_systems(Update, eco_view::eco_dev);
    }
    app.insert_resource(eco_dev);
    app.run();
}
