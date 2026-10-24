//! Array algorithms recorded once, step by step, and sampled by time.

/// One step of an array algorithm: the values after it and what it looked at.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ArrayStep {
    pub values: Vec<i64>,
    /// The cells compared in this step.
    pub compare: Vec<usize>,
    /// Where each cell was before this step (`values[k]` came from `moved[k]`), when the step
    /// moved cells: a swap, or a cell taken out and put back further left.
    pub moved: Option<Vec<usize>>,
    /// Cells in their final place (or, for insertion sort, the sorted prefix).
    pub sorted: Vec<usize>,
    /// Named markers under cells: `("lo", 0)`, `("pivot", 7)`.
    pub pointers: Vec<(String, usize)>,
    /// The cells still in play, both ends included; the rest are dimmed.
    pub range: Option<(usize, usize)>,
    /// A cell that was searched for and found.
    pub found: Option<usize>,
    /// What happens in this step, in a few words.
    pub note: String,
}

/// The steps of an array algorithm, recorded once (in a constructor or a `OnceLock`) and
/// sampled by time with [`ArrayTrace::at`]. Step 0 is the input as given. The recorders
/// [`bubble`](ArrayTrace::bubble), [`insertion`](ArrayTrace::insertion),
/// [`merge`](ArrayTrace::merge), [`quick`](ArrayTrace::quick) and
/// [`binary_search`](ArrayTrace::binary_search) fill it; [`ArrayTrace::new`] takes steps of
/// your own.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ArrayTrace {
    steps: Vec<ArrayStep>,
    /// Comparisons and moves made up to and including each step.
    counts: Vec<(usize, usize)>,
}

/// Where a trace is at a time: the step, the one before it and how far into the step.
#[derive(Debug, Clone, Copy)]
pub struct TraceAt<'t> {
    pub index: usize,
    pub step: &'t ArrayStep,
    pub previous: &'t ArrayStep,
    /// Seconds since the step began (0 before the trace starts).
    pub into: f32,
    /// Comparisons and moves so far.
    pub counts: (usize, usize),
}

impl ArrayTrace {
    pub fn new(steps: Vec<ArrayStep>) -> Self {
        let mut counts = Vec::with_capacity(steps.len());
        let (mut compares, mut moves) = (0, 0);
        for (i, step) in steps.iter().enumerate() {
            if i > 0 {
                // a step that moves cells is not a comparison of its own
                compares += usize::from(!step.compare.is_empty() && step.moved.is_none());
                moves += usize::from(step.moved.is_some());
            }
            counts.push((compares, moves));
        }
        Self { steps, counts }
    }

    pub fn steps(&self) -> &[ArrayStep] {
        &self.steps
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// The values after the last step.
    pub fn result(&self) -> &[i64] {
        self.steps.last().map_or(&[], |s| &s.values)
    }

    /// Comparisons and moves of the whole run.
    pub fn totals(&self) -> (usize, usize) {
        self.counts.last().copied().unwrap_or_default()
    }

    /// The step `seconds` after the start, one step every `step_time` seconds. Before the
    /// start it is step 0, after the end the last step.
    pub fn at(&self, seconds: f32, step_time: f32) -> Option<TraceAt<'_>> {
        let (index, into) = super::keyed::locate(self.steps.len(), seconds, step_time)?;
        Some(TraceAt {
            index,
            step: &self.steps[index],
            previous: &self.steps[index.saturating_sub(1)],
            into,
            counts: self.counts[index],
        })
    }

    /// Bubble sort: neighbours compared left to right, the larger moved right, the largest
    /// left settling at the end of each pass. Stops after a pass without moves.
    pub fn bubble(input: &[i64]) -> Self {
        let mut r = Recorder::new(input);
        let n = input.len();
        for pass in 0..n {
            let mut moved = false;
            for j in 0..n.saturating_sub(pass + 1) {
                r.compare(
                    &[j, j + 1],
                    format!("compare {} and {}", r.a[j], r.a[j + 1]),
                );
                if r.a[j] > r.a[j + 1] {
                    r.swap(j, j + 1);
                    moved = true;
                }
            }
            let end = n - pass - 1;
            r.settle(end..n, format!("{} is in place", r.a[end]));
            if !moved {
                break;
            }
        }
        r.done()
    }

    /// Insertion sort: each value moved left past the larger ones before it. The sorted
    /// prefix grows by one each round.
    pub fn insertion(input: &[i64]) -> Self {
        let mut r = Recorder::new(input);
        let n = input.len();
        for i in 1..n {
            let mut j = i;
            while j > 0 {
                r.compare(
                    &[j - 1, j],
                    format!("compare {} and {}", r.a[j - 1], r.a[j]),
                );
                if r.a[j - 1] <= r.a[j] {
                    break;
                }
                r.swap(j - 1, j);
                j -= 1;
            }
            r.settle(0..=i, format!("the first {} are sorted", i + 1));
        }
        r.done()
    }

