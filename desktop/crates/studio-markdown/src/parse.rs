//! Full parse: pulldown-cmark events folded into a [`BlockTree`].

use std::borrow::Cow;
use std::ops::Range;
use std::sync::Arc;

use pulldown_cmark::{Alignment, CodeBlockKind, Event, Options, Parser, Tag};

use crate::destinations::rewrite_absolute_destinations;
use crate::inline::{heading_level, merge_runs, parse_inline_container, parse_inline_event};
use crate::model::{Block, BlockTree, InlineRun, InlineStyle, TableAlign, TopBlock};

pub(crate) fn options() -> Options {
    Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH | Options::ENABLE_TASKLISTS
}

/// Parse a whole source into a [`BlockTree`].
pub fn parse_full(source: &str) -> BlockTree {
    parse_at(source, 0)
}

pub(crate) fn parse_at(source: &str, offset: usize) -> BlockTree {
    let (text, insertions) = match rewrite_absolute_destinations(source) {
        Some((text, insertions)) => (Cow::Owned(text), Some(insertions)),
        None => (Cow::Borrowed(source), None),
    };
    let events: Vec<(Event, Range<usize>)> = Parser::new_ext(&text, options())
        .into_offset_iter()
        .map(|(event, range)| {
            // Inserted angle brackets shift every following offset; ranges
            // come from the rewritten text and must point into the original.
            let range = match &insertions {
                Some(insertions) => insertions.original_range(range),
                None => range,
            };
            (event, range.start + offset..range.end + offset)
        })
        .collect();
    let mut cur = Cursor {
        events: &events,
        ix: 0,
    };
    let mut blocks = Vec::new();
    while let Some((event, range)) = cur.peek() {
        let range = range.clone();
        match event {
            Event::Rule => {
                cur.bump();
                blocks.push(Arc::new(TopBlock {
                    range,
                    block: Block::Rule,
                }));
            }
            Event::Start(_) => {
                for block in parse_started_block(&mut cur) {
                    blocks.push(Arc::new(TopBlock {
                        range: range.clone(),
                        block,
                    }));
                }
            }
            // Stray inline events at top level (shouldn't happen): skip.
            _ => cur.bump(),
        }
    }
    BlockTree { blocks }
}

pub(crate) struct Cursor<'a, 'e> {
    pub(crate) events: &'a [(Event<'e>, Range<usize>)],
    pub(crate) ix: usize,
}

impl<'a, 'e> Cursor<'a, 'e> {
    pub(crate) fn peek(&self) -> Option<&(Event<'e>, Range<usize>)> {
        self.events.get(self.ix)
    }

    pub(crate) fn peek_event(&self) -> Option<&Event<'e>> {
        self.peek().map(|(e, _)| e)
    }

    pub(crate) fn bump(&mut self) {
        self.ix += 1;
    }

    pub(crate) fn next_event(&mut self) -> Option<Event<'e>> {
        let event = self.events.get(self.ix).map(|(e, _)| e.clone());
        if event.is_some() {
            self.ix += 1;
        }
        event
    }
}

pub(crate) fn is_block_tag(tag: &Tag) -> bool {
    matches!(
        tag,
        Tag::Paragraph
            | Tag::Heading { .. }
            | Tag::CodeBlock(_)
            | Tag::BlockQuote(_)
            | Tag::List(_)
            | Tag::Item
            | Tag::Table(_)
            | Tag::HtmlBlock
            | Tag::FootnoteDefinition(_)
    )
}

