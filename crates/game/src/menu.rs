//! Main menu (host / join / singleplayer) and the connecting screen.

use std::sync::Mutex;
use std::sync::mpsc::{Receiver, channel};

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use sbct_net::steam::{FriendLobby, LobbyInfo, SteamEvent, Visibility};
use sbct_net::tcp::{self, TcpClient, TcpHost};

use crate::AppState;
use crate::session::{EndSession, OFFLINE_ID, Session};
use crate::steam::{SteamClient, SteamInbox, SteamStatus};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Main,
    Host,
    Join,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NetMode {
    Steam,
    Lan,
}

#[derive(Resource)]
pub struct MenuState {
    pub screen: Screen,
    pub error: Option<String>,
    pub name: String,
    pub host_mode: NetMode,
    pub visibility: Visibility,
    pub max_players: u32,
    pub port: String,
    pub seed: String,
    pub join_mode: NetMode,
    pub address: String,
    pub lobby_id: String,
    pub lobbies: Vec<LobbyInfo>,
    pub friends: Vec<FriendLobby>,
    pub searching: bool,
    friends_timer: f32,
}

impl MenuState {
    pub fn new(steam: Option<&SteamClient>) -> MenuState {
        let mode = if steam.is_some() {
            NetMode::Steam
        } else {
            NetMode::Lan
        };
        MenuState {
            screen: Screen::Main,
            error: None,
            name: steam.map_or_else(|| "Player".into(), |s| s.my_name()),
            host_mode: mode,
            visibility: Visibility::FriendsOnly,
            max_players: 4,
            port: tcp::DEFAULT_PORT.to_string(),
            seed: random_seed().to_string(),
            join_mode: mode,
            address: "127.0.0.1".into(),
            lobby_id: String::new(),
            lobbies: Vec::new(),
            friends: Vec::new(),
            searching: false,
            friends_timer: 0.0,
        }
    }
}

fn random_seed() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos() as u64);
    sbct_sim::rng::hash2(nanos, 0, 0) % 1_000_000
}

/// A connection in progress. Removed when entering the game or the menu.
#[derive(Resource)]
pub enum Connecting {
    CreatingLobby {
        seed: u64,
    },
    JoiningLobby,
    Tcp(Mutex<Receiver<Result<TcpClient, String>>>),
    /// Transport is up; waiting for the welcome and the world download.
    Downloading,
}

enum Action {
    Singleplayer,
    HostSteam,
    HostLan,
    JoinSteam(u64),
    JoinLan,
    Quit,
}

pub fn setup_menu(mut commands: Commands, steam: Option<Res<SteamClient>>, mut exit: MessageWriter<AppExit>) {
    let mut menu = MenuState::new(steam.as_deref());
    // Steam passes `+connect_lobby <id>` when launching the game from an invite.
    if let (Some(steam), Some(lobby)) = (&steam, crate::steam::lobby_from_args()) {
        steam.join_lobby(lobby);
        commands.insert_resource(Connecting::JoiningLobby);
        commands.set_state(AppState::Connecting);
    } else if let Some(action) = action_from_args(&mut menu) {
        run_action(action, &mut commands, &mut menu, steam.as_deref(), &mut exit);
    }
    commands.insert_resource(menu);
}

/// Dev shortcuts that skip the menu: `--singleplayer`, `--host-lan [port]`,
/// `--join-lan <addr>`, plus `--name <name>` and `--seed <n>`.
fn action_from_args(menu: &mut MenuState) -> Option<Action> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let value = |flag: &str| {
        let i = args.iter().position(|a| a == flag)?;
        args.get(i + 1).filter(|v| !v.starts_with("--")).cloned()
    };
    if let Some(name) = value("--name") {
        menu.name = name;
    }
    if let Some(seed) = value("--seed") {
        menu.seed = seed;
    }
    if args.iter().any(|a| a == "--singleplayer") {
        Some(Action::Singleplayer)
    } else if args.iter().any(|a| a == "--host-lan") {
        if let Some(port) = value("--host-lan") {
            menu.port = port;
        }
        Some(Action::HostLan)
    } else if let Some(addr) = value("--join-lan") {
        menu.address = addr;
        Some(Action::JoinLan)
    } else {
        None
    }
}

