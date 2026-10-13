---
phase: 7
title: "Remove the bounds-probe harness"
status: pending
priority: P1
effort: "2-3d"
dependencies: [4]
---

# Phase 7: Remove the bounds-probe harness

## Goal

Delete the bounds-probe qualification harness, its evidence, scripts and CI jobs now that parity tests cover the behavior, and replace the release workflow's qualification gate with an explicit one. Production safety checks stay.

## Keep (production code, not harness)

- `studio-engine::containment` (moved in phase 1), including ledger verification and `containment::qualify_writer` with the `qualify-writer` subcommand in `app/src/main.rs`. Until M3 phase 4 replaces the ledger with runtime checks, they are the only way a real adapter becomes `ProcessGroupContained`; deleting them here would block real agent apply on Linux. M3 phase 4 deletes them.
- Capability probes: `probe_no_clobber` and `probe_mechanism` (`studio-project/src/publish.rs`), `probe_apply_gate` (`edit_transaction.rs`), `probe_adapter` (`studio-harness` discovery), `probe_candidate` (`studio-bootstrap/src/process.rs`), `probe_bundled_compiler` (`studio-sdk/src/doctor.rs`), `CandidateProbe` (`candidate_validation.rs`).

## Delete (verify each by scouting)

- Code: the `qualify-m2`, `qualify-m3` (with its `--test-writer-containment` ownership injection at `main.rs:171`) and other qualification subcommands except `qualify-writer`; the qualification and telemetry parts of the shell, `timeline_view`, the conversation panel UI and `conversation_panel/qualification.rs` (minus what phase 1 moved); `presentation_stress.rs`, `stress_metrics.rs`, `evidence_preview.rs` (minus `decode_png`, moved in phase 1), the `ui/probe` component, `desktop/app/examples/probe_m6_providers.rs`.
- Tests: `m3_development_evidence.rs`, `m5_resource_qualification.rs`.
- Data: `desktop/qualification/` once the release gate below no longer reads it. The runtime ledger lives in `<data>/qualification/` on each machine, not in the repository, so this does not affect writer qualification.
- Scripts: `desktop/scripts/qualify-*.py`, `test-qualification-m4.py` to `m7.py`, `validate-qualification.py`. `package-phase-zero.py` reads `desktop/qualification/m0-results.json`; drop that input in the same commit.
- CI: qualification and M2 stress jobs in `.github/workflows/desktop.yml` and `desktop-self-hosted.yml`.

## Tasks

1. Scout every reference to the delete list across `desktop/`, `.github/`, `justfile` and `docs/`.
2. Confirm each parity item has a gpui test; add any missing one before deleting.
3. Release gate replacement in `.github/workflows/desktop-release.yml` (today at around `:187-188`, `:397-411`, `:453`): an `eligible_targets` list in the release policy file, a package verifier kept as a non-qualification script (`desktop/scripts/verify-package.py`, moved out of `test-qualification-m7.py`), and a check that the tag commit equals the commit of a green `full` run of `desktop-self-hosted.yml` and a green `desktop.yml` run on `main` (a fast-only push run does not count). Add tests that an ineligible target and a tag with only a fast run are refused.
4. Delete the list above.
5. Note for the user: the self-hosted Windows runner no longer needs a visible RDP session.

## Todo

- [ ] Scout references
- [ ] Parity coverage check
- [ ] Release gate replacement with refusal test
- [ ] Harness code, tests, data, scripts and CI jobs deleted

## Success criteria

`grep -rnE "qualify_|qualification::|presentation_stress|stress_metrics|bounds_probe|start_agent_telemetry" desktop --include=*.rs` matches only `studio-engine/src/containment/` and the `qualify-writer` subcommand (both deleted in M3 phase 4); the kept probes still exist; the containment ledger tests and `qualify_writer` tests pass; all CI workflows and a release dry run with `consumer_release_enabled` set pass.
