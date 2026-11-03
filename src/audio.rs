//! Procedurally synthesized sound effects and music.
//!
//! Every sound can be replaced by dropping a file into `assets/sfx/<name>.wav|ogg`
//! or `assets/music/<name>.wav|ogg`; the file wins over the synthesized version.

use macroquad::audio::{load_sound_from_bytes, play_sound, set_sound_volume, stop_sound, PlaySoundParams, Sound};
use macroquad::time::get_time;
use std::collections::HashMap;
use std::f32::consts::TAU;

const SR: f32 = 44100.0;

struct Noise(u32);

impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// One-pole low-pass filter.
struct Lp(f32);

impl Lp {
    fn run(&mut self, x: f32, cutoff: f32) -> f32 {
        let a = 1.0 - (-TAU * cutoff / SR).exp();
        self.0 += a * (x - self.0);
        self.0
    }
}

fn buf(secs: f32) -> Vec<f32> {
    vec![0.0; (secs * SR) as usize]
}

fn t_of(i: usize) -> f32 {
    i as f32 / SR
}

fn saw(phase: f32) -> f32 {
    2.0 * (phase - phase.floor()) - 1.0
}

fn to_wav(samples: &[f32]) -> Vec<u8> {
    let peak = samples.iter().fold(0.0f32, |m, s| m.max(s.abs())).max(0.0001);
    let norm = if peak > 0.95 { 0.95 / peak } else { 1.0 };
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&44100u32.to_le_bytes());
    out.extend_from_slice(&(44100u32 * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        let v = (s * norm).clamp(-1.0, 1.0);
        out.extend_from_slice(&((v * 32767.0) as i16).to_le_bytes());
    }
    out
}

/// Fade in/out a few ms at both ends so nothing clicks.
fn declick(s: &mut [f32]) {
    let n = (0.004 * SR) as usize;
    let len = s.len();
    for i in 0..n.min(len) {
        let g = i as f32 / n as f32;
        s[i] *= g;
        s[len - 1 - i] *= g;
    }
}

fn noise_burst(secs: f32, seed: u32, cut_from: f32, cut_to: f32, decay: f32, hp: bool) -> Vec<f32> {
    let mut s = buf(secs);
    let mut n = Noise(seed);
    let mut lp = Lp(0.0);
    let mut lp2 = Lp(0.0);
    let len = s.len() as f32;
    for (i, v) in s.iter_mut().enumerate() {
        let k = i as f32 / len;
        let cut = cut_from + (cut_to - cut_from) * k;
        let x = lp.run(n.next(), cut);
        let y = if hp { x - lp2.run(x, cut * 0.3) } else { x };
        *v = y * (-t_of(i) * decay).exp() * (k * 40.0).min(1.0);
    }
    s
}

fn bell(secs: f32, f: f32, decay: f32) -> Vec<f32> {
    let mut s = buf(secs);
    let partials = [(1.0, 1.0), (2.76, 0.5), (5.40, 0.25), (8.93, 0.12), (0.5, 0.3)];
    for (i, v) in s.iter_mut().enumerate() {
        let t = t_of(i);
        let mut acc = 0.0;
        for (k, (m, a)) in partials.iter().enumerate() {
            acc += (TAU * f * m * t).sin() * a * (-t * decay * (1.0 + k as f32 * 0.6)).exp();
        }
        *v = acc;
    }
    s
}

fn mix(into: &mut Vec<f32>, other: &[f32], at: f32, gain: f32) {
    let start = (at * SR) as usize;
    if into.len() < start + other.len() {
        into.resize(start + other.len(), 0.0);
    }
    for (i, v) in other.iter().enumerate() {
        into[start + i] += v * gain;
    }
}

fn boom(secs: f32, f0: f32, seed: u32) -> Vec<f32> {
    drum(secs, f0, seed, 0.8)
}

/// A tuned thump with `noise` parts of rumble on top; music drums use little.
fn drum(secs: f32, f0: f32, seed: u32, noise: f32) -> Vec<f32> {
    let mut s = noise_burst(secs, seed, 900.0, 120.0, 6.0, false);
    let mut ph = 0.0;
    for (i, v) in s.iter_mut().enumerate() {
        let t = t_of(i);
        let f = f0 * (1.0 + 2.0 * (-t * 18.0).exp());
        ph += f / SR;
        *v = *v * noise + (TAU * ph).sin() * (-t * 5.0).exp() * 1.2;
    }
    s
}