/// Handles Steam events while sitting in the menu: invites, lobby search
/// results, and stray lobby joins from a cancelled connection.
pub fn menu_steam_events(
    mut commands: Commands,
    mut inbox: ResMut<SteamInbox>,
    steam: Option<Res<SteamClient>>,
    mut menu: ResMut<MenuState>,
    time: Res<Time>,
) {
    let Some(steam) = steam else { return };
    for event in std::mem::take(&mut inbox.0) {
        match event {
            SteamEvent::LobbyList(list) => {
                menu.lobbies = list;
                menu.searching = false;
            }
            SteamEvent::JoinRequested(lobby) => {
                steam.join_lobby(lobby);
                commands.insert_resource(Connecting::JoiningLobby);
                commands.set_state(AppState::Connecting);
            }
            SteamEvent::LobbyCreated(Ok(lobby)) | SteamEvent::LobbyEntered(Ok(lobby)) => {
                steam.leave_lobby(lobby);
            }
            _ => {}
        }
    }
    if menu.screen == Screen::Join {
        menu.friends_timer -= time.delta_secs();
        if menu.friends_timer <= 0.0 {
            menu.friends_timer = 2.0;
            menu.friends = steam.friend_lobbies();
        }
    }
}

/// A root `Ui` covering the whole window, for full-screen panels.
fn screen_ui(ctx: &egui::Context) -> egui::Ui {
    egui::Ui::new(
        ctx.clone(),
        "screen".into(),
        egui::UiBuilder::new()
            .layer_id(egui::LayerId::background())
            .max_rect(ctx.viewport_rect()),
    )
}

const BUTTON: [f32; 2] = [260.0, 44.0];

fn big_button(ui: &mut egui::Ui, text: &str) -> bool {
    ui.add_sized(BUTTON, egui::Button::new(egui::RichText::new(text).size(20.0)))
        .clicked()
}

pub fn menu_ui(
    mut contexts: EguiContexts,
    mut commands: Commands,
    mut menu: ResMut<MenuState>,
    steam: Option<Res<SteamClient>>,
    status: Res<SteamStatus>,
    mut exit: MessageWriter<AppExit>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let mut action = None;

    let mut root = screen_ui(ctx);
    egui::CentralPanel::default().show(&mut root, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(50.0);
            ui.label(
                egui::RichText::new("SBCT")
                    .size(72.0)
                    .strong()
                    .color(egui::Color32::from_rgb(230, 170, 70)),
            );
            ui.label(egui::RichText::new("a falling-sand sandbox").size(16.0).italics());
            ui.add_space(8.0);
            match (&steam, &status.error) {
                (Some(s), _) => ui.colored_label(
                    egui::Color32::LIGHT_GREEN,
                    format!("Steam: signed in as {}", s.my_name()),
                ),
                (None, Some(e)) => ui.colored_label(
                    egui::Color32::from_rgb(230, 120, 90),
                    format!("Steam unavailable (LAN only): {e}"),
                ),
                _ => ui.label(""),
            };
            if let Some(err) = menu.error.clone() {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.colored_label(egui::Color32::from_rgb(255, 110, 110), err);
                    if ui.small_button("x").clicked() {
                        menu.error = None;
                    }
                });
            }
            ui.add_space(24.0);

            match menu.screen {
                Screen::Main => main_screen(ui, &mut menu, &mut action),
                Screen::Host => host_screen(ui, &mut menu, steam.is_some(), &mut action),
                Screen::Join => join_screen(ui, &mut menu, steam.as_deref(), &mut action),
            }
        });
    });

    if let Some(action) = action {
        run_action(action, &mut commands, &mut menu, steam.as_deref(), &mut exit);
    }
    Ok(())
}

