//! World rendering. Everything is drawn from primitives, no sprite assets needed.

mod actors;
mod events;
mod ground;
mod light;

pub use light::Lighting;

use crate::data::*;
use crate::world::*;
use macroquad::prelude::*;
use std::f32::consts::TAU;

pub(crate) fn rgba(r: u8, g: u8, b: u8, a: u8) -> Color {
    Color::from_rgba(r, g, b, a)
}

pub(crate) fn with_alpha(c: Color, a: f32) -> Color {
    Color::new(c.r, c.g, c.b, c.a * a)
}

fn glow(p: Vec2, r: f32, c: Color, strength: f32) {
    draw_circle(p.x, p.y, r * 2.4, with_alpha(c, 0.06 * strength));
    draw_circle(p.x, p.y, r * 1.6, with_alpha(c, 0.12 * strength));
    draw_circle(p.x, p.y, r, with_alpha(c, 0.9));
}

const BULLET_RED: Color = Color::new(0.85, 0.06, 0.05, 1.0);

/// A heart centred on `p`, about `r` across each lobe.
fn heart(p: Vec2, r: f32, c: Color) {
    // Small lobes need few corners; there can be thousands of hearts on screen.
    let sides = if r < 6.0 { 8 } else { 16 };
    draw_poly(p.x - r * 0.45, p.y - r * 0.2, sides, r * 0.6, 0.0, c);
    draw_poly(p.x + r * 0.45, p.y - r * 0.2, sides, r * 0.6, 0.0, c);
    draw_triangle(vec2(p.x - r * 1.02, p.y), vec2(p.x + r * 1.02, p.y), vec2(p.x, p.y + r * 1.1), c);
}
/// Above this many enemies on screen, models switch to the cheap version.
const LOD_ENEMIES: usize = 350;

thread_local! {
    static BRIGHTNESS: std::cell::Cell<f32> = const { std::cell::Cell::new(1.0) };
}

/// Takes the brightness setting (1.0 is normal); called every frame.
pub fn set_brightness(k: f32) {
    BRIGHTNESS.with(|b| b.set(k));
}

pub(crate) fn brightness() -> f32 {
    BRIGHTNESS.with(|b| b.get())
}

/// Pixels per world unit: 1 up to 1080 lines, then the view stays 1080 units tall,
/// so every resolution sees the same slice of the field.
pub fn view_scale() -> f32 {
    (screen_height() / 1080.0).max(1.0)
}

/// The part of the field the window shows, in world units.
pub fn view_size() -> Vec2 {
    vec2(screen_width(), screen_height()) / view_scale()
}

/// Draws `hero` standing at `at` (UI space) `scale` times its size, for the wardrobe.
/// Leaves the default camera set.
pub fn hero_preview(w: &World, hero: &Hero, at: Vec2, scale: f32) {
    let px = crate::ui::ui_scale();
    let (sw, sh) = (screen_width(), screen_height());
    let s = scale * px;
    let q = at * px;
    let target = hero.p.pos - vec2((q.x - sw * 0.5) / s, (q.y - sh * 0.5) / s);
    set_camera(&Camera2D { target, zoom: vec2(2.0 * s / sw, 2.0 * s / sh), ..Default::default() });
    w.draw_hero(hero, false);
    set_default_camera();
}

impl World {
    pub fn camera(&self) -> Camera2D {
        let shake = if self.shake > 0.0 { vec2(macroquad::rand::gen_range(-1.0, 1.0), macroquad::rand::gen_range(-1.0, 1.0)) * self.shake * self.shake_mult } else { Vec2::ZERO };
        let v = view_size();
        Camera2D { target: self.cam + shake, zoom: vec2(2.0 / v.x, 2.0 / v.y), ..Default::default() }
    }

    /// True when a circle at `p` with radius `r` can be on screen (with a little margin).
    pub(super) fn on_screen(&self, p: Vec2, r: f32) -> bool {
        let d = (p - self.cam).abs();
        let v = view_size();
        d.x < v.x * 0.5 + r + 60.0 && d.y < v.y * 0.5 + r + 60.0
    }

    /// Where a world point lands in UI space.
    pub fn world_to_screen(&self, p: Vec2) -> Vec2 {
        (p - self.cam) * (view_scale() / crate::ui::ui_scale()) + vec2(crate::ui::ui_w(), crate::ui::ui_h()) * 0.5
    }

