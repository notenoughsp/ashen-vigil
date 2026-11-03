//! Tiny key=value save file next to the executable.

use crate::data::*;
use macroquad::prelude::{KeyCode, Vec2};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Something a key can be bound to. Arrow keys, Shift (roll) and Esc (pause) always work too.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Up,
    Down,
    Left,
    Right,
    Roll,
    AutoAim,
    Reroll,
    Pause,
    Weapons,
    Numbers,
    Mute,
}

pub const ACTIONS: [Action; 11] =
    [Action::Up, Action::Down, Action::Left, Action::Right, Action::Roll, Action::AutoAim, Action::Reroll, Action::Pause, Action::Weapons, Action::Numbers, Action::Mute];

impl Action {
    pub fn label(self) -> &'static str {
        match self {
            Action::Up => "Move up",
            Action::Down => "Move down",
            Action::Left => "Move left",
            Action::Right => "Move right",
            Action::Roll => "Roll",
            Action::AutoAim => "Auto-aim on/off",
            Action::Reroll => "Reroll level-up",
            Action::Pause => "Pause",
            Action::Weapons => "Damage per weapon (hold)",
            Action::Numbers => "Damage numbers on/off",
            Action::Mute => "Mute",
        }
    }

    pub fn default_key(self) -> KeyCode {
        match self {
            Action::Up => KeyCode::W,
            Action::Down => KeyCode::S,
            Action::Left => KeyCode::A,
            Action::Right => KeyCode::D,
            Action::Roll => KeyCode::Space,
            Action::AutoAim => KeyCode::F,
            Action::Reroll => KeyCode::R,
            Action::Pause => KeyCode::P,
            Action::Weapons => KeyCode::Tab,
            Action::Numbers => KeyCode::N,
            Action::Mute => KeyCode::M,
        }
    }

    fn key(self) -> &'static str {
        match self {
            Action::Up => "up",
            Action::Down => "down",
            Action::Left => "left",
            Action::Right => "right",
            Action::Roll => "roll",
            Action::AutoAim => "auto_aim",
            Action::Reroll => "reroll",
            Action::Pause => "pause",
            Action::Weapons => "weapons",
            Action::Numbers => "numbers",
            Action::Mute => "mute",
        }
    }
}

/// Keys that can be bound, with the names shown in menus and stored in the save.
/// Esc, Enter, F3 and F11 have fixed jobs and are left out.
const BINDABLE: &[(KeyCode, &str)] = &[
    (KeyCode::A, "A"),
    (KeyCode::B, "B"),
    (KeyCode::C, "C"),
    (KeyCode::D, "D"),
    (KeyCode::E, "E"),
    (KeyCode::F, "F"),
    (KeyCode::G, "G"),
    (KeyCode::H, "H"),
    (KeyCode::I, "I"),
    (KeyCode::J, "J"),
    (KeyCode::K, "K"),
    (KeyCode::L, "L"),
    (KeyCode::M, "M"),
    (KeyCode::N, "N"),
    (KeyCode::O, "O"),
    (KeyCode::P, "P"),
    (KeyCode::Q, "Q"),
    (KeyCode::R, "R"),
    (KeyCode::S, "S"),
    (KeyCode::T, "T"),
    (KeyCode::U, "U"),
    (KeyCode::V, "V"),
    (KeyCode::W, "W"),
    (KeyCode::X, "X"),
    (KeyCode::Y, "Y"),
    (KeyCode::Z, "Z"),
    (KeyCode::Key0, "0"),
    (KeyCode::Key1, "1"),
    (KeyCode::Key2, "2"),
    (KeyCode::Key3, "3"),
    (KeyCode::Key4, "4"),
    (KeyCode::Key5, "5"),
    (KeyCode::Key6, "6"),
    (KeyCode::Key7, "7"),
    (KeyCode::Key8, "8"),
    (KeyCode::Key9, "9"),
    (KeyCode::Space, "Space"),
    (KeyCode::Tab, "Tab"),
    (KeyCode::LeftShift, "LShift"),
    (KeyCode::RightShift, "RShift"),
    (KeyCode::LeftControl, "LCtrl"),
    (KeyCode::RightControl, "RCtrl"),
    (KeyCode::LeftAlt, "LAlt"),
    (KeyCode::RightAlt, "RAlt"),
    (KeyCode::CapsLock, "Caps"),
    (KeyCode::Up, "Up"),
    (KeyCode::Down, "Down"),
    (KeyCode::Left, "Left"),
    (KeyCode::Right, "Right"),
    (KeyCode::Comma, ","),
    (KeyCode::Period, "."),
    (KeyCode::Slash, "/"),
    (KeyCode::Semicolon, ";"),
    (KeyCode::Apostrophe, "'"),
    (KeyCode::LeftBracket, "["),
    (KeyCode::RightBracket, "]"),
    (KeyCode::Minus, "-"),
    (KeyCode::Equal, "="),
    (KeyCode::GraveAccent, "`"),
    (KeyCode::Backslash, "\\"),
    (KeyCode::Kp0, "Num0"),
    (KeyCode::Kp1, "Num1"),
    (KeyCode::Kp2, "Num2"),
    (KeyCode::Kp3, "Num3"),
    (KeyCode::Kp4, "Num4"),
    (KeyCode::Kp5, "Num5"),
    (KeyCode::Kp6, "Num6"),
    (KeyCode::Kp7, "Num7"),
    (KeyCode::Kp8, "Num8"),
    (KeyCode::Kp9, "Num9"),
    (KeyCode::F1, "F1"),
    (KeyCode::F2, "F2"),
    (KeyCode::F4, "F4"),
    (KeyCode::F5, "F5"),
    (KeyCode::F6, "F6"),
    (KeyCode::F7, "F7"),
    (KeyCode::F8, "F8"),
    (KeyCode::F9, "F9"),
    (KeyCode::F10, "F10"),
    (KeyCode::F12, "F12"),
];

