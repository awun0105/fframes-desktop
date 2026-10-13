# Phase 3: Adapter install

**Files (own):** new `desktop/crates/studio-agent-spike/src/adapter_install.rs`, the Setup Install button wiring, tests.

## Requirements

- Explicit click only. Pinned packages `@agentclientprotocol/claude-agent-acp` and `@agentclientprotocol/codex-acp`; pin exact versions after checking npm and record the date.
- Run the resolved `npm` into a `.tmp-*` sibling of `<app data>/adapters/<pkg>/<version>` with a private npm cache beside it. Rename into place only after the bin entry resolves and a marker file is written. Launch through `node <entry>`, never `npx`.
- 15 minute timeout; cancel kills the process tree; output bounded and redacted; no elevation; no inherited secrets.
- Disabled with a hint when `node` or `npm` is not found.

## Steps

1. Tests with a fake `npm` script: success, failure, cancel, concurrent installers, kill mid-install leaves nothing usable.
2. Implement and wire the button; rerun detection after success.
3. `cargo test -p studio-agent-spike` and clippy.
