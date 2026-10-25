//! Linked lists, stacks and queues: items that keep their identity from step to step.

use std::fmt::Display;

use fframes::{Color, Svgr};
use kurbo::{BezPath, Point, RoundedRect, Vec2};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    geometry::Shape,
    motion::{cubic_in_out, progress},
    text::{TextAnchor, line},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::{
    array::{CellColors, index_row},
    keyed::{StepAt, Tween, find, markers, mix, notes, sample, stroke, tip, tween, tween_links},
};

/// One step of a list, a stack or a queue.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ListStep {
    /// The items in order (a stack's top and a queue's back last), each with an id that stays
    /// with it from step to step.
    pub items: Vec<(u32, String)>,
    /// The links of a linked list, from id to id. `None` links each item to the next.
    pub links: Option<Vec<(u32, u32)>>,
    /// The items this step looks at.
    pub highlight: Vec<u32>,
    /// An item that was searched for and found.
    pub found: Option<u32>,
    /// Named markers at items: `("cur", 3)`.
    pub pointers: Vec<(String, u32)>,
    /// What happens in this step, in a few words.
    pub note: String,
}

impl ListStep {
    /// The links of the step: `links`, or each item to the next.
    pub fn links(&self) -> Vec<(u32, u32)> {
        self.links.clone().unwrap_or_else(|| {
            self.items
                .windows(2)
                .map(|pair| (pair[0].0, pair[1].0))
                .collect()
        })
    }

    fn plain(&self) -> bool {
        self.highlight.is_empty() && self.found.is_none() && self.pointers.is_empty()
    }
}

/// The steps of a linked list, a stack or a queue, recorded once (in a constructor or a
/// `OnceLock`) and sampled by time with [`ListTrace::at`]. [`ListTrace::record`] starts a
/// [`ListRecorder`] from values; [`ListTrace::new`] takes steps of your own.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ListTrace {
    steps: Vec<ListStep>,
}

impl ListTrace {
    pub fn new(steps: Vec<ListStep>) -> Self {
        Self { steps }
    }

    /// Records operations on a list holding `values`, front (or bottom) first.
    pub fn record<T: Display>(values: &[T]) -> ListRecorder {
        let items: Vec<(u32, String)> = values
            .iter()
            .enumerate()
            .map(|(i, v)| (i as u32, v.to_string()))
            .collect();
        ListRecorder {
            next_id: items.len() as u32,
            steps: vec![ListStep {
                items: items.clone(),
                ..ListStep::default()
            }],
            items,
        }
    }

    pub fn steps(&self) -> &[ListStep] {
        &self.steps
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// The items after the last step, in order.
    pub fn result(&self) -> Vec<&str> {
        self.steps.last().map_or_else(Vec::new, |s| {
            s.items.iter().map(|(_, label)| label.as_str()).collect()
        })
    }

    /// The step `seconds` after the start, one step every `step_time` seconds. Before the
    /// start it is step 0, after the end the last step.
    pub fn at(&self, seconds: f32, step_time: f32) -> Option<StepAt<'_, ListStep>> {
        sample(&self.steps, seconds, step_time)
    }
}

/// Records operations step by step, from [`ListTrace::record`]. A stack pushes and pops at
/// the back (its top), a queue enqueues at the back and dequeues at the front, and a linked
/// list walks a `cur` pointer from the head to where it inserts, removes or finds. Every
/// operation adds the steps that show it and a note; [`ListRecorder::finish`] gives the trace.
#[derive(Debug, Clone)]
pub struct ListRecorder {
    steps: Vec<ListStep>,
    items: Vec<(u32, String)>,
    next_id: u32,
}

impl ListRecorder {
    fn step(&self, note: String) -> ListStep {
        ListStep {
            items: self.items.clone(),
            note,
            ..ListStep::default()
        }
    }

    fn put(&mut self, k: usize, label: String, note: String, pointers: Vec<(String, u32)>) {
        let id = self.next_id;
        self.next_id += 1;
        self.items.insert(k.min(self.items.len()), (id, label));
        let step = ListStep {
            highlight: vec![id],
            pointers,
            ..self.step(note)
        };
        self.steps.push(step);
    }

