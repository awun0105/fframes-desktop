use crate::{
    anim::RateFunc,
    geometry::{Look, Shape},
    motion::progress,
    ui::Ui,
};

use super::{ShapeAnimation, create::themed};

/// The run time of `count` parts: `run_time`, or `time_per_char` each (Manim's
/// `max(0.06, time_per_char) * len(text)`).
fn letters_time(run_time: Option<f32>, time_per_char: f32, count: usize) -> f32 {
    run_time.unwrap_or(time_per_char.max(0.06) * count as f32)
}

/// Parts shown at eased progress `p` of `count`, each at once (Manim rounds up).
fn shown(p: f32, count: usize) -> usize {
    ((p * count as f32 - 1e-4).ceil().max(0.0) as usize).min(count)
}

/// Shows the glyphs of a [`TextPath`](crate::shapes::TextPath) one after another, each at
/// once (Manim's `AddTextLetterByLetter`). For a `<text>` line it is a
/// [`Typewriter`](crate::text::Typewriter) with `cursor: None`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AddTextLetterByLetter {
    /// Seconds per glyph when `run_time` is `None`.
    pub time_per_char: f32,
    pub delay: f32,
    /// `None` is `time_per_char` per glyph.
    pub run_time: Option<f32>,
    pub rate_func: RateFunc,
}

themed!(AddTextLetterByLetter, |_t| AddTextLetterByLetter {
    time_per_char: 0.1,
    delay: 0.0,
    run_time: None,
    rate_func: RateFunc::Linear,
});

impl ShapeAnimation for AddTextLetterByLetter {
    fn look(&self, ui: &Ui<'_, '_, '_>, shape: &Shape, look: &mut Look) {
        self.look_part(ui, shape, shape, 0, 1, look);
    }

    fn per_part(&self) -> bool {
        true
    }

    fn look_part(
        &self,
        ui: &Ui<'_, '_, '_>,
        _whole: &Shape,
        _part: &Shape,
        index: usize,
        count: usize,
        look: &mut Look,
    ) {
        let run_time = letters_time(self.run_time, self.time_per_char, count);
        let t = progress(ui.seconds(), self.delay, self.delay + run_time);
        if index >= shown(self.rate_func.apply(t, run_time), count) {
            look.opacity = 0.0;
        }
    }
}

/// Removes the glyphs of a [`TextPath`](crate::shapes::TextPath) from the last one back, each
/// at once, ending `delay` seconds before the end of the container (Manim's
/// `RemoveTextLetterByLetter`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RemoveTextLetterByLetter {
    pub time_per_char: f32,
    /// Seconds between the end of the animation and the end of `ui.duration`.
    pub delay: f32,
    pub run_time: Option<f32>,
    pub rate_func: RateFunc,
}

themed!(RemoveTextLetterByLetter, |_t| RemoveTextLetterByLetter {
    time_per_char: 0.1,
    delay: 0.0,
    run_time: None,
    rate_func: RateFunc::Linear,
});

impl ShapeAnimation for RemoveTextLetterByLetter {
    fn look(&self, ui: &Ui<'_, '_, '_>, shape: &Shape, look: &mut Look) {
        self.look_part(ui, shape, shape, 0, 1, look);
    }

    fn per_part(&self) -> bool {
        true
    }

    fn look_part(
        &self,
        ui: &Ui<'_, '_, '_>,
        _whole: &Shape,
        _part: &Shape,
        index: usize,
        count: usize,
        look: &mut Look,
    ) {
        let Some(end) = ui.duration_seconds() else {
            return;
        };
        let run_time = letters_time(self.run_time, self.time_per_char, count);
        let stop = end - self.delay;
        let t = progress(ui.seconds(), stop - run_time, stop);
        let removed = shown(self.rate_func.apply(t, run_time), count);
        if index + removed >= count {
            look.opacity = 0.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_appear_one_per_time_step() {
        assert_eq!(letters_time(None, 0.1, 5), 0.5);
        assert!((letters_time(None, 0.01, 10) - 0.6).abs() < 1e-6);
        assert_eq!(letters_time(Some(2.0), 0.1, 5), 2.0);
        assert_eq!(shown(0.0, 5), 0);
        // just after the start the first letter is on
        assert_eq!(shown(0.01, 5), 1);
        assert_eq!(shown(0.2, 5), 1);
        assert_eq!(shown(0.21, 5), 2);
        assert_eq!(shown(1.0, 5), 5);
    }
}
