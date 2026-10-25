use std::borrow::Cow;

use fframes::{Color, Svgr, Transform, animation::Animatable};

use crate::{
    motion::progress,
    text::{CaptionWord, TextAnchor, line, width},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

/// One word of a [`TikTokPage`], with its own timing.
#[derive(Debug, Clone, PartialEq)]
pub struct TikTokToken {
    pub text: String,
    pub from_ms: f64,
    pub to_ms: f64,
}

/// The words shown together, from `start_ms` for `duration_ms` (Remotion's `TikTokPage`).
#[derive(Debug, Clone, PartialEq)]
pub struct TikTokPage {
    pub text: String,
    pub start_ms: f64,
    pub duration_ms: f64,
    pub tokens: Vec<TikTokToken>,
}

/// Words grouped into pages (Remotion's `createTikTokStyleCaptions`).
///
/// A word that starts with a space starts a new page once the page has run for more than
/// `combine_within_ms`, so 0 gives a page per word. Words without a leading space never
/// start a page: give transcript words their spaces (`from_whisper_json` keeps them). The
/// first word of a page loses its leading space and words of only spaces are dropped. A
/// page lasts until the next one starts, the last until its last word ends.
pub fn tiktok_pages(captions: &[CaptionWord], combine_within_ms: f64) -> Vec<TikTokPage> {
    let mut pages: Vec<TikTokPage> = Vec::new();
    for caption in captions {
        let breaks = caption.text.starts_with(char::is_whitespace);
        let new_page = pages.last().is_none_or(|page| {
            breaks
                && !page.tokens.is_empty()
                && page.start_ms + combine_within_ms < caption.start_ms
        });
        if new_page {
            pages.push(TikTokPage {
                text: String::new(),
                start_ms: caption.start_ms,
                duration_ms: 0.0,
                tokens: Vec::new(),
            });
        }
        let Some(page) = pages.last_mut() else {
            continue;
        };
        if caption.text.trim().is_empty() {
            continue;
        }
        if page.tokens.is_empty() {
            page.start_ms = caption.start_ms;
        }
        let text = if page.tokens.is_empty() {
            caption.text.trim_start()
        } else {
            caption.text.as_str()
        };
        page.text.push_str(text);
        page.tokens.push(TikTokToken {
            text: text.to_owned(),
            from_ms: caption.start_ms,
            to_ms: caption.end_ms,
        });
    }
    pages.retain(|page| !page.tokens.is_empty());
    let starts: Vec<f64> = pages.iter().skip(1).map(|p| p.start_ms).collect();
    for (i, page) in pages.iter_mut().enumerate() {
        let end = starts
            .get(i)
            .copied()
            .unwrap_or_else(|| page.tokens.last().map_or(page.start_ms, |t| t.to_ms));
        page.duration_ms = (end - page.start_ms).max(0.0);
        page.text = page.text.trim_end().to_owned();
    }
    pages
}

/// How a page of [`WordCaptions`] comes in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PageEntrance {
    /// It is there at once.
    None,
    Fade,
    /// It fades in while it rises half a line into place.
    Rise,
    /// It fades in while it grows from 80%.
    #[default]
    Pop,
}

/// TikTok-style captions: the page of `pages` that plays now, centred on `(x, y)` and
/// wrapped into `width` pixels, with the word being said in `highlight`, scaled by `pop`.
/// Times are milliseconds of the container. A page that needs more than `max_lines` lines
/// is shrunk until it fits. Between pages and after the last one nothing is drawn.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct WordCaptions {
    pub pages: Vec<TikTokPage>,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub max_lines: usize,
    pub style: TextStyle,
    pub fill: Color,
    pub highlight: Color,
    /// The size of the word being said, 1 for none.
    pub pop: f32,
    /// Seconds a word takes to light up and to fade back.
    pub fade: f32,
    pub uppercase: bool,
    /// A rounded box behind the page, in this colour.
    pub pill: Option<Color>,
    pub pill_opacity: f32,
    /// Space between the text and the edge of the pill.
    pub padding: f32,
    pub entrance: PageEntrance,
    /// Seconds of the entrance.
    pub enter: f32,
    /// Seconds the page takes to fade out before the next one starts, or the last one ends.
    pub exit: f32,
}

impl Themed for WordCaptions {
    fn themed(t: &Theme) -> Self {
        Self {
            pages: Vec::new(),
            x: 960.0,
            y: 860.0,
            width: 1400.0,
            max_lines: 2,
            style: t.typography.title,
            fill: t.colors.caption,
            highlight: t.colors.accent,
            pop: 1.12,
            fade: 0.08,
            uppercase: false,
            pill: None,
            pill_opacity: 0.75,
            padding: 24.0,
            entrance: PageEntrance::Pop,
            enter: 0.15,
            exit: 0.1,
        }
    }
}

