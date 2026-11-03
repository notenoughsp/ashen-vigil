//! The ground: stone, ruins, the dead, blood and drifting fog.

use super::*;

const TILE: f32 = 64.0;
const REGION: i32 = 6;

fn hash(x: i32, y: i32, salt: u32) -> u32 {
    let mut h = (x as u32).wrapping_mul(374_761_393) ^ (y as u32).wrapping_mul(668_265_263) ^ salt.wrapping_mul(2_246_822_519);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^ (h >> 16)
}

fn hf(x: i32, y: i32, salt: u32) -> f32 {
    (hash(x, y, salt) % 10000) as f32 / 10000.0
}

/// Visible tile range around the camera, with a margin.
fn tile_range(cam: Vec2, margin: f32) -> (i32, i32, i32, i32) {
    let half = super::view_size() * 0.5 + vec2(margin, margin);
    (((cam.x - half.x) / TILE).floor() as i32, ((cam.x + half.x) / TILE).ceil() as i32, ((cam.y - half.y) / TILE).floor() as i32, ((cam.y + half.y) / TILE).ceil() as i32)
}

fn gravestone(x: f32, y: f32, tilt: f32) {
    draw_ellipse(x, y + 12.0, 12.0, 4.0, 0.0, rgba(0, 0, 0, 100));
    let c = rgba(50, 48, 52, 255);
    draw_rectangle_ex(x, y + 12.0, 16.0, 22.0, DrawRectangleParams { offset: vec2(0.5, 1.0), rotation: tilt, color: c });
    let top = vec2(x, y + 12.0) + Vec2::from_angle(tilt - std::f32::consts::FRAC_PI_2) * 22.0;
    draw_circle(top.x, top.y, 8.0, c);
    draw_line(top.x, top.y - 2.0, top.x, top.y + 12.0, 2.0, rgba(28, 26, 30, 255));
    draw_line(top.x - 4.0, top.y + 3.0, top.x + 4.0, top.y + 3.0, 2.0, rgba(28, 26, 30, 255));
}

fn dead_tree(x: f32, y: f32, seed: u32) {
    let c = rgba(24, 20, 20, 255);
    draw_ellipse(x, y + 4.0, 22.0, 7.0, 0.0, rgba(0, 0, 0, 110));
    draw_line(x, y + 4.0, x + 2.0, y - 46.0, 6.0, c);
    for k in 0..5 {
        let s = hash(seed as i32, k, 3);
        let h = 14.0 + k as f32 * 8.0;
        let side = if k % 2 == 0 { -1.0 } else { 1.0 };
        let len = 14.0 + (s % 14) as f32;
        let from = vec2(x + 1.0, y - h);
        let to = from + vec2(side * len, -len * 0.6 - (s % 8) as f32);
        draw_line(from.x, from.y, to.x, to.y, 2.5, c);
        draw_line(to.x, to.y, to.x + side * 6.0, to.y - 7.0, 1.5, c);
    }
}

/// A body impaled on a stake: a warning to the living.
fn impaled(x: f32, y: f32, t: f32) {
    draw_ellipse(x, y + 6.0, 12.0, 4.0, 0.0, rgba(0, 0, 0, 110));
    draw_circle(x + 3.0, y + 6.0, 9.0, rgba(60, 6, 8, 140));
    draw_line(x, y + 6.0, x, y - 44.0, 3.0, rgba(60, 46, 34, 255));
    let sway = (t * 0.7 + x * 0.01).sin() * 1.5;
    draw_ellipse(x + sway, y - 26.0, 7.0, 12.0, 0.15, rgba(58, 50, 46, 255));
    draw_circle(x + sway + 1.0, y - 40.0, 5.0, rgba(78, 70, 64, 255));
    draw_line(x + sway - 6.0, y - 30.0, x + sway - 12.0, y - 18.0, 2.0, rgba(58, 50, 46, 255));
    draw_line(x + sway + 6.0, y - 30.0, x + sway + 11.0, y - 20.0, 2.0, rgba(58, 50, 46, 255));
    draw_line(x, y - 22.0, x, y - 12.0, 2.5, rgba(110, 10, 12, 230));
}

