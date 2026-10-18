//! Rendering tests of the markdown row in a gpui test window.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    Bounds, Context, Entity, IntoElement, Modifiers, ParentElement, Pixels, Render, Styled,
    TestAppContext, VisualTestContext, Window, div, point, px,
};
use studio_engine::agent::{Row, RowId, RowKind};

use super::links::LinkEffect;
use super::render::Probe;
use super::{MarkdownRow, Voice};
use crate::kit::{bind_selectable_text_keys, selected_text};

type Effects = Rc<RefCell<Vec<LinkEffect>>>;

struct Page {
    row: Arc<Row>,
    root: Option<PathBuf>,
    probe: Rc<RefCell<Probe>>,
    effects: Effects,
}

fn row(text: &str, streaming: bool) -> Arc<Row> {
    Arc::new(Row {
        id: RowId(7),
        task: None,
        at_unix: 0,
        kind: RowKind::Agent {
            text: text.to_owned(),
            turn: 1,
            streaming,
        },
    })
}

impl Render for Page {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        let effects = self.effects.clone();
        div().w(px(480.)).child(
            MarkdownRow::new(&self.row, Voice::Answer)
                .project_root(self.root.clone())
                .link_sink(Rc::new(move |effect, _| {
                    effects.borrow_mut().push(effect.clone())
                }))
                .probe(self.probe.clone()),
        )
    }
}

fn page<'a>(
    cx: &'a mut TestAppContext,
    text: &str,
    root: Option<PathBuf>,
) -> (Entity<Page>, &'a mut VisualTestContext) {
    page_streaming(cx, text, root, false)
}

fn page_streaming<'a>(
    cx: &'a mut TestAppContext,
    text: &str,
    root: Option<PathBuf>,
    streaming: bool,
) -> (Entity<Page>, &'a mut VisualTestContext) {
    cx.update(bind_selectable_text_keys);
    let (page, cx) = cx.add_window_view(|_, _| Page {
        row: row(text, streaming),
        root,
        probe: Rc::default(),
        effects: Rc::default(),
    });
    cx.run_until_parked();
    (page, cx)
}

fn show(page: &Entity<Page>, cx: &mut VisualTestContext, text: &str, streaming: bool) {
    page.update(cx, |page, cx| {
        page.row = row(text, streaming);
        cx.notify();
    });
    cx.run_until_parked();
}

fn probe(page: &Entity<Page>, cx: &VisualTestContext) -> Probe {
    page.read_with(cx, |page, _| page.probe.borrow().clone())
}

fn effects(page: &Entity<Page>, cx: &VisualTestContext) -> Vec<LinkEffect> {
    page.read_with(cx, |page, _| page.effects.borrow().clone())
}

fn bounds(cx: &mut VisualTestContext, name: &'static str) -> Bounds<Pixels> {
    cx.debug_bounds(name)
        .unwrap_or_else(|| panic!("{name} was not laid out"))
}

fn click_left_of(cx: &mut VisualTestContext, name: &'static str) {
    let b = bounds(cx, name);
    cx.simulate_click(
        point(b.left() + px(3.), b.top() + px(6.)),
        Modifiers::none(),
    );
}

const COPY_KEY: &str = if cfg!(target_os = "macos") {
    "cmd-c"
} else {
    "ctrl-c"
};

const REPLY: &str = "# Title\n\nIntro paragraph.\n\n- one\n- two\n\n| name | value |\n|------|-------|\n| a | 1 |\n| b | 2 |\n\n```rust\nfn main() {}\n```\n";

#[gpui::test]
fn a_reply_renders_one_element_per_block_in_order(cx: &mut TestAppContext) {
    let (page, cx) = page(cx, REPLY, None);
    let names = [
        "md-heading-0",
        "md-paragraph-1",
        "md-list-2",
        "md-table-3",
        "md-code-4",
    ];
    let boxes: Vec<_> = names.iter().map(|n| bounds(cx, n)).collect();
    for pair in boxes.windows(2) {
        assert!(
            pair[0].bottom() <= pair[1].top(),
            "blocks stack top to bottom"
        );
    }
    // The table has its header and both body rows; the list has both items.
    assert!(boxes[3].size.height > boxes[1].size.height * 2.0);
    assert!(boxes[2].size.height > boxes[1].size.height);
    assert_eq!(probe(&page, cx).prepared, 5);
    assert!(cx.debug_bounds("md-paragraph-5").is_none());
}

