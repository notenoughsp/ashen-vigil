//! The cursed shrine and the wandering merchant.

use super::*;

impl World {
    pub(super) fn draw_events(&self) {
        let t = self.time;
        self.draw_lost_grave(t);
        self.draw_altars(t);
        self.draw_hourglass(t);
        self.draw_wager_stone(t);
        self.draw_vault(t);
        if let Some(c) = &self.cursed {
            let p = c.pos;
            let active = c.curse > 0.0;
            let pulse = 0.6 + 0.4 * (t * if active { 6.0 } else { 2.0 }).sin();
            draw_circle(p.x, p.y, 70.0, Color::new(0.4, 0.1, 0.55, 0.08 * pulse));
            draw_ellipse(p.x, p.y + 14.0, 26.0, 8.0, 0.0, rgba(0, 0, 0, 140));
            // A cracked black obelisk wrapped in chains, runes glowing.
            draw_triangle(vec2(p.x - 14.0, p.y + 12.0), vec2(p.x + 14.0, p.y + 12.0), vec2(p.x + 3.0, p.y - 58.0), rgba(4, 3, 6, 255));
            draw_triangle(vec2(p.x - 12.0, p.y + 10.0), vec2(p.x + 12.0, p.y + 10.0), vec2(p.x + 3.0, p.y - 54.0), rgba(26, 22, 32, 255));
            draw_triangle(vec2(p.x - 12.0, p.y + 10.0), vec2(p.x - 2.0, p.y + 10.0), vec2(p.x + 3.0, p.y - 54.0), rgba(38, 32, 46, 255));
            let rune = Color::new(0.75, 0.35, 1.0, 0.5 + 0.5 * pulse);
            for k in 0..4 {
                let y = p.y - 6.0 - k as f32 * 11.0;
                draw_line(p.x - 3.0, y, p.x + 4.0, y - 4.0, 1.5, rune);
                draw_line(p.x + 4.0, y - 4.0, p.x + 1.0, y - 7.0, 1.5, rune);
            }
            for (a, b) in [(vec2(-12.0, -10.0), vec2(14.0, -24.0)), (vec2(-10.0, -30.0), vec2(12.0, -16.0))] {
                let n = 7;
                for k in 0..=n {
                    let q = p + a.lerp(b, k as f32 / n as f32);
                    draw_circle_lines(q.x, q.y, 2.0, 1.0, rgba(90, 86, 96, 255));
                }
            }
            if !c.accepted && c.kindle > 0.0 {
                draw_arc(p.x, p.y, 40, 46.0, -90.0, 3.0, 360.0 * (c.kindle / 1.5), Color::new(0.75, 0.35, 1.0, 1.0));
            }
            if active {
                draw_circle_lines(p.x, p.y, 60.0 + pulse * 6.0, 2.0, rune);
            }
        }
        if let Some(m) = &self.merchant {
            let p = m.pos;
            draw_ellipse(p.x, p.y + 14.0, 22.0, 6.0, 0.0, rgba(0, 0, 0, 130));
            // A hunched peddler under a huge pack, a lantern swinging on a pole.
            draw_ellipse(p.x - 8.0, p.y - 12.0, 16.0, 18.0, 0.0, rgba(4, 3, 4, 255));
            draw_ellipse(p.x - 8.0, p.y - 12.0, 14.5, 16.5, 0.0, rgba(86, 64, 40, 255));
            for k in 0..3 {
                let y = p.y - 22.0 + k as f32 * 9.0;
                draw_line(p.x - 20.0, y, p.x + 4.0, y + 2.0, 1.5, rgba(60, 42, 26, 255));
            }
            draw_circle(p.x - 14.0, p.y - 28.0, 4.0, rgba(120, 110, 90, 255));
            draw_triangle(vec2(p.x - 6.0, p.y + 12.0), vec2(p.x + 14.0, p.y + 12.0), vec2(p.x + 6.0, p.y - 16.0), rgba(52, 44, 40, 255));
            draw_circle(p.x + 7.0, p.y - 14.0, 6.5, rgba(52, 44, 40, 255));
            draw_circle(p.x + 8.5, p.y - 13.0, 4.0, rgba(8, 6, 6, 255));
            draw_circle(p.x + 9.0, p.y - 13.5, 1.0, rgba(255, 200, 120, 255));
            draw_circle(p.x + 11.5, p.y - 13.5, 1.0, rgba(255, 200, 120, 255));
            let pole_top = vec2(p.x + 22.0, p.y - 34.0);
            draw_line(p.x + 12.0, p.y + 8.0, pole_top.x, pole_top.y, 2.0, rgba(70, 50, 30, 255));
            let lamp = pole_top + vec2((t * 2.0).sin() * 3.0, 9.0);
            draw_line(pole_top.x, pole_top.y, lamp.x, lamp.y - 3.0, 1.0, rgba(90, 90, 96, 255));
            draw_rectangle(lamp.x - 3.0, lamp.y - 3.0, 6.0, 7.0, rgba(50, 50, 54, 255));
            draw_rectangle(lamp.x - 2.0, lamp.y - 2.0, 4.0, 5.0, rgba(255, 190, 90, 255));
            let name = MERCHANT_NAME;
            let nw = measure_text(name, None, 15, 1.0).width;
            draw_text(name, p.x - nw * 0.5 + 1.0, p.y - 45.0, 15.0, Color::new(0.0, 0.0, 0.0, 0.8));
            draw_text(name, p.x - nw * 0.5, p.y - 46.0, 15.0, rgba(230, 200, 140, 255));
            // The wares on stone pedestals with their prices.
            for (i, ware) in m.wares.iter().enumerate() {
                let q = m.pedestal(i);
                draw_ellipse(q.x, q.y + 6.0, 13.0, 4.0, 0.0, rgba(0, 0, 0, 120));
                draw_rectangle(q.x - 10.0, q.y - 4.0, 20.0, 10.0, rgba(48, 46, 50, 255));
                draw_rectangle(q.x - 10.0, q.y - 4.0, 20.0, 3.0, rgba(70, 68, 74, 255));
                if m.sold[i] {
                    draw_text("SOLD", q.x - 14.0, q.y - 10.0, 14.0, rgba(120, 110, 100, 200));
                    continue;
                }
                let bob = (t * 2.5 + i as f32).sin() * 2.0;
                let item = q + vec2(0.0, -12.0 + bob);
                match ware {
                    Ware::Draught => {
                        draw_circle(item.x, item.y + 2.0, 5.0, rgba(230, 120, 40, 255));
                        draw_rectangle(item.x - 1.5, item.y - 6.0, 3.0, 5.0, rgba(180, 170, 150, 255));
                    }
                    Ware::Relic => {
                        draw_rectangle(item.x - 5.0, item.y - 6.0, 10.0, 12.0, rgba(50, 40, 60, 255));
                        draw_rectangle_lines(item.x - 5.0, item.y - 6.0, 10.0, 12.0, 1.5, rgba(190, 150, 255, 255));
                    }
                    Ware::Coffer => {
                        draw_rectangle(item.x - 7.0, item.y - 5.0, 14.0, 10.0, rgba(100, 66, 38, 255));
                        draw_rectangle_lines(item.x - 7.0, item.y - 5.0, 14.0, 10.0, 1.5, rgba(220, 180, 90, 255));
                    }
                    Ware::Dice => {
                        for (dx, dy) in [(-4.0, 1.0), (4.0, -2.0)] {
                            draw_rectangle(item.x + dx - 4.0, item.y + dy - 4.0, 8.0, 8.0, rgba(225, 215, 190, 255));
                            draw_circle(item.x + dx, item.y + dy, 1.2, rgba(40, 20, 20, 255));
                        }
                    }
                    Ware::Rage => {
                        draw_circle(item.x, item.y + 2.0, 5.0, rgba(200, 30, 30, 255));
                        draw_rectangle(item.x - 1.5, item.y - 6.0, 3.0, 5.0, rgba(180, 170, 150, 255));
                    }
                    Ware::Wings => {
                        draw_triangle(item, item + vec2(-9.0, -5.0), item + vec2(-3.0, 4.0), rgba(150, 230, 170, 255));
                        draw_triangle(item, item + vec2(9.0, -5.0), item + vec2(3.0, 4.0), rgba(150, 230, 170, 255));
                    }
                    Ware::Ward => {
                        draw_circle(item.x, item.y, 6.0, rgba(90, 140, 230, 255));
                        draw_circle_lines(item.x, item.y, 6.0, 1.5, rgba(200, 225, 255, 255));
                    }
                    Ware::Lodestone => {
                        draw_arc(item.x, item.y, 12, 4.0, 0.0, 3.0, 180.0, rgba(170, 60, 60, 255));
                        draw_rectangle(item.x - 5.5, item.y - 5.0, 3.0, 4.0, rgba(200, 200, 210, 255));
                        draw_rectangle(item.x + 2.5, item.y - 5.0, 3.0, 4.0, rgba(200, 200, 210, 255));
                    }
                }
                let price = format!("{}", m.prices[i]);
                let w = measure_text(&price, None, 14, 1.0).width;
                draw_text(&price, q.x - w * 0.5 + 1.0, q.y + 22.0, 14.0, Color::new(0.0, 0.0, 0.0, 0.8));
                draw_text(&price, q.x - w * 0.5, q.y + 21.0, 14.0, rgba(170, 210, 255, 255));
            }
        }
    }

