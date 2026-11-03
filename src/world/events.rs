//! Events on the field: the Blood Moon, cursed shrines and the wandering merchant.

use super::*;

/// Blood Moons rise at these times (seconds) and last `MOON_LENGTH`.
const MOONS: [f32; 2] = [480.0, 1080.0];
pub const MOON_LENGTH: f32 = 60.0;
const CURSED_SHRINES: [f32; 3] = [240.0, 780.0, 1320.0];
pub const CURSE_LENGTH: f32 = 40.0;
const MERCHANTS: [f32; 3] = [360.0, 960.0, 1560.0];
pub const MERCHANT_STAY: f32 = 75.0;

/// A shrine that asks for a trial: survive its curse, take its reward.
pub struct CursedShrine {
    pub pos: Vec2,
    pub life: f32,
    pub kindle: f32,
    /// Seconds of curse left once it has been accepted.
    pub curse: f32,
    pub accepted: bool,
}

/// The wares a merchant offers, in pedestal order.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Ware {
    Draught,
    Relic,
    Coffer,
}

pub const WARES: [Ware; 3] = [Ware::Draught, Ware::Relic, Ware::Coffer];

impl Ware {
    pub fn name(self) -> &'static str {
        match self {
            Ware::Draught => "Ashen Draught",
            Ware::Relic => "Relic",
            Ware::Coffer => "Coffer",
        }
    }

    fn base_price(self) -> f32 {
        match self {
            Ware::Draught => 250.0,
            Ware::Relic => 700.0,
            Ware::Coffer => 1400.0,
        }
    }
}

pub struct Merchant {
    pub pos: Vec2,
    pub life: f32,
    pub sold: [bool; 3],
    pub prices: [u32; 3],
}

impl Merchant {
    pub fn pedestal(&self, i: usize) -> Vec2 {
        self.pos + [vec2(-64.0, 46.0), vec2(0.0, 58.0), vec2(64.0, 46.0)][i]
    }
}

fn passed(time: f32, dt: f32, at: f32) -> bool {
    time >= at && time - dt < at
}

impl World {
    /// Enemies come faster and souls pay double under the Blood Moon or a curse.
    pub fn spawn_pressure(&self) -> f32 {
        let mut k = 1.0;
        if self.blood_moon > 0.0 {
            k *= 1.6;
        }
        if self.cursed.as_ref().map(|c| c.curse > 0.0).unwrap_or(false) {
            k *= 2.0;
        }
        k
    }

    pub(crate) fn update_events(&mut self, dt: f32) {
        let t = self.time;
        // Blood Moon.
        if MOONS.iter().any(|m| passed(t, dt, *m)) {
            self.blood_moon = MOON_LENGTH;
            self.banner("THE BLOOD MOON RISES", "Souls are doubled. So is the hunger of the dead.", Color::from_rgba(200, 30, 30, 255), 4.0);
            self.sfx("roar", 0.8);
        }
        if self.blood_moon > 0.0 {
            self.blood_moon -= dt;
            if self.blood_moon <= 0.0 {
                self.blood_moon = 0.0;
                if self.heroes.iter().any(|h| h.alive()) {
                    self.moons_survived += 1;
                    self.banner("THE MOON WANES", "", Color::from_rgba(200, 120, 110, 255), 2.5);
                }
            }
        }

        // Cursed shrines.
        if CURSED_SHRINES.iter().any(|c| passed(t, dt, *c)) && self.cursed.is_none() {
            let pos = clamp_world(self.center() + rand_dir() * 450.0, 220.0);
            self.cursed = Some(CursedShrine { pos, life: 75.0, kindle: 0.0, curse: 0.0, accepted: false });
            self.toast("A cursed shrine calls for a trial. Stand in it to accept.");
        }
        let alive = self.alive_positions();
        let mut accepted = false;
        let mut finished = false;
        if let Some(c) = self.cursed.as_mut() {
            if !c.accepted {
                c.life -= dt;
                if alive.iter().any(|p| p.distance(c.pos) < 50.0) {
                    c.kindle += dt;
                    if c.kindle >= 1.5 {
                        c.accepted = true;
                        c.curse = CURSE_LENGTH;
                        accepted = true;
                    }
                } else {
                    c.kindle = (c.kindle - dt).max(0.0);
                }
            } else {
                c.curse -= dt;
                finished = c.curse <= 0.0;
            }
        }
        if accepted {
            self.banner("THE TRIAL BEGINS", "Survive the curse.", Color::from_rgba(170, 80, 220, 255), 3.0);
            self.sfx("bell", 0.9);
        }
        if finished {
            self.cursed = None;
            if self.heroes.iter().any(|h| h.alive()) {
                for h in 0..self.heroes.len() {
                    let pos = self.heroes[h].p.pos + rand_dir() * 30.0;
                    self.pickups.push(Pickup::new(pos, PickKind::Coffer(true)));
                    self.heroes[h].p.souls += 600;
                }
                self.trials_won += 1;
                self.banner("TRIAL ENDURED", "The shrine pays its debt.", Color::from_rgba(200, 150, 255, 255), 3.5);
                self.sfx("evolve", 0.8);
            }
        } else if self.cursed.as_ref().map(|c| !c.accepted && c.life <= 0.0).unwrap_or(false) {
            self.cursed = None;
        }

        // The merchant.
        if MERCHANTS.iter().any(|m| passed(t, dt, *m)) {
            let pos = clamp_world(self.center() + rand_dir() * 420.0, 220.0);
            let scale = 1.0 + self.time / 900.0;
            let prices = WARES.map(|w| (w.base_price() * scale / 10.0).round() as u32 * 10);
            self.merchant = Some(Merchant { pos, life: MERCHANT_STAY, sold: [false; 3], prices });
            self.toast("A merchant has set up shop. Walk onto a ware to buy it with carried souls.");
        }
        let mut bought = Vec::new();
        if let Some(m) = self.merchant.as_mut() {
            m.life -= dt;
            for (i, ware) in WARES.into_iter().enumerate() {
                if m.sold[i] {
                    continue;
                }
                let at = m.pedestal(i);
                if let Some(h) = self.heroes.iter().position(|h| h.alive() && h.p.pos.distance(at) < 22.0 && h.p.souls >= m.prices[i] as u64) {
                    m.sold[i] = true;
                    bought.push((h, ware, m.prices[i]));
                }
            }
            if m.life <= 0.0 {
                self.merchant = None;
            }
        }
        for (h, ware, price) in bought {
            self.heroes[h].p.souls -= price as u64;
            self.heroes[h].purchases += 1;
            match ware {
                Ware::Draught => {
                    let max = self.heroes[h].stats.max_hp;
                    self.heroes[h].heal(max);
                }
                Ware::Relic => self.grant_relic(h),
                Ware::Coffer => {
                    let pos = self.heroes[h].p.pos;
                    self.pickups.push(Pickup::new(pos, PickKind::Coffer(false)));
                }
            }
            let who = if self.multiplayer() { format!("{} buys ", self.heroes[h].name) } else { "Bought ".to_string() };
            self.toast(&format!("{}{} for {} souls", who, ware.name(), price));
            self.sfx("chest", 0.7);
        }
    }
}
