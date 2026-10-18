//! The review window against a stand-in for the engine: what it calls, what it refuses to call,
//! and how it reacts to the snapshots that answer.

use std::sync::Mutex;

use gpui::{Entity, Modifiers, TestAppContext, VisualTestContext};
use studio_engine::candidate_validation::{ChangeKind, FileText};

use super::*;

#[derive(Default)]
struct Fake {
    calls: Mutex<Vec<String>>,
    /// Paths the candidate currently lists.
    paths: Mutex<Vec<&'static str>>,
}

impl Fake {
    fn with(paths: &[&'static str]) -> Arc<Self> {
        let fake = Arc::new(Self::default());
        *fake.paths.lock().unwrap() = paths.to_vec();
        fake
    }

    fn calls(&self) -> Vec<String> {
        self.calls.lock().unwrap().clone()
    }

    fn note(&self, call: String) {
        self.calls.lock().unwrap().push(call);
    }

    fn listing(&self) -> Vec<EngineDiff> {
        self.paths
            .lock()
            .unwrap()
            .iter()
            .map(|path| EngineDiff {
                path: (*path).into(),
                change: ChangeKind::Modified,
                before: FileText::Text("old\n".into()),
                after: FileText::Text("new\n".into()),
            })
            .collect()
    }
}

impl ReviewBackend for Fake {
    fn candidate_diffs(&self) -> Result<Vec<EngineDiff>, String> {
        self.note("candidate_diffs".into());
        Ok(self.listing())
    }

    fn revision_diffs(&self, revision: &str) -> Result<Vec<EngineDiff>, String> {
        self.note(format!("revision_diffs {revision}"));
        Ok(self.listing())
    }

    fn reject_files(&self, paths: &[String]) -> Result<(), String> {
        self.note(format!("reject_files {}", paths.join(",")));
        Ok(())
    }

    fn revert_path(&self, revision: &str, path: &str) -> Result<(), String> {
        self.note(format!("revert_path {revision} {path}"));
        Ok(())
    }

    fn discard(&self) -> Result<(), String> {
        self.note("discard".into());
        Ok(())
    }
}

fn reviewing(candidate: &str) -> Facts {
    Facts {
        phase: Some(TaskPhase::AwaitingReview),
        candidate: Some(candidate.into()),
        task_active: true,
        ..Facts::default()
    }
}

fn applied() -> Facts {
    Facts::default()
}

fn open<'a>(
    cx: &'a mut TestAppContext,
    backend: &Arc<Fake>,
    target: ReviewTarget,
    facts: Facts,
) -> (
    Entity<ReviewHost>,
    &'a mut VisualTestContext,
    std::rc::Rc<std::cell::RefCell<Vec<ReviewHostEvent>>>,
) {
    let backend: Arc<dyn ReviewBackend> = backend.clone();
    let (host, cx) = cx.add_window_view(|_, cx| ReviewHost::new(target, backend, facts, cx));
    let events = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let sink = events.clone();
    cx.update(|_, cx| {
        cx.subscribe(&host, move |_, event: &ReviewHostEvent, _| {
            sink.borrow_mut().push(event.clone())
        })
        .detach()
    });
    cx.run_until_parked();
    (host, cx, events)
}

fn click(cx: &mut VisualTestContext, selector: &'static str) {
    let bounds = cx
        .debug_bounds(selector)
        .unwrap_or_else(|| panic!("{selector} was not laid out"));
    cx.simulate_click(bounds.center(), Modifiers::none());
    cx.run_until_parked();
}

fn listed(host: &Entity<ReviewHost>, cx: &mut VisualTestContext) -> Vec<String> {
    host.read_with(cx, |host, cx| {
        host.changes
            .read(cx)
            .files
            .iter()
            .map(|file| file.path.clone())
            .collect()
    })
}

fn busy(host: &Entity<ReviewHost>, cx: &mut VisualTestContext) -> bool {
    host.read_with(cx, |host, cx| host.changes.read(cx).busy())
}

