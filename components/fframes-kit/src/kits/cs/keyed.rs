//! What the figures of items with identities share (lists, stacks, queues, trees): steps
//! sampled by time, items moving, entering and leaving between two steps, named markers and
//! the step's note.

use fframes::{Color, Svgr, animation::Animatable};
use kurbo::{BezPath, CubicBez, ParamCurve, ParamCurveArclen, Point, Shape as _, Vec2};

use crate::{
    geometry::Shape,
    motion::progress,
    text::{TextAnchor, line},
    theme::TextStyle,
};

/// Where a recorded trace is at a time: the step, the one before it and how far into the
/// step.
#[derive(Debug)]
pub struct StepAt<'t, S> {
    pub index: usize,
    pub step: &'t S,
    pub previous: &'t S,
    /// Seconds since the step began (0 before the trace starts).
    pub into: f32,
}

impl<S> Clone for StepAt<'_, S> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<S> Copy for StepAt<'_, S> {}

/// The step `seconds` after the start of `len` steps, one every `step_time` seconds, and the
/// seconds since it began. Before the start it is step 0, after the end the last step.
pub(super) fn locate(len: usize, seconds: f32, step_time: f32) -> Option<(usize, f32)> {
    let last = len.checked_sub(1)?;
    if seconds <= 0.0 {
        return Some((0, 0.0));
    }
    let step_time = step_time.max(1e-3);
    let index = ((seconds / step_time) as usize).min(last);
    Some((index, seconds - index as f32 * step_time))
}

pub(super) fn sample<S>(steps: &[S], seconds: f32, step_time: f32) -> Option<StepAt<'_, S>> {
    let (index, into) = locate(steps.len(), seconds, step_time)?;
    Some(StepAt {
        index,
        step: &steps[index],
        previous: &steps[index.saturating_sub(1)],
        into,
    })
}

/// An item placed between two steps.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Tween {
    pub id: u32,
    /// Its index in the previous and in the current step, when it is in them.
    pub from: Option<usize>,
    pub to: Option<usize>,
    pub at: Point,
    pub opacity: f64,
    /// It passes other items on its way, so it swerves and is drawn above them.
    pub crossing: bool,
}

/// Places the items of the previous and the current step at `p` (0 at the previous step, 1
/// at the current one). Items in both move in a straight line, unless they pass another item
/// that also moves: then they swerve by `swerve` times the distance, to the left of their
/// way (up when moving right), so two items trading places pass each other. New items come
/// from `enter` away from their place and fade in; items that went move by `exit` and fade
/// out.
pub(super) fn tween(
    previous: &[(u32, Point)],
    current: &[(u32, Point)],
    p: f64,
    enter: Vec2,
    exit: Vec2,
    swerve: f64,
) -> Vec<Tween> {
    let find = |items: &[(u32, Point)], id: u32| items.iter().position(|(i, _)| *i == id);
    // whether the item passes another one on the way: their order along x flips
    let crosses = |id: u32, a: Point, b: Point| {
        current.iter().any(|&(other, b2)| {
            other != id
                && find(previous, other).is_some_and(|k| {
                    let a2 = previous[k].1;
                    (a.x - a2.x) * (b.x - b2.x) < 0.0
                })
        })
    };
    let wave = (std::f64::consts::PI * p).sin();
    let mut out = Vec::with_capacity(current.len());
    for (to, &(id, b)) in current.iter().enumerate() {
        out.push(match find(previous, id) {
            Some(from) => {
                let a = previous[from].1;
                let d = b - a;
                let crossing = d.hypot() > 1e-9 && crosses(id, a, b);
                let mut at = a + d * p;
                if crossing {
                    at += Vec2::new(d.y, -d.x) * (swerve * wave);
                }
                Tween {
                    id,
                    from: Some(from),
                    to: Some(to),
                    at,
                    opacity: 1.0,
                    crossing,
                }
            }
            None => Tween {
                id,
                from: None,
                to: Some(to),
                at: b + enter * (1.0 - p),
                opacity: p,
                crossing: false,
            },
        });
    }
    if p < 1.0 {
        for (from, &(id, a)) in previous.iter().enumerate() {
            if find(current, id).is_none() {
                out.push(Tween {
                    id,
                    from: Some(from),
                    to: None,
                    at: a + exit * p,
                    opacity: 1.0 - p,
                    crossing: false,
                });
            }
        }
    }
    out
}

