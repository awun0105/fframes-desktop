//! Placing things relative to each other: [`Anchor`], the nine points of a box, also used as
//! a direction (Manim's `UP`, `UR`, `DL`, ...), and boxes in rows, columns and grids. Sizes
//! and boxes are pixels; every arrangement is centered on the origin, so add the point it
//! goes to.

use kurbo::{Point, Rect, Vec2};

/// Where `(x, y)` is on a box.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Anchor {
    #[default]
    TopLeft,
    Top,
    TopRight,
    Left,
    Center,
    Right,
    BottomLeft,
    Bottom,
    BottomRight,
}

impl Anchor {
    /// The anchor point of a `width` x `height` box whose top left is at the origin.
    pub fn point(self, width: f64, height: f64) -> (f64, f64) {
        let (dx, dy) = self.direction();
        (width * (dx + 1.0) / 2.0, height * (dy + 1.0) / 2.0)
    }

    /// The direction from the center to this point of a box on screen (`y` down), each
    /// component -1, 0 or 1: `TopRight` is Manim's `UR`, `(1, -1)`.
    pub fn direction(self) -> (f64, f64) {
        use Anchor::*;
        let x = match self {
            TopLeft | Left | BottomLeft => -1.0,
            Top | Center | Bottom => 0.0,
            TopRight | Right | BottomRight => 1.0,
        };
        let y = match self {
            TopLeft | Top | TopRight => -1.0,
            Left | Center | Right => 0.0,
            BottomLeft | Bottom | BottomRight => 1.0,
        };
        (x, y)
    }

    /// The point on the opposite side: the side of a label that faces what it labels.
    pub fn opposite(self) -> Self {
        use Anchor::*;
        match self {
            TopLeft => BottomRight,
            Top => Bottom,
            TopRight => BottomLeft,
            Left => Right,
            Center => Center,
            Right => Left,
            BottomLeft => TopRight,
            Bottom => Top,
            BottomRight => TopLeft,
        }
    }

    /// The top left of a `width` x `height` box placed next to `(x, y)` in this direction,
    /// `buff` pixels away (Manim's `next_to(point, direction, buff)`).
    pub fn next_to(self, x: f64, y: f64, width: f64, height: f64, buff: f64) -> (f64, f64) {
        let (dx, dy) = self.direction();
        let (ax, ay) = self.opposite().point(width, height);
        (x + dx * buff - ax, y + dy * buff - ay)
    }

    /// The anchor whose direction is nearest to `(dx, dy)` (screen, `y` down): the side a
    /// label goes on at the end of a line pointing that way. `Center` for a zero vector.
    pub fn toward(dx: f64, dy: f64) -> Self {
        use Anchor::*;
        if dx == 0.0 && dy == 0.0 || dx.is_nan() || dy.is_nan() {
            return Center;
        }
        let octant = (dy.atan2(dx) / std::f64::consts::FRAC_PI_4).round() as i32;
        match octant.rem_euclid(8) {
            0 => Right,
            1 => BottomRight,
            2 => Bottom,
            3 => BottomLeft,
            4 => Left,
            5 => TopLeft,
            6 => Top,
            _ => TopRight,
        }
    }

    /// The top left of a `width` x `height` box at this point of `area`, `margin` pixels in
    /// from its edges: `TopRight` is the top right corner of the safe area.
    pub fn within(self, area: Rect, width: f64, height: f64, margin: f64) -> (f64, f64) {
        let inner = area.inset(-margin);
        let (px, py) = self.point(inner.width(), inner.height());
        let (ax, ay) = self.point(width, height);
        (inner.x0 + px - ax, inner.y0 + py - ay)
    }
}

/// `area` without `margin` pixels on every side, the area titles and captions stay in.
pub fn safe_area(area: Rect, margin: f64) -> Rect {
    area.inset(-margin)
}

/// Boxes of `sizes` placed one after another in `direction`, `gap` apart, centers aligned
/// across it (Manim's `arrange(direction, buff)`). The whole run is centered on the origin.
pub fn arrange(sizes: &[(f64, f64)], direction: Anchor, gap: f64) -> Vec<Rect> {
    let (dx, dy) = direction.direction();
    let mut centers: Vec<Point> = Vec::with_capacity(sizes.len());
    let mut prev: Option<(Point, (f64, f64))> = None;
    for &(w, h) in sizes {
        // the next center is half of both boxes plus the gap further along
        let center = prev.map_or(Point::ZERO, |(p, (pw, ph))| {
            Point::new(
                p.x + dx * (f64::midpoint(pw, w) + gap),
                p.y + dy * (f64::midpoint(ph, h) + gap),
            )
        });
        centers.push(center);
        prev = Some((center, (w, h)));
    }
    let boxes: Vec<Rect> = centers
        .iter()
        .zip(sizes)
        .map(|(&c, &(w, h))| Rect::from_center_size(c, (w, h)))
        .collect();
    centered(boxes)
}

