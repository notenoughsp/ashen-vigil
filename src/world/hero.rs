//! Heroes: stats, movement, level-up choices, evolutions and fusions.

use super::*;

pub struct Player {
    pub pos: Vec2,
    pub facing: Vec2,
    pub moving: bool,
    pub hp: f32,
    pub stamina: f32,
    pub stamina_delay: f32,
    pub roll_t: f32,
    pub roll_dir: Vec2,
    pub iframes: f32,
    pub souls: u64,
    pub revives: u32,
    pub walk: f32,
    /// How the hero moves on ice, where they turn only slowly.
    pub slide: Vec2,
}

/// A weapon slot. A fused weapon carries its other weapons in `fused`: one for a fusion,
/// three for a mega fusion, seven for an ultra fusion. Every part keeps its own timer and spin.
/// Once every weapon a hero can get has gone into one slot, that slot is `whole`.
#[derive(Clone)]
pub struct Weapon {
    pub kind: WeaponKind,
    pub level: u32,
    pub evolved: bool,
    pub whole: bool,
    pub fused: Vec<WeaponKind>,
    pub timers: Vec<f32>,
    pub spins: Vec<f32>,
}

/// What each step of the soul burden gives.
pub const BURDEN_MIGHT: f32 = 0.05;
pub const BURDEN_AREA: f32 = 0.03;
pub const BURDEN_SPEED: f32 = 0.02;
pub const BURDEN_MAGNET: f32 = 0.06;
pub const BURDEN_COOLDOWN: f32 = 0.01;
/// The burden has at most this many steps.
pub const BURDEN_MAX: u32 = 10;

/// The steps of soul burden for `souls` carried: the first at 1,000 souls, and one more each
/// time the purse doubles, up to `BURDEN_MAX` (512,000 souls).
pub fn soul_burden(souls: u64) -> u32 {
    if souls < 1_000 {
        return 0;
    }
    ((souls / 1_000).ilog2() + 1).min(BURDEN_MAX)
}

/// Most health per second that lifesteal can return.
pub const LEECH_RATE: f32 = 4.0;

/// One firing part of a weapon slot; `slot` indexes its timer and spin.
#[derive(Clone, Copy)]
pub struct Part {
    /// How it fights: the weapon itself, or for a newer field's weapon its twin.
    pub kind: WeaponKind,
    /// Which weapon it is, for the damage meter and the element.
    pub own: WeaponKind,
    pub evo: bool,
    pub mult: f32,
    pub slot: usize,
}

impl Weapon {
    pub fn new(kind: WeaponKind) -> Weapon {
        Weapon::with_parts(kind, Vec::new())
    }

    pub fn with_parts(kind: WeaponKind, fused: Vec<WeaponKind>) -> Weapon {
        let n = fused.len() + 1;
        // Staggered first shots, so the parts of a fusion don't all fire on the same frame.
        let timers = (0..n).map(|i| 0.3 + i as f32 * 0.3).collect();
        Weapon { kind, level: 1, evolved: false, whole: false, fused, timers, spins: vec![0.0; n] }
    }

    /// 0 plain, 1 fusion, 2 mega fusion, 3 ultra fusion, 4 the last flame.
    pub fn tier(&self) -> u8 {
        if self.whole {
            return 4;
        }
        match self.fused.len() {
            0 => 0,
            1 | 2 => 1,
            3..=6 => 2,
            _ => 3,
        }
    }

    pub fn is_fused(&self) -> bool {
        !self.fused.is_empty()
    }

    /// How many weapons fire from this slot.
    pub fn kind_count(&self) -> usize {
        self.fused.len() + 1
    }

    /// Every weapon kind in this slot, its own first.
    pub fn kinds(&self) -> Vec<WeaponKind> {
        std::iter::once(self.kind).chain(self.fused.iter().copied()).collect()
    }

    pub fn parts(&self) -> Vec<Part> {
        let mult = [1.0, FUSION_MULT, MEGA_MULT, ULTRA_MULT, LAST_MULT][self.tier() as usize];
        let evo = self.evolved || self.is_fused();
        self.kinds().into_iter().enumerate().map(|(slot, own)| Part { kind: own.twin(), own, evo, mult, slot }).collect()
    }

    pub fn has(&self, k: WeaponKind) -> bool {
        self.kind == k || self.fused.contains(&k)
    }

    /// Holds a weapon that fights like `k`: `k` itself or a newer field's twin of it.
    pub fn has_like(&self, k: WeaponKind) -> bool {
        self.kind.twin() == k || self.fused.iter().any(|f| f.twin() == k)
    }

    pub fn display_name(&self) -> &'static str {
        match self.tier() {
            0 => self.kind.name(self.evolved),
            1 => fusion_name(self.kind, self.fused[0]),
            2 => mega_name(&self.kinds()),
            3 => ultra_name(&self.kinds()),
            _ if self.kind.is_cathedral() => LAST_HYMN,
            _ if self.kind.field() == 2 => LAST_GOD,
            _ if self.kind.field() == 3 => LAST_GROVE,
            _ if self.kind.field() == 4 => LAST_WINTER,
            _ if self.kind.field() == 5 => LAST_BLOOM,
            _ => LAST_FLAME,
        }
    }

    pub fn maxed(&self) -> bool {
        self.is_fused() || self.evolved || self.level >= WEAPON_MAX_LEVEL
    }
}

