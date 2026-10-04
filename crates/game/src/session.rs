//! The running game session: world, players, and the network role.
//!
//! The host is authoritative over the world. It simulates, applies edits
//! from clients, and streams changed chunks. Clients don't simulate cells;
//! they own only their player's movement and send it to the host.

use std::collections::HashMap;

use bevy::prelude::*;
use sbct_net::protocol::{self, ClientMsg, Edit, HostMsg, PROTOCOL_VERSION, PlayerState};
use sbct_net::{Delivery, NetEvent, PeerId, Transport};
use sbct_sim::{Material, World, worldgen};

use crate::AppState;
use crate::menu::{Connecting, MenuState};
use crate::player::LocalPlayer;
use crate::steam::SteamClient;

pub const WORLD_WIDTH: usize = 1536;
pub const WORLD_HEIGHT: usize = 768;
/// How often positions and dirty chunks are sent (seconds).
const SEND_INTERVAL: f32 = 1.0 / 20.0;
const CLIENT_TIMEOUT_SECS: f32 = 15.0;
const MAX_EDIT_RADIUS: u8 = 8;
/// Peer id used for the local player when there's no network (singleplayer).
pub const OFFLINE_ID: PeerId = 1;

pub enum Role {
    /// Singleplayer, or a host with no transport.
    Offline,
    Host,
    Client {
        host: PeerId,
    },
}

pub struct RemotePlayer {
    pub name: String,
    pub state: PlayerState,
}

#[derive(Resource)]
pub struct Session {
    pub role: Role,
    transport: Option<Box<dyn Transport>>,
    pub local_id: PeerId,
    pub local_name: String,
    /// Steam lobby, if any, for the invite button.
    pub lobby: Option<u64>,
    pub world: Option<World>,
    pub players: HashMap<PeerId, RemotePlayer>,
    pub spawn: Vec2,
    /// Draw the dark "underground" backdrop below the original surface.
    pub backdrop: bool,
    /// Client only: chunks received during the initial download.
    pub chunks_received: usize,
    since_heard: f32,
    send_timer: f32,
}

impl Session {
    /// Starts hosting. `transport` is `None` for singleplayer.
    pub fn host(
        transport: Option<Box<dyn Transport>>,
        local_id: PeerId,
        local_name: String,
        lobby: Option<u64>,
        seed: u64,
    ) -> Session {
        let world = worldgen::generate(WORLD_WIDTH, WORLD_HEIGHT, seed);
        let (sx, sy) = worldgen::find_spawn(&world, world.width() as i32 / 2);
        Session {
            role: if transport.is_some() {
                Role::Host
            } else {
                Role::Offline
            },
            transport,
            local_id,
            local_name,
            lobby,
            chunks_received: world.chunks_x() * world.chunks_y(),
            world: Some(world),
            players: HashMap::new(),
            spawn: Vec2::new(sx as f32, sy as f32),
            backdrop: true,
            since_heard: 0.0,
            send_timer: 0.0,
        }
    }

    /// A local-only session around an existing world (the roguelike).
    pub fn offline(world: World, spawn: Vec2, local_name: String) -> Session {
        Session {
            role: Role::Offline,
            transport: None,
            local_id: OFFLINE_ID,
            local_name,
            lobby: None,
            chunks_received: world.chunks_x() * world.chunks_y(),
            world: Some(world),
            players: HashMap::new(),
            spawn,
            backdrop: false,
            since_heard: 0.0,
            send_timer: 0.0,
        }
    }

    /// Connects to a host over an established transport and says hello.
    pub fn client(
        mut transport: Box<dyn Transport>,
        host: PeerId,
        local_name: String,
        lobby: Option<u64>,
    ) -> Session {
        let hello = ClientMsg::Hello {
            version: PROTOCOL_VERSION,
            name: local_name.clone(),
        };
        transport.send(host, Delivery::Reliable, &protocol::encode(&hello));
        Session {
            role: Role::Client { host },
            transport: Some(transport),
            local_id: 0,
            local_name,
            lobby,
            world: None,
            players: HashMap::new(),
            spawn: Vec2::ZERO,
            backdrop: true,
            chunks_received: 0,
            since_heard: 0.0,
            send_timer: 0.0,
        }
    }

    pub fn is_authority(&self) -> bool {
        !matches!(self.role, Role::Client { .. })
    }

    pub fn total_chunks(&self) -> usize {
        self.world.as_ref().map_or(0, |w| w.chunks_x() * w.chunks_y())
    }

