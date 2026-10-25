---
phase: 3
status: todo
---

# Phase 3: rewire and remove here

## Steps

- [ ] `fframes-studio-kit`, the kit-props fixture and `examples/fframes-studio-intro` depend on `fframes-kit` by git (pinned `rev`).
- [ ] Remove `components/`, `docs/components/`, `docs/latex-setup.md`; drop `components` from the root `exclude`; point `AGENTS.md` at the new repo.
- [ ] Verify: the fixture and studio-intro build, `kit_props_roundtrip` passes.
- [ ] Commit from a clean worktree under `C:\hc` (root `Cargo.toml`/`Cargo.lock` carry the user's svgr override).
