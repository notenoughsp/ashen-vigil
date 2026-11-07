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

/// One song of the vigil: a drone, four chords that walk the 16 s loop, and a bell motif.
struct Act {
    /// Drone root and its slightly detuned twin.
    drone: f32,
    /// Second drone voice as a ratio of the root.
    drone_hi: f32,
    chords: [[f32; 3]; 4],
    /// (seconds into the loop, frequency) of each bell.
    bells: &'static [(f32, f32)],
    bell_decay: f32,
    bell_gain: f32,
    pad_gain: f32,
    /// A soft heartbeat under everything, beats per loop (0 for none).
    heart: u32,
    /// High music-box notes instead of church bells.
    music_box: bool,
}

/// The first ten minutes: A minor, slow and wide.
const ACT_DUSK: Act = Act {
    drone: 55.0,
    drone_hi: 1.4983,
    chords: [[220.0, 261.63, 329.63], [174.61, 220.0, 261.63], [146.83, 174.61, 220.0], [164.81, 207.65, 246.94]],
    bells: &[(1.0, 440.0), (3.5, 392.0), (6.0, 349.23), (7.0, 329.63), (10.0, 293.66), (12.5, 261.63), (14.0, 329.63)],
    bell_decay: 1.4,
    bell_gain: 0.12,
    pad_gain: 0.045,
    heart: 0,
    music_box: false,
};

/// Minutes ten to twenty: D minor, a heartbeat, the bells come closer together.
const ACT_MIDNIGHT: Act = Act {
    drone: 73.42,
    drone_hi: 1.4983,
    chords: [[293.66, 349.23, 440.0], [233.08, 293.66, 349.23], [196.0, 233.08, 293.66], [220.0, 277.18, 329.63]],
    bells: &[(0.5, 587.33), (1.5, 523.25), (2.5, 440.0), (4.5, 466.16), (5.5, 440.0), (8.5, 392.0), (9.5, 349.23), (10.5, 329.63), (12.5, 349.23), (14.0, 277.18)],
    bell_decay: 1.8,
    bell_gain: 0.1,
    pad_gain: 0.05,
    heart: 14,
    music_box: false,
};

/// Minutes twenty to thirty: C sharp minor with a lowered second, tolling, the last stretch before dawn.
const ACT_WITCHING: Act = Act {
    drone: 69.3,
    drone_hi: 1.0595,
    chords: [[277.18, 329.63, 415.3], [293.66, 369.99, 440.0], [220.0, 277.18, 329.63], [207.65, 261.63, 311.13]],
    bells: &[(0.0, 138.59), (2.0, 554.37), (3.0, 523.25), (4.0, 138.59), (6.0, 493.88), (8.0, 138.59), (9.0, 440.0), (10.0, 415.3), (12.0, 146.83), (14.0, 415.3), (15.0, 392.0)],
    bell_decay: 1.1,
    bell_gain: 0.11,
    pad_gain: 0.055,
    heart: 28,
    music_box: false,
};

/// Cute mode: C major on a music box.
const ACT_LULLABY: Act = Act {
    drone: 65.41,
    drone_hi: 1.5,
    chords: [[261.63, 329.63, 392.0], [220.0, 261.63, 329.63], [174.61, 220.0, 261.63], [196.0, 246.94, 293.66]],
    bells: &[
        (0.0, 1046.5),
        (0.5, 1318.5),
        (1.0, 1568.0),
        (2.0, 1318.5),
        (4.0, 880.0),
        (4.5, 1046.5),
        (5.0, 1318.5),
        (6.0, 1174.7),
        (8.0, 1046.5),
        (8.5, 880.0),
        (9.0, 698.46),
        (10.0, 880.0),
        (12.0, 987.77),
        (12.5, 1174.7),
        (13.0, 1568.0),
        (14.0, 1318.5),
        (15.0, 1174.7),
    ],
    bell_decay: 3.5,
    bell_gain: 0.07,
    pad_gain: 0.03,
    heart: 0,
    music_box: true,
};

