//! A compiled worker renders a real frame through the preview transport Studio uses, a click
//! on the title resolves to its source anchor in the fixture's own source file, and a
//! crashed worker is replaced by a new generation that renders again.
//!
//! The fixture binary comes from `ANNOTATED_WORKER_BINARY`, else from the fixture's own
//! target directory (build it first):
//!
//! ```text
//! cargo build --release --manifest-path desktop/fixtures/annotated-video-overlay/Cargo.toml
//! cargo test --locked --manifest-path desktop/Cargo.toml -p fframes-studio \
//!   --test worker_roundtrip -- --ignored
//! ```
use fframes_studio_protocol::{EditorFrameStatus, PreviewIdentity};
use sha2::{Digest, Sha256};
use std::{path::PathBuf, process::Command, time::Duration};
use studio_bootstrap::{ChildEnvironment, ProcessTreeManager};
use studio_engine::services::{
    preview_worker_client::PreviewWorkerClient, worker_client::WorkerClient,
};
use studio_engine::{CanvasRect, CanvasViewport, DisplayedFrameTag, hit_test};
use studio_project::{
    ProjectPath, SourceInventory, SourceRevision,
    revision::FileKind,
    source_index::{SourceAnchor, SourceIndex, SourceIndexInput},
};

fn fixture_binary(project: &std::path::Path) -> PathBuf {
    if let Some(binary) = std::env::var_os("ANNOTATED_WORKER_BINARY") {
        return PathBuf::from(binary);
    }
    let metadata = Command::new("cargo")
        .args([
            "metadata",
            "--no-deps",
            "--format-version",
            "1",
            "--manifest-path",
        ])
        .arg(project.join("Cargo.toml"))
        .output()
        .unwrap();
    assert!(metadata.status.success());
    let metadata: serde_json::Value = serde_json::from_slice(&metadata.stdout).unwrap();
    let root = PathBuf::from(metadata["target_directory"].as_str().unwrap());
    let name = if cfg!(windows) {
        "annotated-video-overlay.exe"
    } else {
        "annotated-video-overlay"
    };
    let debug = root.join("debug").join(name);
    if debug.is_file() {
        debug
    } else {
        root.join("release").join(name)
    }
}

/// An abrupt death of the worker process, as a crash would be.
fn kill_process(pid: u32) {
    let output = if cfg!(windows) {
        Command::new("taskkill")
            .args(["/F", "/T", "/PID", &pid.to_string()])
            .output()
    } else {
        Command::new("kill")
            .args(["-KILL", &pid.to_string()])
            .output()
    }
    .unwrap();
    assert!(output.status.success(), "{output:?}");
}

