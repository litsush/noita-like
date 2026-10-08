//! The versus match: roster, rounds, and the network role.
//!
//! The host owns the match state (who's ready, wins, points) and the arena.
//! Clients send a design when ready and receive the roster, round starts,
//! arena chunks, snapshots and events. Offline practice is a host with an
//! AI-designed opponent and no transport.

use bevy::prelude::*;
use sbct_net::protocol::{self, PROTOCOL_VERSION, RosterEntry, VersusClientMsg, VersusHostMsg};
use sbct_net::{Delivery, NetEvent, PeerId, Transport};
use sbct_sim::rng::Rng;
use sbct_sim::versus::design::{Design, ROUND_POINTS, START_POINTS};
use sbct_sim::versus::mirror::Mirror;
use sbct_sim::versus::{Arena, DT, FightEvent, ROUNDS_TO_WIN, RoundResult, SNAPSHOT_HZ};

use crate::session::EndSession;

const CLIENT_TIMEOUT_SECS: f32 = 15.0;
const ROSTER_INTERVAL: f32 = 2.0;
const CHUNK_INTERVAL: f32 = 0.1;
/// Seconds the host lets the finish play out before calling the round.
const ROUND_END_DELAY: f32 = 3.5;
/// The practice opponent's peer id.
pub const AI_ID: PeerId = u64::MAX - 1;

pub enum Role {
    /// Practice: host with no transport.
    Offline,
    Host,
    Client {
        host: PeerId,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    /// Client waiting for the host's welcome.
    Connecting,
    /// Designing and waiting for everyone to be ready.
    Lobby,
    Fight,
    Results,
}

#[derive(Clone, Debug)]
pub struct Slot {
    pub id: PeerId,
    pub name: String,
    pub design: Option<Design>,
    pub ready: bool,
    pub wins: u8,
    pub spectator: bool,
    pub ai: bool,
}

#[derive(Resource)]
pub struct Versus {
    pub role: Role,
    transport: Option<Box<dyn Transport>>,
    pub local_id: PeerId,
    pub local_name: String,
    pub lobby: Option<u64>,
    pub phase: Phase,
    pub round: u32,
    /// Points the local player has to spend.
    pub points: u32,
    /// Everyone in the match, host-authoritative. Clients mirror it.
    pub slots: Vec<Slot>,
    /// The local player's sheet, what it's locked to from the last round,
    /// and whether it's been submitted.
    pub design: Design,
    pub base: Option<Design>,
    pub ready: bool,
    /// Host only.
    pub arena: Option<Arena>,
    /// Everyone's view of the fight.
    pub mirror: Option<Mirror>,
    /// This round's teams in order: (player, name, design).
    pub teams: Vec<(PeerId, String, Design)>,
    pub result: Option<RoundResult>,
    /// Wins per team of the current round, for the results screen.
    pub wins: Vec<u8>,
    pub match_winner: Option<PeerId>,
    pub seed_override: Option<u64>,
    /// Skip the designer: auto-fill and ready up (testing).
    pub auto_ready: bool,
    snapshot_acc: f32,
    chunk_timer: f32,
    roster_timer: f32,
    since_heard: f32,
    rng: Rng,
    /// Rounds the practice AI has evolved through.
    ai_budget: u32,
}

impl Versus {
    fn fresh(
        role: Role,
        transport: Option<Box<dyn Transport>>,
        local_id: PeerId,
        local_name: String,
        lobby: Option<u64>,
    ) -> Versus {
        let mut rng = Rng::new(random_seed());
        let design = Design::new(rng.next_u64());
        Versus {
            role,
            transport,
            local_id,
            local_name,
            lobby,
            phase: Phase::Lobby,
            round: 1,
            points: START_POINTS,
            slots: Vec::new(),
            design,
            base: None,
            ready: false,
            arena: None,
            mirror: None,
            teams: Vec::new(),
            result: None,
            wins: Vec::new(),
            match_winner: None,
            seed_override: None,
            auto_ready: false,
            snapshot_acc: 0.0,
            chunk_timer: 0.0,
            roster_timer: 0.0,
            since_heard: 0.0,
            rng,
            ai_budget: START_POINTS,
        }
    }

