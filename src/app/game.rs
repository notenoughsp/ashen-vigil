//! Starting, running and ending a vigil for each network role.

use super::*;
use crate::net::{ClientEvent, ClientInput, HostEvent};

impl App {
    fn reset_run_state(&mut self) {
        self.snap_dec = Default::default();
        self.since_snap = 0.0;
        self.pick_pending = None;
        self.reroll_seq = 0;
        self.synced_banked = 0;
        self.level_sel = 0;
    }

    fn new_world(&self, diff: Difficulty, remnant: Option<(Vec2, u64)>) -> World {
        let mut w = World::new(diff, remnant);
        w.screen = crate::draw::view_size();
        let st = &self.save.settings;
        w.show_numbers = st.numbers;
        w.gore = st.gore;
        w.detail_high = st.detail_high;
        w.particle_mult = [0.35, 0.65, 1.0][st.particles.min(2) as usize];
        w.fog = st.fog;
        w.shake_mult = if st.shake { 1.0 } else { 0.0 };
        w
    }

    fn begin(&mut self, w: World) {
        self.reset_run_log();
        self.endless_paid = 0;
        self.endless_record = None;
        self.tips = if self.save.vigils == 0 { first_vigil_tips() } else { Vec::new() };
        let mode = match &self.net {
            NetMode::Host(_) => "hosted",
            NetMode::Client(_) => "joined",
            _ => "single player",
        };
        let names: Vec<String> = w.heroes.iter().map(|h| format!("{} ({})", h.name, h.class.name())).collect();
        // A client learns the heroes from the first snapshot; its own pick is what it knows.
        let heroes = if names.is_empty() { format!("me as {}", self.class().name()) } else { names.join(", ") };
        crate::log!("vigil started, {}, difficulty {}, heroes: {}, challenges {:03b}", mode, w.diff.preset_name(), heroes, self.save.challenges);
        self.world = Some(w);
        self.screen = Screen::Playing;
        self.audio.music(Music::Ambient);
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
        w.grant_head_start();
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
        self.audio.music(Music::Ambient);
    }

