//! Binary trees (search trees and heaps) recorded step by step and laid out once per step.

use std::collections::HashSet;

use fframes::{Color, Svgr};
use kurbo::{BezPath, Circle, Point, Vec2};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    geometry::Shape,
    layout::tidy_tree,
    motion::{cubic_in_out, progress},
    text::{TextAnchor, line},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::{
    array::CellColors,
    keyed::{StepAt, find, mix, notes, sample, tween, tween_links},
    list::stroke,
};

/// A node of a binary tree, with an id that stays with it from step to step.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TreeNode {
    pub id: u32,
    pub label: String,
    pub left: Option<u32>,
    pub right: Option<u32>,
}

/// One step of a binary tree.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TreeStep {
    pub nodes: Vec<TreeNode>,
    pub root: Option<u32>,
    /// The nodes this step looks at.
    pub highlight: Vec<u32>,
    /// The nodes passed on the way to the highlighted ones.
    pub path: Vec<u32>,
    /// A node that was searched for and found.
    pub found: Option<u32>,
    /// What happens in this step, in a few words.
    pub note: String,
}

impl TreeStep {
    fn plain(&self) -> bool {
        self.highlight.is_empty() && self.path.is_empty() && self.found.is_none()
    }

    /// The edges from parents to children, by id.
    pub fn edges(&self) -> Vec<(u32, u32)> {
        self.nodes
            .iter()
            .flat_map(|n| {
                [n.left, n.right]
                    .into_iter()
                    .flatten()
                    .map(move |child| (n.id, child))
            })
            .collect()
    }
}

/// Where each node reached from the root sits: `x` in slots, `y` the depth. A node with one
/// child gets an empty place on its other side, so a left child leans left and a right child
/// right, as a binary tree is drawn.
fn layout(step: &TreeStep) -> Vec<(u32, (f64, f64))> {
    let index_of = |id: u32| step.nodes.iter().position(|n| n.id == id);
    let Some(root) = step.root.and_then(index_of) else {
        return Vec::new();
    };
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); step.nodes.len()];
    for (i, node) in step.nodes.iter().enumerate() {
        let (left, right) = (node.left.and_then(index_of), node.right.and_then(index_of));
        if left.is_none() && right.is_none() {
            continue;
        }
        let mut gap = || {
            children.push(Vec::new());
            children.len() - 1
        };
        let pair = [
            left.unwrap_or_else(&mut gap),
            right.unwrap_or_else(&mut gap),
        ];
        children[i] = pair.to_vec();
    }
    let placed = tidy_tree(&children, root);
    // the nodes reached from the root, each once (the layout keeps a node with its first parent)
    let mut seen = HashSet::new();
    let mut stack = vec![root];
    let mut out = Vec::new();
    while let Some(i) = stack.pop() {
        if i >= step.nodes.len() || !seen.insert(i) {
            continue;
        }
        out.push((step.nodes[i].id, placed[i]));
        stack.extend(children[i].iter().rev());
    }
    out
}

/// The steps of a binary tree, recorded once (in a constructor or a `OnceLock`) and sampled
/// by time with [`TreeTrace::at`]. [`TreeTrace::bst`], [`TreeTrace::max_heap`] and
/// [`TreeTrace::min_heap`] start recorders; [`TreeTrace::new`] takes steps of your own. Every
/// step is laid out once, when the trace is made.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct TreeTrace {
    steps: Vec<TreeStep>,
    layouts: Vec<Vec<(u32, (f64, f64))>>,
    depth: f64,
}

impl TreeTrace {
    pub fn new(steps: Vec<TreeStep>) -> Self {
        let layouts: Vec<_> = steps.iter().map(layout).collect();
        let depth = layouts
            .iter()
            .flatten()
            .map(|(_, (_, y))| *y)
            .fold(0.0, f64::max);
        Self {
            steps,
            layouts,
            depth,
        }
    }

    /// Records operations on a binary search tree that starts with `values` inserted in
    /// order.
    pub fn bst(values: &[i64]) -> BstRecorder {
        let mut r = BstRecorder {
            steps: Vec::new(),
            values: Vec::new(),
            kids: Vec::new(),
            alive: Vec::new(),
            root: None,
        };
        for &v in values {
            if let (None, parent, _) = r.descend(v, false) {
                r.attach(v, parent);
            }
        }
        r.steps.push(r.snapshot(String::new()));
        r
    }

