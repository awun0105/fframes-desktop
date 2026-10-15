use super::*;
use windows_sys::Win32::Security::Authorization::SE_KERNEL_OBJECT;
use windows_sys::Win32::{
    Security::{CreateRestrictedToken, SID_AND_ATTRIBUTES, TOKEN_ASSIGN_PRIMARY, TOKEN_DUPLICATE},
    System::{Pipes::GetNamedPipeInfo, Threading::GetCurrentProcess},
};

fn own_token(access: u32) -> OwnedHandle {
    let mut token: HANDLE = ptr::null_mut();
    // SAFETY: this process's pseudo handle and a valid out-parameter.
    assert_ne!(
        unsafe { OpenProcessToken(GetCurrentProcess(), access, &mut token) },
        0
    );
    owned(token).unwrap()
}

fn listening_handle(listener: &Listener) -> HANDLE {
    listener
        .listening
        .lock()
        .unwrap()
        .as_ref()
        .expect("an instance listens")
        .handle()
}

#[test]
fn the_pipe_grants_only_the_current_user_and_refuses_remote_clients() {
    let temp = tempfile::tempdir().unwrap();
    let runtime = prepare_runtime_dir(&temp.path().join("rt")).unwrap();
    let listener = Listener::bind(&runtime).unwrap();
    assert!(
        listener
            .endpoint()
            .to_str()
            .unwrap()
            .starts_with(r"\\.\pipe\fframes-studio-")
    );
    let handle = listening_handle(&listener);
    assert_eq!(not_private(handle, SE_KERNEL_OBJECT), None);
    let mut flags = 0u32;
    // SAFETY: a pipe handle and a valid out-parameter.
    assert_ne!(
        unsafe {
            GetNamedPipeInfo(
                handle,
                &mut flags,
                ptr::null_mut(),
                ptr::null_mut(),
                ptr::null_mut(),
            )
        },
        0
    );
    assert_ne!(
        flags & PIPE_REJECT_REMOTE_CLIENTS,
        0,
        "the pipe must refuse remote clients (flags {flags:#x})"
    );
}

#[test]
fn the_current_users_token_passes_and_a_restricted_one_does_not() {
    let token = own_token(TOKEN_QUERY | TOKEN_DUPLICATE | TOKEN_ASSIGN_PRIMARY);
    assert!(token_is_current_user(token.as_raw_handle()));
    let user = current_user().unwrap().to_vec();
    let restricting = SID_AND_ATTRIBUTES {
        Sid: user.as_ptr() as PSID,
        Attributes: 0,
    };
    let mut restricted: HANDLE = ptr::null_mut();
    // SAFETY: a token opened with TOKEN_DUPLICATE and one valid restricting SID.
    assert_ne!(
        unsafe {
            CreateRestrictedToken(
                token.as_raw_handle(),
                0,
                0,
                ptr::null(),
                0,
                ptr::null(),
                1,
                &restricting,
                &mut restricted,
            )
        },
        0,
        "{}",
        last_error()
    );
    let restricted = owned(restricted).unwrap();
    assert!(!token_is_current_user(restricted.as_raw_handle()));
}

