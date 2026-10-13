---
status: in-progress
branch: feat/cap-style-studio-ui
created: 2026-10-12
---

# Real-adapter capture and Windows Apply

## Outcome

A real ACP adapter (first: Claude's ACP adapter) can finish an agent task on a real machine:
its candidate is captured and validated, and on Windows it can be applied to the project
source, without weakening the safety model.

## Decision (user, 2026-10-12)

Keep "writer containment is not a setting". Earn it with measured, authentic evidence
(route A). No opt-in trust switch. First adapter to qualify: Claude ACP adapter.

## Constraints

- Ownership stays `Unknown` unless the installed ledger holds authentic evidence for this
  exact launch identity and platform (`app/src/conversation_panel/qualification.rs`).
- Evidence is never fabricated: an unmeasured property is recorded as not measured and
  the gate does not pass.
- The real run uses the user's own sign-in in a throwaway scratch project, only when the
  user starts it. Credentials are never written; only variable names.
- Apply never overwrites a name it did not verify (no-clobber publication), same contract
  as Linux.
- Host notes: bundled Python must be on PATH for tests; RDP window visible for GUI runs.

## Non-goals

- macOS (no host). Authentic gates other than writer containment (repair, IME, audio).
- An opt-in "trust this adapter" setting.

## Phases

| # | Phase | Status | Unblocks |
|---|-------|--------|----------|
| 1 | [Windows escape scan](phase-01-windows-escape-scan.md) | done | honest `escaped_descendants` on Windows |
| 2 | [`qualify-writer` command](phase-02-qualify-writer-command.md) | done (dev run vs scripted peer passes on Windows) | capture + validation + review/export with a real adapter |
| 3 | [Real Claude adapter run](phase-03-real-adapter-run.md) | pending (needs user) | installed M6 evidence on this host |
| 4 | [Windows publish backend](phase-04-windows-publish-backend.md) | pending | Apply / Undo on Windows |

Dependencies: 2 needs 1; 3 needs 2 and the user; 4 is independent of 1-3.

## Acceptance criteria

1. On Windows, `escaped_descendants()` reports a real scan (descendants outside the job),
   and its unit test spawns a process outside the job and sees it.
2. `fframes-studio qualify-writer --profile <id>` runs edit, stop and provider_crash
   scenarios against the selected adapter in a scratch folder, writes an `m6-authentic/1`
   record and upserts `<app data>/qualification/m6-results.json` without touching other
   providers. A scenario that escaped or left the group non-empty makes the gate `fail`.
3. After a passing run, Setup shows "qualified for this exact adapter and platform" and a
   real task reaches review (or Accepted where Apply works) instead of "capture blocked".
4. On Windows, `probe_apply_gate` returns Ready on NTFS, and the existing transaction tests
   (Apply, rollback, Undo, conflict retention) pass on Windows.
5. `just clippy`, `cargo fmt --all` and the touched crates' tests pass on Windows.

## Key evidence

- Gate check: `qualification.rs` `verify_m6` / `check_record` (scenarios edit, stop,
  provider_crash; `escaped_descendants == 0`, `group_empty_after == true`,
  `cleanup.owned_processes_after == 0`, platform `Windows`/`AMD64`).
- Windows containment: `studio-bootstrap/src/process.rs` job object
  (`KILL_ON_JOB_CLOSE`, suspended start, no breakaway); `escaped_descendants` is empty
  off Linux today.
- Apply: `studio-engine/src/edit_transaction.rs:715` blocks non-Linux; the transaction
  machine (`edit_transaction/linux.rs`) has no OS calls; all OS work is in
  `studio-project/src/publish.rs`.
