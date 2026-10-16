//! The viewer in a test window: layouts, the mode switch and the events it emits.

use gpui::{Entity, Modifiers, TestAppContext, VisualTestContext};

use super::*;
type Log<T> = std::rc::Rc<std::cell::RefCell<Vec<T>>>;

fn log<T>() -> Log<T> {
    Log::default()
}

fn files() -> Vec<DiffFile> {
    vec![
        DiffFile::new(
            "src/lib.rs",
            Some("fn a() {}\nfn b() {}\n".into()),
            Some("fn a() {}\nfn c() {}\n".into()),
        ),
        DiffFile::new("notes.md", Some("same\n".into()), Some("same\n".into())),
        DiffFile::new("new.toml", None, Some("[a]\nb = 1\n".into())),
    ]
}

fn open(
    cx: &mut TestAppContext,
    files: Vec<DiffFile>,
) -> (Entity<Changes>, &mut VisualTestContext, Log<ChangesEvent>) {
    let (view, cx) = cx.add_window_view(|_, _| Changes::new(files));
    let events = log();
    let sink = events.clone();
    cx.update(|_, cx| {
        cx.subscribe(&view, move |_, event: &ChangesEvent, _| {
            sink.borrow_mut().push(event.clone())
        })
        .detach()
    });
    cx.run_until_parked();
    (view, cx, events)
}

fn click(cx: &mut VisualTestContext, selector: &'static str) {
    let bounds = cx
        .debug_bounds(selector)
        .unwrap_or_else(|| panic!("{selector} was not laid out"));
    cx.simulate_click(bounds.center(), Modifiers::none());
    cx.run_until_parked();
}

fn laid_out(cx: &mut VisualTestContext, selector: &'static str) -> bool {
    cx.debug_bounds(selector).is_some()
}

#[gpui::test]
fn two_files_render_as_a_unified_diff(cx: &mut TestAppContext) {
    let (view, cx, _) = open(cx, files());
    view.read_with(cx, |changes, _| {
        assert_eq!(changes.mode(), DiffMode::Unified);
        assert_eq!(changes.files.len(), 3);
        assert_eq!(changes.files[0].additions, 1);
        assert_eq!(changes.files[0].deletions, 1);
        assert_eq!(changes.files[2].additions, 2);
    });
    // The first line row of each diffed file is laid out.
    let first_lines = view.read_with(cx, |changes, _| {
        [0, 2].map(|file| {
            changes
                .rows
                .iter()
                .position(|row| matches!(row, model::DiffRow::Line { file: f, .. } if *f == file))
                .expect("file has line rows")
        })
    });
    for ix in first_lines {
        let selector: &'static str = Box::leak(format!("changes-line-{ix}").into_boxed_str());
        assert!(laid_out(cx, selector), "{selector} is on screen");
    }
    assert!(!laid_out(cx, "changes-split-3"));
}

#[gpui::test]
fn wrapping_is_on_by_default_and_the_toolbar_turns_it_off(cx: &mut TestAppContext) {
    let (view, cx, _) = open(cx, files());
    assert!(view.read_with(cx, |changes, _| changes.wrap()));
    click(cx, "changes-wrap");
    assert!(!view.read_with(cx, |changes, _| changes.wrap()));
    assert!(
        laid_out(cx, "changes-line-3"),
        "rows still render unwrapped"
    );
}

#[gpui::test]
fn the_mode_toggle_switches_to_split_rows(cx: &mut TestAppContext) {
    let (view, cx, _) = open(cx, files());
    let bounds = cx.debug_bounds("changes-mode").expect("toolbar laid out");
    // The right half of the control is the Split segment.
    let point = gpui::point(bounds.origin.x + bounds.size.width * 0.8, bounds.center().y);
    cx.simulate_click(point, Modifiers::none());
    cx.run_until_parked();
    view.read_with(cx, |changes, _| assert_eq!(changes.mode(), DiffMode::Split));
    assert!(laid_out(cx, "changes-split-3"), "a split row is on screen");
    assert!(!laid_out(cx, "changes-line-3"));
}