    fn draw_projs(&self) {
        for pr in self.projs.iter().filter(|pr| self.on_screen(pr.pos, pr.r * 3.0 + 40.0)) {
            let p = pr.pos;
            let d = pr.vel.normalize_or_zero();
            match pr.kind {
                PKind::Dagger => {
                    let side = vec2(-d.y, d.x);
                    let c = if pr.evo { rgba(255, 210, 210, 255) } else { rgba(222, 214, 196, 255) };
                    let trail = p - d * 22.0;
                    draw_line(trail.x, trail.y, p.x, p.y, 3.0, with_alpha(c, 0.12));
                    let base = p - d * 8.0;
                    draw_triangle(p + d * 4.0, base + side * 2.4, base - side * 2.4, c);
                    draw_line(base.x, base.y, p.x, p.y, 0.8, rgba(255, 255, 250, 255));
                    let grip = base - d * 5.0;
                    draw_line(base.x, base.y, grip.x, grip.y, 2.4, rgba(70, 50, 34, 255));
                    draw_line(base.x + side.x * 3.0, base.y + side.y * 3.0, base.x - side.x * 3.0, base.y - side.y * 3.0, 1.5, rgba(120, 100, 70, 255));
                }
                PKind::Quarrel => {
                    let tail = p - d * 14.0;
                    draw_line(tail.x, tail.y, p.x, p.y, 2.0, rgba(90, 70, 50, 255));
                    draw_line(p.x - d.x * 4.0, p.y - d.y * 4.0, p.x, p.y, 3.0, rgba(170, 170, 180, 255));
                    if pr.evo {
                        draw_circle(p.x, p.y, 6.0, rgba(140, 60, 200, 40));
                    }
                }
                PKind::Lance => {
                    let len = 34.0 * pr.area;
                    let tail = p - d * len;
                    draw_line(tail.x, tail.y, p.x, p.y, pr.r * 0.9, rgba(110, 4, 10, 200));
                    draw_line(tail.x, tail.y, p.x, p.y, pr.r * 0.4, rgba(220, 30, 40, 255));
                    draw_circle(p.x, p.y, pr.r * 0.5, rgba(255, 90, 90, 255));
                }
                PKind::Orb => {
                    let c = if pr.evo { rgba(255, 90, 30, 255) } else { rgba(255, 140, 40, 255) };
                    glow(p, pr.r, c, 1.5);
                    draw_circle(p.x, p.y, pr.r * 0.5, rgba(255, 230, 160, 255));
                }
                PKind::Axe => {
                    let a = pr.age * 18.0;
                    let r = pr.r;
                    let c = if pr.evo { rgba(200, 220, 255, 255) } else { rgba(180, 180, 195, 255) };
                    draw_arc(p.x, p.y, 20, r * 0.6, a.to_degrees(), r * 0.5, 160.0, c);
                    draw_arc(p.x, p.y, 20, r * 0.6, a.to_degrees() + 180.0, r * 0.5, 160.0, c);
                    draw_circle(p.x, p.y, 3.0, rgba(90, 70, 50, 255));
                    if pr.evo {
                        draw_circle(p.x, p.y, r * 1.3, rgba(160, 190, 255, 14));
                    }
                }
                PKind::Bolt => glow(p, 4.0, rgba(150, 230, 255, 255), 1.5),
            }
        }
    }

    fn draw_fx_under(&self) {
        for pool in &self.pools {
            let a = (pool.life / pool.max_life * 2.0).min(1.0);
            let (outer, inner) = if pool.hex {
                (Color::new(0.55, 0.1, 0.7, 0.16 * a), Color::new(0.85, 0.3, 1.0, 0.16 * a))
            } else {
                (Color::new(1.0, 0.35, 0.05, 0.18 * a), Color::new(1.0, 0.5, 0.1, 0.18 * a))
            };
            draw_circle(pool.pos.x, pool.pos.y, pool.r, outer);
            draw_circle(pool.pos.x, pool.pos.y, pool.r * 0.6, inner);
            if pool.hex {
                draw_poly_lines(pool.pos.x, pool.pos.y, 5, pool.r * 0.7, self.time * 40.0, 1.0, with_alpha(inner, 2.5));
            }
        }
    }

    /// Attack warnings are drawn after lighting so they read even in the dark.
    fn draw_warnings(&self) {
        for t in &self.telegraphs {
            let k = 1.0 - t.life / t.max_life;
            let pulse = 0.5 + 0.5 * (self.time * 20.0).sin();
            if t.line {
                let end = t.a + (t.b - t.a) * k.min(1.0);
                draw_line(t.a.x, t.a.y, t.b.x, t.b.y, t.width, Color::new(0.8, 0.1, 0.05, 0.12));
                draw_line(t.a.x, t.a.y, end.x, end.y, t.width * 0.4, Color::new(0.9, 0.15, 0.1, 0.25 + 0.2 * pulse));
            } else {
                draw_circle(t.a.x, t.a.y, t.width, Color::new(0.8, 0.1, 0.05, 0.10));
                draw_circle(t.a.x, t.a.y, t.width * k, Color::new(0.9, 0.15, 0.1, 0.22));
                draw_circle_lines(t.a.x, t.a.y, t.width, 2.0, Color::new(0.95, 0.2, 0.1, 0.5 + 0.3 * pulse));
            }
        }
        for p in &self.pillars {
            if p.delay > 0.0 {
                let k = 1.0 - p.delay / p.max_delay;
                draw_circle(p.pos.x, p.pos.y, p.r, Color::new(0.8, 0.2, 0.05, 0.12));
                draw_circle(p.pos.x, p.pos.y, p.r * k, Color::new(1.0, 0.35, 0.1, 0.25));
                draw_circle_lines(p.pos.x, p.pos.y, p.r, 1.5, Color::new(1.0, 0.4, 0.1, 0.6));
            }
        }
    }

    fn draw_auras(&self) {
        // Chime of Warding auras, also when it is half of a fused weapon.
        for hero in self.heroes.iter().filter(|h| !h.downed) {
            for w in &hero.weapons {
                for part in w.parts().into_iter().filter(|p| p.kind == WeaponKind::Chime) {
                    let s = WeaponKind::Chime.stats(w.level, part.evo);
                    let r = 75.0 * s.area * hero.stats.area;
                    let timer = w.timers.get(part.slot).copied().unwrap_or(0.0);
                    let pulse = (timer / (s.cd * hero.stats.cooldown)).clamp(0.0, 1.0);
                    let c = if part.evo { Color::new(1.0, 0.9, 0.6, 1.0) } else { Color::new(0.95, 0.8, 0.45, 1.0) };
                    let p = hero.p.pos;
                    draw_circle(p.x, p.y, r, with_alpha(c, 0.05 + 0.05 * pulse));
                    draw_circle_lines(p.x, p.y, r, 1.5, with_alpha(c, 0.25 + 0.35 * pulse));
                }
            }
        }
    }

