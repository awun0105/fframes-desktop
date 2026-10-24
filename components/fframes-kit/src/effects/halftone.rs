use std::fmt::Write;

use fframes::{Color, Svgr};

use super::{
    Region, filtered,
    patterns::{coarsen, cover, draw_over, masked, steps},
};
use crate::{
    motion::{clamp01, lerp},
    theme::Theme,
    ui::{Children, Component, Themed, Ui, no_children},
};

/// The shape of a [`Halftone`] dot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HalftoneShape {
    #[default]
    Circle,
    Square,
    /// Stripes as long as the area and as thick as a dot.
    Line,
}

/// Where [`Halftone`] and [`HalftoneLinearGradient`] dots take their colour from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HalftoneColor {
    /// The `dot_color`.
    #[default]
    Solid,
    /// The children under the dot.
    Source,
}

/// Circles `(x, y, r)` as one path.
fn circles(parts: impl IntoIterator<Item = (f32, f32, f32)>) -> String {
    let mut d = String::new();
    for (x, y, r) in parts {
        let _ = write!(
            d,
            "M{} {y}a{r} {r} 0 1 0 {} 0a{r} {r} 0 1 0 {} 0Z",
            x - r,
            2.0 * r,
            -2.0 * r
        );
    }
    d
}

/// Its children printed as a grid of dots (Remotion's `halftone()`): a dot every
/// `dot_spacing` units (`dot_size` when `None`), as large as `dot_size` where the children
/// are black and shrinking to nothing where they are white or transparent (the other way
/// round with `invert`). The grid is centred on the area, turned by `rotation` degrees
/// clockwise and moved by `offset`. Dots are `dot_color` or, with [`HalftoneColor::Source`],
/// the children's colours; they replace the children.
///
/// Remotion sizes each dot by the brightness at its middle; here every pixel compares its
/// own brightness with a field of gradients that peaks in the dot middles, so dots on an
/// edge in the picture take the edge's shape. A spacing below the dot size draws later dots
/// over earlier ones.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Halftone<'a> {
    pub shape: HalftoneShape,
    pub dot_size: f32,
    pub dot_spacing: Option<f32>,
    pub rotation: f32,
    pub offset: (f32, f32),
    pub color: HalftoneColor,
    pub dot_color: Color,
    pub invert: bool,
    pub area: (f32, f32),
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

impl Themed for Halftone<'_> {
    /// Remotion's defaults: red circles 20 units wide.
    fn themed(_: &Theme) -> Self {
        Self {
            shape: HalftoneShape::Circle,
            dot_size: 20.0,
            dot_spacing: None,
            rotation: 0.0,
            offset: (0.0, 0.0),
            color: HalftoneColor::Solid,
            dot_color: Color::rgb(0xff, 0, 0),
            invert: false,
            area: (1920.0, 1080.0),
            key: None,
            children: no_children(),
        }
    }
}

