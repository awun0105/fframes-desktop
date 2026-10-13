# Phase 2: `qualify-writer` command

## Context

Nothing produces authentic writer-containment evidence today. The app has the pieces:
`launch_identity` (qualification.rs), `AcpDriver` + `DriverOutcome` (termination report,
escaped pids), `ProcessTreeManager`, the provider registry. Producing the record in Rust
keeps digest and platform strings identical to what `verify_m6` checks.

Studio advertises no client `fs`/terminal capability (`ClientCapabilities::new()`), so the
adapter writes files itself or through shell subprocesses, all inside the job.

The M6 gate `interruption_and_cleanup` criteria: "verified process group termination on
user Stop or adapter crash with zero escaped descendants" (`desktop/qualification/
m6-results.json`). The record covers exactly that plus an edit; nothing more is claimed.

## Requirements

`fframes-studio qualify-writer [--profile ID] [--timeout SECS]`, next to `qualify-m3`:

1. Resolve the profile (default: selected) from the provider registry; refuse fixture
   agent names unless `--development`, which writes `fixture_only: true`.
2. Compute the launch identity. Scratch folder under the OS temp dir (not inside the
   ledger bundle), a few small files, removed at the end.
3. Scenarios, each with a fresh driver and `ProcessTreeManager`; permission requests are
   answered with an allow option (the scratch folder is the session cwd):
   - `edit`: prompt the agent to create and change a named file; the file must change.
   - `stop`: prompt a long shell command (subprocess); cancel once a tool call runs.
   - `provider_crash`: same long shell command; kill the adapter leader through its own
     `Child` handle (narrow accessor, never by pid); then verify the job empties.
   Each scenario records `descendants_observed` (from the member history),
   `escaped_descendants`, `group_empty_after`. `stop` and `provider_crash` with zero
   descendants fail ("no subprocess ran; nothing was measured").
4. Record: `schema m6-authentic/1`, `gate interruption_and_cleanup`, `evidence_kind
   authentic`, `fixture_only false`, `platform`, `adapter{agent_name, launch_identity,
   protocol_version}`, `measurements.scenarios[]`, `cleanup.owned_processes_after`,
   `limits`. Written to `<app data>/qualification/evidence/`.
5. Upsert `<app data>/qualification/m6-results.json` (kind m6, schema 1, providers keyed
   by profile id, matching the repo ledger shape): only this provider's
   `launch_identity` and `interruption_and_cleanup` gate change; temp + rename.
6. Prompts, agent text and credentials are never stored.

## Files

- `desktop/app/src/qualify_writer.rs` (new), `desktop/app/src/main.rs` (routing),
  narrow leader accessor in `studio-agent-spike` driver.
- `docs/desktop/README.md` (how to qualify, limits).

## Validation

- Unit: write a bundle, then `resolve_with` qualifies it; refused for an escape, non-empty
  group, zero descendants, fixture name, another platform, a tampered file.
- Development run on Windows against `tests/support/acp-agent.py` (bundled Python on
  PATH) with `--development`: `resolve_with` refuses it; scenarios show descendants and
  an empty group.

## Risk / rollback

App-local ledger; delete `<app data>/qualification/` to revert.
