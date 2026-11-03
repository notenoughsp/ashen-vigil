//! Main menu, options, the Ash Shrine (souls) and the Reliquary (ashen marks).

use super::*;
use crate::save::{bindable, key_name, Action, ACTIONS};
use crate::save::{FPS_CAPS, MSAA, RESOLUTIONS};
use crate::ui::{ui_h, ui_w, BLOOD as BLOOD_RED, EMBER, GOLD, TEAL, UI_SCALES};

const MENU_ITEMS: [&str; 7] = ["Start", "Online Lobby", "Reliquary", "Records", "Options", "Credits", "Exit"];
/// Ash Shrine rows: class, the challenges, the soul upgrades, begin.
const HUB_CHALLENGE_ROW: usize = 1;
const HUB_META_ROW: usize = HUB_CHALLENGE_ROW + ALL_CHALLENGES.len();
const HUB_ROWS: usize = HUB_META_ROW + META_COUNT + 1;
const VOLUMES: [f32; 11] = [0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.9, 1.0];

/// One row on the options screen.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Opt {
    Name,
    Music,
    Sfx,
    Numbers,
    Meter,
    Overlay,
    Shake,
    UiScale,
    Bind(u8),
    ResetBinds,
    AutoAim,
    Resolution,
    Fullscreen,
    Vsync,
    FpsCap,
    Msaa,
    Lighting,
    LightQuality,
    Detail,
    Particles,
    Gore,
    Fog,
    Grain,
    Preset,
    Count,
    Hp,
    Dmg,
    Speed,
    BossHp,
    Back,
}

impl Opt {
    fn label(self) -> &'static str {
        match self {
            Opt::Name => "Name",
            Opt::Music => "Music volume",
            Opt::Sfx => "Effects volume",
            Opt::Numbers => "Damage numbers",
            Opt::Meter => "Damage meter",
            Opt::Overlay => "FPS and network overlay",
            Opt::Shake => "Screen shake",
            Opt::UiScale => "Interface size",
            Opt::Bind(i) => ACTIONS[i as usize].label(),
            Opt::ResetBinds => "Reset to defaults",
            Opt::AutoAim => "Auto-aim (F)",
            Opt::Resolution => "Resolution",
            Opt::Fullscreen => "Fullscreen",
            Opt::Vsync => "VSync",
            Opt::FpsCap => "FPS limit",
            Opt::Msaa => "Anti-aliasing",
            Opt::Lighting => "Dynamic lighting",
            Opt::LightQuality => "Light quality",
            Opt::Detail => "Model detail",
            Opt::Particles => "Particles",
            Opt::Gore => "Blood and gore",
            Opt::Fog => "Fog",
            Opt::Grain => "Film grain",
            Opt::Preset => "Difficulty",
            Opt::Count => "Enemy count",
            Opt::Hp => "Enemy health",
            Opt::Dmg => "Enemy damage",
            Opt::Speed => "Enemy speed",
            Opt::BossHp => "Boss health",
            Opt::Back => "Back",
        }
    }

    fn hint(self) -> Option<&'static str> {
        Some(match self {
            Opt::Resolution => "Window size. Fullscreen always uses the monitor's resolution.",
            Opt::AutoAim => "Weapons aim at the nearest enemy instead of where you walk. F toggles it in a vigil.",
            Opt::Vsync => "Syncs to the monitor refresh. Takes effect after a restart.",
            Opt::Bind(_) => "Enter, then press the new key. Arrow keys, Shift and Esc always work too.",
            Opt::UiScale => "Size of menus and HUD. Auto grows with the window above 1080p.",
            Opt::FpsCap => "Caps the frame rate. With VSync on, the lower of both wins.",
            Opt::Msaa => "Smooths edges of all shapes. Takes effect after a restart.",
            Opt::LightQuality => "Resolution of the light map. Low is fastest.",
            Opt::Detail => "Low draws simpler enemies, for very large hordes on slow machines.",
            Opt::Particles => "Blood, sparks and embers.",
            Opt::Lighting => "Torches, fire and spells light a dark world.",
            _ => return None,
        })
    }
}

