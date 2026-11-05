//! The application: screens, menus, network roles and the bridge to the save file.

mod autotest;
mod game;
pub(crate) mod input;
mod lobby;
mod menus;
mod records;
mod wardrobe;

use crate::audio::{Audio, Music};
use crate::data::*;
use crate::net::{self, ClientNet, HostNet, LobbyPlayer, TsStatus};
use crate::save::{Action, Save};
use crate::snapshot;
use crate::ui::{mono, set_ui_camera, set_ui_scale, ui_h, ui_w, Ui, DIM, PALE};
use crate::world::*;
use input::*;
use macroquad::prelude::*;
use macroquad::rand::gen_range;
use std::thread::JoinHandle;

pub use autotest::AutoTest;

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Screen {
    Menu,
    Options,
    Hub,
    Reliquary,
    Wardrobe,
    Credits,
    Records,
    Lobby,
    Playing,
    Paused,
    Dead { t: f32, lost: u64, marks: u64 },
    Victory { t: f32, marks: u64 },
}

pub enum NetMode {
    Offline,
    Host(HostNet),
    Client(ClientNet),
}

#[derive(Clone, Copy, PartialEq)]
pub enum LobbyState {
    Browse,
    Hosting,
    Joining(f32),
    Joined,
}

pub struct Lobby {
    pub state: LobbyState,
    pub sel: usize,
    pub ip: String,
    pub msg: Option<(String, Color)>,
    pub players: Vec<LobbyPlayer>,
    pub diff: Difficulty,
    /// The host chose a boss rush, and these challenge bits.
    pub rush: bool,
    pub challenges: u8,
    pub biome: u8,
    pub curses: u8,
    pub started: bool,
    pub ts: Option<TsStatus>,
    pub ts_job: Option<JoinHandle<TsStatus>>,
    pub lan: Option<std::net::IpAddr>,
}

struct TitleEmber {
    pos: Vec2,
    vel: Vec2,
    life: f32,
}

pub struct App {
    pub screen: Screen,
    pub save: Save,
    pub audio: Audio,
    pub world: Option<World>,
    pub net: NetMode,
    pub lobby: Lobby,
    menu_sel: usize,
    opt_sel: usize,
    opt_page: usize,
    /// A client on the end screen saw the host go back to the lobby.
    follow_host: bool,
    /// The binding row waiting for a key press.
    binding: Option<usize>,
    hub_sel: usize,
    relic_sel: usize,
    /// Reliquary page: 0 soul upgrades, 1 relics and unlocks.
    relic_page: usize,
    wardrobe_sel: usize,
    /// The selected button of the pause menu.
    pause_sel: usize,
    /// The options were opened from the pause menu: Back returns there.
    options_in_game: bool,
    /// "Leave the vigil" was chosen once and waits for a second confirm.
    leave_armed: bool,
    /// The sound was turned off by us because the window lost focus.
    muted_away: bool,
    /// The lord's words last heard and how long they had left, to speak each new line once.
    heard_quote: Option<(String, f32)>,
    discord: crate::discord::Discord,
    /// Newer versions on GitHub, offered in the main menu.
    updater: crate::update::Updater,
    /// Unix seconds when the current vigil began, for Discord's clock.
    vigil_since: i64,
    /// A tiny world for drawing heroes in the wardrobe.
    preview: Option<World>,
    /// "party" was typed in the main menu: the embers change colour.
    party: bool,
    /// Our rolls so far and when the last ones happened, for the dizzy secret.
    last_rolls: u32,
    roll_times: Vec<f32>,
    said_404: bool,
    credits_scroll: f32,
    level_sel: usize,
    class_idx: usize,
    konami: Vec<KeyCode>,
    /// Clicks on the title, for the blue embers.
    title_clicks: u32,
    /// Letters typed in the main menu, for the secret words.
    typed: String,
    /// Seconds left of the cat walking across the main menu.
    menu_cat: f32,
    /// How long a volume key has been held, for quick repeats.
    opt_hold: f32,
    note: Option<(String, f32)>,
    vignette: Texture2D,
    grain: Texture2D,
    font: Option<Font>,
    /// The text scale the glyph atlas was last filled for.
    glyphs_for: f32,
    t: f32,
    embers: Vec<TitleEmber>,
    pub auto: Option<AutoTest>,
    snap_dec: snapshot::Decoder,
    since_snap: f32,
    pick_pending: Option<(u16, u8)>,
    reroll_seq: u8,
    skill_seq: u8,
    synced_banked: u64,
    /// Complete snapshots received this vigil (for the network tests).
    snapshots: u32,
    /// Smoothed fight intensity for the music.
    intensity: f32,
    run_log: records::RunLog,
    run_achievements: Vec<Achievement>,
    popups: Vec<(String, f32)>,
    summary: records::EndSummary,
    records_page: usize,
    /// Marks already paid when an endless vigil began, and its leaderboard entry.
    endless_paid: u64,
    endless_record: Option<RunRecord>,
    /// Beginner tips still to show in the first vigil: (vigil time, text).
    tips: Vec<(f32, String)>,
    input_acc: f32,
    level_round_seen: u16,
    level_shown_at: f64,
    /// A level-up card was just taken with a key that also rolls: no roll until then.
    roll_blocked_until: f64,
    lighting: Option<crate::draw::Lighting>,
}

