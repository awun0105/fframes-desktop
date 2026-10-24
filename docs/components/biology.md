# fframes-kit: biology kit

The biology kit is the `biology` feature of `fframes-kit`
(`fframes-kit = { .., features = ["biology"] }`) and needs no other crate. It builds for
wasm32, so every figure here also runs in the editor, except `DnaHelix3D`, which needs the
`3d` feature as well. Every entry names its scene in the components gallery (`cargo run --release -p components-gallery -- strip <Scene> -n 8`).

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

## DnaHelix3D

`DnaHelix3D` needs the `3d` feature as well (`features = ["biology", "3d"]`), which turns on
`fframes/3d`. It renders the helix with `fframes::three_d` on every frame, on the CPU or, with
`wgpu`, on the GPU, into a `width` x `height` image around `center`.

- **Model:** the helix has B-DNA proportions, with ten pairs a turn and a minor and a major
  groove. Each pair is a rung of its two bases in `bases` colours, the same as `DnaHelix`.
  The rung runs between two backbones of beads in `strand_color`.
- **View:** the axis runs across the image and the camera fits the whole length. The camera
  looks from `elevation` degrees above while the helix turns `spin` degrees a second about
  its own axis.
- **Motion:** from `delay` the pairs build up from the left, one every `pair_time` seconds.
  Their beads grow, the backbone reaches over from the pair before, and the two bases meet
  in the middle.

Gallery scene: `DnaHelix3D`. Its five seconds of 24 pairs render on the CPU in about three
seconds on this machine's 32 threads.

```rust
use fframes_kit::prelude::*;

# #[cfg(feature = "3d")]
fn helix<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(DnaHelix3D {
        sequence: "GATTACAGATTACA".into(),
        width: 1000.0,
        height: 420.0,
        spin: 20.0,
        ..ui.defaults()
    })
}
```
## Transcription

`Transcription` copies a gene into mRNA. The DNA of `coding` (its coding strand on top, the
template strand under it) lies across the figure, `spacing` pixels per base. From `delay`
RNA polymerase moves along it, one base every `base_time` seconds: the strands open into a
bubble around it, the template base it reads lights up in `lit_color`, and the matching RNA
base (`U` across from `A`) drops onto the growing mRNA below. The mRNA is the coding strand
with `U` for `T` (`transcribe(coding)`). With `labels` the rows and the polymerase are
named. Gallery scene: `Transcription`.

```rust
use fframes_kit::prelude::*;
use fframes_kit::kits::biology::transcribe;

assert_eq!(transcribe("ATGGCTTAA"), "AUGGCUUAA");

fn gene<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(Transcription {
        coding: "ATGAAACGCTAG".into(),
        base_time: 0.3,
        ..ui.defaults()
    })
}
```

## Translation

`Translation` reads `mrna` into a protein. The ribosome slides onto the strand at the first
`AUG` and reads the reading frame (`reading_frame(mrna)`) one codon every `codon_time`
seconds from `delay`: a tRNA with the matching anticodon drops in carrying its amino acid
(`amino_acid(codon)`), the chain moves over onto it and gains a bond, and the empty tRNA
leaves. At the stop codon the ribosome comes apart and lets the chain go. The names of the
chain (`Met-Ala-Ser`) are written under the ribosome as it grows. Gallery scene:
`Translation`.

```rust
use fframes_kit::prelude::*;
use fframes_kit::kits::biology::{amino_acid, reading_frame};

assert_eq!(amino_acid("AUG"), Some("Met"));
assert_eq!(amino_acid("UAA"), None); // a stop codon
let (start, codons) = reading_frame("GCAUGUUUUGAC");
assert_eq!((start, codons), (2, vec!["AUG".into(), "UUU".into(), "UGA".into()]));

fn protein<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(Translation {
        mrna: "GAUGUUUGGCUAAC".into(),
        codon_time: 1.0,
        ..ui.defaults()
    })
}
```
## MitosisStages

`MitosisStages` shows one cell dividing into two, `size` pixels from its centre to its side.
From `delay` it goes through the six stages of `MitosisStage::ALL`, each `stage_time` seconds
long, and names each one above the cell (`label`):

- **Interphase:** the `chromosomes` lie loose and thin in the nucleus (`seed` places them).
- **Prophase:** they condense into X-shaped pairs of sister chromatids, the nuclear envelope
  breaks down and the centrosomes move to the poles.
- **Metaphase:** spindle fibres line them up across the middle.
- **Anaphase:** the sisters are pulled to opposite poles and the cell stretches.
- **Telophase:** a nucleus forms around each group and the chromosomes loosen.
- **Cytokinesis:** the cell pinches into two.

Chromosomes come in homologous pairs of one length, one of each pair coloured from each
parent (`colors`). `stage_at(seconds)` says which stage shows at a time. Gallery scene:
`MitosisStages`.

```rust
use fframes_kit::prelude::*;

let figure = MitosisStages { delay: 1.0, stage_time: 2.0, ..MitosisStages::default() };
assert_eq!(figure.stage_at(4.0), MitosisStage::Prophase);
assert_eq!(MitosisStage::ALL[5].name(), "Cytokinesis");

fn division<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(MitosisStages {
        chromosomes: 6,
        stage_time: 2.0,
        ..ui.defaults()
    })
}
```
## Neuron