fn synth(name: &str) -> Vec<f32> {
    match name {
        "slash" => noise_burst(0.2, 11, 6000.0, 700.0, 14.0, true),
        "knife" => noise_burst(0.09, 23, 9000.0, 3000.0, 30.0, true),
        "fire" => {
            let mut s = noise_burst(0.4, 31, 1800.0, 500.0, 7.0, false);
            let mut n = Noise(77);
            for v in s.iter_mut() {
                if n.next() > 0.985 {
                    *v += n.next() * 0.8;
                }
            }
            s
        }
        "explode" => boom(0.6, 55.0, 41),
        "slam" => {
            let mut s = boom(1.0, 38.0, 43);
            let b = bell(1.0, 70.0, 4.0);
            mix(&mut s, &b, 0.0, 0.5);
            s
        }
        "zap" => {
            let mut s = buf(0.3);
            let mut n = Noise(51);
            let mut f = 900.0;
            let mut ph = 0.0;
            for (i, v) in s.iter_mut().enumerate() {
                if i % 600 == 0 {
                    f = 300.0 + (n.next() + 1.0) * 900.0;
                }
                ph += f / SR;
                let sq = if (ph % 1.0) < 0.5 { 1.0 } else { -1.0 };
                *v = (sq * 0.4 + n.next() * 0.6) * (-t_of(i) * 12.0).exp();
            }
            s
        }
        "chime" => bell(0.5, 1320.0, 9.0),
        "bell" => bell(2.0, 220.0, 1.6),
        "hit" => {
            let mut s = buf(0.07);
            let mut n = Noise(61);
            let mut ph = 0.0;
            for (i, v) in s.iter_mut().enumerate() {
                let t = t_of(i);
                ph += (220.0 - t * 1800.0) / SR;
                *v = ((TAU * ph).sin() * 0.8 + n.next() * 0.3 * (-t * 80.0).exp()) * (-t * 40.0).exp();
            }
            s
        }
        "die" => {
            let mut s = noise_burst(0.18, 71, 2500.0, 300.0, 14.0, false);
            let mut ph = 0.0;
            for (i, v) in s.iter_mut().enumerate() {
                let t = t_of(i);
                ph += (160.0 - t * 500.0) / SR;
                *v += saw(ph) * 0.3 * (-t * 16.0).exp();
            }
            s
        }
        "pickup" => {
            let mut s = buf(0.07);
            let mut ph = 0.0;
            for (i, v) in s.iter_mut().enumerate() {
                let t = t_of(i);
                ph += (1000.0 + t * 6000.0) / SR;
                *v = (TAU * ph).sin() * (-t * 45.0).exp();
            }
            s
        }
        "levelup" => {
            let mut s = buf(1.2);
            for (k, f) in [440.0, 523.25, 659.25, 880.0].iter().enumerate() {
                let b = bell(0.9, *f, 3.0);
                mix(&mut s, &b, k as f32 * 0.07, 0.5);
            }
            s
        }
        "evolve" => {
            let mut s = buf(2.2);
            for (k, f) in [220.0, 277.18, 329.63, 440.0, 554.37, 659.25].iter().enumerate() {
                let b = bell(1.8, *f, 1.6);
                mix(&mut s, &b, k as f32 * 0.09, 0.4);
            }
            s
        }
        "roll" => noise_burst(0.22, 81, 1400.0, 400.0, 10.0, false),
        "hurt" => {
            let mut s = buf(0.25);
            let mut ph = 0.0;
            for (i, v) in s.iter_mut().enumerate() {
                let t = t_of(i);
                ph += (130.0 - t * 200.0) / SR;
                *v = ((TAU * ph).sin() * 3.0).tanh() * (-t * 12.0).exp();
            }
            s
        }
        "roar" => {
            let mut s = buf(1.8);
            let mut n = Noise(91);
            let mut lp = Lp(0.0);
            let mut ph = 0.0;
            for (i, v) in s.iter_mut().enumerate() {
                let t = t_of(i);
                let f = 62.0 + (TAU * 6.0 * t).sin() * 6.0 - t * 10.0;
                ph += f / SR;
                let env = (t * 6.0).min(1.0) * (-(t - 0.3).max(0.0) * 1.8).exp();
                let raw = saw(ph) * 0.7 + saw(ph * 1.51) * 0.4 + n.next() * 0.5;
                *v = (lp.run(raw, 900.0) * 2.5).tanh() * env;
            }
            s
        }
        "chest" => {
            let mut s = buf(1.2);
            for k in 0..8 {
                let b = bell(0.6, 600.0 * 1.12f32.powi(k), 6.0);
                mix(&mut s, &b, k as f32 * 0.06, 0.35);
            }
            s
        }
        "kindle" => {
            let mut s = noise_burst(1.6, 101, 600.0, 2500.0, 1.8, false);
            for f in [110.0, 164.81, 220.0] {
                let b = bell(1.6, f, 1.2);
                mix(&mut s, &b, 0.1, 0.4);
            }
            s
        }
        "death" => {
            let mut s = bell(3.0, 55.0, 0.8);
            let b = bell(3.0, 58.27, 0.9);
            mix(&mut s, &b, 0.05, 0.8);
            let n = noise_burst(2.5, 111, 300.0, 100.0, 1.2, false);
            mix(&mut s, &n, 0.0, 0.5);
            s
        }
        "select" => {
            let mut s = buf(0.035);
            for (i, v) in s.iter_mut().enumerate() {
                let t = t_of(i);
                *v = (TAU * 1800.0 * t).sin() * (-t * 120.0).exp();
            }
            s
        }
        "clang" => {
            let mut s = buf(0.6);
            let partials = [(820.0, 1.0), (1371.0, 0.7), (2219.0, 0.5), (3107.0, 0.35), (4511.0, 0.2)];
            for (i, v) in s.iter_mut().enumerate() {
                let t = t_of(i);
                let mut acc = 0.0;
                for (f, a) in partials {
                    acc += (TAU * f * t).sin() * a;
                }
                *v = acc * (-t * 9.0).exp();
            }
            s
        }
        "arrow" => noise_burst(0.12, 121, 4000.0, 1500.0, 20.0, true),
        "meow" => {
            // Pitch rises then falls with a little vibrato; two harmonics for a nasal tone.
            let mut s = buf(0.55);
            let mut ph = 0.0;
            for (i, v) in s.iter_mut().enumerate() {
                let t = t_of(i);
                let k = t / 0.55;
                let f = 520.0 + 380.0 * (k * std::f32::consts::PI).sin() - 120.0 * k + (TAU * 7.0 * t).sin() * 12.0;
                ph += f / SR;
                let env = (t * 30.0).min(1.0) * (1.0 - k).powf(0.7);
                *v = ((TAU * ph).sin() * 0.6 + (TAU * ph * 2.0).sin() * 0.3 + (TAU * ph * 3.0).sin() * 0.15) * env;
            }
            s
        }
        "bullet" => {
            let mut s = buf(0.08);
            for (i, v) in s.iter_mut().enumerate() {
                let t = t_of(i);
                *v = (TAU * (500.0 - t * 2500.0) * t).sin() * (-t * 35.0).exp();
            }
            s
        }
        _ => buf(0.05),
    }
}

