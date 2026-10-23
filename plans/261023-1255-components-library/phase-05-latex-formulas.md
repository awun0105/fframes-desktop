---
phase: 5
title: "Phase 5: LaTeX formulas"
status: todo
priority: P1
effort: 4d
dependencies: [2]
---

# Phase 5: LaTeX formulas

## Goal

Render full LaTeX (any package the user's TeX installation has: amsmath, mhchem, physics,
siunitx, TikZ, chemfig, circuitikz) into glyph paths that `svgr!` draws and that the kit's
animations (`Write`, `Create`, `TransformMatchingTex`, `Indicate` on a part) can move. TeX is
a prerequisite the user installs: MiKTeX on Windows, Ubuntu and macOS (TeX Live and MacTeX
work too, since only `latex`, `xelatex`/`lualatex` and `dvisvgm` on `PATH` are needed). No
Typst.

## Context

- This is Manim's pipeline: write a `.tex` file from a template, run `latex` to DVI (or
  `xelatex --no-pdf` to XDV), run `dvisvgm --no-fonts --exact-bbox` to SVG with glyphs as
  paths, cache the SVG by a hash of the input. Manim names: `MathTex`, `Tex`, `TexTemplate`.
- `fframes` re-exports `usvgr` (`fframes/src/lib.rs:205`). `usvgr` parses the SVG once and
  resolves dvisvgm's `<use>` references into plain paths with absolute transforms and fills.
- The native `Svgr` cannot take an SVG string (`fframes/src/svgr.rs:150-161`), so each glyph
  becomes one `<path d={..}>` built once, the same output type the earlier research planned
  (`Formula { glyphs, width, height, baseline }`).
- `render_frame` must not do I/O. Running TeX happens in the video constructor (or a
  `OnceLock`), which is native code; per frame the component only emits prebuilt `d` strings.
- The web editor (wasm32) cannot run TeX. Native renders and previews can.
- This VM has no TeX installed (checked 2026-10-23). The user approved installing MiKTeX on
  this VM (2026-10-23), so step 1 runs without asking.

## Requirements

Discovery and errors:
- Find `latex`, `xelatex`, `lualatex` and `dvisvgm` on `PATH`; `FFRAMES_TEX_BIN` overrides the
  directory. On Windows also look in the default MiKTeX locations
  (`%LOCALAPPDATA%\Programs\MiKTeX\miktex\bin\x64`, `C:\Program Files\MiKTeX\miktex\bin\x64`).
- A missing tool gives `TexError::NotInstalled` with the install instructions for the current
  OS (MiKTeX download page; `apt install texlive-latex-extra texlive-science dvisvgm` style
  alternative on Ubuntu; `brew install --cask mactex-no-gui` on macOS), never a panic.
- A TeX error gives `TexError::Compile` with the relevant lines of the `.log` (from the first
  `!` line) and the generated `.tex` source.
- Runs use `-interaction=nonstopmode -halt-on-error` and a timeout (default 120 s, because
  MiKTeX may install a missing package on first use). The docs tell MiKTeX users to enable
  automatic package installation (`initexmf --set-config-value [MPM]AutoInstall=1`) so no
  dialog blocks a render.

API (Manim names):
- `TexTemplate { compiler: TexCompiler::{Latex, XeLatex, LuaLatex}, preamble, body_wrapper }`
  with `TexTemplate::default()` (`standalone` class, amsmath, amssymb) and presets
  `TexTemplate::chemistry()` (adds mhchem, chemfig), `physics()` (physics, siunitx),
  `tikz()` (TikZ with `\def\pgfsysdriver{pgfsys-dvisvgm.def}`).
- `MathTex::new(tex)` (math mode, `{{ }}` marks parts like Manim) and `Tex::new(text)` (text
  mode, anything including TikZ pictures), both with `.template(..)` and `.typeset(&tex) ->
  Result<Formula, TexError>`, where `tex: &TexEngine` holds discovery results and the cache.
