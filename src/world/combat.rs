//! Damage in both directions: hitting enemies, hurting heroes, enemy movement and hazards.

use super::*;
use std::collections::HashMap;

/// Sven's lunge: how long he winds up (with a red line on the ground), how long the lunge
/// lasts and how many times his pace it goes.
const RIVAL_WINDUP: f32 = 0.85;
const RIVAL_LUNGE_TIME: f32 = 0.35;
const RIVAL_LUNGE_SPEED: f32 = 3.6;

impl World {
    /// Damages enemy `i`. `tag` stamps the per-weapon hit timer used by lingering attacks.
    pub fn hit(&mut self, i: usize, base: f32, knock: Vec2, src: Src, tag: bool) -> bool {
        if self.enemies[i].dead {
            return false;
        }
        let (crit_chance, elite_dmg) = self.heroes.get(src.hero).map(|h| (h.stats.crit, h.stats.elite_dmg)).unwrap_or((0.05, 1.0));
        let crit = chance(crit_chance);
        let blocked = self.enemies[i].kind == EnemyKind::Shieldbearer && knock != Vec2::ZERO && {
            let e = &self.enemies[i];
            let facing = self.nearest_hero(e.pos).map(|h| (self.heroes[h].p.pos - e.pos).normalize_or_zero()).unwrap_or(Vec2::ZERO);
            knock.normalize_or_zero().dot(facing) < -0.3
        };
        let e = &mut self.enemies[i];
        let mut dmg = base * if crit { 2.0 } else { 1.0 } * gen_range(0.9, 1.1);
        if blocked {
            dmg *= 0.2;
        }
        if e.elite || e.boss {
            dmg *= elite_dmg;
        }
        if self.weather.is_some_and(|(w, _)| w == Weather::AshRain) && src.weapon.is_fire() {
            dmg *= 1.4;
        }
        if self.weather.is_some_and(|(w, _)| w == Weather::Blizzard) {
            e.slow = e.slow.max(0.8);
        }
        if e.affix == Affix::Warded {
            dmg *= 0.6;
        }
        if e.freeze > 0.0 {
            dmg *= FROZEN_TAKES;
        }
        if !self.dot_tick {
            match src.weapon.element() {
                // Fire: burns for a while after.
                3 => {
                    e.burn = e.burn.max(2.0);
                    e.burn_dps = e.burn_dps.max(base * 0.2);
                    e.burn_src = src;
                }
                // Frost: chilled, slower for a moment.
                4 => e.slow = e.slow.max(0.9),
                // Poison: a slower burn that lasts longer.
                5 => {
                    e.burn = e.burn.max(4.0);
                    e.burn_dps = e.burn_dps.max(base * 0.12);
                    e.burn_src = src;
                }
                _ => {}
            }
        }
        if self.twins.is_some_and(|(a, b)| e.id == a || e.id == b) {
            dmg *= TWIN_GUARD;
        }
        if e.boss || matches!(e.kind, EnemyKind::Rival | EnemyKind::Champion) {
            // No single blow takes more than a sliver of a lord (or of Sven): every fight lasts
            // a while.
            dmg = dmg.min(e.max_hp * BOSS_HIT_CAP);
        }
        if e.boss {
            // Nor more than a sliver of it in any one second. In a boss rush that sliver shrinks
            // with the difficulty's lord health: a hero who fells every lord as fast as this
            // allows would otherwise never feel the health a harder rush gives its lords.
            let second_cap = if self.boss_rush { BOSS_SECOND_CAP / self.diff.boss_hp.max(1.0).sqrt() } else { BOSS_SECOND_CAP };
            let id = e.id;
            let now = self.time;
            let slot = match self.boss_second.iter().position(|s| s.0 == id) {
                Some(k) => k,
                None => {
                    self.boss_second.push((id, now, 0.0));
                    self.boss_second.len() - 1
                }
            };
            let s = &mut self.boss_second[slot];
            if now - s.1 >= 1.0 {
                *s = (id, now, 0.0);
            }
            let left = (e.max_hp * second_cap - s.2).max(0.0);
            dmg = dmg.min(left);
            s.2 += dmg;
        }
        let dealt = dmg.min(e.hp.max(0.0));
        e.hp -= dmg;
        e.flash = 0.1;
        let resist = if e.boss || matches!(e.kind, EnemyKind::Prop(_)) {
            0.0
        } else if e.elite || matches!(e.kind, EnemyKind::Knight | EnemyKind::Shieldbearer | EnemyKind::BarkHusk | EnemyKind::FrozenKnight | EnemyKind::Statue) {
            0.3
        } else {
            1.0
        };
        e.knock += knock * resist;
        if tag {
            e.last_hit[src.weapon.idx()] = self.time;
        }
        let (pos, kind, r) = (e.pos, e.kind, e.r);
        let killed = e.hp <= 0.0;
        if blocked {
            self.particle(pos - knock.normalize_or_zero() * r, rand_dir() * 120.0, 0.2, 2.0, Color::from_rgba(255, 220, 150, 255), true);
            self.sfx_gap_at("clang", 0.15, 0.12, pos);
        }
        if let Some(h) = self.heroes.get_mut(src.hero) {
            h.record_damage(src.weapon, dealt, self.time);
        }
        let num = DmgNum { pos: pos + vec2(gen_range(-6.0, 6.0), -10.0), val: dmg, life: 0.7, crit };
        // A lord holding out against the cap takes nothing: no zeros to show.
        if self.show_numbers && self.nums.len() < 250 && dmg >= 0.5 {
            self.nums.push(num);
        }
        if self.record_net && self.net_nums.len() < 60 && dmg >= 0.5 {
            self.net_nums.push(num);
        }
        if chance(0.55) {
            self.blood_spray(pos, knock.normalize_or_zero(), kind.blood(), r, if crit { 5 } else { 2 });
        }
        self.sfx_gap_at("hit", 0.25, 0.06, pos);
        // A Stormcaller's crit leaps on to the next foe, once.
        if crit && !self.leaping && self.heroes.get(src.hero).is_some_and(|h| h.class == Class::Stormcaller) {
            let next = self.nearest_enemies(pos, 170.0, 4).into_iter().find(|&j| j != i && !self.enemies[j].dead);
            if let Some(j) = next {
                let to = self.enemies[j].pos;
                self.fx.push(Fx { pos: to, life: 0.2, max_life: 0.2, kind: FxKind::Lightning { from: pos } });
                self.leaping = true;
                self.hit(j, base * 0.5, Vec2::ZERO, src, false);
                self.leaping = false;
            }
        }
        if killed {
            self.enemies[i].dead = true;
            if matches!(self.enemies[i].kind, EnemyKind::Prop(_)) {
                self.break_prop(i);
            } else {
                self.on_kill(i, src);
            }
        }
        killed
    }