    /// Hosts a match. `transport` is `None` for practice against the AI.
    pub fn host(
        transport: Option<Box<dyn Transport>>,
        local_id: PeerId,
        local_name: String,
        lobby: Option<u64>,
    ) -> Versus {
        let practice = transport.is_none();
        let role = if practice { Role::Offline } else { Role::Host };
        let mut v = Versus::fresh(role, transport, local_id, local_name.clone(), lobby);
        v.slots.push(Slot {
            id: local_id,
            name: local_name,
            design: None,
            ready: false,
            wins: 0,
            spectator: false,
            ai: false,
        });
        if practice {
            let ai = Design::random(&mut v.rng, START_POINTS, None);
            v.slots.push(Slot {
                id: AI_ID,
                name: "Wild species".into(),
                design: Some(ai),
                ready: true,
                wins: 0,
                spectator: false,
                ai: true,
            });
        }
        v
    }

    /// Joins a host over an established transport and says hello.
    pub fn client(
        mut transport: Box<dyn Transport>,
        host: PeerId,
        local_name: String,
        lobby: Option<u64>,
    ) -> Versus {
        let hello = VersusClientMsg::Hello {
            version: PROTOCOL_VERSION,
            name: local_name.clone(),
        };
        transport.send(host, Delivery::Reliable, &protocol::encode(&hello));
        let mut v = Versus::fresh(Role::Client { host }, Some(transport), 0, local_name, lobby);
        v.phase = Phase::Connecting;
        v
    }

    pub fn is_authority(&self) -> bool {
        !matches!(self.role, Role::Client { .. })
    }

    pub fn describe(&self) -> String {
        match (&self.role, &self.transport) {
            (Role::Offline, _) => "Practice".into(),
            (_, Some(t)) => t.describe(),
            _ => String::new(),
        }
    }

    /// Names of players who aren't ready yet.
    pub fn waiting_on(&self) -> Vec<String> {
        self.slots
            .iter()
            .filter(|s| !s.ready && !s.spectator)
            .map(|s| s.name.clone())
            .collect()
    }

    // ---- lobby -------------------------------------------------------------

    /// Submits the local design.
    pub fn set_ready(&mut self) {
        let mut d = self.design.clone();
        d.sanitize();
        self.design = d.clone();
        self.ready = true;
        if self.is_authority() {
            let id = self.local_id;
            if let Some(s) = self.slots.iter_mut().find(|s| s.id == id) {
                s.design = Some(d);
                s.ready = true;
                s.spectator = false;
            }
            self.send_roster();
        } else if let Role::Client { host } = self.role {
            self.send(
                host,
                Delivery::Reliable,
                &protocol::encode(&VersusClientMsg::Ready(d)),
            );
        }
    }

    pub fn unready(&mut self) {
        self.ready = false;
        if self.is_authority() {
            let id = self.local_id;
            if let Some(s) = self.slots.iter_mut().find(|s| s.id == id) {
                s.ready = false;
            }
            self.send_roster();
        } else if let Role::Client { host } = self.role {
            self.send(
                host,
                Delivery::Reliable,
                &protocol::encode(&VersusClientMsg::Unready),
            );
        }
    }

    /// Spends whatever points are left at random.
    pub fn auto_fill(&mut self) {
        let total = self.points;
        let base = self.base.clone().or_else(|| Some(self.design.clone()));
        let spend = total.saturating_sub(base.as_ref().map_or(0, |b| b.cost()));
        let mut d = Design::random(&mut self.rng, spend, base.as_ref());
        d.name = self.design.name.clone();
        d.look_seed = self.design.look_seed;
        self.design = d;
    }

    fn can_start(&self) -> bool {
        let players: Vec<&Slot> = self.slots.iter().filter(|s| !s.spectator).collect();
        players.len() >= 2 && players.iter().all(|s| s.ready && s.design.is_some())
    }