#[gpui::test]
fn appending_chunks_only_prepares_the_tail(cx: &mut TestAppContext) {
    let first = "# Title\n\nFirst paragraph.\n\nSecond paragraph.\n\nThird par";
    let (page, cx) = page(cx, first, None);
    let before = probe(&page, cx);
    assert_eq!((before.prepared, before.cached.len()), (4, 4));

    show(&page, cx, &format!("{first}agraph keeps growing"), true);
    let grown = probe(&page, cx);
    assert_eq!(grown.cached.len(), 4);
    // The parser re-derives at most the last two blocks of an append; everything before keeps
    // its block, so it keeps its prepared value too.
    assert_eq!(
        &grown.cached[..2],
        &before.cached[..2],
        "settled blocks keep their cache"
    );
    assert_ne!(grown.cached[3], before.cached[3]);
    assert!(grown.prepared > before.prepared && grown.prepared <= before.prepared + 2);

    let longer = format!("{first}agraph keeps growing\n\nFourth paragraph.");
    show(&page, cx, &longer, true);
    let more = probe(&page, cx);
    assert_eq!(more.cached.len(), 5);
    assert_eq!(&more.cached[..2], &before.cached[..2]);
    // The last two blocks and the new one; the settled head is untouched.
    assert!(more.prepared <= grown.prepared + 3);

    // The same text again changes nothing.
    let prepared = more.prepared;
    show(&page, cx, &longer, true);
    assert_eq!(probe(&page, cx).prepared, prepared);
}

#[gpui::test]
fn a_link_still_streaming_in_is_not_clickable(cx: &mut TestAppContext) {
    let (page, cx) = page(cx, "[docs](", None);
    click_left_of(cx, "md-paragraph-0");
    assert!(effects(&page, cx).is_empty());
    assert_eq!(probe(&page, cx).confirm, None);
}

#[gpui::test]
fn text_for_a_refused_destination_is_not_clickable(cx: &mut TestAppContext) {
    let (page, cx) = page(
        cx,
        "[docs](javascript:alert(1)) and [more](file:///c:/x)",
        None,
    );
    click_left_of(cx, "md-paragraph-0");
    assert!(effects(&page, cx).is_empty());
    assert_eq!(probe(&page, cx).confirm, None);
}

#[gpui::test]
fn an_external_link_shows_the_full_url_and_opens_only_after_the_reader_accepts(
    cx: &mut TestAppContext,
) {
    let url = "https://example.com/a/long/path?query=1&other=2";
    let (page, cx) = page(cx, &format!("[docs]({url})"), None);
    click_left_of(cx, "md-paragraph-0");
    assert_eq!(probe(&page, cx).confirm.as_deref(), Some(url));
    assert!(
        effects(&page, cx).is_empty(),
        "nothing opens before the answer"
    );

    let cancel = bounds(cx, "md-confirm-cancel");
    cx.simulate_click(cancel.center(), Modifiers::none());
    assert_eq!(probe(&page, cx).confirm, None);
    assert!(effects(&page, cx).is_empty());

    click_left_of(cx, "md-paragraph-0");
    let open = bounds(cx, "md-confirm-open");
    cx.simulate_click(open.center(), Modifiers::none());
    assert_eq!(
        effects(&page, cx),
        vec![LinkEffect::OpenUrl(url.to_owned())]
    );
    assert_eq!(probe(&page, cx).confirm, None);
}

#[gpui::test]
fn a_project_file_link_reveals_the_file_and_never_opens_it(cx: &mut TestAppContext) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("tools")).unwrap();
    std::fs::write(dir.path().join("tools").join("x.cmd"), "@echo off").unwrap();
    let (page, cx) = page(cx, "[run it](tools/x.cmd)", Some(dir.path().to_path_buf()));
    click_left_of(cx, "md-paragraph-0");
    let effects = effects(&page, cx);
    match effects.as_slice() {
        [LinkEffect::Reveal(path)] => assert!(path.ends_with("x.cmd"), "{path:?}"),
        other => panic!("expected one reveal, got {other:?}"),
    }
    assert_eq!(probe(&page, cx).confirm, None);
}

#[gpui::test]
fn project_file_links_do_nothing_without_a_project(cx: &mut TestAppContext) {
    let (page, cx) = page(cx, "[run it](tools/x.cmd)", None);
    click_left_of(cx, "md-paragraph-0");
    assert!(effects(&page, cx).is_empty());
}

