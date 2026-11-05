//! The Wardrobe: hero skins bought with souls on one page, and on another those given for
//! what was offered in the Reliquary. A skin wins over the class look.

use super::*;
use crate::ui::{EMBER, GOLD, TEAL};

const COLS: usize = 2;
const PAGES: [&str; 2] = ["Bought with souls", "Given for offerings"];

/// The looks on a wardrobe page, as skin numbers (0 is the class look).
pub(super) fn page_skins(page: usize) -> Vec<usize> {
    let offered = page == 1;
    let mut v: Vec<usize> = if offered { Vec::new() } else { vec![0] };
    v.extend((1..=SKINS.len()).filter(|&i| SKINS[i - 1].offering.is_some() == offered));
    v
}

/// The page and row that show skin `skin`.
pub(super) fn wardrobe_spot(skin: usize) -> (usize, usize) {
    for page in 0..PAGES.len() {
        if let Some(row) = page_skins(page).iter().position(|s| *s == skin) {
            return (page, row);
        }
    }
    (0, 0)
}

impl App {
    fn owns_skin(&self, skin: usize) -> bool {
        skin == 0 || self.save.skins & 1 << (skin - 1) != 0 || self.offered_enough(skin)
    }

    /// Whether a skin that comes with offerings has been earned.
    fn offered_enough(&self, skin: usize) -> bool {
        match SKINS.get(skin.wrapping_sub(1)).and_then(|s| s.offering) {
            Some((true, n)) => self.save.offered_souls >= n,
            Some((false, n)) => self.save.offered_marks >= n,
            None => false,
        }
    }

    /// What a skin that comes with offerings asks for, in words.
    fn offering_text(skin: usize) -> Option<String> {
        SKINS.get(skin.wrapping_sub(1)).and_then(|s| s.offering).map(|(souls, n)| format!("Offer {} {}", menus::short_num(n), if souls { "souls" } else { "marks" }))
    }

    pub(super) fn update_wardrobe(&mut self) {
        drain_chars();
        if self.preview.is_none() {
            self.preview = Some(World::new(Difficulty::default(), None));
        }
        if let Some(w) = self.preview.as_mut() {
            w.time += get_frame_time();
        }
        // Q / E, Tab or a click on a tab: the other page.
        let page_dir = if is_key_pressed(KeyCode::Tab) || is_key_pressed(KeyCode::E) || is_key_pressed(KeyCode::PageDown) || pad_pressed(Pad::PageNext) {
            1
        } else if is_key_pressed(KeyCode::Q) || is_key_pressed(KeyCode::PageUp) || pad_pressed(Pad::PagePrev) {
            -1
        } else {
            0
        };
        let clicked_page = clicked_tab().filter(|t| *t != self.wardrobe_page && *t < PAGES.len());
        if page_dir != 0 || clicked_page.is_some() {
            self.wardrobe_page = clicked_page.unwrap_or((self.wardrobe_page as i32 + page_dir).rem_euclid(PAGES.len() as i32) as usize);
            self.wardrobe_sel = 0;
            self.audio.play("select", 0.5);
            return;
        }
        let skins = page_skins(self.wardrobe_page);
        let rows = skins.len() + 1;
        if grid_nav(&mut self.wardrobe_sel, 0, skins.len(), COLS) || nav(&mut self.wardrobe_sel, rows, false) {
            self.audio.play("select", 0.4);
        }
        let click = mouse_rows(&mut self.wardrobe_sel).left;
        if back() || (self.wardrobe_sel == rows - 1 && (confirm() || click)) {
            self.save.store();
            self.screen = Screen::Menu;
            return;
        }
        if !confirm() && !click {
            return;
        }
        let Some(&i) = skins.get(self.wardrobe_sel) else { return };
        if self.owns_skin(i) {
            self.save.profile.skin = i as u8;
            self.save.store();
            self.audio.play("select", 0.6);
            return;
        }
        if let Some(text) = Self::offering_text(i) {
            // Not for sale: it is given for what has been offered in the Reliquary.
            self.audio.play("hurt", 0.3);
            self.set_note(&format!("{} in the Reliquary to earn this look.", text), 3.0);
            return;
        }
        let cost = SKINS[i - 1].cost;
        if self.save.souls < cost {
            self.audio.play("hurt", 0.3);
            self.set_note("Not enough souls. Bank them at shrines during a vigil.", 3.0);
            return;
        }
        self.save.souls -= cost;
        self.save.skins |= 1 << (i - 1);
        self.save.profile.skin = i as u8;
        self.save.store();
        self.audio.play("evolve", 0.6);
        self.unlock(Achievement::Dressed);
        if self.save.skins.count_ones() >= 10 {
            self.unlock(Achievement::Wardrobe);
        }
    }