    /// Merge sort, bottom up: runs of 1, 2, 4 and so on merged in place. Each comparison
    /// takes the smaller head; a value from the right run moves left into its place, the
    /// rest shifting right, so the array always holds every value once.
    pub fn merge(input: &[i64]) -> Self {
        let mut r = Recorder::new(input);
        let n = input.len();
        let mut width = 1;
        while width < n {
            let mut lo = 0;
            while lo + width < n {
                let hi = (lo + 2 * width).min(n);
                r.range = Some((lo, hi - 1));
                let (mut left, mut mid) = (lo, lo + width);
                while left < mid && mid < hi {
                    r.compare(
                        &[left, mid],
                        format!("merge: {} or {}", r.a[left], r.a[mid]),
                    );
                    if r.a[mid] < r.a[left] {
                        r.rotate(left, mid);
                        mid += 1;
                    }
                    left += 1;
                }
                r.note(format!("merged {} values", hi - lo));
                lo = hi;
            }
            width *= 2;
        }
        r.range = None;
        r.done()
    }

    /// Quicksort with the last value of each range as the pivot (Lomuto's partition), the
    /// ranges taken from a stack, left first.
    pub fn quick(input: &[i64]) -> Self {
        let mut r = Recorder::new(input);
        let mut ranges = vec![(0, input.len())];
        while let Some((lo, hi)) = ranges.pop() {
            if hi <= lo {
                continue;
            }
            if hi - lo == 1 {
                r.settle(lo..=lo, format!("{} is in place", r.a[lo]));
                continue;
            }
            let p = hi - 1;
            r.range = Some((lo, p));
            r.pointers = vec![("pivot".into(), p), ("i".into(), lo)];
            r.note(format!("pivot {}", r.a[p]));
            let mut i = lo;
            for j in lo..p {
                r.pointers = vec![("pivot".into(), p), ("i".into(), i), ("j".into(), j)];
                r.compare(&[j, p], format!("is {} < {}?", r.a[j], r.a[p]));
                if r.a[j] < r.a[p] {
                    if i != j {
                        r.swap(i, j);
                    }
                    i += 1;
                }
            }
            r.pointers = vec![("pivot".into(), p), ("i".into(), i)];
            if i != p {
                r.swap(i, p);
            }
            r.pointers.clear();
            r.settle(i..=i, format!("{} is in place", r.a[i]));
            r.range = None;
            // the left part is sorted first: it goes on the stack last
            ranges.push((i + 1, hi));
            ranges.push((lo, i));
        }
        r.done()
    }

    /// Binary search for `target` in `sorted` (sort it first): the range halves around the
    /// middle cell until the target is found or the range is empty.
    pub fn binary_search(sorted: &[i64], target: i64) -> Self {
        let mut r = Recorder::new(sorted);
        if sorted.is_empty() {
            return ArrayTrace::new(r.steps);
        }
        let (mut lo, mut hi) = (0, sorted.len() - 1);
        loop {
            r.range = Some((lo, hi));
            r.pointers = vec![("lo".into(), lo), ("hi".into(), hi)];
            r.note(format!("search {lo} to {hi}"));
            let mid = lo + (hi - lo) / 2;
            r.pointers.push(("mid".into(), mid));
            let value = r.a[mid];
            r.compare(&[mid], format!("is {value} = {target}?"));
            if value == target {
                r.found = Some(mid);
                r.range = Some((mid, mid));
                r.pointers = vec![("mid".into(), mid)];
                r.note(format!("found {target} at {mid}"));
                break;
            }
            if value < target {
                if mid == hi {
                    break;
                }
                lo = mid + 1;
            } else {
                if mid == lo {
                    break;
                }
                hi = mid - 1;
            }
        }
        if r.found.is_none() {
            r.pointers.clear();
            r.range = None;
            r.note(format!("{target} is not there"));
        }
        ArrayTrace::new(r.steps)
    }
}

/// Records steps while an algorithm runs on `a`.
struct Recorder {
    a: Vec<i64>,
    steps: Vec<ArrayStep>,
    sorted: Vec<usize>,
    pointers: Vec<(String, usize)>,
    range: Option<(usize, usize)>,
    found: Option<usize>,
}

impl Recorder {
    fn new(input: &[i64]) -> Self {
        let mut r = Self {
            a: input.to_vec(),
            steps: Vec::new(),
            sorted: Vec::new(),
            pointers: Vec::new(),
            range: None,
            found: None,
        };
        r.push(Vec::new(), None, String::new());
        r
    }

    fn push(&mut self, compare: Vec<usize>, moved: Option<Vec<usize>>, note: String) {
        let mut sorted = self.sorted.clone();
        sorted.sort_unstable();
        sorted.dedup();
        self.steps.push(ArrayStep {
            values: self.a.clone(),
            compare,
            moved,
            sorted,
            pointers: self.pointers.clone(),
            range: self.range,
            found: self.found,
            note,
        });
    }

