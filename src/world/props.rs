//! Things on the field that are not foes: breakable props that fit the field (graves and urns,
//! candelabra and reliquaries, egg sacs and bone piles) and four teleport runes, one near each
//! corner, each throwing a hero on to the next round the field.
//!
//! Props are foes of a kind (`EnemyKind::Prop`) so every weapon breaks them, but they never
//! move, never hurt, are never aimed at and never count as felled. The host keeps a few of each
//! around the party; clients see them in snapshots and step through runes themselves.

use super::*;

/// Props kept within reach of the party, and how far out they appear and vanish.
const PROPS_NEAR: usize = 22;
const PROP_RING: (f32, f32) = (650.0, 1300.0);
const PROP_GONE: f32 = 2600.0;
/// How big a rune is, how long it (and the one it threw to) rests after, how far in from the
/// corners the runes lie (clear of the fields' secrets) and how far beside its rune a hero lands.
pub const RUNE_RADIUS: f32 = 26.0;
pub const RUNE_REST: f32 = 25.0;
const RUNE_INSET: f32 = 700.0;
const RUNE_LAND: f32 = 75.0;

/// A rune on the floor: step on it and land beside the rune at `to`.
pub struct TeleRune {
    pub pos: Vec2,
    /// The rune it throws to.
    pub to: Vec2,
    /// Seconds until it works again.
    pub rest: f32,
}

impl TeleRune {
    pub fn ready(&self) -> bool {
        self.rest <= 0.0
    }

    /// Where a hero it throws lands: beside the far rune, toward the middle of the field, so
    /// they do not stand on it.
    pub fn landing(&self) -> Vec2 {
        self.to - self.to.normalize_or_zero() * RUNE_LAND
    }
}

/// The four runes, near the corners, each throwing to the next clockwise.
pub fn corner_runes() -> Vec<TeleRune> {
    let c = WORLD_HALF - RUNE_INSET;
    let corners = [vec2(-c, -c), vec2(c, -c), vec2(c, c), vec2(-c, c)];
    (0..4).map(|k| TeleRune { pos: corners[k], to: corners[(k + 1) % 4], rest: 0.0 }).collect()
}

impl World {
    pub(crate) fn update_props(&mut self, dt: f32) {
        if self.boss_rush || self.heroes.is_empty() {
            return;
        }
        self.prop_timer -= dt;
        if self.prop_timer <= 0.0 {
            self.prop_timer = 1.0;
            self.keep_props();
            self.keep_runes();
        }
        self.update_runes(dt);
    }

    fn keep_props(&mut self) {
        let heroes: Vec<Vec2> = self.heroes.iter().filter(|h| h.connected).map(|h| h.p.pos).collect();
        let near = |p: Vec2, d: f32| heroes.iter().any(|h| h.distance(p) < d);
        // Props far behind everyone crumble away unseen.
        for e in self.enemies.iter_mut().filter(|e| matches!(e.kind, EnemyKind::Prop(_)) && !e.dead) {
            if !near(e.pos, PROP_GONE) {
                e.dead = true;
            }
        }
        let count = self.enemies.iter().filter(|e| matches!(e.kind, EnemyKind::Prop(_)) && !e.dead && near(e.pos, PROP_RING.1 + 200.0)).count();
        for _ in count..PROPS_NEAR.min(count + 3) {
            let (at, _, _) = self.random_hero_pos();
            let pos = clamp_world(at + rand_dir() * gen_range(PROP_RING.0, PROP_RING.1), 80.0);
            if self.arena.is_some_and(|(c, r)| c.distance(pos) < r + 60.0) {
                continue;
            }
            let v = gen_range(0, 3) as u8;
            let mut e = self.make_enemy(EnemyKind::Prop(v), pos);
            e.hp = 30.0 * self.hp_scale().sqrt();
            e.max_hp = e.hp;
            self.enemies.push(e);
        }
    }

