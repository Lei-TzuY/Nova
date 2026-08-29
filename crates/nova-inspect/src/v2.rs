//! Data model for semantic-inspection schema version 2.
//!
//! Version 2 preserves the complete v1 program projection and adds verified
//! function-level control-flow graphs. The schema remains tooling-owned rather
//! than exposing `nova-sema`'s Rust representation directly.

use crate::v1;
use serde::Serialize;

/// Stable schema family name carried by every document.
pub const SCHEMA_NAME: &str = v1::SCHEMA_NAME;

/// Numeric version of the schema in this module.
pub const SCHEMA_VERSION: u32 = 2;

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
    /// Stable HIR-derived facts, unchanged from schema v1.
    pub program: v1::Program,
    /// Verified control-flow graphs in HIR function order.
    pub control_flow: Vec<ControlFlowGraph>,
}

/// One verified function-level control-flow graph.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ControlFlowGraph {
    /// Document-local CFG identity derived from the owning function.
    pub id: String,
    /// Function represented by this graph.
    pub function: String,
    /// Unique graph-entry node.
    pub entry: String,
    /// Function-owned bindings participating in flow events.
    pub bindings: Vec<String>,
    /// Terminal nodes representing normal body completion.
    pub normal_exits: Vec<String>,
    /// Nodes in deterministic graph-local identity order.
    pub nodes: Vec<FlowNode>,
}

/// One node in a verified control-flow graph.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FlowNode {
    /// Document-local graph-node identity.
    pub id: String,
    /// Semantic action represented by the node.
    pub kind: FlowNodeKind,
    /// Resolved binding for an initialize or read event, otherwise absent.
    pub binding: Option<String>,
    /// Incoming edges in canonical predecessor order.
    pub predecessors: Vec<FlowEdge>,
    /// Source range associated with the action, when one exists.
    pub span: Option<v1::Span>,
}

/// Stable node categories represented in schema v2.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowNodeKind {
    /// Unique function entry.
    Entry,
    /// Entry into one branch or loop path.
    Branch,
    /// Continuation intersection or loop header.
    Join,
    /// A binding becomes definitely initialized.
    Initialize,
    /// A resolved binding is read.
    Read,
    /// Explicit function return.
    Return,
    /// Exit from the nearest loop.
    Break,
    /// Start the nearest loop's next condition test.
    Continue,
    /// Normal function-body completion.
    Exit,
}

/// One incoming control-flow edge.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct FlowEdge {
    /// Predecessor node identity.
    pub from: String,
    /// Reachability class of this edge.
    pub kind: FlowEdgeKind,
}

/// Stable edge categories represented in schema v2.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowEdgeKind {
    /// A path that may execute and contribute dataflow facts.
    Execution,
    /// Source retained only for deterministic static diagnostics.
    Diagnostic,
    /// Executable loop fallthrough or continue edge to a loop header.
    Backedge,
}
