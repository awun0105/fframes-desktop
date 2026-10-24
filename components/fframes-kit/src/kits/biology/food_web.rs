//! A food web: who eats whom, in trophic levels from the producers up, with energy flowing
//! along the arrows.

use std::f64::consts::PI;

use fframes::{Color, Svgr};
use kurbo::{CubicBez, ParamCurve, Point, RoundedRect, Shape as _};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    geometry::{Look, Shape},
    kits::draw::{arrow_along, clear_bend, link},
    motion::{cubic_in_out, progress},
    text::{TextAnchor, line},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

/// Whether `p` is inside the pill of `size` (width, height) centred on `center`.
fn inside(p: Point, center: Point, size: (f64, f64)) -> bool {
    let r = size.1 / 2.0;
    let dx = ((p.x - center.x).abs() - (size.0 / 2.0 - r)).max(0.0);
    let dy = p.y - center.y;
    dx * dx + dy * dy <= r * r
}

/// The part of `curve` outside both pills: from where it leaves `a` to where it enters `b`.
fn between(curve: CubicBez, a: (Point, (f64, f64)), b: (Point, (f64, f64))) -> CubicBez {
    let steps = 64;
    let at = |i: usize| i as f64 / steps as f64;
    let start = (0..=steps)
        .map(at)
        .find(|&t| !inside(curve.eval(t), a.0, a.1))
        .unwrap_or(0.0);
    let end = (0..=steps)
        .rev()
        .map(at)
        .find(|&t| !inside(curve.eval(t), b.0, b.1))
        .unwrap_or(1.0);
    curve.subsegment(start..end.max(start))
}

/// A food web of `species` and the arrows of `eats` (`(prey, predator)` by name, pointing the
/// way energy flows), laid out in trophic levels with the producers at the bottom (the longest
/// chain of food sets each level), `level_gap` pixels between levels and
/// `slot` pixels between neighbours, centred on `center` (units of `ui.space`). Each species
/// is a pill coloured by its level from `level_colors`, and the levels are named at the left
/// from `level_names`. From `delay` the levels appear from the bottom one every `level_time`
/// seconds, the arrows into each level drawing as it does; with `flow` dots of energy then
/// run along every arrow. Names in `eats` that are not in `species` are left out.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct FoodWeb {
    pub species: Vec<String>,
    pub eats: Vec<(String, String)>,
    pub delay: f32,
    pub level_time: f32,
    pub center: (f64, f64),
    pub level_gap: f32,
    pub slot: f32,
    pub flow: bool,
    pub level_colors: Vec<Color>,
    pub level_names: Vec<String>,
    pub arrow_color: Color,
    pub flow_color: Color,
    pub name_style: TextStyle,
    pub level_style: TextStyle,
    pub text_color: Color,
    pub level_color: Color,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for FoodWeb {
    fn themed(t: &Theme) -> Self {
        let pairs = [
            ("Grass", "Rabbit"),
            ("Grass", "Grasshopper"),
            ("Grass", "Deer"),
            ("Shrubs", "Deer"),
            ("Shrubs", "Rabbit"),
            ("Grasshopper", "Frog"),
            ("Grasshopper", "Mouse"),
            ("Shrubs", "Mouse"),
            ("Frog", "Snake"),
            ("Mouse", "Snake"),
            ("Rabbit", "Fox"),
            ("Mouse", "Fox"),
            ("Snake", "Hawk"),
            ("Rabbit", "Hawk"),
        ];
        let mut name_style = t.typography.caption;
        name_style.weight = 600;
        Self {
            species: [
                "Grass",
                "Shrubs",
                "Grasshopper",
                "Rabbit",
                "Mouse",
                "Deer",
                "Frog",
                "Fox",
                "Snake",
                "Hawk",
            ]
            .map(String::from)
            .to_vec(),
            eats: pairs
                .iter()
                .map(|&(a, b)| (a.to_owned(), b.to_owned()))
                .collect(),
            delay: 0.0,
            level_time: 0.8,
            center: (0.4, -0.35),
            level_gap: 150.0,
            slot: 230.0,
            flow: true,
            level_colors: vec![
                Color::hex("#2f9e44"),
                Color::hex("#e8a317"),
                Color::hex("#e8590c"),
                Color::hex("#c92a2a"),
                Color::hex("#862e9c"),
            ],
            level_names: [
                "producers",
                "primary consumers",
                "secondary consumers",
                "tertiary consumers",
                "quaternary consumers",
            ]
            .map(String::from)
            .to_vec(),
            arrow_color: t.colors.muted,
            flow_color: Color::hex("#ffe066"),
            name_style,
            level_style: t.typography.caption,
            text_color: Color::WHITE,
            level_color: t.colors.muted,
            enter: None,
            exit: None,
        }
    }
}

