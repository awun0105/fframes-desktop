//! Finite state machines running over an input, recorded step by step.

use fframes::{Color, Svgr};
use kurbo::{Circle, CubicBez, ParamCurve, Point, RoundedRect, Vec2};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    geometry::Shape,
    motion::{cubic_in_out, progress},
    text::{TextAnchor, line},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::{
    array::CellColors,
    keyed::{
        StepAt, arrow_along, clear_bend, label_point, link, mix, notes, sample, self_loop, stroke,
    },
};

/// A finite state machine: named states, transitions labelled with the symbols they read
/// (`"a"`, or several as `"0,1"`), a start state and accepting states. The states are placed
/// in columns by the fewest transitions from the start (`x`, the start on the left) and rows
/// (`y`) unless [`Machine::at`] places them.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Machine {
    pub states: Vec<String>,
    pub transitions: Vec<(usize, usize, String)>,
    pub start: usize,
    pub accepting: Vec<usize>,
    pub positions: Vec<(f64, f64)>,
}

impl Machine {
    /// Transitions to states that do not exist are dropped.
    pub fn new(
        states: &[&str],
        transitions: &[(usize, usize, &str)],
        start: usize,
        accepting: &[usize],
    ) -> Self {
        let n = states.len();
        let transitions: Vec<(usize, usize, String)> = transitions
            .iter()
            .filter(|&&(a, b, _)| a < n && b < n)
            .map(|&(a, b, label)| (a, b, label.to_owned()))
            .collect();
        let start = start.min(n.saturating_sub(1));
        let positions = columns(n, start, &transitions);
        Self {
            states: states.iter().map(|&s| s.to_owned()).collect(),
            transitions,
            start,
            accepting: accepting.to_vec(),
            positions,
        }
    }

    /// Places the states yourself, in columns and rows like the layout.
    pub fn at(mut self, positions: &[(f64, f64)]) -> Self {
        for (place, &at) in self.positions.iter_mut().zip(positions) {
            *place = at;
        }
        self
    }

    /// The transition taken from `state` on `symbol`.
    pub fn next(&self, state: usize, symbol: &str) -> Option<usize> {
        self.transitions.iter().position(|(from, _, label)| {
            *from == state && label.split(',').any(|s| s.trim() == symbol)
        })
    }
}

/// Places the states in columns by the fewest transitions from `start` (so the machine reads
/// left to right like its input), in the order they are reached within a column and centred
/// on row 0. States the start cannot reach follow in one more column.
fn columns(n: usize, start: usize, transitions: &[(usize, usize, String)]) -> Vec<(f64, f64)> {
    let mut depth: Vec<Option<usize>> = vec![None; n];
    let mut order = Vec::with_capacity(n);
    if n > 0 {
        depth[start] = Some(0);
        order.push(start);
    }
    let mut i = 0;
    while let Some(&s) = order.get(i) {
        i += 1;
        for &(from, to, _) in transitions {
            if from == s && depth[to].is_none() {
                depth[to] = depth[s].map(|d| d + 1);
                order.push(to);
            }
        }
    }
    let last = depth.iter().flatten().max().map_or(0, |d| d + 1);
    order.extend((0..n).filter(|&s| depth[s].is_none()));
    let column = |s: usize| depth[s].unwrap_or(last);
    let mut positions = vec![(0.0, 0.0); n];
    for c in 0..=last {
        let members: Vec<usize> = order.iter().copied().filter(|&s| column(s) == c).collect();
        let count = members.len() as f64;
        for (row, s) in members.into_iter().enumerate() {
            positions[s] = (c as f64, row as f64 - (count - 1.0) / 2.0);
        }
    }
    positions
}

/// One step of a machine's run.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MachineStep {
    /// The state after the step; `None` once the machine is stuck.
    pub state: Option<usize>,
    /// The transition taken in this step.
    pub transition: Option<usize>,
    /// How many symbols are read after the step.
    pub read: usize,
    /// Set on the last step: whether the input was accepted.
    pub accepted: Option<bool>,
    /// What happens in this step, in a few words.
    pub note: String,
}

/// A run of a [`Machine`] over an input, recorded once by [`MachineTrace::run`] and sampled by
/// time with [`MachineTrace::at`]. Step 0 is the start; each symbol takes a step.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct MachineTrace {
    machine: Machine,
    input: Vec<String>,
    steps: Vec<MachineStep>,
}

