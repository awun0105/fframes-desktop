//! Validation failure and the one repair turn.

use super::*;

#[test]
fn a_compiler_failure_gets_structured_repair_context_and_exactly_one_repair_turn() {
    let w = World::new();
    w.submit_plan(&[
        broken(),
        json!({"text": "fixed", "write": {"src/lib.rs": "// fixed\n"}}),
    ]);
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    assert_eq!(task.repair.used, 1);
    assert_eq!(task.repair.max, 1);
    assert!(task.repair.context_summary.is_some());
    assert_eq!(w.source("src/lib.rs"), "// fixed\n");
    let prompts = w.evidence("prompts.jsonl");
    assert_eq!(
        prompts.len(),
        2,
        "one first turn and exactly one repair turn"
    );
    let repair_text = prompts[1]["text"].as_str().unwrap();
    assert!(repair_text.contains("failed validation"), "{repair_text}");
    assert!(
        repair_text.contains("BROKEN"),
        "the compiler output is in the context"
    );
    // A fresh adapter session/process served the repair (the old writer was reaped).
    assert_ne!(prompts[0]["pid"], prompts[1]["pid"]);
    let s = w.snap();
    assert!(s.rows.iter().any(|r| matches!(
        &r.kind,
        RowKind::User {
            source: UserSource::Repair,
            ..
        }
    )));
    let cards: Vec<&ValidationCard> = s
        .rows
        .iter()
        .filter_map(|r| match &r.kind {
            RowKind::Validation(c) => Some(&**c),
            _ => None,
        })
        .collect();
    assert_eq!(cards.len(), 2);
    assert!(!cards[0].passed && cards[0].failure.is_some());
    assert!(cards[1].passed);
    assert_eq!(cards[1].repair_count, 1);
    w.assert_clean();
}

#[test]
fn a_second_failure_is_terminal_with_the_draft_and_candidate_retained() {
    let w = World::new();
    let original = w.source("src/lib.rs");
    w.submit_plan(&[broken(), broken()]);
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Failed);
    assert_eq!(task.repair.used, 1);
    assert_eq!(task.error.as_ref().unwrap().code, "validation_failed");
    assert!(task.error.as_ref().unwrap().retained);
    assert_eq!(
        w.evidence("prompts.jsonl").len(),
        2,
        "no third turn is ever started"
    );
    assert_eq!(w.source("src/lib.rs"), original);
    assert!(matches!(
        task.draft_state,
        Some(DraftState::Retained { .. })
    ));
    assert_eq!(w.snap().history.len(), 0);
    w.assert_clean();
}