    /// Records operations on a heap with the largest value at the root, starting with
    /// `values` pushed in order.
    pub fn max_heap(values: &[i64]) -> HeapRecorder {
        HeapRecorder::new(values, true)
    }

    /// Records operations on a heap with the smallest value at the root.
    pub fn min_heap(values: &[i64]) -> HeapRecorder {
        HeapRecorder::new(values, false)
    }

    pub fn steps(&self) -> &[TreeStep] {
        &self.steps
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Where the nodes of step `index` sit: `x` in slots, `y` the depth.
    pub fn layout(&self, index: usize) -> &[(u32, (f64, f64))] {
        self.layouts.get(index).map_or(&[], Vec::as_slice)
    }

    /// The deepest depth of any step.
    pub fn depth(&self) -> f64 {
        self.depth
    }

    /// The step `seconds` after the start, one step every `step_time` seconds. Before the
    /// start it is step 0, after the end the last step.
    pub fn at(&self, seconds: f32, step_time: f32) -> Option<StepAt<'_, TreeStep>> {
        sample(&self.steps, seconds, step_time)
    }
}

/// Records operations on a binary search tree, from [`TreeTrace::bst`]: every operation
/// walks down from the root comparing, one step a node, and notes which way it goes.
#[derive(Debug, Clone)]
pub struct BstRecorder {
    steps: Vec<TreeStep>,
    values: Vec<i64>,
    kids: Vec<[Option<u32>; 2]>,
    alive: Vec<bool>,
    root: Option<u32>,
}

impl BstRecorder {
    fn snapshot(&self, note: String) -> TreeStep {
        let nodes = (0..self.values.len())
            .filter(|&i| self.alive[i])
            .map(|i| TreeNode {
                id: i as u32,
                label: self.values[i].to_string(),
                left: self.kids[i][0],
                right: self.kids[i][1],
            })
            .collect();
        TreeStep {
            nodes,
            root: self.root,
            note,
            ..TreeStep::default()
        }
    }

    fn value(&self, id: u32) -> i64 {
        self.values[id as usize]
    }

    /// Walks down from the root looking for `v`, recording a step per node when `record`.
    /// Gives the node holding `v`, the last node and side passed, and the nodes passed.
    fn descend(&mut self, v: i64, record: bool) -> (Option<u32>, Option<(u32, usize)>, Vec<u32>) {
        let (mut cur, mut parent, mut path) = (self.root, None, Vec::new());
        while let Some(c) = cur {
            let x = self.value(c);
            if v == x {
                return (Some(c), parent, path);
            }
            let side = usize::from(v > x);
            if record {
                let note = if side == 0 {
                    format!("{v} < {x}, go left")
                } else {
                    format!("{v} > {x}, go right")
                };
                let step = TreeStep {
                    highlight: vec![c],
                    path: path.clone(),
                    ..self.snapshot(note)
                };
                self.steps.push(step);
            }
            path.push(c);
            parent = Some((c, side));
            cur = self.kids[c as usize][side];
        }
        (None, parent, path)
    }

    fn attach(&mut self, v: i64, parent: Option<(u32, usize)>) -> u32 {
        let id = self.values.len() as u32;
        self.values.push(v);
        self.kids.push([None, None]);
        self.alive.push(true);
        self.replace(parent, Some(id));
        id
    }

    /// Puts `node` where the child `side` of `parent` is, or at the root.
    fn replace(&mut self, parent: Option<(u32, usize)>, node: Option<u32>) {
        match parent {
            Some((p, side)) => self.kids[p as usize][side] = node,
            None => self.root = node,
        }
    }

    /// Inserts `v` where the walk down ends.
    pub fn insert(mut self, v: i64) -> Self {
        let (found, parent, path) = self.descend(v, true);
        let step = if let Some(c) = found {
            TreeStep {
                highlight: vec![c],
                path,
                ..self.snapshot(format!("{v} is already there"))
            }
        } else {
            let id = self.attach(v, parent);
            TreeStep {
                highlight: vec![id],
                path,
                ..self.snapshot(format!("insert {v}"))
            }
        };
        self.steps.push(step);
        self
    }

    /// Walks down to `v`.
    pub fn search(mut self, v: i64) -> Self {
        let (found, _, path) = self.descend(v, true);
        let step = match found {
            Some(c) => TreeStep {
                found: Some(c),
                path,
                ..self.snapshot(format!("found {v}"))
            },
            None => TreeStep {
                path,
                ..self.snapshot(format!("{v} is not there"))
            },
        };
        self.steps.push(step);
        self
    }

