---
phase: 4
title: "Runtime writer containment"
status: in-progress
priority: P1
effort: "2d (re-estimated after the scout: the Job Object launch, the escape scans and the capture gate already existed)"
dependencies: []
---

# Phase 4: Runtime writer containment

## Goal

The engine itself proves that an agent's writer processes are contained, on Linux, macOS and Windows, so candidate capture no longer depends on a qualification ledger file. Starts after M2 phases 1 and 8 (containment moved to the engine; `process.rs` split).

## Context

- Gate: `studio-engine/src/agent_task.rs:316` refuses with `UnqualifiedOwnership` unless `ownership.is_qualified()`; only `ProcessGroupContained` qualifies (`studio-bootstrap/src/process.rs:399-410` on 2026-10-13).
- Today's ledger path: `resolve_ownership` and `launch_identity` (moved to `studio-engine::containment` in M2 phase 1), reading `<data>/qualification/m3-results.json` and `m6-results.json`, which only `containment::qualify_writer` (the `qualify-writer` subcommand) writes for real adapters. M2 phase 7 kept both so real agents can still apply on Linux until this phase.
- The `qualify-m3 --test-writer-containment` ownership injection (`desktop/app/src/main.rs:171` on 2026-10-13) is deleted in M2 phase 7. Tests inject ownership only through `OwnershipPolicy::TestInjected`.

## Tasks

