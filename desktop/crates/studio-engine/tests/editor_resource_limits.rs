//! The editor's source index and frame metadata at their limits: every file is indexed, a
//! frame with the most objects fits the metadata budget, and the topmost object wins a hit.

use fframes_studio_protocol::{
    EditorFrameMetadata, EditorFrameStatus, EditorGeometrySupport, EditorObjectGeometry,
    EditorObjectIdentity, MAX_EDITOR_METADATA_BYTES, MAX_EDITOR_OBJECTS_PER_FRAME, PreviewIdentity,
    Rect,
};
use sha2::{Digest, Sha256};
use studio_engine::{CanvasRect, CanvasViewport, DisplayedFrameTag, hit_test};
use studio_project::{
    ProjectPath, SourceRevision,
    revision::FileKind,
    source_index::{SourceAnchor, SourceIndex, SourceIndexInput},
};

#[test]
fn a_256_file_project_is_indexed_in_full_and_an_anchor_resolves_by_its_marker() {
    let target_source = "pub fn render_title() { /* title-anchor */ }\n";
    let source_inputs: Vec<_> = (0..256)
        .map(|index| {
            let text = if index == 0 {
                target_source.to_owned()
            } else {
                format!("pub fn helper_{index}() {{}}\n")
            };
            let bytes = text.into_bytes();
            SourceIndexInput {
                path: ProjectPath::try_from(format!("src/module_{index:03}.rs")).unwrap(),
                kind: FileKind::Rust,
                expected_sha256: format!("{:x}", Sha256::digest(&bytes)),
                expected_size: bytes.len() as u64,
                bytes,
            }
        })
        .collect();
    let source_bytes: usize = source_inputs.iter().map(|input| input.bytes.len()).sum();
    let revision: SourceRevision = "f".repeat(64).try_into().unwrap();
    let index = SourceIndex::build(revision, source_inputs, &|| false).unwrap();
    assert_eq!(index.indexed_files, 256);
    assert_eq!(index.indexed_bytes, source_bytes);

    let anchor = SourceAnchor {
        path: ProjectPath::try_from("src/module_000.rs".to_owned()).unwrap(),
        symbol: "render_title".into(),
        expected_sha256: format!("{:x}", Sha256::digest(target_source.as_bytes())),
        marker: Some("title-anchor".into()),
    };
    let result = index.lookup(&anchor).unwrap();
    assert_eq!(result.snippets[0].confidence, "explicit_marker");
}

#[test]
fn a_frame_with_the_most_objects_fits_the_metadata_budget_and_hits_the_topmost() {
    let width = 1920;
    let height = 1080;
    let metadata = EditorFrameMetadata {
        frame_index: 0,
        seek_serial: 1,
        video_width: width,
        video_height: height,
        editor_index_digest: "a".repeat(64),
        frame_geometry_digest: "b".repeat(64),
        status: EditorFrameStatus::Supported,
        reason: None,
        objects: (0..MAX_EDITOR_OBJECTS_PER_FRAME)
            .map(|index| EditorObjectGeometry {
                identity: EditorObjectIdentity {
                    scene_instance_key: "s".into(),
                    component_key: "c".into(),
                    object_key: format!("o{index:04}"),
                    repeat_key: "r".into(),
                },
                parent: None,
                source_anchor: None,
                style_tokens: Vec::new(),
                bounds: Rect {
                    x: 0.0,
                    y: 0.0,
                    width: width as f32,
                    height: height as f32,
                },
                paint_order: index as u32,
                support: EditorGeometrySupport::ExactBounds,
            })
            .collect(),
    };
    // The envelope around the metadata needs room too.
    let metadata_bytes = serde_json::to_vec(&metadata).unwrap().len();
    assert!(metadata_bytes <= MAX_EDITOR_METADATA_BYTES - 4096);
    let tag = DisplayedFrameTag::from_metadata(
        PreviewIdentity {
            project_id: "resource-limits".into(),
            open_session: "session".into(),
            source_revision: "c".repeat(64),
            worker_generation: 1,
        },
        &metadata,
    )
    .unwrap();
    let viewport = CanvasViewport::new(
        CanvasRect {
            x: 0.0,
            y: 0.0,
            width: 960.0,
            height: 540.0,
        },
        width,
        height,
    )
    .unwrap();
    let selected = hit_test(&metadata, &tag, &viewport, 480.0, 270.0)
        .unwrap()
        .unwrap();
    assert_eq!(
        selected.identity.object_key,
        format!("o{:04}", metadata.objects.len() - 1)
    );
}
