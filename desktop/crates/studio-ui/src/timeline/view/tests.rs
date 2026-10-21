use super::*;
use crate::timeline::model::tests::report;

#[test]
fn ruler_click_scrubs_and_lane_click_selects() {
    let ruler_h = METRICS.ruler_h;
    assert_eq!(ruler_h, 26.);
    assert_eq!(hit_zone(100., 100.), HitZone::Ruler);
    assert_eq!(hit_zone(100. + ruler_h - 0.5, 100.), HitZone::Ruler);
    assert_eq!(hit_zone(100. + ruler_h, 100.), HitZone::Lanes);
    assert_eq!(hit_zone(300., 100.), HitZone::Lanes);
}

#[test]
fn zoom_slider_runs_from_fit_to_the_closest_zoom() {
    let fit = 25.;
    assert_eq!(zoom_to_fraction(fit, fit), 0.);
    assert_eq!(zoom_to_fraction(ZOOM_MAX, fit), 1.);
    assert_eq!(zoom_to_fraction(fit / 10., fit), 0.);
    assert_eq!(zoom_to_fraction(ZOOM_MAX * 10., fit), 1.);
    assert_eq!(zoom_to_fraction(10., ZOOM_MAX), 0.);
    assert_eq!(fraction_to_zoom(-1., fit), fit);
    for pps in [30., 100., 4_000.] {
        let back = fraction_to_zoom(zoom_to_fraction(pps, fit), fit);
        assert!((back / pps - 1.).abs() < 1e-3, "{pps} -> {back}");
    }
    assert!(fraction_to_zoom(0.25, fit) < fraction_to_zoom(0.75, fit));
}

#[gpui::test]
fn the_controls_turn_on_with_a_timeline_and_follow_the_zoom(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, cx| TimelineView::new(cx));
    let reported = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let _subscription = cx.update(|_, cx| {
        let reported = reported.clone();
        cx.subscribe(&view, move |_, event, _| {
            if let TimelineEvent::Controls(controls) = event {
                reported.borrow_mut().push(*controls);
            }
        })
    });
    view.update(cx, |view, cx| {
        view.install(thirty_clips(), Arc::from([]), 0, cx)
    });
    cx.run_until_parked();
    let fitted = *reported.borrow().last().expect("the controls are reported");
    assert!(fitted.enabled, "{fitted:?}");
    assert_eq!(fitted.zoom_fraction, 0.);
    view.update(cx, |view, cx| view.zoom(2., cx));
    cx.run_until_parked();
    let zoomed = *reported.borrow().last().unwrap();
    assert!(zoomed.zoom_fraction > 0., "{zoomed:?}");
}

/// 30 one-second clips at 10 fps.
fn thirty_clips() -> TimelineModel {
    let keys: Vec<_> = (0..30).map(|index| format!("clip-{index}")).collect();
    let scenes: Vec<_> = keys
        .iter()
        .enumerate()
        .map(|(index, key)| (key.as_str(), index * 10, index * 10 + 10))
        .collect();
    TimelineModel::new(report(10, &scenes)).unwrap()
}

#[gpui::test]
fn only_segments_near_the_view_are_painted(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, cx| TimelineView::new(cx));
    view.update(cx, |view, cx| {
        view.install(thirty_clips(), Arc::from([]), 0, cx)
    });
    cx.run_until_parked();
    assert_eq!(view.read_with(cx, |view, _| view.painted_segments), 30);
    view.update(cx, |view, cx| {
        let model = view.model.clone().unwrap();
        view.viewport.update_zoom(3., 0., &model);
        view.viewport.set_position(10., &model);
        cx.notify();
    });
    cx.run_until_parked();
    // [10 s, 13 s) and two seconds of padding either side: clips 7 to 15.
    let metrics = view.read_with(cx, |view, _| view.qualification_metrics());
    assert_eq!(metrics["painted_segments"], 9, "{metrics:#}");
    assert_eq!(metrics["clips"], 30);
}

