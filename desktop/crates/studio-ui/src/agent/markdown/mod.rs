//! Agent markdown: rendering of streamed agent and thought text, and link safety.
mod inline;
mod inline_code_links;
mod link_ui;
pub mod links;
mod render;
mod selection;
mod style;
mod text_element;
pub mod veil;

#[cfg(test)]
mod tests;

pub use link_ui::{Activation, LinkSink, activate};
pub use render::{MarkdownRow, Voice};
