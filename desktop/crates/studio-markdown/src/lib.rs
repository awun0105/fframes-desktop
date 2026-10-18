//! Block-level markdown over pulldown-cmark. UI-free.
//!
//! - [`parse_full`] builds a [`BlockTree`] of top-level blocks with source
//!   ranges; [`IncrementalParser`] reparses only from the last stable
//!   top-level block boundary, so a streamed delta costs O(delta + last block).
//! - [`mend`] auto-closes hanging inline markers in the streaming *display*
//!   parse only; the canonical tree settles honestly on completion.

mod destinations;
mod incremental;
mod inline;
pub mod mend;
mod model;
mod parse;

#[cfg(test)]
mod tests;

pub use incremental::IncrementalParser;
pub use model::{
    Block, BlockTree, InlineImage, InlineRun, InlineStyle, TableAlign, TaskMarker, TopBlock,
};
pub use parse::parse_full;
