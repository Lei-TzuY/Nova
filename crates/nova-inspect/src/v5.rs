//! Data model for semantic-inspection schema version 5.
//!
//! Version 5 preserves the v4 program, function-CFG, and match-pattern projections while
//! adding anonymous-closure definitions, immutable capture edges, and closure-owned CFGs.

use crate::{v1, v2, v3};
use serde::Serialize;

/// Stable schema family name carried by every document.
pub const SCHEMA_NAME: &str = v1::SCHEMA_NAME;

/// Numeric version of the schema in this module.
pub const SCHEMA_VERSION: u32 = 5;

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
    /// Stable semantic program facts using the established structural projection.
    pub program: v1::Program,
    /// Verified top-level function CFGs using the established v2 shape.
    pub control_flow: Vec<v2::ControlFlowGraph>,
    /// Explicit match payload modes using the established v3 shape.
    pub match_patterns: Vec<v3::MatchPattern>,
    /// Anonymous closures in deterministic semantic-traversal order.
    pub closures: Vec<Closure>,
    /// Verified closure-owned control-flow graphs.
    pub closure_control_flow: Vec<ClosureControlFlowGraph>,
}

/// One typed anonymous callable and its environment contract.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Closure {
    /// Document-local closure identity.
    pub id: String,
    /// Expression that creates this closure.
    pub expression: String,
    /// Structural callable type.
    pub type_id: String,
    /// Explicit return type.
    pub return_type: String,
    /// Parameter bindings in source order.
    pub parameters: Vec<String>,
    /// Immutable captures in first-lexical-use order.
    pub captures: Vec<ClosureCapture>,
    /// Closure body block.
    pub body: String,
    /// Complete anonymous-function range.
    pub span: v1::Span,
}

/// One binding copied into a closure environment at creation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ClosureCapture {
    /// Existing lexical binding identity.
    pub binding: String,
    /// Captured value type.
    pub type_id: String,
    /// First lexical use that establishes capture order.
    pub first_use: v1::Span,
}

/// One verified closure-level CFG.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ClosureControlFlowGraph {
    /// Document-local CFG identity derived from the owning closure.
    pub id: String,
    /// Closure represented by this graph.
    pub closure: String,
    /// Unique graph-entry node.
    pub entry: String,
    /// Captures and closure-owned bindings participating in flow events.
    pub bindings: Vec<String>,
    /// Terminal nodes representing normal body completion.
    pub normal_exits: Vec<String>,
    /// Nodes in deterministic graph-local identity order.
    pub nodes: Vec<v2::FlowNode>,
}
