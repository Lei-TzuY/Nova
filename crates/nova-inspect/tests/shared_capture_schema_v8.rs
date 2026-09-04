use nova_inspect::{build_document_v7, build_document_v8, render_json_v8, v8};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::analyze;
use nova_sema::hir::CaptureMode as HirCaptureMode;
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
    assert_eq!(
        document.closures[0].captures[0].mode,
        v8::CaptureMode::ByReference
    );
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
    assert_eq!(
        document.closures[0].captures[0].mode,
        v8::CaptureMode::ByValue
    );
}

#[test]
fn v8_rejects_malformed_by_reference_capture_of_immutable_binding() {
    let (source, mut analyzed) =
        analysis("fn main() -> Int { let value = 1; let get = fn() -> Int { value }; get() }");
    let closure = match &mut analyzed.program.functions[0].body.statements[1].kind {
        nova_sema::hir::StatementKind::Binding { initializer, .. } => match &mut initializer.kind {
            nova_sema::hir::ExpressionKind::Closure(closure) => closure,
            _ => panic!("closure initializer"),
        },
        _ => panic!("closure binding"),
    };
    closure.captures[0].mode = HirCaptureMode::ByReference;
    let error = build_document_v8(&analyzed, &source).expect_err("malformed HIR must fail closed");
    assert!(
        error.message().contains("immutable") || error.message().contains("by-reference"),
        "{error}"
    );
}
