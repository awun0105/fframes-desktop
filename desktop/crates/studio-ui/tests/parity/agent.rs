//! Agent panel: a prompt sent to the scripted ACP agent, its streamed reply, a permission
//! request answered from the panel, and Apply.
use crate::{
    build_fixture::FAKE_CONFIG,
    harness::{FIXTURE_CONFIG, Studio},
};
use serde_json::{Value, json};
use std::{fs, time::Duration};
use studio_ui::actions::{
    AnswerPermission, ApplyCandidate, BuildPreview, FocusPrompt, OpenProject, Seek, SubmitPrompt,
    ToggleReviewPolicy,
};

const BRIEF: &str = "Make the title red";
const REPLY: [&str; 2] = ["Working on it. ", "done"];
const EDIT: &str = "// the title is red\n";

/// The fixture open with the scripted agent attached, playing one turn that streams
/// [`REPLY`], asks permission to edit, and on Allow writes [`EDIT`].
fn studio_with_agent(cx: &mut gpui::TestAppContext) -> Studio {
    let mut studio = Studio::with_agent(cx);
    let turn = json!({
        "text": REPLY,
        "permission": {"title": "Edit src/lib.rs"},
        "write": {"src/lib.rs": EDIT, FAKE_CONFIG: FIXTURE_CONFIG},
    });
    fs::write(
        studio.agent_evidence().join("plan.json"),
        json!({"turns": [turn]}).to_string(),
    )
    .unwrap();
    studio.wait("agent panel attached", |t| t["panel"]["attached"] == true);
    studio
}

fn phase(t: &Value) -> &str {
    t["panel"]["phase"].as_str().unwrap_or_default()
}

fn rows<'a>(t: &'a Value, kind: &str) -> Vec<&'a Value> {
    t["panel"]["rows"]
        .as_array()
        .map(|rows| rows.iter().filter(|row| row["kind"] == kind).collect())
        .unwrap_or_default()
}

/// Types [`BRIEF`] into the prompt and sends it, then waits for the permission request.
fn send_brief(studio: &mut Studio) -> Value {
    studio.dispatch(FocusPrompt);
    studio.settle(Duration::from_millis(50));
    studio.type_text(BRIEF);
    studio.dispatch(SubmitPrompt);
    studio.wait("the permission request", |t| {
        phase(t) == "Waiting for your permission" && t["panel"]["open_permissions"] == 1
    })
}

/// Keys pressed in the prompt are text, and a real Enter sends it: the keyboard path,
/// next to the `SubmitPrompt` action [`send_brief`] uses.
#[gpui::test]
fn space_types_in_the_prompt_and_enter_sends_it(cx: &mut gpui::TestAppContext) {
    let mut studio = studio_with_agent(cx);
    studio.dispatch(FocusPrompt);
    studio.settle(Duration::from_millis(50));
    studio.type_text("Make");
    studio.press("space");
    studio.type_text("the title red");
    studio.press("enter");
    let asking = studio.wait("the permission request", |t| {
        phase(t) == "Waiting for your permission"
    });
    assert_eq!(rows(&asking, "user")[0]["chars"], BRIEF.len());
    assert_eq!(asking["playing"], false);
    let prompts = fs::read_to_string(studio.agent_evidence().join("prompts.jsonl")).unwrap();
    assert!(prompts.contains(BRIEF), "{prompts}");
}

