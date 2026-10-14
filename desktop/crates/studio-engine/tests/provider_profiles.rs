//! Integration tests for provider profiles, migration, and qualification resolution.

use serde_json::json;
use sha2::{Digest, Sha256};
use studio_bootstrap::{ProcessTreeManager, WriterOwnership};
use studio_engine::app_paths::AppPaths;
use studio_engine::containment::{self, Containment, Platform};
use studio_engine::providers::{
    host::{self, AdapterFile, McpChoice, REGISTRY_BACKUP_SUFFIX, REGISTRY_FILE, SETTINGS_FILE},
    provider_profiles::{
        BUILTIN_ANTIGRAVITY_ID, BUILTIN_CLAUDE_ID, BUILTIN_CODEX_ID, BUILTIN_PI_ID,
        ProviderProfile, ProviderQualificationStatus, ProviderRegistry, QualificationRankingStatus,
        QualificationSnapshot, REGISTRY_SCHEMA_VERSION,
    },
};
use studio_harness::{
    AdapterConfig,
    discovery::{AdapterStatus, ExecutableSearch, ProbeOptions, probe_adapter},
};
use tempfile::TempDir;

fn setup_paths() -> (TempDir, AppPaths) {
    let temp = TempDir::new().unwrap();
    let paths = AppPaths::new(temp.path().join("data")).unwrap();
    (temp, paths)
}

fn write_evidence_file(
    dir: &std::path::Path,
    name: &str,
    content: &serde_json::Value,
) -> (String, String) {
    let ev_dir = dir.join("evidence");
    std::fs::create_dir_all(&ev_dir).unwrap();
    let file = ev_dir.join(name);
    let bytes = serde_json::to_vec(content).unwrap();
    std::fs::write(&file, &bytes).unwrap();
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let hash = format!("{:x}", hasher.finalize());
    (format!("evidence/{name}"), hash)
}

#[test]
fn default_registry_contains_four_profiles_and_starts_unselected() {
    let registry = ProviderRegistry::default();
    assert_eq!(registry.schema_version, REGISTRY_SCHEMA_VERSION);
    assert_eq!(registry.selected_profile_id, None);
    assert_eq!(registry.profiles.len(), 4);

    let ids: Vec<&str> = registry.profiles.iter().map(|p| p.id.as_str()).collect();
    assert!(ids.contains(&BUILTIN_CLAUDE_ID));
    assert!(ids.contains(&BUILTIN_CODEX_ID));
    assert!(ids.contains(&BUILTIN_PI_ID));
    assert!(ids.contains(&BUILTIN_ANTIGRAVITY_ID));

    for profile in &registry.profiles {
        assert!(profile.builtin, "all default profiles must be builtin");
        assert!(
            profile.experimental,
            "all default profiles start experimental"
        );
    }

    let pi = registry
        .profiles
        .iter()
        .find(|p| p.id == BUILTIN_PI_ID)
        .unwrap();
    assert!(
        pi.adapter.auth_env_names.is_empty(),
        "Pi uses CLI login, no env credentials"
    );
    assert_eq!(
        pi.adapter.mcp,
        McpChoice::Unsupported,
        "Pi uses CLI tool route only"
    );
}

#[test]
fn qualification_snapshot_displays_per_provider_ledger_status_and_ranking() {
    let snapshot = QualificationSnapshot::parse(include_str!("fixtures/m6-results.json")).unwrap();

    assert_eq!(
        snapshot.ranking,
        QualificationRankingStatus::InsufficientEvidence
    );
    assert_eq!(
        snapshot.status_for(BUILTIN_CLAUDE_ID),
        ProviderQualificationStatus::NotRun
    );
    assert!(snapshot.recommended.is_empty());
}

#[test]
fn qualification_snapshot_displays_distinct_qualified_and_blocked_provider_states() {
    let mut ledger: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/m6-results.json")).unwrap();
    ledger["providers"]["claude"]["status"] = json!("qualified");
    ledger["providers"]["codex"]["status"] = json!("qualified");
    ledger["providers"]["pi"]["status"] = json!("blocked");
    ledger["providers"]["antigravity"]["status"] = json!("experimental");
    ledger["ranking"]["status"] = json!("recommended");
    ledger["ranking"]["recommended"] = json!(["claude", "codex"]);

    let snapshot = QualificationSnapshot::parse(&ledger.to_string()).unwrap();

    assert_eq!(snapshot.ranking, QualificationRankingStatus::Recommended);
    assert_eq!(
        snapshot.status_for(BUILTIN_CLAUDE_ID),
        ProviderQualificationStatus::Qualified
    );
    assert_eq!(
        snapshot.status_for(BUILTIN_PI_ID),
        ProviderQualificationStatus::Blocked
    );
    assert_eq!(
        snapshot.status_for(BUILTIN_ANTIGRAVITY_ID),
        ProviderQualificationStatus::Experimental
    );
    assert_eq!(snapshot.recommended, ["claude", "codex"]);
}

#[test]
fn qualification_snapshot_refuses_inconsistent_or_unqualified_recommendations() {
    let mut ledger: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/m6-results.json")).unwrap();
    ledger["ranking"]["status"] = json!("recommended");
    ledger["ranking"]["recommended"] = json!(["claude", "codex"]);

    assert!(QualificationSnapshot::parse(&ledger.to_string()).is_err());
}