    /// Lights up item `k` with `note`, then takes it out.
    fn take(&mut self, k: usize, note: String, pointers: Vec<(String, u32)>) {
        let id = self.items[k].0;
        let step = ListStep {
            highlight: vec![id],
            pointers: pointers.clone(),
            ..self.step(note.clone())
        };
        self.steps.push(step);
        self.items.remove(k);
        let step = ListStep {
            pointers,
            ..self.step(note)
        };
        self.steps.push(step);
    }

    /// `cur` walks from the head over the first `count` items.
    fn walk(&mut self, count: usize, note: &str) {
        for k in 0..count.min(self.items.len()) {
            let id = self.items[k].0;
            let step = ListStep {
                highlight: vec![id],
                pointers: vec![("cur".into(), id)],
                ..self.step(note.to_owned())
            };
            self.steps.push(step);
        }
    }

    fn cur(&self, k: Option<usize>) -> Vec<(String, u32)> {
        k.map(|k| ("cur".into(), self.items[k].0))
            .into_iter()
            .collect()
    }

    /// Pushes `value` on top (the back).
    pub fn push(mut self, value: impl Display) -> Self {
        let label = value.to_string();
        self.put(
            self.items.len(),
            label.clone(),
            format!("push {label}"),
            vec![],
        );
        self
    }

    /// Takes the top (the back) off.
    pub fn pop(mut self) -> Self {
        match self.items.last() {
            Some((_, label)) => {
                let note = format!("pop {label}");
                self.take(self.items.len() - 1, note, vec![]);
            }
            None => self.steps.push(self.step("nothing to pop".into())),
        }
        self
    }

    /// Lights up the top (the back).
    pub fn peek(mut self) -> Self {
        let step = match self.items.last() {
            Some((id, label)) => ListStep {
                highlight: vec![*id],
                ..self.step(format!("the top is {label}"))
            },
            None => self.step("it is empty".into()),
        };
        self.steps.push(step);
        self
    }

    /// Adds `value` at the back.
    pub fn enqueue(mut self, value: impl Display) -> Self {
        let label = value.to_string();
        self.put(
            self.items.len(),
            label.clone(),
            format!("enqueue {label}"),
            vec![],
        );
        self
    }

    /// Takes the front off.
    pub fn dequeue(mut self) -> Self {
        match self.items.first() {
            Some((_, label)) => {
                let note = format!("dequeue {label}");
                self.take(0, note, vec![]);
            }
            None => self.steps.push(self.step("nothing to dequeue".into())),
        }
        self
    }

    /// Lights up the front.
    pub fn peek_front(mut self) -> Self {
        let step = match self.items.first() {
            Some((id, label)) => ListStep {
                highlight: vec![*id],
                ..self.step(format!("the front is {label}"))
            },
            None => self.step("it is empty".into()),
        };
        self.steps.push(step);
        self
    }

    /// Inserts `value` at `index` of a linked list: `cur` walks to the node before it, then
    /// the new node goes in and the links change.
    pub fn insert(mut self, index: usize, value: impl Display) -> Self {
        let index = index.min(self.items.len());
        let label = value.to_string();
        let note = match index.checked_sub(1).map(|k| self.items[k].1.clone()) {
            Some(before) => {
                self.walk(index, &format!("walk to {before}"));
                format!("insert {label} after {before}")
            }
            None => format!("insert {label} at the head"),
        };
        let pointers = self.cur(index.checked_sub(1));
        self.put(index, label, note, pointers);
        self
    }

    /// Removes the node at `index` of a linked list: `cur` walks to the node before it,
    /// which then links past it.
    pub fn remove(mut self, index: usize) -> Self {
        let Some((_, label)) = self.items.get(index).cloned() else {
            self.steps
                .push(self.step(format!("there is no node {index}")));
            return self;
        };
        if index > 0 {
            let before = self.items[index - 1].1.clone();
            self.walk(index, &format!("walk to {before}"));
        }
        let pointers = self.cur(index.checked_sub(1));
        self.take(index, format!("remove {label}"), pointers);
        self
    }