impl Default for WordCaptions {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

/// The line of every token when tokens `widths` pixels wide, `space` apart, are wrapped
/// into `max_width`. A token wider than the line gets a line of its own.
fn wrap(widths: &[f32], space: f32, max_width: f32) -> Vec<usize> {
    let mut lines = Vec::with_capacity(widths.len());
    let (mut line, mut used) = (0, 0.0);
    for &w in widths {
        if used > 0.0 && used + space + w > max_width {
            line += 1;
            used = 0.0;
        }
        used += if used > 0.0 { space + w } else { w };
        lines.push(line);
    }
    lines
}

/// The scale that keeps `widths` within `max_lines` lines of `max_width`, and those lines.
fn fit(widths: &[f32], space: f32, max_width: f32, max_lines: usize) -> (f32, Vec<usize>) {
    let mut scale = 1.0;
    let mut lines = wrap(widths, space, max_width);
    for _ in 0..40 {
        if lines.last().map_or(0, |l| l + 1) <= max_lines.max(1) {
            break;
        }
        scale *= 0.95;
        lines = wrap(widths, space, max_width / scale);
    }
    (scale, lines)
}

impl WordCaptions {
    /// The page that plays at `ms`.
    fn page_at(&self, ms: f64) -> Option<&TikTokPage> {
        self.pages
            .iter()
            .rev()
            .find(|p| p.start_ms <= ms)
            .filter(|p| ms < p.start_ms + p.duration_ms)
    }

    /// How lit the token is at `t` seconds: up over `fade` from its start, down over `fade`
    /// from its end.
    fn lit(&self, token: &TikTokToken, t: f32) -> f32 {
        let (from, to) = (token.from_ms as f32 / 1000.0, token.to_ms as f32 / 1000.0);
        progress(t, from, from + self.fade) * (1.0 - progress(t, to, to + self.fade))
    }
}

impl<'a> Component<'a> for WordCaptions {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let t = ui.seconds();
        let Some(page) = self.page_at(f64::from(t) * 1000.0) else {
            return Svgr::empty();
        };
        let size = self.style.size as f32;
        let texts: Vec<String> = page
            .tokens
            .iter()
            .map(|token| {
                let text = token.text.trim();
                if self.uppercase {
                    text.to_uppercase()
                } else {
                    text.to_owned()
                }
            })
            .collect();
        let measure = |text: &str| {
            width(ui, self.style, text).unwrap_or(text.chars().count() as f32 * size * 0.55)
        };
        let widths: Vec<f32> = texts.iter().map(|text| measure(text)).collect();
        // with room for the word that pops
        let space = width(ui, self.style, " ").unwrap_or(size * 0.28)
            + size * (self.pop - 1.0).max(0.0) * 1.5;
        let (scale, lines) = fit(&widths, space, self.width, self.max_lines);
        let count = lines.last().map_or(1, |l| l + 1);
        let line_height = size * self.style.line_height;
        let top = -line_height * count as f32 / 2.0;
        let line_widths: Vec<f32> = (0..count)
            .map(|l| {
                let (sum, n) = lines
                    .iter()
                    .zip(&widths)
                    .filter(|(line, _)| **line == l)
                    .fold((0.0, 0), |(sum, n), (_, w)| (sum + w, n + 1));
                sum + space * (n.max(1) - 1) as f32
            })
            .collect();

        // where the next word of each line starts
        let mut x: Vec<Option<f32>> = vec![None; count];
        let mut words: Vec<Svgr<'a>> = Vec::with_capacity(texts.len());
        for (i, text) in texts.into_iter().enumerate() {
            let l = lines[i];
            let left = x[l].map_or(-line_widths[l] / 2.0, |end| end + space);
            let center = (left + widths[i] / 2.0, top + line_height * (l as f32 + 0.5));
            x[l] = Some(left + widths[i]);
            let lit = self.lit(&page.tokens[i], t);
            let fill = Animatable::apply_progress(&self.fill, &self.highlight, lit);
            let grow = Transform::scale(1.0 + (self.pop - 1.0) * lit);
            let place = Transform::translate(center.0, center.1);
            let word = line(
                0.0,
                size * 0.35,
                self.style,
                fill,
                TextAnchor::Middle,
                0.0,
                1.0,
                Cow::Owned(text),
            );
            words.push(fframes::svgr!(<g transform={place}><g transform={grow}>{word}</g></g>));
        }

