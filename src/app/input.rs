//! Keyboard and gamepad helpers shared by all screens.

use crate::save::{key_name, Action, ACTIONS};
use gilrs::{Axis, Button, EventType, Gilrs};
use macroquad::prelude::*;
use std::cell::RefCell;

pub const KONAMI: [KeyCode; 10] = [KeyCode::Up, KeyCode::Up, KeyCode::Down, KeyCode::Down, KeyCode::Left, KeyCode::Right, KeyCode::Left, KeyCode::Right, KeyCode::B, KeyCode::A];

pub fn pressed_any(keys: &[KeyCode]) -> bool {
    keys.iter().any(|k| is_key_pressed(*k))
}

pub fn confirm() -> bool {
    pressed_any(&[KeyCode::Enter, KeyCode::KpEnter, KeyCode::Space]) || pad_pressed(Pad::Confirm)
}

/// On a text row only the arrow keys navigate, so W/A/S/D can be typed.
pub fn up(text_row: bool) -> bool {
    is_key_pressed(KeyCode::Up) || (!text_row && is_key_pressed(KeyCode::W)) || pad_pressed(Pad::Up)
}

pub fn down(text_row: bool) -> bool {
    is_key_pressed(KeyCode::Down) || (!text_row && is_key_pressed(KeyCode::S)) || pad_pressed(Pad::Down)
}

pub fn left(text_row: bool) -> bool {
    is_key_pressed(KeyCode::Left) || (!text_row && is_key_pressed(KeyCode::A)) || pad_pressed(Pad::Left)
}

pub fn right(text_row: bool) -> bool {
    is_key_pressed(KeyCode::Right) || (!text_row && is_key_pressed(KeyCode::D)) || pad_pressed(Pad::Right)
}

/// Moves a cursor through a block of `count` rows laid out `cols` to a line, starting at row
/// `start`; returns true when it took the key. Up from the top line leaves to the row before
/// the block, down from the bottom line to the row after it.
pub fn grid_nav(sel: &mut usize, start: usize, count: usize, cols: usize) -> bool {
    if *sel < start || *sel >= start + count {
        return false;
    }
    let i = *sel - start;
    if left(false) && !i.is_multiple_of(cols) {
        *sel -= 1;
    } else if right(false) && i % cols + 1 < cols && i + 1 < count {
        *sel += 1;
    } else if up(false) {
        *sel = if i >= cols { *sel - cols } else { start.saturating_sub(1) };
    } else if down(false) {
        *sel = if i + cols < count { *sel + cols } else { start + count };
    } else {
        return false;
    }
    true
}

/// Moves a menu cursor; returns true when it moved.
pub fn nav(sel: &mut usize, rows: usize, text_row: bool) -> bool {
    if rows == 0 {
        return false;
    }
    if up(text_row) {
        *sel = (*sel + rows - 1) % rows;
        true
    } else if down(text_row) {
        *sel = (*sel + 1) % rows;
        true
    } else {
        false
    }
}

/// Feeds typed characters into `s`.
pub fn edit_text(s: &mut String, max: usize, allowed: fn(char) -> bool) {
    while let Some(c) = get_char_pressed() {
        if allowed(c) && s.chars().count() < max {
            s.push(c);
        }
    }
    if is_key_pressed(KeyCode::Backspace) {
        s.pop();
    }
}

pub fn name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || " _-'.".contains(c)
}

pub fn addr_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || ".:-_".contains(c)
}

/// Throws away typed characters nobody asked for.
pub fn drain_chars() {
    while get_char_pressed().is_some() {}
}

// ---------------------------------------------------------------- key bindings

thread_local! {
    static BINDS: std::cell::Cell<[KeyCode; ACTIONS.len()]> = const { std::cell::Cell::new([KeyCode::Unknown; ACTIONS.len()]) };
}

/// Takes the player's bindings; called every frame, so changes in the options apply at once.
pub fn set_binds(binds: [KeyCode; ACTIONS.len()]) {
    BINDS.with(|b| b.set(binds));
}

