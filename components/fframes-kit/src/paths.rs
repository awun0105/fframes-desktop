//! Path utilities over [`kurbo::BezPath`], with `@remotion/paths` names in snake case, plus
//! [`align_points`] for morphing (Manim's `align_points`).
//!
//! Paths are absolute (kurbo has no relative commands or shorthands); lengths are arc lengths
//! in canvas units.

use kurbo::{
    Affine, BezPath, CubicBez, ParamCurve, ParamCurveArclen, PathEl, PathSeg, Point, Rect,
    Shape as _, Vec2,
};

/// Accuracy of arc-length computations, in canvas units.
const ACCURACY: f64 = 1e-3;

/// Arc length of the whole path.
pub fn get_length(path: &BezPath) -> f64 {
    path.segments().map(|s| s.arclen(ACCURACY)).sum()
}

/// The segment and its parameter at arc length `length`, clamped to the path.
fn seg_at_length(path: &BezPath, length: f64) -> Option<(PathSeg, f64)> {
    let mut rest = length.max(0.0);
    let mut last = None;
    for seg in path.segments() {
        let len = seg.arclen(ACCURACY);
        if rest <= len {
            return Some((seg, seg.inv_arclen(rest, ACCURACY)));
        }
        rest -= len;
        last = Some(seg);
    }
    last.map(|seg| (seg, 1.0))
}

/// The point at arc length `length` from the start; `None` for a path without segments.
pub fn get_point_at_length(path: &BezPath, length: f64) -> Option<Point> {
    seg_at_length(path, length).map(|(seg, t)| seg.eval(t))
}

/// The unit direction of the path at arc length `length`.
pub fn get_tangent_at_length(path: &BezPath, length: f64) -> Option<Vec2> {
    let (seg, t) = seg_at_length(path, length)?;
    let h = 1e-4;
    let (a, b) = ((t - h).max(0.0), (t + h).min(1.0));
    let d = seg.eval(b) - seg.eval(a);
    (d.hypot() > 0.0).then(|| d.normalize())
}

/// Dash settings that draw the first `progress` (0..=1) of the outline.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Evolve {
    /// `stroke-dasharray`: one dash and one gap of the full length.
    pub dasharray: [f64; 2],
    pub dashoffset: f64,
}

/// `stroke-dasharray` and `stroke-dashoffset` that draw the first `progress` of `path`.
pub fn evolve_path(progress: f64, path: &BezPath) -> Evolve {
    let length = get_length(path);
    Evolve {
        dasharray: [length, length],
        dashoffset: length * (1.0 - progress.clamp(0.0, 1.0)),
    }
}

/// Moves every point by `(dx, dy)`.
pub fn translate_path(path: &BezPath, dx: f64, dy: f64) -> BezPath {
    Affine::translate((dx, dy)) * path.clone()
}

/// Scales every point about the origin.
pub fn scale_path(path: &BezPath, sx: f64, sy: f64) -> BezPath {
    Affine::scale_non_uniform(sx, sy) * path.clone()
}

/// The same outline drawn in the opposite direction.
pub fn reverse_path(path: &BezPath) -> BezPath {
    path.reverse_subpaths()
}

/// The first `length` units of the path.
pub fn cut_path(path: &BezPath, length: f64) -> BezPath {
    let mut out = BezPath::new();
    let mut rest = length.max(0.0);
    let mut pen: Option<Point> = None;
    for seg in path.segments() {
        if rest <= 0.0 {
            break;
        }
        let len = seg.arclen(ACCURACY);
        let piece = if rest >= len {
            seg
        } else {
            seg.subsegment(0.0..seg.inv_arclen(rest, ACCURACY))
        };
        rest -= len;
        let start = piece.start();
        if pen != Some(start) {
            out.move_to(start);
        }
        push_seg(&mut out, piece);
        pen = Some(piece.end());
    }
    out
}

fn push_seg(out: &mut BezPath, seg: PathSeg) {
    match seg {
        PathSeg::Line(l) => out.line_to(l.p1),
        PathSeg::Quad(q) => out.quad_to(q.p1, q.p2),
        PathSeg::Cubic(c) => out.curve_to(c.p1, c.p2, c.p3),
    }
}

