//! What each weapon does when it fires, and the projectiles they leave behind.

use super::*;
use std::f32::consts::PI;

/// How far the weapons look for a target.
const AIM_RANGE: f32 = 650.0;
/// How far the soulbolt looks for a foe to seek.
const HOMING_RANGE: f32 = 450.0;

impl World {
    pub(crate) fn update_weapons(&mut self, dt: f32) {
        for h in 0..self.heroes.len() {
            if self.heroes[h].downed || self.heroes[h].possessing {
                continue;
            }
            self.update_aim(h);
            // The faster the weapons fire and the more of them there are, the softer each
            // shot sounds, so a late arsenal is not a wall of noise.
            let hero = &self.heroes[h];
            let parts: usize = hero.weapons.iter().map(|w| w.kind_count()).sum();
            let pace = hero.cooldown().clamp(0.3, 1.0);
            self.fire_vol = (pace * (4.0 / parts.max(4) as f32).sqrt()).clamp(0.25, 1.0);
            for wi in 0..self.heroes[h].weapons.len() {
                for part in self.heroes[h].weapons[wi].parts() {
                    self.update_part(h, wi, part, dt);
                }
            }
            self.fire_vol = 1.0;
        }
    }

    /// Weapons point at the nearest enemy in reach, or where the hero walks when none is near.
    fn update_aim(&mut self, h: usize) {
        let (pos, facing) = (self.heroes[h].p.pos, self.heroes[h].p.facing);
        let target = self.nearest_enemies(pos, AIM_RANGE, 1).first().map(|&i| self.enemies[i].pos);
        self.heroes[h].aim = target.map(|t| (t - pos).normalize_or_zero()).filter(|d| *d != Vec2::ZERO).unwrap_or(facing);
    }

