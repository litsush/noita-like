//! The running game session: the cell world, the colony, and the network role.
//!
//! The host is authoritative. It simulates the world and the colony, applies
//! actions from clients, and streams changes to them. Clients own only their
//! character's movement; everything else they hold is a replica.

use std::collections::HashMap;
use std::path::PathBuf;

use bevy::prelude::*;
use sbct_net::protocol::{self, ClientMsg, HostMsg, PROTOCOL_VERSION, PoseUpdate};
use sbct_net::{Delivery, NetEvent, PeerId, Transport};
use sbct_sim::World;
use sbct_sim::colony::actions::Action;
use sbct_sim::colony::geom::V2;
use sbct_sim::colony::save::SaveFile;
use sbct_sim::colony::{Colony, Event, Player, PlayerKey, Pose, PublicPlayer};
use sbct_sim::planetgen;

use crate::AppState;
use crate::menu::{Connecting, MenuState};
use crate::settings::data_dir;
use crate::steam::SteamClient;

/// How often poses and dirty chunks are sent (seconds).
const SEND_INTERVAL: f32 = 1.0 / 20.0;
/// How often changed entities, vitals and moving entities are sent.
const ENTITY_INTERVAL: f32 = 1.0 / 12.0;
const GLOBALS_INTERVAL: f32 = 0.25;
const AUTOSAVE_SECS: f32 = 180.0;
const CLIENT_TIMEOUT_SECS: f32 = 20.0;
/// Clients are sent moving entities and effects within this many cells.
const INTEREST_RADIUS: f32 = 520.0;
pub const MAX_PLAYERS: usize = 8;
/// Chunks simulated around each player.
const ACTIVE_CHUNKS: usize = 5;

pub enum Role {
    /// Singleplayer.
    Offline,
    Host,
    Client {
        host: PeerId,
    },
}

struct Peer {
    key: PlayerKey,
    /// The player revision last sent to them.
    you_rev: u32,
}

#[derive(Resource)]
pub struct Session {
    pub role: Role,
    transport: Option<Box<dyn Transport>>,
    /// This player's key.
    pub me: PlayerKey,
    /// Steam lobby, if any, for the invite button.
    pub lobby: Option<u64>,
    pub world: Option<World>,
    pub colony: Option<Colony>,
    peers: HashMap<PeerId, Peer>,
    /// Clients: public info about everyone online, as last sent by the host.
    roster: HashMap<PlayerKey, PublicPlayer>,
    roster_stamp: u64,
    meta_sent: u32,
    /// Events to present this frame (sounds, particles, toasts).
    pub events: Vec<Event>,
    /// Client only: chunks received during the initial download.
    pub chunks_received: usize,
    /// The file this world is saved to (hosts only).
    pub save_name: Option<String>,
    since_heard: f32,
    send_timer: f32,
    entity_timer: f32,
    globals_timer: f32,
    autosave_timer: f32,
}

pub fn saves_dir() -> PathBuf {
    data_dir().join("saves")
}

/// Saved worlds, newest first.
pub fn list_saves() -> Vec<String> {
    let mut saves: Vec<(std::time::SystemTime, String)> = std::fs::read_dir(saves_dir())
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|e| {
            let path = e.path();
            if path.extension()? != "sbct" {
                return None;
            }
            let time = e.metadata().ok()?.modified().ok()?;
            Some((time, path.file_stem()?.to_string_lossy().into_owned()))
        })
        .collect();
    saves.sort_by_key(|s| std::cmp::Reverse(s.0));
    saves.into_iter().map(|s| s.1).collect()
}

