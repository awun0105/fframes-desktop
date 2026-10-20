---
phase: 4
title: "SDK and Agent steps"
status: pending
effort: "1.5 days"
depends_on: [3]
---

# Phase 4: SDK and Agent steps

## Objective

Let the user get from nothing to a ready SDK and a ready agent inside onboarding, using the
flows that already work in the Project drawer and Agent settings.

## SDK step

States, from the existing managed SDK status (`shell/left_column.rs` ~184-305, `studio-engine/src/sdk_setup.rs`):

| State | Shows | Actions |
|---|---|---|
| Ready | version, location (`~/.fframes/sdk/active`) | Next |
| Missing, bundle for this host found | size, location | **Install SDK** (primary), Import offline bundle…, Select installed SDK… |
| Missing, no bundle | `missing_sdk_bundle_message` text | Import offline bundle…, Select installed SDK… |
| Installing | progress and the step being run | Cancel (if the install supports it, else disabled with reason) |
| Failed | the existing structured error | Retry, Import…, Select… |

- Dispatch `Command::InstallSdk`, `Picker::SdkBundle` and `Picker::Sdk`, the same calls as the
  Project drawer. Do not add a second installer path.
- Skip: "Without the SDK, Studio cannot build previews or export. Install it later from
  **Project → Managed SDK**."
- Check whether `desktop/crates/studio-ui/src/setup_view.rs` (`SetupView`, not wired into the app
  today) has reusable preflight or progress rendering. Reuse it if so; delete it if it is dead
  code that duplicates this step (ask before deleting).

## Agent step

Reuse `agent/provider_setup.rs` (Detect agents, Select, Install adapter, Install guide, Get
Node.js) and `agent/setup_page.rs` (sign-in env names, Check adapter):

- On entering the step, run **Detect agents**. It is read-only and starts nothing, so it is
  allowed automatically.
- Show one row per built-in profile (Claude, Codex, Pi, Antigravity) plus "Custom…", with the
  existing statuses: Ready, Install adapter, Install guide, Sign-in needed, Not checked.
- **Select** saves the profile and runs one connection test, which is today's behaviour. On
  Sign-in needed, show the provider's sign-in step and **Check again**.
- If more than one agent is ready, the first Ready one is preselected. Next is enabled once one
  is selected (Ready or Sign-in needed). Skip says: "You can turn on an agent later from the
  Agent panel."
- Factor the row rendering out of the Agent settings page so both use the same component. Do
  not fork it.

## Tests (keep)

- Parity: with `Studio::without_managed_sdk`, the SDK step shows Select installed SDK…, and
  selecting the fixture SDK moves the step to Ready.
- Parity: with the scripted adapter saved in the registry, the Agent step shows it Ready after
  Select (the connection test runs against `acp-agent.py`).

## Done when

- Both steps work against the parity fixtures. Captures of SDK Missing/Installing/Ready and Agent
  Detect/Ready/Sign-in needed, in light and dark, are inspected.
