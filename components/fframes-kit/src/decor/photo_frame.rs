use fframes::{Color, Svgr};

use crate::{
    theme::Theme,
    ui::{Component, Themed, Ui},
};

/// An image of the video's media in a stroked frame with rounded corners; the image is
/// clipped to the frame and covers it (`xMidYMid slice`). A missing image draws the frame
/// alone. `key` names the clip path when two frames are drawn in one frame.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct PhotoFrame<'a> {
    pub image: &'a str,
    pub key: Option<&'a str>,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub radius: f32,
    pub stroke: Color,
    pub stroke_width: f32,
    pub opacity: f32,
}

impl Themed for PhotoFrame<'_> {
    fn themed(t: &Theme) -> Self {
        Self {
            image: "",
            key: None,
            x: 0.0,
            y: 0.0,
            width: 480.0,
            height: 320.0,
            radius: t.radius.card,
            stroke: t.colors.title,
            stroke_width: 12.0,
            opacity: 1.0,
        }
    }
}

impl Default for PhotoFrame<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for PhotoFrame<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let (x, y, w, h, r) = (self.x, self.y, self.width, self.height, self.radius);
        let photo = ui.ctx.get_image(self.image).map_or_else(Svgr::empty, |image| {
            let id = ui.id(self.key, 0, "photo-frame");
            let clip = format!("url(#{id})");
            fframes::svgr!(
                <g>
                    <defs><clipPath id={id}><rect x={x} y={y} width={w} height={h} rx={r} /></clipPath></defs>
                    <image href={image.href()} x={x} y={y} width={w} height={h} clip-path={clip}
                           preserveAspectRatio="xMidYMid slice" />
                </g>
            )
        });
        fframes::svgr!(
            <g opacity={self.opacity}>
                {photo}
                <rect x={x} y={y} width={w} height={h} rx={r} fill="none" stroke={self.stroke} stroke-width={self.stroke_width} />
            </g>
        )
    }
}