#[gpui::test]
fn toggling_a_file_off_emits_its_path_and_reject_lists_it(cx: &mut TestAppContext) {
    let (view, cx, events) = open(cx, files());
    click(cx, "changes-toggle-2");
    assert_eq!(
        events.borrow().as_slice(),
        [ChangesEvent::FileToggled {
            path: "new.toml".into(),
            accepted: false
        }]
    );
    assert_eq!(view.read_with(cx, |c, _| c.rejected()), ["new.toml"]);
    click(cx, "changes-reject");
    assert_eq!(
        events.borrow().last(),
        Some(&ChangesEvent::Reject(vec!["new.toml".into()]))
    );
    click(cx, "changes-toggle-2");
    assert_eq!(
        events.borrow().last(),
        Some(&ChangesEvent::FileToggled {
            path: "new.toml".into(),
            accepted: true
        })
    );
    assert!(view.read_with(cx, |c, _| c.rejected().is_empty()));
}

#[gpui::test]
fn a_file_without_textual_change_says_no_changes(cx: &mut TestAppContext) {
    let (view, cx, _) = open(
        cx,
        vec![DiffFile::new(
            "a.md",
            Some("x\n".into()),
            Some("x\n".into()),
        )],
    );
    view.read_with(cx, |changes, _| {
        assert_eq!(changes.files[0].notices, ["No changes"]);
        assert!(changes.files[0].hunks.is_empty());
    });
    assert!(laid_out(cx, "changes-notice-0-0"));
}

#[gpui::test]
fn close_and_revert_are_events_not_actions(cx: &mut TestAppContext) {
    let (view, cx, events) = open(cx, files());
    click(cx, "changes-close");
    assert_eq!(events.borrow().as_slice(), [ChangesEvent::Close]);
    view.update(cx, |changes, cx| changes.set_review(Review::Applied, cx));
    cx.run_until_parked();
    assert!(
        !laid_out(cx, "changes-reject"),
        "applied files cannot be rejected"
    );
    click(cx, "changes-revert-1");
    assert_eq!(
        events.borrow().last(),
        Some(&ChangesEvent::RevertFile("notes.md".into()))
    );
    assert_eq!(
        view.read_with(cx, |c, _| c.collapsed.len()),
        0,
        "the revert click does not also collapse the file"
    );
}

#[gpui::test]
fn collapsing_a_file_keeps_only_its_header_row(cx: &mut TestAppContext) {
    let (view, cx, _) = open(cx, files());
    let before = view.read_with(cx, |c, _| c.rows.len());
    view.update(cx, |changes, cx| changes.toggle_collapsed("src/lib.rs", cx));
    let after = view.read_with(cx, |c, _| c.rows.len());
    assert!(after < before);
    view.update(cx, |changes, cx| changes.set_all_collapsed(true, cx));
    assert_eq!(view.read_with(cx, |c, _| c.rows.len()), 3);
}

#[gpui::test]
fn replacing_the_files_keeps_choices_for_paths_still_there(cx: &mut TestAppContext) {
    let (view, cx, _) = open(cx, files());
    view.update(cx, |changes, cx| {
        changes.set_accepted("new.toml", false, cx);
        changes.set_accepted("src/lib.rs", false, cx);
    });
    view.update(cx, |changes, cx| {
        changes.set_files(files().into_iter().skip(1).collect(), cx)
    });
    assert_eq!(view.read_with(cx, |c, _| c.rejected()), ["new.toml"]);
}

#[gpui::test]
fn every_not_shown_reason_has_its_notice_and_no_line_counts(cx: &mut TestAppContext) {
    let reasons = [
        (NotShown::Binary, "Binary file"),
        (NotShown::TooLarge, "too large"),
        (NotShown::Budget, "Too much text"),
        (NotShown::Unreadable, "could not be read"),
    ];
    let hidden = reasons
        .iter()
        .enumerate()
        .map(|(ix, (reason, _))| {
            let mut file = DiffFile::new(format!("f{ix}.bin"), None, Some(String::new()));
            file.not_shown = Some(*reason);
            file
        })
        .collect();
    let (view, cx, _) = open(cx, hidden);
    view.read_with(cx, |changes, _| {
        for (ix, (_, expected)) in reasons.iter().enumerate() {
            assert!(
                changes.files[ix]
                    .notices
                    .iter()
                    .any(|n| n.contains(expected)),
                "{:?}",
                changes.files[ix].notices
            );
        }
    });
}

