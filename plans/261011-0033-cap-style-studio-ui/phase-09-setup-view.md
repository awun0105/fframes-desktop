# Phase 9: Setup view

Status: done (see reports/fullstack-developer-261012-0602-phase-09.md). Depends on: Phase 3. Effort: 1 day.

## Goal

Restyle the guided setup screen (`desktop/app/src/setup_view.rs`, `SetupView`, `render` at `:457`) as a centred step card in Cap's style. No change to the setup flow.

## Prerequisite

`setup_view.rs` had uncommitted changes from the release workstream when this plan was written. Make sure they are committed or stashed before starting.

## Current state

- States `SetupState` (`:38`): Checking, PrerequisitesNeeded, SdkInstallNeeded, Installing, SdkReady, CreatingProject, Building, Rendered, Failed. (`CreatingProject` and `Building` render, `:611-621`, but nothing in this file assigns them; out of scope, noted only.)
- Controls: `btn-recheck` "Re-run Preflight Check" (`:512`), `btn-install-sdk` "Install Managed SDK & Build Template" (`:566`), `btn-create-proj` "Build and Render Template Project" (`:603`), `btn-retry` "Retry" (`:675`, calls `run_preflight`).
- Status texts at `:468-672`; colour sites: root `:689`, `:692`; canvas `:503`; accent `:508`, `:551`; warning `:492`; success `:587`, `:635`; danger `:665`.

## Target

- Window bg `window`; one centred `card` (max width 560, padding 24) with `Heading` "Set up fframes Studio" and a short `Body` `text_2` line.
- A vertical step list: Preflight, Install SDK, Build template, Render. Each step shows a `StatusDot` or icon (pending `text_3`, active `accent` with a spinner-free "In progress" `Badge`, done CircleCheck `success`, failed AlertTriangle `danger`) and its status text in Body `text_2`.
- The current state's primary action as a `Button` `Primary` under the steps (`btn-install-sdk`, `btn-create-proj`), secondary actions `Editor` (`btn-recheck`, `btn-retry`); same ids and labels.
- Failure text in a `Notice` Danger with details collapsible.

## Steps

1. Map each `SetupState` to step statuses in a pure function `step_statuses(state) -> [StepStatus; 4]`.
2. Render the card and steps with components; replace colour sites with tokens.

## Tests

- New: `step_statuses_cover_every_state`.
- `cargo test -p fframes-studio` (setup is exercised indirectly by SDK tests; ignored ones need an SDK bundle).

## Validation

On the user's machine, run through setup on a user profile without an installed SDK: each state renders legibly in both appearances. States that are hard to reach by hand (Failed, Installing) are covered by rendering `SetupView` with that state in the gallery example.

## Done when

The setup screen uses only tokens and components, the flow is unchanged, and the four button ids keep working.

## Rollback

Revert the file.
