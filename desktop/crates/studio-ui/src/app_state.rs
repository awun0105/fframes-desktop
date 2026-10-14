//! The window's view of the engine: one entity per window that receives every engine event
//! on the main thread, keeps track of the open project session and the command in flight,
//! and passes on only what belongs to the current session.

use gpui::{Context, EventEmitter};
use std::cell::Cell;
use studio_engine::engine::handle::{CommandFinished, EngineEvent, EngineHandle};
use studio_proto::{CommandId, SessionId};

/// An engine event the window should act on: from the current session, or a finished
/// command. The window's one subscriber takes it (its payload, a build to start, is not
/// shared).
pub struct AppEvent(Cell<Option<EngineEvent>>);

impl AppEvent {
    pub fn take(&self) -> Option<EngineEvent> {
        self.0.take()
    }
}

pub struct AppState {
    engine: EngineHandle,
    /// The session the window shows.
    session: Option<SessionId>,
    /// A session the window is leaving: a command that closes or replaces it is in flight,
    /// and nothing it still says is shown.
    leaving: Option<SessionId>,
    in_flight: Option<CommandId>,
    /// Events dropped because their session was not the current one.
    dropped: u64,
}

impl EventEmitter<AppEvent> for AppState {}

impl AppState {
    /// Applies `events` (the engine's channel) as they arrive.
    pub fn new(
        engine: EngineHandle,
        events: async_channel::Receiver<EngineEvent>,
        cx: &mut Context<Self>,
    ) -> Self {
        cx.spawn(async move |this, cx| {
            while let Ok(event) = events.recv().await {
                if this.update(cx, |state, cx| state.apply(event, cx)).is_err() {
                    break;
                }
            }
        })
        .detach();
        Self {
            engine,
            session: None,
            leaving: None,
            in_flight: None,
            dropped: 0,
        }
    }

    pub fn engine(&self) -> &EngineHandle {
        &self.engine
    }

    /// Whether a command is in flight.
    pub fn busy(&self) -> bool {
        self.in_flight.is_some()
    }

    pub fn session(&self) -> Option<SessionId> {
        self.session
    }

    pub fn dropped(&self) -> u64 {
        self.dropped
    }

    /// Records the command just sent. `leaves_session` when it closes or replaces the open
    /// project: from now on nothing the current session says is passed on.
    pub fn sent(&mut self, id: CommandId, leaves_session: bool) {
        self.in_flight = Some(id);
        if leaves_session {
            self.leaving = self.session;
        }
    }