fn main_screen(ui: &mut egui::Ui, menu: &mut MenuState, action: &mut Option<Action>) {
    ui.horizontal(|ui| {
        ui.add_space(ui.available_width() / 2.0 - 130.0);
        ui.label("Name");
        ui.add(
            egui::TextEdit::singleline(&mut menu.name)
                .desired_width(210.0)
                .char_limit(32),
        );
    });
    ui.add_space(16.0);
    if big_button(ui, "Host Game") {
        menu.screen = Screen::Host;
    }
    if big_button(ui, "Join Game") {
        menu.screen = Screen::Join;
    }
    if big_button(ui, "Singleplayer") {
        *action = Some(Action::Singleplayer);
    }
    ui.add_space(16.0);
    if big_button(ui, "Quit") {
        *action = Some(Action::Quit);
    }
}

fn mode_tabs(ui: &mut egui::Ui, mode: &mut NetMode, steam_ok: bool) {
    ui.horizontal(|ui| {
        ui.add_space(ui.available_width() / 2.0 - 80.0);
        ui.add_enabled_ui(steam_ok, |ui| {
            ui.selectable_value(mode, NetMode::Steam, "  Steam  ")
        });
        ui.selectable_value(mode, NetMode::Lan, "  LAN / Direct IP  ");
    });
    if !steam_ok {
        *mode = NetMode::Lan;
    }
}

fn host_screen(ui: &mut egui::Ui, menu: &mut MenuState, steam_ok: bool, action: &mut Option<Action>) {
    ui.heading("Host Game");
    mode_tabs(ui, &mut menu.host_mode, steam_ok);
    ui.add_space(12.0);
    egui::Grid::new("host")
        .num_columns(2)
        .spacing([16.0, 10.0])
        .show(ui, |ui| {
            match menu.host_mode {
                NetMode::Steam => {
                    ui.label("Visibility");
                    ui.horizontal(|ui| {
                        ui.radio_value(&mut menu.visibility, Visibility::FriendsOnly, "Friends");
                        ui.radio_value(&mut menu.visibility, Visibility::Public, "Public");
                        ui.radio_value(&mut menu.visibility, Visibility::InviteOnly, "Invite only");
                    });
                    ui.end_row();
                    ui.label("Max players");
                    ui.add(egui::Slider::new(&mut menu.max_players, 2..=8));
                    ui.end_row();
                }
                NetMode::Lan => {
                    ui.label("Port");
                    ui.add(egui::TextEdit::singleline(&mut menu.port).desired_width(80.0));
                    ui.end_row();
                }
            }
            ui.label("World seed");
            ui.add(egui::TextEdit::singleline(&mut menu.seed).desired_width(120.0));
            ui.end_row();
        });
    ui.add_space(16.0);
    if big_button(ui, "Start") {
        *action = Some(match menu.host_mode {
            NetMode::Steam => Action::HostSteam,
            NetMode::Lan => Action::HostLan,
        });
    }
    if big_button(ui, "Back") {
        menu.screen = Screen::Main;
    }
}

