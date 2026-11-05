//! Heroes, enemies, bosses and the cat. Built from shaded primitives with dark outlines.
//!
//! Models are drawn in units of the actor's radius `r`, facing left or right (`fx` = -1/1),
//! with a walk cycle from time and a per-actor phase.

use super::*;
use std::f32::consts::PI;

const OUTLINE: Color = Color::new(0.02, 0.016, 0.024, 1.0);

fn shade(c: Color, k: f32) -> Color {
    Color::new((c.r * k).min(1.0), (c.g * k).min(1.0), (c.b * k).min(1.0), c.a)
}

/// A ball with an outline, a highlight up-left and a shadow down-right.
fn ball(p: Vec2, r: f32, c: Color, hi: bool) {
    if hi {
        draw_circle(p.x, p.y, r + 1.3, with_alpha(OUTLINE, c.a));
    }
    draw_circle(p.x, p.y, r, c);
    if hi && r > 3.0 {
        draw_circle(p.x + r * 0.22, p.y + r * 0.25, r * 0.72, shade(c, 0.78));
        draw_circle(p.x - r * 0.05, p.y - r * 0.05, r * 0.7, c);
        draw_circle(p.x - r * 0.32, p.y - r * 0.36, r * 0.32, with_alpha(shade(c, 1.45), 0.55));
    }
}

/// An ellipse with an outline and a top highlight.
fn blob(p: Vec2, rx: f32, ry: f32, rot: f32, c: Color, hi: bool) {
    if hi {
        draw_ellipse(p.x, p.y, rx + 1.3, ry + 1.3, rot, with_alpha(OUTLINE, c.a));
    }
    draw_ellipse(p.x, p.y, rx, ry, rot, c);
    if hi && rx > 3.0 {
        draw_ellipse(p.x - rx * 0.15, p.y - ry * 0.3, rx * 0.6, ry * 0.4, rot, with_alpha(shade(c, 1.3), 0.45));
    }
}

/// A limb segment with rounded ends.
fn limb(a: Vec2, b: Vec2, w: f32, c: Color, hi: bool) {
    if hi {
        draw_line(a.x, a.y, b.x, b.y, w + 2.0, with_alpha(OUTLINE, c.a));
    }
    draw_line(a.x, a.y, b.x, b.y, w, c);
    draw_circle(b.x, b.y, w * 0.5, c);
}

/// Ragged cloth hanging between two shoulder points, its hem torn into points.
fn rag(l: Vec2, r: Vec2, drop: f32, sway: f32, c: Color, seed: f32, hi: bool) {
    let n = 5;
    let hem: Vec<Vec2> = (0..=n)
        .map(|i| {
            let k = i as f32 / n as f32;
            let tear = if i % 2 == 0 { 1.0 } else { 0.78 + 0.12 * (seed + i as f32).sin() };
            l.lerp(r, k) + vec2(sway * (0.5 + k * 0.5), drop * tear)
        })
        .collect();
    let top_mid = (l + r) * 0.5;
    if hi {
        for i in 0..n {
            draw_triangle(top_mid - vec2(0.0, 1.5), hem[i] + vec2(0.0, 1.5), hem[i + 1] + vec2(0.0, 1.5), with_alpha(OUTLINE, c.a));
        }
    }
    draw_triangle(l, r, hem[n], c);
    draw_triangle(l, hem[n], hem[0], c);
    for i in 0..n {
        draw_triangle(top_mid, hem[i], hem[i + 1], c);
    }
    if hi {
        // Folds.
        for (i, h) in hem.iter().enumerate().take(n).skip(1) {
            let a = l.lerp(r, i as f32 / n as f32);
            draw_line(a.x, a.y + drop * 0.15, h.x, h.y - drop * 0.2, 1.0, shade(c, 0.7));
        }
    }
}

/// A short chain of links between two points.
fn chain(a: Vec2, b: Vec2, c: Color) {
    let n = ((b - a).length() / 4.0).max(1.0) as i32;
    for k in 0..=n {
        let q = a.lerp(b, k as f32 / n as f32) + vec2(0.0, (k as f32 * 0.9).sin() * 1.2);
        draw_circle_lines(q.x, q.y, 1.7, 0.9, c);
    }
}

/// Little cross-stitches along a line, for stitched wounds.
fn stitches(a: Vec2, b: Vec2, n: i32, c: Color) {
    draw_line(a.x, a.y, b.x, b.y, 1.0, c);
    let side = (b - a).normalize_or_zero().perp() * 2.2;
    for k in 1..n {
        let q = a.lerp(b, k as f32 / n as f32);
        draw_line(q.x - side.x, q.y - side.y, q.x + side.x, q.y + side.y, 1.0, c);
    }
}

fn eyes(head: Vec2, spread: f32, size: f32, c: Color) {
    for s in [-1.0, 1.0] {
        let e = head + vec2(s * spread, 0.0);
        draw_circle(e.x, e.y, size * 2.6, with_alpha(c, 0.14));
        draw_circle(e.x, e.y, size, c);
    }
}

fn flashed(c: Color, flash: f32) -> Color {
    if flash > 0.0 {
        Color::new(1.0, 0.86, 0.82, c.a)
    } else {
        c
    }
}

impl World {
    /// Direction from `p` to the nearest hero, or to the right.
    pub(super) fn heading(&self, p: Vec2) -> Vec2 {
        self.nearest_hero(p).map(|h| (self.heroes[h].p.pos - p).normalize_or_zero()).filter(|d| *d != Vec2::ZERO).unwrap_or(vec2(1.0, 0.0))
    }

    // ---------------------------------------------------------------- heroes

    pub(super) fn draw_hero(&self, hero: &Hero, _others: bool) {
        let pl = &hero.p;
        let p = pl.pos;
        let hi = true;
        let skin = skin_of(hero.skin);
        let tint = |c: (u8, u8, u8)| rgba(c.0, c.1, c.2, 255);
        // A skin wins over the class look; the scarf keeps the player's colour.
        let body = skin.map(|s| tint(s.body)).unwrap_or(slot_color(hero.slot));
        if hero.downed {
            draw_ellipse(p.x + 4.0, p.y + 6.0, 20.0, 9.0, 0.2, rgba(70, 4, 6, 170));
            blob(p + vec2(0.0, 3.0), 13.0, 6.0, 0.15, shade(body, 0.8), hi);
            ball(p + vec2(11.0, 1.0), 5.0, rgba(100, 100, 110, 255), hi);
            limb(p + vec2(-4.0, 4.0), p + vec2(-14.0, 9.0), 3.0, shade(body, 0.6), hi);
            let k = (hero.revive / REVIVE_TIME).clamp(0.0, 1.0);
            draw_circle_lines(p.x, p.y, 45.0, 1.0, Color::new(1.0, 0.7, 0.3, 0.3));
            if k > 0.0 {
                draw_arc(p.x, p.y, 40, 44.0, -90.0, 3.0, 360.0 * k, rgba(255, 190, 110, 255));
            }
            return;
        }
        // Blessings from map relics.
        if hero.buff(Buff::Fury) > 0.0 {
            let pulse = 0.6 + 0.4 * (self.time * 8.0).sin();
            draw_circle(p.x, p.y, 24.0, Color::new(0.8, 0.05, 0.05, 0.12 * pulse));
            draw_circle_lines(p.x, p.y, 20.0, 1.5, Color::new(0.9, 0.1, 0.1, 0.5 * pulse));
        }
        if hero.buff(Buff::Aegis) > 0.0 {
            draw_circle(p.x, p.y, 26.0, Color::new(1.0, 0.85, 0.5, 0.10));
            draw_circle_lines(p.x, p.y, 26.0, 2.0, Color::new(1.0, 0.85, 0.5, 0.6));
        }
        if pl.iframes > 0.0 && (self.time * 30.0).sin() > 0.3 && pl.roll_t <= 0.0 {
            return;
        }
        draw_ellipse(p.x, p.y + 13.0, 16.0, 5.5, 0.0, rgba(0, 0, 0, 60));
        draw_ellipse(p.x, p.y + 13.0, 10.0, 3.5, 0.0, rgba(0, 0, 0, 130));
        if pl.roll_t > 0.0 {
            // Tumbling: a ball of cloak with the helm flashing round.
            let a = (1.0 - pl.roll_t / 0.3) * TAU;
            ball(p, 10.0, shade(body, 0.9), hi);
            let s = Vec2::from_angle(a) * 6.0;
            ball(p + s, 5.0, rgba(140, 140, 150, 255), hi);
            draw_arc(p.x, p.y, 16, 11.0, a.to_degrees(), 2.0, 140.0, shade(body, 0.6));
            return;
        }
        let f = pl.facing;
        // The weapon follows the aim; the body keeps facing where it walks.
        let aim = hero.aim;
        let fx = if f.x >= 0.0 { 1.0 } else { -1.0 };
        let moving = pl.moving;
        let walk = pl.walk;
        let bob = if moving { walk.sin().abs() * -1.5 } else { (self.time * 2.0).sin() * 0.4 };
        let p = p + vec2(0.0, bob);
        let steel = skin.map(|s| tint(s.metal)).unwrap_or(rgba(128, 128, 138, 255));
        let dark = shade(body, 0.55);

        // Cape behind, trailing against the movement.
        let sway = if moving { -f.x * 7.0 + (walk * 0.5).sin() * 2.0 } else { (self.time * 1.5).sin() * 1.5 };
        rag(p + vec2(-7.0, -6.0), p + vec2(7.0, -6.0), 19.0 + if moving { 2.0 } else { 0.0 }, sway, shade(body, 0.7), hero.slot as f32, hi);
        // Darker lining shows where the cape turns.
        rag(p + vec2(-4.0, -5.0), p + vec2(4.0, -5.0), 15.0, sway * 1.1, shade(body, 0.45), hero.slot as f32 + 3.0, false);

        // Legs.
        let swing = if moving { walk.sin() * 4.0 } else { 0.0 };
        for (side, s) in [(-1.0, swing), (1.0, -swing)] {
            let hip = p + vec2(side * 3.5, 5.0);
            let foot = p + vec2(side * 3.5 + s * fx, 12.0 - s.abs() * 0.3);
            limb(hip, foot, 3.6, rgba(46, 40, 38, 255), hi);
            // Boots with a turned-down cuff.
            let ankle = hip.lerp(foot, 0.7);
            draw_line(ankle.x - 2.2, ankle.y, ankle.x + 2.2, ankle.y, 1.6, rgba(78, 60, 44, 255));
            ball(foot + vec2(fx * 1.0, 0.5), 2.4, rgba(30, 26, 24, 255), false);
            draw_circle(foot.x + fx * 1.8, foot.y, 0.7, rgba(90, 80, 70, 255));
        }

        // Torso: tunic, mail skirt, breastplate, belt with pouch and sheath.
        blob(p + vec2(0.0, 1.0), 8.0, 9.0, 0.0, body, hi);
        for row in 0..2 {
            for k in 0..6 {
                let q = p + vec2(-5.0 + k as f32 * 2.0 + row as f32, 6.5 + row as f32 * 1.8);
                draw_circle(q.x, q.y, 0.8, rgba(110, 110, 118, 255));
            }
        }
        blob(p + vec2(fx * 0.5, -1.0), 6.0, 5.5, 0.0, steel, hi);
        draw_line(p.x + fx * 0.5, p.y - 5.5, p.x + fx * 0.5, p.y + 3.0, 1.0, shade(steel, 1.3));
        for (dx, dy) in [(-4.0, -3.0), (4.0, -3.0), (-4.5, 1.5), (4.5, 1.5)] {
            draw_circle(p.x + dx + fx * 0.5, p.y + dy, 0.7, rgba(70, 70, 76, 255));
        }
        draw_line(p.x - 7.0, p.y + 5.0, p.x + 7.0, p.y + 5.0, 2.0, rgba(60, 40, 26, 255));
        draw_circle(p.x, p.y + 5.0, 1.4, rgba(190, 160, 90, 255));
        draw_rectangle(p.x - fx * 6.5 - 1.5, p.y + 5.0, 3.0, 3.5, rgba(74, 52, 34, 255));
        draw_line(p.x + fx * 6.0, p.y + 5.0, p.x + fx * 8.0, p.y + 10.0, 1.6, rgba(40, 30, 24, 255));
        // Collar: a scarf in the player's colour.
        draw_ellipse(p.x, p.y - 6.0, 5.5, 2.0, 0.0, shade(slot_color(hero.slot), 1.2));
        // Pauldrons.
        ball(p + vec2(-7.0, -4.0), 3.2, shade(steel, 0.9), hi);
        ball(p + vec2(7.0, -4.0), 3.2, shade(steel, 0.9), hi);

        // Weapon arm and the class weapon.
        let hand = p + vec2(fx * 6.0, -3.0) + aim * 5.0 + vec2(0.0, 2.0 + swing * 0.2);
        limb(p + vec2(fx * 6.0, -3.0), hand, 3.2, dark, hi);
        let afx = if aim.x >= 0.0 { 1.0 } else { -1.0 };
        self.draw_held_weapon(hero, hand, aim, afx);
        ball(hand, 1.9, rgba(110, 110, 118, 255), false);
        // Off arm.
        limb(p + vec2(-fx * 6.0, -3.0), p + vec2(-fx * 8.0, 4.0 - swing * 0.2), 3.0, dark, hi);

        // Head.
        let head = p + vec2(fx * 0.5, -10.0);
        match skin.map(|s| s.helm) {
            Some(helm) => self.draw_skin_helm(helm, head, fx, steel, hi),
            None => self.draw_helm(hero.class, head, fx, body, hi),
        }
        // A hero called by a cat's name grows cat ears.
        let catlike = |w: &str| ["cat", "kitty", "kitten", "meow", "neko"].iter().any(|n| w.eq_ignore_ascii_case(n));
        if hero.name.split(|c: char| !c.is_ascii_alphanumeric()).any(catlike) {
            let ear = rgba(30, 26, 32, 255);
            for s in [-1.0, 1.0] {
                let base = head + vec2(s * 4.0, -5.0);
                draw_triangle(base + vec2(-2.6, 1.0), base + vec2(2.6, 1.0), base + vec2(s * 1.5, -6.0), ear);
                draw_triangle(base + vec2(-1.2, 0.4), base + vec2(1.2, 0.4), base + vec2(s * 1.0, -3.6), rgba(240, 150, 180, 255));
            }
        }
        if hero.has_cat {
            // A little cat-shaped charm on the belt.
            draw_circle(p.x - fx * 5.0, p.y + 7.0, 1.6, rgba(20, 18, 22, 255));
            draw_circle(p.x - fx * 5.0, p.y + 6.6, 0.5, rgba(255, 220, 80, 255));
        }
    }

