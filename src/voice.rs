//! The lords' voices. Their lines are recorded once with Kokoro and reshaped here into each
//! lord's voice: a pitch, growl, breath, echo or a choir (`process_from`). The Hound has no
//! words, its growls are synthesized through vowel formants (`speak`, `hound`).
//!
//! All of that runs when the lines are baked (`tests::voice_jobs`, `tests::voice_bake`); the
//! game itself only plays the baked files and needs `line_key` to find them.
#![cfg_attr(not(test), allow(dead_code))]

use std::f32::consts::TAU;

const SR: f32 = 44100.0;

/// How one lord sounds.
#[derive(Clone, Copy)]
pub struct VoiceStyle {
    /// Pitch in Hz at the start of a line.
    pub pitch: f32,
    /// Formant scale: below 1 a larger throat, above 1 a smaller one.
    pub size: f32,
    /// Rough amplitude flutter and clipping.
    pub growl: f32,
    /// Breath noise mixed into the voice.
    pub breath: f32,
    /// Echo feedback, 0 for none.
    pub echo: f32,
    /// Extra voices around the lead (a choir).
    pub choir: bool,
    /// A low voice an octave below, for the Sovereign.
    pub sub: bool,
    /// Fast vibrato in Hz (moths flutter), 0 for a steady voice.
    pub flutter: f32,
    /// Lowpass in Hz: below 2000 sounds like under water or far away.
    pub muffle: f32,
    /// Seconds per vowel.
    pub pace: f32,
}

/// A two-pole band-pass filter (RBJ biquad), for one formant.
struct Band {
    b0: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    x1: f32,
    x2: f32,
    y1: f32,
    y2: f32,
}

impl Band {
    fn new(freq: f32, q: f32) -> Band {
        let mut b = Band { b0: 0.0, b2: 0.0, a1: 0.0, a2: 0.0, x1: 0.0, x2: 0.0, y1: 0.0, y2: 0.0 };
        b.tune(freq, q);
        b
    }

    fn tune(&mut self, freq: f32, q: f32) {
        let w = TAU * freq.clamp(40.0, SR * 0.45) / SR;
        let alpha = w.sin() / (2.0 * q);
        let a0 = 1.0 + alpha;
        self.b0 = alpha / a0;
        self.b2 = -alpha / a0;
        self.a1 = -2.0 * w.cos() / a0;
        self.a2 = (1.0 - alpha) / a0;
    }

    fn run(&mut self, x: f32) -> f32 {
        let y = self.b0 * x + self.b2 * self.x2 - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = y;
        y
    }
}

/// One piece of a line: a vowel, a voiced or unvoiced consonant, or a pause.
#[derive(Clone, Copy)]
enum Seg {
    Vowel([f32; 3]),
    Voiced([f32; 3]),
    Hiss(f32),
    Pop,
    Pause(f32),
}

fn segments(text: &str) -> Vec<Seg> {
    let mut out = Vec::new();
    for c in text.to_ascii_lowercase().chars() {
        let seg = match c {
            'a' => Seg::Vowel([730.0, 1090.0, 2440.0]),
            'e' => Seg::Vowel([530.0, 1840.0, 2480.0]),
            'i' | 'y' => Seg::Vowel([300.0, 2200.0, 2950.0]),
            'o' => Seg::Vowel([570.0, 840.0, 2410.0]),
            'u' => Seg::Vowel([320.0, 870.0, 2240.0]),
            'm' | 'n' => Seg::Voiced([280.0, 1100.0, 2400.0]),
            'l' | 'r' | 'w' => Seg::Voiced([360.0, 1000.0, 2200.0]),
            'v' | 'z' | 'j' => Seg::Voiced([300.0, 1500.0, 2600.0]),
            's' | 'c' | 'x' => Seg::Hiss(6000.0),
            'f' | 'h' => Seg::Hiss(2500.0),
            't' | 'p' | 'k' | 'q' | 'b' | 'd' | 'g' => Seg::Pop,
            ' ' => Seg::Pause(0.05),
            ',' => Seg::Pause(0.16),
            '.' | '!' | '?' => Seg::Pause(0.24),
            _ => continue,
        };
        out.push(seg);
    }
    out
}

