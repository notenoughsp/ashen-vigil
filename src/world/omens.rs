//! Omens: things that happen on the field now and then, apart from the lords.
//!
//! - Blood altars: stand on one to give a quarter of your life for more might until the next
//!   lord falls. The health bar shows the part given away.
//! - Cursed coffers: better loot, but opening one calls a wave of elites.
//! - The courier: a runner who flees with a key; catch it, and the key opens a sealed vault.
//! - The mist: a sudden fog in which a champion hunts.
//! - Weather for a few minutes: ash rain (fire burns hotter) or grave mist (foes wade slower,
//!   souls are richer, the light is shorter).
//! - The rival: whoever goes long without a wound is hunted by a dark hero carrying many souls.
//! - The wager stone before each lord: bet a third of your carried souls on felling the lord
//!   within 30, 60 or 90 seconds, for three, two or one and a half times the stake.
//!
//! The host runs them; clients see their state in snapshots.

use super::*;

/// The share of life a blood pact keeps, and the might it gives.
pub const PACT_KEEP: f32 = 0.75;
pub const PACT_MIGHT: f32 = 1.35;
/// Seconds of standing on an altar to make the pact.
pub const ALTAR_KINDLE: f32 = 1.5;
const ALTAR_STAY: f32 = 60.0;
/// The courier gets away after this long.
const COURIER_ESCAPE: f32 = 35.0;
/// How long the mist lasts if its champion is not felled.
const MIST_LENGTH: f32 = 90.0;
/// Seconds without a wound before a rival comes for that hero.
pub const RIVAL_AFTER: f32 = 100.0;
const RIVAL_STAY: f32 = 90.0;

/// The wager stone's three bets: seconds to fell the lord in, and what the stake becomes.
pub const WAGERS: [(f32, f32); 3] = [(30.0, 3.0), (60.0, 2.0), (90.0, 1.5)];
/// Seconds of standing on a tile to place the bet.
const WAGER_KINDLE: f32 = 1.0;
/// The stone stays this long after its lord has come (a boss rush leaves no time before).
const WAGER_LINGER: f32 = 8.0;

/// A hero's bet on the lord at hand.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Wager {
    /// Which of `WAGERS`.
    pub tier: usize,
    pub stake: u64,
    /// When the clock started: when the lord came, or when the bet was placed during the fight.
    pub since: Option<f32>,
}

impl Wager {
    pub fn limit(&self) -> f32 {
        WAGERS[self.tier.min(2)].0
    }

    pub fn mult(&self) -> f32 {
        WAGERS[self.tier.min(2)].1
    }

    /// Seconds left to win, once the clock runs.
    pub fn left(&self, now: f32) -> Option<f32> {
        self.since.map(|s| self.limit() - (now - s))
    }
}

/// Where the wager stone stands; its three tiles are `wager_tile`.
pub struct WagerStone {
    pub pos: Vec2,
}

impl WagerStone {
    pub fn tile(&self, i: usize) -> Vec2 {
        self.pos + vec2((i as f32 - 1.0) * 110.0, 60.0)
    }
}

pub struct Altar {
    pub pos: Vec2,
    pub life: f32,
    pub kindle: f32,
}

pub struct Vault {
    pub pos: Vec2,
    pub open: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Weather {
    /// Fire weapons deal 40% more.
    AshRain,
    /// Foes are slower, souls richer, the light shorter.
    GraveMist,
}

impl Weather {
    pub fn name(self) -> &'static str {
        match self {
            Weather::AshRain => "Ash rain",
            Weather::GraveMist => "Grave mist",
        }
    }

    pub fn desc(self) -> &'static str {
        match self {
            Weather::AshRain => "fire burns 40% hotter",
            Weather::GraveMist => "foes wade slower, souls are richer, the light is short",
        }
    }

    pub fn from_u8(v: u8) -> Option<Weather> {
        match v {
            1 => Some(Weather::AshRain),
            2 => Some(Weather::GraveMist),
            _ => None,
        }
    }

    pub fn to_u8(w: Option<Weather>) -> u8 {
        match w {
            None => 0,
            Some(Weather::AshRain) => 1,
            Some(Weather::GraveMist) => 2,
        }
    }
}

impl WeaponKind {
    /// Weapons that burn, for the ash rain.
    pub fn is_fire(self) -> bool {
        matches!(self, WeaponKind::Orb | WeaponKind::Hex)
    }
}

