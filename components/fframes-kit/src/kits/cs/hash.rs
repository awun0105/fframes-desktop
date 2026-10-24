//! Hash tables with separate chaining, recorded step by step.

use fframes::{Color, Svgr};
use kurbo::{CubicBez, Point, RoundedRect, Vec2};

use crate::{
    anim::{FadeIn, FadeOut, fade},
    geometry::Shape,
    motion::{cubic_in_out, progress},
    text::{TextAnchor, line},
    theme::{TextStyle, Theme},
    ui::{Component, Themed, Ui},
};

use super::{
    array::CellColors,
    keyed::{StepAt, arrow_along, find, mix, notes, sample, tween, tween_links},
    list::item,
};

/// One step of a hash table.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HashStep {
    /// The chain of each bucket, each entry with an id that stays with it.
    pub buckets: Vec<Vec<(u32, String)>>,
    /// The bucket this step looks at.
    pub bucket: Option<usize>,
    /// The entries this step looks at.
    pub highlight: Vec<u32>,
    /// An entry that was searched for and found.
    pub found: Option<u32>,
    /// What happens in this step, in a few words.
    pub note: String,
}

impl HashStep {
    fn plain(&self) -> bool {
        self.bucket.is_none() && self.highlight.is_empty() && self.found.is_none()
    }
}

/// The steps of a hash table, recorded once (in a constructor or a `OnceLock`) and sampled by
/// time with [`HashTrace::at`]. [`HashTrace::record`] starts a [`HashRecorder`];
/// [`HashTrace::new`] takes steps of your own.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct HashTrace {
    steps: Vec<HashStep>,
    /// The longest chain of any step.
    longest: usize,
}

impl HashTrace {
    pub fn new(steps: Vec<HashStep>) -> Self {
        let longest = steps
            .iter()
            .flat_map(|s| s.buckets.iter().map(Vec::len))
            .max()
            .unwrap_or(0);
        Self { steps, longest }
    }

    /// Records operations on a table of `buckets` buckets (at least 1) holding `keys`. A key
    /// goes into bucket `key mod buckets`.
    pub fn record(buckets: usize, keys: &[i64]) -> HashRecorder {
        let mut r = HashRecorder {
            steps: Vec::new(),
            buckets: vec![Vec::new(); buckets.max(1)],
            next_id: 0,
        };
        for &key in keys {
            let b = r.hash(key);
            if !r.buckets[b].iter().any(|&(_, k)| k == key) {
                let id = r.next_id;
                r.next_id += 1;
                r.buckets[b].push((id, key));
            }
        }
        r.steps.push(r.step(String::new()));
        r
    }

    pub fn steps(&self) -> &[HashStep] {
        &self.steps
    }

    pub fn len(&self) -> usize {
        self.steps.len()
    }

    pub fn is_empty(&self) -> bool {
        self.steps.is_empty()
    }

    /// The step `seconds` after the start, one step every `step_time` seconds. Before the
    /// start it is step 0, after the end the last step.
    pub fn at(&self, seconds: f32, step_time: f32) -> Option<StepAt<'_, HashStep>> {
        sample(&self.steps, seconds, step_time)
    }
}

/// Records operations on a hash table, from [`HashTrace::record`]: every operation hashes the
/// key to its bucket, then walks the bucket's chain one entry a step.
#[derive(Debug, Clone)]
pub struct HashRecorder {
    steps: Vec<HashStep>,
    buckets: Vec<Vec<(u32, i64)>>,
    next_id: u32,
}

impl HashRecorder {
    fn hash(&self, key: i64) -> usize {
        key.rem_euclid(self.buckets.len() as i64) as usize
    }

    fn step(&self, note: String) -> HashStep {
        HashStep {
            buckets: self
                .buckets
                .iter()
                .map(|chain| chain.iter().map(|&(id, k)| (id, k.to_string())).collect())
                .collect(),
            note,
            ..HashStep::default()
        }
    }

    fn push(&mut self, bucket: usize, highlight: Vec<u32>, found: Option<u32>, note: String) {
        let step = HashStep {
            bucket: Some(bucket),
            highlight,
            found,
            ..self.step(note)
        };
        self.steps.push(step);
    }