        let pill = self.pill.map(|color| {
            let w = line_widths.iter().copied().fold(0.0, f32::max) + self.padding * 2.0;
            let h = line_height * count as f32 + self.padding;
            let r = line_height.midpoint(self.padding);
            let (left, top) = (-w / 2.0, top - self.padding / 2.0);
            fframes::svgr!(
                <rect x={left} y={top} width={w} height={h} rx={r} fill={color} opacity={self.pill_opacity} />
            )
        });
        let pill = pill.unwrap_or_else(Svgr::empty);

        let start = page.start_ms as f32 / 1000.0;
        let end = (page.start_ms + page.duration_ms) as f32 / 1000.0;
        let p = progress(t - start, 0.0, self.enter);
        let eased = 1.0 - (1.0 - p).powi(3);
        let gone = progress(t, end - self.exit, end);
        let (opacity, rise, grow) = match self.entrance {
            PageEntrance::None => (1.0, 0.0, 1.0),
            PageEntrance::Fade => (p, 0.0, 1.0),
            PageEntrance::Rise => (p, (1.0 - eased) * line_height * 0.5, 1.0),
            PageEntrance::Pop => (p, 0.0, 0.8 + 0.2 * eased),
        };
        let opacity = opacity * (1.0 - gone);
        let place = Transform::translate(self.x, self.y + rise);
        let size = Transform::scale(scale * grow);
        fframes::svgr!(
            <g transform={place} opacity={opacity}>
                <g transform={size}>{pill}{words}</g>
            </g>
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn words() -> Vec<CaptionWord> {
        [
            ("Hello", 0.0, 300.0),
            (" there,", 300.0, 700.0),
            (" how", 900.0, 1100.0),
            (" are", 1100.0, 1300.0),
            (" you", 1300.0, 1500.0),
            ("  ", 1500.0, 1600.0),
            (" today?", 2400.0, 2900.0),
        ]
        .into_iter()
        .map(|(text, start, end)| CaptionWord::new(text, start, end))
        .collect()
    }

    #[test]
    fn combine_zero_gives_a_page_per_word() {
        let pages = tiktok_pages(&words(), 0.0);
        let texts: Vec<_> = pages.iter().map(|p| p.text.as_str()).collect();
        assert_eq!(texts, ["Hello", "there,", "how", "are", "you", "today?"]);
        assert_eq!(pages[1].start_ms, 300.0);
        assert_eq!(pages[1].duration_ms, 600.0, "until the next page starts");
        assert_eq!(pages[5].duration_ms, 500.0, "the last ends with its word");
    }

    #[test]
    fn pages_combine_words_within_the_window() {
        let pages = tiktok_pages(&words(), 1200.0);
        let texts: Vec<_> = pages.iter().map(|p| p.text.as_str()).collect();
        assert_eq!(texts, ["Hello there, how are", "you today?"]);
        assert_eq!(pages[0].tokens.len(), 4);
        assert_eq!(pages[0].tokens[1].text, " there,");
        assert_eq!(
            (pages[0].tokens[2].from_ms, pages[0].tokens[2].to_ms),
            (900.0, 1100.0)
        );
        assert_eq!(pages[0].duration_ms, 1300.0);
    }

    #[test]
    fn words_without_spaces_stay_on_their_page() {
        let cues = [
            CaptionWord::new("One", 0.0, 900.0),
            CaptionWord::new("Two", 5000.0, 6000.0),
        ];
        assert_eq!(tiktok_pages(&cues, 0.0).len(), 1);
        assert_eq!(tiktok_pages(&[], 1200.0), Vec::new());
    }

    #[test]
    fn long_pages_shrink_to_their_lines() {
        let widths = [300.0; 8];
        assert_eq!(wrap(&widths, 20.0, 1000.0), [0, 0, 0, 1, 1, 1, 2, 2]);
        let (scale, lines) = fit(&widths, 20.0, 1000.0, 2);
        assert!(scale < 1.0);
        assert_eq!(lines.last(), Some(&1));
        let (scale, _) = fit(&widths, 20.0, 1000.0, 3);
        assert_eq!(scale, 1.0);
    }

    #[test]
    fn the_word_being_said_lights_up() {
        let pages = tiktok_pages(&words(), 1200.0);
        let captions = WordCaptions {
            pages,
            fade: 0.1,
            ..WordCaptions::default()
        };
        let token = &captions.pages[0].tokens[2];
        assert_eq!(captions.lit(token, 0.85), 0.0);
        assert_eq!(captions.lit(token, 1.0), 1.0);
        assert_eq!(captions.lit(token, 1.25), 0.0);
        assert!(captions.page_at(1250.0).is_some());
        assert_eq!(
            captions.page_at(2500.0).map(|p| p.text.as_str()),
            Some("you today?")
        );
        assert!(captions.page_at(3000.0).is_none(), "after the last word");
    }
}
