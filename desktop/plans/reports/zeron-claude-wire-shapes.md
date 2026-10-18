# Zeron Claude Code stream-json wire shapes

Source: C:\Users\Administrator\code\zeron\crates\harness (rev 09beace0). Paths below are relative to that `crates\harness`.
Abbreviations: W = src/claude/wire.rs, M = src/claude/mod.rs, N = src/claude/normalize.rs, O = src/process/owned.rs, F = tests/fixtures/fake-claude.sh, T = tests/claude.rs.
Everything is line-delimited JSON; each stdin line is the JSON followed by a newline (M:771-791 `stdin_writer`).

## 1. `initialize` control request and response parsing

Run-time request, sent as the first stdin line of a normal run (not for title runs), before the prompt (M:645-649):
```rust
// W:373-380
json!({
    "type": "control_request",
    "request_id": request_id,            // M:647 passes "zeron_initialize"
    "request": { "subtype": "initialize", "perTaskStopAffordance": true },
})
```
Doc comment (W:368-372): `perTaskStopAffordance` tells the CLI the host stops tasks individually, so `interrupt` ends the turn and spares background agents (verified live on 2.1.285; background shells survive either way).

The live run does NOT parse the response: the `control_response` is consumed and dropped in the run loop (M:903-908), only to match `stopping` request ids (stop settle timer). `ControlResponseFrame` reads only `response.request_id` (W:28-38).

Separate probe child (discovery only, no model turn; M:310-385): spawned with `--print --input-format stream-json --output-format stream-json --verbose`, stderr null, sends
```json
{"type":"control_request","request_id":"zeron-command-probe","request":{"subtype":"initialize"}}
```
(M:344-348, no `perTaskStopAffordance`), reads stdout lines until a `type:"control_response"` whose `response.request_id == "zeron-command-probe"` (M:359-365). If `response.subtype == "error"` -> error from `response.error` (string) or "initialize control request failed" (M:366-372). Otherwise returns the `response` object (the whole envelope). 10 s timeout (M:379), then child shut down. Result cached 120 s keyed by credential/executable identity (src/claude/discovery.rs:36-49).

Fields read from the response envelope (envelope = `{"type":"control_response","response":{"subtype":"success","request_id":..,"response":{...payload}}}`; F:19, F:396):
- commands: `response.response.commands[]` -> `name` (trimmed, empty dropped), `description` (default ""), `argumentHint` (trimmed, empty -> None) (M:388-419).
- models: `response.response.models[]` (src/claude/catalog.rs:228-231; empty/missing = error "Claude returned an empty model catalog"): per entry `resolvedModel` else `value` (catalog.rs:246; bare aliases `default|opus|sonnet|haiku|fable`, optionally with `[1m]`, are skipped, 250-255), `value == "default"` marks the default (262), `supportedEffortLevels[]` strings low/medium/high/xhigh/max (269-287), `displayName` (293), `description` (294).
- Fake CLI replies with an empty payload for runs: `{"type":"control_response","response":{"subtype":"success","request_id":"zeron_initialize","response":{}}}` (F:19).

No other initialize fields (account, output_style, agents...) are read.

## 2. Prompt user lines

Opening prompt (W:322-335), `uuid` = `Uuid::new_v4()` string (M:658):
```json
{"type":"user","uuid":"<id>","message":{"role":"user","content":"<text>"},"parent_tool_use_id":null}
```
With images, `content` becomes an array: image blocks first, then one text block (W:350-366):
```rust
fn content_blocks(text: &str, images: &[ImageBlock]) -> Vec<Value> {
    let mut blocks: Vec<Value> = images.iter().map(|img| json!({
        "type": "image",
        "source": { "type": "base64", "media_type": img.media_type, "data": img.data },
    })).collect();
    blocks.push(json!({ "type": "text", "text": text }));
    blocks
}
```
i.e. `{"type":"image","source":{"type":"base64","media_type":"image/png","data":"<raw base64, no data: prefix>"}}` then `{"type":"text","text":"<prompt>"}`. Empty `images` -> plain string content. (A test-only `user_message_line` without `uuid` exists, W:265-275.)