#[gpui::test]
fn the_engine_snapshot_drives_the_clip_track(cx: &mut gpui::TestAppContext) {
    use crate::timeline::model::tests::snapshot;
    let (view, cx) = cx.add_window_view(|_, cx| TimelineView::new(cx));
    view.update(cx, |view, cx| {
        view.install(thirty_clips(), Arc::from([]), 0, cx)
    });
    cx.run_until_parked();
    // The document plays two clips; until the report catches up the second has no scene.
    view.update(cx, |view, cx| {
        view.set_snapshot(
            Some(snapshot(&[("clip-3", 10, 0, 0), ("clip-3", 10, 5, 0)])),
            cx,
        )
    });
    cx.run_until_parked();
    let (clips, painted) = view.read_with(cx, |view, _| {
        let spans: Vec<_> = view
            .clips
            .iter()
            .map(|c| (c.start, c.end, c.scene))
            .collect();
        (spans, view.painted_segments)
    });
    // The first clip keeps its scene's frames, the second follows it, and the 29
    // scenes no clip plays stay on their rows.
    assert_eq!(clips[..2], [(30, 40, Some(3)), (40, 45, None)]);
    assert_eq!(clips.len(), 31);
    assert!(!clips[2..].iter().any(|(_, _, scene)| *scene == Some(3)));
    assert_eq!(painted, 31);
    view.update(cx, |view, cx| view.set_snapshot(None, cx));
    assert_eq!(view.read_with(cx, |view, _| view.clips.len()), 30);
}

#[gpui::test]
fn a_minimap_press_centres_the_view_and_a_fitted_one_ignores_it(cx: &mut gpui::TestAppContext) {
    let (view, cx) = cx.add_window_view(|_, cx| TimelineView::new(cx));
    view.update(cx, |view, cx| {
        view.install(thirty_clips(), Arc::from([]), 0, cx)
    });
    cx.run_until_parked();
    let centre = |cx: &mut gpui::VisualTestContext| {
        let [left, top, width, height] = view.read_with(cx, |view, _| view.qualification_metrics())
            ["minimap_bounds"]
            .as_array()
            .expect("the minimap reports where it is")
            .iter()
            .map(|value| value.as_f64().unwrap() as f32)
            .collect::<Vec<_>>()[..]
        else {
            panic!("four numbers")
        };
        gpui::point(px(left + width / 2.), px(top + height / 2.))
    };
    // Fitted: the minimap is hidden and the press changes nothing.
    let at = centre(cx);
    cx.simulate_click(at, gpui::Modifiers::none());
    assert_eq!(view.read_with(cx, |view, _| view.viewport.position), 0.);

    view.update(cx, |view, cx| {
        let model = view.model.clone().unwrap();
        view.viewport.update_zoom(3., 0., &model);
        cx.notify();
    });
    cx.run_until_parked();
    // The bar's middle is 15 s; the 3 s view is centred on it.
    let at = centre(cx);
    cx.simulate_click(at, gpui::Modifiers::none());
    let position = view.read_with(cx, |view, _| view.viewport.position);
    assert!((position - 13.5).abs() < 1e-3, "{position}");
}

/// Three three-second clips at 10 fps, editable, with every edit the view asks of the
/// engine collected.
fn editable(
    cx: &mut gpui::TestAppContext,
) -> (
    gpui::Entity<TimelineView>,
    &mut gpui::VisualTestContext,
    std::rc::Rc<std::cell::RefCell<Vec<TimelineCommand>>>,
) {
    use crate::timeline::model::tests::snapshot;
    let (view, cx) = cx.add_window_view(|_, cx| TimelineView::new(cx));
    let edits = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let log = edits.clone();
    cx.update(|_, app| {
        app.subscribe(&view, move |_, event: &TimelineEvent, _| {
            if let TimelineEvent::Edit(command) = event {
                log.borrow_mut().push(command.clone());
            }
        })
        .detach();
    });
    view.update(cx, |view, cx| {
        let model =
            TimelineModel::new(report(10, &[("a", 0, 30), ("b", 30, 60), ("c", 60, 90)])).unwrap();
        view.install(model, Arc::from([]), 0, cx);
        view.set_snapshot(
            Some(snapshot(&[
                ("a", 30, 0, 0),
                ("b", 30, 0, 0),
                ("c", 30, 0, 0),
            ])),
            cx,
        );
    });
    cx.run_until_parked();
    // The first frame measures the track and fits it; the handles are laid out on the next.
    view.update(cx, |_, cx| cx.notify());
    cx.run_until_parked();
    (view, cx, edits)
}

/// The window bounds of clip `clip`'s box: left, top, width, height.
fn clip_bounds(
    view: &gpui::Entity<TimelineView>,
    cx: &mut gpui::VisualTestContext,
    clip: usize,
) -> [f32; 4] {
    let metrics = view.read_with(cx, |view, _| view.qualification_metrics());
    let found = metrics["edit"]["clips"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["clip"] == clip)
        .unwrap_or_else(|| panic!("clip {clip} in {metrics:#}"));
    let at = |i: usize| found["bounds"][i].as_f64().unwrap() as f32;
    [at(0), at(1), at(2), at(3)]
}

