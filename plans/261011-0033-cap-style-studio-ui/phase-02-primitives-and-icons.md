# Phase 2: Primitives, icons and the bounds probe

Status: done (2026-10-12; native telemetry runs not executed on this VM). Depends on: Phase 1. Effort: 1-2 days.

## Goal

Add the small building blocks every component uses: an icon system, surfaces (card, divider, section), text helpers, and one shared way to report an element's on-screen bounds to telemetry. After this phase no component needs to draw its own card or icon.

## Current state

- No `AssetSource` is registered and no `svg()` is used anywhere in `desktop/app/src`.
- Bounds for telemetry are captured by hand-written canvas probes in two places: `StudioShell::button` (`studio_shell.rs:2505`, probe at `:2578-2599`, only while `self.qualifying` and only for ten ids listed at `:2530-2550`) and `ConversationPanel::button` (`conversation_panel.rs:1036`, only for `agent-policy` and `agent-apply`). The preview probe is separate (`studio_shell.rs:3040-3083`).
- `design_system::surface()` draws a bordered rounded box.

## Deliverables

```
desktop/app/assets/icons/*.svg        Lucide subset (ISC), about 30 files
desktop/app/src/assets.rs             AssetSource over embedded files
desktop/app/src/ui/mod.rs             module root, re-exports
desktop/app/src/ui/icon.rs            Icon enum and icon() element
desktop/app/src/ui/surface.rs         card(), card_2(), divider(), section()
desktop/app/src/ui/probe.rs           BoundsProbe, the shared telemetry hook
desktop/app/src/ui/text.rs            label(), muted(), heading() helpers over TextStyle
desktop/packaging/sdk/notices/lucide-LICENSE.txt
```

## API

```rust
// icon.rs
pub enum Icon {
    Play, Pause, SkipBack, SkipForward, StepBack, StepForward, ZoomIn, ZoomOut, Maximize,
    Volume2, VolumeX, Speaker, RefreshCw, Hammer /*build*/, Square /*stop or cancel*/,
    Folder, FolderOpen, File, Image, Palette, Bot, Download, Upload, Plus, X, Check,
    ChevronDown, ChevronRight, MoreHorizontal, Settings, AlertTriangle, Info, CircleCheck,
    History, Save, Eye, Layers, Package,
}
pub fn icon(icon: Icon, size: f32, color: Rgba) -> Svg;   // sets the colour on the svg itself

// surface.rs
pub fn card() -> Div;        // card fill, 1 px line border, radius_card, Shadow::Card
pub fn card_2() -> Div;      // nested surface, no shadow
pub fn divider() -> Div;     // 1 px line
pub fn section(title: impl Into<SharedString>) -> Div; // Label style title in text_2 + children column

// probe.rs: the same mechanism as today (studio_shell.rs:2576-2600): an absolute, full-size
// canvas child whose prepaint callback receives the element bounds and hands them to the
// owner, which stores them in its own `button_bounds: HashMap<String, [f32; 4]>` (`:552`)
pub struct BoundsProbe(Rc<dyn Fn([f32; 4], &mut App)>);
impl BoundsProbe {
    pub fn new(record: impl Fn([f32; 4], &mut App) + 'static) -> Self;
}
pub fn with_probe<E: ParentElement>(element: E, probe: Option<BoundsProbe>) -> E;
// shell side, unchanged storage:
// BoundsProbe::new(move |b, cx| entity.update(cx, |shell, _| { shell.button_bounds.insert(name.into(), b); }))
```

The probe keeps the existing behavior: it records only when the caller passes `Some(probe)`, and the shell passes it only while qualifying and only for the listed ids. The values stay `[left, top, width, height]` from `bounds.left()`, `bounds.top()` and the size, so `tests/x11_shell.rs` (`button_centre`, `:251`), `qualify-m2-playback.py` (`button`, `:206`) and the native scripts read the same values.

## Steps

1. Copy the needed Lucide SVGs (from the Lucide release, not from Cap) into `assets/icons/`, keeping Lucide's file names. Add `lucide-LICENSE.txt` and check that `desktop/scripts/package-phase-zero.py` stages everything in `packaging/sdk/notices/` (add it if it does not).
2. `assets.rs`: an `AssetSource` that serves `icons/*.svg` from `include_bytes!` entries (no extra crate). Register it in `main.rs` with `Application::with_assets` (available at the pinned gpui rev).
3. `icon.rs`: map each variant to its path. gpui's `svg()` keeps only the glyph's alpha and does not inherit `text_color`, so `icon()` always calls `.text_color(color)` on the svg element itself.
4. `surface.rs` and `text.rs` over the Phase 1 tokens.
5. `probe.rs`: move the existing probe logic out of `StudioShell::button` into `with_probe`, then make the shell's and the conversation panel's button helpers call it. Behavior must be byte-identical: same names, same coordinates.
6. Replace `design_system::surface()` with `ui::card()`; delete the old function when no caller is left.

## Tests

| Test | Asserts |
| --- | --- |
| `every_icon_has_an_asset` | each `Icon` path loads from the asset source |
| `asset_source_lists_icons` | `list("icons/")` returns all files |
| `bounds_to_record_matches_existing_format` | the pure conversion from `Bounds<Pixels>` to `[left, top, width, height]` gives the same array as the old inline code |

gpui's `TestAppContext` is not available in this workspace (the `test-support` feature needs git-only dependencies that are not in the lockfile, see the header of `tests/x11_shell.rs`), so rendering behavior is checked through pure functions and the native telemetry runs, not element-tree tests.

Existing telemetry consumers must keep passing unchanged: `cargo test -p fframes-studio --test x11_shell` and `desktop/scripts/qualify-m2-playback.py` (both need Xvfb and xdotool on Linux; record as not run if unavailable).

## Validation

- `cargo test --locked --manifest-path desktop/Cargo.toml -p fframes-studio ui::`
- `cargo clippy ... -D warnings`
- Package build (`desktop/scripts/package-phase-zero.py`) contains the Lucide notice, and icons render from the packaged binary (checked in Phase 10).

## Done when

Icons render with the correct colour in both appearances, the probe is shared by both button helpers with identical telemetry output, and `surface()` is gone.

## Risks and rollback

A probe refactor that shifts coordinates breaks native qualification silently: keep the exact window-coordinate calculation and compare telemetry JSON before and after on the same window. Rollback: revert; the old probes return.
