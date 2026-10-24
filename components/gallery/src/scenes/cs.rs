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

pub fn scenes() -> Vec<GalleryScene> {
    vec![
        // eight steps of a second from 0.6 s
        GalleryScene::new("ArrayView", 9.0, array_view),
        // bubble sort, the longest of the four, ends at about 7.4 s
        GalleryScene::new("SortVisualizer", 8.5, sort_visualizer),
        GalleryScene::new("BinarySearch", 9.0, binary_search),
    ]
}
