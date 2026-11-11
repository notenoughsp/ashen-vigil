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
/// The name of the weapon that holds all the others, on the graveyard and in the cathedral.
pub const LAST_FLAME: &str = "The Last Flame";
pub const LAST_HYMN: &str = "The Final Hymn";
pub const LAST_GOD: &str = "The Hungry God";
pub const LAST_GROVE: &str = "The Burning Grove";
pub const LAST_WINTER: &str = "The Endless Winter";
pub const LAST_BLOOM: &str = "The Black Bloom";

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
    // The cathedral's own arsenal, in the same order: each mirrors the graveyard weapon twelve
    // places before it, and fuses with the mirrors of that weapon's partners.
    Rosary,
    Nails,
    HolyWater,
    Halo,
    Censer,
    Judgement,
    Gargoyle,
    Runes,
    Crozier,
    Candles,
    Organ,
    Glass,
    // The flesh pits' arsenal, in the same order again.
    Cleaver,
    Teeth,
    Sac,
    Heart,
    Eyes,
    Spikes,
    Jaw,
    Motes,
    Ribcage,
    Steps,
    Spine,
    Tendrils,
    // The Ashwood's, the Frozen Abbey's and the Starless Garden's arsenals, in the same order.
    // Each of their weapons works like an older weapon in its place (`twin`), with a look,
    // a name and an element of its own.
    AshHatchet,
    AshShards,
    AshSling,
    AshBell,
    AshShield,
    AshRoots,
    AshBrand,
    AshFlies,
    AshSickle,
    AshTrail,
    AshPike,
    AshBurst,
    IceScourge,
    IceNeedles,
    IceFlask,
    IceHalo,
    IceRosary,
    IceFall,
    IceOwl,
    IceRunes,
    IceCrook,
    IceBreath,
    IceLance,
    IcePanes,
    ThornWhip,
    ThornVolley,
    ThornBloom,
    ThornMirror,
    ThornMoths,
    ThornSnare,
    ThornShears,
    ThornSpores,
    ThornHook,
    ThornTrail,
    ThornLash,
    ThornRoots,
}

/// Weapons of one field's arsenal.
pub const ARSENAL: usize = 12;

