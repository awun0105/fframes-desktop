---
phase: 2
status: todo
---

# Phase 2: new repository on crates.io fframes

## Steps

- [ ] `git subtree split --prefix=components` into `C:\Users\Administrator\fframes-kit`.
- [ ] Move `docs/components/*.md` to `fframes-kit/docs/` (inside the package, for `include_str!` and `cargo package`) and `docs/latex-setup.md` to `docs/`; fix the paths and the LaTeX error text.
- [ ] Workspace deps to crates.io (`fframes`, `fframes_skia_renderer`, `fframes-editor-controller` 1.3.0); drop `boundary`; `package` without `--no-verify --exclude-lockfile`; `repository` URL; README, LICENSE, `.gitignore`.
- [ ] Verify: tests, clippy, `check-wasm`, `e2e cpu` (attribute any drift to the svgr source before touching snapshots), `cargo package`.
- [ ] Create the private GitHub repo and push.