    fn hero_pos(&self, owner: u8, fallback: Vec2) -> Vec2 {
        self.heroes.iter().find(|h| h.slot == owner).map(|h| h.p.pos).unwrap_or(fallback)
    }

    fn draw_fx_over(&self) {
        for f in &self.fx {
            let k = f.life / f.max_life;
            match f.kind {
                FxKind::Slash { ang, radius, evo, owner } => {
                    let p = self.hero_pos(owner, f.pos);
                    let c = if evo { Color::new(1.0, 0.45, 0.15, 1.0) } else { Color::new(0.92, 0.88, 0.8, 1.0) };
                    let start = ang.to_degrees() - 60.0;
                    for layer in 0..3 {
                        let lr = radius * (0.55 + layer as f32 * 0.15);
                        draw_arc(p.x, p.y, 40, lr, start, 6.0 - layer as f32 * 1.5, 120.0, with_alpha(c, k * (0.7 - layer as f32 * 0.18)));
                    }
                }
                FxKind::Sweep { radius, evo, owner } => {
                    let p = self.hero_pos(owner, f.pos);
                    let c = if evo { Color::new(0.75, 0.05, 0.1, 1.0) } else { Color::new(0.75, 0.72, 0.8, 1.0) };
                    let turn = (1.0 - k) * 360.0;
                    draw_arc(p.x, p.y, 48, radius * 0.75, turn, 9.0 * k + 2.0, 300.0 * (1.0 - k) + 30.0, with_alpha(c, 0.7 * k));
                    draw_arc(p.x, p.y, 48, radius * 0.92, turn - 20.0, 3.0, 300.0 * (1.0 - k) + 30.0, with_alpha(c, 0.4 * k));
                    let blade = p + Vec2::from_angle(turn.to_radians()) * radius;
                    draw_line(p.x, p.y, blade.x, blade.y, 2.0, with_alpha(rgba(60, 50, 40, 255), k));
                }
                FxKind::Explosion { r, color } => {
                    let rr = r * (1.0 - k * 0.4);
                    draw_circle(f.pos.x, f.pos.y, rr, with_alpha(color, 0.35 * k));
                    draw_circle(f.pos.x, f.pos.y, rr * 0.55, Color::new(1.0, 0.9, 0.6, 0.4 * k));
                    draw_circle_lines(f.pos.x, f.pos.y, rr, 2.0, with_alpha(color, 0.8 * k));
                }
                FxKind::Lightning { from } => {
                    let mut prev = from;
                    let segs = 8;
                    for s in 1..=segs {
                        let tt = s as f32 / segs as f32;
                        let mut p = from.lerp(f.pos, tt);
                        if s < segs {
                            p += vec2(macroquad::rand::gen_range(-14.0, 14.0), macroquad::rand::gen_range(-6.0, 6.0));
                        }
                        draw_line(prev.x, prev.y, p.x, p.y, 5.0, Color::new(0.5, 0.6, 1.0, 0.3 * k));
                        draw_line(prev.x, prev.y, p.x, p.y, 2.0, Color::new(0.9, 0.95, 1.0, k));
                        prev = p;
                    }
                    draw_circle(f.pos.x, f.pos.y, 30.0 * k, Color::new(0.7, 0.8, 1.0, 0.3 * k));
                }
                FxKind::Ring { r, color } => {
                    draw_circle_lines(f.pos.x, f.pos.y, r * (1.0 - k), 4.0 * k + 1.0, with_alpha(color, k));
                }
                FxKind::Ghost => draw_circle(f.pos.x, f.pos.y, 9.0, Color::new(0.5, 0.45, 0.4, 0.35 * k)),
            }
        }
        for w in &self.waves {
            let k = 1.0 - w.r / w.max_r;
            draw_circle_lines(w.pos.x, w.pos.y, w.r, 10.0, Color::new(0.75, 0.65, 0.5, 0.25 * k));
            draw_circle_lines(w.pos.x, w.pos.y, w.r, 3.0, Color::new(0.95, 0.85, 0.7, 0.7 * k));
        }
        for p in &self.pillars {
            if p.delay <= 0.0 {
                let k = (p.after / 0.35).max(0.0);
                draw_rectangle(p.pos.x - p.r * 0.5, p.pos.y - 160.0, p.r, 160.0, Color::new(1.0, 0.45, 0.1, 0.35 * k));
                draw_circle(p.pos.x, p.pos.y, p.r, Color::new(1.0, 0.5, 0.15, 0.5 * k));
            }
        }
    }