pub struct Item {
    pub kind: ItemKind,
    pub level: u32,
}

#[derive(Clone, Copy, Default, Debug)]
pub struct Input {
    pub dir: Vec2,
    pub roll: bool,
    pub skill: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Choice {
    NewWeapon(WeaponKind),
    UpWeapon(usize),
    NewItem(ItemKind),
    UpItem(usize),
    Draught,
    Souls,
}

impl Choice {
    pub fn encode(self) -> (u8, u8) {
        match self {
            Choice::NewWeapon(k) => (0, k.idx() as u8),
            Choice::UpWeapon(i) => (1, i as u8),
            Choice::NewItem(k) => (2, k.idx() as u8),
            Choice::UpItem(i) => (3, i as u8),
            Choice::Draught => (4, 0),
            Choice::Souls => (5, 0),
        }
    }

    pub fn decode(t: u8, a: u8) -> Choice {
        match t {
            0 => Choice::NewWeapon(WeaponKind::from_idx(a)),
            1 => Choice::UpWeapon(a as usize),
            2 => Choice::NewItem(ItemKind::from_idx(a)),
            3 => Choice::UpItem(a as usize),
            4 => Choice::Draught,
            _ => Choice::Souls,
        }
    }
}

/// One player character with everything it owns.
pub struct Hero {
    pub slot: u8,
    pub name: String,
    pub class: Class,
    pub profile: Profile,
    pub p: Player,
    pub stats: Stats,
    pub weapons: Vec<Weapon>,
    pub items: Vec<Item>,
    pub rerolls: u32,
    pub choices: Vec<Choice>,
    pub picked: bool,
    pub reroll_seq: u8,
    pub banked: u64,
    pub downed: bool,
    pub revive: f32,
    /// Where a fallen hero's ghost floats, chained to the body at `p.pos`.
    pub ghost: Vec2,
    /// This hero's player holds a lord (the crown's secret): the hero is off the field.
    pub possessing: bool,
    /// The last ping count heard from this hero's client.
    pub ping_seq: u8,
    pub input: Input,
    /// Movement comes from a network client instead of `input`.
    pub remote: bool,
    pub connected: bool,
    pub rolls: u32,
    /// Remaining seconds per `Buff`.
    pub buffs: [f32; 3],
    pub dmg_total: f64,
    pub dmg_weapon: [f64; ALL_WEAPONS.len()],
    /// Damage per second of the last minute, as (second, amount) ring.
    dmg_ring: [(i64, f32); 60],
    pub dmg_minute: f32,
    /// The Grave Cat follows this hero and brings luck.
    pub has_cat: bool,
    /// Wares bought from merchants this vigil.
    pub purchases: u32,
    /// Health that kills can still give back right now; refills over time, so lifesteal
    /// cannot outheal everything once the hordes get thick.
    pub leech: f32,
    /// Seconds without moving, and whether this hero already rested by a fire this vigil.
    pub idle: f32,
    /// Vigil time of the first hit taken; MAX while untouched.
    pub first_hurt: f32,
    /// When this hero was last wounded, for the rival.
    pub last_hurt: f32,
    /// A blood pact: a quarter of the life given for more might until the next lord falls.
    pub pact: bool,
    /// A bet on the lord at hand, from the wager stone.
    pub wager: Option<Wager>,
    /// When the next ember may rekindle this hero (`EMBER_COOLDOWN` after the last).
    pub ember_ready_at: f32,
    /// Which tile of the wager stone the hero stands on, and for how long.
    pub wager_tile: usize,
    pub wager_kindle: f32,
    /// The weight of the souls carried: each step makes the hero stronger, and banking them
    /// at a shrine (or losing them) gives it up. See `soul_burden`.
    pub burden: u32,
    /// Whose arsenal the hero fights with: 0 the graveyard's, 1 the cathedral's, 2 the
    /// flesh pits'.
    pub field: usize,
    /// The player has the pause menu (or the options) open.
    pub menu_open: bool,
    pub rested: bool,
    /// Challenge bits (single player).
    pub challenges: u8,
    /// Where the weapons point this frame: at the nearest foe, or where the hero walks.
    pub aim: Vec2,
    /// The look worn, see `SKINS`; 0 is the class look.
    pub skin: u8,
    /// Seconds until the class skill can be used again.
    pub skill_cd: f32,
    /// Counts the client's skill presses, like `reroll_seq`.
    pub skill_seq: u8,
    /// A skill was asked for and is cast in the next update.
    pub skill_wanted: bool,
}

impl Hero {
    pub fn new(slot: u8, name: &str, class: Class, profile: Profile) -> Hero {
        let mut h = Hero {
            slot,
            name: name.to_string(),
            class,
            profile,
            p: Player {
                pos: vec2(slot as f32 * 40.0 - 60.0, 0.0),
                facing: vec2(1.0, 0.0),
                moving: false,
                hp: 100.0,
                stamina: 100.0,
                stamina_delay: 0.0,
                roll_t: 0.0,
                roll_dir: vec2(1.0, 0.0),
                iframes: 0.0,
                souls: 0,
                revives: profile.meta[7] + profile.relics[5],
                walk: 0.0,
                slide: Vec2::ZERO,
            },
            stats: Stats::from_array([1.0; 17]),
            weapons: vec![Weapon { level: 1 + profile.m(M_KINDLING), ..Weapon::new(class.weapon()) }],
            items: Vec::new(),
            rerolls: profile.meta[8] + profile.relics[2],
            choices: Vec::new(),
            picked: true,
            reroll_seq: 0,
            banked: 0,
            downed: false,
            ghost: Vec2::ZERO,
            possessing: false,
            ping_seq: 0,
            revive: 0.0,
            input: Input::default(),
            remote: false,
            connected: true,
            rolls: 0,
            buffs: [0.0; 3],
            dmg_total: 0.0,
            dmg_weapon: [0.0; ALL_WEAPONS.len()],
            dmg_ring: [(-100, 0.0); 60],
            dmg_minute: 0.0,
            has_cat: false,
            purchases: 0,
            leech: LEECH_RATE,
            idle: 0.0,
            first_hurt: f32::MAX,
            last_hurt: 0.0,
            pact: false,
            wager: None,
            ember_ready_at: 0.0,
            wager_tile: 0,
            wager_kindle: 0.0,
            burden: 0,
            field: 0,
            menu_open: false,
            rested: false,
            challenges: 0,
            aim: vec2(1.0, 0.0),
            skin: profile.skin,
            skill_cd: 0.0,
            skill_seq: 0,
            skill_wanted: false,
        };
        h.recompute_stats();
        h.p.hp = h.stats.max_hp;
        h.p.stamina = h.stats.stamina_max;
        h
    }

