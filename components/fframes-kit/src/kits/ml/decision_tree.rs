use fframes::{Color, Svgr};
use kurbo::{BezPath, Point, Rect, RoundedRect, Shape as _};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    explain::AxesMap,
    geometry::Shape,
    kits::{
        draw::{mix, stroke},
        short,
    },
    motion::cubic_in_out,
    text::{TextAnchor, line, width},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

/// One node of a [`DecisionTree`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DecisionNode {
    /// Sends a sample whose `feature` is at most `threshold` to the node `left`, the others
    /// to `right` (indices into [`DecisionTree::nodes`]).
    Split {
        feature: usize,
        threshold: f64,
        left: usize,
        right: usize,
    },
    /// Predicts `class` (an index into [`DecisionTree::classes`]).
    Leaf { class: usize },
}

/// A trained decision tree, given as its `nodes` with the root first, the names of its
/// `features` and of its `classes`. A child index out of range or a node reached twice
/// ends that branch.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct DecisionTree<'a> {
    pub nodes: Vec<DecisionNode>,
    pub features: Vec<&'a str>,
    pub classes: Vec<&'a str>,
}

/// A node reached from the root: its index, its depth, and its parent with the side it hangs
/// on (`true` for the left, the samples at most the threshold).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Reached {
    pub node: usize,
    pub depth: usize,
    pub parent: Option<(usize, bool)>,
}