    /// Removes `v`: a leaf goes, a node with one child is replaced by it, and a node with two
    /// is replaced by the smallest node on its right.
    pub fn remove(mut self, v: i64) -> Self {
        let (found, parent, mut path) = self.descend(v, true);
        let Some(t) = found else {
            let step = TreeStep {
                path,
                ..self.snapshot(format!("{v} is not there"))
            };
            self.steps.push(step);
            return self;
        };
        let step = TreeStep {
            highlight: vec![t],
            path: path.clone(),
            ..self.snapshot(format!("remove {v}"))
        };
        self.steps.push(step);
        let (replacement, note) = match self.kids[t as usize] {
            [None, None] => (None, format!("{v} was a leaf")),
            [Some(c), None] | [None, Some(c)] => (Some(c), format!("{} moves up", self.value(c))),
            [Some(left), Some(right)] => {
                // the smallest on the right: one step right, then left to the end
                path.push(t);
                let (mut sp, mut s) = (t, right);
                let step = TreeStep {
                    highlight: vec![s],
                    path: path.clone(),
                    ..self.snapshot(format!("the smallest right of {v}"))
                };
                self.steps.push(step);
                while let Some(next) = self.kids[s as usize][0] {
                    path.push(s);
                    (sp, s) = (s, next);
                    let step = TreeStep {
                        highlight: vec![s],
                        path: path.clone(),
                        ..self.snapshot(format!("the smallest right of {v}"))
                    };
                    self.steps.push(step);
                }
                if sp != t {
                    self.kids[sp as usize][0] = self.kids[s as usize][1];
                    self.kids[s as usize][1] = Some(right);
                }
                self.kids[s as usize][0] = Some(left);
                (Some(s), format!("{} takes its place", self.value(s)))
            }
        };
        self.replace(parent, replacement);
        self.alive[t as usize] = false;
        let step = TreeStep {
            highlight: replacement.into_iter().collect(),
            ..self.snapshot(note)
        };
        self.steps.push(step);
        self
    }

    /// The values in order, smallest first.
    fn in_order(&self) -> Vec<i64> {
        let mut out = Vec::new();
        let mut stack = Vec::new();
        let mut cur = self.root;
        while cur.is_some() || !stack.is_empty() {
            while let Some(c) = cur {
                stack.push(c);
                cur = self.kids[c as usize][0];
            }
            let Some(c) = stack.pop() else { break };
            out.push(self.value(c));
            cur = self.kids[c as usize][1];
        }
        out
    }

    /// The trace, ending with a step that lights nothing up.
    pub fn finish(mut self) -> TreeTrace {
        if self.steps.last().is_some_and(|s| !s.plain()) {
            let note = self
                .steps
                .last()
                .map(|s| s.note.clone())
                .unwrap_or_default();
            self.steps.push(self.snapshot(note));
        }
        debug_assert!(self.in_order().is_sorted());
        TreeTrace::new(self.steps)
    }
}

/// Records operations on a binary heap, from [`TreeTrace::max_heap`] or
/// [`TreeTrace::min_heap`]: pushed values sift up and the value under a popped root sifts
/// down, one compare and one swap a step. Nodes keep their values, so swapped nodes trade
/// places in the drawing.
#[derive(Debug, Clone)]
pub struct HeapRecorder {
    steps: Vec<TreeStep>,
    values: Vec<i64>,
    heap: Vec<u32>,
    max: bool,
}

impl HeapRecorder {
    fn new(values: &[i64], max: bool) -> Self {
        let mut r = Self {
            steps: Vec::new(),
            values: Vec::new(),
            heap: Vec::new(),
            max,
        };
        for &v in values {
            let id = r.add(v);
            r.heap.push(id);
            r.sift_up(r.heap.len() - 1, false);
        }
        r.steps.push(r.snapshot(String::new()));
        r
    }

    fn add(&mut self, v: i64) -> u32 {
        self.values.push(v);
        (self.values.len() - 1) as u32
    }

    fn value(&self, id: u32) -> i64 {
        self.values[id as usize]
    }