    /// Runs one part of a weapon slot; a fused weapon has two.
    fn update_part(&mut self, h: usize, wi: usize, part: Part, dt: f32) {
        let kind = part.kind;
        let evo = part.evo;
        let hero = &self.heroes[h];
        let level = hero.weapons[wi].level;
        let s = kind.stats(level, evo);
        let dmg = s.dmg * hero.might() * part.mult * hero.weapon_bonus(&hero.weapons[wi]);
        let cd = (s.cd * hero.cooldown()).max(0.05);
        let amount = s.amount + hero.stats.amount;
        let area = s.area * hero.stats.area;
        let p = hero.p.pos;
        let face = hero.aim;
        let src = Src { hero: h, weapon: part.own };

        let (mut timer, mut spin) = {
            let w = &self.heroes[h].weapons[wi];
            (w.timers.get(part.slot).copied().unwrap_or(0.0), w.spins.get(part.slot).copied().unwrap_or(0.0))
        };
        match kind {
            WeaponKind::Lanterns => {
                spin += s.speed * dt;
                self.store_part(h, wi, part.slot, timer, spin);
                for (lp, r) in self.heroes[h].lantern_positions() {
                    self.hit_circle(lp, r, dmg, 120.0, src, Some(cd));
                }
                return;
            }
            WeaponKind::Halo => {
                // The ring breathes out and back; whatever its edge passes is seared.
                spin += s.speed * dt;
                self.store_part(h, wi, part.slot, timer, spin);
                let max = 135.0 * area;
                let r = 40.0 + (max - 40.0) * (0.5 - 0.5 * spin.cos());
                let band = 18.0 * area.sqrt();
                let mut q = std::mem::take(&mut self.qbuf);
                self.grid.query(p, r + band, &mut q);
                let mut hit = Vec::new();
                for &i in &q {
                    let e = &self.enemies[i];
                    let d = e.pos.distance(p);
                    if !e.dead && (d - r).abs() < band + e.r && self.time - e.last_hit[kind.idx()] >= cd * 0.5 {
                        hit.push(i);
                    }
                }
                self.qbuf = q;
                for i in hit {
                    let k = (self.enemies[i].pos - p).normalize_or_zero() * 90.0;
                    self.hit(i, dmg, k, src, true);
                }
                self.damage_bullets_where(|b, pad| (b.distance(p) - r).abs() < band + pad, dmg);
                return;
            }
            WeaponKind::Censer => {
                // Censers swing in a figure of eight and leave a choking smoke behind.
                spin += s.speed * dt;
                timer -= dt;
                let smoke = timer <= 0.0;
                if smoke {
                    timer = 0.45;
                }
                self.store_part(h, wi, part.slot, timer, spin);
                for (cp, r) in self.heroes[h].censer_positions() {
                    self.hit_circle(cp, r, dmg, 140.0, src, Some(cd));
                    if smoke {
                        let life = if evo { 2.2 } else { 1.4 };
                        self.pools.push(FirePool { pos: cp, r: r * 1.5, life, max_life: life, dps: dmg * 0.9, tick: 0.2, src, kind: PoolKind::Smoke });
                    }
                }
                return;
            }
            WeaponKind::Heart => {
                // Two beats close together, the second the harder; the spin times the second.
                timer -= dt;
                let first = timer <= 0.0;
                if first {
                    timer = cd;
                    spin = 0.18;
                }
                let r = 80.0 * area;
                let second = spin > 0.0 && {
                    spin -= dt;
                    spin <= 0.0
                };
                self.store_part(h, wi, part.slot, timer, spin.max(0.0));
                if first {
                    self.hit_circle(p, r * 0.8, dmg * 0.6, 70.0, src, None);
                }
                if second {
                    let n = self.hit_circle(p, r, dmg * 1.2, 130.0, src, None);
                    self.fx.push(Fx { pos: p, life: 0.3, max_life: 0.3, kind: FxKind::Ring { r, color: Color::from_rgba(220, 40, 60, 255) } });
                    if n > 0 {
                        self.sfx_gap_at("hit", 0.2, 0.4, p);
                    }
                }
                return;
            }
            WeaponKind::Eyes => {
                // The eyes circle; every so often each glares at the nearest foe it sees.
                spin += s.speed * dt;
                timer -= dt;
                let glare = timer <= 0.0;
                if glare {
                    timer = cd * 1.6;
                }
                self.store_part(h, wi, part.slot, timer, spin);
                if glare {
                    for (ep, _) in self.heroes[h].eye_positions() {
                        if let Some(&i) = self.nearest_enemies(ep, 260.0, 1).first() {
                            let to = self.enemies[i].pos;
                            self.hit(i, dmg * 1.4, (to - ep).normalize_or_zero() * 60.0, src, false);
                            self.fx.push(Fx { pos: ep, life: 0.18, max_life: 0.18, kind: FxKind::Gaze { to } });
                        }
                    }
                }
                return;
            }
            WeaponKind::Chime => {
                timer -= dt;
                let fire = timer <= 0.0;
                if fire {
                    timer = cd;
                }
                self.store_part(h, wi, part.slot, timer, spin);
                if fire {
                    let r = 75.0 * area;
                    let n = self.hit_circle(p, r, dmg, 60.0, src, None);
                    for e in self.enemies.iter_mut() {
                        if e.pos.distance(p) < r + e.r {
                            e.slow = if evo { 0.5 } else { 0.25 };
                        }
                    }
                    if n > 0 {
                        self.sfx_gap_at("chime", 0.12, 0.5, p);
                    }
                }
                return;
            }
            _ => {}
        }
        timer -= dt;
        if timer > 0.0 {
            self.store_part(h, wi, part.slot, timer, spin);
            return;
        }
        timer = cd;
        if matches!(kind, WeaponKind::Crossbow | WeaponKind::Glass | WeaponKind::Tendrils) {
            // Each volley turns a little so the gaps between quarrels move.
            spin += PI / amount as f32 * 0.5;
        }
        self.store_part(h, wi, part.slot, timer, spin);
        let base = face.to_angle();
        let proj = |pos: Vec2, dir: Vec2, speed: f32, r: f32, pierce: i32, life: f32, kind: PKind| Proj {
            pos,
            vel: dir * speed,
            dir,
            r,
            dmg,
            pierce,
            life,
            age: 0.0,
            kind,
            evo,
            area,
            src,
            hits: Vec::new(),
            dead: false,
        };
        match kind {
            WeaponKind::Blade => {
                let mut angles = vec![base];
                if amount >= 2 || evo {
                    angles.push(base + PI);
                }
                if amount >= 3 {
                    angles.push(base + PI * 0.5);
                }
                if amount >= 4 {
                    angles.push(base - PI * 0.5);
                }
                let radius = 110.0 * area;
                for a in angles {
                    self.sector_hit(p, a, 1.05, radius, dmg, 160.0, src, evo);
                    self.fx.push(Fx { pos: p, life: 0.22, max_life: 0.22, kind: FxKind::Slash { ang: a, radius, evo, owner: h as u8 } });
                }
                self.sfx_at("slash", 0.5, p);
            }
            WeaponKind::Scythe => {
                let radius = 125.0 * area;
                for k in 0..amount {
                    let r = radius * (1.0 - k as f32 * 0.25);
                    let killed_before = self.kills;
                    let hits = self.hit_circle(p, r, dmg, 260.0, src, None);
                    if evo {
                        let reaped = hits as f32 * 0.3 + (self.kills - killed_before) as f32 * 0.5;
                        self.heroes[h].heal(reaped.min(8.0));
                    }
                    self.fx.push(Fx { pos: p, life: 0.35, max_life: 0.35, kind: FxKind::Sweep { radius: r, evo, owner: h as u8 } });
                }
                self.sfx_at("slash", 0.6, p);
            }
            WeaponKind::Daggers => {
                // Each dagger flies at a foe; several on the same foe fan out a little.
                let dirs = self.shot_dirs(p, amount as usize, face);
                for (k, aim) in dirs.into_iter().enumerate() {
                    let off = if k >= self.enemies.len().max(1) { (k as f32 - (amount - 1) as f32 * 0.5) * 0.14 } else { 0.0 };
                    let jitter = if evo { gen_range(-0.08, 0.08) } else { 0.0 };
                    let dir = Vec2::from_angle(aim.to_angle() + off + jitter);
                    self.projs.push(proj(p + dir * 10.0, dir, s.speed, 6.0, s.pierce, 1.1, PKind::Dagger));
                }
                self.sfx_gap_at("knife", 0.3, 0.07, p);
            }
            WeaponKind::Crossbow => {
                let turn = spin;
                for k in 0..amount {
                    let dir = Vec2::from_angle(turn + k as f32 / amount as f32 * TAU);
                    self.projs.push(proj(p + dir * 12.0, dir, s.speed, 5.0, s.pierce, 1.2, PKind::Quarrel));
                }
                self.sfx_gap_at("arrow", 0.35, 0.08, p);
            }
            WeaponKind::Lance => {
                // The lances fly side by side at the nearest foe.
                let face = self.shot_dirs(p, 1, face)[0];
                for k in 0..amount {
                    let side = vec2(-face.y, face.x) * (k as f32 - (amount - 1) as f32 * 0.5) * 22.0;
                    let r = 9.0 * area;
                    self.projs.push(proj(p + side + face * 16.0, face, s.speed, r, s.pierce, 0.9, PKind::Lance));
                }
                self.sfx_gap_at("knife", 0.5, 0.1, p);
            }
            WeaponKind::Hex => {
                let r = 28.0 * area;
                let life = s.speed;
                self.pools.push(FirePool { pos: p + rand_dir() * 4.0, r, life, max_life: life, dps: dmg, tick: 0.1, src, kind: PoolKind::Hex });
            }
            WeaponKind::Orb => {
                let targets = self.nearest_enemies(p, 700.0, amount as usize);
                for k in 0..amount as usize {
                    let dir = match targets.get(k).or(targets.first()) {
                        Some(&i) => (self.enemies[i].pos - p).normalize_or_zero(),
                        None => Vec2::from_angle(base + k as f32 * 0.4),
                    };
                    let r = if evo { 18.0 } else { 9.0 } * area.sqrt();
                    // Well faster than a hero runs, fused or not, so an orb never seems to float along
                    // beside its thrower.
                    self.projs.push(proj(p, dir, s.speed, r, 0, 1.5, PKind::Orb));
                }
                self.sfx_gap_at("fire", 0.4, 0.1, p);
            }
            WeaponKind::Spear => {
                let candidates = self.nearest_enemies(p, 600.0, 40);
                if candidates.is_empty() {
                    self.store_part(h, wi, part.slot, 0.2, spin);
                    return;
                }
                for _ in 0..amount {
                    let i = candidates[gen_range(0, candidates.len())];
                    let mut at = self.enemies[i].pos;
                    self.hit_circle(at, 45.0 * area, dmg, 80.0, src, None);
                    self.fx.push(Fx { pos: at, life: 0.25, max_life: 0.25, kind: FxKind::Lightning { from: at + vec2(gen_range(-40.0, 40.0), -500.0) } });
                    self.add_decal(at, 18.0 * area, Color::from_rgba(15, 15, 22, 150), DecalKind::Scorch, 12.0);
                    if evo {
                        let mut visited = vec![self.enemies[i].id];
                        for _ in 0..5 {
                            let next = self.nearest_enemies(at, 180.0, 8).into_iter().find(|&j| !visited.contains(&self.enemies[j].id));
                            let Some(j) = next else { break };
                            visited.push(self.enemies[j].id);
                            let to = self.enemies[j].pos;
                            self.hit(j, dmg * 0.7, Vec2::ZERO, src, false);
                            self.fx.push(Fx { pos: to, life: 0.25, max_life: 0.25, kind: FxKind::Lightning { from: at } });
                            at = to;
                        }
                    }
                }
                self.shake = self.shake.max(2.0);
                self.sfx_gap_at("zap", 0.45, 0.1, p);
            }
            WeaponKind::Axe => {
                let dirs = self.shot_dirs(p, amount as usize, face);
                for dir in dirs {
                    self.projs.push(proj(p, dir, s.speed, 15.0 * area, 999, 3.0, PKind::Axe));
                }
                self.sfx_gap_at("slash", 0.3, 0.1, p);
            }
            WeaponKind::Bolt => {
                let dirs = self.shot_dirs(p, amount as usize, face);
                for aim in dirs {
                    let dir = Vec2::from_angle(aim.to_angle() + gen_range(-0.2, 0.2));
                    self.projs.push(proj(p, dir, s.speed, 6.0, 0, 3.0, PKind::Bolt));
                }
                self.sfx_gap_at("knife", 0.2, 0.1, p);
            }
            WeaponKind::Rosary => {
                // The beads lash ahead (and behind, and to the sides with more), and each lash
                // leaps on to the next foe beyond its tip.
                let mut angles = vec![base];
                if amount >= 2 || evo {
                    angles.push(base + PI);
                }
                if amount >= 3 {
                    angles.push(base + PI * 0.5);
                }
                if amount >= 4 {
                    angles.push(base - PI * 0.5);
                }
                let radius = 155.0 * area;
                for a in angles {
                    self.sector_hit(p, a, 0.42, radius, dmg, 120.0, src, false);
                    let tip = p + Vec2::from_angle(a) * radius;
                    self.fx.push(Fx { pos: p, life: 0.22, max_life: 0.22, kind: FxKind::Whip { to: tip } });
                    let leaps = if evo { 3 } else { 1 };
                    let mut from = tip;
                    let mut struck: Vec<u32> = Vec::new();
                    for _ in 0..leaps {
                        let next = self.nearest_enemies(from, 170.0, 6).into_iter().find(|&j| !struck.contains(&self.enemies[j].id));
                        let Some(j) = next else { break };
                        struck.push(self.enemies[j].id);
                        let to = self.enemies[j].pos;
                        self.hit(j, dmg * 0.6, (to - from).normalize_or_zero() * 80.0, src, false);
                        self.fx.push(Fx { pos: from, life: 0.2, max_life: 0.2, kind: FxKind::Whip { to } });
                        from = to;
                    }
                }
                self.sfx_at("slash", 0.45, p);
            }
            WeaponKind::Nails => {
                let dirs = self.shot_dirs(p, amount as usize, face);
                for (k, aim) in dirs.into_iter().enumerate() {
                    let spread = (k as f32 - (amount as f32 - 1.0) * 0.5) * 0.05 + gen_range(-0.04, 0.04);
                    let dir = Vec2::from_angle(aim.to_angle() + spread);
                    self.projs.push(proj(p + dir * 10.0, dir, s.speed, 4.5, s.pierce, 0.9, PKind::Nail));
                }
                self.sfx_gap_at("knife", 0.25, 0.06, p);
            }
            WeaponKind::HolyWater => {
                let targets = self.nearest_enemies(p, 600.0, amount as usize);
                for k in 0..amount as usize {
                    let dir = match targets.get(k).or(targets.first()) {
                        Some(&i) => (self.enemies[i].pos - p).normalize_or_zero(),
                        None => Vec2::from_angle(base + k as f32 * 0.6),
                    };
                    self.projs.push(proj(p, dir, s.speed, 7.0, 0, 1.2, PKind::Flask));
                }
                self.sfx_gap_at("chime", 0.25, 0.1, p);
            }
            WeaponKind::Judgement => {
                let candidates = self.nearest_enemies(p, 600.0, 40);
                if candidates.is_empty() {
                    self.store_part(h, wi, part.slot, 0.2, spin);
                    return;
                }
                for _ in 0..amount {
                    let i = candidates[gen_range(0, candidates.len())];
                    let at = self.enemies[i].pos;
                    let r = 62.0 * area;
                    self.hit_circle(at, r, dmg * 1.3, 140.0, src, None);
                    self.fx.push(Fx { pos: at, life: 0.4, max_life: 0.4, kind: FxKind::Sword { r } });
                    self.add_decal(at, 22.0 * area, Color::from_rgba(20, 18, 24, 150), DecalKind::Scorch, 12.0);
                }
                self.shake = self.shake.max(3.0);
                self.sfx_gap_at("slam", 0.4, 0.12, p);
            }
            WeaponKind::Gargoyle => {
                // At the strongest foes in reach, not the nearest.
                let mut near = self.nearest_enemies(p, AIM_RANGE, 30);
                near.sort_by(|a, b| self.enemies[*b].hp.total_cmp(&self.enemies[*a].hp));
                for k in 0..amount as usize {
                    let dir = match near.get(k % near.len().max(1)) {
                        Some(&i) => (self.enemies[i].pos - p).normalize_or_zero(),
                        None => Vec2::from_angle(base + k as f32 * TAU / amount as f32),
                    };
                    let dir = if dir == Vec2::ZERO { face } else { dir };
                    self.projs.push(proj(p, dir, s.speed, 16.0 * area, 999, 3.0, PKind::Gargoyle));
                }
                self.sfx_gap_at("roar", 0.15, 0.3, p);
            }
            WeaponKind::Runes => {
                // The letters start circling the hero, then seek (see `update_projs`).
                for k in 0..amount {
                    let a = base + k as f32 / amount as f32 * TAU;
                    let at = p + Vec2::from_angle(a) * 30.0;
                    let tangent = Vec2::from_angle(a + PI * 0.5);
                    self.projs.push(proj(at, tangent, 140.0, 6.0, 0, 3.5, PKind::Rune));
                }
                self.sfx_gap_at("chime", 0.2, 0.15, p);
            }
            WeaponKind::Crozier => {
                let radius = 130.0 * area;
                for k in 0..amount {
                    let r = radius * (1.0 - k as f32 * 0.25);
                    self.hit_circle(p, r, dmg, 60.0, src, None);
                    // Hooked off their feet: a moment of near standstill.
                    let stun = if evo { 1.2 } else { 0.8 };
                    for e in self.enemies.iter_mut().filter(|e| !e.boss && !e.dead && e.pos.distance(p) < r + e.r) {
                        e.slow = e.slow.max(stun);
                    }
                    self.fx.push(Fx { pos: p, life: 0.35, max_life: 0.35, kind: FxKind::Crook { radius: r, owner: h as u8 } });
                }
                self.sfx_at("slash", 0.6, p);
            }
            WeaponKind::Candles => {
                // A candle set down where the hero stands; it spits flame at foes near it.
                let life = s.speed;
                self.pools.push(FirePool { pos: p + rand_dir() * 6.0, r: 10.0, life, max_life: life, dps: dmg, tick: 0.3, src, kind: PoolKind::Candle });
                self.sfx_gap_at("kindle", 0.3, 0.3, p);
            }
            WeaponKind::Organ => {
                // Pipes burst up in a line ahead, one after another.
                let aim = self.shot_dirs(p, 1, face)[0];
                let n = 4 + amount;
                let side = vec2(-aim.y, aim.x);
                for k in 0..n {
                    let at = p + aim * (55.0 + k as f32 * 52.0) * area.sqrt() + side * if k % 2 == 0 { 8.0 } else { -8.0 };
                    let r = 30.0 * area;
                    self.hit_circle(at, r, dmg, 110.0, src, None);
                    let life = 0.35 + k as f32 * 0.04;
                    self.fx.push(Fx { pos: at, life, max_life: life, kind: FxKind::Pipe { r } });
                }
                self.sfx_gap_at("bell", 0.3, 0.15, p);
            }
            WeaponKind::Glass => {
                let turn = spin;
                for k in 0..amount {
                    let dir = Vec2::from_angle(turn + k as f32 / amount as f32 * TAU);
                    self.projs.push(proj(p + dir * 12.0, dir, s.speed, 6.0, s.pierce, 1.1, PKind::Shard));
                }
                self.sfx_gap_at("hit", 0.35, 0.08, p);
            }
            WeaponKind::Cleaver => {
                // A heavy cleave ahead (and behind with more); what it cuts bleeds on.
                let mut angles = vec![base];
                if amount >= 2 || evo {
                    angles.push(base + PI);
                }
                if amount >= 3 {
                    angles.push(base + PI * 0.5);
                }
                if amount >= 4 {
                    angles.push(base - PI * 0.5);
                }
                let radius = 120.0 * area;
                for a in angles {
                    self.sector_hit(p, a, 0.95, radius, dmg * 1.1, 180.0, src, true);
                    self.fx.push(Fx { pos: p, life: 0.24, max_life: 0.24, kind: FxKind::Gash { ang: a, radius, owner: h as u8 } });
                }
                self.sfx_at("slash", 0.55, p);
            }
            WeaponKind::Teeth => {
                let dirs = self.shot_dirs(p, amount as usize, face);
                for (k, aim) in dirs.into_iter().enumerate() {
                    let spread = (k as f32 - (amount as f32 - 1.0) * 0.5) * 0.08;
                    let dir = Vec2::from_angle(aim.to_angle() + spread);
                    // Pierce one more: the bounce.
                    self.projs.push(proj(p + dir * 10.0, dir, s.speed, 5.0, s.pierce + 1, 1.0, PKind::Tooth));
                }
                self.sfx_gap_at("knife", 0.25, 0.07, p);
            }
            WeaponKind::Sac => {
                let targets = self.nearest_enemies(p, 600.0, amount as usize);
                for k in 0..amount as usize {
                    let dir = match targets.get(k).or(targets.first()) {
                        Some(&i) => (self.enemies[i].pos - p).normalize_or_zero(),
                        None => Vec2::from_angle(base + k as f32 * 0.5),
                    };
                    let r = if evo { 14.0 } else { 9.0 } * area.sqrt();
                    self.projs.push(proj(p, dir, s.speed, r, 0, 1.5, PKind::Sac));
                }
                self.sfx_gap_at("hit", 0.3, 0.1, p);
            }
            WeaponKind::Spikes => {
                let candidates = self.nearest_enemies(p, 600.0, 40);
                if candidates.is_empty() {
                    self.store_part(h, wi, part.slot, 0.2, spin);
                    return;
                }
                for _ in 0..amount {
                    let i = candidates[gen_range(0, candidates.len())];
                    let at = self.enemies[i].pos;
                    let r = 42.0 * area;
                    self.hit_circle(at, r, dmg, 40.0, src, None);
                    // Held fast on the bone a moment.
                    let hold = if evo { 1.1 } else { 0.7 };
                    for e in self.enemies.iter_mut().filter(|e| !e.boss && !e.dead && e.pos.distance(at) < r + e.r) {
                        e.slow = e.slow.max(hold);
                    }
                    self.fx.push(Fx { pos: at, life: 0.4, max_life: 0.4, kind: FxKind::Spike { r } });
                    self.add_decal(at, 16.0 * area, Color::from_rgba(80, 4, 10, 170), DecalKind::Blood, 14.0);
                }
                self.sfx_gap_at("slam", 0.3, 0.12, p);
            }
            WeaponKind::Jaw => {
                let dirs = self.shot_dirs(p, amount as usize, face);
                for dir in dirs {
                    self.projs.push(proj(p, dir, s.speed, 15.0 * area, 999, 3.0, PKind::Jaw));
                }
                self.sfx_gap_at("roar", 0.12, 0.3, p);
            }
            WeaponKind::Motes => {
                let dirs = self.shot_dirs(p, amount as usize, face);
                for aim in dirs {
                    let dir = Vec2::from_angle(aim.to_angle() + gen_range(-0.35, 0.35));
                    self.projs.push(proj(p, dir, s.speed, 6.0, 0, 3.0, PKind::Mote));
                }
                self.sfx_gap_at("hit", 0.15, 0.1, p);
            }
            WeaponKind::Ribcage => {
                let radius = 125.0 * area;
                for k in 0..amount {
                    let r = radius * (1.0 - k as f32 * 0.25);
                    self.hit_circle(p, r, dmg, 0.0, src, None);
                    // Snapping shut drags them in.
                    for e in self.enemies.iter_mut().filter(|e| !e.boss && !e.dead && e.pos.distance(p) < r + e.r) {
                        e.knock += (p - e.pos).normalize_or_zero() * 260.0;
                    }
                    self.fx.push(Fx { pos: p, life: 0.35, max_life: 0.35, kind: FxKind::Ribs { radius: r, owner: h as u8 } });
                }
                self.sfx_at("slash", 0.55, p);
            }
            WeaponKind::Steps => {
                let r = 26.0 * area;
                let life = s.speed;
                self.pools.push(FirePool { pos: p + rand_dir() * 4.0, r, life, max_life: life, dps: dmg, tick: 0.1, src, kind: PoolKind::Blood });
            }
            WeaponKind::Spine => {
                // The spine lashes straight at the nearest foe, through everything in line.
                let aim = self.shot_dirs(p, 1, face)[0];
                let len = 330.0 * area.sqrt() * if evo { 1.3 } else { 1.0 };
                let width = 20.0 * area.sqrt();
                for k in 0..amount {
                    let side = vec2(-aim.y, aim.x) * (k as f32 - (amount - 1) as f32 * 0.5) * 30.0;
                    let a = p + side;
                    let b = a + aim * len;
                    self.hit_line(a, b, width, dmg, 120.0, src);
                    self.fx.push(Fx { pos: a, life: 0.22, max_life: 0.22, kind: FxKind::Lash { to: b } });
                }
                self.sfx_gap_at("slash", 0.4, 0.1, p);
            }
            WeaponKind::Tendrils => {
                let turn = spin;
                for k in 0..amount {
                    let dir = Vec2::from_angle(turn + k as f32 / amount as f32 * TAU);
                    self.projs.push(proj(p + dir * 12.0, dir, s.speed, 6.0, s.pierce, 0.7, PKind::Tendril));
                }
                self.sfx_gap_at("hit", 0.25, 0.1, p);
            }
            WeaponKind::Lanterns | WeaponKind::Chime | WeaponKind::Halo | WeaponKind::Censer | WeaponKind::Heart | WeaponKind::Eyes => {}
            // A newer field's weapon always fights as its twin, which has been handled above.
            _ => {}
        }
    }