/// Maps every point through `f`. Segments are first split into cubics no longer than
/// `threshold` units, so straight lines bend smoothly.
pub fn warp_path(path: &BezPath, threshold: f64, f: impl Fn(Point) -> Point) -> BezPath {
    let mut out = BezPath::new();
    for sub in get_subpaths(path) {
        let (start, cubics, closed) = subpath_cubics(&sub);
        let Some(start) = start else { continue };
        out.move_to(f(start));
        for c in cubics {
            let pieces = (c.arclen(ACCURACY) / threshold.max(1e-6)).ceil().max(1.0) as usize;
            for i in 0..pieces {
                let piece = c.subsegment(i as f64 / pieces as f64..(i + 1) as f64 / pieces as f64);
                out.curve_to(f(piece.p1), f(piece.p2), f(piece.p3));
            }
        }
        if closed {
            out.close_path();
        }
    }
    out
}

/// The path with absolute commands only. kurbo paths are always absolute, so this is a copy;
/// it is here for Remotion's name.
pub fn normalize_path(path: &BezPath) -> BezPath {
    path.clone()
}

/// The path with `M`, `L`, `C` and `Z` only: quadratic curves become cubics.
pub fn reduce_instructions(path: &BezPath) -> BezPath {
    let mut out = BezPath::new();
    let mut last = Point::ZERO;
    for el in path.elements() {
        match *el {
            PathEl::QuadTo(p1, p2) => {
                let c = kurbo::QuadBez::new(last, p1, p2).raise();
                out.curve_to(c.p1, c.p2, c.p3);
                last = p2;
            }
            el => {
                out.push(el);
                if let Some(p) = el.end_point() {
                    last = p;
                }
            }
        }
    }
    out
}

/// The smallest rectangle around the path.
pub fn get_bounding_box(path: &BezPath) -> Rect {
    path.bounding_box()
}

/// Each `M ...` run of the path as its own path.
pub fn get_subpaths(path: &BezPath) -> Vec<BezPath> {
    let mut subs: Vec<BezPath> = Vec::new();
    for el in path.elements() {
        if matches!(el, PathEl::MoveTo(_)) || subs.is_empty() {
            subs.push(BezPath::new());
        }
        if let Some(sub) = subs.last_mut() {
            sub.push(*el);
        }
    }
    subs.retain(|s| s.elements().len() > 1);
    subs
}

/// `rect` scaled by `scale` about its center, for a view box with room around a path.
pub fn extend_view_box(rect: Rect, scale: f64) -> Rect {
    let c = rect.center();
    let (w, h) = (rect.width() * scale / 2.0, rect.height() * scale / 2.0);
    Rect::new(c.x - w, c.y - h, c.x + w, c.y + h)
}

/// The path moved so the center of its bounding box is at `(cx, cy)`.
pub fn center_path(path: &BezPath, cx: f64, cy: f64) -> BezPath {
    let c = path.bounding_box().center();
    translate_path(path, cx - c.x, cy - c.y)
}

/// The path moved so its bounding box starts at `(0, 0)`.
pub fn reset_path(path: &BezPath) -> BezPath {
    let b = path.bounding_box();
    translate_path(path, -b.x0, -b.y0)
}

/// Parses SVG path data (`d`).
pub fn parse_path(d: &str) -> Result<BezPath, kurbo::SvgParseError> {
    BezPath::from_svg(d)
}

/// SVG path data (`d`) for the path.
pub fn serialize(path: &BezPath) -> String {
    path.to_svg()
}

/// A subpath as cubics: start point, segments and whether it is closed.
fn subpath_cubics(sub: &BezPath) -> (Option<Point>, Vec<CubicBez>, bool) {
    let start = sub.elements().iter().find_map(|el| match el {
        PathEl::MoveTo(p) => Some(*p),
        _ => None,
    });
    let closed = matches!(sub.elements().last(), Some(PathEl::ClosePath));
    let cubics = sub
        .segments()
        .map(|seg| match seg {
            PathSeg::Line(l) => CubicBez::new(
                l.p0,
                l.p0.lerp(l.p1, 1.0 / 3.0),
                l.p0.lerp(l.p1, 2.0 / 3.0),
                l.p1,
            ),
            PathSeg::Quad(q) => q.raise(),
            PathSeg::Cubic(c) => c,
        })
        .collect();
    (start, cubics, closed)
}

