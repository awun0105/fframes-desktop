---
title: "Move fframes-kit to its own repository"
description: "Cut the kit's tie to the fork (Studio props through a kit-owned trait, a Studio adapter crate here), move components/ to github.com/khanhthanhdev/fframes-kit building against crates.io fframes, then depend on it by git."
status: completed
priority: P1
branch: main
tags: [refactor, kit, studio]
created: 2026-10-25
---

# Move fframes-kit to its own repository

## Outcome

`fframes-kit` lives in the private repository `github.com/khanhthanhdev/fframes-kit` (history
kept with `git subtree split`), builds and tests against crates.io `fframes` 1.3.0 with every
feature, `studio` included, and is packageable (`cargo package`). fframes Studio stays in
this repository and keeps its props panel through a new adapter crate, `fframes-studio-kit`.

## Decisions (user, 2026-10-25)

| Decision | Choice |
| --- | --- |
| Kit dependency on fframes | crates.io `fframes`; the fork's `[patch.crates-io] fframes = { path = .. }` makes kit and Studio share one `fframes` here |
| Studio glue | moves out of the kit into `fframes-studio-kit/` (this repo, own `[workspace]`, scope `studio-props`) |
| New repository | GitHub, private, `khanhthanhdev/fframes-kit` |
| History | kept: `git subtree split --prefix=components`, docs moved in one commit |
| `components/intro/` | dropped (deleted 2026-10-25) |
| After the move | `components/`, `docs/components/`, `docs/latex-setup.md` leave this repo; consumers use the git dependency |

## The seam

- The kit keeps the feature `studio`, `#[derive(Props)]`, `PropValue`, `Editable`, and owns
  `PropKind`, `PropField`, `PropsSchema` (same shape as the protocol's).
- The kit defines `PropsSource`: the values that override an object, whether to collect code
  values, a record of how it rendered, and the id of the `<g>` it is drawn in. `Ui` owns an
  `Arc<dyn PropsSource + 'a>`, so a view made in `render_frame` keeps the per-`Ui` snapshot.
- The scene part of the identity comes from the source: `store.view()` (video) or
  `store.scene_view(&scene)` (uses the fork's `editor_instance_key`).
- `fframes-studio-kit` owns `PropsStore`, the views, the conversion to protocol types and
  `EditorObjectKey::render_id`.

## Phases

| # | Phase | Status |
| --- | --- | --- |
| 1 | [Seam and Studio adapter](./phase-01-seam-and-adapter.md) | Completed |
| 2 | [New repository on crates.io fframes](./phase-02-new-repository.md) | Completed |
| 3 | [Rewire and remove here](./phase-03-rewire-and-remove.md) | Completed |

## Acceptance criteria

- [x] `desktop/app/tests/kit_props_roundtrip.rs` passes with the adapter (phase 1, and again on the git dependency in phase 3).
- [x] In the new repo: `cargo test -p fframes-kit --all-features`, `just clippy`, `just check-wasm`, `just e2e cpu` pass against crates.io crates; `cargo package -p fframes-kit-macros -p fframes-kit` succeeds.
- [x] `examples/fframes-studio-intro` and the kit-props fixture build from the git dependency.
- [x] No `components/`, `docs/components/`, `docs/latex-setup.md` left here; `AGENTS.md` points to the new repo.

## Constraints

- Never stage the user's uncommitted edits (root `Cargo.toml` svgr override, `components/Cargo.lock` svgr lines, `desktop/**`, workflows). Root-workspace commits are made from a clean worktree under `C:\hc`.
- `[patch.crates-io] fframes` only applies while the fork's `fframes` version satisfies the kit's requirement (`^1.3.0`). When the fork bumps its version, bump the kit's requirement too.
- Commit scopes: `kit` for kit files, `studio-props` for the adapter and fixture.
