from pathlib import Path
import json

# nova-inspect v8 data model
v8 = r'''//! Data model for semantic-inspection schema version 8.
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
'''
Path('crates/nova-inspect/src/v8.rs').write_text(v8)

p = Path('crates/nova-inspect/src/lib.rs')
text = p.read_text()
text = text.replace('pub mod v7;\n', 'pub mod v7;\npub mod v8;\n', 1)
# Existing build_parts callers now ignore capture-mode table.
text = text.replace('.build_parts()?;', '.build_parts()?;', 1)
text = text.replace('self.build_parts().map(|(document, _, _)| document)', 'self.build_parts().map(|(document, _, _, _)| document)', 1)
text = text.replace('let (program_document, match_patterns, _) = Builder::new', 'let (program_document, match_patterns, _, _) = Builder::new')
text = text.replace('let (program_document, match_patterns, closures) = Builder::new', 'let (program_document, match_patterns, closures, _) = Builder::new')
# Insert v8 builder after v7 renderer.
needle = '''pub fn render_json_v7(
    analysis: &AnalysisOutput,
    source: &SourceFile,
) -> Result<String, InspectionError> {
    let document = build_document_v7(analysis, source)?;
    serde_json::to_string_pretty(&document)
        .map_err(|error| InspectionError::invalid(format!("could not encode schema v7: {error}")))
}
'''
insert = needle + r'''

/// Builds schema v8, the first tooling contract that exposes shared mutable captures.
pub fn build_document_v8(
    analysis: &AnalysisOutput,
    source: &SourceFile,
) -> Result<v8::Document, InspectionError> {
    if !analysis.is_success() {
        return Err(InspectionError::invalid(
            "schema v8 requires a successful semantic analysis",
        ));
    }
    let (program_document, match_patterns, closures, capture_modes) =
        Builder::new(&analysis.program, source)
            .with_payload_discard()
            .with_string()
            .with_closures()
            .with_module_identity()
            .with_unsigned()
            .with_mutable_captures()
            .with_reference_captures()
            .build_parts()?;
    let control_flow = project_control_flow(
        &analysis.control_flow,
        &program_document.program,
        analysis.program.module.id,
        source,
    )?;
    let closure_control_flow = project_closure_control_flow(
        &analysis.control_flow,
        &program_document.program,
        &closures,
        analysis.program.module.id,
        source,
    )?;
    let module = v6::Module {
        id: module_id(analysis.program.module.id),
        source: program_document.source.id.clone(),
        implicit_root: analysis.program.module.id == hir::ModuleId::ROOT,
        span: program_document.program.span.clone(),
        records: program_document.program.records.iter().map(|record| record.id.clone()).collect(),
        enums: program_document.program.enums.iter().map(|item| item.id.clone()).collect(),
        functions: program_document.program.functions.iter().map(|function| function.id.clone()).collect(),
        bindings: program_document.program.bindings.iter().map(|binding| binding.id.clone()).collect(),
        closures: closures.iter().map(|closure| closure.id.clone()).collect(),
    };
    let closures = closures
        .into_iter()
        .zip(capture_modes)
        .map(|(closure, modes)| v8::Closure::from_v5(closure, modes))
        .collect();
    Ok(v8::Document {
        schema: v8::SCHEMA_NAME.to_owned(),
        schema_version: v8::SCHEMA_VERSION,
        producer: program_document.producer,
        source: program_document.source,
        module,
        program: program_document.program,
        control_flow,
        match_patterns,
        closures,
        closure_control_flow,
    })
}

/// Renders one successful analysis as deterministic schema-v8 JSON.
pub fn render_json_v8(
    analysis: &AnalysisOutput,
    source: &SourceFile,
) -> Result<String, InspectionError> {
    let document = build_document_v8(analysis, source)?;
    serde_json::to_string_pretty(&document)
        .map_err(|error| InspectionError::invalid(format!("could not encode schema v8: {error}")))
}
'''
if text.count(needle) != 1:
    raise SystemExit('v7 renderer anchor mismatch')
