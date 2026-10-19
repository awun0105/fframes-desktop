---
status: done
created: 2026-10-18
follows: ../261018-1948-vinuni-brand-assets/plan.md
---

# Graduate outcomes, more motion, more footage

## Outcome

- A new scene shows where VinUni graduates go: the share employed or in further study,
  the companies they work at, the universities that invited them to graduate study.
- Text, cards, lockups and logo plates move more: masked line reveals, tracking-in labels,
  card flips with a light sweep.
- New footage from the official YouTube channel: the graduation and Career Day films.

## Constraints and non-goals

- Still 120 s on the 2 s grid, six chapters, CPU render.
- Facts only from official VinUni sources, each figure labelled with its cohort, no year
  on screen. Companies and universities as names, not logos (no official logo files,
  and logos read as endorsement).
- No new storyline; the outcomes scene sits in chapter 06 "Your future" before Join.

## Facts used

| On screen | Source |
| --- | --- |
| 99% of Cohort 2 graduates employed or in further study within six months | Annual Report 2025 (vinuni.edu.vn/wp-content/uploads/2025/04/Anual_Report_2025_19_8.pdf): stated twice, as a clean sentence on the overview page ("99% of Cohort 2 graduates secured employment and/or progressed to further study within six months after graduation") and on the graduate outcomes page |
| "Job offers from" Google, Qualcomm, BCG, Unilever, P&G, McKinsey, IBM, Bosch "and top Vietnamese technology companies and banks" | Annual Report 2025: students "employed by" Google, Qualcomm, BCG, Unilever, P&G; "VinUniversity graduates the first academic cohort": "job offers from" McKinsey, BCG, Google, Bosch, IBM "as well as top Vietnamese technology companies and financial institutions". Worded "job offers", not "work at", because the article only reports offers |
| Invited to graduate study at Harvard, Cornell, Penn, NUS, NTU, Duke, EPFL | "VinUniversity graduates the first academic cohort" (first cohort, "invitations") |

Rejected: 25%, 90%, "20% higher salary", 8% (the annual report's text for them is
scrambled, numbers and labels may not pair); 96.3% and the median salary (Class of 2024
outcomes PDF, text not extractable); 55% vs 60% offers (two measures, conflicting).

## Timing

Engineering, Who, Mission and Join go from 8 s to 6 s (their overlays land before the
new end), the outcomes scene takes the 8 s between Values and Join.

| Chapter | Before | After |
| --- | --- | --- |
| 01 Who we are | 10–34 | 10–30 |
| 02 What you study | 34–68 | 30–62 |
| 03 How you learn | 68–76 | 62–70 |
| 04 The world | 76–88 | 70–82 |
| 05 Life on campus | 88–106 | 82–100 |
| 06 Your future | 106–114 | 100–114 |

## Footage

| Clip | Source | Shot |
| --- | --- | --- |
| `gowns.mp4` | dPWZDM5HujQ (Graduation, "Legacy of Excellence") 300.98–302.45, x1.63 | graduates in gowns, close (the span between two whip transitions) |
| `careerday.mp4` | afS5OOQWReA (Career Day) 139.50–142.25, x1.09 | students with brochures at the fair |
| `hallgrads.mp4` | dPWZDM5HujQ 304.50–307.20, x1.24 | the hall of graduates standing |

The graduation film is letterboxed with subtitles under the picture: crop the 16:9 window
inside the picture band (79%). Rejected: shots with "Graduation 2025", "Class of 2024" or
"Career Day 2024" titles and banners, named speaker interviews, the outcomes slide with
third-party logos.

## Validation

`cargo test` (120 s, cuts on the grid, chapters on cuts), clippy `-D warnings`, fmt,
`inspect --every 0.25s`, frames of every changed scene, the coverage check, final render
loudness.

## Result

- Tests 5/5, clippy `-D warnings` clean, `inspect --every 0.25s`: 498 frames, no problems.
- Coverage: every clip runs past its next cut and scene end (outcome clips recut at
  2.4 / 3.0 / 3.35 s to clear it).
- `vinuni-future-rises.mp4`: 1920x1080, 60 fps, 7200 frames, rendered on the CPU in 138 s;
  -14.1 LUFS integrated, true peak -1.7 dBTP, no clipping.
- The 99% figure is Cohort 2's and the names come from the first cohorts: refresh both
  when a newer outcomes report is published.
