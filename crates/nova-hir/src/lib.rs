//! Resolved high-level intermediate representation for the implemented Nova
//! semantic subset.
//!
//! This crate owns the first semantic compiler boundary. It lowers the parsed
//! AST into a purpose-built HIR while resolving value and built-in type names.
//! It deliberately performs no type inference or execution.

mod lower;

pub mod hir;

pub use lower::{LowerOutput, lower};

/// Formats resolved HIR using a deterministic, span-preserving debug tree.
///
/// The representation is useful for bootstrap tests and debugging. It is not
/// a versioned semantic-introspection protocol.
#[must_use]
pub fn format_hir(program: &hir::Program) -> String {
    format!("{program:#?}")
}
