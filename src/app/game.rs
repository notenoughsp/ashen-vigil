//! Starting, running and ending a vigil for each network role.

use super::*;
use crate::net::{ClientEvent, ClientInput, HostEvent};

impl App {
    fn reset_run_state(&mut self) {
        self.beasts_added = [0; BEASTS.len()];
        self.last_rolls = 0;
        self.roll_times.clear();
        self.said_404 = false;
        self.snap_dec = Default::default();
        self.since_snap = 0.0;
        self.pick_pending = None;
        self.reroll_seq = 0;
        self.skill_seq = 0;
        self.synced_banked = 0;
        self.level_sel = 0;
    }

    fn new_world(&self, diff: Difficulty, remnant: Option<(Vec2, u64)>) -> World {
        let mut w = World::new(diff, remnant);
        // Clients learn the host's field from the first snapshot.
        w.biome = Biome::from_u8(self.save.settings.biome);
        w.screen = crate::draw::view_size();
        apply_settings(&mut w, &self.save.settings);
        w
    }

    /// Hands changed options to the running vigil.
    pub(super) fn settings_to_world(&mut self) {
        if let Some(w) = self.world.as_mut() {
            apply_settings(w, &self.save.settings);
        }
    }

    fn begin(&mut self, w: World) {
        self.vigil_since = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs() as i64).unwrap_or(0);
        self.reset_run_log();
        self.endless_paid = 0;
        self.endless_record = None;
        // The tips explain the vigil; a boss rush works differently.
        self.tips = if self.save.vigils == 0 && !w.boss_rush { first_vigil_tips() } else { Vec::new() };
        let mode = match &self.net {
            NetMode::Host(_) => "hosted",
            NetMode::Client(_) => "joined",
            _ => "single player",
        };
        let names: Vec<String> = w.heroes.iter().map(|h| format!("{} ({})", h.name, h.class.name())).collect();
        // A client learns the heroes from the first snapshot; its own pick is what it knows.
        let heroes = if names.is_empty() { format!("me as {}", self.class().name()) } else { names.join(", ") };
        let kind = if w.boss_rush { "boss rush" } else { "vigil" };
        // A client's world learns the host's settings only from the first snapshot: what the
        // lobby said is what is true.
        let client = matches!(self.net, NetMode::Client(_));
        let (field, diff, challenges, curses) = if client {
            (if self.lobby.rush { "a boss rush field" } else { Biome::from_u8(self.lobby.biome).name() }, self.lobby.diff, self.lobby.challenges, self.lobby.curses)
        } else {
            (w.biome.name(), w.diff, self.save.challenges, w.curses)
        };
        crate::log!(
            "{} started on {}, {}, difficulty {}{}, heroes: {}, challenges {:03b}, curses {:06b}",
            kind,
            field,
            mode,
            diff.preset_name(),
            if diff.boost == [0; 3] { String::new() } else { format!(" (empowered: {})", diff.boost_text()) },
            heroes,
            challenges,
            curses
        );
        self.world = Some(w);
        self.screen = Screen::Playing;
        self.audio.music(self.menu_music());
        self.audio.play("kindle", 0.6);
    }

    pub(super) fn start_single(&mut self) {
        self.reset_run_state();
        let mut w = self.new_world(self.save.settings.diff, self.save.remnant);
        let h = w.add_hero(0, &self.player_name(), self.class(), self.save.profile, false);
        w.heroes[h].challenges = self.save.challenges;
        w.heroes[h].recompute_stats();
        let max = w.heroes[h].stats.max_hp;
        w.heroes[h].p.hp = max;
        w.grant_head_start();
        if self.save.settings.boss_rush {
            w.start_boss_rush();
        }
        w.lay_curses(self.save.curses);
        if w.remnant.is_some() {
            w.toast("Your remnant still glows where you fell.");
        }
        self.begin(w);
    }

    pub(super) fn start_hosted(&mut self) {
        self.reset_run_state();
        let mut w = self.new_world(self.save.settings.diff, None);
        w.record_net = true;
        w.add_hero(0, &self.player_name(), self.class(), self.save.profile, false);
        let NetMode::Host(host) = &mut self.net else { return };
        for p in &host.peers {
            w.add_hero(p.slot, &p.name, class_from_u8(p.class), p.profile, true);
        }
        w.local = w.heroes.iter().position(|h| h.slot == 0).unwrap_or(0);
        // The host's challenges and curses hold for everyone.
        w.lay_curses(self.save.curses);
        let challenges = self.save.challenges;
        for h in w.heroes.iter_mut() {
            h.challenges = challenges;
            h.recompute_stats();
            h.p.hp = h.stats.max_hp;
        }
        w.grant_head_start();
        if self.save.settings.boss_rush {
            w.start_boss_rush();
        }
        // The crown's secret: a companion, by chance, takes every lord that comes alone.
        if self.usurp && !host.peers.is_empty() && !self.save.settings.boss_rush {
            let p = &host.peers[gen_range(0, host.peers.len())];
            w.usurper = Some(p.slot);
            crate::log!("{} wears the crown", p.name);
            w.banner(&format!("{} WEARS THE CROWN", p.name.to_uppercase()), "Tonight the lords answer to them", Color::from_rgba(200, 120, 255, 255), 5.0);
        }
        host.started = true;
        host.left.clear();
        let alone = w.heroes.len() == 1;
        self.begin(w);
        if alone {
            self.set_note("Nobody joined, the vigil begins alone.", 3.0);
        }
    }

    pub(super) fn start_client_world(&mut self) {
        self.reset_run_state();
        let mut w = self.new_world(self.lobby.diff, None);
        w.is_client = true;
        w.spectating = matches!(&self.net, NetMode::Client(c) if c.spectating);
        if w.spectating {
            crate::log!("watching the running vigil");
        }
        self.begin(w);
    }

    /// Leaves the end screen: online back to the lobby, alone back to the Ash Shrine.
    pub(super) fn end_vigil(&mut self) {
        if self.online() {
            self.return_to_lobby();
        } else {
            self.back_to_menu();
        }
    }

    pub(super) fn back_to_menu(&mut self) {
        self.tips.clear();
        self.note = None;
        let was_online = self.online();
        self.leave_online();
        self.world = None;
        self.screen = if was_online { Screen::Menu } else { Screen::Hub };
        self.audio.music(self.menu_music());
    }

    /// Adds newly banked souls to the save file as soon as they are banked.
    fn sync_bank(&mut self) {
        let Some(w) = &self.world else { return };
        if w.heroes.is_empty() || w.spectating {
            return;
        }
        let b = w.me().banked;
        if b > self.synced_banked {
            self.save.souls += b - self.synced_banked;
            self.synced_banked = b;
            self.save.store();
        }
    }

    /// Ashen Marks for the vigil that just ended, added to the save.
    fn award_marks(&mut self, won: bool) -> u64 {
        let w = self.world.as_ref().unwrap();
        // An endless vigil was won already; it pays what it earned beyond the first payout.
        let won = won || w.endless;
        let total = if w.time < 60.0 && !won { 0 } else { run_marks(w, won) };
        let base = total.saturating_sub(self.endless_paid);
        // Challenges only exist offline and pay extra.
        let mult = challenge_mult(w.me().challenges) * curse_mult(w.curses);
        let marks = (base as f32 * mult).round() as u64;
        self.save.marks += marks;
        self.save.best_time = self.save.best_time.max(w.time);
        if !w.endless {
            self.save.vigils += 1;
        }
        marks
    }

    /// Goes on past dawn: the clock keeps running, bosses keep coming, and the vigil ends
    /// only when the hero falls. What was paid at dawn stays paid.
    pub(super) fn continue_endless(&mut self) {
        crate::log!("endless night continues");
        let Some(w) = self.world.as_mut() else { return };
        w.won = false;
        w.endless = true;
        w.endless_since = w.time;
        if w.boss_rush {
            w.next_boss = w.time + RUSH_BREATHER;
            w.banner("THE LORDS KEEP COMING", "One after another, until you fall.", Color::from_rgba(200, 120, 90, 255), 4.0);
        } else {
            w.next_boss = w.time + 60.0;
            w.banner("THE NIGHT GOES ON", "Every five minutes another lord of ash comes.", Color::from_rgba(200, 120, 90, 255), 4.0);
        }
        // In the same units as `award_marks` before the challenge bonus.
        self.endless_paid = run_marks(w, true);
        self.endless_record = self.save.runs.iter().find(|r| r.won && (r.time - w.time).abs() < 0.5 && r.kills == w.kills).cloned();
        let field = self.world.as_ref().map(|w| w.biome as u8).unwrap_or(0);
        self.audio.music(Music::Endless(field, 0));
        self.screen = Screen::Playing;
    }

    /// What plays in the menus: the first vigil song, or the lullaby in cute mode.
    pub(super) fn menu_music(&self) -> Music {
        if self.save.settings.cute {
            Music::Lullaby
        } else {
            Music::Menu
        }
    }

    /// The song for this moment of the vigil when no boss is about.
    fn vigil_music(&self) -> Music {
        let Some(w) = &self.world else { return self.menu_music() };
        if w.endless {
            // Harsher every three minutes from the moment the endless night began.
            let stage = ((w.time - w.endless_since).max(0.0) / crate::audio::ENDLESS_STAGE) as u8;
            Music::Endless(w.biome as u8, stage.min(crate::audio::ENDLESS_STAGES - 1))
        } else if self.save.settings.cute {
            Music::Lullaby
        } else {
            Music::Vigil(w.biome as u8, ((w.time / 600.0) as u8).min(2))
        }
    }

    /// One beginner tip at a time, at its moment in the first vigil.
    fn show_tips(&mut self) {
        let Some(w) = &self.world else { return };
        if self.tips.first().is_some_and(|(at, _)| w.time >= *at) && self.note.is_none() {
            let (_, text) = self.tips.remove(0);
            self.set_note(&text, 6.0);
        }
    }

    /// The run's damage by weapon and the souls in hand, for the log at the end.
    fn log_summary(&self) {
        let Some(w) = self.world.as_ref().filter(|w| !w.heroes.is_empty()) else { return };
        let me = w.me();
        let mut by: Vec<(WeaponKind, f64)> = ALL_WEAPONS.iter().map(|k| (*k, me.dmg_weapon[k.idx()])).filter(|(_, d)| *d > 0.0).collect();
        by.sort_by(|a, b| b.1.total_cmp(&a.1));
        let top: Vec<String> = by.iter().take(6).map(|(k, d)| format!("{} {}", k.name(false), crate::ui::short(*d))).collect();
        crate::log!("damage by weapon: {}", top.join(", "));
        crate::log!(
            "souls in hand {}, banked {}, burden {}, lords felled {}, quests done {}, weapons: {}",
            me.p.souls,
            me.banked,
            me.burden,
            w.bosses_killed,
            w.quests_done,
            me.weapons.iter().map(|wp| wp.display_name()).collect::<Vec<_>>().join(", ")
        );
    }

    fn on_death(&mut self) {
        if self.spectating() {
            self.end_watching(false);
            return;
        }
        if self.world.as_ref().is_some_and(|w| w.withdrew) {
            self.on_withdraw();
            return;
        }
        crate::log!("vigil lost: {}", self.log_status());
        self.log_summary();
        if matches!(self.net, NetMode::Client(_)) {
            crate::net::upload_log("vigil");
        }
        let online = self.online();
        // The bestiary remembers what felled us.
        if let Some(w) = self.world.as_ref().filter(|w| !w.heroes.is_empty()) {
            if let Some(i) = w.killer_of(w.me().slot).and_then(beast_index) {
                self.save.beasts[i].1 = self.save.beasts[i].1.saturating_add(1);
            }
        }
        let marks = self.award_marks(false);
        let w = self.world.as_mut().unwrap();
        let lost = w.me().p.souls;
        if !online {
            // Dying again before reclaiming the old remnant loses it for good.
            self.save.remnant = if lost > 0 { Some((w.me().p.pos, lost)) } else { None };
        }
        self.finish_run(false, marks);
        self.save.store();
        self.audio.music(Music::Silence);
        self.screen = Screen::Dead { t: 0.0, lost, marks };
    }

    /// The party left through the ash gate: every carried soul is banked, an old remnant
    /// stays where it lies, and the vigil counts as ended without the dawn.
    fn on_withdraw(&mut self) {
        crate::log!("vigil left through the ash gate: {}", self.log_status());
        self.log_summary();
        if matches!(self.net, NetMode::Client(_)) {
            crate::net::upload_log("vigil");
        }
        self.sync_bank();
        let marks = self.award_marks(false);
        self.finish_run(false, marks);
        self.save.store();
        self.audio.music(Music::Silence);
        self.screen = Screen::Dead { t: 0.0, lost: 0, marks };
    }

    /// The vigil we watched is over: nothing of it is ours, no souls, marks or records.
    fn end_watching(&mut self, won: bool) {
        crate::log!("the watched vigil ended: {}", self.log_status());
        self.summary = super::records::EndSummary { won, marks: 0, challenge_bonus: false, rank: None, new_bests: Vec::new(), achievements: Vec::new() };
        self.audio.music(Music::Silence);
        self.screen = if won { Screen::Victory { t: 0.0, marks: 0 } } else { Screen::Dead { t: 0.0, lost: 0, marks: 0 } };
    }

    /// We joined a running vigil to watch it.
    pub(super) fn spectating(&self) -> bool {
        self.world.as_ref().is_some_and(|w| w.spectating)
    }

    fn on_victory(&mut self) {
        if self.spectating() {
            self.end_watching(true);
            return;
        }
        crate::log!("vigil kept until dawn: {}", self.log_status());
        self.log_summary();
        if matches!(self.net, NetMode::Client(_)) {
            crate::net::upload_log("vigil");
        }
        self.sync_bank();
        let marks = self.award_marks(true);
        self.finish_run(true, marks);
        self.save.store();
        self.audio.music(Music::Silence);
        self.screen = Screen::Victory { t: 0.0, marks };
    }

    fn handle_events(&mut self) {
        let Some(w) = self.world.as_mut() else { return };
        let sounds = std::mem::take(&mut w.sounds);
        for (name, vol, gap, pos) in sounds {
            let k = w.hearing(pos);
            if k > 0.02 {
                self.audio.play_gap(name, vol * k, gap);
            }
        }
        let events: Vec<Event> = std::mem::take(&mut w.events);
        let (client, dead, won, boss, empty, endless) = (w.is_client, w.dead, w.won, w.boss_kind.is_some(), w.heroes.is_empty(), w.endless);
        let boss_index = w.boss_kind.map(|k| k as usize).unwrap_or(0);
        // Drums swell with the number of enemies around us.
        let near = if empty {
            0
        } else {
            let me = w.me().p.pos;
            w.enemies.iter().filter(|e| e.pos.distance_squared(me) < 500.0 * 500.0).count()
        };
        self.intensity += ((near as f32 / 120.0).min(1.0) - self.intensity) * 0.02;
        self.audio.set_intensity(self.intensity);
        for ev in events {
            match ev {
                Event::BossStart => {
                    crate::log!("boss {} appears at {:.0}s", boss_index, self.world.as_ref().map(|w| w.time).unwrap_or(0.0));
                }
                Event::BossDefeated => {
                    let (fight, rush) = self.world.as_ref().map(|w| (w.time - w.boss_started, w.boss_rush)).unwrap_or((f32::MAX, false));
                    crate::log!("boss defeated after {:.1}s", fight);
                    if fight <= 10.0 && !rush {
                        self.unlock(Achievement::Executioner);
                    }
                }
                Event::RemnantClaimed if self.spectating() => {}
                Event::RemnantClaimed => {
                    self.save.remnant = None;
                    self.save.store();
                }
                Event::Died => self.on_death(),
                Event::Victory => self.on_victory(),
                Event::CatFound(slot) => self.cat_found(slot),
            }
        }
        self.sync_bank();
        // Clients learn about the end of the vigil from snapshot flags.
        // The host chose the endless night: follow it from the victory screen.
        if client && !empty && endless && !won && matches!(self.screen, Screen::Victory { .. }) {
            self.continue_endless();
        }
        // The song follows the clock: a boss theme, a song per ten minutes, the endless stages.
        if !empty && matches!(self.screen, Screen::Playing | Screen::Paused) {
            let field = self.world.as_ref().map(|w| w.biome as u8).unwrap_or(0);
            let rush = self.world.as_ref().is_some_and(|w| w.boss_rush);
            // A lord's song begins a little before the lord itself.
            let soon = self.world.as_ref().and_then(|w| w.lord_soon);
            let song = if rush {
                Music::Rush
            } else if boss {
                Music::Boss(field, boss_index)
            } else if let Some(k) = soon {
                Music::Boss(field, k as usize)
            } else {
                self.vigil_music()
            };
            self.audio.music(song);
        }
        if client && !empty && matches!(self.screen, Screen::Playing | Screen::Paused) {
            if dead {
                self.on_death();
            } else if won {
                self.on_victory();
            }
        }
    }

    fn cat_found(&mut self, slot: u8) {
        crate::log!("the Grave Cat chose slot {}", slot);
        let mine = self.world.as_ref().map(|w| !w.heroes.is_empty() && w.me().slot == slot).unwrap_or(false);
        if mine {
            self.save.cats += 1;
            self.save.store();
        }
    }

    pub(super) fn update_playing(&mut self, dt: f32) {
        drain_chars();
        // Alt-Tab pauses a single-player vigil; the self-test runs unfocused on purpose.
        if self.screen == Screen::Playing && !self.online() && self.auto.is_none() && self.save.settings.pause_unfocused && !window_focused() {
            self.screen = Screen::Paused;
        }
        self.show_tips();
        let paused = self.screen == Screen::Paused;
        if is_key_pressed(KeyCode::Escape) || act_pressed(Action::Pause) || pad_pressed(Pad::Pause) || (paused && pad_pressed(Pad::Back)) {
            self.screen = if paused { Screen::Playing } else { Screen::Paused };
            self.pause_sel = 0;
            return;
        }
        if paused && self.pause_menu() {
            return;
        }
        if paused && !self.online() {
            return;
        }
        if act_pressed(Action::Numbers) {
            self.save.settings.numbers = !self.save.settings.numbers;
            if let Some(w) = self.world.as_mut() {
                w.show_numbers = self.save.settings.numbers;
            }
        }
        let mut input = if paused {
            Input::default()
        } else {
            Input {
                dir: move_input(),
                roll: (pressed_any(&[key_for(Action::Roll), KeyCode::LeftShift, KeyCode::RightShift]) || pad_pressed(Pad::Roll)) && get_time() >= self.roll_blocked_until,
                skill: act_pressed(Action::Skill) || pad_pressed(Pad::Skill),
            }
        };
        if let Some(i) = self.auto.as_ref().map(|a| a.input()) {
            input = i;
        }
        self.step_game(dt, input, !paused);
    }

    /// The pause menu's buttons; returns true when one of them took the frame.
    fn pause_menu(&mut self) -> bool {
        let n = PAUSE_BUTTONS;
        if nav(&mut self.pause_sel, n, false) {
            self.audio.play("select", 0.4);
        }
        // Leaving asks twice; moving away from the button takes the question back.
        if self.pause_sel != 3 {
            self.leave_armed = false;
        }
        let under = hovered(&pause_rows());
        if let Some(i) = under.filter(|_| mouse_moved()) {
            if i != self.pause_sel {
                self.pause_sel = i;
                self.audio.play("select", 0.3);
            }
        }
        let click = clicked() && under.is_some();
        if click {
            self.pause_sel = under.unwrap_or(self.pause_sel);
        }
        if !confirm() && !click {
            return false;
        }
        self.audio.play("select", 0.6);
        match self.pause_sel {
            0 => self.screen = Screen::Playing,
            1 => {
                self.screen = Screen::Options;
                self.options_in_game = true;
                self.opt_sel = 0;
                // The difficulty page is not there during a vigil.
                self.opt_page = self.opt_page.min(3);
            }
            2 => self.audio.toggle_mute(),
            _ if !self.leave_armed => self.leave_armed = true,
            _ => {
                self.leave_armed = false;
                self.abandon();
            }
        }
        true
    }

    /// The pause menu over the vigil.
    pub(super) fn draw_pause(&self, ui: &Ui) {
        let (sw, sh) = (ui_w(), ui_h());
        // Dark enough that banners and numbers of the vigil behind do not compete.
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.01, 0.01, 0.02, 0.82));
        let online = self.online();
        ui.spaced_c(if online { "MENU" } else { "PAUSED" }, sw * 0.5, sh * 0.3, 40.0, 6.0, PALE);
        if online {
            let held = self.world.as_ref().is_some_and(|w| w.pause == Pause::Held);
            let line = if held { "Everyone has paused: the world waits for you." } else { "The world does not wait for you online, unless every player pauses." };
            ui.text_c(line, sw * 0.5, sh * 0.3 + 34.0, 18.0, crate::ui::EMBER);
        }
        let leave = match (self.leave_armed, online) {
            (true, _) => "Really? Click again to leave",
            (false, true) => "Leave the vigil",
            (false, false) => "Give up the vigil",
        };
        let sound = if self.audio.muted { "Sound on" } else { "Sound off" };
        for (i, (label, r)) in ["Resume", "Options", sound, leave].iter().zip(pause_rows()).enumerate() {
            let on = self.pause_sel == i;
            ui.row(r.x, r.y, r.w, r.h, on);
            let col = if i == 3 {
                crate::ui::BLOOD
            } else if on {
                crate::ui::GOLD
            } else {
                PALE
            };
            ui.text_c(label, sw * 0.5, r.y + 26.0, 22.0, col);
        }
        ui.text_c(hint("W/S or mouse to choose, Enter or click to confirm, Esc to resume", "Stick to choose, A to confirm, Start or B to resume"), sw * 0.5, sh - 30.0, 16.0, DIM);
        self.draw_pause_loadout(ui);
    }

    /// The vigil so far beside the pause menu: time, level, damage per second, and every
    /// weapon and passive large enough to read, each explained under the mouse.
    fn draw_pause_loadout(&self, ui: &Ui) {
        let Some(w) = self.world.as_ref() else { return };
        if w.heroes.is_empty() {
            return;
        }
        let (sw, sh) = (ui_w(), ui_h());
        let me = w.me();
        let x = 40.0;
        let mut y = sh * 0.3 - 30.0;
        let pw = (sw * 0.5 - 250.0).clamp(240.0, 520.0);
        let secs = w.time as u32;
        // Damage per second over the last minute (or the vigil so far, if shorter).
        let dps = me.dmg_minute / w.time.clamp(1.0, 60.0);
        let lines = [
            ("Vigil", format!("{:02}:{:02}", secs / 60, secs % 60)),
            ("Level", w.level.to_string()),
            ("Damage per second", crate::ui::short(dps as f64)),
            ("Damage this vigil", crate::ui::short(me.dmg_total)),
            ("Felled", w.kills.to_string()),
        ];
        draw_rectangle(x - 16.0, y - 30.0, pw + 32.0, sh * 0.62, Color::new(0.0, 0.0, 0.0, 0.35));
        for (label, value) in lines {
            ui.text(label, x, y, 19.0, DIM);
            ui.text_r(&value, x + pw, y, 21.0, PALE);
            y += 28.0;
        }
        y += 12.0;
        ui.text("Weapons and passives", x, y, 17.0, crate::ui::GOLD);
        let (tips, _) = ui.loadout(w, me, x, y + 14.0, 1.4, pw);
        ui.text("Rest the mouse on one to read what it does.", x, sh * 0.3 - 74.0 + sh * 0.62, 14.0, DIM);
        ui.tooltip(&tips);
    }

    /// Leaves the current vigil from the pause menu.
    fn abandon(&mut self) {
        if self.online() {
            // Leaving an online vigil just drops us; the others play on.
            self.leave_online();
            self.world = None;
            self.screen = Screen::Menu;
            self.audio.music(self.menu_music());
            return;
        }
        if let Some(w) = self.world.as_mut() {
            w.dead = true;
            w.sfx("death", 1.0);
        }
        self.handle_events();
        self.on_death();
    }

    /// Level-up card selection for our own hero.
    fn level_up_input(&mut self) {
        let Some(w) = self.world.as_mut() else { return };
        if w.pause != Pause::Choosing || w.heroes.is_empty() || w.me().picked || self.pick_pending.map(|p| p.0) == Some(w.level_round) {
            return;
        }
        let n = w.me().choices.len().max(1);
        if self.level_sel >= n {
            self.level_sel = 0;
        }
        if nav(&mut self.level_sel, n, false) {
            self.audio.play("select", 0.4);
        }
        let under = hovered(&crate::ui::level_up_cards(n));
        if let Some(i) = under.filter(|_| mouse_moved()) {
            if i != self.level_sel {
                self.level_sel = i;
                self.audio.play("select", 0.3);
            }
        }
        if self.level_round_seen != w.level_round {
            self.level_round_seen = w.level_round;
            self.level_shown_at = get_time();
        }
        // A short grace period so a key meant for the fight doesn't pick a card.
        if get_time() - self.level_shown_at < 0.35 {
            return;
        }
        let mut pick = [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4].iter().take(n).position(|k| is_key_pressed(*k));
        // Enter or Space takes the highlighted card; the short grace period above keeps a
        // roll pressed in the fight from picking one by accident.
        if pressed_any(&[KeyCode::Enter, KeyCode::KpEnter, KeyCode::Space]) || pad_pressed(Pad::Confirm) {
            pick = Some(self.level_sel);
        }
        if clicked() {
            if let Some(i) = under {
                self.level_sel = i;
                pick = Some(i);
            }
        }
        if let Some(auto) = &self.auto {
            if auto.should_pick() {
                pick = Some(0);
            }
        }
        let reroll = (act_pressed(Action::Reroll) || pad_pressed(Pad::Reroll)) && w.me().rerolls > 0;
        if let Some(i) = pick {
            if let Some(c) = w.me().choices.get(i) {
                crate::log!("level {}: picked {}", w.level, w.me().choice_text(*c).0);
            }
            // Space (or a pad button) both takes a card and rolls; taking one must not roll.
            self.roll_blocked_until = get_time() + 0.4;
        }
        if let NetMode::Client(_) = self.net {
            if reroll {
                self.reroll_seq = self.reroll_seq.wrapping_add(1);
            }
            if let Some(i) = pick {
                self.pick_pending = Some((w.level_round, i as u8));
                self.audio.play("select", 0.6);
            }
        } else {
            let l = w.local;
            if reroll {
                w.reroll(l);
            }
            if let Some(i) = pick {
                w.pick(l, w.level_round, i);
            }
        }
        if pick.is_some() {
            self.level_sel = 0;
        }
    }

    /// One frame of the running game for whichever network role we have.
    pub(super) fn step_game(&mut self, dt: f32, mut input: Input, level_keys: bool) {
        // The level-up cards are read after the fight's input, so a Space that just took a
        // card (or one pressed while the cards are up) must not reach the hero as a roll.
        let choosing = self.world.as_ref().is_some_and(|w| w.pause == Pause::Choosing);
        if level_keys {
            self.level_up_input();
        }
        let now = get_time();
        if choosing || now < self.roll_blocked_until {
            input.roll = false;
        }
        let my_class = class_to_u8(self.class());
        let name = self.player_name();
        let in_menu = self.screen == Screen::Paused || (self.screen == Screen::Options && self.options_in_game);
        let ping = !choosing && !in_menu && (act_pressed(Action::Ping) || is_mouse_button_pressed(MouseButton::Middle) || pad_pressed(Pad::Ping));
        let Some(w) = self.world.as_mut() else { return };
        w.screen = crate::draw::view_size();
        if !w.heroes.is_empty() {
            let l = w.local;
            w.heroes[l].input = input;
            w.heroes[l].menu_open = in_menu;
        }
        // A ping goes where the mouse points, or a little ahead of us on a controller.
        if ping && !w.heroes.is_empty() && !w.spectating {
            let me = w.me();
            let at = if using_pad() { me.p.pos + me.p.facing * 160.0 } else { w.screen_to_world(mouse_ui()) };
            if w.is_client {
                self.ping_seq = self.ping_seq.wrapping_add(1);
                self.ping_at = at;
            } else {
                let slot = me.slot;
                w.add_ping(slot, at);
            }
        }
        match &mut self.net {
            NetMode::Offline => w.update(dt),
            NetMode::Host(host) => {
                for ev in host.poll(now) {
                    match ev {
                        HostEvent::Left(name, slot) => {
                            crate::log!("{} (slot {}) left the vigil", name, slot);
                            if let Some(h) = w.heroes.iter().position(|h| h.remote && h.connected && h.slot == slot) {
                                w.disconnect(h);
                                w.toast(&format!("{} has left the vigil. They can rejoin from the lobby.", name));
                            } else {
                                w.toast(&format!("{} stopped watching.", name));
                            }
                        }
                        HostEvent::Spectating(name) => {
                            crate::log!("{} joined the running vigil to watch", name);
                            w.toast(&format!("{} is watching, and joins the next vigil.", name));
                        }
                        HostEvent::Rejoined(name, slot) => {
                            crate::log!("{} (slot {}) rejoined the vigil", name, slot);
                            if w.reconnect(slot) {
                                w.toast(&format!("{} has returned to the vigil.", name));
                            }
                        }
                        HostEvent::Log(name, reason, text) => super::lobby::store_player_log(self.auto.as_ref(), &name, &reason, &text),
                        HostEvent::Joined(_) => {}
                    }
                }
                for p in &host.peers {
                    let Some(inp) = p.input else { continue };
                    let Some(h) = w.heroes.iter().position(|h| h.slot == p.slot) else { continue };
                    apply_client_input(w, h, &inp);
                }
                let players = host.lobby_players(&name, my_class);
                host.tick_lobby(dt, now, &players, &w.diff, w.boss_rush, self.save.challenges, w.biome as u8, w.curses);
                w.update(dt);
                if host.snapshot_due(dt, w.pause != Pause::Running) {
                    host.send_snapshots(w);
                }
            }
            NetMode::Client(client) => {
                client.tick(dt, now, &name, my_class, &self.save.profile);
                self.since_snap += dt;
                for ev in client.poll(now) {
                    match ev {
                        ClientEvent::Snapshot(data) => {
                            let slot = client.slot.unwrap_or(1);
                            if self.snap_dec.decode(w, &data, slot, self.since_snap) {
                                self.snapshots += 1;
                                // Our own movement runs here, so our hero needs our upgrades
                                // (cheaper rolls); snapshots only carry what others need to see.
                                if let Some(me) = w.heroes.iter_mut().find(|h| h.slot == slot) {
                                    me.profile = self.save.profile;
                                }
                                if w.spectating {
                                    w.local = w.heroes.iter().position(|h| h.slot == self.watch_slot).unwrap_or(0);
                                }
                            }
                            self.since_snap = 0.0;
                        }
                        // The host went back to the lobby; follow it from the end screen.
                        ClientEvent::Lobby { started: false, players, diff, rush, challenges, biome, curses } if w.dead || w.won => {
                            self.lobby.curses = curses;
                            self.lobby.rush = rush;
                            self.lobby.challenges = challenges;
                            self.lobby.biome = biome;
                            self.lobby.players = players;
                            self.lobby.diff = diff;
                            self.follow_host = true;
                        }
                        ClientEvent::HostLeft => {
                            crate::log!("host left or timed out during the vigil");
                            self.note = Some(("The host has left. The vigil is over.".into(), 5.0));
                            w.dead = true;
                        }
                        _ => {}
                    }
                }
                if w.heroes.is_empty() {
                    return;
                }
                if w.spectating {
                    // Left and right pick whom to follow.
                    let step = right(false) as i32 - left(false) as i32;
                    if step != 0 {
                        let n = w.heroes.len() as i32;
                        w.local = (w.local as i32 + step).rem_euclid(n) as usize;
                    }
                    self.watch_slot = w.heroes[w.local.min(w.heroes.len() - 1)].slot;
                }
                if self.pick_pending.map(|p| p.0 != w.level_round || w.pause != Pause::Choosing).unwrap_or(false) {
                    self.pick_pending = None;
                }
                // A client asks the host for its skill; the host casts it.
                if w.heroes.get(w.local).is_some_and(|h| h.input.skill) {
                    self.skill_seq = self.skill_seq.wrapping_add(1);
                }
                if w.client_update(dt) {
                    let bucket = w.me().class == Class::BucketKnight;
                    self.audio.play(if bucket { "clang" } else { "roll" }, if bucket { 0.4 } else { 0.5 });
                }
                self.input_acc += dt;
                if self.input_acc >= 1.0 / 60.0 {
                    // Keep the remainder so 60 Hz holds at any frame rate.
                    self.input_acc = (self.input_acc - 1.0 / 60.0).min(1.0 / 60.0);
                    let me = w.me();
                    let (pick_round, pick_idx) = self.pick_pending.unwrap_or((0, 255));
                    client.send_input(&ClientInput {
                        // A fallen hero's body lies still; what moves is its ghost.
                        pos: if me.downed { (me.ghost.x, me.ghost.y) } else { (me.p.pos.x, me.p.pos.y) },
                        facing: (me.p.facing.x, me.p.facing.y),
                        moving: me.p.moving,
                        roll_t: me.p.roll_t,
                        roll_dir: (me.p.roll_dir.x, me.p.roll_dir.y),
                        stamina: me.p.stamina,
                        pick_round,
                        pick_idx,
                        reroll_seq: self.reroll_seq,
                        rolls: me.rolls,
                        skill_seq: self.skill_seq,
                        ack: self.snap_dec.ack,
                        view: (w.screen.x as u16, w.screen.y as u16),
                        menu: in_menu,
                        ping_seq: self.ping_seq,
                        ping_at: (self.ping_at.x, self.ping_at.y),
                    });
                }
            }
        }
        self.handle_events();
        if std::mem::take(&mut self.follow_host) && matches!(self.screen, Screen::Dead { .. } | Screen::Victory { .. }) {
            self.return_to_lobby();
            return;
        }
        if matches!(self.screen, Screen::Playing | Screen::Paused) {
            self.track_run();
        }
    }
}

