//! Online lobby: hosting, joining by address and Tailscale device discovery.

use super::*;
use crate::net::{game_port, ClientEvent, HostEvent};
use crate::ui::{BLOOD, EMBER, GOLD, TEAL};

impl App {
    pub(super) fn open_lobby(&mut self) {
        self.screen = Screen::Lobby;
        self.lobby.sel = 0;
        if self.lobby.ip.is_empty() {
            self.lobby.ip = self.save.settings.last_ip.clone();
        }
        self.refresh_tailscale();
    }

    fn lobby_rows(&self) -> usize {
        match self.lobby.state {
            LobbyState::Browse => 5 + self.ts_peers().len() + 1,
            // Class, difficulty, mode, three challenges, begin, close.
            LobbyState::Hosting => 5 + ALL_CHALLENGES.len(),
            // Class, leave.
            LobbyState::Joining(_) | LobbyState::Joined => 2,
        }
    }

    fn ts_peers(&self) -> Vec<net::TsPeer> {
        self.lobby.ts.as_ref().map(|t| t.peers.iter().filter(|p| p.online).cloned().collect()).unwrap_or_default()
    }

    pub(super) fn refresh_tailscale(&mut self) {
        if self.lobby.ts_job.is_none() {
            self.lobby.ts_job = Some(std::thread::spawn(net::tailscale_status));
            self.lobby.lan = net::lan_ip();
        }
    }

    pub(super) fn host_lobby(&mut self) {
        match HostNet::open(game_port()) {
            Ok(h) => {
                crate::log!("hosting a lobby on UDP {}", game_port());
                self.net = NetMode::Host(h);
                self.lobby.state = LobbyState::Hosting;
                self.lobby.sel = 0;
                self.lobby.msg = Some(("Lobby open. Share one of the addresses above with your companions.".into(), TEAL));
                self.audio.play("kindle", 0.5);
            }
            Err(e) => {
                crate::log!("could not host: {}", e);
                self.lobby.msg = Some((e, BLOOD));
            }
        }
    }

    pub(super) fn join(&mut self, addr_text: &str) {
        let result = net::resolve(addr_text).and_then(|addr| ClientNet::connect(addr, get_time()).map(|c| (addr, c)));
        match result {
            Ok((addr, c)) => {
                crate::log!("joining {}", addr);
                self.net = NetMode::Client(c);
                self.lobby.state = LobbyState::Joining(0.0);
                self.lobby.sel = 0;
                self.lobby.msg = Some((format!("Connecting to {}...", addr), PALE));
                self.save.settings.last_ip = addr_text.trim().to_string();
                self.save.store();
            }
            Err(e) => self.lobby.msg = Some((e, BLOOD)),
        }
    }

    /// Steps through the difficulty presets; the host's choice applies to everyone.
    fn cycle_difficulty(&mut self, dir: i32) {
        let st = &mut self.save.settings;
        let now = PRESETS.iter().position(|(_, d)| *d == st.diff).map(|i| i as i32).unwrap_or(if dir > 0 { -1 } else { 0 });
        st.diff = PRESETS[(now + dir).rem_euclid(PRESETS.len() as i32) as usize].1;
        self.lobby.diff = st.diff;
        self.save.store();
        self.audio.play("select", 0.5);
    }

    /// Switches between a vigil until dawn and a boss rush; the host's choice applies to everyone.
    pub(super) fn toggle_boss_rush(&mut self) {
        self.save.settings.boss_rush = !self.save.settings.boss_rush;
        self.save.store();
        self.audio.play("select", 0.5);
    }

    /// After a vigil online, everyone goes back to the same lobby, still connected,
    /// to pick a hero and go again.
    pub(super) fn return_to_lobby(&mut self) {
        self.world = None;
        self.tips.clear();
        self.note = None;
        self.screen = Screen::Lobby;
        self.lobby.sel = 0;
        self.audio.music(self.menu_music());
        match &mut self.net {
            NetMode::Host(h) => {
                h.started = false;
                h.left.clear();
                self.lobby.state = LobbyState::Hosting;
                self.lobby.started = false;
                self.lobby.msg = Some(("Back in the lobby. Pick your hero and begin again when everyone is ready.".into(), TEAL));
            }
            NetMode::Client(_) => {
                self.lobby.state = LobbyState::Joined;
                // Until the host says the old vigil is over, its snapshots must not start a new one.
                self.lobby.started = true;
                self.lobby.msg = Some(("Back in the lobby. Pick your hero; the host begins the next vigil.".into(), TEAL));
            }
            NetMode::Offline => self.screen = Screen::Menu,
        }
        crate::log!("back in the lobby: {}", self.log_status());
    }

