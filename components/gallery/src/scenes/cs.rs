//! The computer science kit's figures, in the order of the plan's table.

use std::sync::OnceLock;

use fframes_kit::prelude::*;

use super::core::title;
use crate::GalleryScene;

fn fade(delay: f32) -> FadeIn {
    FadeIn {
        delay,
        ..FadeIn::default()
    }
}

/// An array reversed in place by two pointers walking towards each other.
fn reversal() -> ArrayTrace {
    let mut values = vec![3, 8, 1, 9, 4, 7];
    let step = |values: &[i64], i: usize, j: usize, note: String| ArrayStep {
        values: values.to_vec(),
        pointers: vec![("i".into(), i), ("j".into(), j)],
        note,
        ..ArrayStep::default()
    };
    let mut steps = vec![ArrayStep {
        values: values.clone(),
        ..ArrayStep::default()
    }];
    let (mut i, mut j) = (0, values.len() - 1);
    steps.push(step(&values, i, j, "two pointers at the ends".into()));
    while i < j {
        values.swap(i, j);
        let mut moved: Vec<usize> = (0..values.len()).collect();
        moved.swap(i, j);
        steps.push(ArrayStep {
            compare: vec![i, j],
            moved: Some(moved),
            ..step(
                &values,
                i,
                j,
                format!("swap {} and {}", values[j], values[i]),
            )
        });
        i += 1;
        j -= 1;
        if i < j {
            steps.push(step(&values, i, j, "move inwards".into()));
        }
    }
    steps.push(ArrayStep {
        values,
        note: "reversed".into(),
        ..ArrayStep::default()
    });
    ArrayTrace::new(steps)
}

