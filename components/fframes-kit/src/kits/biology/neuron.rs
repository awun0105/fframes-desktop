//! A neuron passing an impulse along its axon, and the action potential that carries it.

use std::f64::consts::PI;

use fframes::{Color, Svgr};
use kurbo::{BezPath, Circle, Point, RoundedRect, Shape as _};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    explain::{Axes, Range},
    geometry::Shape,
    kits::draw::{partial, stroke},
    motion::{Rng, cubic_in_out, progress},
    text::{TextAnchor, line},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

fn disc(center: Point, radius: f64, fill: Color, opacity: f32) -> Shape {
    Shape {
        path: Circle::new(center, radius).to_path(0.1),
        fill: Some(fill),
        stroke: None,
        stroke_width: 0.0,
        dash: None,
        opacity,
    }
}

fn segment(from: Point, to: Point) -> BezPath {
    let mut path = BezPath::new();
    path.move_to(from);
    path.line_to(to);
    path
}

/// How bright a spot lit at `at` seconds is at `t`: up in 0.1 seconds, gone after 0.5.
fn flash(t: f32, at: f32) -> f32 {
    if t < at {
        return 0.0;
    }
    progress(t, at, at + 0.1) * (1.0 - progress(t, at + 0.1, at + 0.5))
}

/// The parts of a [`Neuron`], placed once.
struct Layout {
    soma: Point,
    soma_r: f64,
    /// Branches of the dendrites: from, to and stroke width.
    dendrites: Vec<(Point, Point, f32)>,
    dendrite_tip: Point,
    axon: (f64, f64),
    /// The myelin segments, left to right, as their x ranges.
    myelin: Vec<(f64, f64)>,
    /// The axon terminals: the branch and its bouton.
    terminals: Vec<(BezPath, Point)>,
}

impl Layout {
    fn new(middle: Point, length: f64, seed: u64) -> Self {
        let soma = Point::new(middle.x - length / 2.0 + 270.0, middle.y);
        let soma_r = 62.0;
        let mut rng = Rng::new(seed);
        let mut dendrites = Vec::new();
        let mut dendrite_tip = soma;
        for d in 0..6 {
            let angle = PI * (0.62 + 0.76 * f64::from(d) / 5.0) + rng.range(-0.08, 0.08);
            let dir = Point::new(angle.cos(), angle.sin()).to_vec2();
            let start = soma + dir * (soma_r - 6.0);
            let len = rng.range(110.0, 160.0);
            let end = start + dir * len;
            dendrites.push((start, end, 8.0));
            let fork = start + dir * (len * 0.6);
            for turn in [-0.5, 0.45] {
                let a = angle + turn + rng.range(-0.1, 0.1);
                let tip = fork + Point::new(a.cos(), a.sin()).to_vec2() * (len * 0.55);
                dendrites.push((fork, tip, 4.0));
                // the name goes on the branch reaching furthest up and left
                if tip.x + tip.y < dendrite_tip.x + dendrite_tip.y {
                    dendrite_tip = tip;
                }
            }
        }
        let axon = (soma.x + soma_r - 4.0, middle.x + length / 2.0 - 150.0);
        let (seg, gap) = (120.0, 22.0);
        let first = axon.0 + 70.0;
        let count = ((axon.1 - first + gap) / (seg + gap)).floor().max(1.0) as usize;
        let used = count as f64 * (seg + gap) - gap;
        let start = first + (axon.1 - first - used) / 2.0;
        let myelin = (0..count)
            .map(|i| {
                let x = start + i as f64 * (seg + gap);
                (x, x + seg)
            })
            .collect();
        let end = Point::new(axon.1, middle.y);
        let terminals = (0..4)
            .map(|i| {
                let a = -0.6 + 1.2 * f64::from(i) / 3.0;
                let bouton = end + Point::new(a.cos(), a.sin()).to_vec2() * 110.0;
                let mut path = BezPath::new();
                path.move_to(end);
                path.quad_to(
                    Point::new(end.x + 55.0, end.y + (bouton.y - end.y) * 0.2),
                    bouton,
                );
                (path, bouton)
            })
            .collect();
        Self {
            soma,
            soma_r,
            dendrites,
            dendrite_tip,
            axon,
            myelin,
            terminals,
        }
    }

