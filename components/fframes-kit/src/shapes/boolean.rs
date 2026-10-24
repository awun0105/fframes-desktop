//! Manim's boolean operations: `Union`, `Intersection`, `Difference` and `Exclusion` of paths.
//!
//! The paths are read with the nonzero fill rule, like SVG and Manim, and their curves are
//! flattened into polygons for `i_overlay`; the result is straight lines. Combining detailed
//! paths costs time, so when the inputs do not change compute the path once with [`boolean`]
//! and keep it.

use i_overlay::{
    core::{fill_rule::FillRule, overlay_rule::OverlayRule},
    float::single::SingleFloatOverlay,
};
use kurbo::{BezPath, PathEl, Point};

use super::{Paint, shape};

/// How closely the flattened polygons follow the curves, in pixels.
pub const BOOLEAN_TOLERANCE: f64 = 0.25;

/// A boolean operation on two paths.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::PropChoice))]
pub enum BooleanOp {
    /// The area inside either path.
    Union,
    /// The area inside both.
    Intersection,
    /// The area inside the first and outside the second.
    Difference,
    /// The area inside exactly one of them.
    Exclusion,
}

impl BooleanOp {
    fn rule(self) -> OverlayRule {
        match self {
            Self::Union => OverlayRule::Union,
            Self::Intersection => OverlayRule::Intersect,
            Self::Difference => OverlayRule::Difference,
            Self::Exclusion => OverlayRule::Xor,
        }
    }
}

type Contours = Vec<Vec<[f64; 2]>>;

/// Every subpath as a closed polygon.
fn contours(path: &BezPath) -> Contours {
    let mut out: Contours = Vec::new();
    kurbo::flatten(path, BOOLEAN_TOLERANCE, |el| match el {
        PathEl::MoveTo(p) => out.push(vec![[p.x, p.y]]),
        PathEl::LineTo(p) => {
            if let Some(contour) = out.last_mut() {
                contour.push([p.x, p.y]);
            }
        }
        _ => {}
    });
    out.retain(|c| c.len() > 2);
    out
}

fn overlay(a: &Contours, b: &Contours, rule: OverlayRule) -> Contours {
    a.overlay(b, rule, FillRule::NonZero)
        .into_iter()
        .flatten()
        .collect()
}

fn to_path(contours: &Contours) -> BezPath {
    let mut path = BezPath::new();
    for contour in contours {
        let mut points = contour.iter().map(|&[x, y]| Point::new(x, y));
        if let Some(first) = points.next() {
            path.move_to(first);
            points.for_each(|p| path.line_to(p));
            path.close_path();
        }
    }
    path
}

/// `a` combined with `b` by `op`. Outlines run one way and holes the other, so the result
/// draws the same under either fill rule.
pub fn boolean(a: &BezPath, b: &BezPath, op: BooleanOp) -> BezPath {
    to_path(&overlay(&contours(a), &contours(b), op.rule()))
}

/// The paths combined one after another: the first, then each next one by `rule`.
fn fold(paths: &[BezPath], rule: OverlayRule) -> BezPath {
    let Some((first, rest)) = paths.split_first() else {
        return BezPath::new();
    };
    let start = overlay(&contours(first), &Vec::new(), OverlayRule::Union);
    let result = rest
        .iter()
        .fold(start, |acc, path| overlay(&acc, &contours(path), rule));
    to_path(&result)
}

shape! {
    /// The area inside any of `paths` (Manim `Union`).
    Union(t) {
        paths: Vec<BezPath> = Vec::new(),
    }
    paint: Paint::solid(t);
    path(s) {
        fold(&s.paths, OverlayRule::Union)
    }
}

shape! {
    /// The area inside all of `paths` (Manim `Intersection`).
    Intersection(t) {
        paths: Vec<BezPath> = Vec::new(),
    }
    paint: Paint::solid(t);
    path(s) {
        fold(&s.paths, OverlayRule::Intersect)
    }
}

shape! {
    /// The area of `subject` outside `clip` (Manim `Difference`).
    Difference(t) {
        subject: BezPath = BezPath::new(),
        clip: BezPath = BezPath::new(),
    }
    paint: Paint::solid(t);
    path(s) {
        boolean(&s.subject, &s.clip, BooleanOp::Difference)
    }
}

shape! {
    /// The area inside exactly one of `subject` and `clip` (Manim `Exclusion`).
    Exclusion(t) {
        subject: BezPath = BezPath::new(),
        clip: BezPath = BezPath::new(),
    }
    paint: Paint::solid(t);
    path(s) {
        boolean(&s.subject, &s.clip, BooleanOp::Exclusion)
    }
}

#[cfg(test)]
mod tests {
    use kurbo::Shape as _;

    use super::*;
    use crate::{geometry::Geometry, paths::get_subpaths, shapes::RegularPolygram};

    fn square(x: f64, y: f64, size: f64) -> BezPath {
        kurbo::Rect::new(x, y, x + size, y + size).to_path(0.1)
    }

    /// Every piece counted as covered area, whichever way it runs.
    fn area(path: &BezPath) -> f64 {
        path.area().abs()
    }

    #[test]
    fn operations_on_two_overlapping_squares_have_the_known_areas() {
        let (a, b) = (square(0.0, 0.0, 2.0), square(1.0, 1.0, 2.0));
        let close = |op, expected: f64| {
            let got = area(&boolean(&a, &b, op));
            assert!((got - expected).abs() < 1e-9, "{op:?}: {got}");
        };
        close(BooleanOp::Union, 7.0);
        close(BooleanOp::Intersection, 1.0);
        close(BooleanOp::Difference, 3.0);
        close(BooleanOp::Exclusion, 6.0);
        let union = Union {
            paths: vec![a.clone(), b.clone(), square(10.0, 0.0, 1.0)],
            ..Union::default()
        };
        assert!((area(&union.path()) - 8.0).abs() < 1e-9);
        let both = Intersection {
            paths: vec![a, b],
            ..Intersection::default()
        };
        assert!((area(&both.path()) - 1.0).abs() < 1e-9);
    }

    #[test]
    fn a_star_keeps_its_middle_under_the_nonzero_rule() {
        // {5/2} crosses itself; nonzero fills the pentagon in the middle, even-odd would not
        let star = RegularPolygram {
            radius: 100.0,
            ..RegularPolygram::default()
        };
        let union = Union {
            paths: vec![star.path()],
            ..Union::default()
        };
        let inner = 100.0 * (0.4 * std::f64::consts::PI).cos() / (0.2 * std::f64::consts::PI).cos();
        let filled = 10.0 * 0.5 * 100.0 * inner * (0.2 * std::f64::consts::PI).sin();
        let got = area(&union.path());
        assert!((got - filled).abs() < filled * 0.01, "{got} vs {filled}");
    }

    #[test]
    fn a_hole_runs_against_its_outline() {
        let ring = boolean(
            &square(0.0, 0.0, 10.0),
            &square(3.0, 3.0, 4.0),
            BooleanOp::Difference,
        );
        let subs = get_subpaths(&ring);
        assert_eq!(subs.len(), 2);
        let (outer, hole) = (subs[0].area(), subs[1].area());
        assert!(outer.signum() != hole.signum(), "{outer} {hole}");
        assert!((ring.area().abs() - 84.0).abs() < 1e-9);
    }

    #[test]
    fn empty_inputs_give_empty_paths() {
        assert!(Union::default().path().is_empty());
        assert!(boolean(&BezPath::new(), &BezPath::new(), BooleanOp::Union).is_empty());
    }
}
