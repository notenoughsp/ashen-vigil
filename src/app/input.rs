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
    let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
    let shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
    // Ctrl+V (or Shift+Insert) pastes; whatever the field does not take, such as the spaces
    // and line breaks that come along when copying an address, is left out.
    if (ctrl && is_key_pressed(KeyCode::V)) || (shift && is_key_pressed(KeyCode::Insert)) {
        if let Some(text) = macroquad::miniquad::window::clipboard_get() {
            for c in text.trim().chars().filter(|c| allowed(*c)) {
                if s.chars().count() >= max {
                    break;
                }
                s.push(c);
            }
        }
    }
    if is_key_pressed(KeyCode::Backspace) {
        // Ctrl+Backspace clears the whole field.
        if ctrl {
            s.clear();
        } else {
            s.pop();
        }
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
    } else if cfg!(target_os = "android") {
        // On a phone everything is tapped; key hints would only confuse.
        ""
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
        let pad = pad_stick().normalize_or_zero();
        return if pad == Vec2::ZERO { touch_dir() } else { pad };
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
    /// X
    Skill,
    /// Y
    Reroll,
    /// RB
    PageNext,
    /// LB
    PagePrev,
    /// Right stick press
    Ping,
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
    /// The rumble playing now; dropping it stops it.
    rumble: Option<gilrs::ff::Effect>,
    /// Rumble strength from the options, 0 to 1.
    rumble_scale: f32,
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
        rumble: None,
        rumble_scale: 0.6,
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

/// Takes the rumble setting (percent); called every frame.
pub fn set_rumble(percent: u8) {
    PAD.with(|p| p.borrow_mut().rumble_scale = percent as f32 / 100.0);
}

/// Shakes every controller that can, at `strength` (0 to 1) for `ms` milliseconds, scaled
/// by the rumble option. A stronger rumble replaces a weaker one still playing.
pub fn rumble(strength: f32, ms: u32) {
    use gilrs::ff::{BaseEffect, BaseEffectType, EffectBuilder, Replay, Ticks};
    PAD.with(|p| {
        let mut p = p.borrow_mut();
        let state = &mut *p;
        let k = (strength * state.rumble_scale).clamp(0.0, 1.0);
        if k <= 0.01 || !state.active {
            return;
        }
        let Some(g) = state.gilrs.as_mut() else { return };
        let pads: Vec<_> = g.gamepads().filter(|(_, gp)| gp.is_ff_supported()).map(|(id, _)| id).collect();
        if pads.is_empty() {
            return;
        }
        let magnitude = (k * u16::MAX as f32) as u16;
        let replay = Replay { play_for: Ticks::from_ms(ms), ..Default::default() };
        let effect = EffectBuilder::new()
            .add_effect(BaseEffect { kind: BaseEffectType::Strong { magnitude }, scheduling: replay, ..Default::default() })
            .add_effect(BaseEffect { kind: BaseEffectType::Weak { magnitude }, scheduling: replay, ..Default::default() })
            .gamepads(&pads)
            .finish(g);
        if let Ok(e) = effect {
            if e.play().is_ok() {
                state.rumble = Some(e);
            }
        }
    });
}

// ---------------------------------------------------------------- touch

/// The buttons on a touch screen, at the right edge, and the pause button by the clock.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TouchButton {
    Roll,
    Skill,
    Ping,
    Pause,
}

pub const TOUCH_BUTTONS: [TouchButton; 4] = [TouchButton::Roll, TouchButton::Skill, TouchButton::Ping, TouchButton::Pause];

impl TouchButton {
    pub fn label(self) -> &'static str {
        match self {
            TouchButton::Roll => "Roll",
            TouchButton::Skill => "Skill",
            TouchButton::Ping => "Ping",
            TouchButton::Pause => "II",
        }
    }

    /// Where the button sits and how big it is, in UI space.
    pub fn circle(self) -> (Vec2, f32) {
        let (w, h) = (crate::ui::ui_w(), crate::ui::ui_h());
        match self {
            TouchButton::Roll => (vec2(w - 120.0, h - 150.0), 62.0),
            TouchButton::Skill => (vec2(w - 255.0, h - 95.0), 46.0),
            TouchButton::Ping => (vec2(w - 255.0, h - 215.0), 34.0),
            TouchButton::Pause => (vec2(w * 0.5 + 105.0, 30.0), 24.0),
        }
    }
}

/// How far the thumb may move the stick from where it went down, in UI space.
pub const STICK_REACH: f32 = 70.0;

#[derive(Default)]
struct TouchState {
    /// The stick's touch: its id, where it went down and where it is now (UI space).
    stick: Option<(u64, Vec2, Vec2)>,
    pressed: Vec<TouchButton>,
    held: Vec<TouchButton>,
    /// Touches already counted as a press of a button, so a held finger presses only once.
    counted: Vec<u64>,
    /// The screen has been touched: from then on the touch controls show.
    used: bool,
}

