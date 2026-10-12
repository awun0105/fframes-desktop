# Phase 1: Design system foundation

Status: implemented (pending on-machine appearance check). Depends on: none. Effort: 2-3 days.

## Goal

Replace the compile-time palette in `desktop/app/src/design_system.rs` with a runtime design system that follows the OS light or dark appearance and carries Cap's editor tokens: colours, metrics, typography, shadows and motion. Every later phase reads from it and nothing else.

Target values: [Cap design spec](../reports/xia-261011-0016-cap-design-system-spec.md), section 1 (colours, shadows, typography) and section 2 (sizes). Values are exact Cap values (user decision); do not copy Cap's Rust code.

## Current state (inventory)

`design_system.rs` today (`:10-294`):
- `ThemePreset { Graphite, Pearl }`, `DensityPreset { Comfortable, Compact }`, `DesignSystemConfig`, `CONFIG` (Graphite, Comfortable).
- `Palette`: 22 `u32` fields (`window`, `sidebar`, `surface`, `surface_raised`, `surface_hover`, `canvas`, `border`, `border_subtle`, `text`, `muted`, `accent`, `accent_hover`, `accent_tint`, `selection_overlay`, `on_accent`, `success`, `success_surface`, `warning`, `warning_surface`, `danger`, `danger_surface`, `user_bubble`).
- `Metrics`: `space_1/2/3/4/6`, `radius_sm/md/lg`, `control_pad_x/y`, `sidebar_width`, `agent_panel_width`.
- `const fn tokens()`, `const fn tokens_for(ThemePreset)`, `const fn metrics_for(DensityPreset)`.
- `pub(crate) mod colors`: `BACKGROUND, PANEL, BORDER, TEXT, MUTED, ACCENT, WARNING, WARNING_TEXT, SUCCESS, SUCCESS_TEXT, DANGER, DANGER_TEXT, BUBBLE, CARD, CANVAS` as `u32`.
- `ButtonStyle { Secondary, Primary, Selected, Destructive }`, `fn button(...)`, `fn surface(...)`.
- Two tests (`:276`, `:288`).

Consumers (all through `colors::*`, `tokens()`, `button()` or `surface()`; no `rgb(0x..)` literals outside this file): `studio_shell.rs`, `text_input.rs`, `timeline_view.rs`, `conversation_panel.rs`, `preset_panel.rs`, `setup_view.rs`. `app.rs` and `agent_spike.rs` use their own literals and are untouched.

## Target module layout

Split `design_system.rs` into a folder, because it grows from about 300 to about 700 lines across clearly separate concerns:

```
desktop/app/src/design_system/
  mod.rs          appearance state, tokens(), re-exports, compatibility shims
  palette.rs      Palette struct, LIGHT and DARK constants, track colours, segment mix
  metrics.rs      Metrics struct (spacing, radii, sizes), one constant set
  typography.rs   TextStyle enum and helpers (size, weight, line height)
  elevation.rs    Shadow enum and BoxShadow builders
  motion.rs       durations and easing constants, reduced-motion flag (stub)
  contrast.rs     relative luminance, compositing and ratio, used by tests
```

## API to implement

