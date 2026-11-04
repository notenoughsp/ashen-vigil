//! Static game data: weapons, items, classes, unlocks, upgrades and their numbers.

use macroquad::prelude::*;

/// Weapon and item slots before the Arsenal and Satchel upgrades.
pub const MAX_WEAPONS: usize = 6;
pub const MAX_ITEMS: usize = 6;
pub const WEAPON_MAX_LEVEL: u32 = 10;
/// Damage multiplier of a fused weapon over its two evolved parts.
pub const FUSION_MULT: f32 = 1.5;
/// Damage of each part in a mega fusion (four weapons) and an ultra fusion (eight).
pub const MEGA_MULT: f32 = 1.9;
pub const ULTRA_MULT: f32 = 2.4;
/// Damage of each part once every weapon has become one.
pub const LAST_MULT: f32 = 3.0;
/// The name of the weapon that holds all the others.
pub const LAST_FLAME: &str = "The Last Flame";

// ---------------------------------------------------------------- weapons

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WeaponKind {
    Blade,
    Daggers,
    Orb,
    Chime,
    Lanterns,
    Spear,
    Axe,
    Bolt,
    Scythe,
    Hex,
    Lance,
    Crossbow,
}

pub const ALL_WEAPONS: [WeaponKind; 12] = [
    WeaponKind::Blade,
    WeaponKind::Daggers,
    WeaponKind::Orb,
    WeaponKind::Chime,
    WeaponKind::Lanterns,
    WeaponKind::Spear,
    WeaponKind::Axe,
    WeaponKind::Bolt,
    WeaponKind::Scythe,
    WeaponKind::Hex,
    WeaponKind::Lance,
    WeaponKind::Crossbow,
];

#[derive(Clone, Copy, Debug)]
pub struct WStat {
    pub dmg: f32,
    pub cd: f32,
    pub amount: u32,
    pub area: f32,
    pub speed: f32,
    pub pierce: i32,
}

impl WeaponKind {
    pub fn idx(self) -> usize {
        self as usize
    }

    pub fn from_idx(i: u8) -> WeaponKind {
        ALL_WEAPONS[(i as usize).min(ALL_WEAPONS.len() - 1)]
    }

