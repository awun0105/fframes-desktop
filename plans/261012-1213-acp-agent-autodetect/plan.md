---
title: "ACP agent auto-detection for fframes-studio"
description: "Detect installed ACP-capable agents on the machine (PATH, install dirs, Node version managers), list them in the agent panel Setup tab, and offer explicit installs of the Claude and Codex ACP adapters. No API key: each agent uses its own login."
status: completed
priority: P2
effort: "4-6 engineer-days"
branch: feat/cap-style-studio-ui
tags: [desktop, acp, agent, discovery]
blockedBy: []
blocks: []
created: 2026-10-12
---

# ACP agent auto-detection

Source analysis: [xia report](../reports/xia-261012-1213-zeron-acp-agent-autodetect.md) (Zeron `crates/harness` @ 344436e).

## Outcome

Open Setup, press **Detect agents**, see which agents are installed, pick one, and run Check adapter, with no manual path or API key. Claude and Codex show "CLI found, ACP adapter missing" with an explicit **Install adapter** action.

## Constraints and non-goals

- Detection is read-only: it checks file existence and never launches anything. Launching still requires the user's selection, then Check adapter or Send.
- `ExecutableSearch` for launching stays as is (absolute path or managed dir). A detected result resolves to an absolute path that the user confirms into the saved adapter file.
- Credentials are never stored or shown. `auth_env_names` stays names only and becomes optional.
- Install runs only on an explicit click, never at project open or during detect. Pinned npm versions, atomic install into `<app data>/adapters`, no `npx`.
- Non-goals: native Claude or Codex drivers, OAuth flows inside Studio, auto-update of adapters.

## Phases

| # | Phase | Status |
|---|-------|--------|
| 1 | [Detection engine](phase-01-detection-engine.md) | done |
| 2 | [Profiles and Setup UI](phase-02-profiles-and-setup-ui.md) | done |
| 3 | [Adapter install](phase-03-adapter-install.md) | done |
| 4 | [Docs and verification](phase-04-docs-and-verification.md) | done |

## Acceptance criteria

1. On a machine with `claude.exe` in `~/.local/bin` and no adapter, Setup lists Claude as "CLI found, adapter missing".
2. A fake ACP agent on `PATH` is detected, selectable, and passes Check adapter without any env var.
3. Relative `PATH` entries are ignored. Detection never spawns a process.
4. Install adapter does nothing until clicked, is atomic, and a killed install leaves no half-installed adapter.
5. `cargo clippy -- -D warnings` and the affected test packages pass.

## Risks

- Windows `.cmd` shims and npm global paths (`%APPDATA%\npm`): covered by tests with temp dirs.
- Native ACP flags for gemini, opencode, cursor, grok, devin and hermes must be checked against the installed CLI before a profile ships; unverified ones stay out.
- The documented rule "the global PATH is never consulted" becomes "never used to launch": update docs in phase 4.
- Zeron license and attribution must be checked before any code is copied.

Rollback: the work is additive; removing the Detect and Install actions restores current behavior.

## Delivery notes (2026-10-12)

- Delivered: detection (`detect.rs`), registry schema 2 with `cli_executable`, Detect/Select/Install rows in Setup, pinned installer (`adapter_install.rs`: claude-agent-acp 0.89.1, codex-acp 2.2.2, checked on npm 2026-10-12), docs. Reimplemented from the design; no Zeron code copied.
- Script adapters found on `PATH` (npm `.cmd` shims, `.js` files) are saved as `node <entry>`, because the launch `PATH` never contains `node`.
- Not done: the Unix login-shell `PATH` (it needs a shell process, which detection must not start; the common Unix install folders are searched directly). No native-ACP profiles were added: none was verified against an installed CLI.
- Host check on Windows: Detect found `claude.exe` and reported the adapter missing; Install adapter produced a launchable adapter; the probe with the app's own search (`<data>/adapters` only) reached `initialize` (sign-in not verified). Codex was not installed or launched on this host.
- `cargo test` failures in `fframes-studio` (`worker_client`, `preview_worker_client`, `agent_workflow_apply_gate`, `build_sharing`, `candidate_runner`) are from the host lacking `python` and a built worker; none touches the registry or Setup code.
