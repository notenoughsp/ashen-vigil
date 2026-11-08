//! Events on the field: the Blood Moon, cursed shrines and the wandering merchant.

use super::*;

/// Blood Moons rise at these times (seconds) and last `MOON_LENGTH`.
const MOONS: [f32; 2] = [480.0, 1080.0];
pub const MOON_LENGTH: f32 = 60.0;
const CURSED_SHRINES: [f32; 3] = [240.0, 780.0, 1320.0];
pub const CURSE_LENGTH: f32 = 40.0;
const MERCHANTS: [f32; 5] = [300.0, 660.0, 960.0, 1260.0, 1560.0];
pub const MERCHANT_STAY: f32 = 90.0;
/// The wandering merchant's name.
pub const MERCHANT_NAME: &str = "Ksur";
/// How many wares the merchant lays out.
pub const SHELF: usize = 6;

/// A shrine that asks for a trial: survive its curse, take its reward.
pub struct CursedShrine {
    pub pos: Vec2,
    pub life: f32,
    pub kindle: f32,
    /// Seconds of curse left once it has been accepted.
    pub curse: f32,
    pub accepted: bool,
}

/// What the merchant may sell; each visit lays out `SHELF` of them.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Ware {
    Draught,
    Relic,
    Coffer,
    /// Two more rerolls of the level-up cards.
    Dice,
    /// Fury for half a minute.
    Rage,
    /// Haste for half a minute.
    Wings,
    /// A ward for eight seconds.
    Ward,
    /// Every soul on the field flies to you.
    Lodestone,
}

pub const WARES: [Ware; 8] = [Ware::Draught, Ware::Relic, Ware::Coffer, Ware::Dice, Ware::Rage, Ware::Wings, Ware::Ward, Ware::Lodestone];

impl Ware {
    pub fn name(self) -> &'static str {
        match self {
            Ware::Draught => "Ashen Draught",
            Ware::Relic => "Relic",
            Ware::Coffer => "Coffer",
            Ware::Dice => "Bone Dice",
            Ware::Rage => "Draught of Rage",
            Ware::Wings => "Ashen Wings",
            Ware::Ward => "Warding Charm",
            Ware::Lodestone => "Lodestone",
        }
    }

    fn base_price(self) -> f32 {
        match self {
            Ware::Draught => 250.0,
            Ware::Relic => 700.0,
            Ware::Coffer => 1400.0,
            Ware::Dice => 500.0,
            Ware::Rage => 450.0,
            Ware::Wings => 350.0,
            Ware::Ward => 400.0,
            Ware::Lodestone => 300.0,
        }
    }

    pub fn from_u8(v: u8) -> Ware {
        WARES.get(v as usize).copied().unwrap_or(Ware::Draught)
    }

    pub fn to_u8(self) -> u8 {
        WARES.iter().position(|w| *w == self).unwrap_or(0) as u8
    }
}

pub struct Merchant {
    pub pos: Vec2,
    pub life: f32,
    pub wares: [Ware; SHELF],
    pub sold: [bool; SHELF],
    pub prices: [u32; SHELF],
}

impl Merchant {
    /// The pedestals stand in an arc in front of the merchant.
    pub fn pedestal(&self, i: usize) -> Vec2 {
        let k = i as f32 - (SHELF as f32 - 1.0) * 0.5;
        self.pos + vec2(k * 54.0, 52.0 - k.abs() * k.abs() * 4.0)
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
        k * self.mutated(Mutator::Swarm, 1.25)
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
            // A draught and a relic always, four of the rest by chance.
            let mut rest: Vec<Ware> = WARES[2..].to_vec();
            let mut wares = [Ware::Draught, Ware::Relic, Ware::Coffer, Ware::Dice, Ware::Rage, Ware::Wings];
            for slot in wares.iter_mut().skip(2) {
                *slot = rest.remove(gen_range(0, rest.len()));
            }
            let prices = wares.map(|w| (w.base_price() * scale / 10.0).round() as u32 * 10);
            self.merchant = Some(Merchant { pos, life: MERCHANT_STAY, wares, sold: [false; SHELF], prices });
            self.toast(&format!("{} the Ash Peddler has set up shop. Walk onto a ware to buy it with carried souls.", MERCHANT_NAME));
        }
        let mut bought = Vec::new();
        if let Some(m) = self.merchant.as_mut() {
            m.life -= dt;
            for (i, ware) in m.wares.into_iter().enumerate() {
                if m.sold[i] {
                    continue;
                }
                let at = m.pedestal(i);
                // Barter makes the same ware cheaper for whoever has it.
                let price_for = |h: &Hero| (m.prices[i] as f32 * (1.0 - 0.08 * h.profile.m(M_BARTER) as f32)).round() as u32;
                if let Some(h) = self.heroes.iter().position(|h| h.alive() && h.p.pos.distance(at) < 22.0 && h.p.souls >= price_for(h) as u64) {
                    m.sold[i] = true;
                    bought.push((h, ware, price_for(&self.heroes[h])));
                }
            }
            // An empty shelf: the merchant packs up and goes after a short farewell.
            let sold_out = m.sold.iter().all(|s| *s) && m.life > 3.0;
            if sold_out {
                m.life = 3.0;
            }
            if m.life <= 0.0 {
                self.merchant = None;
            }
            if sold_out {
                self.toast(&format!("{} has sold everything and moves on.", MERCHANT_NAME));
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
                Ware::Dice => self.heroes[h].rerolls += 2,
                Ware::Rage => self.heroes[h].buffs[Buff::Fury as usize] += 30.0,
                Ware::Wings => self.heroes[h].buffs[Buff::Haste as usize] += 30.0,
                Ware::Ward => self.heroes[h].buffs[Buff::Aegis as usize] += 8.0,
                Ware::Lodestone => {
                    let pos = self.heroes[h].p.pos;
                    self.pickups.push(Pickup::new(pos, PickKind::Magnet));
                }
            }
            let line = format!("{} buys {} from {} for {} souls", self.heroes[h].name, ware.name(), MERCHANT_NAME, price);
            self.log_event(&line);
            let who = if self.multiplayer() { format!("{} buys ", self.heroes[h].name) } else { "Bought ".to_string() };
            self.toast(&format!("{}{} for {} souls", who, ware.name(), price));
            self.sfx_at("chest", 0.7, self.heroes[h].p.pos);
        }
    }
}