```rust
// mod.rs
pub enum Appearance { Light, Dark }
pub fn appearance() -> Appearance;                 // reads a process-wide AtomicU8
pub fn set_appearance(appearance: Appearance);      // only main.rs calls this
pub fn appearance_from_window(a: gpui::WindowAppearance) -> Appearance;
pub fn tokens() -> &'static DesignTokens;           // current appearance
pub fn tokens_for(a: Appearance) -> &'static DesignTokens;
pub struct DesignTokens { pub palette: Palette, pub metrics: Metrics }

// palette.rs (all colour roles are gpui::Rgba so alpha survives)
pub struct Palette {
    pub window: Rgba, pub card: Rgba, pub card_2: Rgba, pub stage: Rgba,
    pub line: Rgba, pub line_strong: Rgba,
    pub text_1: Rgba, pub text_2: Rgba, pub text_3: Rgba,
    pub ctl: Rgba, pub ctl_hover: Rgba, pub ctl_active: Rgba,
    pub accent: Rgba, pub accent_2: Rgba, pub on_accent: Rgba,
    pub playhead: Rgba, pub thumb: Rgba,
    pub success: Rgba, pub success_surface: Rgba,
    pub warning: Rgba, pub warning_surface: Rgba,
    pub danger: Rgba, pub danger_surface: Rgba,
    pub selection: Rgba,            // accent at 25%, for canvas selection and text selection
    pub tooltip_bg: Rgba,           // Radix grey 12: light #202020, dark #eeeeee
    pub tooltip_text: Rgba,         // Radix grey 1: light #fcfcfc, dark #111111
}
pub enum TrackKind { Clip, Overlay, Audio }
pub fn track_color(kind: TrackKind) -> Rgba;       // #3b82f6, #8b5cf6, #22c55e
pub struct SegmentColors { pub fill: Rgba, pub border: Rgba, pub label: Rgba }
pub fn segment_colors(kind: TrackKind, selected: bool) -> SegmentColors; // spec mix rule

// metrics.rs
pub struct Metrics {
    // spacing scale (kept)
    pub space_1: f32, pub space_2: f32, pub space_3: f32, pub space_4: f32, pub space_6: f32,
    // radii
    pub radius_control: f32 /*7*/, pub radius_item: f32 /*6*/, pub radius_segment: f32 /*8*/,
    pub radius_card: f32 /*12*/, pub radius_popover: f32 /*12*/, pub radius_tab: f32 /*9*/,
    pub radius_frame: f32 /*6*/,
    // control sizes
    pub control_h: f32 /*28*/, pub select_h: f32 /*30*/, pub input_h: f32 /*32*/,
    pub field_row_h: f32 /*34*/, pub icon_sm: f32 /*14*/, pub icon_md: f32 /*16*/,
    // shell sizes
    pub header_h: f32 /*52*/, pub tab_bar_h: f32 /*46*/, pub toolbar_h: f32 /*44*/,
    pub transport_h: f32 /*48*/, pub gutter: f32 /*8*/, pub left_column_w: f32 /*248*/,
    pub agent_panel_w: f32 /*400*/,
    pub timeline_default_h: f32 /*260*/, pub timeline_min_h: f32 /*240*/, pub player_min_h: f32 /*320*/,
    // timeline geometry
    pub ruler_h: f32 /*26*/, pub track_h: f32 /*44*/, pub track_gap: f32 /*6*/, pub track_gutter_w: f32 /*104*/,
}

// typography.rs
pub enum TextStyle { Caption /*10 medium*/, Small /*11*/, Label /*12 medium*/, Body /*13*/,
                     BodyStrong /*13 semibold*/, Title /*14 medium*/, Heading /*16 semibold*/ }
pub fn text_style<E: Styled>(element: E, style: TextStyle) -> E; // size, weight, line height
pub const UI_FONT: &str = ".SystemUIFont";

// elevation.rs
pub enum Shadow { Card, Pop, Preview, RaisedPill, Thumb }
pub fn shadow(kind: Shadow) -> SmallVec<[BoxShadow; 2]>; // per appearance, spec table

// motion.rs
pub const FAST_MS: u64 = 120; pub const NORMAL_MS: u64 = 200; // collapsible 0.2 s ease-out
```

`Heading` (16) is not in Cap's widget sizes; it is the one size fframes adds, for the empty-state title. `left_column_w` 248 (today's width) is an fframes value; Cap has no left column.

## Steps

