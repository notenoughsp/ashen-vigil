//! Braziers on the field: stand by one to light it, and for a while it lights the dark,
//! mends the heroes near it and burns the foes that come close.

use super::*;

/// Seconds a hero must stand by a cold brazier to light it, and how long it burns.
pub const BRAZIER_KINDLE: f32 = 1.2;
pub const BRAZIER_BURN: f32 = 120.0;
/// How far its warmth reaches: healing for heroes, fire for foes.
pub const BRAZIER_REACH: f32 = 210.0;

pub struct Brazier {
    pub pos: Vec2,
    /// Seconds it still burns; 0 when cold.
    pub burn: f32,
    pub kindle: f32,
}

/// The same scattered spots on every machine: a jittered grid over the field, clear of the
/// centre where the vigil starts.
pub fn brazier_spots() -> Vec<Vec2> {
    let mix = |x: i32, y: i32, salt: u32| {
        let mut h = (x as u32).wrapping_mul(374_761_393) ^ (y as u32).wrapping_mul(668_265_263) ^ salt.wrapping_mul(2_246_822_519);
        h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
        ((h ^ (h >> 16)) % 10_000) as f32 / 10_000.0
    };
    let mut out = Vec::new();
    // About one brazier per 1200 units square: 36 on a vigil's field, 4 in a boss rush.
    let half = field_half();
    let cells = ((half * 2.0 / 1200.0).round() as i32).max(2);
    let step = half * 2.0 / cells as f32;
    for gy in 0..cells {
        for gx in 0..cells {
            let c = vec2(-half + (gx as f32 + 0.5) * step, -half + (gy as f32 + 0.5) * step);
            let p = c + vec2(mix(gx, gy, 1) - 0.5, mix(gx, gy, 2) - 0.5) * step * 0.6;
            if p.length() > 400.0 && inside_world(p, 200.0) {
                out.push(p);
            }
        }
    }
    out
}

impl World {
    pub(crate) fn update_braziers(&mut self, dt: f32) {
        let mut lit = Vec::new();
        for (i, b) in self.braziers.iter_mut().enumerate() {
            if b.burn > 0.0 {
                b.burn = (b.burn - dt).max(0.0);
                continue;
            }
            let near = self.heroes.iter().any(|h| h.alive() && h.p.pos.distance(b.pos) < 48.0);
            b.kindle = if near { b.kindle + dt } else { (b.kindle - dt).max(0.0) };
            if b.kindle >= BRAZIER_KINDLE {
                b.kindle = 0.0;
                b.burn = BRAZIER_BURN;
                lit.push(i);
            }
        }
        for i in lit {
            let p = self.braziers[i].pos;
            self.braziers_lit += 1;
            self.quest_count(QuestKind::Kindle, 1);
            let burning = self.braziers.iter().filter(|b| b.burn > 0.0).count();
            self.toast(&format!("A brazier burns again. ({} of {} alight)", burning, self.braziers.len()));
            self.sfx_at("kindle", 0.8, p);
            self.fx.push(Fx { pos: p, life: 0.6, max_life: 0.6, kind: FxKind::Ring { r: BRAZIER_REACH, color: Color::from_rgba(255, 170, 80, 255) } });
            for _ in 0..8 {
                let v = 6.0 * (1.0 + self.time / 300.0);
                self.drop_soul(p + rand_dir() * gen_range(20.0, 60.0), v);
            }
        }
        // Warmth for the heroes, fire for the foes, around every burning brazier.
        let burning: Vec<Vec2> = self.braziers.iter().filter(|b| b.burn > 0.0).map(|b| b.pos).collect();
        if burning.is_empty() {
            return;
        }
        let famine = self.cursed_by(Curse::Famine);
        for h in self.heroes.iter_mut().filter(|h| h.alive()) {
            if !famine && burning.iter().any(|b| b.distance(h.p.pos) < BRAZIER_REACH) {
                h.heal(2.0 * dt);
            }
        }
        let dps = 5.0 * (1.0 + self.time / 300.0);
        for e in self.enemies.iter_mut().filter(|e| !e.boss && !e.dead) {
            if burning.iter().any(|b| b.distance(e.pos) < BRAZIER_REACH) {
                e.burn = e.burn.max(1.0);
                e.burn_dps = e.burn_dps.max(dps);
            }
        }
    }
}
