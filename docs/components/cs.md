# fframes-kit: computer science kit

The computer science kit is the `cs` feature of `fframes-kit`
(`fframes-kit = { .., features = ["cs"] }`). It brings one crate,
[`rust-sugiyama`](https://crates.io/crates/rust-sugiyama), for the layered graph layout of
`layout::layered`. It builds for wasm32, so every figure here also runs in the editor. Every
entry names its scene in the components gallery
(`cargo run --release -p components-gallery -- strip <Scene> -n 8`).

- **Recorded once.** An algorithm runs once, in a constructor or a `OnceLock`, and records
  its steps. The figures sample the steps by time (`delay`, then one step every `step_time`
  seconds), so nothing runs again per frame, and every frame is a pure function of the steps.
- **Placement.** A figure is placed in units of `ui.space` (Manim's frame: 8 units tall, `y`
  up, the origin in the middle). Its sizes are in pixels: cell sizes, gaps, bar areas.
- **No jumps.** A change of state blends over a fraction of the step: colours, the dimming of
  cells out of range, pointers sliding to their next cell and notes fading from one to the
  next.

## Layouts

Two layouts in `fframes_kit::layout` place the nodes of the tree and graph figures. Both
compute once and give the same result on every run and machine.

- `tidy_tree(children, root)` places a tree: `children[i]` lists the children of node `i`.
  `x` is in slots (siblings and neighbouring subtrees are at least one slot apart at every
  depth), `y` is the depth. Parents sit midway over their children, and the leftmost node is
  at `x = 0`.
- `layered(count, edges)` (with `cs` or `biology`) places a directed graph in layers with
  Sugiyama's method: `y` is the layer, sources at 0 and edges pointing down, and `x` the slot
  in the layer, with crossings reduced. Cycles are fine. Connected parts sit side by side,
  and nodes without edges follow in layer 0.

```rust
use fframes_kit::layout::{layered, tidy_tree};

// a root with two children, the first with two of its own
let tree = tidy_tree(&[vec![1, 2], vec![3, 4], vec![], vec![], vec![]], 0);
assert_eq!(tree[1], (0.5, 1.0));
assert_eq!(tree[0].0, f64::midpoint(tree[1].0, tree[2].0));

let graph = layered(4, &[(0, 1), (0, 2), (1, 3), (2, 3)]);
assert_eq!((graph[0].1, graph[3].1), (0.0, 2.0));
```

## ArrayTrace

`ArrayTrace` holds the steps of an array algorithm. Each `ArrayStep` has the values after
the step, the cells it compared, where each cell came from when it moved cells (`moved`), the
cells in their final place, named pointers, the range still in play, a found cell and a
short note. Step 0 is the input. The recorders:

- `bubble`, `insertion`, `merge` (bottom up, merged in place, so the array holds every value
  once at every step) and `quick` (Lomuto's partition, the last value of a range as pivot);
- `binary_search(sorted, target)`, with `lo`, `mid` and `hi` pointers.

`ArrayTrace::new` takes steps of your own. `at(seconds, step_time)` samples a trace, and
`totals` counts its comparisons and moves.

```rust
use fframes_kit::prelude::*;

let trace = ArrayTrace::bubble(&[3, 1, 2]);
assert_eq!(trace.result(), [1, 2, 3]);
// two comparisons in the first pass, one in the second, and two values moved
assert_eq!(trace.totals(), (3, 2));

let search = ArrayTrace::binary_search(&[2, 5, 8, 12, 16, 23], 12);
assert_eq!(search.steps().last().unwrap().found, Some(3));
```

## ArrayView and BinarySearch

`ArrayView` draws a trace as a row of cells, `cell` pixels wide and `gap` apart, with the
index under each cell. Compared cells take `colors.compare`, moved cells travel to their new
place on an arc (the one moving right over the top, the one moving left underneath),
cells in their final place turn `colors.sorted`, cells out of the step's range dim, and the
pointers slide under the cells with their names. Pointers on the same cell stack. With
`notes` the step's note is written under the pointers. Light fills get dark text.
Gallery scene: `ArrayView` (an array reversed by two pointers, from steps of its own).

`BinarySearch` is an `ArrayView` (its `view`) playing `ArrayTrace::binary_search`, with
"looking for 23" above the cells. Gallery scene: `BinarySearch`.

```rust
use fframes_kit::prelude::*;

fn search<'a>(ui: &mut Ui<'a, '_, '_>, trace: &'a ArrayTrace) -> Svgr<'a> {
    // trace = ArrayTrace::binary_search(&values, 23), built once
    ui.show(BinarySearch {
        view: ArrayView {
            trace: Some(trace),
            delay: 0.5,
            step_time: 0.9,
            cell: 104.0,
            ..ui.defaults()
        },
        target: 23,
        ..ui.defaults()
    })
}
```

## SortVisualizer

`SortVisualizer` draws a sort's trace as bars, `width` by `height` pixels, each as tall as its
value. The compared bars light up, moved bars slide to their place and bars in their final
place turn green. `name` is written above with the comparisons and moves so far, `values`
writes each value under its bar and `notes` the step under the figure. Gallery scene:
`SortVisualizer` (the four sorts racing on the same twelve values, one step every 0.07 s).

```rust
use fframes_kit::prelude::*;

fn race<'a>(ui: &mut Ui<'a, '_, '_>, quick: &'a ArrayTrace) -> Svgr<'a> {
    // quick = ArrayTrace::quick(&values), built once
    ui.show(SortVisualizer {
        trace: Some(quick),
        name: "quicksort".into(),
        step_time: 0.2,
        width: 800.0,
        height: 300.0,
        ..ui.defaults()
    })
}
```