fn music_act(act: &Act) -> Vec<f32> {
    let len = 16.0;
    let mut s = buf(len);
    let (lo, lo_b, hi) = (loop_hz(act.drone), loop_hz(act.drone) + 0.25, loop_hz(act.drone * act.drone_hi));
    let chords = act.chords.map(|c| c.map(loop_hz));
    for (i, v) in s.iter_mut().enumerate() {
        let t = t_of(i);
        // LFO rates are whole cycles per loop so the loop is seamless.
        let breath = 0.75 + 0.25 * (TAU * t / 8.0).sin();
        let drone = (voice(lo, t) + voice(lo_b, t) * 0.8) * 0.18 + voice(hi, t) * 0.08;
        let mut pad = 0.0;
        for (k, chord) in chords.iter().enumerate() {
            let w = chord_weight(k, t);
            if w > 0.0 {
                pad += chord.iter().map(|f| voice(*f, t)).sum::<f32>() * w;
            }
        }
        let drone_gain = if act.music_box { 0.4 } else { 1.0 };
        *v = drone * breath * drone_gain + pad * act.pad_gain * (0.85 + 0.15 * (TAU * t / 4.0).sin());
    }
    for &(at, f) in act.bells {
        let b = if act.music_box { music_box(f, act.bell_decay) } else { bell(2.5, f, act.bell_decay) };
        let start = (at * SR) as usize;
        for (k, x) in b.iter().enumerate() {
            let idx = (start + k) % s.len();
            s[idx] += x * act.bell_gain;
        }
    }
    if act.heart > 0 {
        // Lub-dub, quiet and low, in step with the combat drums.
        let thump = drum(0.25, 46.0, 29, 0.0);
        let beat = len / act.heart as f32;
        for b in 0..act.heart {
            let at = b as f32 * beat;
            for (off, g) in [(0.0, 0.22), (0.16, 0.14)] {
                let start = ((at + off) * SR) as usize;
                for (k, x) in thump.iter().enumerate() {
                    let idx = (start + k) % s.len();
                    s[idx] += x * g;
                }
            }
        }
    }
    s
}