fn make_vignette() -> Texture2D {
    let n = 256u16;
    let mut bytes = Vec::with_capacity(n as usize * n as usize * 4);
    for y in 0..n {
        for x in 0..n {
            let dx = (x as f32 + 0.5) / n as f32 * 2.0 - 1.0;
            let dy = (y as f32 + 0.5) / n as f32 * 2.0 - 1.0;
            let d = (dx * dx + dy * dy).sqrt();
            let v = ((d - 0.3) / 0.9).clamp(0.0, 1.0).powf(1.4) * 0.85;
            bytes.extend_from_slice(&[255, 255, 255, (v * 255.0) as u8]);
        }
    }
    let t = Texture2D::from_rgba8(n, n, &bytes);
    t.set_filter(FilterMode::Linear);
    t
}

/// Film grain, tiled over the screen with a random offset every frame.
fn make_grain() -> Texture2D {
    let n = 128u16;
    let bytes: Vec<u8> = (0..n as usize * n as usize)
        .flat_map(|_| {
            let v = gen_range(0u8, 255u8);
            [v, v, v, 255]
        })
        .collect();
    let t = Texture2D::from_rgba8(n, n, &bytes);
    t.set_filter(FilterMode::Nearest);
    t
}

impl App {
    pub fn new(save: Save, audio: Audio, font: Option<Font>, auto: Option<AutoTest>) -> App {
        let check_updates = save.settings.check_updates;
        App {
            screen: Screen::Menu,
            save,
            audio,
            world: None,
            net: NetMode::Offline,
            lobby: Lobby {
                state: LobbyState::Browse,
                sel: 0,
                ip: String::new(),
                msg: None,
                players: Vec::new(),
                diff: Difficulty::default(),
                rush: false,
                challenges: 0,
                biome: 0,
                curses: 0,
                started: false,
                ts: None,
                ts_job: None,
                lan: None,
            },
            menu_sel: 0,
            opt_sel: 0,
            opt_page: 0,
            binding: None,
            follow_host: false,
            hub_sel: 0,
            relic_sel: 0,
            relic_page: 0,
            wardrobe_sel: 0,
            pause_sel: 0,
            options_in_game: false,
            leave_armed: false,
            muted_away: false,
            heard_quote: None,
            // The self-test never talks to Discord.
            discord: if auto.is_some() && std::env::var("ASHEN_AUTOTEST_DISCORD").is_err() && !std::env::var("ASHEN_AUTOTEST_ROLE").is_ok_and(|r| r == "menus") {
                crate::discord::Discord::off()
            } else {
                crate::discord::Discord::start()
            },
            // The self-test never asks GitHub; the player may turn the check off.
            updater: if auto.is_some() || !check_updates { crate::update::Updater::off() } else { crate::update::Updater::start() },
            vigil_since: 0,
            preview: None,
            party: false,
            last_rolls: 0,
            roll_times: Vec::new(),
            said_404: false,
            credits_scroll: 0.0,
            level_sel: 0,
            class_idx: 0,
            konami: Vec::new(),
            title_clicks: 0,
            typed: String::new(),
            menu_cat: 0.0,
            opt_hold: 0.0,
            note: None,
            vignette: make_vignette(),
            grain: make_grain(),
            font,
            glyphs_for: 0.0,
            t: 0.0,
            embers: Vec::new(),
            auto,
            snap_dec: Default::default(),
            since_snap: 0.0,
            pick_pending: None,
            reroll_seq: 0,
            skill_seq: 0,
            synced_banked: 0,
            snapshots: 0,
            intensity: 0.0,
            run_log: records::RunLog::default(),
            run_achievements: Vec::new(),
            popups: Vec::new(),
            summary: records::EndSummary::default(),
            records_page: 0,
            endless_paid: 0,
            endless_record: None,
            tips: Vec::new(),
            input_acc: 0.0,
            level_round_seen: 0,
            level_shown_at: 0.0,
            roll_blocked_until: 0.0,
            lighting: crate::draw::Lighting::new(),
        }
    }

