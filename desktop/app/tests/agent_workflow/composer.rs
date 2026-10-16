//! The composer: slash commands, the agent's plan, steering and sending while the agent
//! works, questions, attachments, and the history a loaded session replays.

use super::*;
use studio_engine::agent::{AttachmentRequest, ComposerView, QuestionAnswer, SteerSupport};

fn steering_agent() -> World {
    let w = World::new();
    fs::write(w.agent.join("steering"), "1").unwrap();
    w
}

fn claude(mode: &'static str) -> World {
    World::with(Options {
        fake_cli: Some((studio_harness::AgentWire::ClaudeStreamJson, mode)),
        ..Options::default()
    })
}

fn codex(mode: &'static str) -> World {
    World::with(Options {
        fake_cli: Some((studio_harness::AgentWire::CodexAppServer, mode)),
        ..Options::default()
    })
}

impl World {
    fn wait_composer(
        &self,
        what: &str,
        check: impl Fn(&ComposerView) -> bool,
    ) -> Arc<WorkflowSnapshot> {
        self.wait(what, |s| check(&s.composer))
    }

    fn followups(&self) -> Vec<String> {
        self.snap()
            .rows
            .iter()
            .filter_map(|r| match &r.kind {
                RowKind::User {
                    text,
                    source: UserSource::Followup,
                } => Some(text.clone()),
                _ => None,
            })
            .collect()
    }
}

