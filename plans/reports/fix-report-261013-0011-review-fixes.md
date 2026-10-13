# Fix report: uncommitted-work review findings

Plan: `plans/261012-2310-review-fixes/plan.md`. Review: `code-review-261012-2303-uncommitted-parallel-review.md`.
Nothing is committed. Snapshots in the session scratchpad: `snapshot-2310/` (before) and `snapshot-fixed/` (after).

## Result

Every confirmed finding is fixed at its cause. On this Windows host, `cargo fmt --all --check`,
`cargo clippy --workspace --all-targets --locked -D warnings` and `cargo test --workspace --locked`
pass in `desktop/`. The example passes clippy and fmt. The Python suites pass:
`test-qualification-m6.py` and `test-packaging.py`, which ran 60 tests and skipped 6.
`validate-qualification.py` accepts the evidence that is committed.

## What changed, by layer

- **studio-bootstrap.**
  - The escape scan returns `EscapeScan { Clean, Escaped, Unverifiable }` and fails closed on
    Windows and Linux.
  - Windows: it handles ERROR_MORE_DATA and records a gap when it reaches the member cap. A
    background sampler records job members. The parent pid is re-read from the opened handle,
    so a reused pid is not counted as an escape. The undefined behaviour in the id read is gone,
    and `DIE_ON_UNHANDLED_EXCEPTION` is set.
  - The child environment now passes the proxy and CA variables, and proxy values are redacted.
  - On macOS, no scan exists, so the result is `Clean`, as before (the user chose this).
- **studio-agent-spike driver.**
  - A prompt, images included, is checked against the wire message limit before it is sent.
  - The server-tool case is recognised by its structured `errorKind`, for the Claude adapter
    only, and the turn is reported as `reported_incomplete`.
  - Only real activity resets the idle deadline, and a turn is capped at `max_turn` (2 h).
  - The redaction state is reset between turns.
  - `containment_unverified` is reported on the outcome and demotes the writer to Detached.
- **Detection and install.**
  - A broken final folder is repaired. The rename is retried on a briefly locked file, and the
    stale sweep is safe.
  - A cancel after npm has exited is honoured.
  - The npm output and the end of its log are kept in errors.
  - Both shim templates are understood, and a shim with an unknown entry falls back to running
    with Node first on its PATH.
  - Found paths are not canonicalized, and `probe_version` is deleted.
- **studio-test-support (new, dev-only).** One test interpreter lookup, now used by every
  Rust test.
- **App panel.**
  - Detection, install and turn-on moved to `conversation_panel/provider_setup.rs`.
  - Turn-on is one enum.
  - Built-ins are detected by their fixed adapter name, and a profile is saved by its id.
  - Install has a ticket and is cancelled and reset when the project closes, under the project's
    process scope.
  - Detect and Install exclude each other, and the config menu commits by value.
- **App core.**
  - The whole CLI uses clap. `studio` still accepts launcher-forwarded arguments.
  - `qualify-writer`:
    - the baseline is taken before the prompt, and permission-reply errors are recorded;
    - the shutdown outcome is kept on every path;
    - names are unique per run and the ledger is locked while it is updated;
    - one overall deadline applies (default 1800 s);
    - an `--out` inside the qualification folder is refused.
  - `check_record` and the Python validator both require `contained`.
  - Adapter checks carry a generation number. New chat clears the saved session before
    archiving, and the image budget comes from the driver.
  - `reported_incomplete` is handled, and the log archive never overwrites an existing file.
  - The engine demotes on unverified containment.
  - The failed step is stored in `SetupState::Failed`, development bundles are read in debug
    builds only, and the overflow-menu cache is removed.
- **Example.** One shared `render_cli.rs` helper is used by the three binaries, with a
  CPU fallback.
  - Setup errors are reported as JSON through `serde_json`, which was already in the lock.
  - The portrait HUD is aligned with the landscape cut, and the rendered mp4 files are ignored.
- **Docs.** The README covers turn limits, the prompt size limit, the `qualify-writer` flags and
  timeout, the verifier's `contained` check, the Node PATH order, development bundles, New chat
  conditions and the registry write.

## Not verified on this host

- These Linux-only test targets ran 0 tests here: `m3_development_evidence`, `provider_handoff`,
  `agent_workflow_ui`, `scoped_editing`, `preset_workflow`, `promotion_handoff` and most of
  `agent_workflow`.
- The cross-check against `x86_64-unknown-linux-gnu` cannot build, because `wayland-backend`
  needs a Linux C compiler.
- `x11_shell.rs` and `m3_development_evidence.rs` were edited (the shared test-Python helper)
  but were never compiled. `provider_handoff.rs` and `agent_workflow_ui.rs` were type-checked
  once with their gate widened.
- The macOS code paths were not compiled.
- Real adapters were not exercised. The fallback after `new_vulkan` fails has never run.

## Known limits

- On Windows the member history is per task and capped at 4096. A long task that runs very
  large builds through tool calls can reach the cap; after that, its containment is reported
  as unverifiable.
- npm is stopped on quit only if the shell takes the controller lock within 50 ms. Otherwise
  the OS job cleanup stops it. Closing the project always stops it.

## Decisions taken after the review (2026-10-13)

- The 2-hour turn cap stays.
- `qualify-writer` keeps one overall limit, 1800 s by default.
- The portrait copy now matches the landscape cut.
- Adapters install with `npm ci` from a committed lockfile per adapter
  (`studio-agent-spike/adapters/<profile>/`). An install from another lock is replaced.
- The config menu, New chat and the SDK manifest rework stay on this branch.