    /// Walks `cur` from the head until it finds `value`.
    pub fn find(mut self, value: impl Display) -> Self {
        let label = value.to_string();
        for k in 0..self.items.len() {
            let (id, here) = self.items[k].clone();
            let pointers = vec![("cur".into(), id)];
            if here == label {
                let step = ListStep {
                    found: Some(id),
                    pointers,
                    ..self.step(format!("found {label}"))
                };
                self.steps.push(step);
                return self;
            }
            let step = ListStep {
                highlight: vec![id],
                pointers,
                ..self.step(format!("{here} is not {label}"))
            };
            self.steps.push(step);
        }
        self.steps.push(self.step(format!("{label} is not there")));
        self
    }

    /// Reverses a linked list in place: `prev`, `cur` and `next` walk along and each node's
    /// link turns back, then the nodes move to their new order.
    pub fn reverse(mut self) -> Self {
        let n = self.items.len();
        if n < 2 {
            return self;
        }
        let ids: Vec<u32> = self.items.iter().map(|(id, _)| *id).collect();
        let label = |k: usize| self.items[k].1.clone();
        let mut links = self.step(String::new()).links();
        let mut steps = Vec::with_capacity(n + 1);
        for k in 0..n {
            links.retain(|&(from, _)| from != ids[k]);
            let mut pointers = Vec::new();
            if k > 0 {
                links.push((ids[k], ids[k - 1]));
                pointers.push(("prev".into(), ids[k - 1]));
            }
            pointers.push(("cur".into(), ids[k]));
            if k + 1 < n {
                pointers.push(("next".into(), ids[k + 1]));
            }
            let note = if k == 0 {
                format!("{} will be the tail", label(0))
            } else {
                format!("{} points back to {}", label(k), label(k - 1))
            };
            steps.push(ListStep {
                links: Some(links.clone()),
                highlight: vec![ids[k]],
                pointers,
                ..self.step(note)
            });
        }
        self.steps.extend(steps);
        self.items.reverse();
        self.steps.push(self.step("reversed".into()));
        self
    }

    /// Replaces the note of the last step.
    pub fn note(mut self, note: impl Into<String>) -> Self {
        if let Some(last) = self.steps.last_mut() {
            last.note = note.into();
        }
        self
    }

    /// The trace, ending with a step that lights nothing up.
    pub fn finish(mut self) -> ListTrace {
        if self.steps.last().is_some_and(|s| !s.plain()) {
            let note = self
                .steps
                .last()
                .map(|s| s.note.clone())
                .unwrap_or_default();
            self.steps.push(self.step(note));
        }
        ListTrace::new(self.steps)
    }
}

/// The colour of item `id` in `step`.
fn fill_of(colors: &CellColors, step: &ListStep, id: u32) -> Color {
    if step.found == Some(id) {
        colors.found
    } else if step.highlight.contains(&id) {
        colors.compare
    } else {
        colors.base
    }
}

/// A tweened item's fill, blended from the previous step over `blend` seconds.
pub(super) fn blended_fill(
    colors: &CellColors,
    at: &StepAt<'_, ListStep>,
    t: &Tween,
    blend: f32,
) -> Color {
    let now = fill_of(colors, at.step, t.id);
    match (t.from, t.to) {
        (Some(_), Some(_)) => mix(
            fill_of(colors, at.previous, t.id),
            now,
            progress(at.into, 0.0, blend),
        ),
        (Some(_), None) => fill_of(colors, at.previous, t.id),
        _ => now,
    }
}

fn label_of<'s>(at: &StepAt<'s, ListStep>, t: &Tween) -> &'s str {
    match (t.to, t.from) {
        (Some(k), _) => &at.step.items[k].1,
        (None, Some(k)) => &at.previous.items[k].1,
        (None, None) => "",
    }
}

/// A box with a value, as an item of a list, a stack or a queue.
#[allow(clippy::too_many_arguments)]
pub(super) fn item<'a>(
    center: Point,
    width: f64,
    height: f64,
    fill: Color,
    colors: &CellColors,
    style: TextStyle,
    label: &str,
    opacity: f32,
    value_x: f64,
) -> Svgr<'a> {
    let rect = RoundedRect::new(
        center.x - width / 2.0,
        center.y - height / 2.0,
        center.x + width / 2.0,
        center.y + height / 2.0,
        height.min(width) * 0.14,
    );
    let body = Shape {
        path: kurbo::Shape::to_path(&rect, 0.1),
        fill: Some(fill),
        stroke: Some(colors.text.with_alpha(64)),
        stroke_width: 1.5,
        dash: None,
        opacity,
    };
    let text = line(
        value_x as f32,
        (center.y + style.size as f64 * 0.35) as f32,
        style,
        colors.text_on(fill),
        TextAnchor::Middle,
        0.0,
        opacity,
        label.to_owned().into(),
    );
    fframes::svgr!(<g>{body.draw()}{text}</g>)
}

