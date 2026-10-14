# Spike S5: native Claude and Codex wires on real CLIs

Date: 2026-10-14. Host: Windows Server 2022 VM (CPU only).

- **Claude Code 2.1.296**, signed in with a claude.ai account (OAuth), driven by a Node probe over pipes with `--model haiku`.
- **Codex CLI 0.162.1**, signed in with ChatGPT, driven by a second Node probe over `codex app-server` stdio with model `gpt-6-luna` at effort `low`.

The model choice does not affect either wire. Scratch projects were small git repos with `hello.txt` and a project instruction file holding a codeword (`CLAUDE.md`, or `AGENTS.md` for Codex); the Claude project also had a `.claude/settings.json` with one Bash allow rule.

## Result

**Both native wires work on real CLIs on Windows.**
- Permission requests reach the client, a refusal blocks the action, approval lets it run, an interrupt stops the turn while the process stays up, and resume continues the conversation.
- Both CLIs also behave differently from what M5a phases 2 to 4 assume. The changes M5a needs are summarised here, and the details follow in each CLI's section.

**Changes M5a needs:**

1. **Turn end (phases 2, 3 and 4).** Neither CLI's turn-end frame means "the agent's work is finished".
   - **Claude:** a background command produces a second, unprompted turn with its own `result`. With `--replay-user-messages`, it is recognisable because no `isReplay` user frame precedes it and a `task_notification` does.
   - **Codex:** sends no extra turn. Instead, a command can still be running after `turn/completed`, and its `item/completed` arrives later, with the finished turn's `turnId`.
   - **Detached children are invisible:** a child started with `Start-Process` produces no frame at all, neither a late item nor a turn. Only containment can see it.
   - Phase 4's "turn end exactly once" must become "each prompt's turn ends exactly once, plus Claude's autonomous turns and Codex's late item completions". Candidate capture must keep waiting for quiescence.
2. **Codex stop (phase 3).** Codex 0.162.1 has no `turn/aborted`. `turn/interrupt` returns `{}` and is followed by `turn/completed` with `status: "interrupted"`. That interrupt does not kill the running command, which stays alive until it ends or the app-server exits.
3. **Codex approvals (phase 3).**
   - `approvalPolicy: "on-request"` with the `workspace-write` sandbox asked for nothing: every command ran in the sandbox.
   - `untrusted` asks before every command, read-only ones included. The most likely reason is that Windows commands are wrapped in `powershell.exe -Command`.
   - For user-answered permissions, phase 3 must use `untrusted` plus `workspace-write`, not `on-request`.
   - `decline` worked even though the request's `availableDecisions` did not list it. Phase 3 should answer from `availableDecisions` (using `cancel` to refuse) or pin the validated version.
   - The client must answer server-to-client JSON-RPC requests (`item/commandExecution/requestApproval`, `item/fileChange/requestApproval`) with `{"decision": "accept"|"decline"|"cancel"|…}`.
4. **Session stop (phases 2 and 3).** The two CLIs behave in opposite ways:
   - Closing Claude's stdin waits for background tasks and their follow-up turn (32.7 s here).
   - Closing Codex's stdin exits within 0.5 s and kills commands running in its exec sessions. A detached `Start-Process` child was not tested this way; it had already left the tree.
   - Both drivers need `interrupt`, then stdin close, then a grace period, then a containment kill. The Claude driver must keep its permission channel live between turns, because autonomous turns can call tools.
5. **Quiescence (phase 4 and M3 containment).** A parent-process walk is not a reliable "tree quiet" signal on Windows:
   - MCP servers, hooks and Codex's `codex-code-mode-host` are long-lived children.
   - Codex runs commands through `codex-command-runner` as a separate sandbox user.
   - Detached `Start-Process` children leave the tree entirely.
   - PID reuse makes naive `ParentProcessId` walks attribute unrelated processes.
   - Quiescence must come from the writer containment (a Job Object), excluding the MCP and helper processes the driver itself started.
6. **Claude launch details (phase 2):**
   - `--permission-prompt-tool stdio` is hidden but works.
   - `--permission-mode default` is accepted although `--help` no longer lists it.
   - A bare `{"behavior":"allow"}` and `cancel_queued: true` both work.
   - `--bare` cannot be used with a claude.ai login.
   - Which user settings load needs a user decision (see the matrix).
7. **Fakes (phase 4).** Generate the Codex fake against `codex app-server generate-json-schema`. Replay both CLIs' background, interrupt, deny or decline, and resume transcripts saved in `spike-native-wires/`.

### Claude Code summary

**Claude Code passes.** Every check in the spike works on 2.1.296:

