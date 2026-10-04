//! Main menu (host / join / singleplayer) and the connecting screen.

use std::sync::Mutex;
use std::sync::mpsc::{Receiver, channel};

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use sbct_net::steam::{FriendLobby, LobbyInfo, SteamEvent, Visibility};
use sbct_net::tcp::{self, TcpClient, TcpHost};

use crate::AppState;
use crate::run::achievements::{AchievementId, Loadout};
use crate::run::save::SaveData;
use crate::ui::{self, UiIcons};
use crate::session::{EndSession, OFFLINE_ID, Session};
use crate::steam::{SteamClient, SteamInbox, SteamStatus};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    Main,
    NewRun,
    Unlocks,
    Settings,
    Multiplayer,
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
    /// Seed typed on the New Run screen; empty means random.
    pub run_seed: String,
    pub unlocks_tab: usize,
    pub confirm_reset: bool,
    /// Mirrors the saved loadout choice for starting runs.
    pub loadout: Loadout,
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
            run_seed: String::new(),
            unlocks_tab: 0,
            confirm_reset: false,
            loadout: Loadout::Standard,
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
    NewRun,
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
    menu.screen = match value("--menu-screen").as_deref() {
        Some("new-run") => Screen::NewRun,
        Some("unlocks") => Screen::Unlocks,
        Some("items") => {
            menu.unlocks_tab = 1;
            Screen::Unlocks
        }
        Some("settings") => Screen::Settings,
        Some("multiplayer") => Screen::Multiplayer,
        _ => Screen::Main,
    };
    if args.iter().any(|a| a == "--run") {
        menu.run_seed = value("--run").unwrap_or_default();
        Some(Action::NewRun)
    } else if args.iter().any(|a| a == "--singleplayer") {
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

fn big_button(ui: &mut egui::Ui, text: &str) -> bool {
    ui::menu_button(ui, text).clicked()
}

pub fn menu_ui(
    mut contexts: EguiContexts,
    mut commands: Commands,
    mut menu: ResMut<MenuState>,
    steam: Option<Res<SteamClient>>,
    status: Res<SteamStatus>,
    mut exit: MessageWriter<AppExit>,
    mut save: ResMut<SaveData>,
    icons: Option<Res<UiIcons>>,
    time: Res<Time<Real>>,
) -> Result {
    let Some(icons) = icons else { return Ok(()) };
    let ctx = contexts.ctx_mut()?;
    let mut action = None;
    let t = time.elapsed_secs();

    let mut root = ui::screen_ui(ctx);
    egui::CentralPanel::default().frame(egui::Frame::new().fill(egui::Color32::from_rgb(12, 9, 16))).show(&mut root, |ui| {
        paint_menu_backdrop(ui, &icons, t);
        ui.vertical_centered(|ui| {
            let main = menu.screen == Screen::Main;
            ui.add_space(if main { 36.0 } else { 16.0 });
            ui.label(egui::RichText::new("DESCENT").size(if main { 64.0 } else { 40.0 }).color(ui::ACCENT));
            if main {
                ui.label(egui::RichText::new("to the core").size(32.0).color(ui::TEXT_DIM));
            }
            ui.add_space(6.0);
            let stats = &save.stats;
            if stats.runs > 0 && main {
                let best = stats.best_time.map_or(String::new(), |b| format!("   best time {}", ui::clock(b)));
                ui.label(
                    egui::RichText::new(format!(
                        "runs {}   wins {}   deepest {} m{best}",
                        stats.runs,
                        stats.wins,
                        stats.best_depth / 4
                    ))
                    .color(ui::TEXT_DIM),
                );
            }
            if let Some(err) = menu.error.clone() {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.add_space(ui.available_width() / 2.0 - 220.0);
                    ui.colored_label(ui::DANGER, err);
                    if ui.small_button("x").clicked() {
                        menu.error = None;
                    }
                });
            }
            ui.add_space(if main { 20.0 } else { 8.0 });

            match menu.screen {
                Screen::Main => main_screen(ui, &mut menu, &mut action),
                Screen::NewRun => new_run_screen(ui, &mut menu, &mut save, &icons, &mut action),
                Screen::Unlocks => unlocks_screen(ui, &mut menu, &save, &icons),
                Screen::Settings => settings_screen(ui, &mut menu, &mut save),
                Screen::Multiplayer => {
                    let line = match (&steam, &status.error) {
                        (Some(s), _) => format!("Steam: signed in as {}", s.my_name()),
                        (None, Some(e)) => format!("Steam unavailable (LAN only): {e}"),
                        _ => String::new(),
                    };
                    ui.label(egui::RichText::new(line).color(if steam.is_some() { ui::GOOD } else { ui::DANGER }));
                    ui.add_space(8.0);
                    multiplayer_screen(ui, &mut menu, &mut action)
                }
                Screen::Host => host_screen(ui, &mut menu, steam.is_some(), &mut action),
                Screen::Join => join_screen(ui, &mut menu, steam.as_deref(), &mut action),
            }
        });
    });

    menu.loadout = if save.loadout_unlocked(save.loadout) { save.loadout } else { Loadout::Standard };
    if let Some(action) = action {
        run_action(action, &mut commands, &mut menu, steam.as_deref(), &mut exit);
    }
    Ok(())
}

