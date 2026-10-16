---
phase: 4
title: "Diff review"
status: pending
priority: P1
effort: "4-6d"
dependencies: [1]
---

# Phase 4: Diff review

## Goal

Users review what the agent changed as unified or split diffs, reject files before apply (which forms a new candidate that must pass validation), and revert single files after apply (also validated). Data comes from candidates and edit transactions, not git.

## Context

- Apply is authorised only for the exact candidate revision that passed validation (`controller/promotion.rs` `authorizes`).
- Undo history marks whole revisions as reversed (`state.rs` `undo_target`, `reversed`); an Undo candidate must pass validation (`promotion.rs`).

## Port sources

Zeron `crates/ui/src/changes.rs` (6,581 lines): unified and split viewer, `list()` at line granularity, paint-only syntax (from phase 1). Replace its `WatchCheckoutDiffs` source with fframes data.

## Tasks

1. Scout the candidate, validation and apply flow (agent actor apply module after M2, `edit_transaction`, `checkpoint`, `candidate_validation`) and what `ChangeCard` holds.
2. Engine: per-file before and after text for a candidate.
3. Reject before apply: rejecting files creates a new candidate from the accepted files, which runs the full validation pipeline; the panel shows "validating" and then the result. Apply is enabled only for a candidate that passed; the existing `authorizes` check stays unchanged.
4. Revert after apply: reverting a file forms an undo candidate for that path only, validated like any undo. Extend the task revision record and `reversed` to track reversal per path, with a journal format bump that decodes every earlier format (2 and the M3 phase 1 format); a later whole Undo inverts only the paths not yet reverted.
5. Port the viewer; because the agent panel is a narrow sidebar tab, the viewer opens in the main editor area over the preview (unified and split views need the width), entered from the `Changes` row in the sidebar and closed back to the preview.
6. Tests on Windows, macOS and Linux CI: rejecting a file that the rest depends on fails validation and blocks apply; a valid subset applies exactly the accepted files; revert of A then Undo of the revision inverts only B; revert after an outside edit is refused with a reason.

## Todo

- [x] Scout
- [x] Per-file deltas ([engine report](./reports/phase-04-engine.md), [review](./reports/phase-04-engine-review.md))
- [x] Reject forms a validated candidate
- [x] Per-path revert with journal format bump
- [x] Viewer ported and split ([viewer](./reports/phase-04-viewer.md)), opened over the preview from the sidebar ([wiring](./reports/phase-04-wiring.md))
- [ ] Tests on three platforms: the engine tests passed on Windows, macOS and Linux CI; the wiring has run on this VM only so far

Open: no real agent has been driven through review in the running app by hand (the parity case drives the fake agent through the real shell; screenshots came from a stand-in window). A mode-only change shows "No changes".

Open before done: the "Review changes" button sits on the candidate card, which at the default window size is scrolled above the validation and outcome cards exactly when review is asked for (the parity case enlarges the window to reach it). The spec enters the viewer from the sidebar's `Changes` row; put the entry in the pinned Apply banner and drop the test's resize.

## Success criteria

The roadmap's agent Done-means item holds on this VM; tests pass on three platforms; no file under `studio-ui/src/agent/changes` exceeds 1,500 lines.
