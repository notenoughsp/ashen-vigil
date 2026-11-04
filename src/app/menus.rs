//! Main menu, options, the Ash Shrine (souls) and the Reliquary (ashen marks).

use super::*;
use crate::save::{bindable, key_name, Action, ACTIONS};
use crate::save::{FPS_CAPS, MSAA, RESOLUTIONS};
use crate::ui::{ui_h, ui_w, BLOOD as BLOOD_RED, EMBER, GOLD, TEAL, UI_SCALES};

const MENU_ITEMS: [&str; 8] = ["Start", "Online Lobby", "Reliquary", "Wardrobe", "Records", "Options", "Credits", "Exit"];
/// Columns of the soul upgrades in the Reliquary.
const SHOP_COLS: usize = 3;
/// Ash Shrine rows: class, the challenges, begin.
const HUB_CHALLENGE_ROW: usize = 1;
const HUB_ROWS: usize = HUB_CHALLENGE_ROW + ALL_CHALLENGES.len() + 1;

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
    Cute,
    Bind(u8),
    ResetBinds,
    Markers,
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
            Opt::Cute => "Cute mode",
            Opt::Bind(i) => ACTIONS[i as usize].label(),
            Opt::ResetBinds => "Reset to defaults",
            Opt::Markers => "Off-screen markers",
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
            Opt::Markers => "How far inside the screen edge the arrows to things off screen sit.",
            Opt::Vsync => "Syncs to the monitor refresh. Takes effect after a restart.",
            Opt::Bind(_) => "Enter, then press the new key. Arrow keys, Shift and Esc always work too.",
            Opt::Cute => "Pink instead of blood, hearts instead of bullets, and everyone blushes.",
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
        &[Opt::Bind(0), Opt::Bind(1), Opt::Bind(2), Opt::Bind(3), Opt::Bind(4), Opt::Bind(5), Opt::Bind(6), Opt::Bind(7), Opt::Bind(8), Opt::Bind(9), Opt::ResetBinds, Opt::Back],
    ),
    ("General", &[Opt::Name, Opt::Music, Opt::Sfx, Opt::Numbers, Opt::Meter, Opt::Overlay, Opt::Shake, Opt::UiScale, Opt::Markers, Opt::Cute, Opt::Back]),
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

/// A price in a few characters: 950, 12.5K, 1.20M.
pub(super) fn short_num(v: u64) -> String {
    match v {
        0..=9_999 => v.to_string(),
        10_000..=999_999 => format!("{:.1}K", v as f64 / 1e3),
        _ => format!("{:.2}M", v as f64 / 1e6),
    }
}

/// A volume one percent up or down.
fn percent_step(v: f32, dir: i32) -> f32 {
    ((v * 100.0).round() + dir as f32).clamp(0.0, 100.0) / 100.0
}

/// Where the main menu rows sit, for drawing and for the mouse.
fn menu_rows() -> Vec<Rect> {
    let (sw, sh) = (ui_w(), ui_h());
    (0..MENU_ITEMS.len()).map(|i| Rect::new(sw * 0.5 - 160.0, sh * 0.36 + i as f32 * 42.0, 320.0, 36.0)).collect()
}

/// A selectable row with a label on the left and a value on the right.
fn value_row(ui: &Ui, x: f32, y: f32, w: f32, on: bool, label: &str, value: &str) {
    ui.row(x, y, w, 34.0, on);
    ui.text(label, x + 14.0, y + 23.0, 19.0, PALE);
    ui.text_r(value, x + w - 14.0, y + 23.0, 18.0, if on { GOLD } else { DIM });
}

