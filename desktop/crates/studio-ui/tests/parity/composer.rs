//! The agent panel's composer, todo list, question card and history list, driven by real
//! keys and clicks against the scripted agents: what the user types, picks and clicks ends up
//! in what the agent receives and in what the panel shows next.
use crate::{
    build_fixture::FAKE_CONFIG,
    harness::{FIXTURE_CONFIG, Studio},
};
use serde_json::{Value, json};
use std::{fs, time::Duration};
use studio_ui::actions::ToggleReviewPolicy;

fn phase(t: &Value) -> &str {
    t["panel"]["phase"].as_str().unwrap_or_default()
}

fn composer(t: &Value) -> &Value {
    &t["panel"]["composer"]
}

fn rows<'a>(t: &'a Value, kind: &str) -> Vec<&'a Value> {
    t["panel"]["rows"]
        .as_array()
        .map(|rows| rows.iter().filter(|row| row["kind"] == kind).collect())
        .unwrap_or_default()
}

/// The scripted agent with `turns` queued; attached and ready.
fn studio_with_turns(cx: &mut gpui::TestAppContext, flags: &[&str], turns: Vec<Value>) -> Studio {
    let mut studio = Studio::with_agent(cx);
    for flag in flags {
        fs::write(studio.agent_evidence().join(flag), "1").unwrap();
    }
    fs::write(
        studio.agent_evidence().join("plan.json"),
        json!({"turns": turns}).to_string(),
    )
    .unwrap();
    studio.wait("agent panel attached", |t| t["panel"]["attached"] == true);
    studio
}

/// A turn that writes the project and ends, so the task is accepted.
fn edit_turn(marker: &str) -> Value {
    json!({
        "text": ["done"],
        "write": {"src/lib.rs": format!("// {marker}\n"), FAKE_CONFIG: FIXTURE_CONFIG},
    })
}

fn evidence(studio: &Studio, name: &str) -> Vec<Value> {
    fs::read_to_string(studio.agent_evidence().join(name))
        .unwrap_or_default()
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}

fn type_in_composer(studio: &mut Studio, text: &str) {
    studio.focus_prompt();
    studio.type_text(text);
}

fn button_center(t: &Value, id: &str) -> Option<(f32, f32)> {
    let b = t["panel"]["buttons"][id].as_array()?;
    let at = |i: usize| b[i].as_f64().map(|v| v as f32);
    Some((at(0)? + at(2)? / 2., at(1)? + at(3)? / 2.))
}

/// Clicks the element the panel reports under `id`, like a pointer would. The layout can
/// still be moving (a notice appearing above the tabs), so it clicks once two measurements
/// a frame apart agree.
fn click_id(studio: &mut Studio, id: &str) {
    let mut last = None;
    for _ in 0..40 {
        studio.report_button_bounds();
        let measured = studio.wait(&format!("{id} measured"), |t| {
            !t["panel"]["buttons"][id].is_null()
        });
        let at = button_center(&measured, id).unwrap();
        if last == Some(at) {
            studio.click(at);
            return;
        }
        last = Some(at);
        studio.settle(Duration::from_millis(50));
    }
    panic!("{id} never stopped moving");
}

/// Opens the History tab, which lists the stored sessions once on opening. A click can
/// still land on the old layout, so it clicks again until the panel shows the tab; opening
/// it twice only lists again.
fn open_history(studio: &mut Studio) {
    let deadline = std::time::Instant::now() + Duration::from_secs(60);
    loop {
        click_id(studio, "agent-tab-History");
        let shown = studio.settle(Duration::from_millis(100));
        if shown["panel"]["tab"] == "History" {
            return;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the History tab never opened"
        );
    }
}

fn tiny_jpeg(path: &std::path::Path) {
    let mut out = std::io::Cursor::new(Vec::new());
    image::DynamicImage::new_rgb8(8, 4)
        .write_to(&mut out, image::ImageFormat::Jpeg)
        .unwrap();
    fs::write(path, out.into_inner()).unwrap();
}

#[gpui::test]
fn slash_commands_are_listed_and_inserted(cx: &mut gpui::TestAppContext) {
    let mut studio = studio_with_turns(cx, &["commands"], vec![edit_turn("commands")]);
    type_in_composer(&mut studio, "make an edit");
    studio.press("enter");
    let done = studio.wait("the commands after the turn", |t| {
        phase(t) == "Accepted"
            && composer(t)["commands"]
                .as_array()
                .is_some_and(|c| c.len() == 2)
    });
    assert_eq!(composer(&done)["commands"], json!(["compact", "review"]));

    type_in_composer(&mut studio, "/");
    let open = studio.wait("the slash menu", |t| {
        composer(t)["slash_rows"].as_array().map(Vec::len) == Some(2)
    });
    assert_eq!(composer(&open)["text_chars"], 1);
    studio.press("tab");
    let inserted = studio.wait("the command inserted", |t| composer(t)["text_chars"] == 9);
    assert!(
        composer(&inserted)["slash_rows"]
            .as_array()
            .unwrap()
            .is_empty()
    );
}

