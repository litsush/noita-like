//! Main menu (new world, load, join, settings) and the connecting screen.

use std::sync::Mutex;
use std::sync::mpsc::{Receiver, channel};

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use sbct_net::Transport;
use sbct_net::steam::{FriendLobby, LobbyInfo, SteamEvent, Visibility};
use sbct_net::tcp::{self, TcpClient, TcpHost};
use sbct_sim::colony::PlayerKey;

use crate::AppState;
use crate::controls::Rebind;
use crate::dev::{arg_value, has_flag};
use crate::session::{EndSession, MAX_PLAYERS, Session, list_saves};
use crate::settings::{Config, SettingsTab, settings_ui};
use crate::steam::{SteamClient, SteamInbox, SteamStatus};
use crate::ui::{self, ACCENT, DANGER, GOOD, TEXT_DIM, UiIcons};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Main,
    NewWorld,
    Load,
    Join,
    Settings,
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum NetMode {
    Solo,
    Steam,
    Lan,
}

/// Which world to host.
#[derive(Clone)]
pub enum WorldChoice {
    New { seed: u64, name: String },
    Saved(String),
}

#[derive(Resource)]
pub struct MenuState {
    pub screen: Screen,
    pub error: Option<String>,
    pub host_mode: NetMode,
    pub visibility: Visibility,
    pub max_players: u32,
    pub port: String,
    pub seed: String,
    pub world_name: String,
    pub saves: Vec<String>,
    pub selected_save: Option<String>,
    pub join_mode: NetMode,
    pub address: String,
    pub lobby_id: String,
    pub lobbies: Vec<LobbyInfo>,
    pub friends: Vec<FriendLobby>,
    pub searching: bool,
    pub settings_tab: SettingsTab,
    friends_timer: f32,
}

