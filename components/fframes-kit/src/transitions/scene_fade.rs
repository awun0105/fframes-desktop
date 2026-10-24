use fframes::Svgr;

use crate::{
    anim::RateFunc,
    motion::clamp01,
    theme::Theme,
    ui::{Children, Component, Themed, Ui, no_children},
};

use super::{Presentation, Side, Timing};

/// Fades its children in at the start and out at the end of the enclosing `Sequence` or
/// scene (`ui.duration`). Without a known end only the fade in runs. A 0 length skips that
/// fade.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct SceneFade<'a> {
    /// Seconds of the fade in.
    pub fade_in: f32,
    /// Seconds of the fade out, ending with `ui.duration`.
    pub fade_out: f32,
    pub rate_func: RateFunc,
    pub children: Children<'a>,
}

impl Themed for SceneFade<'_> {
    /// Fades of the theme's `motion.duration.fast` with its standard easing, no children.
    fn themed(t: &Theme) -> Self {
        Self {
            fade_in: t.motion.duration.fast,
            fade_out: t.motion.duration.fast,
            rate_func: t.motion.easing.standard,
            children: no_children(),
        }
    }
}

impl<'a> SceneFade<'a> {
    /// Fades of the theme's `motion.duration.fast` with its standard easing.
    pub fn new(ui: &Ui<'a, '_, '_>, children: Children<'a>) -> Self {
        Self {
            children,
            ..Self::themed(ui.theme)
        }
    }

    /// Opacity at `t` seconds of a container that lasts `length` seconds.
    pub fn opacity(&self, t: f32, length: Option<f32>) -> f32 {
        let ramp = |t: f32, run_time: f32| {
            if run_time <= 0.0 {
                1.0
            } else {
                self.rate_func.apply(clamp01(t / run_time), run_time)
            }
        };
        let fade_in = ramp(t, self.fade_in);
        let fade_out = length.map_or(1.0, |length| ramp(length - t, self.fade_out));
        clamp01(fade_in.min(fade_out))
    }
}

impl<'a> Component<'a> for SceneFade<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let opacity = self.opacity(ui.seconds(), ui.duration_seconds());
        let content = (self.children)(ui);
        if opacity >= 1.0 {
            return content;
        }
        fframes::svgr!(<g opacity={opacity}>{content}</g>)
    }
}

/// Applies `presentation` to the content of a core `Scene` that declares an
/// `Overlap`, so two scenes cross-fade, slide or wipe without a [`TransitionSeries`](super::TransitionSeries).
/// Build the scene's `Ui` with [`Ui::for_scene`] so `ui.duration` covers the overlap.
///
/// - `Side::Entering` runs over the first `timing.duration()` seconds of the scene: give it
///   to the scene with `Overlap::Previous(d)` of the same length.
/// - `Side::Exiting` runs over the last `timing.duration()` seconds: give it to the scene
///   before, which then ends under the next one.
///
/// `key` names the transition's clip path, as the key of a component does; scenes on screen
/// at once need different keys.
pub fn scene_transition<'a>(
    ui: &Ui<'a, '_, '_>,
    presentation: &impl Presentation,
    timing: Timing,
    side: Side,
    key: Option<&'static str>,
    content: Svgr<'a>,
) -> Svgr<'a> {
    let length = timing.duration();
    let t = match side {
        Side::Entering => ui.seconds(),
        Side::Exiting => match ui.duration_seconds() {
            Some(end) => ui.seconds() - (end - length),
            None => return content,
        },
    };
    if length <= 0.0 || t < 0.0 || t >= length {
        return content;
    }
    let id = ui.id(key, 0, "scene-transition");
    let progress = timing.progress(t, length);
    // presentations that count in seconds read the length from the `Ui`
    let mut ui = ui.shifted(0, ui.duration);
    let state = &mut ui.transition;
    match side {
        Side::Entering => {
            state.progress.entering = progress;
            state.entering_seconds = Some(length);
        }
        Side::Exiting => {
            state.progress.exiting = progress;
            state.exiting_seconds = Some(length);
        }
    }
    presentation.apply(progress, side, content, &ui, &id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fade(fade_in: f32, fade_out: f32) -> SceneFade<'static> {
        SceneFade {
            fade_in,
            fade_out,
            rate_func: RateFunc::Linear,
            children: crate::ui::children(|_| Svgr::empty()),
        }
    }

    #[test]
    fn fades_at_both_edges() {
        let f = fade(0.5, 1.0);
        assert_eq!(f.opacity(0.0, Some(4.0)), 0.0);
        assert!((f.opacity(0.25, Some(4.0)) - 0.5).abs() < 1e-6);
        assert_eq!(f.opacity(2.0, Some(4.0)), 1.0);
        assert!((f.opacity(3.5, Some(4.0)) - 0.5).abs() < 1e-6);
        assert_eq!(f.opacity(4.0, Some(4.0)), 0.0);
        // no end known: only the fade in
        assert_eq!(f.opacity(100.0, None), 1.0);
        assert_eq!(fade(0.0, 0.0).opacity(0.0, Some(1.0)), 1.0);
    }
}
