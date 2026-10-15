//! The broker client: round trips, timeouts and capability file checks.

use super::*;

#[test]
fn client_roundtrip_and_error_mapping() {
    let h = harness();
    let task = task();
    let cap = h.grant(&task);
    let mut client = BrokerClient::connect(&cap.file).unwrap();
    let value = client.call("render_frame", json!({"frame": 3})).unwrap();
    assert_eq!(value["artifacts"][0]["id"], "art-0123456789abcdef");
    let error = client
        .call("render_frame", json!({"frame": 999}))
        .unwrap_err();
    assert_eq!(error.code, ToolErrorCode::NotFound);
    let error = client
        .call("render_frame", json!({"frame": -1}))
        .unwrap_err();
    assert_eq!(error.code, ToolErrorCode::InvalidParams);
    let error = client.call("nope", json!({})).unwrap_err();
    assert_eq!(error.code, ToolErrorCode::MethodNotFound);
    // The client refuses an oversized request itself and stays usable.
    let huge = json!({"project": "p".repeat(MAX_REQUEST_BYTES)});
    assert_eq!(
        client.call("timeline", huge).unwrap_err().code,
        ToolErrorCode::TooLarge
    );
    assert!(client.call("timeline", json!({})).is_ok());

    let text = format!("{client:?}");
    assert!(!text.contains(&cap.secret));

    h.liveness.kill(&task);
    let failure = client.try_call("timeline", json!({})).unwrap_err();
    assert!(matches!(&failure, CallFailure::Tool(e) if e.code == ToolErrorCode::StaleTask));
    let failure = client.try_call("timeline", json!({})).unwrap_err();
    assert!(
        matches!(failure, CallFailure::Transport(_)),
        "a closed connection is a transport failure"
    );
    assert_eq!(
        client.call("timeline", json!({})).unwrap_err().code,
        ToolErrorCode::Unavailable
    );
    match BrokerClient::connect(&cap.file) {
        Err(ClientError::Rejected(error)) => assert_eq!(error.code, ToolErrorCode::StaleTask),
        other => panic!("expected a rejection, got {other:?}"),
    }
}

#[test]
fn client_call_timeout_marks_the_connection_broken() {
    let h = harness();
    let gate = Gate::new();
    {
        let gate = gate.clone();
        h.backend.set(move |_, request, cancelled| {
            gate.pass(cancelled);
            Ok(plain_reply(request))
        });
    }
    let cap = h.grant(&task());
    let mut client = BrokerClient::connect(&cap.file)
        .unwrap()
        .with_call_timeout(Duration::from_millis(300));
    let started = Instant::now();
    let error = client.call("timeline", json!({})).unwrap_err();
    assert_eq!(error.code, ToolErrorCode::Unavailable);
    assert!(started.elapsed() < Duration::from_secs(10));
    // A late reply must never be taken for the answer to the next call.
    gate.open();
    assert_eq!(
        client.call("timeline", json!({})).unwrap_err().code,
        ToolErrorCode::Unavailable
    );
}

#[test]
fn client_refuses_unsafe_or_malformed_capability_files() {
    let h = harness();
    let cap = h.grant(&task());
    let good = fs::read(&cap.file).unwrap();
    let dir = short_tempdir();

    let message = |result: Result<BrokerClient, ClientError>| match result {
        Err(ClientError::CapabilityFile(message)) => message,
        other => panic!("expected CapabilityFile, got {other:?}"),
    };

    let link = dir.path().join("link.json");
    symlink(&cap.file, &link).unwrap();
    message(BrokerClient::connect(&link));

    let readable = private_copy(dir.path(), "group.json", &good, 0o640);
    assert!(message(BrokerClient::connect(&readable)).contains("0600"));
    let world = private_copy(dir.path(), "world.json", &good, 0o604);
    message(BrokerClient::connect(&world));

    let mut bloated = good.clone();
    bloated.resize(5000, b' ');
    let bloated = private_copy(dir.path(), "big.json", &bloated, 0o600);
    assert!(message(BrokerClient::connect(&bloated)).contains("4096"));

    let garbage = private_copy(dir.path(), "garbage.json", b"{\"secret\": 12", 0o600);
    message(BrokerClient::connect(&garbage));
    let mut wrong_version: Value = serde_json::from_slice(&good).unwrap();
    wrong_version["version"] = json!(2);
    let wrong_version = private_copy(
        dir.path(),
        "v2.json",
        wrong_version.to_string().as_bytes(),
        0o600,
    );
    message(BrokerClient::connect(&wrong_version));
    let mut short_secret: Value = serde_json::from_slice(&good).unwrap();
    short_secret["secret"] = json!("abc");
    let short_secret = private_copy(
        dir.path(),
        "short.json",
        short_secret.to_string().as_bytes(),
        0o600,
    );
    let error = message(BrokerClient::connect(&short_secret));
    assert!(!error.contains(&cap.secret));
    message(BrokerClient::connect(&dir.path().join("missing.json")));
    message(BrokerClient::connect(dir.path()));

    // Owned by someone else (only checkable when the test runs as root).
    let foreign = private_copy(dir.path(), "foreign.json", &good, 0o600);
    if std::os::unix::fs::chown(&foreign, Some(65534), None).is_ok() {
        assert!(message(BrokerClient::connect(&foreign)).contains("owned"));
    }

    // A well-formed file naming a socket nobody serves is "unreachable", not "unusable".
    let mut dangling: Value = serde_json::from_slice(&good).unwrap();
    dangling["socket"] = json!(dir.path().join("nobody-home.sock").to_str().unwrap());
    let dangling = private_copy(
        dir.path(),
        "dangling.json",
        dangling.to_string().as_bytes(),
        0o600,
    );
    assert!(matches!(
        BrokerClient::connect(&dangling),
        Err(ClientError::Unreachable(_))
    ));
    let mut relative: Value = serde_json::from_slice(&good).unwrap();
    relative["socket"] = json!("relative.sock");
    let relative = private_copy(
        dir.path(),
        "relative.json",
        relative.to_string().as_bytes(),
        0o600,
    );
    message(BrokerClient::connect(&relative));

    // The unmodified original still works.
    assert!(BrokerClient::connect(&cap.file).is_ok());
}