#[test]
fn registry_save_load_round_trip_re_derives_builtin_and_experimental_flags() {
    let (_temp, paths) = setup_paths();
    let mut registry = ProviderRegistry::default();
    registry.select_profile(BUILTIN_CLAUDE_ID).unwrap();
    registry.save_custom_adapter(AdapterFile {
        provider: "My Custom Service".into(),
        executable: "/opt/custom".into(),
        args: vec![],
        auth_env_names: vec![],
        auth_method: None,
        mcp: McpChoice::Baseline,
    });
    host::save_registry(&paths, &registry).unwrap();

    let loaded = host::load_registry(&paths)
        .unwrap()
        .expect("loaded registry");
    let claude = loaded
        .profiles
        .iter()
        .find(|p| p.id == BUILTIN_CLAUDE_ID)
        .unwrap();
    assert!(claude.builtin, "builtin flag must be true after reload");
    assert!(
        claude.experimental,
        "experimental flag must be true after reload"
    );

    let custom = loaded
        .profiles
        .iter()
        .find(|p| p.id == "my-custom-service")
        .unwrap();
    assert!(
        !custom.builtin,
        "custom profile must have builtin: false after reload"
    );
    assert!(
        custom.experimental,
        "custom profile must have experimental: true after reload"
    );
}

#[test]
fn migration_from_custom_adapter_preserves_all_fields_and_selects_it() {
    let (_temp, paths) = setup_paths();

    let legacy = AdapterFile {
        provider: "My Custom Agent".into(),
        executable: "/opt/custom/agent".into(),
        args: vec!["--acp".into(), "--fast".into()],
        auth_env_names: vec!["CUSTOM_TOKEN".into(), "CUSTOM_API_KEY".into()],
        auth_method: Some("custom-method".into()),
        mcp: McpChoice::Unsupported,
    };
    std::fs::write(paths.data.join(SETTINGS_FILE), legacy.to_pretty()).unwrap();

    let registry = host::load_registry(&paths)
        .unwrap()
        .expect("migrated registry");
    assert_eq!(registry.schema_version, REGISTRY_SCHEMA_VERSION);
    assert_eq!(
        registry.selected_profile_id,
        Some("my-custom-agent".to_string())
    );

    let profile = registry
        .selected_profile()
        .expect("selected profile exists");
    assert_eq!(profile.label, "My Custom Agent");
    assert_eq!(profile.adapter.executable, "/opt/custom/agent");
    assert_eq!(profile.adapter.args, vec!["--acp", "--fast"]);
    assert_eq!(
        profile.adapter.auth_env_names,
        vec!["CUSTOM_TOKEN", "CUSTOM_API_KEY"]
    );
    assert_eq!(
        profile.adapter.auth_method.as_deref(),
        Some("custom-method")
    );
    assert_eq!(profile.adapter.mcp, McpChoice::Unsupported);

    // Rollback backup copy must exist and old file must remain intact
    let backup_path = paths
        .data
        .join(format!("{SETTINGS_FILE}{REGISTRY_BACKUP_SUFFIX}"));
    assert!(backup_path.exists(), "rollback backup must be created");
    assert!(
        paths.data.join(SETTINGS_FILE).exists(),
        "legacy file remains intact"
    );
    assert!(
        paths.data.join(REGISTRY_FILE).exists(),
        "new registry file created"
    );
}

#[test]
fn migration_matches_builtin_provider_by_name() {
    let (_temp, paths) = setup_paths();

    let legacy = AdapterFile {
        provider: "Codex".into(),
        executable: "/custom/path/codex-acp".into(),
        args: vec!["--port".into(), "9000".into()],
        auth_env_names: vec!["OPENAI_API_KEY".into()],
        auth_method: None,
        mcp: McpChoice::Baseline,
    };
    std::fs::write(paths.data.join(SETTINGS_FILE), legacy.to_pretty()).unwrap();

    let registry = host::load_registry(&paths)
        .unwrap()
        .expect("migrated registry");
    assert_eq!(
        registry.selected_profile_id,
        Some(BUILTIN_CODEX_ID.to_string())
    );

    let profile = registry.selected_profile().unwrap();
    assert_eq!(profile.id, BUILTIN_CODEX_ID);
    assert_eq!(profile.adapter.executable, "/custom/path/codex-acp");
    assert_eq!(profile.adapter.args, vec!["--port", "9000"]);
}

#[test]
fn validation_rejects_duplicate_ids() {
    let mut registry = ProviderRegistry::default();
    registry.profiles.push(ProviderProfile {
        id: BUILTIN_CLAUDE_ID.into(),
        label: "Duplicate Claude".into(),
        description: "".into(),
        cli_executable: String::new(),
        adapter: AdapterFile {
            provider: "Dup".into(),
            executable: "/bin/true".into(),
            args: vec![],
            auth_env_names: vec![],
            auth_method: None,
            mcp: McpChoice::Baseline,
        },
        builtin: false,
        experimental: true,
    });
    assert!(registry.validate().is_err());
}

#[test]
fn validation_rejects_newer_schema_version_as_diagnostic_only() {
    let registry = ProviderRegistry {
        schema_version: 999,
        ..Default::default()
    };
    let err = registry.validate().unwrap_err();
    assert!(err.contains("diagnostic-only"));
}

