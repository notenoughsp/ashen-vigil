//! Duo strikes: two heroes who keep close together charge a shared strike, and when it is
//! full it breaks loose by itself. What it is depends on the pair's callings.
//!
//! The host charges and fires them; clients see the bond's charge in snapshots and the strike
//! through its effects.

use super::*;

/// How close two heroes must stay, and the seconds of standing together it takes to charge.
pub const DUO_RANGE: f32 = 160.0;
pub const DUO_CHARGE: f32 = 14.0;

/// What a class brings to a duo strike.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Calling {
    Steel,
    Flame,
    Faith,
    Hunt,
}

impl Class {
    pub fn calling(self) -> Calling {
        match self {
            Class::Wanderer | Class::BloodKnight | Class::BucketKnight | Class::Reaper => Calling::Steel,
            Class::CinderAdept | Class::Hexblade | Class::Stormcaller => Calling::Flame,
            Class::BellAcolyte | Class::LanternWarden => Calling::Faith,
            Class::GraveHunter | Class::Arbalist => Calling::Hunt,
        }
    }
}

/// The strike a pair of callings makes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum DuoStrike {
    /// A wave of ash round both, flinging foes back.
    AshNova,
    /// A chain of light between the two that burns all it crosses.
    ChainOfLight,
    /// Both are warded, healed and roused.
    TwinWard,
    /// Every foe near either is marked: wounded and slowed.
    HuntersMark,
}

impl DuoStrike {
    pub fn of(a: Calling, b: Calling) -> DuoStrike {
        use Calling::*;
        match (a, b) {
            (Faith, Faith) | (Steel, Faith) | (Faith, Steel) => DuoStrike::TwinWard,
            (Faith, _) | (_, Faith) => DuoStrike::ChainOfLight,
            (Hunt, _) | (_, Hunt) => DuoStrike::HuntersMark,
            _ => DuoStrike::AshNova,
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            DuoStrike::AshNova => "Ash Nova",
            DuoStrike::ChainOfLight => "Chain of Light",
            DuoStrike::TwinWard => "Twin Ward",
            DuoStrike::HuntersMark => "Hunter's Mark",
        }
    }

    pub fn color(self) -> Color {
        match self {
            DuoStrike::AshNova => Color::from_rgba(255, 150, 80, 255),
            DuoStrike::ChainOfLight => Color::from_rgba(255, 240, 170, 255),
            DuoStrike::TwinWard => Color::from_rgba(140, 220, 255, 255),
            DuoStrike::HuntersMark => Color::from_rgba(200, 255, 140, 255),
        }
    }
}

/// The charge between two heroes, by slot.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Bond {
    pub a: u8,
    pub b: u8,
    pub charge: f32,
}

impl World {
    pub(crate) fn update_duos(&mut self, dt: f32) {
        if self.heroes.len() < 2 {
            self.bonds.clear();
            return;
        }
        let mut fire = Vec::new();
        for i in 0..self.heroes.len() {
            for j in i + 1..self.heroes.len() {
                let (hi, hj) = (&self.heroes[i], &self.heroes[j]);
                let (a, b) = (hi.slot, hj.slot);
                let close = hi.alive() && hj.alive() && hi.connected && hj.connected && hi.p.pos.distance(hj.p.pos) < DUO_RANGE;
                let k = match self.bonds.iter().position(|d| d.a == a && d.b == b) {
                    Some(k) => k,
                    None if close => {
                        self.bonds.push(Bond { a, b, charge: 0.0 });
                        self.bonds.len() - 1
                    }
                    None => continue,
                };
                let d = &mut self.bonds[k];
                if close {
                    d.charge += dt / DUO_CHARGE;
                } else {
                    d.charge -= dt / DUO_CHARGE * 0.5;
                }
                if d.charge >= 1.0 {
                    d.charge = 0.0;
                    fire.push((i, j));
                }
            }
        }
        self.bonds.retain(|d| d.charge > 0.0);
        for (i, j) in fire {
            self.duo_strike(i, j);
        }
    }

