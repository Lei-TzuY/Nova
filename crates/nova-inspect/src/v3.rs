//! Data model for semantic-inspection schema version 3.
//!
//! Version 3 preserves the established program and CFG projections while adding
//! explicit pattern payload modes so tooling can distinguish binding, discard,
//! and payload-free variant arms without reinterpreting older schema fields.

use crate::{v1, v2};
use serde::Serialize;

/// Stable schema family name carried by every document.
pub const SCHEMA_NAME: &str = v1::SCHEMA_NAME;

/// Numeric version of the schema in this module.
pub const SCHEMA_VERSION: u32 = 3;

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
    /// Stable semantic program facts using the established v1 structural shape.
    pub program: v1::Program,
    /// Verified CFG projection using the established v2 structural shape.
    pub control_flow: Vec<v2::ControlFlowGraph>,
    /// Explicit pattern mode for every published exhaustive-match arm.
    pub match_patterns: Vec<MatchPattern>,
}

/// Tooling fact that makes one match arm's payload treatment explicit.
#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct MatchPattern {
    /// Existing document-local match-arm identity.
    pub arm: String,
    /// Whether the concrete variant has no payload, binds it, or discards it.
    pub payload_mode: MatchPayloadMode,
}

/// Stable payload-treatment categories introduced by schema v3.
#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchPayloadMode {
    /// The resolved variant is payload-free.
    None,
    /// The resolved payload is bound to the arm-local binding published in `program`.
    Bind,
    /// The resolved payload is explicitly discarded with `_` and creates no binding.
    Discard,
}
