# Spike S1: Cap's GPUI fork on Windows and macOS

Date: 2026-10-13. Host: this Windows Server 2022 VM over RDP, no GPU, Rust 1.98.1 (MSVC).

## Result

**Pass on Windows; macOS and Linux CI pending a push.** Cap's patched fork renders on this VM's
software adapter, its custom Windows frame works, `#[gpui::test]` builds and passes without a
test-platform commit, Cap's theme and button compile with no edits, fframes' button closure
compiles with no edits, and the whole fframes app needs **three** small source changes.

| Exit criterion | Status |
|---|---|
| Patched mirror exists and is pinned | Done: `khanhthanhdev/zed-cap`, branch `cap-patches`, `c32bfd3962e172b566c9d5514f4fa5cf325e9b06` (Cap's patches at `cb5d0daa`, plus seven upstream fixes) |
| Window opens here | Done (software adapter); see screenshot |
| gpui test passes here | Done |
| gpui test passes on Linux and macOS CI | Pending: jobs added to `.github/workflows/desktop.yml`, not run yet (needs a push and a pull request) |
| API change list and patch review | This report and [the patch review](./review-261013-2035-cap-gpui-patches.md) |

## Mirror

- Forked `wingleeio/zed` as public `khanhthanhdev/zed-cap`. Branch `cap-patches` = `5d1f83d9f2` plus
  one commit per Cap patch, in Cap's script order, taken byte-exact from Cap `a32e4806b`
  (`git show HEAD:apps/desktop-gpui/patches/…`, LF only; the patches are zero-context):
  `267b344c7f` zed-gpui.patch, `8aa700d61d` zed-windows.patch, `cb5d0daaa8` zed-linux.patch.
- Independent check: Cap's own `scripts/prepare-gpui-dependency.sh` (reverse-apply verification of
  an existing `../zed-cap`) passes against a checkout of the mirror.
- Never force-push `cap-patches`; the pin must stay reachable.
- On the user's decision, seven upstream Zed fixes from between the fork's base and our old pin
  sit on top (`git cherry-pick -x`, so each message names its upstream commit):

  | Mirror | Upstream | Fix | Conflict and resolution |
  |---|---|---|---|
  | `5d88ae5b` | `dc1e815e` | complete pending key bindings before IME input | none |
  | `8f611cb7` | `7040aa56` | clear render target before compositing COLR emoji (Windows) | none |
  | `46c62fdc` | `a50a292f` | dropped frames when scrolling an unfocused window | the fork's 30 fps cap already skips frames with fresh content; kept that and added upstream's high-input-rate exemption |
  | `de009449` | `7bddd16a` | `TestWindow` returns `HandleError::NotSupported` instead of panicking in `window_handle` | fix applied; upstream's test needs a helper the fork lacks, dropped |
  | `0d0f5614` | `1d7e5f1d` | window placement on a secondary monitor with different DPI | Cap's patch rewrote `retrieve_window_placement` (work-area fit, workspace offset); kept Cap's body and took every conversion from the target display's scale, not the window's |
  | `42195836` | `f66ed399` | stale pending input when a window is blurred (`Window::blur` now takes `cx`) | conflicts only in upstream tests and Zed app crates; fixed the one remaining `blur()` caller (a `workspace` test) |
  | `c32bfd39` | `59b2ebf1` | `img` keeps an explicit `aspect_ratio` | the three-line fix applied; upstream's test dropped |

  Skipped: `58df5a14` (rounded `ObjectFit::Cover`), already fixed in the fork by `a5b59b53` with
  `paint_image_fitted` (the same upstream issue #44339); `b95b188b` (`TestWindow` scale-factor
  simulation), a test feature that depends on other upstream test-window state.
- Verification: the fork's own workspace cannot resolve on this host (cargo's libgit2 checkout of
  Zed's `pet` dependency exceeds the Windows path limit), so the fixes were checked through the
  spike with a `[patch]` to the local mirror: `gpui` with `test-support`, `gpui_windows` and
  `gpui_platform` compile, all spike tests pass, and the window opens at the right size and DPI
  at every commit of the branch (982×641 at 144 DPI, bisected after one run opened at 96 DPI;
  that run was launched while the RDP desktop was not the input desktop, the same condition that
  logs `GetCursorPos` "Access is denied", and did not reproduce). macOS and Linux are covered by the CI jobs.

## Where the fork sits

