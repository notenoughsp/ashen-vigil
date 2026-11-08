//! The running vigil.
//!
//! The host (or the single-player game) simulates everything here. A network client keeps
//! a `World` too, but only fills it from snapshots and predicts its own hero's movement.

mod boss;
mod braziers;
mod combat;
mod duos;
pub use duos::*;
mod mutators;
pub use mutators::*;
mod bestiary;
mod events;
mod fx;
mod hero;
pub use bestiary::*;
mod fields;
pub use fields::*;
mod passages;
mod pings;
mod usurper;
pub use pings::*;
mod props;
pub use passages::*;
pub use props::*;
mod pickups;
mod quests;
pub use quests::*;
mod omens;
pub use omens::*;
mod spawn;
pub use braziers::*;
mod skills;
mod weapons;

pub use events::*;
pub use hero::*;

use crate::data::*;
use macroquad::prelude::*;
use macroquad::rand::gen_range;
use std::collections::VecDeque;
use std::f32::consts::TAU;

pub const VICTORY_TIME: f32 = 30.0 * 60.0;
/// Most foes alive at once. Hordes and swarms grow with time, so without a ceiling an
/// endless night would pile up thousands.
pub const MAX_ENEMIES: usize = 1600;
/// How far past `MAX_ENEMIES` the lords' summoned adds may go; single elites, couriers and
/// champions still find room above that.
pub const LORD_ADDS_OVER: usize = 80;
/// Boss health grows by this factor every minute past 5:00.
pub const BOSS_GROWTH: f32 = 1.17;
/// How much tougher ordinary foes get every minute after the fifth, on top of the old ramp.
pub const ENEMY_GROWTH: f32 = 1.09;
/// The most of a lord's health that can go in one second: every lord fight lasts at least
/// twenty seconds, however strong the heroes.
pub const BOSS_SECOND_CAP: f32 = 0.05;
/// The chance that a lord (from the second on) comes with a twin, the share of health each
/// of the two has, and what of a blow reaches one while the other still stands.
pub const TWIN_CHANCE: f32 = 0.2;
pub const TWIN_HP: f32 = 0.7;
pub const TWIN_GUARD: f32 = 0.75;
/// The fog ring round a lord, and how much wider it grows for each other lord, champion or
/// Sven in the fight.
pub const ARENA_RADIUS: f32 = 560.0;
pub const ARENA_GROWTH: f32 = 150.0;
/// Seconds a boss fight can last before its fog ring lifts.
pub const BOSS_FOG_TIME: f32 = 180.0;
/// A lone grave in the far south-west corner of the field, for those who wander.
pub const LOST_GRAVE: Vec2 = Vec2::new(-WORLD_HALF + 260.0, WORLD_HALF - 240.0);
/// The cathedral's secret instead: a cracked bell fallen in the far north-east corner.
pub const FALLEN_BELL: Vec2 = Vec2::new(WORLD_HALF - 250.0, -WORLD_HALF + 260.0);
/// The flesh pits' secret: a great heart still beating in the far south-east corner.
pub const BEATING_HEART: Vec2 = Vec2::new(WORLD_HALF - 260.0, WORLD_HALF - 250.0);
/// Seconds of standing still, with no foe near, before a hero sits down by a fire.
pub const REST_AFTER: f32 = 20.0;
/// Health of an enemy projectile at the start; it grows like the foes' health.
pub const BULLET_HP: f32 = 6.0;
/// Pickups on the floor before old soul orbs start merging...
pub const MAX_PICKUPS: usize = 500;
/// ...and how far a merge brings the count back down.
pub const MERGED_PICKUPS: usize = 350;
/// Sounds are at full volume this close to the listener...
pub const HEAR_FULL: f32 = 450.0;
/// ...and silent from here on, about a screen away.
pub const HEAR_MAX: f32 = 1100.0;
pub const BOSS_INTERVAL: f32 = 5.0 * 60.0;
/// An Ash Shrine stirs this often, twice between two lords.
pub const SHRINE_INTERVAL: f32 = 2.5 * 60.0;
/// The most of a lord's health one hit can take.
pub const BOSS_HIT_CAP: f32 = 0.025;
/// Seconds of warning before a lord arrives (not in the boss rush).
pub const BOSS_WARNING: f32 = 5.0;
/// Seconds before a lord comes that its song begins.
pub const MUSIC_LEAD: f32 = 10.0;
/// Lords to defeat in a boss rush, and the breather between two of them.
pub const RUSH_LORDS: u32 = 12;
pub const RUSH_BREATHER: f32 = 3.0;
/// Level-ups a boss rush begins with, and each fallen lord grants.
pub const RUSH_HEAD_START: u32 = 6;
pub const RUSH_LEVELS_PER_LORD: u32 = 3;
/// Health of the first lord of a boss rush, before difficulty and co-op, and how much
/// more each following lord has.
pub const RUSH_LORD_HP: f32 = 900.0;
pub const RUSH_LORD_GROWTH: f32 = 1.6;
pub const RESUME_DELAY: f32 = 2.5;
/// The countdown never gets shorter than this...
pub const MIN_RESUME_DELAY: f32 = 0.7;
/// ...and shrinks by this factor every five levels.
const RESUME_SHRINK: f32 = 0.95;
pub const MAX_HEROES: usize = 4;
pub const REVIVE_TIME: f32 = 3.0;
/// How far a fallen hero's ghost may drift from its body, and how fast.
pub const GHOST_LEASH: f32 = 240.0;
pub const GHOST_SPEED: f32 = 190.0;
/// Seconds before a spent ember can rekindle the same hero again.
pub const EMBER_COOLDOWN: f32 = 30.0;
const MAX_DECALS: usize = 900;
/// The field is a square from -WORLD_HALF to WORLD_HALF, walled in, with the abyss beyond.
pub const WORLD_HALF: f32 = 3600.0;
/// A boss rush is fought on a small walled field instead, with no fog ring around its lords.
pub const RUSH_HALF: f32 = 1200.0;

thread_local! {
    /// Half the side of the field in play: every world method that runs sets it (see
    /// `World::fit_field`), so the walls, spawns and drawing agree on one size.
    static FIELD_HALF: std::cell::Cell<f32> = const { std::cell::Cell::new(WORLD_HALF) };
}

/// Sets the field in play, for drawing a world without changing it.
pub fn set_field_half(h: f32) {
    FIELD_HALF.with(|c| c.set(h));
}

/// Half the side of the field in play.
pub fn field_half() -> f32 {
    FIELD_HALF.with(|h| h.get())
}

pub fn inside_world(p: Vec2, margin: f32) -> bool {
    let h = field_half();
    p.x.abs() < h - margin && p.y.abs() < h - margin
}

pub fn clamp_world(p: Vec2, margin: f32) -> Vec2 {
    let m = field_half() - margin;
    vec2(p.x.clamp(-m, m), p.y.clamp(-m, m))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BossKind {
    Gravewarden,
    Matriarch,
    Hermit,
    Sovereign,
    Bride,
    BogMother,
    Penitent,
    Choir,
    Seraph,
    Hound,
    BurntOak,
    SnowAbbess,
    Gardener,
}

/// The field a vigil is kept on: its floor, its props, its foes and its lords.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Biome {
    #[default]
    Graveyard,
    Cathedral,
    Flesh,
    Ashwood,
    Abbey,
    Garden,
}

pub const ALL_BIOMES: [Biome; 6] = [Biome::Graveyard, Biome::Cathedral, Biome::Flesh, Biome::Ashwood, Biome::Abbey, Biome::Garden];

impl Biome {
    pub fn name(self) -> &'static str {
        match self {
            Biome::Graveyard => "The Graveyard",
            Biome::Cathedral => "The Hallowed Cathedral",
            Biome::Flesh => "The Flesh Pits",
            Biome::Ashwood => "The Ashwood",
            Biome::Abbey => "The Frozen Abbey",
            Biome::Garden => "The Starless Garden",
        }
    }

    /// The arsenal this field's level-ups draw from (see `data::ALL_WEAPONS`).
    pub fn arsenal(self) -> usize {
        self as usize
    }

    /// The older field a newer one is akin to, whose songs it borrows while it has none.
    pub fn akin(self) -> usize {
        match self {
            Biome::Graveyard | Biome::Ashwood => 0,
            Biome::Cathedral | Biome::Abbey => 1,
            Biome::Flesh | Biome::Garden => 2,
        }
    }

    pub fn desc(self) -> &'static str {
        match self {
            Biome::Graveyard => "Old stones, older dead. Archers and the restless.",
            Biome::Cathedral => "Light through coloured glass. Zealots, knights and gargoyles.",
            Biome::Flesh => "Living ground and rivers of blood. Leeches, ghouls and bloated things.",
            Biome::Ashwood => "A burnt forest on smouldering ground. Embers burn all who stand in them. Ember wolves, bark husks, smoke wraiths.",
            Biome::Abbey => "A cloister in endless winter. Heroes slide on its ice, and blizzards freeze what they strike. Ice monks, frost crows, frozen knights.",
            Biome::Garden => "Hedges under a sky without stars. Its statues move only while no one looks. Thorn vines, gardener puppets, moth queens.",
        }
    }

    pub fn from_u8(v: u8) -> Biome {
        ALL_BIOMES.get(v as usize).copied().unwrap_or_default()
    }

    /// The lords at 5, 10, 15, 20 and 25 minutes; the Sovereign always ends the night.
    pub fn lords(self) -> [BossKind; 5] {
        use BossKind::*;
        match self {
            Biome::Graveyard => [Gravewarden, Hound, Matriarch, BogMother, Hermit],
            Biome::Cathedral => [Seraph, Choir, Hermit, Penitent, Bride],
            Biome::Flesh => [Matriarch, Hound, Bride, BogMother, Penitent],
            Biome::Ashwood => [Hound, Gravewarden, BurntOak, Penitent, Matriarch],
            Biome::Abbey => [Seraph, Hermit, SnowAbbess, Choir, Bride],
            Biome::Garden => [Matriarch, Bride, Gardener, Hound, BogMother],
        }
    }

    /// How much more (or less) often a kind of foe shows up here.
    pub fn weight(self, kind: EnemyKind) -> f32 {
        use EnemyKind::*;
        match (self, kind) {
            (Biome::Graveyard, Hollow | Archer | Necromancer | Moth) => 1.5,
            (Biome::Graveyard, Leech) => 0.4,
            (Biome::Cathedral, Cultist | Knight | Gargoyle | Banshee | Shieldbearer) => 1.7,
            (Biome::Cathedral, Ghoul | Leech) => 0.5,
            (Biome::Flesh, Ghoul | Bloat | Leech) => 1.8,
            (Biome::Flesh, Archer | Banshee | Gargoyle) => 0.5,
            // Each newer field's own foes live there and nowhere else.
            (Biome::Ashwood, EmberWolf | BarkHusk | SmokeWraith) => 1.8,
            (Biome::Abbey, IceMonk | FrostCrow | FrozenKnight) => 1.8,
            (Biome::Garden, ThornVine | Puppet | MothQueen | Statue) => 1.8,
            (_, EmberWolf | BarkHusk | SmokeWraith | IceMonk | FrostCrow | FrozenKnight | ThornVine | Puppet | MothQueen | Statue) => 0.0,
            (Biome::Ashwood, Wraith | Moth | Leech) => 0.4,
            (Biome::Abbey, Ghoul | Leech | Bloat) => 0.4,
            (Biome::Garden, Archer | Knight | Leech) => 0.4,
            _ => 1.0,
        }
    }
}