pub fn key_name(k: KeyCode) -> &'static str {
    BINDABLE.iter().find(|(c, _)| *c == k).map(|(_, n)| *n).unwrap_or("?")
}

pub fn bindable(k: KeyCode) -> bool {
    BINDABLE.iter().any(|(c, _)| *c == k)
}

fn key_from_name(n: &str) -> Option<KeyCode> {
    BINDABLE.iter().find(|(_, s)| *s == n).map(|(c, _)| *c)
}

#[derive(Clone, Debug)]
pub struct Settings {
    pub name: String,
    /// One key per `Action`, in `ACTIONS` order.
    pub binds: [KeyCode; ACTIONS.len()],
    pub music: f32,
    pub sfx: f32,
    pub numbers: bool,
    pub overlay: bool,
    pub meter: bool,
    pub fullscreen: bool,
    pub gore: bool,
    pub lighting: bool,
    pub diff: Difficulty,
    pub last_ip: String,
    /// Windowed resolution.
    pub resolution: (u32, u32),
    pub vsync: bool,
    /// 0 = unlimited.
    pub fps_cap: u32,
    /// MSAA samples: 1 (off), 2, 4 or 8.
    pub msaa: u32,
    /// 0 low, 1 medium, 2 high.
    pub light_quality: u8,
    pub detail_high: bool,
    /// 0 low, 1 medium, 2 high.
    pub particles: u8,
    pub fog: bool,
    pub grain: bool,
    pub shake: bool,
    /// Index into `ui::UI_SCALES`; 0 is automatic.
    pub ui_scale: u8,
    /// Weapons aim at the nearest enemy instead of where you walk.
    pub auto_aim: bool,
}

pub const RESOLUTIONS: [(u32, u32); 6] = [(1280, 720), (1366, 768), (1600, 900), (1920, 1080), (2560, 1440), (3840, 2160)];
pub const FPS_CAPS: [u32; 9] = [30, 60, 75, 90, 120, 144, 165, 240, 0];
pub const MSAA: [u32; 4] = [1, 2, 4, 8];