/// The links of the previous and the current step with their opacity at `p`: kept links stay,
/// new ones fade in and the ones that went fade out. An undirected link matches its reverse.
pub(super) fn tween_links(
    previous: &[(u32, u32)],
    current: &[(u32, u32)],
    p: f64,
    directed: bool,
) -> Vec<(u32, u32, f64)> {
    let same = |a: (u32, u32), b: (u32, u32)| a == b || (!directed && a == (b.1, b.0));
    let mut out: Vec<(u32, u32, f64)> = current
        .iter()
        .map(|&link| {
            let kept = previous.iter().any(|&was| same(was, link));
            (link.0, link.1, if kept { 1.0 } else { p })
        })
        .collect();
    if p < 1.0 {
        for &was in previous {
            if !current.iter().any(|&link| same(was, link)) {
                out.push((was.0, was.1, 1.0 - p));
            }
        }
    }
    out
}

pub(super) fn find(tweens: &[Tween], id: u32) -> Option<&Tween> {
    tweens.iter().find(|t| t.id == id)
}

/// Blends two colours, `p` from 0 (`a`) to 1 (`b`).
pub(super) fn mix(a: Color, b: Color, p: f32) -> Color {
    Animatable::apply_progress(&a, &b, p)
}

/// Named markers next to the items: a small arrow to the item and its name, sliding from
/// where it was in the previous step to where it is (`p` from 0 to 1), fading in and out when
/// it appears or goes. Markers at the same `x` stack in the order given. With `below` the
/// arrows point up to `edge` from under it, otherwise down to it from above.
pub(super) fn markers<'a>(
    previous: &[(&str, f64)],
    current: &[(&str, f64)],
    p: f64,
    edge: f64,
    below: bool,
    style: TextStyle,
    color: Color,
) -> Svgr<'a> {
    fn find(marks: &[(&str, f64)], name: &str) -> Option<f64> {
        marks.iter().find(|(n, _)| *n == name).map(|&(_, x)| x)
    }
    let row_of = |marks: &[(&str, f64)], name: &str| {
        let x = find(marks, name)?;
        Some(
            marks
                .iter()
                .take_while(|(n, _)| *n != name)
                .filter(|(_, other)| (other - x).abs() < 0.5)
                .count() as f64,
        )
    };
    // (name, x, opacity, row), each moving from the previous step to this one
    let mut marks: Vec<(&str, f64, f64, f64)> = Vec::new();
    for &(name, x) in current {
        let row = row_of(current, name).unwrap_or(0.0);
        marks.push(match (find(previous, name), row_of(previous, name)) {
            (Some(was), Some(was_row)) => (
                name,
                was + (x - was) * p,
                1.0,
                was_row + (row - was_row) * p,
            ),
            _ => (name, x, p, row),
        });
    }
    if p < 1.0 {
        for &(name, was) in previous {
            if find(current, name).is_none() {
                let row = row_of(previous, name).unwrap_or(0.0);
                marks.push((name, was, 1.0 - p, row));
            }
        }
    }
    let side = if below { 1.0 } else { -1.0 };
    let row_height = style.size as f64 * 1.15;
    let mut drawn = Vec::new();
    for (name, x, opacity, row) in marks {
        if opacity <= 0.0 {
            continue;
        }
        let mut arrow = BezPath::new();
        arrow.move_to((x, edge));
        arrow.line_to((x - 9.0, edge + side * 14.0));
        arrow.line_to((x + 9.0, edge + side * 14.0));
        arrow.close_path();
        let shape = Shape {
            path: arrow,
            fill: Some(color),
            stroke: None,
            stroke_width: 0.0,
            dash: None,
            opacity: opacity as f32,
        };
        drawn.push(shape.draw());
        let baseline = if below {
            edge + 18.0 + style.size as f64 * 0.8 + row * row_height
        } else {
            edge - 20.0 - row * row_height
        };
        drawn.push(line(
            x as f32,
            baseline as f32,
            style,
            color,
            TextAnchor::Middle,
            0.0,
            opacity as f32,
            name.to_owned().into(),
        ));
    }
    drawn.into()
}

