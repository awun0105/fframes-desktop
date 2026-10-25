//! Graphs and their searches (breadth first, depth first, Dijkstra) recorded step by step.

use std::collections::VecDeque;

use fframes::{Color, Svgr};
use kurbo::{Circle, CubicBez, ParamCurve, ParamCurveExtrema, Point, RoundedRect};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    geometry::Shape,
    layout::{GraphLayout, graph_layout, layered},
    motion::progress,
    text::{TextAnchor, line},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::{
    array::CellColors,
    keyed::{StepAt, arrow_along, clear_bend, link, mix, notes, sample},
};

/// A graph with named nodes and weighted edges, placed in slots (`x` right) and layers (`y`
/// down) by [`layered`](crate::layout::layered) unless [`Graph::at`] places it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Graph {
    pub nodes: Vec<String>,
    /// `(from, to, weight)`; an undirected edge goes both ways.
    pub edges: Vec<(usize, usize, f64)>,
    pub directed: bool,
    /// Whether the weights are written on the edges.
    pub weighted: bool,
    pub positions: Vec<(f64, f64)>,
}

impl Graph {
    /// A graph whose edges all weigh 1. Edges to nodes that do not exist are dropped.
    pub fn new(nodes: &[&str], edges: &[(usize, usize)], directed: bool) -> Self {
        let edges: Vec<_> = edges.iter().map(|&(a, b)| (a, b, 1.0)).collect();
        Self {
            weighted: false,
            ..Self::weighted(nodes, &edges, directed)
        }
    }

    /// A graph with weights on its edges, written on them.
    pub fn weighted(nodes: &[&str], edges: &[(usize, usize, f64)], directed: bool) -> Self {
        let n = nodes.len();
        let edges: Vec<_> = edges
            .iter()
            .copied()
            .filter(|&(a, b, _)| a < n && b < n)
            .collect();
        let pairs: Vec<_> = edges.iter().map(|&(a, b, _)| (a, b)).collect();
        Self {
            nodes: nodes.iter().map(|&s| s.to_owned()).collect(),
            edges,
            directed,
            weighted: true,
            positions: layered(n, &pairs),
        }
    }

    /// Places the nodes yourself, in slots and layers like the layout.
    pub fn at(mut self, positions: &[(f64, f64)]) -> Self {
        for (place, &at) in self.positions.iter_mut().zip(positions) {
            *place = at;
        }
        self
    }

    /// Places the nodes with one of the shared graph layouts (circular, Kamada-Kawai, spring,
    /// tree, ...) instead of the layers. The farthest node is `scale` slots from the middle
    /// across and `scale` layers down or up (a slot is `spacing` pixels of the view, a layer
    /// `level` pixels). Edges count both ways, whether the graph is directed or not.
    pub fn layout(self, layout: &GraphLayout, scale: f64) -> Self {
        let pairs: Vec<_> = self.edges.iter().map(|&(a, b, _)| (a, b)).collect();
        // the layouts have `y` up, the view `y` down
        let positions: Vec<_> = graph_layout(layout, self.nodes.len(), &pairs, scale)
            .into_iter()
            .map(|(x, y)| (x, -y))
            .collect();
        self.at(&positions)
    }

    /// The edges leaving `u` in the order given: `(edge, other end, weight)`.
    pub fn neighbours(&self, u: usize) -> Vec<(usize, usize, f64)> {
        self.edges
            .iter()
            .enumerate()
            .filter_map(|(e, &(a, b, w))| {
                if a == u {
                    Some((e, b, w))
                } else if b == u && !self.directed {
                    Some((e, a, w))
                } else {
                    None
                }
            })
            .collect()
    }
}

/// One step of a graph search.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GraphStep {
    /// The node being looked at.
    pub current: Option<usize>,
    /// Nodes found and waiting: the queue of a breadth first search, the stack of a depth
    /// first one, the nodes with a distance for Dijkstra.
    pub frontier: Vec<usize>,
    /// Nodes that are finished.
    pub done: Vec<usize>,
    /// The edges of the search tree so far (the shortest paths for Dijkstra).
    pub tree: Vec<usize>,
    /// The edge followed in this step.
    pub edge: Option<usize>,
    /// The distances, when the search keeps them (`None` is not reached yet).
    pub dist: Vec<Option<f64>>,
    /// What happens in this step, in a few words.
    pub note: String,
}