impl Halftone<'_> {
    /// The left, top, right and bottom of what the grid must cover before it is turned:
    /// the area, or the square that still covers it turned.
    fn bounds(&self, spacing: f32) -> (f32, f32, f32, f32) {
        let (w, h) = self.area;
        if self.rotation == 0.0 {
            return (0.0, 0.0, w, h);
        }
        let (cx, cy, half) = cover(self.area, spacing);
        (cx - half, cy - half, cx + half, cy + half)
    }

    /// The dot size and spacing, scaled up together so the circles stay below
    /// [`MAX_PATTERN_SHAPES`](super::MAX_PATTERN_SHAPES).
    fn grid(&self) -> (f32, f32) {
        let size = self.dot_size.max(1.0);
        let spacing = self.dot_spacing.unwrap_or(size).max(1.0);
        if self.shape != HalftoneShape::Circle {
            return (size, spacing);
        }
        let (left, top, right, bottom) = self.bounds(spacing);
        let scale = coarsen((right - left) * (bottom - top) / (spacing * spacing), 2);
        (size * scale, spacing * scale)
    }

    /// The field the brightness is compared with: 1 in the middle of every dot falling to 0
    /// at its full size, drawn black between the dots. `id` names its gradients.
    fn field(&self, id: &str) -> Svgr<'static> {
        let (size, spacing) = self.grid();
        let (cx, cy) = (self.area.0 / 2.0, self.area.1 / 2.0);
        let (start_x, start_y) = (cx - self.offset.0, cy - self.offset.1);
        let (left, top, right, bottom) = self.bounds(spacing);
        let (width, height) = (right - left, bottom - top);
        let radius = size / 2.0;
        // a ramp from a dot middle to halfway to the next, mirrored from there
        let ramp = |name: String, (x1, y1, x2, y2): (f32, f32, f32, f32)| {
            let reach = radius / (spacing / 2.0);
            let (end, rest) = if reach <= 1.0 {
                (reach, 0.0)
            } else {
                (1.0, 1.0 - 1.0 / reach)
            };
            let grey = (rest * 255.0).round() as u8;
            let rest = Color::rgb(grey, grey, grey);
            fframes::svgr!(
                <linearGradient id={name} gradientUnits="userSpaceOnUse" spreadMethod="reflect"
                                x1={x1} y1={y1} x2={x2} y2={y2}>
                    <stop offset="0" stop-color="#ffffff" />
                    <stop offset={end} stop-color={rest} />
                </linearGradient>
            )
        };
        let across = format!("{id}-across");
        let down = format!("{id}-down");
        let across_fill = format!("url(#{across})");
        let down_fill = format!("url(#{down})");
        let shapes = match self.shape {
            HalftoneShape::Circle => {
                let dot = format!("{id}-dot");
                let dot_fill = format!("url(#{dot})");
                let dots: Vec<Svgr<'static>> = steps(top, bottom, start_y, spacing)
                    .flat_map(|j| {
                        steps(left, right, start_x, spacing).map(move |i| {
                            (start_x + i as f32 * spacing, start_y + j as f32 * spacing)
                        })
                    })
                    .map(|(x, y)| {
                        let fill = dot_fill.clone();
                        fframes::svgr!(<circle cx={x} cy={y} r={radius} fill={fill} />)
                    })
                    .collect();
                fframes::svgr!(
                    <g>
                        <defs>
                            <radialGradient id={dot}>
                                <stop offset="0" stop-color="#ffffff" />
                                <stop offset="1" stop-color="#000000" />
                            </radialGradient>
                        </defs>
                        <rect x={left} y={top} width={width} height={height} fill="#000000" />
                        {Svgr::from(dots)}
                    </g>
                )
            }
            HalftoneShape::Square => fframes::svgr!(
                <g>
                    <defs>
                        {ramp(across, (start_x, 0.0, start_x + spacing / 2.0, 0.0))}
                        {ramp(down, (0.0, start_y, 0.0, start_y + spacing / 2.0))}
                    </defs>
                    <rect x={left} y={top} width={width} height={height} fill={across_fill} />
                    <rect x={left} y={top} width={width} height={height} fill={down_fill} mix-blend-mode="darken" />
                </g>
            ),
            HalftoneShape::Line => fframes::svgr!(
                <g>
                    <defs>{ramp(down, (0.0, start_y, 0.0, start_y + spacing / 2.0))}</defs>
                    <rect x={left} y={top} width={width} height={height} fill={down_fill} />
                </g>
            ),
        };
        if self.rotation == 0.0 {
            return shapes;
        }
        let turn = format!("rotate({} {cx} {cy})", self.rotation);
        fframes::svgr!(<g transform={turn}>{shapes}</g>)
    }
}

impl<'a> Component<'a> for Halftone<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let id = ui.id(self.key, 0, "halftone");
        let (w, h) = self.area;
        let picture = format!("{id}-field");
        let href = format!("#{picture}");
        let field = self.field(&id);
        let defs = fframes::svgr!(<defs><g id={picture}>{field}</g></defs>);
        // the field rises 1 / radius a unit: the threshold softens the edge over 1.5 units
        let radius = self.grid().0 / 2.0;
        let sharp = radius / 0.75;
        let threshold = format!(
            "0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 {sharp} 0 0 0 {}",
            -0.5 * sharp
        );
        // a dot covers a pixel where field + (1 - brightness) passes 1
        let level = if self.invert {
            "1 0 0 0 0 1 0 0 0 0 1 0 0 0 0 0 0 0 1 0"
        } else {
            "-1 0 0 0 1 -1 0 0 0 1 -1 0 0 0 1 0 0 0 1 0"
        };
        let paint = match self.color {
            HalftoneColor::Solid => {
                let fill = self.dot_color.with_alpha(255);
                let opacity = f32::from(self.dot_color.a) / 255.0;
                Svgr::from(vec![
                    fframes::svgr!(<feFlood flood-color={fill} flood-opacity={opacity} />),
                    fframes::svgr!(<feComposite in2="coverage" operator="in" />),
                ])
            }
            HalftoneColor::Source => {
                fframes::svgr!(<feComposite in="SourceGraphic" in2="coverage" operator="in" />)
            }
        };
        let primitives = Svgr::from(vec![
            // the brightness over white, so transparent parts are white
            fframes::svgr!(
                <feColorMatrix in="SourceGraphic" type="matrix" result="brightness"
                               values="0.299 0.587 0.114 0 0 0.299 0.587 0.114 0 0 0.299 0.587 0.114 0 0 0 0 0 1 0" />
            ),
            fframes::svgr!(<feFlood flood-color="#ffffff" result="white" />),
            fframes::svgr!(<feComposite in="brightness" in2="white" operator="over" result="grey" />),
            fframes::svgr!(<feColorMatrix in="grey" type="matrix" values={level} result="level" />),
            fframes::svgr!(<feImage href={href} result="field" />),
            fframes::svgr!(
                <feComposite in="field" in2="level" operator="arithmetic"
                             k1="0" k2="0.5" k3="0.5" k4="0" result="sum" />
            ),
            fframes::svgr!(<feColorMatrix in="sum" type="matrix" values={threshold} result="coverage" />),
            paint,
        ]);
        let dotted = filtered((self.children)(ui), id, Region::Area(w, h), primitives);
        fframes::svgr!(<g>{defs}{dotted}</g>)
    }
}