    /// True once the client has the whole world (always true for hosts).
    pub fn is_loaded(&self) -> bool {
        self.world.is_some() && self.chunks_received >= self.total_chunks()
    }

    pub fn describe(&self) -> String {
        match (&self.role, &self.transport) {
            (Role::Offline, _) => "Singleplayer".into(),
            (_, Some(t)) => t.describe(),
            _ => String::new(),
        }
    }

    /// Paints locally and, on clients, asks the host to do the same.
    /// The client applies it immediately so digging feels responsive; the
    /// host's chunk updates overwrite it if they disagree.
    pub fn edit(&mut self, edit: Edit) {
        if let Some(world) = &mut self.world {
            world.paint_circle(
                edit.x,
                edit.y,
                edit.radius as i32,
                Material::from_u8(edit.material),
            );
        }
        if let Role::Client { host } = self.role {
            self.send(
                host,
                Delivery::Reliable,
                &protocol::encode(&ClientMsg::Edit(edit)),
            );
        }
    }

    fn send(&mut self, to: PeerId, delivery: Delivery, bytes: &[u8]) {
        if let Some(t) = &mut self.transport {
            t.send(to, delivery, bytes);
        }
    }

    fn broadcast(&mut self, msg: &HostMsg, delivery: Delivery, except: Option<PeerId>) {
        let bytes = protocol::encode(msg);
        let ids: Vec<PeerId> = self
            .players
            .keys()
            .copied()
            .filter(|&id| Some(id) != except)
            .collect();
        for id in ids {
            self.send(id, delivery, &bytes);
        }
    }

    // ---- host ------------------------------------------------------------

    fn host_handle(&mut self, from: PeerId, msg: ClientMsg) {
        match msg {
            ClientMsg::Hello { version, name } => {
                if version != PROTOCOL_VERSION {
                    let reason = format!("version mismatch (host {PROTOCOL_VERSION}, you {version})");
                    self.send(
                        from,
                        Delivery::Reliable,
                        &protocol::encode(&HostMsg::Reject { reason }),
                    );
                    return;
                }
                if self.players.contains_key(&from) {
                    return;
                }
                self.host_welcome(from, sanitize_name(&name));
            }
            // Ignore anything from peers who haven't said hello.
            _ if !self.players.contains_key(&from) => {}
            ClientMsg::State(state) => {
                if let Some(p) = self.players.get_mut(&from) {
                    p.state = state;
                }
            }
            ClientMsg::Edit(mut edit) => {
                edit.radius = edit.radius.min(MAX_EDIT_RADIUS);
                if let Some(world) = &mut self.world {
                    world.paint_circle(
                        edit.x,
                        edit.y,
                        edit.radius as i32,
                        Material::from_u8(edit.material),
                    );
                }
            }
        }
    }

    fn host_welcome(&mut self, id: PeerId, name: String) {
        let world = self.world.as_ref().expect("host always has a world");
        let mut out = vec![protocol::encode(&HostMsg::Welcome {
            your_id: id,
            width: world.width() as u32,
            height: world.height() as u32,
            seed: world.seed(),
            spawn: (self.spawn.x, self.spawn.y),
        })];
        for cy in 0..world.chunks_y() {
            for cx in 0..world.chunks_x() {
                let data = world.encode_chunk(cx, cy);
                out.push(protocol::encode(&HostMsg::Chunk {
                    cx: cx as u16,
                    cy: cy as u16,
                    data,
                }));
            }
        }
        out.push(protocol::encode(&HostMsg::PlayerJoined {
            id: self.local_id,
            name: self.local_name.clone(),
        }));
        for (&pid, p) in &self.players {
            out.push(protocol::encode(&HostMsg::PlayerJoined {
                id: pid,
                name: p.name.clone(),
            }));
        }
        for bytes in out {
            self.send(id, Delivery::Reliable, &bytes);
        }

        info!("{name} joined ({id})");
        self.broadcast(
            &HostMsg::PlayerJoined {
                id,
                name: name.clone(),
            },
            Delivery::Reliable,
            None,
        );
        let state = PlayerState {
            x: self.spawn.x,
            y: self.spawn.y,
            ..default()
        };
        self.players.insert(id, RemotePlayer { name, state });
    }