    fn send_roster(&mut self) {
        let roster = self.roster();
        self.broadcast(&VersusHostMsg::Roster(roster), Delivery::Reliable, None);
    }

    fn roster(&self) -> Vec<RosterEntry> {
        self.slots
            .iter()
            .map(|s| RosterEntry {
                id: s.id,
                name: s.name.clone(),
                ready: s.ready,
                wins: s.wins,
                spectator: s.spectator,
            })
            .collect()
    }

    /// Host: builds the arena from the ready designs and tells everyone.
    fn start_round(&mut self) {
        let teams: Vec<(PeerId, String, Design)> = self
            .slots
            .iter()
            .filter(|s| !s.spectator && s.ready)
            .filter_map(|s| s.design.clone().map(|d| (s.id, s.name.clone(), d)))
            .collect();
        if teams.len() < 2 {
            return;
        }
        let seed = self
            .seed_override
            .unwrap_or_else(|| self.rng.next_u64() % 1_000_000);
        let designs: Vec<(String, Design)> = teams.iter().map(|(_, n, d)| (n.clone(), d.clone())).collect();
        let arena = Arena::new(seed, &designs);
        let seed = arena.seed;
        info!("Round {} in {} (seed {seed})", self.round, arena.biome.name);
        self.teams = teams.clone();
        self.mirror = Some(Mirror::new(seed, &designs));
        self.result = None;
        self.phase = Phase::Fight;
        self.snapshot_acc = 0.0;
        self.chunk_timer = 0.0;
        self.broadcast(
            &VersusHostMsg::RoundStart {
                round: self.round,
                seed,
                teams,
            },
            Delivery::Reliable,
            None,
        );
        // The world is regenerated from the seed on every machine, but send
        // it anyway so the 90 settling steps can't drift.
        let chunks = all_chunks(&arena);
        self.arena = Some(arena);
        for (cx, cy, data) in chunks {
            self.apply_chunk(cx, cy, &data);
            self.broadcast(
                &VersusHostMsg::Chunk {
                    cx: cx as u16,
                    cy: cy as u16,
                    data,
                },
                Delivery::Reliable,
                None,
            );
        }
    }

    /// Host: the fight is over, hand out wins and points.
    fn finish_round(&mut self) {
        let Some(arena) = &self.arena else { return };
        let Some(result) = arena.result.clone() else {
            return;
        };
        if let Some(w) = result.winner
            && let Some((pid, _, _)) = self.teams.get(w as usize)
            && let Some(s) = self.slots.iter_mut().find(|s| s.id == *pid)
        {
            s.wins += 1;
        }
        let winner_id = self.slots.iter().find(|s| s.wins >= ROUNDS_TO_WIN).map(|s| s.id);
        let wins: Vec<u8> = self
            .teams
            .iter()
            .map(|(pid, _, _)| self.slots.iter().find(|s| s.id == *pid).map_or(0, |s| s.wins))
            .collect();
        let points = START_POINTS + ROUND_POINTS * self.round;
        self.broadcast(
            &VersusHostMsg::RoundOver {
                result: result.clone(),
                wins: wins.clone(),
                points,
                match_winner: winner_id,
            },
            Delivery::Reliable,
            None,
        );
        self.arena = None;
        self.round_over(result, wins, points, winner_id);
        // Everyone designs again; the practice AI evolves on its own.
        for s in self.slots.iter_mut() {
            s.ready = s.ai;
            if s.ai
                && let Some(d) = &s.design
            {
                s.design = Some(Design::random(&mut self.rng, ROUND_POINTS, Some(d)));
            }
        }
        self.ai_budget += ROUND_POINTS;
    }

    /// Everyone: switch to the results with an instant replay.
    fn round_over(&mut self, result: RoundResult, wins: Vec<u8>, points: u32, match_winner: Option<PeerId>) {
        self.result = Some(result);
        self.wins = wins;
        self.points = points;
        self.match_winner = match_winner;
        self.phase = Phase::Results;
        self.ready = false;
        if let Some(m) = &mut self.mirror {
            m.start_replay();
        }
    }

