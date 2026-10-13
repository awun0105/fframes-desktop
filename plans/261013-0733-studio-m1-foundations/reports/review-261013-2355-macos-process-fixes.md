# Review: macOS process-group probe fix and test portability

Scope: commits `aae73f1`, `bc0ed10`, `8f9694e` and `e53af94` on `feat/studio-rewrite`. The gpui pin in `7a815ba` is excluded. This was a read-only review on a Windows host, so nothing here was executed on macOS. Line numbers are for `desktop/crates/studio-bootstrap/src/process.rs` unless another file is named.

## Verdict

I found no blocking defect. The probe change is sound under the kernel behaviour that the code and the empirical note rely on. One conditional risk remains (finding M1). It depends on libproc behaviour that I could not verify from this host, and it needs a single check on a Mac. The test gates are justified, and none of them hides a product bug.

## Findings

### M1 (Medium, conditional): an invisible live member could now be reported as Empty

The `Exists` branch already trusted the libproc scan before this change. The only new exposure is the case where `kill(-pgid, 0)` fails while a live member still exists. On macOS that means the group has no member that we may signal: it holds only zombies, or only live processes owned by another uid, or only protected processes. Signal 0 can fail only with ESRCH or EPERM, so in practice `Probe::Unknown` means EPERM (lines 546-554).

Once the probe is inconclusive, the scan can end in one of three ways:

- If `proc_pidinfo` refuses the other-uid pid with `read == 0` and an errno other than ESRCH, the scan is marked incomplete and the result is `Present` (lines 662-668, 744). This is safe.
- If `proc_pidinfo` succeeds, the live entry is not a zombie, so the result is `Members` (lines 714-719, 746). This is safe.
- The unsafe case is when `proc_listpgrppids` silently leaves the pid out, or `proc_pidinfo` answers ESRCH for a pid that is alive but not visible to us. Line 666 counts that pid as exited. Two quiet scans then return `Empty` (lines 747-751). `await_group_empty` reaps the leader (line 1712), and `report` or `latch_report` latches a verified termination through `set_terminal`.

Before this change, the same situation returned `Present` and ended in `GroupSurvivors`, which is the honest answer.

Failure scenario: an agent runs `sudo some-helper`. The helper keeps running as root in the task group, and the shell that started it exits. `kill(-pgid, 0)` returns EPERM. If libproc hides or ESRCH-masks the root process from an unprivileged caller, cancellation reports the tree as verified empty while the root helper is still running.

I do not have primary evidence for which libproc behaviour applies. As I recall, XNU enforces a same-user check on `PROC_PIDTBSDINFO` with EPERM (which would be the safe first case) and does not filter `proc_listpids` by uid. That recollection is not verified. The pre-change behaviour was safe either way.

The risk is latent today. Preset mutation is qualified only on Linux x86_64 (`desktop/crates/studio-engine/src/preset_state.rs:43`). Writer qualification needs ledger evidence for the current platform (`desktop/app/src/conversation_panel/qualification.rs:255-269`).

Fix: before macOS mutation or macOS qualification opens, run the existing probe script on a Mac with a live `sudo sleep 60` in a group whose leader has exited. Confirm that `proc_listpgrppids` lists the pid and that `proc_pidinfo(..., 1, ...)` either succeeds or fails with EPERM. Then add a unit case to `macos_membership_ignores_zombies_but_requires_two_complete_scans` that feeds `Probe::Unknown` together with a live, non-zombie entry and expects `Members` (see L2). If the check shows ESRCH masking, make an EPERM probe require the leader to be a zombie in the scan before `Quiet` can count.

### L1 (Low): the macOS ACP tests now assert nothing about ownership

`desktop/crates/studio-agent-spike/tests/acp_v1.rs:1094-1105` and `desktop/crates/studio-agent-spike/tests/acp_v1_review.rs:362-372` gate the demotion on Linux. On macOS they have no other branch. The test names still say "demotes ownership". If macOS later gains an escape scan, or starts returning wrong pids, nothing would notice.

Fix: add a `#[cfg(not(target_os = "linux"))]` branch that asserts `outcome.escaped_pids.is_empty()` and that the ownership is unchanged. Comment it with something like "No escape scan on this platform: the escape is invisible and ownership stays as claimed." The test would then pin the documented gap instead of passing silently.

### L2 (Low): the new unit test covers only the zombie path

The new unit test at lines 2932-2954 checks two cases: `Unknown` with only zombies gives `Empty`, and `Unknown` with an incomplete scan gives `Present`. It has no case where `Unknown` meets a live, non-zombie entry and must give `Members`. That case is the safety property in M1.

### L3 (Low): a doc comment now contradicts the code

