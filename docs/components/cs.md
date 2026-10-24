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

## ListTrace

`ListTrace` holds the steps of a linked list, a stack or a queue. Each `ListStep` has the
items in order, each with an id that stays with it from step to step (so the figures can
move, add and remove items smoothly), the links when they are not simply each item to the
next, the highlighted items, a found item, named pointers and a note.
`ListTrace::record(&values)` starts a `ListRecorder`; every operation adds the steps that
show it, and `finish` gives the trace:

- stack: `push`, `pop` (the item lights up, then leaves), `peek`;
- queue: `enqueue`, `dequeue`, `peek_front`;
- linked list: `insert(index, value)` and `remove(index)` walk a `cur` pointer to the node
  before, `find(value)` walks until it finds the value, and `reverse` walks `prev`, `cur`
  and `next` along while each link turns back, then puts the nodes in their new order;
- `note` replaces the note of the last step.

```rust
use fframes_kit::prelude::*;

let list = ListTrace::record(&[3, 8, 1]).insert(2, 5).remove(0).reverse().finish();
assert_eq!(list.result(), ["1", "5", "8"]);

let stack = ListTrace::record(&[4]).push(7).pop().finish();
assert_eq!(stack.result(), ["4"]);
```

## LinkedList, Stack and Queue

`LinkedList` draws a `ListTrace` as boxes with a value and a link cell, `node` by `height`
pixels and `gap` apart. Links are arrows from the link cell to the next node; a link back to
a node on the left dips under the nodes, and a node without a link shows a slash. New nodes
drop in, removed ones drop out, and nodes that pass each other (when a reversed list takes
its new order) swing apart. The pointers, with `head` on the first node, sit above the nodes.
Gallery scene: `LinkedList` (an insert, a find, a remove and a reversal).

`Stack` piles the items up in a container `capacity` boxes tall, the last item on top with a
`top` marker; pushed boxes drop in, popped ones rise out. `Queue` lines the items up in a
channel `capacity` boxes long, the front at the left, with `front` and `back` markers;
enqueued boxes slide in from the right and dequeued ones leave to the left. Gallery scenes:
`Stack`, `Queue`.

```rust
use fframes_kit::prelude::*;

fn queue<'a>(ui: &mut Ui<'a, '_, '_>, trace: &'a ListTrace) -> Svgr<'a> {
    // trace = ListTrace::record(&[4]).enqueue(7).dequeue().finish(), built once
    ui.show(Queue {
        trace: Some(trace),
        delay: 0.5,
        step_time: 0.7,
        capacity: 5,
        ..ui.defaults()
    })
}
```

## TreeTrace and TreeView

`TreeTrace` holds the steps of a binary tree. Each `TreeStep` has the nodes (an id, a label
and the ids of the left and right child), the root, the highlighted nodes, the path walked
to them, a found node and a note. Every step is laid out once when the trace is made, with
`layout::tidy_tree`; a node with one child leans it to its side.

- `TreeTrace::bst(&values)` starts a search tree with the values inserted. `insert`, `search`
  and `remove` walk down from the root one node a step, noting which way they go; `remove`
  replaces a node with two children by the smallest node on its right.
- `TreeTrace::max_heap(&values)` and `min_heap` start a heap. `push` adds at the bottom and
  sifts up, `pop` takes the root and sifts the last node down, a compare and a swap a step.
  Nodes keep their values, so swapped nodes trade places.

`TreeView` draws the trace as round nodes `radius` pixels, `spacing` pixels between slots and
`level` pixels between depths. Highlighted nodes light up, the path is outlined, a found node
turns green, nodes slide to their new places (two swapped nodes pass side by side), new ones
drop in and removed ones fade. Gallery scenes: `TreeView` (a search tree: an insert, a search
and a remove), `Heap` (a push and a pop).

```rust
use fframes_kit::prelude::*;

let bst = TreeTrace::bst(&[50, 30, 70]).insert(40).remove(30).finish();
let last = bst.steps().last().unwrap();
assert_eq!(last.nodes.len(), 3);
assert_eq!(last.note, "40 moves up");

fn tree<'a>(ui: &mut Ui<'a, '_, '_>, trace: &'a TreeTrace) -> Svgr<'a> {
    ui.show(TreeView {
        trace: Some(trace),
        step_time: 0.8,
        ..ui.defaults()
    })
}
```

## Graph, GraphTrace and GraphView

`Graph` holds named nodes and edges, directed or not. `Graph::new(&names, &edges, directed)`
gives every edge weight 1; `Graph::weighted` takes `(from, to, weight)` and shows the
weights. Nodes are placed in layers along the edges with `layout::layered`; `at` places
them yourself, in the same slots and layers (fractions are fine).