    pub fn name(self, evolved: bool) -> &'static str {
        use WeaponKind::*;
        match (self, evolved) {
            (Blade, false) => "Rusted Blade",
            (Blade, true) => "Cinder Edge",
            (Daggers, false) => "Pale Daggers",
            (Daggers, true) => "Thousand Thorns",
            (Orb, false) => "Kindled Orb",
            (Orb, true) => "Ruin Sun",
            (Chime, false) => "Chime of Warding",
            (Chime, true) => "Sanctum Bell",
            (Lanterns, false) => "Wraith Lanterns",
            (Lanterns, true) => "Choir of the Drowned",
            (Spear, false) => "Storm Spear",
            (Spear, true) => "Heaven's Reproach",
            (Axe, false) => "Crescent Axe",
            (Axe, true) => "Moonreaver",
            (Bolt, false) => "Soulbolt",
            (Bolt, true) => "Soul Torrent",
            (Scythe, false) => "Grave Scythe",
            (Scythe, true) => "Widow's Embrace",
            (Hex, false) => "Hex Brand",
            (Hex, true) => "Hellbrand",
            (Lance, false) => "Blood Lance",
            (Lance, true) => "Sanguine Pike",
            (Crossbow, false) => "Gloom Crossbow",
            (Crossbow, true) => "Arbalest of the Damned",
        }
    }

    pub fn desc(self) -> &'static str {
        use WeaponKind::*;
        match self {
            Blade => "Cleaves the dark in front of you.",
            Daggers => "Bone daggers flung where you walk.",
            Orb => "A burning orb that bursts on impact.",
            Chime => "A ward that sears all who come near.",
            Lanterns => "Restless spirits circle their keeper.",
            Spear => "Calls lightning down on the unworthy.",
            Axe => "A thrown axe that always returns.",
            Bolt => "Seeking bolts of condensed soul.",
            Scythe => "A full sweep that reaps everything around you.",
            Hex => "Your footsteps leave burning sigils behind.",
            Lance => "A lance of blood that pierces a whole line.",
            Crossbow => "Quarrels fly in every direction at once.",
        }
    }

    pub fn evo_desc(self) -> &'static str {
        use WeaponKind::*;
        match self {
            Blade => "Twin burning arcs that set foes alight.",
            Daggers => "An endless storm of piercing thorns.",
            Orb => "A dying sun that scorches the earth.",
            Chime => "A vast ward that mends its bearer.",
            Lanterns => "Two choirs of the drowned, ever turning.",
            Spear => "Lightning that leaps from foe to foe.",
            Axe => "Great axes that carve the moon's path.",
            Bolt => "A torrent of hungering souls.",
            Scythe => "Every soul reaped mends its wielder.",
            Hex => "The ground itself burns where you tread.",
            Lance => "Each foe pierced feeds your blood.",
            Crossbow => "A wall of quarrels in twelve directions.",
        }
    }

    /// Passive item that evolves this weapon at max level.
    pub fn evo_item(self) -> ItemKind {
        use WeaponKind::*;
        match self {
            Blade => ItemKind::EmberHeart,
            Daggers => ItemKind::Boots,
            Orb => ItemKind::Lens,
            Chime => ItemKind::Moss,
            Lanterns => ItemKind::Sigil,
            Spear => ItemKind::Feather,
            Axe => ItemKind::Shell,
            Bolt => ItemKind::Hourglass,
            Scythe => ItemKind::LeechFang,
            Hex => ItemKind::Lung,
            Lance => ItemKind::GraveCandle,
            Crossbow => ItemKind::Censer,
        }
    }

    /// Weapons this one fuses with when both reach max level; the first is its classic partner.
    pub fn fusion_partners(self) -> [WeaponKind; 2] {
        use WeaponKind::*;
        match self {
            Blade => [Axe, Scythe],
            Axe => [Blade, Crossbow],
            Daggers => [Crossbow, Bolt],
            Crossbow => [Daggers, Axe],
            Orb => [Hex, Spear],
            Hex => [Orb, Chime],
            Chime => [Lanterns, Hex],
            Lanterns => [Chime, Lance],
            Spear => [Bolt, Orb],
            Bolt => [Spear, Daggers],
            Scythe => [Lance, Blade],
            Lance => [Scythe, Lanterns],
        }
    }

    /// The classic partner, for the table in the README and the evolution hints.
    pub fn fusion_partner(self) -> WeaponKind {
        self.fusion_partners()[0]
    }

    pub fn color(self) -> Color {
        use WeaponKind::*;
        match self {
            Blade => Color::from_rgba(200, 170, 140, 255),
            Daggers => Color::from_rgba(225, 220, 205, 255),
            Orb => Color::from_rgba(255, 130, 40, 255),
            Chime => Color::from_rgba(240, 210, 120, 255),
            Lanterns => Color::from_rgba(120, 200, 255, 255),
            Spear => Color::from_rgba(170, 190, 255, 255),
            Axe => Color::from_rgba(180, 180, 195, 255),
            Bolt => Color::from_rgba(150, 230, 255, 255),
            Scythe => Color::from_rgba(150, 140, 160, 255),
            Hex => Color::from_rgba(200, 60, 220, 255),
            Lance => Color::from_rgba(190, 20, 30, 255),
            Crossbow => Color::from_rgba(110, 100, 80, 255),
        }
    }

    /// Numbers for a weapon at a given level, before player stats are applied.
    pub fn stats(self, level: u32, evolved: bool) -> WStat {
        use WeaponKind::*;
        let l = level.saturating_sub(1) as f32;
        let at = |lvls: &[u32]| lvls.iter().filter(|&&x| level >= x).count() as u32;
        let mut s = match self {
            Blade => WStat { dmg: 14.0 + 5.0 * l, cd: 1.25 - 0.05 * l, amount: 1 + at(&[3, 6, 9]), area: 1.0 + 0.1 * at(&[2, 4, 7, 10]) as f32, speed: 0.0, pierce: 0 },
            Daggers => WStat { dmg: 7.0 + 2.5 * l, cd: 0.9 - 0.04 * l, amount: 1 + at(&[2, 4, 6, 8, 10]), area: 1.0, speed: 650.0, pierce: 1 + at(&[5, 9]) as i32 },
            Orb => WStat { dmg: 18.0 + 6.0 * l, cd: 1.8 - 0.08 * l, amount: 1 + at(&[4, 7, 10]), area: 1.0 + 0.15 * at(&[3, 5, 8]) as f32, speed: 330.0, pierce: 0 },
            Chime => WStat { dmg: 5.0 + 2.0 * l, cd: 0.55 - 0.02 * l, amount: 1, area: 1.0 + 0.1 * l, speed: 0.0, pierce: 0 },
            Lanterns => WStat { dmg: 10.0 + 3.0 * l, cd: 0.45, amount: 2 + at(&[2, 4, 6, 8, 10]), area: 1.0 + 0.04 * l, speed: 3.0 + 0.15 * l, pierce: 0 },
            Spear => WStat { dmg: 22.0 + 7.0 * l, cd: 2.0 - 0.09 * l, amount: 1 + at(&[3, 5, 7, 9]), area: 1.0 + 0.1 * at(&[4, 8, 10]) as f32, speed: 0.0, pierce: 0 },
            Axe => WStat { dmg: 20.0 + 6.0 * l, cd: 2.2 - 0.09 * l, amount: 1 + at(&[4, 7, 10]), area: 1.0 + 0.1 * at(&[3, 6, 9]) as f32, speed: 520.0, pierce: 999 },
            Bolt => WStat { dmg: 9.0 + 3.0 * l, cd: 1.3 - 0.05 * l, amount: 1 + at(&[2, 5, 7, 10]), area: 1.0, speed: 420.0, pierce: 0 },
            Scythe => WStat { dmg: 16.0 + 5.0 * l, cd: 2.0 - 0.08 * l, amount: 1 + at(&[5, 9]), area: 1.0 + 0.1 * at(&[3, 6, 8, 10]) as f32, speed: 0.0, pierce: 0 },
            Hex => WStat { dmg: 6.0 + 2.0 * l, cd: 0.35 - 0.01 * l, amount: 1, area: 1.0 + 0.1 * at(&[3, 6, 9]) as f32, speed: 2.5, pierce: 0 },
            Lance => WStat { dmg: 25.0 + 8.0 * l, cd: 1.6 - 0.07 * l, amount: 1 + at(&[4, 8]), area: 1.0 + 0.1 * at(&[5, 10]) as f32, speed: 900.0, pierce: 999 },
            Crossbow => WStat { dmg: 10.0 + 3.0 * l, cd: 1.2 - 0.05 * l, amount: 4 + 2 * at(&[3, 6, 9]), area: 1.0, speed: 560.0, pierce: 2 + at(&[5, 10]) as i32 },
        };
        if evolved {
            match self {
                Blade => {
                    s.dmg += 20.0;
                    s.cd = 0.9;
                    s.area *= 1.25;
                }
                Daggers => {
                    s.cd = 0.12;
                    s.amount = 1;
                    s.pierce = 999;
                    s.speed = 800.0;
                }
                Orb => {
                    s.dmg *= 2.0;
                    s.area *= 1.8;
                    s.cd = 1.6;
                }
                Chime => {
                    s.dmg += 8.0;
                    s.area *= 1.4;
                }
                Lanterns => {
                    s.amount += 4;
                    s.dmg += 10.0;
                }
                Spear => {
                    s.cd = 1.0;
                    s.dmg += 15.0;
                }
                Axe => {
                    s.amount += 2;
                    s.area *= 1.5;
                    s.cd = 1.3;
                }
                Bolt => {
                    s.cd = 0.35;
                    s.amount += 2;
                    s.speed = 560.0;
                }
                Scythe => {
                    s.cd = 1.4;
                    s.area *= 1.3;
                    s.dmg += 15.0;
                }
                Hex => {
                    s.area *= 1.6;
                    s.speed = 4.0;
                    s.dmg += 8.0;
                }
                Lance => {
                    s.dmg += 25.0;
                    s.area *= 1.4;
                }
                Crossbow => {
                    s.amount = 12;
                    s.pierce = 5;
                    s.cd = 0.6;
                }
            }
        }
        s
    }

    pub fn level_desc(self, next: u32) -> String {
        let a = self.stats(next.saturating_sub(1).max(1), false);
        let b = self.stats(next, false);
        let mut parts = vec![format!("+{:.0} damage", b.dmg - a.dmg)];
        if b.amount > a.amount {
            parts.push(format!("+{} amount", b.amount - a.amount));
        }
        if b.area > a.area + 0.001 {
            parts.push(format!("+{:.0}% area", (b.area - a.area) * 100.0));
        }
        if b.cd < a.cd - 0.001 {
            parts.push("faster".to_string());
        }
        if b.pierce > a.pierce && b.pierce < 999 {
            parts.push(format!("+{} pierce", b.pierce - a.pierce));
        }
        parts.join(", ")
    }
}

/// Name of the weapon born from fusing `a` and `b`.
pub fn fusion_name(a: WeaponKind, b: WeaponKind) -> &'static str {
    use WeaponKind::*;
    let key = if a.idx() < b.idx() { (a, b) } else { (b, a) };
    match key {
        (Blade, Axe) => "Executioner's Gyre",
        (Daggers, Crossbow) => "Thousandfold Volley",
        (Orb, Hex) => "Cinderstorm",
        (Chime, Lanterns) => "Requiem of Bells",
        (Spear, Bolt) => "Wrath of the Pale Sky",
        (Scythe, Lance) => "Crimson Harvest",
        (Blade, Scythe) => "Reaper's Waltz",
        (Daggers, Bolt) => "Ghostfang Storm",
        (Orb, Spear) => "Thunderpyre",
        (Chime, Hex) => "Tolling Brand",
        (Lanterns, Lance) => "Lantern Pike",
        (Axe, Crossbow) => "Moonfall Barrage",
        _ => names_for(&[a, b], &ODD_NAMES),
    }
}