The comment at lines 604-606 says that `killpg(..., 0)` "continues to report zombie-only groups as present". The fixed comment at lines 735-737 says that it fails with EPERM. Suggested wording: "and `kill(-pgid, 0)` fails with EPERM while a group holds only unreaped zombies, so termination verification needs process state as well as the probe."

### Informational

- `setsid_escaped_helper_survives_group_verification_and_is_observable_only_while_parented` now needs `python3` on Linux as well, where it used `setsid` before. If the interpreter is missing, the test fails deterministically: the `helper pid line` parse at line 2457 panics on EOF. It does not hang or flake. The ACP fixture `acp-peer.py` already needs Python.
- `current_target_triple` (`desktop/crates/studio-sdk/src/install.rs:776-791`) maps every other host, x86_64 macOS included, to `aarch64-apple-darwin`. This predates the reviewed commits. The receipt test now matches the two sibling tests at lines 1091 and 1122.

## Answers to the four questions

### 1. Can the probe change report Empty while a live member remains?

Only in the M1 case. The other cases hold up:

- **Foreign verdict** (lines 705-713, 745): the leader's pid stays reserved, because the leader is not reaped until membership has already returned `Empty` (line 1712; spawn-time comment at lines 1832-1835). BSD pid allocation also skips ids that are still in use as a process group. A leader entry whose start time does not match therefore implies that the original group is gone, so `Empty` is correct.
- **`leader_start`**: it is always `Some` on macOS (lines 1829-1845, 1520), so the check is always active.
- **Other errno values**: signal 0 never yields EINVAL, so nothing besides EPERM is routed to the scan.
- **Exiting processes**: a process that is exiting but not yet `SZOMB` reports a running state, so it counts as a member. This errs toward `Present`.
- **Pid recycled between list and info**: a reused pid appears with a different `pbi_pgid` and is filtered out by the group check at line 717. That is correct, because the original member has exited.

### 2. Does any other macOS caller still treat Unknown as present in a way that stalls?

No. `group_membership` is the only place that calls `probe_group` on macOS (lines 1514-1525).

- Line 774 is the Linux path and is unchanged. On Linux, signal 0 to a zombie succeeds, so groups that hold only zombies take the `Exists` branch.
- Line 1528 applies only to Unix systems other than Linux and macOS.
- `signal_group` ignores the result of `kill` (lines 1677-1679). An EPERM on SIGTERM or SIGKILL therefore cannot stall anything. It is bounded by `GROUP_VERIFY_TIMEOUT` (2 s, line 420) in `await_group_empty`, and the report that follows records the outcome.
- `Drop`, `is_alive`, `terminate_verified`, `kill_forcefully` and `observe` (lines 1429, 1456, 1464, 1711, 1753, 1777, 2166) all go through `group_membership`.

### 3. Are the Linux-only gates justified, and is the escape-scan gap correctly described as by design?

Yes on both counts.

- `escaped_descendants` documents that "Other platforms: escapes are not scanned, so always `Clean`" (lines 1560-1561). It returns `Clean` with a comment saying that the scan only demotes (lines 1605-1610). The phase file records this as a known gap that must be decided before M3. It is a real capability gap, not a hidden bug. The production path to `ProcessGroupContained` on macOS goes through platform ledger evidence (`qualification.rs:255-269`).
- The APFS skip (`desktop/crates/studio-project/tests/revision.rs:208-223`) is justified, because APFS refuses names that are not valid UTF-8 (EILSEQ). The product code is not gated, and Linux keeps the coverage.
- The host-triple change is justified.

### 4. Are any races or flaky spots left in the changed tests?

I found none.

- **Escape test** (lines 2545-2547): Python calls `setsid()` before `print`. When the pid is read, the helper has therefore already left the group. `sh -c` runs without job control, so the background job is not a group leader and `setsid` cannot fail with EPERM. `execvp` keeps the pid.
- **TERM-trap test** (lines 2811-2812): `echo $$` runs after `trap`, so TERM cannot arrive before the trap is installed.
- **`ps` fallback** (lines 2474-2481, `acp_v1.rs:143-150`): for a pid that has gone, `ps` prints nothing, which maps to `None` and therefore "not alive". The window between `kill(pid, 0)` and `ps` resolves to "not alive", which is the correct answer when the process is exiting. `/bin/ps` always exists on macOS and shows zombies as `Z`.
- **Pid reuse**: there is a theoretical reuse window in the `!pid_alive(helper)` checks right after SIGKILL. It predates these commits and is negligible.

Status: DONE_WITH_CONCERNS
Summary: The EPERM-to-scan change fixes the macOS zombie-group stall, and every test gate matches a real platform gap. No blocking defect survived verification.
Concerns/Blockers: M1 has not been verified on a Mac. If libproc hides or ESRCH-masks a live member owned by another uid, an EPERM probe can now become a false verified Empty, where the old code returned Present. This needs one macOS check before macOS mutation or qualification opens.