impl MenuState {
    pub fn new(steam: Option<&SteamClient>, config: &Config) -> MenuState {
        let mode = if steam.is_some() {
            NetMode::Steam
        } else {
            NetMode::Lan
        };
        MenuState {
            screen: Screen::Main,
            error: None,
            host_mode: NetMode::Solo,
            visibility: Visibility::FriendsOnly,
            max_players: MAX_PLAYERS as u32,
            port: tcp::DEFAULT_PORT.to_string(),
            seed: random_seed().to_string(),
            world_name: "New Colony".into(),
            saves: Vec::new(),
            selected_save: None,
            join_mode: mode,
            address: if config.last_address.is_empty() {
                "127.0.0.1".into()
            } else {
                config.last_address.clone()
            },
            lobby_id: String::new(),
            lobbies: Vec::new(),
            friends: Vec::new(),
            searching: false,
            settings_tab: SettingsTab::General,
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

/// A connection or world start in progress. Removed when entering the game
/// or the menu.
#[derive(Resource)]
pub enum Connecting {
    CreatingLobby(WorldChoice),
    JoiningLobby,
    Tcp(Mutex<Receiver<Result<TcpClient, String>>>),
    /// Generating or loading a world on another thread.
    Building(Mutex<Receiver<Result<Session, String>>>),
    /// Transport is up; waiting for the welcome and the world download.
    Downloading,
}

enum Action {
    Host(WorldChoice, NetMode),
    JoinSteam(u64),
    JoinLan,
    Quit,
}

pub fn setup_menu(
    mut commands: Commands,
    steam: Option<Res<SteamClient>>,
    mut config: ResMut<Config>,
    mut exit: MessageWriter<AppExit>,
) {
    // `--name` is a dev shortcut for running several players on one
    // machine: each name gets its own identity.
    if let Some(name) = arg_value("--name") {
        config.key =
            sbct_sim::rng::hash2(0x5EED, name.len() as i32, name.bytes().map(|b| b as i32).sum()) | 1;
        config.name = name;
        config.ephemeral = true;
    }
    let mut menu = MenuState::new(steam.as_deref(), &config);
    // Steam passes `+connect_lobby <id>` when launching the game from an invite.
    if let (Some(steam), Some(lobby)) = (&steam, crate::steam::lobby_from_args()) {
        steam.join_lobby(lobby);
        commands.insert_resource(Connecting::JoiningLobby);
        commands.set_state(AppState::Connecting);
    } else if let Some(action) = action_from_args(&mut menu) {
        run_action(
            action,
            &mut commands,
            &mut menu,
            &mut config,
            steam.as_deref(),
            &mut exit,
        );
    }
    commands.insert_resource(menu);
}

/// Shortcuts that skip the menu: `--singleplayer`, `--host-lan [port]`,
/// `--join-lan <addr>`, plus `--name <name>`, `--seed <n>`, `--world <name>`
/// (the save to create) and `--load <name>` (host a saved world instead).
fn action_from_args(menu: &mut MenuState) -> Option<Action> {
    if let Some(seed) = arg_value("--seed") {
        menu.seed = seed;
    }
    let choice = |menu: &MenuState| match arg_value("--load") {
        Some(name) => WorldChoice::Saved(name),
        None => WorldChoice::New {
            seed: parse_seed(&menu.seed),
            name: arg_value("--world").unwrap_or_else(|| "dev".into()),
        },
    };
    if has_flag("--singleplayer") {
        Some(Action::Host(choice(menu), NetMode::Solo))
    } else if has_flag("--host-lan") {
        if let Some(port) = arg_value("--host-lan").filter(|v| !v.starts_with("--")) {
            menu.port = port;
        }
        Some(Action::Host(choice(menu), NetMode::Lan))
    } else if let Some(addr) = arg_value("--join-lan") {
        menu.address = addr;
        Some(Action::JoinLan)
    } else {
        None
    }
}

fn parse_seed(text: &str) -> u64 {
    let text = text.trim();
    text.parse::<u64>().unwrap_or_else(|_| {
        // Words work as seeds too.
        text.bytes().fold(1469598103934665603u64, |h, b| {
            (h ^ b as u64).wrapping_mul(1099511628211)
        })
    })
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

/// Paints the title backdrop so it covers the window.
fn backdrop(ctx: &egui::Context, icons: Option<&UiIcons>) {
    let painter = ctx.layer_painter(egui::LayerId::background());
    let rect = ctx.viewport_rect();
    painter.rect_filled(rect, 0.0, egui::Color32::from_rgb(8, 13, 15));
    if let Some(icons) = icons {
        // 640×400 art, scaled by whole numbers where possible.
        let scale = (rect.width() / 640.0).max(rect.height() / 400.0);
        let size = egui::vec2(640.0, 400.0) * scale;
        let r = egui::Rect::from_center_size(rect.center(), size);
        painter.image(
            icons.menu,
            r,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
        painter.rect_filled(rect, 0.0, egui::Color32::from_black_alpha(70));
    }
}

pub fn menu_ui(
    mut contexts: EguiContexts,
    mut commands: Commands,
    mut menu: ResMut<MenuState>,
    mut config: ResMut<Config>,
    mut rebind: ResMut<Rebind>,
    steam: Option<Res<SteamClient>>,
    status: Res<SteamStatus>,
    icons: Option<Res<UiIcons>>,
    mut exit: MessageWriter<AppExit>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    let mut action = None;
    backdrop(ctx, icons.as_deref());

    egui::Area::new("menu".into())
        .anchor(egui::Align2::CENTER_TOP, [0.0, 24.0])
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.label(ui::title("PLANET", 40.0).color(ui::TEXT));
                ui.label(ui::title("TERRAFORMING", 64.0).color(ACCENT));
                ui::flourish(ui);
                ui.label(
                    egui::RichText::new("make a new world livable, together")
                        .italics()
                        .color(TEXT_DIM),
                );
            });
        });

    egui::Area::new("menu_body".into())
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 70.0])
        .show(ctx, |ui| {
            ui::ornate(ui, |ui| {
                ui.set_min_width(380.0);
                ui.vertical_centered(|ui| {
                    if let Some(err) = menu.error.clone() {
                        ui.horizontal(|ui| {
                            ui.colored_label(DANGER, err);
                            if ui.small_button("x").clicked() {
                                menu.error = None;
                            }
                        });
                        ui.add_space(4.0);
                    }
                    match menu.screen {
                        Screen::Main => main_screen(ui, &mut menu, &mut config, &mut action),
                        Screen::NewWorld => new_world_screen(ui, &mut menu, steam.is_some(), &mut action),
                        Screen::Load => load_screen(ui, &mut menu, steam.is_some(), &mut action),
                        Screen::Join => join_screen(ui, &mut menu, steam.as_deref(), &mut action),
                        Screen::Settings => {
                            ui.set_width(560.0);
                            ui.set_height(400.0);
                            settings_ui(ui, &mut config, &mut rebind, &mut menu.settings_tab);
                            if ui.button("Back").clicked() {
                                menu.screen = Screen::Main;
                            }
                        }
                    }
                });
            });
        });

    egui::Area::new("steam_status".into())
        .anchor(egui::Align2::LEFT_BOTTOM, [10.0, -10.0])
        .show(ctx, |ui| {
            match (&steam, &status.error) {
                (Some(s), _) => ui.colored_label(GOOD, format!("Steam: signed in as {}", s.my_name())),
                (None, Some(e)) => {
                    ui.colored_label(TEXT_DIM, format!("Steam unavailable (LAN and solo only): {e}"))
                }
                _ => ui.label(""),
            };
        });

    if let Some(action) = action {
        run_action(
            action,
            &mut commands,
            &mut menu,
            &mut config,
            steam.as_deref(),
            &mut exit,
        );
    }
    Ok(())
}

