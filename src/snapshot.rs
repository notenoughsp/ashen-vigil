//! Host -> client world snapshots.
//!
//! Each client gets its own snapshot, cut to what its screen can show. Enemies and the
//! slow-changing part of each hero are delta-coded against the newest snapshot the client
//! acknowledged, so a quiet frame costs a few bytes per enemy. Nothing is lost: positions
//! arrive exactly as in a full snapshot, and a lost packet only means the next one is
//! coded against an older acknowledgement.

use crate::audio::SFX_NAMES;
use crate::data::*;
use crate::net::{Reader, Writer};
use crate::world::*;
use macroquad::prelude::*;
use std::collections::{HashMap, VecDeque};
use std::f32::consts::TAU;

const RANGE: f32 = 30000.0;
/// Room around a client's screen, so enemies walking in are already there.
const VIEW_MARGIN: f32 = 300.0;
/// Snapshots a client can be coded against, about a second.
const HISTORY: usize = 32;
/// Lantern orbits turn at 1 and -0.7 times `spin`; both repeat after 10 turns.
const SPIN_PERIOD: f32 = 10.0 * TAU;
/// Upper bound for an inflated snapshot, so garbage can't make us allocate gigabytes.
const MAX_SNAPSHOT: usize = 1 << 20;

/// What one snapshot told a client; later ones only send what changed since.
#[derive(Default)]
struct Baseline {
    /// Enemy id -> (kind and rank bits, x, y) as sent.
    enemies: HashMap<u16, (u8, i16, i16)>,
    /// Hero slot -> the hero's slow and fast parts as sent.
    heroes: HashMap<u8, (Vec<u8>, Vec<u8>)>,
    /// Pickup id -> (kind, value or life, x, y) as sent.
    pickups: HashMap<u16, (u8, u8, i16, i16)>,
}

/// Host side, one per client: what was sent, by tick.
#[derive(Default)]
pub struct Encoder {
    sent: VecDeque<(u32, Baseline)>,
}

/// Client side: what was applied, by tick, plus enemy positions for velocity estimates.
#[derive(Default)]
pub struct Decoder {
    seen: VecDeque<(u32, Baseline)>,
    prev: HashMap<u32, Vec2>,
    /// Newest applied snapshot; goes back to the host with every input.
    pub ack: u32,
}

/// One-shot cosmetic events since the last snapshot round, shared by every client.
#[derive(Default)]
pub struct Events {
    nums: Vec<DmgNum>,
    deaths: Vec<(Vec2, EnemyKind, bool)>,
    sounds: Vec<(&'static str, f32, Option<Vec2>)>,
}

/// Starts a snapshot round: tags new pickups and takes the one-shot events.
pub fn begin_round(w: &mut World) -> Events {
    for p in w.pickups.iter_mut().filter(|p| p.id == 0) {
        p.id = w.next_pickup_id;
        w.next_pickup_id = w.next_pickup_id.wrapping_add(1).max(1);
    }
    Events { nums: std::mem::take(&mut w.net_nums), deaths: std::mem::take(&mut w.net_deaths), sounds: std::mem::take(&mut w.net_sounds) }
}

/// What one client can see: its own hero's screen, or everyone's while it is down.
pub struct View {
    slot: u8,
    eyes: Vec<Vec2>,
    half: Vec2,
}

impl View {
    pub fn of(w: &World, slot: u8, screen: Vec2) -> View {
        let screen = if screen.x < 320.0 || screen.y < 200.0 { vec2(2560.0, 1440.0) } else { screen };
        let own = w.heroes.iter().find(|h| h.slot == slot);
        let eyes = match own {
            Some(h) if !h.downed => vec![h.p.pos],
            _ => w.heroes.iter().map(|h| h.p.pos).collect(),
        };
        View { slot, eyes, half: screen * 0.5 + Vec2::splat(VIEW_MARGIN) }
    }

    fn hears(&self, p: Vec2) -> bool {
        self.eyes.iter().any(|e| e.distance(p) < HEAR_MAX)
    }

    fn sees(&self, p: Vec2, r: f32) -> bool {
        self.eyes.iter().any(|e| (p.x - e.x).abs() < self.half.x + r && (p.y - e.y).abs() < self.half.y + r)
    }
}

/// World position as sent: whole pixels.
fn quant(v: f32) -> i16 {
    v.round().clamp(-32000.0, 32000.0) as i16
}

fn pick_code(kind: PickKind) -> u8 {
    match kind {
        PickKind::Soul(_) => 0,
        PickKind::Draught => 1,
        PickKind::Magnet => 2,
        PickKind::Coffer(false) => 3,
        PickKind::Coffer(true) => 4,
        PickKind::Buff(b) => 5 + b as u8,
        PickKind::Relic => 8,
        PickKind::Pyre => 9,
        PickKind::Bread => 10,
        PickKind::CursedCoffer => 11,
        PickKind::Key => 12,
    }
}

/// The inverse of `pick_code`; souls get their value from `extra`.
fn pick_kind(code: u8, extra: u8) -> PickKind {
    match code {
        0 => PickKind::Soul((extra as f32).max(0.5)),
        1 => PickKind::Draught,
        2 => PickKind::Magnet,
        3 => PickKind::Coffer(false),
        4 => PickKind::Coffer(true),
        5..=7 => PickKind::Buff(ALL_BUFFS[(code - 5) as usize]),
        8 => PickKind::Relic,
        9 => PickKind::Pyre,
        11 => PickKind::CursedCoffer,
        12 => PickKind::Key,
        _ => PickKind::Bread,
    }
}

fn zigzag(v: i32) -> u32 {
    ((v << 1) ^ (v >> 31)) as u32
}

fn unzigzag(v: u32) -> i32 {
    (v >> 1) as i32 ^ -((v & 1) as i32)
}

fn rgb(w: &mut Writer, c: Color) {
    w.u8((c.r * 255.0) as u8);
    w.u8((c.g * 255.0) as u8);
    w.u8((c.b * 255.0) as u8);
}

fn read_rgb(r: &mut Reader) -> Color {
    Color::from_rgba(r.u8(), r.u8(), r.u8(), 255)
}

/// A duration or factor in thousandths, up to 65 s.
fn ms(v: f32) -> u16 {
    (v * 1000.0).clamp(0.0, 65535.0) as u16
}

fn frac(v: f32, scale: f32) -> u8 {
    (v * scale).clamp(0.0, 255.0) as u8
}

struct Enc {
    w: Writer,
    anchor: Vec2,
}

impl Enc {
    fn in_range(&self, p: Vec2) -> bool {
        let d = p - self.anchor;
        d.x.abs() < RANGE && d.y.abs() < RANGE
    }
    fn pos(&mut self, p: Vec2) {
        let d = p - self.anchor;
        self.w.i16(d.x.clamp(-RANGE, RANGE) as i16);
        self.w.i16(d.y.clamp(-RANGE, RANGE) as i16);
    }
    fn vel(&mut self, v: Vec2) {
        self.w.i16(v.x.clamp(-30000.0, 30000.0) as i16);
        self.w.i16(v.y.clamp(-30000.0, 30000.0) as i16);
    }
}

struct Dec<'a> {
    r: Reader<'a>,
    anchor: Vec2,
}