#[gpui::test]
fn a_jpeg_arrives_at_the_agent_as_a_png(cx: &mut gpui::TestAppContext) {
    let mut studio = studio_with_turns(
        cx,
        &["prompt-image"],
        vec![
            json!({"text": "looking", "hang": true}),
            edit_turn("saw image"),
        ],
    );
    type_in_composer(&mut studio, "start");
    studio.press("enter");
    studio.wait("a live turn that takes images", |t| {
        composer(t)["turn_live"] == true && composer(t)["images_off"].is_null()
    });
    let jpeg = studio.path("shot.jpg");
    tiny_jpeg(&jpeg);
    click_id(&mut studio, "composer-attach-image");
    studio.choose_path(&jpeg);
    studio.wait("the image chip", |t| {
        composer(t)["chips"] == json!(["image"])
    });

    type_in_composer(&mut studio, "look at this");
    studio.press(if cfg!(target_os = "macos") {
        "cmd-enter"
    } else {
        "ctrl-enter"
    });
    studio.wait("the second prompt", |t| {
        phase(t) == "Accepted" && composer(t)["chips"] == json!([])
    });
    let prompts = evidence(&studio, "prompts.jsonl");
    assert_eq!(prompts.len(), 2, "{prompts:?}");
    let blocks = prompts[1]["image_blocks"].as_array().unwrap();
    assert_eq!(blocks.len(), 1, "{prompts:?}");
    assert_eq!(blocks[0]["mime_type"], "image/png");
}

#[gpui::test]
fn a_secret_file_is_refused_with_the_engines_reason(cx: &mut gpui::TestAppContext) {
    let mut studio = studio_with_turns(cx, &[], vec![edit_turn("unused")]);
    fs::write(studio.project_file(".env"), "TOKEN=1").unwrap();
    studio.wait("files accepted", |t| composer(t)["files_off"].is_null());
    click_id(&mut studio, "composer-attach-file");
    studio.choose_path(&studio.project_file(".env"));
    let refused = studio.wait("the refusal", |t| {
        composer(t)["last_refusal"]
            .as_str()
            .is_some_and(|r| !r.is_empty())
    });
    assert_eq!(composer(&refused)["chips"], json!([]));
    assert!(
        composer(&refused)["last_refusal"]
            .as_str()
            .unwrap()
            .to_lowercase()
            .contains("secret"),
        "{:#}",
        composer(&refused)
    );
}

#[gpui::test]
fn a_file_outside_the_project_needs_a_confirmation(cx: &mut gpui::TestAppContext) {
    let mut studio = studio_with_turns(cx, &[], vec![edit_turn("unused")]);
    let outside = studio.path("elsewhere.txt");
    fs::write(&outside, "hello").unwrap();
    studio.wait("files accepted", |t| composer(t)["files_off"].is_null());
    click_id(&mut studio, "composer-attach-file");
    studio.choose_path(&outside);
    let staged = studio.wait("the confirmation", |t| !composer(t)["stage"].is_null());
    assert_eq!(composer(&staged)["chips"], json!([]));
    click_id(&mut studio, "agent-stage-confirm");
    studio.wait("the chip", |t| {
        composer(t)["chips"] == json!(["file"]) && composer(t)["stage"].is_null()
    });
}

#[gpui::test]
fn a_message_for_the_next_step_is_held_until_the_open_tool_call_ends(
    cx: &mut gpui::TestAppContext,
) {
    let mut studio = studio_with_turns(
        cx,
        &[],
        vec![
            json!({"tool_open": [{"id": "open-1", "title": "Long step"}], "hang": true}),
            edit_turn("after step"),
        ],
    );
    type_in_composer(&mut studio, "start");
    studio.press("enter");
    studio.wait("a turn with an open tool", |t| {
        composer(t)["turn_live"] == true && composer(t)["steer"] == "AtBoundary"
    });
    studio.wait("the tool row", |t| !rows(t, "tool").is_empty());

    type_in_composer(&mut studio, "then do this");
    studio.press("enter");
    let held = studio.wait("the held message", |t| composer(t)["held"] == true);
    assert_eq!(evidence(&studio, "prompts.jsonl").len(), 1, "{held:#}");

    fs::write(studio.agent_evidence().join("close-tools"), "1").unwrap();
    studio.wait("the held message delivered", |t| {
        phase(t) == "Accepted" && composer(t)["held"] == false
    });
    let prompts = evidence(&studio, "prompts.jsonl");
    assert_eq!(prompts.len(), 2, "{prompts:?}");
    assert!(
        prompts[1]["text"]
            .as_str()
            .unwrap()
            .contains("then do this")
    );
}