/// The graveyard's weapons, then the cathedral's.
pub const GRAVE_WEAPONS: [WeaponKind; ARSENAL] = [
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
pub const CATHEDRAL_WEAPONS: [WeaponKind; ARSENAL] = [
    WeaponKind::Rosary,
    WeaponKind::Nails,
    WeaponKind::HolyWater,
    WeaponKind::Halo,
    WeaponKind::Censer,
    WeaponKind::Judgement,
    WeaponKind::Gargoyle,
    WeaponKind::Runes,
    WeaponKind::Crozier,
    WeaponKind::Candles,
    WeaponKind::Organ,
    WeaponKind::Glass,
];

pub const FLESH_WEAPONS: [WeaponKind; ARSENAL] = [
    WeaponKind::Cleaver,
    WeaponKind::Teeth,
    WeaponKind::Sac,
    WeaponKind::Heart,
    WeaponKind::Eyes,
    WeaponKind::Spikes,
    WeaponKind::Jaw,
    WeaponKind::Motes,
    WeaponKind::Ribcage,
    WeaponKind::Steps,
    WeaponKind::Spine,
    WeaponKind::Tendrils,
];

pub const ASH_WEAPONS: [WeaponKind; ARSENAL] = [
    WeaponKind::AshHatchet,
    WeaponKind::AshShards,
    WeaponKind::AshSling,
    WeaponKind::AshBell,
    WeaponKind::AshShield,
    WeaponKind::AshRoots,
    WeaponKind::AshBrand,
    WeaponKind::AshFlies,
    WeaponKind::AshSickle,
    WeaponKind::AshTrail,
    WeaponKind::AshPike,
    WeaponKind::AshBurst,
];
pub const ABBEY_WEAPONS: [WeaponKind; ARSENAL] = [
    WeaponKind::IceScourge,
    WeaponKind::IceNeedles,
    WeaponKind::IceFlask,
    WeaponKind::IceHalo,
    WeaponKind::IceRosary,
    WeaponKind::IceFall,
    WeaponKind::IceOwl,
    WeaponKind::IceRunes,
    WeaponKind::IceCrook,
    WeaponKind::IceBreath,
    WeaponKind::IceLance,
    WeaponKind::IcePanes,
];
pub const GARDEN_WEAPONS: [WeaponKind; ARSENAL] = [
    WeaponKind::ThornWhip,
    WeaponKind::ThornVolley,
    WeaponKind::ThornBloom,
    WeaponKind::ThornMirror,
    WeaponKind::ThornMoths,
    WeaponKind::ThornSnare,
    WeaponKind::ThornShears,
    WeaponKind::ThornSpores,
    WeaponKind::ThornHook,
    WeaponKind::ThornTrail,
    WeaponKind::ThornLash,
    WeaponKind::ThornRoots,
];

pub const ALL_WEAPONS: [WeaponKind; 72] = [
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
    WeaponKind::Rosary,
    WeaponKind::Nails,
    WeaponKind::HolyWater,
    WeaponKind::Halo,
    WeaponKind::Censer,
    WeaponKind::Judgement,
    WeaponKind::Gargoyle,
    WeaponKind::Runes,
    WeaponKind::Crozier,
    WeaponKind::Candles,
    WeaponKind::Organ,
    WeaponKind::Glass,
    WeaponKind::Cleaver,
    WeaponKind::Teeth,
    WeaponKind::Sac,
    WeaponKind::Heart,
    WeaponKind::Eyes,
    WeaponKind::Spikes,
    WeaponKind::Jaw,
    WeaponKind::Motes,
    WeaponKind::Ribcage,
    WeaponKind::Steps,
    WeaponKind::Spine,
    WeaponKind::Tendrils,
    WeaponKind::AshHatchet,
    WeaponKind::AshShards,
    WeaponKind::AshSling,
    WeaponKind::AshBell,
    WeaponKind::AshShield,
    WeaponKind::AshRoots,
    WeaponKind::AshBrand,
    WeaponKind::AshFlies,
    WeaponKind::AshSickle,
    WeaponKind::AshTrail,
    WeaponKind::AshPike,
    WeaponKind::AshBurst,
    WeaponKind::IceScourge,
    WeaponKind::IceNeedles,
    WeaponKind::IceFlask,
    WeaponKind::IceHalo,
    WeaponKind::IceRosary,
    WeaponKind::IceFall,
    WeaponKind::IceOwl,
    WeaponKind::IceRunes,
    WeaponKind::IceCrook,
    WeaponKind::IceBreath,
    WeaponKind::IceLance,
    WeaponKind::IcePanes,
    WeaponKind::ThornWhip,
    WeaponKind::ThornVolley,
    WeaponKind::ThornBloom,
    WeaponKind::ThornMirror,
    WeaponKind::ThornMoths,
    WeaponKind::ThornSnare,
    WeaponKind::ThornShears,
    WeaponKind::ThornSpores,
    WeaponKind::ThornHook,
    WeaponKind::ThornTrail,
    WeaponKind::ThornLash,
    WeaponKind::ThornRoots,
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

    /// The older weapon whose way of fighting this one shares; an older weapon is its own.
    pub fn twin(self) -> WeaponKind {
        use WeaponKind::*;
        match self {
            AshHatchet => Blade,
            AshShards => Daggers,
            AshSling => Orb,
            AshBell => Chime,
            AshShield => Lanterns,
            AshRoots => Spikes,
            AshBrand => Axe,
            AshFlies => Bolt,
            AshSickle => Scythe,
            AshTrail => Steps,
            AshPike => Lance,
            AshBurst => Crossbow,
            IceScourge => Rosary,
            IceNeedles => Nails,
            IceFlask => HolyWater,
            IceHalo => Halo,
            IceRosary => Lanterns,
            IceFall => Judgement,
            IceOwl => Gargoyle,
            IceRunes => Runes,
            IceCrook => Crozier,
            IceBreath => Hex,
            IceLance => Lance,
            IcePanes => Glass,
            ThornWhip => Rosary,
            ThornVolley => Teeth,
            ThornBloom => HolyWater,
            ThornMirror => Halo,
            ThornMoths => Eyes,
            ThornSnare => Spikes,
            ThornShears => Jaw,
            ThornSpores => Motes,
            ThornHook => Crozier,
            ThornTrail => Steps,
            ThornLash => Spine,
            ThornRoots => Tendrils,
            k => k,
        }
    }

    /// The newer fields' weapons: every one fights in a way of its own (its twin only lends
    /// its numbers, its place in the arsenal and the shape of its icon).
    pub fn own_mechanic(self) -> bool {
        self.field() >= 3
    }

    /// How a weapon fights: by its own code, or for most newer ones like their twin.
    pub fn mechanic(self) -> WeaponKind {
        if self.own_mechanic() {
            self
        } else {
            self.twin()
        }
    }

    /// The element of a newer field's weapon, laid on whatever it strikes: 3 fire, 4 frost,
    /// 5 poison; none for the older fields.
    pub fn element(self) -> usize {
        if self.field() >= 3 {
            self.field()
        } else {
            0
        }
    }

    /// Whose arsenal this is: 0 the graveyard's, 1 the cathedral's, 2 the flesh pits'.
    pub fn field(self) -> usize {
        self.idx() / ARSENAL
    }

    /// One of the cathedral's weapons.
    pub fn is_cathedral(self) -> bool {
        self.field() == 1
    }

    /// The graveyard weapon in the same place, which every other arsenal is built on.
    pub fn base(self) -> WeaponKind {
        ALL_WEAPONS[self.idx() % ARSENAL]
    }

    /// This weapon's place in the arsenal of field `field`.
    pub fn for_field(self, field: usize) -> WeaponKind {
        ALL_WEAPONS[(field.min(ALL_WEAPONS.len() / ARSENAL - 1)) * ARSENAL + self.idx() % ARSENAL]
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
            (Rosary, false) => "Rosary Whip",
            (Rosary, true) => "Penitent's Rosary",
            (Nails, false) => "Martyr's Nails",
            (Nails, true) => "Crown of Nails",
            (HolyWater, false) => "Holy Water",
            (HolyWater, true) => "Font of Wrath",
            (Halo, false) => "Halo",
            (Halo, true) => "Corona of Saints",
            (Censer, false) => "Swinging Censer",
            (Censer, true) => "Thurible of Embers",
            (Judgement, false) => "Judgement Blade",
            (Judgement, true) => "Sword of Heaven",
            (Gargoyle, false) => "Gargoyle Familiar",
            (Gargoyle, true) => "Spire Guardian",
            (Runes, false) => "Psalm Runes",
            (Runes, true) => "Living Scripture",
            (Crozier, false) => "Crozier",
            (Crozier, true) => "Shepherd's Hook",
            (Candles, false) => "Candelabrum",
            (Candles, true) => "Altar of Candles",
            (Organ, false) => "Organ Pipes",
            (Organ, true) => "The Great Organ",
            (Glass, false) => "Stained Glass",
            (Glass, true) => "Rose Window",
            (Cleaver, false) => "Bone Cleaver",
            (Cleaver, true) => "Marrow Splitter",
            (Teeth, false) => "Gnashing Teeth",
            (Teeth, true) => "Maw of Many",
            (Sac, false) => "Swollen Sac",
            (Sac, true) => "Bursting Womb",
            (Heart, false) => "Pulsing Heart",
            (Heart, true) => "Drum of the Pit",
            (Eyes, false) => "Watching Eyes",
            (Eyes, true) => "The Unblinking",
            (Spikes, false) => "Impaling Spikes",
            (Spikes, true) => "Forest of Bone",
            (Jaw, false) => "Snapping Jaw",
            (Jaw, true) => "Gnawing Jaw",
            (Motes, false) => "Hungry Motes",
            (Motes, true) => "Red Swarm",
            (Ribcage, false) => "Ribcage Snap",
            (Ribcage, true) => "Iron Maiden of Bone",
            (Steps, false) => "Bleeding Steps",
            (Steps, true) => "River of Blood",
            (Spine, false) => "Spine Lash",
            (Spine, true) => "The Long Spine",
            (Tendrils, false) => "Writhing Tendrils",
            (Tendrils, true) => "Grasping Mass",
            (AshHatchet, false) => "Charred Hatchet",
            (AshHatchet, true) => "Ember Cleft",
            (AshShards, false) => "Cinder Shards",
            (AshShards, true) => "Ember Storm",
            (AshSling, false) => "Spark Sling",
            (AshSling, true) => "Firefall",
            (AshBell, false) => "Ember Bell",
            (AshBell, true) => "Kindling Toll",
            (AshShield, false) => "Bark Shield",
            (AshShield, true) => "Oaken Ward",
            (AshRoots, false) => "Burning Roots",
            (AshRoots, true) => "Wildfire Roots",
            (AshBrand, false) => "Hurled Brand",
            (AshBrand, true) => "Wheel of Cinders",
            (AshFlies, false) => "Fireflies",
            (AshFlies, true) => "Swarm of Sparks",
            (AshSickle, false) => "Ash Sickle",
            (AshSickle, true) => "Burnt Harvest",
            (AshTrail, false) => "Smoulder Trail",
            (AshTrail, true) => "Path of Cinders",
            (AshPike, false) => "Burning Pike",
            (AshPike, true) => "Spear of the Pyre",
            (AshBurst, false) => "Ember Burst",
            (AshBurst, true) => "Ring of Fire",
            (IceScourge, false) => "Icicle Scourge",
            (IceScourge, true) => "Winter's Lash",
            (IceNeedles, false) => "Ice Needles",
            (IceNeedles, true) => "Hail of Needles",
            (IceFlask, false) => "Meltwater Flask",
            (IceFlask, true) => "Black Ice",
            (IceHalo, false) => "Rime Halo",
            (IceHalo, true) => "Crown of Frost",
            (IceRosary, false) => "Frost Rosary",
            (IceRosary, true) => "Winter Chain",
            (IceFall, false) => "Falling Icicles",
            (IceFall, true) => "Avalanche",
            (IceOwl, false) => "Snow Owl",
            (IceOwl, true) => "White Owl of the Abbey",
            (IceRunes, false) => "Frost Runes",
            (IceRunes, true) => "Glyphs of Winter",
            (IceCrook, false) => "Abbess's Crook",
            (IceCrook, true) => "Shepherd of Snow",
            (IceBreath, false) => "Snowburn",
            (IceBreath, true) => "White Silence",
            (IceLance, false) => "Ice Lance",
            (IceLance, true) => "Glacier Spear",
            (IcePanes, false) => "Frost Panes",
            (IcePanes, true) => "Shattered Winter",
            (ThornWhip, false) => "Thorn Whip",
            (ThornWhip, true) => "Rose Hedge",
            (ThornVolley, false) => "Rose Thorns",
            (ThornVolley, true) => "Bramble Volley",
            (ThornBloom, false) => "Bloom Cannon",
            (ThornBloom, true) => "Black Bloom",
            (ThornMirror, false) => "Mirror Pond",
            (ThornMirror, true) => "Starlit Mirror",
            (ThornMoths, false) => "Watching Moths",
            (ThornMoths, true) => "Moths of Night",
            (ThornSnare, false) => "Bramble Snare",
            (ThornSnare, true) => "Strangling Briar",
            (ThornShears, false) => "Garden Shears",
            (ThornShears, true) => "Snipping Shears",
            (ThornSpores, false) => "Pollen Wisps",
            (ThornSpores, true) => "Poison Spores",
            (ThornHook, false) => "Hedge Hook",
            (ThornHook, true) => "Pruning Hook",
            (ThornTrail, false) => "Nightshade Trail",
            (ThornTrail, true) => "Path of Nightshade",
            (ThornLash, false) => "Vine Lash",
            (ThornLash, true) => "Strangler Vine",
            (ThornRoots, false) => "Creeping Roots",
            (ThornRoots, true) => "Root Web",
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
            Rosary => "A string of beads lashes ahead and leaps to the next foe.",
            Nails => "A stream of iron nails at the nearest foes.",
            HolyWater => "A thrown flask that leaves a slowing, searing pool.",
            Halo => "A ring of light pulses out from you and back.",
            Censer => "Censers swing about you in a figure of eight, trailing smoke.",
            Judgement => "A great sword falls from the sky on the wicked.",
            Gargoyle => "A stone familiar swoops at the strongest foe and back.",
            Runes => "Letters of light circle a moment, then seek their foes.",
            Crozier => "A shepherd's crook swept round you, hooking foes off their feet.",
            Candles => "Candles set down where you stand spit flame at foes near them.",
            Organ => "Organ pipes burst from the ground in a line ahead.",
            Glass => "Shards of coloured glass burst all around and split on impact.",
            Cleaver => "A heavy cleave ahead; what it cuts bleeds on.",
            Teeth => "Teeth spat at the nearest foes; each bounces on to one more.",
            Sac => "A swollen sac, thrown, bursts into a spray of blood.",
            Heart => "A heartbeat around you: two rings, the second the harder.",
            Eyes => "Eyes circle you and glare at the foes they see.",
            Spikes => "Spikes of bone burst up under foes and hold them a moment.",
            Jaw => "A severed jaw flung out and back, biting all it passes.",
            Motes => "Seeking drops of blood; each that strikes mends you a little.",
            Ribcage => "Ribs burst up around you and snap shut, dragging foes in.",
            Steps => "Your steps leave a trail of blood that eats at foes.",
            Spine => "A long spine lashes straight ahead through everything.",
            Tendrils => "Tendrils shoot out all around, then snap back.",
            AshHatchet => "Hatchets flung in a widening spiral around you, cleaving all they pass.",
            AshShards => "A blast of burning splinters in a cone, cruellest up close.",
            AshSling => "A glowing ember slung at the nearest foe; it bursts into flame.",
            AshBell => "A bell that tolls around you; every third toll rings out a wave of fire.",
            AshShield => "Plates of burning bark circle you and batter foes.",
            AshRoots => "Burning roots crawl to the foes, leave fire behind and hold what they reach.",
            AshBrand => "A burning brand lobbed onto the foes; it lands in a ring of fire.",
            AshFlies => "Fireflies drift around you and sting the nearest foes with sparks.",
            AshSickle => "A wide sweep of a blackened sickle that sets its path alight.",
            AshTrail => "Your steps leave smouldering ash; a roll leaves a wall of flame.",
            AshPike => "A burning pike thrust; the first foe on its point blows up.",
            AshBurst => "Embers burst out all round, slow, and fall burning to the ground.",
            IceScourge => "A scourge of icicles: what it strikes is chilled, what its tip strikes freezes.",
            IceNeedles => "Volleys of ice needles; each sting gathers cold until the foe freezes.",
            IceFlask => "A flask of meltwater; its pool freezes the foes that wade in it.",
            IceHalo => "A ring of rime that shoves foes out, chills them and slows bullets.",
            IceRosary => "Beads of ice circle you and chill all they touch.",
            IceFall => "Icicles fall where the foes stand: a shadow first, then the ice.",
            IceOwl => "A snow owl swoops at the strongest foe and back.",
            IceRunes => "Frost runes laid in the foes' way; the first to step on one sets it off.",
            IceCrook => "A crook of ice that hooks a row of foes and hauls them in to you.",
            IceBreath => "A breath of cold that slows and gnaws at foes around you.",
            IceLance => "A lance of ice that pierces a whole row.",
            IcePanes => "Panes of ice raised in the foes' way; bullets break on them.",
            ThornWhip => "A thorned whip lashes in an arc; thorns stay and bleed.",
            ThornVolley => "Rose thorns that spring from foe to foe and leave them bleeding.",
            ThornBloom => "Seeds that burst into poison flowers.",
            ThornMirror => "A pond of dark water ripples out around you.",
            ThornMoths => "Moths flutter wide around you and poison what they brush.",
            ThornSnare => "Brambles burst up under the foes, hold them fast and make them bleed.",
            ThornShears => "Shears flung out to snap shut on the foes, then back to you.",
            ThornSpores => "Wisps of pollen drift after foes, burst into poison and mend you.",
            ThornHook => "A hooked swing that drags the foes in its arc in to you, bleeding.",
            ThornTrail => "Nightshade springs up in your steps and bursts when a foe comes near.",
            ThornLash => "A vine lashes the nearest foe and flings it into the ones around it.",
            ThornRoots => "Roots creep out every way from you and hold fast what they grow under.",
        }
    }

    /// Weapons this one fuses with when both reach max level; the first is its classic partner.
    pub fn fusion_partners(self) -> [WeaponKind; 2] {
        use WeaponKind::*;
        if self.field() > 0 {
            let [a, b] = self.base().fusion_partners();
            return [a.for_field(self.field()), b.for_field(self.field())];
        }
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
            _ => unreachable!("cathedral weapons take their partners from the graveyard's"),
        }
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
            Rosary => Color::from_rgba(190, 150, 100, 255),
            Nails => Color::from_rgba(160, 160, 172, 255),
            HolyWater => Color::from_rgba(140, 200, 255, 255),
            Halo => Color::from_rgba(255, 225, 140, 255),
            Censer => Color::from_rgba(200, 175, 120, 255),
            Judgement => Color::from_rgba(225, 230, 255, 255),
            Gargoyle => Color::from_rgba(140, 135, 130, 255),
            Runes => Color::from_rgba(255, 205, 120, 255),
            Crozier => Color::from_rgba(210, 175, 95, 255),
            Candles => Color::from_rgba(255, 180, 90, 255),
            Organ => Color::from_rgba(175, 145, 110, 255),
            Glass => Color::from_rgba(120, 200, 255, 255),
            Cleaver => Color::from_rgba(214, 196, 170, 255),
            Teeth => Color::from_rgba(236, 226, 200, 255),
            Sac => Color::from_rgba(200, 80, 100, 255),
            Heart => Color::from_rgba(220, 40, 60, 255),
            Eyes => Color::from_rgba(240, 200, 80, 255),
            Spikes => Color::from_rgba(226, 214, 190, 255),
            Jaw => Color::from_rgba(200, 120, 110, 255),
            Motes => Color::from_rgba(230, 30, 40, 255),
            Ribcage => Color::from_rgba(220, 206, 180, 255),
            Steps => Color::from_rgba(150, 10, 20, 255),
            Spine => Color::from_rgba(196, 170, 150, 255),
            Tendrils => Color::from_rgba(200, 90, 120, 255),
            AshHatchet => Color::from_rgba(210, 120, 60, 255),
            AshShards => Color::from_rgba(230, 140, 70, 255),
            AshSling => Color::from_rgba(255, 120, 40, 255),
            AshBell => Color::from_rgba(220, 150, 80, 255),
            AshShield => Color::from_rgba(160, 100, 60, 255),
            AshRoots => Color::from_rgba(240, 90, 40, 255),
            AshBrand => Color::from_rgba(250, 150, 60, 255),
            AshFlies => Color::from_rgba(255, 200, 90, 255),
            AshSickle => Color::from_rgba(190, 110, 70, 255),
            AshTrail => Color::from_rgba(200, 80, 30, 255),
            AshPike => Color::from_rgba(255, 110, 50, 255),
            AshBurst => Color::from_rgba(255, 170, 80, 255),
            IceScourge => Color::from_rgba(170, 210, 240, 255),
            IceNeedles => Color::from_rgba(200, 230, 255, 255),
            IceFlask => Color::from_rgba(140, 190, 235, 255),
            IceHalo => Color::from_rgba(210, 235, 255, 255),
            IceRosary => Color::from_rgba(150, 210, 250, 255),
            IceFall => Color::from_rgba(180, 220, 245, 255),
            IceOwl => Color::from_rgba(235, 240, 250, 255),
            IceRunes => Color::from_rgba(120, 180, 230, 255),
            IceCrook => Color::from_rgba(190, 215, 235, 255),
            IceBreath => Color::from_rgba(225, 240, 255, 255),
            IceLance => Color::from_rgba(130, 200, 245, 255),
            IcePanes => Color::from_rgba(180, 225, 250, 255),
            ThornWhip => Color::from_rgba(110, 160, 70, 255),
            ThornVolley => Color::from_rgba(150, 190, 90, 255),
            ThornBloom => Color::from_rgba(170, 110, 200, 255),
            ThornMirror => Color::from_rgba(90, 120, 200, 255),
            ThornMoths => Color::from_rgba(200, 190, 150, 255),
            ThornSnare => Color::from_rgba(90, 140, 60, 255),
            ThornShears => Color::from_rgba(190, 190, 200, 255),
            ThornSpores => Color::from_rgba(220, 210, 120, 255),
            ThornHook => Color::from_rgba(120, 150, 90, 255),
            ThornTrail => Color::from_rgba(130, 70, 160, 255),
            ThornLash => Color::from_rgba(80, 150, 70, 255),
            ThornRoots => Color::from_rgba(110, 90, 60, 255),
        }
    }

    /// Numbers for a weapon at a given level, before player stats are applied.
    /// What the weapon does at `level`: its numbers as designed, with its damage tuned by
    /// `tune` so the arsenals stand level with one another.
    pub fn stats(self, level: u32, evolved: bool) -> WStat {
        let mut s = self.stats_raw(level, evolved);
        s.dmg *= self.tune();
        s
    }

    /// The balance pass: how much of its designed damage each weapon keeps. Measured alone
    /// against the same horde (`weapon_dps_report`), pools, lines through a crowd and falling
    /// things dealt many times what bolts, orbiters and trails did; this brings them into one
    /// band. Weapons that also freeze, ward, block or heal stay a little under the others.
    pub fn tune(self) -> f32 {
        use WeaponKind::*;
        match self {
            // Far above the rest: held back.
            AshPike => 0.26,
            HolyWater => 0.35,
            IceFlask | ThornBloom => 0.38,
            ThornLash => 0.41,
            Judgement => 0.45,
            IceLance => 0.47,
            IceFall => 0.48,
            AshRoots | ThornSnare => 0.5,
            AshBrand => 0.65,
            IceRunes => 0.69,
            Spine => 0.84,
            Spear => 0.85,
            ThornShears => 0.88,
            Spikes => 0.9,
            Orb => 0.85,
            // Far below: lifted.
            Tendrils => 1.1,
            AshSling => 1.15,
            ThornTrail => 1.2,
            Lanterns => 1.9,
            Chime => 1.6,
            // The Frozen Abbey's arsenal freezes more than it kills; a night there fell far
            // behind the other fields, so its quieter weapons are lifted further.
            IceScourge => 1.25,
            IceBreath | IcePanes => 1.4,
            IceOwl => 1.8,
            Crossbow | ThornMirror => 1.5,
            AshShield => 1.6,
            AshShards => 2.0,
            IceNeedles => 3.1,
            ThornMoths => 2.6,
            IceHalo => 3.4,
            Hex => 2.8,
            Eyes => 3.0,
            IceRosary => 4.0,
            Steps => 3.3,
            Motes | AshFlies => 3.5,
            AshTrail => 4.0,
            Bolt => 4.1,
            Runes => 4.3,
            _ => 1.0,
        }
    }

    fn stats_raw(self, level: u32, evolved: bool) -> WStat {
        use WeaponKind::*;
        if self.field() >= 3 {
            return self.twin().stats_raw(level, evolved);
        }
        if self.field() > 0 {
            // Built on the graveyard weapon in its place, with a pace of its own.
            let mut s = self.base().stats_raw(level, evolved);
            let l = level.saturating_sub(1) as f32;
            match self {
                Nails => s.speed = 820.0,
                Glass => s.speed = 430.0,
                HolyWater => s.speed = 400.0,
                // How fast the halo breathes and the censers swing.
                Halo => s.speed = 2.2 + 0.08 * l,
                Censer => s.speed = 2.4 + 0.1 * l,
                Gargoyle => s.speed = 500.0,
                Runes => s.speed = 420.0,
                // How long a candle burns.
                Candles => s.speed = 5.0 + 0.3 * l,
                Teeth => s.speed = 760.0,
                Sac => s.speed = 400.0,
                Motes => s.speed = 440.0,
                Tendrils => s.speed = 520.0,
                _ => {}
            }
            return s;
        }
        let l = level.saturating_sub(1) as f32;
        let at = |lvls: &[u32]| lvls.iter().filter(|&&x| level >= x).count() as u32;
        let mut s = match self {
            Blade => WStat { dmg: 14.0 + 5.0 * l, cd: 1.25 - 0.05 * l, amount: 1 + at(&[3, 6, 9]), area: 1.0 + 0.1 * at(&[2, 4, 7, 10]) as f32, speed: 0.0, pierce: 0 },
            Daggers => WStat { dmg: 7.0 + 2.5 * l, cd: 0.9 - 0.04 * l, amount: 1 + at(&[2, 4, 6, 8, 10]), area: 1.0, speed: 650.0, pierce: 1 + at(&[5, 9]) as i32 },
            Orb => WStat { dmg: 18.0 + 6.0 * l, cd: 1.8 - 0.08 * l, amount: 1 + at(&[4, 7, 10]), area: 1.0 + 0.15 * at(&[3, 5, 8]) as f32, speed: 470.0, pierce: 0 },
            Chime => WStat { dmg: 5.0 + 2.0 * l, cd: 0.55 - 0.02 * l, amount: 1, area: 1.0 + 0.1 * l, speed: 0.0, pierce: 0 },
            Lanterns => WStat { dmg: 10.0 + 3.0 * l, cd: 0.45, amount: 2 + at(&[2, 4, 6, 8, 10]), area: 1.0 + 0.04 * l, speed: 3.0 + 0.15 * l, pierce: 0 },
            Spear => WStat { dmg: 22.0 + 7.0 * l, cd: 2.0 - 0.09 * l, amount: 1 + at(&[3, 5, 7, 9]), area: 1.0 + 0.1 * at(&[4, 8, 10]) as f32, speed: 0.0, pierce: 0 },
            Axe => WStat { dmg: 20.0 + 6.0 * l, cd: 2.2 - 0.09 * l, amount: 1 + at(&[4, 7, 10]), area: 1.0 + 0.1 * at(&[3, 6, 9]) as f32, speed: 520.0, pierce: 999 },
            Bolt => WStat { dmg: 9.0 + 3.0 * l, cd: 1.3 - 0.05 * l, amount: 1 + at(&[2, 5, 7, 10]), area: 1.0, speed: 420.0, pierce: 0 },
            Scythe => WStat { dmg: 16.0 + 5.0 * l, cd: 2.0 - 0.08 * l, amount: 1 + at(&[5, 9]), area: 1.0 + 0.1 * at(&[3, 6, 8, 10]) as f32, speed: 0.0, pierce: 0 },
            Hex => WStat { dmg: 6.0 + 2.0 * l, cd: 0.35 - 0.01 * l, amount: 1, area: 1.0 + 0.1 * at(&[3, 6, 9]) as f32, speed: 2.5, pierce: 0 },
            Lance => WStat { dmg: 25.0 + 8.0 * l, cd: 1.6 - 0.07 * l, amount: 1 + at(&[4, 8]), area: 1.0 + 0.1 * at(&[5, 10]) as f32, speed: 900.0, pierce: 999 },
            Crossbow => WStat { dmg: 10.0 + 3.0 * l, cd: 1.2 - 0.05 * l, amount: 4 + 2 * at(&[3, 6, 9]), area: 1.0, speed: 560.0, pierce: 2 + at(&[5, 10]) as i32 },
            _ => unreachable!("the cathedral's weapons are built on the graveyard's"),
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
                _ => {}
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
        (Rosary, Gargoyle) => "Stone Litany",
        (Nails, Glass) => "Shattered Martyrdom",
        (HolyWater, Candles) => "Burning Font",
        (Halo, Censer) => "Incense Crown",
        (Judgement, Runes) => "Verdict of Heaven",
        (Crozier, Organ) => "Shepherd's Requiem",
        (Rosary, Crozier) => "Pilgrim's Scourge",
        (Nails, Runes) => "Written in Iron",
        (HolyWater, Judgement) => "Baptism of Fire",
        (Halo, Candles) => "Vigil Light",
        (Censer, Organ) => "Smoke and Psalm",
        (Gargoyle, Glass) => "Broken Spire",
        (Cleaver, Jaw) => "Butcher's Bite",
        (Teeth, Tendrils) => "The Gnawing Dark",
        (Sac, Steps) => "Red Tide",
        (Heart, Eyes) => "The Watching Heart",
        (Spikes, Motes) => "Crimson Thorns",
        (Ribcage, Spine) => "Marrow Harvest",
        (Cleaver, Ribcage) => "Slaughter Waltz",
        (Teeth, Motes) => "Swarm of Mouths",
        (Sac, Spikes) => "Bursting Thorns",
        (Heart, Steps) => "Bleeding Heart",
        (Eyes, Spine) => "The Seeing Spine",
        (Jaw, Tendrils) => "Hungry Grasp",
        (AshHatchet, AshBrand) => "Woodsman's Pyre",
        (AshShards, AshBurst) => "Cinder Gale",
        (AshSling, AshTrail) => "Wildfire",
        (AshBell, AshShield) => "Ember Chorus",
        (AshRoots, AshFlies) => "Burning Swarm",
        (AshSickle, AshPike) => "Harvest of Ash",
        (AshHatchet, AshSickle) => "Charred Waltz",
        (AshShards, AshFlies) => "Firefly Storm",
        (AshSling, AshRoots) => "Rooted Inferno",
        (AshBell, AshTrail) => "Smouldering Toll",
        (AshShield, AshPike) => "Pyre Pike",
        (AshBrand, AshBurst) => "Ring of Brands",
        (IceScourge, IceOwl) => "Scourge of Owls",
        (IceNeedles, IcePanes) => "Needle Glass",
        (IceFlask, IceBreath) => "Black Ice Font",
        (IceHalo, IceRosary) => "Crown of Beads",
        (IceFall, IceRunes) => "Icefall Runes",
        (IceCrook, IceLance) => "Glacier Crook",
        (IceScourge, IceCrook) => "Penance of Ice",
        (IceNeedles, IceRunes) => "Rune Hail",
        (IceFlask, IceFall) => "Avalanche Font",
        (IceHalo, IceBreath) => "Silent Halo",
        (IceRosary, IceLance) => "Winter Pike",
        (IceOwl, IcePanes) => "Shards of the Owl",
        (ThornWhip, ThornShears) => "Shears of the Rose",
        (ThornVolley, ThornRoots) => "Briar Web",
        (ThornBloom, ThornTrail) => "Bloom of Thorns",
        (ThornMirror, ThornMoths) => "Mirror of Moths",
        (ThornSnare, ThornSpores) => "Spore Briar",
        (ThornHook, ThornLash) => "Strangling Hook",
        (ThornWhip, ThornHook) => "Pruned Rose",
        (ThornVolley, ThornSpores) => "Pollen Volley",
        (ThornBloom, ThornSnare) => "Briar Bloom",
        (ThornMirror, ThornTrail) => "Nightshade Mirror",
        (ThornMoths, ThornLash) => "Vine of Eyes",
        (ThornShears, ThornRoots) => "Root Shears",
        _ => odd_name(a, b),
    }
}