    /// Classes this player may pick.
    fn classes(&self) -> Vec<Class> {
        ALL_CLASSES.iter().copied().filter(|c| if *c == Class::BucketKnight { self.save.bucket_unlocked } else { self.save.profile.has(Unlock::Class(*c)) }).collect()
    }

    fn class(&self) -> Class {
        let c = self.classes();
        c[self.class_idx.min(c.len() - 1)]
    }

    fn cycle_class(&mut self, dir: i32) {
        let n = self.classes().len() as i32;
        self.class_idx = ((self.class_idx as i32 + dir).rem_euclid(n)) as usize;
        self.audio.play("select", 0.4);
    }

    fn online(&self) -> bool {
        !matches!(self.net, NetMode::Offline)
    }

    fn net_stats(&self) -> Option<&net::NetStats> {
        match &self.net {
            NetMode::Host(h) => Some(&h.sock.stats),
            NetMode::Client(c) => Some(&c.sock.stats),
            NetMode::Offline => None,
        }
    }

    /// Our display name, never empty.
    fn player_name(&self) -> String {
        let n = self.save.settings.name.trim();
        if n.is_empty() {
            "Ashen One".into()
        } else {
            n.to_string()
        }
    }

    /// One line on what is going on, for the log.
    pub fn log_status(&self) -> String {
        let mode = match &self.net {
            NetMode::Host(h) => format!("host of {} players", h.peers.len() + 1),
            NetMode::Client(_) => "client".to_string(),
            _ => "offline".to_string(),
        };
        match &self.world {
            Some(w) if !w.heroes.is_empty() => format!(
                "{:?}, {}, vigil {:02}:{:02}, level {}, {} foes, {} kills{}",
                self.screen,
                mode,
                w.time as u32 / 60,
                w.time as u32 % 60,
                w.level,
                w.enemies.len(),
                w.kills,
                if w.endless { ", endless" } else { "" }
            ),
            _ => format!("{:?}, {}", self.screen, mode),
        }
    }

    /// What Discord shows: where the player is and, in a vigil, which kind and how far.
    fn presence(&self) -> crate::discord::Presence {
        use crate::discord::Presence;
        let menus = |s: &str| Presence { details: s.to_string(), state: String::new(), since: None, party: None };
        match self.screen {
            Screen::Lobby => {
                let n = self.lobby.players.len().max(1) as u32;
                let state = match self.lobby.state {
                    LobbyState::Hosting => "Hosting",
                    LobbyState::Joined => "Waiting for the host",
                    LobbyState::Joining(_) => "Joining",
                    LobbyState::Browse => "Looking for a vigil",
                };
                Presence { details: "In the online lobby".into(), state: state.into(), since: None, party: (n > 1).then_some((n, 4)) }
            }
            Screen::Playing | Screen::Paused | Screen::Dead { .. } | Screen::Victory { .. } | Screen::Options if self.world.as_ref().is_some_and(|w| !w.heroes.is_empty()) => {
                let w = self.world.as_ref().expect("checked above");
                let online = self.online();
                let kind = match (w.boss_rush, w.endless) {
                    (true, true) => "Endless boss rush",
                    (true, false) => "Boss Rush",
                    (false, true) => "Endless night",
                    (false, false) => "Vigil",
                };
                let details = if online { format!("{}, online", kind) } else { format!("{} on {}", kind, w.biome.name()) };
                let state = match self.screen {
                    Screen::Dead { .. } => format!("Fell at level {}", w.level),
                    Screen::Victory { .. } if w.boss_rush => "Felled every lord".to_string(),
                    Screen::Victory { .. } => "Kept the vigil until dawn".to_string(),
                    _ if w.boss_rush && !w.endless => format!("Lord {} of {}, level {}", (w.bosses_killed + 1).min(RUSH_LORDS), RUSH_LORDS, w.level),
                    _ if w.boss_kind.is_some() => format!("Fighting {}", w.boss_kind.map(|k| k.name()).unwrap_or("")),
                    _ => format!("Level {}, {} felled", w.level, w.kills / 100 * 100),
                };
                let party = online.then_some((w.heroes.iter().filter(|h| h.connected).count() as u32, 4));
                Presence { details, state, since: Some(self.vigil_since), party }
            }
            Screen::Hub => menus("At the Ash Shrine"),
            Screen::Wardrobe => menus("In the Wardrobe"),
            _ => menus("In the menus"),
        }
    }

