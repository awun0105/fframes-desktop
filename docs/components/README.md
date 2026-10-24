# fframes-kit references

Agent and author references for `fframes-kit`, the component library in `components/` (its
own Cargo workspace). Each file is compiled as doctests of the kit (`cargo test -p
fframes-kit` from `components/`).

| File | Covers |
| --- | --- |
| [components.md](components.md) | the import, the component model, time containers, fades, rate functions, `Anim`, `Theme`, writing your own component, a whole video |
| [math.md](math.md) | the math kit (feature `math`): its figures, each with an example and its gallery scene; the plots of `fframes_kit::plot` (no feature) |
| [physics.md](physics.md) | the physics kit (feature `physics`): its figures, simulations stepped once in the constructor, each with an example and its gallery scene |
| [chemistry.md](chemistry.md) | the chemistry kit (feature `chemistry`): Bohr atoms, element tiles and the periodic table from `mendeleev`, skeletal formulas by hand or from SMILES with `chematic`, each with an example and its gallery scene |

LaTeX formulas (feature `latex`) need a TeX installation: see [latex-setup.md](../latex-setup.md).

Commands, from `components/` (or `just -f components/justfile <recipe>` from the root):

- `just test`: unit tests and these doctests
- `just clippy`, `just fmt`
- `just check-wasm`: the gallery's editor bridge for `wasm32-unknown-unknown`
- `just e2e cpu [--update]`: gallery key frames against `gallery/snapshots`
- `just boundary`: must print nothing; kit commits never touch upstream fframes or Studio