/// Every lord; a boss rush meets them in this order, the Sovereign last.
pub const ALL_LORDS: [BossKind; 13] = [
    BossKind::Gravewarden,
    BossKind::Hound,
    BossKind::Matriarch,
    BossKind::BurntOak,
    BossKind::BogMother,
    BossKind::Bride,
    BossKind::SnowAbbess,
    BossKind::Hermit,
    BossKind::Seraph,
    BossKind::Gardener,
    BossKind::Choir,
    BossKind::Penitent,
    BossKind::Sovereign,
];

impl BossKind {
    /// The lord's name in folder names, e.g. "bride" for `assets/music/boss/bride`.
    pub fn key(self) -> &'static str {
        match self {
            BossKind::Gravewarden => "gravewarden",
            BossKind::Matriarch => "matriarch",
            BossKind::Hermit => "hermit",
            BossKind::Sovereign => "sovereign",
            BossKind::Bride => "bride",
            BossKind::BogMother => "bogmother",
            BossKind::Penitent => "penitent",
            BossKind::Choir => "choir",
            BossKind::Seraph => "seraph",
            BossKind::Hound => "hound",
            BossKind::BurntOak => "burntoak",
            BossKind::SnowAbbess => "snowabbess",
            BossKind::Gardener => "gardener",
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            BossKind::Gravewarden => "The Gravewarden",
            BossKind::Matriarch => "Pale Matriarch",
            BossKind::Hermit => "The Tolling Hermit",
            BossKind::Sovereign => "Ashen Sovereign",
            BossKind::Bride => "The Drowned Bride",
            BossKind::BogMother => "The Bog Mother",
            BossKind::Penitent => "The Iron Penitent",
            BossKind::Choir => "The Hollow Choir",
            BossKind::Seraph => "The Stone Seraph",
            BossKind::Hound => "The Pale Hound",
            BossKind::BurntOak => "The Burnt Oak",
            BossKind::SnowAbbess => "The Snow Abbess",
            BossKind::Gardener => "The Gardener",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            BossKind::Gravewarden => "Keeper of the Sunken Graves",
            BossKind::Matriarch => "Mother of the Gloom Moths",
            BossKind::Hermit => "He Who Rings the Last Hour",
            BossKind::Sovereign => "Last Lord of the Cinder Throne",
            BossKind::Bride => "Still Waiting at the Altar of the Deep",
            BossKind::BogMother => "She Who Swallows the Lost",
            BossKind::Penitent => "Chained to a Sin Without End",
            BossKind::Choir => "Three Voices, No Throat",
            BossKind::Seraph => "Fallen Guardian of the Bell Tower",
            BossKind::Hound => "The Hunger That Follows Footsteps",
            BossKind::BurntOak => "Root of the Forest That Burned",
            BossKind::SnowAbbess => "Mother of the Long Winter",
            BossKind::Gardener => "Keeper of the Hedges Without Stars",
        }
    }

    /// What it says when it arrives, at its second phase, at its third, and as it falls.
    pub fn lines(self) -> [&'static str; 4] {
        match self {
            BossKind::Gravewarden => ["Another body for the earth.", "The graves are hungry tonight!", "I will bury you myself!", "...the earth... takes me back..."],
            BossKind::Matriarch => ["So many little lights out tonight. My children will feast.", "", "", ""],
            BossKind::Hermit => ["Can you hear it? The last hour is tolling.", "Every bell rings for you now.", "", "The bell... falls silent..."],
            BossKind::Sovereign => {
                ["Kneel. The dawn will not come for you.", "I have burned brighter flames than you!", "Then let the whole world burn with me!", "The throne... is ash..."]
            }
            BossKind::Bride => ["He promised he would come back for me...", "You are not him!", "Then drown with me, and never leave!", "At last... the water is warm..."],
            BossKind::BogMother => ["The bog... takes me home...", "", "", ""],
            BossKind::Penitent => {
                ["Every link of this chain is a sin. Count them.", "Pain is the only prayer that is answered!", "Break me, and be broken!", "Free... at last... free..."]
            }
            BossKind::Choir => ["We sing for the dead. Now we sing for you.", "Louder! LOUDER!", "Our last hymn is your name.", "...and the silence... is beautiful..."],
            BossKind::Seraph => ["The tower fell... I did not.", "Stone does not tire, little flame.", "", "...crumbling... like the rest..."],
            BossKind::Hound => ["*sniffing* ...fresh...", "*a howl that shakes the stones*", "", "*a last whimper*"],
            BossKind::BurntOak => ["Every ring of me remembers the fire.", "Then burn with me!", "", "...ash... returns to the soil..."],
            BossKind::SnowAbbess => ["Kneel, child. The winter hears confession.", "Your breath is a sin. I will still it.", "", "...spring... at last..."],
            BossKind::Gardener => ["Mind the hedges. They mind you.", "You trampled my roses!", "", "...who will tend... the garden..."],
        }
    }

    /// The lords that grow a third phase at a quarter of their health.
    pub fn has_third_phase(self) -> bool {
        matches!(self, BossKind::Gravewarden | BossKind::Sovereign | BossKind::Bride | BossKind::Penitent | BossKind::Choir)
    }

    pub fn from_u8(v: u8) -> BossKind {
        ALL_LORDS.iter().copied().find(|k| *k as u8 == v).unwrap_or(BossKind::Gravewarden)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum EnemyKind {
    Hollow,
    Ghoul,
    Moth,
    Archer,
    Bloat,
    Wraith,
    Knight,
    /// Runs at you, lights a bomb, explodes.
    Cultist,
    /// Blocks most damage that comes from the front.
    Shieldbearer,
    /// Keeps away and raises the dead around it.
    Necromancer,
    /// Small, fast, comes in swarms.
    Leech,
    /// A stone statue that lunges at its prey.
    Gargoyle,
    /// Keeps away and wails rings of slow bullets.
    Banshee,
    /// Flees with the key to a vault.
    Courier,
    /// The mist's champion: charges and slams rings of bullets.
    Champion,
    /// A dark hero who hunts whoever has gone too long without a wound.
    Rival,
    /// A coffer with teeth: it lies still like any coffer until a hero comes near (false),
    /// then hops after them and bites (true). It spills a hoard when it falls.
    Mimic(bool),
    /// A breakable thing on the field (see `props`), variant 0 to 2; what it looks like
    /// depends on the field.
    Prop(u8),
    /// The Ashwood: a wolf of embers that hunts in packs and leaps.
    EmberWolf,
    /// The Ashwood: a husk of burnt bark, slow and hard to fell.
    BarkHusk,
    /// The Ashwood: smoke in a shape, fading in and out; it bursts into embers.
    SmokeWraith,
    /// The Frozen Abbey: a monk who keeps away and throws shards of ice.
    IceMonk,
    /// The Frozen Abbey: a crow of frost, in flocks.
    FrostCrow,
    /// The Frozen Abbey: a knight frozen in its armour; it shatters into shards.
    FrozenKnight,
    /// The Starless Garden: a creeping vine that spits thorns.
    ThornVine,
    /// The Starless Garden: a gardener's puppet that charges with its shears.
    Puppet,
    /// The Starless Garden: a great moth that keeps away and calls moths.
    MothQueen,
    /// The Starless Garden: a statue that moves only while no hero looks at it.
    Statue,
    Boss(BossKind),
}

impl EnemyKind {
    pub fn to_u8(self) -> u8 {
        match self {
            EnemyKind::Hollow => 0,
            EnemyKind::Ghoul => 1,
            EnemyKind::Moth => 2,
            EnemyKind::Archer => 3,
            EnemyKind::Bloat => 4,
            EnemyKind::Wraith => 5,
            EnemyKind::Knight => 6,
            EnemyKind::Cultist => 11,
            EnemyKind::Shieldbearer => 12,
            EnemyKind::Necromancer => 13,
            EnemyKind::Leech => 20,
            EnemyKind::Gargoyle => 21,
            EnemyKind::Banshee => 22,
            EnemyKind::Courier => 23,
            EnemyKind::Champion => 24,
            EnemyKind::Rival => 25,
            EnemyKind::Mimic(awake) => 26 + awake as u8,
            EnemyKind::Prop(v) => 28 + v.min(2),
            EnemyKind::EmberWolf => 31,
            EnemyKind::BarkHusk => 32,
            EnemyKind::SmokeWraith => 33,
            EnemyKind::IceMonk => 34,
            EnemyKind::FrostCrow => 35,
            EnemyKind::FrozenKnight => 36,
            EnemyKind::ThornVine => 37,
            EnemyKind::Puppet => 38,
            EnemyKind::MothQueen => 39,
            EnemyKind::Statue => 40,
            // The first four lords keep their old numbers.
            EnemyKind::Boss(b) if (b as u8) < 4 => 7 + b as u8,
            EnemyKind::Boss(b) if (b as u8) < 10 => 10 + b as u8,
            EnemyKind::Boss(b) => 100 + b as u8,
        }
    }

    pub fn from_u8(v: u8) -> EnemyKind {
        match v {
            0 => EnemyKind::Hollow,
            1 => EnemyKind::Ghoul,
            2 => EnemyKind::Moth,
            3 => EnemyKind::Archer,
            4 => EnemyKind::Bloat,
            5 => EnemyKind::Wraith,
            6 => EnemyKind::Knight,
            11 => EnemyKind::Cultist,
            12 => EnemyKind::Shieldbearer,
            13 => EnemyKind::Necromancer,
            20 => EnemyKind::Leech,
            21 => EnemyKind::Gargoyle,
            22 => EnemyKind::Banshee,
            23 => EnemyKind::Courier,
            24 => EnemyKind::Champion,
            25 => EnemyKind::Rival,
            26 => EnemyKind::Mimic(false),
            27 => EnemyKind::Mimic(true),
            28..=30 => EnemyKind::Prop(v - 28),
            31 => EnemyKind::EmberWolf,
            32 => EnemyKind::BarkHusk,
            33 => EnemyKind::SmokeWraith,
            34 => EnemyKind::IceMonk,
            35 => EnemyKind::FrostCrow,
            36 => EnemyKind::FrozenKnight,
            37 => EnemyKind::ThornVine,
            38 => EnemyKind::Puppet,
            39 => EnemyKind::MothQueen,
            40 => EnemyKind::Statue,
            100..=120 => EnemyKind::Boss(BossKind::from_u8(v - 100)),
            7..=10 => EnemyKind::Boss(BossKind::from_u8(v - 7)),
            14..=19 => EnemyKind::Boss(BossKind::from_u8(v - 10)),
            _ => EnemyKind::Hollow,
        }
    }

    /// Body radius, so clients need not be told it.
    pub fn radius(self, elite: bool) -> f32 {
        let r = match self {
            EnemyKind::Hollow => 12.0,
            EnemyKind::Ghoul => 10.0,
            EnemyKind::Moth => 7.0,
            EnemyKind::Archer => 11.0,
            EnemyKind::Bloat => 20.0,
            EnemyKind::Wraith => 13.0,
            EnemyKind::Knight => 16.0,
            EnemyKind::Cultist => 10.0,
            EnemyKind::Shieldbearer => 17.0,
            EnemyKind::Necromancer => 13.0,
            EnemyKind::Leech => 7.0,
            EnemyKind::Gargoyle => 14.0,
            EnemyKind::Banshee => 12.0,
            EnemyKind::Courier => 10.0,
            EnemyKind::Champion => 18.0,
            EnemyKind::Rival => 13.0,
            EnemyKind::Mimic(_) => 15.0,
            EnemyKind::Prop(_) => 14.0,
            EnemyKind::EmberWolf => 11.0,
            EnemyKind::BarkHusk => 18.0,
            EnemyKind::SmokeWraith => 12.0,
            EnemyKind::IceMonk => 12.0,
            EnemyKind::FrostCrow => 7.0,
            EnemyKind::FrozenKnight => 16.0,
            EnemyKind::ThornVine => 13.0,
            EnemyKind::Puppet => 13.0,
            EnemyKind::MothQueen => 16.0,
            EnemyKind::Statue => 15.0,
            EnemyKind::Boss(BossKind::BurntOak) => 54.0,
            EnemyKind::Boss(BossKind::SnowAbbess) => 40.0,
            EnemyKind::Boss(BossKind::Gardener) => 42.0,
            EnemyKind::Boss(BossKind::Gravewarden) => 44.0,
            EnemyKind::Boss(BossKind::Matriarch) => 38.0,
            EnemyKind::Boss(BossKind::Hermit) => 34.0,
            EnemyKind::Boss(BossKind::Sovereign) => 52.0,
            EnemyKind::Boss(BossKind::Bride) => 36.0,
            EnemyKind::Boss(BossKind::BogMother) => 50.0,
            EnemyKind::Boss(BossKind::Penitent) => 46.0,
            EnemyKind::Boss(BossKind::Choir) => 36.0,
            EnemyKind::Boss(BossKind::Seraph) => 40.0,
            EnemyKind::Boss(BossKind::Hound) => 34.0,
        };
        if elite {
            r * 1.6
        } else {
            r
        }
    }

    /// Only these fade in and out, so only they send an alpha.
    pub fn has_alpha(self) -> bool {
        matches!(
            self,
            EnemyKind::Wraith | EnemyKind::Banshee | EnemyKind::SmokeWraith | EnemyKind::Boss(BossKind::Hermit | BossKind::Choir | BossKind::Seraph | BossKind::BogMother)
        )
    }

    /// What comes out when it is cut.
    pub fn blood(self) -> Color {
        match self {
            EnemyKind::Moth => Color::from_rgba(150, 140, 110, 255),
            EnemyKind::Wraith => Color::from_rgba(70, 110, 190, 255),
            EnemyKind::Bloat => Color::from_rgba(90, 110, 30, 255),
            EnemyKind::Archer => Color::from_rgba(110, 100, 90, 255),
            EnemyKind::Gargoyle => Color::from_rgba(110, 108, 104, 255),
            EnemyKind::Banshee => Color::from_rgba(150, 170, 200, 255),
            EnemyKind::Leech => Color::from_rgba(90, 10, 20, 255),
            EnemyKind::EmberWolf | EnemyKind::SmokeWraith => Color::from_rgba(80, 70, 66, 255),
            EnemyKind::BarkHusk => Color::from_rgba(70, 50, 34, 255),
            EnemyKind::IceMonk | EnemyKind::FrostCrow | EnemyKind::FrozenKnight => Color::from_rgba(150, 200, 235, 255),
            EnemyKind::ThornVine => Color::from_rgba(60, 100, 40, 255),
            EnemyKind::Puppet => Color::from_rgba(120, 100, 80, 255),
            EnemyKind::MothQueen => Color::from_rgba(150, 140, 110, 255),
            EnemyKind::Statue => Color::from_rgba(110, 108, 104, 255),
            _ => Color::from_rgba(120, 8, 10, 255),
        }
    }
}

pub struct Enemy {
    pub id: u32,
    pub kind: EnemyKind,
    pub pos: Vec2,
    pub knock: Vec2,
    pub hp: f32,
    pub max_hp: f32,
    pub speed: f32,
    pub dmg: f32,
    pub r: f32,
    pub xp: f32,
    pub flash: f32,
    pub burn: f32,
    pub burn_dps: f32,
    pub burn_acc: f32,
    pub burn_src: Src,
    pub slow: f32,
    pub last_hit: [f32; ALL_WEAPONS.len()],
    pub timer: f32,
    pub elite: bool,
    pub boss: bool,
    pub dead: bool,
    pub phase: f32,
    pub alpha: f32,
    /// Cultist bomb: seconds until it goes off, negative while unlit.
    pub fuse: f32,
    /// Velocity estimated from snapshots, used by clients to extrapolate.
    pub net_vel: Vec2,
    /// Sven's dance: 0 circling, 1 winding up a lunge, 2 lunging, 3 recovering; and the
    /// direction the lunge was locked to when he wound up.
    pub step: u8,
    pub lunge: Vec2,
    /// A hero this foe hunts instead of the nearest (the rival).
    pub hunt: Option<usize>,
    /// Seconds until it leaves the field by itself (the courier, the rival).
    pub life_left: f32,
    /// An elite's trait, see `Affix`; ordinary foes have none.
    pub affix: Affix,
    /// Seconds it stays frozen: it does not move, and takes more from every blow.
    pub freeze: f32,
    /// Thorns left in it: seconds it bleeds on, how hard, and from whom.
    pub bleed: f32,
    pub bleed_dps: f32,
    pub bleed_acc: f32,
    pub bleed_src: Src,
    /// Seconds it is held fast by roots or brambles.
    pub held: f32,
    /// Cold gathered from ice needles and meltwater; at 1 it freezes.
    pub chill: f32,
}

/// How much more a frozen foe takes from every blow.
pub const FROZEN_TAKES: f32 = 1.3;

/// What makes an elite more than big: each elite gets one, shown by a coloured mark above its
/// head.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Affix {
    #[default]
    None,
    /// Yellow: far faster.
    Swift,
    /// Orange: bursts into a ring of bullets when it falls.
    Volatile,
    /// Green: mends the foes around it.
    Mender,
    /// Blue: shrugs off much of every blow.
    Warded,
    /// Violet: steps through the air toward its prey now and then.
    Blinker,
}

