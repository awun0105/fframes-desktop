# Studio M2 hand check (Windows, 2026-10-15)

App: `cargo run --locked --manifest-path desktop/Cargo.toml -p fframes-studio`, debug build, RDP session. First run on `b25c18d`; second run on `e68e7a3` (the changes asked for in the first run).
Legend: `[x]` works, `[!]` broken (see note), `[ ]` not checked yet.

## A. Window and title bar (lost from the removed native harness)

- [ ] A1. The window opens with the editor layout: sidebar on the left, preview on its right, timeline below.
- [ ] A2. Dragging an empty part of the title bar moves the window (over the project name or path a drag selects text instead).
- [ ] A3. Double-clicking an empty part of the title bar maximizes, double-clicking again restores.
- [ ] A4. Minimize, maximize and close buttons change colour on hover; close turns red.
- [ ] A5. Resizing the window from a corner redraws without gaps or a frozen frame.

## B. Project

- [ ] B1. A recent project opens (or Open/Create works) and the preview shows its first frame.
- [ ] B2. Play/pause (button and Space) plays the video with sound; the playhead moves on the timeline.
- [ ] B3. Clicking the timeline seeks; the preview updates.

## C. Sidebar Agent tab (new in M2)

- [ ] C1. The sidebar's first tab is Agent, and it is the one selected when the app opens.
- [ ] C2. Switching to Project, Assets and Style and back to Agent works.
- [ ] C3. Inside Agent, the Chat and Project tabs switch.
- [ ] C4. The message box stays visible at the bottom of the Agent tab in a short window (make the window about 800 px tall); the content above it scrolls with the mouse wheel.
- [ ] C5. Clicking the message box puts the cursor in it; typing, Ctrl+A, Ctrl+C / Ctrl+V and Ctrl+Z work there.
- [ ] C6. The sidebar width (416 px) feels right: nothing is cut off, no awkward wrapping.

## D. Settings > Agent page (new in M2)

- [ ] D1. The settings button in the Agent panel header opens Settings on the Agent page.
- [ ] D2. Settings has an Appearance page and an Agent page; the nav switches between them.
- [ ] D3. Appearance: switching Light / Dark / System changes the theme at once.
- [ ] D4. Agent page: Detect agents lists installed agents (or says none), without starting any.
- [ ] D5. Agent page: editing the label and pressing Save keeps the value after closing and reopening Settings.
- [ ] D6. A setup notice or error (for example a bad executable path) shows on the Agent page.

## E. Agent task (only if an agent is signed in)

- [ ] E1. Sending a prompt starts a task; the transcript follows new rows to the bottom.
- [ ] E2. Scrolling up during a task stops the follow; scrolling back to the bottom resumes it.
- [ ] E3. An accepted change updates the preview; Undo (Agent > Project tab) restores it.

## F. Theme follows the OS (open since phase 2)

- [ ] F1. With Appearance on System, switching Windows between light and dark mode (Settings > Personalization > Colors) flips the app without a restart.

## G. Changes asked for in the first run

- [ ] G1. The sidebar is on the left of the preview (with a project open and without one).
- [ ] G2. While a preview builds for the first time, the badge says "Building preview" (not "prior preview retained").
- [ ] G3. Clicking the empty stage before a preview exists shows no "editor metadata is invalid or unavailable" note.
- [ ] G4. The status card stays one line while a build runs; its chevron opens the details, which now include the full Source and Checkpoint ids.
- [ ] G5. Copy: drag over an agent message, a notice, the status details, the timeline caption or the SDK status, then Ctrl+C and paste elsewhere.
- [ ] G6. Copy: double-click selects a word, triple-click the whole block; clicking elsewhere clears the selection.
- [ ] G7. Copy: after copying agent text, select text in the prompt box (Ctrl+A or mouse) and Ctrl+C copies the prompt's text, not the agent text.
- [ ] G9. With the preview built, clicking on an object in the video selects it and shows no "editor metadata is invalid or unavailable" note.
- [ ] G8. Copy: the project name and path in the header can be selected (triple-click copies the whole path); dragging the empty part of the header still moves the window.

## Notes

- First run: the sidebar was on the right (Cap's layout), nothing could be copied, the status card opened during the build, the badge said "prior preview retained" with no preview, and a click on the empty stage showed "editor metadata is invalid or unavailable". Fixed in `e8d065b`, `9726cd8`, `8bf4610`, `b8dde89` and `e68e7a3`.
- Not selectable by design: button, tab, menu and list-row labels (clicking them does something), and collapsible section titles. A selection never spans two text blocks.
