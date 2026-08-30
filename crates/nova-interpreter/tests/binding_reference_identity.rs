use nova_interpreter::{Value, execute};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{
    analyze,
    hir::{ExpressionKind, StatementKind},
};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "binding-reference-runtime.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    analyzed
}

#[test]
fn rejects_same_name_same_type_shadow_read_retargeting() {
    let mut analyzed = analyze_text("fn main() -> Int { let x: Int = 1; { let x: Int = 2; x } }");
    let main = &mut analyzed.program.functions[0];
    let outer = match &main.body.statements[0].kind {
        StatementKind::Binding { binding, .. } => binding.id,
        _ => panic!("outer binding"),
    };
    let tail = main.body.tail.as_deref_mut().expect("block tail");
    let ExpressionKind::Block(block) = &mut tail.kind else {
        panic!("inner block");
    };
    let read = block.tail.as_deref_mut().expect("inner read");
    let ExpressionKind::Binding(reference) = &mut read.kind else {
        panic!("binding read");
    };
    reference.binding = outer;

    let error = execute(&analyzed.program).expect_err("shadow retargeting must fail closed");
    assert_eq!(error.code, "N4005");
}

#[test]
fn rejects_same_type_assignment_target_retargeting() {
    let mut analyzed =
        analyze_text("fn main() -> Int { var left: Int = 1; var right: Int = 2; left = 3; left }");
    let main = &mut analyzed.program.functions[0];
    let right = match &main.body.statements[1].kind {
        StatementKind::Binding { binding, .. } => binding.id,
        _ => panic!("right binding"),
    };
    let StatementKind::Assignment { target, .. } = &mut main.body.statements[2].kind else {
        panic!("assignment");
    };
    target.as_mut().expect("resolved target").binding = right;

    let error = execute(&analyzed.program).expect_err("assignment retargeting must fail closed");
    assert_eq!(error.code, "N4005");
}

#[test]
fn assignment_rhs_return_precedes_value_only_target_validation() {
    let mut analyzed = analyze_text(
        "fn main() -> Int { var left: Int = 1; var right: Int = 2; left = { return 9; }; left }",
    );
    let main = &mut analyzed.program.functions[0];
    let right = match &main.body.statements[1].kind {
        StatementKind::Binding { binding, .. } => binding.id,
        _ => panic!("right binding"),
    };
    let StatementKind::Assignment { target, .. } = &mut main.body.statements[2].kind else {
        panic!("assignment");
    };
    target.as_mut().expect("resolved target").binding = right;

    let value =
        execute(&analyzed.program).expect("return must bypass unused assignment target validation");
    assert_eq!(value, Value::Int(9));
}
