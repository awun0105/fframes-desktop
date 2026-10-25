---
phase: 1
status: in-progress
---

# Phase 1: seam and Studio adapter

## Steps

- [ ] Kit: own `PropKind`, `PropField`, `PropsSchema`; add `PropsSource` and `EditableObject`; `Ui` holds `Option<Arc<dyn PropsSource + 'a>>`, `with_props(source)`; drop `PropsStore`, `PropsView`, the protocol re-exports, `EditorObjectKey`, `editor_instance_key`, `scene_key`; drop the `fframes-studio-protocol` dependency.
- [ ] Gallery `tests/props.rs`: prove the seam with an in-test `PropsSource`.
- [ ] `fframes-studio-kit/` (own workspace, patch to the fork's fframes): `PropsStore` (load, set, document, listen, info), `view()`, `scene_view(&scene)`, schema conversion, render ids; the store tests.
- [ ] Fixture `desktop/fixtures/kit-props`: use the adapter.
- [ ] Verify: kit tests (`--all-features`), clippy, adapter tests, `kit_props_roundtrip`, a scratch build of the kit with `studio` against crates.io `fframes` 1.3.0.
- [ ] Commits: `refactor(kit): ...` (components/, docs/components/), `feat(studio-props): ...` (adapter, fixture).

## Risk

The roundtrip test is the only end-to-end proof of the props panel; it must pass before phase 2.
