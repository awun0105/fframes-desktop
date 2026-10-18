//! Audio checks of a candidate's prepared mix.

use super::*;

/// Prepared audio whose PCM is 0.5 inside `active` sample ranges and silent elsewhere,
/// scanned by the real streaming scan against the timeline's windows.
fn active_audio(
    t: &PreviewTimelineResponse,
    active: Vec<(u64, u64)>,
    mutate: impl FnOnce(&mut AudioProbe),
) -> AudioProbe {
    let mut probe = silent_audio(t, &t.envelope.identity.source_revision);
    let total = probe.descriptor.sample_count;
    let mut bytes = Vec::with_capacity(total as usize * 8);
    for frame in 0..total {
        let on = active.iter().any(|&(s, e)| frame >= s && frame < e);
        let v = if on { 0.5f32 } else { 0. };
        bytes.extend_from_slice(&v.to_le_bytes());
        bytes.extend_from_slice(&v.to_le_bytes());
    }
    probe.stats = scan_pcm(
        bytes.as_slice(),
        48_000,
        &placement_windows(t, 48_000),
        &|| false,
    )
    .unwrap();
    mutate(&mut probe);
    probe
}

fn audio_failure(
    mutate: impl FnOnce(&mut AudioProbe),
    tracks: &[(f64, f64)],
    active: Vec<(u64, u64)>,
) -> Vec<String> {
    let t = timeline("r", 90, &[], tracks);
    let probe = active_audio(&t, active, mutate);
    let (_report, errors) = check_audio(&t, &probe, "r");
    errors.into_iter().map(|(k, _)| k).collect()
}

#[test]
fn audio_checks_cover_revision_geometry_duration_finiteness_and_placement() {
    let ok_tracks = [(0.5, 2.0)];
    let inside = vec![(24_000, 96_000)];
    assert!(audio_failure(|_| {}, &ok_tracks, inside.clone()).is_empty());
    assert_eq!(
        audio_failure(
            |a| a.descriptor.envelope.identity.source_revision = "other".into(),
            &ok_tracks,
            inside.clone()
        ),
        ["audio_revision"]
    );
    assert!(
        audio_failure(
            |a| a.descriptor.sample_rate = 7000,
            &ok_tracks,
            inside.clone()
        )
        .contains(&"audio_geometry".to_owned())
    );
    assert!(
        audio_failure(|a| a.descriptor.channels = 1, &ok_tracks, inside.clone())
            .contains(&"audio_geometry".to_owned())
    );
    assert!(
        audio_failure(
            |a| {
                a.descriptor.sample_count -= 480;
                a.descriptor.byte_count -= 480 * 8;
                a.stats.sample_frames -= 480;
            },
            &ok_tracks,
            inside.clone()
        )
        .contains(&"audio_duration".to_owned())
    );
    assert!(
        audio_failure(|a| a.stats.non_finite = 3, &ok_tracks, inside.clone())
            .contains(&"audio_finite".to_owned())
    );
    assert!(
        audio_failure(|a| a.stats.sample_frames -= 8, &ok_tracks, inside.clone())
            .contains(&"audio_pcm_length".to_owned())
    );
    assert!(
        audio_failure(|a| a.descriptor.silent = true, &ok_tracks, inside.clone())
            .contains(&"audio_silent_flag".to_owned())
    );
    // Audible content far outside the declared window is a placement failure...
    assert!(
        audio_failure(|_| {}, &ok_tracks, vec![(120_000, 140_000)])
            .contains(&"audio_track_placement".to_owned())
    );
    // ...and a track starting beyond the video is out of bounds.
    assert!(
        audio_failure(|_| {}, &[(30.0, 31.0)], vec![]).contains(&"audio_track_bounds".to_owned())
    );
    // A fragmented activity map never hides out-of-window audio: the streaming scan
    // already recorded it.
    assert!(
        audio_failure(
            |a| a.stats.activity_truncated = true,
            &ok_tracks,
            vec![(120_000, 140_000)]
        )
        .contains(&"audio_track_placement".to_owned())
    );
}

