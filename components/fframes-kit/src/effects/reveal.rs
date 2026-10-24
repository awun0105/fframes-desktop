use std::fmt::Write;

use fframes::{Color, Svgr};

use super::patterns::{draw_over, masked};
use crate::{
    anim::Anim,
    motion::clamp01,
    theme::Theme,
    ui::{Children, Component, Themed, Ui, no_children},
};

/// The stops a soft [`Evolve`] edge samples its smoothstep at.
const EDGE_STEPS: usize = 8;

/// Which way the slats of [`VenetianBlinds`] run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BlindsDirection {
    /// Upright slats side by side.
    #[default]
    Vertical,
    /// Lying slats one above the other.
    Horizontal,
}

/// Its children shown through blinds that open (Remotion's `venetianBlinds()`): `slats`
/// strips across the area, each showing a band from its middle that is `progress` (0
/// hidden to 1 open) of the strip wide.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct VenetianBlinds<'a> {
    pub progress: Anim<'a, f32>,
    pub direction: BlindsDirection,
    pub slats: u32,
    pub area: (f32, f32),
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

impl Themed for VenetianBlinds<'_> {
    fn themed(_: &Theme) -> Self {
        Self {
            progress: Anim::Fixed(0.5),
            direction: BlindsDirection::Vertical,
            slats: 12,
            area: (1920.0, 1080.0),
            key: None,
            children: no_children(),
        }
    }
}

impl VenetianBlinds<'_> {
    /// The open bands as `(x, y, w, h)` at `progress`.
    pub fn bands(&self, progress: f32) -> Vec<(f32, f32, f32, f32)> {
        let (w, h) = self.area;
        let slats = self.slats.max(1);
        let vertical = self.direction == BlindsDirection::Vertical;
        let slat = if vertical { w } else { h } / slats as f32;
        let open = clamp01(progress) * slat;
        (0..slats)
            .map(|k| {
                let start = (k as f32 + 0.5) * slat - open / 2.0;
                if vertical {
                    (start, 0.0, open, h)
                } else {
                    (0.0, start, w, open)
                }
            })
            .collect()
    }
}

impl<'a> Component<'a> for VenetianBlinds<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let progress = ui.sample(&self.progress);
        if progress <= 1e-6 {
            return Svgr::empty();
        }
        let content = (self.children)(ui);
        if progress >= 1.0 - 1e-6 {
            return content;
        }
        let mut d = String::new();
        for (x, y, w, h) in self.bands(progress) {
            let _ = write!(d, "M{x} {y}h{w}v{h}h{}Z", -w);
        }
        let id = ui.id(self.key, 0, "venetian-blinds");
        let bands = fframes::svgr!(<path d={d} fill="#ffffff" />);
        masked(content, bands, &id, self.area)
    }
}

/// Where [`Evolve`] starts revealing.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum EvolveDirection {
    #[default]
    Left,
    Right,
    Top,
    Bottom,
}

/// Its children revealed by an edge sweeping across the area (Remotion's `evolve()`): from
/// the `direction` side, `progress` (0 hidden to 1 shown) of the way across, the edge
/// softened over `feather` (0 to 1) of the area behind it.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct Evolve<'a> {
    pub progress: Anim<'a, f32>,
    pub direction: EvolveDirection,
    pub feather: f32,
    pub area: (f32, f32),
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

impl Themed for Evolve<'_> {
    fn themed(_: &Theme) -> Self {
        Self {
            progress: Anim::Fixed(0.5),
            direction: EvolveDirection::Left,
            feather: 0.1,
            area: (1920.0, 1080.0),
            key: None,
            children: no_children(),
        }
    }
}

