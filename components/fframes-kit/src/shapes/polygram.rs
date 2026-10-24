//! Manim's polygons of several rings and screen frames: `Polygram`, `RegularPolygram`,
//! `ConvexHull`, `Cutout`, `ScreenRectangle` and `FullScreenRectangle`, and the polylabel
//! search `LabeledPolygram` places its label with.

use std::{cmp::Ordering, collections::BinaryHeap, f64::consts::TAU};

use kurbo::{BezPath, ParamCurveNearest, Point, Rect, Shape as _};

use super::{
    Paint, TOLERANCE,
    basic::{closed, first_vertex, polar},
    pt, shape,
};

/// The rings as canvas points.
pub(super) fn rings(groups: &[Vec<(f32, f32)>]) -> Vec<Vec<Point>> {
    groups
        .iter()
        .map(|g| g.iter().map(|&(x, y)| pt(x, y)).collect())
        .collect()
}

/// Every ring as a closed subpath of one path.
pub(super) fn ring_path(rings: &[Vec<Point>]) -> BezPath {
    let mut path = BezPath::new();
    for ring in rings.iter().filter(|r| r.len() > 1) {
        path.extend(closed(ring.iter().copied()).iter());
    }
    path
}

shape! {
    /// Several closed polygons in one shape (Manim `Polygram`), one per group of vertices.
    Polygram(t) {
        vertex_groups: Vec<Vec<(f32, f32)>> = vec![
            vec![(0.0, -100.0), (95.1, -30.9), (58.8, 80.9), (-58.8, 80.9), (-95.1, -30.9)],
        ],
    }
    paint: Paint::outline(t);
    path(s) {
        ring_path(&rings(&s.vertex_groups))
    }
}

/// The vertex groups of a regular star polygon `{n/density}`: `n` corners on a circle, each
/// joined to the one `density` steps on. When the two share a divisor the star is that many
/// polygons.
fn star_rings(center: Point, radius: f64, n: usize, density: usize, start: f64) -> Vec<Vec<Point>> {
    let n = n.max(3);
    let d = density.clamp(1, n - 1);
    let corners: Vec<Point> = (0..n)
        .map(|i| polar(center, radius, start + TAU * i as f64 / n as f64))
        .collect();
    let groups = gcd(n, d);
    (0..groups)
        .map(|g| (0..n / groups).map(|k| corners[(g + k * d) % n]).collect())
        .collect()
}

fn gcd(a: usize, b: usize) -> usize {
    if b == 0 { a } else { gcd(b, a % b) }
}

shape! {
    /// A regular star polygon (Manim `RegularPolygram`): `num_vertices` corners on a circle
    /// of `radius`, each joined to the one `density` steps on. `start_angle` is the first
    /// corner; `None` is Manim's default (right for an even count, top for an odd one).
    RegularPolygram(t) {
        cx: f32 = 0.0,
        cy: f32 = 0.0,
        radius: f32 = 100.0,
        num_vertices: usize = 5,
        density: usize = 2,
        start_angle: Option<f32> = None,
    }
    paint: Paint::outline(t);
    path(s) {
        let start = first_vertex(s.num_vertices, s.start_angle);
        let rings = star_rings(pt(s.cx, s.cy), f64::from(s.radius), s.num_vertices, s.density, start);
        ring_path(&rings)
    }
}

