//! Achievements, the leaderboard, personal bests and the summary after a vigil.

use super::*;
use crate::ui::{a, mono, mono_width, short, ui_h, ui_w, BLOOD, EMBER, GOLD, TEAL};

/// What happened during the vigil, sampled for the summary graphs.
#[derive(Default)]
pub struct RunLog {
    /// (time, kills, our damage) every `SAMPLE` seconds.
    pub samples: Vec<(f32, u32, f64)>,
    next: f32,
}

const SAMPLE: f32 = 15.0;
/// Achievements on one page of the records screen: three columns of ten.
const ACH_PER_PAGE: usize = 30;

/// The leaderboard plus as many achievement pages as needed.
fn record_pages() -> usize {
    1 + ALL_ACHIEVEMENTS.len().div_ceil(ACH_PER_PAGE)
}

/// The verdict on a finished vigil.
#[derive(Default)]
pub struct EndSummary {
    pub won: bool,
    pub marks: u64,
    pub challenge_bonus: bool,
    pub rank: Option<usize>,
    pub new_bests: Vec<&'static str>,
    pub achievements: Vec<Achievement>,
}

impl App {
    pub(super) fn reset_run_log(&mut self) {
        self.run_log = RunLog::default();
        self.run_achievements.clear();
    }

    /// Earns an achievement once, pays its marks and shows a popup.
    pub(super) fn unlock(&mut self, ach: Achievement) {
        if self.save.achievements & ach.bit() != 0 || self.save.readonly && self.auto.is_none() {
            return;
        }
        self.save.achievements |= ach.bit();
        self.save.marks += ach.reward();
        self.save.store();
        self.run_achievements.push(ach);
        self.popups.push((format!("Achievement: {}  (+{} marks)", ach.name(), ach.reward()), 5.0));
        self.audio.play("evolve", 0.6);
    }

    /// Samples the run for the summary and checks achievements that can happen mid-vigil.
    pub(super) fn track_run(&mut self) {
        let Some(w) = &self.world else { return };
        if w.heroes.is_empty() {
            return;
        }
        let me = w.me();
        if w.time >= self.run_log.next {
            self.run_log.next = w.time + SAMPLE;
            self.run_log.samples.push((w.time, w.kills, me.dmg_total));
        }
        let mut earned = Vec::new();
        let mut check = |cond: bool, a: Achievement| {
            if cond {
                earned.push(a);
            }
        };
        check(w.time >= 300.0, Achievement::FirstLight);
        check(w.bosses_killed >= 1, Achievement::Gravebreaker);
        check(w.kills >= 10_000, Achievement::Legion);
        check(me.weapons.iter().any(|wp| wp.is_fused()), Achievement::Fusionist);
        check(me.weapons.iter().any(|wp| wp.level >= WEAPON_MAX_LEVEL || wp.is_fused()), Achievement::Evolved);
        check(me.rolls == 0 && w.time >= 600.0 && !me.downed, Achievement::Steadfast);
        check(me.has_cat, Achievement::CatPerson);
        check(me.purchases >= 3, Achievement::Patron);
        check(w.moons_survived >= 1, Achievement::MoonWatcher);
        check(w.trials_won >= 1, Achievement::TrialEndured);
        check(w.heroes.len() >= 2 && w.time >= 600.0, Achievement::Fellowship);
        check(me.weapons.iter().any(|wp| wp.tier() >= 2), Achievement::GrandFusion);
        check(me.weapons.iter().any(|wp| wp.tier() >= 3), Achievement::Singularity);
        // The past-dawn achievements belong to the vigil; a boss rush has no dawn.
        let past_dawn = if w.endless && !w.boss_rush { w.time - VICTORY_TIME } else { 0.0 };
        check(past_dawn >= 600.0, Achievement::LongNight);
        check(w.level >= 100, Achievement::Centurion);
        check(w.kills >= 100_000, Achievement::AshAndBone);
        check(w.time >= 180.0 && me.first_hurt > 180.0, Achievement::Untouched);
        check(w.grave_found, Achievement::LastKeeper);
        check(me.rested, Achievement::Fireside);
        check(me.p.souls >= 100_000, Achievement::Hoarder);
        check(w.heroes.len() >= 4 && w.time >= 60.0, Achievement::FullHouse);
        check(me.rolls >= 1000, Achievement::RollModel);
        check(me.banked >= 50_000, Achievement::Banker);
        check(w.time >= 900.0, Achievement::HalfNight);
        check(w.time >= 1200.0, Achievement::DeepNight);
        check(w.level >= 50, Achievement::Ascendant);
        check(w.level >= 200, Achievement::BeyondMeasure);
        check(w.kills >= 1_000, Achievement::ThinTheHerd);
        check(w.kills >= 50_000, Achievement::SeaOfAsh);
        check(w.kills >= 250_000, Achievement::Extinction);
        // Lords come one after another in a boss rush; these count in the vigil only.
        check(w.bosses_killed >= 3 && !w.boss_rush, Achievement::Lordbane);
        check(w.bosses_killed >= 10 && !w.boss_rush, Achievement::Kingslayer);
        check(me.weapons.iter().any(|wp| wp.whole), Achievement::LastFlame);
        check(me.weapons.len() >= 6, Achievement::FullArsenal);
        check(me.items.len() >= 6, Achievement::Packrat);
        check(me.items.iter().any(|it| it.level >= it.kind.max_level() && it.kind.max_level() >= 5), Achievement::Polished);
        check(past_dawn >= 1800.0, Achievement::Eternal);
        check(past_dawn >= 3600.0, Achievement::Nightless);
        check(me.dmg_total >= 1.0e6, Achievement::MillionCuts);
        check(w.braziers.iter().filter(|b| b.burn > 0.0).count() >= 10, Achievement::Lamplighter);
        check(me.dmg_total >= 1.0e9, Achievement::Unmaking);
        check(w.time >= 300.0 && me.first_hurt > 300.0, Achievement::Ghost);
        for a in earned {
            self.unlock(a);
        }
        self.track_secrets();
    }