fn status(host: &Entity<ReviewHost>, cx: &mut VisualTestContext) -> Option<String> {
    host.read_with(cx, |host, _| host.status().map(str::to_owned))
}

#[gpui::test]
fn the_candidate_is_read_once_it_awaits_review(cx: &mut TestAppContext) {
    let backend = Fake::with(&["a.rs", "b.rs"]);
    let (host, cx, _) = open(cx, &backend, ReviewTarget::Candidate, reviewing("t1/c1"));
    assert_eq!(listed(&host, cx), ["a.rs", "b.rs"]);
    assert_eq!(backend.calls(), ["candidate_diffs"]);

    // The same candidate again is not read again.
    host.update(cx, |host, cx| host.on_facts(reviewing("t1/c1"), cx));
    cx.run_until_parked();
    assert_eq!(backend.calls(), ["candidate_diffs"]);
}

#[gpui::test]
fn reject_sends_the_toggled_off_paths_and_never_an_empty_one(cx: &mut TestAppContext) {
    let backend = Fake::with(&["a.rs", "b.rs"]);
    let (host, cx, _) = open(cx, &backend, ReviewTarget::Candidate, reviewing("t1/c1"));
    let changes = host.read_with(cx, |host, _| host.changes.clone());

    // An empty path names no file: nothing reaches the engine.
    changes.update(cx, |_, cx| {
        cx.emit(ChangesEvent::Reject(vec![String::new()]));
    });
    cx.run_until_parked();
    assert_eq!(backend.calls(), ["candidate_diffs"]);
    assert!(!busy(&host, cx));

    click(cx, "changes-toggle-1");
    click(cx, "changes-reject");
    assert_eq!(backend.calls(), ["candidate_diffs", "reject_files b.rs"]);
    // Until the engine answers, the viewer says so and takes no second action.
    assert!(busy(&host, cx));
    assert_eq!(
        status(&host, cx).as_deref(),
        Some("Validating the remaining files…")
    );
}

#[gpui::test]
fn a_replaced_candidate_is_read_again_after_validation(cx: &mut TestAppContext) {
    let backend = Fake::with(&["a.rs", "b.rs"]);
    let (host, cx, _) = open(cx, &backend, ReviewTarget::Candidate, reviewing("t1/c1"));
    click(cx, "changes-toggle-1");
    click(cx, "changes-reject");

    // The engine validates the rest: there is no candidate to read meanwhile.
    let validating = Facts {
        phase: Some(TaskPhase::Validating),
        candidate: None,
        task_active: true,
        ..Facts::default()
    };
    host.update(cx, |host, cx| host.on_facts(validating, cx));
    cx.run_until_parked();
    assert!(busy(&host, cx));
    assert_eq!(backend.calls().len(), 2, "nothing is read while validating");

    *backend.paths.lock().unwrap() = vec!["a.rs"];
    host.update(cx, |host, cx| host.on_facts(reviewing("t1/c2"), cx));
    cx.run_until_parked();
    assert_eq!(listed(&host, cx), ["a.rs"]);
    assert!(!busy(&host, cx));
    assert!(status(&host, cx).unwrap().contains("passed validation"));
}

#[gpui::test]
fn a_refused_reject_keeps_the_candidate_and_shows_why(cx: &mut TestAppContext) {
    let backend = Fake::with(&["a.rs", "b.rs"]);
    let (host, cx, _) = open(cx, &backend, ReviewTarget::Candidate, reviewing("t1/c1"));
    click(cx, "changes-toggle-1");
    click(cx, "changes-reject");

    let refused = Facts {
        newest_row: 7,
        refusal: Some((7, "Cannot reject: the rest does not build".into())),
        ..reviewing("t1/c1")
    };
    host.update(cx, |host, cx| host.on_facts(refused, cx));
    cx.run_until_parked();
    assert!(!busy(&host, cx));
    assert_eq!(
        status(&host, cx).as_deref(),
        Some("Cannot reject: the rest does not build")
    );
    assert_eq!(listed(&host, cx), ["a.rs", "b.rs"]);
}

