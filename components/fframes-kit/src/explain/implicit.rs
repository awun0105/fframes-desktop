//! Manim's `ImplicitFunction`: the curve where `f(x, y) = 0`.

use std::{collections::HashMap, sync::Arc};

use kurbo::Point;

use crate::{
    shapes::{Paint, shape},
    ui::Ui,
};

use super::{Axes, AxesMap, Space, graph::cached, smooth};

/// A function of two variables whose zeros make a curve. A plain `fn`, so the traced curve
/// is cached by the function's address.
pub type Implicit = fn(f64, f64) -> f64;

/// The points where `f` is zero in the box `x_range` by `y_range`, by marching squares on a
/// grid `resolution` cells wide (and as many cells high as keep them square). Pieces are
/// chained into curves, separated by a `(NaN, NaN)` point; a closed curve ends where it
/// starts. Cells where `f` is not finite are skipped.
pub fn march(
    f: Implicit,
    x_range: (f64, f64),
    y_range: (f64, f64),
    resolution: usize,
) -> Arc<[(f64, f64)]> {
    let params = [x_range.0, x_range.1, y_range.0, y_range.1].map(f64::to_bits);
    let key = (3, f as usize, [&params[..], &[resolution as u64]].concat());
    cached(key, || trace(f, x_range, y_range, resolution).into())
}

fn trace(
    f: Implicit,
    x_range: (f64, f64),
    y_range: (f64, f64),
    resolution: usize,
) -> Vec<(f64, f64)> {
    let (w, h) = (x_range.1 - x_range.0, y_range.1 - y_range.0);
    if !(w.is_finite() && h.is_finite() && w > 0.0 && h > 0.0) {
        return Vec::new();
    }
    let nx = resolution.clamp(1, 2_000);
    let ny = ((nx as f64 * h / w).round() as usize).clamp(1, 2_000);
    let (dx, dy) = (w / nx as f64, h / ny as f64);
    let at = |i: usize, j: usize| (x_range.0 + i as f64 * dx, y_range.0 + j as f64 * dy);
    let values: Vec<f64> = (0..=ny)
        .flat_map(|j| (0..=nx).map(move |i| (i, j)))
        .map(|(i, j)| {
            let (x, y) = at(i, j);
            f(x, y)
        })
        .collect();
    let v = |i: usize, j: usize| values[j * (nx + 1) + i];

    // every grid edge has an id; a crossing on it is where `f` changes sign
    let horizontal = |i: usize, j: usize| j * nx + i;
    let vertical = |i: usize, j: usize| (ny + 1) * nx + j * (nx + 1) + i;
    let mut points: HashMap<usize, (f64, f64)> = HashMap::new();
    let mut crossing = |id: usize, (i0, j0): (usize, usize), (i1, j1): (usize, usize)| {
        let (a, b) = (v(i0, j0), v(i1, j1));
        if (a >= 0.0) == (b >= 0.0) {
            return None;
        }
        points.entry(id).or_insert_with(|| {
            let t = a / (a - b);
            let ((x0, y0), (x1, y1)) = (at(i0, j0), at(i1, j1));
            (x0 + (x1 - x0) * t, y0 + (y1 - y0) * t)
        });
        Some(id)
    };
    let mut segments: Vec<(usize, usize)> = Vec::new();
    for j in 0..ny {
        for i in 0..nx {
            let corners = [v(i, j), v(i + 1, j), v(i + 1, j + 1), v(i, j + 1)];
            if !corners.iter().all(|c| c.is_finite()) {
                continue;
            }
            let bottom = crossing(horizontal(i, j), (i, j), (i + 1, j));
            let right = crossing(vertical(i + 1, j), (i + 1, j), (i + 1, j + 1));
            let top = crossing(horizontal(i, j + 1), (i, j + 1), (i + 1, j + 1));
            let left = crossing(vertical(i, j), (i, j), (i, j + 1));
            if let (Some(b), Some(r), Some(t), Some(l)) = (bottom, right, top, left) {
                // a saddle: the middle decides which corners are joined
                let middle = corners.iter().sum::<f64>() / 4.0;
                if (middle >= 0.0) == (corners[0] >= 0.0) {
                    segments.extend([(b, r), (t, l)]);
                } else {
                    segments.extend([(b, l), (t, r)]);
                }
            } else {
                let ends: Vec<usize> = [bottom, right, top, left].into_iter().flatten().collect();
                if let [a, b] = ends[..] {
                    segments.push((a, b));
                }
            }
        }
    }
    chain(&segments, &points)
}

