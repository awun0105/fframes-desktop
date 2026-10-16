use crate::parse::*;
use crate::*;

const STORY: &str = "\"How do we negotiate with machines that won't speak?\" someone asked.\n\nYuki almost laughed. \"You don't. You listen to the silence. And you finally understand what it means to be powerless.\"";

#[test]
fn full_parse_keeps_trailing_quote_in_block() {
    let tree = parse_full(STORY);
    for b in &tree.blocks {
        eprintln!("block {:?} => {:?}", b.range, &STORY[b.range.clone()]);
    }
    assert_eq!(tree.blocks.len(), 2, "two paragraphs expected");
    let last = &tree.blocks[1];
    assert!(STORY[last.range.clone()].ends_with("powerless.\""));
}

#[test]
fn streamed_boundary_at_quote_adds_no_block() {
    // Stream with a commit boundary exactly between `powerless.` and `"`.
    let split = STORY.len() - 1;
    let mut p = IncrementalParser::new();
    p.set_text(&STORY[..split]);
    p.set_text(STORY);
    let tree = p.tree();
    for b in &tree.blocks {
        eprintln!("block {:?} => {:?}", b.range, &STORY[b.range.clone()]);
    }
    assert_eq!(tree.blocks.len(), 2, "streamed split must not add blocks");
    assert!(STORY[tree.blocks[1].range.clone()].ends_with("powerless.\""));
}

#[test]
fn streamed_small_chunks_match_full_parse() {
    let mut p = IncrementalParser::new();
    let mut fed = String::new();
    for chunk in STORY.as_bytes().chunks(7) {
        fed.push_str(std::str::from_utf8(chunk).unwrap());
        p.set_text(&fed);
    }
    let full = parse_full(STORY);
    assert_eq!(p.tree().blocks.len(), full.blocks.len());
    for (a, b) in p.tree().blocks.iter().zip(full.blocks.iter()) {
        assert_eq!(a.range, b.range);
    }
}
