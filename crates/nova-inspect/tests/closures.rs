use nova_inspect::{
    build_document, build_document_v2, build_document_v3, build_document_v4, build_document_v5,
};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::analyze;
use nova_sema::hir::{ExpressionKind, StatementKind, Type};
use nova_source::{SourceFile, SourceId};

fn analyzed(text: &str) -> (SourceFile, nova_sema::AnalysisOutput) {
    let source = SourceFile::new(SourceId::new(0), "closures.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    (source, analyzed)
}

#[test]
fn v5_projects_closure_captures_and_cfgs_while_v1_through_v4_fail_closed() {
    let (source, analysis) =
        analyzed("fn make(base: Int) -> fn(Int) -> Int { fn(value: Int) -> Int { base + value } }");

    let errors = [
        build_document(&analysis.program, &source).expect_err("v1 must reject closures"),
        build_document_v2(&analysis, &source).expect_err("v2 must reject closures"),
        build_document_v3(&analysis, &source).expect_err("v3 must reject closures"),
        build_document_v4(&analysis, &source).expect_err("v4 must reject closures"),
    ];
    assert!(
        errors
            .iter()
            .all(|error| error.message().contains("select schema v5")),
        "{errors:?}"
    );

    let document = build_document_v5(&analysis, &source).expect("v5 represents closures");
    assert_eq!(document.schema_version, 5);
    assert_eq!(document.closures.len(), 1);
    let closure = &document.closures[0];
    assert_eq!(closure.id, "closure:0");
    assert_eq!(closure.parameters.len(), 1);
    assert_eq!(closure.captures.len(), 1);
    assert_eq!(closure.captures[0].binding, "binding:0");
    assert_eq!(document.closure_control_flow.len(), 1);
    assert_eq!(document.closure_control_flow[0].closure, "closure:0");
    assert!(
        document.closure_control_flow[0]
            .bindings
            .contains(&"binding:0".to_owned())
    );
}

#[test]
fn v5_rejects_capture_type_or_free_reference_drift() {
    let (source, mut analysis) = analyzed(
        "fn main() -> Int { let base = 40; let add = fn(value: Int) -> Int { base + value }; add(2) }",
    );
    let StatementKind::Binding { initializer, .. } =
        &mut analysis.program.functions[0].body.statements[1].kind
    else {
        panic!("closure binding");
    };
    let ExpressionKind::Closure(closure) = &mut initializer.kind else {
        panic!("closure initializer");
    };
    closure.captures[0].ty = Type::Bool;
    let error = build_document_v5(&analysis, &source).expect_err("type drift must fail closed");
    assert!(error.message().contains("capture type"), "{error}");

    let (source, mut analysis) = analyzed(
        "fn main() -> Int { let base = 40; let add = fn(value: Int) -> Int { base + value }; add(2) }",
    );
    let StatementKind::Binding { initializer, .. } =
        &mut analysis.program.functions[0].body.statements[1].kind
    else {
        panic!("closure binding");
    };
    let ExpressionKind::Closure(closure) = &mut initializer.kind else {
        panic!("closure initializer");
    };
    closure.captures.clear();
    let error =
        build_document_v5(&analysis, &source).expect_err("missing capture must fail closed");
    assert!(error.message().contains("capture") || error.message().contains("ownership"));
}

#[test]
fn published_v5_schema_names_closure_categories() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../../../docs/schemas/semantic-inspection-v5.schema.json"
    ))
    .expect("published schema is JSON");
    assert_eq!(schema["$id"], "urn:nova:semantic-inspection:v5");
    assert_eq!(schema["properties"]["schema_version"]["const"], 5);
    assert!(schema["properties"]["closures"].is_object());
    assert!(schema["properties"]["closure_control_flow"].is_object());
}