    /// Adds newly banked souls to the save file as soon as they are banked.
    fn sync_bank(&mut self) {
        let Some(w) = &self.world else { return };
        if w.heroes.is_empty() {
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
        let total = if w.time < 60.0 && !won { 0 } else { marks_for_run(w.time / 60.0, w.bosses_killed, w.level, w.kills, won, &w.diff) };
        let base = total.saturating_sub(self.endless_paid);
        // Challenges only exist offline and pay extra.
        let mult = if self.online() { 1.0 } else { challenge_mult(self.save.challenges) };
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
        w.next_boss = w.time + 60.0;
        w.banner("THE NIGHT GOES ON", "Every five minutes another lord of ash comes.", Color::from_rgba(200, 120, 90, 255), 4.0);
        // In the same units as `award_marks` before the challenge bonus.
        self.endless_paid = marks_for_run(w.time / 60.0, w.bosses_killed, w.level, w.kills, true, &w.diff);
        self.endless_record = self.save.runs.iter().find(|r| r.won && (r.time - w.time).abs() < 0.5 && r.kills == w.kills).cloned();
        self.audio.music(Music::Ambient);
        self.screen = Screen::Playing;
    }

    /// One beginner tip at a time, at its moment in the first vigil.
    fn show_tips(&mut self) {
        let Some(w) = &self.world else { return };
        if self.tips.first().is_some_and(|(at, _)| w.time >= *at) && self.note.is_none() {
            let (_, text) = self.tips.remove(0);
            self.set_note(&text, 6.0);
        }
    }

    fn on_death(&mut self) {
        crate::log!("vigil lost: {}", self.log_status());
        if matches!(self.net, NetMode::Client(_)) {
            crate::net::upload_log("vigil");
        }
        let online = self.online();
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

    fn on_victory(&mut self) {
        crate::log!("vigil kept until dawn: {}", self.log_status());
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
                    self.audio.music(Music::Boss(boss_index));
                }
                Event::BossDefeated => {
                    let fight = self.world.as_ref().map(|w| w.time - w.boss_started).unwrap_or(f32::MAX);
                    crate::log!("boss defeated after {:.1}s", fight);
                    if fight <= 10.0 {
                        self.unlock(Achievement::Executioner);
                    }
                    self.audio.music(Music::Ambient);
                }
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
        if client && !empty && matches!(self.screen, Screen::Playing | Screen::Paused) {
            self.audio.music(if boss { Music::Boss(boss_index) } else { Music::Ambient });
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
        if self.screen == Screen::Playing && !self.online() && self.auto.is_none() && !window_focused() {
            self.screen = Screen::Paused;
        }
        self.show_tips();
        let paused = self.screen == Screen::Paused;
        if is_key_pressed(KeyCode::Escape) || act_pressed(Action::Pause) || pad_pressed(Pad::Pause) || (paused && (pad_pressed(Pad::Back) || clicked())) {
            self.screen = if paused { Screen::Playing } else { Screen::Paused };
            return;
        }
        if paused && (is_key_pressed(KeyCode::Q) || pad_pressed(Pad::Leave)) {
            self.abandon();
            return;
        }
        if paused && !self.online() {
            return;
        }
        if act_pressed(Action::AutoAim) || pad_pressed(Pad::AutoAim) {
            self.save.settings.auto_aim = !self.save.settings.auto_aim;
            let on = self.save.settings.auto_aim;
            self.set_note(if on { "Auto-aim on: weapons seek the nearest foe." } else { "Auto-aim off: weapons follow your steps." }, 2.0);
            self.audio.play("select", 0.6);
            self.save.store();
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
            Input { dir: move_input(), roll: pressed_any(&[key_for(Action::Roll), KeyCode::LeftShift, KeyCode::RightShift]) || pad_pressed(Pad::Roll) }
        };
        if let Some(i) = self.auto.as_ref().map(|a| a.input()) {
            input = i;
        }
        self.step_game(dt, input, !paused);
    }

    /// Leaves the current vigil from the pause menu.
    fn abandon(&mut self) {
        if self.online() {
            // Leaving an online vigil just drops us; the others play on.
            self.leave_online();
            self.world = None;
            self.screen = Screen::Menu;
            self.audio.music(Music::Ambient);
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
        if pressed_any(&[KeyCode::Enter, KeyCode::KpEnter]) || pad_pressed(Pad::Confirm) {
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
    pub(super) fn step_game(&mut self, dt: f32, input: Input, level_keys: bool) {
        if level_keys {
            self.level_up_input();
        }
        let now = get_time();
        let my_class = class_to_u8(self.class());
        let name = self.player_name();
        let Some(w) = self.world.as_mut() else { return };
        w.screen = crate::draw::view_size();
        if !w.heroes.is_empty() {
            let l = w.local;
            w.heroes[l].input = input;
            w.heroes[l].auto_aim = self.save.settings.auto_aim;
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
                            }
                            w.toast(&format!("{} has left the vigil. They can rejoin from the lobby.", name));
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
                host.tick_lobby(dt, now, &players, &w.diff);
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
                            }
                            self.since_snap = 0.0;
                        }
                        // The host went back to the lobby; follow it from the end screen.
                        ClientEvent::Lobby { started: false, players, diff } if w.dead || w.won => {
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
                if self.pick_pending.map(|p| p.0 != w.level_round || w.pause != Pause::Choosing).unwrap_or(false) {
                    self.pick_pending = None;
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
                        pos: (me.p.pos.x, me.p.pos.y),
                        facing: (me.p.facing.x, me.p.facing.y),
                        moving: me.p.moving,
                        roll_t: me.p.roll_t,
                        roll_dir: (me.p.roll_dir.x, me.p.roll_dir.y),
                        stamina: me.p.stamina,
                        pick_round,
                        pick_idx,
                        reroll_seq: self.reroll_seq,
                        rolls: me.rolls,
                        auto_aim: me.auto_aim,
                        ack: self.snap_dec.ack,
                        view: (w.screen.x as u16, w.screen.y as u16),
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

/// The host takes a client's own movement as truth and forwards its choices.
fn apply_client_input(w: &mut World, h: usize, inp: &ClientInput) {
    let hero = &mut w.heroes[h];
    if !hero.downed {
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
        hero.auto_aim = inp.auto_aim;
    }
    if inp.reroll_seq != hero.reroll_seq {
        hero.reroll_seq = inp.reroll_seq;
        w.reroll(h);
    }
    if inp.pick_idx != 255 {
        w.pick(h, inp.pick_round, inp.pick_idx as usize);
    }
}

fn first_vigil_tips() -> Vec<(f32, String)> {
    let pad = using_pad();
    let k = |a: Action| key_label(a).to_string();
    let roll = if pad { "A".to_string() } else { k(Action::Roll) };
    let aim = if pad { "X".to_string() } else { k(Action::AutoAim) };
    vec![
        (1.0, format!("Your weapons strike on their own. Move to stay alive, {} rolls through danger.", roll)),
        (12.0, "Fallen foes leave souls: experience now, and currency at the Ash Shrine later.".into()),
        (40.0, format!("{} switches auto-aim: weapons turn to the nearest foe instead of where you walk.", aim)),
        (100.0, "Two weapons at level 10 that belong together fuse into one.".into()),
        (270.0, "A boss comes every five minutes. Its attacks always show where they will land.".into()),
    ]
}

/// True while the game window has the keyboard focus.
#[cfg(windows)]
fn window_focused() -> bool {
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
fn window_focused() -> bool {
    true
}
