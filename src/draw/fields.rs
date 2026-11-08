//! How the three newer fields look: their floors, props and hazards, their foes and lords.

use super::*;
use crate::world::{hazard_patch, EMBER_SALT, EMBER_SHARE, HAZARD_CELL, ICE_SALT, ICE_SHARE};

fn h2(x: i32, y: i32, salt: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(0x27D4_EB2D) ^ (y as u32).wrapping_mul(0x1656_67B1) ^ salt.wrapping_mul(0x9E37_79B9);
    h ^= h >> 15;
    h = h.wrapping_mul(0x85EB_CA6B);
    h ^= h >> 13;
    (h % 10_000) as f32 / 10_000.0
}

impl World {
    /// A floor tile of the Ashwood, the Frozen Abbey or the Starless Garden.
    pub(super) fn field_tile(&self, tx: i32, ty: i32, px: f32, py: f32, tile: f32) {
        let v = h2(tx, ty, 1);
        match self.biome {
            Biome::Ashwood => {
                // Grey ash over black soil, soot specks and charred roots.
                let g = 30.0 + v * 10.0;
                draw_rectangle(px, py, tile, tile, Color::new(g / 255.0, (g - 3.0) / 255.0, (g - 5.0) / 255.0, 1.0));
                for k in 0..6 {
                    let at = vec2(px + h2(tx, ty, 10 + k) * tile, py + h2(tx, ty, 20 + k) * tile);
                    draw_circle(at.x, at.y, 1.5 + h2(tx, ty, 30 + k) * 3.0, Color::new(0.09, 0.08, 0.075, 0.9));
                }
                for k in 0..3 {
                    let at = vec2(px + h2(tx, ty, 40 + k) * tile, py + h2(tx, ty, 50 + k) * tile);
                    draw_circle(at.x, at.y, 6.0 + h2(tx, ty, 60 + k) * 10.0, Color::new(0.22, 0.21, 0.2, 0.5));
                }
                if h2(tx, ty, 70) < 0.3 {
                    let mut q = vec2(px + h2(tx, ty, 71) * tile, py + h2(tx, ty, 72) * tile);
                    for s in 0..4 {
                        let n = q + Vec2::from_angle(h2(tx, ty, 73 + s) * TAU) * 12.0;
                        draw_line(q.x, q.y, n.x, n.y, 2.0, Color::new(0.06, 0.04, 0.03, 1.0));
                        q = n;
                    }
                }
            }
            Biome::Abbey => {
                // Pale flagstones under snow, frost in the joints.
                let b = 150.0 + v * 20.0;
                let c = Color::new((b - 10.0) / 255.0, (b - 2.0) / 255.0, (b + 12.0) / 255.0, 1.0);
                // Frosted grout under the slab.
                draw_rectangle(px, py, tile, tile, Color::new(0.5, 0.56, 0.64, 1.0));
                draw_rectangle(px + 1.5, py + 1.5, tile - 3.0, tile - 3.0, c);
                draw_rectangle(px + 1.5, py + 1.5, tile - 3.0, 2.0, Color::new(0.9, 0.93, 1.0, 1.0));
                draw_rectangle(px + 1.5, py + tile - 3.5, tile - 3.0, 2.0, Color::new(0.45, 0.5, 0.6, 1.0));
                // Snow drifted against one side.
                if h2(tx, ty, 80) < 0.3 {
                    let at = vec2(px + h2(tx, ty, 81) * tile, py + h2(tx, ty, 82) * tile);
                    draw_ellipse(at.x, at.y, 20.0 + v * 18.0, 9.0 + v * 6.0, 0.0, Color::new(0.94, 0.96, 1.0, 0.9));
                    draw_ellipse(at.x - 4.0, at.y - 2.0, 12.0, 4.0, 0.0, WHITE);
                }
            }
            _ => {
                // A dark lawn, gravel paths in a grid between the hedges.
                let path = tx.rem_euclid(9) == 0 || ty.rem_euclid(9) == 0;
                if path {
                    let g = 54.0 + v * 12.0;
                    draw_rectangle(px, py, tile, tile, Color::new(g / 255.0, (g - 4.0) / 255.0, (g - 8.0) / 255.0, 1.0));
                    for k in 0..14 {
                        let at = vec2(px + h2(tx, ty, 90 + k) * tile, py + h2(tx, ty, 110 + k) * tile);
                        draw_circle(at.x, at.y, 1.2 + h2(tx, ty, 130 + k) * 1.5, Color::new(0.32, 0.31, 0.3, 1.0));
                    }
                } else {
                    let g = 22.0 + v * 9.0;
                    draw_rectangle(px, py, tile, tile, Color::new((g * 0.6) / 255.0, g / 255.0, (g * 0.7) / 255.0, 1.0));
                    for k in 0..10 {
                        let at = vec2(px + h2(tx, ty, 140 + k) * tile, py + h2(tx, ty, 160 + k) * tile);
                        draw_line(at.x, at.y, at.x + (h2(tx, ty, 180 + k) - 0.5) * 4.0, at.y - 5.0, 1.0, Color::new(0.1, 0.2, 0.11, 1.0));
                    }
                }
            }
        }
    }

