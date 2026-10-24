//! `TransitionSeries` in a real render: the clip ids of wipes on screen at once, which
//! sequences draw around a transition, and the warnings `inspect` shows.

use std::sync::Mutex;

use components_gallery::GalleryMedia;
use fframes::{Previewer, RenderOptions, StaticMediaProvider};
use fframes_kit::prelude::*;

/// Two series, each a second of one rectangle, a 0.5 s wipe and a second of another: the
/// wipe runs from frame 15 to 30.
#[derive(Debug)]
struct Probe {
    theme: Theme,
    keys: [Option<&'static str>; 2],
    /// Adds a transition at the end of the first series, which has nothing to go to.
    misplaced: bool,
    drawn: Mutex<String>,
}

fn panel<'a>(fill: &'static str) -> Children<'a> {
    children(move |_| svgr!(<rect width="640" height="360" fill={fill} />))
}

impl Video for Probe {
    const FPS: usize = 30;
    const WIDTH: usize = 640;
    const HEIGHT: usize = 360;

    fn duration(&self) -> Duration<'_> {
        Duration::Seconds(2.0)
    }

    fn audio(&self) -> AudioMap<'_> {
        AudioMap::none()
    }

    fn render_frame<'a>(&'a self, frame: Frame, ctx: &FFramesContext<'a, '_>) -> Svgr<'a> {
        let mut ui = Ui::new(frame, ctx, &self.theme);
        let timing = Timing::Linear {
            duration: 0.5,
            easing: RateFunc::Linear,
        };
        let wipe = Wipe {
            size: (640.0, 360.0),
            ..Wipe::default()
        };
        let series: Svgr<'a> = self
            .keys
            .iter()
            .enumerate()
            .map(|(i, &key)| {
                let mut items = vec![
                    TransitionSeries::sequence(1.0, panel("#e11d48")),
                    TransitionSeries::transition(timing, wipe),
                    TransitionSeries::sequence(1.0, panel("#2563eb")),
                ];
                if self.misplaced && i == 0 {
                    items.push(TransitionSeries::transition(timing, Fade::default()));
                }
                ui.show(TransitionSeries { key, items })
            })
            .collect();
        *self.drawn.lock().unwrap() = series.to_string();
        svgr!(<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 640 360" width="640" height="360">{series}</svg>)
    }
}

fn probe(keys: [Option<&'static str>; 2], misplaced: bool) -> Probe {
    Probe {
        theme: Theme::default(),
        keys,
        misplaced,
        drawn: Mutex::new(String::new()),
    }
}

/// The element tree of `frame`, as `Svgr` prints it.
fn drawn(probe: &Probe, frame: usize) -> String {
    let media = GalleryMedia::prepare().expect("gallery media");
    let options = RenderOptions {
        media: Some(&media),
        ..Default::default()
    };
    let mut previewer = Previewer::new(probe, &options).expect("previewer");
    previewer.svg(frame).expect("frame");
    probe.drawn.lock().unwrap().clone()
}

#[test]
fn two_wipes_in_one_frame_clip_with_their_own_ids() {
    let tree = drawn(&probe([Some("left"), Some("right")], false), 20);
    assert!(tree.contains("kit-transition-k-left-1"), "{tree}");
    assert!(tree.contains("kit-transition-k-right-1"), "{tree}");
    assert_eq!(tree.matches("tag_name: clipPath").count(), 2, "{tree}");
}

#[test]
fn both_sequences_draw_during_the_wipe_and_one_outside_it() {
    let p = probe([Some("a"), Some("b")], false);
    let rects = |frame| drawn(&p, frame).matches("tag_name: rect").count();
    // one panel per series before and after the wipe, two plus the clip rectangle during it;
    // on its first frame nothing is uncovered yet, so the entering panel is not drawn
    assert_eq!(rects(14), 2);
    assert_eq!(rects(15), 2);
    assert_eq!(rects(16), 6);
    assert_eq!(rects(29), 6);
    assert_eq!(rects(30), 2);
}

#[test]
fn the_first_frame_of_a_wipe_has_no_empty_clip() {
    let media = GalleryMedia::prepare().expect("gallery media");
    let options = RenderOptions {
        media: Some(&media),
        ..Default::default()
    };
    let p = probe([Some("a"), Some("b")], false);
    let mut previewer = Previewer::new(&p, &options).expect("previewer");
    let report = previewer.inspect(15).expect("inspect");
    assert!(
        !report
            .diagnostics
            .iter()
            .any(|d| d.message.contains("invalid")),
        "{:?}",
        report.diagnostics
    );
}

#[test]
fn a_transition_without_a_sequence_after_it_warns() {
    let media = GalleryMedia::prepare().expect("gallery media");
    let options = RenderOptions {
        media: Some(&media),
        ..Default::default()
    };
    let p = probe([Some("a"), Some("b")], true);
    let mut previewer = Previewer::new(&p, &options).expect("previewer");
    let report = previewer.inspect(10).expect("inspect");
    assert!(
        report.diagnostics.iter().any(|d| d
            .message
            .contains("transition 3 needs a sequence on both sides")),
        "{:?}",
        report.diagnostics
    );
}
