//! Screen-space UI: HUD, overlays and shared drawing helpers for the menus.

use crate::app::input::{hint, key_label, using_pad};
use crate::data::*;
use crate::net::NetStats;
use crate::save::Action;
use crate::world::*;
use macroquad::prelude::*;

pub const GOLD: Color = Color::new(0.85, 0.7, 0.4, 1.0);
pub const PALE: Color = Color::new(0.86, 0.83, 0.78, 1.0);
pub const DIM: Color = Color::new(0.55, 0.52, 0.48, 1.0);
pub const BLOOD: Color = Color::new(0.62, 0.08, 0.06, 1.0);
pub const TEAL: Color = Color::new(0.4, 0.95, 0.8, 1.0);
pub const EMBER: Color = Color::new(1.0, 0.65, 0.35, 1.0);

pub struct Ui<'a> {
    pub font: Option<&'a Font>,
}

thread_local! {
    static UI_SCALE: std::cell::Cell<f32> = const { std::cell::Cell::new(1.0) };
    /// Off-screen markers sit this share of the shorter screen side inside the edge.
    static MARKER_INSET: std::cell::Cell<f32> = const { std::cell::Cell::new(0.12) };
}

/// Takes the marker setting (percent); called every frame.
pub fn set_marker_inset(percent: u8) {
    MARKER_INSET.with(|m| m.set(percent as f32 / 100.0));
}

