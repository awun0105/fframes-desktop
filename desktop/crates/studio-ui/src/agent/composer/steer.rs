//! What Enter means while a turn runs.
//!
//! The composer only chooses which event to emit. Delivering a message at a
//! boundary, steering the agent, or cancelling the turn is the owner's job.

/// How the agent takes a message sent while it is working.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SteerMode {
    /// The agent cannot take a message mid-turn. Only cancelling and sending
    /// again is possible.
    #[default]
    None,
    /// The message is held and delivered at the next turn or step boundary.
    AtBoundary,
    /// The agent accepts the message during the turn without cancelling it.
    Native,
}

/// The event a send gesture produces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendAction {
    /// Start a turn. Nothing is running.
    Submit,
    /// Hold the message for the next boundary.
    SendAtBoundary,
    /// Hand the message to the running turn.
    Steer,
    /// Cancel the turn, then send.
    SendNow,
}

impl SendAction {
    /// The button and tooltip text for the action.
    pub fn label(self) -> &'static str {
        match self {
            SendAction::Submit => "Send",
            SendAction::SendAtBoundary => "Send at boundary",
            SendAction::Steer => "Steer",
            SendAction::SendNow => "Send now (cancels)",
        }
    }
}

/// What each send gesture does in one state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SteerActions {
    /// Enter and the send button. `None` when the agent cannot take the
    /// message now.
    pub primary: Option<SendAction>,
    /// The send-now chord (Cmd or Ctrl with Enter).
    pub now: SendAction,
    /// Shown instead of a send button when `primary` is `None`.
    pub hint: Option<&'static str>,
}

/// The gestures for a steer mode and whether a turn is running.
pub fn steer_actions(mode: SteerMode, running: bool) -> SteerActions {
    if !running {
        return SteerActions {
            primary: Some(SendAction::Submit),
            now: SendAction::Submit,
            hint: None,
        };
    }
    match mode {
        SteerMode::AtBoundary => SteerActions {
            primary: Some(SendAction::SendAtBoundary),
            now: SendAction::SendNow,
            hint: None,
        },
        SteerMode::Native => SteerActions {
            primary: Some(SendAction::Steer),
            now: SendAction::SendNow,
            hint: None,
        },
        SteerMode::None => SteerActions {
            primary: None,
            now: SendAction::SendNow,
            hint: Some("This agent cannot take a message while it works"),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_composer_sends_with_either_gesture_in_every_mode() {
        for mode in [SteerMode::None, SteerMode::AtBoundary, SteerMode::Native] {
            let actions = steer_actions(mode, false);
            assert_eq!(actions.primary, Some(SendAction::Submit));
            assert_eq!(actions.now, SendAction::Submit);
            assert_eq!(actions.hint, None);
        }
    }

    #[test]
    fn running_composer_follows_the_steer_mode() {
        let boundary = steer_actions(SteerMode::AtBoundary, true);
        assert_eq!(boundary.primary, Some(SendAction::SendAtBoundary));
        assert_eq!(boundary.now, SendAction::SendNow);

        let native = steer_actions(SteerMode::Native, true);
        assert_eq!(native.primary, Some(SendAction::Steer));
        assert_eq!(native.now, SendAction::SendNow);

        let none = steer_actions(SteerMode::None, true);
        assert_eq!(none.primary, None);
        assert_eq!(none.now, SendAction::SendNow);
        assert!(none.hint.is_some());
    }

    #[test]
    fn only_send_now_is_labelled_as_cancelling() {
        assert_eq!(SendAction::SendNow.label(), "Send now (cancels)");
        assert_eq!(SendAction::SendAtBoundary.label(), "Send at boundary");
        assert!(!SendAction::Steer.label().contains("cancel"));
    }
}