fn clean_save_name(name: &str) -> String {
    let s: String = name
        .chars()
        .filter(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'))
        .take(40)
        .collect();
    if s.trim().is_empty() {
        "world".into()
    } else {
        s.trim().to_string()
    }
}

impl Session {
    fn new(role: Role, transport: Option<Box<dyn Transport>>, me: PlayerKey, lobby: Option<u64>) -> Session {
        Session {
            role,
            transport,
            me,
            lobby,
            world: None,
            colony: None,
            peers: HashMap::new(),
            roster: HashMap::new(),
            roster_stamp: 0,
            meta_sent: 0,
            events: Vec::new(),
            chunks_received: 0,
            save_name: None,
            since_heard: 0.0,
            send_timer: 0.0,
            entity_timer: 0.0,
            globals_timer: 0.0,
            autosave_timer: 0.0,
        }
    }

    fn host_role(transport: &Option<Box<dyn Transport>>) -> Role {
        if transport.is_some() {
            Role::Host
        } else {
            Role::Offline
        }
    }

    /// Starts a new world. `transport` is `None` for singleplayer.
    pub fn host_new(
        transport: Option<Box<dyn Transport>>,
        me: PlayerKey,
        name: String,
        lobby: Option<u64>,
        seed: u64,
        save_name: &str,
    ) -> Session {
        let mut planet = planetgen::generate(seed);
        let mut colony = Colony::found(&mut planet);
        colony.join(me, name.clone());
        let mut s = Session::new(Self::host_role(&transport), transport, me, lobby);
        s.chunks_received = planet.world.chunks_x() * planet.world.chunks_y();
        s.world = Some(planet.world);
        s.colony = Some(colony);
        s.save_name = Some(clean_save_name(save_name));
        s
    }

    /// Loads a saved world and hosts it.
    pub fn host_saved(
        transport: Option<Box<dyn Transport>>,
        me: PlayerKey,
        name: String,
        lobby: Option<u64>,
        save_name: &str,
    ) -> Result<Session, String> {
        let path = saves_dir().join(format!("{save_name}.sbct"));
        let bytes = std::fs::read(&path).map_err(|e| format!("Couldn't read {}: {e}", path.display()))?;
        let (world, mut colony) = SaveFile::from_bytes(&bytes)
            .and_then(|f| f.restore())
            .map_err(|e| format!("Couldn't load {save_name}: {e}"))?;
        colony.join(me, name.clone());
        let mut s = Session::new(Self::host_role(&transport), transport, me, lobby);
        s.chunks_received = world.chunks_x() * world.chunks_y();
        s.world = Some(world);
        s.colony = Some(colony);
        s.save_name = Some(save_name.to_string());
        Ok(s)
    }

    /// Connects to a host over an established transport and says hello.
    pub fn client(
        mut transport: Box<dyn Transport>,
        host: PeerId,
        name: String,
        key: PlayerKey,
        lobby: Option<u64>,
    ) -> Session {
        let hello = ClientMsg::Hello {
            version: PROTOCOL_VERSION,
            name: name.clone(),
            key,
        };
        transport.send(host, Delivery::Reliable, &protocol::encode(&hello));
        Session::new(Role::Client { host }, Some(transport), key, lobby)
    }

    pub fn is_authority(&self) -> bool {
        !matches!(self.role, Role::Client { .. })
    }

    pub fn total_chunks(&self) -> usize {
        self.world.as_ref().map_or(0, |w| w.chunks_x() * w.chunks_y())
    }

    /// True once the client has the whole world (always true for hosts).
    pub fn is_loaded(&self) -> bool {
        self.world.is_some() && self.colony.is_some() && self.chunks_received >= self.total_chunks()
    }

    pub fn describe(&self) -> String {
        match (&self.role, &self.transport) {
            (Role::Offline, _) => "Singleplayer".into(),
            (_, Some(t)) => t.describe(),
            _ => String::new(),
        }
    }

    pub fn player(&self) -> Option<&Player> {
        self.colony.as_ref()?.players.get(&self.me)
    }

    /// Public info about everyone online (including this player).
    pub fn roster(&self) -> Vec<PublicPlayer> {
        match (&self.role, &self.colony) {
            (Role::Client { .. }, _) => self.roster.values().cloned().collect(),
            (_, Some(c)) => c.online().map(|p| p.public()).collect(),
            _ => Vec::new(),
        }
    }

    /// Does something to the colony as this player: applied directly on the
    /// host, sent to the host from a client.
    pub fn act(&mut self, action: Action) {
        match self.role {
            Role::Client { host } => {
                self.send(
                    host,
                    Delivery::Reliable,
                    &protocol::encode(&ClientMsg::Action(action)),
                );
            }
            _ => {
                let me = self.me;
                if let (Some(world), Some(colony)) = (&mut self.world, &mut self.colony)
                    && let Err(reason) = colony.apply(world, me, action)
                {
                    colony.toast(Some(me), reason, false);
                }
            }
        }
    }

    /// Records this player's pose (the client owns its own movement).
    pub fn set_pose(&mut self, pose: Pose) {
        let me = self.me;
        let authority = self.is_authority();
        if let Some(colony) = &mut self.colony {
            if authority {
                colony.set_pose(me, pose);
            } else if let Some(p) = colony.players.get_mut(&me) {
                p.pose = pose;
            }
        }
    }

    fn send(&mut self, to: PeerId, delivery: Delivery, bytes: &[u8]) {
        if let Some(t) = &mut self.transport {
            t.send(to, delivery, bytes);
        }
    }

    /// Writes the world to its save file. Hosts only.
    pub fn save(&mut self) -> Result<(), String> {
        let (Some(world), Some(colony), Some(name)) = (&self.world, &self.colony, &self.save_name) else {
            return Ok(());
        };
        if !self.is_authority() {
            return Ok(());
        }
        let dir = saves_dir();
        let bytes = SaveFile::capture(world, colony).to_bytes();
        let result = (|| -> std::io::Result<()> {
            std::fs::create_dir_all(&dir)?;
            let tmp = dir.join(format!("{name}.sbct.tmp"));
            std::fs::write(&tmp, bytes)?;
            std::fs::rename(tmp, dir.join(format!("{name}.sbct")))
        })();
        result.map_err(|e| format!("Couldn't save to {}: {e}", dir.display()))
    }

    // ---- host ------------------------------------------------------------

    fn host_handle(&mut self, from: PeerId, msg: ClientMsg) {
        match msg {
            ClientMsg::Hello { version, name, key } => {
                let reject = |s: &mut Session, reason: String| {
                    s.send(
                        from,
                        Delivery::Reliable,
                        &protocol::encode(&HostMsg::Reject { reason }),
                    );
                };
                if version != PROTOCOL_VERSION {
                    return reject(
                        self,
                        format!("version mismatch (host {PROTOCOL_VERSION}, you {version})"),
                    );
                }
                if self.peers.contains_key(&from) {
                    return;
                }
                if self.peers.len() + 1 >= MAX_PLAYERS {
                    return reject(self, format!("the colony is full ({MAX_PLAYERS} players)"));
                }
                if key == self.me || self.peers.values().any(|p| p.key == key) || key == 0 {
                    return reject(self, "that player is already here".into());
                }
                self.host_welcome(from, key, sanitize_name(&name));
            }
            ClientMsg::Pose(pose) => {
                if let (Some(peer), Some(colony)) = (self.peers.get(&from), &mut self.colony) {
                    colony.set_pose(peer.key, pose);
                }
            }
            ClientMsg::Action(action) => {
                let Some(key) = self.peers.get(&from).map(|p| p.key) else {
                    return;
                };
                if let (Some(world), Some(colony)) = (&mut self.world, &mut self.colony)
                    && let Err(reason) = colony.apply(world, key, action)
                {
                    colony.toast(Some(key), reason, false);
                }
            }
        }
    }

    fn host_welcome(&mut self, id: PeerId, key: PlayerKey, name: String) {
        let (Some(world), Some(colony)) = (&self.world, &mut self.colony) else {
            return;
        };
        colony.join(key, name.clone());
        colony.toast(None, format!("{name} joined the colony"), true);
        let mut out = vec![protocol::encode(&HostMsg::Welcome {
            key,
            width: world.width() as u32,
            height: world.height() as u32,
            seed: world.seed(),
            colony: Box::new(colony.clone()),
        })];
        for cy in 0..world.chunks_y() {
            for cx in 0..world.chunks_x() {
                out.push(protocol::encode(&HostMsg::Chunk {
                    cx: cx as u16,
                    cy: cy as u16,
                    data: world.encode_chunk(cx, cy),
                }));
            }
        }
        for bytes in out {
            self.send(id, Delivery::Reliable, &bytes);
        }
        info!("{name} joined ({id})");
        self.peers.insert(id, Peer { key, you_rev: 0 });
    }

    fn host_drop(&mut self, id: PeerId) {
        if let Some(peer) = self.peers.remove(&id)
            && let Some(colony) = &mut self.colony
        {
            let name = colony
                .players
                .get(&peer.key)
                .map_or(String::new(), |p| p.name.clone());
            colony.leave(peer.key);
            colony.toast(None, format!("{name} left"), false);
            info!("{name} left");
        }
    }

    fn host_send(&mut self, dt: f32) {
        let Some(world) = &mut self.world else { return };
        let dirty = world.take_net_dirty();
        let Some(colony) = &mut self.colony else { return };

        self.entity_timer += dt;
        let entities_due = self.entity_timer >= ENTITY_INTERVAL;
        if entities_due {
            self.entity_timer = 0.0;
        }
        self.globals_timer += dt;
        let globals_due = self.globals_timer >= GLOBALS_INTERVAL;
        if globals_due {
            self.globals_timer = 0.0;
        }

        // Events go to this player's presentation and to interested peers.
        let events = colony.take_events();
        let pipes = colony.take_pipe_changes();
        let (changes, removed) = if entities_due {
            colony.take_changes()
        } else {
            (Vec::new(), Vec::new())
        };
        let me = self.me;
        let relevant = |e: &Event, key: PlayerKey, at: V2| match e {
            Event::Toast { to, .. } => to.is_none_or(|k| k == key),
            Event::Fx { pos, .. } => *pos == V2::ZERO || pos.distance(at) < INTEREST_RADIUS,
        };
        let my_pos = colony.players.get(&me).map_or(V2::ZERO, |p| p.pose.pos);
        self.events
            .extend(events.iter().filter(|e| relevant(e, me, my_pos)).cloned());
        if self.peers.is_empty() {
            return;
        }

        let mut out: Vec<(Option<PeerId>, Delivery, Vec<u8>)> = Vec::new();
        for (cx, cy) in dirty {
            let msg = HostMsg::Chunk {
                cx: cx as u16,
                cy: cy as u16,
                data: self.world.as_ref().unwrap().encode_chunk(cx, cy),
            };
            out.push((None, Delivery::Reliable, protocol::encode(&msg)));
        }
        let poses: Vec<PoseUpdate> = colony
            .online()
            .map(|p| {
                let s = p.stats();
                PoseUpdate {
                    key: p.key,
                    pose: p.pose,
                    hp: (p.hp / s.max_hp * 255.0).clamp(0.0, 255.0) as u8,
                    o2: (p.o2 / s.o2_capacity * 255.0).clamp(0.0, 255.0) as u8,
                }
            })
            .collect();
        out.push((
            None,
            Delivery::Unreliable,
            protocol::encode(&HostMsg::Poses(poses)),
        ));

        // The roster, whenever anyone's public state changed.
        let stamp = colony.online().fold(colony.online().count() as u64, |acc, p| {
            acc.wrapping_mul(31).wrapping_add(p.key ^ p.public_rev as u64)
        });
        if stamp != self.roster_stamp {
            self.roster_stamp = stamp;
            let roster = colony.online().map(|p| p.public()).collect();
            out.push((
                None,
                Delivery::Reliable,
                protocol::encode(&HostMsg::Roster(roster)),
            ));
        }
        if !pipes.is_empty() {
            out.push((None, Delivery::Reliable, protocol::encode(&HostMsg::Pipes(pipes))));
        }
        if colony.meta_rev != self.meta_sent {
            self.meta_sent = colony.meta_rev;
            out.push((
                None,
                Delivery::Reliable,
                protocol::encode(&HostMsg::Meta(Box::new(colony.meta()))),
            ));
        }
        if !changes.is_empty() || !removed.is_empty() {
            let msg = HostMsg::Ents {
                upserts: changes,
                removed,
            };
            out.push((None, Delivery::Reliable, protocol::encode(&msg)));
        }
        if globals_due {
            out.push((
                None,
                Delivery::Unreliable,
                protocol::encode(&HostMsg::Globals(colony.globals())),
            ));
        }

        for (&id, peer) in self.peers.iter_mut() {
            let Some(p) = colony.players.get(&peer.key) else {
                continue;
            };
            if p.rev != peer.you_rev {
                peer.you_rev = p.rev;
                out.push((
                    Some(id),
                    Delivery::Reliable,
                    protocol::encode(&HostMsg::You(Box::new(p.clone()))),
                ));
            }
            let at = p.pose.pos;
            if entities_due {
                if let Some(v) = colony.vitals(peer.key) {
                    out.push((
                        Some(id),
                        Delivery::Unreliable,
                        protocol::encode(&HostMsg::Vitals(v)),
                    ));
                }
                let motion = colony.motion_near(at, INTEREST_RADIUS);
                if !motion.is_empty() {
                    out.push((
                        Some(id),
                        Delivery::Unreliable,
                        protocol::encode(&HostMsg::Motion(motion)),
                    ));
                }
            }
            let theirs: Vec<Event> = events
                .iter()
                .filter(|e| relevant(e, peer.key, at))
                .cloned()
                .collect();
            if !theirs.is_empty() {
                out.push((
                    Some(id),
                    Delivery::Reliable,
                    protocol::encode(&HostMsg::Events(theirs)),
                ));
            }
        }

        let ids: Vec<PeerId> = self.peers.keys().copied().collect();
        for (to, delivery, bytes) in out {
            match to {
                Some(id) => self.send(id, delivery, &bytes),
                None => {
                    for &id in &ids {
                        self.send(id, delivery, &bytes);
                    }
                }
            }
        }
    }

    // ---- client ----------------------------------------------------------

    /// Returns an error message if the session should end.
    fn client_handle(&mut self, msg: HostMsg) -> Result<(), String> {
        match msg {
            HostMsg::Welcome {
                key,
                width,
                height,
                seed,
                colony,
            } => {
                info!("Welcomed by host; downloading {width}x{height} world");
                self.me = key;
                self.world = Some(World::new(width as usize, height as usize, seed));
                let mut colony = *colony;
                // Only people the roster lists are online.
                if let Some(p) = colony.players.get_mut(&key) {
                    p.online = true;
                }
                self.colony = Some(colony);
            }
            HostMsg::Reject { reason } => return Err(format!("Host refused connection: {reason}")),
            HostMsg::Chunk { cx, cy, data } => {
                let world = self.world.as_mut().ok_or("host sent chunks before welcome")?;
                world
                    .decode_chunk(cx as usize, cy as usize, &data)
                    .map_err(|_| "host sent a corrupt chunk".to_string())?;
                self.chunks_received += 1;
            }
            _ if self.colony.is_none() => {}
            HostMsg::Roster(list) => {
                let colony = self.colony.as_mut().unwrap();
                for p in colony.players.values_mut() {
                    p.online = false;
                }
                self.roster.clear();
                for public in list {
                    let p = colony.players.entry(public.key).or_insert_with(|| {
                        Player::new(public.key, public.name.clone(), public.color, V2::ZERO)
                    });
                    p.online = true;
                    p.name = public.name.clone();
                    p.color = public.color;
                    if public.key != self.me {
                        p.suit = public.suit;
                        p.dead = public.dead.then_some(0.0);
                        // What they wear, so their stats can be worked out here too.
                        p.mods.clear();
                        p.equipped = [None; 8];
                        for (slot, m) in public.mods.iter().enumerate() {
                            if let Some((set, tier)) = m {
                                p.mods.insert(*set, *tier);
                                p.equipped[slot] = Some(*set);
                            }
                        }
                        p.gear.free_arm = public.arm;
                    }
                    self.roster.insert(public.key, public);
                }
            }
            HostMsg::Poses(list) => {
                let colony = self.colony.as_mut().unwrap();
                for u in list {
                    // Our own pose is ours to decide.
                    if u.key == self.me {
                        continue;
                    }
                    if let Some(p) = colony.players.get_mut(&u.key) {
                        p.pose = u.pose;
                        let s = p.stats();
                        p.hp = u.hp as f32 / 255.0 * s.max_hp;
                        p.o2 = u.o2 as f32 / 255.0 * s.o2_capacity;
                    }
                }
            }
            HostMsg::You(player) => {
                let colony = self.colony.as_mut().unwrap();
                let mut player = *player;
                player.online = true;
                // Keep our own pose unless the host moved us.
                if let Some(old) = colony.players.get(&player.key)
                    && old.warp == player.warp
                {
                    player.pose = old.pose;
                }
                colony.players.insert(player.key, player);
            }
            HostMsg::Vitals(v) => {
                let me = self.me;
                self.colony.as_mut().unwrap().apply_vitals(me, v);
            }
            HostMsg::Ents { upserts, removed } => {
                self.colony.as_mut().unwrap().apply_changes(upserts, removed);
            }
            HostMsg::Motion(motion) => self.colony.as_mut().unwrap().apply_motion(&motion),
            HostMsg::Globals(g) => self.colony.as_mut().unwrap().apply_globals(g),
            HostMsg::Meta(m) => self.colony.as_mut().unwrap().apply_meta(*m),
            HostMsg::Pipes(changes) => self.colony.as_mut().unwrap().apply_pipe_changes(&changes),
            HostMsg::Events(events) => self.events.extend(events),
        }
        Ok(())
    }
}

fn sanitize_name(name: &str) -> String {
    let name: String = name.chars().filter(|c| !c.is_control()).take(24).collect();
    if name.trim().is_empty() {
        "Terraformer".into()
    } else {
        name
    }
}

/// Why the session ended; shown on the main menu.
#[derive(Resource)]
pub struct EndSession(pub Option<String>);

/// Receives and handles everything from the network.
pub fn receive(mut session: ResMut<Session>, time: Res<Time<Real>>, mut commands: Commands) {
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
            (Role::Host, NetEvent::Disconnected(id)) => session.host_drop(id),
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

/// Sends state at a fixed rate: everything from the host, our pose from clients.
pub fn send(mut session: ResMut<Session>, time: Res<Time<Real>>) {
    let dt = time.delta_secs();
    session.send_timer += dt;
    if session.send_timer < SEND_INTERVAL {
        return;
    }
    let elapsed = std::mem::take(&mut session.send_timer);
    match session.role {
        Role::Offline | Role::Host => session.host_send(elapsed),
        Role::Client { host } => {
            if session.is_loaded()
                && let Some(pose) = session.player().map(|p| p.pose)
            {
                session.send(
                    host,
                    Delivery::Unreliable,
                    &protocol::encode(&ClientMsg::Pose(pose)),
                );
            }
        }
    }

    if session.is_authority() {
        session.autosave_timer += elapsed;
        if session.autosave_timer > AUTOSAVE_SECS {
            session.autosave_timer = 0.0;
            if let Err(e) = session.save() {
                warn!("{e}");
            }
        }
    }
}

/// Advances the cell simulation and the colony. Only the authority simulates.
pub fn step_world(mut session: ResMut<Session>, time: Res<Time>) {
    if !session.is_authority() {
        return;
    }
    let s = &mut *session;
    let (Some(world), Some(colony)) = (&mut s.world, &mut s.colony) else {
        return;
    };
    let centers: Vec<(i32, i32)> = colony.online().map(|p| p.pose.pos.cell()).collect();
    world.set_active_region(Some(&centers), ACTIVE_CHUNKS);
    world.step();
    colony.step(world, time.delta_secs());
}

/// A Steam lobby to join once the current session has been torn down
/// (accepting an invite while already in a game).
#[derive(Resource)]
pub struct PendingJoin(pub u64);

/// Tears the session down (saving a hosted world; dropping the transport
/// leaves the lobby and closes sockets) and returns to the main menu.
pub fn end_session(
    mut commands: Commands,
    end: Res<EndSession>,
    session: Option<ResMut<Session>>,
    mut menu: ResMut<MenuState>,
    mut next: ResMut<NextState<AppState>>,
    pending: Option<Res<PendingJoin>>,
    steam: Option<Res<SteamClient>>,
) {
    if let Some(reason) = &end.0 {
        menu.error = Some(reason.clone());
    }
    if let Some(mut session) = session
        && let Err(e) = session.save()
    {
        menu.error = Some(e);
    }
    commands.remove_resource::<EndSession>();
    commands.remove_resource::<Session>();
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

/// Saves a hosted world when the window closes.
pub fn save_on_exit(mut exit: MessageReader<AppExit>, session: Option<ResMut<Session>>) {
    if exit.read().next().is_some()
        && let Some(mut session) = session
        && let Err(e) = session.save()
    {
        warn!("{e}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sbct_net::tcp::{self, TcpClient, TcpHost};
    use sbct_sim::Material;
    use sbct_sim::colony::EntKind;
    use sbct_sim::colony::actions::{Container, InvOp};
    use sbct_sim::colony::geom::v2;
    use sbct_sim::colony::items::Item;

    fn app_with(session: Session) -> App {
        let mut app = App::new();
        app.add_plugins(MinimalPlugins)
            .insert_resource(session)
            .add_systems(Update, (receive, step_world, send).chain());
        app
    }

    fn pump(host: &mut App, client: &mut App, mut done: impl FnMut(&Session, &Session) -> bool, what: &str) {
        let start = std::time::Instant::now();
        loop {
            host.update();
            client.update();
            if done(
                host.world().resource::<Session>(),
                client.world().resource::<Session>(),
            ) {
                return;
            }
            assert!(start.elapsed().as_secs() < 20, "timed out waiting for {what}");
            std::thread::sleep(std::time::Duration::from_millis(4));
        }
    }

    fn pair() -> (App, App) {
        // Tests must not write into the real saves folder.
        // SAFETY: set before any thread reads it; every test sets the same value.
        unsafe { std::env::set_var("SBCT_SAVE_DIR", std::env::temp_dir().join("sbct-session-test")) };
        let host_transport = TcpHost::bind(0).unwrap();
        let port = host_transport
            .describe()
            .rsplit(' ')
            .next()
            .unwrap()
            .parse::<u16>()
            .unwrap();
        let host = app_with(Session::host_new(
            Some(Box::new(host_transport)),
            100,
            "host".into(),
            None,
            7,
            "test",
        ));
        let client_transport = TcpClient::connect(&format!("127.0.0.1:{port}")).unwrap();
        let client = app_with(Session::client(
            Box::new(client_transport),
            tcp::HOST_ID,
            "guest".into(),
            200,
            None,
        ));
        (host, client)
    }

    #[test]
    fn client_joins_plays_and_stays_in_sync() {
        let (mut host, mut client) = pair();
        pump(
            &mut host,
            &mut client,
            |_, c| c.is_loaded() && c.roster.len() == 2,
            "world download",
        );

        let (hs, cs) = (
            host.world().resource::<Session>(),
            client.world().resource::<Session>(),
        );
        assert_eq!(cs.me, 200);
        assert_eq!(hs.roster().len(), 2);
        let (hw, cw) = (hs.world.as_ref().unwrap(), cs.world.as_ref().unwrap());
        for cy in 0..hw.chunks_y() {
            for cx in 0..hw.chunks_x() {
                assert_eq!(hw.encode_chunk(cx, cy), cw.encode_chunk(cx, cy));
            }
        }
        let home = hs.colony.as_ref().unwrap().home;
        assert!(cs.colony.as_ref().unwrap().dome(home).is_some());

        // The guest takes seeds from the starter chest: the host applies it
        // and both sides agree on the result.
        let chest = Container::DomeChest(home, 0);
        let spot = {
            let c = hs.colony.as_ref().unwrap();
            let e = &c.ents[&home];
            v2(e.pos.x + 8.0, e.pos.y)
        };
        let mut pose = cs.player().unwrap().pose;
        pose.pos = spot;
        client.world_mut().resource_mut::<Session>().set_pose(pose);
        pump(
            &mut host,
            &mut client,
            |h, _| h.colony.as_ref().unwrap().players[&200].pose.pos == spot,
            "pose",
        );
        client
            .world_mut()
            .resource_mut::<Session>()
            .act(Action::Inv(InvOp::LootAll(chest)));
        pump(
            &mut host,
            &mut client,
            |h, c| {
                let seeds = |s: &Session| s.colony.as_ref().unwrap().players[&200].inv.count(Item::Seed(0));
                seeds(h) == 4
                    && seeds(c) == 4
                    && c.colony.as_ref().unwrap().dome(home).unwrap().1.chests[0].is_empty()
            },
            "loot to replicate",
        );
    }

    impl Session {
        fn test_dig(&mut self, x: i32, y: i32) {
            self.act(Action::Dig { x, y });
        }
    }

    #[test]
    fn digging_from_a_client_reaches_the_host() {
        let (mut host, mut client) = pair();
        pump(
            &mut host,
            &mut client,
            |_, c| c.is_loaded() && c.roster.len() == 2,
            "world download",
        );
        let (dx, dy) = {
            let c = host.world().resource::<Session>().colony.as_ref().unwrap();
            (
                c.profile.spawn_x + 160,
                c.profile.surface_at(c.profile.spawn_x + 160),
            )
        };
        let mut pose = client.world().resource::<Session>().player().unwrap().pose;
        pose.pos = v2(dx as f32, dy as f32);
        client.world_mut().resource_mut::<Session>().set_pose(pose);
        let start = std::time::Instant::now();
        loop {
            client.world_mut().resource_mut::<Session>().test_dig(dx, dy + 3);
            host.update();
            client.update();
            let h = host.world().resource::<Session>();
            let c = client.world().resource::<Session>();
            let inv = &c.colony.as_ref().unwrap().players[&200].inv;
            let got = inv.count(Item::Dirt) + inv.count(Item::Sand) + inv.count(Item::Stone);
            if h.world.as_ref().unwrap().material(dx, dy + 3) == Material::Empty
                && c.world.as_ref().unwrap().material(dx, dy + 3) == Material::Empty
                && got > 0
            {
                break;
            }
            assert!(start.elapsed().as_secs() < 20, "dig never replicated");
            std::thread::sleep(std::time::Duration::from_millis(4));
        }
        // Creatures the host spawns show up on the client with their motion.
        let id = {
            let mut s = host.world_mut().resource_mut::<Session>();
            let c = s.colony.as_mut().unwrap();
            c.spawn(
                v2(dx as f32 + 40.0, dy as f32 - 4.0),
                EntKind::Creature(sbct_sim::colony::creatures::Creature::new(
                    sbct_sim::colony::creatures::CreatureKind::Puffback,
                )),
            )
        };
        pump(
            &mut host,
            &mut client,
            |_, c| c.colony.as_ref().unwrap().ents.contains_key(&id),
            "creature",
        );
        // Leaving frees the slot and marks the player offline.
        let key = client.world().resource::<Session>().me;
        drop(client);
        let start = std::time::Instant::now();
        loop {
            host.update();
            let h = host.world().resource::<Session>();
            if !h.colony.as_ref().unwrap().players[&key].online {
                assert!(
                    h.colony.as_ref().unwrap().players.contains_key(&key),
                    "character is kept"
                );
                break;
            }
            assert!(start.elapsed().as_secs() < 10, "disconnect not noticed");
            std::thread::sleep(std::time::Duration::from_millis(4));
        }
    }

    #[test]
    fn hosted_worlds_save_and_load() {
        unsafe { std::env::set_var("SBCT_SAVE_DIR", std::env::temp_dir().join("sbct-session-test")) };
        let mut s = Session::host_new(None, 5, "solo".into(), None, 3, "My World!");
        assert_eq!(s.save_name.as_deref(), Some("My World"));
        s.colony.as_mut().unwrap().credits = 77;
        s.save().unwrap();
        assert!(list_saves().contains(&"My World".to_string()));
        let loaded = Session::host_saved(None, 5, "solo".into(), None, "My World").unwrap();
        assert_eq!(loaded.colony.as_ref().unwrap().credits, 77);
        assert!(loaded.player().is_some_and(|p| p.online));
        assert!(Session::host_saved(None, 5, "solo".into(), None, "nope").is_err());
        let _ = std::fs::remove_file(saves_dir().join("My World.sbct"));
    }
}
