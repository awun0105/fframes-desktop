---
title: "First-run onboarding: SDK, theme, agent and permission policy"
description: "When Studio opens for the first time it walks the user through installing or choosing the SDK, picking Light/Dark/System, turning on an ACP agent, and choosing how agent permission requests are answered, so the first prompt runs without stops."
status: in-review
priority: P2
effort: "5-7 engineer-days"
branch: main
tags: [desktop, onboarding, sdk, acp, agent, permissions, feature]
blockedBy: [261019-1437-studio-layout-a]
blocks: []
created: 2026-10-20
---

# First-run onboarding

## Outcome

On the first launch, Studio shows a short setup flow instead of the empty workspace:

```diagram
┌─────────┐   ┌────────────┐   ┌─────────┐   ┌─────────┐   ┌──────────────┐   ┌─────────────┐
│ Welcome │──▶│ Appearance │──▶│   SDK   │──▶│  Agent  │──▶│ Permissions  │──▶│ First video │
└─────────┘   │ System/    │   │ install │   │ detect, │   │ ask / allow  │   │ create/open │
              │ Light/Dark │   │ import  │   │ install,│   │ working copy │   └─────────────┘
              └────────────┘   │ select  │   │ select, │   │ / allow all  │
                               └─────────┘   │ check   │   └──────────────┘
                                             └─────────┘
```

- Every step saves when it is left and can be skipped. Skipping an important step (SDK, agent)
  says what will not work, and where to finish it later (Project drawer, Agent settings).
- With the "allow in the working copy" policy, the agent edits the project's draft without
  stopping to ask. Studio answers those requests itself and records each answer in the
  transcript. Shell commands and network access still ask unless the user picked "Allow
  everything".
- The flow never opens again on its own after it is finished or skipped. **Settings → Run
  setup again** reopens it. Existing installs that already have an SDK and a selected agent
  are marked as onboarded and never see it.

## Key decision to confirm: auto-answering permissions

This reverses a documented rule. [`docs/desktop/README.md`](../../docs/desktop/README.md)
says "Studio never answers one for you", and [`research.md`](../../docs/desktop/research.md)
says that Studio should expose the agent's real access mode and honor user decisions, and
that worktree isolation does not confine filesystem or network access. The Claude launch
also refuses `bypassPermissions` on purpose
(`studio-harness/src/driver/runtime/claude/launch.rs`).

The plan keeps the intent of that rule, which is that the user decides. The choice moves to
onboarding as a standing policy, and every automatic answer is visible:

