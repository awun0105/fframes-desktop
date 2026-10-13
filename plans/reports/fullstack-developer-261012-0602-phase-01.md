# Phase 1 report: design system foundation

Outcome: `desktop/app/src/design_system.rs` is now a folder (`mod.rs`, `palette.rs`, `metrics.rs`, `typography.rs`, `elevation.rs`, `motion.rs`, `contrast.rs`) with a runtime light/dark appearance. All six consumers were migrated per the field map, and the `colors` module, `ThemePreset`, `DensityPreset`, `DesignSystemConfig`, `CONFIG` and `metrics_for` are gone (grep confirmed none was read elsewhere). `main.rs` sets the appearance before `open_window` and observes OS appearance changes in the shell's creation closure (subscription detached).

Files: design_system/*, main.rs, conversation_panel.rs, preset_panel.rs, setup_view.rs, studio_shell.rs, text_input.rs, timeline_view.rs (all under desktop/app/src). The phase file status line was updated. Nothing in examples/ touched; no commit.

Validation:
- `cargo fmt --check`: pass.
- Design-system tests (11, incl. all listed ones plus text style and shadow tests): pass. Contrast shortfalls measured within 0.01 of the recorded values.
- `rg "rgb\("` outside app.rs, agent_spike.rs and design_system/ and `rg "colors::"`: no matches.
- clippy `-D warnings`: passes with `-A deprecated`; without it one error remains in untouched `worker_client.rs:38` (`fetch_update` deprecated by this toolchain). Pre-existing, not from this phase.
- Full CI test command: 155 lib tests pass; 2 lib tests and about 30 integration tests fail because `python` is not on PATH on this VM (`SpawnFailed { command: "python" }`), unrelated to this change.

Decisions and deviations:
- Status colours: `danger`/`success`/`warning` use the Radix step 11 (readable text), surfaces step 3; values fetched from radix-ui/colors. Step 9 is unused because the palette has no separate role for it.
- `ctl*` alphas rounded to whole bytes (0x0b, 0x13, 0x1c light; 0x0f, 0x17, 0x21 dark) from the spec percentages.
- Segment mix: a selected segment adds 10 points to the fill tint only; the border is unchanged (spec wording was ambiguous).
- `control_pad_x/y` kept until Phase 3. Appearance defaults to Dark before `set_appearance` is called.
- Not verified here: live repaint on OS theme switch (needs the user's machine).

Status: DONE_WITH_CONCERNS
Summary: Phase 1 is implemented and its unit tests, fmt and grep checks pass; layout and element ids are unchanged.
Concerns/Blockers: Clippy -D warnings fails on a pre-existing deprecation in worker_client.rs, and python-dependent tests fail on this VM; neither is caused by this phase. Live theme switching still needs a check on the user's machine.
