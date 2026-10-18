//! Streaming fade-in of new text.
//!
//! The text is committed to layout at once; a purely cosmetic veil dissolves over the
//! characters that just arrived. [`ElemVeil`] tracks one rendered text element: it remembers the
//! text it saw last, registers every appended byte range as a chunk with its arrival time, and
//! reports each chunk's current opacity until it has faded. A chunk fades exactly once, so text
//! that has settled never animates again, and a settled element reports no spans at all.
//!
//! [`apply_veil`] multiplies those opacities into the colours of the [`TextRun`]s covering each
//! chunk. A colour-only split of a run cannot change layout: gpui shapes adjacent runs of the same
//! font as one line, so wrapping stays byte-identical to the unsplit text.
//!
//! The fade lasts `clamp(3 x the gap between appends, 120 ms, 400 ms)`, eases as
//! `1 - (1 - p)^1.6`, and speeds up by 30% for every chunk beyond two fading at once.

use std::collections::HashMap;
use std::ops::Range;
use std::time::Instant;

use gpui::TextRun;

/// The seed of the append-gap average, in milliseconds.
pub const EMA_SEED_MS: f32 = 160.0;
/// Shortest and longest fade of one chunk, in milliseconds.
pub const MIN_FADE_MS: f32 = 120.0;
pub const MAX_FADE_MS: f32 = 400.0;
/// Exponent of the dissolve curve.
pub const CURVE_POW: f32 = 1.6;
/// A gap longer than this counts as this long in the average.
const GAP_CLAMP_MS: f32 = 1000.0;

/// A byte range of a text element and its current opacity (0 to 1).
pub type VeilSpan = (Range<usize>, f32);

#[derive(Debug, Clone)]
struct Chunk {
    range: Range<usize>,
    started: Instant,
    /// Fixed at arrival from the cadence of the stream.
    duration_ms: f32,
}

/// Text opacity at fade progress `p` (0 to 1).
pub fn opacity(p: f32) -> f32 {
    1.0 - (1.0 - p.clamp(0.0, 1.0)).powf(CURVE_POW)
}

/// Fade length of a chunk for the current average gap between appends.
pub fn duration_ms(ema_ms: f32) -> f32 {
    (ema_ms * 3.0).clamp(MIN_FADE_MS, MAX_FADE_MS)
}

/// Speed-up while many chunks fade at once.
pub fn boost(active_chunks: usize) -> f32 {
    1.0 + 0.3 * active_chunks.saturating_sub(2) as f32
}

fn next_ema(ema_ms: f32, gap_ms: f32) -> f32 {
    ema_ms * 0.7 + gap_ms.min(GAP_CLAMP_MS) * 0.3
}

/// The longest common prefix of `a` and `b`, snapped back to a char boundary of both.
fn common_prefix(a: &str, b: &str) -> usize {
    let mut p = a
        .as_bytes()
        .iter()
        .zip(b.as_bytes())
        .take_while(|(x, y)| x == y)
        .count();
    while p > 0 && !(a.is_char_boundary(p) && b.is_char_boundary(p)) {
        p -= 1;
    }
    p
}

/// The fade state of one text element.
#[derive(Debug)]
pub struct ElemVeil {
    prev: String,
    chunks: Vec<Chunk>,
    ema_ms: f32,
    last_append: Option<Instant>,
}

impl Default for ElemVeil {
    fn default() -> Self {
        Self {
            prev: String::new(),
            chunks: Vec::new(),
            ema_ms: EMA_SEED_MS,
            last_append: None,
        }
    }
}

impl ElemVeil {
    /// Moves to `text` at `now`: registers a chunk for the new bytes, drops settled chunks and
    /// returns the spans still fading. Calling it twice with the same text and time is harmless.
    pub fn advance(&mut self, text: &str, now: Instant) -> Vec<VeilSpan> {
        if text != self.prev {
            // A rewrite that is not an append (a block re-derived by the parser) keeps the fades
            // of the common prefix and veils only what changed.
            let p = common_prefix(&self.prev, text);
            self.chunks.retain_mut(|c| {
                c.range.end = c.range.end.min(p);
                c.range.start < c.range.end
            });
            if text.len() > p {
                if let Some(last) = self.last_append {
                    let gap = now.saturating_duration_since(last).as_secs_f32() * 1000.0;
                    self.ema_ms = next_ema(self.ema_ms, gap);
                }
                self.last_append = Some(now);
                self.chunks.push(Chunk {
                    range: p..text.len(),
                    started: now,
                    duration_ms: duration_ms(self.ema_ms),
                });
            }
            self.prev.clear();
            self.prev.push_str(text);
        }
        let elapsed = |c: &Chunk| now.saturating_duration_since(c.started).as_secs_f32() * 1000.0;
        let speed = boost(self.chunks.len());
        self.chunks.retain(|c| elapsed(c) * speed < c.duration_ms);
        let speed = boost(self.chunks.len());
        self.chunks
            .iter()
            .map(|c| {
                let progress = (elapsed(c) * speed / c.duration_ms).clamp(0.0, 1.0);
                (c.range.clone(), opacity(progress))
            })
            .collect()
    }