/// The steps of a search of a [`Graph`], recorded once (in a constructor or a `OnceLock`)
/// and sampled by time with [`GraphTrace::at`]: [`bfs`](GraphTrace::bfs),
/// [`dfs`](GraphTrace::dfs) and [`dijkstra`](GraphTrace::dijkstra) record a search;
/// [`GraphTrace::new`] takes steps of your own.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GraphTrace {
    graph: Graph,
    steps: Vec<GraphStep>,
    /// What the frontier is called under the graph (`queue`, `stack`), empty for none.
    container: &'static str,
}

struct Recorder<'g> {
    graph: &'g Graph,
    steps: Vec<GraphStep>,
    frontier: Vec<usize>,
    done: Vec<usize>,
    tree: Vec<usize>,
    dist: Vec<Option<f64>>,
}

impl<'g> Recorder<'g> {
    fn new(graph: &'g Graph) -> Self {
        Self {
            graph,
            steps: Vec::new(),
            frontier: Vec::new(),
            done: Vec::new(),
            tree: Vec::new(),
            dist: Vec::new(),
        }
    }

    fn name(&self, v: usize) -> &str {
        &self.graph.nodes[v]
    }

    fn step(&mut self, current: Option<usize>, edge: Option<usize>, note: String) {
        self.steps.push(GraphStep {
            current,
            frontier: self.frontier.clone(),
            done: self.done.clone(),
            tree: self.tree.clone(),
            edge,
            dist: self.dist.clone(),
            note,
        });
    }
}

impl GraphTrace {
    pub fn new(graph: Graph, steps: Vec<GraphStep>) -> Self {
        Self {
            graph,
            steps,
            container: "",
        }
    }

    /// Breadth first from `start`: nodes leave a queue in the order they were found, and
    /// each discovers its neighbours not seen yet.
    pub fn bfs(graph: &Graph, start: usize) -> Self {
        let mut r = Recorder::new(graph);
        let n = graph.nodes.len();
        if start >= n {
            return Self::new(graph.clone(), Vec::new());
        }
        let mut seen = vec![false; n];
        let mut queue = VecDeque::from([start]);
        seen[start] = true;
        r.frontier = vec![start];
        r.step(None, None, format!("start at {}", r.name(start)));
        while let Some(u) = queue.pop_front() {
            r.frontier = queue.iter().copied().collect();
            r.step(Some(u), None, format!("visit {}", r.name(u)));
            for (e, v, _) in graph.neighbours(u) {
                if seen[v] {
                    continue;
                }
                seen[v] = true;
                queue.push_back(v);
                r.frontier = queue.iter().copied().collect();
                r.tree.push(e);
                r.step(Some(u), Some(e), format!("discover {}", r.name(v)));
            }
            r.done.push(u);
        }
        r.step(None, None, "every node reached is done".into());
        Self {
            container: "queue",
            ..Self::new(graph.clone(), r.steps)
        }
    }

    /// Depth first from `start`: it goes to the first neighbour not seen yet, and back when a
    /// node has none left. The frontier is the stack of nodes on the way.
    pub fn dfs(graph: &Graph, start: usize) -> Self {
        let mut r = Recorder::new(graph);
        let n = graph.nodes.len();
        if start >= n {
            return Self::new(graph.clone(), Vec::new());
        }
        let mut seen = vec![false; n];
        seen[start] = true;
        let mut stack = vec![(start, 0)];
        r.frontier = vec![start];
        r.step(Some(start), None, format!("visit {}", r.name(start)));
        while let Some(&(u, i)) = stack.last() {
            let next = graph.neighbours(u).get(i).copied();
            if let Some(top) = stack.last_mut() {
                top.1 += 1;
            }
            match next {
                Some((_, v, _)) if seen[v] => {}
                Some((e, v, _)) => {
                    seen[v] = true;
                    stack.push((v, 0));
                    r.frontier = stack.iter().map(|&(s, _)| s).collect();
                    r.tree.push(e);
                    r.step(Some(v), Some(e), format!("go to {}", r.name(v)));
                }
                None => {
                    stack.pop();
                    r.done.push(u);
                    r.frontier = stack.iter().map(|&(s, _)| s).collect();
                    if let Some(&(p, _)) = stack.last() {
                        r.step(Some(p), None, format!("back to {}", r.name(p)));
                    }
                }
            }
        }
        r.step(None, None, "every node reached is done".into());
        Self {
            container: "stack",
            ..Self::new(graph.clone(), r.steps)
        }
    }

