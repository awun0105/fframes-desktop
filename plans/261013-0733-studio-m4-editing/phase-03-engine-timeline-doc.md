---
phase: 3
title: "Engine timing document and edit history"
status: in-progress
priority: P1
effort: "3-4d"
dependencies: [1]
---

# Phase 3: Engine timing document and edit history

## Goal

The project session owns the `TimingDoc` with its own timing revision, applies edit commands, keeps Cap-style snapshot undo/redo, previews every change live, and writes the file at gesture end without touching the build.

## Port sources

Cap `editor_edits.rs`: snapshot history; `pause()`/`resume()` bracket a drag into one undo step; split, trim and move maths. Target `studio-engine/src/timing/{doc,edits,history}.rs`, rewritten against `TimingDoc`.

## Tasks

1. Scout `studio-engine/src/timeline.rs`, `revision.rs` (`excluded`, `kind`), `state.rs` (`reconcile_source`), the watcher in `controller.rs`, and `export-source` binding.
2. Timing revision: give `timeline.json` its own file kind in `studio-project/src/revision.rs` (do not add it to `excluded()`). It stays in the source inventory and in `SourceRevision`, which candidate capture (`capture_verified` in `candidate_validation.rs`), checkpoints, diffs, publication, apply authorization (`authorizes` in `controller/promotion.rs`) and the source watcher use, so a timing-only agent candidate is a distinct revision. Add a build key derived from the inventory without the timing kind, and make `reconcile_source`/`invalidate_source` (`state.rs`) and the build service drop the build only when the build key changes. Track a separate timing revision (content hash) in the project session. Export binds to the pair (source revision, timing revision), and candidate validation of agent edits includes `timeline.json` when an agent changed it.
3. Proto: `EngineCommand::{TimelineSplit, TimelineTrim, TimelineMove, TimelineBeginGesture, TimelineEndGesture, Undo, Redo}` and `EngineEvent::TimelineChanged(TimelineSnapshot)` tagged with the session id.
4. Each edit updates the doc and sends `SetTiming` with a new generation. Gesture end writes `timeline.json` through the M3 publication primitives, which have no single-step replace: publish the new content under a temporary name in `.fframes/cache/timing/` (an excluded tree, so neither the source revision nor the watcher sees it), check that the live `timeline.json` still has the hash (and `FileId`) the session last wrote or loaded (refuse and reload otherwise), then displace the old file into `.fframes/cache/timing/` and publish the new one in its place. Recovery on project open: if `timeline.json` is missing and `.fframes/cache/timing/` holds a valid copy, adopt the newest; never leave the project without a timing doc. Store the written hash so the watcher ignores self-writes. No task journal record, so no journal format change.
5. Retime lane in the preview coordinator (moved from phase 1): apply `SetTiming` on the displayed worker through `PreviewWorkerClient::set_timing`, re-prepare audio when the timing generation changes (add the generation to the prepared audio key), release the previous prepared artifact, and publish the new timeline. Refuse on workers without `set_timing_v1`. Test end to end against a materialized build.
6. History lives in the project session (cleared on project switch and lost on app restart, a roadmap non-goal). An outside change to `timeline.json` (agent or editor) reloads the doc and clears history with a notice.
7. Timing edits are refused with a reason while an agent task holds the project, matching the preset rule.
8. Tests use a scened fixture (`examples/fframes-studio-intro` or a three-scene fixture added here), since the phase 2 template may not exist yet: each edit; a 60-step drag gives one undo entry, one file write and no `JobKind::Build` queued, with the temporary and displaced copies invisible to the source revision; an agent candidate that changes only `timeline.json` captures and shows that change; a crash injected between displace and publish leaves a recoverable doc; an outside write during a drag is not overwritten; undo after an outside change is refused; switching projects clears history; edits during an agent task are refused.

## Todo

- [x] Scout
- [x] Timing revision outside the build key; export binding
- [x] Proto commands; the timeline snapshot rides on `Presentation` instead of an event
- [x] Live `SetTiming`; gesture-end write; self-write hash
- [x] Coordinator retime lane and audio re-prepare
- [x] Per-session history; outside-change reload
- [x] Agent-task refusal
- [x] Tests

## Success criteria

`cargo test -p studio-engine timing` passes on three platforms, including the no-build assertion.

## Delivery notes

- **Mapping.** studio-proto has no `EngineCommand`/`EngineEvent` for project commands.
  Timeline edits are `Command::Timeline(TimelineCommand)` in `engine/backend.rs`. The
  snapshot (`TimelineSnapshot`: generation, document, clips, undo/redo, editing state,
  notice) rides on `Presentation::timeline`, which every command, refresh and preview
  gate already returns with the session id. A separate `TimelineChanged` event would
  have duplicated that path.
- **Timing revision.** `timeline.json` is `FileKind::Timing`. The source inventory hashes
  it as `Other`, so stored checkpoints and revisions keep their values. The new
  `SourceInventory::build_revision()` leaves the timing document out. Builds, worker
  projects, materialization, export binding, preview installs and candidate build
  identities use the build revision. A timing-only rescan changes `source` but not the
  build, the generation or a running job.
- **Session.** `TimingSession` on the `Controller` holds the document, its generation, the
  file identity last loaded or written (the self-write hash) and the history. Its write
  goes through `.fframes/cache/timing/` with no-replace renames and is recovered on
  open. Edits share the preset rule's gates and also refuse during an agent task or
  before a preview of the current build has reported its scenes.
- **Retime lane.** `PreviewCoordinator::set_timing` takes the session's document. The
  pump applies it to the displayed worker, inspects the new boundaries, prepares and
  accounts new PCM, releases the previous artifact, and publishes
  `PreviewEvents::retimed`. A failure after `set_timing` puts the worker back on its
  previous document. Builds apply the document before reading their timeline.
  Candidate previews stay in code order, because their probed audio is laid out that
  way, and an adopted candidate is retimed by the same lane. A worker without
  `set_timing_v1` is refused unless the document is the code order.
- **Candidates.** A captured candidate and its report carry the build revision, so
  validation matches the build identity. A changed `timeline.json` broadens validation
  to the full video.
- **Tests.** `tests/timing.rs` covers each case listed above against a synthetic
  three-scene timeline. `tests/timing_retime.rs` (ignored, `SDK_BUNDLE`) retimes a
  real worker built from the template.