`5d1f83d9f2` is upstream Zed `f14fea9bf3` (2026-07-19) plus 17 wingleeio commits (12 files: text
wrap before closing punctuation, `EdgeFade`, `BackdropBlur`, Metal memory bounds,
`ImageSource::evict`/`App::peek_asset`, transparent-window alpha). Our current pin `1a84d5d`
(2026-09-10) is 817 upstream commits past that base, 128 of them in `crates/gpui*` (149 files,
+24.8k/−3.9k). **Moving to the fork steps GPUI back about two months**; the compile check below
shows fframes uses almost none of that churn, and the mirror carries the fixes listed above.

## Git sources besides the fork

From `cargo metadata` over all targets (not just the host), so macOS- and Linux-only crates are included.
Phase 2 mirrors or allow-lists each in `deny.toml`:

| Source | Crates |
|---|---|
| `github.com/proptest-rs/proptest` @ `3dca198a` | proptest, proptest-macro |
| `github.com/zed-industries/font-kit` @ `94b0f281` | zed-font-kit |
| `github.com/zed-industries/scap` @ `4afea48c` | zed-scap |
| `github.com/zed-industries/xim-rs` @ `16f35a2c` | xim-ctext, xim-parser, zed-xim |

## Spike crate

`desktop/spikes/cap-gpui-hello/` (own workspace, deleted in phase 2), pinned to the mirror with
Cap's exact `gpui_platform` features (`wayland`, `x11`, `font-kit`, `runtime_shaders`) and Cap's
dev profile; `Cargo.lock` seeded from Cap's. It inherits `desktop/rust-toolchain.toml` (1.98.1)
on purpose, since phase 2 builds the fork with that toolchain. Two crates:

- `cap-gpui-hello`: Cap's `theme.rs`, `ui/button.rs`, `ui/windows_caption.rs` copied **unchanged**
  at the crate root, plus stand-ins with Cap's exact signatures for what they reach:
  `platform` (appearance enums, `active_material`, no-op theme application, Cap's real Windows
  `native_window`/`zoom_native`), `store::AppTheme`, `menus::close_window_by_handle`, and
  `ui::menu::{OpenHandler, Menu::trigger}` (`trigger` verbatim). The view: a drag header,
  Cap's caption, a rounded card, Cap's button and fframes' button, both dispatching an action.
- `ff-ui`: fframes' `ui/{button,icon,interact,probe,tooltip,kbd}.rs`, `design_system/` and
  `assets.rs`, copied **unchanged** (only a reduced `ui/mod.rs`).

### Results on this VM

- Window: `Using GPU: Microsoft Basic Render Driver`, Direct3D 11.1, Segoe UI. Card, text, both
  buttons and caption glyphs draw ([screenshot](./spike-cap-gpui-hello-window.png), captured with
  `PrintWindow(PW_RENDERFULLCONTENT)`; the dark strips are the invisible resize border).
- Confirmed by the user by hand: dragging the header moves the window; minimize,
  maximize/restore and close work; both buttons increment the counter. Painting stops while the
  RDP client is minimized, and `GetCursorPos` then logs "Access is denied", as already seen for the native harness.
- The fframes button was unreadable at first because fframes' tokens default to dark until
  `design_system::set_appearance` runs (`desktop/app/src/main.rs:294`); the spike now makes the
  same call and the user confirmed it reads correctly in every state. Phase 2 note: any new
  window entry point must keep that call.
- `cargo test --workspace`: 9 tests in `cap-gpui-hello` (Cap's 8 theme tests and the
  `#[gpui::test]` that builds `HelloView` through `VisualTestContext` and dispatches `Increment`
  twice) and fframes' 28 UI and design-system tests, all passing. gpui's `test-support` feature
  builds on the patched fork; **no test-platform mirror commit was needed** on Windows.

### APIs later phases rely on

All present in the fork: `AnyView::cached(StyleRefinement)` (`gpui/src/view.rs:39`),
`App::drop_image` (`app.rs:2578`), `Context::observe_window_appearance` (`app/context.rs:462`),
`EntityInputHandler` (`input.rs:10`), `canvas` with prepaint bounds (`elements/canvas.rs:10`, used
by `Menu::trigger`), `with_animation` (`elements/animation.rs:59`), and the patched
`Window::start_window_move` (`window.rs:2436`), `set_client_inset` (`window.rs:2441`) and
`window_control_area` (`elements/div.rs:683`).

## API changes fframes code needs (sizes phase 2)