/// Weapons that were never meant for each other, bound together anyway.
const ODD_NAMES: [&str; 8] = ["Grafted Woe", "Twinned Ruin", "Bound Relic", "Mismatched Wrath", "Stitched Fury", "Unholy Union", "Chimera's Bite", "Forced Accord"];

const MEGA_NAMES: [&str; 8] = ["Pyre of Kings", "Grave Tide", "Choir of Ash", "The Hollow Crown", "Eclipse Engine", "Black Cathedral", "Wake of Ruin", "The Long Night"];
const ULTRA_NAMES: [&str; 4] = ["Dawnbreaker", "The Unending", "Ash Incarnate", "World Pyre"];

/// The same set of weapons always gets the same name.
fn names_for(kinds: &[WeaponKind], names: &[&'static str]) -> &'static str {
    let mut idx: Vec<usize> = kinds.iter().map(|k| k.idx()).collect();
    idx.sort_unstable();
    let h = idx.iter().fold(7usize, |h, i| h.wrapping_mul(31).wrapping_add(*i + 1));
    names[h % names.len()]
}

pub fn mega_name(kinds: &[WeaponKind]) -> &'static str {
    names_for(kinds, &MEGA_NAMES)
}

pub fn ultra_name(kinds: &[WeaponKind]) -> &'static str {
    names_for(kinds, &ULTRA_NAMES)
}

// ---------------------------------------------------------------- items

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ItemKind {
    EmberHeart,
    Hourglass,
    Boots,
    Lens,
    Sigil,
    Censer,
    Shell,
    Feather,
    Moss,
    Lung,
    LeechFang,
    CrackedCrown,
    GraveCandle,
}

pub const ALL_ITEMS: [ItemKind; 13] = [
    ItemKind::EmberHeart,
    ItemKind::Hourglass,
    ItemKind::Boots,
    ItemKind::Lens,
    ItemKind::Sigil,
    ItemKind::Censer,
    ItemKind::Shell,
    ItemKind::Feather,
    ItemKind::Moss,
    ItemKind::Lung,
    ItemKind::LeechFang,
    ItemKind::CrackedCrown,
    ItemKind::GraveCandle,
];

impl ItemKind {
    pub fn idx(self) -> usize {
        self as usize
    }

    pub fn from_idx(i: u8) -> ItemKind {
        ALL_ITEMS[(i as usize).min(ALL_ITEMS.len() - 1)]
    }

    pub fn name(self) -> &'static str {
        use ItemKind::*;
        match self {
            EmberHeart => "Ember Heart",
            Hourglass => "Hollow Hourglass",
            Boots => "Pilgrim's Boots",
            Lens => "Ashen Lens",
            Sigil => "Twin Sigil",
            Censer => "Soulward Censer",
            Shell => "Iron Shell",
            Feather => "Crow Feather",
            Moss => "Verdant Moss",
            Lung => "Warden's Lung",
            LeechFang => "Leech Fang",
            CrackedCrown => "Cracked Crown",
            GraveCandle => "Grave Candle",
        }
    }

    pub fn desc(self) -> &'static str {
        use ItemKind::*;
        match self {
            EmberHeart => "+10% might",
            Hourglass => "-8% weapon cooldown",
            Boots => "+10% move speed",
            Lens => "+10% area",
            Sigil => "+1 projectile amount",
            Censer => "+35% soul pickup range",
            Shell => "+1 armor",
            Feather => "+12% luck and crit chance",
            Moss => "+0.3 health regen per second",
            Lung => "+20 stamina, +15% stamina regen",
            LeechFang => "+0.4 health per kill",
            CrackedCrown => "+10% experience",
            GraveCandle => "+15% damage to elites and bosses",
        }
    }

    pub fn max_level(self) -> u32 {
        match self {
            ItemKind::Sigil => 2,
            _ => 5,
        }
    }

    pub fn color(self) -> Color {
        use ItemKind::*;
        match self {
            EmberHeart => Color::from_rgba(230, 70, 40, 255),
            Hourglass => Color::from_rgba(210, 190, 150, 255),
            Boots => Color::from_rgba(140, 100, 70, 255),
            Lens => Color::from_rgba(160, 160, 170, 255),
            Sigil => Color::from_rgba(200, 120, 255, 255),
            Censer => Color::from_rgba(130, 220, 230, 255),
            Shell => Color::from_rgba(120, 130, 145, 255),
            Feather => Color::from_rgba(60, 60, 80, 255),
            Moss => Color::from_rgba(90, 170, 80, 255),
            Lung => Color::from_rgba(200, 110, 120, 255),
            LeechFang => Color::from_rgba(150, 10, 20, 255),
            CrackedCrown => Color::from_rgba(200, 170, 60, 255),
            GraveCandle => Color::from_rgba(235, 225, 190, 255),
        }
    }
}

// ---------------------------------------------------------------- classes

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Class {
    Wanderer,
    GraveHunter,
    CinderAdept,
    BellAcolyte,
    Reaper,
    Hexblade,
    BloodKnight,
    Arbalist,
    BucketKnight,
}

pub const ALL_CLASSES: [Class; 9] =
    [Class::Wanderer, Class::GraveHunter, Class::CinderAdept, Class::BellAcolyte, Class::Reaper, Class::Hexblade, Class::BloodKnight, Class::Arbalist, Class::BucketKnight];

impl Class {
    pub fn name(self) -> &'static str {
        match self {
            Class::Wanderer => "Wanderer",
            Class::GraveHunter => "Grave Hunter",
            Class::CinderAdept => "Cinder Adept",
            Class::BellAcolyte => "Bell Acolyte",
            Class::Reaper => "Reaper",
            Class::Hexblade => "Hexblade",
            Class::BloodKnight => "Blood Knight",
            Class::Arbalist => "Arbalist",
            Class::BucketKnight => "The Bucket Knight",
        }
    }

    pub fn desc(self) -> &'static str {
        match self {
            Class::Wanderer => "Rusted Blade. +10 max health.",
            Class::GraveHunter => "Pale Daggers. +10% move speed, +10 max health.",
            Class::CinderAdept => "Kindled Orb. +10% area.",
            Class::BellAcolyte => "Chime of Warding. +0.4 health regen.",
            Class::Reaper => "Grave Scythe. +0.5 health per kill, up to 4 per second.",
            Class::Hexblade => "Hex Brand. +25 stamina, +1 armor.",
            Class::BloodKnight => "Blood Lance. +20 max health.",
            Class::Arbalist => "Gloom Crossbow. -10% weapon cooldown, +10 max health.",
            Class::BucketKnight => "Crescent Axe. +2 armor. Cheap rolls. Clang.",
        }
    }

    pub fn weapon(self) -> WeaponKind {
        match self {
            Class::Wanderer => WeaponKind::Blade,
            Class::GraveHunter => WeaponKind::Daggers,
            Class::CinderAdept => WeaponKind::Orb,
            Class::BellAcolyte => WeaponKind::Chime,
            Class::Reaper => WeaponKind::Scythe,
            Class::Hexblade => WeaponKind::Hex,
            Class::BloodKnight => WeaponKind::Lance,
            Class::Arbalist => WeaponKind::Crossbow,
            Class::BucketKnight => WeaponKind::Axe,
        }
    }
}