fn skull_pile(x: f32, y: f32) {
    draw_ellipse(x, y + 4.0, 16.0, 5.0, 0.0, rgba(0, 0, 0, 100));
    for (dx, dy) in [(-7.0, 2.0), (6.0, 2.0), (0.0, -4.0), (-3.0, 5.0), (8.0, -2.0)] {
        draw_circle(x + dx, y + dy, 4.5, rgba(150, 142, 124, 255));
        draw_circle(x + dx - 1.5, y + dy, 1.1, rgba(20, 18, 16, 255));
        draw_circle(x + dx + 1.5, y + dy, 1.1, rgba(20, 18, 16, 255));
    }
}

fn cage(x: f32, y: f32, t: f32) {
    let sway = (t * 0.9 + y * 0.02).sin() * 2.0;
    draw_ellipse(x, y + 18.0, 14.0, 4.0, 0.0, rgba(0, 0, 0, 90));
    draw_line(x - 18.0, y - 36.0, x + 18.0, y - 36.0, 3.0, rgba(40, 32, 26, 255));
    draw_line(x - 16.0, y - 36.0, x - 16.0, y + 18.0, 3.0, rgba(40, 32, 26, 255));
    draw_line(x + sway, y - 36.0, x + sway, y - 26.0, 1.0, rgba(60, 60, 64, 255));
    let c = rgba(56, 56, 62, 255);
    for k in 0..5 {
        let bx = x + sway - 8.0 + k as f32 * 4.0;
        draw_line(bx, y - 26.0, bx, y - 2.0, 1.2, c);
    }
    draw_line(x + sway - 9.0, y - 26.0, x + sway + 9.0, y - 26.0, 1.5, c);
    draw_line(x + sway - 9.0, y - 2.0, x + sway + 9.0, y - 2.0, 1.5, c);
    draw_circle(x + sway, y - 8.0, 3.5, rgba(130, 122, 106, 255));
}

impl World {
    pub(super) fn draw_ground(&self) {
        clear_background(rgba(7, 6, 8, 255));
        let (x0, x1, y0, y1) = tile_range(self.cam, TILE * 2.0);
        for ty in y0..y1 {
            for tx in x0..x1 {
                let px = tx as f32 * TILE;
                let py = ty as f32 * TILE;
                if !inside_world(vec2(px + TILE * 0.5, py + TILE * 0.5), -TILE * 0.5) {
                    continue;
                }
                let v = hf(tx, ty, 1);
                // Large-scale variation so the floor reads as regions, not a grid.
                let region = hf(tx.div_euclid(REGION), ty.div_euclid(REGION), 9);
                let base = 13.0 + v * 7.0 + region * 5.0;
                let red = if hf(tx.div_euclid(3), ty.div_euclid(3), 17) > 0.93 { 4.0 } else { 0.0 };
                let c = Color::new((base + red) / 255.0, (base - 1.5) / 255.0, (base + 1.0) / 255.0, 1.0);
                draw_rectangle(px + 1.0, py + 1.0, TILE - 2.0, TILE - 2.0, c);
                // Worn stone: some tiles are split into smaller slabs.
                if hash(tx, ty, 21).is_multiple_of(5) {
                    draw_line(px + 1.0, py + TILE * 0.5, px + TILE - 1.0, py + TILE * 0.5, 1.0, rgba(6, 5, 7, 255));
                }
            }
        }
    }