    pub(super) fn draw_wardrobe(&self, ui: &Ui) {
        let (sw, sh) = (ui_w(), ui_h());
        let cx = sw * 0.5;
        ui.spaced_c("THE WARDROBE", cx, 56.0, 40.0, 8.0, PALE);
        ui.text_c(&format!("Souls  {}", self.save.souls), cx, 88.0, 22.0, GOLD);
        let w = 1100.0f32.min(sw - 40.0);
        let x = cx - w * 0.5;
        // The preview on the left, the skins on the right.
        let pw = 300.0f32.min(w * 0.3);
        let gx = x + pw + 16.0;
        let gw = w - pw - 16.0;
        // Page tabs.
        let tab_w = 260.0;
        for (i, name) in PAGES.iter().enumerate() {
            let tx = cx - tab_w + i as f32 * tab_w;
            crate::ui::register_tab(Rect::new(tx + 4.0, 104.0, tab_w - 8.0, 28.0));
            let on = i == self.wardrobe_page;
            draw_rectangle(tx + 4.0, 104.0, tab_w - 8.0, 28.0, Color::new(0.07, 0.06, 0.06, if on { 0.95 } else { 0.5 }));
            if on {
                draw_rectangle(tx + 4.0, 130.0, tab_w - 8.0, 2.0, GOLD);
            }
            ui.text_c(name, tx + tab_w * 0.5, 124.0, 18.0, if on { GOLD } else { DIM });
        }
        let top = 144.0;
        let row_h = 30.0;
        let cw = (gw - 8.0) / COLS as f32;
        let skins = page_skins(self.wardrobe_page);
        for (n, &i) in skins.iter().enumerate() {
            let (rx, ry) = (gx + (n % COLS) as f32 * (cw + 8.0), top + (n / COLS) as f32 * row_h);
            let on = self.wardrobe_sel == n;
            ui.row(rx, ry, cw, row_h - 3.0, on);
            let worn = self.save.profile.skin as usize == i;
            let (name, price) = match i {
                0 => ("Class look", String::new()),
                _ => (SKINS[i - 1].name, menus::short_num(SKINS[i - 1].cost)),
            };
            if i > 0 {
                let (r, g, b) = SKINS[i - 1].body;
                draw_rectangle(rx + 10.0, ry + 8.0, 10.0, 10.0, Color::from_rgba(r, g, b, 255));
            }
            ui.text(name, rx + 28.0, ry + 19.0, 16.0, if worn { GOLD } else { PALE });
            let (tag, col) = if worn {
                ("WORN".to_string(), TEAL)
            } else if self.owns_skin(i) {
                ("OWNED".to_string(), DIM)
            } else if let Some(text) = Self::offering_text(i) {
                (text, EMBER)
            } else {
                let afford = self.save.souls >= SKINS[i - 1].cost;
                (price, if afford { GOLD } else { DIM })
            };
            ui.text_r(&tag, rx + cw - 10.0, ry + 19.0, 15.0, col);
        }
        let back_y = top + skins.len().div_ceil(COLS) as f32 * row_h + 6.0;
        ui.row(gx, back_y, gw, row_h - 3.0, self.wardrobe_sel == skins.len());
        ui.text_c("Back", gx + gw * 0.5, back_y + 20.0, 18.0, DIM);

        // The selected look on the hero of the chosen class.
        let skin = skins.get(self.wardrobe_sel).copied().unwrap_or(self.save.profile.skin as usize);
        draw_rectangle(x, top, pw, back_y + row_h - top - 3.0, Color::new(0.06, 0.05, 0.05, 0.8));
        if let Some(world) = &self.preview {
            let profile = Profile { skin: skin as u8, ..self.save.profile };
            let mut hero = Hero::new(0, &self.player_name(), self.class(), profile);
            hero.p.pos = Vec2::ZERO;
            hero.aim = vec2(1.0, -0.3).normalize();
            crate::draw::hero_preview(world, &hero, vec2(x + pw * 0.5, top + 200.0), 5.0);
            set_ui_camera();
        }
        let (title, desc) = match skin {
            0 => (self.class().name(), "Every class wears its own colours."),
            s => (SKINS[s - 1].name, SKINS[s - 1].desc),
        };
        ui.text_c(title, x + pw * 0.5, top + 330.0, 20.0, GOLD);
        ui.text_c(desc, x + pw * 0.5, top + 354.0, 13.0, DIM);
        if skin > 0 && !self.owns_skin(skin) {
            let ask = Self::offering_text(skin).map(|t| format!("{} in the Reliquary", t)).unwrap_or_else(|| format!("{} souls", SKINS[skin - 1].cost));
            ui.text_c(&ask, x + pw * 0.5, top + 380.0, 16.0, EMBER);
        }
        ui.text_c("A skin changes how your hero looks in every class, online too.", cx, sh - 44.0, 15.0, DIM);
        ui.text_c(
            hint("Q / E or Tab: page    W/S/A/D select    Enter buy or wear    Esc back", "LB / RB: page    Stick select    A buy or wear    B back"),
            cx,
            sh - 20.0,
            15.0,
            DIM,
        );
    }
}
