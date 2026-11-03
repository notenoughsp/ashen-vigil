//! Boss behaviour. Every attack is telegraphed so it can be read and rolled through.

use super::*;

const RED: Color = Color::new(0.9, 0.15, 0.1, 1.0);

impl World {
    fn bullet(&mut self, pos: Vec2, ang: f32, speed: f32, dmg: f32, r: f32) {
        self.enemy_bullet(pos, Vec2::from_angle(ang) * speed, r, dmg, 0.0, 6.0);
    }

    /// A ring of bullets, optionally with a gap the player can slip through.
    fn ring(&mut self, pos: Vec2, n: u32, speed: f32, dmg: f32, offset: f32, gap: Option<(f32, f32)>) {
        for k in 0..n {
            let a = offset + k as f32 / n as f32 * TAU;
            if let Some((g, w)) = gap {
                let mut diff = (a - g).rem_euclid(TAU);
                if diff > std::f32::consts::PI {
                    diff -= TAU;
                }
                if diff.abs() < w * 0.5 {
                    continue;
                }
            }
            self.bullet(pos, a, speed, dmg, 7.0);
        }
    }

    fn tele_line(&mut self, a: Vec2, b: Vec2, width: f32, dur: f32) {
        self.telegraphs.push(Telegraph { a, b, width, life: dur, max_life: dur, line: true });
    }

    fn tele_circle(&mut self, c: Vec2, r: f32, dur: f32) {
        self.telegraphs.push(Telegraph { a: c, b: c, width: r, life: dur, max_life: dur, line: false });
    }

    fn pillar(&mut self, pos: Vec2, r: f32, delay: f32, dmg: f32) {
        self.pillars.push(Pillar { pos, r, delay, max_delay: delay, dmg, after: 0.35 });
    }

    fn slam(&mut self, pos: Vec2, r: f32, dmg: f32) {
        self.hurt_area(pos, r + 8.0, dmg);
        self.waves.push(Shockwave { pos, r: r * 0.5, max_r: 650.0, speed: 380.0, dmg: dmg * 0.7, hit: [false; MAX_HEROES] });
        self.fx.push(Fx { pos, life: 0.4, max_life: 0.4, kind: FxKind::Explosion { r, color: Color::from_rgba(200, 170, 130, 255) } });
        for _ in 0..30 {
            self.particle(
                pos + Vec2::from_angle(gen_range(0.0, TAU)) * gen_range(0.0, r),
                Vec2::from_angle(gen_range(0.0, TAU)) * gen_range(40.0, 200.0),
                0.8,
                4.0,
                Color::from_rgba(110, 100, 90, 255),
                false,
            );
        }
        self.shake = self.shake.max(14.0);
        self.sfx_at("slam", 0.9, pos);
    }

    fn summon(&mut self, kind: EnemyKind, around: Vec2, n: u32, r: f32) {
        // A boss always gets at least a few adds, even on a crowded field.
        let n = n.min(self.enemy_room().max(4) as u32);
        for k in 0..n {
            let a = k as f32 / n as f32 * TAU;
            let pos = around + Vec2::from_angle(a) * r;
            let mut e = self.make_enemy(kind, pos);
            e.xp *= 0.5;
            self.enemies.push(e);
        }
    }