impl MachineTrace {
    /// Runs `machine` over `input`, one symbol a character. It stops when no transition
    /// reads the next symbol, and accepts when all of the input is read in an accepting state.
    pub fn run(machine: &Machine, input: &str) -> Self {
        let input: Vec<String> = input.chars().map(String::from).collect();
        let name = |s: usize| machine.states.get(s).map_or("", String::as_str);
        let mut steps = vec![MachineStep {
            state: Some(machine.start),
            note: format!("start in {}", name(machine.start)),
            ..MachineStep::default()
        }];
        let mut state = machine.start;
        for (i, symbol) in input.iter().enumerate() {
            let Some(t) = machine.next(state, symbol) else {
                steps.push(MachineStep {
                    state: None,
                    read: i,
                    note: format!("no move on {symbol}"),
                    ..MachineStep::default()
                });
                break;
            };
            let to = machine.transitions[t].1;
            steps.push(MachineStep {
                state: Some(to),
                transition: Some(t),
                read: i + 1,
                note: format!("read {symbol}: {} to {}", name(state), name(to)),
                ..MachineStep::default()
            });
            state = to;
        }
        let last = steps.last().cloned().unwrap_or_default();
        let accepted =
            last.read == input.len() && last.state.is_some_and(|s| machine.accepting.contains(&s));
        steps.push(MachineStep {
            transition: None,
            accepted: Some(accepted),
            note: if accepted { "accepted" } else { "rejected" }.into(),
            ..last
        });
        Self {
            machine: machine.clone(),
            input,
            steps,
        }
    }

    pub fn machine(&self) -> &Machine {
        &self.machine
    }

    pub fn input(&self) -> &[String] {
        &self.input
    }

    pub fn steps(&self) -> &[MachineStep] {
        &self.steps
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// Whether the run accepted its input.
    pub fn accepted(&self) -> bool {
        self.steps.last().and_then(|s| s.accepted).unwrap_or(false)
    }

    /// The step `seconds` after the start, one step every `step_time` seconds. Before the
    /// start it is step 0, after the end the last step.
    pub fn at(&self, seconds: f32, step_time: f32) -> Option<StepAt<'_, MachineStep>> {
        sample(&self.steps, seconds, step_time)
    }
}

/// A state machine running over its input: round states `radius` pixels (accepting ones with
/// a second ring), `spacing` pixels between columns and `level` pixels between rows, centred
/// on `center` (units of `ui.space`), an arrow into the start state, labelled transitions
/// (loops above their state, pairs that point both ways bent apart) and the input as a tape
/// underneath. It plays a [`MachineTrace`], one step every `step_time` seconds from `delay`:
/// a token travels along the transition taken while the tape's symbol lights up, the current
/// state lights up, and at the end it turns `colors.found` when the input is accepted or
/// `reject_color` when not. The whole figure fades with `enter` and `exit`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct StateMachine<'a> {
    pub trace: Option<&'a MachineTrace>,
    pub delay: f32,
    pub step_time: f32,
    pub center: (f64, f64),
    pub spacing: f32,
    pub level: f32,
    pub radius: f32,
    pub tape: bool,
    pub notes: bool,
    pub colors: CellColors,
    pub edge_color: Color,
    pub active_color: Color,
    pub reject_color: Color,
    pub value_style: TextStyle,
    pub small_style: TextStyle,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for StateMachine<'_> {
    fn themed(t: &Theme) -> Self {
        let mut value_style = t.typography.body;
        value_style.weight = 600;
        Self {
            trace: None,
            delay: 0.0,
            step_time: 0.9,
            center: (0.0, 0.0),
            spacing: 240.0,
            level: 170.0,
            radius: 40.0,
            tape: true,
            notes: true,
            colors: CellColors::themed(t),
            edge_color: t.colors.muted,
            active_color: t.colors.highlight,
            reject_color: Color::hex("#ff6b6b"),
            value_style,
            small_style: t.typography.caption,
            enter: None,
            exit: None,
        }
    }
}