impl Default for FoodWeb {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

impl FoodWeb {
    /// The trophic level of each of `species`: 0 for producers (they eat nothing here),
    /// otherwise one above the highest of their prey, so the longest food chain sets it.
    /// Feeding cycles stop climbing after one level per species.
    pub fn levels(&self) -> Vec<usize> {
        let edges = self.edges();
        let mut levels = vec![0; self.species.len()];
        for _ in 0..self.species.len() {
            let mut changed = false;
            for &(prey, predator) in &edges {
                if levels[predator] < levels[prey] + 1 {
                    levels[predator] = levels[prey] + 1;
                    changed = true;
                }
            }
            if !changed {
                break;
            }
        }
        levels
    }

    /// Where each species sits: its slot in its level (centred on 0) and its level. The
    /// producers keep the order of `species`; each level above is ordered by where its prey
    /// sit, then each level below by where its predators sit, to keep arrows from crossing.
    fn layout(&self) -> Vec<(f64, f64)> {
        let edges = self.edges();
        let levels = self.levels();
        let top = levels.iter().copied().max().unwrap_or(0);
        let mut rows: Vec<Vec<usize>> = (0..=top)
            .map(|l| (0..levels.len()).filter(|&i| levels[i] == l).collect())
            .collect();
        let mut slot = vec![0.0; levels.len()];
        let place = |rows: &[Vec<usize>], slot: &mut [f64]| {
            for row in rows {
                for (k, &i) in row.iter().enumerate() {
                    slot[i] = k as f64 - (row.len() as f64 - 1.0) / 2.0;
                }
            }
        };
        place(&rows, &mut slot);
        let mean = |of: &mut dyn Iterator<Item = usize>, slot: &[f64], own: f64| {
            let (sum, n) = of.fold((0.0, 0.0), |(s, n), i| (s + slot[i], n + 1.0));
            if n > 0.0 { sum / n } else { own }
        };
        for _ in 0..2 {
            for level in 1..=top {
                let mut row = rows[level].clone();
                let keys: Vec<f64> = row
                    .iter()
                    .map(|&i| {
                        let mut prey = edges.iter().filter(|e| e.1 == i).map(|e| e.0);
                        mean(&mut prey, &slot, slot[i])
                    })
                    .collect();
                let mut order: Vec<usize> = (0..row.len()).collect();
                order.sort_by(|&a, &b| keys[a].total_cmp(&keys[b]));
                row = order.iter().map(|&k| row[k]).collect();
                rows[level] = row;
                place(&rows, &mut slot);
            }
            for level in (0..top).rev() {
                let row = rows[level].clone();
                let keys: Vec<f64> = row
                    .iter()
                    .map(|&i| {
                        let mut predators = edges.iter().filter(|e| e.0 == i).map(|e| e.1);
                        mean(&mut predators, &slot, slot[i])
                    })
                    .collect();
                let mut order: Vec<usize> = (0..row.len()).collect();
                order.sort_by(|&a, &b| keys[a].total_cmp(&keys[b]));
                rows[level] = order.iter().map(|&k| row[k]).collect();
                place(&rows, &mut slot);
            }
        }
        (0..levels.len())
            .map(|i| (slot[i], levels[i] as f64))
            .collect()
    }

    /// The arrows of `eats` as indices into `species`.
    fn edges(&self) -> Vec<(usize, usize)> {
        let index = |name: &str| self.species.iter().position(|s| s == name);
        self.eats
            .iter()
            .filter_map(|(prey, predator)| Some((index(prey)?, index(predator)?)))
            .collect()
    }
}

impl<'a> Component<'a> for FoodWeb {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let edges = self.edges();
        let placed = self.layout();
        let t = ui.seconds() - self.delay;
        let (gap, slot) = (f64::from(self.level_gap), f64::from(self.slot));
        let top_level = placed.iter().map(|p| p.1).fold(0.0, f64::max);
        let (lo, hi) = placed
            .iter()
            .fold((f64::INFINITY, f64::NEG_INFINITY), |(lo, hi), p| {
                (lo.min(p.0), hi.max(p.0))
            });
        let mid_slot = if lo.is_finite() {
            f64::midpoint(lo, hi)
        } else {
            0.0
        };
        // producers at the bottom
        let at = |i: usize| {
            Point::new(
                middle.x + (placed[i].0 - mid_slot) * slot,
                middle.y + (top_level / 2.0 - placed[i].1) * gap,
            )
        };
        let size_of = |name: &str| {
            let size = self.name_style.size as f64;
            (
                name.chars().count() as f64 * size * 0.56 + 40.0,
                size * 1.75,
            )
        };
        let shown_at = |level: f64| level as f32 * self.level_time;

        let mut out = Vec::new();
        // the names of the levels, at the left
        let left = middle.x + (lo.min(hi) - mid_slot) * slot - 120.0;
        for level in 0..=(top_level.round() as usize) {
            let Some(name) = self.level_names.get(level) else {
                continue;
            };
            let y = middle.y + (top_level / 2.0 - level as f64) * gap;
            let shown = progress(t, shown_at(level as f64), shown_at(level as f64) + 0.5);
            if shown > 0.0 {
                out.push(line(
                    left as f32,
                    (y + self.level_style.size as f64 * 0.35) as f32,
                    self.level_style,
                    self.level_color,
                    TextAnchor::End,
                    0.0,
                    shown,
                    name.clone().into(),
                ));
            }
        }

