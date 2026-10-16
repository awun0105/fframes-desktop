//! Shared pumping for the real-CLI checks (`claude_real_cli.rs`, `codex_real_cli.rs`).

use std::time::{Duration, Instant};
use studio_harness::driver::{
    AgentDriver, AgentEventKind, MessageRole, PermissionReply, PromptOutcome, QuestionAnswer,
    QuestionPrompt, TurnOrigin,
};

/// A code word a steer asks for and a resumed session must remember.
pub const WORD: &str = "KESTREL-4417";

/// What happened until the agent was idle.
#[derive(Debug, Default)]
pub struct Record {
    pub text: String,
    pub outcomes: Vec<PromptOutcome>,
    pub asked: u32,
    pub started: Vec<TurnOrigin>,
    /// Background tasks still running when the first turn ended.
    pub tasks_at_first_end: u32,
    pub questions: Vec<QuestionPrompt>,
    /// The option labels sent back as answers.
    pub chosen: Vec<String>,
}

/// Pumps until the agent is idle: no turn runs, no background task runs and the agent is
/// not about to continue by itself, which is when the engine may capture. Every permission
/// request gets `choice`, every question its last option; `on_text` runs once, at the
/// first agent text.
pub fn run_until_idle(
    driver: &dyn AgentDriver,
    limit: Duration,
    choice: &str,
    on_text: impl FnOnce(),
) -> Record {
    let deadline = Instant::now() + limit;
    let mut on_text = Some(on_text);
    let mut record = Record::default();
    let (mut in_turn, mut tasks, mut continuing) = (true, 0, false);
    let mut idle_since: Option<Instant> = None;
    loop {
        assert!(
            Instant::now() < deadline,
            "the agent never went idle: {record:?}"
        );
        for event in driver.drain_events(64) {
            idle_since = None;
            match event.kind {
                AgentEventKind::PermissionRequested(prompt) => {
                    record.asked += 1;
                    eprintln!("asked: {}", prompt.title);
                    driver
                        .reply_permission(prompt.id, PermissionReply::Select(choice.into()))
                        .unwrap();
                }
                AgentEventKind::MessageDelta {
                    role: MessageRole::Agent,
                    text,
                } => {
                    if let Some(first) = on_text.take() {
                        first();
                    }
                    record.text.push_str(&text);
                }
                // Each question gets its last option, so a mix-up with the first is visible.
                AgentEventKind::Question(prompt) => {
                    let answers: Vec<QuestionAnswer> = prompt
                        .questions
                        .iter()
                        .map(|item| QuestionAnswer {
                            selected: item
                                .options
                                .last()
                                .map(|o| o.label.clone())
                                .into_iter()
                                .collect(),
                            text: None,
                        })
                        .collect();
                    record
                        .chosen
                        .extend(answers.iter().flat_map(|a| a.selected.clone()));
                    eprintln!("question with {} item(s)", prompt.questions.len());
                    driver.answer_question(prompt.id, answers).unwrap();
                    record.questions.push(prompt);
                }
                AgentEventKind::TurnStarted { origin, .. } => {
                    in_turn = true;
                    record.started.push(origin);
                }
                AgentEventKind::Background {
                    tasks: now,
                    continuing: next,
                } => (tasks, continuing) = (now, next),
                AgentEventKind::PromptFinished(outcome) => {
                    if record.outcomes.is_empty() {
                        record.tasks_at_first_end = tasks;
                    }
                    in_turn = false;
                    record.outcomes.push(outcome);
                }
                AgentEventKind::Failure(failure) => panic!("the session failed: {failure:?}"),
                AgentEventKind::Closed(closed) => panic!("the session closed: {closed:?}"),
                _ => {}
            }
        }
        if !in_turn && tasks == 0 && !continuing {
            // Stay quiet for a moment: a turn of the agent's own is announced right away.
            if idle_since.get_or_insert_with(Instant::now).elapsed() > Duration::from_secs(3) {
                return record;
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}