    pub fn alive(&self) -> bool {
        !self.downed && !self.possessing
    }

    pub fn buff(&self, b: Buff) -> f32 {
        self.buffs[b as usize]
    }

    pub fn roll_cost(&self) -> f32 {
        let base = if self.class == Class::BucketKnight { 15.0 } else { 30.0 };
        (base - 3.0 * self.profile.m(M_FLEETFOOT) as f32).max(6.0)
    }

    /// Weapon slots, more with the Arsenal upgrade.
    pub fn max_weapons(&self) -> usize {
        MAX_WEAPONS + self.profile.m(M_ARSENAL) as usize
    }

    /// Item slots, more with the Satchel upgrade.
    pub fn max_items(&self) -> usize {
        MAX_ITEMS + self.profile.m(M_SATCHEL) as usize
    }

    /// Share of health a fallen hero rises with.
    pub fn rise_health(&self, base: f32) -> f32 {
        self.stats.max_hp * (base + 0.1 * self.profile.m(M_SECOND_WIND) as f32).min(1.0)
    }

    /// Extra damage from the Whetstone and Fusion Lore upgrades for one weapon slot.
    pub fn weapon_bonus(&self, w: &Weapon) -> f32 {
        let mut k = 1.0;
        if w.level >= WEAPON_MAX_LEVEL || w.evolved {
            k += 0.08 * self.profile.m(M_WHETSTONE) as f32;
        }
        if w.is_fused() {
            k += 0.06 * self.profile.m(M_FUSION_LORE) as f32;
        }
        k
    }

    /// The weapons this hero's field offers.
    pub fn arsenal(&self) -> [WeaponKind; ARSENAL] {
        match self.field {
            1 => CATHEDRAL_WEAPONS,
            2 => FLESH_WEAPONS,
            3 => ASH_WEAPONS,
            4 => ABBEY_WEAPONS,
            5 => GARDEN_WEAPONS,
            _ => GRAVE_WEAPONS,
        }
    }

    /// Takes up the arsenal of a field: its weapons trade places with their mirrors.
    pub fn arm_for(&mut self, field: usize) {
        self.field = field;
        for w in self.weapons.iter_mut() {
            let kind = w.kind.for_field(field);
            if kind != w.kind {
                let fused = w.fused.iter().map(|k| k.for_field(field)).collect();
                let (level, evolved, whole) = (w.level, w.evolved, w.whole);
                *w = Weapon::with_parts(kind, fused);
                w.level = level;
                w.evolved = evolved;
                w.whole = whole;
            }
        }
    }

    /// Whether a weapon can come in a level-up: from this field, and unlocked (the
    /// cathedral's need no unlocking).
    fn offers(&self, k: WeaponKind) -> bool {
        k.field() == self.field && (k.field() > 0 || self.profile.has(Unlock::Weapon(k)))
    }

    /// A new weapon can still come from a level-up.
    fn can_gain_weapon(&self) -> bool {
        self.challenges & Challenge::OneWeapon.bit() == 0 && self.weapons.len() < self.max_weapons() && self.arsenal().iter().any(|k| !self.owns_weapon(*k) && self.offers(*k))
    }

    /// Might including temporary blessings.
    pub fn might(&self) -> f32 {
        self.stats.might * if self.buff(Buff::Fury) > 0.0 { 1.5 } else { 1.0 } * if self.pact { PACT_MIGHT } else { 1.0 }
    }

    pub fn cooldown(&self) -> f32 {
        self.stats.cooldown * if self.buff(Buff::Haste) > 0.0 { 0.6 } else { 1.0 }
    }

