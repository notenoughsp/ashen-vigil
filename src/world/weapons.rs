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
            if self.heroes[h].downed {
                continue;
            }
            self.update_aim(h);
            for wi in 0..self.heroes[h].weapons.len() {
                for part in self.heroes[h].weapons[wi].parts() {
                    self.update_part(h, wi, part, dt);
                }
            }
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
        let src = Src { hero: h, weapon: kind };

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
        if kind == WeaponKind::Crossbow {
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
                self.pools.push(FirePool { pos: p + rand_dir() * 4.0, r, life, max_life: life, dps: dmg, tick: 0.1, src, hex: true });
            }
            WeaponKind::Orb => {
                let targets = self.nearest_enemies(p, 700.0, amount as usize);
                for k in 0..amount as usize {
                    let dir = match targets.get(k).or(targets.first()) {
                        Some(&i) => (self.enemies[i].pos - p).normalize_or_zero(),
                        None => Vec2::from_angle(base + k as f32 * 0.4),
                    };
                    let r = if evo { 18.0 } else { 9.0 } * area.sqrt();
                    self.projs.push(proj(p, dir, s.speed * if evo { 0.6 } else { 1.0 }, r, 0, 2.0, PKind::Orb));
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
            WeaponKind::Lanterns | WeaponKind::Chime => {}
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

    pub(crate) fn update_projs(&mut self, dt: f32) {
        for pi in 0..self.projs.len() {
            let (kind, evo, src) = (self.projs[pi].kind, self.projs[pi].evo, self.projs[pi].src);
            let home = self.heroes.get(src.hero).map(|h| h.p.pos).unwrap_or(self.projs[pi].pos);
            {
                let pr = &mut self.projs[pi];
                pr.age += dt;
                pr.life -= dt;
            }
            // The axe flies out, slows down and comes back to its thrower.
            if kind == PKind::Axe {
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
            // Only the soulbolt seeks; everything else flies where it was aimed.
            if kind == PKind::Bolt {
                self.home_in(pi, 5.0, dt);
            }
            let pr = &mut self.projs[pi];
            pr.pos += pr.vel * dt;
            if !inside_world(pr.pos, -300.0) {
                pr.life = 0.0;
            }
            if pr.life <= 0.0 {
                pr.dead = true;
                if kind == PKind::Orb {
                    let (c, a, d) = (pr.pos, pr.area, pr.dmg);
                    self.explode(c, 55.0 * a, d, evo, src);
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
                if kind == PKind::Axe {
                    if self.time - e.last_hit[WeaponKind::Axe.idx()] < 0.3 {
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
                self.hit(i, dmg, knock, src, kind == PKind::Axe);
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
    }
}