    /// Enemy projectiles: big, red and dark-cored so they read against everything.
    fn draw_bullets(&self) {
        let pulse = 0.85 + 0.15 * (self.time * 12.0).sin();
        for b in self.ebullets.iter().filter(|b| self.on_screen(b.pos, b.r * 2.0)) {
            let p = b.pos;
            if self.cute {
                // A pink heart instead of a blood-red orb.
                let (r, pink) = (b.r * pulse, Color::new(1.0, 0.45, 0.7, 1.0));
                draw_circle(p.x, p.y, b.r * 2.0, with_alpha(pink, 0.12));
                heart(p, r, pink);
                draw_circle(p.x - r * 0.45, p.y - r * 0.35, r * 0.18, Color::new(1.0, 0.9, 0.95, 0.9));
                continue;
            }
            draw_circle(p.x, p.y, b.r * 2.0, with_alpha(BULLET_RED, 0.12));
            draw_circle(p.x, p.y, b.r * pulse, BULLET_RED);
            draw_circle(p.x, p.y, b.r * 0.55, rgba(40, 0, 0, 255));
            draw_circle(p.x - b.r * 0.25, p.y - b.r * 0.25, b.r * 0.2, rgba(255, 140, 120, 220));
            if b.homing > 0.0 {
                draw_circle_lines(p.x, p.y, b.r * 1.3, 1.5, with_alpha(BULLET_RED, 0.7));
            }
        }
    }

    fn draw_pickups(&self) {
        for pk in self.pickups.iter().filter(|pk| self.on_screen(pk.pos, 200.0)) {
            let p = pk.pos + vec2(0.0, pk.bob.sin() * 2.0);
            // Relics on the map fade out in their last seconds.
            let fade = if pk.life.is_finite() { (pk.life / 5.0).clamp(0.0, 1.0) } else { 1.0 };
            if pk.is_relic() {
                let beam = 0.5 + 0.5 * (self.time * 3.0).sin();
                draw_rectangle(p.x - 4.0, p.y - 140.0, 8.0, 140.0, Color::new(0.9, 0.3, 0.2, (0.04 + 0.03 * beam) * fade));
                draw_circle(p.x, p.y, 22.0, Color::new(0.0, 0.0, 0.0, 0.4 * fade));
            }
            match pk.kind {
                PickKind::Soul(v) => {
                    let (c, s) = if v < 2.0 {
                        (rgba(170, 210, 255, 255), 3.5)
                    } else if v < 6.0 {
                        (rgba(120, 255, 220, 255), 4.5)
                    } else if v < 25.0 {
                        (rgba(255, 220, 120, 255), 5.5)
                    } else if v < 100.0 {
                        (rgba(255, 110, 90, 255), 7.0)
                    } else {
                        // Merged hoards.
                        (rgba(230, 170, 255, 255), 9.0)
                    };
                    draw_circle(p.x, p.y, s * 2.4, with_alpha(c, 0.1));
                    for k in 1..4 {
                        let rise = k as f32 * s * 0.8;
                        let wob = (self.time * 6.0 + pk.bob + k as f32).sin() * s * 0.35;
                        draw_circle(p.x + wob, p.y - rise, s * (0.75 - k as f32 * 0.17), with_alpha(c, 0.55 - k as f32 * 0.13));
                    }
                    draw_circle(p.x, p.y, s, c);
                    draw_circle(p.x - s * 0.2, p.y - s * 0.2, s * 0.45, WHITE);
                }
                PickKind::Draught => {
                    draw_circle(p.x, p.y, 14.0, rgba(255, 140, 50, 30));
                    draw_circle(p.x, p.y + 2.0, 6.0, rgba(230, 120, 40, 255));
                    draw_rectangle(p.x - 2.0, p.y - 8.0, 4.0, 6.0, rgba(180, 170, 150, 255));
                }
                PickKind::Magnet => {
                    draw_circle(p.x, p.y, 14.0, rgba(120, 200, 255, 30));
                    draw_circle_lines(p.x, p.y, 7.0, 2.5, rgba(130, 210, 255, 255));
                    draw_circle(p.x, p.y, 2.5, WHITE);
                }
                PickKind::Coffer(boss) => {
                    let g = if boss { rgba(255, 210, 100, 255) } else { rgba(220, 180, 90, 255) };
                    let beam = 0.5 + 0.5 * (self.time * 3.0).sin();
                    draw_rectangle(p.x - 6.0, p.y - 200.0, 12.0, 200.0, Color::new(1.0, 0.85, 0.4, 0.06 + 0.04 * beam));
                    draw_circle(p.x, p.y, 26.0, Color::new(1.0, 0.8, 0.3, 0.1));
                    draw_ellipse(p.x, p.y + 11.0, 16.0, 4.0, 0.0, rgba(0, 0, 0, 120));
                    draw_rectangle(p.x - 14.5, p.y - 10.5, 29.0, 21.0, rgba(6, 5, 7, 255));
                    draw_rectangle(p.x - 13.0, p.y - 3.0, 26.0, 12.0, rgba(84, 54, 32, 255));
                    // Rounded lid.
                    draw_rectangle(p.x - 13.0, p.y - 9.0, 26.0, 6.0, rgba(110, 72, 42, 255));
                    draw_ellipse(p.x, p.y - 9.0, 13.0, 3.5, 0.0, rgba(124, 82, 48, 255));
                    for x in [-9.0, 0.0, 9.0] {
                        draw_line(p.x + x, p.y - 12.0, p.x + x, p.y + 9.0, 2.0, rgba(60, 60, 66, 255));
                    }
                    draw_line(p.x - 13.0, p.y - 3.0, p.x + 13.0, p.y - 3.0, 1.5, rgba(30, 20, 14, 255));
                    for (x, y) in [(-12.0, -8.0), (12.0, -8.0), (-12.0, 8.0), (12.0, 8.0)] {
                        draw_circle(p.x + x, p.y + y, 1.6, rgba(150, 150, 160, 255));
                    }
                    draw_rectangle(p.x - 3.0, p.y - 5.0, 6.0, 7.0, g);
                    draw_circle(p.x, p.y - 2.0, 1.2, rgba(30, 20, 10, 255));
                    let glint = (self.time * 2.0).sin().max(0.0);
                    draw_circle(p.x - 1.5, p.y - 4.0, 1.0 + glint, with_alpha(WHITE, 0.6 * glint));
                }
                PickKind::Buff(b) => {
                    let c = with_alpha(b.color(), fade);
                    glow(p, 6.0, c, 1.3);
                    draw_poly_lines(p.x, p.y, 6, 11.0, self.time * 60.0, 1.5, c);
                }
                PickKind::Relic => {
                    let c = Color::new(0.75, 0.6, 1.0, fade);
                    draw_rectangle(p.x - 7.0, p.y - 9.0, 14.0, 18.0, Color::new(0.2, 0.16, 0.24, fade));
                    draw_rectangle_lines(p.x - 7.0, p.y - 9.0, 14.0, 18.0, 1.5, c);
                    draw_circle(p.x, p.y, 3.0, c);
                    draw_circle(p.x, p.y, 16.0, with_alpha(c, 0.1));
                }
                PickKind::Pyre => {
                    let fl = 0.8 + 0.2 * (self.time * 14.0).sin();
                    draw_circle(p.x, p.y + 4.0, 9.0, Color::new(0.2, 0.12, 0.08, fade));
                    draw_circle(p.x, p.y - 2.0, 6.0 * fl, Color::new(1.0, 0.45, 0.1, fade));
                    draw_circle(p.x, p.y - 5.0, 3.0 * fl, Color::new(1.0, 0.85, 0.4, fade));
                    draw_circle(p.x, p.y, 20.0, Color::new(1.0, 0.4, 0.1, 0.08 * fade));
                }
                PickKind::Bread => {
                    draw_ellipse(p.x, p.y, 9.0, 6.0, 0.2, Color::new(0.55, 0.38, 0.2, fade));
                    draw_line(p.x - 4.0, p.y - 2.0, p.x + 3.0, p.y - 4.0, 1.0, Color::new(0.3, 0.2, 0.1, fade));
                }
            }
        }
    }