    /// Props stand on the ground but under everything that moves.
    pub(super) fn draw_props(&self) {
        let (x0, x1, y0, y1) = tile_range(self.cam, TILE * 3.0);
        for ty in y0..y1 {
            for tx in x0..x1 {
                let px = tx as f32 * TILE;
                let py = ty as f32 * TILE;
                let d = hash(tx, ty, 2) % 1000;
                let ox = px + 12.0 + hf(tx, ty, 3) * 40.0;
                let oy = py + 12.0 + hf(tx, ty, 4) * 40.0;
                if !inside_world(vec2(ox, oy), 90.0) {
                    continue;
                }
                match d {
                    0..=12 => gravestone(ox, oy, (hf(tx, ty, 5) - 0.5) * 0.4),
                    13..=20 => {
                        draw_circle(ox, oy, 13.0, rgba(36, 34, 38, 255));
                        draw_circle(ox, oy, 10.0, rgba(46, 44, 48, 255));
                        draw_line(ox - 6.0, oy - 3.0, ox + 4.0, oy + 5.0, 1.5, rgba(22, 20, 24, 255));
                    }
                    21..=50 => {
                        let a = hf(tx, ty, 5) * std::f32::consts::PI;
                        let dir = Vec2::from_angle(a) * 7.0;
                        draw_line(ox - dir.x, oy - dir.y, ox + dir.x, oy + dir.y, 2.0, rgba(110, 102, 88, 200));
                        draw_circle(ox + dir.x + 6.0, oy + dir.y, 3.5, rgba(120, 112, 96, 200));
                    }
                    51..=130 => {
                        let a = hf(tx, ty, 6) * TAU;
                        let mut p = vec2(ox, oy);
                        for k in 0..3 {
                            let n = p + Vec2::from_angle(a + hf(tx, ty, 10 + k) - 0.5) * 9.0;
                            draw_line(p.x, p.y, n.x, n.y, 1.0, rgba(4, 3, 5, 255));
                            p = n;
                        }
                    }
                    131..=170 => draw_circle(ox, oy, 9.0 + hf(tx, ty, 7) * 8.0, rgba(20, 28, 18, 150)),
                    171..=176 => {
                        let fl = 0.8 + 0.2 * (self.time * 9.0 + ox).sin();
                        for k in 0..4 {
                            draw_circle(ox, oy, (14.0 + k as f32 * 12.0) * fl, rgba(255, 130, 50, 6));
                        }
                        for k in 0..3 {
                            let cx = ox + (k as f32 - 1.0) * 6.0;
                            let h = 6.0 + k as f32 * 2.0;
                            draw_rectangle(cx - 1.5, oy - h, 3.0, h, rgba(190, 180, 150, 255));
                            draw_circle(cx, oy - h - 2.0, 1.8 * fl, rgba(255, 190, 90, 255));
                        }
                    }
                    177..=182 => dead_tree(ox, oy, hash(tx, ty, 8)),
                    183..=186 => impaled(ox, oy, self.time),
                    187..=192 => skull_pile(ox, oy),
                    193..=195 => cage(ox, oy, self.time),
                    196..=215 => {
                        // Old blood soaked into the stone.
                        draw_circle(ox, oy, 10.0 + hf(tx, ty, 11) * 10.0, rgba(40, 4, 6, 110));
                        draw_circle(ox + 8.0, oy + 4.0, 5.0, rgba(40, 4, 6, 110));
                    }
                    _ => {}
                }
            }
        }
    }

    /// Blood, bodies and scorch marks, fading over their lifetime.
    pub(super) fn draw_decals(&self) {
        let half = super::view_size() * 0.5 + vec2(80.0, 80.0);
        for d in &self.decals {
            let rel = d.pos - self.cam;
            if rel.x.abs() > half.x || rel.y.abs() > half.y {
                continue;
            }
            let fade = (d.life / d.max_life * 3.0).min(1.0);
            let c = with_alpha(d.color, fade);
            match d.kind {
                DecalKind::Blood => {
                    draw_ellipse(d.pos.x, d.pos.y, d.r, d.r * 0.7, d.rot, c);
                    let n = 2 + d.seed % 3;
                    for k in 0..n {
                        let s = d.seed.wrapping_mul(k + 7);
                        let a = d.rot + (s % 628) as f32 / 100.0;
                        let off = Vec2::from_angle(a) * d.r * (0.8 + (s % 50) as f32 / 60.0);
                        draw_circle(d.pos.x + off.x, d.pos.y + off.y, d.r * 0.22, c);
                    }
                }
                DecalKind::Scorch => {
                    draw_circle(d.pos.x, d.pos.y, d.r, c);
                    draw_circle(d.pos.x, d.pos.y, d.r * 0.6, with_alpha(c, 0.6));
                }
                DecalKind::Corpse(kind) => self.draw_corpse(d, kind, fade),
            }
        }
    }