    pub(super) fn leave_online(&mut self) {
        crate::net::forget_upload();
        match &mut self.net {
            NetMode::Host(h) => h.close(),
            NetMode::Client(c) => c.leave(),
            NetMode::Offline => {}
        }
        self.net = NetMode::Offline;
        self.lobby.state = LobbyState::Browse;
        self.lobby.started = false;
        self.lobby.players.clear();
    }

    /// Keeps the connection alive while waiting; returns true when the run starts.
    fn lobby_network(&mut self, dt: f32) -> bool {
        let now = get_time();
        let my_class = class_to_u8(self.class());
        let name = self.player_name();
        match &mut self.net {
            NetMode::Host(host) => {
                for ev in host.poll(now) {
                    match ev {
                        HostEvent::Joined(n) => {
                            crate::log!("{} joined the lobby", n);
                            self.lobby.msg = Some((format!("{} joined the lobby.", n), TEAL));
                            self.audio.play("bell", 0.4);
                        }
                        HostEvent::Left(n, _) => {
                            crate::log!("{} left the lobby", n);
                            self.lobby.msg = Some((format!("{} left the lobby.", n), DIM));
                        }
                        HostEvent::Rejoined(..) => {}
                        HostEvent::Log(name, reason, text) => store_player_log(self.auto.as_ref(), &name, &reason, &text),
                    }
                }
                if let Some(v) = host.sock.stats.foreign_version.take() {
                    crate::log!("a player with protocol {} tried to join (this build: {})", v, net::VERSION);
                    self.lobby.msg = Some((format!("Someone with another version of the game tried to join. Everyone needs v{}.", net::VERSION), BLOOD));
                }
                let players = host.lobby_players(&name, my_class);
                host.tick_lobby(dt, now, &players, &self.save.settings.diff, self.save.settings.boss_rush, self.save.challenges);
                self.lobby.players = players;
                self.lobby.diff = self.save.settings.diff;
                self.lobby.rush = self.save.settings.boss_rush;
                self.lobby.challenges = self.save.challenges;
            }
            NetMode::Client(client) => {
                client.tick(dt, now, &name, my_class, &self.save.profile);
                let mut start = false;
                for ev in client.poll(now) {
                    match ev {
                        ClientEvent::Welcome => {
                            crate::log!("joined the lobby");
                            self.lobby.state = LobbyState::Joined;
                            self.lobby.msg = Some(("Joined. Waiting for the host to begin the vigil.".into(), TEAL));
                            self.audio.play("bell", 0.4);
                        }
                        ClientEvent::Refused(why) => {
                            crate::log!("refused by the host: {}", why);
                            self.lobby.msg = Some((why, BLOOD));
                            self.lobby.state = LobbyState::Browse;
                        }
                        ClientEvent::Lobby { started, players, diff, rush, challenges } => {
                            self.lobby.players = players;
                            self.lobby.diff = diff;
                            self.lobby.rush = rush;
                            self.lobby.challenges = challenges;
                            start |= started && !self.lobby.started;
                            self.lobby.started = started;
                        }
                        // The first snapshot can overtake the lobby packet that says "started";
                        // snapshots of a vigil that already ended don't count.
                        ClientEvent::Snapshot(_) if !self.lobby.started => start = true,
                        ClientEvent::Snapshot(_) => {}
                        ClientEvent::HostLeft => {
                            let msg = if matches!(self.lobby.state, LobbyState::Joining(_)) {
                                "No answer. Check the address, that the host has the lobby open, UDP port 7777 is allowed in their firewall and everyone runs the same version."
                            } else {
                                "The host closed the lobby or cannot be reached."
                            };
                            self.lobby.msg = Some((msg.into(), BLOOD));
                            self.lobby.state = LobbyState::Browse;
                        }
                    }
                }
                if self.lobby.state == LobbyState::Browse {
                    self.net = NetMode::Offline;
                } else if start && self.lobby.state == LobbyState::Joined {
                    return true;
                }
            }
            NetMode::Offline => {}
        }
        if let LobbyState::Joining(t) = self.lobby.state {
            let t = t + dt;
            self.lobby.state = LobbyState::Joining(t);
            if t > 8.0 {
                self.leave_online();
                self.lobby.msg = Some((
                    "No answer. Check the address, that the host has the lobby open, UDP port 7777 is allowed in their firewall and everyone runs the same version.".into(),
                    BLOOD,
                ));
            }
        }
        false
    }