/// Weapons that were never meant for each other, bound together anyway: a word from the one
/// and a word from the other, so every such pair has a name of its own.
fn odd_name(a: WeaponKind, b: WeaponKind) -> &'static str {
    const FIRST: [&str; ARSENAL] = ["Rusted", "Pale", "Kindled", "Tolling", "Wraithlit", "Stormborn", "Crescent", "Soulbound", "Grave", "Hexed", "Bloodied", "Gloomy"];
    const SECOND: [&str; ARSENAL] = ["Edge", "Thorns", "Embers", "Knell", "Lanterns", "Thunder", "Gyre", "Torrent", "Harvest", "Brand", "Pike", "Volley"];
    const HOLY_FIRST: [&str; ARSENAL] = ["Penitent", "Martyred", "Hallowed", "Haloed", "Smoking", "Judged", "Stony", "Written", "Shepherded", "Candlelit", "Thundering", "Painted"];
    const HOLY_SECOND: [&str; ARSENAL] = ["Litany", "Nails", "Font", "Corona", "Censer", "Verdict", "Gargoyle", "Psalm", "Crook", "Vigil", "Organ", "Window"];
    const FLESH_FIRST: [&str; ARSENAL] = ["Butchered", "Gnashing", "Swollen", "Beating", "Staring", "Impaled", "Snapping", "Hungry", "Caged", "Bleeding", "Lashing", "Writhing"];
    const FLESH_SECOND: [&str; ARSENAL] = ["Cleaver", "Teeth", "Sac", "Heart", "Eyes", "Spikes", "Jaw", "Motes", "Ribs", "Trail", "Spine", "Tendrils"];
    let (lo, hi) = if a.idx() < b.idx() { (a, b) } else { (b, a) };
    let (i, j) = (lo.idx() % ARSENAL, hi.idx() % ARSENAL);
    const ASH_FIRST: [&str; ARSENAL] = ["Charred", "Smoking", "Sparking", "Tolling", "Barked", "Rooted", "Hurled", "Drifting", "Blackened", "Smouldering", "Burning", "Scattered"];
    const ASH_SECOND: [&str; ARSENAL] = ["Hatchet", "Shards", "Sparks", "Bell", "Bark", "Roots", "Brand", "Flies", "Sickle", "Trail", "Pike", "Embers"];
    const ICE_FIRST: [&str; ARSENAL] = ["Frozen", "Needled", "Melted", "Rimed", "Beaded", "Falling", "Snowy", "Written", "Crooked", "Silent", "Glacial", "Frosted"];
    const ICE_SECOND: [&str; ARSENAL] = ["Scourge", "Needles", "Flask", "Halo", "Rosary", "Icicles", "Owl", "Runes", "Crook", "Breath", "Lance", "Panes"];
    const THORN_FIRST: [&str; ARSENAL] =
        ["Thorned", "Bramble", "Blooming", "Mirrored", "Watching", "Strangling", "Snipping", "Drifting", "Pruned", "Poisoned", "Lashing", "Creeping"];
    const THORN_SECOND: [&str; ARSENAL] = ["Whip", "Thorns", "Bloom", "Pond", "Moths", "Briar", "Shears", "Pollen", "Hook", "Nightshade", "Vine", "Roots"];
    let (first, second) = match lo.field() {
        1 => (&HOLY_FIRST, &HOLY_SECOND),
        2 => (&FLESH_FIRST, &FLESH_SECOND),
        3 => (&ASH_FIRST, &ASH_SECOND),
        4 => (&ICE_FIRST, &ICE_SECOND),
        5 => (&THORN_FIRST, &THORN_SECOND),
        _ => (&FIRST, &SECOND),
    };
    // Leaked once per pair and kept: at most 132 short names for the whole game.
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    type Names = Mutex<HashMap<(usize, usize, usize), &'static str>>;
    static NAMES: OnceLock<Names> = OnceLock::new();
    let mut names = NAMES.get_or_init(|| Mutex::new(HashMap::new())).lock().unwrap_or_else(|e| e.into_inner());
    names.entry((i, j, lo.field())).or_insert_with(|| Box::leak(format!("{} {}", first[i], second[j]).into_boxed_str()))
}

