//! The computer science kit (feature `cs`): data structures and algorithms. Algorithms are
//! recorded once as steps ([`ArrayTrace`]) and the figures sample the steps by time, so
//! nothing runs again per frame. Figures are placed in units of `ui.space` and sized in
//! pixels. `docs/components/cs.md` has an example of each with its gallery scene.

mod array;
mod sort;
mod trace;

pub use array::{ArrayView, BinarySearch, CellColors};
pub use sort::SortVisualizer;
pub use trace::{ArrayStep, ArrayTrace, TraceAt};