/// A linked list: each node a box with its value and a link cell, `node` by `height` pixels
/// and `gap` apart, centred on `center` (units of `ui.space`). Links are arrows from the link
/// cell to the next node; a link that points back to a node on the left dips under the
/// nodes, and a node without a link shows a slash. It plays a [`ListTrace`], one step every
/// `step_time` seconds from `delay`: nodes slide to their places, new ones drop in, removed
/// ones drop out, links fade as they change, and the pointers (`cur`, and `head` with `head`)
/// sit above the nodes. The whole figure fades with `enter` and `exit`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct LinkedList<'a> {
    pub trace: Option<&'a ListTrace>,
    pub delay: f32,
    pub step_time: f32,
    pub center: (f64, f64),
    pub node: f32,
    pub height: f32,
    pub gap: f32,
    pub head: bool,
    pub notes: bool,
    pub colors: CellColors,
    pub link_color: Color,
    pub pointer_color: Color,
    pub value_style: TextStyle,
    pub small_style: TextStyle,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for LinkedList<'_> {
    fn themed(t: &Theme) -> Self {
        let mut value_style = t.typography.body;
        value_style.weight = 600;
        Self {
            trace: None,
            delay: 0.0,
            step_time: 0.8,
            center: (0.0, 0.0),
            node: 120.0,
            height: 64.0,
            gap: 60.0,
            head: true,
            notes: true,
            colors: CellColors::themed(t),
            link_color: t.colors.text,
            pointer_color: t.colors.highlight,
            value_style,
            small_style: t.typography.caption,
            enter: None,
            exit: None,
        }
    }
}

