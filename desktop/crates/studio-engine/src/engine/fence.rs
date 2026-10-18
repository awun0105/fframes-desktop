//! Decisions the shell makes when a preview is ready: whether a fresh presentation still
//! matches the preparation in flight, and the nonblocking source fence for the commit.
use crate::Controller;
use parking_lot::Mutex;

/// What a completed command's fresh presentation means for the preparation in flight.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentationVerdict {
    /// Take the presentation; preparation (if any) is still the right one.
    Replace,
    /// The source moved on without a matching promotion: preparation is obsolete.
    CancelPreparation,
    /// Captured before a publication the shell already knows: it replaces nothing and
    /// cancels nothing (a fresh presentation is taken instead).
    Older,
}

/// Decides what a presentation captured by a command means. A refresh that raced a
/// promotion's handoff must not cancel the preparation adopted for exactly that
/// promotion: only preparation that is obsolete under the *returned* state is cancelled,
/// and a presentation older than what the shell already knows is ignored.
pub fn presentation_verdict(
    preview: &crate::PreviewState,
    known_generation: Option<u64>,
    incoming: Option<&crate::ProjectState>,
) -> PresentationVerdict {
    let incoming_generation = incoming.map(crate::ProjectState::generation);
    if let (Some(known), Some(new)) = (known_generation, incoming_generation)
        && new < known
    {
        return PresentationVerdict::Older;
    }
    if known_generation == incoming_generation {
        return PresentationVerdict::Replace;
    }
    if let Some(state) = incoming
        && let Some(promotion) = state.promotion()
        && preview.candidate() == Some(promotion.tag())
        && promotion.is_current(state)
    {
        return PresentationVerdict::Replace;
    }
    PresentationVerdict::CancelPreparation
}

/// The nonblocking source fence for committing a preview: the controller's guard, only
/// while it is free and no source scan is pending. Holding the guard excludes every
/// workflow publication and reconcile (they lock the same controller), so whatever is
/// validated against `guard.state()` is still true when the commit completes.
pub fn install_fence(
    controller: &Mutex<Controller>,
) -> Option<parking_lot::MutexGuard<'_, Controller>> {
    let guard = controller.try_lock()?;
    (!guard.changed_hint()).then_some(guard)
}