    /// The nodes of Ranvier: the gaps before, between and after the myelin segments.
    fn nodes(&self) -> Vec<f64> {
        let mut nodes = vec![self.axon.0 + 35.0];
        nodes.extend(
            self.myelin
                .windows(2)
                .map(|w| f64::midpoint(w[0].1, w[1].0)),
        );
        if let Some(last) = self.myelin.last() {
            nodes.push(f64::midpoint(last.1, self.axon.1));
        }
        nodes
    }
}

/// A neuron lying across the frame, `length` pixels from its dendrites to its axon terminals,
/// centred on `center` (units of `ui.space`): dendrites branching from the cell body, the
/// axon wrapped in myelin with nodes of Ranvier between the segments, and the axon terminals.
/// It grows in from `delay`, soma first; with `labels` each part is then named above or
/// below it. At each time in `impulses` the cell body fires and the impulse jumps from node
/// to node along the axon (`speed` pixels a second) to the terminals, which release
/// neurotransmitter. The dendrites are placed from `seed`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Neuron {
    pub delay: f32,
    pub center: (f64, f64),
    pub length: f32,
    pub impulses: Vec<f32>,
    pub speed: f32,
    pub labels: bool,
    pub seed: u64,
    pub body_color: Color,
    pub nucleus_color: Color,
    pub myelin_color: Color,
    pub impulse_color: Color,
    pub leader_color: Color,
    pub label_color: Color,
    pub label_style: TextStyle,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for Neuron {
    fn themed(t: &Theme) -> Self {
        Self {
            delay: 0.0,
            center: (0.0, -0.2),
            length: 1320.0,
            impulses: vec![4.6, 6.6],
            speed: 700.0,
            labels: true,
            seed: 5,
            body_color: Color::hex("#4dabf7"),
            nucleus_color: Color::hex("#1c4a7a"),
            myelin_color: Color::hex("#ffd8a8"),
            impulse_color: Color::hex("#ffe066"),
            leader_color: t.colors.muted,
            label_color: t.colors.text,
            label_style: t.typography.caption,
            enter: None,
            exit: None,
        }
    }
}

impl Default for Neuron {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for Neuron {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let layout = Layout::new(middle, f64::from(self.length), self.seed);
        let t = ui.seconds() - self.delay;
        let y = middle.y;
        let body = self.body_color;

        let grown = cubic_in_out(progress(t, 0.0, 0.8));
        let mut out = Vec::new();
        // the dendrites grow out of the cell body
        for &(from, to, width) in &layout.dendrites {
            let tip = from + (to - from) * f64::from(grown);
            out.push(stroke(segment(from, tip), body, width, grown).draw());
        }
        // the axon draws from the cell body to the terminals
        let along = f64::from(cubic_in_out(progress(t, 0.4, 1.6)));
        let reach = layout.axon.0 + (layout.axon.1 - layout.axon.0) * along;
        if along > 0.0 {
            out.push(
                stroke(
                    segment(Point::new(layout.axon.0, y), Point::new(reach, y)),
                    body,
                    9.0,
                    1.0,
                )
                .draw(),
            );
        }
        for &(x0, x1) in &layout.myelin {
            // each segment wraps the axon as it passes
            let on = cubic_in_out(progress(reach as f32, x0 as f32, x1 as f32));
            if on > 0.0 {
                out.push(
                    Shape {
                        path: RoundedRect::new(
                            x0,
                            y - 17.0,
                            x0 + (x1 - x0) * f64::from(on),
                            y + 17.0,
                            15.0,
                        )
                        .to_path(0.1),
                        fill: Some(self.myelin_color),
                        stroke: None,
                        stroke_width: 0.0,
                        dash: None,
                        opacity: 1.0,
                    }
                    .draw(),
                );
            }
        }
        let branches = cubic_in_out(progress(t, 1.5, 2.1));
        for (path, bouton) in &layout.terminals {
            if branches > 0.0 {
                let mut shape = stroke(path.clone(), body, 5.0, 1.0);
                shape.opacity = branches;
                out.push(shape.draw());
                out.push(disc(*bouton, 13.0 * f64::from(branches), body, 1.0).draw());
            }
        }
        // the cell body and its nucleus, over the roots of the dendrites
        out.push(
            disc(
                layout.soma,
                layout.soma_r * (0.6 + 0.4 * f64::from(grown)),
                body,
                grown.min(1.0),
            )
            .draw(),
        );
        out.push(
            disc(
                layout.soma,
                24.0 * f64::from(grown),
                self.nucleus_color,
                grown,
            )
            .draw(),
        );

