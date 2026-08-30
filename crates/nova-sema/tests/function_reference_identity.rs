use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{analyze, hir::ExpressionKind};
use nova_source::{SourceFile, SourceId};

#[test]
fn direct_function_reference_retains_resolved_name_and_identity() {
    let source = SourceFile::new(
        SourceId::new(0),
        "function-reference-identity.nv",
        "fn first() -> Int { 1 } fn second() -> Int { 2 } fn main() -> Int { first() }",
    );
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);

    let tail = analyzed.program.functions[2]
        .body
        .tail
        .as_deref()
        .expect("main tail");
    let ExpressionKind::Call { callee, .. } = &tail.kind else {
        panic!("expected call HIR");
    };
    let ExpressionKind::Function {
        function,
        function_name,
    } = &callee.kind
    else {
        panic!("expected direct function reference");
    };
    assert_eq!(*function, analyzed.program.functions[0].id);
    assert_eq!(function_name, "first");
}