    fn snapshot(&self, note: String) -> TreeStep {
        let at = |i: usize| self.heap.get(i).copied();
        let nodes = self
            .heap
            .iter()
            .enumerate()
            .map(|(i, &id)| TreeNode {
                id,
                label: self.value(id).to_string(),
                left: at(2 * i + 1),
                right: at(2 * i + 2),
            })
            .collect();
        TreeStep {
            nodes,
            root: at(0),
            note,
            ..TreeStep::default()
        }
    }

    /// Whether `a` belongs above `b`.
    fn above(&self, a: u32, b: u32) -> bool {
        let (a, b) = (self.value(a), self.value(b));
        if self.max { a > b } else { a < b }
    }

    fn word(&self) -> &'static str {
        if self.max { "larger" } else { "smaller" }
    }

    fn record(&mut self, highlight: Vec<u32>, note: String) {
        let step = TreeStep {
            highlight,
            ..self.snapshot(note)
        };
        self.steps.push(step);
    }

    fn sift_up(&mut self, mut i: usize, record: bool) {
        while i > 0 {
            let parent = (i - 1) / 2;
            let (c, q) = (self.heap[i], self.heap[parent]);
            let (cv, qv, word) = (self.value(c), self.value(q), self.word());
            if !self.above(c, q) {
                if record {
                    self.record(
                        vec![c, q],
                        format!("{cv} is not {word} than {qv}, it stays"),
                    );
                }
                return;
            }
            let note = format!("{cv} is {word} than {qv}, swap");
            if record {
                self.record(vec![c, q], note.clone());
            }
            self.heap.swap(i, parent);
            if record {
                self.record(vec![c, q], note);
            }
            i = parent;
        }
    }

    fn sift_down(&mut self, mut i: usize) {
        loop {
            let (l, r) = (2 * i + 1, 2 * i + 2);
            let Some(&left) = self.heap.get(l) else {
                return;
            };
            let child = match self.heap.get(r) {
                Some(&right) if self.above(right, left) => r,
                _ => l,
            };
            let (c, q) = (self.heap[child], self.heap[i]);
            let (cv, qv, word) = (self.value(c), self.value(q), self.word());
            if !self.above(c, q) {
                self.record(
                    vec![c, q],
                    format!("{cv} is not {word} than {qv}, it stays"),
                );
                return;
            }
            let note = format!("{cv} is {word} than {qv}, swap");
            self.record(vec![c, q], note.clone());
            self.heap.swap(i, child);
            self.record(vec![c, q], note);
            i = child;
        }
    }

    /// Adds `v` at the bottom and sifts it up.
    pub fn push(mut self, v: i64) -> Self {
        let id = self.add(v);
        self.heap.push(id);
        self.record(vec![id], format!("push {v}"));
        self.sift_up(self.heap.len() - 1, true);
        self
    }

    /// Takes the root off, moves the last node up and sifts it down.
    pub fn pop(mut self) -> Self {
        let Some(&root) = self.heap.first() else {
            self.record(vec![], "the heap is empty".into());
            return self;
        };
        let note = format!("pop {}", self.value(root));
        self.record(vec![root], note.clone());
        let last = self.heap.pop().unwrap_or(root);
        if self.heap.is_empty() {
            self.record(vec![], note);
            return self;
        }
        self.heap[0] = last;
        let note = format!("{} moves to the root", self.value(last));
        self.record(vec![last], note);
        self.sift_down(0);
        self
    }

    /// The trace, ending with a step that lights nothing up.
    pub fn finish(mut self) -> TreeTrace {
        if self.steps.last().is_some_and(|s| !s.plain()) {
            let note = self
                .steps
                .last()
                .map(|s| s.note.clone())
                .unwrap_or_default();
            self.steps.push(self.snapshot(note));
        }
        TreeTrace::new(self.steps)
    }
}

