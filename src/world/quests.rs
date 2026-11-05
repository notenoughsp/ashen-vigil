//! Small quests during a vigil: a task with a time limit, now and then, that pays a coffer
//! to every hero when it is done in time. The host keeps them; clients see them in snapshots.

use super::*;

/// The first quest comes this long into a vigil, the next this long after one ends.
const QUEST_FIRST: f32 = 75.0;
const QUEST_GAP: f32 = 100.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum QuestKind {
    /// Fell this many foes.
    Slay,
    /// Fell this many elites.
    Elites,
    /// Nobody is hurt for this many seconds in a row.
    Untouched,
    /// Light this many braziers.
    Kindle,
    /// Take this many relics or blessings from the field.
    Relics,
}

pub const QUEST_KINDS: [QuestKind; 5] = [QuestKind::Slay, QuestKind::Elites, QuestKind::Untouched, QuestKind::Kindle, QuestKind::Relics];

impl QuestKind {
    pub fn from_u8(v: u8) -> QuestKind {
        QUEST_KINDS.get(v as usize).copied().unwrap_or(QuestKind::Slay)
    }

    pub fn to_u8(self) -> u8 {
        QUEST_KINDS.iter().position(|k| *k == self).unwrap_or(0) as u8
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Quest {
    pub kind: QuestKind,
    pub goal: u32,
    pub progress: u32,
    /// Seconds left to do it.
    pub left: f32,
}

impl Quest {
    /// The seconds a quest of this kind is given.
    pub fn left_at_start(&self) -> f32 {
        match self.kind {
            QuestKind::Slay => 60.0,
            QuestKind::Elites => 120.0,
            QuestKind::Untouched | QuestKind::Relics => 90.0,
            QuestKind::Kindle => 75.0,
        }
    }

    /// What the quest asks, for the HUD.
    pub fn text(&self) -> String {
        match self.kind {
            QuestKind::Slay => format!("Fell {} foes", self.goal),
            QuestKind::Elites => format!("Fell {} elites", self.goal),
            QuestKind::Untouched => format!("Take no wound for {} seconds", self.goal),
            QuestKind::Kindle => format!("Light {} braziers", self.goal),
            QuestKind::Relics => format!("Take {} relics or blessings", self.goal),
        }
    }
}

impl World {
    /// Offers a quest now and then, counts down the one in hand, and pays or drops it.
    pub(crate) fn update_quests(&mut self, dt: f32) {
        if self.boss_rush {
            return;
        }
        if let Some(q) = self.quest.as_mut() {
            q.left -= dt;
            if q.kind == QuestKind::Untouched {
                q.progress = (self.time - self.last_wound).max(0.0) as u32;
            }
            let (done, failed) = (q.progress >= q.goal, q.left <= 0.0);
            if done {
                let text = q.text();
                crate::log!("[{:02}:{:02}] quest done: {}", self.time as u32 / 60, self.time as u32 % 60, text);
                self.quest = None;
                self.next_quest = self.time + QUEST_GAP;
                self.quests_done += 1;
                for h in 0..self.heroes.len() {
                    let pos = self.heroes[h].p.pos + rand_dir() * 30.0;
                    self.pickups.push(Pickup::new(pos, PickKind::Coffer(false)));
                }
                self.banner("QUEST DONE", &text, Color::from_rgba(240, 210, 120, 255), 3.0);
                self.sfx("evolve", 0.7);
            } else if failed {
                let text = q.text();
                crate::log!("[{:02}:{:02}] quest failed: {}", self.time as u32 / 60, self.time as u32 % 60, text);
                self.quest = None;
                self.next_quest = self.time + QUEST_GAP;
                self.toast("The quest has slipped away.");
            }
            return;
        }
        if self.next_quest == 0.0 {
            self.next_quest = self.time + QUEST_FIRST;
        }
        if self.time < self.next_quest || self.boss.is_some() || self.pause != Pause::Running {
            return;
        }
        // Harder the longer the night has gone on.
        let k = 1.0 + self.time / 600.0;
        let braziers_left = self.braziers.iter().filter(|b| b.burn <= 0.0).count() as u32;
        let mut kinds = vec![QuestKind::Slay, QuestKind::Elites, QuestKind::Untouched, QuestKind::Relics];
        if braziers_left >= 2 {
            kinds.push(QuestKind::Kindle);
        }
        let kind = kinds[gen_range(0, kinds.len())];
        let goal = match kind {
            QuestKind::Slay => (150.0 * k) as u32 / 10 * 10,
            QuestKind::Elites => (1.0 + k).min(5.0) as u32,
            QuestKind::Untouched => ((20.0 + 5.0 * k) as u32).min(45),
            QuestKind::Kindle | QuestKind::Relics => 2,
        };
        let mut q = Quest { kind, goal, progress: 0, left: 0.0 };
        q.left = q.left_at_start();
        if kind == QuestKind::Untouched {
            self.last_wound = self.time;
        }
        self.log_event(&format!("quest offered: {}", q.text()));
        self.toast(&format!("A quest: {}. A coffer for every hero awaits.", q.text()));
        self.sfx("chime", 0.8);
        self.quest = Some(q);
    }

    /// Counts something toward the quest in hand, if it asks for it.
    pub(crate) fn quest_count(&mut self, kind: QuestKind, n: u32) {
        if let Some(q) = self.quest.as_mut().filter(|q| q.kind == kind) {
            q.progress = (q.progress + n).min(q.goal);
        }
    }
}
