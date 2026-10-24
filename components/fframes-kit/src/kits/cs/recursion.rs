//! Recursive calls recorded once: the call stack frame by frame and the tree of calls.

use std::fmt::Display;

use fframes::{Color, Svgr};
use kurbo::{BezPath, Point, RoundedRect, Vec2};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    geometry::Shape,
    layout::tidy_tree,
    motion::{cubic_in_out, progress},
    text::{TextAnchor, line},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::{
    array::CellColors,
    keyed::{StepAt, mix, notes, sample, stroke, tween},
};

/// One call: its label (`fib(3)`), the call that made it, the calls it made in order and
/// what it returned.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Call {
    pub label: String,
    pub parent: Option<usize>,
    pub children: Vec<usize>,
    pub result: Option<String>,
    /// The steps where it is called and where it returns.
    pub called_at: usize,
    pub returned_at: Option<usize>,
}

/// One step of a recursion: the open calls from the outermost to the innermost, the call
/// returning in this step (still on the stack, with its result) and a note.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CallStep {
    pub stack: Vec<usize>,
    pub returning: Option<usize>,
    pub note: String,
}

/// The calls of a recursion and its steps, recorded once with [`CallTrace::recurse`] or a
/// [`CallRecorder`], and sampled by time with [`CallTrace::at`]. Every call and every return
/// takes a step; the tree of calls is laid out once with [`tidy_tree`].
#[derive(Debug, Clone, PartialEq, Default)]
pub struct CallTrace {
    calls: Vec<Call>,
    steps: Vec<CallStep>,
    positions: Vec<(f64, f64)>,
    depth: f64,
}

impl CallTrace {
    /// Starts an empty trace; record the calls by hand with [`CallRecorder::call`] and
    /// [`CallRecorder::ret`].
    pub fn record() -> CallRecorder {
        CallRecorder::default()
    }

    /// Runs `f` on `arg` and records every call it makes through [`Recursion::call`], labelled
    /// `name(arg)`. The calls really run, so keep the input small: a tree of more than about
    /// 30 calls does not fit on the canvas.
    ///
    /// ```
    /// use fframes_kit::prelude::*;
    ///
    /// let fib = CallTrace::recurse("fib", 4_u64, |r, n| if n < 2 { n } else { r.call(n - 1) + r.call(n - 2) });
    /// assert_eq!(fib.calls().len(), 9);
    /// assert_eq!(fib.calls()[0].result.as_deref(), Some("3"));
    /// ```
    pub fn recurse<A, R>(name: &str, arg: A, f: impl Fn(&mut Recursion<'_, A, R>, A) -> R) -> Self
    where
        A: Display,
        R: Display,
    {
        let mut recursion = Recursion {
            recorder: CallRecorder::default(),
            name: name.to_owned(),
            f: &f,
        };
        recursion.call(arg);
        recursion.recorder.finish()
    }

    /// `fib(n)` the naive way: two calls a level.
    pub fn fib(n: u64) -> Self {
        Self::recurse("fib", n, |r, n| {
            if n < 2 {
                n
            } else {
                r.call(n - 1) + r.call(n - 2)
            }
        })
    }

    /// `fact(n)`: one call a level, so the stack grows `n` deep and unwinds.
    pub fn factorial(n: u64) -> Self {
        Self::recurse("fact", n, |r, n| if n < 2 { 1 } else { n * r.call(n - 1) })
    }

    fn new(calls: Vec<Call>, steps: Vec<CallStep>) -> Self {
        let roots: Vec<usize> = (0..calls.len())
            .filter(|&c| calls[c].parent.is_none())
            .collect();
        // several outermost calls hang under a root that is not drawn
        let mut children: Vec<Vec<usize>> = calls.iter().map(|c| c.children.clone()).collect();
        children.push(roots);
        let mut positions = tidy_tree(&children, calls.len());
        positions.pop();
        for p in &mut positions {
            p.1 -= 1.0;
        }
        let depth = positions.iter().map(|p| p.1).fold(0.0, f64::max);
        Self {
            calls,
            steps,
            positions,
            depth,
        }
    }

    pub fn calls(&self) -> &[Call] {
        &self.calls
    }

    pub fn steps(&self) -> &[CallStep] {
        &self.steps
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Where each call sits in the tree: `x` in slots, `y` the depth (0 for the first call).
    pub fn positions(&self) -> &[(f64, f64)] {
        &self.positions
    }

    /// The depth of the deepest call (0 for a single call).
    pub fn depth(&self) -> f64 {
        self.depth
    }

    /// The most calls open at once.
    pub fn max_stack(&self) -> usize {
        self.steps.iter().map(|s| s.stack.len()).max().unwrap_or(0)
    }

    /// The step `seconds` after the start, one step every `step_time` seconds. Before the
    /// start it is step 0, after the end the last step.
    pub fn at(&self, seconds: f32, step_time: f32) -> Option<StepAt<'_, CallStep>> {
        sample(&self.steps, seconds, step_time)
    }
}

/// A recursion being recorded by [`CallTrace::recurse`].
pub struct Recursion<'f, A, R> {
    recorder: CallRecorder,
    name: String,
    #[allow(clippy::type_complexity)]
    f: &'f dyn Fn(&mut Recursion<'f, A, R>, A) -> R,
}