        // impulses: the cell body fires, the nodes light in turn, the terminals release
        let nodes = layout.nodes();
        let mut glows = Vec::new();
        for &fire in &self.impulses {
            let fire = fire - self.delay;
            if t < fire {
                continue;
            }
            let lit = flash(t, fire);
            if lit > 0.0 {
                glows.push(
                    disc(
                        layout.soma,
                        layout.soma_r + 10.0,
                        self.impulse_color,
                        0.35 * lit,
                    )
                    .draw(),
                );
            }
            for &x in &nodes {
                let at = fire + ((x - layout.axon.0) / f64::from(self.speed)) as f32;
                let lit = flash(t, at);
                if lit > 0.0 {
                    glows.push(disc(Point::new(x, y), 26.0, self.impulse_color, 0.4 * lit).draw());
                    glows.push(disc(Point::new(x, y), 9.0, self.impulse_color, lit).draw());
                }
            }
            let arrive =
                fire + ((layout.axon.1 - layout.axon.0) / f64::from(self.speed)) as f32 + 0.1;
            let mut rng = Rng::new(self.seed + 1);
            for (_, bouton) in &layout.terminals {
                let lit = flash(t, arrive);
                if lit > 0.0 {
                    glows.push(disc(*bouton, 22.0, self.impulse_color, 0.5 * lit).draw());
                }
                // neurotransmitter spreads from each bouton
                let spread = progress(t, arrive + 0.05, arrive + 0.9);
                for _ in 0..5 {
                    let a = rng.range(-1.0, 1.0);
                    let dist = rng.range(24.0, 46.0);
                    if spread > 0.0 && spread < 1.0 {
                        let p = *bouton
                            + Point::new(a.cos(), a.sin()).to_vec2()
                                * (dist * f64::from(cubic_in_out(spread)));
                        let fade_out = 1.0 - progress(spread, 0.6, 1.0);
                        glows.push(
                            disc(
                                p,
                                4.0,
                                self.impulse_color,
                                fade_out * progress(spread, 0.0, 0.1),
                            )
                            .draw(),
                        );
                    }
                }
            }
        }
        out.extend(glows);