fn agent_text(s: &WorkflowSnapshot) -> String {
    s.rows
        .iter()
        .filter_map(|r| match &r.kind {
            RowKind::Agent { text, .. } => Some(text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn script_line(script: &Value, text: &str) -> String {
    format!("SCRIPT {script}\n{text}")
}

fn file(path: impl Into<PathBuf>) -> AttachmentRequest {
    AttachmentRequest::File { path: path.into() }
}

fn names(view: &ComposerView) -> Vec<&str> {
    view.commands.iter().map(|c| c.name.as_str()).collect()
}

fn tiny_jpeg() -> Vec<u8> {
    let mut out = std::io::Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(8, 4)
        .write_to(&mut out, image::ImageFormat::Jpeg)
        .unwrap();
    out.into_inner()
}

fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack.windows(needle.len()).any(|w| w == needle)
}

fn tree_contains(dir: &std::path::Path, needle: &[u8]) -> bool {
    fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .any(|entry| {
            let path = entry.path();
            if path.is_dir() {
                tree_contains(&path, needle)
            } else {
                fs::read(&path).is_ok_and(|bytes| contains(&bytes, needle))
            }
        })
}

/// The conversation log and session records of the project (the draft copy of the project
/// is not part of them).
fn session_files_contain(w: &World, needle: &[u8]) -> bool {
    let project = w.snap().project.clone();
    let agent = w.paths.agent(&project);
    let sessions = w.paths.agent_sessions(&project);
    let manifest = w.paths.agent_session_manifest(&project);
    let legacy = w.paths.legacy_conversation_log(&project);
    let in_file = |path: &std::path::Path| fs::read(path).is_ok_and(|b| contains(&b, needle));
    tree_contains(&sessions, needle)
        || in_file(&manifest)
        || in_file(&legacy)
        || fs::read_dir(&agent)
            .into_iter()
            .flatten()
            .flatten()
            .any(|e| e.path().is_file() && in_file(&e.path()))
}

// ---- commands and plan ---------------------------------------------------------------------------------------

#[test]
fn commands_and_the_plan_are_published_and_the_plan_ends_with_its_task() {
    let w = World::new();
    fs::write(w.agent.join("commands"), "1").unwrap();
    let mut script = good("planned");
    script["plan"] = json!([
        {"text": "Edit the title", "status": "in_progress"},
        {"text": "Check the result"},
    ]);
    w.submit(&script);
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    let view = w.snap().composer.clone();
    assert_eq!(names(&view), ["compact", "review"]);
    assert_eq!(
        view.commands[1].input_hint.as_deref(),
        Some("what to review")
    );
    assert_eq!(view.plan.len(), 2);
    assert_eq!(view.plan[0].text, "Edit the title");
    assert_eq!(
        view.plan[0].status,
        studio_engine::agent::PlanStatus::InProgress
    );
    assert_eq!(
        view.plan[1].status,
        studio_engine::agent::PlanStatus::Pending
    );

    // The next task starts without the previous plan; the commands stay.
    w.submit(&good("second"));
    let second = w.wait_task(Some(&task.id));
    assert_eq!(second.phase, TaskPhase::Accepted, "{:?}", second.error);
    let view = w.snap().composer.clone();
    assert!(view.plan.is_empty(), "{:?}", view.plan);
    assert_eq!(names(&view), ["compact", "review"]);

    // A new chat is a new agent session: its commands are not carried over.
    w.wf().new_chat().unwrap();
    w.wait_composer("the commands cleared by the new chat", |c| {
        c.commands.is_empty()
    });
    w.assert_clean();
}

#[test]
fn commands_are_published_while_the_session_is_live() {
    let w = World::new();
    fs::write(w.agent.join("commands"), "1").unwrap();
    w.submit(&json!({"text": "working", "hang": true}));
    let s = w.wait_composer("the commands", |c| !c.commands.is_empty());
    assert_eq!(names(&s.composer), ["compact", "review"]);
    assert_eq!(
        s.composer.commands[1].input_hint.as_deref(),
        Some("what to review")
    );
    w.wf().stop().unwrap();
    let first = w.wait_task(None);
    // The list survives the end of the session, so the slash menu is not empty when the
    // next prompt is typed.
    assert_eq!(names(&w.snap().composer), ["compact", "review"]);

    // The next task's session announces its commands again; the list is replaced, not doubled.
    w.submit(&json!({"text": "again", "hang": true}));
    w.wait_composer("the next session live", |c| c.turn_live);
    assert_eq!(names(&w.snap().composer), ["compact", "review"]);
    w.wf().stop().unwrap();
    w.wait_task(Some(&first.id));
}

#[test]
fn claude_and_codex_publish_their_commands_and_plan() {
    let w = claude("turns");
    w.cli_writes("writes.json", "first");
    w.wf().submit("TODO please make the edit").unwrap();
    assert_eq!(w.wait_task(None).phase, TaskPhase::Accepted);
    assert_eq!(names(&w.snap().composer), ["compact", "review", "init"]);
    assert_two_step_plan(&w.snap().composer);

    let w = codex("turns");
    w.cli_writes("writes.json", "first");
    w.wf().submit("PLAN please make the edit").unwrap();
    assert_eq!(w.wait_task(None).phase, TaskPhase::Accepted);
    assert_eq!(names(&w.snap().composer), ["compact", "review"]);
    assert_two_step_plan(&w.snap().composer);
}

fn assert_two_step_plan(view: &ComposerView) {
    use studio_engine::agent::PlanStatus;
    let plan: Vec<_> = view
        .plan
        .iter()
        .map(|e| (e.text.as_str(), e.status))
        .collect();
    assert_eq!(
        plan,
        [
            ("First", PlanStatus::Completed),
            ("Second", PlanStatus::InProgress)
        ]
    );
}

// ---- replay on load ------------------------------------------------------------------------------------------

#[test]
fn a_loaded_session_does_not_replay_its_history_and_keeps_commands_and_plan() {
    let w = World::new();
    // The agent advertises loading from the start: the capabilities stored with the session decide
    // whether it can be resumed.
    fs::write(w.agent.join("load-session"), "1").unwrap();
    let mut first = good("seed");
    first["text"] = json!("first run said this");
    w.submit(&first);
    let seed = w.wait_task(None);
    assert_eq!(
        agent_text(&w.snap()).matches("first run said this").count(),
        1
    );

    fs::write(w.agent.join("replay-on-load"), "1").unwrap();
    let mut manifest =
        studio_engine::agent::session_store::SessionStore::load(&w.paths, &w.snap().project)
            .unwrap()
            .expect("manifest created by the first task");
    manifest.wire_session_id = Some("opaque-session".into());
    manifest.state = studio_engine::agent::session_store::SessionManifestState::Ready;
    studio_engine::agent::session_store::SessionStore::save(&w.paths, &w.snap().project, &manifest)
        .unwrap();
    w.wf().restore_session().unwrap();

    w.submit(&good("resumed"));
    let task = w.wait_task(Some(&seed.id));
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    w.poll("the load request", || {
        w.agent.join("session-load.json").is_file()
    });

    let s = w.snap();
    let text = format!("{:?}", s.rows);
    assert!(!text.contains("replayed question"), "{text}");
    assert!(!text.contains("replayed answer"), "{text}");
    assert!(!text.contains("Replayed tool"), "{text}");
    // The first task's rows are shown once, not again by the replay.
    assert_eq!(agent_text(&s).matches("first run said this").count(), 1);
    assert_eq!(names(&s.composer), ["compact", "review"]);
    assert_eq!(
        s.composer
            .plan
            .iter()
            .map(|e| e.text.as_str())
            .collect::<Vec<_>>(),
        ["resume work"]
    );
    w.assert_clean();
}

// ---- steering and sending while the agent works --------------------------------------------------------------

#[test]
fn an_agent_with_the_steering_extension_takes_a_message_in_the_running_turn() {
    let w = steering_agent();
    let mut script = good("steered");
    script["wait_steer"] = json!(true);
    w.submit(&script);
    let s = w.wait_composer("a steerable turn", |c| {
        c.turn_live && c.steer == SteerSupport::Native
    });
    assert!(s.task.is_some());
    assert!(matches!(
        w.wf().steer("   "),
        Err(WorkflowError::InvalidBrief(_))
    ));
    w.wf().steer("use the blue title").unwrap();
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    let steering = w.evidence("steering.jsonl");
    assert_eq!(steering.len(), 1, "{steering:?}");
    assert!(
        steering[0].to_string().contains("use the blue title"),
        "{steering:?}"
    );
    assert_eq!(w.followups(), ["use the blue title"]);
    assert!(agent_text(&w.snap()).contains("steered"));
    assert_eq!(
        w.evidence("prompts.jsonl").len(),
        1,
        "a steer is not a new prompt"
    );
    // Nothing is steerable once the turn is over.
    assert!(matches!(
        w.wf().steer("late"),
        Err(WorkflowError::Refused(_))
    ));
    w.assert_clean();
}

#[test]
fn an_agent_without_the_extension_is_redirected_by_send_now() {
    let w = World::new();
    w.submit(&json!({"text": "starting", "hang": true}));
    let s = w.wait_composer("a redirectable turn", |c| {
        c.turn_live && c.steer == SteerSupport::AtBoundary
    });
    assert!(s.composer.commands.is_empty());
    assert!(matches!(
        w.wf().steer("use the blue title"),
        Err(WorkflowError::Refused(_))
    ));
    w.wf()
        .send_now(
            &script_line(&good("redirected"), "use the blue title"),
            Vec::new(),
        )
        .unwrap();
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    assert_eq!(w.source("src/lib.rs"), "// redirected\n");
    let prompts = w.evidence("prompts.jsonl");
    assert_eq!(prompts.len(), 2);
    assert_eq!(
        prompts[0]["session"], prompts[1]["session"],
        "one provider session"
    );
    assert_eq!(w.followups().len(), 1);
    assert!(!w.evidence("cancels.jsonl").is_empty());
    w.assert_clean();
}

#[test]
fn a_message_for_the_next_step_waits_for_the_open_tool_call_and_can_be_withdrawn() {
    let w = World::new();
    w.submit(&json!({"tool_open": [{"id": "open-1", "title": "Long step"}], "hang": true}));
    w.wait_composer("a turn with an open tool", |c| c.turn_live);
    w.poll("the open tool call", || {
        w.snap()
            .rows
            .iter()
            .any(|r| matches!(&r.kind, RowKind::Tool(card) if card.call_id == "open-1"))
    });
    let held_text = script_line(&good("after step"), "then do this");
    w.wf().send_at_boundary(&held_text, Vec::new()).unwrap();
    let s = w.wait_composer("the held message", |c| c.held.is_some());
    assert!(s.composer.held.as_ref().unwrap().contains("then do this"));
    assert!(
        matches!(
            w.wf().send_at_boundary("another", Vec::new()),
            Err(WorkflowError::Refused(_))
        ),
        "one message waits at a time"
    );
    assert_eq!(
        w.evidence("prompts.jsonl").len(),
        1,
        "nothing was sent before the boundary"
    );

    // Withdrawing drops it; a new one is held and the step finishing releases it.
    w.wf().cancel_held().unwrap();
    w.wait_composer("the held message withdrawn", |c| c.held.is_none());
    w.wf().send_at_boundary(&held_text, Vec::new()).unwrap();
    w.wait_composer("the held message again", |c| c.held.is_some());
    w.release("close-tools");
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    assert_eq!(w.source("src/lib.rs"), "// after step\n");
    assert_eq!(w.evidence("prompts.jsonl").len(), 2);
    assert!(w.snap().composer.held.is_none());
    w.assert_clean();
}

#[test]
fn a_message_for_the_next_step_goes_at_once_when_no_tool_call_is_open() {
    let w = World::new();
    w.submit(&json!({"text": "thinking", "hang": true}));
    w.wait_composer("a live turn", |c| c.turn_live);
    w.wf()
        .send_at_boundary(&script_line(&good("immediate"), "go"), Vec::new())
        .unwrap();
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    assert_eq!(w.source("src/lib.rs"), "// immediate\n");
    w.assert_clean();
}

fn has_warning(s: &WorkflowSnapshot, needle: &str) -> bool {
    s.rows.iter().any(|r| {
        matches!(&r.kind, RowKind::Notice { level: NoticeLevel::Warning, text } if text.contains(needle))
    })
}

#[test]
fn sending_now_drops_the_held_message_visibly_and_delivers_only_the_new_one() {
    let w = World::new();
    w.submit(&json!({"tool_open": [{"id": "open-1", "title": "Long step"}], "hang": true}));
    w.wait_composer("a turn with an open tool", |c| c.turn_live);
    w.poll("the open tool call", || {
        w.snap()
            .rows
            .iter()
            .any(|r| matches!(&r.kind, RowKind::Tool(card) if card.call_id == "open-1"))
    });
    w.wf()
        .send_at_boundary(&script_line(&good("held text"), "HELD-MESSAGE"), Vec::new())
        .unwrap();
    w.wait_composer("the held message", |c| c.held.is_some());
    w.wf()
        .send_now(&script_line(&good("sent now"), "NOW-MESSAGE"), Vec::new())
        .unwrap();
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    assert_eq!(w.source("src/lib.rs"), "// sent now\n");
    let prompts = w.evidence("prompts.jsonl");
    assert_eq!(prompts.len(), 2, "{prompts:?}");
    assert!(
        prompts
            .iter()
            .all(|p| !p["text"].as_str().unwrap().contains("HELD-MESSAGE")),
        "the dropped message was sent later: {prompts:?}"
    );
    let s = w.snap();
    assert!(s.composer.held.is_none());
    assert!(has_warning(&s, "waiting for the next step was dropped"));
    w.assert_clean();
}

#[test]
fn a_held_message_is_dropped_with_a_warning_when_the_session_ends() {
    let w = World::new();
    w.submit(&json!({"tool_open": [{"id": "open-1", "title": "Long step"}], "hang": true}));
    w.wait_composer("a turn with an open tool", |c| c.turn_live);
    w.poll("the open tool call", || {
        w.snap()
            .rows
            .iter()
            .any(|r| matches!(&r.kind, RowKind::Tool(card) if card.call_id == "open-1"))
    });
    w.wf()
        .send_at_boundary(&script_line(&good("held text"), "HELD-MESSAGE"), Vec::new())
        .unwrap();
    w.wait_composer("the held message", |c| c.held.is_some());
    w.wf().stop().unwrap();
    w.wait_task(None);
    let s = w.wait_composer("the held message gone", |c| c.held.is_none());
    assert!(
        has_warning(
            &s,
            "waiting to be sent was dropped because the agent session ended"
        ),
        "{:?}",
        s.rows
    );
    assert_eq!(w.evidence("prompts.jsonl").len(), 1);
}

#[test]
fn a_steer_the_agent_refuses_is_not_shown_as_sent_and_comes_back_to_the_composer() {
    let w = steering_agent();
    fs::write(w.agent.join("steer-reject"), "1").unwrap();
    w.submit(&json!({"text": "working", "hang": true}));
    w.wait_composer("a steerable turn", |c| {
        c.turn_live && c.steer == SteerSupport::Native
    });
    w.wf().steer("use the blue title").unwrap();
    let s = w.wait_composer("the refused steer returned", |c| c.unsent.is_some());
    assert_eq!(s.composer.unsent.as_deref(), Some("use the blue title"));
    assert!(w.followups().is_empty(), "{:?}", w.followups());
    assert!(has_warning(&s, "was not sent"), "{:?}", s.rows);
    assert!(w.evidence("steering.jsonl").is_empty());
    w.wf().stop().unwrap();
    w.wait_task(None);
}

#[test]
fn claude_takes_a_steering_message() {
    let w = claude("turns");
    w.cli_writes("writes.json", "first");
    w.wf().submit("STEER please make the edit").unwrap();
    w.wait_composer("a steerable turn", |c| {
        c.turn_live && c.steer == SteerSupport::Native
    });
    w.wf().steer("also be brief").unwrap();
    assert_eq!(w.wait_task(None).phase, TaskPhase::Accepted);
    assert!(w.agent.join("steer.json").is_file());
    assert_eq!(w.followups(), ["also be brief"]);
}

#[test]
fn codex_takes_a_steering_message() {
    let w = codex("turns");
    w.cli_writes("writes.json", "first");
    w.wf().submit("STEER please make the edit").unwrap();
    // Asked before Codex announced the turn: delivered once it is.
    w.wait_composer("a steerable turn", |c| {
        c.turn_live && c.steer == SteerSupport::Native
    });
    w.wf().steer("also be brief").unwrap();
    assert_eq!(w.wait_task(None).phase, TaskPhase::Accepted);
    assert!(w.agent.join("steer.json").is_file());
}

// ---- questions -----------------------------------------------------------------------------------------------

#[test]
fn every_question_item_is_answered_before_anything_is_sent() {
    let w = claude("turns");
    w.cli_writes("writes.json", "first");
    w.wf().submit("ASK please make the edit").unwrap();
    let s = w.wait_composer("the question", |c| !c.questions.is_empty());
    let question = s.composer.questions[0].clone();
    assert_eq!(question.items.len(), 2);
    assert_eq!(question.items[0].options.len(), 2);
    assert!(question.items[1].multi_select);
    let reference = question.reference.clone();

    // Old single-choice replies cannot answer several questions.
    assert!(matches!(
        w.wf()
            .reply_permission(&reference, PermissionAnswer::Select("Red".into())),
        Err(WorkflowError::Refused(_))
    ));
    let pick = |label: &str| QuestionAnswer {
        selected: vec![label.into()],
        text: None,
    };
    // One answer for two questions, an option nobody offered, two picks for one choice.
    assert!(matches!(
        w.wf().answer_questions(&reference, vec![pick("Red")]),
        Err(WorkflowError::Refused(_))
    ));
    assert_eq!(
        w.wf()
            .answer_questions(&reference, vec![pick("Green"), pick("S")])
            .unwrap_err(),
        WorkflowError::UnknownChoice("Green".into())
    );
    assert!(matches!(
        w.wf().answer_questions(
            &reference,
            vec![
                QuestionAnswer {
                    selected: vec!["Red".into(), "Blue".into()],
                    text: None
                },
                pick("S"),
            ]
        ),
        Err(WorkflowError::Refused(_))
    ));
    assert!(
        !w.agent.join("question.json").exists(),
        "the agent saw an answer from a refused attempt"
    );
    assert_eq!(
        w.snap().composer.questions.len(),
        1,
        "the question stays open"
    );

    w.wf()
        .answer_questions(
            &reference,
            vec![
                pick("Blue"),
                QuestionAnswer {
                    selected: vec!["S".into(), "M".into()],
                    text: Some("both".into()),
                },
            ],
        )
        .unwrap();
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    let answered = fs::read_to_string(w.agent.join("question.json")).unwrap();
    for needle in ["Blue", "S", "M", "both"] {
        assert!(
            answered.contains(needle),
            "{needle} missing from {answered}"
        );
    }
    assert!(w.snap().composer.questions.is_empty());
    assert_eq!(
        w.wf()
            .answer_questions(&reference, vec![pick("Blue"), pick("S")])
            .unwrap_err(),
        WorkflowError::DuplicatePermission
    );
    w.assert_clean();
}

// ---- attachments ---------------------------------------------------------------------------------------------

#[test]
fn a_project_file_goes_to_the_agent_as_a_link_and_its_content_stays_out_of_the_log() {
    let w = World::new();
    fs::write(w.root.join("notes.txt"), "UNIQUE-FILE-CONTENT-7731").unwrap();
    let attached = w.wf().prepare_attachment(file("notes.txt")).unwrap();
    assert_eq!(attached.label(), "notes.txt");
    w.wf()
        .submit_with_attachments(
            &script_line(&good("with file"), "use the notes"),
            None,
            vec![attached],
        )
        .unwrap();
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    let prompts = w.evidence("prompts.jsonl");
    let links = prompts[0]["resource_links"].as_array().unwrap();
    assert_eq!(links.len(), 1, "{prompts:?}");
    assert!(
        links[0]["uri"].as_str().unwrap().starts_with("file:"),
        "{links:?}"
    );
    assert_eq!(links[0]["name"], "notes.txt");
    // The link points into the agent's working copy, never at the real project.
    let uri = links[0]["uri"].as_str().unwrap().to_lowercase();
    let cwd = prompts[0]["cwd"].as_str().unwrap();
    let cwd_leaf = std::path::Path::new(cwd)
        .file_name()
        .unwrap()
        .to_string_lossy()
        .to_lowercase();
    assert!(uri.contains(&cwd_leaf), "{uri} is not under {cwd}");
    let real = w.root.file_name().unwrap().to_string_lossy().to_lowercase();
    if real != cwd_leaf {
        assert!(
            !uri.contains(&format!("/{real}/")),
            "{uri} points at the project"
        );
    }
    let s = w.snap();
    assert!(
        s.rows.iter().any(|r| matches!(&r.kind,
        RowKind::User { text, source: UserSource::Brief } if text.contains("Attached: notes.txt")))
    );
    assert!(!format!("{:?}", s.rows).contains("UNIQUE-FILE-CONTENT-7731"));
    assert!(!session_files_contain(&w, b"UNIQUE-FILE-CONTENT-7731"));
    w.assert_clean();
}

#[test]
fn secret_files_are_refused_and_outside_files_need_a_confirmation_naming_them() {
    let w = World::new();
    fs::write(w.root.join(".env"), "TOKEN=1").unwrap();
    fs::create_dir_all(w.root.join(".ssh")).unwrap();
    fs::write(w.root.join(".ssh/id"), "k").unwrap();
    for denied in [".env", ".ssh/id"] {
        let error = w.wf().prepare_attachment(file(denied)).unwrap_err();
        assert!(
            matches!(error, WorkflowError::Refused(_)),
            "{denied}: {error:?}"
        );
    }

    let outside = tempfile::tempdir().unwrap();
    let path = outside.path().join("elsewhere.txt");
    fs::write(&path, "hello").unwrap();
    assert!(matches!(
        w.wf().prepare_attachment(file(&path)),
        Err(WorkflowError::Refused(_))
    ));
    let staged = w.wf().stage_attachment(&path).unwrap();
    assert_eq!(staged.label, "elsewhere.txt");
    assert_eq!(staged.path, fs::canonicalize(&path).unwrap());
    let kept = w
        .wf()
        .prepare_attachment(AttachmentRequest::Staged {
            token: staged.token.clone(),
        })
        .unwrap();
    assert_eq!(kept.label(), "elsewhere.txt");
    // A confirmation is good for one attachment.
    assert!(matches!(
        w.wf().prepare_attachment(AttachmentRequest::Staged {
            token: staged.token
        }),
        Err(WorkflowError::Refused(_))
    ));
    // A secret cannot even be staged for confirmation.
    let secret = outside.path().join("server.pem");
    fs::write(&secret, "x").unwrap();
    assert!(matches!(
        w.wf().stage_attachment(&secret),
        Err(WorkflowError::Refused(_))
    ));
    // Nor can a file inside the project be staged as if it were outside.
    assert!(matches!(
        w.wf().stage_attachment(&w.root.join("src/lib.rs")),
        Err(WorkflowError::Refused(_))
    ));
}

#[test]
fn a_confirmed_outside_file_travels_as_content_when_the_agent_takes_it() {
    let w = World::new();
    fs::write(w.agent.join("embedded-context"), "1").unwrap();
    let outside = tempfile::tempdir().unwrap();
    let path = outside.path().join("elsewhere.txt");
    fs::write(&path, "OUTSIDE-CONTENT-4412").unwrap();
    let staged = w.wf().stage_attachment(&path).unwrap();
    let attached = w
        .wf()
        .prepare_attachment(AttachmentRequest::Staged {
            token: staged.token,
        })
        .unwrap();
    w.wf()
        .submit_with_attachments(
            &script_line(&good("with outside"), "use that file"),
            None,
            vec![attached],
        )
        .unwrap();
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    let prompts = w.evidence("prompts.jsonl");
    assert!(prompts[0]["resource_links"].as_array().unwrap().is_empty());
    let embedded = prompts[0]["embedded_resources"].as_array().unwrap();
    assert_eq!(embedded.len(), 1, "{prompts:?}");
    assert!(
        embedded[0].to_string().contains("OUTSIDE-CONTENT-4412") || embedded[0]["blob"].is_string()
    );
    let s = w.snap();
    assert!(
        s.rows.iter().any(|r| matches!(&r.kind,
        RowKind::User { text, source: UserSource::Brief } if text.contains("Attached: elsewhere.txt")))
    );
    assert!(!format!("{:?}", s.rows).contains("OUTSIDE-CONTENT-4412"));
    assert!(!session_files_contain(&w, b"OUTSIDE-CONTENT-4412"));
    w.assert_clean();
}

#[test]
fn an_outside_file_is_not_sent_or_named_when_the_agent_did_not_negotiate_content() {
    let w = World::new();
    let outside = tempfile::tempdir().unwrap();
    let path = outside.path().join("elsewhere.txt");
    fs::write(&path, "OUTSIDE-CONTENT-9901").unwrap();
    let staged = w.wf().stage_attachment(&path).unwrap();
    let attached = w
        .wf()
        .prepare_attachment(AttachmentRequest::Staged {
            token: staged.token,
        })
        .unwrap();
    w.wf()
        .submit_with_attachments(
            &script_line(&good("without"), "use that file"),
            None,
            vec![attached],
        )
        .unwrap();
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    let prompts = w.evidence("prompts.jsonl");
    assert!(
        prompts[0]["embedded_resources"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(prompts[0]["resource_links"].as_array().unwrap().is_empty());
    let s = w.snap();
    assert!(
        !s.rows.iter().any(|r| matches!(&r.kind,
        RowKind::User { text, .. } if text.contains("Attached:"))),
        "the conversation names a file that was not sent"
    );
    assert!(s.rows.iter().any(|r| matches!(&r.kind,
        RowKind::Notice { level: NoticeLevel::Warning, text } if text.contains("attachments were not sent"))));
    w.assert_clean();
}

#[test]
fn an_image_is_sent_as_png_within_a_running_turn_and_its_bytes_stay_out_of_the_log() {
    let w = World::new();
    fs::write(w.agent.join("prompt-image"), "1").unwrap();
    w.submit(&json!({"text": "looking", "hang": true}));
    let s = w.wait_composer("a live turn that takes images", |c| c.turn_live && c.images);
    assert!(s.composer.files);
    let image = w
        .wf()
        .prepare_attachment(AttachmentRequest::Image {
            label: "shot.jpg".into(),
            bytes: tiny_jpeg(),
        })
        .unwrap();
    assert!(image.is_image());
    assert!(!format!("{image:?}").contains("iVBOR"));
    w.wf()
        .send_now(
            &script_line(&good("saw image"), "look at this"),
            vec![image],
        )
        .unwrap();
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    let prompts = w.evidence("prompts.jsonl");
    let blocks = prompts[1]["image_blocks"].as_array().unwrap();
    assert_eq!(blocks.len(), 1, "{prompts:?}");
    assert_eq!(blocks[0]["mime_type"], "image/png");
    assert!(w.followups()[0].contains("Attached: shot.jpg"));
    assert!(!session_files_contain(&w, b"\x89PNG"));
    assert!(!session_files_contain(&w, b"iVBORw0KGgo"));
    w.assert_clean();
}

#[test]
fn an_image_is_refused_with_a_reason_when_the_agent_did_not_advertise_image_prompts() {
    let w = World::new();
    w.submit(&json!({"text": "looking", "hang": true}));
    w.wait_composer("a live turn", |c| c.turn_live);
    assert!(!w.snap().composer.images);
    let error = w
        .wf()
        .prepare_attachment(AttachmentRequest::Image {
            label: "shot.jpg".into(),
            bytes: tiny_jpeg(),
        })
        .unwrap_err();
    assert!(
        matches!(&error, WorkflowError::Refused(reason) if reason.contains("image")),
        "{error:?}"
    );
    w.wf().stop().unwrap();
    w.wait_task(None);
}

#[test]
fn claude_and_codex_refuse_files_with_a_reason() {
    for w in [claude("turns"), codex("turns")] {
        fs::write(w.root.join("notes.txt"), "x").unwrap();
        assert!(!w.snap().composer.files);
        let error = w.wf().prepare_attachment(file("notes.txt")).unwrap_err();
        assert!(
            matches!(&error, WorkflowError::Refused(reason) if reason.contains("file")),
            "{error:?}"
        );
    }
}

// ---- commands and plan across conversations ------------------------------------------------------------------

/// Runs a task that publishes two commands and a plan and checks both are on show.
fn publish_commands_and_plan(w: &World) {
    fs::write(w.agent.join("commands"), "1").unwrap();
    let mut script = good("planned");
    script["plan"] = json!([{"text": "Edit the title", "status": "in_progress"}]);
    w.submit(&script);
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    let view = w.snap().composer.clone();
    assert_eq!(names(&view), ["compact", "review"]);
    assert_eq!(view.plan.len(), 1);
}

#[test]
fn opening_another_stored_session_clears_the_plan_and_commands_until_the_agent_publishes_again() {
    let w = World::new();
    let project = w.snap().project.clone();
    let other =
        studio_engine::agent::session_store::SessionStore::create_session(&w.paths, &project, None)
            .unwrap();
    publish_commands_and_plan(&w);

    w.wf().open_session(&other).unwrap();
    let s = w.wait("the other session", |s| s.chat == 1);
    assert!(s.composer.plan.is_empty(), "{:?}", s.composer.plan);
    assert!(s.composer.commands.is_empty(), "{:?}", s.composer.commands);

    // The agent of the open session announces its own again.
    let mut script = good("again");
    script["plan"] = json!([{"text": "Second plan"}]);
    w.submit(&script);
    w.wait_task(None);
    let view = w.snap().composer.clone();
    assert_eq!(names(&view), ["compact", "review"]);
    assert_eq!(view.plan.len(), 1);
    assert_eq!(view.plan[0].text, "Second plan");
    w.assert_clean();
}

#[test]
fn starting_a_new_chat_clears_the_plan_and_commands_until_the_agent_publishes_again() {
    let w = World::new();
    publish_commands_and_plan(&w);

    w.wf().new_session().unwrap();
    let s = w.wait("the new chat", |s| s.chat == 1);
    assert!(s.composer.plan.is_empty(), "{:?}", s.composer.plan);
    assert!(s.composer.commands.is_empty(), "{:?}", s.composer.commands);

    w.submit(&good("fresh"));
    w.wait_task(None);
    let view = w.snap().composer.clone();
    assert_eq!(names(&view), ["compact", "review"]);
    assert!(view.plan.is_empty(), "{:?}", view.plan);
    w.assert_clean();
}
