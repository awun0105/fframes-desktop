use crate::{
    anim::RateFunc,
    geometry::{Look, Shape},
    motion::progress,
    ui::Ui,
};

use super::{DrawBorderThenFill, ShapeAnimation, create::themed};

/// `DrawBorderThenFill` on each part in turn (Manim's `Write`): the glyphs of a
/// [`TextPath`](crate::shapes::TextPath), or a shape as one part.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Write {
    pub delay: f32,
    pub run_time: f32,
    pub rate_func: RateFunc,
    /// Fraction of a part's time after which the next part starts. `None` is Manim's
    /// default, `min(4 / parts, 0.2)`.
    pub lag_ratio: Option<f32>,
}

themed!(Write, |t| Write {
    delay: 0.0,
    run_time: 1.0,
    rate_func: t.motion.easing.standard,
    lag_ratio: None,
});

impl Write {
    /// Start and length in seconds of part `i` of `n`.
    pub(crate) fn window(&self, i: usize, n: usize) -> (f32, f32) {
        let n = n.max(1) as f32;
        let lag = self.lag_ratio.unwrap_or((4.0 / n).min(0.2));
        let each = self.run_time / (1.0 + (n - 1.0) * lag);
        (self.delay + i as f32 * lag * each, each)
    }
}

impl ShapeAnimation for Write {
    fn look(&self, ui: &Ui<'_, '_, '_>, shape: &Shape, look: &mut Look) {
        self.look_part(ui, shape, shape, 0, 1, look);
    }

    fn per_part(&self) -> bool {
        true
    }

    /// `DrawBorderThenFill` in the part's own window; the temporary border is as wide as the
    /// first part's outline (`whole` starts as the first part), at least 2.
    fn look_part(
        &self,
        ui: &Ui<'_, '_, '_>,
        whole: &Shape,
        part: &Shape,
        index: usize,
        count: usize,
        look: &mut Look,
    ) {
        let (start, each) = self.window(index, count);
        let p = self
            .rate_func
            .apply(progress(ui.seconds(), start, start + each), each);
        let border = DrawBorderThenFill {
            delay: 0.0,
            run_time: self.run_time,
            rate_func: self.rate_func,
            stroke_width: whole.stroke_width.max(2.0),
        };
        border.at(p, part, look);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parts_share_the_run_time() {
        let w = Write {
            lag_ratio: Some(0.5),
            ..Write::default()
        };
        // 3 parts: each lasts 1 / (1 + 2 * 0.5) = 0.5s, starting 0.25s apart
        assert_eq!(w.window(0, 3), (0.0, 0.5));
        assert_eq!(w.window(2, 3), (0.5, 0.5));
        let (start, each) = w.window(2, 3);
        assert!((start + each - w.run_time).abs() < 1e-6);
        assert_eq!(Write::default().window(0, 1), (0.0, 1.0));
    }
}
