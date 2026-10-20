//! Permissions and questions.

use super::*;

#[test]
fn permission_replies_are_correlated_and_stale_or_duplicate_ones_are_rejected() {
    let w = World::new();
    let mut script = good("permitted");
    script["permission"] = json!({"title": "Edit title"});
    w.submit(&script);
    let s = w.wait_phase_snapshot(TaskPhase::WaitingPermission);
    let card = s.open_permissions()[0].clone();
    assert_eq!(
        card.options
            .iter()
            .map(|o| o.option_id.as_str())
            .collect::<Vec<_>>(),
        ["allow", "reject"]
    );
    // An option the request never offered.
    assert_eq!(
        w.wf()
            .reply_permission(&card.reference, PermissionAnswer::Select("nope".into()))
            .unwrap_err(),
        WorkflowError::UnknownChoice("nope".into())
    );
    // A request of another task / agent session.
    let mut foreign = card.reference.clone();
    foreign.task = AgentTaskId::new();
    assert_eq!(
        w.wf()
            .reply_permission(&foreign, PermissionAnswer::Select("allow".into()))
            .unwrap_err(),
        WorkflowError::StalePermission
    );
    let mut other_writer = card.reference.clone();
    other_writer.writer += 1;
    assert_eq!(
        w.wf()
            .reply_permission(&other_writer, PermissionAnswer::Select("allow".into()))
            .unwrap_err(),
        WorkflowError::StalePermission
    );
    w.wf()
        .reply_permission(&card.reference, PermissionAnswer::Select("allow".into()))
        .unwrap();
    assert_eq!(
        w.wf()
            .reply_permission(&card.reference, PermissionAnswer::Select("allow".into()))
            .unwrap_err(),
        WorkflowError::DuplicatePermission
    );
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    let s = w.snap();
    assert!(
        s.rows
            .iter()
            .any(|r| matches!(&r.kind, RowKind::Permission(c)
        if c.state == PermissionState::Selected { option_id: "allow".into() }))
    );
    assert_eq!(
        w.wf()
            .reply_permission(&card.reference, PermissionAnswer::Select("allow".into()))
            .unwrap_err(),
        WorkflowError::DuplicatePermission
    );
    w.assert_clean();
}

/// Under "Allow in the working copy" Studio answers an edit inside the draft itself (the
/// agent's `allow_once`, recorded on the row), while a command still waits for the user.
#[test]
fn the_working_copy_policy_answers_draft_edits_and_leaves_commands_to_the_user() {
    let w = World::new();
    w.wf()
        .set_permission_policy(PermissionPolicy::AllowInWorkingCopy)
        .unwrap();
    let mut edit = good("auto");
    edit["permission"] =
        json!({"title": "Edit src/lib.rs", "kind": "edit", "locations": ["src/lib.rs"]});
    w.submit(&edit);
    let first = w.wait_task(None);
    assert_eq!(first.phase, TaskPhase::Accepted, "{:?}", first.error);
    assert_eq!(
        first.permission_policy,
        PermissionPolicy::AllowInWorkingCopy
    );
    assert_eq!(w.source("src/lib.rs"), "// auto\n");
    let answered = w.snap().rows.iter().any(|row| {
        matches!(&row.kind, RowKind::Permission(card)
            if card.answered_by == Some(PermissionPolicy::AllowInWorkingCopy)
                && card.state == PermissionState::Selected { option_id: "allow".into() })
    });
    assert!(answered, "{:#?}", w.snap().rows);

    let mut command = good("asked");
    command["permission"] = json!({"title": "Run cargo check", "kind": "execute"});
    w.submit(&command);
    let s = w.wait_phase_snapshot(TaskPhase::WaitingPermission);
    let card = s.open_permissions()[0].clone();
    assert_eq!(card.answered_by, None);
    w.wf()
        .reply_permission(&card.reference, PermissionAnswer::Select("allow".into()))
        .unwrap();
    let second = w.wait_task(Some(&first.id));
    assert_eq!(second.phase, TaskPhase::Accepted, "{:?}", second.error);
    w.assert_clean();
}

#[test]
fn a_rejected_permission_lets_the_agent_finish_without_changes_and_the_user_answers() {
    let w = World::new();
    w.submit_plan(&[
        json!({"permission": {"title": "Edit title"}, "write": {"src/lib.rs": "// nope\n"}, "text": "I could not edit"}),
        good("after question"),
    ]);
    let s = w.wait_phase_snapshot(TaskPhase::WaitingPermission);
    let card = s.open_permissions()[0].clone();
    w.wf()
        .reply_permission(&card.reference, PermissionAnswer::Select("reject".into()))
        .unwrap();
    // No file changed: the turn is a question the user must answer.
    let task = w.wait_phase(TaskPhase::WaitingClarification);
    assert_ne!(w.source("src/lib.rs"), "// nope\n");
    w.wf()
        .reply_clarification(&task.id, "Go ahead and edit it")
        .unwrap();
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    assert_eq!(w.source("src/lib.rs"), "// after question\n");
}

#[test]
fn a_clarification_is_a_subsequent_prompt_in_the_same_provider_session() {
    let w = World::new();
    w.submit_plan(&[json!({"text": "Which title should I use?"}), good("titled")]);
    let task = w.wait_phase(TaskPhase::WaitingClarification);
    assert_eq!(
        w.wf()
            .reply_clarification(&AgentTaskId::new(), "wrong task")
            .unwrap_err(),
        WorkflowError::NotWaiting
    );
    w.wf().reply_clarification(&task.id, "Use Title A").unwrap();
    assert_eq!(
        w.wf().reply_clarification(&task.id, "again").unwrap_err(),
        WorkflowError::NotWaiting,
        "the answer was claimed"
    );
    let done = w.wait_task(None);
    assert_eq!(done.phase, TaskPhase::Accepted, "{:?}", done.error);
    assert_eq!(done.turns, 2);
    let prompts = w.evidence("prompts.jsonl");
    assert_eq!(prompts.len(), 2);
    assert_eq!(
        prompts[0]["session"], prompts[1]["session"],
        "one provider session"
    );
    assert_eq!(prompts[0]["pid"], prompts[1]["pid"], "one adapter process");
    assert!(
        prompts[1]["text"]
            .as_str()
            .unwrap()
            .starts_with("Use Title A")
    );
    assert_eq!(w.evidence("session-params.json").len().max(1), 1);
    assert!(w.snap().rows.iter().any(|r| matches!(&r.kind, RowKind::User { source: UserSource::Clarification, text } if text == "Use Title A")));
    w.assert_clean();
}
