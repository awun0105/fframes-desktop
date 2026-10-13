# Zeron: how it detects, and controls, coding agents

**Target:** Zeron 0.2.112, installed at `%LOCALAPPDATA%\Programs\Zeron\zeron.exe`. It is a 166 MB native Rust app with a gpui UI.

**Method:** Zeron is MIT-licensed and public at `github.com/zeronsh/zeron`, so the analysis reads the source at tag `v0.2.112` rather than decompiling the binary.

**Provenance check:** the installed exe matches the source tag.
- Workspace `Cargo.toml:30` reads `version = "0.2.112"`. The string `0.2.112` occurs 11 times in the installed exe.
- Strings specific to this version are present in the exe: `@cursor/sdk@1.0.32`, `zeron-cursor-shim`, `perTaskStopAffordance`, `promptRequired`, `cognition.ai/subagentSupport`, `--permission-prompt-tool`, `session/prompt` and `agy_acp_server`.
- The exe has no PE version resource.

**Searches that found nothing, and why** [I]
- `grok agent stdio` is absent because the source keeps those arguments as separate array elements (`acp/mod.rs:220`).
- `validated live` exists only in a source comment.
- `EQHXZ8M8AV` sits in macOS-only `cfg` code.
- `session/request_permission`, `allow_always`, `_x.ai/session/prompt_complete` and `zeron-native` exist in the source only as `match` or `==` comparison targets, or as small `json!` values. Optimised builds commonly turn those into inline integer compares or immediate stores, so the full string never appears in the binary.
- None of these misses suggests a build mismatch.

**Runtime evidence on this host**
- `%LOCALAPPDATA%\Zeron\model-catalogs\claude-code\<sha256>.json` exists with `binary_version: 2.1.296`. The Claude catalog probe described below therefore ran here.
- `%LOCALAPPDATA%\Zeron` has no `adapters` folder, so the Cursor SDK shim and Antigravity have never been installed on this machine. Of the probes described here, only Claude's has left a trace.

Paths are relative to `crates/harness/src/` unless prefixed. Tags: **[O]** means read in the code, **[I]** means inferred.

## Headline

The Rust process links **no** agent SDK, and Zeron does **not** use ACP for the major agents. `lib.rs:1-15` [O] says so in its own words. The only SDK, `@cursor/sdk`, runs out of process in a Node shim.

**Native wires, driven directly from Rust:**

| Agent | Wire |
|---|---|
| Claude Code | stream-json plus its stdio control protocol |
| Codex | `codex app-server` JSON-RPC |
| OpenCode | `opencode serve` HTTP + SSE |
| Pi | `pi --mode rpc` JSONL |

**One vendor SDK:** Cursor runs `@cursor/sdk@1.0.32` inside a Zeron-written Node shim (`cursor/shim.mjs`). Rust talks to that shim over a Zeron-defined JSONL protocol.

**ACP, only for ACP-native agents:** Devin (`devin acp`), Grok (`grok agent stdio`), Hermes (`hermes acp`) and Antigravity (`agy_acp_server`).

**ACP for Claude, Codex, Cursor and OpenCode was retired.** It used to go through adapters. The adapters held turns open for background work that the CLIs settle eagerly, which caused false "still running" and "done" states (`lib.rs:11-15`, `acp/mod.rs:8-11` [O]). The other recorded reasons [O]:
- Cursor's ACP output strips subagent transcripts (`cursor/mod.rs:5-6`).
- OpenCode's ACP layer settles on the first idle event it cannot match to a turn, and drops subagent and thinking traffic (`opencode/mod.rs:4-10`).

Every driver implements one trait, `Harness` (`lib.rs:153-262`). The trait covers:
- `installed()`, `executable_path()`, `models()` and `model_catalog()`;
- `run()`, which returns a stream of `AgentEvent`s ending in `Done`;
- `RunControls`, which carries `request_input`, the `steering` mailbox, `interrupt` and `turn.stop_turn()`;
- capability flags: `deterministic_turn_end`, `stops_turn_in_place` and `reconfigures_in_place`.

