use kurbo::BezPath;

use crate::{
    explain::{AxesMap, graph_px, smooth},
    shapes::{Paint, shape},
};

shape! {
    /// A line through data points on axes (Manim `axes.plot_line_graph` without its dots;
    /// add a [`ScatterPlot`](super::ScatterPlot) for them). Straight segments, or a smooth
    /// curve with `smooth`. A point that is not finite breaks the line.
    LinePlot(t) {
        points: Vec<(f64, f64)> = Vec::new(),
        axes: AxesMap = AxesMap::default(),
        smooth: bool = false,
    }
    paint: Paint::outline(t);
    path(s) {
        let points = s.points.iter().map(|&(x, y)| graph_px(&s.axes, x, y));
        if s.smooth {
            return smooth(points);
        }
        let mut path = BezPath::new();
        let mut open = false;
        for p in points {
            match p {
                Some(p) if open => path.line_to(p),
                Some(p) => {
                    path.move_to(p);
                    open = true;
                }
                None => open = false,
            }
        }
        path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{explain::Range, geometry::Geometry};

    #[test]
    fn the_line_goes_through_the_points_and_breaks_at_gaps() {
        let axes = AxesMap {
            x: Range::new(0.0, 10.0, 1.0),
            y: Range::new(0.0, 10.0, 1.0),
            ..AxesMap::default()
        };
        let line = LinePlot {
            points: vec![
                (0.0, 0.0),
                (10.0, 10.0),
                (5.0, f64::NAN),
                (0.0, 10.0),
                (10.0, 0.0),
            ],
            axes,
            ..LinePlot::default()
        };
        let path = line.path();
        let moves = path
            .elements()
            .iter()
            .filter(|e| matches!(e, kurbo::PathEl::MoveTo(_)))
            .count();
        assert_eq!(moves, 2);
        let bbox = kurbo::Shape::bounding_box(&path);
        let (low, high) = (axes.c2p(0.0, 0.0), axes.c2p(10.0, 10.0));
        assert!((bbox.x0 - low.x).abs() < 1e-9 && (bbox.y1 - low.y).abs() < 1e-9);
        assert!((bbox.x1 - high.x).abs() < 1e-9 && (bbox.y0 - high.y).abs() < 1e-9);
    }
}
