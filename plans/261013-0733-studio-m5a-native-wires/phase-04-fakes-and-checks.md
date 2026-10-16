---
phase: 4
title: "Fake CLIs, cross-driver tests and platform checks"
status: in-progress
priority: P1
effort: "2-3d"
dependencies: [2, 3]
---

# Phase 4: Fake CLIs, cross-driver tests and platform checks

> **Spike S5 (2026-10-14) contradicts parts of this phase.** Read the [native-wires report, Result section](../261013-0733-studio-m1-foundations/reports/spike-native-wires.md#result) before building: neither CLI's turn-end frame means the work is done (Claude runs unprompted turns with their own `result`; Codex completes items after `turn/completed`), Codex has no `turn/aborted` and its interrupt leaves the command running, only `untrusted` asks before every command, and session stop needs interrupt, stdin close, a grace period and a containment kill.

## Goal

All three drivers are tested the same way on Windows, macOS and Linux CI without real CLIs, and checked by hand with real ones.

## Tasks

1. Fake CLIs in `desktop/crates/studio-harness/tests/support/`: `fake-claude.py` (stream-json and control protocol) and `fake-codex.py` (app-server JSON-RPC), next to the existing `acp-agent.py`. Port the scenarios of Zeron's `tests/claude.rs` and `tests/codex.rs` (its fake is a `.sh`, which does not run on Windows); replay frames recorded from the real CLIs in S5.
2. One cross-driver suite, run once per wire: streaming text, permission asked and denied (tool not run), permission allowed, unanswered permission blocks the tool, question answered, steer at the declared boundary, stop, resume, images, plan updates, turn end exactly once.
3. The case Zeron left ACP for: the agent starts a background command and ends its reply. Assert that the turn ends on the protocol's end frame and that candidate capture waits until the writer process tree is quiet.
4. Parity agent test runs with each wire.
5. Manual checks: Claude on this VM (Windows) and the user's Mac; Codex where installed. Record versions and results in `reports/native-wires-checks.md`.

## Todo

- [x] Python fake CLIs with recorded frames: `fake-claude.py` and `fake-codex.py`, with frame shapes from Claude Code 2.1.296 and codex-cli 0.162.1. They sit in `crates/studio-harness/tests/` next to `acp-peer.py` (the ACP fake, not `acp-agent.py`), not in `tests/support/`. Each also has its own suite: `claude_stream_json.rs` and `codex_app_server.rs`.
- [x] Cross-driver suite, `crates/studio-harness/tests/cross_driver.rs`. It drives every scenario through `start_driver` and the `AgentDriver` trait, once per wire, with a fixture that maps each scenario to the fake's mode and prompt. Scenarios:
  - text, and exactly one end per turn;
  - a permission denied (the tool does not run) and allowed;
  - an unanswered permission blocks the tool until Stop closes it;
  - questions;
  - steering at the declared boundary, refused on ACP;
  - stop keeps the agent running;
  - resume;
  - images;
  - plans;
  - background work.

  What a wire cannot do is asserted through its capabilities: ACP has no steering, questions or background work. ACP plans are not checked, because the ACP driver drops plan updates until the M5 todo panel adds them (M5 phase 3). The ACP peer's `permission` mode now writes only when the allow option is chosen, so the deny case means something there too.
- [x] Background-work turn-end test. In the cross-driver suite, the work is reported before the wire's own end frame and clears afterwards. In the engine (`app/tests/agent_workflow/{claude,codex}_wire.rs`), capture waits for it, and Stop during the wait captures nothing.
- [x] Parity agent test per wire, `crates/studio-ui/tests/parity/agent.rs`. The panel flow (prompt, permission card, allow, review, Apply) runs over Claude stream-json and over Codex app-server through `Studio::with_native_agent`. The other parity agent tests stay on the scripted ACP agent.
- [ ] Manual checks recorded ([report](../../desktop/plans/reports/native-wires-checks.md)). Done on this Windows VM for Claude Code 2.1.296 and Codex 0.162.1. Still open: Claude and Codex on the user's Mac.
- [x] Cross-driver suite on macOS and Linux CI: green on all three platforms for `6225af8` ([self-hosted Linux and Windows](https://github.com/khanhthanhdev/fframes-desktop/actions/runs/38905684378), [macOS 15](https://github.com/khanhthanhdev/fframes-desktop/actions/runs/38905719933)).

## Success criteria

The cross-driver suite passes for ACP, Claude and Codex on three CI platforms; the manual checks report shows each wire working on Windows and macOS.
