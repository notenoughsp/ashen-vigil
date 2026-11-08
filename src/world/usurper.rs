//! A secret of the online vigil: one player may wear the crown. Each lord that comes alone
//! answers to them: they steer it, their skill key makes it strike, and while it stands their
//! hero is gone from the field, untouchable. When the lord falls they return, whole.
//!
//! The host runs it; the usurper's client only moves its hero, which the lord follows.

use super::*;

/// Seconds between two of the usurper's strikes.
const STRIKE_REST: f32 = 4.0;

impl World {
    /// A lord came: if someone wears the crown and is free to, they take it.
    pub(crate) fn crown_lord(&mut self, id: u32) {
        let Some(slot) = self.usurper else { return };
        if self.possessed.is_some() || self.boss_rush {
            return;
        }
        let Some(h) = self.heroes.iter().position(|h| h.slot == slot && h.connected && !h.downed) else { return };
        let Some(lord) = self.enemies.iter().find(|e| e.id == id) else { return };
        let (at, kind) = (lord.pos, lord.kind);
        self.possessed = Some(id);
        let hero = &mut self.heroes[h];
        hero.possessing = true;
        hero.p.pos = at;
        hero.skill_cd = 1.0;
        let name = hero.name.clone();
        let lord = if let EnemyKind::Boss(k) = kind { k.name() } else { "the lord" };
        self.log_event(&format!("{} takes {}", name, lord));
        self.banner(
            &format!("{} TAKES {}", name.to_uppercase(), lord.to_uppercase()),
            "The crown answers. Fell the lord to bring them back.",
            Color::from_rgba(200, 120, 255, 255),
            4.0,
        );
    }

    /// The usurper's lord follows their hero, at its own pace (even a lord that never walks
    /// drifts a little for them).
    pub(crate) fn steer_lord(&self, id: u32, pos: Vec2, speed: f32, dt: f32) -> Option<Vec2> {
        if self.possessed != Some(id) {
            return None;
        }
        let h = self.heroes.iter().find(|h| h.possessing)?;
        let to = h.p.pos - pos;
        Some(pos + to.clamp_length_max(speed.max(90.0) * 1.15 * dt))
    }

    /// The usurper's skill key: the lord slams a ring of bullets and a shockwave.
    pub(crate) fn lord_strike(&mut self, h: usize) {
        if self.heroes[h].skill_cd > 0.0 {
            return;
        }
        self.heroes[h].skill_cd = STRIKE_REST;
        let Some(id) = self.possessed else { return };
        let Some(lord) = self.enemies.iter().find(|e| e.id == id && !e.dead) else { return };
        let (at, dmg) = (lord.pos, lord.dmg * 0.6);
        let off = gen_range(0.0, TAU);
        for k in 0..18 {
            self.enemy_bullet(at, Vec2::from_angle(off + k as f32 / 18.0 * TAU) * 200.0, 7.0, dmg, 0.0, 3.0);
        }
        self.waves.push(Shockwave { pos: at, r: 10.0, max_r: 260.0, speed: 560.0, dmg, hit: [false; MAX_HEROES] });
        self.shake = self.shake.max(8.0);
        self.sfx_at("slam", 0.9, at);
    }

    /// The usurper's lord fell: they come back to their own hero, whole.
    pub(crate) fn uncrown(&mut self, id: u32) {
        if self.possessed != Some(id) {
            return;
        }
        self.possessed = None;
        let Some(h) = self.heroes.iter().position(|h| h.possessing) else { return };
        let hero = &mut self.heroes[h];
        hero.possessing = false;
        hero.p.hp = hero.stats.max_hp;
        hero.p.iframes = 3.0;
        let name = hero.name.clone();
        self.toast(&format!("{} lets go of the crown and returns.", name));
    }
}