- `--permission-prompt-tool stdio` is accepted and every gated tool arrives as a `can_use_tool` control request.
- A `deny` answer blocks the edit, and the file stays unchanged.
- A bare `{"behavior":"allow"}` answer lets the edit through; `updatedInput` is not required.
- `interrupt` (with `cancel_queued: true`) stops a streaming turn and a running foreground tool, and the process stays up for the next prompt.
- `--resume <session_id>` continues the session.

The run also turned up three findings that change the M5a contract:

1. **A background command produces a second, unprompted turn.**
   - The first turn's `result` arrived at 5.6 s; the background `sleep 45` was still running.
   - When the command finished (49.3 s), Claude Code emitted `task_updated`, `task_notification` and `background_tasks_changed`, then a fresh `system/init`, a streamed assistant message, and a second `result` (52.1 s, `result_index: 1`).
   - Nobody sent a prompt. Phase 2's "turn end on `result` only" and phase 4's "turn end exactly once" do not hold as written.
   - A driver can tell this turn apart: with `--replay-user-messages`, every prompted turn starts with `init` followed by a `user` frame carrying `isReplay: true`. The autonomous turn has no replayed user frame and directly follows a `task_notification`.
   - The process tree only went quiet at 51.2 s, which confirms that candidate capture must wait for quiescence and not for `result`.
2. **Closing stdin does not end a session that has a background task.**
   - With stdin closed right after the first `result`, the process kept running until the background command finished.
   - It then ran the autonomous follow-up turn (including a `Read` tool call) and exited 32.7 s after the close.
   - Session stop therefore cannot rely on stdin close plus a short grace period; it needs the containment kill.