#[gpui::test]
fn a_held_message_can_be_withdrawn(cx: &mut gpui::TestAppContext) {
    let mut studio = studio_with_turns(
        cx,
        &[],
        vec![json!({"tool_open": [{"id": "open-1", "title": "Long step"}], "hang": true})],
    );
    type_in_composer(&mut studio, "start");
    studio.press("enter");
    studio.wait("the tool row", |t| !rows(t, "tool").is_empty());
    type_in_composer(&mut studio, "then do this");
    studio.press("enter");
    studio.wait("the held message", |t| composer(t)["held"] == true);
    click_id(&mut studio, "agent-held-cancel");
    studio.wait("the message withdrawn", |t| composer(t)["held"] == false);
    assert_eq!(evidence(&studio, "prompts.jsonl").len(), 1);
}

#[gpui::test]
fn a_steer_reaches_the_agent_in_the_running_turn(cx: &mut gpui::TestAppContext) {
    let mut studio = studio_with_turns(
        cx,
        &["steering"],
        vec![
            json!({"text": "steered", "wait_steer": true, "write": {"src/lib.rs": "// steered\n", FAKE_CONFIG: FIXTURE_CONFIG}}),
        ],
    );
    type_in_composer(&mut studio, "start");
    studio.press("enter");
    studio.wait("a steerable turn", |t| {
        composer(t)["turn_live"] == true && composer(t)["steer"] == "Native"
    });
    type_in_composer(&mut studio, "use the blue title");
    studio.press("enter");
    studio.wait("the turn finished", |t| phase(t) == "Accepted");
    let steering = evidence(&studio, "steering.jsonl");
    assert_eq!(steering.len(), 1, "{steering:?}");
    assert!(steering[0].to_string().contains("use the blue title"));
    assert_eq!(evidence(&studio, "prompts.jsonl").len(), 1);
}

#[gpui::test]
fn send_now_cancels_the_turn_and_sends_the_new_message(cx: &mut gpui::TestAppContext) {
    let mut studio = studio_with_turns(
        cx,
        &[],
        vec![
            json!({"text": "starting", "hang": true}),
            edit_turn("redirected"),
        ],
    );
    type_in_composer(&mut studio, "start");
    studio.press("enter");
    studio.wait("a redirectable turn", |t| {
        composer(t)["turn_live"] == true && composer(t)["steer"] == "AtBoundary"
    });
    type_in_composer(&mut studio, "use the blue title");
    studio.press(if cfg!(target_os = "macos") {
        "cmd-enter"
    } else {
        "ctrl-enter"
    });
    studio.wait("the redirected turn", |t| phase(t) == "Accepted");
    assert!(!evidence(&studio, "cancels.jsonl").is_empty());
    let prompts = evidence(&studio, "prompts.jsonl");
    assert_eq!(prompts.len(), 2, "{prompts:?}");
    assert!(
        prompts[1]["text"]
            .as_str()
            .unwrap()
            .contains("use the blue title")
    );
}

#[gpui::test]
fn the_agents_plan_shows_as_a_todo_list(cx: &mut gpui::TestAppContext) {
    let mut turn = edit_turn("planned");
    turn["plan"] = json!([
        {"text": "Edit the title", "status": "in_progress"},
        {"text": "Check the result"},
    ]);
    let mut studio = studio_with_turns(cx, &[], vec![turn]);
    type_in_composer(&mut studio, "plan it");
    studio.press("enter");
    let shown = studio.wait("the todo list", |t| composer(t)["todo"]["visible"] == true);
    let items = composer(&shown)["todo"]["items"]
        .as_array()
        .unwrap()
        .clone();
    assert_eq!(items.len(), 2, "{items:?}");
    assert_eq!(items[0]["text"], "Edit the title");
    assert_eq!(items[0]["status"], "InProgress");
    assert_eq!(items[1]["status"], "Pending");
}

/// At the default window size a long todo list scrolls in its own box; the banner with
/// Review changes stays reachable by a real click.
#[gpui::test]
fn a_long_todo_list_leaves_review_reachable_at_the_default_size(cx: &mut gpui::TestAppContext) {
    let mut turn = edit_turn("reviewed");
    turn["plan"] = Value::Array(
        (1..=10)
            .map(|n| json!({"text": format!("Step {n} of the edit")}))
            .collect(),
    );
    let mut studio = studio_with_turns(cx, &[], vec![turn]);
    studio.dispatch(ToggleReviewPolicy);
    studio.wait("review before Apply", |t| {
        t["panel"]["review_policy"] == "ManualReview"
    });
    type_in_composer(&mut studio, "plan and edit");
    studio.press("enter");
    let waiting = studio.wait("the candidate with its todo list", |t| {
        matches!(phase(t), "Waiting for your review" | "Failed" | "Stopped")
            && composer(t)["todo"]["visible"] == true
    });
    assert_eq!(phase(&waiting), "Waiting for your review", "{waiting:#}");

    click_id(&mut studio, "agent-review-banner");
    let open = studio.wait("the review window", |t| {
        t["review"]["files"].as_u64() > Some(0)
    });
    assert_eq!(open["review"]["target"], "candidate", "{open:#}");
}

