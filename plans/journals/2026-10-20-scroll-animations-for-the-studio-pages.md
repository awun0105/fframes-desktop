---
title: Scroll animations for the Studio pages
date: 2026-10-20
summary: "Six Studio figures animate on scroll (hero, loop, filters, select, timing, swap) from hooks emitted by scripts/build-studio-figures.mjs. studio.js picks scrub (900px+, view() and timeline-scope) or play once; nothing animates without its class, under reduced motion or in print. Static, print and final frames verified against a pre-change capture in headless Chrome; Safari and Firefox untested. Design and lab: plans/reports/design-261020-1617-studio-scroll-animations.md."
---

# Scroll animations for the Studio pages

Six Studio figures animate on scroll (hero, loop, filters, select, timing, swap) from hooks emitted by scripts/build-studio-figures.mjs. studio.js picks scrub (900px+, view() and timeline-scope) or play once; nothing animates without its class, under reduced motion or in print. Static, print and final frames verified against a pre-change capture in headless Chrome; Safari and Firefox untested. Design and lab: plans/reports/design-261020-1617-studio-scroll-animations.md.

> Historical work record — not durable authority. Prefer docs/specs/ADRs for current decisions.
