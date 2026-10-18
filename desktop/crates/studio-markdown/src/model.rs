//! Block tree model: the data a parse produces.

use std::ops::Range;
use std::sync::Arc;

/// Inline styling flags threaded through nested emphasis/links.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InlineStyle {
    /// Strong emphasis.
    pub bold: bool,
    /// Emphasis.
    pub italic: bool,
    /// Inline code span.
    pub code: bool,
    /// GFM strikethrough.
    pub strikethrough: bool,
    /// Destination URL when inside a link.
    pub link: Option<String>,
    /// The label to show instead of `text` for a file link whose text is the
    /// path itself (an inline-code span) rather than a label the author
    /// wrote. Never set by the parser; `None` shows `text`. When set, copy
    /// and selection still map back to `text`.
    pub file_label: Option<String>,
    /// Set when the run stands for an image.
    pub image: Option<InlineImage>,
    /// Set when the run is a task-list marker.
    pub task: Option<TaskMarker>,
}

/// A task-list checkbox at the start of a list item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskMarker {
    /// Whether the box is ticked.
    pub checked: bool,
    /// Byte range of `[ ]`, `[x]` or `[X]` in the original document.
    pub range: Range<usize>,
}

/// An image remains inline in the AST; hosts can opt in to visual media.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineImage {
    /// Image destination as written.
    pub source: String,
    /// Plain-text alt text.
    pub alt: String,
    /// Title attribute, empty when absent.
    pub title: String,
    /// Destination of the link wrapping the image, if any.
    pub link: Option<String>,
}

/// One run of identically-styled inline text.
#[derive(Debug, Clone, PartialEq)]
pub struct InlineRun {
    /// The run's text.
    pub text: String,
    /// The run's styling.
    pub style: InlineStyle,
}

/// A markdown block. Containers nest.
#[derive(Debug, Clone, PartialEq)]
pub enum Block {
    /// A paragraph of inline runs.
    Paragraph {
        /// The paragraph's text.
        runs: Vec<InlineRun>,
    },
    /// An ATX or setext heading.
    Heading {
        /// Level `1..=6`.
        level: u8,
        /// The heading's text.
        runs: Vec<InlineRun>,
    },
    /// A fenced or indented code block.
    CodeBlock {
        /// First word of the fence info string, if any.
        language: Option<String>,
        /// The code, verbatim.
        code: String,
    },
    /// A block quote.
    BlockQuote {
        /// Blocks inside the quote.
        children: Vec<Block>,
    },
    /// An ordered or unordered list.
    List {
        /// Number of the first item for an ordered list; `None` when unordered.
        ordered_start: Option<u64>,
        /// One block sequence per item.
        items: Vec<Vec<Block>>,
    },
    /// A GFM table.
    Table {
        /// Header cells.
        header: Vec<Vec<InlineRun>>,
        /// Body rows of cells.
        rows: Vec<Vec<Vec<InlineRun>>>,
        /// Per-column GFM alignment (`:--`/`:-:`/`--:`); unspecified is Left.
        align: Vec<TableAlign>,
    },
    /// A thematic break.
    Rule,
}

/// GFM column alignment for a table; an unspecified column is `Left`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TableAlign {
    /// Left-aligned (the default).
    #[default]
    Left,
    /// Centered.
    Center,
    /// Right-aligned.
    Right,
}

/// A top-level block plus its byte range in the source. The range start is the
/// stable-boundary anchor for incremental reparses.
#[derive(Debug, Clone, PartialEq)]
pub struct TopBlock {
    /// Byte range of the block in the source.
    pub range: Range<usize>,
    /// The parsed block.
    pub block: Block,
}

/// The parse result: top-level blocks in document order.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct BlockTree {
    /// Completed blocks are immutable. Canonical/display trees and old/new
    /// transcript rows share their contents across streamed tail updates.
    pub blocks: Vec<Arc<TopBlock>>,
}

impl BlockTree {
    /// Whether the tree has no blocks.
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// Number of top-level blocks.
    pub fn len(&self) -> usize {
        self.blocks.len()
    }
}