/// Boxes left to right, `gap` apart, vertically centered, the row centered on the origin.
pub fn row(sizes: &[(f64, f64)], gap: f64) -> Vec<Rect> {
    arrange(sizes, Anchor::Right, gap)
}

/// Boxes top to bottom, `gap` apart, horizontally centered, the column centered on the
/// origin.
pub fn column(sizes: &[(f64, f64)], gap: f64) -> Vec<Rect> {
    arrange(sizes, Anchor::Bottom, gap)
}

/// Boxes in rows of `columns` cells, row by row (Manim's `arrange_in_grid`). A column is as
/// wide as its widest box and a row as tall as its tallest; each box is centered in its
/// cell and the grid on the origin. `gap` is `(between columns, between rows)`.
pub fn grid(sizes: &[(f64, f64)], columns: usize, gap: (f64, f64)) -> Vec<Rect> {
    let columns = columns.max(1);
    let rows = sizes.len().div_ceil(columns);
    let mut widths = vec![0.0_f64; columns];
    let mut heights = vec![0.0_f64; rows];
    for (i, &(w, h)) in sizes.iter().enumerate() {
        widths[i % columns] = widths[i % columns].max(w);
        heights[i / columns] = heights[i / columns].max(h);
    }
    let starts = |lengths: &[f64], gap: f64| -> Vec<f64> {
        lengths
            .iter()
            .scan(0.0, |at, &l| {
                let start = *at;
                *at += l + gap;
                Some(start)
            })
            .collect()
    };
    let (xs, ys) = (starts(&widths, gap.0), starts(&heights, gap.1));
    let boxes = sizes
        .iter()
        .enumerate()
        .map(|(i, &size)| {
            let (c, r) = (i % columns, i / columns);
            let cell = Point::new(xs[c] + widths[c] / 2.0, ys[r] + heights[r] / 2.0);
            Rect::from_center_size(cell, size)
        })
        .collect();
    centered(boxes)
}

/// `boxes` moved together so the box around all of them is centered on the origin.
fn centered(boxes: Vec<Rect>) -> Vec<Rect> {
    let Some(all) = boxes.iter().copied().reduce(|a, b| a.union(b)) else {
        return boxes;
    };
    let shift = Point::ZERO - all.center();
    boxes.into_iter().map(|b| b + shift).collect()
}

/// How far to move each box down so no two overlap, keeping `gap` between them: the boxes
/// are placed in order and each one goes below every earlier box it would overlap. Labels
/// pass their boxes and add the offsets to their `y`.
pub fn separate(boxes: &[Rect], gap: f64) -> Vec<f64> {
    let mut placed: Vec<Rect> = Vec::with_capacity(boxes.len());
    boxes
        .iter()
        .map(|&b| {
            let mut moved = b;
            // each pass moves below one overlapping box, so `placed.len()` passes are enough
            for _ in 0..=placed.len() {
                let hit = placed
                    .iter()
                    .filter(|p| overlaps(**p, moved, gap))
                    .map(|p| p.y1)
                    .reduce(f64::max);
                match hit {
                    Some(bottom) => moved = moved + Vec2::new(0.0, bottom + gap - moved.y0),
                    None => break,
                }
            }
            placed.push(moved);
            moved.y0 - b.y0
        })
        .collect()
}

fn overlaps(a: Rect, b: Rect, gap: f64) -> bool {
    a.x0 < b.x1 && b.x0 < a.x1 && a.y0 < b.y1 + gap && b.y0 < a.y1 + gap
}

