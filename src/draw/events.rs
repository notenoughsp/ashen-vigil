//! The cursed shrine and the wandering merchant.

use super::*;

impl World {
    pub(super) fn draw_events(&self) {
        let t = self.time;
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
            // The wares on stone pedestals with their prices.
            for (i, ware) in WARES.iter().enumerate() {
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
                }
                let price = format!("{}", m.prices[i]);
                let w = measure_text(&price, None, 14, 1.0).width;
                draw_text(&price, q.x - w * 0.5 + 1.0, q.y + 22.0, 14.0, Color::new(0.0, 0.0, 0.0, 0.8));
                draw_text(&price, q.x - w * 0.5, q.y + 21.0, 14.0, rgba(170, 210, 255, 255));
            }
        }
    }
}