    /// Braziers in the look of the field, cold or burning, with the kindling ring.
    fn draw_braziers(&self) {
        let t = self.time;
        for (i, b) in self.braziers.iter().enumerate() {
            if !self.on_screen(b.pos, 420.0) {
                continue;
            }
            let p = b.pos;
            let lit = b.burn > 0.0;
            let fl = 0.85 + 0.15 * (t * 9.0 + i as f32).sin();
            if lit {
                draw_circle(p.x, p.y, BRAZIER_REACH, Color::new(1.0, 0.55, 0.2, 0.035));
                draw_circle_lines(p.x, p.y, BRAZIER_REACH, 1.5, Color::new(1.0, 0.6, 0.25, 0.18));
            }
            draw_ellipse(p.x, p.y + 6.0, 20.0, 6.0, 0.0, rgba(0, 0, 0, 120));
            let iron = rgba(46, 42, 40, 255);
            let top = match self.biome {
                Biome::Graveyard => {
                    // A bowl on three legs.
                    for dx in [-10.0, 0.0, 10.0] {
                        draw_line(p.x + dx, p.y + 4.0, p.x + dx * 0.4, p.y - 14.0, 2.5, iron);
                    }
                    draw_ellipse(p.x, p.y - 16.0, 15.0, 6.0, 0.0, rgba(60, 54, 50, 255));
                    draw_ellipse(p.x, p.y - 18.0, 12.0, 4.0, 0.0, rgba(24, 20, 18, 255));
                    p + vec2(0.0, -20.0)
                }
                Biome::Marsh => {
                    // A lantern hung from a crooked pole.
                    draw_line(p.x, p.y + 4.0, p.x - 2.0, p.y - 46.0, 3.0, rgba(50, 40, 30, 255));
                    draw_line(p.x - 2.0, p.y - 46.0, p.x + 12.0, p.y - 44.0, 2.0, rgba(50, 40, 30, 255));
                    let lamp = p + vec2(12.0 + (t * 1.3 + i as f32).sin() * 1.5, -34.0);
                    draw_line(p.x + 12.0, p.y - 44.0, lamp.x, lamp.y - 6.0, 1.0, iron);
                    draw_rectangle(lamp.x - 5.0, lamp.y - 6.0, 10.0, 12.0, iron);
                    draw_rectangle(lamp.x - 3.5, lamp.y - 4.5, 7.0, 9.0, if lit { rgba(255, 190, 90, 255) } else { rgba(30, 30, 26, 255) });
                    lamp
                }
                Biome::Cathedral => {
                    // A tall candelabrum with three candles.
                    draw_line(p.x, p.y + 4.0, p.x, p.y - 40.0, 3.0, rgba(90, 70, 40, 255));
                    draw_line(p.x - 12.0, p.y - 34.0, p.x + 12.0, p.y - 34.0, 2.5, rgba(90, 70, 40, 255));
                    for dx in [-12.0, 0.0, 12.0] {
                        draw_rectangle(p.x + dx - 2.0, p.y - 44.0 + if dx == 0.0 { -6.0 } else { 0.0 }, 4.0, 10.0, rgba(210, 200, 170, 255));
                    }
                    p + vec2(0.0, -48.0)
                }
            };
            if lit {
                let flames: &[f32] = if self.biome == Biome::Cathedral { &[-12.0, 0.0, 12.0] } else { &[0.0] };
                for dx in flames {
                    let base = top + vec2(*dx, if self.biome == Biome::Cathedral && *dx != 0.0 { 6.0 } else { 0.0 });
                    let h = if self.biome == Biome::Graveyard { 16.0 } else { 7.0 };
                    draw_circle(base.x, base.y - h * 0.4, h * 0.9 * fl, Color::new(1.0, 0.5, 0.15, 0.2));
                    draw_triangle(base + vec2(-h * 0.4, 0.0), base + vec2(h * 0.4, 0.0), base + vec2((t * 8.0 + dx).sin() * 2.0, -h * fl), rgba(255, 140, 40, 230));
                    draw_triangle(base + vec2(-h * 0.2, 0.0), base + vec2(h * 0.2, 0.0), base + vec2(0.0, -h * 0.6 * fl), rgba(255, 230, 150, 255));
                }
            } else {
                draw_circle(top.x, top.y, 2.0, Color::new(1.0, 0.4, 0.1, 0.3 + 0.2 * (t * 2.0 + i as f32).sin()));
            }
            if b.kindle > 0.0 && !lit {
                let k = (b.kindle / BRAZIER_KINDLE).clamp(0.0, 1.0);
                draw_arc(p.x, p.y, 40, 34.0, -90.0, 3.0, 360.0 * k, rgba(255, 190, 110, 255));
            }
        }
    }