#[test]
fn validation_rejects_credentials_in_auth_env_names() {
    let mut registry = ProviderRegistry::default();
    registry.profiles[0].adapter.auth_env_names = vec!["sk-ant-api03-secret1234567890".into()];
    assert!(registry.validate().is_err());
}

#[test]
fn save_registry_does_not_modify_legacy_agent_adapter_json() {
    let (_temp, paths) = setup_paths();

    let original_legacy = AdapterFile {
        provider: "Original".into(),
        executable: "/bin/orig".into(),
        args: vec![],
        auth_env_names: vec![],
        auth_method: None,
        mcp: McpChoice::Baseline,
    };
    let legacy_bytes = original_legacy.to_pretty();
    std::fs::write(paths.data.join(SETTINGS_FILE), &legacy_bytes).unwrap();

    let mut registry = ProviderRegistry::default();
    registry.select_profile(BUILTIN_CODEX_ID).unwrap();
    host::save_registry(&paths, &registry).unwrap();

    // Verify agent-adapter.json is strictly untouched!
    let legacy_after = std::fs::read_to_string(paths.data.join(SETTINGS_FILE)).unwrap();
    assert_eq!(
        legacy_after, legacy_bytes,
        "legacy file must remain strictly untouched"
    );
}

#[test]
fn save_settings_updates_registry_without_clobbering_builtin_defaults() {
    let (_temp, paths) = setup_paths();

    let registry = ProviderRegistry::default();
    host::save_registry(&paths, &registry).unwrap();

    // Saving a custom adapter configuration
    let custom_adapter = AdapterFile {
        provider: "My Custom Service".into(),
        executable: "/opt/custom/exec".into(),
        args: vec!["--flag".into()],
        auth_env_names: vec!["CUSTOM_KEY".into()],
        auth_method: None,
        mcp: McpChoice::Baseline,
    };
    host::save_settings(&paths, &custom_adapter).unwrap();

    let loaded_reg = host::load_registry(&paths).unwrap().unwrap();
    // Builtin Claude defaults must remain untouched!
    let claude = loaded_reg
        .profiles
        .iter()
        .find(|p| p.id == BUILTIN_CLAUDE_ID)
        .unwrap();
    assert_eq!(claude.adapter.executable, "claude-agent-acp");

    // The custom profile was added and selected
    let selected = loaded_reg.selected_profile().unwrap();
    assert_eq!(selected.label, "My Custom Service");
    assert_eq!(selected.adapter.executable, "/opt/custom/exec");
}

#[test]
fn scratch_probe_records_authentic_prerequisites_for_all_four_providers() {
    let (temp, _paths) = setup_paths();
    let scratch_dir = temp.path().join("scratch");
    std::fs::create_dir_all(&scratch_dir).unwrap();

    let search = ExecutableSearch {
        managed_dirs: vec![temp.path().join("adapters")],
        gui_path: None,
        node: None,
    };
    let options = ProbeOptions::new(scratch_dir);
    let processes = ProcessTreeManager::new();

    let providers = [
        (
            "Claude",
            "claude-agent-acp",
            vec!["ANTHROPIC_API_KEY".to_string()],
        ),
        (
            "Codex",
            "codex-acp",
            vec!["OPENAI_API_KEY".to_string(), "CODEX_API_KEY".to_string()],
        ),
        ("Pi", "pi-acp", vec!["PI_API_KEY".to_string()]),
        (
            "Antigravity",
            "antigravity-acp",
            vec!["GEMINI_API_KEY".to_string(), "GOOGLE_API_KEY".to_string()],
        ),
    ];

    for (name, exe, auth) in providers {
        let config = AdapterConfig {
            executable: exe.to_string(),
            args: vec![],
            auth_env_names: auth,
        };
        let report = probe_adapter(&config, &search, &options, &processes);
        match report.status {
            AdapterStatus::MissingExecutable { searched } => {
                assert!(
                    !searched.is_empty(),
                    "{name} probe must record searched paths"
                );
            }
            other => panic!("{name} unexpected probe status: {other:?}"),
        }
    }
}