    /// Leaves the results screen for the designer (or a fresh match).
    pub fn leave_results(&mut self) {
        if self.match_winner.is_some() {
            self.new_match();
            return;
        }
        self.base = Some(self.design.clone());
        self.round += 1;
        self.phase = Phase::Lobby;
        self.mirror = None;
        if self.is_authority() {
            self.send_roster();
        }
    }

    /// Starts over with fresh designs and no wins. Host-driven; clients
    /// follow the next welcome.
    fn new_match(&mut self) {
        self.round = 1;
        self.points = START_POINTS;
        self.base = None;
        self.design = Design::new(self.rng.next_u64());
        self.phase = Phase::Lobby;
        self.mirror = None;
        self.match_winner = None;
        self.ready = false;
        if self.is_authority() {
            for s in self.slots.iter_mut() {
                s.wins = 0;
                s.spectator = false;
                s.ready = s.ai;
                s.design = if s.ai {
                    Some(Design::random(&mut self.rng, START_POINTS, None))
                } else {
                    None
                };
            }
            self.ai_budget = START_POINTS;
            self.broadcast(
                &VersusHostMsg::NewMatch { points: START_POINTS },
                Delivery::Reliable,
                None,
            );
            self.send_roster();
        }
    }

    pub fn skip_replay(&mut self) {
        if let Some(m) = &mut self.mirror {
            m.stop_replay();
        }
    }

    // ---- networking ----------------------------------------------------------

    fn send(&mut self, to: PeerId, delivery: Delivery, bytes: &[u8]) {
        if let Some(t) = &mut self.transport {
            t.send(to, delivery, bytes);
        }
    }

    fn broadcast(&mut self, msg: &VersusHostMsg, delivery: Delivery, except: Option<PeerId>) {
        if self.transport.is_none() {
            return;
        }
        let bytes = protocol::encode(msg);
        let ids: Vec<PeerId> = self
            .slots
            .iter()
            .filter(|s| !s.ai && s.id != self.local_id && Some(s.id) != except)
            .map(|s| s.id)
            .collect();
        for id in ids {
            self.send(id, delivery, &bytes);
        }
    }

    fn host_handle(&mut self, from: PeerId, msg: VersusClientMsg) {
        match msg {
            VersusClientMsg::Hello { version, name } => {
                if version != PROTOCOL_VERSION {
                    let reason = format!("version mismatch (host {PROTOCOL_VERSION}, you {version})");
                    self.send(
                        from,
                        Delivery::Reliable,
                        &protocol::encode(&VersusHostMsg::Reject { reason }),
                    );
                    return;
                }
                if self.slots.iter().any(|s| s.id == from) {
                    return;
                }
                let name = sanitize_name(&name);
                info!("{name} joined the match ({from})");
                let mid_fight = self.phase == Phase::Fight;
                self.slots.push(Slot {
                    id: from,
                    name,
                    design: None,
                    ready: false,
                    wins: 0,
                    spectator: mid_fight,
                    ai: false,
                });
                let points = START_POINTS + ROUND_POINTS * (self.round.saturating_sub(1));
                let mut out = vec![protocol::encode(&VersusHostMsg::Welcome {
                    your_id: from,
                    round: self.round,
                    points,
                })];
                out.push(protocol::encode(&VersusHostMsg::Roster(self.roster())));
                if mid_fight && let Some(arena) = &self.arena {
                    out.push(protocol::encode(&VersusHostMsg::RoundStart {
                        round: self.round,
                        seed: arena.seed,
                        teams: self.teams.clone(),
                    }));
                    for (cx, cy, data) in all_chunks(arena) {
                        out.push(protocol::encode(&VersusHostMsg::Chunk {
                            cx: cx as u16,
                            cy: cy as u16,
                            data,
                        }));
                    }
                }
                for bytes in out {
                    self.send(from, Delivery::Reliable, &bytes);
                }
                self.send_roster();
            }
            _ if !self.slots.iter().any(|s| s.id == from) => {}
            VersusClientMsg::Ready(mut design) => {
                design.sanitize();
                if let Some(s) = self.slots.iter_mut().find(|s| s.id == from) {
                    s.design = Some(design);
                    s.ready = true;
                    s.spectator = false;
                }
                self.send_roster();
            }
            VersusClientMsg::Unready => {
                if let Some(s) = self.slots.iter_mut().find(|s| s.id == from) {
                    s.ready = false;
                }
                self.send_roster();
            }
        }
    }

