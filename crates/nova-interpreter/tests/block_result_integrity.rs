use nova_interpreter::{Value, execute};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{
    analyze,
    hir::{ExpressionKind, StatementKind, Type},
};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "block-result.nv", text);
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
fn rejects_function_body_block_type_drift() {
    let mut analyzed = analyze_text("fn main() -> Unit {}");
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    main.body.ty = Type::Bool;

    let error = execute(&analyzed.program).expect_err("function block drift must fail closed");
    assert_eq!(error.code, "N4005");
}

#[test]
fn rejects_selected_if_branch_block_type_drift() {
    let mut analyzed = analyze_text("fn main() -> Int { if true { 42 } else { 0 } }");
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    let tail = main.body.tail.as_deref_mut().expect("main tail");
    let ExpressionKind::If { then_branch, .. } = &mut tail.kind else {
        panic!("expected if expression");
    };
    then_branch.ty = Type::Bool;

    let error = execute(&analyzed.program).expect_err("selected branch drift must fail closed");
    assert_eq!(error.code, "N4005");
}

#[test]
fn rejects_discarded_while_body_block_type_drift() {
    let mut analyzed =
        analyze_text("fn main() -> Int { var again = true; while again { again = false; 42 } 0 }");
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    let StatementKind::While { body, .. } = &mut main.body.statements[1].kind else {
        panic!("expected while statement");
    };
    body.ty = Type::Bool;

    let error = execute(&analyzed.program).expect_err("discarded loop-body drift must fail closed");
    assert_eq!(error.code, "N4005");
}

#[test]
fn structured_return_bypasses_block_value_postcondition() {
    let mut analyzed = analyze_text("fn main() -> Int { return 42; }");
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    main.body.ty = Type::Bool;

    let value = execute(&analyzed.program).expect("structured return must remain executable");
    assert_eq!(value, Value::Int(42));
}

#[test]
fn structured_break_bypasses_block_value_postcondition() {
    let mut analyzed = analyze_text("fn main() -> Int { while true { break; } 42 }");
    let main = analyzed
        .program
        .functions
        .iter_mut()
        .find(|function| function.name == "main")
        .expect("main function");
    let StatementKind::While { body, .. } = &mut main.body.statements[0].kind else {
        panic!("expected while statement");
    };
    body.ty = Type::Bool;

    let value = execute(&analyzed.program).expect("structured break must remain executable");
    assert_eq!(value, Value::Int(42));
}