`GraphTrace` records a search over a graph, one `GraphStep` per visit or discovery: the
current node, the frontier, the finished nodes, the tree edges that reached each node, the
edge being looked at, the distances and a note.

- `GraphTrace::bfs(&graph, start)` keeps a queue and visits by distance in edges;
- `GraphTrace::dfs(&graph, start)` keeps a stack, goes deep first and backs out;
- `GraphTrace::dijkstra(&graph, start)` takes the closest unfinished node each time and
  notes each relaxation (`T: 3 + 5 is not below 4`).

`order()` is the order nodes were finished in. `GraphView` draws the trace: the frontier,
current and finished nodes in the cell colours, tree edges in `tree_color`, the active edge
in `active_color`, distances above the nodes (`∞` until reached) for Dijkstra, and the
frontier (`queue: B, C`) and the note under the graph. Gallery scenes: `GraphView` (BFS
and DFS side by side), `Dijkstra`.

```rust
use fframes_kit::prelude::*;

let graph = Graph::new(&["A", "B", "C", "D"], &[(0, 1), (0, 2), (1, 3)], false);
let bfs = GraphTrace::bfs(&graph, 0);
assert_eq!(bfs.order(), [0, 1, 2, 3]);

fn search<'a>(ui: &mut Ui<'a, '_, '_>, trace: &'a GraphTrace) -> Svgr<'a> {
    ui.show(GraphView {
        trace: Some(trace),
        step_time: 0.6,
        ..ui.defaults()
    })
}
```

## HashTrace and HashTable

`HashTrace::record(buckets, &keys)` starts a table with separate chaining: a key goes to
bucket `key mod buckets` (Euclidean, so negative keys land in a bucket too) and is added to
the end of its chain. `insert`, `find` and `remove` show the hash, walk the chain one entry
a step and note each comparison; a key that is already there is not added twice. `HashTable`
draws the buckets as a column of index cells with each chain to its right, linked by
arrows. Gallery scene: `HashTable`.

```rust
use fframes_kit::prelude::*;

let table = HashTrace::record(5, &[12, 7, 3]).insert(17).find(9).finish();
let last = table.steps().last().unwrap();
assert_eq!(last.buckets[2].len(), 3);
assert_eq!(last.note, "9 is not there");
```

## Machine, MachineTrace and StateMachine

`Machine::new(&states, &transitions, start, &accepting)` is a finite state machine. A
transition `(from, to, "0,1")` reads any of its comma separated symbols. States are placed in
columns by their distance from the start (`at` places them yourself). `MachineTrace::run(&machine,
input)` reads the input one character a step and stops when no transition matches;
`accepted()` says whether it ended in an accepting state.