    /// Dijkstra's shortest paths from `start` (weights must not be negative): the closest
    /// node not finished yet relaxes its edges, one edge a step, keeping a shorter distance
    /// and the edge it came by.
    pub fn dijkstra(graph: &Graph, start: usize) -> Self {
        let mut r = Recorder::new(graph);
        let n = graph.nodes.len();
        if start >= n {
            return Self::new(graph.clone(), Vec::new());
        }
        let mut parent: Vec<Option<usize>> = vec![None; n];
        let mut finished = vec![false; n];
        r.dist = vec![None; n];
        r.dist[start] = Some(0.0);
        r.frontier = vec![start];
        r.step(None, None, format!("{} starts at 0", r.name(start)));
        loop {
            let closest = (0..n)
                .filter(|&v| !finished[v])
                .filter_map(|v| r.dist[v].map(|d| (v, d)))
                .min_by(|a, b| a.1.total_cmp(&b.1));
            let Some((u, d)) = closest else { break };
            r.step(Some(u), None, format!("closest: {} at {d}", r.name(u)));
            for (e, v, w) in graph.neighbours(u) {
                if finished[v] {
                    continue;
                }
                let through = d + w;
                let note = match r.dist[v] {
                    Some(old) if old <= through => {
                        format!("{}: {d} + {w} is not below {old}", r.name(v))
                    }
                    _ => {
                        r.dist[v] = Some(through);
                        parent[v] = Some(e);
                        r.tree = parent.iter().flatten().copied().collect();
                        if !r.frontier.contains(&v) {
                            r.frontier.push(v);
                        }
                        format!("{}: {d} + {w} = {through}", r.name(v))
                    }
                };
                r.step(Some(u), Some(e), note);
            }
            finished[u] = true;
            r.done.push(u);
            r.frontier.retain(|&v| v != u);
        }
        r.step(None, None, format!("shortest paths from {}", r.name(start)));
        Self::new(graph.clone(), r.steps)
    }

    pub fn graph(&self) -> &Graph {
        &self.graph
    }

    pub fn steps(&self) -> &[GraphStep] {
        &self.steps
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// The order the nodes were finished in.
    pub fn order(&self) -> &[usize] {
        self.steps.last().map_or(&[], |s| &s.done)
    }

    /// The step `seconds` after the start, one step every `step_time` seconds. Before the
    /// start it is step 0, after the end the last step.
    pub fn at(&self, seconds: f32, step_time: f32) -> Option<StepAt<'_, GraphStep>> {
        sample(&self.steps, seconds, step_time)
    }
}

/// A graph search: round nodes `radius` pixels with their names, `spacing` pixels between
/// slots and `level` pixels between layers, centred on `center` (units of `ui.space`). It
/// plays a [`GraphTrace`], one step every `step_time` seconds from `delay`: the current node
/// lights up, nodes waiting in the frontier take `colors.moved`, finished ones
/// `colors.sorted`, the edge followed lights up and the search tree's edges thicken. A long
/// edge bends round the nodes in its way, and the two edges of a pair that point both ways
/// bend apart. With `curved` every edge is an arc. A backward edge of a directed graph (one
/// that points up against the layers, or left within a layer) always curves, wide enough to
/// pass beside the forward edges instead of over them. Weights sit on the middle of their edge, on a small
/// `weight_fill` plate. Distances (Dijkstra) are written above the nodes, the frontier
/// (`queue: A, B`) and the note under the graph. The whole figure fades with `enter` and
/// `exit`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct GraphView<'a> {
    pub trace: Option<&'a GraphTrace>,
    pub delay: f32,
    pub step_time: f32,
    pub center: (f64, f64),
    pub spacing: f32,
    pub level: f32,
    pub radius: f32,
    pub frontier: bool,
    pub notes: bool,
    /// Every edge an arc, not only the backward ones and the pairs.
    pub curved: bool,
    pub colors: CellColors,
    pub edge_color: Color,
    pub tree_color: Color,
    pub active_color: Color,
    pub dist_color: Color,
    /// The plate behind a weight, so the edge does not run through the digits.
    pub weight_fill: Color,
    pub value_style: TextStyle,
    pub small_style: TextStyle,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for GraphView<'_> {
    fn themed(t: &Theme) -> Self {
        let mut value_style = t.typography.body;
        value_style.weight = 600;
        Self {
            trace: None,
            delay: 0.0,
            step_time: 0.8,
            center: (0.0, 0.0),
            spacing: 170.0,
            level: 150.0,
            radius: 34.0,
            frontier: true,
            notes: true,
            curved: false,
            colors: CellColors::themed(t),
            edge_color: t.colors.muted,
            tree_color: t.colors.accent,
            active_color: t.colors.highlight,
            dist_color: t.colors.highlight,
            weight_fill: t.colors.background,
            value_style,
            small_style: t.typography.caption,
            enter: None,
            exit: None,
        }
    }
}