const MEGA_NAMES: [&str; ARSENAL] = [
    "Pyre of Kings",
    "Grave Tide",
    "Choir of Ash",
    "The Hollow Crown",
    "Eclipse Engine",
    "Black Cathedral",
    "Wake of Ruin",
    "The Long Night",
    "Ossuary Storm",
    "The Pale Host",
    "Ember Tomb",
    "Requiem Engine",
];
const ULTRA_NAMES: [&str; 4] = ["Dawnbreaker", "The Unending", "Ash Incarnate", "World Pyre"];

/// The same set of weapons always gets the same name.
fn names_for(kinds: &[WeaponKind], names: &[&'static str]) -> &'static str {
    let mut idx: Vec<usize> = kinds.iter().map(|k| k.idx()).collect();
    idx.sort_unstable();
    let h = idx.iter().fold(7usize, |h, i| h.wrapping_mul(31).wrapping_add(*i + 1));
    names[h % names.len()]
}

const MEGA_HOLY: [&str; ARSENAL] = [
    "The Great Nave",
    "Choir of Embers",
    "Reliquary of Wrath",
    "The Bleeding Saint",
    "Sanctum Engine",
    "The Iron Psalm",
    "Vault of Heaven",
    "The Long Mass",
    "Rose of Fire",
    "The Last Sermon",
    "Bells of Judgement",
    "The Gilded Ark",
];
const ULTRA_HOLY: [&str; 4] = ["Cathedral Ascendant", "The Eternal Rite", "Heaven's Forge", "The Burning Choir"];
const MEGA_FLESH: [&str; ARSENAL] = [
    "The Slaughterhouse",
    "Mouth of the Pit",
    "The Red Flood",
    "The Beating Choir",
    "Eye of the Storm",
    "Thicket of Bone",
    "The Feeding Frenzy",
    "Bloodswarm",
    "The Marrow Engine",
    "The Crimson Path",
    "The Spinal Choir",
    "The Writhing Host",
];
const ULTRA_FLESH: [&str; 4] = ["The Great Maw", "Flesh Ascendant", "The Red Communion", "Mother of Wounds"];
const MEGA_ASH: [&str; ARSENAL] = [
    "The Forest Pyre",
    "Storm of Cinders",
    "The Red Canopy",
    "Bells of the Burning",
    "The Oaken Wall",
    "Roots of Hell",
    "Wheel of Brands",
    "The Ember Host",
    "Harvest of the Fire",
    "The Smoking Road",
    "Pikes of the Pyre",
    "Crown of Sparks",
];
const ULTRA_ASH: [&str; 4] = ["The Great Burning", "Ash Over the World", "The Last Forest", "Heart of the Wildfire"];
const MEGA_ICE: [&str; ARSENAL] = [
    "The White Penance",
    "Storm of Needles",
    "The Frozen Font",
    "Halo of Winter",
    "The Endless Rosary",
    "The Great Thaw",
    "The Owl's Vigil",
    "The Frozen Psalm",
    "Crook of the Abbess",
    "The Long Silence",
    "The Glacier",
    "Cathedral of Ice",
];
const ULTRA_ICE: [&str; 4] = ["The Eternal Winter", "Frost Ascendant", "The Silent Abbey", "Heart of the Glacier"];
const MEGA_THORN: [&str; ARSENAL] = [
    "The Rose Labyrinth",
    "Storm of Thorns",
    "The Poison Garden",
    "The Black Pond",
    "Moths of the Starless",
    "The Briar Crown",
    "The Gardener's Hand",
    "The Pollen Host",
    "The Pruning",
    "The Nightshade Road",
    "The Strangling Wood",
    "Root of All",
];
const ULTRA_THORN: [&str; 4] = ["The Starless Bloom", "Garden Ascendant", "The Endless Hedge", "Heart of the Thorn"];

/// Named after the first of its weapons: the mega fusions one hero can hold at once share no
/// weapon, so they never share a name.
pub fn mega_name(kinds: &[WeaponKind]) -> &'static str {
    let first = kinds.iter().map(|k| k.idx() % ARSENAL).min().unwrap_or(0);
    match kinds.first().map(|k| k.field()) {
        Some(3) => return MEGA_ASH[first],
        Some(4) => return MEGA_ICE[first],
        Some(5) => return MEGA_THORN[first],
        _ => {}
    }
    if kinds.first().is_some_and(|k| k.field() == 2) {
        MEGA_FLESH[first]
    } else if kinds.first().is_some_and(|k| k.is_cathedral()) {
        MEGA_HOLY[first]
    } else {
        MEGA_NAMES[first]
    }
}

pub fn ultra_name(kinds: &[WeaponKind]) -> &'static str {
    let list = match kinds.first().map(|k| k.field()) {
        Some(1) => &ULTRA_HOLY,
        Some(2) => &ULTRA_FLESH,
        Some(3) => &ULTRA_ASH,
        Some(4) => &ULTRA_ICE,
        Some(5) => &ULTRA_THORN,
        _ => &ULTRA_NAMES,
    };
    names_for(kinds, list)
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
            Shell => "+1.5 armor",
            Feather => "+12% luck and crit chance",
            Moss => "+0.4 health regen per second",
            Lung => "+20 stamina, +15% stamina regen",
            LeechFang => "+0.5 health per kill",
            CrackedCrown => "+10% experience",
            GraveCandle => "+15% damage to elites and bosses",
        }
    }

    /// What the number means in play, for the passives whose short line is not obvious.
    pub fn detail(self) -> Option<&'static str> {
        use ItemKind::*;
        match self {
            EmberHeart => Some("Might: every weapon deals more damage."),
            Hourglass => Some("Every weapon attacks more often."),
            Lens => Some("Swings, blasts, auras and sigils grow larger."),
            Sigil => Some("Every weapon fires one more blade, bolt or orb."),
            Censer => Some("Souls and experience are drawn in from further away."),
            Shell => Some("Every hit you take is lessened by your armor."),
            Feather => Some("Luck brings better drops; crits deal double damage."),
            Lung => Some("Stamina is spent on rolls and refills over time."),
            CrackedCrown => Some("You level up faster."),
            GraveCandle => Some("Elite foes and lords take the extra damage."),
            Boots | Moss | LeechFang => None,
        }
    }

    pub fn max_level(self) -> u32 {
        match self {
            ItemKind::Sigil => 2,
            _ => 8,
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
    LanternWarden,
    Stormcaller,
}

pub const ALL_CLASSES: [Class; 11] = [
    Class::Wanderer,
    Class::GraveHunter,
    Class::CinderAdept,
    Class::BellAcolyte,
    Class::Reaper,
    Class::Hexblade,
    Class::BloodKnight,
    Class::Arbalist,
    Class::BucketKnight,
    Class::LanternWarden,
    Class::Stormcaller,
];

/// Classes given for what was offered in the Reliquary: souls (true) or Ashen Marks, and
/// how many.
pub const OFFERED_CLASSES: [(Class, bool, u64); 2] = [(Class::LanternWarden, true, 25_000), (Class::Stormcaller, false, 250)];

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
            Class::LanternWarden => "Lantern Warden",
            Class::Stormcaller => "Stormcaller",
        }
    }

    pub fn desc(self) -> &'static str {
        match self {
            Class::Wanderer => "Rusted Blade. +25 max health, +8% might.",
            Class::GraveHunter => "Pale Daggers. +10% move speed, +10 max health.",
            Class::CinderAdept => "Kindled Orb. +10% area.",
            Class::BellAcolyte => "Chime of Warding. +0.8 health regen, +10 max health.",
            Class::Reaper => "Grave Scythe. +0.5 health per kill, up to 4 per second, +15 max health.",
            Class::Hexblade => "Hex Brand. +25 stamina, +1 armor.",
            Class::BloodKnight => "Blood Lance. +20 max health.",
            Class::Arbalist => "Gloom Crossbow. -10% weapon cooldown, +10 max health.",
            Class::BucketKnight => "Crescent Axe. +2 armor. Cheap rolls. Clang.",
            Class::LanternWarden => "Wraith Lanterns. One more lantern, a wider light, braziers burn twice as long, +1 armor, +10 max health.",
            Class::Stormcaller => "Storm Spear. +10% crit; a crit leaps on to a second foe.",
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
            Class::LanternWarden => WeaponKind::Lanterns,
            Class::Stormcaller => WeaponKind::Spear,
        }
    }
}

