//! Nova semantic analysis: HIR lowering, lexical name resolution, and bootstrap typing.

mod analyzer;
mod constant_condition;
mod constant_int;
pub mod control_flow;
#[cfg(test)]
mod control_flow_backedge_tests;
pub mod equality_rules;
mod numeric_surface;
mod type_rules;

pub mod hir;

pub use analyzer::AnalysisOutput;

/// Lowers a parsed program to HIR while resolving names and checking bootstrap types.
#[must_use]
pub fn analyze(program: &nova_parser::ast::Program) -> AnalysisOutput {
    analyze_in_module(program, hir::ModuleId::ROOT)
}

/// Lowers one parsed source as the specified compiler-session module.
///
/// Built-in numeric associated constants are canonicalized before ordinary semantic
/// name/type resolution so they share the exact same signed-64 literal contract as
/// source integer literals.
#[must_use]
pub fn analyze_in_module(
    program: &nova_parser::ast::Program,
    module: hir::ModuleId,
) -> AnalysisOutput {
    let program = numeric_surface::canonicalize_int_constants(program);
    analyzer::analyze_in_module(&program, module)
}
