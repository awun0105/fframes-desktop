# studio-markdown port

Source: Zeron `crates/markdown` at 09beace0. Crate: `desktop/crates/studio-markdown`. Depends only on `pulldown-cmark` (workspace, no feature change).

## Modules (src/)
- `lib.rs` re-exports the public API; `mod tests` is cfg(test).
- `model.rs` (~140 lines): block tree types.
- `parse.rs` (~290): `parse_full`, event-to-block folding, tables, `Cursor`.
- `inline.rs` (~220): inline runs, autolinking, `merge_runs`, `heading_level`.
- `destinations.rs` (~270): angle-bracket rewrite for absolute link destinations with spaces, with offset mapping.
- `incremental.rs` (~190): `IncrementalParser`.
- `mend.rs` (413): `close_hanging`, `PENDING_LINK_URL`, own unit tests.
- `tests/`: `parser.rs`, `closing_quote.rs`, `destinations.rs`, `image_model.rs` (Zeron's tests, unchanged), `streaming.rs` (new).

## Public API
- `parse_full(source: &str) -> BlockTree`
- `BlockTree { blocks: Vec<Arc<TopBlock>> }`, `is_empty()`, `len()`
- `TopBlock { range: Range<usize>, block: Block }`
- `Block::{Paragraph{runs}, Heading{level:u8,runs}, CodeBlock{language:Option<String>,code}, BlockQuote{children}, List{ordered_start:Option<u64>,items:Vec<Vec<Block>>}, Table{header,rows,align:Vec<TableAlign>}, Rule}`
- `TableAlign::{Left,Center,Right}`
- `InlineRun { text, style: InlineStyle }`
- `InlineStyle { bold, italic, code, strikethrough, link: Option<String>, file_label: Option<String>, image: Option<InlineImage>, task: Option<TaskMarker> }`
- `InlineImage { source, alt, title, link }`, `TaskMarker { checked, range }`
- `IncrementalParser`: `new()`, `source() -> &str`, `tree() -> &BlockTree` (canonical, equals `parse_full`), `display_tree() -> BlockTree` (last block mended for hanging inline markers; use for rendering while streaming), `set_text(&str)`, `append(&str)`, `reset(&str)`, `last_parse_bytes() -> usize` (re-parsed-byte counter, already existed in Zeron), `stable_prefix_blocks() -> usize` (leading blocks whose render caches stay valid).
- `mend::close_hanging(text: &str) -> Option<String>`, `mend::PENDING_LINK_URL = "studio:pending-link"` (renderer must treat this destination as non-clickable).

## Tests
53 pass: 46 ported (incl. mend's) plus 7 new in `tests/streaming.rs`:
- recorded reply (headings, nested list, table, Rust fence, blockquote, tasks, link) streamed at six chunk-size/seed combos down to 1 byte; canonical tree equals `parse_full` after every chunk;
- explicit cuts inside the fence and inside a table row;
- display tree equals full parse once complete; multibyte text;
- generated 2,000-line reply (~21 KB) in <=48-byte chunks: `last_parse_bytes()` stays <= 1,024 in both halves of the stream;
- link-reference definition falls back to full reparse.

`cargo test -p studio-markdown`, `cargo clippy -p studio-markdown --all-targets -- -D warnings` and `cargo fmt -p studio-markdown` are clean; `missing_docs` warns on nothing.

## Changes and drops
- Renamed the pending-link sentinel from `zeron:pending-link` to `studio:pending-link`; zeron wording removed from docs.
- Internal helpers became `pub(crate)`; `parser` and `mend` are no longer both public modules (only `mend` is; parser types are re-exported at the crate root).
- Nothing functional dropped. Zeron's test URLs (github.com/zeronsh/...) kept as plain data.
- Cargo.lock gained entries from other work in the worktree, not from this crate.