    pub fn record_damage(&mut self, weapon: WeaponKind, amount: f32, time: f32) {
        self.dmg_total += amount as f64;
        self.dmg_weapon[weapon.idx()] += amount as f64;
        let sec = time.floor() as i64;
        let slot = &mut self.dmg_ring[sec.rem_euclid(60) as usize];
        if slot.0 != sec {
            *slot = (sec, 0.0);
        }
        slot.1 += amount;
    }

    pub fn refresh_minute(&mut self, time: f32) {
        let now = time.floor() as i64;
        self.dmg_minute = self.dmg_ring.iter().filter(|(s, _)| now - s < 60).map(|(_, a)| a).sum();
    }

    /// Keeps a fallen hero's ghost on its chain. The chain shortens as the hero is revived,
    /// so the ghost is drawn back into its body.
    pub fn leash_ghost(&mut self, revive_time: f32) {
        let k = (self.revive / revive_time).clamp(0.0, 1.0);
        let reach = GHOST_LEASH * (1.0 - k);
        let d = self.ghost - self.p.pos;
        if d.length() > reach {
            self.ghost = self.p.pos + d.normalize_or_zero() * reach;
        }
        self.ghost = clamp_world(self.ghost, 24.0);
    }

    pub fn heal(&mut self, amount: f32) {
        if !self.downed {
            self.p.hp = (self.p.hp + amount).min(self.stats.max_hp);
        }
    }

    pub fn recompute_stats(&mut self) {
        let m = self.profile.meta;
        let r = self.profile.relics;
        let ml = |i: usize| m[i] as f32;
        let rl = |i: usize| r[i] as f32;
        let mut might = 1.0 + 0.06 * ml(2) + 0.05 * rl(0);
        let mut cooldown = 1.0 - 0.04 * ml(4) - 0.03 * rl(7);
        let mut area = 1.0 + 0.06 * ml(11);
        let mut amount = m[16];
        let mut speed_mul = 1.0 + 0.04 * ml(3) + 0.05 * rl(10);
        let mut magnet_mul = 1.0 + 0.2 * rl(4) + 0.12 * ml(13);
        let mut luck = 1.0 + 0.1 * ml(5) + 0.15 * rl(11);
        let mut armor = ml(9) + rl(8);
        let mut regen = 0.15 * ml(10);
        let mut max_hp = 100.0 * (1.0 + 0.1 * ml(0)) + 15.0 * rl(1) + 25.0 * ml(M_COLOSSUS);
        let mut stamina_max = 100.0 + 15.0 * ml(1);
        let mut stamina_regen_mul = 1.0 + 0.1 * ml(1);
        let greed = 1.0 + 0.12 * ml(6) + 0.05 * rl(6);
        let mut crit = 0.05 + 0.02 * ml(12);
        let mut lifesteal = 0.15 * ml(M_BLOODTHIRST);
        let mut growth = 1.0 + 0.05 * ml(14) + 0.05 * rl(9);
        let mut elite_dmg = 1.0 + 0.08 * ml(15);
        match self.class {
            Class::Wanderer => max_hp += 10.0,
            Class::GraveHunter => {
                speed_mul += 0.1;
                max_hp += 10.0;
            }
            Class::CinderAdept => area += 0.1,
            Class::BellAcolyte => regen += 0.4,
            Class::Reaper => lifesteal += 0.5,
            Class::Hexblade => {
                stamina_max += 25.0;
                armor += 1.0;
            }
            Class::BloodKnight => max_hp += 20.0,
            Class::Arbalist => {
                cooldown -= 0.1;
                max_hp += 10.0;
            }
            Class::BucketKnight => armor += 2.0,
            Class::LanternWarden => {}
            Class::Stormcaller => crit += 0.1,
        }
        for it in &self.items {
            let l = it.level as f32;
            match it.kind {
                ItemKind::EmberHeart => might += 0.1 * l,
                ItemKind::Hourglass => cooldown -= 0.08 * l,
                ItemKind::Boots => speed_mul += 0.1 * l,
                ItemKind::Lens => area += 0.1 * l,
                ItemKind::Sigil => amount += it.level,
                ItemKind::Censer => magnet_mul += 0.35 * l,
                ItemKind::Shell => armor += l,
                ItemKind::Feather => {
                    luck += 0.12 * l;
                    crit += 0.03 * l;
                }
                ItemKind::Moss => regen += 0.3 * l,
                ItemKind::Lung => {
                    stamina_max += 20.0 * l;
                    stamina_regen_mul += 0.15 * l;
                }
                ItemKind::LeechFang => lifesteal += 0.4 * l,
                ItemKind::CrackedCrown => growth += 0.1 * l,
                ItemKind::GraveCandle => elite_dmg += 0.15 * l,
            }
        }
        if self.weapons.iter().any(|w| w.has_like(WeaponKind::Chime) && (w.evolved || w.is_fused())) {
            regen += 1.0;
        }
        if self.has_cat {
            luck += 0.13;
        }
        // The souls carried: might, area, pace, reach and quicker weapons.
        let b = self.burden as f32;
        might += BURDEN_MIGHT * b;
        area += BURDEN_AREA * b;
        speed_mul += BURDEN_SPEED * b;
        magnet_mul += BURDEN_MAGNET * b;
        cooldown -= BURDEN_COOLDOWN * b;
        if self.challenges & Challenge::GlassCannon.bit() != 0 {
            max_hp *= 0.5;
            might += 0.6;
        }
        let old_max = self.stats.max_hp;
        self.stats = Stats {
            might,
            cooldown: cooldown.max(0.4),
            area,
            amount,
            speed: 150.0 * speed_mul,
            magnet: 70.0 * magnet_mul,
            luck,
            armor,
            regen,
            max_hp,
            stamina_max,
            stamina_regen: 35.0 * stamina_regen_mul,
            greed,
            crit: crit + (luck - 1.0) * 0.1,
            lifesteal,
            growth,
            elite_dmg,
        };
        if max_hp > old_max && old_max > 1.5 {
            self.p.hp += max_hp - old_max;
        }
    }