/// A tiny noise source.
struct Noise(u32);

impl Noise {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 as f32 / u32::MAX as f32) * 2.0 - 1.0
    }
}

/// Renders a line in a style. A rising question, a lifted exclamation, a falling statement.
pub fn speak(text: &str, st: VoiceStyle) -> Vec<f32> {
    let segs = segments(text);
    let shout = text.contains('!');
    let ask = text.trim_end().ends_with('?');
    let total: f32 = segs
        .iter()
        .map(|s| match s {
            Seg::Vowel(_) => st.pace,
            Seg::Voiced(_) => st.pace * 0.6,
            Seg::Hiss(_) => st.pace * 0.7,
            Seg::Pop => st.pace * 0.35,
            Seg::Pause(p) => *p,
        })
        .sum();
    let mut out = Vec::with_capacity(((total + 1.5) * SR) as usize);
    let mut bands = [Band::new(500.0, 6.0), Band::new(1500.0, 8.0), Band::new(2500.0, 10.0)];
    let mut hiss = Band::new(4000.0, 2.0);
    let mut noise = Noise(0x1234_5678 ^ text.len() as u32);
    let (mut phase, mut sub_phase, mut c1, mut c2) = (0.0f32, 0.0f32, 0.0f32, 0.0f32);
    let mut lp = 0.0f32;
    let mut t_line = 0.0f32;
    let mut formants = [500.0f32, 1500.0, 2500.0];
    for seg in segs {
        let (len, voiced, target, noisy) = match seg {
            Seg::Vowel(f) => (st.pace, 1.0, Some(f), 0.0),
            Seg::Voiced(f) => (st.pace * 0.6, 0.7, Some(f), 0.0),
            Seg::Hiss(f) => {
                hiss.tune(f * st.size, 2.0);
                (st.pace * 0.7, 0.0, None, 0.5)
            }
            Seg::Pop => (st.pace * 0.35, 0.2, None, 1.0),
            Seg::Pause(p) => (p, 0.0, None, 0.0),
        };
        let n = (len * SR) as usize;
        for i in 0..n {
            let k = i as f32 / n as f32;
            // Formants glide toward the sound, so the voice slurs like speech.
            if let Some(f) = target {
                for (cur, want) in formants.iter_mut().zip(f) {
                    *cur += (want * st.size - *cur) * 0.002;
                }
            }
            for (b, (f, q)) in bands.iter_mut().zip(formants.iter().zip([6.0, 8.0, 10.0])) {
                if i % 64 == 0 {
                    b.tune(*f, q);
                }
            }
            let progress = t_line / total.max(0.1);
            let contour = if ask && progress > 0.8 {
                1.0 + (progress - 0.8) * 2.5
            } else if shout {
                1.15 - progress * 0.1
            } else {
                1.0 - progress * 0.18
            };
            let vib = 1.0 + 0.012 * (TAU * 5.0 * t_line).sin() + if st.flutter > 0.0 { 0.05 * (TAU * st.flutter * t_line).sin() } else { 0.0 };
            let f0 = st.pitch * contour * vib;
            phase = (phase + f0 / SR).fract();
            // A buzz rich in harmonics, a little softened.
            let mut src = (phase * 2.0 - 1.0) * 0.6 + (TAU * phase).sin() * 0.4;
            if st.sub {
                sub_phase = (sub_phase + f0 * 0.5 / SR).fract();
                src += (sub_phase * 2.0 - 1.0) * 0.5;
            }
            if st.choir {
                c1 = (c1 + f0 * 1.26 / SR).fract();
                c2 = (c2 + f0 * 1.5 / SR).fract();
                src += (c1 * 2.0 - 1.0) * 0.45 + (c2 * 2.0 - 1.0) * 0.35;
            }
            let breath = noise.next();
            let env = (k * 12.0).min(1.0) * ((1.0 - k) * 10.0).min(1.0);
            let voice_in = (src * (1.0 - st.breath) + breath * st.breath) * voiced * env;
            let mut v = bands[0].run(voice_in) * 1.0 + bands[1].run(voice_in) * 0.7 + bands[2].run(voice_in) * 0.35;
            if noisy > 0.0 {
                let burst = if matches!(seg, Seg::Pop) { (-(k * 25.0)).exp() } else { env };
                v += hiss.run(breath) * noisy * burst * 0.8;
            }
            if st.growl > 0.0 {
                let rough = 1.0 - st.growl * 0.5 * (1.0 + (TAU * 33.0 * t_line).sin() * (TAU * 7.0 * t_line).sin());
                v = (v * rough * (1.0 + st.growl * 2.0)).tanh();
            }
            lp += (v - lp) * (1.0 - (-TAU * st.muffle / SR).exp());
            out.push(lp);
            t_line += 1.0 / SR;
        }
    }
    // A little air at the end, then the echo of the hall.
    out.extend(std::iter::repeat_n(0.0, (0.25 * SR) as usize));
    if st.echo > 0.0 {
        let delays = [(0.11, 0.6), (0.19, 0.45), (0.31, 0.3)];
        let tail = (0.9 * SR) as usize;
        out.extend(std::iter::repeat_n(0.0, tail));
        let dry = out.clone();
        for (d, g) in delays {
            let off = (d * SR) as usize;
            for i in off..out.len() {
                out[i] += dry[i - off] * g * st.echo + out[i - off] * 0.25 * st.echo * g;
            }
        }
    }
    // Normalize and soften the ends.
    let peak = out.iter().fold(0.0f32, |m, s| m.max(s.abs())).max(1e-4);
    let fade = (0.01 * SR) as usize;
    let len = out.len();
    for (i, s) in out.iter_mut().enumerate() {
        let edge = (i.min(len - 1 - i) as f32 / fade as f32).min(1.0);
        *s = *s / peak * 0.9 * edge;
    }
    out
}