/// Positions of a tree's nodes, the tidy way: `x` in slots (siblings are at least one slot
/// apart, and so are neighbouring subtrees at every depth), `y` the depth (the root is 0).
/// `children[i]` lists the children of node `i` from left to right. Each parent sits midway
/// between its first and last child, and the leftmost node is at `x = 0`. Nodes not reached
/// from `root` stay at `(0, 0)`; a node reached twice is placed the first time only.
pub fn tidy_tree(children: &[Vec<usize>], root: usize) -> Vec<(f64, f64)> {
    /// Places the subtree of `node`: sets its children's offsets and returns its extent at
    /// each depth, relative to `node`.
    fn place(
        node: usize,
        children: &[Vec<usize>],
        offset: &mut [f64],
        seen: &mut [bool],
        kept: &mut [Vec<usize>],
    ) -> Vec<(f64, f64)> {
        // claim the children before descending, so no deeper node takes them
        let mut kids = Vec::new();
        for &c in &children[node] {
            if c < children.len() && !seen[c] {
                seen[c] = true;
                kids.push(c);
            }
        }
        let mut merged: Vec<(f64, f64)> = Vec::new();
        let mut xs = Vec::with_capacity(kids.len());
        for &c in &kids {
            let contour = place(c, children, offset, seen, kept);
            // the child moves right until it clears every depth both subtrees reach
            let shift = if merged.is_empty() {
                0.0
            } else {
                merged
                    .iter()
                    .zip(&contour)
                    .map(|(m, k)| m.1 - k.0 + 1.0)
                    .fold(f64::NEG_INFINITY, f64::max)
            };
            for (d, &(l, r)) in contour.iter().enumerate() {
                match merged.get_mut(d) {
                    Some(m) => m.1 = r + shift,
                    None => merged.push((l + shift, r + shift)),
                }
            }
            xs.push(shift);
        }
        let middle = match (xs.first(), xs.last()) {
            (Some(a), Some(b)) => f64::midpoint(*a, *b),
            _ => 0.0,
        };
        for (&c, x) in kids.iter().zip(&xs) {
            offset[c] = x - middle;
        }
        kept[node] = kids;
        let mut contour = vec![(0.0, 0.0)];
        contour.extend(merged.into_iter().map(|(l, r)| (l - middle, r - middle)));
        contour
    }
    let n = children.len();
    let mut positions = vec![(0.0, 0.0); n];
    if root >= n {
        return positions;
    }
    // the offset of every node from its parent, and each subtree's extent per depth
    let mut offset = vec![0.0_f64; n];
    let mut seen = vec![false; n];
    // the children each node keeps: a node listed twice belongs to its first parent
    let mut kept: Vec<Vec<usize>> = vec![Vec::new(); n];
    seen[root] = true;
    place(root, children, &mut offset, &mut seen, &mut kept);
    // absolute positions from the root down
    let mut stack = vec![(root, 0.0, 0.0)];
    let mut placed = vec![false; n];
    while let Some((node, x, y)) = stack.pop() {
        placed[node] = true;
        positions[node] = (x, y);
        for &c in &kept[node] {
            stack.push((c, x + offset[c], y + 1.0));
        }
    }
    let left = (0..n)
        .filter(|&i| placed[i])
        .map(|i| positions[i].0)
        .fold(f64::INFINITY, f64::min);
    for (i, p) in positions.iter_mut().enumerate() {
        if placed[i] {
            p.0 -= left;
        }
    }
    positions
}