#[test]
fn qualification_resolution_grants_m6_containment_with_authentic_evidence() {
    let (_temp, paths) = setup_paths();
    let ledger_dir = paths.data.join(containment::LEDGER_DIR);
    std::fs::create_dir_all(&ledger_dir).unwrap();

    let digest = "44".repeat(32);
    let platform = Platform {
        system: "Linux".into(),
        machine: "x86_64".into(),
    };

    let evidence_record = json!({
        "evidence_kind": "authentic",
        "schema": "m3-authentic/1",
        "gate": "auth_writer_process_group",
        "fixture_only": false,
        "platform": {"system": "Linux", "machine": "x86_64"},
        "adapter": {"agent_name": "claude-agent-acp", "protocol_version": 1, "launch_identity": digest},
        "cleanup": {"owned_processes_after": 0},
        "measurements": {"scenarios": [
            {"name": "edit", "escaped_descendants": 0, "group_empty_after": true, "contained": true},
            {"name": "stop", "escaped_descendants": 0, "group_empty_after": true, "contained": true},
            {"name": "provider_crash", "escaped_descendants": 0, "group_empty_after": true, "contained": true}
        ]}
    });
    let (ev_path, ev_hash) = write_evidence_file(&ledger_dir, "ev_claude.json", &evidence_record);

    let m6_content = json!({
        "kind": "m6",
        "schema_version": 1,
        "timestamp": "2026-10-07T07:12:00Z",
        "environment": {
            "scope": "test",
            "os": "linux",
            "arch": "x86_64",
            "sdk_id": "test",
            "sdk_manifest_sha256": "0".repeat(64)
        },
        "providers": {
            "claude": {
                "id": "claude",
                "label": "Claude",
                "distribution": "claude-agent-acp",
                "launch_identity": digest,
                "status": "qualified",
                "gates": {
                    "interruption_and_cleanup": {
                        "kind": "authentic",
                        "status": "pass",
                        "criteria": "clean teardown",
                        "notes": "passed",
                        "evidence": [{"path": ev_path, "sha256": ev_hash}]
                    }
                }
            }
        },
        "ranking": {
            "status": "insufficient_evidence",
            "recommended": [],
            "experimental": ["claude"],
            "notes": "testing"
        },
        "acceptance": {
            "development": "not_run",
            "authentic": "not_run",
            "full": "not_run",
            "reason": "testing"
        }
    });
    std::fs::write(
        ledger_dir.join(containment::LEDGER_FILE_M6),
        m6_content.to_string(),
    )
    .unwrap();

    let resolution = containment::resolve_with(&ledger_dir, &digest, &platform);
    match resolution.ownership {
        WriterOwnership::ProcessGroupContained { qualification } => {
            assert!(qualification.starts_with("m6-ledger:interruption_and_cleanup:"));
        }
        _ => panic!(
            "Expected M6 ProcessGroupContained, got {:?}",
            resolution.ownership
        ),
    }
    match resolution.containment {
        Containment::Qualified { evidence_sha256 } => {
            assert_eq!(evidence_sha256, ev_hash);
        }
        _ => panic!("Expected Containment::Qualified"),
    }
}

#[test]
fn qualification_resolution_strictly_refuses_unqualified_m6_launch_without_m3_fallback() {
    let (_temp, paths) = setup_paths();
    let ledger_dir = paths.data.join(containment::LEDGER_DIR);
    std::fs::create_dir_all(&ledger_dir).unwrap();

    let digest = "55".repeat(32);
    let platform = Platform {
        system: "Linux".into(),
        machine: "x86_64".into(),
    };

    // 1. Create a passing legacy M3 ledger for this exact digest
    let m3_evidence = json!({
        "evidence_kind": "authentic",
        "schema": "m3-authentic/1",
        "gate": "auth_writer_process_group",
        "fixture_only": false,
        "platform": {"system": "Linux", "machine": "x86_64"},
        "adapter": {"agent_name": "claude-legacy", "protocol_version": 1, "launch_identity": digest, "fixture_detected": false},
        "cleanup": {"owned_processes_after": 0},
        "measurements": {"scenarios": [
            {"name": "edit", "escaped_descendants": 0, "group_empty_after": true, "contained": true},
            {"name": "stop", "escaped_descendants": 0, "group_empty_after": true, "contained": true},
            {"name": "provider_crash", "escaped_descendants": 0, "group_empty_after": true, "contained": true}
        ]}
    });
    let (ev_m3_path, ev_m3_hash) = write_evidence_file(&ledger_dir, "ev_m3.json", &m3_evidence);
    let m3_content = json!({
        "kind": "m3",
        "schema_version": 1,
        "environment": {
            "adapter": {
                "agent_name": "claude-legacy",
                "protocol_version": 1,
                "fixture_detected": false,
                "launch_identity": digest
            }
        },
        "gates": {
            "auth_writer_process_group": {
                "kind": "authentic",
                "status": "pass",
                "evidence": [{"path": ev_m3_path, "sha256": ev_m3_hash}]
            }
        }
    });
    std::fs::write(
        ledger_dir.join(containment::LEDGER_FILE),
        m3_content.to_string(),
    )
    .unwrap();

    // 2. Create M6 ledger where this launch is present but UNQUALIFIED (status: not_run)
    let m6_content = json!({
        "kind": "m6",
        "schema_version": 1,
        "timestamp": "2026-10-07T07:12:00Z",
        "environment": {
            "scope": "test",
            "os": "linux",
            "arch": "x86_64",
            "sdk_id": "test",
            "sdk_manifest_sha256": "0".repeat(64)
        },
        "providers": {
            "claude": {
                "id": "claude",
                "label": "Claude",
                "distribution": "claude-agent-acp",
                "launch_identity": digest,
                "status": "not_run",
                "gates": {
                    "interruption_and_cleanup": {
                        "kind": "authentic",
                        "status": "not_run",
                        "criteria": "clean teardown",
                        "notes": "not run",
                        "prerequisite": "requires credentials"
                    }
                }
            }
        },
        "ranking": {
            "status": "insufficient_evidence",
            "recommended": [],
            "experimental": ["claude"],
            "notes": "none"
        },
        "acceptance": {
            "development": "not_run",
            "authentic": "not_run",
            "full": "not_run",
            "reason": "none"
        }
    });
    std::fs::write(
        ledger_dir.join(containment::LEDGER_FILE_M6),
        m6_content.to_string(),
    )
    .unwrap();

    // The resolution MUST be Unknown: M6 strictly refuses and MUST NEVER fall back to the passing M3 ledger!
    let resolution = containment::resolve_with(&ledger_dir, &digest, &platform);
    assert_eq!(
        resolution.ownership,
        WriterOwnership::Unknown,
        "Unqualified M6 provider must be refused, not fall back to M3!"
    );
    match resolution.containment {
        Containment::Unknown { reason } => {
            assert!(reason.contains("has not passed qualification"));
        }
        _ => panic!("Expected Containment::Unknown"),
    }
}

