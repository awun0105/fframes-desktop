//! The review window in the shell: opened from the agent panel's "Review changes", shown in
//! place of the preview, and kept in step with the workflow's snapshots.

use gpui::{AnyElement, AppContext, Context, Entity, IntoElement, Subscription};

use super::StudioShell;
use crate::agent::changes::{ReviewHost, ReviewHostEvent, ReviewRequest, ReviewTarget, facts};
use crate::agent::{ConversationPanel, changes::ReviewBackend};

/// The open review window, if any.
pub(super) struct ReviewState {
    host: Option<Entity<ReviewHost>>,
    _host_events: Option<Subscription>,
    _requests: Subscription,
}

impl ReviewState {
    pub(super) fn new(panel: &Entity<ConversationPanel>, cx: &mut Context<StudioShell>) -> Self {
        let requests = cx.subscribe(panel, |shell, _, request: &ReviewRequest, cx| {
            shell.open_review(request.0.clone(), cx)
        });
        Self {
            host: None,
            _host_events: None,
            _requests: requests,
        }
    }

    /// The window to draw in place of the preview.
    pub(super) fn view(&self) -> Option<AnyElement> {
        self.host.clone().map(|host| host.into_any_element())
    }
}

impl StudioShell {
    /// Shows the review window for `target` in place of the preview. The preview is out of
    /// view while it shows, so playback stops.
    pub(super) fn open_review(&mut self, target: ReviewTarget, cx: &mut Context<Self>) {
        let Some(workflow) = self.agent.workflow.clone() else {
            return;
        };
        self.pause_at_painted_frame(cx);
        let initial = facts(
            &workflow.snapshot(),
            match &target {
                ReviewTarget::Candidate => None,
                ReviewTarget::Revision(revision) => Some(revision.as_str()),
            },
        );
        let backend: std::sync::Arc<dyn ReviewBackend> = workflow;
        let host = cx.new(|cx| ReviewHost::new(target, backend, initial, cx));
        self.review._host_events = Some(cx.subscribe(&host, |shell, _, event, cx| match event {
            ReviewHostEvent::Close => shell.close_review(cx),
        }));
        self.review.host = Some(host);
        cx.notify();
    }

    pub(super) fn close_review(&mut self, cx: &mut Context<Self>) {
        if self.review.host.take().is_some() {
            self.review._host_events = None;
            cx.notify();
        }
    }

    /// Hands the open review window the workflow's newest snapshot.
    pub(super) fn sync_review(&mut self, cx: &mut Context<Self>) {
        let (Some(host), Some(workflow)) = (self.review.host.clone(), self.agent.workflow.clone())
        else {
            return;
        };
        let snapshot = workflow.snapshot();
        host.update(cx, |host, cx| host.on_snapshot(&snapshot, cx));
    }

    /// What the open review window shows, for the observation interface. Absent while closed.
    pub(super) fn review_telemetry(&self, cx: &gpui::App) -> Option<serde_json::Value> {
        self.review
            .host
            .as_ref()
            .map(|host| host.read(cx).telemetry(cx))
    }
}