3. **"Tree quiet" must exclude MCP servers.**
   - Every MCP server that Claude Code starts (the user's `rea` server here) is a long-lived child of `claude.exe`.
   - User hooks also briefly spawn `bash.exe` → `node.exe` children.
   - Quiet has to mean "no descendants other than MCP servers and `conhost.exe`", or it never arrives. The timing runs below used `--strict-mcp-config` to get a clean tree.

Other findings that affect phase 2:

- **`--permission-prompt-tool` is a hidden flag in 2.1.296.** It has no help line of its own and appears only inside the description of `--permission-prompts host|none`.
- **The help text omits `default` from the `--permission-mode` choices.** The choices listed are `acceptEdits`, `auto`, `bypassPermissions`, `manual`, `dontAsk` and `plan`. `--permission-mode default` is still accepted, and both the initialize response (`current_permission_mode`) and `system/init` (`permissionMode`) report `"default"`.
- **`--bare` cannot be used with a claude.ai login.** It never reads OAuth credentials, so the first prompt returns `result` `"Not logged in · Please run /login"`, `is_error: true`, and the process exits with code 1. It works only with `ANTHROPIC_API_KEY` or an `apiKeyHelper`.
- **Claude Code refuses a long foreground `sleep` by itself.**
  - The tool result is `<tool_use_error>Blocked: sleep 40 followed by: echo finished. …`, and it comes before any `can_use_tool`.
  - It also happened with `--setting-sources "" --strict-mcp-config`, so it is not a user hook.
  - `sleep 5 && echo finished` ran with no block and no `can_use_tool`.
- **Long foreground Bash commands are tracked as tasks.** They emit `task_started` with `is_backgrounded: false`, and `task_notification` with `status: "stopped"` when interrupted.

### Codex summary

**Codex passes, with differences from the plan.** On 0.162.1:

- **Handshake and turns.** `initialize`, then `initialized`, then `thread/start`, then `turn/start` all work.
- **Approval policy.** `untrusted` sends an `item/commandExecution/requestApproval` for every command. `on-request` with `workspace-write` sends none.
- **Decline and accept.** `decline` blocks the command (item `status: "declined"`, no file) and the turn continues. `accept` lets it run.
- **Interrupt.** `turn/interrupt` ends the turn as `turn/completed` `status: "interrupted"`; no `turn/aborted` exists. The app-server stays up, but the running command does not stop.
- **Resume.** `thread/resume` continues the thread.
- **Background commands.** `turn/completed` arrives while the command still runs. Codex later sends that item's `item/completed` but never starts an unprompted turn.

## Claude Code

### Version and launch

`claude --version` reports `2.1.296 (Claude Code)`. Every run removed `CLAUDECODE` and all `CLAUDE_CODE_*` variables from the child's environment, and set the working directory to the scratch project.

The flags phase 2 specifies, used for the recorded transcripts:

```
claude --print --input-format stream-json --output-format stream-json --verbose
       --include-partial-messages --replay-user-messages
       --permission-prompt-tool stdio --permission-mode default --model haiku
       [--strict-mcp-config] [--setting-sources <list>] [--resume <session_id>]
```

The first runs (`main`, `interrupt`, `background` without the `plan-` prefix) left out `--replay-user-messages`, answered allow with `updatedInput`, and sent `interrupt` without `cancel_queued`. They gave the same results. The transcripts are taken from the runs that use the plan's exact shapes.

### Frame shapes

The examples below are excerpts from the sanitized transcripts, shortened with `…`.

**Initialize.** The probe writes this as the first stdin line:

```json
{"type":"control_request","request_id":"probe_1","request":{"subtype":"initialize"}}
```

The answer arrives about 1.2 s later. It can come after the `SessionStart` hook frames.

```json
{"type":"control_response","response":{"subtype":"success","request_id":"probe_1","response":{
  "commands":[…],"agents":[…],"output_style":"default","models":[…],"account":{…},"pid":19580,
  "claude_code_version":"2.1.296","current_permission_mode":"default","session_state":"idle",
  "fast_mode_state":"off","capabilities":["ui_surface_v1"], …}}}
```

`claude_code_version` gives the version gate a second source besides `claude --version`. `account` carries the signed-in identity; never log it.

**`system/init` is re-emitted at the start of every turn,** including the autonomous one. It carries:

- `session_id`, `cwd`, `model` and `permissionMode`;
- `tools` (32 built-in tools, plus `mcp__…` tools when MCP servers load);
- `mcp_servers` (`[{name,status,source}]`, where status is `connected`, `pending` or `needs-auth`);
- `skills`, `agents`, `slash_commands` and `plugins`;
- `memory_paths.auto` (the auto-memory directory only; CLAUDE.md files are not listed);
- `capabilities`: `interrupt_receipt_v1`, `interrupt_cancel_queued_v1`, `interrupt_send_now_v1`, `msg_lifecycle_v1`, …

**Prompt.** The probe writes:

```json
{"type":"user","message":{"role":"user","content":"…"},"parent_tool_use_id":null,"session_id":""}
```

With `--replay-user-messages` the prompt is echoed back as `{"type":"user",…,"isReplay":true}` right after that turn's `init`.

**Permission request.** Claude Code writes:

```json
{"type":"control_request","request_id":"<uuid>","request":{"subtype":"can_use_tool","tool_name":"Edit",
  "display_name":"Edit","input":{"file_path":"<PROJECT>\\hello.txt","old_string":"hello","new_string":"denied","replace_all":false},
  "description":"hello.txt","permission_suggestions":[{"type":"setMode","mode":"acceptEdits","destination":"session"}],
  "tool_use_id":"toolu_…"}}
```

Bash requests also carry `decision_reason` (`"This command requires approval"`) and `decision_reason_type`. A write outside the recognised working directories adds `blocked_path` and an `addDirectories` suggestion.

**Deny answer.** The probe writes:

```json
{"type":"control_response","response":{"subtype":"success","request_id":"<same uuid>","response":{"behavior":"deny","message":"Denied by the spike probe user."}}}
```

The model then receives a `tool_result` with `is_error: true` and the message as its content. The `user` frame carries `tool_result_meta: [{"non_execution_kind":"permission-rule","permission_decision":{"decision":"reject","source":"user_reject"}}]`. The turn's `result` lists the call in `permission_denials`, and `hello.txt` stays `hello`.

**Allow answer.** A bare `{"behavior":"allow"}` in the same envelope is accepted, and `hello.txt` became `allowed`. With `--replay-user-messages`, Claude Code also echoes the probe's `control_response` on stdout; a driver must ignore control responses it did not request.

**Interrupt.** The probe writes:

```json
{"type":"control_request","request_id":"probe_2","request":{"subtype":"interrupt","cancel_queued":true}}
```

The acknowledgement comes within 10 ms:

```json
{"type":"control_response","response":{"subtype":"success","request_id":"probe_2","response":{"still_queued":[],"cancelled":[]}}}
```

What follows depends on what the turn was doing:

- **During text streaming:** a `user` frame `[Request interrupted by user]`, then `result` with `subtype: "error_during_execution"`, `is_error: true`, `terminal_reason: "aborted_streaming"`, `stop_reason: null`.
- **During a running foreground Bash (`ping -n 40`):**
  - `task_notification` with `status: "stopped"`.
  - A `tool_result` with `is_error: true`, "The user doesn't want to proceed…", and `tool_result_meta` with `non_execution_kind: "user-rejected"`.
  - `[Request interrupted by user for tool use]`.
  - `result` with `error_during_execution` and `terminal_reason: "aborted_tools"`.
  - The `bash.exe` → `ping` tree was gone 2 s later.

In both cases the next prompt ran normally ("alive"). No `result` reports `subtype: "interrupted"`, so a driver should treat `terminal_reason` `aborted_*` after its own interrupt as a stopped turn rather than a failure.

**Result.** `{"type":"result","subtype":"success","is_error":false,"num_turns":2,"result":"refused","stop_reason":"end_turn","terminal_reason":"completed","session_id":…,"permission_denials":[…],"queued_turn_count":0,"result_index":0,"usage":{…},"total_cost_usd":…,…}`. `result_index` counts results per process, and it also counts the autonomous one.

**Resume.** A new process with `--resume <session_id>` keeps the same `session_id` in `init`, fires `SessionStart:resume` hooks, and answered "allowed" to "what word did you put into hello.txt" without using tools. Phase 2 uses `--resume=ID`; only the space-separated form was run here.

### Background command timing

Prompt: "Use the Bash tool with `run_in_background` … `sleep 45 && echo bgdone > bg.txt` … immediately reply 'started'". `--strict-mcp-config` was used for a clean tree, and the tree was polled every ~2.4 s through `Get-CimInstance Win32_Process`.

| t (ms) | event |
| --- | --- |
| 3,664 | `can_use_tool` Bash (`run_in_background: true`), allowed |
| 4,260 | `background_tasks_changed`, `task_started` (`is_backgrounded: true`, `task_type: "local_bash"`) |
| 5,643 | `result` success, "started" (`result_index: 0`); tree: two `bash.exe` |
| 49,345 | `task_updated` (`status: completed`), `task_notification` (`status: completed`, `output_file`, `summary`), `background_tasks_changed` (`tasks: []`) |
| ~49,600 | a user hook briefly runs (`bash.exe` → `node.exe`) |
| 50,116 | `system/init` (no replayed user frame), then a streamed assistant message |
| 51,219 | process tree quiet (first empty poll) |
| 52,072 | second `result` success (`result_index: 1`): "The background command finished…" |

`bg.txt` contained `bgdone`. The earlier run without `--replay-user-messages` gave the same timings: `result` at 5.5 s, quiet at 52.0 s, and a second `result` at 52.0 s.

The stdin-close variant (`sleep 30`) went as follows:

- First `result` at 5.7 s, then stdin was closed.
- `task_notification` at 34.3 s.
- An autonomous turn read `bg2.txt` with `Read`, which needs no permission, and emitted `result` at 37.9 s.
- The process exited with code 0 at 38.7 s, 32.7 s after the close.
- None of the tree's processes survived.

### Settings-source matrix

Six configurations were run with the same prompt: Read, `git status --short`, `echo hi`, Write a new file, Edit `hello.txt`. A second prompt then ran under the `all`, `""`, `project` and `user` source settings. It covered `touch t1.txt` and `ping -n 1` (with a project allow rule for each), plus a request to quote a line from each CLAUDE.md. In the table, "all" means no `--setting-sources` flag, that is user, project and local.

| Config | Hooks seen in the stream | MCP servers in `init` | Skills / agents / slash commands | User `~/.claude/CLAUDE.md` | Project `CLAUDE.md` |
| --- | --- | --- | --- | --- | --- |
| all | 2× `SessionStart:startup` | `rea` (user, pending) and 11 claude.ai connectors (1 connected, adding 8 `mcp__claude_ai_…` tools; 10 needs-auth) | 37 / 10 / 72 | loaded | loaded |
| `--strict-mcp-config` | 2× `SessionStart:startup` | none | 37 / 10 / 72 | loaded | loaded |
| `--setting-sources user` | 2× `SessionStart:startup` | same as all | 37 / 10 / 72 | loaded | not loaded |
| `--setting-sources project` | none | none | 19 / 5 / 54 | loaded (see note) | loaded |
| `--setting-sources ""` | none | none | 19 / 5 / 54 | not loaded | not loaded |
| `--setting-sources "" --strict-mcp-config` | none | none | 19 / 5 / 54 | not loaded | not loaded |
| `--bare` | none | none | 18 / 5 / 51, tools `Bash, Edit, PowerShell, Read` only | n/a | n/a |

The `--bare` row is not usable: the run fails "Not logged in", as described in the Result section.

Notes:

- **Hooks.** User hooks load only with the `user` source. Only `SessionStart` hooks surface as `system/hook_started` and `hook_response` frames. Tool hooks (`PreToolUse`, `PostToolUse`, `Stop`) are configured in the user settings but emitted no frames. One hook process (`bash.exe` → `node.exe`) was seen in the process tree during the background run.
- **MCP servers.** User MCP servers from `~/.claude.json` and claude.ai connectors load with the `user` source, and both `--setting-sources ""` and `--strict-mcp-config` remove them. M5a's `--mcp-config` with `--strict-mcp-config` gives exactly the studio server.
- **Skills, agents and commands.** User skills, agents (`advisor`, `brainstormer`, `code-reviewer`, `planner`, `scout`) and commands come from the `user` source. The built-in 19 skills, 5 agents and 4 built-in plugins (`cc-plugin-sec-default`, `cc-plugin-agents-md`, `cc-plugin-telemetry`, `cc-plugin-plugin-authoring`) always load.
- **CLAUDE.md.** These columns are what the model reported (it quoted the lines verbatim); no frame lists the files loaded. Under `--setting-sources project` the model still quoted `~/.claude/CLAUDE.md`. This is most likely the project-memory walk up from the working directory, which finds `~/.claude/CLAUDE.md` (the home directory is an ancestor) as an ancestor's `.claude/CLAUDE.md`, because the scratch project is under the user's home. M5a draft directories under the home directory will behave the same way.

Which tools produced `can_use_tool`:

- **Never asked in any configuration:** `Read`, read-only Bash (`git status --short`, `echo hi`), and a `Read` inside an autonomous turn.
- **Always asked in every configuration:**
  - `Write` and `Edit` (default mode).
  - Bash commands that are not read-only: `ping` and `touch`.
  - A `sleep` with `run_in_background` also asked, but it was only run with `--strict-mcp-config`.
- **Allow rules:**
  - A rule passed with `--settings '{"permissions":{"allow":["Bash(ping:*)"]}}'` was honored: no `can_use_tool`, and the command ran.
  - The same rule in the project's `.claude/settings.json` was not honored with `all`, `project` or `user` sources.
  - The most likely cause is the scratch location. The project lives under Claude Code's own `%TEMP%\claude` directory and was never trusted, and every write there also reported `blocked_path` even with `--add-dir`.
  - This needs a recheck from a normal, trusted directory before phase 2 relies on project allow lists.

### Transcripts

The transcripts are in `reports/spike-native-wires/`. They are sanitized as follows:

- Paths are replaced with `<PROJECT>`, `<PROJECT_SLUG>`, `<HOME>`, `<TEMP>` and `<USER>`.
- UUIDs, `msg_` and `toolu_` IDs are replaced with stable fakes, consistent across files, so the resume pair shares one session ID.
- `account`, thinking signatures and hook output are redacted.
- Content deltas are trimmed to three per kind per message; the first line of each file records the exact launch flags, the answer shapes the probe used, and how many deltas were dropped. `messaging_socket_path` is also redacted.

Each line has the form `{t, dir: in|out|note, frame}`.

| File | Content |
| --- | --- |
| `claude-permission-deny-allow.jsonl` | initialize; Edit denied (file unchanged); Edit allowed with bare allow (file changed); "alive" turn |
| `claude-resume.jsonl` | `--resume` of the session above recalling "allowed" |
| `claude-interrupt.jsonl` | interrupt during streaming, then during a foreground Bash, then a normal turn; process-tree notes |
| `claude-background.jsonl` | background command, first `result`, task frames, autonomous turn and second `result`; tree polls |
| `claude-background-stdin-close.jsonl` | stdin closed while a background task runs; autonomous turn with `Read`; exit timing |
| `claude-tools-default-mode.jsonl` | all settings sources: Read and read-only Bash not asked, Write and Edit asked; `init` with MCP server statuses |
| `claude-bare-not-logged-in.jsonl` | `--bare` with an OAuth login: initialize works, the first prompt fails "Not logged in" |

### Implications for M5a

- **Phase 2, turn end.** A `result` that follows a `task_notification`, and was not preceded by a replayed user frame, is an autonomous turn. The driver must surface it (as a new agent turn or a continuation), not drop it or count it against "exactly once". `--replay-user-messages` is required to tell the two apart.
- **Phase 2, session stop.** Closing stdin waits for background tasks to finish, so stop is: `interrupt`, close stdin, wait `shutdown_grace`, then kill through containment. Interrupting a running *background* task was not tested.
- **Phase 2, permissions between turns.** Autonomous turns can call tools; the stdin-close run made a `Read` call. A gated tool in such a turn would send `can_use_tool` while no user turn is active, so the permission channel must stay live between turns. After stdin is closed, nothing can answer such a request.
- **Phase 2, version gate.** Accept `--permission-mode default` (still valid). Refuse a version whose `claude --help` lacks `--permission-prompts` and therefore probably dropped the hidden stdio flag, or better, probe it with a no-op launch.
- **Phase 2, settings.** Decide `--setting-sources` and `--strict-mcp-config` with the user. `--bare` is not an option for claude.ai logins.
- **Phase 4, fake CLI.** Replay the background sequence (`task_*` frames, `init`, stream, second `result`) and the interrupt variants (`aborted_streaming`, `aborted_tools`). Echo the control response under `--replay-user-messages`.
- **Phase 4, quiescence.** Exclude MCP server processes and `conhost.exe` when deciding that the writer tree is quiet.

### Not covered

These items were outside the S5 steps and were not exercised: `AskUserQuestion` routing, steering with `priority`, image blocks, `TodoWrite` plans, `set_model`, and subagent frames (`parent_tool_use_id`). Whether `~/.claude/rules/*.md` loads was not checked separately from CLAUDE.md, although phase 2 lists rules as their own item.

## Codex

### Version and launch

- **Version:** `codex --version` reports `codex-cli 0.162.1`. `codex login status` reports "Logged in using ChatGPT" (plan type `plus`).
- **Launch:** the probe launched the native `codex.exe` from the npm package's `vendor/x86_64-pc-windows-msvc/bin` directly, as `codex app-server` (stdio transport, the default).
- **Working directory:** a scratch git repo with `hello.txt` and an `AGENTS.md` holding a codeword.
- **Model:** `gpt-6-luna` ("Fast and affordable model for easier tasks") at effort `low`, passed in `thread/start`. The model does not change the wire.
- **User config:** the user's `~/.codex/config.toml` was loaded unchanged. It adds the `rea` MCP server and `windows.sandbox = "elevated"`.
- **Protocol reference:** `codex app-server generate-json-schema --out DIR` writes the full JSON Schema of this version's protocol. Phase 3 and the fake CLI should be generated against it, not written by hand.

The machine already had local users `CodexSandboxOffline` and `CodexSandboxOnline` and four `codex_sandbox_offline_*` block firewall rules. The users were created at 06:40:25 local time, before this spike's first Codex run (about 07:30), so this spike did not create them; when the rules were created was not checked. The first command Codex ran in this spike waited 74 s between the model's tool call and `item/started`. No notification reported the delay. It was most likely one-time sandbox preparation for the new workspace; this is an inference. Later runs started commands within about 1 s.

### Frame shapes

The wire is JSON-RPC 2.0 over newline-delimited JSON. Messages carry `id` and `method`, and no `jsonrpc` field is needed.

**1. Handshake.** The probe sends:

```json
{"id":1,"method":"initialize","params":{"clientInfo":{"name":"fframes-spike","title":"fframes S5 spike","version":"0.0.1"},"capabilities":{"experimentalApi":true}}}
```

Codex answers within 100 ms: `{"userAgent":"fframes-spike/0.162.1 (Windows …)","codexHome":"<HOME>\\.codex","platformFamily":"windows","platformOs":"windows"}`. The probe then sends `{"method":"initialized"}`.

**2. Thread start.** The probe sends:

```json
{"id":2,"method":"thread/start","params":{"cwd":"<PROJECT>","approvalPolicy":"untrusted","sandbox":"workspace-write","model":"gpt-6-luna"}}
```

The result contains:
- `thread.id` (a UUIDv7) and `model`;
- `approvalPolicy`;
- the resolved `sandbox` (`{"type":"workspaceWrite","writableRoots":[],"networkAccess":false,…}`);
- `approvalsReviewer: "user"`;
- `instructionSources`, which lists the loaded `AGENTS.md` path. Unlike Claude Code, Codex reports which instruction files it loaded.

Notifications follow: `thread/started`, `mcpServer/startupStatus/updated` (`rea` and `codex_apps`, going from `starting` to `ready`), and `thread/settings/updated`.

**3. Turn.** The probe sends `{"id":3,"method":"turn/start","params":{"threadId":…,"input":[{"type":"text","text":"…"}],"approvalPolicy":"untrusted","effort":"low"}}`. The result is `{"turn":{"id":…,"status":"inProgress",…}}`. Notifications follow:
- `turn/started` and `thread/status/changed` (`active`);
- `item/started` and `item/completed` for `userMessage`, `agentMessage` (with `phase` `commentary` or `final_answer`), `reasoning`, `commandExecution` and `fileChange`;
- `item/agentMessage/delta`, `item/commandExecution/outputDelta` and `thread/tokenUsage/updated`;
- finally `thread/status/changed` (`idle`) and `turn/completed {threadId, turn:{id, status, error, items, startedAt, completedAt, durationMs}}`.

**4. Approval request.** This is a server-to-client *request*, with an `id` the client must answer:

```json
{"method":"item/commandExecution/requestApproval","id":0,"params":{"kind":"command","threadId":…,"turnId":…,"itemId":"exec-…",
  "command":"\"C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe\" -Command 'echo denied > deny.txt'","cwd":"<PROJECT>",
  "commandActions":[{"type":"unknown","command":"echo denied > deny.txt"}],"proposedExecpolicyAmendment":[…],
  "availableDecisions":["accept",{"acceptWithExecpolicyAmendment":{…}},"cancel"]}}
```

It arrives together with `item/started` (`commandExecution`, `inProgress`) and `thread/status/changed` with `activeFlags:["waitingOnApproval"]`. File edits arrive as `item/fileChange/requestApproval`, with the decisions `accept`, `acceptForSession`, `decline` and `cancel`.
- Its params are only `{threadId, turnId, itemId, startedAtMs, reason, grantRoot}`; the proposed change comes on the `fileChange` item's `item/started`.
- Declining it (`on-request`/`read-only`) completed the item as `declined`, the file was not written, and the turn completed.

**5. Decline.** The probe answers `{"id":0,"result":{"decision":"decline"}}`.
- Codex accepted this even though `availableDecisions` did not list `decline`; the schema does define it.
- The item completes with `status: "declined"`, `deny.txt` is not created, and the turn goes on to `turn/completed` `completed`. The model replied "declined".
- `cancel` declines and also interrupts the turn, per the schema; it was not run.

**6. Accept.** The answer is `{"decision":"accept"}`. The command ran, `allow.txt` was written, and the item completed with `status: "completed"` and an `exitCode`. Each answer is followed by `serverRequest/resolved {threadId, requestId}`.

**7. Interrupt.** The probe sends `{"id":7,"method":"turn/interrupt","params":{"threadId":…,"turnId":…}}`, which returns `{}`.
- `turn/completed` with `turn.status: "interrupted"` and `error: null` follows within 1 ms.
- **This version sends no `turn/aborted` notification.** Neither the notification list nor the observed frames contain it, and the turn statuses are `completed`, `interrupted`, `failed` and `inProgress`. M5a phase 3 must treat `turn/completed` with `status: "interrupted"` as the stop.
- The app-server stayed up, and the next turn answered "alive".
- **Interrupt does not stop the running command.** The `Start-Sleep -Seconds 40` PowerShell, under `codex-command-runner`, was still alive 2 s and 12 s after the interrupt.
- In a second run, closing stdin right after the interrupt made the app-server exit within 0.2 s, and that killed the command.

**8. Resume.** A new app-server process ran `thread/resume {threadId, cwd, approvalPolicy, sandbox}` and got the same thread ID back. The model answered "allowed" from the earlier turn without running commands.

**9. Stdin close.** The app-server exits within about 0.5 s of stdin closing, unlike Claude Code. That kills any command still running inside its exec sessions.

### Approval policy matrix

One prompt asked for five shell commands (`git status --short`, `Get-Content hello.txt`, `echo hi`, `echo x > m1.txt`, `ping …`) and one `apply_patch` file creation, with every request accepted.

| `approvalPolicy` / `sandbox` | Command approvals | File-change approvals | What happened |
| --- | --- | --- | --- |
| `on-request` / `workspace-write` | none | none | Every command and the file creation ran inside the sandbox without asking. |
| `on-request` / `read-only` | none | asked | Read-only commands ran; `echo x > m1.txt` failed in the sandbox, and the model did not ask to escalate. |
| `untrusted` / `workspace-write` | **every command, including `git status` and `Get-Content`** | asked | Everything ran after approval. |
| `untrusted` / `read-only` | not measured | not measured | The model declined to run anything ("this workspace is read-only"). |

**`untrusted` is the policy that asks before every command, including read-only ones.** The most likely reason is that on Windows every command is wrapped in `powershell.exe -Command '…'`, and every `commandActions` entry came back as `type: "unknown"`, so even `git status` is not treated as known-safe. `on-request` only asks when the model requests escalation out of the sandbox, and in these runs it never did. M5a needs `untrusted` with `workspace-write` for user-answered permissions.

### Background command timing

The background check produced three runs:

**Run 1: Codex's own exec session** (`codex-background-exec.jsonl`). The prompt asked the model to run `Start-Sleep -Seconds 45; Set-Content S5MARK-bg.txt done` directly and end the turn without waiting.

| t (ms) | event |
| --- | --- |
| 7,631 | `item/started` (`commandExecution`, `inProgress`) + `requestApproval`, accepted |
| 19,565 | `turn/completed` `completed`; the command item is still open, and `codex-command-runner` + `powershell.exe` are still running |
| 53,141 | `item/completed` for that command (`status: completed`, `exitCode: 0`, `durationMs: 45095`, `source: "unifiedExecStartup"`), carrying the finished turn's `turnId`, **after** `turn/completed` |
| ~53,500 | command-runner and PowerShell gone; the process tree is back to the baseline |

No `turn/started` and no second `turn/completed` followed. **Codex does not start an unprompted turn** when a background command ends; it only completes the dangling item late. The marker file was written.

**Run 2: a detached `Start-Process`** (`codex-background-start-process.jsonl`). The model wrapped the command in `Start-Process`.
- `turn/completed` arrived at 15.2 s.
- The detached PowerShell's parent had already exited, so it **never appeared in a parent walk from `codex.exe`**. The probe found it only by command line.
- It ran until about 58 to 60 s and wrote the file.
- No late item and no extra turn followed.

**Run 3: stdin closed right after `turn/completed`** (`codex-background-stdin-close.jsonl`). The app-server exited within 0.5 s and the command died; the marker file was never written.

The first two attempts were refused by `gpt-6-luna` ("I can't start an unverified background process that writes a file"). Neutral wording got it to comply.

Process-tree caveats for M5a quiescence on Windows:

- **Long-lived children:**
  - MCP servers (`rea` from the user config) live as long as the app-server.
  - `codex-code-mode-host.exe` starts lazily, on the first command, and also stays. Neither must block "quiet".
  - `codex-command-runner-*.exe` comes and goes with commands.
- **Commands run as another user.** They go through `codex-command-runner` in the elevated Windows sandbox, under a separate sandbox user (git warned that it could not read the user's global config).
- **A parent walk misses detached work.** `Start-Process` (and anything else whose parent exits) is invisible to a `ParentProcessId` walk.
- **PID reuse fools a naive walk.** One walk attributed `claude.exe daemon` processes to Codex's tree, through a `conhost.exe` that had reused a dead parent's PID. The probe only gave correct trees once it required each parent to be created before its child.
- **Conclusion:** quiescence has to come from a containment boundary (a Job Object, as in the M3 writer containment), not from parent walks.

### Transcripts

These files are sanitized the same way as the Claude ones. In addition, `serverName` (the host name) and `installationId` are redacted, and delta notifications are trimmed to three per method per turn. The first line of each file records the launch command, model, policy, sandbox and effort.

| File | Content |
| --- | --- |
| `codex-decline-accept-interrupt.jsonl` | `untrusted`/`workspace-write`: command declined (no file), accepted (file written), `turn/interrupt` during `Start-Sleep 40` (`interrupted`, command still alive), then a normal turn |
| `codex-resume.jsonl` | `thread/resume` of that thread recalling "allowed" |
| `codex-background-exec.jsonl` | background via Codex's exec session: `turn/completed` then a late `item/completed`, no extra turn |
| `codex-background-start-process.jsonl` | background via `Start-Process`: detached process outside the tree, no late item |
| `codex-background-stdin-close.jsonl` | stdin closed after `turn/completed`: server exits and the command dies |
| `codex-interrupt-then-close.jsonl` | interrupt, then stdin close: the interrupted command dies with the server |
| `codex-policy-on-request-workspace-write.jsonl` | no approvals at all (includes the 74 s first-run sandbox pause) |
| `codex-policy-on-request-read-only.jsonl` | only the file change asks; the write command fails in the sandbox |
| `codex-policy-untrusted-workspace-write.jsonl` | every command and the file change ask |
| `codex-file-change-decline.jsonl` | `on-request`/`read-only`: `apply_patch` file change declined, no file written |

### Not covered

These items were not exercised for Codex:
- `turn/steer`;
- `item/tool/requestUserInput`;
- `item/permissions/requestApproval`;
- image inputs, `turn/plan/updated`, `cancel` and `acceptForSession` decisions;
- `thread/start` options that drop the user's MCP servers;
- the `approvalsReviewer` alternatives.

## Unresolved questions

- Are project `.claude/settings.json` allow rules honored for a trusted project outside `%TEMP%\claude`? This was not provable from the scratch location.
- Does `interrupt` (or `TaskStop`) end a running background task, or only the foreground turn?
- Should M5a show the autonomous follow-up turn to the user as its own turn, or fold it into the previous one?
- For Codex, should M5a drop the user's MCP servers and config (for example with `-c` overrides or a separate `CODEX_HOME`), as `--strict-mcp-config` does for Claude?
- How should the Codex driver report a command that keeps running after `turn/completed`? It shows up as an open `commandExecution` item that completes later.