/// Positions of a directed graph's `count` nodes in layers (Sugiyama's method, through
/// `rust-sugiyama`): `y` is the layer (sources at 0, edges point down), `x` the slot in the
/// layer (neighbours at least one apart), with crossings reduced. Cycles are fine. The
/// connected parts sit side by side, one slot apart, in the order the layout returns them,
/// and nodes without edges follow in layer 0. Self-loops and edges to missing nodes are
/// left out. The result is the same on every run and machine.
#[cfg(any(feature = "cs", feature = "biology"))]
pub fn layered(count: usize, edges: &[(usize, usize)]) -> Vec<(f64, f64)> {
    use rust_sugiyama::configure::Config;

    let mut positions = vec![(0.0, 0.0); count];
    let edges: Vec<(u32, u32)> = edges
        .iter()
        .filter(|(a, b)| a != b && *a < count && *b < count)
        .filter_map(|&(a, b)| Some((u32::try_from(a).ok()?, u32::try_from(b).ok()?)))
        .collect();
    // the defaults, never the environment, so every machine lays out the same
    let config = Config::default();
    let spacing = config.vertex_spacing;
    let mut placed = vec![false; count];
    let mut next = 0.0_f64;
    if !edges.is_empty() {
        for (part, _, _) in rust_sugiyama::from_edges(&edges, &config) {
            let left = part
                .iter()
                .map(|(_, (x, _))| *x)
                .fold(f64::INFINITY, f64::min);
            let mut right = next;
            for (id, (x, y)) in part {
                if let Some(p) = positions.get_mut(id) {
                    let x = next + (x - left) / spacing;
                    *p = (x, y / spacing);
                    placed[id] = true;
                    right = right.max(x);
                }
            }
            next = right + 1.0;
        }
    }
    for (p, _) in positions
        .iter_mut()
        .zip(&placed)
        .filter(|(_, placed)| !**placed)
    {
        *p = (next, 0.0);
        next += 1.0;
    }
    positions
}
#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Rect, b: (f64, f64, f64, f64)) -> bool {
        [(a.x0, b.0), (a.y0, b.1), (a.x1, b.2), (a.y1, b.3)]
            .iter()
            .all(|(p, q)| (p - q).abs() < 1e-9)
    }

    #[test]
    fn rows_columns_and_grids_are_centered() {
        let r = row(&[(10.0, 4.0), (20.0, 8.0)], 2.0);
        // 10 + 2 + 20 = 32 wide, centered: from -16 to 16
        assert!(close(r[0], (-16.0, -2.0, -6.0, 2.0)), "{r:?}");
        assert!(close(r[1], (-4.0, -4.0, 16.0, 4.0)), "{r:?}");
        let c = column(&[(10.0, 4.0), (6.0, 4.0)], 2.0);
        assert!(close(c[0], (-5.0, -5.0, 5.0, -1.0)), "{c:?}");
        assert!(close(c[1], (-3.0, 1.0, 3.0, 5.0)), "{c:?}");
        // a 2x2 grid of one tall and three small boxes
        let g = grid(
            &[(10.0, 10.0), (4.0, 4.0), (4.0, 4.0), (4.0, 4.0)],
            2,
            (2.0, 2.0),
        );
        // columns 10 and 4 wide, rows 10 and 4 tall: 16 x 16, centered
        assert!(close(g[0], (-8.0, -8.0, 2.0, 2.0)), "{g:?}");
        assert!(close(g[1], (4.0, -5.0, 8.0, -1.0)), "{g:?}");
        assert!(close(g[2], (-5.0, 4.0, -1.0, 8.0)), "{g:?}");
        assert!(close(g[3], (4.0, 4.0, 8.0, 8.0)), "{g:?}");
        assert!(row(&[], 2.0).is_empty() && grid(&[], 3, (1.0, 1.0)).is_empty());
    }

    #[test]
    fn diagonal_arrangement_steps_both_ways() {
        let d = arrange(&[(2.0, 2.0), (2.0, 2.0)], Anchor::BottomRight, 0.0);
        assert!((d[1].center() - d[0].center() - Vec2::new(2.0, 2.0)).hypot() < 1e-9);
    }

    #[test]
    fn toward_picks_the_nearest_of_eight_directions() {
        assert_eq!(Anchor::toward(1.0, 0.0), Anchor::Right);
        assert_eq!(Anchor::toward(1.0, -1.0), Anchor::TopRight);
        assert_eq!(Anchor::toward(-1.0, 0.1), Anchor::Left);
        assert_eq!(Anchor::toward(0.2, 1.0), Anchor::Bottom);
        assert_eq!(Anchor::toward(0.0, 0.0), Anchor::Center);
    }

    #[test]
    fn safe_area_corners() {
        let screen = Rect::new(0.0, 0.0, 1920.0, 1080.0);
        assert_eq!(
            safe_area(screen, 96.0),
            Rect::new(96.0, 96.0, 1824.0, 984.0)
        );
        assert_eq!(
            Anchor::TopRight.within(screen, 200.0, 50.0, 96.0),
            (1624.0, 96.0)
        );
        assert_eq!(
            Anchor::Center.within(screen, 200.0, 50.0, 96.0),
            (860.0, 515.0)
        );
        assert_eq!(
            Anchor::BottomLeft.within(screen, 200.0, 50.0, 96.0),
            (96.0, 934.0)
        );
    }

    #[test]
    fn separate_pushes_overlapping_boxes_down() {
        let boxes = [
            Rect::new(0.0, 0.0, 100.0, 20.0),
            Rect::new(50.0, 10.0, 150.0, 30.0),
            Rect::new(200.0, 0.0, 300.0, 20.0),
            Rect::new(0.0, 5.0, 100.0, 25.0),
        ];
        let offsets = separate(&boxes, 4.0);
        // the second goes below the first, the third is clear, the fourth below the second
        assert_eq!(offsets, vec![0.0, 14.0, 0.0, 43.0]);
    }

    #[test]
    fn points_and_next_to() {
        assert_eq!(Anchor::TopLeft.point(10.0, 4.0), (0.0, 0.0));
        assert_eq!(Anchor::Center.point(10.0, 4.0), (5.0, 2.0));
        assert_eq!(Anchor::BottomRight.point(10.0, 4.0), (10.0, 4.0));
        // above the point: the box's bottom center is `buff` above it
        assert_eq!(
            Anchor::Top.next_to(100.0, 100.0, 10.0, 4.0, 2.0),
            (95.0, 94.0)
        );
        // up right: the box's bottom left corner is diagonally away
        assert_eq!(
            Anchor::TopRight.next_to(100.0, 100.0, 10.0, 4.0, 2.0),
            (102.0, 94.0)
        );
        assert_eq!(
            Anchor::Center.next_to(100.0, 100.0, 10.0, 4.0, 2.0),
            (95.0, 98.0)
        );
    }

    #[test]
    fn tidy_trees_center_parents_and_keep_subtrees_apart() {
        // 0 has children 1, 2; 1 has 3, 4; 2 has 5
        let children = vec![vec![1, 2], vec![3, 4], vec![5], vec![], vec![], vec![]];
        let p = tidy_tree(&children, 0);
        assert_eq!(p[3], (0.0, 2.0));
        assert_eq!(p[4], (1.0, 2.0));
        // the parents sit over the middle of their children
        assert_eq!(p[1], (0.5, 1.0));
        assert_eq!(p[5].0, p[2].0);
        assert_eq!(p[0].0, f64::midpoint(p[1].0, p[2].0));
        // no two nodes at a depth are closer than one slot
        for a in 0..6 {
            for b in a + 1..6 {
                if p[a].1 == p[b].1 {
                    assert!((p[a].0 - p[b].0).abs() >= 1.0 - 1e-9, "{a} {b} {p:?}");
                }
            }
        }
        // deterministic, and the same again
        assert_eq!(p, tidy_tree(&children, 0));
    }

    #[test]
    fn tidy_trees_clear_deep_neighbours() {
        // two chains under a root: the left one grows a wide bottom, which must not touch
        // the right one
        let children = vec![
            vec![1, 2],
            vec![3],
            vec![4],
            vec![5, 6, 7],
            vec![8],
            vec![],
            vec![],
            vec![],
            vec![],
        ];
        let p = tidy_tree(&children, 0);
        assert!(p[8].0 - p[7].0 >= 1.0 - 1e-9, "{p:?}");
        // a child listed under two parents stays with the first
        let shared = vec![vec![1, 2], vec![2, 3], vec![], vec![]];
        let r = tidy_tree(&shared, 0);
        assert_eq!((r[2].1, r[3].1), (1.0, 2.0));
        // a cycle or an unreachable node does not loop or move
        let odd = vec![vec![1], vec![0, 1], vec![]];
        let q = tidy_tree(&odd, 0);
        assert_eq!(q, vec![(0.0, 0.0), (0.0, 1.0), (0.0, 0.0)]);
    }

    #[cfg(any(feature = "cs", feature = "biology"))]
    #[test]
    fn layered_graphs_point_down_and_keep_every_node() {
        let edges = [(0, 1), (0, 2), (1, 3), (2, 3), (3, 1), (5, 6), (4, 4)];
        let p = layered(8, &edges);
        assert_eq!(p.len(), 8);
        assert_eq!(p[0].1, 0.0);
        assert!(p[1].1 > p[0].1 && p[2].1 > p[0].1);
        // the second part sits right of the first, the lone nodes after both
        let first = [0, 1, 2, 3].map(|i| p[i].0).into_iter().fold(0.0, f64::max);
        assert!(p[5].0 > first && p[6].0 > first, "{p:?}");
        assert!(p[4].0 > p[5].0.max(p[6].0) && p[7].0 > p[4].0, "{p:?}");
        assert_eq!((p[4].1, p[7].1), (0.0, 0.0));
        // nodes in a layer are a slot apart
        for a in 0..8 {
            for b in a + 1..8 {
                if p[a].1 == p[b].1 {
                    assert!((p[a].0 - p[b].0).abs() >= 1.0 - 1e-9, "{a} {b} {p:?}");
                }
            }
        }
        assert_eq!(p, layered(8, &edges));
    }
}