        // the names, above and below their parts
        let mut names = Vec::new();
        if self.labels {
            let nodes_at = nodes.get(2).copied().unwrap_or(layout.axon.0);
            let sheath = layout
                .myelin
                .get(3)
                .map_or(layout.axon.0, |&(x0, x1)| f64::midpoint(x0, x1));
            let terminal = layout
                .terminals
                .first()
                .map_or(Point::new(layout.axon.1, y), |t| t.1);
            let parts = [
                ("dendrites", layout.dendrite_tip, true),
                ("nucleus", Point::new(layout.soma.x + 10.0, y - 20.0), true),
                (
                    "cell body",
                    Point::new(layout.soma.x + 15.0, y + layout.soma_r - 6.0),
                    false,
                ),
                ("axon", Point::new(layout.axon.0 + 80.0, y + 4.0), false),
                ("myelin sheath", Point::new(sheath, y - 17.0), true),
                ("node of Ranvier", Point::new(nodes_at, y + 4.0), false),
                ("axon terminals", terminal, true),
            ];
            let row = self.label_style.size as f64;
            for (i, (text, anchor, above)) in parts.into_iter().enumerate() {
                let start = 2.0 + i as f32 * 0.35;
                let drawn = cubic_in_out(progress(t, start, start + 0.4));
                if drawn <= 0.0 {
                    continue;
                }
                let end_y = if above { y - 250.0 } else { y + 230.0 };
                let reached = partial(&[anchor, Point::new(anchor.x, end_y)], f64::from(drawn));
                names.push(
                    stroke(
                        segment(reached[0], reached[reached.len() - 1]),
                        self.leader_color,
                        2.0,
                        1.0,
                    )
                    .draw(),
                );
                names.push(disc(anchor, 5.0, self.leader_color, drawn).draw());
                let text_y = if above {
                    end_y - 12.0
                } else {
                    end_y + row + 4.0
                };
                names.push(line(
                    anchor.x as f32,
                    text_y as f32,
                    self.label_style,
                    self.label_color,
                    TextAnchor::Middle,
                    0.0,
                    progress(t, start + 0.3, start + 0.6),
                    text.into(),
                ));
            }
        }
        out.extend(names);
        fade(ui, out.into(), self.enter.as_ref(), self.exit.as_ref())
    }
}

/// The membrane potential of a neuron at `ms` milliseconds into an action potential, in
/// millivolts: resting at -70 until a stimulus at 1.5 ms takes it to the threshold of -55,
/// then it shoots up to +40, falls past rest to -80 and recovers to -70 by 5 ms.
pub fn membrane_potential(ms: f64) -> f64 {
    let ease = |a: f64, b: f64| f64::from(cubic_in_out(progress(ms as f32, a as f32, b as f32)));
    let mut v = -70.0;
    v += 15.0 * ease(1.5, 1.8);
    v += 95.0 * ease(1.8, 2.3);
    v -= 120.0 * ease(2.3, 3.1);
    v += 10.0 * ease(3.1, 5.0);
    v
}

/// An action potential traced on axes of time (ms) against membrane potential (mV),
/// `x_length` by `y_length` (units of `ui.space`) around `center`. From `delay` the trace
/// draws from 0 to 6 ms in `duration` seconds, a dot at its head; the dashed threshold and
/// resting lines are there from the start. Each phase is named as the trace reaches it:
/// resting potential, stimulus, depolarization, the peak, repolarization and
/// hyperpolarization.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct ActionPotential {
    pub delay: f32,
    pub duration: f32,
    pub center: (f64, f64),
    pub x_length: f64,
    pub y_length: f64,
    pub trace_color: Color,
    pub threshold_color: Color,
    pub axis_color: Color,
    pub label_style: TextStyle,
    pub phase_style: TextStyle,
    pub text_color: Color,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for ActionPotential {
    fn themed(t: &Theme) -> Self {
        let mut label_style = t.typography.caption;
        label_style.weight = 600;
        Self {
            delay: 0.0,
            duration: 5.0,
            center: (0.0, -0.3),
            x_length: 9.0,
            y_length: 5.2,
            trace_color: Color::hex("#ffd43b"),
            threshold_color: Color::hex("#ff8787"),
            axis_color: t.colors.muted,
            label_style,
            phase_style: t.typography.caption,
            text_color: t.colors.text,
            enter: None,
            exit: None,
        }
    }
}

impl Default for ActionPotential {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for ActionPotential {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let (x_range, y_range) = (Range::new(0.0, 6.0, 1.0), Range::new(-90.0, 50.0, 30.0));
        let axes = Axes {
            x_range,
            y_range,
            x_length: self.x_length,
            y_length: self.y_length,
            center: self.center,
            color: self.axis_color,
            x_numbers: x_range.ticks(),
            y_numbers: y_range.ticks(),
            x_label: Some("time (ms)".into()),
            y_label: Some("mV".into()),
            label_style: self.label_style,
            ..ui.defaults()
        };
        let map = axes.map(ui);
        let t = ui.seconds() - self.delay;
        let ms = 6.0 * f64::from(progress(t, 0.0, self.duration));

