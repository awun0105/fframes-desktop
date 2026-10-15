//! Repeated edit and failed-task cycles leak nothing.

use super::*;

#[test]
fn twenty_four_edit_and_failed_task_cycles_leak_no_process_capability_or_lease() {
    let w = World::new();
    let mut last: Option<AgentTaskId> = None;
    let mut accepted = 0;
    for cycle in 0..24 {
        match cycle % 6 {
            0 | 1 => {
                w.submit(&good(&format!("cycle {cycle}")));
                let task = w.wait_task(last.as_ref());
                assert_eq!(
                    task.phase,
                    TaskPhase::Accepted,
                    "cycle {cycle}: {:?}",
                    task.error
                );
                accepted += 1;
                last = Some(task.id);
            }
            2 => {
                w.submit_plan(&[broken(), broken()]);
                let task = w.wait_task(last.as_ref());
                assert_eq!(task.phase, TaskPhase::Failed, "cycle {cycle}");
                last = Some(task.id);
            }
            3 => {
                w.submit(&json!({"write": {"c.txt": "x"}, "crash": 3}));
                let task = w.wait_task(last.as_ref());
                assert_eq!(task.phase, TaskPhase::Failed, "cycle {cycle}");
                last = Some(task.id);
            }
            4 => {
                let before = w.evidence("prompts.jsonl").len();
                w.submit(&json!({"hang": true}));
                w.wait_prompts(before + 1);
                w.wait_phase(TaskPhase::Editing);
                w.wf().stop().unwrap();
                let task = w.wait_task(last.as_ref());
                assert_eq!(task.phase, TaskPhase::Cancelled, "cycle {cycle}");
                last = Some(task.id);
            }
            _ => {
                w.submit(&json!({"permission": {"title": "Edit"}, "write": {"y.txt": "y"}}));
                w.wait_phase(TaskPhase::WaitingPermission);
                w.wf().stop().unwrap();
                let task = w.wait_task(last.as_ref());
                assert_eq!(task.phase, TaskPhase::Cancelled, "cycle {cycle}");
                last = Some(task.id);
            }
        }
        assert_eq!(w.processes(), 0, "cycle {cycle} left a process");
        let s = w.snap();
        assert_eq!(
            s.resources.broker_grants, 0,
            "cycle {cycle} left a capability"
        );
        assert_eq!(
            w.service.stats().leased_entries,
            0,
            "cycle {cycle} left a build lease"
        );
        assert!(s.resources.resident_rows <= MAX_RESIDENT_ROWS);
    }
    assert_eq!(w.snap().history.len(), accepted);
    w.wf().close();
    w.assert_clean();
    assert!(
        fs::read_dir(w.runtime.path().join("rt"))
            .map(|d| d.count())
            .unwrap_or(0)
            == 0
    );
}