    fn compare(&mut self, cells: &[usize], note: String) {
        self.push(cells.to_vec(), None, note);
    }

    fn note(&mut self, note: String) {
        self.push(Vec::new(), None, note);
    }

    fn swap(&mut self, i: usize, j: usize) {
        self.a.swap(i, j);
        let mut moved: Vec<usize> = (0..self.a.len()).collect();
        moved.swap(i, j);
        let note = format!("swap {} and {}", self.a[j], self.a[i]);
        self.push(vec![i, j], Some(moved), note);
    }

    /// Takes the value at `from` out and puts it back at `to` (left of it), shifting the
    /// values between one cell right.
    fn rotate(&mut self, to: usize, from: usize) {
        self.a[to..=from].rotate_right(1);
        let mut moved: Vec<usize> = (0..self.a.len()).collect();
        moved[to..=from].rotate_right(1);
        let note = format!("move {} left", self.a[to]);
        self.push(vec![to], Some(moved), note);
    }

    fn settle(&mut self, cells: impl IntoIterator<Item = usize>, note: String) {
        self.sorted.extend(cells);
        self.push(Vec::new(), None, note);
    }

    /// The trace, its last step marking every cell sorted.
    fn done(mut self) -> ArrayTrace {
        self.sorted = (0..self.a.len()).collect();
        self.pointers.clear();
        self.range = None;
        self.push(Vec::new(), None, "sorted".into());
        ArrayTrace::new(self.steps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const INPUTS: [&[i64]; 6] = [
        &[5, 2, 9, 1, 5, 6, 3],
        &[1, 2, 3, 4],
        &[4, 3, 2, 1],
        &[7],
        &[],
        &[3, 3, 1, 8, -2, 0, 3, 11, 6, 6, -5, 2],
    ];

    fn check(trace: &ArrayTrace, input: &[i64]) {
        let mut expected = input.to_vec();
        expected.sort_unstable();
        assert_eq!(trace.result(), expected.as_slice());
        assert_eq!(trace.steps()[0].values, input);
        for pair in trace.steps().windows(2) {
            let (before, after) = (&pair[0], &pair[1]);
            match &after.moved {
                // every value came from where the step says
                Some(moved) => {
                    for (k, &from) in moved.iter().enumerate() {
                        assert_eq!(after.values[k], before.values[from]);
                    }
                }
                None => assert_eq!(after.values, before.values),
            }
        }
        let last = trace.steps().last().unwrap();
        assert_eq!(last.sorted, (0..input.len()).collect::<Vec<_>>());
    }

    #[test]
    fn sorts_match_the_standard_sort() {
        for input in INPUTS {
            for trace in [
                ArrayTrace::bubble(input),
                ArrayTrace::insertion(input),
                ArrayTrace::merge(input),
                ArrayTrace::quick(input),
            ] {
                check(&trace, input);
            }
        }
    }

    #[test]
    fn counts_add_up() {
        let trace = ArrayTrace::bubble(&[3, 2, 1]);
        // 2 + 1 comparisons, all three pairs out of order
        assert_eq!(trace.totals(), (3, 3));
        // a sorted input stops after one pass with no moves
        assert_eq!(ArrayTrace::bubble(&[1, 2, 3]).totals(), (2, 0));
        assert_eq!(ArrayTrace::insertion(&[1, 2, 3]).totals(), (2, 0));
    }

    #[test]
    fn binary_search_halves_the_range() {
        let sorted = [2, 5, 8, 12, 16, 23, 38, 56, 72, 91];
        let trace = ArrayTrace::binary_search(&sorted, 23);
        let found = trace.steps().last().unwrap();
        assert_eq!(found.found, Some(5));
        // three middles: 16, then 56, then 23
        let middles: Vec<usize> = trace
            .steps()
            .iter()
            .filter(|s| s.compare.len() == 1)
            .map(|s| s.compare[0])
            .collect();
        assert_eq!(middles, vec![4, 7, 5]);
        let missing = ArrayTrace::binary_search(&sorted, 13);
        assert_eq!(missing.steps().last().unwrap().found, None);
        assert!(missing.steps().last().unwrap().note.contains("not there"));
        assert!(trace.steps().iter().all(|s| s.sorted.is_empty()));
    }

    #[test]
    fn sampling_clamps_to_the_ends() {
        let trace = ArrayTrace::bubble(&[2, 1]);
        let at = |t| trace.at(t, 0.5).unwrap().index;
        assert_eq!(
            (at(-1.0), at(0.0), at(0.6), at(100.0)),
            (0, 0, 1, trace.len() - 1)
        );
        assert!(ArrayTrace::default().at(1.0, 0.5).is_none());
    }
}
