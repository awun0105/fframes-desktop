//! Writer containment through the production policy on every platform: the supervisor
//! verifies the scripted agent's containment when it launches (a job object on Windows, a
//! process group with an escape scan on Linux and macOS), and no ledger file exists.
#![allow(dead_code, unused_imports)]
include!("../../crates/studio-engine/tests/support/workflow_world.rs");

fn verified() -> World {
    World::with(Options {
        ownership: OwnershipSource::Verified,
        ..Options::default()
    })
}

/// The label of the containment this platform verifies.
fn contained_label() -> &'static str {
    if cfg!(windows) {
        "job-object-contained"
    } else {
        "process-group-contained"
    }
}

fn process_exists(pid: u32) -> bool {
    if cfg!(windows) {
        let out = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
            .output()
            .unwrap();
        String::from_utf8_lossy(&out.stdout).contains(&format!("\"{pid}\""))
    } else {
        std::process::Command::new("kill")
            .args(["-0", &pid.to_string()])
            .status()
            .unwrap()
            .success()
    }
}

/// Waits until the task applied its candidate, holds it for review or failed.
fn wait_settled(w: &World) -> TaskView {
    w.wait(
        "an applied or reviewable candidate, or a failure",
        |snapshot| {
            snapshot.task.as_ref().is_some_and(|task| {
                matches!(
                    task.phase,
                    TaskPhase::Accepted | TaskPhase::AwaitingReview | TaskPhase::Failed
                )
            })
        },
    )
    .task
    .clone()
    .unwrap()
}

#[test]
fn a_launch_verified_writer_and_its_grandchild_are_contained_and_the_candidate_is_captured() {
    let w = verified();
    assert!(
        !w.paths.data.join("qualification").exists(),
        "no qualification ledger is installed"
    );
    let mut script = good("contained at launch");
    script["helper"] = json!(true);
    w.submit(&script);
    let task = wait_settled(&w);
    // Captured, and applied by the default review policy.
    assert_eq!(
        task.phase,
        TaskPhase::Accepted,
        "the candidate was not captured: {:?}",
        task.error
    );
    assert_eq!(
        w.source("src/lib.rs"),
        "// contained at launch
"
    );
    assert_eq!(
        task.writer.as_ref().map(|writer| writer.ownership.as_str()),
        Some(contained_label())
    );
    let launch = w.snap().adapter.last_launch.clone().expect("a launch");
    assert_eq!(launch.ownership, contained_label());
    assert!(launch.contained);
    // The agent's own child stayed inside the containment and was stopped with it before
    // the candidate was captured.
    let helper: u32 = fs::read_to_string(w.agent.join("helper.pid"))
        .unwrap()
        .trim()
        .parse()
        .unwrap();
    assert!(
        !process_exists(helper),
        "the grandchild {helper} survived capture"
    );
    w.assert_clean();
}

#[test]
fn a_launch_whose_containment_check_fails_never_yields_a_captured_candidate() {
    let w = verified();
    let original = w.source("src/lib.rs");
    w.ctl().lock().processes.fail_containment_checks(1);
    w.submit(&good("unproven containment"));
    let task = wait_settled(&w);
    assert_eq!(task.phase, TaskPhase::Failed);
    assert!(task.review.is_none(), "{:?}", task.review);
    if cfg!(target_os = "linux") {
        // The process runs, but its group is not proven: capture is refused.
        assert_eq!(task.error.as_ref().unwrap().code, "quiescence_blocked");
        let launch = w.snap().adapter.last_launch.clone().expect("a launch");
        assert_eq!(launch.ownership, WriterOwnership::Unknown.label());
        assert!(!launch.contained);
    } else {
        // Windows and macOS refuse the launch itself: no writer ever ran.
        assert!(task.writer.is_none(), "{:?}", task.writer);
        assert!(w.evidence("prompts.jsonl").is_empty());
    }
    assert_eq!(w.source("src/lib.rs"), original);
    w.assert_clean();
}

#[test]
fn stop_cancels_the_agent_session_before_its_processes_are_terminated() {
    let w = verified();
    let mut script = good("stopped");
    script["wait_file"] = json!("go");
    w.submit(&script);
    w.wait_phase(TaskPhase::Editing);
    w.wait_prompts(1);
    w.wf().stop().unwrap();
    w.wait("the stopped task", |s| {
        s.task
            .as_ref()
            .is_some_and(|t| t.phase == TaskPhase::Cancelled)
    });
    // The agent answered `session/cancel` itself, so it was still running when Stop
    // reached it: termination came after the cancel, never instead of it.
    let cancel = w.evidence("cancels.jsonl");
    assert_eq!(cancel.len(), 1, "{cancel:?}");
    assert_eq!(cancel[0]["was_active"], json!(true));
    w.assert_clean();
}