    /// Little things the vigil notices: ten rolls in four seconds, and a certain minute.
    fn track_secrets(&mut self) {
        let Some(w) = self.world.as_mut() else { return };
        let me = w.me();
        let (rolls, time) = (me.rolls, w.time);
        if rolls > self.last_rolls {
            for _ in self.last_rolls..rolls {
                self.roll_times.push(time);
            }
        }
        self.last_rolls = rolls;
        self.roll_times.retain(|t| time - *t <= 4.0);
        if self.roll_times.len() >= 10 {
            self.roll_times.clear();
            w.toast("The world spins. Maybe sit down for a second.");
            self.unlock(Achievement::Dizzy);
        }
        let Some(w) = self.world.as_mut() else { return };
        // 4:04 into a vigil.
        if !self.said_404 && (244.0..245.0).contains(&w.time) {
            self.said_404 = true;
            w.toast("404: mercy not found.");
        }
    }

    /// Leaderboard, bests and end-of-vigil achievements.
    pub(super) fn finish_run(&mut self, won: bool, marks: u64) {
        let Some(w) = &self.world else { return };
        if w.heroes.is_empty() {
            return;
        }
        let me = w.me();
        // The last stretch counts too, however short.
        if self.run_log.samples.last().is_some_and(|s| s.0 < w.time) {
            self.run_log.samples.push((w.time, w.kills, me.dmg_total));
        }
        let record = RunRecord {
            time: w.time,
            kills: w.kills,
            level: w.level,
            players: w.heroes.len() as u32,
            won: won || w.endless,
            class: me.class,
            difficulty: if w.boss_rush { format!("{} Rush", w.diff.preset_name()) } else { w.diff.preset_name().to_string() },
            damage: me.dmg_total,
        };
        let abyss = PRESETS[3].1;
        let hard_enough = w.diff.count >= abyss.count && w.diff.hp >= abyss.hp && w.diff.dmg >= abyss.dmg && w.diff.speed >= abyss.speed;
        // The vigil's own challenges: ours alone, or the host's online.
        let challenges = me.challenges;
        let mut new_bests = Vec::new();
        if w.kills > self.save.best_kills {
            self.save.best_kills = w.kills;
            new_bests.push("kills");
        }
        if w.level > self.save.best_level {
            self.save.best_level = w.level;
            new_bests.push("level");
        }
        if me.dmg_total > self.save.best_damage {
            self.save.best_damage = me.dmg_total;
            new_bests.push("damage");
        }
        // An endless vigil replaces the entry it got at dawn.
        if let Some(old) = self.endless_record.take() {
            if let Some(i) = self.save.runs.iter().position(|r| *r == old) {
                self.save.runs.remove(i);
            }
        }
        let rank = if w.time >= 60.0 { self.save.record_run(record) } else { None };
        let preset = w.diff.preset_name();
        let class = me.class;
        let cute = w.cute;
        let rush = w.boss_rush;
        let cursed = w.curses.count_ones();
        if won && rush {
            self.unlock(Achievement::Gauntlet);
        }
        if won && !rush && cursed >= 3 {
            self.unlock(Achievement::Accursed);
        }
        // The dawn achievements belong to the vigil, not to the boss rush.
        let won = won && !rush;
        if won {
            self.unlock(Achievement::Dawnbringer);
            if hard_enough {
                self.unlock(Achievement::AbyssWalker);
            }
            let earned = [
                (challenges & Challenge::OneWeapon.bit() != 0, Achievement::Purist),
                (challenges & Challenge::GlassCannon.bit() != 0, Achievement::ShatteredGlass),
                (challenges & Challenge::NoRolls.bit() != 0, Achievement::DeepRoots),
                (challenges.count_ones() >= 3, Achievement::Martyr),
                (preset == "Pilgrim", Achievement::GentleDawn),
                (preset == "Knight", Achievement::KnightsDawn),
                (preset == "Lord", Achievement::LordsDawn),
                (cute, Achievement::HoldHands),
                (class == Class::BucketKnight, Achievement::TinCan),
            ];
            for (cond, a) in earned {
                if cond {
                    self.unlock(a);
                }
            }
        }
        // Wins from older saves only live on in the leaderboard.
        for r in self.save.runs.iter().filter(|r| r.won && !r.difficulty.ends_with("Rush")) {
            self.save.won_classes |= 1 << class_to_u8(r.class);
        }
        if won {
            self.save.won_classes |= 1 << class_to_u8(class);
        }
        if self.save.won_classes.count_ones() >= 4 {
            self.unlock(Achievement::ManyFaces);
        }
        if self.save.vigils >= 25 {
            self.unlock(Achievement::OldRegular);
        }
        if self.save.cats >= 10 {
            self.unlock(Achievement::CatHerder);
        }
        self.save.store();
        self.summary = EndSummary { won, marks, challenge_bonus: challenges != 0, rank, new_bests, achievements: self.run_achievements.clone() };
    }