/// A binary tree: round nodes `radius` pixels with their values, `spacing` pixels between
/// neighbouring slots and `level` pixels between depths, centred on `center` (units of
/// `ui.space`). It plays a [`TreeTrace`], one step every `step_time` seconds from `delay`:
/// the nodes a step looks at light up, the path to them is outlined, nodes slide to their new
/// places as the tree changes (swapped heap nodes pass each other), new nodes drop in and
/// removed ones fade. The whole figure fades with `enter` and `exit`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct TreeView<'a> {
    pub trace: Option<&'a TreeTrace>,
    pub delay: f32,
    pub step_time: f32,
    pub center: (f64, f64),
    pub spacing: f32,
    pub level: f32,
    pub radius: f32,
    pub notes: bool,
    pub colors: CellColors,
    pub edge_color: Color,
    pub path_color: Color,
    pub value_style: TextStyle,
    pub small_style: TextStyle,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for TreeView<'_> {
    fn themed(t: &Theme) -> Self {
        let mut value_style = t.typography.body;
        value_style.weight = 600;
        Self {
            trace: None,
            delay: 0.0,
            step_time: 0.8,
            center: (0.0, 0.0),
            spacing: 100.0,
            level: 120.0,
            radius: 30.0,
            notes: true,
            colors: CellColors::themed(t),
            edge_color: t.colors.muted,
            path_color: t.colors.accent,
            value_style,
            small_style: t.typography.caption,
            enter: None,
            exit: None,
        }
    }
}

impl Default for TreeView<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl TreeView<'_> {
    fn fill(&self, step: &TreeStep, id: u32) -> Color {
        if step.found == Some(id) {
            self.colors.found
        } else if step.highlight.contains(&id) {
            self.colors.compare
        } else {
            self.colors.base
        }
    }
}