/// A frequency nudged to a whole number of cycles per 16 s loop, so the loop has no seam.
fn loop_hz(f: f32) -> f32 {
    (f * 16.0).round() / 16.0
}

/// A soft voice from a few sine partials: warm, with no hiss and no buzz.
fn voice(f: f32, t: f32) -> f32 {
    (TAU * f * t).sin() + 0.45 * (TAU * 2.0 * f * t).sin() + 0.2 * (TAU * 3.0 * f * t).sin() + 0.08 * (TAU * 4.0 * f * t).sin()
}

/// Weight of chord `k` of four at time `t` in a 16 s loop: four seconds each, with a
/// one-second crossfade, wrapping around the loop.
fn chord_weight(k: usize, t: f32) -> f32 {
    let centre = k as f32 * 4.0 + 2.0;
    let d = (t - centre + 8.0).rem_euclid(16.0) - 8.0;
    let x = ((2.5 - d.abs()) / 1.0).clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn music_ambient() -> Vec<f32> {
    let len = 16.0;
    let mut s = buf(len);
    // A low drone on A that breathes, and a choir-like pad walking Am - F - Dm - E.
    let (a1, a1b, e2) = (55.0, 55.25, loop_hz(82.41));
    let chords: [[f32; 3]; 4] = [[220.0, 261.63, 329.63], [174.61, 220.0, 261.63], [146.83, 174.61, 220.0], [164.81, 207.65, 246.94]];
    let chords = chords.map(|c| c.map(loop_hz));
    for (i, v) in s.iter_mut().enumerate() {
        let t = t_of(i);
        // LFO rates are whole cycles per loop so the loop is seamless.
        let breath = 0.75 + 0.25 * (TAU * t / 8.0).sin();
        let drone = (voice(a1, t) + voice(a1b, t) * 0.8) * 0.18 + voice(e2, t) * 0.08;
        let mut pad = 0.0;
        for (k, chord) in chords.iter().enumerate() {
            let w = chord_weight(k, t);
            if w > 0.0 {
                pad += chord.iter().map(|f| voice(*f, t)).sum::<f32>() * w;
            }
        }
        *v = drone * breath + pad * 0.045 * (0.85 + 0.15 * (TAU * t / 4.0).sin());
    }
    // Sparse bell motif in A minor.
    let notes = [(1.0, 440.0), (3.5, 392.0), (6.0, 349.23), (7.0, 329.63), (10.0, 293.66), (12.5, 261.63), (14.0, 329.63)];
    for (at, f) in notes {
        let b = bell(2.5, f, 1.4);
        let start = (at * SR) as usize;
        for (k, x) in b.iter().enumerate() {
            let idx = (start + k) % s.len();
            s[idx] += x * 0.12;
        }
    }
    s
}

/// How one boss theme sounds.
struct Theme {
    bpm: f32,
    root: f32,
    /// Second drone voice as a ratio of the root; close to 1 grinds.
    clash: f32,
    cutoff: f32,
    /// One bit per sixteenth over a bar of 16, for kick and tom.
    kicks: u16,
    toms: u16,
    /// Tolling bells on the first beat of each bar.
    bells: bool,
    /// A fluttering arpeggio above the drone.
    arp: bool,
}

const THEMES: [Theme; 4] = [
    // The Gravewarden: slow and heavy, grinding minor second.
    Theme { bpm: 108.0, root: 55.0, clash: 1.0595, cutoff: 380.0, kicks: 0b1000_1000_1000_1010, toms: 0b0000_0010_0000_0001, bells: false, arp: false },
    // Pale Matriarch: restless, fluttering.
    Theme { bpm: 132.0, root: 61.74, clash: 1.4983, cutoff: 900.0, kicks: 0b1001_0010_1001_0010, toms: 0b0100_0000_0100_0100, bells: false, arp: true },
    // The Tolling Hermit: slow organ drone and bells.
    Theme { bpm: 84.0, root: 49.0, clash: 1.335, cutoff: 600.0, kicks: 0b1000_0000_1000_0000, toms: 0b0000_0000_0000_1000, bells: true, arp: false },
    // Ashen Sovereign: fast, driving, everything at once.
    Theme { bpm: 146.0, root: 43.65, clash: 1.0595, cutoff: 1100.0, kicks: 0b1010_1010_1010_1011, toms: 0b0001_0001_0001_0101, bells: true, arp: true },
];

/// Wraps whatever spilled past `len` seconds back onto the start, so the loop is seamless.
fn fold_loop(mut s: Vec<f32>, len: f32) -> Vec<f32> {
    let total = (len * SR) as usize;
    if s.len() > total {
        let tail: Vec<f32> = s.drain(total..).collect();
        for (k, x) in tail.iter().enumerate() {
            s[k % total] += x;
        }
    }
    s
}

fn music_boss(th: &Theme) -> Vec<f32> {
    let beat = 60.0 / th.bpm;
    let bars = 4;
    let len = beat * 4.0 * bars as f32;
    let mut s = buf(len);
    let (mut lp, mut lp2) = (Lp(0.0), Lp(0.0));
    for (i, v) in s.iter_mut().enumerate() {
        let t = t_of(i);
        let a = saw(th.root * t) + saw(th.root * th.clash * t) * 0.6 + saw(th.root * 2.0 * t) * 0.35;
        let pulse = 0.6 + 0.4 * (TAU * t / (beat * 2.0)).sin();
        // Two filter stages: the saws keep their grit without the fizz on top.
        let mut x = lp2.run(lp.run(a, th.cutoff), th.cutoff * 1.5) * 0.5 * pulse;
        if th.arp {
            // Root, fifth, octave, minor third, cycling on sixteenths.
            let step = (t / (beat * 0.25)) as usize;
            let ratio = [4.0, 6.0, 8.0, 4.8][step % 4];
            let env = (-((t % (beat * 0.25)) * 18.0)).exp();
            x += (TAU * th.root * ratio * t).sin() * 0.12 * env;
        }
        *v = x;
    }
    let kick = drum(0.4, 50.0, 5, 0.3);
    let tom = drum(0.3, 95.0, 9, 0.3);
    let sixteenth = beat * 0.25;
    for bar in 0..bars {
        for k in 0..16 {
            let at = (bar * 16 + k) as f32 * sixteenth;
            if th.kicks >> (15 - k) & 1 == 1 {
                mix(&mut s, &kick, at, 0.7);
            }
            if th.toms >> (15 - k) & 1 == 1 {
                mix(&mut s, &tom, at, 0.5);
            }
        }
        if th.bells {
            let b = bell(2.5, th.root * 4.0, 1.3);
            mix(&mut s, &b, bar as f32 * beat * 4.0, 0.35);
        }
    }
    fold_loop(s, len)
}

/// Drums under the ambient track, faded in as the fighting grows. Same length as the
/// ambient loop so both stay in step.
fn music_combat() -> Vec<f32> {
    let len = 16.0;
    let beat = len / 28.0;
    let mut s = buf(len);
    let kick = drum(0.35, 48.0, 13, 0.25);
    let tom = drum(0.3, 80.0, 17, 0.25);
    // A muted bell on the offbeat instead of a hissing shaker.
    let tick = bell(0.25, 880.0, 14.0);
    for b in 0..28 {
        let at = b as f32 * beat;
        mix(&mut s, &kick, at, 0.6);
        if b % 4 == 2 {
            mix(&mut s, &tom, at + beat * 0.5, 0.45);
        }
        mix(&mut s, &tick, at + beat * 0.5, 0.05);
    }
    for (i, v) in s.iter_mut().enumerate() {
        let t = t_of(i);
        // A low pulsing string, also whole cycles per loop.
        let pulse = (0.5 + 0.5 * (TAU * t / (beat * 2.0)).sin()).powf(2.0);
        *v += (voice(55.0, t) + voice(55.25, t)) * 0.12 * pulse;
    }
    fold_loop(s, len)
}

pub const SFX_NAMES: [&str; 24] = [
    "slash", "knife", "fire", "explode", "slam", "zap", "chime", "bell", "hit", "die", "pickup", "levelup", "evolve", "roll", "hurt", "roar", "chest", "kindle", "death", "select",
    "clang", "arrow", "bullet", "meow",
];

async fn load_override(dir: &str, name: &str) -> Option<Sound> {
    for ext in ["ogg", "wav"] {
        let p = format!("assets/{}/{}.{}", dir, name, ext);
        if let Ok(bytes) = std::fs::read(&p) {
            if let Ok(s) = load_sound_from_bytes(&bytes).await {
                return Some(s);
            }
        }
    }
    let bytes = if dir == "sfx" { crate::embedded::sfx(name) } else { None }?;
    load_sound_from_bytes(bytes).await.ok()
}

/// What is playing.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Music {
    Silence,
    /// Exploring; the combat drums fade in with `set_intensity`.
    Ambient,
    /// A boss theme by boss index.
    Boss(usize),
}