thread_local! {
    /// Menu rows and tabs as drawn last frame, in drawing order, for the mouse.
    static ROWS: std::cell::RefCell<Vec<Rect>> = const { std::cell::RefCell::new(Vec::new()) };
    static TABS: std::cell::RefCell<Vec<Rect>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Forgets last frame's rows and tabs; called before drawing a frame.
pub fn clear_hit_areas() {
    ROWS.with(|r| r.borrow_mut().clear());
    TABS.with(|t| t.borrow_mut().clear());
}

pub fn rows_drawn() -> Vec<Rect> {
    ROWS.with(|r| r.borrow().clone())
}

pub fn tabs_drawn() -> Vec<Rect> {
    TABS.with(|t| t.borrow().clone())
}

/// Marks a page tab as clickable.
pub fn register_tab(r: Rect) {
    TABS.with(|t| t.borrow_mut().push(r));
}

/// UI scale choices; 0 means automatic (grows with the window above 1080 lines).
pub const UI_SCALES: [f32; 6] = [0.0, 1.0, 1.25, 1.5, 1.75, 2.0];

/// Sets the UI scale for this frame from the setting (an index into `UI_SCALES`).
pub fn set_ui_scale(choice: u8) {
    let s = match UI_SCALES.get(choice as usize).copied().unwrap_or(0.0) {
        0.0 => (screen_height() / 1080.0).max(1.0),
        s => s,
    };
    // Never so large that the menus no longer fit.
    let s = s.min(screen_height() / 640.0).min(screen_width() / 1000.0).max(0.5);
    UI_SCALE.with(|c| c.set(s));
}

pub fn ui_scale() -> f32 {
    UI_SCALE.with(|c| c.get())
}

/// Width of the UI space; all screen-space drawing uses these instead of the window size.
pub fn ui_w() -> f32 {
    screen_width() / ui_scale()
}

pub fn ui_h() -> f32 {
    screen_height() / ui_scale()
}

/// Draws everything after this in UI space.
pub fn set_ui_camera() {
    let (w, h) = (ui_w(), ui_h());
    set_camera(&Camera2D { target: vec2(w, h) * 0.5, zoom: vec2(2.0 / w, 2.0 / h), ..Default::default() });
}

/// Text in the built-in font, rasterized at the real pixel size so it stays sharp when scaled.
pub fn mono(s: impl AsRef<str>, x: f32, y: f32, size: f32, color: Color) {
    let k = ui_scale();
    draw_text_ex(s.as_ref(), x, y, TextParams { font_size: (size * k) as u16, font_scale: 1.0 / k, color, ..Default::default() });
}

pub fn mono_width(s: &str, size: f32) -> f32 {
    let k = ui_scale();
    measure_text(s, None, (size * k) as u16, 1.0 / k).width
}

/// 12345 -> "12.3k".
/// Where the level-up cards sit, for drawing and for the mouse.
pub fn level_up_cards(n: usize) -> Vec<Rect> {
    let (sw, sh) = (ui_w(), ui_h());
    let cw = 560.0f32.min(sw - 40.0);
    (0..n).map(|i| Rect::new((sw - cw) * 0.5, sh * 0.16 + 56.0 + i as f32 * 84.0, cw, 74.0)).collect()
}

pub fn short(v: f64) -> String {
    if v >= 1e12 {
        format!("{:.1}T", v / 1e12)
    } else if v >= 1e9 {
        format!("{:.1}B", v / 1e9)
    } else if v >= 1_000_000.0 {
        format!("{:.1}M", v / 1_000_000.0)
    } else if v >= 10_000.0 {
        format!("{:.0}k", v / 1000.0)
    } else if v >= 1000.0 {
        format!("{:.1}k", v / 1000.0)
    } else {
        format!("{:.0}", v)
    }
}

pub fn a(c: Color, alpha: f32) -> Color {
    Color::new(c.r, c.g, c.b, c.a * alpha)
}

impl<'a> Ui<'a> {
    pub fn width(&self, s: &str, size: f32) -> f32 {
        let k = ui_scale();
        measure_text(s, self.font, (size * k) as u16, 1.0 / k).width
    }

    pub fn text(&self, s: &str, x: f32, y: f32, size: f32, color: Color) {
        let k = ui_scale();
        let p = TextParams { font: self.font, font_size: (size * k) as u16, font_scale: 1.0 / k, color: Color::new(0.0, 0.0, 0.0, color.a * 0.8), ..Default::default() };
        draw_text_ex(s, x + 1.5, y + 1.5, p.clone());
        draw_text_ex(s, x, y, TextParams { color, ..p });
    }

    pub fn text_c(&self, s: &str, cx: f32, y: f32, size: f32, color: Color) {
        let w = self.width(s, size);
        self.text(s, cx - w * 0.5, y, size, color);
    }

    pub fn text_r(&self, s: &str, rx: f32, y: f32, size: f32, color: Color) {
        let w = self.width(s, size);
        self.text(s, rx - w, y, size, color);
    }

    /// Letter-spaced centered text for the big souls-style titles.
    pub fn spaced_c(&self, s: &str, cx: f32, y: f32, size: f32, spacing: f32, color: Color) {
        let widths: Vec<f32> = s.chars().map(|c| self.width(&c.to_string(), size)).collect();
        let total: f32 = widths.iter().sum::<f32>() + spacing * (widths.len().saturating_sub(1)) as f32;
        let mut x = cx - total * 0.5;
        for (c, w) in s.chars().zip(widths) {
            self.text(&c.to_string(), x, y, size, color);
            x += w + spacing;
        }
    }

    /// A selectable menu row background. Rows are remembered in drawing order, which every
    /// menu keeps the same as its selection order, so the mouse can pick them.
    pub fn row(&self, x: f32, y: f32, w: f32, h: f32, on: bool) {
        ROWS.with(|r| r.borrow_mut().push(Rect::new(x, y, w, h)));
        draw_rectangle(x, y, w, h, Color::new(0.07, 0.06, 0.06, if on { 0.95 } else { 0.7 }));
        if on {
            draw_rectangle_lines(x, y, w, h, 2.0, GOLD);
        }
    }

    /// A framed bar filled to `frac`; `lag` (above `frac`) shows recent damage in gold.
    pub fn bar(&self, r: Rect, frac: f32, lag: f32, fill: Color) {
        let Rect { x, y, w, h } = r;
        draw_rectangle(x - 2.0, y - 2.0, w + 4.0, h + 4.0, Color::new(0.0, 0.0, 0.0, 0.7));
        draw_rectangle(x, y, w, h, Color::new(0.08, 0.07, 0.07, 1.0));
        if lag > frac {
            draw_rectangle(x, y, w * lag.clamp(0.0, 1.0), h, Color::new(0.9, 0.75, 0.35, 0.9));
        }
        draw_rectangle(x, y, w * frac.clamp(0.0, 1.0), h, fill);
        draw_rectangle(x, y, w * frac.clamp(0.0, 1.0), h * 0.35, Color::new(1.0, 1.0, 1.0, 0.08));
        draw_rectangle_lines(x - 1.0, y - 1.0, w + 2.0, h + 2.0, 1.0, Color::new(0.4, 0.36, 0.3, 0.6));
    }

    /// Arrows at the screen edge pointing at things off screen. The nearest gets a label;
    /// markers crowding the same spot only get their arrow.
    fn edge_markers(&self, w: &World, mut marks: Vec<(Vec2, Color, String)>) {
        let (sw, sh) = (ui_w(), ui_h());
        let me = w.me().p.pos;
        marks.sort_by(|a, b| a.0.distance_squared(me).total_cmp(&b.0.distance_squared(me)));
        let mut labeled: Vec<Vec2> = Vec::new();
        for (target, color, label) in marks {
            let s = w.world_to_screen(target);
            if s.x > 0.0 && s.y > 0.0 && s.x < sw && s.y < sh {
                continue;
            }
            let c = vec2(sw, sh) * 0.5;
            let d = (s - c).normalize_or_zero();
            // Inside the screen by the chosen amount, so a glance from the middle catches them.
            let m = (sw.min(sh) * MARKER_INSET.with(|m| m.get())).max(20.0);
            // Keep clear of the boss bar along the bottom edge.
            let my = if d.y > 0.0 && w.boss_kind.is_some() { m.max(105.0) } else { m };
            let tx = if d.x.abs() > 0.001 { ((sw * 0.5 - m) / d.x.abs()).abs() } else { f32::MAX };
            let ty = if d.y.abs() > 0.001 { ((sh * 0.5 - my) / d.y.abs()).abs() } else { f32::MAX };
            let p = c + d * tx.min(ty);
            let perp = vec2(-d.y, d.x);
            draw_triangle(p + d * 12.0, p - d * 6.0 + perp * 8.0, p - d * 6.0 - perp * 8.0, color);
            if labeled.iter().any(|q| q.distance(p) < 70.0) {
                continue;
            }
            labeled.push(p);
            let dist = (target - me).length() / 10.0;
            self.text_c(&format!("{} {:.0}m", label, dist), p.x - d.x * 24.0, p.y - d.y * 24.0 + 4.0, 14.0, a(color, 0.9));
        }
    }

    pub fn hud(&self, w: &World) {
        let (sw, sh) = (ui_w(), ui_h());
        let me = w.me();
        let pl = &me.p;

        // Shared experience across the top edge.
        draw_rectangle(0.0, 0.0, sw, 6.0, Color::new(0.05, 0.04, 0.04, 0.9));
        draw_rectangle(0.0, 0.0, sw * (w.xp / w.xp_next).min(1.0), 6.0, Color::new(0.75, 0.6, 0.3, 1.0));
        self.text(&format!("Lv {}", w.level), sw - 70.0, 26.0, 20.0, GOLD);

        let hp_w = 180.0 + me.stats.max_hp * 0.8;
        self.bar(Rect::new(22.0, 20.0, hp_w, 11.0), pl.hp / me.stats.max_hp, 0.0, Color::new(0.6, 0.08, 0.06, 1.0));
        let st_w = 120.0 + me.stats.stamina_max * 0.8;
        self.bar(Rect::new(22.0, 38.0, st_w, 7.0), pl.stamina / me.stats.stamina_max, 0.0, Color::new(0.25, 0.55, 0.28, 1.0));

        // Weapons: an icon each, a coloured frame for evolved and fused ones, the level below.
        let mut x = 22.0;
        for wp in &me.weapons {
            let box_c = vec2(x + 15.0, 71.0);
            draw_rectangle(x, 56.0, 30.0, 30.0, Color::new(0.05, 0.05, 0.05, 0.85));
            match wp.tier() {
                0 => crate::icons::weapon_icon(wp.kind, box_c, 24.0, wp.kind.color()),
                1 => {
                    crate::icons::weapon_icon(wp.kind, box_c + vec2(-5.0, -5.0), 17.0, wp.kind.color());
                    crate::icons::weapon_icon(wp.fused[0], box_c + vec2(5.0, 5.0), 17.0, wp.fused[0].color());
                }
                _ => {
                    // A grid of every weapon inside the mega or ultra fusion.
                    let kinds = wp.kinds();
                    let cols = if kinds.len() > 4 { 4 } else { 2 };
                    let cell = 26.0 / cols as f32;
                    let rows = kinds.len().div_ceil(cols);
                    let cell_h = 26.0 / rows as f32;
                    for (k, kind) in kinds.iter().enumerate() {
                        let at = vec2(x + 2.0 + (k % cols) as f32 * cell + cell * 0.5, 58.0 + (k / cols) as f32 * cell_h + cell_h * 0.5);
                        crate::icons::weapon_icon(*kind, at, cell.min(cell_h) * 1.15, kind.color());
                    }
                }
            }
            let ready = !wp.evolved && wp.level >= WEAPON_MAX_LEVEL && me.items.iter().any(|it| it.kind == wp.kind.evo_item());
            let border = if wp.tier() >= 4 {
                // The last flame burns around its frame.
                let fl = 0.75 + 0.25 * (w.time * 5.0).sin();
                draw_rectangle(x - 3.0, 53.0, 36.0, 36.0, Color::new(1.0, 0.8, 0.4, 0.18 * fl));
                Color::new(1.0, 0.85 + 0.15 * fl, 0.55 + 0.3 * fl, 1.0)
            } else if wp.tier() >= 3 {
                Color::new(1.0, 0.95, 0.75, 1.0)
            } else if wp.tier() == 2 {
                Color::new(0.85, 0.55, 1.0, 1.0)
            } else if wp.is_fused() {
                Color::new(1.0, 0.35, 0.25, 1.0)
            } else if wp.evolved {
                GOLD
            } else if ready {
                Color::new(1.0, 0.5, 0.2, 0.6 + 0.4 * (w.time * 6.0).sin())
            } else {
                Color::new(0.35, 0.32, 0.3, 1.0)
            };
            draw_rectangle_lines(x, 56.0, 30.0, 30.0, 2.0, border);
            let lv = match wp.tier() {
                4 => "LAST".to_string(),
                3 => "ULTRA".to_string(),
                2 => "MEGA".to_string(),
                1 => "FUSED".to_string(),
                _ if wp.evolved => "EVO".to_string(),
                _ => format!("{}", wp.level),
            };
            draw_rectangle(x, 86.0, 30.0, 11.0, Color::new(0.0, 0.0, 0.0, 0.6));
            let lw = mono_width(&lv, 11.0);
            mono(&lv, x + 15.0 - lw * 0.5, 95.0, 11.0, if wp.tier() > 0 || wp.evolved { border } else { PALE });
            x += 34.0;
        }
        // Items: an icon each and their level.
        let mut x = 22.0;
        for it in &me.items {
            draw_rectangle(x, 100.0, 22.0, 22.0, Color::new(0.05, 0.05, 0.05, 0.85));
            crate::icons::item_icon(it.kind, vec2(x + 11.0, 111.0), 18.0, it.kind.color());
            draw_rectangle_lines(x, 100.0, 22.0, 22.0, 1.5, Color::new(0.35, 0.32, 0.3, 1.0));
            let lv = it.level.to_string();
            draw_rectangle(x + 13.0, 113.0, 9.0, 9.0, Color::new(0.0, 0.0, 0.0, 0.7));
            mono(&lv, x + 14.5, 121.0, 11.0, PALE);
            x += 26.0;
        }

        let mut bx = 22.0;
        for b in ALL_BUFFS {
            let t = me.buff(b);
            if t <= 0.0 {
                continue;
            }
            draw_rectangle(bx, 128.0, 70.0, 14.0, Color::new(0.0, 0.0, 0.0, 0.6));
            draw_rectangle(bx, 138.0, 70.0 * (t / b.duration()).min(1.0), 4.0, b.color());
            mono(b.name(), bx + 3.0, 137.0, 13.0, b.color());
            bx += 76.0;
        }

        // Companions.
        if w.heroes.len() > 1 {
            let mut y = 162.0;
            // Bars line up just past the longest name.
            let bar_x = w.heroes.iter().filter(|h| h.slot != me.slot).map(|h| 36.0 + self.width(&h.name, 15.0) + 10.0).fold(140.0, f32::max);
            for h in w.heroes.iter().filter(|h| h.slot != me.slot) {
                draw_rectangle(22.0, y - 11.0, 8.0, 8.0, slot_color(h.slot));
                let col = if h.downed { BLOOD } else { PALE };
                self.text(&h.name, 36.0, y - 2.0, 15.0, col);
                self.bar(
                    Rect::new(bar_x, y - 10.0, 90.0, 6.0),
                    if h.downed { h.revive / 3.0 } else { h.p.hp / h.stats.max_hp },
                    0.0,
                    if h.downed { GOLD } else { Color::new(0.6, 0.08, 0.06, 1.0) },
                );
                y += 18.0;
            }
        }

        let secs = w.time as u32;
        self.text_c(&format!("{:02}:{:02}", secs / 60, secs % 60), sw * 0.5, 38.0, 32.0, PALE);
        self.text(&format!("Felled {}", w.kills), sw - 150.0, 50.0, 16.0, DIM);
        if pl.revives > 0 {
            self.text(&format!("Embers {}", pl.revives), sw - 150.0, 68.0, 16.0, EMBER);
        }

        let souls = format!("{}", pl.souls);
        let sx = sw - 40.0 - self.width(&souls, 28.0);
        draw_rectangle(sx - 46.0, sh - 54.0, sw - sx + 36.0, 38.0, Color::new(0.0, 0.0, 0.0, 0.55));
        draw_poly(sx - 24.0, sh - 35.0, 4, 9.0, 45.0, Color::new(0.7, 0.85, 1.0, 1.0));
        draw_poly(sx - 24.0, sh - 35.0, 4, 4.0, 45.0, WHITE);
        self.text(&souls, sx, sh - 25.0, 28.0, PALE);
        if me.banked > 0 {
            self.text_r(&format!("banked {}", me.banked), sw - 30.0, sh - 62.0, 14.0, DIM);
        }

        if let Some(k) = w.boss_kind {
            let bw = sw * 0.6;
            let bx = (sw - bw) * 0.5;
            let by = sh - 70.0;
            self.text(k.name(), bx, by - 10.0, 22.0, PALE);
            self.bar(Rect::new(bx, by, bw, 10.0), w.boss_hp.0 / w.boss_hp.1.max(1.0), w.boss_bar_lag, Color::new(0.55, 0.07, 0.05, 1.0));
        }

        let mut marks: Vec<(Vec2, Color, String)> = Vec::new();
        if let Some(c) = w.cursed.as_ref().filter(|c| !c.accepted) {
            marks.push((c.pos, Color::new(0.75, 0.4, 1.0, 0.9), "Cursed shrine".into()));
        }
        if let Some(m) = &w.merchant {
            marks.push((m.pos, Color::new(0.65, 0.8, 1.0, 0.9), "Merchant".into()));
        }
        for s in w.shrines.iter().filter(|s| !s.used) {
            marks.push((s.pos, Color::new(1.0, 0.6, 0.25, 0.9), "Shrine".into()));
        }
        if let Some(r) = &w.remnant {
            marks.push((r.pos, TEAL, "Remnant".into()));
        }
        for p in &w.pickups {
            if let PickKind::Coffer(_) = p.kind {
                marks.push((p.pos, GOLD, "Coffer".into()));
            } else if p.is_relic() {
                marks.push((p.pos, Color::new(0.9, 0.35, 0.3, 0.85), "Relic".into()));
            }
        }
        for h in w.heroes.iter().filter(|h| h.slot != me.slot) {
            let c = if h.downed { BLOOD } else { Color::new(0.8, 0.8, 0.9, 0.9) };
            marks.push((h.p.pos, c, h.name.clone()));
        }
        self.edge_markers(w, marks);

        // Events in progress, under the clock.
        // Everything under the clock stacks, so nothing overlaps on small screens.
        let mut ey = 58.0;
        if w.boss_rush {
            let lord = if w.endless { format!("Lords slain {}", w.bosses_killed) } else { format!("Lord {} of {}", (w.bosses_killed + 1).min(RUSH_LORDS), RUSH_LORDS) };
            self.text_c(&lord, sw * 0.5, ey, 17.0, EMBER);
            ey += 20.0;
        }
        if w.boss_warning > 0.0 {
            // A lord is coming: a red pulse at the edges and a countdown under the clock.
            let t = w.boss_warning;
            let pulse = 0.5 + 0.5 * (t * std::f32::consts::TAU).cos();
            let edge = if w.cute { Color::new(1.0, 0.4, 0.7, 0.35 * pulse) } else { Color::new(0.75, 0.05, 0.03, 0.35 * pulse) };
            for (x, y, ww, hh) in [(0.0, 0.0, sw, 10.0), (0.0, sh - 10.0, sw, 10.0), (0.0, 0.0, 10.0, sh), (sw - 10.0, 0.0, 10.0, sh)] {
                draw_rectangle(x, y, ww, hh, edge);
            }
            let red = a(Color::new(0.95, 0.35, 0.25, 1.0), 0.6 + 0.4 * pulse);
            self.spaced_c(&format!("A LORD APPROACHES   {}", t.ceil() as u32), sw * 0.5, ey + 12.0, 26.0, 5.0, red);
            ey += 40.0;
        }
        if w.blood_moon > 0.0 {
            let pulse = 0.7 + 0.3 * (w.time * 3.0).sin();
            draw_circle(sw * 0.5 - 70.0, ey - 5.0, 7.0, Color::new(0.8, 0.08, 0.06, pulse));
            self.text(&format!("Blood Moon {:.0}s", w.blood_moon), sw * 0.5 - 56.0, ey, 16.0, Color::new(0.95, 0.3, 0.25, 1.0));
            ey += 20.0;
        }
        if let Some(c) = w.cursed.as_ref().filter(|c| c.curse > 0.0) {
            self.text_c(&format!("Trial {:.0}s", c.curse), sw * 0.5, ey, 16.0, Color::new(0.8, 0.5, 1.0, 1.0));
            ey += 20.0;
        }
        if let Some(m) = &w.merchant {
            self.text_c(&format!("Merchant leaves in {:.0}s", m.life), sw * 0.5, ey, 15.0, Color::new(0.65, 0.8, 1.0, 0.9));
            ey += 20.0;
        }

        // Level-up cards need the middle of the screen to themselves.
        if w.pause == Pause::Choosing {
            return;
        }
        for (i, (t, life)) in w.toasts.iter().enumerate() {
            // Toasts start below the event timers.
            self.text_c(t, sw * 0.5, ey.max(78.0) + i as f32 * 22.0, 18.0, a(PALE, life.min(1.0)));
        }

        if let Some((big, small, c, life)) = &w.banner {
            let al = (life / 0.6).min(1.0);
            let cy = sh * 0.38;
            if !big.is_empty() {
                draw_rectangle(0.0, cy - 46.0, sw, 70.0, Color::new(0.0, 0.0, 0.0, 0.5 * al));
                self.spaced_c(big, sw * 0.5, cy, 44.0, 6.0, a(*c, al));
            }
            if !small.is_empty() {
                self.text_c(small, sw * 0.5, cy + 34.0, 20.0, a(PALE, al * 0.9));
            }
        }
        if me.downed && !w.dead {
            self.spaced_c("FALLEN", sw * 0.5, sh * 0.6, 34.0, 8.0, a(BLOOD, 0.9));
            self.text_c("A companion can raise you by standing at your side.", sw * 0.5, sh * 0.6 + 28.0, 18.0, PALE);
        }
    }

    pub fn vignette(&self, tex: &Texture2D, w: &World) {
        let (sw, sh) = (ui_w(), ui_h());
        if w.blood_moon > 0.0 {
            let k = (w.blood_moon / 3.0).min(1.0) * 0.18;
            draw_texture_ex(tex, 0.0, 0.0, Color::new(0.5, 0.0, 0.0, k * 3.0), DrawTextureParams { dest_size: Some(vec2(sw, sh)), ..Default::default() });
            draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.35, 0.0, 0.0, k * 0.4));
        }
        // Cute mode keeps only a faint rosy edge instead of the dark.
        // The night closes in from the edges in a deep, cold blue rather than plain black.
        let edge = if w.cute { Color::new(0.55, 0.25, 0.45, 0.45) } else { Color::new(0.01, 0.015, 0.05, 1.0) };
        draw_texture_ex(tex, 0.0, 0.0, edge, DrawTextureParams { dest_size: Some(vec2(sw, sh)), ..Default::default() });
        let hurt = if w.cute { (1.0, 0.4, 0.7) } else { (0.6, 0.0, 0.0) };
        let me = w.me();
        let low = 1.0 - me.p.hp / me.stats.max_hp;
        if low > 0.6 && !w.dead {
            let pulse = 0.5 + 0.5 * (w.time * 5.0).sin();
            draw_texture_ex(
                tex,
                0.0,
                0.0,
                Color::new(hurt.0, hurt.1, hurt.2, (low - 0.6) * 1.5 * pulse),
                DrawTextureParams { dest_size: Some(vec2(sw, sh)), ..Default::default() },
            );
        }
        if me.p.iframes > 0.3 && me.p.roll_t <= 0.0 && !me.downed {
            draw_rectangle(0.0, 0.0, sw, sh, Color::new(hurt.0 * 0.8, hurt.1, hurt.2, 0.12));
        }
    }

    /// Level-up cards for our hero, or who we are still waiting for.
    pub fn level_up(&self, w: &World, sel: usize) {
        let (sw, sh) = (ui_w(), ui_h());
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.6));
        let me = w.me();
        self.spaced_c("LEVEL UP", sw * 0.5, sh * 0.16, 40.0, 5.0, GOLD);
        if me.picked {
            self.text_c("Your choice is made.", sw * 0.5, sh * 0.16 + 40.0, 22.0, PALE);
            let waiting: Vec<&str> = w.heroes.iter().filter(|h| !h.picked).map(|h| h.name.as_str()).collect();
            let dots = ".".repeat(((get_time() * 2.0) as usize) % 4);
            self.text_c(&format!("Waiting for {}{}", waiting.join(", "), dots), sw * 0.5, sh * 0.16 + 72.0, 20.0, DIM);
            return;
        }
        self.text_c("Choose what the ash grants you", sw * 0.5, sh * 0.16 + 30.0, 18.0, DIM);
        let cards = level_up_cards(me.choices.len());
        let (cw, ch) = (cards.first().map(|r| r.w).unwrap_or(0.0), 74.0);
        let mut y = sh * 0.16 + 56.0;
        for ((i, c), card) in me.choices.iter().enumerate().zip(&cards) {
            let x = card.x;
            y = card.y;
            let (title, desc, color, tag) = me.choice_text(*c);
            let on = i == sel;
            draw_rectangle(x, y, cw, ch, Color::new(0.07, 0.06, 0.06, 0.95));
            draw_rectangle_lines(x, y, cw, ch, if on { 2.5 } else { 1.0 }, if on { GOLD } else { Color::new(0.3, 0.27, 0.25, 1.0) });
            draw_rectangle(x + 14.0, y + 17.0, 40.0, 40.0, Color::new(0.02, 0.02, 0.02, 1.0));
            draw_rectangle_lines(x + 14.0, y + 17.0, 40.0, 40.0, 1.5, a(color, 0.6));
            match *c {
                Choice::NewWeapon(k) => crate::icons::weapon_icon(k, vec2(x + 34.0, y + 37.0), 32.0, color),
                Choice::UpWeapon(i) => {
                    if let Some(wp) = me.weapons.get(i) {
                        crate::icons::weapon_icon(wp.kind, vec2(x + 34.0, y + 37.0), 32.0, color);
                    }
                }
                Choice::NewItem(k) => crate::icons::item_icon(k, vec2(x + 34.0, y + 37.0), 30.0, color),
                Choice::UpItem(i) => {
                    if let Some(it) = me.items.get(i) {
                        crate::icons::item_icon(it.kind, vec2(x + 34.0, y + 37.0), 30.0, color);
                    }
                }
                _ => draw_rectangle(x + 20.0, y + 23.0, 28.0, 28.0, color),
            }
            if let Some((hint, hint_col)) = me.choice_hint(*c) {
                // Marks a card that fuses with or evolves something already carried.
                draw_rectangle_lines(x - 3.0, y - 3.0, cw + 6.0, ch + 6.0, 1.5, a(hint_col, 0.5 + 0.3 * (get_time() as f32 * 4.0).sin()));
                self.text_r(&hint, x + cw - 14.0, y + 56.0, 15.0, hint_col);
            }
            self.text(&format!("{}", i + 1), x + cw - 24.0, y + 22.0, 16.0, DIM);
            self.text(&title, x + 70.0, y + 30.0, 22.0, if on { PALE } else { a(PALE, 0.8) });
            let tag_col = if tag == "NEW" { EMBER } else { GOLD };
            self.text(&tag, x + 78.0 + self.width(&title, 22.0), y + 30.0, 16.0, tag_col);
            self.text(&desc, x + 70.0, y + 56.0, 16.0, DIM);
        }
        y += ch + 10.0;
        let reroll_key = if using_pad() { "Y" } else { key_label(Action::Reroll) };
        let reroll = if me.rerolls > 0 { format!("   {}: reroll ({})", reroll_key, me.rerolls) } else { String::new() };
        self.text_c(&format!("{}{}", hint("1-4, W/S + Enter, or click", "Stick + A"), reroll), sw * 0.5, y + 24.0, 16.0, DIM);
        if w.heroes.len() > 1 {
            let others: Vec<String> = w.heroes.iter().filter(|h| h.slot != me.slot).map(|h| format!("{} {}", h.name, if h.picked { "(ready)" } else { "(choosing)" })).collect();
            self.text_c(&others.join("     "), sw * 0.5, y + 50.0, 16.0, DIM);
        }
    }

    pub fn countdown(&self, t: f32) {
        let (sw, sh) = (ui_w(), ui_h());
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.35 * (t / RESUME_DELAY).min(1.0)));
        let n = t.ceil() as u32;
        // Below the banner line (at 0.38), so a banner and the countdown never overlap.
        let k = t - t.floor();
        self.text_c("The vigil resumes", sw * 0.5, sh * 0.52, 24.0, PALE);
        // Fixed font size: changing it every frame would rebuild the glyph cache constantly.
        self.text_c(&n.to_string(), sw * 0.5, sh * 0.52 + 76.0 + (1.0 - k) * 8.0, 72.0, a(GOLD, 0.4 + 0.6 * k));
    }

    pub fn paused(&self, online: bool) {
        let (sw, sh) = (ui_w(), ui_h());
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.0, 0.6));
        self.spaced_c(if online { "MENU" } else { "PAUSED" }, sw * 0.5, sh * 0.4, 40.0, 6.0, PALE);
        if online {
            self.text_c("The world does not wait for you online.", sw * 0.5, sh * 0.4 + 34.0, 18.0, EMBER);
        }
        self.text_c(hint("Esc: resume     Q: leave the vigil", "Start or B: resume     Select: leave the vigil"), sw * 0.5, sh * 0.4 + 62.0, 18.0, DIM);
        let keys = if using_pad() {
            "Y: reroll     LB (hold): damage per weapon".to_string()
        } else {
            format!("{}: mute     {}: damage numbers     F3: performance overlay     F11: fullscreen", key_label(Action::Mute), key_label(Action::Numbers))
        };
        self.text_c(&keys, sw * 0.5, sh * 0.4 + 88.0, 16.0, DIM);
    }

    /// Who dealt how much: this vigil and the last minute. Tab shows our weapons.
    pub fn damage_meter(&self, w: &World, detailed: bool) {
        let sw = ui_w();
        let x = sw - 250.0;
        let mut y = 84.0;
        let me = w.me();
        let rows: Vec<(String, f64, f32, Color)> = if w.heroes.len() > 1 && !detailed {
            let mut v: Vec<(String, f64, f32, Color)> = w.heroes.iter().map(|h| (h.name.clone(), h.dmg_total, h.dmg_minute, slot_color(h.slot))).collect();
            v.sort_by(|a, b| b.1.total_cmp(&a.1));
            v
        } else {
            let mut v: Vec<(String, f64, f32, Color)> = ALL_WEAPONS
                .iter()
                .filter(|k| me.dmg_weapon[k.idx()] > 0.0)
                .map(|k| {
                    let name = me.weapons.iter().find(|wp| wp.has(*k)).map(|wp| if wp.is_fused() { k.name(true) } else { wp.display_name() }).unwrap_or(k.name(false));
                    (name.to_string(), me.dmg_weapon[k.idx()], f32::NAN, k.color())
                })
                .collect();
            v.sort_by(|a, b| b.1.total_cmp(&a.1));
            v.truncate(8);
            v
        };
        if rows.is_empty() {
            return;
        }
        let h = 26.0 + rows.len() as f32 * 19.0;
        draw_rectangle(x - 8.0, y - 4.0, 240.0, h, Color::new(0.0, 0.0, 0.0, 0.45));
        let weapons_key = if using_pad() { "LB" } else { key_label(Action::Weapons) };
        let title = if w.heroes.len() > 1 && !detailed { "Damage   vigil / last min".to_string() } else { format!("Damage by weapon   ({})", weapons_key) };
        mono(title, x, y + 10.0, 15.0, a(DIM, 1.0));
        y += 16.0;
        let max = rows.iter().map(|r| r.1).fold(1.0, f64::max);
        for (name, total, minute, c) in rows {
            let frac = (total / max) as f32;
            draw_rectangle(x, y + 2.0, 224.0 * frac, 15.0, Color::new(c.r, c.g, c.b, 0.35));
            mono(&name, x + 3.0, y + 14.0, 15.0, PALE);
            let value = if minute.is_nan() { short(total) } else { format!("{} / {}", short(total), short(minute as f64)) };
            let tw = mono_width(&value, 15.0);
            mono(&value, x + 224.0 - tw, y + 14.0, 15.0, PALE);
            y += 19.0;
        }
        if w.heroes.len() > 1 && !detailed {
            let total: f64 = w.heroes.iter().map(|h| h.dmg_total).sum();
            let share = if total > 0.0 { me.dmg_total / total * 100.0 } else { 0.0 };
            mono(format!("Your share {:.0}%   {}: your weapons", share, weapons_key), x, y + 12.0, 13.0, DIM);
        }
    }

    /// FPS, ping and traffic in the bottom-left corner.
    pub fn perf_overlay(&self, net: Option<&NetStats>) {
        let sh = ui_h();
        let lines: Vec<String> = match net {
            Some(n) => vec![
                format!("FPS {}", get_fps()),
                format!("Ping {:.0} ms", n.ping_ms),
                format!("Sent {:.1} KB/s  {:.0} pkt/s  ({:.0} KB)", n.sent_rate / 1024.0, n.pkts_sent_rate, n.sent_total as f32 / 1024.0),
                format!("Recv {:.1} KB/s  {:.0} pkt/s  ({:.0} KB)", n.recv_rate / 1024.0, n.pkts_recv_rate, n.recv_total as f32 / 1024.0),
            ],
            None => vec![format!("FPS {}", get_fps()), "Ping -".into(), "Sent -".into(), "Recv -".into()],
        };
        let h = lines.len() as f32 * 15.0 + 8.0;
        let w = lines.iter().map(|l| mono_width(l, 14.0)).fold(0.0, f32::max) + 14.0;
        draw_rectangle(8.0, sh - h - 8.0, w, h, Color::new(0.0, 0.0, 0.0, 0.55));
        for (i, l) in lines.iter().enumerate() {
            // The default font keeps the digits narrow and steady.
            mono(l, 15.0, sh - h + 6.0 + i as f32 * 15.0, 16.0, Color::new(0.75, 0.9, 0.75, 0.95));
        }
    }
}
