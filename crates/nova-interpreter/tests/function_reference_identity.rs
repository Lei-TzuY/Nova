use nova_interpreter::{Value, execute};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{analyze, hir::ExpressionKind};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "function-reference-identity.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    analyzed
}

#[test]
fn rejects_same_signature_direct_function_retargeting() {
    let mut analyzed = analyze_text(
        "fn first() -> Int { 1 } fn second() -> Int { 2 } fn main() -> Int { first() }",
    );
    let second = analyzed.program.functions[1].id;
    let tail = analyzed.program.functions[2]
        .body
        .tail
        .as_deref_mut()
        .expect("main tail");
    let ExpressionKind::Call { callee, .. } = &mut tail.kind else {
        panic!("call HIR");
    };
    let ExpressionKind::Function {
        function,
        function_name,
    } = &mut callee.kind
    else {
        panic!("function reference HIR");
    };
    assert_eq!(function_name, "first");
    *function = second;

    let error = execute(&analyzed.program).expect_err("retargeted function reference must fail");
    assert_eq!(error.code, "N4005");
}

#[test]
fn validated_function_alias_keeps_runtime_declaration_identity() {
    let analyzed =
        analyze_text("fn first() -> Int { 7 } fn main() -> Int { let alias = first; alias() }");
    let value = execute(&analyzed.program).expect("validated alias should execute");
    assert_eq!(value, Value::Int(7));
}