    /// Moves the hero from its input. Shared by the host and client-side prediction.
    /// Returns true when a roll started this frame.
    pub fn step_movement(&mut self, dt: f32, arena: Option<(Vec2, f32)>) -> bool {
        let st = self.stats;
        let cost = self.roll_cost();
        let input = self.input;
        let pl = &mut self.p;
        pl.moving = input.dir.length_squared() > 0.0;
        if pl.moving {
            pl.facing = input.dir.normalize();
            pl.walk += dt * 10.0;
        }
        let mut rolled = false;
        let rooted = self.challenges & Challenge::NoRolls.bit() != 0;
        if input.roll && !rooted && pl.roll_t <= 0.0 && pl.stamina >= cost {
            pl.stamina -= cost;
            pl.stamina_delay = 0.6;
            pl.roll_t = 0.3;
            pl.roll_dir = if pl.moving { pl.facing } else { -pl.facing };
            rolled = true;
        }
        if pl.roll_t > 0.0 {
            pl.roll_t -= dt;
            pl.pos += pl.roll_dir * st.speed * 2.8 * dt;
            pl.slide = pl.roll_dir * st.speed;
        } else {
            let want = if pl.moving { pl.facing * st.speed } else { Vec2::ZERO };
            pl.slide = if ice_at(pl.pos) { pl.slide.lerp(want, (ICE_GRIP * dt).min(1.0)) } else { want };
            pl.pos += pl.slide * dt;
        }
        if pl.stamina_delay > 0.0 {
            pl.stamina_delay -= dt;
        } else {
            pl.stamina = (pl.stamina + st.stamina_regen * dt).min(st.stamina_max);
        }
        if let Some((c, r)) = arena {
            let d = pl.pos - c;
            if d.length() > r - 12.0 {
                pl.pos = c + d.normalize() * (r - 12.0);
            }
        }
        pl.pos = clamp_world(pl.pos, 24.0);
        if rolled {
            self.rolls += 1;
        }
        rolled
    }

    /// Where the censers swing: a figure of eight about the hero, each a little behind the
    /// last, and how big each one hits.
    pub fn censer_positions(&self) -> Vec<(Vec2, f32)> {
        let mut out = Vec::new();
        for w in &self.weapons {
            for part in w.parts().into_iter().filter(|p| p.kind == WeaponKind::Censer) {
                let spin = w.spins.get(part.slot).copied().unwrap_or(0.0);
                let s = WeaponKind::Censer.stats(w.level, part.evo);
                let n = (s.amount + self.stats.amount).div_ceil(2).max(1) + u32::from(self.class == Class::LanternWarden);
                let area = s.area * self.stats.area;
                let r = 130.0 * area;
                for k in 0..n {
                    let t = spin + k as f32 / n as f32 * TAU;
                    let d = 1.0 + t.sin() * t.sin();
                    let at = vec2(r * t.cos() / d, r * t.sin() * t.cos() / d * 1.4);
                    out.push((self.p.pos + at, 18.0 * area));
                }
            }
        }
        out
    }

    /// Where the watching eyes circle: one ring about the hero, and how big each one is.
    pub fn eye_positions(&self) -> Vec<(Vec2, f32)> {
        let mut out = Vec::new();
        for w in &self.weapons {
            for part in w.parts().into_iter().filter(|p| p.kind == WeaponKind::Eyes) {
                let spin = w.spins.get(part.slot).copied().unwrap_or(0.0);
                let s = WeaponKind::Eyes.stats(w.level, part.evo);
                let n = (s.amount + self.stats.amount).div_ceil(2).max(2) + u32::from(self.class == Class::LanternWarden);
                let area = s.area * self.stats.area;
                for k in 0..n {
                    let a = spin * 0.6 + k as f32 / n as f32 * TAU;
                    out.push((self.p.pos + Vec2::from_angle(a) * 105.0 * area, 9.0 * area.sqrt()));
                }
            }
        }
        out
    }

    /// The heartbeat aura's reach for each heart carried, and how far into its beat it is
    /// (1 just after the first beat, falling to 0).
    pub fn heart_beats(&self) -> Vec<(f32, f32)> {
        let mut out = Vec::new();
        for w in &self.weapons {
            for part in w.parts().into_iter().filter(|p| p.kind == WeaponKind::Heart) {
                let s = WeaponKind::Heart.stats(w.level, part.evo);
                let timer = w.timers.get(part.slot).copied().unwrap_or(0.0);
                let cd = s.cd * self.stats.cooldown;
                out.push((80.0 * s.area * self.stats.area, (timer / cd.max(0.05)).clamp(0.0, 1.0)));
            }
        }
        out
    }

