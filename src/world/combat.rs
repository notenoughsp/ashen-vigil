//! Damage in both directions: hitting enemies, hurting heroes, enemy movement and hazards.

use super::*;

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
        let dealt = dmg.min(e.hp.max(0.0));
        e.hp -= dmg;
        e.flash = 0.1;
        let resist = if e.boss {
            0.0
        } else if e.elite || matches!(e.kind, EnemyKind::Knight | EnemyKind::Shieldbearer) {
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
            self.sfx_gap("clang", 0.15, 0.12);
        }
        if let Some(h) = self.heroes.get_mut(src.hero) {
            h.record_damage(src.weapon, dealt, self.time);
        }
        let num = DmgNum { pos: pos + vec2(gen_range(-6.0, 6.0), -10.0), val: dmg, life: 0.7, crit };
        if self.show_numbers && self.nums.len() < 250 {
            self.nums.push(num);
        }
        if self.record_net && self.net_nums.len() < 60 {
            self.net_nums.push(num);
        }
        if chance(0.55) {
            self.blood_spray(pos, knock.normalize_or_zero(), kind.blood(), r, if crit { 5 } else { 2 });
        }
        self.sfx_gap("hit", 0.25, 0.06);
        if killed {
            self.enemies[i].dead = true;
            self.on_kill(i, src);
        }
        killed
    }

    fn on_kill(&mut self, i: usize, src: Src) {
        self.kills += 1;
        let (pos, kind, xp, elite, boss, r) = {
            let e = &self.enemies[i];
            (e.pos, e.kind, e.xp, e.elite, e.boss, e.r)
        };
        if let Some(h) = self.heroes.get_mut(src.hero) {
            let ls = h.stats.lifesteal.min(h.leech);
            h.leech -= ls;
            h.heal(ls);
        }
        self.death_burst(pos, kind, elite, r);
        if self.record_net && self.net_deaths.len() < 80 {
            self.net_deaths.push((pos, kind, elite));
        }
        self.sfx_gap("die", 0.3, 0.05);
        if kind == EnemyKind::Bloat {
            for k in 0..8 {
                let a = k as f32 / 8.0 * TAU;
                self.enemy_bullet(pos, Vec2::from_angle(a) * 90.0, 6.0, 8.0 * self.diff.at(self.time).dmg, 0.0, 2.5);
            }
        }
        if boss {
            self.boss = None;
            self.boss_kind = None;
            self.bosses_killed += 1;
            self.arena = None;
            self.ebullets.clear();
            self.pillars.clear();
            self.telegraphs.clear();
            self.waves.clear();
            // Every hero gets a coffer from a boss.
            let n = self.heroes.len().max(1);
            for k in 0..n {
                let off = if n > 1 { Vec2::from_angle(k as f32 / n as f32 * TAU) * 40.0 } else { Vec2::ZERO };
                self.pickups.push(Pickup::new(pos + off, PickKind::Coffer(true)));
            }
            for _ in 0..30 {
                let v = 10.0 * (1.0 + self.time / 300.0);
                self.drop_soul(pos + rand_dir() * gen_range(10.0, 90.0), v);
            }
            self.banner("FOE VANQUISHED", "", Color::from_rgba(230, 190, 90, 255), 4.0);
            self.sfx("bell", 1.0);
            self.sfx("explode", 0.8);
            self.shake = 18.0;
            self.fx.push(Fx { pos, life: 1.0, max_life: 1.0, kind: FxKind::Ring { r: 300.0, color: Color::from_rgba(255, 210, 140, 255) } });
            // Fallen companions rise again once the boss is down.
            for h in self.heroes.iter_mut().filter(|h| h.downed && h.connected) {
                h.downed = false;
                h.p.hp = h.stats.max_hp * 0.4;
                h.p.iframes = 2.0;
            }
            self.events.push(Event::BossDefeated);
            return;
        }
        if elite {
            self.pickups.push(Pickup::new(pos, PickKind::Coffer(false)));
        }
        self.drop_soul(pos, xp);
        let luck = self.heroes.iter().map(|h| h.stats.luck).fold(1.0f32, f32::max);
        if chance(0.006 * luck) {
            self.pickups.push(Pickup::new(pos + rand_dir() * 8.0, PickKind::Draught));
        } else if chance(0.0015 * luck) {
            self.pickups.push(Pickup::new(pos + rand_dir() * 8.0, PickKind::Magnet));
        }
    }

    fn drop_soul(&mut self, pos: Vec2, v: f32) {
        // Too many orbs on the floor: fold new ones into an existing orb instead, preferably
        // one still lying there, else one already flying to a hero; the souls are never lost.
        if self.pickups.len() > 500 {
            let lying = self.pickups.iter().position(|p| p.pulled.is_none() && matches!(p.kind, PickKind::Soul(_)));
            let any = || self.pickups.iter().position(|p| matches!(p.kind, PickKind::Soul(_)));
            if let Some(i) = lying.or_else(any) {
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

    pub fn hurt_hero(&mut self, h: usize, dmg: f32) {
        if self.dead {
            return;
        }
        let hero = &mut self.heroes[h];
        if hero.downed || hero.p.iframes > 0.0 || hero.p.roll_t > 0.0 || hero.buff(Buff::Aegis) > 0.0 {
            return;
        }
        let d = (dmg - hero.stats.armor).max(1.0);
        hero.p.hp -= d;
        hero.p.iframes = 0.45;
        let p = hero.p.pos;
        if h == self.local {
            self.shake = self.shake.max(6.0);
        }
        self.sfx("hurt", 0.7);
        self.blood_spray(p, rand_dir(), Color::from_rgba(150, 10, 12, 255), 10.0, 8);
        if self.heroes[h].p.hp > 0.0 {
            return;
        }
        let hero = &mut self.heroes[h];
        if hero.p.revives > 0 {
            hero.p.revives -= 1;
            hero.p.hp = hero.stats.max_hp * 0.5;
            hero.p.iframes = 2.0;
            let name = hero.name.clone();
            self.ward_burst(p, 400.0);
            self.banner("EMBER REKINDLED", &format!("Death refuses {}, this once.", name), Color::from_rgba(255, 150, 60, 255), 3.0);
            self.sfx("kindle", 1.0);
            return;
        }
        hero.p.hp = 0.0;
        self.add_decal(p, 26.0, Color::from_rgba(90, 5, 8, 220), DecalKind::Blood, 60.0);
        if self.heroes.len() > 1 {
            let name = self.heroes[h].name.clone();
            self.heroes[h].downed = true;
            self.heroes[h].revive = 0.0;
            self.sfx("death", 0.6);
            self.toast(&format!("{} has fallen. Stand beside them to revive.", name));
            if self.heroes.iter().all(|x| x.downed) {
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
        self.sfx_gap("explode", 0.6, 0.05);
        self.shake = self.shake.max(5.0);
        let e = &mut self.enemies[i];
        e.dead = true;
        let (kind, r) = (e.kind, e.r);
        self.kills += 1;
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
        self.sfx_gap("roar", 0.25, 1.0);
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
        self.ebullets.push(EBullet { pos, vel, r: (r * 1.5).max(8.0), dmg, life, homing });
    }

    /// Destroys enemy projectiles inside a circle. Returns how many.
    pub(crate) fn clear_bullets(&mut self, c: Vec2, r: f32) -> usize {
        let mut gone = Vec::new();
        self.ebullets.retain(|b| {
            let hit = b.pos.distance(c) < r + b.r;
            if hit {
                gone.push(b.pos);
            }
            !hit
        });
        for p in &gone {
            for _ in 0..4 {
                self.particle(*p, rand_dir() * gen_range(40.0, 140.0), 0.3, 2.0, Color::from_rgba(255, 60, 40, 255), true);
            }
        }
        if !gone.is_empty() {
            self.sfx_gap("hit", 0.2, 0.08);
        }
        gone.len()
    }

    /// Nearest living enemies within `max_dist`, closest first.
    pub(crate) fn nearest_enemies(&mut self, from: Vec2, max_dist: f32, n: usize) -> Vec<usize> {
        let mut q = std::mem::take(&mut self.qbuf);
        self.grid.query(from, max_dist, &mut q);
        let mut v: Vec<(f32, usize)> = q
            .iter()
            .filter(|&&i| i < self.enemies.len() && !self.enemies[i].dead)
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
        self.clear_bullets(c, r);
        n
    }

    pub(crate) fn explode(&mut self, c: Vec2, r: f32, dmg: f32, evo: bool, src: Src) {
        self.hit_circle(c, r, dmg, 200.0, src, None);
        self.fx.push(Fx { pos: c, life: 0.35, max_life: 0.35, kind: FxKind::Explosion { r, color: Color::from_rgba(255, 140, 40, 255) } });
        for _ in 0..12 {
            self.particle(c, rand_dir() * gen_range(50.0, 200.0), gen_range(0.3, 0.7), gen_range(2.0, 4.0), Color::from_rgba(255, 160, 60, 255), true);
        }
        self.add_decal(c, r * 0.6, Color::from_rgba(20, 14, 10, 160), DecalKind::Scorch, 25.0);
        self.sfx_gap("explode", 0.35, 0.08);
        if evo {
            self.pools.push(FirePool { pos: c, r: r * 0.8, life: 3.0, max_life: 3.0, dps: dmg * 0.6, tick: 0.0, src, hex: false });
        }
    }

    pub(crate) fn update_hazards(&mut self, dt: f32) {
        for k in 0..self.pools.len() {
            self.pools[k].life -= dt;
            self.pools[k].tick -= dt;
            if self.pools[k].tick <= 0.0 {
                self.pools[k].tick = 0.3;
                let p = &self.pools[k];
                let (c, r, d, src) = (p.pos, p.r, p.dps * 0.3, p.src);
                self.hit_circle(c, r, d, 0.0, src, None);
            }
            if chance(0.4) {
                let p = &self.pools[k];
                let c = p.pos + rand_dir() * gen_range(0.0, p.r);
                let col = if p.hex { Color::from_rgba(200, 60, 230, 255) } else { Color::from_rgba(255, 120, 30, 255) };
                self.particle(c, vec2(0.0, -40.0), 0.5, 3.0, col, true);
            }
        }
        self.pools.retain(|p| p.life > 0.0);

        let targets: Vec<(usize, Vec2)> = self.heroes.iter().enumerate().filter(|(_, h)| h.alive()).map(|(i, h)| (i, h.p.pos)).collect();
        let mut hurts: Vec<(usize, f32)> = Vec::new();
        for b in self.ebullets.iter_mut() {
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
            self.sfx_gap("explode", 0.3, 0.06);
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
        let mut burns: Vec<(usize, f32, Src)> = Vec::new();
        let (anchor, anchor_facing, anchor_moving) = self.random_hero_pos();
        for (i, e) in self.enemies.iter_mut().enumerate() {
            if e.dead {
                continue;
            }
            e.flash -= dt;
            e.slow = (e.slow - dt).max(0.0);
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
            let pp = *targets.iter().min_by(|a, b| a.distance_squared(e.pos).total_cmp(&b.distance_squared(e.pos))).unwrap();
            let to = pp - e.pos;
            let d = to.length().max(0.001);
            let dir = to / d;
            let slow = if e.slow > 0.0 { 0.5 } else { 1.0 };
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
                _ => dir,
            };
            e.pos += (mv * e.speed * slow + e.knock) * dt;
            if d > far {
                // Wrap stragglers around to the front of a hero.
                let ahead = if anchor_moving { anchor_facing } else { rand_dir() };
                let mut a = ahead.to_angle() + gen_range(-0.9, 0.9);
                let mut np = anchor + Vec2::from_angle(a) * far * 0.7;
                // Near a wall, pick a spot that is still on the field.
                for _ in 0..8 {
                    if inside_world(np, 60.0) {
                        break;
                    }
                    a = gen_range(0.0, TAU);
                    np = anchor + Vec2::from_angle(a) * far * 0.7;
                }
                e.pos = np;
            }
            e.pos = clamp_world(e.pos, e.r);
        }
        for (pos, dir) in arrows {
            let dmg = 8.0 * (1.0 + self.time / 600.0) * self.diff.at(self.time).dmg;
            self.enemy_bullet(pos, dir * 260.0, 5.0, dmg, 0.0, 3.0);
            self.sfx_gap("arrow", 0.25, 0.1);
        }
        for (i, d, src) in burns {
            self.hit(i, d, Vec2::ZERO, src, false);
        }
        for i in blasts {
            self.cultist_blast(i);
        }
        for at in raises {
            self.raise_dead(at);
        }

        // Soft separation so hordes don't collapse into one blob.
        let mut q = std::mem::take(&mut self.qbuf);
        for i in 0..self.enemies.len() {
            if self.enemies[i].dead || self.enemies[i].boss {
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
        let boss_pos = self.boss.as_ref().and_then(|b| self.enemies.iter().find(|e| e.id == b.id)).map(|e| (e.pos, e.r, e.dmg));
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
            if let Some((bp, br, bd)) = boss_pos {
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
