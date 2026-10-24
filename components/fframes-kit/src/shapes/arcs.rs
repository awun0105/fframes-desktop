//! Manim's arcs and curves between points: `ArcBetweenPoints`, `CurvedDoubleArrow`,
//! `TangentialArc`, `CubicBezier`, `AnnularSector`, `ArcPolygon` and `ArcPolygonFromArcs`.

use std::f64::consts::PI;

use kurbo::{BezPath, Point, Vec2};

use super::{
    Paint, TOLERANCE,
    angles::crossing,
    arrows::{ArrowTip, arrow_parts, curved},
    basic::{ChordArc, arc, polar},
    pt, shape,
};

/// The turn of the shorter arc of `radius` between points `chord` apart, with the sign of
/// `radius` (Manim's `radius` of `ArcBetweenPoints`). A radius shorter than half the chord
/// gives a half circle.
fn turn_of_radius(radius: f32, chord: f64) -> f64 {
    let r = f64::from(radius.abs());
    let half = (chord / 2.0).min(r);
    if r <= 0.0 {
        return 0.0;
    }
    2.0 * (half / r).asin() * f64::from(radius.signum())
}

/// The path from `from` to `to` along the arc that turns by `angle`, a line when it is
/// straight. Starts with a move to `from` when `start` is set, otherwise continues the path.
fn arc_between(path: &mut BezPath, from: Point, to: Point, angle: f64, start: bool) {
    if start {
        path.move_to(from);
    }
    match ChordArc::new(from, to, angle) {
        Some(a) => {
            arc(a.center, a.radius, a.start, a.angle).to_cubic_beziers(TOLERANCE, |p1, p2, p| {
                path.curve_to(p1, p2, p);
            });
        }
        None => path.line_to(to),
    }
}

shape! {
    /// The arc from `(x1, y1)` to `(x2, y2)` that turns by `angle`, counterclockwise on screen
    /// for a positive angle (Manim `ArcBetweenPoints`). A `radius` replaces the angle with
    /// the shorter arc of that radius (negative for the other side).
    ArcBetweenPoints(t) {
        x1: f32 = 0.0,
        y1: f32 = 0.0,
        x2: f32 = 200.0,
        y2: f32 = 0.0,
        angle: f32 = std::f32::consts::FRAC_PI_2,
        radius: Option<f32> = None,
    }
    paint: Paint::outline(t);
    path(s) {
        let mut path = BezPath::new();
        arc_between(&mut path, pt(s.x1, s.y1), pt(s.x2, s.y2), s.turn(), true);
        path
    }
}

impl ArcBetweenPoints {
    /// The turn of the arc: `angle`, or the one `radius` gives.
    pub fn turn(&self) -> f64 {
        let chord = (pt(self.x2, self.y2) - pt(self.x1, self.y1)).hypot();
        self.radius
            .map_or(f64::from(self.angle), |r| turn_of_radius(r, chord))
    }
}

shape! {
    /// An arrow along an arc with a tip at both ends (Manim `CurvedDoubleArrow`).
    CurvedDoubleArrow(t) {
        x1: f32 = 0.0,
        y1: f32 = 0.0,
        x2: f32 = 200.0,
        y2: f32 = 0.0,
        angle: f32 = std::f32::consts::FRAC_PI_2,
        tip_length: f32 = 24.0,
        tip_width: f32 = 20.0,
        tip_start: Option<ArrowTip> = Some(ArrowTip::TriangleFilled),
        tip_end: Option<ArrowTip> = Some(ArrowTip::TriangleFilled),
    }
    paint: Paint::arrow(t);
    path(s) {
        curved(
            pt(s.x1, s.y1),
            pt(s.x2, s.y2),
            f64::from(s.angle),
            (f64::from(s.tip_length), f64::from(s.tip_width)),
            s.tip_start,
            s.tip_end,
        )
    }
    parts(s) {
        arrow_parts(s.shape(), s.tip_start, s.tip_end, false)
    }
}