/// Slowly rising embers and the pulsing core behind the title.
fn paint_menu_backdrop(ui: &egui::Ui, icons: &UiIcons, t: f32) {
    let rect = ui.max_rect();
    let p = ui.painter();
    let frame = (t * 8.0) as usize % 8;
    let size = rect.height().min(rect.width()) * 0.9;
    let core = egui::Rect::from_center_size(egui::pos2(rect.center().x, rect.bottom() + size * 0.25), egui::vec2(size, size));
    let uv = egui::Rect::from_min_max(egui::pos2(frame as f32 / 8.0, 0.0), egui::pos2((frame + 1) as f32 / 8.0, 1.0));
    p.image(icons.core, core, uv, egui::Color32::from_white_alpha(150));
    for i in 0..60 {
        let seed = sbct_sim::rng::hash2(i, 5, 9);
        let x = rect.left() + (seed % 10_000) as f32 / 10_000.0 * rect.width();
        let speed = 20.0 + (seed >> 20) as f32 % 40.0;
        let y = rect.bottom() - (t * speed + (seed >> 8) as f32 % rect.height()) % rect.height();
        let a = ((y - rect.top()) / rect.height() * 200.0) as u8;
        p.rect_filled(
            egui::Rect::from_center_size(egui::pos2(x, y), egui::vec2(3.0, 3.0)),
            0.0,
            egui::Color32::from_rgba_unmultiplied(255, 150, 60, a),
        );
    }
}

fn main_screen(ui: &mut egui::Ui, menu: &mut MenuState, action: &mut Option<Action>) {
    if ui::menu_button(ui, "New Run").clicked() {
        menu.screen = Screen::NewRun;
    }
    if ui::menu_button(ui, "Unlocks & Achievements").clicked() {
        menu.screen = Screen::Unlocks;
    }
    if ui::menu_button(ui, "Settings").clicked() {
        menu.screen = Screen::Settings;
    }
    if ui::menu_button(ui, "Multiplayer & Sandbox").clicked() {
        menu.screen = Screen::Multiplayer;
    }
    ui.add_space(12.0);
    if ui::menu_button(ui, "Quit").clicked() {
        *action = Some(Action::Quit);
    }
}