pub const ALL_AFFIXES: [Affix; 5] = [Affix::Swift, Affix::Volatile, Affix::Mender, Affix::Warded, Affix::Blinker];

impl Affix {
    pub fn name(self) -> &'static str {
        match self {
            Affix::None => "",
            Affix::Swift => "Swift",
            Affix::Volatile => "Volatile",
            Affix::Mender => "Mender",
            Affix::Warded => "Warded",
            Affix::Blinker => "Blinker",
        }
    }

    pub fn desc(self) -> &'static str {
        match self {
            Affix::None => "",
            Affix::Swift => "60% faster",
            Affix::Volatile => "bursts into a ring of bullets when it falls",
            Affix::Mender => "mends the foes around it",
            Affix::Warded => "takes 40% less from every blow",
            Affix::Blinker => "steps through the air toward you",
        }
    }

    pub fn color(self) -> Color {
        match self {
            Affix::None => WHITE,
            Affix::Swift => Color::from_rgba(255, 225, 60, 255),
            Affix::Volatile => Color::from_rgba(255, 130, 30, 255),
            Affix::Mender => Color::from_rgba(90, 230, 110, 255),
            Affix::Warded => Color::from_rgba(80, 160, 255, 255),
            Affix::Blinker => Color::from_rgba(200, 110, 255, 255),
        }
    }

    pub fn to_u8(self) -> u8 {
        match self {
            Affix::None => 0,
            Affix::Swift => 1,
            Affix::Volatile => 2,
            Affix::Mender => 3,
            Affix::Warded => 4,
            Affix::Blinker => 5,
        }
    }

    pub fn from_u8(v: u8) -> Affix {
        ALL_AFFIXES.get((v as usize).wrapping_sub(1)).copied().unwrap_or(Affix::None)
    }
}