    /// A prop broke: a few souls, and now and then something better.
    pub(crate) fn break_prop(&mut self, i: usize) {
        let (pos, kind) = (self.enemies[i].pos, self.enemies[i].kind);
        let EnemyKind::Prop(v) = kind else { return };
        self.props_broken += 1;
        let worth = 2.0 * (1.0 + self.time / 300.0);
        for _ in 0..gen_range(3, 7) {
            self.drop_soul(pos + rand_dir() * gen_range(4.0, 24.0), worth);
        }
        let luck = self.heroes.iter().map(|h| h.stats.luck).fold(1.0f32, f32::max);
        let roll = gen_range(0.0, 1.0) / luck;
        let room = self.pickups.len() < MAX_PICKUPS - 50;
        if room && roll < 0.015 {
            self.pickups.push(Pickup::new(pos, PickKind::Coffer(false)));
        } else if room && roll < 0.07 {
            self.pickups.push(Pickup::new(pos, PickKind::Draught));
        } else if room && roll < 0.1 {
            self.pickups.push(Pickup::new(pos, PickKind::Magnet));
        }
        // Stone chips, splinters or gore, by field.
        let c = match self.biome {
            Biome::Graveyard => Color::from_rgba(120, 116, 110, 255),
            Biome::Cathedral => Color::from_rgba(150, 110, 70, 255),
            Biome::Flesh | Biome::Garden => Color::from_rgba(140, 40, 50, 255),
            Biome::Ashwood => Color::from_rgba(60, 50, 44, 255),
            Biome::Abbey => Color::from_rgba(200, 225, 240, 255),
        };
        self.blood_spray(pos, Vec2::ZERO, c, 14.0, 10);
        let sound = if self.biome == Biome::Flesh || v == 1 { "hit" } else { "clang" };
        self.sfx_gap_at(sound, 0.35, 0.05, pos);
    }

    fn keep_runes(&mut self) {
        if self.runes.len() != 4 {
            self.runes = corner_runes();
        }
    }

    /// Puts rune `k` and the one it throws to at rest.
    fn rest_runes(&mut self, k: usize) {
        let to = self.runes[k].to;
        for r in self.runes.iter_mut().filter(|r| r.pos.distance(to) < 1.0) {
            r.rest = RUNE_REST;
        }
        self.runes[k].rest = RUNE_REST;
    }

    fn update_runes(&mut self, dt: f32) {
        for r in self.runes.iter_mut() {
            r.rest = (r.rest - dt).max(0.0);
        }
        // Not out of a lord's fog ring.
        if self.arena.is_some() {
            return;
        }
        for k in 0..self.runes.len() {
            if !self.runes[k].ready() {
                continue;
            }
            let (pos, to) = (self.runes[k].pos, self.runes[k].landing());
            // Our own heroes step through here; a client's hero arrives by itself, so the
            // host only sees it land.
            let local = self.heroes.iter().position(|h| h.alive() && !h.remote && h.p.pos.distance(pos) < RUNE_RADIUS);
            let landed = self.heroes.iter().position(|h| h.alive() && h.remote && h.p.pos.distance(to) < 40.0);
            let Some(h) = local.or(landed) else { continue };
            if local.is_some() {
                self.heroes[h].p.pos = to;
            }
            self.heroes[h].p.iframes = self.heroes[h].p.iframes.max(1.0);
            self.rest_runes(k);
            self.runes_used += 1;
            let line = format!("{} steps through a rune", self.heroes[h].name);
            self.log_event(&line);
            self.fx.push(Fx { pos, life: 0.5, max_life: 0.5, kind: FxKind::Ring { r: 90.0, color: Color::from_rgba(110, 230, 220, 255) } });
            self.fx.push(Fx { pos: to, life: 0.6, max_life: 0.6, kind: FxKind::Ring { r: 120.0, color: Color::from_rgba(110, 230, 220, 255) } });
            self.sfx_at("chime", 0.8, to);
        }
    }

    /// A client's own step through a rune: it lands at once, the host learns it from where
    /// the hero now stands.
    pub(crate) fn client_rune(&mut self) {
        if self.arena.is_some() {
            return;
        }
        let Some(hero) = self.heroes.get(self.local) else { return };
        if !hero.alive() || self.spectating {
            return;
        }
        let at = hero.p.pos;
        if let Some(k) = self.runes.iter().position(|r| r.ready() && r.pos.distance(at) < RUNE_RADIUS) {
            let to = self.runes[k].landing();
            self.rest_runes(k);
            let h = &mut self.heroes[self.local];
            h.p.pos = to;
            h.p.iframes = h.p.iframes.max(1.0);
        }
    }
}