## 1. How agents are detected

**Install check**
- "Installed" means a launchable file was found (`claude/mod.rs:444`, `codex/mod.rs:617`, `acp/mod.rs:2178` [O]).
- Listing or detecting agents never runs an installer (`README.md` [O]).

**Search order** (`executable.rs:513-526` [O])
1. The process `PATH`.
2. A login-shell PATH snapshot. This is Unix only: `$SHELL -l -i -c env` with a 5 s timeout (`shell_env.rs:33-148`). It returns nothing on Windows.
3. The agent's own extra directories.
4. Node version-manager bin directories. On Windows these are fnm, nvm-windows, Volta, pnpm, `%APPDATA%\npm`, scoop, `~\.bun\bin` and `~\.local\bin` (`:309-378`).

**Which copy wins**
- Every runnable candidate is collected, not only the first one. Among them, the newest `--version` semver wins (`executable.rs:81-97, 122-144` [O]).
- The version probe has a 2 s timeout. Its result is cached by (canonical path, mtime, size), including failures (`:146-289` [O]).

**Windows specifics** [O]
- Names are tried with the PATHEXT extensions, filtered to `.com`, `.exe`, `.bat` and `.cmd`. `.ps1` is never a candidate (`executable.rs:405-468`).
- `.cmd` and `.bat` shims run through `cmd.exe /e:ON /v:OFF /d /c` (`process/windows/command.rs:119-125`).
- npm is located as `node` + `node_modules\npm\bin\npm-cli.js` (`adapter_install.rs:148-176`).

**Binary names, extra locations and overrides** [O]

| Agent | Binary | Extra locations | Override env var |
|---|---|---|---|
| Claude | `claude` | `~/.claude/local`, `~/.local/bin` | `CLAUDE_CODE_EXECUTABLE` |
| Codex | `codex` | `~/.codex/bin` | `CODEX_EXECUTABLE` |
| Cursor | `cursor-agent` (presence gate only) | `%LOCALAPPDATA%\cursor-agent` | none |
| OpenCode | `opencode` | | `OPENCODE_EXECUTABLE` |
| Pi | `pi` | | `PI_EXECUTABLE` |
| Grok | `grok` | | `GROK_EXECUTABLE` |
| Devin | `devin` | | `DEVIN_EXECUTABLE` |
| Hermes | `hermes` | | `HERMES_EXECUTABLE` |
| Antigravity | `agy_acp_server` | managed archive | `ANTIGRAVITY_ACP_EXECUTABLE` |

On Windows an override must exist and end in `.exe`, `.cmd`, `.bat` or `.com` (`executable.rs:49-79`).

**Model catalogs**
- Each catalog is fetched by spawning the agent [O]:
  - **Claude:** a stream-json `initialize` control request.
  - **Codex:** `app-server` `model/list`.
  - **Cursor:** `node shim models`.
  - **Pi:** `get_available_models`.
  - **OpenCode:** `/provider` or `/api/model`.
  - **Devin:** `devin models list --format json`.
- Catalogs are cached on disk at `<data_dir>/model-catalogs/<harness>/<hash>.json` (`crates/engine/src/model_catalogs.rs:21-67` [O]).
- The hash is SHA-256 over the binary path, its version, mtime and size, the agent's credential/config files, and agent-prefixed environment variables (`model_context.rs:14-128` [O]). A changed login or binary therefore produces a new catalog.

**Reaching the UI and other devices** [O]
- `ListHarnesses` returns a `HarnessDescriptor {id, name, supportsSteering, steeringMode, reasoningLevels, installed, canInstall, enabled}`.
- A harness is enabled when it is detected and not switched off in `harness-prefs.json` (`crates/engine/src/registry.rs:22-99, 355-372`).
- Remote devices pull these through relay-forwarded RPCs with `targetDeviceId` (`crates/engine/src/rpc.rs:1422-1435`).