/// What a hound says, read from its line: sniffing and a growl, a howl, or a last whimper.
pub fn hound(text: &str) -> Vec<f32> {
    let lower = text.to_ascii_lowercase();
    let mut h = Hound { out: Vec::new(), noise: Noise(0xBEEF_1234) };
    if lower.contains("sniff") {
        h.sniffs(4);
        h.rest(0.25);
        h.sniffs(3);
        h.rest(0.2);
    }
    if lower.contains("howl") {
        h.growl(0.6, 0.5);
        h.howl(2.6);
    }
    if lower.contains("whimper") {
        h.whimper();
    }
    if lower.contains("fresh") || h.out.is_empty() {
        h.growl(1.6, 1.0);
    }
    let mut out = h.out;
    // A little room around the beast.
    let dry = out.clone();
    out.extend(std::iter::repeat_n(0.0, (0.5 * SR) as usize));
    for (d, g) in [(0.07, 0.22), (0.16, 0.12)] {
        let off = (d * SR) as usize;
        for (i, x) in dry.iter().enumerate() {
            out[i + off] += x * g;
        }
    }
    let peak = out.iter().fold(0.0f32, |m, s| m.max(s.abs())).max(1e-4);
    out.iter_mut().for_each(|s| *s = *s / peak * 0.9);
    out
}

/// The hound's throat: noise and pulses shaped by a large animal's mouth.
struct Hound {
    out: Vec<f32>,
    noise: Noise,
}

impl Hound {
    fn rest(&mut self, secs: f32) {
        self.out.extend(std::iter::repeat_n(0.0, (secs * SR) as usize));
    }

