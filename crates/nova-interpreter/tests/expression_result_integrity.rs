use nova_interpreter::{Value, execute};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{
    analyze,
    hir::{ExpressionKind, StatementKind, Type},
};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "expression-result.nv", text);
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
fn rejects_discarded_primitive_value_with_drifted_hir_type() {
    let mut analyzed = analyze_text("fn main() -> Unit { 42; }");
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    let StatementKind::Expression(expression) = &mut main.body.statements[0].kind else {
        panic!("expected expression statement");
    };
    assert!(matches!(expression.kind, ExpressionKind::Integer(42)));
    expression.ty = Type::Bool;

    let error = execute(&analyzed.program).expect_err("discarded expression drift must fail");
    assert_eq!(error.code, "N4005");
}

#[test]
fn rejects_composed_field_result_with_drifted_hir_type() {
    let mut analyzed = analyze_text(
        "record Box { value: Int } fn main() -> Unit { let boxed = new Box { value: 42 }; boxed.value; }",
    );
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    let StatementKind::Expression(expression) = &mut main.body.statements[1].kind else {
        panic!("expected field expression statement");
    };
    assert!(matches!(
        expression.kind,
        ExpressionKind::FieldAccess { .. }
    ));
    expression.ty = Type::Bool;

    let error = execute(&analyzed.program).expect_err("field result drift must fail");
    assert_eq!(error.code, "N4005");
}

#[test]
fn structured_return_bypasses_value_postcondition() {
    let analyzed = analyze_text(
        "fn choose(flag: Bool) -> Int { if flag { return 42; } else { 0 } } fn main() -> Int { choose(true) }",
    );
    let value = execute(&analyzed.program).expect("return flow should remain executable");
    assert_eq!(value, Value::Int(42));
}

#[test]
fn structured_loop_transfers_bypass_value_postcondition() {
    let analyzed = analyze_text(
        "fn main() -> Int { var value: Int = 0; while value < 3 { value = value + 1; if value < 3 { continue; } else { break; }; } value }",
    );
    let value = execute(&analyzed.program).expect("loop transfers should remain executable");
    assert_eq!(value, Value::Int(3));
}
