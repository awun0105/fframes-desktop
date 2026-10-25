//! The drawing between two neighbouring layers, one family per kind of pair as in `ManimML`.
//! The lines of a connection are also the paths of the forward pass's pulses.

use kurbo::{Line, Point};

use super::layers::LayerGeometry;

/// The lines between two neighbouring layers.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Connection {
    pub lines: Vec<Line>,
    /// Ends with an arrow tip: the layers have no neurons to join.
    pub arrow: bool,
}

/// `from` to `to`, without the first `start` and the last `end` pixels.
fn trimmed(from: Point, to: Point, start: f64, end: f64) -> Line {
    let d = to - from;
    let length = d.hypot();
    if length <= start + end {
        return Line::new(from, to);
    }
    let unit = d / length;
    Line::new(from + unit * start, to - unit * end)
}

/// The connection from `a` to `b`, the layer after it; neurons are `radius` pixels round.
/// - neurons to neurons (feed-forward to feed-forward): every pair;
/// - neurons to a box, or a box to neurons: a fan from every neuron to the box's side;
/// - a box to a box: one arrow.
pub(super) fn connect(a: &LayerGeometry, b: &LayerGeometry, radius: f64) -> Connection {
    let lines = match (a.nodes.is_empty(), b.nodes.is_empty()) {
        (false, false) => a
            .nodes
            .iter()
            .flat_map(|&p| b.nodes.iter().map(move |&q| trimmed(p, q, radius, radius)))
            .collect(),
        (false, true) => a
            .nodes
            .iter()
            .map(|&p| trimmed(p, b.in_port(), radius, 0.0))
            .collect(),
        (true, false) => b
            .nodes
            .iter()
            .map(|&q| trimmed(a.out_port(), q, 0.0, radius))
            .collect(),
        (true, true) => vec![Line::new(a.out_port(), b.in_port())],
    };
    Connection {
        lines,
        arrow: a.nodes.is_empty() && b.nodes.is_empty(),
    }
}

#[cfg(test)]
mod tests {
    use kurbo::Rect;

    use super::*;

    fn layer(x: f64, nodes: &[f64]) -> LayerGeometry {
        let center = Point::new(x, 0.0);
        LayerGeometry {
            center,
            bounds: Rect::from_center_size(center, (20.0, 100.0)),
            nodes: nodes.iter().map(|&y| Point::new(x, y)).collect(),
        }
    }

    #[test]
    fn families() {
        let dense = connect(
            &layer(0.0, &[-10.0, 10.0]),
            &layer(100.0, &[-20.0, 0.0, 20.0]),
            5.0,
        );
        assert_eq!(dense.lines.len(), 6);
        assert!(!dense.arrow);
        // each line starts on the edge of its neuron
        let first = dense.lines[0];
        assert!((first.p0.distance(Point::new(0.0, -10.0)) - 5.0).abs() < 1e-9);
        let fan = connect(&layer(0.0, &[-10.0, 10.0]), &layer(100.0, &[]), 5.0);
        assert_eq!(fan.lines.len(), 2);
        assert!(fan.lines.iter().all(|l| l.p1 == Point::new(90.0, 0.0)));
        let boxes = connect(&layer(0.0, &[]), &layer(100.0, &[]), 5.0);
        assert_eq!(
            boxes.lines,
            vec![Line::new(Point::new(10.0, 0.0), Point::new(90.0, 0.0))]
        );
        assert!(boxes.arrow);
    }
}
