//! Enemy waves, elites, bosses, shrines and relics lying on the map.

use super::*;

impl World {
    /// How much tougher foes are now than at the start, with difficulty, co-op and curses.
    pub(crate) fn hp_scale(&self) -> f32 {
        let m = self.time / 60.0;
        let coop = 1.0 + 0.35 * (self.heroes.len().max(1) - 1) as f32;
        let cursed = self.cursed.as_ref().map(|c| c.curse > 0.0).unwrap_or(false);
        (1.0 + m * 0.2 + (m / 10.0).powi(2) * 0.6) * self.diff.at(self.time).hp * coop * if cursed { 1.3 } else { 1.0 }
    }

    pub(crate) fn make_enemy(&mut self, kind: EnemyKind, pos: Vec2) -> Enemy {
        let m = self.time / 60.0;
        let hp_scale = self.hp_scale();
        let dmg_scale = (1.0 + m * 0.06) * self.diff.at(self.time).dmg * if self.blood_moon > 0.0 { 1.25 } else { 1.0 };
        let (hp, speed, dmg, xp) = match kind {
            EnemyKind::Hollow => (10.0, 55.0, 8.0, 1.0),
            EnemyKind::Ghoul => (8.0, 95.0, 6.0, 1.0),
            EnemyKind::Moth => (4.0, 120.0, 4.0, 0.5),
            EnemyKind::Archer => (14.0, 55.0, 6.0, 2.0),
            EnemyKind::Bloat => (60.0, 35.0, 14.0, 4.0),
            EnemyKind::Wraith => (25.0, 110.0, 10.0, 3.0),
            EnemyKind::Knight => (120.0, 45.0, 18.0, 8.0),
            EnemyKind::Cultist => (12.0, 130.0, 6.0, 2.0),
            EnemyKind::Shieldbearer => (90.0, 40.0, 16.0, 6.0),
            EnemyKind::Necromancer => (60.0, 50.0, 10.0, 10.0),
            EnemyKind::Boss(_) => (1000.0, 80.0, 25.0, 0.0),
        };
        let r = kind.radius(false);
        let id = self.next_id;
        self.next_id += 1;
        Enemy {
            id,
            kind,
            pos,
            knock: Vec2::ZERO,
            hp: hp * hp_scale,
            max_hp: hp * hp_scale,
            speed: speed * gen_range(0.9, 1.1) * self.diff.at(self.time).speed,
            dmg: dmg * dmg_scale,
            r,
            xp,
            flash: 0.0,
            burn: 0.0,
            burn_dps: 0.0,
            burn_acc: 0.0,
            burn_src: Src { hero: 0, weapon: WeaponKind::Blade },
            slow: 0.0,
            last_hit: [-10.0; 12],
            timer: gen_range(0.5, 2.5),
            elite: false,
            boss: false,
            dead: false,
            phase: gen_range(0.0, TAU),
            alpha: 1.0,
            fuse: -1.0,
            net_vel: Vec2::ZERO,
        }
    }

    pub(crate) fn spawn_ring_dist(&self) -> f32 {
        (self.screen.length() * 0.5 + 60.0).max(500.0)
    }

    /// Position, facing and moving flag of a random living hero.
    pub(crate) fn random_hero_pos(&self) -> (Vec2, Vec2, bool) {
        let alive: Vec<&Hero> = self.heroes.iter().filter(|h| h.alive()).collect();
        if alive.is_empty() {
            return (self.center(), vec2(1.0, 0.0), false);
        }
        let h = alive[gen_range(0, alive.len())];
        (h.p.pos, h.p.facing, h.p.moving)
    }

    fn pick_kind(&self) -> EnemyKind {
        let m = self.time / 60.0;
        let mut table: Vec<(EnemyKind, f32)> = vec![(EnemyKind::Hollow, (10.0 - m * 0.5).max(3.0))];
        for (kind, from, weight) in [
            (EnemyKind::Ghoul, 1.0, 5.0),
            (EnemyKind::Moth, 2.0, 3.0),
            (EnemyKind::Archer, 3.0, 2.5),
            (EnemyKind::Bloat, 5.0, 2.0),
            (EnemyKind::Wraith, 8.0, 4.0),
            (EnemyKind::Knight, 12.0, 3.0),
            (EnemyKind::Cultist, 6.0, 2.5),
            (EnemyKind::Shieldbearer, 10.0, 2.0),
            (EnemyKind::Necromancer, 14.0, 0.8),
        ] {
            if m >= from {
                table.push((kind, weight));
            }
        }
        let total: f32 = table.iter().map(|t| t.1).sum();
        let mut roll = gen_range(0.0, total);
        for (k, w) in &table {
            if roll < *w {
                return *k;
            }
            roll -= w;
        }
        EnemyKind::Hollow
    }