/// Consume a `Start(tag)` and everything through its matching `End`, producing
/// block(s). Unknown containers are transparent (children splice in).
pub(crate) fn parse_started_block(cur: &mut Cursor) -> Vec<Block> {
    let Some(Event::Start(tag)) = cur.next_event() else {
        return Vec::new();
    };
    match tag {
        Tag::Paragraph => {
            vec![Block::Paragraph {
                runs: parse_inline_container(cur, &InlineStyle::default()),
            }]
        }
        Tag::Heading { level, .. } => vec![Block::Heading {
            level: heading_level(level),
            runs: parse_inline_container(cur, &InlineStyle::default()),
        }],
        Tag::CodeBlock(kind) => {
            let language = match kind {
                CodeBlockKind::Fenced(info) => {
                    let lang = info.split_whitespace().next().unwrap_or("");
                    if lang.is_empty() {
                        None
                    } else {
                        Some(lang.to_string())
                    }
                }
                CodeBlockKind::Indented => None,
            };
            let mut code = String::new();
            loop {
                match cur.next_event() {
                    Some(Event::Text(t)) => code.push_str(&t),
                    Some(Event::End(_)) | None => break,
                    Some(_) => {}
                }
            }
            // Fenced blocks carry a trailing newline; render per-line without it.
            if code.ends_with('\n') {
                code.pop();
            }
            vec![Block::CodeBlock { language, code }]
        }
        Tag::BlockQuote(_) => vec![Block::BlockQuote {
            children: parse_block_sequence(cur),
        }],
        Tag::List(ordered_start) => {
            let mut items = Vec::new();
            loop {
                match cur.peek_event() {
                    Some(Event::Start(Tag::Item)) => {
                        cur.bump();
                        items.push(parse_block_sequence(cur));
                    }
                    Some(Event::End(_)) | None => {
                        cur.bump();
                        break;
                    }
                    Some(_) => cur.bump(),
                }
            }
            vec![Block::List {
                ordered_start,
                items,
            }]
        }
        Tag::Table(align) => {
            let align = align
                .iter()
                .map(|a| match a {
                    Alignment::Center => TableAlign::Center,
                    Alignment::Right => TableAlign::Right,
                    Alignment::None | Alignment::Left => TableAlign::Left,
                })
                .collect();
            vec![parse_table(cur, align)]
        }
        Tag::HtmlBlock => {
            // Render raw HTML blocks as plain text (zeron's markdown does the same).
            let mut text = String::new();
            loop {
                match cur.next_event() {
                    Some(Event::Html(t)) | Some(Event::Text(t)) => text.push_str(&t),
                    Some(Event::End(_)) | None => break,
                    Some(_) => {}
                }
            }
            let text = text.trim_end_matches('\n').to_string();
            if text.is_empty() {
                Vec::new()
            } else {
                vec![Block::Paragraph {
                    runs: vec![InlineRun {
                        text,
                        style: InlineStyle::default(),
                    }],
                }]
            }
        }
        // Transparent containers (footnote definitions when enabled, etc.).
        _ => parse_block_sequence(cur),
    }
}

/// Parse a block sequence until the container's `End` (consumed). Bare inline
/// events (tight list items) accumulate into an implicit paragraph.
pub(crate) fn parse_block_sequence(cur: &mut Cursor) -> Vec<Block> {
    let mut out: Vec<Block> = Vec::new();
    let mut inline_acc: Vec<InlineRun> = Vec::new();
    while let Some(event) = cur.peek_event() {
        match event {
            Event::End(_) => {
                cur.bump();
                break;
            }
            Event::Start(tag) if is_block_tag(tag) => {
                flush_paragraph(&mut out, &mut inline_acc);
                out.extend(parse_started_block(cur));
            }
            Event::Rule => {
                flush_paragraph(&mut out, &mut inline_acc);
                cur.bump();
                out.push(Block::Rule);
            }
            _ => parse_inline_event(cur, &mut inline_acc, &InlineStyle::default()),
        }
    }
    flush_paragraph(&mut out, &mut inline_acc);
    out
}

pub(crate) fn flush_paragraph(out: &mut Vec<Block>, acc: &mut Vec<InlineRun>) {
    if !acc.is_empty() {
        out.push(Block::Paragraph {
            runs: merge_runs(std::mem::take(acc)),
        });
    }
}

pub(crate) fn parse_table(cur: &mut Cursor, align: Vec<TableAlign>) -> Block {
    let mut header = Vec::new();
    let mut rows = Vec::new();
    loop {
        match cur.peek_event() {
            Some(Event::Start(Tag::TableHead)) => {
                cur.bump();
                header = parse_table_cells(cur);
            }
            Some(Event::Start(Tag::TableRow)) => {
                cur.bump();
                rows.push(parse_table_cells(cur));
            }
            Some(Event::End(_)) | None => {
                cur.bump();
                break;
            }
            Some(_) => cur.bump(),
        }
    }
    Block::Table {
        header,
        rows,
        align,
    }
}

pub(crate) fn parse_table_cells(cur: &mut Cursor) -> Vec<Vec<InlineRun>> {
    let mut cells = Vec::new();
    loop {
        match cur.peek_event() {
            Some(Event::Start(Tag::TableCell)) => {
                cur.bump();
                cells.push(parse_inline_container(cur, &InlineStyle::default()));
            }
            Some(Event::End(_)) | None => {
                cur.bump();
                break;
            }
            Some(_) => cur.bump(),
        }
    }
    cells
}
