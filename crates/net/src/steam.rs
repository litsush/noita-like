//! Steam integration: lobbies for matchmaking/invites and
//! ISteamNetworkingMessages for relayed P2P traffic (no port forwarding).
//!
//! Flow: the host creates a lobby and becomes its owner. A client joins the
//! lobby (browser, friend list, invite, or lobby id), then messages the lobby
//! owner directly. Leaving the lobby is how peers notice disconnects.

use std::sync::mpsc::{Receiver, Sender, channel};
use std::sync::{Arc, Mutex};

use steamworks::networking_types::{NetworkingIdentity, SendFlags};
use steamworks::{
    CallbackHandle, ChatMemberStateChange, Client, FriendFlags, GameLobbyJoinRequested, LobbyChatUpdate,
    LobbyId, LobbyType, SteamId,
};

use crate::{Delivery, NetEvent, PeerId, Transport};

/// Valve's public "Spacewar" test app. Replace with your own app id once
/// the game has a Steamworks page; lobby filtering keeps us apart until then.
pub const APP_ID: u32 = 480;
/// Lobby metadata key/value used to find our lobbies among every other
/// developer testing on app 480.
const GAME_KEY: &str = "sbct_game";
const GAME_VALUE: &str = "sbct-v1";
const CHANNEL: u32 = 0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Visibility {
    Public,
    FriendsOnly,
    InviteOnly,
}

#[derive(Clone, Debug)]
pub struct LobbyInfo {
    pub id: u64,
    pub name: String,
    pub members: usize,
    pub max_members: usize,
    /// "sandbox" or "versus".
    pub mode: String,
}

#[derive(Clone, Debug)]
pub struct FriendLobby {
    pub friend: String,
    pub lobby: u64,
}

#[derive(Debug)]
pub enum SteamEvent {
    LobbyCreated(Result<u64, String>),
    LobbyEntered(Result<u64, String>),
    LobbyList(Vec<LobbyInfo>),
    /// The user accepted an invite or chose "Join Game" in the friends list.
    JoinRequested(u64),
}

/// Owns the Steam client. Must be pumped with [`Steam::run_callbacks`] every frame.
pub struct Steam {
    client: Client,
    tx: Sender<SteamEvent>,
    rx: Mutex<Receiver<SteamEvent>>,
    _callbacks: Vec<CallbackHandle>,
}

impl Steam {
    pub fn init() -> Result<Steam, String> {
        let client = Client::init_app(APP_ID).map_err(|e| e.to_string())?;
        let (tx, rx) = channel();
        let join_tx = tx.clone();
        let join_cb = client.register_callback(move |req: GameLobbyJoinRequested| {
            let _ = join_tx.send(SteamEvent::JoinRequested(req.lobby_steam_id.raw()));
        });
        Ok(Steam {
            client,
            tx,
            rx: Mutex::new(rx),
            _callbacks: vec![join_cb],
        })
    }

    pub fn run_callbacks(&self) {
        self.client.run_callbacks();
    }

    pub fn poll_events(&self) -> Vec<SteamEvent> {
        self.rx.lock().unwrap().try_iter().collect()
    }

    pub fn my_id(&self) -> PeerId {
        self.client.user().steam_id().raw()
    }

    pub fn my_name(&self) -> String {
        self.client.friends().name()
    }

    /// `mode` tags the lobby ("sandbox" or "versus") so joiners know what
    /// they're getting into.
    pub fn create_lobby(&self, visibility: Visibility, max_members: u32, mode: &str) {
        let ty = match visibility {
            Visibility::Public => LobbyType::Public,
            Visibility::FriendsOnly => LobbyType::FriendsOnly,
            Visibility::InviteOnly => LobbyType::Private,
        };
        let (tx, client) = (self.tx.clone(), self.client.clone());
        let mode = mode.to_string();
        self.client
            .matchmaking()
            .create_lobby(ty, max_members.min(250), move |res| {
                let res = res.map(|lobby| {
                    let mm = client.matchmaking();
                    mm.set_lobby_data(lobby, GAME_KEY, GAME_VALUE);
                    let what = if mode == "versus" { "arena" } else { "world" };
                    mm.set_lobby_data(lobby, "name", &format!("{}'s {what}", client.friends().name()));
                    mm.set_lobby_data(lobby, "mode", &mode);
                    lobby.raw()
                });
                let _ = tx.send(SteamEvent::LobbyCreated(res.map_err(|e| e.to_string())));
            });
    }

    /// The mode tag of a lobby we're in or looking at.
    pub fn lobby_mode(&self, lobby: u64) -> String {
        self.client
            .matchmaking()
            .lobby_data(LobbyId::from_raw(lobby), "mode")
            .unwrap_or_else(|| "sandbox".into())
    }

    pub fn join_lobby(&self, lobby: u64) {
        let tx = self.tx.clone();
        self.client
            .matchmaking()
            .join_lobby(LobbyId::from_raw(lobby), move |res| {
                let res = res
                    .map(|l| l.raw())
                    .map_err(|_| "could not enter lobby (full or closed?)".into());
                let _ = tx.send(SteamEvent::LobbyEntered(res));
            });
    }

    pub fn leave_lobby(&self, lobby: u64) {
        self.client.matchmaking().leave_lobby(LobbyId::from_raw(lobby));
    }