    /// Blood altars: a slab with a bowl of blood (their words are in `draw_omen_labels`).
    fn draw_altars(&self, t: f32) {
        for a in &self.altars {
            let p = a.pos;
            let pulse = 0.6 + 0.4 * (t * 3.0).sin();
            draw_circle(p.x, p.y, 46.0, Color::new(0.7, 0.05, 0.05, 0.12 + 0.06 * pulse));
            draw_circle_lines(p.x, p.y, 34.0, 2.0, Color::new(0.85, 0.1, 0.1, 0.5 + 0.3 * pulse));
            // Runes around the slab.
            for k in 0..6 {
                let q = p + Vec2::from_angle(k as f32 / 6.0 * TAU + t * 0.3) * 40.0;
                draw_poly_lines(q.x, q.y, 3, 4.0, k as f32 * 60.0, 1.5, Color::new(0.9, 0.15, 0.12, 0.7));
            }
            draw_ellipse(p.x, p.y + 10.0, 22.0, 6.0, 0.0, rgba(0, 0, 0, 140));
            draw_rectangle(p.x - 20.0, p.y - 8.0, 40.0, 16.0, rgba(40, 34, 36, 255));
            draw_rectangle(p.x - 20.0, p.y - 8.0, 40.0, 4.0, rgba(64, 56, 58, 255));
            draw_ellipse(p.x, p.y - 10.0, 11.0, 4.0, 0.0, rgba(30, 26, 28, 255));
            draw_ellipse(p.x, p.y - 10.5, 9.0, 3.0, 0.0, rgba(150, 10, 14, 255));
            // Drips down the front.
            for k in 0..3 {
                let x = p.x - 10.0 + k as f32 * 10.0;
                let len = 4.0 + ((t * 0.7 + k as f32) % 1.0) * 8.0;
                draw_line(x, p.y - 4.0, x, p.y - 4.0 + len, 1.5, rgba(150, 10, 14, 230));
            }
            // How far the pact has been made.
            if a.kindle > 0.0 {
                let k = (a.kindle / ALTAR_KINDLE).min(1.0);
                draw_arc(p.x, p.y, 40, 52.0, -90.0, 4.0, 360.0 * k, Color::new(1.0, 0.15, 0.1, 0.9));
            }
        }
    }

