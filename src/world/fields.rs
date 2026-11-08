//! The three newer fields and what is their own: the Ashwood's smouldering ground, the Frozen
//! Abbey's ice, the Starless Garden's statues that only move unseen.
//!
//! Embers and ice lie where a hash of the ground puts them, so host and clients agree on them
//! without a word, and the floor, the hero's footing and the burning all ask the same place.

use super::*;

thread_local! {
    /// The field in play, set with the field's size (see `World::fit_field`), for the hero's
    /// footing, which runs without the world at hand.
    static FIELD_BIOME: std::cell::Cell<Biome> = const { std::cell::Cell::new(Biome::Graveyard) };
}

pub fn set_field_biome(b: Biome) {
    FIELD_BIOME.with(|c| c.set(b));
}

pub fn field_biome() -> Biome {
    FIELD_BIOME.with(|c| c.get())
}

/// The cells hazards are laid out in, and how far one reaches at most.
pub const HAZARD_CELL: f32 = 520.0;

fn cell_hash(cx: i32, cy: i32, salt: u32) -> u32 {
    let mut h = (cx as u32).wrapping_mul(0x9E37_79B1) ^ (cy as u32).wrapping_mul(0x85EB_CA77) ^ salt.wrapping_mul(0xC2B2_AE3D);
    h ^= h >> 15;
    h = h.wrapping_mul(0x2C1B_3C6D);
    h ^= h >> 12;
    h
}

fn cell_f(cx: i32, cy: i32, salt: u32) -> f32 {
    (cell_hash(cx, cy, salt) % 10_000) as f32 / 10_000.0
}

/// A patch in a cell, if the cell has one: its middle, and its two radii.
pub fn hazard_patch(cx: i32, cy: i32, salt: u32, share: f32) -> Option<(Vec2, Vec2)> {
    if cell_f(cx, cy, salt) > share {
        return None;
    }
    let c = vec2((cx as f32 + 0.2 + cell_f(cx, cy, salt + 1) * 0.6) * HAZARD_CELL, (cy as f32 + 0.2 + cell_f(cx, cy, salt + 2) * 0.6) * HAZARD_CELL);
    let r = vec2(70.0 + cell_f(cx, cy, salt + 3) * 90.0, 45.0 + cell_f(cx, cy, salt + 4) * 60.0);
    // Never at the very middle of the field, where every vigil begins.
    if c.length() < 380.0 {
        return None;
    }
    Some((c, r))
}

/// Whether `p` stands in one of the patches laid out with `salt`.
fn in_patch(p: Vec2, salt: u32, share: f32) -> bool {
    let (cx, cy) = ((p.x / HAZARD_CELL).floor() as i32, (p.y / HAZARD_CELL).floor() as i32);
    for dy in -1..=1 {
        for dx in -1..=1 {
            if let Some((c, r)) = hazard_patch(cx + dx, cy + dy, salt, share) {
                let d = p - c;
                if (d.x / r.x).powi(2) + (d.y / r.y).powi(2) < 1.0 {
                    return true;
                }
            }
        }
    }
    false
}

pub const EMBER_SALT: u32 = 401;
pub const EMBER_SHARE: f32 = 0.35;
pub const ICE_SALT: u32 = 433;
pub const ICE_SHARE: f32 = 0.45;

/// Smouldering ground in the Ashwood: it burns heroes and foes alike.
pub fn ember_at(p: Vec2) -> bool {
    field_biome() == Biome::Ashwood && in_patch(p, EMBER_SALT, EMBER_SHARE)
}

/// Ice in the Frozen Abbey: a hero on it keeps sliding the way they went.
pub fn ice_at(p: Vec2) -> bool {
    field_biome() == Biome::Abbey && in_patch(p, ICE_SALT, ICE_SHARE)
}

/// How quickly a hero on ice turns toward where they want to go (per second).
pub const ICE_GRIP: f32 = 2.2;

impl World {
    /// Embers burn whoever stands in them, a little at a time.
    pub(crate) fn update_field_hazards(&mut self, dt: f32) {
        if self.biome != Biome::Ashwood {
            return;
        }
        self.ember_tick -= dt;
        if self.ember_tick > 0.0 {
            return;
        }
        self.ember_tick = 0.5;
        for h in 0..self.heroes.len() {
            let hero = &self.heroes[h];
            if hero.alive() && ember_at(hero.p.pos) {
                let dmg = hero.stats.max_hp * 0.025;
                self.hurt_hero(h, dmg);
            }
        }
        let t = self.time;
        for e in self.enemies.iter_mut().filter(|e| !e.dead && !e.boss && !matches!(e.kind, EnemyKind::Prop(_) | EnemyKind::EmberWolf)) {
            if ember_at(e.pos) {
                e.burn = e.burn.max(1.0);
                e.burn_dps = e.burn_dps.max(e.max_hp * 0.08 + t * 0.01);
            }
        }
    }
}