    pub(super) fn update_lobby(&mut self, dt: f32) {
        if self.lobby.ts_job.as_ref().map(|j| j.is_finished()).unwrap_or(false) {
            let job = self.lobby.ts_job.take().unwrap();
            self.lobby.ts = job.join().ok();
        }
        if self.lobby_network(dt) {
            self.start_client_world();
            return;
        }
        let rows = self.lobby_rows();
        let text_row = self.lobby.state == LobbyState::Browse && (self.lobby.sel == 0 || self.lobby.sel == 3);
        if nav(&mut self.lobby.sel, rows, text_row) {
            self.audio.play("select", 0.4);
        }
        let mouse = mouse_rows(&mut self.lobby.sel);
        if back() {
            if self.lobby.state == LobbyState::Browse {
                self.screen = Screen::Menu;
                self.save.store();
            } else {
                self.leave_online();
            }
            return;
        }
        match self.lobby.state {
            LobbyState::Browse => self.update_browse(rows, text_row, mouse),
            LobbyState::Hosting => {
                drain_chars();
                let dir = if left(false) || mouse.right {
                    -1
                } else if right(false) || mouse.left {
                    1
                } else {
                    0
                };
                let go = confirm() || mouse.left;
                match self.lobby.sel {
                    0 if dir != 0 => self.cycle_class(dir),
                    1 if dir != 0 => self.cycle_difficulty(dir),
                    2 if dir != 0 => self.toggle_boss_rush(),
                    s if (3..3 + ALL_CHALLENGES.len()).contains(&s) && (dir != 0 || go) => {
                        self.save.challenges ^= ALL_CHALLENGES[s - 3].bit();
                        self.save.store();
                        self.audio.play("select", 0.5);
                    }
                    s if s == 3 + ALL_CHALLENGES.len() && go => self.start_hosted(),
                    s if s == 4 + ALL_CHALLENGES.len() && go => self.leave_online(),
                    _ => {}
                }
            }
            LobbyState::Joining(_) | LobbyState::Joined => {
                drain_chars();
                let dir = if left(false) || mouse.right {
                    -1
                } else if right(false) || mouse.left {
                    1
                } else {
                    0
                };
                match self.lobby.sel {
                    0 if dir != 0 => self.cycle_class(dir),
                    1 if confirm() || mouse.left => self.leave_online(),
                    _ => {}
                }
            }
        }
    }

    fn update_browse(&mut self, rows: usize, text_row: bool, mouse: MouseRow) {
        let sel = self.lobby.sel;
        match sel {
            0 => edit_text(&mut self.save.settings.name, 16, name_char),
            3 => edit_text(&mut self.lobby.ip, 64, addr_char),
            _ => drain_chars(),
        }
        if sel == 1 {
            if left(false) || mouse.right {
                self.cycle_class(-1);
            }
            if right(false) || mouse.left {
                self.cycle_class(1);
            }
        }
        // A click on the name row only puts the cursor there; elsewhere it acts like Enter.
        let enter = pressed_any(&[KeyCode::Enter, KeyCode::KpEnter]) || (!text_row && is_key_pressed(KeyCode::Space)) || (mouse.left && sel != 0 && sel != 1);
        if !enter {
            return;
        }
        let peers = self.ts_peers();
        match sel {
            2 => self.host_lobby(),
            3 => {
                let ip = self.lobby.ip.clone();
                self.join(&ip);
            }
            4 => self.refresh_tailscale(),
            s if s >= 5 && s < 5 + peers.len() => {
                let ip = peers[s - 5].ip.clone();
                self.lobby.ip = ip.clone();
                self.join(&ip);
            }
            s if s == rows - 1 => {
                self.screen = Screen::Menu;
                self.save.store();
            }
            _ => {}
        }
    }

