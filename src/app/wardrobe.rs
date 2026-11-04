//! The Wardrobe: hero skins bought with souls. A skin wins over the class look.

use super::*;
use crate::ui::{EMBER, GOLD, TEAL};

/// Rows: the class look, every skin, then Back.
const ROWS: usize = SKINS.len() + 2;
const COLS: usize = 2;

impl App {
    fn owns_skin(&self, skin: usize) -> bool {
        skin == 0 || self.save.skins & 1 << (skin - 1) != 0
    }

    pub(super) fn update_wardrobe(&mut self) {
        drain_chars();
        if self.preview.is_none() {
            self.preview = Some(World::new(Difficulty::default(), None));
        }
        if let Some(w) = self.preview.as_mut() {
            w.time += get_frame_time();
        }
        if grid_nav(&mut self.wardrobe_sel, 0, SKINS.len() + 1, COLS) || nav(&mut self.wardrobe_sel, ROWS, false) {
            self.audio.play("select", 0.4);
        }
        let click = mouse_rows(&mut self.wardrobe_sel).left;
        if back() || (self.wardrobe_sel == ROWS - 1 && (confirm() || click)) {
            self.save.store();
            self.screen = Screen::Menu;
            return;
        }
        if !confirm() && !click {
            return;
        }
        let i = self.wardrobe_sel;
        if self.owns_skin(i) {
            self.save.profile.skin = i as u8;
            self.save.store();
            self.audio.play("select", 0.6);
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
        let top = 108.0;
        let row_h = 30.0;
        let cw = (gw - 8.0) / COLS as f32;
        for i in 0..=SKINS.len() {
            let (rx, ry) = (gx + (i % COLS) as f32 * (cw + 8.0), top + (i / COLS) as f32 * row_h);
            let on = self.wardrobe_sel == i;
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
            } else {
                let afford = self.save.souls >= SKINS[i - 1].cost;
                (price, if afford { GOLD } else { DIM })
            };
            ui.text_r(&tag, rx + cw - 10.0, ry + 19.0, 15.0, col);
        }
        let back_y = top + (SKINS.len() + 1).div_ceil(COLS) as f32 * row_h + 6.0;
        ui.row(gx, back_y, gw, row_h - 3.0, self.wardrobe_sel == ROWS - 1);
        ui.text_c("Back", gx + gw * 0.5, back_y + 20.0, 18.0, DIM);

        // The selected look on the hero of the chosen class.
        let skin = if self.wardrobe_sel <= SKINS.len() { self.wardrobe_sel } else { self.save.profile.skin as usize };
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
            ui.text_c(&format!("{} souls", SKINS[skin - 1].cost), x + pw * 0.5, top + 380.0, 16.0, EMBER);
        }
        ui.text_c("A skin changes how your hero looks in every class, online too.", cx, sh - 44.0, 15.0, DIM);
        ui.text_c(hint("W/S/A/D select    Enter buy or wear    Esc back", "Stick select    A buy or wear    B back"), cx, sh - 20.0, 15.0, DIM);
    }
}
