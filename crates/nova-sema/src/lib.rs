//! Nova semantic analysis: HIR lowering, lexical name resolution, and bootstrap typing.

mod analyzer;

pub mod hir;

pub use analyzer::{AnalysisOutput, analyze};