**Antigravity trust** [O]
- Archives are pinned by sha512.
- Unpinned registry builds must pass a code-signature check: on macOS, `codesign` against Google's team `EQHXZ8M8AV`; on Windows, `WinVerifyTrust` with the leaf certificate's CN and O both "Google LLC" (`code_signature.rs`).

## 2. ACP client (Devin, Grok, Hermes, Antigravity)

**Transport** [O]
- A hand-written ACP v1 client speaks newline-delimited JSON-RPC 2.0 over stdio (`jsonrpc.rs:212-370`).
- On Windows the child starts `CREATE_SUSPENDED|CREATE_NO_WINDOW` inside a kill-on-close Job Object (`process/windows/mod.rs:127-152`).

**Handshake** [O]
- `initialize` sends `{protocolVersion:1, clientInfo:{name:"zeron"}, clientCapabilities:{fs:{readTextFile:false, writeTextFile:false}, terminal:false}}` (`acp/mod.rs:2501-2525`).
- Zeron deliberately **declines** fs and terminal, so agents use their own file and shell access.
- Devin also gets `_meta["cognition.ai/subagentSupport"]: true`.

**Session setup** [O]
- `session/new {cwd, mcpServers}`.
- `session/load` is used for resume. Replayed history is dropped during the load.
- The model goes through `session/set_model`, or through `session/set_config_option` (categories `model`, `thought_level` and `mode`). For `mode`, Zeron prefers `bypassPermissions`, `bypass`, `yolo` or `agent-full-access` (`acp/mod.rs:2660-2832`).
- There is no `session/set_mode` call.

**Prompt** [O]
- `session/prompt {sessionId, prompt:[{type:"text", text}]}` sends text only. Images reach the agent only as path references in the text [I].

**Normalization** (`acp/normalize.rs:349-447` [O])

| `session/update` kind | Becomes |
|---|---|
| `agent_message_chunk` | TextDelta |
| `agent_thought_chunk` | ReasoningDelta |
| `tool_call` / `tool_call_update` | typed ToolCall / ToolResult (output capped at 16 KB, diffs at 64 KB) |
| `plan` | Todo |
| `available_commands_update` | AvailableCommands |
| `usage_update` | ContextUsage |

**Requests from the agent** [O]
- `session/request_permission` is **auto-approved**, preferring `allow_always`, then `allow_once`, then the first option (`acp/normalize.rs:488-503`).
- If any option has no kind, the request is treated as a question and routed to the user through `request_input`.
- All other agent→client methods (`fs/*`, `terminal/*`, …) get `-32601` (`acp/mod.rs:3026-3031`).

**Turn end and stop** [O]
- The turn ends on the `session/prompt` response's `stopReason`. For Grok, the vendor extension `_x.ai/session/prompt_complete` also settles it.
- Stop sends `session/cancel`. A hard interrupt escalates after a 2 s grace, then kills after another 3 s.
- Where the agent advertises support, steering goes through `_session/steering`. Otherwise Zeron cancels and re-prompts once no tool is open.
- Grok has a 30 s stall watchdog.

**Subagents** [O]
- **Grok:** Zeron tails `~/.grok/sessions/<cwd>/<child>/chat_history.jsonl` (`acp/subagent.rs`).
- **Devin:** child traffic is identified by `cognition.ai/*` `_meta` tags (`acp/subagent_devin.rs`).

## 3. Native wires

### Claude Code

**Launch** (`claude/mod.rs:213-277` [O])

The process is persistent:

```
claude --print --input-format stream-json --output-format stream-json --verbose
       --include-partial-messages --replay-user-messages --thinking-display summarized
       --permission-prompt-tool stdio [--model M] [--effort E] [--resume=ID] [--mcp-config JSON]
       (--permission-mode bypassPermissions --dangerously-skip-permissions | --permission-mode default)
```