/// Buttons in the pause menu: resume, options, sound, leave.
const PAUSE_BUTTONS: usize = 4;

/// Where the pause menu's buttons sit, for drawing and for the mouse.
fn pause_rows() -> Vec<Rect> {
    let (sw, sh) = (ui_w(), ui_h());
    (0..PAUSE_BUTTONS).map(|i| Rect::new(sw * 0.5 - 170.0, sh * 0.3 + 70.0 + i as f32 * 48.0, 340.0, 40.0)).collect()
}

/// Copies the settings a world keeps for itself.
fn apply_settings(w: &mut World, st: &crate::save::Settings) {
    w.show_numbers = st.numbers;
    w.gore = st.gore;
    w.detail_high = st.detail_high;
    w.particle_mult = [0.35, 0.65, 1.0][st.particles.min(2) as usize];
    w.fog = st.fog;
    w.shake_mult = st.shake as f32 / 100.0;
    let (r, g, b) = crate::save::COLOR_MODES[st.color_mode as usize % crate::save::COLOR_MODES.len()].1;
    w.bullet_color = Color::from_rgba(r, g, b, 255);
}

/// The host takes a client's own movement as truth and forwards its choices.
fn apply_client_input(w: &mut World, h: usize, inp: &ClientInput) {
    let hero = &mut w.heroes[h];
    hero.menu_open = inp.menu;
    if hero.downed {
        hero.ghost = vec2(inp.pos.0, inp.pos.1);
    } else {
        hero.p.pos = vec2(inp.pos.0, inp.pos.1);
        let f = vec2(inp.facing.0, inp.facing.1);
        if f.length_squared() > 0.0 {
            hero.p.facing = f.normalize();
        }
        hero.p.moving = inp.moving;
        hero.p.roll_t = inp.roll_t;
        hero.p.roll_dir = vec2(inp.roll_dir.0, inp.roll_dir.1);
        hero.p.stamina = inp.stamina;
        hero.rolls = inp.rolls;
    }
    if inp.ping_seq != hero.ping_seq {
        hero.ping_seq = inp.ping_seq;
        let slot = hero.slot;
        w.add_ping(slot, vec2(inp.ping_at.0, inp.ping_at.1));
    }
    let hero = &mut w.heroes[h];
    let cast = inp.skill_seq != hero.skill_seq;
    hero.skill_seq = inp.skill_seq;
    if cast {
        w.use_skill(h);
    }
    let hero = &mut w.heroes[h];
    if inp.reroll_seq != hero.reroll_seq {
        hero.reroll_seq = inp.reroll_seq;
        w.reroll(h);
    }
    if inp.pick_idx != 255 {
        w.pick(h, inp.pick_round, inp.pick_idx as usize);
    }
}