#[gpui::test]
fn rejecting_every_file_asks_first_and_then_discards(cx: &mut TestAppContext) {
    let backend = Fake::with(&["a.rs", "b.rs"]);
    let (host, cx, _) = open(cx, &backend, ReviewTarget::Candidate, reviewing("t1/c1"));
    click(cx, "changes-toggle-0");
    click(cx, "changes-toggle-1");
    click(cx, "changes-reject");

    // Asked, not done.
    assert_eq!(backend.calls(), ["candidate_diffs"]);
    let changes = host.read_with(cx, |host, _| host.changes.clone());
    assert!(changes.read_with(cx, |changes, _| changes.confirming_discard()));

    click(cx, "changes-discard-confirm");
    assert_eq!(backend.calls(), ["candidate_diffs", "discard"]);
}

#[gpui::test]
fn declining_the_discard_changes_nothing(cx: &mut TestAppContext) {
    let backend = Fake::with(&["a.rs"]);
    let (host, cx, _) = open(cx, &backend, ReviewTarget::Candidate, reviewing("t1/c1"));
    click(cx, "changes-toggle-0");
    click(cx, "changes-reject");
    click(cx, "changes-discard-cancel");
    let changes = host.read_with(cx, |host, _| host.changes.clone());
    assert!(!changes.read_with(cx, |changes, _| changes.confirming_discard()));
    assert_eq!(backend.calls(), ["candidate_diffs"]);
}

#[gpui::test]
fn an_answer_for_an_older_read_is_dropped(cx: &mut TestAppContext) {
    let backend = Fake::with(&["a.rs", "b.rs"]);
    let (host, cx, _) = open(cx, &backend, ReviewTarget::Candidate, reviewing("t1/c1"));
    let old_serial = host.read_with(cx, |host, _| host.serial);

    // The candidate changes, which starts a newer read.
    *backend.paths.lock().unwrap() = vec!["only.rs"];
    host.update(cx, |host, cx| host.on_facts(reviewing("t1/c2"), cx));
    cx.run_until_parked();
    assert_eq!(listed(&host, cx), ["only.rs"]);

    // The answer of the first read arrives late.
    let late = Fake::with(&["stale.rs"]).listing();
    host.update(cx, |host, cx| {
        host.on_loaded(
            old_serial,
            Some("t1/c1".into()),
            Ok(Read::from_diffs(late)),
            cx,
        )
    });
    assert_eq!(listed(&host, cx), ["only.rs"]);
}

#[gpui::test]
fn an_answer_for_a_candidate_that_is_gone_is_dropped(cx: &mut TestAppContext) {
    let backend = Fake::with(&["a.rs"]);
    let (host, cx, _) = open(cx, &backend, ReviewTarget::Candidate, reviewing("t1/c1"));
    host.update(cx, |host, cx| host.on_facts(reviewing("t1/c2"), cx));
    cx.run_until_parked();
    let serial = host.read_with(cx, |host, _| host.serial);

    // Newest read, but of a candidate that is not the current one.
    let wrong = Fake::with(&["wrong.rs"]).listing();
    host.update(cx, |host, cx| {
        host.on_loaded(
            serial,
            Some("t1/c1".into()),
            Ok(Read::from_diffs(wrong)),
            cx,
        )
    });
    cx.run_until_parked();
    assert!(!listed(&host, cx).contains(&"wrong.rs".to_owned()));
}

#[gpui::test]
fn the_window_closes_when_the_candidate_is_gone(cx: &mut TestAppContext) {
    let backend = Fake::with(&["a.rs"]);
    let (host, cx, events) = open(cx, &backend, ReviewTarget::Candidate, reviewing("t1/c1"));
    let ended = Facts {
        phase: Some(TaskPhase::Cancelled),
        task_active: false,
        ..Facts::default()
    };
    host.update(cx, |host, cx| host.on_facts(ended, cx));
    assert_eq!(*events.borrow(), [ReviewHostEvent::Close]);
}

