//! Recorded songs: playlists per field that give way to boss songs and come back after.
//!
//! Songs live in folders under `assets/music`, any file name, OGG Vorbis or WAV:
//! `graveyard/`, `cathedral/`, `flesh/` for each field's playlist, `<field>/boss/` for that
//! field's lords, `boss/` for every lord without a song of its own field, and the optional
//! `menu/`, `endless/` and `lullaby/`. Songs found there at build time are embedded in the
//! game; files on disk at run time win over them. A folder without songs keeps the
//! synthesized music.
//!
//! Songs are decoded on threads to 44.1 kHz stereo WAV, so loading never stalls a frame.

use macroquad::audio::{load_sound_from_bytes, play_sound, set_sound_volume, stop_sound, PlaySoundParams, Sound};
use std::collections::{BTreeMap, HashMap};
use std::sync::OnceLock;
use std::thread::JoinHandle;

include!(concat!(env!("OUT_DIR"), "/music_files.rs"));

pub const FIELD_DIRS: [&str; 3] = ["graveyard", "cathedral", "flesh"];

/// One recorded song: where it was found and its bytes.
pub struct SongFile {
    /// Path below `assets/music`, with forward slashes, e.g. "graveyard/rain.ogg".
    pub path: String,
    pub bytes: Vec<u8>,
}

/// Every song, embedded and on disk, sorted by path.
pub fn library() -> &'static [SongFile] {
    static LIB: OnceLock<Vec<SongFile>> = OnceLock::new();
    LIB.get_or_init(|| {
        let mut all: BTreeMap<String, Vec<u8>> = MUSIC_FILES.iter().map(|(p, b)| (p.to_string(), b.to_vec())).collect();
        if cfg!(feature = "songs") {
            scan_disk(std::path::Path::new("assets/music"), "", &mut all);
        }
        let lib: Vec<SongFile> = all.into_iter().map(|(path, bytes)| SongFile { path, bytes }).collect();
        if !lib.is_empty() {
            crate::log!("{} recorded songs: {}", lib.len(), lib.iter().map(|s| s.path.as_str()).collect::<Vec<_>>().join(", "));
        }
        lib
    })
}

fn scan_disk(dir: &std::path::Path, prefix: &str, out: &mut BTreeMap<String, Vec<u8>>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let name = e.file_name().to_string_lossy().to_string();
        let path = e.path();
        if path.is_dir() {
            scan_disk(&path, &format!("{}{}/", prefix, name), out);
        } else if prefix.is_empty() {
            // Loose files replace single synthesized loops (see `audio::load_override`).
        } else if is_song(&name) {
            if let Ok(bytes) = std::fs::read(&path) {
                out.insert(format!("{}{}", prefix, name), bytes);
            }
        }
    }
}

pub fn is_song(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.ends_with(".ogg") || n.ends_with(".wav")
}

/// The songs directly inside a folder, e.g. "graveyard" or "graveyard/boss", by index into `library()`.
pub fn folder(dir: &str) -> Vec<usize> {
    let pre = format!("{}/", dir);
    library().iter().enumerate().filter(|(_, s)| s.path.strip_prefix(&pre).is_some_and(|rest| !rest.contains('/'))).map(|(i, _)| i).collect()
}

/// A decoded song, ready for `load_sound_from_bytes`: raw samples the patched quad-snd only copies.
pub struct Decoded {
    pub wav: Vec<u8>,
    pub secs: f32,
}

/// Decodes a song of the library to 44.1 kHz stereo f32 samples behind quad-snd's raw magic.
pub fn decode(i: usize) -> Result<Decoded, String> {
    let song = library().get(i).ok_or("no such song")?;
    let (rate, channels, samples) = if song.path.to_ascii_lowercase().ends_with(".wav") { read_wav(&song.bytes)? } else { read_ogg(&song.bytes)? };
    let stereo: Vec<f32> = match channels {
        1 => samples.iter().flat_map(|s| [*s, *s]).collect(),
        2 => samples,
        n => samples.chunks(n).flat_map(|c| [c[0], c[1]]).collect(),
    };
    let out = resample(&stereo, rate);
    let secs = out.len() as f32 / 2.0 / 44100.0;
    Ok(Decoded { wav: raw_bytes(&out), secs })
}

fn read_ogg(bytes: &[u8]) -> Result<(u32, usize, Vec<f32>), String> {
    let mut r = lewton::inside_ogg::OggStreamReader::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let rate = r.ident_hdr.audio_sample_rate;
    let channels = r.ident_hdr.audio_channels as usize;
    let mut out = Vec::new();
    while let Some(pck) = r.read_dec_packet_itl().map_err(|e| e.to_string())? {
        out.extend(pck.iter().map(|s| *s as f32 / 32768.0));
    }
    Ok((rate, channels, out))
}