impl<A: Display, R: Display> Recursion<'_, A, R> {
    /// Makes the recursive call `f(arg)`, recording the call and its return.
    pub fn call(&mut self, arg: A) -> R {
        self.recorder.enter(format!("{}({arg})", self.name));
        let f = self.f;
        let result = f(self, arg);
        self.recorder.leave(result.to_string());
        result
    }
}

/// Records calls and returns into a [`CallTrace`]. A call that returns stays on the stack
/// for its step, showing its result, and leaves at the next one.
#[derive(Debug, Clone, Default)]
pub struct CallRecorder {
    calls: Vec<Call>,
    steps: Vec<CallStep>,
    stack: Vec<usize>,
    leaving: bool,
}

impl CallRecorder {
    fn settle(&mut self) {
        if std::mem::take(&mut self.leaving) {
            self.stack.pop();
        }
    }

    fn enter(&mut self, label: String) {
        self.settle();
        let id = self.calls.len();
        let parent = self.stack.last().copied();
        if let Some(p) = parent {
            self.calls[p].children.push(id);
        }
        self.steps.push(CallStep {
            stack: self.stack.iter().copied().chain([id]).collect(),
            returning: None,
            note: match parent {
                Some(p) => format!("{} calls {label}", self.calls[p].label),
                None => format!("call {label}"),
            },
        });
        self.calls.push(Call {
            label,
            parent,
            called_at: self.steps.len() - 1,
            ..Call::default()
        });
        self.stack.push(id);
    }

    fn leave(&mut self, result: String) {
        self.settle();
        let Some(&id) = self.stack.last() else {
            return;
        };
        let note = format!("{} returns {result}", self.calls[id].label);
        self.calls[id].result = Some(result);
        self.calls[id].returned_at = Some(self.steps.len());
        self.steps.push(CallStep {
            stack: self.stack.clone(),
            returning: Some(id),
            note,
        });
        self.leaving = true;
    }

    /// Calls `label` from the innermost open call.
    pub fn call(mut self, label: impl Into<String>) -> Self {
        self.enter(label.into());
        self
    }

    /// Returns `result` from the innermost open call.
    pub fn ret(mut self, result: impl Display) -> Self {
        self.leave(result.to_string());
        self
    }

    /// Replaces the note of the last step.
    pub fn note(mut self, note: impl Into<String>) -> Self {
        if let Some(step) = self.steps.last_mut() {
            step.note = note.into();
        }
        self
    }