    fn draw_shrines(&self) {
        for s in &self.shrines {
            let p = s.pos;
            let fl = 0.85 + 0.15 * (self.time * 7.0).sin();
            let lit = !s.used;
            if lit {
                draw_circle(p.x, p.y, 120.0 * fl, rgba(255, 140, 50, 12));
                draw_circle(p.x, p.y, 60.0 * fl, rgba(255, 150, 60, 22));
            }
            draw_ellipse(p.x, p.y + 10.0, 30.0, 10.0, 0.0, rgba(0, 0, 0, 120));
            draw_circle(p.x, p.y + 4.0, 22.0, rgba(50, 46, 44, 255));
            draw_circle(p.x, p.y + 4.0, 16.0, rgba(30, 26, 24, 255));
            // A blade planted in a mound of ash.
            draw_line(p.x, p.y + 4.0, p.x + 3.0, p.y - 34.0, 4.0, rgba(110, 100, 90, 255));
            draw_line(p.x - 7.0, p.y - 22.0, p.x + 9.0, p.y - 21.0, 3.0, rgba(110, 100, 90, 255));
            let flame = if lit { rgba(255, 150, 50, 255) } else { rgba(255, 210, 140, 255) };
            for k in 0..5 {
                let off = vec2((self.time * 6.0 + k as f32 * 1.3).sin() * 5.0, -(k as f32) * 3.0);
                draw_circle(p.x + off.x, p.y - 2.0 + off.y, (7.0 - k as f32) * fl, with_alpha(flame, 0.7));
            }
            if s.kindle > 0.0 && !s.used {
                draw_arc(p.x, p.y, 40, 40.0, -90.0, 3.0, 360.0 * (s.kindle / 1.5), rgba(255, 190, 110, 255));
            }
        }
        if let Some(r) = &self.remnant {
            let p = r.pos;
            let pulse = 0.5 + 0.5 * (self.time * 4.0).sin();
            draw_circle(p.x, p.y, 30.0 + pulse * 6.0, rgba(80, 220, 180, 25));
            draw_circle(p.x, p.y, 10.0, rgba(100, 240, 200, 200));
            for k in 0..3 {
                let a = self.time * 2.0 + k as f32 * TAU / 3.0;
                let o = Vec2::from_angle(a) * 16.0;
                draw_circle(p.x + o.x, p.y + o.y - 4.0, 3.0, rgba(160, 255, 230, 200));
            }
        }
    }

    fn draw_arena(&self) {
        let Some((c, r)) = self.arena else { return };
        let t = self.time;
        for k in 0..6 {
            let rr = r + (t * 0.8 + k as f32).sin() * 6.0 + k as f32 * 5.0;
            draw_circle_lines(c.x, c.y, rr, 10.0, Color::new(0.75, 0.75, 0.8, 0.05));
        }
        let n = 90;
        for k in 0..n {
            let a = k as f32 / n as f32 * TAU + t * 0.05;
            let wob = (t * 1.3 + k as f32 * 0.7).sin() * 10.0;
            let p = c + Vec2::from_angle(a) * (r + wob);
            draw_circle(p.x, p.y, 16.0 + (t + k as f32).sin() * 4.0, Color::new(0.8, 0.8, 0.85, 0.06));
        }
        draw_circle_lines(c.x, c.y, r, 2.0, Color::new(0.85, 0.85, 0.9, 0.35));
    }