/// Who dealt damage with what, for crits, lifesteal and the damage meter.
#[derive(Clone, Copy, PartialEq, Debug)]
pub struct Src {
    pub hero: usize,
    pub weapon: WeaponKind,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PKind {
    Dagger,
    Orb,
    Axe,
    Bolt,
    Lance,
    Quarrel,
    Nail,
    Shard,
    Flask,
    Gargoyle,
    Rune,
    Ember,
    Tooth,
    Sac,
    Jaw,
    Mote,
    Tendril,
    /// The spark sling's glowing ember: it bursts into sparks.
    Sling,
    Spark,
    /// The bloom cannon's seed: it lands where it was thrown and grows into a poison flower.
    Seed,
    /// A foe's bullet the mirror pond sent back.
    Mirror,
    /// The charred hatchet, spiralling out round its thrower.
    Hatchet,
    /// Burning roots crawling along the ground, leaving fire behind.
    Root,
    /// A burning brand lobbed onto the foes.
    Brand,
    /// An ember of the ember burst; it falls to the ground burning.
    Ember2,
    /// An ice needle.
    Needle,
    /// The snow owl, diving at the strongest foe and back.
    Owl,
    /// A rose thorn; it springs on to the foes nearby.
    Thorn,
    /// The garden shears, flung out to snap shut and back.
    Shears,
    /// A wisp of pollen, seeking.
    Wisp,
    /// Creeping roots that hold what they grow under.
    Vine,
    /// A flask of meltwater, lobbed; it freezes over where it lands.
    Meltwater,
}

impl PKind {
    pub fn to_u8(self) -> u8 {
        self as u8
    }

    pub fn from_u8(v: u8) -> PKind {
        match v {
            0 => PKind::Dagger,
            1 => PKind::Orb,
            2 => PKind::Axe,
            3 => PKind::Bolt,
            4 => PKind::Lance,
            6 => PKind::Nail,
            7 => PKind::Shard,
            8 => PKind::Flask,
            9 => PKind::Gargoyle,
            10 => PKind::Rune,
            11 => PKind::Ember,
            17 => PKind::Sling,
            18 => PKind::Spark,
            19 => PKind::Seed,
            20 => PKind::Mirror,
            21 => PKind::Hatchet,
            22 => PKind::Root,
            23 => PKind::Brand,
            24 => PKind::Ember2,
            25 => PKind::Needle,
            26 => PKind::Owl,
            27 => PKind::Thorn,
            28 => PKind::Shears,
            29 => PKind::Wisp,
            30 => PKind::Vine,
            31 => PKind::Meltwater,
            12 => PKind::Tooth,
            13 => PKind::Sac,
            14 => PKind::Jaw,
            15 => PKind::Mote,
            16 => PKind::Tendril,
            _ => PKind::Quarrel,
        }
    }
}

pub struct Proj {
    pub pos: Vec2,
    pub vel: Vec2,
    pub dir: Vec2,
    pub r: f32,
    pub dmg: f32,
    pub pierce: i32,
    pub life: f32,
    pub age: f32,
    pub kind: PKind,
    pub evo: bool,
    pub area: f32,
    pub src: Src,
    pub hits: Vec<u32>,
    pub dead: bool,
}

/// Enemy projectile. Red, and players can shoot or cut it down.
pub struct EBullet {
    pub pos: Vec2,
    pub vel: Vec2,
    pub r: f32,
    pub dmg: f32,
    pub life: f32,
    pub homing: f32,
    /// Damage it takes to shoot down; grows with the foes.
    pub hp: f32,
    /// Shrugs off hits for a moment after one, so a passing blade counts once.
    pub hit_cd: f32,
}

#[derive(Clone, Copy, Debug)]
pub enum FxKind {
    Slash {
        ang: f32,
        radius: f32,
        evo: bool,
        owner: u8,
    },
    Sweep {
        radius: f32,
        evo: bool,
        owner: u8,
    },
    Explosion {
        r: f32,
        color: Color,
    },
    Lightning {
        from: Vec2,
    },
    Ring {
        r: f32,
        color: Color,
    },
    Ghost,
    /// The rosary's lash from `pos` to `to`.
    Whip {
        to: Vec2,
    },
    /// The judgement blade falling on `pos`.
    Sword {
        r: f32,
    },
    /// The crozier's sweep round its owner.
    Crook {
        radius: f32,
        owner: u8,
    },
    /// An organ pipe bursting from the ground.
    Pipe {
        r: f32,
    },
    /// The cleaver's red cut, round its owner.
    Gash {
        ang: f32,
        radius: f32,
        owner: u8,
    },
    /// An eye's glare from `pos` to `to`.
    Gaze {
        to: Vec2,
    },
    /// Spikes of bone bursting up.
    Spike {
        r: f32,
    },
    /// Ribs bursting up round their owner and snapping shut.
    Ribs {
        radius: f32,
        owner: u8,
    },
    /// The spine's lash from `pos` to `to`.
    Lash {
        to: Vec2,
    },
    /// The ash sickle's wide burning sweep round its owner (both ways once evolved).
    Sickle {
        ang: f32,
        radius: f32,
        evo: bool,
        owner: u8,
    },
    /// A cone of snowburn breathed from its owner.
    Breath {
        ang: f32,
        radius: f32,
        owner: u8,
    },
    /// The thorn whip's lash in an arc round its owner.
    Thorns {
        ang: f32,
        radius: f32,
        owner: u8,
    },
    /// The ice lance's beam from `pos` to `to`.
    Frost {
        to: Vec2,
    },
    /// Spikes of ice bursting up where the ice lance passed.
    IceSpikes {
        r: f32,
    },
    /// A spark leaping from `pos` to `to`: a firefly's sting, or a moth's.
    Zap {
        to: Vec2,
        poison: bool,
    },
    /// A hook flung from `pos` to `to` and hauled back.
    Hook {
        to: Vec2,
    },
    /// A swing that drags foes in toward its owner.
    Haul {
        ang: f32,
        radius: f32,
        owner: u8,
    },
    /// A straight strike from `pos` to `to`: 0 a burning pike, 1 an icicle scourge,
    /// 2 a lashing vine.
    Strike {
        to: Vec2,
        style: u8,
    },
}

pub struct Fx {
    pub pos: Vec2,
    pub life: f32,
    pub max_life: f32,
    pub kind: FxKind,
}

/// What a patch of ground does: burns, hexes, sears with holy water, holds a candle that
/// spits flame, or chokes with censer smoke.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PoolKind {
    Fire,
    Hex,
    Holy,
    Candle,
    Smoke,
    /// The bleeding steps' trail.
    Blood,
    /// The ash sickle's burning trail.
    Cinders,
    /// A poison flower from the bloom cannon: a bud a moment, then a bloom.
    Bloom,
    /// An ice spike the ice lance left: it bursts once, a moment later.
    IceSpike,
    /// A burning pike's victim: it blows up a moment later.
    FireBurst,
    /// A pool of meltwater freezing over.
    Frostpool,
    /// A falling icicle's shadow; it strikes when its wait is over.
    Icicle,
    /// A frost rune laid on the ground: the first foe to step on it sets it off.
    RuneTrap,
    /// A pane of ice standing upright: it stops bullets and bites what touches it.
    Pane,
    /// Brambles that hold whatever stands in them, and make it bleed.
    Bramble,
    /// A cloud of poisonous pollen.
    Pollen,
    /// A nightshade flower: the first foe near it makes it burst into poison.
    Nightshade,
}

impl PoolKind {
    pub fn from_u8(v: u8) -> PoolKind {
        [
            PoolKind::Fire,
            PoolKind::Hex,
            PoolKind::Holy,
            PoolKind::Candle,
            PoolKind::Smoke,
            PoolKind::Blood,
            PoolKind::Cinders,
            PoolKind::Bloom,
            PoolKind::IceSpike,
            PoolKind::FireBurst,
            PoolKind::Frostpool,
            PoolKind::Icicle,
            PoolKind::RuneTrap,
            PoolKind::Pane,
            PoolKind::Bramble,
            PoolKind::Pollen,
            PoolKind::Nightshade,
        ]
        .get(v as usize)
        .copied()
        .unwrap_or(PoolKind::Fire)
    }
}

/// Burning ground from orbs and hex brands, and the cathedral's pools and candles.
pub struct FirePool {
    pub pos: Vec2,
    pub r: f32,
    pub life: f32,
    pub max_life: f32,
    pub dps: f32,
    pub tick: f32,
    pub src: Src,
    pub kind: PoolKind,
}