    /// The trace, ending with the last return gone from the stack.
    pub fn finish(mut self) -> CallTrace {
        if self.leaving {
            self.settle();
            let note = match self.calls.first() {
                Some(Call {
                    label,
                    result: Some(result),
                    ..
                }) if self.stack.is_empty() => format!("{label} = {result}"),
                _ => self
                    .steps
                    .last()
                    .map(|s| s.note.clone())
                    .unwrap_or_default(),
            };
            self.steps.push(CallStep {
                stack: self.stack.clone(),
                returning: None,
                note,
            });
        }
        CallTrace::new(self.calls, self.steps)
    }
}

/// What a call is at a step: open (and whether it is the innermost), returning, returned
/// or not called yet.
fn fill_of(
    colors: &CellColors,
    trace: &CallTrace,
    index: usize,
    step: &CallStep,
    c: usize,
) -> Color {
    if step.returning == Some(c) {
        colors.found
    } else if step.stack.last() == Some(&c) {
        colors.compare
    } else if trace.calls[c].returned_at.is_some_and(|r| r < index) {
        colors.sorted
    } else {
        colors.base
    }
}

/// A frame or a node: a rounded box with a label.
#[allow(clippy::too_many_arguments)]
fn tag<'a>(
    center: Point,
    width: f64,
    height: f64,
    fill: Color,
    outline: (Color, f32),
    colors: &CellColors,
    style: TextStyle,
    label: String,
    opacity: f32,
) -> Svgr<'a> {
    let rect = RoundedRect::new(
        center.x - width / 2.0,
        center.y - height / 2.0,
        center.x + width / 2.0,
        center.y + height / 2.0,
        height * 0.22,
    );
    let body = Shape {
        path: kurbo::Shape::to_path(&rect, 0.1),
        fill: Some(fill),
        stroke: Some(outline.0),
        stroke_width: outline.1,
        dash: None,
        opacity,
    };
    let text = line(
        center.x as f32,
        (center.y + style.size as f64 * 0.35) as f32,
        style,
        colors.text_on(fill),
        TextAnchor::Middle,
        0.0,
        opacity,
        label.into(),
    );
    fframes::svgr!(<g>{body.draw()}{text}</g>)
}

/// The widest of `labels` in `style`, from the font when it can be resolved.
fn widest(ui: &Ui<'_, '_, '_>, style: TextStyle, labels: impl Iterator<Item = String>) -> f64 {
    labels
        .map(|l| {
            ui.measure(style.font(), &l).map_or_else(
                || l.chars().count() as f64 * style.size as f64 * 0.58,
                f64::from,
            )
        })
        .fold(0.0, f64::max)
}

/// The call stack of a recursion: a frame for each open call, `height` pixels tall and wide
/// enough for the longest label, piled up from the bottom of a column `capacity` frames tall
/// (or as tall as the trace needs), centred on `center` (units of `ui.space`). It plays a
/// [`CallTrace`], one step every `step_time` seconds from `delay`: a call drops a frame on
/// top, the innermost frame lights up, a returning frame shows its result in `colors.found`
/// and then rises out. The whole figure fades with `enter` and `exit`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct CallStack<'a> {
    pub trace: Option<&'a CallTrace>,
    pub delay: f32,
    pub step_time: f32,
    pub center: (f64, f64),
    pub height: f32,
    pub gap: f32,
    pub capacity: usize,
    pub notes: bool,
    pub colors: CellColors,
    pub frame_color: Color,
    pub value_style: TextStyle,
    pub small_style: TextStyle,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for CallStack<'_> {
    fn themed(t: &Theme) -> Self {
        let mut value_style = t.typography.body;
        value_style.weight = 600;
        Self {
            trace: None,
            delay: 0.0,
            step_time: 0.6,
            center: (0.0, 0.0),
            height: 64.0,
            gap: 10.0,
            capacity: 0,
            notes: true,
            colors: CellColors::themed(t),
            frame_color: t.colors.muted,
            value_style,
            small_style: t.typography.caption,
            enter: None,
            exit: None,
        }
    }
}

