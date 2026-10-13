//! Stand-in for Cap's `menus::close_window_by_handle`.

use gpui::App;

pub fn close_window_by_handle(handle: gpui::AnyWindowHandle, cx: &mut App) {
    cx.defer(move |cx| {
        handle
            .update(cx, |_, window, _| window.remove_window())
            .ok();
    });
}
