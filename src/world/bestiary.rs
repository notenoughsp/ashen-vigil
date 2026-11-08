//! The bestiary: every kind of foe and every lord, how many the party has felled and how often
//! each has felled you, with a line of lore once it has been met.
//!
//! The host counts felled foes by kind for the vigil and sends the counts with snapshots; each
//! player adds them to their own save when the vigil ends.

use super::*;

/// Everything the bestiary lists, in its order: the foes, the strangers, then the lords.
pub const BEASTS: [EnemyKind; 40] = [
    EnemyKind::Hollow,
    EnemyKind::Ghoul,
    EnemyKind::Moth,
    EnemyKind::Leech,
    EnemyKind::Archer,
    EnemyKind::Bloat,
    EnemyKind::Wraith,
    EnemyKind::Banshee,
    EnemyKind::Cultist,
    EnemyKind::Gargoyle,
    EnemyKind::Shieldbearer,
    EnemyKind::Necromancer,
    EnemyKind::Knight,
    EnemyKind::Courier,
    EnemyKind::Champion,
    EnemyKind::Rival,
    EnemyKind::Mimic(true),
    EnemyKind::EmberWolf,
    EnemyKind::BarkHusk,
    EnemyKind::SmokeWraith,
    EnemyKind::IceMonk,
    EnemyKind::FrostCrow,
    EnemyKind::FrozenKnight,
    EnemyKind::ThornVine,
    EnemyKind::Puppet,
    EnemyKind::MothQueen,
    EnemyKind::Statue,
    EnemyKind::Boss(BossKind::Gravewarden),
    EnemyKind::Boss(BossKind::Hound),
    EnemyKind::Boss(BossKind::Matriarch),
    EnemyKind::Boss(BossKind::BogMother),
    EnemyKind::Boss(BossKind::Bride),
    EnemyKind::Boss(BossKind::Hermit),
    EnemyKind::Boss(BossKind::Seraph),
    EnemyKind::Boss(BossKind::Choir),
    EnemyKind::Boss(BossKind::Penitent),
    EnemyKind::Boss(BossKind::BurntOak),
    EnemyKind::Boss(BossKind::SnowAbbess),
    EnemyKind::Boss(BossKind::Gardener),
    EnemyKind::Boss(BossKind::Sovereign),
];

/// Where a kind of foe stands in `BEASTS`; props stand nowhere.
pub fn beast_index(kind: EnemyKind) -> Option<usize> {
    let kind = if let EnemyKind::Mimic(_) = kind { EnemyKind::Mimic(true) } else { kind };
    BEASTS.iter().position(|k| *k == kind)
}

/// A foe's name in the bestiary.
pub fn beast_name(kind: EnemyKind) -> &'static str {
    match kind {
        EnemyKind::Hollow => "Hollow",
        EnemyKind::Ghoul => "Ghoul",
        EnemyKind::Moth => "Gloom Moth",
        EnemyKind::Archer => "Bone Archer",
        EnemyKind::Bloat => "Bloat",
        EnemyKind::Wraith => "Wraith",
        EnemyKind::Knight => "Fallen Knight",
        EnemyKind::Cultist => "Cultist",
        EnemyKind::Shieldbearer => "Shieldbearer",
        EnemyKind::Necromancer => "Necromancer",
        EnemyKind::Leech => "Leech",
        EnemyKind::Gargoyle => "Gargoyle",
        EnemyKind::Banshee => "Banshee",
        EnemyKind::Courier => "Courier",
        EnemyKind::Champion => "Champion of the Mist",
        EnemyKind::Rival => "Sven",
        EnemyKind::Mimic(_) => "Mimic",
        EnemyKind::Prop(_) => "Prop",
        EnemyKind::EmberWolf => "Ember Wolf",
        EnemyKind::BarkHusk => "Bark Husk",
        EnemyKind::SmokeWraith => "Smoke Wraith",
        EnemyKind::IceMonk => "Ice Monk",
        EnemyKind::FrostCrow => "Frost Crow",
        EnemyKind::FrozenKnight => "Frozen Knight",
        EnemyKind::ThornVine => "Thorn Vine",
        EnemyKind::Puppet => "Gardener's Puppet",
        EnemyKind::MothQueen => "Moth Queen",
        EnemyKind::Statue => "Watching Statue",
        EnemyKind::Boss(k) => k.name(),
    }
}

