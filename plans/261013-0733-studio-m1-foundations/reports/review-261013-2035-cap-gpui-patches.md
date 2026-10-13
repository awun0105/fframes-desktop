# Review: Cap GPUI fork (`zed-cap` `cap-patches` @ `cb5d0daaa8`)

Read-only review. Fork: `C:\Users\Administrator\code\zed-cap`. All `file:line` refs are at `cb5d0daaa8`
(paths relative to `crates/`). Inputs: `git show` of the three patch commits and
`git diff f14fea9bf3 5d1f83d9f2`. **No cargo build or test was run**; compile and runtime claims come
from reading the code and from greps.

Context for fframes: the desktop app runs on Windows (DirectX renderer, `gpui_windows`). Both of its
windows open with `titlebar: Some(TitlebarOptions { appears_transparent: false, .. })`, `kind: Normal`
(`desktop/app/src/main.rs:298-307`, `357-366`). `gpui_wgpu` and the wgpu/Linux changes apply to Linux only.

## Verdict

Safe to adopt **at `cb5d0daaa8` only**. On Windows, do not pin `5d1f83d9f2` or `267b344c7f`: the
wingleeio commits add `fade` to the Rust `Quad`/`PolychromeSprite` without updating HLSL, so the
DirectX structured-buffer strides do not match until `8aa700d61d`. No unsafe block that fframes reaches
breaks its invariants. Nothing makes network or telemetry calls. The one environment variable is
`COMET_GPU_STATS`, which is macOS-only and writes to local stderr. The defects found are a broken
macOS test build, an API that is unimplemented on two of three OSes, cosmetic shader inconsistencies,
and a few changes to global defaults.

## 1. Per-patch summary

### wingleeio `f14fea9bf3..5d1f83d9f2` (17 commits, 12 files, +1332/-39)

| Area | Change | Risk |
|---|---|---|
| gpui scene/window | `EdgeFade` + `Window::with_edge_fade` (`gpui/src/window.rs:682`, `3460`). Per-pixel fade params `EdgeFadeParams` (`scene.rs:563`) added as `pub fade` on `Quad` (`scene.rs:586`) and `PolychromeSprite` (`scene.rs:815`). CPU-side opacity for shadows, paths, glyphs, SVG and underlines | low (opt-in; zeroed = no-op) |
| gpui scene/window | `BackdropBlur` primitive + `Window::paint_backdrop_blur` (`window.rs:4016`), `Scene.backdrop_blurs` (`scene.rs:56`). Inserts a transparent `Shadow` "splitter" on every platform | low (non-Metal renderers ignore the blur and draw an invisible shadow) |
| gpui img | `Window::paint_image_fitted` (`window.rs:4491`) crops atlas-tile UVs for object-fit cover; `Img` paints `bounds ∩ fitted` | low (changes `Img` cover rendering for all apps; corners now round correctly) |
| gpui assets | `App::peek_asset` (`app.rs:2487`), `ImageSource::evict` (`elements/img.rs:599`) | none |
| gpui text | Wrapping no longer breaks before a non-word char that directly follows a word char (`line_wrapper.rs`, `line_layout.rs`) | low (global; a Latin word followed by CJK, e.g. `world你好`, loses that break opportunity) |
| macOS Metal | Backdrop blur via MPS gaussian, scratch-texture release after 30 idle frames (`metal_renderer.rs:51`), lazy path intermediates, instance-pool shrink, `COMET_GPU_STATS` | low |
| macOS Metal | Destination-alpha blend changed from `One` to `OneMinusSourceAlpha` in all blend pipelines (`metal_renderer.rs:2200`, `2305`, `2355`) | low (correct Porter-Duff; affects transparent windows of every app) |
| macOS window | Blurred windows: `UnderWindowBackground` material (`gpui_macos/src/window.rs:3080`), black layer base (`3102`), backdrop filter radius forced to 60 through KVC (`3127-3136`) | medium (macOS; changes the look for every app that uses `Blurred`; see U5) |
| wgpu WGSL | Same fade on quads and poly sprites (`gpui_wgpu/src/shaders.wgsl`) | low (see U4) |

### `267b344c7f` zed-gpui.patch