impl Default for LinkedList<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for LinkedList<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let Some(at) = self
            .trace
            .and_then(|t| t.at(ui.seconds() - self.delay, self.step_time))
        else {
            return Svgr::empty();
        };
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let (w, h) = (f64::from(self.node), f64::from(self.height));
        let cell = h * 0.55;
        let pitch = w + f64::from(self.gap);
        let place = |step: &ListStep| -> Vec<(u32, Point)> {
            let n = step.items.len() as f64;
            step.items
                .iter()
                .enumerate()
                .map(|(k, (id, _))| {
                    (
                        *id,
                        Point::new(middle.x + (k as f64 - (n - 1.0) / 2.0) * pitch, middle.y),
                    )
                })
                .collect()
        };
        let move_time = self.step_time * 0.7;
        let blend = (self.step_time * 0.3).min(0.2);
        let p = f64::from(cubic_in_out(progress(at.into, 0.0, move_time)));
        let (before, now) = (place(at.previous), place(at.step));
        let drop = Vec2::new(0.0, h * 1.3);
        let tweens = tween(&before, &now, p, -drop, drop, 0.3);
        let (links_before, links_now) = (at.previous.links(), at.step.links());
        let has_link = |links: &[(u32, u32)], id: u32| links.iter().any(|(from, _)| *from == id);

        let mut links = Vec::new();
        for (from, to, opacity) in tween_links(&links_before, &links_now, p, true) {
            let (Some(a), Some(b)) = (find(&tweens, from), find(&tweens, to)) else {
                continue;
            };
            let opacity = (opacity * a.opacity.min(b.opacity)) as f32;
            let start = Point::new(a.at.x + w / 2.0 - cell / 2.0, a.at.y);
            let end = Point::new(b.at.x - w / 2.0, b.at.y);
            // forwards a straight arrow, backwards a loop under the nodes, bending smoothly
            // in between so a link never jumps while its nodes move
            let dx = end.x - start.x;
            let depth = (40.0 - dx).max(0.0) * 0.4;
            let reach = if dx > 0.0 {
                (dx / 2.0).clamp(12.0, 36.0)
            } else {
                36.0
            };
            let c2 = end + Vec2::new(-reach, depth);
            let dir = (end - c2).normalize();
            let tip_length = 14.0;
            let mut shaft = BezPath::new();
            shaft.move_to(start);
            shaft.curve_to(
                start + Vec2::new(reach, depth),
                c2 - dir * tip_length,
                end - dir * tip_length,
            );
            links.push(stroke(shaft, self.link_color, 2.5, opacity).draw());
            links.push(tip(end, dir, tip_length, self.link_color, opacity).draw());
            let dot = kurbo::Shape::to_path(&kurbo::Circle::new(start, 5.0), 0.1);
            links.push(
                Shape {
                    path: dot,
                    fill: Some(self.link_color),
                    stroke: None,
                    stroke_width: 0.0,
                    dash: None,
                    opacity,
                }
                .draw(),
            );
        }

        let mut nodes: Vec<(bool, Svgr<'a>)> = Vec::new();
        for t in &tweens {
            let opacity = t.opacity as f32;
            let fill = blended_fill(&self.colors, &at, t, blend);
            let center = t.at;
            let body = item(
                center,
                w,
                h,
                fill,
                &self.colors,
                self.value_style,
                label_of(&at, t),
                opacity,
                center.x - cell / 2.0,
            );
            let divider_x = center.x + w / 2.0 - cell;
            let mut divider = BezPath::new();
            divider.move_to((divider_x, center.y - h / 2.0));
            divider.line_to((divider_x, center.y + h / 2.0));
            let divider = stroke(divider, self.colors.text.with_alpha(64), 1.5, opacity);
            // the slash of a node without a link fades as the link comes or goes
            let linked = |links: &[(u32, u32)], present: bool| {
                if present && !has_link(links, t.id) {
                    1.0
                } else {
                    0.0
                }
            };
            let now = linked(&links_now, t.to.is_some());
            let was = if t.from.is_some() {
                linked(&links_before, true)
            } else {
                now
            };
            let slash_opacity = (was + (now - was) * p) * t.opacity;
            let slash = if slash_opacity > 0.0 {
                let (x0, x1) = (divider_x + cell * 0.25, divider_x + cell * 0.75);
                let mut path = BezPath::new();
                path.move_to((x0, center.y + h * 0.25));
                path.line_to((x1, center.y - h * 0.25));
                stroke(path, self.colors.text_on(fill), 2.0, slash_opacity as f32).draw()
            } else {
                Svgr::empty()
            };
            nodes.push((
                t.crossing,
                fframes::svgr!(<g>{body}{divider.draw()}{slash}</g>),
            ));
        }
        nodes.sort_by_key(|(crossing, _)| *crossing);
        let nodes: Vec<Svgr<'a>> = nodes.into_iter().map(|(_, node)| node).collect();

        let marks = |step: &'a ListStep, places: &[(u32, Point)]| -> Vec<(&'a str, f64)> {
            let x_of = |id: u32| places.iter().find(|(i, _)| *i == id).map(|(_, at)| at.x);
            let head = step
                .items
                .first()
                .filter(|_| self.head)
                .and_then(|(id, _)| x_of(*id).map(|x| ("head", x)));
            head.into_iter()
                .chain(
                    step.pointers
                        .iter()
                        .filter_map(|(name, id)| x_of(*id).map(|x| (name.as_str(), x))),
                )
                .collect()
        };
        let pointers = markers(
            &marks(at.previous, &before),
            &marks(at.step, &now),
            p,
            middle.y - h / 2.0 - 10.0,
            false,
            self.small_style,
            self.pointer_color,
        );
        let mut parts = vec![links.into(), nodes.into(), pointers];
        if self.notes {
            parts.push(notes(
                &at.previous.note,
                &at.step.note,
                at.into,
                middle.x as f32,
                (middle.y + h / 2.0 + 150.0) as f32,
                self.small_style,
                self.colors.text,
                blend,
            ));
        }
        fade(ui, parts.into(), self.enter.as_ref(), self.exit.as_ref())
    }
}

