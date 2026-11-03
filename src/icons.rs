//! Small vector icons for weapons and items, readable at a glance in the HUD and on cards.
//! Each one draws into a square of side `s` centred on `c`.

use crate::data::*;
use macroquad::prelude::*;
use std::f32::consts::{PI, TAU};

fn line(c: Vec2, s: f32, a: Vec2, b: Vec2, w: f32, col: Color) {
    let (a, b) = (c + a * s, c + b * s);
    draw_line(a.x, a.y, b.x, b.y, (w * s).max(1.0), col);
}

fn tri(c: Vec2, s: f32, a: Vec2, b: Vec2, d: Vec2, col: Color) {
    draw_triangle(c + a * s, c + b * s, c + d * s, col);
}

fn circle(c: Vec2, s: f32, at: Vec2, r: f32, col: Color) {
    let p = c + at * s;
    draw_circle(p.x, p.y, r * s, col);
}

fn ring(c: Vec2, s: f32, at: Vec2, r: f32, w: f32, col: Color) {
    let p = c + at * s;
    draw_circle_lines(p.x, p.y, r * s, (w * s).max(1.0), col);
}

/// An arc of `n` segments from angle `a0` to `a1`, radius `r`, around `at`.
#[allow(clippy::too_many_arguments)]
fn arc(c: Vec2, s: f32, at: Vec2, r: f32, a0: f32, a1: f32, w: f32, col: Color) {
    let n = 10;
    for k in 0..n {
        let (t0, t1) = (a0 + (a1 - a0) * k as f32 / n as f32, a0 + (a1 - a0) * (k + 1) as f32 / n as f32);
        line(c, s, at + Vec2::from_angle(t0) * r, at + Vec2::from_angle(t1) * r, w, col);
    }
}

fn darker(c: Color, k: f32) -> Color {
    Color::new(c.r * k, c.g * k, c.b * k, c.a)
}

pub fn weapon_icon(kind: WeaponKind, c: Vec2, s: f32, col: Color) {
    let dark = darker(col, 0.55);
    match kind {
        WeaponKind::Blade => {
            line(c, s, vec2(-0.32, 0.32), vec2(0.34, -0.34), 0.13, col);
            line(c, s, vec2(-0.3, 0.06), vec2(-0.06, 0.3), 0.09, dark);
            line(c, s, vec2(-0.42, 0.42), vec2(-0.26, 0.26), 0.11, dark);
        }
        WeaponKind::Daggers => {
            for dx in [-0.18, 0.12] {
                line(c, s, vec2(dx - 0.12, 0.34), vec2(dx + 0.14, -0.34), 0.09, col);
                line(c, s, vec2(dx - 0.2, 0.18), vec2(dx + 0.02, 0.24), 0.07, dark);
            }
        }
        WeaponKind::Orb => {
            circle(c, s, Vec2::ZERO, 0.36, darker(col, 0.5));
            circle(c, s, Vec2::ZERO, 0.26, col);
            circle(c, s, vec2(-0.08, -0.08), 0.09, WHITE);
        }
        WeaponKind::Chime => {
            circle(c, s, vec2(0.0, -0.08), 0.24, col);
            tri(c, s, vec2(-0.34, 0.24), vec2(0.34, 0.24), vec2(0.0, -0.22), col);
            circle(c, s, vec2(0.0, 0.32), 0.07, dark);
            line(c, s, vec2(0.0, -0.4), vec2(0.0, -0.3), 0.08, dark);
        }
        WeaponKind::Lanterns => {
            arc(c, s, vec2(0.0, -0.2), 0.14, PI, TAU, 0.06, dark);
            draw_rectangle(c.x - 0.18 * s, c.y - 0.16 * s, 0.36 * s, 0.44 * s, dark);
            draw_rectangle(c.x - 0.12 * s, c.y - 0.1 * s, 0.24 * s, 0.32 * s, col);
        }
        WeaponKind::Spear => {
            let pts = [vec2(0.12, -0.42), vec2(-0.14, 0.02), vec2(0.06, 0.02), vec2(-0.12, 0.42)];
            for w in pts.windows(2) {
                line(c, s, w[0], w[1], 0.12, col);
            }
        }
        WeaponKind::Axe => {
            line(c, s, vec2(-0.3, 0.38), vec2(0.18, -0.3), 0.08, dark);
            arc(c, s, vec2(0.12, -0.22), 0.26, -0.4, 2.0, 0.14, col);
        }
        WeaponKind::Bolt => {
            tri(c, s, vec2(-0.38, 0.3), vec2(0.06, -0.02), vec2(-0.1, 0.12), darker(col, 0.6));
            circle(c, s, vec2(0.14, -0.14), 0.2, col);
            circle(c, s, vec2(0.14, -0.14), 0.08, WHITE);
        }
        WeaponKind::Scythe => {
            line(c, s, vec2(-0.24, 0.4), vec2(0.12, -0.36), 0.08, dark);
            arc(c, s, vec2(-0.08, -0.32), 0.26, -0.3, 1.2, 0.12, col);
        }
        WeaponKind::Hex => {
            let hex: Vec<Vec2> = (0..7).map(|k| Vec2::from_angle(k as f32 / 6.0 * TAU + PI / 6.0) * 0.36).collect();
            for w in hex.windows(2) {
                line(c, s, w[0], w[1], 0.08, col);
            }
            circle(c, s, Vec2::ZERO, 0.1, col);
        }
        WeaponKind::Lance => {
            line(c, s, vec2(-0.38, 0.38), vec2(0.22, -0.22), 0.09, dark);
            tri(c, s, vec2(0.38, -0.38), vec2(0.12, -0.26), vec2(0.26, -0.12), col);
        }
        WeaponKind::Crossbow => {
            arc(c, s, vec2(0.0, 0.32), 0.42, PI + 0.5, TAU - 0.5, 0.09, col);
            line(c, s, vec2(0.0, 0.4), vec2(0.0, -0.4), 0.08, dark);
            tri(c, s, vec2(0.0, -0.44), vec2(-0.08, -0.3), vec2(0.08, -0.3), col);
        }
    }
}

