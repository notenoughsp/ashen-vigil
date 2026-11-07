//! Main menu, options, the Ash Shrine (souls) and the Reliquary (ashen marks).

use super::*;
use crate::save::{bindable, key_name, Action, ACTIONS};
use crate::save::{FPS_CAPS, MSAA, RESOLUTIONS};
use crate::ui::{ui_h, ui_w, BLOOD as BLOOD_RED, EMBER, GOLD, TEAL, UI_SCALES};

const MENU_ITEMS: [&str; 8] = ["Start", "Online Lobby", "Reliquary", "Wardrobe", "Records", "Options", "Credits", "Exit"];
/// Columns of the soul upgrades in the Reliquary.
const SHOP_COLS: usize = 3;
/// Ash Shrine rows: class, mode, field, the challenges, begin.
const HUB_MODE_ROW: usize = 1;
const HUB_BIOME_ROW: usize = 2;
const HUB_DIFF_ROW: usize = 3;
const HUB_BOOST_ROW: usize = 4;
const HUB_CHALLENGE_ROW: usize = HUB_BOOST_ROW + 3;
const HUB_CURSE_ROW: usize = HUB_CHALLENGE_ROW + ALL_CHALLENGES.len();
const HUB_ROWS: usize = HUB_CURSE_ROW + ALL_CURSES.len() + 1;