fn main_screen(ui: &mut egui::Ui, menu: &mut MenuState, config: &mut Config, action: &mut Option<Action>) {
    ui.horizontal(|ui| {
        ui.label("Name");
        if ui
            .add(
                egui::TextEdit::singleline(&mut config.name)
                    .desired_width(220.0)
                    .char_limit(24),
            )
            .lost_focus()
        {
            config.store();
        }
    });
    ui.add_space(8.0);
    if ui::menu_button(ui, "New World").clicked() {
        menu.screen = Screen::NewWorld;
        menu.seed = random_seed().to_string();
    }
    if ui::menu_button(ui, "Load World").clicked() {
        menu.saves = list_saves();
        menu.selected_save = menu.saves.first().cloned();
        menu.screen = Screen::Load;
    }
    if ui::menu_button(ui, "Join Game").clicked() {
        menu.screen = Screen::Join;
    }
    if ui::menu_button(ui, "Settings").clicked() {
        menu.screen = Screen::Settings;
    }
    ui.add_space(8.0);
    if ui::menu_button(ui, "Quit").clicked() {
        *action = Some(Action::Quit);
    }
}

/// Who can join: nobody, Steam friends, or the local network.
fn host_options(ui: &mut egui::Ui, menu: &mut MenuState, steam_ok: bool) {
    ui.horizontal(|ui| {
        ui.selectable_value(&mut menu.host_mode, NetMode::Solo, " Solo ");
        ui.add_enabled_ui(steam_ok, |ui| {
            ui.selectable_value(&mut menu.host_mode, NetMode::Steam, " Steam co-op ")
        });
        ui.selectable_value(&mut menu.host_mode, NetMode::Lan, " LAN co-op ");
    });
    if !steam_ok && menu.host_mode == NetMode::Steam {
        menu.host_mode = NetMode::Solo;
    }
    egui::Grid::new("host")
        .num_columns(2)
        .spacing([16.0, 8.0])
        .show(ui, |ui| match menu.host_mode {
            NetMode::Steam => {
                ui.label("Visibility");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut menu.visibility, Visibility::FriendsOnly, "Friends");
                    ui.radio_value(&mut menu.visibility, Visibility::Public, "Public");
                    ui.radio_value(&mut menu.visibility, Visibility::InviteOnly, "Invite only");
                });
                ui.end_row();
                ui.label("Max players");
                ui.add(egui::Slider::new(&mut menu.max_players, 2..=MAX_PLAYERS as u32));
                ui.end_row();
            }
            NetMode::Lan => {
                ui.label("Port");
                ui.add(egui::TextEdit::singleline(&mut menu.port).desired_width(80.0));
                ui.end_row();
            }
            NetMode::Solo => {}
        });
    ui.label(
        egui::RichText::new(match menu.host_mode {
            NetMode::Solo => "Just you. You can host the same world for friends later.",
            NetMode::Steam => "Up to 8 players. Friends join from the Join screen or by invite.",
            NetMode::Lan => "Up to 8 players join by your IP address. No Steam needed.",
        })
        .color(TEXT_DIM),
    );
}

fn new_world_screen(ui: &mut egui::Ui, menu: &mut MenuState, steam_ok: bool, action: &mut Option<Action>) {
    ui.label(ui::title("New World", 32.0));
    egui::Grid::new("new")
        .num_columns(2)
        .spacing([16.0, 8.0])
        .show(ui, |ui| {
            ui.label("Colony name");
            ui.add(
                egui::TextEdit::singleline(&mut menu.world_name)
                    .desired_width(200.0)
                    .char_limit(40),
            );
            ui.end_row();
            ui.label("Seed");
            ui.horizontal(|ui| {
                ui.add(egui::TextEdit::singleline(&mut menu.seed).desired_width(120.0));
                if ui.button("Random").clicked() {
                    menu.seed = random_seed().to_string();
                }
            });
            ui.end_row();
        });
    if list_saves().contains(&menu.world_name.trim().to_string()) {
        ui.colored_label(ui::WARN, "A saved world has this name and will be replaced.");
    }
    ui.add_space(6.0);
    host_options(ui, menu, steam_ok);
    ui.add_space(8.0);
    if ui::menu_button(ui, "Land").clicked() {
        *action = Some(Action::Host(
            WorldChoice::New {
                seed: parse_seed(&menu.seed),
                name: menu.world_name.clone(),
            },
            menu.host_mode,
        ));
    }
    if ui::menu_button(ui, "Back").clicked() {
        menu.screen = Screen::Main;
    }
}