    fn draw_helm(&self, class: Class, head: Vec2, fx: f32, body: Color, hi: bool) {
        let steel = rgba(122, 122, 132, 255);
        let slit = rgba(10, 10, 12, 255);
        match class {
            Class::BucketKnight => {
                draw_rectangle(head.x - 7.5, head.y - 8.0, 15.0, 15.0, OUTLINE);
                draw_rectangle(head.x - 6.5, head.y - 7.0, 13.0, 13.0, rgba(140, 140, 150, 255));
                draw_rectangle(head.x - 6.5, head.y - 7.0, 4.0, 13.0, rgba(165, 165, 175, 255));
                draw_rectangle(head.x - 8.0, head.y - 8.5, 16.0, 3.0, rgba(160, 160, 170, 255));
                draw_line(head.x - 5.0 + fx * 2.0, head.y - 1.0, head.x + 5.0 + fx * 2.0, head.y - 1.0, 2.0, slit);
                draw_arc(head.x, head.y - 8.0, 16, 7.0, 180.0, 1.2, 180.0, rgba(110, 110, 115, 255));
            }
            Class::Reaper | Class::GraveHunter | Class::BellAcolyte | Class::CinderAdept => {
                // Hood with a shadowed face.
                let hood = match class {
                    Class::Reaper => rgba(26, 24, 28, 255),
                    Class::GraveHunter => rgba(56, 50, 40, 255),
                    Class::BellAcolyte => rgba(120, 104, 70, 255),
                    _ => rgba(90, 30, 24, 255),
                };
                draw_triangle(head + vec2(-7.0, 2.0), head + vec2(7.0, 2.0), head + vec2(-fx * 3.0, -11.0), hood);
                ball(head, 6.8, hood, hi);
                draw_circle(head.x + fx * 1.8, head.y + 0.8, 4.4, rgba(8, 6, 8, 255));
                let eye = match class {
                    Class::Reaper => rgba(180, 230, 255, 255),
                    Class::CinderAdept => rgba(255, 140, 40, 255),
                    _ => rgba(255, 180, 90, 200),
                };
                eyes(head + vec2(fx * 2.4, 0.6), 1.7, 0.8, eye);
                if class == Class::GraveHunter {
                    // A bone mask.
                    draw_ellipse(head.x + fx * 2.0, head.y + 1.5, 3.4, 2.4, 0.0, rgba(200, 192, 170, 255));
                }
            }
            Class::Hexblade => {
                // A tall, bent witch-hunter hat.
                ball(head, 6.0, rgba(60, 52, 60, 255), hi);
                draw_ellipse(head.x, head.y - 3.0, 10.0, 2.6, 0.0, rgba(30, 22, 36, 255));
                draw_triangle(head + vec2(-5.0, -3.0), head + vec2(5.0, -3.0), head + vec2(-fx * 6.0, -17.0), rgba(36, 26, 44, 255));
                draw_line(head.x - 5.0, head.y - 4.0, head.x + 5.0, head.y - 4.0, 1.5, rgba(170, 60, 210, 255));
                eyes(head + vec2(fx * 2.0, 1.0), 1.8, 0.8, rgba(220, 120, 255, 255));
            }
            Class::BloodKnight => {
                ball(head, 6.8, rgba(110, 20, 26, 255), hi);
                draw_triangle(head + vec2(-5.0, -4.0), head + vec2(-3.0, -5.0), head + vec2(-8.0, -12.0), rgba(200, 190, 170, 255));
                draw_triangle(head + vec2(5.0, -4.0), head + vec2(3.0, -5.0), head + vec2(8.0, -12.0), rgba(200, 190, 170, 255));
                draw_line(head.x - 4.0 + fx * 2.0, head.y, head.x + 4.0 + fx * 2.0, head.y, 1.8, slit);
                draw_circle(head.x + fx * 3.0, head.y, 1.0, rgba(255, 60, 40, 255));
            }
            Class::Arbalist => {
                ball(head, 6.2, rgba(150, 120, 100, 255), hi);
                draw_ellipse(head.x, head.y - 3.5, 11.0, 3.0, 0.0, rgba(50, 40, 32, 255));
                ball(head + vec2(0.0, -6.0), 4.5, rgba(60, 48, 38, 255), false);
                draw_line(head.x + fx * 3.0, head.y - 8.0, head.x + fx * 9.0, head.y - 14.0, 1.5, rgba(160, 30, 30, 255));
                eyes(head + vec2(fx * 2.0, 0.5), 1.6, 0.7, rgba(30, 20, 20, 255));
            }
            Class::Wanderer => {
                // Kettle helm with a visor slit and a red crest.
                ball(head, 6.8, steel, hi);
                draw_ellipse(head.x, head.y - 1.0, 9.5, 2.4, 0.0, shade(steel, 0.8));
                draw_line(head.x - 4.5 + fx * 2.0, head.y + 1.0, head.x + 4.5 + fx * 2.0, head.y + 1.0, 1.6, slit);
                draw_circle(head.x + fx * 3.0, head.y + 1.0, 0.9, rgba(255, 160, 80, 255));
                let _ = body;
                draw_triangle(head + vec2(-1.5, -6.0), head + vec2(1.5, -6.0), head + vec2(-fx * 8.0, -10.0), rgba(140, 26, 24, 255));
            }
        }
    }

    /// Head pieces that only come with skins.
    fn draw_skin_helm(&self, helm: Helm, head: Vec2, fx: f32, metal: Color, hi: bool) {
        let t = self.time;
        let fl = 0.85 + 0.15 * (t * 9.0).sin();
        let slit = rgba(10, 10, 12, 255);
        match helm {
            Helm::Class(c) => self.draw_helm(c, head, fx, metal, hi),
            Helm::PlagueMask => {
                ball(head, 6.4, rgba(30, 26, 28, 255), hi);
                draw_triangle(head + vec2(fx * 2.0, -1.0), head + vec2(fx * 2.0, 3.6), head + vec2(fx * 13.0, 4.5), rgba(205, 190, 150, 255));
                draw_line(head.x + fx * 2.0, head.y + 1.3, head.x + fx * 12.0, head.y + 4.2, 0.8, rgba(120, 100, 70, 255));
                draw_circle(head.x + fx * 2.6, head.y - 1.2, 2.0, rgba(70, 56, 40, 255));
                draw_circle(head.x + fx * 2.6, head.y - 1.2, 1.1, rgba(200, 230, 255, 255));
                draw_ellipse(head.x, head.y - 5.0, 11.5, 2.6, 0.0, rgba(18, 16, 18, 255));
                draw_rectangle(head.x - 5.0, head.y - 11.5, 10.0, 6.5, rgba(24, 22, 24, 255));
                draw_line(head.x - 5.0, head.y - 6.5, head.x + 5.0, head.y - 6.5, 1.2, rgba(110, 80, 50, 255));
            }
            Helm::Pumpkin => {
                let orange = rgba(222, 112, 30, 255);
                ball(head, 7.6, orange, hi);
                for dx in [-3.5, 0.0, 3.5] {
                    draw_ellipse_lines(head.x + dx * 0.6, head.y, 2.6, 7.0, 0.0, 0.8, shade(orange, 0.7));
                }
                draw_line(head.x, head.y - 7.0, head.x + fx * 1.8, head.y - 10.5, 2.2, rgba(70, 100, 40, 255));
                let glow = Color::new(1.0, 0.85, 0.3, fl);
                draw_triangle(head + vec2(fx * 1.0 - 2.0, -1.0), head + vec2(fx * 1.0, -3.5), head + vec2(fx * 1.0 + 0.5, -0.5), glow);
                draw_triangle(head + vec2(fx * 4.0 - 1.5, -1.0), head + vec2(fx * 4.0, -3.5), head + vec2(fx * 4.0 + 1.0, -0.5), glow);
                for k in 0..4 {
                    let x = head.x + fx * (-1.0 + k as f32 * 1.6);
                    draw_triangle(vec2(x, head.y + 2.0), vec2(x + 1.6 * fx, head.y + 2.0), vec2(x + 0.8 * fx, head.y + if k % 2 == 0 { 4.0 } else { 3.0 }), glow);
                }
            }
            Helm::SkullCrown => {
                let bone = rgba(225, 215, 190, 255);
                ball(head, 6.6, bone, hi);
                draw_circle(head.x + fx * 1.0, head.y, 1.8, slit);
                draw_circle(head.x + fx * 4.2, head.y, 1.6, slit);
                draw_line(head.x + fx * 0.5, head.y + 4.0, head.x + fx * 5.0, head.y + 4.0, 1.4, slit);
                for k in 0..3 {
                    let x = head.x - 4.0 + k as f32 * 4.0;
                    draw_triangle(vec2(x - 2.0, head.y - 5.0), vec2(x + 2.0, head.y - 5.0), vec2(x, head.y - 10.0), rgba(230, 190, 80, 255));
                }
                draw_rectangle(head.x - 6.0, head.y - 6.0, 12.0, 2.0, rgba(210, 170, 60, 255));
            }
            Helm::Antlers => {
                ball(head, 6.6, rgba(52, 84, 46, 255), hi);
                draw_circle(head.x + fx * 1.8, head.y + 0.8, 4.2, rgba(8, 10, 6, 255));
                eyes(head + vec2(fx * 2.4, 0.6), 1.7, 0.8, rgba(170, 255, 140, 255));
                let wood = rgba(150, 120, 80, 255);
                for s in [-1.0, 1.0] {
                    let a = head + vec2(s * 4.0, -5.0);
                    let b = a + vec2(s * 4.0, -8.0);
                    draw_line(a.x, a.y, b.x, b.y, 1.6, wood);
                    for k in 1..3 {
                        let m = a.lerp(b, k as f32 * 0.35);
                        draw_line(m.x, m.y, m.x + s * 3.5, m.y - 2.0, 1.2, wood);
                    }
                }
            }
            Helm::IceCrown => {
                ball(head, 6.6, rgba(200, 220, 235, 255), hi);
                draw_line(head.x - 4.5 + fx * 2.0, head.y + 1.0, head.x + 4.5 + fx * 2.0, head.y + 1.0, 1.6, rgba(40, 70, 110, 255));
                for k in 0..5 {
                    let x = head.x - 6.0 + k as f32 * 3.0;
                    let h = if k == 2 { 10.0 } else { 7.0 };
                    draw_triangle(vec2(x - 1.6, head.y - 5.0), vec2(x + 1.6, head.y - 5.0), vec2(x, head.y - 5.0 - h), rgba(170, 225, 255, 230));
                }
                draw_circle(head.x - 2.0, head.y - 9.0, 0.8 + 0.5 * (t * 3.0).sin().abs(), WHITE);
            }
            Helm::Halo => {
                let gold = rgba(240, 205, 110, 255);
                ball(head, 6.8, gold, hi);
                draw_line(head.x - 4.5 + fx * 2.0, head.y + 1.0, head.x + 4.5 + fx * 2.0, head.y + 1.0, 1.6, slit);
                draw_ellipse(head.x, head.y - 11.0, 8.0, 2.4, 0.0, Color::new(1.0, 0.9, 0.5, 0.25 * fl));
                draw_ellipse_lines(head.x, head.y - 11.0, 7.0, 2.0, 0.0, 1.6, Color::new(1.0, 0.92, 0.6, 0.95));
            }
            Helm::Moth => {
                let fuzz = rgba(205, 190, 210, 255);
                ball(head, 6.8, fuzz, hi);
                for k in 0..8 {
                    let a = k as f32 / 8.0 * TAU;
                    let q = head + Vec2::from_angle(a) * 6.6;
                    draw_circle(q.x, q.y, 1.6, fuzz);
                }
                draw_circle(head.x + fx * 1.0, head.y + 0.5, 2.2, rgba(20, 16, 24, 255));
                draw_circle(head.x + fx * 4.4, head.y + 0.5, 2.0, rgba(20, 16, 24, 255));
                for s in [-1.0, 1.0] {
                    let a = head + vec2(s * 2.0, -6.0);
                    let b = a + vec2(s * 5.0, -6.0 + (t * 4.0 + s).sin());
                    draw_line(a.x, a.y, b.x, b.y, 1.0, rgba(120, 100, 90, 255));
                    for k in 1..4 {
                        let m = a.lerp(b, k as f32 * 0.25);
                        draw_line(m.x, m.y, m.x + s * 1.5, m.y + 1.5, 0.8, rgba(150, 130, 120, 255));
                    }
                }
            }
            Helm::Jester => {
                ball(head, 6.2, rgba(235, 225, 215, 255), hi);
                eyes(head + vec2(fx * 2.2, 0.8), 1.7, 0.9, rgba(20, 20, 30, 255));
                draw_line(head.x + fx * 0.5, head.y + 3.6, head.x + fx * 4.0, head.y + 3.0, 1.0, rgba(180, 40, 60, 255));
                let sway = (t * 3.0).sin() * 1.5;
                for (s, c) in [(-1.0, rgba(120, 40, 150, 255)), (1.0, rgba(60, 150, 90, 255))] {
                    let tip = head + vec2(s * 10.0, -4.0 + sway * s);
                    draw_triangle(head + vec2(0.0, -6.0), head + vec2(s * 6.0, -3.0), tip, c);
                    draw_circle(tip.x, tip.y, 1.8, rgba(240, 200, 80, 255));
                }
            }
            Helm::Flame => {
                ball(head, 6.0, rgba(30, 18, 16, 255), hi);
                for k in 0..5 {
                    let x = head.x - 5.0 + k as f32 * 2.5;
                    let h = 8.0 + (t * 11.0 + k as f32 * 1.7).sin() * 3.0;
                    draw_triangle(vec2(x - 2.2, head.y - 3.0), vec2(x + 2.2, head.y - 3.0), vec2(x + (t * 6.0 + k as f32).sin(), head.y - 3.0 - h), rgba(255, 120, 30, 220));
                    draw_triangle(vec2(x - 1.2, head.y - 3.0), vec2(x + 1.2, head.y - 3.0), vec2(x, head.y - 3.0 - h * 0.6), rgba(255, 220, 110, 240));
                }
                eyes(head + vec2(fx * 2.4, 0.6), 1.8, 1.0, rgba(255, 210, 90, 255));
            }
            Helm::CatHood => {
                let fur = rgba(26, 24, 30, 255);
                for s in [-1.0, 1.0] {
                    let base = head + vec2(s * 4.0, -4.5);
                    draw_triangle(base + vec2(-3.0, 1.0), base + vec2(3.0, 1.0), base + vec2(s * 1.5, -6.5), fur);
                    draw_triangle(base + vec2(-1.4, 0.4), base + vec2(1.4, 0.4), base + vec2(s * 1.0, -4.0), rgba(230, 140, 170, 255));
                }
                ball(head, 6.8, fur, hi);
                eyes(head + vec2(fx * 2.4, 0.4), 1.9, 1.1, rgba(250, 220, 80, 255));
                draw_circle(head.x + fx * 2.4, head.y + 2.4, 0.8, rgba(240, 140, 170, 255));
            }
            Helm::LanternHood => {
                let hood = rgba(30, 80, 82, 255);
                draw_triangle(head + vec2(-7.0, 2.0), head + vec2(7.0, 2.0), head + vec2(-fx * 3.0, -11.0), hood);
                ball(head, 6.8, hood, hi);
                draw_circle(head.x + fx * 1.8, head.y + 0.8, 4.4, rgba(6, 10, 10, 255));
                eyes(head + vec2(fx * 2.4, 0.6), 1.7, 0.8, rgba(150, 255, 230, 255));
                let lamp = head + vec2(-fx * 10.0, -6.0 + (t * 2.0).sin() * 1.5);
                draw_circle(lamp.x, lamp.y, 7.0 * fl, Color::new(0.5, 1.0, 0.9, 0.18));
                draw_rectangle(lamp.x - 2.2, lamp.y - 2.6, 4.4, 5.2, rgba(50, 60, 60, 255));
                draw_rectangle(lamp.x - 1.4, lamp.y - 1.8, 2.8, 3.6, rgba(170, 255, 230, 255));
            }
            Helm::Greathelm => {
                let plume = rgba(170, 30, 36, 255);
                for k in 0..4 {
                    let a = head + vec2(-fx * (1.0 + k as f32 * 2.2), -7.0 - (3 - k) as f32 * 0.6);
                    draw_circle(a.x, a.y, 2.6, plume);
                }
                draw_rectangle(head.x - 7.0, head.y - 7.5, 14.0, 14.5, OUTLINE);
                draw_rectangle(head.x - 6.0, head.y - 6.5, 12.0, 12.5, metal);
                draw_rectangle(head.x - 6.0, head.y - 6.5, 3.5, 12.5, shade(metal, 1.15));
                draw_line(head.x - 4.5 + fx * 1.5, head.y - 1.0, head.x + 4.5 + fx * 1.5, head.y - 1.0, 1.8, slit);
                draw_line(head.x + fx * 1.5, head.y - 1.0, head.x + fx * 1.5, head.y + 4.0, 1.4, slit);
            }
            Helm::StarHat => {
                ball(head, 6.0, rgba(40, 34, 50, 255), hi);
                eyes(head + vec2(fx * 2.2, 1.0), 1.8, 0.9, rgba(255, 240, 160, 255));
                let blue = rgba(36, 48, 120, 255);
                draw_ellipse(head.x, head.y - 3.5, 11.0, 2.8, 0.0, blue);
                draw_triangle(head + vec2(-5.5, -3.5), head + vec2(5.5, -3.5), head + vec2(-fx * 7.0, -19.0), blue);
                for (k, (dx, dy)) in [(-2.0, -7.0), (1.5, -10.0), (-3.5, -13.0), (0.5, -5.5)].iter().enumerate() {
                    let tw = 0.5 + 0.5 * (t * 3.0 + k as f32 * 1.9).sin();
                    draw_circle(head.x + dx, head.y + dy, 0.6 + tw * 0.6, Color::new(1.0, 0.95, 0.7, 0.5 + 0.5 * tw));
                }
            }
            Helm::LastCrown => {
                draw_circle(head.x, head.y, 11.0 * fl, Color::new(1.0, 0.9, 0.6, 0.12));
                ball(head, 6.8, rgba(250, 245, 235, 255), hi);
                draw_line(head.x - 4.5 + fx * 2.0, head.y + 1.0, head.x + 4.5 + fx * 2.0, head.y + 1.0, 1.6, rgba(255, 170, 60, 255));
                for k in 0..5 {
                    let x = head.x - 6.0 + k as f32 * 3.0;
                    let h = 6.0 + (t * 10.0 + k as f32 * 2.1).sin() * 2.0 + if k == 2 { 3.0 } else { 0.0 };
                    draw_triangle(vec2(x - 1.6, head.y - 5.0), vec2(x + 1.6, head.y - 5.0), vec2(x, head.y - 5.0 - h), rgba(255, 220, 120, 240));
                    draw_triangle(vec2(x - 0.8, head.y - 5.0), vec2(x + 0.8, head.y - 5.0), vec2(x, head.y - 5.0 - h * 0.6), WHITE);
                }
            }
            Helm::Strawberry => {
                ball(head, 6.0, rgba(240, 220, 205, 255), hi);
                eyes(head + vec2(fx * 2.2, 1.5), 1.7, 0.9, rgba(30, 20, 20, 255));
                let red = rgba(220, 40, 60, 255);
                draw_circle(head.x, head.y - 4.5, 7.2, OUTLINE);
                draw_circle(head.x, head.y - 4.5, 6.4, red);
                draw_rectangle(head.x - 7.2, head.y - 4.0, 14.4, 4.5, rgba(240, 220, 205, 255));
                for (dx, dy) in [(-3.5, -6.0), (0.0, -8.5), (3.5, -6.0), (-1.5, -5.0), (2.0, -9.5), (-4.5, -8.5)] {
                    draw_circle(head.x + dx, head.y + dy, 0.7, rgba(255, 230, 100, 255));
                }
                for k in 0..4 {
                    let a = -PI * 0.5 + (k as f32 - 1.5) * 0.6;
                    let tip = head + vec2(0.0, -11.0) + Vec2::from_angle(a) * 4.0;
                    draw_triangle(head + vec2(-1.0, -10.5), head + vec2(1.0, -10.5), tip, rgba(90, 170, 70, 255));
                }
            }
        }
    }