impl DecisionTree<'_> {
    /// The children of `node` that exist, left first.
    fn children(&self, node: usize) -> Vec<(usize, bool)> {
        match self.nodes.get(node) {
            Some(&DecisionNode::Split { left, right, .. }) => [(left, true), (right, false)]
                .into_iter()
                .filter(|&(c, _)| c < self.nodes.len())
                .collect(),
            _ => Vec::new(),
        }
    }

    /// The class the tree predicts for the sample `x` (its features in order), `None` when a
    /// branch ends without a leaf.
    pub fn predict(&self, x: &[f64]) -> Option<usize> {
        let mut node = 0;
        for _ in 0..self.nodes.len() {
            match *self.nodes.get(node)? {
                DecisionNode::Leaf { class } => return Some(class),
                DecisionNode::Split {
                    feature,
                    threshold,
                    left,
                    right,
                } => {
                    node = if *x.get(feature)? <= threshold {
                        left
                    } else {
                        right
                    }
                }
            }
        }
        None
    }

    /// The nodes reached from the root, level by level and left to right.
    pub fn breadth_first(&self) -> Vec<Reached> {
        let mut seen = vec![false; self.nodes.len()];
        let mut order = Vec::new();
        if self.nodes.is_empty() {
            return order;
        }
        seen[0] = true;
        order.push(Reached {
            node: 0,
            depth: 0,
            parent: None,
        });
        let mut next = 0;
        while let Some(&Reached { node, depth, .. }) = order.get(next) {
            next += 1;
            for (child, left) in self.children(node) {
                if !std::mem::replace(&mut seen[child], true) {
                    order.push(Reached {
                        node: child,
                        depth: depth + 1,
                        parent: Some((node, left)),
                    });
                }
            }
        }
        order
    }

    /// The horizontal slot of every reached node: the leaves take the slots 0, 1, 2.. from
    /// left to right and a split sits in the middle of its children.
    pub fn slots(&self) -> Vec<Option<f64>> {
        let reached = self.breadth_first();
        let mut kids: Vec<Vec<usize>> = vec![Vec::new(); self.nodes.len()];
        for r in &reached {
            if let Some((parent, _)) = r.parent {
                kids[parent].push(r.node);
            }
        }
        let mut slots = vec![None; self.nodes.len()];
        let mut next = 0.0;
        // in order, without recursion: a node is placed once its children are
        let mut stack = Vec::new();
        if !reached.is_empty() {
            stack.push((0, false));
        }
        while let Some((node, expanded)) = stack.pop() {
            if kids[node].is_empty() {
                slots[node] = Some(next);
                next += 1.0;
            } else if expanded {
                let placed: Vec<f64> = kids[node].iter().filter_map(|&k| slots[k]).collect();
                slots[node] = Some(placed.iter().sum::<f64>() / placed.len() as f64);
            } else {
                stack.push((node, true));
                stack.extend(kids[node].iter().rev().map(|&k| (k, false)));
            }
        }
        slots
    }

    /// The regions the tree cuts the box `bounds` (data units) into, for a tree that splits
    /// on `x_feature` and `y_feature` only: each leaf with its rectangle, and each split with
    /// the segment it cuts along. A split on another feature gives both its sides its whole
    /// region and draws no segment.
    pub fn regions(&self, x_feature: usize, y_feature: usize, bounds: Rect) -> Partition {
        let mut rects = vec![Rect::ZERO; self.nodes.len()];
        let (mut leaves, mut cuts) = (Vec::new(), Vec::new());
        for r in self.breadth_first() {
            let region = match r.parent {
                None => bounds,
                Some((parent, left)) => {
                    let p = rects[parent];
                    match self.nodes[parent] {
                        DecisionNode::Split {
                            feature, threshold, ..
                        } if feature == x_feature => {
                            let t = threshold.clamp(p.x0, p.x1);
                            if left {
                                Rect::new(p.x0, p.y0, t, p.y1)
                            } else {
                                Rect::new(t, p.y0, p.x1, p.y1)
                            }
                        }
                        DecisionNode::Split {
                            feature, threshold, ..
                        } if feature == y_feature => {
                            let t = threshold.clamp(p.y0, p.y1);
                            if left {
                                Rect::new(p.x0, p.y0, p.x1, t)
                            } else {
                                Rect::new(p.x0, t, p.x1, p.y1)
                            }
                        }
                        _ => p,
                    }
                }
            };
            rects[r.node] = region;
            match self.nodes[r.node] {
                DecisionNode::Leaf { .. } => leaves.push((r.node, region)),
                DecisionNode::Split {
                    feature, threshold, ..
                } => {
                    let (b, t) = (region, threshold);
                    if feature == x_feature && (b.x0..=b.x1).contains(&t) {
                        cuts.push((r.node, Point::new(t, b.y0), Point::new(t, b.y1)));
                    } else if feature == y_feature && (b.y0..=b.y1).contains(&t) {
                        cuts.push((r.node, Point::new(b.x0, t), Point::new(b.x1, t)));
                    }
                }
            }
        }
        Partition { leaves, cuts }
    }
}

/// The regions a [`DecisionTree`] cuts a box into, in the units of the box.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Partition {
    /// Each leaf with its region.
    pub leaves: Vec<(usize, Rect)>,
    /// Each split on a drawn feature with the ends of its cut.
    pub cuts: Vec<(usize, Point, Point)>,
}

/// The order a decision tree grows in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RevealOrder {
    /// A whole level at a time.
    #[default]
    LevelByLevel,
    /// One node at a time, level by level and left to right.
    BreadthFirst,
}

/// A tree growing from `start`, a level or a node every `step` seconds.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TreeReveal {
    pub order: RevealOrder,
    pub start: f32,
    pub step: f32,
}

/// How long a node takes to appear, in seconds (shorter when the steps are).
const APPEAR: f32 = 0.3;

impl TreeReveal {
    /// When each node of `tree` starts to appear, `None` for the nodes not reached.
    pub fn times(&self, tree: &DecisionTree<'_>) -> Vec<Option<f32>> {
        let mut times = vec![None; tree.nodes.len()];
        for (rank, r) in tree.breadth_first().into_iter().enumerate() {
            let k = match self.order {
                RevealOrder::LevelByLevel => r.depth,
                RevealOrder::BreadthFirst => rank,
            };
            times[r.node] = Some(self.start + k as f32 * self.step);
        }
        times
    }
}

