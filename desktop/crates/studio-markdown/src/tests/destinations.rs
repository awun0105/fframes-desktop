use crate::destinations::*;
use crate::parse::*;
use crate::*;

fn paragraph(source: &str) -> Vec<InlineRun> {
    let tree = parse_full(source);
    let Some(TopBlock {
        block: Block::Paragraph { runs },
        ..
    }) = tree.blocks.first().map(|top| top.as_ref())
    else {
        panic!("expected a paragraph for {source:?}");
    };
    runs.clone()
}

fn link_of(runs: &[InlineRun]) -> Option<&str> {
    runs.iter().find_map(|run| run.style.link.as_deref())
}

fn text(runs: &[InlineRun]) -> String {
    runs.iter().map(|run| run.text.as_str()).collect()
}

#[test]
fn absolute_destinations_with_spaces_become_links() {
    let runs = paragraph("see [it's here](/tmp/2026/Some Folder/it's here.txt) now");
    assert_eq!(text(&runs), "see it's here now");
    assert_eq!(link_of(&runs), Some("/tmp/2026/Some Folder/it's here.txt"));

    let runs = paragraph("[x](file:///tmp/a b/c.md)");
    assert_eq!(link_of(&runs), Some("file:///tmp/a b/c.md"));

    let runs = paragraph("shot ![two](/tmp/pics/two words.png) done");
    let image = runs.iter().find_map(|run| run.style.image.as_ref());
    assert_eq!(
        image.map(|image| image.source.as_str()),
        Some("/tmp/pics/two words.png")
    );
}

#[test]
fn non_absolute_or_unsafe_destinations_keep_their_text() {
    for source in [
        // Relative destinations with spaces stay plain text.
        "[x](Some Folder/a.md)",
        // The file-name dot rule applies to plain absolute paths.
        "[x](/usr/bin/some thing)",
        // `file://localhost` is an ordinary URL, not a host path.
        "[x](file://localhost/tmp/a b.md)",
        "[x](/tmp/a b/c/)",
        "[x](/tmp/a?b c.md)",
        "[x](/tmp/../a b.md)",
    ] {
        let runs = paragraph(source);
        assert_eq!(link_of(&runs), None, "{source}");
        assert!(text(&runs).contains('('), "{source} should stay literal");
    }
}

#[test]
fn already_bracketed_destinations_are_left_alone() {
    let runs = paragraph("see [it](/tmp/Some Folder/it's here.txt) now");
    assert_eq!(text(&runs), "see it now");
    assert_eq!(link_of(&runs), Some("/tmp/Some Folder/it's here.txt"));
}

#[test]
fn code_keeps_its_spelling() {
    let tree = parse_full("```\n[x](/tmp/a b.md)\n```\n");
    let Block::CodeBlock { code, .. } = &tree.blocks[0].block else {
        panic!("fenced code");
    };
    assert_eq!(code.trim_end(), "[x](/tmp/a b.md)");

    let tree = parse_full("    [x](/tmp/a b.md)\n");
    let Block::CodeBlock { code, .. } = &tree.blocks[0].block else {
        panic!("indented code");
    };
    assert_eq!(code.trim_end(), "[x](/tmp/a b.md)");

    let runs = paragraph("`[x](/tmp/a b.md)`");
    assert!(runs.iter().all(|run| run.style.link.is_none()));
    assert_eq!(text(&runs), "[x](/tmp/a b.md)");
}

#[test]
fn unclosed_destinations_wait_for_their_closing_paren() {
    let mut parser = IncrementalParser::new();
    parser.append("see [x](/tmp/a b");
    assert!(link_of(&paragraph(parser.source())).is_none());
    parser.append(".md) now");
    assert_eq!(link_of(&paragraph(parser.source())), Some("/tmp/a b.md"));
    assert_eq!(parser.tree(), &parse_full(parser.source()));
}

#[test]
fn rewritten_ranges_map_back_to_the_original_source() {
    let source = "intro [x](/tmp/a b.md) tail\n\nsecond paragraph\n";
    let tree = parse_full(source);
    assert_eq!(tree.blocks.len(), 2);
    assert_eq!(
        &source[tree.blocks[0].range.clone()],
        "intro [x](/tmp/a b.md) tail\n"
    );
    assert_eq!(&source[tree.blocks[1].range.clone()], "second paragraph\n");
}

#[test]
fn rewrite_is_idempotent_on_its_own_output() {
    let source = "see [it](/tmp/Some Folder/it's here.txt) and [web](https://x.dev/a b)";
    let once = rewrite_absolute_destinations(source).expect("rewrite");
    assert!(once.0.contains("](</tmp/Some Folder/it's here.txt>)"));
    assert!(once.0.contains("https://x.dev/a b"));
    assert!(
        rewrite_absolute_destinations(&once.0).is_none(),
        "second pass must find nothing to rewrite"
    );
}
