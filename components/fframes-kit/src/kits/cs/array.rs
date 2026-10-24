use std::f64::consts::PI;

use fframes::{Color, Svgr, animation::Animatable};
use kurbo::{BezPath, RoundedRect};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    geometry::Shape,
    motion::{cubic_in_out, progress},
    text::{TextAnchor, line},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::trace::{ArrayStep, ArrayTrace, TraceAt};

/// The colours of the cells in each state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CellColors {
    pub base: Color,
    pub compare: Color,
    pub moved: Color,
    pub sorted: Color,
    pub found: Color,
    pub text: Color,
    /// The text on light fills (the highlight, the found cell).
    pub ink: Color,
}

impl CellColors {
    pub fn themed(t: &Theme) -> Self {
        Self {
            base: t.colors.surface,
            compare: t.colors.highlight,
            moved: t.colors.accent,
            sorted: Color::hex("#2f9e44"),
            found: Color::hex("#69db7c"),
            text: t.colors.text,
            ink: t.colors.background,
        }
    }

    /// `text` on a dark fill, `ink` on a light one.
    pub fn text_on(&self, fill: Color) -> Color {
        let light =
            0.299 * f32::from(fill.r) + 0.587 * f32::from(fill.g) + 0.114 * f32::from(fill.b);
        if light > 150.0 { self.ink } else { self.text }
    }

    /// The colour of cell `k` in `step`, and whether it is in play.
    pub(super) fn of(&self, step: &ArrayStep, k: usize) -> (Color, bool) {
        let in_play = step.range.is_none_or(|(lo, hi)| (lo..=hi).contains(&k));
        let color = if step.found == Some(k) {
            self.found
        } else if step.compare.contains(&k) {
            if step.moved.is_some() {
                self.moved
            } else {
                self.compare
            }
        } else if step.sorted.contains(&k) {
            self.sorted
        } else {
            self.base
        };
        (color, in_play)
    }

    /// The colour and opacity of cell `k`, blended from the previous step over `blend`
    /// seconds so a change of state never jumps in one frame.
    pub(super) fn blended(&self, at: &TraceAt<'_>, k: usize, blend: f32, dim: f32) -> (Color, f32) {
        let p = progress(at.into, 0.0, blend);
        let (from, was) = self.of(at.previous, k);
        let (to, is) = self.of(at.step, k);
        let alpha = |inside: bool| if inside { 1.0 } else { dim };
        let opacity = alpha(was) + (alpha(is) - alpha(was)) * p;
        (Animatable::apply_progress(&from, &to, p), opacity)
    }
}

/// How far each cell has travelled in a step that moves cells: its slot as a fraction
/// between where it was and where it ends, and how far it is lifted on the way, up to 1 for a
/// cell moving right and down to -1 for one moving left, so two swapped cells pass each other.
pub(super) fn travel(at: &TraceAt<'_>, k: usize, move_time: f32) -> (f64, f64) {
    let Some(moved) = &at.step.moved else {
        return (k as f64, 0.0);
    };
    let from = moved.get(k).copied().unwrap_or(k);
    let p = f64::from(cubic_in_out(progress(at.into, 0.0, move_time)));
    let slot = from as f64 + (k as f64 - from as f64) * p;
    let lift = if from == k || p >= 1.0 {
        0.0
    } else {
        (PI * p).sin() * (k as f64 - from as f64).signum()
    };
    (slot, lift)
}

/// The note of the step, cross-faded from the previous one over `blend` seconds.
pub(super) fn notes<'a>(
    at: &TraceAt<'_>,
    x: f32,
    y: f32,
    style: TextStyle,
    fill: Color,
    blend: f32,
) -> Svgr<'a> {
    let p = progress(at.into, 0.0, blend);
    let (old, new) = (&at.previous.note, &at.step.note);
    // the old note fades out in the first half, the new one in during the second
    let (text, opacity) = if old == new {
        (new, 1.0)
    } else if p < 0.5 {
        (old, 1.0 - 2.0 * p)
    } else {
        (new, 2.0 * p - 1.0)
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
        text.into(),
    )
}

