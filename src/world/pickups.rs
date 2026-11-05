//! Souls, coffers, map relics, shrines and the remnant.

use super::*;

impl World {
    pub(crate) fn update_pickups(&mut self, dt: f32) {
        let heroes: Vec<(usize, Vec2, f32)> = self.heroes.iter().enumerate().filter(|(_, h)| h.alive()).map(|(i, h)| (i, h.p.pos, h.stats.magnet)).collect();
        let mut collected: Vec<(usize, usize)> = Vec::new();
        for (k, p) in self.pickups.iter_mut().enumerate() {
            p.bob += dt * 3.0;
            p.life -= dt;
            if p.pulled.map(|h| !heroes.iter().any(|x| x.0 == h)).unwrap_or(false) {
                p.pulled = None;
            }
            if p.pulled.is_none() {
                // Coffers and relics must be walked over; souls fly to the nearest hero.
                let reach = |mag: f32| if matches!(p.kind, PickKind::Soul(_) | PickKind::Draught | PickKind::Magnet) { mag } else { 30.0 };
                if let Some((h, _, _)) = heroes.iter().find(|(_, hp, mag)| p.pos.distance(*hp) < reach(*mag)) {
                    p.pulled = Some(*h);
                }
            }
            if let Some(h) = p.pulled {
                let hp = heroes.iter().find(|x| x.0 == h).map(|x| x.1).unwrap_or(p.pos);
                let d = p.pos.distance(hp);
                let speed = 380.0 + (100.0 - d).max(0.0) * 3.0;
                p.pos += (hp - p.pos).normalize_or_zero() * speed * dt;
                if p.pos.distance(hp) < 14.0 {
                    collected.push((k, h));
                }
            }
        }
        // Highest index first, so swap_remove never moves one we still need.
        collected.sort_by_key(|c| std::cmp::Reverse(c.0));
        let got: Vec<(PickKind, usize, Vec2)> = collected
            .into_iter()
            .map(|(k, h)| {
                let p = self.pickups.swap_remove(k);
                (p.kind, h, p.pos)
            })
            .collect();
        self.pickups.retain(|p| p.life > 0.0);

        let rich = if self.weather.is_some_and(|(w, _)| w == Weather::GraveMist) { 1.25 } else { 1.0 };
        let soul_mult = self.diff.soul_mult() * if self.blood_moon > 0.0 { 2.0 } else { 1.0 } * rich;
        let mut gained_xp = 0.0;
        let mut picked_at = None;
        for (kind, h, pos) in got {
            match kind {
                PickKind::Soul(v) => {
                    picked_at = Some(pos);
                    let hero = &mut self.heroes[h];
                    hero.p.souls += (v * 3.0 * hero.stats.greed * soul_mult).round() as u64;
                    gained_xp += v * hero.stats.growth;
                }
                PickKind::Draught => {
                    let amount = 40.0 * self.heroes[h].draught_mult() * if self.cursed_by(Curse::Famine) { 0.5 } else { 1.0 };
                    self.heroes[h].heal(amount);
                    self.sfx_at("kindle", 0.4, self.heroes[h].p.pos);
                    self.toast(&format!("Ashen Draught: +{:.0} health", amount));
                }
                PickKind::Bread => {
                    let amount = if self.cursed_by(Curse::Famine) { 30.0 } else { 60.0 };
                    self.heroes[h].heal(amount);
                    self.sfx("kindle", 0.4);
                    self.toast(&format!("{} eats grave bread: +60 health", self.heroes[h].name.clone()));
                }
                PickKind::Magnet => {
                    for p in self.pickups.iter_mut() {
                        if matches!(p.kind, PickKind::Soul(_)) && p.pulled.is_none() {
                            p.pulled = Some(h);
                        }
                    }
                    self.sfx_at("chest", 0.4, self.heroes[h].p.pos);
                }
                PickKind::Coffer(boss) => self.open_coffer(h, boss),
                PickKind::Buff(b) => {
                    self.quest_count(QuestKind::Relics, 1);
                    self.heroes[h].buffs[b as usize] = b.duration();
                    let who = if self.multiplayer() { format!("{}: ", self.heroes[h].name) } else { String::new() };
                    self.toast(&format!("{}{} ({} for {:.0}s)", who, b.name(), b.desc(), b.duration()));
                    self.sfx_at("evolve", 0.5, self.heroes[h].p.pos);
                    self.fx.push(Fx { pos, life: 0.6, max_life: 0.6, kind: FxKind::Ring { r: 120.0, color: b.color() } });
                }
                PickKind::CursedCoffer => self.open_cursed_coffer(h, pos),
                PickKind::Key => {
                    self.vault_key = true;
                    let who = self.heroes[h].name.clone();
                    self.toast(&format!("{} takes the courier's key. The vault is marked.", who));
                    self.sfx_at("chime", 0.8, pos);
                }
                PickKind::Relic => {
                    self.quest_count(QuestKind::Relics, 1);
                    self.grant_relic(h);
                }
                PickKind::Pyre => self.pyre(pos),
            }
        }
        if gained_xp > 0.0 {
            self.xp += gained_xp;
            match picked_at {
                Some(at) => self.sfx_gap_at("pickup", 0.18, 0.04, at),
                None => self.sfx_gap("pickup", 0.18, 0.04),
            }
        }
        while self.xp >= self.xp_next {
            self.xp -= self.xp_next;
            self.level += 1;
            self.xp_next = xp_for(self.level);
            self.pending_levels += 1;
        }
    }