    /// Asks Steam for public lobbies of this game; answers with `SteamEvent::LobbyList`.
    pub fn request_lobby_list(&self) {
        let (tx, client) = (self.tx.clone(), self.client.clone());
        let mm = self.client.matchmaking();
        mm.add_request_lobby_list_string_filter(steamworks::StringFilter(
            steamworks::LobbyKey::new(GAME_KEY),
            GAME_VALUE,
            steamworks::StringFilterKind::Equal,
        ));
        mm.set_request_lobby_list_distance_filter(steamworks::DistanceFilter::Worldwide);
        mm.request_lobby_list(move |res| {
            let mm = client.matchmaking();
            let list = res
                .unwrap_or_default()
                .into_iter()
                .map(|l| LobbyInfo {
                    id: l.raw(),
                    name: mm.lobby_data(l, "name").unwrap_or_else(|| "Unnamed".into()),
                    members: mm.lobby_member_count(l),
                    max_members: mm.lobby_member_limit(l).unwrap_or(0),
                    mode: mm.lobby_data(l, "mode").unwrap_or_else(|| "sandbox".into()),
                })
                .collect();
            let _ = tx.send(SteamEvent::LobbyList(list));
        });
    }

    /// Friends currently in a lobby of this game (covers friends-only lobbies,
    /// which never show up in the public lobby search).
    pub fn friend_lobbies(&self) -> Vec<FriendLobby> {
        self.client
            .friends()
            .get_friends(FriendFlags::IMMEDIATE)
            .into_iter()
            .filter_map(|f| {
                let game = f.game_played()?;
                (game.game.app_id().0 == APP_ID && game.lobby.raw() != 0).then(|| FriendLobby {
                    friend: f.name(),
                    lobby: game.lobby.raw(),
                })
            })
            .collect()
    }

    pub fn lobby_owner(&self, lobby: u64) -> PeerId {
        self.client
            .matchmaking()
            .lobby_owner(LobbyId::from_raw(lobby))
            .raw()
    }

    pub fn open_invite_dialog(&self, lobby: u64) {
        self.client
            .friends()
            .activate_invite_dialog(LobbyId::from_raw(lobby));
    }

    /// Creates the message transport for a lobby we are already in.
    /// `host` is the lobby owner's id (our own id when hosting).
    pub fn transport(&self, lobby: u64, host: PeerId) -> SteamTransport {
        SteamTransport::new(self.client.clone(), LobbyId::from_raw(lobby), host)
    }
}

pub struct SteamTransport {
    client: Client,
    lobby: LobbyId,
    host: PeerId,
    is_host: bool,
    disconnects: Arc<Mutex<Vec<PeerId>>>,
    _callbacks: Vec<CallbackHandle>,
}

impl SteamTransport {
    fn new(client: Client, lobby: LobbyId, host: PeerId) -> SteamTransport {
        let is_host = client.user().steam_id().raw() == host;

        // Only accept P2P sessions from people who are in our lobby.
        let accept_client = client.clone();
        client.networking_messages().session_request_callback(move |req| {
            let from = req.remote().steam_id();
            let members = accept_client.matchmaking().lobby_members(lobby);
            if from.is_some_and(|id| members.contains(&id)) {
                req.accept();
            } else {
                req.reject();
            }
        });

        let disconnects: Arc<Mutex<Vec<PeerId>>> = Default::default();
        let d = disconnects.clone();
        let chat_cb = client.register_callback(move |u: LobbyChatUpdate| {
            if u.lobby != lobby {
                return;
            }
            let gone = matches!(
                u.member_state_change,
                ChatMemberStateChange::Left
                    | ChatMemberStateChange::Disconnected
                    | ChatMemberStateChange::Kicked
                    | ChatMemberStateChange::Banned
            );
            // Hosts care about everyone; clients only about the host.
            if gone && (is_host || u.user_changed.raw() == host) {
                d.lock().unwrap().push(u.user_changed.raw());
            }
        });

        SteamTransport {
            client,
            lobby,
            host,
            is_host,
            disconnects,
            _callbacks: vec![chat_cb],
        }
    }

    pub fn lobby(&self) -> u64 {
        self.lobby.raw()
    }
}

impl Transport for SteamTransport {
    fn send(&mut self, to: PeerId, delivery: Delivery, data: &[u8]) {
        let flags = match delivery {
            Delivery::Reliable => SendFlags::RELIABLE,
            Delivery::Unreliable => SendFlags::UNRELIABLE_NO_DELAY,
        } | SendFlags::AUTO_RESTART_BROKEN_SESSION;
        let identity = NetworkingIdentity::new_steam_id(SteamId::from_raw(to));
        // Failures surface as lobby leave events; nothing useful to do here.
        let _ = self
            .client
            .networking_messages()
            .send_message_to_user(identity, flags, data, CHANNEL);
    }

    fn poll(&mut self) -> Vec<NetEvent> {
        let nm = self.client.networking_messages();
        let mut events = Vec::new();
        loop {
            let batch = nm.receive_messages_on_channel(CHANNEL, 128);
            if batch.is_empty() {
                break;
            }
            for msg in batch {
                if let Some(id) = msg.identity_peer().steam_id() {
                    // Clients only accept traffic from the host.
                    if self.is_host || id.raw() == self.host {
                        events.push(NetEvent::Message(id.raw(), msg.data().to_vec()));
                    }
                }
            }
        }
        events.extend(
            self.disconnects
                .lock()
                .unwrap()
                .drain(..)
                .map(NetEvent::Disconnected),
        );
        events
    }

    fn describe(&self) -> String {
        format!("Steam lobby {}", self.lobby.raw())
    }
}

impl Drop for SteamTransport {
    fn drop(&mut self) {
        self.client.matchmaking().leave_lobby(self.lobby);
    }
}
