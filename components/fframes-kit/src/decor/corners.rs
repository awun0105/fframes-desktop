use fframes::{Color, Svgr};
use kurbo::BezPath;

use crate::{
    animate::{EnterAnim, draw_animated},
    geometry::{Geometry, Shape},
    theme::Theme,
    ui::{Component, Themed, Ui},
};

/// Crop-mark corners around the rectangle `(x, y, width, height)`: an L of `arm` pixels at
/// each corner, like the brackets around a viewfinder. With `enter` (such as `Create`) the
/// marks draw on; without, they are one static path.
#[derive(Debug, Clone, PartialEq)]
pub struct CornerMarks {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub arm: f32,
    pub color: Color,
    pub stroke_width: f32,
    pub enter: Option<EnterAnim>,
}

impl Themed for CornerMarks {
    fn themed(t: &Theme) -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            width: 0.0,
            height: 0.0,
            arm: 24.0,
            color: t.colors.muted,
            stroke_width: t.stroke.thin,
            enter: None,
        }
    }
}

impl Default for CornerMarks {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl CornerMarks {
    /// The four marks as SVG path data, clockwise from the top left.
    pub fn d(&self) -> String {
        format!(
            "M{x} {y1} V{y} H{x1} M{x2} {y} H{x3} V{y1} M{x3} {y2} V{y3} H{x2} M{x1} {y3} H{x} V{y2}",
            x = self.x,
            y = self.y,
            x1 = self.x + self.arm,
            x2 = self.x + self.width - self.arm,
            x3 = self.x + self.width,
            y1 = self.y + self.arm,
            y2 = self.y + self.height - self.arm,
            y3 = self.y + self.height,
        )
    }

    /// The static marks (`enter` is ignored), as one `<path>`; it needs no `Ui`.
    pub fn draw<'a>(&self) -> Svgr<'a> {
        let d = self.d();
        fframes::svgr!(<path d={d} fill="none" stroke={self.color} stroke-width={self.stroke_width} />)
    }
}

impl Geometry for CornerMarks {
    fn path(&self) -> BezPath {
        BezPath::from_svg(&self.d()).unwrap_or_default()
    }
}

impl<'a> Component<'a> for CornerMarks {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        match &self.enter {
            None => self.draw(),
            Some(enter) => {
                let shape = Shape {
                    path: self.path(),
                    fill: None,
                    stroke: Some(self.color),
                    stroke_width: self.stroke_width,
                    dash: None,
                    opacity: 1.0,
                };
                draw_animated(ui, &[shape], Some(enter), None, None)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_marks_span_the_rectangle() {
        let marks = CornerMarks {
            x: 10.0,
            y: 20.0,
            width: 100.0,
            height: 50.0,
            arm: 8.0,
            ..CornerMarks::default()
        };
        assert!(marks.d().starts_with("M10 28 V20 H18 M102 20 H110"));
        crate::shapes::assert_bbox(&marks, 10.0, 20.0, 110.0, 70.0);
    }
}