    /// The class weapon in the hero's hand.
    fn draw_held_weapon(&self, hero: &Hero, hand: Vec2, f: Vec2, fx: f32) {
        let kind = hero.weapons.first().map(|w| w.kind).unwrap_or(hero.class.weapon());
        let metal = rgba(175, 175, 185, 255);
        let wood = rgba(80, 58, 38, 255);
        let up = vec2(fx * 0.35, -1.0).normalize();
        match kind {
            WeaponKind::Blade | WeaponKind::Daggers => {
                let len = if kind == WeaponKind::Blade { 18.0 } else { 10.0 };
                let dir = (f * 0.6 + up * 0.8).normalize();
                let tip = hand + dir * len;
                draw_line(hand.x, hand.y, tip.x, tip.y, 3.4, OUTLINE);
                draw_line(hand.x, hand.y, tip.x, tip.y, 2.4, metal);
                draw_line(hand.x, hand.y, tip.x, tip.y, 0.8, rgba(230, 230, 240, 255));
                let guard = vec2(-dir.y, dir.x) * 3.5;
                draw_line(hand.x - guard.x, hand.y - guard.y, hand.x + guard.x, hand.y + guard.y, 2.0, rgba(120, 96, 50, 255));
            }
            WeaponKind::Scythe => {
                let top = hand + up * 22.0;
                let bottom = hand - up * 10.0;
                draw_line(bottom.x, bottom.y, top.x, top.y, 2.6, wood);
                let a = (up.y.atan2(up.x)).to_degrees();
                draw_arc(top.x, top.y, 20, 9.0, a + if fx > 0.0 { 0.0 } else { 90.0 }, 3.0, 110.0, metal);
            }
            WeaponKind::Axe => {
                let top = hand + up * 14.0;
                draw_line(hand.x, hand.y, top.x, top.y, 2.6, wood);
                draw_ellipse(top.x + fx * 3.0, top.y + 1.0, 4.0, 6.0, 0.0, metal);
            }
            WeaponKind::Lance | WeaponKind::Spear => {
                let tip = hand + (f + up * 0.6).normalize() * 24.0;
                let butt = hand - (f + up * 0.6).normalize() * 8.0;
                draw_line(butt.x, butt.y, tip.x, tip.y, 2.4, if kind == WeaponKind::Lance { rgba(90, 10, 14, 255) } else { wood });
                let d = (tip - hand).normalize();
                let side = vec2(-d.y, d.x) * 3.0;
                let col = if kind == WeaponKind::Lance { rgba(200, 30, 40, 255) } else { metal };
                draw_triangle(tip + d * 6.0, tip + side, tip - side, col);
            }
            WeaponKind::Crossbow => {
                let d = f;
                let side = vec2(-d.y, d.x);
                let front = hand + d * 9.0;
                draw_line(hand.x, hand.y, front.x, front.y, 3.0, wood);
                draw_line(front.x + side.x * 7.0, front.y + side.y * 7.0, front.x - side.x * 7.0, front.y - side.y * 7.0, 2.0, rgba(60, 60, 66, 255));
                draw_line(front.x + side.x * 7.0, front.y + side.y * 7.0, hand.x, hand.y, 0.8, rgba(200, 200, 200, 200));
                draw_line(front.x - side.x * 7.0, front.y - side.y * 7.0, hand.x, hand.y, 0.8, rgba(200, 200, 200, 200));
            }
            WeaponKind::Orb | WeaponKind::Bolt | WeaponKind::Hex => {
                // A staff with a glowing head.
                let top = hand + up * 20.0;
                let bottom = hand - up * 8.0;
                draw_line(bottom.x, bottom.y, top.x, top.y, 2.4, wood);
                let glow_col = match kind {
                    WeaponKind::Orb => rgba(255, 130, 40, 255),
                    WeaponKind::Hex => rgba(210, 90, 255, 255),
                    _ => rgba(150, 230, 255, 255),
                };
                let fl = 0.8 + 0.2 * (self.time * 9.0).sin();
                draw_circle(top.x, top.y, 7.0 * fl, with_alpha(glow_col, 0.2));
                draw_circle(top.x, top.y, 3.2, glow_col);
            }
            WeaponKind::Chime | WeaponKind::Lanterns => {
                // A hand bell or a lantern on a chain.
                let low = hand + vec2(0.0, 6.0 + (self.time * 3.0).sin());
                draw_line(hand.x, hand.y, low.x, low.y, 1.0, rgba(90, 90, 96, 255));
                if kind == WeaponKind::Chime {
                    draw_triangle(low + vec2(-4.0, 6.0), low + vec2(4.0, 6.0), low + vec2(0.0, -2.0), rgba(200, 170, 90, 255));
                } else {
                    draw_rectangle(low.x - 3.0, low.y, 6.0, 7.0, rgba(60, 60, 66, 255));
                    draw_rectangle(low.x - 2.0, low.y + 1.0, 4.0, 5.0, rgba(140, 210, 255, 255));
                }
            }
        }
    }

    pub(super) fn draw_name(&self, hero: &Hero) {
        let p = hero.p.pos;
        let size = 14u16;
        let w = measure_text(&hero.name, None, size, 1.0).width;
        draw_text(&hero.name, p.x - w * 0.5 + 1.0, p.y - 31.0, size as f32, Color::new(0.0, 0.0, 0.0, 0.8));
        draw_text(&hero.name, p.x - w * 0.5, p.y - 32.0, size as f32, Color::new(0.9, 0.85, 0.75, 0.9));
        let frac = (hero.p.hp / hero.stats.max_hp).clamp(0.0, 1.0);
        draw_rectangle(p.x - 14.0, p.y - 28.0, 28.0, 3.0, Color::new(0.0, 0.0, 0.0, 0.7));
        draw_rectangle(p.x - 14.0, p.y - 28.0, 28.0 * frac, 3.0, Color::new(0.7, 0.1, 0.08, 1.0));
    }

    // ---------------------------------------------------------------- enemies

