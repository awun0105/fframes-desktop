//! Candidate validation and the six project tools against a real preview-worker process
//! (a deterministic python worker installed by an injected compiler) and the real shared
//! build service. No SDK is needed.
use serde_json::{Value, json};
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, atomic::Ordering},
    time::{Duration, Instant},
};
use studio_bootstrap::{ProcessTreeManager, WriterOwnership};
use studio_engine::agent_tools::{
    BoundRevision, ToolBinding, ToolDispatcher, ToolErrorCode,
    backend::{ArtifactStore, ProjectToolBackend, ToolBackendConfig, WriterGate},
};
use studio_engine::services::build_service::{BuildKey, BuildLimits, BuildService};
use studio_engine::services::candidate_runner::{
    CandidateRunConfig, RunScopes, run_candidate_validation,
};
use studio_engine::services::preview_coordinator::{BuildSpec, PreviewCoordinator, SeekIntent};
use studio_engine::{
    AgentTaskContext, CaptureTicket, Controller, JobKind, JobResult, PreviewState,
    QuiescenceEvidence, RepairDecision, TaskState, TurnCompletion, WriterObservation,
    app_paths::AppPaths,
    candidate_validation::{
        CandidateError, CapturePhase, CapturedCandidate, FailureKind, NextStep, ValidationStage,
        capture_candidate, capture_candidate_observed, next_step,
    },
};
use studio_project::checkpoint::Checkpoints;

#[path = "../../../crates/studio-engine/tests/support/build_fixture.rs"]
mod fixture;
use fixture::*;

mod broker;
mod tools;
mod validation;

struct World {
    _temp: tempfile::TempDir,
    sdk: PathBuf,
    paths: AppPaths,
    controller: Controller,
    compiler: Arc<FakeCompiler>,
    service: BuildService,
}

fn world() -> World {
    let temp = tempfile::tempdir().unwrap();
    let sdk = fake_sdk(temp.path());
    let root = temp.path().join("video");
    create_project(&root);
    let paths = AppPaths::new(temp.path().join("data")).unwrap();
    let controller = Controller::open(&root, &paths).unwrap();
    let compiler = FakeCompiler::new(true);
    let service = BuildService::new(
        ProcessTreeManager::new(),
        compiler.clone(),
        BuildLimits::default(),
    );
    World {
        _temp: temp,
        sdk,
        paths,
        controller,
        compiler,
        service,
    }
}

fn qualified() -> WriterOwnership {
    WriterOwnership::ProcessGroupContained {
        qualification: "test-adapter".into(),
    }
}

/// Clean end-turn evidence bound to the task's current writer generation; termination
/// is derived by the controller from the task's own process scope.
fn evidence(controller: &Controller, context: &AgentTaskContext) -> QuiescenceEvidence {
    QuiescenceEvidence {
        identity: context.identity.clone(),
        provider_session: None,
        writer: controller
            .agent_task()
            .unwrap()
            .writer_generation()
            .expect("a writer was started"),
        completion: TurnCompletion::EndTurn,
        cancel_requested: false,
        unresolved_requests: 0,
        observed: WriterObservation {
            ownership: qualified(),
            escaped_pids: vec![],
        },
    }
}

impl World {
    fn start(&mut self) -> AgentTaskContext {
        let context = self.controller.begin_agent_task("edit").unwrap();
        self.reopen(&context);
        context
    }
    fn reopen(&mut self, context: &AgentTaskContext) {
        let id = &context.identity;
        self.controller
            .agent_writer_started(id, qualified())
            .unwrap();
        // A repair attempt already put the task back into `Editing`.
        if self.controller.agent_task().unwrap().state() != TaskState::Editing {
            self.controller
                .agent_task_transition(id, TaskState::Editing)
                .unwrap();
        }
        self.controller
            .agent_task_transition(id, TaskState::Quiescing)
            .unwrap();
    }
    fn checkpoints(&self) -> Checkpoints {
        Checkpoints::new(
            &self
                .paths
                .project(&self.controller.project.manifest.project_id),
        )
        .unwrap()
    }
    fn ticket(&mut self, context: &AgentTaskContext) -> CaptureTicket {
        let evidence = evidence(&self.controller, context);
        self.controller
            .agent_complete_quiescence(&context.identity, &evidence)
            .unwrap()
    }
    fn capture(&mut self, context: &AgentTaskContext) -> CapturedCandidate {
        let ticket = self.ticket(context);
        let captured = capture_candidate(&ticket, &self.checkpoints()).unwrap();
        self.controller
            .agent_record_candidate(ticket, &captured)
            .unwrap();
        captured
    }
    fn config(&self) -> CandidateRunConfig {
        CandidateRunConfig {
            service: self.service.clone(),
            sdk: self.sdk.clone(),
            compatibility: manifest(),
            builds: self.paths.builds(),
        }
    }
    fn scopes(&self) -> RunScopes {
        RunScopes {
            compiler: self.controller.processes.sub_manager(),
            worker: self.controller.processes.sub_manager(),
        }
    }
    fn run(
        &self,
        captured: &CapturedCandidate,
        repairs: u32,
    ) -> studio_engine::services::candidate_runner::CandidateRun {
        run_candidate_validation(
            captured,
            &self.checkpoints(),
            &self.config(),
            &self.scopes(),
            10,
            repairs,
        )
    }
}

