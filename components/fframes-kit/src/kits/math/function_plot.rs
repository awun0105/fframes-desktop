use std::borrow::Cow;

use fframes::{Color, Svgr};

use crate::{
    anim::{FadeIn, FadeOut, ValueTracker, fade},
    explain::{Axes, FunctionGraph},
    label::Label,
    layout::Anchor,
    shapes::Dot,
    theme::Theme,
    ui::{Component, Themed, Ui},
};

/// Axes with a graph, its label and a tracer: a dot that follows a
/// [`ValueTracker`] along the graph, a dashed line down to the x-axis and the point's
/// coordinates (Manim's plot with an `always_redraw` dot). Build `graph` with
/// [`Axes::plot`] on the same axes.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct FunctionPlot<'a> {
    pub axes: Axes<'a>,
    pub graph: FunctionGraph,
    /// Written next to the graph at `label_at`, empty for none.
    pub label: Cow<'a, str>,
    pub label_at: f64,
    pub label_direction: Anchor,
    pub label_enter: Option<FadeIn>,
    /// `x` of the tracer, `None` for no tracer.
    pub tracer: Option<ValueTracker<'a>>,
    /// Decimals of the coordinates written at the tracer.
    pub decimals: usize,
    pub tracer_color: Color,
    pub tracer_enter: Option<FadeIn>,
    pub tracer_exit: Option<FadeOut>,
}

impl Themed for FunctionPlot<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            axes: Axes::themed(t),
            graph: FunctionGraph::themed(t),
            label: Cow::Borrowed(""),
            label_at: 0.0,
            label_direction: Anchor::TopRight,
            label_enter: None,
            tracer: None,
            decimals: 2,
            tracer_color: t.colors.highlight,
            tracer_enter: None,
            tracer_exit: None,
        }
    }
}

impl Default for FunctionPlot<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// The coordinates written at the tracer.
fn coordinates(x: f64, y: f64, decimals: usize) -> String {
    format!("({x:.decimals$}, {y:.decimals$})")
}

impl<'a> Component<'a> for FunctionPlot<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let label = if self.label.is_empty() {
            Svgr::empty()
        } else {
            let label = Label {
                enter: self.label_enter,
                ..self.axes.graph_label(
                    ui,
                    &self.graph,
                    self.label.clone(),
                    self.label_at,
                    self.label_direction,
                )
            };
            ui.show(label)
        };
        let tracer = match &self.tracer {
            Some(x) => {
                let x = f64::from(ui.sample(x));
                let y = (self.graph.f)(x);
                if y.is_finite() {
                    let (px, py) = self.graph.point(x);
                    let drop = ui.show(self.axes.vertical_line(ui, x, y));
                    let dot = ui.show(Dot {
                        cx: px,
                        cy: py,
                        radius: 12.0,
                        fill: Some(self.tracer_color),
                        ..ui.defaults()
                    });
                    let reading = ui.show(Label {
                        text: coordinates(x, y, self.decimals).into(),
                        x: px,
                        y: py,
                        direction: Anchor::TopLeft,
                        buff: ui.theme.spacing.md,
                        style: ui.theme.typography.caption,
                        ..ui.defaults()
                    });
                    fade(
                        ui,
                        fframes::svgr!(<g>{drop}{dot}{reading}</g>),
                        self.tracer_enter.as_ref(),
                        self.tracer_exit.as_ref(),
                    )
                } else {
                    Svgr::empty()
                }
            }
            None => Svgr::empty(),
        };
        let axes = ui.show(self.axes);
        let graph = ui.show(self.graph);
        fframes::svgr!(<g>{axes}{graph}{label}{tracer}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coordinates_use_the_decimals() {
        assert_eq!(coordinates(1.0, 0.5, 2), "(1.00, 0.50)");
        assert_eq!(coordinates(-1.24, 2.0, 1), "(-1.2, 2.0)");
    }
}
