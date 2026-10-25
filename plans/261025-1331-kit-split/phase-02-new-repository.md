---
phase: 2
status: completed
---

# Phase 2: new repository on crates.io fframes

## Steps

- [x] `git subtree split --prefix=components` into `C:\Users\Administrator\fframes-kit`.
- [x] Move `docs/components/*.md` to `fframes-kit/docs/` (inside the package, for `include_str!` and `cargo package`) and `docs/latex-setup.md` with them; fix the paths and the LaTeX error text.
- [x] Workspace deps to crates.io (`fframes`, `fframes_skia_renderer`, `fframes-editor-controller` 1.3.0); drop `boundary`; `package` without `--no-verify --exclude-lockfile`; `repository` URL; README, LICENSE, `.gitignore`.
- [x] Verify: tests, clippy, `check-wasm`, `e2e cpu` (attribute any drift to the svgr source before touching snapshots), `cargo package`.
- [x] Create the private GitHub repo and push.

## Result

All 167 gallery frames match their snapshots at 0.000% on crates.io fframes and registry svgr. The new repo needs `.gitattributes` (`eol=lf`): a CRLF checkout breaks the gallery's `.vtt` parser.