    fn duo_strike(&mut self, i: usize, j: usize) {
        let strike = DuoStrike::of(self.heroes[i].class.calling(), self.heroes[j].class.calling());
        let (pa, pb) = (self.heroes[i].p.pos, self.heroes[j].p.pos);
        let mid = (pa + pb) * 0.5;
        let color = strike.color();
        let power = 60.0 * self.hp_scale();
        let src_of = |w: &World, h: usize| Src { hero: h, weapon: w.heroes[h].weapons.first().map(|wp| wp.kind).unwrap_or(WeaponKind::Blade) };
        let (si, sj) = (src_of(self, i), src_of(self, j));
        match strike {
            DuoStrike::AshNova => {
                let r = 340.0;
                for k in 0..self.enemies.len() {
                    let e = &self.enemies[k];
                    if e.dead {
                        continue;
                    }
                    let d = e.pos - mid;
                    if d.length() < r {
                        let src = if k % 2 == 0 { si } else { sj };
                        self.hit(k, power * 1.5, d.normalize_or_zero() * 600.0, src, false);
                    }
                }
                self.fx.push(Fx { pos: mid, life: 0.6, max_life: 0.6, kind: FxKind::Explosion { r, color } });
                self.fx.push(Fx { pos: mid, life: 0.9, max_life: 0.9, kind: FxKind::Ring { r: r * 1.2, color } });
                self.sfx_at("explode", 0.9, mid);
                self.shake = self.shake.max(10.0);
            }
            DuoStrike::ChainOfLight => {
                // Everything near the line between the two, stretched past both ends.
                let dir = if pa.distance(pb) > 1.0 { (pb - pa).normalize() } else { vec2(1.0, 0.0) };
                let (from, to) = (pa - dir * 260.0, pb + dir * 260.0);
                let len = from.distance(to);
                for k in 0..self.enemies.len() {
                    let e = &self.enemies[k];
                    if e.dead {
                        continue;
                    }
                    let along = (e.pos - from).dot(dir).clamp(0.0, len);
                    let off = e.pos.distance(from + dir * along);
                    if off < 60.0 + e.r {
                        let src = if along < len * 0.5 { si } else { sj };
                        self.hit(k, power * 2.2, Vec2::ZERO, src, false);
                    }
                }
                for s in 0..4 {
                    let a = from.lerp(to, s as f32 / 4.0);
                    let b = from.lerp(to, (s + 1) as f32 / 4.0);
                    self.fx.push(Fx { pos: b, life: 0.5, max_life: 0.5, kind: FxKind::Lightning { from: a } });
                }
                self.fx.push(Fx { pos: mid, life: 0.6, max_life: 0.6, kind: FxKind::Ring { r: 160.0, color } });
                self.sfx_at("chime", 1.0, mid);
            }
            DuoStrike::TwinWard => {
                for h in [i, j] {
                    let max = self.heroes[h].stats.max_hp;
                    self.heroes[h].heal(max * 0.3);
                    self.heroes[h].buffs[Buff::Aegis as usize] += 4.0;
                    self.heroes[h].buffs[Buff::Fury as usize] += 8.0;
                    let at = self.heroes[h].p.pos;
                    self.fx.push(Fx { pos: at, life: 0.8, max_life: 0.8, kind: FxKind::Ring { r: 120.0, color } });
                }
                self.sfx_at("bell", 0.8, mid);
            }
            DuoStrike::HuntersMark => {
                for k in 0..self.enemies.len() {
                    let e = &self.enemies[k];
                    if e.dead || (e.pos.distance(pa) > 520.0 && e.pos.distance(pb) > 520.0) {
                        continue;
                    }
                    let src = if e.pos.distance(pa) < e.pos.distance(pb) { si } else { sj };
                    self.enemies[k].slow = self.enemies[k].slow.max(4.0);
                    self.hit(k, power, Vec2::ZERO, src, false);
                }
                self.fx.push(Fx { pos: pa, life: 0.7, max_life: 0.7, kind: FxKind::Ring { r: 520.0, color } });
                self.fx.push(Fx { pos: pb, life: 0.7, max_life: 0.7, kind: FxKind::Ring { r: 520.0, color } });
                self.sfx_at("arrow", 1.0, mid);
            }
        }
        self.duo_strikes += 1;
        let line = format!("{} and {}: {}", self.heroes[i].name, self.heroes[j].name, strike.name());
        self.log_event(&line);
        self.toast(&format!("{}!", line));
    }
}