/// One row on the options screen.
#[derive(Clone, Copy, PartialEq, Debug)]
enum Opt {
    Name,
    Music,
    /// The lords' songs, as a share of the music volume.
    BossMusic,
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
    PauseUnfocused,
    MuteUnfocused,
    HurtFlash,
    HeroHp,
    HeroStamina,
    BossSongOut,
    Discord,
    Updates,
    /// Volume of one of `audio::SOUND_KINDS`.
    Vol(u8),
    FxAlpha,
    Brightness,
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
            Opt::BossMusic => "Boss music",
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
            Opt::PauseUnfocused => "Pause when not in focus",
            Opt::MuteUnfocused => "Mute when not in focus",
            Opt::HurtFlash => "Flash when hit",
            Opt::HeroHp => "Health bar under hero",
            Opt::HeroStamina => "Stamina bar under hero",
            Opt::BossSongOut => "Boss song plays out",
            Opt::Discord => "Discord status",
            Opt::Updates => "Check for updates",
            Opt::Vol(i) => crate::audio::SOUND_KINDS[i as usize].1,
            Opt::FxAlpha => "Weapon and skill opacity",
            Opt::Brightness => "Brightness",
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
            Opt::PauseUnfocused => "A single-player vigil pauses when you switch to another window. Online the world never waits.",
            Opt::MuteUnfocused => "No sound while another window has the focus.",
            Opt::HurtFlash => "The screen edge flashes red when your hero is hit.",
            Opt::HeroHp => "A small health bar right under your hero, so you need not look to the corner.",
            Opt::HeroStamina => "A small stamina bar under your hero: how many rolls are left.",
            Opt::BossSongOut => "Once the lords have fallen, a recorded boss song plays to its end before the field's music comes back. Off: it fades out at once.",
            Opt::Discord => "Friends on Discord see what you are playing: menus, lobby, vigil or boss rush.",
            Opt::Updates => "At start, ask GitHub whether a newer version is out. The main menu then offers to update.",
            Opt::Vol(0) => "Foes falling, lords roaring, the bell before a lord.",
            Opt::Vol(_) => "Menus and everything else.",
            Opt::BossMusic => "The lords' songs, as a share of the music volume. The boss rush keeps the music volume.",
            Opt::Sfx => "All sound effects: weapons, hits and pickups directly, and the kinds below as shares of it.",
            Opt::FxAlpha => "Lower it to see foes and lords' attacks through your own weapons and skills.",
            Opt::Brightness => "How dark the night is away from the fires.",
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

const PAGES: [(&str, &[Opt]); 5] = [
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
    (
        "General",
        &[
            Opt::Name,
            Opt::Numbers,
            Opt::Meter,
            Opt::Overlay,
            Opt::Shake,
            Opt::HurtFlash,
            Opt::HeroHp,
            Opt::HeroStamina,
            Opt::UiScale,
            Opt::Markers,
            Opt::PauseUnfocused,
            Opt::Discord,
            Opt::Updates,
            Opt::Cute,
            Opt::Back,
        ],
    ),
    ("Sound", &[Opt::Music, Opt::BossMusic, Opt::Sfx, Opt::Vol(0), Opt::Vol(1), Opt::BossSongOut, Opt::MuteUnfocused, Opt::Back]),
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
            Opt::Brightness,
            Opt::FxAlpha,
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

/// The Reliquary's offerings: souls (true) or Ashen Marks, and how many (0 for everything).
const OFFERINGS: [(bool, u64); 7] = [(true, 1_000), (true, 10_000), (true, 100_000), (true, 0), (false, 10), (false, 100), (false, 0)];

/// Where the notice about a newer version sits: the top right of the main menu.
fn update_notice() -> Rect {
    Rect::new(ui_w() - 356.0, 16.0, 340.0, 58.0)
}

/// What is new in this version, in a few words each, shown once after an update.
const WHATS_NEW: [&str; 9] = [
    "Recorded songs for every field, the boss rush and each lord.",
    "A lord's song begins ten seconds before the lord arrives.",
    "New option: Boss music, its own volume.",
    "New option: Boss song plays out after the fight.",
    "Weapons, hits and pickups now follow the sound effects volume.",
    "The lords no longer speak aloud; their words still show.",
    "A new secret word waits in the main menu.",
    "Updates let you pick the build with songs or the small synth one.",
    "Fixed: the lords' adds no longer pile up in long endless nights.",
];

/// The button on the left of the main menu that folds out what is new.
fn whats_new_button() -> Rect {
    Rect::new(16.0, 16.0, 300.0, 40.0)
}

/// The two buttons under the notice while a newer version is on offer: with the recorded
/// songs, and with the synthesized music only.
fn update_buttons() -> [Rect; 2] {
    let r = update_notice();
    let w = (r.w - 8.0) * 0.5;
    [Rect::new(r.x, r.y + r.h + 6.0, w, 46.0), Rect::new(r.x + w + 8.0, r.y + r.h + 6.0, w, 46.0)]
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
        // What is new: a click folds it out or in.
        if let Some(open) = self.whats_new {
            if clicked() && whats_new_button().contains(mouse_ui()) {
                self.whats_new = Some(!open);
                self.audio.play("select", 0.5);
                return;
            }
        }
        // A newer version: pick a build under the notice (or Y with songs, X without on a pad),
        // then click the notice to restart.
        use crate::update::State;
        let state = self.updater.state();
        if matches!(state, State::Available { .. }) {
            let [with, without] = update_buttons();
            let (songs, synth) = self.updater.choices();
            let pick = if (clicked() && with.contains(mouse_ui())) || pad_pressed(Pad::Reroll) {
                songs.map(|_| true)
            } else if (clicked() && without.contains(mouse_ui())) || pad_pressed(Pad::Skill) {
                synth.map(|_| false)
            } else {
                None
            };
            if let Some(with_songs) = pick {
                self.audio.play("select", 0.6);
                self.updater.install(with_songs);
                return;
            }
        } else if matches!(state, State::Ready { .. } | State::Failed(_)) && ((clicked() && update_notice().contains(mouse_ui())) || pad_pressed(Pad::Reroll)) {
            self.audio.play("select", 0.6);
            if matches!(state, State::Ready { .. }) {
                self.save.store();
                self.updater.restart();
            } else {
                self.updater.retry();
            }
            return;
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
            } else if self.typed.ends_with("encore") {
                self.typed.clear();
                if self.audio.play_secret() {
                    self.set_note("The bard of ash clears its throat. One song, then the night goes on.", 5.0);
                    self.unlock(Achievement::Encore);
                } else {
                    self.set_note("No one is here to sing.", 3.0);
                }
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
                (self.wardrobe_page, self.wardrobe_sel) = wardrobe::wardrobe_spot(self.save.profile.skin as usize);
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
        self.draw_update_notice(ui);
        self.draw_whats_new(ui);
    }

    /// The fold-out list of what is new, on the left of the main menu.
    fn draw_whats_new(&self, ui: &Ui) {
        let Some(open) = self.whats_new else { return };
        let b = whats_new_button();
        let hot = b.contains(mouse_ui());
        draw_rectangle(b.x, b.y, b.w, b.h, Color::new(0.07, 0.06, 0.06, if hot { 0.95 } else { 0.75 }));
        draw_rectangle_lines(b.x, b.y, b.w, b.h, 1.5, if hot { GOLD } else { crate::ui::EMBER });
        let arrow = if open { "-" } else { "+" };
        ui.text(&format!("{}  What's new in v{}", arrow, crate::net::VERSION), b.x + 14.0, b.y + 27.0, 18.0, GOLD);
        if !open {
            return;
        }
        // Narrow enough to leave the title free at 1280 wide.
        let w = 300.0;
        let lines: Vec<String> = WHATS_NEW.iter().flat_map(|l| ui.wrap(&format!("- {}", l), 15.0, w - 28.0)).collect();
        let h = 20.0 + lines.len() as f32 * 21.0;
        let (x, y) = (b.x, b.y + b.h + 6.0);
        draw_rectangle(x, y, w, h, Color::new(0.07, 0.06, 0.06, 0.88));
        draw_rectangle_lines(x, y, w, h, 1.5, crate::ui::EMBER);
        for (i, l) in lines.iter().enumerate() {
            ui.text(l, x + 14.0, y + 26.0 + i as f32 * 21.0, 15.0, WHITE);
        }
    }

    /// The corner notice about a newer version and how far its update has come.
    fn draw_update_notice(&self, ui: &Ui) {
        use crate::update::State;
        let (title, sub) = match self.updater.state() {
            State::Idle => return,
            State::Available { version } => (format!("Version {} is out", version), hint("Pick a build below to update", "Y with songs, X without").to_string()),
            State::Downloading { version } => (format!("Downloading {}...", version), "This takes a moment.".to_string()),
            State::Ready { version } => (format!("Version {} is ready", version), hint("Click here to restart", "Y to restart").to_string()),
            State::Failed(why) => ("The update failed".to_string(), format!("{}. {}", why.chars().take(48).collect::<String>(), hint("Click to retry", "Y to retry"))),
        };
        let r = update_notice();
        let hot = r.contains(mouse_ui());
        draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.07, 0.06, 0.06, if hot { 0.95 } else { 0.75 }));
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.5, if hot { GOLD } else { crate::ui::EMBER });
        ui.text_c(&title, r.x + r.w * 0.5, r.y + 24.0, 20.0, GOLD);
        ui.text_c(&sub, r.x + r.w * 0.5, r.y + 46.0, 15.0, DIM);
        if !matches!(self.updater.state(), State::Available { .. }) {
            return;
        }
        // The two builds on offer, each with its download size.
        let (songs, synth) = self.updater.choices();
        let mb = |b: u64| format!("{:.0} MB", b as f64 / 1_048_576.0);
        for (b, (label, size)) in update_buttons().iter().zip([("With songs", songs), ("Synth music only", synth)]) {
            let hot = size.is_some() && b.contains(mouse_ui());
            draw_rectangle(b.x, b.y, b.w, b.h, Color::new(0.07, 0.06, 0.06, if hot { 0.95 } else { 0.75 }));
            draw_rectangle_lines(b.x, b.y, b.w, b.h, 1.5, if hot { GOLD } else { crate::ui::EMBER });
            let (color, note) = match size {
                Some(s) => (if hot { GOLD } else { WHITE }, mb(s)),
                None => (DIM, "not in this release".to_string()),
            };
            ui.text_c(label, b.x + b.w * 0.5, b.y + 20.0, 16.0, color);
            ui.text_c(&note, b.x + b.w * 0.5, b.y + 38.0, 13.0, DIM);
        }
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