#[test]
fn qualification_resolution_falls_back_to_m3_when_launch_not_in_m6() {
    let (_temp, paths) = setup_paths();
    let ledger_dir = paths.data.join(containment::LEDGER_DIR);
    std::fs::create_dir_all(&ledger_dir).unwrap();

    let digest = "66".repeat(32);
    let platform = Platform {
        system: "Linux".into(),
        machine: "x86_64".into(),
    };

    // 1. Create a passing legacy M3 ledger for this exact digest
    let m3_evidence = json!({
        "evidence_kind": "authentic",
        "schema": "m3-authentic/1",
        "gate": "auth_writer_process_group",
        "fixture_only": false,
        "platform": {"system": "Linux", "machine": "x86_64"},
        "adapter": {"agent_name": "legacy-agent", "protocol_version": 1, "launch_identity": digest, "fixture_detected": false},
        "cleanup": {"owned_processes_after": 0},
        "measurements": {"scenarios": [
            {"name": "edit", "escaped_descendants": 0, "group_empty_after": true, "contained": true},
            {"name": "stop", "escaped_descendants": 0, "group_empty_after": true, "contained": true},
            {"name": "provider_crash", "escaped_descendants": 0, "group_empty_after": true, "contained": true}
        ]}
    });
    let (ev_m3_path, ev_m3_hash) =
        write_evidence_file(&ledger_dir, "ev_m3_legacy.json", &m3_evidence);
    let m3_content = json!({
        "kind": "m3",
        "schema_version": 1,
        "environment": {
            "adapter": {
                "agent_name": "legacy-agent",
                "protocol_version": 1,
                "fixture_detected": false,
                "launch_identity": digest
            }
        },
        "gates": {
            "auth_writer_process_group": {
                "kind": "authentic",
                "status": "pass",
                "evidence": [{"path": ev_m3_path, "sha256": ev_m3_hash}]
            }
        }
    });
    std::fs::write(
        ledger_dir.join(containment::LEDGER_FILE),
        m3_content.to_string(),
    )
    .unwrap();

    // 2. Create M6 ledger that DOES NOT have this digest at all (only has claude with different digest)
    let m6_content = json!({
        "kind": "m6",
        "schema_version": 1,
        "timestamp": "2026-10-07T07:12:00Z",
        "environment": {
            "scope": "test",
            "os": "linux",
            "arch": "x86_64",
            "sdk_id": "test",
            "sdk_manifest_sha256": "0".repeat(64)
        },
        "providers": {
            "claude": {
                "id": "claude",
                "label": "Claude",
                "distribution": "claude-agent-acp",
                "launch_identity": "77".repeat(32),
                "status": "not_run",
                "gates": {}
            }
        },
        "ranking": {
            "status": "insufficient_evidence",
            "recommended": [],
            "experimental": ["claude"],
            "notes": "none"
        },
        "acceptance": {
            "development": "not_run",
            "authentic": "not_run",
            "full": "not_run",
            "reason": "none"
        }
    });
    std::fs::write(
        ledger_dir.join(containment::LEDGER_FILE_M6),
        m6_content.to_string(),
    )
    .unwrap();

    // Because this launch is NOT in M6, it legitimately falls back to M3 and qualifies via M3!
    let resolution = containment::resolve_with(&ledger_dir, &digest, &platform);
    match resolution.ownership {
        WriterOwnership::ProcessGroupContained { qualification } => {
            assert!(qualification.starts_with("m3-ledger:auth_writer_process_group:"));
        }
        _ => panic!(
            "Expected legacy M3 ProcessGroupContained, got {:?}",
            resolution.ownership
        ),
    }
}