        let mut out = Vec::new();
        // the threshold and resting lines
        for (mv, color, name) in [
            (-55.0, self.threshold_color, "threshold"),
            (-70.0, self.axis_color, "resting"),
        ] {
            let (a, b) = (map.c2p(0.0, mv), map.c2p(6.0, mv));
            let mut shape = stroke(segment(a, b), color, 2.0, 0.8);
            shape.dash = Some((10.0, 8.0));
            out.push(shape.draw());
            if name == "threshold" {
                out.push(line(
                    (b.x + 14.0) as f32,
                    (b.y + 9.0) as f32,
                    self.phase_style,
                    color,
                    TextAnchor::Start,
                    0.0,
                    1.0,
                    "threshold".into(),
                ));
            }
        }
        if t > 0.0 {
            let steps = (ms / 6.0 * 240.0).ceil().max(1.0) as usize;
            let mut path = BezPath::new();
            for i in 0..=steps {
                let x = ms * i as f64 / steps as f64;
                let p = map.c2p(x, membrane_potential(x));
                if i == 0 {
                    path.move_to(p);
                } else {
                    path.line_to(p);
                }
            }
            out.push(stroke(path, self.trace_color, 5.0, 1.0).draw());
            out.push(
                disc(
                    map.c2p(ms, membrane_potential(ms)),
                    9.0,
                    self.trace_color,
                    1.0,
                )
                .draw(),
            );
        }

        // the phases, named as the trace reaches them: time, name, offset, anchor
        let phases = [
            (0.0, "resting potential", (12.0, -16.0), TextAnchor::Start),
            (1.65, "stimulus", (0.0, 80.0), TextAnchor::Middle),
            (2.1, "depolarization", (-24.0, 0.0), TextAnchor::End),
            (2.3, "peak +40 mV", (0.0, -24.0), TextAnchor::Middle),
            (2.7, "repolarization", (24.0, 0.0), TextAnchor::Start),
            (3.1, "hyperpolarization", (14.0, 40.0), TextAnchor::Start),
        ];
        for (at, name, (dx, dy), anchor) in phases {
            let shown = progress(ms as f32, at as f32, at as f32 + 0.3);
            if shown <= 0.0 {
                continue;
            }
            let p = map.c2p(at, membrane_potential(at));
            out.push(line(
                (p.x + dx) as f32,
                (p.y + dy) as f32,
                self.phase_style,
                self.text_color,
                anchor,
                0.0,
                shown,
                name.into(),
            ));
        }

        let axes = ui.show(axes);
        let parts: Svgr<'a> = vec![axes, out.into()].into();
        fade(ui, parts, self.enter.as_ref(), self.exit.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_action_potential_has_its_landmarks() {
        assert_eq!(membrane_potential(0.0), -70.0);
        assert!((membrane_potential(1.8) + 55.0).abs() < 1e-6);
        assert!((membrane_potential(2.3) - 40.0).abs() < 1e-6);
        assert!((membrane_potential(3.1) + 80.0).abs() < 1e-6);
        assert!((membrane_potential(6.0) + 70.0).abs() < 1e-6);
        let lowest = (0..=600)
            .map(|i| membrane_potential(f64::from(i) / 100.0))
            .fold(f64::INFINITY, f64::min);
        assert!((lowest + 80.0).abs() < 1e-6);
    }

    #[test]
    fn the_myelin_leaves_nodes_along_the_axon() {
        let layout = Layout::new(Point::new(960.0, 540.0), 1320.0, 5);
        assert!(layout.myelin.len() >= 4);
        let nodes = layout.nodes();
        assert_eq!(nodes.len(), layout.myelin.len() + 1);
        for (node, w) in nodes[1..].iter().zip(layout.myelin.windows(2)) {
            assert!(w[0].1 < *node && *node < w[1].0);
        }
        assert!(nodes.windows(2).all(|w| w[0] < w[1]));
    }
}