/// The note of the step, `into` seconds after it began: the previous note fades out in the
/// first half of `blend` and the new one in during the second, so they never overlap.
#[allow(clippy::too_many_arguments)]
pub(super) fn notes<'a>(
    previous: &str,
    current: &str,
    into: f32,
    x: f32,
    y: f32,
    style: TextStyle,
    fill: Color,
    blend: f32,
) -> Svgr<'a> {
    let p = progress(into, 0.0, blend);
    let (text, opacity) = if previous == current {
        (current, 1.0)
    } else if p < 0.5 {
        (previous, 1.0 - 2.0 * p)
    } else {
        (current, 2.0 * p - 1.0)
    };
    if text.is_empty() || opacity <= 0.0 {
        return Svgr::empty();
    }
    line(
        x,
        y,
        style,
        fill,
        TextAnchor::Middle,
        0.0,
        opacity,
        text.to_owned().into(),
    )
}

pub(super) fn stroke(path: BezPath, color: Color, width: f32, opacity: f32) -> Shape {
    Shape {
        path,
        fill: None,
        stroke: Some(color),
        stroke_width: width,
        dash: None,
        opacity,
    }
}

/// A filled arrow tip pointing along `dir` (a unit vector) with its point at `at`.
pub(super) fn tip(at: Point, dir: Vec2, length: f64, color: Color, opacity: f32) -> Shape {
    let side = Vec2::new(-dir.y, dir.x) * (length * 0.45);
    let base = at - dir * length;
    let mut path = BezPath::new();
    path.move_to(at);
    path.line_to(base + side);
    path.line_to(base - side);
    path.close_path();
    Shape {
        path,
        fill: Some(color),
        stroke: None,
        stroke_width: 0.0,
        dash: None,
        opacity,
    }
}

/// The left of the way from `a` to `b` on screen (up when moving right), as a unit vector.
fn left_of(a: Point, b: Point) -> Vec2 {
    let d = b - a;
    let length = d.hypot();
    if length < 1e-9 {
        return Vec2::new(0.0, -1.0);
    }
    Vec2::new(d.y, -d.x) / length
}

/// A link between round nodes at `a` and `b`, leaving and entering them at their edges
/// (`ra`, `rb`) and bent `bend` pixels to the left of its way at the middle.
pub(super) fn link(a: Point, b: Point, ra: f64, rb: f64, bend: f64) -> CubicBez {
    let control = a.midpoint(b) + left_of(a, b) * (2.0 * bend);
    let towards = |from: Point| {
        let d = control - from;
        if d.hypot() < 1e-9 {
            Vec2::ZERO
        } else {
            d.normalize()
        }
    };
    let start = a + towards(a) * ra;
    let end = b + towards(b) * rb;
    CubicBez::new(
        start,
        start + (control - start) * (2.0 / 3.0),
        end + (control - end) * (2.0 / 3.0),
        end,
    )
}

/// A loop from a round node of `radius` back to itself, above it.
pub(super) fn self_loop(center: Point, radius: f64) -> CubicBez {
    let (dx, dy) = (radius * 0.5, radius * 0.87);
    CubicBez::new(
        center + Vec2::new(-dx, -dy),
        center + Vec2::new(-radius * 1.4, -radius * 2.6),
        center + Vec2::new(radius * 1.4, -radius * 2.6),
        center + Vec2::new(dx, -dy),
    )
}

/// The bend nearest to `base` (in steps of 40 pixels) that keeps the link from `a` to `b`
/// `clearance` pixels away from every point of `obstacles`, so a long edge goes round the
/// nodes in its way instead of through them.
pub(super) fn clear_bend(
    a: Point,
    b: Point,
    radius: f64,
    obstacles: &[Point],
    clearance: f64,
    base: f64,
) -> f64 {
    let clear = |bend: f64| {
        let curve = link(a, b, radius, radius, bend);
        (1..16).all(|i| {
            let p = curve.eval(f64::from(i) / 16.0);
            obstacles.iter().all(|o| (p - *o).hypot() >= clearance)
        })
    };
    [0.0, 1.0, -1.0, 2.0, -2.0, 3.0, -3.0]
        .into_iter()
        .map(|k| base + k * 40.0)
        .find(|&bend| clear(bend))
        .unwrap_or(base)
}