    /// The tint of the patches of earth between the stones, and of what grows there.
    pub(super) fn field_earth(&self) -> (Color, Color) {
        match self.biome {
            Biome::Ashwood => (Color::new(0.05, 0.04, 0.035, 0.7), Color::new(0.75, 0.3, 0.08, 0.6)),
            Biome::Abbey => (Color::new(0.93, 0.95, 1.0, 0.55), Color::new(0.0, 0.0, 0.0, 0.0)),
            _ => (Color::new(0.06, 0.08, 0.05, 0.7), Color::new(0.12, 0.26, 0.12, 0.95)),
        }
    }

    /// The haze drifting over the field.
    pub(super) fn field_fog(&self) -> Color {
        match self.biome {
            Biome::Ashwood => Color::new(0.45, 0.32, 0.25, 0.03),
            Biome::Abbey => Color::new(0.92, 0.95, 1.0, 0.035),
            _ => Color::new(0.35, 0.25, 0.5, 0.03),
        }
    }

    /// Embers and ice lie on the ground where the field's hazards are.
    pub(super) fn draw_hazards(&self) {
        let (salt, share) = match self.biome {
            Biome::Ashwood => (EMBER_SALT, EMBER_SHARE),
            Biome::Abbey => (ICE_SALT, ICE_SHARE),
            _ => return,
        };
        let half = super::view_size() * 0.5 + vec2(HAZARD_CELL, HAZARD_CELL);
        let (x0, x1) = (((self.cam.x - half.x) / HAZARD_CELL).floor() as i32, ((self.cam.x + half.x) / HAZARD_CELL).ceil() as i32);
        let (y0, y1) = (((self.cam.y - half.y) / HAZARD_CELL).floor() as i32, ((self.cam.y + half.y) / HAZARD_CELL).ceil() as i32);
        let t = self.time;
        for cy in y0..y1 {
            for cx in x0..x1 {
                let Some((c, r)) = hazard_patch(cx, cy, salt, share) else { continue };
                if !inside_world(c, 0.0) {
                    continue;
                }
                if self.biome == Biome::Ashwood {
                    let flick = 0.75 + 0.25 * (t * 3.0 + c.x * 0.01).sin();
                    draw_ellipse(c.x, c.y, r.x, r.y, 0.0, Color::new(0.16, 0.05, 0.02, 0.85));
                    draw_ellipse(c.x, c.y, r.x * 0.85, r.y * 0.8, 0.0, Color::new(0.75, 0.22, 0.04, 0.35 * flick));
                    for k in 0..12 {
                        let a = h2(cx, cy, 200 + k) * TAU;
                        let d = h2(cx, cy, 220 + k).sqrt() * 0.85;
                        let q = c + vec2(a.cos() * r.x * d, a.sin() * r.y * d);
                        let glow = 0.5 + 0.5 * (t * (2.0 + h2(cx, cy, 240 + k) * 3.0) + k as f32).sin();
                        draw_circle(q.x, q.y, 2.5 + glow * 2.0, Color::new(1.0, 0.45 + 0.3 * glow, 0.1, 0.7 + 0.3 * glow));
                    }
                } else {
                    draw_ellipse(c.x, c.y, r.x, r.y, 0.0, Color::new(0.62, 0.8, 0.95, 0.55));
                    draw_ellipse(c.x - r.x * 0.2, c.y - r.y * 0.25, r.x * 0.5, r.y * 0.3, 0.0, Color::new(0.92, 0.97, 1.0, 0.45));
                    draw_ellipse_lines(c.x, c.y, r.x, r.y, 0.0, 1.5, Color::new(0.85, 0.93, 1.0, 0.7));
                    for k in 0..3 {
                        let a = h2(cx, cy, 300 + k) * TAU;
                        let q = c + vec2(a.cos() * r.x * 0.5, a.sin() * r.y * 0.5);
                        draw_line(q.x - 10.0, q.y - 3.0, q.x + 12.0, q.y + 4.0, 1.0, Color::new(1.0, 1.0, 1.0, 0.6));
                    }
                }
            }
        }
    }