fn new_run_screen(
    ui: &mut egui::Ui,
    menu: &mut MenuState,
    save: &mut SaveData,
    icons: &UiIcons,
    action: &mut Option<Action>,
) {
    ui.label(egui::RichText::new("Choose a loadout").size(24.0));
    ui.add_space(4.0);
    ui.horizontal_top(|ui| {
        ui.add_space(ui.available_width() / 2.0 - 3.0 * 196.0 / 2.0);
        for loadout in Loadout::ALL {
            let unlocked = save.loadout_unlocked(loadout);
            let selected = save.loadout == loadout && unlocked;
            let frame = ui::panel_frame().stroke(egui::Stroke::new(2.0, if selected { ui::ACCENT } else { ui::BORDER }));
            let r = frame
                .show(ui, |ui| {
                    ui.set_width(168.0);
                    ui.set_height(150.0);
                    ui.vertical_centered(|ui| {
                        let index = match loadout {
                            Loadout::Standard => crate::run::items::icon::PICKAXE,
                            Loadout::Gifted => crate::run::items::icon::GEM,
                            Loadout::Excavator => crate::run::items::ItemId::BlastCharges.def().icon,
                        };
                        if unlocked {
                            ui::icon(ui, icons, index, 32.0);
                            ui.label(egui::RichText::new(loadout.name()).color(if selected { ui::ACCENT } else { ui::TEXT }));
                            ui.label(egui::RichText::new(loadout.desc()).size(16.0).color(ui::TEXT_DIM));
                        } else {
                            ui::icon(ui, icons, crate::run::items::icon::LOCK, 32.0);
                            ui.label(egui::RichText::new(loadout.name()).color(ui::TEXT_DIM));
                            let hint = loadout.unlock().map_or("", |a| a.def().hint);
                            ui.label(egui::RichText::new(hint).size(16.0).color(ui::TEXT_DIM));
                        }
                    });
                })
                .response
                .interact(egui::Sense::click());
            if r.clicked() && unlocked {
                save.loadout = loadout;
                crate::run::save::store(save);
            }
        }
    });
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        ui.add_space(ui.available_width() / 2.0 - 170.0);
        ui::icon(ui, icons, crate::run::items::icon::COMPASS, 16.0);
        ui.label("Seed");
        ui.add(egui::TextEdit::singleline(&mut menu.run_seed).desired_width(170.0).hint_text("random").char_limit(12));
    });
    ui.label(egui::RichText::new("Leave empty for a fresh planet; share a seed to replay one.").color(ui::TEXT_DIM));
    ui.add_space(12.0);
    if ui::menu_button(ui, "Descend").clicked() {
        *action = Some(Action::NewRun);
    }
    if ui::menu_button(ui, "Back").clicked() {
        menu.screen = Screen::Main;
    }
}

fn unlocks_screen(ui: &mut egui::Ui, menu: &mut MenuState, save: &SaveData, icons: &UiIcons) {
    let earned = save.achievements.len();
    let items = save.unlocked_items().len();
    ui.horizontal(|ui| {
        ui.add_space(ui.available_width() / 2.0 - 200.0);
        for (i, label) in [
            format!("Achievements {earned}/{}", AchievementId::ALL.len()),
            format!("Items {items}/{}", crate::run::items::ItemId::ALL.len()),
        ]
        .iter()
        .enumerate()
        {
            if ui.selectable_label(menu.unlocks_tab == i, label).clicked() {
                menu.unlocks_tab = i;
            }
        }
    });
    ui.add_space(8.0);
    // Cards: (icon, unlocked, title, line1, line2).
    let cards: Vec<(usize, bool, String, String, String)> = if menu.unlocks_tab == 0 {
        AchievementId::ALL
            .iter()
            .map(|&a| {
                let def = a.def();
                let got = save.achievements.contains(&a);
                let title = if got { def.name } else { "???" };
                (a.icon(), got, title.to_string(), def.hint.to_string(), format!("Reward: {}", a.reward_name()))
            })
            .collect()
    } else {
        crate::run::items::ItemId::ALL
            .iter()
            .map(|&item| {
                let def = item.def();
                let got = save.is_unlocked(item);
                let line2 = match def.unlock {
                    None => "Available from the start".to_string(),
                    Some(a) if got => format!("Unlocked by {}", a.def().name),
                    Some(a) => format!("Locked: {}", a.def().hint),
                };
                let line1 = if got { def.desc.to_string() } else { "Unknown item".to_string() };
                (def.icon, got, if got { def.name } else { "???" }.to_string(), line1, line2)
            })
            .collect()
    };
    const CARD: egui::Vec2 = egui::vec2(380.0, 104.0);
    let margin = ((ui.available_width() - CARD.x * 2.0 - 20.0) / 2.0).max(0.0);
    egui::ScrollArea::vertical().max_height(ui.available_height() - 70.0).show(ui, |ui| {
        for pair in cards.chunks(2) {
            ui.horizontal_top(|ui| {
                ui.add_space(margin);
                for (icon, got, title, line1, line2) in pair {
                    ui::panel_frame().show(ui, |ui| {
                        ui.set_min_size(CARD - egui::vec2(20.0, 20.0));
                        ui.set_max_width(CARD.x - 20.0);
                        ui.horizontal_top(|ui| {
                            if *got {
                                ui::icon(ui, icons, *icon, 32.0);
                            } else {
                                ui::icon_tinted(ui, icons, *icon, 32.0, egui::Color32::from_rgb(40, 32, 50));
                            }
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new(title).color(if *got { ui::ACCENT } else { ui::TEXT_DIM }));
                                ui.label(egui::RichText::new(line1).size(16.0));
                                ui.label(egui::RichText::new(line2).size(16.0).color(ui::TEXT_DIM));
                            });
                        });
                    });
                }
            });
        }
    });
    ui.add_space(8.0);
    if ui::menu_button(ui, "Back").clicked() {
        menu.screen = Screen::Main;
    }
}