pub struct Audio {
    sfx: HashMap<&'static str, (Sound, f64)>,
    ambient: Sound,
    combat: Sound,
    bosses: Vec<Sound>,
    current: Music,
    intensity: f32,
    pub sfx_vol: f32,
    pub music_vol: f32,
    pub muted: bool,
}

impl Audio {
    pub async fn new() -> Audio {
        let mut sfx = HashMap::new();
        for name in SFX_NAMES {
            let snd = match load_override("sfx", name).await {
                Some(s) => s,
                None => {
                    let mut data = synth(name);
                    declick(&mut data);
                    load_sound_from_bytes(&to_wav(&data)).await.expect("synth wav")
                }
            };
            sfx.insert(name, (snd, 0.0));
        }
        let ambient = match load_override("music", "ambient").await {
            Some(s) => s,
            None => load_sound_from_bytes(&to_wav(&music_ambient())).await.expect("music"),
        };
        let combat = match load_override("music", "combat").await {
            Some(s) => s,
            None => load_sound_from_bytes(&to_wav(&music_combat())).await.expect("music"),
        };
        let mut bosses = Vec::new();
        for (i, th) in THEMES.iter().enumerate() {
            let snd = match load_override("music", &format!("boss{}", i + 1)).await {
                Some(s) => s,
                None => load_sound_from_bytes(&to_wav(&music_boss(th))).await.expect("music"),
            };
            bosses.push(snd);
        }
        Audio { sfx, ambient, combat, bosses, current: Music::Silence, intensity: 0.0, sfx_vol: 0.55, music_vol: 0.4, muted: false }
    }

