//! Data model for semantic-inspection schema version 6.
//!
//! Version 6 preserves every v5 program, CFG, pattern, and closure fact while adding
//! the compiler-session module that owns all declaration and local identities.

use crate::{v1, v2, v3, v5};
use serde::Serialize;

/// Stable schema family name carried by every document.
pub const SCHEMA_NAME: &str = v1::SCHEMA_NAME;

/// Numeric version of the schema in this module.
pub const SCHEMA_VERSION: u32 = 6;

/// One complete semantic-inspection document.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Document {
    /// Schema family, always [`SCHEMA_NAME`].
    pub schema: String,
    /// Schema version, always [`SCHEMA_VERSION`].
    pub schema_version: u32,
    /// Compiler component that produced this document.
    pub producer: v1::Producer,
    /// The single source accepted by the bootstrap pipeline.
    pub source: v1::Source,
    /// Module ownership for every identity in this document.
    pub module: Module,
    /// Stable semantic program facts using the established structural projection.
    pub program: v1::Program,
    /// Verified top-level function CFGs using the established v2 shape.
    pub control_flow: Vec<v2::ControlFlowGraph>,
    /// Explicit match payload modes using the established v3 shape.
    pub match_patterns: Vec<v3::MatchPattern>,
    /// Anonymous closures in deterministic semantic-traversal order.
    pub closures: Vec<v5::Closure>,
    /// Verified closure-owned control-flow graphs.
    pub closure_control_flow: Vec<v5::ClosureControlFlowGraph>,
}

/// One compiler-session module and the document-local identities it owns.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Module {
    /// Compiler-session module identity.
    pub id: String,
    /// Source represented by this module.
    pub source: String,
    /// True only for the implicit root used by the current CLI pipeline.
    pub implicit_root: bool,
    /// Complete module/source range.
    pub span: v1::Span,
    /// Nominal record declarations owned by this module.
    pub records: Vec<String>,
    /// Nominal enum declarations owned by this module.
    pub enums: Vec<String>,
    /// Top-level function declarations owned by this module.
    pub functions: Vec<String>,
    /// Local, parameter, and match-payload declarations owned by this module.
    pub bindings: Vec<String>,
    /// Anonymous callable declarations owned by this module.
    pub closures: Vec<String>,
}
