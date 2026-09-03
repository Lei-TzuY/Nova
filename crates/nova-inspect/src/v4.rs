//! Data model for semantic-inspection schema version 4.
//!
//! Version 4 preserves the v3 CFG and match-pattern projections while extending
//! the program projection with the `string` type and expression categories.

use crate::{v1, v2, v3};
use serde::Serialize;

/// Stable schema family name carried by every document.
pub const SCHEMA_NAME: &str = v1::SCHEMA_NAME;

/// Numeric version of the schema in this module.
pub const SCHEMA_VERSION: u32 = 4;

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
    /// Stable semantic program facts extended with `String` categories.
    pub program: v1::Program,
    /// Verified CFG projection using the established v2 structural shape.
    pub control_flow: Vec<v2::ControlFlowGraph>,
    /// Explicit pattern modes using the established v3 structural shape.
    pub match_patterns: Vec<v3::MatchPattern>,
}