| Policy | Answers automatically | Still asks |
|---|---|---|
| Ask every time | nothing (today's behaviour) | everything |
| Allow in the working copy (preselected) | read, search, edit, delete, move and think, when every location is inside the task's draft | execute, fetch, other, any path outside the draft, unknown kind |
| Allow everything | every request, choosing the agent's `allow_once` option | nothing (Stop still works) |

The policy only picks an option the agent offered. It never invents an option, never answers
a question (`QuestionPrompt`) and never picks `allow_always`, so the agent's own stored rules
stay untouched. Edits are still drafts: validation, Review and Undo apply as they do today.

**Open question for the owner:** Is "Allow in the working copy" the right preselection? Or
should onboarding preselect "Allow everything", which runs shell commands such as
`cargo check` without asking but is the smoothest? See the end of this file.

## Constraints and non-goals

- No new crates. App-wide preferences stay in `app-settings.json` (`studio-engine/src/app_settings.rs`).
- Existing flows are reused as they are: SDK install/import/select (`Command::InstallSdk`, `Picker::SdkBundle`,
  `Picker::Sdk`), agent Detect/Select/Install adapter/Check adapter (`agent/provider_setup.rs`), and the theme
  write (`shell/settings.rs`). Onboarding is a new front for them, not a second implementation.
- Opening the app or the onboarding never starts an agent. Only Select or Install adapter runs
  the one connection test, as today.
- Non-goals: per-project permission policies, sign-in inside Studio (OAuth), installing agent
  CLIs, choosing ACP session modes (`acceptEdits`, `bypassPermissions`) for the user, and a guided
  tour of the editor.
- Builds on the uncommitted layout A (`plans/261019-1437-studio-layout-a`): the rail, the right
  agent column and the empty state it introduces. Land that first.

## Phases

| # | Phase | Status | Depends on |
|---|-------|--------|------------|
| 1 | [Onboarding state and permission policy in app settings](phase-01-settings-state.md) | done | – |
| 2 | [Permission policy in the engine](phase-02-permission-policy-engine.md) | done | 1 |
| 3 | [Onboarding view, Welcome, Appearance and First video](phase-03-onboarding-view.md) | done | 1 |
| 4 | [SDK and Agent steps](phase-04-sdk-and-agent-steps.md) | done | 3 |
| 5 | [Permissions step, settings, panel indicator, docs and verification](phase-05-permissions-step-docs-verification.md) | done (manual real-agent check open) | 2, 4 |

Phases 2 and 3 touch different crates (`studio-harness`/`studio-engine` vs. `studio-ui`) and can
run in parallel after phase 1.

## Acceptance criteria

- A fresh app-data folder opens the onboarding. Finishing it or pressing Skip all writes
  `onboarding.completed`, and the next launch opens the workspace.
- An app-data folder that already has a managed SDK and a selected agent profile, but no
  `onboarding` key, opens the workspace and is marked as onboarded.
- The theme step changes the window theme on click and persists it, exactly like Settings.
- The SDK step installs the managed SDK with progress, imports a bundle, or selects an
  installed SDK. A failure shows the existing error with Retry.
- The agent step lists detected agents with Ready, Install adapter, Install guide or Sign-in
  needed, and Select runs one connection test.
- Under "Allow in the working copy", a scripted-agent `edit` request inside the draft is
  answered with `allow_once` without a card. An `execute` request and an edit outside the draft
  still show the pinned card. The transcript row says "Allowed automatically · Allow in the
  working copy".
- Under "Ask every time", behaviour is unchanged (existing parity tests pass unchanged).
- `just clippy`-equivalent for the desktop workspace, `cargo test -p studio-engine -p studio-harness -p studio-ui`
  (parity included) pass on Windows. Inspected light and dark captures exist for every step.

## Risks

- **Security:** "Allow everything" runs agent shell commands on the host. A job object keeps
  the process tree contained, but it is not a sandbox. Mitigation: it is never preselected, the
  wording says so plainly, and the agent header shows the active policy.
- **Missing tool kind:** some wires may send no kind or no locations. Unknown always means ask,
  so a missing field can only cost smoothness, never safety.
- **Path checks:** locations are canonicalized and compared with the canonical draft root;
  symlinks or `..` that escape the draft count as outside.
- **Existing users:** a wrong migration rule would nag them. The rule lives in one function with
  unit tests (phase 1).

## Open questions

1. Default preselection on the Permissions step: "Allow in the working copy" (recommended) or
   "Allow everything"?
2. Should the "Allow everything" choice expire per project open (asked again), or persist like the
   others? The plan assumes it persists.

## Implementation notes (2026-10-20)

Built as planned with these deviations:

- **Step order:** Welcome, Appearance, SDK, First video, Agent, Permissions. The agent is set up
  through an open project's workflow, so the project step comes before it. Without a project the
  Agent step says so; the choice made there is saved for every project.
- **Preselection:** the owner did not answer the open question, so a first setup preselects
  "Allow in the working copy" and saves it only on Finish. Skip keeps "Ask every time".
- **Record:** pp-settings.json gains onboarding_completed: Option<bool> (not a struct with
  skipped steps) and permission_policy.
- **Settings:** the Permissions choice sits below the adapter setup on the Agent page; **Run
  setup again** is on the Appearance page.
- **Not built:** Enter/Escape shortcuts for Next/Back, a clickable header badge (it only shows the
  policy), changes to the unused setup_view.rs.
- **Open:** the end-to-end check with a real Claude adapter (phase 5) was not run.