    /// The newer fields' scenery, by the same roll as every other field's.
    pub(super) fn new_field_prop(&self, d: u32, tx: i32, ty: i32, ox: f32, oy: f32) {
        let t = self.time;
        let shadow = |w: f32| draw_ellipse(ox, oy + 6.0, w, w * 0.3, 0.0, rgba(0, 0, 0, 90));
        match (self.biome, d) {
            (Biome::Ashwood, 0..=30) => {
                // A burnt tree, a few embers still in its bark.
                shadow(22.0);
                draw_rectangle(ox - 5.0, oy - 70.0, 10.0, 74.0, rgba(24, 20, 18, 255));
                for (a, l) in [(-2.3, 26.0), (-0.9, 30.0), (-1.7, 22.0)] {
                    let b = vec2(ox, oy - 50.0 - l * 0.3) + Vec2::from_angle(a) * l;
                    draw_line(ox, oy - 50.0 - l * 0.3, b.x, b.y, 3.0, rgba(28, 22, 20, 255));
                }
                let g = 0.5 + 0.5 * (t * 2.0 + ox).sin();
                draw_circle(ox + 2.0, oy - 30.0, 2.0, Color::new(1.0, 0.45, 0.1, 0.5 + 0.5 * g));
            }
            (Biome::Ashwood, 31..=55) => {
                shadow(16.0);
                draw_ellipse(ox, oy, 14.0, 7.0, 0.0, rgba(34, 28, 24, 255));
                draw_ellipse(ox, oy - 3.0, 12.0, 5.0, 0.0, rgba(60, 48, 38, 255));
                draw_ellipse_lines(ox, oy - 3.0, 7.0, 3.0, 0.0, 1.0, rgba(30, 24, 20, 255));
            }
            (Biome::Ashwood, 56..=70) => {
                // A smouldering log.
                shadow(26.0);
                draw_rectangle(ox - 24.0, oy - 6.0, 48.0, 12.0, rgba(30, 24, 20, 255));
                draw_circle(ox + 24.0, oy, 6.0, rgba(50, 38, 30, 255));
                let g = 0.6 + 0.4 * (t * 4.0 + oy).sin();
                for k in 0..4 {
                    draw_circle(ox - 16.0 + k as f32 * 10.0, oy - 4.0, 2.0, Color::new(1.0, 0.4, 0.1, g));
                }
                draw_circle(ox, oy - 14.0 - (t * 20.0 + ox).rem_euclid(20.0), 3.0, Color::new(0.4, 0.38, 0.36, 0.25));
            }
            (Biome::Ashwood, 71..=95) => {
                // A soft heap of ash.
                draw_ellipse(ox, oy, 20.0 + h2(tx, ty, 5) * 10.0, 8.0, 0.0, rgba(70, 68, 66, 200));
                draw_ellipse(ox - 4.0, oy - 2.0, 10.0, 4.0, 0.0, rgba(96, 94, 92, 200));
            }
            (Biome::Abbey, 0..=25) => {
                // A snow drift.
                draw_ellipse(ox, oy, 30.0, 12.0, 0.0, rgba(236, 242, 250, 240));
                draw_ellipse(ox - 6.0, oy - 4.0, 16.0, 6.0, 0.0, WHITE);
            }
            (Biome::Abbey, 26..=45) => {
                // Icicles thrust up out of the ground.
                shadow(14.0);
                for k in 0..4 {
                    let x = ox - 9.0 + k as f32 * 6.0;
                    let h = 14.0 + h2(tx, ty, 10 + k) * 18.0;
                    draw_triangle(vec2(x - 3.0, oy), vec2(x + 3.0, oy), vec2(x, oy - h), rgba(170, 215, 245, 230));
                    draw_line(x - 1.0, oy - 2.0, x, oy - h + 3.0, 1.0, rgba(240, 250, 255, 255));
                }
            }
            (Biome::Abbey, 46..=60) => {
                // A monk frozen where he stood.
                shadow(14.0);
                draw_triangle(vec2(ox - 12.0, oy + 4.0), vec2(ox + 12.0, oy + 4.0), vec2(ox, oy - 34.0), rgba(120, 130, 145, 255));
                draw_circle(ox, oy - 34.0, 7.0, rgba(140, 150, 165, 255));
                draw_circle(ox, oy - 33.0, 4.0, rgba(40, 46, 56, 255));
                draw_triangle(vec2(ox - 12.0, oy + 4.0), vec2(ox - 4.0, oy + 4.0), vec2(ox - 2.0, oy - 30.0), rgba(220, 235, 250, 160));
            }
            (Biome::Abbey, 61..=80) => {
                // A grave cross under snow.
                shadow(10.0);
                draw_rectangle(ox - 2.5, oy - 28.0, 5.0, 30.0, rgba(90, 96, 106, 255));
                draw_rectangle(ox - 10.0, oy - 20.0, 20.0, 5.0, rgba(90, 96, 106, 255));
                draw_rectangle(ox - 10.0, oy - 21.0, 20.0, 2.0, WHITE);
                draw_ellipse(ox, oy + 2.0, 12.0, 4.0, 0.0, rgba(240, 245, 252, 255));
            }
            (Biome::Garden, 0..=40) => {
                // A clipped hedge.
                draw_ellipse(ox, oy + 10.0, 34.0, 8.0, 0.0, rgba(0, 0, 0, 110));
                draw_rectangle(ox - 32.0, oy - 22.0, 64.0, 30.0, rgba(22, 52, 26, 255));
                draw_rectangle(ox - 32.0, oy - 22.0, 64.0, 6.0, rgba(38, 78, 40, 255));
                for k in 0..6 {
                    let x = ox - 28.0 + k as f32 * 11.0;
                    draw_circle(x, oy - 22.0, 4.0, rgba(34, 70, 36, 255));
                }
            }
            (Biome::Garden, 41..=60) => {
                // A topiary ball on a stem.
                shadow(12.0);
                draw_rectangle(ox - 2.0, oy - 20.0, 4.0, 22.0, rgba(60, 40, 26, 255));
                draw_circle(ox, oy - 28.0, 14.0, rgba(26, 60, 30, 255));
                draw_circle(ox - 4.0, oy - 32.0, 6.0, rgba(40, 84, 42, 255));
            }
            (Biome::Garden, 61..=78) => {
                // A rose bush, dark red in the starless night.
                draw_circle(ox, oy - 6.0, 13.0, rgba(20, 44, 22, 255));
                for k in 0..5 {
                    let a = h2(tx, ty, 20 + k) * TAU;
                    let q = vec2(ox, oy - 6.0) + Vec2::from_angle(a) * 8.0;
                    draw_circle(q.x, q.y, 2.6, rgba(140, 16, 30, 255));
                }
            }
            (Biome::Garden, 79..=88) => {
                // A lantern on a post, dim.
                shadow(8.0);
                draw_rectangle(ox - 1.5, oy - 40.0, 3.0, 42.0, rgba(40, 36, 34, 255));
                let g = 0.7 + 0.3 * (t * 3.0 + ox).sin();
                draw_circle(ox, oy - 44.0, 14.0, Color::new(0.75, 0.6, 1.0, 0.06 * g));
                draw_rectangle(ox - 4.0, oy - 50.0, 8.0, 10.0, Color::new(0.8, 0.7, 1.0, 0.8 * g));
            }
            (Biome::Garden, 89..=100) => {
                // A stone urn of dead flowers.
                shadow(12.0);
                draw_rectangle(ox - 4.0, oy - 8.0, 8.0, 10.0, rgba(100, 98, 94, 255));
                draw_ellipse(ox, oy - 12.0, 12.0, 6.0, 0.0, rgba(120, 118, 112, 255));
                for k in 0..3 {
                    draw_line(ox - 4.0 + k as f32 * 4.0, oy - 14.0, ox - 6.0 + k as f32 * 6.0, oy - 24.0, 1.0, rgba(70, 60, 50, 255));
                }
            }
            _ => {}
        }
    }

