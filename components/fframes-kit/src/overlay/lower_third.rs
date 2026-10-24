use std::borrow::Cow;

use fframes::{Color, Svgr, Transform};

use crate::{
    anim::RateFunc,
    motion::progress,
    text::{TextAnchor, line},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

/// A name and a role next to an accent bar, the caption of a speaker. The bar grows first,
/// then the name and the role slide in from behind it, `stagger` seconds apart, and all of
/// it leaves in reverse order before the end of the container. `(x, y)` is the top left of
/// the bar. With `background`, a plate as wide as the longer line sits behind the text
/// (without a measurable font it is `min_width` wide).
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct LowerThird<'a> {
    pub name: Cow<'a, str>,
    pub role: Cow<'a, str>,
    pub x: f32,
    pub y: f32,
    pub name_style: TextStyle,
    pub role_style: TextStyle,
    pub name_fill: Color,
    pub role_fill: Color,
    pub bar: Color,
    pub bar_width: f32,
    pub background: Option<Color>,
    pub min_width: f32,
    /// Space between the bar and the text, and around the text on the plate.
    pub padding: f32,
    pub delay: f32,
    pub stagger: f32,
    pub run_time: f32,
    /// Seconds of the exit, which ends with the container; 0 never leaves.
    pub exit: f32,
    pub rate_func: RateFunc,
}

impl Themed for LowerThird<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            name: Cow::Borrowed(""),
            role: Cow::Borrowed(""),
            x: 96.0,
            y: 820.0,
            name_style: t.typography.heading,
            role_style: t.typography.body,
            name_fill: t.colors.title,
            role_fill: t.colors.muted,
            bar: t.colors.accent,
            bar_width: 6.0,
            background: None,
            min_width: 400.0,
            padding: t.spacing.md,
            delay: 0.0,
            stagger: t.motion.stagger.loose,
            run_time: t.motion.duration.enter,
            exit: t.motion.duration.exit,
            rate_func: t.motion.easing.enter,
        }
    }
}

impl Default for LowerThird<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl LowerThird<'_> {
    /// Height of the bar: the two lines and the padding around them.
    pub fn height(&self) -> f32 {
        let line = |s: TextStyle| s.size as f32 * s.line_height;
        line(self.name_style) + line(self.role_style) + self.padding
    }

    /// How far part `i` (0 bar, 1 name, 2 role) is in at `t`, and how far it is out.
    fn part(&self, i: usize, t: f32, end: Option<f32>) -> f32 {
        let start = self.delay + i as f32 * self.stagger;
        let entered = self
            .rate_func
            .apply(progress(t, start, start + self.run_time), self.run_time);
        let left = match end {
            Some(end) if self.exit > 0.0 => {
                // the role leaves first, the bar last
                let stop = end - (2 - i) as f32 * self.stagger * 0.5;
                progress(t, stop - self.exit, stop)
            }
            _ => 0.0,
        };
        entered.min(1.0) * (1.0 - left)
    }
}

impl<'a> Component<'a> for LowerThird<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let (t, end) = (ui.seconds(), ui.duration_seconds());
        let (bar, name, role) = (
            self.part(0, t, end),
            self.part(1, t, end),
            self.part(2, t, end),
        );
        if bar <= 0.0 {
            return Svgr::empty();
        }
        let height = self.height();
        let text_x = self.x + self.bar_width + self.padding;
        let name_y = self.y + self.padding * 0.5 + self.name_style.size as f32;
        let role_y = name_y + self.role_style.size as f32 * self.role_style.line_height;
        let plate = self.background.map_or_else(Svgr::empty, |fill| {
            let text_width = ui
                .measure(self.name_style.font(), &self.name)
                .zip(ui.measure(self.role_style.font(), &self.role))
                .map_or(self.min_width, |(a, b)| a.max(b).max(self.min_width));
            let width = (text_width + self.padding * 2.0) * bar;
            fframes::svgr!(<rect x={self.x} y={self.y} width={width} height={height} fill={fill} />)
        });
        // each line slides out from behind the bar, clipped at its edge by fading in
        let slide = |shown: f32, text: Svgr<'a>| {
            if shown <= 0.0 {
                return Svgr::empty();
            }
            let offset = Transform::translate(-40.0 * (1.0 - shown), 0.0);
            fframes::svgr!(<g transform={offset} opacity={shown}>{text}</g>)
        };
        let name_line = line(
            text_x,
            name_y,
            self.name_style,
            self.name_fill,
            TextAnchor::Start,
            0.0,
            1.0,
            self.name,
        );
        let role_line = line(
            text_x,
            role_y,
            self.role_style,
            self.role_fill,
            TextAnchor::Start,
            0.0,
            1.0,
            self.role,
        );
        let name_line = slide(name, name_line);
        let role_line = slide(role, role_line);
        let bar_height = height * bar;
        fframes::svgr!(
            <g>
                {plate}
                <rect x={self.x} y={self.y} width={self.bar_width} height={bar_height} fill={self.bar} />
                {name_line}
                {role_line}
            </g>
        )
    }
}
