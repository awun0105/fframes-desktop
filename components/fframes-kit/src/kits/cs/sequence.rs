//! Sequence diagrams: participants with lifelines and the messages between them, in order.

use fframes::{Color, Svgr};
use kurbo::{BezPath, Circle, Point, RoundedRect};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    geometry::Shape,
    motion::{cubic_in_out, progress},
    text::{TextAnchor, line},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::keyed::{locate, mix, stroke, tip};
use crate::kits::draw::partial;

/// A message from one participant to another (or to itself); a reply is drawn dashed.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Message {
    pub from: usize,
    pub to: usize,
    pub label: String,
    pub reply: bool,
}

impl Message {
    pub fn new(from: usize, to: usize, label: impl Into<String>) -> Self {
        Self {
            from,
            to,
            label: label.into(),
            reply: false,
        }
    }

    /// A reply, drawn dashed.
    pub fn reply(from: usize, to: usize, label: impl Into<String>) -> Self {
        Self {
            reply: true,
            ..Self::new(from, to, label)
        }
    }
}

/// `text` cut with `…` to at most `max` pixels in `style`, measured from the font when it can
/// be resolved.
fn fit_to(ui: &Ui<'_, '_, '_>, style: TextStyle, text: &str, max: f64) -> String {
    let width = |s: &str| {
        ui.measure(style.font(), s).map_or_else(
            || s.chars().count() as f64 * style.size as f64 * 0.55,
            f64::from,
        )
    };
    if width(text) <= max {
        return text.to_owned();
    }
    let mut kept: Vec<char> = text.chars().collect();
    while !kept.is_empty() {
        kept.pop();
        let cut: String = kept.iter().collect::<String>().trim_end().to_owned() + "…";
        if width(&cut) <= max {
            return cut;
        }
    }
    String::new()
}

/// A sequence diagram: `participants` as boxes along the top, `spacing` pixels apart, with
/// dashed lifelines below, centred on `center` (units of `ui.space`). From `delay` it sends
/// one of `messages` every `step_time` seconds, `row` pixels below the one before: a packet
/// travels from the sender's lifeline to the receiver's in the first `travel` of the step,
/// drawing the arrow behind it, and the label appears above it, cut with `…` to fit between
/// the lifelines. A message to the sender itself loops out to the right. The message being
/// sent is in `active_color`, earlier ones in `message_color`; replies are dashed. With
/// `numbers` the labels are numbered. The whole figure fades with `enter` and `exit`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct SequenceDiagram {
    pub participants: Vec<String>,
    pub messages: Vec<Message>,
    pub delay: f32,
    pub step_time: f32,
    pub travel: f32,
    pub center: (f64, f64),
    pub spacing: f32,
    pub row: f32,
    pub numbers: bool,
    pub box_color: Color,
    pub line_color: Color,
    pub message_color: Color,
    pub active_color: Color,
    pub text_color: Color,
    pub name_style: TextStyle,
    pub label_style: TextStyle,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for SequenceDiagram {
    fn themed(t: &Theme) -> Self {
        let mut name_style = t.typography.caption;
        name_style.weight = 600;
        Self {
            participants: Vec::new(),
            messages: Vec::new(),
            delay: 0.0,
            step_time: 1.0,
            travel: 0.6,
            center: (0.0, 0.0),
            spacing: 420.0,
            row: 76.0,
            numbers: true,
            box_color: t.colors.surface,
            line_color: t.colors.muted,
            message_color: t.colors.text,
            active_color: t.colors.highlight,
            text_color: t.colors.text,
            name_style,
            label_style: t.typography.caption,
            enter: None,
            exit: None,
        }
    }
}

impl Default for SequenceDiagram {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl SequenceDiagram {
    /// How many messages are sent `seconds` in, and how far into the last one's step.
    fn sent(&self, seconds: f32) -> Option<(usize, f32)> {
        if seconds < self.delay {
            return None;
        }
        locate(self.messages.len(), seconds - self.delay, self.step_time)
            .map(|(index, into)| (index + 1, into))
    }
}

impl<'a> Component<'a> for SequenceDiagram {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let n = self.participants.len();
        if n == 0 {
            return Svgr::empty();
        }
        let (spacing, row) = (f64::from(self.spacing), f64::from(self.row));
        let header = self.name_style.size as f64 * 2.2;
        let height = header + (self.messages.len() as f64 + 0.6) * row;
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let top = middle.y - height / 2.0;
        let x_of = |p: usize| middle.x + (p as f64 - (n as f64 - 1.0) / 2.0) * spacing;
        let y_of = |m: usize| top + header + (m as f64 + 0.8) * row;
        let bottom = top + height;

