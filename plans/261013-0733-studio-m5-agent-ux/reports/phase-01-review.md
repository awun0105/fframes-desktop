# Phase 01 review (streaming markdown, link safety, CI)

Tests run: `cargo test -p studio-markdown` (53 pass), `cargo test -p studio-ui --lib agent::markdown` (49 pass). Clippy and the no-default-features build were not run.

## Security verdict
No path from a mouse event reaches `cx.open_url` without classify (http/https only) plus the confirm overlay. Trace: SelectRoot mouse-up -> `Selection::release` (drag and slop guarded) -> `LinkActivate` -> `activate(dest, root)` re-classifies the raw destination on every click -> `Activation::Confirm(url)` -> overlay shows `confirm.url` -> Open calls `accepted(&url)` with that same string. Files only ever reach `reveal_path` (gpui Windows: `SHOpenFolderAndSelectItems`, fallback opens the parent folder, never the file). `studio:pending-link` is Ignored in `activate`, unrecorded in `flatten`, and `studio` is a refused scheme anyway. Non-ASCII, userinfo, whitespace, control chars and backslash are rejected in `normalize_url`, so the confirm text has no bidi or confusable hosts. Findings below are about correctness and defence in depth.

## Findings (verified unless marked)

1. HIGH - project root is always cleared, file links never work. `agent/mod.rs` `attach()` starts with `self.detach(cx)`, and `detach()` now sets `project_root = None`. `shell/engine_events.rs:652-655` calls `set_project_root(root)` BEFORE `panel.attach(..)`, so the root is wiped immediately. `files` is therefore false for every row and project-file links are inert (the safe direction, but the feature is dead and no test covers the shell path). Fix: call `set_project_root` after `attach`, or carry the root in `Attachment`. Test idea: after adopt_opened, `panel.project_root.is_some()`.

2. MEDIUM - URL port and path suffix are silently stripped. `links.rs` `classify` runs `split_colon_line` on http(s) URLs before the scheme check. `http://localhost:8080` becomes `http://localhost`; `https://example.com:8443` loses the port; `https://example.com/a:12` loses `:12` (reproduced with a standalone rustc copy of the function). The confirm shows the stripped URL, so it is honest but opens the wrong destination (a dev-server link opens port 80). Existing tests only use URLs ending in `/`. Fix: check `scheme_of(dest)` on the unsplit string first and only call `split_colon_line` for non-http(s). Test: `classify("http://localhost:8080") == External{url:"http://localhost:8080"}`.

3. MEDIUM - veil keeps requesting frames forever for vanished elements. `veil.rs` `RowVeil.elems` entries are only trimmed inside `advance`, which runs only for elements that still exist. If a rewrite reduces the text-element count (a tail that parsed as a table then as a paragraph, a removed list item) a stale `ElemVeil` with live chunks stays in the map, `is_fading()` stays true, `veil_wanted` stays true after streaming ends and `request_animation_frame` fires every frame for the life of the row. Fix: after a build, `elems.retain(|ix, _| *ix < texts.len())`. Test: `advance(5,"abc",t0)`, never advance 5 again, `is_fading()` after 2s must be false.

4. MEDIUM - CI script ignores build failure. `scripts/grammar-build-cost.sh`: `timed_build` runs inside `$(...)`; bash clears `set -e` in command substitutions (no `inherit_errexit`), so a failing `cargo build` is not fatal, the function still echoes an elapsed time and the job can pass (verified with a minimal script that prints "continued"). A broken "with" build gives a tiny `with` and passes; a broken `--no-default-features` build gives a misleading error. Fix: `shopt -s inherit_errexit` after `set -euo pipefail`, or check cargo's exit status explicitly inside `timed_build`.

5. MEDIUM (perf) - table cells are re-shaped every frame. `render/table.rs::natural_width` calls `shape_line` for every cell on each `build`, which runs each frame (veil animation = every frame while streaming). Cost grows with table size and is outside the per-block `prepare` cache. Fix: cache column widths in `Prepared::Table` or per theme.

6. LOW - highlight spans are not char-boundary checked. `render/code.rs::highlighted` guards ordering and length but not `is_char_boundary`; `syntax/mod.rs` documents boundary-safe spans but `engine.rs` does not enforce it. If a grammar ever splits a multibyte char, `with_runs` gets a mid-char run (probable panic in gpui shaping; speculative). Cheap guard: skip spans where start or end is not a char boundary. Separately, the 128 KiB tree-sitter parse re-runs on every append of a streaming code block (the module doc says callers should coalesce; they do not) and there is no parse timeout.