pub struct Shockwave {
    pub pos: Vec2,
    pub r: f32,
    pub max_r: f32,
    pub speed: f32,
    pub dmg: f32,
    pub hit: [bool; MAX_HEROES],
}

pub struct Pillar {
    pub pos: Vec2,
    pub r: f32,
    pub delay: f32,
    pub max_delay: f32,
    pub dmg: f32,
    pub after: f32,
}

pub struct Telegraph {
    pub a: Vec2,
    pub b: Vec2,
    pub width: f32,
    pub life: f32,
    pub max_life: f32,
    pub line: bool,
}

pub struct Particle {
    pub pos: Vec2,
    pub vel: Vec2,
    pub life: f32,
    pub max_life: f32,
    pub size: f32,
    pub color: Color,
    pub glow: bool,
    /// Blood drops leave a stain where they land.
    pub stain: bool,
}

#[derive(Clone, Copy, Debug)]
pub enum DecalKind {
    Blood,
    Corpse(EnemyKind),
    Scorch,
}

/// Something left on the ground: blood, bodies, burn marks.
pub struct Decal {
    pub pos: Vec2,
    pub r: f32,
    pub rot: f32,
    pub color: Color,
    pub kind: DecalKind,
    pub life: f32,
    pub max_life: f32,
    pub seed: u32,
}

#[derive(Clone, Copy)]
pub struct DmgNum {
    pub pos: Vec2,
    pub val: f32,
    pub life: f32,
    pub crit: bool,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum PickKind {
    Soul(f32),
    Draught,
    Magnet,
    Coffer(bool),
    Buff(Buff),
    Relic,
    Pyre,
    Bread,
    /// Richer loot, and a wave of elites when opened.
    CursedCoffer,
    /// The courier's key to the vault.
    Key,
}

pub struct Pickup {
    pub pos: Vec2,
    pub kind: PickKind,
    pub pulled: Option<usize>,
    pub bob: f32,
    /// Map relics vanish after a while; everything else stays.
    pub life: f32,
    /// Network id, handed out when the next snapshot round starts; 0 until then.
    pub id: u32,
}

impl Pickup {
    pub fn new(pos: Vec2, kind: PickKind) -> Pickup {
        Pickup { pos, kind, pulled: None, bob: pos.x * 0.1, life: f32::INFINITY, id: 0 }
    }

    /// Relics lying on the map are marked on the screen edge.
    pub fn is_relic(&self) -> bool {
        matches!(self.kind, PickKind::Buff(_) | PickKind::Relic | PickKind::Pyre | PickKind::Bread)
    }
}

pub struct Shrine {
    pub pos: Vec2,
    pub life: f32,
    pub kindle: f32,
    pub used: bool,
}

pub struct Remnant {
    pub pos: Vec2,
    pub souls: u64,
}

/// Codes for the champion's and Sven's health bars in `World::bars`.
pub const BAR_CHAMPION: u8 = 200;
pub const BAR_RIVAL: u8 = 201;

pub struct BossAi {
    pub id: u32,
    pub kind: BossKind,
    pub state: u32,
    pub t: f32,
    pub aim: Vec2,
    pub spin: f32,
    pub count: u32,
    pub shot: f32,
    pub phase2: bool,
    /// At a quarter of its health, for the lords that have one.
    pub phase3: bool,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Pause {
    Running,
    Choosing,
    Countdown(f32),
    /// Online, every player has the menu open: the world waits for them.
    Held,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    BossStart,
    BossDefeated,
    RemnantClaimed,
    Died,
    Victory,
    /// The Grave Cat chose a hero (by slot).
    CatFound(u8),
}

/// A black cat hiding somewhere on the field. Find it and it follows you.
pub struct Cat {
    pub pos: Vec2,
    pub owner: Option<u8>,
    pub facing: f32,
    pub sitting: bool,
    pub meow: f32,
}

/// Spatial hash so area queries stay cheap with a thousand enemies, however far apart
/// the heroes wander.
pub(crate) struct Grid {
    cell: f32,
    buckets: Vec<Vec<(i32, i32, u32)>>,
}

const GRID_BUCKETS: usize = 4096;

impl Grid {
    fn new() -> Grid {
        Grid { cell: 64.0, buckets: (0..GRID_BUCKETS).map(|_| Vec::new()).collect() }
    }

    fn bucket(cx: i32, cy: i32) -> usize {
        ((cx as u32).wrapping_mul(73_856_093) ^ (cy as u32).wrapping_mul(19_349_663)) as usize % GRID_BUCKETS
    }

    pub(crate) fn rebuild(&mut self, enemies: &[Enemy]) {
        for b in self.buckets.iter_mut() {
            b.clear();
        }
        for (i, e) in enemies.iter().enumerate() {
            if e.dead {
                continue;
            }
            let cx = (e.pos.x / self.cell).floor() as i32;
            let cy = (e.pos.y / self.cell).floor() as i32;
            self.buckets[Self::bucket(cx, cy)].push((cx, cy, i as u32));
        }
    }