impl Class {
    /// The class's active skill: name, what it does, and seconds between uses.
    pub fn skill(self) -> (&'static str, &'static str, f32) {
        match self {
            Class::Wanderer => ("Rally", "A great shockwave hurls foes back and wipes out their shots; you are untouchable for two seconds.", 20.0),
            Class::GraveHunter => ("Fan of Knives", "Thirty-six piercing daggers in every direction.", 15.0),
            Class::CinderAdept => ("Meteor", "Three falling stars strike the nearest foes and burn all around them.", 22.0),
            Class::BellAcolyte => ("Sanctuary", "Heals every hero nearby by more than half and wards them.", 25.0),
            Class::Reaper => ("Harvest", "One vast sweep of the scythe; every soul reaped mends you.", 17.0),
            Class::Hexblade => ("Hex Field", "A great sigil burns into the ground around you for eight seconds.", 20.0),
            Class::BloodKnight => ("Blood Rite", "Fury for nine seconds and two fifths of your health back.", 28.0),
            Class::Arbalist => ("Bolt Storm", "Forty-eight quarrels fly out in every direction.", 17.0),
            Class::BucketKnight => ("CLANG", "A deafening clang hurts and stuns every foe around you.", 21.0),
            Class::LanternWarden => ("Beacon", "A blinding light: foes around you are slowed and burnt, heroes nearby healed.", 22.0),
            Class::Stormcaller => ("Tempest", "Eight bolts fall on the strongest foes in reach.", 20.0),
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
    MetaDef { key: "vigor", name: "Vigor", desc: "+8% max health", max: 8, base_cost: 300 },
    MetaDef { key: "endurance", name: "Endurance", desc: "+15 stamina, +10% regen", max: 5, base_cost: 300 },
    MetaDef { key: "strength", name: "Strength", desc: "+5% might", max: 8, base_cost: 350 },
    MetaDef { key: "swiftness", name: "Swiftness", desc: "+4% move speed", max: 5, base_cost: 360 },
    MetaDef { key: "haste", name: "Haste", desc: "-4% weapon cooldown", max: 5, base_cost: 500 },
    MetaDef { key: "fortune", name: "Fortune", desc: "+10% luck", max: 5, base_cost: 320 },
    MetaDef { key: "greed", name: "Greed", desc: "+12% souls gained", max: 6, base_cost: 900 },
    MetaDef { key: "ember", name: "Ember", desc: "+1 revival per vigil", max: 3, base_cost: 2000 },
    MetaDef { key: "insight", name: "Insight", desc: "+1 reroll per vigil", max: 4, base_cost: 600 },
    MetaDef { key: "toughness", name: "Toughness", desc: "+1 armor", max: 5, base_cost: 800 },
    MetaDef { key: "renewal", name: "Renewal", desc: "+0.15 health per second", max: 5, base_cost: 600 },
    MetaDef { key: "reach", name: "Reach", desc: "+6% weapon area", max: 5, base_cost: 700 },
    MetaDef { key: "precision", name: "Precision", desc: "+2% critical chance", max: 5, base_cost: 600 },
    MetaDef { key: "magnetism", name: "Magnetism", desc: "+12% pickup range", max: 5, base_cost: 400 },
    MetaDef { key: "scholar", name: "Scholar", desc: "+5% experience", max: 5, base_cost: 700 },
    MetaDef { key: "slayer", name: "Slayer", desc: "+6% damage to elites and bosses", max: 5, base_cost: 650 },
    MetaDef { key: "volley", name: "Volley", desc: "+1 projectile for every weapon", max: 2, base_cost: 40_000 },
    MetaDef { key: "colossus", name: "Colossus", desc: "+18 max health", max: 6, base_cost: 1200 },
    MetaDef { key: "warding", name: "Warding", desc: "+0.08 s untouchable after a hit", max: 4, base_cost: 800 },
    MetaDef { key: "bloodthirst", name: "Bloodthirst", desc: "+0.15 health per kill", max: 4, base_cost: 1500 },
    MetaDef { key: "fleetfoot", name: "Fleetfoot", desc: "Rolls cost 3 less stamina", max: 4, base_cost: 1000 },
    MetaDef { key: "barter", name: "Barter", desc: "Merchant wares cost 8% less", max: 4, base_cost: 1200 },
    MetaDef { key: "foresight", name: "Foresight", desc: "+8% chance of a fourth card", max: 5, base_cost: 1400 },
    MetaDef { key: "second_wind", name: "Second Wind", desc: "Rise again with +10% health", max: 4, base_cost: 1200 },
    MetaDef { key: "kindling", name: "Kindling", desc: "Start with your weapon one level higher", max: 4, base_cost: 2500 },
    MetaDef { key: "whetstone", name: "Whetstone", desc: "+5% damage for max-level weapons", max: 5, base_cost: 2200 },
    MetaDef { key: "fusion_lore", name: "Fusion Lore", desc: "+4% damage for fused weapons", max: 5, base_cost: 3500 },
    MetaDef { key: "evasion", name: "Evasion", desc: "2% chance to avoid a hit", max: 5, base_cost: 3000 },
    MetaDef { key: "bulwark", name: "Bulwark", desc: "-2% damage taken", max: 5, base_cost: 2500 },
    MetaDef { key: "alchemy", name: "Alchemy", desc: "Draughts heal 25% more", max: 4, base_cost: 1000 },
    MetaDef { key: "tithe", name: "Tithe", desc: "Bosses drop 20% more souls", max: 5, base_cost: 2000 },
    MetaDef { key: "arsenal", name: "Arsenal", desc: "+1 weapon slot", max: 2, base_cost: 90_000 },
    MetaDef { key: "satchel", name: "Satchel", desc: "+1 item slot", max: 2, base_cost: 70_000 },
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

/// Weapons that once had to be bought, and what they cost: every weapon is free now, and
/// whoever bought one gets the marks back once.
pub const FREED_WEAPONS: [(WeaponKind, u64); 4] = [(WeaponKind::Scythe, 22), (WeaponKind::Hex, 22), (WeaponKind::Lance, 30), (WeaponKind::Crossbow, 30)];

pub const UNLOCKS: [(Unlock, u64); 7] = [
    (Unlock::Class(Class::Reaper), 45),
    (Unlock::Class(Class::Hexblade), 45),
    (Unlock::Class(Class::BloodKnight), 60),
    (Unlock::Class(Class::Arbalist), 60),
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
    // Every weapon is there from the start (the cathedral's never needed unlocking).
    for w in &GRAVE_WEAPONS {
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
    // Kills count by their root: a night of fifty thousand pays for them like one of a
    // hundred and twelve hundreds, not three hundred times one of a hundred and fifty.
    let base = minutes * 1.5 + bosses as f32 * 6.0 + level as f32 * 0.4 + (kills as f32).sqrt() * 0.5 + if won { 25.0 } else { 0.0 };
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
    AlmsHood,
    PilgrimHat,
    Mitre,
    MarkVeil,
    Thorns,
    CharityLamp,
    AlmonerCrown,
    WingedCrown,
    Wimple,
    SixWings,
    Butcher,
    FlayedHalo,
    HeartCrown,
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
    /// Not bought: worn once this much has been offered in the Reliquary, souls (true) or
    /// Ashen Marks (false).
    pub offering: Option<(bool, u64)>,
}

/// Skin 0 is the class look; these are bought with souls in the Wardrobe.
pub const SKINS: [SkinDef; 31] = [
    SkinDef {
        name: "Ashen Knight",
        desc: "Grey as the dust of old fires.",
        cost: 2_000,
        body: (96, 92, 90),
        metal: (150, 150, 156),
        helm: Helm::Class(Class::Wanderer),
        offering: None,
    },
    SkinDef {
        name: "Tin Pail",
        desc: "A bucket for any class. Clang.",
        cost: 3_000,
        body: (90, 70, 50),
        metal: (140, 140, 150),
        helm: Helm::Class(Class::BucketKnight),
        offering: None,
    },
    SkinDef {
        name: "Crimson Paladin",
        desc: "Red cloth and horned steel.",
        cost: 5_000,
        body: (150, 24, 30),
        metal: (190, 170, 120),
        helm: Helm::Class(Class::BloodKnight),
        offering: None,
    },
    SkinDef { name: "Berry Knight", desc: "Sweet, red and slightly seedy.", cost: 6_000, body: (210, 60, 80), metal: (120, 190, 90), helm: Helm::Strawberry, offering: None },
    SkinDef { name: "Plague Doctor", desc: "A beaked mask against the miasma.", cost: 8_000, body: (34, 30, 32), metal: (90, 70, 50), helm: Helm::PlagueMask, offering: None },
    SkinDef { name: "Pumpkin Lord", desc: "A grin that glows all night.", cost: 10_000, body: (200, 100, 30), metal: (70, 110, 50), helm: Helm::Pumpkin, offering: None },
    SkinDef { name: "Antlered Druid", desc: "The forest walks with you.", cost: 12_000, body: (60, 100, 54), metal: (120, 96, 70), helm: Helm::Antlers, offering: None },
    SkinDef { name: "Frost Warden", desc: "Pale blue, with a crown of ice.", cost: 15_000, body: (110, 160, 200), metal: (200, 225, 240), helm: Helm::IceCrown, offering: None },
    SkinDef {
        name: "Bone Monarch",
        desc: "A skull wearing a crown, or the other way round.",
        cost: 15_000,
        body: (190, 180, 150),
        metal: (220, 210, 180),
        helm: Helm::SkullCrown,
        offering: None,
    },
    SkinDef { name: "Moth Priest", desc: "Drawn to every flame on the field.", cost: 18_000, body: (150, 130, 160), metal: (190, 170, 150), helm: Helm::Moth, offering: None },
    SkinDef { name: "Hollow Jester", desc: "The joke is on the dark.", cost: 20_000, body: (110, 40, 140), metal: (60, 150, 90), helm: Helm::Jester, offering: None },
    SkinDef {
        name: "Lantern Bearer",
        desc: "A little light that never goes out.",
        cost: 22_000,
        body: (40, 110, 110),
        metal: (150, 150, 140),
        helm: Helm::LanternHood,
        offering: None,
    },
    SkinDef { name: "Gilded Saint", desc: "Gold leaf over old iron, and a halo.", cost: 25_000, body: (200, 160, 70), metal: (240, 210, 120), helm: Helm::Halo, offering: None },
    SkinDef { name: "Ember Wraith", desc: "A head of living fire.", cost: 30_000, body: (40, 24, 22), metal: (90, 60, 50), helm: Helm::Flame, offering: None },
    SkinDef { name: "Iron Duke", desc: "A great helm and a proud plume.", cost: 35_000, body: (70, 74, 90), metal: (175, 178, 190), helm: Helm::Greathelm, offering: None },
    SkinDef { name: "Grave Cat", desc: "Ears up, tail high, nine lives.", cost: 40_000, body: (26, 24, 30), metal: (60, 56, 66), helm: Helm::CatHood, offering: None },
    SkinDef {
        name: "Starlit Wanderer",
        desc: "The night sky, stitched into a hat.",
        cost: 50_000,
        body: (30, 40, 100),
        metal: (150, 160, 210),
        helm: Helm::StarHat,
        offering: None,
    },
    SkinDef {
        name: "The Last Flame",
        desc: "White fire and a burning crown.",
        cost: 250_000,
        body: (235, 225, 205),
        metal: (255, 220, 140),
        helm: Helm::LastCrown,
        offering: None,
    },
    // Given, not bought: these come with what has been offered in the Reliquary.
    SkinDef {
        name: "Almsgiver",
        desc: "A plain hood and a coin given freely.",
        cost: 0,
        body: (120, 104, 84),
        metal: (200, 170, 90),
        helm: Helm::AlmsHood,
        offering: Some((true, 10_000)),
    },
    SkinDef {
        name: "Pilgrim of Ash",
        desc: "A wide hat and a shell from a far shore.",
        cost: 0,
        body: (96, 78, 60),
        metal: (170, 150, 120),
        helm: Helm::PilgrimHat,
        offering: Some((true, 100_000)),
    },
    SkinDef {
        name: "Saint of Ash",
        desc: "A white mitre for one who gave a fortune away.",
        cost: 0,
        body: (225, 215, 195),
        metal: (230, 190, 90),
        helm: Helm::Mitre,
        offering: Some((true, 1_000_000)),
    },
    SkinDef {
        name: "The Marked",
        desc: "A veil, and behind it a mark that glows.",
        cost: 0,
        body: (60, 30, 40),
        metal: (200, 120, 70),
        helm: Helm::MarkVeil,
        offering: Some((false, 100)),
    },
    SkinDef {
        name: "The Selfless",
        desc: "A crown of thorns under a quiet light.",
        cost: 0,
        body: (70, 60, 56),
        metal: (150, 130, 100),
        helm: Helm::Thorns,
        offering: Some((false, 1_000)),
    },
    SkinDef {
        name: "Lamp of Charity",
        desc: "A little lamp carried for others.",
        cost: 0,
        body: (70, 90, 110),
        metal: (190, 170, 120),
        helm: Helm::CharityLamp,
        offering: Some((true, 50_000)),
    },
    SkinDef {
        name: "Gilded Almoner",
        desc: "A circlet hung with coins, all of them given.",
        cost: 0,
        body: (130, 40, 50),
        metal: (235, 200, 100),
        helm: Helm::AlmonerCrown,
        offering: Some((true, 500_000)),
    },
    SkinDef {
        name: "The Unburdened",
        desc: "Nothing kept, and wings for it.",
        cost: 0,
        body: (235, 235, 240),
        metal: (210, 220, 235),
        helm: Helm::WingedCrown,
        offering: Some((true, 5_000_000)),
    },
    SkinDef {
        name: "Ember Sister",
        desc: "A wimple, and an ember kept in prayer.",
        cost: 0,
        body: (40, 36, 44),
        metal: (180, 120, 80),
        helm: Helm::Wimple,
        offering: Some((false, 300)),
    },
    SkinDef {
        name: "Ashen Seraph",
        desc: "Six wings of light, folded about the head.",
        cost: 0,
        body: (240, 220, 190),
        metal: (255, 210, 130),
        helm: Helm::SixWings,
        offering: Some((false, 5_000)),
    },
    // From the flesh pits: one to buy, two given for offerings.
    SkinDef {
        name: "Pit Butcher",
        desc: "A stitched leather hood and an apron that has seen things.",
        cost: 60_000,
        body: (110, 40, 38),
        metal: (150, 120, 100),
        helm: Helm::Butcher,
        offering: None,
    },
    SkinDef {
        name: "Flayed Saint",
        desc: "No skin left to give, and a halo of bone for it.",
        cost: 0,
        body: (150, 50, 60),
        metal: (226, 212, 186),
        helm: Helm::FlayedHalo,
        offering: Some((true, 2_000_000)),
    },
    SkinDef { name: "Heart Regent", desc: "A crown that beats.", cost: 0, body: (90, 10, 24), metal: (210, 170, 80), helm: Helm::HeartCrown, offering: Some((false, 2_500)) },
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
    /// Empowered foes on top of all that, in steps of `BOOST_STEP`: more health (lords too),
    /// more speed, more of them. Each step pays more souls and marks.
    pub boost: [u8; 3],
}

/// The empowerments: their names, the percent each small step adds, how far the small steps
/// go before each step adds `BOOST_BIG`, the most steps (up to +`BOOST_TOP`%), and how much
/// more each full +100% pays.
pub const BOOST_NAMES: [&str; 3] = ["Foes' health", "Foes' speed", "Foes' numbers"];
pub const BOOST_STEP: [f32; 3] = [25.0, 10.0, 25.0];
const BOOST_FINE: [f32; 3] = [200.0, 100.0, 200.0];
const BOOST_BIG: f32 = 50.0;
const BOOST_TOP: f32 = 1000.0;
pub const BOOST_MAX: [u8; 3] = [
    (BOOST_FINE[0] / BOOST_STEP[0] + (BOOST_TOP - BOOST_FINE[0]) / BOOST_BIG) as u8,
    (BOOST_FINE[1] / BOOST_STEP[1] + (BOOST_TOP - BOOST_FINE[1]) / BOOST_BIG) as u8,
    (BOOST_FINE[2] / BOOST_STEP[2] + (BOOST_TOP - BOOST_FINE[2]) / BOOST_BIG) as u8,
];
const BOOST_PAY: [f32; 3] = [0.35, 1.0, 0.45];

/// The percent `steps` of empowerment `i` add: small steps first, then bigger ones.
pub fn boost_pct(i: usize, steps: u8) -> f32 {
    let fine = BOOST_FINE[i] / BOOST_STEP[i];
    let s = steps.min(BOOST_MAX[i]) as f32;
    if s <= fine {
        s * BOOST_STEP[i]
    } else {
        BOOST_FINE[i] + (s - fine) * BOOST_BIG
    }
}

/// Seconds over which a harder difficulty ramps up to its full strength.
const DIFF_RAMP: f32 = 360.0;

impl Difficulty {
    /// The difficulty in effect at `time`: harder settings start at a third of their extra
    /// strength and reach all of it after six minutes, so the first lord can be learned.
    pub fn at(self, time: f32) -> Difficulty {
        let k = 0.33 + 0.67 * (time / DIFF_RAMP).clamp(0.0, 1.0);
        let ramp = |v: f32| if v > 1.0 { 1.0 + (v - 1.0) * k } else { v };
        let b = |i: usize| self.boost_mult(i);
        Difficulty {
            count: ramp(self.count) * b(2),
            hp: ramp(self.hp) * b(0),
            dmg: ramp(self.dmg),
            speed: ramp(self.speed) * b(1),
            boss_hp: self.boss_hp * b(0),
            boost: self.boost,
        }
    }

    /// How much empowerment `i` multiplies its value by.
    pub fn boost_mult(&self, i: usize) -> f32 {
        1.0 + boost_pct(i, self.boost[i]) / 100.0
    }

    /// What the empowerments pay on top: more souls and marks the stronger the foes.
    pub fn boost_reward(&self) -> f32 {
        1.0 + (0..3).map(|i| (self.boost_mult(i) - 1.0) * BOOST_PAY[i]).sum::<f32>()
    }

    /// The empowerments in words, or nothing when there are none.
    pub fn boost_text(&self) -> String {
        let parts: Vec<String> = (0..3).filter(|&i| self.boost[i] > 0).map(|i| format!("{} +{:.0}%", ["health", "speed", "numbers"][i], boost_pct(i, self.boost[i]))).collect();
        parts.join(", ")
    }

    /// The same settings without empowerments, to compare with the presets.
    pub fn base(&self) -> Difficulty {
        Difficulty { boost: [0; 3], ..*self }
    }
}

pub const DIFF_COUNT: [f32; 9] = [0.5, 0.75, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0];
pub const DIFF_HP: [f32; 8] = [0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 4.0, 6.0];
pub const DIFF_DMG: [f32; 9] = [0.5, 0.75, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0];
pub const DIFF_SPEED: [f32; 7] = [0.8, 0.9, 1.0, 1.15, 1.3, 1.5, 1.7];
pub const DIFF_BOSS: [f32; 8] = [0.5, 0.75, 1.0, 1.5, 2.0, 3.0, 4.0, 6.0];

pub const PRESETS: [(&str, Difficulty); 6] = [
    ("Pilgrim", Difficulty { count: 0.75, hp: 0.75, dmg: 0.5, speed: 0.9, boss_hp: 0.75, boost: [0; 3] }),
    ("Knight", Difficulty { count: 1.0, hp: 1.0, dmg: 1.0, speed: 1.0, boss_hp: 1.0, boost: [0; 3] }),
    ("Lord", Difficulty { count: 1.5, hp: 1.5, dmg: 1.5, speed: 1.15, boss_hp: 1.5, boost: [0; 3] }),
    ("Abyss", Difficulty { count: 2.5, hp: 3.0, dmg: 2.5, speed: 1.3, boss_hp: 3.0, boost: [0; 3] }),
    ("Eclipse", Difficulty { count: 3.0, hp: 4.0, dmg: 3.0, speed: 1.5, boss_hp: 4.0, boost: [0; 3] }),
    ("Oblivion", Difficulty { count: 4.0, hp: 6.0, dmg: 4.0, speed: 1.7, boss_hp: 6.0, boost: [0; 3] }),
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
        // Past Knight the extra pays at three fifths: more foes already means more souls.
        let m = if m > 1.0 { 1.0 + (m - 1.0) * 0.6 } else { m };
        m.clamp(0.4, 4.0) * self.boost_reward()
    }

    pub fn preset_name(&self) -> &'static str {
        PRESETS.iter().find(|p| p.1 == self.base()).map(|p| p.0).unwrap_or("Custom")
    }
}

/// Moves `cur` one step through `list` in direction `dir`.
pub fn step(list: &[f32], cur: f32, dir: i32) -> f32 {
    let i = list.iter().position(|v| (*v - cur).abs() < 0.001).unwrap_or(list.len() / 2) as i32;
    list[(i + dir).clamp(0, list.len() as i32 - 1) as usize]
}

// ---------------------------------------------------------------- challenges

/// Optional handicaps for a single-player vigil; each adds Ashen Marks (see `reward`).
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

impl Challenge {
    /// The share of Ashen Marks it adds: more for the two that cost a hero most.
    pub fn reward(self) -> f32 {
        match self {
            Challenge::NoRolls | Challenge::OneWeapon => 0.35,
            Challenge::GlassCannon => 0.25,
        }
    }
}

pub fn challenge_mult(bits: u8) -> f32 {
    1.0 + ALL_CHALLENGES.iter().filter(|c| bits & c.bit() != 0).map(|c| c.reward()).sum::<f32>()
}

// ---------------------------------------------------------------- curses

/// Curses laid on a whole vigil, for everyone in it; each adds Ashen Marks (see `reward`).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Curse {
    Frenzy,
    Famine,
    Volatile,
    EliteTide,
    DarkHour,
    Brittle,
}

pub const ALL_CURSES: [Curse; 6] = [Curse::Frenzy, Curse::Famine, Curse::Volatile, Curse::EliteTide, Curse::DarkHour, Curse::Brittle];

impl Curse {
    pub fn bit(self) -> u8 {
        1 << self as u8
    }