const PAGES: [(&str, &[Opt]); 4] = [
    (
        "Controls",
        &[
            Opt::Bind(0),
            Opt::Bind(1),
            Opt::Bind(2),
            Opt::Bind(3),
            Opt::Bind(4),
            Opt::Bind(5),
            Opt::Bind(6),
            Opt::Bind(7),
            Opt::Bind(8),
            Opt::Bind(9),
            Opt::Bind(10),
            Opt::ResetBinds,
            Opt::Back,
        ],
    ),
    ("General", &[Opt::Name, Opt::Music, Opt::Sfx, Opt::AutoAim, Opt::Numbers, Opt::Meter, Opt::Overlay, Opt::Shake, Opt::UiScale, Opt::Back]),
    (
        "Graphics",
        &[
            Opt::Resolution,
            Opt::Fullscreen,
            Opt::Vsync,
            Opt::FpsCap,
            Opt::Msaa,
            Opt::Lighting,
            Opt::LightQuality,
            Opt::Detail,
            Opt::Particles,
            Opt::Gore,
            Opt::Fog,
            Opt::Grain,
            Opt::Back,
        ],
    ),
    ("Difficulty", &[Opt::Preset, Opt::Count, Opt::Hp, Opt::Dmg, Opt::Speed, Opt::BossHp, Opt::Back]),
];

/// Where the main menu rows sit, for drawing and for the mouse.
fn menu_rows() -> Vec<Rect> {
    let (sw, sh) = (ui_w(), ui_h());
    (0..MENU_ITEMS.len()).map(|i| Rect::new(sw * 0.5 - 160.0, sh * 0.37 + i as f32 * 44.0, 320.0, 38.0)).collect()
}

/// A selectable row with a label on the left and a value on the right.
fn value_row(ui: &Ui, x: f32, y: f32, w: f32, on: bool, label: &str, value: &str) {
    ui.row(x, y, w, 34.0, on);
    ui.text(label, x + 14.0, y + 23.0, 19.0, PALE);
    ui.text_r(value, x + w - 14.0, y + 23.0, 18.0, if on { GOLD } else { DIM });
}

impl App {
    pub(super) fn update_menu(&mut self) {
        self.audio.music(Music::Ambient);
        for k in [KeyCode::Up, KeyCode::Down, KeyCode::Left, KeyCode::Right, KeyCode::B, KeyCode::A] {
            if is_key_pressed(k) {
                self.konami.push(k);
                if self.konami.len() > KONAMI.len() {
                    self.konami.remove(0);
                }
            }
        }
        if self.konami.as_slice() == KONAMI {
            self.konami.clear();
            self.audio.play("clang", 1.0);
            if self.save.bucket_unlocked {
                self.set_note("The bucket is already yours. Wear it with pride.", 4.0);
            } else {
                self.save.bucket_unlocked = true;
                self.save.store();
                self.set_note("A bucket clangs somewhere in the dark... The Bucket Knight joins the vigil.", 6.0);
            }
        }
        if nav(&mut self.menu_sel, MENU_ITEMS.len(), false) {
            self.audio.play("select", 0.4);
        }
        drain_chars();
        if back() {
            self.menu_sel = MENU_ITEMS.len() - 1;
        }
        let under = hovered(&menu_rows());
        if let Some(i) = under.filter(|_| mouse_moved()) {
            if i != self.menu_sel {
                self.menu_sel = i;
                self.audio.play("select", 0.3);
            }
        }
        let click = clicked() && under.is_some();
        if click {
            self.menu_sel = under.unwrap_or(self.menu_sel);
        }
        if !confirm() && !click {
            return;
        }
        self.audio.play("select", 0.6);
        match self.menu_sel {
            0 => self.screen = Screen::Hub,
            1 => self.open_lobby(),
            2 => {
                self.screen = Screen::Reliquary;
                self.relic_sel = 0;
            }
            3 => {
                self.screen = Screen::Records;
                self.records_page = 0;
            }
            4 => {
                self.screen = Screen::Options;
                self.opt_sel = 0;
            }
            5 => {
                self.screen = Screen::Credits;
                self.credits_scroll = 0.0;
            }
            _ => {
                self.save.store();
                crate::log!("exit from the main menu");
                std::process::exit(0);
            }
        }
    }