    fn draw_corpse(&self, d: &Decal, kind: EnemyKind, fade: f32) {
        let p = d.pos;
        let r = d.r;
        let body = match kind {
            EnemyKind::Ghoul => rgba(70, 82, 60, 255),
            EnemyKind::Archer => rgba(150, 144, 130, 255),
            EnemyKind::Bloat => rgba(78, 84, 50, 255),
            EnemyKind::Knight => rgba(52, 54, 62, 255),
            EnemyKind::Boss(_) => rgba(48, 40, 38, 255),
            _ => rgba(56, 50, 46, 255),
        };
        let c = with_alpha(body, fade * 0.9);
        let dir = Vec2::from_angle(d.rot);
        // Torso, head and the head torn a little away.
        draw_ellipse(p.x, p.y, r, r * 0.55, d.rot, c);
        let head = p + dir * (r * 1.15);
        draw_circle(head.x, head.y, r * 0.42, with_alpha(body, fade * 0.8));
        draw_circle(p.x + dir.x * r * 0.7, p.y + dir.y * r * 0.7, r * 0.3, with_alpha(rgba(110, 6, 8, 255), fade * 0.8));
        if d.seed.is_multiple_of(3) {
            let limb = p - dir * r * 1.4 + vec2(-dir.y, dir.x) * r * 0.6;
            draw_line(p.x, p.y, limb.x, limb.y, 2.5, c);
        }
    }

    /// Slow banks of fog drifting across the field.
    pub(super) fn draw_fog(&self) {
        let (sw, sh) = super::view_size().into();
        let t = self.time * 12.0;
        let cell = 360.0;
        let x0 = ((self.cam.x - sw) / cell).floor() as i32;
        let x1 = ((self.cam.x + sw) / cell).ceil() as i32;
        let y0 = ((self.cam.y - sh) / cell).floor() as i32;
        let y1 = ((self.cam.y + sh) / cell).ceil() as i32;
        for cy in y0..y1 {
            for cx in x0..x1 {
                let v = hf(cx, cy, 31);
                if v < 0.45 {
                    continue;
                }
                let p = vec2(cx as f32 * cell + hf(cx, cy, 32) * cell + t, cy as f32 * cell + hf(cx, cy, 33) * cell + (t * 0.3 + v * 9.0).sin() * 20.0);
                let r = 110.0 + v * 130.0;
                for k in 0..3 {
                    let rr = r * (1.0 - k as f32 * 0.25);
                    draw_circle(p.x + k as f32 * 18.0, p.y, rr, Color::new(0.5, 0.5, 0.55, 0.022));
                }
            }
        }
    }