impl Default for CallStack<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for CallStack<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let Some(trace) = self.trace else {
            return Svgr::empty();
        };
        let Some(at) = trace.at(ui.seconds() - self.delay, self.step_time) else {
            return Svgr::empty();
        };
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let label = |c: usize, returning: bool| {
            let call = &trace.calls[c];
            match (&call.result, returning) {
                (Some(result), true) => format!("{} = {result}", call.label),
                _ => call.label.clone(),
            }
        };
        let w = widest(
            ui,
            self.value_style,
            (0..trace.calls.len()).map(|c| label(c, true)),
        ) + 48.0;
        let (h, gap) = (f64::from(self.height), f64::from(self.gap));
        let pitch = h + gap;
        let capacity = self.capacity.max(trace.max_stack()).max(1);
        let bottom = middle.y + capacity as f64 * pitch / 2.0;
        let place = |step: &CallStep| -> Vec<(u32, Point)> {
            step.stack
                .iter()
                .enumerate()
                .map(|(k, &c)| {
                    (
                        c as u32,
                        Point::new(middle.x, bottom - gap - h / 2.0 - k as f64 * pitch),
                    )
                })
                .collect()
        };
        let move_time = self.step_time * 0.7;
        let blend = (self.step_time * 0.3).min(0.2);
        let p = f64::from(cubic_in_out(progress(at.into, 0.0, move_time)));
        let q = progress(at.into, 0.0, blend);
        let rise = Vec2::new(0.0, -1.5 * pitch);
        let tweens = tween(&place(at.previous), &place(at.step), p, rise, rise, 0.0);

        let (left, right) = (middle.x - w / 2.0 - gap, middle.x + w / 2.0 + gap);
        let top = bottom - capacity as f64 * pitch - gap;
        let mut frame = BezPath::new();
        frame.move_to((left, top));
        frame.line_to((left, bottom));
        frame.line_to((right, bottom));
        frame.line_to((right, top));
        let frame = stroke(frame, self.frame_color, 3.0, 1.0).draw();

        let previous = at.index.saturating_sub(1);
        let frames: Vec<Svgr<'a>> = tweens
            .iter()
            .map(|t| {
                let c = t.id as usize;
                let now = fill_of(&self.colors, trace, at.index, at.step, c);
                let fill = match (t.from, t.to) {
                    (Some(_), Some(_)) => mix(
                        fill_of(&self.colors, trace, previous, at.previous, c),
                        now,
                        q,
                    ),
                    (Some(_), None) => fill_of(&self.colors, trace, previous, at.previous, c),
                    _ => now,
                };
                let returning = at.step.returning == Some(c)
                    || (t.to.is_none() && at.previous.returning == Some(c));
                tag(
                    t.at,
                    w,
                    h,
                    fill,
                    (self.colors.text.with_alpha(64), 1.5),
                    &self.colors,
                    self.value_style,
                    label(c, returning),
                    t.opacity as f32,
                )
            })
            .collect();

        let mut parts = vec![frame, frames.into()];
        if self.notes {
            parts.push(notes(
                &at.previous.note,
                &at.step.note,
                at.into,
                middle.x as f32,
                (bottom + 24.0 + self.small_style.size as f64 * 0.8) as f32,
                self.small_style,
                self.colors.text,
                blend,
            ));
        }
        fade(ui, parts.into(), self.enter.as_ref(), self.exit.as_ref())
    }
}