`Neuron` lies across the frame, `length` pixels from its dendrites to its axon terminals:
dendrites branching from the cell body (placed from `seed`), the axon wrapped in myelin with
nodes of Ranvier between the segments, and the terminals. It grows in from `delay`, and with
`labels` each part is then named above or below it. At each time in `impulses` the cell body
fires and the impulse jumps from node to node along the axon at `speed` pixels a second; at
the terminals it releases neurotransmitter. Gallery scene: `Neuron`.

```rust
use fframes_kit::prelude::*;

fn two_signals<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(Neuron {
        impulses: vec![2.5, 3.5],
        labels: false,
        ..ui.defaults()
    })
}
```

## ActionPotential

`ActionPotential` traces the membrane potential of an action potential on axes of time (0 to
6 ms) against millivolts, drawing it in `duration` seconds from `delay`, a dot at its head.
The dashed threshold (-55 mV) and resting (-70 mV) lines are there from the start, and each
phase is named as the trace reaches it: resting potential, stimulus, depolarization, the peak,
repolarization and hyperpolarization. `membrane_potential(ms)` gives the curve. Gallery
scene: `ActionPotential`.

```rust
use fframes_kit::prelude::*;
use fframes_kit::kits::biology::membrane_potential;

assert_eq!(membrane_potential(0.0), -70.0);
assert!((membrane_potential(2.3) - 40.0).abs() < 1e-6); // the peak

fn spike<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(ActionPotential { duration: 3.0, ..ui.defaults() })
}
```
## PopulationCurve

`PopulationCurve` draws population growth on axes of time against population, from 0 to
`years` in `duration` seconds from `delay`, a dot at the head of each curve. Its `model` is
one of three:

- **`PopulationModel::Logistic`:** growth from `n0` at rate `r` levels off at the dashed
  carrying capacity `k`. With `compare` the exponential curve with the same start and rate is
  drawn faded beside it, until it leaves the top.
- **`PopulationModel::Exponential`:** the same growth with no limit.
- **`PopulationModel::PredatorPrey`:** Lotka-Volterra, with prey and predators rising and
  falling in turn. `PopulationModel::PREDATOR_PREY` cycles about every six time units.

A legend names each curve in its colour from `colors`. `series(count)` gives the values the
curves draw. Gallery scenes: `PopulationCurve` (logistic) and `PredatorPrey`.

```rust
use fframes_kit::prelude::*;

let logistic = PopulationCurve { k: 500.0, n0: 5.0, ..PopulationCurve::default() };
let n = &logistic.series(100)[0];
assert!((n[100] - 500.0).abs() < 5.0); // levelled off at K

fn cycles<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(PopulationCurve {
        model: PopulationModel::PREDATOR_PREY,
        years: 20.0,
        ..ui.defaults()
    })
}
```

## PhyloTree

`PhyloTree` reads a tree in Newick format (`Newick::parse`, which says where a malformed
string goes wrong) and grows it from the root at the left, `width` pixels from the root to
the leaves and `row` pixels between leaves:

- **Scale:** when every branch has a length the tree is drawn to scale; otherwise it is a
  cladogram, with the leaves in one column.
- **Growth:** from `delay` the branches draw outward one level every `step` seconds, and each
  leaf's name appears as its branch arrives.
- **Traits:** named inner nodes are traits shared by everything after them, marked on the
  branch that leads to them (`(Mouse,Human)hair`).
- **Highlight:** `highlight` names a node whose clade turns `highlight_color` at
  `highlight_at`.

Names with spaces are quoted (`'four limbs'`) or written with `_`. Gallery scene:
`PhyloTree` (vertebrates, then the mammals).

```rust
use fframes_kit::prelude::*;
use fframes_kit::kits::biology::Newick;

let tree = Newick::parse("((Cat:1,Dog:1):2,Fish:3)root;").unwrap();
assert_eq!(tree.leaves(), ["Cat", "Dog", "Fish"]);
assert!(Newick::parse("((Cat,Dog)").is_err());

fn pets<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(PhyloTree {
        newick: "((Cat:1,Dog:1)mammals:2,Fish:3);".into(),
        highlight: Some("mammals".into()),
        highlight_at: 3.0,
        ..ui.defaults()
    })
}
```
## FoodWeb

`FoodWeb` draws who eats whom: `species`, and the arrows of `eats` as `(prey, predator)`
pairs pointing the way energy flows. Names that are not in `species` are left out.

- **Levels:** producers (species that eat nothing here) sit at the bottom, and every other
  species sits one level above its highest prey, so the longest food chain sets its trophic
  level (`levels()`).
- **Order:** each level is ordered by where its prey and predators sit, to keep arrows from
  crossing. Arrows that would run through another species bend round it.
- **Look:** each species is a pill coloured by its level from `level_colors`, and the levels
  are named at the left from `level_names`.