fn worker_config(draft: &Path, value: Value) {
    fs::write(draft.join(FAKE_CONFIG), serde_json::to_vec(&value).unwrap()).unwrap();
}

/// Display the current source through the UI coordinator so "old playback" exists.
fn display_base(w: &mut World) -> (PreviewCoordinator, PreviewState) {
    let coordinator = PreviewCoordinator::new(w.controller.processes.sub_manager());
    let mut state = PreviewState::default();
    let tag = w.controller.begin_job(JobKind::Build).unwrap();
    state.begin(tag.clone());
    coordinator.build(BuildSpec {
        project: w.controller.project.clone(),
        sdk: w.sdk.clone(),
        compatibility: manifest(),
        builds: w.paths.builds(),
        tag,
        compiler: w.controller.operation_processes(),
        worker: w.controller.processes.sub_manager(),
        service: w.service.clone(),
    });
    let deadline = Instant::now() + Duration::from_secs(60);
    let ready = loop {
        let e = coordinator.events();
        if let Some(tag) = e.compiled {
            w.controller
                .complete(&tag, JobResult::Built(tag.base_source.clone()))
                .unwrap();
        }
        if let Some((_, error)) = e.error {
            panic!("base preview failed: {error}");
        }
        if let Some(r) = e.ready {
            w.controller.reconcile().unwrap();
            break r;
        }
        assert!(Instant::now() < deadline, "base preview deadline");
        std::thread::sleep(Duration::from_millis(10));
    };
    assert!(coordinator.commit(ready.identity().clone(), ready.seek_serial));
    state.install(&ready, w.controller.state()).unwrap();
    (coordinator, state)
}

fn seek_and_wait(p: &PreviewCoordinator, state: &mut PreviewState, position: usize) {
    let serial = state.seek(position, state.scale()).unwrap();
    p.seek(SeekIntent {
        identity: state.displayed().unwrap().clone(),
        serial,
        position,
        scale: state.scale(),
    });
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(f) = p.events().frame
            && state.accepts_frame(&f)
        {
            assert_eq!(f.response.frame_index, position);
            return;
        }
        assert!(
            Instant::now() < deadline,
            "the displayed worker stopped answering"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

// ---- the six tools --------------------------------------------------------------------------

struct Tools {
    backend: Arc<ProjectToolBackend>,
    dispatcher: ToolDispatcher,
    artifacts: PathBuf,
}

fn tools(w: &World) -> Tools {
    let artifacts = w._temp.path().join("artifacts");
    let backend = Arc::new(
        ProjectToolBackend::new(ToolBackendConfig {
            project_id: w.controller.project.manifest.project_id.clone(),
            service: w.service.clone(),
            sdk: w.sdk.clone(),
            compatibility: manifest(),
            builds: w.paths.builds(),
            history: w.paths.project(&w.controller.project.manifest.project_id),
            artifacts: artifacts.clone(),
            processes: w.controller.processes.sub_manager(),
            gate: WriterGate::default(),
        })
        .unwrap(),
    );
    Tools {
        dispatcher: ToolDispatcher::new(backend.clone()),
        backend,
        artifacts,
    }
}

fn call(
    t: &Tools,
    binding: &ToolBinding,
    method: &str,
    params: Value,
) -> Result<Value, ToolErrorCode> {
    t.dispatcher
        .dispatch(binding, method, &params, &|| false)
        .map_err(|e| e.code)
}

fn draft_binding(t: &Tools, ctx: &AgentTaskContext) -> ToolBinding {
    t.backend.register_task(
        &ctx.identity,
        ctx.draft.clone(),
        ctx.source_base.revision().clone(),
    );
    ToolBinding {
        task: ctx.identity.clone(),
        revision: BoundRevision::Draft,
    }
}

fn sha2_hex(bytes: &[u8]) -> impl std::fmt::LowerHex {
    use sha2::Digest;
    sha2::Sha256::digest(bytes)
}

/// Runs `studio-mcp` against `capability`, feeding `requests`, and returns every stdout
/// line parsed as JSON (stdout must hold nothing else).
#[cfg(unix)]
fn mcp_session(capability: &Path, requests: &[Value]) -> Vec<Value> {
    use std::io::Write;
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_studio-mcp"))
        .arg("--capability")
        .arg(capability)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
    let mut stdin = child.stdin.take().unwrap();
    for request in requests {
        writeln!(stdin, "{request}").unwrap();
    }
    drop(stdin);
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).expect("stdout is JSON-RPC only"))
        .collect()
}