/// The tree of a recursion's calls: a node for each call, wide enough for the longest label
/// and `height` pixels tall, `spacing` pixels between slots and `level` pixels between
/// depths, centred on `center` (units of `ui.space`). It plays a [`CallTrace`], one step
/// every `step_time` seconds from `delay`: each call's node drops in under its caller, the
/// open calls are outlined in `path_color` with the innermost lit, a returning call shows its
/// result beside it in `colors.found`, and finished calls keep their result in
/// `colors.sorted`. The whole figure fades with `enter` and `exit`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct RecursionTree<'a> {
    pub trace: Option<&'a CallTrace>,
    pub delay: f32,
    pub step_time: f32,
    pub center: (f64, f64),
    pub spacing: f32,
    pub level: f32,
    pub height: f32,
    pub notes: bool,
    pub colors: CellColors,
    pub edge_color: Color,
    pub path_color: Color,
    pub value_style: TextStyle,
    pub small_style: TextStyle,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for RecursionTree<'_> {
    fn themed(t: &Theme) -> Self {
        let mut value_style = t.typography.body;
        value_style.weight = 600;
        Self {
            trace: None,
            delay: 0.0,
            step_time: 0.6,
            center: (0.0, 0.0),
            spacing: 190.0,
            level: 130.0,
            height: 60.0,
            notes: true,
            colors: CellColors::themed(t),
            edge_color: t.colors.muted,
            path_color: t.colors.accent,
            value_style,
            small_style: t.typography.caption,
            enter: None,
            exit: None,
        }
    }
}