/// A stack: boxes `width` by `height` pixels piled up from the bottom of a container that
/// holds `capacity` of them, centred on `center` (units of `ui.space`), with a `top` marker
/// beside the top box. It plays a [`ListTrace`] (the last item is the top), one step every
/// `step_time` seconds from `delay`: pushed boxes drop in from above, popped ones rise out.
/// With `indices` (manim-dsa's `add_indexes`) each box has its index left of the container,
/// 0 at the bottom, and `label` (`add_label`, empty for none) is written under it. The whole
/// figure fades with `enter` and `exit`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Stack<'a> {
    pub trace: Option<&'a ListTrace>,
    pub delay: f32,
    pub step_time: f32,
    pub center: (f64, f64),
    pub width: f32,
    pub height: f32,
    pub gap: f32,
    pub capacity: usize,
    pub indices: bool,
    pub label: &'a str,
    pub notes: bool,
    pub colors: CellColors,
    pub frame_color: Color,
    pub marker_color: Color,
    pub index_color: Color,
    pub label_color: Color,
    pub value_style: TextStyle,
    pub small_style: TextStyle,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for Stack<'_> {
    fn themed(t: &Theme) -> Self {
        let mut value_style = t.typography.body;
        value_style.weight = 600;
        Self {
            trace: None,
            delay: 0.0,
            step_time: 0.7,
            center: (0.0, 0.0),
            width: 220.0,
            height: 60.0,
            gap: 10.0,
            capacity: 5,
            indices: false,
            label: "",
            notes: true,
            colors: CellColors::themed(t),
            frame_color: t.colors.muted,
            marker_color: t.colors.highlight,
            index_color: t.colors.muted,
            label_color: t.colors.text,
            value_style,
            small_style: t.typography.caption,
            enter: None,
            exit: None,
        }
    }
}

impl Default for Stack<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for Stack<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let Some(at) = self
            .trace
            .and_then(|t| t.at(ui.seconds() - self.delay, self.step_time))
        else {
            return Svgr::empty();
        };
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let (w, h, gap) = (
            f64::from(self.width),
            f64::from(self.height),
            f64::from(self.gap),
        );
        let pitch = h + gap;
        let bottom = middle.y + self.capacity as f64 * pitch / 2.0;
        let place = |step: &ListStep| -> Vec<(u32, Point)> {
            step.items
                .iter()
                .enumerate()
                .map(|(k, (id, _))| {
                    (
                        *id,
                        Point::new(middle.x, bottom - gap - h / 2.0 - k as f64 * pitch),
                    )
                })
                .collect()
        };
        let move_time = self.step_time * 0.7;
        let blend = (self.step_time * 0.3).min(0.2);
        let p = f64::from(cubic_in_out(progress(at.into, 0.0, move_time)));
        let (before, now) = (place(at.previous), place(at.step));
        let rise = Vec2::new(0.0, -2.0 * pitch);
        let tweens = tween(&before, &now, p, rise, rise, 0.0);

        let (left, right) = (middle.x - w / 2.0 - gap, middle.x + w / 2.0 + gap);
        let top = bottom - self.capacity as f64 * pitch - gap;
        let mut frame = BezPath::new();
        frame.move_to((left, top));
        frame.line_to((left, bottom));
        frame.line_to((right, bottom));
        frame.line_to((right, top));
        let frame = stroke(frame, self.frame_color, 3.0, 1.0).draw();

        let boxes: Vec<Svgr<'a>> = tweens
            .iter()
            .map(|t| {
                let fill = blended_fill(&self.colors, &at, t, blend);
                item(
                    t.at,
                    w,
                    h,
                    fill,
                    &self.colors,
                    self.value_style,
                    label_of(&at, t),
                    t.opacity as f32,
                    t.at.x,
                )
            })
            .collect();

        // the `top` marker follows the top box
        let top_y = |places: &[(u32, Point)]| places.last().map(|(_, at)| at.y);
        let marker = match (top_y(&before), top_y(&now)) {
            (Some(a), Some(b)) => Some((a + (b - a) * p, 1.0)),
            (None, Some(b)) => Some((b, p)),
            (Some(a), None) => Some((a, 1.0 - p)),
            (None, None) => None,
        };
        let marker = marker.map_or_else(Svgr::empty, |(y, opacity)| {
            let x = right + 14.0;
            let arrow = tip(
                Point::new(x, y),
                Vec2::new(-1.0, 0.0),
                16.0,
                self.marker_color,
                opacity as f32,
            );
            let name = line(
                (x + 24.0) as f32,
                (y + self.small_style.size as f64 * 0.35) as f32,
                self.small_style,
                self.marker_color,
                TextAnchor::Start,
                0.0,
                opacity as f32,
                "top".into(),
            );
            fframes::svgr!(<g>{arrow.draw()}{name}</g>)
        });
        let mut parts = vec![frame, boxes.into(), marker];
        if self.indices {
            // fixed to the slots, so an index waits for its box instead of falling with it
            let (now, was) = (at.step.items.len(), at.previous.items.len());
            let indices: Vec<Svgr<'a>> = index_row(now, was, p)
                .into_iter()
                .map(|(k, opacity)| {
                    let y = bottom - gap - h / 2.0 - k as f64 * pitch;
                    line(
                        (left - 12.0) as f32,
                        (y + self.small_style.size as f64 * 0.35) as f32,
                        self.small_style,
                        self.index_color,
                        TextAnchor::End,
                        0.0,
                        opacity,
                        k.to_string().into(),
                    )
                })
                .collect();
            parts.push(indices.into());
        }
        let mut below = bottom + 24.0;
        if !self.label.is_empty() {
            below += self.value_style.size as f64 * 0.8;
            parts.push(line(
                middle.x as f32,
                below as f32,
                self.value_style,
                self.label_color,
                TextAnchor::Middle,
                0.0,
                1.0,
                self.label.into(),
            ));
            below += 16.0;
        }
        if self.notes {
            parts.push(notes(
                &at.previous.note,
                &at.step.note,
                at.into,
                middle.x as f32,
                (below + self.small_style.size as f64 * 0.8) as f32,
                self.small_style,
                self.colors.text,
                blend,
            ));
        }
        fade(ui, parts.into(), self.enter.as_ref(), self.exit.as_ref())
    }
}