    /// Quick sharp breaths in through the nose, each a hiss with a hollow ring.
    fn sniffs(&mut self, n: usize) {
        let mut hiss = Band::new(3200.0, 1.4);
        let mut nose = Band::new(1100.0, 4.0);
        for i in 0..n {
            let len = 0.075 + 0.02 * (i % 2) as f32;
            let m = (len * SR) as usize;
            for j in 0..m {
                let k = j as f32 / m as f32;
                let env = (k * 12.0).min(1.0) * (1.0 - k).powf(1.5);
                let x = self.noise.next();
                self.out.push((hiss.run(x) * 0.8 + nose.run(x) * 0.5) * env);
            }
            self.rest(0.045);
        }
    }

    /// A deep, wet growl: uneven throat pulses through a big mouth, swelling and rattling.
    /// `swell` is how loud it gets (0..1).
    fn growl(&mut self, secs: f32, swell: f32) {
        let mut f1 = Band::new(320.0, 3.0);
        let mut f2 = Band::new(850.0, 4.0);
        let mut f3 = Band::new(2300.0, 5.0);
        let n = (secs * SR) as usize;
        let mut phase = 0.0f32;
        let mut pulse = 0.0f32;
        let mut odd = false;
        for i in 0..n {
            let k = i as f32 / n as f32;
            let t = i as f32 / SR;
            // Around 60 Hz, never steady.
            let f0 = 58.0 + 10.0 * (TAU * 1.3 * t).sin() + 6.0 * self.noise.next();
            phase += f0 / SR;
            if phase >= 1.0 {
                phase -= 1.0;
                // Every other pulse weaker: the rattle of a growl.
                odd = !odd;
                pulse = if odd { 1.0 } else { 0.55 };
            }
            // Each pulse rings out over a few milliseconds.
            pulse *= 0.994;
            let src = self.noise.next() * pulse * 0.8 + pulse * 0.6;
            let v = f1.run(src) * 1.0 + f2.run(src) * 0.6 + f3.run(src) * 0.25;
            let env = (k * 5.0).min(1.0) * ((1.0 - k) * 4.0).min(1.0) * (0.75 + 0.25 * (TAU * 2.1 * t).sin()) * swell;
            self.out.push((v * 3.0).tanh() * env);
        }
    }

    /// A long howl: the voice climbs, holds with a waver and sinks, the mouth opening from
    /// "oo" to "ah" and closing again, with breath in it.
    fn howl(&mut self, secs: f32) {
        let mut f1 = Band::new(400.0, 5.0);
        let mut f2 = Band::new(900.0, 6.0);
        let mut f3 = Band::new(2600.0, 8.0);
        let mut breath = Band::new(1500.0, 1.0);
        let n = (secs * SR) as usize;
        let mut phase = 0.0f32;
        for i in 0..n {
            let k = i as f32 / n as f32;
            let t = i as f32 / SR;
            let rise = (k / 0.22).min(1.0);
            let fall = ((1.0 - k) / 0.3).min(1.0);
            let f0 = (260.0 + 270.0 * rise.powf(0.6)) * (0.72 + 0.28 * fall) * (1.0 + 0.025 * (TAU * 5.2 * t).sin());
            phase = (phase + f0 / SR).fract();
            // A soft buzz: rich enough for the mouth to shape.
            let src = (phase * 2.0 - 1.0) * 0.5 + (TAU * phase).sin() * 0.5;
            let open = (k * std::f32::consts::PI).sin();
            if i % 64 == 0 {
                f1.tune(350.0 + 400.0 * open, 5.0);
                f2.tune(800.0 + 450.0 * open, 6.0);
            }
            let v = f1.run(src) + f2.run(src) * 0.5 + f3.run(src) * 0.15 + breath.run(self.noise.next()) * 0.12;
            let env = (k * 6.0).min(1.0) * ((1.0 - k) * 3.0).min(1.0);
            self.out.push(v * env * 1.4);
        }
    }