7. LOW (defence in depth, partly speculative) - `accepted(url)` does not re-validate; it wraps any string. It is module-private and the URL is exactly the one displayed, so no exploit today. Re-run `classify` inside it and refuse anything but External. `normalize_url` also allows `"`, `<`, `>`, `^`, `|`, backtick and `'` in http URLs; on Windows `open_url` is `ShellExecuteW`, whose handlers have a history of quoting bugs. Consider refusing or percent-encoding them. Speculative.

8. LOW - Open button is not idempotent: `on_open` does not check `state.confirm` is still Some, so two clicks delivered before the re-render open the URL twice. Guard with `state.confirm.take().is_some()`.

9. LOW - press on plain text next to a link and release on the link within `CLICK_SLOP` (4 px) activates the confirm (release only checks `link_at(release_pos)`). Confirm only, no open. Fix: require `link_at(press_pos)` to equal the release link.

10. LOW (speculative) - keyed row state is keyed by `("agent-md", row_id)` and row ids restart at 1 per project log. `attach()` calls `detach()` and rebuilds with no frame in between when re-attaching, so a row of the new session could inherit the old `RowState` (pending `confirm` overlay, selection, veil seeding). The normal project-switch path renders an empty frame first, so state is dropped; the re-attach path is not proven. Fix: include the attachment serial in the key.

11. LOW - `ActiveCopy` global keeps the last selected text after its row is dropped (scrolled out of the virtualized list, thought collapsed); the copy shortcut then copies that stale text. Clear it when the row state is dropped.

12. LOW (CI hygiene) - `desktop.yml` `on.pull_request.paths` lacks `scripts/**`, so a PR that only changes `scripts/grammar-build-cost.sh` never triggers the workflow although `grammar-scope` lists it. The Linux leg uses the composite setup action, which turns the rust cache on, contradicting the "no cache" intent (harmless, wasted upload). The YAML reads as valid; the `${{ }}` expressions, the grep regex and `git diff base head` with `fetch-depth: 0` are correct.

## Checked, not issues
- Cache keyed by Arc ptr cannot alias: the cache holds the Arcs.
- Selection slicing: `range_for` floors to char boundaries and clamps; `press` indexes `texts[ix]` with `ix` from `entries`, which `begin_frame` sizes to `texts.len()`.
- `common_prefix` snaps to char boundaries; `apply_veil` and `wash` cut only at run/span boundaries.
- Images render as alt text only (no fetch); an image with empty alt is dropped.

Status: DONE_WITH_CONCERNS

## Resolution (lead, 2026-10-16)

| # | Finding | Outcome |
|---|---------|---------|
| 1 | Project root wiped by `attach()` | Fixed: `detach` no longer clears the root and the shell sets it after `attach`; no shell-level test |
| 2 | Port and `:NN` stripped from web URLs | Fixed: http(s) is recognised on the unsplit text; tests for `:8080`, `:8443`, `/a:12` |
| 3 | Veil frame loop for vanished elements | Fixed: `RowVeil::retain_elements` after each build; test added |
| 4 | Build-cost script ignores build failures | Fixed: the build returns 1 explicitly on failure (portable to bash 3.2 on macOS, unlike `inherit_errexit`); checked with a minimal script: exit 1, no continuation |
| 5 | Table cells shaped every frame | Fixed: `WidthCache` in the prepared table, keyed by the font |
| 6 | Highlight spans not boundary checked | Fixed: spans off a char boundary are skipped. Re-highlighting on each append stays (128 KiB cap) |
| 7 | `accepted` trusts its input; shell-sensitive characters | Fixed: `accepted` re-classifies; `" < > ^ \| \` { }` refused in URLs; tests added |
| 8 | Open not idempotent | Fixed: Open takes the pending confirmation and opens only when one was pending |
| 9 | Press beside a link, release on it shows the confirm | Not fixed (low): it never opens anything without the confirm |
| 10 | Keyed row state could survive a re-attach | Not fixed (speculative, unproven) |
| 11 | Copy shortcut keeps a dropped row's selection | Not fixed (low) |
| 12 | Workflow paths lack the script | Fixed: `scripts/grammar-build-cost.sh` added to `pull_request.paths`. The Linux leg's cache upload stays (composite action) |