impl World {
    pub(crate) fn update_omens(&mut self, dt: f32) {
        self.update_wagers(dt);
        if self.boss_rush {
            return;
        }
        let t = self.time;
        self.update_altars(dt);

        // A blood pact keeps a quarter of the life away.
        for h in self.heroes.iter_mut().filter(|h| h.pact) {
            h.p.hp = h.p.hp.min(h.stats.max_hp * PACT_KEEP);
        }

        // Cursed coffers.
        if t >= self.next_cursed {
            self.next_cursed = t + gen_range(150.0, 210.0);
            let (at, _, _) = self.random_hero_pos();
            let pos = clamp_world(at + rand_dir() * gen_range(320.0, 520.0), 150.0);
            self.pickups.push(Pickup::new(pos, PickKind::CursedCoffer));
            self.toast("A cursed coffer lies nearby. Its treasure is richer, but something guards it.");
        }

        // The courier and the vault.
        if t >= self.next_courier && self.boss.is_none() {
            self.next_courier = t + gen_range(330.0, 420.0);
            let (at, _, _) = self.random_hero_pos();
            let mut e = self.make_enemy(EnemyKind::Courier, clamp_world(at + rand_dir() * 380.0, 80.0));
            e.hp *= 1.0 + t / 300.0;
            e.max_hp = e.hp;
            e.life_left = COURIER_ESCAPE;
            self.courier = Some(e.id);
            self.enemies.push(e);
            let vault = clamp_world(self.center() + rand_dir() * gen_range(900.0, 1300.0), 200.0);
            self.vault = Some(Vault { pos: vault, open: false });
            self.vault_key = false;
            self.banner("A COURIER FLEES", "It carries the key to a sealed vault. Catch it!", Color::from_rgba(240, 200, 90, 255), 3.5);
            self.sfx("chime", 0.9);
        }
        if let Some(id) = self.courier {
            if !self.enemies.iter().any(|e| e.id == id && !e.dead) {
                self.courier = None;
                let dropped = self.pickups.iter().any(|p| matches!(p.kind, PickKind::Key));
                if !self.vault_key && !dropped {
                    // It got away: the vault seals itself.
                    self.vault = None;
                    self.toast("The courier got away. The vault seals itself.");
                }
            }
        }
        let opener = match &self.vault {
            Some(v) if !v.open && self.vault_key => self.heroes.iter().position(|h| h.alive() && h.p.pos.distance(v.pos) < 44.0),
            _ => None,
        };
        if let (Some(h), Some(v)) = (opener, self.vault.as_mut()) {
            v.open = true;
            let pos = v.pos;
            self.vault_key = false;
            for (k, boss) in [(0, true), (1, false), (2, false)] {
                let off = Vec2::from_angle(k as f32 * TAU / 3.0) * 34.0;
                self.pickups.push(Pickup::new(pos + off, PickKind::Coffer(boss)));
            }
            for _ in 0..25 {
                let v = 12.0 * (1.0 + t / 300.0);
                self.drop_soul(pos + rand_dir() * gen_range(20.0, 90.0), v);
            }
            let who = self.heroes[h].name.clone();
            self.banner("THE VAULT OPENS", &format!("{} turns the courier's key", who), Color::from_rgba(240, 200, 90, 255), 3.5);
            self.sfx_at("chest", 1.0, pos);
        }

        // The mist and its champion.
        if self.mist > 0.0 {
            self.mist -= dt;
            let alive = self.champion.is_some_and(|id| self.enemies.iter().any(|e| e.id == id && !e.dead));
            if !alive && self.mist > 3.0 {
                // The champion fell: the fog lifts.
                self.mist = 3.0;
                self.champion = None;
            }
            if self.mist <= 0.0 {
                self.mist = 0.0;
                if let Some(id) = self.champion.take() {
                    if let Some(e) = self.enemies.iter_mut().find(|e| e.id == id) {
                        e.dead = true;
                        self.toast("The champion withdraws into the mist.");
                    }
                }
            }
        } else if t >= self.next_mist && self.boss.is_none() && self.boss_warning <= 0.0 {
            self.next_mist = t + gen_range(360.0, 480.0);
            self.mist = MIST_LENGTH;
            let (at, _, _) = self.random_hero_pos();
            let mut e = self.make_enemy(EnemyKind::Champion, clamp_world(at + rand_dir() * 420.0, 80.0));
            let coop = 1.0 + 0.6 * (self.heroes.len().max(1) - 1) as f32;
            e.hp = 2400.0 * (1.0 + t / 300.0).powf(1.6) * self.diff.hp * coop;
            e.max_hp = e.hp;
            e.elite = true;
            e.r = EnemyKind::Champion.radius(true);
            e.xp *= 1.0 + t / 300.0;
            self.champion = Some(e.id);
            self.enemies.push(e);
            self.banner("THE MIST RISES", "A champion stalks you in the fog", Color::from_rgba(190, 200, 210, 255), 4.0);
            self.sfx("roar", 0.8);
        }

        // Weather.
        if let Some((w, left)) = self.weather.as_mut() {
            *left -= dt;
            if *left <= 0.0 {
                let name = w.name();
                self.weather = None;
                self.toast(&format!("The {} passes.", name.to_lowercase()));
            }
        } else if t >= self.next_weather {
            let w = if chance(0.5) { Weather::AshRain } else { Weather::GraveMist };
            let left = gen_range(120.0, 180.0);
            self.weather = Some((w, left));
            self.next_weather = t + left + gen_range(240.0, 360.0);
            self.banner(&w.name().to_uppercase(), w.desc(), Color::from_rgba(200, 190, 170, 255), 3.5);
        }

        // The rival comes for whoever has gone long without a wound.
        let rival_alive = self.enemies.iter().any(|e| e.kind == EnemyKind::Rival && !e.dead);
        if !rival_alive && t >= self.next_rival && t > 180.0 {
            if let Some(h) = self.heroes.iter().position(|h| h.alive() && t - h.last_hurt.max(0.0) >= RIVAL_AFTER) {
                self.next_rival = t + 240.0;
                let at = self.heroes[h].p.pos;
                let mut e = self.make_enemy(EnemyKind::Rival, clamp_world(at + rand_dir() * 460.0, 80.0));
                e.hp = 900.0 * (1.0 + t / 300.0).powf(1.5) * self.diff.hp;
                e.max_hp = e.hp;
                e.dmg *= 1.0 + t / 600.0;
                e.hunt = Some(h);
                e.life_left = RIVAL_STAY;
                self.enemies.push(e);
                let name = self.heroes[h].name.clone();
                self.banner("A RIVAL HUNTS YOU", &format!("Untouched too long, {} draws a hunter. It carries many souls.", name), Color::from_rgba(230, 80, 70, 255), 4.0);
                self.sfx("bell", 0.8);
            }
        }
    }

