# Link safety (agent markdown)

File: desktop/crates/studio-ui/src/agent/markdown/links.rs (12 tests pass, clippy clean; symlink test ran, not skipped).

## API
- `classify(destination: &str) -> LinkTarget` (pure, no fs). The `project_root` argument was dropped: the lexical check needs none.
- `LinkTarget::{External{url}, ProjectFile{path: ProjectPath, line: Option<u32>}, NotClickable(NotClickable)}`
- `NotClickable::{Empty, Scheme(String), BadUrl, Network, Absolute, OutsideProject, Fragment, Unavailable}` + `.message()`
- `action_for(&LinkTarget, project_root: &Path) -> LinkAction` (only fs step)
- `resolve(destination, project_root) -> (LinkTarget, LinkAction)`: a ProjectFile failing the disk check is returned as `NotClickable::Unavailable`, so the renderer styles it as plain text.
- `LinkAction::{ConfirmExternal(url), Reveal(PathBuf), None}`
- `effect(&LinkAction, confirmed: bool) -> Option<LinkEffect>`; `LinkEffect::{Reveal(PathBuf), OpenUrl(String)}`
- `perform(&LinkEffect, cx: &mut gpui::App)`: `cx.reveal_path` / `cx.open_url`.

Renderer contract: call `resolve` at render (or click); on `ConfirmExternal(url)` show the full url, and call `effect(&action, true)` only after the user accepts; `Reveal` goes straight to `effect(&action, false)` + `perform`.

## Security decisions
- No default-verb open exists in the API: `LinkEffect` has no Open-file variant. Any extension (.cmd/.exe/.bat/.ps1/.lnk/.html) maps to Reveal (tested).
- External: only http/https (any case; scheme lowercased). Rejected as BadUrl: userinfo (`@`), empty host, missing `//`, non-ASCII (homograph), whitespace/control/backslash, odd host chars. Full URL is kept for display.
- Every other scheme is NotClickable (file, vscode, javascript, data, ms-settings, mailto, ftp, ...); single-letter scheme = drive = Absolute.
- UNC (`\x`, `//x`), leading `/` or `\`, drive paths: NotClickable. Percent-decoding is applied to the path before validation, so `%2e%2e`, `%2f%2f`, `%5c`, `%3a`, `%00` are caught.
- Paths go through `ProjectPath::try_from` (no `..`, backslash, `:`, reserved names, empty components). `./` prefix is stripped.
- On disk, `ProjectPath::resolve_existing` is used: each component checked with symlink_metadata (links/reparse points rejected), canonical result must stay under the root; missing file -> Unavailable. The revealed path is `project_root.join(components)` (not the `\?\` canonical form).
- Line hints: `path:12`, `path:12:5`, `path#L12`, `#L12-L20`, `#L12C3` (display-only; Reveal ignores them). Other fragments -> NotClickable(Fragment). `javascript:1`-style (scheme + digits) is read as a relative file name `javascript`, which is harmless (reveal only if such a file exists).
- Not done: confirmation UI (the renderer owns it); existing `provider_setup::open_link` is a separate https-only path with no confirmation.

Status: DONE
