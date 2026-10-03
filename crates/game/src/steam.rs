//! Bevy glue for the Steam client. The game still runs (LAN/singleplayer)
//! when Steam isn't available.

use bevy::prelude::*;
use sbct_net::steam::{Steam, SteamEvent};

#[derive(Resource, Deref)]
pub struct SteamClient(pub Steam);

/// Why Steam isn't available, if it isn't.
#[derive(Resource, Default)]
pub struct SteamStatus {
    pub error: Option<String>,
}

/// Steam events received this frame. Filled in `PreUpdate`, consumed in `Update`.
#[derive(Resource, Default)]
pub struct SteamInbox(pub Vec<SteamEvent>);

pub struct SteamPlugin;

impl Plugin for SteamPlugin {
    fn build(&self, app: &mut App) {
        let mut status = SteamStatus::default();
        match Steam::init() {
            Ok(steam) => {
                info!("Steam initialised as {}", steam.my_name());
                app.insert_resource(SteamClient(steam));
            }
            Err(e) => {
                warn!("Steam unavailable: {e}");
                status.error = Some(e);
            }
        }
        app.insert_resource(status)
            .init_resource::<SteamInbox>()
            .add_systems(PreUpdate, pump.run_if(resource_exists::<SteamClient>));
    }
}

fn pump(steam: Res<SteamClient>, mut inbox: ResMut<SteamInbox>) {
    steam.run_callbacks();
    inbox.0 = steam.poll_events();
}

/// Lobby id passed by Steam when the game is launched from an invite
/// (`+connect_lobby <id>`).
pub fn lobby_from_args() -> Option<u64> {
    let args: Vec<String> = std::env::args().collect();
    args.windows(2)
        .find(|w| w[0] == "+connect_lobby")
        .and_then(|w| w[1].parse().ok())
}