fn array_view<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static TRACE: OnceLock<ArrayTrace> = OnceLock::new();
    let head = title(ui, "ArrayView");
    let figure = ui.show(ArrayView {
        trace: Some(TRACE.get_or_init(reversal)),
        delay: 0.6,
        step_time: 1.0,
        center: (0.0, 0.6),
        cell: 120.0,
        gap: 16.0,
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

const UNSORTED: [i64; 12] = [7, 3, 11, 1, 9, 4, 12, 6, 2, 10, 5, 8];

/// The four sorts racing on the same values, one step every `SORT_STEP` seconds.
const SORT_STEP: f32 = 0.07;

fn sort_visualizer<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static TRACES: OnceLock<[ArrayTrace; 4]> = OnceLock::new();
    let traces = TRACES.get_or_init(|| {
        [
            ArrayTrace::bubble(&UNSORTED),
            ArrayTrace::insertion(&UNSORTED),
            ArrayTrace::merge(&UNSORTED),
            ArrayTrace::quick(&UNSORTED),
        ]
    });
    let head = title(ui, "SortVisualizer");
    let names = ["bubble sort", "insertion sort", "merge sort", "quicksort"];
    let figures: Vec<_> = traces
        .iter()
        .zip(names)
        .enumerate()
        .map(|(i, (trace, name))| {
            let (column, row) = ((i % 2) as f64, (i / 2) as f64);
            ui.show(SortVisualizer {
                trace: Some(trace),
                name: name.into(),
                delay: 0.5,
                step_time: SORT_STEP,
                center: (-3.3 + 6.6 * column, 0.85 - 3.1 * row),
                width: 760.0,
                height: 230.0,
                values: false,
                enter: Some(fade(0.0)),
                ..ui.defaults()
            })
        })
        .collect();
    svgr!(<g>{head}{figures}</g>)
}

const SORTED: [i64; 10] = [2, 5, 8, 12, 16, 23, 38, 56, 72, 91];

fn binary_search<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static TRACE: OnceLock<ArrayTrace> = OnceLock::new();
    let head = title(ui, "BinarySearch");
    let figure = ui.show(BinarySearch {
        view: ArrayView {
            trace: Some(TRACE.get_or_init(|| ArrayTrace::binary_search(&SORTED, 23))),
            delay: 0.6,
            step_time: 0.9,
            center: (0.0, 0.4),
            cell: 104.0,
            enter: Some(fade(0.0)),
            ..ui.defaults()
        },
        target: 23,
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

fn linked_list<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static TRACE: OnceLock<ListTrace> = OnceLock::new();
    let trace = TRACE.get_or_init(|| {
        ListTrace::record(&[3, 8, 1])
            .insert(2, 5)
            .find(1)
            .remove(1)
            .reverse()
            .finish()
    });
    let head = title(ui, "LinkedList");
    let figure = ui.show(LinkedList {
        trace: Some(trace),
        delay: 0.6,
        step_time: 0.75,
        center: (0.0, 0.5),
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

fn stack<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static TRACE: OnceLock<ListTrace> = OnceLock::new();
    let trace = TRACE.get_or_init(|| {
        ListTrace::record(&[4])
            .push(7)
            .push(2)
            .peek()
            .pop()
            .push(9)
            .pop()
            .pop()
            .finish()
    });
    let head = title(ui, "Stack");
    let figure = ui.show(Stack {
        trace: Some(trace),
        delay: 0.6,
        center: (0.0, -0.2),
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

fn queue<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static TRACE: OnceLock<ListTrace> = OnceLock::new();
    let trace = TRACE.get_or_init(|| {
        ListTrace::record(&[4])
            .enqueue(7)
            .enqueue(2)
            .peek_front()
            .dequeue()
            .enqueue(9)
            .dequeue()
            .finish()
    });
    let head = title(ui, "Queue");
    let figure = ui.show(Queue {
        trace: Some(trace),
        delay: 0.6,
        center: (0.0, 0.4),
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

fn tree_view<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static TRACE: OnceLock<TreeTrace> = OnceLock::new();
    let trace = TRACE.get_or_init(|| {
        TreeTrace::bst(&[50, 30, 70, 20, 40, 60, 80])
            .insert(35)
            .search(60)
            .remove(30)
            .finish()
    });
    let head = title(ui, "TreeView");
    let figure = ui.show(TreeView {
        trace: Some(trace),
        delay: 0.6,
        step_time: 0.8,
        center: (0.0, 0.1),
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

fn heap<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static TRACE: OnceLock<TreeTrace> = OnceLock::new();
    let trace = TRACE.get_or_init(|| {
        TreeTrace::max_heap(&[40, 25, 30, 10, 15])
            .push(45)
            .pop()
            .finish()
    });
    let head = title(ui, "Heap");
    let figure = ui.show(TreeView {
        trace: Some(trace),
        delay: 0.6,
        step_time: 0.8,
        center: (0.0, 0.1),
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

/// Seven nodes in three layers, with a cycle through E.
fn search_graph() -> Graph {
    Graph::new(
        &["A", "B", "C", "D", "E", "F", "G"],
        &[
            (0, 1),
            (0, 2),
            (1, 3),
            (1, 4),
            (2, 5),
            (4, 5),
            (3, 6),
            (4, 6),
        ],
        false,
    )
}

fn graph_view<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static TRACES: OnceLock<[GraphTrace; 2]> = OnceLock::new();
    let traces = TRACES.get_or_init(|| {
        let graph = search_graph();
        [GraphTrace::bfs(&graph, 0), GraphTrace::dfs(&graph, 0)]
    });
    let head = title(ui, "GraphView");
    let figures: Vec<_> = traces
        .iter()
        .zip([-3.6, 3.6])
        .map(|(trace, x)| {
            ui.show(GraphView {
                trace: Some(trace),
                delay: 0.6,
                step_time: 0.55,
                center: (x, 0.5),
                spacing: 150.0,
                level: 140.0,
                radius: 30.0,
                enter: Some(fade(0.0)),
                ..ui.defaults()
            })
        })
        .collect();
    svgr!(<g>{head}{figures}</g>)
}

fn dijkstra<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static TRACE: OnceLock<GraphTrace> = OnceLock::new();
    let trace = TRACE.get_or_init(|| {
        let graph = Graph::weighted(
            &["S", "A", "B", "C", "T"],
            &[
                (0, 1, 4.0),
                (0, 2, 1.0),
                (2, 1, 2.0),
                (1, 3, 1.0),
                (2, 3, 5.0),
                (3, 4, 3.0),
                (1, 4, 6.0),
            ],
            true,
        )
        // left to right from S to T
        .at(&[(0.0, 0.5), (1.3, 0.0), (0.8, 1.0), (2.0, 0.5), (3.0, 0.5)]);
        GraphTrace::dijkstra(&graph, 0)
    });
    let head = title(ui, "Dijkstra");
    let figure = ui.show(GraphView {
        trace: Some(trace),
        delay: 0.6,
        step_time: 0.7,
        center: (0.0, 0.6),
        spacing: 280.0,
        level: 280.0,
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

/// A directed graph with weights, a cycle back to the start and a pair of edges both ways.
fn weighted_graph() -> Graph {
    Graph::weighted(
        &["S", "A", "B", "C", "T"],
        &[
            (0, 1, 4.0),
            (0, 2, 1.0),
            (2, 1, 2.0),
            (1, 3, 1.0),
            (3, 1, 3.0),
            (3, 4, 2.0),
            (4, 0, 7.0),
        ],
        true,
    )
    // left to right from S to T; C -> A and T -> S close cycles, so they point back
    .at(&[(0.0, 1.0), (1.0, 0.0), (1.0, 2.0), (2.0, 1.0), (3.0, 1.0)])
}

fn weighted_graph_scene<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static TRACES: OnceLock<[GraphTrace; 2]> = OnceLock::new();
    let traces = TRACES.get_or_init(|| {
        let ring = Graph::weighted(
            &["A", "B", "C", "D", "E", "F"],
            &[
                (0, 1, 2.0),
                (1, 2, 4.0),
                (2, 3, 1.0),
                (3, 4, 3.0),
                (4, 5, 2.0),
                (5, 0, 5.0),
                (0, 3, 6.0),
            ],
            false,
        )
        .layout(&GraphLayout::Circular, 1.0);
        [
            GraphTrace::dijkstra(&weighted_graph(), 0),
            GraphTrace::dijkstra(&ring, 0),
        ]
    });
    let head = title(ui, "WeightedGraph");
    let left = ui.show(GraphView {
        trace: Some(&traces[0]),
        delay: 0.6,
        step_time: 0.45,
        center: (-3.5, 0.6),
        spacing: 220.0,
        level: 150.0,
        radius: 30.0,
        frontier: false,
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    let right = ui.show(GraphView {
        trace: Some(&traces[1]),
        delay: 0.6,
        step_time: 0.45,
        center: (3.6, 0.5),
        spacing: 230.0,
        level: 230.0,
        radius: 30.0,
        frontier: false,
        curved: true,
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{left}{right}</g>)
}

fn variable_box<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    /// The array, its length and last value at each step, and a stack doing the same.
    type Figures = (ArrayTrace, Vec<String>, Vec<String>, ListTrace);
    static FIGURES: OnceLock<Figures> = OnceLock::new();
    let (array, lengths, lasts, stack) = FIGURES.get_or_init(|| {
        let array = ArrayTrace::record(&[4, 7])
            .append(2)
            .append(9)
            .append(5)
            .pop()
            .pop()
            .finish();
        let lengths = array
            .steps()
            .iter()
            .map(|s| s.values.len().to_string())
            .collect();
        let lasts = array
            .steps()
            .iter()
            .map(|s| s.values.last().map_or("-".into(), i64::to_string))
            .collect();
        let stack = ListTrace::record(&[4, 7])
            .push(2)
            .push(9)
            .push(5)
            .pop()
            .pop()
            .finish();
        (array, lengths, lasts, stack)
    });
    let head = title(ui, "VariableBox");
    let row = ui.show(ArrayView {
        trace: Some(array),
        delay: 0.6,
        step_time: 0.8,
        center: (-2.4, 1.3),
        cell: 112.0,
        label: "a",
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    let variables: Vec<Svgr<'a>> = [("len(a)", lengths, -4.0), ("a[-1]", lasts, -1.0)]
        .into_iter()
        .map(|(name, values, x)| {
            ui.show(VariableBox {
                name,
                values: values.clone(),
                delay: 0.6,
                step_time: 0.8,
                center: (x, -1.5),
                width: 150.0,
                height: 96.0,
                enter: Some(fade(0.0)),
                ..ui.defaults()
            })
        })
        .collect();
    let pile = ui.show(Stack {
        trace: Some(stack),
        delay: 0.6,
        step_time: 0.6,
        center: (4.6, 0.1),
        indices: true,
        label: "stack",
        notes: false,
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{row}{variables}{pile}</g>)
}

fn hash_table<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static TRACE: OnceLock<HashTrace> = OnceLock::new();
    let trace = TRACE.get_or_init(|| {
        HashTrace::record(5, &[12, 7, 3, 20])
            .insert(17)
            .find(3)
            .insert(22)
            .remove(7)
            .finish()
    });
    let head = title(ui, "HashTable");
    let figure = ui.show(HashTable {
        trace: Some(trace),
        delay: 0.6,
        step_time: 0.6,
        center: (0.0, 0.4),
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

fn state_machine<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static TRACE: OnceLock<MachineTrace> = OnceLock::new();
    let trace = TRACE.get_or_init(|| {
        // binary numbers divisible by three: the state is the remainder so far
        let machine = Machine::new(
            &["0", "1", "2"],
            &[
                (0, 0, "0"),
                (0, 1, "1"),
                (1, 0, "1"),
                (1, 2, "0"),
                (2, 1, "0"),
                (2, 2, "1"),
            ],
            0,
            &[0],
        );
        MachineTrace::run(&machine, "10010")
    });
    let head = title(ui, "StateMachine");
    let figure = ui.show(StateMachine {
        trace: Some(trace),
        delay: 0.6,
        center: (0.0, 0.6),
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

fn call_stack<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static TRACE: OnceLock<CallTrace> = OnceLock::new();
    let trace = TRACE.get_or_init(|| CallTrace::factorial(5));
    let head = title(ui, "CallStack");
    let figure = ui.show(CallStack {
        trace: Some(trace),
        delay: 0.6,
        step_time: 0.6,
        center: (0.0, 0.3),
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

fn recursion_tree<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static TRACE: OnceLock<CallTrace> = OnceLock::new();
    let trace = TRACE.get_or_init(|| CallTrace::fib(4));
    let head = title(ui, "RecursionTree");
    // the tree and the stack of the same calls, in step
    let tree = ui.show(RecursionTree {
        trace: Some(trace),
        delay: 0.6,
        step_time: 0.45,
        center: (1.6, 0.3),
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    let stack = ui.show(CallStack {
        trace: Some(trace),
        delay: 0.6,
        step_time: 0.45,
        center: (-4.6, 0.3),
        notes: false,
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{stack}{tree}</g>)
}

fn complexity_plot<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "ComplexityPlot");
    let figure = ui.show(ComplexityPlot {
        delay: 0.8,
        duration: 4.5,
        center: (-1.4, -0.3),
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

const BINARY_SEARCH: &str = "def binary_search(a, target):
    lo, hi = 0, len(a) - 1
    while lo <= hi:
        mid = (lo + hi) // 2
        if a[mid] == target:
            return mid  # found it
        if a[mid] < target:
            lo = mid + 1
        else:
            hi = mid - 1
    return -1";

/// The line of `BINARY_SEARCH` each step of the search runs.
fn search_lines(trace: &ArrayTrace) -> Vec<usize> {
    trace
        .steps()
        .iter()
        .enumerate()
        .map(|(i, step)| match step.note.split(' ').next() {
            _ if i == 0 => 2,
            Some("search") => 3,
            Some("is") => 5,
            Some("found") => 6,
            _ => 11,
        })
        .collect()
}

fn code_block<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    static TRACE: OnceLock<(ArrayTrace, Vec<usize>)> = OnceLock::new();
    let (trace, lines) = TRACE.get_or_init(|| {
        let trace = ArrayTrace::binary_search(&SORTED, 23);
        let lines = search_lines(&trace);
        (trace, lines)
    });
    let head = title(ui, "CodeBlock");
    // the code follows the search, step for step
    let search = ui.show(BinarySearch {
        view: ArrayView {
            trace: Some(trace),
            delay: 0.6,
            step_time: 0.9,
            center: (0.0, 2.2),
            cell: 84.0,
            notes: false,
            enter: Some(fade(0.0)),
            ..ui.defaults()
        },
        target: 23,
        ..ui.defaults()
    });
    let code = ui.show(CodeBlock {
        code: BINARY_SEARCH.into(),
        language: Language::Python,
        lines: lines.clone(),
        delay: 0.6,
        step_time: 0.9,
        center: (0.0, -1.1),
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{search}{code}</g>)
}

fn sequence_diagram<'a>(ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
    let head = title(ui, "SequenceDiagram");
    let figure = ui.show(SequenceDiagram {
        participants: vec!["client".into(), "server".into(), "database".into()],
        messages: vec![
            Message::new(0, 1, "GET /users/42"),
            Message::new(1, 1, "check the session"),
            // longer than the gap between the lifelines, so it is cut
            Message::new(1, 2, "SELECT * FROM users WHERE id = 42"),
            Message::reply(2, 1, "row 42"),
            Message::reply(1, 0, "200 OK, JSON"),
        ],
        delay: 0.6,
        center: (0.0, -0.2),
        enter: Some(fade(0.0)),
        ..ui.defaults()
    });
    svgr!(<g>{head}{figure}</g>)
}

pub fn scenes() -> Vec<GalleryScene> {
    vec![
        // eight steps of a second from 0.6 s
        GalleryScene::new("ArrayView", 9.0, array_view),
        // bubble sort, the longest of the four, ends at about 7.4 s
        GalleryScene::new("SortVisualizer", 8.5, sort_visualizer),
        GalleryScene::new("BinarySearch", 9.0, binary_search),
        GalleryScene::new("LinkedList", 12.5, linked_list),
        GalleryScene::new("Stack", 9.0, stack),
        GalleryScene::new("Queue", 7.5, queue),
        GalleryScene::new("TreeView", 13.0, tree_view),
        GalleryScene::new("Heap", 10.0, heap),
        GalleryScene::new("GraphView", 10.5, graph_view),
        GalleryScene::new("Dijkstra", 13.0, dijkstra),
        GalleryScene::new("WeightedGraph", 9.0, weighted_graph_scene),
        // six steps of 0.8 s from 0.6 s, the stack eight of 0.6 s
        GalleryScene::new("VariableBox", 7.0, variable_box),
        GalleryScene::new("HashTable", 12.5, hash_table),
        GalleryScene::new("StateMachine", 8.5, state_machine),
        // five calls and five returns of 0.6 s from 0.6 s
        GalleryScene::new("CallStack", 8.5, call_stack),
        // 19 steps of 0.45 s from 0.6 s
        GalleryScene::new("RecursionTree", 10.5, recursion_tree),
        GalleryScene::new("ComplexityPlot", 7.0, complexity_plot),
        GalleryScene::new("CodeBlock", 9.0, code_block),
        GalleryScene::new("SequenceDiagram", 7.0, sequence_diagram),
    ]
}