impl<'a> Dec<'a> {
    fn pos(&mut self) -> Vec2 {
        self.anchor + vec2(self.r.i16() as f32, self.r.i16() as f32)
    }
    fn vel(&mut self) -> Vec2 {
        vec2(self.r.i16() as f32, self.r.i16() as f32)
    }
}

/// The part of a hero that changes on level-ups, pickups and toggles, not every frame.
fn hero_slow(h: &Hero) -> Vec<u8> {
    let mut w = Writer::default();
    w.str(&h.name);
    w.u8(class_to_u8(h.class));
    w.u8(h.p.revives as u8);
    w.u8(h.rerolls as u8);
    w.u64(h.banked);
    for v in h.stats.to_array() {
        w.f32(v);
    }
    w.u8(h.weapons.len() as u8);
    for wp in &h.weapons {
        w.u8(wp.kind.idx() as u8);
        w.u8(wp.level as u8);
        w.u8(wp.evolved as u8 | (wp.whole as u8) << 1);
        w.u8(wp.fused.len() as u8);
        for k in &wp.fused {
            w.u8(k.idx() as u8);
        }
    }
    w.u8(h.items.len() as u8);
    for it in &h.items {
        w.u8(it.kind.idx() as u8);
        w.u8(it.level as u8);
    }
    w.f32(h.ember_ready_at);
    match &h.wager {
        Some(g) => {
            w.u8(1 + g.tier as u8);
            w.u64(g.stake);
            w.f32(g.since.unwrap_or(-1.0));
        }
        None => w.u8(0),
    }
    w.u8(h.choices.len() as u8);
    for c in &h.choices {
        let (t, a) = c.encode();
        w.u8(t);
        w.u8(a);
    }
    w.u8(h.has_cat as u8);
    w.u8(h.skin);
    w.u8(h.challenges);
    w.0
}

fn read_hero_slow(data: &[u8], h: &mut Hero) -> bool {
    let mut r = Reader::new(data);
    h.name = r.str();
    h.class = class_from_u8(r.u8());
    h.p.revives = r.u8() as u32;
    h.rerolls = r.u8() as u32;
    h.banked = r.u64();
    let mut f = [0.0f32; 17];
    for v in f.iter_mut() {
        *v = r.f32();
    }
    h.stats = Stats::from_array(f);
    // Keep the animation state; the fast part refreshes it right after.
    let old = std::mem::take(&mut h.weapons);
    for i in 0..r.u8() as usize {
        let kind = WeaponKind::from_idx(r.u8());
        let mut wp = old.get(i).filter(|o| o.kind == kind).cloned().unwrap_or_else(|| Weapon::new(kind));
        wp.level = r.u8() as u32;
        let flags = r.u8();
        wp.evolved = flags & 1 != 0;
        let fused: Vec<WeaponKind> = (0..r.u8().min(11)).map(|_| WeaponKind::from_idx(r.u8())).collect();
        if fused != wp.fused {
            let (level, evolved) = (wp.level, wp.evolved);
            wp = Weapon::with_parts(kind, fused);
            wp.level = level;
            wp.evolved = evolved;
        }
        wp.whole = flags & 2 != 0;
        h.weapons.push(wp);
    }
    h.items.clear();
    for _ in 0..r.u8() {
        let kind = ItemKind::from_idx(r.u8());
        let level = r.u8() as u32;
        h.items.push(Item { kind, level });
    }
    h.ember_ready_at = r.f32();
    h.wager = match r.u8() {
        0 => None,
        n => {
            let stake = r.u64();
            let since = r.f32();
            Some(Wager { tier: (n as usize - 1).min(2), stake, since: (since >= 0.0).then_some(since) })
        }
    };
    h.choices.clear();
    for _ in 0..r.u8() {
        let (t, a) = (r.u8(), r.u8());
        h.choices.push(Choice::decode(t, a));
    }
    h.has_cat = r.u8() & 1 != 0;
    h.skin = r.u8().min(SKINS.len() as u8);
    h.challenges = r.u8() & 7;
    r.ok
}

/// What moves every frame. Per-weapon damage only goes to its owner, who alone shows it.
fn write_hero_fast(w: &mut Writer, h: &Hero, viewer: bool) {
    for v in [h.p.pos.x, h.p.pos.y, h.p.facing.x, h.p.facing.y] {
        w.f32(v);
    }
    for v in [h.p.hp, h.p.stamina, h.p.roll_t, h.p.roll_dir.x, h.p.roll_dir.y, h.p.iframes, h.revive, h.p.walk] {
        w.f32(v);
    }
    w.u8(h.p.moving as u8
        | (h.downed as u8) << 1
        | (h.connected as u8) << 2
        | (h.picked as u8) << 3
        | (h.rested as u8) << 4
        | ((h.first_hurt < f32::MAX) as u8) << 5
        | (h.pact as u8) << 6);
    w.u64(h.p.souls);
    for b in h.buffs {
        w.f32(b);
    }
    w.f64(h.dmg_total);
    w.f32(h.dmg_minute);
    if viewer {
        for d in h.dmg_weapon {
            w.f32(d as f32);
        }
    }
    w.f32(h.aim.x);
    w.f32(h.aim.y);
    // The skill cooldown in quarter seconds, for the HUD.
    w.u8((h.skill_cd * 4.0).ceil().clamp(0.0, 255.0) as u8);
    // Only lanterns (orbit angle) and the chime (pulse) show their timers.
    for wp in &h.weapons {
        for part in wp.parts() {
            match part.kind {
                WeaponKind::Lanterns => w.u16((wp.spins[part.slot].rem_euclid(SPIN_PERIOD) / SPIN_PERIOD * 65535.0) as u16),
                WeaponKind::Chime => w.u16((wp.timers[part.slot] * 1000.0).clamp(0.0, 65535.0) as u16),
                _ => {}
            }
        }
    }
}