/// Ashen Marks a vigil has earned before the challenge bonus. A boss rush meets its lords
/// in quick succession, so each counts for half.
fn run_marks(w: &World, won: bool) -> u64 {
    let lords = if w.boss_rush { w.bosses_killed / 2 } else { w.bosses_killed };
    marks_for_run(w.time / 60.0, lords, w.level, w.kills, won, &w.diff)
}

fn first_vigil_tips() -> Vec<(f32, String)> {
    let pad = using_pad();
    let k = |a: Action| key_label(a).to_string();
    let roll = if pad { "A".to_string() } else { k(Action::Roll) };
    vec![
        (1.0, format!("Your weapons strike on their own and aim at the nearest foe. Move to stay alive, {} rolls through danger.", roll)),
        (12.0, "Fallen foes leave souls: experience now, and currency at the Ash Shrine later.".into()),
        (40.0, "Souls left on the ground merge into bigger ones, so none are ever lost.".into()),
        (100.0, "Two weapons at level 10 that belong together fuse into one.".into()),
        (270.0, "A boss comes every five minutes. Its attacks always show where they will land.".into()),
    ]
}

/// True while the game window has the keyboard focus.
#[cfg(windows)]
pub(super) fn window_focused() -> bool {
    #[link(name = "user32")]
    extern "system" {
        fn GetForegroundWindow() -> isize;
        fn GetWindowThreadProcessId(hwnd: isize, pid: *mut u32) -> u32;
    }
    let mut pid = 0u32;
    // SAFETY: plain Win32 queries; `pid` is a valid out pointer for the duration of the call.
    unsafe {
        let hwnd = GetForegroundWindow();
        if hwnd == 0 {
            return true;
        }
        GetWindowThreadProcessId(hwnd, &mut pid);
    }
    pid == std::process::id()
}

#[cfg(not(windows))]
pub(super) fn window_focused() -> bool {
    true
}
