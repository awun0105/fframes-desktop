use std::borrow::Cow;

use fframes::{Color, Svgr};

use crate::{
    anim::RateFunc,
    motion::progress,
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

/// An annotation: a dot on the `target`, an elbow line to `label` (first across, then up or
/// down) and the text at its end. The line draws on over `run_time` seconds from `delay`;
/// the text fades in over the last 40 %. (The shape called `Callout` is the Remotion speech
/// bubble, so this one has another name.)
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Annotation<'a> {
    pub target: (f32, f32),
    pub label: (f32, f32),
    pub text: Cow<'a, str>,
    pub color: Color,
    pub style: TextStyle,
    pub letter_spacing: f32,
    pub stroke_width: f32,
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
}

impl Themed for Annotation<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            target: (0.0, 0.0),
            label: (0.0, 0.0),
            text: Cow::Borrowed(""),
            color: t.colors.accent,
            style: t.typography.caption,
            letter_spacing: 2.0,
            stroke_width: t.stroke.thin,
            delay: 0.0,
            run_time: t.motion.duration.slow,
            rate_func: t.motion.easing.standard,
        }
    }
}

impl Default for Annotation<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Annotation<'a> {
    /// The annotation `progress` (0 to 1) of the way drawn.
    pub fn draw(self, progress: f32) -> Svgr<'a> {
        if progress <= 0.0 {
            return Svgr::empty();
        }
        let p = progress.clamp(0.0, 1.0);
        let ((tx, ty), (lx, ly)) = (self.target, self.label);
        // first leg across to the label's x, then up or down to it
        let l1 = (lx - tx).abs() + 0.01;
        let l2 = (ly - ty).abs() + 0.01;
        let total = l1 + l2;
        let drawn = total * p;
        let path = format!("M{tx} {ty} H{lx} V{ly}");
        let dash = format!("{drawn} {total}");
        let right = lx >= tx;
        let anchor = if right { "start" } else { "end" };
        let text_x = if right { lx + 10.0 } else { lx - 10.0 };
        let text_opacity = ((p - 0.6) / 0.4).clamp(0.0, 1.0);
        let (c, s) = (self.color, self.style);
        let text = self.text.into_owned();
        fframes::svgr!(
            <g>
                <circle cx={tx} cy={ty} r="5" fill={c} opacity={p.min(1.0)} />
                <circle cx={tx} cy={ty} r="12" fill="none" stroke={c} stroke-width={self.stroke_width} opacity={p * 0.6} />
                <path d={path} fill="none" stroke={c} stroke-width={self.stroke_width} stroke-dasharray={dash} />
                <text x={text_x} y={ly + 6.0} font-family={s.family} font-weight={s.weight} font-size={s.size}
                      letter-spacing={self.letter_spacing} fill={c} text-anchor={anchor} opacity={text_opacity}>{text}</text>
            </g>
        )
    }
}

impl<'a> Component<'a> for Annotation<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let p = progress(ui.seconds(), self.delay, self.delay + self.run_time);
        let p = self.rate_func.apply(p, self.run_time);
        self.draw(p)
    }
}
