# Phase 7: Left column content (Project, Assets, Style)

Status: done (with concerns, see report). Depends on: Phase 4. Effort: 2 days.

## Goal

Fill the left column tabs created in Phase 4 with restyled content: the project file inventory with the Managed SDK and recent projects sections, the asset list, and the preset panel (moved here from under the timeline). Content width is 216 (248 minus 16 padding each side).

## Element map

### Project tab

| Current | New |
| --- | --- |
| Inventory `inventory` `:2815-2853` with `file-{file}` buttons (reveal path), "Showing n of m files", "Create or open a project to browse its files." | `SectionTitle` "Files" with a `Badge` count; list rows h 28 (File icon 14 `text_2`, name Body 13 truncated, hover `ctl_hover`, radius 6) keeping ids `file-{file}`; "Showing n of m" as Small `text_3` footer; empty text Body `text_2` |
| Managed SDK block `:3340-3364`: status text, `install` "Install managed SDK", `import-sdk-bundle` "Import offline SDK bundle…", `sdk` "Select installed SDK…" | `Collapsible` section "Managed SDK": status line with `StatusDot` (green ready, amber missing, accent busy "Operation in progress…"), the three actions as `Button` `Editor` stacked full width, same ids, labels and enable rules |
| "Recent projects" `:3365-3420`: `recent-{id}` (name, "· Missing"), path line, `locate-{id}` "Locate…", `remove-{id}` "Remove recent", "No recent projects" | `Collapsible` section "Recent projects": rows with name (Body), path (Small `text_3`), Missing `Badge` Warning; `locate-{id}` and `remove-{id}` as small buttons shown on hover and on keyboard focus; same ids. A shared `recents_list()` function is reused by the Phase 4 empty state |

### Assets tab

| Current | New |
| --- | --- |
| Asset list (inventory with `navigation == "Assets"`), "No assets imported" | Same row style as files with the Image icon; empty text |
| `asset` "Copy asset…" `:2855-2861` | `Button` `Editor` with Plus icon at the bottom of the tab, same id and enable rule |

### Style tab (preset panel)

`desktop/app/src/preset_panel.rs` (`PresetPanel`, root id `preset-panel`, `render` at `:719`) moves from the centre column (`studio_shell.rs:3326-3328`) into this tab. Elements:

| Current (`preset_panel.rs`) | New |
| --- | --- |
| Root border and panel bg `:729-730` | No own card (the column is the card); sections separated by `divider()` |
| Warning banner `:745` | `Notice` Warning |
| Preset list `preset-{key}` `:781` (choice list) | Radio-style list rows (row h 34, check icon on the selected preset), same ids |
| `preset-apply`, `preset-reapply`, `preset-reset` `:813-839` | Row of `Button`s: Apply `Primary`, Reapply and Reset `Editor`; same ids and labels |
| `preset-import`, `preset-export`, `preset-css` `:854-876` (file pickers) | Overflow `Menu` "More" or a stacked group of `Editor` buttons; keep ids (they are not in telemetry) |
| Overrides: `clear-{token}` `:901`, `override-token` and `override-value` inputs `:928-929`, `override-set` `:930` (only when a preset is applied, `:889`) | `SectionTitle` "Overrides"; each override as a `FieldRow` (token name, value, `clear-{token}` X icon button); add form with the two inputs stacked (compact `TextInput`, h 28) and `override-set` `Button` |
| Status box `preset-status` `:948` (success or danger bg `:943-945`) | `Notice` Success or Danger, same id on the root |
| CSS report `css-report` `:961` | `Collapsible` "CSS import report", Small text |
| Keyboard routing in `:688-716` (Enter, Tab, Escape in inputs) | Unchanged |

## Steps

1. Project tab, then Assets, then Style.
2. Move the `PresetPanel` entity's render from the centre column into the Style tab; its state and commands (`PresetCommand`, `execute`, `:173`, `:405`) are untouched.
3. Replace colour sites in `preset_panel.rs` (`:729-980`) with tokens and components.
4. At 216 px content width, truncate long file names and paths with an ellipsis and a tooltip, and stack button groups vertically where a row does not fit.

## Tests

- `cargo test -p fframes-studio --test preset_workflow`, `--test real_sdk_presets` (ignored, needs an SDK), and `preset_panel.rs` unit tests (`:995`, `:1003`).
- New (pure functions): `recent_row_actions(recent) -> Vec<&str>` (Missing shows `locate-{id}`, all show `remove-{id}`), `left_tab_content(tab) -> Content` maps Style to the preset panel.

## Validation

On the user's machine: browse files and reveal one, install or select an SDK, open a recent project, locate a missing one, apply a preset, add and clear an override, import CSS and read the report, at 1280 and 1920 window widths.

## Done when

All three tabs show their content in Cap's style with ids unchanged, and the preset panel no longer renders under the timeline.

## Risks and rollback

216 px is narrow for the override form: inputs are stacked, not side by side. Rollback: revert; the preset panel returns to the centre column.