pub fn class_from_u8(v: u8) -> Class {
    ALL_CLASSES.get(v as usize).copied().unwrap_or(Class::Wanderer)
}

pub fn class_to_u8(c: Class) -> u8 {
    ALL_CLASSES.iter().position(|x| *x == c).unwrap_or(0) as u8
}

// ---------------------------------------------------------------- soul upgrades (shrine)

pub const META_COUNT: usize = 33;

/// Indexes of the later shrine upgrades, used where they take effect.
pub const M_COLOSSUS: usize = 17;
pub const M_WARDING: usize = 18;
pub const M_BLOODTHIRST: usize = 19;
pub const M_FLEETFOOT: usize = 20;
pub const M_BARTER: usize = 21;
pub const M_FORESIGHT: usize = 22;
pub const M_SECOND_WIND: usize = 23;
pub const M_KINDLING: usize = 24;
pub const M_WHETSTONE: usize = 25;
pub const M_FUSION_LORE: usize = 26;
pub const M_EVASION: usize = 27;
pub const M_BULWARK: usize = 28;
pub const M_ALCHEMY: usize = 29;
pub const M_TITHE: usize = 30;
pub const M_ARSENAL: usize = 31;
pub const M_SATCHEL: usize = 32;

pub struct MetaDef {
    pub key: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub max: u32,
    pub base_cost: u64,
}

pub const META: [MetaDef; META_COUNT] = [
    MetaDef { key: "vigor", name: "Vigor", desc: "+10% max health", max: 8, base_cost: 300 },
    MetaDef { key: "endurance", name: "Endurance", desc: "+15 stamina, +10% regen", max: 5, base_cost: 300 },
    MetaDef { key: "strength", name: "Strength", desc: "+6% might", max: 8, base_cost: 400 },
    MetaDef { key: "swiftness", name: "Swiftness", desc: "+4% move speed", max: 5, base_cost: 360 },
    MetaDef { key: "haste", name: "Haste", desc: "-4% weapon cooldown", max: 5, base_cost: 500 },
    MetaDef { key: "fortune", name: "Fortune", desc: "+10% luck", max: 5, base_cost: 320 },
    MetaDef { key: "greed", name: "Greed", desc: "+12% souls gained", max: 6, base_cost: 400 },
    MetaDef { key: "ember", name: "Ember", desc: "+1 revival per vigil", max: 3, base_cost: 2000 },
    MetaDef { key: "insight", name: "Insight", desc: "+1 reroll per vigil", max: 4, base_cost: 600 },
    MetaDef { key: "toughness", name: "Toughness", desc: "+1 armor", max: 5, base_cost: 800 },
    MetaDef { key: "renewal", name: "Renewal", desc: "+0.15 health per second", max: 5, base_cost: 600 },
    MetaDef { key: "reach", name: "Reach", desc: "+6% weapon area", max: 5, base_cost: 700 },
    MetaDef { key: "precision", name: "Precision", desc: "+2% critical chance", max: 5, base_cost: 600 },
    MetaDef { key: "magnetism", name: "Magnetism", desc: "+12% pickup range", max: 5, base_cost: 400 },
    MetaDef { key: "scholar", name: "Scholar", desc: "+5% experience", max: 5, base_cost: 700 },
    MetaDef { key: "slayer", name: "Slayer", desc: "+8% damage to elites and bosses", max: 5, base_cost: 800 },
    MetaDef { key: "volley", name: "Volley", desc: "+1 projectile for every weapon", max: 2, base_cost: 25000 },
    MetaDef { key: "colossus", name: "Colossus", desc: "+25 max health", max: 6, base_cost: 1500 },
    MetaDef { key: "warding", name: "Warding", desc: "+0.15 s untouchable after a hit", max: 4, base_cost: 1200 },
    MetaDef { key: "bloodthirst", name: "Bloodthirst", desc: "+0.15 health per kill", max: 4, base_cost: 1500 },
    MetaDef { key: "fleetfoot", name: "Fleetfoot", desc: "Rolls cost 3 less stamina", max: 4, base_cost: 1000 },
    MetaDef { key: "barter", name: "Barter", desc: "Merchant wares cost 8% less", max: 4, base_cost: 1200 },
    MetaDef { key: "foresight", name: "Foresight", desc: "+8% chance of a fourth card", max: 5, base_cost: 1400 },
    MetaDef { key: "second_wind", name: "Second Wind", desc: "Rise again with +10% health", max: 4, base_cost: 1200 },
    MetaDef { key: "kindling", name: "Kindling", desc: "Start with your weapon one level higher", max: 4, base_cost: 2500 },
    MetaDef { key: "whetstone", name: "Whetstone", desc: "+8% damage for max-level weapons", max: 5, base_cost: 3000 },
    MetaDef { key: "fusion_lore", name: "Fusion Lore", desc: "+6% damage for fused weapons", max: 5, base_cost: 5000 },
    MetaDef { key: "evasion", name: "Evasion", desc: "2% chance to avoid a hit", max: 5, base_cost: 4000 },
    MetaDef { key: "bulwark", name: "Bulwark", desc: "-3% damage taken", max: 5, base_cost: 3500 },
    MetaDef { key: "alchemy", name: "Alchemy", desc: "Draughts heal 25% more", max: 4, base_cost: 1000 },
    MetaDef { key: "tithe", name: "Tithe", desc: "Bosses drop 20% more souls", max: 5, base_cost: 2000 },
    MetaDef { key: "arsenal", name: "Arsenal", desc: "+1 weapon slot", max: 2, base_cost: 150000 },
    MetaDef { key: "satchel", name: "Satchel", desc: "+1 item slot", max: 2, base_cost: 120000 },
];

pub fn meta_cost(i: usize, level: u32) -> u64 {
    (META[i].base_cost as f64 * 1.75f64.powi(level as i32)).round() as u64
}

// ---------------------------------------------------------------- reliquary (ashen marks)

/// Something bought once with Ashen Marks.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Unlock {
    Class(Class),
    Weapon(WeaponKind),
    Item(ItemKind),
}

pub const UNLOCKS: [(Unlock, u64); 11] = [
    (Unlock::Class(Class::Reaper), 45),
    (Unlock::Class(Class::Hexblade), 45),
    (Unlock::Class(Class::BloodKnight), 60),
    (Unlock::Class(Class::Arbalist), 60),
    (Unlock::Weapon(WeaponKind::Scythe), 22),
    (Unlock::Weapon(WeaponKind::Hex), 22),
    (Unlock::Weapon(WeaponKind::Lance), 30),
    (Unlock::Weapon(WeaponKind::Crossbow), 30),
    (Unlock::Item(ItemKind::LeechFang), 15),
    (Unlock::Item(ItemKind::CrackedCrown), 15),
    (Unlock::Item(ItemKind::GraveCandle), 18),
];

impl Unlock {
    /// Bit in the unlock mask: weapons 0..16, items 16..32, classes 32..48.
    pub fn bit(self) -> u64 {
        match self {
            Unlock::Weapon(w) => 1 << w.idx(),
            Unlock::Item(i) => 1 << (16 + i.idx()),
            Unlock::Class(c) => 1 << (32 + class_to_u8(c) as usize),
        }
    }

