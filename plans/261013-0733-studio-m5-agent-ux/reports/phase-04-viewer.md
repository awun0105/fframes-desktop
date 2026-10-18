# Phase 4 task 5: change viewer (viewer only)

Standalone gpui view in `desktop/crates/studio-ui/src/agent/changes/`. No engine, Cargo.toml or Cargo.lock edits. Nothing is wired into the shell yet.

## Module layout (lines)

| File | Lines | Holds |
|---|---|---|
| `mod.rs` | 525 | `Changes` view, `ChangesEvent`, `Review`, toolbar, file header and row dispatch, virtualised `list` |
| `model.rs` | 652 | `DiffFile` input, `FileDiff`/`Hunk`/`DiffLine` built with `similar`, `split_pairs`, `DiffRow` + `flatten_rows`, tab/width expansion, 13 unit tests |
| `paint.rs` | 406 | colours from `studio_theme`, row metrics, `RowStyle`, per-line syntax spans, `line_runs`, shared `Cell` (accent bar, gutters, marker, code), 4 unit tests |
| `unified.rs` | 45 | unified row |
| `split.rs` | 70 | split row (two halves, filler, divider, marker row spanning both) |
| `file_list.rs` | 120 | file list: +/- counts, accept toggle (kit `Toggle`), jump to file |
| `tests.rs` | 195 | 8 gpui tests |

All under 800 lines (limit 1,500).

## Public API

Input (named `DiffFile`, not `FileDelta`, to avoid the engine type):

```rust
pub struct DiffFile { pub path: String, pub before: Option<String>, pub after: Option<String>, pub not_shown: Option<NotShown> }
pub enum NotShown { Binary, TooLarge }
DiffFile::new(path, before, after); DiffFile::binary(path); DiffFile::too_large(path)
```

`None` is absent on that side (added file has no `before`). Text over 2 MiB on either side becomes `TooLarge` automatically. Not-shown files are listed with a "not shown" notice and are never diffed.

View: `Changes::new(Vec<DiffFile>)`, `set_files(files, cx)` (keeps collapse and reject choices for paths still present), `set_mode(DiffMode::{Unified,Split}, cx)`, `mode()`, `set_review(Review::{Pending,Applied}, cx)`, `set_wrap(bool, cx)`, `wrap()`, `rejected() -> Vec<String>`.

Events (`impl EventEmitter<ChangesEvent> for Changes`; the viewer performs none of them):

- `FileToggled { path, accepted }`: a file's toggle was flipped (Pending only).
- `Reject(Vec<String>)`: toolbar "Reject N" button, every file currently toggled off.
- `RevertFile(String)`: per-file "Revert" button in the file header (Applied only).
- `Close`.

`pub use` of `DiffFile`, `DiffMode`, `NotShown` from `changes`; `Changes`, `ChangesEvent`, `Review` are defined in `changes`.

## Behaviour

- Diff: `TextDiff::configure().timeout(1s).diff_lines`, `grouped_ops(3)`, git-style `@@ -a,b +c,d @@` headers. Line terminators are stripped from display text but compared, so a CRLF to LF change shows as modified lines plus an "Only line endings changed" notice. A last line without a newline gets a `\ No newline at end of file` marker row (paired across sides in split mode). Identical text gives a "No changes" notice row; added and deleted files get "New file" / "Deleted file".
- Modes: unified (old and new gutters) and split (paired per change block, filler on the empty side), toggled with the kit `SegmentedControl`. Per-file collapse by clicking the header; "Collapse all / Expand all" button. Long lines wrap by default (rows grow, `gpui::list` measures them); the "No wrap" toolbar button clips at the column edge instead.
- Highlighting: `syntax::highlight(extension, whole file text)` per side, spans cut per line, converted to `TextRun`s coloured by `syntax::token_color`. Text layout is unchanged (paint-only). Tabs are expanded to 4-column stops with `unicode-width`, and run lengths follow the expanded text. rs, toml, json, md, sh work; other extensions are plain.
- Large diffs: rows are flattened to a `Vec<DiffRow>` and drawn through `gpui::list` with `ListState` (only visible rows build). Files past 2 MiB are not diffed.

