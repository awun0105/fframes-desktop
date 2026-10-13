# Linux verification and minor fixes

## Linux: NOT truly compiled; verified by a Windows-side substitute
A real `x86_64-unknown-linux-gnu` check is impossible here. The target is installed, but `cargo check --target` fails in build scripts. wayland-backend, freetype-sys, alsa-sys and fontconfig need a Linux C toolchain, headers and pkg-config. `ffmpeg-sys-fframes` tries to build FFmpeg from source for Linux and needs a sysroot. WSL has no distro, and docker and zig are absent. I got past wayland, freetype and fontconfig with a fake compiler (per-command env only) before ffmpeg-sys blocked it, so I abandoned that route.

Substitute: for each Linux-gated test file I stripped the `cfg(unix)` / `cfg(target_os = "linux")` crate attribute temporarily, ran `cargo check --test` and `cargo clippy --test ... -D warnings` on Windows, then restored the files with `git checkout`.
- preset_workflow, agent_workflow_ui, agent_workflow, promotion_handoff: type-check and clippy clean. provider_handoff compiles and is clean as is.
- x11_shell (including `click_panel_button`): the only errors were `std::os::unix`, `libc` and `process_group`. With those stubbed, it type-checks and clippy shows no other lint, so the new helper and the telemetry key names compile against current APIs.
- agent_tools: only unix-API errors (permissions `mode`, `chown`, `cmdline`). It is unchanged by the restyle, and the agent_tools modules it imports are unchanged.
- `cfg(unix/linux)` blocks in `desktop/app/src` (agent_tools, host.rs, qualification.rs, stress_metrics.rs, audio_service.rs and others): none were touched by the restyle (`git diff 8328996..HEAD` is empty for them). They are unchanged and not re-verified beyond that.
- The only test file changed since the last Linux-green state is x11_shell.rs. `lib.rs` gained 2 lines.

Residual risk: unix-only API usage inside the stripped files (permissions and similar) was not type-checked. Those lines are untouched by this work.

## Linux runtime review
- Appearance: `cx.window_appearance()` on gpui_linux defaults to Light and updates from the xdg portal. Under bare Xvfb with no portal the app renders the light theme, which is a valid token set. `observe_window_appearance` handles later changes.
- Icons: embedded with `include_bytes!` through `with_assets`, so there is no filesystem lookup. SVG rendering is done by gpui and is platform independent.
- Fonts: the code sets no font family, so it uses gpui's default. Linux resolves it through fontconfig, and a minimal CI image without fonts is the only risk. It behaves as before the restyle.

## Fixes
1. Overflow menu (`studio_shell.rs`, `ui/menu.rs`): new `MenuState::set_disabled`, `sync_overflow()` called from `render` while the menu is open, and `run_overflow` now checks the current availability via the pure `enabled_overflow_action`. Tests: `a_click_on_an_action_that_became_unavailable_is_inert`, `availability_can_change_while_the_menu_is_open`.
2. `SetupView::fail()` (`setup_view.rs`): new pure `failed_step_after` keeps the remembered step when the previous state is already `Failed`. Test: `a_second_failure_keeps_the_original_step`.

## Verification (CI commands, no -A)
- `cargo fmt --manifest-path desktop/Cargo.toml --all --check`: clean.
- clippy with `-p studio-project -p studio-engine -p studio-bootstrap -p studio-sdk -p studio-agent-spike -p fframes-studio --all-targets -- -D warnings`: clean.
- `cargo test --locked --no-fail-fast` over the same set (python shim on PATH plus `PYTHONHOME`, for the cargo process only): 644 passed, 0 failed. Without `PYTHONHOME` the shim makes the fake-worker tests fail, so it is required.
- No background processes left. Scratch files are only in the scratchpad.

## Commits (not pushed)
- 9da2b55 fix(desktop): keep the open overflow menu in step with current availability
- 3fbe519 fix(desktop): keep the original failed setup step on a repeat failure

The untracked report and the pre-existing modified `examples/fframes-studio-intro` files were left alone.

Status: DONE_WITH_CONCERNS
Summary: Both minor fixes are committed with pure tests, and fmt, clippy and the 644 tests are clean. Linux was desk-checked and compiled on Windows with the gates stripped; it was not compiled for the Linux target.
Concerns/Blockers: A real Linux compile needs a Linux host or CI; unix-only API usage in the stripped test files was not type-checked.
