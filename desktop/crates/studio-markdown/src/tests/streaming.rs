//! Streamed parses against full parses, and the work done per chunk.

use crate::{IncrementalParser, parse_full};

/// A reply shaped like an agent answer: headings, nested lists, a table, a
/// fenced Rust block, a blockquote, inline styling and a link.
const REPLY: &str = "\
# Plan for the export panel

I looked at the **render path** and the `ExportJob` type. Here is what I found:

## Findings

1. The estimate runs on the *UI thread*.
2. Progress is reported per frame.
   - cancel is checked once per segment
   - the last segment ignores it
3. The preset list is hard-coded.

| Preset | Width | Height |
| ------ | ----: | -----: |
| Draft  |   960 |    540 |
| Final  |  1920 |   1080 |

## Change

```rust
fn estimate(job: &ExportJob) -> Duration {
    let frames = job.frames();

    frames.saturating_mul(job.cost_per_frame())
}
```

> Keep the estimate pure so the panel can call it on every edit.

- [x] write the estimate
- [ ] wire the cancel button

See [the docs](https://example.com/docs/export) for details.
";

/// Deterministic chunk sizes in `1..=max`, always cut on char boundaries.
fn chunks(text: &str, max: usize, seed: u64) -> Vec<&str> {
    let mut state = seed;
    let mut out = Vec::new();
    let mut at = 0;
    while at < text.len() {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let mut end = (at + 1 + (state >> 33) as usize % max).min(text.len());
        while !text.is_char_boundary(end) {
            end += 1;
        }
        out.push(&text[at..end]);
        at = end;
    }
    out
}

fn assert_stream_matches_full(parts: &[&str]) {
    let mut parser = IncrementalParser::new();
    let mut seen = String::new();
    for part in parts {
        seen.push_str(part);
        parser.append(part);
        assert_eq!(parser.source(), seen);
        assert_eq!(
            parser.tree(),
            &parse_full(&seen),
            "diverged after {} bytes: {seen:?}",
            seen.len()
        );
    }
}

#[test]
fn recorded_stream_matches_full_parse_after_every_chunk() {
    for (max, seed) in [(1, 1), (3, 7), (5, 11), (9, 23), (17, 42), (40, 5)] {
        assert_stream_matches_full(&chunks(REPLY, max, seed));
    }
}

#[test]
fn chunk_boundaries_inside_a_fence_and_a_table_row_match_full_parse() {
    let fence = REPLY.find("fn estimate").unwrap();
    let row = REPLY.find("| Final").unwrap();
    let mut cuts = vec![
        fence + 4,
        fence + 20,
        REPLY[fence..].find("\n\n").unwrap() + fence + 1,
        REPLY.find("```\n\n>").unwrap() + 2,
        row + 1,
        row + 12,
        row + 20,
    ];
    cuts.sort_unstable();
    let mut parts = Vec::new();
    let mut at = 0;
    for cut in cuts {
        parts.push(&REPLY[at..cut]);
        at = cut;
    }
    parts.push(&REPLY[at..]);
    assert_stream_matches_full(&parts);
}

#[test]
fn display_tree_has_the_canonical_shape_once_complete() {
    let mut parser = IncrementalParser::new();
    for part in chunks(REPLY, 6, 3) {
        parser.append(part);
    }
    assert_eq!(parser.display_tree(), parse_full(REPLY));
}

#[test]
fn multibyte_text_streams_cleanly() {
    let text = "# Título 🦀\n\nSegundo párrafo con **énfasis** y `código`.\n\n- uno 🦀\n- dos\n";
    assert_stream_matches_full(&chunks(text, 4, 9));
}

fn generated_reply(lines: usize) -> String {
    let mut out = String::new();
    let mut n = 0;
    while out.lines().count() < lines {
        n += 1;
        out.push_str(&format!("## Section {n}\n\n"));
        out.push_str(&format!(
            "Paragraph {n} with **bold** text, `code` and a [link](https://example.com/{n}).\n\n"
        ));
        out.push_str(&format!("- item {n}.a\n- item {n}.b\n\n"));
        out.push_str(&format!(
            "| k | v |\n| - | - |\n| {n} | row |\n\n```rust\nlet x = {n};\n\nlet y = x + 1;\n```\n\n> quote {n}\n\n"
        ));
    }
    out
}

#[test]
fn reparsed_bytes_per_chunk_do_not_grow_with_document_length() {
    let text = generated_reply(2_000);
    assert!(text.lines().count() >= 2_000);
    let parts = chunks(&text, 48, 99);

    let mut parser = IncrementalParser::new();
    let mut worst_first_half = 0;
    let mut worst_second_half = 0;
    for (i, part) in parts.iter().enumerate() {
        parser.append(part);
        let worked = parser.last_parse_bytes();
        if i < parts.len() / 2 {
            worst_first_half = worst_first_half.max(worked);
        } else {
            worst_second_half = worst_second_half.max(worked);
        }
    }

    // The largest unit that can be reparsed is the last two blocks plus the
    // chunk and a mended copy of the tail: a few hundred bytes here, against
    // a document of ~20 KB.
    let bound = 1_024;
    assert!(
        text.len() > 20 * bound,
        "document too small: {}",
        text.len()
    );
    assert!(worst_first_half <= bound, "first half: {worst_first_half}");
    assert!(
        worst_second_half <= bound,
        "second half: {worst_second_half}"
    );
    assert_eq!(parser.tree(), &parse_full(&text));
}

#[test]
fn a_link_reference_definition_falls_back_to_full_reparses() {
    let mut parser = IncrementalParser::new();
    parser.append("Intro paragraph.\n\nSee [docs].\n\n");
    parser.append("[docs]: https://example.com\n");
    assert_eq!(parser.last_parse_bytes(), parser.source().len());
    assert_eq!(parser.tree(), &parse_full(parser.source()));
}
