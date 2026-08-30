from pathlib import Path
import json


def replace_once(path: str, old: str, new: str) -> None:
    p = Path(path)
    text = p.read_text()
    count = text.count(old)
    if count != 1:
        raise RuntimeError(f"expected one anchor in {path}, found {count}: {old[:80]!r}")
    p.write_text(text.replace(old, new, 1))

# v3 data model: v1-shaped program + v2 CFG + explicit match payload modes.
Path("crates/nova-inspect/src/v3.rs").write_text(r'''//! Data model for semantic-inspection schema version 3.
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
''')

# Inspector module + v3 API.
replace_once(
    "crates/nova-inspect/src/lib.rs",
    "pub mod v1;\npub mod v2;\n",
    "pub mod v1;\npub mod v2;\npub mod v3;\n",
)
insert_after = '''pub fn render_json_v2(\n    analysis: &AnalysisOutput,\n    source: &SourceFile,\n) -> Result<String, InspectionError> {\n    let document = build_document_v2(analysis, source)?;\n    serde_json::to_string_pretty(&document)\n        .map_err(|error| InspectionError::invalid(format!("could not encode schema v2: {error}")))\n}\n'''
addition = insert_after + r'''

/// Builds a schema-v3 document from one successful semantic analysis.
///
/// Version 3 preserves the established program and CFG structural projections and adds
/// explicit match-arm payload modes. This is the first inspection schema that can
/// represent `Enum::Variant(_)` payload discard without reinterpreting v1/v2 fields.
pub fn build_document_v3(
    analysis: &AnalysisOutput,
    source: &SourceFile,
) -> Result<v3::Document, InspectionError> {
    if !analysis.is_success() {
        return Err(InspectionError::invalid(
            "schema v3 requires a successful semantic analysis",
        ));
    }

    let (program_document, match_patterns) = Builder::new(&analysis.program, source)
        .with_payload_discard()
        .build_parts()?;
    let control_flow =
        project_control_flow(&analysis.control_flow, &program_document.program, source)?;
    Ok(v3::Document {
        schema: v3::SCHEMA_NAME.to_owned(),
        schema_version: v3::SCHEMA_VERSION,
        producer: program_document.producer,
        source: program_document.source,
        program: program_document.program,
        control_flow,
        match_patterns,
    })
}

/// Renders one successful analysis as deterministic, pretty-printed schema-v3 JSON.
pub fn render_json_v3(
    analysis: &AnalysisOutput,
    source: &SourceFile,
) -> Result<String, InspectionError> {
    let document = build_document_v3(analysis, source)?;
    serde_json::to_string_pretty(&document)
        .map_err(|error| InspectionError::invalid(format!("could not encode schema v3: {error}")))
}
'''
replace_once("crates/nova-inspect/src/lib.rs", insert_after, addition)

replace_once(
    "crates/nova-inspect/src/lib.rs",
    '''    matches: Vec<Option<v1::Match>>,\n    active_scopes: Vec<String>,\n    loop_depth: usize,\n''',
    '''    matches: Vec<Option<v1::Match>>,\n    match_patterns: Vec<v3::MatchPattern>,\n    allow_payload_discard: bool,\n    active_scopes: Vec<String>,\n    loop_depth: usize,\n''',
)
replace_once(
    "crates/nova-inspect/src/lib.rs",
    '''            matches: Vec::new(),\n            active_scopes: Vec::new(),\n            loop_depth: 0,\n''',
    '''            matches: Vec::new(),\n            match_patterns: Vec::new(),\n            allow_payload_discard: false,\n            active_scopes: Vec::new(),\n            loop_depth: 0,\n''',
)
replace_once(
    "crates/nova-inspect/src/lib.rs",
    '''    fn build(mut self) -> Result<v1::Document, InspectionError> {\n        let program_span = self.span(self.program.span)?;\n''',
    '''    fn with_payload_discard(mut self) -> Self {\n        self.allow_payload_discard = true;\n        self\n    }\n\n    fn build(self) -> Result<v1::Document, InspectionError> {\n        self.build_parts().map(|(document, _)| document)\n    }\n\n    fn build_parts(mut self) -> Result<(v1::Document, Vec<v3::MatchPattern>), InspectionError> {\n        let program_span = self.span(self.program.span)?;\n''',
)
replace_once(
    "crates/nova-inspect/src/lib.rs",
    '''        Ok(v1::Document {\n            schema: v1::SCHEMA_NAME.to_owned(),\n''',
    '''        let match_patterns = self.match_patterns;\n        let document = v1::Document {\n            schema: v1::SCHEMA_NAME.to_owned(),\n''',
)
replace_once(
    "crates/nova-inspect/src/lib.rs",
    '''                matches,\n            },\n        })\n    }\n\n    fn prepare_type_order''',
    '''                matches,\n            },\n        };\n        Ok((document, match_patterns))\n    }\n\n    fn prepare_type_order''',
)