    /// Speaks a lord's words aloud when they appear, on the host and on clients alike (clients
    /// get the words in snapshots). A line said again shows again with its time renewed.
    fn speak_quotes(&mut self) {
        let in_vigil = match self.screen {
            Screen::Playing | Screen::Paused | Screen::Dead { .. } | Screen::Victory { .. } => true,
            Screen::Options => self.options_in_game,
            _ => false,
        };
        let quote = self.world.as_ref().filter(|w| in_vigil && w.boss_lines).and_then(|w| w.quote.as_ref()).map(|(line, _, life)| (line.clone(), *life));
        let fresh = match (&quote, &self.heard_quote) {
            (Some((line, life)), Some((old, old_life))) => line != old || *life > old_life + 0.5,
            (Some(_), None) => true,
            _ => false,
        };
        if fresh {
            if let Some(key) = quote.as_ref().and_then(|(line, _)| crate::voice::line_key(line)) {
                crate::log!("a lord speaks: {}", key);
                self.audio.speak(&key, key.ends_with("_3"));
            }
        }
        self.heard_quote = quote;
    }

    fn set_note(&mut self, text: &str, secs: f32) {
        self.note = Some((text.to_string(), secs));
    }

    pub fn update(&mut self, dt: f32) {
        poll_gamepad();
        if let Some(w) = self.world.as_mut() {
            w.cute = self.save.settings.cute;
        }
        set_binds(self.save.settings.binds);
        crate::ui::set_marker_inset(self.save.settings.marker_inset);
        let presence = if self.save.settings.discord { Some(self.presence()) } else { None };
        self.discord.set(presence);
        crate::ui::set_hurt_flash(self.save.settings.hurt_flash);
        crate::ui::set_hero_bars(self.save.settings.hero_hp_bar, self.save.settings.hero_stamina_bar);
        crate::draw::set_brightness(self.save.settings.brightness as f32 / 100.0);
        crate::draw::set_effect_alpha(self.save.settings.effect_alpha as f32 / 100.0);
        if let Some(w) = self.world.as_mut() {
            w.boss_lines = self.save.settings.boss_lines;
        }
        self.speak_quotes();
        self.audio.update_voice();
        // Quiet while another window has the focus, if wanted; the player's own mute stays theirs.
        let away = self.auto.is_none() && self.save.settings.mute_unfocused && !game::window_focused();
        if away && !self.muted_away && !self.audio.muted {
            self.audio.toggle_mute();
            self.muted_away = true;
        } else if !away && self.muted_away {
            // Only undo the mute this did itself.
            if self.audio.muted {
                self.audio.toggle_mute();
            }
            self.muted_away = false;
        }
        self.t += dt;
        // The main menu listens for secret words, so M must stay a letter there.
        let typing = matches!(self.screen, Screen::Options | Screen::Lobby | Screen::Menu);
        if act_pressed(Action::Mute) && !typing {
            self.audio.toggle_mute();
        }
        if is_key_pressed(KeyCode::F3) {
            self.save.settings.overlay = !self.save.settings.overlay;
        }
        if is_key_pressed(KeyCode::F11) {
            self.save.settings.fullscreen = !self.save.settings.fullscreen;
            set_fullscreen(self.save.settings.fullscreen);
        }
        if let Some((_, life)) = &mut self.note {
            *life -= dt;
            if *life <= 0.0 {
                self.note = None;
            }
        }
        match self.screen {
            Screen::Menu => self.update_menu(),
            Screen::Options => {
                self.update_options();
                // Online the vigil goes on while someone looks at the options.
                if self.options_in_game && self.online() {
                    self.step_game(dt, Input::default(), false);
                }
            }
            Screen::Hub => self.update_hub(),
            Screen::Reliquary => self.update_reliquary(),
            Screen::Wardrobe => self.update_wardrobe(),
            Screen::Credits => self.update_credits(dt),
            Screen::Records => self.update_records(),
            Screen::Lobby => self.update_lobby(dt),
            Screen::Playing | Screen::Paused => self.update_playing(dt),
            Screen::Dead { t, lost, marks } => {
                self.screen = Screen::Dead { t: t + dt, lost, marks };
                self.step_game(dt, Input::default(), false);
                if t > 2.5 && (confirm() || clicked()) {
                    self.end_vigil();
                }
            }
            Screen::Victory { t, marks } => {
                self.screen = Screen::Victory { t: t + dt, marks };
                self.step_game(dt, Input::default(), false);
                // The host (or a lone player) picks with keys or the two buttons; a click
                // beside the buttons does nothing, so nobody leaves the endless night by accident.
                let chooser = !matches!(self.net, NetMode::Client(_));
                let button = clicked_tab();
                if t > 2.5 && chooser && (is_key_pressed(KeyCode::C) || pad_pressed(Pad::Reroll) || button == Some(1)) {
                    self.continue_endless();
                } else if t > 2.5 && (confirm() || button == Some(0) || (!chooser && clicked())) {
                    self.end_vigil();
                }
            }
        }
    }

