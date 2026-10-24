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
        GalleryScene::new("HashTable", 12.5, hash_table),
        GalleryScene::new("StateMachine", 8.5, state_machine),
        // five calls and five returns of 0.6 s from 0.6 s
        GalleryScene::new("CallStack", 8.5, call_stack),
        // 19 steps of 0.45 s from 0.6 s
        GalleryScene::new("RecursionTree", 10.5, recursion_tree),
    ]
}
