//! Fenced and indented code blocks: a monospace panel with the language, a copy button and the
//! highlighted text.

use gpui::{
    AnyElement, ClipboardItem, InteractiveElement, IntoElement, ParentElement, SharedString,
    StatefulInteractiveElement, Styled, div, px,
};
use studio_theme::{TextStyle, text_style};

use super::super::inline::{Flat, Seg};
use super::super::style::{CODE_SIZE, MdStyle};
use super::blocks::BuildCx;
use crate::agent::syntax::{self, HighlightSpan};
use crate::kit::{Button, ButtonSize, ButtonVariant};

/// The only place code gets its colours: the coloured byte ranges of `code` for the fence tag
/// `language`, or none when the language is unknown, the grammars are not built in, or the block
/// is too large to highlight (the text then shows in the plain code colour).
fn highlight_spans(language: Option<&str>, code: &str) -> Vec<HighlightSpan> {
    language.map_or_else(Vec::new, |tag| syntax::highlight(tag, code))
}

/// `code` as monospace segments coloured by [`highlight_spans`].
pub fn highlighted(language: Option<&str>, code: &str, style: &MdStyle) -> Flat {
    let mut segs = Vec::new();
    let mut pos = 0;
    let mono = |len, color| Seg {
        len,
        mono: true,
        color,
        ..Seg::default()
    };
    for span in highlight_spans(language, code) {
        if span.range.start < pos
            || span.range.end > code.len()
            || span.range.is_empty()
            || !code.is_char_boundary(span.range.start)
            || !code.is_char_boundary(span.range.end)
        {
            continue;
        }
        if span.range.start > pos {
            segs.push(mono(span.range.start - pos, None));
        }
        let color = syntax::token_color(span.class, &style.theme);
        segs.push(mono(span.range.len(), Some(color.into())));
        pos = span.range.end;
    }
    if pos < code.len() {
        segs.push(mono(code.len() - pos, None));
    }
    Flat {
        text: code.to_owned(),
        segs,
        links: Vec::new(),
    }
}

pub fn build(language: Option<&str>, raw: &str, flat: &Flat, cx: &mut BuildCx) -> AnyElement {
    let style = cx.style.clone();
    let id = cx.row_id;
    let text = cx.text(flat);
    let copy = SharedString::from(raw.to_owned());
    let copy_id = SharedString::from(format!("md-code-copy-{id}-{}", text.ix));
    let scroll_id = SharedString::from(format!("md-code-scroll-{id}-{}", text.ix));
    div()
        .flex()
        .flex_col()
        .rounded(px(8.))
        .border_1()
        .border_color(style.code_border)
        .bg(style.code_bg)
        .overflow_hidden()
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .px(px(10.))
                .py(px(3.))
                .child(
                    text_style(div(), TextStyle::Small)
                        .text_color(style.muted)
                        .child(SharedString::from(language.unwrap_or("text").to_owned())),
                )
                .child(
                    Button::plain(&style.theme, copy_id, ButtonVariant::Gray, ButtonSize::Xs)
                        .label("Copy")
                        .on_click(move |_, _, cx| {
                            cx.write_to_clipboard(ClipboardItem::new_string(copy.to_string()));
                        }),
                ),
        )
        .child(
            div()
                .id(scroll_id)
                .overflow_x_scroll()
                .px(px(10.))
                .pb(px(8.))
                .text_size(px(CODE_SIZE))
                .line_height(px(17.))
                .text_color(style.text)
                .child(div().whitespace_nowrap().child(text)),
        )
        .into_any_element()
}