    fn draw_embers(&mut self, dt: f32) {
        let (sw, sh) = (ui_w(), ui_h());
        if self.embers.len() < 90 {
            self.embers.push(TitleEmber { pos: vec2(gen_range(0.0, sw), sh + 10.0), vel: vec2(gen_range(-15.0, 15.0), gen_range(-60.0, -25.0)), life: gen_range(6.0, 14.0) });
        }
        for e in self.embers.iter_mut() {
            e.pos += e.vel * dt;
            e.pos.x += (self.t * 0.8 + e.life).sin() * 0.3;
            e.life -= dt;
            let k = (e.life / 3.0).min(1.0);
            let (glow, core) = if self.party {
                let h = (self.t * 0.3 + e.life * 0.21).fract();
                let c = macroquad::color::hsl_to_rgb(h, 0.9, 0.6);
                (Color::new(c.r, c.g, c.b, 0.1 * k), Color::new(c.r, c.g, c.b, 0.95 * k))
            } else if self.save.settings.cute {
                (Color::new(1.0, 0.5, 0.75, 0.08 * k), Color::new(1.0, 0.7, 0.85, 0.9 * k))
            } else if self.save.blue_embers {
                (Color::new(0.3, 0.6, 1.0, 0.08 * k), Color::new(0.5, 0.8, 1.0, 0.9 * k))
            } else {
                (Color::new(1.0, 0.5, 0.15, 0.08 * k), Color::new(1.0, 0.6, 0.25, 0.9 * k))
            };
            draw_circle(e.pos.x, e.pos.y, 5.0, glow);
            draw_circle(e.pos.x, e.pos.y, 1.6, core);
        }
        self.embers.retain(|e| e.life > 0.0 && e.pos.y > -10.0);
    }

    /// Dark backdrop with drifting embers and a fire glow at `glow_y`.
    fn backdrop(&mut self, dt: f32, glow_y: f32) {
        let cute = self.save.settings.cute;
        clear_background(if cute { Color::from_rgba(46, 26, 48, 255) } else { Color::from_rgba(7, 6, 8, 255) });
        self.draw_embers(dt);
        let fl = 0.85 + 0.15 * (self.t * 7.0).sin();
        let glow = if cute { Color::new(1.0, 0.55, 0.8, 0.012) } else { Color::new(1.0, 0.4, 0.12, 0.008) };
        for k in 0..10 {
            let r = (60.0 + k as f32 * 28.0) * fl;
            draw_circle(ui_w() * 0.5, glow_y, r, glow);
        }
    }

    fn draw_grain(&self) {
        let (sw, sh) = (ui_w(), ui_h());
        let size = 256.0;
        let off = vec2(gen_range(0.0, size), gen_range(0.0, size));
        let c = Color::new(1.0, 1.0, 1.0, 0.035);
        let mut y = -off.y;
        while y < sh {
            let mut x = -off.x;
            while x < sw {
                draw_texture_ex(&self.grain, x, y, c, DrawTextureParams { dest_size: Some(vec2(size, size)), ..Default::default() });
                x += size;
            }
            y += size;
        }
    }