    /// The wager stone: an obelisk and its three tiles (their words are in `draw_omen_labels`).
    fn draw_wager_stone(&self, t: f32) {
        let Some(s) = &self.stone else { return };
        let p = s.pos;
        draw_ellipse(p.x, p.y + 8.0, 20.0, 6.0, 0.0, rgba(0, 0, 0, 140));
        draw_triangle(vec2(p.x - 14.0, p.y + 6.0), vec2(p.x + 14.0, p.y + 6.0), vec2(p.x, p.y - 70.0), rgba(56, 54, 62, 255));
        draw_triangle(vec2(p.x, p.y + 6.0), vec2(p.x + 14.0, p.y + 6.0), vec2(p.x, p.y - 70.0), rgba(40, 38, 46, 255));
        let glow = 0.6 + 0.4 * (t * 2.5).sin();
        draw_circle(p.x, p.y - 30.0, 5.0, Color::new(0.55, 0.85, 1.0, glow));
        let mine = self.me().wager;
        for i in 0..WAGERS.len() {
            let q = s.tile(i);
            let chosen = mine.is_some_and(|w| w.tier == i);
            let c = if chosen { Color::new(0.4, 1.0, 0.5, 1.0) } else { Color::new(0.55, 0.85, 1.0, 1.0) };
            draw_circle(q.x, q.y, 26.0, with_alpha(c, 0.12 + 0.05 * glow));
            draw_circle_lines(q.x, q.y, 26.0, 2.0, with_alpha(c, 0.8));
            // Standing on a tile fills it, from the middle out and round the rim, until the
            // bet is placed.
            let me = self.me();
            if me.wager.is_none() && me.wager_tile == i && me.wager_kindle > 0.0 {
                let k = (me.wager_kindle / WAGER_KINDLE).clamp(0.0, 1.0);
                draw_circle(q.x, q.y, 26.0 * k, with_alpha(c, 0.35 + 0.25 * k));
                draw_arc(q.x, q.y, 40, 27.0, -90.0, 4.0, 360.0 * k, with_alpha(c, 1.0));
            }
        }
    }

