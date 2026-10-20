//! Claude Code stream-json lines the driver writes, and the answers to control requests.

use crate::driver::{QuestionAnswer, QuestionItem};
use serde_json::{Value, json};

/// The request id of the driver's one `initialize` request.
pub(crate) const INITIALIZE_ID: &str = "fframes_initialize";

pub(crate) fn initialize_line() -> String {
    json!({
        "type": "control_request",
        "request_id": INITIALIZE_ID,
        "request": { "subtype": "initialize" },
    })
    .to_string()
}

/// Stops the running turn; queued steers are dropped. The process stays up.
pub(crate) fn interrupt_line(request_id: &str) -> String {
    json!({
        "type": "control_request",
        "request_id": request_id,
        "request": { "subtype": "interrupt", "cancel_queued": true },
    })
    .to_string()
}

/// A base64 PNG for a prompt.
pub(crate) struct Image<'a> {
    pub data: &'a str,
}

/// Plain string content without images; image blocks first, then the text, with them.
fn content(text: &str, images: &[Image<'_>]) -> Value {
    if images.is_empty() {
        return Value::from(text);
    }
    let mut blocks: Vec<Value> = images
        .iter()
        .map(|image| {
            json!({
                "type": "image",
                "source": { "type": "base64", "media_type": "image/png", "data": image.data },
            })
        })
        .collect();
    blocks.push(json!({ "type": "text", "text": text }));
    Value::Array(blocks)
}

/// A prompt, or with `priority` a steer into the running turn (`"now"` at the next step
/// boundary, `"next"` after the tool calls in flight).
pub(crate) fn user_line(
    uuid: &str,
    text: &str,
    images: &[Image<'_>],
    priority: Option<&str>,
) -> String {
    let mut line = json!({
        "type": "user",
        "uuid": uuid,
        "message": { "role": "user", "content": content(text, images) },
        "parent_tool_use_id": null,
    });
    if let Some(priority) = priority {
        line["priority"] = Value::from(priority);
    }
    line.to_string()
}

fn control_response(request_id: &str, response: Value) -> String {
    json!({
        "type": "control_response",
        "response": { "subtype": "success", "request_id": request_id, "response": response },
    })
    .to_string()
}

/// Unsupported control requests are answered with an error so the CLI never waits on them.
pub(crate) fn control_error(request_id: &str, message: &str) -> String {
    json!({
        "type": "control_response",
        "response": { "subtype": "error", "request_id": request_id, "error": message },
    })
    .to_string()
}

pub(crate) fn allow_line(request_id: &str) -> String {
    control_response(request_id, json!({ "behavior": "allow" }))
}

pub(crate) fn deny_line(request_id: &str, message: &str) -> String {
    control_response(
        request_id,
        json!({ "behavior": "deny", "message": message }),
    )
}

/// `AskUserQuestion` answered: the original input with `answers` keyed by question text,
/// a string for a single-select question and an array for a multi-select one. Free text
/// is the answer when nothing was selected, and is added to the selection otherwise.
pub(crate) fn answers_line(
    request_id: &str,
    input: &Value,
    questions: &[QuestionItem],
    answers: &[QuestionAnswer],
) -> String {
    let mut updated = match input {
        Value::Object(map) => map.clone(),
        _ => serde_json::Map::new(),
    };
    let mut by_question = serde_json::Map::new();
    for (index, question) in questions.iter().enumerate() {
        let mut labels = answers
            .get(index)
            .map(|answer| answer.selected.clone())
            .unwrap_or_default();
        if let Some(text) = answers
            .get(index)
            .and_then(|answer| answer.text.as_deref())
            .map(str::trim)
            .filter(|text| !text.is_empty())
        {
            labels.push(text.to_owned());
        }
        let value = if question.multi_select {
            Value::Array(labels.into_iter().map(Value::from).collect())
        } else {
            Value::from(labels.into_iter().next().unwrap_or_default())
        };
        by_question.insert(question.question.clone(), value);
    }
    updated.insert("answers".into(), Value::Object(by_question));
    control_response(
        request_id,
        json!({ "behavior": "allow", "updatedInput": Value::Object(updated) }),
    )
}

/// The ACP tool kind of a Claude Code tool, and the paths its input names. An unknown
/// tool (a subagent `Task`, an MCP tool) is `other`, so a permission policy never mistakes
/// it for an edit.
pub(crate) fn tool_kind_and_paths(tool: &str, input: &Value) -> (String, Vec<std::path::PathBuf>) {
    let kind = match tool {
        "Read" | "NotebookRead" | "LS" => "read",
        "Glob" | "Grep" => "search",
        "Edit" | "MultiEdit" | "Write" | "NotebookEdit" => "edit",
        "Bash" | "BashOutput" | "KillShell" => "execute",
        "WebFetch" | "WebSearch" => "fetch",
        "TodoWrite" | "ExitPlanMode" => "think",
        _ => "other",
    };
    let paths = ["file_path", "notebook_path", "path"]
        .iter()
        .filter_map(|key| input.get(*key).and_then(Value::as_str))
        .filter(|path| !path.is_empty())
        .map(std::path::PathBuf::from)
        .collect();
    (kind.to_owned(), paths)
}

/// Why a question request cannot be answered, if it cannot: Claude reads the answers by
/// question text, so an empty list, an empty text or a repeated text has no unambiguous
/// answer.
pub(crate) fn unanswerable_questions(questions: &[QuestionItem]) -> Option<&'static str> {
    if questions.is_empty() {
        return Some("The question request contained no questions.");
    }
    let mut seen = std::collections::HashSet::new();
    for question in questions {
        if question.question.trim().is_empty() {
            return Some("A question had no text, so it cannot be answered.");
        }
        if !seen.insert(question.question.as_str()) {
            return Some("Two questions had the same text, so the answers cannot be told apart.");
        }
    }
    None
}

/// The questions of an `AskUserQuestion` input. Option objects give their `label`
/// (descriptions are kept), plain strings are labels.
pub(crate) fn parse_questions(input: &Value) -> Vec<QuestionItem> {
    let Some(list) = input.get("questions").and_then(Value::as_array) else {
        return Vec::new();
    };
    list.iter()
        .map(|question| {
            let field = |keys: [&str; 2]| {
                keys.iter()
                    .find_map(|key| question.get(*key).and_then(Value::as_str))
            };
            QuestionItem {
                header: field(["header", "title"]).unwrap_or_default().to_owned(),
                question: field(["question", "prompt"]).unwrap_or_default().to_owned(),
                multi_select: ["multiSelect", "multi_select"]
                    .iter()
                    .find_map(|key| question.get(*key).and_then(Value::as_bool))
                    .unwrap_or(false),
                options: question
                    .get("options")
                    .and_then(Value::as_array)
                    .map(|options| {
                        options
                            .iter()
                            .map(|option| match option {
                                Value::String(label) => crate::driver::QuestionOption {
                                    label: label.clone(),
                                    description: String::new(),
                                },
                                other => crate::driver::QuestionOption {
                                    label: other
                                        .get("label")
                                        .or_else(|| other.get("value"))
                                        .and_then(Value::as_str)
                                        .unwrap_or_default()
                                        .to_owned(),
                                    description: other
                                        .get("description")
                                        .and_then(Value::as_str)
                                        .unwrap_or_default()
                                        .to_owned(),
                                },
                            })
                            .collect()
                    })
                    .unwrap_or_default(),
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn prompts_are_plain_strings_and_images_come_before_the_text() {
        let line: Value = serde_json::from_str(&user_line("u1", "hi", &[], None)).unwrap();
        assert_eq!(line["message"]["content"], "hi");
        assert_eq!(line["uuid"], "u1");
        assert!(line.get("priority").is_none());
        let line: Value = serde_json::from_str(&user_line(
            "u2",
            "look",
            &[Image { data: "AAAA" }],
            Some("now"),
        ))
        .unwrap();
        assert_eq!(line["priority"], "now");
        let blocks = line["message"]["content"].as_array().unwrap();
        assert_eq!(blocks[0]["source"]["data"], "AAAA");
        assert_eq!(blocks[1]["text"], "look");
    }

    #[test]
    fn questions_are_answered_by_question_text_inside_the_original_input() {
        let input = json!({"questions": [
            {"header": "Choice", "question": "Pick one", "options": ["A", {"label": "B", "description": "second"}]},
            {"question": "Which?", "options": [{"label": "X"}, {"label": "Y"}], "multiSelect": true},
            {"question": "Name it"},
        ]});
        let questions = parse_questions(&input);
        assert_eq!(questions.len(), 3);
        assert_eq!(questions[0].options[1].label, "B");
        assert_eq!(questions[0].options[1].description, "second");
        assert!(questions[1].multi_select);
        assert!(questions[2].options.is_empty());
        let answers = vec![
            QuestionAnswer {
                selected: vec!["B".into()],
                text: None,
            },
            QuestionAnswer {
                selected: vec!["X".into(), "Y".into()],
                text: None,
            },
            QuestionAnswer {
                selected: vec![],
                text: Some(" a name ".into()),
            },
        ];
        let line: Value =
            serde_json::from_str(&answers_line("r1", &input, &questions, &answers)).unwrap();
        let response = &line["response"]["response"];
        assert_eq!(response["behavior"], "allow");
        let updated = &response["updatedInput"];
        assert_eq!(updated["questions"], input["questions"]);
        assert_eq!(updated["answers"]["Pick one"], "B");
        assert_eq!(updated["answers"]["Which?"], json!(["X", "Y"]));
        assert_eq!(updated["answers"]["Name it"], "a name");
    }

    #[test]
    fn a_question_request_that_cannot_be_answered_unambiguously_is_declined() {
        assert!(unanswerable_questions(&parse_questions(&json!({"questions": []}))).is_some());
        assert!(unanswerable_questions(&parse_questions(&json!({}))).is_some());
        assert!(
            unanswerable_questions(&parse_questions(&json!({"questions": [{"header": "h"}]})))
                .is_some()
        );
        let twice = json!({"questions": [{"question": "Same?"}, {"question": "Same?"}]});
        assert!(unanswerable_questions(&parse_questions(&twice)).is_some());
        let fine = json!({"questions": [{"question": "One?"}, {"question": "Two?"}]});
        assert!(unanswerable_questions(&parse_questions(&fine)).is_none());
    }

    #[test]
    fn deny_carries_the_message_and_allow_needs_no_input() {
        let deny: Value = serde_json::from_str(&deny_line("r", "no")).unwrap();
        assert_eq!(deny["response"]["request_id"], "r");
        assert_eq!(deny["response"]["response"]["behavior"], "deny");
        assert_eq!(deny["response"]["response"]["message"], "no");
        let allow: Value = serde_json::from_str(&allow_line("r")).unwrap();
        assert_eq!(allow["response"]["response"], json!({"behavior": "allow"}));
        let interrupt: Value = serde_json::from_str(&interrupt_line("i")).unwrap();
        assert_eq!(interrupt["request"]["cancel_queued"], true);
    }

    #[test]
    fn tools_map_to_acp_kinds_and_the_paths_their_input_names() {
        let (kind, paths) = tool_kind_and_paths("Write", &json!({"file_path": "src/lib.rs"}));
        assert_eq!(kind, "edit");
        assert_eq!(paths, [std::path::PathBuf::from("src/lib.rs")]);
        let (kind, paths) = tool_kind_and_paths("Bash", &json!({"command": "cargo check"}));
        assert_eq!((kind.as_str(), paths.len()), ("execute", 0));
        assert_eq!(
            tool_kind_and_paths("Grep", &json!({"path": "."})).0,
            "search"
        );
        assert_eq!(tool_kind_and_paths("WebFetch", &json!({})).0, "fetch");
        // A subagent or an MCP tool can do anything: never an edit.
        assert_eq!(tool_kind_and_paths("Task", &json!({})).0, "other");
        assert_eq!(
            tool_kind_and_paths("mcp__x__write", &json!({"path": "a"})).0,
            "other"
        );
    }
}