    /// Plays a sound unless it already played within `gap` seconds.
    pub fn play_gap(&mut self, name: &str, vol: f32, gap: f64) {
        if self.muted {
            return;
        }
        let now = get_time();
        if let Some((snd, last)) = self.sfx.get_mut(name) {
            if now - *last < gap {
                return;
            }
            *last = now;
            play_sound(snd, PlaySoundParams { looped: false, volume: vol * self.sfx_vol });
        }
    }

    pub fn play(&mut self, name: &str, vol: f32) {
        self.play_gap(name, vol, 0.03);
    }

    fn stop_music(&self) {
        stop_sound(&self.ambient);
        stop_sound(&self.combat);
        for b in &self.bosses {
            stop_sound(b);
        }
    }

    pub fn music(&mut self, which: Music) {
        if self.current == which {
            return;
        }
        self.stop_music();
        self.current = which;
        if self.muted {
            return;
        }
        let params = PlaySoundParams { looped: true, volume: self.music_vol };
        match which {
            Music::Ambient => {
                play_sound(&self.ambient, params);
                play_sound(&self.combat, PlaySoundParams { looped: true, volume: self.music_vol * self.intensity });
            }
            Music::Boss(i) => play_sound(&self.bosses[i.min(self.bosses.len() - 1)], params),
            Music::Silence => {}
        }
    }