    fn store_part(&mut self, h: usize, wi: usize, slot: usize, timer: f32, spin: f32) {
        let w = &mut self.heroes[h].weapons[wi];
        if let (Some(t), Some(s)) = (w.timers.get_mut(slot), w.spins.get_mut(slot)) {
            *t = timer;
            *s = spin;
        }
    }

    /// Directions to the nearest foes in reach, one per shot; with fewer foes than shots
    /// they are shared, and with none the shots go where the hero faces.
    fn shot_dirs(&mut self, p: Vec2, n: usize, fallback: Vec2) -> Vec<Vec2> {
        let targets = self.nearest_enemies(p, AIM_RANGE, n.max(1));
        (0..n)
            .map(|k| match targets.get(k % targets.len().max(1)) {
                Some(&i) => (self.enemies[i].pos - p).normalize_or_zero(),
                None => fallback,
            })
            .map(|d| if d == Vec2::ZERO { fallback } else { d })
            .collect()
    }

    /// Hits enemies (and bullets) in a cone in front of `p`.
    #[allow(clippy::too_many_arguments)]
    fn sector_hit(&mut self, p: Vec2, a: f32, half: f32, radius: f32, dmg: f32, knock: f32, src: Src, burn: bool) {
        let in_cone = |q: Vec2, pad: f32| {
            let d = q - p;
            let dist = d.length();
            if dist > radius + pad {
                return false;
            }
            let mut diff = (d.to_angle() - a).rem_euclid(TAU);
            if diff > PI {
                diff -= TAU;
            }
            diff.abs() < half + pad / dist.max(1.0)
        };
        let mut q = std::mem::take(&mut self.qbuf);
        self.grid.query(p, radius, &mut q);
        for &i in &q {
            let e = &self.enemies[i];
            if e.dead || !in_cone(e.pos, e.r) {
                continue;
            }
            let k = (e.pos - p).normalize_or_zero() * knock;
            self.hit(i, dmg, k, src, false);
            if burn {
                let e = &mut self.enemies[i];
                e.burn = 3.0;
                e.burn_dps = dmg * 0.4;
                e.burn_src = src;
            }
        }
        self.qbuf = q;
        self.damage_bullets_where(in_cone, dmg);
    }

