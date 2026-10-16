---
phase: 2
title: "Composer"
status: pending
priority: P1
effort: "5-7d"
dependencies: []
---

# Phase 2: Composer

## Goal

Replace the prompt field with Zeron's composer: multiline input, slash commands the agent advertises, file and image attachments within confinement rules, stop, and steering while a turn runs.

The 5-7d estimate holds only if the composer keeps Zeron's `ComposerInput` (compiled on the Cap fork with one GPUI API change in [spike S4](../261013-0733-studio-m1-foundations/reports/spike-port-closure.md)) rather than building on the kit `TextInput` as pattern 1 says; that route is 8-10d. **Decided 2026-10-14: keep `ComposerInput`** and apply Cap styling on top; pattern 1 applies only to styling.

## Port sources

Zeron `crates/ui/src/composer.rs` (16,330 lines): `ComposerInput`, `ComposerEvent`, `SlashState`, `AttachmentDraft`, `Composer`; `composer/chip.rs`; tests in `composer/*_tests.rs`. Steering semantics from Zeron's ACP harness (`crates/harness/src/acp/mod.rs`): `StepBoundary` (preempting) for some adapters, `TurnBoundary` for others. Drop dictation, voice, frost, question panels and Zeron settings (closure from spike S4).

## Tasks

1. Scout the current prompt field and how a prompt and images reach the actor (`prompt_with_images`).
2. Port the composer into the module layout in [the plan](./plan.md), each file at most 1,500 lines.
3. Driver: add `AgentEventKind::AvailableCommands`, filled by ACP `AvailableCommandsUpdate`, by the slash commands Claude Code lists in its `init` frame, and by Codex's fixed set (`/compact`, `/review`, mapped in M5a phase 3); publish the list in `WorkflowSnapshot` and feed `SlashState`. Commands are display and insertion only.
4. Images: route every image through the existing `prompt_with_images`; transcode JPEG and other formats to PNG within the existing size cap; disabled with a reason when the agent lacks the image capability.
5. File attachments as ACP resource links: paths inside the project must be `ProjectPath`-valid and are opened without following links; a path outside the project is attached only after a confirmation that names the file, also opened without following links; the deny list (`.env*`, `*.pem`, `*.key`, `.git/`, `.ssh/`) applies to both and cannot be confirmed past. Attachment bytes are never written to the session store.
6. Stop sends ACP cancel. Steering: the boundary comes from the driver's capability flag (ACP: whether the agent advertises `_session/steering`; Claude: user lines with `priority: "now"`; Codex: `turn/steer`), published in `WorkflowSnapshot`; there is no separate per-adapter descriptor value. A message sent during a turn is delivered at that boundary, and "send now" cancels and sends.
7. Port the relevant Zeron composer tests; extend `tests/support/acp-agent.py` to advertise commands, accept images, and run with and without the `_session/steering` extension so both steering paths are tested.

## Reference patterns (Cap, Zeron)

From [the Zeron report](../reports/research-261013-1749-zeron-agent-control.md) and [the Cap report](../reports/research-261013-1801-cap-gpui-desktop-ui.md); see the roadmap's pattern map.

1. Input: superseded by the 2026-10-14 decision above; the composer keeps the ported `ComposerInput` (its own `EntityInputHandler` for IME, dead keys, selection, undo and clipboard) with Cap styling. Multiline: Enter sends and Shift+Enter inserts a newline, bound in the composer key context.
2. Steering (Zeron `acp/mod.rs`): when the agent advertises the `_session/steering` extension, send steers through it; otherwise hold the message and, at the declared boundary with no tool call open, cancel and re-prompt with it. Read the steering mode from one place, the driver's negotiated capability published in `WorkflowSnapshot`. Zeron's driver and engine descriptors disagree for Grok and Devin (`acp/mod.rs:246, 331` against `registry.rs:652, 670`); that is the bug to avoid.
3. Stop: `session/cancel`, then the existing `shutdown_grace` and verified termination (M3 phase 4). Zeron escalates the same way.
4. Questions: the driver's `Question` event (Claude `AskUserQuestion`, Codex `item/tool/requestUserInput`) is shown as a question listing its options rather than as allow or deny. fframes never auto-approves, unlike Zeron.
   - **Corrected 2026-10-16 (M5a phase 1):** an ACP permission option cannot be kind-less in ACP v1 (the kind is a required enum), and the Zeron lines once cited here (`acp/normalize.rs:488-503`) only pick an auto-approve option.
   - ACP questions would come from `elicitation/create`, which fframes does not advertise.
5. Images: fframes already sends image blocks through `prompt_with_images`; Zeron's ACP client sends text only, so nothing is ported there.

## Todo

- [ ] Composer on the ported `ComposerInput` ([component report](./reports/phase-02-composer-ui.md)); steering extension or cancel-and-re-prompt; driver `Question` events as question cards

- [ ] Scout prompt path
- [ ] Composer ported and split
- [ ] Commands event in the driver and slash UI
- [ ] Images through `prompt_with_images` with transcoding
- [ ] Confined file attachments
- [ ] Stop and per-adapter steering
- [ ] Question card for every question (handed over from M5a phase 2). Today the actor shows only the first question's options and sends `QuestionAnswer::default()` for questions 2..n (`studio-engine/src/agent/actor/prompt.rs`, `reply_permission`), so the agent receives answers the user never gave; a free-text question with no options can only be cancelled. The card must collect an answer, or free text, for each question before anything is sent.
- [ ] Tests

## Success criteria

With the fake agent: slash commands list and insert; a JPEG arrives as a PNG image block; a denied file is refused with its reason; a steer message arrives at the declared boundary; "send now" cancels then sends.
