//! The project's draft writer gate and the tool worker launch slot.
use super::*;

/// The project's draft writer gate. The orchestrator holds the guard while a provider
/// writer may modify the draft (from writer start until quiescence); tool calls that
/// need a fresh draft snapshot take it themselves and report contention as `busy`,
/// never waiting it out.
#[derive(Clone, Default)]
pub struct WriterGate(Arc<AtomicBool>);

pub struct WriterGuard(Arc<AtomicBool>);

impl WriterGate {
    pub fn try_acquire(&self) -> Option<WriterGuard> {
        self.0
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .ok()
            .map(|_| WriterGuard(self.0.clone()))
    }
}

impl Drop for WriterGuard {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

/// A launching worker's claim on one of the [`TOOL_WORKERS`] slots.
pub(super) struct LaunchSlot<'a>(pub(super) &'a AtomicUsize);

impl Drop for LaunchSlot<'_> {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::AcqRel);
    }
}