/// The markers under the cells: a small arrow up to the cell and its name, moving with the
/// pointer from step to step, fading in and out when it appears or goes. `slot_x` gives the
/// centre of a slot, `top` the arrow tip.
pub(super) fn pointers<'a>(
    at: &TraceAt<'_>,
    slot_x: impl Fn(f64) -> f64,
    top: f64,
    style: TextStyle,
    color: Color,
    move_time: f32,
) -> Svgr<'a> {
    let p = f64::from(cubic_in_out(progress(at.into, 0.0, move_time)));
    let find = |step: &ArrayStep, name: &str| {
        step.pointers
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, i)| *i)
    };
    // pointers sharing a cell stack downwards in the order the step lists them
    let row_of = |step: &ArrayStep, name: &str| {
        let i = find(step, name)?;
        Some(
            step.pointers
                .iter()
                .take_while(|(n, _)| n != name)
                .filter(|(_, j)| *j == i)
                .count() as f64,
        )
    };
    // (name, slot, opacity, row), each moving from the previous step to this one
    let mut marks: Vec<(String, f64, f64, f64)> = Vec::new();
    for (name, i) in &at.step.pointers {
        let row = row_of(at.step, name).unwrap_or(0.0);
        let (slot, opacity, row) = match (find(at.previous, name), row_of(at.previous, name)) {
            (Some(was), Some(was_row)) => (
                was as f64 + (*i as f64 - was as f64) * p,
                1.0,
                was_row + (row - was_row) * p,
            ),
            _ => (*i as f64, p, row),
        };
        marks.push((name.clone(), slot, opacity, row));
    }
    for (name, was) in &at.previous.pointers {
        if find(at.step, name).is_none() && p < 1.0 {
            let row = row_of(at.previous, name).unwrap_or(0.0);
            marks.push((name.clone(), *was as f64, 1.0 - p, row));
        }
    }
    let row_height = style.size as f64 * 1.15;
    let mut drawn = Vec::new();
    for (name, slot, opacity, below) in &marks {
        let x = slot_x(*slot);
        let mut arrow = BezPath::new();
        arrow.move_to((x, top));
        arrow.line_to((x - 9.0, top + 14.0));
        arrow.line_to((x + 9.0, top + 14.0));
        arrow.close_path();
        let shape = Shape {
            path: arrow,
            fill: Some(color),
            stroke: None,
            stroke_width: 0.0,
            dash: None,
            opacity: *opacity as f32,
        };
        drawn.push(shape.draw());
        let baseline = top + 18.0 + style.size as f64 * 0.8 + below * row_height;
        drawn.push(line(
            x as f32,
            baseline as f32,
            style,
            color,
            TextAnchor::Middle,
            0.0,
            *opacity as f32,
            name.as_str().into(),
        ));
    }
    drawn.into()
}

/// An array as a row of cells with its values, placed around `center` (units of
/// `ui.space`), each `cell` pixels wide and `gap` apart, with the index under each cell.
/// It plays an [`ArrayTrace`], one step every `step_time` seconds from `delay`: compared
/// cells light up, moved cells travel to their new place on an arc, cells in their final
/// place turn `colors.sorted`, cells outside the step's range dim and pointers move under
/// the cells. With `notes` the step's note is written under the pointers. The whole figure
/// fades with `enter` and `exit`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct ArrayView<'a> {
    pub trace: Option<&'a ArrayTrace>,
    pub delay: f32,
    pub step_time: f32,
    pub center: (f64, f64),
    pub cell: f32,
    pub gap: f32,
    pub indices: bool,
    pub notes: bool,
    pub colors: CellColors,
    pub pointer_color: Color,
    pub index_color: Color,
    pub value_style: TextStyle,
    pub small_style: TextStyle,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for ArrayView<'_> {
    fn themed(t: &Theme) -> Self {
        let mut value_style = t.typography.body;
        value_style.weight = 600;
        Self {
            trace: None,
            delay: 0.0,
            step_time: 0.6,
            center: (0.0, 0.0),
            cell: 96.0,
            gap: 12.0,
            indices: true,
            notes: true,
            colors: CellColors::themed(t),
            pointer_color: t.colors.highlight,
            index_color: t.colors.muted,
            value_style,
            small_style: t.typography.caption,
            enter: None,
            exit: None,
        }
    }
}

impl Default for ArrayView<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl ArrayView<'_> {
    /// The pixel centre of slot `slot` (fractional while cells move) for `n` cells.
    pub fn slot_x(&self, middle: f64, n: usize, slot: f64) -> f64 {
        let pitch = f64::from(self.cell + self.gap);
        middle + (slot - (n as f64 - 1.0) / 2.0) * pitch
    }
}