    pub(super) fn draw_lobby(&self, ui: &Ui) {
        let (sw, sh) = (ui_w(), ui_h());
        let cx = sw * 0.5;
        ui.spaced_c("ONLINE LOBBY", cx, 60.0, 40.0, 8.0, PALE);
        let w = 680.0f32.min(sw - 40.0);
        let x = cx - w * 0.5;
        let mut y = 90.0;
        let caret = if (self.t * 2.0) as i32 % 2 == 0 { "_" } else { " " };

        // Our addresses, so the host can share them.
        let ts = self.lobby.ts.as_ref();
        let lan = self.lobby.lan.map(|i| format!("{}:{}", i, game_port())).unwrap_or("-".into());
        let ts_line = match ts {
            None => "Tailscale: checking...".to_string(),
            Some(t) if t.running => format!("Tailscale: {}:{}  ({})", t.self_ip.clone().unwrap_or("-".into()), game_port(), t.self_name),
            Some(t) => format!("Tailscale: {}", t.error.clone().unwrap_or("not running".into())),
        };
        ui.text(&format!("LAN: {}", lan), x, y + 16.0, 16.0, DIM);
        ui.text_r(&ts_line, x + w, y + 16.0, 16.0, if ts.map(|t| t.running).unwrap_or(false) { TEAL } else { DIM });
        y += 32.0;

        match self.lobby.state {
            LobbyState::Browse => {
                let peers = self.ts_peers();
                let row = |i: usize, label: &str, value: &str, y: &mut f32, accent: Color| {
                    let on = self.lobby.sel == i;
                    ui.row(x, *y, w, 34.0, on);
                    ui.text(label, x + 14.0, *y + 23.0, 19.0, accent);
                    ui.text_r(value, x + w - 14.0, *y + 23.0, 18.0, if on { GOLD } else { DIM });
                    *y += 38.0;
                };
                let name = format!("{}{}", self.save.settings.name, if self.lobby.sel == 0 { caret } else { "" });
                row(0, "Name", &name, &mut y, PALE);
                row(1, "Class", &format!("<  {}  >", self.class().name()), &mut y, PALE);
                row(2, "Host a vigil", &format!("UDP port {}", game_port()), &mut y, EMBER);
                let shown = if self.lobby.ip.is_empty() && self.lobby.sel != 3 { "ip or name[:port]" } else { &self.lobby.ip };
                row(3, "Join by address", &format!("{}{}", shown, if self.lobby.sel == 3 { caret } else { "" }), &mut y, EMBER);
                row(4, "Refresh Tailscale", if self.lobby.ts_job.is_some() { "..." } else { "" }, &mut y, PALE);
                if !peers.is_empty() {
                    ui.text("Tailscale devices online:", x, y + 16.0, 15.0, TEAL);
                    y += 24.0;
                }
                for (k, p) in peers.iter().enumerate() {
                    row(5 + k, &format!("Join {}", p.name), &format!("{}  {}", p.ip, p.os), &mut y, TEAL);
                }
                row(5 + peers.len(), "Back", "", &mut y, DIM);
            }
            LobbyState::Hosting | LobbyState::Joined | LobbyState::Joining(_) => {
                let title = match self.lobby.state {
                    LobbyState::Hosting => "You are hosting",
                    LobbyState::Joined => "Joined a vigil",
                    _ => "Connecting...",
                };
                ui.text(title, x, y + 22.0, 24.0, GOLD);
                y += 40.0;
                for p in &self.lobby.players {
                    draw_rectangle(x, y + 2.0, 12.0, 12.0, slot_color(p.slot));
                    let ping = match &self.net {
                        NetMode::Host(h) if p.slot != 0 => h.peers.iter().find(|q| q.slot == p.slot).map(|q| format!("{:.0} ms", q.ping_ms)).unwrap_or_default(),
                        _ => String::new(),
                    };
                    let host_tag = if p.slot == 0 { "  (host)" } else { "" };
                    ui.text(&format!("{}{}", p.name, host_tag), x + 22.0, y + 14.0, 20.0, PALE);
                    ui.text_r(&format!("{}   {}", class_from_u8(p.class).name(), ping), x + w, y + 14.0, 18.0, DIM);
                    y += 28.0;
                }
                let d = self.lobby.diff;
                ui.text(
                    &format!(
                        "{}   Difficulty: {}  (enemies {:.0}%, health {:.0}%, damage {:.0}%)",
                        if self.lobby.rush { "Boss Rush" } else { "Vigil" },
                        d.preset_name(),
                        d.count * 100.0,
                        d.hp * 100.0,
                        d.dmg * 100.0
                    ),
                    x,
                    y + 18.0,
                    16.0,
                    DIM,
                );
                y += 22.0;
                let chosen: Vec<&str> = ALL_CHALLENGES.iter().filter(|c| self.lobby.challenges & c.bit() != 0).map(|c| c.name()).collect();
                if !chosen.is_empty() {
                    let bonus = (challenge_mult(self.lobby.challenges) - 1.0) * 100.0;
                    ui.text(&format!("Challenges for everyone: {}   (+{:.0}% marks)", chosen.join(", "), bonus), x, y + 14.0, 16.0, EMBER);
                    y += 22.0;
                }
                y += 14.0;
                let value_row = |i: usize, label: &str, value: &str, y: &mut f32| {
                    let on = self.lobby.sel == i;
                    ui.row(x, *y, w, 34.0, on);
                    ui.text(label, x + 14.0, *y + 23.0, 19.0, PALE);
                    ui.text_r(value, x + w - 14.0, *y + 23.0, 18.0, if on { GOLD } else { DIM });
                    *y += 38.0;
                };
                let button = |i: usize, label: &str, col: Color, y: &mut f32| {
                    ui.row(x, *y, w, 34.0, self.lobby.sel == i);
                    ui.text_c(label, cx, *y + 23.0, 20.0, col);
                    *y += 38.0;
                };
                value_row(0, "Your hero", &format!("<  {}  >", self.class().name()), &mut y);
                if self.lobby.state == LobbyState::Hosting {
                    value_row(1, "Difficulty", &format!("<  {}  >", self.save.settings.diff.preset_name()), &mut y);
                    value_row(2, "Mode", &format!("<  {}  >", mode_name(self.save.settings.boss_rush)), &mut y);
                    for (k, ch) in ALL_CHALLENGES.iter().enumerate() {
                        let on = self.save.challenges & ch.bit() != 0;
                        value_row(3 + k, &format!("Challenge: {}", ch.name()), if on { "On  (+25% marks)" } else { "Off" }, &mut y);
                    }
                    let n = 3 + ALL_CHALLENGES.len();
                    button(n, if self.save.settings.boss_rush { "Begin the Boss Rush" } else { "Begin the Vigil" }, EMBER, &mut y);
                    button(n + 1, "Close lobby", DIM, &mut y);
                } else {
                    button(1, "Leave", DIM, &mut y);
                }
            }
        }
        if let Some((m, c)) = &self.lobby.msg {
            ui.text_c(m, cx, sh - 54.0, 17.0, *c);
        }
        ui.text_c("Tailscale: all players in the same tailnet, then pick the host's device or enter its 100.x address", cx, sh - 24.0, 15.0, DIM);
    }
}

/// The mode as the menus show it.
pub(super) fn mode_name(rush: bool) -> &'static str {
    if rush {
        "Boss Rush: twelve lords, one after another"
    } else {
        "The Vigil: hold out until dawn"
    }
}

/// Keeps a log a player sent: next to the host's own logs, or in the test folder.
pub(super) fn store_player_log(auto: Option<&AutoTest>, name: &str, reason: &str, text: &str) {
    let dir = match auto {
        Some(a) => a.log_file().parent().map(|p| p.to_path_buf()).unwrap_or_default(),
        None => crate::log::log_dir(),
    };
    match crate::log::store_received(&dir, name, reason, text) {
        Some(p) => crate::log!("received {}'s log ({}), saved as {}", name, reason, p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()),
        None => crate::log!("received {}'s log ({}) but could not save it", name, reason),
    }
}