impl Default for GraphView<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl GraphView<'_> {
    fn fill(&self, step: &GraphStep, v: usize) -> Color {
        if step.current == Some(v) {
            self.colors.compare
        } else if step.frontier.contains(&v) {
            self.colors.moved
        } else if step.done.contains(&v) {
            self.colors.sorted
        } else {
            self.colors.base
        }
    }

    /// A plate in `weight_fill` behind `label` written in `small_style` around `center`.
    fn plate(&self, ui: &Ui<'_, '_, '_>, center: Point, label: &str) -> Shape {
        let size = self.small_style.size as f64;
        let width = ui
            .measure(self.small_style.font(), label)
            .map_or(label.chars().count() as f64 * size * 0.55, f64::from);
        let (half_w, half_h) = (width / 2.0 + 6.0, size * 0.62);
        let rect = RoundedRect::new(
            center.x - half_w,
            center.y - half_h,
            center.x + half_w,
            center.y + half_h,
            half_h * 0.6,
        );
        Shape {
            path: kurbo::Shape::to_path(&rect, 0.1),
            fill: Some(self.weight_fill),
            stroke: None,
            stroke_width: 0.0,
            dash: None,
            opacity: 1.0,
        }
    }

    /// The colour and width of edge `e` in `step`.
    fn edge_look(&self, step: &GraphStep, e: usize) -> (Color, f32) {
        if step.edge == Some(e) {
            (self.active_color, 5.0)
        } else if step.tree.contains(&e) {
            (self.tree_color, 4.5)
        } else {
            (self.edge_color, 2.5)
        }
    }
}

fn distance(d: Option<f64>) -> String {
    d.map_or_else(|| "∞".to_owned(), |d| d.to_string())
}

/// Which edges of a directed graph point backward: the edges that close a cycle, found by a
/// depth first search from the nodes in order (an edge to a node still on the way). The layered
/// layout turns exactly these edges against the layers. An undirected graph has none.
fn back_edges(graph: &Graph) -> Vec<bool> {
    let n = graph.nodes.len();
    let mut back = vec![false; graph.edges.len()];
    if !graph.directed {
        return back;
    }
    let out: Vec<Vec<usize>> = (0..n)
        .map(|u| {
            (0..graph.edges.len())
                .filter(|&e| graph.edges[e].0 == u)
                .collect()
        })
        .collect();
    // 0 not seen, 1 on the way, 2 finished
    let mut state = vec![0_u8; n];
    for start in 0..n {
        if state[start] != 0 {
            continue;
        }
        state[start] = 1;
        let mut stack = vec![(start, 0)];
        while let Some(top) = stack.last_mut() {
            let (u, i) = *top;
            top.1 += 1;
            let Some(&e) = out[u].get(i) else {
                state[u] = 2;
                stack.pop();
                continue;
            };
            let v = graph.edges[e].1;
            match state[v] {
                0 => {
                    state[v] = 1;
                    stack.push((v, 0));
                }
                1 => back[e] = v != u,
                _ => {}
            }
        }
    }
    back
}