fn press(cx: &mut gpui::VisualTestContext, x: f32, y: f32) {
    // The first move paints the hover the press is tested against.
    for at in [x + 12., x] {
        cx.simulate_mouse_move(gpui::point(px(at), px(y)), None, gpui::Modifiers::none());
        cx.run_until_parked();
    }
    cx.simulate_mouse_down(
        gpui::point(px(x), px(y)),
        MouseButton::Left,
        gpui::Modifiers::none(),
    );
}

#[gpui::test]
fn dragging_a_clip_edge_trims_in_one_gesture(cx: &mut gpui::TestAppContext) {
    let (view, cx, edits) = editable(cx);
    let [left, top, width, height] = clip_bounds(&view, cx, 1);
    let per_second = width / 3.;
    // A press on the end handle that never moves asks for nothing.
    press(cx, left + width - 2., top + height / 2.);
    assert!(view.read_with(cx, |view, _| view.edit_dragging()));
    view.update(cx, |view, cx| view.end_edit_drag(cx));
    assert!(edits.borrow().is_empty(), "{:?}", edits.borrow());

    // Pulling the end back by a second leaves 20 of the clip's 30 frames.
    press(cx, left + width - 2., top + height / 2.);
    view.update(cx, |view, cx| {
        view.edit_drag_move(left + width - 2. - per_second, true, cx);
        view.edit_drag_move(left + width - 2. - per_second, true, cx);
        view.end_edit_drag(cx);
    });
    assert_eq!(
        *edits.borrow(),
        [
            TimelineCommand::BeginGesture,
            TimelineCommand::Trim {
                clip: 1,
                edge: TrimEdge::End,
                frames: 10
            },
            TimelineCommand::EndGesture,
        ]
    );
    assert!(!view.read_with(cx, |view, _| view.edit_dragging()));
}

#[gpui::test]
fn the_end_edge_snaps_to_the_playhead_unless_shift_is_held(cx: &mut gpui::TestAppContext) {
    let (view, cx, edits) = editable(cx);
    view.update(cx, |view, _| view.set_position(70));
    cx.run_until_parked();
    let [left, top, width, height] = clip_bounds(&view, cx, 2);
    let per_second = width / 3.;
    // The end of the last clip, 9 s, dragged to 7.04 s: within 8 px of the playhead.
    press(cx, left + width - 2., top + height / 2.);
    let near = left + width - 2. - 1.96 * per_second;
    view.update(cx, |view, cx| view.edit_drag_move(near, false, cx));
    let snapped =
        view.read_with(cx, |view, _| view.qualification_metrics())["edit"]["snapped"].clone();
    assert_eq!(snapped, 7.0);
    view.update(cx, |view, cx| view.edit_drag_move(near, true, cx));
    let held =
        view.read_with(cx, |view, _| view.qualification_metrics())["edit"]["snapped"].clone();
    assert!(held.is_null(), "{held}");
    view.update(cx, |view, cx| view.end_edit_drag(cx));
    assert_eq!(edits.borrow().last(), Some(&TimelineCommand::EndGesture));
}

#[gpui::test]
fn dragging_a_clip_along_the_track_moves_it(cx: &mut gpui::TestAppContext) {
    let (view, cx, edits) = editable(cx);
    let [left, top, width, height] = clip_bounds(&view, cx, 0);
    let [last_left, _, last_width, _] = clip_bounds(&view, cx, 2);
    // Press in the first clip's middle, away from its handles.
    press(cx, left + width / 2., top + height / 2.);
    view.update(cx, |view, cx| {
        view.edit_drag_move(last_left + last_width - 4., false, cx)
    });
    let metrics = view.read_with(cx, |view, _| view.qualification_metrics());
    assert_eq!(
        (&metrics["edit"]["drag"], &metrics["edit"]["drop"]),
        (&"reorder".into(), &2.into())
    );
    view.update(cx, |view, cx| view.end_edit_drag(cx));
    assert_eq!(*edits.borrow(), [TimelineCommand::Move { clip: 0, to: 2 }]);
}

