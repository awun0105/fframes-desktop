use fframes::{Color, Svgr, Transform, animation::Animatable};
use kurbo::Point;

use crate::{
    anim::{FadeIn, FadeOut, fade},
    animate::draw_animated,
    kits::{label, short},
    label::Label,
    layout::Anchor,
    motion::progress,
    shapes::RoundedRectangle,
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::{Category, atomic_weight, element, name, symbol, table_cell};

/// A property of the elements that [`PeriodicTable`] can colour the tiles by.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trend {
    /// The empirical atomic radius, in pm.
    AtomicRadius,
    /// The first ionization energy, in eV.
    IonizationEnergy,
    /// The standard atomic weight.
    AtomicWeight,
}

impl Trend {
    /// The value for element `number`, `None` where it is not known.
    pub fn value(self, number: u32) -> Option<f64> {
        let e = element(number)?;
        match self {
            Trend::AtomicRadius => e.atomic_radius().map(|r| r.0),
            Trend::IonizationEnergy => e.ionization_energy().map(|v| v.0),
            Trend::AtomicWeight => atomic_weight(number),
        }
    }

    /// The name and unit for a legend.
    pub fn name(self) -> (&'static str, &'static str) {
        match self {
            Trend::AtomicRadius => ("atomic radius", "pm"),
            Trend::IonizationEnergy => ("ionization energy", "eV"),
            Trend::AtomicWeight => ("atomic weight", "u"),
        }
    }

    /// The lowest and the highest known value over the 118 elements.
    pub fn range(self) -> (f64, f64) {
        (1..=118)
            .filter_map(|n| self.value(n))
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), v| {
                (lo.min(v), hi.max(v))
            })
    }
}

fn style(base: TextStyle, size: f32, weight: u16) -> TextStyle {
    TextStyle {
        size: size.round().max(6.0) as usize,
        weight,
        ..base
    }
}

/// One square tile `size` pixels wide at `at`: the atomic number in the top left corner and
/// the symbol, and with `details` the name and atomic weight under it, tinted with `color`.
fn tile<'a>(
    ui: &mut Ui<'a, '_, '_>,
    number: u32,
    at: Point,
    size: f32,
    details: bool,
    color: Color,
    text: Color,
) -> Svgr<'a> {
    let (x, y) = (at.x as f32, at.y as f32);
    let corner = size * 0.08;
    let mut back = RoundedRectangle {
        cx: x,
        cy: y,
        width: size,
        height: size,
        corner_radius: corner,
        fill: Some(color),
        stroke: None,
        ..ui.defaults()
    }
    .shape();
    back.opacity = 0.18;
    let border = RoundedRectangle {
        cx: x,
        cy: y,
        width: size,
        height: size,
        corner_radius: corner,
        fill: None,
        stroke: Some(color),
        stroke_width: (size / 50.0).max(1.5),
        ..ui.defaults()
    }
    .shape();
    let box_ = draw_animated(ui, &[back, border], None, None, None);
    let base = ui.theme.typography.body;
    let pad = size * 0.09;
    let number_label = ui.show(Label {
        text: number.to_string().into(),
        x: x - size / 2.0 + pad,
        y: y - size / 2.0 + pad,
        direction: Anchor::BottomRight,
        buff: 0.0,
        style: style(base, size * 0.15, 500),
        fill: text,
        ..ui.defaults()
    });
    let symbol_y = if details {
        y - size * 0.06
    } else {
        y + size * 0.08
    };
    let symbol_label = ui.show(Label {
        text: symbol(number).into(),
        x,
        y: symbol_y,
        direction: Anchor::Center,
        buff: 0.0,
        style: style(base, size * 0.34, 700),
        fill: ui.theme.colors.title,
        ..ui.defaults()
    });
    let mut parts = vec![box_, number_label, symbol_label];
    if details {
        parts.push(ui.show(Label {
            text: name(number).into(),
            x,
            y: y + size * 0.2,
            direction: Anchor::Center,
            buff: 0.0,
            style: style(base, size * 0.12, 500),
            fill: text,
            ..ui.defaults()
        }));
        if let Some(weight) = atomic_weight(number) {
            parts.push(ui.show(Label {
                text: short(weight, 3).into(),
                x,
                y: y + size * 0.36,
                direction: Anchor::Center,
                buff: 0.0,
                style: style(base, size * 0.11, 400),
                fill: text,
                ..ui.defaults()
            }));
        }
    }
    fframes::svgr!(<g>{parts}</g>)
}