old_match = '''                        let binding = match (\n                            &variant.payload,\n                            &arm.binding,\n                            arm.payload_discarded,\n                        ) {\n                            (Some(expected), Some(binding), false) => {\n                                if &binding.ty != expected {\n                                    return Err(InspectionError::invalid(format!(\n                                        "match payload binding type does not match {}",\n                                        variant_id(enumeration.index(), arm.variant_index)\n                                    )));\n                                }\n                                Some(self.add_binding(\n                                    binding,\n                                    v1::BindingRole::MatchPayload,\n                                    owner,\n                                    &arm_identity,\n                                )?)\n                            }\n                            (Some(_), None, true) | (None, None, false) => None,\n                            _ => {\n                                return Err(InspectionError::invalid(format!(\n                                    "match payload mode does not match {}",\n                                    variant_id(enumeration.index(), arm.variant_index)\n                                )));\n                            }\n                        };\n                        let value = self.collect_expression(&arm.value, owner)?;\n                        Ok::<_, InspectionError>((binding, value))\n'''
new_match = '''                        let (binding, payload_mode) = match (\n                            &variant.payload,\n                            &arm.binding,\n                            arm.payload_discarded,\n                        ) {\n                            (Some(expected), Some(binding), false) => {\n                                if &binding.ty != expected {\n                                    return Err(InspectionError::invalid(format!(\n                                        "match payload binding type does not match {}",\n                                        variant_id(enumeration.index(), arm.variant_index)\n                                    )));\n                                }\n                                (\n                                    Some(self.add_binding(\n                                        binding,\n                                        v1::BindingRole::MatchPayload,\n                                        owner,\n                                        &arm_identity,\n                                    )?),\n                                    v3::MatchPayloadMode::Bind,\n                                )\n                            }\n                            (Some(_), None, true) if self.allow_payload_discard => {\n                                (None, v3::MatchPayloadMode::Discard)\n                            }\n                            (Some(_), None, true) => {\n                                return Err(InspectionError::invalid(\n                                    "semantic-inspection schema v1/v2 cannot represent an explicitly discarded enum payload; select schema v3",\n                                ));\n                            }\n                            (None, None, false) => (None, v3::MatchPayloadMode::None),\n                            _ => {\n                                return Err(InspectionError::invalid(format!(\n                                    "match payload mode does not match {}",\n                                    variant_id(enumeration.index(), arm.variant_index)\n                                )));\n                            }\n                        };\n                        let value = self.collect_expression(&arm.value, owner)?;\n                        Ok::<_, InspectionError>((binding, value, payload_mode))\n'''
replace_once("crates/nova-inspect/src/lib.rs", old_match, new_match)
replace_once(
    "crates/nova-inspect/src/lib.rs",
    '''                    let (binding, value) = arm_contents?;\n                    children.push(value.clone());\n''',
    '''                    let (binding, value, payload_mode) = arm_contents?;\n                    self.match_patterns.push(v3::MatchPattern {\n                        arm: arm_identity.clone(),\n                        payload_mode,\n                    });\n                    children.push(value.clone());\n''',
)