#[gpui::test]
fn a_rebuilt_preview_drops_a_reorder_but_closes_a_trim(cx: &mut gpui::TestAppContext) {
    let rebuilt = || {
        let mut report = report(10, &[("a", 0, 30), ("b", 30, 60), ("c", 60, 90)]);
        report.envelope.identity.source_revision = "b".repeat(64);
        TimelineModel::new(report).unwrap()
    };
    let (view, cx, edits) = editable(cx);
    let [left, top, width, height] = clip_bounds(&view, cx, 0);
    let [last_left, _, last_width, _] = clip_bounds(&view, cx, 2);
    press(cx, left + width / 2., top + height / 2.);
    view.update(cx, |view, cx| {
        view.edit_drag_move(last_left + last_width - 4., false, cx)
    });
    view.update(cx, |view, cx| view.install(rebuilt(), Arc::from([]), 0, cx));
    assert!(edits.borrow().is_empty(), "{:?}", edits.borrow());
    assert!(!view.read_with(cx, |view, _| view.edit_dragging()));

    let [left, top, width, height] = clip_bounds(&view, cx, 2);
    press(cx, left + width - 2., top + height / 2.);
    view.update(cx, |view, cx| {
        view.edit_drag_move(left + width - 20., false, cx)
    });
    view.update(cx, |view, cx| view.clear(cx));
    assert_eq!(edits.borrow().last(), Some(&TimelineCommand::EndGesture));
}

#[gpui::test]
fn the_playhead_clamps_to_a_shorter_timeline_and_runs_ahead_only_while_playing(
    cx: &mut gpui::TestAppContext,
) {
    let (view, cx, _) = editable(cx);
    view.update(cx, |view, _| view.set_position(40));
    view.read_with(cx, |view, _| {
        assert_eq!(view.playhead_frame(90, 30.), 40.);
    });
    view.update(cx, |view, _| {
        view.set_playing(true);
        view.anchor = Instant::now() - std::time::Duration::from_secs(5);
    });
    // A whole frame at most, however late the next one is.
    view.read_with(cx, |view, _| assert_eq!(view.playhead_frame(90, 30.), 41.));
    view.update(cx, |view, _| view.set_position(89));
    view.update(cx, |view, _| {
        view.anchor = Instant::now() - std::time::Duration::from_secs(5);
    });
    view.read_with(cx, |view, _| assert_eq!(view.playhead_frame(89, 30.), 89.));
    // Another timeline, shorter than where the playhead was.
    let shorter = TimelineModel::new(report(10, &[("a", 0, 30)])).unwrap();
    view.update(cx, |view, cx| view.install(shorter, Arc::from([]), 70, cx));
    view.read_with(cx, |view, _| assert_eq!(view.position, 30));
}

#[gpui::test]
fn split_cuts_the_clip_under_the_playhead(cx: &mut gpui::TestAppContext) {
    let (view, cx, edits) = editable(cx);
    view.update(cx, |view, _| view.set_position(40));
    view.update(cx, |view, cx| view.split_at_playhead(cx));
    assert_eq!(
        *edits.borrow(),
        [TimelineCommand::Split { clip: 1, at: 10 }]
    );
    // On a cut between clips nothing is under the playhead.
    view.update(cx, |view, _| view.set_position(30));
    view.update(cx, |view, cx| view.split_at_playhead(cx));
    assert_eq!(edits.borrow().len(), 1);
}

#[gpui::test]
fn edits_are_refused_with_the_engines_reason(cx: &mut gpui::TestAppContext) {
    use crate::timeline::model::tests::snapshot;
    let (view, cx, edits) = editable(cx);
    view.update(cx, |view, cx| {
        let mut locked = snapshot(&[("a", 30, 0, 0), ("b", 30, 0, 0), ("c", 30, 0, 0)]);
        locked.editing = TimingEditing::Disabled("Build the preview to edit".into());
        view.set_snapshot(Some(locked), cx);
        view.set_position(40);
        view.split_at_playhead(cx);
    });
    cx.run_until_parked();
    let metrics = view.read_with(cx, |view, _| view.qualification_metrics());
    assert_eq!(metrics["edit"]["enabled"], false);
    assert_eq!(metrics["edit"]["reason"], "Build the preview to edit");
    // A press on a clip edge grabs nothing.
    let [left, top, width, height] = clip_bounds(&view, cx, 1);
    press(cx, left + width - 2., top + height / 2.);
    assert!(!view.read_with(cx, |view, _| view.edit_dragging()));
    assert!(edits.borrow().is_empty());
}
