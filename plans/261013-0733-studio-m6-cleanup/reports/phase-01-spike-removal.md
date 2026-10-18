# Phase 1 report: spike removal

Branch `feat/studio-m6`, base `5af6e30`. Windows host, toolchain 1.98.1, CPU only.

## Commits
- `c025205` worker test driven through the preview transport and canvas selection (fixture emits an anchored title object)
- `c48aaa3` app spike, `--spike-ui` and unused dependencies removed
- `8449293` harness ACP session removed (`session.rs`, Acp structs, spike-only `acp-peer.py` modes; `supervisor.rs` trimmed to what the driver runtime uses; `AdapterConfig` moved)
- `5950ffc` bundle id `studio.fframes.app`
- `0505f1c` stale wording

## Verification
- `cargo fmt --all -- --check`: pass
- `cargo clippy --workspace --all-targets -- -D warnings`: pass
- `cargo test -p fframes-studio -p studio-harness -p studio-engine --no-fail-fast`: all pass, 0 failures
- `cargo test -p studio-ui --lib`: pass
- `cargo test -p studio-ui --test parity --no-fail-fast`: 77 tests pass; `services::cancelling_an_export_leaves_no_file` did not report. The parity harness has a watchdog (`harness.rs`, deadline 60 s + 30 s) that ends the test process with exit 101 when an export-cancel wait hangs, so the binary dies without a result line. Re-run alone through cargo (twice, and once through `cmd /c`): same death, exit 101. The same test executable run directly passes (105 s with cwd `desktop`, 70 s with cwd `desktop/crates/studio-ui`, which is what cargo uses). So the test passes when run directly but dies under cargo on this VM; it is the known export-cancel hang, not caused by this phase (nothing here touches the export or the harness), and it is already measured first in the flaky-tests phase.- `cargo test -p fframes-studio --test worker_roundtrip -- --ignored`: pass (fixture built in debug; CI builds release and the test falls back to it)
- `python desktop/scripts/test-packaging.py`: pass
- Spike grep over `desktop`, `.github`, `scripts` (excluding plans, target, node_modules): empty

## Notes
- The fixture `main.rs` gained an anchored title (`EditorObjectKey` with a source anchor); the legacy `--worker` path stays because the SDK embeds it as a template.
- `docs/` left for the docs phase.
- Legacy identifiers (qualify_, presentation_stress, stress_metrics, qualification::) over *.rs: no hits.
- start_agent_telemetry and ounds_probe hits are the production sites only (pp/src/main.rs, studio-ui probe.rs, shell/observation.rs, kit and agent widgets).
- desktop/app/src holds main.rs, ssets.rs, lib.rs, in/studio_mcp.rs, in/studio_tools.rs.