/// A queue: boxes `width` by `height` pixels in a channel `capacity` boxes long, centred on
/// `center` (units of `ui.space`), the front at the left, with `front` and `back` markers
/// under the boxes. It plays a [`ListTrace`] (the first item is the front), one step every
/// `step_time` seconds from `delay`: enqueued boxes slide in from the right, dequeued ones
/// leave to the left and the rest move up. The whole figure fades with `enter` and `exit`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Queue<'a> {
    pub trace: Option<&'a ListTrace>,
    pub delay: f32,
    pub step_time: f32,
    pub center: (f64, f64),
    pub width: f32,
    pub height: f32,
    pub gap: f32,
    pub capacity: usize,
    pub notes: bool,
    pub colors: CellColors,
    pub frame_color: Color,
    pub marker_color: Color,
    pub value_style: TextStyle,
    pub small_style: TextStyle,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for Queue<'_> {
    fn themed(t: &Theme) -> Self {
        let mut value_style = t.typography.body;
        value_style.weight = 600;
        Self {
            trace: None,
            delay: 0.0,
            step_time: 0.7,
            center: (0.0, 0.0),
            width: 110.0,
            height: 80.0,
            gap: 12.0,
            capacity: 6,
            notes: true,
            colors: CellColors::themed(t),
            frame_color: t.colors.muted,
            marker_color: t.colors.highlight,
            value_style,
            small_style: t.typography.caption,
            enter: None,
            exit: None,
        }
    }
}