    /// Whether a chunk was still fading at the last [`Self::advance`].
    pub fn is_fading(&self) -> bool {
        !self.chunks.is_empty()
    }
}

/// The fade state of one streaming row, by the index of the text element in the row. The indices
/// stay stable across appends because the parser only rebuilds the tail.
#[derive(Debug, Default)]
pub struct RowVeil {
    elems: HashMap<usize, ElemVeil>,
    /// Text first seen while seeding is the baseline, not new content: a row that already had
    /// text when the transcript showed it does not dissolve as a whole.
    seeding: bool,
}

impl RowVeil {
    /// A veil whose first pass adopts what is already there.
    pub fn seeded() -> Self {
        Self {
            elems: HashMap::new(),
            seeding: true,
        }
    }

    /// Ends the seeding pass; elements first seen later are new content and fade.
    pub fn finish_seeding(&mut self) {
        self.seeding = false;
    }

    pub fn advance(&mut self, elem: usize, text: &str, now: Instant) -> Vec<VeilSpan> {
        if self.seeding && !self.elems.contains_key(&elem) {
            let baseline = ElemVeil {
                prev: text.to_owned(),
                ..ElemVeil::default()
            };
            self.elems.insert(elem, baseline);
            return Vec::new();
        }
        self.elems.entry(elem).or_default().advance(text, now)
    }

    /// Whether any element is still fading, which asks for another frame.
    pub fn is_fading(&self) -> bool {
        self.elems.values().any(ElemVeil::is_fading)
    }

    /// Forgets the elements at `count` and beyond, which the row no longer renders, so their
    /// unfinished chunks never keep asking for frames.
    pub fn retain_elements(&mut self, count: usize) {
        self.elems.retain(|ix, _| *ix < count);
    }
}

/// Whether new text fades in: not while the reader prefers reduced motion, and only for a row
/// that is still streaming or has a fade in flight.
pub fn veil_wanted(reduced_motion: bool, streaming: bool, fading: bool) -> bool {
    !reduced_motion && (streaming || fading)
}