    /// Turns projectile `pi` toward the nearest foe it has not hit yet, keeping its speed.
    fn home_in(&mut self, pi: usize, turn: f32, dt: f32) {
        let p = self.projs[pi].pos;
        let near = self.nearest_enemies(p, HOMING_RANGE, 6);
        let pr = &self.projs[pi];
        let Some(t) = near.iter().map(|&i| &self.enemies[i]).find(|e| !pr.hits.contains(&e.id)).map(|e| e.pos) else { return };
        let pr = &mut self.projs[pi];
        let speed = pr.vel.length();
        if speed < 1.0 {
            return;
        }
        let desired = (t - pr.pos).normalize_or_zero() * speed;
        // `dir` stays the launch direction: the axe brakes along it before it turns back.
        pr.vel = pr.vel.lerp(desired, (turn * dt).min(1.0)).normalize_or_zero() * speed;
    }

    /// A swollen sac bursting: a spray of blood that hits all around, and a stain left behind.
    fn blood_burst(&mut self, c: Vec2, area: f32, dmg: f32, evo: bool, src: Src) {
        let r = 60.0 * area * if evo { 1.4 } else { 1.0 };
        self.hit_circle(c, r, dmg, 180.0, src, None);
        self.fx.push(Fx { pos: c, life: 0.35, max_life: 0.35, kind: FxKind::Explosion { r, color: Color::from_rgba(200, 20, 30, 255) } });
        self.blood_spray(c, rand_dir(), Color::from_rgba(150, 10, 14, 255), r * 0.4, 14);
        self.add_decal(c, r * 0.7, Color::from_rgba(90, 4, 10, 180), DecalKind::Blood, 25.0);
        self.sfx_gap_at("explode", 0.3, 0.08, c);
    }