    /// How far the halo has breathed out, for each halo the hero carries.
    pub fn halo_radii(&self) -> Vec<f32> {
        let mut out = Vec::new();
        for w in &self.weapons {
            for part in w.parts().into_iter().filter(|p| p.kind == WeaponKind::Halo) {
                let spin = w.spins.get(part.slot).copied().unwrap_or(0.0);
                let s = WeaponKind::Halo.stats(w.level, part.evo);
                let max = 135.0 * s.area * self.stats.area;
                out.push(40.0 + (max - 40.0) * (0.5 - 0.5 * spin.cos()));
            }
        }
        out
    }

    pub fn lantern_positions(&self) -> Vec<(Vec2, f32)> {
        let mut out = Vec::new();
        for w in &self.weapons {
            for part in w.parts().into_iter().filter(|p| p.kind == WeaponKind::Lanterns) {
                let spin = w.spins.get(part.slot).copied().unwrap_or(0.0);
                let s = WeaponKind::Lanterns.stats(w.level, part.evo);
                // The Lantern Warden keeps one more.
                let n = s.amount + self.stats.amount + u32::from(self.class == Class::LanternWarden);
                let area = s.area * self.stats.area;
                let (inner, outer) = if part.evo { (n / 2, n - n / 2) } else { (n, 0) };
                for k in 0..inner {
                    let a = spin + k as f32 / inner as f32 * TAU;
                    out.push((self.p.pos + Vec2::from_angle(a) * 90.0 * area, 13.0 * area));
                }
                for k in 0..outer {
                    let a = -spin * 0.7 + k as f32 / outer as f32 * TAU;
                    out.push((self.p.pos + Vec2::from_angle(a) * 150.0 * area, 13.0 * area));
                }
            }
        }
        out
    }

    fn owns_weapon(&self, k: WeaponKind) -> bool {
        self.weapons.iter().any(|w| w.has(k))
    }

    pub(crate) fn pool(&self, include_new: bool) -> Vec<(Choice, f32)> {
        let mut pool = Vec::new();
        for (i, w) in self.weapons.iter().enumerate() {
            if !w.maxed() {
                pool.push((Choice::UpWeapon(i), 3.0));
            }
        }
        for (i, it) in self.items.iter().enumerate() {
            if it.level < it.kind.max_level() {
                pool.push((Choice::UpItem(i), 2.5));
            }
        }
        if include_new && self.challenges & Challenge::OneWeapon.bit() != 0 {
            // One Blade: no new weapons, items still come.
            if self.items.len() < self.max_items() {
                for k in ALL_ITEMS {
                    if !self.items.iter().any(|it| it.kind == k) && self.profile.has(Unlock::Item(k)) {
                        pool.push((Choice::NewItem(k), 1.4));
                    }
                }
            }
        } else if include_new {
            if self.weapons.len() < self.max_weapons() {
                for k in self.arsenal() {
                    if !self.owns_weapon(k) && self.offers(k) {
                        pool.push((Choice::NewWeapon(k), 1.6));
                    }
                }
            }
            if self.items.len() < self.max_items() {
                for k in ALL_ITEMS {
                    if !self.items.iter().any(|it| it.kind == k) && self.profile.has(Unlock::Item(k)) {
                        pool.push((Choice::NewItem(k), 1.4));
                    }
                }
            }
        }
        pool
    }

    /// Every weapon and item is as good as it gets; level-ups have nothing left to offer.
    pub fn maxed(&self) -> bool {
        self.pool(true).is_empty()
    }

    pub fn roll_choices(&self) -> Vec<Choice> {
        let mut pool = self.pool(true);
        let n = if chance((self.stats.luck - 1.0) * 0.8 + 0.1 + 0.08 * self.profile.m(M_FORESIGHT) as f32) { 4 } else { 3 };
        let mut out = Vec::new();
        while out.len() < n && !pool.is_empty() {
            let total: f32 = pool.iter().map(|p| p.1).sum();
            let mut r = gen_range(0.0, total);
            let mut pick = pool.len() - 1;
            for (k, p) in pool.iter().enumerate() {
                if r < p.1 {
                    pick = k;
                    break;
                }
                r -= p.1;
            }
            out.push(pool.swap_remove(pick).0);
        }
        if out.is_empty() {
            out.push(Choice::Draught);
            out.push(Choice::Souls);
        }
        out
    }

    pub fn apply_choice(&mut self, c: Choice) {
        match c {
            Choice::NewWeapon(k) => {
                if self.weapons.len() < self.max_weapons() && !self.owns_weapon(k) {
                    self.weapons.push(Weapon::new(k));
                }
            }
            Choice::UpWeapon(i) => {
                if let Some(w) = self.weapons.get_mut(i) {
                    if !w.maxed() {
                        w.level += 1;
                    }
                }
            }
            Choice::NewItem(k) => {
                if self.items.len() < self.max_items() && !self.items.iter().any(|it| it.kind == k) {
                    self.items.push(Item { kind: k, level: 1 });
                }
            }
            Choice::UpItem(i) => {
                if let Some(it) = self.items.get_mut(i) {
                    it.level = (it.level + 1).min(it.kind.max_level());
                }
            }
            Choice::Draught => self.heal(30.0 * self.draught_mult()),
            Choice::Souls => self.p.souls += 150,
        }
        self.recompute_stats();
    }