    /// A relic found on the map: a free upgrade of a passive item.
    pub(crate) fn grant_relic(&mut self, h: usize) {
        let options: Vec<Choice> = self.heroes[h].pool(true).into_iter().map(|c| c.0).filter(|c| matches!(c, Choice::NewItem(_) | Choice::UpItem(_))).collect();
        let who = if self.multiplayer() { format!("{} finds ", self.heroes[h].name) } else { "Found ".to_string() };
        if options.is_empty() {
            self.heroes[h].p.souls += 250;
            self.toast(&format!("{}a reliquary: +250 souls", who));
        } else {
            let c = options[gen_range(0, options.len())];
            let (title, _, _, tag) = self.heroes[h].choice_text(c);
            self.heroes[h].apply_choice(c);
            self.toast(&format!("{}a relic: {} {}", who, title, tag));
        }
        self.sfx_at("chest", 0.8, self.heroes[h].p.pos);
    }

    /// Pyre bomb: burns everything around it.
    fn pyre(&mut self, c: Vec2) {
        let dmg = 120.0 * (1.0 + self.time / 120.0);
        let r = 480.0;
        self.hit_circle(c, r, dmg, 500.0, Src { hero: self.nearest_hero(c).unwrap_or(0), weapon: WeaponKind::Orb }, None);
        self.fx.push(Fx { pos: c, life: 0.7, max_life: 0.7, kind: FxKind::Explosion { r, color: Color::from_rgba(255, 90, 30, 255) } });
        self.fx.push(Fx { pos: c, life: 1.0, max_life: 1.0, kind: FxKind::Ring { r: r * 1.2, color: Color::from_rgba(255, 150, 60, 255) } });
        for _ in 0..60 {
            self.particle(
                c + rand_dir() * gen_range(0.0, r * 0.7),
                vec2(gen_range(-40.0, 40.0), gen_range(-260.0, -60.0)),
                gen_range(0.5, 1.2),
                gen_range(2.0, 5.0),
                Color::from_rgba(255, 120, 40, 255),
                true,
            );
        }
        self.add_decal(c, 140.0, Color::from_rgba(14, 10, 8, 190), DecalKind::Scorch, 40.0);
        self.shake = self.shake.max(14.0);
        self.sfx_at("explode", 1.0, c);
        self.toast("A pyre bursts into flame!");
    }