fn join_screen(
    ui: &mut egui::Ui,
    menu: &mut MenuState,
    steam: Option<&SteamClient>,
    action: &mut Option<Action>,
) {
    ui.heading("Join Game");
    mode_tabs(ui, &mut menu.join_mode, steam.is_some());
    ui.add_space(12.0);

    match (menu.join_mode, steam) {
        (NetMode::Steam, Some(steam)) => {
            let width = 460.0;
            ui.allocate_ui(egui::vec2(width, 360.0), |ui| {
                ui.group(|ui| {
                    ui.set_width(width);
                    ui.strong("Friends playing");
                    if menu.friends.is_empty() {
                        ui.weak("No friends are hosting right now. Friends can also invite you via the Steam overlay.");
                    }
                    for f in &menu.friends {
                        ui.horizontal(|ui| {
                            ui.label(&f.friend);
                            if ui.button("Join").clicked() {
                                *action = Some(Action::JoinSteam(f.lobby));
                            }
                        });
                    }
                });
                ui.group(|ui| {
                    ui.set_width(width);
                    ui.horizontal(|ui| {
                        ui.strong("Public lobbies");
                        let label = if menu.searching { "Searching…" } else { "Refresh" };
                        if ui.add_enabled(!menu.searching, egui::Button::new(label)).clicked() {
                            menu.searching = true;
                            steam.request_lobby_list();
                        }
                    });
                    egui::ScrollArea::vertical().max_height(140.0).show(ui, |ui| {
                        if menu.lobbies.is_empty() {
                            ui.weak("No public lobbies found.");
                        }
                        for l in &menu.lobbies {
                            ui.horizontal(|ui| {
                                ui.label(format!("{}  ({}/{})", l.name, l.members, l.max_members));
                                if ui.button("Join").clicked() {
                                    *action = Some(Action::JoinSteam(l.id));
                                }
                            });
                        }
                    });
                });
                ui.group(|ui| {
                    ui.set_width(width);
                    ui.horizontal(|ui| {
                        ui.label("Lobby ID");
                        ui.add(egui::TextEdit::singleline(&mut menu.lobby_id).desired_width(200.0));
                        let parsed = menu.lobby_id.trim().parse::<u64>();
                        if ui.add_enabled(parsed.is_ok(), egui::Button::new("Join")).clicked()
                            && let Ok(id) = parsed
                        {
                            *action = Some(Action::JoinSteam(id));
                        }
                    });
                });
            });
        }
        _ => {
            ui.horizontal(|ui| {
                ui.add_space(ui.available_width() / 2.0 - 150.0);
                ui.label("Address");
                ui.add(egui::TextEdit::singleline(&mut menu.address).desired_width(200.0));
            });
            ui.weak(format!("host or host:port (default port {})", tcp::DEFAULT_PORT));
            ui.add_space(8.0);
            if big_button(ui, "Connect") {
                *action = Some(Action::JoinLan);
            }
        }
    }
    ui.add_space(8.0);
    if big_button(ui, "Back") {
        menu.screen = Screen::Main;
    }
}

fn run_action(
    action: Action,
    commands: &mut Commands,
    menu: &mut MenuState,
    steam: Option<&SteamClient>,
    exit: &mut MessageWriter<AppExit>,
) {
    menu.error = None;
    let seed = || {
        menu.seed
            .trim()
            .parse::<u64>()
            .unwrap_or_else(|_| sbct_sim::rng::hash2(0, menu.seed.len() as i32, 0))
    };
    match action {
        Action::Quit => {
            exit.write(AppExit::Success);
        }
        Action::Singleplayer => {
            commands.insert_resource(Session::host(None, OFFLINE_ID, menu.name.clone(), None, seed()));
            commands.set_state(AppState::InGame);
        }
        Action::HostSteam => {
            let Some(steam) = steam else { return };
            steam.create_lobby(menu.visibility, menu.max_players);
            commands.insert_resource(Connecting::CreatingLobby { seed: seed() });
            commands.set_state(AppState::Connecting);
        }
        Action::HostLan => {
            let port = menu.port.trim().parse().unwrap_or(tcp::DEFAULT_PORT);
            match TcpHost::bind(port) {
                Ok(host) => {
                    commands.insert_resource(Session::host(
                        Some(Box::new(host)),
                        tcp::HOST_ID,
                        menu.name.clone(),
                        None,
                        seed(),
                    ));
                    commands.set_state(AppState::InGame);
                }
                Err(e) => menu.error = Some(format!("Couldn't open port {port}: {e}")),
            }
        }
        Action::JoinSteam(lobby) => {
            let Some(steam) = steam else { return };
            steam.join_lobby(lobby);
            commands.insert_resource(Connecting::JoiningLobby);
            commands.set_state(AppState::Connecting);
        }
        Action::JoinLan => {
            let (tx, rx) = channel();
            let addr = menu.address.trim().to_string();
            std::thread::spawn(move || {
                let _ = tx
                    .send(TcpClient::connect(&addr).map_err(|e| format!("Couldn't connect to {addr}: {e}")));
            });
            commands.insert_resource(Connecting::Tcp(Mutex::new(rx)));
            commands.set_state(AppState::Connecting);
        }
    }
}