    pub(super) fn draw_menu(&self, ui: &Ui) {
        let (sw, sh) = (ui_w(), ui_h());
        ui.spaced_c("ASHEN VIGIL", sw * 0.5, sh * 0.27, 72.0, 14.0, PALE);
        ui.text_c("hold the dark at bay until dawn", sw * 0.5, sh * 0.27 + 44.0, 20.0, DIM);
        for (i, (item, r)) in MENU_ITEMS.iter().zip(menu_rows()).enumerate() {
            let on = self.menu_sel == i;
            ui.row(r.x, r.y, r.w, r.h, on);
            ui.text_c(item, sw * 0.5, r.y + 27.0, 25.0, if on { GOLD } else { PALE });
        }
        ui.text_c(hint("W/S, arrows or mouse to choose, Enter or click to confirm", "Stick or d-pad to choose, A to confirm"), sw * 0.5, sh - 40.0, 16.0, DIM);
        let b = self.save.best_time as u32;
        let cats = if self.save.cats > 0 { format!("    Cats befriended {}", self.save.cats) } else { String::new() };
        ui.text_c(
            &format!("Souls {}    Ashen Marks {}    Vigils {}    Longest {:02}:{:02}{}", self.save.souls, self.save.marks, self.save.vigils, b / 60, b % 60, cats),
            sw * 0.5,
            sh - 64.0,
            16.0,
            DIM,
        );
        ui.text_r(&format!("v{}", crate::net::VERSION), sw - 12.0, sh - 12.0, 14.0, DIM);
    }

    // ---------------------------------------------------------------- credits

    /// Credits, then the full license texts that ship inside the executable.
    fn credit_lines() -> Vec<(String, bool)> {
        let mut lines: Vec<(String, bool)> = vec![
            ("Ashen Vigil".into(), true),
            ("A dark vigil until dawn, made with Rust and macroquad (MIT / Apache-2.0).".into(), false),
            ("All graphics are drawn by code; music and most sounds are synthesized.".into(), false),
            (String::new(), false),
        ];
        for (title, text) in crate::embedded::LICENSES {
            lines.push((title.to_string(), true));
            for l in text.lines() {
                lines.push((l.trim_end().to_string(), false));
            }
            lines.push((String::new(), false));
        }
        lines
    }

    pub(super) fn update_credits(&mut self, dt: f32) {
        drain_chars();
        let speed = 420.0;
        if is_key_down(KeyCode::W) || is_key_down(KeyCode::Up) {
            self.credits_scroll -= speed * dt;
        }
        if is_key_down(KeyCode::S) || is_key_down(KeyCode::Down) {
            self.credits_scroll += speed * dt;
        }
        if is_key_pressed(KeyCode::PageDown) {
            self.credits_scroll += ui_h() * 0.8;
        }
        if is_key_pressed(KeyCode::PageUp) {
            self.credits_scroll -= ui_h() * 0.8;
        }
        let max = (Self::credit_lines().len() as f32 * 17.0 - ui_h() + 200.0).max(0.0);
        self.credits_scroll = self.credits_scroll.clamp(0.0, max);
        if back() || confirm() {
            self.screen = Screen::Menu;
        }
    }

    pub(super) fn draw_credits(&self, ui: &Ui) {
        let (sw, sh) = (ui_w(), ui_h());
        ui.spaced_c("CREDITS", sw * 0.5, 56.0, 40.0, 8.0, PALE);
        let x = (sw * 0.5 - 330.0).max(20.0);
        let top = 90.0;
        let bottom = sh - 50.0;
        let mut y = top - self.credits_scroll;
        for (line, heading) in Self::credit_lines() {
            if y > top - 20.0 && y < bottom {
                if heading {
                    ui.text(&line, x, y + 14.0, 18.0, GOLD);
                } else {
                    mono(&line, x, y + 13.0, 15.0, DIM);
                }
            }
            y += if heading { 24.0 } else { 17.0 };
        }
        // Fade the text out at the edges of the scroll area.
        draw_rectangle(0.0, top - 22.0, sw, 22.0, Color::new(0.03, 0.025, 0.03, 0.9));
        draw_rectangle(0.0, bottom, sw, sh - bottom, Color::new(0.03, 0.025, 0.03, 0.95));
        ui.text_c(hint("W/S scroll    Esc back", "Stick scroll    B back"), sw * 0.5, sh - 20.0, 15.0, DIM);
    }

    // ---------------------------------------------------------------- options