/// Undo in a focused field undoes the field's own edit and nothing outside it: the brief
/// sent is the restored text, and the playhead and the timeline selection stay put.
#[gpui::test]
fn undo_in_the_prompt_undoes_the_typing_there(cx: &mut gpui::TestAppContext) {
    let mut studio = studio_with_agent(cx);
    studio.dispatch(BuildPreview);
    studio.wait("preview displayed", |t| {
        t["preview"]["status"] == "displayed"
    });
    studio.dispatch(Seek { frame: 30 });
    studio.wait("at 30", |t| t["position"] == 30);
    studio.dispatch(FocusPrompt);
    studio.settle(Duration::from_millis(50));
    studio.type_text(BRIEF);
    studio.press("backspace backspace backspace");
    studio.press(if cfg!(target_os = "macos") {
        "cmd-z"
    } else {
        "ctrl-z"
    });
    studio.press("enter");
    let asking = studio.wait("the permission request", |t| {
        phase(t) == "Waiting for your permission"
    });
    assert_eq!(rows(&asking, "user")[0]["chars"], BRIEF.len());
    assert_eq!(asking["position"], 30);
    assert_eq!(asking["timeline"]["selection"]["range"], Value::Null);
    let prompts = fs::read_to_string(studio.agent_evidence().join("prompts.jsonl")).unwrap();
    assert!(prompts.contains(BRIEF), "{prompts}");
}

fn source(studio: &Studio) -> String {
    fs::read_to_string(studio.project_file("src/lib.rs")).unwrap()
}

#[gpui::test]
fn a_prompt_streams_the_reply_asks_permission_and_applies(cx: &mut gpui::TestAppContext) {
    let mut studio = studio_with_agent(cx);
    let original = source(&studio);
    studio.dispatch(ToggleReviewPolicy);
    studio.wait("review before Apply", |t| {
        t["panel"]["review_policy"] == "ManualReview"
    });

    let asking = send_brief(&mut studio);
    let user = rows(&asking, "user");
    assert_eq!(user.len(), 1, "{:#}", asking["panel"]["rows"]);
    assert_eq!(user[0]["chars"], BRIEF.len());
    let reply = rows(&asking, "agent");
    assert_eq!(reply.len(), 1, "{:#}", asking["panel"]["rows"]);
    assert_eq!(reply[0]["chars"], REPLY.concat().len());
    let prompts = fs::read_to_string(studio.agent_evidence().join("prompts.jsonl")).unwrap();
    assert!(prompts.contains(BRIEF), "{prompts}");
    assert_eq!(source(&studio), original);

    studio.dispatch(AnswerPermission {
        option: Some("allow".into()),
    });
    let review = studio.wait("the candidate", |t| {
        matches!(phase(t), "Waiting for your review" | "Failed" | "Stopped")
    });
    assert_eq!(phase(&review), "Waiting for your review", "{review:#}");
    let answer: Value = serde_json::from_slice(
        &fs::read(studio.agent_evidence().join("permission-perm-1.json")).unwrap(),
    )
    .unwrap();
    assert_eq!(answer["optionId"], "allow", "{answer}");
    assert_eq!(
        source(&studio),
        original,
        "nothing is published before Apply"
    );

    assert!(review["panel"]["apply_blocked"].is_null(), "{review:#}");
    studio.dispatch(ApplyCandidate);
    let applied = studio.wait("the edit applied", |t| {
        matches!(phase(t), "Accepted" | "Failed" | "Conflict")
    });
    assert_eq!(phase(&applied), "Accepted", "{applied:#}");
    assert_eq!(source(&studio), EDIT);
}

#[gpui::test]
fn a_declined_permission_writes_nothing(cx: &mut gpui::TestAppContext) {
    let mut studio = studio_with_agent(cx);
    let original = source(&studio);
    send_brief(&mut studio);
    studio.dispatch(AnswerPermission { option: None });
    let ended = studio.wait("the turn to end", |t| {
        t["panel"]["open_permissions"] == 0
            && !matches!(
                phase(t),
                "Waiting for your permission" | "Editing" | "Starting the agent"
            )
    });
    let permission = rows(&ended, "permission");
    assert_eq!(permission.len(), 1);
    assert_eq!(permission[0]["state"], "Cancelled", "{:#}", ended["panel"]);
    assert_ne!(phase(&ended), "Accepted");
    assert_eq!(source(&studio), original);
}

