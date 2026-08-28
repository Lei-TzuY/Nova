//! Nova semantic analysis: HIR lowering, lexical name resolution, and bootstrap typing.

mod analyzer;
mod type_rules;

pub mod hir;

pub use analyzer::{AnalysisOutput, analyze};