| Area | Change | Risk |
|---|---|---|
| Platform trait | `Platform::try_write_to_clipboard` with a default that always errors (`gpui/src/platform.rs:301`); `App::try_write_to_clipboard` (`app.rs:1354`) | medium (API trap, see U2) |
| PlatformWindow trait | `retained_visibility` / `set_retained_visibility`, with defaults returning `None` / error (`platform.rs:823`, `827`); `Window` wrappers (`window.rs:2462`, `2467`) | none (defaults are safe) |
| Frame loop | An inactive window that is dirty is no longer capped at 30 fps (`window.rs:1520-1530`) | low (global; background windows can draw more often) |
| Surfaces (macOS) | `Window::paint_surface_fitted` (`window.rs:4580`); `PaintSurface` gains `pub corner_radii`, `pub source_uv` (`scene.rs:831`, `834`); BGRA `CVPixelBuffer` support; the `assert_eq!` on pixel format replaced with a throttled log | low (removes a panic) |
| Metal hardening | Device, library, pipelines and CV texture cache now return `Result` instead of `expect`/`exit(1)`; `MacWindow::open` returns `Result`; `MetalHeadlessRenderer::new` returns `Result` | low (API change inside `gpui_macos`) |
| Metal surfaces | `CVMetalTexture` wrappers retained until command-buffer completion (`metal_renderer.rs:2093`) | none (fixes a use-after-recycle) |
| macOS drop | AccessKit adapter dropped on the next main-thread tick, after the state lock is released (`gpui_macos/src/window.rs:1175`) | none |
| wgpu | `WgpuResources.surface` becomes `Option`; `take_surface_for_visibility` (`wgpu_renderer.rs:1702`) | low |
| Transparent alpha | Covered by the Metal blend change above; this patch adds no further alpha change | — |

### `8aa700d61d` zed-windows.patch (the part that matters for fframes)

| Area | Change | Risk |
|---|---|---|
| HLSL | `EdgeFadeParams` added to the HLSL `Quad` (`gpui_windows/src/shaders.hlsl:533`) and `PolychromeSprite` (`1244`) in the same field order as Rust, with fade applied (`586-587`, `695`, `1289`). Test `compiled_shader_strides_match_uploaded_primitives` disassembles the shaders and checks `dcl_resource_structured t1, <size_of>` (`directx_renderer.rs:1991-2020`) | none (required; it fixes the stride mismatch the wingleeio commits introduced) |
| DirectX atlas | When a polychrome texture empties, it is kept for reuse and any previously empty one is freed, so at most one empty texture is retained (`directx_atlas.rs:111-139`) | low (holds one texture of GPU memory at idle) |
| Custom frame | `custom_frame = Normal && titlebar.appears_transparent == Some(true)` (`window.rs:415`). Such windows get `WS_CAPTION`, `WS_EX_WINDOWEDGE` (`507`), and an NCCALCSIZE that insets left/right/bottom by the frame and the top by 1-3 px on build ≥ 22000 (`events.rs:717-727`, `1672-1695`) | low (fframes: `appears_transparent: false`, so `custom_frame=false`) |
| Window styles | `PopUp` style changes from `0` to `WS_POPUP` (`window.rs:481`). A `Normal` window with `hide_title_bar && !custom_frame` gets `WS_POPUP`, otherwise `WS_CAPTION` (`484-488`) | low for fframes: its style gains an explicit `WS_CAPTION`, which the system already adds to overlapped windows. medium for apps that pass `titlebar: None` or use `PopUp` |
| Taskbar-aware fitting | Initial placement and `resize()` clamp the outer rect into the display work area while honoring the minimum size (`window.rs:1612`, `661`, `1658-1675`) | low (window is moved or shrunk instead of ending up under the taskbar) |
| Workspace coords | `rcNormalPosition` translated screen→workspace by `work.origin - monitor.origin` of the window's display (`window.rs:217`, `1619`, `1627-1636`) | medium on multi-monitor setups with a top/left taskbar (unverified, see U6) |
| `resize()` | Drops `SWP_NOMOVE`, adds `SWP_NOACTIVATE \| SWP_NOZORDER` (`window.rs:647-695`). Resize no longer raises z-order and can move the window into the work area | low (display is refreshed on `WM_MOVE`, `events.rs:147`) |
| `start_window_move` | New Windows implementation: checks the left button, calls `ReleaseCapture`, then posts `WM_SYSCOMMAND(SC_MOVE\|HTCAPTION)` with packed screen coordinates (`window.rs:700-725`) | low (see U3) |
| DPI change | After `SetWindowPos`, re-reads the client rect and calls `handle_size_change` for both maximized and normal windows (`events.rs:880-893`) | none (fixes outer-vs-client size confusion) |