## GPUI adaptations

- Chrome uses kit `Button`, `SegmentedControl::editor`, `Toggle::plain`, `icon`, `Icon`, and `Theme.editor`/`green_11`/`red_11` colours; no frost, no Zeron theme struct.
- Zeron computes nothing (it parses patch text); `parse_patch` is not ported. The `FileDiff`/`Hunk`/`DiffLine` model, `split_pairs` and `DiffRow`/`flatten_rows` are re-created from before/after text.
- Rows use the fork's `list(ListState, cx.processor(..))` as the transcript does; `ListState::scroll_to` jumps to a file.
- `markdown::style` is private, so the monospace family and code size are two small constants in `paint.rs` (same values as `MONO_FAMILY`/`CODE_SIZE` there).
- The nested Revert button and the accept toggle stop click propagation so they do not also collapse the file or scroll the diff.

## Zeron function mapping

| Zeron | Here |
|---|---|
| `parse_patch`, `FileDiff`, `Hunk`, `DiffLine` | not ported as a parser; same model built by `FileDiff::compute` (`similar`, `grouped_ops(3)`) |
| `split_pairs` | `model::split_pairs` (no `max_rows` variant, no fold tween) |
| `body_rows`, `flatten_rows`, `DiffRow` | `model::flatten_rows`, `model::DiffRow` (no comment rows) |
| `diff_line_row`, `readonly_diff_line` | `unified::row` over `paint::Cell` |
| `split_line_cell`, `split_filler`, `split_row`, `readonly_split_line` | `split::row` and `split::half` over `paint::Cell`; marker rows span both halves |
| `hunk_header_row` | the `HunkHeader` arm of `Changes::row` (mono 11px, faint text, tinted band) |
| `meta_line_row` | `paint::meta_row` |
| `code_text_viewport` (wrapped path) | `paint::code`; the scroll-handle path is not ported |
| `render_file_body_with_syntax`, `line_runs` | `Changes::row` plus `paint::line_runs` and `Highlights` |
| `file_notices`, notice row | `FileDiff.notices` and the `Notice` row |

## Dropped

Review comments and comment UI, history and commit scopes, ref menus, `WatchCheckoutDiffs`/RPC, popovers, sticky blurred header (frost), fold tween, settings-driven font size, per-file horizontal scroll (wrapping or clipping replaces it), `truncate_file_lines`. Zeron has no intra-line emphasis, so none was added.

## Tests (25, `cargo test -p studio-ui --lib agent::changes`: all pass)

- Model: add, delete, modify, added file, deleted file, identical file, hunk grouping with 3 context lines, CRLF vs LF, CRLF to LF notice, no trailing newline markers, unicode-width and tab columns, binary and too-large, split pairing, flatten with collapse.
- Paint: spans cut at line ends and `\r`, grammar vs unknown extension, runs cover the expanded text exactly.
- gpui: unified rows render for two files; clicking the Split segment switches to split rows; toggling a file off emits `FileToggled` with its path and the toolbar then emits `Reject([path])`; a no-change file shows its "No changes" notice row; Close and Revert emit events (and Revert does not collapse); collapse and collapse-all row counts; wrap on by default and the toolbar turns it off; a line row from each of two diffed files is laid out; `set_files` keeps choices for surviving paths.

Clippy was clean for the crate before the concurrent composer files broke it (see concerns); rustfmt ran on the changes files only.

Status: DONE_WITH_CONCERNS
Summary: Viewer is complete and tested; the engine mapping into `DiffFile` and shell entry from the Changes row remain for later tasks.
Concerns/Blockers: not visually verified on screen, only through gpui layout tests. Final `cargo clippy -p studio-ui --all-targets` could not complete because the concurrent composer files (`composer/metrics.rs`, `composer/tests.rs`) fail it; no diagnostic points at `changes/`, and clippy was clean for the whole crate before they broke.
