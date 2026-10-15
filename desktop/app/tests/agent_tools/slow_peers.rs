//! Slow-peer adversaries: dripped hellos, oversized lines and dripping replies against the deadlines.

use super::*;

/// Connects, optionally authenticates, then sends one byte every `interval` (never a
/// newline) until the broker closes the connection, `stop` is set or 30 s pass. Returns
/// how long the connection stayed open after the first byte.
fn spawn_dripper(
    socket: PathBuf,
    auth: Option<(String, String)>,
    interval: Duration,
    stop: Arc<AtomicBool>,
) -> thread::JoinHandle<Duration> {
    thread::spawn(move || {
        let mut wire = Wire::connect(&socket);
        if let Some((capability, secret)) = auth {
            assert_eq!(wire.hello(&capability, &secret), json!({"ok": true}));
        }
        let begun = Instant::now();
        while !stop.load(Ordering::SeqCst) && begun.elapsed() < Duration::from_secs(30) {
            if wire.stream.write_all(b" ").is_err() {
                break;
            }
            thread::sleep(interval);
        }
        begun.elapsed()
    })
}

#[test]
fn a_hello_dripped_without_a_newline_is_closed_at_the_deadline_and_frees_capacity() {
    let h = harness();
    let cap = h.grant(&task());
    let stop = Arc::new(AtomicBool::new(false));
    let drippers: Vec<_> = (0..8)
        .map(|_| {
            spawn_dripper(
                cap.socket.clone(),
                None,
                Duration::from_millis(30),
                stop.clone(),
            )
        })
        .collect();
    wait_until("8 dripping connections", || {
        h.broker.stats().live_connections == 8
    });
    let mut refused = Wire::connect(&cap.socket);
    assert_eq!(
        error_code(&refused.recv().expect("a refusal line")),
        "busy",
        "the broker is full while the drippers hold their slots"
    );
    for dripper in drippers {
        let open = dripper.join().unwrap();
        assert!(
            open >= Duration::from_millis(4500),
            "closed too early: {open:?}"
        );
        assert!(
            open < Duration::from_secs(7),
            "a dripping hello outlived the hello deadline: {open:?}"
        );
    }
    wait_until("every dripper's slot freed", || {
        h.broker.stats().live_connections == 0
    });
    assert_eq!(h.broker.stats().rejected_auth, 8);
    let mut wire = cap.wire();
    assert!(wire.call(1, "timeline", json!({})).get("result").is_some());
}

#[test]
fn an_oversized_hello_is_rejected_at_the_limit_before_any_newline() {
    let h = harness();
    let cap = h.grant(&task());
    let mut wire = Wire::connect(&cap.socket);
    let started = Instant::now();
    // Exactly the limit, no newline: still waiting for the rest of the line.
    for _ in 0..8 {
        wire.send_raw(&" ".repeat(512));
        thread::sleep(Duration::from_millis(10));
    }
    wire.stream
        .set_read_timeout(Some(Duration::from_millis(300)))
        .unwrap();
    let mut probe = [0u8; 1];
    match wire.stream.read(&mut probe) {
        Err(error)
            if matches!(
                error.kind(),
                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
            ) => {}
        other => panic!("the connection must stay open at exactly 4096 bytes: {other:?}"),
    }
    // One byte over: rejected without waiting for a newline or the deadline.
    wire.send_raw(" ");
    wire.stream.set_read_timeout(Some(WAIT)).unwrap();
    assert!(
        wire.recv().is_none(),
        "the oversized hello closes the connection"
    );
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "rejected only at the deadline: {:?}",
        started.elapsed()
    );
    wait_until("the slot is freed", || {
        h.broker.stats().live_connections == 0
    });
    assert_eq!(h.broker.stats().rejected_auth, 1);
    assert_eq!(h.backend.call_count(), 0);
}

#[test]
fn a_client_call_against_a_dripping_reply_fails_at_its_deadline() {
    let dir = short_tempdir();
    let socket = dir.path().join("fake.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let capability = private_copy(
        dir.path(),
        "cap-fake.json",
        json!({
            "version": 1,
            "socket": socket.to_str().unwrap(),
            "capability": "fake",
            "secret": "0".repeat(64),
        })
        .to_string()
        .as_bytes(),
        0o600,
    );
    let server = thread::spawn(move || {
        let (stream, _) = listener.accept().unwrap();
        let mut reader = BufReader::new(stream.try_clone().unwrap());
        let mut writer = stream;
        let mut line = String::new();
        reader.read_line(&mut line).unwrap(); // hello
        writer.write_all(b"{\"ok\":true}\n").unwrap();
        line.clear();
        reader.read_line(&mut line).unwrap(); // the request
        // A reply that never completes: one byte per 20 ms, no newline.
        let begun = Instant::now();
        while begun.elapsed() < Duration::from_secs(30) {
            if writer.write_all(b"x").is_err() {
                break;
            }
            thread::sleep(Duration::from_millis(20));
        }
    });
    let timeout = Duration::from_millis(700);
    let mut client = BrokerClient::connect(&capability)
        .unwrap()
        .with_call_timeout(timeout);
    let started = Instant::now();
    let failure = client.try_call("timeline", json!({})).unwrap_err();
    let took = started.elapsed();
    assert!(
        matches!(&failure, CallFailure::Transport(message) if message.contains("timed out")),
        "{failure:?}"
    );
    assert!(took >= timeout, "returned before the deadline: {took:?}");
    assert!(
        took < timeout + Duration::from_millis(400),
        "the dripping reply held the call past its deadline: {took:?}"
    );
    assert!(matches!(
        client.try_call("timeline", json!({})).unwrap_err(),
        CallFailure::Transport(message) if message.contains("closed")
    ));
    drop(client);
    server.join().unwrap();
}

#[test]
fn shutdown_with_live_drippers_is_bounded_and_peers_see_the_close() {
    let h = harness();
    let cap = h.grant(&task());
    let stop = Arc::new(AtomicBool::new(false));
    let interval = Duration::from_millis(20);
    let mut drippers: Vec<_> = (0..5)
        .map(|_| spawn_dripper(cap.socket.clone(), None, interval, stop.clone()))
        .collect();
    // Authenticated connections that drip a request line forever.
    drippers.extend((0..2).map(|_| {
        spawn_dripper(
            cap.socket.clone(),
            Some((cap.capability.clone(), cap.secret.clone())),
            interval,
            stop.clone(),
        )
    }));
    wait_until("7 dripping connections", || {
        h.broker.stats().live_connections == 7
    });
    thread::sleep(Duration::from_millis(200));
    let started = Instant::now();
    h.broker.shutdown();
    let took = started.elapsed();
    assert!(
        took < Duration::from_secs(2),
        "shutdown waited for dripping peers: {took:?}"
    );
    for dripper in drippers {
        let open = dripper.join().unwrap();
        assert!(
            open < Duration::from_secs(5),
            "a peer never saw the connection close: {open:?}"
        );
    }
    let stats = h.broker.stats();
    assert_eq!(stats.live_connections, 0);
    assert_eq!(stats.live_threads, 0);
    assert!(!h.broker.socket_path().exists());
}
