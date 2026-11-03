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
        check(me.weapons.iter().any(|wp| wp.fused.is_some()), Achievement::Fusionist);
        check(me.weapons.iter().any(|wp| wp.evolved && wp.fused.is_none()), Achievement::Evolved);
        check(me.rolls == 0 && w.time >= 600.0 && !me.downed, Achievement::Steadfast);
        check(me.has_cat, Achievement::CatPerson);
        check(me.purchases >= 3, Achievement::Patron);
        check(w.moons_survived >= 1, Achievement::MoonWatcher);
        check(w.trials_won >= 1, Achievement::TrialEndured);
        check(w.heroes.len() >= 2 && w.time >= 600.0, Achievement::Fellowship);
        for a in earned {
            self.unlock(a);
        }
    }

    /// Leaderboard, bests and end-of-vigil achievements.
    pub(super) fn finish_run(&mut self, won: bool, marks: u64) {
        let online = self.online();
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
            difficulty: w.diff.preset_name().to_string(),
            damage: me.dmg_total,
        };
        let abyss = PRESETS[3].1;
        let hard_enough = w.diff.count >= abyss.count && w.diff.hp >= abyss.hp && w.diff.dmg >= abyss.dmg && w.diff.speed >= abyss.speed;
        let challenges = if online { 0 } else { self.save.challenges };
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
        if won {
            self.unlock(Achievement::Dawnbringer);
            if hard_enough {
                self.unlock(Achievement::AbyssWalker);
            }
            if challenges & Challenge::OneWeapon.bit() != 0 {
                self.unlock(Achievement::Purist);
            }
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
        for (i, (text, life)) in self.popups.iter().enumerate() {
            let k = life.min(1.0);
            // Below the level-up cards and their hints, above the boss bar.
            // Above a tip or note on screen, otherwise just above the boss bar.
            let base = if self.note.is_some() { (sh * 0.82 - 44.0).min(sh - 130.0) } else { sh - 130.0 };
            let y = base - i as f32 * 34.0;
            let w = ui.width(text, 19.0) + 40.0;
            draw_rectangle(sw * 0.5 - w * 0.5, y - 22.0, w, 30.0, Color::new(0.05, 0.03, 0.02, 0.85 * k));
            draw_rectangle_lines(sw * 0.5 - w * 0.5, y - 22.0, w, 30.0, 1.5, a(GOLD, k));
            ui.text_c(text, sw * 0.5, y, 19.0, a(GOLD, k));
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
            let more = if s.won && !self.online() {
                hint("Enter: leave     C: keep fighting into the endless night", "A: leave     Y: keep fighting into the endless night")
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
        if pressed_any(&[KeyCode::Tab, KeyCode::Q, KeyCode::E, KeyCode::A, KeyCode::D, KeyCode::Left, KeyCode::Right]) || pad_pressed(Pad::PageNext) {
            self.records_page = 1 - self.records_page;
            self.audio.play("select", 0.5);
        }
        if back() || confirm() {
            self.screen = Screen::Menu;
        }
    }

    pub(super) fn draw_records(&self, ui: &Ui) {
        let (sw, sh) = (ui_w(), ui_h());
        let cx = sw * 0.5;
        ui.spaced_c("RECORDS", cx, 54.0, 40.0, 8.0, PALE);
        let tabs = ["Leaderboard", "Achievements"];
        for (i, name) in tabs.iter().enumerate() {
            let x = cx - 190.0 + i as f32 * 200.0;
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
            let col_w = w * 0.5;
            for (i, ach) in ALL_ACHIEVEMENTS.iter().enumerate() {
                let have = self.save.achievements & ach.bit() != 0;
                let cx0 = x + (i % 2) as f32 * col_w;
                let y = 150.0 + (i / 2) as f32 * 60.0;
                ui.row(cx0, y, col_w - 10.0, 54.0, false);
                draw_poly(cx0 + 24.0, y + 27.0, 6, 14.0, 0.0, if have { GOLD } else { Color::new(0.2, 0.18, 0.16, 1.0) });
                ui.text(ach.name(), cx0 + 48.0, y + 22.0, 19.0, if have { GOLD } else { DIM });
                ui.text(ach.desc(), cx0 + 48.0, y + 44.0, 14.0, if have { PALE } else { DIM });
                ui.text_r(&format!("+{}", ach.reward()), cx0 + col_w - 22.0, y + 22.0, 15.0, if have { TEAL } else { EMBER });
            }
        }
        ui.text_c(hint("Tab / A / D: page    Esc back", "RB: page    B back"), cx, sh - 20.0, 15.0, DIM);
    }
}