impl<'a> Component<'a> for ArrayView<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let Some(at) = self
            .trace
            .and_then(|t| t.at(ui.seconds() - self.delay, self.step_time))
        else {
            return Svgr::empty();
        };
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let n = at.step.values.len();
        let size = f64::from(self.cell);
        let move_time = self.step_time * 0.7;
        let blend = (self.step_time * 0.3).min(0.2);
        // each cell with its value, the moving ones drawn last so they pass over the rest
        let mut cells: Vec<(bool, Svgr<'a>)> = Vec::new();
        for (k, value) in at.step.values.iter().enumerate() {
            let (slot, lift) = travel(&at, k, move_time);
            let x = self.slot_x(middle.x, n, slot);
            let y = middle.y - lift * size * 0.8;
            let (fill, opacity) = self.colors.blended(&at, k, blend, 0.3);
            let rect = RoundedRect::new(
                x - size / 2.0,
                y - size / 2.0,
                x + size / 2.0,
                y + size / 2.0,
                size * 0.14,
            );
            let cell = Shape {
                path: kurbo::Shape::to_path(&rect, 0.1),
                fill: Some(fill),
                stroke: Some(self.colors.text.with_alpha(64)),
                stroke_width: 1.5,
                dash: None,
                opacity,
            };
            let baseline = y + (self.value_style.size as f64) * 0.35;
            let text = line(
                x as f32,
                baseline as f32,
                self.value_style,
                self.colors.text_on(fill),
                TextAnchor::Middle,
                0.0,
                opacity,
                value.to_string().into(),
            );
            cells.push((lift != 0.0, fframes::svgr!(<g>{cell.draw()}{text}</g>)));
        }
        cells.sort_by_key(|(moving, _)| *moving);
        let cells: Vec<Svgr<'a>> = cells.into_iter().map(|(_, cell)| cell).collect();
        let mut parts = Vec::new();
        let bottom = middle.y + size / 2.0;
        let mut below = bottom + 10.0;
        if self.indices {
            let baseline = below + (self.small_style.size as f64) * 0.8;
            let indices: Vec<Svgr<'a>> = (0..n)
                .map(|k| {
                    line(
                        self.slot_x(middle.x, n, k as f64) as f32,
                        baseline as f32,
                        self.small_style,
                        self.index_color,
                        TextAnchor::Middle,
                        0.0,
                        1.0,
                        k.to_string().into(),
                    )
                })
                .collect();
            parts.push(indices.into());
            below += (self.small_style.size as f64) * 1.2;
        }
        parts.push(cells.into());
        parts.push(pointers(
            &at,
            |slot| self.slot_x(middle.x, n, slot),
            below + 4.0,
            self.small_style,
            self.pointer_color,
            move_time,
        ));
        if self.notes {
            let rows = 1.0
                + at.step
                    .pointers
                    .len()
                    .max(at.previous.pointers.len())
                    .min(3) as f64;
            let y = below + 30.0 + rows * (self.small_style.size as f64) * 1.2;
            parts.push(notes(
                &at,
                middle.x as f32,
                y as f32,
                self.small_style,
                self.colors.text,
                blend,
            ));
        }
        fade(ui, parts.into(), self.enter.as_ref(), self.exit.as_ref())
    }
}

/// [`ArrayView`] playing [`ArrayTrace::binary_search`], with what it looks for written
/// above the cells (`"looking for 23"`). The search's range dims the cells it has ruled
/// out, and the `lo`, `mid` and `hi` pointers move under the cells.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct BinarySearch<'a> {
    pub view: ArrayView<'a>,
    pub target: i64,
    pub caption: bool,
    pub caption_color: Color,
}

impl Themed for BinarySearch<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            view: ArrayView {
                step_time: 0.9,
                ..ArrayView::themed(t)
            },
            target: 0,
            caption: true,
            caption_color: t.colors.text,
        }
    }
}

impl Default for BinarySearch<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for BinarySearch<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let middle = ui.space.to_px(self.view.center.0, self.view.center.1);
        let top = middle.y - f64::from(self.view.cell) / 2.0 - 40.0;
        let style = self.view.value_style;
        let caption = if self.caption && self.view.trace.is_some() {
            let text = line(
                middle.x as f32,
                top as f32,
                style,
                self.caption_color,
                TextAnchor::Middle,
                0.0,
                1.0,
                format!("looking for {}", self.target).into(),
            );
            fade(ui, text, self.view.enter.as_ref(), self.view.exit.as_ref())
        } else {
            Svgr::empty()
        };
        let view = ui.show(self.view);
        fframes::svgr!(<g>{caption}{view}</g>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moved_cells_travel_on_an_arc() {
        let trace = ArrayTrace::bubble(&[2, 1]);
        // step 1 compares, step 2 (from 1.2 s) swaps
        let at = trace.at(1.2 + 0.21, 0.6).unwrap();
        assert!(at.step.moved.is_some());
        let (slot, lift) = travel(&at, 0, 0.42);
        // halfway: between the two slots, the cell moving left at the bottom of its arc and
        // the one moving right at the top
        assert!(
            (slot - 0.5).abs() < 1e-4 && (lift + 1.0).abs() < 1e-4,
            "{slot} {lift}"
        );
        assert!((travel(&at, 1, 0.42).1 - 1.0).abs() < 1e-4);
        let done = trace.at(1.2 + 0.5, 0.6).unwrap();
        assert_eq!(travel(&done, 0, 0.42), (0.0, 0.0));
    }

    #[test]
    fn cell_states_blend() {
        let colors = CellColors::themed(&Theme::default());
        let trace = ArrayTrace::bubble(&[1, 2]);
        // step 1 compares cells 0 and 1: right at its start they still look like step 0
        let start = trace.at(0.6, 0.6).unwrap();
        assert_eq!(colors.blended(&start, 0, 0.2, 0.3).0, colors.base);
        let later = trace.at(0.9, 0.6).unwrap();
        assert_eq!(colors.blended(&later, 0, 0.2, 0.3).0, colors.compare);
    }
}
