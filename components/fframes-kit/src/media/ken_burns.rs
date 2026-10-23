use fframes::Svgr;

use crate::{
    motion::progress,
    theme::Theme,
    ui::{Component, Themed, Ui},
};

/// An image of the video's media slowly pushed in from `from` seconds and drifted sideways,
/// clipped to the box `(x, y, width, height)`, which it covers (`xMidYMid slice`). It fades
/// in over `fade` seconds. The scale is `zoom + zoom_per_second * t` and the center moves
/// `drift / 4` pixels a second, `t` seconds after `from`. A missing image draws nothing;
/// `key` names the clip path when two are drawn in one frame.
#[derive(Debug, Clone, PartialEq)]
pub struct KenBurns<'a> {
    pub image: &'a str,
    pub key: Option<&'a str>,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub from: f32,
    pub zoom: f32,
    pub zoom_per_second: f32,
    pub drift: f32,
    pub fade: f32,
}

impl Themed for KenBurns<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            image: "",
            key: None,
            x: 0.0,
            y: 0.0,
            width: 1920.0,
            height: 1080.0,
            from: 0.0,
            zoom: 1.06,
            zoom_per_second: 0.02,
            drift: 0.0,
            fade: t.motion.duration.base,
        }
    }
}

impl Default for KenBurns<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl KenBurns<'_> {
    /// `(scale, center x)` at `t` seconds of the container.
    pub fn at(&self, t: f32) -> (f32, f32) {
        let k = (t - self.from).max(0.0);
        let scale = self.zoom + self.zoom_per_second * k;
        (scale, self.x + self.width / 2.0 + self.drift * k / 4.0)
    }
}

impl<'a> Component<'a> for KenBurns<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let t = ui.seconds();
        if t < self.from {
            return Svgr::empty();
        }
        let Some(image) = ui.ctx.get_image(self.image) else {
            return Svgr::empty();
        };
        let (x, y, w, h) = (self.x, self.y, self.width, self.height);
        let (scale, cx) = self.at(t);
        let cy = y + h / 2.0;
        let id = ui.id(self.key, 0, "ken-burns");
        let clip = format!("url(#{id})");
        let opacity = progress(t, self.from, self.from + self.fade);
        let transform = format!(
            "translate({cx} {cy}) scale({scale}) translate({} {})",
            -w / 2.0,
            -h / 2.0
        );
        fframes::svgr!(
            <g>
                <defs><clipPath id={id}><rect x={x} y={y} width={w} height={h} /></clipPath></defs>
                <g clip-path={clip} opacity={opacity}>
                    <g transform={transform}>
                        <image href={image.href()} x="0" y="0" width={w} height={h} preserveAspectRatio="xMidYMid slice" />
                    </g>
                </g>
            </g>
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_pushes_in_and_drifts_after_the_start() {
        let k = KenBurns {
            from: 1.0,
            drift: 40.0,
            ..KenBurns::default()
        };
        assert_eq!(k.at(0.0), (1.06, 960.0));
        let (scale, cx) = k.at(3.0);
        assert!((scale - 1.10).abs() < 1e-6);
        assert_eq!(cx, 980.0);
    }
}
