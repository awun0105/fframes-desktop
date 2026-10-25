---
phase: 3
status: completed
---

# Phase 3: rewire and remove here

## Steps

- [x] `fframes-studio-kit`, the kit-props fixture and `examples/fframes-studio-intro` depend on `fframes-kit` by git (pinned `rev`).
- [x] Remove `components/`, `docs/components/`, `docs/latex-setup.md`; drop `components` from the root `exclude`; point `AGENTS.md` at the new repo.
- [x] Verify: the fixture and studio-intro build, `kit_props_roundtrip` passes.
- [x] Commit only this change: root `Cargo.toml` and the desktop docs are staged from HEAD plus the edit, and the lock files are generated in a clean worktree under `C:\hc`, because the working copies carry the user's svgr override and other edits.
