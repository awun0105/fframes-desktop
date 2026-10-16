# Native wires: checks with the real CLIs

Checks of the M5a drivers against the real, signed-in CLIs. Plan: [M5a native wires](../../../plans/261013-0733-studio-m5a-native-wires/plan.md).

## Claude Code (2026-10-16)

- **Host:** this Windows Server 2022 VM. Claude Code 2.1.296 (`C:\Users\Administrator\.local\bin\claude.exe`), model `haiku`.
- **Command:** `cargo test -p studio-harness --test claude_real_cli -- --ignored --nocapture`, with Claude Code on `PATH`. The test is in [`crates/studio-harness/tests/claude_real_cli.rs`](../../crates/studio-harness/tests/claude_real_cli.rs).
- **Setup:** the driver runs in a temporary directory containing `hello.txt` with the text `hello`. No MCP servers are passed.

| Check | Result |
|-------|--------|
| The prompt asks for an edit; the user denies every permission request | 1 `can_use_tool` request. The turn ended `EndTurn`, and `hello.txt` is unchanged. |
| The same prompt; the user allows the request | 1 request. The turn ended `EndTurn`, eligible for capture, and `hello.txt` contains `goodbye`. |
| Stop during a long text answer (interrupt after the first text) | The turn ended `Cancelled`. The owned process count is unchanged, so the CLI kept running. |
| The next prompt in the same session | The turn ended `EndTurn` and is eligible for capture. |
| Session stop | No failure, and the process group is empty. |

The whole run took 16 s. Only stop reasons and request counts were recorded; account data was not.

A second run added three checks. Each test pumps the driver until the agent is idle, which means no turn runs, no background task runs and Claude is not about to continue by itself. That is the point at which the engine may start capture.

| Check | Result |
|-------|--------|
| A steer sent at the first text of a long list, asking for a code word instead | `steer` was accepted (`priority: "now"`). The turn ended once, `EndTurn`, after 35 characters of text that contain the code word. |
| The session is stopped and a new driver resumes it with `--resume` | The session reports itself restored with the same id. Asked for the earlier code word, it answers `KESTREL-4417`. |
| A Bash command run with `run_in_background` (`sleep 20 && echo done > bg.txt`), allowed by the user | 1 request. Background work was reported before the prompted turn ended. Claude then ran a turn of its own (`TurnStarted`, `Autonomous`), and both turns ended `EndTurn`. When the agent was idle, `bg.txt` existed. |
| `AskUserQuestion` with two questions in one call, each answered with its last option | One `Question` event carrying both items. Both answers went back through `answer_question`, and Claude replied `Green, Mango`. This proves the driver. The engine's card still answers only the first question; M5 phase 2 owns that fix. |

The first three ran in 57 s together with the first test; the question check took 6 s. The engine's handling of that frame sequence (capture deferred until idle, Stop during the wait) is covered with the fake CLI, which replays the same frames (`claude_wire` cases in `app/tests/agent_workflow`).

## Codex (2026-10-16)

- **Host:** this VM. Codex 0.162.1 (`C:\Users\Administrator\AppData\Local\Programs\OpenAI\Codex\bin\codex.exe`), signed in with ChatGPT, the account's default model, `--effort low`.
- **Command:** `cargo test -p studio-harness --test codex_real_cli -- --ignored --nocapture`, with `FFRAMES_CODEX` set to that path. The test is in [`crates/studio-harness/tests/codex_real_cli.rs`](../../crates/studio-harness/tests/codex_real_cli.rs).
- **Setup:** the driver runs in an empty temporary directory, and the user's Codex home and login are kept. The studio MCP server is a Python stand-in that records its start. The user config has the MCP server `rea` and the built-in apps.

| Check | Result |
|-------|--------|
| The prompt asks to create `ran.txt`; the user declines every request | 1 approval under `untrusted`: a file change in one run, a PowerShell write in the other. The turn ended `EndTurn`, and `ran.txt` does not exist. |
| The same prompt; the user accepts | 1 or 2 approvals. The turn ended `EndTurn`, eligible for capture, and `ran.txt` was written inside the draft with `excludeTmpdirEnvVar` and `excludeSlashTmp` set. |
| Which servers ran | The stand-in started. Below the test process ran only `codex.exe`, `codex-code-mode-host.exe`, `conhost.exe` and the stand-in's `python.exe`. This is weak evidence on its own, since `rea` might not be a long-lived local process. The firmer evidence is the earlier raw probe: with the same `thread/start` config, only `studio` reported `mcpServer/startupStatus/updated` (see the phase 3 file). |
| Stop during a long text answer | The turn ended `Cancelled` (`turn/completed` status `interrupted`). The owned process count is unchanged, so the app server stayed up. |
| The next prompt in the same thread | `EndTurn`, eligible for capture. |
| An accepted `ping -n 120 127.0.0.1`, stopped while it runs | The turn ended `Cancelled`, and `ping` kept running after the interrupt, as S5 found. The driver reported `Background { tasks: 1 }` before the turn's end, so capture waits for it. |
| Session stop with that command still running | No failure, and the process group is empty. None of the processes seen during the session is still running, and the number of `PING.EXE` processes is back to its value before the turn. |

The last run took 37 s and ran the driver with the review and interrupt-id changes described below. Only stop reasons, approval titles and executable names were recorded. Command lines and account data were not.

Probes of the raw app server, using scripts in the session scratchpad that print method names and statuses only, settled two shapes the driver depends on:
- `thread/compact/start` replies `{}` and then runs as a turn (`turn/started`, a `contextCompaction` item, `turn/completed`).
- `review/start` replies with one turn id, and its `turn/started` announces another. `turn/interrupt` with the announced id stops it, and `turn/completed` (status `interrupted` or `completed`) carries the reply's id.
- `review/start` with `uncommittedChanges` in a directory that is not a git repository is not refused. Its read-only commands ran without approval requests, as `untrusted` allows for commands it knows to be safe. A refused slash command still ends only its own turn, as incomplete (fake CLI test).

A second run added steering and resume:

| Check | Result |
|-------|--------|
| A `turn/steer` sent at the first text of a long list, asking for a code word instead | Accepted. The turn ended once, `EndTurn`, after 51 characters of text that contain the code word. |
| The thread is stopped and a new driver resumes it with `thread/resume` | The thread reports itself restored with the same id. Asked for the earlier code word, it answers `KESTREL-4417`. |

Both Codex tests passed in 67 s.

Not covered against the real CLI:
- a command that completes after its turn (only the stopped case above);
- `requestUserInput` questions.

These two are covered with the fake CLI (`codex_app_server.rs`, and the `codex_wire` workflow cases in `app/tests/agent_workflow`).

## macOS

Not fully tested. The real-CLI checks were not run because no Mac is available (decided by the user, 2026-10-16). CI runs the fake-CLI suites on macOS 15. If a Mac becomes available, run the same two commands there.