    /// Title, description, color and tag for a level-up card.
    pub fn choice_text(&self, c: Choice) -> (String, String, Color, String) {
        match c {
            Choice::NewWeapon(k) => (k.name(false).into(), k.desc().into(), k.color(), "NEW".into()),
            Choice::UpWeapon(i) => {
                let Some(w) = self.weapons.get(i) else { return ("?".into(), String::new(), WHITE, String::new()) };
                let mut d = w.kind.level_desc(w.level + 1);
                if w.level + 1 == WEAPON_MAX_LEVEL {
                    let partner = w.kind.fusion_partner();
                    d += &format!(". Max: fuses with {}", partner.name(false));
                }
                (w.kind.name(false).into(), d, w.kind.color(), format!("Lv {}", w.level + 1))
            }
            Choice::NewItem(k) => (k.name().into(), k.desc().into(), k.color(), "NEW".into()),
            Choice::UpItem(i) => {
                let Some(it) = self.items.get(i) else { return ("?".into(), String::new(), WHITE, String::new()) };
                (it.kind.name().into(), it.kind.desc().into(), it.kind.color(), format!("Lv {}", it.level + 1))
            }
            Choice::Draught => ("Ashen Draught".into(), "Restore 30 health".into(), Color::from_rgba(230, 140, 60, 255), "".into()),
            Choice::Souls => ("Soul Remnant".into(), "+150 souls".into(), Color::from_rgba(150, 220, 230, 255), "".into()),
        }
    }

    /// Whether a card is a weapon or a passive (or neither), and a line on what it does.
    pub fn choice_kind(&self, c: Choice) -> (&'static str, Option<&'static str>) {
        match c {
            Choice::NewWeapon(_) => ("WEAPON", None),
            Choice::UpWeapon(i) => ("WEAPON", self.weapons.get(i).map(|w| w.kind.desc())),
            Choice::NewItem(k) => ("PASSIVE", k.detail()),
            Choice::UpItem(i) => ("PASSIVE", self.items.get(i).and_then(|it| it.kind.detail())),
            _ => ("", None),
        }
    }

    /// Whether a card fuses with something this hero already carries. (Weapons no longer
    /// evolve on their own: every weapon ends in the one fused flame anyway.)
    pub fn choice_hint(&self, c: Choice) -> Option<(String, Color)> {
        let fuse = Color::from_rgba(255, 130, 100, 255);
        let k = match c {
            Choice::NewWeapon(k) => Some(k),
            Choice::UpWeapon(i) => self.weapons.get(i).filter(|w| !w.is_fused()).map(|w| w.kind),
            _ => None,
        }?;
        let partner = self.weapons.iter().find(|w| !w.is_fused() && w.kind != k && k.fusion_partners().contains(&w.kind));
        partner.map(|w| (format!("Fuses with {}", w.kind.name(false)), fuse))
    }

    /// Draughts heal more with the Alchemy upgrade.
    pub fn draught_mult(&self) -> f32 {
        1.0 + 0.25 * self.profile.m(M_ALCHEMY) as f32
    }

    /// Two slots that merge now: partner weapons at max level into a fusion, or two fusions
    /// of the same tier into the next (two fusions into a mega fusion, two megas into an ultra).
    fn fusable(&self) -> Option<(usize, usize)> {
        for (i, a) in self.weapons.iter().enumerate() {
            if a.is_fused() || a.level < WEAPON_MAX_LEVEL {
                continue;
            }
            for partner in a.kind.fusion_partners() {
                if let Some(j) = self.weapons.iter().position(|b| b.kind == partner && !b.is_fused() && b.level >= WEAPON_MAX_LEVEL) {
                    if i != j {
                        return Some((i, j));
                    }
                }
            }
        }
        for tier in 1..=2 {
            let same: Vec<usize> = (0..self.weapons.len()).filter(|&i| self.weapons[i].tier() == tier).collect();
            if same.len() >= 2 {
                return Some((same[0], same[1]));
            }
        }
        // Nothing left to level and no new weapon can come: the two smallest slots merge,
        // until in the end every weapon has become one.
        if self.weapons.len() >= 2 && self.weapons.iter().all(|w| w.maxed()) && !self.can_gain_weapon() {
            let mut order: Vec<usize> = (0..self.weapons.len()).collect();
            order.sort_by_key(|&i| (self.weapons[i].fused.len(), i));
            return Some((order[0], order[1]));
        }
        None
    }

    /// The last slot holds everything this hero could ever carry.
    fn all_in_one(&self) -> bool {
        self.weapons.len() == 1 && self.weapons[0].is_fused() && !self.can_gain_weapon() && self.challenges & Challenge::OneWeapon.bit() == 0
    }
}

impl World {
    /// Fuses two max-level partner weapons into one, freeing a weapon slot.
    pub(crate) fn check_fusion(&mut self, h: usize) {
        while self.fuse_once(h) {}
    }