impl Evolve<'_> {
    /// The mask along the sweep as `(offset, opacity)` stops at `progress`: shown up to
    /// `progress`, then a smoothstep down to hidden over `feather`.
    pub fn stops(&self, progress: f32) -> Vec<(f32, f32)> {
        let start = clamp01(progress);
        let end = (start + self.feather.max(0.0)).min(1.0);
        if end - start <= 1e-4 {
            return vec![(start, 1.0), (start, 0.0)];
        }
        (0..=EDGE_STEPS)
            .map(|i| {
                let t = i as f32 / EDGE_STEPS as f32;
                (start + t * (end - start), 1.0 - t * t * (3.0 - 2.0 * t))
            })
            .collect()
    }
}

impl<'a> Component<'a> for Evolve<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let progress = ui.sample(&self.progress);
        if progress <= 1e-6 {
            return Svgr::empty();
        }
        let content = (self.children)(ui);
        if progress >= 1.0 - 1e-6 {
            return content;
        }
        let (w, h) = self.area;
        let (x1, y1, x2, y2) = match self.direction {
            EvolveDirection::Left => (0.0, 0.0, w, 0.0),
            EvolveDirection::Right => (w, 0.0, 0.0, 0.0),
            EvolveDirection::Top => (0.0, 0.0, 0.0, h),
            EvolveDirection::Bottom => (0.0, h, 0.0, 0.0),
        };
        let id = ui.id(self.key, 0, "evolve");
        let gradient = format!("{id}-sweep");
        let fill = format!("url(#{gradient})");
        let stops: Vec<Svgr<'a>> = self
            .stops(progress)
            .into_iter()
            .map(|(offset, opacity)| {
                fframes::svgr!(<stop offset={offset} stop-color="#ffffff" stop-opacity={opacity} />)
            })
            .collect();
        let sweep = fframes::svgr!(
            <g>
                <defs>
                    <linearGradient id={gradient} gradientUnits="userSpaceOnUse"
                                    x1={x1} y1={y1} x2={x2} y2={y2}>
                        {Svgr::from(stops)}
                    </linearGradient>
                </defs>
                <rect x="0" y="0" width={w} height={h} fill={fill} />
            </g>
        );
        masked(content, sweep, &id, self.area)
    }
}

/// The colours of the top colour bars, left to right.
const TOP_BARS: [Color; 7] = [
    Color::rgb(189, 189, 189),
    Color::rgb(255, 255, 0),
    Color::rgb(0, 255, 255),
    Color::rgb(0, 255, 0),
    Color::rgb(255, 0, 255),
    Color::rgb(255, 0, 0),
    Color::rgb(0, 0, 255),
];

/// The colours of the thin bars below them.
const MIDDLE_BARS: [Color; 7] = [
    Color::rgb(0, 0, 255),
    Color::BLACK,
    Color::rgb(255, 0, 255),
    Color::BLACK,
    Color::rgb(0, 255, 255),
    Color::BLACK,
    Color::rgb(189, 189, 189),
];

/// Its children replaced by television test bars (Remotion's `tvSignalOff()`), mixed in by
/// `amount` (0 to 1) inside the children's alpha: seven colour bars, a row of thin bars,
/// white running into a dark ramp and a grey scale. The bars are drawn over the children,
/// so semi-transparent parts mix a little differently from Remotion. The children render
/// twice.
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct TvSignalOff<'a> {
    pub amount: Anim<'a, f32>,
    pub area: (f32, f32),
    pub key: Option<&'a str>,
    pub children: Children<'a>,
}

impl Themed for TvSignalOff<'_> {
    fn themed(_: &Theme) -> Self {
        Self {
            amount: Anim::Fixed(1.0),
            area: (1920.0, 1080.0),
            key: None,
            children: no_children(),
        }
    }
}