    pub(super) fn draw_enemy(&self, e: &Enemy, crowded: bool) {
        if let EnemyKind::Boss(k) = e.kind {
            self.draw_boss(e, k);
            return;
        }
        let p = e.pos;
        let r = e.r;
        let a = e.alpha;
        let hi = self.detail_high && !crowded;
        let fl = e.flash;
        let t = self.time + e.phase;
        let dir = self.heading(p);
        let fx = if dir.x >= 0.0 { 1.0 } else { -1.0 };
        if !matches!(e.kind, EnemyKind::Moth | EnemyKind::Wraith) {
            // A soft shadow: a wide faint one and a tight dark core where the feet touch.
            // A big horde keeps the single shadow; nobody sees the difference there.
            if crowded {
                draw_ellipse(p.x, p.y + r * 0.95, r * 1.05, r * 0.32, 0.0, rgba(0, 0, 0, (110.0 * a) as u8));
            } else {
                draw_ellipse(p.x, p.y + r * 0.95, r * 1.35, r * 0.42, 0.0, rgba(0, 0, 0, (55.0 * a) as u8));
                draw_ellipse(p.x, p.y + r * 0.95, r * 0.85, r * 0.26, 0.0, rgba(0, 0, 0, (110.0 * a) as u8));
            }
        } else if !crowded {
            // Fliers cast a faint shadow further below them.
            draw_ellipse(p.x, p.y + r * 1.6, r * 0.8, r * 0.22, 0.0, rgba(0, 0, 0, (45.0 * a) as u8));
        }
        if e.elite {
            draw_circle(p.x, p.y, r + 8.0, rgba(230, 180, 60, 36));
            draw_circle_lines(p.x, p.y, r + 5.0, 2.0, rgba(230, 180, 60, 190));
            for k in 0..5 {
                let ang = -PI * 0.5 + (k as f32 - 2.0) * 0.32;
                let base = p + Vec2::from_angle(ang) * (r + 5.0);
                let tip = p + Vec2::from_angle(ang) * (r + 11.0);
                draw_line(base.x, base.y, tip.x, tip.y, 2.0, rgba(230, 180, 60, 220));
            }
        }
        if e.burn > 0.0 {
            draw_circle(p.x, p.y, r + 3.0, rgba(255, 110, 30, 50));
        }
        if !hi {
            self.draw_enemy_simple(e, fx);
            return;
        }
        let c = |col: Color| with_alpha(flashed(col, fl), a);
        let walk = t * 7.0;
        match e.kind {
            EnemyKind::Hollow => {
                let sway = (t * 3.0).sin() * 1.5;
                let s = walk.sin() * r * 0.3;
                for (side, sw) in [(-1.0, s), (1.0, -s)] {
                    limb(p + vec2(side * r * 0.3, r * 0.3), p + vec2(side * r * 0.35 + sw * fx, r * 0.95), r * 0.22, c(rgba(58, 52, 48, 255)), true);
                }
                // Hanging arm, then the body, then the arm reaching for you.
                limb(p + vec2(-fx * r * 0.45, -r * 0.35), p + vec2(-fx * r * 0.6, r * 0.3 + sway), r * 0.18, c(rgba(104, 94, 84, 255)), true);
                rag(p + vec2(-r * 0.6, -r * 0.55), p + vec2(r * 0.6, -r * 0.55), r * 1.15, -fx * 2.0 + sway, c(rgba(60, 54, 49, 255)), e.phase, true);
                // Ribs show through a tear in the rags.
                for k in 0..3 {
                    let y = p.y - r * 0.2 + k as f32 * r * 0.18;
                    draw_arc(p.x + fx * r * 0.1, y, 10, r * 0.22, 200.0, 1.0, 140.0, c(rgba(150, 138, 120, 255)));
                }
                let reach = p + vec2(fx * r * 1.25, -r * 0.15 + sway * 0.6);
                limb(p + vec2(fx * r * 0.45, -r * 0.4), reach, r * 0.18, c(rgba(118, 106, 94, 255)), true);
                for k in -1..=1 {
                    let tip = reach + vec2(fx * r * 0.22, k as f32 * r * 0.1);
                    draw_line(reach.x, reach.y, tip.x, tip.y, 1.0, c(rgba(130, 118, 104, 255)));
                }
                let head = p + vec2(fx * r * 0.18, -r * 0.82);
                ball(head - vec2(fx * r * 0.08, r * 0.06), r * 0.55, c(rgba(42, 37, 34, 255)), true);
                ball(head + vec2(fx * r * 0.1, r * 0.05), r * 0.38, c(rgba(112, 102, 90, 255)), false);
                draw_circle(head.x + fx * r * 0.12, head.y + r * 0.08, r * 0.22, c(rgba(20, 16, 14, 255)));
                eyes(head + vec2(fx * r * 0.15, 0.0), r * 0.13, r * 0.08, with_alpha(rgba(255, 60, 30, 255), a));
                let jaw = head + vec2(fx * r * 0.14, r * 0.25);
                draw_line(jaw.x - r * 0.12, jaw.y, jaw.x + r * 0.12, jaw.y, 1.0, c(rgba(40, 30, 26, 255)));
                let drip = (t * 1.3).fract();
                draw_line(jaw.x, jaw.y, jaw.x, jaw.y + r * 0.2 + drip * r * 0.3, 1.2, c(rgba(120, 8, 10, 255)));
            }
            EnemyKind::Ghoul => {
                // A low crawler, stretched along its heading.
                let ang = dir.to_angle();
                let fwd = dir;
                let side = vec2(-dir.y, dir.x);
                for k in 0..4 {
                    let front = k < 2;
                    let sgn = if k % 2 == 0 { 1.0 } else { -1.0 };
                    let ph = (walk + if (k == 0) || (k == 3) { 0.0 } else { PI }).sin();
                    let root = p + fwd * (if front { r * 0.45 } else { -r * 0.45 }) + side * sgn * r * 0.35;
                    let knee = root + side * sgn * r * 0.55 + fwd * ph * r * 0.25 - vec2(0.0, r * 0.25);
                    let foot = knee + side * sgn * r * 0.2 + fwd * (ph * r * 0.35 + if front { r * 0.3 } else { -r * 0.2 }) + vec2(0.0, r * 0.45);
                    limb(root, knee, r * 0.18, c(rgba(84, 98, 72, 255)), true);
                    limb(knee, foot, r * 0.14, c(rgba(96, 110, 82, 255)), true);
                    for cl in -1..=1 {
                        let tip = foot + fwd * r * 0.18 + side * cl as f32 * r * 0.08;
                        draw_line(foot.x, foot.y, tip.x, tip.y, 1.0, c(rgba(200, 196, 170, 255)));
                    }
                }
                blob(p, r * 1.05, r * 0.62, ang, c(rgba(92, 108, 80, 255)), true);
                for k in 0..4 {
                    let s = p - fwd * (r * 0.5 - k as f32 * r * 0.3) - vec2(0.0, r * 0.35);
                    draw_circle(s.x, s.y, r * 0.12, c(rgba(130, 145, 110, 255)));
                }
                for k in 0..3 {
                    let base = p + fwd * (r * 0.3 - k as f32 * r * 0.28);
                    for sgn in [-1.0, 1.0] {
                        let rib = base + side * sgn * r * 0.45;
                        draw_line(base.x, base.y, rib.x, rib.y + r * 0.1, 1.0, c(rgba(70, 82, 60, 255)));
                    }
                }
                let head = p + fwd * r * 0.95;
                ball(head, r * 0.42, c(rgba(118, 132, 100, 255)), true);
                let maw = head + fwd * r * 0.2;
                draw_circle(maw.x, maw.y, r * 0.2, c(rgba(110, 10, 12, 255)));
                eyes(head - vec2(0.0, r * 0.12), r * 0.15, r * 0.08, with_alpha(rgba(220, 230, 80, 255), a));
            }
            EnemyKind::Moth => {
                let flap = (t * 22.0).sin();
                let wing = c(rgba(176, 168, 140, 220));
                let dark = c(rgba(110, 100, 80, 220));
                for s in [-1.0, 1.0] {
                    let span = r * (0.9 + 0.5 * flap.abs());
                    let up = p + vec2(s * span, -r * 0.45);
                    let lo = p + vec2(s * span * 0.7, r * 0.6);
                    draw_triangle(p, up, p + vec2(s * span * 1.1, r * 0.15), wing);
                    draw_triangle(p, lo, p + vec2(s * span * 0.9, r * 0.2), dark);
                    draw_circle(up.x - s * span * 0.35, up.y + r * 0.25, r * 0.22, c(rgba(60, 50, 40, 255)));
                    draw_circle(up.x - s * span * 0.35, up.y + r * 0.25, r * 0.1, c(rgba(230, 200, 120, 255)));
                    draw_line(p.x, p.y - r * 0.5, p.x + s * r * 0.5, p.y - r * 1.2, 0.8, c(rgba(80, 70, 50, 255)));
                }
                blob(p, r * 0.3, r * 0.75, 0.0, c(rgba(80, 70, 52, 255)), true);
                for k in 0..3 {
                    let y = p.y + k as f32 * r * 0.22;
                    draw_line(p.x - r * 0.26, y, p.x + r * 0.26, y, 0.8, c(rgba(50, 42, 30, 255)));
                }
                for k in 0..8 {
                    let q = p - vec2(0.0, r * 0.35) + Vec2::from_angle(k as f32 / 8.0 * TAU) * r * 0.28;
                    draw_circle(q.x, q.y, r * 0.1, c(rgba(200, 186, 150, 255)));
                }
                eyes(p - vec2(0.0, r * 0.5), r * 0.14, r * 0.12, with_alpha(rgba(255, 220, 120, 255), a));
            }
            EnemyKind::Archer => {
                let bone = c(rgba(196, 190, 172, 255));
                let s = walk.sin() * r * 0.25;
                for (side, sw) in [(-1.0, s), (1.0, -s)] {
                    limb(p + vec2(side * r * 0.25, r * 0.35), p + vec2(side * r * 0.3 + sw * fx, r * 0.95), r * 0.13, bone, true);
                }
                // A rotted loincloth, pelvis, spine, ribcage.
                rag(p + vec2(-r * 0.35, r * 0.3), p + vec2(r * 0.35, r * 0.3), r * 0.5, -fx * 1.0, c(rgba(70, 52, 40, 255)), e.phase, false);
                draw_ellipse(p.x, p.y + r * 0.35, r * 0.4, r * 0.18, 0.0, bone);
                draw_line(p.x, p.y + r * 0.35, p.x, p.y - r * 0.6, r * 0.14, bone);
                for k in 0..3 {
                    let y = p.y - r * 0.45 + k as f32 * r * 0.22;
                    draw_arc(p.x, y, 12, r * (0.38 - k as f32 * 0.05), 200.0, 1.4, 140.0, bone);
                }
                // A quiver on the back.
                draw_line(p.x - fx * r * 0.4, p.y - r * 0.9, p.x - fx * r * 0.6, p.y + r * 0.2, r * 0.25, c(rgba(80, 56, 36, 255)));
                let head = p + vec2(0.0, -r * 0.95);
                ball(head, r * 0.42, bone, true);
                draw_circle(head.x - r * 0.15 + fx * r * 0.08, head.y, r * 0.11, c(rgba(20, 16, 14, 255)));
                draw_circle(head.x + r * 0.15 + fx * r * 0.08, head.y, r * 0.11, c(rgba(20, 16, 14, 255)));
                draw_line(head.x - r * 0.15, head.y + r * 0.25, head.x + r * 0.15, head.y + r * 0.25, 1.0, c(rgba(60, 50, 40, 255)));
                for k in 0..4 {
                    let x = head.x - r * 0.12 + k as f32 * r * 0.08;
                    draw_line(x, head.y + r * 0.2, x, head.y + r * 0.3, 0.8, c(rgba(60, 50, 40, 255)));
                }
                draw_line(head.x - r * 0.05, head.y - r * 0.4, head.x + r * 0.08, head.y - r * 0.18, 0.8, c(rgba(80, 70, 60, 255)));
                // Bow drawn toward the target with an arrow on the string.
                let hand = p + dir * r * 0.9 - vec2(0.0, r * 0.2);
                limb(p - vec2(0.0, r * 0.45), hand, r * 0.12, bone, true);
                let ang = dir.to_angle().to_degrees();
                draw_arc(hand.x, hand.y, 16, r * 0.8, ang - 65.0, 2.0, 130.0, c(rgba(110, 76, 44, 255)));
                let side = vec2(-dir.y, dir.x) * r * 0.75;
                let string_mid = hand - dir * r * 0.15;
                let bow_a = hand + Vec2::from_angle((ang - 65.0).to_radians()) * r * 0.8;
                let bow_b = hand + Vec2::from_angle((ang + 65.0).to_radians()) * r * 0.8;
                draw_line(bow_a.x, bow_a.y, string_mid.x, string_mid.y, 0.6, c(rgba(200, 200, 200, 200)));
                draw_line(bow_b.x, bow_b.y, string_mid.x, string_mid.y, 0.6, c(rgba(200, 200, 200, 200)));
                let tip = string_mid + dir * r * 1.2;
                draw_line(string_mid.x, string_mid.y, tip.x, tip.y, 1.0, c(rgba(160, 140, 110, 255)));
                let _ = side;
            }
            EnemyKind::Bloat => {
                let pulse = 1.0 + (t * 3.0).sin() * 0.06;
                for s in [-1.0, 1.0] {
                    limb(p + vec2(s * r * 0.4, r * 0.6), p + vec2(s * r * 0.5, r * 1.0), r * 0.18, c(rgba(80, 84, 54, 255)), true);
                }
                ball(p, r * pulse, c(rgba(96, 102, 64, 255)), true);
                for (dx, dy, rr) in [(-0.45, -0.3, 0.32), (0.4, 0.3, 0.25), (-0.2, 0.45, 0.2)] {
                    draw_circle(p.x + dx * r, p.y + dy * r, rr * r, c(rgba(76, 80, 48, 255)));
                }
                // Veins.
                for k in 0..4 {
                    let a0 = e.phase + k as f32 * 1.6;
                    let from = p + Vec2::from_angle(a0) * r * 0.2;
                    let to = p + Vec2::from_angle(a0 + 0.4) * r * 0.85;
                    draw_line(from.x, from.y, to.x, to.y, 1.0, c(rgba(110, 40, 50, 200)));
                }
                for (dx, dy) in [(0.5, -0.45), (-0.55, 0.2), (0.15, -0.7)] {
                    draw_circle(p.x + dx * r, p.y + dy * r, r * 0.12, c(rgba(200, 190, 90, 255)));
                    draw_circle(p.x + dx * r - 1.0, p.y + dy * r - 1.0, r * 0.05, c(rgba(255, 250, 200, 255)));
                }
                stitches(p + vec2(-r * 0.6, -r * 0.1), p + vec2(-r * 0.15, r * 0.35), 5, c(rgba(40, 30, 20, 255)));
                stitches(p + vec2(r * 0.25, -r * 0.65), p + vec2(r * 0.65, -r * 0.2), 4, c(rgba(40, 30, 20, 255)));
                // Flies.
                for k in 0..4 {
                    let ang = t * (3.0 + k as f32) + k as f32 * 1.7;
                    let q = p + Vec2::from_angle(ang) * r * (1.2 + 0.15 * (t * 5.0 + k as f32).sin()) - vec2(0.0, r * 0.3);
                    draw_circle(q.x, q.y, 1.1, rgba(20, 20, 20, (230.0 * a) as u8));
                }
                // A wide mouth full of teeth and one big eye.
                let mouth = p + vec2(fx * r * 0.3, r * 0.25);
                draw_ellipse(mouth.x, mouth.y, r * 0.38, r * 0.18, 0.0, c(rgba(60, 6, 8, 255)));
                for k in 0..5 {
                    let x = mouth.x - r * 0.3 + k as f32 * r * 0.15;
                    draw_triangle(vec2(x, mouth.y - r * 0.15), vec2(x + r * 0.06, mouth.y - r * 0.15), vec2(x + r * 0.03, mouth.y - r * 0.02), c(rgba(220, 210, 180, 255)));
                }
                let eye = p + vec2(fx * r * 0.2, -r * 0.35);
                ball(eye, r * 0.2, c(rgba(230, 220, 190, 255)), false);
                draw_circle(eye.x + fx * r * 0.06, eye.y, r * 0.09, with_alpha(rgba(255, 60, 30, 255), a));
            }
            EnemyKind::Wraith => {
                let cloak = Color::new(0.32, 0.42, 0.7, 0.5 * a);
                let back = -dir;
                // Wisps trailing behind.
                for k in 0..5 {
                    let wob = (t * 4.0 + k as f32).sin() * r * 0.3;
                    let q = p + back * (r * 0.6 + k as f32 * r * 0.35) + vec2(-back.y, back.x) * wob;
                    draw_circle(q.x, q.y, r * (0.75 - k as f32 * 0.12), with_alpha(cloak, 0.7 - k as f32 * 0.12));
                }
                rag(p + vec2(-r * 0.7, -r * 0.5), p + vec2(r * 0.7, -r * 0.5), r * 1.3, back.x * r * 0.6, flashed(cloak, fl), e.phase, false);
                for s in [-1.0, 1.0] {
                    let hand = p + vec2(s * r * 0.95, r * 0.1 + (t * 3.0 + s).sin() * r * 0.15);
                    draw_line(p.x + s * r * 0.4, p.y - r * 0.3, hand.x, hand.y, r * 0.15, with_alpha(cloak, 1.2));
                    for k in -1..=1 {
                        draw_line(hand.x, hand.y, hand.x + s * r * 0.25, hand.y + k as f32 * r * 0.12, 1.0, Color::new(0.8, 0.9, 1.0, 0.6 * a));
                    }
                }
                // The shackles it died in still hang from its wrists.
                for s in [-1.0, 1.0] {
                    let wrist = p + vec2(s * r * 0.85, r * 0.1);
                    draw_circle_lines(wrist.x, wrist.y, r * 0.12, 1.4, Color::new(0.45, 0.45, 0.5, 0.8 * a));
                    let end = wrist + vec2(s * r * 0.1, r * 0.9 + (t * 2.0 + s).sin() * r * 0.15);
                    chain(wrist, end, Color::new(0.4, 0.4, 0.45, 0.7 * a));
                }
                let head = p + vec2(0.0, -r * 0.65);
                draw_triangle(head + vec2(-r * 0.55, r * 0.3), head + vec2(r * 0.55, r * 0.3), head + vec2(0.0, -r * 0.75), flashed(Color::new(0.22, 0.3, 0.55, 0.8 * a), fl));
                draw_circle(head.x, head.y + r * 0.05, r * 0.32, Color::new(0.0, 0.0, 0.02, 0.9 * a));
                eyes(head + vec2(fx * r * 0.06, 0.0), r * 0.13, r * 0.09, Color::new(0.85, 0.95, 1.0, a));
            }
            EnemyKind::Knight => {
                let plate = c(rgba(70, 74, 86, 255));
                let s = walk.sin() * r * 0.22;
                for (side, sw) in [(-1.0, s), (1.0, -s)] {
                    limb(p + vec2(side * r * 0.3, r * 0.35), p + vec2(side * r * 0.32 + sw * fx, r * 0.95), r * 0.25, shade(plate, 0.8), true);
                }
                rag(p + vec2(-r * 0.55, -r * 0.45), p + vec2(r * 0.55, -r * 0.45), r * 1.1, -fx * 2.0, c(rgba(90, 22, 24, 255)), e.phase, true);
                blob(p + vec2(0.0, -r * 0.05), r * 0.62, r * 0.7, 0.0, plate, true);
                draw_line(p.x, p.y - r * 0.6, p.x, p.y + r * 0.5, 1.2, c(rgba(120, 124, 136, 255)));
                for (dx, dy) in [(-0.4, -0.4), (0.4, -0.4), (-0.45, 0.2), (0.45, 0.2)] {
                    draw_circle(p.x + dx * r, p.y + dy * r, 1.0, c(rgba(150, 150, 160, 255)));
                }
                // A tabard with a faded sigil.
                draw_rectangle(p.x - r * 0.22, p.y - r * 0.1, r * 0.44, r * 0.7, c(rgba(110, 24, 26, 255)));
                draw_line(p.x, p.y, p.x, p.y + r * 0.45, 1.5, c(rgba(200, 170, 90, 255)));
                draw_line(p.x - r * 0.12, p.y + r * 0.15, p.x + r * 0.12, p.y + r * 0.15, 1.5, c(rgba(200, 170, 90, 255)));
                ball(p + vec2(-r * 0.6, -r * 0.45), r * 0.3, shade(plate, 1.1), true);
                ball(p + vec2(r * 0.6, -r * 0.45), r * 0.3, shade(plate, 1.1), true);
                let head = p + vec2(fx * r * 0.05, -r * 0.95);
                ball(head, r * 0.42, c(rgba(96, 100, 112, 255)), true);
                draw_line(head.x - r * 0.25 + fx * r * 0.1, head.y, head.x + r * 0.25 + fx * r * 0.1, head.y, 2.0, c(rgba(10, 10, 12, 255)));
                for k in 0..3 {
                    draw_circle(head.x + fx * r * 0.1 - r * 0.1 + k as f32 * r * 0.1, head.y + r * 0.18, 0.7, c(rgba(10, 10, 12, 255)));
                }
                draw_line(head.x, head.y - r * 0.42, head.x, head.y - r * 0.05, 1.0, c(rgba(140, 144, 156, 255)));
                draw_circle(head.x + fx * r * 0.15, head.y, 1.2, with_alpha(rgba(255, 70, 40, 255), a));
                draw_triangle(head + vec2(-r * 0.1, -r * 0.35), head + vec2(r * 0.1, -r * 0.35), head + vec2(-fx * r * 0.7, -r * 0.8), c(rgba(150, 26, 26, 255)));
                // Sword raised behind, shield in front.
                let hilt = p + vec2(-fx * r * 0.7, -r * 0.2);
                let tip = hilt + vec2(-fx * r * 0.3, -r * 1.3);
                draw_line(hilt.x, hilt.y, tip.x, tip.y, 3.2, OUTLINE);
                draw_line(hilt.x, hilt.y, tip.x, tip.y, 2.2, c(rgba(180, 180, 190, 255)));
                let sh = p + vec2(fx * r * 0.75, r * 0.05);
                draw_triangle(sh + vec2(-r * 0.32, -r * 0.5), sh + vec2(r * 0.32, -r * 0.5), sh + vec2(0.0, r * 0.65), OUTLINE);
                draw_triangle(sh + vec2(-r * 0.27, -r * 0.45), sh + vec2(r * 0.27, -r * 0.45), sh + vec2(0.0, r * 0.57), c(rgba(84, 70, 48, 255)));
                draw_line(sh.x, sh.y - r * 0.4, sh.x, sh.y + r * 0.45, 1.5, c(rgba(150, 120, 60, 255)));
                draw_line(sh.x - r * 0.22, sh.y - r * 0.12, sh.x + r * 0.22, sh.y - r * 0.12, 1.5, c(rgba(150, 120, 60, 255)));
                ball(sh - vec2(0.0, r * 0.12), r * 0.09, c(rgba(160, 150, 120, 255)), false);
                ball(hilt, r * 0.12, c(rgba(90, 92, 100, 255)), false);
            }
            EnemyKind::Cultist => {
                // A hooded zealot sprinting with a bomb held high.
                let s = (t * 11.0).sin() * r * 0.35;
                for (side, sw) in [(-1.0, s), (1.0, -s)] {
                    limb(p + vec2(side * r * 0.25, r * 0.3), p + vec2(side * r * 0.3 + sw * fx, r * 0.95), r * 0.17, c(rgba(50, 30, 28, 255)), true);
                }
                rag(p + vec2(-r * 0.55, -r * 0.5), p + vec2(r * 0.55, -r * 0.5), r * 1.15, -fx * r * 0.5, c(rgba(110, 22, 22, 255)), e.phase, true);
                let head = p + vec2(fx * r * 0.25, -r * 0.8);
                draw_triangle(head + vec2(-r * 0.45, r * 0.25), head + vec2(r * 0.45, r * 0.25), head + vec2(-fx * r * 0.3, -r * 0.7), c(rgba(90, 16, 16, 255)));
                ball(head, r * 0.4, c(rgba(90, 16, 16, 255)), true);
                draw_circle(head.x + fx * r * 0.12, head.y + r * 0.05, r * 0.25, rgba(6, 4, 4, (255.0 * a) as u8));
                eyes(head + vec2(fx * r * 0.14, 0.0), r * 0.1, r * 0.07, with_alpha(rgba(255, 200, 80, 255), a));
                let hand = p + vec2(fx * r * 0.35, -r * 1.35);
                limb(p + vec2(fx * r * 0.3, -r * 0.4), hand, r * 0.15, c(rgba(150, 120, 100, 255)), true);
                let lit = e.fuse > 0.0 || fl > 0.0;
                let bomb = hand + vec2(0.0, -r * 0.3);
                if lit {
                    draw_circle(bomb.x, bomb.y, r * 1.1, rgba(255, 60, 20, (60.0 * a) as u8));
                }
                ball(bomb, r * 0.42, if lit { rgba(150, 30, 20, 255) } else { rgba(30, 28, 32, 255) }, true);
                let wick = bomb + vec2(r * 0.15, -r * 0.5);
                draw_line(bomb.x + r * 0.1, bomb.y - r * 0.35, wick.x, wick.y, 1.2, rgba(150, 130, 100, 255));
                let spark = 0.7 + 0.3 * (t * 30.0).sin();
                draw_circle(wick.x, wick.y, 2.4 * spark, rgba(255, 220, 120, (255.0 * a) as u8));
            }
            EnemyKind::Shieldbearer => {
                let plate = c(rgba(64, 66, 74, 255));
                let s = walk.sin() * r * 0.15;
                for (side, sw) in [(-1.0, s), (1.0, -s)] {
                    limb(p + vec2(side * r * 0.3, r * 0.35), p + vec2(side * r * 0.32 + sw * fx, r * 0.95), r * 0.26, shade(plate, 0.8), true);
                }
                blob(p, r * 0.6, r * 0.7, 0.0, plate, true);
                let head = p + vec2(-dir.x * r * 0.15, -r * 0.85);
                ball(head, r * 0.4, c(rgba(84, 86, 96, 255)), true);
                for k in 0..3 {
                    let ang = -PI * 0.5 + (k as f32 - 1.0) * 0.5;
                    let b0 = head + Vec2::from_angle(ang) * r * 0.35;
                    let b1 = head + Vec2::from_angle(ang) * r * 0.7;
                    draw_line(b0.x, b0.y, b1.x, b1.y, 2.0, c(rgba(150, 150, 160, 255)));
                }
                eyes(head + dir * r * 0.12, r * 0.12, 1.2, with_alpha(rgba(255, 90, 40, 255), a));
                // A tower shield held square towards its target.
                let side = dir.perp();
                let front = p + dir * r * 0.75;
                let (h, wd) = (r * 1.25, r * 0.9);
                let corners = [
                    front + side * wd - vec2(0.0, h * 0.6),
                    front - side * wd - vec2(0.0, h * 0.6),
                    front - side * wd + vec2(0.0, h * 0.5),
                    front + side * wd + vec2(0.0, h * 0.5),
                ];
                let grow = |q: Vec2| front + (q - front) * 1.08;
                draw_triangle(grow(corners[0]), grow(corners[1]), grow(corners[2]), OUTLINE);
                draw_triangle(grow(corners[0]), grow(corners[2]), grow(corners[3]), OUTLINE);
                let wood = c(rgba(86, 64, 42, 255));
                draw_triangle(corners[0], corners[1], corners[2], wood);
                draw_triangle(corners[0], corners[2], corners[3], wood);
                for k in 0..3 {
                    let y = -h * 0.4 + k as f32 * h * 0.4;
                    let a0 = front + side * wd + vec2(0.0, y);
                    let a1 = front - side * wd + vec2(0.0, y);
                    draw_line(a0.x, a0.y, a1.x, a1.y, 2.0, c(rgba(70, 70, 78, 255)));
                }
                ball(front - vec2(0.0, h * 0.05), r * 0.16, c(rgba(150, 140, 110, 255)), false);
            }
            EnemyKind::Necromancer => {
                // Floats rather than walks, trailing a tattered robe.
                let p = p + vec2(0.0, (t * 2.0).sin() * r * 0.15);
                draw_circle(p.x, p.y, r * 1.6, rgba(80, 255, 120, (14.0 * a) as u8));
                rag(p + vec2(-r * 0.6, -r * 0.5), p + vec2(r * 0.6, -r * 0.5), r * 1.4, (t * 1.5).sin() * r * 0.3, c(rgba(36, 40, 34, 255)), e.phase, true);
                for k in 0..4 {
                    let q = p + vec2(-r * 0.45 + k as f32 * r * 0.3, -r * 0.45);
                    ball(q, r * 0.1, c(rgba(200, 192, 170, 255)), false);
                }
                let head = p + vec2(fx * r * 0.1, -r * 0.85);
                draw_triangle(head + vec2(-r * 0.5, r * 0.3), head + vec2(r * 0.5, r * 0.3), head + vec2(0.0, -r * 0.8), c(rgba(30, 34, 30, 255)));
                ball(head, r * 0.36, c(rgba(176, 170, 150, 255)), true);
                draw_circle(head.x - r * 0.12, head.y, r * 0.09, OUTLINE);
                draw_circle(head.x + r * 0.12, head.y, r * 0.09, OUTLINE);
                eyes(head, r * 0.12, r * 0.05, with_alpha(rgba(120, 255, 140, 255), a));
                let hand = p + vec2(fx * r * 0.8, -r * 0.1);
                limb(p + vec2(fx * r * 0.35, -r * 0.4), hand, r * 0.13, c(rgba(176, 170, 150, 255)), true);
                let top = hand + vec2(fx * r * 0.1, -r * 1.2);
                draw_line(hand.x, hand.y + r * 0.7, top.x, top.y, 2.6, c(rgba(60, 46, 36, 255)));
                let fl2 = 0.8 + 0.2 * (t * 6.0).sin();
                draw_circle(top.x, top.y, r * 0.6 * fl2, rgba(100, 255, 140, (50.0 * a) as u8));
                ball(top, r * 0.25, c(rgba(200, 196, 176, 255)), false);
                eyes(top, r * 0.08, 1.0, with_alpha(rgba(120, 255, 140, 255), a));
            }
            EnemyKind::Leech => {
                // A blind, segmented worm that wriggles toward warmth.
                let ang = dir.to_angle();
                let body = c(rgba(96, 22, 34, 255));
                for k in (0..5).rev() {
                    let back = -dir * (k as f32 * r * 0.38);
                    let wig = dir.perp() * (t * 12.0 + k as f32 * 1.2).sin() * r * 0.25 * (k as f32 / 4.0);
                    let rr = r * (0.62 - k as f32 * 0.08);
                    ball(p + back + wig, rr, if k % 2 == 0 { body } else { shade(body, 0.8) }, k == 0);
                }
                // A round mouth ringed with teeth.
                let mouth = p + dir * r * 0.4;
                draw_circle(mouth.x, mouth.y, r * 0.3, c(rgba(40, 4, 8, 255)));
                for k in 0..6 {
                    let tip = mouth + Vec2::from_angle(ang + k as f32 / 6.0 * TAU) * r * 0.25;
                    draw_circle(tip.x, tip.y, 0.8, c(rgba(230, 210, 190, 255)));
                }
            }
            EnemyKind::Gargoyle => {
                // A crouching stone devil with folded wings; it springs when it lunges.
                let stone = c(rgba(104, 102, 98, 255));
                let lunging = e.fuse > 0.0;
                let spread = if lunging { 1.0 } else { 0.55 + 0.1 * (t * 2.0).sin() };
                for s in [-1.0, 1.0] {
                    let root = p + vec2(s * r * 0.35, -r * 0.5);
                    let tip = root + vec2(s * r * 1.3 * spread, -r * 0.7);
                    let low = root + vec2(s * r * 1.1 * spread, r * 0.5);
                    draw_triangle(root, tip, low, c(rgba(78, 76, 74, 255)));
                    draw_line(root.x, root.y, tip.x, tip.y, 1.2, c(rgba(60, 58, 56, 255)));
                    for k in 1..3 {
                        let rib = root.lerp(tip, 0.33 * k as f32);
                        let end = root.lerp(low, 0.33 * k as f32 + 0.2);
                        draw_line(rib.x, rib.y, end.x, end.y, 0.8, c(rgba(60, 58, 56, 255)));
                    }
                }
                for s in [-1.0, 1.0] {
                    limb(p + vec2(s * r * 0.3, r * 0.3), p + vec2(s * r * 0.55, r * 0.95), r * 0.22, shade(stone, 0.85), true);
                }
                blob(p, r * 0.6, r * 0.7, 0.0, stone, true);
                // Cracks in the stone.
                draw_line(p.x - r * 0.2, p.y - r * 0.3, p.x + r * 0.05, p.y + r * 0.2, 1.0, c(rgba(50, 48, 46, 255)));
                let head = p + vec2(fx * r * 0.2, -r * 0.75);
                ball(head, r * 0.42, stone, true);
                for s in [-1.0, 1.0] {
                    draw_triangle(head + vec2(s * r * 0.2, -r * 0.25), head + vec2(s * r * 0.05, -r * 0.3), head + vec2(s * r * 0.35, -r * 0.75), c(rgba(80, 78, 76, 255)));
                }
                draw_line(head.x + fx * r * 0.05, head.y + r * 0.2, head.x + fx * r * 0.35, head.y + r * 0.2, 1.2, c(rgba(30, 28, 28, 255)));
                eyes(head + vec2(fx * r * 0.12, -r * 0.02), r * 0.14, r * 0.08, with_alpha(rgba(255, 150, 60, 255), a));
            }
            EnemyKind::Banshee => {
                // A pale woman of mist with streaming hair, mouth open in a wail.
                let p = p + vec2(0.0, (t * 2.4).sin() * r * 0.15);
                let mist = Color::new(0.78, 0.84, 0.95, 0.55 * a);
                rag(p + vec2(-r * 0.55, -r * 0.4), p + vec2(r * 0.55, -r * 0.4), r * 1.5, (t * 1.8).sin() * r * 0.4, flashed(mist, fl), e.phase, false);
                for k in 0..5 {
                    let base = p + vec2(-r * 0.45 + k as f32 * r * 0.22, -r * 0.9);
                    let tip = base + vec2(-fx * r * 0.9 + (t * 3.0 + k as f32).sin() * r * 0.2, r * 0.3 + k as f32 * r * 0.1);
                    draw_line(base.x, base.y, tip.x, tip.y, 2.0, Color::new(0.85, 0.88, 0.95, 0.5 * a));
                }
                for s in [-1.0, 1.0] {
                    let hand = p + vec2(s * r * 0.9, -r * 0.1 + (t * 3.0 + s).sin() * r * 0.15);
                    draw_line(p.x + s * r * 0.3, p.y - r * 0.4, hand.x, hand.y, r * 0.12, Color::new(0.85, 0.9, 1.0, 0.6 * a));
                }
                let head = p + vec2(fx * r * 0.05, -r * 0.75);
                ball(head, r * 0.36, flashed(Color::new(0.88, 0.9, 0.96, 0.85 * a), fl), false);
                draw_circle(head.x - r * 0.12, head.y - r * 0.02, r * 0.08, Color::new(0.05, 0.05, 0.1, a));
                draw_circle(head.x + r * 0.12, head.y - r * 0.02, r * 0.08, Color::new(0.05, 0.05, 0.1, a));
                let wail = 0.12 + 0.05 * (t * 9.0).sin();
                draw_ellipse(head.x, head.y + r * 0.2, r * 0.1, r * wail, 0.0, Color::new(0.05, 0.05, 0.1, a));
            }
            EnemyKind::Courier => {
                // A small hooded runner, bent low, a golden key bouncing on its back.
                let s = (t * 14.0).sin() * r * 0.35;
                for (side, sw) in [(-1.0, s), (1.0, -s)] {
                    limb(p + vec2(side * r * 0.25, r * 0.2), p + vec2(side * r * 0.3 + sw * fx, r * 0.95), r * 0.2, c(rgba(60, 50, 40, 255)), true);
                }
                rag(p + vec2(-r * 0.55, -r * 0.6), p + vec2(r * 0.55, -r * 0.6), r * 1.1, -fx * 3.0, c(rgba(110, 86, 50, 255)), e.phase, true);
                let head = p + vec2(fx * r * 0.35, -r * 0.8);
                ball(head, r * 0.38, c(rgba(90, 70, 44, 255)), true);
                draw_circle(head.x + fx * r * 0.12, head.y + r * 0.05, r * 0.22, c(rgba(16, 12, 10, 255)));
                let key = p + vec2(-fx * r * 0.55, -r * 0.65 + (t * 14.0).cos().abs() * r * 0.2);
                draw_circle(key.x, key.y, r * 0.9, Color::new(1.0, 0.85, 0.35, 0.18 * a));
                draw_circle_lines(key.x, key.y, r * 0.3, 2.0, with_alpha(rgba(240, 200, 90, 255), a));
                draw_line(key.x, key.y + r * 0.3, key.x, key.y + r * 0.9, 2.0, with_alpha(rgba(240, 200, 90, 255), a));
            }
            EnemyKind::Champion => {
                // A towering knight of the mist: horned helm, a great maul, fog trailing off it.
                let mist = Color::new(0.75, 0.78, 0.82, 0.12 * a);
                for k in 0..4 {
                    let off = vec2((t * 0.8 + k as f32).sin() * r * 0.6, r * 0.3 - k as f32 * r * 0.2);
                    draw_circle(p.x + off.x, p.y + off.y, r * (1.0 - k as f32 * 0.12), mist);
                }
                let s = walk.sin() * r * 0.2;
                for (side, sw) in [(-1.0, s), (1.0, -s)] {
                    limb(p + vec2(side * r * 0.3, r * 0.3), p + vec2(side * r * 0.38 + sw * fx, r * 1.0), r * 0.26, c(rgba(52, 54, 60, 255)), true);
                }
                ball(p + vec2(0.0, -r * 0.15), r * 0.72, c(rgba(78, 82, 92, 255)), true);
                draw_line(p.x - r * 0.6, p.y - r * 0.2, p.x + r * 0.6, p.y - r * 0.2, r * 0.12, c(rgba(46, 48, 54, 255)));
                let head = p + vec2(fx * r * 0.1, -r * 0.95);
                ball(head, r * 0.36, c(rgba(96, 100, 110, 255)), true);
                for sd in [-1.0, 1.0] {
                    draw_triangle(head + vec2(sd * r * 0.25, -r * 0.15), head + vec2(sd * r * 0.4, -r * 0.05), head + vec2(sd * r * 0.62, -r * 0.6), c(rgba(200, 190, 170, 255)));
                }
                draw_line(head.x - r * 0.25, head.y + r * 0.02, head.x + r * 0.25, head.y + r * 0.02, r * 0.07, c(rgba(10, 10, 14, 255)));
                let swing = if e.fuse > 0.0 { 1.2 } else { (t * 1.5).sin() * 0.3 };
                let grip = p + vec2(fx * r * 0.7, -r * 0.2);
                let head2 = grip + Vec2::from_angle(-1.2 * fx + swing * fx) * r * 1.3 * vec2(fx, 1.0);
                draw_line(grip.x, grip.y, head2.x, head2.y, r * 0.1, c(rgba(70, 50, 34, 255)));
                draw_rectangle(head2.x - r * 0.3, head2.y - r * 0.22, r * 0.6, r * 0.44, c(rgba(90, 92, 100, 255)));
            }
            EnemyKind::Rival => {
                // A dark hero like you: a red scarf, a black blade, eyes like coals.
                let s = walk.sin() * r * 0.3;
                for (side, sw) in [(-1.0, s), (1.0, -s)] {
                    limb(p + vec2(side * r * 0.28, r * 0.25), p + vec2(side * r * 0.32 + sw * fx, r * 0.95), r * 0.2, c(rgba(24, 22, 28, 255)), true);
                }
                ball(p + vec2(0.0, -r * 0.2), r * 0.6, c(rgba(36, 32, 40, 255)), true);
                let scarf = c(rgba(170, 20, 24, 255));
                let tail = p + vec2(-fx * r * (1.2 + (t * 6.0).sin() * 0.2), -r * 0.45 + (t * 6.0).cos() * r * 0.15);
                draw_line(p.x, p.y - r * 0.6, tail.x, tail.y, r * 0.22, scarf);
                draw_line(p.x - r * 0.35, p.y - r * 0.6, p.x + r * 0.35, p.y - r * 0.6, r * 0.2, scarf);
                let head = p + vec2(fx * r * 0.12, -r * 0.92);
                ball(head, r * 0.34, c(rgba(30, 28, 34, 255)), true);
                let hand = p + vec2(fx * r * 0.65, -r * 0.1);
                let tip = hand + vec2(fx * r * 1.3, -r * 0.6);
                draw_line(hand.x, hand.y, tip.x, tip.y, r * 0.12, c(rgba(18, 16, 20, 255)));
                draw_line(hand.x, hand.y, tip.x, tip.y, r * 0.04, with_alpha(rgba(200, 60, 50, 255), a * 0.8));
                draw_circle_lines(p.x, p.y, r * 1.5, 1.5, Color::new(0.9, 0.15, 0.1, 0.35 * a));
            }
            EnemyKind::Boss(_) => {}
        }
    }

