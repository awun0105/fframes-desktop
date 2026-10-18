use super::*;

#[derive(Debug)]
struct Clip(&'static str, usize, Option<(f32, f32)>);

impl Scene for Clip {
    fn duration(&self) -> Duration<'_> {
        Duration::Frames(self.1)
    }

    fn overlap(&self) -> Overlap {
        match self.2 {
            Some((previous, next)) => Overlap::PreviousAndNext { previous, next },
            _ => Overlap::None,
        }
    }

    fn editor_instance_key(&self) -> Option<&str> {
        Some(self.0)
    }

    fn render_frame<'a>(&'a self, _: Frame, _: &FFramesContext<'a, '_>) -> Svgr<'a> {
        Svgr::empty()
    }
}

fn layout(order: &[usize], next: f32) -> (usize, Vec<std::ops::Range<usize>>) {
    let clips = [
        Clip("first", 60, None),
        Clip("overlay", 60, Some((0.5, next))),
        Clip("second", 60, None),
    ];
    let scenes = ScenesWithAudio::new(&Scenes::from(
        clips
            .iter()
            .map(|clip| clip as &dyn Scene)
            .collect::<Vec<_>>(),
    ));
    let timing = crate::TimingDoc {
        version: 1,
        clips: order
            .iter()
            .map(|&i| crate::TimingClip {
                key: clips[i].0.into(),
                trim_start_frames: 0,
                trim_end_frames: 0,
            })
            .collect(),
    };
    let timeline = ResolvedScenesTimeline::from_scenes(
        &TimeBase {
            fps: 30,
            sample_rate: 48_000,
        },
        scenes.0.as_ref().unwrap(),
        Some(&timing),
        &|_| Ok(0.),
    )
    .unwrap();
    (
        timeline.total_scenes_duration,
        timeline.iter().map(|clip| clip.range.clone()).collect(),
    )
}

#[test]
fn a_reordered_last_scene_has_no_outgoing_overlap() {
    let (duration, ranges) = layout(&[2, 0, 1], 0.5);
    assert_eq!(ranges, [0..60, 60..120, 105..180]);
    assert_eq!(duration, 180);
}

#[test]
fn all_scene_orders_include_every_scene_end_in_the_duration() {
    for order in [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ] {
        for next in [0.5, 5.] {
            let (duration, ranges) = layout(&order, next);
            assert!(
                ranges
                    .iter()
                    .all(|range| range.start < range.end && range.end <= duration),
                "{order:?}, overlap {next}: {ranges:?}, duration {duration}"
            );
            assert_eq!(ranges.iter().map(|range| range.end).max(), Some(duration));
        }
    }
}
