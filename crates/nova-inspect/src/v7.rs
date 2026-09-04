//! Data model for semantic-inspection schema version 7.
//!
//! Version 7 preserves every v6 module, program, CFG, pattern, and closure fact
//! while adding the `UInt` type, its executable expression categories, and an
//! explicit by-value mode for closure captures, including mutable-source snapshots.

use crate::{v1, v2, v3, v5, v6};
use serde::Serialize;

/// Stable schema family name carried by every document.
pub const SCHEMA_NAME: &str = v1::SCHEMA_NAME;

/// Numeric version of the schema in this module.
pub const SCHEMA_VERSION: u32 = 7;

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
    pub module: v6::Module,
    /// Stable semantic program facts using the established structural projection.
    pub program: v1::Program,
    /// Verified top-level function CFGs using the established v2 shape.
    pub control_flow: Vec<v2::ControlFlowGraph>,
    /// Explicit match payload modes using the established v3 shape.
    pub match_patterns: Vec<v3::MatchPattern>,
    /// Anonymous closures in deterministic semantic-traversal order.
    pub closures: Vec<Closure>,
    /// Verified closure-owned control-flow graphs.
    pub closure_control_flow: Vec<v5::ClosureControlFlowGraph>,
}

/// One typed anonymous callable and its explicit environment contract.
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
    /// By-value captures in first-lexical-use order.
    pub captures: Vec<ClosureCapture>,
    /// Closure body block.
    pub body: String,
    /// Complete anonymous-function range.
    pub span: v1::Span,
}

impl From<v5::Closure> for Closure {
    fn from(closure: v5::Closure) -> Self {
        Self {
            id: closure.id,
            expression: closure.expression,
            type_id: closure.type_id,
            return_type: closure.return_type,
            parameters: closure.parameters,
            captures: closure
                .captures
                .into_iter()
                .map(ClosureCapture::from)
                .collect(),
            body: closure.body,
            span: closure.span,
        }
    }
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
    /// Environment transfer mode, currently always [`CaptureMode::ByValue`].
    pub mode: CaptureMode,
}

impl From<v5::ClosureCapture> for ClosureCapture {
    fn from(capture: v5::ClosureCapture) -> Self {
        Self {
            binding: capture.binding,
            type_id: capture.type_id,
            first_use: capture.first_use,
            mode: CaptureMode::ByValue,
        }
    }
}

/// How one lexical value enters the closure environment.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureMode {
    /// Copy the value when the closure expression is evaluated.
    ByValue,
}
