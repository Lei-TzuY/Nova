use nova_inspect::build_document;
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{
    analyze,
    hir::{ExpressionKind, FunctionType, Type},
};
use nova_source::{SourceFile, SourceId};

fn checked(text: &str) -> (SourceFile, nova_sema::hir::Program) {
    let source = SourceFile::new(SourceId::new(0), "function-reference-inspect.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    (source, analyzed.program)
}

fn callee_mut(program: &mut nova_sema::hir::Program) -> &mut nova_sema::hir::Expression {
    let tail = program.functions[2]
        .body
        .tail
        .as_deref_mut()
        .expect("main tail");
    let ExpressionKind::Call { callee, .. } = &mut tail.kind else {
        panic!("call HIR");
    };
    callee
}

#[test]
fn rejects_same_signature_function_name_id_drift() {
    let (source, mut program) =
        checked("fn first() -> Int { 1 } fn second() -> Int { 2 } fn main() -> Int { first() }");
    let second = program.functions[1].id;
    let callee = callee_mut(&mut program);
    let ExpressionKind::Function {
        function,
        function_name,
    } = &mut callee.kind
    else {
        panic!("function reference HIR");
    };
    assert_eq!(function_name, "first");
    *function = second;

    let error = build_document(&program, &source).expect_err("function identity drift must fail");
    assert!(error.message().contains("function reference `first`"));
}

#[test]
fn rejects_function_reference_signature_drift() {
    let (source, mut program) =
        checked("fn first() -> Int { 1 } fn second() -> Int { 2 } fn main() -> Int { first() }");
    let callee = callee_mut(&mut program);
    callee.ty = Type::Function(FunctionType {
        parameters: Vec::new(),
        return_type: Box::new(Type::Bool),
    });

    let error = build_document(&program, &source).expect_err("function type drift must fail");
    assert!(
        error
            .message()
            .contains("does not match declaration signature")
    );
}
