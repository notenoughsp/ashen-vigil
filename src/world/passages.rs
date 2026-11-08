//! Passages: ways out of the night, into a hidden chamber, and onto another field.
//!
//! - The ash gate: twice a vigil an exit opens for a minute. If every hero still standing
//!   waits in it, the party leaves the night early with every carried soul banked, but
//!   without the dawn.
//! - The rift: now and then a crack in the air. Stand in it and fog seals a small chamber
//!   around it for half a minute: gilded guardians come in waves, and whoever outlasts them
//!   finds a great hoard.
//! - The wandering portal: once a vigil a gate to another field. If every hero still standing
//!   steps in, the rest of the night is kept there, with that field's foes and lords.
//!
//! The host runs them; clients see their state in snapshots.

use super::*;

/// When the ash gate opens (seconds into a vigil) and how long it stays.
const GATE_AT: [f32; 2] = [750.0, 1350.0];
const GATE_STAY: f32 = 60.0;
pub const GATE_RADIUS: f32 = 60.0;
pub const GATE_KINDLE: f32 = 3.0;

/// The rift: how long it waits to be entered, the chamber's size and how long it holds.
const RIFT_STAY: f32 = 40.0;
pub const RIFT_KINDLE: f32 = 1.5;
pub const CHAMBER_RADIUS: f32 = 300.0;
pub const CHAMBER_TIME: f32 = 30.0;
/// Seconds between two waves of guardians in the chamber.
const CHAMBER_WAVE: f32 = 5.0;

/// The portal opens once a vigil (between 11:00 and 18:00) and waits this long.
const PORTAL_STAY: f32 = 45.0;
pub const PORTAL_RADIUS: f32 = 60.0;
pub const PORTAL_KINDLE: f32 = 2.0;

/// An exit from the night.
pub struct AshGate {
    pub pos: Vec2,
    pub life: f32,
    pub kindle: f32,
}

/// A crack in the air; entered, it seals into a chamber.
pub struct Rift {
    pub pos: Vec2,
    pub life: f32,
    pub kindle: f32,
    /// Seconds the chamber still holds; zero until it is entered.
    pub sealed: f32,
    /// Seconds until the next wave of guardians.
    pub wave: f32,
}

/// A gate to another field.
pub struct Portal {
    pub pos: Vec2,
    pub life: f32,
    pub kindle: f32,
    pub to: Biome,
}

impl World {
    pub(crate) fn update_passages(&mut self, dt: f32) {
        if self.boss_rush {
            return;
        }
        self.update_gate(dt);
        self.update_rift(dt);
        self.update_portal(dt);
    }

    /// Whether every hero still standing is within `r` of `pos` (and at least one stands).
    fn party_within(&self, pos: Vec2, r: f32) -> bool {
        let alive: Vec<Vec2> = self.alive_positions();
        !alive.is_empty() && alive.iter().all(|p| p.distance(pos) < r)
    }

    fn update_gate(&mut self, dt: f32) {
        let t = self.time;
        if !self.endless && GATE_AT.iter().any(|at| t >= *at && t - dt < *at) && self.gate.is_none() {
            let (at, _, _) = self.random_hero_pos();
            let pos = clamp_world(at + rand_dir() * gen_range(380.0, 520.0), 200.0);
            self.gate = Some(AshGate { pos, life: GATE_STAY, kindle: 0.0 });
            self.log_event("an ash gate opens");
            let all = if self.heroes.len() > 1 { "Everyone still standing must" } else { "You can" };
            self.banner("AN ASH GATE OPENS", &format!("{} step through to leave the night with every carried soul", all), Color::from_rgba(150, 230, 210, 255), 4.0);
            self.sfx("chime", 0.9);
        }
        let Some(g) = self.gate.as_ref() else { return };
        let (pos, life) = (g.pos, g.life - dt);
        let inside = self.party_within(pos, GATE_RADIUS);
        let busy = self.boss.is_some();
        let g = self.gate.as_mut().expect("checked above");
        g.life = life;
        if inside && !busy {
            g.kindle += dt;
        } else {
            g.kindle = (g.kindle - dt * 2.0).max(0.0);
        }
        if g.kindle >= GATE_KINDLE {
            self.gate = None;
            self.withdraw();
        } else if life <= 0.0 {
            self.gate = None;
            self.toast("The ash gate closes.");
        }
    }

    /// The party leaves through the ash gate: every carried soul is banked, and the vigil
    /// ends without the dawn.
    fn withdraw(&mut self) {
        for h in self.heroes.iter_mut() {
            h.banked = h.banked.saturating_add(h.p.souls);
            h.p.souls = 0;
        }
        let line = format!("the party leaves through the ash gate at level {}", self.level);
        self.log_event(&line);
        self.withdrew = true;
        self.dead = true;
        self.sfx("bell", 1.0);
        self.events.push(Event::Died);
    }

