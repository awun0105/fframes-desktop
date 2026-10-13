---
phase: 4
title: "Fake CLIs, cross-driver tests and platform checks"
status: pending
priority: P1
effort: "2-3d"
dependencies: [2, 3]
---

# Phase 4: Fake CLIs, cross-driver tests and platform checks

## Goal

All three drivers are tested the same way on Windows, macOS and Linux CI without real CLIs, and checked by hand with real ones.

## Tasks

1. Fake CLIs in `desktop/crates/studio-harness/tests/support/`: `fake-claude.py` (stream-json and control protocol) and `fake-codex.py` (app-server JSON-RPC), next to the existing `acp-agent.py`. Port the scenarios of Zeron's `tests/claude.rs` and `tests/codex.rs` (its fake is a `.sh`, which does not run on Windows); replay frames recorded from the real CLIs in S5.
2. One cross-driver suite, run once per wire: streaming text, permission asked and denied (tool not run), permission allowed, unanswered permission blocks the tool, question answered, steer at the declared boundary, stop, resume, images, plan updates, turn end exactly once.
3. The case Zeron left ACP for: the agent starts a background command and ends its reply. Assert that the turn ends on the protocol's end frame and that candidate capture waits until the writer process tree is quiet.
4. Parity agent test runs with each wire.
5. Manual checks: Claude on this VM (Windows) and the user's Mac; Codex where installed. Record versions and results in `reports/native-wires-checks.md`.

## Todo

- [ ] Python fake CLIs with recorded frames
- [ ] Cross-driver suite
- [ ] Background-work turn-end test
- [ ] Parity agent test per wire
- [ ] Manual checks recorded

## Success criteria

The cross-driver suite passes for ACP, Claude and Codex on three CI platforms; the manual checks report shows each wire working on Windows and macOS.