### `cb5d0daaa8` zed-linux.patch

| Area | Change | Risk |
|---|---|---|
| gpui | `wayland` feature pulls in `wayland-client 0.31.11` (`gpui/Cargo.toml:32`, `114`), matching gpui_linux; adds `PlatformWindow::wayland_surface` and `Window::wayland_surface` (`platform.rs:799`, `window.rs:1936`) | none (cfg-gated) |
| Wayland | Retained visibility state machine (unmap via a null buffer plus `wl_display.sync`, remap with fresh configure), retiring the surface on a background thread (`wayland/window.rs:1052-1170`). Extensive unit tests | medium (complex, Wayland only) |
| Wayland | Fixed-size windows stay fixed through min=max geometry (`update_size_constraints`); fullscreen targets the preferred output (`664`) | low |
| Clipboard | Wayland and X11 can serve images and file lists (`text/uri-list`, `x-special/gnome-copied-files`) and expose `try_write_to_clipboard`. Wayland pipe made non-blocking (`wayland/clipboard.rs:351-357`) | low |
| X11 | `WM_DELETE_WINDOW` and `should_close` / `close` no longer hold the `RefCell` borrow across callbacks; the loop drains buffered events after each dispatch (`x11/client.rs:1816-1823`) | low (fixes re-entrancy panics) |
| wgpu | `SurfaceFrame` destroys the acquired texture after present (`gpui_wgpu/src/surface_frame.rs:32`). Suboptimal frames are presented, then the surface is reconfigured (`wgpu_renderer.rs:1338`) | low (Linux only; `destroy()` on a surface texture not verified against wgpu internals) |

## 2. Unsafe or risky (fork `file:line`)

**U1. macOS test build is broken.** `gpui_macos/src/metal_renderer.rs:2572` builds `Quad { .. }` with
no `fade` field and no `..Default::default()`, so `cargo test -p gpui_macos` cannot compile.
(`gpui/src/window.rs:4141` uses `..quad`, so gpui itself compiles.) fframes does not run these tests,
but the fork's macOS CI would fail.

**U2. `try_write_to_clipboard` works only on Linux.** Only `gpui_linux` implements it (`linux/platform.rs:741`,
`wayland/client.rs:1126`, `x11/client.rs:1760`). Windows (`gpui_windows/src/platform.rs:775` implements
only `write_to_clipboard`), macOS (`gpui_macos/src/platform.rs:1097`) and `TestPlatform` all fall
through to the default, which always returns `Err` (`gpui/src/platform.rs:301`).
`App::try_write_to_clipboard` (`app.rs:1354`) therefore always fails on Windows. fframes uses
`write_to_clipboard` (`desktop/app/src/text_input.rs:278`, `286`) and is unaffected. Do not switch
to the `try_` variant.

**U3. `start_window_move` uses physical button state.** `gpui_windows/src/window.rs:709` checks
`GetAsyncKeyState(VK_LBUTTON)`, and per Win32 docs that reports the *physical* left button. With
swapped mouse buttons, a logical-left drag silently does nothing. The rest is fine: the lparam packing
(`cursor.x as u16`) round-trips negative multi-monitor coordinates, `PostMessageW` to a destroyed HWND
only logs an error, and the HWND is owned by the `Rc` captured in the task. While the move is active,
the native modal move loop swallows the button-up, the same as a native caption drag.

**U4. WGSL fades the quad fill twice.** `gpui_wgpu/src/shaders.wgsl:610-611` multiplies
`background_color.a` by the fade, and `929` multiplies the final alpha by it again. On Linux the
non-fast-path fill gets ramp⁴ and the border gets ramp², while Metal and HLSL give ramp² for both
(`shaders.hlsl:586-587`, `695`). This is cosmetic and only shows inside `with_edge_fade`.