    /// Indices of enemies in cells overlapping the circle (coarse, callers check distance).
    fn query(&self, p: Vec2, r: f32, out: &mut Vec<usize>) {
        out.clear();
        let r = r + 32.0;
        let x0 = ((p.x - r) / self.cell).floor() as i32;
        let y0 = ((p.y - r) / self.cell).floor() as i32;
        let x1 = ((p.x + r) / self.cell).floor() as i32;
        let y1 = ((p.y + r) / self.cell).floor() as i32;
        for cy in y0..=y1 {
            for cx in x0..=x1 {
                for &(ex, ey, i) in &self.buckets[Self::bucket(cx, cy)] {
                    if ex == cx && ey == cy {
                        out.push(i as usize);
                    }
                }
            }
        }
    }
}

pub struct World {
    pub time: f32,
    pub heroes: Vec<Hero>,
    /// The hero this machine controls and follows with the camera.
    pub local: usize,
    pub level: u32,
    pub xp: f32,
    pub xp_next: f32,
    pub pending_levels: u32,
    pub pause: Pause,
    pub level_round: u16,
    pub diff: Difficulty,
    pub enemies: Vec<Enemy>,
    next_id: u32,
    pub(crate) next_pickup_id: u32,
    pub projs: Vec<Proj>,
    pub ebullets: Vec<EBullet>,
    pub fx: Vec<Fx>,
    pub pools: Vec<FirePool>,
    pub waves: Vec<Shockwave>,
    pub pillars: Vec<Pillar>,
    pub telegraphs: Vec<Telegraph>,
    pub particles: Vec<Particle>,
    pub decals: VecDeque<Decal>,
    pub nums: Vec<DmgNum>,
    pub pickups: Vec<Pickup>,
    pub shrines: Vec<Shrine>,
    pub braziers: Vec<Brazier>,
    /// Braziers lit this vigil, for the records.
    pub braziers_lit: u32,
    pub remnant: Option<Remnant>,
    pub cat: Option<Cat>,
    /// Seconds of Blood Moon left; 0 when the sky is normal.
    pub blood_moon: f32,
    pub moons_survived: u32,
    pub cursed: Option<CursedShrine>,
    pub trials_won: u32,
    pub merchant: Option<Merchant>,
    /// The quest in hand, if any.
    pub quest: Option<Quest>,
    pub altars: Vec<Altar>,
    /// The lord's damage in the current second, for `BOSS_SECOND_CAP`: when the second began
    /// and how much it took.
    /// For each lord: when its current second began and how much it has taken in it.
    boss_second: Vec<(u32, f32, f32)>,
    /// The hourglass shrine on the field, if any.
    pub hourglass: Option<Hourglass>,
    /// The wager stone before a lord.
    pub stone: Option<WagerStone>,
    /// The lord number the last stone was raised for.
    stone_made: u32,
    next_altar: f32,
    pub vault: Option<Vault>,
    /// Someone picked up the courier's key; anyone may open the vault with it.
    pub vault_key: bool,
    courier: Option<u32>,
    next_courier: f32,
    next_cursed: f32,
    pub(crate) next_mimic: f32,
    /// Seconds of mist left while a champion hunts.
    pub mist: f32,
    champion: Option<u32>,
    next_mist: f32,
    /// The weather and how long it lasts.
    pub weather: Option<(Weather, f32)>,
    next_weather: f32,
    next_rival: f32,
    /// How often Sven has come this vigil; he grows each time.
    rivals_come: u32,
    next_quest: f32,
    pub quests_done: u32,
    /// When a hero was last hurt, for the quest to stay unhurt.
    last_wound: f32,
    pub boss: Option<BossAi>,
    /// Lords that came while another still stood; the first takes over when that one falls.
    pub more_bosses: Vec<BossAi>,
    /// Two lords who came together, by id: while both stand, each shields the other.
    pub twins: Option<(u32, u32)>,
    /// The smaller health bars: other lords (their `BossKind` code), the mist's champion
    /// (`BAR_CHAMPION`) and Sven (`BAR_RIVAL`), with health and its most.
    pub bars: Vec<(u8, f32, f32)>,
    pub bosses_spawned: u32,
    pub bosses_killed: u32,
    pub arena: Option<(Vec2, f32)>,
    pub next_boss: f32,
    /// Seconds until the next lord arrives while the warning shows, else 0. Sent to clients.
    pub boss_warning: f32,
    /// The lord coming within `MUSIC_LEAD` seconds, so its song can begin; synced.
    pub lord_soon: Option<BossKind>,
    /// A Stormcaller's crit is leaping right now: it does not leap again from there.
    pub leaping: bool,
    /// Weapon sounds grow quieter the faster a hero attacks; 1 outside the weapons' turn.
    pub fire_vol: f32,
    /// The hourglass ran past a lord: its warning lasts the whole wait.
    pub long_warning: bool,
    /// Lord after lord with only a short breather, no warning, and thin hordes.
    pub boss_rush: bool,
    /// Curse bits on this vigil, see `Curse`.
    pub curses: u8,
    /// Vigil time when the endless night began (at dawn, or after the last lord of a rush).
    pub endless_since: f32,
    next_shrine: f32,
    next_elite: f32,
    next_horde: f32,
    next_swarm: f32,
    next_relic: f32,
    spawn_acc: f32,
    pub kills: u32,
    pub cam: Vec2,
    pub shake: f32,
    pub events: Vec<Event>,
    pub sounds: Vec<(&'static str, f32, f64, Option<Vec2>)>,
    pub banner: Option<(String, String, Color, f32)>,
    /// A lord's words: what is said, in its colour, and how long it stays.
    pub quote: Option<(String, Color, f32)>,
    pub biome: Biome,
    pub toasts: Vec<(String, f32)>,
    pub(crate) grid: Grid,
    qbuf: Vec<usize>,
    pub show_numbers: bool,
    pub gore: bool,
    /// Cute mode: soft colours for blood and fire, hearts for bullets, blushing foes.
    pub cute: bool,
    /// Draw the full models; off draws cheaper ones for big hordes on slow machines.
    pub detail_high: bool,
    /// Particle budget multiplier (0.35 low, 0.65 medium, 1 high).
    pub particle_mult: f32,
    pub fog: bool,
    pub shake_mult: f32,
    /// The colour foes' projectiles are drawn in (see `save::COLOR_MODES`).
    pub bullet_color: Color,
    pub boss_bar_lag: f32,
    /// When the current boss arrived, for the quick-kill achievement.
    pub boss_started: f32,
    /// The Sovereign of the last hour has come: dawn waits for it.
    pub finale_came: bool,
    pub screen: Vec2,
    pub dead: bool,
    pub won: bool,
    /// The party left through the ash gate: the vigil is over, every carried soul banked.
    pub withdrew: bool,
    pub gate: Option<AshGate>,
    pub rift: Option<Rift>,
    /// The chamber's guardians, by id.
    pub guards: Vec<u32>,
    pub(crate) next_rift: f32,
    pub portal: Option<Portal>,
    /// When this vigil's portal opens, and whether it has.
    pub(crate) portal_at: f32,
    portal_done: bool,
    pub chambers_won: u32,
    /// Charges between heroes who keep close, toward a duo strike.
    pub bonds: Vec<Bond>,
    pub duo_strikes: u32,
    /// Stacks of each endless mutator, by `Mutator as usize`, and when the next comes.
    pub mutators: [u8; ALL_MUTATORS.len()],
    pub(crate) next_mutator: f32,
    pub(crate) prop_timer: f32,
    pub(crate) ember_tick: f32,
    /// Damage over time is being dealt: it lays no element of its own.
    pub(crate) dot_tick: bool,
    pub runes: Vec<TeleRune>,
    pub props_broken: u32,
    pub runes_used: u32,
    /// The player who wears the crown this vigil (a secret), and the lord they hold now.
    pub usurper: Option<u8>,
    pub possessed: Option<u32>,
    /// Foes felled this vigil by kind, in `BEASTS` order.
    pub felled: [u32; BEASTS.len()],
    /// Marks players placed for each other, and when each slot last pinged.
    pub pings: Vec<Ping>,
    pub(crate) ping_last: [f32; MAX_HEROES],
    pub portals_crossed: u32,
    /// The vigil was kept and goes on past dawn; it only ends when everyone falls.
    pub endless: bool,
    /// The lost grave gave up its relic this vigil.
    pub grave_found: bool,
    pub bell_found: bool,
    pub heart_found: bool,
    /// Set on a network client: the world is only displayed, never simulated.
    pub is_client: bool,
    /// A client that joined the running vigil to watch it: it has no hero of its own and
    /// follows `local`, someone else's.
    pub spectating: bool,
    /// Set on a host: record cosmetic events so they can be sent to clients.
    pub record_net: bool,
    pub net_nums: Vec<DmgNum>,
    pub net_deaths: Vec<(Vec2, EnemyKind, bool)>,
    pub net_sounds: Vec<(&'static str, f32, Option<Vec2>)>,
    /// Boss health as (current, max), kept separately so clients can show the bar.
    pub boss_hp: (f32, f32),
    /// Boss kind for the health bar on clients, which have no `BossAi`.
    pub boss_kind: Option<BossKind>,
}

pub fn xp_for(level: u32) -> f32 {
    let l = level as f32;
    // Past level 25 each level asks 10% more on top for every level beyond: a night ends
    // nearer level 70 than 230, and a full arsenal comes late in it rather than at its middle.
    let late = 1.0 + (l - 25.0).max(0.0) * 0.10;
    (6.0 + (l - 1.0) * 10.0 + l.powf(1.7)) * late
}

pub(crate) fn rand_dir() -> Vec2 {
    Vec2::from_angle(gen_range(0.0, TAU))
}

pub(crate) fn chance(p: f32) -> bool {
    gen_range(0.0, 1.0) < p
}

pub fn slot_color(slot: u8) -> Color {
    match slot {
        0 => Color::from_rgba(64, 54, 48, 255),
        1 => Color::from_rgba(44, 56, 84, 255),
        2 => Color::from_rgba(46, 72, 50, 255),
        _ => Color::from_rgba(84, 44, 44, 255),
    }
}

impl World {
    pub fn new(diff: Difficulty, remnant: Option<(Vec2, u64)>) -> World {
        // A new world starts on the full field, whatever the last one was.
        set_field_half(WORLD_HALF);
        World {
            time: 0.0,
            heroes: Vec::new(),
            local: 0,
            level: 1,
            xp: 0.0,
            xp_next: xp_for(1),
            pending_levels: 0,
            pause: Pause::Running,
            level_round: 0,
            diff,
            enemies: Vec::new(),
            next_id: 1,
            next_pickup_id: 1,
            projs: Vec::new(),
            ebullets: Vec::new(),
            fx: Vec::new(),
            pools: Vec::new(),
            waves: Vec::new(),
            pillars: Vec::new(),
            telegraphs: Vec::new(),
            particles: Vec::new(),
            decals: VecDeque::new(),
            nums: Vec::new(),
            pickups: Vec::new(),
            shrines: Vec::new(),
            braziers: brazier_spots().into_iter().map(|pos| Brazier { pos, burn: 0.0, kindle: 0.0 }).collect(),
            braziers_lit: 0,
            remnant: remnant.map(|(p, n)| Remnant { pos: clamp_world(p.clamp_length_max(1400.0), 80.0), souls: n }),
            blood_moon: 0.0,
            moons_survived: 0,
            cursed: None,
            trials_won: 0,
            merchant: None,
            quest: None,
            altars: Vec::new(),
            boss_second: Vec::new(),
            hourglass: None,
            stone: None,
            stone_made: u32::MAX,
            next_altar: 200.0,
            vault: None,
            vault_key: false,
            courier: None,
            next_courier: 250.0,
            next_cursed: 130.0,
            next_mimic: 200.0,
            mist: 0.0,
            champion: None,
            next_mist: 340.0,
            weather: None,
            next_weather: 420.0,
            next_rival: 180.0,
            rivals_come: 0,
            next_quest: 0.0,
            quests_done: 0,
            last_wound: 0.0,
            cat: Some(Cat { pos: clamp_world(rand_dir() * gen_range(1800.0, 3000.0), 300.0), owner: None, facing: 1.0, sitting: true, meow: gen_range(8.0, 20.0) }),
            boss: None,
            more_bosses: Vec::new(),
            twins: None,
            bars: Vec::new(),
            bosses_spawned: 0,
            bosses_killed: 0,
            arena: None,
            next_boss: BOSS_INTERVAL,
            boss_warning: 0.0,
            lord_soon: None,
            leaping: false,
            fire_vol: 1.0,
            long_warning: false,
            boss_rush: false,
            curses: 0,
            endless_since: 0.0,
            next_shrine: SHRINE_INTERVAL,
            next_elite: 75.0,
            next_horde: 90.0,
            next_swarm: 140.0,
            next_relic: 25.0,
            spawn_acc: 0.0,
            kills: 0,
            cam: Vec2::ZERO,
            shake: 0.0,
            events: Vec::new(),
            sounds: Vec::new(),
            banner: None,
            quote: None,
            biome: Biome::Graveyard,
            toasts: Vec::new(),
            grid: Grid::new(),
            qbuf: Vec::new(),
            show_numbers: true,
            gore: true,
            cute: false,
            detail_high: true,
            particle_mult: 1.0,
            fog: true,
            shake_mult: 1.0,
            bullet_color: Color::new(0.85, 0.06, 0.05, 1.0),
            boss_bar_lag: 1.0,
            boss_started: 0.0,
            finale_came: false,
            screen: vec2(1280.0, 720.0),
            dead: false,
            won: false,
            withdrew: false,
            gate: None,
            rift: None,
            guards: Vec::new(),
            next_rift: gen_range(300.0, 480.0),
            portal: None,
            portal_at: gen_range(660.0, 1080.0),
            portal_done: false,
            chambers_won: 0,
            bonds: Vec::new(),
            duo_strikes: 0,
            mutators: [0; ALL_MUTATORS.len()],
            next_mutator: 0.0,
            prop_timer: 0.0,
            ember_tick: 0.0,
            dot_tick: false,
            runes: Vec::new(),
            props_broken: 0,
            runes_used: 0,
            usurper: None,
            possessed: None,
            felled: [0; BEASTS.len()],
            pings: Vec::new(),
            ping_last: [0.0; MAX_HEROES],
            portals_crossed: 0,
            endless: false,
            grave_found: false,
            bell_found: false,
            heart_found: false,
            is_client: false,
            spectating: false,
            record_net: false,
            net_nums: Vec::new(),
            net_deaths: Vec::new(),
            net_sounds: Vec::new(),
            boss_hp: (0.0, 1.0),
            boss_kind: None,
        }
    }

    pub fn add_hero(&mut self, slot: u8, name: &str, class: Class, profile: Profile, remote: bool) -> usize {
        let mut h = Hero::new(slot, name, class, profile);
        h.remote = remote;
        // The cathedral has an arsenal of its own.
        h.arm_for(self.biome.arsenal());
        self.heroes.push(h);
        self.heroes.sort_by_key(|h| h.slot);
        self.heroes.iter().position(|h| h.slot == slot).unwrap()
    }

    /// Applies the "Head Start" relic: the vigil opens with a level-up.
    pub fn cursed_by(&self, c: Curse) -> bool {
        self.curses & c.bit() != 0
    }

    /// Lays curses on the vigil; Brittle takes the embers at once.
    pub fn lay_curses(&mut self, bits: u8) {
        self.curses = bits & 63;
        if self.cursed_by(Curse::Brittle) {
            for h in self.heroes.iter_mut() {
                h.p.revives = 0;
            }
        }
    }

    /// Turns this vigil into a boss rush: the first lord comes almost at once, and the
    /// heroes get a few level-ups to meet it.
    pub fn start_boss_rush(&mut self) {
        self.boss_rush = true;
        self.next_boss = RUSH_BREATHER;
        self.pending_levels += RUSH_HEAD_START;
        self.fit_field();
    }

    /// Half the side of this world's field: small for a boss rush.
    pub fn half(&self) -> f32 {
        if self.boss_rush {
            RUSH_HALF
        } else {
            WORLD_HALF
        }
    }

    /// Makes the field in play this world's: sets its size and, when the size changed (a
    /// boss rush began, or a client learned of one), moves the braziers, the cat and
    /// anything else that stood outside the walls.
    pub fn fit_field(&mut self) {
        let half = self.half();
        FIELD_HALF.with(|h| h.set(half));
        set_field_biome(self.biome);
        let spots = brazier_spots();
        if self.braziers.len() == spots.len() && self.braziers.iter().zip(&spots).all(|(b, s)| b.pos == *s) {
            return;
        }
        self.braziers = spots.into_iter().map(|pos| Brazier { pos, burn: 0.0, kindle: 0.0 }).collect();
        if let Some(cat) = self.cat.as_mut() {
            cat.pos = clamp_world(cat.pos, 150.0);
        }
        if let Some(r) = self.remnant.as_mut() {
            r.pos = clamp_world(r.pos, 80.0);
        }
        for h in self.heroes.iter_mut() {
            h.p.pos = clamp_world(h.p.pos, 24.0);
        }
    }

    pub fn grant_head_start(&mut self) {
        let n = self.heroes.iter().map(|h| h.profile.relics[3]).max().unwrap_or(0);
        self.pending_levels += n;
    }

    pub fn me(&self) -> &Hero {
        &self.heroes[self.local.min(self.heroes.len().saturating_sub(1))]
    }

    pub fn multiplayer(&self) -> bool {
        self.heroes.len() > 1
    }

    /// A sound heard everywhere: announcements and things that concern everyone.
    pub fn sfx(&mut self, name: &'static str, vol: f32) {
        self.sfx_gap(name, vol, 0.03);
    }

    /// A sound from a place on the field; it fades with distance from whoever listens.
    pub fn sfx_at(&mut self, name: &'static str, vol: f32, pos: Vec2) {
        self.sfx_gap_at(name, vol, 0.03, pos);
    }

    pub(crate) fn sfx_quiet(&mut self, name: &'static str) {
        self.sfx_gap(name, 0.15, 0.09);
    }

    pub(crate) fn sfx_gap(&mut self, name: &'static str, vol: f32, gap: f64) {
        self.push_sound(name, vol, gap, None);
    }

    pub(crate) fn sfx_gap_at(&mut self, name: &'static str, vol: f32, gap: f64, pos: Vec2) {
        self.push_sound(name, vol, gap, Some(pos));
    }

    fn push_sound(&mut self, name: &'static str, vol: f32, gap: f64, pos: Option<Vec2>) {
        let vol = vol * self.fire_vol;
        self.sounds.push((name, vol, gap, pos));
        // Clients hear the same sound once per spot, not once per foe.
        let near = |p: Option<Vec2>| match (p, pos) {
            (Some(a), Some(b)) => a.distance(b) < 300.0,
            (None, None) => true,
            _ => false,
        };
        if self.record_net && self.net_sounds.len() < 24 && !self.net_sounds.iter().any(|s| s.0 == name && near(s.2)) {
            self.net_sounds.push((name, vol, pos));
        }
    }

    /// How loud a sound from `pos` is for this player: full up close, gone a screen away.
    pub fn hearing(&self, pos: Option<Vec2>) -> f32 {
        let Some(p) = pos else { return 1.0 };
        let Some(me) = self.heroes.get(self.local).map(|h| h.p.pos) else { return 1.0 };
        let d = me.distance(p);
        (1.0 - (d - HEAR_FULL) / (HEAR_MAX - HEAR_FULL)).clamp(0.0, 1.0)
    }

    pub fn toast(&mut self, text: &str) {
        self.toasts.push((text.to_string(), 4.0));
        if self.toasts.len() > 4 {
            self.toasts.remove(0);
        }
    }

    pub fn banner(&mut self, big: &str, small: &str, color: Color, secs: f32) {
        self.banner = Some((big.to_string(), small.to_string(), color, secs));
    }

    /// A lord speaks; empty lines say nothing.
    pub fn say(&mut self, line: &str, color: Color) {
        if !line.is_empty() {
            self.quote = Some((line.to_string(), color, 4.5));
        }
    }

    fn alive_positions(&self) -> Vec<Vec2> {
        self.heroes.iter().filter(|h| h.alive()).map(|h| h.p.pos).collect()
    }

    /// Index of the living hero closest to `p`.
    pub fn nearest_hero(&self, p: Vec2) -> Option<usize> {
        self.heroes.iter().enumerate().filter(|(_, h)| h.alive()).min_by(|a, b| a.1.p.pos.distance_squared(p).total_cmp(&b.1.p.pos.distance_squared(p))).map(|(i, _)| i)
    }

    pub(crate) fn center(&self) -> Vec2 {
        let ps = self.alive_positions();
        if ps.is_empty() {
            return self.heroes.first().map(|h| h.p.pos).unwrap_or(Vec2::ZERO);
        }
        ps.iter().fold(Vec2::ZERO, |a, b| a + *b) / ps.len() as f32
    }

    // ---------------------------------------------------------------- level-ups

    /// Starts a level-up round: everyone gets cards and the world waits.
    /// The countdown after a level-up: 5% shorter every five levels, down to `MIN_RESUME_DELAY`.
    pub fn resume_delay(&self) -> f32 {
        (RESUME_DELAY * RESUME_SHRINK.powi((self.level / 5) as i32)).max(MIN_RESUME_DELAY)
    }

    fn begin_round(&mut self) {
        self.level_round = self.level_round.wrapping_add(1);
        let mut auto = Vec::new();
        let mut rush_picks = Vec::new();
        for (h, hero) in self.heroes.iter_mut().enumerate() {
            hero.choices = hero.roll_choices();
            // Players who left can't choose; don't let them hold everyone up.
            hero.picked = !hero.connected;
            // Nothing left to choose: take the souls without asking.
            if hero.connected && hero.maxed() {
                hero.choices = vec![Choice::Souls];
                hero.apply_choice(Choice::Souls);
                hero.picked = true;
                auto.push(h);
                crate::log!("level {}: {} has everything maxed and takes 150 souls", self.level, hero.name);
            } else if hero.connected && self.boss_rush && !hero.choices.is_empty() {
                // A boss rush never stops for cards: each level takes one at random.
                let c = hero.choices[gen_range(0, hero.choices.len())];
                crate::log!("level {}: {} takes {} (boss rush)", self.level, hero.name, hero.choice_text(c).0);
                hero.apply_choice(c);
                hero.picked = true;
                rush_picks.push((h, hero.choice_text(c).0));
            }
        }
        for (h, text) in rush_picks {
            self.check_fusion(h);
            let who = if self.multiplayer() { format!("{}: ", self.heroes[h].name) } else { String::new() };
            self.toast(&format!("{}Level {}: {}", who, self.level, text));
        }
        if self.heroes.iter().all(|h| h.picked) {
            // Nobody has a choice to make: no pause, no countdown.
            self.pending_levels = self.pending_levels.saturating_sub(1);
            for h in auto {
                let pos = self.heroes[h].p.pos;
                self.sfx_at("pickup", 0.5, pos);
            }
            // After a real choice the usual countdown still follows; otherwise play straight on.
            let counted = self.pause == Pause::Choosing && self.multiplayer() && !self.boss_rush;
            self.pause = if counted { Pause::Countdown(self.resume_delay()) } else { Pause::Running };
            return;
        }
        self.pause = Pause::Choosing;
        self.sfx("levelup", 0.7);
    }

    /// A hero picks card `idx` of round `round`. Stale or duplicate picks are ignored.
    pub fn pick(&mut self, h: usize, round: u16, idx: usize) {
        if self.pause != Pause::Choosing || round != self.level_round {
            return;
        }
        let Some(hero) = self.heroes.get_mut(h) else { return };
        if hero.picked || idx >= hero.choices.len() {
            return;
        }
        let c = hero.choices[idx];
        hero.apply_choice(c);
        hero.picked = true;
        self.check_fusion(h);
        self.sfx("select", 0.6);
    }

    pub fn reroll(&mut self, h: usize) {
        if self.pause != Pause::Choosing {
            return;
        }
        let Some(hero) = self.heroes.get_mut(h) else { return };
        if hero.picked || hero.rerolls == 0 {
            return;
        }
        hero.rerolls -= 1;
        hero.choices = hero.roll_choices();
        self.sfx("chime", 0.5);
    }

    /// Marks a hero as gone; they stay in the world as a fallen body.
    pub fn disconnect(&mut self, h: usize) {
        let Some(hero) = self.heroes.get_mut(h) else { return };
        hero.connected = false;
        hero.downed = true;
        hero.picked = true;
        hero.name = format!("{} (left)", hero.name);
        // If everyone still here is already down, nobody is left to revive them.
        if !self.dead && self.heroes.iter().all(|h| h.downed || h.possessing) {
            self.dead = true;
            self.sfx("death", 1.0);
            self.events.push(Event::Died);
        }
    }

    /// Gives a returning player their hero back, standing next to someone still fighting.
    pub fn reconnect(&mut self, slot: u8) -> bool {
        let anchor = self.heroes.iter().find(|h| h.connected && !h.downed).map(|h| h.p.pos);
        let choosing = self.pause == Pause::Choosing;
        let Some(hero) = self.heroes.iter_mut().find(|h| h.slot == slot && !h.connected) else { return false };
        hero.connected = true;
        if let Some(name) = hero.name.strip_suffix(" (left)") {
            hero.name = name.to_string();
        }
        hero.downed = false;
        hero.revive = 0.0;
        hero.p.hp = hero.stats.max_hp * 0.5;
        hero.p.iframes = 2.0;
        // A level-up round that is already running goes on without them.
        hero.picked = choosing;
        if let Some(p) = anchor {
            hero.p.pos = clamp_world(p + vec2(40.0, 30.0), 24.0);
        }
        true
    }

    /// Online, every hero still here has the menu open.
    fn all_in_menu(&self) -> bool {
        self.multiplayer() && self.heroes.iter().filter(|h| h.connected).all(|h| h.menu_open)
    }

    fn update_pause(&mut self, dt: f32) {
        match self.pause {
            Pause::Running => {
                if self.all_in_menu() {
                    self.pause = Pause::Held;
                } else if self.pending_levels > 0 {
                    self.begin_round();
                }
            }
            Pause::Choosing => {
                if self.heroes.iter().all(|h| h.picked) {
                    self.pending_levels = self.pending_levels.saturating_sub(1);
                    if self.pending_levels > 0 {
                        self.begin_round();
                    } else if self.multiplayer() {
                        self.pause = Pause::Countdown(self.resume_delay());
                    } else {
                        self.pause = Pause::Running;
                    }
                }
            }
            Pause::Held => {
                // Someone came back from the menu: a countdown, then on.
                if !self.all_in_menu() {
                    self.pause = Pause::Countdown(self.resume_delay());
                }
            }
            Pause::Countdown(t) => {
                let t = t - dt;
                self.pause = if t <= 0.0 { Pause::Running } else { Pause::Countdown(t) };
            }
        }
    }

    // ---------------------------------------------------------------- frame

    pub fn update(&mut self, dt: f32) {
        self.fit_field();
        if self.dead || self.won || self.is_client {
            self.update_fx(dt);
            return;
        }
        self.update_pause(dt);
        if self.pause != Pause::Running {
            // Souls already flying go on to their hero and count, so a level gained meanwhile
            // comes in the same pause rather than in another one right after.
            if matches!(self.pause, Pause::Choosing | Pause::Countdown(_)) {
                self.update_pickups(dt, true);
            }
            self.update_fx(0.0);
            return;
        }
        self.time += dt;
        self.update_heroes(dt);
        self.update_spawns(dt);
        self.grid.rebuild(&self.enemies);
        self.cast_skills();
        self.update_weapons(dt);
        self.update_projs(dt);
        self.update_hazards(dt);
        self.update_enemies(dt);
        self.update_boss(dt);
        self.update_pickups(dt, false);
        self.update_shrines(dt);
        self.update_braziers(dt);
        self.update_cat(dt);
        self.update_secrets(dt);
        self.update_events(dt);
        self.update_quests(dt);
        self.update_omens(dt);
        self.update_passages(dt);
        self.update_duos(dt);
        self.update_mutators();
        self.update_props(dt);
        self.update_field_hazards(dt);
        self.update_pings(dt);
        self.update_burden();
        self.boss_hp = match &self.boss {
            Some(b) => self.enemies.iter().find(|e| e.id == b.id).map(|e| (e.hp, e.max_hp)).unwrap_or((0.0, 1.0)),
            None => (0.0, 1.0),
        };
        // The other lords, then the champion and Sven, as smaller bars.
        let mut bars = Vec::new();
        for b in &self.more_bosses {
            if let Some(e) = self.enemies.iter().find(|e| e.id == b.id && !e.dead) {
                bars.push((b.kind as u8, e.hp.max(0.0), e.max_hp));
            }
        }
        for e in self.enemies.iter().filter(|e| !e.dead) {
            match e.kind {
                EnemyKind::Champion => bars.push((BAR_CHAMPION, e.hp.max(0.0), e.max_hp)),
                EnemyKind::Rival => bars.push((BAR_RIVAL, e.hp.max(0.0), e.max_hp)),
                _ => {}
            }
        }
        self.bars = bars;
        let now = self.time;
        for h in self.heroes.iter_mut() {
            h.refresh_minute(now);
        }
        self.update_fx(dt);
        // Last of all, after every split, summon and call of this frame.
        self.trim_overflow();
        self.enemies.retain(|e| !e.dead);
        // Dawn ends a vigil once the Sovereign of the last hour has fallen; the last of the
        // lords ends a boss rush.
        let done = if self.boss_rush { self.bosses_killed >= RUSH_LORDS } else { self.time >= VICTORY_TIME && self.finale_came };
        if done && self.boss.is_none() && !self.won && !self.endless {
            self.won = true;
            for h in self.heroes.iter_mut() {
                h.banked += h.p.souls;
                h.p.souls = 0;
            }
            self.sfx("evolve", 1.0);
            self.events.push(Event::Victory);
        }
    }

    /// Client-side frame: predict our own hero and move the rest along their velocities.
    /// Returns true when our hero started a roll.
    /// A line in the log about what happened in the vigil, stamped with the vigil's clock.
    pub(crate) fn log_event(&self, text: &str) {
        if self.is_client {
            return;
        }
        let t = self.time.max(0.0) as u32;
        crate::log!("[{:02}:{:02}] {}", t / 60, t % 60, text);
    }

    /// Heroes grow stronger with the souls they carry; the stats follow every step.
    fn update_burden(&mut self) {
        let mut notes = Vec::new();
        let multi = self.heroes.len() > 1;
        for h in self.heroes.iter_mut() {
            let b = soul_burden(h.p.souls);
            if b != h.burden {
                let up = b > h.burden;
                h.burden = b;
                h.recompute_stats();
                crate::log!("[{:02}:{:02}] {} carries {} souls: burden {}", self.time as u32 / 60, self.time as u32 % 60, h.name, h.p.souls, b);
                if up {
                    notes.push(format!("{}The souls you carry weigh in: burden {} of {}", if multi { format!("{}: ", h.name) } else { String::new() }, b, BURDEN_MAX));
                }
            }
        }
        for n in notes {
            self.toast(&n);
        }
    }

    pub fn client_update(&mut self, dt: f32) -> bool {
        self.fit_field();
        let mut rolled = false;
        // The countdown runs here too, so it stays smooth between the slower paused snapshots.
        if let Pause::Countdown(t) = self.pause {
            self.pause = Pause::Countdown((t - dt).max(0.0));
        }
        for p in self.pings.iter_mut() {
            p.life -= dt;
        }
        if self.pause == Pause::Running && !self.dead && !self.won && !self.spectating {
            self.client_rune();
            let arena = self.arena;
            // The wager tile fills under our feet here too; the host decides the bet itself.
            let tiles = self.stone.as_ref().map(|s| [s.tile(0), s.tile(1), s.tile(2)]);
            if let Some(hero) = self.heroes.get_mut(self.local) {
                let on = tiles.and_then(|t| t.iter().position(|q| hero.p.pos.distance(*q) < 26.0)).filter(|_| hero.wager.is_none());
                match on {
                    Some(i) if hero.wager_tile == i => hero.wager_kindle = (hero.wager_kindle + dt).min(1.0),
                    Some(i) => {
                        hero.wager_tile = i;
                        hero.wager_kindle = 0.0;
                    }
                    None => hero.wager_kindle = 0.0,
                }
                if hero.downed {
                    hero.ghost += hero.input.dir * GHOST_SPEED * dt;
                    hero.leash_ghost(REVIVE_TIME);
                }
                if !hero.downed {
                    rolled = hero.step_movement(dt, arena);
                    let (pos, rolling) = (hero.p.pos, hero.p.roll_t > 0.0);
                    if rolling && chance(0.6) {
                        self.fx.push(Fx { pos, life: 0.25, max_life: 0.25, kind: FxKind::Ghost });
                    }
                }
            }
            for e in self.enemies.iter_mut() {
                e.pos += e.net_vel * dt;
                e.flash -= dt;
            }
            for b in self.ebullets.iter_mut() {
                b.pos += b.vel * dt;
            }
            for p in self.projs.iter_mut() {
                p.pos += p.vel * dt;
                p.age += dt;
            }
            let local = self.local;
            for (i, h) in self.heroes.iter_mut().enumerate() {
                if i != local && h.p.moving && h.p.roll_t <= 0.0 && !h.downed {
                    h.p.walk += dt * 10.0;
                }
            }
            if rolled && self.me().rolls == 100 {
                self.toast("100 rolls. Your knees file a formal complaint.");
            } else if rolled && self.me().rolls == 1000 {
                self.toast("1,000 rolls. The floor knows you by name now.");
            }
        }
        self.update_fx(dt);
        rolled
    }
}
