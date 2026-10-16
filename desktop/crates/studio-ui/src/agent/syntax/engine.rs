//! The tree-sitter backed implementation of [`super::highlight_language`].

use std::{cell::RefCell, sync::OnceLock};

use tree_sitter_highlight::{HighlightConfiguration, HighlightEvent, Highlighter};

use super::{HighlightSpan, Language, TokenClass};

/// Capture names handed to every configuration; a highlight index is an index into this list.
/// Longer query names (`keyword.control`) match the longest listed prefix.
const CAPTURES: &[(&str, TokenClass)] = &[
    ("comment", TokenClass::Comment),
    ("keyword", TokenClass::Keyword),
    ("string", TokenClass::String),
    ("string.special", TokenClass::StringSpecial),
    ("string.escape", TokenClass::Escape),
    ("number", TokenClass::Number),
    ("boolean", TokenClass::Boolean),
    ("type", TokenClass::Type),
    ("type.builtin", TokenClass::TypeBuiltin),
    ("constructor", TokenClass::Constructor),
    ("function", TokenClass::Function),
    ("function.builtin", TokenClass::FunctionBuiltin),
    ("function.macro", TokenClass::Macro),
    ("property", TokenClass::Property),
    ("constant", TokenClass::Constant),
    ("variable", TokenClass::Variable),
    ("variable.builtin", TokenClass::VariableBuiltin),
    ("variable.parameter", TokenClass::Parameter),
    ("operator", TokenClass::Operator),
    ("punctuation", TokenClass::Punctuation),
    ("tag", TokenClass::Tag),
    ("attribute", TokenClass::Attribute),
    ("label", TokenClass::Label),
    ("text.title", TokenClass::Heading),
    ("text.literal", TokenClass::Raw),
    ("text.uri", TokenClass::Link),
    ("text.reference", TokenClass::Reference),
    ("text.emphasis", TokenClass::Emphasis),
    ("text.strong", TokenClass::Strong),
    ("embedded", TokenClass::Embedded),
    ("error", TokenClass::Invalid),
];

thread_local! {
    static HIGHLIGHTER: RefCell<Highlighter> = RefCell::new(Highlighter::new());
}

fn build(
    language: tree_sitter::Language,
    name: &str,
    highlights: &str,
    injections: &str,
) -> Option<HighlightConfiguration> {
    let mut config =
        HighlightConfiguration::new(language, name, highlights, injections, "").ok()?;
    let names: Vec<&str> = CAPTURES.iter().map(|(name, _)| *name).collect();
    config.configure(&names);
    Some(config)
}

fn rust_config() -> Option<HighlightConfiguration> {
    // The upstream query files numbers and booleans under `constant.builtin`.
    let highlights = tree_sitter_rust::HIGHLIGHTS_QUERY
        .replace(
            "(boolean_literal) @constant.builtin",
            "(boolean_literal) @boolean",
        )
        .replace(
            "(integer_literal) @constant.builtin",
            "(integer_literal) @number",
        )
        .replace(
            "(float_literal) @constant.builtin",
            "(float_literal) @number",
        );
    build(
        tree_sitter_rust::LANGUAGE.into(),
        "rust",
        &highlights,
        tree_sitter_rust::INJECTIONS_QUERY,
    )
}

fn markdown_config() -> Option<HighlightConfiguration> {
    // The block grammar's `inline` node owns anonymous children, so without
    // `include-children` the upstream injection would cover an empty range.
    let injections = tree_sitter_md::INJECTION_QUERY_BLOCK.replace(
        "((inline) @injection.content\n  (#set! injection.language \"markdown_inline\"))",
        "((inline) @injection.content\n  (#set! injection.language \"markdown_inline\")\n  (#set! injection.include-children))",
    );
    build(
        tree_sitter_md::LANGUAGE.into(),
        "markdown",
        tree_sitter_md::HIGHLIGHT_QUERY_BLOCK,
        &injections,
    )
}

fn markdown_inline_config() -> Option<&'static HighlightConfiguration> {
    static CONFIG: OnceLock<Option<HighlightConfiguration>> = OnceLock::new();
    CONFIG
        .get_or_init(|| {
            build(
                tree_sitter_md::INLINE_LANGUAGE.into(),
                "markdown_inline",
                tree_sitter_md::HIGHLIGHT_QUERY_INLINE,
                tree_sitter_md::INJECTION_QUERY_INLINE,
            )
        })
        .as_ref()
}

fn config(language: Language) -> Option<&'static HighlightConfiguration> {
    macro_rules! cached {
        ($make:expr) => {{
            static CONFIG: OnceLock<Option<HighlightConfiguration>> = OnceLock::new();
            CONFIG.get_or_init(|| $make).as_ref()
        }};
    }
    match language {
        Language::Rust => cached!(rust_config()),
        Language::Toml => cached!(build(
            tree_sitter_toml_ng::LANGUAGE.into(),
            "toml",
            tree_sitter_toml_ng::HIGHLIGHTS_QUERY,
            "",
        )),
        Language::Json => cached!(build(
            tree_sitter_json::LANGUAGE.into(),
            "json",
            tree_sitter_json::HIGHLIGHTS_QUERY,
            "",
        )),
        Language::Shell => cached!(build(
            tree_sitter_bash::LANGUAGE.into(),
            "bash",
            tree_sitter_bash::HIGHLIGHT_QUERY,
            "",
        )),
        Language::Markdown => cached!(markdown_config()),
    }
}

pub(super) fn highlight(language: Language, code: &str) -> Vec<HighlightSpan> {
    let Some(primary) = config(language) else {
        return Vec::new();
    };
    HIGHLIGHTER.with(|highlighter| {
        let Ok(mut highlighter) = highlighter.try_borrow_mut() else {
            return Vec::new();
        };
        let events = highlighter.highlight(primary, code.as_bytes(), None, |name| {
            if name == "markdown_inline" {
                return markdown_inline_config();
            }
            // Fenced code inside Markdown: the other bundled languages, one level deep.
            match Language::from_tag(name)? {
                Language::Markdown => None,
                nested => config(nested),
            }
        });
        let Ok(events) = events else {
            return Vec::new();
        };

        let mut stack: Vec<TokenClass> = Vec::new();
        let mut spans: Vec<HighlightSpan> = Vec::new();
        for event in events {
            let Ok(event) = event else {
                return Vec::new();
            };
            match event {
                HighlightEvent::HighlightStart(h) => stack.push(CAPTURES[h.0].1),
                HighlightEvent::HighlightEnd => {
                    stack.pop();
                }
                HighlightEvent::Source { start, end } => {
                    let Some(&class) = stack.last() else { continue };
                    if start >= end || end > code.len() {
                        continue;
                    }
                    match spans.last_mut() {
                        Some(last) if last.class == class && last.range.end == start => {
                            last.range.end = end;
                        }
                        _ => spans.push(HighlightSpan {
                            range: start..end,
                            class,
                        }),
                    }
                }
            }
        }
        spans
    })
}