impl App {
    pub(super) fn update_menu(&mut self) {
        self.audio.music(self.menu_music());
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
                self.unlock(Achievement::Clang);
                self.set_note("A bucket clangs somewhere in the dark... The Bucket Knight joins the vigil.", 6.0);
            }
        }
        if nav(&mut self.menu_sel, MENU_ITEMS.len(), false) {
            self.audio.play("select", 0.4);
        }
        // Seven clicks on the title stir the embers.
        let title = Rect::new(ui_w() * 0.5 - 320.0, ui_h() * 0.27 - 70.0, 640.0, 90.0);
        if clicked() && title.contains(mouse_ui()) {
            self.title_clicks += 1;
            self.audio.play("chime", 0.3 + 0.05 * self.title_clicks as f32);
            if self.title_clicks >= 7 {
                self.title_clicks = 0;
                self.save.blue_embers = !self.save.blue_embers;
                self.save.store();
                self.unlock(Achievement::ColdEmbers);
                self.audio.play("bell", 0.8);
                let msg = if self.save.blue_embers { "The embers burn cold and blue. Something old remembers you." } else { "The embers warm again." };
                self.set_note(msg, 5.0);
            }
        }
        // Secret words typed in the menu.
        while let Some(c) = get_char_pressed() {
            self.typed.push(c.to_ascii_lowercase());
            if self.typed.len() > 12 {
                self.typed.remove(0);
            }
            if self.typed.ends_with("cute") {
                self.typed.clear();
                let st = &mut self.save.settings;
                st.cute = !st.cute;
                self.save.cute_found = true;
                self.save.store();
                self.unlock(Achievement::SoftHeart);
                self.audio.play("meow", 0.8);
                let msg = if self.save.settings.cute { "Everything is soft now. (Options > General: Cute mode)" } else { "The dark returns, a little reluctantly." };
                self.set_note(msg, 5.0);
            } else if self.typed.ends_with("meow") {
                self.typed.clear();
                self.menu_cat = 6.0;
                self.audio.play("meow", 0.9);
                self.unlock(Achievement::Purrfect);
            } else if self.typed.ends_with("dawn") {
                self.typed.clear();
                self.audio.play("bell", 0.6);
                self.set_note("Not yet. The night is long, and the dawn has to be earned.", 5.0);
                self.unlock(Achievement::Impatient);
            } else if self.typed.ends_with("party") {
                self.typed.clear();
                self.party = !self.party;
                self.audio.play("levelup", 0.7);
                let msg = if self.party { "The embers put on their party hats." } else { "The party winds down. Someone has to clean up." };
                self.set_note(msg, 4.0);
                self.unlock(Achievement::Partygoer);
            }
        }
        self.menu_cat = (self.menu_cat - get_frame_time()).max(0.0);
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
                self.relic_page = 0;
            }
            3 => {
                self.screen = Screen::Wardrobe;
                self.wardrobe_sel = self.save.profile.skin as usize;
            }
            4 => {
                self.screen = Screen::Records;
                self.records_page = 0;
            }
            5 => {
                self.screen = Screen::Options;
                self.opt_sel = 0;
            }
            6 => {
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
        let motto = if self.save.settings.cute { "hold hands until dawn" } else { "hold the dark at bay until dawn" };
        ui.text_c(motto, sw * 0.5, sh * 0.27 + 44.0, 20.0, DIM);
        if self.menu_cat > 0.0 {
            // A black cat strolls along the bottom of the menu and is gone.
            let k = 1.0 - self.menu_cat / 6.0;
            let (cx, cy) = (-40.0 + k * (sw + 80.0), sh - 96.0);
            let step = (self.t * 10.0).sin() * 2.0;
            let black = Color::new(0.05, 0.04, 0.05, 1.0);
            draw_ellipse(cx, cy, 18.0, 9.0, 0.0, black);
            draw_circle(cx + 17.0, cy - 8.0, 8.0, black);
            draw_triangle(vec2(cx + 12.0, cy - 13.0), vec2(cx + 16.0, cy - 22.0), vec2(cx + 19.0, cy - 13.0), black);
            draw_triangle(vec2(cx + 18.0, cy - 13.0), vec2(cx + 23.0, cy - 21.0), vec2(cx + 25.0, cy - 11.0), black);
            draw_line(cx - 16.0, cy - 2.0, cx - 26.0, cy - 16.0 + step, 3.0, black);
            for (dx, ph) in [(-10.0, 0.0), (-4.0, 1.0), (6.0, 0.5), (12.0, 1.5)] {
                draw_line(cx + dx, cy + 5.0, cx + dx + (self.t * 10.0 + ph).sin() * 3.0, cy + 14.0, 2.5, black);
            }
            draw_circle(cx + 20.0, cy - 9.0, 1.4, Color::new(0.9, 0.8, 0.3, 1.0));
        }
        for (i, (item, r)) in MENU_ITEMS.iter().zip(menu_rows()).enumerate() {
            let on = self.menu_sel == i;
            ui.row(r.x, r.y, r.w, r.h, on);
            ui.text_c(item, sw * 0.5, r.y + 26.0, 24.0, if on { GOLD } else { PALE });
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
        self.credits_scroll -= mouse_wheel().1 * 40.0;
        self.credits_scroll = self.credits_scroll.clamp(0.0, max);
        if back() || confirm() || clicked() {
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

    /// The rows of the current page; the cute option only once it has been found.
    fn option_rows(&self) -> Vec<Opt> {
        PAGES[self.opt_page.min(PAGES.len() - 1)].1.iter().copied().filter(|o| *o != Opt::Cute || self.save.cute_found).collect()
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
        if let Some(t) = clicked_tab() {
            if t != self.opt_page {
                self.opt_page = t;
                self.opt_sel = 0;
                self.audio.play("select", 0.5);
                return;
            }
        }
        let rows = self.option_rows();
        if nav(&mut self.opt_sel, rows.len(), on_name) {
            self.audio.play("select", 0.4);
        }
        let mouse = mouse_rows(&mut self.opt_sel);
        let opt = rows[self.opt_sel.min(rows.len() - 1)];
        if opt == Opt::Name {
            edit_text(&mut self.save.settings.name, 16, name_char);
        } else {
            drain_chars();
        }
        let on_back = opt == Opt::Back;
        let dir = if left(on_name) || mouse.right {
            -1
        } else if right(on_name) || (!on_name && !on_back && (confirm() || mouse.left)) {
            1
        } else {
            0
        };
        if matches!(opt, Opt::Music | Opt::Sfx | Opt::Markers) {
            let held = if is_key_down(KeyCode::Left) || is_key_down(KeyCode::A) || pad_stick().x < -0.5 {
                -1
            } else if is_key_down(KeyCode::Right) || is_key_down(KeyCode::D) || pad_stick().x > 0.5 {
                1
            } else {
                0
            };
            self.opt_hold = if held != 0 { self.opt_hold + get_frame_time() } else { 0.0 };
            let wheel = mouse_wheel().1;
            if dir == 0 && self.opt_hold > 0.35 {
                self.change_option(opt, held);
            } else if dir == 0 && wheel != 0.0 && hovered(&crate::ui::rows_drawn()).is_some() {
                self.change_option(opt, wheel.signum() as i32);
            }
        }
        if let Opt::Bind(i) = opt {
            if confirm() || mouse.left {
                self.binding = Some(i as usize);
                self.audio.play("select", 0.5);
            }
        } else if dir != 0 && opt != Opt::Name && !on_back {
            self.change_option(opt, dir);
            self.audio.play("select", 0.5);
        }
        if back() || (on_back && (confirm() || mouse.left)) {
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
                st.music = percent_step(st.music, dir);
                self.audio.set_music_volume(st.music);
            }
            Opt::Sfx => {
                st.sfx = percent_step(st.sfx, dir);
                self.audio.sfx_vol = st.sfx;
            }
            Opt::Numbers => st.numbers = !st.numbers,
            Opt::Meter => st.meter = !st.meter,
            Opt::Overlay => st.overlay = !st.overlay,
            Opt::Shake => st.shake = !st.shake,
            Opt::ResetBinds => st.binds = ACTIONS.map(Action::default_key),
            Opt::Bind(_) => {}
            Opt::UiScale => st.ui_scale = (st.ui_scale as i32 + dir).rem_euclid(UI_SCALES.len() as i32) as u8,
            Opt::Markers => st.marker_inset = (st.marker_inset as i32 + dir).clamp(0, crate::save::MAX_MARKER_INSET as i32) as u8,
            Opt::Cute => st.cute = !st.cute,
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
            Opt::Markers => match st.marker_inset {
                0 => "At the edge".into(),
                v => format!("{}% inside", v),
            },
            Opt::Cute => onoff(st.cute),
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
            crate::ui::register_tab(Rect::new(x + 4.0, 74.0, tab_w - 8.0, 30.0));
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
        let mouse = mouse_rows(&mut self.hub_sel);
        // Clicking the class row picks a class (right click: the other way); it does not start.
        if self.hub_sel == 0 && (mouse.left || mouse.right) {
            self.cycle_class(if mouse.right { -1 } else { 1 });
            return;
        }
        let on_challenge = (HUB_CHALLENGE_ROW..HUB_ROWS - 1).contains(&self.hub_sel);
        if on_challenge && (confirm() || left(false) || right(false) || mouse.left || mouse.right) {
            let ch = ALL_CHALLENGES[self.hub_sel - HUB_CHALLENGE_ROW];
            self.save.challenges ^= ch.bit();
            self.save.store();
            self.audio.play("select", 0.6);
            return;
        }
        if confirm() || mouse.left {
            self.start_single();
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
        let w = 1100.0f32.min(sw - 40.0);
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
        y += 10.0;
        ui.row(x, y, w, 34.0, self.hub_sel == HUB_ROWS - 1);
        ui.text_c(&format!("Begin the Vigil  ({})", self.save.settings.diff.preset_name()), cx, y + 24.0, 22.0, EMBER);
        ui.text_c("Permanent upgrades are bought in the Reliquary.", cx, y + 64.0, 15.0, DIM);
        ui.text_c(
            hint("W/S select    A/D change class    Enter toggle / begin    Esc back", "Stick select, left / right class    A toggle / begin    B back"),
            cx,
            sh - 24.0,
            16.0,
            DIM,
        );
    }

    // ---------------------------------------------------------------- reliquary

    /// Rows on the current Reliquary page, Back included.
    fn reliquary_rows(&self) -> usize {
        if self.relic_page == 0 {
            META_COUNT + 1
        } else {
            UNLOCKS.len() + RELIC_COUNT + 1
        }
    }

    pub(super) fn update_reliquary(&mut self) {
        drain_chars();
        // Two pages: soul upgrades and the things Ashen Marks buy.
        let page_dir = if is_key_pressed(KeyCode::Tab) || is_key_pressed(KeyCode::E) || is_key_pressed(KeyCode::PageDown) || pad_pressed(Pad::PageNext) {
            1
        } else if is_key_pressed(KeyCode::Q) || is_key_pressed(KeyCode::PageUp) || pad_pressed(Pad::PagePrev) {
            -1
        } else {
            0
        };
        let clicked_page = clicked_tab().filter(|t| *t != self.relic_page);
        if page_dir != 0 || clicked_page.is_some() {
            self.relic_page = clicked_page.unwrap_or((self.relic_page as i32 + page_dir).rem_euclid(2) as usize);
            self.relic_sel = 0;
            self.audio.play("select", 0.5);
            return;
        }
        let rows = self.reliquary_rows();
        let moved = if self.relic_page == 0 {
            grid_nav(&mut self.relic_sel, 0, META_COUNT, SHOP_COLS)
        } else {
            grid_nav(&mut self.relic_sel, 0, UNLOCKS.len(), 2) || grid_nav(&mut self.relic_sel, UNLOCKS.len(), RELIC_COUNT, 2)
        };
        if moved || nav(&mut self.relic_sel, rows, false) {
            self.audio.play("select", 0.4);
        }
        let click = mouse_rows(&mut self.relic_sel).left;
        if back() || (self.relic_sel == rows - 1 && (confirm() || click)) {
            self.screen = Screen::Menu;
            return;
        }
        if !confirm() && !click {
            return;
        }
        if self.relic_page == 0 {
            self.buy_upgrade(self.relic_sel);
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

    /// One more level of soul upgrade `i`, if the souls are there.
    fn buy_upgrade(&mut self, i: usize) {
        let lvl = self.save.profile.meta[i];
        if lvl >= META[i].max {
            return;
        }
        let cost = meta_cost(i, lvl);
        if self.save.souls < cost {
            self.audio.play("hurt", 0.3);
            self.set_note("Not enough souls. Bank them at shrines during a vigil.", 2.5);
            return;
        }
        self.save.souls -= cost;
        self.save.profile.meta[i] += 1;
        self.save.shrine_levels += 1;
        self.save.store();
        self.audio.play("kindle", 0.6);
        if self.save.shrine_levels >= 50 {
            self.unlock(Achievement::Devoted);
        }
    }

    pub(super) fn draw_reliquary(&self, ui: &Ui) {
        let (sw, sh) = (ui_w(), ui_h());
        let cx = sw * 0.5;
        ui.spaced_c("THE RELIQUARY", cx, 50.0, 40.0, 8.0, PALE);
        ui.text_c(&format!("Souls  {}        Ashen Marks  {}", self.save.souls, self.save.marks), cx, 80.0, 20.0, if self.relic_page == 0 { GOLD } else { EMBER });
        // Page tabs.
        let tab_w = 260.0;
        for (i, name) in ["Soul Upgrades", "Relics and Unlocks"].iter().enumerate() {
            let tx = cx - tab_w + i as f32 * tab_w;
            crate::ui::register_tab(Rect::new(tx + 4.0, 94.0, tab_w - 8.0, 30.0));
            let on = i == self.relic_page;
            draw_rectangle(tx + 4.0, 94.0, tab_w - 8.0, 30.0, Color::new(0.07, 0.06, 0.06, if on { 0.95 } else { 0.5 }));
            if on {
                draw_rectangle(tx + 4.0, 122.0, tab_w - 8.0, 2.0, GOLD);
            }
            ui.text_c(name, tx + tab_w * 0.5, 116.0, 19.0, if on { GOLD } else { DIM });
        }
        let w = 1100.0f32.min(sw - 40.0);
        let x = cx - w * 0.5;
        let top = 138.0;
        if self.relic_page == 0 {
            self.draw_upgrades(ui, x, top, w);
        } else {
            self.draw_relics(ui, x, top, w);
        }
        let foot = if self.relic_page == 0 {
            "Souls are banked at shrines during a vigil. Upgrades last forever."
        } else {
            "Ashen Marks are earned at the end of every vigil: time, bosses, level and kills count."
        };
        ui.text_c(foot, cx, sh - 44.0, 15.0, DIM);
        ui.text_c(hint("Q / E or Tab: page    W/S/A/D select    Enter buy    Esc back", "LB / RB: page    Stick select    A buy    B back"), cx, sh - 20.0, 15.0, DIM);
    }

    /// The soul upgrades in three columns, the selected one explained below.
    fn draw_upgrades(&self, ui: &Ui, x: f32, top: f32, w: f32) {
        let cx = x + w * 0.5;
        let gap = 6.0;
        let cw = (w - gap * (SHOP_COLS - 1) as f32) / SHOP_COLS as f32;
        for (i, m) in META.iter().enumerate() {
            let (rx, ry) = (x + (i % SHOP_COLS) as f32 * (cw + gap), top + (i / SHOP_COLS) as f32 * 31.0);
            ui.row(rx, ry, cw, 28.0, self.relic_sel == i);
            let lvl = self.save.profile.meta[i];
            ui.text(m.name, rx + 10.0, ry + 20.0, 16.0, PALE);
            for k in 0..m.max {
                let c = if k < lvl { GOLD } else { Color::new(0.25, 0.22, 0.2, 1.0) };
                draw_rectangle(rx + 132.0 + k as f32 * 8.0, ry + 11.0, 5.0, 5.0, c);
            }
            let cost = if lvl >= m.max { "MAX".to_string() } else { short_num(meta_cost(i, lvl)) };
            let afford = lvl < m.max && self.save.souls >= meta_cost(i, lvl);
            ui.text_r(&cost, rx + cw - 10.0, ry + 20.0, 15.0, if afford { GOLD } else { DIM });
        }
        let mut y = top + META_COUNT.div_ceil(SHOP_COLS) as f32 * 31.0 + 4.0;
        let about = match META.get(self.relic_sel) {
            Some(m) => format!("{}: {} per level   (level {} of {})", m.name, m.desc, self.save.profile.meta[self.relic_sel], m.max),
            None => String::new(),
        };
        ui.text_c(&about, cx, y + 14.0, 15.0, PALE);
        y += 24.0;
        ui.row(x, y, w, 27.0, self.relic_sel == META_COUNT);
        ui.text_c("Back", cx, y + 20.0, 18.0, DIM);
    }

    /// Unlocks and relics, bought with Ashen Marks.
    fn draw_relics(&self, ui: &Ui, x: f32, top: f32, w: f32) {
        let cx = x + w * 0.5;
        let mut y = top;
        let row_h = 30.0;
        let cw = (w - 8.0) * 0.5;
        let cell = |i: usize, y0: f32| (x + (i % 2) as f32 * (cw + 8.0), y0 + (i / 2) as f32 * row_h);
        for (i, (u, cost)) in UNLOCKS.iter().enumerate() {
            let (rx, ry) = cell(i, y);
            let owned = self.save.profile.has(*u);
            ui.row(rx, ry, cw, row_h - 3.0, self.relic_sel == i);
            ui.text(&u.label(), rx + 10.0, ry + 19.0, 16.0, if owned { DIM } else { PALE });
            let price = if owned { "OWNED".to_string() } else { format!("{}", cost) };
            let col = if owned {
                TEAL
            } else if self.save.marks >= *cost {
                EMBER
            } else {
                DIM
            };
            ui.text_r(&price, rx + cw - 10.0, ry + 19.0, 15.0, col);
        }
        y += UNLOCKS.len().div_ceil(2) as f32 * row_h + 8.0;
        for (i, r) in RELICS.iter().enumerate() {
            let (rx, ry) = cell(i, y);
            let row = UNLOCKS.len() + i;
            let lvl = self.save.profile.relics[i];
            ui.row(rx, ry, cw, row_h - 3.0, self.relic_sel == row);
            ui.text(r.name, rx + 10.0, ry + 19.0, 16.0, PALE);
            for k in 0..r.max {
                let c = if k < lvl { EMBER } else { Color::new(0.25, 0.22, 0.2, 1.0) };
                draw_rectangle(rx + 150.0 + k as f32 * 11.0, ry + 10.0, 8.0, 8.0, c);
            }
            ui.text(r.desc, rx + 220.0, ry + 18.0, 12.0, DIM);
            let maxed = lvl >= r.max;
            let price = if maxed { "MAX".to_string() } else { format!("{}", relic_cost(i, lvl)) };
            let col = if maxed {
                TEAL
            } else if self.save.marks >= relic_cost(i, lvl) {
                EMBER
            } else {
                DIM
            };
            ui.text_r(&price, rx + cw - 10.0, ry + 19.0, 15.0, col);
        }
        y += RELIC_COUNT.div_ceil(2) as f32 * row_h + 6.0;
        ui.row(x, y, w, row_h - 3.0, self.relic_sel == UNLOCKS.len() + RELIC_COUNT);
        ui.text_c("Back", cx, y + 20.0, 18.0, DIM);
        // What the selected unlock gives; relic rows say it themselves.
        if let Some((u, _)) = UNLOCKS.get(self.relic_sel) {
            ui.text_c(u.desc(), cx, y + 58.0, 17.0, PALE);
        }
    }
}