/// Reads the fast part into `h`. With `predicted`, movement state the client owns is kept.
fn read_hero_fast(r: &mut Reader, h: &mut Hero, predicted: bool) {
    let pos = vec2(r.f32(), r.f32());
    let facing = vec2(r.f32(), r.f32());
    let hp = r.f32();
    let stamina = r.f32();
    let roll_t = r.f32();
    let roll_dir = vec2(r.f32(), r.f32());
    h.p.iframes = r.f32();
    h.revive = r.f32();
    let walk = r.f32();
    let bits = r.u8();
    let moving = bits & 1 != 0;
    let was_downed = h.downed;
    h.downed = bits & 2 != 0;
    h.connected = bits & 4 != 0;
    h.picked = bits & 8 != 0;
    h.rested = bits & 16 != 0;
    h.pact = bits & 64 != 0;
    // Only whether the hero has been hit matters to clients, not when.
    if bits & 32 != 0 {
        h.first_hurt = h.first_hurt.min(0.0);
    }
    h.p.hp = hp;
    h.p.souls = r.u64();
    for b in h.buffs.iter_mut() {
        *b = r.f32();
    }
    h.dmg_total = r.f64();
    h.dmg_minute = r.f32();
    if predicted {
        for d in h.dmg_weapon.iter_mut() {
            *d = r.f32() as f64;
        }
    }
    h.aim = vec2(r.f32(), r.f32());
    h.skill_cd = r.u8() as f32 / 4.0;
    for wp in h.weapons.iter_mut() {
        for part in wp.parts() {
            match part.kind {
                WeaponKind::Lanterns => wp.spins[part.slot] = r.u16() as f32 / 65535.0 * SPIN_PERIOD,
                WeaponKind::Chime => wp.timers[part.slot] = r.u16() as f32 / 1000.0,
                _ => {}
            }
        }
    }
    // Snap our predicted position back if the host moved us (revive, fog ring) or we drifted.
    let resync = !predicted || h.downed || was_downed != h.downed || h.p.pos.distance(pos) > 220.0;
    if resync {
        h.p.pos = pos;
        h.p.facing = facing;
        h.p.moving = moving;
        h.p.roll_t = roll_t;
        h.p.roll_dir = roll_dir;
        h.p.walk = walk;
        if !predicted {
            h.p.stamina = stamina;
        }
    }
}

#[cfg(test)]
thread_local! { static LAST_RAW: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }

/// Size of the last snapshot before deflate, for the size report test.
#[cfg(test)]
pub fn last_raw_len() -> usize {
    LAST_RAW.with(|c| c.get())
}

impl Encoder {
    /// A deflated snapshot for one client, coded against the newest snapshot it acknowledged.
    pub fn encode(&mut self, w: &World, ev: &Events, view: &View, tick: u32, ack: u32) -> Vec<u8> {
        // Older snapshots than the acknowledged one will never be needed again.
        if self.sent.iter().any(|(t, _)| *t == ack) {
            self.sent.retain(|(t, _)| *t >= ack);
        }
        let empty = Baseline::default();
        let (base_tick, base) = self.sent.iter().find(|(t, _)| *t == ack).map(|(t, b)| (*t, b)).unwrap_or((0, &empty));
        let (raw, next) = encode_raw(w, ev, view, tick, base_tick, base);
        self.sent.push_back((tick, next));
        while self.sent.len() > HISTORY {
            self.sent.pop_front();
        }
        #[cfg(test)]
        LAST_RAW.with(|c| c.set(raw.len()));
        miniz_oxide::deflate::compress_to_vec(&raw, 1)
    }
}

impl Decoder {
    /// Applies a snapshot. False when it is damaged or coded against something we no longer have.
    pub fn decode(&mut self, w: &mut World, data: &[u8], local_slot: u8, since: f32) -> bool {
        let Ok(raw) = miniz_oxide::inflate::decompress_to_vec_with_limit(data, MAX_SNAPSHOT) else { return false };
        let mut r = Reader::new(&raw);
        let tick = r.u32();
        let base_tick = r.u32();
        if !r.ok || tick <= self.ack {
            return false;
        }
        let empty = Baseline::default();
        let base = match base_tick {
            0 => &empty,
            t => match self.seen.iter().find(|(s, _)| *s == t) {
                Some((_, b)) => b,
                None => return false,
            },
        };
        let Some(next) = decode_raw(w, r, base, local_slot, &mut self.prev, since) else { return false };
        self.seen.push_back((tick, next));
        while self.seen.len() > HISTORY {
            self.seen.pop_front();
        }
        self.ack = tick;
        true
    }
}

