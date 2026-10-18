//! Opening a project, readiness probes and commands refused before they reach the actor.

use super::*;

#[test]
fn opening_a_project_launches_no_agent_and_readiness_is_an_explicit_probe() {
    let w = World::new();
    let s = w.snap();
    assert_eq!(s.adapter.readiness, AdapterReadiness::Unchecked);
    assert_eq!(s.adapter.provider.as_deref(), Some("scripted"));
    assert!(s.task.is_none());
    assert!(
        !w.agent.join("calls.jsonl").exists() && !w.agent.join("starts").exists(),
        "project open must never start the adapter"
    );
    assert_eq!(w.processes(), 0);
    w.wf().check_adapter().unwrap();
    let s = w.wait("the probe result", |s| {
        matches!(s.adapter.readiness, AdapterReadiness::Checked { .. })
    });
    let AdapterReadiness::Checked { report, .. } = &s.adapter.readiness else {
        unreachable!()
    };
    assert!(
        matches!(&report.status, studio_harness::AdapterStatus::Ready { agent, .. } if agent == "scripted-agent"),
        "{:?}",
        report.status
    );
    assert_eq!(w.processes(), 0, "the probe reaps its adapter");
    // The probe is not a task.
    assert!(w.snap().task.is_none());
}

#[test]
fn a_missing_adapter_is_reported_by_the_probe_and_by_submit_without_a_task() {
    let w = World::with(Options {
        executable: Some("definitely-not-an-adapter-xyz".into()),
        ..Options::default()
    });
    w.wf().check_adapter().unwrap();
    let s = w.wait("the probe result", |s| {
        matches!(s.adapter.readiness, AdapterReadiness::Checked { .. })
    });
    let AdapterReadiness::Checked { report, .. } = &s.adapter.readiness else {
        unreachable!()
    };
    assert!(matches!(
        report.status,
        studio_harness::AdapterStatus::MissingExecutable { .. }
    ));
    w.wf().submit("do something").unwrap();
    w.wait("the missing-adapter error", |s| {
        has_error(s, "adapter_missing")
    });
    assert!(
        w.snap().task.is_none(),
        "no task is created for an adapter that cannot start"
    );
    assert_eq!(w.engine_state(), None);
}

#[test]
fn commands_are_refused_before_they_reach_the_actor() {
    let w = World::with(Options {
        adapter: false,
        ..Options::default()
    });
    assert_eq!(w.snap().adapter.readiness, AdapterReadiness::NotConfigured);
    assert_eq!(
        w.wf().submit("hello").unwrap_err(),
        WorkflowError::NotConfigured
    );
    assert_eq!(
        w.wf().check_adapter().unwrap_err(),
        WorkflowError::NotConfigured
    );
    let w = World::new();
    assert!(matches!(
        w.wf().submit("  ").unwrap_err(),
        WorkflowError::InvalidBrief(_)
    ));
    assert_eq!(
        w.wf().set_mode("code").unwrap_err(),
        WorkflowError::UnknownOption("code".into()),
        "options are only settable once the adapter advertised them"
    );
    w.wf().close();
    assert_eq!(w.wf().submit("hello").unwrap_err(), WorkflowError::Closed);
    assert!(w.snap().closed);
}