/// The key a foe's counts are saved under.
pub fn beast_key(kind: EnemyKind) -> String {
    match kind {
        EnemyKind::Boss(k) => format!("lord_{}", k.key()),
        k => beast_name(k).to_lowercase().replace(' ', "_"),
    }
}

/// A line of lore for each.
pub fn beast_lore(kind: EnemyKind) -> &'static str {
    match kind {
        EnemyKind::Hollow => "Once they kept vigils too. Now they only remember the walking.",
        EnemyKind::Ghoul => "Fast and starving, it never stops to chew.",
        EnemyKind::Moth => "Drawn to every flame, yours most of all.",
        EnemyKind::Leech => "A mouth that learned to crawl.",
        EnemyKind::Archer => "Its bowstring is a tendon. Its arrows are its own ribs.",
        EnemyKind::Bloat => "Swollen with the dead it has eaten. Do not stand close when it bursts.",
        EnemyKind::Wraith => "Half here, half somewhere colder.",
        EnemyKind::Banshee => "She wails for the dead, and the dead come to listen.",
        EnemyKind::Cultist => "Its prayer is a lit fuse.",
        EnemyKind::Gargoyle => "It sat on the cathedral roof for four hundred years. It is hungry.",
        EnemyKind::Shieldbearer => "Strike it from the side; its faith is all in front.",
        EnemyKind::Necromancer => "Every grave is a door to it, and it never knocks.",
        EnemyKind::Knight => "Its oath outlived its heart.",
        EnemyKind::Courier => "It carries a key it was never meant to lose.",
        EnemyKind::Champion => "The mist is its cloak, and the fog its herald.",
        EnemyKind::Rival => "Another keeper of the vigil, gone wrong. He hates the unwounded.",
        EnemyKind::Mimic(_) => "Not every coffer wants to be opened.",
        EnemyKind::Prop(_) => "",
        EnemyKind::EmberWolf => "The fire that took the forest still runs in packs.",
        EnemyKind::BarkHusk => "A tree that walked out of the fire, and never stopped burning inside.",
        EnemyKind::SmokeWraith => "Breathe it in and it learns your name.",
        EnemyKind::IceMonk => "His vow of silence froze with him.",
        EnemyKind::FrostCrow => "They peck at the windows of the abbey, asking to be let in.",
        EnemyKind::FrozenKnight => "Strike it and the winter inside comes out.",
        EnemyKind::ThornVine => "The roses grew teeth when the stars went out.",
        EnemyKind::Puppet => "It still trims the hedges. And anything else that grows.",
        EnemyKind::MothQueen => "Every moth in the garden answers her.",
        EnemyKind::Statue => "It never moves. Not while you are looking.",
        EnemyKind::Boss(k) => k.title(),
    }
}

impl World {
    /// Counts a felled foe for the bestiary.
    pub(crate) fn count_beast(&mut self, kind: EnemyKind) {
        if let Some(i) = beast_index(kind) {
            self.felled[i] = self.felled[i].saturating_add(1);
        }
    }

    /// What most likely felled the hero in `slot`: a lord close by, else the nearest foe.
    pub fn killer_of(&self, slot: u8) -> Option<EnemyKind> {
        let at = self.heroes.iter().find(|h| h.slot == slot)?.p.pos;
        let foes = self.enemies.iter().filter(|e| !e.dead && !matches!(e.kind, EnemyKind::Prop(_) | EnemyKind::Mimic(false)));
        let lord = foes.clone().filter(|e| e.boss && e.pos.distance(at) < 700.0).min_by(|a, b| a.pos.distance_squared(at).total_cmp(&b.pos.distance_squared(at)));
        lord.or_else(|| foes.min_by(|a, b| a.pos.distance_squared(at).total_cmp(&b.pos.distance_squared(at)))).map(|e| e.kind)
    }
}