    fn update_altars(&mut self, dt: f32) {
        let t = self.time;
        if t >= self.next_altar {
            self.next_altar = t + gen_range(240.0, 320.0);
            let (at, _, _) = self.random_hero_pos();
            let pos = clamp_world(at + rand_dir() * gen_range(340.0, 480.0), 160.0);
            self.altars.push(Altar { pos, life: ALTAR_STAY, kindle: 0.0 });
            self.toast("A blood altar rises. Stand on it to give a quarter of your life for might.");
        }
        let mut pacts = Vec::new();
        for a in self.altars.iter_mut() {
            a.life -= dt;
            let on = self.heroes.iter().position(|h| h.alive() && !h.pact && h.p.pos.distance(a.pos) < 34.0);
            match on {
                Some(h) => {
                    a.kindle += dt;
                    if a.kindle >= ALTAR_KINDLE {
                        pacts.push((h, a.pos));
                        a.life = 0.0;
                    }
                }
                None => a.kindle = (a.kindle - dt * 2.0).max(0.0),
            }
        }
        self.altars.retain(|a| a.life > 0.0);
        for (h, pos) in pacts {
            let hero = &mut self.heroes[h];
            hero.pact = true;
            hero.p.hp = hero.p.hp.min(hero.stats.max_hp * PACT_KEEP);
            let name = hero.name.clone();
            self.banner("BLOOD PACT", &format!("{} gives a quarter of their life: +35% might until the next lord falls", name), Color::from_rgba(200, 20, 30, 255), 4.0);
            self.blood_spray(pos, rand_dir(), Color::from_rgba(150, 10, 12, 255), 18.0, 24);
            self.fx.push(Fx { pos, life: 0.8, max_life: 0.8, kind: FxKind::Ring { r: 140.0, color: Color::from_rgba(200, 20, 30, 255) } });
            self.sfx_at("roar", 0.7, pos);
        }
    }