fn encode_raw(w: &World, ev: &Events, view: &View, tick: u32, base_tick: u32, base: &Baseline) -> (Vec<u8>, Baseline) {
    let mut next = Baseline::default();
    let anchor = w.heroes.iter().map(|h| h.p.pos).fold(Vec2::ZERO, |a, b| a + b) / w.heroes.len().max(1) as f32;
    let mut e = Enc { w: Writer::default(), anchor };
    e.w.u32(tick);
    e.w.u32(base_tick);
    e.w.f32(anchor.x);
    e.w.f32(anchor.y);
    e.w.f32(w.time);
    e.w.u32(w.kills);
    e.w.u16(w.level as u16);
    e.w.f32(w.xp);
    e.w.f32(w.xp_next);
    let (pause, countdown) = match w.pause {
        Pause::Running => (0, 0.0),
        Pause::Choosing => (1, 0.0),
        Pause::Countdown(t) => (2, t),
        Pause::Held => (3, 0.0),
    };
    e.w.u8(pause);
    e.w.f32(countdown);
    e.w.u16(w.level_round);
    e.w.u8(w.dead as u8
        | (w.won as u8) << 1
        | (w.endless as u8) << 2
        | (w.grave_found as u8) << 3
        | (w.boss_rush as u8) << 4
        | (w.bell_found as u8) << 5
        | (w.heart_found as u8) << 6);
    e.w.u8(w.biome as u8);
    e.w.u8(w.curses);
    e.w.u16(w.bosses_killed as u16);
    // Tenths of a second until the next lord while the warning shows.
    e.w.u8((w.boss_warning * 10.0).ceil().clamp(0.0, 255.0) as u8);
    // When the endless night began, so a player who rejoins hears the right stage.
    if w.endless {
        e.w.f32(w.endless_since);
    }

    e.w.u8(w.heroes.len() as u8);
    for h in &w.heroes {
        e.w.u8(h.slot);
        let slow = hero_slow(h);
        let mut fast = Writer::default();
        write_hero_fast(&mut fast, h, h.slot == view.slot);
        let fast = fast.0;
        let (base_slow, base_fast) = base.heroes.get(&h.slot).map(|(s, f)| (s.as_slice(), f.as_slice())).unwrap_or_default();
        // Bit 0: slow part unchanged. Bit 1: fast part XORed with the last one, so the
        // fields that did not change turn into zeros and deflate away.
        let same_slow = base.heroes.contains_key(&h.slot) && base_slow == slow;
        let xor = base_fast.len() == fast.len() && !fast.is_empty();
        e.w.u8(same_slow as u8 | (xor as u8) << 1);
        if !same_slow {
            e.w.u16(slow.len() as u16);
            e.w.0.extend_from_slice(&slow);
        }
        if xor {
            e.w.0.extend(fast.iter().zip(base_fast).map(|(a, b)| a ^ b));
        } else {
            e.w.u16(fast.len() as u16);
            e.w.0.extend_from_slice(&fast);
        }
        next.heroes.insert(h.slot, (slow, fast));
    }

    // Enemies by id, each as a change against what the client already has; bosses always
    // go along for the health bar.
    let mut enemies: Vec<&Enemy> = w.enemies.iter().filter(|en| !en.dead && (en.boss || view.sees(en.pos, en.r))).collect();
    enemies.sort_by_key(|en| en.id as u16);
    e.w.u16(enemies.len() as u16);
    let mut last_id = 0u16;
    for en in enemies {
        let id = en.id as u16;
        e.w.varint(id.wrapping_sub(last_id) as u32);
        last_id = id;
        // Five bits of kind (there are more than sixteen), then elite and boss.
        let code = en.kind.to_u8() | (en.elite as u8) << 5 | (en.boss as u8) << 6;
        // A lit cultist bomb blinks through the flash bit.
        let blink = en.fuse > 0.0 && (en.fuse * 12.0) as i32 % 2 == 0;
        let live = (en.burn > 0.0) as u8 | ((en.flash > 0.0 || blink) as u8) << 1;
        let (x, y) = (quant(en.pos.x), quant(en.pos.y));
        match base.enemies.get(&id) {
            Some(&(c, bx, by)) if c == code => {
                e.w.u8(live);
                e.w.varint(zigzag(x as i32 - bx as i32));
                e.w.varint(zigzag(y as i32 - by as i32));
            }
            _ => {
                e.w.u8(live | 0x80);
                e.w.u8(code);
                e.w.i16(x);
                e.w.i16(y);
            }
        }
        if en.kind.has_alpha() {
            e.w.u8(frac(en.alpha, 255.0));
        }
        next.enemies.insert(id, (code, x, y));
    }

    let projs: Vec<usize> = (0..w.projs.len()).filter(|&i| e.in_range(w.projs[i].pos) && view.sees(w.projs[i].pos, w.projs[i].r)).collect();
    e.w.u16(projs.len() as u16);
    for i in projs {
        let p = &w.projs[i];
        e.w.u8(p.kind.to_u8() | (p.evo as u8) << 5);
        let (pos, vel, r, age, area) = (p.pos, p.vel, p.r, p.age, p.area);
        e.pos(pos);
        e.vel(vel);
        e.w.u16(r.clamp(0.0, 65535.0) as u16);
        e.w.u16(ms(age));
        e.w.u16(ms(area));
    }

    let bullets: Vec<usize> = (0..w.ebullets.len()).filter(|&i| e.in_range(w.ebullets[i].pos) && view.sees(w.ebullets[i].pos, w.ebullets[i].r)).collect();
    e.w.u16(bullets.len() as u16);
    for i in bullets {
        let b = &w.ebullets[i];
        let (pos, vel, r, homing) = (b.pos, b.vel, b.r, b.homing > 0.0);
        e.pos(pos);
        e.vel(vel);
        e.w.u8(r.min(255.0) as u8);
        e.w.bool(homing);
    }

    // Souls only on screen; relics, coffers and the like everywhere for the edge markers.
    let mut picks: Vec<&Pickup> = w.pickups.iter().filter(|p| e.in_range(p.pos) && (!matches!(p.kind, PickKind::Soul(_)) || view.sees(p.pos, 20.0))).collect();
    picks.sort_by_key(|p| p.id as u16);
    e.w.u16(picks.len() as u16);
    let mut last_id = 0u16;
    for p in picks {
        let id = p.id as u16;
        e.w.varint(id.wrapping_sub(last_id) as u32);
        last_id = id;
        let code = pick_code(p.kind);
        // Souls carry their value (for size and colour), relics their remaining seconds.
        let extra = match p.kind {
            PickKind::Soul(v) => v.clamp(0.0, 255.0) as u8,
            _ if p.life.is_finite() => p.life.clamp(0.0, 254.0) as u8,
            _ => 255,
        };
        let (x, y) = (quant(p.pos.x), quant(p.pos.y));
        match base.pickups.get(&id) {
            // An untagged pickup (id 0) never matches, so it always goes in full.
            Some(&(c, ex, bx, by)) if p.id != 0 && c == code && ex == extra => {
                if (bx, by) == (x, y) {
                    e.w.u8(0);
                } else {
                    e.w.u8(1);
                    e.w.varint(zigzag(x as i32 - bx as i32));
                    e.w.varint(zigzag(y as i32 - by as i32));
                }
            }
            _ => {
                e.w.u8(2);
                e.w.u8(code);
                e.w.u8(extra);
                e.w.i16(x);
                e.w.i16(y);
            }
        }
        if p.id != 0 {
            next.pickups.insert(id, (code, extra, x, y));
        }
    }

    let fx: Vec<usize> = (0..w.fx.len())
        .filter(|&i| {
            let f = &w.fx[i];
            match f.kind {
                FxKind::Lightning { from } => view.sees(f.pos, 50.0) || view.sees(from, 50.0),
                FxKind::Explosion { r, .. } | FxKind::Ring { r, .. } => view.sees(f.pos, r),
                FxKind::Slash { radius, .. } | FxKind::Sweep { radius, .. } => view.sees(f.pos, radius),
                FxKind::Ghost => view.sees(f.pos, 30.0),
                FxKind::Whip { to } => view.sees(f.pos, 40.0) || view.sees(to, 40.0),
                FxKind::Sword { r } | FxKind::Pipe { r } => view.sees(f.pos, r + 80.0),
                FxKind::Crook { radius, .. } | FxKind::Gash { radius, .. } | FxKind::Ribs { radius, .. } => view.sees(f.pos, radius),
                FxKind::Gaze { to } | FxKind::Lash { to } => view.sees(f.pos, 40.0) || view.sees(to, 40.0),
                FxKind::Spike { r } => view.sees(f.pos, r + 40.0),
            }
        })
        .take(255)
        .collect();
    e.w.u8(fx.len() as u8);
    for i in fx {
        let f = &w.fx[i];
        let (pos, life, max_life, kind) = (f.pos, f.life, f.max_life, f.kind);
        e.pos(pos);
        e.w.u16(ms(life));
        e.w.u16(ms(max_life));
        match kind {
            FxKind::Slash { ang, radius, evo, owner } => {
                e.w.u8(0);
                e.w.f32(ang);
                e.w.u16(radius as u16);
                e.w.bool(evo);
                e.w.u8(owner);
            }
            FxKind::Sweep { radius, evo, owner } => {
                e.w.u8(5);
                e.w.u16(radius as u16);
                e.w.bool(evo);
                e.w.u8(owner);
            }
            FxKind::Explosion { r, color } => {
                e.w.u8(1);
                e.w.u16(r as u16);
                rgb(&mut e.w, color);
            }
            FxKind::Lightning { from } => {
                e.w.u8(2);
                e.pos(from);
            }
            FxKind::Ring { r, color } => {
                e.w.u8(3);
                e.w.u16(r as u16);
                rgb(&mut e.w, color);
            }
            FxKind::Ghost => e.w.u8(4),
            FxKind::Whip { to } => {
                e.w.u8(6);
                e.pos(to);
            }
            FxKind::Sword { r } => {
                e.w.u8(7);
                e.w.u16(r as u16);
            }
            FxKind::Crook { radius, owner } => {
                e.w.u8(8);
                e.w.u16(radius as u16);
                e.w.u8(owner);
            }
            FxKind::Pipe { r } => {
                e.w.u8(9);
                e.w.u16(r as u16);
            }
            FxKind::Gash { ang, radius, owner } => {
                e.w.u8(10);
                e.w.f32(ang);
                e.w.u16(radius as u16);
                e.w.u8(owner);
            }
            FxKind::Gaze { to } => {
                e.w.u8(11);
                e.pos(to);
            }
            FxKind::Spike { r } => {
                e.w.u8(12);
                e.w.u16(r as u16);
            }
            FxKind::Ribs { radius, owner } => {
                e.w.u8(13);
                e.w.u16(radius as u16);
                e.w.u8(owner);
            }
            FxKind::Lash { to } => {
                e.w.u8(14);
                e.pos(to);
            }
        }
    }

    let pools: Vec<&FirePool> = w.pools.iter().filter(|pl| view.sees(pl.pos, pl.r)).take(255).collect();
    e.w.u8(pools.len() as u8);
    for pl in pools {
        let (pos, r, life, max_life, kind) = (pl.pos, pl.r, pl.life, pl.max_life, pl.kind);
        e.pos(pos);
        e.w.u16(r as u16);
        e.w.u16(ms(life));
        e.w.u16(ms(max_life));
        e.w.u8(kind as u8);
    }
    e.w.u8(w.waves.len().min(255) as u8);
    for i in 0..w.waves.len().min(255) {
        let (pos, r, max_r) = (w.waves[i].pos, w.waves[i].r, w.waves[i].max_r);
        e.pos(pos);
        e.w.u16(r as u16);
        e.w.u16(max_r as u16);
    }
    e.w.u8(w.pillars.len().min(255) as u8);
    for i in 0..w.pillars.len().min(255) {
        let p = &w.pillars[i];
        let (pos, r, delay, max_delay, after) = (p.pos, p.r, p.delay, p.max_delay, p.after);
        e.pos(pos);
        e.w.u8(r.min(255.0) as u8);
        e.w.u16(ms(delay));
        e.w.u16(ms(max_delay));
        e.w.u16(ms(after));
    }
    e.w.u8(w.telegraphs.len().min(255) as u8);
    for i in 0..w.telegraphs.len().min(255) {
        let t = &w.telegraphs[i];
        let (a, b, width, life, max_life, line) = (t.a, t.b, t.width, t.life, t.max_life, t.line);
        e.pos(a);
        e.pos(b);
        e.w.u16(width as u16);
        e.w.u16(ms(life));
        e.w.u16(ms(max_life));
        e.w.bool(line);
    }
    e.w.u8(w.shrines.len().min(255) as u8);
    for i in 0..w.shrines.len().min(255) {
        let (pos, kindle, used) = (w.shrines[i].pos, w.shrines[i].kindle, w.shrines[i].used);
        e.pos(pos);
        e.w.u16(ms(kindle));
        e.w.bool(used);
    }

    e.w.u8(w.braziers.len().min(255) as u8);
    for b in w.braziers.iter().take(255) {
        e.w.u8((b.burn / 2.0).ceil().clamp(0.0, 255.0) as u8);
        e.w.u8((b.kindle / BRAZIER_KINDLE * 255.0).clamp(0.0, 255.0) as u8);
    }
    e.w.f32(w.blood_moon);
    match &w.cursed {
        Some(c) => {
            let (pos, kindle, curse, accepted) = (c.pos, c.kindle, c.curse, c.accepted);
            e.w.bool(true);
            e.pos(pos);
            e.w.u16(ms(kindle));
            e.w.f32(curse);
            e.w.bool(accepted);
        }
        None => e.w.bool(false),
    }
    match &w.stone {
        Some(s) => {
            e.w.bool(true);
            e.pos(s.pos);
        }
        None => e.w.bool(false),
    }
    match &w.hourglass {
        Some(g) => {
            e.w.bool(true);
            e.pos(g.pos);
            e.w.u8((g.kindle / HOURGLASS_KINDLE * 255.0).clamp(0.0, 255.0) as u8);
        }
        None => e.w.bool(false),
    }
    // Omens: altars, the vault, the mist and the weather.
    e.w.u8(w.altars.len().min(8) as u8);
    for a in w.altars.iter().take(8) {
        e.pos(a.pos);
        e.w.u8((a.kindle / ALTAR_KINDLE * 255.0).clamp(0.0, 255.0) as u8);
        e.w.u8(a.life.clamp(0.0, 255.0) as u8);
    }
    match &w.vault {
        Some(v) => {
            e.w.bool(true);
            e.pos(v.pos);
            e.w.u8(v.open as u8 | (w.vault_key as u8) << 1);
        }
        None => e.w.bool(false),
    }
    e.w.u8(w.mist.clamp(0.0, 255.0).ceil() as u8);
    e.w.u8(Weather::to_u8(w.weather.map(|(k, _)| k)));
    e.w.u16(w.weather.map(|(_, left)| left.clamp(0.0, 65535.0) as u16).unwrap_or(0));
    match &w.quest {
        Some(q) => {
            e.w.bool(true);
            e.w.u8(q.kind.to_u8());
            e.w.u16(q.goal.min(u16::MAX as u32) as u16);
            e.w.u16(q.progress.min(u16::MAX as u32) as u16);
            e.w.u8(q.left.clamp(0.0, 255.0).ceil() as u8);
        }
        None => e.w.bool(false),
    }
    match &w.merchant {
        Some(m) => {
            let (pos, life, wares, sold, prices) = (m.pos, m.life, m.wares, m.sold, m.prices);
            e.w.bool(true);
            e.pos(pos);
            e.w.f32(life);
            e.w.u8(sold.iter().enumerate().fold(0u8, |bits, (i, s)| bits | (*s as u8) << i));
            for (w, p) in wares.iter().zip(prices) {
                e.w.u8(w.to_u8());
                e.w.u32(p);
            }
        }
        None => e.w.bool(false),
    }
    match &w.cat {
        Some(c) => {
            let (pos, owner, facing, sitting) = (c.pos, c.owner.unwrap_or(255), c.facing, c.sitting);
            e.w.bool(true);
            e.pos(pos);
            e.w.u8(owner);
            e.w.bool(facing > 0.0);
            e.w.bool(sitting);
        }
        None => e.w.bool(false),
    }
    match w.arena {
        Some((c, r)) => {
            e.w.bool(true);
            e.pos(c);
            e.w.u16(r as u16);
        }
        None => e.w.bool(false),
    }
    match w.boss_kind {
        Some(k) => {
            e.w.bool(true);
            e.w.u8(k as u8);
            e.w.f32(w.boss_hp.0);
            e.w.f32(w.boss_hp.1);
        }
        None => e.w.bool(false),
    }
    e.w.u8(w.bars.len().min(16) as u8);
    for &(code, hp, max) in w.bars.iter().take(16) {
        e.w.u8(code);
        e.w.f32(hp);
        e.w.f32(max);
    }
    match &w.banner {
        Some((big, small, c, life)) => {
            e.w.bool(true);
            e.w.str(big);
            e.w.str(small);
            rgb(&mut e.w, *c);
            e.w.f32(*life);
        }
        None => e.w.bool(false),
    }
    match &w.quote {
        Some((text, c, life)) => {
            e.w.bool(true);
            e.w.str(text);
            rgb(&mut e.w, *c);
            e.w.f32(*life);
        }
        None => e.w.bool(false),
    }
    e.w.u8(w.toasts.len() as u8);
    for (t, life) in &w.toasts {
        e.w.str(t);
        e.w.f32(*life);
    }

    // One-shot cosmetic events since the last snapshot.
    let nums: Vec<&DmgNum> = ev.nums.iter().filter(|n| view.sees(n.pos, 40.0)).take(255).collect();
    e.w.u8(nums.len() as u8);
    for n in nums {
        e.pos(n.pos);
        e.w.varint(n.val.clamp(0.0, u32::MAX as f32) as u32);
        e.w.bool(n.crit);
    }
    let deaths: Vec<&(Vec2, EnemyKind, bool)> = ev.deaths.iter().filter(|d| view.sees(d.0, 60.0)).take(255).collect();
    e.w.u8(deaths.len() as u8);
    for (p, k, elite) in deaths {
        e.pos(*p);
        e.w.u8(k.to_u8());
        e.w.bool(*elite);
    }
    // Sounds this client could not hear anyway stay home; the rest carry their place.
    let audible: Vec<&(&'static str, f32, Option<Vec2>)> = ev.sounds.iter().filter(|s| s.2.is_none_or(|p| view.hears(p))).take(255).collect();
    e.w.u8(audible.len() as u8);
    for (name, vol, pos) in audible {
        let idx = SFX_NAMES.iter().position(|n| n == name).unwrap_or(0) as u8;
        e.w.u8(idx | if pos.is_some() { 0x80 } else { 0 });
        e.w.u8(frac(*vol, 255.0));
        if let Some(p) = pos {
            e.pos(*p);
        }
    }
    (e.w.0, next)
}

/// Applies a snapshot body to a client world; returns what it told us, as the next baseline.
/// `prev` remembers enemy positions to estimate velocities; `since` is the time since the last snapshot.
fn decode_raw(w: &mut World, r: Reader, base: &Baseline, local_slot: u8, prev: &mut HashMap<u32, Vec2>, since: f32) -> Option<Baseline> {
    let mut next = Baseline::default();
    let mut d = Dec { r, anchor: Vec2::ZERO };
    d.anchor = vec2(d.r.f32(), d.r.f32());
    w.time = d.r.f32();
    w.kills = d.r.u32();
    w.level = d.r.u16() as u32;
    w.xp = d.r.f32();
    w.xp_next = d.r.f32();
    let pause = d.r.u8();
    let countdown = d.r.f32();
    w.pause = match pause {
        1 => Pause::Choosing,
        2 => Pause::Countdown(countdown),
        3 => Pause::Held,
        _ => Pause::Running,
    };
    w.level_round = d.r.u16();
    let flags = d.r.u8();
    w.dead = flags & 1 != 0;
    w.won = flags & 2 != 0;
    w.endless = flags & 4 != 0;
    w.grave_found = flags & 8 != 0;
    w.bell_found = flags & 32 != 0;
    w.heart_found = flags & 64 != 0;
    w.boss_rush = flags & 16 != 0;
    // The field shrinks when a boss rush is learned of; braziers must match the host's.
    w.fit_field();
    w.biome = Biome::from_u8(d.r.u8());
    w.curses = d.r.u8() & 63;
    w.bosses_killed = d.r.u16() as u32;
    w.boss_warning = d.r.u8() as f32 / 10.0;
    if w.endless {
        w.endless_since = d.r.f32();
    }

    let n = d.r.u8() as usize;
    for _ in 0..n {
        let slot = d.r.u8();
        let bits = d.r.u8();
        let based = base.heroes.get(&slot);
        let slow = if bits & 1 != 0 {
            based?.0.clone()
        } else {
            let len = d.r.u16() as usize;
            d.r.bytes(len).to_vec()
        };
        let fast: Vec<u8> = if bits & 2 != 0 {
            let base_fast = &based?.1;
            d.r.bytes(base_fast.len()).iter().zip(base_fast).map(|(a, b)| a ^ b).collect()
        } else {
            let len = d.r.u16() as usize;
            d.r.bytes(len).to_vec()
        };
        let idx = match w.heroes.iter().position(|h| h.slot == slot) {
            Some(i) => i,
            None => {
                let mut h = Hero::new(slot, "", Class::Wanderer, Profile::default());
                h.remote = true;
                w.heroes.push(h);
                w.heroes.len() - 1
            }
        };
        let predicted = slot == local_slot;
        let h = &mut w.heroes[idx];
        h.slot = slot;
        if !read_hero_slow(&slow, h) {
            return None;
        }
        let mut fr = Reader::new(&fast);
        read_hero_fast(&mut fr, h, predicted);
        if !fr.ok {
            return None;
        }
        next.heroes.insert(slot, (slow, fast));
    }
    w.heroes.sort_by_key(|h| h.slot);
    w.local = w.heroes.iter().position(|h| h.slot == local_slot).unwrap_or(0);

    let n = d.r.u16() as usize;
    let since = since.clamp(1.0 / 120.0, 0.25);
    let mut next_prev = HashMap::with_capacity(n);
    let mut enemies = Vec::with_capacity(n);
    let mut id = 0u16;
    for _ in 0..n {
        id = id.wrapping_add(d.r.varint() as u16);
        let live = d.r.u8();
        let (code, x, y) = if live & 0x80 != 0 {
            (d.r.u8(), d.r.i16(), d.r.i16())
        } else {
            let &(code, bx, by) = base.enemies.get(&id)?;
            let x = bx as i32 + unzigzag(d.r.varint());
            let y = by as i32 + unzigzag(d.r.varint());
            (code, x.clamp(-32000, 32000) as i16, y.clamp(-32000, 32000) as i16)
        };
        next.enemies.insert(id, (code, x, y));
        let kind = EnemyKind::from_u8(code & 31);
        let elite = code & 32 != 0;
        let pos = vec2(x as f32, y as f32);
        let alpha = if kind.has_alpha() { d.r.u8() as f32 / 255.0 } else { 1.0 };
        let id = id as u32;
        let vel = prev.get(&id).map(|p| ((pos - *p) / since).clamp_length_max(700.0)).unwrap_or(Vec2::ZERO);
        next_prev.insert(id, pos);
        enemies.push(Enemy {
            id,
            kind,
            pos,
            knock: Vec2::ZERO,
            hp: 1.0,
            max_hp: 1.0,
            speed: 0.0,
            dmg: 0.0,
            r: kind.radius(elite),
            xp: 0.0,
            flash: if live & 2 != 0 { 0.05 } else { 0.0 },
            burn: if live & 1 != 0 { 1.0 } else { 0.0 },
            burn_dps: 0.0,
            burn_acc: 0.0,
            slow: 0.0,
            last_hit: [0.0; ALL_WEAPONS.len()],
            burn_src: Src { hero: 0, weapon: WeaponKind::Blade },
            timer: 0.0,
            elite,
            boss: code & 64 != 0,
            dead: false,
            phase: (id % 628) as f32 / 100.0,
            alpha,
            fuse: -1.0,
            hunt: None,
            life_left: f32::INFINITY,
            net_vel: vel,
            step: 0,
            lunge: Vec2::ZERO,
        });
    }
    *prev = next_prev;
    w.enemies = enemies;

    w.projs.clear();
    for _ in 0..d.r.u16() {
        let k = d.r.u8();
        let kind = PKind::from_u8(k & 31);
        let pos = d.pos();
        let vel = d.vel();
        let r = d.r.u16() as f32;
        let age = d.r.u16() as f32 / 1000.0;
        let area = d.r.u16() as f32 / 1000.0;
        let src = Src { hero: 0, weapon: WeaponKind::Blade };
        w.projs.push(Proj { pos, vel, dir: vel.normalize_or_zero(), r, dmg: 0.0, pierce: 0, life: 1.0, age, kind, evo: k & 32 != 0, area, src, hits: Vec::new(), dead: false });
    }

    w.ebullets.clear();
    for _ in 0..d.r.u16() {
        let pos = d.pos();
        let vel = d.vel();
        let r = d.r.u8() as f32;
        let homing = if d.r.bool() { 1.0 } else { 0.0 };
        w.ebullets.push(EBullet { pos, vel, r, dmg: 0.0, life: 1.0, homing, hp: 1.0, hit_cd: 0.0 });
    }

    w.pickups.clear();
    let mut id = 0u16;
    for _ in 0..d.r.u16() {
        id = id.wrapping_add(d.r.varint() as u16);
        let (code, extra, x, y) = match d.r.u8() {
            0 => *base.pickups.get(&id)?,
            1 => {
                let &(code, extra, bx, by) = base.pickups.get(&id)?;
                let x = bx as i32 + unzigzag(d.r.varint());
                let y = by as i32 + unzigzag(d.r.varint());
                (code, extra, x.clamp(-32000, 32000) as i16, y.clamp(-32000, 32000) as i16)
            }
            _ => (d.r.u8(), d.r.u8(), d.r.i16(), d.r.i16()),
        };
        next.pickups.insert(id, (code, extra, x, y));
        let kind = pick_kind(code, extra);
        let mut p = Pickup::new(vec2(x as f32, y as f32), kind);
        p.id = id as u32;
        p.life = if code == 0 || extra == 255 { f32::INFINITY } else { extra as f32 };
        w.pickups.push(p);
    }

    // Keep our own predicted roll ghosts, take everything else from the host.
    w.fx.retain(|f| matches!(f.kind, FxKind::Ghost));
    for _ in 0..d.r.u8() {
        let pos = d.pos();
        let life = d.r.u16() as f32 / 1000.0;
        let max_life = (d.r.u16() as f32 / 1000.0).max(0.01);
        let kind = match d.r.u8() {
            0 => FxKind::Slash { ang: d.r.f32(), radius: d.r.u16() as f32, evo: d.r.bool(), owner: d.r.u8() },
            1 => FxKind::Explosion { r: d.r.u16() as f32, color: read_rgb(&mut d.r) },
            2 => FxKind::Lightning { from: d.pos() },
            3 => FxKind::Ring { r: d.r.u16() as f32, color: read_rgb(&mut d.r) },
            5 => FxKind::Sweep { radius: d.r.u16() as f32, evo: d.r.bool(), owner: d.r.u8() },
            6 => FxKind::Whip { to: d.pos() },
            7 => FxKind::Sword { r: d.r.u16() as f32 },
            8 => FxKind::Crook { radius: d.r.u16() as f32, owner: d.r.u8() },
            9 => FxKind::Pipe { r: d.r.u16() as f32 },
            10 => FxKind::Gash { ang: d.r.f32(), radius: d.r.u16() as f32, owner: d.r.u8() },
            11 => FxKind::Gaze { to: d.pos() },
            12 => FxKind::Spike { r: d.r.u16() as f32 },
            13 => FxKind::Ribs { radius: d.r.u16() as f32, owner: d.r.u8() },
            14 => FxKind::Lash { to: d.pos() },
            _ => FxKind::Ghost,
        };
        w.fx.push(Fx { pos, life, max_life, kind });
    }

    w.pools.clear();
    for _ in 0..d.r.u8() {
        let pos = d.pos();
        let r = d.r.u16() as f32;
        let life = d.r.u16() as f32 / 1000.0;
        let max_life = (d.r.u16() as f32 / 1000.0).max(0.02);
        let kind = PoolKind::from_u8(d.r.u8());
        let src = Src { hero: 0, weapon: WeaponKind::Hex };
        // Clients only draw candles; a tick far off keeps them from spitting flame of their own.
        w.pools.push(FirePool { pos, r, life, max_life, dps: 0.0, tick: f32::INFINITY, src, kind });
    }
    w.waves.clear();
    for _ in 0..d.r.u8() {
        let pos = d.pos();
        let r = d.r.u16() as f32;
        let max_r = d.r.u16() as f32;
        w.waves.push(Shockwave { pos, r, max_r, speed: 0.0, dmg: 0.0, hit: [true; MAX_HEROES] });
    }
    w.pillars.clear();
    for _ in 0..d.r.u8() {
        let pos = d.pos();
        let r = d.r.u8() as f32;
        let delay = d.r.u16() as f32 / 1000.0;
        let max_delay = (d.r.u16() as f32 / 1000.0).max(0.01);
        let after = d.r.u16() as f32 / 1000.0;
        w.pillars.push(Pillar { pos, r, delay, max_delay, dmg: 0.0, after });
    }
    w.telegraphs.clear();
    for _ in 0..d.r.u8() {
        let a = d.pos();
        let b = d.pos();
        let width = d.r.u16() as f32;
        let life = d.r.u16() as f32 / 1000.0;
        let max_life = (d.r.u16() as f32 / 1000.0).max(0.01);
        let line = d.r.bool();
        w.telegraphs.push(Telegraph { a, b, width, life, max_life, line });
    }
    w.shrines.clear();
    for _ in 0..d.r.u8() {
        let pos = d.pos();
        let kindle = d.r.u16() as f32 / 1000.0;
        let used = d.r.bool();
        w.shrines.push(Shrine { pos, life: 10.0, kindle, used });
    }

    let n = d.r.u8() as usize;
    for i in 0..n {
        let (burn, kindle) = (d.r.u8() as f32 * 2.0, d.r.u8() as f32 / 255.0 * BRAZIER_KINDLE);
        if let Some(b) = w.braziers.get_mut(i) {
            b.burn = burn;
            b.kindle = kindle;
        }
    }
    w.blood_moon = d.r.f32();
    w.cursed = if d.r.bool() {
        let pos = d.pos();
        let kindle = d.r.u16() as f32 / 1000.0;
        let curse = d.r.f32();
        let accepted = d.r.bool();
        Some(CursedShrine { pos, life: 10.0, kindle, curse, accepted })
    } else {
        None
    };
    w.stone = if d.r.bool() { Some(WagerStone { pos: d.pos() }) } else { None };
    w.hourglass = if d.r.bool() {
        let pos = d.pos();
        let kindle = d.r.u8() as f32 / 255.0 * HOURGLASS_KINDLE;
        Some(Hourglass { pos, life: 10.0, kindle })
    } else {
        None
    };
    let n = d.r.u8() as usize;
    w.altars.clear();
    for _ in 0..n {
        let pos = d.pos();
        let kindle = d.r.u8() as f32 / 255.0 * ALTAR_KINDLE;
        let life = d.r.u8() as f32;
        w.altars.push(Altar { pos, life, kindle });
    }
    w.vault = if d.r.bool() {
        let pos = d.pos();
        let bits = d.r.u8();
        w.vault_key = bits & 2 != 0;
        Some(Vault { pos, open: bits & 1 != 0, left: 1.0 })
    } else {
        w.vault_key = false;
        None
    };
    w.mist = d.r.u8() as f32;
    let weather = Weather::from_u8(d.r.u8());
    let left = d.r.u16() as f32;
    w.weather = weather.map(|k| (k, left));
    w.quest = if d.r.bool() {
        let kind = QuestKind::from_u8(d.r.u8());
        let goal = d.r.u16() as u32;
        let progress = d.r.u16() as u32;
        let left = d.r.u8() as f32;
        Some(Quest { kind, goal, progress, left })
    } else {
        None
    };
    w.merchant = if d.r.bool() {
        let pos = d.pos();
        let life = d.r.f32();
        let bits = d.r.u8();
        let mut wares = [Ware::Draught; SHELF];
        let mut prices = [0u32; SHELF];
        for (w, p) in wares.iter_mut().zip(prices.iter_mut()) {
            *w = Ware::from_u8(d.r.u8());
            *p = d.r.u32();
        }
        Some(Merchant { pos, life, wares, sold: std::array::from_fn(|i| bits & (1 << i) != 0), prices })
    } else {
        None
    };
    w.cat = if d.r.bool() {
        let pos = d.pos();
        let owner = d.r.u8();
        let facing = if d.r.bool() { 1.0 } else { -1.0 };
        let sitting = d.r.bool();
        Some(Cat { pos, owner: (owner != 255).then_some(owner), facing, sitting, meow: 0.0 })
    } else {
        None
    };
    w.arena = if d.r.bool() {
        let c = d.pos();
        Some((c, d.r.u16() as f32))
    } else {
        None
    };
    w.boss_kind = if d.r.bool() {
        let k = BossKind::from_u8(d.r.u8());
        w.boss_hp = (d.r.f32(), d.r.f32());
        Some(k)
    } else {
        None
    };
    w.bars.clear();
    for _ in 0..d.r.u8().min(16) {
        let (code, hp, max) = (d.r.u8(), d.r.f32(), d.r.f32());
        w.bars.push((code, hp, max));
    }
    w.banner = if d.r.bool() {
        let big = d.r.str();
        let small = d.r.str();
        let c = read_rgb(&mut d.r);
        let life = d.r.f32();
        Some((big, small, c, life))
    } else {
        None
    };
    w.quote = if d.r.bool() {
        let text = d.r.str();
        let c = read_rgb(&mut d.r);
        let life = d.r.f32();
        Some((text, c, life))
    } else {
        None
    };
    w.toasts.clear();
    for _ in 0..d.r.u8() {
        let t = d.r.str();
        let life = d.r.f32();
        w.toasts.push((t, life));
    }

    for _ in 0..d.r.u8() {
        let pos = d.pos();
        let val = d.r.varint() as f32;
        let crit = d.r.bool();
        if w.show_numbers && w.nums.len() < 250 {
            w.nums.push(DmgNum { pos, val, life: 0.7, crit });
        }
        if macroquad::rand::gen_range(0.0, 1.0) < 0.5 {
            w.blood_spray(pos + vec2(0.0, 10.0), Vec2::ZERO, EnemyKind::Hollow.blood(), 10.0, if crit { 5 } else { 2 });
        }
    }
    for _ in 0..d.r.u8() {
        let pos = d.pos();
        let kind = EnemyKind::from_u8(d.r.u8());
        let elite = d.r.bool();
        let r = if matches!(kind, EnemyKind::Boss(_)) {
            44.0
        } else if elite {
            20.0
        } else {
            12.0
        };
        w.death_burst(pos, kind, elite, r);
    }
    for _ in 0..d.r.u8() {
        let code = d.r.u8();
        let vol = d.r.u8() as f32 / 255.0;
        let pos = (code & 0x80 != 0).then(|| d.pos());
        if let Some(name) = SFX_NAMES.get((code & 0x7f) as usize) {
            w.sounds.push((name, vol, 0.03, pos));
        }
    }
    d.r.ok.then_some(next)
}