    /// Three thin, falling whines through the nose, then a last breath out.
    fn whimper(&mut self) {
        for (len, from, to, gain) in [(0.32, 980.0, 720.0, 1.0), (0.26, 900.0, 640.0, 0.8), (0.55, 820.0, 430.0, 0.65)] {
            let mut nasal = Band::new(2100.0, 6.0);
            let mut low = Band::new(320.0, 3.0);
            let n = (len * SR) as usize;
            let mut phase = 0.0f32;
            for i in 0..n {
                let k = i as f32 / n as f32;
                let t = i as f32 / SR;
                let f0 = from + (to - from) * k.powf(0.7) + 15.0 * (TAU * 9.0 * t).sin();
                phase = (phase + f0 / SR).fract();
                let src = (TAU * phase).sin() + 0.3 * (phase * 2.0 - 1.0) + 0.08 * self.noise.next();
                let v = src * 0.5 + nasal.run(src) * 0.6 + low.run(src) * 0.3;
                let env = (k * 14.0).min(1.0) * (1.0 - k).powf(0.8);
                self.out.push(v * env * gain * 0.6);
            }
            self.rest(0.12);
        }
        let mut sigh = Band::new(900.0, 1.0);
        let n = (0.7 * SR) as usize;
        for i in 0..n {
            let k = i as f32 / n as f32;
            let v = sigh.run(self.noise.next()) * (k * 6.0).min(1.0) * (1.0 - k) * 0.5;
            self.out.push(v);
        }
    }
}

use crate::world::BossKind;

/// Each lord's voice.
pub fn style(kind: BossKind) -> VoiceStyle {
    let base = VoiceStyle { pitch: 100.0, size: 1.0, growl: 0.0, breath: 0.1, echo: 0.0, choir: false, sub: false, flutter: 0.0, muffle: 6000.0, pace: 0.085 };
    match kind {
        BossKind::Gravewarden => VoiceStyle { pitch: 72.0, size: 0.8, growl: 0.5, echo: 0.3, pace: 0.095, ..base },
        BossKind::Matriarch => VoiceStyle { pitch: 190.0, size: 1.15, breath: 0.45, flutter: 9.0, ..base },
        BossKind::Hermit => VoiceStyle { pitch: 95.0, size: 0.9, breath: 0.25, echo: 0.6, pace: 0.11, ..base },
        BossKind::Sovereign => VoiceStyle { pitch: 62.0, size: 0.75, growl: 0.65, sub: true, echo: 0.35, ..base },
        BossKind::Bride => VoiceStyle { pitch: 215.0, size: 1.15, breath: 0.55, echo: 0.5, muffle: 1800.0, pace: 0.1, ..base },
        BossKind::BogMother => VoiceStyle { pitch: 58.0, size: 0.68, growl: 0.8, breath: 0.2, muffle: 2500.0, ..base },
        BossKind::Penitent => VoiceStyle { pitch: 82.0, size: 0.85, growl: 0.35, echo: 0.4, ..base },
        BossKind::Choir => VoiceStyle { pitch: 100.0, size: 1.0, choir: true, echo: 0.5, pace: 0.1, ..base },
        BossKind::Seraph => VoiceStyle { pitch: 105.0, size: 0.9, growl: 0.2, echo: 0.45, breath: 0.2, ..base },
        BossKind::Hound => VoiceStyle { pitch: 90.0, growl: 0.7, ..base },
    }
}

/// The Kokoro voice (a name, or a blend like "af_kore*0.5+af_nicole*0.5") and speed a
/// lord's lines were recorded with (see `tests::voice_jobs`);
/// None for the Hound, whose growls are synthesized.
pub fn recorded_with(kind: BossKind) -> Option<(&'static str, f32)> {
    Some(match kind {
        BossKind::Gravewarden => ("am_fenrir", 0.9),
        BossKind::Matriarch => ("af_kore*0.5+af_nicole*0.5", 0.9),
        BossKind::Hermit => ("bm_george", 0.9),
        BossKind::Sovereign => ("bm_lewis", 0.88),
        BossKind::Bride => ("af_nicole", 0.92),
        BossKind::BogMother => ("bf_emma*0.6+am_onyx*0.4", 0.9),
        BossKind::Penitent => ("am_adam", 0.9),
        BossKind::Choir => ("bm_lewis", 0.85),
        BossKind::Seraph => ("am_michael", 0.9),
        BossKind::Hound => return None,
    })
}