    pub fn draw(&mut self, dt: f32) {
        set_ui_scale(self.save.settings.ui_scale);
        let k = crate::ui::raster_k();
        if k != self.glyphs_for {
            self.glyphs_for = k;
            crate::ui::warm_glyphs(self.font.as_ref());
        }
        set_ui_camera();
        crate::ui::clear_hit_areas();
        let font = self.font.clone();
        let ui = Ui { font: font.as_ref() };
        let (sw, sh) = (ui_w(), ui_h());
        match self.screen {
            Screen::Menu => {
                self.backdrop(dt, sh * 0.3);
                self.draw_menu(&ui);
            }
            Screen::Options => {
                if self.options_in_game && self.world.is_some() {
                    // Over the paused vigil, so it is clear the game is still there.
                    self.draw_game(&ui);
                    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.02, 0.02, 0.03, 0.82));
                } else {
                    self.backdrop(dt, 80.0);
                }
                self.draw_options(&ui);
            }
            Screen::Lobby => {
                self.backdrop(dt, 80.0);
                self.draw_lobby(&ui);
            }
            Screen::Hub => {
                self.backdrop(dt, 90.0);
                self.draw_hub(&ui);
            }
            Screen::Reliquary => {
                self.backdrop(dt, 90.0);
                self.draw_reliquary(&ui);
            }
            Screen::Wardrobe => {
                self.backdrop(dt, 90.0);
                self.draw_wardrobe(&ui);
            }
            Screen::Records => {
                self.backdrop(dt, 70.0);
                self.draw_records(&ui);
            }
            Screen::Credits => {
                self.backdrop(dt, 70.0);
                self.draw_credits(&ui);
            }
            _ => self.draw_game(&ui),
        }
        // Tips from the vigil wait while the pause menu or the options are open.
        let menu_over_vigil = self.screen == Screen::Paused || (self.screen == Screen::Options && self.options_in_game);
        if let Some((n, life)) = self.note.as_ref().filter(|_| !menu_over_vigil) {
            // In the main menu the last row reaches low on small screens; keep notes below it.
            let y = match self.screen {
                Screen::Menu => sh - 92.0,
                // Below the Ash Shrine's last button, above the key hints.
                Screen::Hub => sh - 58.0,
                _ => sh * 0.82,
            };
            ui.text_c(n, sw * 0.5, y, 20.0, Color::new(1.0, 0.75, 0.4, life.min(1.0)));
        }
        self.draw_popups(&ui, dt);
        if self.save.settings.overlay {
            ui.perf_overlay(self.net_stats());
        }
    }

    fn draw_game(&mut self, ui: &Ui) {
        let quality = self.save.settings.light_quality;
        if let Some(l) = self.lighting.as_mut() {
            l.fit_screen(quality);
        }
        let (sw, sh) = (ui_w(), ui_h());
        let Some(w) = &self.world else { return };
        if w.heroes.is_empty() {
            clear_background(Color::from_rgba(7, 6, 8, 255));
            ui.text_c("Waiting for the host...", sw * 0.5, sh * 0.5, 24.0, PALE);
            return;
        }
        let light = if self.save.settings.lighting { self.lighting.as_ref() } else { None };
        w.draw(light);
        set_ui_camera();
        ui.vignette(&self.vignette, w);
        if self.save.settings.grain {
            self.draw_grain();
        }
        let ended = matches!(self.screen, Screen::Dead { .. } | Screen::Victory { .. });
        if !ended {
            ui.hud(w);
            if self.save.settings.meter {
                ui.damage_meter(w, act_down(Action::Weapons) || pad_detail());
            }
            match w.pause {
                Pause::Choosing => ui.level_up(w, self.level_sel),
                Pause::Countdown(t) => ui.countdown(t),
                Pause::Running => {}
            }
        }
        match self.screen {
            Screen::Paused => self.draw_pause(ui),
            Screen::Dead { t, lost, .. } => self.draw_summary(ui, w, t, lost),
            Screen::Victory { t, .. } => self.draw_summary(ui, w, t, 0),
            _ => {}
        }
        let _ = DIM;
    }
}