#[test]
fn clipping_and_silence_are_visible_diagnostics_not_failures() {
    let f = fixture();
    let mut probe = Fake::new("x", 90);
    let t = timeline("x", 90, &[], &[(0.0, 3.0)]);
    probe.audio = Ok(AudioProbe {
        stats: PcmStats {
            peak: 1.2,
            clipped: 44,
            active: vec![(0, 144_000)],
            ..silent_audio(&t, "x").stats
        },
        ..silent_audio(&t, "x")
    });
    probe.timeline = t;
    let (report, _c) = run(&f, &mut probe, |_| {});
    assert!(report.passed(), "{:?}", report.failure());
    assert!(
        report
            .diagnostics()
            .iter()
            .any(|d| d.key == "audio_clipping")
    );
    assert!(report.warnings().iter().any(|w| w.contains("full scale")));

    let f = fixture();
    let mut probe = Fake::new("x", 90);
    let t = timeline("x", 90, &[], &[(0.0, 3.0)]);
    probe.audio = Ok(silent_audio(&t, "x"));
    probe.timeline = t;
    let (report, _c) = run(&f, &mut probe, |_| {});
    assert!(report.passed());
    assert!(
        report
            .diagnostics()
            .iter()
            .any(|d| d.key == "audio_silent_mix")
    );
    assert!(!report.audio().unwrap().tracks[0].audible);
}

#[test]
fn audio_only_edit_with_unchanged_video_passes_and_checks_the_mix() {
    let f = fixture();
    let t = timeline("x", 90, &[(0, 90)], &[(0.0, 3.0)]);
    let mut probe = Fake::new("x", 90).with_timeline(t.clone());
    probe.audio = Ok(active_audio(&t, vec![(0, 144_000)], |_| {}));
    probe.timeline = t;
    let (report, _c) = run(&f, &mut probe, |draft| {
        fs::write(draft.join("media/cue.wav"), b"RIFF new audio").unwrap();
    });
    assert!(report.passed(), "{:?}", report.failure());
    let audio = report.audio().unwrap();
    assert!(audio.checks_passed.contains(&"track_placement".to_owned()));
    assert!(audio.tracks[0].audible);
    assert!(
        report
            .frames()
            .windows(2)
            .all(|w| w[0].sha256 == w[1].sha256)
    );
}

#[test]
fn scan_pcm_reports_activity_clipping_and_non_finite_samples() {
    let rate = 48_000u32;
    let mut bytes = Vec::new();
    let mut push = |l: f32, r: f32, n: usize| {
        for _ in 0..n {
            bytes.extend_from_slice(&l.to_le_bytes());
            bytes.extend_from_slice(&r.to_le_bytes());
        }
    };
    push(0., 0., 4800); // 100 ms silence
    push(0.5, -0.25, 4800); // 100 ms tone
    push(1.0, 0., 3); // clipped
    push(f32::NAN, 0., 2);
    push(0., 0., 4795);
    let stats = scan_pcm(bytes.as_slice(), rate, &[], &|| false).unwrap();
    assert_eq!(stats.sample_frames, 4800 + 4800 + 3 + 2 + 4795);
    assert_eq!(stats.non_finite, 2);
    assert_eq!(stats.clipped, 3);
    assert!((stats.peak - 1.0).abs() < f32::EPSILON);
    assert_eq!(stats.active, vec![(4800, 9600 + 480)]);
    assert!(!stats.activity_truncated);
    assert!(scan_pcm(bytes.as_slice(), rate, &[], &|| true).is_err());
}