    fn apply(&mut self, event: EngineEvent, cx: &mut Context<Self>) {
        match &event {
            EngineEvent::CommandFinished(finished) => {
                let CommandFinished { id, session, .. } = &**finished;
                if self.in_flight == Some(*id) {
                    self.in_flight = None;
                }
                self.session = *session;
                if self.leaving.is_some() && self.leaving != self.session {
                    // The command replaced the session it was leaving.
                    self.leaving = None;
                } else if self.in_flight.is_none() {
                    // It finished without replacing it (a refusal, the same folder).
                    self.leaving = None;
                }
            }
            event => {
                let session = event.session().flatten();
                if session != self.session || (session.is_some() && session == self.leaving) {
                    self.dropped += 1;
                    return;
                }
            }
        }
        cx.emit(AppEvent(Cell::new(Some(event))));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::{AppContext, Entity, TestAppContext};
    use studio_engine::engine::backend::{Presentation, StudioEnvironment};

    struct Window {
        state: Entity<AppState>,
        seen: Entity<Vec<String>>,
        events: async_channel::Sender<EngineEvent>,
    }

    fn window(cx: &mut TestAppContext) -> Window {
        let (engine, _engine_events) = EngineHandle::new(StudioEnvironment::system(), "0.0.0");
        let (events, receiver) = async_channel::unbounded();
        let state = cx.new(|cx| AppState::new(engine, receiver, cx));
        let seen = cx.new(|_| Vec::new());
        let sink = seen.clone();
        cx.update(|cx| {
            cx.subscribe(&state, move |_, event: &AppEvent, cx| {
                let label = label(&event.take().unwrap());
                sink.update(cx, |seen, _| seen.push(label));
            })
            .detach();
        });
        Window {
            state,
            seen,
            events,
        }
    }

    fn label(event: &EngineEvent) -> String {
        match event {
            EngineEvent::CommandFinished(finished) => format!("finished {}", finished.id.0),
            EngineEvent::Refreshed { session, .. } => {
                format!("refreshed {:?}", session.map(|s| s.0))
            }
            EngineEvent::PreviewGated(gated) => {
                format!("gated {:?}", gated.session.map(|s| s.0))
            }
            EngineEvent::SourceChanged { session } => {
                format!("changed {:?}", session.map(|s| s.0))
            }
            EngineEvent::CleanupProblem(report) => format!("cleanup {}", report.label),
            EngineEvent::WorkflowChanged { session, serial } => {
                format!("workflow {} of {}", serial, session.0)
            }
        }
    }

    fn finished(id: u64, session: Option<u64>) -> EngineEvent {
        EngineEvent::CommandFinished(Box::new(CommandFinished {
            id: CommandId(id),
            session: session.map(SessionId),
            result: Ok(()),
            presentation: Some(Presentation {
                session: session.map(SessionId),
                ..Default::default()
            }),
            build: None,
            preset: None,
        }))
    }

    fn refreshed(session: u64) -> EngineEvent {
        EngineEvent::Refreshed {
            session: Some(SessionId(session)),
            presentation: None,
        }
    }

    fn deliver(cx: &mut TestAppContext, window: &Window, events: Vec<EngineEvent>) {
        for event in events {
            window.events.send_blocking(event).unwrap();
        }
        cx.run_until_parked();
    }

    fn seen(cx: &mut TestAppContext, window: &Window) -> Vec<String> {
        window.seen.read_with(cx, |seen, _| seen.clone())
    }

    #[gpui::test]
    fn events_of_another_session_are_dropped(cx: &mut TestAppContext) {
        let w = window(cx);
        deliver(
            cx,
            &w,
            vec![finished(1, Some(1)), refreshed(1), refreshed(2)],
        );
        assert_eq!(seen(cx, &w), ["finished 1", "refreshed Some(1)"]);
        assert_eq!(w.state.read_with(cx, |s, _| s.dropped()), 1);
    }

    #[gpui::test]
    fn a_workflow_change_of_a_session_no_longer_open_is_dropped(cx: &mut TestAppContext) {
        let changed = |session, serial| EngineEvent::WorkflowChanged {
            session: SessionId(session),
            serial,
        };
        let w = window(cx);
        deliver(cx, &w, vec![finished(1, Some(1)), changed(1, 1)]);
        deliver(
            cx,
            &w,
            vec![finished(2, Some(2)), changed(1, 1), changed(2, 2)],
        );
        assert_eq!(
            seen(cx, &w),
            [
                "finished 1",
                "workflow 1 of 1",
                "finished 2",
                "workflow 2 of 2"
            ]
        );
        assert_eq!(w.state.read_with(cx, |s, _| s.dropped()), 1);
    }

    #[gpui::test]
    fn a_finished_command_is_never_dropped_and_moves_the_window(cx: &mut TestAppContext) {
        let w = window(cx);
        deliver(cx, &w, vec![finished(1, Some(1)), finished(2, Some(2))]);
        deliver(cx, &w, vec![refreshed(1), refreshed(2)]);
        assert_eq!(
            seen(cx, &w),
            ["finished 1", "finished 2", "refreshed Some(2)"]
        );
        assert_eq!(
            w.state.read_with(cx, |s, _| s.session()),
            Some(SessionId(2))
        );
    }

    #[gpui::test]
    fn the_session_being_left_says_nothing_more(cx: &mut TestAppContext) {
        let w = window(cx);
        deliver(cx, &w, vec![finished(1, Some(1))]);
        w.state
            .update(cx, |state, _| state.sent(CommandId(2), true));
        assert!(w.state.read_with(cx, |s, _| s.busy()));
        // Still in flight: the old session's late events are dropped.
        deliver(cx, &w, vec![refreshed(1)]);
        deliver(
            cx,
            &w,
            vec![finished(2, Some(2)), refreshed(1), refreshed(2)],
        );
        assert_eq!(
            seen(cx, &w),
            ["finished 1", "finished 2", "refreshed Some(2)"]
        );
        assert!(!w.state.read_with(cx, |s, _| s.busy()));
    }

    #[gpui::test]
    fn a_finished_command_without_a_presentation_still_moves_the_window(cx: &mut TestAppContext) {
        let w = window(cx);
        let mut failed = finished(1, Some(1));
        if let EngineEvent::CommandFinished(finished) = &mut failed {
            finished.presentation = None;
            finished.result = Err("presentation unavailable".into());
        }
        deliver(cx, &w, vec![failed, refreshed(1)]);
        assert_eq!(seen(cx, &w), ["finished 1", "refreshed Some(1)"]);
    }

    #[gpui::test]
    fn a_refused_switch_keeps_the_session(cx: &mut TestAppContext) {
        let w = window(cx);
        deliver(cx, &w, vec![finished(1, Some(1))]);
        w.state
            .update(cx, |state, _| state.sent(CommandId(2), true));
        deliver(cx, &w, vec![finished(2, Some(1)), refreshed(1)]);
        assert_eq!(
            seen(cx, &w),
            ["finished 1", "finished 2", "refreshed Some(1)"]
        );
    }

    #[gpui::test]
    fn events_arrive_in_the_order_they_were_sent(cx: &mut TestAppContext) {
        let w = window(cx);
        let sender = w.events.clone();
        std::thread::spawn(move || {
            sender.send_blocking(finished(1, Some(1))).unwrap();
            for _ in 0..50 {
                sender.send_blocking(refreshed(1)).unwrap();
            }
            sender.send_blocking(finished(2, Some(1))).unwrap();
        })
        .join()
        .unwrap();
        cx.run_until_parked();
        let seen = seen(cx, &w);
        assert_eq!(seen.len(), 52);
        assert_eq!(seen.first().unwrap(), "finished 1");
        assert_eq!(seen.last().unwrap(), "finished 2");
    }
}
