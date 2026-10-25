//! The drawing between two neighbouring layers, one family per kind of pair as in `ManimML`.
//! The lines of a connection are also the paths of the forward pass's pulses.

use kurbo::{Line, Point};

use crate::motion::cubic_in_out;

use super::layers::LayerGeometry;

/// The window a convolution or a pooling slides over a map, and the cell it makes in the next
/// layer's map, each as its corners (top left, top right, bottom right, bottom left).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct Filter {
    pub source: [Point; 4],
    pub target: [Point; 4],
}

/// The lines between two neighbouring layers.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Connection {
    pub lines: Vec<Line>,
    /// Ends with an arrow tip: the layers have neither neurons nor maps to join.
    pub arrow: bool,
    /// Between two layers with maps: the window and the cell its lines join.
    pub filter: Option<Filter>,
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

/// The corners of the square of `map` from cell `(col, row)`, `cells` of the map's `side`
/// cells wide.
fn window(
    g: &LayerGeometry,
    map: usize,
    col: usize,
    row: usize,
    cells: usize,
) -> Option<[Point; 4]> {
    let side = g.cells.max(1) as f64;
    let (u0, v0) = (col as f64 / side, row as f64 / side);
    let (u1, v1) = ((col + cells) as f64 / side, (row + cells) as f64 / side);
    Some([
        g.map_point(map, u0, v0)?,
        g.map_point(map, u1, v0)?,
        g.map_point(map, u1, v1)?,
        g.map_point(map, u0, v1)?,
    ])
}

/// The window `b` slides over the front map of `a` after `sweep` (0 to 1) of its slide, row
/// by row from the top left, and the cell of `b`'s back map it makes. The window glides from
/// each position to the next, slowing down at every position, so it never jumps.
fn filter(a: &LayerGeometry, b: &LayerGeometry, sweep: f64) -> Option<Filter> {
    let size = b.window.clamp(1, a.cells.max(1));
    let stride = b.stride.max(1);
    let positions = (a.cells.max(1) - size) / stride + 1;
    let count = positions * positions;
    let front = a.maps.len().checked_sub(1)?;
    let out = b.cells.max(1);
    let at = |index: usize| -> Option<Filter> {
        let (row, col) = (index / positions, index % positions);
        Some(Filter {
            source: window(a, front, col * stride, row * stride, size)?,
            target: window(
                b,
                0,
                (col * out / positions).min(out - 1),
                (row * out / positions).min(out - 1),
                1,
            )?,
        })
    };
    let x = sweep.clamp(0.0, 1.0) * (count - 1) as f64;
    let index = (x as usize).min(count - 1);
    let (from, to) = (at(index)?, at((index + 1).min(count - 1))?);
    let p = f64::from(cubic_in_out((x - index as f64) as f32));
    let blend = |a: [Point; 4], b: [Point; 4]| std::array::from_fn(|k| a[k].lerp(b[k], p));
    Some(Filter {
        source: blend(from.source, to.source),
        target: blend(from.target, to.target),
    })
}

