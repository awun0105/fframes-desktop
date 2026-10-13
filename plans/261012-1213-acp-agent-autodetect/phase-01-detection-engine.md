# Phase 1: Detection engine

**Files (own):** `desktop/crates/studio-agent-spike/src/discovery.rs` (and its tests), new `desktop/crates/studio-agent-spike/src/detect.rs`, `lib.rs` export.

## Requirements

- `DetectionSpec { id, cli_executable, adapter_executable, extra_dirs }` and `detect_agents(&[DetectionSpec], &DetectionEnv) -> Vec<Detected>`, where `Detected { id, cli: Option<PathBuf>, adapter: Option<PathBuf> }`.
- Search order: managed adapters dir, `PATH`, login-shell `PATH` (Unix; on Windows the process `PATH` is enough), per-agent dirs (`~/.local/bin`, `~/.npm-global/bin`, `%APPDATA%\npm`, homebrew), Node version-manager bins (fnm, nvm, volta, pnpm, bun) following Zeron's `node_version_manager_bins_with`.
- Reuse `candidates()` and `is_executable_file`; skip relative roots and empty Windows entries.
- Existence checks only; no process is spawned. Dedupe by canonical path.
- Version comes from a separate on-demand `probe_version(path)` run through `ProcessTreeManager` with a 2 s timeout, never from `detect_agents`.

## Steps

1. Write failing tests with temp dirs: order, duplicates, relative root, `.cmd` on Windows.
2. Implement. Inject the environment through `DetectionEnv` so tests never read the real one.
3. Check Zeron's `LICENSE` and `THIRD_PARTY_NOTICES.md` before copying any code; otherwise reimplement from the design.
4. `cargo test -p studio-agent-spike` and `cargo clippy -- -D warnings`.