# Update inspector focused test to preserve old schemas and prove v3 representation.
Path("crates/nova-inspect/tests/pattern_payload_discard.rs").write_text(r'''use nova_inspect::{build_document, build_document_v2, build_document_v3};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{analyze, hir::ExpressionKind};
use nova_source::{SourceFile, SourceId};

fn accepted(text: &str) -> (SourceFile, nova_sema::AnalysisOutput) {
    let source = SourceFile::new(SourceId::new(0), "pattern-payload-discard.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    (source, analyzed)
}

#[test]
fn v1_and_v2_refuse_to_reinterpret_payload_binding_null_as_discard() {
    let (source, analyzed) = accepted("enum Maybe { None, Some(Int) } fn main() -> Int { match Maybe::Some(7) { Maybe::None => 0, Maybe::Some(_) => 1 } }");
    let v1 = build_document(&analyzed.program, &source).expect_err("v1 cannot encode discard");
    assert!(v1.message().contains("select schema v3"), "{}", v1.message());
    let v2 = build_document_v2(&analyzed, &source).expect_err("v2 cannot encode discard");
    assert!(v2.message().contains("select schema v3"), "{}", v2.message());
}

#[test]
fn schema_v3_projects_explicit_payload_modes_without_adding_a_binding() {
    let (source, analyzed) = accepted("enum Maybe { None, Some(Int) } fn main() -> Int { match Maybe::Some(7) { Maybe::None => 0, Maybe::Some(_) => 1 } }");
    let document = build_document_v3(&analyzed, &source).expect("v3 must represent discard");
    assert_eq!(document.schema_version, 3);
    assert!(document.program.matches[0].arms[1].binding.is_none());
    assert_eq!(document.match_patterns.len(), 2);
    assert_eq!(document.match_patterns[0].arm, "match:0.arm:0");
    assert_eq!(document.match_patterns[0].payload_mode, nova_inspect::v3::MatchPayloadMode::None);
    assert_eq!(document.match_patterns[1].payload_mode, nova_inspect::v3::MatchPayloadMode::Discard);
    assert_eq!(document.control_flow.len(), document.program.functions.len());
}

#[test]
fn inspector_rejects_discard_metadata_removed_from_payload_variant() {
    let (source, mut analyzed) = accepted("enum Maybe { None, Some(Int) } fn main() -> Int { match Maybe::Some(7) { Maybe::None => 0, Maybe::Some(_) => 1 } }");
    let tail = analyzed.program.functions[0].body.tail.as_deref_mut().expect("tail");
    let ExpressionKind::Match { arms, .. } = &mut tail.kind else { panic!("expected match"); };
    arms[1].payload_discarded = false;
    let error = build_document_v3(&analyzed, &source).expect_err("malformed HIR must fail");
    assert!(error.message().contains("match payload mode does not match"), "{}", error.message());
}
''')

# Add v3 schema regression to internal inspector tests and imports.
replace_once(
    "crates/nova-inspect/src/lib.rs",
    '''    use super::{build_document, build_document_v2, render_json, render_json_v2};\n''',
    '''    use super::{\n        build_document, build_document_v2, build_document_v3, render_json, render_json_v2,\n        render_json_v3,\n    };\n''',
)
v3_test = r'''

    #[test]
    fn published_json_schema_is_well_formed_and_names_v3() {
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../../docs/schemas/semantic-inspection-v3.schema.json"
        ))
        .expect("published schema must be valid JSON");

        assert_eq!(schema["$id"], "urn:nova:semantic-inspection:v3");
        assert_eq!(schema["properties"]["schema_version"]["const"], 3);
        assert_eq!(
            schema["properties"]["program"]["$ref"],
            "urn:nova:semantic-inspection:v1#/$defs/program"
        );
        assert_eq!(
            schema["properties"]["control_flow"]["items"]["$ref"],
            "urn:nova:semantic-inspection:v2#/$defs/controlFlowGraph"
        );

        let (source, analyzed) = checked_analysis(
            "enum Maybe { None, Some(Int) } fn main() -> Int { match Maybe::Some(1) { Maybe::None => 0, Maybe::Some(_) => 1 } }",
        );
        let document = serde_json::to_value(
            build_document_v3(&analyzed, &source).expect("valid v3 analysis should inspect"),
        )
        .expect("document should serialize");
        assert_required_keys(&schema, &document);
        assert_required_keys(
            &schema["$defs"]["matchPattern"],
            &document["match_patterns"][0],
        );
        assert_eq!(document["match_patterns"][1]["payload_mode"], "discard");

        let first = render_json_v3(&analyzed, &source).expect("v3 JSON should render");
        let second = render_json_v3(&analyzed, &source).expect("v3 JSON should repeat");
        assert_eq!(first, second);
    }
'''
replace_once(
    "crates/nova-inspect/src/lib.rs",
    '''    fn assert_required_keys(schema: &serde_json::Value, value: &serde_json::Value) {\n''',
    v3_test + '''\n    fn assert_required_keys(schema: &serde_json::Value, value: &serde_json::Value) {\n''',
)

