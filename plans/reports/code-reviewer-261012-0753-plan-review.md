# Plan review: Cap-style studio UI

Scope: uncommitted diff under desktop/ and docs/desktop/ vs plans/261011-0033-cap-style-studio-ui. No code edited; only plan.md changed.

## Checks run (toolchain 1.98.1, python shim on PATH)
- `cargo +1.98.1 fmt --check`: clean. CI clippy command (-D warnings, --all-targets): clean. `cargo test -p fframes-studio --lib`: 239 pass.
- Note: running cargo from the repo root picks 1.99.0 (the pin is desktop/rust-toolchain.toml, CI sets 1.98.1 globally); use `+1.98.1` locally.

## 1. Acceptance criteria and telemetry/ids
- Ids: every kebab-case string literal in HEAD's studio_shell, conversation_panel, timeline_view, canvas_view, preview_element, preset_panel, setup_view, text_input still exists, except `"center"` (not an id). Every label literal survives except `"Reasoning · writing…"` (documented deviation) and format strings (`preset-{}` is now `preset-{key}`, same value; "more error" text moved).
- All probe ids (build-preview ... close, timeline-fit, agent-policy/apply/prompt) present; probes gated on `qualifying` as before. Transport buttons are rendered unconditionally.
- AC2 holds: rgb(/rgba(/colors:: only in app.rs, agent_spike.rs (both unmodified); other hits are `new_straight_rgba`.
- AC1, 6, 7 (live theme switch, screenshots, Cap comparison) and native x11/M2 runs: not verifiable here, left pending.
- AC3 deviations: all now listed in plan.md "Deliberate differences". Additions to the `buttons` telemetry map (agent-tab-*, agent-undo) are documented.

## 2. Findings (not fixed)
- Medium, desktop/app/src/ui/menu.rs:332 with studio_shell.rs:3948: clicking the overflow trigger while the menu is open fires mouse_down_out (dismiss) and then the trigger click re-opens it, so the trigger cannot toggle the menu closed. Fix: ignore outside-down when it lands inside the anchor bounds, or toggle in `open_overflow`.
- Low, studio_shell.rs:3965: overflow enabled flags are a snapshot at open; if a build starts while the menu is open, "Refresh/Checkpoint" stay enabled. `dispatch` is expected to guard; worth confirming.
- Low, setup_view.rs:132: `fail()` called while already `Failed` would attribute to step 0 (and from `PrerequisitesNeeded` to step 1). No such call path exists today (only entry is from Checking/Installing). `failed_step` is not cleared on retry but is read only in `Failed`, so harmless.
- Low: text_input.rs fixed field height (32/28) with a 1 px border: unverified on screen.
- Geometry reviewed by reading and consistent: timeline `pointer_frame`, `axis_inset`, `hit_zone` vs ruler_bounds, scroll-wheel anchor all use the same inset; menu step/skip logic and Home/End correct.

## 3. Licensing
No Cap source, names or identifiers in src/ui, design_system, assets, gallery (only one "Cap has no left column" comment in metrics.rs:66 and the doc's attribution section). lucide-LICENSE.txt present with ISC and the Feather (Cole Bemis) MIT text; ships via the notices directory copy.

## 4. Phase/plan references
New and changed code, the gallery, tests and docs/desktop/design-system.md contain none. The remaining `Phase N` hits are pre-existing and belong to older milestone plans, outside this diff: desktop/app/tests/provider_handoff.rs:1, desktop/app/src/conversation_panel/qualification.rs:398,425 (local pass labels), docs/desktop/implementation-plan.md, docs/desktop/README.md:50,62,118. Left alone.

## 5. plan.md
Added "Deliberate differences" (design system, screens, telemetry, worker_client) and set status to "implemented; pending on-machine checks".

## Unresolved
- x11_shell.rs compiles only on unix and was never run; Linux target not cross-checked.
- test-packaging.py `test_help_documents_every_argument` fails on this host only (no python3).

Status: DONE_WITH_CONCERNS
Summary: Gates are green and ids/telemetry match HEAD apart from documented deviations; plan.md now has the deviations and an "implemented, pending on-machine checks" status.
Concerns/Blockers: Overflow-menu toggle bug (menu.rs:332) unfixed; rendering, theme switch, x11 and M2 native checks remain unverified.
