//! Provider setup state on the agent panel: adapter launch mapping, turn-on and install tracking.

use tempfile::TempDir;

#[test]
fn a_detected_adapter_file_maps_to_its_launch_and_names_what_is_missing() {
    use fframes_studio::conversation_panel::provider_setup::{DetectedLaunch, detected_launch};
    let temp = TempDir::new().unwrap();
    let script = temp.path().join("adapter.js");
    std::fs::write(&script, "").unwrap();
    let node = temp.path().join("node");
    assert_eq!(
        detected_launch(&script, Some(&node)),
        Ok(DetectedLaunch::Node {
            node: node.clone(),
            entry: script.clone()
        })
    );
    let error = detected_launch(&script, None).unwrap_err();
    assert!(error.contains("node"), "{error}");
    let native = temp.path().join("adapter.exe");
    assert_eq!(
        detected_launch(&native, None),
        Ok(DetectedLaunch::Direct(native.clone()))
    );
    // A launcher that cannot be read is reported, never started as a binary.
    assert!(detected_launch(&temp.path().join("missing.cmd"), Some(&node)).is_err());
}

#[test]
fn turn_on_shows_readiness_only_on_the_row_it_turned_on() {
    use fframes_studio::conversation_panel::provider_setup::{RowStatus as R, TurnOn};
    use studio_engine::providers::provider_profiles::DetectionState as D;
    let mut turn_on = TurnOn::default();
    assert_eq!(turn_on.row_status("a", D::Ready), R::Detected(D::Ready));

    turn_on = TurnOn::Saving {
        ticket: 1,
        profile: "a".into(),
    };
    assert_eq!(turn_on.row_status("a", D::Ready), R::TurningOn);
    assert_eq!(turn_on.row_status("b", D::Ready), R::Detected(D::Ready));
    assert!(!turn_on.finish(2, true), "another save is not this turn-on");
    assert!(turn_on.finish(1, true));
    assert_eq!(turn_on.row_status("a", D::Ready), R::Readiness);
    assert_eq!(
        turn_on.row_status("b", D::NotFound),
        R::Detected(D::NotFound)
    );

    // Turning on B moves the readiness to B's row.
    turn_on = TurnOn::Saving {
        ticket: 2,
        profile: "b".into(),
    };
    assert!(turn_on.finish(2, true));
    assert_eq!(turn_on.row_status("a", D::Ready), R::Detected(D::Ready));
    assert_eq!(turn_on.row_status("b", D::Ready), R::Readiness);

    // A provider switch or a Save resets it.
    turn_on.reset();
    assert_eq!(turn_on, TurnOn::Idle);

    // A failed save shows no readiness and runs no test.
    turn_on = TurnOn::Saving {
        ticket: 3,
        profile: "a".into(),
    };
    assert!(!turn_on.finish(3, false));
    assert_eq!(turn_on, TurnOn::Idle);
    assert_eq!(turn_on.row_status("a", D::Ready), R::Detected(D::Ready));
}

#[test]
fn an_install_cancelled_after_npm_finished_is_not_turned_on() {
    use fframes_studio::conversation_panel::provider_setup::{
        InstallFinished, InstallOutcome as O, InstallTracker,
    };
    use studio_harness::adapter_install::{InstallError, InstalledAdapter};
    let installed = || -> Result<InstalledAdapter, InstallError> {
        Ok(InstalledAdapter {
            node: "/n/node".into(),
            entry: "/a/index.js".into(),
        })
    };

    let mut tracker = InstallTracker::default();
    let (ticket, flag) = tracker.begin("claude").unwrap();
    assert!(tracker.begin("codex").is_none(), "one install at a time");
    assert_eq!(tracker.installing(), Some("claude"));
    // npm has finished and its result is on its way when Cancel is pressed.
    tracker.cancel();
    assert!(flag.load(std::sync::atomic::Ordering::SeqCst));
    let finished = tracker.finish(ticket).unwrap();
    assert_eq!(
        finished,
        InstallFinished {
            profile: "claude".into(),
            cancelled: true
        }
    );
    assert_eq!(
        O::of(&installed(), finished.cancelled, true),
        O::Cancelled { installed: true }
    );
    assert_eq!(
        O::of(&Err(InstallError::Cancelled), true, true),
        O::Cancelled { installed: false }
    );
    assert!(!tracker.is_running());

    // Not cancelled: turned on when settings may change, otherwise left for the user.
    assert_eq!(O::of(&installed(), false, true), O::TurnOn);
    assert_eq!(O::of(&installed(), false, false), O::Installed);
    assert!(matches!(
        O::of(&Err(InstallError::TimedOut(1)), false, true),
        O::Failed(_)
    ));
}

#[test]
fn leaving_the_project_stops_the_install_and_drops_its_result() {
    use fframes_studio::conversation_panel::provider_setup::{InstallTracker, ScanTracker};
    let mut tracker = InstallTracker::default();
    let (ticket, flag) = tracker.begin("claude").unwrap();
    tracker.abandon();
    assert!(
        flag.load(std::sync::atomic::Ordering::SeqCst),
        "npm is stopped"
    );
    assert_eq!(tracker.finish(ticket), None, "the late result is dropped");
    // A new install after re-attaching is not confused with the old one.
    let (next, _) = tracker.begin("claude").unwrap();
    assert_eq!(tracker.finish(ticket), None);
    assert!(tracker.finish(next).is_some());

    let mut scans = ScanTracker::default();
    let old = scans.begin().unwrap();
    assert!(scans.begin().is_none(), "one scan at a time");
    scans.abandon();
    let new = scans.begin().unwrap();
    assert!(!scans.finish(old), "an old project's scan is dropped");
    assert!(scans.finish(new));
}