pub fn key_for(a: Action) -> KeyCode {
    let i = ACTIONS.iter().position(|x| *x == a).unwrap_or(0);
    BINDS.with(|b| b.get()[i])
}

pub fn act_pressed(a: Action) -> bool {
    is_key_pressed(key_for(a))
}

pub fn act_down(a: Action) -> bool {
    is_key_down(key_for(a))
}

/// The bound key's name, for hints.
pub fn key_label(a: Action) -> &'static str {
    key_name(key_for(a))
}

/// The hint for whichever device was used last.
pub fn hint<'a>(keys: &'a str, pad: &'a str) -> &'a str {
    if using_pad() {
        pad
    } else {
        keys
    }
}

pub fn move_input() -> Vec2 {
    let mut dir = Vec2::ZERO;
    if act_down(Action::Up) || is_key_down(KeyCode::Up) {
        dir.y -= 1.0;
    }
    if act_down(Action::Down) || is_key_down(KeyCode::Down) {
        dir.y += 1.0;
    }
    if act_down(Action::Left) || is_key_down(KeyCode::Left) {
        dir.x -= 1.0;
    }
    if act_down(Action::Right) || is_key_down(KeyCode::Right) {
        dir.x += 1.0;
    }
    if dir == Vec2::ZERO {
        // The stick only steers; speed stays the hero's own.
        return pad_stick().normalize_or_zero();
    }
    dir.normalize_or_zero()
}

// ---------------------------------------------------------------- gamepad

/// Gamepad actions, mapped to Xbox-style buttons.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Pad {
    /// A
    Confirm,
    /// B
    Back,
    /// A or RB
    Roll,
    /// Start
    Pause,
    /// Select / View: leave the vigil from the pause menu
    Leave,
    /// Y
    Reroll,
    /// RB
    PageNext,
    /// LB
    PagePrev,
    Up,
    Down,
    Left,
    Right,
}

struct PadState {
    gilrs: Option<Gilrs>,
    pressed: Vec<Button>,
    /// Left stick plus d-pad, y down.
    stick: Vec2,
    /// Stick flicks this frame, for menus: up, down, left, right.
    flick: [bool; 4],
    held_dir: [bool; 4],
    /// LB held, for the per-weapon damage view.
    detail: bool,
    /// The controller was used more recently than the keyboard.
    active: bool,
}

thread_local! {
    // Leaked on purpose: dropping gilrs during process::exit panics inside its Windows backend.
    static PAD: &'static RefCell<PadState> = Box::leak(Box::new(RefCell::new(PadState {
        gilrs: Gilrs::new().ok(),
        pressed: Vec::new(),
        stick: Vec2::ZERO,
        flick: [false; 4],
        held_dir: [false; 4],
        detail: false,
        active: false,
    })));
}

// ---------------------------------------------------------------- mouse

thread_local! {
    /// Mouse position last frame, and whether it moved since.
    static MOUSE: std::cell::Cell<(Vec2, bool)> = const { std::cell::Cell::new((Vec2::ZERO, false)) };
}

/// The mouse in UI space.
pub fn mouse_ui() -> Vec2 {
    Vec2::from(mouse_position()) / crate::ui::ui_scale()
}

/// The mouse moved this frame; only then does hovering change a selection, so a resting
/// mouse never fights the keyboard.
pub fn mouse_moved() -> bool {
    MOUSE.with(|m| m.get().1)
}

pub fn clicked() -> bool {
    is_mouse_button_pressed(MouseButton::Left)
}

pub fn right_clicked() -> bool {
    is_mouse_button_pressed(MouseButton::Right)
}

/// Lets the mouse drive a menu: hovering a row drawn last frame selects it, and the result
/// says whether it was left- or right-clicked this frame.
pub fn mouse_rows(sel: &mut usize) -> MouseRow {
    let under = hovered(&crate::ui::rows_drawn());
    let Some(i) = under else { return MouseRow::default() };
    let (left, right) = (clicked(), right_clicked());
    if mouse_moved() || left || right {
        *sel = i;
    }
    MouseRow { left, right }
}

