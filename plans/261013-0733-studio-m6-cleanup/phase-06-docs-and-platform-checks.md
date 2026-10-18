---
phase: 6
title: "Docs and platform checks"
status: completed
priority: P2
effort: "1.5d"
dependencies: [1]
---

# Phase 6: Docs and platform checks

**2026-10-18 acceptance:** the owner confirmed all Windows app hand checks passed ("test all okay by my hand"). [Platform checks](./reports/platform-checks.md) record the tested build and owner-reported results. Docs and Windows validation are complete; the Windows preview dry run in step 6 passed on 2026-10-18 ([39110213274](https://github.com/khanhthanhdev/fframes-desktop/actions/runs/39110213274)).

**Release-path decision 2026-10-18:** the owner selected Windows unsigned preview preparation. Step 6 is now a Windows preview dry run with `preview_release=true`, `dry_run=true`, `windows_only=true`, `consumer_release=false`; consumer publication remains disabled. The signing and provenance requirements for a future consumer release are unchanged.

## Context

Docs were last rewritten before M3 to M5a. Checked at `df53e56` (2026-10-17). `AGENTS.md` at the repo root has no desktop section and no `/desktop` entry in its Project Structure list; its timeline-document text from M4 is already present, so nothing from M4 is outstanding there. macOS is not fully tested: CI only, no hand checks (decided 2026-10-16, [roadmap, Platforms](../261013-0733-studio-rewrite-roadmap/plan.md#confirmed-contract)). Windows hand checks go to `reports/platform-checks.md`.

## Requirements

- Each doc below describes the system as built; points to the owning source, script or test instead of copying it.
- Every link and path in the touched docs resolves.
- `reports/platform-checks.md` records each roadmap Done-means item on Windows, plus the open hand checks below.

## Files

| File | What is wrong now | Source of truth to point to |
|------|-------------------|-----------------------------|
| `docs/desktop/architecture.md` | Line 3 says "proposed architecture"; workspace block (lines 72-93) lists non-existent `studio-protocol`, `studio-agent`, `studio-media`, `studio-ui` as proposed and omits `studio-engine`, `studio-harness`, `studio-proto`, `studio-theme`, `studio-markdown`, `studio-bootstrap`, `studio-sdk`, `studio-test-support`; line 66 "integration spike"; line 173 calls native drivers optional (Claude and Codex are native, ACP is the rest); the sections "Implemented native workflow (M3 Stage 4)", "Implemented M6 provider profiles" and "Implemented M4 presets" describe the old ACP-only panel and Linux-only evidence limits | `desktop/Cargo.toml` members; `studio-engine/src/engine/`; `studio-engine/src/services/`; `.github/workflows/desktop-*.yml` |
| `docs/desktop/README.md` | Lines 20-54 describe the pre-rewrite panel (Agent tab beside Project, Assets and Style; ACP only; single-line prompt; Chat and Project tabs; one `conversation.jsonl`; Claude, Codex, Pi, Antigravity profiles). Lines 58-60 show `spike-ui`. Line 72 holds the X11 command and unqualified platform statements | `studio-ui/src/agent/`; `studio-engine` session store; `desktop/app/src/main.rs` command list |
| `docs/desktop/design-system.md` | Line 3 names `app.rs` and `agent_spike.rs` as exceptions (both gone after phase 1); lines 62-64 use qualification-telemetry wording | `studio-theme`, `studio-ui/src/kit/` |
| `docs/desktop/phase-zero-feasibility.md`, `implementation-plan.md` | Historical; spike wording is accurate for then. Add one line at the top of each saying it is a record, not the current design; do not rewrite | n/a |
| `docs/desktop/self-hosted-runners.md` | Check the runner and X11 notes against `desktop-self-hosted.yml` | the workflow |
| `AGENTS.md` | No `/desktop` entry | add one Project Structure line and one pointer to `docs/desktop/README.md`; nothing else |
| `plans/261011-0033-cap-style-studio-ui/plan.md` | "Facts only" licensing line is superseded | the roadmap's porting rules |

README must cover: setup, project migration, editing, agent panel, the one keymap, native Claude and Codex wires with ACP for the rest, multi-session history, the composer, diff review, and which platforms are tested. Architecture must cover: crate layout, engine command and event flow with session lifetimes, timing document and revision, playback pipeline, containment, tool broker transports, test strategy, and the kept production probes (telemetry, `bounds_probe`, `qualification.json`).

## Steps

1. Run phase 1 first so the docs do not describe deleted files.
2. Read each doc in full, then edit the smallest owning section. Delete the three "Implemented ..." sections and replace them with one current section each.
3. Add the superseded note and the `AGENTS.md` lines.
4. Windows hand checks (needs the user's RDP session; real input cannot be driven from this agent): every roadmap Done-means item; the M2 open check of the Windows title bar; real-input checks that no automated test covers since the native harness was removed (caption hit-test, title drag, Close hover colour); native file dialogs; the Ctrl+Enter chord and the review-banner layout in the real app.
5. Write `reports/platform-checks.md`: Windows rows pass or fail with date and build; macOS row lists the latest green macOS CI run and "hand checks not run, not fully tested". If a Mac appears, run `cargo test -p studio-harness --test claude_real_cli --test codex_real_cli -- --ignored` ([checks report](../../desktop/plans/reports/native-wires-checks.md)).
6. Release dry run: replaced by the Windows preview dry run (see the release-path decision above). Done: [run 39110213274](https://github.com/khanhthanhdev/fframes-desktop/actions/runs/39110213274) passed on 2026-10-18 and is recorded in the report.

## Validation

- A link check over `docs/desktop/*.md`, `AGENTS.md` and the plan files touched (every relative link and backticked path resolves).
- `grep -rniE "spike|proposed|ACP only" docs/desktop/README.md docs/desktop/architecture.md docs/desktop/design-system.md` returns nothing that is not a clearly dated history line.
- The crate list in `architecture.md` equals the `members` array in `desktop/Cargo.toml`.
- `reports/platform-checks.md` has one row per Done-means item.

## Risk

- Low: docs drift again. Mitigation: link to sources instead of copying.
- Medium: Windows checks need the user's session and may find bugs. Mitigation: a failure becomes a finding in the report and a new fix item, not a silent edit.

## Rollback

Revert the docs commit; docs have no runtime effect.