    /// Opens a coffer for hero `h`; the result is announced to everyone.
    pub fn open_coffer(&mut self, h: usize, boss: bool) {
        let name = self.heroes[h].name.clone();
        let prefix = if self.multiplayer() { format!("{}: ", name) } else { String::new() };
        if let Some(i) = self.heroes[h].evolvable() {
            let k = self.heroes[h].weapons[i].kind;
            self.heroes[h].weapons[i].evolved = true;
            self.heroes[h].recompute_stats();
            self.banner("EVOLUTION", &format!("{}{} becomes {}", prefix, k.name(false), k.name(true)), Color::from_rgba(255, 200, 110, 255), 4.0);
            self.toast(k.evo_desc());
            self.sfx_at("evolve", 1.0, self.heroes[h].p.pos);
            return;
        }
        let mut n = if boss { 3 } else { 1 };
        if chance(0.15 * self.heroes[h].stats.luck) {
            n += 2;
        }
        let mut got = Vec::new();
        for _ in 0..n {
            let pool = self.heroes[h].pool(false);
            if pool.is_empty() {
                self.heroes[h].p.souls += 200;
                got.push("+200 souls".to_string());
                continue;
            }
            let c = pool[gen_range(0, pool.len())].0;
            let (title, _, _, tag) = self.heroes[h].choice_text(c);
            self.heroes[h].apply_choice(c);
            got.push(format!("{} {}", title, tag));
        }
        self.toast(&format!("{}Coffer: {}", prefix, got.join(", ")));
        self.sfx_at("chest", 0.9, self.heroes[h].p.pos);
        self.check_fusion(h);
    }

    /// The Grave Cat: sits until a hero comes close, then follows them for the rest of the vigil.
    pub(crate) fn update_cat(&mut self, dt: f32) {
        let Some(mut cat) = self.cat.take() else { return };
        self.step_cat(&mut cat, dt);
        self.cat = Some(cat);
    }

    fn step_cat(&mut self, cat: &mut Cat, dt: f32) {
        cat.meow -= dt;
        let owner = cat.owner.and_then(|s| self.heroes.iter().position(|h| h.slot == s));
        let Some(h) = owner else {
            let found = self.heroes.iter().position(|h| h.alive() && h.p.pos.distance(cat.pos) < 34.0);
            if let Some(h) = found {
                cat.owner = Some(self.heroes[h].slot);
                self.heroes[h].has_cat = true;
                self.heroes[h].recompute_stats();
                let slot = self.heroes[h].slot;
                let who = if self.multiplayer() { format!("{}: ", self.heroes[h].name) } else { String::new() };
                self.banner("A GRAVE CAT", &format!("{}it has decided you are its human. +13% luck", who), Color::from_rgba(230, 200, 90, 255), 4.0);
                self.sfx_at("meow", 0.9, cat.pos);
                self.events.push(Event::CatFound(slot));
            } else if cat.meow <= 0.0 {
                cat.meow = gen_range(10.0, 25.0);
                // Only heard nearby, as a hint.
                if self.heroes.iter().any(|h| h.p.pos.distance(cat.pos) < 700.0) {
                    self.sfx_at("meow", 0.35, cat.pos);
                }
            }
            return;
        };
        let hero = &self.heroes[h];
        let target = hero.p.pos - hero.p.facing * 28.0 + vec2(-hero.p.facing.y, hero.p.facing.x) * 14.0;
        let to = target - cat.pos;
        let far = to.length();
        cat.sitting = far < 8.0 || hero.downed;
        if !cat.sitting {
            let speed = (hero.stats.speed * 1.25).max(far * 2.5);
            cat.pos += to.normalize_or_zero() * (speed * dt).min(far);
            if to.x.abs() > 1.0 {
                cat.facing = to.x.signum();
            }
        }
        if far > 900.0 {
            cat.pos = target;
        }
        if cat.meow <= 0.0 {
            cat.meow = gen_range(20.0, 45.0);
            self.sfx_at("meow", 0.3, cat.pos);
        }
    }