**U5. macOS private KVC on a CA filter.** `gpui_macos/src/window.rs:3127-3136` finds a backdrop filter
by matching `description` against "Blur" and calls `setValue:forKey:"inputRadius"`. If a future macOS
filter is not KVC-compliant for that key, `NSUndefinedKeyException` crosses the Rust FFI boundary and
aborts. Reachable only with `WindowBackgroundAppearance::Blurred` on macOS. Together with `3080` and
`3102`, this is a visible default change for every app that uses Blurred.

**U6. Workspace-coordinate assumption (unverified).** `window.rs:1627-1636` assumes workspace
coordinates for the window's own display are `screen - (rcWork.origin - rcMonitor.origin)`. Microsoft's
WINDOWPLACEMENT docs say only that workspace coordinates account for the taskbar. They do not say which
monitor's work area applies, and my search did not settle it. On one monitor this is correct, and the
tests at `window.rs:1838-1841` only check round-tripping. **Action:** before release, manually test a
secondary monitor with the taskbar on top or left, including creation placement and `bounds()`
read-back (`window.rs:217`).

**U7. Mixed-DPI rounding (low).** `workspace_offset` and `fit_window_rect` convert
`display.visible_bounds()` with the *window's* scale factor (`window.rs:661-668`, `1615`,
`1633`), but `visible_bounds` was computed with the *display's* scale (`display.rs:61-66`). Right
after a cross-DPI move the two can differ until `WM_DPICHANGED`, which leaves at most a few pixels of
clamp error.

**U8. Retained empty atlas texture (low).** `directx_atlas.rs:128-133` keeps one fully-unreferenced
polychrome texture alive. Polychrome textures grow to the largest image requested, so after a very large
image is evicted, up to one such texture (e.g. 4K RGBA ≈ 64 MB) stays resident. The index found by
`position` before removal cannot equal the slot being freed, because that slot still had at least one
live key. The borrow pattern is sound.

**U9. Wayland panics on a dead connection (low).** `wayland/window.rs:1077` (`expect("Live surface
connection required")`) and `1325` (`.unwrap()` on `backend().upgrade()`) panic if the Wayland
connection has gone away. The same pattern exists upstream at `2163`.

**U10. X11 loop stops on an event error (low).** `x11/client.rs:1816-1823` calls `loop_signal.stop()`
when `process_x11_events` errors. Its errors come from `poll_for_event` connection failures
(`x11/client.rs:568-580`), so in practice this triggers only when the X connection is lost.

**U11. Global behavior changes for apps other than Cap (low).** Unfocused dirty windows render at full
rate (`gpui/src/window.rs:1530`). The wrap rule changes for all text (see table). Metal dst-alpha
blending changes. `Img` cover now paints the cropped visible rect.

### Checked and fine
- NCCALCSIZE cast to `NCCALCSIZE_PARAMS`: guarded by `wparam.0 == 0 → return None` (`events.rs:713`, `721`).
  This matches the Win32 contract (the pointer is valid only when wParam is TRUE).
- `RtlGetVersion` with `dwOSVersionInfoSize` set and a `OnceLock` cache (`events.rs:1681-1695`); fine.
- `fit_window_rect` (`window.rs:1658-1675`): every `clamp(min, max)` has `max >= min` (via `.max(...)`), so it cannot panic.
- `D3DDisassemble` / `from_raw_parts` (`directx_renderer.rs:2006-2012`): test-only, and the blob owns the buffer while the slice is in use.
- MPS kernel: `alloc`/`initWithDevice:sigma:` (`float` matches `f32`), `setEdgeMode: 1` (= Clamp, NSUInteger).
  It is released when sigma changes, on idle release and in `Drop` (`metal_renderer.rs:1377`, `1391`, `1398-1403`). Balanced.
- `CVMetalTextureGetTexture` results are null-checked before `TextureRef::from_ptr` (`metal_renderer.rs:1962-2000`).
  Wrappers are kept until completion (`2093`); dropping on Metal's completion thread is a thread-safe CFRelease.
- wgpu `.expect("Configured surface missing")` (`wgpu_renderer.rs:1126`): guarded by the `surface_configured`
  early return (`1092`), which `take_surface_for_visibility` clears (`1703`).
- `fcntl` F_GETFL/F_SETFL on an owned fd with errno checked (`wayland/clipboard.rs:351-357`).
- Network and telemetry: none. Grepping all four diffs for `env::var|var_os|reqwest|http|telemetry|Command::new`
  finds only `COMET_GPU_STATS` (`metal_renderer.rs:1194`, opt-in, `eprintln!` at `1226`) and a URL in a comment.

