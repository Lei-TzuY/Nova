use nova_interpreter::{Value, execute};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{
    analyze,
    hir::{ExpressionKind, Type},
};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "aggregate-boundary.nv", text);
    let lexed = lex(&source);
    assert!(
        lexed.is_success(),
        "lex diagnostics: {:?}",
        lexed.diagnostics
    );
    let parsed = parse(&source, &lexed.tokens);
    assert!(
        parsed.is_success(),
        "parse diagnostics: {:?}",
        parsed.diagnostics
    );
    let analyzed = analyze(&parsed.program);
    assert!(
        analyzed.is_success(),
        "semantic diagnostics: {:?}",
        analyzed.diagnostics
    );
    analyzed
}

#[test]
fn rejects_malformed_record_field_even_when_value_is_discarded_locally() {
    let mut analyzed =
        analyze_text("record Box { value: Int } fn main() -> Unit { new Box { value: 42 }; }");
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .unwrap();
    let statement = &mut main.body.statements[0];
    let nova_sema::hir::StatementKind::Expression(expression) = &mut statement.kind else {
        panic!("expected expression statement");
    };
    let ExpressionKind::RecordLiteral { fields, .. } = &mut expression.kind else {
        panic!("expected record literal");
    };
    fields[0].value.kind = ExpressionKind::Boolean(true);
    fields[0].value.ty = Type::Bool;

    let error =
        execute(&analyzed.program).expect_err("record field drift must fail at construction");
    assert_eq!(error.code, "N4005");
}

#[test]
fn rejects_malformed_enum_payload_even_when_value_is_discarded_locally() {
    let mut analyzed =
        analyze_text("enum Maybe { Some(Int), None } fn main() -> Unit { Maybe::Some(42); }");
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .unwrap();
    let statement = &mut main.body.statements[0];
    let nova_sema::hir::StatementKind::Expression(expression) = &mut statement.kind else {
        panic!("expected expression statement");
    };
    let ExpressionKind::EnumConstructor { payload, .. } = &mut expression.kind else {
        panic!("expected enum constructor");
    };
    let payload = payload.as_deref_mut().expect("payload");
    payload.kind = ExpressionKind::Boolean(true);
    payload.ty = Type::Bool;

    let error =
        execute(&analyzed.program).expect_err("enum payload drift must fail at construction");
    assert_eq!(error.code, "N4005");
}

#[test]
fn recursively_rejects_nested_nominal_record_identity_drift_at_construction() {
    let mut analyzed = analyze_text(
        "record Left { value: Int } record Right { value: Int } record Outer { inner: Left } fn main() -> Unit { new Outer { inner: new Left { value: 42 } }; }",
    );
    let right = analyzed.program.records[1].id;
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .unwrap();
    let statement = &mut main.body.statements[0];
    let nova_sema::hir::StatementKind::Expression(expression) = &mut statement.kind else {
        panic!("expected expression statement");
    };
    let ExpressionKind::RecordLiteral { fields, .. } = &mut expression.kind else {
        panic!("expected outer record literal");
    };
    let ExpressionKind::RecordLiteral { record, .. } = &mut fields[0].value.kind else {
        panic!("expected nested record literal");
    };
    *record = right;

    let error =
        execute(&analyzed.program).expect_err("nested nominal drift must fail at construction");
    assert_eq!(error.code, "N4005");
}

#[test]
fn valid_local_aggregates_remain_executable() {
    let analyzed = analyze_text(
        "record Box { value: Int } enum Maybe { Some(Int), None } fn main() -> Unit { new Box { value: 42 }; Maybe::Some(7); }",
    );
    let value = execute(&analyzed.program).expect("valid aggregate construction should execute");
    assert_eq!(value, Value::Unit);
}
