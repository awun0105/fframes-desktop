# fframes Studio

fframes Studio is a native GPUI app in Rust where a person describes a video, watches it, selects a scene or visible object, and asks a coding agent to change it. The agent edits an ordinary fframes Rust project. The app owns project setup, style presets, agent connections, compilation, preview, validation, history and export. Rust source remains the authority for the video; templates are starting points.

**Install the unsigned Windows preview** (per user, no administrator rights): `irm https://github.com/khanhthanhdev/fframes-desktop/releases/latest/download/install.ps1 | iex`, Scoop, or the Setup.exe from the [latest release](https://github.com/khanhthanhdev/fframes-desktop/releases/latest). winget follows once `khanhthanhdev.fframesStudio` is accepted into the community repository. See [preview releases](phase-zero-feasibility.md#unsigned-preview-releases).

Documents:

1. This README: setup and use, and which platforms are tested.
2. [Architecture](architecture.md): crates, engine flow, agent wires, preview pipeline, containment, tools, tests and probes, as built.
3. [Design system](design-system.md): the UI tokens, component kit and gallery every Studio screen is built from.
4. [Self-hosted runners](self-hosted-runners.md): which CI job runs which test, and how the runners are set up.
5. History, not the current design: [research findings](research.md), [implementation plan](implementation-plan.md), [feasibility record](phase-zero-feasibility.md).

## Setup

For a Windows-only preview release dry run, dispatch [Desktop Release](../../.github/workflows/desktop-release.yml) on the source branch with `preview_release=true`, `windows_only=true`, `dry_run=true` and `consumer_release=false`. Set `tag` to `v` plus the app version. It prepares the ZIP, installer, checksums and install manifests as the `studio-preview-dry-run` artifact and publishes nothing. Consumer publishing retains its separate signing and eligibility gates.

Run Studio from a checkout:

```sh
cargo run --locked --manifest-path desktop/Cargo.toml -p fframes-studio
```

The app is the `fframes-studio` package in [`desktop/app`](../../desktop/app). The package also builds the `studio-tools` CLI and the `studio-mcp` server that agents use; `studio_setup` ([`desktop/crates/studio-sdk/src/bin/studio_setup.rs`](../../desktop/crates/studio-sdk/src/bin/studio_setup.rs)) is the GPUI-free doctor and project scaffolder.

Projects compile against a managed SDK. The setup screen installs it step by step and marks the failing step if setup fails. The SDK manifest comes from the packaged bundle or `FFRAMES_SDK_BUNDLE`, then from the built-in manifest when it targets this host; development builds also look for a bundle assembled into the desktop workspace's target folder, `phase-zero-sdk` ([`build-phase-zero-sdk.ps1`](../../desktop/scripts/build-phase-zero-sdk.ps1) on Windows, [`build-phase-zero-sdk.sh`](../../desktop/scripts/build-phase-zero-sdk.sh) elsewhere); otherwise Studio uses the installed SDK's receipt (`~/.fframes/sdk/active`) or asks you to import a bundle ([`sdk_setup.rs`](../../desktop/crates/studio-engine/src/sdk_setup.rs)). Host prerequisites are checked against the selected SDK's manifest. Opening a project never installs an SDK or runs Cargo.

App data lives in `%LOCALAPPDATA%/fframes-studio` on Windows, `~/Library/Application Support/fframes-studio` on macOS and `${XDG_DATA_HOME:-~/.local/share}/fframes-studio` on Linux ([`app_paths.rs`](../../desktop/crates/studio-engine/src/app_paths.rs)). It holds the SQLite metadata, per-project journals, checkpoint objects, the agent draft and stored agent sessions, build copies and `app-settings.json`. None of it travels with the project.

## Projects

**Create** chooses a new folder and writes the template ([`desktop/crates/studio-project/templates`](../../desktop/crates/studio-project/templates)): a Rust crate whose scenes carry an `editor_instance_key`, a `timeline.json` opt-in in `src/main.rs`, a Studio worker entry, style tokens and `AGENTS.md`. **Import Rust** selects a package's `Cargo.toml`, keeps existing files, instructions and Git history, and adds only `studio.json`. Managed builds support contained workspace dependencies; external paths, symlinks, inherited Cargo configuration and overrides produce diagnostics instead of hidden rewrites.

Source and copied assets travel with the folder; local history does not. A checkpoint saves bytes, not proof of a successful build. **Restore as copy** exports a checkpoint to a separately identified folder and never replaces the current source. **Locate** reconnects a moved, unchanged project; **Remove recent** removes only the list entry.

**Projects made by an older Studio are refused.** There is no migration. A `studio.json` of an earlier schema or instruction version is refused with "made by an older Studio … The project is left as it is and cannot be opened" ([`manifest.rs`](../../desktop/crates/studio-project/src/manifest.rs)), and the same rule covers the history database, journals, checkpoint inventories, stored agent sessions and the provider registry: an old file is reported and left byte for byte, and nothing repairs or rewrites it. A file from a newer Studio asks you to update Studio. An `agent-adapter.json` or a single-session conversation log from an earlier build is simply ignored, and Setup is shown again.

## Editing

**Build preview** compiles the project in an isolated, SDK-bound copy and prepares the timeline, inspection, first frame and audio before replacing the displayed revision. Failed or cancelled builds keep the previous preview, and installing a preview never moves the saved checkpoint.

The player plays, pauses and steps frames; the ruler scrubs. Click a scene or use **Cycle scene** to select overlapping instances, and drag or Shift-drag on the tracks to select a half-open range. When the worker supports canvas selection, a click on the preview pauses and selects the object under the pointer, **C** cycles through overlapping objects, Ctrl-wheel zooms, middle-drag pans, **0** fits and Shift-drag selects a rectangle. Selections do not edit source; they scope the next agent task.

The timeline ([`studio-ui/src/timeline`](../../desktop/crates/studio-ui/src/timeline)) shows the scene clips in play order, the audio files and the mix waveform. **Split**, trim and move edit `timeline.json` directly, with undo and redo; a drag is one undo step and one write, and a timing-only change does not rebuild. Timing edits are refused while an agent task runs. See [architecture, timing](architecture.md#timing-document-and-revision).

**Mute** silences output without stopping the clock; **Next output**, **Retry audio** and **No audio** pick the output device or a silent clock. Output latency and underruns appear below the preview.

The **Style** tab applies a preset or edits token overrides through the same durable file-set transaction as an agent edit. **Export…** opens a panel in place of the editor with the formats the project's worker can encode, the size and frame rate, an estimate of the file size, progress and **Cancel export**.

## Agent panel

The agent panel is the sidebar's first tab, **Agent**, next to Project, Assets and Style. It has three tabs: **Chat** (the conversation, the plan and the composer), **History** (stored sessions) and **Project** (review policy, Apply, Undo, accepted edits and recovery). The settings button in the panel header opens the **Agent** page of the settings frame.

### Providers and wires

Studio has three agent wires, and the **Protocol** control on the Agent page chooses one per saved adapter: Claude Code's native stream-json wire and Codex's native app-server wire, which start the `claude` or `codex` CLI you point the executable at, and the Agent Client Protocol (ACP v1) for every other agent. New Claude and Codex profiles use their native CLIs; Pi and Antigravity use ACP. Saved protocol choices are retained ([`provider_profiles.rs`](../../desktop/crates/studio-engine/src/providers/provider_profiles.rs)). The native wires refuse a CLI older than the minimum version in [`claude/launch.rs`](../../desktop/crates/studio-harness/src/driver/runtime/claude/launch.rs) and [`codex/launch.rs`](../../desktop/crates/studio-harness/src/driver/runtime/codex/launch.rs). See [architecture, agent wires](architecture.md#agent-wires).

- **Detect agents** lists installed agents by checking that files exist; it never starts them. Launching uses only the saved absolute path or the managed adapters folder, never a search of the global `PATH`.
- **Select** saves the adapter in `provider-registry.json` and runs one connection test. **Check adapter** tests again, for example after you sign in with the agent's own tool. **Install adapter** installs a pinned ACP adapter package from the lockfile Studio ships; **Install guide** opens the agent's official install page. Opening a project never launches an agent.
- For an ACP profile, when the agent's CLI is found but its ACP adapter is not, the Agent page says so and offers **Install adapter** (and **Get Node.js** when Node is missing); an install stops after 15 minutes ([`adapter_install.rs`](../../desktop/crates/studio-harness/src/adapter_install.rs)).
- Sign-in stays with each agent. The registry stores only the names of sign-in environment variables, never their values, and refuses a value pasted where a name belongs. `CLAUDECODE*` variables are never passed to an agent ([`discovery.rs`](../../desktop/crates/studio-harness/src/discovery.rs)).
- A turn stops after 15 minutes without agent activity or 2 hours in total ([`runtime/mod.rs`](../../desktop/crates/studio-harness/src/driver/runtime/mod.rs)). Every permission request becomes a card with the agent's choices and **Decline**; Studio never answers one for you.
- The Agent page shows the writer containment the last launch verified (a job object on Windows, a process group with an escape scan on Linux and macOS). A writer that is not contained cannot have its candidate captured.

### The composer

The composer ([`studio-ui/src/agent/composer`](../../desktop/crates/studio-ui/src/agent/composer/mod.rs)) is a multi-line input with IME support. Enter sends, Shift+Enter breaks the line, Tab accepts a slash-command completion from the list the agent advertises, and Escape closes the completion. Paste or attach project files, files outside the project (after you confirm the exact path shown) and images; the engine checks every attachment before it shows a chip.

While a turn runs, Enter does what the agent supports: **Steer** hands the message to the running turn, **Send at boundary** holds it for the next step or turn, and Ctrl+Enter (Cmd+Enter on macOS) is **Send now**, which cancels the turn and sends. A brief that is not delivered to the running turn waits in the queue for the next task, eight at most, and starts from the then-current source. The agent's plan appears as a todo list above the composer, and its questions appear as cards that you answer item by item. **Stop** is available in every running state.

### Multi-session history

Each project keeps many agent sessions. **New chat** in the panel header starts a new one; the History tab lists every stored session with search, newest first, and **New session**. Opening a stored session shows its rows and resumes the agent's own session only when the project, its draft and revisions and the provider's launch still match; otherwise the panel says why it cannot resume. Attachment bytes are never stored.

### Review and diffs

A validated candidate is applied automatically under the default review policy, or waits for **Apply** under manual review. **Review changes** opens the review window in place of the preview: a file list with change counts beside unified or split diffs with syntax colours ([`studio-ui/src/agent/changes`](../../desktop/crates/studio-ui/src/agent/changes/mod.rs)). Before Apply you can turn files off; the remaining subset is validated again before it can be applied, and turning every file off asks first, then discards the candidate. After Apply, **Revert** on a file builds an undo candidate for that file alone, validated like an Apply; it is refused when the file changed since or was already reverted (`AgentWorkflow::revert_path` in [`agent.rs`](../../desktop/crates/studio-engine/src/agent.rs)). **Undo last agent edit** reverts one accepted revision, keeps unrelated edits and is validated like an Apply. Conflicts list every preserved variant.

Validation reports the rendered and inspected frames, boundary frames, audio checks, diagnostics and capability gaps. At most one automatic repair turn runs.

## Keyboard

All key bindings are installed in one place, `bind_keys` in [`shell/keymap.rs`](../../desktop/crates/studio-ui/src/shell/keymap.rs), which also binds the text input keys and the composer's keys ([`composer/keymap.rs`](../../desktop/crates/studio-ui/src/agent/composer/keymap.rs)). Key contexts go from most specific to least: a focused text field, the agent composer, then the player and the timeline, then the window. Plain keys are bound only in the player and the timeline, so typing in a field never reaches them.

| Context | Keys |
| --- | --- |
| Player | Space play/pause, Left/Right step, Home/End seek, C cycle canvas selection, 0 fit, Escape clear selection |
| Timeline | Space, Left/Right, Home/End as in the player; M mute; + / - zoom; PageUp/PageDown pan; S cycle scene; C split at the playhead; [ and ] mark the range; Ctrl+Z / Ctrl+Shift+Z undo and redo timing edits |
| Window, outside text fields | Ctrl+Z / Ctrl+Shift+Z undo and redo timing edits |
| Composer | Enter send, Shift+Enter new line, Tab accept completion, Ctrl+Enter send now; the usual editing keys |

On macOS the command key replaces Ctrl. Controls are tab stops and activate on Enter and Space; the Windows caption buttons are the exception ([design system](design-system.md#keyboard-and-focus)).

## Platforms

- **Windows** is the platform the hand checks are made on ([platform checks](../../plans/261013-0733-studio-m6-cleanup/reports/platform-checks.md) records them), and the full local test sets run there. A self-hosted Windows runner also runs the tests in CI.
- **Linux** compiles and runs the tests on the self-hosted CI runner, including the X11 shell test on Xvfb in full runs ([self-hosted runners](self-hosted-runners.md)).
- **macOS is not fully tested.** It is covered only by CI: the Desktop CI `desktop-macos` job runs the tests and clippy on hosted macOS for pull requests and manual runs ([`desktop.yml`](../../.github/workflows/desktop.yml)), and Desktop Release builds the macOS package. No hand check, real-CLI check or on-screen check runs on macOS ([roadmap, Platforms](../../plans/261013-0733-studio-rewrite-roadmap/plan.md#confirmed-contract)).

Ordinary agent tests use scripted peers or fake CLIs. The ignored `claude_acp_real_cli` test installs the pinned adapter, checks its actual advisor-result conversion, then verifies denied and approved edits with a signed-in Claude session; see its source for the required environment variables. The real Claude Code and Codex CLI tests are ignored by default and need a signed-in CLI: `cargo test --manifest-path desktop/Cargo.toml -p studio-harness --test claude_real_cli --test codex_real_cli -- --ignored`.

## Architecture at a glance

```mermaid
flowchart LR
  User[User: prompt or selection] --> UI[GPUI workspace]
  UI <--> Engine[Rust project engine]
  Engine <--> Agents[Agent workflow and drivers]
  Agents <--> Coding[Claude Code, Codex, ACP agents]
  Coding --> Draft[Draft Rust project]
  Engine --> Context[Selection, source index, style]
  Context --> Agents
  Coding <--> Tools[fframes tools: MCP or local CLI]
  Tools <--> Engine
  Draft --> Build[Managed build service]
  Build --> Runner[Project preview worker]
  Runner --> Frames[Frames, timeline, diagnostics, audio]
  Frames --> UI
  Engine --> History[Accepted revisions and recovery]
```

"Agent finished" and "video ready" are separate states: the app still builds, inspects and renders the result before it shows it as accepted.