impl Default for RecursionTree<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for RecursionTree<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let Some(trace) = self.trace else {
            return Svgr::empty();
        };
        let Some(at) = trace.at(ui.seconds() - self.delay, self.step_time) else {
            return Svgr::empty();
        };
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let (spacing, level, h) = (
            f64::from(self.spacing),
            f64::from(self.level),
            f64::from(self.height),
        );
        let w = widest(
            ui,
            self.value_style,
            trace.calls.iter().map(|c| c.label.clone()),
        ) + 28.0;
        let (lo, hi) = trace
            .positions
            .iter()
            .fold((f64::MAX, f64::MIN), |(lo, hi), p| {
                (lo.min(p.0), hi.max(p.0))
            });
        let mid = f64::midpoint(lo, hi);
        let top = middle.y - trace.depth * level / 2.0;
        let point = |c: usize| {
            let (x, y) = trace.positions[c];
            Point::new(middle.x + (x - mid) * spacing, top + y * level)
        };
        let blend = (self.step_time * 0.3).min(0.2);
        let q = progress(at.into, 0.0, blend);
        let drop = f64::from(cubic_in_out(progress(at.into, 0.0, self.step_time * 0.6)));
        let previous = at.index.saturating_sub(1);
        // how far a call is in: 0 before it is called, 1 once it is
        let shown = |c: usize| {
            let called = trace.calls[c].called_at;
            if called < at.index || (called == 0 && at.index == 0) {
                1.0
            } else if called == at.index {
                drop
            } else {
                0.0
            }
        };
        let open = |step: &CallStep, c: usize| if step.stack.contains(&c) { 1.0 } else { 0.0 };

        let mut edges = Vec::new();
        let mut nodes = Vec::new();
        let mut results = Vec::new();
        for (c, call) in trace.calls.iter().enumerate() {
            let s = shown(c);
            if s <= 0.0 {
                continue;
            }
            let lift = Vec2::new(0.0, -level * 0.3 * (1.0 - s));
            let at_c = point(c) + lift;
            if let Some(parent) = call.parent {
                let from = point(parent);
                let mut path = BezPath::new();
                path.move_to(from + Vec2::new(0.0, h / 2.0));
                path.line_to(at_c - Vec2::new(0.0, h / 2.0));
                let lit = open(at.previous, c) + (open(at.step, c) - open(at.previous, c)) * q;
                edges.push(
                    stroke(
                        path,
                        mix(self.edge_color, self.path_color, lit),
                        2.5 + 1.5 * lit,
                        s as f32,
                    )
                    .draw(),
                );
            }
            let fill = mix(
                fill_of(&self.colors, trace, previous, at.previous, c),
                fill_of(&self.colors, trace, at.index, at.step, c),
                q,
            );
            let outline = open(at.previous, c) + (open(at.step, c) - open(at.previous, c)) * q;
            nodes.push(tag(
                at_c,
                w,
                h,
                fill,
                (
                    mix(self.colors.text.with_alpha(64), self.path_color, outline),
                    1.5 + 2.5 * outline,
                ),
                &self.colors,
                self.value_style,
                call.label.clone(),
                s as f32,
            ));
            // the result appears beside the node as the call returns
            let returned = |index: usize| call.returned_at.is_some_and(|r| r <= index);
            let r = match (returned(previous), returned(at.index)) {
                (_, false) => 0.0,
                (false, true) if at.index > 0 => q,
                _ => 1.0,
            };
            if let (Some(result), true) = (&call.result, r > 0.0) {
                let color = if at.step.returning == Some(c) {
                    self.colors.found
                } else {
                    self.colors.sorted
                };
                results.push(line(
                    (at_c.x + w / 2.0 + 8.0) as f32,
                    (at_c.y + self.small_style.size as f64 * 0.35) as f32,
                    self.small_style,
                    color,
                    TextAnchor::Start,
                    0.0,
                    r,
                    format!("= {result}").into(),
                ));
            }
        }

        let mut parts = vec![edges.into(), nodes.into(), results.into()];
        if self.notes {
            let y = top + trace.depth * level + h / 2.0 + 40.0;
            parts.push(notes(
                &at.previous.note,
                &at.step.note,
                at.into,
                middle.x as f32,
                (y + self.small_style.size as f64 * 0.8) as f32,
                self.small_style,
                self.colors.text,
                blend,
            ));
        }
        fade(ui, parts.into(), self.enter.as_ref(), self.exit.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fib_calls_form_its_tree() {
        let fib = CallTrace::fib(4);
        // fib(4) makes 9 calls, each called and returned in a step, and one step at the end
        assert_eq!(fib.calls().len(), 9);
        assert_eq!(fib.len(), 19);
        assert_eq!(fib.calls()[0].result.as_deref(), Some("3"));
        assert_eq!(fib.calls()[0].children.len(), 2);
        assert_eq!(fib.depth(), 3.0);
        assert_eq!(fib.max_stack(), 4);
        assert_eq!(fib.steps()[1].note, "fib(4) calls fib(3)");
        assert_eq!(fib.steps().last().unwrap().note, "fib(4) = 3");
        assert_eq!(fib.steps().last().unwrap().stack.len(), 0);
        // every call sits a level under its caller
        for (c, call) in fib.calls().iter().enumerate() {
            let y = call.parent.map_or(0.0, |p| fib.positions()[p].1 + 1.0);
            assert_eq!(fib.positions()[c].1, y);
        }
    }

    #[test]
    fn a_returning_call_leaves_at_the_next_step() {
        let fact = CallTrace::factorial(3);
        let returns: Vec<usize> = fact.calls().iter().filter_map(|c| c.returned_at).collect();
        // the innermost call returns first
        assert_eq!(returns, [5, 4, 3]);
        let step = &fact.steps()[3];
        assert_eq!(step.returning, Some(2));
        assert_eq!(step.stack, [0, 1, 2]);
        assert_eq!(step.note, "fact(1) returns 1");
        assert_eq!(fact.steps()[4].stack, [0, 1]);
        assert_eq!(fact.calls()[0].result.as_deref(), Some("6"));
    }

    #[test]
    fn recorded_by_hand() {
        let trace = CallTrace::record()
            .call("sum([1, 2])")
            .call("sum([2])")
            .ret(2)
            .ret(3)
            .note("done")
            .finish();
        assert_eq!(trace.calls().len(), 2);
        assert_eq!(trace.calls()[1].parent, Some(0));
        assert_eq!(trace.len(), 5);
        assert_eq!(trace.steps()[3].note, "done");
        assert_eq!(trace.steps()[4].note, "sum([1, 2]) = 3");
        // a return with nothing open is ignored
        let empty = CallTrace::record().ret(1).finish();
        assert!(empty.is_empty());
    }
}