/// The same panel flow over a native wire: the fake CLI asks before its edit (the
/// uppercase `EDIT` in the brief), the card's allow option answers it, and the candidate
/// holds what the agent wrote into its draft.
fn native_prompt_asks_permission_and_applies(
    cx: &mut gpui::TestAppContext,
    wire: studio_harness::AgentWire,
    allow: &str,
) {
    let mut studio = Studio::with_native_agent(cx, wire);
    fs::write(
        studio.agent_evidence().join("writes.json"),
        json!({"src/lib.rs": EDIT, FAKE_CONFIG: FIXTURE_CONFIG}).to_string(),
    )
    .unwrap();
    studio.wait("agent panel attached", |t| t["panel"]["attached"] == true);
    let original = source(&studio);
    studio.dispatch(ToggleReviewPolicy);
    studio.wait("review before Apply", |t| {
        t["panel"]["review_policy"] == "ManualReview"
    });
    studio.dispatch(FocusPrompt);
    studio.settle(Duration::from_millis(50));
    studio.type_text("EDIT the title");
    studio.dispatch(SubmitPrompt);
    let asking = studio.wait("the permission request", |t| {
        phase(t) == "Waiting for your permission" && t["panel"]["open_permissions"] == 1
    });
    assert_eq!(
        rows(&asking, "permission").len(),
        1,
        "{:#}",
        asking["panel"]
    );
    assert!(!rows(&asking, "agent").is_empty(), "{:#}", asking["panel"]);

    studio.dispatch(AnswerPermission {
        option: Some(allow.into()),
    });
    let review = studio.wait("the candidate", |t| {
        matches!(phase(t), "Waiting for your review" | "Failed" | "Stopped")
    });
    assert_eq!(phase(&review), "Waiting for your review", "{review:#}");
    assert_eq!(
        source(&studio),
        original,
        "nothing is published before Apply"
    );
    studio.dispatch(ApplyCandidate);
    let applied = studio.wait("the edit applied", |t| {
        matches!(phase(t), "Accepted" | "Failed" | "Conflict")
    });
    assert_eq!(phase(&applied), "Accepted", "{applied:#}");
    assert_eq!(source(&studio), EDIT);
}

#[gpui::test]
fn over_claude_stream_json_a_prompt_asks_permission_and_applies(cx: &mut gpui::TestAppContext) {
    native_prompt_asks_permission_and_applies(
        cx,
        studio_harness::AgentWire::ClaudeStreamJson,
        "allow",
    );
}

#[gpui::test]
fn over_codex_app_server_a_prompt_asks_permission_and_applies(cx: &mut gpui::TestAppContext) {
    native_prompt_asks_permission_and_applies(
        cx,
        studio_harness::AgentWire::CodexAppServer,
        "accept",
    );
}

/// Opening another project while a turn waits for permission ends that project's agent: the
/// new project's panel shows nothing of the old turn, then or later.
#[gpui::test]
fn switching_projects_mid_turn_shows_nothing_of_the_old_turn(cx: &mut gpui::TestAppContext) {
    let mut studio = studio_with_agent(cx);
    let original = source(&studio);
    send_brief(&mut studio);
    let other = studio.another_project("other");
    // The shell ignores Open while it is still preparing the first project.
    studio.wait("the first project ready", |t| t["busy"] == false);
    studio.dispatch(OpenProject);
    studio.choose_path(&other);
    let switched = studio.wait("the other project with its own agent", |t| {
        t["busy"] == false
            && t["project_path"]
                .as_str()
                .is_some_and(|path| path.ends_with("other"))
            && t["panel"]["attached"] == true
    });
    let after = studio.settle(Duration::from_secs(2));
    for t in [&switched, &after] {
        assert!(rows(t, "user").is_empty(), "{:#}", t["panel"]);
        assert!(rows(t, "agent").is_empty(), "{:#}", t["panel"]);
        assert!(rows(t, "permission").is_empty(), "{:#}", t["panel"]);
        assert_eq!(t["panel"]["open_permissions"], 0, "{:#}", t["panel"]);
        assert_eq!(t["panel"]["phase"], Value::Null, "{:#}", t["panel"]);
    }
    assert_eq!(source(&studio), original);
}