text = text.replace(needle, insert, 1)
# Builder fields/init/method.
text = text.replace('    allow_mutable_captures: bool,\n', '    allow_mutable_captures: bool,\n    allow_reference_captures: bool,\n    closure_capture_modes: Vec<Option<Vec<hir::CaptureMode>>>,\n', 1)
text = text.replace('            allow_mutable_captures: false,\n', '            allow_mutable_captures: false,\n            allow_reference_captures: false,\n            closure_capture_modes: Vec::new(),\n', 1)
needle2 = '''    fn with_mutable_captures(mut self) -> Self {
        self.allow_mutable_captures = true;
        self
    }
'''
insert2 = needle2 + '''
    fn with_reference_captures(mut self) -> Self {
        self.allow_reference_captures = true;
        self
    }
'''
text = text.replace(needle2, insert2, 1)
# build_parts signature and return.
text = text.replace('Result<(v1::Document, Vec<v3::MatchPattern>, Vec<v5::Closure>), InspectionError>', 'Result<(v1::Document, Vec<v3::MatchPattern>, Vec<v5::Closure>, Vec<Vec<hir::CaptureMode>>), InspectionError>', 1)
text = text.replace('        let closures = take_complete("closure", self.closures)?;\n', '        let closures = take_complete("closure", self.closures)?;\n        let closure_capture_modes = take_complete("closure capture modes", self.closure_capture_modes)?;\n', 1)
text = text.replace('        Ok((document, match_patterns, closures))', '        Ok((document, match_patterns, closures, closure_capture_modes))', 1)
# Closure projection: parallel modes and old-schema rejection.
text = text.replace('                self.closures.push(None);\n', '                self.closures.push(None);\n                self.closure_capture_modes.push(None);\n', 1)
text = text.replace('                let mut previous_first_use = None;\n                for capture in &closure.captures {', '                let mut previous_first_use = None;\n                let mut capture_modes = Vec::with_capacity(closure.captures.len());\n                for capture in &closure.captures {', 1)
anchor = '''                    if binding.mutable && !self.allow_mutable_captures {
                        return Err(InspectionError::invalid(
                            "semantic-inspection schema v5/v6 cannot represent a mutable-source snapshot capture; select schema v7",
                        ));
                    }
'''
replacement = anchor + '''                    if capture.mode == hir::CaptureMode::ByReference
                        && !self.allow_reference_captures
                    {
                        return Err(InspectionError::invalid(
                            "semantic-inspection schema v1-v7 cannot represent a by-reference closure capture; select schema v8",
                        ));
                    }
                    capture_modes.push(capture.mode);
'''
if text.count(anchor) != 1:
    raise SystemExit('capture validation anchor mismatch')
text = text.replace(anchor, replacement, 1)
assign = '''                self.closures[closure_index] = Some(v5::Closure {
'''
# Add mode table assignment after closure object closing, before target. Use exact tail.
tail = '''                    span: span.clone(),
                });
                target = Some(closure_owner);
'''
newtail = '''                    span: span.clone(),
                });
                self.closure_capture_modes[closure_index] = Some(capture_modes);
                target = Some(closure_owner);
'''
if text.count(tail) != 1:
    raise SystemExit('closure assignment tail mismatch')
text = text.replace(tail, newtail, 1)
p.write_text(text)

