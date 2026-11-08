//! Purely visual state: particles, blood, bodies, numbers, banners. Runs on host and clients.

use super::*;

impl World {
    fn particle_cap(&self) -> usize {
        (3000.0 * self.particle_mult) as usize
    }

    /// In cute mode anything red turns pink, and soot and scorch marks turn lilac.
    pub(crate) fn soften(&self, c: Color) -> Color {
        if !self.cute {
            c
        } else if c.r > c.g * 1.4 && c.r > c.b * 1.4 {
            Color::new(1.0, 0.55 + c.g * 0.2, 0.75, c.a)
        } else if c.r + c.g + c.b < 0.4 {
            Color::new(0.82, 0.7, 0.9, c.a * 0.7)
        } else {
            c
        }
    }

    pub fn particle(&mut self, pos: Vec2, vel: Vec2, life: f32, size: f32, color: Color, glow: bool) {
        let color = self.soften(color);
        if self.particles.len() < self.particle_cap() {
            self.particles.push(Particle { pos, vel, life, max_life: life, size, color, glow, stain: false });
        }
    }

    pub fn add_decal(&mut self, pos: Vec2, r: f32, color: Color, kind: DecalKind, life: f32) {
        let color = self.soften(color);
        if self.decals.len() >= MAX_DECALS {
            self.decals.pop_front();
        }
        self.decals.push_back(Decal { pos, r, rot: gen_range(0.0, TAU), color, kind, life, max_life: life, seed: gen_range(0, u32::MAX) });
    }

    /// A spurt of blood in direction `dir`; some drops stain the ground.
    pub fn blood_spray(&mut self, pos: Vec2, dir: Vec2, color: Color, r: f32, n: usize) {
        let n = ((if self.gore { n } else { n / 2 }) as f32 * self.particle_mult).ceil() as usize;
        let cap = self.particle_cap();
        for _ in 0..n {
            if self.particles.len() >= cap {
                break;
            }
            let d = (dir + rand_dir() * 0.7).normalize_or_zero();
            let life = gen_range(0.25, 0.6);
            self.particles.push(Particle {
                pos: pos + rand_dir() * r * 0.4,
                vel: d * gen_range(60.0, 220.0),
                life,
                max_life: life,
                size: gen_range(1.5, 3.2),
                color,
                glow: false,
                stain: self.gore && chance(0.35),
            });
        }
    }

    /// What is left when something dies: a body, a pool, gore flying.
    pub fn death_burst(&mut self, pos: Vec2, kind: EnemyKind, elite: bool, r: f32) {
        let blood = kind.blood();
        let boss = matches!(kind, EnemyKind::Boss(_));
        let big = boss || elite;
        let gore = if self.gore { 1.0 } else { 0.4 };
        let n = ((if boss {
            60.0
        } else if elite {
            24.0
        } else {
            8.0
        }) * gore) as usize;
        self.blood_spray(pos, Vec2::ZERO, blood, r, n);
        // Chunks.
        let chunks = ((if big { 12.0 } else { 3.0 }) * gore * self.particle_mult) as usize;
        let cap = self.particle_cap();
        let dark = Color::new(blood.r * 0.6, blood.g * 0.5, blood.b * 0.5, 1.0);
        for _ in 0..chunks {
            if self.particles.len() >= cap {
                break;
            }
            let life = gen_range(0.4, 0.8);
            self.particles.push(Particle {
                pos,
                vel: rand_dir() * gen_range(80.0, 260.0),
                life,
                max_life: life,
                size: gen_range(3.0, 5.0),
                color: dark,
                glow: false,
                stain: self.gore,
            });
        }
        let pool = Color::new(blood.r * 0.7, blood.g * 0.6, blood.b * 0.6, 0.75);
        self.add_decal(pos, r * if big { 2.2 } else { 1.3 }, pool, DecalKind::Blood, if big { 60.0 } else { 30.0 });
        if self.gore && !matches!(kind, EnemyKind::Moth | EnemyKind::Wraith) {
            let body = Color::from_rgba(40, 34, 32, 230);
            self.add_decal(pos, r, body, DecalKind::Corpse(kind), if big { 30.0 } else { 9.0 });
        }
        if kind == EnemyKind::Wraith {
            for _ in 0..10 {
                self.particle(pos, rand_dir() * gen_range(20.0, 90.0), 0.8, 3.0, Color::from_rgba(120, 170, 255, 255), true);
            }
        }
    }

    pub fn update_fx(&mut self, dt: f32) {
        let mut stains = Vec::new();
        for p in self.particles.iter_mut() {
            p.life -= dt;
            p.pos += p.vel * dt;
            p.vel *= (-2.5 * dt).exp();
            if p.life <= 0.0 && p.stain {
                stains.push((p.pos, p.size, p.color));
            }
        }
        self.particles.retain(|p| p.life > 0.0);
        for (pos, size, c) in stains {
            self.add_decal(pos, size * gen_range(1.5, 3.0), Color::new(c.r * 0.75, c.g * 0.7, c.b * 0.7, 0.8), DecalKind::Blood, 25.0);
        }
        for d in self.decals.iter_mut() {
            d.life -= dt;
        }
        while self.decals.front().map(|d| d.life <= 0.0).unwrap_or(false) {
            self.decals.pop_front();
        }
        for f in self.fx.iter_mut() {
            f.life -= dt;
        }
        self.fx.retain(|f| f.life > 0.0);
        for n in self.nums.iter_mut() {
            n.life -= dt;
            n.pos.y -= 30.0 * dt;
        }
        self.nums.retain(|n| n.life > 0.0);
        if !self.is_client {
            for t in self.toasts.iter_mut() {
                t.1 -= dt;
            }
            self.toasts.retain(|t| t.1 > 0.0);
            if let Some(b) = &mut self.banner {
                b.3 -= dt;
                if b.3 <= 0.0 {
                    self.banner = None;
                }
            }
            if let Some(q) = &mut self.quote {
                q.2 -= dt;
                if q.2 <= 0.0 {
                    self.quote = None;
                }
            }
        }
        // Drifting embers around the camera.
        let embers = self.particles.iter().filter(|p| p.glow && p.vel.y < -5.0 && p.size < 2.0).count();
        if embers < 70 {
            let c = self.cam + vec2(gen_range(-0.6, 0.6) * self.screen.x, gen_range(-0.3, 0.7) * self.screen.y);
            let col = Color::from_rgba(255, gen_range(90, 170), 40, 255);
            self.particle(c, vec2(gen_range(-15.0, 15.0), gen_range(-45.0, -20.0)), gen_range(3.0, 6.0), gen_range(1.0, 1.9), col, true);
        }
        self.shake = (self.shake - dt * 30.0).max(0.0);
        let frac = (self.boss_hp.0 / self.boss_hp.1.max(1.0)).max(0.0);
        if self.boss_bar_lag > frac {
            self.boss_bar_lag = (self.boss_bar_lag - dt * 0.25).max(frac);
        }
        if !self.heroes.is_empty() {
            let me = self.me();
            let target = if me.downed && me.ghost != Vec2::ZERO { me.ghost } else { me.p.pos };
            self.cam = self.cam.lerp(target, (10.0 * dt).min(1.0));
        }
    }
}