        // the arrows, drawing as their predator's level appears
        let centres: Vec<Point> = (0..self.species.len()).map(at).collect();
        let mut arrows = Vec::new();
        let mut dots = Vec::new();
        for (k, &(prey, predator)) in edges.iter().enumerate() {
            let (a, b) = (centres[prey], centres[predator]);
            let others: Vec<Point> = centres
                .iter()
                .enumerate()
                .filter(|&(i, _)| i != prey && i != predator)
                .map(|(_, &p)| p)
                .collect();
            let bend = clear_bend(a, b, 0.0, &others, 60.0, 0.0);
            let curve = between(
                link(a, b, 0.0, 0.0, bend),
                (a, size_of(&self.species[prey])),
                (b, size_of(&self.species[predator])),
            );
            let start = shown_at(placed[predator].1) - 0.2;
            let drawn = cubic_in_out(progress(t, start, start + 0.6));
            if drawn <= 0.0 {
                continue;
            }
            let part = curve.subsegment(0.0..f64::from(drawn));
            arrows.push(arrow_along(part, true, self.arrow_color, 3.0, 1.0));
            // energy runs from prey to predator once everything is there
            let built = shown_at(top_level) + 0.8;
            if self.flow && t > built {
                let ramp = progress(t, built, built + 0.6);
                let u = (f64::from(t - built) / 1.6 + k as f64 * 0.37).fract();
                let p = curve.eval(u);
                let fade_ends = (PI * u).sin() as f32;
                dots.push(
                    Shape {
                        path: kurbo::Circle::new(p, 6.0).to_path(0.1),
                        fill: Some(self.flow_color),
                        stroke: None,
                        stroke_width: 0.0,
                        dash: None,
                        opacity: ramp * fade_ends,
                    }
                    .draw(),
                );
            }
        }
        out.extend(arrows);
        out.extend(dots);

        // the species, over the arrows
        for (i, name) in self.species.iter().enumerate() {
            let level = placed[i].1;
            let start = shown_at(level);
            let shown = cubic_in_out(progress(t, start, start + 0.5));
            if shown <= 0.0 {
                continue;
            }
            let c = centres[i];
            let (w, h) = size_of(name);
            let color = self
                .level_colors
                .get(level.round() as usize)
                .or(self.level_colors.last())
                .copied()
                .unwrap_or(self.arrow_color);
            let grow = 0.8 + 0.2 * f64::from(shown);
            let pill = Shape {
                path: RoundedRect::new(
                    c.x - w / 2.0,
                    c.y - h / 2.0,
                    c.x + w / 2.0,
                    c.y + h / 2.0,
                    h / 2.0,
                )
                .to_path(0.1),
                fill: Some(color),
                stroke: None,
                stroke_width: 0.0,
                dash: None,
                opacity: 1.0,
            };
            let look = Look {
                transform: kurbo::Affine::translate(c.to_vec2())
                    * kurbo::Affine::scale(grow)
                    * kurbo::Affine::translate(-c.to_vec2()),
                opacity: shown,
                ..Look::default()
            };
            out.push(pill.draw_with(&look));
            out.push(line(
                c.x as f32,
                (c.y + self.name_style.size as f64 * 0.35) as f32,
                self.name_style,
                self.text_color,
                TextAnchor::Middle,
                0.0,
                shown,
                name.clone().into(),
            ));
        }

        fade(ui, out.into(), self.enter.as_ref(), self.exit.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_follow_the_longest_food_chain() {
        let web = FoodWeb::default();
        let levels = web.levels();
        let level = |name: &str| levels[web.species.iter().position(|s| s == name).unwrap()];
        assert_eq!(level("Grass"), 0);
        assert_eq!(level("Shrubs"), 0);
        assert_eq!(level("Rabbit"), 1);
        assert!(level("Snake") > level("Frog") && level("Frog") > level("Grasshopper"));
        assert!(level("Hawk") > level("Snake"));
        // every arrow points up a level
        for (prey, predator) in web.edges() {
            assert!(levels[predator] > levels[prey]);
        }
    }

    #[test]
    fn unknown_names_are_left_out() {
        let web = FoodWeb {
            species: vec!["Algae".into(), "Fish".into()],
            eats: vec![
                ("Algae".into(), "Fish".into()),
                ("Fish".into(), "Bear".into()),
            ],
            ..FoodWeb::default()
        };
        assert_eq!(web.edges(), [(0, 1)]);
        assert_eq!(web.levels(), [0, 1]);
    }

    #[test]
    fn arrows_start_and_end_outside_the_pills() {
        let (a, b) = (Point::new(0.0, 300.0), Point::new(0.0, 0.0));
        let size = (160.0, 48.0);
        let curve = between(link(a, b, 0.0, 0.0, 0.0), (a, size), (b, size));
        assert!(!inside(curve.p0, a, size) && !inside(curve.p3, b, size));
        assert!((curve.p0.y - 276.0).abs() < 6.0 && (curve.p3.y - 24.0).abs() < 6.0);
    }
}