/// Drives an in-progress connection to the point where we're in the game.
pub fn connecting_update(
    mut commands: Commands,
    connecting: Option<Res<Connecting>>,
    steam: Option<Res<SteamClient>>,
    mut inbox: ResMut<SteamInbox>,
    session: Option<Res<Session>>,
    menu: Res<MenuState>,
) {
    let Some(connecting) = connecting else { return };
    let fail = |commands: &mut Commands, msg: String| commands.insert_resource(EndSession(Some(msg)));

    match &*connecting {
        Connecting::CreatingLobby { seed } => {
            let Some(steam) = steam else { return };
            for event in std::mem::take(&mut inbox.0) {
                match event {
                    SteamEvent::LobbyCreated(Ok(lobby)) => {
                        info!("Created lobby {lobby}");
                        let transport = steam.transport(lobby, steam.my_id());
                        commands.insert_resource(Session::host(
                            Some(Box::new(transport)),
                            steam.my_id(),
                            steam.my_name(),
                            Some(lobby),
                            *seed,
                        ));
                        commands.remove_resource::<Connecting>();
                        commands.set_state(AppState::InGame);
                    }
                    SteamEvent::LobbyCreated(Err(e)) => {
                        fail(&mut commands, format!("Couldn't create lobby: {e}"))
                    }
                    _ => {}
                }
            }
        }
        Connecting::JoiningLobby => {
            let Some(steam) = steam else { return };
            for event in std::mem::take(&mut inbox.0) {
                match event {
                    SteamEvent::LobbyEntered(Ok(lobby)) => {
                        let host = steam.lobby_owner(lobby);
                        if host == steam.my_id() {
                            steam.leave_lobby(lobby);
                            fail(&mut commands, "That lobby has no host.".into());
                            return;
                        }
                        info!("Entered lobby {lobby}, host {host}");
                        let transport = steam.transport(lobby, host);
                        commands.insert_resource(Session::client(
                            Box::new(transport),
                            host,
                            steam.my_name(),
                            Some(lobby),
                        ));
                        commands.insert_resource(Connecting::Downloading);
                    }
                    SteamEvent::LobbyEntered(Err(e)) => fail(&mut commands, e),
                    _ => {}
                }
            }
        }
        Connecting::Tcp(rx) => {
            let result = rx.lock().unwrap().try_recv();
            match result {
                Ok(Ok(client)) => {
                    commands.insert_resource(Session::client(
                        Box::new(client),
                        tcp::HOST_ID,
                        menu.name.clone(),
                        None,
                    ));
                    commands.insert_resource(Connecting::Downloading);
                }
                Ok(Err(e)) => fail(&mut commands, e),
                Err(_) => {}
            }
        }
        Connecting::Downloading => {
            if session.is_some_and(|s| s.is_loaded()) {
                commands.remove_resource::<Connecting>();
                commands.set_state(AppState::InGame);
            }
        }
    }
}

pub fn connecting_ui(
    mut contexts: EguiContexts,
    mut commands: Commands,
    connecting: Option<Res<Connecting>>,
    session: Option<Res<Session>>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let mut root = screen_ui(ctx);
    egui::CentralPanel::default().show(&mut root, |ui| {
        ui.vertical_centered(|ui| {
            ui.add_space(ui.available_height() / 2.0 - 60.0);
            let text = match connecting.as_deref() {
                Some(Connecting::CreatingLobby { .. }) => "Creating Steam lobby…".to_string(),
                Some(Connecting::JoiningLobby) => "Joining Steam lobby…".to_string(),
                Some(Connecting::Tcp(_)) => "Connecting…".to_string(),
                Some(Connecting::Downloading) | None => match &session {
                    Some(s) if s.world.is_some() => {
                        format!("Downloading world {}/{}", s.chunks_received, s.total_chunks())
                    }
                    _ => "Waiting for host…".to_string(),
                },
            };
            ui.label(egui::RichText::new(text).size(22.0));
            if let Some(s) = &session
                && s.total_chunks() > 0
            {
                let p = s.chunks_received as f32 / s.total_chunks() as f32;
                ui.add(egui::ProgressBar::new(p).desired_width(300.0));
            }
            ui.add_space(16.0);
            if big_button(ui, "Cancel") {
                commands.insert_resource(EndSession(None));
            }
        });
    });
    Ok(())
}