    fn on_kill(&mut self, i: usize, src: Src) {
        self.kills += 1;
        let kind = self.enemies[i].kind;
        self.count_beast(kind);
        let (pos, kind, xp, elite, boss, r, affix) = {
            let e = &self.enemies[i];
            (e.pos, e.kind, e.xp, e.elite, e.boss, e.r, e.affix)
        };
        match kind {
            EnemyKind::SmokeWraith => {
                let dmg = 6.0 * (1.0 + self.time / 600.0) * self.diff.at(self.time).dmg;
                let off = gen_range(0.0, TAU);
                for k in 0..5 {
                    self.enemy_bullet(pos, Vec2::from_angle(off + k as f32 * TAU / 5.0) * 110.0, 5.0, dmg, 0.0, 2.5);
                }
            }
            EnemyKind::FrozenKnight => {
                let dmg = 8.0 * (1.0 + self.time / 600.0) * self.diff.at(self.time).dmg;
                let off = gen_range(0.0, TAU);
                for k in 0..8 {
                    self.enemy_bullet(pos, Vec2::from_angle(off + k as f32 * TAU / 8.0) * 170.0, 5.0, dmg, 0.0, 2.5);
                }
                self.sfx_gap_at("chime", 0.4, 0.1, pos);
            }
            _ => {}
        }
        if affix == Affix::Volatile {
            let dmg = 8.0 * (1.0 + self.time / 600.0) * self.diff.at(self.time).dmg;
            let off = gen_range(0.0, TAU);
            for k in 0..8 {
                self.enemy_bullet(pos, Vec2::from_angle(off + k as f32 * TAU / 8.0) * 150.0, 5.0, dmg, 0.0, 3.0);
            }
        }
        self.quest_count(QuestKind::Slay, 1);
        if elite {
            self.quest_count(QuestKind::Elites, 1);
        }
        if self.cursed_by(Curse::Volatile) && !boss && chance(0.2) {
            // Volatile: the body bursts into a few slow bullets.
            let dmg = 6.0 * (1.0 + self.time / 600.0) * self.diff.at(self.time).dmg;
            let off = gen_range(0.0, TAU);
            for k in 0..4 {
                self.enemy_bullet(pos, Vec2::from_angle(off + k as f32 * TAU / 4.0) * 120.0, 5.0, dmg, 0.0, 2.5);
            }
        }
        if let Some(h) = self.heroes.get_mut(src.hero) {
            let ls = h.stats.lifesteal.min(h.leech);
            h.leech -= ls;
            h.heal(ls);
        }
        self.death_burst(pos, kind, elite, r);
        if self.record_net && self.net_deaths.len() < 80 {
            self.net_deaths.push((pos, kind, elite));
        }
        self.sfx_gap_at("die", 0.3, 0.05, pos);
        if kind == EnemyKind::Bloat {
            for k in 0..8 {
                let a = k as f32 / 8.0 * TAU;
                self.enemy_bullet(pos, Vec2::from_angle(a) * 90.0, 6.0, 8.0 * self.diff.at(self.time).dmg, 0.0, 2.5);
            }
        }
        if boss {
            let id = self.enemies[i].id;
            let fallen = match kind {
                EnemyKind::Boss(k) => Some(k),
                _ => None,
            };
            self.boss_second.retain(|s| s.0 != id);
            self.uncrown(id);
            // One of twin lords falls: the chain breaks and the other rages.
            if let Some((a, b)) = self.twins.filter(|(a, b)| *a == id || *b == id) {
                self.twins = None;
                let other = if a == id { b } else { a };
                if let Some(o) = self.enemies.iter_mut().find(|e| e.id == other && !e.dead) {
                    o.speed *= 1.3;
                    o.dmg *= 1.25;
                    let name = if let EnemyKind::Boss(k) = o.kind { k.name() } else { "The other lord" };
                    let line = format!("the chain breaks: {} rages", name);
                    self.log_event(&line);
                    self.banner("THE CHAIN BREAKS", &format!("{} rages over its twin", name), Color::from_rgba(230, 110, 255, 255), 3.5);
                }
            }
            if self.boss.as_ref().is_some_and(|b| b.id == id) {
                // The next lord still standing takes over the fight.
                self.boss = if self.more_bosses.is_empty() { None } else { Some(self.more_bosses.remove(0)) };
            } else {
                self.more_bosses.retain(|b| b.id != id);
            }
            self.boss_kind = self.boss.as_ref().map(|b| b.kind);
            let last = self.boss.is_none();
            let still = 1 + self.more_bosses.len();
            let name = fallen.map(|k| k.name()).unwrap_or("A lord");
            let line = if last {
                format!("{} falls (lord {} of the vigil)", name, self.bosses_killed + 1)
            } else {
                format!("{} falls; {} lord{} still stand", name, still, if still == 1 { "" } else { "s" })
            };
            self.log_event(&line);
            self.bosses_killed += 1;
            if last {
                self.arena = None;
                self.ebullets.clear();
                self.pillars.clear();
                self.telegraphs.clear();
                self.waves.clear();
            } else {
                self.boss_bar_lag = 1.0;
            }
            // Every hero gets a coffer from a boss.
            let n = self.heroes.len().max(1);
            for k in 0..n {
                let off = if n > 1 { Vec2::from_angle(k as f32 / n as f32 * TAU) * 40.0 } else { Vec2::ZERO };
                self.pickups.push(Pickup::new(pos + off, PickKind::Coffer(true)));
            }
            // Tithe: the most generous hero's upgrade counts for the whole party.
            let tithe = 1.0 + 0.2 * self.heroes.iter().map(|h| h.profile.m(M_TITHE)).max().unwrap_or(0) as f32;
            for _ in 0..30 {
                let v = 10.0 * (1.0 + self.time / 300.0) * tithe;
                self.drop_soul(pos + rand_dir() * gen_range(10.0, 90.0), v);
            }
            self.banner("FOE VANQUISHED", "", Color::from_rgba(230, 190, 90, 255), 4.0);
            self.sfx("bell", 1.0);
            self.sfx("explode", 0.8);
            self.shake = 18.0;
            self.fx.push(Fx { pos, life: 1.0, max_life: 1.0, kind: FxKind::Ring { r: 300.0, color: Color::from_rgba(255, 210, 140, 255) } });
            if let Some(k) = fallen {
                self.say(k.lines()[3], super::spawn::boss_voice(k));
            }
            self.fulfil_pacts();
            self.settle_wagers();
            if !last {
                // Another lord still stands: the fight goes on.
                return;
            }
            // Fallen companions rise again once the boss is down.
            for h in self.heroes.iter_mut().filter(|h| h.downed && h.connected) {
                h.downed = false;
                h.p.hp = h.rise_health(0.4);
                h.p.iframes = 2.0;
            }
            if self.boss_rush {
                // The next lord follows after a breather, and the heroes grow to meet it.
                self.next_boss = self.time + RUSH_BREATHER;
                self.pending_levels += RUSH_LEVELS_PER_LORD;
            } else {
                // A lord that outlived its five minutes: the next one still gets its warning.
                self.next_boss = self.next_boss.max(self.time + BOSS_WARNING);
            }
            self.events.push(Event::BossDefeated);
            return;
        }
        if elite {
            self.pickups.push(Pickup::new(pos, PickKind::Coffer(false)));
        }
        if kind == EnemyKind::Champion {
            self.log_event("the champion of the mist falls");
        }
        match kind {
            EnemyKind::Courier => {
                self.log_event("the courier is caught and drops the vault key");
                self.pickups.push(Pickup::new(pos, PickKind::Key));
                self.banner("THE COURIER FALLS", "Take its key to the vault", Color::from_rgba(240, 200, 90, 255), 3.0);
            }
            EnemyKind::Mimic(_) => {
                self.log_event("a mimic falls");
                for k in 0..2 {
                    let off = Vec2::from_angle(k as f32 * std::f32::consts::PI) * 24.0;
                    self.pickups.push(Pickup::new(pos + off, PickKind::Coffer(false)));
                }
                for _ in 0..16 {
                    self.drop_soul(pos + rand_dir() * gen_range(10.0, 70.0), xp * 0.5);
                }
                self.toast("The mimic spills its hoard.");
            }
            EnemyKind::Rival => {
                self.log_event("Sven falls");
                for _ in 0..24 {
                    self.drop_soul(pos + rand_dir() * gen_range(10.0, 80.0), xp * 0.4);
                }
                if chance(0.35) {
                    self.pickups.push(Pickup::new(pos, PickKind::Coffer(false)));
                }
                self.banner("SVEN FALLS", "His souls are yours", Color::from_rgba(230, 80, 70, 255), 3.0);
            }
            _ => {}
        }
        self.drop_soul(pos, xp);
        let luck = self.heroes.iter().map(|h| h.stats.luck).fold(1.0f32, f32::max);
        // Draughts and magnets lie until picked up, so on a crowded floor they stop dropping.
        let room = self.pickups.len() < MAX_PICKUPS - 50;
        // Late in the night far more foes fall each second; each drops a draught or a magnet
        // that much more rarely, so the floor is not strewn with them.
        let late = 1.0 / (1.0 + self.time / 60.0 * 0.3);
        if room && chance(0.009 * luck * late) {
            self.pickups.push(Pickup::new(pos + rand_dir() * 8.0, PickKind::Draught));
        } else if room && chance(0.0025 * luck * late) {
            self.pickups.push(Pickup::new(pos + rand_dir() * 8.0, PickKind::Magnet));
        }
    }