    /// Cheap models for the low detail setting.
    fn draw_enemy_simple(&self, e: &Enemy, fx: f32) {
        let p = e.pos;
        let r = e.r;
        let col = match e.kind {
            EnemyKind::Hollow => rgba(70, 62, 56, 255),
            EnemyKind::Ghoul => rgba(96, 112, 84, 255),
            EnemyKind::Moth => rgba(180, 176, 150, 220),
            EnemyKind::Archer => rgba(190, 184, 166, 255),
            EnemyKind::Bloat => rgba(98, 104, 66, 255),
            EnemyKind::Wraith => Color::new(0.45, 0.6, 0.95, 0.55),
            EnemyKind::Cultist => rgba(110, 22, 22, 255),
            EnemyKind::Shieldbearer => rgba(86, 64, 42, 255),
            EnemyKind::Necromancer => rgba(60, 80, 56, 255),
            EnemyKind::Leech => rgba(96, 22, 34, 255),
            EnemyKind::Gargoyle => rgba(104, 102, 98, 255),
            EnemyKind::Banshee => Color::new(0.8, 0.85, 0.95, 0.6),
            EnemyKind::Courier => rgba(110, 86, 50, 255),
            EnemyKind::Champion => rgba(84, 88, 98, 255),
            EnemyKind::Rival => rgba(36, 32, 40, 255),
            _ => rgba(66, 70, 82, 255),
        };
        let col = with_alpha(flashed(col, e.flash), e.alpha);
        draw_circle(p.x, p.y, r, col);
        draw_circle(p.x + fx * r * 0.2, p.y - r * 0.55, r * 0.5, shade(col, 1.2));
        eyes(p + vec2(fx * r * 0.25, -r * 0.55), r * 0.15, 1.3, with_alpha(rgba(255, 70, 30, 255), e.alpha));
    }