    pub fn label(self) -> String {
        match self {
            Unlock::Class(c) => format!("Class: {}", c.name()),
            Unlock::Weapon(w) => format!("Weapon: {}", w.name(false)),
            Unlock::Item(i) => format!("Item: {}", i.name()),
        }
    }

    pub fn desc(self) -> &'static str {
        match self {
            Unlock::Class(c) => c.desc(),
            Unlock::Weapon(w) => w.desc(),
            Unlock::Item(i) => i.desc(),
        }
    }
}

/// Everything available without buying anything.
pub fn default_unlocks() -> u64 {
    let mut m = 0;
    for w in &ALL_WEAPONS[..8] {
        m |= Unlock::Weapon(*w).bit();
    }
    for i in &ALL_ITEMS[..10] {
        m |= Unlock::Item(*i).bit();
    }
    for c in &ALL_CLASSES[..4] {
        m |= Unlock::Class(*c).bit();
    }
    m
}

pub const RELIC_COUNT: usize = 12;

pub struct RelicDef {
    pub key: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub max: u32,
    pub base_cost: u64,
}

pub const RELICS: [RelicDef; RELIC_COUNT] = [
    RelicDef { key: "relic_blessing", name: "Ashen Blessing", desc: "+5% might", max: 4, base_cost: 22 },
    RelicDef { key: "relic_fortified", name: "Fortified Soul", desc: "+15 max health", max: 4, base_cost: 18 },
    RelicDef { key: "relic_pockets", name: "Deep Pockets", desc: "+1 reroll per vigil", max: 2, base_cost: 38 },
    RelicDef { key: "relic_headstart", name: "Head Start", desc: "Begin each vigil with a level-up", max: 2, base_cost: 45 },
    RelicDef { key: "relic_lantern", name: "Soul Lantern", desc: "+20% soul pickup range", max: 3, base_cost: 15 },
    RelicDef { key: "relic_vial", name: "Ember Vial", desc: "+1 revival per vigil", max: 1, base_cost: 90 },
    RelicDef { key: "relic_crown", name: "Ashen Crown", desc: "+5% souls gained", max: 3, base_cost: 25 },
    RelicDef { key: "relic_hands", name: "Quick Hands", desc: "-3% weapon cooldown", max: 3, base_cost: 35 },
    RelicDef { key: "relic_iron", name: "Iron Will", desc: "+1 armor", max: 2, base_cost: 40 },
    RelicDef { key: "relic_tome", name: "Scholar's Tome", desc: "+5% experience", max: 3, base_cost: 25 },
    RelicDef { key: "relic_stride", name: "Pilgrim's Stride", desc: "+5% move speed", max: 2, base_cost: 25 },
    RelicDef { key: "relic_deck", name: "Fate's Deck", desc: "+15% luck", max: 2, base_cost: 35 },
];

pub fn relic_cost(i: usize, level: u32) -> u64 {
    RELICS[i].base_cost * (level as u64 + 1)
}

/// Ashen Marks earned at the end of a vigil.
pub fn marks_for_run(minutes: f32, bosses: u32, level: u32, kills: u32, won: bool, diff: &Difficulty) -> u64 {
    let base = minutes * 1.5 + bosses as f32 * 6.0 + level as f32 * 0.4 + kills as f32 / 150.0 + if won { 25.0 } else { 0.0 };
    ((base * diff.soul_mult()).round() as u64).max(1)
}

/// Permanent progression a player brings into a vigil.
#[derive(Clone, Copy, Debug)]
pub struct Profile {
    pub meta: [u32; META_COUNT],
    pub relics: [u32; RELIC_COUNT],
    pub unlocks: u64,
    /// The hero skin worn, 0 for the class look.
    pub skin: u8,
}

impl Default for Profile {
    fn default() -> Self {
        Profile { meta: [0; META_COUNT], relics: [0; RELIC_COUNT], unlocks: default_unlocks(), skin: 0 }
    }
}

impl Profile {
    pub fn has(&self, u: Unlock) -> bool {
        self.unlocks & u.bit() != 0
    }

    /// Level of a shrine upgrade.
    pub fn m(&self, i: usize) -> u32 {
        self.meta.get(i).copied().unwrap_or(0)
    }
}

// ---------------------------------------------------------------- skins

/// What a skin puts on the hero's head.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Helm {
    /// The helm of a class.
    Class(Class),
    PlagueMask,
    Pumpkin,
    SkullCrown,
    Antlers,
    IceCrown,
    Halo,
    Moth,
    Jester,
    Flame,
    CatHood,
    LanternHood,
    Greathelm,
    StarHat,
    LastCrown,
    Strawberry,
}

/// A look for the hero that wins over the class look.
pub struct SkinDef {
    pub name: &'static str,
    pub desc: &'static str,
    pub cost: u64,
    /// Tunic and cape.
    pub body: (u8, u8, u8),
    /// Breastplate and pauldrons.
    pub metal: (u8, u8, u8),
    pub helm: Helm,
}

/// Skin 0 is the class look; these are bought with souls in the Wardrobe.
pub const SKINS: [SkinDef; 18] = [
    SkinDef { name: "Ashen Knight", desc: "Grey as the dust of old fires.", cost: 2_000, body: (96, 92, 90), metal: (150, 150, 156), helm: Helm::Class(Class::Wanderer) },
    SkinDef { name: "Tin Pail", desc: "A bucket for any class. Clang.", cost: 3_000, body: (90, 70, 50), metal: (140, 140, 150), helm: Helm::Class(Class::BucketKnight) },
    SkinDef { name: "Crimson Paladin", desc: "Red cloth and horned steel.", cost: 5_000, body: (150, 24, 30), metal: (190, 170, 120), helm: Helm::Class(Class::BloodKnight) },
    SkinDef { name: "Berry Knight", desc: "Sweet, red and slightly seedy.", cost: 6_000, body: (210, 60, 80), metal: (120, 190, 90), helm: Helm::Strawberry },
    SkinDef { name: "Plague Doctor", desc: "A beaked mask against the miasma.", cost: 8_000, body: (34, 30, 32), metal: (90, 70, 50), helm: Helm::PlagueMask },
    SkinDef { name: "Pumpkin Lord", desc: "A grin that glows all night.", cost: 10_000, body: (200, 100, 30), metal: (70, 110, 50), helm: Helm::Pumpkin },
    SkinDef { name: "Antlered Druid", desc: "The forest walks with you.", cost: 12_000, body: (60, 100, 54), metal: (120, 96, 70), helm: Helm::Antlers },
    SkinDef { name: "Frost Warden", desc: "Pale blue, with a crown of ice.", cost: 15_000, body: (110, 160, 200), metal: (200, 225, 240), helm: Helm::IceCrown },
    SkinDef { name: "Bone Monarch", desc: "A skull wearing a crown, or the other way round.", cost: 15_000, body: (190, 180, 150), metal: (220, 210, 180), helm: Helm::SkullCrown },
    SkinDef { name: "Moth Priest", desc: "Drawn to every flame on the field.", cost: 18_000, body: (150, 130, 160), metal: (190, 170, 150), helm: Helm::Moth },
    SkinDef { name: "Hollow Jester", desc: "The joke is on the dark.", cost: 20_000, body: (110, 40, 140), metal: (60, 150, 90), helm: Helm::Jester },
    SkinDef { name: "Lantern Bearer", desc: "A little light that never goes out.", cost: 22_000, body: (40, 110, 110), metal: (150, 150, 140), helm: Helm::LanternHood },
    SkinDef { name: "Gilded Saint", desc: "Gold leaf over old iron, and a halo.", cost: 25_000, body: (200, 160, 70), metal: (240, 210, 120), helm: Helm::Halo },
    SkinDef { name: "Ember Wraith", desc: "A head of living fire.", cost: 30_000, body: (40, 24, 22), metal: (90, 60, 50), helm: Helm::Flame },
    SkinDef { name: "Iron Duke", desc: "A great helm and a proud plume.", cost: 35_000, body: (70, 74, 90), metal: (175, 178, 190), helm: Helm::Greathelm },
    SkinDef { name: "Grave Cat", desc: "Ears up, tail high, nine lives.", cost: 40_000, body: (26, 24, 30), metal: (60, 56, 66), helm: Helm::CatHood },
    SkinDef { name: "Starlit Wanderer", desc: "The night sky, stitched into a hat.", cost: 50_000, body: (30, 40, 100), metal: (150, 160, 210), helm: Helm::StarHat },
    SkinDef { name: "The Last Flame", desc: "White fire and a burning crown.", cost: 250_000, body: (235, 225, 205), metal: (255, 220, 140), helm: Helm::LastCrown },
];