    fn fuse_once(&mut self, h: usize) -> bool {
        let Some((i, j)) = self.heroes[h].fusable() else { return false };
        let prefix = if self.multiplayer() { format!("{}: ", self.heroes[h].name) } else { String::new() };
        let hero = &mut self.heroes[h];
        let (a_name, b_name) = (hero.weapons[i].display_name(), hero.weapons[j].display_name());
        let b = hero.weapons.remove(j);
        let ai = if j < i { i - 1 } else { i };
        let a = &mut hero.weapons[ai];
        a.fused.extend(b.kinds());
        a.timers.extend(b.timers);
        a.spins.extend(b.spins);
        a.evolved = true;
        if hero.all_in_one() {
            hero.weapons[ai].whole = true;
        }
        let (tier, name) = (hero.weapons[ai].tier(), hero.weapons[ai].display_name());
        hero.recompute_stats();
        let (title, color) = match tier {
            1 => ("FUSION", Color::from_rgba(255, 120, 90, 255)),
            2 => ("MEGA FUSION", Color::from_rgba(220, 140, 255, 255)),
            3 => ("ULTRA FUSION", Color::from_rgba(255, 245, 200, 255)),
            _ => ("THE LAST FLAME", Color::from_rgba(255, 255, 235, 255)),
        };
        crate::log!("{}: {} and {} became {} ({})", self.heroes[h].name, a_name, b_name, name, title.to_lowercase());
        self.banner(title, &format!("{}{} and {} become {}", prefix, a_name, b_name, name), color, 4.5);
        self.sfx("evolve", 1.0);
        self.sfx("roar", 0.5);
        self.shake = self.shake.max(10.0);
        true
    }

    pub(crate) fn update_heroes(&mut self, dt: f32) {
        let arena = self.arena;
        let famine = self.cursed_by(Curse::Famine);
        let revive_time = if self.cursed_by(Curse::Brittle) { REVIVE_TIME * 2.0 } else { REVIVE_TIME };
        let mut skills = Vec::new();
        for h in 0..self.heroes.len() {
            let hero = &mut self.heroes[h];
            hero.p.iframes = (hero.p.iframes - dt).max(0.0);
            hero.skill_cd = (hero.skill_cd - dt).max(0.0);
            for b in hero.buffs.iter_mut() {
                *b = (*b - dt).max(0.0);
            }
            if hero.downed {
                // The ghost drifts where its player steers it, on its chain; the client
                // steers its own and says where it is.
                if !hero.remote {
                    hero.ghost += hero.input.dir * GHOST_SPEED * dt;
                }
                hero.leash_ghost(revive_time);
                continue;
            }
            if !famine {
                hero.p.hp = (hero.p.hp + hero.stats.regen * dt).min(hero.stats.max_hp);
            }
            hero.leech = (hero.leech + LEECH_RATE * dt).min(LEECH_RATE);
            if hero.remote {
                // Position and roll come from the client; keep them inside the fog ring.
                if let Some((c, r)) = arena {
                    let d = hero.p.pos - c;
                    if d.length() > r - 12.0 {
                        hero.p.pos = c + d.normalize() * (r - 12.0);
                    }
                }
                hero.p.pos = clamp_world(hero.p.pos, 24.0);
                continue;
            }
            let wants_skill = hero.input.skill;
            let rolled = hero.step_movement(dt, arena);
            if wants_skill {
                skills.push(h);
            }
            let (pos, rolling, rolls, bucket) = (hero.p.pos, hero.p.roll_t > 0.0, hero.rolls, hero.class == Class::BucketKnight);
            if rolled && h == self.local {
                self.sfx(if bucket { "clang" } else { "roll" }, if bucket { 0.4 } else { 0.5 });
                if rolls == 100 {
                    self.toast("100 rolls. Your knees file a formal complaint.");
                } else if rolls == 1000 {
                    self.toast("1,000 rolls. The floor knows you by name now.");
                }
            }
            if rolling && chance(0.6) {
                self.fx.push(Fx { pos, life: 0.25, max_life: 0.25, kind: FxKind::Ghost });
            }
        }
        for h in skills {
            self.use_skill(h);
        }
        // Reviving fallen companions by standing next to them.
        for h in 0..self.heroes.len() {
            if !self.heroes[h].downed || !self.heroes[h].connected {
                continue;
            }
            let p = self.heroes[h].p.pos;
            let helped = self.heroes.iter().any(|o| o.alive() && o.p.pos.distance(p) < 45.0);
            let hero = &mut self.heroes[h];
            hero.revive = if helped { hero.revive + dt } else { (hero.revive - dt * 0.5).max(0.0) };
            if hero.revive >= revive_time {
                hero.downed = false;
                hero.revive = 0.0;
                hero.p.hp = hero.rise_health(0.4);
                hero.p.iframes = 2.0;
                let (name, hero_pos) = (hero.name.clone(), hero.p.pos);
                self.toast(&format!("{} rises again.", name));
                self.sfx_at("kindle", 0.7, hero_pos);
            }
        }
    }
}