    /// The sealed vault: a stone door with a golden lock that the courier's key opens.
    fn draw_vault(&self, t: f32) {
        let Some(v) = &self.vault else { return };
        let p = v.pos;
        draw_ellipse(p.x, p.y + 22.0, 40.0, 9.0, 0.0, rgba(0, 0, 0, 150));
        draw_rectangle(p.x - 36.0, p.y - 50.0, 72.0, 72.0, rgba(44, 42, 48, 255));
        draw_rectangle(p.x - 36.0, p.y - 50.0, 72.0, 8.0, rgba(70, 68, 76, 255));
        draw_rectangle(p.x - 24.0, p.y - 36.0, 48.0, 58.0, if v.open { rgba(8, 6, 8, 255) } else { rgba(76, 60, 40, 255) });
        if !v.open {
            for x in [-12.0, 0.0, 12.0] {
                draw_line(p.x + x, p.y - 36.0, p.x + x, p.y + 22.0, 2.0, rgba(50, 40, 28, 255));
            }
            let glow = if self.vault_key { 0.6 + 0.4 * (t * 4.0).sin() } else { 0.3 };
            draw_circle(p.x, p.y - 6.0, 14.0, Color::new(1.0, 0.8, 0.3, 0.15 * glow));
            draw_circle(p.x, p.y - 6.0, 6.0, rgba(220, 180, 80, 255));
            draw_rectangle(p.x - 1.5, p.y - 6.0, 3.0, 7.0, rgba(40, 26, 10, 255));
        }
    }

