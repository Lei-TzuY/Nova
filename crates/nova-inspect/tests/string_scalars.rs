use nova_inspect::v1::{ExpressionKind, TypeKind};
use nova_inspect::{build_document, build_document_v2, build_document_v3, build_document_v4};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze, hir};
use nova_source::{SourceFile, SourceId};

fn checked_analysis(text: &str) -> (SourceFile, AnalysisOutput) {
    let source = SourceFile::new(SourceId::new(0), "strings.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    (source, analyzed)
}

#[test]
fn schema_v4_projects_string_types_and_literals_while_older_schemas_fail_closed() {
    let (source, analyzed) = checked_analysis(
        r#"record Message { text: String }
           fn main() -> String { new Message { text: "Nova" }.text }"#,
    );

    let v1 = build_document(&analyzed.program, &source).expect_err("v1 must remain frozen");
    let v2 = build_document_v2(&analyzed, &source).expect_err("v2 must remain frozen");
    let v3 = build_document_v3(&analyzed, &source).expect_err("v3 must remain frozen");
    for error in [v1, v2, v3] {
        assert!(error.message().contains("select schema v4"), "{error}");
    }

    let document = build_document_v4(&analyzed, &source).expect("v4 represents String");
    assert_eq!(document.schema_version, 4);
    assert!(
        document
            .program
            .types
            .iter()
            .any(|ty| ty.kind == TypeKind::String && ty.display == "String")
    );
    assert!(
        document
            .program
            .expressions
            .iter()
            .any(|expression| expression.kind == ExpressionKind::String)
    );
    assert_eq!(document.control_flow.len(), 1);
}

#[test]
fn schema_v4_rejects_a_string_literal_with_a_forged_hir_type() {
    let (source, mut analyzed) = checked_analysis(r#"fn main() -> String { "Nova" }"#);
    analyzed.program.functions[0]
        .body
        .tail
        .as_deref_mut()
        .expect("string tail")
        .ty = hir::Type::Bool;

    let error = build_document_v4(&analyzed, &source).expect_err("forged HIR must fail closed");
    assert!(
        error
            .message()
            .contains("string literal expression has HIR type Bool instead of String")
    );
}

#[test]
fn published_v4_schema_names_only_the_new_string_categories() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/schemas/semantic-inspection-v4.schema.json"
    ))
    .expect("published schema must be valid JSON");

    assert_eq!(schema["$id"], "urn:nova:semantic-inspection:v4");
    assert_eq!(schema["properties"]["schema_version"]["const"], 4);
    assert_eq!(schema["properties"]["program"]["$ref"], "#/$defs/program");
    assert!(
        schema["$defs"]["type"]["properties"]["kind"]["enum"]
            .as_array()
            .expect("type kinds")
            .iter()
            .any(|kind| kind == "string")
    );
    assert!(
        schema["$defs"]["expression"]["properties"]["kind"]["enum"]
            .as_array()
            .expect("expression kinds")
            .iter()
            .any(|kind| kind == "string")
    );
}