Image loading (M:696-767, `load_image_blocks`): best-effort per attachment path; unreadable, oversized or unsupported files are skipped (the path ref still rides the prompt text), never fatal.
- Size cap: `const MAX_INLINE_IMAGE_BYTES: u64 = 5 * 1024 * 1024;` raw bytes (M:698, check M:753). No count cap, no dimension cap.
- media types: by extension png/jpg/jpeg/gif/webp (M:704-715), else magic bytes PNG `89 50 4E 47`, JPEG `FF D8 FF`, GIF `GIF8`, WEBP `RIFF....WEBP` (M:716-735); anything else (svg/bmp/tiff) is skipped.
- Standard base64 (`base64::engine::general_purpose::STANDARD`, M:763).

Ultrathink: prompt text prefixed `"Ultrathink:\n"` unless it starts with a slash command (src/claude/catalog.rs:15-25); applied to the opening prompt (M:660) and every steer (M:1083).
Launch uses `--replay-user-messages` (M:222), so user lines are echoed back as `user` frames.

## 3. Steering

Steer line (W:291-305; uuid minted per steer, M:1080):
```rust
serde_json::json!({"type":"user", "uuid":id, "priority": if immediate { "now" } else { "next" },
    "message":{"role":"user","content":content}, "parent_tool_use_id":null})
```
`content` = string, or the image+text block array (same as section 2).

Priority policy (M:1085 passes `open_tools.is_empty()` as `immediate`; doc W:277-284):
- `"now"` when no top-level tool call is in flight: streaming text/thinking stops at once, steer answered next (verified CLI 2.1.280).
- `"next"` when any tool is open, because `now` aborts an in-flight MCP tool call ("The tool call was interrupted before a result was received"); with `next` the tool finishes and the steer lands right after its result in the same turn.
- No other values used. `open_tools` = ids from `AgentEvent::ToolCall` inserted, removed on `ToolResult`, cleared on `Done` (M:989-998); only top-level calls count (subagent events are wrapped in `Subagent` and do not match).

Settings changes before a steer: `set_model` / `apply_flag_settings` control requests are written first on the same stdin (M:1062-1079):
```json
{"type":"control_request","request_id":"zeron_model_N","request":{"subtype":"set_model","model":"<string|null>"}}
{"type":"control_request","request_id":"zeron_settings_N","request":{"subtype":"apply_flag_settings","settings":{"effortLevel":"..","fastMode":true,"alwaysThinkingEnabled":true,"ultracode":true}}}
```
(W:383-400, M:126-134; each of fastMode/alwaysThinkingEnabled/ultracode is `true` or `null`; `null` drops the override back to user settings, verified 2.1.286.)

Confirmation tracking: `command_lifecycle` frames `{"type":"command_lifecycle","command_uuid":..,"state":"queued|started|completed|cancelled|canceled|failed"}` (W:40-52, M:1271-1273), emitted for every stdin user line carrying a `uuid` (verified 2.1.286). The replay of the user frame (`--replay-user-messages`; top-level `user` frame whose `uuid` matches a pending steer) is the fallback confirmation (M:939-945). A `result` while a written/queued command is waiting is treated as a steer boundary, held rather than ending the turn (M:1004-1026, `Commands` M:1239-1318; settle timers UNREPORTED 5 s, UNACKNOWLEDGED 30 s, M:1261/1265). Mailbox closed -> stdin closed (`StdinMsg::Close`, M:1092-1097).

## 4. `can_use_tool` control requests

Request read (W:227-244): `{"type":"control_request","request_id":..,"request":{"subtype":"can_use_tool","tool_name":..,"input":{...}}}` -> fields `request_id`, `request.subtype`, `request.tool_name`, `request.input`. Fixture: F:102.
Launch flag enabling it: `--permission-prompt-tool stdio` (M:230-231; "undocumented, absent from `claude --help`").