# CLI schema v8.
p = Path('crates/nova-cli/src/main.rs')
text = p.read_text()
text = text.replace('    render_json_v7 as render_semantic_json_v7,\n', '    render_json_v7 as render_semantic_json_v7, render_json_v8 as render_semantic_json_v8,\n', 1)
text = text.replace('[--schema-version 1|2|3|4|5|6|7]', '[--schema-version 1|2|3|4|5|6|7|8]')
text = text.replace('    V7,\n}', '    V7,\n    V8,\n}', 1)
text = text.replace('            InspectSchemaVersion::V7 => render_semantic_json_v7(&analyzed, &source),\n', '            InspectSchemaVersion::V7 => render_semantic_json_v7(&analyzed, &source),\n            InspectSchemaVersion::V8 => render_semantic_json_v8(&analyzed, &source),\n', 1)
text = text.replace('        "7" => Ok(InspectSchemaVersion::V7),\n', '        "7" => Ok(InspectSchemaVersion::V7),\n        "8" => Ok(InspectSchemaVersion::V8),\n', 1)
text = text.replace('expected one of `1`, `2`, `3`, `4`, `5`, `6`, or `7`', 'expected one of `1`, `2`, `3`, `4`, `5`, `6`, `7`, or `8`')
p.write_text(text)

# Published JSON schema v8 derives from frozen v7.
src = Path('docs/schemas/semantic-inspection-v7.schema.json')
data = json.loads(src.read_text())
data['$id'] = 'urn:nova:semantic-inspection:v8'
data['title'] = 'Nova semantic inspection v8'
data['properties']['schema_version']['const'] = 8
mode = data['$defs']['closureCapture']['properties']['mode']
mode.clear(); mode['enum'] = ['by_value', 'by_reference']
Path('docs/schemas/semantic-inspection-v8.schema.json').write_text(json.dumps(data, indent=2) + '\n')

# Integration/contract tests.
test = r'''use nova_inspect::{build_document_v7, build_document_v8, render_json_v8, v8};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::hir::CaptureMode as HirCaptureMode;
use nova_sema::analyze;
use nova_source::{SourceFile, SourceId};

fn analysis(text: &str) -> (SourceFile, nova_sema::AnalysisOutput) {
    let source = SourceFile::new(SourceId::new(0), "shared.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    (source, analyzed)
}

#[test]
fn v8_projects_by_reference_while_v7_fails_closed() {
    let (source, analyzed) = analysis(
        "fn main() -> Int { var value = 1; let bump = fn() -> Int { value = value + 1; value }; bump() }",
    );
    let error = build_document_v7(&analyzed, &source).expect_err("v7 must stay frozen");
    assert!(error.message().contains("schema v8"), "{error}");
    let document = build_document_v8(&analyzed, &source).expect("v8 supports shared captures");
    assert_eq!(document.schema_version, 8);
    assert_eq!(document.closures[0].captures[0].mode, v8::CaptureMode::ByReference);
    let first = render_json_v8(&analyzed, &source).expect("first render");
    let second = render_json_v8(&analyzed, &source).expect("second render");
    assert_eq!(first, second);
    assert!(first.contains("\"mode\": \"by_reference\""));
}

#[test]
fn v8_keeps_read_only_mutable_capture_by_value() {
    let (source, analyzed) = analysis(
        "fn main() -> Int { var value = 1; let get = fn() -> Int { value }; value = 2; get() }",
    );
    let document = build_document_v8(&analyzed, &source).expect("v8 document");
    assert_eq!(document.closures[0].captures[0].mode, v8::CaptureMode::ByValue);
}

#[test]
fn v8_rejects_malformed_by_reference_capture_of_immutable_binding() {
    let (source, mut analyzed) = analysis(
        "fn main() -> Int { let value = 1; let get = fn() -> Int { value }; get() }",
    );
    let closure = match &mut analyzed.program.functions[0].body.statements[1].kind {
        nova_sema::hir::StatementKind::Binding { initializer, .. } => match &mut initializer.kind {
            nova_sema::hir::ExpressionKind::Closure(closure) => closure,
            _ => panic!("closure initializer"),
        },
        _ => panic!("closure binding"),
    };
    closure.captures[0].mode = HirCaptureMode::ByReference;
    let error = build_document_v8(&analyzed, &source).expect_err("malformed HIR must fail closed");
    assert!(error.message().contains("immutable") || error.message().contains("by-reference"), "{error}");
}
'''
Path('crates/nova-inspect/tests/shared_capture_schema_v8.rs').write_text(test)