`StateMachine` draws the states as circles (accepting states with an inner ring), an arrow
into the start state, the transitions as labelled arrows (a state's own loops above it), a
token that travels along the transition taken, the input tape with the symbol being read
lit, and the note. The last state turns green when the input is accepted and red when it is
not. Gallery scene: `StateMachine` (multiples of three in binary).

```rust
use fframes_kit::prelude::*;

let even = Machine::new(
    &["even", "odd"],
    &[(0, 1, "1"), (1, 0, "1"), (0, 0, "0"), (1, 1, "0")],
    0,
    &[0],
);
assert!(MachineTrace::run(&even, "1001").accepted());
assert!(!MachineTrace::run(&even, "1000").accepted());
```
## CallTrace, CallStack and RecursionTree

`CallTrace` records a recursion: every call and every return takes a step. A returning call
stays on the stack for its step, showing its result, and leaves at the next. Each `Call`
keeps its label, its caller, the calls it made, its result and the steps where it is called
and returns. The tree of calls is laid out once with `layout::tidy_tree`.

- `CallTrace::recurse(name, arg, f)` runs `f` and records every call it makes through
  `r.call(arg)`, labelled `name(arg)`. The calls really run, so keep the input small: a tree
  of more than about 30 calls does not fit on the canvas.
- `CallTrace::fib(n)` and `CallTrace::factorial(n)` are two ready ones.
- `CallTrace::record()` records by hand: `call(label)`, `ret(result)`, `note`, `finish`.

`CallStack` piles a frame for each open call into a column as tall as the deepest stack (or
`capacity`). A call drops its frame on top, the innermost frame lights up, and a returning
frame shows `fact(2) = 2` in `colors.found`, then rises out. `RecursionTree` draws the call
tree: each node drops in under its caller, the open calls are outlined in `path_color`, the
innermost is lit, and each call's result appears beside it as it returns. Both play the same
trace, so they can run side by side in step. Gallery scenes: `CallStack` (`fact(5)`),
`RecursionTree` (`fib(4)` with its stack).

```rust
use fframes_kit::prelude::*;

let sum = CallTrace::recurse("sum", 3_u32, |r, n| if n == 0 { 0 } else { n + r.call(n - 1) });
assert_eq!(sum.calls().len(), 4);
assert_eq!(sum.max_stack(), 4);
assert_eq!(sum.steps().last().unwrap().note, "sum(3) = 6");

fn recursion<'a>(ui: &mut Ui<'a, '_, '_>, trace: &'a CallTrace) -> Svgr<'a> {
    let tree = ui.show(RecursionTree {
        trace: Some(trace),
        center: (1.6, 0.0),
        ..ui.defaults()
    });
    let stack = ui.show(CallStack {
        trace: Some(trace),
        center: (-4.6, 0.0),
        notes: false,
        ..ui.defaults()
    });
    fframes::svgr!(<g>{stack}{tree}</g>)
}
```
## ComplexityPlot

`ComplexityPlot` races growth rates on the same axes. `n` sweeps along `n_range` in
`duration` seconds from `delay`, and each `Growth` in `growths` grows with it, a dot at its
head; a curve that leaves the top of `steps` stops there. A legend beside the axes shows
`n` and each rate in its colour with its steps at that `n` (`1,048,576`). The rates are
`Constant`, `Log`, `Linear`, `NLogN`, `Quadratic`, `Cubic`, `Exponential` and `Factorial`
(logarithms base 2); `Growth::eval(n)` gives the steps, `label()` the notation (`O(n log n)`).
`palette` has one colour per rate, slowest first, so a rate keeps its colour in every plot.
Gallery scene: `ComplexityPlot`.

```rust
use fframes_kit::prelude::*;

assert_eq!(Growth::Quadratic.eval(12.0), 144.0);
assert_eq!(Growth::Exponential.label(), "O(2^n)");

fn race<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(ComplexityPlot {
        growths: vec![Growth::Linear, Growth::NLogN, Growth::Quadratic],
        delay: 0.5,
        duration: 4.0,
        steps: Range::new(0.0, 200.0, 50.0),
        ..ui.defaults()
    })
}
```

## CodeBlock

`CodeBlock` shows source code on a panel in a monospaced `style` (JetBrains Mono by default,
ship it in `media/`), with line numbers and colours from a small tokenizer for `Language::Rust`,
`Python`, `JavaScript` and `Pseudo` (`Plain` has none): keywords, strings, numbers, comments,
called names and capitalised names. Strings and comments end with their line. The panel is as wide
as the longest line, or `width` pixels with longer lines ending in `…`. `lines` lights one line
(from 1, 0 for none) a step, one step every `step_time` seconds from `delay`; give it the same
timing as another figure's trace and it follows that figure step for step: the bar slides to the
line and the others dim to `dim`. `tokenize(line, language)` gives the coloured pieces. Gallery
scene: `CodeBlock` (a binary search with its Python code).

```rust
use fframes_kit::prelude::*;
use fframes_kit::kits::cs::{Token, tokenize};

let pieces = tokenize("return mid  # found", Language::Python);
assert_eq!(pieces[0], (Token::Keyword, "return"));
assert_eq!(pieces.last(), Some(&(Token::Comment, "# found")));

fn code<'a>(ui: &mut Ui<'a, '_, '_>, trace: &'a ArrayTrace, lines: Vec<usize>) -> Svgr<'a> {
    // lines[i] is the line that step i of `trace` runs
    let search = ui.show(ArrayView { trace: Some(trace), step_time: 0.9, ..ui.defaults() });
    let code = ui.show(CodeBlock {
        code: "while lo <= hi:\n    mid = (lo + hi) // 2".into(),
        language: Language::Python,
        lines,
        step_time: 0.9,
        center: (0.0, -1.5),
        ..ui.defaults()
    });
    fframes::svgr!(<g>{search}{code}</g>)
}
```

## SequenceDiagram

`SequenceDiagram` draws `participants` as boxes along the top, `spacing` pixels apart, with
dashed lifelines below, and sends one of `messages` every `step_time` seconds from `delay`, each
`row` pixels below the one before. A packet travels from the sender's lifeline to the receiver's
in the first `travel` of its step, drawing the arrow behind it, and turns into the arrowhead when
it lands. The label appears above the arrow, numbered with `numbers` and cut with `…` to fit
between the lifelines. `Message::new(from, to, label)` is a call, `Message::reply` is dashed, and a
message to the sender itself loops out to the right. The message being sent is lit in
`active_color`. Gallery scene: `SequenceDiagram` (a request through a server to a database).

```rust
use fframes_kit::prelude::*;

fn request<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    ui.show(SequenceDiagram {
        participants: vec!["client".into(), "server".into()],
        messages: vec![
            Message::new(0, 1, "GET /users/42"),
            Message::new(1, 1, "check the session"),
            Message::reply(1, 0, "200 OK"),
        ],
        delay: 0.5,
        ..ui.defaults()
    })
}
```
