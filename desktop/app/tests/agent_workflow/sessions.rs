//! Several stored sessions per project: New chat, switching and resuming.

use super::*;

use studio_engine::agent::WorkflowError;
use studio_engine::agent::session_store::{SessionManifestState, SessionStore};

fn users(s: &WorkflowSnapshot) -> Vec<String> {
    s.rows
        .iter()
        .filter_map(|r| match &r.kind {
            RowKind::User { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

fn has_user(s: &WorkflowSnapshot, marker: &str) -> bool {
    users(s).iter().any(|t| t.contains(marker))
}

fn has_notice(s: &WorkflowSnapshot, needle: &str) -> bool {
    s.rows
        .iter()
        .any(|r| matches!(&r.kind, RowKind::Notice { text, .. } if text.contains(needle)))
}

/// Marks the current session's stored agent session as resumable under `wire`.
fn store_wire_session(w: &World, wire: &str) {
    let project = w.snap().project.clone();
    let mut manifest = SessionStore::load(&w.paths, &project)
        .unwrap()
        .expect("the task saved a manifest");
    manifest.wire_session_id = Some(wire.to_owned());
    manifest.state = SessionManifestState::Ready;
    SessionStore::save(&w.paths, &project, &manifest).unwrap();
}

fn read_load_params(w: &World) -> Option<Value> {
    fs::read_to_string(w.agent.join("session-load.json"))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
}

#[test]
fn files_of_the_single_session_layout_are_ignored_and_left_as_they_are() {
    let mut w = World::new();
    w.submit(&good("earlier-question"));
    w.wait_task(None);
    let project = w.snap().project.clone();

    // Leave the project holding only what the single-session builds wrote: one log directly in
    // the project's directory and no sessions.
    let log = SessionStore::current_log_path(&w.paths, &project).unwrap();
    w.workflow = None;
    w.controller = None;
    let old_log = w.paths.project(&project).join("conversation.jsonl");
    fs::copy(&log, &old_log).unwrap();
    let before = fs::read(&old_log).unwrap();
    fs::remove_dir_all(w.paths.agent_sessions(&project)).unwrap();

    w.restart();
    let s = w.snap();
    assert!(!has_user(&s, "earlier-question"), "{:?}", s.rows);
    assert_eq!(fs::read(&old_log).unwrap(), before);
    let sessions = w.wf().sessions().unwrap();
    assert_eq!(sessions.len(), 1);
    assert!(sessions[0].is_current && sessions[0].rows == 0);
}

#[test]
fn sessions_list_newest_first_and_opening_an_older_one_shows_its_original_rows() {
    let w = World::new();
    w.submit(&good("first-chat"));
    w.wait_task(None);
    let first_last_id = w.snap().rows.last().unwrap().id;

    w.wf().new_session().unwrap();
    w.wait("the new session", |s| s.chat == 1);
    w.submit(&good("second-chat"));
    w.wait_task(None);
    let second = w.snap();
    assert!(has_user(&second, "second-chat") && !has_user(&second, "first-chat"));
    assert!(
        second.rows.first().unwrap().id > first_last_id,
        "row ids continue across sessions"
    );

    let sessions = w.wf().sessions().unwrap();
    assert_eq!(sessions.len(), 2);
    assert!(sessions[0].is_current && !sessions[1].is_current);
    assert!(
        sessions[0].prompt_text.contains("second-chat"),
        "{:?}",
        sessions[0]
    );
    assert!(
        sessions[1].prompt_text.contains("first-chat"),
        "{:?}",
        sessions[1]
    );

    w.wf().open_session(&sessions[1].id).unwrap();
    let opened = w.wait("the older session", |s| s.chat == 2);
    assert!(has_user(&opened, "first-chat") && !has_user(&opened, "second-chat"));
    let current = w.wf().sessions().unwrap();
    assert!(
        current
            .iter()
            .any(|s| s.is_current && s.id == sessions[1].id)
    );

    // Opening the session that is already open does nothing; an unknown id is refused.
    w.wf().open_session(&sessions[1].id).unwrap();
    assert!(matches!(
        w.wf().open_session("no-such-session"),
        Err(WorkflowError::History(_))
    ));
    w.assert_clean();
}

#[test]
fn a_new_chat_continues_the_row_ids_of_the_session_it_leaves() {
    let w = World::new();
    w.submit(&good("numbered"));
    w.wait_task(None);
    let last_id = w.snap().rows.last().unwrap().id;
    w.wf().new_session().unwrap();
    w.wait("the new session", |s| s.chat == 1);
    w.submit(&good("continued"));
    w.wait_task(None);
    assert!(w.snap().rows.first().unwrap().id > last_id);
    w.assert_clean();
}

#[test]
fn switching_sessions_is_refused_while_a_task_runs() {
    let w = World::new();
    w.submit(&good("finished"));
    w.wait_task(None);
    w.wf().new_session().unwrap();
    w.wait("the new session", |s| s.chat == 1);
    let other = w
        .wf()
        .sessions()
        .unwrap()
        .into_iter()
        .find(|s| !s.is_current)
        .unwrap()
        .id;

    w.submit(&json!({"text": "starting", "hang": true}));
    w.wait_phase(TaskPhase::Editing);
    assert!(matches!(
        w.wf().open_session(&other),
        Err(WorkflowError::Busy(_))
    ));
    assert!(matches!(w.wf().new_session(), Err(WorkflowError::Busy(_))));
    assert_eq!(w.snap().chat, 1);

    w.wf().stop().unwrap();
    w.wait_task(None);
    w.wf().open_session(&other).unwrap();
    w.wait("the stored session", |s| s.chat == 2);
    w.assert_clean();
}

#[test]
fn a_stored_agent_session_is_resumed_when_the_agent_can_load_sessions() {
    let w = World::new();
    fs::write(w.agent.join("load-session"), "1").unwrap();
    w.submit(&good("resumable"));
    w.wait_task(None);
    store_wire_session(&w, "wire-session-of-the-first-chat");
    let first = w.wf().sessions().unwrap()[0].id.clone();

    w.wf().new_session().unwrap();
    w.wait("the new session", |s| s.chat == 1);
    w.wf().open_session(&first).unwrap();
    let s = w.wait("the stored session", |s| s.chat == 2);
    let restore = s
        .session_restore
        .as_ref()
        .expect("a restoration is offered");
    assert!(restore.is_resumable, "{restore:?}");
    assert!(has_user(&s, "resumable"));
    assert!(!has_notice(&s, "cannot be resumed"), "{:?}", s.rows);
    assert!(
        read_load_params(&w).is_none(),
        "resuming waits for the next prompt"
    );

    w.submit(&good("follow-up"));
    w.poll("the stored agent session is loaded", || {
        read_load_params(&w).is_some()
    });
    assert_eq!(
        read_load_params(&w)
            .and_then(|p| p
                .get("sessionId")
                .and_then(|v| v.as_str().map(str::to_owned)))
            .as_deref(),
        Some("wire-session-of-the-first-chat")
    );
    w.wait_task(None);
    assert!(has_user(&w.snap(), "follow-up"));
}

#[test]
fn without_resume_support_the_stored_rows_are_read_only_and_a_fresh_agent_session_continues_them() {
    let w = World::new();
    w.submit(&good("unresumable"));
    w.wait_task(None);
    store_wire_session(&w, "wire-session-that-cannot-load");
    let first = w.wf().sessions().unwrap()[0].id.clone();

    w.wf().new_session().unwrap();
    w.wait("the new session", |s| s.chat == 1);
    w.wf().open_session(&first).unwrap();
    let s = w.wait("the stored session", |s| s.chat == 2);
    let restore = s.session_restore.as_ref().expect("the refusal is shown");
    assert!(!restore.is_resumable, "{restore:?}");
    assert!(has_user(&s, "unresumable"));
    let notice = restore.notice.as_deref().unwrap_or_default();
    assert!(notice.contains("read-only"), "{restore:?}");
    assert!(
        notice.contains("does not support resuming"),
        "the stored capabilities decide: {restore:?}"
    );
    assert!(
        !has_notice(&s, "read-only"),
        "opening a session writes no rows: {:?}",
        s.rows
    );

    w.submit(&good("fresh-start"));
    w.wait_task(None);
    assert!(
        read_load_params(&w).is_none(),
        "no stored session is loaded"
    );
    let s = w.snap();
    assert!(has_user(&s, "unresumable") && has_user(&s, "fresh-start"));
    let sessions = w.wf().sessions().unwrap();
    assert_eq!(sessions.len(), 2, "the prompt joined the stored session");
    assert!(sessions.iter().any(|x| x.is_current && x.id == first));
    w.assert_clean();
}

/// The files of a stored session with their sizes and modification times.
fn stamp(w: &World, session: &str) -> Vec<(String, u64, Option<std::time::SystemTime>)> {
    let project = w.snap().project.clone();
    let mut files: Vec<_> = fs::read_dir(w.paths.agent_session_dir(&project, session))
        .unwrap()
        .map(|entry| {
            let entry = entry.unwrap();
            let meta = entry.metadata().unwrap();
            (
                entry.file_name().to_string_lossy().into_owned(),
                meta.len(),
                meta.modified().ok(),
            )
        })
        .collect();
    files.sort();
    files
}

#[test]
fn opening_a_session_leaves_its_stored_files_and_recency_alone() {
    let w = World::new();
    w.submit(&good("untouched"));
    w.wait_task(None);
    let first = w.wf().sessions().unwrap()[0].id.clone();
    w.wf().new_session().unwrap();
    w.wait("the new session", |s| s.chat == 1);
    let before = stamp(&w, &first);
    let order: Vec<_> = w
        .wf()
        .sessions()
        .unwrap()
        .into_iter()
        .map(|s| s.id)
        .collect();

    w.wf().open_session(&first).unwrap();
    let opened = w.wait("the stored session", |s| s.chat == 2);
    assert!(has_user(&opened, "untouched"));
    assert_eq!(stamp(&w, &first), before);
    // Leaving it again changes nothing either, and the list keeps its order.
    let other = order.iter().find(|id| **id != first).unwrap().clone();
    w.wf().open_session(&other).unwrap();
    w.wait("the other session", |s| s.chat == 3);
    assert_eq!(stamp(&w, &first), before);
    let after: Vec<_> = w
        .wf()
        .sessions()
        .unwrap()
        .into_iter()
        .map(|s| s.id)
        .collect();
    assert_eq!(after, order);
    w.assert_clean();
}

#[test]
fn a_history_page_asked_for_an_earlier_conversation_is_refused() {
    let w = World::new();
    w.submit(&good("old-chat"));
    w.wait_task(None);

    // Keep the actor busy on the controller so the new chat and the page request queue up
    // behind it: the page is stamped with the conversation the last snapshot showed.
    let controller = w.ctl();
    let guard = controller.lock();
    w.wf().refresh().unwrap();
    w.wf().new_session().unwrap();
    let (tx, rx) = std::sync::mpsc::channel();
    w.wf()
        .history_page_with(RowId(u64::MAX), 10, move |page| {
            let _ = tx.send(page);
        })
        .unwrap();
    drop(guard);
    let page = rx.recv_timeout(Duration::from_secs(30)).unwrap();
    assert!(matches!(page, Err(WorkflowError::History(_))), "{page:?}");
    w.wait("the new session", |s| s.chat == 1);
}

/// The offer made when the project opens applies the stored capabilities, as opening the
/// session does; with resume support the same session is still offered.
#[test]
fn the_session_offered_at_startup_is_resumable_only_when_its_agent_can_resume() {
    for (load_session, resumable) in [(false, false), (true, true)] {
        let mut w = World::new();
        if load_session {
            fs::write(w.agent.join("load-session"), "1").unwrap();
        }
        w.submit(&good("before-the-restart"));
        w.wait_task(None);
        store_wire_session(&w, "wire-session-at-startup");
        w.restart();
        let s = w.wait("the offer", |s| s.session_restore.is_some());
        let restore = s.session_restore.as_ref().unwrap();
        assert_eq!(restore.is_resumable, resumable, "{restore:?}");
        if !resumable {
            assert!(
                restore
                    .notice
                    .as_deref()
                    .is_some_and(|n| n.contains("does not support resuming")),
                "{restore:?}"
            );
        }
    }
}

/// The startup offer follows the configured agent: refused while another one is configured,
/// offered again once the agent that ran it is back.
#[test]
fn the_session_offered_at_startup_follows_an_adapter_change() {
    let mut w = World::new();
    fs::write(w.agent.join("load-session"), "1").unwrap();
    w.submit(&good("before-the-restart"));
    w.wait_task(None);
    store_wire_session(&w, "wire-session-at-startup");
    w.restart();
    w.wait("the offer", |s| {
        s.session_restore.as_ref().is_some_and(|r| r.is_resumable)
    });

    let own = w.adapter_settings();
    let mut other = own.clone();
    other.provider = "scripted-other".into();
    w.wf().set_adapter(Some(other)).unwrap();
    let s = w.wait("the refused offer", |s| {
        s.session_restore.as_ref().is_some_and(|r| !r.is_resumable)
    });
    let notice = s.session_restore.as_ref().unwrap().notice.clone();
    assert!(
        notice
            .as_deref()
            .is_some_and(|n| n.contains("the configured agent is scripted-other")),
        "{notice:?}"
    );

    w.wf().set_adapter(Some(own)).unwrap();
    w.wait("the offer for the agent that ran it", |s| {
        s.session_restore.as_ref().is_some_and(|r| r.is_resumable)
    });
}

#[test]
fn row_ids_stay_unique_across_sessions_and_restarts() {
    let mut w = World::new();
    w.submit(&good("in-the-first"));
    w.wait_task(None);
    let first = w.wf().sessions().unwrap()[0].id.clone();
    w.wf().new_session().unwrap();
    w.wait("the new session", |s| s.chat == 1);
    w.submit(&good("in-the-second"));
    w.wait_task(None);
    let highest = w.snap().rows.last().unwrap().id;

    // Back to the first session, which holds the lower ids, then a restart.
    w.wf().open_session(&first).unwrap();
    w.wait("the first session", |s| s.chat == 2);
    w.restart();
    let s = w.snap();
    assert!(has_user(&s, "in-the-first"));
    w.submit(&good("after-the-restart"));
    w.poll("the new prompt", || {
        has_user(&w.snap(), "after-the-restart")
    });
    let fresh: Vec<_> = w
        .snap()
        .rows
        .iter()
        .filter(
            |r| matches!(&r.kind, RowKind::User { text, .. } if text.contains("after-the-restart")),
        )
        .map(|r| r.id)
        .collect();
    assert!(
        fresh.iter().all(|id| *id > highest),
        "{fresh:?} must be above {highest:?}"
    );
    w.wait_task(None);
}

#[test]
fn a_new_chat_in_an_empty_conversation_does_nothing() {
    let w = World::new();
    w.wf().new_session().unwrap();
    // Messages are handled in order: a page request answers after the new chat was handled.
    let _ = w.wf().history_page(RowId(u64::MAX), 1);
    assert_eq!(w.snap().chat, 0);
    assert_eq!(w.wf().sessions().unwrap().len(), 1);
    w.assert_clean();
}

#[test]
fn a_stored_session_that_cannot_resume_refuses_a_restore_request() {
    let w = World::new();
    w.submit(&good("no-load"));
    let first = w.wait_task(None);
    store_wire_session(&w, "wire-session-without-load-support");

    w.wf().restore_session().unwrap();
    w.wait("the refusal", |s| {
        has_notice(s, "does not support resuming")
    });
    w.submit(&good("fresh"));
    w.wait_task(Some(&first.id));
    assert!(read_load_params(&w).is_none(), "nothing is loaded");
    w.assert_clean();
}

#[test]
fn switching_to_and_restoring_a_session_with_a_refused_manifest_shows_the_reason() {
    let w = World::new();
    w.submit(&good("old-format"));
    w.wait_task(None);
    let project = w.snap().project.clone();
    let first = w.wf().sessions().unwrap()[0].id.clone();
    let manifest = w
        .paths
        .agent_session_dir(&project, &first)
        .join("manifest.json");
    let mut value: Value = serde_json::from_slice(&fs::read(&manifest).unwrap()).unwrap();
    value["schema_version"] = serde_json::json!(1);
    let foreign = serde_json::to_vec_pretty(&value).unwrap();
    fs::write(&manifest, &foreign).unwrap();

    w.wf().new_session().unwrap();
    w.wait("the new session", |s| s.chat == 1);
    w.wf().open_session(&first).unwrap();
    let s = w.wait("the stored session", |s| s.chat == 2);
    assert!(has_user(&s, "old-format"), "its rows are shown");
    let restore = s.session_restore.as_ref().expect("the refusal is shown");
    assert!(!restore.is_resumable, "{restore:?}");
    let notice = restore.notice.as_deref().unwrap_or_default();
    assert!(notice.contains("older Studio"), "{restore:?}");
    assert!(notice.contains("manifest.json"), "{restore:?}");
    assert!(notice.contains("read-only"), "{restore:?}");

    w.wf().restore_session().unwrap();
    w.wait("the restore refusal", |s| has_notice(s, "older Studio"));
    assert_eq!(fs::read(&manifest).unwrap(), foreign, "left as it is");
    w.assert_clean();
}

#[test]
fn switching_sessions_is_refused_while_a_candidate_awaits_review_and_while_it_is_applied() {
    let w = World::new();
    w.wf()
        .set_review_policy(studio_engine::ReviewPolicy::ManualReview)
        .unwrap();
    let project = w.snap().project.clone();
    let other = SessionStore::create_session(&w.paths, &project, None).unwrap();
    w.submit(&good("under-review"));
    w.wait_phase(TaskPhase::AwaitingReview);
    match w.wf().open_session(&other) {
        Err(WorkflowError::Busy(reason)) => {
            assert!(reason.contains("Apply or Discard"), "{reason}")
        }
        other => panic!("expected a busy refusal, got {other:?}"),
    }
    assert!(matches!(w.wf().new_session(), Err(WorkflowError::Busy(_))));

    let controller = w.ctl();
    let guard = controller.lock();
    w.wf().apply().unwrap();
    w.wait_phase(TaskPhase::Promoting);
    assert!(matches!(
        w.wf().open_session(&other),
        Err(WorkflowError::Busy(_))
    ));
    assert!(matches!(w.wf().new_session(), Err(WorkflowError::Busy(_))));
    drop(guard);
    w.wait_task(None);
    assert_eq!(w.snap().chat, 0);
    w.assert_clean();
}

#[test]
fn switching_sessions_is_refused_while_the_compile_of_a_task_runs() {
    let w = World::with(Options {
        block_compiler: true,
        ..Options::default()
    });
    let project = w.snap().project.clone();
    let other = SessionStore::create_session(&w.paths, &project, None).unwrap();
    w.submit(&good("compiling"));
    w.compiler.wait_started(1);
    w.wait_phase(TaskPhase::Validating);
    assert!(matches!(
        w.wf().open_session(&other),
        Err(WorkflowError::Busy(_))
    ));
    assert!(matches!(w.wf().new_session(), Err(WorkflowError::Busy(_))));
    assert_eq!(w.snap().chat, 0);
    w.compiler.release.store(true, Ordering::SeqCst);
    w.wait_task(None);
    w.assert_clean();
}

fn agent_messages(s: &WorkflowSnapshot) -> Vec<String> {
    s.rows
        .iter()
        .filter_map(|r| match &r.kind {
            RowKind::Agent { text, .. } => Some(text.clone()),
            _ => None,
        })
        .collect()
}

#[test]
fn a_reopened_session_restored_by_loading_shows_each_stored_row_once_while_the_agent_replays_it() {
    let w = World::new();
    fs::write(w.agent.join("load-session"), "1").unwrap();
    fs::write(w.agent.join("replay-on-load"), "1").unwrap();
    let mut script = good("stored-answer");
    script["text"] = json!("stored agent reply");
    w.submit(&script);
    w.wait_task(None);
    store_wire_session(&w, "wire-session-to-replay");
    let first = w.wf().sessions().unwrap()[0].id.clone();

    w.wf().new_session().unwrap();
    w.wait("the new session", |s| s.chat == 1);
    w.wf().open_session(&first).unwrap();
    let opened = w.wait("the stored session", |s| s.chat == 2);
    assert_eq!(
        agent_messages(&opened)
            .iter()
            .filter(|t| t.contains("stored agent reply"))
            .count(),
        1,
        "{:?}",
        opened.rows
    );

    // The next prompt loads the agent session, which replays its history before answering.
    w.submit(&good("after-reopen"));
    w.poll("the stored agent session is loaded", || {
        read_load_params(&w).is_some()
    });
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);

    let s = w.snap();
    let rows = format!("{:?}", s.rows);
    for replayed in ["replayed question", "replayed answer", "Replayed tool"] {
        assert!(!rows.contains(replayed), "{replayed} was shown: {rows}");
    }
    assert_eq!(
        users(&s)
            .iter()
            .filter(|t| t.contains("stored-answer"))
            .count(),
        1,
        "{rows}"
    );
    assert_eq!(
        agent_messages(&s)
            .iter()
            .filter(|t| t.contains("stored agent reply"))
            .count(),
        1,
        "{rows}"
    );
    let mut ids: Vec<_> = s.rows.iter().map(|r| r.id).collect();
    let total = ids.len();
    ids.dedup();
    assert_eq!(ids.len(), total, "row ids are unique: {rows}");
    w.assert_clean();
}
