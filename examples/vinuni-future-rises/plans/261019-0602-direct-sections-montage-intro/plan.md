---
status: done
created: 2026-10-19
follows: ../261018-2212-three-minutes-bridges-logos/plan.md
---

# Direct section cuts, matching college plates, montage intro

## Outcome

- No section openers and no corner section label: each section starts straight away,
  behind the chapter wipe. The film runs 2:36 (156 s, 9360 frames), 19 scenes.
- The four college plates are the same size: every college uses the horizontal lockup
  layout (mark, rule, `VINUNIVERSITY`, the college's own "College of ..." line), built by
  `tools/fetch-media.py` from the official lockups and drawn at one fixed height.
- A new 10 s intro: a beat montage (a cut every beat with a punch zoom and a light flash),
  a push into the main building, one white hit, the logo assembling from its pieces, and
  a large "HANOI, VIETNAM".

## Constraints

- CPU render, local checks only, nothing committed; scene lengths after the intro unchanged.
- Health and Business wide lockups are assembled from parts of their official stacked
  lockups; say so in the report.

## Timeline (seconds)

dawn 0, who 10, mission 18, stars 26, colleges 34, the four colleges 42-74, learn 74,
community 84, globe 92, campus 102, clubs 110, values 116, outcomes 124, study 132, join 138,
rise 146-156. Chapters start at 10, 34, 74, 84, 102 and 124.

## Validation

fmt, clippy `-D warnings`, tests (156 s on the grid, chapters on cuts), coverage check,
`inspect --every 0.25s`, frames of the intro, the four plates and a wipe, final render
(9360 frames) and loudness.

## Results (2026-10-19)

- fmt, clippy `-D warnings` and 6 tests pass; every clip covers its shot; inspect checked
  643 frames with no problems.
- `vinuni-future-rises.mp4`: 1920x1080, 60 fps, 9360 frames, 156.0 s.
- Audio: -14.1 LUFS integrated, true peak -1.6 dBTP, no clipping; silent only in the final
  1.1 s fade.
- A frame-by-frame band check over 7.75-8.5 s showed the amber rules touching "HANOI,
  VIETNAM" while it was still tracking in; they now grow only after the tracking settles
  (from 8.25 s), and the re-checked band is clear. At 10.3, 34.3 and 74.3 s the section starts straight
  behind the wipe with no label or ticks.