/// How far each node of `tree` has appeared at `now`, from 0 to 1.
fn shown(tree: &DecisionTree<'_>, reveal: Option<&TreeReveal>, now: f32) -> Vec<f32> {
    let Some(reveal) = reveal else {
        return vec![1.0; tree.nodes.len()];
    };
    let ramp = APPEAR.min(reveal.step.max(1e-3));
    reveal
        .times(tree)
        .into_iter()
        .map(|t| t.map_or(0.0, |t| cubic_in_out(((now - t) / ramp).clamp(0.0, 1.0))))
        .collect()
}

fn class_color(colors: &[Color], class: usize, fallback: Color) -> Color {
    if colors.is_empty() {
        fallback
    } else {
        colors[class % colors.len()]
    }
}

fn filled(path: BezPath, color: Color, opacity: f32) -> Shape {
    Shape {
        path,
        fill: Some(color),
        stroke: None,
        stroke_width: 0.0,
        dash: None,
        opacity,
    }
}

fn class_colors(t: &Theme) -> Vec<Color> {
    vec![
        t.colors.accent,
        t.colors.highlight,
        Color::hex("#69db7c"),
        Color::hex("#ff6b6b"),
    ]
}

/// A decision tree as `ManimML` draws it (`DecisionTreeDiagram`): each split a box with its
/// test (`feature ≤ threshold`), each leaf a box with its class in that class's colour from
/// `class_colors`, the root at the top, `level` pixels between levels and `gap` pixels
/// between neighbouring leaves. The edge to the left child, the samples that pass the test,
/// is labelled yes. The boxes are as wide as the longest text and at least `node_width`. With
/// `reveal` the tree grows a level or a node at a time.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct DecisionTreeDiagram<'a> {
    pub tree: DecisionTree<'a>,
    pub center: (f64, f64),
    pub node_width: f32,
    pub node_height: f32,
    pub gap: f32,
    pub level: f32,
    pub reveal: Option<TreeReveal>,
    pub class_colors: Vec<Color>,
    pub node_color: Color,
    pub node_fill: Color,
    pub edge_color: Color,
    pub text_style: TextStyle,
    pub stroke_width: f32,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for DecisionTreeDiagram<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            tree: DecisionTree::default(),
            center: (0.0, 0.0),
            node_width: 190.0,
            node_height: 54.0,
            gap: 24.0,
            level: 110.0,
            reveal: None,
            class_colors: class_colors(t),
            node_color: t.colors.text,
            node_fill: t.colors.surface,
            edge_color: t.colors.muted,
            text_style: t.typography.caption,
            stroke_width: 2.5,
            enter: None,
            exit: None,
        }
    }
}

impl Default for DecisionTreeDiagram<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl DecisionTreeDiagram<'_> {
    /// The box of every node with the middle of the tree at `middle`, `node_width` pixels
    /// wide; `None` for the nodes not reached from the root.
    pub fn layout_at(&self, middle: Point) -> Vec<Option<Rect>> {
        self.layout_with(middle, f64::from(self.node_width))
    }

    /// The box of every node, in pixels, all as wide as the longest text needs and at least
    /// `node_width`.
    pub fn layout(&self, ui: &Ui<'_, '_, '_>) -> Vec<Option<Rect>> {
        let padding = f64::from(self.node_height) * 0.6;
        let widest = self
            .tree
            .nodes
            .iter()
            .filter_map(|n| width(ui, self.text_style, &self.text(n)))
            .fold(0.0_f64, |a, w| a.max(f64::from(w) + padding));
        let middle = ui.space.to_px(self.center.0, self.center.1);
        self.layout_with(middle, widest.max(f64::from(self.node_width)))
    }

    fn layout_with(&self, middle: Point, w: f64) -> Vec<Option<Rect>> {
        let slots = self.tree.slots();
        let reached = self.tree.breadth_first();
        let width = slots.iter().flatten().fold(0.0_f64, |a, &s| a.max(s));
        let depth = reached.iter().map(|r| r.depth).max().unwrap_or(0) as f64;
        let h = f64::from(self.node_height);
        let pitch = w + f64::from(self.gap);
        let mut boxes = vec![None; self.tree.nodes.len()];
        for r in reached {
            if let Some(slot) = slots[r.node] {
                let c = Point::new(
                    middle.x + (slot - width / 2.0) * pitch,
                    middle.y + (r.depth as f64 - depth / 2.0) * f64::from(self.level),
                );
                boxes[r.node] = Some(Rect::from_center_size(c, (w, h)));
            }
        }
        boxes
    }

    /// The text of `node`.
    fn text(&self, node: &DecisionNode) -> String {
        match *node {
            DecisionNode::Split {
                feature, threshold, ..
            } => {
                let name = self
                    .tree
                    .features
                    .get(feature)
                    .map_or_else(|| format!("x{feature}"), |f| (*f).to_owned());
                format!("{name} ≤ {}", short(threshold, 2))
            }
            DecisionNode::Leaf { class } => self
                .tree
                .classes
                .get(class)
                .map_or_else(|| format!("class {class}"), |c| (*c).to_owned()),
        }
    }
}

