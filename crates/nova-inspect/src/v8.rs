//! Data model for semantic-inspection schema version 8.
//!
//! Version 8 preserves v7 and exposes whether each closure capture is a
//! creation-time snapshot or a shared by-reference mutable cell.

use crate::{v1, v2, v3, v5, v6};
use nova_sema::hir;
use serde::Serialize;

pub const SCHEMA_NAME: &str = v1::SCHEMA_NAME;
pub const SCHEMA_VERSION: u32 = 8;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Document {
    pub schema: String,
    pub schema_version: u32,
    pub producer: v1::Producer,
    pub source: v1::Source,
    pub module: v6::Module,
    pub program: v1::Program,
    pub control_flow: Vec<v2::ControlFlowGraph>,
    pub match_patterns: Vec<v3::MatchPattern>,
    pub closures: Vec<Closure>,
    pub closure_control_flow: Vec<v5::ClosureControlFlowGraph>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Closure {
    pub id: String,
    pub expression: String,
    pub type_id: String,
    pub return_type: String,
    pub parameters: Vec<String>,
    pub captures: Vec<ClosureCapture>,
    pub body: String,
    pub span: v1::Span,
}

impl Closure {
    pub(crate) fn from_v5(closure: v5::Closure, modes: Vec<hir::CaptureMode>) -> Self {
        debug_assert_eq!(closure.captures.len(), modes.len());
        let captures = closure
            .captures
            .into_iter()
            .zip(modes)
            .map(|(capture, mode)| ClosureCapture {
                binding: capture.binding,
                type_id: capture.type_id,
                first_use: capture.first_use,
                mode: mode.into(),
            })
            .collect();
        Self {
            id: closure.id,
            expression: closure.expression,
            type_id: closure.type_id,
            return_type: closure.return_type,
            parameters: closure.parameters,
            captures,
            body: closure.body,
            span: closure.span,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ClosureCapture {
    pub binding: String,
    pub type_id: String,
    pub first_use: v1::Span,
    pub mode: CaptureMode,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CaptureMode {
    ByValue,
    ByReference,
}

impl From<hir::CaptureMode> for CaptureMode {
    fn from(mode: hir::CaptureMode) -> Self {
        match mode {
            hir::CaptureMode::ByValue => Self::ByValue,
            hir::CaptureMode::ByReference => Self::ByReference,
        }
    }
}
