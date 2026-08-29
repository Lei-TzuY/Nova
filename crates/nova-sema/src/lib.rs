//! Nova semantic analysis: HIR lowering, lexical name resolution, and bootstrap typing.

mod analyzer;
mod constant_condition;
mod constant_int;
pub mod control_flow;
mod type_rules;

pub mod hir;

pub use analyzer::{AnalysisOutput, analyze};
