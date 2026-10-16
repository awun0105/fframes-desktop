# Syntax highlighting for agent code blocks

## API (`studio_ui::agent::syntax`)

```rust
pub const MAX_HIGHLIGHT_BYTES: usize = 128 * 1024;
pub enum Language { Rust, Toml, Json, Markdown, Shell }
impl Language { pub fn from_tag(tag: &str) -> Option<Self> }   // rs rust toml json jsonc md markdown sh bash shell zsh console; case-insensitive, first word of "rust,ignore" / "rust title=x"
pub enum TokenClass { Comment, Keyword, String, StringSpecial, Escape, Number, Boolean, Type, TypeBuiltin,
    Constructor, Function, FunctionBuiltin, Macro, Property, Constant, Variable, VariableBuiltin, Parameter,
    Operator, Punctuation, Tag, Attribute, Label, Heading, Raw, Link, Reference, Emphasis, Strong, Embedded, Invalid }
pub struct HighlightSpan { pub range: Range<usize>, pub class: TokenClass }   // byte range into `code`
pub fn highlight(tag: &str, code: &str) -> Vec<HighlightSpan>
pub fn highlight_language(language: Language, code: &str) -> Vec<HighlightSpan>
pub fn token_color(class: TokenClass, theme: &studio_theme::Theme) -> gpui::Rgba
```

- Spans are ordered, non-overlapping, non-empty, on char boundaries and within `code`. Uncoloured text has no span; adjacent same-class spans are merged. Innermost capture wins.
- Unknown tag, text over `MAX_HIGHLIGHT_BYTES`, parser failure, or feature off: empty `Vec`.
- studio-theme has no syntax palette, so `token_color` maps classes onto existing `Theme` fields (comment gray_10, keyword/operator/tag red_11, string green_11, number/constant amber_11, type blue_11, function blue_10, ...). Renderer: paint `code[range]` as a text run with that colour; text and layout are untouched.
- Markdown also colours inline spans (via markdown_inline) and fenced code inside it in the other four languages (one level).

## Cost / streaming

Grammar + query compile happens once per language (`OnceLock`), on first use. A `Highlighter` is reused per thread. Each call re-parses the whole text (no incremental reuse), so it is linear in block size. For a streaming block, call it per chunk, but coalesce to a few calls per frame for large blocks. An unterminated fence is parsed with error recovery (tested on every prefix of multibyte samples).

## Crates and feature

All pinned with `=` in `desktop/crates/studio-ui/Cargo.toml`, optional, versions taken from Zeron's workspace: tree-sitter 0.26.11, tree-sitter-highlight 0.26.11, tree-sitter-rust 0.24.2, tree-sitter-json 0.24.8, tree-sitter-bash 0.25.1, tree-sitter-toml-ng 0.7.0, tree-sitter-md 0.5.3. Feature `syntax-grammars` (default on) gates them; off, every call returns no spans. `desktop/Cargo.lock` gained 119 lines (the caller owns that file).

Files: `agent/syntax/mod.rs` (API, tags, colours), `agent/syntax/engine.rs` (tree-sitter, gated), `agent/syntax/tests.rs`.

## CI

Job `grammar-build-cost` in `.github/workflows/desktop.yml`: matrix `ubuntu-24.04`, `macos-15`, `windows-2025` (hosted; the self-hosted Windows runner is push-only in the other workflow, so it is not used here). Setup mirrors the existing jobs (Linux composite action; macOS and Windows steps copied from desktop-macos / desktop-release). Then `bash scripts/grammar-build-cost.sh`: `cargo fetch`, then two clean builds of `studio-ui --lib` into separate empty `CARGO_TARGET_DIR`s (`--no-default-features` first, then default), prints and appends to the step summary the seconds and percent, and exits 1 above `MAX_INCREASE_PERCENT` (default 20). The script file is not marked executable in git, hence `bash` in the step. Not run in CI from here; the YAML was parsed with PyYAML (4 jobs, 10 steps in the new one) and the script with `bash -n`.

Caveat: the unfavourable order bias is the other way round: the "without" build runs first, so a warm disk cache favours the "with" build. The 20% limit absorbs this; CI numbers per OS appear in each run's summary and are not recorded here.

## Measured locally (Windows VM, dev profile, run through the script, debug info as per local env)

Clean `studio-ui --lib`, separate target dirs: 110 s without grammars, 107 s with (-2%, i.e. within noise; the grammar C parsers add little next to the gpui/ffmpeg graph). Linux and macOS were not measured.

## Verification

- `cargo test -p studio-ui --lib agent::syntax`: 12 passed (default features); 5 passed with `--no-default-features`.
- `cargo clippy -p studio-ui --all-targets -- -D warnings`: clean, also with `--no-default-features`.
- rustfmt run on the three syntax files only.