/// Joins segments that share an edge crossing into curves.
fn chain(segments: &[(usize, usize)], points: &HashMap<usize, (f64, f64)>) -> Vec<(f64, f64)> {
    let mut at_edge: HashMap<usize, Vec<usize>> = HashMap::new();
    for (k, &(a, b)) in segments.iter().enumerate() {
        at_edge.entry(a).or_default().push(k);
        at_edge.entry(b).or_default().push(k);
    }
    let mut used = vec![false; segments.len()];
    // follows the curve from `end`, away from the segments already used
    let follow = |mut end: usize, used: &mut Vec<bool>| {
        let mut ids = Vec::new();
        while let Some(&k) = at_edge
            .get(&end)
            .and_then(|ks| ks.iter().find(|&&k| !used[k]))
        {
            used[k] = true;
            let (a, b) = segments[k];
            end = if a == end { b } else { a };
            ids.push(end);
        }
        ids
    };
    let mut out = Vec::new();
    for k in 0..segments.len() {
        if used[k] {
            continue;
        }
        used[k] = true;
        let (a, b) = segments[k];
        let mut ids: Vec<usize> = follow(a, &mut used).into_iter().rev().collect();
        ids.extend([a, b]);
        ids.extend(follow(b, &mut used));
        if !out.is_empty() {
            out.push((f64::NAN, f64::NAN));
        }
        out.extend(ids.iter().map(|id| points[id]));
    }
    out
}

shape! {
    /// The curve where `f(x, y) = 0` (Manim `ImplicitFunction`, `axes.plot_implicit_curve`),
    /// traced by marching squares on a grid `resolution` cells wide over `x_range` by
    /// `y_range`. Its points are in data units of `axes` when set, otherwise in units of
    /// `space`; build one with [`ImplicitFunction::new`] to take `ui.space`, or with
    /// [`Axes::plot_implicit_curve`]. The curve is traced once and cached.
    #[allow(unpredictable_function_pointer_comparisons)] // equal props, not equal functions
    ImplicitFunction(t) {
        f: Implicit = |x, y| x * x + y * y - 1.0,
        x_range: (f64, f64) = (-7.1, 7.1),
        y_range: (f64, f64) = (-4.0, 4.0),
        /// Grid cells across `x_range`; a finer grid follows tighter turns.
        resolution: usize = 128,
        axes: Option<AxesMap> = None,
        space: Space = Space::FULL_HD,
    }
    paint: Paint::outline(t);
    path(s) {
        let points = march(s.f, s.x_range, s.y_range, s.resolution);
        smooth(points.iter().map(|&(x, y)| s.px(x, y)))
    }
}

impl ImplicitFunction {
    /// The curve of `f` over Manim's frame, in `ui.space`, with the theme's paint.
    pub fn new(ui: &Ui<'_, '_, '_>, f: Implicit) -> Self {
        Self {
            f,
            space: ui.space,
            ..ui.defaults()
        }
    }

    /// The pixel of a point of the curve, `None` for a break.
    fn px(&self, x: f64, y: f64) -> Option<Point> {
        if !(x.is_finite() && y.is_finite()) {
            return None;
        }
        Some(match &self.axes {
            Some(map) => map.c2p(x, y),
            None => self.space.to_px(x, y),
        })
    }
}

impl Axes<'_> {
    /// The curve where `f(x, y) = 0` over the axes' ranges (Manim `plot_implicit_curve`).
    pub fn plot_implicit_curve(&self, ui: &Ui<'_, '_, '_>, f: Implicit) -> ImplicitFunction {
        let map = self.map(ui);
        let span = |r: super::Range, s: super::Scaling| (s.from_axis(r.min), s.from_axis(r.max));
        ImplicitFunction {
            f,
            x_range: span(self.x_range, self.x_scaling),
            y_range: span(self.y_range, self.y_scaling),
            axes: Some(map),
            ..ui.defaults()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_circle_is_traced_at_its_radius_as_one_closed_curve() {
        let circle: Implicit = |x, y| x * x + y * y - 4.0;
        let points = march(circle, (-3.0, 3.0), (-3.0, 3.0), 60);
        let step = 6.0 / 60.0;
        assert!(points.len() > 100, "{}", points.len());
        for &(x, y) in points.iter() {
            assert!((x.hypot(y) - 2.0).abs() < step, "{x} {y}");
        }
        assert_eq!(points.first(), points.last());
        assert!(points.iter().all(|p| p.0.is_finite()), "one piece");
    }

    #[test]
    fn separate_curves_are_broken_and_saddles_do_not_cross() {
        // x y = 0.25: two branches, the grid meets the saddle point at the origin
        let hyperbola: Implicit = |x, y| x * y - 0.25;
        let points = march(hyperbola, (-2.0, 2.0), (-2.0, 2.0), 40);
        let breaks = points.iter().filter(|p| p.0.is_nan()).count();
        assert_eq!(breaks, 1);
        for &(x, y) in points.iter().filter(|p| p.0.is_finite()) {
            assert!(x * y > 0.0, "{x} {y}");
        }
        let nothing: Implicit = |_, _| 1.0;
        assert!(march(nothing, (-1.0, 1.0), (-1.0, 1.0), 10).is_empty());
    }
}