    fn update_rift(&mut self, dt: f32) {
        let t = self.time;
        // A rift only opens when the chamber is sure to close before the next lord.
        let room = self.next_boss - t > RIFT_STAY + CHAMBER_TIME + 15.0;
        if self.rift.is_none() && t >= self.next_rift && t > 240.0 && self.boss.is_none() && self.arena.is_none() && room {
            self.next_rift = t + gen_range(360.0, 540.0);
            let (at, _, _) = self.random_hero_pos();
            let pos = clamp_world(at + rand_dir() * gen_range(360.0, 520.0), CHAMBER_RADIUS + 60.0);
            self.rift = Some(Rift { pos, life: RIFT_STAY, kindle: 0.0, sealed: 0.0, wave: 0.0 });
            self.log_event("a rift opens");
            self.toast("A rift tears the air. Stand in it to seal yourself in with what it hides.");
            self.sfx("chime", 0.7);
        } else if self.rift.is_none() && t >= self.next_rift {
            // Not now: try again a little later.
            self.next_rift = t + 30.0;
        }
        let Some(r) = self.rift.as_ref() else { return };
        let (pos, sealed) = (r.pos, r.sealed);
        if sealed <= 0.0 {
            let on = self.heroes.iter().any(|h| h.alive() && h.p.pos.distance(pos) < 40.0);
            let r = self.rift.as_mut().expect("checked above");
            r.life -= dt;
            if on {
                r.kindle += dt;
            } else {
                r.kindle = (r.kindle - dt * 2.0).max(0.0);
            }
            if r.kindle >= RIFT_KINDLE {
                r.sealed = CHAMBER_TIME;
                r.wave = 0.0;
                self.seal_chamber(pos);
            } else if r.life <= 0.0 {
                self.rift = None;
            }
            return;
        }
        // A lord breaks in: the chamber gives up its hoard at once.
        if self.boss.is_some() {
            self.open_chamber(pos);
            return;
        }
        // Inside the chamber: waves of guardians, the outside held back by the fog.
        let r = self.rift.as_mut().expect("checked above");
        r.sealed -= dt;
        r.wave -= dt;
        let (left, wave) = (r.sealed, r.wave <= 0.0);
        if wave && left > 3.0 {
            self.rift.as_mut().expect("checked above").wave = CHAMBER_WAVE;
            self.chamber_wave(pos);
        }
        for e in self.enemies.iter_mut().filter(|e| !e.dead && !e.boss && !self.guards.contains(&e.id)) {
            let d = e.pos - pos;
            let edge = CHAMBER_RADIUS + e.r + 6.0;
            if d.length() < edge {
                let n = if d == Vec2::ZERO { vec2(1.0, 0.0) } else { d.normalize() };
                e.pos = pos + n * edge;
            }
        }
        if left <= 0.0 {
            self.open_chamber(pos);
        }
    }

    /// The fog closes round the rift: everyone is drawn inside, every ordinary foe within is
    /// swept away.
    fn seal_chamber(&mut self, pos: Vec2) {
        self.arena = Some((pos, CHAMBER_RADIUS));
        for e in self.enemies.iter_mut().filter(|e| !e.dead && !e.boss && e.pos.distance(pos) < CHAMBER_RADIUS + 40.0) {
            if matches!(e.kind, EnemyKind::Courier | EnemyKind::Champion | EnemyKind::Rival | EnemyKind::Mimic(_)) {
                continue;
            }
            e.dead = true;
            self.fx.push(Fx { pos: e.pos, life: 0.25, max_life: 0.25, kind: FxKind::Ghost });
        }
        self.enemies.retain(|e| !e.dead);
        self.ebullets.retain(|b| b.pos.distance(pos) > CHAMBER_RADIUS);
        self.log_event("the rift seals into a chamber");
        self.banner("THE RIFT SEALS", &format!("Outlast its guardians for {:.0} seconds", CHAMBER_TIME), Color::from_rgba(190, 150, 255, 255), 3.5);
        self.fx.push(Fx { pos, life: 0.8, max_life: 0.8, kind: FxKind::Ring { r: CHAMBER_RADIUS, color: Color::from_rgba(190, 150, 255, 255) } });
        self.sfx("bell", 0.9);
    }

    /// A wave of gilded guardians steps out of the chamber's fog.
    fn chamber_wave(&mut self, pos: Vec2) {
        let t = self.time;
        let kinds = [EnemyKind::Knight, EnemyKind::Wraith, EnemyKind::Gargoyle, EnemyKind::Ghoul];
        let n = 3 + self.heroes.len().max(1) - 1 + (t / 600.0) as usize;
        let start = gen_range(0.0, TAU);
        for k in 0..n.min(8) {
            let kind = kinds[gen_range(0, kinds.len())];
            let at = pos + Vec2::from_angle(start + k as f32 / n as f32 * TAU) * (CHAMBER_RADIUS - 40.0);
            let mut e = self.make_enemy(kind, at);
            e.elite = true;
            e.hp *= 6.0;
            e.max_hp = e.hp;
            e.r = kind.radius(true);
            e.dmg *= 1.3;
            e.xp *= 6.0;
            self.guards.push(e.id);
            self.enemies.push(e);
        }
        self.sfx_at("roar", 0.5, pos);
    }