/// The skin with number `skin`, if any; 0 is the class look.
pub fn skin_of(skin: u8) -> Option<&'static SkinDef> {
    (skin as usize).checked_sub(1).and_then(|i| SKINS.get(i))
}

// ---------------------------------------------------------------- stats

/// Player stats after items, class and permanent upgrades.
#[derive(Clone, Copy, Debug)]
pub struct Stats {
    pub might: f32,
    pub cooldown: f32,
    pub area: f32,
    pub amount: u32,
    pub speed: f32,
    pub magnet: f32,
    pub luck: f32,
    pub armor: f32,
    pub regen: f32,
    pub max_hp: f32,
    pub stamina_max: f32,
    pub stamina_regen: f32,
    pub greed: f32,
    pub crit: f32,
    pub lifesteal: f32,
    pub growth: f32,
    pub elite_dmg: f32,
}

impl Stats {
    pub fn to_array(self) -> [f32; 17] {
        [
            self.might,
            self.cooldown,
            self.area,
            self.amount as f32,
            self.speed,
            self.magnet,
            self.luck,
            self.armor,
            self.regen,
            self.max_hp,
            self.stamina_max,
            self.stamina_regen,
            self.greed,
            self.crit,
            self.lifesteal,
            self.growth,
            self.elite_dmg,
        ]
    }

    pub fn from_array(f: [f32; 17]) -> Stats {
        Stats {
            might: f[0],
            cooldown: f[1],
            area: f[2],
            amount: f[3] as u32,
            speed: f[4],
            magnet: f[5],
            luck: f[6],
            armor: f[7],
            regen: f[8],
            max_hp: f[9],
            stamina_max: f[10],
            stamina_regen: f[11],
            greed: f[12],
            crit: f[13],
            lifesteal: f[14],
            growth: f[15],
            elite_dmg: f[16],
        }
    }
}

// ---------------------------------------------------------------- buffs

/// Temporary blessings from relics found on the map.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Buff {
    Fury,
    Haste,
    Aegis,
}

pub const ALL_BUFFS: [Buff; 3] = [Buff::Fury, Buff::Haste, Buff::Aegis];

impl Buff {
    pub fn name(self) -> &'static str {
        match self {
            Buff::Fury => "Bloodstone",
            Buff::Haste => "Quicksilver",
            Buff::Aegis => "Aegis Ash",
        }
    }

    pub fn desc(self) -> &'static str {
        match self {
            Buff::Fury => "+50% might",
            Buff::Haste => "weapons 40% faster",
            Buff::Aegis => "untouchable",
        }
    }

    pub fn duration(self) -> f32 {
        match self {
            Buff::Fury => 15.0,
            Buff::Haste => 15.0,
            Buff::Aegis => 6.0,
        }
    }

    pub fn color(self) -> Color {
        match self {
            Buff::Fury => Color::from_rgba(220, 30, 40, 255),
            Buff::Haste => Color::from_rgba(190, 200, 220, 255),
            Buff::Aegis => Color::from_rgba(255, 210, 120, 255),
        }
    }
}

// ---------------------------------------------------------------- difficulty

/// Enemy tuning chosen in the options; the host's values apply online.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Difficulty {
    pub count: f32,
    pub hp: f32,
    pub dmg: f32,
    pub speed: f32,
    pub boss_hp: f32,
}

/// Seconds over which a harder difficulty ramps up to its full strength.
const DIFF_RAMP: f32 = 180.0;

impl Difficulty {
    /// The difficulty in effect at `time`: harder settings start at a third of their extra
    /// strength and reach all of it after three minutes, so the first wave can be learned.
    pub fn at(self, time: f32) -> Difficulty {
        let k = 0.33 + 0.67 * (time / DIFF_RAMP).clamp(0.0, 1.0);
        let ramp = |v: f32| if v > 1.0 { 1.0 + (v - 1.0) * k } else { v };
        Difficulty { count: ramp(self.count), hp: ramp(self.hp), dmg: ramp(self.dmg), speed: ramp(self.speed), boss_hp: self.boss_hp }
    }
}

pub const DIFF_COUNT: [f32; 6] = [0.5, 0.75, 1.0, 1.5, 2.0, 3.0];
pub const DIFF_HP: [f32; 7] = [0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 4.0];
pub const DIFF_DMG: [f32; 6] = [0.5, 0.75, 1.0, 1.5, 2.0, 3.0];
pub const DIFF_SPEED: [f32; 6] = [0.8, 0.9, 1.0, 1.15, 1.3, 1.5];
pub const DIFF_BOSS: [f32; 6] = [0.5, 0.75, 1.0, 1.5, 2.0, 3.0];

pub const PRESETS: [(&str, Difficulty); 4] = [
    ("Pilgrim", Difficulty { count: 0.75, hp: 0.75, dmg: 0.5, speed: 0.9, boss_hp: 0.75 }),
    ("Knight", Difficulty { count: 1.0, hp: 1.0, dmg: 1.0, speed: 1.0, boss_hp: 1.0 }),
    ("Lord", Difficulty { count: 1.5, hp: 1.5, dmg: 1.5, speed: 1.15, boss_hp: 1.5 }),
    ("Abyss", Difficulty { count: 2.0, hp: 3.0, dmg: 2.0, speed: 1.3, boss_hp: 3.0 }),
];

impl Default for Difficulty {
    fn default() -> Self {
        PRESETS[1].1
    }
}

impl Difficulty {
    /// Harder settings pay out more souls and marks.
    pub fn soul_mult(&self) -> f32 {
        let m = 1.0 + (self.count - 1.0) * 0.3 + (self.hp - 1.0) * 0.25 + (self.dmg - 1.0) * 0.3 + (self.speed - 1.0) * 0.8 + (self.boss_hp - 1.0) * 0.1;
        m.clamp(0.4, 4.0)
    }