    pub fn name(self) -> &'static str {
        match self {
            Curse::Frenzy => "Frenzy",
            Curse::Famine => "Famine",
            Curse::Volatile => "Volatile",
            Curse::EliteTide => "Elite Tide",
            Curse::DarkHour => "The Dark Hour",
            Curse::Brittle => "Brittle",
        }
    }

    pub fn desc(self) -> &'static str {
        match self {
            Curse::Frenzy => "Foes move 25% faster.",
            Curse::Famine => "No health regeneration; draughts and bread heal half.",
            Curse::Volatile => "Slain foes sometimes burst into bullets.",
            Curse::EliteTide => "Elites come three times as often.",
            Curse::DarkHour => "Your light reaches half as far; foes are 15% tougher.",
            Curse::Brittle => "No embers, and raising a companion takes twice as long.",
        }
    }
}

impl Curse {
    /// The share of Ashen Marks it adds, by what it costs: the bullets and the dark most, an
    /// elite tide (which also feeds the hero) and a curse that only bites when one falls least.
    pub fn reward(self) -> f32 {
        match self {
            Curse::Volatile | Curse::DarkHour => 0.3,
            Curse::Frenzy => 0.2,
            Curse::Famine => 0.15,
            Curse::EliteTide | Curse::Brittle => 0.1,
        }
    }
}