    /// A breakable prop of the newer fields.
    pub(super) fn draw_new_prop(&self, p: Vec2, v: u8, lit: &dyn Fn(Color) -> Color) {
        match (self.biome, v % 3) {
            (Biome::Ashwood, 0) => {
                // A charred stump.
                draw_ellipse(p.x, p.y + 4.0, 14.0, 6.0, 0.0, lit(rgba(34, 28, 24, 255)));
                draw_rectangle(p.x - 11.0, p.y - 14.0, 22.0, 18.0, lit(rgba(40, 32, 26, 255)));
                draw_ellipse(p.x, p.y - 14.0, 11.0, 5.0, 0.0, lit(rgba(70, 54, 40, 255)));
                draw_circle(p.x + 3.0, p.y - 6.0, 1.8, rgba(255, 120, 40, 255));
            }
            (Biome::Ashwood, 1) => {
                // A smoking cairn.
                for (dx, dy, r) in [(-6.0, 0.0, 8.0), (6.0, 1.0, 7.0), (0.0, -9.0, 7.0)] {
                    draw_circle(p.x + dx, p.y + dy, r, lit(rgba(70, 66, 62, 255)));
                }
                draw_circle(p.x, p.y - 22.0 - (self.time * 14.0).rem_euclid(14.0), 4.0, Color::new(0.5, 0.48, 0.45, 0.3));
            }
            (Biome::Ashwood, _) => {
                // A hollow log, embers inside.
                draw_rectangle(p.x - 16.0, p.y - 8.0, 32.0, 14.0, lit(rgba(46, 34, 26, 255)));
                draw_ellipse(p.x + 16.0, p.y - 1.0, 5.0, 7.0, 0.0, lit(rgba(20, 14, 10, 255)));
                draw_circle(p.x + 16.0, p.y - 1.0, 2.0, rgba(255, 110, 30, 255));
            }
            (Biome::Abbey, 0) => {
                // A block of ice with something dark inside.
                draw_rectangle(p.x - 12.0, p.y - 18.0, 24.0, 24.0, lit(Color::new(0.65, 0.82, 0.95, 0.85)));
                draw_circle(p.x, p.y - 6.0, 5.0, Color::new(0.2, 0.25, 0.35, 0.7));
                draw_line(p.x - 10.0, p.y - 16.0, p.x - 4.0, p.y - 4.0, 1.5, Color::new(1.0, 1.0, 1.0, 0.7));
            }
            (Biome::Abbey, 1) => {
                // A frozen bell, fallen.
                draw_triangle(vec2(p.x - 13.0, p.y + 4.0), vec2(p.x + 13.0, p.y + 4.0), vec2(p.x, p.y - 18.0), lit(rgba(120, 104, 70, 255)));
                draw_rectangle(p.x - 13.0, p.y + 2.0, 26.0, 4.0, lit(rgba(160, 140, 96, 255)));
                draw_ellipse(p.x, p.y - 14.0, 8.0, 3.0, 0.0, Color::new(0.9, 0.95, 1.0, 0.9));
            }
            (Biome::Abbey, _) => {
                // A reading desk under snow.
                draw_rectangle(p.x - 2.0, p.y - 10.0, 4.0, 14.0, lit(rgba(80, 60, 40, 255)));
                draw_triangle(vec2(p.x - 14.0, p.y - 10.0), vec2(p.x + 14.0, p.y - 10.0), vec2(p.x + 10.0, p.y - 20.0), lit(rgba(96, 72, 46, 255)));
                draw_rectangle(p.x - 12.0, p.y - 22.0, 22.0, 3.0, WHITE);
            }
            (Biome::Garden, 0) => {
                // A clay pot of thorns.
                draw_rectangle(p.x - 9.0, p.y - 10.0, 18.0, 14.0, lit(rgba(130, 70, 44, 255)));
                draw_rectangle(p.x - 11.0, p.y - 13.0, 22.0, 4.0, lit(rgba(150, 84, 52, 255)));
                for k in 0..4 {
                    let x = p.x - 6.0 + k as f32 * 4.0;
                    draw_line(x, p.y - 13.0, x + (k as f32 - 1.5) * 2.0, p.y - 26.0, 1.5, rgba(40, 80, 40, 255));
                }
            }
            (Biome::Garden, 1) => {
                // A birdbath.
                draw_rectangle(p.x - 3.0, p.y - 12.0, 6.0, 16.0, lit(rgba(110, 108, 104, 255)));
                draw_ellipse(p.x, p.y - 14.0, 14.0, 5.0, 0.0, lit(rgba(130, 128, 124, 255)));
                draw_ellipse(p.x, p.y - 15.0, 10.0, 3.0, 0.0, rgba(30, 40, 60, 255));
            }
            (Biome::Garden, _) => {
                // A small sundial, its shadow gone with the stars.
                draw_rectangle(p.x - 10.0, p.y - 6.0, 20.0, 10.0, lit(rgba(120, 116, 110, 255)));
                draw_triangle(vec2(p.x - 2.0, p.y - 6.0), vec2(p.x + 6.0, p.y - 6.0), vec2(p.x - 2.0, p.y - 16.0), lit(rgba(160, 140, 90, 255)));
            }
            _ => {}
        }
    }
}