    /// The hourglass shrine: a tall glass of pale sand between two stone plates.
    fn draw_hourglass(&self, t: f32) {
        let Some(g) = &self.hourglass else { return };
        let p = g.pos;
        let glow = 0.6 + 0.4 * (t * 2.0).sin();
        draw_circle(p.x, p.y, 44.0, Color::new(0.55, 0.75, 1.0, 0.1 + 0.05 * glow));
        draw_circle_lines(p.x, p.y, 36.0, 2.0, Color::new(0.65, 0.82, 1.0, 0.6));
        draw_ellipse(p.x, p.y + 10.0, 20.0, 6.0, 0.0, rgba(0, 0, 0, 140));
        draw_rectangle(p.x - 18.0, p.y + 2.0, 36.0, 7.0, rgba(60, 58, 66, 255));
        draw_rectangle(p.x - 18.0, p.y - 50.0, 36.0, 7.0, rgba(60, 58, 66, 255));
        let glass = Color::new(0.75, 0.85, 1.0, 0.35);
        draw_triangle(vec2(p.x - 13.0, p.y - 43.0), vec2(p.x + 13.0, p.y - 43.0), vec2(p.x, p.y - 21.0), glass);
        draw_triangle(vec2(p.x - 13.0, p.y + 2.0), vec2(p.x + 13.0, p.y + 2.0), vec2(p.x, p.y - 21.0), glass);
        // Sand: draining from the top into the bottom as the shrine is held.
        let k = (g.kindle / HOURGLASS_KINDLE).clamp(0.0, 1.0);
        let sand = rgba(235, 215, 160, 255);
        let top = 1.0 - k * 0.8;
        draw_triangle(vec2(p.x - 11.0 * top, p.y - 21.0 - 20.0 * top), vec2(p.x + 11.0 * top, p.y - 21.0 - 20.0 * top), vec2(p.x, p.y - 22.0), sand);
        let bottom = 0.2 + k * 0.8;
        draw_triangle(vec2(p.x - 12.0, p.y + 1.0), vec2(p.x + 12.0, p.y + 1.0), vec2(p.x, p.y + 1.0 - 18.0 * bottom), sand);
        draw_line(p.x, p.y - 21.0, p.x, p.y, 1.0, with_alpha(sand, 0.8));
        if k > 0.0 {
            draw_arc(p.x, p.y, 40, 50.0, -90.0, 4.0, 360.0 * k, Color::new(0.7, 0.85, 1.0, 0.9));
        }
    }