    /// The option pages there are now. During a vigil the last page, the difficulty, is
    /// left out: nothing that would change the vigil itself can be changed in the middle of it.
    fn page_count(&self) -> usize {
        if self.options_in_game {
            PAGES.len() - 1
        } else {
            PAGES.len()
        }
    }

    /// The rows of the current page; the cute option only once it has been found.
    fn option_rows(&self) -> Vec<Opt> {
        PAGES[self.opt_page.min(self.page_count() - 1)].1.iter().copied().filter(|o| *o != Opt::Cute || self.save.cute_found).collect()
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
            self.opt_page = (self.opt_page.min(self.page_count() - 1) as i32 + page_dir).rem_euclid(self.page_count() as i32) as usize;
            self.opt_sel = 0;
            self.audio.play("select", 0.5);
            drain_chars();
            return;
        }
        if let Some(t) = clicked_tab().filter(|t| *t < self.page_count()) {
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
        if matches!(opt, Opt::Music | Opt::BossMusic | Opt::Sfx | Opt::Vol(_) | Opt::FxAlpha | Opt::Markers | Opt::Brightness) {
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
            if std::mem::take(&mut self.options_in_game) && self.world.is_some() {
                // Back to the paused vigil, with the new options in effect.
                self.settings_to_world();
                self.screen = Screen::Paused;
            } else {
                self.screen = Screen::Menu;
            }
        }
    }

    /// The option row under the cursor changes the window or waits for a key: the
    /// self-test leaves those alone, so it never takes over a monitor or hangs.
    pub(super) fn option_is_risky(&self) -> bool {
        let rows = PAGES[self.opt_page.min(self.page_count() - 1)].1;
        matches!(rows.get(self.opt_sel), Some(Opt::Resolution | Opt::Fullscreen | Opt::Vsync | Opt::Msaa | Opt::Bind(_) | Opt::Name))
    }