/// Where the label of a link sits: beside its middle, on the side it bends to.
pub(super) fn label_point(curve: &CubicBez, offset: f64) -> Point {
    let middle = curve.eval(0.5);
    let side = left_of(curve.p0, curve.p3);
    let bulge = middle - curve.p0.midpoint(curve.p3);
    let sign = if bulge.dot(side) < -1e-6 { -1.0 } else { 1.0 };
    middle + side * (offset * sign)
}

/// `curve` drawn as an arrow (with a tip at its end when `head`).
pub(super) fn arrow_along<'a>(
    curve: CubicBez,
    head: bool,
    color: Color,
    width: f32,
    opacity: f32,
) -> Svgr<'a> {
    let tip_length = 8.0 + f64::from(width) * 2.4;
    let length = curve.arclen(0.5);
    if !head || length <= tip_length {
        return stroke(curve.into_path(0.1), color, width, opacity).draw();
    }
    let t = 1.0 - tip_length / length;
    let shaft = curve.subsegment(0.0..t);
    let dir = (curve.p3 - shaft.p3).normalize();
    let tip = tip(curve.p3, dir, tip_length, color, opacity);
    fframes::svgr!(<g>{stroke(shaft.into_path(0.1), color, width, opacity).draw()}{tip.draw()}</g>)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_are_located_by_time() {
        assert_eq!(locate(0, 1.0, 0.5), None);
        assert_eq!(locate(3, -1.0, 0.5), Some((0, 0.0)));
        assert_eq!(locate(3, 0.6, 0.5).map(|(i, _)| i), Some(1));
        assert_eq!(locate(3, 9.0, 0.5).map(|(i, _)| i), Some(2));
    }

    #[test]
    fn items_move_enter_and_leave() {
        let previous = [(1, Point::new(0.0, 0.0)), (2, Point::new(100.0, 0.0))];
        let current = [(2, Point::new(0.0, 0.0)), (3, Point::new(100.0, 0.0))];
        let half = tween(
            &previous,
            &current,
            0.5,
            Vec2::new(0.0, -40.0),
            Vec2::new(0.0, 40.0),
            0.25,
        );
        let two = find(&half, 2).unwrap();
        // 2 passes nothing that moves (1 leaves), so it slides straight
        assert_eq!((two.at, two.crossing), (Point::new(50.0, 0.0), false));
        let three = find(&half, 3).unwrap();
        assert_eq!((three.at, three.opacity), (Point::new(100.0, -20.0), 0.5));
        let one = find(&half, 1).unwrap();
        assert_eq!(
            (one.at, one.opacity, one.to),
            (Point::new(0.0, 20.0), 0.5, None)
        );
        // at the end the item that went is gone
        assert!(
            find(
                &tween(&previous, &current, 1.0, Vec2::ZERO, Vec2::ZERO, 0.0),
                1
            )
            .is_none()
        );
    }

    #[test]
    fn items_trading_places_pass_each_other() {
        let a = [(1, Point::new(0.0, 0.0)), (2, Point::new(100.0, 0.0))];
        let b = [(1, Point::new(100.0, 0.0)), (2, Point::new(0.0, 0.0))];
        let half = tween(&a, &b, 0.5, Vec2::ZERO, Vec2::ZERO, 0.25);
        let (one, two) = (find(&half, 1).unwrap(), find(&half, 2).unwrap());
        assert!(one.crossing && two.crossing);
        // the one moving right goes over the top (y up is negative on screen)
        assert!((one.at.y + 25.0).abs() < 1e-9 && (two.at.y - 25.0).abs() < 1e-9);
    }

    #[test]
    fn links_fade_in_and_out() {
        let links = tween_links(&[(1, 2), (2, 3)], &[(2, 1), (1, 3)], 0.25, false);
        assert_eq!(links, [(2, 1, 1.0), (1, 3, 0.25), (2, 3, 0.75)]);
        let directed = tween_links(&[(1, 2)], &[(2, 1)], 0.25, true);
        assert_eq!(directed, [(2, 1, 0.25), (1, 2, 0.75)]);
    }
}
