//! Names in columns beside a figure, each joined to its part by a leader line that draws
//! when the part appears.

use fframes::{Color, Svgr};
use kurbo::{BezPath, Circle, Point, Shape as _};

use crate::{
    geometry::Shape,
    kits::draw::{partial, stroke},
    motion::{cubic_in_out, progress},
    text::{TextAnchor, line},
    theme::TextStyle,
};

/// A name for the point `anchor` (pixels), shown from `start` seconds.
pub(crate) struct Callout {
    pub text: String,
    pub anchor: Point,
    pub start: f32,
}

/// The rows of names in one column: as near their anchors' heights as `row` pixels between
/// them allow, inside `top..bottom`.
pub(crate) fn rows(anchors: &[f64], row: f64, top: f64, bottom: f64) -> Vec<f64> {
    let mut order: Vec<usize> = (0..anchors.len()).collect();
    order.sort_by(|&a, &b| anchors[a].total_cmp(&anchors[b]));
    let mut ys: Vec<f64> = order
        .iter()
        .map(|&i| anchors[i].clamp(top, bottom))
        .collect();
    for k in 1..ys.len() {
        ys[k] = ys[k].max(ys[k - 1] + row);
    }
    if let Some(last) = ys.last_mut() {
        *last = last.min(bottom.max(top + row * (anchors.len() - 1) as f64));
    }
    for k in (0..ys.len().saturating_sub(1)).rev() {
        ys[k] = ys[k].min(ys[k + 1] - row);
    }
    let mut out = vec![0.0; anchors.len()];
    for (k, &i) in order.iter().enumerate() {
        out[i] = ys[k];
    }
    out
}

/// Draws `items` at `t` seconds: names left of `columns.0` for anchors in the left half,
/// right of `columns.1` for the others, rows inside `span`. Each leader runs from a dot on
/// its anchor to an elbow at its row and on to the name, drawing in 0.4 seconds from its
/// start; the name fades in as it arrives.
#[allow(clippy::too_many_arguments)]
pub(crate) fn callouts<'a>(
    items: &[Callout],
    t: f32,
    columns: (f64, f64),
    span: (f64, f64),
    style: TextStyle,
    text_color: Color,
    leader_color: Color,
) -> Svgr<'a> {
    let middle = f64::midpoint(columns.0, columns.1);
    let row = style.size as f64 * 1.45;
    let mut drawn = Vec::new();
    for left in [true, false] {
        let side: Vec<&Callout> = items
            .iter()
            .filter(|c| (c.anchor.x < middle) == left)
            .collect();
        let anchors: Vec<f64> = side.iter().map(|c| c.anchor.y).collect();
        let ys = rows(&anchors, row, span.0, span.1);
        let (x, out, text_anchor) = if left {
            (columns.0, -1.0, TextAnchor::End)
        } else {
            (columns.1, 1.0, TextAnchor::Start)
        };
        for (callout, y) in side.into_iter().zip(ys) {
            let p = cubic_in_out(progress(t, callout.start, callout.start + 0.4));
            if p <= 0.0 {
                continue;
            }
            let points = [
                callout.anchor,
                Point::new(x - out * 36.0, y),
                Point::new(x - out * 8.0, y),
            ];
            let way = partial(&points, f64::from(p));
            let mut path = BezPath::new();
            path.move_to(way[0]);
            for &q in &way[1..] {
                path.line_to(q);
            }
            drawn.push(stroke(path, leader_color, 2.0, 1.0).draw());
            drawn.push(
                Shape {
                    path: Circle::new(callout.anchor, 5.0).to_path(0.1),
                    fill: Some(leader_color),
                    stroke: None,
                    stroke_width: 0.0,
                    dash: None,
                    opacity: progress(t, callout.start, callout.start + 0.15),
                }
                .draw(),
            );
            let opacity = progress(t, callout.start + 0.3, callout.start + 0.6);
            if opacity > 0.0 {
                drawn.push(line(
                    x as f32,
                    (y + style.size as f64 * 0.35) as f32,
                    style,
                    text_color,
                    text_anchor,
                    0.0,
                    opacity,
                    callout.text.clone().into(),
                ));
            }
        }
    }
    drawn.into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_keep_apart_inside_their_span() {
        let ys = rows(&[100.0, 105.0, 400.0, 102.0], 40.0, 0.0, 420.0);
        assert_eq!(ys[0], 100.0);
        assert_eq!(ys[3], 140.0);
        assert_eq!(ys[1], 180.0);
        assert_eq!(ys[2], 400.0);
        // pushed past the bottom, they move up
        let ys = rows(&[390.0, 400.0, 410.0], 40.0, 0.0, 420.0);
        assert_eq!(ys, [340.0, 380.0, 420.0]);
    }
}
