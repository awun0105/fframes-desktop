use super::*;

#[cfg(feature = "syntax-grammars")]
fn class_of(code: &str, spans: &[HighlightSpan], needle: &str) -> Option<TokenClass> {
    let at = code.find(needle)?;
    spans
        .iter()
        .find(|span| span.range.start <= at && at < span.range.end)
        .map(|span| span.class)
}

#[cfg(feature = "syntax-grammars")]
fn assert_well_formed(code: &str, spans: &[HighlightSpan]) {
    let mut previous_end = 0;
    for span in spans {
        assert!(span.range.start < span.range.end, "empty span {span:?}");
        assert!(span.range.start >= previous_end, "overlap at {span:?}");
        assert!(span.range.end <= code.len(), "out of bounds {span:?}");
        assert!(code.is_char_boundary(span.range.start), "start {span:?}");
        assert!(code.is_char_boundary(span.range.end), "end {span:?}");
        previous_end = span.range.end;
    }
}

#[test]
fn aliases_resolve_to_the_five_languages() {
    for (tags, language) in [
        (
            &[
                "rs",
                "rust",
                "Rust",
                " rust ",
                "rust,ignore",
                "rust title=x",
            ][..],
            Language::Rust,
        ),
        (&["toml", "TOML"][..], Language::Toml),
        (&["json", "jsonc"][..], Language::Json),
        (&["md", "markdown"][..], Language::Markdown),
        (
            &["sh", "bash", "shell", "zsh", "console"][..],
            Language::Shell,
        ),
    ] {
        for tag in tags {
            assert_eq!(Language::from_tag(tag), Some(language), "{tag}");
        }
    }
}

#[test]
fn unknown_languages_return_no_spans() {
    for tag in ["", "python", "text", "yaml", "rustacean"] {
        assert_eq!(Language::from_tag(tag), None, "{tag}");
        assert!(highlight(tag, "let x = 1;").is_empty(), "{tag}");
    }
}

#[test]
fn text_over_the_limit_is_left_plain() {
    let code = "a = 1\n".repeat(MAX_HIGHLIGHT_BYTES);
    assert!(highlight("toml", &code).is_empty());
}

#[test]
fn colours_come_from_the_theme() {
    let theme = Theme::dark();
    assert_eq!(token_color(TokenClass::Comment, &theme), theme.gray_10);
    assert_ne!(
        token_color(TokenClass::Keyword, &theme),
        token_color(TokenClass::String, &theme)
    );
}

#[cfg(not(feature = "syntax-grammars"))]
#[test]
fn no_spans_without_the_grammars() {
    assert!(highlight("rust", "fn main() {}").is_empty());
}

#[cfg(feature = "syntax-grammars")]
mod grammars {
    use super::*;

    #[test]
    fn rust() {
        let code = "// note\nfn main() {\n    let n = 42;\n    println!(\"héllo\");\n}\n";
        let spans = highlight("rs", code);
        assert_well_formed(code, &spans);
        assert_eq!(class_of(code, &spans, "// note"), Some(TokenClass::Comment));
        assert_eq!(class_of(code, &spans, "fn"), Some(TokenClass::Keyword));
        assert_eq!(class_of(code, &spans, "main"), Some(TokenClass::Function));
        assert_eq!(class_of(code, &spans, "42"), Some(TokenClass::Number));
        assert_eq!(class_of(code, &spans, "\"h"), Some(TokenClass::String));
    }

    #[test]
    fn toml() {
        let code = "# c\n[package]\nname = \"fframes\"\nversion = 3\nok = true\n";
        let spans = highlight("toml", code);
        assert_well_formed(code, &spans);
        assert_eq!(class_of(code, &spans, "# c"), Some(TokenClass::Comment));
        assert_eq!(class_of(code, &spans, "name"), Some(TokenClass::Property));
        assert_eq!(
            class_of(code, &spans, "\"fframes"),
            Some(TokenClass::String)
        );
        assert_eq!(class_of(code, &spans, "3"), Some(TokenClass::Number));
    }

    #[test]
    fn json() {
        let code = "{\"key\": [1, true, null, \"v\"]}";
        let spans = highlight("json", code);
        assert_well_formed(code, &spans);
        assert!(!spans.is_empty());
        assert_eq!(class_of(code, &spans, "1"), Some(TokenClass::Number));
        assert_eq!(class_of(code, &spans, "\"v"), Some(TokenClass::String));
    }

    #[test]
    fn markdown_with_inline_and_fenced_code() {
        let code = "# Title\n\nSome `code` and **bold**.\n\n```rust\nfn x() {}\n```\n";
        let spans = highlight("md", code);
        assert_well_formed(code, &spans);
        assert_eq!(class_of(code, &spans, "Title"), Some(TokenClass::Heading));
        assert_eq!(class_of(code, &spans, "code`"), Some(TokenClass::Raw));
        assert_eq!(class_of(code, &spans, "fn x"), Some(TokenClass::Keyword));
    }

    #[test]
    fn shell() {
        let code = "# build\nexport A=\"b\"\ncargo build --release | tee out.log\n";
        let spans = highlight("bash", code);
        assert_well_formed(code, &spans);
        assert_eq!(class_of(code, &spans, "# build"), Some(TokenClass::Comment));
        assert_eq!(class_of(code, &spans, "\"b"), Some(TokenClass::String));
        assert_eq!(class_of(code, &spans, "cargo"), Some(TokenClass::Function));
    }

    #[test]
    fn a_growing_unterminated_block_stays_well_formed() {
        let full = "fn greet(name: &str) -> String {\n    format!(\"hi {name} \u{1f600}\")\n}\n";
        for end in (0..=full.len()).filter(|end| full.is_char_boundary(*end)) {
            let partial = &full[..end];
            assert_well_formed(partial, &highlight("rust", partial));
        }
        // The finished keyword keeps its colour while the string is still open.
        let open = "fn a() { let s = \"unterminated";
        let spans = highlight("rust", open);
        assert_well_formed(open, &spans);
        assert_eq!(class_of(open, &spans, "fn"), Some(TokenClass::Keyword));
    }

    #[test]
    fn every_language_survives_every_prefix_of_multibyte_text() {
        let samples = [
            ("toml", "title = \"\u{e9}t\u{e9}\"\n[a.b]\nx = [1, 2]\n"),
            ("json", "{\"k\u{e9}\": [\"\u{1f600}\", 1.5e3]}"),
            ("md", "# \u{e9}\n- *a* `b`\n```sh\necho \u{e9}\n```\n"),
            ("sh", "echo \"\u{e9}\" && ls $HOME\n"),
        ];
        for (tag, full) in samples {
            for end in (0..=full.len()).filter(|end| full.is_char_boundary(*end)) {
                let partial = &full[..end];
                assert_well_formed(partial, &highlight(tag, partial));
            }
        }
    }

    #[test]
    fn repeated_calls_give_the_same_spans() {
        let code = "let a = 1;\n";
        assert_eq!(highlight("rust", code), highlight("rust", code));
    }
}
