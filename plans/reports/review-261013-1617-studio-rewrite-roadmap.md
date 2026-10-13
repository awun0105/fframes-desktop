# Review: studio rewrite roadmap and milestone phases

Reviewed on 2026-10-13: `261013-0733-studio-rewrite-roadmap` and its six milestone plans (M1 to M6, 30 phase files). Claims were spot-checked against the repository on `ci/fast-push-checks`; findings marked "verified" were checked in source.

## Status

All 23 findings were applied to the plans on 2026-10-13 (see the roadmap's Validation Log, session 2). The open questions were answered: the CI branches are merged; the Linux apply gap is closed by keeping `qualify-writer` until M3 phase 4; timing undo across restarts is a non-goal; manual checks move to the user's machine if zui does not render on this VM.

## Verdict

The roadmap is sound and executable. Milestones are cut along real seams, each one ends mergeable, the risky assumptions are spiked first, and the red-team pass already fixed the hard safety issues (containment, alias bypass, validated partial apply, the timing revision outside the build key). What remains is a set of ordering contradictions, stale success criteria, design gaps in M4 phase 3 and around the harness removal, and merge and release gates that no longer match how CI runs after `bec9275`. None needs a redesign; most are short plan edits. The most important are items 1, 10, 11, 15 and 18.

## Gates and CI (fix before M1 starts)

1. **Merge gates assume CI runs things it no longer runs on push.** Since `bec9275`, `desktop-self-hosted.yml` runs the integration tests, SDK, packaging, presentation and stress jobs only with `full` ticked (verified). Gates 2 to 4 ("`cargo test` passes on Linux, Windows and macOS CI", "qualification CI still passes") do not say which run counts. Add: "a `full` run of `desktop-self-hosted.yml` on the milestone's merge commit is green". Also state the macOS job's trigger: M1 S1 adds it as `workflow_dispatch` only and M1 phase 2 keeps it without changing the trigger, while gate 2 and every macOS success criterion depend on it.
2. **The "Before M1" gate is already satisfied but recorded as open.** `ci/faster-desktop-ci` was merged as PR #3 (`b8e4c88`, on `origin/main`); only the local `main` is stale. Tick it, and decide whether `ci/fast-push-checks` must also land before `feat/studio-rewrite` branches.
3. **Merge gate 7 cannot pass if zui does not render on this VM.** M1 S1 lists that risk and moves manual checks to the user's machine, but gate 7 ("the app opens on this VM") has no fallback. Add the same fallback to the gate.

## Contradictions between phases

4. **M1 plan says qualification jobs are removed in M6**, a stale claim (`plan.md` success criteria, last item) that the consistency sweep missed; M2 phase 7 removes them. The M1 description also says "three risky assumptions" while there are four spikes.
5. **The order of M2 phases 4, 6 and 7 conflicts.** Phase 4's success criterion says the app crate's `src/` holds only setup, window, bins and qualification code, but `conversation_panel*` only leaves in phase 6, which depends on phase 4. Phase 6 says "the qualification UI is removed in phase 7", while phase 7 depends only on phase 4, so phase 6 may carry dead qualification UI into `studio-ui`. Recommendation: run phase 7 before phase 6 (add `7` to phase 6's dependencies) and relax phase 4's criterion to "except `conversation_panel*`, moved in phase 6".
6. **The M3 phase 3 success grep points at a file that phase 1 removes.** Phase 1 splits `edit_transaction.rs` into `edit_transaction/{mod,executor_unix,executor_windows}.rs`, so `grep -c 'target_os = "linux"' .../edit_transaction.rs` fails or proves nothing. Grep `edit_transaction/mod.rs` (or the directory, allowing the per-OS modules).
7. **(Clarity only) M3 phase 3 transitively waits for M2 phase 8**, through M3 phase 4. The roadmap's milestone row already says "phases 3-5 also need M2 phases 1, 3, 8", but the prose below the table and M3's own text say only "after M2 phase 3". Make the prose match the row.
8. **The `app/tests/agent_workflow_apply_gate.rs` location is stale.** M2 phase 1 moves engine tests out of `app/tests` with no re-export layer, but M2 phase 5 and M3 phase 3 still name `app/tests/agent_workflow_apply_gate.rs`. Add it to the M2 phase 1 move list or say why it stays.
9. **M4 phase 7 says the export options shipped in phase 2.** Phase 1 adds them; phase 2 bundles them.

## Design gaps

10. **M4 phase 3: the gesture-end write is not crash-safe as planned.** `publish.rs` has no single-step replace: it offers `create_new`, `unlink` and a no-replace link or rename (verified). Overwriting `timeline.json` is therefore a two-step displace-then-publish, and task 4 deliberately skips the task journal, so a crash between the two steps leaves no `timeline.json` and no record to recover from. The replace must also be conditional on the file still being the one the session last wrote (stored hash or `FileId`), or an outside edit made during a drag is overwritten. Options: journal the write after all (accepting the format bump), or publish under a new name and keep the old file until the new one is verified, with startup recovery that picks the newest valid copy.
11. **M4 phase 3: excluding `timeline.json` from the source inventory hides it from agent candidates.** `excluded()` in `studio-project/src/revision.rs` feeds the revision inventory (`revision.rs:117,270`) and the manifest checks (`manifest.rs:197,202`), and candidate capture builds its view with the same `SourceInventory::scan` (`candidate_validation.rs:246,269`; verified). Adding the file to `excluded()` would drop an agent's change from the candidate, its diff and its publication, contradicting task 2. If the source watcher filters on the same inventory (not verified), task 5's "outside change reloads the doc" would also stop working. Specify a separate classification ("timing" kind: outside the build key, inside capture, diff, publication and watching).
12. **"An export matches the preview" has no tolerance.** H.264, HEVC and VP9 are lossy, so an equality check will fail. Define the comparison in M4 phase 1 or 7: compare the renderer's frames before encoding, or decoded frames with an `odiff` threshold (the e2e tests already use `odiff`).
13. **(Minor) The `main.rs:171` ownership injection belongs to M2 phase 7, not M3 phase 4.** It is not a production path: it is reached only through the `QualifyM3 --test-writer-containment` subcommand (verified). M3 phase 4 cites it as still to be fixed. List the `QualifyM2`, `QualifyM3` and `QualifyWriter` subcommands explicitly in M2 phase 7's delete list and drop the line from M3 phase 4's context.
14. **M4 phase 3's undo is in memory only.** Done-means says Ctrl+Z restores `timeline.json`; history is cleared on project switch and outside change, so an app crash or restart loses timing undo while other edits are journalled. If intended (it matches Cap), record it as a non-goal; otherwise persist the history with the session.
15. **M2 phase 7 opens a window in which no new writer can qualify on Linux.** The ledger is read at runtime from the app data directory (`<data>/qualification/m3-results.json` and `m6-results.json`, `conversation_panel/qualification.rs:5,41-42`; verified), so deleting the repository's `desktop/qualification/` does not break existing installs. But M2 phase 7 also deletes `qualify_writer.rs`, whose doc calls it "the only input that may make a writer qualified". From M2 phase 7 until M3 phase 4, a fresh install or a new adapter cannot reach `ProcessGroupContained`, so agent apply is blocked on Linux too. Either run M3 phase 4 before M2 phase 7 (M3 phase 4 needs only M2 phases 1 and 8), or record the gap as accepted on the branch. Also, `app/tests/provider_profiles.rs:84,100` uses `include_str!` on the repository's `m6-results.json`, so that test must change in the same commit that removes the data.
16. **M2's success criterion contradicts M2 phase 3.** M2's `plan.md` says the UI crates hold "no agent, preview or audio polling", but phase 3 task 6 keeps the 16 ms tick reading the audio output snapshot until M4 phase 6. Change the criterion to "no agent or preview polling; the playback tick stays until M4 phase 6".
17. **The M3 phase 5 tests conflict with the Windows host constraints.** "A client running as another user is refused" needs a second account and "a remote client is refused" needs a second machine, but this host must not get account or network changes. Use tests that need neither: a client under a restricted token or AppContainer (different SID, same account), and an assertion on the created pipe's DACL and `PIPE_REJECT_REMOTE_CLIENTS` flag instead of a remote connection.
18. **The release gate replacement inherits the fast-push weakness.** M2 phase 7 checks that "the tag commit equals the commit of a green CI run on `main`". Since `bec9275`, a push run on `main` is fast-only, and the workflow's own comment says "Run the full set before a release". Require a green `full` run on the tag commit, or the new gate is weaker than the one it replaces.
19. **M5 phase 2 task 5 contradicts itself.** It allows attachments from "only `ProjectPath`-valid, no-follow paths inside the project", then says anything outside the project needs a confirmation. Pick one: refuse outside paths, or allow them after a confirmation that names the file, with the deny list still applied.

## Weak or unmeasurable criteria

20. **The M6 spike grep is ineffective.** `grep -rniw "spike"` treats `_` as part of a word, so it does not match `agent_spike`, `selection_spike` or `StudioSpikeApp`. On the current tree `-w` finds 17 lines and a plain `-i` search finds 133 (verified). Use `grep -rni "spike" desktop --include=*.rs`, and the same over `Cargo.toml` files and workflows.
21. **The M5 phase 1 grammar build-time budget is "an agreed budget".** Put a number in the plan (a measured baseline plus an allowed increase), or the CI step cannot fail.
22. **M1 phase 3 spot-checks three of nine parity items** by deleting a behavior's body. One mutation per parity module (nine) is cheap and this suite is the safety net for M2 to M5.
23. **M2 phase 2's live OS appearance switch is checked by hand only.** A gpui test that sets the window appearance and asserts the theme variant keeps it from regressing.

## Estimates

- Phase sums do not match two milestone totals: M2 phases add up to 24-32d (the plan says 24-31d) and M3 to 18-27d (the plan says 18-26d). The roadmap total becomes 97-138d.
- Likely underestimates: M5 phase 2 (porting a 16,330-line composer in 5-7d), M2 phase 6 (parts of a 16,091-line transcript in 3-4d) and M3 phase 1 (NT handle-relative publication, a new journal format and a Windows executor with crash injection in 5-7d). Spike S4 already re-estimates the first two; add "re-estimate M3 phase 1 from the prototype" to S3's exit.
- M4 phase 3 depends on phase 2, but only its manual check needs a migrated project; the engine document, commands and history can start once phase 1 lands, shortening M4's critical path by about four days with two executors.

## Smaller notes

- The two journal format bumps (M3 phase 1 and M5 phase 4) are sequenced, but neither says a build after the second bump must still read format 2. Say "decodes every earlier format".
- M5 phase 4 diffs come from candidates; when an agent edits `timeline.json`, the diff should show it like any file (follows from item 11).
- M4 phase 2 uses the existing `pre_m4_project` fixture (`desktop/app/tests/fixtures/`); it moves with the tests in M2 phase 1, so cite the new path.

## Coverage by phase

| Phase | Findings |
|-------|----------|
| Roadmap and merge gates | 1, 2, 3, 18 |
| M1 plan | 4 |
| M1 p1 Feasibility spikes | Estimates (S3 re-estimates M3 p1) |
| M1 p2 zui switch | 1 (macOS trigger) |
| M1 p3 Parity tests | 22 |
| M2 plan | 16; estimate sum |
| M2 p1 Crate skeleton | 8 |
| M2 p2 Theme and kit | 23 |
| M2 p3 Engine handle | 16 |
| M2 p4 Shell split | 5 |
| M2 p5 Actor split | 8 |
| M2 p6 Transcript rows | 5; estimate |
| M2 p7 Harness removal | 5, 13, 15, 18 |
| M2 p8 Oversize splits | No issues |
| M3 plan | 7; estimate sum |
| M3 p1 Windows publication | Estimate; journal note |
| M3 p2 macOS publication | No issues |
| M3 p3 Open the gates | 6, 7, 8 |
| M3 p4 Runtime containment | 13, 15 |
| M3 p5 Windows tool broker | 17 |
| M4 p1 Timing hook | 12 |
| M4 p2 SDK and migration | Fixture path note |
| M4 p3 Engine timing doc | 10, 11, 14; dependency could be relaxed |
| M4 p4 Timeline view | No issues |
| M4 p5 Edit interactions | No issues |
| M4 p6 Playback pump | No issues |
| M4 p7 Export panel | 9, 12 |
| M5 p1 Streaming markdown | 21 |
| M5 p2 Composer | 19; estimate |
| M5 p3 Todo panel | No issues |
| M5 p4 Diff review | Journal note; `timeline.json` note |
| M5 p5 Session history | No issues |
| M6 p1 Spike removal | 20 |
| M6 p2 Docs and checks | No issues |