/// A line as it is handed to Kokoro: the same words, with `[pause]` where the lord should
/// fall silent for a moment longer than a comma or dots give.
pub fn spoken(line: &str) -> String {
    line.replace("The tower fell... ", "The tower fell. [pause] ")
}

/// How far the recording voice speaks, for shifting it toward the lord's pitch.
pub fn recorded_pitch(voice: &str) -> f32 {
    if voice.starts_with("am_") || voice.starts_with("bm_") {
        115.0
    } else {
        200.0
    }
}

/// A lord's name in file names.
pub fn key(kind: BossKind) -> &'static str {
    match kind {
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
    }
}

/// The sound for a line a lord speaks, e.g. "bride_1", if it is one of theirs.
pub fn line_key(text: &str) -> Option<String> {
    crate::world::ALL_LORDS.iter().find_map(|&k| k.lines().iter().position(|l| !l.is_empty() && *l == text).map(|i| format!("{}_{}", key(k), i)))
}

/// Cuts the silence before and after a line, with a short fade at the end.
pub fn trim(s: &[f32]) -> Vec<f32> {
    let loud = |x: &f32| x.abs() > 0.01;
    let (Some(a), Some(b)) = (s.iter().position(loud), s.iter().rposition(loud)) else { return Vec::new() };
    let a = a.saturating_sub((0.02 * SR) as usize);
    let b = (b + (0.05 * SR) as usize).min(s.len());
    let mut out = s[a..b].to_vec();
    let fade = out.len().min((0.08 * SR) as usize);
    let n = out.len();
    for (i, x) in out[n - fade..].iter_mut().enumerate() {
        *x *= 1.0 - i as f32 / fade as f32;
    }
    out
}

/// A lord's line as the game says it.
pub fn line_audio(kind: BossKind, text: &str) -> Vec<f32> {
    if kind == BossKind::Hound {
        hound(text)
    } else {
        speak(text, style(kind))
    }
}

/// Reads `samples` at `rate` and plays them back `k` times as fast (k < 1 is deeper and slower).
fn resample(s: &[f32], k: f32) -> Vec<f32> {
    let n = (s.len() as f32 / k) as usize;
    (0..n)
        .map(|i| {
            let x = i as f32 * k;
            let j = x as usize;
            let f = x - j as f32;
            let a = s.get(j).copied().unwrap_or(0.0);
            let b = s.get(j + 1).copied().unwrap_or(0.0);
            a + (b - a) * f
        })
        .collect()
}

/// Changes the pitch by `ratio` but keeps the length, by resampling and then stretching the
/// result back with overlapping windows.
fn pitch_shift(s: &[f32], ratio: f32) -> Vec<f32> {
    let y = resample(s, ratio);
    let n = 1764usize;
    let hop = n / 4;
    let win: Vec<f32> = (0..n).map(|i| 0.5 - 0.5 * (TAU * i as f32 / n as f32).cos()).collect();
    let mut out = vec![0.0f32; s.len() + n];
    let mut norm = vec![0.0f32; s.len() + n];
    let mut t = 0;
    while t < s.len() {
        let src = (t as f32 / ratio) as usize;
        for i in 0..n {
            let v = y.get(src + i).copied().unwrap_or(0.0);
            out[t + i] += v * win[i];
            norm[t + i] += win[i];
        }
        t += hop;
    }
    out.truncate(s.len());
    out.iter_mut().zip(norm).for_each(|(o, w)| *o /= w.max(1e-3));
    out
}