fn read_wav(bytes: &[u8]) -> Result<(u32, usize, Vec<f32>), String> {
    let mut r = hound::WavReader::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let spec = r.spec();
    let out: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => r.samples::<f32>().filter_map(Result::ok).collect(),
        hound::SampleFormat::Int => {
            let scale = 1.0 / (1u64 << (spec.bits_per_sample - 1)) as f32;
            r.samples::<i32>().filter_map(Result::ok).map(|s| s as f32 * scale).collect()
        }
    };
    Ok((spec.sample_rate, spec.channels as usize, out))
}

/// Linear resampling of interleaved stereo to 44.1 kHz (the mixer's own is nearest-neighbour).
fn resample(stereo: &[f32], rate: u32) -> Vec<f32> {
    if rate == 44100 || rate == 0 {
        return stereo.to_vec();
    }
    let frames = stereo.len() / 2;
    let step = rate as f64 / 44100.0;
    let n = (frames as f64 / step) as usize;
    let mut out = Vec::with_capacity(n * 2);
    for j in 0..n {
        let x = j as f64 * step;
        let i = x as usize;
        let f = (x - i as f64) as f32;
        let i1 = (i + 1).min(frames - 1);
        for c in 0..2 {
            let a = stereo[i * 2 + c];
            let b = stereo[i1 * 2 + c];
            out.push(a + (b - a) * f);
        }
    }
    out
}

/// Samples behind the magic the patched quad-snd copies without parsing (see vendor/quad-snd).
fn raw_bytes(stereo: &[f32]) -> Vec<u8> {
    let mut out = Vec::with_capacity(8 + stereo.len() * 4);
    out.extend_from_slice(b"AVF32RAW");
    for s in stereo {
        out.extend_from_slice(&s.clamp(-1.0, 1.0).to_le_bytes());
    }
    out
}

/// Seconds for the field's songs to fall silent when a lord comes, and to come back after.
const MAP_OUT: f32 = 2.0;
const MAP_IN: f32 = 4.0;
/// Seconds for a boss song to rise, and to fade once the last lord has fallen.
const BOSS_IN: f32 = 1.0;
const BOSS_OUT: f32 = 3.0;
/// Seconds for a song to fade when another playlist takes its place.
const SWAP_OUT: f32 = 2.0;

/// Polls a future once: macroquad's loading is ready at once on the desktop.
fn now_or_never<F: std::future::Future>(f: F) -> Option<F::Output> {
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    let mut f = std::pin::pin!(f);
    match f.as_mut().poll(&mut cx) {
        std::task::Poll::Ready(v) => Some(v),
        std::task::Poll::Pending => None,
    }
}

struct Track {
    song: usize,
    sound: Sound,
    gain: f32,
    /// When the song ends, by `get_time`; looping songs never do.
    ends: f64,
    /// When it began and how long one pass takes, to let a boss song play to its end.
    started: f64,
    secs: f32,
    /// A lord's song, which follows the boss music volume.
    boss: bool,
}

struct Playlist {
    dir: String,
    order: Vec<usize>,
    pos: usize,
    now: Option<Track>,
    gain: f32,
    target: f32,
}

impl Playlist {
    fn next_song(&self) -> usize {
        self.order[self.pos % self.order.len()]
    }
}

/// Plays the recorded songs: the field's playlist, a boss song over it, and the fading.
#[derive(Default)]
pub struct Player {
    map: Option<Playlist>,
    /// The boss song wanted, and its track once it plays.
    boss_want: Option<usize>,
    boss: Option<Track>,
    /// Songs fading out, with the seconds their fade takes.
    leaving: Vec<(Track, f32)>,
    cache: HashMap<usize, (Sound, f32)>,
    jobs: HashMap<usize, JoinHandle<Result<Decoded, String>>>,
    /// Songs worth having decoded before they are needed: the lords' songs of this field.
    keep: Vec<usize>,
    /// Once the lords have fallen, their song plays to its end instead of fading.
    pub play_out: bool,
    /// A song played once over everything (the menu's secret), and its track once it plays.
    once_want: Option<usize>,
    once: Option<Track>,
    /// The song played once has ended: the music that was there comes back.
    once_over: bool,
}

fn shuffled(mut v: Vec<usize>) -> Vec<usize> {
    for i in (1..v.len()).rev() {
        let j = macroquad::rand::gen_range(0, i + 1);
        v.swap(i, j);
    }
    v
}

