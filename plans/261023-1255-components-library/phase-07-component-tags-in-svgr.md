---
phase: 7
title: "Phase 7: Component tags with the ui! macro"
status: completed
priority: P2
effort: 3d
dependencies: [2]
---

# Phase 7: Component tags with the ui! macro

## Goal

Let videos write `<LowerThird name="Ada" role="Mathematician" />` in markup, expanding to
`ui.show(LowerThird { name: "Ada".into(), role: "Mathematician".into(), ..ui.defaults() })`,
with nested component children. The kit ships its own proc macro, `fframes_kit::ui!`, that
wraps `svgr!`; `svgr-macro` is not changed.

## Context

- Design: component model report section 4 ("`svgr-macro` is not changed", steps 1 to 6).
- Macro facts (read only, verified 2026-10-23): the `svgr!` parser accepts any tag name and
  rejects unknown ones only in code generation (`svgr-macro/src/parser.rs:459`,
  `nodes_to_svgtree.rs:949`); an expression child becomes `Svgr::from(expr).as_subtree()`
  (`nodes_to_svgtree.rs:866`); `timeline!` is hoisted only in attribute blocks
  (`process_block` called from `parser.rs:399`), so animated props borrow a `timeline!` stored
  in the video (`Anim::Keyframes`).
- Struct form must already work for every component (phases 1 and 2), so this phase is sugar.
- The macro name is `ui!` and it expects a variable named `ui` in scope (plan decision; the
  memory note's `kit!` is superseded).

## Requirements

- `components/fframes-kit-macros` (proc-macro crate, deps `proc-macro2`, `quote`, `syn` as
  the root workspace pins them), re-exported as `fframes_kit::ui`.
- It scans the token stream. A tag whose name is one identifier starting with an uppercase
  letter is a component tag (every SVG element name starts lowercase). Everything else, SVG
  tags, attributes, text and `{expr}` blocks, is copied through unchanged.
- Attributes become fields; literal and `{expr}` values go through `.into()`. Bare attributes
  become `true`. Dashed names are an error naming the Rust field (`font-size` is `font_size`).
- Children become `children: Box::new(move |ui| ::fframes_kit::ui!(<g>..</g>))`, so nested
  component tags are handled by the same macro, recursively.
- A tag is replaced by `{ui.show(Comp { .., ..ui.defaults() })}` and the whole stream is
  emitted as `::fframes::svgr!(..)`. Static subtrees keep their `static_hash`; both `svgr!`
  code paths and the editor bridge work unchanged.
- When no `ui` is in scope, the error carries a span note: "component tags need a `ui: Ui` in
  scope; create one with `let mut ui = Ui::new(frame, ctx, &self.theme)`".
- Errors point at the attribute or tag, not at the macro call.

## Files

- Create: `components/fframes-kit-macros/{Cargo.toml,src/lib.rs,src/tags.rs}`,
  `components/fframes-kit/tests/tags.rs`, `components/gallery/src/scenes/tags.rs`.
- Modify: `components/Cargo.toml` (member, macro deps), `components/fframes-kit/Cargo.toml`
  and `src/lib.rs` (re-export `ui!`), `docs/components/components.md` (tag form section; the
  struct form stays the primary documented way until both are stable).
- Do not modify: `svgr-macro/`, `fframes/`, `AGENTS.md`.

## Tasks & Steps

1. Unit-test the rewrite on token streams inside the macro crate (a plain function from
   `TokenStream` to `TokenStream`): simple tag, expression attribute, bare attribute, dashed
   attribute error, nested children, component inside a normal `<g>`, SVG-only input passes
   through byte-identical.
2. Implement `tags.rs`; expose `#[proc_macro] pub fn ui`.
3. Integration tests in `components/fframes-kit/tests/tags.rs`: the tag form equals the struct
   form (compare `Svgr` debug output, or `fframes_test_utils` snapshots if that crate is usable
   from the kit workspace by path).
4. Gallery `Tags` scene: the `SquareToCircle` and `FadeIn` scenes written in tag form; its e2e
   frames must equal the struct-form snapshots (odiff 0%).
5. `trybuild` is not in the tree; check error messages with `compile_fail` doctests in
   `fframes-kit` for the dashed-name and missing-`ui` cases.

## Verification

- From `components/`: `cargo test -p fframes-kit-macros`, `cargo test -p fframes-kit`.
- `just -f components/justfile check-wasm` (the bridge exercises the string path).
- `just -f components/justfile e2e cpu`: `Tags` frames identical to their struct-form twins.
- `just -f components/justfile clippy`; `just -f components/justfile boundary` prints nothing.

## Risks

- **Token-level parsing.** `ui!` must find tags without a full SVG parser. Keep it to the
  `<Ident ...>` / `</Ident>` / `<Ident ... />` shapes and pass every other token through; the
  SVG-only pass-through test guards against damage to normal markup.
- **Span quality.** Errors must point at the attribute, not the macro call; test them.

## Todo

- [x] Rewrite unit tests, including byte-identical pass-through
- [x] `ui!` in `fframes-kit-macros`, re-exported from `fframes-kit`
- [x] Tag form equals struct form (integration tests)
- [x] `compile_fail` doctests for error messages
- [x] Gallery `Tags` scene identical to struct-form snapshots
- [x] `components.md` tag form section