    fn update_wagers(&mut self, dt: f32) {
        let t = self.time;
        // The stone rises before each lord.
        let due = self.next_boss - t;
        if self.stone.is_none() && self.boss.is_none() && due > 0.5 && due <= 25.0 && self.stone_made != self.bosses_spawned {
            self.stone_made = self.bosses_spawned;
            let pos = clamp_world(self.center() + rand_dir() * gen_range(180.0, 260.0), 200.0);
            self.stone = Some(WagerStone { pos });
            self.toast("A wager stone rises. Bet a third of your souls on how fast the lord falls.");
        }
        if self.boss.is_some() && t - self.boss_started > WAGER_LINGER {
            self.stone = None;
        }
        // The clock of every bet starts with the lord.
        if self.boss.is_some() {
            let started = self.boss_started;
            for h in self.heroes.iter_mut() {
                if let Some(w) = h.wager.as_mut().filter(|w| w.since.is_none()) {
                    w.since = Some(started.max(t - dt));
                }
            }
        }
        let Some(stone) = &self.stone else { return };
        let tiles = [stone.tile(0), stone.tile(1), stone.tile(2)];
        let mut placed = Vec::new();
        for (h, hero) in self.heroes.iter_mut().enumerate() {
            let on = tiles.iter().position(|q| hero.p.pos.distance(*q) < 26.0);
            match on.filter(|_| hero.alive() && hero.wager.is_none() && hero.p.souls >= 3) {
                Some(i) if hero.wager_tile == i => {
                    hero.wager_kindle += dt;
                    if hero.wager_kindle >= WAGER_KINDLE {
                        let stake = hero.p.souls / 3;
                        hero.p.souls -= stake;
                        let since = self.boss.is_some().then_some(t);
                        hero.wager = Some(Wager { tier: i, stake, since });
                        hero.wager_kindle = 0.0;
                        placed.push((h, i, stake));
                    }
                }
                Some(i) => {
                    hero.wager_tile = i;
                    hero.wager_kindle = 0.0;
                }
                None => hero.wager_kindle = 0.0,
            }
        }
        for (h, i, stake) in placed {
            let (limit, mult) = WAGERS[i];
            let who = if self.multiplayer() { format!("{} bets", self.heroes[h].name) } else { "You bet".to_string() };
            self.toast(&format!("{} {} souls: the lord falls within {:.0} s, for x{}", who, stake, limit, mult));
            self.sfx_at("chime", 0.8, tiles[i]);
        }
    }

    /// A lord fell: every bet on it is paid or lost.
    pub(crate) fn settle_wagers(&mut self) {
        let t = self.time;
        let mut notes = Vec::new();
        for h in self.heroes.iter_mut() {
            let Some(w) = h.wager.take() else { continue };
            let won = w.left(t).is_some_and(|left| left >= 0.0);
            let who = h.name.clone();
            if won {
                let pay = (w.stake as f64 * w.mult() as f64).round() as u64;
                h.p.souls = h.p.souls.saturating_add(pay);
                notes.push(format!("{} wins the wager: {} souls", who, pay));
            } else {
                notes.push(format!("{} loses the wager of {} souls", who, w.stake));
            }
        }
        for n in notes {
            self.toast(&n);
        }
    }

    /// A lord fell: every blood pact is fulfilled and the life given returns.
    pub(crate) fn fulfil_pacts(&mut self) {
        let mut any = false;
        for h in self.heroes.iter_mut().filter(|h| h.pact) {
            h.pact = false;
            any = true;
        }
        if any {
            self.toast("The blood pact is fulfilled. Your life is your own again.");
        }
    }

    /// A cursed coffer was opened: rich loot, and a wave of elites.
    pub(crate) fn open_cursed_coffer(&mut self, h: usize, pos: Vec2) {
        self.open_coffer(h, true);
        let kinds = [EnemyKind::Knight, EnemyKind::Wraith, EnemyKind::Gargoyle, EnemyKind::Bloat, EnemyKind::Hollow];
        let n = 4 + (self.time / 300.0) as usize;
        for k in 0..n.min(9) {
            let at = clamp_world(pos + Vec2::from_angle(k as f32 / n as f32 * TAU) * 260.0, 60.0);
            let kind = kinds[k % kinds.len()];
            let mut e = self.make_enemy(kind, at);
            e.elite = true;
            e.hp *= 10.0;
            e.max_hp = e.hp;
            e.r = kind.radius(true);
            e.dmg *= 1.5;
            e.xp *= 10.0;
            self.enemies.push(e);
        }
        self.banner("THE COFFER WAS CURSED", "Its guardians rise around you", Color::from_rgba(170, 80, 220, 255), 3.0);
        self.sfx_at("roar", 0.8, pos);
    }
}
