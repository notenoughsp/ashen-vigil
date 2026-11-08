//! Pings: a player marks a spot for everyone, by key, middle mouse button or the right stick.
//! What the mark says follows from where it is: a foe worth fearing there means danger, a
//! wounded or fallen hero pinging themself asks for help, anything else is "here".
//!
//! The host (or the single-player game) places them; clients ask for theirs through their
//! input and see everyone's in snapshots.

use super::*;

/// How long a ping stays, how many each player may have at once, and how often they may ping.
pub const PING_LIFE: f32 = 5.0;
const PINGS_EACH: usize = 3;
const PING_GAP: f32 = 0.4;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PingKind {
    Here,
    Danger,
    Help,
}

impl PingKind {
    pub fn label(self) -> &'static str {
        match self {
            PingKind::Here => "Here",
            PingKind::Danger => "Danger",
            PingKind::Help => "Help",
        }
    }

    pub fn to_u8(self) -> u8 {
        self as u8
    }

    pub fn from_u8(v: u8) -> PingKind {
        match v {
            1 => PingKind::Danger,
            2 => PingKind::Help,
            _ => PingKind::Here,
        }
    }
}

pub struct Ping {
    pub pos: Vec2,
    /// The player who placed it.
    pub slot: u8,
    pub kind: PingKind,
    pub life: f32,
}

impl World {
    /// Places a ping for the hero in `slot`, if they have not just pinged.
    pub fn add_ping(&mut self, slot: u8, pos: Vec2) {
        let s = (slot as usize).min(MAX_HEROES - 1);
        if self.time - self.ping_last[s] < PING_GAP && self.ping_last[s] > 0.0 {
            return;
        }
        self.ping_last[s] = self.time.max(0.001);
        let pos = clamp_world(pos, 10.0);
        let fearful = self
            .enemies
            .iter()
            .any(|e| !e.dead && (e.boss || e.elite || matches!(e.kind, EnemyKind::Champion | EnemyKind::Rival | EnemyKind::Mimic(true))) && e.pos.distance(pos) < 120.0 + e.r);
        let me = self.heroes.iter().find(|h| h.slot == slot);
        let hurt =
            me.is_some_and(|h| (h.downed && (h.p.pos.distance(pos) < 120.0 || h.ghost.distance(pos) < 120.0)) || (h.p.pos.distance(pos) < 90.0 && h.p.hp < h.stats.max_hp * 0.4));
        let kind = if hurt {
            PingKind::Help
        } else if fearful {
            PingKind::Danger
        } else {
            PingKind::Here
        };
        // Each player's oldest mark makes way.
        if self.pings.iter().filter(|p| p.slot == slot).count() >= PINGS_EACH {
            if let Some(k) = self.pings.iter().position(|p| p.slot == slot) {
                self.pings.remove(k);
            }
        }
        self.pings.push(Ping { pos, slot, kind, life: PING_LIFE });
        self.sfx(if kind == PingKind::Here { "select" } else { "chime" }, 0.6);
    }

    pub(crate) fn update_pings(&mut self, dt: f32) {
        for p in self.pings.iter_mut() {
            p.life -= dt;
        }
        self.pings.retain(|p| p.life > 0.0);
    }
}
