# Phase 9 report: setup view

Outcome: `desktop/app/src/setup_view.rs` now renders a centred 560 px card (window bg, padding 24) with a Heading, a muted Body line, a four-step list (Preflight, Install SDK, Build template, Render), a divider and the per-state status block. Only that file changed; no commit.

- `step_statuses(&SetupState) -> [StepStatus; 4]` is pure; test `step_statuses_cover_every_state` plus `at_most_one_step_is_active` pass (2/2).
- Step markers: pending StatusDot neutral, active StatusDot info plus "In progress" Badge, done CircleCheck success, failed AlertTriangle danger.
- Buttons keep ids and labels: `btn-install-sdk` and `btn-create-proj` Primary; `btn-recheck` and `btn-retry` Editor. Failure is a Danger `Notice` (id `setup-failed`) with the error under Details and Retry as its action. All colours come from tokens; no `rgb`, no `FontWeight`.
- Behaviour unchanged: same handlers (`run_preflight`, `start_install_and_build`), same status strings.

Deviations and notes:
- `Failed` carries no stage, so all four steps stay Pending and only the notice shows the error (a mapping to a specific step would be guessed). `PrerequisitesNeeded` marks Preflight Failed.
- `CreatingProject` and `Building` both map to "Build template" Active; nothing in this file assigns them (noted only, not fixed).
- Added one intro line ("Get the managed SDK ready and render a template project.") under the heading, as the spec asks for a short Body line.

Verification: `cargo test -p fframes-studio --lib setup_view` passes and rustfmt applied. Full clippy and the whole lib suite could not be run: `conversation_panel.rs` (the other agent's in-flight file) currently fails to compile (`ButtonStyle`, `with_probe` unresolved, unused imports), so clippy reported nothing attributable to setup_view. Rendering of every state in both appearances and the gallery coverage of Failed/Installing were not done (no GPU; gallery file is not mine to edit).

Status: DONE_WITH_CONCERNS
Summary: Setup view restyled with `step_statuses` and tests passing; flow, ids and labels unchanged.
Concerns/Blockers: Full clippy and lib-suite run blocked by the concurrent conversation_panel compile errors; no on-screen verification.