    /// The words on altars, the wager stone, the vault and cursed coffers, drawn after lighting
    /// and large, so they can always be read.
    pub(super) fn draw_omen_labels(&self) {
        let say = |lines: &[(&str, f32, Color)], x: f32, mut y: f32| {
            for (text, size, c) in lines {
                if text.is_empty() {
                    continue;
                }
                let w = measure_text(text, None, *size as u16, 1.0).width;
                draw_rectangle(x - w * 0.5 - 4.0, y - size * 0.8, w + 8.0, size + 2.0, Color::new(0.0, 0.0, 0.0, 0.55));
                draw_text(text, x - w * 0.5 + 1.0, y + 1.0, *size, Color::new(0.0, 0.0, 0.0, 0.9));
                draw_text(text, x - w * 0.5, y, *size, *c);
                y += size + 4.0;
            }
        };
        for a in &self.altars {
            let pale = rgba(245, 215, 205, 255);
            say(
                &[("BLOOD ALTAR", 24.0, rgba(240, 60, 50, 255)), ("Stand here: LOSE 25% LIFE", 18.0, pale), ("gain +35% might until the next lord falls", 18.0, pale)],
                a.pos.x,
                a.pos.y - 96.0,
            );
        }
        if let Some(s) = &self.stone {
            let pale = rgba(220, 232, 245, 255);
            say(
                &[
                    ("WAGER STONE", 24.0, rgba(140, 210, 255, 255)),
                    ("Stand on a tile: bet a third of your souls", 18.0, pale),
                    ("(a capped stake) on how fast the next lord falls", 18.0, pale),
                ],
                s.pos.x,
                s.pos.y - 130.0,
            );
            let stake = self.me().p.souls / 3;
            let mine = self.me().wager;
            for (i, (limit, mult)) in WAGERS.iter().enumerate() {
                let q = s.tile(i);
                let chosen = mine.is_some_and(|w| w.tier == i);
                let c = if chosen { Color::new(0.4, 1.0, 0.5, 1.0) } else { Color::new(0.6, 0.88, 1.0, 1.0) };
                let top = format!("{:.0} s  x{}", limit, mult);
                let cost = if chosen {
                    "your bet".to_string()
                } else if mine.is_some() {
                    String::new()
                } else {
                    format!("costs {}", stake)
                };
                say(&[(&top, 18.0, c), (&cost, 15.0, pale)], q.x, q.y + 44.0);
            }
        }
        if let Some(g) = &self.hourglass {
            let pale = rgba(225, 235, 250, 255);
            say(&[("HOURGLASS SHRINE", 24.0, rgba(170, 210, 255, 255)), ("Stand here: one to five minutes of the night pass at once", 18.0, pale)], g.pos.x, g.pos.y - 80.0);
        }
        if let Some(v) = &self.vault {
            let label = if v.open {
                "THE VAULT (OPEN)"
            } else if self.vault_key {
                "THE VAULT: bring the key here"
            } else {
                "THE VAULT: sealed"
            };
            say(&[(label, 20.0, rgba(245, 205, 95, 255))], v.pos.x, v.pos.y - 64.0);
        }
        for e in self.enemies.iter().filter(|e| e.kind == EnemyKind::Rival && !e.dead) {
            say(&[("SVEN", 18.0, rgba(240, 80, 70, 255))], e.pos.x, e.pos.y - e.r * 2.6);
        }
        for p in self.pickups.iter().filter(|p| matches!(p.kind, PickKind::CursedCoffer)) {
            say(&[("CURSED COFFER", 16.0, Color::new(0.85, 0.55, 1.0, 1.0))], p.pos.x, p.pos.y - 26.0);
        }
    }

    /// A weathered headstone with a candle that is always lit, far from everything.
    fn draw_lost_grave(&self, t: f32) {
        let p = LOST_GRAVE;
        if !self.on_screen(p, 120.0) {
            return;
        }
        draw_ellipse(p.x, p.y + 12.0, 22.0, 7.0, 0.0, rgba(0, 0, 0, 130));
        draw_rectangle(p.x - 12.0, p.y - 26.0, 24.0, 38.0, rgba(20, 20, 24, 255));
        draw_circle(p.x, p.y - 26.0, 12.0, rgba(20, 20, 24, 255));
        draw_rectangle(p.x - 10.0, p.y - 24.0, 20.0, 34.0, rgba(58, 58, 66, 255));
        draw_circle(p.x, p.y - 24.0, 10.0, rgba(58, 58, 66, 255));
        draw_line(p.x - 5.0, p.y - 18.0, p.x + 5.0, p.y - 18.0, 1.5, rgba(30, 30, 36, 255));
        draw_line(p.x, p.y - 23.0, p.x, p.y - 8.0, 1.5, rgba(30, 30, 36, 255));
        let flame = 0.8 + 0.2 * (t * 9.0).sin();
        let c = vec2(p.x + 17.0, p.y + 6.0);
        draw_rectangle(c.x - 2.0, c.y - 8.0, 4.0, 8.0, rgba(220, 210, 190, 255));
        draw_circle(c.x, c.y - 11.0, 6.0 * flame, Color::new(1.0, 0.6, 0.2, 0.25));
        draw_circle(c.x, c.y - 11.0, 2.2 * flame, Color::new(1.0, 0.85, 0.5, 1.0));
    }
}