impl Default for Settings {
    fn default() -> Self {
        Settings {
            binds: ACTIONS.map(Action::default_key),
            name: "Ashen One".into(),
            music: 0.4,
            sfx: 0.55,
            numbers: true,
            overlay: true,
            meter: true,
            fullscreen: false,
            gore: true,
            lighting: true,
            diff: Difficulty::default(),
            last_ip: String::new(),
            resolution: (1280, 720),
            // Off: on a 60 Hz screen VSync can halve the frame rate; the FPS cap paces frames instead.
            vsync: false,
            fps_cap: 90,
            msaa: 4,
            light_quality: 1,
            detail_high: true,
            particles: 2,
            fog: true,
            grain: true,
            shake: true,
            ui_scale: 0,
            auto_aim: false,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Save {
    /// Banked at shrines, spent on shrine upgrades.
    pub souls: u64,
    /// Earned at the end of each vigil, spent in the Reliquary.
    pub marks: u64,
    pub profile: Profile,
    /// Souls left behind at the place of the last death (single player only).
    pub remnant: Option<(Vec2, u64)>,
    pub bucket_unlocked: bool,
    pub best_time: f32,
    pub vigils: u32,
    /// Grave Cats befriended over all vigils.
    pub cats: u32,
    pub achievements: u32,
    /// Best ten vigils, best first.
    pub runs: Vec<RunRecord>,
    pub best_kills: u32,
    pub best_level: u32,
    pub best_damage: f64,
    /// Challenge bits chosen at the Ash Shrine.
    pub challenges: u8,
    pub readonly: bool,
    pub settings: Settings,
}

impl Default for Save {
    fn default() -> Self {
        Save {
            souls: 0,
            marks: 0,
            profile: Profile::default(),
            remnant: None,
            bucket_unlocked: false,
            best_time: 0.0,
            vigils: 0,
            cats: 0,
            achievements: 0,
            runs: Vec::new(),
            best_kills: 0,
            best_level: 0,
            best_damage: 0.0,
            challenges: 0,
            readonly: false,
            settings: Settings::default(),
        }
    }
}

const FILE: &str = "ashen_vigil.sav";

/// Where older versions kept the save: next to the executable.
fn legacy_path() -> Option<PathBuf> {
    std::env::current_exe().ok().and_then(|p| p.parent().map(|d| d.join(FILE)))
}

/// `%APPDATA%\AshenVigil\ashen_vigil.sav`, or next to the executable if there is no APPDATA.
fn backup_path(p: &Path) -> PathBuf {
    p.with_extension("sav.bak")
}

fn path() -> PathBuf {
    match std::env::var_os("APPDATA") {
        Some(appdata) => PathBuf::from(appdata).join("AshenVigil").join(FILE),
        None => legacy_path().unwrap_or_else(|| PathBuf::from(FILE)),
    }
}

/// Moves a save from next to the executable into the new location, once.
fn migrate() {
    let new = path();
    if new.exists() {
        return;
    }
    let Some(old) = legacy_path().filter(|p| p.exists() && *p != new) else { return };
    if let Some(dir) = new.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if std::fs::copy(&old, &new).is_ok() {
        let _ = std::fs::remove_file(&old);
    }
}

fn flag(v: &str) -> bool {
    v == "1"
}

fn parse_run(v: &str) -> Option<RunRecord> {
    let p: Vec<&str> = v.split(',').collect();
    if p.len() != 8 {
        return None;
    }
    Some(RunRecord {
        time: p[0].parse().ok()?,
        kills: p[1].parse().ok()?,
        level: p[2].parse().ok()?,
        players: p[3].parse().ok()?,
        won: flag(p[4]),
        class: class_from_u8(p[5].parse().ok()?),
        difficulty: p[6].to_string(),
        damage: p[7].parse().ok()?,
    })
}

impl Save {
    /// Adds a vigil to the leaderboard, keeping the best ten. Returns its rank (0-based) if it made it.
    pub fn record_run(&mut self, r: RunRecord) -> Option<usize> {
        let rank = self.runs.iter().position(|o| r.better_than(o)).unwrap_or(self.runs.len());
        if rank >= 10 {
            return None;
        }
        self.runs.insert(rank, r);
        self.runs.truncate(10);
        Some(rank)
    }
}

impl Save {
    pub fn load() -> Save {
        migrate();
        Save::load_at(&path())
    }

    /// The save at `p`, or its backup when the save is missing or damaged.
    pub fn load_at(p: &Path) -> Save {
        let read = |p: &Path| std::fs::read_to_string(p).ok().filter(|t| t.lines().any(|l| l.starts_with("souls=")));
        match read(p).or_else(|| read(&backup_path(p))) {
            Some(text) => Save::from_text(&text),
            None => Save::default(),
        }
    }

    pub fn from_text(text: &str) -> Save {
        let mut s = Save::default();
        let st = &mut s.settings;
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            let (k, v) = (k.trim(), v.trim());
            let f = |d: f32| v.parse().unwrap_or(d);
            match k {
                "souls" => s.souls = v.parse().unwrap_or(0),
                "marks" => s.marks = v.parse().unwrap_or(0),
                // Older saves predate unlocks; never take away the defaults.
                "unlocks" => s.profile.unlocks = v.parse::<u64>().unwrap_or(0) | default_unlocks(),
                "bucket" => s.bucket_unlocked = flag(v),
                "best" => s.best_time = f(0.0),
                "vigils" => s.vigils = v.parse().unwrap_or(0),
                "cats" => s.cats = v.parse().unwrap_or(0),
                "achievements" => s.achievements = v.parse().unwrap_or(0),
                "best_kills" => s.best_kills = v.parse().unwrap_or(0),
                "best_level" => s.best_level = v.parse().unwrap_or(0),
                "best_damage" => s.best_damage = v.parse().unwrap_or(0.0),
                "challenges" => s.challenges = v.parse().unwrap_or(0),
                "run" => {
                    if let Some(r) = parse_run(v) {
                        s.runs.push(r);
                    }
                }
                "lighting" => st.lighting = flag(v),
                "resolution" => {
                    if let Some((w, h)) = v.split_once('x') {
                        if let (Ok(w), Ok(h)) = (w.parse(), h.parse()) {
                            st.resolution = (w, h);
                        }
                    }
                }
                "vsync" => st.vsync = flag(v),
                "fps_cap" => st.fps_cap = v.parse().unwrap_or(0),
                "msaa" => st.msaa = v.parse::<u32>().ok().filter(|m| MSAA.contains(m)).unwrap_or(4),
                "light_quality" => st.light_quality = v.parse::<u8>().unwrap_or(1).min(2),
                "detail" => st.detail_high = flag(v),
                "particles" => st.particles = v.parse::<u8>().unwrap_or(2).min(2),
                "fog" => st.fog = flag(v),
                "grain" => st.grain = flag(v),
                "shake" => st.shake = flag(v),
                "ui_scale" => st.ui_scale = v.parse::<u8>().unwrap_or(0).min(5),
                _ if k.starts_with("bind_") => {
                    if let (Some(i), Some(key)) = (ACTIONS.iter().position(|a| a.key() == &k[5..]), key_from_name(v)) {
                        st.binds[i] = key;
                    }
                }
                "auto_aim" => st.auto_aim = flag(v),
                "remnant" => {
                    let p: Vec<&str> = v.split(',').collect();
                    if let [x, y, n] = p[..] {
                        if let (Ok(x), Ok(y), Ok(n)) = (x.parse(), y.parse(), n.parse()) {
                            s.remnant = Some((Vec2::new(x, y), n));
                        }
                    }
                }
                "name" if !v.is_empty() => st.name = v.chars().take(16).collect(),
                "last_ip" => st.last_ip = v.to_string(),
                "music" => st.music = f(0.4),
                "sfx" => st.sfx = f(0.55),
                "numbers" => st.numbers = flag(v),
                "overlay" => st.overlay = flag(v),
                "meter" => st.meter = flag(v),
                "gore" => st.gore = flag(v),
                "fullscreen" => st.fullscreen = flag(v),
                "diff_count" => st.diff.count = f(1.0),
                "diff_hp" => st.diff.hp = f(1.0),
                "diff_dmg" => st.diff.dmg = f(1.0),
                "diff_speed" => st.diff.speed = f(1.0),
                "diff_boss" => st.diff.boss_hp = f(1.0),
                _ => {
                    if let Some(i) = META.iter().position(|m| m.key == k) {
                        s.profile.meta[i] = v.parse::<u32>().unwrap_or(0).min(META[i].max);
                    } else if let Some(i) = RELICS.iter().position(|r| r.key == k) {
                        s.profile.relics[i] = v.parse::<u32>().unwrap_or(0).min(RELICS[i].max);
                    }
                }
            }
        }
        s
    }

    pub fn store(&self) {
        if self.readonly {
            return;
        }
        if let Err(e) = self.store_at(&path()) {
            crate::log!("saving failed: {}", e);
        }
    }

    /// Writes the save so that a crash at any moment leaves a readable save or backup behind:
    /// the new text goes to a temporary file first, the old save becomes the backup, and only
    /// then the new file takes its place.
    pub fn store_at(&self, p: &Path) -> std::io::Result<()> {
        if let Some(dir) = p.parent() {
            std::fs::create_dir_all(dir)?;
        }
        let tmp = p.with_extension("sav.tmp");
        {
            let mut f = std::fs::File::create(&tmp)?;
            f.write_all(self.to_text().as_bytes())?;
            f.sync_all()?;
        }
        if p.exists() {
            std::fs::rename(p, backup_path(p))?;
        }
        std::fs::rename(&tmp, p)
    }

    pub fn to_text(&self) -> String {
        let b = |x: bool| if x { 1 } else { 0 };
        let st = &self.settings;
        let mut out = String::new();
        out += &format!("souls={}\nmarks={}\nunlocks={}\n", self.souls, self.marks, self.profile.unlocks);
        out += &format!("bucket={}\nbest={:.1}\nvigils={}\ncats={}\n", b(self.bucket_unlocked), self.best_time, self.vigils, self.cats);
        out += &format!(
            "achievements={}\nbest_kills={}\nbest_level={}\nbest_damage={:.0}\nchallenges={}\n",
            self.achievements, self.best_kills, self.best_level, self.best_damage, self.challenges
        );
        for r in &self.runs {
            out += &format!("run={:.1},{},{},{},{},{},{},{:.0}\n", r.time, r.kills, r.level, r.players, b(r.won), class_to_u8(r.class), r.difficulty.replace(',', " "), r.damage);
        }
        if let Some((p, n)) = self.remnant {
            out += &format!("remnant={:.0},{:.0},{}\n", p.x, p.y, n);
        }
        out += &format!("name={}\nlast_ip={}\nui_scale={}\n", st.name, st.last_ip, st.ui_scale);
        for (a, k) in ACTIONS.iter().zip(st.binds) {
            out += &format!("bind_{}={}\n", a.key(), key_name(k));
        }
        out += &format!("music={:.2}\nsfx={:.2}\n", st.music, st.sfx);
        out += &format!(
            "numbers={}\noverlay={}\nmeter={}\ngore={}\nlighting={}\nfullscreen={}\n",
            b(st.numbers),
            b(st.overlay),
            b(st.meter),
            b(st.gore),
            b(st.lighting),
            b(st.fullscreen)
        );
        out += &format!("resolution={}x{}\nvsync={}\nfps_cap={}\nmsaa={}\n", st.resolution.0, st.resolution.1, b(st.vsync), st.fps_cap, st.msaa);
        out += &format!(
            "light_quality={}\ndetail={}\nparticles={}\nfog={}\ngrain={}\nshake={}\nauto_aim={}\n",
            st.light_quality,
            b(st.detail_high),
            st.particles,
            b(st.fog),
            b(st.grain),
            b(st.shake),
            b(st.auto_aim)
        );
        out += &format!("diff_count={}\ndiff_hp={}\ndiff_dmg={}\ndiff_speed={}\ndiff_boss={}\n", st.diff.count, st.diff.hp, st.diff.dmg, st.diff.speed, st.diff.boss_hp);
        for (i, m) in META.iter().enumerate() {
            out += &format!("{}={}\n", m.key, self.profile.meta[i]);
        }
        for (i, r) in RELICS.iter().enumerate() {
            out += &format!("{}={}\n", r.key, self.profile.relics[i]);
        }
        out
    }
}