impl<'a> Component<'a> for TreeView<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let Some(trace) = self.trace else {
            return Svgr::empty();
        };
        let Some(at) = trace.at(ui.seconds() - self.delay, self.step_time) else {
            return Svgr::empty();
        };
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let (spacing, level, radius) = (
            f64::from(self.spacing),
            f64::from(self.level),
            f64::from(self.radius),
        );
        let top = middle.y - trace.depth() * level / 2.0;
        // each step centred on its own width, so the tree stays in the middle as it grows
        let place = |index: usize| -> Vec<(u32, Point)> {
            let layout = trace.layout(index);
            let (lo, hi) = layout
                .iter()
                .fold((f64::MAX, f64::MIN), |(lo, hi), (_, (x, _))| {
                    (lo.min(*x), hi.max(*x))
                });
            let mid = f64::midpoint(lo, hi);
            layout
                .iter()
                .map(|&(id, (x, y))| {
                    (
                        id,
                        Point::new(middle.x + (x - mid) * spacing, top + y * level),
                    )
                })
                .collect()
        };
        let move_time = self.step_time * 0.7;
        let blend = (self.step_time * 0.3).min(0.2);
        let p = f64::from(cubic_in_out(progress(at.into, 0.0, move_time)));
        let q = progress(at.into, 0.0, blend);
        let (before, now) = (place(at.index.saturating_sub(1)), place(at.index));
        let tweens = tween(
            &before,
            &now,
            p,
            Vec2::new(0.0, -level * 0.4),
            Vec2::new(0.0, -level * 0.4),
            0.3,
        );

        let mut edges = Vec::new();
        for (a, b, opacity) in tween_links(&at.previous.edges(), &at.step.edges(), p, false) {
            let (Some(a), Some(b)) = (find(&tweens, a), find(&tweens, b)) else {
                continue;
            };
            let d = b.at - a.at;
            if d.hypot() <= 2.0 * radius {
                continue;
            }
            let dir = d.normalize();
            let mut path = BezPath::new();
            path.move_to(a.at + dir * radius);
            path.line_to(b.at - dir * radius);
            let opacity = (opacity * a.opacity.min(b.opacity)) as f32;
            edges.push(stroke(path, self.edge_color, 3.0, opacity).draw());
        }

        let on_path = |step: &TreeStep, id: u32| if step.path.contains(&id) { 1.0 } else { 0.0 };
        let mut nodes: Vec<(bool, Svgr<'a>)> = Vec::new();
        for t in &tweens {
            let (step, other) = match t.to {
                Some(_) => (at.step, t.from.map(|_| at.previous)),
                None => (at.previous, None),
            };
            let label = step
                .nodes
                .iter()
                .find(|n| n.id == t.id)
                .map_or("", |n| n.label.as_str());
            let now = self.fill(step, t.id);
            let (fill, outline) = match other {
                Some(was) => (
                    mix(self.fill(was, t.id), now, q),
                    on_path(was, t.id) + (on_path(step, t.id) - on_path(was, t.id)) * q,
                ),
                None => (now, on_path(step, t.id)),
            };
            let opacity = t.opacity as f32;
            let body = Shape {
                path: kurbo::Shape::to_path(&Circle::new(t.at, radius), 0.1),
                fill: Some(fill),
                stroke: Some(mix(
                    self.colors.text.with_alpha(64),
                    self.path_color,
                    outline,
                )),
                stroke_width: 1.5 + 2.5 * outline,
                dash: None,
                opacity,
            };
            let text = line(
                t.at.x as f32,
                (t.at.y + self.value_style.size as f64 * 0.35) as f32,
                self.value_style,
                self.colors.text_on(fill),
                TextAnchor::Middle,
                0.0,
                opacity,
                label.to_owned().into(),
            );
            nodes.push((t.crossing, fframes::svgr!(<g>{body.draw()}{text}</g>)));
        }
        nodes.sort_by_key(|(crossing, _)| *crossing);
        let nodes: Vec<Svgr<'a>> = nodes.into_iter().map(|(_, node)| node).collect();

        let mut parts = vec![edges.into(), nodes.into()];
        if self.notes {
            let y = top + trace.depth() * level + radius + 44.0;
            parts.push(notes(
                &at.previous.note,
                &at.step.note,
                at.into,
                middle.x as f32,
                y as f32,
                self.small_style,
                self.colors.text,
                blend,
            ));
        }
        fade(ui, parts.into(), self.enter.as_ref(), self.exit.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn in_order(step: &TreeStep) -> Vec<i64> {
        fn walk(step: &TreeStep, id: Option<u32>, out: &mut Vec<i64>) {
            let Some(node) = id.and_then(|id| step.nodes.iter().find(|n| n.id == id)) else {
                return;
            };
            walk(step, node.left, out);
            out.push(node.label.parse().unwrap());
            walk(step, node.right, out);
        }
        let mut out = Vec::new();
        walk(step, step.root, &mut out);
        out
    }

    #[test]
    fn search_trees_stay_sorted() {
        let trace = TreeTrace::bst(&[50, 30, 70, 20, 40, 60, 80])
            .insert(35)
            .search(60)
            .remove(30)
            .remove(20)
            .remove(70)
            .remove(99)
            .finish();
        for step in trace.steps() {
            assert!(in_order(step).is_sorted(), "{step:?}");
        }
        assert_eq!(
            in_order(trace.steps().last().unwrap()),
            [35, 40, 50, 60, 80]
        );
        // 35 walks 50 (left), 30 (right), 40 (left) and goes under 40
        let insert = &trace.steps()[4];
        assert_eq!(
            (insert.note.as_str(), insert.path.as_slice()),
            ("insert 35", &[0, 1, 4][..])
        );
        assert!(trace.steps().iter().any(|s| s.note == "35 takes its place"));
        assert!(trace.steps().iter().any(|s| s.found.is_some()));
    }

    #[test]
    fn heaps_keep_the_largest_on_top() {
        let trace = TreeTrace::max_heap(&[])
            .push(4)
            .push(9)
            .push(2)
            .push(7)
            .pop()
            .finish();
        for step in trace.steps() {
            for node in &step.nodes {
                let v: i64 = node.label.parse().unwrap();
                for child in [node.left, node.right].into_iter().flatten() {
                    let c = step.nodes.iter().find(|n| n.id == child).unwrap();
                    // only the pair being swapped may be out of order
                    if !step.highlight.contains(&child) && !step.highlight.contains(&node.id) {
                        assert!(c.label.parse::<i64>().unwrap() <= v, "{step:?}");
                    }
                }
            }
        }
        let last = trace.steps().last().unwrap();
        let root = last.nodes.iter().find(|n| Some(n.id) == last.root).unwrap();
        assert_eq!(root.label, "7");
        assert_eq!(last.nodes.len(), 3);
        let min = TreeTrace::min_heap(&[5, 3, 8, 1]).finish();
        let first = &min.steps()[0];
        assert_eq!(first.nodes[0].label, "1");
    }

    #[test]
    fn layouts_lean_single_children_to_their_side() {
        let trace = TreeTrace::bst(&[2, 1, 3, 4]).finish();
        let layout = trace.layout(0);
        let x = |id: u32| layout.iter().find(|(i, _)| *i == id).unwrap().1;
        // 4 is the right child of 3: one slot right of it and one level down
        assert_eq!(x(3).1 - x(2).1, 1.0);
        assert!(x(3).0 > x(2).0);
        assert!(x(1).0 < x(0).0 && x(0).0 < x(2).0);
        assert_eq!(trace.depth(), 2.0);
    }
}