    pub(crate) fn update_shrines(&mut self, dt: f32) {
        let heroes = self.alive_positions();
        let mut kindled = false;
        for s in self.shrines.iter_mut() {
            s.life -= dt;
            if s.used {
                continue;
            }
            if heroes.iter().any(|p| s.pos.distance(*p) < 50.0) {
                s.kindle += dt;
                if s.kindle >= 1.5 {
                    s.used = true;
                    s.life = s.life.min(20.0);
                    kindled = true;
                }
            } else {
                s.kindle = (s.kindle - dt).max(0.0);
            }
        }
        self.shrines.retain(|s| s.life > 0.0);
        if kindled {
            let mut total = 0;
            for h in self.heroes.iter_mut() {
                total += h.p.souls;
                h.banked += h.p.souls;
                h.p.souls = 0;
                h.p.hp = h.stats.max_hp;
                if h.downed && h.connected {
                    h.downed = false;
                    h.revive = 0.0;
                    h.p.iframes = 2.0;
                }
            }
            let c = self.center();
            self.ward_burst(c, 320.0);
            self.banner("ASH KINDLED", &format!("{} souls are safe", total), Color::from_rgba(255, 170, 90, 255), 3.0);
            self.sfx("kindle", 0.9);
        }
        let claim = self.remnant.as_ref().and_then(|r| heroes.iter().any(|p| r.pos.distance(*p) < 30.0).then_some(r.souls));
        if let Some(n) = claim {
            self.heroes[0].p.souls += n;
            self.remnant = None;
            self.banner("SOULS RECLAIMED", &format!("+{} souls", n), Color::from_rgba(140, 220, 200, 255), 3.0);
            self.sfx_at("chest", 0.8, self.heroes[0].p.pos);
            self.events.push(Event::RemnantClaimed);
        }
    }
}

impl World {
    /// The small secrets of the field: the lost grave, and a fire for those who stand still.
    pub(crate) fn update_secrets(&mut self, dt: f32) {
        if !self.grave_found {
            if let Some(h) = self.heroes.iter().position(|h| h.alive() && h.p.pos.distance(LOST_GRAVE) < 40.0) {
                self.grave_found = true;
                self.banner("HERE LIES THE LAST KEEPER", "\"I kept the vigil. I forgot to roll.\"", Color::from_rgba(200, 200, 220, 255), 5.0);
                self.sfx_at("bell", 0.7, LOST_GRAVE);
                self.grant_relic(h);
            }
        }
        for h in 0..self.heroes.len() {
            let pos = self.heroes[h].p.pos;
            let quiet = !self.enemies.iter().any(|e| !e.dead && e.pos.distance_squared(pos) < 400.0 * 400.0);
            let hero = &mut self.heroes[h];
            if !hero.alive() || hero.p.moving || hero.p.roll_t > 0.0 || !quiet {
                hero.idle = 0.0;
                continue;
            }
            hero.idle += dt;
            if hero.idle >= REST_AFTER && !hero.rested {
                hero.rested = true;
                hero.heal(hero.stats.max_hp * 0.25);
                let at = pos + vec2(22.0, 10.0);
                let who = if self.multiplayer() { format!("{} rests", self.heroes[h].name) } else { "You rest".to_string() };
                self.toast(&format!("{} a while by a small fire. The dark can wait.", who));
                // A harmless little fire to sit by.
                self.pools.push(FirePool { pos: at, r: 12.0, life: 12.0, max_life: 12.0, dps: 0.0, tick: 1.0, src: Src { hero: h, weapon: WeaponKind::Orb }, hex: false });
                self.sfx_at("kindle", 0.6, at);
            }
        }
    }
}
