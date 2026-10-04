mod assets;
mod audio;
mod dev;
mod fx;
mod hud;
mod menu;
mod player;
mod render;
mod run;
mod session;
mod steam;
mod ui;

use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    Menu,
    /// Joining a multiplayer game.
    Connecting,
    /// Sandbox / multiplayer.
    InGame,
    /// Generating a roguelike run.
    Loading,
    /// Playing a roguelike run.
    Run,
}

fn main() {
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Descent to the Core".into(),
                        resolution: (1280, 800).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin { file_path: assets::ASSET_ROOT.into(), ..default() })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(EguiPlugin::default())
        .add_plugins(steam::SteamPlugin)
        .add_plugins((run::RunPlugin, fx::FxPlugin, audio::AudioPlugin, dev::DevPlugin))
        .add_systems(Startup, assets::load_assets)
        .add_systems(OnEnter(AppState::Menu), |mut m: ResMut<audio::MusicTrack>| *m = audio::MusicTrack::Menu)
        .add_systems(OnEnter(AppState::InGame), |mut m: ResMut<audio::MusicTrack>| *m = audio::MusicTrack::None)
        .add_systems(EguiPrimaryContextPass, ui::setup_ui.before(menu::menu_ui))
        .init_state::<AppState>()
        .insert_resource(ClearColor(Color::srgb(0.45, 0.62, 0.85)))
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .init_resource::<render::UiHasPointer>()
        .init_resource::<render::ParticlePool>()
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
        .add_systems(OnExit(AppState::InGame), (render::cleanup_world_view, render::reset_particle_pool))
        .add_systems(
            FixedUpdate,
            (session::step_world, session::drain_events)
                .chain()
                .run_if(in_state(AppState::InGame).and_then(resource_exists::<session::Session>)),
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
                render::draw_sim_particles,
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
        .run();
}
