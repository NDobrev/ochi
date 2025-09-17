pub mod analyze;
pub mod model;

// Re-export commonly used types/functions for consumers (GUI)
pub use analyze::{analyze_entries, Block, EdgeKind, EdgeOut, FunctionOut, Report};
pub use model::{load_raw_bin, read_u8, read_u32, Image};