impl TvSignalOff<'_> {
    /// The flat parts of the test picture as `(x, y, w, h, colour)`.
    pub fn bars(&self) -> Vec<(f32, f32, f32, f32, Color)> {
        let (w, h) = self.area;
        let bar = w / 7.0;
        let row = |colors: [Color; 7], top: f32, bottom: f32| {
            colors
                .into_iter()
                .enumerate()
                .map(move |(i, c)| (i as f32 * bar, top * h, bar, (bottom - top) * h, c))
        };
        let mut bars: Vec<_> = row(TOP_BARS, 0.0, 0.64)
            .chain(row(MIDDLE_BARS, 0.64, 0.76))
            .collect();
        // the bottom row: twelve grey steps cut at 58% of the width, black after them
        let edge = 0.58 * w;
        for k in 0..12 {
            let x = k as f32 / 12.0 * w;
            if x >= edge {
                break;
            }
            let grey = (k as f32 / 11.0 * 255.0).round() as u8;
            let width = (w / 12.0).min(edge - x);
            bars.push((x, 0.88 * h, width, 0.12 * h, Color::rgb(grey, grey, grey)));
        }
        bars.push((edge, 0.88 * h, w - edge, 0.12 * h, Color::BLACK));
        bars
    }
}

impl<'a> Component<'a> for TvSignalOff<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let amount = clamp01(ui.sample(&self.amount));
        let content = (self.children)(ui);
        if amount <= 0.0 {
            return content;
        }
        let (w, h) = self.area;
        let id = ui.id(self.key, 0, "tv-signal-off");
        let ramp = format!("{id}-ramp");
        let ramp_fill = format!("url(#{ramp})");
        // white to 58%, then a smoothstep down to black
        let stops: Vec<Svgr<'a>> = (0..=EDGE_STEPS)
            .map(|i| {
                let t = i as f32 / EDGE_STEPS as f32;
                let grey = ((1.0 - t * t * (3.0 - 2.0 * t)) * 255.0).round() as u8;
                let color = Color::rgb(grey, grey, grey);
                fframes::svgr!(<stop offset={0.58 + 0.42 * t} stop-color={color} />)
            })
            .collect();
        let rects: Vec<Svgr<'a>> = self
            .bars()
            .into_iter()
            .map(|(x, y, bw, bh, fill)| {
                fframes::svgr!(<rect x={x} y={y} width={bw} height={bh} fill={fill} />)
            })
            .collect();
        let bars = fframes::svgr!(
            <g opacity={amount}>
                <defs>
                    <linearGradient id={ramp} gradientUnits="userSpaceOnUse" x1="0" y1="0" x2={w} y2="0">
                        <stop offset="0" stop-color="#ffffff" />
                        {Svgr::from(stops)}
                    </linearGradient>
                </defs>
                {Svgr::from(rects)}
                <rect x="0" y={0.76 * h} width={w} height={0.12 * h} fill={ramp_fill} />
            </g>
        );
        let shape = (self.children)(ui);
        draw_over(content, bars, Some(shape), &id, self.area)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blinds_open_from_the_slat_middles() {
        let blinds = VenetianBlinds {
            slats: 4,
            area: (400.0, 100.0),
            ..VenetianBlinds::themed(&Theme::default())
        };
        assert_eq!(blinds.bands(0.5)[0], (25.0, 0.0, 50.0, 100.0));
        assert_eq!(blinds.bands(1.0)[3], (300.0, 0.0, 100.0, 100.0));
    }

    #[test]
    fn evolve_edge_runs_from_shown_to_hidden() {
        let evolve = Evolve::themed(&Theme::default());
        let stops = evolve.stops(0.5);
        assert_eq!(stops.first(), Some(&(0.5, 1.0)));
        assert_eq!(stops.last(), Some(&(0.6, 0.0)));
        let sharp = Evolve {
            feather: 0.0,
            ..evolve
        };
        assert_eq!(sharp.stops(0.3), [(0.3, 1.0), (0.3, 0.0)]);
    }

    #[test]
    fn tv_bars_fill_their_rows() {
        let tv = TvSignalOff::themed(&Theme::default());
        let bars = tv.bars();
        let top: f32 = bars.iter().filter(|b| b.1 == 0.0).map(|b| b.2).sum();
        assert!((top - 1920.0).abs() < 0.01);
        let bottom: f32 = bars
            .iter()
            .filter(|b| b.1 > 0.8 * 1080.0)
            .map(|b| b.2)
            .sum();
        assert!((bottom - 1920.0).abs() < 0.01);
    }
}
