# Centralize Studio settings in the settings window

Status: done (2026-10-21)

## Outcome

Every app, agent, SDK, preset and export setting is edited in the floating settings
window. The editor keeps only editing and the per-message agent mode/model pickers.

## Decisions (user, 2026-10-21)

- Pages: **General / Agent / SDK / Presets / Export**. Per-project parts say to open a
  project when none is open.
- Presets: the whole preset panel moves (library, apply/reset, overrides).
- Also move: review policy, export defaults, audio output.
- Not moved: agent mode/model pickers under the composer.

## Pages

| Page (id) | Icon | Content | Comes from |
| --- | --- | --- | --- |
| General (`settings-general`) | Settings | Appearance (theme), Audio output (status + Next / Retry / No audio), Setup (Run setup again), Updates | `settings.rs`, the player toolbar buttons (`player.rs:803-829`) |
| Agent (`settings-agent`) | Bot | Provider setup, Review policy (`agent-policy`), Permissions | `setup_page.rs`, the review section of `agent/project_tab.rs` |
| SDK (`settings-sdk`) | Package | Managed SDK (`sdk_controls`) | the Project tab's `sdk-section` (`left_column.rs:220`) |
| Presets (`settings-presets`) | Palette | The project's style files and `PresetPanel` | the Style tab (`left_column.rs:285`) |
| Export (`settings-export`) | Download | Default format, size and frame rate, saved in `app-settings.json` | `export/panel.rs` rows; `choose_export_settings` |

## What stays in the editor

- Style tab: the style file list, plus "Style presets…" that opens Settings → Presets.
- Agent Changes tab: the review policy as read-only text that points to Settings → Agent.
- Export panel: its pickers choose for this export only. They start from the defaults
  and are no longer saved.
- Player toolbar: the audio status text.
- Project tab: files and recents.
- Onboarding: its steps still reuse these sections.

## Acceptance

- Each moved control works from the settings window (UI tests drive them there).
- `just clippy`, `cargo fmt`, studio-ui unit and UI tests and fframes-studio tests pass.
- Docs (`docs/desktop/design-system.md`, `docs/desktop/README.md`) say where each
  setting now lives.

## Risks

- Per-project pages (SDK select, presets, review policy) need an open project.
- The export page has no worker answer, so it offers every known format, both sizes and
  "the video's own" or 30 fps. `usable_settings` now falls back one choice at a time, so
  a frame rate a video cannot take no longer resets the format.
- UI tests that click these controls in the main window must move to the settings
  window: `window.rs:277-287`, `presets.rs`, `setup.rs`, and `services.rs` for export.
