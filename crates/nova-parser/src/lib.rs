//! AST and parser for the implemented Nova v0.1 frontend grammar.

mod parser;

pub mod ast;

pub use parser::{ParseOutput, parse};

/// Formats the AST using a deterministic, span-preserving debug tree.
///
/// This is intended for bootstrap inspection and is not a stable semantic
/// introspection schema.
#[must_use]
pub fn format_ast(program: &ast::Program) -> String {
    format!("{program:#?}")
}