pub fn item_icon(kind: ItemKind, c: Vec2, s: f32, col: Color) {
    let dark = darker(col, 0.55);
    match kind {
        ItemKind::EmberHeart => {
            circle(c, s, vec2(-0.14, -0.08), 0.17, col);
            circle(c, s, vec2(0.14, -0.08), 0.17, col);
            tri(c, s, vec2(-0.3, 0.0), vec2(0.3, 0.0), vec2(0.0, 0.34), col);
        }
        ItemKind::Hourglass => {
            tri(c, s, vec2(-0.26, -0.34), vec2(0.26, -0.34), vec2(0.0, 0.0), col);
            tri(c, s, vec2(-0.26, 0.34), vec2(0.26, 0.34), vec2(0.0, 0.0), col);
            line(c, s, vec2(-0.32, -0.38), vec2(0.32, -0.38), 0.06, dark);
            line(c, s, vec2(-0.32, 0.38), vec2(0.32, 0.38), 0.06, dark);
        }
        ItemKind::Boots => {
            draw_rectangle(c.x - 0.18 * s, c.y - 0.34 * s, 0.22 * s, 0.5 * s, col);
            draw_rectangle(c.x - 0.18 * s, c.y + 0.08 * s, 0.46 * s, 0.2 * s, col);
            line(c, s, vec2(-0.2, 0.32), vec2(0.3, 0.32), 0.06, dark);
        }
        ItemKind::Lens => {
            ring(c, s, vec2(-0.06, -0.06), 0.22, 0.08, col);
            line(c, s, vec2(0.1, 0.1), vec2(0.34, 0.34), 0.1, dark);
        }
        ItemKind::Sigil => {
            ring(c, s, vec2(-0.13, 0.0), 0.2, 0.07, col);
            ring(c, s, vec2(0.13, 0.0), 0.2, 0.07, col);
        }
        ItemKind::Censer => {
            line(c, s, vec2(0.0, -0.42), vec2(0.0, -0.16), 0.05, dark);
            circle(c, s, vec2(0.0, 0.06), 0.24, col);
            draw_rectangle(c.x - 0.26 * s, c.y - 0.2 * s, 0.52 * s, 0.14 * s, dark);
        }
        ItemKind::Shell => {
            tri(c, s, vec2(-0.32, -0.3), vec2(0.32, -0.3), vec2(0.0, 0.38), col);
            draw_rectangle(c.x - 0.32 * s, c.y - 0.36 * s, 0.64 * s, 0.1 * s, col);
            line(c, s, vec2(0.0, -0.28), vec2(0.0, 0.24), 0.06, dark);
        }
        ItemKind::Feather => {
            line(c, s, vec2(-0.3, 0.36), vec2(0.28, -0.32), 0.05, dark);
            tri(c, s, vec2(0.3, -0.36), vec2(-0.18, 0.12), vec2(0.0, -0.02), col);
            tri(c, s, vec2(0.3, -0.36), vec2(0.06, 0.22), vec2(0.0, -0.02), col);
        }
        ItemKind::Moss => {
            circle(c, s, vec2(-0.14, 0.08), 0.18, col);
            circle(c, s, vec2(0.12, 0.12), 0.16, col);
            circle(c, s, vec2(0.0, -0.1), 0.18, darker(col, 0.8));
        }
        ItemKind::Lung => {
            circle(c, s, vec2(-0.16, 0.06), 0.18, col);
            circle(c, s, vec2(0.16, 0.06), 0.18, col);
            line(c, s, vec2(0.0, -0.36), vec2(0.0, -0.06), 0.07, dark);
        }
        ItemKind::LeechFang => {
            tri(c, s, vec2(-0.26, -0.3), vec2(0.04, -0.3), vec2(-0.12, 0.36), col);
            tri(c, s, vec2(0.0, -0.3), vec2(0.28, -0.3), vec2(0.16, 0.24), darker(col, 0.8));
        }
        ItemKind::CrackedCrown => {
            draw_rectangle(c.x - 0.32 * s, c.y + 0.02 * s, 0.64 * s, 0.22 * s, col);
            for x in [-0.32, 0.0, 0.32] {
                tri(c, s, vec2(x - 0.12, 0.04), vec2(x + 0.12, 0.04), vec2(x, -0.3), col);
            }
            line(c, s, vec2(-0.06, 0.04), vec2(0.04, 0.22), 0.05, dark);
        }
        ItemKind::GraveCandle => {
            draw_rectangle(c.x - 0.12 * s, c.y - 0.08 * s, 0.24 * s, 0.44 * s, Color::new(0.85, 0.82, 0.75, 1.0));
            circle(c, s, vec2(0.0, -0.22), 0.11, col);
            circle(c, s, vec2(0.0, -0.22), 0.05, WHITE);
        }
    }
}
