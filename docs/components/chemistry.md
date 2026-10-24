# fframes-kit: chemistry kit

The chemistry kit is the `chemistry` feature of `fframes-kit`
(`fframes-kit = { .., features = ["chemistry"] }`). It brings two crates:
[`mendeleev`](https://crates.io/crates/mendeleev) for the element data and
[`chematic`](https://crates.io/crates/chematic) to read SMILES strings and lay molecules out in
2D. Both build for wasm32, so every figure here also runs in the editor, except the 3D\nfigures, which need the `3d` feature as well. Every entry names its
scene in the components gallery
(`cargo run --release -p components-gallery -- strip <Scene> -n 8`).

- **Placement.** A figure is placed in units of `ui.space` (Manim's frame: 8 units tall, `y`
  up, the origin in the middle). Its sizes are in pixels: shell gaps, tile sizes, bond
  lengths.
- **Elements** are given by atomic number. `symbol`, `name`, `atomic_weight`,
  `atomic_number` (from a symbol), `shells` (electrons per shell) and `table_cell` are in
  `fframes_kit::kits::chemistry`. `Category::of(number)` gives the family, and its `color`
  tints the tiles.
- **Animation props.** The fixed parts (rings, bonds) draw with `enter`. The rest (electrons,
  labels) fades with `parts_enter`. Tiles fade with `enter`.

## BohrAtom

The Bohr model of an element: its symbol on the nucleus and its electrons on rings,
`shell_gap` pixels apart. The shells come from the ground state configuration, so copper is
2, 8, 18, 1. After `delay` the electrons appear one by one over `fill` seconds, innermost
shell first. Every shell turns, the outer ones slower (`speed` radians a second on the first).
With `labels` the name and the electrons per shell sit under the atom. Gallery scene:
`BohrAtom` (sodium and chlorine).

```rust
use fframes_kit::prelude::*;

fn figure<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(BohrAtom {
        element: 11,
        center: (-3.6, 0.0),
        shell_gap: 50.0,
        delay: 0.5,
        enter: Some(Create::default().into()),
        ..ui.defaults()
    })
}
```

## ElementTile and PeriodicTable

`ElementTile` is one element as a square: the atomic number, the symbol, the name and the
atomic weight, tinted with the colour of its category (or `color` without
`category_colors`). Gallery scene: `ElementTile`.

`PeriodicTable` lays out all 118 tiles, `cell` pixels wide, with the lanthanides and
actinides in two rows under the main table.

- **Rise.** After `delay` the tiles rise in by atomic number, `stagger` seconds apart.
- **Highlight.** From `highlight_at` every tile outside `highlight_group` (1 to 18) and
  `highlight_category` dims to `dim`.
- **Trend.** From `trend_at` a sweep runs left to right over `sweep` seconds and recolours
  the tiles by `trend`, from `low_color` to `high_color`. The trends are
  `Trend::AtomicRadius`, `Trend::IonizationEnergy` and `Trend::AtomicWeight`. Tiles without a
  value (the radius of the heaviest elements is not known) turn `muted`. With `labels` the
  range is written in the empty top row.

`tile_center` gives the middle of an element's tile, for an arrow or a callout. Gallery
scenes: `PeriodicTable` (the halogens) and `PeriodicTrend` (the atomic radius).

```rust
use fframes_kit::prelude::*;

fn figures<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let carbon = ui.show(ElementTile {
        element: 6,
        center: (-5.0, 2.0),
        size: 160.0,
        ..ui.defaults()
    });
    let table = ui.show(PeriodicTable {
        center: (0.0, -0.3),
        cell: 60.0,
        highlight_group: Some(17),
        highlight_at: 2.0,
        trend: Some(Trend::IonizationEnergy),
        trend_at: 3.5,
        ..ui.defaults()
    });
    svgr!(<g>{carbon}{table}</g>)
}
```

## Molecule2D and Molecule

`Molecule2D` draws a skeletal formula from a `Structure`: atoms with positions in bond
lengths (`y` up) and bonds between them. It draws the bonds `bond_length` pixels long:

- single, double and triple bonds;
- a solid `Wedge` out of the page and a hashed `Dash` behind it, both wide at the second atom;
- in a ring, the second line of a double bond inside the ring;
- with the same number of neighbours on both sides (a carbonyl), the two lines centred.

Carbons are left out. Other atoms are labelled with their hydrogens (on the side away from
their bonds) and their charge, and with `color_atoms` oxygen, nitrogen, sulfur, phosphorus
and the halogens get their own colours. With `Create` as `enter` the bonds draw on one after
the other. Gallery scene: `Molecule2D` (a stereocentre with a wedge and a dash).

`Molecule` takes a SMILES string instead. `Structure::from_smiles` reads it with `chematic`,
which lays it out in 2D. Aromatic rings become alternating single and double bonds. The
layout is computed once per string and kept for the rest of the process. An invalid string
draws nothing, so check new strings with `Structure::from_smiles`, which returns a
`SmilesError`. Gallery scene: `Molecule` (aspirin and caffeine).

```rust
use fframes_kit::prelude::*;

fn water() -> Structure {
    Structure::new(
        vec![
            ChemAtom::new("O", 0.0, 0.0),
            ChemAtom::new("H", -0.8, -0.6),
            ChemAtom::new("H", 0.8, -0.6),
        ],
        vec![
            ChemBond::new(0, 1, BondKind::Single),
            ChemBond::new(0, 2, BondKind::Single),
        ],
    )
}

fn figures<'a>(ui: &mut Ui<'a, '_, '_>, water: &'a Structure) -> Svgr<'a> {
    let water = ui.show(Molecule2D {
        structure: Some(water),
        center: (-4.0, 0.0),
        bond_length: 90.0,
        ..ui.defaults()
    });
    let aspirin = ui.show(Molecule {
        smiles: "CC(=O)Oc1ccccc1C(=O)O".into(),
        center: (2.5, 0.0),
        enter: Some(Create::default().into()),
        ..ui.defaults()
    });
    svgr!(<g>{water}{aspirin}</g>)
}
```

## Molecule3D and Orbital

These two need the `3d` feature as well (`features = ["chemistry", "3d"]`), which turns on
`fframes/3d`. They render a 3D scene with `fframes::three_d` on every frame, on the CPU or,
with the `wgpu` features, on the GPU, and place it as an image of `width` x `height` pixels
around `center`. The scene uses chemistry axes: `z` up, `x` towards the viewer, `y` to the
right. The gallery's eleven seconds of 3D scenes render on the CPU in about seven seconds on 32
threads. The gallery leaves them out of the editor bridge.

`Molecule3D` draws a `Structure3D` as balls and sticks in CPK colours. `Structure3D::from_xyz`
reads an XYZ file (the number of atoms, a comment line, then `symbol x y z` in ångström) and
finds single bonds between atoms closer than 1.2 times the sum of their covalent radii. It
returns an `XyzError` for a broken file. Set `bonds` yourself (`(from, to, order)`) for double
and triple bonds, which draw as two or three thinner sticks. The camera fits the whole
molecule and turns `spin` degrees a second around the vertical axis. The bonds listed in
`highlight` (indices into `bonds`) turn `highlight_color` and glow over half a second from
`highlight_at`. Gallery scene: `Molecule3D` (ethanol, its C-O bond lighting up).

`Orbital` draws an `OrbitalKind`: `S`, `Px`, `Py`, `Pz`, `Dxy`, `Dxz`, `Dyz`, `Dx2y2` or `Dz2`.
The lobes are the polar plot of `size · cos²θ`, `positive` and `negative` for the sign of the
wave function, and `d_z²` has its ring of opposite sign. `OrbitalKind::lobes` lists the
directions and signs. The lobes grow and fade in over `grow` seconds from `delay`, then the
view turns. `axes` draws and labels x, y and z, and `label` names the orbital under the
image with its subscript. Gallery scene: `Orbital` (s, p_z, d_xy and d_z²).

```rust
use fframes_kit::prelude::*;

# #[cfg(feature = "3d")]
let water = {
    let water = Structure3D::from_xyz(
        "3\nwater\nO 0 0 0.117\nH 0 0.757 -0.469\nH 0 -0.757 -0.469\n",
    )
    .expect("a valid XYZ file");
    // two O-H bonds, the hydrogens too far apart for a third
    assert_eq!(water.bonds, vec![(0, 1, 1), (0, 2, 1)]);
    water
};

# #[cfg(feature = "3d")]
fn figures<'a>(ui: &mut Ui<'a, '_, '_>, water: &'a Structure3D) -> Svgr<'a> {
    let molecule = ui.show(Molecule3D {
        structure: Some(water),
        center: (-3.0, 0.0),
        width: 600.0,
        height: 600.0,
        highlight: vec![0],
        highlight_at: 1.0,
        ..ui.defaults()
    });
    let orbital = ui.show(Orbital {
        kind: OrbitalKind::Dz2,
        center: (3.0, 0.0),
        delay: 0.5,
        ..ui.defaults()
    });
    svgr!(<g>{molecule}{orbital}</g>)
}
```
## ReactionEquation

`Reaction` holds a reaction's formulas. `atom_counts` reads one, with parentheses (`Ca(OH)2`).
`balance` finds the smallest whole coefficients. It returns `None` when the reaction cannot be
balanced or has more than one balance. `tally` counts each element on both sides.

`ReactionEquation` needs the `latex` feature as well. It turns the equation without
coefficients into the balanced one: the species slide apart and the coefficients fade in,
starting at `delay` and taking `run_time` seconds. Typeset both with `Reaction::typeset` in
the constructor. With `tally` a line under the equation counts the atoms of each element on
both sides. It shows `unequal_color` until the balance and `equal_color` after it. Gallery
scene: `ReactionEquation` (propane burning).

```rust
use fframes_kit::prelude::*;

let reaction = Reaction::new(&["C3H8", "O2"], &["CO2", "H2O"]);
assert_eq!(reaction.balance(), Some(vec![1, 5, 3, 4]));
assert_eq!(reaction.ce(Some(&[1, 5, 3, 4])), "C3H8 + 5O2 -> 3CO2 + 4H2O");

# #[cfg(feature = "latex")]
fn figure<'a>(
    ui: &mut Ui<'a, '_, '_>,
    reaction: &'a Reaction,
    unbalanced: &'a Formula,
    balanced: &'a Formula,
) -> Svgr<'a> {
    // unbalanced = reaction.typeset(&engine, false)?, balanced = reaction.typeset(&engine, true)?
    ui.show(ReactionEquation {
        reaction: Some(reaction),
        unbalanced: Some(unbalanced),
        balanced: Some(balanced),
        delay: 1.0,
        ..ui.defaults()
    })
}
```

## BondFormation

Two atoms with their valence electrons on a ring around each symbol, placed as in a Lewis
diagram. From `at` over `duration` seconds they bond:

- **`BondType::Ionic`**: the left atom's electrons arc over to the gaps of the right one. The
  empty ring fades and both atoms get their charge.
- **`BondType::Covalent`**: the atoms move together until their rings overlap. The electrons
  each one lacks pair up between them in `shared_color`, and a label counts the shared pairs.

`valence`, `missing` and `bond_electrons` give the numbers it uses. Gallery scenes:
`BondFormation` (sodium chloride) and `CovalentBond` (oxygen).

```rust
use fframes_kit::prelude::*;

fn figures<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let salt = ui.show(BondFormation {
        center: (0.0, 1.8),
        at: 1.0,
        ..ui.defaults()
    });
    let oxygen = ui.show(BondFormation {
        kind: BondType::Covalent,
        left: 8,
        right: 8,
        center: (0.0, -1.8),
        at: 1.0,
        ..ui.defaults()
    });
    svgr!(<g>{salt}{oxygen}</g>)
}
```

## EnergyProfile

A reaction's energy along its progress: the reactants' level, the hump of the transition state
and the products' level, in `unit`. Arrows mark the activation energy (`activation()`) and the
enthalpy change (`enthalpy()`, negative when the reaction releases heat). A dashed guide
carries the reactants' level across. A `catalyst` gives a second, lower peak. Its dashed path
draws at `catalyst_at`. Gallery scene: `EnergyProfile`.

```rust
use fframes_kit::prelude::*;

fn figure<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(EnergyProfile {
        reactants: 50.0,
        transition: 150.0,
        products: 10.0,
        catalyst: Some(95.0),
        catalyst_at: 2.0,
        enter: Some(Create::default().into()),
        ..ui.defaults()
    })
}
```

## TitrationCurve and PhScale

`Titration` is an acid titrated with sodium hydroxide: the acid's concentration and volume,
the base's concentration, and a `pka` for a weak acid (`Titration::weak`) or none for a strong
one (`Titration::strong`). `ph(added)` solves the charge balance exactly, so the start, the
buffer region and the equivalence point all come out right: 25 mL of 0.1 M acetic acid starts
at pH 2.88, is at 4.76 at half equivalence and at 8.72 at equivalence. `equivalence()` gives
the base needed and `curve(until, steps)` the points.

`TitrationCurve` draws the curve on the `axes` of an `Axes` you show yourself (mL across, pH
up). From `delay` the base pours in at a constant rate up to `until` mL over `run_time`
seconds, with a dot at the current pH. As the pour passes them, the equivalence point is marked
with guides to both axes and its pH. For a weak acid, the half-equivalence point is marked too,
where the pH equals the pKa. `ph_at(seconds)` gives the pH at a time, for a readout or a
`PhScale` pointer.

`PhScale` is the scale as 15 cells in the colours of universal indicator (`indicator_color`
blends them), with the values and "acidic", "neutral" and "basic" under it. Each of `markers`,
a name and a pH, points at its place from above. They appear `stagger` seconds apart after
`delay`, in two rows. A `pointer` marks a pH on the bar. Gallery scenes: `TitrationCurve`
(acetic acid, with a scale following the pH) and `PhScale` (everyday substances).

```rust
use fframes_kit::prelude::*;

let acetic = Titration::weak(0.1, 25.0, 0.1, 4.76);
assert_eq!(acetic.equivalence(), 25.0);
assert!((acetic.ph(12.5) - 4.76).abs() < 0.01);

fn figure<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let axes = Axes {
        x_range: Range::new(0.0, 50.0, 10.0),
        y_range: Range::new(0.0, 14.0, 2.0),
        x_length: 8.0,
        y_length: 4.2,
        y_label: Some("pH".into()),
        ..ui.defaults()
    };
    let curve = TitrationCurve {
        titration: Titration::strong(0.1, 25.0, 0.1),
        axes: Some(axes.map(ui)),
        delay: 0.5,
        ..ui.defaults()
    };
    let ph = curve.ph_at(ui.seconds());
    let curve = ui.show(curve);
    let axes = ui.show(axes);
    let scale = ui.show(PhScale {
        center: (0.0, -3.0),
        width: 900.0,
        height: 36.0,
        pointer: Some(ph),
        ..ui.defaults()
    });
    svgr!(<g>{axes}{curve}{scale}</g>)
}
```

## StatesOfMatter

Particles in a box going from `from` to `to` (`MatterState::Solid`, `Liquid` or `Gas`) over
`duration` seconds after `at`:

- in a solid they vibrate on a lattice at the bottom;
- in a liquid they spread over the floor and wander past each other;
- in a gas they fly across the whole box and bounce off its walls.

Between two states the motions blend and the particles turn from `cold_color` to `hot_color`.
The positions are a function of the time and `seed` only, not a stepped simulation, so any
frame renders on its own and comes out the same everywhere. `positions(seconds)` gives them.
With `labels` the state, or the change under way ("melting", "boiling", "condensing",
"freezing"), is written under the box. Gallery scene: `StatesOfMatter` (solid to gas).

```rust
use fframes_kit::prelude::*;

fn figure<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(StatesOfMatter {
        seed: 3,
        from: MatterState::Gas,
        to: MatterState::Solid,
        at: 1.0,
        duration: 4.0,
        ..ui.defaults()
    })
}
```

## Equilibrium

`EquilibriumModel` is a reaction in a closed vessel. Each species has a name, a coefficient
(negative for a reactant) and a starting concentration in mol/L. The mixture moves along the
reaction to where the reaction quotient `Q` equals `k`, closing in by a factor of `e` every
`settle` seconds. `disturb(at, species, delta)` adds or removes some of a species later, and the
mixture settles again, as Le Chatelier's principle says. `concentrations(seconds)` and
`quotient` give the numbers.

`Equilibrium` shows the concentrations as bars, reactants in `reactant_color` and products in
`product_color`, starting the model's clock at `delay`. With `labels` each bar has its formula
under it (the digits set as subscripts) and its concentration over it. A line above compares
`Q` with `K`: whether the reaction runs forward, runs in reverse or is at equilibrium. Gallery
scene: `Equilibrium` (hydrogen iodide, then more hydrogen).

```rust
use fframes_kit::prelude::*;

let model = EquilibriumModel::new(&[("H2", -1, 1.0), ("I2", -1, 1.0), ("HI", 2, 0.0)], 49.0, 0.5)
    .disturb(3.0, 0, 1.0);
// settled again long after the extra hydrogen
assert!((model.quotient(&model.concentrations(30.0)) - 49.0).abs() < 1e-6);

fn figure<'a>(ui: &mut Ui<'a, '_, '_>, model: &'a EquilibriumModel) -> Svgr<'a> {
    ui.show(Equilibrium {
        model: Some(model),
        delay: 0.5,
        ..ui.defaults()
    })
}
```