Handling (M:1333-1370): other subtypes are logged and NOT answered (M:1338-1344). Any tool except `AskUserQuestion` is auto-allowed, echoing the input back:
```rust
// W:413-429
pub(crate) fn control_response_line(request_id: &str, response: Value) -> String {
    json!({ "type": "control_response",
        "response": { "subtype": "success", "request_id": request_id, "response": response } }).to_string()
}
pub(crate) fn allow_response(updated_input: Value) -> Value {
    json!({ "behavior": "allow", "updatedInput": updated_input })
}
```
Wire result:
```json
{"type":"control_response","response":{"subtype":"success","request_id":"cr-0","response":{"behavior":"allow","updatedInput":{"command":"ls"}}}}
```
Deny: the ONLY deny in Zeron is title-only runs (M:915-919):
```rust
control_response_line(&req.request_id, serde_json::json!({
    "behavior": "deny", "message": "Tools are disabled for title generation"
}))
```
No `interrupt` field is ever sent in an allow/deny response; no `message` on allow; no `updatedPermissions`/`toolUseID` handling. Zeron never denies in normal runs (comment M:1326-1328: unattended parity; the CLI blocks until some response arrives). The `auto_approve` flag only changes launch flags (`--permission-mode bypassPermissions --dangerously-skip-permissions`, else `--permission-mode default`, M:242-253); `can_use_tool` is auto-allowed in both modes. `control_cancel_request` maps to `Frame::Other` and is not handled (W:24-25,260). No `control_response` of subtype `error` is ever written.

## 5. `AskUserQuestion`

Input parse (M:1372-1412, verbatim):
```rust
fn parse_questions(input: &Value) -> Vec<UserInputQuestion> {
    let raw = input.get("questions").and_then(Value::as_array);
    raw.map(|a| a.as_slice()).unwrap_or_default().iter().map(|q| {
        let field = |keys: [&str; 2]| keys.iter().find_map(|k| q.get(*k).and_then(Value::as_str));
        UserInputQuestion {
            id: uuid::Uuid::new_v4().to_string(),
            header: field(["header", "title"]).unwrap_or("Question").into(),
            question: field(["question", "prompt"]).unwrap_or("").into(),
            prefill: None,
            multiline: false,
            multi_select: ["multiSelect", "multi_select"].iter()
                .find_map(|k| q.get(*k).and_then(Value::as_bool)).unwrap_or(false),
            options: q.get("options").and_then(Value::as_array).map(|a| a.as_slice())
                .unwrap_or_default().iter().map(|op| match op {
                    Value::String(s) => s.clone(),
                    other => other.get("label").or_else(|| other.get("value"))
                        .and_then(Value::as_str).unwrap_or("").into(),
                }).collect(),
        }
    }).collect()
}
```
(Formatting condensed from the source; logic unchanged.) Fixture input (F:112): `{"questions":[{"header":"Choice","question":"Pick one","options":["A","B"],"multiSelect":false}]}`; the unit test also covers `{"label":"B","description":"second"}` option objects (M:1455). Option descriptions are dropped. The question id is a locally minted uuid, not from the wire.

Flow (M:1350-1369): runs in a spawned task so the frame loop keeps flowing; `request_input(questions)` returns a oneshot of `Vec<UserInputAnswer{question_id, labels: Vec<String>}>`; a dropped sender degrades to empty answers (`unwrap_or_default`) so the agent is unblocked.

Answer shape (M:1414-1441, verbatim):
```rust
fn updated_input_with_answers(input: &Value, questions: &[UserInputQuestion], answers: &[UserInputAnswer]) -> Value {
    let mut updated = match input { Value::Object(map) => map.clone(), _ => serde_json::Map::new() };
    let mut by_question = serde_json::Map::new();
    for q in questions {
        let labels: Vec<String> = answers.iter().find(|a| a.question_id == q.id)
            .map(|a| a.labels.clone()).unwrap_or_default();
        let value = if q.multi_select {
            Value::Array(labels.into_iter().map(Value::String).collect())
        } else {
            Value::String(labels.into_iter().next().unwrap_or_default())
        };
        by_question.insert(q.question.clone(), value);
    }
    updated.insert("answers".into(), Value::Object(by_question));
    Value::Object(updated)
}
```
Wire result: an allow response with the ORIGINAL input preserved and `answers` added, keyed by question TEXT; string for single-select, array of strings for multi-select:
```json
{"type":"control_response","response":{"subtype":"success","request_id":"cr-1","response":{"behavior":"allow","updatedInput":{"questions":[ ...original... ],"answers":{"Pick one":"B"}}}}}
```
(test T:284-342; fixture F:112-128 checks `"Pick one":"B"`). Free-text / "Other" answers get no special treatment: whatever label strings the host returns are placed as-is. Zeron does not emit its own InputRequested events; the host's request_input bridge owns that (M:1356-1364). Not handled: `annotations`, `preview`, or other question fields.

