//! Stand-in for Cap's `src/platform.rs` with the signatures `theme.rs` and
//! `ui/windows_caption.rs` call. The Windows bodies of `native_window` and
//! `zoom_native` are Cap's (`platform/windows.rs`); the rest are no-ops.

use gpui::Window;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForcedAppearance {
    System,
    Light,
    Dark,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MaterialKind {
    LiquidGlass,
    Vibrancy,
}

#[derive(Debug, Clone, Copy, Default)]
pub struct WindowMaterial(pub Option<MaterialKind>);

impl gpui::Global for WindowMaterial {}

pub fn active_material(cx: &gpui::App) -> Option<MaterialKind> {
    cx.try_global::<WindowMaterial>()
        .and_then(|material| material.0)
}

pub fn window_is_dark(_window: &Window) -> Option<bool> {
    None
}

#[cfg(target_os = "windows")]
pub fn apply_window_theme(
    _window: &Window,
    _appearance: ForcedAppearance,
    _executor: &gpui::ForegroundExecutor,
) {
}

#[cfg(not(target_os = "windows"))]
pub fn apply_window_theme(_window: &Window, _appearance: ForcedAppearance) {}

#[cfg(target_os = "windows")]
pub use windows::{NativeWindow, native_window, zoom_native};

#[cfg(target_os = "windows")]
mod windows {
    use gpui::Window;
    use raw_window_handle::{HasWindowHandle, RawWindowHandle};
    use windows_sys::Win32::Foundation::HWND;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        IsZoomed, SW_MAXIMIZE, SW_RESTORE, ShowWindowAsync,
    };

    #[derive(Clone, Copy)]
    pub struct NativeWindow(isize);

    impl NativeWindow {
        fn hwnd(self) -> HWND {
            self.0 as HWND
        }
    }

    pub fn native_window(window: &Window) -> Option<NativeWindow> {
        let handle = HasWindowHandle::window_handle(window).ok()?;
        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return None;
        };
        Some(NativeWindow(handle.hwnd.get()))
    }

    pub fn zoom_native(native: &NativeWindow) {
        unsafe {
            let command = if IsZoomed(native.hwnd()) != 0 {
                SW_RESTORE
            } else {
                SW_MAXIMIZE
            };
            ShowWindowAsync(native.hwnd(), command);
        }
    }
}