    pub(crate) fn drop_soul(&mut self, pos: Vec2, v: f32) {
        let v = v * self.mutated(Mutator::Gilded, 1.25);
        // A full floor makes room by merging old orbs, so new ones still drop where foes fall.
        if self.pickups.len() >= MAX_PICKUPS {
            self.merge_souls();
        }
        // Nothing left to merge (every orb is flying to a hero): join the nearest orb instead.
        if self.pickups.len() >= MAX_PICKUPS {
            let nearest = self
                .pickups
                .iter()
                .enumerate()
                .filter(|(_, p)| matches!(p.kind, PickKind::Soul(_)))
                .min_by(|a, b| a.1.pos.distance_squared(pos).total_cmp(&b.1.pos.distance_squared(pos)))
                .map(|(i, _)| i);
            if let Some(i) = nearest {
                if let PickKind::Soul(ref mut x) = self.pickups[i].kind {
                    *x += v;
                    return;
                }
            }
        }
        let mut p = Pickup::new(pos, PickKind::Soul(v));
        p.bob = gen_range(0.0, TAU);
        self.pickups.push(p);
    }

    /// Merges lying soul orbs that are close together into one worth all of them, at their
    /// value-weighted centre, until the floor is back to `MERGED_PICKUPS`. Neighbours on a fine
    /// grid go first; the grid grows only while that is not enough. Orbs flying to a hero stay.
    pub(crate) fn merge_souls(&mut self) {
        let mut cell = 96.0;
        while self.pickups.len() > MERGED_PICKUPS && cell < 8000.0 {
            let mut keeper: HashMap<(i32, i32), usize> = HashMap::new();
            let mut gone = vec![false; self.pickups.len()];
            for (i, merged) in gone.iter_mut().enumerate() {
                let (pos, v) = match self.pickups[i].kind {
                    PickKind::Soul(v) if self.pickups[i].pulled.is_none() => (self.pickups[i].pos, v),
                    _ => continue,
                };
                let key = ((pos.x / cell).floor() as i32, (pos.y / cell).floor() as i32);
                match keeper.get(&key) {
                    Some(&k) => {
                        let keep = &mut self.pickups[k];
                        if let PickKind::Soul(kv) = keep.kind {
                            let total = kv + v;
                            keep.pos = (keep.pos * kv + pos * v) / total.max(0.001);
                            keep.kind = PickKind::Soul(total);
                        }
                        *merged = true;
                    }
                    None => {
                        keeper.insert(key, i);
                    }
                }
            }
            let mut i = 0;
            self.pickups.retain(|_| {
                i += 1;
                !gone[i - 1]
            });
            cell *= 2.0;
        }
    }

    pub fn hurt_hero(&mut self, h: usize, dmg: f32) {
        if self.dead {
            return;
        }
        let now = self.time;
        let hero = &mut self.heroes[h];
        if hero.downed || hero.possessing || hero.p.iframes > 0.0 || hero.p.roll_t > 0.0 || hero.buff(Buff::Aegis) > 0.0 {
            return;
        }
        let p = hero.p.pos;
        if chance(0.02 * hero.profile.m(M_EVASION) as f32) {
            // Evasion: the blow finds only air, and a short grace follows.
            hero.p.iframes = 0.3;
            self.particle(p + vec2(0.0, -20.0), vec2(0.0, -30.0), 0.5, 3.0, Color::from_rgba(220, 220, 255, 255), true);
            return;
        }
        hero.first_hurt = hero.first_hurt.min(now);
        hero.last_hurt = now;
        let taken = 1.0 - 0.02 * hero.profile.m(M_BULWARK) as f32;
        let d = (dmg * taken - hero.stats.armor).max(1.0);
        hero.p.hp -= d;
        hero.p.iframes = 0.45 + 0.08 * hero.profile.m(M_WARDING) as f32;
        if h == self.local {
            self.shake = self.shake.max(6.0);
        }
        self.sfx_at("hurt", 0.7, p);
        self.last_wound = now;
        self.blood_spray(p, rand_dir(), Color::from_rgba(150, 10, 12, 255), 10.0, 8);
        if self.heroes[h].p.hp > 0.0 {
            return;
        }
        let hero = &mut self.heroes[h];
        // An ember rekindles a hero at most once in `EMBER_COOLDOWN` seconds.
        if hero.p.revives > 0 && now >= hero.ember_ready_at {
            hero.p.revives -= 1;
            hero.ember_ready_at = now + EMBER_COOLDOWN;
            hero.p.hp = hero.rise_health(0.5);
            hero.p.iframes = 2.0;
            let name = hero.name.clone();
            let left = hero.p.revives;
            self.log_event(&format!("{} is rekindled by an ember ({} left)", name, left));
            self.ward_burst(p, 400.0);
            self.banner("EMBER REKINDLED", &format!("Death refuses {}, this once.", name), Color::from_rgba(255, 150, 60, 255), 3.0);
            self.sfx_at("kindle", 1.0, p);
            return;
        }
        hero.p.hp = 0.0;
        let near = self.enemies.iter().filter(|e| !e.dead && e.pos.distance(p) < 300.0).count();
        let lords: Vec<&str> = self.enemies.iter().filter(|e| e.boss && !e.dead).filter_map(|e| if let EnemyKind::Boss(k) = e.kind { Some(k.name()) } else { None }).collect();
        let fighting = if lords.is_empty() { String::new() } else { format!(", fighting {}", lords.join(" and ")) };
        let line = format!("{} falls at level {} with {} foes close by{}", self.heroes[h].name, self.level, near, fighting);
        self.log_event(&line);
        self.add_decal(p, 26.0, Color::from_rgba(90, 5, 8, 220), DecalKind::Blood, 60.0);
        if self.heroes.len() > 1 {
            let name = self.heroes[h].name.clone();
            self.heroes[h].downed = true;
            self.heroes[h].revive = 0.0;
            self.heroes[h].ghost = self.heroes[h].p.pos;
            self.sfx("death", 0.6);
            self.toast(&format!("{} has fallen. Stand beside them to revive.", name));
            if self.heroes.iter().all(|x| x.downed || x.possessing) {
                self.dead = true;
                self.sfx("death", 1.0);
                self.events.push(Event::Died);
            }
        } else {
            self.dead = true;
            self.sfx("death", 1.0);
            self.events.push(Event::Died);
        }
    }

    /// A cultist's bomb goes off: it hurts heroes and the cultist dies in the blast.
    fn cultist_blast(&mut self, i: usize) {
        if self.enemies[i].dead {
            return;
        }
        let (pos, dmg) = (self.enemies[i].pos, self.enemies[i].dmg);
        self.hurt_area(pos, 80.0, dmg * 3.0);
        self.fx.push(Fx { pos, life: 0.45, max_life: 0.45, kind: FxKind::Explosion { r: 80.0, color: Color::from_rgba(255, 110, 40, 255) } });
        self.add_decal(pos, 40.0, Color::from_rgba(16, 10, 8, 180), DecalKind::Scorch, 25.0);
        self.sfx_gap_at("explode", 0.6, 0.05, pos);
        self.shake = self.shake.max(5.0);
        let e = &mut self.enemies[i];
        e.dead = true;
        let (kind, r) = (e.kind, e.r);
        self.kills += 1;
        self.count_beast(kind);
        self.death_burst(pos, kind, false, r);
        if self.record_net && self.net_deaths.len() < 80 {
            self.net_deaths.push((pos, kind, false));
        }
    }

    /// A necromancer pulls three hollows out of the ground around it.
    fn raise_dead(&mut self, at: Vec2) {
        if self.enemy_room() < 3 {
            return;
        }
        for k in 0..3 {
            let pos = clamp_world(at + Vec2::from_angle(k as f32 * TAU / 3.0 + gen_range(0.0, 1.0)) * 60.0, 40.0);
            let mut e = self.make_enemy(EnemyKind::Hollow, pos);
            e.xp *= 0.5;
            self.enemies.push(e);
            for _ in 0..6 {
                self.particle(pos, vec2(gen_range(-30.0, 30.0), gen_range(-90.0, -30.0)), 0.6, 2.5, Color::from_rgba(120, 255, 140, 255), true);
            }
        }
        self.fx.push(Fx { pos: at, life: 0.6, max_life: 0.6, kind: FxKind::Ring { r: 140.0, color: Color::from_rgba(120, 255, 140, 255) } });
        self.sfx_gap_at("roar", 0.25, 1.0, at);
    }