#[test]
fn the_runtime_dir_and_capability_files_are_private_and_shared_ones_are_refused() {
    let temp = tempfile::tempdir().unwrap();
    let runtime = prepare_runtime_dir(&temp.path().join("rt")).unwrap();
    assert!(!runtime.to_str().unwrap().starts_with(r"\\?\"));
    // Already present and private: accepted again.
    prepare_runtime_dir(&runtime).unwrap();

    let file = runtime.join("cap-test.json");
    create_private_file(&file, b"{}\n").unwrap();
    let mut text = String::new();
    open_private_file(&file)
        .unwrap()
        .read_to_string(&mut text)
        .unwrap();
    assert_eq!(text, "{}\n");
    assert!(
        create_private_file(&file, b"again").is_err(),
        "never replaced"
    );

    // The temp directory's inherited permissions grant administrators and SYSTEM.
    let shared_file = temp.path().join("shared.json");
    fs::write(&shared_file, b"{}").unwrap();
    let error = open_private_file(&shared_file).unwrap_err();
    assert!(
        error.contains("grants access to") || error.contains("is not owned"),
        "{error}"
    );

    let shared_dir = temp.path().join("shared");
    fs::create_dir(&shared_dir).unwrap();
    let error = prepare_runtime_dir(&shared_dir).unwrap_err();
    assert!(
        matches!(&error, BrokerError::InsecureRuntimeDir { reason, .. } if reason.contains("grants access to") || reason.contains("is not owned")),
        "{error}"
    );

    let junction = temp.path().join("junction");
    let made = std::process::Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(&junction)
        .arg(&runtime)
        .output()
        .unwrap();
    assert!(made.status.success(), "{made:?}");
    let error = prepare_runtime_dir(&junction).unwrap_err();
    assert!(
        matches!(&error, BrokerError::InsecureRuntimeDir { reason, .. } if reason.contains("junction")),
        "{error}"
    );
}

#[test]
fn a_second_listener_in_the_same_runtime_dir_is_refused() {
    let temp = tempfile::tempdir().unwrap();
    let runtime = prepare_runtime_dir(&temp.path().join("rt")).unwrap();
    let first = Listener::bind(&runtime).unwrap();
    assert!(matches!(
        Listener::bind(&runtime),
        Err(BrokerError::AlreadyRunning(_))
    ));
    drop(first);
    assert!(!runtime.join("broker.lock").exists(), "deleted on close");
    Listener::bind(&runtime).unwrap();
}

#[test]
fn a_client_round_trips_and_a_closed_server_end_reads_as_the_end_of_the_stream() {
    let temp = tempfile::tempdir().unwrap();
    let runtime = prepare_runtime_dir(&temp.path().join("rt")).unwrap();
    let listener = Listener::bind(&runtime).unwrap();
    let endpoint = listener.endpoint().to_path_buf();
    let client = thread::spawn(move || {
        let mut stream = connect(&endpoint, Duration::from_secs(5)).unwrap();
        stream
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        stream.write_all(b"ping\n").unwrap();
        let mut reply = Vec::new();
        stream.read_to_end(&mut reply).unwrap();
        reply
    });
    let mut server = loop {
        match listener.accept(Duration::from_millis(100)) {
            Accepted::Peer(stream) => break stream,
            Accepted::Refused => panic!("our own client was refused"),
            Accepted::Idle => {}
        }
    };
    server
        .set_read_timeout(Some(Duration::from_secs(5)))
        .unwrap();
    let mut line = [0u8; 5];
    server.read_exact(&mut line).unwrap();
    assert_eq!(&line, b"ping\n");
    assert!(verify_peer(&server), "the client runs as the current user");
    server.write_all(b"pong\n").unwrap();
    finish(server);
    assert_eq!(
        client.join().unwrap(),
        b"pong\n",
        "replies survive the close"
    );
}

#[test]
fn a_read_times_out_and_close_from_another_thread_ends_it() {
    let temp = tempfile::tempdir().unwrap();
    let runtime = prepare_runtime_dir(&temp.path().join("rt")).unwrap();
    let listener = Listener::bind(&runtime).unwrap();
    let mut client = connect(listener.endpoint(), Duration::from_secs(5)).unwrap();
    let server = loop {
        if let Accepted::Peer(stream) = listener.accept(Duration::from_millis(100)) {
            break stream;
        }
    };
    client
        .set_read_timeout(Some(Duration::from_millis(50)))
        .unwrap();
    let started = Instant::now();
    let error = client.read(&mut [0u8; 8]).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::TimedOut);
    assert!(started.elapsed() < Duration::from_secs(2));

    let mut reader = server.try_clone().unwrap();
    let blocked = thread::spawn(move || reader.read(&mut [0u8; 8]));
    thread::sleep(Duration::from_millis(100));
    close(&server);
    assert_eq!(
        blocked.join().unwrap().unwrap(),
        0,
        "a closed stream reads as the end"
    );
    client.set_read_timeout(None).unwrap();
    assert_eq!(
        client.read(&mut [0u8; 8]).unwrap(),
        0,
        "the client sees the disconnect"
    );
}

#[test]
fn connect_refuses_anything_but_a_local_pipe() {
    let error = connect(Path::new(r"C:\Windows\win.ini"), Duration::from_secs(1)).unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::InvalidInput);
    let error = connect(
        Path::new(r"\\.\pipe\fframes-studio-does-not-exist"),
        Duration::from_secs(1),
    )
    .unwrap_err();
    assert_eq!(error.kind(), io::ErrorKind::NotFound);
}