#[gpui::test]
fn the_history_lists_searches_and_reopens_stored_sessions(cx: &mut gpui::TestAppContext) {
    let mut studio = studio_with_turns(cx, &[], vec![edit_turn("first"), edit_turn("second")]);
    let first = "First request about the intro";
    let second = "Second one";
    type_in_composer(&mut studio, first);
    studio.press("enter");
    studio.wait("the first task accepted", |t| phase(t) == "Accepted");

    open_history(&mut studio);
    studio.wait("one stored session", |t| {
        composer(t)["history"]["shown"].as_array().map(Vec::len) == Some(1)
    });
    click_id(&mut studio, "history-new");
    studio.wait("a fresh chat", |t| {
        rows(t, "user").is_empty() && t["panel"]["tab"] == "Chat"
    });
    // The history button that was clicked is gone; the composer has the keys.
    studio.type_text(second);
    studio.press("enter");
    studio.wait("the second task accepted", |t| phase(t) == "Accepted");

    open_history(&mut studio);
    let listed = studio.wait("two stored sessions", |t| {
        composer(t)["history"]["shown"].as_array().map(Vec::len) == Some(2)
    });
    let shown = composer(&listed)["history"]["shown"]
        .as_array()
        .unwrap()
        .clone();
    let old = shown
        .iter()
        .position(|s| s["current"] == false)
        .expect("the earlier session is not the current one");
    assert_eq!(shown[old]["title_chars"], first.chars().count());

    // The search narrows the list to the matching session.
    click_id(&mut studio, "history-search");
    studio.type_text("intro");
    let narrowed = studio.wait("the filtered list", |t| {
        composer(t)["history"]["shown"].as_array().map(Vec::len) == Some(1)
    });
    assert_eq!(composer(&narrowed)["history"]["shown"][0]["current"], false);

    click_id(&mut studio, "history-row-0");
    let reopened = studio.wait("the stored session shown", |t| {
        rows(t, "user").len() == 1 && rows(t, "user")[0]["chars"] == first.len()
    });
    assert_eq!(reopened["panel"]["tab"], "Chat", "{:#}", reopened["panel"]);
}

#[gpui::test]
fn a_question_with_two_items_is_answered_from_its_card(cx: &mut gpui::TestAppContext) {
    let mut studio = Studio::with_native_agent(cx, studio_harness::AgentWire::ClaudeStreamJson);
    fs::write(
        studio.agent_evidence().join("writes.json"),
        json!({"src/lib.rs": "// asked\n", FAKE_CONFIG: FIXTURE_CONFIG}).to_string(),
    )
    .unwrap();
    studio.wait("agent panel attached", |t| t["panel"]["attached"] == true);
    type_in_composer(&mut studio, "ASK please make the edit");
    studio.press("enter");
    let asked = studio.wait("the question card", |t| {
        composer(t)["questions"]
            .as_array()
            .is_some_and(|q| !q.is_empty())
    });
    assert_eq!(composer(&asked)["questions"][0]["items"], 2);

    click_id(&mut studio, "question-0-0-opt-1");
    click_id(&mut studio, "question-0-1-opt-0");
    click_id(&mut studio, "question-0-1-opt-1");
    click_id(&mut studio, "question-0-1-text");
    studio.type_text("both");
    let ready = studio.wait("every item answered", |t| {
        composer(t)["questions"][0]["selected"] == json!([1, 2])
            && composer(t)["questions"][0]["typed"] == json!([false, true])
    });
    assert_eq!(phase(&ready), "Waiting for your permission", "{ready:#}");
    // The sidebar shares the window's height with the timeline: the card scrolls to Send.
    let (x, y) = button_center(&ready, "question-0-0-opt-0").unwrap();
    studio.scroll(x, y, -400., false);
    click_id(&mut studio, "question-0-send");
    studio.wait("the turn finished", |t| {
        phase(t) == "Accepted" && composer(t)["questions"] == json!([])
    });
    let answered = fs::read_to_string(studio.agent_evidence().join("question.json")).unwrap();
    for needle in ["Blue", "S", "M", "both"] {
        assert!(
            answered.contains(needle),
            "{needle} missing from {answered}"
        );
    }
}
