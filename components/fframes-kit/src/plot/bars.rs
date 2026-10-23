use std::borrow::Cow;

use crate::{data::BarChart, explain::AxesMap, ui::Ui};

/// A [`BarChart`] placed on axes: the bars share the x span evenly, each slot `bar` (0 to 1)
/// filled by its bar and centered in it, so bar `i` stands at
/// `x.min + (i + 0.5) * span / n`. They stand on the x-axis and are as high as their value
/// on the y-axis. The rest of `BarChart`'s props (colours, labels, timing) come from the theme
/// and can be set on the result.
pub fn bars<'a>(
    ui: &Ui<'_, '_, '_>,
    axes: &AxesMap,
    bars: Vec<(Cow<'a, str>, f64)>,
    bar: f32,
) -> BarChart<'a> {
    place(
        BarChart {
            bars,
            ..ui.defaults()
        },
        axes,
        bar,
    )
}

/// `chart` with its position and scale taken from the axes.
fn place<'a>(chart: BarChart<'a>, axes: &AxesMap, bar: f32) -> BarChart<'a> {
    let n = chart.bars.len().max(1) as f64;
    let base = axes.x_axis_y();
    let left = axes.c2p(axes.x.min, base);
    let right = axes.c2p(axes.x.max, base);
    let top = axes.c2p(axes.x.min, axes.y.max);
    let slot = ((right.x - left.x) / n) as f32;
    let bar = bar.clamp(0.0, 1.0);
    BarChart {
        x: left.x as f32 + slot * (1.0 - bar) / 2.0,
        y: left.y as f32,
        bar_width: slot * bar,
        gap: slot * (1.0 - bar),
        max_height: (left.y - top.y) as f32,
        max_value: Some(axes.y.max - base),
        ..chart
    }
}

#[cfg(test)]
mod tests {
    use crate::explain::Range;

    use super::*;

    #[test]
    fn bars_share_the_x_span() {
        let axes = AxesMap {
            x: Range::new(0.0, 4.0, 1.0),
            y: Range::new(0.0, 10.0, 1.0),
            ..AxesMap::default()
        };
        let chart = BarChart {
            bars: vec![("a".into(), 5.0), ("b".into(), 10.0)],
            ..BarChart::default()
        };
        let chart = place(chart, &axes, 0.5);
        // bar 1's center is at x = 3, the middle of the second half
        let center = chart.x + (chart.bar_width + chart.gap) + chart.bar_width / 2.0;
        assert!((f64::from(center) - axes.c2p(3.0, 0.0).x).abs() < 1e-3);
        assert!((f64::from(chart.y) - axes.c2p(0.0, 0.0).y).abs() < 1e-3);
        // a value of 10 reaches the top of the y-axis
        let full = f64::from(chart.max_height);
        assert!((full - (axes.c2p(0.0, 0.0).y - axes.c2p(0.0, 10.0).y)).abs() < 1e-3);
        assert_eq!(chart.max_value, Some(10.0));
    }
}