    fn change_option(&mut self, opt: Opt, dir: i32) {
        let st = &mut self.save.settings;
        let cycle_u8 = |v: u8, n: u8| ((v as i32 + dir).rem_euclid(n as i32)) as u8;
        match opt {
            Opt::Music => {
                st.music = percent_step(st.music, dir);
                self.audio.set_music_volume(st.music);
            }
            Opt::BossMusic => {
                st.boss_music = percent_step(st.boss_music, dir);
                self.audio.set_boss_volume(st.boss_music);
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
            Opt::PauseUnfocused => st.pause_unfocused = !st.pause_unfocused,
            Opt::MuteUnfocused => st.mute_unfocused = !st.mute_unfocused,
            Opt::HurtFlash => st.hurt_flash = !st.hurt_flash,
            Opt::HeroHp => st.hero_hp_bar = !st.hero_hp_bar,
            Opt::HeroStamina => st.hero_stamina_bar = !st.hero_stamina_bar,
            Opt::BossSongOut => st.boss_song_out = !st.boss_song_out,
            Opt::Discord => st.discord = !st.discord,
            Opt::Updates => st.check_updates = !st.check_updates,
            Opt::Vol(i) => {
                st.vols[i as usize] = percent_step(st.vols[i as usize], dir);
                self.audio.kind_vol = st.vols;
            }
            Opt::FxAlpha => st.effect_alpha = (st.effect_alpha as i32 + dir * 5).clamp(crate::save::MIN_EFFECT_ALPHA as i32, 100) as u8,
            Opt::Brightness => st.brightness = (st.brightness as i32 + dir * 5).clamp(crate::save::MIN_BRIGHTNESS as i32, crate::save::MAX_BRIGHTNESS as i32) as u8,
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
                let i = PRESETS.iter().position(|p| p.1 == st.diff.base()).map(|i| i as i32).unwrap_or(1);
                let boost = st.diff.boost;
                st.diff = PRESETS[(i + dir).rem_euclid(PRESETS.len() as i32) as usize].1;
                st.diff.boost = boost;
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
            Opt::BossMusic => pct(st.boss_music),
            Opt::Sfx => pct(st.sfx),
            Opt::Numbers => onoff(st.numbers),
            Opt::Meter => onoff(st.meter),
            Opt::Overlay => onoff(st.overlay),
            Opt::Shake => onoff(st.shake),
            Opt::UiScale => match UI_SCALES[st.ui_scale as usize % UI_SCALES.len()] {
                0.0 => format!("Auto ({:.0}%)", crate::ui::ui_scale() * 100.0),
                s => format!("{:.0}%", s * 100.0),
            },
            Opt::PauseUnfocused => onoff(st.pause_unfocused),
            Opt::MuteUnfocused => onoff(st.mute_unfocused),
            Opt::HurtFlash => onoff(st.hurt_flash),
            Opt::HeroHp => onoff(st.hero_hp_bar),
            Opt::HeroStamina => onoff(st.hero_stamina_bar),
            Opt::BossSongOut => onoff(st.boss_song_out),
            Opt::Discord => onoff(st.discord),
            Opt::Updates => onoff(st.check_updates),
            Opt::Vol(i) => pct(st.vols[i as usize]),
            Opt::FxAlpha => format!("{}%", st.effect_alpha),
            Opt::Brightness => format!("{}%", st.brightness),
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
        let x0 = cx - tab_w * self.page_count() as f32 * 0.5;
        for (i, (name, _)) in PAGES.iter().take(self.page_count()).enumerate() {
            let x = x0 + i as f32 * tab_w;
            crate::ui::register_tab(Rect::new(x + 4.0, 74.0, tab_w - 8.0, 30.0));
            let on = i == self.opt_page.min(self.page_count() - 1);
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

    /// A small world to draw heroes in for the class card and the wardrobe.
    pub(super) fn ensure_preview(&mut self) {
        if self.preview.is_none() {
            self.preview = Some(World::new(Difficulty::default(), None));
        }
        if let Some(w) = self.preview.as_mut() {
            w.time += get_frame_time();
        }
    }

    /// Makes the picture of every field once.
    pub(super) fn ensure_field_shots(&mut self) {
        if self.field_shots.len() != ALL_BIOMES.len() {
            self.field_shots = (0..ALL_BIOMES.len()).map(|_| None).collect();
        }
        for (i, b) in ALL_BIOMES.iter().enumerate() {
            if self.field_shots[i].is_none() {
                self.field_shots[i] = Some(crate::draw::field_shot(*b));
                set_ui_camera();
            }
        }
    }

    /// The field card: a picture of the field, what is found there and its lords.
    pub(super) fn draw_field_card(&self, ui: &Ui, biome: Biome, anchor: Rect) {
        let (sw, sh) = (ui_w(), ui_h());
        let cw = 460.0f32.min(sw - 20.0);
        let (pw, ph) = (cw - 20.0, (cw - 20.0) * 9.0 / 16.0);
        // Room under the picture for the name, a line or two of words and the lords.
        let ch = ph + 150.0;
        let x = (anchor.x + anchor.w * 0.5 - cw * 0.5).clamp(10.0, sw - cw - 10.0);
        let y = if anchor.y + anchor.h + ch + 8.0 < sh { anchor.y + anchor.h + 6.0 } else { (sh - ch - 6.0).max(6.0) };
        draw_rectangle(x, y, cw, ch, Color::new(0.04, 0.035, 0.035, 0.97));
        draw_rectangle_lines(x, y, cw, ch, 1.5, Color::new(0.85, 0.7, 0.4, 0.7));
        if let Some(Some(rt)) = self.field_shots.get(biome as usize) {
            draw_texture_ex(&rt.texture, x + 10.0, y + 10.0, WHITE, DrawTextureParams { dest_size: Some(vec2(pw, ph)), ..Default::default() });
        }
        draw_rectangle_lines(x + 10.0, y + 10.0, pw, ph, 1.0, Color::new(0.3, 0.27, 0.25, 1.0));
        let mut ty = y + ph + 34.0;
        ui.text(biome.name(), x + 14.0, ty, 21.0, GOLD);
        ty += 22.0;
        for line in ui.wrap(biome.desc(), 15.0, cw - 28.0) {
            ui.text(&line, x + 14.0, ty, 15.0, PALE);
            ty += 18.0;
        }
        let lords: Vec<&str> = biome.lords().iter().map(|k| k.name()).collect();
        for line in ui.wrap(&format!("Lords: {}, and the Sovereign at dawn", lords.join(", ")), 14.0, cw - 28.0) {
            ui.text(&line, x + 14.0, ty, 14.0, DIM);
            ty += 17.0;
        }
    }

    /// The class card: the hero in the class's look, its weapons on both fields, what the
    /// class does and its skill. Shown under (or over) `anchor`.
    pub(super) fn draw_class_card(&self, ui: &Ui, class: Class, anchor: Rect) {
        let Some(world) = &self.preview else { return };
        let (sw, sh) = (ui_w(), ui_h());
        let (cw, ch) = (560.0f32.min(sw - 20.0), 250.0);
        let x = (anchor.x + anchor.w * 0.5 - cw * 0.5).clamp(10.0, sw - cw - 10.0);
        let y = if anchor.y + anchor.h + ch + 8.0 < sh { anchor.y + anchor.h + 6.0 } else { (anchor.y - ch - 6.0).max(6.0) };
        draw_rectangle(x, y, cw, ch, Color::new(0.04, 0.035, 0.035, 0.97));
        draw_rectangle_lines(x, y, cw, ch, 1.5, Color::new(0.85, 0.7, 0.4, 0.7));
        draw_rectangle(x + 10.0, y + 10.0, 150.0, ch - 20.0, Color::new(0.08, 0.07, 0.07, 1.0));
        let profile = Profile { skin: 0, ..self.save.profile };
        let mut hero = Hero::new(0, &self.player_name(), class, profile);
        hero.p.pos = Vec2::ZERO;
        hero.aim = vec2(1.0, -0.3).normalize();
        crate::draw::hero_preview(world, &hero, vec2(x + 85.0, y + ch * 0.55), 3.6);
        set_ui_camera();
        let tx = x + 176.0;
        let tw = cw - 190.0;
        let mut ty = y + 30.0;
        ui.text(class.name(), tx, ty, 22.0, GOLD);
        ty += 24.0;
        let w = class.weapon();
        ui.text(&format!("{}  /  {}  /  {}", w.name(false), w.for_field(1).name(false), w.for_field(2).name(false)), tx, ty, 14.0, EMBER);
        ty += 22.0;
        // What the class gives, without the weapon it starts with (said above).
        let gift = class.desc().split_once(". ").map(|(_, rest)| rest).unwrap_or(class.desc());
        for line in ui.wrap(gift, 16.0, tw) {
            ui.text(&line, tx, ty, 16.0, PALE);
            ty += 19.0;
        }
        ty += 8.0;
        let (skill, about, cd) = class.skill();
        let key = if using_pad() { "X" } else { key_label(Action::Skill) };
        ui.text(&format!("Skill [{}]: {}   every {:.0} s", key, skill, cd), tx, ty, 17.0, TEAL);
        ty += 21.0;
        for line in ui.wrap(about, 15.0, tw) {
            ui.text(&line, tx, ty, 15.0, DIM);
            ty += 18.0;
        }
    }

    pub(super) fn update_hub(&mut self) {
        drain_chars();
        self.ensure_preview();
        self.ensure_field_shots();
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
        if self.hub_sel == HUB_MODE_ROW && (confirm() || left(false) || right(false) || mouse.left || mouse.right) {
            self.toggle_boss_rush();
            return;
        }
        if self.hub_sel == HUB_BIOME_ROW && (confirm() || left(false) || right(false) || mouse.left || mouse.right) {
            self.cycle_biome(if left(false) || mouse.right { -1 } else { 1 });
            return;
        }
        if self.hub_sel == HUB_DIFF_ROW && (confirm() || left(false) || right(false) || mouse.left || mouse.right) {
            self.cycle_difficulty(if left(false) || mouse.right { -1 } else { 1 });
            return;
        }
        if (HUB_BOOST_ROW..HUB_CHALLENGE_ROW).contains(&self.hub_sel) && (left(false) || right(false) || mouse.left || mouse.right) {
            self.change_boost(self.hub_sel - HUB_BOOST_ROW, if left(false) || mouse.right { -1 } else { 1 });
            return;
        }
        let toggle = confirm() || left(false) || right(false) || mouse.left || mouse.right;
        if (HUB_CURSE_ROW..HUB_ROWS - 1).contains(&self.hub_sel) && toggle {
            self.save.curses ^= ALL_CURSES[self.hub_sel - HUB_CURSE_ROW].bit();
            self.save.store();
            self.audio.play("select", 0.6);
            return;
        }
        let on_challenge = (HUB_CHALLENGE_ROW..HUB_CURSE_ROW).contains(&self.hub_sel);
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
        let class_row = Rect::new(x, y, w, 34.0);
        let col = if class == Class::BucketKnight { Color::new(0.75, 0.75, 0.8, 1.0) } else { PALE };
        ui.text(&format!("<  {}  >", class.name()), x + 14.0, y + 23.0, 20.0, col);
        ui.text_r(class.desc(), x + w - 14.0, y + 23.0, 16.0, DIM);
        y += 38.0;
        let rush = self.save.settings.boss_rush;
        ui.row(x, y, w, 34.0, self.hub_sel == HUB_MODE_ROW);
        ui.text(&format!("<  {}  >", if rush { "Boss Rush" } else { "The Vigil" }), x + 14.0, y + 23.0, 20.0, if rush { EMBER } else { PALE });
        let about =
            if rush { "Twelve lords, one after another, no warning. Thin hordes, level-ups after every lord." } else { "Hold out until dawn at 30:00. A lord every five minutes." };
        ui.text_r(about, x + w - 14.0, y + 23.0, 16.0, DIM);
        y += 38.0;
        let biome = Biome::from_u8(self.save.settings.biome);
        ui.row(x, y, w, 34.0, self.hub_sel == HUB_BIOME_ROW);
        let field_row = Rect::new(x, y, w, 34.0);
        ui.text(&format!("<  {}  >", biome.name()), x + 14.0, y + 23.0, 20.0, PALE);
        ui.text_r(biome.desc(), x + w - 14.0, y + 23.0, 16.0, DIM);
        y += 38.0;
        // The same presets as in the lobby; finer settings stay under Options > Difficulty.
        let d = self.save.settings.diff;
        ui.row(x, y, w, 34.0, self.hub_sel == HUB_DIFF_ROW);
        ui.text(&format!("<  {}  >", d.preset_name()), x + 14.0, y + 23.0, 20.0, PALE);
        ui.text_r(
            &format!("Foes {:.0}%  health {:.0}%  damage {:.0}%   souls and marks x{:.2}", d.count * 100.0, d.hp * 100.0, d.dmg * 100.0, d.soul_mult()),
            x + w - 14.0,
            y + 23.0,
            16.0,
            DIM,
        );
        y += 38.0;
        // Empowered foes, each on its own, on top of the difficulty: stronger, and richer.
        for i in 0..3 {
            let n = d.boost[i];
            ui.row(x, y, w, 26.0, self.hub_sel == HUB_BOOST_ROW + i);
            let col = if n > 0 { EMBER } else { DIM };
            ui.text(BOOST_NAMES[i], x + 36.0, y + 19.0, 17.0, if n > 0 { PALE } else { DIM });
            ui.text(&format!("<  +{:.0}%  >", n as f32 * BOOST_STEP[i]), x + 245.0, y + 19.0, 16.0, col);
            let pay = (d.boost_mult(i) - 1.0) * [0.35, 1.0, 0.45][i] * 100.0;
            ui.text_r(&format!("+{:.0}% souls and marks", pay), x + w - 14.0, y + 19.0, 15.0, col);
            y += 29.0;
        }
        for (i, ch) in ALL_CHALLENGES.iter().enumerate() {
            let on = self.save.challenges & ch.bit() != 0;
            ui.row(x, y, w, 26.0, self.hub_sel == HUB_CHALLENGE_ROW + i);
            draw_rectangle_lines(x + 14.0, y + 8.0, 13.0, 13.0, 1.5, if on { BLOOD_RED } else { DIM });
            if on {
                draw_rectangle(x + 17.0, y + 11.0, 7.0, 7.0, BLOOD_RED);
            }
            ui.text(ch.name(), x + 36.0, y + 20.0, 17.0, if on { PALE } else { DIM });
            ui.text(ch.desc(), x + 245.0, y + 20.0, 15.0, DIM);
            ui.text_r("+25% marks", x + w - 14.0, y + 20.0, 15.0, if on { EMBER } else { DIM });
            y += 29.0;
        }
        for (i, cu) in ALL_CURSES.iter().enumerate() {
            let on = self.save.curses & cu.bit() != 0;
            let col = Color::new(0.75, 0.45, 1.0, 1.0);
            ui.row(x, y, w, 26.0, self.hub_sel == HUB_CURSE_ROW + i);
            draw_poly_lines(x + 20.5, y + 14.5, 4, 8.0, 45.0, 1.5, if on { col } else { DIM });
            if on {
                draw_poly(x + 20.5, y + 14.5, 4, 4.5, 45.0, col);
            }
            ui.text(&format!("Curse: {}", cu.name()), x + 36.0, y + 20.0, 17.0, if on { PALE } else { DIM });
            ui.text(cu.desc(), x + 245.0, y + 20.0, 15.0, DIM);
            ui.text_r("+20% marks", x + w - 14.0, y + 20.0, 15.0, if on { col } else { DIM });
            y += 29.0;
        }
        y += 10.0;
        ui.row(x, y, w, 34.0, self.hub_sel == HUB_ROWS - 1);
        let begin = if rush { "Begin the Boss Rush" } else { "Begin the Vigil" };
        let d = self.save.settings.diff;
        let label = if d.boost == [0; 3] { d.preset_name().to_string() } else { format!("{}, empowered", d.preset_name()) };
        ui.text_c(&format!("{}  ({})", begin, label), cx, y + 24.0, 22.0, EMBER);
        // A reminder where there is room for it; small screens need the space for notes.
        if sh >= 800.0 {
            ui.text_c("Permanent upgrades are bought in the Reliquary.", cx, y + 64.0, 15.0, DIM);
        }
        // The field card, under the mouse or for a moment after the field changed.
        let near_field = Rect::new(field_row.x, field_row.y, 420.0, field_row.h).contains(crate::app::input::mouse_ui());
        if near_field || (self.hub_sel == HUB_BIOME_ROW && get_time() < self.field_card_until) {
            self.draw_field_card(ui, biome, field_row);
        }
        // The class card, under the mouse or for a moment after the class changed.
        let near = Rect::new(class_row.x, class_row.y, 360.0, class_row.h).contains(crate::app::input::mouse_ui());
        if near || (self.hub_sel == 0 && get_time() < self.class_card_until) {
            self.draw_class_card(ui, class, class_row);
        }
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
        match self.relic_page {
            0 => META_COUNT + 1,
            1 => UNLOCKS.len() + RELIC_COUNT + 1,
            _ => OFFERINGS.len() + 1,
        }
    }

    /// Gives offering `i` away, if there is enough to give, and earns what it earns.
    fn make_offering(&mut self, i: usize) {
        let (souls, amount) = OFFERINGS[i];
        let have = if souls { self.save.souls } else { self.save.marks };
        // 0 means everything there is.
        let give = if amount == 0 { have } else { amount };
        if give == 0 || have < give {
            self.audio.play("hurt", 0.3);
            self.set_note(if souls { "Not that many souls to give." } else { "Not that many Ashen Marks to give." }, 2.5);
            return;
        }
        let before = (self.save.offered_souls, self.save.offered_marks);
        if souls {
            self.save.souls -= give;
            self.save.offered_souls = self.save.offered_souls.saturating_add(give);
        } else {
            self.save.marks -= give;
            self.save.offered_marks = self.save.offered_marks.saturating_add(give);
        }
        self.save.store();
        self.audio.play("bell", 0.6);
        let what = if souls { "souls" } else { "Ashen Marks" };
        self.set_note(&format!("You offer {} {}. The ash remembers.", give, what), 3.0);
        for (class, by_souls, n) in OFFERED_CLASSES {
            let (was, now) = if by_souls { (before.0, self.save.offered_souls) } else { (before.1, self.save.offered_marks) };
            if was < n && now >= n {
                self.audio.play("evolve", 0.7);
                self.set_note(&format!("A new class answers your offering: the {}.", class.name()), 4.0);
            }
        }
        for (done, ach) in [
            (self.save.offered_souls >= 10_000, Achievement::Alms),
            (self.save.offered_souls >= 100_000, Achievement::Benefactor),
            (self.save.offered_souls >= 1_000_000, Achievement::PatronOfAsh),
            (self.save.offered_marks >= 100, Achievement::MarkGiver),
            (self.save.offered_marks >= 1_000, Achievement::Selfless),
        ] {
            if done {
                self.unlock(ach);
            }
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
            self.relic_page = clicked_page.unwrap_or((self.relic_page as i32 + page_dir).rem_euclid(3) as usize);
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
        if self.relic_page == 2 {
            self.make_offering(self.relic_sel);
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
        for (i, name) in ["Soul Upgrades", "Relics and Unlocks", "Offerings"].iter().enumerate() {
            let tx = cx - tab_w * 1.5 + i as f32 * tab_w;
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
        match self.relic_page {
            0 => self.draw_upgrades(ui, x, top, w),
            1 => self.draw_relics(ui, x, top, w),
            _ => self.draw_offerings(ui, x, top, w),
        }
        let foot = match self.relic_page {
            0 => "Souls are banked at shrines during a vigil. Upgrades last forever.",
            1 => "Ashen Marks are earned at the end of every vigil: time, bosses, level and kills count.",
            _ => "Offerings buy nothing. They are given to the dark, and the dark keeps count.",
        };
        ui.text_c(foot, cx, sh - 44.0, 15.0, DIM);
        ui.text_c(hint("Q / E or Tab: page    W/S/A/D select    Enter buy    Esc back", "LB / RB: page    Stick select    A buy    B back"), cx, sh - 20.0, 15.0, DIM);
    }

    /// The offerings: give souls or Ashen Marks away, and how much has been given so far.
    fn draw_offerings(&self, ui: &Ui, x: f32, top: f32, w: f32) {
        let cx = x + w * 0.5;
        let rw = 520.0f32.min(w);
        let rx = cx - rw * 0.5;
        let mut y = top + 10.0;
        ui.text_c(&format!("Offered so far:  {} souls,  {} Ashen Marks", self.save.offered_souls, self.save.offered_marks), cx, y + 18.0, 18.0, PALE);
        y += 40.0;
        for (i, (souls, amount)) in OFFERINGS.iter().enumerate() {
            let have = if *souls { self.save.souls } else { self.save.marks };
            let give = if *amount == 0 { have } else { *amount };
            let label = match (*souls, *amount) {
                (true, 0) => format!("Offer all your souls ({})", have),
                (false, 0) => format!("Offer all your Ashen Marks ({})", have),
                (true, n) => format!("Offer {} souls", n),
                (false, n) => format!("Offer {} Ashen Marks", n),
            };
            let can = give > 0 && have >= give;
            ui.row(rx, y, rw, 34.0, self.relic_sel == i);
            ui.text_c(
                &label,
                cx,
                y + 23.0,
                18.0,
                if !can {
                    DIM
                } else if *souls {
                    GOLD
                } else {
                    EMBER
                },
            );
            y += 40.0;
        }
        ui.row(rx, y, rw, 34.0, self.relic_sel == OFFERINGS.len());
        ui.text_c("Back", cx, y + 23.0, 18.0, DIM);
        // The next milestones.
        let next_souls = [10_000u64, 100_000, 1_000_000].into_iter().find(|n| self.save.offered_souls < *n);
        let next_marks = [100u64, 1_000].into_iter().find(|n| self.save.offered_marks < *n);
        let mut ny = y + 64.0;
        for (next, have, what) in [(next_souls, self.save.offered_souls, "souls"), (next_marks, self.save.offered_marks, "Ashen Marks")] {
            let line = match next {
                Some(n) => format!("Next honour at {} {} offered ({} to go)", n, what, n - have),
                None => format!("Every honour for {} is yours", what),
            };
            ui.text_c(&line, cx, ny, 16.0, DIM);
            ny += 22.0;
        }
        // The classes that come with offerings.
        ny += 8.0;
        for (class, by_souls, n) in OFFERED_CLASSES {
            let have = if by_souls { self.save.offered_souls } else { self.save.offered_marks };
            let what = if by_souls { "souls" } else { "Ashen Marks" };
            let (line, col) =
                if have >= n { (format!("Class: the {} walks with you", class.name()), TEAL) } else { (format!("Class: the {} at {} {} offered", class.name(), n, what), EMBER) };
            ui.text_c(&line, cx, ny, 16.0, col);
            ny += 22.0;
        }
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
