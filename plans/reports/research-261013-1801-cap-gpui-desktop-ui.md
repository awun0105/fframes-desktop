# How Cap builds its desktop UI with GPUI

Source: `C:\Users\Administrator\code\Cap` at `a32e4806b`, app in `apps/desktop-gpui`
(≈168k lines of Rust, `README.md` is a 2.4k-line design record). Runtime: the
installed copy under `%LOCALAPPDATA%\Cap`.

## Outcome

Cap rewrote its Tauri + SolidJS desktop app as a pure-Rust gpui app
(`cap-gpui`, shipped as `Cap` from 0.6.1). There is no webview: every window is
a gpui `Entity` whose `render` returns a `div()` tree styled with Tailwind-like
methods. The recording, editor, rendering and export crates are the same ones
the Tauri app used; only the UI layer was replaced. The UI was built as a
**transcription** of the Solid app — Tailwind classes, Radix colours and
`theme.css` rules are quoted in comments next to each value.

## Runtime observation (important caveat)

- **Observed:** the running `Cap.exe` (PID 15844) is **0.6.0, the Tauri
  build**. It loads `EmbeddedBrowserWebView.dll`, and all 7 `msedgewebview2`
  processes descend from it (one child, PID 16492, with 6 children of its own).
- **Observed:** `cap-gpui.exe` (96 MB, version 0.6.0) is installed beside
  `Cap.exe`. It contains `gpui_windows` and `directx_renderer` strings and no
  WebView2 references. It is not running.
- **Inferred:** 0.6.0 shipped the GPUI binary as a sidecar. The NSIS hook
  (`packaging/windows/installer-hooks.nsh:7`) deletes a "Tauri-era
  `cap-gpui.exe`" on update to 0.6.1+.
- **Unknown:** whether that installed binary matches source `a32e4806b`.

The GPUI UI was therefore not observed live. Everything below is read from
source. Claims taken from the README rather than from code are marked
*(README)*.

## 1. Dependency setup

- `apps/desktop-gpui/Cargo.toml` is its **own workspace**, so gpui's large tree
  (its own wgpu 29 and font stacks) does not touch the root `Cargo.lock`,
  workspace-hack or the Tauri-only `[patch.crates-io]` entries.
- gpui comes from the `wingleeio/zed` fork at a pinned rev (needed for correct
  alpha blending on transparent windows), with `gpui_platform` (the
  per-OS backends) and `gpui_tokio` (a tokio runtime bridge).
- Cap carries three patches on that fork (`patches/zed-gpui.patch`,
  `zed-windows.patch`, `zed-linux.patch`). `scripts/prepare-gpui-dependency.sh`
  clones the rev into `../zed-cap` and applies them, and `Cargo.toml`
  redirects the git dependency to that local checkout. The checkout is not
  present at `C:\Users\Administrator\code\zed-cap`, so a build from source must
  run that script first. The patches add, for example, `paint_surface_fitted`
  and 32BGRA CVPixelBuffer support (zero-copy video on macOS), and on Windows: custom-frame insets, `start_window_move`, taskbar-aware
  window fitting, and DirectX atlas/shader fixes.
- Build profile: deps at `opt-level = 2`, app crate at 0 with incremental
  builds (~22 s rebuild). Per-pixel and per-sample loops live in a separate
  `kernels/` crate at `opt-level = 3`. Unoptimised, frame conversion cost
  42.7 ms per frame; in `kernels/` it costs 1.0 ms.

## 2. App bootstrap (`src/main.rs:242-560`)

1. `gpui_platform::application().with_assets(Assets)`: every SVG, PNG and
   font is baked in with `include_bytes!` behind an `AssetSource`
   (`src/assets.rs:253`), and `Assets.load_fonts` registers the fonts with
   `cx.text_system().add_fonts`.
2. Inside `app.run`: `gpui_tokio::init_from_handle`, the text-input keymap
   (`ui::bind_text_input_keys`), and app **globals** (`theme::init`,
   `RecordingSession::init`, `Feeds::init`, `TargetSelect::init`).
3. `cx.open_window(WindowOptions { .. }, |window, cx| cx.new(|cx| MainWindow::new(..)))`.
   The options vary per OS. On macOS the window has no titlebar, a
   transparent background and `WindowKind::Floating`. On Windows it has a
   transparent-titlebar custom frame, an opaque background and is minimisable.
   On Linux it uses client decorations.