/// PCM that alternates 10 ms of signal and silence (more than `MAX_ACTIVITY_RANGES`
/// ranges), scanned by the real streaming scan against the timeline's windows.
fn fragmented_audio(t: &PreviewTimelineResponse) -> AudioProbe {
    let mut probe = silent_audio(t, "x");
    let total = probe.descriptor.sample_count;
    let mut bytes = Vec::with_capacity(total as usize * 8);
    for frame in 0..total {
        let v = if (frame / 480) % 2 == 0 { 0.5f32 } else { 0. };
        bytes.extend_from_slice(&v.to_le_bytes());
        bytes.extend_from_slice(&v.to_le_bytes());
    }
    probe.stats = scan_pcm(
        bytes.as_slice(),
        48_000,
        &placement_windows(t, 48_000),
        &|| false,
    )
    .unwrap();
    assert!(
        probe.stats.activity_truncated,
        "the activity map overflowed"
    );
    probe
}

#[test]
fn fragmented_pcm_cannot_hide_audio_outside_the_track_windows() {
    // 83 s of video, audible content in every other 10 ms bucket; the single track only
    // covers the first 40 s. The bounded activity map overflows long before the
    // out-of-window content, yet the streaming scan has recorded it: not Passed, and a
    // source failure the agent may repair.
    let f = fixture();
    let t = timeline("x", 2500, &[(0, 2500)], &[(0.0, 40.0)]);
    let mut probe = Fake::new("x", 2500).with_timeline(t.clone());
    probe.audio = Ok(fragmented_audio(&t));
    probe.timeline = t;
    let (report, controller) = run(&f, &mut probe, |_| {});
    assert!(!report.passed());
    let failure = report.failure().unwrap();
    assert_eq!(failure.stage, ValidationStage::Audio);
    assert_eq!(failure.kind, FailureKind::Source);
    assert!(report.audio().unwrap().placement_verified);
    assert!(!report.audio().unwrap().audibility_complete);
    assert!(
        report
            .errors()
            .iter()
            .any(|d| d.key == "audio_track_placement")
    );
    assert!(matches!(
        next_step(&report, controller.agent_task().unwrap().repair()),
        NextStep::Repair { .. }
    ));

    // The same fragmented PCM wholly inside a track window is verified and passes;
    // incomplete audibility is a visible limitation, not an acceptance input.
    let f = fixture();
    let t = timeline("x", 2500, &[(0, 2500)], &[(0.0, 90.0)]);
    let mut probe = Fake::new("x", 2500).with_timeline(t.clone());
    probe.audio = Ok(fragmented_audio(&t));
    probe.timeline = t;
    let (report, _c) = run(&f, &mut probe, |_| {});
    assert!(report.passed(), "{:?}", report.failure());
    let audio = report.audio().unwrap();
    assert!(audio.placement_verified);
    assert!(!audio.audibility_complete);
    assert!(audio.checks_passed.contains(&"track_placement".to_owned()));
}

#[test]
fn unverifiable_track_placement_is_never_passed() {
    // A scan that was not given the compiled windows cannot vouch for placement, even
    // with audible content that happens to lie inside them.
    let f = fixture();
    let t = timeline("x", 90, &[(0, 90)], &[(0.5, 2.0)]);
    let mut probe = Fake::new("x", 90).with_timeline(t.clone());
    probe.audio = Ok(active_audio(&t, vec![(24_000, 96_000)], |a| {
        a.stats.checked_windows.clear();
    }));
    probe.timeline = t;
    let (report, controller) = run(&f, &mut probe, |_| {});
    assert!(!report.passed());
    let failure = report.failure().unwrap();
    assert_eq!(failure.stage, ValidationStage::Audio);
    assert_eq!(failure.kind, FailureKind::Environment);
    let audio = report.audio().unwrap();
    assert!(!audio.placement_verified);
    assert!(!audio.checks_passed.contains(&"track_placement".to_owned()));
    assert!(
        report
            .errors()
            .iter()
            .any(|d| d.key == "audio_placement_unverified")
    );
    assert!(matches!(
        next_step(&report, controller.agent_task().unwrap().repair()),
        NextStep::Retain { .. }
    ));
}
