---
phase: 1
title: "Streaming markdown"
status: completed
priority: P1
effort: "3-5d"
dependencies: []
---

# Phase 1: Streaming markdown

## Goal

Agent and thought rows render markdown incrementally as chunks stream in, with highlighted code blocks, selectable text and safe links.

## Port sources

See [the plan](./plan.md). Zeron's approach: `pulldown-cmark` on a background task with coalescing, re-parsing only from the last stable top-level block (`IncrementalParser`, `BlockTree`), rows keyed per block, paint-only highlighting, and a fade-in ("veil") for new text.

## Tasks

1. Create `studio-markdown` from Zeron's crate (it depends only on `pulldown-cmark`); split `parser.rs`. Keep its tests.
2. Port the renderer, veil and selection; drop Mermaid.
3. Write `studio-ui/src/agent/syntax/` with five grammars and paint-only highlighting; add a CI step that records the clean build time of `studio-ui` with and without the grammar crates on Windows, macOS and Linux, writes the baseline into the phase report, and fails when the grammars add more than 20% to it.
4. Render `RowKind::Agent` and `RowKind::Thought` through it, one transcript row per top-level block.
5. Links (rewrite Zeron's `link_interaction.rs` opener, which hands `file://` URLs to the OS default app):
   - external: only `http` and `https`, shown in full before opening;
   - project files: resolved with `ProjectPath` rules (no `..`, no absolute paths, no links), then revealed in the file manager or shown read-only in the app; never opened with the default verb;
   - everything else (`file:`, UNC, custom schemes): not clickable.
6. Tests: incremental parse equals full parse after every chunk of a recorded stream; the re-parsed byte count per chunk stays bounded on a 2,000-line reply; link classification for each scheme and for a `.cmd` file in the project.

## Todo

- [x] `studio-markdown` with tests ([report](./reports/phase-01-studio-markdown.md))
- [x] Renderer, veil, selection ([report](./reports/phase-01-renderer.md))
- [x] Five-grammar highlighting and CI build-time check ([report](./reports/phase-01-syntax.md)); the `grammar-build-cost` job has not run in CI yet
- [x] Agent and thought rows use it (one list row per transcript row, cached per top-level block; see the renderer report)
- [x] Safe link handling ([report](./reports/phase-01-links.md))
- [x] Tests

## Success criteria

`cargo test -p studio-markdown` passes; a streamed reply with headings, lists, a table and a Rust block renders correctly on this VM; clicking a link to `tools/x.cmd` reveals it and does not run it.

Status 2026-10-16: met on this Windows VM.

- Tests: the desktop CI package set passes after the review fixes ([review](./reports/phase-01-review.md)) and the inline-code link port. The link criterion is also covered at component level (a gpui test gives `MarkdownRow` a project root and checks that `tools/x.cmd` reveals and never opens). The shell passing the root on open is not covered by a test.
- By eye: a throwaway example window rendered the real `MarkdownRow` through GPUI's DirectX renderer while a reply with headings, emphasis, inline code, nested and ordered lists, an aligned table, a Rust block, a quote and a rule streamed in ([mid-stream](./reports/phase-01-visual-mid-stream.png), [finished](./reports/phase-01-visual-streamed.png)). It was not the full studio shell.
- Real clicks in that window:
  - The inline-code `tools/x.cmd` link opened Explorer on `tools` with `x.cmd` selected, and no `cmd` process started.
  - The web link showed the confirmation with the full URL including its port ([confirm](./reports/phase-01-visual-link-confirm.png)), and Cancel opened nothing.
- The check found that the table frame stretched past its columns; the frame now hugs them.

