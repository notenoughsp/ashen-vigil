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
}

/// A weapon slot. A fused weapon carries its other weapons in `fused`: one for a fusion,
/// three for a mega fusion, seven for an ultra fusion. Every part keeps its own timer and spin.
#[derive(Clone)]
pub struct Weapon {
    pub kind: WeaponKind,
    pub level: u32,
    pub evolved: bool,
    pub fused: Vec<WeaponKind>,
    pub timers: Vec<f32>,
    pub spins: Vec<f32>,
}

/// Most health per second that lifesteal can return.
pub const LEECH_RATE: f32 = 4.0;

/// One firing part of a weapon slot; `slot` indexes its timer and spin.
#[derive(Clone, Copy)]
pub struct Part {
    pub kind: WeaponKind,
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
        Weapon { kind, level: 1, evolved: false, fused, timers, spins: vec![0.0; n] }
    }

    /// 0 plain, 1 fusion, 2 mega fusion, 3 ultra fusion.
    pub fn tier(&self) -> u8 {
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

    /// Every weapon kind in this slot, its own first.
    pub fn kinds(&self) -> Vec<WeaponKind> {
        std::iter::once(self.kind).chain(self.fused.iter().copied()).collect()
    }

    pub fn parts(&self) -> Vec<Part> {
        let mult = [1.0, FUSION_MULT, MEGA_MULT, ULTRA_MULT][self.tier() as usize];
        let evo = self.evolved || self.is_fused();
        self.kinds().into_iter().enumerate().map(|(slot, kind)| Part { kind, evo, mult, slot }).collect()
    }

    pub fn has(&self, k: WeaponKind) -> bool {
        self.kind == k || self.fused.contains(&k)
    }

    pub fn display_name(&self) -> &'static str {
        match self.tier() {
            0 => self.kind.name(self.evolved),
            1 => fusion_name(self.kind, self.fused[0]),
            2 => mega_name(&self.kinds()),
            _ => ultra_name(&self.kinds()),
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
    pub input: Input,
    /// Movement comes from a network client instead of `input`.
    pub remote: bool,
    pub connected: bool,
    pub rolls: u32,
    /// Remaining seconds per `Buff`.
    pub buffs: [f32; 3],
    pub dmg_total: f64,
    pub dmg_weapon: [f64; 12],
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
    pub rested: bool,
    /// Challenge bits (single player).
    pub challenges: u8,
    /// Weapons aim at the nearest enemy instead of the walking direction.
    pub auto_aim: bool,
    /// Where the weapons point this frame.
    pub aim: Vec2,
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
                revives: profile.meta[7],
                walk: 0.0,
            },
            stats: Stats::from_array([1.0; 17]),
            weapons: vec![Weapon::new(class.weapon())],
            items: Vec::new(),
            rerolls: profile.meta[8] + profile.relics[2],
            choices: Vec::new(),
            picked: true,
            reroll_seq: 0,
            banked: 0,
            downed: false,
            revive: 0.0,
            input: Input::default(),
            remote: false,
            connected: true,
            rolls: 0,
            buffs: [0.0; 3],
            dmg_total: 0.0,
            dmg_weapon: [0.0; 12],
            dmg_ring: [(-100, 0.0); 60],
            dmg_minute: 0.0,
            has_cat: false,
            purchases: 0,
            leech: LEECH_RATE,
            idle: 0.0,
            rested: false,
            challenges: 0,
            auto_aim: false,
            aim: vec2(1.0, 0.0),
        };
        h.recompute_stats();
        h.p.hp = h.stats.max_hp;
        h.p.stamina = h.stats.stamina_max;
        h
    }

    pub fn alive(&self) -> bool {
        !self.downed
    }

    pub fn buff(&self, b: Buff) -> f32 {
        self.buffs[b as usize]
    }

    pub fn roll_cost(&self) -> f32 {
        if self.class == Class::BucketKnight {
            15.0
        } else {
            30.0
        }
    }

    /// Might including temporary blessings.
    pub fn might(&self) -> f32 {
        self.stats.might * if self.buff(Buff::Fury) > 0.0 { 1.5 } else { 1.0 }
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

    pub fn heal(&mut self, amount: f32) {
        if !self.downed {
            self.p.hp = (self.p.hp + amount).min(self.stats.max_hp);
        }
    }

    pub fn recompute_stats(&mut self) {
        let m = self.profile.meta;
        let r = self.profile.relics;
        let ml = |i: usize| m[i] as f32;
        let mut might = 1.0 + 0.06 * ml(2) + 0.05 * r[0] as f32;
        let mut cooldown = 1.0 - 0.04 * ml(4);
        let mut area = 1.0;
        let mut amount = 0;
        let mut speed_mul = 1.0 + 0.04 * ml(3);
        let mut magnet_mul = 1.0 + 0.2 * r[4] as f32;
        let mut luck = 1.0 + 0.1 * ml(5);
        let mut armor = 0.0;
        let mut regen = 0.0;
        let mut max_hp = 100.0 * (1.0 + 0.1 * ml(0)) + 15.0 * r[1] as f32;
        let mut stamina_max = 100.0 + 15.0 * ml(1);
        let mut stamina_regen_mul = 1.0 + 0.1 * ml(1);
        let greed = 1.0 + 0.12 * ml(6);
        let mut crit = 0.05;
        let mut lifesteal = 0.0;
        let mut growth = 1.0;
        let mut elite_dmg = 1.0;
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
        if self.weapons.iter().any(|w| w.has(WeaponKind::Chime) && (w.evolved || w.is_fused())) {
            regen += 1.0;
        }
        if self.has_cat {
            luck += 0.13;
        }
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
        } else if pl.moving {
            pl.pos += pl.facing * st.speed * dt;
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

    pub fn lantern_positions(&self) -> Vec<(Vec2, f32)> {
        let mut out = Vec::new();
        for w in &self.weapons {
            for part in w.parts().into_iter().filter(|p| p.kind == WeaponKind::Lanterns) {
                let spin = w.spins.get(part.slot).copied().unwrap_or(0.0);
                let s = WeaponKind::Lanterns.stats(w.level, part.evo);
                let n = s.amount + self.stats.amount;
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
            if self.items.len() < MAX_ITEMS {
                for k in ALL_ITEMS {
                    if !self.items.iter().any(|it| it.kind == k) && self.profile.has(Unlock::Item(k)) {
                        pool.push((Choice::NewItem(k), 1.4));
                    }
                }
            }
        } else if include_new {
            if self.weapons.len() < MAX_WEAPONS {
                for k in ALL_WEAPONS {
                    if !self.owns_weapon(k) && self.profile.has(Unlock::Weapon(k)) {
                        pool.push((Choice::NewWeapon(k), 1.6));
                    }
                }
            }
            if self.items.len() < MAX_ITEMS {
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
        let n = if chance((self.stats.luck - 1.0) * 0.8 + 0.1) { 4 } else { 3 };
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
                if self.weapons.len() < MAX_WEAPONS && !self.owns_weapon(k) {
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
                if self.items.len() < MAX_ITEMS && !self.items.iter().any(|it| it.kind == k) {
                    self.items.push(Item { kind: k, level: 1 });
                }
            }
            Choice::UpItem(i) => {
                if let Some(it) = self.items.get_mut(i) {
                    it.level = (it.level + 1).min(it.kind.max_level());
                }
            }
            Choice::Draught => self.heal(30.0),
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
                    d += &format!(". Max: evolves with {}, fuses with {}", w.kind.evo_item().name(), partner.name(false));
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

    pub fn evolvable(&self) -> Option<usize> {
        self.weapons.iter().position(|w| !w.is_fused() && !w.evolved && w.level >= WEAPON_MAX_LEVEL && self.items.iter().any(|it| it.kind == w.kind.evo_item()))
    }

    /// Two max-level fusion partners, if this hero has them.
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
        None
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
        let a = &mut hero.weapons[if j < i { i - 1 } else { i }];
        a.fused.extend(b.kinds());
        a.timers.extend(b.timers);
        a.spins.extend(b.spins);
        a.evolved = true;
        let (tier, name) = (a.tier(), a.display_name());
        hero.recompute_stats();
        let (title, color) = match tier {
            1 => ("FUSION", Color::from_rgba(255, 120, 90, 255)),
            2 => ("MEGA FUSION", Color::from_rgba(220, 140, 255, 255)),
            _ => ("ULTRA FUSION", Color::from_rgba(255, 245, 200, 255)),
        };
        self.banner(title, &format!("{}{} and {} become {}", prefix, a_name, b_name, name), color, 4.5);
        self.sfx("evolve", 1.0);
        self.sfx("roar", 0.5);
        self.shake = self.shake.max(10.0);
        true
    }

    pub(crate) fn update_heroes(&mut self, dt: f32) {
        let arena = self.arena;
        for h in 0..self.heroes.len() {
            let hero = &mut self.heroes[h];
            hero.p.iframes = (hero.p.iframes - dt).max(0.0);
            for b in hero.buffs.iter_mut() {
                *b = (*b - dt).max(0.0);
            }
            if hero.downed {
                continue;
            }
            hero.p.hp = (hero.p.hp + hero.stats.regen * dt).min(hero.stats.max_hp);
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
            let rolled = hero.step_movement(dt, arena);
            let (pos, rolling, rolls, bucket) = (hero.p.pos, hero.p.roll_t > 0.0, hero.rolls, hero.class == Class::BucketKnight);
            if rolled && h == self.local {
                self.sfx(if bucket { "clang" } else { "roll" }, if bucket { 0.4 } else { 0.5 });
                if rolls == 100 {
                    self.toast("100 rolls. Your knees file a formal complaint.");
                }
            }
            if rolling && chance(0.6) {
                self.fx.push(Fx { pos, life: 0.25, max_life: 0.25, kind: FxKind::Ghost });
            }
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
            if hero.revive >= REVIVE_TIME {
                hero.downed = false;
                hero.revive = 0.0;
                hero.p.hp = hero.stats.max_hp * 0.4;
                hero.p.iframes = 2.0;
                let (name, hero_pos) = (hero.name.clone(), hero.p.pos);
                self.toast(&format!("{} rises again.", name));
                self.sfx_at("kindle", 0.7, hero_pos);
            }
        }
    }
}