thread_local! {
    static TOUCH: RefCell<TouchState> = RefCell::new(TouchState::default());
}

/// Reads this frame's touches: the stick on the left part of the screen, buttons on the right.
fn poll_touch() {
    let scale = crate::ui::ui_scale();
    TOUCH.with(|t| {
        let mut t = t.borrow_mut();
        t.pressed.clear();
        t.held.clear();
        let mut stick_alive = false;
        for touch in touches() {
            t.used = true;
            let p = touch.position / scale;
            let on = TOUCH_BUTTONS.into_iter().find(|b| {
                let (c, r) = b.circle();
                c.distance(p) < r * 1.15
            });
            match touch.phase {
                TouchPhase::Started => {
                    if let Some(b) = on {
                        t.pressed.push(b);
                        t.held.push(b);
                        t.counted.push(touch.id);
                    } else if p.x < crate::ui::ui_w() * 0.45 && t.stick.is_none() {
                        t.stick = Some((touch.id, p, p));
                        stick_alive = true;
                    }
                }
                TouchPhase::Moved | TouchPhase::Stationary => {
                    if let Some((id, origin, _)) = t.stick {
                        if id == touch.id {
                            t.stick = Some((id, origin, p));
                            stick_alive = true;
                        }
                    }
                    if let Some(b) = on {
                        t.held.push(b);
                        // A start we never saw still counts as a press.
                        if t.stick.map(|s| s.0) != Some(touch.id) && !t.counted.contains(&touch.id) {
                            t.pressed.push(b);
                            t.counted.push(touch.id);
                        }
                    }
                }
                TouchPhase::Ended | TouchPhase::Cancelled => {
                    // A quick tap can go down and up within one frame: only its end is seen.
                    if let Some(b) = on {
                        if touch.phase == TouchPhase::Ended && !t.counted.contains(&touch.id) && t.stick.map(|s| s.0) != Some(touch.id) {
                            t.pressed.push(b);
                        }
                    }
                    t.counted.retain(|&id| id != touch.id);
                }
            }
        }
        if !stick_alive {
            t.stick = None;
        }
    });
}

/// Where the touch stick points, scaled to its reach: zero without a thumb on it.
pub fn touch_dir() -> Vec2 {
    TOUCH
        .with(|t| {
            t.borrow().stick.map(|(_, origin, at)| {
                let d = at - origin;
                if d.length() < 8.0 {
                    Vec2::ZERO
                } else {
                    d.normalize()
                }
            })
        })
        .unwrap_or(Vec2::ZERO)
}

/// The stick as it is drawn: where it went down and where the thumb is.
pub fn touch_stick() -> Option<(Vec2, Vec2)> {
    TOUCH.with(|t| t.borrow().stick.map(|(_, o, p)| (o, o + (p - o).clamp_length_max(STICK_REACH))))
}

pub fn touch_pressed(b: TouchButton) -> bool {
    TOUCH.with(|t| t.borrow().pressed.contains(&b))
}

pub fn touch_held(b: TouchButton) -> bool {
    TOUCH.with(|t| t.borrow().held.contains(&b))
}

/// Whether to show the touch controls: always on a phone, else once the screen was touched.
pub fn touch_shown() -> bool {
    cfg!(target_os = "android") || TOUCH.with(|t| t.borrow().used)
}

/// Reads all gamepad events and the mouse; call once at the start of every frame.
pub fn poll_gamepad() {
    poll_touch();
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

thread_local! {
    /// A button the self-test presses this frame, as if on a gamepad.
    static FAKE_PAD: std::cell::Cell<Option<Pad>> = const { std::cell::Cell::new(None) };
}

/// Presses `p` for this frame (the self-test's hands); None lets go.
pub fn press_fake(p: Option<Pad>) {
    FAKE_PAD.with(|f| f.set(p));
}

pub fn pad_pressed(action: Pad) -> bool {
    if FAKE_PAD.with(|f| f.get()) == Some(action) {
        return true;
    }
    PAD.with(|p| {
        let p = p.borrow();
        let has = |b: Button| p.pressed.contains(&b);
        match action {
            Pad::Confirm => has(Button::South),
            Pad::Back => has(Button::East),
            Pad::Roll => has(Button::South) || has(Button::RightTrigger) || has(Button::RightTrigger2),
            Pad::Pause => has(Button::Start),
            Pad::Reroll => has(Button::North),
            Pad::Skill => has(Button::West),
            Pad::PageNext => has(Button::RightTrigger),
            Pad::PagePrev => has(Button::LeftTrigger),
            Pad::Ping => has(Button::RightThumb),
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

/// Escape or a phone's back button (keys only).
pub fn back_key() -> bool {
    is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Back)
}

/// Escape, B, or a phone's back button.
pub fn back() -> bool {
    is_key_pressed(KeyCode::Escape) || is_key_pressed(KeyCode::Back) || pad_pressed(Pad::Back)
}