/// After a switch mid-turn the new project's agent still reaches the panel: its reply
/// streams in to the last chunk.
#[gpui::test]
fn the_agent_of_the_project_switched_to_still_updates_the_panel(cx: &mut gpui::TestAppContext) {
    let mut studio = studio_with_agent(cx);
    let second = ["Second ", "reply"];
    fs::write(
        studio.agent_evidence().join("plan.json"),
        json!({"turns": [
            {"text": REPLY, "permission": {"title": "Edit src/lib.rs"}},
            {"text": second},
        ]})
        .to_string(),
    )
    .unwrap();
    send_brief(&mut studio);
    let other = studio.another_project("other");
    // The shell ignores Open while it is still preparing the first project.
    studio.wait("the first project ready", |t| t["busy"] == false);
    studio.dispatch(OpenProject);
    studio.choose_path(&other);
    studio.wait("the other project with its own agent", |t| {
        t["busy"] == false
            && t["project_path"]
                .as_str()
                .is_some_and(|path| path.ends_with("other"))
            && t["panel"]["attached"] == true
    });
    studio.dispatch(FocusPrompt);
    studio.settle(Duration::from_millis(50));
    studio.type_text(BRIEF);
    studio.dispatch(SubmitPrompt);
    let replied = studio.wait("the second reply", |t| {
        rows(t, "agent")
            .first()
            .is_some_and(|row| row["chars"] == second.concat().len())
    });
    assert_eq!(rows(&replied, "user").len(), 1, "{:#}", replied["panel"]);
}

fn scroll_state(t: &Value) -> (bool, f64) {
    let scroll = &t["panel"]["scroll"];
    (
        scroll["following"] == true,
        scroll["distance_from_bottom"].as_f64().unwrap(),
    )
}

/// New rows keep the transcript at its end unless the reader scrolled up by more than
/// the stick threshold (70 px): a 50 px nudge is pinned again by the next reply, a
/// 150 px scroll is left where it is.
#[gpui::test]
fn new_rows_stay_pinned_unless_the_reader_scrolled_up_past_the_threshold(
    cx: &mut gpui::TestAppContext,
) {
    let mut studio = studio_with_agent(cx);
    fs::write(
        studio.agent_evidence().join("plan.json"),
        json!({"turns": [
            {"text": REPLY, "tool": 30},
            {"text": REPLY, "tool": 4},
            {"text": REPLY, "tool": 4},
        ]})
        .to_string(),
    )
    .unwrap();
    let send = |studio: &mut Studio, tools: usize| {
        studio.dispatch(FocusPrompt);
        studio.settle(Duration::from_millis(50));
        studio.type_text(BRIEF);
        studio.dispatch(SubmitPrompt);
        // The turn is over, so no row arrives after the reader moves the list.
        studio.wait("the reply's tool cards and the end of the turn", |t| {
            rows(t, "tool").len() == tools
                && phase(t) == "Waiting for your answer"
                && scroll_state(t).1.is_finite()
        });
        studio.settle(Duration::from_millis(200))
    };
    let first = send(&mut studio, 30);
    assert_eq!(scroll_state(&first), (true, 0.), "{:#}", first["panel"]);

    let prompt = first["panel"]["prompt_bounds"].as_array().unwrap().clone();
    let (x, y) = (
        prompt[0].as_f64().unwrap() as f32 + 40.,
        prompt[1].as_f64().unwrap() as f32 - 120.,
    );
    studio.scroll(x, y, 50., false);
    let nudged = studio.settle(Duration::from_millis(100));
    let (following, distance) = scroll_state(&nudged);
    assert!(
        !following && (40. ..=60.).contains(&distance),
        "{:#}",
        nudged["panel"]
    );
    let second = send(&mut studio, 34);
    assert_eq!(scroll_state(&second), (true, 0.), "{:#}", second["panel"]);

    studio.scroll(x, y, 150., false);
    let away = studio.settle(Duration::from_millis(100));
    assert!(scroll_state(&away).1 >= 140., "{:#}", away["panel"]);
    let third = send(&mut studio, 38);
    let (following, distance) = scroll_state(&third);
    assert!(!following && distance > 70., "{:#}", third["panel"]);
}