    fn client_handle(&mut self, msg: VersusHostMsg) -> Result<(), String> {
        match msg {
            VersusHostMsg::Welcome {
                your_id,
                round,
                points,
            } => {
                info!("Welcomed into the match as {your_id}");
                self.local_id = your_id;
                self.round = round;
                self.points = points;
                if self.phase == Phase::Connecting {
                    self.phase = Phase::Lobby;
                }
            }
            VersusHostMsg::Reject { reason } => return Err(format!("Host refused connection: {reason}")),
            VersusHostMsg::Roster(list) => {
                self.slots = list
                    .into_iter()
                    .map(|r| Slot {
                        id: r.id,
                        name: r.name,
                        design: None,
                        ready: r.ready,
                        wins: r.wins,
                        spectator: r.spectator,
                        ai: r.id == AI_ID,
                    })
                    .collect();
            }
            VersusHostMsg::RoundStart { round, seed, teams } => {
                self.round = round;
                let designs: Vec<(String, Design)> =
                    teams.iter().map(|(_, n, d)| (n.clone(), d.clone())).collect();
                self.mirror = Some(Mirror::new(seed, &designs));
                self.teams = teams;
                self.result = None;
                self.phase = Phase::Fight;
            }
            VersusHostMsg::Chunk { cx, cy, data } => {
                self.apply_chunk(cx as usize, cy as usize, &data);
            }
            VersusHostMsg::Snapshot(snap) => {
                if let Some(m) = &mut self.mirror {
                    m.apply(&snap);
                }
            }
            VersusHostMsg::Events(events) => {
                if let Some(m) = &mut self.mirror {
                    for e in &events {
                        m.apply_event(e);
                    }
                }
            }
            VersusHostMsg::RoundOver {
                result,
                wins,
                points,
                match_winner,
            } => {
                self.round_over(result, wins, points, match_winner);
            }
            VersusHostMsg::NewMatch { points } => {
                self.new_match();
                self.points = points;
            }
            VersusHostMsg::PlayerLeft { id } => {
                self.slots.retain(|s| s.id != id);
            }
        }
        Ok(())
    }

    fn apply_chunk(&mut self, cx: usize, cy: usize, data: &[u8]) {
        if let Some(m) = &mut self.mirror
            && m.apply_chunk(cx, cy, data).is_err()
        {
            warn!("corrupt arena chunk {cx},{cy}");
        }
    }

    // ---- per frame -----------------------------------------------------------

    /// Advances the match by `dt` real seconds.
    fn advance(&mut self, dt: f32) {
        if self.is_authority() {
            self.roster_timer += dt;
            if self.roster_timer > ROSTER_INTERVAL {
                self.roster_timer = 0.0;
                if self.phase == Phase::Lobby {
                    self.send_roster();
                }
            }
            if self.phase == Phase::Lobby {
                if self.auto_ready && !self.ready {
                    self.auto_fill();
                    self.set_ready();
                }
                if self.can_start() {
                    self.start_round();
                }
            }
            if self.phase == Phase::Fight {
                self.host_step(dt);
            }
        } else if self.phase == Phase::Lobby && self.auto_ready && !self.ready {
            self.auto_fill();
            self.set_ready();
        }
        if let Some(m) = &mut self.mirror {
            m.advance(dt);
        }
    }