    /// The wall around the field, its torches, and the abyss beyond.
    pub(super) fn draw_walls(&self) {
        let half = super::view_size() * 0.5 + vec2(200.0, 200.0);
        let (lo, hi) = (self.cam - half, self.cam + half);
        let h = WORLD_HALF;
        // The abyss: the floor just ends in mist and nothing.
        for k in 0..6 {
            let d = 40.0 + k as f32 * 40.0;
            let c = Color::new(0.05, 0.05, 0.07, 0.25 - k as f32 * 0.04);
            draw_rectangle_lines(-h - d, -h - d, (h + d) * 2.0, (h + d) * 2.0, 40.0, c);
        }
        let block = 64.0;
        let thick = 90.0;
        let face = 26.0;
        let t = self.time;
        // (start, direction along the wall, outward normal)
        let edges = [
            (vec2(-h, -h), vec2(1.0, 0.0), vec2(0.0, -1.0)),
            (vec2(-h, h), vec2(1.0, 0.0), vec2(0.0, 1.0)),
            (vec2(-h, -h), vec2(0.0, 1.0), vec2(-1.0, 0.0)),
            (vec2(h, -h), vec2(0.0, 1.0), vec2(1.0, 0.0)),
        ];
        let n = (h * 2.0 / block).ceil() as i32;
        for (ei, (start, along, out)) in edges.iter().enumerate() {
            for i in 0..=n {
                let a = *start + *along * (i as f32 * block);
                if a.x < lo.x - block || a.x > hi.x + block || a.y < lo.y - block || a.y > hi.y + block {
                    continue;
                }
                let v = hf(i, ei as i32, 41);
                let stone = Color::new((44.0 + v * 14.0) / 255.0, (41.0 + v * 12.0) / 255.0, (46.0 + v * 14.0) / 255.0, 1.0);
                // The inner face of the wall, darker, with a shadow cast onto the floor.
                let f0 = a;
                let f1 = a + *along * block;
                let g0 = f0 - *out * face;
                let g1 = f1 - *out * face;
                let face_col = Color::new(stone.r * 0.55, stone.g * 0.55, stone.b * 0.6, 1.0);
                draw_triangle(f0, f1, g1, face_col);
                draw_triangle(f0, g1, g0, face_col);
                let s0 = g0 - *out * 30.0;
                let s1 = g1 - *out * 30.0;
                draw_triangle(g0, g1, s1, Color::new(0.0, 0.0, 0.0, 0.35));
                draw_triangle(g0, s1, s0, Color::new(0.0, 0.0, 0.0, 0.35));
                // A block of the wall, its top face lighter, mortar lines dark.
                let p0 = a;
                let p1 = a + *along * block;
                let q0 = p0 + *out * thick;
                let q1 = p1 + *out * thick;
                draw_triangle(p0, p1, q1, stone);
                draw_triangle(p0, q1, q0, stone);
                let rim = Color::new(stone.r * 1.5, stone.g * 1.5, stone.b * 1.5, 1.0);
                draw_line(p0.x, p0.y, p1.x, p1.y, 4.0, rim);
                draw_line(p0.x, p0.y, q0.x, q0.y, 1.5, rgba(8, 7, 9, 255));
                let mid0 = p0 + *out * (thick * 0.5);
                let mid1 = p1 + *out * (thick * 0.5);
                draw_line(mid0.x, mid0.y, mid1.x, mid1.y, 1.0, rgba(10, 9, 12, 255));
                // Crenellations on the inner face, and the odd crack.
                if i % 2 == 0 {
                    let c0 = p0 - *out * 10.0 + *along * 12.0;
                    let c1 = c0 + *along * 26.0;
                    draw_line(c0.x, c0.y, c1.x, c1.y, 10.0, stone);
                }
                if v > 0.8 {
                    let c = a + *along * 30.0 + *out * 10.0;
                    draw_line(c.x, c.y, c.x + out.x * 18.0 + along.x * 6.0, c.y + out.y * 18.0 + along.y * 6.0, 1.0, rgba(6, 5, 7, 255));
                }
                // Torches every eighth block.
                if i % 8 == 4 {
                    let base = a + *along * (block * 0.5) - *out * (face + 2.0);
                    let tip = base - *out * 14.0 - vec2(0.0, 16.0);
                    draw_line(base.x, base.y, tip.x, tip.y + 8.0, 4.0, rgba(60, 44, 30, 255));
                    draw_circle(base.x, base.y, 4.0, rgba(40, 40, 46, 255));
                    let fl = 0.8 + 0.2 * (t * 13.0 + i as f32).sin();
                    for k in 0..4u8 {
                        let o = vec2((t * 9.0 + k as f32 * 1.7 + i as f32).sin() * 2.5, -(k as f32) * 4.0);
                        draw_circle(tip.x + o.x, tip.y + o.y, (9.0 - k as f32 * 2.0) * fl, rgba(255, 110 + k * 30, 40, 220));
                    }
                    draw_circle(tip.x, tip.y, 3.0, rgba(255, 230, 160, 255));
                }
            }
        }
    }

    /// Fixed light sources of the scenery: candles and wall torches.
    pub(super) fn static_lights(&self, out: &mut Vec<(Vec2, f32, Color)>) {
        let fl = |seed: f32| 0.85 + 0.15 * (self.time * 9.0 + seed).sin();
        let (x0, x1, y0, y1) = tile_range(self.cam, TILE * 6.0);
        for ty in y0..y1 {
            for tx in x0..x1 {
                if (171..=176).contains(&(hash(tx, ty, 2) % 1000)) {
                    let ox = tx as f32 * TILE + 12.0 + hf(tx, ty, 3) * 40.0;
                    let oy = ty as f32 * TILE + 12.0 + hf(tx, ty, 4) * 40.0;
                    if inside_world(vec2(ox, oy), 90.0) {
                        out.push((vec2(ox, oy), 170.0 * fl(ox), Color::new(0.75, 0.42, 0.16, 1.0)));
                    }
                }
            }
        }
        let h = WORLD_HALF;
        let n = (h * 2.0 / 64.0).ceil() as i32;
        let half = super::view_size() * 0.5 + vec2(400.0, 400.0);
        for i in (4..=n).step_by(8) {
            let off = i as f32 * 64.0 + 32.0 - h;
            for p in [vec2(off, -h + 10.0), vec2(off, h - 10.0), vec2(-h + 10.0, off), vec2(h - 10.0, off)] {
                let d = p - self.cam;
                if d.x.abs() < half.x && d.y.abs() < half.y {
                    out.push((p, 340.0 * fl(off), Color::new(1.0, 0.55, 0.22, 1.0)));
                }
            }
        }
    }
}