impl Default for StateMachine<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl StateMachine<'_> {
    fn fill(&self, step: &MachineStep, s: usize) -> Color {
        match (step.state == Some(s), step.accepted) {
            (true, Some(true)) => self.colors.found,
            (true, Some(false)) => self.reject_color,
            (true, None) => self.colors.compare,
            _ => self.colors.base,
        }
    }
}

impl<'a> Component<'a> for StateMachine<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let Some(trace) = self.trace else {
            return Svgr::empty();
        };
        let Some(at) = trace.at(ui.seconds() - self.delay, self.step_time) else {
            return Svgr::empty();
        };
        let machine = trace.machine();
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let (spacing, level, radius) = (
            f64::from(self.spacing),
            f64::from(self.level),
            f64::from(self.radius),
        );
        let (lo, hi) = machine.positions.iter().fold(
            ((f64::MAX, f64::MAX), (f64::MIN, f64::MIN)),
            |(lo, hi), &(x, y)| ((lo.0.min(x), lo.1.min(y)), (hi.0.max(x), hi.1.max(y))),
        );
        let (mx, my) = (f64::midpoint(lo.0, hi.0), f64::midpoint(lo.1, hi.1));
        let points: Vec<Point> = machine
            .positions
            .iter()
            .map(|&(x, y)| Point::new(middle.x + (x - mx) * spacing, middle.y + (y - my) * level))
            .collect();
        let blend = (self.step_time * 0.3).min(0.2);
        let q = progress(at.into, 0.0, blend);
        let travel = f64::from(cubic_in_out(progress(at.into, 0.0, self.step_time * 0.6)));

        // every transition's curve, once
        let curves: Vec<CubicBez> = machine
            .transitions
            .iter()
            .map(|(a, b, _)| {
                if a == b {
                    return self_loop(points[*a], radius);
                }
                let others: Vec<Point> = (0..points.len())
                    .filter(|v| v != a && v != b)
                    .map(|v| points[v])
                    .collect();
                let paired = machine.transitions.iter().any(|(c, d, _)| c == b && d == a);
                let base = if paired { 30.0 } else { 0.0 };
                let bend = clear_bend(points[*a], points[*b], radius, &others, radius + 10.0, base);
                link(points[*a], points[*b], radius, radius, bend)
            })
            .collect();

        let lit = |step: &MachineStep, t: usize| if step.transition == Some(t) { 1.0 } else { 0.0 };
        let mut edges = Vec::new();
        let mut labels = Vec::new();
        for (t, (a, b, label)) in machine.transitions.iter().enumerate() {
            let glow = lit(at.previous, t) + (lit(at.step, t) - lit(at.previous, t)) * q;
            let color = mix(self.edge_color, self.active_color, glow);
            let curve = curves[t];
            edges.push(arrow_along(curve, true, color, 2.5 + 2.0 * glow, 1.0));
            let place = if a == b {
                Point::new(points[*a].x, points[*a].y - radius * 2.75)
            } else {
                label_point(&curve, 20.0)
            };
            labels.push(line(
                place.x as f32,
                (place.y + self.small_style.size as f64 * 0.35) as f32,
                self.small_style,
                mix(self.colors.text, self.active_color, glow),
                TextAnchor::Middle,
                0.0,
                1.0,
                label.as_str().into(),
            ));
        }
        // the start arrow comes in from the left
        let start = points[machine.start];
        let entry = CubicBez::new(
            start + Vec2::new(-radius * 2.4, 0.0),
            start + Vec2::new(-radius * 1.9, 0.0),
            start + Vec2::new(-radius * 1.5, 0.0),
            start + Vec2::new(-radius, 0.0),
        );
        edges.push(arrow_along(entry, true, self.edge_color, 2.5, 1.0));

        let mut states = Vec::new();
        for (s, name) in machine.states.iter().enumerate() {
            let p = points[s];
            let fill = mix(self.fill(at.previous, s), self.fill(at.step, s), q);
            states.push(
                Shape {
                    path: kurbo::Shape::to_path(&Circle::new(p, radius), 0.1),
                    fill: Some(fill),
                    stroke: Some(self.colors.text.with_alpha(96)),
                    stroke_width: 2.0,
                    dash: None,
                    opacity: 1.0,
                }
                .draw(),
            );
            if machine.accepting.contains(&s) {
                let ring = kurbo::Shape::to_path(&Circle::new(p, radius - 7.0), 0.1);
                states
                    .push(stroke(ring, self.colors.text_on(fill).with_alpha(160), 2.0, 1.0).draw());
            }
            states.push(line(
                p.x as f32,
                (p.y + self.value_style.size as f64 * 0.35) as f32,
                self.value_style,
                self.colors.text_on(fill),
                TextAnchor::Middle,
                0.0,
                1.0,
                name.as_str().into(),
            ));
        }

        // the token on the transition taken
        let token = at
            .step
            .transition
            .filter(|_| travel < 1.0)
            .map_or_else(Svgr::empty, |t| {
                let p = curves[t].eval(travel);
                Shape {
                    path: kurbo::Shape::to_path(&Circle::new(p, 9.0), 0.1),
                    fill: Some(self.active_color),
                    stroke: None,
                    stroke_width: 0.0,
                    dash: None,
                    opacity: 1.0,
                }
                .draw()
            });

        let mut parts = vec![edges.into(), labels.into(), states.into(), token];
        // below the lowest state, clear of the transitions bending under it and their labels
        let mut y = middle.y + (hi.1 - my) * level + radius + 80.0;
        if self.tape && !trace.input().is_empty() {
            let cell = 52.0;
            let n = trace.input().len();
            let read = |step: &MachineStep| step.read as f64;
            let done = read(at.previous) + (read(at.step) - read(at.previous)) * f64::from(q);
            let mut cells = Vec::new();
            for (i, symbol) in trace.input().iter().enumerate() {
                let x = middle.x + (i as f64 - (n as f64 - 1.0) / 2.0) * (cell + 6.0);
                // the symbol being read lights up, the ones read dim
                let reading = |step: &MachineStep| {
                    if step.transition.is_some() && step.read == i + 1 {
                        1.0
                    } else {
                        0.0
                    }
                };
                let glow = reading(at.previous) + (reading(at.step) - reading(at.previous)) * q;
                let fill = mix(self.colors.base, self.colors.compare, glow);
                let read = (done - i as f64).clamp(0.0, 1.0) as f32;
                let opacity = 1.0 - 0.6 * read * (1.0 - glow);
                let rect = RoundedRect::new(x - cell / 2.0, y, x + cell / 2.0, y + cell, 6.0);
                cells.push(
                    Shape {
                        path: kurbo::Shape::to_path(&rect, 0.1),
                        fill: Some(fill),
                        stroke: Some(self.colors.text.with_alpha(64)),
                        stroke_width: 1.5,
                        dash: None,
                        opacity,
                    }
                    .draw(),
                );
                cells.push(line(
                    x as f32,
                    (y + cell / 2.0 + self.value_style.size as f64 * 0.35) as f32,
                    self.value_style,
                    self.colors.text_on(fill),
                    TextAnchor::Middle,
                    0.0,
                    opacity,
                    symbol.as_str().into(),
                ));
            }
            parts.push(cells.into());
            y += cell + 20.0;
        }
        if self.notes {
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

    /// Binary numbers with an even number of ones.
    fn even_ones() -> Machine {
        Machine::new(
            &["even", "odd"],
            &[(0, 0, "0"), (0, 1, "1"), (1, 1, "0"), (1, 0, "1")],
            0,
            &[0],
        )
    }

    #[test]
    fn runs_accept_and_reject() {
        let machine = even_ones();
        let yes = MachineTrace::run(&machine, "1001");
        assert!(yes.accepted());
        assert_eq!(yes.len(), 6);
        assert_eq!(yes.steps()[1].note, "read 1: even to odd");
        assert!(!MachineTrace::run(&machine, "10").accepted());
        let stuck = MachineTrace::run(&machine, "12");
        assert!(!stuck.accepted());
        assert_eq!(stuck.steps()[2].note, "no move on 2");
        assert!(MachineTrace::run(&machine, "").accepted());
    }

    #[test]
    fn labels_list_their_symbols() {
        let m = Machine::new(&["a", "b"], &[(0, 1, "0, 1")], 0, &[1]);
        assert_eq!(m.next(0, "1"), Some(0));
        assert_eq!(m.next(0, "2"), None);
        // the start is in the first column
        assert!(m.positions[0].0 < m.positions[1].0);
    }
}