/// The convex hull of `points` in order round it (Andrew's monotone chain).
pub fn convex_hull(points: &[(f32, f32)]) -> Vec<(f32, f32)> {
    let mut p: Vec<(f32, f32)> = points.to_vec();
    p.sort_by(|a, b| a.partial_cmp(b).unwrap_or(Ordering::Equal));
    p.dedup();
    if p.len() < 3 {
        return p;
    }
    let cross = |o: (f32, f32), a: (f32, f32), b: (f32, f32)| {
        (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
    };
    let mut hull: Vec<(f32, f32)> = Vec::with_capacity(2 * p.len());
    for pass in [p.clone(), p.into_iter().rev().collect()] {
        let floor = hull.len();
        for q in pass {
            while hull.len() >= floor + 2
                && cross(hull[hull.len() - 2], hull[hull.len() - 1], q) <= 0.0
            {
                hull.pop();
            }
            hull.push(q);
        }
        hull.pop();
    }
    hull
}

shape! {
    /// The smallest convex polygon around `points` (Manim `ConvexHull`).
    ConvexHull(t) {
        points: Vec<(f32, f32)> = vec![(0.0, 0.0), (100.0, 20.0), (60.0, 90.0), (20.0, 40.0)],
    }
    paint: Paint::outline(t);
    path(s) {
        closed(convex_hull(&s.points).into_iter().map(|(x, y)| pt(x, y)))
    }
}

shape! {
    /// A filled shape with holes (Manim `Cutout`): `outer` with every path of `holes` cut out.
    /// The holes are turned against the outline, so they stay empty however they were drawn.
    Cutout(t) {
        outer: BezPath = kurbo::Rect::new(-100.0, -100.0, 100.0, 100.0).to_path(TOLERANCE),
        holes: Vec<BezPath> = Vec::new(),
    }
    paint: Paint::solid(t);
    path(s) {
        let sign = s.outer.area().signum();
        let mut path = s.outer.clone();
        for hole in &s.holes {
            let hole = if hole.area().signum() == sign { hole.reverse_subpaths() } else { hole.clone() };
            path.extend(hole.iter());
        }
        path
    }
}

/// A `height` tall rectangle of `aspect_ratio` centered on `(cx, cy)`.
fn screen(cx: f32, cy: f32, height: f32, aspect_ratio: f32) -> BezPath {
    let (h, w) = (
        f64::from(height) / 2.0,
        f64::from(height * aspect_ratio) / 2.0,
    );
    let c = pt(cx, cy);
    Rect::new(c.x - w, c.y - h, c.x + w, c.y + h).to_path(TOLERANCE)
}

shape! {
    /// A rectangle in a screen's proportions (Manim `ScreenRectangle`): 16:9, half the canvas
    /// tall, in its middle.
    ScreenRectangle(t) {
        cx: f32 = 960.0,
        cy: f32 = 540.0,
        height: f32 = 540.0,
        aspect_ratio: f32 = 16.0 / 9.0,
    }
    paint: Paint::outline(t);
    path(s) {
        screen(s.cx, s.cy, s.height, s.aspect_ratio)
    }
}

shape! {
    /// A rectangle over the whole 1920 x 1080 canvas (Manim `FullScreenRectangle`).
    FullScreenRectangle(t) {
        cx: f32 = 960.0,
        cy: f32 = 540.0,
        height: f32 = 1080.0,
        aspect_ratio: f32 = 16.0 / 9.0,
    }
    paint: Paint::outline(t);
    path(s) {
        screen(s.cx, s.cy, s.height, s.aspect_ratio)
    }
}

/// The distance from `p` to the nearest edge of the rings, negative outside them (even-odd).
fn signed_distance(p: Point, rings: &[Vec<Point>]) -> f64 {
    let mut inside = false;
    let mut nearest = f64::INFINITY;
    for ring in rings {
        for (i, &a) in ring.iter().enumerate() {
            let b = ring[(i + 1) % ring.len()];
            if (a.y > p.y) != (b.y > p.y) && p.x < (b.x - a.x) * (p.y - a.y) / (b.y - a.y) + a.x {
                inside = !inside;
            }
            nearest = nearest.min(kurbo::Line::new(a, b).nearest(p, 1e-9).distance_sq);
        }
    }
    let d = nearest.sqrt();
    if inside { d } else { -d }
}

/// A square cell of the polylabel search, ordered by how far inside a point of it can be.
struct Cell {
    center: Point,
    half: f64,
    distance: f64,
    reach: f64,
}

impl Cell {
    fn new(center: Point, half: f64, rings: &[Vec<Point>]) -> Self {
        let distance = signed_distance(center, rings);
        Self {
            center,
            half,
            distance,
            reach: distance + half * std::f64::consts::SQRT_2,
        }
    }
}

impl PartialEq for Cell {
    fn eq(&self, other: &Self) -> bool {
        self.reach == other.reach
    }
}

impl Eq for Cell {}

impl PartialOrd for Cell {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Cell {
    fn cmp(&self, other: &Self) -> Ordering {
        self.reach
            .partial_cmp(&other.reach)
            .unwrap_or(Ordering::Equal)
    }
}

/// The point inside the rings furthest from their edges, found to within `precision` pixels
/// (Mapbox's polylabel, which Manim's `LabeledPolygram` uses): where a label fits best.
pub fn polylabel(rings: &[Vec<Point>], precision: f64) -> Option<Point> {
    let all: Vec<Point> = rings.iter().flatten().copied().collect();
    let first = *all.first()?;
    let bounds = all
        .iter()
        .fold(Rect::from_points(first, first), |r, &p| r.union_pt(p));
    let size = bounds.width().min(bounds.height());
    if size <= 0.0 {
        return Some(bounds.center());
    }
    let precision = precision.max(1e-3);
    let mut queue = BinaryHeap::new();
    let mut y = bounds.y0;
    while y < bounds.y1 {
        let mut x = bounds.x0;
        while x < bounds.x1 {
            queue.push(Cell::new(
                Point::new(x + size / 2.0, y + size / 2.0),
                size / 2.0,
                rings,
            ));
            x += size;
        }
        y += size;
    }
    let mut best = Cell::new(bounds.center(), 0.0, rings);
    while let Some(cell) = queue.pop() {
        if cell.distance > best.distance {
            best = Cell::new(cell.center, 0.0, rings);
        }
        if cell.reach - best.distance <= precision {
            continue;
        }
        let h = cell.half / 2.0;
        for (dx, dy) in [(-h, -h), (h, -h), (-h, h), (h, h)] {
            queue.push(Cell::new(cell.center + (dx, dy), h, rings));
        }
    }
    Some(best.center)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{geometry::Geometry, shapes::assert_bbox};

    #[test]
    fn polygrams_join_their_rings() {
        assert_eq!(
            crate::paths::get_subpaths(&Polygram::default().path()).len(),
            1
        );
        // {5/2} is one star; {6/2} is two triangles
        let star = RegularPolygram::default();
        assert_eq!(crate::paths::get_subpaths(&star.path()).len(), 1);
        let hexagram = RegularPolygram {
            num_vertices: 6,
            ..star
        };
        assert_eq!(crate::paths::get_subpaths(&hexagram.path()).len(), 2);
        assert_bbox(&hexagram, -100.0, -86.6, 100.0, 86.6);
    }

    #[test]
    fn convex_hull_drops_the_inner_points() {
        let hull = convex_hull(&[
            (0.0, 0.0),
            (10.0, 0.0),
            (10.0, 10.0),
            (0.0, 10.0),
            (5.0, 5.0),
            (2.0, 3.0),
        ]);
        assert_eq!(hull.len(), 4);
        assert!(!hull.contains(&(5.0, 5.0)));
    }

    #[test]
    fn cutout_keeps_its_holes_empty() {
        let hole = kurbo::Rect::new(-50.0, -50.0, 50.0, 50.0).to_path(TOLERANCE);
        let c = Cutout {
            holes: vec![hole],
            ..Cutout::default()
        };
        // a 200 x 200 square less a 100 x 100 hole, whichever way the hole was drawn
        assert!((c.path().area().abs() - 30_000.0).abs() < 1.0);
    }

    #[test]
    fn screen_rectangles() {
        assert_bbox(&FullScreenRectangle::default(), 0.0, 0.0, 1920.0, 1080.0);
        assert_bbox(&ScreenRectangle::default(), 480.0, 270.0, 1440.0, 810.0);
    }

    #[test]
    fn polylabel_finds_the_widest_place() {
        // an L: the label goes into the wide foot, not the bend
        let l = vec![vec![
            Point::new(0.0, 0.0),
            Point::new(40.0, 0.0),
            Point::new(40.0, 160.0),
            Point::new(200.0, 160.0),
            Point::new(200.0, 240.0),
            Point::new(0.0, 240.0),
        ]];
        let p = polylabel(&l, 0.5).unwrap();
        assert!(signed_distance(p, &l) > 38.0, "{p:?}");
        assert!(p.y > 160.0, "{p:?}");
    }
}
