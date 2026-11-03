//! The running vigil.
//!
//! The host (or the single-player game) simulates everything here. A network client keeps
//! a `World` too, but only fills it from snapshots and predicts its own hero's movement.

mod boss;
mod combat;
mod events;
mod fx;
mod hero;
mod pickups;
mod spawn;
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
/// Sounds are at full volume this close to the listener...
pub const HEAR_FULL: f32 = 450.0;
/// ...and silent from here on, about a screen away.
pub const HEAR_MAX: f32 = 1100.0;
pub const BOSS_INTERVAL: f32 = 5.0 * 60.0;
pub const RESUME_DELAY: f32 = 2.5;
pub const MAX_HEROES: usize = 4;
pub const REVIVE_TIME: f32 = 3.0;
const MAX_DECALS: usize = 900;
/// The field is a square from -WORLD_HALF to WORLD_HALF, walled in, with the abyss beyond.
pub const WORLD_HALF: f32 = 3600.0;

pub fn inside_world(p: Vec2, margin: f32) -> bool {
    p.x.abs() < WORLD_HALF - margin && p.y.abs() < WORLD_HALF - margin
}

pub fn clamp_world(p: Vec2, margin: f32) -> Vec2 {
    let m = WORLD_HALF - margin;
    vec2(p.x.clamp(-m, m), p.y.clamp(-m, m))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BossKind {
    Gravewarden,
    Matriarch,
    Hermit,
    Sovereign,
}

impl BossKind {
    pub fn name(self) -> &'static str {
        match self {
            BossKind::Gravewarden => "The Gravewarden",
            BossKind::Matriarch => "Pale Matriarch",
            BossKind::Hermit => "The Tolling Hermit",
            BossKind::Sovereign => "Ashen Sovereign",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            BossKind::Gravewarden => "Keeper of the Sunken Graves",
            BossKind::Matriarch => "Mother of the Gloom Moths",
            BossKind::Hermit => "He Who Rings the Last Hour",
            BossKind::Sovereign => "Last Lord of the Cinder Throne",
        }
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
            EnemyKind::Boss(b) => 7 + b as u8,
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
            7 => EnemyKind::Boss(BossKind::Gravewarden),
            8 => EnemyKind::Boss(BossKind::Matriarch),
            9 => EnemyKind::Boss(BossKind::Hermit),
            _ => EnemyKind::Boss(BossKind::Sovereign),
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
            EnemyKind::Boss(BossKind::Gravewarden) => 44.0,
            EnemyKind::Boss(BossKind::Matriarch) => 38.0,
            EnemyKind::Boss(BossKind::Hermit) => 34.0,
            EnemyKind::Boss(BossKind::Sovereign) => 52.0,
        };
        if elite {
            r * 1.6
        } else {
            r
        }
    }

    /// Only these fade in and out, so only they send an alpha.
    pub fn has_alpha(self) -> bool {
        matches!(self, EnemyKind::Wraith | EnemyKind::Boss(BossKind::Hermit))
    }

    /// What comes out when it is cut.
    pub fn blood(self) -> Color {
        match self {
            EnemyKind::Moth => Color::from_rgba(150, 140, 110, 255),
            EnemyKind::Wraith => Color::from_rgba(70, 110, 190, 255),
            EnemyKind::Bloat => Color::from_rgba(90, 110, 30, 255),
            EnemyKind::Archer => Color::from_rgba(110, 100, 90, 255),
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
    pub last_hit: [f32; 12],
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
}

#[derive(Clone, Copy, Debug)]
pub enum FxKind {
    Slash { ang: f32, radius: f32, evo: bool, owner: u8 },
    Sweep { radius: f32, evo: bool, owner: u8 },
    Explosion { r: f32, color: Color },
    Lightning { from: Vec2 },
    Ring { r: f32, color: Color },
    Ghost,
}

pub struct Fx {
    pub pos: Vec2,
    pub life: f32,
    pub max_life: f32,
    pub kind: FxKind,
}

/// Burning ground from orbs and hex brands.
pub struct FirePool {
    pub pos: Vec2,
    pub r: f32,
    pub life: f32,
    pub max_life: f32,
    pub dps: f32,
    pub tick: f32,
    pub src: Src,
    pub hex: bool,
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
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Pause {
    Running,
    Choosing,
    Countdown(f32),
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
    pub remnant: Option<Remnant>,
    pub cat: Option<Cat>,
    /// Seconds of Blood Moon left; 0 when the sky is normal.
    pub blood_moon: f32,
    pub moons_survived: u32,
    pub cursed: Option<CursedShrine>,
    pub trials_won: u32,
    pub merchant: Option<Merchant>,
    pub boss: Option<BossAi>,
    pub bosses_spawned: u32,
    pub bosses_killed: u32,
    pub arena: Option<(Vec2, f32)>,
    pub next_boss: f32,
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
    pub toasts: Vec<(String, f32)>,
    pub(crate) grid: Grid,
    qbuf: Vec<usize>,
    pub show_numbers: bool,
    pub gore: bool,
    /// Draw the full models; off draws cheaper ones for big hordes on slow machines.
    pub detail_high: bool,
    /// Particle budget multiplier (0.35 low, 0.65 medium, 1 high).
    pub particle_mult: f32,
    pub fog: bool,
    pub shake_mult: f32,
    pub boss_bar_lag: f32,
    pub screen: Vec2,
    pub dead: bool,
    pub won: bool,
    /// The vigil was kept and goes on past dawn; it only ends when everyone falls.
    pub endless: bool,
    /// Set on a network client: the world is only displayed, never simulated.
    pub is_client: bool,
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
    6.0 + (l - 1.0) * 10.0 + l.powf(1.7)
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
            remnant: remnant.map(|(p, n)| Remnant { pos: clamp_world(p.clamp_length_max(1400.0), 80.0), souls: n }),
            blood_moon: 0.0,
            moons_survived: 0,
            cursed: None,
            trials_won: 0,
            merchant: None,
            cat: Some(Cat { pos: clamp_world(rand_dir() * gen_range(1800.0, 3000.0), 300.0), owner: None, facing: 1.0, sitting: true, meow: gen_range(8.0, 20.0) }),
            boss: None,
            bosses_spawned: 0,
            bosses_killed: 0,
            arena: None,
            next_boss: BOSS_INTERVAL,
            next_shrine: 150.0,
            next_elite: 75.0,
            next_horde: 90.0,
            next_swarm: 140.0,
            next_relic: 40.0,
            spawn_acc: 0.0,
            kills: 0,
            cam: Vec2::ZERO,
            shake: 0.0,
            events: Vec::new(),
            sounds: Vec::new(),
            banner: None,
            toasts: Vec::new(),
            grid: Grid::new(),
            qbuf: Vec::new(),
            show_numbers: true,
            gore: true,
            detail_high: true,
            particle_mult: 1.0,
            fog: true,
            shake_mult: 1.0,
            boss_bar_lag: 1.0,
            screen: vec2(1280.0, 720.0),
            dead: false,
            won: false,
            endless: false,
            is_client: false,
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
        self.heroes.push(h);
        self.heroes.sort_by_key(|h| h.slot);
        self.heroes.iter().position(|h| h.slot == slot).unwrap()
    }

    /// Applies the "Head Start" relic: the vigil opens with a level-up.
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
    fn begin_round(&mut self) {
        self.level_round = self.level_round.wrapping_add(1);
        for hero in self.heroes.iter_mut() {
            hero.choices = hero.roll_choices();
            // Players who left can't choose; don't let them hold everyone up.
            hero.picked = !hero.connected;
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
        if !self.dead && self.heroes.iter().all(|h| h.downed) {
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

    fn update_pause(&mut self, dt: f32) {
        match self.pause {
            Pause::Running => {
                if self.pending_levels > 0 {
                    self.begin_round();
                }
            }
            Pause::Choosing => {
                if self.heroes.iter().all(|h| h.picked) {
                    self.pending_levels = self.pending_levels.saturating_sub(1);
                    if self.pending_levels > 0 {
                        self.begin_round();
                    } else if self.multiplayer() {
                        self.pause = Pause::Countdown(RESUME_DELAY);
                    } else {
                        self.pause = Pause::Running;
                    }
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
        if self.dead || self.won || self.is_client {
            self.update_fx(dt);
            return;
        }
        self.update_pause(dt);
        if self.pause != Pause::Running {
            self.update_fx(0.0);
            return;
        }
        self.time += dt;
        self.update_heroes(dt);
        self.update_spawns(dt);
        self.grid.rebuild(&self.enemies);
        self.update_weapons(dt);
        self.update_projs(dt);
        self.update_hazards(dt);
        self.update_enemies(dt);
        self.update_boss(dt);
        self.update_pickups(dt);
        self.update_shrines(dt);
        self.update_cat(dt);
        self.update_events(dt);
        self.boss_hp = match &self.boss {
            Some(b) => self.enemies.iter().find(|e| e.id == b.id).map(|e| (e.hp, e.max_hp)).unwrap_or((0.0, 1.0)),
            None => (0.0, 1.0),
        };
        let now = self.time;
        for h in self.heroes.iter_mut() {
            h.refresh_minute(now);
        }
        self.update_fx(dt);
        self.enemies.retain(|e| !e.dead);
        if self.time >= VICTORY_TIME && self.boss.is_none() && !self.won && !self.endless {
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
    pub fn client_update(&mut self, dt: f32) -> bool {
        let mut rolled = false;
        // The countdown runs here too, so it stays smooth between the slower paused snapshots.
        if let Pause::Countdown(t) = self.pause {
            self.pause = Pause::Countdown((t - dt).max(0.0));
        }
        if self.pause == Pause::Running && !self.dead && !self.won {
            let arena = self.arena;
            if let Some(hero) = self.heroes.get_mut(self.local) {
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
            }
        }
        self.update_fx(dt);
        rolled
    }
}