    /// Hurts every living hero within `r` of `c`.
    pub fn hurt_area(&mut self, c: Vec2, r: f32, dmg: f32) {
        for h in 0..self.heroes.len() {
            if self.heroes[h].alive() && self.heroes[h].p.pos.distance(c) < r {
                self.hurt_hero(h, dmg);
            }
        }
    }

    /// Knocks back and clears bullets around `p`.
    pub(crate) fn ward_burst(&mut self, p: Vec2, radius: f32) {
        for e in self.enemies.iter_mut() {
            let d = e.pos - p;
            if !e.boss && d.length() < radius {
                e.knock += d.normalize_or_zero() * 900.0;
            }
        }
        self.clear_bullets(p, radius);
        self.fx.push(Fx { pos: p, life: 0.8, max_life: 0.8, kind: FxKind::Ring { r: radius, color: Color::from_rgba(255, 170, 80, 255) } });
    }

    /// Enemy projectiles are bigger than they used to be and always red.
    pub(crate) fn enemy_bullet(&mut self, pos: Vec2, vel: Vec2, r: f32, dmg: f32, homing: f32, life: f32) {
        let hp = BULLET_HP * self.hp_scale();
        self.ebullets.push(EBullet { pos, vel, r: (r * 1.5).max(8.0), dmg, life, homing, hp, hit_cd: 0.0 });
    }

    /// Destroys every enemy projectile inside a circle. Returns how many.
    pub(crate) fn clear_bullets(&mut self, c: Vec2, r: f32) -> usize {
        self.damage_bullets(c, r, f32::INFINITY)
    }

    /// Deals `dmg` to enemy projectiles inside a circle; those it breaks are gone. Returns how many.
    pub(crate) fn damage_bullets(&mut self, c: Vec2, r: f32, dmg: f32) -> usize {
        self.damage_bullets_where(|p, br| p.distance(c) < r + br, dmg)
    }

    pub(crate) fn damage_bullets_where(&mut self, inside: impl Fn(Vec2, f32) -> bool, dmg: f32) -> usize {
        let mut gone = Vec::new();
        let mut struck = Vec::new();
        self.ebullets.retain_mut(|b| {
            if !inside(b.pos, b.r) || b.hit_cd > 0.0 {
                return true;
            }
            b.hp -= dmg;
            b.hit_cd = 0.15;
            if b.hp <= 0.0 {
                gone.push(b.pos);
                false
            } else {
                struck.push(b.pos);
                true
            }
        });
        // A bullet that holds out flickers instead of bursting.
        for p in &struck {
            self.particle(*p, rand_dir() * 60.0, 0.15, 1.5, Color::from_rgba(255, 200, 160, 255), true);
        }
        for p in &gone {
            for _ in 0..4 {
                self.particle(*p, rand_dir() * gen_range(40.0, 140.0), 0.3, 2.0, Color::from_rgba(255, 60, 40, 255), true);
            }
        }
        if let Some(at) = gone.first().copied() {
            self.sfx_gap_at("hit", 0.2, 0.08, at);
        }
        gone.len()
    }

    /// Nearest living enemies within `max_dist`, closest first.
    pub(crate) fn nearest_enemies(&mut self, from: Vec2, max_dist: f32, n: usize) -> Vec<usize> {
        let mut q = std::mem::take(&mut self.qbuf);
        self.grid.query(from, max_dist, &mut q);
        let mut v: Vec<(f32, usize)> = q
            .iter()
            .filter(|&&i| i < self.enemies.len() && !self.enemies[i].dead && !matches!(self.enemies[i].kind, EnemyKind::Prop(_)))
            .map(|&i| (self.enemies[i].pos.distance_squared(from), i))
            .filter(|(d, _)| *d < max_dist * max_dist)
            .collect();
        self.qbuf = q;
        v.sort_by(|a, b| a.0.total_cmp(&b.0));
        v.truncate(n);
        v.into_iter().map(|x| x.1).collect()
    }

    /// Hits everything in a circle once and cuts down enemy bullets there.
    /// With `interval`, each enemy is hit at most once per that many seconds by this weapon.
    pub(crate) fn hit_circle(&mut self, c: Vec2, r: f32, dmg: f32, knock: f32, src: Src, interval: Option<f32>) -> usize {
        let mut q = std::mem::take(&mut self.qbuf);
        self.grid.query(c, r, &mut q);
        let mut n = 0;
        for &i in &q {
            let e = &self.enemies[i];
            if e.dead || e.pos.distance(c) > r + e.r {
                continue;
            }
            if let Some(iv) = interval {
                if self.time - e.last_hit[src.weapon.idx()] < iv {
                    continue;
                }
            }
            let k = (e.pos - c).normalize_or_zero() * knock;
            self.hit(i, dmg, k, src, interval.is_some());
            n += 1;
        }
        self.qbuf = q;
        self.damage_bullets(c, r, dmg);
        n
    }

    /// Gathers cold on foe `i`; once it has gathered enough, it freezes.
    pub(crate) fn chill(&mut self, i: usize, amount: f32) {
        let e = &mut self.enemies[i];
        if e.dead || e.boss || matches!(e.kind, EnemyKind::Prop(_)) {
            return;
        }
        e.chill += amount;
        if e.chill >= 1.0 {
            e.chill = 0.0;
            e.freeze = e.freeze.max(1.3);
            let (pos, r) = (e.pos, e.r);
            self.fx.push(Fx { pos, life: 0.3, max_life: 0.3, kind: FxKind::Ring { r: r + 10.0, color: Color::from_rgba(190, 230, 255, 255) } });
        }
    }

    pub(crate) fn explode(&mut self, c: Vec2, r: f32, dmg: f32, evo: bool, src: Src) {
        self.hit_circle(c, r, dmg, 200.0, src, None);
        self.fx.push(Fx { pos: c, life: 0.35, max_life: 0.35, kind: FxKind::Explosion { r, color: Color::from_rgba(255, 140, 40, 255) } });
        for _ in 0..12 {
            self.particle(c, rand_dir() * gen_range(50.0, 200.0), gen_range(0.3, 0.7), gen_range(2.0, 4.0), Color::from_rgba(255, 160, 60, 255), true);
        }
        self.add_decal(c, r * 0.6, Color::from_rgba(20, 14, 10, 160), DecalKind::Scorch, 25.0);
        self.sfx_gap_at("explode", 0.35, 0.08, c);
        if evo {
            self.pools.push(FirePool { pos: c, r: r * 0.8, life: 3.0, max_life: 3.0, dps: dmg * 0.6, tick: 0.0, src, kind: PoolKind::Fire });
        }
    }

