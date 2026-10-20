---
phase: 3
title: "Onboarding view, Welcome, Appearance and First video"
status: pending
effort: "1-1.5 days"
depends_on: [1]
---

# Phase 3: Onboarding view, Welcome, Appearance and First video

## Objective

Have a full-window stepper that replaces the workspace while onboarding is open, with the three
steps that need no new engine work.

## Layout

```diagram
┌──────────────────────────────────────────────────────────────────────────┐
│ header (unchanged: logo, caption buttons)                                │
├──────────────┬───────────────────────────────────────────────────────────┤
│ ● Welcome    │  Appearance                                               │
│ ● Appearance │  Choose how Studio looks. You can change it in Settings.  │
│ ○ SDK        │                                                           │
│ ○ Agent      │  [ System ][ Light ][ Dark ]   (segmented, live preview)  │
│ ○ Permissions│                                                           │
│ ○ First video│                                                           │
│              │                              [Skip setup]  [Back] [Next] │
└──────────────┴───────────────────────────────────────────────────────────┘
```

- The step list on the left uses the existing `SettingsFrame` + `SettingsNavItem` (same component
  as Settings), with a status dot per step (done, skipped, current). Steps are clickable once visited.
- The content card uses existing kit pieces (`Card`, `SegmentedControl`, `Button`, `Notice`). No
  new tokens, so it matches `docs/desktop/design-system.md`.

## Changes

- `desktop/crates/studio-ui/src/onboarding/mod.rs` (create): `OnboardingView` entity with
  `step: OnboardingStep`, `visited`, `skipped` and render. It emits `OnboardingEvent::{Finished, Skipped}`.
- `desktop/crates/studio-ui/src/shell/mod.rs` / `layout.rs` (modify): when `needs_onboarding`, render
  `OnboardingView` in place of the workspace body (rail, editor and agent column). On `Finished` or
  `Skipped`, save `OnboardingState` and show the workspace.
- Welcome: one sentence on what Studio does, and **Get started**. **Skip setup** is available on every step.
- Appearance: reuse the theme choice list and the save path from `shell/settings.rs` (lines ~19-24 and
  ~102-236). Extract the "apply + save" call into a function both use, instead of duplicating it.
- First video: **Create video…**, **Open folder…** and **Recent** (if any), dispatching the existing
  shell commands. Finishing from here opens the project.
- `desktop/crates/studio-ui/src/shell/settings.rs`: add **Run setup again**, which reopens the view
  at Welcome without clearing choices.
- Keyboard: Enter = Next, Escape = Back, Tab order step list → content → buttons. Add the
  actions to `actions.rs` and bind them in `keymap.rs`, matching the existing action pattern.
- Telemetry (`shell/observation.rs`): expose `onboarding: { open, step }` so parity tests can drive it.

## Tests (keep)

- Parity `desktop/crates/studio-ui/tests/parity/onboarding.rs` (create):
  - A fresh harness without an SDK or agent opens onboarding. Skip writes `onboarding.completed`,
    and the next window opens the workspace.
  - Picking Dark on the Appearance step changes `theme` in telemetry and in `app-settings.json`.
  - The harness with SDK and agent preconfigured never shows onboarding.
- The existing parity tests must keep starting in the workspace. `Studio::build` writes a completed
  `OnboardingState`, or the migration rule covers it. Verify this, don't assume it.

## Done when

- `cargo test -p studio-ui` passes. Captures of Welcome and Appearance in light and dark are
  inspected (see the verification notes in phase 5).