#[derive(Default, Clone, Copy)]
pub struct MouseRow {
    pub left: bool,
    pub right: bool,
}

/// The tab clicked this frame, if any.
pub fn clicked_tab() -> Option<usize> {
    if clicked() {
        hovered(&crate::ui::tabs_drawn())
    } else {
        None
    }
}

/// The row under the mouse.
pub fn hovered(rows: &[Rect]) -> Option<usize> {
    let m = mouse_ui();
    rows.iter().position(|r| r.contains(m))
}

/// Reads all gamepad events and the mouse; call once at the start of every frame.
pub fn poll_gamepad() {
    let mouse_used = MOUSE.with(|m| {
        let now = Vec2::from(mouse_position());
        let moved = now.distance(m.get().0) > 0.5;
        m.set((now, moved));
        moved || clicked()
    });
    PAD.with(|p| {
        let mut p = p.borrow_mut();
        let state = &mut *p;
        state.pressed.clear();
        state.flick = [false; 4];
        if get_last_key_pressed().is_some() || mouse_used {
            state.active = false;
        }
        let Some(g) = state.gilrs.as_mut() else { return };
        while let Some(ev) = g.next_event() {
            if let EventType::ButtonPressed(b, _) = ev.event {
                state.pressed.push(b);
                state.active = true;
            }
        }
        let mut stick = Vec2::ZERO;
        if let Some((_, gp)) = g.gamepads().next() {
            stick = vec2(gp.value(Axis::LeftStickX), -gp.value(Axis::LeftStickY));
            if gp.is_pressed(Button::DPadUp) {
                stick.y -= 1.0;
            }
            if gp.is_pressed(Button::DPadDown) {
                stick.y += 1.0;
            }
            if gp.is_pressed(Button::DPadLeft) {
                stick.x -= 1.0;
            }
            if gp.is_pressed(Button::DPadRight) {
                stick.x += 1.0;
            }
            state.detail = gp.is_pressed(Button::LeftTrigger);
        }
        if stick.length() > 0.5 {
            state.active = true;
        }
        state.stick = stick;
        // Menus move one step per push of the stick, with some hysteresis.
        let dirs = [-stick.y, stick.y, -stick.x, stick.x];
        for ((d, held), flick) in dirs.into_iter().zip(state.held_dir.iter_mut()).zip(state.flick.iter_mut()) {
            if !*held && d > 0.6 {
                *held = true;
                *flick = true;
            } else if *held && d < 0.35 {
                *held = false;
            }
        }
    });
}

pub fn pad_pressed(action: Pad) -> bool {
    PAD.with(|p| {
        let p = p.borrow();
        let has = |b: Button| p.pressed.contains(&b);
        match action {
            Pad::Confirm => has(Button::South),
            Pad::Back => has(Button::East),
            Pad::Roll => has(Button::South) || has(Button::RightTrigger) || has(Button::RightTrigger2),
            Pad::Pause => has(Button::Start),
            Pad::Leave => has(Button::Select),
            Pad::Reroll => has(Button::North),
            Pad::PageNext => has(Button::RightTrigger),
            Pad::PagePrev => has(Button::LeftTrigger),
            Pad::Up => p.flick[0],
            Pad::Down => p.flick[1],
            Pad::Left => p.flick[2],
            Pad::Right => p.flick[3],
        }
    })
}

/// Movement from the left stick and d-pad, with a dead zone.
pub fn pad_stick() -> Vec2 {
    let s = PAD.with(|p| p.borrow().stick);
    if s.length() < 0.25 {
        Vec2::ZERO
    } else {
        s.clamp_length_max(1.0)
    }
}

/// LB is held.
pub fn pad_detail() -> bool {
    PAD.with(|p| p.borrow().detail)
}

/// True when the controller was used more recently than the keyboard.
pub fn using_pad() -> bool {
    PAD.with(|p| p.borrow().active)
}

/// Escape or B.
pub fn back() -> bool {
    is_key_pressed(KeyCode::Escape) || pad_pressed(Pad::Back)
}