#[gpui::test]
fn files_the_listing_left_out_are_a_notice_and_never_a_row(cx: &mut TestAppContext) {
    let (view, cx, _) = open(cx, files());
    assert!(!laid_out(cx, "changes-more"));
    view.update(cx, |changes, cx| changes.set_more(5, cx));
    cx.run_until_parked();
    assert!(laid_out(cx, "changes-more"));
    assert_eq!(strips::more_files_text(5), "5 more files not shown");
    assert_eq!(strips::more_files_text(1), "1 more file not shown");
    // Still three rows of files: the notice adds none.
    assert_eq!(view.read_with(cx, |c, _| c.files.len()), 3);
}

#[gpui::test]
fn rejecting_every_listed_file_asks_before_discarding(cx: &mut TestAppContext) {
    let (view, cx, events) = open(cx, files());
    for ix in 0..3 {
        let selector: &'static str =
            ["changes-toggle-0", "changes-toggle-1", "changes-toggle-2"][ix];
        click(cx, selector);
    }
    assert!(view.read_with(cx, |c, _| c.rejects_everything()));
    click(cx, "changes-reject");
    assert!(
        !events
            .borrow()
            .iter()
            .any(|e| matches!(e, ChangesEvent::Reject(_) | ChangesEvent::Discard)),
        "asking is not discarding"
    );
    assert!(view.read_with(cx, |c, _| c.confirming_discard()));
    assert!(laid_out(cx, "changes-discard-confirm"));
    click(cx, "changes-discard-confirm");
    assert_eq!(events.borrow().last(), Some(&ChangesEvent::Discard));
    assert!(!view.read_with(cx, |c, _| c.confirming_discard()));
}

#[gpui::test]
fn not_every_file_is_rejected_when_some_are_not_listed(cx: &mut TestAppContext) {
    let (view, cx, events) = open(cx, files());
    view.update(cx, |changes, cx| changes.set_more(2, cx));
    for selector in ["changes-toggle-0", "changes-toggle-1", "changes-toggle-2"] {
        click(cx, selector);
    }
    assert!(!view.read_with(cx, |c, _| c.rejects_everything()));
    click(cx, "changes-reject");
    assert!(matches!(
        events.borrow().last(),
        Some(ChangesEvent::Reject(paths)) if paths.len() == 3
    ));
}

#[gpui::test]
fn while_busy_nothing_can_be_rejected_or_toggled(cx: &mut TestAppContext) {
    let (view, cx, events) = open(cx, files());
    click(cx, "changes-toggle-2");
    view.update(cx, |changes, cx| changes.set_busy(true, cx));
    cx.run_until_parked();
    let before = events.borrow().len();
    click(cx, "changes-reject");
    click(cx, "changes-toggle-0");
    assert_eq!(events.borrow().len(), before);
    assert_eq!(view.read_with(cx, |c, _| c.rejected()), ["new.toml"]);
}

#[gpui::test]
fn a_reverted_file_has_a_label_and_no_revert_button(cx: &mut TestAppContext) {
    let (view, cx, events) = open(cx, files());
    view.update(cx, |changes, cx| {
        changes.set_review(Review::Applied, cx);
        changes.set_reverted(["notes.md".to_owned()].into_iter().collect(), cx);
    });
    cx.run_until_parked();
    assert!(laid_out(cx, "changes-reverted-1"));
    assert!(!laid_out(cx, "changes-revert-1"));
    assert!(laid_out(cx, "changes-revert-0"));
    assert!(events.borrow().is_empty());
}