4. After the handle exists: `app_windows::init` (the window registry),
   menus, tray, global hotkeys, the updater and onboarding. Native work
   (panel level, Liquid Glass/vibrancy material, Windows frame policy) runs
   from a `cx.spawn` task, never inside the builder closure.

`AppWindows` (`src/app_windows.rs:192`) is a gpui `Global` holding a typed
`WindowHandle<T>` for each window: main, controls bar, camera, settings,
onboarding, mode select, teleprompter, per-display overlays, and per-project
editors keyed by path.

## 3. The view model

This is standard gpui, used consistently:

- **A window is `Entity<V>` + `impl Render for V`.** For example,
  `MainWindow::render` (`src/main_window.rs:2568`) returns
  `div().size_full().flex().flex_col().rounded(px(16.)).bg(theme.shell_bg())…
  .child(self.render_header(..)).child(self.render_body(..))`. Sub-sections are
  plain `fn render_x(&self, cx) -> impl IntoElement` methods.
- **State changes call `cx.notify()`.** gpui re-renders only on invalidation.
  Cross-entity reactivity uses `cx.observe(&entity, …)`,
  `cx.observe_in(&session, window, …)` and `cx.subscribe(&input, handler)`
  for typed events.
- **Events use `cx.listener(|this, event, window, cx| …)`** in `.on_click`,
  `.on_key_down` and `.on_mouse_*`.
- **Theme is a `Copy` struct** (`src/theme.rs:345`) of resolved Radix
  steps (`gray_1..12`, `blue_*`, …) plus `MaterialTokens` for the native glass.
  It is rebuilt per window via `Theme::for_window` and refreshed in `render`
  through `sync_appearance`. `theme::bind_window` forces the native appearance
  and calls `cx.observe_window_appearance` so OS light/dark flips repaint.
- **Background work** runs on `cx.background_executor()` or
  `gpui_tokio::Tokio::spawn`, and results come back via `cx.update` or
  `this.update`. Device enumeration, for example, is kept off the main thread
  because inline it cost about 180 ms of blank window.

## 4. The component library (`src/ui/`)

There are 22 shared components: Button and IconButton, Toggle, Slider,
SegmentedControl, Menu with MenuState, Select, TextInput, Card and Popover,
Collapsible, TabRail, NumberField, ColorPicker, Tooltip, Kbd, and others.
They follow one pattern:

- `#[derive(IntoElement)] struct X { … }` + `impl RenderOnce for X`. This is a
  builder value consumed at render time (`src/ui/button.rs:113, 410`).
- **One named constructor per surface instead of CSS cascading**:
  `::body` (main window), `::settings`, `::glass` (teleprompter) and `::plain`
  (editor). Each constructor quotes the `theme.css` rule it reproduces.
- Handlers are `impl Fn(&ClickEvent, &mut Window, &mut App)`, which is what
  `cx.listener` produces, so components work in any window without generics.
- Stateful pieces are separated as plain structs or entities.
  `MenuState`, `CollapsibleState`, `NumberFieldState` and `TextInputState` are
  unit-tested without a window.
- **TextInput** implements gpui's `EntityInputHandler` with a custom `Element`
  and `window.handle_input`, which gives real IME, dead keys, selection, undo
  and clipboard. Every editing chord is a bound action in the `TextInput` key
  context, so a focused field structurally swallows Backspace and ⌘Z before
  editor shortcuts see them.
- Slider and drag interactions measure bounds with a zero-size `canvas` during
  prepaint. They use a full-window transparent "drag layer" to emulate pointer
  capture.

## 5. Native window integration (`src/platform*.rs`)

gpui exposes no API for window levels, Spaces, materials or panel behaviour,
so `platform` reaches the native window through `raw-window-handle`:

- **macOS:** inserts an `NSGlassEffectView` (macOS 26+) or an
  `NSVisualEffectView` below gpui's Metal view, clips corners on the layer,
  sets level and collection behaviour, and installs a
  `constrainFrameRect` shim so overlays can cover the menu bar. The shell then
  paints a translucent tint over the material.