    fn option_rows(&self) -> &'static [Opt] {
        PAGES[self.opt_page.min(PAGES.len() - 1)].1
    }

    pub(super) fn update_options(&mut self) {
        // Waiting for the key to bind: Esc cancels, anything bindable takes the slot and
        // swaps with whatever action had that key before.
        if let Some(i) = self.binding {
            if is_key_pressed(KeyCode::Escape) || pad_pressed(Pad::Back) {
                self.binding = None;
            } else if let Some(k) = get_last_key_pressed().filter(|k| bindable(*k)) {
                let binds = &mut self.save.settings.binds;
                if let Some(j) = binds.iter().position(|b| *b == k) {
                    binds[j] = binds[i];
                }
                binds[i] = k;
                self.binding = None;
                self.audio.play("select", 0.6);
            }
            drain_chars();
            return;
        }
        // Q/E or Tab switch pages, but Q and E may be typed into the name.
        let rows = self.option_rows();
        let on_name = rows.get(self.opt_sel) == Some(&Opt::Name);
        let page_dir = if is_key_pressed(KeyCode::Tab) || (!on_name && is_key_pressed(KeyCode::E)) || is_key_pressed(KeyCode::PageDown) || pad_pressed(Pad::PageNext) {
            1
        } else if (!on_name && is_key_pressed(KeyCode::Q)) || is_key_pressed(KeyCode::PageUp) || pad_pressed(Pad::PagePrev) {
            -1
        } else {
            0
        };
        if page_dir != 0 {
            self.opt_page = (self.opt_page as i32 + page_dir).rem_euclid(PAGES.len() as i32) as usize;
            self.opt_sel = 0;
            self.audio.play("select", 0.5);
            drain_chars();
            return;
        }
        let rows = self.option_rows();
        if nav(&mut self.opt_sel, rows.len(), on_name) {
            self.audio.play("select", 0.4);
        }
        let opt = rows[self.opt_sel.min(rows.len() - 1)];
        if opt == Opt::Name {
            edit_text(&mut self.save.settings.name, 16, name_char);
        } else {
            drain_chars();
        }
        let on_back = opt == Opt::Back;
        let dir = if left(on_name) {
            -1
        } else if right(on_name) || (!on_name && !on_back && confirm()) {
            1
        } else {
            0
        };
        if let Opt::Bind(i) = opt {
            if confirm() {
                self.binding = Some(i as usize);
                self.audio.play("select", 0.5);
            }
        } else if dir != 0 && opt != Opt::Name && !on_back {
            self.change_option(opt, dir);
            self.audio.play("select", 0.5);
        }
        if back() || (on_back && confirm()) {
            if self.save.settings.name.trim().is_empty() {
                self.save.settings.name = "Ashen One".into();
            }
            self.save.store();
            self.screen = Screen::Menu;
        }
    }

    fn change_option(&mut self, opt: Opt, dir: i32) {
        let st = &mut self.save.settings;
        let cycle_u8 = |v: u8, n: u8| ((v as i32 + dir).rem_euclid(n as i32)) as u8;
        match opt {
            Opt::Music => {
                st.music = step(&VOLUMES, st.music, dir);
                self.audio.set_music_volume(st.music);
            }
            Opt::Sfx => {
                st.sfx = step(&VOLUMES, st.sfx, dir);
                self.audio.sfx_vol = st.sfx;
            }
            Opt::Numbers => st.numbers = !st.numbers,
            Opt::Meter => st.meter = !st.meter,
            Opt::Overlay => st.overlay = !st.overlay,
            Opt::Shake => st.shake = !st.shake,
            Opt::ResetBinds => st.binds = ACTIONS.map(Action::default_key),
            Opt::Bind(_) => {}
            Opt::UiScale => st.ui_scale = (st.ui_scale as i32 + dir).rem_euclid(UI_SCALES.len() as i32) as u8,
            Opt::AutoAim => st.auto_aim = !st.auto_aim,
            Opt::Resolution => {
                let i = RESOLUTIONS.iter().position(|r| *r == st.resolution).map(|i| i as i32).unwrap_or(0);
                st.resolution = RESOLUTIONS[(i + dir).clamp(0, RESOLUTIONS.len() as i32 - 1) as usize];
                if !st.fullscreen {
                    request_new_screen_size(st.resolution.0 as f32, st.resolution.1 as f32);
                }
            }
            Opt::Fullscreen => {
                st.fullscreen = !st.fullscreen;
                set_fullscreen(st.fullscreen);
                if !st.fullscreen {
                    request_new_screen_size(st.resolution.0 as f32, st.resolution.1 as f32);
                }
            }
            Opt::Vsync => {
                st.vsync = !st.vsync;
                self.note = Some(("VSync changes after a restart.".into(), 3.0));
            }
            Opt::FpsCap => {
                let i = FPS_CAPS.iter().position(|c| *c == st.fps_cap).map(|i| i as i32).unwrap_or(FPS_CAPS.len() as i32 - 1);
                st.fps_cap = FPS_CAPS[(i + dir).rem_euclid(FPS_CAPS.len() as i32) as usize];
            }
            Opt::Msaa => {
                let i = MSAA.iter().position(|m| *m == st.msaa).map(|i| i as i32).unwrap_or(2);
                st.msaa = MSAA[(i + dir).rem_euclid(MSAA.len() as i32) as usize];
                self.note = Some(("Anti-aliasing changes after a restart.".into(), 3.0));
            }
            Opt::Lighting => st.lighting = !st.lighting,
            Opt::LightQuality => st.light_quality = cycle_u8(st.light_quality, 3),
            Opt::Detail => st.detail_high = !st.detail_high,
            Opt::Particles => st.particles = cycle_u8(st.particles, 3),
            Opt::Gore => st.gore = !st.gore,
            Opt::Fog => st.fog = !st.fog,
            Opt::Grain => st.grain = !st.grain,
            Opt::Preset => {
                let i = PRESETS.iter().position(|p| p.1 == st.diff).map(|i| i as i32).unwrap_or(1);
                st.diff = PRESETS[(i + dir).rem_euclid(PRESETS.len() as i32) as usize].1;
            }
            Opt::Count => st.diff.count = step(&DIFF_COUNT, st.diff.count, dir),
            Opt::Hp => st.diff.hp = step(&DIFF_HP, st.diff.hp, dir),
            Opt::Dmg => st.diff.dmg = step(&DIFF_DMG, st.diff.dmg, dir),
            Opt::Speed => st.diff.speed = step(&DIFF_SPEED, st.diff.speed, dir),
            Opt::BossHp => st.diff.boss_hp = step(&DIFF_BOSS, st.diff.boss_hp, dir),
            Opt::Name | Opt::Back => {}
        }
    }

    /// What a row shows on the right: the name being edited, nothing, or a value to cycle.
    fn option_text(&self, opt: Opt) -> String {
        match opt {
            Opt::Name => {
                let caret = if (self.t * 2.0) as i32 % 2 == 0 { "_" } else { " " };
                format!("{}{}", self.save.settings.name, caret)
            }
            Opt::Back => String::new(),
            Opt::Bind(i) if self.binding == Some(i as usize) => "Press a key...".into(),
            Opt::Bind(i) => key_name(self.save.settings.binds[i as usize]).into(),
            Opt::ResetBinds => String::new(),
            _ => format!("<  {}  >", self.option_value(opt)),
        }
    }

    fn option_value(&self, opt: Opt) -> String {
        let st = &self.save.settings;
        let pct = |v: f32| format!("{:.0}%", v * 100.0);
        let onoff = |b: bool| if b { "On" } else { "Off" }.to_string();
        let level = |v: u8| ["Low", "Medium", "High"][v.min(2) as usize].to_string();
        match opt {
            Opt::Name | Opt::Back | Opt::Bind(_) | Opt::ResetBinds => String::new(),
            Opt::Music => pct(st.music),
            Opt::Sfx => pct(st.sfx),
            Opt::Numbers => onoff(st.numbers),
            Opt::Meter => onoff(st.meter),
            Opt::Overlay => onoff(st.overlay),
            Opt::Shake => onoff(st.shake),
            Opt::UiScale => match UI_SCALES[st.ui_scale as usize % UI_SCALES.len()] {
                0.0 => format!("Auto ({:.0}%)", crate::ui::ui_scale() * 100.0),
                s => format!("{:.0}%", s * 100.0),
            },
            Opt::AutoAim => onoff(st.auto_aim),
            Opt::Resolution => {
                let r = format!("{} x {}", st.resolution.0, st.resolution.1);
                if st.fullscreen {
                    format!("{} (windowed)", r)
                } else {
                    r
                }
            }
            Opt::Fullscreen => onoff(st.fullscreen),
            Opt::Vsync => onoff(st.vsync),
            Opt::FpsCap => {
                if st.fps_cap == 0 {
                    "Unlimited".into()
                } else {
                    format!("{} FPS", st.fps_cap)
                }
            }
            Opt::Msaa => {
                if st.msaa <= 1 {
                    "Off".into()
                } else {
                    format!("{}x MSAA", st.msaa)
                }
            }
            Opt::Lighting => onoff(st.lighting),
            Opt::LightQuality => level(st.light_quality),
            Opt::Detail => {
                if st.detail_high {
                    "High".into()
                } else {
                    "Low".into()
                }
            }
            Opt::Particles => level(st.particles),
            Opt::Gore => onoff(st.gore),
            Opt::Fog => onoff(st.fog),
            Opt::Grain => onoff(st.grain),
            Opt::Preset => st.diff.preset_name().to_string(),
            Opt::Count => pct(st.diff.count),
            Opt::Hp => pct(st.diff.hp),
            Opt::Dmg => pct(st.diff.dmg),
            Opt::Speed => pct(st.diff.speed),
            Opt::BossHp => pct(st.diff.boss_hp),
        }
    }

    pub(super) fn draw_options(&self, ui: &Ui) {
        let (sw, sh) = (ui_w(), ui_h());
        let cx = sw * 0.5;
        ui.spaced_c("OPTIONS", cx, 54.0, 40.0, 8.0, PALE);
        // Page tabs.
        let tab_w = 180.0;
        let x0 = cx - tab_w * PAGES.len() as f32 * 0.5;
        for (i, (name, _)) in PAGES.iter().enumerate() {
            let x = x0 + i as f32 * tab_w;
            let on = i == self.opt_page;
            draw_rectangle(x + 4.0, 74.0, tab_w - 8.0, 30.0, Color::new(0.07, 0.06, 0.06, if on { 0.95 } else { 0.5 }));
            if on {
                draw_rectangle(x + 4.0, 102.0, tab_w - 8.0, 2.0, GOLD);
            }
            ui.text_c(name, x + tab_w * 0.5, 96.0, 19.0, if on { GOLD } else { DIM });
        }
        let rows = self.option_rows();
        let w = 640.0f32.min(sw - 40.0);
        let x = cx - w * 0.5;
        let mut y = 120.0;
        for (i, opt) in rows.iter().enumerate() {
            if *opt == Opt::Preset {
                let st = &self.save.settings;
                ui.text(&format!("The host's settings apply online.   Souls and marks x{:.2}", st.diff.soul_mult()), x, y + 16.0, 15.0, GOLD);
                y += 26.0;
            }
            if *opt == Opt::Back {
                ui.row(x, y, w, 34.0, self.opt_sel == i);
                ui.text_c("Back", cx, y + 23.0, 20.0, EMBER);
            } else {
                value_row(ui, x, y, w, self.opt_sel == i, opt.label(), &self.option_text(*opt));
            }
            y += if rows.len() > 12 { 36.0 } else { 38.0 };
        }
        if let Some(hint) = rows.get(self.opt_sel).and_then(|o| o.hint()) {
            ui.text_c(hint, cx, y + 16.0, 15.0, DIM);
        }
        ui.text_c(hint("Q / E or Tab: page    W/S select    A/D change    Esc back", "LB / RB: page    Stick select and change    A set    B back"), cx, sh - 20.0, 15.0, DIM);
    }

    // ---------------------------------------------------------------- ash shrine

    pub(super) fn update_hub(&mut self) {
        drain_chars();
        if nav(&mut self.hub_sel, HUB_ROWS, false) {
            self.audio.play("select", 0.4);
        }
        if self.hub_sel == 0 {
            if left(false) {
                self.cycle_class(-1);
            }
            if right(false) {
                self.cycle_class(1);
            }
        }
        if back() {
            self.screen = Screen::Menu;
            return;
        }
        let on_challenge = (HUB_CHALLENGE_ROW..HUB_META_ROW).contains(&self.hub_sel);
        if on_challenge && (confirm() || left(false) || right(false)) {
            let ch = ALL_CHALLENGES[self.hub_sel - HUB_CHALLENGE_ROW];
            self.save.challenges ^= ch.bit();
            self.save.store();
            self.audio.play("select", 0.6);
            return;
        }
        if !confirm() {
            return;
        }
        if self.hub_sel == 0 || self.hub_sel == HUB_ROWS - 1 {
            self.start_single();
            return;
        }
        let i = self.hub_sel - HUB_META_ROW;
        let lvl = self.save.profile.meta[i];
        if lvl >= META[i].max {
            return;
        }
        let cost = meta_cost(i, lvl);
        if self.save.souls >= cost {
            self.save.souls -= cost;
            self.save.profile.meta[i] += 1;
            self.save.store();
            self.audio.play("kindle", 0.6);
        } else {
            self.audio.play("hurt", 0.3);
            self.set_note("Not enough souls.", 2.0);
        }
    }

    pub(super) fn draw_hub(&self, ui: &Ui) {
        let (sw, sh) = (ui_w(), ui_h());
        let cx = sw * 0.5;
        ui.spaced_c("THE ASH SHRINE", cx, 70.0, 40.0, 8.0, PALE);
        ui.text_c(&format!("Souls  {}", self.save.souls), cx, 104.0, 24.0, GOLD);
        if let Some((_, n)) = self.save.remnant {
            ui.text_c(&format!("Your remnant of {} souls waits where you last fell.", n), cx, 130.0, 16.0, TEAL);
        }
        let w = 660.0f32.min(sw - 40.0);
        let x = cx - w * 0.5;
        let mut y = 146.0;
        let class = self.class();
        ui.row(x, y, w, 34.0, self.hub_sel == 0);
        let col = if class == Class::BucketKnight { Color::new(0.75, 0.75, 0.8, 1.0) } else { PALE };
        ui.text(&format!("<  {}  >", class.name()), x + 14.0, y + 23.0, 20.0, col);
        ui.text_r(class.desc(), x + w - 14.0, y + 23.0, 16.0, DIM);
        y += 38.0;
        for (i, ch) in ALL_CHALLENGES.iter().enumerate() {
            let on = self.save.challenges & ch.bit() != 0;
            ui.row(x, y, w, 28.0, self.hub_sel == HUB_CHALLENGE_ROW + i);
            draw_rectangle_lines(x + 14.0, y + 8.0, 13.0, 13.0, 1.5, if on { BLOOD_RED } else { DIM });
            if on {
                draw_rectangle(x + 17.0, y + 11.0, 7.0, 7.0, BLOOD_RED);
            }
            ui.text(ch.name(), x + 36.0, y + 20.0, 17.0, if on { PALE } else { DIM });
            ui.text(ch.desc(), x + 245.0, y + 20.0, 15.0, DIM);
            ui.text_r("+25% marks", x + w - 14.0, y + 20.0, 15.0, if on { EMBER } else { DIM });
            y += 31.0;
        }
        y += 6.0;
        for (i, m) in META.iter().enumerate() {
            ui.row(x, y, w, 30.0, self.hub_sel == HUB_META_ROW + i);
            let lvl = self.save.profile.meta[i];
            ui.text(m.name, x + 14.0, y + 21.0, 19.0, PALE);
            for k in 0..m.max {
                let c = if k < lvl { GOLD } else { Color::new(0.25, 0.22, 0.2, 1.0) };
                draw_rectangle(x + 160.0 + k as f32 * 14.0, y + 10.0, 10.0, 10.0, c);
            }
            ui.text(m.desc, x + 245.0, y + 21.0, 16.0, DIM);
            let cost = if lvl >= m.max { "MAX".to_string() } else { format!("{}", meta_cost(i, lvl)) };
            let afford = lvl < m.max && self.save.souls >= meta_cost(i, lvl);
            ui.text_r(&cost, x + w - 14.0, y + 21.0, 18.0, if afford { GOLD } else { DIM });
            y += 33.0;
        }
        y += 6.0;
        ui.row(x, y, w, 34.0, self.hub_sel == HUB_ROWS - 1);
        ui.text_c(&format!("Begin the Vigil  ({})", self.save.settings.diff.preset_name()), cx, y + 24.0, 22.0, EMBER);
        ui.text_c(hint("W/S select    A/D change class    Enter buy / begin    Esc back", "Stick select, left / right class    A buy / begin    B back"), cx, sh - 24.0, 16.0, DIM);
    }

    // ---------------------------------------------------------------- reliquary

    pub(super) fn update_reliquary(&mut self) {
        drain_chars();
        let rows = UNLOCKS.len() + RELIC_COUNT + 1;
        if nav(&mut self.relic_sel, rows, false) {
            self.audio.play("select", 0.4);
        }
        if back() || (self.relic_sel == rows - 1 && confirm()) {
            self.screen = Screen::Menu;
            return;
        }
        if !confirm() {
            return;
        }
        let (cost, owned) = if self.relic_sel < UNLOCKS.len() {
            let (u, c) = UNLOCKS[self.relic_sel];
            (c, self.save.profile.has(u))
        } else {
            let i = self.relic_sel - UNLOCKS.len();
            let lvl = self.save.profile.relics[i];
            (relic_cost(i, lvl), lvl >= RELICS[i].max)
        };
        if owned {
            return;
        }
        if self.save.marks < cost {
            self.audio.play("hurt", 0.3);
            self.set_note("Not enough Ashen Marks. Every vigil earns some, win or lose.", 3.0);
            return;
        }
        self.save.marks -= cost;
        if self.relic_sel < UNLOCKS.len() {
            self.save.profile.unlocks |= UNLOCKS[self.relic_sel].0.bit();
        } else {
            self.save.profile.relics[self.relic_sel - UNLOCKS.len()] += 1;
        }
        self.save.store();
        self.audio.play("evolve", 0.6);
    }

    pub(super) fn draw_reliquary(&self, ui: &Ui) {
        let (sw, sh) = (ui_w(), ui_h());
        let cx = sw * 0.5;
        ui.spaced_c("THE RELIQUARY", cx, 56.0, 40.0, 8.0, PALE);
        ui.text_c(&format!("Ashen Marks  {}", self.save.marks), cx, 88.0, 22.0, EMBER);
        let w = 820.0f32.min(sw - 40.0);
        let x = cx - w * 0.5;
        let mut y = 104.0;
        let row_h = 30.0;
        for (i, (u, cost)) in UNLOCKS.iter().enumerate() {
            let on = self.relic_sel == i;
            let owned = self.save.profile.has(*u);
            ui.row(x, y, w, row_h - 3.0, on);
            ui.text(&u.label(), x + 12.0, y + 19.0, 17.0, if owned { DIM } else { PALE });
            ui.text(u.desc(), x + 300.0, y + 19.0, 14.0, DIM);
            let price = if owned { "OWNED".to_string() } else { format!("{}", cost) };
            let col = if owned {
                TEAL
            } else if self.save.marks >= *cost {
                EMBER
            } else {
                DIM
            };
            ui.text_r(&price, x + w - 12.0, y + 19.0, 16.0, col);
            y += row_h;
        }
        y += 8.0;
        for (i, r) in RELICS.iter().enumerate() {
            let row = UNLOCKS.len() + i;
            let lvl = self.save.profile.relics[i];
            ui.row(x, y, w, row_h - 3.0, self.relic_sel == row);
            ui.text(r.name, x + 12.0, y + 19.0, 17.0, PALE);
            for k in 0..r.max {
                let c = if k < lvl { EMBER } else { Color::new(0.25, 0.22, 0.2, 1.0) };
                draw_rectangle(x + 170.0 + k as f32 * 13.0, y + 9.0, 9.0, 9.0, c);
            }
            ui.text(r.desc, x + 300.0, y + 19.0, 14.0, DIM);
            let maxed = lvl >= r.max;
            let price = if maxed { "MAX".to_string() } else { format!("{}", relic_cost(i, lvl)) };
            let col = if maxed {
                TEAL
            } else if self.save.marks >= relic_cost(i, lvl) {
                EMBER
            } else {
                DIM
            };
            ui.text_r(&price, x + w - 12.0, y + 19.0, 16.0, col);
            y += row_h;
        }
        y += 6.0;
        ui.row(x, y, w, row_h - 3.0, self.relic_sel == UNLOCKS.len() + RELIC_COUNT);
        ui.text_c("Back", cx, y + 20.0, 18.0, DIM);
        ui.text_c("Ashen Marks are earned at the end of every vigil: time, bosses, level and kills count.", cx, sh - 44.0, 15.0, DIM);
        ui.text_c(hint("W/S select    Enter buy    Esc back", "Stick select    A buy    B back"), cx, sh - 20.0, 15.0, DIM);
    }
}
