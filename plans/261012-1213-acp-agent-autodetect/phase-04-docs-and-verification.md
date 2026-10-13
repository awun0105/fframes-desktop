# Phase 4: Docs and verification

**Files (own):** `docs/desktop/README.md` (Setup paragraph), `docs/desktop/architecture.md` if it states the PATH rule.

## Steps

1. Replace "the global `PATH` is never consulted" with: detection reads `PATH` but never launches; launching uses only the saved absolute path or the managed directory.
2. Document Detect agents and Install adapter, and that no API key is needed.
3. On this Windows host: Detect finds `claude.exe` and reports the adapter missing; Install adapter yields a launchable `claude-agent-acp`; Check adapter reaches `initialize`. Record results honestly. A real provider run needs the user's Claude login.
4. Run `just clippy`, `cargo test -p studio-agent-spike`, `cargo test -p fframes-studio`.
5. Audit the diff against this plan's constraints before committing.