#[test]
fn qualification_resolution_refuses_when_second_cited_file_is_tampered() {
    let (_temp, paths) = setup_paths();
    let ledger_dir = paths.data.join(containment::LEDGER_DIR);
    std::fs::create_dir_all(&ledger_dir).unwrap();

    let digest = "88".repeat(32);
    let platform = Platform {
        system: "Linux".into(),
        machine: "x86_64".into(),
    };

    let good_evidence = json!({
        "evidence_kind": "authentic",
        "schema": "m6-authentic/1",
        "gate": "interruption_and_cleanup",
        "fixture_only": false,
        "platform": {"system": "Linux", "arch": "x86_64"},
        "adapter": {"name": "claude-agent-acp", "protocol_version": 1, "launch_identity": digest},
        "cleanup": {"owned_processes_after": 0},
        "measurements": {"scenarios": [
            {"name": "edit", "escaped_descendants": 0, "group_empty_after": true, "contained": true},
            {"name": "stop", "escaped_descendants": 0, "group_empty_after": true, "contained": true},
            {"name": "provider_crash", "escaped_descendants": 0, "group_empty_after": true, "contained": true}
        ]}
    });
    let (ev_good_path, ev_good_hash) =
        write_evidence_file(&ledger_dir, "ev_good.json", &good_evidence);

    let second_evidence = json!({
        "evidence_kind": "authentic",
        "schema": "m6-authentic/1",
        "gate": "interruption_and_cleanup",
        "fixture_only": false,
        "platform": {"system": "Linux", "arch": "x86_64"},
        "adapter": {"name": "claude-agent-acp", "protocol_version": 1, "launch_identity": digest},
        "cleanup": {"owned_processes_after": 0},
        "measurements": {"scenarios": []}
    });
    let (ev_tampered_path, _real_hash) =
        write_evidence_file(&ledger_dir, "ev_tampered.json", &second_evidence);
    // Claim a different/tampered hash in the ledger!
    let tampered_claim = "99".repeat(32);

    let m6_content = json!({
        "kind": "m6",
        "schema_version": 1,
        "timestamp": "2026-10-07T07:12:00Z",
        "environment": {
            "scope": "test",
            "os": "linux",
            "arch": "x86_64",
            "sdk_id": "test",
            "sdk_manifest_sha256": "0".repeat(64)
        },
        "providers": {
            "claude": {
                "id": "claude",
                "label": "Claude",
                "distribution": "claude-agent-acp",
                "launch_identity": digest,
                "status": "qualified",
                "gates": {
                    "interruption_and_cleanup": {
                        "kind": "authentic",
                        "status": "pass",
                        "criteria": "clean teardown",
                        "notes": "passed",
                        "evidence": [
                            {"path": ev_good_path, "sha256": ev_good_hash},
                            {"path": ev_tampered_path, "sha256": tampered_claim}
                        ]
                    }
                }
            }
        },
        "ranking": {
            "status": "insufficient_evidence",
            "recommended": [],
            "experimental": ["claude"],
            "notes": "testing"
        },
        "acceptance": {
            "development": "not_run",
            "authentic": "not_run",
            "full": "not_run",
            "reason": "testing"
        }
    });
    std::fs::write(
        ledger_dir.join(containment::LEDGER_FILE_M6),
        m6_content.to_string(),
    )
    .unwrap();

    let resolution = containment::resolve_with(&ledger_dir, &digest, &platform);
    assert_eq!(
        resolution.ownership,
        WriterOwnership::Unknown,
        "When second cited file is tampered, writer containment must be strictly refused!"
    );
}

#[test]
fn version_one_registry_is_upgraded_with_first_party_cli_names() {
    let mut value = serde_json::to_value(ProviderRegistry::default()).unwrap();
    value["schema_version"] = json!(1);
    for profile in value["profiles"].as_array_mut().unwrap() {
        profile.as_object_mut().unwrap().remove("cli_executable");
    }
    let registry = ProviderRegistry::parse(&value.to_string()).unwrap();
    assert_eq!(registry.schema_version, REGISTRY_SCHEMA_VERSION);
    let cli = |id: &str| {
        registry
            .profiles
            .iter()
            .find(|p| p.id == id)
            .unwrap()
            .cli_executable
            .clone()
    };
    assert_eq!(cli(BUILTIN_CLAUDE_ID), "claude");
    assert_eq!(cli(BUILTIN_CODEX_ID), "codex");
    assert_eq!(cli(BUILTIN_ANTIGRAVITY_ID), "");
}

#[test]
fn registry_round_trips_and_derives_detection_specs() {
    let registry = ProviderRegistry::default();
    let parsed = ProviderRegistry::parse(&registry.to_pretty()).unwrap();
    assert_eq!(parsed, registry);
    let specs = registry.detection_specs();
    let claude = specs.iter().find(|s| s.id == BUILTIN_CLAUDE_ID).unwrap();
    assert_eq!(claude.cli_executable, "claude");
    assert_eq!(claude.adapter_executable, "claude-agent-acp");
    let native = specs
        .iter()
        .find(|s| s.id == BUILTIN_ANTIGRAVITY_ID)
        .unwrap();
    assert_eq!(native.cli_executable, native.adapter_executable);
}

#[test]
fn detection_result_maps_to_row_state() {
    use studio_engine::providers::provider_profiles::DetectionState;
    use studio_harness::Detected;
    let found = |cli: bool, adapter: bool| Detected {
        id: "x".into(),
        cli: cli.then(|| "/bin/x".into()),
        adapter: adapter.then(|| "/bin/x-acp".into()),
    };
    assert_eq!(
        DetectionState::of(&found(true, true)),
        DetectionState::Ready
    );
    assert_eq!(
        DetectionState::of(&found(false, true)),
        DetectionState::Ready
    );
    assert_eq!(
        DetectionState::of(&found(true, false)),
        DetectionState::AdapterMissing
    );
    assert_eq!(
        DetectionState::of(&found(false, false)),
        DetectionState::NotFound
    );
}

#[test]
fn node_launch_of_a_selected_adapter_survives_the_setup_fields() {
    // Select puts `node <entry>` into the Setup text fields and Save reads them back.
    let file = AdapterFile {
        provider: "Claude".into(),
        executable: r"C:\Program Files\nodejs\node.exe".into(),
        args: vec![r"C:\Users\Some One\AppData\Roaming\npm\pkg\lib\index.js".into()],
        auth_env_names: vec![],
        auth_method: None,
        mcp: McpChoice::Baseline,
    };
    let rebuilt = AdapterFile::from_fields(None, &file.fields(), McpChoice::Baseline)
        .unwrap()
        .unwrap();
    assert_eq!(rebuilt, file);
}