    pub fn preset_name(&self) -> &'static str {
        PRESETS.iter().find(|p| p.1 == *self).map(|p| p.0).unwrap_or("Custom")
    }
}

/// Moves `cur` one step through `list` in direction `dir`.
pub fn step(list: &[f32], cur: f32, dir: i32) -> f32 {
    let i = list.iter().position(|v| (*v - cur).abs() < 0.001).unwrap_or(list.len() / 2) as i32;
    list[(i + dir).clamp(0, list.len() as i32 - 1) as usize]
}

// ---------------------------------------------------------------- challenges

/// Optional handicaps for a single-player vigil; each adds 25% Ashen Marks.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Challenge {
    NoRolls,
    GlassCannon,
    OneWeapon,
}

pub const ALL_CHALLENGES: [Challenge; 3] = [Challenge::NoRolls, Challenge::GlassCannon, Challenge::OneWeapon];

impl Challenge {
    pub fn bit(self) -> u8 {
        1 << self as u8
    }

    pub fn name(self) -> &'static str {
        match self {
            Challenge::NoRolls => "Rooted",
            Challenge::GlassCannon => "Glass Cannon",
            Challenge::OneWeapon => "One Blade",
        }
    }

    pub fn desc(self) -> &'static str {
        match self {
            Challenge::NoRolls => "You cannot roll.",
            Challenge::GlassCannon => "Half health, +60% might.",
            Challenge::OneWeapon => "Your starting weapon is all you get.",
        }
    }
}

pub fn challenge_mult(bits: u8) -> f32 {
    1.0 + 0.25 * bits.count_ones() as f32
}

// ---------------------------------------------------------------- achievements

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Achievement {
    FirstLight,
    Gravebreaker,
    Dawnbringer,
    AbyssWalker,
    Legion,
    Fusionist,
    Evolved,
    Steadfast,
    CatPerson,
    Patron,
    MoonWatcher,
    TrialEndured,
    Purist,
    Fellowship,
    GrandFusion,
    Singularity,
    LongNight,
    Centurion,
    AshAndBone,
    Executioner,
    Untouched,
    LastKeeper,
    Fireside,
    ColdEmbers,
    SoftHeart,
    Clang,
    Hoarder,
    FullHouse,
    RollModel,
    Banker,
    HalfNight,
    DeepNight,
    Ascendant,
    BeyondMeasure,
    ThinTheHerd,
    SeaOfAsh,
    Extinction,
    Lordbane,
    Kingslayer,
    LastFlame,
    FullArsenal,
    Packrat,
    Polished,
    Eternal,
    Nightless,
    GentleDawn,
    KnightsDawn,
    LordsDawn,
    ShatteredGlass,
    DeepRoots,
    Martyr,
    MillionCuts,
    Unmaking,
    HoldHands,
    Purrfect,
    CatHerder,
    OldRegular,
    ManyFaces,
    TinCan,
    Dressed,
    Wardrobe,
    Devoted,
    Ghost,
    Dizzy,
    Impatient,
    Partygoer,
    Gauntlet,
}

pub const ALL_ACHIEVEMENTS: [Achievement; 67] = [
    Achievement::FirstLight,
    Achievement::Gravebreaker,
    Achievement::Dawnbringer,
    Achievement::AbyssWalker,
    Achievement::Legion,
    Achievement::Fusionist,
    Achievement::Evolved,
    Achievement::Steadfast,
    Achievement::CatPerson,
    Achievement::Patron,
    Achievement::MoonWatcher,
    Achievement::TrialEndured,
    Achievement::Purist,
    Achievement::Fellowship,
    Achievement::GrandFusion,
    Achievement::Singularity,
    Achievement::LongNight,
    Achievement::Centurion,
    Achievement::AshAndBone,
    Achievement::Executioner,
    Achievement::Untouched,
    Achievement::LastKeeper,
    Achievement::Fireside,
    Achievement::ColdEmbers,
    Achievement::SoftHeart,
    Achievement::Clang,
    Achievement::Hoarder,
    Achievement::FullHouse,
    Achievement::RollModel,
    Achievement::Banker,
    Achievement::HalfNight,
    Achievement::DeepNight,
    Achievement::Ascendant,
    Achievement::BeyondMeasure,
    Achievement::ThinTheHerd,
    Achievement::SeaOfAsh,
    Achievement::Extinction,
    Achievement::Lordbane,
    Achievement::Kingslayer,
    Achievement::LastFlame,
    Achievement::FullArsenal,
    Achievement::Packrat,
    Achievement::Polished,
    Achievement::Eternal,
    Achievement::Nightless,
    Achievement::GentleDawn,
    Achievement::KnightsDawn,
    Achievement::LordsDawn,
    Achievement::ShatteredGlass,
    Achievement::DeepRoots,
    Achievement::Martyr,
    Achievement::MillionCuts,
    Achievement::Unmaking,
    Achievement::HoldHands,
    Achievement::Purrfect,
    Achievement::CatHerder,
    Achievement::OldRegular,
    Achievement::ManyFaces,
    Achievement::TinCan,
    Achievement::Dressed,
    Achievement::Wardrobe,
    Achievement::Devoted,
    Achievement::Ghost,
    Achievement::Dizzy,
    Achievement::Impatient,
    Achievement::Partygoer,
    Achievement::Gauntlet,
];

impl Achievement {
    pub fn bit(self) -> u128 {
        1 << self as u128
    }

