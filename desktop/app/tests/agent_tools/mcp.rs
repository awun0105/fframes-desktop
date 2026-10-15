//! The MCP server: lifecycle, errors and verbatim forwarding.

use super::*;

#[test]
fn mcp_lifecycle_errors_and_lazy_connection() {
    let connects = Arc::new(AtomicUsize::new(0));
    let mut server = {
        let connects = connects.clone();
        McpServer::new(move || {
            connects.fetch_add(1, Ordering::SeqCst);
            Err(ToolError::new(ToolErrorCode::Unavailable, "no broker"))
        })
    };
    // Requests other than ping/initialize need initialize first.
    let early = rpc(
        &mut server,
        json!({"jsonrpc":"2.0","id":9,"method":"tools/list"}),
    );
    assert_eq!(early["error"]["code"], NOT_INITIALIZED);
    assert_eq!(early["id"], 9);
    assert_eq!(
        rpc(&mut server, json!({"jsonrpc":"2.0","id":8,"method":"ping"}))["result"],
        json!({})
    );

    let init = initialize(&mut server);
    assert_eq!(init["jsonrpc"], "2.0");
    assert_eq!(init["id"], 1);
    assert_eq!(init["result"]["protocolVersion"], MCP_PROTOCOL_VERSION);
    assert_eq!(
        init["result"]["capabilities"],
        json!({"tools": {"listChanged": false}})
    );
    assert_eq!(init["result"]["serverInfo"]["name"], "fframes-studio");
    assert!(init["result"]["serverInfo"]["version"].is_string());

    // A client asking for another version still gets ours.
    let other = rpc(
        &mut server,
        json!({"jsonrpc":"2.0","id":"abc","method":"initialize","params":{"protocolVersion":"1999-01-01"}}),
    );
    assert_eq!(other["id"], "abc");
    assert_eq!(other["result"]["protocolVersion"], MCP_PROTOCOL_VERSION);
    let bad = rpc(
        &mut server,
        json!({"jsonrpc":"2.0","id":3,"method":"initialize"}),
    );
    assert_eq!(bad["error"]["code"], INVALID_PARAMS);

    assert!(
        server
            .handle_line(r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#)
            .is_none()
    );
    assert!(
        server
            .handle_line(r#"{"jsonrpc":"2.0","method":"notifications/whatever","params":{}}"#)
            .is_none()
    );
    assert!(
        server
            .handle_line(r#"{"jsonrpc":"2.0","method":"nonexistent/notification"}"#)
            .is_none()
    );
    assert!(server.handle_line("").is_none());
    assert!(server.handle_line("   ").is_none());
    assert!(
        server
            .handle_line(r#"{"jsonrpc":"2.0","id":1,"result":{}}"#)
            .is_none(),
        "responses are ignored"
    );

    assert!(
        rpc(
            &mut server,
            json!({"jsonrpc":"2.0","id":4,"method":"tools/list"})
        )
        .get("result")
        .is_some()
    );
    assert_eq!(
        connects.load(Ordering::SeqCst),
        0,
        "no broker connection before the first tool call"
    );

    let unknown = rpc(
        &mut server,
        json!({"jsonrpc":"2.0","id":5,"method":"resources/list"}),
    );
    assert_eq!(unknown["error"]["code"], METHOD_NOT_FOUND);

    let parse = server.handle_line("{not json").unwrap();
    let parse: Value = serde_json::from_str(&parse).unwrap();
    assert_eq!(parse["error"]["code"], PARSE_ERROR);
    assert_eq!(parse["id"], Value::Null);
    for invalid in [
        "[]",
        "[{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"ping\"}]",
        "42",
        "\"text\"",
        "null",
        "{}",
    ] {
        let reply: Value = serde_json::from_str(&server.handle_line(invalid).unwrap()).unwrap();
        assert_eq!(reply["error"]["code"], INVALID_REQUEST, "{invalid}");
        assert_eq!(reply["id"], Value::Null);
    }
    let bad_id = rpc(
        &mut server,
        json!({"jsonrpc":"2.0","id":{"a":1},"method":"ping"}),
    );
    assert_eq!(bad_id["error"]["code"], INVALID_REQUEST);
    let bad_version = rpc(&mut server, json!({"jsonrpc":"1.0","id":6,"method":"ping"}));
    assert_eq!(bad_version["error"]["code"], INVALID_REQUEST);
    assert_eq!(bad_version["id"], 6);
    let bad_method = rpc(&mut server, json!({"jsonrpc":"2.0","id":7,"method":5}));
    assert_eq!(bad_method["error"]["code"], INVALID_REQUEST);

    let oversized = format!(
        r#"{{"jsonrpc":"2.0","id":1,"method":"ping","pad":"{}"}}"#,
        "x".repeat(MAX_MCP_LINE_BYTES)
    );
    let reply: Value = serde_json::from_str(&server.handle_line(&oversized).unwrap()).unwrap();
    assert_eq!(reply["error"]["code"], INVALID_REQUEST);

    for (id, params) in [
        (10, json!({})),
        (11, json!({"name": 5})),
        (12, json!({"name": "shell"})),
        (13, json!({"name": "timeline", "arguments": [1]})),
        (14, json!("timeline")),
    ] {
        let reply = rpc(
            &mut server,
            json!({"jsonrpc":"2.0","id":id,"method":"tools/call","params":params}),
        );
        assert_eq!(reply["error"]["code"], INVALID_PARAMS, "{params}");
    }
    assert_eq!(
        connects.load(Ordering::SeqCst),
        0,
        "rejected calls never connect"
    );

    // A tool failure is a successful JSON-RPC response with isError.
    let call = rpc(
        &mut server,
        json!({"jsonrpc":"2.0","id":15,"method":"tools/call","params":{"name":"timeline"}}),
    );
    assert!(call.get("error").is_none());
    assert_eq!(call["result"]["isError"], true);
    let text: Value =
        serde_json::from_str(call["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(text, json!({"code": "unavailable", "message": "no broker"}));
    assert_eq!(connects.load(Ordering::SeqCst), 1);
}

type Answer = dyn Fn(&str) -> Result<Value, ToolError> + Send + Sync;

struct RecordingCaller {
    calls: Arc<Mutex<Vec<(String, Value)>>>,
    answer: Arc<Answer>,
}

impl ToolCaller for RecordingCaller {
    fn call(&mut self, method: &str, params: Value) -> Result<Value, ToolError> {
        self.calls.lock().push((method.into(), params));
        (self.answer)(method)
    }
}

#[test]
fn mcp_forwards_calls_verbatim_and_maps_replies_and_errors() {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let connects = Arc::new(AtomicUsize::new(0));
    let reply_value = json!({"revision": {"id": REV_A, "label": "draft_snapshot", "validated": false},
        "result": {"zebra": 1, "alpha": [1, 2, {"deep": null}]}, "artifacts": []});
    let mut server = {
        let (calls, connects, reply_value) = (calls.clone(), connects.clone(), reply_value.clone());
        McpServer::new(move || {
            connects.fetch_add(1, Ordering::SeqCst);
            let reply_value = reply_value.clone();
            Ok(Box::new(RecordingCaller {
                calls: calls.clone(),
                answer: Arc::new(move |method| match method {
                    "render_frame" => Err(ToolError::new(ToolErrorCode::Unavailable, "link down")),
                    "build_status" => Err(ToolError::new(ToolErrorCode::Busy, "queue is full")),
                    _ => Ok(reply_value.clone()),
                }),
            }))
        })
    };
    initialize(&mut server);

    let arguments =
        json!({"frame": 3, "scale": 0.25, "project": "proj-1", "z": {"nested": [true]}});
    let call = rpc(
        &mut server,
        json!({"jsonrpc":"2.0","id":2,"method":"tools/call",
        "params":{"name":"timeline","arguments":arguments.clone()}}),
    );
    assert_eq!(call["result"]["isError"], false);
    assert_eq!(call["result"]["structuredContent"], reply_value);
    let content = &call["result"]["content"];
    assert_eq!(content.as_array().unwrap().len(), 1);
    assert_eq!(content[0]["type"], "text");
    assert_eq!(
        content[0]["text"],
        serde_json::to_string(&reply_value).unwrap()
    );
    assert_eq!(
        calls.lock().last().unwrap(),
        &("timeline".to_string(), arguments),
        "method and arguments are forwarded verbatim"
    );

    // Missing arguments become an empty object, exactly like the CLI.
    rpc(
        &mut server,
        json!({"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"project_context"}}),
    );
    assert_eq!(
        calls.lock().last().unwrap(),
        &("project_context".to_string(), json!({}))
    );

    // A tool error: isError content, no structuredContent, not a JSON-RPC error.
    let busy = rpc(
        &mut server,
        json!({"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"build_status"}}),
    );
    assert!(busy.get("error").is_none());
    assert_eq!(busy["result"]["isError"], true);
    assert!(busy["result"].get("structuredContent").is_none());
    let text: Value =
        serde_json::from_str(busy["result"]["content"][0]["text"].as_str().unwrap()).unwrap();
    assert_eq!(text, json!({"code": "busy", "message": "queue is full"}));
    assert_eq!(
        connects.load(Ordering::SeqCst),
        1,
        "a tool error keeps the connection"
    );

    // A transport failure drops the connection; the next call reconnects.
    let down = rpc(
        &mut server,
        json!({"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"render_frame","arguments":{"frame":1}}}),
    );
    assert_eq!(down["result"]["isError"], true);
    rpc(
        &mut server,
        json!({"jsonrpc":"2.0","id":6,"method":"tools/call","params":{"name":"timeline"}}),
    );
    assert_eq!(connects.load(Ordering::SeqCst), 2);
    assert!(!server.drain_diagnostics().is_empty());
}