- **Windows:** keeps the native DWM frame, which provides the rounded corners,
  but draws its own caption buttons (`src/ui/windows_caption.rs`, 46 px
  buttons). It installs a frame policy and a session-end handler, and applies
  capture exclusion (`src/platform/windows/capture_exclusion.rs`). The tray
  uses `tray-icon`.
- **Rule they learned the hard way:** never mutate native window state inside
  a gpui update, because AppKit and Win32 callbacks re-enter the `App` and
  cause "RefCell already borrowed". Grab the handle in the update and act from
  a spawned task.

## 6. Real-time video in the UI

The camera bubble and the editor preview show how they get 60 fps:

- **Producer:** `cap-editor` / `cap-rendering` render on wgpu 25, a separate
  tree from gpui's wgpu. A frame callback fires on foreign threads and pushes
  to a bounded `flume` channel (`make_frame_callback`,
  `src/editor_window.rs:11779`). On macOS and Linux it uses `try_send`, which
  drops the frame when the channel is full. On Windows it uses
  `send_timeout(100ms)`, which blocks the producer for up to 100 ms before
  dropping. The main thread drains the channel.
- **Conversion, Windows and Linux:** there is no zero-copy path. Each
  `RenderedFrame` is un-padded from wgpu's 256-byte row stride and swizzled
  from RGBA to BGRA in `kernels::frame` into a `gpui::RenderImage`
  (`frame_image`, `:563`), which is uploaded to the sprite atlas every frame.
  The camera bubble uses `gpui::img(image)` here (`src/camera_window.rs:1216`).
- **Conversion, macOS only:** the CVPixelBuffer/IOSurface is painted directly
  with the patch-added `window.paint_surface_fitted`, with no copies.
- **Painting:** a dedicated `PreviewFrameView` entity renders a
  `gpui::canvas(..)` that calls `paint_drop_shadows`, `paint_quad` and
  `paint_image`/`paint_surface_fitted` directly (`:647`).
- **Invalidation scoping:** each frame calls `notify()` on only the preview
  child entity. The header, toolbar, sidebar and timeline are separate
  `EditorSectionView` entities mounted with `.cached(StyleRefinement…)`, so
  they do not rebuild at 60 Hz. A per-frame `window.refresh()` was measured to
  drop 224 of 298 frames (code comment, `:6345`).
- **Memory:** the replaced `RenderImage` is released with `window.drop_image`
  so the sprite atlas does not grow.
- Measured result: `playback fps=59.8 dropped=0` *(README; the platform is
  not stated, but it was most likely macOS)*.

## 7. Animation and dev loop

- gpui only paints when invalidated. Animations use `with_animation` for
  one-shot reveals, and ticker tasks that `notify()` for things like the
  Collapsible height or the window resize ease. There is no CSS-like
  transform or mask in this rev, so they skip effects such as the sliding
  indicator, carousel masks, hover lift and backdrop blur.
- `dev.sh` watches the source, rebuilds, and swaps the running binary only
  on a successful build. `src/dev_restore.rs` reopens the same windows,
  page and editor scroll through the same entry points real clicks use, and
  it will not kill the app during a recording.
- Harness environment variables (`CAP_GPUI_AUTO_RECORD`,
  `CAP_GPUI_AUTO_PLAYBACK`, `CAP_GPUI_AUTO_CLOSE`, …) drive real code paths
  for verification without synthetic input.

## Patterns worth reusing

1. Treat each window as an entity, with a global registry of typed handles.
2. Build components as `RenderOnce` builders with per-surface named
   constructors, and keep their state machines in plain testable structs.
3. Give a high-rate surface its own child entity, and mount the other
   sections with `.cached()`. Call `drop_image` for frames you replace.
4. Put hot pixel loops in a separate crate built at `opt-level = 3`.
5. Make native window calls from tasks, never inside a gpui update.
6. Keep a separate cargo workspace for the gpui tree.

## Unresolved questions

- The running app is the Tauri 0.6.0 build, so the GPUI UI was not observed
  live. Launching `%LOCALAPPDATA%\Cap\cap-gpui.exe` would allow runtime
  confirmation. I did not launch it, because it shares the single-instance
  lock and the settings store with the running app.
- The exact contents of the `zed-gpui.patch` hunks were summarised by file
  and function, not reviewed line by line.
