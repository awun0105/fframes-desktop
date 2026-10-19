---
status: done
created: 2026-10-18
follows: ../261018-2126-graduate-outcomes-motion/plan.md
---

# Three minutes, chapter bridges, real logos

## Outcome

- The film runs 3:00 (180 s, 10800 frames at 60 fps), still on the 2 s grid.
- The six chapters are connected: each opens with a 4 s bridge (an establishing shot, the
  chapter number and title, one line that hands over from the last chapter), a navy and
  amber wipe crosses every chapter cut, and the music breaks down under each bridge and
  drops back in on the chapter's first bar.
- Inside a chapter every cut dissolves the footage over 0.5 s (`scenes::carry_in`), so the
  only hard cuts are the chapter wipes.
- Real logos: employers and universities as their official logos on white chips; the
  strategic partners (Cornell, Penn, NTU) as logos on the globe; each college as its official
  lockup; the VinUni logo stays.

## Constraints and non-goals

- CPU render, local checks only, nothing committed.
- Logos from Wikimedia Commons (files marked public domain as text or simple logos, still
  trademarks); NUS and NTU only exist as non-free files on en.wikipedia. All downloaded by
  `tools/fetch-media.py` into git-ignored `dynamic_media/`.
- Colleges: their sites carry only the shared VinUni header logo, but each college's own film
  on the official channel ends on its lockup (VinUni logo over the college name).
  `fetch-media.py` cuts those end cards out with a white key (`lockup-*.png`).
- Facts only from official sources, labelled by cohort, no year on screen.

## Timeline (seconds)

| # | Scene | Len | Chapter |
| --- | --- | --- | --- |
| 0 | dawn | 10 | |
| 1 | bridge 01 Who we are | 4 | 01 starts 10 |
| 2 | who | 8 | |
| 3 | mission | 8 | |
| 4 | stars | 8 | |
| 5 | bridge 02 What you study | 4 | 02 starts 38 |
| 6 | colleges | 8 | |
| 7-10 | engineering, health, business, arts | 8 each | |
| 11 | bridge 03 How you learn | 4 | 03 starts 82 |
| 12 | learn | 10 | |
| 13 | bridge 04 The world | 4 | 04 starts 96 |
| 14 | community | 8 | |
| 15 | globe (partner logos) | 10 | |
| 16 | bridge 05 Life on campus | 4 | 05 starts 118 |
| 17 | campus | 8 | |
| 18 | clubs | 6 | |
| 19 | values | 8 | |
| 20 | bridge 06 Your future | 4 | 06 starts 144 |
| 21 | outcomes: 99% and employer logos | 8 | |
| 22 | study: graduate study logos | 6 | |
| 23 | join | 8 | |
| 24 | rise | 10 | ends 180 |

## Facts

- Employers and graduate schools: as in the previous plan, and VinUni's own outcomes
  slides in "VinUniversity - Soaring | Graduation Day 2024" (BvOySZfXag8, 3:20-3:52): 25%
  of the first cohort invited to master's and doctoral study, 32% with job offers before
  graduation, with these institutions' logos.

## Footage

New clips from the official channel, all cut by `fetch-media.py`: the bridges (sunrise,
machine shop, lab teaching, hackathon, study pair, arrivals, golden aerial, horizon), a
third shot for health, business and arts from each college's own film, a pitch for how you
learn, basketball and the pool for the values, booths for the outcomes, sashes and flags
for graduate study, and the spire for the invitation. Covers allow 0.55 s past each scene
end where the next scene dissolves from it.

## Validation

Tests (180 s on the grid, chapters on cuts), clippy `-D warnings`, fmt, inspect every
0.25 s, the coverage check for every clip, frames of each bridge and logo wall, final render
and loudness.

## Results

- `cargo test` (180 s on the 2 s grid, every scene's length), clippy `-D warnings` and fmt
  pass; `inspect --every 0.25s` checked 745 frames with no problems; every clip covers its
  shot and dissolve.
- `vinuni-future-rises.mp4`: 1920x1080, 60 fps, 10800 frames, 180.0 s, H.264 + AAC stereo,
  rendered on the CPU in 196 s.
- Audio: -14.5 LUFS integrated, true peak -1.4 dBTP, no clipping; the bridges sit about 4 LU
  under the chapters around them, the only silence is the final fade.
