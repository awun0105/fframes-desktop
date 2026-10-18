# Phase 1 report: markdown renderer and transcript hookup

Agent and thought rows render through `studio-ui/src/agent/markdown/` (`MarkdownRow`). The port
of Zeron's `render.rs`, `veil.rs` and `selection.rs` (09beace0) was written by a delegated agent
that was stopped before it reported; this report was written from the code it left, after the
lead finished the hookup and ran the checks.

## Module layout (`studio-ui/src/agent/markdown/`)

| File | Role |
|------|------|
| `render/mod.rs` | `MarkdownRow` (RenderOnce) and its keyed per-row state: `IncrementalParser`, one prepared value per top-level block, veil, selection, pending link confirmation |
| `render/blocks.rs` | headings, paragraphs, lists, quotes, rules; per-block prepare and build |
| `render/code.rs` | code panel with language label, Copy, and `syntax::highlight` colours |
| `render/table.rs` | tables with column alignment |
| `inline_code_links.rs` | code spans that name an existing project file become file links (see Inline code links) |
| `inline.rs` | runs flattened into one text with styled segments and link spans; only links the policy acts on are styled |
| `text_element.rs` | `MdText` (layout, selection wash, veil as run colours) and `SelectRoot` (mouse, link clicks, copy) |
| `selection.rs` | selection across the text elements of one row |
| `veil.rs` | fade-in of newly arrived bytes; colour only, never layout; off under `reduced_motion()` and for rows that are not streaming |
| `link_ui.rs` | `activate` (Perform / Confirm / Ignored) and the confirmation overlay showing the full URL |
| `style.rs` | colours and sizes from the Cap theme |

Mermaid was dropped. Zeron's `link_interaction.rs` opener was not ported; clicks go through
`links.rs` (see `phase-01-links.md`).

## Row model decision

One transcript list row per agent or thought row, not one per top-level block. Each top-level
block keeps a cached prepared value keyed by the parser's `Arc<TopBlock>`, so an appended chunk
re-prepares only the tail block (test `appending_chunks_only_prepares_the_tail`). Splitting a
row into several `gpui::list` items would change the row identity and scroll-anchoring rules in
`transcript/rows.rs`, which the plan allowed falling back from. Virtualising inside a very long
single reply is therefore not done; a reply is measured as one list item.

## Hookup

- `cards.rs`: `RowKind::Agent` and `RowKind::Thought` render `MarkdownRow` (`Voice::Answer` /
  `Voice::Reasoning`).
- `ConversationPanel::set_project_root` (new) gives the rows the project folder; the shell sets
  it from `presentation.project.root` after it attaches a workflow; `detach` leaves it, since it clears the rows.
  Without a root, project-file links are plain text. Links resolve against the user's project, not the task's draft, so a file the agent created in the current task stays plain text until it is applied. A setter was used instead of a new
  `Attachment` field so the constructors in the parity harness stay unchanged.

## Tests (`markdown/tests.rs`, gpui)

One element per block in order for a reply with a heading, list, table and Rust block; appending
chunks prepares only the tail; a link still streaming in is not clickable; a refused destination
is not clickable; an external link shows the full URL and opens only after accept; a
`tools/x.cmd` link reveals and never opens; project links do nothing without a project; a drag
across blocks selects and copies in order; streamed text fades; a finished row never fades.

## Known gaps

- Inline code links: a file created after its span was probed stays plain until the row is rebuilt; backslash, absolute and `~` spans never link; complete spans in a block the mend rewrote wait until that block settles.
- Checked by eye on this VM in a throwaway window rendering `MarkdownRow` (see the phase file), not inside the full studio shell.
- No per-block list virtualisation inside one long reply (see row model).
- Highlighting re-parses a growing code block on each chunk that changes it (linear in block
  size, capped at 128 KiB by the syntax module).

## Inline code links

`inline_code_links.rs` ports Zeron's rewrite of code spans that name an existing file (`` `tools/x.cmd` ``,
`` `src/main.rs:12` ``) into the link a markdown link to the same path would be. A resolved span
shows its file name (as many trailing components as it takes to tell two paths of one block
apart, `:line` kept); a directory span is not a link but is the context for later bare names in
the same top-level block; spans with nothing behind them stay inline code.

- Policy: a span is turned into a destination and goes through `links::classify` (so `..`,
  absolute, drive, UNC, scheme, backslash and reserved names never become candidates), and the
  disk check is `links::action_for` (`Reveal`), which carries the symlink and reparse checks.
  There is no second validator. The only addition to `links.rs` is `without_location`. A link
  made from a span is an ordinary `LinkSpan`, so a click is `activate`, then `links::resolve`,
  and a script is revealed, never opened. Without a project root nothing is linked.
- No filesystem in render: `link_block` is pure and asks a lookup (`Probes`, per row) for the
  kind of each candidate path. An unanswered path leaves the span as plain code and is
  collected; `RowState::probe` claims those paths and checks them with `kind_on_disk` on the
  gpui background executor, then stores the answers and notifies the row. Spans in a block that
  the mend changed (a block still being written, differing from the canonical parse) are not
  looked up, so only complete spans are probed.
- Caching: each cached block keeps the paths its spans asked about with the answers it saw. A
  block is reused only while those answers still hold, so an arriving answer re-prepares only
  the blocks that asked (test `an_answer_re_prepares_only_the_blocks_that_asked`). Answers are
  per row and per project root (a root change clears them; late answers for an old root are
  dropped), capped at 512 per row (answers plus probes in flight; past that nothing more is probed and the spans stay code), with at
  most 64 spans resolved and 8 context directories per block. A file that appears after its
  span was probed stays plain until the row is rebuilt.
- Dropped from Zeron: roots list and remote chats, absolute and `~/` paths (the project root is
  the only root), `file://` targets, the part-wide cache keyed by link-root revision, and
  copy-as-path (the label is the selected text; the destination is not copied).
