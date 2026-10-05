mod assets;
mod audio;
mod avatars;
mod controls;
mod dev;
mod ending;
mod entities;
mod events;
mod fx;
mod hud;
mod lighting;
mod map;
mod menu;
mod overlays;
mod panels;
mod pipes;
mod placement;
mod plantart;
mod plants;
mod player;
mod render;
mod session;
mod settings;
mod sky;
mod steam;
mod ui;

use bevy::prelude::*;
use bevy_egui::{EguiPlugin, EguiPrimaryContextPass};

#[derive(States, Default, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppState {
    #[default]
    Menu,
    /// Joining a game or generating a world.
    Connecting,
    InGame,
}

fn main() {
    let in_game = || in_state(AppState::InGame).and_then(resource_exists::<session::Session>);
    let playing = || {
        in_state(AppState::InGame)
            .and_then(resource_exists::<session::Session>)
            .and_then(resource_exists::<player::LocalPlayer>)
    };
    App::new()
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Planet Terraforming".into(),
                        resolution: dev::window_size().unwrap_or((1280, 800)).into(),
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    file_path: assets::ASSET_ROOT.into(),
                    ..default()
                })
                .set(ImagePlugin::default_nearest()),
        )
        .add_plugins(EguiPlugin::default())
        .add_plugins(steam::SteamPlugin)
        .add_plugins((fx::FxPlugin, audio::AudioPlugin, dev::DevPlugin))
        .insert_resource(settings::Config::load())
        .init_state::<AppState>()
        .insert_resource(ClearColor(Color::srgb(0.03, 0.05, 0.06)))
        .insert_resource(Time::<Fixed>::from_hz(60.0))
        .init_resource::<render::UiHasPointer>()
        .init_resource::<render::ParticlePool>()
        .init_resource::<controls::Rebind>()
        .init_resource::<panels::Panels>()
        .init_resource::<panels::Focus>()
        .init_resource::<hud::Toasts>()
        .init_resource::<lighting::PointLights>()
        .init_resource::<entities::EntSprites>()
        .init_resource::<plants::PlantSprites>()
        .init_resource::<plantart::PlantArt>()
        .init_resource::<placement::Placement>()
        .init_resource::<overlays::Overlays>()
        .init_resource::<ending::Ending>()
        .init_resource::<map::MapView>()
        .add_systems(
            Startup,
            (assets::load_assets, render::spawn_camera, menu::setup_menu).chain(),
        )
        .add_systems(PreUpdate, controls::capture_binding)
        .add_systems(
            OnEnter(AppState::Menu),
            |mut m: ResMut<audio::MusicTrack>, mut a: ResMut<audio::Ambience>| {
                *m = audio::MusicTrack::Menu;
                *a = audio::Ambience(None, None);
            },
        )
        .add_systems(EguiPrimaryContextPass, ui::setup_ui.before(menu::menu_ui))
        .add_systems(
            EguiPrimaryContextPass,
            ui::ui_sounds_and_fade
                .after(menu::menu_ui)
                .after(panels::panels_ui),
        )
        // Session lifecycle, in any state.
        .add_systems(
            Update,
            (
                session::receive.run_if(resource_exists::<session::Session>),
                session::end_session.run_if(resource_exists::<session::EndSession>),
            )
                .chain(),
        )
        .add_systems(Last, session::save_on_exit)
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
            (
                player::spawn_local_player,
                render::spawn_world_view,
                lighting::setup_lighting,
                sky::spawn_sky,
                placement::spawn_ghost,
                pipes::setup_pipes,
            ),
        )
        .add_systems(
            OnExit(AppState::InGame),
            (
                render::cleanup_world_view,
                render::reset_particle_pool,
                entities::reset_entities,
                plants::reset_plants,
                pipes::reset_pipes,
                ending::reset_ending,
                map::reset_map,
                |mut commands: Commands| commands.remove_resource::<player::LocalPlayer>(),
            ),
        )
        .add_systems(FixedUpdate, session::step_world.run_if(in_game()))
        .add_systems(
            Update,
            (
                (
                    hud::toggle_pause,
                    hud::ingame_steam_events,
                    panels::find_focus,
                    panels::interact,
                    overlays::toggle_overlays,
                    pipes::drag_pipes,
                    player::use_tools,
                    player::move_player,
                    session::send,
                    entities::extrapolate,
                    events::present_events,
                    events::soundscape,
                    map::explore,
                    map::refresh,
                )
                    .chain(),
                (
                    avatars::sync_avatars,
                    avatars::animate_avatars,
                    entities::sync_entities,
                    entities::animate_entities,
                    plants::sync_plants,
                    plants::animate_plants,
                    placement::update_ghost,
                    pipes::sync_pipes,
                    ending::update_ship,
                    overlays::draw_overlay_shapes,
                    player::follow_camera,
                    render::upload_dirty_chunks,
                    render::draw_sim_particles,
                    sky::update_sky,
                    lighting::update_lighting,
                )
                    .chain(),
            )
                .chain()
                .after(session::receive)
                .before(session::end_session)
                .run_if(playing()),
        )
        .add_systems(
            EguiPrimaryContextPass,
            (
                overlays::overlay_labels,
                hud::hud_ui,
                map::prepare,
                panels::panels_ui,
                ending::ending_banner,
            )
                .chain()
                .run_if(playing()),
        )
        .run();
}