#[test]
fn detect_rows_offer_the_action_their_state_allows() {
    use studio_engine::providers::provider_profiles::{
        DetectRowAction as A, DetectionState as S, row_action,
    };
    let guide = Some("https://example.com/install");
    for installable in [false, true] {
        for tools in [false, true] {
            for g in [None, guide] {
                assert_eq!(row_action(S::Ready, installable, tools, g), A::TurnOn);
                let missing = row_action(S::AdapterMissing, installable, tools, g);
                let expected = match (installable, tools, g) {
                    (true, true, _) => A::InstallAdapter,
                    (true, false, _) => A::GetNode,
                    (false, _, Some(url)) => A::InstallGuide(url),
                    (false, _, None) => A::None,
                };
                assert_eq!(missing, expected, "{installable} {tools} {g:?}");
                assert_eq!(
                    row_action(S::NotFound, installable, tools, g),
                    g.map_or(A::None, A::InstallGuide)
                );
            }
        }
    }
}

#[test]
fn only_https_install_guides_are_opened() {
    use studio_engine::providers::provider_profiles::{
        NODE_DOWNLOAD_URL, install_guide_url, is_openable,
    };
    for refused in [
        "http://example.com",
        "file:///C:/Windows/system32/calc.exe",
        "ms-settings:privacy",
        "javascript:alert(1)",
        "",
        " https://example.com",
        "https://",
        "HTTPS//example.com",
    ] {
        assert!(!is_openable(refused), "{refused:?}");
    }
    assert!(is_openable("https://example.com/install"));
    assert!(is_openable(NODE_DOWNLOAD_URL));
    for id in [BUILTIN_CLAUDE_ID, BUILTIN_CODEX_ID, BUILTIN_PI_ID] {
        let url = install_guide_url(id).unwrap_or_else(|| panic!("{id} has a guide"));
        assert!(is_openable(url), "{url}");
    }
    assert_eq!(install_guide_url(BUILTIN_ANTIGRAVITY_ID), None);
    assert_eq!(install_guide_url("my-custom-adapter"), None);
}

/// An executable file Detect agents accepts: `<name>.exe` on Windows, mode 755 elsewhere.
fn fake_executable(dir: &std::path::Path, name: &str) -> std::path::PathBuf {
    std::fs::create_dir_all(dir).unwrap();
    let path = dir.join(if cfg!(windows) {
        format!("{name}.exe")
    } else {
        name.to_owned()
    });
    std::fs::write(&path, b"x").unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    path
}

/// A detection environment whose only search root is `bin`.
fn only_path(bin: &std::path::Path) -> studio_harness::DetectionEnv {
    studio_harness::DetectionEnv {
        path: Some(std::env::join_paths([bin]).unwrap()),
        ..Default::default()
    }
}

fn node_launch(node: &std::path::Path, entry: &std::path::Path) -> AdapterFile {
    AdapterFile {
        provider: "Claude".into(),
        executable: node.to_string_lossy().into_owned(),
        args: vec![entry.to_string_lossy().into_owned()],
        auth_env_names: vec!["ANTHROPIC_API_KEY".into()],
        auth_method: None,
        mcp: McpChoice::Baseline,
    }
}

#[test]
fn a_saved_node_launch_never_makes_a_deleted_builtin_adapter_look_ready() {
    use studio_engine::providers::provider_profiles::{DetectionState, scan_agents};
    let (temp, paths) = setup_paths();
    let bin = temp.path().join("bin");
    let node = fake_executable(&bin, "node");
    let entry = temp.path().join("deleted").join("index.js");
    let env = only_path(&bin);
    let adapters = paths.data.join("adapters");
    let claude_state = |registry: &ProviderRegistry| {
        let scan = scan_agents(registry, &env, &adapters);
        let found = scan
            .found
            .iter()
            .find(|found| found.id == BUILTIN_CLAUDE_ID)
            .unwrap();
        (scan.state(found), scan.is_ready(BUILTIN_CLAUDE_ID))
    };

    // Saved by id (the turn-on path) and by label (Save, or a registry written before).
    host::save_profile_settings(&paths, BUILTIN_CLAUDE_ID, &node_launch(&node, &entry)).unwrap();
    let by_id = host::load_registry(&paths).unwrap().unwrap();
    host::save_settings(&paths, &node_launch(&node, &entry)).unwrap();
    let by_label = host::load_registry(&paths).unwrap().unwrap();
    for registry in [&by_id, &by_label] {
        let claude = registry.selected_profile().unwrap();
        assert_eq!(claude.id, BUILTIN_CLAUDE_ID);
        assert_eq!(claude.adapter.executable, node.to_string_lossy());
        assert_eq!(
            claude_state(registry),
            (DetectionState::NotFound, false),
            "only node is installed, so the adapter is not"
        );
    }

    // The adapter itself on the search path is what makes the row ready.
    fake_executable(&bin, "claude-agent-acp");
    assert_eq!(claude_state(&by_id), (DetectionState::Ready, true));
}

