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
        // A boss always gets at least a few adds, even on a crowded field, but never past a hard
        // ceiling: with many lords at once in an endless night the adds piled up without end.
        let ceiling = (MAX_ENEMIES + LORD_ADDS_OVER).saturating_sub(self.enemies.len());
        let n = n.min(self.enemy_room().max(4).min(ceiling) as u32);
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
        // The fog ring widens for every lord, champion and Sven in the fight, smoothly, and
        // never narrows again while it lasts.
        if let Some((c, r)) = self.arena.filter(|_| self.boss.is_some()) {
            let foes = 1 + self.more_bosses.len() + self.enemies.iter().filter(|e| !e.dead && matches!(e.kind, EnemyKind::Champion | EnemyKind::Rival)).count();
            let want = ARENA_RADIUS + ARENA_GROWTH * (foes - 1) as f32;
            if want > r {
                let grown = (r + (want - r) * (2.0 * dt).min(1.0)).min(want);
                // Still inside the walls, and never narrower than it was.
                let room = field_half() - c.x.abs().max(c.y.abs()) - 40.0;
                self.arena = Some((c, grown.min(room).max(r)));
            }
        }
        // Each lord acts in turn, the first of them last so it stays in `self.boss`.
        let extras = std::mem::take(&mut self.more_bosses);
        let first = self.boss.take();
        let mut kept = Vec::new();
        for ai in extras {
            self.boss = Some(ai);
            self.update_one_boss(dt);
            kept.extend(self.boss.take());
        }
        self.boss = first;
        self.update_one_boss(dt);
        // Lords that came while it acted (none do) or the first gone: the next takes over.
        if self.boss.is_none() && !kept.is_empty() {
            self.boss = Some(kept.remove(0));
            self.boss_kind = self.boss.as_ref().map(|b| b.kind);
        }
        self.more_bosses = kept;
    }

    fn update_one_boss(&mut self, dt: f32) {
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
        let voice = super::spawn::boss_voice(ai.kind);
        if !ai.phase2 && hp < max_hp * 0.5 {
            ai.phase2 = true;
            let line = ai.kind.lines()[1];
            if line.is_empty() {
                self.banner("", "The foe grows desperate...", RED, 2.0);
            }
            self.say(line, voice);
            self.sfx("roar", 0.8);
            self.shake = 10.0;
        }
        // A last phase for the greatest lords: faster, angrier, and a new trick.
        if ai.kind.has_third_phase() && !ai.phase3 && hp < max_hp * 0.25 {
            ai.phase3 = true;
            self.say(ai.kind.lines()[2], voice);
            self.banner("", "...and it rises once more", RED, 2.5);
            self.sfx("roar", 1.0);
            self.sfx_at("slam", 0.7, pos);
            self.shake = 16.0;
            // Its fury clears the air around it first.
            let off = gen_range(0.0, TAU);
            self.ring(pos, 36, 200.0, 16.0, off, None);
            self.fx.push(Fx { pos, life: 0.8, max_life: 0.8, kind: FxKind::Ring { r: 260.0, color: voice } });
        }
        let p2 = ai.phase2;
        let p3 = ai.phase3;
        let fast = if p3 {
            1.6
        } else if p2 {
            1.35
        } else {
            1.0
        };
        let to = (pp - pos).normalize_or_zero();
        let lap = 1.0 + (self.bosses_spawned.saturating_sub(1) / 5) as f32 * 0.4;
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
                        if p3 {
                            // The graves open: hands of stone burst up around the hunted.
                            for k in 0..8 {
                                let at = pp + Vec2::from_angle(k as f32 / 8.0 * TAU) * 130.0;
                                self.pillar(at, 45.0, 0.9, 22.0 * lap);
                            }
                            self.summon(EnemyKind::Hollow, pos, 6, 90.0);
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
            BossKind::Bride => match ai.state {
                // Drifts after you, weeping slow tears of water.
                0 => {
                    pos += to * speed * fast * dt;
                    ai.shot -= dt;
                    if ai.shot <= 0.0 {
                        ai.shot = 1.3 / fast;
                        let base = to.to_angle();
                        for k in -2..=2 {
                            self.enemy_bullet(pos, Vec2::from_angle(base + k as f32 * 0.25) * 150.0, 9.0, 13.0 * lap, 0.0, 6.0);
                        }
                        self.sfx_quiet("bullet");
                    }
                    if ai.t <= 0.0 {
                        ai.state = 1;
                        ai.t = 0.8;
                        ai.count = 0;
                        ai.aim = to;
                        let side = to.perp();
                        self.tele_line(pos - side * 380.0, pos + side * 380.0, 30.0, 0.8 / fast);
                    }
                }
                // The tide: a wall of water rolls out with one gap in it.
                1 => {
                    if ai.t <= 0.0 {
                        let side = ai.aim.perp();
                        let gap: i32 = gen_range(-4, 5);
                        for k in -9..=9 {
                            if (k - gap).abs() <= 1 {
                                continue;
                            }
                            let at = pos + side * k as f32 * 42.0;
                            self.enemy_bullet(at, ai.aim * 170.0, 10.0, 14.0 * lap, 0.0, 5.0);
                        }
                        self.sfx_at("explode", 0.4, pos);
                        ai.count += 1;
                        let walls = if p3 {
                            3
                        } else if p2 {
                            2
                        } else {
                            1
                        };
                        if ai.count < walls {
                            ai.t = 0.7;
                            ai.aim = to;
                            let side = to.perp();
                            self.tele_line(pos - side * 380.0, pos + side * 380.0, 30.0, 0.7 / fast);
                        } else {
                            ai.state = 2;
                            ai.t = 0.6;
                        }
                    }
                }
                // Weeping: pillars of black water spiral in on the hunted.
                2 => {
                    if ai.t <= 0.0 {
                        let n = if p2 { 12 } else { 8 };
                        let spin = gen_range(0.0, TAU);
                        for k in 0..n {
                            let f = k as f32 / n as f32;
                            let at = pp + Vec2::from_angle(spin + f * TAU * 1.5) * (240.0 - f * 200.0);
                            self.pillar(at, 48.0, 0.8 + f * 0.9, 24.0 * lap);
                        }
                        ai.state = 3;
                        ai.t = 1.6;
                    }
                }
                3 => {
                    if ai.t <= 0.0 {
                        self.summon(EnemyKind::Leech, pos, if p2 { 14 } else { 8 }, 70.0);
                        if p3 {
                            self.summon(EnemyKind::Wraith, pos, 3, 120.0);
                        }
                        ai.state = 0;
                        ai.t = 3.0;
                    }
                }
                _ => ai.state = 0,
            },
            BossKind::BogMother => match ai.state {
                // Crawls closer and spits mud.
                0 => {
                    pos += to * speed * fast * dt;
                    ai.shot -= dt;
                    if ai.shot <= 0.0 {
                        ai.shot = 1.6 / fast;
                        let base = to.to_angle();
                        for k in -1..=1 {
                            self.enemy_bullet(pos, Vec2::from_angle(base + k as f32 * 0.35) * 180.0, 12.0, 15.0 * lap, 0.0, 4.0);
                        }
                        self.sfx_at("explode", 0.3, pos);
                    }
                    if ai.t <= 0.0 {
                        ai.state = 1;
                        ai.t = 1.0;
                        ai.aim = pp;
                        self.tele_circle(pp, 170.0, 1.0 / fast);
                    }
                }
                // Gathers itself, then leaps.
                1 => {
                    if ai.t <= 0.0 {
                        ai.state = 2;
                        ai.t = 0.5;
                        self.sfx_at("roll", 1.0, pos);
                    }
                }
                2 => {
                    alpha = 0.45;
                    let target = ai.aim;
                    pos += (target - pos) * (dt / ai.t.max(dt)).min(1.0);
                    if ai.t <= 0.0 {
                        pos = target;
                        self.slam(pos, 170.0, 30.0 * lap);
                        let off = gen_range(0.0, TAU);
                        self.ring(pos, if p2 { 24 } else { 16 }, 160.0, 14.0 * lap, off, None);
                        ai.count += 1;
                        if p2 && ai.count < 2 {
                            ai.state = 1;
                            ai.t = 0.7;
                            ai.aim = pp;
                            self.tele_circle(pp, 170.0, 0.7 / fast);
                        } else {
                            ai.count = 0;
                            ai.state = 3;
                            ai.t = 0.8;
                        }
                    }
                }
                3 => {
                    if ai.t <= 0.0 {
                        self.summon(EnemyKind::Ghoul, pos, if p2 { 10 } else { 6 }, 90.0);
                        self.sfx_at("roar", 0.6, pos);
                        ai.state = 0;
                        ai.t = 2.4;
                    }
                }
                _ => ai.state = 0,
            },
            BossKind::Penitent => match ai.state {
                0 => {
                    pos += to * speed * fast * dt;
                    if ai.t <= 0.0 {
                        ai.state = 1;
                        ai.t = 2.6;
                    }
                }
                // The flail whirls, flinging iron in a widening spiral.
                1 => {
                    pos += to * speed * 0.4 * dt;
                    ai.spin += if p2 { 2.4 } else { 1.8 } * dt;
                    ai.shot -= dt;
                    if ai.shot <= 0.0 {
                        ai.shot = 0.12;
                        let arms = if p3 {
                            4
                        } else if p2 {
                            3
                        } else {
                            2
                        };
                        for k in 0..arms {
                            let a = ai.spin + k as f32 / arms as f32 * TAU;
                            self.bullet(pos, a, 140.0, 13.0 * lap, 8.0);
                        }
                        self.sfx_quiet("bullet");
                    }
                    if ai.t <= 0.0 {
                        ai.state = 2;
                        ai.t = 0.6;
                        ai.aim = to;
                        self.tele_line(pos, pos + to * 600.0, 34.0, 0.6 / fast);
                    }
                }
                // The chain lashes out along the marked line.
                2 => {
                    if ai.t <= 0.0 {
                        for k in 1..16 {
                            let at = pos + ai.aim * k as f32 * 38.0;
                            self.enemy_bullet(at, Vec2::ZERO, 14.0, 22.0 * lap, 0.0, 0.35);
                        }
                        self.sfx_at("clang", 0.8, pos);
                        self.shake = self.shake.max(8.0);
                        ai.state = 3;
                        ai.t = 0.5;
                        ai.count = 0;
                    }
                }
                // Three crushing steps toward the hunted.
                3 => {
                    if ai.t <= 0.0 {
                        let step = pos + to * 130.0;
                        self.tele_circle(step, 120.0, 0.5 / fast);
                        ai.aim = step;
                        ai.state = 4;
                        ai.t = 0.5;
                    }
                }
                4 => {
                    if ai.t <= 0.0 {
                        pos = ai.aim;
                        self.slam(pos, 120.0, 26.0 * lap);
                        ai.count += 1;
                        if ai.count < 3 {
                            ai.state = 3;
                            ai.t = 0.15;
                        } else {
                            if p2 {
                                self.summon(EnemyKind::Shieldbearer, pos, 2, 140.0);
                            }
                            if p3 {
                                // It tears a link from its chain and hurls the pieces everywhere.
                                let off = gen_range(0.0, TAU);
                                self.ring(pos, 28, 220.0, 15.0 * lap, off, None);
                            }
                            ai.state = 0;
                            ai.t = 1.4;
                        }
                    }
                }
                _ => ai.state = 0,
            },
            BossKind::Choir => match ai.state {
                // Fades out and sings itself somewhere else.
                0 => {
                    alpha = (ai.t / 0.6).clamp(0.0, 1.0);
                    if ai.t <= 0.0 {
                        pos = pp + Vec2::from_angle(gen_range(0.0, TAU)) * 300.0;
                        ai.state = 1;
                        ai.t = 0.6;
                    }
                }
                1 => {
                    alpha = 1.0 - (ai.t / 0.6).clamp(0.0, 1.0);
                    if ai.t <= 0.0 {
                        ai.state = 2;
                        ai.t = 2.6;
                        ai.spin = to.to_angle() - 1.2;
                    }
                }
                // The hymn: a beam of voices sweeping across the ring.
                2 => {
                    ai.spin += if p2 { 1.3 } else { 1.0 } * dt;
                    ai.shot -= dt;
                    if ai.shot <= 0.0 {
                        ai.shot = 0.05;
                        let beams = if p3 {
                            3
                        } else if p2 {
                            2
                        } else {
                            1
                        };
                        for k in 0..beams {
                            let a = ai.spin + k as f32 / beams as f32 * TAU;
                            self.bullet(pos, a, 280.0, 11.0 * lap, 6.0);
                        }
                        self.sfx_quiet("bullet");
                    }
                    if ai.t <= 0.0 {
                        ai.state = 3;
                        ai.t = 0.6;
                    }
                }
                // Notes that follow you for a while.
                3 => {
                    if ai.t <= 0.0 {
                        let n = if p2 { 9 } else { 6 };
                        for k in 0..n {
                            let a = k as f32 / n as f32 * TAU;
                            self.enemy_bullet(pos, Vec2::from_angle(a) * 120.0, 9.0, 14.0 * lap, 1.1, 5.0);
                        }
                        self.sfx_at("chime", 0.8, pos);
                        ai.state = 4;
                        ai.t = 1.4;
                    }
                }
                4 => {
                    if ai.t <= 0.0 {
                        self.summon(EnemyKind::Banshee, pos, if p2 { 4 } else { 2 }, 130.0);
                        if p3 {
                            // The last hymn: pillars of sound on every side.
                            for k in 0..10 {
                                let at = pp + Vec2::from_angle(k as f32 / 10.0 * TAU) * 180.0;
                                self.pillar(at, 50.0, 1.0, 22.0 * lap);
                            }
                        }
                        ai.state = 0;
                        ai.t = 0.6;
                    }
                }
                _ => ai.state = 0,
            },
            BossKind::Seraph => match ai.state {
                // Hovers after you and drops stones where you stand.
                0 => {
                    pos += to * speed * fast * dt;
                    ai.shot -= dt;
                    if ai.shot <= 0.0 {
                        ai.shot = 1.1 / fast;
                        self.pillar(pp, 50.0, 1.0, 20.0 * lap);
                    }
                    if ai.t <= 0.0 {
                        ai.state = 1;
                        ai.t = 0.7;
                        ai.count = 0;
                    }
                }
                // Rises into the dark, marks its dive, and falls.
                1 => {
                    alpha = 0.35 + 0.65 * (ai.t / 0.7).clamp(0.0, 1.0);
                    if ai.t <= 0.0 {
                        ai.aim = to;
                        self.tele_line(pos, pos + to * 700.0, r * 1.2, 0.55 / fast);
                        ai.state = 2;
                        ai.t = 0.55;
                    }
                }
                2 => {
                    alpha = 0.35;
                    if ai.t <= 0.0 {
                        ai.state = 3;
                        ai.t = 0.45;
                        self.sfx_at("roll", 1.0, pos);
                    }
                }
                3 => {
                    pos += ai.aim * 1350.0 * dt;
                    if ai.t <= 0.0 {
                        self.slam(pos, 140.0, 26.0 * lap);
                        if p2 {
                            let off = gen_range(0.0, TAU);
                            self.ring(pos, 20, 170.0, 14.0 * lap, off, None);
                        }
                        ai.count += 1;
                        if ai.count < if p2 { 3 } else { 2 } {
                            ai.state = 1;
                            ai.t = 0.4;
                        } else {
                            ai.state = 4;
                            ai.t = 0.8;
                        }
                    }
                }
                4 => {
                    if ai.t <= 0.0 {
                        self.summon(EnemyKind::Gargoyle, pos, if p2 { 5 } else { 3 }, 110.0);
                        ai.state = 0;
                        ai.t = 2.4;
                    }
                }
                _ => ai.state = 0,
            },
            BossKind::Hound => match ai.state {
                // Circles its prey, closer and closer.
                0 => {
                    ai.spin += 1.4 * dt;
                    let target = pp + Vec2::from_angle(ai.spin) * 230.0;
                    pos += (target - pos).clamp_length_max(speed * fast * dt);
                    if ai.t <= 0.0 {
                        ai.state = 1;
                        ai.count = 0;
                        ai.t = 0.35;
                        ai.aim = (to + to.perp() * 0.35).normalize_or_zero();
                        let a = ai.aim;
                        self.tele_line(pos, pos + a * 420.0, r * 1.1, 0.35 / fast);
                    }
                }
                // Zig-zag lunges, each one marked an instant before.
                1 => {
                    if ai.t <= 0.0 {
                        ai.state = 2;
                        ai.t = 0.28;
                    }
                }
                2 => {
                    pos += ai.aim * 1250.0 * dt;
                    if ai.t <= 0.0 {
                        ai.count += 1;
                        if ai.count < if p2 { 5 } else { 3 } {
                            let zig = if ai.count % 2 == 0 { 0.4 } else { -0.4 };
                            ai.aim = (to + to.perp() * zig).normalize_or_zero();
                            let a = ai.aim;
                            self.tele_line(pos, pos + a * 420.0, r * 1.1, 0.3 / fast);
                            ai.state = 1;
                            ai.t = 0.3;
                        } else {
                            let off = gen_range(0.0, TAU);
                            self.ring(pos, 14, 180.0, 13.0 * lap, off, None);
                            ai.state = 3;
                            ai.t = 0.6;
                        }
                    }
                }
                // A howl that calls the pack.
                3 => {
                    if ai.t <= 0.0 {
                        self.summon(EnemyKind::Ghoul, pos, if p2 { 10 } else { 6 }, 80.0);
                        self.sfx_at("roar", 0.8, pos);
                        self.fx.push(Fx { pos, life: 0.7, max_life: 0.7, kind: FxKind::Ring { r: 220.0, color: Color::from_rgba(230, 220, 220, 255) } });
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
                        if p3 && chance(0.3) {
                            // The throne burns: fire rains wherever the heroes stand.
                            let at = pp + Vec2::from_angle(gen_range(0.0, TAU)) * gen_range(0.0, 160.0);
                            self.pillar(at, 40.0, 0.9, 20.0 * lap);
                        }
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

        // A lord the crown's wearer holds goes where they steer it.
        if let Some(p) = self.steer_lord(ai.id, pos, speed, dt) {
            pos = p;
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