    /// Hits every foe within `width` of the line from `a` to `b`, pushing them along it.
    fn hit_line(&mut self, a: Vec2, b: Vec2, width: f32, dmg: f32, knock: f32, src: Src) {
        let d = b - a;
        let len = d.length().max(1.0);
        let dir = d / len;
        let near = |q: Vec2, pad: f32| {
            let t = ((q - a).dot(dir)).clamp(0.0, len);
            (a + dir * t).distance(q) < width + pad
        };
        let mut q = std::mem::take(&mut self.qbuf);
        self.grid.query((a + b) * 0.5, len * 0.5 + width, &mut q);
        let hits: Vec<usize> = q.iter().copied().filter(|&i| !self.enemies[i].dead && near(self.enemies[i].pos, self.enemies[i].r)).collect();
        self.qbuf = q;
        for i in hits {
            self.hit(i, dmg, dir * knock, src, false);
        }
        self.damage_bullets_where(near, dmg);
    }

    /// Holy water bursting: a pool that sears and slows what wades in it.
    fn holy_pool(&mut self, c: Vec2, area: f32, dmg: f32, evo: bool, src: Src) {
        let life = if evo { 4.5 } else { 3.0 };
        let r = 52.0 * area * if evo { 1.3 } else { 1.0 };
        self.pools.push(FirePool { pos: c, r, life, max_life: life, dps: dmg * 1.1, tick: 0.0, src, kind: PoolKind::Holy });
        self.fx.push(Fx { pos: c, life: 0.3, max_life: 0.3, kind: FxKind::Ring { r, color: Color::from_rgba(150, 210, 255, 255) } });
        self.sfx_gap_at("hit", 0.3, 0.08, c);
    }

