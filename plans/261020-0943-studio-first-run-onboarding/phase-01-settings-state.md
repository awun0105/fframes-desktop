---
phase: 1
title: "Onboarding state and permission policy in app settings"
status: pending
effort: "0.5 day"
depends_on: []
---

# Phase 1: Onboarding state and permission policy in app settings

## Objective

Give the app one place that says whether onboarding is finished and which permission policy
the user chose, with a migration rule that leaves existing installs alone.

## Changes

- `desktop/crates/studio-proto/src/lib.rs` (modify): add `PermissionPolicy { AskEveryTime, AllowInWorkingCopy, AllowEverything }`
  next to `AppTheme`, serde `snake_case`, `Default = AskEveryTime`. It lives here so the
  engine and UI share it, like `AppTheme`.
- `desktop/crates/studio-engine/src/app_settings.rs` (modify): extend `AppSettings`:
  ```rust
  pub onboarding: Option<OnboardingState>, // None = never recorded (old file or first launch)
  pub permission_policy: PermissionPolicy, // #[serde(default)]
  pub struct OnboardingState { pub completed: bool, pub skipped_steps: Vec<OnboardingStep> }
  ```
  Use the existing lenient pattern: an unreadable value falls back to the default without
  dropping `theme` or `export`.
- Same file: `pub fn needs_onboarding(settings: &AppSettings, sdk_ready: bool, agent_selected: bool) -> bool`.
  It returns `true` only when `onboarding` is `None` and either the SDK or the agent is missing.
  When it returns `false` and `onboarding` is `None`, the caller writes
  `OnboardingState { completed: true, .. }` once, so the decision is made a single time.
- `desktop/crates/studio-ui/src/shell/mod.rs` (modify): at startup, after app storage loads
  (the point where `busy` turns false), compute `needs_onboarding` from the managed SDK status
  and `provider-registry.json`'s `selected_profile_id`.

## Tests (keep)

- `app_settings.rs`: the old file `{"theme":"dark","export":{..}}` loads with `onboarding: None`,
  `AskEveryTime`, and the theme kept. An unknown policy string loads as the default and keeps
  the other fields.
- `needs_onboarding`: the four combinations of SDK and agent with `None`, plus
  `Some(completed: false)` (user quit mid-flow, show again) and `Some(completed: true)`.

## Done when

- `cargo test -p studio-engine app_settings` passes. No UI change is visible yet.