    pub(crate) fn update_hazards(&mut self, dt: f32) {
        for k in 0..self.pools.len() {
            self.pools[k].life -= dt;
            self.pools[k].tick -= dt;
            if self.pools[k].kind == PoolKind::Pane {
                // Whatever is shot at a pane of ice breaks on it.
                let (c, r) = (self.pools[k].pos, self.pools[k].r);
                self.ebullets.retain(|b| b.pos.distance(c) >= r + b.r + 4.0);
            }
            if self.pools[k].tick <= 0.0 {
                let p = &self.pools[k];
                let (c, r, kind, src) = (p.pos, p.r, p.kind, p.src);
                if kind == PoolKind::Candle {
                    // A candle spits a flame at the nearest foe in reach.
                    self.pools[k].tick = 0.55;
                    let dmg = self.pools[k].dps;
                    if let Some(&i) = self.nearest_enemies(c, 260.0, 1).first() {
                        let dir = (self.enemies[i].pos - c).normalize_or_zero();
                        let at = c + vec2(0.0, -14.0);
                        self.projs.push(Proj {
                            pos: at,
                            vel: dir * 380.0,
                            dir,
                            r: 5.0,
                            dmg,
                            pierce: 0,
                            life: 1.0,
                            age: 0.0,
                            kind: PKind::Ember,
                            evo: false,
                            area: 1.0,
                            src,
                            hits: Vec::new(),
                            dead: false,
                        });
                    }
                } else if kind == PoolKind::IceSpike {
                    // It bursts up once, and again a little later if there is time left.
                    let d = self.pools[k].dps;
                    self.pools[k].tick = 0.45;
                    self.hit_circle(c, r, d, 60.0, src, None);
                    for e in self.enemies.iter_mut().filter(|e| !e.dead && !e.boss && e.pos.distance(c) < r + e.r) {
                        e.freeze = e.freeze.max(0.4);
                    }
                    self.fx.push(Fx { pos: c, life: 0.4, max_life: 0.4, kind: FxKind::IceSpikes { r } });
                } else if kind == PoolKind::FireBurst {
                    // The pike's victim blows up.
                    let d = self.pools[k].dps;
                    self.pools[k].life = 0.0;
                    self.pools[k].tick = 99.0;
                    self.explode(c, r, d, false, src);
                } else if kind == PoolKind::Icicle {
                    // The icicle strikes where its shadow fell.
                    let d = self.pools[k].dps;
                    self.pools[k].life = 0.0;
                    self.pools[k].tick = 99.0;
                    self.hit_circle(c, r, d, 80.0, src, None);
                    for e in self.enemies.iter_mut().filter(|e| !e.dead && !e.boss && e.pos.distance(c) < r + e.r) {
                        e.freeze = e.freeze.max(0.6);
                    }
                    self.fx.push(Fx { pos: c, life: 0.4, max_life: 0.4, kind: FxKind::IceSpikes { r: r * 0.8 } });
                    self.fx.push(Fx { pos: c, life: 0.3, max_life: 0.3, kind: FxKind::Ring { r, color: Color::from_rgba(200, 235, 255, 255) } });
                    self.shake = self.shake.max(2.0);
                    self.sfx_gap_at("slam", 0.25, 0.1, c);
                } else if kind == PoolKind::RuneTrap || kind == PoolKind::Nightshade {
                    // Waiting for a foe to step near.
                    self.pools[k].tick = 0.1;
                    let reach = if kind == PoolKind::RuneTrap { r + 8.0 } else { r * 3.0 };
                    if !self.foes_within(c, reach).is_empty() {
                        let d = self.pools[k].dps;
                        self.pools[k].life = 0.0;
                        let big = r * 4.2;
                        if kind == PoolKind::RuneTrap {
                            self.hit_circle(c, big, d, 150.0, src, None);
                            for e in self.enemies.iter_mut().filter(|e| !e.dead && !e.boss && e.pos.distance(c) < big + e.r) {
                                e.freeze = e.freeze.max(1.0);
                            }
                            self.fx.push(Fx { pos: c, life: 0.4, max_life: 0.4, kind: FxKind::IceSpikes { r: big * 0.7 } });
                            self.fx.push(Fx { pos: c, life: 0.35, max_life: 0.35, kind: FxKind::Ring { r: big, color: Color::from_rgba(150, 210, 255, 255) } });
                            self.sfx_gap_at("chime", 0.3, 0.1, c);
                        } else {
                            self.hit_circle(c, big, d, 60.0, src, None);
                            let life = 2.5;
                            self.pools.push(FirePool { pos: c, r: big, life, max_life: life, dps: d * 0.35, tick: 0.0, src, kind: PoolKind::Pollen });
                            self.fx.push(Fx { pos: c, life: 0.35, max_life: 0.35, kind: FxKind::Explosion { r: big, color: Color::from_rgba(150, 70, 190, 255) } });
                            self.sfx_gap_at("hit", 0.3, 0.1, c);
                        }
                    }
                } else if kind == PoolKind::Pane {
                    // Foes that touch the pane are bitten and thrown back.
                    self.pools[k].tick = 0.3;
                    let d = self.pools[k].dps * 0.5;
                    self.hit_circle(c, r + 6.0, d, 260.0, src, None);
                } else {
                    self.pools[k].tick = 0.3;
                    let d = self.pools[k].dps * 0.3;
                    self.hit_circle(c, r, d, 0.0, src, None);
                    if matches!(kind, PoolKind::Frostpool | PoolKind::Bramble | PoolKind::Pollen) {
                        let dps = self.pools[k].dps;
                        for i in self.foes_within(c, r) {
                            match kind {
                                PoolKind::Frostpool => {
                                    self.enemies[i].slow = self.enemies[i].slow.max(0.6);
                                    self.chill(i, 0.12);
                                }
                                PoolKind::Bramble => {
                                    let e = &mut self.enemies[i];
                                    e.held = e.held.max(0.35);
                                    e.bleed = e.bleed.max(1.5);
                                    e.bleed_dps = e.bleed_dps.max(dps * 0.25);
                                    e.bleed_src = src;
                                }
                                _ => self.enemies[i].slow = self.enemies[i].slow.max(0.3),
                            }
                        }
                    }
                    if kind == PoolKind::Holy {
                        for e in self.enemies.iter_mut().filter(|e| !e.dead && e.pos.distance(c) < r + e.r) {
                            e.slow = e.slow.max(0.45);
                        }
                    }
                }
            }
            if chance(0.4) {
                let p = &self.pools[k];
                let c = p.pos + rand_dir() * gen_range(0.0, p.r);
                let col = match p.kind {
                    PoolKind::Hex => Color::from_rgba(200, 60, 230, 255),
                    PoolKind::Holy => Color::from_rgba(170, 220, 255, 255),
                    PoolKind::Smoke => Color::from_rgba(150, 140, 130, 255),
                    PoolKind::Blood => Color::from_rgba(150, 10, 18, 255),
                    PoolKind::Frostpool | PoolKind::Icicle | PoolKind::RuneTrap | PoolKind::Pane | PoolKind::IceSpike => Color::from_rgba(190, 230, 255, 255),
                    PoolKind::Bramble => Color::from_rgba(110, 160, 70, 255),
                    PoolKind::Pollen => Color::from_rgba(230, 220, 120, 255),
                    PoolKind::Nightshade | PoolKind::Bloom => Color::from_rgba(160, 90, 200, 255),
                    _ => Color::from_rgba(255, 120, 30, 255),
                };
                self.particle(c, vec2(0.0, -40.0), 0.5, 3.0, col, true);
            }
        }
        self.pools.retain(|p| p.life > 0.0);

        let targets: Vec<(usize, Vec2)> = self.heroes.iter().enumerate().filter(|(_, h)| h.alive()).map(|(i, h)| (i, h.p.pos)).collect();
        let mut hurts: Vec<(usize, f32)> = Vec::new();
        for b in self.ebullets.iter_mut() {
            b.hit_cd = (b.hit_cd - dt).max(0.0);
            if b.homing > 0.0 {
                if let Some((_, t)) = targets.iter().min_by(|a, c| a.1.distance(b.pos).total_cmp(&c.1.distance(b.pos))) {
                    let speed = b.vel.length();
                    let desired = (*t - b.pos).normalize_or_zero() * speed;
                    b.vel = b.vel.lerp(desired, (b.homing * dt).min(1.0)).normalize_or_zero() * speed;
                }
            }
            b.pos += b.vel * dt;
            b.life -= dt;
            if !inside_world(b.pos, 0.0) {
                b.life = 0.0;
            }
            for (h, t) in &targets {
                if b.life > 0.0 && b.pos.distance(*t) < b.r + 8.0 {
                    hurts.push((*h, b.dmg));
                    b.life = 0.0;
                }
            }
        }
        self.ebullets.retain(|b| b.life > 0.0);

        for w in self.waves.iter_mut() {
            w.r += w.speed * dt;
            for (h, t) in &targets {
                let d = w.pos.distance(*t);
                if *h < MAX_HEROES && !w.hit[*h] && (d - w.r).abs() < 14.0 {
                    w.hit[*h] = true;
                    hurts.push((*h, w.dmg));
                }
            }
        }
        self.waves.retain(|w| w.r < w.max_r);

        let mut erupted = Vec::new();
        for p in self.pillars.iter_mut() {
            if p.delay > 0.0 {
                p.delay -= dt;
                if p.delay <= 0.0 {
                    for (h, t) in &targets {
                        if p.pos.distance(*t) < p.r + 6.0 {
                            hurts.push((*h, p.dmg));
                        }
                    }
                    erupted.push((p.pos, p.r));
                }
            } else {
                p.after -= dt;
            }
        }
        for (c, r) in erupted {
            self.fx.push(Fx { pos: c, life: 0.4, max_life: 0.4, kind: FxKind::Explosion { r, color: Color::from_rgba(255, 90, 40, 255) } });
            for _ in 0..8 {
                self.particle(c + rand_dir() * gen_range(0.0, r), vec2(gen_range(-20.0, 20.0), gen_range(-260.0, -120.0)), 0.6, 3.5, Color::from_rgba(255, 120, 50, 255), true);
            }
            self.add_decal(c, r * 0.8, Color::from_rgba(18, 12, 10, 170), DecalKind::Scorch, 20.0);
            self.sfx_gap_at("explode", 0.3, 0.06, c);
        }
        self.pillars.retain(|p| p.delay > 0.0 || p.after > 0.0);
        for (h, d) in hurts {
            self.hurt_hero(h, d);
        }

        for t in self.telegraphs.iter_mut() {
            t.life -= dt;
        }
        self.telegraphs.retain(|t| t.life > 0.0);
    }