/// One element as a tile `size` pixels wide centred at `center` (units of `ui.space`): its
/// atomic number, symbol, name and atomic weight, tinted with the colour of its
/// [`Category`] (or `color` without `category_colors`).
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct ElementTile {
    pub element: u32,
    pub center: (f64, f64),
    pub size: f32,
    pub details: bool,
    pub category_colors: bool,
    pub color: Color,
    pub text_color: Color,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for ElementTile {
    fn themed(t: &Theme) -> Self {
        Self {
            element: 6,
            center: (0.0, 0.0),
            size: 220.0,
            details: true,
            category_colors: true,
            color: t.colors.accent,
            text_color: t.colors.text,
            enter: None,
            exit: None,
        }
    }
}

impl Default for ElementTile {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl<'a> Component<'a> for ElementTile {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        if element(self.element).is_none() {
            return Svgr::empty();
        }
        let at = ui.space.to_px(self.center.0, self.center.1);
        let color = if self.category_colors {
            Category::of(self.element).color()
        } else {
            self.color
        };
        let drawn = tile(
            ui,
            self.element,
            at,
            self.size,
            self.details,
            color,
            self.text_color,
        );
        fade(ui, drawn, self.enter.as_ref(), self.exit.as_ref())
    }
}

/// The periodic table: 118 tiles `cell` pixels wide, `gap` apart, centred at `center`
/// (units of `ui.space`), the lanthanides and actinides in two rows under it. After `delay`
/// the tiles rise in by atomic number, `stagger` seconds apart. From `highlight_at` the
/// tiles outside group `highlight_group` (1 to 18) and outside `highlight_category` dim to
/// `dim`. From `trend_at` a sweep runs left to right over `sweep` seconds and recolours the
/// tiles by `trend`, from `low_color` to `high_color` (tiles without a value turn
/// `muted`), and with `labels` a legend under the table gives the range.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct PeriodicTable {
    pub center: (f64, f64),
    pub cell: f32,
    pub gap: f32,
    pub delay: f32,
    pub stagger: f32,
    pub highlight_group: Option<u32>,
    pub highlight_category: Option<Category>,
    pub highlight_at: f32,
    pub dim: f32,
    pub trend: Option<Trend>,
    pub trend_at: f32,
    pub sweep: f32,
    pub labels: bool,
    pub low_color: Color,
    pub high_color: Color,
    pub muted: Color,
    pub text_color: Color,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for PeriodicTable {
    fn themed(t: &Theme) -> Self {
        Self {
            center: (0.0, 0.0),
            cell: 78.0,
            gap: 6.0,
            delay: 0.0,
            stagger: 0.01,
            highlight_group: None,
            highlight_category: None,
            highlight_at: 0.0,
            dim: 0.2,
            trend: None,
            trend_at: 0.0,
            sweep: 1.2,
            labels: true,
            low_color: Color::hex("#4dabf7"),
            high_color: Color::hex("#ff6b6b"),
            muted: t.colors.muted,
            text_color: t.colors.text,
            enter: None,
            exit: None,
        }
    }
}

impl Default for PeriodicTable {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl PeriodicTable {
    /// The middle of element `number`'s tile, in pixels.
    pub fn tile_center(&self, ui: &Ui<'_, '_, '_>, number: u32) -> Option<Point> {
        let (column, row) = table_cell(number)?;
        let step = f64::from(self.cell + self.gap);
        let cell = f64::from(self.cell);
        let width = 18.0 * step - f64::from(self.gap);
        let height = 9.0 * step - f64::from(self.gap) + cell * 0.4;
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let left = middle.x - width / 2.0;
        let top = middle.y - height / 2.0;
        // the f block sits apart from the main table
        let apart = if row >= 7 { cell * 0.4 } else { 0.0 };
        Some(Point::new(
            left + f64::from(column) * step + cell / 2.0,
            top + f64::from(row) * step + apart + cell / 2.0,
        ))
    }

    fn highlighted(&self, number: u32) -> bool {
        let in_group = self.highlight_group.is_none_or(|g| {
            !matches!(number, 57..=71 | 89..=103)
                && table_cell(number).is_some_and(|(column, _)| column + 1 == g)
        });
        let in_category = self
            .highlight_category
            .is_none_or(|c| Category::of(number) == c);
        in_group && in_category
    }
}

impl<'a> Component<'a> for PeriodicTable {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let now = ui.seconds();
        let dimming = if self.highlight_group.is_some() || self.highlight_category.is_some() {
            progress(now, self.highlight_at, self.highlight_at + 0.4)
        } else {
            0.0
        };
        let range = self.trend.map(Trend::range);
        let mut tiles = Vec::with_capacity(118);
        for number in 1..=118 {
            let Some(at) = self.tile_center(ui, number) else {
                continue;
            };
            let shown = progress(
                now,
                self.delay + self.stagger * (number - 1) as f32,
                self.delay + self.stagger * (number - 1) as f32 + 0.3,
            );
            if shown <= 0.0 {
                continue;
            }
            let mut color = Category::of(number).color();
            if let (Some(trend), Some((lo, hi))) = (self.trend, range) {
                let column = table_cell(number).map_or(0, |c| c.0);
                let start = self.trend_at + self.sweep * column as f32 / 18.0;
                let turned = progress(now, start, start + 0.3);
                let target = match trend.value(number) {
                    Some(v) if hi > lo => {
                        let f = ((v - lo) / (hi - lo)) as f32;
                        self.low_color.apply_progress(&self.high_color, f)
                    }
                    Some(_) => self.low_color,
                    None => self.muted,
                };
                color = color.apply_progress(&target, turned);
            }
            let drawn = tile(ui, number, at, self.cell, false, color, self.text_color);
            let opacity = if self.highlighted(number) {
                shown
            } else {
                shown * (1.0 - (1.0 - self.dim) * dimming)
            };
            let rise = Transform::translate(0.0, 12.0 * (1.0 - shown));
            tiles.push(fframes::svgr!(<g opacity={opacity} transform={rise}>{drawn}</g>));
        }
        let mut parts = vec![fframes::svgr!(<g>{tiles}</g>)];
        if let (true, Some(trend), Some((lo, hi))) = (self.labels, self.trend, range) {
            let shown = progress(now, self.trend_at, self.trend_at + 0.4);
            // in the empty top row, from the third column
            if let Some(hydrogen) = self.tile_center(ui, 1) {
                let (name, unit) = trend.name();
                let text = format!("{name}: {} to {} {unit}", short(lo, 1), short(hi, 1));
                let step = f64::from(self.cell + self.gap);
                let x = hydrogen.x - f64::from(self.cell) / 2.0 + 2.0 * step;
                let at = Point::new(x, hydrogen.y);
                let legend = label(ui, text, at, Anchor::Right, self.text_color);
                parts.push(fframes::svgr!(<g opacity={shown}>{legend}</g>));
            }
        }
        let drawn = fframes::svgr!(<g>{parts}</g>);
        fade(ui, drawn, self.enter.as_ref(), self.exit.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trends_have_values_and_ranges() {
        let radius = Trend::AtomicRadius;
        // radius grows down a group and shrinks along a period
        assert!(radius.value(55).unwrap() > radius.value(3).unwrap());
        assert!(radius.value(9).unwrap() < radius.value(3).unwrap());
        let energy = Trend::IonizationEnergy;
        assert!(energy.value(2).unwrap() > energy.value(11).unwrap());
        let (lo, hi) = energy.range();
        assert!(lo > 3.0 && hi > 24.0, "{lo} {hi}");
    }

    #[test]
    fn highlights_pick_the_group_and_the_category() {
        let table = PeriodicTable {
            highlight_group: Some(1),
            ..PeriodicTable::default()
        };
        assert!(table.highlighted(1) && table.highlighted(11));
        assert!(!table.highlighted(12));
        let table = PeriodicTable {
            highlight_group: Some(3),
            ..PeriodicTable::default()
        };
        assert!(table.highlighted(21) && !table.highlighted(58));
        let table = PeriodicTable {
            highlight_category: Some(Category::Halogen),
            ..PeriodicTable::default()
        };
        assert!(table.highlighted(17) && !table.highlighted(18));
        assert!(PeriodicTable::default().highlighted(50));
    }
}