`--permission-prompt-tool stdio` is undocumented. The code says it was "validated live against 2.1.228".

**stdin** [O]
- First line: control request `{"subtype":"initialize"}`.
- Prompts: `{"type":"user",...}` lines.
- Steers: user lines with `priority:"now"` or `"next"`.
- Live model switch: `set_model` and `apply_flag_settings` control requests.
- Images: base64 blocks up to 5 MB.

**Control** [O]
- Every `can_use_tool` is answered `{"behavior":"allow"}`, whatever `auto_approve` is set to (`claude/mod.rs:1326-1368`).
- `AskUserQuestion` is intercepted and sent to the user. The answers come back in `updatedInput`.

**Stop and turn end** [O]
- Stop: `{"subtype":"interrupt","cancel_queued":true}`; the process stays up.
- Turn end: the `result` frame.
- Frames carrying `parent_tool_use_id` become Subagent events.

### Codex

**Launch** [O]
- `codex app-server`, validated against codex-cli 0.153.4, using the experimental API.
- `initialize {clientInfo:{name:"zeron-native"}, capabilities:{experimentalApi:true}}`, then `initialized`.
- Then `thread/start` or `thread/resume`.

**Turns** [O]
- `turn/start {threadId, input, approvalPolicy, sandboxPolicy, ...}`.
- `turn/steer {expectedTurnId, ...}`.
- `turn/interrupt`.
- `/compact` maps to `thread/compact/start` and `/review` to `review/start`.

**Permissions** [O]
- Always `approvalPolicy:"never"` with the `danger-full-access` sandbox. Title runs use read-only (`codex/mod.rs:770-781`).
- Stray `requestApproval` requests are accepted or sent to the user.
- `item/tool/requestUserInput` is answered with the user's input.

**Turn end** [O]: `turn/completed`, `turn/failed` or `turn/aborted`.

### Cursor (the one SDK)

**Runtime** [O]
- `node %LOCALAPPDATA%\Zeron\adapters\...\zeron-cursor-shim-<sha256>.mjs`.
- `@cursor/sdk@1.0.32` is npm-installed into that adapters directory (`adapter_install.rs:66-103, 416-509`).

**Rust ↔ shim JSONL** [O]
- stdin ops: `run`, `user`, `steer`, `interrupt`, `stop`.
- stdout events: `ready`, `text`, `thinking`, `tool`, `usage`, `turn`, `fatal`.

**SDK calls** [O]: the shim calls `Agent.create`/`Agent.resume`, `agent.send(...,{onDelta})`, `run.steer`, `run.cancel` and `Cursor.models.list()`.

**Permissions** [O]: there is no approval channel. `askQuestion` and `generateImage` are disallowed.

**Pin updates** [O]: a daily CI job bumps the pin (`.github/workflows/cursor-sdk-update.yml`).

**Install gate** [O]: `installed()` checks only that `cursor-agent` exists. Chats never run it.

### OpenCode

**Launch** [O]
- `opencode serve --port <free> --hostname 127.0.0.1`.
- HTTP Basic auth uses a random `OPENCODE_SERVER_PASSWORD`. `OPENCODE_CLIENT=zeron`.
- MCP is merged into `OPENCODE_CONFIG_CONTENT`.
- Both the v1 and v2 HTTP APIs are supported.

**Prompts and events** [O]
- Prompts: `POST /session/{id}/prompt_async` (v1) or `/api/session/{id}/prompt` (v2).
- Events come over SSE: `/global/event` (v1) or `/api/event` (v2).

**Permissions** [O]
- `permission.asked` is always answered `"once"`.
- `question.asked` is routed to the user.

**Stop and turn end** [O]: stop is `/session/{id}/abort`. The turn ends on `session.idle`.

### Pi

**Launch** [O]: `pi --mode rpc --no-themes [--session ...] [--extension zeron-mcp.mjs]`. It requires Pi 0.85.1 or newer.

