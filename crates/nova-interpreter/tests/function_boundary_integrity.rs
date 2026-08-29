use nova_interpreter::execute;
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{
    analyze,
    hir::{ExpressionKind, Type},
};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "function-boundary.nv", text);
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
fn rejects_malformed_runtime_return_type() {
    let mut analyzed = analyze_text("fn main() -> Int { 42 }");
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    let tail = main.body.tail.as_deref_mut().expect("main tail");
    tail.kind = ExpressionKind::Boolean(true);
    tail.ty = Type::Bool;

    let error = execute(&analyzed.program).expect_err("runtime return drift must fail");
    assert_eq!(error.code, "N4005");
}

#[test]
fn rejects_malformed_runtime_argument_type_before_calling_the_function() {
    let mut analyzed =
        analyze_text("fn take(value: Int) -> Int { 7 } fn main() -> Int { take(42) }");
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    let tail = main.body.tail.as_deref_mut().expect("main tail");
    let ExpressionKind::Call { arguments, .. } = &mut tail.kind else {
        panic!("expected call tail");
    };
    arguments[0].kind = ExpressionKind::Boolean(true);
    arguments[0].ty = Type::Bool;

    let error = execute(&analyzed.program).expect_err("runtime argument drift must fail");
    assert_eq!(error.code, "N4005");
}

#[test]
fn recursively_validates_nominal_record_slots_at_function_boundaries() {
    let mut analyzed = analyze_text(
        "record Box { value: Int } fn make() -> Box { new Box { value: 42 } } fn main() -> Unit { make(); }",
    );
    let make = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "make")
        .expect("make function");
    let tail = make.body.tail.as_deref_mut().expect("make tail");
    let ExpressionKind::RecordLiteral { fields, .. } = &mut tail.kind else {
        panic!("expected record literal tail");
    };
    fields[0].value.kind = ExpressionKind::Boolean(true);
    fields[0].value.ty = Type::Bool;

    let error = execute(&analyzed.program).expect_err("corrupt record slot must fail");
    assert_eq!(error.code, "N4005");
}

#[test]
fn rejects_nominal_return_identity_drift() {
    let mut analyzed = analyze_text(
        "record Left { value: Int } record Right { value: Int } fn make() -> Left { new Left { value: 42 } } fn main() -> Unit { make(); }",
    );
    let right = analyzed.program.records[1].id;
    let make = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "make")
        .expect("make function");
    let tail = make.body.tail.as_deref_mut().expect("make tail");
    let ExpressionKind::RecordLiteral { record, .. } = &mut tail.kind else {
        panic!("expected record literal tail");
    };
    *record = right;

    let error = execute(&analyzed.program).expect_err("nominal identity drift must fail");
    assert_eq!(error.code, "N4005");
}