    /// Hashes `key` and walks its chain; gives the bucket and where the key is in it.
    fn locate(&mut self, key: i64) -> (usize, Option<usize>) {
        let b = self.hash(key);
        let n = self.buckets.len();
        self.push(b, vec![], None, format!("{key} mod {n} = {b}"));
        for k in 0..self.buckets[b].len() {
            let (id, here) = self.buckets[b][k];
            if here == key {
                return (b, Some(k));
            }
            self.push(b, vec![id], None, format!("{here} is not {key}"));
        }
        (b, None)
    }

    /// Adds `key` at the end of its bucket's chain.
    pub fn insert(mut self, key: i64) -> Self {
        let (b, at) = self.locate(key);
        if let Some(k) = at {
            let id = self.buckets[b][k].0;
            self.push(b, vec![], Some(id), format!("{key} is already there"));
        } else {
            let id = self.next_id;
            self.next_id += 1;
            self.buckets[b].push((id, key));
            self.push(b, vec![id], None, format!("insert {key} in bucket {b}"));
        }
        self
    }

    /// Looks `key` up.
    pub fn find(mut self, key: i64) -> Self {
        let (b, at) = self.locate(key);
        match at {
            Some(k) => {
                let id = self.buckets[b][k].0;
                self.push(b, vec![], Some(id), format!("found {key}"));
            }
            None => self.push(b, vec![], None, format!("{key} is not there")),
        }
        self
    }

    /// Takes `key` out of its chain.
    pub fn remove(mut self, key: i64) -> Self {
        let (b, at) = self.locate(key);
        match at {
            Some(k) => {
                let id = self.buckets[b][k].0;
                let note = format!("remove {key}");
                self.push(b, vec![id], None, note.clone());
                self.buckets[b].remove(k);
                self.push(b, vec![], None, note);
            }
            None => self.push(b, vec![], None, format!("{key} is not there")),
        }
        self
    }

    /// The trace, ending with a step that lights nothing up.
    pub fn finish(mut self) -> HashTrace {
        if self.steps.last().is_some_and(|s| !s.plain()) {
            let note = self
                .steps
                .last()
                .map(|s| s.note.clone())
                .unwrap_or_default();
            self.steps.push(self.step(note));
        }
        HashTrace::new(self.steps)
    }
}

/// The pseudo id of bucket `b` among the entries, for links from a bucket to its chain.
fn bucket_id(b: usize) -> u32 {
    u32::MAX - b as u32
}

/// A hash table: a column of numbered buckets, each `height` pixels tall and `gap` apart,
/// with its chain of entries (`cell` pixels wide, `link` pixels apart) linked to its right,
/// centred on `center` (units of `ui.space`). It plays a [`HashTrace`], one step every
/// `step_time` seconds from `delay`: the bucket a key hashes to lights up, the entries walked
/// light up one by one, a found entry turns green, new entries slide in at the end of their
/// chain and removed ones leave as the chain closes. The whole figure fades with `enter` and
/// `exit`.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "studio", derive(fframes_kit_macros::Props))]
pub struct HashTable<'a> {
    pub trace: Option<&'a HashTrace>,
    pub delay: f32,
    pub step_time: f32,
    pub center: (f64, f64),
    pub bucket: f32,
    pub cell: f32,
    pub height: f32,
    pub gap: f32,
    pub link: f32,
    pub notes: bool,
    pub colors: CellColors,
    pub bucket_color: Color,
    pub link_color: Color,
    pub index_color: Color,
    pub value_style: TextStyle,
    pub small_style: TextStyle,
    pub enter: Option<FadeIn>,
    pub exit: Option<FadeOut>,
}