pub fn curse_mult(bits: u8) -> f32 {
    1.0 + ALL_CURSES.iter().filter(|c| bits & c.bit() != 0).map(|c| c.reward()).sum::<f32>()
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
    Accursed,
    Lamplighter,
    Alms,
    Benefactor,
    PatronOfAsh,
    MarkGiver,
    Selfless,
    Bellringer,
    Heartseeker,
    Encore,
    Greenkeeper,
    Thaw,
    Unwatched,
}

pub const ALL_ACHIEVEMENTS: [Achievement; 80] = [
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
    Achievement::Accursed,
    Achievement::Lamplighter,
    Achievement::Alms,
    Achievement::Benefactor,
    Achievement::PatronOfAsh,
    Achievement::MarkGiver,
    Achievement::Selfless,
    Achievement::Bellringer,
    Achievement::Heartseeker,
    Achievement::Encore,
    Achievement::Greenkeeper,
    Achievement::Thaw,
    Achievement::Unwatched,
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
            Accursed => "Accursed",
            Lamplighter => "Lamplighter",
            Alms => "Alms",
            Benefactor => "Benefactor",
            PatronOfAsh => "Patron of Ash",
            MarkGiver => "Giver of Marks",
            Selfless => "Selfless",
            Bellringer => "The Last Toll",
            Heartseeker => "The Heart of It",
            Greenkeeper => "Greenkeeper",
            Thaw => "The Thaw",
            Unwatched => "Looked At",
            Encore => "Encore",
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
            Evolved => "Bring a weapon to its highest level.",
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
            Accursed => "Keep a vigil until dawn under three curses.",
            Lamplighter => "Have ten braziers burning at once.",
            Alms => "Offer 10,000 souls in the Reliquary.",
            Benefactor => "Offer 100,000 souls in the Reliquary.",
            PatronOfAsh => "Offer 1,000,000 souls in the Reliquary.",
            MarkGiver => "Offer 100 Ashen Marks in the Reliquary.",
            Selfless => "Offer 1,000 Ashen Marks in the Reliquary.",
            Bellringer => "Find the bell that fell from the cathedral.",
            Heartseeker => "Find the heart that beats beneath the flesh pits.",
            Greenkeeper => "Find the last green thing in the Ashwood.",
            Thaw => "Free the abbot frozen in the abbey's ice.",
            Unwatched => "Find the one statue in the garden that never moves.",
            Encore => "Ask the main menu for another song.",
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
            LastKeeper | Clang | Bellringer | Heartseeker | Greenkeeper | Thaw | Unwatched => 15,
            Untouched | Hoarder | FullHouse | RollModel | Banker => 20,
            GrandFusion | Executioner => 25,
            Centurion => 30,
            LongNight | AshAndBone => 40,
            Singularity => 60,
            Gauntlet => 50,
            Accursed => 50,
            Lamplighter => 20,
            Alms | MarkGiver => 10,
            Benefactor => 25,
            PatronOfAsh | Selfless => 60,
            ThinTheHerd | Polished => 5,
            FullArsenal | Packrat | GentleDawn | Purrfect | Dressed | Dizzy | Impatient | Partygoer | Encore => 10,
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