/// Dots growing along a gradient over its children (Remotion's `halftoneLinearGradient()`):
/// a dot every `grid_size` units, centred on the area, `first_dot_size` wide at `first_stop`
/// growing to `second_dot_size` at `second_stop` (0 to 1 across the area). Solid dots in
/// `dot_color` are laid over the children, only over their opaque parts with
/// `mask_to_source_alpha`; [`HalftoneColor::Source`] keeps the children only inside the dots.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct HalftoneLinearGradient<'a> {
    pub first_dot_size: f32,
    pub second_dot_size: f32,
    pub first_stop: (f32, f32),
    pub second_stop: (f32, f32),
    pub grid_size: f32,
    pub color: HalftoneColor,
    pub dot_color: Color,
    pub mask_to_source_alpha: bool,
    pub area: (f32, f32),
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

impl Themed for HalftoneLinearGradient<'_> {
    /// Remotion's defaults: black dots on a 24 unit grid, none on the left growing to 40
    /// units on the right.
    fn themed(_: &Theme) -> Self {
        Self {
            first_dot_size: 0.0,
            second_dot_size: 40.0,
            first_stop: (0.0, 0.5),
            second_stop: (1.0, 0.5),
            grid_size: 24.0,
            color: HalftoneColor::Solid,
            dot_color: Color::BLACK,
            mask_to_source_alpha: false,
            area: (1920.0, 1080.0),
            key: None,
            children: no_children(),
        }
    }
}

impl HalftoneLinearGradient<'_> {
    /// The dots `(x, y, radius)`; a grid finer than [`MAX_PATTERN_SHAPES`](super::MAX_PATTERN_SHAPES) dots is drawn
    /// coarser.
    pub fn dots(&self) -> Vec<(f32, f32, f32)> {
        let (w, h) = self.area;
        if w <= 0.0 || h <= 0.0 {
            return Vec::new();
        }
        let grid = self.grid_size.max(1.0);
        let scale = coarsen((w / grid + 1.0) * (h / grid + 1.0), 2);
        let grid = grid * scale;
        let (ax, ay) = self.first_stop;
        let (vx, vy) = (self.second_stop.0 - ax, self.second_stop.1 - ay);
        let length = vx * vx + vy * vy;
        let (cx, cy) = (w / 2.0, h / 2.0);
        steps(0.0, h, cy, grid)
            .flat_map(|j| steps(0.0, w, cx, grid).map(move |i| (i, j)))
            .filter_map(|(i, j)| {
                let (x, y) = (cx + i as f32 * grid, cy + j as f32 * grid);
                let progress = if length > 0.0 {
                    clamp01(((x / w - ax) * vx + (y / h - ay) * vy) / length)
                } else {
                    0.0
                };
                let size = lerp(self.first_dot_size, self.second_dot_size, progress) * scale;
                (size > 0.01).then_some((x, y, size / 2.0))
            })
            .collect()
    }
}

impl<'a> Component<'a> for HalftoneLinearGradient<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let id = ui.id(self.key, 0, "halftone-gradient");
        let d = circles(self.dots());
        let content = (self.children)(ui);
        match self.color {
            HalftoneColor::Source => {
                let dots = fframes::svgr!(<path d={d} fill="#ffffff" />);
                masked(content, dots, &id, self.area)
            }
            HalftoneColor::Solid => {
                let dots = fframes::svgr!(<path d={d} fill={self.dot_color} />);
                let shape = self.mask_to_source_alpha.then(|| (self.children)(ui));
                draw_over(content, dots, shape, &id, self.area)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::effects::MAX_PATTERN_SHAPES;

    #[test]
    fn gradient_dots_grow_from_the_first_stop() {
        let gradient = HalftoneLinearGradient::themed(&Theme::default());
        let dots = gradient.dots();
        let left = dots
            .iter()
            .filter(|d| d.0 < 200.0)
            .fold(0.0_f32, |r, d| r.max(d.2));
        let right = dots
            .iter()
            .filter(|d| d.0 > 1700.0)
            .fold(0.0_f32, |r, d| r.max(d.2));
        assert!(left < 4.0, "{left}");
        assert!((right - 20.0).abs() < 2.0, "{right}");
        // centred: a dot sits on the middle of the area
        assert!(dots.iter().any(|d| d.0 == 960.0 && d.1 == 540.0));
    }

    #[test]
    fn gradient_dots_stay_below_the_cap() {
        let fine = HalftoneLinearGradient {
            grid_size: 1.0,
            first_dot_size: 1.0,
            ..HalftoneLinearGradient::themed(&Theme::default())
        };
        assert!(fine.dots().len() <= MAX_PATTERN_SHAPES + 2 * 1922);
    }

    #[test]
    fn halftone_grid_is_coarsened_for_circles_only() {
        let fine = Halftone {
            dot_size: 1.0,
            ..Halftone::themed(&Theme::default())
        };
        assert!(fine.grid().1 > 1.0);
        let squares = Halftone {
            shape: HalftoneShape::Square,
            ..fine
        };
        assert_eq!(squares.grid(), (1.0, 1.0));
    }
}
