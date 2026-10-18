//! Attachment chips above the input.
//!
//! The composer validates nothing on disk and reads no file. The owner turns an
//! [`AttachRequest`](super::AttachRequest) into [`AttachmentDraft`]s after its
//! own checks (project confinement, the deny list, image capability and size),
//! and the chips only show what it accepted.

use std::{rc::Rc, sync::Arc};

use gpui::{
    App, InteractiveElement, IntoElement, ParentElement, SharedString, Styled, Window, div, img,
    prelude::*, px,
};
use studio_theme::Theme;

use super::metrics::{STRIP_GAP, STRIP_PAD_TOP, STRIP_PAD_X, STRIP_THUMB};
use crate::kit::{Icon, IconButton, icon};

/// Identifies a chip. Ids only grow within one composer, so a stale id from a
/// removed chip never names a newer one.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AttachmentId(pub u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttachmentKind {
    File,
    Image,
}

/// What the owner hands the composer to show as a chip.
#[derive(Clone)]
pub struct AttachmentDraft {
    pub kind: AttachmentKind,
    /// The file name, or a description of the image.
    pub label: SharedString,
    /// The pixels of an image chip's thumbnail.
    pub image: Option<Arc<gpui::Image>>,
}

impl AttachmentDraft {
    pub fn file(label: impl Into<SharedString>) -> Self {
        Self {
            kind: AttachmentKind::File,
            label: label.into(),
            image: None,
        }
    }

    pub fn image(label: impl Into<SharedString>, image: Arc<gpui::Image>) -> Self {
        Self {
            kind: AttachmentKind::Image,
            label: label.into(),
            image: Some(image),
        }
    }
}

/// A chip: a draft with its id.
#[derive(Clone)]
pub struct Attachment {
    pub id: AttachmentId,
    pub draft: AttachmentDraft,
}

/// The chips on a draft message, in the order they were added.
#[derive(Clone, Default)]
pub struct Attachments {
    items: Vec<Attachment>,
    next_id: u64,
}

impl Attachments {
    pub fn add(&mut self, draft: AttachmentDraft) -> AttachmentId {
        let id = AttachmentId(self.next_id);
        self.next_id += 1;
        self.items.push(Attachment { id, draft });
        id
    }

    /// Removes a chip. Returns whether it was there.
    pub fn remove(&mut self, id: AttachmentId) -> bool {
        let before = self.items.len();
        self.items.retain(|item| item.id != id);
        self.items.len() != before
    }

    pub fn items(&self) -> &[Attachment] {
        &self.items
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Takes every chip, leaving the strip empty. Ids keep growing.
    pub fn take(&mut self) -> Vec<Attachment> {
        std::mem::take(&mut self.items)
    }

    /// Puts chips back under their own ids, as when the owner refuses a send.
    pub fn restore(&mut self, items: Vec<Attachment>) {
        for item in &items {
            self.next_id = self.next_id.max(item.id.0 + 1);
        }
        let mut merged = items;
        merged.append(&mut self.items);
        merged.sort_by_key(|item| item.id);
        self.items = merged;
    }
}

/// Called with the id of the chip whose remove button was pressed.
pub type RemoveHandler = Rc<dyn Fn(AttachmentId, &mut Window, &mut App)>;

/// The wrapping strip of chips, or nothing when there are none.
pub fn strip(
    attachments: &Attachments,
    theme: &Theme,
    on_remove: RemoveHandler,
) -> Option<impl IntoElement> {
    if attachments.is_empty() {
        return None;
    }
    let chips = attachments
        .items()
        .iter()
        .map(|attachment| chip(attachment, theme, on_remove.clone()))
        .collect::<Vec<_>>();
    Some(
        div()
            .id("composer-attachments")
            .flex()
            .flex_row()
            .flex_wrap()
            .gap(px(STRIP_GAP))
            .pt(px(STRIP_PAD_TOP))
            .px(px(STRIP_PAD_X))
            .children(chips),
    )
}

fn chip(attachment: &Attachment, theme: &Theme, on_remove: RemoveHandler) -> impl IntoElement {
    let id = attachment.id;
    let label = attachment.draft.label.clone();
    let remove = IconButton::new(("attachment-remove", id.0 as usize), Icon::X.path())
        .size(px(16.0))
        .icon_size(px(10.0))
        .rounded(px(8.0))
        .color(theme.gray(12))
        .filled(theme.gray(4), Some(theme.gray(6)))
        .hover_bg(theme.gray(5))
        .label(format!("Remove {label}"))
        .tooltip(theme, "Remove")
        .on_click(move |_, window, cx| on_remove(id, window, cx));
    let base = div()
        .id(("attachment", id.0 as usize))
        .relative()
        .flex_none()
        .rounded(px(8.0))
        .border_1()
        .border_color(theme.gray(6))
        .bg(theme.gray(3));
    let body = match (&attachment.draft.kind, &attachment.draft.image) {
        (AttachmentKind::Image, Some(image)) => base.size(px(STRIP_THUMB)).overflow_hidden().child(
            img(image.clone())
                .size_full()
                .object_fit(gpui::ObjectFit::Cover),
        ),
        _ => base
            .h(px(28.0))
            .max_w(px(180.0))
            .flex()
            .items_center()
            .gap(px(6.0))
            .pl(px(8.0))
            .pr(px(24.0))
            .child(icon(Icon::File, 12.0, theme.gray(11)))
            .child(
                div()
                    .text_size(px(12.0))
                    .text_color(theme.gray(12))
                    .truncate()
                    .child(label),
            ),
    };
    body.child(div().absolute().top(px(2.0)).right(px(2.0)).child(remove))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ids_only_grow_even_after_removal() {
        let mut chips = Attachments::default();
        let a = chips.add(AttachmentDraft::file("a.txt"));
        let b = chips.add(AttachmentDraft::file("b.txt"));
        assert!(chips.remove(b));
        let c = chips.add(AttachmentDraft::file("c.txt"));
        assert!(a < b && b < c);
        assert!(!chips.remove(b), "a removed id is gone");
        assert_eq!(chips.len(), 2);
    }

    #[test]
    fn restore_keeps_ids_and_order_and_never_reuses_them() {
        let mut chips = Attachments::default();
        let a = chips.add(AttachmentDraft::file("a.txt"));
        let b = chips.add(AttachmentDraft::file("b.txt"));
        let taken = chips.take();
        assert!(chips.is_empty());
        let fresh = chips.add(AttachmentDraft::file("fresh.txt"));
        assert!(fresh > b);
        chips.restore(taken);
        let ids: Vec<_> = chips.items().iter().map(|item| item.id).collect();
        assert_eq!(ids, [a, b, fresh]);
        assert!(chips.add(AttachmentDraft::file("later.txt")) > fresh);
    }
}