/// A recorded line in a lord's voice, for a recording whose speaker talks at about
/// `src_pitch` Hz: deeper or higher, with its growl, choir, muffle and echo.
pub fn process_from(recorded: &[f32], kind: BossKind, src_pitch: f32) -> Vec<f32> {
    let st = style(kind);
    // Shift toward the lord's pitch, but not so far that the words drag; deep lords get
    // their depth from an octave below as well.
    let k = (st.pitch / src_pitch).clamp(0.68, 1.15);
    let mut out = resample(recorded, k);
    if st.pitch < 110.0 && !st.sub {
        let low = resample(recorded, k * 0.5);
        for (o, x) in out.iter_mut().zip(low) {
            *o += x * 0.35;
        }
    }
    let mix = |out: &mut Vec<f32>, other: Vec<f32>, g: f32| {
        for (o, x) in out.iter_mut().zip(other) {
            *o += x * g;
        }
    };
    if st.sub {
        mix(&mut out, resample(recorded, k * 0.5), 0.6);
    }
    if st.choir {
        // Voices a fourth and an octave below that keep the words in time with the lead,
        // a few milliseconds late as a real choir is.
        let lead = out.clone();
        for (ratio, delay, g) in [(0.75, 0.015, 0.4), (0.5, 0.028, 0.35)] {
            let off = (delay * SR) as usize;
            let mut layer = vec![0.0; off];
            layer.extend(pitch_shift(&lead, ratio));
            mix(&mut out, layer, g);
        }
    }
    finish(out, st)
}

/// What every recorded line gets last: growl, flutter, muffle and echo, then the level.
fn finish(mut out: Vec<f32>, st: VoiceStyle) -> Vec<f32> {
    let mut lp = 0.0f32;
    for (i, s) in out.iter_mut().enumerate() {
        let t = i as f32 / SR;
        let mut v = *s;
        if st.growl > 0.0 {
            let rough = 1.0 - st.growl * 0.45 * (1.0 + (TAU * 31.0 * t).sin());
            v = (v * rough * (1.0 + st.growl * 3.0)).tanh();
        }
        if st.flutter > 0.0 {
            v *= 0.8 + 0.2 * (TAU * st.flutter * t).sin();
        }
        lp += (v - lp) * (1.0 - (-TAU * st.muffle / SR).exp());
        *s = lp;
    }
    if st.echo > 0.0 {
        out.extend(std::iter::repeat_n(0.0, (0.9 * SR) as usize));
        let dry = out.clone();
        for (d, g) in [(0.11, 0.55), (0.23, 0.4), (0.37, 0.25)] {
            let off = (d * SR) as usize;
            for i in off..out.len() {
                out[i] += dry[i - off] * g * st.echo;
            }
        }
    }
    let peak = out.iter().fold(0.0f32, |m, s| m.max(s.abs())).max(1e-4);
    out.iter_mut().for_each(|s| *s = *s / peak * 0.9);
    out
}

/// 16-bit mono WAV at 44.1 kHz.
pub fn to_wav(samples: &[f32]) -> Vec<u8> {
    let data_len = (samples.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&44100u32.to_le_bytes());
    out.extend_from_slice(&88200u32.to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        out.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    out
}

/// Reads a 16-bit mono WAV, whatever its rate, as 44.1 kHz samples.
pub fn from_wav(bytes: &[u8]) -> Option<Vec<f32>> {
    let rate = u32::from_le_bytes(bytes.get(24..28)?.try_into().ok()?) as f32;
    let pos = bytes.windows(4).position(|w| w == b"data")? + 8;
    let raw: Vec<f32> = bytes[pos..].as_chunks::<2>().0.iter().map(|c| i16::from_le_bytes(*c) as f32 / 32768.0).collect();
    Some(if (rate - SR).abs() > 1.0 { resample(&raw, rate / SR) } else { raw })
}