    pub(super) fn draw_popups(&mut self, ui: &Ui, dt: f32) {
        let (sw, sh) = (ui_w(), ui_h());
        for p in self.popups.iter_mut() {
            p.1 -= dt;
        }
        self.popups.retain(|p| p.1 > 0.0);
        let in_vigil = matches!(self.screen, Screen::Playing | Screen::Dead { .. } | Screen::Victory { .. });
        for (i, (text, life)) in self.popups.iter().enumerate() {
            let k = life.min(1.0);
            let w = ui.width(text, 19.0) + 40.0;
            // In a vigil: above a tip or note on screen, otherwise just above the boss bar.
            // In menus and the pause menu: the bottom right corner, clear of rows and tabs.
            let (cx, y) = if in_vigil {
                let base = if self.note.is_some() { (sh * 0.82 - 44.0).min(sh - 130.0) } else { sh - 130.0 };
                (sw * 0.5, base - i as f32 * 34.0)
            } else {
                (sw - 16.0 - w * 0.5, sh - 64.0 - i as f32 * 34.0)
            };
            draw_rectangle(cx - w * 0.5, y - 22.0, w, 30.0, Color::new(0.05, 0.03, 0.02, 0.85 * k));
            draw_rectangle_lines(cx - w * 0.5, y - 22.0, w, 30.0, 1.5, a(GOLD, k));
            ui.text_c(text, cx, y, 19.0, a(GOLD, k));
        }
    }