/// Splits the longest cubics in half until there are `count`.
fn subdivide(mut cubics: Vec<CubicBez>, count: usize) -> Vec<CubicBez> {
    while cubics.len() < count {
        let (i, _) = cubics
            .iter()
            .enumerate()
            .map(|(i, c)| {
                (
                    i,
                    (c.p1 - c.p0).hypot() + (c.p2 - c.p1).hypot() + (c.p3 - c.p2).hypot(),
                )
            })
            .fold((0, f64::MIN), |best, x| if x.1 > best.1 { x } else { best });
        let c = cubics[i];
        cubics.splice(i..=i, [c.subsegment(0.0..0.5), c.subsegment(0.5..1.0)]);
    }
    cubics
}

struct Aligned {
    cubics: Vec<CubicBez>,
    closed: bool,
}

fn aligned_subpaths(path: &BezPath) -> Vec<Aligned> {
    get_subpaths(path)
        .iter()
        .map(subpath_cubics)
        .filter(|(_, c, _)| !c.is_empty())
        .map(|(_, cubics, closed)| Aligned { cubics, closed })
        .collect()
}

fn build(subs: &[Aligned]) -> BezPath {
    let mut out = BezPath::new();
    for sub in subs {
        let Some(first) = sub.cubics.first() else {
            continue;
        };
        out.move_to(first.p0);
        for c in &sub.cubics {
            out.curve_to(c.p1, c.p2, c.p3);
        }
        if sub.closed {
            out.close_path();
        }
    }
    out
}

/// Both paths rewritten with the same number of subpaths, and the same number (at least `n`)
/// of cubic segments in each, so they can be interpolated point by point. Missing subpaths
/// are added as points at the end of the last one.
pub fn align_points(a: &BezPath, b: &BezPath, n: usize) -> (BezPath, BezPath) {
    let (mut sa, mut sb) = (aligned_subpaths(a), aligned_subpaths(b));
    let pad = |subs: &mut Vec<Aligned>, count: usize| {
        let end = subs
            .last()
            .and_then(|s| s.cubics.last())
            .map_or(Point::ZERO, |c| c.p3);
        while subs.len() < count {
            subs.push(Aligned {
                cubics: vec![CubicBez::new(end, end, end, end)],
                closed: false,
            });
        }
    };
    let count = sa.len().max(sb.len());
    pad(&mut sa, count);
    pad(&mut sb, count);
    for (x, y) in sa.iter_mut().zip(sb.iter_mut()) {
        let k = x.cubics.len().max(y.cubics.len()).max(n);
        x.cubics = subdivide(std::mem::take(&mut x.cubics), k);
        y.cubics = subdivide(std::mem::take(&mut y.cubics), k);
        let closed = x.closed && y.closed;
        x.closed = closed;
        y.closed = closed;
    }
    (build(&sa), build(&sb))
}

/// The path `t` of the way from `a` to `b` (0 is `a`'s shape, 1 is `b`'s).
pub fn interpolate_path(t: f64, a: &BezPath, b: &BezPath) -> BezPath {
    let (a, b) = align_points(a, b, 0);
    lerp_aligned(t, &a, &b)
}