Measured by pointing `desktop/Cargo.toml` at the mirror (`cargo update -p gpui -p gpui_platform`)
and running `cargo check -p fframes-studio --all-targets` on Windows (then reverted), at
`cb5d0daa` and again at `c32bfd39` with the same result. The app is the only desktop crate that
depends on GPUI and calls none of the APIs the cherry-picks changed. Three errors, two changes:

| Upstream `1a84d5d` | Mirror `c32bfd39` | Site | Change |
|---|---|---|---|
| `Window::paint_image(bounds, clip, radii, image, frame, grayscale)` | `Window::paint_image_fitted(bounds, clip, …)` (same arguments); `paint_image` takes no clip | `desktop/app/src/preview_element.rs:88` | rename the call |
| `EntityInputHandler::paste` trait method | not in the trait | `desktop/app/src/text_input.rs:443`, `:292` | move `paste` to an inherent `impl TextInput` |

fframes' button closure needed none. Code behind `cfg(target_os = "macos")` or `cfg(target_os = "linux")` in the app
is not covered by this Windows check; the phase 2 CI run covers it. Phase 2's API work is well under
a day; its effort is the `deny.toml` mirrors, CI and the parity run.

## Patch review (summary)

Full review: [review-261013-2035-cap-gpui-patches.md](./review-261013-2035-cap-gpui-patches.md)
(read-only code review, no builds). Nothing unsafe in paths fframes reaches; no network or telemetry
(`COMET_GPU_STATS` is macOS-only, opt-in, stderr). Points that matter to us:

- **Never pin below `cb5d0daa`.** The wingleeio commits add a `fade` field to `Quad` and
  `PolychromeSprite` without updating the DirectX HLSL; `8aa700d61d` (zed-windows.patch) fixes the
  layout and adds a size test. Intermediate commits are broken on Windows.
- fframes' windows use `appears_transparent: false`, so they avoid the new custom-frame path; only
  `titlebar: None` and `WindowKind::PopUp` now get `WS_POPUP`.
- The test platform's `start_window_move` is `unimplemented!()`: a parity test must not reach
  titlebar-drag code (phase 3 note).
- `try_write_to_clipboard` works only on Linux; keep `write_to_clipboard`.
- Medium risk, needs a manual check before shipping window-position restore: Windows work-area
  coordinate conversion on a second monitor with the taskbar at the top or left.
- Low: `start_window_move` ignores swapped mouse buttons; Linux double-fades `EdgeFade` quads;
  unfocused windows with new content render at full rate; no wrap before closing punctuation;
  `resize()` may move a window to stay inside the work area; macOS `Blurred` windows set a
  private filter key, which could abort on a future macOS; `cargo test -p gpui_macos` itself
  does not compile (a test helper lacks `fade`), which does not affect consumers.

## CI

Added to `.github/workflows/desktop.yml` (kept after the spike): `gpui-spike-macos` on `macos-15`
and `gpui-spike-linux` on `ubuntu-24.04` (GPUI's apt packages only, no FFmpeg). Both run
`cargo test --locked --workspace` in the spike, on same-repository pull requests (the workflow's path filter) and on
`workflow_dispatch`. S3 adds its macOS publish tests as another step of `gpui-spike-macos`.
`workflow_dispatch` only works once this file is on the default branch, so the first run needs a
pull request from `feat/studio-rewrite`.

Adding `workflow_dispatch` would also have started `fork-tests` on every dispatch (its guard compares a
pull request head that a dispatch lacks), so that job's `if:` now requires `pull_request`. The
Linux job's apt list covers every `-sys` crate the spike resolves for `x86_64-unknown-linux-gnu`
(fontconfig, freetype via fontconfig, wayland; no OpenSSL). **Phase 2 deletes the spike, so it must
repoint both jobs at the app in the same change.**

## Cost of stepping back

The three-error delta is compile-only: upstream fixes between `f14fea9` and `1a84d5d` that the app
relies on would be lost silently, and phase 3's parity tests, written after the switch, would
record such a regression as today's behavior. The user chose to cherry-pick the relevant fixes
(see Mirror): text input (`dc1e815e`, `f66ed399`), images (`59b2ebf1`; `58df5a14` already fixed by
the fork), Windows (`1d7e5f1d`, `7040aa56`, `a50a292f`) and tests (`7bddd16a`). The other ~120
upstream GPUI commits in that range are not carried; their behavioral cost is still unmeasured.
Phase 3's parity run is the check.

## Open items

- Run both CI jobs (needs pushing `feat/studio-rewrite` and opening a pull request).
- If the macOS job fails to build `test-support`, add the test-platform mirror commit then.