        let mut back = Vec::new();
        for (p, name) in self.participants.iter().enumerate() {
            let x = x_of(p);
            let mut life = BezPath::new();
            life.move_to((x, top + header));
            life.line_to((x, bottom));
            back.push(
                Shape {
                    dash: Some((10.0, 8.0)),
                    ..stroke(life, self.line_color, 2.0, 1.0)
                }
                .draw(),
            );
            let name = fit_to(ui, self.name_style, name, spacing - 64.0);
            let w = ui
                .measure(self.name_style.font(), &name)
                .map_or(spacing * 0.6, f64::from)
                + 48.0;
            back.push(
                Shape {
                    path: kurbo::Shape::to_path(
                        &RoundedRect::new(x - w / 2.0, top, x + w / 2.0, top + header, 12.0),
                        0.1,
                    ),
                    fill: Some(self.box_color),
                    stroke: Some(self.line_color),
                    stroke_width: 2.0,
                    dash: None,
                    opacity: 1.0,
                }
                .draw(),
            );
            back.push(line(
                x as f32,
                (top + header / 2.0 + self.name_style.size as f64 * 0.35) as f32,
                self.name_style,
                self.text_color,
                TextAnchor::Middle,
                0.0,
                1.0,
                name.into(),
            ));
        }

        let mut front = Vec::new();
        let sent = self.sent(ui.seconds());
        let (count, into) = sent.unwrap_or((0, 0.0));
        let travel = cubic_in_out(progress(into, 0.0, self.step_time * self.travel));
        let settle = progress(into, 0.0, (self.step_time * 0.3).min(0.2));
        let loop_out = (spacing * 0.22).min(80.0);
        for (m, message) in self.messages.iter().enumerate().take(count) {
            let (from, to) = (message.from.min(n - 1), message.to.min(n - 1));
            let y = y_of(m);
            let (xa, xb) = (x_of(from), x_of(to));
            let points: Vec<Point> = if from == to {
                vec![
                    Point::new(xa, y - row * 0.2),
                    Point::new(xa + loop_out, y - row * 0.2),
                    Point::new(xa + loop_out, y + row * 0.2),
                    Point::new(xa + 2.0, y + row * 0.2),
                ]
            } else {
                vec![Point::new(xa, y), Point::new(xb, y)]
            };
            let current = m + 1 == count;
            let p = if current { f64::from(travel) } else { 1.0 };
            // the message being sent is lit; the one before it settles back
            let color = if current {
                self.active_color
            } else if m + 2 == count {
                mix(self.active_color, self.message_color, settle)
            } else {
                self.message_color
            };
            let drawn = partial(&points, p);
            let mut path = BezPath::new();
            path.move_to(drawn[0]);
            for &q in &drawn[1..] {
                path.line_to(q);
            }
            front.push(
                Shape {
                    dash: message.reply.then_some((12.0, 8.0)),
                    ..stroke(path, color, 3.0, 1.0)
                }
                .draw(),
            );
            let head = *drawn.last().unwrap_or(&points[0]);
            // the packet shrinks into the arrowhead over the last part of its travel
            let land = ((p - 0.8) / 0.2).clamp(0.0, 1.0);
            if land > 0.0 {
                let k = points.len();
                let dir = (points[k - 1] - points[k - 2]).normalize();
                front.push(tip(head, dir, 18.0 * land, color, 1.0).draw());
            }
            if land < 1.0 {
                front.push(
                    Shape {
                        path: kurbo::Shape::to_path(&Circle::new(head, 9.0 * (1.0 - land)), 0.1),
                        fill: Some(self.active_color),
                        stroke: None,
                        stroke_width: 0.0,
                        dash: None,
                        opacity: 1.0,
                    }
                    .draw(),
                );
            }
            let label = if self.numbers {
                format!("{}. {}", m + 1, message.label)
            } else {
                message.label.clone()
            };
            let (x, anchor, room) = if from == to {
                (
                    xa + loop_out + 14.0,
                    TextAnchor::Start,
                    spacing - loop_out - 40.0,
                )
            } else {
                (
                    f64::midpoint(xa, xb),
                    TextAnchor::Middle,
                    (xb - xa).abs() - 24.0,
                )
            };
            let label_y = if from == to {
                y + self.label_style.size as f64 * 0.35
            } else {
                y - 14.0
            };
            let opacity = if current {
                progress(into, 0.0, self.step_time * self.travel * 0.6)
            } else {
                1.0
            };
            front.push(line(
                x as f32,
                label_y as f32,
                self.label_style,
                mix(self.text_color, color, 0.5),
                anchor,
                0.0,
                opacity,
                fit_to(ui, self.label_style, &label, room).into(),
            ));
        }
        let parts: Svgr<'a> = vec![back.into(), front.into()].into();
        fade(ui, parts, self.enter.as_ref(), self.exit.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn messages_are_sent_one_a_step() {
        let diagram = SequenceDiagram {
            participants: vec!["client".into(), "server".into()],
            messages: vec![Message::new(0, 1, "GET /"), Message::reply(1, 0, "200 OK")],
            delay: 1.0,
            step_time: 1.0,
            ..SequenceDiagram::default()
        };
        assert_eq!(diagram.sent(0.5), None);
        assert_eq!(diagram.sent(1.25), Some((1, 0.25)));
        assert_eq!(diagram.sent(2.5), Some((2, 0.5)));
        assert_eq!(diagram.sent(9.0).map(|s| s.0), Some(2));
        assert!(diagram.messages[1].reply);
    }
}