**Protocol** [O]
- Plain JSONL, not JSON-RPC.
- Commands: `prompt` (with `images` and `streamingBehavior` steer/followUp), `set_model`, `set_thinking_level`, `abort`.
- The turn ends on `agent_settled`.

**Permissions** [O]: there is no permission channel. Only `extension_ui_request` reaches the user.

## 4. Cross-cutting behaviour worth knowing

**Approval posture.** Zeron runs agents essentially unattended, and the details differ by agent [O]:

| Agent | Approval behaviour |
|---|---|
| Claude | always `allow` |
| Codex | `never` + full access |
| OpenCode | always `once` |
| ACP agents | auto-select `allow_always` |
| Cursor, Pi | no approval channel |

Only question-type prompts reach the user.

**MCP injection.** The engine injects its own MCP server into every agent [O]:
- Claude: `--mcp-config`.
- Codex: dotted `config` overrides.
- ACP agents: `session/new` `mcpServers`.
- OpenCode: config environment variable.
- Pi: an extension.
- Cursor: the shim's `mcp` field.

**Windows teardown.** Agents run in a kill-on-close Job Object. `send_signal` ignores TERM vs KILL and terminates the whole job (`lib.rs:601-605` [O]). Graceful shutdown hooks therefore do not run on Windows [I].

**Environment hygiene.** `CLAUDECODE*` markers are stripped only for ACP launches, Pi and the installer (`process/owned.rs:6-17`). The installer also drops `ZERON_*` [O]. The version and catalog probes for Claude, Codex and Cursor inherit the full environment [O].

## Evidence ledger

| Claim | Evidence | Confidence |
|---|---|---|
| Installed build = source tag | `Cargo.toml:30`; 11 hits of `0.2.112` in the exe | High |
| Native wires vs ACP split | `lib.rs:1-15`; spot-checked `claude/mod.rs:213-252`, `codex/mod.rs:770-792` | High |
| ACP capabilities declined, permissions auto-allowed | `acp/mod.rs:2501-2525`, `acp/normalize.rs:488-503`, `acp/mod.rs:3026-3031` (re-read) | High |
| Newest-version executable selection | `executable.rs:81-97, 513-526` (re-read) | High |
| Claude catalog probe ran on this host | `%LOCALAPPDATA%\Zeron\model-catalogs\claude-code\*.json` | High |
| Other line citations | Read by sub-readers, spot-checked rather than re-read one by one | Medium |

## Doc and code inconsistencies found

1. `lib.rs` names `docs/research/acp.md` as its decision record. Searching that file for `retir` or `native` finds only the Grok, Hermes and Devin ACP entries, the Pi migration and a retired quiet timer (`acp.md:12, 34, 41, 44, 176`). The rationale for leaving ACP for Claude, Codex and Cursor lives only in code comments, and lines 16-31 still describe Claude and Codex as ACP. `docs/PARITY.md:59` still lists Cursor as ACP.
2. Steering mode for Grok and Devin is `StepBoundary` in the driver (`acp/mod.rs:246, 331`) but `TurnBoundary` in the engine descriptors (`crates/engine/src/registry.rs:652, 670`).
3. `ListHarnesses` re-runs detection on every call: `descriptors()` calls `installed()` for every harness (`crates/engine/src/registry.rs:498-521`) [O]. That detection can run `--version` probes when an agent has two or more candidates and the version cache is cold [I]. A comment in `crates/engine/src/harness_updates.rs:1-3` says the call does no subprocess work.

## Unresolved questions

- Nothing was observed at runtime beyond the cached catalog file. Wire behaviour is from static reading only, and no live session was captured.
- The Devin catalog path was only partly read.
- Whether any crate outside `engine/src` pushes harness descriptors to peers, rather than peers pulling them, was not searched exhaustively.
- `harness_updates.rs:2066-2083` `classify_source` matches forward-slash paths only. Windows npm shims may be labelled "Vendor" [I, untested].
