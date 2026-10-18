# Windows hand-check guide

Step-by-step checks for the rows of [platform checks](./platform-checks.md) that need a person at the screen. Each step says what to do and what you should see. Note pass or fail, and anything odd, as you go; a failure becomes a finding, not a silent fix.

**Completed 2026-10-18:** the owner reported all Windows hand checks passed. Results are recorded in [platform checks](./platform-checks.md). Keep this guide for later regression checks; the release dry run is still pending separately.

## What is prepared

| Item | Where |
| --- | --- |
| Release build of `feat/studio-m6` at `84ba69c` plus the 2026-10-18 fixes | `C:\Users\Administrator\fframes-m6\desktop\target\release\fframes-studio.exe` |
| SDK bundle `studio-sdk-x86_64-pc-windows-msvc-v4`, assembled from the same checkout | `C:\Users\Administrator\fframes-m6\desktop\target\m6-fixed-sdk` |
| Launcher (sets `FFRAMES_SDK_BUNDLE`, puts FFmpeg's DLLs on `PATH`) | `C:\Users\Administrator\studio-hand-checks\start-studio.ps1` |
| Mock projects | `C:\Users\Administrator\studio-hand-checks\01-â€¦` to `05-â€¦` (below) |
| Your pre-M6 Studio data, moved aside so Studio starts clean | `%LOCALAPPDATA%\fframes-studio.pre-m6-backup-20261017`; put it back with `restore-old-studio-data.ps1` in the same folder |

The corrected SDK is installed in `~\.fframes\sdk\active`; the preceding SDK is in `previous`. Projects 01, 02 and 05 now pin the corrected SDK checksum. Their original manifests are backed up under `C:\Users\Administrator\fframes-desktop\output\m6-completion`; their timelines were preserved. Project 03 still awaits Import Rust, and project 04 remains the older-format refusal fixture.

| Project | Made from | Used for |
| --- | --- | --- |
| `01-template` | the studio template: three scenes, `intro`, `story` and `outro` | timeline split, trim, undo, export |
| `02-scenes-and-audio` | the template with the timeline fixture: a scene repeated twice, an overlapping overlay, an audio cue | reorder, audio, overlapping scenes |
| `03-import-me` | the template without `studio.json` | Import Rust |
| `04-older-studio` | the template with `generated_instruction_version` 1 | refusal of an older project; its SHA-256 is in `04-older-studio.sha256.txt` |
| `05-agent-playground` | the studio template | agent turns, so agent edits stay out of the timeline checks |

## 0. Start

Windows is the immediate production priority (owner decision, 2026-10-18). Use this order: window chrome and dialogs (sections 2–3), timeline edits and export on 01 (section 4), reorder and audible playback on 02 (section 4), then agent connection, review and composer on 05 (sections 5–7). Record each result as you go and fix a failure before repeating its affected checks. The release dry run follows the Windows app checks.

1. Connect over RDP and keep the session visible.
2. In PowerShell: `& C:\Users\Administrator\studio-hand-checks\start-studio.ps1`.
3. **Expect** the fframes Studio window. If it says Studio is already running, close the other window first.

## 1. SDK setup

1. The first screen is SDK setup with the steps Preflight, Install SDK, Build template, Render. Start it.
2. **Expect** each step to finish in turn. Install SDK copies about 400 MB, so it takes a few minutes. A failing step is marked with its reason.
3. **Expect** the start screen with Create, Open and Import Rust afterwards.

## 2. Window chrome (M2 open check, caption hit-test)

1. Drag the window by its title bar. **Expect** it to follow the pointer.
2. Double-click the title bar. **Expect** maximise, then restore on a second double-click.
3. Hover Minimise, Maximise and Close, including their outer edges and top corner. **Expect** each to highlight, Close in red, and a click right at the edge to work.
4. Press Tab through the controls. **Expect** a visible focus ring; the caption buttons are the only controls that are not tab stops.

## 3. Native file dialogs and the older-project refusal

1. Click **Open** and choose `04-older-studio`. **Expect** a native Windows folder dialog.
2. **Expect** a refusal: "made by an older Studio (version 1, this Studio reads 2)" and "The project is left as it is and cannot be opened". Nothing opens.
3. Check that the file is untouched:
   `(Get-FileHash C:\Users\Administrator\studio-hand-checks\04-older-studio\studio.json).Hash`
   **Expect** `CBCDE65FDDC5329049CCDB2863E75816D3E2C99CC900229919B379A53A84CD06`.
4. Click **Import Rust** and choose `03-import-me\Cargo.toml`. **Expect** it to open, and a `studio.json` to appear in that folder. No other file should change.

## 4. Timeline edits in a template project (Done-means 2)

Open `01-template`. This folder retains the six clips from the saved session (three `intro`, two `story`, one `outro`, 314 frames). The steps below undo only the new edits you make; they do not reset that starting layout.

1. Click **Build preview**. **Expect** the first build to take a minute or two, then the preview and the saved six-clip timeline. A fresh template starts with `intro`, `story`, `outro`.
2. Press Space to play and Space to pause. Left and Right step a frame; Home and End seek.
3. Seek to about the middle, click the timeline so it has focus, and press **C**. **Expect** the clip under the playhead to become two. Note how long the preview takes to update: it should update without a "compiling" state.
4. Check the file: `Get-Content C:\Users\Administrator\studio-hand-checks\01-template\timeline.json`. **Expect** two adjacent clips with the key of the scene you split (for example `story`) and different trims; the other saved clips stay in place. Splitting exactly on a clip edge is refused; move the playhead inside it.
5. Drag the right edge of the second clip to the left. **Expect** it to shrink, snapping to the playhead unless you hold Shift. Run step 4 again: `trim_end_frames` has changed.
6. Press Ctrl+Z twice. **Expect** the trim, then the split, to be undone. Run step 4 again: after both undos the timeline is back to the layout you opened (six clips in the prepared folder). Ctrl+Shift+Z redoes.
7. Make one split and one trim again, then click **Exportâ€¦**. Pick MP4 and a destination. **Expect** a native save dialog, a size estimate, progress, and a finished file.
8. Open the exported MP4 in a player. **Expect** the same cut and length as the preview.
9. Start another export and press **Cancel export**. **Expect** no output file left behind.

Open `02-scenes-and-audio`, then **Build preview**.

10. Drag a clip to a new position in the order. **Expect** the order to change, and `timeline.json` to list the clips in the new order.
11. Play across the overlay. **Expect** the overlapping scenes to blend and their authored audio cues to play. **Mute** silences the sound while the clock keeps running.

## 5. Agent connection (Done-means 3)

Open `05-agent-playground`. The Agent tab is the first sidebar tab. Its settings button opens Settings â€º Agent.

New Claude and Codex profiles start on their native protocols; saved choices remain in effect. Check all three wires:

**A. Claude Code, native wire**
1. On the Agent page, set **Protocol** to **Claude Code (stream-json)**, and **Executable** to `C:\Users\Administrator\.local\bin\claude.exe` with Arguments empty. Click **Save**, then **Check adapter**.
2. **Expect** a passing check (version 2.1.302 is above the 2.1.296 minimum, and sign-in comes from your own Claude login).

**B. Codex, native wire**
1. Do the same with **Codex (app-server)** and `C:\Users\Administrator\AppData\Local\Programs\OpenAI\Codex\bin\codex.exe`.
2. **Expect** a passing check (0.162.1 is the minimum).

**C. ACP**
1. Click **Detect agents**. **Expect** Claude and Codex to be listed without being started.
2. Select the Claude profile; if needed, choose **ACP** explicitly and **Save** before **Detect agents**. The corrected managed adapter is already installed for these hand checks. Select it, then **Check adapter**. On another setup, **Install adapter** installs that pinned correction (it needs Node.js and npm). A saved custom launch path is not rewritten. The corrected version directory ends in `0.89.1-advisor-result-v1`.

## 6. An agent turn (Done-means 3)

Run this once on each wire, starting with Claude native. Switch the wire on the Agent page between runs.

1. On the Project tab, set the review policy to **Review before Apply**.
2. On the Chat tab, type:
   `Make the title text say "Hand check" and make it red. First write a three-step plan, then do it. Use the fframes project tools to check the scene list.`
   Shift+Enter makes a new line; Enter sends.
3. **Expect** while it runs:
   - the reply streams as formatted markdown;
   - the plan shows as a todo list above the composer;
   - a permission card with the agent's choices and **Decline**, which waits for you (choose to allow);
   - a tool call to the Studio project tools, which shows the tools are available.
4. **Expect** validation to run, then **Review changes**. Open it: the file list with change counts, and unified or split diffs with colours.
5. Turn one file off if there are several. **Expect** the rest to be validated again. Then **Apply**.
6. After Apply, click **Revert** on one file. **Expect** it validated and reverted alone. Then try **Undo last agent edit** on the Project tab.
7. **Expect** the review banner to be laid out cleanly: no overlap, and buttons inside the panel at the default and the maximised window size.

## 7. Composer and history

1. Send a long request (for example, "Rewrite the scene with five animated shapes"). While it runs:
   - type a follow-up and press Enter. **Expect** Steer or Send at boundary, as the agent supports;
   - type another and press **Ctrl+Enter**. **Expect** Send now: the turn is cancelled and the new message sent;
   - **Stop** ends the turn.
2. Type `/`. **Expect** the agent's slash commands; Tab accepts one, Escape closes the list.
3. Attach a project file and an image (paste or the attach button). **Expect** a native dialog, and a chip for each attachment.
4. Click **New chat**, then open the **History** tab. **Expect** both sessions, newest first, with titles. Search for a word from the first prompt and open that session. **Expect** its rows, and a resume or a reason why it cannot resume.
5. Close Studio, start it again with the launcher, and open the project. **Expect** the saved-session offer. It is resumable only on the wire that ran it; switch the wire and the offer should change.

## 8. Report

Tell me pass or fail for each step, with a screenshot of anything wrong, and I will fill the Windows rows of [platform checks](./platform-checks.md). Afterwards, `restore-old-studio-data.ps1` puts your pre-M6 Studio data back.