fn settings_screen(ui: &mut egui::Ui, menu: &mut MenuState, save: &mut SaveData) {
    ui::panel_frame().show(ui, |ui| {
        ui.set_width(420.0);
        crate::run::hud::settings_controls(ui, save);
        ui.add_space(10.0);
        ui.separator();
        ui.label(egui::RichText::new(format!("Save file: {}", crate::run::save::save_dir().join("save.json").display())).size(16.0).color(ui::TEXT_DIM));
        if menu.confirm_reset {
            ui.horizontal(|ui| {
                ui.colored_label(ui::DANGER, "Erase all unlocks and stats?");
                if ui.button("Erase").clicked() {
                    let settings = save.settings.clone();
                    *save = SaveData { settings, ..Default::default() };
                    crate::run::save::store(save);
                    menu.confirm_reset = false;
                }
                if ui.button("Cancel").clicked() {
                    menu.confirm_reset = false;
                }
            });
        } else if ui.button("Reset progress").clicked() {
            menu.confirm_reset = true;
        }
    });
    ui.add_space(12.0);
    if ui::menu_button(ui, "Back").clicked() {
        menu.screen = Screen::Main;
    }
}

fn multiplayer_screen(ui: &mut egui::Ui, menu: &mut MenuState, action: &mut Option<Action>) {
    ui.horizontal(|ui| {
        ui.add_space(ui.available_width() / 2.0 - 140.0);
        ui.label("Name");
        ui.add(egui::TextEdit::singleline(&mut menu.name).desired_width(210.0).char_limit(32));
    });
    ui.add_space(8.0);
    ui.label(egui::RichText::new("Sandbox worlds: paint and dig freely, alone or together.").color(ui::TEXT_DIM));
    ui.add_space(8.0);
    if ui::menu_button(ui, "Host Game").clicked() {
        menu.screen = Screen::Host;
    }
    if ui::menu_button(ui, "Join Game").clicked() {
        menu.screen = Screen::Join;
    }
    if ui::menu_button(ui, "Sandbox (offline)").clicked() {
        *action = Some(Action::Singleplayer);
    }
    ui.add_space(12.0);
    if ui::menu_button(ui, "Back").clicked() {
        menu.screen = Screen::Main;
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
        menu.screen = Screen::Multiplayer;
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
        menu.screen = Screen::Multiplayer;
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
        Action::NewRun => {
            let seed = menu.run_seed.trim().parse().unwrap_or_else(|_| crate::run::random_seed());
            crate::run::start_run(commands, seed, menu.loadout);
        }
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
    let mut root = ui::screen_ui(ctx);
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
