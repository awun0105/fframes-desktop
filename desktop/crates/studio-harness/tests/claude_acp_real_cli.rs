//! Live qualification of the pinned, Studio-patched Claude ACP adapter.
//! Set FFRAMES_NODE and FFRAMES_NPM to absolute paths, ACP_ADAPTER_HOME to an
//! isolated install cache. Requires signed-in Claude; downloads npm and calls a model.
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    time::{Duration, Instant},
};

fn files(root: &Path) -> BTreeMap<String, Vec<u8>> {
    fn visit(root: &Path, dir: &Path, result: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            assert!(!entry.file_type().unwrap().is_symlink());
            if path.is_dir() {
                visit(root, &path, result);
            } else {
                result.insert(
                    path.strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/"),
                    fs::read(path).unwrap(),
                );
            }
        }
    }
    let mut result = BTreeMap::new();
    visit(root, root, &mut result);
    result
}
#[test]
#[ignore = "requires Node/npm, npm network and signed-in Claude; makes model calls"]
fn pinned_claude_acp_permissions_edit_and_authoritative_completion() {
    use studio_bootstrap::ProcessTreeManager;
    use studio_harness::{
        AdapterConfig,
        discovery::{AdapterLaunch, ExecutableSearch},
        driver::{
            AgentEventKind, AgentWire, DriverConfig, DriverLimits, DriverMode, McpStdioSupport,
            OwnershipSource, PermissionReply, start_driver,
        },
    };
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("project");
    let out = temporary.path().join("evidence");
    fs::create_dir_all(root.join("src")).unwrap();
    fs::create_dir_all(&out).unwrap();
    fs::write(
        root.join("src/lib.rs"),
        "pub const TITLE: &str = \"Your video starts here\";\n",
    )
    .unwrap();
    let before = files(&root);
    let manager = ProcessTreeManager::new();
    let installed = studio_harness::adapter_install::install_adapter(
        &studio_harness::adapter_install::CLAUDE_ADAPTER,
        &PathBuf::from(std::env::var_os("ACP_ADAPTER_HOME").expect("ACP_ADAPTER_HOME"))
            .join("studio-managed"),
        &studio_harness::adapter_install::InstallTools {
            node: std::env::var_os("FFRAMES_NODE")
                .expect("FFRAMES_NODE absolute path")
                .into(),
            npm: std::env::var_os("FFRAMES_NPM")
                .expect("FFRAMES_NPM absolute path")
                .into(),
        },
        &manager,
        &std::sync::atomic::AtomicBool::new(false),
        Duration::from_secs(300),
    )
    .unwrap();
    let adapter = installed.entry;
    assert!(adapter.is_file());
    let status = std::process::Command::new(&installed.node)
        .arg(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("adapters/claude/advisor-result.test.mjs"),
        )
        .arg(adapter.parent().unwrap().join("acp-agent.js"))
        .status()
        .unwrap();
    assert!(
        status.success(),
        "actual pinned adapter conversion regression"
    );
    let config = AdapterConfig {
        executable: installed.node.to_string_lossy().into_owned(),
        args: vec![adapter.to_string_lossy().into_owned()],
        auth_env_names: vec![],
    };
    let launch = AdapterLaunch::resolve_with_env(
        &config,
        &ExecutableSearch {
            managed_dirs: vec![],
            gui_path: std::env::var_os("PATH"),
            node: None,
        },
        |_| None,
    )
    .unwrap();
    let driver = start_driver(
        AgentWire::Acp,
        DriverConfig {
            provider: "claude-acp-live".into(),
            task: "hand-check-05".into(),
            cwd: root.clone(),
            launch,
            limits: DriverLimits::default(),
            resume_session: None,
            ownership: OwnershipSource::Verified,
            mode: DriverMode::Full,
            mcp_servers: vec![],
            mcp_stdio: McpStdioSupport::Baseline,
        },
        &manager,
    )
    .unwrap();
    driver
        .wait_ready(Duration::from_secs(180))
        .expect("Claude ACP session authenticates");
    let run = |allow: bool| {
        driver.prompt("Read src/lib.rs. Use the Edit tool to replace only the title text Your video starts here with Hand check. Change no other file. Do not run shell commands. Request permission for the edit and do not bypass a denial.",vec![]).unwrap();
        let deadline = Instant::now() + Duration::from_secs(240);
        let mut asked = 0;
        let mut text = String::new();
        loop {
            for event in driver.drain_events(64) {
                match event.kind {
                    AgentEventKind::PermissionRequested(prompt) => {
                        asked += 1;
                        let wanted = if allow { "allow_once" } else { "reject_once" };
                        let option =
                            prompt
                                .options
                                .iter()
                                .find(|o| o.kind == wanted)
                                .or_else(|| {
                                    prompt.options.iter().find(|o| {
                                        o.kind.starts_with(if allow { "allow" } else { "reject" })
                                    })
                                });
                        let reply = match option {
                            Some(option) => PermissionReply::Select(option.option_id.clone()),
                            None if !allow => PermissionReply::Cancel,
                            None => panic!("ACP offered no allow choice"),
                        };
                        driver.reply_permission(prompt.id, reply).unwrap();
                    }
                    AgentEventKind::MessageDelta { text: delta, .. } => text.push_str(&delta),
                    AgentEventKind::PromptFinished(outcome) => {
                        let report = serde_json::json!({"allow":allow,"asked":asked,"outcome":outcome,"driver_failure":format!("{:?}",driver.failure()),"reply":text.chars().take(1500).collect::<String>()});
                        fs::write(
                            out.join(if allow {
                                "acp-allowed-turn.json"
                            } else {
                                "acp-denied-turn.json"
                            }),
                            serde_json::to_vec_pretty(&report).unwrap(),
                        )
                        .unwrap();
                        eprintln!("Claude ACP allow={allow}: {outcome:?}, permissions={asked}");
                        return (asked, text, outcome);
                    }
                    AgentEventKind::Failure(error) => panic!("Claude ACP failed: {error:?}"),
                    AgentEventKind::Closed(reason) => panic!("Claude ACP closed: {reason:?}"),
                    _ => {}
                }
            }
            assert!(Instant::now() < deadline, "Claude ACP turn deadline");
            std::thread::sleep(Duration::from_millis(20));
        }
    };
    let (denied, _, denial) = run(false);
    assert!(denied > 0, "ACP must ask before editing");
    assert_eq!(files(&root), before);
    let (allowed, text, acceptance) = run(true);
    assert!(allowed > 0);
    if !acceptance.eligible_for_quiescence() {
        fs::write(out.join("acp-result.json"),serde_json::to_vec_pretty(&serde_json::json!({"status":"fail","wire":"acp","finding":"allowed edit lacks an authoritative clean turn end","outcome":acceptance,"denial":denial,"deny_permissions":denied,"allow_permissions":allowed,"source_diff":files(&root)!=before})).unwrap()).unwrap();
        manager.shutdown(Duration::ZERO);
        panic!("Claude ACP allowed edit cannot be accepted: {acceptance:?}");
    }
    assert!(!text.is_empty());
    let mut after = files(&root);
    let edited = after.remove("src/lib.rs").unwrap();
    let mut expected = before.clone();
    let original_lib = String::from_utf8(expected.remove("src/lib.rs").unwrap()).unwrap();
    assert_eq!(
        String::from_utf8(edited).unwrap(),
        original_lib.replace("Your video starts here", "Hand check")
    );
    assert_eq!(after, expected);
    let mut expected_after = expected;
    expected_after.insert(
        "src/lib.rs".into(),
        original_lib
            .replace("Your video starts here", "Hand check")
            .into_bytes(),
    );
    let outcome = driver.shutdown();
    assert!(outcome.failure.is_none(), "{:?}", outcome.failure);
    assert!(outcome.termination.group_empty);
    assert_eq!(manager.active_count(), 0);
    assert_eq!(files(&root), expected_after);
    fs::write(out.join("acp-result.json"),serde_json::to_vec_pretty(&serde_json::json!({"status":"pass","wire":"acp","adapter":"claude-agent-acp 0.89.1",
        "deny_permissions":denied,"allow_permissions":allowed,"denial":format!("{:?}",denial.stop_reason),"acceptance":format!("{:?}",acceptance.stop_reason),
        "only_src_lib_changed":true,"original_unchanged":true,"process_cleanup":"pass"})).unwrap()).unwrap();
    eprintln!("05-agent-playground: LIVE ACP PASS");
}
