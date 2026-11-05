//! Class skills: one active ability per class, on its own key, with a cooldown.

use super::*;

impl World {
    /// Asks for hero `h`'s skill; it is cast in the next update, once the enemy grid is fresh.
    pub fn use_skill(&mut self, h: usize) {
        if let Some(hero) = self.heroes.get_mut(h) {
            hero.skill_wanted = true;
        }
    }

    /// Casts every skill that was asked for. Runs on the host after the grid rebuild.
    pub(crate) fn cast_skills(&mut self) {
        for h in 0..self.heroes.len() {
            if std::mem::take(&mut self.heroes[h].skill_wanted) {
                self.cast_skill(h);
            }
        }
    }

    fn cast_skill(&mut self, h: usize) {
        let Some(hero) = self.heroes.get(h) else { return };
        if hero.downed || hero.skill_cd > 0.0 || self.pause != Pause::Running {
            return;
        }
        let (class, p, might) = (hero.class, hero.p.pos, hero.might());
        let (name, _, cd) = class.skill();
        self.heroes[h].skill_cd = cd * self.heroes[h].cooldown().max(0.5);
        // Skills grow with the vigil, like everything else.
        let power = might * (1.0 + 0.08 * self.level as f32);
        let src = Src { hero: h, weapon: class.weapon().for_field(self.heroes[h].field) };
        crate::log!("{} uses {}", self.heroes[h].name, name);
        let color = match class {
            Class::CinderAdept | Class::BloodKnight => Color::from_rgba(255, 120, 60, 255),
            Class::BellAcolyte => Color::from_rgba(255, 220, 140, 255),
            Class::LanternWarden => Color::from_rgba(150, 220, 255, 255),
            Class::Stormcaller => Color::from_rgba(170, 190, 255, 255),
            Class::Hexblade => Color::from_rgba(200, 90, 255, 255),
            _ => Color::from_rgba(220, 220, 240, 255),
        };
        self.fx.push(Fx { pos: p, life: 0.5, max_life: 0.5, kind: FxKind::Ring { r: 90.0, color } });
        match class {
            Class::Wanderer => {
                self.hit_circle(p, 300.0, 80.0 * power, 620.0, src, None);
                self.damage_bullets(p, 300.0, 1e9);
                for hero in self.heroes.iter_mut().filter(|o| o.alive() && o.p.pos.distance(p) < 300.0) {
                    hero.p.iframes = hero.p.iframes.max(2.0);
                }
                self.fx.push(Fx { pos: p, life: 0.6, max_life: 0.6, kind: FxKind::Ring { r: 300.0, color } });
                self.shake = self.shake.max(10.0);
                self.sfx_at("slam", 0.9, p);
            }
            Class::GraveHunter | Class::Arbalist => {
                let (n, kind, speed, dmg) = if class == Class::Arbalist { (48, PKind::Quarrel, 620.0, 36.0) } else { (36, PKind::Dagger, 720.0, 40.0) };
                for k in 0..n {
                    let dir = Vec2::from_angle(k as f32 / n as f32 * TAU);
                    self.projs.push(Proj {
                        pos: p + dir * 12.0,
                        vel: dir * speed,
                        dir,
                        r: 6.0,
                        dmg: dmg * power,
                        pierce: 6,
                        life: 1.3,
                        age: 0.0,
                        kind,
                        evo: true,
                        area: 1.0,
                        src,
                        hits: Vec::new(),
                        dead: false,
                    });
                }
                self.sfx_at(if class == Class::Arbalist { "arrow" } else { "knife" }, 1.0, p);
            }
            Class::CinderAdept => {
                // Three stars: on the nearest foes, or ahead of the hero where there are fewer.
                let mut spots: Vec<Vec2> = self.nearest_enemies(p, 700.0, 3).iter().map(|&i| self.enemies[i].pos).collect();
                let aim = self.heroes[h].aim;
                while spots.len() < 3 {
                    let k = spots.len() as f32;
                    spots.push(p + Vec2::from_angle(aim.to_angle() + (k - 1.0) * 0.6) * 220.0);
                }
                for at in spots {
                    self.explode(at, 220.0, 200.0 * power, true, src);
                    self.fx.push(Fx { pos: at, life: 0.3, max_life: 0.3, kind: FxKind::Lightning { from: at + vec2(-120.0, -600.0) } });
                    self.add_decal(at, 100.0, Color::from_rgba(15, 12, 10, 170), DecalKind::Scorch, 20.0);
                }
                self.shake = self.shake.max(16.0);
                self.sfx_at("explode", 1.0, p);
            }
            Class::BellAcolyte => {
                for hero in self.heroes.iter_mut().filter(|o| o.alive() && o.p.pos.distance(p) < 380.0) {
                    let max = hero.stats.max_hp;
                    hero.heal(max * 0.55);
                    hero.buffs[Buff::Aegis as usize] = hero.buffs[Buff::Aegis as usize].max(3.5);
                }
                self.fx.push(Fx { pos: p, life: 0.8, max_life: 0.8, kind: FxKind::Ring { r: 380.0, color } });
                self.sfx_at("bell", 1.0, p);
            }
            Class::Reaper => {
                let killed = self.kills;
                let hits = self.hit_circle(p, 340.0, 110.0 * power, 340.0, src, None);
                let reaped = hits as f32 * 0.8 + (self.kills - killed) as f32 * 1.5;
                self.heroes[h].heal(reaped.min(100.0));
                self.fx.push(Fx { pos: p, life: 0.45, max_life: 0.45, kind: FxKind::Sweep { radius: 340.0, evo: true, owner: h as u8 } });
                self.sfx_at("slash", 1.0, p);
            }
            Class::Hexblade => {
                self.pools.push(FirePool { pos: p, r: 210.0, life: 8.0, max_life: 8.0, dps: 45.0 * power, tick: 0.1, src, kind: PoolKind::Hex });
                self.sfx_at("fire", 1.0, p);
            }
            Class::BloodKnight => {
                let hero = &mut self.heroes[h];
                hero.buffs[Buff::Fury as usize] = hero.buffs[Buff::Fury as usize].max(9.0);
                let max = hero.stats.max_hp;
                hero.heal(max * 0.4);
                self.blood_spray(p, rand_dir(), Color::from_rgba(150, 10, 12, 255), 16.0, 20);
                self.sfx_at("roar", 0.6, p);
            }
            Class::LanternWarden => {
                let r = 380.0;
                self.hit_circle(p, r, 60.0 * power, 120.0, src, None);
                for e in self.enemies.iter_mut().filter(|e| !e.dead && e.pos.distance(p) < r) {
                    e.slow = e.slow.max(if e.boss { 1.0 } else { 3.0 });
                    if !e.boss {
                        e.burn = e.burn.max(3.0);
                        e.burn_dps = e.burn_dps.max(20.0 * power);
                        e.burn_src = src;
                    }
                }
                for hero in self.heroes.iter_mut().filter(|o| o.alive() && o.p.pos.distance(p) < r) {
                    let max = hero.stats.max_hp;
                    hero.heal(max * 0.3);
                }
                self.fx.push(Fx { pos: p, life: 0.8, max_life: 0.8, kind: FxKind::Explosion { r: 140.0, color: Color::from_rgba(150, 220, 255, 255) } });
                self.fx.push(Fx { pos: p, life: 0.8, max_life: 0.8, kind: FxKind::Ring { r, color } });
                self.sfx_at("bell", 0.9, p);
            }
            Class::Stormcaller => {
                // On the strongest foes in reach, the lords first.
                let mut near = self.nearest_enemies(p, 650.0, 40);
                near.sort_by(|a, b| self.enemies[*b].hp.total_cmp(&self.enemies[*a].hp));
                for k in 0..8 {
                    let at = match near.get(k % near.len().max(1)) {
                        Some(&i) => self.enemies[i].pos,
                        None => p + Vec2::from_angle(k as f32 / 8.0 * TAU) * 220.0,
                    };
                    self.hit_circle(at, 75.0, 150.0 * power, 160.0, src, None);
                    self.fx.push(Fx { pos: at, life: 0.35, max_life: 0.35, kind: FxKind::Lightning { from: at + vec2(gen_range(-80.0, 80.0), -650.0) } });
                    self.add_decal(at, 30.0, Color::from_rgba(15, 15, 22, 160), DecalKind::Scorch, 15.0);
                }
                self.shake = self.shake.max(12.0);
                self.sfx_at("zap", 1.0, p);
            }
            Class::BucketKnight => {
                self.hit_circle(p, 340.0, 50.0 * power, 0.0, src, None);
                for e in self.enemies.iter_mut().filter(|e| !e.boss && e.pos.distance(p) < 340.0) {
                    e.slow = e.slow.max(4.5);
                    e.knock += (e.pos - p).normalize_or_zero() * 320.0;
                }
                for hero in self.heroes.iter_mut().filter(|o| o.alive() && o.p.pos.distance(p) < 340.0) {
                    hero.buffs[Buff::Aegis as usize] = hero.buffs[Buff::Aegis as usize].max(2.0);
                }
                self.fx.push(Fx { pos: p, life: 0.6, max_life: 0.6, kind: FxKind::Ring { r: 340.0, color } });
                self.shake = self.shake.max(8.0);
                self.sfx_at("clang", 1.0, p);
            }
        }
    }
}