    /// Where an enemy's eyes are and what colour they burn, matching its model.
    fn eye_spots(&self, e: &Enemy) -> Option<(Vec2, f32, Color)> {
        let (p, r) = (e.pos, e.r);
        let dir = self.heading(p);
        let fx = if dir.x >= 0.0 { 1.0 } else { -1.0 };
        let t = self.time + e.phase;
        Some(match e.kind {
            EnemyKind::Hollow => (p + vec2(fx * r * 0.33, -r * 0.82), r * 0.13, rgba(255, 70, 30, 255)),
            EnemyKind::Ghoul => (p + dir * r * 0.95 - vec2(0.0, r * 0.12), r * 0.15, rgba(220, 235, 80, 255)),
            EnemyKind::Moth => (p - vec2(0.0, r * 0.5), r * 0.14, rgba(255, 220, 120, 255)),
            EnemyKind::Archer => (p + vec2(fx * r * 0.08, -r * 0.95), r * 0.15, rgba(255, 90, 50, 255)),
            EnemyKind::Bloat => (p + vec2(fx * r * 0.26, -r * 0.35), 0.0, rgba(255, 60, 30, 255)),
            EnemyKind::Wraith => (p + vec2(fx * r * 0.06, -r * 0.65), r * 0.13, rgba(210, 235, 255, 255)),
            EnemyKind::Knight => (p + vec2(fx * r * 0.2, -r * 0.95), 0.0, rgba(255, 70, 40, 255)),
            EnemyKind::Cultist => (p + vec2(fx * r * 0.39, -r * 0.8), r * 0.1, rgba(255, 200, 80, 255)),
            EnemyKind::Shieldbearer => (p + vec2(-dir.x * r * 0.15, -r * 0.85) + dir * r * 0.12, r * 0.12, rgba(255, 90, 40, 255)),
            EnemyKind::Necromancer => (p + vec2(fx * r * 0.1, -r * 0.85 + (t * 2.0).sin() * r * 0.15), r * 0.12, rgba(120, 255, 140, 255)),
            EnemyKind::Leech => return None,
            EnemyKind::Gargoyle => (p + vec2(fx * r * 0.32, -r * 0.77), r * 0.14, rgba(255, 150, 60, 255)),
            EnemyKind::Banshee => (p + vec2(fx * r * 0.05, -r * 0.77 + (t * 2.4).sin() * r * 0.15), r * 0.12, rgba(200, 220, 255, 255)),
            EnemyKind::Boss(_) => return None,
        })
    }

    /// Enemy eyes drawn after the lighting, so foes in the dark show as pairs of embers.
    fn draw_eye_glow(&self) {
        if self.cute {
            return;
        }
        let shown = self.enemies.iter().filter(|e| !e.boss && !e.dead && self.on_screen(e.pos, e.r));
        // A big horde gets tiny squares only: cheaper, and nobody can tell at that size.
        let halo = shown.clone().count() <= LOD_ENEMIES;
        for e in shown {
            let Some((at, spread, c)) = self.eye_spots(e) else { continue };
            let flick = 0.8 + 0.2 * (self.time * 7.0 + e.phase * 3.0).sin();
            let pairs: &[f32] = if spread > 0.0 { &[-1.0, 1.0] } else { &[0.0] };
            for s in pairs {
                let q = at + vec2(s * spread, 0.0);
                if halo {
                    draw_poly(q.x, q.y, 10, e.r * 0.3, 0.0, with_alpha(c, 0.14 * flick * e.alpha));
                    draw_poly(q.x, q.y, 6, (e.r * 0.07).max(0.9), 0.0, with_alpha(c, 0.85 * flick * e.alpha));
                } else {
                    draw_rectangle(q.x - 0.9, q.y - 0.9, 1.8, 1.8, with_alpha(c, 0.85 * flick * e.alpha));
                }
            }
        }
    }

    /// Ash drifting down through the light of the camera, now and then an ember; petals
    /// in cute mode. Fixed to the world, so it moves with the field, not the screen.
    fn draw_ash(&self) {
        let view = view_size() + vec2(200.0, 200.0);
        let origin = self.cam - view * 0.5;
        let n = (90.0 * self.particle_mult) as usize;
        for k in 0..n {
            let s = k as f32 * 12.9898;
            let seed = (s.sin() * 43758.547).fract().abs();
            let seed2 = ((s * 1.7).sin() * 24634.635).fract().abs();
            let speed = 12.0 + seed2 * 18.0;
            let drift = (self.time * (0.3 + seed * 0.4) + seed * 30.0).sin() * 25.0;
            // Wrap inside the view so there is always the same amount on screen.
            let x = (seed * 7919.0 + self.time * 6.0 + drift - origin.x).rem_euclid(view.x) + origin.x;
            let y = (seed2 * 6271.0 + self.time * speed - origin.y).rem_euclid(view.y) + origin.y;
            let ember = k % 11 == 0;
            let c = if self.cute {
                [Color::new(1.0, 0.75, 0.88, 0.75), Color::new(1.0, 0.95, 0.98, 0.7)][k % 2]
            } else if ember {
                Color::new(1.0, 0.55, 0.2, 0.55 + 0.35 * (self.time * 5.0 + seed * 9.0).sin().abs())
            } else {
                Color::new(0.6, 0.58, 0.56, 0.22)
            };
            let size = if ember { 1.4 } else { 1.0 + seed * 1.2 };
            if self.cute {
                draw_ellipse(x, y, size * 2.2, size * 1.3, self.time + seed * 6.0, c);
            } else {
                draw_poly(x, y, 5, size, seed * 6.0, c);
            }
        }
    }

    /// A little warm light around each living hero; the rest of the world stays dark.
    fn draw_hero_light(&self) {
        let fl = 0.92 + 0.08 * (self.time * 11.0).sin() * (self.time * 7.3).cos();
        let c = if self.cute { Color::new(1.0, 0.7, 0.85, 0.016) } else { Color::new(1.0, 0.6, 0.3, 0.012) };
        for h in self.heroes.iter().filter(|h| !h.downed) {
            let p = h.p.pos;
            for k in 0..5 {
                draw_circle(p.x, p.y, (60.0 + k as f32 * 45.0) * fl, c);
            }
        }
    }