## 6. `interrupt`

Two distinct shapes:
1. Full teardown (the run's `interrupt` cancellation token, M:1148-1168; W:431-439):
```json
{"type":"control_request","request_id":"int_1","request":{"subtype":"interrupt"}}
```
then sets `interrupted`, snapshots descendants (unix), and escalates: after `interrupt_grace` (2 s default, M:161) SIGTERM, after `kill_grace` (3 s) SIGKILL (M:1160-1167). Once a `Done` arrives the loop breaks; if none arrives a synthetic `Done{Interrupted}` is emitted (M:1203-1211); the child is shut down (M:1225). The fixture wedges without reading stdin to force escalation (F:379-383).
2. Turn stop, process stays alive (`TurnControl::stop_turn`, M:1104-1125; W:402-411):
```json
{"type":"control_request","request_id":"zeron_stop_N","request":{"subtype":"interrupt","cancel_queued":true}}
```
`cancel_queued` drops steers queued but unconsumed. The CLI answers `{"type":"control_response","response":{"subtype":"success","request_id":..,"response":{"still_queued":[]}}}` (F:361), which starts a 3 s `STOP_SETTLE` (M:852,903-906); if no turn-ending `result` follows (stop between turns), a synthetic `Done{Interrupted}` is sent (M:1129-1146). The post-stop `result` is `subtype:"error_during_execution"` with `errors:["[ede_diagnostic] ..."]` (F:363); a late duplicate result within the window and with no new activity is dropped (M:976-987). The CLI also emits a user frame `[Request interrupted by user]` (F:362; filtered as synthetic, N:173-176).

## 7. Frame normalisation

Parser (W:246-263): `type` in system, stream_event, assistant, user, rate_limit_event, result, control_request, control_response, command_lifecycle; anything else is `Other`; a non-JSON line is an Err (logged and skipped, M:896-902). All struct fields are `#[serde(default)]`.

`stream_event` (W:87-111, N:428-469): only `event.type == "content_block_delta"` is processed, others dropped (N:429-431). Top-level `parent_tool_use_id` (nullable). `event.delta.type`: `text_delta` -> `TextDelta{text}` (field `text`); `thinking_delta` -> `ReasoningDelta{text}` (field `thinking`; empty thinking = redacted heartbeat shape, still forwarded); `input_json_delta` -> empty `ReasoningDelta` liveness heartbeat (N:464-466); other delta types dropped. `content_block_start` (tool_use) is NOT handled: tool calls come only from the full `assistant` frame. With `parent_tool_use_id` set, text_delta and non-empty thinking_delta are wrapped in a `Subagent` tag and the rest dropped (N:432-451). `--include-partial-messages` is passed (M:221). Note (N:474-478): 2.1.x streams no tagged partial deltas for subagents.

`assistant` (W:113-165, N:471-605): `message.content` array blocks parsed into `ContentBlock{type,text,id,name,input,tool_use_id,is_error}`; also `message.model`, `message.usage`, `uuid`, `error`, `parent_tool_use_id`. For top-level frames only `tool_use` blocks are emitted, as `ToolCall{id, call: decode_tool_use(name,input)}` (N:516-568); `text` and `thinking` blocks of top-level assistant frames are ignored (text arrives via stream deltas; no fallback if partial messages are off). Subagent-tagged assistant frames: `text` (non-empty, emitted as `text.trim_end() + "\n\n"`) and `tool_use` are emitted tagged (N:479-498). `message.usage` -> `ContextUsage{tokens = input_tokens + cache_read_input_tokens + cache_creation_input_tokens, window None}` (N:570-589). `error` code -> `Error` text (N:13-27: authentication_failed, oauth_org_not_allowed, billing_error, rate_limit, overloaded, invalid_request, model_not_found, server_error, max_output_tokens, unknown). Each top-level assistant frame emits `AssistantMessageCompleted` and rotates the message id (N:598-603).

Tool decoding (N:71-144): Bash{command}; Read{file_path}; Write{file_path,content}; Edit{file_path,old_string,new_string}; Grep{pattern,path}; Glob{pattern}; WebFetch{url,prompt}; WebSearch{query}; `Agent`|`Task` -> Unknown named `Agent: <input.description>` (else "Agent") with raw input; `mcp__<server>__<tool>` -> Mcp{server,tool,input}; else Unknown{name,input}.

`user` (N:607-655): only `tool_result` blocks are read, emitting `ToolResult{id: tool_use_id, is_error: is_error.unwrap_or(false), output: None, diff: None}`. The tool result `content` is NOT read (output always None). Tagged (subagent) user frames also surface non-empty `text` blocks as tagged `UserMessage` unless synthetic (`<system-reminder>` or `[Request interrupted` prefix, N:173-176). Untagged user frames' text is not emitted as a UserMessage; the top-level user frame is used only for steer replay confirmation by `uuid` (M:939-945).

`TodoWrite` (N:102-116): `input.todos[]` -> per item `content` (string) and `status` -> `TodoStatus::parse` (proto/src/agent.rs:433-439: `completed|complete|done` -> Completed; `in_progress|inProgress|in-progress|active` -> InProgress; anything else or missing -> Pending). `activeForm` and `id` are not read. Item = `TodoItem{text, done, status}`.

`parent_tool_use_id` grouping: any assistant/user/stream_event with a non-null `parent_tool_use_id` becomes `AgentEvent::Subagent{parent_tool_use_id, event}`, never merged into the parent feed (N:420-428; M:23-27). Parent ids are re-keyed through `agent_tool_spawns` for resumed tasks (N:433).

`result` (W:203-225, N:673-771): fields read `subtype`, `result` (string), `errors[]` (strings, or other JSON via to_string), `usage.input_tokens`, `usage.output_tokens`, `session_id`, `modelUsage` map (for `contextWindow`, matched to the last assistant model or `canonicalModel`, N:674-693). Emits `ContextUsage{window}` (if >0), `Usage{input,output}`, then `Done`. `subtype == "success"` -> Completed (Interrupted if interrupted/stopping) with the `result` text; otherwise Errored with `errors.join("; ")` after dropping `[ede_diagnostic]` breadcrumbs (N:58-60,716-723); known subtypes error_max_turns / error_max_budget_usd / error_max_structured_output_retries map to fixed wording, default fallback "The run ended with an error." (N:43-50). `is_error`, `stop_reason` and `terminal_reason` are NOT read from the result frame (`terminal_reason`/`is_error`/`num_turns` appear in fixtures, F:162,171,313, but only `subtype` drives status). `total_cost_usd` ignored; cache tokens in result usage not read.

`rate_limit_event` (W:167-201, N:660-671): `rate_limit_info.status == "rejected"` and not overage-carried (`overageStatus` allowed/allowed_warning, `isUsingOverage`, `overageInUse`) -> `Error`; `rateLimitType` five_hour/seven_day/seven_day_opus/seven_day_sonnet/overage -> label.

`system` (W:54-85, N:340-418): fields read `subtype, model, tools[], cwd, session_id, tool_use_id (alias toolUseId), status, task_id (alias taskId), subagent_type, tasks[]`.
- `init`: first one only (`saw_init`) -> `SessionStarted{model,tools,cwd,session_id}`; re-emitted inits (wake turns) are deduped (N:405-417). Other subtypes (e.g. `thinking_tokens`, `session_state_changed`) are tolerated and dropped (F:65-66).
- `task_started` with `subagent_type` present plus non-empty `task_id` and `tool_use_id`: records agent task -> spawning tool_use id (N:389-404); without `subagent_type` (background shell) ignored.
- `task_notification`: parent = known `task_id` mapping, else `tool_use_id`; only if the parent is a known Agent/Task spawn; `status` completed|complete|succeeded|success -> Completed, failed|errored|error -> Errored, killed|cancelled|canceled|stopped|interrupted -> Interrupted, else ignored; emits a tagged `Done` (N:347-385).
- `background_tasks_changed`: `tasks.len()` forwarded to `turn.set_background(n)` (M:909-913); the array is the authoritative set, not a delta (W:81-84). Fixture: `{"type":"system","subtype":"background_tasks_changed","tasks":[{"task_id":"bg-1","task_type":"local_agent","description":"scout"}]}` (F:355). Task entry fields are not parsed.
- No `task_progress`/`task_updated` handling. Resume: `Normalizer::for_resume` scans `<CLAUDE_CONFIG_DIR or ~/.claude>/projects/*/<session_id>.jsonl` to restore spawn ids (N:232-327, M:606-614).

## 8. Launch flags, env hygiene, mcp/settings, resume

Base args (M:213-234): `--print --input-format stream-json --output-format stream-json --verbose --include-partial-messages --replay-user-messages --thinking-display summarized --permission-prompt-tool stdio`.
Then:
- `--model <id>`, suffix `[1m]` when `model_options.contextWindow == "1m"` (M:106-120,235-238).
- `--effort <low|medium|high|xhigh|max>`; xhigh clamps to max unless the model matches fable-5/opus-4-7+/opus-5+/sonnet-5+ (catalog.rs:34-64); ultrathink = no flag (prompt prefix instead).
- `--permission-mode bypassPermissions --dangerously-skip-permissions` when `auto_approve`, else `--permission-mode default` (M:242-253).
- `--settings <inline JSON string>` (not a file): subset of `{"fastMode":true,"alwaysThinkingEnabled":true,"ultracode":true}`, only keys that are on, omitted if empty (M:254-267).
- `--mcp-config <inline JSON string>` (the CLI accepts JSON or a path; M:91-104,600-605): `{"mcpServers":{"<name>":{"command":..,"args":[..],"env":{..}}}}`; no `--strict-mcp-config` for normal runs (merges with the user's own servers).
- Title runs add `--system-prompt <TITLE_INSTRUCTIONS> --tools "" --strict-mcp-config --mcp-config {"mcpServers":{}} --setting-sources ""` (M:589-599); no initialize request, no resume, every permission request denied.

Resume: `--resume=<session_id>` as a single arg, `format!("--resume={resume}")` (M:275-277). `--session-id` is NOT used anywhere in the claude module (grep empty).
cwd: `current_dir(request.cwd)` when non-empty (M:278-280); stdio piped, `kill_on_drop(true)` (M:281-284). PATH via `crate::compose_child_path` (M:273).
Executable resolution (M:73-89,188-207): `with_executable` override, `CLAUDE_CODE_EXECUTABLE` env, then PATH / login-shell PATH / `~/.claude/local/claude`, `~/.local/bin/claude`, `/opt/homebrew/bin/claude`, `/usr/local/bin/claude`.

Env hygiene: `owned::configure` (O:6-17) removes `CLAUDECODE`, `CLAUDE_CODE_ENTRYPOINT`, `CLAUDE_CODE_SSE_PORT`, `CLAUDE_AGENT_SDK_VERSION` and sets a unix process group. BUT it is called only from the Pi harness (src/pi/mod.rs:133) and is not referenced by the Claude harness (grep for `owned::` / `env_remove` across src finds only pi/mod.rs and owned.rs). The Claude `build_command` (M:271-286) removes NO env vars. The owned.rs unit test is named `acp_environment_removes_nested_claude_markers` (O:108). Nothing else (ANTHROPIC_API_KEY etc.) is touched. Port note: if fframes may itself run inside a Claude Code session, it must remove those four vars on its own.
Teardown: `crate::shutdown_agent(&mut child, torn_down, kill_grace)` (M:1225); a stderr tail is kept for the crash message (M:631-641,1218).

## 9. Version detection / gating

None on the Claude run path: no `--version` call gates flags, and the protocol is not version-gated ("Tolerant by construction", W:1-5; the `commands.reported` fallback covers CLIs without `command_lifecycle`, M:1243-1245). `executable::binary_version` (src/executable.rs:165-276, runs `<exe> --version`, `parse_version` e.g. "Claude Code v2.1.3 (native)", executable.rs:542-546) is used only to key the model/initialize caches (src/model_context.rs:84-91,126). Version notes appear only in comments (flags verified against CLI 2.1.228, 2.1.280, 2.1.285, 2.1.286; M:1-5, W:42-44,278-282,368-372).