#[test]
fn a_turn_on_save_changes_only_that_profile_and_keeps_its_identity() {
    let (temp, paths) = setup_paths();
    let mut registry = ProviderRegistry::default();
    registry.save_custom_adapter(AdapterFile {
        provider: "Mine".into(),
        executable: "/opt/mine-acp".into(),
        args: vec![],
        auth_env_names: vec![],
        auth_method: None,
        mcp: McpChoice::Baseline,
    });
    host::save_registry(&paths, &registry).unwrap();
    let before = host::load_registry(&paths).unwrap().unwrap();

    // A launch whose label names the built-in Claude, saved for the custom profile.
    let launch = node_launch(&temp.path().join("node"), &temp.path().join("a.js"));
    host::save_profile_settings(&paths, "mine", &launch).unwrap();
    let after = host::load_registry(&paths).unwrap().unwrap();

    assert_eq!(after.selected_profile_id.as_deref(), Some("mine"));
    for (old, new) in before.profiles.iter().zip(&after.profiles) {
        assert_eq!(
            (&old.id, &old.label, &old.description, &old.cli_executable),
            (&new.id, &new.label, &new.description, &new.cli_executable)
        );
        if new.id == "mine" {
            assert_eq!(new.adapter, launch);
        } else {
            assert_eq!(old.adapter, new.adapter, "{} is untouched", new.id);
        }
    }
    assert!(host::save_profile_settings(&paths, "gone", &launch).is_err());
}

#[test]
fn a_turn_on_save_creates_the_registry_when_none_exists() {
    let (temp, paths) = setup_paths();
    let launch = node_launch(&temp.path().join("node"), &temp.path().join("a.js"));
    host::save_profile_settings(&paths, BUILTIN_CODEX_ID, &launch).unwrap();
    let registry = host::load_registry(&paths).unwrap().unwrap();
    let codex = registry.selected_profile().unwrap();
    assert_eq!(codex.id, BUILTIN_CODEX_ID);
    assert_eq!(codex.label, "Codex");
    assert_eq!(codex.adapter, launch);
    assert_eq!(host::load_settings(&paths).unwrap(), Some(launch));
}

#[test]
fn a_custom_node_launch_is_detected_by_its_entry() {
    use studio_engine::providers::provider_profiles::{
        AdapterIdentity, DetectionState, scan_agents,
    };
    let (temp, paths) = setup_paths();
    let bin = temp.path().join("bin");
    let node = fake_executable(&bin, "node");
    let entry = temp.path().join("pkg").join("cli.js");
    let mut launch = node_launch(&node, &entry);
    launch.provider = "Mine".into();
    launch.args = vec![
        "--no-warnings".into(),
        launch.args[0].clone(),
        "--acp".into(),
    ];
    let mut registry = ProviderRegistry::default();
    registry.save_custom_adapter(launch);
    let mine = registry.selected_profile().unwrap().clone();
    assert_eq!(
        mine.adapter_identity(),
        AdapterIdentity::Entry(Some(entry.clone()))
    );
    // Selecting another file never carries the old entry along.
    assert_eq!(mine.launch_template().args, vec!["--acp"]);
    assert!(
        registry
            .detection_specs()
            .iter()
            .all(|spec| spec.id != mine.id),
        "node is never searched for as the adapter"
    );

    let state = || {
        let scan = scan_agents(&registry, &only_path(&bin), &paths.data.join("adapters"));
        let found = scan.found.iter().find(|f| f.id == mine.id).unwrap();
        scan.state(found)
    };
    assert_eq!(state(), DetectionState::NotFound);
    std::fs::create_dir_all(entry.parent().unwrap()).unwrap();
    std::fs::write(&entry, "").unwrap();
    assert_eq!(state(), DetectionState::Ready);
}

#[test]
fn builtins_are_detected_by_their_own_adapter_name_whatever_launch_is_saved() {
    use studio_engine::providers::provider_profiles::{AdapterIdentity, builtin_adapter};
    let mut registry = ProviderRegistry::default();
    for profile in &mut registry.profiles {
        profile.adapter.executable = "/usr/bin/node".into();
        profile.adapter.args = vec!["/x/index.js".into()];
    }
    let specs = registry.detection_specs();
    assert_eq!(specs.len(), registry.profiles.len());
    for spec in specs {
        assert_eq!(
            Some(spec.adapter_executable.as_str()),
            builtin_adapter(&spec.id)
        );
    }
    for profile in &registry.profiles {
        assert!(matches!(
            profile.adapter_identity(),
            AdapterIdentity::Name(_)
        ));
        assert!(profile.launch_template().args.is_empty());
    }
}

#[test]
fn a_missing_install_tool_is_named() {
    use studio_engine::providers::provider_profiles::DetectionScan;
    let scan = |node: bool, npm: bool| DetectionScan {
        node: node.then(|| "/n/node".into()),
        npm: npm.then(|| "/n/npm".into()),
        ..Default::default()
    };
    assert_eq!(scan(true, false).missing_tools(), ["npm"]);
    assert_eq!(scan(false, true).missing_tools(), ["Node.js"]);
    assert_eq!(scan(false, false).missing_tools(), ["Node.js", "npm"]);
    assert!(scan(true, false).tools().is_none());
    assert!(scan(true, true).tools().is_some());
    assert!(scan(true, true).missing_tools().is_empty());
}
