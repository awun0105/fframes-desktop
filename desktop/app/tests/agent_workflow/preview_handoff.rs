//! Handing a committed revision to the preview.

use super::*;

#[test]
fn a_committed_revision_is_handed_to_the_preview_and_a_failed_handoff_is_exposed_honestly() {
    let owner = ProcessTreeManager::new();
    let sink = Arc::new(Sink {
        coordinator: Mutex::new(Some(Arc::new(PreviewCoordinator::new(owner.sub_manager())))),
        fail: AtomicBool::new(false),
        adopted: Mutex::new(Vec::new()),
    });
    let w = World::with(Options {
        handoff: Some(sink.clone()),
        ..Options::default()
    });
    w.submit(&good("handed off"));
    let task = w.wait_task(None);
    assert_eq!(task.phase, TaskPhase::Accepted, "{:?}", task.error);
    let published = w.ctl().lock().state().source().as_str().to_owned();
    assert_eq!(
        w.snap().handoff.as_ref().unwrap().state,
        HandoffState::Adopted
    );
    let coordinator = sink.coordinator.lock().clone().unwrap();
    let ready = wait_ready(&coordinator);
    assert_eq!(ready.identity().source_revision, published);
    assert_eq!(
        sink.adopted.lock().as_slice(),
        std::slice::from_ref(&published)
    );
    // The UI reports what it displays; the awaiting label clears.
    w.wf()
        .preview_displayed("not-the-published-revision")
        .unwrap();
    w.wf().preview_displayed(&published).unwrap();
    w.wait("the displayed preview", |s| {
        s.handoff
            .as_ref()
            .is_some_and(|h| h.state == HandoffState::Displayed)
    });
    // A failing handoff keeps the old preview and accepts the source anyway.
    sink.fail.store(true, Ordering::SeqCst);
    w.submit(&good("not adopted"));
    let second = w.wait_task(Some(&task.id));
    assert_eq!(second.phase, TaskPhase::Accepted);
    assert_eq!(w.source("src/lib.rs"), "// not adopted\n");
    assert_eq!(
        w.snap().handoff.as_ref().unwrap().state,
        HandoffState::AwaitingPreview {
            reason: Some("preview worker refused".into())
        },
        "accepted-awaiting-preview is exposed with the reason"
    );
    assert_eq!(sink.adopted.lock().len(), 1);
    // A sink that adopts later (on another thread) reports the outcome afterwards.
    let published_two = w.ctl().lock().state().source().as_str().to_owned();
    w.wf()
        .report_handoff("some-other-revision", Ok(()))
        .unwrap();
    w.wf().report_handoff(&published_two, Ok(())).unwrap();
    w.wait("the reported adoption", |s| {
        s.handoff
            .as_ref()
            .is_some_and(|h| h.state == HandoffState::Adopted)
    });
    // Dropping the coordinator reaps the adopted worker it owns.
    drop(ready);
    let owned = sink.coordinator.lock().take().unwrap();
    drop(coordinator);
    owned.close();
    drop(owned);
    w.assert_clean();
}