impl Themed for HashTable<'_> {
    fn themed(t: &Theme) -> Self {
        let mut value_style = t.typography.body;
        value_style.weight = 600;
        Self {
            trace: None,
            delay: 0.0,
            step_time: 0.7,
            center: (0.0, 0.0),
            bucket: 64.0,
            cell: 96.0,
            height: 56.0,
            gap: 14.0,
            link: 48.0,
            notes: true,
            colors: CellColors::themed(t),
            bucket_color: t.colors.surface,
            link_color: t.colors.text,
            index_color: t.colors.muted,
            value_style,
            small_style: t.typography.caption,
            enter: None,
            exit: None,
        }
    }
}

impl Default for HashTable<'_> {
    fn default() -> Self {
        Self::themed(&Theme::default())
    }
}

fn fill_of(colors: &CellColors, step: &HashStep, id: u32) -> Color {
    if step.found == Some(id) {
        colors.found
    } else if step.highlight.contains(&id) {
        colors.compare
    } else {
        colors.base
    }
}

impl<'a> Component<'a> for HashTable<'a> {
    fn render(self, ui: &mut Ui<'a, '_, '_>) -> Svgr<'a> {
        let Some(trace) = self.trace else {
            return Svgr::empty();
        };
        let Some(at) = trace.at(ui.seconds() - self.delay, self.step_time) else {
            return Svgr::empty();
        };
        let middle = ui.space.to_px(self.center.0, self.center.1);
        let (bw, cw, h, gap, link) = (
            f64::from(self.bucket),
            f64::from(self.cell),
            f64::from(self.height),
            f64::from(self.gap),
            f64::from(self.link),
        );
        let rows = at.step.buckets.len();
        let width = bw + trace.longest as f64 * (link + cw);
        let bucket_x = middle.x - width / 2.0 + bw / 2.0;
        let row_y = |b: usize| middle.y + (b as f64 - (rows as f64 - 1.0) / 2.0) * (h + gap);
        let entry_x = |k: usize| bucket_x + bw / 2.0 + link + cw / 2.0 + k as f64 * (link + cw);
        let place = |step: &HashStep| -> Vec<(u32, Point)> {
            step.buckets
                .iter()
                .enumerate()
                .flat_map(|(b, chain)| {
                    chain
                        .iter()
                        .enumerate()
                        .map(move |(k, (id, _))| (*id, Point::new(entry_x(k), row_y(b))))
                })
                .collect()
        };
        let links = |step: &HashStep| -> Vec<(u32, u32)> {
            step.buckets
                .iter()
                .enumerate()
                .flat_map(|(b, chain)| {
                    let ids: Vec<u32> = std::iter::once(bucket_id(b))
                        .chain(chain.iter().map(|(id, _)| *id))
                        .collect();
                    ids.windows(2).map(|w| (w[0], w[1])).collect::<Vec<_>>()
                })
                .collect()
        };
        let move_time = self.step_time * 0.7;
        let blend = (self.step_time * 0.3).min(0.2);
        let p = f64::from(cubic_in_out(progress(at.into, 0.0, move_time)));
        let q = progress(at.into, 0.0, blend);
        let tweens = tween(
            &place(at.previous),
            &place(at.step),
            p,
            Vec2::new(link + cw, 0.0),
            Vec2::new(0.0, -h * 0.8),
            0.0,
        );

        // the buckets with their numbers
        let lit = |step: &HashStep, b: usize| if step.bucket == Some(b) { 1.0 } else { 0.0 };
        let mut buckets = Vec::new();
        for b in 0..rows {
            let y = row_y(b);
            let glow = lit(at.previous, b) + (lit(at.step, b) - lit(at.previous, b)) * q;
            let fill = mix(self.bucket_color, self.colors.compare, glow);
            let rect = RoundedRect::new(
                bucket_x - bw / 2.0,
                y - h / 2.0,
                bucket_x + bw / 2.0,
                y + h / 2.0,
                8.0,
            );
            buckets.push(
                Shape {
                    path: kurbo::Shape::to_path(&rect, 0.1),
                    fill: Some(fill),
                    stroke: Some(self.colors.text.with_alpha(64)),
                    stroke_width: 1.5,
                    dash: None,
                    opacity: 1.0,
                }
                .draw(),
            );
            buckets.push(line(
                bucket_x as f32,
                (y + self.small_style.size as f64 * 0.35) as f32,
                self.small_style,
                if glow > 0.5 {
                    self.colors.text_on(fill)
                } else {
                    self.index_color
                },
                TextAnchor::Middle,
                0.0,
                1.0,
                b.to_string().into(),
            ));
        }

        let mut arrows = Vec::new();
        for (from, to, opacity) in tween_links(&links(at.previous), &links(at.step), p, true) {
            let Some(b) = find(&tweens, to) else { continue };
            let (start, opacity) = if from > u32::MAX - rows as u32 {
                let row = (u32::MAX - from) as usize;
                (
                    Point::new(bucket_x + bw / 2.0, row_y(row)),
                    opacity * b.opacity,
                )
            } else {
                let Some(a) = find(&tweens, from) else {
                    continue;
                };
                (
                    Point::new(a.at.x + cw / 2.0, a.at.y),
                    opacity * a.opacity.min(b.opacity),
                )
            };
            let end = Point::new(b.at.x - cw / 2.0, b.at.y);
            let curve = CubicBez::new(
                start,
                start.lerp(end, 1.0 / 3.0),
                start.lerp(end, 2.0 / 3.0),
                end,
            );
            arrows.push(arrow_along(
                curve,
                true,
                self.link_color,
                2.5,
                opacity as f32,
            ));
        }

        let entries: Vec<Svgr<'a>> = tweens
            .iter()
            .map(|t| {
                let now = fill_of(&self.colors, at.step, t.id);
                let fill = match (t.from, t.to) {
                    (Some(_), Some(_)) => mix(fill_of(&self.colors, at.previous, t.id), now, q),
                    (Some(_), None) => fill_of(&self.colors, at.previous, t.id),
                    _ => now,
                };
                let step = if t.to.is_some() { at.step } else { at.previous };
                let label = step
                    .buckets
                    .iter()
                    .flatten()
                    .find(|(id, _)| *id == t.id)
                    .map_or("", |(_, label)| label.as_str());
                item(
                    t.at,
                    cw,
                    h,
                    fill,
                    &self.colors,
                    self.value_style,
                    label,
                    t.opacity as f32,
                    t.at.x,
                )
            })
            .collect();

        let mut parts = vec![buckets.into(), arrows.into(), entries.into()];
        if self.notes {
            let y =
                row_y(rows.saturating_sub(1)) + h / 2.0 + 36.0 + self.small_style.size as f64 * 0.8;
            parts.push(notes(
                &at.previous.note,
                &at.step.note,
                at.into,
                middle.x as f32,
                y as f32,
                self.small_style,
                self.colors.text,
                blend,
            ));
        }
        fade(ui, parts.into(), self.enter.as_ref(), self.exit.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn keys(step: &HashStep, b: usize) -> Vec<&str> {
        step.buckets[b].iter().map(|(_, k)| k.as_str()).collect()
    }

    #[test]
    fn keys_chain_in_their_bucket() {
        let trace = HashTrace::record(5, &[12, 7, 3])
            .insert(17)
            .insert(-3)
            .find(7)
            .remove(12)
            .finish();
        let last = trace.steps().last().unwrap();
        // 12, 7, 17 and -3 hash to 2, 3 to 3
        assert_eq!(keys(last, 2), ["7", "17", "-3"]);
        assert_eq!(keys(last, 3), ["3"]);
        let steps = trace.steps();
        assert_eq!(steps[1].note, "17 mod 5 = 2");
        assert_eq!(steps[2].note, "12 is not 17");
        assert!(
            steps
                .iter()
                .any(|s| s.note == "found 7" && s.found == Some(1))
        );
        assert!(last.plain());
        assert_eq!(trace.longest, 4);
    }

    #[test]
    fn duplicates_and_missing_keys() {
        let trace = HashTrace::record(3, &[4, 4]).insert(4).remove(9).finish();
        assert_eq!(trace.steps()[0].buckets[1].len(), 1);
        let notes: Vec<&str> = trace.steps().iter().map(|s| s.note.as_str()).collect();
        assert!(notes.contains(&"4 is already there"));
        assert!(notes.contains(&"9 is not there"));
    }
}
