//! The computer science kit (feature `cs`): data structures and algorithms. Algorithms are
//! recorded once as steps ([`ArrayTrace`], [`ListTrace`], [`TreeTrace`]) and the figures sample the steps by time, so
//! nothing runs again per frame. Figures are placed in units of `ui.space` and sized in
//! pixels. `docs/components/cs.md` has an example of each with its gallery scene.

mod array;
mod keyed;
mod list;
mod sort;
mod trace;
mod tree;

pub use array::{ArrayView, BinarySearch, CellColors};
pub use keyed::StepAt;
pub use list::{LinkedList, ListRecorder, ListStep, ListTrace, Queue, Stack};
pub use sort::SortVisualizer;
pub use trace::{ArrayStep, ArrayTrace, TraceAt};
pub use tree::{BstRecorder, HeapRecorder, TreeNode, TreeStep, TreeTrace, TreeView};
