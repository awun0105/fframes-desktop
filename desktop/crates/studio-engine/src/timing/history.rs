//! Snapshot undo and redo of the timing document (Cap's editor history): every step
//! stores the whole document, and a gesture (a drag) collects its changes into one step.
//! The props document uses the same history.
use super::doc::TimingDoc;

/// Undo steps kept; the oldest go first.
pub const MAX_HISTORY: usize = 100;

#[derive(Debug, Clone, PartialEq, Eq)]
enum Gesture<D> {
    None,
    /// The document when the gesture began.
    Active(D),
    /// The document changed outside the studio during the gesture: its remaining
    /// edits are refused until it ends.
    Aborted,
}

#[derive(Debug, Clone)]
pub struct History<D = TimingDoc> {
    undo: Vec<D>,
    redo: Vec<D>,
    gesture: Gesture<D>,
}

impl<D> Default for History<D> {
    fn default() -> Self {
        Self {
            undo: Vec::new(),
            redo: Vec::new(),
            gesture: Gesture::None,
        }
    }
}

impl<D: Clone + PartialEq> History<D> {
    pub fn can_undo(&self) -> bool {
        !self.undo.is_empty()
    }
    pub fn can_redo(&self) -> bool {
        !self.redo.is_empty()
    }
    pub fn in_gesture(&self) -> bool {
        self.gesture != Gesture::None
    }
    pub fn gesture_aborted(&self) -> bool {
        self.gesture == Gesture::Aborted
    }

    /// Starts collecting changes into one step. A second begin keeps the first start.
    pub fn begin_gesture(&mut self, current: &D) {
        if self.gesture == Gesture::None {
            self.gesture = Gesture::Active(current.clone());
        }
    }

    /// Ends the gesture; true when it changed the document and became one undo step.
    pub fn end_gesture(&mut self, current: &D) -> bool {
        match std::mem::replace(&mut self.gesture, Gesture::None) {
            Gesture::Active(start) if start != *current => {
                self.push(start);
                true
            }
            _ => false,
        }
    }

    /// Records `before` as an undo step for a change, unless a gesture collects it.
    pub fn record(&mut self, before: &D) {
        if self.gesture == Gesture::None {
            self.push(before.clone());
        }
    }

    fn push(&mut self, before: D) {
        self.undo.push(before);
        if self.undo.len() > MAX_HISTORY {
            self.undo.remove(0);
        }
        self.redo.clear();
    }

    /// The document to go back to, remembering `current` for redo.
    pub fn undo(&mut self, current: &D) -> Option<D> {
        let previous = self.undo.pop()?;
        self.redo.push(current.clone());
        Some(previous)
    }

    pub fn redo(&mut self, current: &D) -> Option<D> {
        let next = self.redo.pop()?;
        self.undo.push(current.clone());
        Some(next)
    }

    /// Forgets every step; a running gesture is aborted, not ended.
    pub fn clear(&mut self) {
        self.undo.clear();
        self.redo.clear();
        if self.gesture != Gesture::None {
            self.gesture = Gesture::Aborted;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::timing::doc::TimingClip;

    fn doc(trim: usize) -> TimingDoc {
        TimingDoc {
            clips: vec![TimingClip {
                key: "intro".into(),
                trim_start_frames: trim,
                trim_end_frames: 0,
            }],
            ..TimingDoc::default()
        }
    }

    #[test]
    fn a_gesture_is_one_step() {
        let mut history = History::default();
        history.begin_gesture(&doc(0));
        for trim in 1..=60 {
            history.record(&doc(trim - 1));
        }
        assert!(!history.can_undo());
        assert!(history.end_gesture(&doc(60)));
        assert_eq!(history.undo(&doc(60)), Some(doc(0)));
        assert!(!history.can_undo());
        assert_eq!(history.redo(&doc(0)), Some(doc(60)));
    }

    #[test]
    fn an_unchanged_gesture_leaves_no_step_and_new_changes_drop_redo() {
        let mut history = History::default();
        history.begin_gesture(&doc(0));
        assert!(!history.end_gesture(&doc(0)));
        history.record(&doc(0));
        history.record(&doc(1));
        assert_eq!(history.undo(&doc(2)), Some(doc(1)));
        assert!(history.can_redo());
        history.record(&doc(1));
        assert!(!history.can_redo());
    }

    #[test]
    fn clearing_aborts_a_gesture_until_it_ends() {
        let mut history = History::default();
        history.record(&doc(0));
        history.begin_gesture(&doc(1));
        history.clear();
        assert!(!history.can_undo());
        assert!(history.gesture_aborted());
        assert!(!history.end_gesture(&doc(5)));
        assert!(!history.in_gesture());
    }

    #[test]
    fn history_is_bounded() {
        let mut history = History::default();
        for trim in 0..MAX_HISTORY + 10 {
            history.record(&doc(trim));
        }
        let mut steps = 0;
        let mut current = doc(MAX_HISTORY + 10);
        while let Some(previous) = history.undo(&current) {
            current = previous;
            steps += 1;
        }
        assert_eq!(steps, MAX_HISTORY);
        assert_eq!(current, doc(10));
    }
}