/// The curve of edge `e` between round nodes `radius` pixels centred on `points`. It is
/// straight unless `curved`, unless it is `backward` (bent by a third of its length, so it
/// passes beside the forward edges) or its reverse is an edge too (the two bend apart). Any
/// edge bends further when a node is in its way.
fn edge_curve(
    graph: &Graph,
    points: &[Point],
    e: usize,
    radius: f64,
    curved: bool,
    backward: bool,
) -> CubicBez {
    let (a, b, _) = graph.edges[e];
    let length = (points[b] - points[a]).hypot();
    let others: Vec<Point> = (0..points.len())
        .filter(|&v| v != a && v != b)
        .map(|v| points[v])
        .collect();
    let paired = graph.directed && graph.edges.iter().any(|&(c, d, _)| (c, d) == (b, a));
    let mut base: f64 = 0.0;
    if curved {
        base = base.max(0.15 * length);
    }
    if paired {
        base = base.max(26.0);
    }
    if backward {
        base = base.max((length / 3.0).max(48.0));
    }
    let bend = clear_bend(points[a], points[b], radius, &others, radius + 8.0, base);
    link(points[a], points[b], radius, radius, bend)
}

/// Where the weight of an edge is written: on the middle of its curve.
fn weight_place(curve: &CubicBez) -> Point {
    curve.eval(0.5)
}

