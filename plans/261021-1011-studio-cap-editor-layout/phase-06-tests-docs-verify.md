# Phase 6: Tests, docs and screenshot check

## Steps

1. Parity suite (`desktop/crates/studio-ui/tests/parity/`):
   - `window.rs`: the sidebar card (`agent-column`) is right of the player and above the
     timeline; the timeline spans both; the tab switch replaces the `rail-*` drawer toggles;
     `FocusPrompt` selects Agent.
   - `transport.rs`: the new transport geometry.
   - `timeline.rs`: Fit and Split now sit in the transport.
   - `composer.rs`: the `agent-tab-History` flow still works.
2. Read `desktop/app/tests/x11_shell.rs` (it runs on Linux, not here) and update any
   assumption about the drawer or the agent column's position, so it does not break later.
3. Gallery (`app/examples/ui_gallery`): drop `NavRail` if phase 3 deleted it, and show the
   tab rail dot.
4. Docs:
   - `docs/desktop/design-system.md`: rewrite the layout bullet in "Deliberate differences
     from Cap" for the new layout, the "Exact Cap colours" bullet and the `NavRail` row.
   - `docs/desktop/README.md` "Agent panel": it is the sidebar's Agent tab, the grip,
     Changes.
   - The layout lines in `docs/desktop/architecture.md`.
5. Gates on Windows: `cargo fmt --all`,
   `cargo clippy --manifest-path desktop/Cargo.toml --workspace -- -D warnings`,
   `cargo test --manifest-path desktop/Cargo.toml -p studio-theme -p studio-ui`.
6. Run `cargo run --manifest-path desktop/Cargo.toml -p fframes-studio` with the Hello World
   example open. Take screenshots in light and dark at 1280×800 and 1920×1080, compare them
   with the user's Cap screenshot, and list what still differs.

## Acceptance

All gates pass. The screenshot differences are only the Cap-only controls listed in the
plan's non-goals.
