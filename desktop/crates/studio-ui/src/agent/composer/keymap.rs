//! The composer's actions and key bindings.
//!
//! Every binding is scoped to [`KEY_CONTEXT`], which the kit text input does
//! not use, so the two never shadow each other. Nothing here is a bare
//! printable key: gpui matches a binding before the input method composes the
//! character, so binding `a` would make the letter untypeable.

use gpui::{App, KeyBinding, actions};

/// The key context the composer input carries.
pub const KEY_CONTEXT: &str = "AgentComposer";

actions!(
    agent_composer,
    [
        Backspace,
        Delete,
        Left,
        Right,
        Up,
        Down,
        SelectLeft,
        SelectRight,
        SelectUp,
        SelectDown,
        SelectAll,
        Home,
        End,
        SelectHome,
        SelectEnd,
        DocStart,
        DocEnd,
        SelectDocStart,
        SelectDocEnd,
        WordLeft,
        WordRight,
        SelectWordLeft,
        SelectWordRight,
        DeleteWordLeft,
        DeleteWordRight,
        DeleteToLineStart,
        DeleteToLineEnd,
        Copy,
        Cut,
        Paste,
        Newline,
        Submit,
        SubmitNow,
        Undo,
        Redo,
        AcceptCompletion,
    ]
);

/// The bindings for `macos` or for the other platforms.
///
/// They differ in more than the command key: on macOS Cmd+arrows go to the
/// line and document edges and Option moves by word; elsewhere Ctrl moves by
/// word and Home and End go to the line edges.
pub fn bindings(macos: bool) -> Vec<KeyBinding> {
    let ctx = Some(KEY_CONTEXT);
    let mut bindings = vec![
        // Enter sends, Shift+Enter breaks the line. While a completion is open the
        // input redirects both to the completion (see `ComposerInput::submit`).
        KeyBinding::new("enter", Submit, ctx),
        KeyBinding::new("shift-enter", Newline, ctx),
        KeyBinding::new("tab", AcceptCompletion, ctx),
        KeyBinding::new("backspace", Backspace, ctx),
        KeyBinding::new("shift-backspace", Backspace, ctx),
        KeyBinding::new("delete", Delete, ctx),
        KeyBinding::new("left", Left, ctx),
        KeyBinding::new("right", Right, ctx),
        KeyBinding::new("up", Up, ctx),
        KeyBinding::new("down", Down, ctx),
        KeyBinding::new("shift-left", SelectLeft, ctx),
        KeyBinding::new("shift-right", SelectRight, ctx),
        KeyBinding::new("shift-up", SelectUp, ctx),
        KeyBinding::new("shift-down", SelectDown, ctx),
        KeyBinding::new("home", Home, ctx),
        KeyBinding::new("end", End, ctx),
        KeyBinding::new("shift-home", SelectHome, ctx),
        KeyBinding::new("shift-end", SelectEnd, ctx),
    ];
    if macos {
        bindings.extend([
            KeyBinding::new("cmd-enter", SubmitNow, ctx),
            KeyBinding::new("cmd-left", Home, ctx),
            KeyBinding::new("cmd-right", End, ctx),
            KeyBinding::new("cmd-up", DocStart, ctx),
            KeyBinding::new("cmd-down", DocEnd, ctx),
            KeyBinding::new("shift-cmd-left", SelectHome, ctx),
            KeyBinding::new("shift-cmd-right", SelectEnd, ctx),
            KeyBinding::new("shift-cmd-up", SelectDocStart, ctx),
            KeyBinding::new("shift-cmd-down", SelectDocEnd, ctx),
            KeyBinding::new("cmd-backspace", DeleteToLineStart, ctx),
            KeyBinding::new("cmd-delete", DeleteToLineEnd, ctx),
            KeyBinding::new("alt-backspace", DeleteWordLeft, ctx),
            KeyBinding::new("alt-delete", DeleteWordRight, ctx),
            KeyBinding::new("alt-left", WordLeft, ctx),
            KeyBinding::new("alt-right", WordRight, ctx),
            KeyBinding::new("shift-alt-left", SelectWordLeft, ctx),
            KeyBinding::new("shift-alt-right", SelectWordRight, ctx),
            KeyBinding::new("cmd-a", SelectAll, ctx),
            KeyBinding::new("cmd-c", Copy, ctx),
            KeyBinding::new("cmd-x", Cut, ctx),
            KeyBinding::new("cmd-v", Paste, ctx),
            KeyBinding::new("cmd-z", Undo, ctx),
            KeyBinding::new("shift-cmd-z", Redo, ctx),
        ]);
    } else {
        bindings.extend([
            KeyBinding::new("ctrl-enter", SubmitNow, ctx),
            KeyBinding::new("ctrl-home", DocStart, ctx),
            KeyBinding::new("ctrl-end", DocEnd, ctx),
            KeyBinding::new("shift-ctrl-home", SelectDocStart, ctx),
            KeyBinding::new("shift-ctrl-end", SelectDocEnd, ctx),
            KeyBinding::new("ctrl-backspace", DeleteWordLeft, ctx),
            KeyBinding::new("ctrl-delete", DeleteWordRight, ctx),
            KeyBinding::new("ctrl-left", WordLeft, ctx),
            KeyBinding::new("ctrl-right", WordRight, ctx),
            KeyBinding::new("shift-ctrl-left", SelectWordLeft, ctx),
            KeyBinding::new("shift-ctrl-right", SelectWordRight, ctx),
            KeyBinding::new("ctrl-a", SelectAll, ctx),
            KeyBinding::new("ctrl-c", Copy, ctx),
            KeyBinding::new("ctrl-x", Cut, ctx),
            KeyBinding::new("ctrl-v", Paste, ctx),
            KeyBinding::new("shift-insert", Paste, ctx),
            KeyBinding::new("ctrl-insert", Copy, ctx),
            KeyBinding::new("shift-delete", Cut, ctx),
            KeyBinding::new("ctrl-z", Undo, ctx),
            KeyBinding::new("ctrl-y", Redo, ctx),
            KeyBinding::new("shift-ctrl-z", Redo, ctx),
        ]);
    }
    bindings
}

/// Binds the composer's keys. The owner calls this once at start; tests call it
/// in their setup.
pub fn init(cx: &mut App) {
    cx.bind_keys(bindings(cfg!(target_os = "macos")));
}

/// What Enter does, given whether a completion row is selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EnterOutcome {
    /// Accept the highlighted completion.
    AcceptCompletion,
    /// Send the message.
    Submit,
    /// Insert a line break.
    Newline,
}

/// A selected completion row takes Enter before anything else does.
pub fn enter_outcome(has_completion: bool, fallback: EnterOutcome) -> EnterOutcome {
    if has_completion {
        EnterOutcome::AcceptCompletion
    } else {
        fallback
    }
}

/// Whether Escape closes an open completion rather than reaching the owner.
pub fn escape_dismisses_completion(key: &str, completion_open: bool) -> bool {
    key == "escape" && completion_open
}