1. Scout `process.rs` (Linux and macOS process-group code after the M2 phase 8 split), `containment`, and every reader of `WriterOwnership`.
2. Windows containment mode: the Job Object launch already exists in `studio-bootstrap/src/process.rs` (child created `CREATE_SUSPENDED | CREATE_NO_WINDOW`, assigned to a job with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE`, then resumed, around `:1866-1967`; accounting query around `:1271`; `IsProcessInJob` around `:1086`), matching Zeron's launch (`process/windows/mod.rs:127-152`). Verify that breakaway stays refused (the job sets only `KILL_ON_JOB_CLOSE | DIE_ON_UNHANDLED_EXCEPTION` around `:1882`, no breakaway flag) and add what is missing: a `JobObjectContained` ownership, and quiescence as "the job has no active processes" (`JobObjectBasicAccountingInformation`). Re-estimate the phase after the scout; it is likely a day shorter than planned.
3. Replace the ledger with a runtime check at launch: the engine verifies containment for this adapter launch (process group created and owned on Unix, Job Object assigned before the process runs on Windows) and records `ProcessGroupContained` or a new `JobObjectContained` only if the check passes; otherwise `Unknown`, which blocks capture as today.
4. Keep `launch_identity` (it ties capture to the exact launched adapter) without the ledger fields.
5. Remove the ledger reading code, `containment::qualify_writer`, the `qualify-writer` subcommand, the provider registry's `QualificationSnapshot` (`host.rs` `load_qualification_snapshot`) and the `qualification/` data dependency, in one commit with the runtime check from task 3. `QualificationSnapshot` is display-only (provider ranking, recommendation and notice in the setup and provider UI): replace it with the runtime containment result of each provider's last launch, drop the ledger-based ranking, and update the moved `provider_profiles.rs` tests and the parity setup test where they assert recommendations.
6. Tests on all three CI platforms, through the production policy (`OwnershipPolicy::Validated`, never `TestInjected`): the fake ACP agent launched with no ledger file reaches a contained ownership and its candidate is captured (and applies on Linux; Windows and macOS apply opens in phase 3); a writer that spawns a grandchild is still contained and is stopped before capture; a launch where containment setup fails yields `Unknown` and capture is refused.

## Reference patterns (Cap, Zeron)

From [the Zeron report](../reports/research-261013-1749-zeron-agent-control.md) and [the Cap report](../reports/research-261013-1801-cap-gpui-desktop-ui.md); see the roadmap's pattern map.

1. Verify at the right moment: `IsProcessInJob` is not called between `AssignProcessToJobObject` (around `:1914`) and `ResumeThread` (around `:1967`) today (checked); add the check there; if the check fails, terminate the suspended child and record `Unknown`.
2. Stop on Windows: `TerminateJobObject` skips the agent's own shutdown (Zeron `lib.rs:601-605`). Keep fframes' order: ACP `session/cancel`, drain for `shutdown_grace` (2 s, `driver/runtime.rs:106`), then `terminate_verified`; Zeron uses the same 2 s grace. Test that the cancel notification is written before the job is terminated.
3. Adapter environment: remove `CLAUDECODE*` variables from the adapter's environment (Zeron `process/owned.rs:6-17`). fframes has no such filter today (checked), so a Claude-based adapter started from a Claude Code terminal believes it is nested.
4. Windows adapter files: accept only `.exe`, `.cmd`, `.bat` and `.com`, never `.ps1` (Zeron `executable.rs:49-79, 405-468`). Keep `detect.rs`, which resolves npm `.cmd` shims to their package script instead of running `cmd.exe`.

## Todo

- [x] Verification before resume, cancel-before-terminate test, `CLAUDECODE*` filter, extension check

- [x] Scout
- [x] Windows Job Object mode
- [x] Runtime verification replaces the ledger
- [x] Launch identity kept
- [x] Ledger code removed
- [ ] Containment tests on three platforms (green on this Windows VM; Linux and macOS CI need a push)

## Progress (2026-10-15)

- **Windows verification before resume.** `spawn_tracked` checks `IsProcessInJob` after `AssignProcessToJobObject` while the adapter is still suspended; a failed or unanswerable check terminates it and fails the launch (`JobAssignmentFailed`), so there is no writer. Breakaway stays refused (only test builds set `BREAKAWAY_OK`). Quiescence already required an empty job (accounting query in `terminate_verified`).
- **Runtime ownership.** `TrackedChild::containment()` reports what the spawn verified: `JobObjectContained` on Windows, `ProcessGroupContained` on Linux and macOS (own group, pinned leader, escape scan; macOS got a whole-table libproc scan), `Unknown` elsewhere. **Addition:** on Linux a leader whose start time cannot be read is not pinned, so that launch is `Unknown`. `DriverConfig.writer_ownership` became `ownership: OwnershipSource` (`Verified`, or `Injected` for `OwnershipPolicy::TestInjected`); `AcpDriver::start` takes the ownership from the spawned child and `AcpDriver::ownership()` exposes it; the workflow passes that to `agent_writer_started`. Observations still only demote.
- **Removed:** `containment::qualify_writer`, the `qualify-writer` subcommand, the ledger verifier (`resolve_ownership`, `resolve_with`, `verify_m3`/`verify_m6`, `LEDGER_*`, `Platform`), `AdapterSettings::ownership_probe`, `Resolution`, `QualificationSnapshot` and its ranking. The success grep is empty.
- **Launch identity** now hashes the program the launch actually runs (`AdapterLaunch`: the resolved executable, for an npm shim the interpreter with its script as an argument), so the resume digest matches what was started. **Behaviour change:** a saved session of an npm-shim adapter refuses to resume once after this change (its digest now names `node` and the script); native executables keep their digest.
- **Display. Deviation:** the last-launch result is kept in the workflow snapshot (`AdapterView::last_launch`, reset when the adapter changes) and, per provider label, in the agent panel's memory, instead of in the provider host: the workflow is where the launch happens, and the host keeps no state across launches. The provider list shows "Contained at last launch (...)", "Not contained at last launch (...)" or "Not launched yet"; the ranking and recommendation are gone.
- **Behaviour change:** a provider switch no longer refuses a target as "not qualified" up front; its containment is verified when it launches, like any task.
- **Adapter environment:** `CLAUDECODE` and `CLAUDECODE_*` (any case) are never forwarded, even when listed as sign-in names. **Correction to the plan's premise:** `ChildEnvironment::apply_to_command` already calls `env_clear()` and the allowlist never carried them, so an inherited `CLAUDECODE` never reached adapters; the filter guards the sign-in-name route, the only one left. Windows adapter files were already limited to `.exe`, `.cmd`, `.bat` and `.com`.
- **Fault injection:** `studio-bootstrap` feature `fault-injection` (enabled only from `fframes-studio`'s dev-dependencies) makes the next N containment checks fail through each platform's real path: Windows and macOS refuse the launch, Linux records `Unknown`.
- The fault is consumed by the next spawn in the controller's scopes; in these tests that is the adapter, because tool workers and MCP servers are started by the agent inside its own scope and builds run after the turn.
- Tests: `app/tests/agent_writer_containment.rs` (not platform-gated) runs the scripted agent under `OwnershipPolicy::Validated` with no ledger. The writer is `job-object-contained` on this VM, its helper grandchild is gone before capture, and the candidate is captured and applied. A failed check gives no captured candidate. Stop delivers `session/cancel` to the running agent before its processes are terminated. Bootstrap, harness and engine unit tests cover the fault path, the env filter and the launch identity.
- Opening a project no longer hashes the adapter or reads a ledger, so the agent panel attaches sooner. Two parity tests then sent Open while the shell was still preparing the first project (Open is ignored while busy) and now wait for an idle shell first.
- **Residual risk:** work a pre-existing service does for the adapter (COM, Task Scheduler and WMI on Windows; `systemd-run` and D-Bus activation on Linux; `launchd` on macOS) runs outside the job or group and is not observed.
- **Known failure, owned by phase 3:** `app/tests/agent_workflow_apply_gate.rs` still expects Apply to be blocked off Linux. Since Windows publication landed, the candidate is applied on this VM, so the test fails here until phase 3 rewrites it.

## Success criteria

On this VM the fake agent reaches `JobObjectContained` and its candidate is captured (apply on this VM is checked in phase 3); the grandchild and failed-setup tests pass on Windows, macOS and Linux CI; `grep -rn "m3-results.json\|m6-results.json\|qualify_writer\|QualificationSnapshot" desktop --include=*.rs` is empty.
