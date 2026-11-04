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
        let src = Src { hero: h, weapon: class.weapon() };
        crate::log!("{} uses {}", self.heroes[h].name, name);
        let color = match class {
            Class::CinderAdept | Class::BloodKnight => Color::from_rgba(255, 120, 60, 255),
            Class::BellAcolyte => Color::from_rgba(255, 220, 140, 255),
            Class::Hexblade => Color::from_rgba(200, 90, 255, 255),
            _ => Color::from_rgba(220, 220, 240, 255),
        };
        self.fx.push(Fx { pos: p, life: 0.5, max_life: 0.5, kind: FxKind::Ring { r: 90.0, color } });
        match class {
            Class::Wanderer => {
                self.hit_circle(p, 240.0, 40.0 * power, 520.0, src, None);
                self.damage_bullets(p, 240.0, 1e9);
                for hero in self.heroes.iter_mut().filter(|o| o.alive() && o.p.pos.distance(p) < 240.0) {
                    hero.p.iframes = hero.p.iframes.max(1.5);
                }
                self.fx.push(Fx { pos: p, life: 0.6, max_life: 0.6, kind: FxKind::Ring { r: 240.0, color } });
                self.shake = self.shake.max(10.0);
                self.sfx_at("slam", 0.9, p);
            }
            Class::GraveHunter | Class::Arbalist => {
                let (n, kind, speed, dmg) = if class == Class::Arbalist { (36, PKind::Quarrel, 600.0, 22.0) } else { (24, PKind::Dagger, 700.0, 25.0) };
                for k in 0..n {
                    let dir = Vec2::from_angle(k as f32 / n as f32 * TAU);
                    self.projs.push(Proj {
                        pos: p + dir * 12.0,
                        vel: dir * speed,
                        dir,
                        r: 6.0,
                        dmg: dmg * power,
                        pierce: 4,
                        life: 1.2,
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
                let at = self.nearest_enemies(p, 650.0, 1).first().map(|&i| self.enemies[i].pos).unwrap_or(p + self.heroes[h].aim * 200.0);
                self.explode(at, 180.0, 120.0 * power, true, src);
                self.fx.push(Fx { pos: at, life: 0.3, max_life: 0.3, kind: FxKind::Lightning { from: at + vec2(-120.0, -600.0) } });
                self.add_decal(at, 90.0, Color::from_rgba(15, 12, 10, 170), DecalKind::Scorch, 20.0);
                self.shake = self.shake.max(14.0);
                self.sfx_at("explode", 1.0, at);
            }
            Class::BellAcolyte => {
                for hero in self.heroes.iter_mut().filter(|o| o.alive() && o.p.pos.distance(p) < 320.0) {
                    let max = hero.stats.max_hp;
                    hero.heal(max * 0.35);
                    hero.buffs[Buff::Aegis as usize] = hero.buffs[Buff::Aegis as usize].max(2.0);
                }
                self.fx.push(Fx { pos: p, life: 0.8, max_life: 0.8, kind: FxKind::Ring { r: 320.0, color } });
                self.sfx_at("bell", 1.0, p);
            }
            Class::Reaper => {
                let killed = self.kills;
                let hits = self.hit_circle(p, 290.0, 60.0 * power, 300.0, src, None);
                let reaped = hits as f32 * 0.6 + (self.kills - killed) as f32;
                self.heroes[h].heal(reaped.min(60.0));
                self.fx.push(Fx { pos: p, life: 0.45, max_life: 0.45, kind: FxKind::Sweep { radius: 290.0, evo: true, owner: h as u8 } });
                self.sfx_at("slash", 1.0, p);
            }
            Class::Hexblade => {
                self.pools.push(FirePool { pos: p, r: 150.0, life: 6.0, max_life: 6.0, dps: 25.0 * power, tick: 0.1, src, hex: true });
                self.sfx_at("fire", 1.0, p);
            }
            Class::BloodKnight => {
                let hero = &mut self.heroes[h];
                hero.buffs[Buff::Fury as usize] = hero.buffs[Buff::Fury as usize].max(6.0);
                let max = hero.stats.max_hp;
                hero.heal(max * 0.25);
                self.blood_spray(p, rand_dir(), Color::from_rgba(150, 10, 12, 255), 16.0, 20);
                self.sfx_at("roar", 0.6, p);
            }
            Class::BucketKnight => {
                for e in self.enemies.iter_mut().filter(|e| !e.boss && e.pos.distance(p) < 280.0) {
                    e.slow = e.slow.max(3.0);
                    e.knock += (e.pos - p).normalize_or_zero() * 260.0;
                }
                for hero in self.heroes.iter_mut().filter(|o| o.alive() && o.p.pos.distance(p) < 280.0) {
                    hero.buffs[Buff::Aegis as usize] = hero.buffs[Buff::Aegis as usize].max(1.0);
                }
                self.fx.push(Fx { pos: p, life: 0.6, max_life: 0.6, kind: FxKind::Ring { r: 280.0, color } });
                self.shake = self.shake.max(8.0);
                self.sfx_at("clang", 1.0, p);
            }
        }
    }
}
