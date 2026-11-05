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
    Skill,
    Reroll,
    Pause,
    Weapons,
    Numbers,
    Mute,
}

pub const ACTIONS: [Action; 11] =
    [Action::Up, Action::Down, Action::Left, Action::Right, Action::Roll, Action::Skill, Action::Reroll, Action::Pause, Action::Weapons, Action::Numbers, Action::Mute];

impl Action {
    pub fn label(self) -> &'static str {
        match self {
            Action::Up => "Move up",
            Action::Down => "Move down",
            Action::Left => "Move left",
            Action::Right => "Move right",
            Action::Roll => "Roll",
            Action::Skill => "Class skill",
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
            Action::Skill => KeyCode::E,
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
            Action::Skill => "skill",
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
    /// Volume of each kind of sound, see `audio::SOUND_KINDS`.
    pub vols: [f32; crate::audio::SOUND_KINDS.len()],
    /// How opaque the heroes' weapons and skills are drawn, in percent.
    pub effect_alpha: u8,
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
    /// How far off-screen markers sit inside the screen edge, in percent of the shorter side.
    pub marker_inset: u8,
    /// The next vigil (alone, or hosted) is a boss rush.
    pub boss_rush: bool,
    /// The field to keep the vigil on, see `world::ALL_BIOMES`.
    pub biome: u8,
    /// A single-player vigil pauses when the window loses focus.
    pub pause_unfocused: bool,
    /// The sound goes quiet while another window has the focus.
    pub mute_unfocused: bool,
    /// The screen flashes when the hero is hit.
    pub hurt_flash: bool,
    /// Small health and stamina bars under our own hero.
    pub hero_hp_bar: bool,
    pub hero_stamina_bar: bool,
    /// How bright the night is, in percent.
    pub brightness: u8,
    /// Lords speak their lines.
    pub boss_lines: bool,
    /// Friends on Discord see what you are doing.
    pub discord: bool,
    /// Ask GitHub at start whether a newer version is out.
    pub check_updates: bool,
    /// The soft variant: pink instead of blood, hearts instead of bullets, blushing foes.
    pub cute: bool,
}

pub const RESOLUTIONS: [(u32, u32); 6] = [(1280, 720), (1366, 768), (1600, 900), (1920, 1080), (2560, 1440), (3840, 2160)];
pub const FPS_CAPS: [u32; 9] = [30, 60, 75, 90, 120, 144, 165, 240, 0];
pub const MSAA: [u32; 4] = [1, 2, 4, 8];
/// The farthest off-screen markers can sit from the edge, in percent.
pub const MAX_MARKER_INSET: u8 = 35;
/// How dark or bright the night can be made, in percent.
pub const MIN_BRIGHTNESS: u8 = 70;
pub const MAX_BRIGHTNESS: u8 = 150;
/// The faintest the heroes' weapons and skills can be drawn, in percent.
pub const MIN_EFFECT_ALPHA: u8 = 15;

impl Default for Settings {
    fn default() -> Self {
        Settings {
            binds: ACTIONS.map(Action::default_key),
            name: "Ashen One".into(),
            music: 0.4,
            sfx: 0.55,
            vols: [1.0; crate::audio::SOUND_KINDS.len()],
            effect_alpha: 100,
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
            marker_inset: 12,
            boss_rush: false,
            biome: 0,
            pause_unfocused: true,
            mute_unfocused: false,
            hurt_flash: true,
            hero_hp_bar: true,
            hero_stamina_bar: true,
            brightness: 100,
            boss_lines: true,
            discord: true,
            check_updates: true,
            cute: false,
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
    /// Seven clicks on the title turn the menu's embers blue.
    pub blue_embers: bool,
    /// "cute" was typed in the main menu once; the option shows from then on.
    pub cute_found: bool,
    pub achievements: u128,
    /// Hero skins bought, one bit per entry of `SKINS`.
    pub skins: u32,
    /// Levels bought at the Ash Shrine over all time.
    pub shrine_levels: u32,
    /// Souls and Ashen Marks given away in the Reliquary's offerings.
    pub offered_souls: u64,
    pub offered_marks: u64,
    /// The marks for weapons bought before every weapon was free have been given back.
    pub weapons_refunded: bool,
    /// Classes that have kept a vigil until dawn, one bit per class.
    pub won_classes: u16,
    /// Best ten vigils, best first.
    pub runs: Vec<RunRecord>,
    pub best_kills: u32,
    pub best_level: u32,
    pub best_damage: f64,
    /// Challenge bits chosen at the Ash Shrine.
    pub challenges: u8,
    /// Curse bits chosen at the Ash Shrine, see `Curse`.
    pub curses: u8,
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
            blue_embers: false,
            cute_found: false,
            achievements: 0,
            skins: 0,
            shrine_levels: 0,
            offered_souls: 0,
            offered_marks: 0,
            weapons_refunded: false,
            won_classes: 0,
            runs: Vec::new(),
            best_kills: 0,
            best_level: 0,
            best_damage: 0.0,
            challenges: 0,
            curses: 0,
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
        let mut bought = 0u64;
        let st = &mut s.settings;
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            let (k, v) = (k.trim(), v.trim());
            let f = |d: f32| v.parse().unwrap_or(d);
            match k {
                "souls" => s.souls = v.parse().unwrap_or(0),
                "marks" => s.marks = v.parse().unwrap_or(0),
                // Older saves predate unlocks; never take away the defaults.
                "unlocks" => {
                    bought = v.parse::<u64>().unwrap_or(0);
                    s.profile.unlocks = bought | default_unlocks();
                }
                "weapons_refunded" => s.weapons_refunded = flag(v),
                "bucket" => s.bucket_unlocked = flag(v),
                "best" => s.best_time = f(0.0),
                "vigils" => s.vigils = v.parse().unwrap_or(0),
                "cats" => s.cats = v.parse().unwrap_or(0),
                "blue_embers" => s.blue_embers = flag(v),
                "achievements" => s.achievements = v.parse().unwrap_or(0),
                "skins" => s.skins = v.parse().unwrap_or(0),
                "skin" => s.profile.skin = v.parse::<u8>().unwrap_or(0).min(SKINS.len() as u8),
                "shrine_levels" => s.shrine_levels = v.parse().unwrap_or(0),
                "offered_souls" => s.offered_souls = v.parse().unwrap_or(0),
                "offered_marks" => s.offered_marks = v.parse().unwrap_or(0),
                "won_classes" => s.won_classes = v.parse().unwrap_or(0),
                "boss_rush" => st.boss_rush = flag(v),
                // Before the marsh was drained: 2 was the cathedral, 1 the marsh.
                "biome" => st.biome = u8::from(v.parse::<u8>().unwrap_or(0) == 2),
                "field" => st.biome = v.parse::<u8>().unwrap_or(0).min(crate::world::ALL_BIOMES.len() as u8 - 1),
                "pause_unfocused" => st.pause_unfocused = flag(v),
                "mute_unfocused" => st.mute_unfocused = flag(v),
                "hurt_flash" => st.hurt_flash = flag(v),
                "hero_hp_bar" => st.hero_hp_bar = flag(v),
                "hero_stamina_bar" => st.hero_stamina_bar = flag(v),
                "brightness" => st.brightness = v.parse::<u8>().unwrap_or(100).clamp(MIN_BRIGHTNESS, MAX_BRIGHTNESS),
                "boss_lines" => st.boss_lines = flag(v),
                "discord" => st.discord = flag(v),
                "check_updates" => st.check_updates = flag(v),
                "marker_inset" => st.marker_inset = v.parse::<u8>().unwrap_or(12).min(MAX_MARKER_INSET),
                "best_kills" => s.best_kills = v.parse().unwrap_or(0),
                "best_level" => s.best_level = v.parse().unwrap_or(0),
                "best_damage" => s.best_damage = v.parse().unwrap_or(0.0),
                "challenges" => s.challenges = v.parse().unwrap_or(0),
                "curses" => s.curses = v.parse::<u8>().unwrap_or(0) & 63,
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
                "cute" => st.cute = flag(v),
                "cute_found" => s.cute_found = flag(v),
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
                k if k.starts_with("vol_") => {
                    if let Some(i) = crate::audio::SOUND_KINDS.iter().position(|(key, _)| k == format!("vol_{}", key)) {
                        st.vols[i] = f(1.0).clamp(0.0, 1.0);
                    }
                }
                "effect_alpha" => st.effect_alpha = v.parse::<u8>().unwrap_or(100).clamp(MIN_EFFECT_ALPHA, 100),
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
                "diff_boost" => {
                    for (i, n) in v.split(',').take(3).enumerate() {
                        st.diff.boost[i] = n.parse::<u8>().unwrap_or(0).min(BOOST_MAX[i]);
                    }
                }
                _ => {
                    if let Some(i) = META.iter().position(|m| m.key == k) {
                        s.profile.meta[i] = v.parse::<u32>().unwrap_or(0).min(META[i].max);
                    } else if let Some(i) = RELICS.iter().position(|r| r.key == k) {
                        s.profile.relics[i] = v.parse::<u32>().unwrap_or(0).min(RELICS[i].max);
                    }
                }
            }
        }
        // A skin is kept if it was bought or if what was offered earns it.
        let earned = |skin: u8| match SKINS.get(skin as usize - 1).and_then(|d| d.offering) {
            Some((true, n)) => s.offered_souls >= n,
            Some((false, n)) => s.offered_marks >= n,
            None => false,
        };
        if s.profile.skin > 0 && s.skins & 1 << (s.profile.skin - 1) == 0 && !earned(s.profile.skin) {
            s.profile.skin = 0;
        }
        if !s.weapons_refunded {
            s.weapons_refunded = true;
            for (w, cost) in FREED_WEAPONS {
                if bought & Unlock::Weapon(w).bit() != 0 {
                    s.marks += cost;
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
        out += &format!("bucket={}\nbest={:.1}\nvigils={}\ncats={}\nblue_embers={}\n", b(self.bucket_unlocked), self.best_time, self.vigils, self.cats, b(self.blue_embers));
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
        out += &format!("cute={}\ncute_found={}\nboss_rush={}\nfield={}\ncurses={}\n", b(st.cute), b(self.cute_found), b(st.boss_rush), st.biome, self.curses);
        out += &format!(
            "pause_unfocused={}\nmute_unfocused={}\nhurt_flash={}\nbrightness={}\nboss_lines={}\ndiscord={}\ncheck_updates={}\n",
            b(st.pause_unfocused),
            b(st.mute_unfocused),
            b(st.hurt_flash),
            st.brightness,
            b(st.boss_lines),
            b(st.discord),
            b(st.check_updates)
        );
        out += &format!("hero_hp_bar={}\nhero_stamina_bar={}\n", b(st.hero_hp_bar), b(st.hero_stamina_bar));
        out += &format!(
            "skins={}\nskin={}\nshrine_levels={}\nwon_classes={}\nmarker_inset={}\n",
            self.skins, self.profile.skin, self.shrine_levels, self.won_classes, st.marker_inset
        );
        out += &format!("offered_souls={}\noffered_marks={}\nweapons_refunded={}\n", self.offered_souls, self.offered_marks, b(self.weapons_refunded));
        for (a, k) in ACTIONS.iter().zip(st.binds) {
            out += &format!("bind_{}={}\n", a.key(), key_name(k));
        }
        out += &format!("music={:.2}\nsfx={:.2}\neffect_alpha={}\n", st.music, st.sfx, st.effect_alpha);
        for ((key, _), v) in crate::audio::SOUND_KINDS.iter().zip(st.vols) {
            out += &format!("vol_{}={:.2}\n", key, v);
        }
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
            "light_quality={}\ndetail={}\nparticles={}\nfog={}\ngrain={}\nshake={}\n",
            st.light_quality,
            b(st.detail_high),
            st.particles,
            b(st.fog),
            b(st.grain),
            b(st.shake)
        );
        out += &format!("diff_count={}\ndiff_hp={}\ndiff_dmg={}\ndiff_speed={}\ndiff_boss={}\n", st.diff.count, st.diff.hp, st.diff.dmg, st.diff.speed, st.diff.boss_hp);
        out += &format!("diff_boost={},{},{}\n", st.diff.boost[0], st.diff.boost[1], st.diff.boost[2]);
        for (i, m) in META.iter().enumerate() {
            out += &format!("{}={}\n", m.key, self.profile.meta[i]);
        }
        for (i, r) in RELICS.iter().enumerate() {
            out += &format!("{}={}\n", r.key, self.profile.relics[i]);
        }
        out
    }
}