- `Formula` implements `Geometry` (phase 2), so `Create`, `DrawBorderThenFill` and `Write`
  work unchanged; `Write` staggers glyphs in drawing order.
- `Formula` component props: `of`, `size` (scale relative to the TeX output, default so that
  10 pt text is 48 px), `x`, `y`, `anchor`, `color` (replaces black, keeps explicit colours),
  `color_map: &[(Part, Color)]`, `emphasis: Option<Indicate>`, `enter`, `exit`, `key`.
- Parts: `Part::Index(n)` and `Part::Label(&str)` (`{{ }}` groups are indexed; `\part{name}{..}`
  from the default preamble labels one). `formula.part_bbox(part)`.
- `TransformMatchingTex { from, to }`: match parts by label, then by equal TeX source, move
  matched glyphs, fade out unmatched, fade in new ones.
- `ChemEq { ce }` is `MathTex` with `\ce{..}` and the chemistry template; per-species parts by
  splitting on `+` and arrows into `{{ }}` groups.

Caching:
- Key: hash of compiler, preamble, wrapped source and a kit cache version. Value: the `.svg`
  from dvisvgm (small, text).
- Directory: `TexEngine::new(cache_dir)`. The path must not depend on the working directory:
  `components/scripts/e2e.mjs` runs binaries from `components/gallery`, while `cargo run -p` runs from the
  repository root. The gallery and the docs use
  `concat!(env!("CARGO_MANIFEST_DIR"), "/tex-cache")`, and the docs recommend committing it, so other machines and CI render without TeX installed. A cached formula never
  runs TeX.
- Without TeX and without a cache hit the error says which formula is missing from the cache.

Editor fallback (wasm32):
- `TexEngine` on wasm32 has no process access; `typeset` reads the cache only if it was
  embedded, otherwise the `Formula` component draws the TeX source as plain text and
  `inspect` reports it. Embedding the cache in the editor bridge is not part of this phase.

## Files

- Create: `components/fframes-kit/src/latex/{mod.rs,engine.rs,discover.rs,template.rs,run.rs,cache.rs,svg_to_glyphs.rs,parts.rs,component.rs,matching.rs,chem.rs,error.rs}`
  behind feature `latex`, `components/gallery/src/scenes/latex.rs`,
  `components/gallery/tex-cache/` (committed), `docs/latex-setup.md` (install and
  configure MiKTeX on Windows, Ubuntu, macOS; TeX Live alternative; automatic package
  installation; how to check with `latex --version` and `dvisvgm --version`).
- Modify: `components/fframes-kit/Cargo.toml` (feature `latex`), `prelude.rs`, `lib.rs` (Manim rows
  `MathTex`, `Tex`, `TexTemplate`), `components.md` (formulas section: prerequisites link,
  the `{{ }}` rule, `ChemEq`, the cache), `docs/components/README.md` (one line: LaTeX
  needs a TeX install, link to `docs/latex-setup.md`), gallery `Cargo.toml` (enable `latex`), `components/scripts/e2e.mjs` (frames).

## Tasks & Steps

1. Install MiKTeX on this VM (approved). This VM has Chocolatey and no winget:
   `choco install miktex -y` (the package is flagged "possibly broken" on 2026-10-23). If it
   fails, use MiKTeX's own command-line installer (`miktexsetup_standalone` from miktex.org:
   `--package-set=basic download`, then `install`). Then enable automatic package installation
   (`initexmf --set-config-value [MPM]AutoInstall=1`), check `latex --version` and
   `dvisvgm --version`, and point `FFRAMES_TEX_BIN` at its `bin` folder if `PATH` is not
   refreshed. Record the version and the method. If both methods fail, mark the phase
   `Blocked` and stop.