    pub(crate) fn update_projs(&mut self, dt: f32) {
        let mut splinters: Vec<Proj> = Vec::new();
        for pi in 0..self.projs.len() {
            let (kind, evo, src) = (self.projs[pi].kind, self.projs[pi].evo, self.projs[pi].src);
            {
                let pr = &mut self.projs[pi];
                pr.age += dt;
                pr.life -= dt;
            }
            // The axe flies out, slows down and comes back to its thrower.
            if kind == PKind::Tendril && self.projs[pi].age > 0.35 {
                // Snapping back the way it came.
                let pr = &mut self.projs[pi];
                if pr.vel.dot(pr.dir) > 0.0 {
                    pr.vel = -pr.vel;
                    pr.hits.clear();
                }
            }
            if kind == PKind::Mote {
                self.home_in(pi, 6.0, dt);
            }
            if matches!(kind, PKind::Axe | PKind::Gargoyle | PKind::Jaw) {
                let home = self.heroes.get(src.hero).map(|h| h.p.pos).unwrap_or(self.projs[pi].pos);
                let pr = &mut self.projs[pi];
                if pr.age < 0.55 {
                    let decel = pr.dir * 520.0 * 1.6 * dt;
                    pr.vel -= decel;
                } else {
                    let to = (home - pr.pos).normalize_or_zero();
                    pr.vel = pr.vel.lerp(to * 650.0, (6.0 * dt).min(1.0));
                    if pr.pos.distance(home) < 20.0 {
                        pr.dead = true;
                    }
                }
            }
            // Only the soulbolt and the runes seek; everything else flies where it was aimed.
            if kind == PKind::Bolt {
                self.home_in(pi, 5.0, dt);
            }
            if kind == PKind::Rune {
                let pr = &mut self.projs[pi];
                if pr.age < 0.45 {
                    // Circling the hero a moment before they go.
                    pr.vel = Vec2::from_angle(pr.vel.to_angle() + 5.5 * dt) * pr.vel.length();
                } else {
                    let speed = (pr.vel.length() + 1600.0 * dt).min(440.0);
                    pr.vel = pr.vel.normalize_or_zero() * speed;
                    self.home_in(pi, 9.0, dt);
                }
            }
            let pr = &mut self.projs[pi];
            pr.pos += pr.vel * dt;
            if !inside_world(pr.pos, -300.0) {
                pr.life = 0.0;
            }
            if pr.life <= 0.0 {
                pr.dead = true;
                let (c, a, d) = (pr.pos, pr.area, pr.dmg);
                if kind == PKind::Orb {
                    self.explode(c, 55.0 * a, d, evo, src);
                }
                if kind == PKind::Flask {
                    self.holy_pool(c, a, d, evo, src);
                }
                if kind == PKind::Sac {
                    self.blood_burst(c, a, d, evo, src);
                }
                continue;
            }
            if pr.dead {
                continue;
            }
            let (pos, r) = (pr.pos, pr.r);
            match kind {
                PKind::Bolt if chance(0.5) => self.particle(pos, Vec2::ZERO, 0.25, 2.5, Color::from_rgba(150, 230, 255, 255), true),
                PKind::Orb if chance(0.7) => self.particle(pos + rand_dir() * 4.0, rand_dir() * 20.0, 0.35, 3.0, Color::from_rgba(255, 140, 40, 255), true),
                PKind::Lance if chance(0.6) => self.particle(pos, rand_dir() * 15.0, 0.4, 2.5, Color::from_rgba(150, 10, 15, 255), false),
                _ => {}
            }
            // Any of our projectiles cuts through enemy bullets.
            if self.ebullets.iter().any(|b| b.pos.distance(pos) < r + b.r) {
                let dmg = self.projs[pi].dmg;
                self.damage_bullets(pos, r, dmg);
            }
            let mut q = std::mem::take(&mut self.qbuf);
            self.grid.query(pos, r, &mut q);
            for &i in &q {
                let e = &self.enemies[i];
                if e.dead || e.pos.distance(pos) > r + e.r {
                    continue;
                }
                let id = e.id;
                if matches!(kind, PKind::Axe | PKind::Gargoyle | PKind::Jaw) {
                    if self.time - e.last_hit[src.weapon.idx()] < 0.3 {
                        continue;
                    }
                } else if self.projs[pi].hits.contains(&id) {
                    continue;
                }
                self.projs[pi].hits.push(id);
                let pr = &self.projs[pi];
                let knock = pr.vel.normalize_or_zero() * 140.0;
                let dmg = pr.dmg;
                if kind == PKind::Orb {
                    let a = pr.area;
                    self.projs[pi].dead = true;
                    self.explode(pos, 55.0 * a, dmg, evo, src);
                    break;
                }
                if kind == PKind::Flask {
                    let a = pr.area;
                    self.projs[pi].dead = true;
                    self.holy_pool(pos, a, dmg, evo, src);
                    break;
                }
                if kind == PKind::Sac {
                    let a = pr.area;
                    self.projs[pi].dead = true;
                    self.blood_burst(pos, a, dmg, evo, src);
                    break;
                }
                if kind == PKind::Tooth && self.projs[pi].hits.len() == 1 {
                    // The first foe struck sends the tooth on to the next one near.
                    let next = self.nearest_enemies(pos, 220.0, 4).into_iter().find(|&j| self.enemies[j].id != id && !self.enemies[j].dead);
                    if let Some(j) = next {
                        let to = (self.enemies[j].pos - pos).normalize_or_zero();
                        let pr = &mut self.projs[pi];
                        let speed = pr.vel.length();
                        pr.vel = to * speed;
                    }
                }
                let pr = &self.projs[pi];
                if kind == PKind::Shard && pr.r > 4.5 {
                    // A big shard breaks in two on what it strikes.
                    let (d, a) = (pr.vel.normalize_or_zero(), pr.area);
                    for turn in [-0.7, 0.7] {
                        let dir = Vec2::from_angle(d.to_angle() + turn);
                        splinters.push(Proj {
                            pos,
                            vel: dir * 380.0,
                            dir,
                            r: 3.5,
                            dmg: dmg * 0.5,
                            pierce: 0,
                            life: 0.45,
                            age: 0.0,
                            kind: PKind::Shard,
                            evo,
                            area: a,
                            src,
                            hits: vec![id],
                            dead: false,
                        });
                    }
                }
                self.hit(i, dmg, knock, src, matches!(kind, PKind::Axe | PKind::Gargoyle | PKind::Jaw));
                if matches!(kind, PKind::Jaw) {
                    // A bite bleeds on a little.
                    let e = &mut self.enemies[i];
                    e.burn = e.burn.max(2.0);
                    e.burn_dps = e.burn_dps.max(dmg * 0.3);
                    e.burn_src = src;
                }
                if kind == PKind::Mote {
                    let gain = if evo { 0.5 } else { 0.25 };
                    if let Some(hero) = self.heroes.get_mut(src.hero) {
                        hero.heal(gain);
                    }
                }
                if kind == PKind::Lance {
                    // The blood lance feeds its wielder.
                    let gain = if evo { 1.0 } else { 0.3 };
                    if let Some(hero) = self.heroes.get_mut(src.hero) {
                        hero.heal(gain);
                    }
                }
                let pr = &mut self.projs[pi];
                pr.pierce -= 1;
                if pr.pierce < 0 {
                    pr.dead = true;
                    break;
                }
            }
            self.qbuf = q;
        }
        self.projs.retain(|p| !p.dead);
        self.projs.extend(splinters);
    }
}