    // ---------------------------------------------------------------- end of vigil

    /// The end screen: title, numbers, damage per weapon and how the fight went.
    pub(super) fn draw_summary(&self, ui: &Ui, w: &World, t: f32, lost: u64) {
        let (sw, sh) = (ui_w(), ui_h());
        let s = &self.summary;
        let k = (t / 1.5).min(1.0);
        let tint = if s.won { Color::new(0.95, 0.8, 0.5, 0.18 * k) } else { Color::new(0.0, 0.0, 0.0, 0.0) };
        draw_rectangle(0.0, 0.0, sw, sh, tint);
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.72 * k));
        let (title, col) = match (s.won, w.heroes.len() > 1) {
            (true, _) if self.save.settings.cute => ("WHAT A LOVELY NIGHT", GOLD),
            (false, true) if self.save.settings.cute => ("NAP TIME FOR EVERYONE", BLOOD),
            (false, false) if self.save.settings.cute => ("YOU NEED A NAP", BLOOD),
            (true, _) if w.boss_rush => ("THE LORDS HAVE FALLEN", GOLD),
            (true, _) => ("THE VIGIL IS KEPT", GOLD),
            (false, true) => ("ALL HAVE FALLEN", BLOOD),
            (false, false) => ("YOU HAVE FALLEN", BLOOD),
        };
        ui.spaced_c(title, sw * 0.5, sh * 0.13, 48.0, 9.0, a(col, k));
        if t < 1.2 {
            return;
        }
        let k2 = ((t - 1.2) / 0.8).min(1.0);
        let me = w.me();
        let panel_w = (sw - 80.0).min(1060.0);
        let x0 = sw * 0.5 - panel_w * 0.5;
        let top = sh * 0.19;
        // Left: the numbers.
        let secs = w.time as u32;
        let dps = if w.time > 0.0 { me.dmg_total / w.time as f64 } else { 0.0 };
        let mut rows: Vec<(String, String, Color)> = vec![
            ("Endured".into(), format!("{:02}:{:02}", secs / 60, secs % 60), PALE),
            ("Level".into(), w.level.to_string(), PALE),
            ("Felled".into(), w.kills.to_string(), PALE),
            ("Bosses slain".into(), w.bosses_killed.to_string(), PALE),
            ("Your damage".into(), format!("{}  ({:.0}/s)", short(me.dmg_total), dps), PALE),
            ("Souls banked".into(), me.banked.to_string(), TEAL),
            ("Ashen Marks".into(), format!("+{}{}", s.marks, if s.challenge_bonus { "  (challenge bonus)" } else { "" }), EMBER),
        ];
        if lost > 0 {
            let where_ = if w.heroes.len() > 1 { "lost" } else { "left where you fell" };
            rows.push(("Souls".into(), format!("{} {}", lost, where_), BLOOD));
        }
        if let Some(r) = s.rank {
            rows.push(("Leaderboard".into(), format!("#{}", r + 1), GOLD));
        }
        if !s.new_bests.is_empty() {
            rows.push(("New best".into(), s.new_bests.join(", "), GOLD));
        }
        let lx = x0 + 10.0;
        let mut y = top + 30.0;
        for (label, value, c) in rows {
            ui.text(&label, lx, y, 18.0, a(DIM, k2));
            ui.text(&value, lx + 150.0, y, 18.0, a(c, k2));
            y += 26.0;
        }
        for ach in &s.achievements {
            ui.text(&format!("Achievement: {}", ach.name()), lx, y + 6.0, 16.0, a(GOLD, k2));
            y += 22.0;
        }
        // Right top: damage by weapon.
        let rx = x0 + panel_w * 0.45;
        let rw = panel_w * 0.55 - 10.0;
        ui.text("Damage by weapon", rx, top + 30.0, 18.0, a(PALE, k2));
        let mut by_weapon: Vec<(WeaponKind, f64)> = ALL_WEAPONS.iter().map(|k| (*k, me.dmg_weapon[k.idx()])).filter(|(_, d)| *d > 0.0).collect();
        by_weapon.sort_by(|a, b| b.1.total_cmp(&a.1));
        let max = by_weapon.first().map(|b| b.1).unwrap_or(1.0);
        let mut wy = top + 44.0;
        for (kind, dmg) in by_weapon.iter().take(7) {
            let frac = (*dmg / max) as f32;
            draw_rectangle(rx, wy, rw * frac, 18.0, a(Color::new(kind.color().r, kind.color().g, kind.color().b, 0.45), k2));
            mono(kind.name(false), rx + 4.0, wy + 14.0, 16.0, a(PALE, k2));
            let v = short(*dmg);
            let tw = mono_width(&v, 16.0);
            mono(&v, rx + rw - tw - 4.0, wy + 14.0, 16.0, a(PALE, k2));
            wy += 22.0;
        }
        // Right bottom: kills and damage over time.
        let gy = wy + 26.0;
        let gh = (sh * 0.86 - gy - 30.0).max(60.0);
        ui.text("The vigil over time", rx, gy, 18.0, a(PALE, k2));
        self.draw_run_graph(rx, gy + 12.0, rw, gh, k2);
        if t > 2.5 {
            let client = matches!(self.net, NetMode::Client(_));
            let rush = self.world.as_ref().is_some_and(|w| w.boss_rush);
            let onward = if rush { "Endless boss rush" } else { "Into the endless night" };
            if s.won && !client {
                // Two buttons for the mouse: leave, or fight on.
                for (i, label) in ["Leave", onward].iter().enumerate() {
                    let r = Rect::new(sw * 0.5 - 250.0 + i as f32 * 260.0, sh - 92.0, 240.0, 34.0);
                    crate::ui::register_tab(r);
                    let hot = r.contains(mouse_ui());
                    draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.07, 0.06, 0.06, if hot { 0.95 } else { 0.7 } * k2));
                    draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.5, a(if hot { GOLD } else { DIM }, k2));
                    ui.text_c(label, r.x + r.w * 0.5, r.y + 23.0, 18.0, a(if i == 1 { EMBER } else { PALE }, k2));
                }
            }
            let more = if s.won && !client && rush {
                hint("Enter: leave     C: the lords keep coming, until you fall", "A: leave     Y: the lords keep coming, until you fall")
            } else if s.won && !client {
                hint("Enter: leave     C: keep fighting into the endless night", "A: leave     Y: keep fighting into the endless night")
            } else if s.won && rush {
                "The host decides: back to the lobby, or on into an endless boss rush."
            } else if s.won {
                "The host decides: back to the lobby, or on into the endless night."
            } else {
                hint("Enter", "A")
            };
            ui.text_c(more, sw * 0.5, sh - 30.0, 18.0, a(DIM, k2));
        }
    }

    /// Kills and damage per sample interval as two lines.
    fn draw_run_graph(&self, x: f32, y: f32, w: f32, h: f32, k: f32) {
        draw_rectangle(x, y, w, h, Color::new(0.0, 0.0, 0.0, 0.45 * k));
        let s = &self.run_log.samples;
        if s.len() < 2 {
            return;
        }
        // Per-second rates, so a short last stretch does not dip.
        let mut rates: Vec<(f32, f32)> = s
            .windows(2)
            .map(|p| {
                let dt = (p[1].0 - p[0].0).max(1.0);
                ((p[1].1 - p[0].1) as f32 / dt, (p[1].2 - p[0].2) as f32 / dt)
            })
            .collect();
        if rates.len() == 1 {
            rates.push(rates[0]);
        }
        let max_k = rates.iter().map(|r| r.0).fold(0.01, f32::max);
        let max_d = rates.iter().map(|r| r.1).fold(0.01, f32::max);
        let n = rates.len() - 1;
        let pt = |i: usize, v: f32, max: f32| vec2(x + 6.0 + (w - 12.0) * i as f32 / n as f32, y + h - 6.0 - (h - 12.0) * v / max);
        for i in 1..rates.len() {
            let (a0, a1) = (pt(i - 1, rates[i - 1].1, max_d), pt(i, rates[i].1, max_d));
            draw_line(a0.x, a0.y, a1.x, a1.y, 2.0, Color::new(1.0, 0.55, 0.25, 0.9 * k));
            let (b0, b1) = (pt(i - 1, rates[i - 1].0, max_k), pt(i, rates[i].0, max_k));
            draw_line(b0.x, b0.y, b1.x, b1.y, 2.0, Color::new(0.6, 0.85, 1.0, 0.9 * k));
        }
        mono("damage", x + 8.0, y + 16.0, 14.0, Color::new(1.0, 0.55, 0.25, k));
        mono("kills", x + 70.0, y + 16.0, 14.0, Color::new(0.6, 0.85, 1.0, k));
    }

    // ---------------------------------------------------------------- records screen

    pub(super) fn update_records(&mut self) {
        drain_chars();
        let pages = record_pages();
        let dir = if pressed_any(&[KeyCode::Tab, KeyCode::E, KeyCode::D, KeyCode::Right]) || pad_pressed(Pad::PageNext) {
            1
        } else if pressed_any(&[KeyCode::Q, KeyCode::A, KeyCode::Left]) || pad_pressed(Pad::PagePrev) {
            -1
        } else {
            0
        };
        if dir != 0 {
            self.records_page = (self.records_page as i32 + dir).rem_euclid(pages as i32) as usize;
            self.audio.play("select", 0.5);
        }
        if let Some(t) = clicked_tab() {
            if t < pages && t != self.records_page {
                self.records_page = t;
                self.audio.play("select", 0.5);
            } else if t == pages {
                self.screen = Screen::Menu;
            }
        }
        if back() || confirm() {
            self.screen = Screen::Menu;
        }
    }

    pub(super) fn draw_records(&self, ui: &Ui) {
        let (sw, sh) = (ui_w(), ui_h());
        let cx = sw * 0.5;
        ui.spaced_c("RECORDS", cx, 54.0, 40.0, 8.0, PALE);
        let mut tabs = vec!["Leaderboard".to_string()];
        let ach_pages = record_pages() - 1;
        for k in 0..ach_pages {
            tabs.push(format!("Achievements {}", ["I", "II", "III", "IV", "V"][k.min(4)]));
        }
        let x0 = cx - (tabs.len() as f32 * 200.0 - 20.0) * 0.5;
        for (i, name) in tabs.iter().enumerate() {
            let x = x0 + i as f32 * 200.0;
            crate::ui::register_tab(Rect::new(x, 72.0, 180.0, 30.0));
            let on = i == self.records_page;
            draw_rectangle(x, 72.0, 180.0, 30.0, Color::new(0.07, 0.06, 0.06, if on { 0.95 } else { 0.5 }));
            if on {
                draw_rectangle(x, 100.0, 180.0, 2.0, GOLD);
            }
            ui.text_c(name, x + 90.0, 94.0, 19.0, if on { GOLD } else { DIM });
        }
        let w = 860.0f32.min(sw - 40.0);
        let x = cx - w * 0.5;
        if self.records_page == 0 {
            let cols =
                [("#", 0.0), ("Result", 40.0), ("Time", 150.0), ("Kills", 240.0), ("Level", 340.0), ("Damage", 420.0), ("Class", 530.0), ("Players", 680.0), ("Difficulty", 760.0)];
            for (name, off) in cols {
                ui.text(name, x + off, 136.0, 15.0, DIM);
            }
            if self.save.runs.is_empty() {
                ui.text_c("No vigils yet. The first one is waiting.", cx, 190.0, 18.0, DIM);
            }
            for (i, r) in self.save.runs.iter().enumerate() {
                let y = 162.0 + i as f32 * 34.0;
                ui.row(x - 8.0, y - 20.0, w + 16.0, 30.0, false);
                let secs = r.time as u32;
                let vals = [
                    format!("{}", i + 1),
                    if r.won { "Dawn".into() } else { "Fallen".into() },
                    format!("{:02}:{:02}", secs / 60, secs % 60),
                    r.kills.to_string(),
                    r.level.to_string(),
                    short(r.damage),
                    r.class.name().to_string(),
                    r.players.to_string(),
                    r.difficulty.clone(),
                ];
                for ((_, off), v) in cols.iter().zip(vals.iter()) {
                    let c = if *off == 40.0 {
                        if r.won {
                            GOLD
                        } else {
                            BLOOD
                        }
                    } else {
                        PALE
                    };
                    ui.text(v, x + off, y, 17.0, c);
                }
            }
            let b = self.save.best_time as u32;
            ui.text_c(
                &format!(
                    "Bests: longest {:02}:{:02}   most kills {}   highest level {}   most damage {}",
                    b / 60,
                    b % 60,
                    self.save.best_kills,
                    self.save.best_level,
                    short(self.save.best_damage)
                ),
                cx,
                sh - 54.0,
                16.0,
                GOLD,
            );
        } else {
            let done = ALL_ACHIEVEMENTS.iter().filter(|a| self.save.achievements & a.bit() != 0).count();
            ui.text_c(&format!("{} of {} earned", done, ALL_ACHIEVEMENTS.len()), cx, 128.0, 16.0, DIM);
            let wide = (sw - 40.0).min(1180.0);
            let x = cx - wide * 0.5;
            let col_w = wide / 3.0;
            let first = (self.records_page - 1) * ACH_PER_PAGE;
            for (i, ach) in ALL_ACHIEVEMENTS.iter().skip(first).take(ACH_PER_PAGE).enumerate() {
                let have = self.save.achievements & ach.bit() != 0;
                let cx0 = x + (i % 3) as f32 * col_w;
                let y = 144.0 + (i / 3) as f32 * 44.0;
                ui.row(cx0, y, col_w - 8.0, 40.0, false);
                draw_poly(cx0 + 18.0, y + 20.0, 6, 10.0, 0.0, if have { GOLD } else { Color::new(0.2, 0.18, 0.16, 1.0) });
                ui.text(ach.name(), cx0 + 36.0, y + 17.0, 16.0, if have { GOLD } else { DIM });
                ui.text(ach.desc(), cx0 + 36.0, y + 34.0, 12.0, if have { PALE } else { DIM });
                ui.text_r(&format!("+{}", ach.reward()), cx0 + col_w - 18.0, y + 17.0, 13.0, if have { TEAL } else { EMBER });
            }
        }
        // A back button, the third clickable tab.
        let back_r = Rect::new(cx - 80.0, sh - 108.0, 160.0, 30.0);
        crate::ui::register_tab(back_r);
        let hot = back_r.contains(mouse_ui());
        draw_rectangle(back_r.x, back_r.y, back_r.w, back_r.h, Color::new(0.07, 0.06, 0.06, if hot { 0.95 } else { 0.6 }));
        if hot {
            draw_rectangle_lines(back_r.x, back_r.y, back_r.w, back_r.h, 2.0, GOLD);
        }
        ui.text_c("Back", cx, back_r.y + 21.0, 18.0, if hot { GOLD } else { DIM });
        ui.text_c(hint("Tab / A / D: page    Esc back", "RB: page    B back"), cx, sh - 20.0, 15.0, DIM);
    }
}
