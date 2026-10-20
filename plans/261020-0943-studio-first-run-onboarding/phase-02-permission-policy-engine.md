---
phase: 2
title: "Permission policy in the engine"
status: pending
effort: "1.5-2 days"
depends_on: [1]
---

# Phase 2: Permission policy in the engine

## Objective

Studio answers a permission request itself when the user's policy covers it, and records that it
did. Anything the policy does not clearly cover goes to the user as today.

## Why the driver must change first

`PermissionPrompt` (`desktop/crates/studio-harness/src/driver/events.rs`) carries only
`title` and `options`, so the engine cannot tell an edit from a shell command. ACP's
`session/request_permission` sends a `toolCall` with `kind` (`read`, `edit`, `delete`, `move`,
`search`, `execute`, `think`, `fetch`, `other`) and `locations` (paths). The native wires name
the tool (`Edit`, `Write`, `Bash`, `WebFetch` and so on, and Codex `apply_patch`/`exec`).

## Changes

- `studio-harness/src/driver/events.rs`: add `kind: Option<ToolKind>` and `locations: Vec<PathBuf>`
  to `PermissionPrompt`, with `ToolKind` mirroring ACP's kinds.
- ACP driver (`studio-harness/src/driver/runtime/shared.rs`, where ACP `PermissionRequested` is built; the
  Claude and Codex ones are in `runtime/claude/connection.rs` and `runtime/codex/connection.rs`): fill both from `toolCall.kind` and
  `toolCall.locations[].path`. Missing or unknown values become `None` and an empty list.
- Claude stream-json and Codex app-server drivers: map the tool name to a kind
  (Read/Glob/Grep → read/search, Edit/Write/MultiEdit/NotebookEdit → edit, Bash → execute,
  WebFetch/WebSearch → fetch, anything else → other) and take paths from the tool input
  (`file_path`, `path`, `notebook_path`). Pin the mapping in one table with a unit test.
- `studio-engine/src/agent/model.rs`: `PermissionCard` gains `kind`, `locations` and
  `answered_by: Option<AutoAnswer>` (`AutoAnswer { policy: PermissionPolicy }`). `PermissionState`
  is unchanged.
- `studio-engine/src/agent/actor/prompt.rs` (where `PermissionRequested` becomes a card): before
  publishing the card as `Open`, call
  `fn auto_answer(policy, card, draft_root) -> Option<String /* option_id */>`:
  - `AskEveryTime` → `None`.
  - `AllowInWorkingCopy` → `Some` only when the kind is one of read/search/edit/delete/move/think,
    `locations` is non-empty, and every location canonicalizes inside the canonical draft root.
    A non-existent file is checked through its nearest existing parent.
  - `AllowEverything` → `Some` for any kind.
  - The chosen option is the first option whose kind is `allow_once`. If there is none, the
    result is `None` (ask). `allow_always` is never chosen.

  When `Some`, reply through the same path as `reply_permission` (so correlation, stale and
  duplicate checks still apply) and store the card already `Selected` with `answered_by` set.
  The policy is read from `AppSettings` when the task starts and frozen on the task like
  `review_policy`, so a settings change mid-task cannot race a reply.
- Questions (`QuestionPrompt`) never go through `auto_answer`.
- `studio-ui/src/agent/cards.rs` `permission_card`: an auto-answered row reads
  "Allowed automatically · <policy label>" followed by the title, in the neutral tone.

## Tests (keep)

- `auto_answer` unit table in `studio-engine`: inside/outside the draft, `..` escape, a symlink to
  outside (Unix-only case gated), empty locations, `execute` under each policy, no `allow_once`
  option, unknown kind.
- Extend the scripted ACP agent (`studio-engine/tests/support/acp-agent.py`): its `permission`
  spec accepts optional `kind` and `locations` (relative paths resolved against its cwd, the
  draft) and passes them in `toolCall`.
- `desktop/app/tests/agent_workflow/permissions.rs`: under `AllowInWorkingCopy`, an `edit` in the
  draft completes the turn without `WaitingPermission`, and `permission-perm-1.json` holds
  `allow`. An `execute` request still waits.
- The existing permission tests stay unchanged and pass (default policy is `AskEveryTime`).

## Done when

- `cargo test -p studio-harness -p studio-engine` and `desktop/app` agent_workflow tests pass on Windows.
