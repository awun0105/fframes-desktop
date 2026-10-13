# Phase 10: Verification and evidence

Status: partially verified (VM checks done 2026-10-12; display and Cap checks pending the user). See reports/tester-261012-0602-phase-10.md. Depends on: Phases 1 to 9. Effort: 1-2 days.

## Goal

Prove the acceptance criteria in `plan.md`, compare against Cap, and record what could not be verified. Existing qualification screenshots under `desktop/qualification/evidence/` show the old UI and stay as historical records; new screenshots go under `plans/reports/`.

## Checks on this VM

1. Literal colour gate: `rg "rgb\(|rgba\(" desktop/app/src --glob '!app.rs' --glob '!agent_spike.rs' --glob '!design_system/**'` returns nothing; `rg "colors::" desktop/app/src` returns nothing.
2. Full gate from `.github/workflows/desktop.yml`:
   - `cargo fmt --manifest-path desktop/Cargo.toml --all --check`
   - `cargo test --locked --manifest-path desktop/Cargo.toml -p studio-project -p studio-engine -p studio-bootstrap -p studio-sdk -p studio-agent-spike -p fframes-studio`
   - `cargo clippy --locked --manifest-path desktop/Cargo.toml -p studio-project -p studio-engine -p studio-bootstrap -p studio-sdk -p studio-agent-spike -p fframes-studio --all-targets -- -D warnings`
   Report any failure with its output; do not weaken tests.
3. Packaged build: `desktop/scripts/package-phase-zero.py`; icons load from the packaged binary; the Lucide notice is in the package.

## Checks on the user's machine (GPU and display), or a Linux host with Xvfb

4. Id and telemetry inventory: run the app in qualifying mode (`start_qualification`) and agent telemetry mode (`start_agent_telemetry`) with a project open, on the old build and the new build, and diff the JSON: keys must be identical, only coordinates may change, and `buttons` must contain `build-preview`, `cancel-preview`, `play-pause`, `mute-audio`, `next-audio-output`, `retry-audio-output`, `no-audio-output`, `previous-frame`, `next-frame`, `close`, `timeline-fit`, plus the agent panel's `agent-policy`, `agent-apply` and `prompt_bounds`. These need a running shell with a worker, so they cannot run on this VM.
5. `desktop/scripts/qualify-m2-playback.py` and `cargo test -p fframes-studio --test x11_shell` on Linux with Xvfb.
6. Screenshots in light and dark at 1280 x 800, 1440 x 900 and 1920 x 1080: empty state, project with preview, Style tab with a preset applied, agent mid-stream, an error notice expanded, and the setup screen. Save under `plans/reports/` with a short review note.
7. Side-by-side with Cap's editor (running Cap or `~/code/Cap/apps/desktop-gpui/docs/sidebar-*.png`) in the same appearance; list visible differences that are not documented deliberate differences.
8. Keyboard-only pass: Tab through the whole window, every control shows the focus ring and activates with Enter or Space; menus follow their key contract.
9. OS theme switch while the app is open.
10. Native scripts: `qualify-windows-native.py` (Windows) and, on Linux with X11, `qualify-linux-native.py` and `cargo test -p fframes-studio --test x11_shell`; name-based and telemetry-based clicks must pass.
11. Ignored real-SDK tests where an SDK bundle exists (`managed_worker`, `portable_managed_worker`, `preview_coordination`, `timeline_controls`, `audio_preview`, `real_sdk_*`).

## Docs

`docs/desktop/design-system.md` (written in Phase 3) is updated with any value that changed during implementation. Update other docs only if they describe the old layout.

## Exit criteria

Every acceptance criterion in `plan.md` met, or listed as not verified with the reason (for example "native X11 test not run: no Linux display available"). Review the diff against the non-goals: no new features, no protocol or worker changes, `app.rs` and `agent_spike.rs` untouched, no Cap code copied.

## Rollback

Nothing ships from this phase; fixes go back into the owning phase.