    fn host_send(&mut self, me: PlayerState) {
        let Some(world) = &mut self.world else { return };
        let dirty = world.take_net_dirty();
        if self.players.is_empty() {
            return;
        }
        for (cx, cy) in dirty {
            let data = self.world.as_ref().unwrap().encode_chunk(cx, cy);
            let msg = HostMsg::Chunk {
                cx: cx as u16,
                cy: cy as u16,
                data,
            };
            self.broadcast(&msg, Delivery::Reliable, None);
        }
        let mut states: Vec<(PeerId, PlayerState)> =
            self.players.iter().map(|(&id, p)| (id, p.state)).collect();
        states.push((self.local_id, me));
        self.broadcast(&HostMsg::Players(states), Delivery::Unreliable, None);
    }

    // ---- client ----------------------------------------------------------

    /// Returns an error message if the session should end.
    fn client_handle(&mut self, msg: HostMsg) -> Result<(), String> {
        match msg {
            HostMsg::Welcome {
                your_id,
                width,
                height,
                seed,
                spawn,
            } => {
                info!("Welcomed by host as {your_id}; downloading {width}x{height} world");
                self.local_id = your_id;
                self.spawn = Vec2::new(spawn.0, spawn.1);
                self.world = Some(World::new(width as usize, height as usize, seed));
            }
            HostMsg::Reject { reason } => return Err(format!("Host refused connection: {reason}")),
            HostMsg::Chunk { cx, cy, data } => {
                let world = self.world.as_mut().ok_or("host sent chunks before welcome")?;
                world
                    .decode_chunk(cx as usize, cy as usize, &data)
                    .map_err(|_| "host sent a corrupt chunk".to_string())?;
                self.chunks_received += 1;
            }
            HostMsg::PlayerJoined { id, name } => {
                if id != self.local_id {
                    self.players.insert(
                        id,
                        RemotePlayer {
                            name,
                            state: default(),
                        },
                    );
                }
            }
            HostMsg::PlayerLeft { id } => {
                self.players.remove(&id);
            }
            HostMsg::Players(states) => {
                for (id, state) in states {
                    if let Some(p) = self.players.get_mut(&id) {
                        p.state = state;
                    }
                }
            }
        }
        Ok(())
    }
}

fn sanitize_name(name: &str) -> String {
    let name: String = name.chars().filter(|c| !c.is_control()).take(32).collect();
    if name.trim().is_empty() {
        "Player".into()
    } else {
        name
    }
}

/// Why the session ended; shown on the main menu.
#[derive(Resource)]
pub struct EndSession(pub Option<String>);

/// Receives and handles everything from the network.
pub fn receive(mut session: ResMut<Session>, time: Res<Time>, mut commands: Commands) {
    let Some(transport) = &mut session.transport else {
        return;
    };
    let events = transport.poll();
    if !events.is_empty() {
        session.since_heard = 0.0;
    }

    for event in events {
        match (&session.role, event) {
            (Role::Host, NetEvent::Message(from, bytes)) => match protocol::decode::<ClientMsg>(&bytes) {
                Some(msg) => session.host_handle(from, msg),
                None => warn!("dropping malformed message from {from}"),
            },
            (Role::Host, NetEvent::Disconnected(id)) => {
                if let Some(p) = session.players.remove(&id) {
                    info!("{} left", p.name);
                    session.broadcast(&HostMsg::PlayerLeft { id }, Delivery::Reliable, None);
                }
            }
            (Role::Client { .. }, NetEvent::Message(_, bytes)) => {
                let result = protocol::decode::<HostMsg>(&bytes)
                    .ok_or_else(|| "host sent a malformed message".to_string())
                    .and_then(|msg| session.client_handle(msg));
                if let Err(reason) = result {
                    commands.insert_resource(EndSession(Some(reason)));
                    return;
                }
            }
            (Role::Client { .. }, NetEvent::Disconnected(_)) => {
                commands.insert_resource(EndSession(Some("Lost connection to host.".into())));
                return;
            }
            (Role::Offline, _) => {}
        }
    }

    if matches!(session.role, Role::Client { .. }) {
        session.since_heard += time.delta_secs();
        if session.since_heard > CLIENT_TIMEOUT_SECS {
            commands.insert_resource(EndSession(Some("Host stopped responding.".into())));
        }
    }
}