    /// 0 = calm, 1 = surrounded. Fades the combat drums under the ambient track.
    pub fn set_intensity(&mut self, k: f32) {
        self.intensity = k.clamp(0.0, 1.0);
        if self.current == Music::Ambient && !self.muted {
            set_sound_volume(&self.combat, self.music_vol * self.intensity);
        }
    }

    pub fn set_music_volume(&mut self, v: f32) {
        self.music_vol = v;
        set_sound_volume(&self.ambient, v);
        set_sound_volume(&self.combat, v * self.intensity);
        for b in &self.bosses {
            set_sound_volume(b, v);
        }
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        let cur = self.current;
        if self.muted {
            self.stop_music();
        } else {
            self.current = Music::Silence;
            self.music(cur);
        }
    }
}

/// How much of each music loop is hiss and buzz: the share of energy in the sample-to-sample
/// difference, which grows with noise and harsh overtones. For `tests::music_report`.
#[cfg(test)]
pub fn music_roughness() -> Vec<(&'static str, f32)> {
    let rough = |s: &[f32]| {
        let e: f32 = s.iter().map(|x| x * x).sum::<f32>().max(1e-9);
        let d: f32 = s.windows(2).map(|w| (w[1] - w[0]).powi(2)).sum();
        d / e * 1000.0
    };
    let mut out = vec![("ambient", rough(&music_ambient())), ("combat", rough(&music_combat()))];
    for (k, th) in THEMES.iter().enumerate() {
        out.push((["boss 1", "boss 2", "boss 3", "boss 4"][k], rough(&music_boss(th))));
    }
    out
}
