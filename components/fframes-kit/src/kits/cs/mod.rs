//! The computer science kit (feature `cs`): data structures and algorithms. Algorithms are
//! recorded once as steps ([`ArrayTrace`], [`ListTrace`], [`TreeTrace`], [`GraphTrace`],
//! [`HashTrace`], [`MachineTrace`], [`CallTrace`]) and the figures sample the steps by time,
//! so nothing runs again per frame. Figures are placed in units of `ui.space` and sized in
//! pixels. `docs/components/cs.md` has an example of each with its gallery scene.

mod array;
mod code;
mod complexity;
mod graph;
mod hash;
mod keyed;
mod list;
mod machine;
mod recursion;
mod sequence;
mod sort;
mod trace;
mod tree;

pub use array::{ArrayView, BinarySearch, CellColors};
pub use code::{CodeBlock, CodeColors, Language, Token, tokenize};
pub use complexity::{ComplexityPlot, Growth};
pub use graph::{Graph, GraphStep, GraphTrace, GraphView};
pub use hash::{HashRecorder, HashStep, HashTable, HashTrace};
pub use keyed::StepAt;
pub use list::{LinkedList, ListRecorder, ListStep, ListTrace, Queue, Stack};
pub use machine::{Machine, MachineStep, MachineTrace, StateMachine};
pub use recursion::{Call, CallRecorder, CallStack, CallStep, CallTrace, Recursion, RecursionTree};
pub use sequence::{Message, SequenceDiagram};
pub use sort::SortVisualizer;
pub use trace::{ArrayStep, ArrayTrace, TraceAt};
pub use tree::{BstRecorder, HeapRecorder, TreeNode, TreeStep, TreeTrace, TreeView};
