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