fn load_screen(ui: &mut egui::Ui, menu: &mut MenuState, steam_ok: bool, action: &mut Option<Action>) {
    ui.label(ui::title("Load World", 32.0));
    if menu.saves.is_empty() {
        ui.label(egui::RichText::new("No saved worlds yet.").color(TEXT_DIM));
    }
    egui::ScrollArea::vertical().max_height(160.0).show(ui, |ui| {
        for save in menu.saves.clone() {
            let selected = menu.selected_save.as_ref() == Some(&save);
            if ui.selectable_label(selected, &save).clicked() {
                menu.selected_save = Some(save);
            }
        }
    });
    ui.add_space(6.0);
    host_options(ui, menu, steam_ok);
    ui.add_space(8.0);
    ui.add_enabled_ui(menu.selected_save.is_some(), |ui| {
        if ui::menu_button(ui, "Continue").clicked()
            && let Some(save) = menu.selected_save.clone()
        {
            *action = Some(Action::Host(WorldChoice::Saved(save), menu.host_mode));
        }
    });
    if ui::menu_button(ui, "Back").clicked() {
        menu.screen = Screen::Main;
    }
}

fn join_screen(
    ui: &mut egui::Ui,
    menu: &mut MenuState,
    steam: Option<&SteamClient>,
    action: &mut Option<Action>,
) {
    ui.label(ui::title("Join Game", 32.0));
    ui.horizontal(|ui| {
        ui.add_enabled_ui(steam.is_some(), |ui| {
            ui.selectable_value(&mut menu.join_mode, NetMode::Steam, " Steam ")
        });
        ui.selectable_value(&mut menu.join_mode, NetMode::Lan, " LAN / direct IP ");
    });
    ui.add_space(6.0);

    match (menu.join_mode, steam) {
        (NetMode::Steam, Some(steam)) => {
            let width = 440.0;
            ui.group(|ui| {
                ui.set_width(width);
                ui.strong("Friends playing");
                if menu.friends.is_empty() {
                    ui.label(
                        egui::RichText::new(
                            "No friends are hosting right now. Friends can also invite you through the Steam overlay.",
                        )
                        .color(TEXT_DIM),
                    );
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
                    ui.strong("Public colonies");
                    let label = if menu.searching { "Searching…" } else { "Refresh" };
                    if ui
                        .add_enabled(!menu.searching, egui::Button::new(label))
                        .clicked()
                    {
                        menu.searching = true;
                        steam.request_lobby_list();
                    }
                });
                egui::ScrollArea::vertical().max_height(110.0).show(ui, |ui| {
                    if menu.lobbies.is_empty() {
                        ui.label(egui::RichText::new("No public colonies found.").color(TEXT_DIM));
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
            ui.horizontal(|ui| {
                ui.label("Lobby ID");
                ui.add(egui::TextEdit::singleline(&mut menu.lobby_id).desired_width(200.0));
                let parsed = menu.lobby_id.trim().parse::<u64>();
                if ui
                    .add_enabled(parsed.is_ok(), egui::Button::new("Join"))
                    .clicked()
                    && let Ok(id) = parsed
                {
                    *action = Some(Action::JoinSteam(id));
                }
            });
        }
        _ => {
            ui.horizontal(|ui| {
                ui.label("Address");
                ui.add(egui::TextEdit::singleline(&mut menu.address).desired_width(200.0));
            });
            ui.label(
                egui::RichText::new(format!("host or host:port (default port {})", tcp::DEFAULT_PORT))
                    .color(TEXT_DIM),
            );
            ui.add_space(6.0);
            if ui::menu_button(ui, "Connect").clicked() {
                *action = Some(Action::JoinLan);
            }
        }
    }
    ui.add_space(6.0);
    if ui::menu_button(ui, "Back").clicked() {
        menu.screen = Screen::Main;
    }
}

/// Generates or loads the world on another thread so the window stays alive.
fn build_world(
    commands: &mut Commands,
    choice: WorldChoice,
    transport: Option<Box<dyn Transport>>,
    me: PlayerKey,
    name: String,
    lobby: Option<u64>,
) {
    let (tx, rx) = channel();
    std::thread::spawn(move || {
        let session = match choice {
            WorldChoice::New { seed, name: world } => {
                Ok(Session::host_new(transport, me, name, lobby, seed, &world))
            }
            WorldChoice::Saved(save) => Session::host_saved(transport, me, name, lobby, &save),
        };
        let _ = tx.send(session);
    });
    commands.insert_resource(Connecting::Building(Mutex::new(rx)));
    commands.set_state(AppState::Connecting);
}

fn run_action(
    action: Action,
    commands: &mut Commands,
    menu: &mut MenuState,
    config: &mut Config,
    steam: Option<&SteamClient>,
    exit: &mut MessageWriter<AppExit>,
) {
    menu.error = None;
    match action {
        Action::Quit => {
            exit.write(AppExit::Success);
        }
        Action::Host(choice, NetMode::Solo) => {
            build_world(commands, choice, None, config.key, config.name.clone(), None);
        }
        Action::Host(choice, NetMode::Steam) => {
            let Some(steam) = steam else { return };
            steam.create_lobby(menu.visibility, menu.max_players);
            commands.insert_resource(Connecting::CreatingLobby(choice));
            commands.set_state(AppState::Connecting);
        }
        Action::Host(choice, NetMode::Lan) => {
            let port = menu.port.trim().parse().unwrap_or(tcp::DEFAULT_PORT);
            match TcpHost::bind(port) {
                Ok(host) => build_world(
                    commands,
                    choice,
                    Some(Box::new(host)),
                    config.key,
                    config.name.clone(),
                    None,
                ),
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
            config.last_address = addr.clone();
            config.store();
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
    config: Res<Config>,
) {
    let Some(connecting) = connecting else { return };
    let fail = |commands: &mut Commands, msg: String| commands.insert_resource(EndSession(Some(msg)));

    match &*connecting {
        Connecting::CreatingLobby(choice) => {
            let Some(steam) = steam else { return };
            for event in std::mem::take(&mut inbox.0) {
                match event {
                    SteamEvent::LobbyCreated(Ok(lobby)) => {
                        info!("Created lobby {lobby}");
                        let transport = steam.transport(lobby, steam.my_id());
                        build_world(
                            &mut commands,
                            choice.clone(),
                            Some(Box::new(transport)),
                            steam.my_id(),
                            steam.my_name(),
                            Some(lobby),
                        );
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
                        // On Steam the player's identity is their Steam id.
                        commands.insert_resource(Session::client(
                            Box::new(transport),
                            host,
                            steam.my_name(),
                            steam.my_id(),
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
                        config.name.clone(),
                        config.key,
                        None,
                    ));
                    commands.insert_resource(Connecting::Downloading);
                }
                Ok(Err(e)) => fail(&mut commands, e),
                Err(_) => {}
            }
        }
        Connecting::Building(rx) => {
            let result = rx.lock().unwrap().try_recv();
            match result {
                Ok(Ok(session)) => {
                    commands.insert_resource(session);
                    commands.remove_resource::<Connecting>();
                    commands.set_state(AppState::InGame);
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
    icons: Option<Res<UiIcons>>,
) -> Result {
    let ctx = contexts.ctx_mut()?;
    backdrop(ctx, icons.as_deref());
    egui::Area::new("connecting".into())
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(ctx, |ui| {
            ui::ornate(ui, |ui| {
                ui.set_min_width(360.0);
                ui.vertical_centered(|ui| {
                    let text = match connecting.as_deref() {
                        Some(Connecting::CreatingLobby(_)) => "Creating Steam lobby…".to_string(),
                        Some(Connecting::JoiningLobby) => "Joining Steam lobby…".to_string(),
                        Some(Connecting::Tcp(_)) => "Connecting…".to_string(),
                        Some(Connecting::Building(_)) => "Preparing the planet…".to_string(),
                        Some(Connecting::Downloading) | None => match &session {
                            Some(s) if s.world.is_some() => {
                                format!("Downloading the world {}/{}", s.chunks_received, s.total_chunks())
                            }
                            _ => "Waiting for the host…".to_string(),
                        },
                    };
                    ui.label(ui::title(text, 24.0));
                    if let Some(s) = &session
                        && s.total_chunks() > 0
                    {
                        let p = s.chunks_received as f32 / s.total_chunks() as f32;
                        ui.add(egui::ProgressBar::new(p).desired_width(300.0));
                    }
                    ui.add_space(8.0);
                    if !matches!(connecting.as_deref(), Some(Connecting::Building(_)))
                        && ui::menu_button(ui, "Cancel").clicked()
                    {
                        commands.insert_resource(EndSession(None));
                    }
                });
            });
        });
    Ok(())
}