/// A plucked comb note: bright, round and short.
fn music_box(f: f32, decay: f32) -> Vec<f32> {
    let mut s = buf(1.2);
    for (i, v) in s.iter_mut().enumerate() {
        let t = t_of(i);
        let env = (-t * decay).exp() * (t * 400.0).min(1.0);
        *v = ((TAU * f * t).sin() + 0.3 * (TAU * 2.0 * f * t).sin() * (-t * decay * 2.0).exp() + 0.12 * (TAU * 3.01 * f * t).sin() * (-t * decay * 3.0).exp()) * env;
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

const THEMES: [Theme; 10] = [
    // The Gravewarden: slow and heavy, grinding minor second.
    Theme { bpm: 108.0, root: 55.0, clash: 1.0595, cutoff: 380.0, kicks: 0b1000_1000_1000_1010, toms: 0b0000_0010_0000_0001, bells: false, arp: false },
    // Pale Matriarch: restless, fluttering.
    Theme { bpm: 132.0, root: 61.74, clash: 1.4983, cutoff: 900.0, kicks: 0b1001_0010_1001_0010, toms: 0b0100_0000_0100_0100, bells: false, arp: true },
    // The Tolling Hermit: slow organ drone and bells.
    Theme { bpm: 84.0, root: 49.0, clash: 1.335, cutoff: 600.0, kicks: 0b1000_0000_1000_0000, toms: 0b0000_0000_0000_1000, bells: true, arp: false },
    // Ashen Sovereign: fast, driving, everything at once.
    Theme { bpm: 146.0, root: 43.65, clash: 1.0595, cutoff: 1100.0, kicks: 0b1010_1010_1010_1011, toms: 0b0001_0001_0001_0101, bells: true, arp: true },
    // The Drowned Bride: a slow, sad waltz under water.
    Theme { bpm: 90.0, root: 51.91, clash: 1.1892, cutoff: 500.0, kicks: 0b1000_0000_0000_1000, toms: 0b0000_1000_1000_0000, bells: true, arp: true },
    // The Bog Mother: low and wet, a heavy croak of a beat.
    Theme { bpm: 98.0, root: 41.2, clash: 1.0595, cutoff: 320.0, kicks: 0b1001_0000_1001_0010, toms: 0b0000_0100_0000_0100, bells: false, arp: false },
    // The Iron Penitent: marching chains.
    Theme { bpm: 120.0, root: 46.25, clash: std::f32::consts::SQRT_2, cutoff: 800.0, kicks: 0b1000_1000_1000_1000, toms: 0b0010_0010_0010_0011, bells: true, arp: false },
    // The Hollow Choir: an organ that never breathes.
    Theme { bpm: 76.0, root: 55.0, clash: 1.4983, cutoff: 1000.0, kicks: 0b1000_0000_0000_0000, toms: 0b0000_0000_1000_0000, bells: true, arp: true },
    // The Stone Seraph: wide and high, bells in the tower.
    Theme { bpm: 112.0, root: 58.27, clash: 1.335, cutoff: 900.0, kicks: 0b1000_0010_1000_0000, toms: 0b0000_1000_0000_1010, bells: true, arp: false },
    // The Pale Hound: a racing heartbeat.
    Theme { bpm: 158.0, root: 49.0, clash: 1.0595, cutoff: 700.0, kicks: 0b1010_1000_1010_1000, toms: 0b0000_0010_0000_0011, bells: false, arp: true },
];

/// How many endless stages there are; each lasts `ENDLESS_STAGE` seconds, the last one stays.
pub const ENDLESS_STAGES: u8 = 8;
/// The endless stage the boss rush plays when it has no songs of its own.
const RUSH_STAGE: usize = 4;
pub const ENDLESS_STAGE: f32 = 180.0;

/// The endless night grows harsher every stage: faster, higher, denser drums, then the grind.
fn endless_theme(k: u8) -> Theme {
    let k = k.min(ENDLESS_STAGES - 1);
    const KICKS: [u16; 8] = [
        0b1000_0000_1000_0000,
        0b1000_0000_1000_1000,
        0b1000_1000_1000_1000,
        0b1000_1000_1000_1010,
        0b1010_1000_1010_1000,
        0b1010_1010_1010_1010,
        0b1010_1010_1010_1011,
        0b1011_1010_1011_1011,
    ];
    const TOMS: [u16; 8] =
        [0, 0b0000_0000_0000_1000, 0b0000_0010_0000_0010, 0b0000_0010_0000_0101, 0b0001_0001_0001_0101, 0b0101_0001_0101_0101, 0b0101_0101_0101_0101, 0b0101_0101_0111_0101];
    Theme {
        bpm: 112.0 + 9.0 * k as f32,
        // Up a semitone each stage.
        root: 49.0 * 2f32.powf(k as f32 / 12.0),
        clash: match k {
            0..=1 => 1.4983,
            2..=4 => 1.335,
            _ => 1.0595,
        },
        cutoff: 450.0 + 140.0 * k as f32,
        kicks: KICKS[k as usize],
        toms: TOMS[k as usize],
        bells: k >= 3,
        arp: k >= 2,
    }
}

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

/// Music loop number `i` in the order `Audio::new` loads them: three acts, the lullaby,
/// the combat drums, the boss themes, the endless stages.
fn music_track(i: usize) -> Vec<f32> {
    let bosses = THEMES.len();
    match i {
        0 => music_act(&ACT_DUSK),
        1 => music_act(&ACT_MIDNIGHT),
        2 => music_act(&ACT_WITCHING),
        3 => music_act(&ACT_LULLABY),
        4 => music_combat(),
        i if i < 5 + bosses => music_boss(&THEMES[i - 5]),
        i => music_boss(&endless_theme((i - 5 - bosses) as u8)),
    }
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
    let bytes = match dir {
        "sfx" => crate::embedded::sfx(name),
        _ => None,
    }?;
    load_sound_from_bytes(bytes).await.ok()
}

/// The kinds of sound with a volume of their own: save key and name in the options.
/// Weapons, hits and pickups have none: they follow the sound effects volume alone.
pub const SOUND_KINDS: [(&str, &str); 2] = [("foes", "Foes"), ("interface", "Interface")];

/// Which kind of sound a sound is, for its volume; None for those without a volume of their own.
pub fn sound_kind(name: &str) -> Option<usize> {
    match name {
        "slash" | "knife" | "fire" | "explode" | "slam" | "zap" | "arrow" | "bullet" | "clang" | "roll" => None,
        "hit" | "hurt" | "death" => None,
        "pickup" | "chest" | "levelup" | "evolve" | "kindle" | "chime" => None,
        "die" | "roar" | "bell" => Some(0),
        _ => Some(1),
    }
}
/// What is playing.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Music {
    Silence,
    /// The menus: the `menu` songs, else the graveyard's, else the first act.
    Menu,
    /// A field's songs (see `songs`), else one synthesized song per ten minutes of the
    /// vigil (0, 1, 2) with combat drums that fade in with `set_intensity`.
    Vigil(u8, u8),
    /// Cute mode's music box instead of the vigil songs.
    Lullaby,
    /// After dawn on a field: a stage that grows harsher every three minutes.
    Endless(u8, u8),
    /// A lord's song on a field, by boss index.
    Boss(u8, usize),
    /// The boss rush: its own songs, one after another, whichever lord is about; else
    /// one synthesized track that never changes with the lords.
    Rush,
}

pub struct Audio {
    sfx: HashMap<&'static str, (Sound, f64)>,
    acts: Vec<Sound>,
    lullaby: Sound,
    combat: Sound,
    endless: Vec<Sound>,
    bosses: Vec<Sound>,
    current: Music,
    /// The recorded songs, which win over the synthesized ones.
    songs: crate::songs::Player,
    /// The boss song played last, so the next lord hears another.
    last_boss_song: Option<usize>,
    intensity: f32,
    pub sfx_vol: f32,
    /// Volume of each of `SOUND_KINDS`, on top of `sfx_vol`.
    pub kind_vol: [f32; SOUND_KINDS.len()],
    pub music_vol: f32,
    /// The lords' music, as a share of `music_vol`. The boss rush keeps the plain music volume.
    pub boss_vol: f32,
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
        // Every loop is synthesized on its own thread; only loading the sounds needs this one.
        let mut names: Vec<String> = vec!["ambient".into(), "midnight".into(), "witching".into(), "lullaby".into(), "combat".into()];
        names.extend((1..=THEMES.len()).map(|i| format!("boss{}", i)));
        names.extend((1..=ENDLESS_STAGES).map(|i| format!("endless{}", i)));
        let wavs: Vec<Vec<u8>> = std::thread::scope(|sc| {
            let jobs: Vec<_> = names
                .iter()
                .enumerate()
                .map(|(i, name)| {
                    let custom = ["ogg", "wav"].iter().any(|ext| std::path::Path::new(&format!("assets/music/{}.{}", name, ext)).exists());
                    sc.spawn(move || if custom { Vec::new() } else { to_wav(&music_track(i)) })
                })
                .collect();
            jobs.into_iter().map(|j| j.join().unwrap_or_default()).collect()
        });
        let mut sounds = Vec::new();
        for (name, wav) in names.iter().zip(&wavs) {
            let snd = match load_override("music", name).await {
                Some(s) => s,
                None => load_sound_from_bytes(wav).await.expect("music"),
            };
            sounds.push(snd);
        }
        let mut it = sounds.into_iter();
        let mut take = |n: usize| (0..n).filter_map(|_| it.next()).collect::<Vec<Sound>>();
        let acts = take(3);
        let lullaby = take(1).remove(0);
        let combat = take(1).remove(0);
        let bosses = take(THEMES.len());
        let endless = take(ENDLESS_STAGES as usize);
        Audio {
            sfx,
            acts,
            lullaby,
            combat,
            endless,
            bosses,
            current: Music::Silence,
            songs: Default::default(),
            last_boss_song: None,
            intensity: 0.0,
            sfx_vol: 0.55,
            kind_vol: [1.0; SOUND_KINDS.len()],
            music_vol: 0.4,
            boss_vol: 1.0,
            muted: false,
        }
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
            play_sound(snd, PlaySoundParams { looped: false, volume: vol * self.sfx_vol * sound_kind(name).map_or(1.0, |k| self.kind_vol[k]) });
        }
    }

    pub fn play(&mut self, name: &str, vol: f32) {
        self.play_gap(name, vol, 0.03);
    }

    fn loops(&self) -> impl Iterator<Item = &Sound> {
        self.acts.iter().chain([&self.lullaby, &self.combat]).chain(&self.bosses).chain(&self.endless)
    }

    fn stop_music(&self) {
        for s in self.loops() {
            stop_sound(s);
        }
    }

    pub fn music(&mut self, which: Music) {
        if self.current == which {
            return;
        }
        self.stop_music();
        self.current = which;
        self.songs.cancel_once();
        if self.muted {
            return;
        }
        use crate::songs::{boss_songs, first_folder, FIELD_DIRS};
        let dir = |f: u8| FIELD_DIRS[(f as usize).min(FIELD_DIRS.len() - 1)];
        // Recorded songs first. A lord's song plays over the field's playlist, which goes on
        // quietly underneath and comes back once the lords have fallen.
        let playlist = match which {
            Music::Silence => None,
            Music::Menu => first_folder(&["menu", "graveyard"]),
            Music::Vigil(f, _) => first_folder(&[dir(f)]),
            Music::Lullaby => first_folder(&["lullaby"]),
            Music::Rush => first_folder(&["rush"]),
            Music::Endless(f, _) => first_folder(&["endless", dir(f)]),
            // The playlist stays as it was.
            Music::Boss(..) => Some(String::new()),
        };
        if playlist.as_deref() != Some("") {
            self.songs.set_playlist(playlist.as_deref());
        }
        // The lord's own songs, else the field's, else the shared ones. They are decoded during
        // the lead before the lord arrives, so nothing is kept ready in between.
        let lords = match which {
            Music::Boss(f, i) => {
                let kind = crate::world::ALL_LORDS.iter().copied().find(|k| *k as usize == i).unwrap_or(crate::world::BossKind::Gravewarden);
                boss_songs(f as usize, kind.key())
            }
            _ => Vec::new(),
        };
        self.songs.prefetch(Vec::new());
        // One of those songs by chance, but not the one heard last if there are more.
        let boss_song = match which {
            Music::Boss(..) if !lords.is_empty() => {
                let fresh: Vec<usize> = lords.iter().copied().filter(|s| lords.len() == 1 || Some(*s) != self.last_boss_song).collect();
                let pick = fresh[macroquad::rand::gen_range(0, fresh.len())];
                self.last_boss_song = Some(pick);
                Some(pick)
            }
            _ => None,
        };
        self.songs.set_boss(boss_song);
        let recorded = playlist.as_deref().is_some_and(|p| !p.is_empty());
        let params = PlaySoundParams { looped: true, volume: self.music_vol };
        match which {
            Music::Boss(_, i) if boss_song.is_none() => {
                // A synthesized theme drowns out the recorded playlist for as long.
                self.songs.duck(true);
                play_sound(&self.bosses[i.min(self.bosses.len() - 1)], PlaySoundParams { looped: true, volume: self.music_vol * self.boss_vol });
            }
            Music::Boss(..) | Music::Silence => {}
            _ if recorded => {}
            Music::Menu => play_sound(&self.acts[0], params),
            Music::Vigil(_, act) => {
                play_sound(&self.acts[(act as usize).min(self.acts.len() - 1)], params);
                play_sound(&self.combat, PlaySoundParams { looped: true, volume: self.music_vol * self.intensity });
            }
            Music::Lullaby => play_sound(&self.lullaby, params),
            Music::Endless(_, k) => play_sound(&self.endless[(k as usize).min(self.endless.len() - 1)], params),
            Music::Rush => play_sound(&self.endless[RUSH_STAGE.min(self.endless.len() - 1)], params),
        }
    }

    /// Fades and the next recorded song. Call every frame.
    pub fn update_music(&mut self, play_out: bool) {
        self.songs.play_out = play_out;
        let dt = macroquad::time::get_frame_time().min(0.1);
        let vol = if self.muted { 0.0 } else { self.music_vol };
        self.songs.update(vol, self.boss_vol, dt);
        // The secret song is over: the menu's own music comes back.
        if self.songs.once_ended() && !self.muted {
            let cur = self.current;
            self.current = Music::Silence;
            self.music(cur);
        }
    }

    /// The menu's secret: a song of `assets/music/secret` played once, if there is one.
    pub fn play_secret(&mut self) -> bool {
        let songs = crate::songs::folder("secret");
        if songs.is_empty() || self.muted {
            return false;
        }
        let pick = songs[macroquad::rand::gen_range(0, songs.len())];
        // The synthesized loops stop; recorded ones wait underneath.
        for s in self.loops() {
            stop_sound(s);
        }
        self.songs.play_once(pick);
        true
    }

    /// 0 = calm, 1 = surrounded. Fades the combat drums under the vigil songs.
    pub fn set_intensity(&mut self, k: f32) {
        self.intensity = k.clamp(0.0, 1.0);
        if matches!(self.current, Music::Vigil(..)) && !self.muted {
            set_sound_volume(&self.combat, self.music_vol * self.intensity);
        }
    }

    pub fn set_music_volume(&mut self, v: f32) {
        self.music_vol = v;
        for s in self.loops() {
            set_sound_volume(s, v);
        }
        for s in &self.bosses {
            set_sound_volume(s, v * self.boss_vol);
        }
        set_sound_volume(&self.combat, v * self.intensity);
    }

    /// The lords' music as a share of the music volume.
    pub fn set_boss_volume(&mut self, v: f32) {
        self.boss_vol = v;
        let music = self.music_vol;
        self.set_music_volume(music);
    }

    pub fn toggle_mute(&mut self) {
        self.muted = !self.muted;
        let cur = self.current;
        if self.muted {
            self.stop_music();
            self.songs.silence();
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
    let mut names: Vec<String> = ["dusk", "midnight", "witching", "lullaby", "combat"].iter().map(|s| s.to_string()).collect();
    names.extend((1..=THEMES.len()).map(|i| format!("boss {}", i)));
    names.extend((1..=ENDLESS_STAGES).map(|i| format!("endless {}", i)));
    names.into_iter().enumerate().map(|(i, n)| (&*Box::leak(n.into_boxed_str()), rough(&music_track(i)))).collect()
}
