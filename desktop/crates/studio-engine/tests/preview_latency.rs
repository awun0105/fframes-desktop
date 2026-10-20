//! How long a Studio preview takes to show its first frame, against agreed budgets:
//!
//! * install: installing the SDK and warming the shared build cache from it;
//! * cold: the first preview of a new project after that install;
//! * warm: reopening the unchanged project in a new app session;
//! * edit: changing one line of `src` and waiting for the new frame.
//!
//! ```text
//! SDK_BUNDLE=/absolute/path/to/assembled-bundle cargo test --locked \
//!   --manifest-path desktop/Cargo.toml -p studio-engine --test preview_latency -- --ignored --nocapture
//! ```
//!
//! `PREVIEW_BUDGET_<CASE>_S` overrides a budget (seconds), `PREVIEW_LATENCY_EVIDENCE`
//! names a JSON file for the measurements.
use fframes_studio_protocol::PreviewIdentity;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant},
};
use studio_bootstrap::ProcessTreeManager;
use studio_engine::{
    build_materialization::sdk_pin,
    sdk_setup,
    services::{
        build_service::{BuildLimits, BuildService, Subscriber, SubscriberKind},
        worker_project::{
            CargoCompiler, compile_portable_worker_via, launch_preview_worker, warm_build_cache,
        },
    },
};
use studio_sdk::CompatibilityManifest;

/// Budgets in seconds, about twice what the self-hosted Windows runner measures.
const BUDGETS: [(&str, f64); 4] = [
    ("install", 420.),
    ("cold", 30.),
    ("warm", 10.),
    ("edit", 20.),
];

/// Time to compile (or reuse) the worker, then to start it and render frame 0.
struct Measured {
    build: Duration,
    first_frame: Duration,
    pixels: Vec<u8>,
}

/// One preview as the app opens it: compile through `service`, launch, timeline, frame 0.
fn preview(
    service: &BuildService,
    root: &Path,
    sdk: &Path,
    manifest: &CompatibilityManifest,
    builds: &Path,
) -> Measured {
    let manager = ProcessTreeManager::new();
    let started = Instant::now();
    let project = studio_project::open(root).unwrap();
    let build = compile_portable_worker_via(
        service,
        Subscriber::new(SubscriberKind::Ui, "preview latency"),
        &project,
        sdk,
        manifest.clone(),
        builds,
        &manager,
    )
    .expect("compile the preview worker");
    let built = started.elapsed();
    let identity = PreviewIdentity {
        project_id: project.manifest.project_id.clone().into(),
        open_session: uuid::Uuid::new_v4().to_string(),
        source_revision: project.inventory.revision.as_str().to_owned(),
        worker_generation: 1,
    };
    let mut worker = launch_preview_worker(build, identity, &manager).expect("launch the worker");
    worker.timeline().expect("compiled timeline");
    let frame = worker.frame(0, 1, 1.0).expect("render frame 0");
    let measured = Measured {
        build: built,
        first_frame: started.elapsed() - built,
        pixels: frame.pixels,
    };
    manager.shutdown(Duration::from_secs(5));
    measured
}

fn fresh_service() -> BuildService {
    BuildService::new(
        ProcessTreeManager::new(),
        Arc::new(CargoCompiler),
        BuildLimits::default(),
    )
}

#[test]
#[ignore = "requires SDK_BUNDLE containing the current assembled managed SDK"]
fn preview_latency_stays_within_budget() {
    let bundle = PathBuf::from(std::env::var_os("SDK_BUNDLE").expect("SDK_BUNDLE"));
    let manifest = CompatibilityManifest::from_json_str(
        &fs::read_to_string(bundle.join("compatibility.json")).expect("SDK compatibility manifest"),
    )
    .expect("valid SDK compatibility manifest");
    let artifacts: Vec<_> = manifest
        .artifacts
        .iter()
        .map(|artifact| {
            (
                artifact.clone(),
                bundle.join(artifact.url.trim_start_matches("file://")),
            )
        })
        .collect();
    // A clean Studio data directory. Short, for the Windows linker: build-script objects
    // nest about 230 characters below the builds directory.
    let temporary = tempfile::tempdir().unwrap();
    let builds = temporary.path().join("builds");

    let started = Instant::now();
    let processes = ProcessTreeManager::new();
    // As the app installs: checked in place by building the starter project, then warmed.
    let sdk = sdk_setup::installer(&temporary.path().join("sdk"), &processes, &builds)
        .install_from_local_artifacts(&manifest, &artifacts)
        .expect("install the SDK bundle");
    warm_build_cache(&sdk, &builds, &processes).expect("warm the build cache");
    let install = started.elapsed();

    let root = temporary.path().join("video");
    studio_project::create(
        &root,
        "Preview latency",
        sdk_pin(&manifest),
        &manifest.fframes_version,
        &manifest.runtime_version,
    )
    .unwrap();
    let cold = preview(&fresh_service(), &root, &sdk, &manifest, &builds);
    // Reopening starts a new app session: nothing is cached in memory.
    let session = fresh_service();
    let warm = preview(&session, &root, &sdk, &manifest, &builds);
    assert_eq!(
        warm.pixels, cold.pixels,
        "the unchanged project renders the same frame"
    );

    let lib = root.join("src/lib.rs");
    let source = fs::read_to_string(&lib).unwrap();
    assert!(source.contains("y=\"300\""), "the starter title line moved");
    fs::write(&lib, source.replacen("y=\"300\"", "y=\"420\"", 1)).unwrap();
    let edit = preview(&session, &root, &sdk, &manifest, &builds);
    assert_ne!(edit.pixels, warm.pixels, "the edit shows a new frame");

    let seconds = |duration: Duration| duration.as_secs_f64();
    let cases = [
        ("install", seconds(install), 0., 0.),
        (
            "cold",
            seconds(cold.build + cold.first_frame),
            seconds(cold.build),
            seconds(cold.first_frame),
        ),
        (
            "warm",
            seconds(warm.build + warm.first_frame),
            seconds(warm.build),
            seconds(warm.first_frame),
        ),
        (
            "edit",
            seconds(edit.build + edit.first_frame),
            seconds(edit.build),
            seconds(edit.first_frame),
        ),
    ];
    let mut evidence = Vec::new();
    let mut over = Vec::new();
    println!("case     total_s  build_s  first_frame_s  budget_s");
    for ((case, total, build, first_frame), (name, default)) in cases.into_iter().zip(BUDGETS) {
        assert_eq!(case, name);
        let budget = std::env::var(format!("PREVIEW_BUDGET_{}_S", case.to_uppercase()))
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(default);
        println!("{case:<8} {total:>7.2}  {build:>7.2}  {first_frame:>13.2}  {budget:>8.0}");
        evidence.push(serde_json::json!({
            "case": case, "total_s": total, "build_s": build,
            "first_frame_s": first_frame, "budget_s": budget,
        }));
        if total > budget {
            over.push(format!(
                "{case}: {total:.1} s over its {budget:.0} s budget"
            ));
        }
    }
    if let Some(path) = std::env::var_os("PREVIEW_LATENCY_EVIDENCE") {
        fs::write(path, serde_json::to_vec_pretty(&evidence).unwrap()).unwrap();
    }
    assert!(over.is_empty(), "{}", over.join("; "));
}