- **Motion:** from `delay` the levels appear from the bottom, one every `level_time` seconds,
  and the arrows into each level draw as it does. With `flow`, dots of energy then run along
  every arrow.

Gallery scene: `FoodWeb`.

```rust
use fframes_kit::prelude::*;

let pond = FoodWeb {
    species: ["Algae", "Snail", "Fish", "Heron"].map(String::from).to_vec(),
    eats: [("Algae", "Snail"), ("Snail", "Fish"), ("Algae", "Fish"), ("Fish", "Heron")]
        .map(|(a, b)| (a.to_owned(), b.to_owned()))
        .to_vec(),
    ..FoodWeb::default()
};
assert_eq!(pond.levels(), [0, 1, 2, 3]); // the fish eats the snail, so it sits above it

fn pond_web<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(FoodWeb { flow: false, ..ui.defaults() })
}
```

## Circulation

`Circulation` draws double circulation: the four chambers of the heart, with the lungs above
and the body below. The right side of the heart sits on the viewer's left, as in a textbook.

- **Path:** the vessels follow the blood. It leaves the right ventricle, goes up the
  pulmonary artery to the lungs, back down the pulmonary vein to the left atrium, through the
  left ventricle, then along the aorta to the body and up the vena cava to the right atrium.
  The vena cava passes over the pulmonary artery with a gap of `gap_color`.
- **Colour:** each vessel is coloured by the blood in it, from `deoxygenated` to
  `oxygenated`. The blood turns red across the lungs and blue across the body.
- **Motion:** from `delay` the chambers appear and the vessels draw along the loop. Each name
  fades in as the line reaches it. Then blood cells run round at `speed` pixels a second,
  passing under the chambers, and the heart beats `bpm` times a minute, atria first.
- **Labels:** with `labels`, the lungs, the body and the four vessels are named.

Gallery scene: `Circulation`.

```rust
use fframes_kit::prelude::*;

fn heart<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(Circulation {
        delay: 0.5,
        bpm: 90.0,      // a faster beat
        labels: false,  // only the chambers' names
        ..ui.defaults()
    })
}
```

## EnzymeFit

`EnzymeFit` shows an enzyme splitting its substrate. The substrate is two halves joined by a
bond; it drifts in and settles into the active site cut into the top of the enzyme.

- **Model:** with `induced` (the default) the site starts wider than the substrate and closes
  round it as it binds (induced fit). Without it the site already has the substrate's shape
  (lock and key). The model is named under the caption.
- **Motion:** from `delay` the substrate arrives over 0.6..1.8 s and binds over 2..2.8 s.
  The bond glows from 3.4 s and breaks at 4.8 s, then the two products drift away and an
  induced-fit site opens again. Everything has finished by 6.2 s.
- **Text:** `labels` names the enzyme, the active site, the substrate and the products.
  `captions` describes each step underneath, one caption at a time.

Gallery scenes: `EnzymeFit` (induced fit) and `LockAndKey`.

```rust
use fframes_kit::prelude::*;

fn lock_and_key<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(EnzymeFit {
        induced: false,
        delay: 0.5,
        scale: 1.3,
        ..ui.defaults()
    })
}
```

## PunnettSquare

`PunnettSquare` crosses two `parents`, genotypes of one to three genes written as pairs of a
letter with the dominant allele in upper case (`"Pp"`, `"YyRr"`). Both parents must have the
same genes; otherwise nothing is drawn.

- **Square:** the first parent's gametes run across the top and the second's down the side,
  each in a circle. Every cell shows its genotype, dominant allele first, and is coloured by
  its phenotype from `phenotype_colors`. The colours are spread over the list, so with one
  gene the recessive phenotype takes the last colour. Text shrinks to fit the cells.
- **Ratios:** below the square, the genotype and phenotype ratios count up from zero as the
  cells fill. Phenotypes are named from `traits`, one (dominant, recessive) pair per gene,
  or written as `P_` and `pp` without them. Each name takes its phenotype's colour.
- **Motion:** from `delay` the square appears, the gametes split off each parent over
  0.5..1.7 s, and from 1.8 s the cells fill row by row over `fill_time` seconds. The gamete
  circles of the filling cell light up.

`Punnett::cross` gives the same cross as data, and `gametes` the gametes of one genotype.

Gallery scenes: `PunnettSquare` (`Pp` × `Pp`, purple and white flowers) and `DihybridCross`
(`YyRr` × `YyRr`, yellow or green and round or wrinkled seeds).

```rust
use fframes_kit::kits::biology::{Punnett, gametes};
use fframes_kit::prelude::*;

assert_eq!(gametes("AaBb").unwrap(), ["AB", "Ab", "aB", "ab"]);
let test_cross = Punnett::cross("Pp", "pp").unwrap();
assert_eq!(test_cross.genotype_ratio(), [("Pp".to_owned(), 2), ("pp".to_owned(), 2)]);

fn seeds<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(PunnettSquare {
        parents: ("YyRr".into(), "YyRr".into()),
        traits: vec![("yellow".into(), "green".into()), ("round".into(), "wrinkled".into())],
        ..ui.defaults()
    })
}
```