shape! {
    /// The arc of `radius` that touches two lines (Manim `TangentialArc`): the first from
    /// `line1.0` to `line1.1`, the second likewise. Of the four corners where the lines cross,
    /// `corner` picks the one on the side of each line's direction (`1`) or against it
    /// (`-1`).
    TangentialArc(t) {
        line1: ((f32, f32), (f32, f32)) = ((0.0, 0.0), (200.0, 0.0)),
        line2: ((f32, f32), (f32, f32)) = ((0.0, 0.0), (0.0, 200.0)),
        radius: f32 = 50.0,
        corner: (f32, f32) = (1.0, 1.0),
    }
    paint: Paint::outline(t);
    path(s) {
        let line = |l: ((f32, f32), (f32, f32))| (pt(l.0.0, l.0.1), pt(l.1.0, l.1.1));
        let (a, b) = (line(s.line1), line(s.line2));
        let mut path = BezPath::new();
        let Some(p) = crossing(a, b) else {
            return path;
        };
        let side = |v: Vec2, k: f32| v.normalize() * if k < 0.0 { -1.0 } else { 1.0 };
        let (u1, u2) = (side(a.1 - a.0, s.corner.0), side(b.1 - b.0, s.corner.1));
        let half = u1.dot(u2).clamp(-1.0, 1.0).acos() / 2.0;
        if half < 1e-6 || half > PI / 2.0 - 1e-6 {
            path.move_to(p);
            return path;
        }
        let r = f64::from(s.radius);
        let reach = r / half.tan();
        let (t1, t2) = (p + u1 * reach, p + u2 * reach);
        let center = p + (u1 + u2).normalize() * (r / half.sin());
        // the shorter way round, the side that faces the corner
        let angle_of = |q: Point| (-(q.y - center.y)).atan2(q.x - center.x);
        let (a1, a2) = (angle_of(t1), angle_of(t2));
        let sweep = (a2 - a1 + PI).rem_euclid(2.0 * PI) - PI;
        path.move_to(t1);
        arc(center, r, a1, sweep).to_cubic_beziers(TOLERANCE, |p1, p2, q| {
            path.curve_to(p1, p2, q);
        });
        path
    }
}

shape! {
    /// A cubic Bézier curve from `start` to `end` pulled by two handles (Manim `CubicBezier`).
    CubicBezier(t) {
        start: (f32, f32) = (0.0, 0.0),
        start_handle: (f32, f32) = (60.0, -100.0),
        end_handle: (f32, f32) = (140.0, -100.0),
        end: (f32, f32) = (200.0, 0.0),
    }
    paint: Paint::outline(t);
    path(s) {
        let p = |(x, y): (f32, f32)| pt(x, y);
        let mut path = BezPath::new();
        path.move_to(p(s.start));
        path.curve_to(p(s.start_handle), p(s.end_handle), p(s.end));
        path
    }
}

shape! {
    /// The part of a ring from `start_angle` by `angle` (Manim `AnnularSector`).
    AnnularSector(t) {
        cx: f32 = 0.0,
        cy: f32 = 0.0,
        inner_radius: f32 = 50.0,
        outer_radius: f32 = 100.0,
        start_angle: f32 = 0.0,
        angle: f32 = std::f32::consts::FRAC_PI_2,
    }
    paint: Paint::solid(t);
    path(s) {
        let c = pt(s.cx, s.cy);
        let (start, angle) = (f64::from(s.start_angle), f64::from(s.angle));
        let (inner, outer) = (f64::from(s.inner_radius), f64::from(s.outer_radius));
        let mut path = BezPath::new();
        path.move_to(polar(c, outer, start));
        arc(c, outer, start, angle).to_cubic_beziers(TOLERANCE, |p1, p2, p| {
            path.curve_to(p1, p2, p);
        });
        path.line_to(polar(c, inner, start + angle));
        arc(c, inner, start + angle, -angle).to_cubic_beziers(TOLERANCE, |p1, p2, p| {
            path.curve_to(p1, p2, p);
        });
        path.close_path();
        path
    }
}

