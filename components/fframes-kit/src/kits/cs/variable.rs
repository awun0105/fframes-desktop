//! A named variable whose value changes step by step, manim-dsa's `MVariable`.

use fframes::{Color, Svgr};
use kurbo::RoundedRect;

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
    keyed::{mix, sample},
};

/// A variable as manim-dsa draws it (`MVariable` with `add_label`): its `name` above a box
/// with its value, centred on `center` (units of `ui.space`). The box is `width` by `height`
/// pixels and widens to fit a longer value. `values` holds the value of each step, the first
/// one at the start; the value changes every `step_time` seconds from `delay`, like the steps
/// of the other figures, so it can follow a trace in step. On a change the old value rises
/// out of the box, the new one rises in, the box flashes `colors.compare` and resizes. The
/// whole figure fades with `enter` and `exit`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct VariableBox<'a> {
    pub name: &'a str,
    pub values: Vec<String>,
    pub delay: f32,
    pub step_time: f32,
    pub center: (f64, f64),
    pub width: f32,
    pub height: f32,
    pub colors: CellColors,
    pub name_color: Color,
    pub value_style: TextStyle,
    pub name_style: TextStyle,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for VariableBox<'_> {
    fn themed(t: &Theme) -> Self {
        let mut value_style = t.typography.body;
        value_style.weight = 600;
        Self {
            name: "",
            values: Vec::new(),
            delay: 0.0,
            step_time: 0.8,
            center: (0.0, 0.0),
            width: 120.0,
            height: 80.0,
            colors: CellColors::themed(t),
            name_color: t.colors.text,
            value_style,
            name_style: t.typography.caption,
            enter: None,
            exit: None,
        }
    }
}

impl Default for VariableBox<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl VariableBox<'_> {
    /// The value `seconds` into the video: the first value before `delay`, the last after the
    /// last step. `None` without values.
    pub fn value_at(&self, seconds: f32) -> Option<&str> {
        sample(&self.values, seconds - self.delay, self.step_time).map(|at| at.step.as_str())
    }
}

impl<'a> Component<'a> for VariableBox<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let Some(at) = sample(&self.values, ui.seconds() - self.delay, self.step_time) else {
            return Svgr::empty();
        };
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let h = f64::from(self.height);
        let size = self.value_style.size as f64;
        let fits = |value: &str| {
            let text = ui
                .measure(self.value_style.font(), value)
                .map_or(value.chars().count() as f64 * size * 0.6, f64::from);
            f64::from(self.width).max(text + size * 1.2)
        };
        let blend = (self.step_time * 0.3).min(0.2);
        let q = f64::from(cubic_in_out(progress(at.into, 0.0, self.step_time * 0.6)));
        let changed = at.previous != at.step;
        let w = fits(at.previous) + (fits(at.step) - fits(at.previous)) * q;
        // the flash rises at the start of a change and is gone before the next step
        let flash = if changed {
            progress(at.into, 0.0, blend)
                - progress(at.into, self.step_time * 0.5, self.step_time * 0.9)
        } else {
            0.0
        };
        let fill = mix(self.colors.base, self.colors.compare, flash);
        let rect = RoundedRect::new(
            middle.x - w / 2.0,
            middle.y - h / 2.0,
            middle.x + w / 2.0,
            middle.y + h / 2.0,
            h * 0.14,
        );
        let body = Shape {
            path: kurbo::Shape::to_path(&rect, 0.1),
            fill: Some(fill),
            stroke: Some(self.colors.text.with_alpha(64)),
            stroke_width: 1.5,
            dash: None,
            opacity: 1.0,
        };
        let value = |text: &str, rise: f64, opacity: f64| {
            if opacity <= 0.0 {
                return Svgr::empty();
            }
            line(
                middle.x as f32,
                (middle.y + size * 0.35 - rise) as f32,
                self.value_style,
                self.colors.text_on(fill),
                TextAnchor::Middle,
                0.0,
                opacity as f32,
                text.to_owned().into(),
            )
        };
        // the old value rises out in the first half, the new one rises in during the second
        let values = if changed {
            fframes::svgr!(<g>
                {value(at.previous, q * h * 0.5, 1.0 - 2.0 * q)}
                {value(at.step, (q - 1.0) * h * 0.5, 2.0 * q - 1.0)}
            </g>)
        } else {
            value(at.step, 0.0, 1.0)
        };
        let name = if self.name.is_empty() {
            Svgr::empty()
        } else {
            line(
                middle.x as f32,
                (middle.y - h / 2.0 - 14.0) as f32,
                self.name_style,
                self.name_color,
                TextAnchor::Middle,
                0.0,
                1.0,
                self.name.into(),
            )
        };
        let parts: Vec<Svgr<'a>> = vec![body.draw(), values, name];
        fade(ui, parts.into(), self.enter.as_ref(), self.exit.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_value_follows_the_steps() {
        let variable = VariableBox {
            name: "sum",
            values: ["0", "3", "8", "8", "15"].map(String::from).to_vec(),
            delay: 1.0,
            step_time: 0.5,
            ..VariableBox::default()
        };
        let at = |t: f32| variable.value_at(t).unwrap();
        assert_eq!(
            [
                at(0.0),
                at(1.0),
                at(1.6),
                at(2.2),
                at(2.6),
                at(3.0),
                at(99.0)
            ],
            ["0", "0", "3", "8", "8", "15", "15"]
        );
        assert_eq!(VariableBox::default().value_at(1.0), None);
    }
}