    /// The chamber holds no longer: the fog lifts and its hoard lies at its heart.
    fn open_chamber(&mut self, pos: Vec2) {
        self.rift = None;
        if self.boss.is_none() {
            self.arena = None;
        }
        let guards = std::mem::take(&mut self.guards);
        for e in self.enemies.iter_mut().filter(|e| guards.contains(&e.id)) {
            e.dead = true;
            self.fx.push(Fx { pos: e.pos, life: 0.25, max_life: 0.25, kind: FxKind::Ghost });
        }
        self.enemies.retain(|e| !e.dead);
        if !self.heroes.iter().any(|h| h.alive()) {
            return;
        }
        for (k, boss) in [(0, true), (1, false), (2, false), (3, false)] {
            let off = Vec2::from_angle(k as f32 * TAU / 4.0) * 40.0;
            self.pickups.push(Pickup::new(pos + off, PickKind::Coffer(boss)));
        }
        for _ in 0..40 {
            let v = 14.0 * (1.0 + self.time / 300.0);
            self.drop_soul(pos + rand_dir() * gen_range(20.0, 120.0), v);
        }
        self.chambers_won += 1;
        self.log_event("the chamber opens its hoard");
        self.banner("THE CHAMBER YIELDS", "Its hoard is yours", Color::from_rgba(240, 200, 90, 255), 3.5);
        self.sfx_at("chest", 1.0, pos);
        self.sfx("evolve", 0.8);
    }

    fn update_portal(&mut self, dt: f32) {
        let t = self.time;
        if !self.portal_done && self.portal.is_none() && t >= self.portal_at && self.boss.is_none() && self.arena.is_none() && self.next_boss - t > PORTAL_STAY + 10.0 {
            self.portal_done = true;
            let others: Vec<Biome> = ALL_BIOMES.iter().copied().filter(|b| *b != self.biome).collect();
            let to = others[gen_range(0, others.len())];
            let (at, _, _) = self.random_hero_pos();
            let pos = clamp_world(at + rand_dir() * gen_range(380.0, 520.0), 200.0);
            self.portal = Some(Portal { pos, life: PORTAL_STAY, kindle: 0.0, to });
            self.log_event(&format!("a portal to {} opens", to.name()));
            let all = if self.heroes.len() > 1 { "everyone still standing steps in" } else { "you step in" };
            self.banner("A WANDERING PORTAL", &format!("{} lies beyond. Keep the rest of the night there if {}", to.name(), all), Color::from_rgba(255, 170, 120, 255), 4.5);
            self.sfx("chime", 0.9);
        }
        let Some(p) = self.portal.as_ref() else { return };
        let (pos, to) = (p.pos, p.to);
        let inside = self.party_within(pos, PORTAL_RADIUS);
        let busy = self.boss.is_some() || self.boss_warning > 0.0;
        let p = self.portal.as_mut().expect("checked above");
        p.life -= dt;
        if inside && !busy {
            p.kindle += dt;
        } else {
            p.kindle = (p.kindle - dt * 2.0).max(0.0);
        }
        if p.kindle >= PORTAL_KINDLE {
            self.portal = None;
            self.cross_to(to, pos);
        } else if p.life <= 0.0 {
            self.portal = None;
            self.toast("The portal fades.");
        }
    }

    /// The party steps onto another field: the ground, the foes and the lords to come all
    /// belong to it now.
    fn cross_to(&mut self, to: Biome, pos: Vec2) {
        let from = self.biome;
        self.biome = to;
        self.log_event(&format!("the party crosses from {} to {}", from.name(), to.name()));
        // The old field's foes stay behind.
        for e in self.enemies.iter_mut().filter(|e| !e.dead && !e.boss) {
            if matches!(e.kind, EnemyKind::Courier | EnemyKind::Champion | EnemyKind::Rival | EnemyKind::Mimic(_)) {
                continue;
            }
            e.dead = true;
        }
        self.enemies.retain(|e| !e.dead);
        self.ebullets.clear();
        self.decals.clear();
        self.portals_crossed += 1;
        self.banner(&to.name().to_uppercase(), to.desc(), Color::from_rgba(255, 190, 140, 255), 4.0);
        self.fx.push(Fx { pos, life: 1.0, max_life: 1.0, kind: FxKind::Ring { r: 400.0, color: Color::from_rgba(255, 170, 120, 255) } });
        self.sfx("evolve", 0.9);
        self.shake = 10.0;
    }
}
