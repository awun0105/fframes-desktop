# fframes-kit: biology kit

The biology kit is the `biology` feature of `fframes-kit`
(`fframes-kit = { .., features = ["biology"] }`). It shares the layered graph layout of the
computer science kit (`layout::layered`, through `rust-sugiyama`) and needs nothing else. It
builds for wasm32, so every figure here also runs in the editor. Every entry names its scene
in the components gallery (`cargo run --release -p components-gallery -- strip <Scene> -n 8`).

- **Placement.** A figure is placed in units of `ui.space` (Manim's frame: 8 units tall, `y`
  up, the origin in the middle). Its sizes are in pixels.
- **Seeded.** Anything random (where the free ribosomes of a cell sit) comes from a `seed`,
  so every render draws the same picture.
- **Named parts.** Figures that name their parts put the names in columns beside the figure,
  each joined to its part by a leader line that draws when the part appears.

## Cell

`Cell` draws an animal or a plant cell (`kind: CellKind::Animal` or `Plant`), `size` pixels
from its centre to its side. From `delay` its `organelles` appear one every `stagger` seconds
in the order given; an empty list shows all of the kind's (`Organelle::of(kind)`): the
membrane, nucleus and nucleolus, rough ER, Golgi apparatus, mitochondria, lysosomes and free
ribosomes of an animal cell, and the cell wall, central vacuole and chloroplasts of a plant
cell as well. Each part fades in and grows into place; the membrane and the wall draw their
outline first. With `labels` a leader line then draws from each part to its name. The free
ribosomes are placed from `seed`, the colours come from `palette` (`CellPalette`). Gallery
scenes: `Cell` (an animal cell) and `PlantCell`.

```rust
use fframes_kit::prelude::*;

assert_eq!(Organelle::of(CellKind::Plant)[0], Organelle::CellWall);
assert_eq!(Organelle::Golgi.name(), "Golgi apparatus");

fn plant<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(Cell {
        kind: CellKind::Plant,
        // only these, in this order
        organelles: vec![Organelle::CellWall, Organelle::Vacuole, Organelle::Chloroplast],
        delay: 0.5,
        ..ui.defaults()
    })
}
```

## DnaHelix

`DnaHelix` draws a double helix from the side, the first base of `sequence` at the left
(the other strand is its complement, `complement(base)`). Each turn is `pitch` pixels long and
ten base pairs; the strands twist `spin` turns a second, and the back of each strand is dimmer
than its front. With `scan` a reading light passes the pairs from `delay`, one every
`step_time` seconds, and writes the lit pair above and below the helix. From `unzip_at`
seconds the strands come apart from the left in `unzip_time` seconds and straighten, each half
of a pair staying with its strand; with `replicate` a new partner strand in `new_color` grows
along each of them behind the fork, its bases pairing with the old ones. The bases are
coloured by `bases` (`BaseColors`). Gallery scene: `DnaHelix` (it reads, then replicates).

```rust
use fframes_kit::prelude::*;
use fframes_kit::kits::biology::complement;

let strand = "ATGC";
assert_eq!(strand.chars().map(complement).collect::<String>(), "TACG");

fn replication<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(DnaHelix {
        sequence: "ATGCGTACCTGAATCGGATC".into(),
        unzip_at: Some(4.0),
        ..ui.defaults()
    })
}
```