impl Default for Queue<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for Queue<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let Some(at) = self
            .trace
            .and_then(|t| t.at(ui.seconds() - self.delay, self.step_time))
        else {
            return Svgr::empty();
        };
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let (w, h, gap) = (
            f64::from(self.width),
            f64::from(self.height),
            f64::from(self.gap),
        );
        let pitch = w + gap;
        let first = middle.x - (self.capacity as f64 - 1.0) * pitch / 2.0;
        let place = |step: &ListStep| -> Vec<(u32, Point)> {
            step.items
                .iter()
                .enumerate()
                .map(|(k, (id, _))| (*id, Point::new(first + k as f64 * pitch, middle.y)))
                .collect()
        };
        let move_time = self.step_time * 0.7;
        let blend = (self.step_time * 0.3).min(0.2);
        let p = f64::from(cubic_in_out(progress(at.into, 0.0, move_time)));
        let (before, now) = (place(at.previous), place(at.step));
        let slide = Vec2::new(-1.5 * pitch, 0.0);
        let tweens = tween(&before, &now, p, -slide, slide, 0.0);

        let (left, right) = (
            first - w / 2.0 - gap,
            first + (self.capacity as f64 - 1.0) * pitch + w / 2.0 + gap,
        );
        let (top, bottom) = (middle.y - h / 2.0 - gap, middle.y + h / 2.0 + gap);
        let mut frame = BezPath::new();
        frame.move_to((left, top));
        frame.line_to((right, top));
        frame.move_to((left, bottom));
        frame.line_to((right, bottom));
        let frame = stroke(frame, self.frame_color, 3.0, 1.0).draw();

        let boxes: Vec<Svgr<'a>> = tweens
            .iter()
            .map(|t| {
                let fill = blended_fill(&self.colors, &at, t, blend);
                item(
                    t.at,
                    w,
                    h,
                    fill,
                    &self.colors,
                    self.value_style,
                    label_of(&at, t),
                    t.opacity as f32,
                    t.at.x,
                )
            })
            .collect();
        let ends = |places: &[(u32, Point)]| -> Vec<(&'static str, f64)> {
            match (places.first(), places.last()) {
                (Some((_, front)), Some((_, back))) => vec![("front", front.x), ("back", back.x)],
                _ => Vec::new(),
            }
        };
        let marks = markers(
            &ends(&before),
            &ends(&now),
            p,
            bottom + 8.0,
            true,
            self.small_style,
            self.marker_color,
        );
        let mut parts = vec![frame, boxes.into(), marks];
        if self.notes {
            parts.push(notes(
                &at.previous.note,
                &at.step.note,
                at.into,
                middle.x as f32,
                (bottom + 40.0 + self.small_style.size as f64 * 3.2) as f32,
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

    fn labels(step: &ListStep) -> Vec<&str> {
        step.items.iter().map(|(_, l)| l.as_str()).collect()
    }

    #[test]
    fn stacks_and_queues_take_from_their_ends() {
        let stack = ListTrace::record(&[1, 2]).push(3).pop().pop().finish();
        assert_eq!(stack.result(), ["1"]);
        let queue = ListTrace::record(&[1, 2]).enqueue(3).dequeue().finish();
        assert_eq!(queue.result(), ["2", "3"]);
        // a pop lights the top up first, then takes it off with the same note
        let steps = stack.steps();
        assert_eq!(steps[2].highlight, [2]);
        assert_eq!(
            (labels(&steps[3]), steps[3].note.as_str()),
            (vec!["1", "2"], "pop 3")
        );
    }

    #[test]
    fn linked_lists_walk_insert_and_remove() {
        let trace = ListTrace::record(&[3, 8, 1])
            .insert(2, 5)
            .remove(0)
            .finish();
        assert_eq!(trace.result(), ["8", "5", "1"]);
        let steps = trace.steps();
        // cur walks over 3 and 8, then 5 goes in after 8
        assert_eq!(steps[1].pointers, [("cur".to_owned(), 0)]);
        assert_eq!(steps[2].pointers, [("cur".to_owned(), 1)]);
        assert_eq!(steps[3].note, "insert 5 after 8");
        assert_eq!(steps[3].highlight, [3]);
        // the new node keeps its id, the head goes
        assert_eq!(steps.last().unwrap().items[1], (3, "5".to_owned()));
        assert!(steps.last().unwrap().plain());
    }

    #[test]
    fn find_stops_at_the_value() {
        let found = ListTrace::record(&[4, 7, 9]).find(7).finish();
        assert_eq!(found.steps()[2].found, Some(1));
        let missing = ListTrace::record(&[4]).find(5).finish();
        assert_eq!(missing.steps().last().unwrap().note, "5 is not there");
    }

    #[test]
    fn reversal_turns_every_link_back() {
        let trace = ListTrace::record(&["a", "b", "c"]).reverse().finish();
        assert_eq!(trace.result(), ["c", "b", "a"]);
        let steps = trace.steps();
        assert_eq!(steps[1].links, Some(vec![(1, 2)]));
        assert_eq!(steps[3].links, Some(vec![(1, 0), (2, 1)]));
        // the last link step and the reordered list have the same links
        let mut flipped = steps[3].links();
        let mut reordered = steps[4].links();
        flipped.sort_unstable();
        reordered.sort_unstable();
        assert_eq!(flipped, reordered);
    }
}