/// Multiplies the span opacities into the run colours, splitting runs at span boundaries. Text,
/// fonts and the total length are untouched, so shaping and wrapping cannot change.
pub fn apply_veil(runs: Vec<TextRun>, spans: &[VeilSpan]) -> Vec<TextRun> {
    if spans.is_empty() || spans.iter().all(|(_, a)| *a >= 1.0) {
        return runs;
    }
    let mut out = Vec::with_capacity(runs.len() + spans.len() * 2);
    let mut pos = 0usize;
    for run in runs {
        let (start, end) = (pos, pos + run.len);
        pos = end;
        let mut cuts = vec![start, end];
        for (r, _) in spans {
            if r.start > start && r.start < end {
                cuts.push(r.start);
            }
            if r.end > start && r.end < end {
                cuts.push(r.end);
            }
        }
        cuts.sort_unstable();
        cuts.dedup();
        for w in cuts.windows(2) {
            let (s, e) = (w[0], w[1]);
            let mut piece = run.clone();
            piece.len = e - s;
            let alpha = spans
                .iter()
                .find(|(r, _)| r.start <= s && e <= r.end)
                .map(|(_, a)| *a)
                .filter(|a| *a < 1.0);
            if let Some(alpha) = alpha {
                piece.color = piece.color.opacity(alpha);
                piece.background_color = piece.background_color.map(|c| c.opacity(alpha));
                if let Some(u) = &mut piece.underline {
                    u.color = u.color.map(|c| c.opacity(alpha));
                }
                if let Some(st) = &mut piece.strikethrough {
                    st.color = st.color.map(|c| c.opacity(alpha));
                }
            }
            out.push(piece);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{font, px};
    use std::time::Duration;

    fn at(base: Instant, ms: u64) -> Instant {
        base + Duration::from_millis(ms)
    }

    fn run(len: usize, color: gpui::Hsla) -> TextRun {
        TextRun {
            len,
            font: font("Test"),
            color,
            background_color: None,
            underline: None,
            strikethrough: None,
        }
    }

    #[test]
    fn first_text_fades_and_settles_once() {
        let t0 = Instant::now();
        let mut v = ElemVeil::default();
        assert_eq!(v.advance("hello", t0), vec![(0..5, 0.0)]);
        let mid = v.advance("hello", at(t0, 250));
        assert!(mid[0].1 > 0.0 && mid[0].1 < 1.0);
        assert!(v.advance("hello", at(t0, 600)).is_empty());
        assert!(!v.is_fading());
        assert!(v.advance("hello", at(t0, 700)).is_empty());
    }

    #[test]
    fn appended_chunks_fade_independently() {
        let t0 = Instant::now();
        let mut v = ElemVeil::default();
        v.advance("one ", t0);
        let spans = v.advance("one two ", at(t0, 100));
        assert_eq!(spans.len(), 2);
        assert_eq!((spans[0].0.clone(), spans[1].0.clone()), (0..4, 4..8));
        assert!(spans[0].1 > spans[1].1);
        let spans = v.advance("one two ", at(t0, 410));
        assert_eq!(spans.len(), 1);
        assert_eq!(spans[0].0, 4..8);
    }

    #[test]
    fn an_element_the_row_stopped_rendering_stops_asking_for_frames() {
        let t0 = Instant::now();
        let mut v = RowVeil::seeded();
        v.finish_seeding();
        v.advance(0, "kept", t0);
        v.advance(5, "gone", t0);
        // Element 5 is never advanced again, so only forgetting it ends its fade.
        v.advance(0, "kept", at(t0, 2_000));
        assert!(v.is_fading());
        v.retain_elements(1);
        assert!(!v.is_fading());
    }

    #[test]
    fn faded_text_never_reanimates_on_append() {
        let t0 = Instant::now();
        let mut v = ElemVeil::default();
        v.advance("stable", t0);
        assert!(v.advance("stable", at(t0, 600)).is_empty());
        assert_eq!(v.advance("stable more", at(t0, 700)), vec![(6..11, 0.0)]);
    }

    #[test]
    fn a_rewrite_keeps_the_prefix_and_reveils_the_tail() {
        let t0 = Instant::now();
        let mut v = ElemVeil::default();
        v.advance("intro **bol", t0);
        let spans = v.advance("intro bold", at(t0, 100));
        assert_eq!(spans.len(), 2);
        assert_eq!(spans[0].0, 0..6);
        assert_eq!(spans[1].0, 6..10);
    }

    #[test]
    fn a_seeded_row_adopts_existing_text_and_fades_only_what_follows() {
        let t0 = Instant::now();
        let mut row = RowVeil::seeded();
        assert!(row.advance(0, "already here", t0).is_empty());
        assert!(!row.is_fading());
        assert_eq!(
            row.advance(0, "already here plus", at(t0, 100)),
            vec![(12..17, 0.0)]
        );
        row.finish_seeding();
        assert_eq!(row.advance(1, "new block", at(t0, 200)), vec![(0..9, 0.0)]);
    }

    #[test]
    fn the_curve_and_timing_constants_hold() {
        assert_eq!(opacity(0.0), 0.0);
        assert_eq!(opacity(1.0), 1.0);
        assert!(opacity(0.5) > 0.5);
        assert_eq!(duration_ms(160.0), 400.0);
        assert_eq!(duration_ms(30.0), 120.0);
        assert_eq!(duration_ms(60.0), 180.0);
        assert_eq!(boost(2), 1.0);
        assert!((boost(3) - 1.3).abs() < 1e-6);
    }

    #[test]
    fn reduced_motion_turns_the_fade_off() {
        assert!(veil_wanted(false, true, false));
        assert!(veil_wanted(false, false, true));
        assert!(!veil_wanted(false, false, false));
        assert!(!veil_wanted(true, true, true));
    }

    #[test]
    fn applying_a_veil_keeps_the_text_length_and_fonts() {
        let runs = vec![run(4, gpui::white()), run(6, gpui::white())];
        let out = apply_veil(runs.clone(), &[(2..8, 0.5)]);
        assert_eq!(
            out.iter().map(|r| r.len).collect::<Vec<_>>(),
            vec![2, 2, 4, 2]
        );
        assert!(out.iter().all(|r| r.font == runs[0].font));
        assert_eq!(
            out.iter().map(|r| r.color.a).collect::<Vec<_>>(),
            vec![1.0, 0.5, 0.5, 1.0]
        );
        assert_eq!(apply_veil(runs.clone(), &[]).len(), 2);
        assert_eq!(apply_veil(runs, &[(0..10, 1.0)]).len(), 2);
    }

    #[test]
    fn applying_a_veil_fades_decorations_too() {
        let mut r = run(5, gpui::white());
        r.background_color = Some(gpui::white());
        r.underline = Some(gpui::UnderlineStyle {
            color: Some(gpui::white()),
            thickness: px(1.0),
            wavy: false,
        });
        let out = apply_veil(vec![r], &[(0..5, 0.25)]);
        assert_eq!(out[0].background_color.unwrap().a, 0.25);
        assert_eq!(out[0].underline.as_ref().unwrap().color.unwrap().a, 0.25);
    }
}