1. Create the folder and move existing code into `mod.rs` unchanged; build passes. Commit point.
2. Add `palette.rs` with `LIGHT` and `DARK` constants holding every spec value as `Rgba` (use a `const fn hex(u32) -> Rgba` and `hexa(u32) -> Rgba` helper so values read like the spec). Add status colours: danger from Radix red 9 and 11 with red 3 surface, warning from amber 11 with amber 3 surface, success from Radix green 9 and 11 with green 3 surface (MIT; Cap has no green). `on_accent` white.
3. Add `metrics.rs`, `typography.rs`, `elevation.rs`, `motion.rs`, `contrast.rs`.
4. Appearance state: an `AtomicU8` in `mod.rs`. `tokens()` loads it and returns `&LIGHT_TOKENS` or `&DARK_TOKENS`.
5. Migrate consumers in one pass, file by file, with this field map:

   | Old | New |
   | --- | --- |
   | `window`, `colors::BACKGROUND` | `window` |
   | `sidebar`, `surface`, `colors::CARD` | `card` |
   | `surface_raised`, `colors::PANEL`, `user_bubble`, `colors::BUBBLE` | `card_2` |
   | `surface_hover` | `ctl_hover` |
   | `canvas`, `colors::CANVAS` | `stage` |
   | `border`, `colors::BORDER` | `line_strong` |
   | `border_subtle` | `line` |
   | `text`, `colors::TEXT` | `text_1` |
   | `muted`, `colors::MUTED` | `text_2` |
   | `accent`, `colors::ACCENT` | `accent` |
   | `accent_hover` | `accent_2` |
   | `accent_tint` | `ctl_hover` (selected fill, as Cap's tab rail) |
   | `selection_overlay` | `selection` |
   | `success*`, `warning*`, `danger*` and their `colors::` names | same roles |

   Call sites change from `.bg(rgb(colors::PANEL))` to `.bg(tokens().palette.card_2)`. Remove the `colors` module at the end of the migration, so no `u32` colour path remains; do not keep shims that hide alpha loss.
6. Metrics renames: `radius_sm` to `radius_item`, `radius_md` to `radius_control`, `radius_lg` to `radius_card`, `sidebar_width` to `left_column_w`, `agent_panel_width` to `agent_panel_w`. Drop `control_pad_x/y` once Phase 3 buttons own their padding; until then keep them.
7. Remove `ThemePreset`, `DesignSystemConfig`, `CONFIG`, `DensityPreset` and `metrics_for` (density is unused by any screen; confirm with grep before removing; keep it if anything reads it).
8. `main.rs` (`run_studio`, around `:148-161`): before `open_window`, call `set_appearance(appearance_from_window(cx.window_appearance()))`. Inside the window's root view creation, `cx.observe_window_appearance(window, |shell, window, cx| { set_appearance(appearance_from_window(window.appearance())); cx.notify(); })`.
9. Keep `ButtonStyle`, `button()` and `surface()` compiling with the new tokens; Phase 3 reimplements them.

## Tests (in `design_system/` modules)

| Test | Asserts |
| --- | --- |
| `light_palette_matches_cap_spec`, `dark_palette_matches_cap_spec` | every role equals the spec value, including alpha |
| `translucent_roles_keep_alpha` | `line`, `line_strong`, `ctl*` have alpha below 1 |
| `body_text_contrast` | composited ratio of `text_1` on `window`, `card`, `card_2` and `text_2` on `card`, `card_2` is at least 4.5 in both appearances |
| `accepted_contrast_shortfalls_are_recorded` | white on `accent` is 4.02 light and 3.65 dark, light `text_2` on `window` 4.49, light `text_3` on `card` 2.60 (each within 0.01): fails if a value changes silently |
| `segment_colors_follow_mix_rule` | fill, border and label for each track kind and selected state |
| `metrics_match_cap_spec` | the size constants above |
| `appearance_from_window_maps_all_variants` | Light, VibrantLight, Dark, VibrantDark |

Tests call `tokens_for` and never `set_appearance`, because tests run in parallel.

## Validation

- `cargo test --locked --manifest-path desktop/Cargo.toml -p fframes-studio design_system`
- `cargo clippy --locked --manifest-path desktop/Cargo.toml -p fframes-studio --all-targets -- -D warnings`
- `rg "rgb\(" desktop/app/src --glob '!app.rs' --glob '!agent_spike.rs' --glob '!design_system/**'` returns nothing.
- On the user's machine: switch the OS theme while the app is open; the window repaints in the other appearance.

## Done when

All consumers use the new roles, the `colors` module is gone, the tests above pass, and the app shows a correct light and dark appearance (layout unchanged at this point).

## Risks and rollback

Mass rename across six files: done mechanically with the field map; the compiler finds misses. Risk of the first frame using the wrong appearance on Windows: step 8 reads the appearance before opening the window. Rollback: revert the phase commits; nothing outside the app crate changes.