#[gpui::test]
fn revert_asks_the_engine_and_shows_its_refusal(cx: &mut TestAppContext) {
    let backend = Fake::with(&["a.rs", "b.rs"]);
    let (host, cx, _) = open(
        cx,
        &backend,
        ReviewTarget::Revision("rev1".into()),
        applied(),
    );
    assert_eq!(backend.calls(), ["revision_diffs rev1"]);

    click(cx, "changes-revert-1");
    assert_eq!(
        backend.calls(),
        ["revision_diffs rev1", "revert_path rev1 b.rs"]
    );
    assert!(busy(&host, cx));

    let refused = Facts {
        newest_row: 9,
        refusal: Some((9, "Cannot revert: b.rs changed after this edit".into())),
        ..applied()
    };
    host.update(cx, |host, cx| host.on_facts(refused, cx));
    cx.run_until_parked();
    assert!(!busy(&host, cx));
    assert_eq!(
        status(&host, cx).as_deref(),
        Some("Cannot revert: b.rs changed after this edit")
    );
    assert!(cx.debug_bounds("review-status").is_some());
}

#[gpui::test]
fn a_reverted_file_shows_as_reverted_without_a_revert_button(cx: &mut TestAppContext) {
    let backend = Fake::with(&["a.rs", "b.rs"]);
    let (host, cx, _) = open(
        cx,
        &backend,
        ReviewTarget::Revision("rev1".into()),
        applied(),
    );
    click(cx, "changes-revert-0");
    let done = Facts {
        reverted: vec!["a.rs".into()],
        ..applied()
    };
    host.update(cx, |host, cx| host.on_facts(done, cx));
    cx.run_until_parked();
    assert_eq!(status(&host, cx).as_deref(), Some("Reverted a.rs."));
    assert!(cx.debug_bounds("changes-reverted-0").is_some());
    assert!(cx.debug_bounds("changes-revert-0").is_none());
    assert!(cx.debug_bounds("changes-revert-1").is_some());
}

#[gpui::test]
fn revert_is_unavailable_while_a_task_is_active(cx: &mut TestAppContext) {
    let backend = Fake::with(&["a.rs"]);
    let working = Facts {
        phase: Some(TaskPhase::Editing),
        task_active: true,
        ..Facts::default()
    };
    let (host, cx, _) = open(cx, &backend, ReviewTarget::Revision("rev1".into()), working);
    assert!(busy(&host, cx));
    click(cx, "changes-revert-0");
    assert_eq!(backend.calls(), ["revision_diffs rev1"]);
}

#[gpui::test]
fn a_stopped_reject_says_the_candidate_is_unchanged(cx: &mut TestAppContext) {
    let backend = Fake::with(&["a.rs", "b.rs"]);
    let (host, cx, _) = open(cx, &backend, ReviewTarget::Candidate, reviewing("t1/c1"));
    click(cx, "changes-toggle-1");
    click(cx, "changes-reject");

    let validating = Facts {
        phase: Some(TaskPhase::Validating),
        task_active: true,
        newest_row: 1,
        ..Facts::default()
    };
    host.update(cx, |host, cx| host.on_facts(validating, cx));
    // Stopped: back in review of the same candidate, with only an informational row.
    let back = Facts {
        newest_row: 2,
        ..reviewing("t1/c1")
    };
    host.update(cx, |host, cx| host.on_facts(back, cx));
    cx.run_until_parked();
    assert!(!busy(&host, cx));
    assert_eq!(
        status(&host, cx).as_deref(),
        Some("The reject did not finish; the candidate is unchanged.")
    );
    assert_eq!(listed(&host, cx), ["a.rs", "b.rs"]);
}