shape! {
    /// A closed shape of arcs between consecutive `points`, each turning by `angle` (Manim
    /// `ArcPolygon`); a `radius` replaces the angle as on [`ArcBetweenPoints`].
    ArcPolygon(t) {
        points: Vec<(f32, f32)> = vec![(0.0, -100.0), (100.0, 100.0), (-100.0, 100.0)],
        angle: f32 = std::f32::consts::FRAC_PI_4,
        radius: Option<f32> = None,
    }
    paint: Paint::outline(t);
    path(s) {
        let arcs: Vec<ArcBetweenPoints> = s
            .points
            .iter()
            .zip(s.points.iter().cycle().skip(1))
            .map(|(&(x1, y1), &(x2, y2))| ArcBetweenPoints {
                x1,
                y1,
                x2,
                y2,
                angle: s.angle,
                radius: s.radius,
                ..ArcBetweenPoints::default()
            })
            .collect();
        joined(&arcs)
    }
}

/// The arcs one after another, joined by straight lines where one does not end where the
/// next starts, and closed.
fn joined(arcs: &[ArcBetweenPoints]) -> BezPath {
    let mut path = BezPath::new();
    for (i, a) in arcs.iter().enumerate() {
        let (from, to) = (pt(a.x1, a.y1), pt(a.x2, a.y2));
        if i == 0 {
            path.move_to(from);
        } else {
            path.line_to(from);
        }
        arc_between(&mut path, from, to, a.turn(), false);
    }
    if !arcs.is_empty() {
        path.close_path();
    }
    path
}

shape! {
    /// A closed shape of the given arcs (Manim `ArcPolygonFromArcs`): each one after the other,
    /// joined by straight lines where one does not end where the next starts.
    ArcPolygonFromArcs(t) {
        arcs: Vec<ArcBetweenPoints> = Vec::new(),
    }
    paint: Paint::outline(t);
    path(s) {
        joined(&s.arcs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{geometry::Geometry, shapes::assert_bbox};

    #[test]
    fn arc_between_points_matches_the_radius() {
        // a half circle of radius 100 over a 200 px chord, below it for a positive turn
        let half = ArcBetweenPoints {
            angle: std::f32::consts::PI,
            ..ArcBetweenPoints::default()
        };
        assert_bbox(&half, 0.0, 0.0, 200.0, 100.0);
        let by_radius = ArcBetweenPoints {
            radius: Some(100.0),
            ..ArcBetweenPoints::default()
        };
        assert!((by_radius.turn() - PI).abs() < 1e-6);
        let flatter = ArcBetweenPoints {
            radius: Some(-200.0),
            ..ArcBetweenPoints::default()
        };
        // the other side, sagging 200 - sqrt(200² - 100²) ≈ 26.8 px
        assert_bbox(&flatter, 0.0, -26.8, 200.0, 0.0);
    }

    #[test]
    fn curved_double_arrow_has_two_tips() {
        let both = CurvedDoubleArrow::default().path();
        assert_eq!(crate::paths::get_subpaths(&both).len(), 3);
    }

    #[test]
    fn tangential_arc_touches_both_lines() {
        // lines along +x and +y from the origin: the corner between them, radius 50
        let a = TangentialArc::default();
        assert_bbox(&a, 0.0, 0.0, 50.0, 50.0);
        let other = TangentialArc {
            corner: (-1.0, 1.0),
            ..a
        };
        assert_bbox(&other, -50.0, 0.0, 0.0, 50.0);
    }

    #[test]
    fn annular_sector_and_arc_polygons() {
        assert_bbox(&AnnularSector::default(), 0.0, -100.0, 100.0, 0.0);
        let p = ArcPolygon {
            angle: 0.0,
            ..ArcPolygon::default()
        };
        assert_bbox(&p, -100.0, -100.0, 100.0, 100.0);
        let from_arcs = ArcPolygonFromArcs {
            arcs: vec![
                ArcBetweenPoints::default(),
                ArcBetweenPoints {
                    x1: 200.0,
                    x2: 0.0,
                    ..ArcBetweenPoints::default()
                },
            ],
            ..ArcPolygonFromArcs::default()
        };
        // two quarter-turn arcs over the same chord make a lens
        let b = from_arcs.bbox();
        assert!(
            (b.y1 - 41.4).abs() < 0.5 && (b.y0 + 41.4).abs() < 0.5,
            "{b:?}"
        );
    }
}