    fn spawn_at_ring(&mut self, kind: EnemyKind) {
        let (at, facing, moving) = self.random_hero_pos();
        // Bias spawns slightly towards where that hero is heading.
        let mut dir = rand_dir();
        if moving && chance(0.4) {
            dir = (facing + rand_dir() * 0.6).normalize_or_zero();
        }
        let ring = self.spawn_ring_dist();
        let mut pos = at + dir * ring * gen_range(1.0, 1.15);
        // Near a wall, look for a direction that stays on the field.
        for _ in 0..8 {
            if inside_world(pos, 40.0) {
                break;
            }
            pos = at + rand_dir() * ring * gen_range(1.0, 1.15);
        }
        let e = self.make_enemy(kind, clamp_world(pos, 40.0));
        self.enemies.push(e);
    }

    /// How many more foes fit under `MAX_ENEMIES`.
    pub(crate) fn enemy_room(&self) -> usize {
        MAX_ENEMIES.saturating_sub(self.enemies.len())
    }

    pub(crate) fn update_spawns(&mut self, dt: f32) {
        let m = self.time / 60.0;
        let coop = 1.0 + 0.5 * (self.heroes.len().max(1) - 1) as f32;
        let mut target = (25.0 + m * 18.0).min(750.0) * self.diff.at(self.time).count * coop * self.spawn_pressure();
        if self.boss.is_some() {
            target *= 0.35;
        }
        let rate = (4.0 + m * 3.0) * self.diff.at(self.time).count * coop * self.spawn_pressure();
        self.spawn_acc += rate * dt;
        while self.spawn_acc >= 1.0 {
            self.spawn_acc -= 1.0;
            if (self.enemies.len() as f32) < target.min(MAX_ENEMIES as f32) {
                let k = self.pick_kind();
                self.spawn_at_ring(k);
            }
        }

        if self.time >= self.next_horde && self.boss.is_none() {
            self.next_horde = self.time + 60.0;
            let n = (((28.0 + m * 4.0) * self.diff.at(self.time).count) as usize).min(self.enemy_room());
            let kind = if m > 6.0 { EnemyKind::Ghoul } else { EnemyKind::Hollow };
            let r = self.spawn_ring_dist() * 0.85;
            let (at, _, _) = self.random_hero_pos();
            for i in 0..n {
                let a = i as f32 / n as f32 * TAU;
                let e = self.make_enemy(kind, clamp_world(at + Vec2::from_angle(a) * r, 40.0));
                self.enemies.push(e);
            }
            self.toast("A horde closes in...");
        }

        if self.time >= self.next_swarm {
            self.next_swarm = self.time + 50.0;
            let (at, _, _) = self.random_hero_pos();
            let base = clamp_world(at + rand_dir() * self.spawn_ring_dist(), 120.0);
            for _ in 0..(((14.0 + m) * self.diff.at(self.time).count) as usize).min(self.enemy_room()) {
                let e = self.make_enemy(EnemyKind::Moth, base + rand_dir() * gen_range(0.0, 60.0));
                self.enemies.push(e);
            }
        }

        if self.time >= self.next_elite {
            self.next_elite = self.time + 70.0;
            let kind = match m as u32 {
                0..=3 => EnemyKind::Hollow,
                4..=7 => EnemyKind::Bloat,
                8..=11 => EnemyKind::Wraith,
                _ => EnemyKind::Knight,
            };
            let (at, _, _) = self.random_hero_pos();
            let mut e = self.make_enemy(kind, clamp_world(at + rand_dir() * self.spawn_ring_dist(), 60.0));
            e.elite = true;
            e.hp *= 10.0;
            e.max_hp = e.hp;
            e.r = kind.radius(true);
            e.dmg *= 1.5;
            e.xp *= 10.0;
            self.enemies.push(e);
        }

        if self.time >= self.next_relic {
            self.next_relic = self.time + gen_range(30.0, 45.0);
            self.spawn_relic();
        }

        if self.time >= self.next_boss && self.boss.is_none() {
            self.next_boss = self.time + BOSS_INTERVAL;
            self.spawn_boss();
        }

        if self.time >= self.next_shrine {
            self.next_shrine = self.time + BOSS_INTERVAL;
            let pos = clamp_world(self.center() + rand_dir() * 380.0, 200.0);
            self.shrines.push(Shrine { pos, life: 90.0, kindle: 0.0, used: false });
            self.toast("An Ash Shrine stirs nearby. Rest to bank your souls.");
        }
    }