/// Sends our state at a fixed rate: chunks + all players from the host,
/// our own player from clients.
pub fn send(mut session: ResMut<Session>, time: Res<Time>, player: Option<Res<LocalPlayer>>) {
    session.send_timer += time.delta_secs();
    if session.send_timer < SEND_INTERVAL {
        return;
    }
    session.send_timer = 0.0;
    let me = player.map(|p| p.state).unwrap_or_default();
    match session.role {
        Role::Offline => {}
        Role::Host => session.host_send(me),
        Role::Client { host } => {
            if session.is_loaded() {
                session.send(
                    host,
                    Delivery::Unreliable,
                    &protocol::encode(&ClientMsg::State(me)),
                );
            }
        }
    }
}

/// Advances the cell simulation. Only the authority simulates.
pub fn step_world(mut session: ResMut<Session>) {
    if session.is_authority()
        && let Some(world) = &mut session.world
    {
        world.step();
    }
}

/// Sandbox worlds don't react to sim events yet; keep them from piling up.
pub fn drain_events(mut session: ResMut<Session>) {
    if let Some(world) = &mut session.world {
        world.take_events();
    }
}

/// A Steam lobby to join once the current session has been torn down
/// (accepting an invite while already in a game).
#[derive(Resource)]
pub struct PendingJoin(pub u64);

/// Tears the session down (dropping the transport leaves the lobby /
/// closes sockets) and returns to the main menu.
pub fn end_session(
    mut commands: Commands,
    end: Res<EndSession>,
    mut menu: ResMut<MenuState>,
    mut next: ResMut<NextState<AppState>>,
    pending: Option<Res<PendingJoin>>,
    steam: Option<Res<SteamClient>>,
) {
    if let Some(reason) = &end.0 {
        menu.error = Some(reason.clone());
    }
    commands.remove_resource::<EndSession>();
    commands.remove_resource::<Session>();
    commands.remove_resource::<LocalPlayer>();
    commands.remove_resource::<Connecting>();
    if let (Some(pending), Some(steam)) = (pending, steam) {
        commands.remove_resource::<PendingJoin>();
        steam.join_lobby(pending.0);
        commands.insert_resource(Connecting::JoiningLobby);
        next.set(AppState::Connecting);
    } else {
        next.set(AppState::Menu);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sbct_net::tcp::{self, TcpClient, TcpHost};

    fn app_with(session: Session) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(session)
            .add_systems(Update, (receive, send).chain());
        app
    }

    #[test]
    fn client_downloads_world_and_edits_reach_host() {
        let host_transport = TcpHost::bind(0).unwrap();
        let port = host_transport
            .describe()
            .rsplit(' ')
            .next()
            .unwrap()
            .parse::<u16>()
            .unwrap();
        let mut host = app_with(Session::host(
            Some(Box::new(host_transport)),
            tcp::HOST_ID,
            "host".into(),
            None,
            7,
        ));
        let client_transport = TcpClient::connect(&format!("127.0.0.1:{port}")).unwrap();
        let mut client = app_with(Session::client(
            Box::new(client_transport),
            tcp::HOST_ID,
            "guest".into(),
            None,
        ));

        let start = std::time::Instant::now();
        while !client.world().resource::<Session>().is_loaded() {
            assert!(start.elapsed().as_secs() < 10, "world download timed out");
            host.update();
            client.update();
            std::thread::sleep(std::time::Duration::from_millis(5));
        }

        let (hs, cs) = (
            host.world().resource::<Session>(),
            client.world().resource::<Session>(),
        );
        assert_eq!(hs.players.len(), 1);
        assert!(
            cs.players.contains_key(&tcp::HOST_ID),
            "client should know the host player"
        );
        let (hw, cw) = (hs.world.as_ref().unwrap(), cs.world.as_ref().unwrap());
        for cy in 0..hw.chunks_y() {
            for cx in 0..hw.chunks_x() {
                assert_eq!(hw.encode_chunk(cx, cy), cw.encode_chunk(cx, cy));
            }
        }

        // Dig a hole from the client; the host should apply it.
        let (x, y) = (100, WORLD_HEIGHT as i32 - 20);
        assert_ne!(hw.material(x, y), Material::Empty);
        client.world_mut().resource_mut::<Session>().edit(Edit {
            x,
            y,
            radius: 3,
            material: 0,
        });
        let start = std::time::Instant::now();
        loop {
            host.update();
            client.update();
            let hw = host.world().resource::<Session>().world.as_ref().unwrap();
            if hw.material(x, y) == Material::Empty {
                break;
            }
            assert!(start.elapsed().as_secs() < 5, "edit never reached host");
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
}