#[gpui::test]
fn a_drag_across_blocks_selects_and_copies_them_in_order(cx: &mut TestAppContext) {
    let (_page, cx) = page(cx, "# Title\n\nIntro paragraph.\n\nLast words.", None);
    let (heading, intro) = (bounds(cx, "md-heading-0"), bounds(cx, "md-paragraph-1"));
    let from = point(heading.left() + px(1.), heading.center().y);
    let to = point(intro.right() - px(1.), intro.center().y);
    cx.simulate_mouse_down(from, gpui::MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_move(to, gpui::MouseButton::Left, Modifiers::none());
    cx.simulate_mouse_up(to, gpui::MouseButton::Left, Modifiers::none());
    cx.run_until_parked();
    cx.simulate_keystrokes(COPY_KEY);
    let copied = cx.update(|_, cx| cx.read_from_clipboard().and_then(|c| c.text()));
    assert_eq!(copied.as_deref(), Some("Title\nIntro paragraph."));
    // Markdown text is not the kit's selectable text.
    assert_eq!(cx.update(|_, cx| selected_text(cx)), None);

    // A press elsewhere in the window ends the selection.
    let below = point(heading.left(), intro.bottom() + px(200.));
    cx.simulate_click(below, Modifiers::none());
    cx.update(|_, cx| cx.write_to_clipboard(gpui::ClipboardItem::new_string("sentinel".into())));
    cx.simulate_keystrokes(COPY_KEY);
    let copied = cx.update(|_, cx| cx.read_from_clipboard().and_then(|c| c.text()));
    assert_eq!(copied.as_deref(), Some("sentinel"));
}

#[gpui::test]
fn text_that_streams_in_fades(cx: &mut TestAppContext) {
    let (page, cx) = page_streaming(cx, "Hello", None, true);
    assert!(!probe(&page, cx).fading, "text already there does not fade");
    show(&page, cx, "Hello world", true);
    assert!(probe(&page, cx).fading, "the appended text fades in");
}

#[gpui::test]
fn a_row_that_is_not_streaming_never_fades(cx: &mut TestAppContext) {
    let (page, cx) = page(cx, "Hello", None);
    show(&page, cx, "Hello world", false);
    assert!(!probe(&page, cx).fading);
}

fn project_with_script() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("tools")).unwrap();
    std::fs::write(dir.path().join("tools").join("x.cmd"), "@echo off").unwrap();
    dir
}

#[gpui::test]
fn a_code_span_naming_a_project_file_links_after_the_probe_and_reveals_never_opens(
    cx: &mut TestAppContext,
) {
    let dir = project_with_script();
    let (page, cx) = page(cx, "`tools/x.cmd`", Some(dir.path().to_path_buf()));
    assert_eq!(probe(&page, cx).links, ["tools/x.cmd"]);
    click_left_of(cx, "md-paragraph-0");
    match effects(&page, cx).as_slice() {
        [LinkEffect::Reveal(path)] => assert!(path.ends_with("x.cmd"), "{path:?}"),
        other => panic!("expected one reveal, got {other:?}"),
    }
    assert_eq!(probe(&page, cx).confirm, None);
}

#[gpui::test]
fn render_does_not_probe_the_disk_before_the_executor_runs(cx: &mut TestAppContext) {
    let dir = project_with_script();
    cx.update(bind_selectable_text_keys);
    let root = Some(dir.path().to_path_buf());
    let probe_cell = Rc::<RefCell<Probe>>::default();
    let window = cx.add_window({
        let probe_cell = probe_cell.clone();
        move |_, _| Page {
            row: row("`tools/x.cmd`", false),
            root,
            probe: probe_cell,
            effects: Rc::default(),
        }
    });
    let cx = VisualTestContext::from_window(*window, cx).into_mut();
    cx.update(|window, _| window.refresh());
    assert!(
        probe_cell.borrow().links.is_empty() && probe_cell.borrow().cached.len() == 1,
        "rendered once, still plain code before the executor runs: {:?}",
        probe_cell.borrow()
    );
    cx.run_until_parked();
    assert_eq!(probe_cell.borrow().links, ["tools/x.cmd"]);
}

#[gpui::test]
fn a_code_span_stays_plain_without_a_project(cx: &mut TestAppContext) {
    let (page, cx) = page(cx, "`tools/x.cmd`", None);
    assert!(probe(&page, cx).links.is_empty());
    click_left_of(cx, "md-paragraph-0");
    assert!(effects(&page, cx).is_empty());
}

#[gpui::test]
fn code_spans_that_leave_the_project_or_are_missing_stay_plain(cx: &mut TestAppContext) {
    let dir = project_with_script();
    let (page, cx) = page(
        cx,
        "`tools/none.cmd` `../x.cmd` `tools/` `C:\\x.cmd`",
        Some(dir.path().to_path_buf()),
    );
    assert!(probe(&page, cx).links.is_empty());
}

#[gpui::test]
fn a_code_span_still_being_written_is_not_probed(cx: &mut TestAppContext) {
    let dir = project_with_script();
    let root = Some(dir.path().to_path_buf());
    let (page, cx) = page_streaming(cx, "Run `tools/x.cmd", root, true);
    assert!(probe(&page, cx).links.is_empty());
    // Once the closing backtick arrives, the span is complete and links.
    show(&page, cx, "Run `tools/x.cmd` now", true);
    assert_eq!(probe(&page, cx).links, ["tools/x.cmd"]);
}

#[gpui::test]
fn an_answer_re_prepares_only_the_blocks_that_asked(cx: &mut TestAppContext) {
    let dir = project_with_script();
    let text = "First paragraph.\n\n`tools/x.cmd`\n\nLast paragraph.";
    let (page, cx) = page(cx, text, Some(dir.path().to_path_buf()));
    let done = probe(&page, cx);
    // Three blocks prepared once, plus the one block whose span got an answer.
    assert_eq!(done.prepared, 4);
    assert_eq!(done.links, ["tools/x.cmd"]);
    show(&page, cx, text, false);
    assert_eq!(probe(&page, cx).prepared, 4, "nothing new to prepare");
}
