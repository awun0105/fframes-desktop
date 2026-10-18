//! An agent turn uses a project tool through the route a real agent gets on every
//! platform: the MCP server named in `session/new`, which reaches the app's tool broker
//! over a unix socket, or a named pipe on Windows.
#![allow(dead_code, unused_imports)]
include!("../../crates/studio-engine/tests/support/workflow_world.rs");

#[test]
fn an_agent_turn_calls_a_project_tool_through_its_mcp_server() {
    let w = World::new();
    let mut script = good("used a project tool");
    script["mcp_call"] = json!({"name": "project_context"});
    w.submit(&script);
    let task = w
        .wait("the turn to finish", |snapshot| {
            snapshot.task.as_ref().is_some_and(|task| {
                matches!(
                    task.phase,
                    TaskPhase::Accepted | TaskPhase::AwaitingReview | TaskPhase::Failed
                )
            })
        })
        .task
        .clone()
        .unwrap();
    assert_ne!(task.phase, TaskPhase::Failed, "{:?}", task.error);

    let reply = w.evidence("mcp-call.json");
    let reply = reply.first().expect("the agent recorded the tool reply");
    let result = &reply["result"];
    assert_eq!(result["isError"], json!(false), "{reply}");
    // The broker answered with this task's project, bound to its draft.
    let context = &result["structuredContent"];
    assert_eq!(
        context["result"]["project"]["entry"]["package"],
        json!("studio-video"),
        "{reply}"
    );
    assert_eq!(
        context["revision"]["label"],
        json!("draft_snapshot"),
        "{reply}"
    );
    assert_eq!(context["result"]["task"]["id"], json!(task.id.0), "{reply}");

    // The capability lived exactly as long as the task.
    let s = w.snap();
    assert_eq!(s.resources.broker_grants, 0);
    assert!(s.mcp.capability_file.is_none());
    w.assert_clean();
}
