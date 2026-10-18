//! Unified layout: one column, old and new line numbers side by side.

use gpui::{Div, ParentElement, Styled, div, px};

use super::model::{DiffLine, LineKind, gutter_width};
use super::paint::{
    ACCENT_W, Cell, Gutter, LINE_H, MARKER_W, RowStyle, UNIFIED_CODE_PAD, meta_row,
};

/// One line of a unified diff.
pub fn row(line: &DiffLine, style: &RowStyle) -> Div {
    let gutter_w = gutter_width(style.max_line);
    if line.kind == LineKind::Meta {
        return meta_row(
            &line.text,
            ACCENT_W + 2.0 * gutter_w + MARKER_W + UNIFIED_CODE_PAD,
            style.colors,
        );
    }
    let gutters = [
        Gutter {
            number: line.old_no,
            changed: line.kind == LineKind::Del,
        },
        Gutter {
            number: line.new_no,
            changed: line.kind == LineKind::Add,
        },
    ];
    let cell = Cell {
        line,
        gutters: &gutters,
        gutter_w,
        marker_w: MARKER_W,
        code_pad: UNIFIED_CODE_PAD,
        highlights: style.highlights,
        wrap: style.wrap,
    };
    div()
        .w_full()
        .min_h(px(LINE_H))
        .flex()
        .flex_row()
        .child(cell.render(style.theme, style.colors))
}