#[test]
#[ignore = "requires the compiled native annotated fixture and loopback sockets"]
fn real_worker_frame_anchor_crash_and_restart() {
    let project =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../fixtures/annotated-video-overlay");
    let binary = fixture_binary(&project);
    let inventory = SourceInventory::scan(&project).unwrap();
    let revision = inventory.revision.as_str().to_owned();
    let manager = ProcessTreeManager::new();
    let mut generations = Vec::new();

    for generation in [41, 42] {
        // The worker reports the generation it was launched with; a restart is a new one.
        let identity = PreviewIdentity {
            project_id: "annotated-video".into(),
            open_session: "worker-roundtrip".into(),
            source_revision: revision.clone(),
            worker_generation: generation,
        };
        let mut transport = WorkerClient::new(&revision, generation);
        transport
            .spawn_worker(
                &binary,
                &[
                    "--worker",
                    "--preview-worker",
                    "--project-id",
                    &identity.project_id,
                    "--open-session",
                    &identity.open_session,
                ],
                Some(&project),
                ChildEnvironment::default_allowlist(),
                &manager,
            )
            .unwrap();
        let mut worker = PreviewWorkerClient::new(transport, identity.clone());
        let hello = worker.negotiate().unwrap();
        assert_eq!(hello.identity, identity);
        assert!(worker.supports_editor_frames());
        let timeline = worker.timeline().unwrap();
        assert_eq!((timeline.width, timeline.height), (1920, 1080));

        // A real frame, fitted to the preview limit, with the title painted where its metadata says.
        let frame = worker.frame(0, 7, 1.0).unwrap();
        let header = &frame.response.header;
        assert_eq!((header.width, header.height), (1280, 720));
        assert_eq!(frame.pixels.len(), header.payload_len);
        let metadata = frame.response.editor_metadata.as_ref().unwrap();
        assert_eq!(metadata.status, EditorFrameStatus::Supported);
        let title = metadata
            .objects
            .iter()
            .find(|object| object.identity.object_key == "headline")
            .expect("the title object is in the frame metadata");
        let scale = f64::from(header.width) / 1920.;
        let to_pixels = |video: f32| (f64::from(video) * scale) as usize;
        let (left, top) = (to_pixels(title.bounds.x), to_pixels(title.bounds.y));
        let right = to_pixels(title.bounds.x + title.bounds.width).min(1280);
        let bottom = to_pixels(title.bounds.y + title.bounds.height).min(720);
        let painted = (top..bottom)
            .flat_map(|y| (left..right).map(move |x| y * header.stride_bytes as usize + x * 4))
            .filter(|&offset| frame.pixels[offset..offset + 3].iter().all(|&c| c > 180))
            .count();
        assert!(
            painted > 0,
            "no bright title pixels inside {:?}",
            title.bounds
        );

        // Click the title on a half-size canvas and take the selection's source anchor.
        let viewport = CanvasViewport::new(
            CanvasRect {
                x: 0.,
                y: 0.,
                width: 960.,
                height: 540.,
            },
            1920,
            1080,
        )
        .unwrap();
        let displayed = DisplayedFrameTag::from_metadata(identity.clone(), metadata).unwrap();
        let (x, y) = (
            f64::from(title.bounds.x + title.bounds.width / 2.) / 2.,
            f64::from(title.bounds.y + title.bounds.height / 2.) / 2.,
        );
        let selected = hit_test(metadata, &displayed, &viewport, x, y)
            .unwrap()
            .expect("the click lands on the title");
        assert_eq!(selected.identity, title.identity);
        assert!(
            hit_test(metadata, &displayed, &viewport, 2., 2.)
                .unwrap()
                .is_none(),
            "the empty corner selects nothing"
        );
        let anchor = selected
            .source_anchor
            .expect("the title has a source anchor");
        assert_eq!(anchor.path, "src/main.rs");
        assert_eq!(anchor.symbol, "render_frame");
        assert_eq!(anchor.marker.as_deref(), Some("overlay-title-anchor"));

        // The anchor resolves in the fixture's own source to the line that sets the title.
        let sources = inventory
            .files
            .iter()
            .filter(|file| file.kind == FileKind::Rust)
            .map(|file| SourceIndexInput {
                path: file.path.clone(),
                kind: file.kind,
                expected_sha256: file.sha256.clone(),
                expected_size: file.size,
                bytes: std::fs::read(project.join(file.path.as_str())).unwrap(),
            })
            .collect();
        let index = SourceIndex::build(
            SourceRevision::try_from(revision.clone()).unwrap(),
            sources,
            &|| false,
        )
        .unwrap();
        let main_rs = inventory
            .files
            .iter()
            .find(|file| file.path.as_str() == anchor.path)
            .unwrap();
        assert_eq!(
            main_rs.sha256,
            format!(
                "{:x}",
                Sha256::digest(std::fs::read(project.join("src/main.rs")).unwrap())
            )
        );
        let lookup = index
            .lookup(&SourceAnchor {
                path: ProjectPath::try_from(anchor.path.clone()).unwrap(),
                symbol: anchor.symbol.clone(),
                expected_sha256: main_rs.sha256.clone(),
                marker: anchor.marker.clone(),
            })
            .unwrap();
        let snippet = &lookup.snippets[0];
        assert_eq!(snippet.confidence, "explicit_marker");
        assert!(
            snippet.text.contains("overlay-title-anchor")
                && snippet.text.contains("let title_text ="),
            "{}",
            snippet.text
        );

        // Crash the worker: its requests fail, and the next generation renders again.
        let pid = worker.process_id().expect("worker process id");
        kill_process(pid);
        assert!(worker.frame(1, 8, 1.0).is_err());
        generations.push(hello.identity.worker_generation);
        drop(worker);
    }
    assert_eq!(generations, [41, 42]);
    manager.terminate_all(Duration::from_millis(300));
    assert_eq!(manager.active_count(), 0);
}
