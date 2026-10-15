//! Undo: validated, published and handed off like an Apply.

use super::*;

#[test]
fn undo_is_validated_published_and_handed_off_like_an_apply() {
    let w = World::new();
    let original = w.source("src/lib.rs");
    w.submit(&good("to undo"));
    w.wait_task(None);
    assert_eq!(w.source("src/lib.rs"), "// to undo\n");
    w.wf().undo(None).unwrap();
    let s = w.wait("the Undo", |s| {
        s.history.len() == 2 && matches!(s.undo, UndoView::Unavailable { .. })
    });
    assert_eq!(w.source("src/lib.rs"), original);
    assert_eq!(s.history[1].kind, "Undo");
    assert_eq!(
        s.history[1].undoes.as_deref(),
        Some(s.history[0].id.as_str())
    );
    assert_eq!(s.handoff.as_ref().unwrap().kind, HandoffKind::Undo);
    w.assert_clean();
}

#[test]
fn undo_without_an_accepted_edit_explains_itself() {
    let w = World::new();
    assert!(
        matches!(&w.snap().undo, UndoView::Unavailable { reason } if reason.contains("No agent edit"))
    );
    w.wf().undo(None).unwrap();
    w.wait("the refusal", |s| has_error(s, "undo_unavailable"));
}
