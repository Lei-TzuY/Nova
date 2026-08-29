use nova_interpreter::{Value, execute};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{
    analyze,
    hir::{ExpressionKind, StatementKind, Type},
};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "frame-slot.nv", text);
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
fn rejects_malformed_local_initializer_before_frame_storage() {
    let mut analyzed = analyze_text("fn main() -> Unit { let value: Int = 42; }");
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    let StatementKind::Binding { initializer, .. } = &mut main.body.statements[0].kind else {
        panic!("expected binding statement");
    };
    initializer.kind = ExpressionKind::Boolean(true);
    initializer.ty = Type::Bool;

    let error = execute(&analyzed.program).expect_err("initializer drift must fail at storage");
    assert_eq!(error.code, "N4005");
}

#[test]
fn rejects_malformed_delayed_assignment_before_frame_storage() {
    let mut analyzed = analyze_text("fn main() -> Unit { var value: Int; value = 42; }");
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    let StatementKind::Assignment { value, .. } = &mut main.body.statements[1].kind else {
        panic!("expected assignment statement");
    };
    value.kind = ExpressionKind::Boolean(true);
    value.ty = Type::Bool;

    let error = execute(&analyzed.program).expect_err("assignment drift must fail at storage");
    assert_eq!(error.code, "N4005");
}

#[test]
fn rejects_assignment_retargeted_to_immutable_runtime_slot() {
    let mut analyzed = analyze_text(
        "fn main() -> Unit { let fixed: Int = 1; var mutable: Int = 2; mutable = 3; }",
    );
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    let fixed = match &main.body.statements[0].kind {
        StatementKind::Binding { binding, .. } => binding.id,
        _ => panic!("expected immutable binding"),
    };
    let StatementKind::Assignment { target, .. } = &mut main.body.statements[2].kind else {
        panic!("expected assignment statement");
    };
    *target = Some(fixed);

    let error = execute(&analyzed.program).expect_err("immutable slot write must fail");
    assert_eq!(error.code, "N4005");
}

#[test]
fn rejects_binding_identity_reused_with_incompatible_slot_metadata() {
    let mut analyzed =
        analyze_text("fn main() -> Unit { let left: Int = 1; let right: Bool = true; }");
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    let left = match &main.body.statements[0].kind {
        StatementKind::Binding { binding, .. } => binding.id,
        _ => panic!("expected first binding"),
    };
    let StatementKind::Binding { binding, .. } = &mut main.body.statements[1].kind else {
        panic!("expected second binding");
    };
    binding.id = left;

    let error = execute(&analyzed.program).expect_err("binding metadata alias must fail");
    assert_eq!(error.code, "N4005");
}

#[test]
fn rejects_match_payload_binding_type_drift_before_arm_execution() {
    let mut analyzed = analyze_text(
        "enum Maybe { Some(Int), None } fn main() -> Unit { match Maybe::Some(42) { Maybe::Some(value) => (), Maybe::None => (), }; }",
    );
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    let StatementKind::Expression(expression) = &mut main.body.statements[0].kind else {
        panic!("expected match expression statement");
    };
    let ExpressionKind::Match { arms, .. } = &mut expression.kind else {
        panic!("expected match expression");
    };
    let binding = arms[0].binding.as_mut().expect("payload binding");
    binding.ty = Type::Bool;

    let error = execute(&analyzed.program).expect_err("payload binding type drift must fail");
    assert_eq!(error.code, "N4005");
}

#[test]
fn repeated_lexical_binding_execution_refreshes_the_same_runtime_slot() {
    let analyzed = analyze_text(
        "fn main() -> Int { var total: Int = 0; while total < 3 { let step: Int = 1; total = total + step; } total }",
    );
    let value = execute(&analyzed.program).expect("loop-local binding should re-enter cleanly");
    assert_eq!(value, Value::Int(3));
}

#[test]
fn valid_runtime_frame_storage_remains_executable() {
    let analyzed =
        analyze_text("fn main() -> Int { var value: Int; value = 41; value = value + 1; value }");
    let value = execute(&analyzed.program).expect("valid frame storage should execute");
    assert_eq!(value, Value::Int(42));
}