    /// Drops a relic somewhere near a hero: a blessing, an item, a bomb or food.
    fn spawn_relic(&mut self) {
        let (at, _, _) = self.random_hero_pos();
        let pos = clamp_world(at + rand_dir() * gen_range(260.0, 520.0), 150.0);
        let kind = match gen_range(0, 100) {
            0..=19 => PickKind::Buff(Buff::Fury),
            20..=37 => PickKind::Buff(Buff::Haste),
            38..=49 => PickKind::Buff(Buff::Aegis),
            50..=67 => PickKind::Relic,
            68..=81 => PickKind::Pyre,
            _ => PickKind::Bread,
        };
        let mut p = Pickup::new(pos, kind);
        p.life = 60.0;
        self.pickups.push(p);
    }

    fn spawn_boss(&mut self) {
        let order = [BossKind::Gravewarden, BossKind::Matriarch, BossKind::Hermit, BossKind::Sovereign];
        let n = self.bosses_spawned;
        // The last hour always belongs to the Sovereign; past dawn the lords take turns again.
        let finale = self.time >= VICTORY_TIME - 1.0 && !self.endless;
        let kind = if finale { BossKind::Sovereign } else { order[(n % 4) as usize] };
        let lap = if finale { 1.5 } else { (n / 4) as f32 };
        self.bosses_spawned += 1;
        let (hp, speed, dmg) = match kind {
            BossKind::Gravewarden => (3200.0, 85.0, 25.0),
            BossKind::Matriarch => (5200.0, 160.0, 20.0),
            BossKind::Hermit => (7800.0, 0.0, 22.0),
            BossKind::Sovereign => (11500.0, 100.0, 32.0),
        };
        let r = EnemyKind::Boss(kind).radius(false);
        // The fog ring must fit inside the walls.
        let center = clamp_world(self.center(), 620.0);
        let coop = 1.0 + 0.6 * (self.heroes.len().max(1) - 1) as f32;
        let mut e = self.make_enemy(EnemyKind::Boss(kind), center + rand_dir() * 360.0);
        e.boss = true;
        // Heroes grow far faster than foes, so a boss gains 17% health for every minute past
        // the first one at 5:00; the finale gets a little extra on top.
        let minutes = (self.time / 60.0 - 5.0).max(0.0);
        e.hp = hp * BOSS_GROWTH.powf(minutes) * if finale { 1.25 } else { 1.0 } * self.diff.boss_hp * coop;
        e.max_hp = e.hp;
        e.r = r;
        e.speed = speed * self.diff.at(self.time).speed;
        e.dmg = dmg * (1.0 + lap * 0.4) * self.diff.at(self.time).dmg;
        let id = e.id;
        self.enemies.push(e);
        self.boss_started = self.time;
        self.boss = Some(BossAi { id, kind, state: 0, t: 2.0, aim: vec2(1.0, 0.0), spin: 0.0, count: 0, shot: 0.0, phase2: false });
        self.boss_kind = Some(kind);
        self.boss_bar_lag = 1.0;
        self.arena = Some((center, 560.0));
        let title = if finale { "The Last Hour" } else { kind.title() };
        self.banner(kind.name(), title, Color::from_rgba(220, 200, 170, 255), 4.0);
        self.sfx("roar", 1.0);
        self.shake = 12.0;
        self.events.push(Event::BossStart);
    }
}