2. Prove the pipeline by hand first: `e^{i\pi} + 1 = 0`, a fraction, a 2x2 matrix,
   `\sum_{k=1}^n k`, `\ce{2H2 + O2 -> 2H2O}`, a small TikZ picture. Run `latex` then
   `dvisvgm --no-fonts --exact-bbox`; open the SVGs; parse them with `usvgr` in a unit test.
3. Parts: wrap each `{{ }}` group so dvisvgm marks it. First try
   `\special{dvisvgm:raw <g id="part-N">}` and `\special{dvisvgm:raw </g>}` around the group;
   if groups do not survive or change spacing, use one unique `\color[RGB]` per part and map
   glyph fills back to parts (then restore the colours). Test `{{a}}^2`, `\frac{{{a}}}{b}` and
   `\left({{x}}\right)`; compare glyph positions with and without markers (must be equal).
4. `discover.rs` with unit tests that use a temporary directory of fake executables (so the
   discovery logic is tested on every OS without TeX), and the per-OS install message.
5. `run.rs`: temp dir, process with timeout, log extraction; `cache.rs`; `svg_to_glyphs.rs`
   (usvgr tree to glyph paths with absolute transforms, fills and the part ids).
6. `Formula` component, `TransformMatchingTex`, `ChemEq`.
7. Gallery scenes `MathTex`, `WriteFormula`, `FormulaParts`, `TransformMatchingTex`,
   `ChemEq`, `TikzDiagram`; the gallery opens its cache at
   `concat!(env!("CARGO_MANIFEST_DIR"), "/tex-cache")`; generate and commit `tex-cache/`.
   Then run `just -f components/justfile e2e cpu` with TeX hidden from `PATH` to prove the harness uses the cache.
8. Measure: first run (no cache) and cached run time for the gallery's formulas, and the
   per-frame cost of a 20-formula scene against the same scene with `<text>`; record them.
9. Write `docs/latex-setup.md` and the `components.md` section.

## Verification

- `cargo test -p fframes-kit --features latex` (discovery with fake executables, SVG parsing
  from checked-in sample SVGs, part mapping, matching).
- With TeX installed: delete the gallery cache, render
  `frame MathTex@50%,TransformMatchingTex@50%,ChemEq@end,TikzDiagram@end`, check the PNGs by
  eye, confirm the cache is regenerated byte-identical.
- With TeX removed from `PATH` (`FFRAMES_TEX_BIN` pointing to an empty dir): the gallery still
  renders from the committed cache; an uncached formula gives the `NotInstalled` message.
- `just -f components/justfile e2e cpu` (uses the committed cache), `just -f components/justfile check-wasm` with
  `latex` (fallback path compiles), `just -f components/justfile clippy`.
- Ubuntu and macOS: discovery is covered by the fake-executable tests here; a real run on those
  systems is a manual check by the user (see open questions in `plan.md`).

## Risks

- **dvisvgm versions differ in output.** Parse with usvgr rather than relying on the exact
  markup; the cache stores the SVG, so the glyph conversion stays testable from samples.
- **MiKTeX first-run dialogs.** Without automatic installation a missing package opens a
  dialog and the run hits the timeout; the error message tells the user to enable it.
- **Font size mapping.** TeX points versus pixels: fix the scale in one constant, test that
  the `x` height of a sample matches the expected pixels.
- **Security.** TeX can run shell commands only with `--shell-escape`; never pass it. Input
  comes from the video's own source, so no further sandboxing is needed.

## Todo

- [ ] MiKTeX installed on the VM (approved 2026-10-23), auto-install enabled
- [ ] Manual pipeline proof with the six samples
- [ ] Parts marking chosen and tested on the three tricky cases
- [ ] Discovery with fake-executable tests and per-OS install messages
- [ ] Runner with timeout and log extraction; cache; SVG to glyphs
- [ ] `Formula` component, `TransformMatchingTex`, `ChemEq`
- [ ] Gallery scenes and committed `tex-cache/`
- [ ] Measurements recorded
- [ ] `docs/latex-setup.md` and `components.md` formulas section