/// Interpolates two paths already aligned with [`align_points`].
pub fn lerp_aligned(t: f64, a: &BezPath, b: &BezPath) -> BezPath {
    let lerp = |p: Point, q: Point| p.lerp(q, t);
    a.elements()
        .iter()
        .zip(b.elements())
        .map(|(x, y)| match (*x, *y) {
            (PathEl::MoveTo(p), PathEl::MoveTo(q)) => PathEl::MoveTo(lerp(p, q)),
            (PathEl::CurveTo(p1, p2, p3), PathEl::CurveTo(q1, q2, q3)) => {
                PathEl::CurveTo(lerp(p1, q1), lerp(p2, q2), lerp(p3, q3))
            }
            (x, _) => x,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use kurbo::Circle;

    fn unit_circle() -> BezPath {
        Circle::new((0.0, 0.0), 1.0).to_path(1e-6)
    }

    fn square() -> BezPath {
        parse_path("M0 0 L10 0 L10 10 L0 10 Z").unwrap()
    }

    #[test]
    fn length_of_unit_circle_is_tau() {
        assert!((get_length(&unit_circle()) - std::f64::consts::TAU).abs() < 1e-3);
        assert!((get_length(&square()) - 40.0).abs() < 1e-9);
    }

    #[test]
    fn points_and_tangents_along_a_square() {
        let s = square();
        let p = get_point_at_length(&s, 15.0).unwrap();
        assert!((p.x - 10.0).abs() < 1e-6 && (p.y - 5.0).abs() < 1e-6);
        let t = get_tangent_at_length(&s, 15.0).unwrap();
        assert!((t.y - 1.0).abs() < 1e-6);
        let end = get_point_at_length(&s, 1000.0).unwrap();
        assert!((end.x).abs() < 1e-9 && (end.y).abs() < 1e-9);
        assert!(get_point_at_length(&BezPath::new(), 1.0).is_none());
    }

    #[test]
    fn cut_path_halves() {
        let c = unit_circle();
        let half = cut_path(&c, get_length(&c) / 2.0);
        assert!((get_length(&half) - std::f64::consts::PI).abs() < 1e-3);
        assert!((get_length(&cut_path(&square(), 25.0)) - 25.0).abs() < 1e-6);
    }

    #[test]
    fn evolve_draws_the_given_fraction() {
        let e = evolve_path(0.25, &square());
        assert_eq!(e.dasharray, [40.0, 40.0]);
        assert!((e.dashoffset - 30.0).abs() < 1e-9);
    }

    #[test]
    fn interpolate_path_endpoints_equal_inputs() {
        let (a, b) = (square(), center_path(&unit_circle(), 5.0, 5.0));
        let start = interpolate_path(0.0, &a, &b);
        let end = interpolate_path(1.0, &a, &b);
        assert!((get_length(&start) - get_length(&a)).abs() < 1e-3);
        assert!((get_length(&end) - get_length(&b)).abs() < 1e-3);
        assert!((get_bounding_box(&start).area() - 100.0).abs() < 1e-6);
        let mid = interpolate_path(0.5, &a, &b);
        assert!(get_length(&mid) > get_length(&b) && get_length(&mid) < get_length(&a));
    }

    #[test]
    fn align_points_matches_counts() {
        let two = parse_path("M0 0 L1 0 L1 1 Z M5 5 L6 5 L6 6 Z").unwrap();
        let (a, b) = align_points(&square(), &two, 6);
        assert_eq!(a.elements().len(), b.elements().len());
        assert_eq!(get_subpaths(&a).len(), 2);
        assert!(get_subpaths(&a)[0].segments().count() >= 6);
    }

    #[test]
    fn transforms_and_boxes() {
        let s = square();
        assert_eq!(get_bounding_box(&translate_path(&s, 5.0, 0.0)).x0, 5.0);
        assert_eq!(get_bounding_box(&scale_path(&s, 2.0, 1.0)).width(), 20.0);
        assert_eq!(
            get_bounding_box(&center_path(&s, 0.0, 0.0)).center(),
            Point::ZERO
        );
        assert_eq!(
            get_bounding_box(&reset_path(&translate_path(&s, 3.0, 4.0))).origin(),
            Point::ZERO
        );
        assert_eq!(
            extend_view_box(Rect::new(0.0, 0.0, 10.0, 10.0), 2.0),
            Rect::new(-5.0, -5.0, 15.0, 15.0)
        );
        let r = reverse_path(&s);
        assert!((get_length(&r) - 40.0).abs() < 1e-9);
        assert_eq!(serialize(&parse_path("M0 0L1 1").unwrap()), "M0,0 L1,1");
    }

    #[test]
    fn warp_and_reduce() {
        let warped = warp_path(&square(), 2.0, |p| Point::new(p.x * 2.0, p.y));
        assert!((get_bounding_box(&warped).width() - 20.0).abs() < 1e-9);
        let quad = parse_path("M0 0 Q5 10 10 0").unwrap();
        let reduced = reduce_instructions(&quad);
        assert!(
            reduced
                .elements()
                .iter()
                .all(|e| !matches!(e, PathEl::QuadTo(..)))
        );
        assert!((get_length(&reduced) - get_length(&quad)).abs() < 1e-6);
        assert_eq!(normalize_path(&quad), quad);
    }
}