impl<'a> Component<'a> for DecisionTreeDiagram<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let boxes = self.layout(ui);
        let shown = shown(&self.tree, self.reveal.as_ref(), ui.seconds());
        let style = self.text_style;
        let small = TextStyle {
            size: (style.size * 4 / 5).max(1),
            ..style
        };
        let r = f64::from(self.node_height) * 0.2;
        let mut parts = Vec::new();
        // edges first, under the boxes
        for reached in self.tree.breadth_first() {
            let (Some((parent, left)), Some(to)) = (reached.parent, boxes[reached.node]) else {
                continue;
            };
            let (Some(from), o) = (boxes[parent], shown[reached.node]) else {
                continue;
            };
            if o <= 0.0 {
                continue;
            }
            let (a, b) = (
                Point::new(from.center().x, from.y1),
                Point::new(to.center().x, to.y0),
            );
            let mut edge = BezPath::new();
            edge.move_to(a);
            edge.line_to(b);
            parts.push(stroke(edge, self.edge_color, self.stroke_width, o).draw());
            let mid = a.midpoint(b);
            let side = if left { -1.0 } else { 1.0 };
            parts.push(line(
                (mid.x + side * small.size as f64 * 0.9) as f32,
                (mid.y + small.size as f64 * 0.35) as f32,
                small,
                self.edge_color,
                if left {
                    TextAnchor::End
                } else {
                    TextAnchor::Start
                },
                0.0,
                o,
                if left { "yes" } else { "no" }.into(),
            ));
        }
        for (i, node) in self.tree.nodes.iter().enumerate() {
            let (Some(b), o) = (boxes[i], shown[i]) else {
                continue;
            };
            if o <= 0.0 {
                continue;
            }
            let frame = RoundedRect::from_rect(b, r).to_path(0.1);
            let (fill, ring) = match *node {
                DecisionNode::Leaf { class } => {
                    let c = class_color(&self.class_colors, class, self.node_color);
                    (mix(self.node_fill, c, 0.35), c)
                }
                DecisionNode::Split { .. } => (self.node_fill, self.node_color),
            };
            parts.push(filled(frame.clone(), fill, o).draw());
            parts.push(stroke(frame, ring, self.stroke_width, o).draw());
            let c = b.center();
            parts.push(line(
                c.x as f32,
                (c.y + style.size as f64 * 0.35) as f32,
                style,
                self.node_color,
                TextAnchor::Middle,
                0.0,
                o,
                self.text(node).into(),
            ));
        }
        fade(ui, parts.into(), self.enter.as_ref(), self.exit.as_ref())
    }
}