/// The connection from `a` to `b`, the layer after it; neurons are `radius` pixels round.
/// - maps to maps (an image or a convolution into a convolution or a pooling): four lines from
///   the corners of the window `b` slides over `a`'s front map to the cell it makes, the window
///   `sweep` (0 to 1) of the way through its slide;
/// - neurons to neurons (feed-forward to feed-forward): every pair;
/// - otherwise from every neuron, or every corner of the front map, or the side of a box, to
///   every neuron of `b` or to its side; an arrow when neither has neurons or maps.
pub(super) fn connect(a: &LayerGeometry, b: &LayerGeometry, radius: f64, sweep: f64) -> Connection {
    if !a.maps.is_empty()
        && !b.maps.is_empty()
        && let Some(f) = filter(a, b, sweep)
    {
        return Connection {
            lines: (0..4)
                .map(|k| Line::new(f.source[k], f.target[k]))
                .collect(),
            arrow: false,
            filter: Some(f),
        };
    }
    let (sources, start): (Vec<Point>, f64) = if !a.nodes.is_empty() {
        (a.nodes.clone(), radius)
    } else if let Some(front) = a.maps.last() {
        (front.to_vec(), 0.0)
    } else {
        (vec![a.out_port()], 0.0)
    };
    let (targets, end): (Vec<Point>, f64) = if b.nodes.is_empty() {
        (vec![b.in_port()], 0.0)
    } else {
        (b.nodes.clone(), radius)
    };
    let lines = sources
        .iter()
        .flat_map(|&p| targets.iter().map(move |&q| trimmed(p, q, start, end)))
        .collect();
    Connection {
        lines,
        arrow: a.nodes.is_empty() && a.maps.is_empty() && b.nodes.is_empty() && b.maps.is_empty(),
        filter: None,
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
            maps: Vec::new(),
            cells: 0,
            window: 1,
            stride: 1,
        }
    }

    /// One upright square map, 10 pixels a cell.
    fn mapped(x: f64, cells: usize, window: usize, stride: usize) -> LayerGeometry {
        let side = cells as f64 * 10.0;
        let mut g = layer(x, &[]);
        g.bounds = Rect::new(x, 0.0, x + side, side);
        g.maps = vec![[
            Point::new(x, 0.0),
            Point::new(x + side, 0.0),
            Point::new(x + side, side),
            Point::new(x, side),
        ]];
        g.cells = cells;
        g.window = window;
        g.stride = stride;
        g
    }

    #[test]
    fn families() {
        let dense = connect(
            &layer(0.0, &[-10.0, 10.0]),
            &layer(100.0, &[-20.0, 0.0, 20.0]),
            5.0,
            0.0,
        );
        assert_eq!(dense.lines.len(), 6);
        assert!(!dense.arrow);
        // each line starts on the edge of its neuron
        let first = dense.lines[0];
        assert!((first.p0.distance(Point::new(0.0, -10.0)) - 5.0).abs() < 1e-9);
        let fan = connect(&layer(0.0, &[-10.0, 10.0]), &layer(100.0, &[]), 5.0, 0.0);
        assert_eq!(fan.lines.len(), 2);
        assert!(fan.lines.iter().all(|l| l.p1 == Point::new(90.0, 0.0)));
        let boxes = connect(&layer(0.0, &[]), &layer(100.0, &[]), 5.0, 0.0);
        assert_eq!(
            boxes.lines,
            vec![Line::new(Point::new(10.0, 0.0), Point::new(90.0, 0.0))]
        );
        assert!(boxes.arrow);
        // the corners of the front map to every neuron
        let flatten = connect(
            &mapped(0.0, 4, 1, 1),
            &layer(200.0, &[-10.0, 10.0]),
            5.0,
            0.0,
        );
        assert_eq!(flatten.lines.len(), 8);
        assert!(!flatten.arrow);
    }

    #[test]
    fn the_filter_slides_row_by_row() {
        // a 3-cell filter over a 6-cell map: 4 by 4 positions into a 4-cell map
        let (a, b) = (mapped(0.0, 6, 1, 1), mapped(100.0, 4, 3, 1));
        let start = connect(&a, &b, 5.0, 0.0);
        assert_eq!(start.lines.len(), 4);
        let f = start.filter.unwrap();
        assert_eq!(f.source[0], Point::new(0.0, 0.0));
        assert_eq!(f.source[2], Point::new(30.0, 30.0));
        assert_eq!(f.target[0], Point::new(100.0, 0.0));
        assert_eq!(f.target[2], Point::new(110.0, 10.0));
        // the lines join the corners of the window to the corners of the cell
        assert_eq!(start.lines[2], Line::new(f.source[2], f.target[2]));
        // the fifth of 16 positions: the second row starts
        let row = connect(&a, &b, 5.0, 4.0 / 15.0).filter.unwrap();
        assert_eq!(row.source[0], Point::new(0.0, 10.0));
        assert_eq!(row.target[0], Point::new(100.0, 10.0));
        // the end: the bottom right corner
        let end = connect(&a, &b, 5.0, 1.0).filter.unwrap();
        assert_eq!(end.source[2], Point::new(60.0, 60.0));
        assert_eq!(end.target[2], Point::new(140.0, 40.0));
    }

    #[test]
    fn pooling_makes_one_cell_of_each_window() {
        // a 2-cell kernel over 6 cells, 2 cells at a time: 3 by 3 windows into 3 cells
        let (a, b) = (mapped(0.0, 6, 1, 1), mapped(100.0, 3, 2, 2));
        let second = connect(&a, &b, 5.0, 1.0 / 8.0).filter.unwrap();
        assert_eq!(second.source[0], Point::new(20.0, 0.0));
        assert_eq!(second.target[0], Point::new(110.0, 0.0));
        let end = connect(&a, &b, 5.0, 1.0).filter.unwrap();
        assert_eq!(end.source[0], Point::new(40.0, 40.0));
        assert_eq!(end.target[0], Point::new(120.0, 20.0));
        // half way between the first two positions the window is half a stride along
        let half = connect(&a, &b, 5.0, 1.0 / 16.0).filter.unwrap();
        assert!((half.source[0].x - 10.0).abs() < 1e-9 && half.source[0].y == 0.0);
    }
}