    pub fn draw(&self, light: Option<&Lighting>) {
        let cam = self.camera();
        set_camera(&cam);
        self.draw_ground(&cam);
        self.draw_decals();
        self.draw_props();
        self.draw_walls();
        if light.is_none() {
            self.draw_hero_light();
        }
        self.draw_shrines();
        self.draw_braziers();
        self.draw_events();
        self.draw_fx_under();
        self.draw_auras();
        self.draw_pickups();
        // Only what can be seen is drawn; a huge horde on screen gets the cheap models.
        let mut order: Vec<&Enemy> = self.enemies.iter().filter(|e| !e.boss && self.on_screen(e.pos, e.r * 2.0)).collect();
        order.sort_by(|a, b| a.pos.y.total_cmp(&b.pos.y));
        let crowded = order.len() > LOD_ENEMIES;
        for e in order {
            self.draw_enemy(e, crowded);
            if self.cute {
                // Rosy cheeks and a happy little face for everyone.
                let (p, r) = (e.pos, e.r);
                let blush = Color::new(1.0, 0.45, 0.6, 0.55 * e.alpha);
                draw_circle(p.x - r * 0.45, p.y - r * 0.55, r * 0.22, blush);
                draw_circle(p.x + r * 0.45, p.y - r * 0.55, r * 0.22, blush);
                if !crowded {
                    let ink = Color::new(0.2, 0.08, 0.14, 0.85 * e.alpha);
                    let sparkle = Color::new(1.0, 1.0, 1.0, 0.9 * e.alpha);
                    for s in [-1.0, 1.0] {
                        let eye = vec2(p.x + s * r * 0.28, p.y - r * 0.78);
                        draw_circle(eye.x, eye.y, r * 0.13, ink);
                        draw_circle(eye.x - r * 0.04, eye.y - r * 0.05, r * 0.05, sparkle);
                    }
                    draw_arc(p.x, p.y - r * 0.62, 10, r * 0.14, 20.0, r * 0.05, 140.0, ink);
                }
            }
        }
        for e in self.enemies.iter().filter(|e| e.boss) {
            self.draw_enemy(e, false);
        }
        if let Some(cat) = &self.cat {
            self.draw_cat(cat);
        }
        // Projectiles first so big weapons never hide the heroes.
        self.draw_projs();
        let others = self.heroes.len() > 1;
        for (i, hero) in self.heroes.iter().enumerate() {
            if i != self.local {
                self.draw_hero(hero, others);
            }
        }
        if let Some(me) = self.heroes.get(self.local) {
            if !(self.dead && !others) {
                self.draw_hero(me, false);
            }
        }
        for hero in self.heroes.iter().filter(|h| !h.downed) {
            let evo = hero.weapons.iter().any(|w| w.has(WeaponKind::Lanterns) && (w.evolved || w.is_fused()));
            let c = if evo { rgba(140, 255, 230, 255) } else { rgba(120, 200, 255, 255) };
            for (p, r) in hero.lantern_positions() {
                glow(p, r * 0.55, c, 1.2);
                draw_circle(p.x, p.y - r * 0.2, r * 0.25, WHITE);
            }
        }
        self.draw_fx_over();
        for p in self.particles.iter().filter(|p| self.on_screen(p.pos, p.size * 3.0)) {
            let k = p.life / p.max_life;
            if p.glow {
                draw_circle(p.pos.x, p.pos.y, p.size * 2.5, with_alpha(p.color, 0.12 * k));
            }
            let (r, c) = (p.size * (0.4 + 0.6 * k), with_alpha(p.color, if p.glow { k } else { 0.4 + 0.6 * k }));
            if self.cute && !p.glow && p.size >= 2.0 {
                heart(p.pos, r * 1.3, c);
            } else {
                draw_circle(p.pos.x, p.pos.y, r, c);
            }
        }
        if self.fog {
            self.draw_fog();
        }
        if let Some(l) = light {
            self.apply_lighting(l, &cam);
            set_camera(&cam);
            // Eyes and ash glow on their own, so they still show in the dark.
            self.draw_eye_glow();
        }
        self.draw_ash();
        // Everything below must stay readable in the dark.
        self.draw_warnings();
        self.draw_bullets();
        self.draw_arena();
        for n in &self.nums {
            let k = (n.life / 0.7).min(1.0);
            let txt = format!("{:.0}", n.val);
            let size = if n.crit { 22.0 } else { 15.0 };
            let c = match (n.crit, self.cute) {
                (true, true) => Color::new(1.0, 0.45, 0.75, k),
                (true, false) => Color::new(1.0, 0.3, 0.2, k),
                (false, true) => Color::new(1.0, 0.95, 1.0, k * 0.9),
                (false, false) => Color::new(0.95, 0.92, 0.88, k * 0.85),
            };
            draw_text(&txt, n.pos.x - 6.0 + 1.0, n.pos.y + 1.0, size, Color::new(0.0, 0.0, 0.0, k * 0.8));
            draw_text(&txt, n.pos.x - 6.0, n.pos.y, size, c);
        }
        let others = self.heroes.len() > 1;
        if others {
            for (i, hero) in self.heroes.iter().enumerate() {
                if i != self.local {
                    self.draw_name(hero);
                }
            }
        }
        set_default_camera();
    }
}