impl<'a> Component<'a> for GraphView<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let Some(trace) = self.trace else {
            return Svgr::empty();
        };
        let Some(at) = trace.at(ui.seconds() - self.delay, self.step_time) else {
            return Svgr::empty();
        };
        let graph = trace.graph();
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let (spacing, level, radius) = (
            f64::from(self.spacing),
            f64::from(self.level),
            f64::from(self.radius),
        );
        let (lo, hi) = graph.positions.iter().fold(
            ((f64::MAX, f64::MAX), (f64::MIN, f64::MIN)),
            |(lo, hi), &(x, y)| ((lo.0.min(x), lo.1.min(y)), (hi.0.max(x), hi.1.max(y))),
        );
        let (mx, my) = (f64::midpoint(lo.0, hi.0), f64::midpoint(lo.1, hi.1));
        let points: Vec<Point> = graph
            .positions
            .iter()
            .map(|&(x, y)| Point::new(middle.x + (x - mx) * spacing, middle.y + (y - my) * level))
            .collect();
        // under the nodes and under any edge that curves below them
        let mut bottom = middle.y + (hi.1 - my) * level + radius;
        let q = progress(at.into, 0.0, (self.step_time * 0.3).min(0.2));

        let back = back_edges(graph);
        let mut edges = Vec::new();
        let mut weights = Vec::new();
        for (e, &(a, b, w)) in graph.edges.iter().enumerate() {
            if a == b {
                continue;
            }
            let curve = edge_curve(graph, &points, e, radius, self.curved, back[e]);
            bottom = bottom.max(curve.bounding_box().y1 + 4.0);
            let (was, was_width) = self.edge_look(at.previous, e);
            let (now, now_width) = self.edge_look(at.step, e);
            let width = was_width + (now_width - was_width) * q;
            edges.push(arrow_along(
                curve,
                graph.directed,
                mix(was, now, q),
                width,
                1.0,
            ));
            if graph.weighted {
                let place = weight_place(&curve);
                let label = w.to_string();
                let size = self.small_style.size as f64;
                let plate = self.plate(ui, place, &label);
                let text = line(
                    place.x as f32,
                    (place.y + size * 0.35) as f32,
                    self.small_style,
                    self.colors.text,
                    TextAnchor::Middle,
                    0.0,
                    1.0,
                    label.into(),
                );
                weights.push(fframes::svgr!(<g>{plate.draw()}{text}</g>));
            }
        }

        let mut nodes = Vec::new();
        let mut dists = Vec::new();
        for (v, name) in graph.nodes.iter().enumerate() {
            let p = points[v];
            let fill = mix(self.fill(at.previous, v), self.fill(at.step, v), q);
            let body = Shape {
                path: kurbo::Shape::to_path(&Circle::new(p, radius), 0.1),
                fill: Some(fill),
                stroke: Some(self.colors.text.with_alpha(64)),
                stroke_width: 1.5,
                dash: None,
                opacity: 1.0,
            };
            let text = line(
                p.x as f32,
                (p.y + self.value_style.size as f64 * 0.35) as f32,
                self.value_style,
                self.colors.text_on(fill),
                TextAnchor::Middle,
                0.0,
                1.0,
                name.as_str().into(),
            );
            nodes.push(fframes::svgr!(<g>{body.draw()}{text}</g>));
            if !at.step.dist.is_empty() {
                let label = |step: &GraphStep| distance(step.dist.get(v).copied().flatten());
                let was = if at.previous.dist.is_empty() {
                    label(at.step)
                } else {
                    label(at.previous)
                };
                let now = label(at.step);
                // on a plate, so an edge coming in from above does not run through it
                let wider = if was.chars().count() > now.chars().count() {
                    &was
                } else {
                    &now
                };
                let baseline = p.y - radius - 10.0;
                let center = Point::new(p.x, baseline - self.small_style.size as f64 * 0.35);
                dists.push(self.plate(ui, center, wider).draw());
                dists.push(notes(
                    &was,
                    &now,
                    at.into,
                    p.x as f32,
                    baseline as f32,
                    self.small_style,
                    self.dist_color,
                    (self.step_time * 0.3).min(0.2),
                ));
            }
        }

        let mut parts = vec![edges.into(), weights.into(), nodes.into(), dists.into()];
        let blend = (self.step_time * 0.3).min(0.2);
        let mut y = bottom + 30.0 + self.small_style.size as f64 * 0.8;
        if self.frontier && !trace.container.is_empty() {
            let names = |step: &GraphStep| {
                let names: Vec<&str> = step
                    .frontier
                    .iter()
                    .map(|&v| graph.nodes[v].as_str())
                    .collect();
                if names.is_empty() {
                    format!("the {} is empty", trace.container)
                } else {
                    format!("{}: {}", trace.container, names.join(", "))
                }
            };
            parts.push(notes(
                &names(at.previous),
                &names(at.step),
                at.into,
                middle.x as f32,
                y as f32,
                self.small_style,
                self.colors.moved,
                blend,
            ));
            y += self.small_style.size as f64 * 1.6;
        }
        if self.notes {
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

    fn sample_graph() -> Graph {
        // A - B, A - C, B - D, C - D, D - E
        Graph::new(
            &["A", "B", "C", "D", "E"],
            &[(0, 1), (0, 2), (1, 3), (2, 3), (3, 4)],
            false,
        )
    }

    #[test]
    fn breadth_first_finishes_by_distance() {
        let trace = GraphTrace::bfs(&sample_graph(), 0);
        assert_eq!(trace.order(), [0, 1, 2, 3, 4]);
        // the tree reaches every node once
        assert_eq!(trace.steps().last().unwrap().tree, [0, 1, 2, 4]);
    }

    #[test]
    fn depth_first_goes_deep_then_back() {
        let trace = GraphTrace::dfs(&sample_graph(), 0);
        // A, B, D, then C from D, then E
        let visits: Vec<_> = trace
            .steps()
            .iter()
            .filter(|s| s.note.starts_with("go to") || s.note.starts_with("visit"))
            .filter_map(|s| s.current)
            .collect();
        assert_eq!(visits, [0, 1, 3, 2, 4]);
        assert_eq!(trace.order(), [2, 4, 3, 1, 0]);
        assert!(trace.steps().iter().any(|s| s.note == "back to D"));
    }

    #[test]
    fn dijkstra_finds_the_shortest_paths() {
        let graph = Graph::weighted(
            &["S", "A", "B", "T"],
            &[
                (0, 1, 4.0),
                (0, 2, 1.0),
                (2, 1, 2.0),
                (1, 3, 1.0),
                (2, 3, 5.0),
            ],
            true,
        );
        let trace = GraphTrace::dijkstra(&graph, 0);
        let last = trace.steps().last().unwrap();
        assert_eq!(last.dist, [Some(0.0), Some(3.0), Some(1.0), Some(4.0)]);
        // S -> B -> A -> T
        let mut tree = last.tree.clone();
        tree.sort_unstable();
        assert_eq!(tree, [1, 2, 3]);
        assert!(trace.steps().iter().any(|s| s.note == "A: 1 + 2 = 3"));
        assert!(
            trace
                .steps()
                .iter()
                .any(|s| s.note == "T: 3 + 5 is not below 4" || s.note == "T: 1 + 5 = 6")
        );
    }

    fn pixels(graph: &Graph) -> Vec<Point> {
        graph
            .positions
            .iter()
            .map(|&(x, y)| Point::new(x * 170.0, y * 150.0))
            .collect()
    }

    #[test]
    fn weights_sit_on_the_middle_of_the_curve() {
        // A -> B -> C down a column, C -> A back up and B -> A against A -> B
        let graph = Graph::weighted(
            &["A", "B", "C"],
            &[(0, 1, 2.0), (1, 2, 3.0), (2, 0, 4.0), (1, 0, 5.0)],
            true,
        )
        .at(&[(0.0, 0.0), (0.0, 1.0), (0.0, 2.0)]);
        let points = pixels(&graph);
        let back = back_edges(&graph);
        for curved in [false, true] {
            for (e, &backward) in back.iter().enumerate() {
                let curve = edge_curve(&graph, &points, e, 34.0, curved, backward);
                assert_eq!(weight_place(&curve), curve.eval(0.5));
            }
        }
        // a straight edge: the middle of the segment between the nodes
        let open = Graph::weighted(&["A", "B"], &[(0, 1, 1.0)], true).at(&[(0.0, 0.0), (2.0, 0.0)]);
        let straight = edge_curve(&open, &pixels(&open), 0, 34.0, false, false);
        let middle = weight_place(&straight);
        assert!(
            (middle - Point::new(170.0, 0.0)).hypot() < 1e-9,
            "{middle:?}"
        );
        // curved, it leaves the segment
        let arc = edge_curve(&open, &pixels(&open), 0, 34.0, true, false);
        assert!(weight_place(&arc).y.abs() > 20.0);
    }

    #[test]
    fn backward_edges_curve_round_the_forward_ones() {
        let graph = Graph::new(&["A", "B", "C"], &[(0, 1), (1, 2), (2, 0), (1, 0)], true).at(&[
            (0.0, 0.0),
            (0.0, 1.0),
            (0.0, 2.0),
        ]);
        let points = pixels(&graph);
        // the edges that close a cycle: C -> A and B -> A
        let back = back_edges(&graph);
        assert_eq!(back, [false, false, true, true]);
        // C -> A passes B (on its way) at least a node and a margin to the side
        let up = edge_curve(&graph, &points, 2, 34.0, false, true);
        assert!((up.eval(0.5).x - points[1].x).abs() > 42.0);
        // B -> A and A -> B bend to opposite sides
        let (down, up) = (
            edge_curve(&graph, &points, 0, 34.0, false, back[0]).eval(0.5),
            edge_curve(&graph, &points, 3, 34.0, false, back[3]).eval(0.5),
        );
        assert!(down.x * up.x < 0.0, "{down:?} {up:?}");
        // a graph without cycles has none, wherever its nodes are, and undirected edges are
        // never backward
        let dag = Graph::new(&["A", "B", "C"], &[(2, 0), (0, 1), (2, 1)], true);
        assert_eq!(back_edges(&dag), [false; 3]);
        let plain = Graph::new(&["A", "B"], &[(0, 1), (1, 0)], false);
        assert_eq!(back_edges(&plain), [false; 2]);
    }

    #[test]
    fn shared_layouts_place_the_graph() {
        let edges = [(0, 1), (1, 2), (2, 3), (3, 4), (4, 0), (0, 2)];
        let base = Graph::new(&["A", "B", "C", "D", "E"], &edges, false);
        for layout in [
            GraphLayout::Circular,
            GraphLayout::KamadaKawai,
            GraphLayout::Planar,
            GraphLayout::Random { seed: 7 },
            GraphLayout::Spring {
                seed: 1,
                iterations: 50,
            },
            GraphLayout::Tree { root: 0 },
        ] {
            let a = base.clone().layout(&layout, 2.0);
            let b = base.clone().layout(&layout, 2.0);
            assert_eq!(a.positions, b.positions, "{layout:?}");
            let min = 0.4 * 2.0 / 5f64.sqrt();
            for &(x, y) in &a.positions {
                // inside the box, give or take the push apart of close nodes
                assert!(x.abs() <= 2.0 + min && y.abs() <= 2.0 + min, "{layout:?}");
            }
        }
        // the tree hangs down from its root: `y` grows downwards in the view
        let tree = base.layout(&GraphLayout::Tree { root: 0 }, 2.0);
        assert!((1..5).all(|v| tree.positions[v].1 > tree.positions[0].1));
    }

    #[test]
    fn directed_edges_go_one_way() {
        let graph = Graph::new(&["A", "B"], &[(1, 0)], true);
        assert_eq!(graph.neighbours(0).len(), 0);
        assert_eq!(GraphTrace::bfs(&graph, 0).order(), [0]);
        // the layout puts the source on top
        assert!(graph.positions[1].1 < graph.positions[0].1);
    }
}