    pub fn name(self) -> &'static str {
        use Achievement::*;
        match self {
            FirstLight => "First Light",
            Gravebreaker => "Gravebreaker",
            Dawnbringer => "Dawnbringer",
            AbyssWalker => "Abyss Walker",
            Legion => "Legion's End",
            Fusionist => "Fusionist",
            Evolved => "Transfigured",
            Steadfast => "Steadfast",
            CatPerson => "Cat Person",
            Patron => "Patron",
            MoonWatcher => "Moon Watcher",
            TrialEndured => "Trial by Curse",
            Purist => "Purist",
            Fellowship => "Fellowship",
            GrandFusion => "Grand Fusion",
            Singularity => "Singularity",
            LongNight => "The Long Night",
            Centurion => "Centurion",
            AshAndBone => "Ash and Bone",
            Executioner => "Executioner",
            Untouched => "Untouched",
            LastKeeper => "The Last Keeper",
            Fireside => "Fireside",
            ColdEmbers => "Cold Embers",
            SoftHeart => "Soft Heart",
            Clang => "Clang",
            Hoarder => "Hoarder",
            FullHouse => "Full House",
            RollModel => "Roll Model",
            Banker => "Banker",
            HalfNight => "Halfway There",
            DeepNight => "Deep Night",
            Ascendant => "Ascendant",
            BeyondMeasure => "Beyond Measure",
            ThinTheHerd => "Thinning the Herd",
            SeaOfAsh => "Sea of Ash",
            Extinction => "Extinction",
            Lordbane => "Lordbane",
            Kingslayer => "Kingslayer",
            LastFlame => "The Last Flame",
            FullArsenal => "Full Arsenal",
            Packrat => "Packrat",
            Polished => "Polished",
            Eternal => "Eternal",
            Nightless => "Nightless",
            GentleDawn => "Gentle Dawn",
            KnightsDawn => "Knight's Dawn",
            LordsDawn => "Lord's Dawn",
            ShatteredGlass => "Shattered Glass",
            DeepRoots => "Deep Roots",
            Martyr => "Martyr",
            MillionCuts => "A Million Cuts",
            Unmaking => "The Unmaking",
            HoldHands => "Hold Hands Until Dawn",
            Purrfect => "Purrfect",
            CatHerder => "Cat Herder",
            OldRegular => "Old Regular",
            ManyFaces => "Many Faces",
            TinCan => "Tin Can Triumph",
            Dressed => "Dressed for the Dark",
            Wardrobe => "Wardrobe",
            Devoted => "Devoted",
            Ghost => "Ghost",
            Dizzy => "Dizzy",
            Impatient => "Impatient",
            Partygoer => "Partygoer",
            Gauntlet => "The Gauntlet",
        }
    }

    pub fn desc(self) -> &'static str {
        use Achievement::*;
        match self {
            FirstLight => "Survive five minutes.",
            Gravebreaker => "Defeat a boss.",
            Dawnbringer => "Keep a vigil until dawn.",
            AbyssWalker => "Keep a vigil until dawn on Abyss.",
            Legion => "Fell 10,000 foes in one vigil.",
            Fusionist => "Fuse two weapons.",
            Evolved => "Evolve a weapon.",
            Steadfast => "Survive ten minutes without rolling.",
            CatPerson => "Befriend the Grave Cat.",
            Patron => "Buy three wares in one vigil.",
            MoonWatcher => "Survive a Blood Moon.",
            TrialEndured => "Endure a cursed shrine's trial.",
            Purist => "Keep a vigil until dawn with One Blade.",
            Fellowship => "Survive ten minutes online with a companion.",
            GrandFusion => "Make a mega fusion of four weapons.",
            Singularity => "Make an ultra fusion of eight weapons.",
            LongNight => "Fight on ten minutes past dawn.",
            Centurion => "Reach level 100 in one vigil.",
            AshAndBone => "Fell 100,000 foes in one vigil.",
            Executioner => "Slay a boss within ten seconds of its arrival.",
            Untouched => "Take no damage in the first three minutes.",
            LastKeeper => "Find the lost grave.",
            Fireside => "Rest a while by a small fire.",
            ColdEmbers => "Stir the embers in the main menu.",
            SoftHeart => "Find out what is cute.",
            Clang => "Find the Bucket Knight.",
            Hoarder => "Carry 100,000 souls at once.",
            FullHouse => "Keep a vigil with four players.",
            RollModel => "Roll 1,000 times in one vigil.",
            Banker => "Bank 50,000 souls in one vigil.",
            HalfNight => "Survive fifteen minutes.",
            DeepNight => "Survive twenty minutes.",
            Ascendant => "Reach level 50 in one vigil.",
            BeyondMeasure => "Reach level 200 in one vigil.",
            ThinTheHerd => "Fell 1,000 foes in one vigil.",
            SeaOfAsh => "Fell 50,000 foes in one vigil.",
            Extinction => "Fell 250,000 foes in one vigil.",
            Lordbane => "Defeat three bosses in one vigil.",
            Kingslayer => "Defeat ten bosses in one vigil.",
            LastFlame => "Let every weapon become one.",
            FullArsenal => "Carry six weapons at once.",
            Packrat => "Carry six items at once.",
            Polished => "Raise an item to its highest level.",
            Eternal => "Fight on thirty minutes past dawn.",
            Nightless => "Fight on sixty minutes past dawn.",
            GentleDawn => "Keep a vigil until dawn on Pilgrim.",
            KnightsDawn => "Keep a vigil until dawn on Knight.",
            LordsDawn => "Keep a vigil until dawn on Lord.",
            ShatteredGlass => "Keep a vigil until dawn as a Glass Cannon.",
            DeepRoots => "Keep a vigil until dawn while Rooted.",
            Martyr => "Keep a vigil until dawn with all three challenges.",
            MillionCuts => "Deal 1,000,000 damage in one vigil.",
            Unmaking => "Deal 1,000,000,000 damage in one vigil.",
            HoldHands => "Keep a vigil until dawn in cute mode.",
            Purrfect => "Say meow in the main menu.",
            CatHerder => "Befriend the Grave Cat ten times.",
            OldRegular => "Keep 25 vigils.",
            ManyFaces => "Keep a vigil until dawn with four different classes.",
            TinCan => "Keep a vigil until dawn as the Bucket Knight.",
            Dressed => "Buy a hero skin.",
            Wardrobe => "Own ten hero skins.",
            Devoted => "Buy 50 levels at the Ash Shrine.",
            Ghost => "Take no damage in the first five minutes.",
            Dizzy => "Roll ten times in four seconds.",
            Impatient => "Ask for dawn in the main menu.",
            Partygoer => "Start a party in the main menu.",
            Gauntlet => "Defeat all twelve lords of a boss rush.",
        }
    }

    /// Ashen Marks paid once when it is earned.
    pub fn reward(self) -> u64 {
        use Achievement::*;
        match self {
            FirstLight => 5,
            Gravebreaker | Evolved => 10,
            Fusionist | CatPerson | Patron | MoonWatcher | Fellowship => 15,
            Steadfast | TrialEndured => 20,
            Legion => 25,
            Dawnbringer => 30,
            Purist => 50,
            AbyssWalker => 60,
            Fireside | ColdEmbers | SoftHeart => 10,
            LastKeeper | Clang => 15,
            Untouched | Hoarder | FullHouse | RollModel | Banker => 20,
            GrandFusion | Executioner => 25,
            Centurion => 30,
            LongNight | AshAndBone => 40,
            Singularity => 60,
            Gauntlet => 50,
            ThinTheHerd | Polished => 5,
            FullArsenal | Packrat | GentleDawn | Purrfect | Dressed | Dizzy | Impatient | Partygoer => 10,
            HalfNight | Ascendant => 15,
            DeepNight | Lordbane | MillionCuts | HoldHands | OldRegular => 20,
            KnightsDawn | CatHerder => 25,
            SeaOfAsh | TinCan | Devoted => 30,
            BeyondMeasure | Kingslayer | LordsDawn | ShatteredGlass | DeepRoots | ManyFaces | Wardrobe | Ghost => 40,
            Extinction | Eternal | Unmaking => 60,
            LastFlame => 80,
            Nightless | Martyr => 100,
        }
    }
}

/// One finished vigil on the leaderboard.
#[derive(Clone, Debug, PartialEq)]
pub struct RunRecord {
    pub time: f32,
    pub kills: u32,
    pub level: u32,
    pub players: u32,
    pub won: bool,
    pub class: Class,
    pub difficulty: String,
    pub damage: f64,
}

impl RunRecord {
    /// Wins first, then the longest vigils, then kills.
    pub fn better_than(&self, o: &RunRecord) -> bool {
        (self.won, self.time as i64, self.kills) > (o.won, o.time as i64, o.kills)
    }
}
