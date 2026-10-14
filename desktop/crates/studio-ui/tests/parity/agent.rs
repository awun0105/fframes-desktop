//! Agent panel: a prompt sent to the scripted ACP agent, its streamed reply, a permission
//! request answered from the panel, and Apply (gated: see `Studio::mutation_open`).
use crate::{
    build_fixture::FAKE_CONFIG,
    harness::{FIXTURE_CONFIG, Mutation, Studio},
};
use serde_json::{Value, json};
use std::{fs, time::Duration};
use studio_ui::actions::{
    AnswerPermission, ApplyCandidate, FocusPrompt, OpenProject, SubmitPrompt, ToggleReviewPolicy,
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

    let open = studio.mutation_open(Mutation::AgentApply);
    if open {
        assert!(review["panel"]["apply_blocked"].is_null(), "{review:#}");
    }
    studio.dispatch(ApplyCandidate);
    if open {
        let applied = studio.wait("the edit applied", |t| {
            matches!(phase(t), "Accepted" | "Failed" | "Conflict")
        });
        assert_eq!(phase(&applied), "Accepted", "{applied:#}");
        assert_eq!(source(&studio), EDIT);
    } else {
        studio.wait("Apply shown as blocked", |t| {
            t["panel"]["apply_blocked"]
                .as_str()
                .is_some_and(|reason| !reason.is_empty())
        });
        let refused = studio.settle(Duration::from_millis(300));
        assert!(
            refused["panel"]["apply_blocked"]
                .as_str()
                .is_some_and(|reason| !reason.is_empty()),
            "Apply stopped being shown as blocked: {:#}",
            refused["panel"]
        );
        assert_eq!(phase(&refused), "Waiting for your review");
        assert_eq!(source(&studio), original);
    }
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

/// Opening another project while a turn waits for permission ends that project's agent: the
/// new project's panel shows nothing of the old turn, then or later.
#[gpui::test]
fn switching_projects_mid_turn_shows_nothing_of_the_old_turn(cx: &mut gpui::TestAppContext) {
    let mut studio = studio_with_agent(cx);
    let original = source(&studio);
    send_brief(&mut studio);
    let other = studio.another_project("other");
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