    pub(crate) fn update_enemies(&mut self, dt: f32) {
        let targets = self.alive_positions();
        if targets.is_empty() {
            return;
        }
        let far = self.spawn_ring_dist() * 1.5;
        let t = self.time;
        let mut arrows: Vec<(Vec2, Vec2)> = Vec::new();
        let mut blasts: Vec<usize> = Vec::new();
        let mut raises: Vec<Vec2> = Vec::new();
        let mut wails: Vec<Vec2> = Vec::new();
        let mut burns: Vec<(usize, f32, Src)> = Vec::new();
        let mut slams: Vec<Vec2> = Vec::new();
        let mut volleys: Vec<(Vec2, Vec2)> = Vec::new();
        let mut lunges: Vec<(Vec2, Vec2, f32)> = Vec::new();
        let mut woke: Vec<Vec2> = Vec::new();
        // The newer fields' foes: shards and thorns thrown, moths called.
        let mut shards: Vec<(Vec2, Vec2)> = Vec::new();
        let mut thorns: Vec<(Vec2, Vec2)> = Vec::new();
        let mut calls: Vec<Vec2> = Vec::new();
        // Where every living hero stands and looks, for the statues.
        let eyes: Vec<(Vec2, Vec2)> = self.heroes.iter().filter(|h| h.alive()).map(|h| (h.p.pos, h.p.facing)).collect();
        let seen = |p: Vec2| {
            eyes.iter().any(|(at, facing)| {
                let to = p - *at;
                let d = to.length();
                d < 1100.0 && (d < 40.0 || to.dot(*facing) / d > 0.35)
            })
        };
        let mut mends: Vec<Vec2> = Vec::new();
        let mut blinks: Vec<(Vec2, Vec2)> = Vec::new();
        let (anchor, anchor_facing, anchor_moving) = self.random_hero_pos();
        let sight = self.spawn_ring_dist() * 0.9;
        // Where each hero stands, for a foe that hunts one of them.
        let hero_at: Vec<Option<Vec2>> = self.heroes.iter().map(|h| h.alive().then_some(h.p.pos)).collect();
        // In the grave mist everything wades.
        let wade = if self.weather.is_some_and(|(w, _)| matches!(w, Weather::GraveMist | Weather::Blizzard)) { 0.8 } else { 1.0 };
        #[cfg(test)]
        let forced_affix = self.force_affix;
        for (i, e) in self.enemies.iter_mut().enumerate() {
            if e.dead {
                continue;
            }
            e.flash -= dt;
            e.slow = (e.slow - dt).max(0.0);
            if e.bleed > 0.0 {
                e.bleed -= dt;
                e.bleed_acc += dt;
                if e.bleed_acc >= 0.5 {
                    e.bleed_acc = 0.0;
                    burns.push((i, e.bleed_dps * 0.5, e.bleed_src));
                }
            }
            if e.burn > 0.0 {
                e.burn -= dt;
                e.burn_acc += dt;
                if e.burn_acc >= 0.5 {
                    e.burn_acc = 0.0;
                    burns.push((i, e.burn_dps * 0.5, e.burn_src));
                }
            }
            e.knock *= (-8.0 * dt).exp();
            if e.boss {
                e.pos = clamp_world(e.pos + e.knock * dt, e.r);
                continue;
            }
            if e.life_left.is_finite() {
                e.life_left -= dt;
                if e.life_left <= 0.0 {
                    // It leaves the field: no souls, no drops.
                    e.dead = true;
                    continue;
                }
            }
            let nearest = *targets.iter().min_by(|a, b| a.distance_squared(e.pos).total_cmp(&b.distance_squared(e.pos))).unwrap();
            let pp = e.hunt.and_then(|h| hero_at.get(h).copied().flatten()).unwrap_or(nearest);
            let to = pp - e.pos;
            let d = to.length().max(0.001);
            let dir = to / d;
            e.chill = (e.chill - dt * 0.1).max(0.0);
            if (e.freeze > 0.0 || e.held > 0.0) && !e.boss {
                // Frozen in place, or held fast by roots.
                e.freeze = (e.freeze - dt).max(0.0);
                e.held = (e.held - dt).max(0.0);
                e.knock *= 0.8;
                continue;
            }
            e.freeze = (e.freeze - dt).max(0.0);
            e.held = (e.held - dt).max(0.0);
            if matches!(e.kind, EnemyKind::Prop(_)) {
                // Props stand where they stand.
                e.knock = Vec2::ZERO;
                continue;
            }
            if e.elite && e.affix == Affix::None && !e.boss && !matches!(e.kind, EnemyKind::Champion | EnemyKind::Rival) {
                e.affix = ALL_AFFIXES[gen_range(0, ALL_AFFIXES.len())];
                #[cfg(test)]
                if let Some(a) = forced_affix {
                    e.affix = a;
                }
            }
            let tick = |every: f32| (t + e.phase * 3.0).rem_euclid(every) < dt;
            match e.affix {
                Affix::Mender if tick(1.0) => mends.push(e.pos),
                Affix::Blinker if tick(4.0) && d > 120.0 && d < 700.0 => {
                    let from = e.pos;
                    e.pos += dir * (d - 90.0).min(240.0);
                    blinks.push((from, e.pos));
                }
                _ => {}
            }
            let swift = if e.affix == Affix::Swift { 1.6 } else { 1.0 };
            let slow = if e.slow > 0.0 { 0.5 } else { 1.0 } * swift;
            let mv = match e.kind {
                EnemyKind::Cultist => {
                    if e.fuse < 0.0 && d < 60.0 {
                        e.fuse = 0.75;
                    }
                    if e.fuse >= 0.0 {
                        e.fuse -= dt;
                        if e.fuse <= 0.0 {
                            blasts.push(i);
                        }
                        dir * 0.25
                    } else {
                        dir
                    }
                }
                EnemyKind::Necromancer => {
                    e.timer -= dt;
                    if e.timer <= 0.0 && d < 700.0 {
                        e.timer = 6.0;
                        raises.push(e.pos);
                    }
                    if d > 420.0 {
                        dir
                    } else if d < 320.0 {
                        -dir
                    } else {
                        vec2(-dir.y, dir.x) * 0.5
                    }
                }
                EnemyKind::Moth => Vec2::from_angle(dir.to_angle() + (t * 5.0 + e.phase).sin() * 0.9),
                EnemyKind::FrostCrow => Vec2::from_angle(dir.to_angle() + (t * 4.0 + e.phase).sin() * 0.7),
                EnemyKind::EmberWolf => {
                    // Runs with the pack and leaps the last stretch.
                    e.timer -= dt;
                    if e.fuse > 0.0 {
                        e.fuse -= dt;
                        dir * 2.6
                    } else if e.timer <= 0.0 && d < 260.0 {
                        e.timer = gen_range(2.0, 3.2);
                        e.fuse = 0.35;
                        dir * 2.6
                    } else {
                        dir
                    }
                }
                EnemyKind::SmokeWraith => {
                    e.alpha = 0.45 + 0.4 * (t * 2.2 + e.phase).sin().abs();
                    Vec2::from_angle(dir.to_angle() + (t * 1.7 + e.phase).sin() * 0.6)
                }
                EnemyKind::IceMonk => {
                    e.timer -= dt;
                    if e.timer <= 0.0 && d < 560.0 {
                        e.timer = 2.8;
                        shards.push((e.pos, dir));
                    }
                    if d > 330.0 {
                        dir
                    } else if d < 220.0 {
                        -dir
                    } else {
                        vec2(-dir.y, dir.x) * 0.5
                    }
                }
                EnemyKind::ThornVine => {
                    e.timer -= dt;
                    if e.timer <= 0.0 && d < 480.0 {
                        e.timer = 3.0;
                        thorns.push((e.pos, dir));
                    }
                    dir * 0.7
                }
                EnemyKind::Puppet => {
                    // Snips along, then charges with its shears.
                    e.timer -= dt;
                    if e.fuse > 0.0 {
                        e.fuse -= dt;
                        dir * 3.0
                    } else if e.timer <= 0.0 && d < 420.0 {
                        e.timer = gen_range(3.5, 4.5);
                        e.fuse = 0.5;
                        dir * 3.0
                    } else {
                        dir
                    }
                }
                EnemyKind::MothQueen => {
                    e.timer -= dt;
                    if e.timer <= 0.0 && d < 650.0 {
                        e.timer = 6.0;
                        calls.push(e.pos);
                    }
                    if d > 380.0 {
                        dir
                    } else if d < 280.0 {
                        -dir
                    } else {
                        vec2(-dir.y, dir.x) * 0.6
                    }
                }
                // A statue moves only while nobody looks at it; `step` says which, for drawing.
                EnemyKind::Statue if seen(e.pos) => {
                    e.step = 0;
                    Vec2::ZERO
                }
                EnemyKind::Statue => {
                    e.step = 1;
                    dir
                }
                EnemyKind::Leech => Vec2::from_angle(dir.to_angle() + (t * 9.0 + e.phase).sin() * 0.5),
                EnemyKind::Gargoyle => {
                    // Creeps closer, then springs: `fuse` counts down the lunge.
                    e.timer -= dt;
                    if e.fuse > 0.0 {
                        e.fuse -= dt;
                        dir * 4.5
                    } else if e.timer <= 0.0 && d < 380.0 {
                        e.timer = gen_range(3.0, 4.5);
                        e.fuse = 0.4;
                        dir * 4.5
                    } else {
                        dir
                    }
                }
                EnemyKind::Banshee => {
                    e.alpha = 0.7 + 0.25 * (t * 2.5 + e.phase).sin();
                    e.timer -= dt;
                    if e.timer <= 0.0 && d < 560.0 {
                        e.timer = 3.4;
                        wails.push(e.pos);
                    }
                    if d > 360.0 {
                        dir
                    } else if d < 250.0 {
                        -dir
                    } else {
                        vec2(-dir.y, dir.x) * 0.5
                    }
                }
                EnemyKind::Wraith => {
                    e.alpha = 0.55 + 0.35 * (t * 3.0 + e.phase).sin();
                    Vec2::from_angle(dir.to_angle() + (t * 2.0 + e.phase).sin() * 0.4)
                }
                EnemyKind::Archer => {
                    e.timer -= dt;
                    if e.timer <= 0.0 && d < 520.0 {
                        e.timer = 2.6;
                        arrows.push((e.pos, dir));
                    }
                    if d > 300.0 {
                        dir
                    } else if d < 200.0 {
                        -dir
                    } else {
                        vec2(-dir.y, dir.x) * (e.phase - std::f32::consts::PI).signum() * 0.6
                    }
                }
                EnemyKind::Courier => {
                    // Runs from the nearest hero, swerving, and keeps off the walls.
                    let away = -dir + vec2(-dir.y, dir.x) * (t * 1.3 + e.phase).sin() * 0.6;
                    let inward = if inside_world(e.pos, 300.0) { Vec2::ZERO } else { -e.pos.normalize_or_zero() * 1.2 };
                    (away + inward).normalize_or_zero() * if d < 500.0 { 1.0 } else { 0.55 }
                }
                EnemyKind::Champion => {
                    // Charges now and then; slams the ground at the end of each charge.
                    e.timer -= dt;
                    if e.fuse > 0.0 {
                        e.fuse -= dt;
                        if e.fuse <= 0.0 {
                            slams.push(e.pos);
                        }
                        dir * 3.2
                    } else if e.timer <= 0.0 && d < 520.0 {
                        e.timer = gen_range(4.0, 5.5);
                        e.fuse = 0.7;
                        dir * 3.2
                    } else {
                        dir
                    }
                }
                EnemyKind::Mimic(false) => {
                    // Lies still like any coffer, until a hero comes close or strikes it.
                    if d < 90.0 || e.hp < e.max_hp {
                        e.kind = EnemyKind::Mimic(true);
                        e.timer = 0.5;
                        woke.push(e.pos);
                    }
                    Vec2::ZERO
                }
                EnemyKind::Mimic(true) => {
                    // Hops after its prey and now and then leaps to bite.
                    e.timer -= dt;
                    if e.fuse > 0.0 {
                        e.fuse -= dt;
                        dir * 3.2
                    } else if e.timer <= 0.0 && d < 380.0 {
                        e.timer = gen_range(1.3, 1.9);
                        e.fuse = 0.35;
                        dir * 3.2
                    } else {
                        dir * (0.6 + 0.5 * (t * 9.0 + e.phase).sin().abs())
                    }
                }
                EnemyKind::Rival => {
                    // A dance that can be read: he circles and throws fans of blades; every few
                    // seconds he stops, marks his lunge with a red line, lunges straight along
                    // it (no turning), and stands a moment to catch his breath.
                    e.fuse -= dt;
                    match e.step {
                        1 => {
                            if e.fuse <= 0.0 {
                                e.step = 2;
                                e.fuse = RIVAL_LUNGE_TIME;
                            }
                            Vec2::ZERO
                        }
                        2 => {
                            if e.fuse <= 0.0 {
                                e.step = 3;
                                e.fuse = 0.7;
                            }
                            e.lunge * RIVAL_LUNGE_SPEED
                        }
                        3 => {
                            if e.fuse <= 0.0 {
                                e.step = 0;
                                e.fuse = gen_range(3.5, 5.0);
                            }
                            Vec2::ZERO
                        }
                        _ => {
                            e.timer -= dt;
                            if e.timer <= 0.0 && d < 650.0 {
                                e.timer = gen_range(1.3, 1.7);
                                volleys.push((e.pos, dir));
                            }
                            if e.fuse <= 0.0 && d < 420.0 {
                                e.step = 1;
                                e.fuse = RIVAL_WINDUP;
                                e.lunge = dir;
                                // Never a volley right on top of the wind-up.
                                e.timer = e.timer.max(RIVAL_WINDUP + RIVAL_LUNGE_TIME + 0.9);
                                lunges.push((e.pos, dir, e.speed));
                                Vec2::ZERO
                            } else if d > 320.0 {
                                dir
                            } else if d < 200.0 {
                                -dir
                            } else {
                                vec2(-dir.y, dir.x) * (e.phase - std::f32::consts::PI).signum()
                            }
                        }
                    }
                }
                _ => dir,
            };
            e.pos += (mv * e.speed * slow * wade + e.knock) * dt;
            // The courier, the champion and the rival never jump ahead of the heroes.
            let roams = matches!(e.kind, EnemyKind::Courier | EnemyKind::Champion | EnemyKind::Rival | EnemyKind::Mimic(_) | EnemyKind::Prop(_));
            if d > far && !roams {
                // Wrap stragglers around to the front of a hero.
                let ahead = if anchor_moving { anchor_facing } else { rand_dir() };
                let mut a = ahead.to_angle() + gen_range(-0.9, 0.9);
                let mut np = anchor + Vec2::from_angle(a) * far * 0.7;
                // Near a wall, pick a spot that is still on the field.
                // Near a wall, or in another hero's view, pick a spot that is still on the
                // field and out of everyone's sight.
                let seen = |p: Vec2| hero_at.iter().flatten().any(|h| h.distance_squared(p) < sight * sight);
                let mut ok = inside_world(np, 60.0) && !seen(np);
                for _ in 0..10 {
                    if ok {
                        break;
                    }
                    a = gen_range(0.0, TAU);
                    np = anchor + Vec2::from_angle(a) * far * 0.7;
                    ok = inside_world(np, 60.0) && !seen(np);
                }
                if ok {
                    e.pos = np;
                }
            }
            e.pos = clamp_world(e.pos, e.r);
        }
        for at in mends {
            for e in self.enemies.iter_mut().filter(|e| !e.dead && !e.boss && e.pos.distance(at) < 170.0) {
                e.hp = (e.hp + e.max_hp * 0.025).min(e.max_hp);
            }
            self.fx.push(Fx { pos: at, life: 0.5, max_life: 0.5, kind: FxKind::Ring { r: 170.0, color: Affix::Mender.color() } });
        }
        for (from, to) in blinks {
            self.fx.push(Fx { pos: from, life: 0.3, max_life: 0.3, kind: FxKind::Ghost });
            self.fx.push(Fx { pos: to, life: 0.4, max_life: 0.4, kind: FxKind::Ring { r: 40.0, color: Affix::Blinker.color() } });
        }
        for (pos, dir) in shards {
            let dmg = 7.0 * (1.0 + self.time / 600.0) * self.diff.at(self.time).dmg;
            for k in -1..=1 {
                let v = Vec2::from_angle(dir.to_angle() + k as f32 * 0.22) * 230.0;
                self.enemy_bullet(pos, v, 5.0, dmg, 0.0, 3.0);
            }
            self.sfx_gap_at("chime", 0.25, 0.15, pos);
        }
        for (pos, dir) in thorns {
            let dmg = 6.0 * (1.0 + self.time / 600.0) * self.diff.at(self.time).dmg;
            for k in -1..=1 {
                let v = Vec2::from_angle(dir.to_angle() + k as f32 * 0.35) * 200.0;
                self.enemy_bullet(pos, v, 5.0, dmg, 0.0, 2.6);
            }
        }
        for pos in calls {
            let room = self.enemy_room().min(4);
            for k in 0..room {
                let e = self.make_enemy(EnemyKind::Moth, pos + Vec2::from_angle(k as f32 * TAU / 4.0) * 30.0);
                self.enemies.push(e);
            }
        }
        for pos in woke {
            self.log_event("a mimic wakes");
            self.banner("A MIMIC!", "The coffer has teeth", Color::from_rgba(240, 190, 90, 255), 2.5);
            self.sfx_at("roar", 0.7, pos);
            self.shake = self.shake.max(6.0);
        }
        for (pos, dir) in arrows {
            let dmg = 8.0 * (1.0 + self.time / 600.0) * self.diff.at(self.time).dmg;
            self.enemy_bullet(pos, dir * 260.0, 5.0, dmg, 0.0, 3.0);
            self.sfx_gap_at("arrow", 0.25, 0.1, pos);
        }
        self.dot_tick = true;
        for (i, d, src) in burns {
            self.hit(i, d, Vec2::ZERO, src, false);
        }
        self.dot_tick = false;
        for at in slams {
            // The champion's maul: a ring of bullets and a shockwave.
            let dmg = 10.0 * (1.0 + self.time / 600.0) * self.diff.at(self.time).dmg;
            let off = gen_range(0.0, TAU);
            for k in 0..14 {
                self.enemy_bullet(at, Vec2::from_angle(off + k as f32 / 14.0 * TAU) * 170.0, 7.0, dmg, 0.0, 3.0);
            }
            self.waves.push(Shockwave { pos: at, r: 10.0, max_r: 220.0, speed: 520.0, dmg, hit: [false; MAX_HEROES] });
            self.shake = self.shake.max(8.0);
            self.sfx_at("slam", 0.8, at);
        }
        for (at, dir, speed) in lunges {
            // The lunge's path, shown for the whole wind-up.
            let len = RIVAL_LUNGE_SPEED * RIVAL_LUNGE_TIME * speed;
            self.telegraphs.push(Telegraph { a: at, b: at + dir * len, width: 34.0, life: RIVAL_WINDUP, max_life: RIVAL_WINDUP, line: true });
            self.sfx_gap_at("roar", 0.25, 0.5, at);
        }
        for (at, dir) in volleys {
            // Sven throws five blades in a fan, fast.
            let dmg = 12.0 * (1.0 + self.time / 600.0) * self.diff.at(self.time).dmg;
            for k in -2..=2 {
                let v = Vec2::from_angle(dir.to_angle() + k as f32 * 0.16) * 380.0;
                self.enemy_bullet(at, v, 6.0, dmg, 0.0, 2.5);
            }
            self.sfx_gap_at("knife", 0.35, 0.1, at);
        }
        for i in blasts {
            self.cultist_blast(i);
        }
        for at in raises {
            self.raise_dead(at);
        }
        for at in wails {
            // A wail: a slow ring with gaps, easy to read, hard to ignore in numbers.
            let dmg = 7.0 * (1.0 + self.time / 600.0) * self.diff.at(self.time).dmg;
            let off = gen_range(0.0, TAU);
            for k in 0..8 {
                self.enemy_bullet(at, Vec2::from_angle(off + k as f32 / 8.0 * TAU) * 110.0, 6.0, dmg, 0.0, 4.0);
            }
            self.sfx_gap_at("chime", 0.3, 0.2, at);
        }

        // Soft separation so hordes don't collapse into one blob.
        let mut q = std::mem::take(&mut self.qbuf);
        for i in 0..self.enemies.len() {
            if self.enemies[i].dead || self.enemies[i].boss || matches!(self.enemies[i].kind, EnemyKind::Prop(_)) {
                continue;
            }
            let (p, r) = (self.enemies[i].pos, self.enemies[i].r);
            self.grid.query(p, r * 2.0, &mut q);
            let mut push = Vec2::ZERO;
            let mut checked = 0;
            for &j in &q {
                if j == i || j >= self.enemies.len() {
                    continue;
                }
                let o = &self.enemies[j];
                let d = p - o.pos;
                let min = r + o.r;
                let l2 = d.length_squared();
                if l2 < min * min && l2 > 0.0001 {
                    let l = l2.sqrt();
                    push += d / l * (min - l) * 0.5;
                    checked += 1;
                    if checked > 6 {
                        break;
                    }
                }
            }
            self.enemies[i].pos += push.clamp_length_max(4.0);
        }

        // Contact damage.
        let boss_pos: Vec<(Vec2, f32, f32)> = self.enemies.iter().filter(|e| e.boss && !e.dead).map(|e| (e.pos, e.r, e.dmg)).collect();
        let mut hurts = Vec::new();
        for h in 0..self.heroes.len() {
            if !self.heroes[h].alive() {
                continue;
            }
            let pp = self.heroes[h].p.pos;
            self.grid.query(pp, 60.0, &mut q);
            let mut contact = 0.0f32;
            for &i in &q {
                let e = &self.enemies[i];
                if !e.dead && e.pos.distance(pp) < e.r + 9.0 {
                    contact = contact.max(e.dmg);
                }
            }
            for &(bp, br, bd) in &boss_pos {
                if bp.distance(pp) < br + 9.0 {
                    contact = contact.max(bd);
                }
            }
            if contact > 0.0 {
                hurts.push((h, contact));
            }
        }
        self.qbuf = q;
        for (h, d) in hurts {
            self.hurt_hero(h, d);
        }
    }
}