impl Player {
    /// The playlist to play from a folder; `None` plays none. Keeps playing if unchanged.
    pub fn set_playlist(&mut self, dir: Option<&str>) {
        if self.map.as_ref().map(|m| m.dir.as_str()) == dir {
            return;
        }
        if let Some(mut old) = self.map.take() {
            if let Some(mut t) = old.now.take() {
                t.gain *= old.gain;
                self.leaving.push((t, SWAP_OUT));
            }
        }
        let Some(dir) = dir else { return };
        let songs = folder(dir);
        if songs.is_empty() {
            return;
        }
        self.map = Some(Playlist { dir: dir.to_string(), order: shuffled(songs), pos: 0, now: None, gain: 0.0, target: 1.0 });
    }

    /// A lord's song: the playlist steps aside for it, and comes back once it is `None`.
    pub fn set_boss(&mut self, song: Option<usize>) {
        if self.boss_want == song {
            if let Some(m) = self.map.as_mut() {
                m.target = if song.is_some() || self.boss.is_some() { 0.0 } else { 1.0 };
            }
            return;
        }
        self.boss_want = song;
        if let Some(mut t) = self.boss.take() {
            if Some(t.song) == song {
                // The same song again (a lord came while it played out): it loops on.
                t.ends = f64::MAX;
                self.boss = Some(t);
            } else if song.is_none() && self.play_out && t.secs > 0.0 {
                // Plays to the end of the pass it is in; the field's song waits for it.
                let now = macroquad::time::get_time();
                let pass = t.secs as f64;
                let passes = ((now - t.started) / pass).floor() + 1.0;
                t.ends = t.started + passes * pass;
                crate::log!("boss song plays out for {:.0} s", t.ends - now);
                self.boss = Some(t);
            } else {
                self.leaving.push((t, BOSS_OUT));
            }
        }
        if let Some(m) = self.map.as_mut() {
            m.target = if song.is_some() || self.boss.is_some() { 0.0 } else { 1.0 };
        }
    }

    /// The playlist is drowned out (a synthesized boss theme plays over it) or comes back.
    pub fn duck(&mut self, quiet: bool) {
        if let Some(m) = self.map.as_mut() {
            m.target = if quiet { 0.0 } else { 1.0 };
        }
    }

    /// Plays a song once over the playlist, which steps aside for it.
    pub fn play_once(&mut self, song: usize) {
        if let Some(t) = self.once.take() {
            stop_sound(&t.sound);
        }
        self.once_want = Some(song);
        self.duck(true);
    }

    /// The song played once has just ended (once per ending).
    pub fn once_ended(&mut self) -> bool {
        std::mem::take(&mut self.once_over)
    }

    /// Ends the song played once early, fading it.
    pub fn cancel_once(&mut self) {
        self.once_want = None;
        if let Some(t) = self.once.take() {
            self.leaving.push((t, SWAP_OUT));
        }
    }

    /// Songs to decode ahead of time.
    pub fn prefetch(&mut self, songs: Vec<usize>) {
        self.keep = songs;
    }

    /// Stops everything at once (mute).
    pub fn silence(&mut self) {
        if let Some(t) = self.map.take().and_then(|mut m| m.now.take()) {
            stop_sound(&t.sound);
        }
        if let Some(t) = self.boss.take() {
            stop_sound(&t.sound);
        }
        self.boss_want = None;
        self.once_want = None;
        if let Some(t) = self.once.take() {
            stop_sound(&t.sound);
        }
        for (t, _) in self.leaving.drain(..) {
            stop_sound(&t.sound);
        }
    }

    fn start_decoding(&mut self, song: usize) {
        if !self.cache.contains_key(&song) {
            self.jobs.entry(song).or_insert_with(|| std::thread::spawn(move || decode(song)));
        }
    }