/// The partition of a 2D feature space by a decision tree (`ManimML`'s
/// `DecisionTreeSurface`): every leaf's region of the box of `axes` filled in its class's
/// colour at `opacity`, and the cuts of the splits drawn in `line_color`. `x_feature` and
/// `y_feature` are the features along the axes; the tree should split on these two only. With
/// the `reveal` of a [`DecisionTreeDiagram`] a cut appears with its split and a region with its
/// leaf. Draw the [`Axes`](crate::explain::Axes) after it to keep their lines on top.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct DecisionSurface<'a> {
    pub tree: DecisionTree<'a>,
    pub axes: AxesMap,
    pub x_feature: usize,
    pub y_feature: usize,
    pub reveal: Option<TreeReveal>,
    pub class_colors: Vec<Color>,
    pub opacity: f32,
    pub line_color: Color,
    pub stroke_width: f32,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for DecisionSurface<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            tree: DecisionTree::default(),
            axes: AxesMap::default(),
            x_feature: 0,
            y_feature: 1,
            reveal: None,
            class_colors: class_colors(t),
            opacity: 0.35,
            line_color: t.colors.text,
            stroke_width: 2.5,
            enter: None,
            exit: None,
        }
    }
}

impl Default for DecisionSurface<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for DecisionSurface<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let shown = shown(&self.tree, self.reveal.as_ref(), ui.seconds());
        let a = &self.axes;
        let bounds = Rect::new(a.x.min, a.y.min, a.x.max, a.y.max);
        let Partition { leaves, cuts } = self.tree.regions(self.x_feature, self.y_feature, bounds);
        let px = |p: Point| a.c2p(p.x, p.y);
        let mut parts = Vec::new();
        for (node, rect) in leaves {
            let DecisionNode::Leaf { class } = self.tree.nodes[node] else {
                continue;
            };
            let o = shown[node];
            if o <= 0.0 || rect.area() <= 0.0 {
                continue;
            }
            let rect = Rect::from_points(px(rect.origin()), px(Point::new(rect.x1, rect.y1)));
            let color = class_color(&self.class_colors, class, self.line_color);
            parts.push(filled(rect.to_path(0.1), color, self.opacity * o).draw());
        }
        for (node, from, to) in cuts {
            let o = shown[node];
            if o <= 0.0 {
                continue;
            }
            let mut cut = BezPath::new();
            cut.move_to(px(from));
            cut.line_to(px(to));
            parts.push(stroke(cut, self.line_color, self.stroke_width, o).draw());
        }
        fade(ui, parts.into(), self.enter.as_ref(), self.exit.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The iris tree: petal length, then petal width.
    fn iris() -> DecisionTree<'static> {
        DecisionTree {
            nodes: vec![
                DecisionNode::Split {
                    feature: 0,
                    threshold: 2.45,
                    left: 1,
                    right: 2,
                },
                DecisionNode::Leaf { class: 0 },
                DecisionNode::Split {
                    feature: 1,
                    threshold: 1.75,
                    left: 3,
                    right: 4,
                },
                DecisionNode::Leaf { class: 1 },
                DecisionNode::Leaf { class: 2 },
            ],
            features: vec!["petal length", "petal width"],
            classes: vec!["setosa", "versicolor", "virginica"],
        }
    }

    #[test]
    fn the_tree_predicts_by_its_tests() {
        let tree = iris();
        assert_eq!(tree.predict(&[1.4, 0.2]), Some(0));
        assert_eq!(tree.predict(&[4.5, 1.5]), Some(1));
        assert_eq!(tree.predict(&[5.5, 2.1]), Some(2));
        // the threshold itself goes left
        assert_eq!(tree.predict(&[2.45, 9.0]), Some(0));
        // a missing feature
        assert_eq!(tree.predict(&[]), None);
    }

    #[test]
    fn a_broken_tree_ends_its_branches() {
        let tree = DecisionTree {
            nodes: vec![DecisionNode::Split {
                feature: 0,
                threshold: 1.0,
                left: 0,
                right: 9,
            }],
            ..Default::default()
        };
        assert_eq!(tree.predict(&[0.0]), None);
        assert_eq!(tree.predict(&[2.0]), None);
        assert_eq!(tree.breadth_first().len(), 1);
        assert_eq!(tree.slots(), vec![Some(0.0)]);
        assert_eq!(DecisionTree::default().breadth_first(), Vec::new());
    }

    #[test]
    fn leaves_take_slots_and_splits_sit_between_their_children() {
        let tree = iris();
        let order: Vec<(usize, usize)> = tree
            .breadth_first()
            .iter()
            .map(|r| (r.node, r.depth))
            .collect();
        assert_eq!(order, vec![(0, 0), (1, 1), (2, 1), (3, 2), (4, 2)]);
        let slots = tree.slots();
        assert_eq!(
            slots,
            vec![Some(0.75), Some(0.0), Some(1.5), Some(1.0), Some(2.0)]
        );
        let diagram = DecisionTreeDiagram {
            tree,
            ..Default::default()
        };
        let boxes = diagram.layout_at(Point::new(960.0, 540.0));
        let b: Vec<Rect> = boxes.into_iter().map(Option::unwrap).collect();
        // the leaves on one level do not overlap, the root is above its children
        assert!(b[3].x1 < b[4].x0);
        assert!(b[0].y1 < b[1].y0);
        assert!((b[2].center().x - b[3].center().x.midpoint(b[4].center().x)).abs() < 1e-9);
        assert!((b[0].center().y + b[3].center().y - 1080.0).abs() < 1e-9);
    }

    #[test]
    fn the_regions_tile_the_box() {
        let tree = iris();
        let bounds = Rect::new(0.0, 0.0, 7.0, 3.0);
        let Partition { leaves, cuts } = tree.regions(0, 1, bounds);
        assert_eq!(leaves.len(), 3);
        assert_eq!(cuts.len(), 2);
        let area: f64 = leaves.iter().map(|(_, r)| r.area()).sum();
        assert!((area - bounds.area()).abs() < 1e-9);
        for (i, (_, a)) in leaves.iter().enumerate() {
            for (_, b) in &leaves[i + 1..] {
                assert!(a.intersect(*b).area() < 1e-9);
            }
        }
        assert_eq!(leaves[0], (1, Rect::new(0.0, 0.0, 2.45, 3.0)));
        assert_eq!(cuts[1], (2, Point::new(2.45, 1.75), Point::new(7.0, 1.75)));
        // every sample lies in the region of the leaf it is sent to
        for x in [[1.0, 2.0], [3.0, 1.0], [6.0, 2.5]] {
            let class = tree.predict(&x).unwrap();
            let p = Point::new(x[0], x[1]);
            assert!(
                leaves
                    .iter()
                    .any(|&(n, r)| r.contains(p) && tree.nodes[n] == DecisionNode::Leaf { class })
            );
        }
    }

    #[test]
    fn the_tree_grows_level_by_level_or_node_by_node() {
        let tree = iris();
        let levels = TreeReveal {
            order: RevealOrder::LevelByLevel,
            start: 1.0,
            step: 0.5,
        };
        assert_eq!(
            levels.times(&tree),
            vec![Some(1.0), Some(1.5), Some(1.5), Some(2.0), Some(2.0)]
        );
        let nodes = TreeReveal {
            order: RevealOrder::BreadthFirst,
            ..levels
        };
        assert_eq!(
            nodes.times(&tree),
            vec![Some(1.0), Some(1.5), Some(2.0), Some(2.5), Some(3.0)]
        );
        let at = |t| shown(&tree, Some(&nodes), t);
        assert_eq!(at(0.9), vec![0.0; 5]);
        assert!(at(1.3)[0] > 0.99);
        assert_eq!(at(9.0), vec![1.0; 5]);
        assert_eq!(shown(&tree, None, 0.0), vec![1.0; 5]);
    }
}