    fn host_step(&mut self, dt: f32) {
        let Some(arena) = &mut self.arena else { return };
        let steps = arena.advance(dt);
        self.snapshot_acc += steps as f32 * DT;
        let mut snaps = Vec::new();
        if self.snapshot_acc >= 1.0 / SNAPSHOT_HZ || (steps > 0 && arena.over()) {
            self.snapshot_acc = 0.0;
            snaps.push(arena.snapshot());
        }
        let events: Vec<FightEvent> = arena.take_events();
        self.chunk_timer += dt;
        let mut chunks = Vec::new();
        if self.chunk_timer >= CHUNK_INTERVAL {
            self.chunk_timer = 0.0;
            for (cx, cy) in arena.world.take_net_dirty() {
                chunks.push((cx, cy, arena.world.encode_chunk(cx, cy)));
            }
        }
        let done = arena.over() && arena.over_for > ROUND_END_DELAY;
        for snap in snaps {
            if let Some(m) = &mut self.mirror {
                m.apply(&snap);
            }
            self.broadcast(&VersusHostMsg::Snapshot(snap), Delivery::Unreliable, None);
        }
        if !events.is_empty() {
            if let Some(m) = &mut self.mirror {
                for e in &events {
                    m.apply_event(e);
                }
            }
            self.broadcast(&VersusHostMsg::Events(events), Delivery::Reliable, None);
        }
        for (cx, cy, data) in chunks {
            self.apply_chunk(cx, cy, &data);
            self.broadcast(
                &VersusHostMsg::Chunk {
                    cx: cx as u16,
                    cy: cy as u16,
                    data,
                },
                Delivery::Reliable,
                None,
            );
        }
        if done {
            self.finish_round();
        }
    }
}

fn all_chunks(arena: &Arena) -> Vec<(usize, usize, Vec<u8>)> {
    let w = &arena.world;
    let mut out = Vec::new();
    for cy in 0..w.chunks_y() {
        for cx in 0..w.chunks_x() {
            out.push((cx, cy, w.encode_chunk(cx, cy)));
        }
    }
    out
}

fn sanitize_name(name: &str) -> String {
    let name: String = name.chars().filter(|c| !c.is_control()).take(32).collect();
    if name.trim().is_empty() {
        "Player".into()
    } else {
        name
    }
}

fn random_seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64);
    sbct_sim::rng::hash2(nanos, 11, 5)
}

/// Receives and handles everything from the network.
pub fn receive(mut versus: ResMut<Versus>, time: Res<Time>, mut commands: Commands) {
    let Some(transport) = &mut versus.transport else {
        return;
    };
    let events = transport.poll();
    if !events.is_empty() {
        versus.since_heard = 0.0;
    }
    for event in events {
        match (&versus.role, event) {
            (Role::Host, NetEvent::Message(from, bytes)) => match protocol::decode::<VersusClientMsg>(&bytes)
            {
                Some(msg) => versus.host_handle(from, msg),
                None => warn!("dropping malformed message from {from}"),
            },
            (Role::Host, NetEvent::Disconnected(id)) => {
                if let Some(i) = versus.slots.iter().position(|s| s.id == id) {
                    let s = versus.slots.remove(i);
                    info!("{} left the match", s.name);
                    versus.broadcast(&VersusHostMsg::PlayerLeft { id }, Delivery::Reliable, None);
                    versus.send_roster();
                }
            }
            (Role::Client { .. }, NetEvent::Message(_, bytes)) => {
                let result = protocol::decode::<VersusHostMsg>(&bytes)
                    .ok_or_else(|| "host sent a malformed message".to_string())
                    .and_then(|msg| versus.client_handle(msg));
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
    if matches!(versus.role, Role::Client { .. }) {
        versus.since_heard += time.delta_secs();
        if versus.since_heard > CLIENT_TIMEOUT_SECS {
            commands.insert_resource(EndSession(Some("Host stopped responding.".into())));
        }
    }
}

/// Runs the match forward each frame.
pub fn update(mut versus: ResMut<Versus>, time: Res<Time>) {
    versus.advance(time.delta_secs());
}