    // ---------------------------------------------------------------- bosses

    pub(super) fn draw_boss(&self, e: &Enemy, k: BossKind) {
        let p = e.pos;
        let r = e.r;
        let t = self.time;
        let a = e.alpha;
        let fl = e.flash;
        let c = |col: Color| with_alpha(flashed(col, fl), a);
        let dir = self.heading(p);
        let fx = if dir.x >= 0.0 { 1.0 } else { -1.0 };
        draw_ellipse(p.x, p.y + r * 0.95, r * 1.1, r * 0.35, 0.0, rgba(0, 0, 0, (130.0 * a) as u8));
        match k {
            BossKind::Gravewarden => {
                let plate = rgba(70, 66, 62, 255);
                let s = (t * 3.0).sin() * r * 0.12;
                rag(p + vec2(-r * 0.75, -r * 0.55), p + vec2(r * 0.75, -r * 0.55), r * 1.4, -fx * r * 0.2, c(rgba(36, 30, 28, 255)), 1.0, true);
                for (side, sw) in [(-1.0, s), (1.0, -s)] {
                    limb(p + vec2(side * r * 0.35, r * 0.4), p + vec2(side * r * 0.4 + sw, r * 0.95), r * 0.28, c(shade(plate, 0.8)), true);
                }
                blob(p, r * 0.85, r * 0.8, 0.0, c(plate), true);
                for k in 0..5 {
                    let x = p.x - r * 0.5 + k as f32 * r * 0.25;
                    draw_circle(x, p.y + r * 0.1, 1.6, c(rgba(140, 130, 110, 255)));
                }
                // Chains across the chest.
                for k in 0..7 {
                    let q = p + vec2(-r * 0.6 + k as f32 * r * 0.2, -r * 0.35 + (k as f32 * 0.9).sin() * 2.0);
                    draw_circle_lines(q.x, q.y, 2.2, 1.2, c(rgba(110, 106, 100, 255)));
                }
                ball(p + vec2(-r * 0.8, -r * 0.5), r * 0.32, c(shade(plate, 1.15)), true);
                ball(p + vec2(r * 0.8, -r * 0.5), r * 0.32, c(shade(plate, 1.15)), true);
                // Skulls of past challengers hang from its belt.
                for k in 0..3 {
                    let anchor = p + vec2(-r * 0.4 + k as f32 * r * 0.4, r * 0.45);
                    let q = anchor + vec2((t * 1.5 + k as f32).sin() * 2.0, r * 0.22);
                    draw_line(anchor.x, anchor.y, q.x, q.y, 1.0, c(rgba(80, 70, 60, 255)));
                    ball(q, r * 0.09, c(rgba(196, 188, 166, 255)), false);
                    draw_circle(q.x - r * 0.03, q.y, r * 0.025, OUTLINE);
                    draw_circle(q.x + r * 0.03, q.y, r * 0.025, OUTLINE);
                }
                let head = p + vec2(fx * r * 0.05, -r * 0.8);
                ball(head, r * 0.42, c(rgba(96, 92, 88, 255)), true);
                for s in [-1.0, 1.0] {
                    draw_triangle(head + vec2(s * r * 0.3, -r * 0.2), head + vec2(s * r * 0.15, -r * 0.35), head + vec2(s * r * 0.75, -r * 0.85), c(rgba(190, 180, 160, 255)));
                }
                draw_line(head.x - r * 0.28, head.y, head.x + r * 0.28, head.y, 3.0, rgba(8, 8, 8, 255));
                eyes(head + vec2(fx * r * 0.05, 0.0), r * 0.13, 2.4, with_alpha(rgba(255, 110, 40, 255), a));
                // Greatshield with a skull, greatsword raised.
                let sh = p + vec2(-fx * r * 1.05, r * 0.05);
                draw_rectangle(sh.x - r * 0.32, sh.y - r * 0.72, r * 0.64, r * 1.44, OUTLINE);
                draw_rectangle(sh.x - r * 0.28, sh.y - r * 0.68, r * 0.56, r * 1.36, c(rgba(78, 68, 52, 255)));
                draw_rectangle_lines(sh.x - r * 0.28, sh.y - r * 0.68, r * 0.56, r * 1.36, 2.0, c(rgba(140, 118, 70, 255)));
                ball(sh - vec2(0.0, r * 0.1), r * 0.16, c(rgba(200, 190, 170, 255)), false);
                draw_circle(sh.x - r * 0.06, sh.y - r * 0.12, r * 0.04, OUTLINE);
                draw_circle(sh.x + r * 0.06, sh.y - r * 0.12, r * 0.04, OUTLINE);
                let sw = (t * 2.0).sin() * 0.15;
                let hilt = p + vec2(fx * r * 0.8, -r * 0.1);
                let tip = hilt + Vec2::from_angle(-PI * 0.5 + fx * (0.55 + sw)) * r * 2.1;
                draw_line(hilt.x, hilt.y, tip.x, tip.y, 9.0, OUTLINE);
                draw_line(hilt.x, hilt.y, tip.x, tip.y, 7.0, c(rgba(150, 150, 160, 255)));
                draw_line(hilt.x, hilt.y, tip.x, tip.y, 2.0, c(rgba(210, 210, 220, 255)));
                let guard = (tip - hilt).normalize().perp() * r * 0.3;
                draw_line(hilt.x - guard.x, hilt.y - guard.y, hilt.x + guard.x, hilt.y + guard.y, 4.0, c(rgba(110, 90, 50, 255)));
            }
            BossKind::Matriarch => {
                let flap = (t * 8.0).sin().abs();
                for s in [-1.0, 1.0] {
                    for (k, (yo, size)) in [(-0.3, 1.0), (0.45, 0.65)].iter().enumerate() {
                        let span = r * size * (0.9 + flap * 0.7);
                        let root = p + vec2(s * r * 0.2, r * yo);
                        let tip = root + vec2(s * span * 1.4, -r * size * 0.6);
                        let low = root + vec2(s * span * 1.1, r * size * 0.55);
                        let wing = c(if k == 0 { rgba(200, 194, 172, 220) } else { rgba(170, 160, 136, 220) });
                        draw_triangle(root, tip, low, wing);
                        draw_triangle(root, tip, root + vec2(s * span * 0.6, -r * size * 0.9), wing);
                        // Veins and an eye spot.
                        draw_line(root.x, root.y, tip.x, tip.y, 1.0, c(rgba(120, 110, 90, 255)));
                        draw_line(root.x, root.y, low.x, low.y, 1.0, c(rgba(120, 110, 90, 255)));
                        let spot = root.lerp(tip, 0.6) + vec2(0.0, r * 0.15);
                        draw_circle(spot.x, spot.y, r * 0.18 * size, c(rgba(70, 50, 40, 255)));
                        draw_circle(spot.x, spot.y, r * 0.1 * size, c(rgba(230, 190, 110, 255)));
                        draw_circle(spot.x, spot.y, r * 0.04 * size, c(rgba(20, 10, 10, 255)));
                    }
                }
                for k in 0..4 {
                    let q = p + vec2(0.0, -r * 0.35 + k as f32 * r * 0.3);
                    blob(q, r * (0.32 - k as f32 * 0.03), r * 0.18, 0.0, c(rgba(96, 84, 70, 255)), true);
                }
                // Scales of dust drifting off the wings.
                for k in 0..10 {
                    let ph = (t * 0.6 + k as f32 * 0.37).fract();
                    let q = p + vec2((k as f32 * 2.3).sin() * r * 1.4, -r * 0.4 + ph * r * 1.8);
                    draw_circle(q.x, q.y, 1.2, Color::new(0.9, 0.85, 0.7, 0.5 * (1.0 - ph) * a));
                }
                let head = p + vec2(0.0, -r * 0.6);
                ball(head, r * 0.26, c(rgba(80, 70, 60, 255)), true);
                for k in 0..3 {
                    eyes(head + vec2(0.0, -r * 0.05 + k as f32 * r * 0.09), r * (0.08 + k as f32 * 0.04), 2.0, with_alpha(rgba(255, 220, 140, 255), a));
                }
                for s in [-1.0, 1.0] {
                    let base = head + vec2(s * r * 0.12, -r * 0.2);
                    let tip = base + vec2(s * r * 0.5, -r * 0.6);
                    draw_line(base.x, base.y, tip.x, tip.y, 1.5, c(rgba(150, 130, 100, 255)));
                    for j in 1..5 {
                        let q = base.lerp(tip, j as f32 / 5.0);
                        draw_line(q.x, q.y, q.x + s * r * 0.08, q.y + r * 0.04, 1.0, c(rgba(150, 130, 100, 255)));
                    }
                }
            }
            BossKind::Hermit => {
                let robe = rgba(44, 38, 52, 255);
                rag(p + vec2(-r * 0.8, -r * 0.35), p + vec2(r * 0.8, -r * 0.35), r * 1.45, -fx * r * 0.15, c(robe), 3.0, true);
                blob(p + vec2(-fx * r * 0.1, -r * 0.2), r * 0.6, r * 0.55, 0.0, c(shade(robe, 1.1)), true);
                // Hunched hood, beard, glowing eyes.
                let head = p + vec2(fx * r * 0.35, -r * 0.55);
                ball(head, r * 0.42, c(rgba(30, 26, 36, 255)), true);
                draw_circle(head.x + fx * r * 0.1, head.y + r * 0.05, r * 0.26, rgba(4, 4, 6, (255.0 * a) as u8));
                draw_triangle(head + vec2(-r * 0.15, r * 0.2), head + vec2(r * 0.25, r * 0.2), head + vec2(fx * r * 0.1, r * 0.9), c(rgba(150, 150, 140, 255)));
                eyes(head + vec2(fx * r * 0.12, 0.0), r * 0.11, 2.4, with_alpha(rgba(160, 255, 200, 255), a));
                // Wax candles burn on its hunched back.
                for k in 0..3 {
                    let base = p + vec2(-fx * r * (0.15 + k as f32 * 0.2), -r * (0.7 - k as f32 * 0.08));
                    draw_rectangle(base.x - 1.5, base.y - r * 0.18, 3.0, r * 0.18, c(rgba(210, 200, 170, 255)));
                    let flick = 0.8 + 0.2 * (t * 11.0 + k as f32).sin();
                    draw_circle(base.x, base.y - r * 0.22, 2.2 * flick, rgba(255, 190, 90, (230.0 * a) as u8));
                }
                // Staff with the great bell.
                let hand = p + vec2(fx * r * 0.8, -r * 0.05);
                limb(head + vec2(0.0, r * 0.3), hand, 3.0, c(shade(robe, 1.2)), true);
                let top = hand + vec2(fx * r * 0.1, -r * 1.3);
                let bottom = hand + vec2(-fx * r * 0.05, r * 0.95);
                draw_line(bottom.x, bottom.y, top.x, top.y, 4.0, OUTLINE);
                draw_line(bottom.x, bottom.y, top.x, top.y, 3.0, c(rgba(80, 60, 40, 255)));
                let swing = (t * 3.0).sin() * 0.3;
                let bell = top + Vec2::from_angle(PI * 0.5 + swing) * r * 0.45;
                draw_line(top.x, top.y, bell.x, bell.y - r * 0.2, 1.5, c(rgba(90, 90, 96, 255)));
                // A bell: domed crown, flared body, lip and clapper.
                let brass = c(rgba(196, 164, 82, 255));
                draw_circle(bell.x, bell.y - r * 0.12, r * 0.2, OUTLINE);
                draw_circle(bell.x, bell.y - r * 0.12, r * 0.17, brass);
                draw_triangle(bell + vec2(-r * 0.19, -r * 0.12), bell + vec2(r * 0.19, -r * 0.12), bell + vec2(r * 0.34, r * 0.28), brass);
                draw_triangle(bell + vec2(-r * 0.19, -r * 0.12), bell + vec2(r * 0.34, r * 0.28), bell + vec2(-r * 0.34, r * 0.28), brass);
                draw_line(bell.x - r * 0.37, bell.y + r * 0.28, bell.x + r * 0.37, bell.y + r * 0.28, 3.0, c(rgba(150, 120, 56, 255)));
                draw_line(bell.x - r * 0.1, bell.y - r * 0.18, bell.x - r * 0.2, bell.y + r * 0.2, 2.0, c(rgba(240, 220, 150, 255)));
                let clap = (t * 3.0).cos() * r * 0.06;
                draw_circle(bell.x + clap, bell.y + r * 0.36, r * 0.07, c(rgba(110, 86, 40, 255)));
            }
            BossKind::Bride => {
                // A drowned bride in a sodden veil, floating above black water.
                let p = p + vec2(0.0, (t * 1.6).sin() * r * 0.06);
                draw_ellipse(p.x, p.y + r * 0.95, r * 1.3, r * 0.35, 0.0, Color::new(0.1, 0.3, 0.35, 0.3 * a));
                let dress = c(rgba(196, 206, 200, 235));
                rag(p + vec2(-r * 0.6, -r * 0.3), p + vec2(r * 0.6, -r * 0.3), r * 1.35, (t * 1.2).sin() * r * 0.15, dress, 2.0, true);
                rag(p + vec2(-r * 0.4, -r * 0.2), p + vec2(r * 0.4, -r * 0.2), r * 1.1, (t * 1.5).sin() * r * 0.2, c(rgba(150, 170, 168, 220)), 4.0, false);
                // Weeds and water dripping from the hem.
                for k in 0..6 {
                    let x = p.x - r * 0.55 + k as f32 * r * 0.22;
                    let drip = ((t * 1.4 + k as f32 * 0.7).fract()) * r * 0.4;
                    draw_line(x, p.y + r * 1.0, x + (k as f32).sin() * 3.0, p.y + r * 1.1 + drip, 1.5, c(rgba(60, 90, 70, 220)));
                    draw_circle(x, p.y + r * 1.15 + drip, 1.4, Color::new(0.5, 0.75, 0.85, 0.7 * a));
                }
                blob(p + vec2(0.0, -r * 0.45), r * 0.42, r * 0.5, 0.0, dress, true);
                // A bouquet of dead flowers held to the chest.
                let bq = p + vec2(fx * r * 0.15, -r * 0.3);
                for (k, col) in [rgba(90, 60, 80, 255), rgba(70, 80, 60, 255), rgba(110, 70, 70, 255)].iter().enumerate() {
                    ball(bq + vec2((k as f32 - 1.0) * r * 0.13, -((k % 2) as f32) * r * 0.08), r * 0.1, c(*col), false);
                }
                for s in [-1.0, 1.0] {
                    limb(p + vec2(s * r * 0.35, -r * 0.65), bq + vec2(s * r * 0.1, r * 0.05), r * 0.09, c(rgba(200, 214, 214, 255)), true);
                }
                let head = p + vec2(fx * r * 0.04, -r * 1.05);
                ball(head, r * 0.3, c(rgba(200, 216, 218, 255)), true);
                // Long wet hair and a veil.
                for k in 0..6 {
                    let base = head + vec2(-r * 0.25 + k as f32 * r * 0.1, -r * 0.1);
                    let tip = base + vec2((t * 1.3 + k as f32).sin() * r * 0.1, r * 0.8);
                    draw_line(base.x, base.y, tip.x, tip.y, 2.0, c(rgba(30, 36, 40, 255)));
                }
                draw_triangle(head + vec2(-r * 0.45, r * 0.1), head + vec2(r * 0.45, r * 0.1), head + vec2(0.0, -r * 0.45), Color::new(0.9, 0.95, 0.95, 0.35 * a));
                draw_circle(head.x - r * 0.1, head.y, r * 0.07, OUTLINE);
                draw_circle(head.x + r * 0.1, head.y, r * 0.07, OUTLINE);
                // Black tears.
                for s in [-1.0, 1.0] {
                    draw_line(head.x + s * r * 0.1, head.y + r * 0.05, head.x + s * r * 0.11, head.y + r * 0.28, 1.2, c(rgba(20, 30, 40, 255)));
                }
            }
            BossKind::BogMother => {
                // A vast swamp toad, crowned with reeds, with a dozen eyes.
                let breathe = 1.0 + (t * 2.0).sin() * 0.04;
                let skin = c(rgba(72, 92, 54, 255));
                for s in [-1.0, 1.0] {
                    limb(p + vec2(s * r * 0.6, r * 0.2), p + vec2(s * r * 1.05, r * 0.85), r * 0.22, shade(skin, 0.8), true);
                    for k in -1..=1 {
                        let toe = p + vec2(s * r * 1.05 + k as f32 * r * 0.1, r * 0.95);
                        draw_circle(toe.x, toe.y, r * 0.07, c(rgba(110, 130, 80, 255)));
                    }
                }
                blob(p, r * 1.05 * breathe, r * 0.8 * breathe, 0.0, skin, true);
                // Warts and the pale belly.
                draw_ellipse(p.x, p.y + r * 0.3, r * 0.7, r * 0.38, 0.0, c(rgba(150, 160, 110, 255)));
                for k in 0..9 {
                    let a0 = e.phase + k as f32 * 0.7;
                    let q = p + vec2(a0.cos() * r * 0.75, a0.sin() * r * 0.35 - r * 0.3);
                    draw_circle(q.x, q.y, r * 0.06, c(rgba(56, 70, 40, 255)));
                }
                // Reeds growing out of its back.
                for k in 0..7 {
                    let base = p + vec2(-r * 0.6 + k as f32 * r * 0.2, -r * 0.55);
                    let tip = base + vec2((t * 1.1 + k as f32).sin() * r * 0.15, -r * (0.6 + (k % 3) as f32 * 0.2));
                    draw_line(base.x, base.y, tip.x, tip.y, 2.0, c(rgba(110, 120, 60, 255)));
                    if k % 2 == 0 {
                        draw_ellipse(tip.x, tip.y, r * 0.05, r * 0.14, 0.0, c(rgba(90, 60, 40, 255)));
                    }
                }
                // A wide mouth and many small eyes.
                let mouth = p + vec2(fx * r * 0.1, r * 0.05);
                draw_arc(mouth.x, mouth.y - r * 0.2, 24, r * 0.75, 30.0, 3.0, 120.0, c(rgba(30, 20, 20, 255)));
                for k in 0..6 {
                    let q = p + vec2(-r * 0.5 + k as f32 * r * 0.2, -r * 0.35 - ((k as f32 - 2.5).abs() * -0.06 + 0.15) * r);
                    ball(q, r * 0.08, c(rgba(220, 200, 90, 255)), false);
                    draw_circle(q.x + fx * r * 0.02, q.y, r * 0.035, OUTLINE);
                }
            }
            BossKind::Penitent => {
                // A hooded giant bound in chains, a spiked flail swinging around it.
                let robe = c(rgba(70, 58, 50, 255));
                let iron = c(rgba(90, 92, 100, 255));
                let s = (t * 2.0).sin() * r * 0.1;
                for (side, sw) in [(-1.0, s), (1.0, -s)] {
                    limb(p + vec2(side * r * 0.35, r * 0.4), p + vec2(side * r * 0.4 + sw, r * 0.95), r * 0.27, shade(robe, 0.8), true);
                }
                rag(p + vec2(-r * 0.75, -r * 0.5), p + vec2(r * 0.75, -r * 0.5), r * 1.4, -fx * r * 0.15, robe, 3.0, true);
                blob(p, r * 0.8, r * 0.8, 0.0, robe, true);
                // Chains wound across the body, an iron cage around the head.
                for row in 0..3 {
                    for k in 0..9 {
                        let q = p + vec2(-r * 0.7 + k as f32 * r * 0.175, -r * 0.45 + row as f32 * r * 0.35 + (k as f32 * 0.8 + row as f32).sin() * 2.0);
                        draw_circle_lines(q.x, q.y, 2.4, 1.3, iron);
                    }
                }
                let head = p + vec2(fx * r * 0.05, -r * 0.85);
                draw_triangle(head + vec2(-r * 0.5, r * 0.3), head + vec2(r * 0.5, r * 0.3), head + vec2(-fx * r * 0.2, -r * 0.7), robe);
                ball(head, r * 0.4, shade(robe, 0.9), true);
                draw_circle(head.x + fx * r * 0.08, head.y + r * 0.05, r * 0.26, OUTLINE);
                for k in 0..4 {
                    let x = head.x - r * 0.3 + k as f32 * r * 0.2;
                    draw_line(x, head.y - r * 0.35, x, head.y + r * 0.3, 2.0, iron);
                }
                eyes(head + vec2(fx * r * 0.08, r * 0.05), r * 0.1, 2.0, with_alpha(rgba(255, 200, 120, 255), a));
                // The flail: a chain from the fist to a spiked ball circling it.
                let fist = p + vec2(fx * r * 0.8, -r * 0.1);
                ball(fist, r * 0.18, iron, true);
                let ang = t * 3.0;
                let ballp = fist + Vec2::from_angle(ang) * r * 1.4;
                chain(fist, ballp, iron);
                ball(ballp, r * 0.3, c(rgba(70, 70, 78, 255)), true);
                for k in 0..8 {
                    let sp = Vec2::from_angle(k as f32 / 8.0 * TAU + ang);
                    let q = ballp + sp * r * 0.3;
                    let tip = ballp + sp * r * 0.48;
                    draw_triangle(q + sp.perp() * r * 0.07, q - sp.perp() * r * 0.07, tip, c(rgba(150, 150, 160, 255)));
                }
            }
            BossKind::Choir => {
                // Three faces on one floating shroud, mouths open in one long note.
                let p = p + vec2(0.0, (t * 1.8).sin() * r * 0.1);
                let shroud = Color::new(0.24, 0.2, 0.32, 0.9 * a);
                rag(p + vec2(-r * 0.9, -r * 0.4), p + vec2(r * 0.9, -r * 0.4), r * 1.6, (t * 1.4).sin() * r * 0.3, flashed(shroud, fl), 5.0, true);
                draw_circle(p.x, p.y - r * 0.2, r * 1.3, Color::new(0.7, 0.5, 1.0, 0.06 * a));
                for (k, &(dx, dy, size)) in [(-0.55f32, -0.55f32, 0.32f32), (0.0, -0.85, 0.38), (0.55, -0.55, 0.32)].iter().enumerate() {
                    let head = p + vec2(dx * r, dy * r + (t * 3.0 + k as f32).sin() * r * 0.05);
                    ball(head, r * size, flashed(Color::new(0.86, 0.84, 0.9, a), fl), true);
                    draw_circle(head.x - r * size * 0.35, head.y - r * size * 0.1, r * size * 0.18, OUTLINE);
                    draw_circle(head.x + r * size * 0.35, head.y - r * size * 0.1, r * size * 0.18, OUTLINE);
                    let sing = 0.3 + 0.1 * (t * 6.0 + k as f32 * 2.0).sin();
                    draw_ellipse(head.x, head.y + r * size * 0.4, r * size * 0.22, r * size * sing, 0.0, OUTLINE);
                    // A thorn crown on the middle face.
                    if k == 1 {
                        for j in 0..5 {
                            let a0 = -PI * 0.5 + (j as f32 - 2.0) * 0.35;
                            let q = head + Vec2::from_angle(a0) * r * size;
                            let tip = head + Vec2::from_angle(a0) * r * size * 1.5;
                            draw_line(q.x, q.y, tip.x, tip.y, 1.5, c(rgba(200, 170, 90, 255)));
                        }
                    }
                }
                // Notes rising from the song.
                for k in 0..3 {
                    let f = (t * 0.5 + k as f32 / 3.0).fract();
                    let q = p + vec2((k as f32 - 1.0) * r * 0.6 + (t * 2.0 + k as f32).sin() * r * 0.2, -r * 1.2 - f * r * 1.2);
                    draw_circle(q.x, q.y, r * 0.07, Color::new(0.9, 0.8, 1.0, (1.0 - f) * a));
                    draw_line(q.x + r * 0.06, q.y, q.x + r * 0.06, q.y - r * 0.25, 1.2, Color::new(0.9, 0.8, 1.0, (1.0 - f) * a));
                }
            }
            BossKind::Seraph => {
                // A broken stone angel with great wings and a cracked halo.
                let stone = c(rgba(150, 148, 142, 255));
                let beat = (t * 2.5).sin() * 0.15;
                for s in [-1.0, 1.0] {
                    for k in 0..4 {
                        let root = p + vec2(s * r * 0.3, -r * 0.5);
                        let a0 = -PI * 0.5 + s * (0.9 + k as f32 * 0.35 + beat);
                        let tip = root + Vec2::from_angle(a0) * r * (1.6 - k as f32 * 0.2);
                        let back = root + Vec2::from_angle(a0 + s * 0.35) * r * (1.3 - k as f32 * 0.2);
                        draw_triangle(root, tip, back, c(rgba(130 - k * 8, 128 - k * 8, 124 - k * 8, 255)));
                    }
                }
                rag(p + vec2(-r * 0.5, -r * 0.3), p + vec2(r * 0.5, -r * 0.3), r * 1.2, 0.0, shade(stone, 0.9), 6.0, true);
                blob(p + vec2(0.0, -r * 0.35), r * 0.45, r * 0.55, 0.0, stone, true);
                // Cracks and a missing arm; the other holds a broken sword.
                for (a0, b0) in [(vec2(-0.2, -0.6), vec2(0.1, -0.1)), (vec2(0.15, -0.4), vec2(0.3, 0.1))] {
                    draw_line(p.x + a0.x * r, p.y + a0.y * r, p.x + b0.x * r, p.y + b0.y * r, 1.2, c(rgba(70, 68, 66, 255)));
                }
                let hand = p + vec2(fx * r * 0.6, -r * 0.2);
                limb(p + vec2(fx * r * 0.35, -r * 0.6), hand, r * 0.13, stone, true);
                let tip = hand + vec2(fx * r * 0.3, -r * 0.9);
                draw_line(hand.x, hand.y, tip.x, tip.y, 5.0, OUTLINE);
                draw_line(hand.x, hand.y, tip.x, tip.y, 3.5, c(rgba(170, 170, 176, 255)));
                let head = p + vec2(fx * r * 0.05, -r * 1.05);
                ball(head, r * 0.3, stone, true);
                draw_line(head.x - r * 0.15, head.y, head.x + r * 0.15, head.y, 1.5, c(rgba(60, 58, 56, 255)));
                // The halo, cracked and flickering.
                let fl2 = 0.7 + 0.3 * (t * 7.0).sin();
                draw_arc(head.x, head.y - r * 0.45, 24, r * 0.35, 200.0, 3.0, 290.0, Color::new(1.0, 0.85, 0.5, fl2 * a));
            }
            BossKind::Hound => {
                // A huge pale hound, ribs showing, running low; seen from the side like the
                // heroes, so it reads as a hound from every direction.
                let fwd = vec2(fx, 0.0);
                let ang = fwd.to_angle();
                let side = vec2(0.0, 0.35);
                let fur = c(rgba(200, 196, 190, 255));
                let run = t * 12.0;
                for k in 0..4 {
                    let front = k < 2;
                    let sgn = if k % 2 == 0 { 1.0 } else { -1.0 };
                    let ph = (run + if k == 0 || k == 3 { 0.0 } else { PI }).sin();
                    let root = p + fwd * if front { r * 0.5 } else { -r * 0.55 } + side * sgn * r * 0.3;
                    let foot = root + fwd * ph * r * 0.35 + vec2(0.0, r * 0.6);
                    limb(root, foot, r * 0.15, shade(fur, 0.85), true);
                }
                blob(p, r * 1.0, r * 0.5, ang, fur, true);
                for k in 0..5 {
                    let q = p + fwd * (r * 0.4 - k as f32 * r * 0.2);
                    draw_line(q.x - side.x * r * 0.3, q.y - side.y * r * 0.3 - r * 0.1, q.x + side.x * r * 0.3, q.y + side.y * r * 0.3 - r * 0.1, 1.0, c(rgba(140, 134, 128, 255)));
                }
                // A tail and a long skull-like head with burning eyes.
                let tail = p - fwd * r * 1.0;
                let tip = tail - fwd * r * 0.6 + vec2(0.0, -r * 0.3 + (t * 8.0).sin() * r * 0.15);
                draw_line(tail.x, tail.y, tip.x, tip.y, 3.0, fur);
                let head = p + fwd * r * 1.0 - vec2(0.0, r * 0.15);
                blob(head, r * 0.45, r * 0.3, ang, fur, true);
                let snout = head + fwd * r * 0.4;
                ball(snout, r * 0.18, c(rgba(180, 176, 170, 255)), false);
                draw_line(snout.x, snout.y + r * 0.05, snout.x - fwd.x * r * 0.25, snout.y + r * 0.1, 1.2, c(rgba(110, 10, 12, 255)));
                for s in [-1.0, 1.0] {
                    let ear = head - fwd * r * 0.15 + side * s * r * 0.2;
                    draw_triangle(ear, ear - fwd * r * 0.1 + vec2(0.0, -r * 0.35), ear + side * s * r * 0.12, fur);
                }
                eyes(head + fwd * r * 0.12 - vec2(0.0, r * 0.08), r * 0.12, r * 0.07, with_alpha(rgba(255, 60, 40, 255), a));
            }
            BossKind::Sovereign => {
                let fl2 = (t * 9.0).sin();
                draw_circle(p.x, p.y, r * 1.7, rgba(255, 90, 30, (16.0 * a) as u8));
                // Burning cape with ember edges.
                let cape = c(rgba(50, 20, 16, 255));
                rag(p + vec2(-r * 0.85, -r * 0.6), p + vec2(r * 0.85, -r * 0.6), r * 1.6, -fx * r * 0.3 + fl2 * 2.0, cape, 5.0, true);
                for k in 0..7 {
                    let q = p + vec2(-r * 0.8 + k as f32 * r * 0.27, r * (0.85 + (t * 4.0 + k as f32).sin() * 0.08));
                    draw_circle(q.x, q.y, r * 0.06, rgba(255, 120, 40, (220.0 * a) as u8));
                }
                let s = (t * 2.5).sin() * r * 0.1;
                for (side, sw) in [(-1.0, s), (1.0, -s)] {
                    limb(p + vec2(side * r * 0.3, r * 0.4), p + vec2(side * r * 0.35 + sw, r * 0.95), r * 0.25, c(rgba(46, 34, 30, 255)), true);
                }
                blob(p, r * 0.8, r * 0.82, 0.0, c(rgba(58, 42, 36, 255)), true);
                // Glowing cracks in the armor.
                for k in 0..4 {
                    let a0 = 0.6 + k as f32 * 1.5;
                    let from = p + Vec2::from_angle(a0) * r * 0.15;
                    let mid = p + Vec2::from_angle(a0 + 0.3) * r * 0.45;
                    let to = p + Vec2::from_angle(a0 + 0.1) * r * 0.7;
                    let glow = rgba(255, 130 + (fl2 * 30.0) as u8, 40, (230.0 * a) as u8);
                    draw_line(from.x, from.y, mid.x, mid.y, 1.6, glow);
                    draw_line(mid.x, mid.y, to.x, to.y, 1.2, glow);
                }
                ball(p + vec2(-r * 0.75, -r * 0.5), r * 0.3, c(rgba(70, 50, 42, 255)), true);
                ball(p + vec2(r * 0.75, -r * 0.5), r * 0.3, c(rgba(70, 50, 42, 255)), true);
                let head = p + vec2(0.0, -r * 0.8);
                ball(head, r * 0.38, c(rgba(34, 28, 28, 255)), true);
                // Crown of fire.
                for k in 0..7 {
                    let ang = -PI * 0.5 + (k as f32 - 3.0) * 0.28;
                    let base = head + Vec2::from_angle(ang) * r * 0.32;
                    let tip = base + Vec2::from_angle(ang) * (r * 0.3 + (t * 10.0 + k as f32).sin() * r * 0.08);
                    draw_triangle(base + Vec2::from_angle(ang + 1.57) * 2.5, base - Vec2::from_angle(ang + 1.57) * 2.5, tip, rgba(255, 150, 50, (255.0 * a) as u8));
                    draw_circle(tip.x, tip.y, 2.0, rgba(255, 230, 150, (255.0 * a) as u8));
                }
                eyes(head + vec2(fx * r * 0.06, r * 0.02), r * 0.13, 3.0, with_alpha(rgba(255, 210, 90, 255), a));
                // Embers rise off him.
                for k in 0..8 {
                    let ph = (t * 0.8 + k as f32 * 0.29).fract();
                    let q = p + vec2((k as f32 * 1.9 + t).sin() * r * 0.8, r * 0.4 - ph * r * 2.2);
                    draw_circle(q.x, q.y, 1.6 * (1.0 - ph) + 0.5, rgba(255, 140, 50, (220.0 * (1.0 - ph) * a) as u8));
                }
                // Flaming greatsword.
                let hilt = p + vec2(fx * r * 0.7, 0.0);
                let tip = hilt + Vec2::from_angle(fx * (0.5 + (t * 1.5).sin() * 0.25) - PI * 0.5 * (1.0 - fx.abs() * 0.0)) * r * 2.3;
                let tip = if fx > 0.0 { tip } else { hilt + (tip - hilt) * vec2(1.0, 1.0) };
                draw_line(hilt.x, hilt.y, tip.x, tip.y, 10.0, rgba(255, 90, 30, (60.0 * a) as u8));
                draw_line(hilt.x, hilt.y, tip.x, tip.y, 7.0, c(rgba(50, 40, 40, 255)));
                draw_line(hilt.x, hilt.y, tip.x, tip.y, 2.0, rgba(255, 140, 50, (255.0 * a) as u8));
                for k in 1..6 {
                    let q = hilt.lerp(tip, k as f32 / 6.0) + vec2(0.0, -(t * 12.0 + k as f32).sin().abs() * 5.0);
                    draw_circle(q.x, q.y, 2.5, rgba(255, 170, 60, (200.0 * a) as u8));
                }
            }
        }
    }