    /// Fades, the next song of the playlist, and decoding. Call every frame.
    /// `boss_vol` is the share of `vol` the lords' songs get.
    pub fn update(&mut self, vol: f32, boss_vol: f32, dt: f32) {
        let now = macroquad::time::get_time();
        // Decoded songs become sounds.
        let done: Vec<usize> = self.jobs.iter().filter(|(_, j)| j.is_finished()).map(|(i, _)| *i).collect();
        for i in done {
            let Some(job) = self.jobs.remove(&i) else { continue };
            let path = &library()[i].path;
            match job.join() {
                Ok(Ok(d)) => match now_or_never(load_sound_from_bytes(&d.wav)) {
                    Some(Ok(s)) => {
                        self.cache.insert(i, (s, d.secs));
                    }
                    _ => crate::log!("song {} could not be loaded", path),
                },
                Ok(Err(e)) => crate::log!("song {} could not be decoded: {}", path, e),
                Err(_) => crate::log!("song {} could not be decoded", path),
            }
        }
        // The playlist: one song after another, on even while a lord's song drowns it out.
        if let Some(mut m) = self.map.take() {
            if m.target > m.gain {
                m.gain = (m.gain + dt / MAP_IN).min(m.target);
            } else {
                m.gain = (m.gain - dt / MAP_OUT).max(m.target);
            }
            let over = m.now.as_ref().is_none_or(|t| now >= t.ends);
            let song = m.next_song();
            if over {
                if let Some((sound, secs)) = self.cache.get(&song).cloned() {
                    if let Some(t) = m.now.take() {
                        stop_sound(&t.sound);
                    }
                    let single = m.order.len() == 1;
                    stop_sound(&sound);
                    play_sound(&sound, PlaySoundParams { looped: single, volume: vol * m.gain });
                    crate::log!("song: {} ({:.0} s)", library()[song].path, secs);
                    m.now = Some(Track { song, sound, gain: 1.0, ends: if single { f64::MAX } else { now + secs as f64 }, started: now, secs, boss: false });
                    m.pos += 1;
                    if m.pos >= m.order.len() && m.order.len() > 1 {
                        // A new round, but never the same song twice in a row.
                        m.order = shuffled(m.order.clone());
                        if m.order[0] == song {
                            m.order.swap(0, 1);
                        }
                        m.pos = 0;
                    }
                }
            }
            if let Some(t) = &m.now {
                set_sound_volume(&t.sound, vol * m.gain * t.gain);
            }
            self.map = Some(m);
        }
        // The boss song.
        if let (Some(song), None) = (self.boss_want, &self.boss) {
            if let Some((sound, secs)) = self.cache.get(&song).cloned() {
                stop_sound(&sound);
                play_sound(&sound, PlaySoundParams { looped: true, volume: 0.0 });
                crate::log!("boss song: {}", library()[song].path);
                self.boss = Some(Track { song, sound, gain: 0.0, ends: f64::MAX, started: now, secs, boss: true });
            }
        }
        if let Some(t) = self.boss.as_mut() {
            t.gain = (t.gain + dt / BOSS_IN).min(1.0);
            set_sound_volume(&t.sound, vol * boss_vol * t.gain);
        }
        // The song played once.
        if let (Some(song), None) = (self.once_want, &self.once) {
            if let Some((sound, secs)) = self.cache.get(&song).cloned() {
                stop_sound(&sound);
                play_sound(&sound, PlaySoundParams { looped: false, volume: vol });
                crate::log!("song once: {}", library()[song].path);
                self.once = Some(Track { song, sound, gain: 1.0, ends: now + secs as f64, started: now, secs, boss: false });
            }
        }
        if let Some(t) = &self.once {
            set_sound_volume(&t.sound, vol);
        }
        if self.once.as_ref().is_some_and(|t| now >= t.ends) {
            self.once = None;
            self.once_want = None;
            self.once_over = true;
            self.duck(false);
        }
        // A boss song that played out has ended: the field's song comes back.
        if self.boss.as_ref().is_some_and(|t| now >= t.ends) {
            if let Some(t) = self.boss.take() {
                stop_sound(&t.sound);
            }
            if let Some(m) = self.map.as_mut() {
                m.target = 1.0;
            }
        }
        // Songs on their way out.
        self.leaving.retain_mut(|(t, secs)| {
            t.gain -= dt / *secs;
            if t.gain <= 0.0 {
                stop_sound(&t.sound);
                false
            } else {
                set_sound_volume(&t.sound, vol * if t.boss { boss_vol } else { 1.0 } * t.gain);
                true
            }
        });
        // Decode ahead, and let go of what is no longer needed.
        let mut want: Vec<usize> = self.keep.clone();
        if let Some(m) = &self.map {
            want.push(m.next_song());
            want.extend(m.now.as_ref().map(|t| t.song));
        }
        want.extend(self.boss_want);
        want.extend(self.boss.as_ref().map(|t| t.song));
        want.extend(self.leaving.iter().map(|(t, _)| t.song));
        want.extend(self.once_want);
        for s in want.clone() {
            self.start_decoding(s);
        }
        self.cache.retain(|k, _| want.contains(k));
    }
}

/// The songs for one lord, e.g. "bride": its own folder "boss/bride", else the field's
/// "<field>/boss", else the shared "boss".
pub fn boss_songs(field: usize, lord: &str) -> Vec<usize> {
    [format!("boss/{}", lord), format!("{}/boss", FIELD_DIRS[field.min(2)]), "boss".to_string()].iter().map(|d| folder(d)).find(|s| !s.is_empty()).unwrap_or_default()
}

/// The first of these folders with songs in it.
pub fn first_folder(dirs: &[&str]) -> Option<String> {
    dirs.iter().find(|d| !folder(d).is_empty()).map(|d| d.to_string())
}