## 3. Public API additions and changes

Additions that downstream code may use:
- `Window::with_edge_fade`, `EdgeFade`, `EdgeFadeParams`, `Window::paint_backdrop_blur` (Metal only), `Window::paint_image_fitted`.
- `App::peek_asset`, `ImageSource::evict`, `App::try_write_to_clipboard` (Linux only in practice, see U2).
- `Window::retained_visibility` / `set_retained_visibility` (Wayland only; elsewhere `None` / `Err`).
- `Window::paint_surface_fitted` (macOS). `Window::wayland_surface` (Linux + `wayland`).
- `PlatformWindow::start_window_move` already existed. Windows now implements it (it was a no-op default).

Breaking changes for code that builds these types or calls these APIs:
- New `pub` fields `Quad.fade`, `PolychromeSprite.fade`, `PaintSurface.corner_radii` / `source_uv`, and `Scene.backdrop_blurs`.
  Struct literals without `..Default::default()` stop compiling. **fframes: not affected.** Grepping `desktop/`
  finds no literals of these types.
- `gpui_macos`: `MetalRenderer::new`, `new_headless`, `MetalHeadlessRenderer::new` return `Result`. `SurfaceBounds` loses `Eq`.
- `gpui_wgpu`: the internal `WgpuResources.surface` is `Option`. No public signature changed apart from the new `take_surface_for_visibility`.
- Trait additions on `Platform` / `PlatformWindow` all have default bodies, so out-of-tree platform impls still compile.
- Behavior (not signature): Windows window styles for `PopUp` and for `titlebar: None`; `resize()` placement; wrap rule.
  `WindowOptions::default()` keeps `titlebar: Some(appears_transparent: false)` (`gpui/src/platform.rs:1937`),
  so default windows are unaffected by the `WS_POPUP` branch.

## 4. Test-platform gaps (`crates/gpui/src/platform/test/`)

- `TestPlatform` does not override `try_write_to_clipboard`. Under `#[gpui::test]` it always returns `Err`,
  although `write_to_clipboard` works (`test/platform.rs:541`).
- `TestWindow::start_window_move` and `show_window_menu` are `unimplemented!()` (`test/window.rs:330`, `334`).
  This was already true upstream at `f14fea9bf3`. With Windows now implementing `start_window_move`, a
  titlebar-drag handler that calls `window.start_window_move()` will panic in gpui tests. Guard it or avoid
  it in test paths.
- `retained_visibility` / `set_retained_visibility` / `wayland_surface` fall back to the defaults (`None` / `Err`).
  They cannot panic, but the visibility flow cannot be exercised in gpui tests. Only the state machine has unit
  tests (`wayland/window.rs`, `retained_visibility_tests`).
- Edge fade, backdrop blur and `paint_image_fitted` produce scene primitives that `TestWindow` never renders,
  so only CPU-side opacity math is observable in tests.

## 5. Recommended actions for fframes

1. Pin `cb5d0daaa8` exactly. Never pin the intermediate commits on Windows (stride mismatch).
2. On Windows CI, run `cargo test -p gpui_windows` from the fork once. It covers the stride test, insets and fit/translate.
3. Run the multi-monitor and top/left taskbar placement test from U6 before shipping restore-bounds behavior.
4. Do not call `cx.try_write_to_clipboard`, or `window.start_window_move()` from code reachable in `#[gpui::test]`.
5. Optional upstream feedback to the fork: fix the `push_quad` literal (U1), the WGSL double fade (U4),
   implement `try_write_to_clipboard` on Windows and macOS (U2), and use `GetSystemMetrics(SM_SWAPBUTTON)` in `start_window_move` (U3).

Status: DONE_WITH_CONCERNS
Summary: The fork at `cb5d0daaa8` is safe for fframes on Windows: no broken unsafe invariants, no network or telemetry, and fframes' window options avoid the changed style paths. Concerns are a broken macOS test build, a clipboard API that works only on Linux, an unverified multi-monitor workspace-coordinate assumption, and a few changed global defaults.
Concerns/Blockers: No cargo build or test was run. U6 needs a manual multi-monitor check.