    /// The Grave Cat: a black cat with yellow eyes, sitting or trotting after its human.
    pub(super) fn draw_cat(&self, cat: &Cat) {
        let p = cat.pos;
        let f = cat.facing;
        let t = self.time;
        let black = rgba(14, 12, 16, 255);
        draw_ellipse(p.x, p.y + 6.0, 10.0, 3.0, 0.0, rgba(0, 0, 0, 120));
        // Tail sways in a lazy curve.
        let mut prev = p + vec2(-f * 7.0, 2.0);
        for k in 1..6 {
            let a = k as f32 * 0.35;
            let sway = (t * 2.0 + k as f32 * 0.6).sin() * 2.5;
            let next = prev + vec2(-f * 3.0 * a.cos(), -3.0 * a.sin() * 1.5 + sway * 0.3);
            draw_line(prev.x, prev.y, next.x, next.y, 2.2, black);
            prev = next;
        }
        if cat.sitting {
            draw_ellipse(p.x, p.y, 7.0, 6.5, 0.0, black);
        } else {
            draw_ellipse(p.x, p.y + 1.0, 9.0, 5.0, 0.0, black);
            let step = (t * 14.0).sin() * 2.0;
            for (dx, ph) in [(-5.0, step), (5.0, -step)] {
                draw_line(p.x + dx, p.y + 3.0, p.x + dx + ph, p.y + 7.0, 1.8, black);
            }
        }
        let head = p + vec2(f * 7.0, if cat.sitting { -8.0 } else { -4.0 });
        draw_circle(head.x, head.y, 5.0, black);
        draw_triangle(head + vec2(-4.5, -2.0), head + vec2(-1.5, -3.5), head + vec2(-4.0, -8.0), black);
        draw_triangle(head + vec2(4.5, -2.0), head + vec2(1.5, -3.5), head + vec2(4.0, -8.0), black);
        // Eyes catch the light, and blink now and then.
        let blink = ((t * 0.7 + p.x * 0.01).sin() > 0.97) as u8 as f32;
        let eye = Color::new(1.0, 0.85, 0.2, 1.0 - blink);
        draw_circle(head.x - 1.8 + f, head.y - 0.5, 1.2, eye);
        draw_circle(head.x + 1.8 + f, head.y - 0.5, 1.2, eye);
        draw_circle(head.x + f, head.y - 0.5, 6.0, Color::new(1.0, 0.8, 0.2, 0.05 * (1.0 - blink)));
    }
}