# Published JSON schema v3. It references the frozen v1 program and v2 CFG definitions.
schema_v3 = {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "$id": "urn:nova:semantic-inspection:v3",
    "title": "Nova semantic inspection v3",
    "type": "object",
    "additionalProperties": False,
    "required": ["schema", "schema_version", "producer", "source", "program", "control_flow", "match_patterns"],
    "properties": {
        "schema": {"const": "nova.semantic-inspection"},
        "schema_version": {"const": 3},
        "producer": {"$ref": "urn:nova:semantic-inspection:v1#/$defs/producer"},
        "source": {"$ref": "urn:nova:semantic-inspection:v1#/$defs/source"},
        "program": {"$ref": "urn:nova:semantic-inspection:v1#/$defs/program"},
        "control_flow": {
            "type": "array",
            "items": {"$ref": "urn:nova:semantic-inspection:v2#/$defs/controlFlowGraph"},
        },
        "match_patterns": {
            "type": "array",
            "items": {"$ref": "#/$defs/matchPattern"},
        },
    },
    "$defs": {
        "matchArmId": {"type": "string", "pattern": r"^match:[0-9]+\.arm:[0-9]+$"},
        "matchPattern": {
            "type": "object",
            "additionalProperties": False,
            "required": ["arm", "payload_mode"],
            "properties": {
                "arm": {"$ref": "#/$defs/matchArmId"},
                "payload_mode": {"enum": ["none", "bind", "discard"]},
            },
        },
    },
}
Path("docs/schemas/semantic-inspection-v3.schema.json").write_text(json.dumps(schema_v3, indent=2) + "\n")

# CLI: explicit v3 selection; default remains v1.
replace_once(
    "crates/nova-cli/src/main.rs",
    '''use nova_inspect::{\n    render_json as render_semantic_json, render_json_v2 as render_semantic_json_v2,\n};\n''',
    '''use nova_inspect::{\n    render_json as render_semantic_json, render_json_v2 as render_semantic_json_v2,\n    render_json_v3 as render_semantic_json_v3,\n};\n''',
)
replace_once(
    "crates/nova-cli/src/main.rs",
    "nova inspect <file> --format json [--schema-version 1|2] [--message-format human|json]",
    "nova inspect <file> --format json [--schema-version 1|2|3] [--message-format human|json]",
)
replace_once(
    "crates/nova-cli/src/main.rs",
    '''enum InspectSchemaVersion {\n    V1,\n    V2,\n}\n''',
    '''enum InspectSchemaVersion {\n    V1,\n    V2,\n    V3,\n}\n''',
)
replace_once(
    "crates/nova-cli/src/main.rs",
    '''            InspectSchemaVersion::V1 => render_semantic_json(&analyzed.program, &source),\n            InspectSchemaVersion::V2 => render_semantic_json_v2(&analyzed, &source),\n''',
    '''            InspectSchemaVersion::V1 => render_semantic_json(&analyzed.program, &source),\n            InspectSchemaVersion::V2 => render_semantic_json_v2(&analyzed, &source),\n            InspectSchemaVersion::V3 => render_semantic_json_v3(&analyzed, &source),\n''',
)
replace_once(
    "crates/nova-cli/src/main.rs",
    '''                return Err("`--schema-version` requires `1` or `2`".to_owned());\n''',
    '''                return Err("`--schema-version` requires `1`, `2`, or `3`".to_owned());\n''',
)
replace_once(
    "crates/nova-cli/src/main.rs",
    '''        "1" => Ok(InspectSchemaVersion::V1),\n        "2" => Ok(InspectSchemaVersion::V2),\n        _ => Err(format!(\n            "unsupported inspection schema version `{value}`; expected `1` or `2`"\n        )),\n''',
    '''        "1" => Ok(InspectSchemaVersion::V1),\n        "2" => Ok(InspectSchemaVersion::V2),\n        "3" => Ok(InspectSchemaVersion::V3),\n        _ => Err(format!(\n            "unsupported inspection schema version `{value}`; expected `1`, `2`, or `3`"\n        )),\n''',
)