    pub(crate) fn update_boss(&mut self, dt: f32) {
        // A fight that drags on lifts the fog, so nobody is trapped with a lord they can't fell.
        if self.arena.is_some() && self.boss.is_some() && self.time - self.boss_started > BOSS_FOG_TIME {
            self.arena = None;
            self.toast("The fog thins. The lord still stands, but you can walk away.");
        }
        let Some(mut ai) = self.boss.take() else { return };
        let Some(bi) = self.enemies.iter().position(|e| e.id == ai.id && !e.dead) else {
            return;
        };
        let (mut pos, hp, max_hp, speed, r) = {
            let e = &self.enemies[bi];
            (e.pos, e.hp, e.max_hp, e.speed, e.r)
        };
        // Bosses hunt whoever is closest.
        let Some(target) = self.nearest_hero(pos) else {
            self.boss = Some(ai);
            return;
        };
        let pp = self.heroes[target].p.pos;
        if !ai.phase2 && hp < max_hp * 0.5 {
            ai.phase2 = true;
            self.banner("", "The foe grows desperate...", RED, 2.0);
            self.sfx("roar", 0.8);
            self.shake = 10.0;
        }
        let p2 = ai.phase2;
        let fast = if p2 { 1.35 } else { 1.0 };
        let to = (pp - pos).normalize_or_zero();
        let lap = 1.0 + (self.bosses_spawned.saturating_sub(1) / 4) as f32 * 0.4;
        ai.t -= dt * fast;
        let mut alpha = 1.0;

        match ai.kind {
            BossKind::Gravewarden => match ai.state {
                0 => {
                    pos += to * speed * fast * dt;
                    if ai.t <= 0.0 {
                        ai.state = 1;
                        ai.t = 0.8;
                        ai.aim = to;
                        self.tele_line(pos, pos + to * 560.0, r * 1.4, 0.8 / fast);
                    }
                }
                1 => {
                    if ai.t <= 0.0 {
                        ai.state = 2;
                        ai.t = 0.5;
                        self.sfx_at("roll", 1.0, pos);
                    }
                }
                2 => {
                    pos += ai.aim * 980.0 * dt;
                    if ai.t <= 0.0 {
                        ai.state = 3;
                        ai.t = 0.9;
                        self.tele_circle(pos, 160.0, 0.9 / fast);
                    }
                }
                3 => {
                    if ai.t <= 0.0 {
                        self.slam(pos, 160.0, 30.0 * lap);
                        if p2 {
                            let off = gen_range(0.0, TAU);
                            self.ring(pos, 18, 170.0, 14.0 * lap, off, None);
                        }
                        ai.state = 0;
                        ai.t = if p2 { 1.2 } else { 1.9 };
                    }
                }
                _ => ai.state = 0,
            },
            BossKind::Matriarch => match ai.state {
                0 => {
                    ai.spin += 0.6 * dt;
                    let target = pp + Vec2::from_angle(ai.spin) * 300.0;
                    pos += (target - pos).clamp_length_max(speed * dt * 1.5);
                    ai.shot -= dt;
                    if ai.shot <= 0.0 {
                        ai.shot = 0.9 / fast;
                        let base = to.y.atan2(to.x);
                        for k in -1..=1 {
                            self.bullet(pos, base + k as f32 * 0.22, 210.0, 14.0 * lap, 7.0);
                        }
                        self.sfx_quiet("bullet");
                    }
                    if ai.t <= 0.0 {
                        ai.state = 1;
                        ai.t = 3.2;
                    }
                }
                1 => {
                    ai.spin += if p2 { 3.4 } else { 2.6 } * dt;
                    ai.shot -= dt;
                    if ai.shot <= 0.0 {
                        ai.shot = 0.075;
                        let arms = if p2 { 4 } else { 2 };
                        for k in 0..arms {
                            let a = ai.spin + k as f32 / arms as f32 * TAU;
                            self.bullet(pos, a, 165.0, 12.0 * lap, 6.0);
                        }
                        self.sfx_quiet("bullet");
                    }
                    if ai.t <= 0.0 {
                        ai.state = 2;
                        ai.t = 0.6;
                        let n = if p2 { 16 } else { 10 };
                        self.summon(EnemyKind::Moth, pos, n, 60.0);
                    }
                }
                2 => {
                    if ai.t <= 0.0 {
                        ai.state = 3;
                        ai.t = 0.7;
                        ai.aim = to;
                        self.tele_line(pos, pos + to * 620.0, r * 1.2, 0.7 / fast);
                    }
                }
                3 => {
                    if ai.t <= 0.0 {
                        ai.state = 4;
                        ai.t = 0.45;
                        self.sfx_at("roll", 1.0, pos);
                    }
                }
                4 => {
                    pos += ai.aim * 1050.0 * dt;
                    if ai.t <= 0.0 {
                        ai.state = 0;
                        ai.t = 3.0;
                    }
                }
                _ => ai.state = 0,
            },
            BossKind::Hermit => match ai.state {
                0 => {
                    alpha = (ai.t / 0.5).clamp(0.0, 1.0);
                    if ai.t <= 0.0 {
                        pos = pp + Vec2::from_angle(gen_range(0.0, TAU)) * 260.0;
                        ai.state = 1;
                        ai.t = 0.5;
                    }
                }
                1 => {
                    alpha = 1.0 - (ai.t / 0.5).clamp(0.0, 1.0);
                    if ai.t <= 0.0 {
                        ai.state = 2;
                        ai.t = 0.0;
                        ai.count = 0;
                    }
                }
                2 => {
                    if ai.t <= 0.0 {
                        let gap = to.y.atan2(to.x) + gen_range(-1.2, 1.2);
                        let off = gen_range(0.0, TAU);
                        self.ring(pos, 30, 150.0, 16.0 * lap, off, Some((gap, 0.75)));
                        self.sfx_at("bell", 0.6, pos);
                        self.fx.push(Fx { pos, life: 0.5, max_life: 0.5, kind: FxKind::Ring { r: 90.0, color: Color::from_rgba(240, 210, 120, 255) } });
                        ai.count += 1;
                        ai.t = 0.5;
                        if ai.count >= if p2 { 4 } else { 3 } {
                            ai.state = 3;
                            ai.t = 0.8;
                        }
                    }
                }
                3 => {
                    if ai.t <= 0.0 {
                        for k in 0..5 {
                            let a = to.y.atan2(to.x) + (k as f32 - 2.0) * 0.5;
                            self.enemy_bullet(pos, Vec2::from_angle(a) * 140.0, 9.0, 16.0 * lap, 1.4, 5.0);
                        }
                        self.sfx_at("zap", 0.5, pos);
                        ai.state = 4;
                        ai.t = 1.4;
                    }
                }
                4 => {
                    if ai.t <= 0.0 {
                        let n = if p2 { 12 } else { 7 };
                        self.pillar(pp, 60.0, 1.1, 28.0 * lap);
                        for _ in 1..n {
                            let at = pp + Vec2::from_angle(gen_range(0.0, TAU)) * gen_range(70.0, 230.0);
                            self.pillar(at, 55.0, gen_range(1.0, 1.5), 28.0 * lap);
                        }
                        ai.state = 0;
                        ai.t = 2.2;
                    }
                }
                _ => ai.state = 0,
            },
            BossKind::Sovereign => match ai.state {
                0 => {
                    pos += to * speed * fast * dt;
                    if ai.t <= 0.0 {
                        ai.state = 1;
                        ai.t = 0.5;
                        ai.aim = to;
                        self.tele_line(pos, pos + to * 520.0, r * 1.3, 0.5 / fast);
                    }
                }
                1 => {
                    if ai.t <= 0.0 {
                        ai.state = 2;
                        ai.t = 0.4;
                        self.sfx_at("roll", 1.0, pos);
                    }
                }
                2 => {
                    pos += ai.aim * 1100.0 * dt;
                    if chance(0.5) {
                        self.pillar(pos, 40.0, 0.6, 18.0 * lap);
                    }
                    if ai.t <= 0.0 {
                        ai.count += 1;
                        if ai.count < 3 {
                            ai.state = 1;
                            ai.t = 0.45;
                            ai.aim = (pp - pos).normalize_or_zero();
                            let a = ai.aim;
                            self.tele_line(pos, pos + a * 520.0, r * 1.3, 0.45 / fast);
                        } else {
                            ai.count = 0;
                            ai.state = 3;
                            ai.t = 0.8;
                            self.tele_circle(pos, 190.0, 0.8 / fast);
                        }
                    }
                }
                3 => {
                    if ai.t <= 0.0 {
                        self.slam(pos, 190.0, 34.0 * lap);
                        let off = gen_range(0.0, TAU);
                        self.ring(pos, if p2 { 32 } else { 22 }, 190.0, 16.0 * lap, off, None);
                        ai.state = 4;
                        ai.t = 1.0;
                    }
                }
                4 => {
                    if ai.t <= 0.0 {
                        // A burning cross centred on the hunted hero.
                        let n = if p2 { 6 } else { 4 };
                        for k in 0..=n {
                            for dir in [vec2(1.0, 0.0), vec2(-1.0, 0.0), vec2(0.0, 1.0), vec2(0.0, -1.0)] {
                                let at = pp + dir * k as f32 * 75.0;
                                self.pillar(at, 42.0, 0.9 + k as f32 * 0.08, 26.0 * lap);
                                if p2 {
                                    let diag = Vec2::from_angle(dir.y.atan2(dir.x) + TAU / 8.0);
                                    self.pillar(pp + diag * k as f32 * 75.0, 42.0, 1.3 + k as f32 * 0.08, 26.0 * lap);
                                }
                            }
                        }
                        ai.state = 5;
                        ai.t = 1.6;
                    }
                }
                5 => {
                    if ai.t <= 0.0 {
                        ai.state = 6;
                        ai.t = 2.4;
                        self.summon(EnemyKind::Knight, pos, if p2 { 6 } else { 4 }, 120.0);
                        self.sfx_at("roar", 0.7, pos);
                    }
                }
                6 => {
                    ai.spin += 2.2 * dt;
                    ai.shot -= dt;
                    if ai.shot <= 0.0 {
                        ai.shot = 0.1;
                        for k in 0..3 {
                            let a = ai.spin + k as f32 / 3.0 * TAU;
                            self.bullet(pos, a, 175.0, 14.0 * lap, 7.0);
                        }
                        self.sfx_quiet("bullet");
                    }
                    if ai.t <= 0.0 {
                        ai.state = 0;
                        ai.t = 1.2;
                    }
                }
                _ => ai.state = 0,
            },
        }

        // Bosses stay inside their fog ring.
        if let Some((c, ar)) = self.arena {
            let d = pos - c;
            if d.length() > ar - r {
                pos = c + d.normalize_or_zero() * (ar - r);
            }
        }
        let e = &mut self.enemies[bi];
        e.pos = pos;
        e.alpha = alpha;
        self.boss = Some(ai);
    }
}
