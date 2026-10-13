---
title: "Turn on a detected agent with a connection test, and guide installs of missing ones"
description: "Selecting a detected or listed agent saves it and runs one ACP connection test (initialize and sign-in). Agents that are not installed get an Install guide button that opens the official install page in the browser."
status: in-review
priority: P2
effort: "1.5-2 engineer-days"
branch: feat/cap-style-studio-ui
tags: [desktop, acp, agent, setup]
blockedBy: []
blocks: []
created: 2026-10-12
---

# Agent turn-on and install guide

Follow-up to the completed [ACP agent auto-detection](../261012-1213-acp-agent-autodetect/plan.md). That plan is a delivered record; this plan changes one of its constraints (see below) instead of editing it.

## Outcome

In Setup, after **Detect agents**:

- **Agent installed:** the user presses **Select**. The app saves the agent and runs one connection test right away: the existing Check adapter probe, which starts the adapter in a scratch folder, runs ACP `initialize` and checks sign-in. The row then shows Ready, Sign-in needed, or the failure with its next step.
- **Agent not installed:** the row shows **Install guide**. It opens the agent's official install page in the default browser, and the row says "After installing, press Detect agents again."

## Decisions (validated with the user, 2026-10-12)

1. "Turn on" means both the Detect row's **Select** and the provider list's **Select**. Both save, then run one connection test.
2. After **Install adapter** succeeds, the app turns the adapter on by itself: it selects it and runs the test.
3. If the test fails on sign-in, the selection stays saved. The row shows "Sign-in needed" with the provider's sign-in step, and the user signs in and presses Check adapter again.

## Constraints and non-goals

- The prior rule "launching still requires the user's selection, then Check adapter or Send" becomes: **an explicit Select or Install adapter click starts at most one connection test. Project open and Detect agents still never start anything.**
- The plain **Save** button does not test. It keeps today's "Use Check adapter to verify them."
- No test starts while a task runs or settings are locked. The test is skipped with a reason, and the selection is still saved when the save itself was allowed.
- Install guide URLs are built into the code for the built-in profiles only, verified on the web and dated. The app opens only `https://` URLs. Custom profiles get no guide button.
- Non-goals: automatic re-detection after an install from the website, installing agent CLIs from inside Studio, and any OAuth or sign-in flow inside Studio.

## Phases

| # | Phase | Status |
|---|-------|--------|
| 1 | [Turn on runs the connection test](phase-01-turn-on-connection-test.md) | done |
| 2 | [Install guide links](phase-02-install-guide-links.md) | done |
| 3 | [Docs, contract text and verification](phase-03-docs-and-verification.md) | done except the manual GUI check |

Phases 1 and 2 both edit the Setup row in `conversation_panel.rs`, so they run in order, not in parallel.

## Acceptance criteria

1. With a detected Ready agent, one click on **Select** saves it, and the Status section shows "Checking the adapter…" followed by a checked result, with no second click.
2. **Select** in the provider list does the same for a profile that is not already selected.
3. **Save** alone never starts the adapter. Project open and **Detect agents** never start it.
4. Once **Install adapter** succeeds, the app selects the new adapter and runs one test without another click.
5. A sign-in failure leaves the agent selected, and the row shows the sign-in next step.
6. A Not found agent (Claude, Codex, Pi, and Antigravity if its page is verified) shows **Install guide**, which opens the recorded `https://` page. A non-`https` URL is refused, and a unit test covers that.
7. A test is never fired while a task runs. The skip is reported.
8. `just clippy`, `cargo test -p fframes-studio --test provider_profiles` and the `controls` unit tests pass.

## Risks

- **Save ordering.** `set_adapter` writes the settings under the lock and then sends `SettingsChanged` over the same channel `check_adapter` uses. A check fired after the save completes therefore sees the new adapter. Phase 1 adds a test that confirms this.
- **Selection mismatch.** The Detect row's Select writes the adapter file but not `registry.selected_profile_id`, so the provider list can show a different selection. Phase 1 tracks which row was turned on, in the session only, and does not fix this mismatch. It is listed as an open question.
- **Stale URLs.** Vendor docs move. The URLs live in one function with a dated comment, so they are cheap to update.

Rollback: revert the phase commits. Select goes back to saving only, and the guide button disappears.

## Open questions

- Should the Detect row's **Select** also set `registry.selected_profile_id`, so both lists agree on which agent is on? It is left out of scope unless the user asks.