# CLI executable fixture and v3 compatibility test.
Path("crates/nova-cli/tests/fixtures/valid/pattern-payload-discard.nv").write_text(r'''enum Packet {
    Empty,
    Data(Int),
}

fn score(packet: Packet) -> Int {
    match packet {
        Packet::Empty => 0,
        Packet::Data(_) => 42,
    }
}

fn main() -> Int {
    score(Packet::Data(99))
}
''')
replace_once(
    "crates/nova-cli/tests/cli.rs",
    '''        "valid/higher-order-functions.nv",\n''',
    '''        "valid/higher-order-functions.nv",\n        "valid/pattern-payload-discard.nv",\n''',
)
replace_once(
    "crates/nova-cli/tests/cli.rs",
    '''        ("valid/higher-order-functions.nv", "42\\n", None),\n''',
    '''        ("valid/higher-order-functions.nv", "42\\n", None),\n        ("valid/pattern-payload-discard.nv", "42\\n", None),\n''',
)
replace_once(
    "crates/nova-cli/tests/cli.rs",
    '''    for version in ["1", "2"] {\n        let inspected = nova(&[\n''',
    '''    for version in ["1", "2", "3"] {\n        let inspected = nova(&[\n''',
)
replace_once(
    "crates/nova-cli/tests/cli.rs",
    '''        assert_eq!(document.contains("\\\"control_flow\\\":"), version == "2");\n''',
    '''        assert_eq!(document.contains("\\\"control_flow\\\":"), version != "1");\n        assert_eq!(document.contains("\\\"match_patterns\\\":"), version == "3");\n''',
)
# Second ["1","2"] occurrence is invalid-source loop; include v3 there too.
replace_once(
    "crates/nova-cli/tests/cli.rs",
    '''    for version in ["1", "2"] {\n        let output = nova(&[\n''',
    '''    for version in ["1", "2", "3"] {\n        let output = nova(&[\n''',
)
v3_cli_test = r'''

#[test]
fn inspect_schema_v3_represents_payload_discard_without_reinterpreting_v1_or_v2() {
    let path = fixture("valid/pattern-payload-discard.nv");
    let path = path.to_str().expect("fixture path is UTF-8");

    for version in ["1", "2"] {
        let output = nova(&[
            "inspect",
            path,
            "--format=json",
            "--schema-version",
            version,
        ]);
        assert!(!output.status.success(), "legacy schema {version} must reject discard");
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(stderr.contains("N5001"), "{stderr}");
        assert!(stderr.contains("select schema v3"), "{stderr}");
    }

    let output = nova(&[
        "inspect",
        path,
        "--format=json",
        "--schema-version=3",
    ]);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(output.stderr.is_empty());
    let stdout = String::from_utf8(output.stdout).expect("v3 output is UTF-8");
    assert!(stdout.contains("\"schema_version\": 3"));
    assert!(stdout.contains("\"control_flow\":"));
    assert!(stdout.contains("\"match_patterns\":"));
    assert!(stdout.contains("\"payload_mode\": \"discard\""));
}
'''
replace_once(
    "crates/nova-cli/tests/cli.rs",
    '''#[test]\nfn inspect_rejects_invalid_source_without_partial_output() {\n''',
    v3_cli_test + '''\n#[test]\nfn inspect_rejects_invalid_source_without_partial_output() {\n''',
)
