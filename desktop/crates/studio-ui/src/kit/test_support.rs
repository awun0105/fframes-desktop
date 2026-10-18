//! Helpers for driving kit components in a gpui test window.

use std::{cell::RefCell, rc::Rc};

use gpui::{KeyDownEvent, KeyUpEvent, Keystroke, VisualTestContext};

/// A whole key press. gpui activates a focused element on the key's release, and
/// `simulate_keystrokes` sends only the press.
pub fn press(cx: &mut VisualTestContext, key: &str) {
    let keystroke = Keystroke::parse(key).unwrap();
    cx.simulate_event(KeyDownEvent {
        keystroke: keystroke.clone(),
        is_held: false,
        prefer_character_input: false,
    });
    cx.simulate_event(KeyUpEvent { keystroke });
    cx.run_until_parked();
}

/// Moves keyboard focus to the next tab stop, as Tab does in the app.
pub fn tab(cx: &mut VisualTestContext) {
    cx.update(|window, cx| window.focus_next(cx));
    cx.run_until_parked();
}

/// What a test's handlers recorded, in order.
pub type Log<T> = Rc<RefCell<Vec<T>>>;

pub fn log<T>() -> Log<T> {
    Rc::new(RefCell::new(Vec::new()))
}
