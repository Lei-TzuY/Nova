use nova_inspect::build_document;
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{
    analyze,
    hir::{ExpressionKind, StatementKind},
};
use nova_source::{SourceFile, SourceId};

fn checked(text: &str) -> (SourceFile, nova_sema::hir::Program) {
    let source = SourceFile::new(SourceId::new(0), "binding-reference-inspect.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    (source, analyzed.program)
}

#[test]
fn rejects_same_name_shadow_reference_id_drift() {
    let (source, mut program) =
        checked("fn main() -> Int { let x: Int = 1; { let x: Int = 2; x } }");
    let main = &mut program.functions[0];
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

    let error = build_document(&program, &source).expect_err("binding drift must fail closed");
    assert!(error.message().contains("declaration span"));
}

#[test]
fn rejects_assignment_target_identity_drift() {
    let (source, mut program) =
        checked("fn main() -> Int { var left: Int = 1; var right: Int = 2; left = 3; left }");
    let main = &mut program.functions[0];
    let right = match &main.body.statements[1].kind {
        StatementKind::Binding { binding, .. } => binding.id,
        _ => panic!("right binding"),
    };
    let StatementKind::Assignment { target, .. } = &mut main.body.statements[2].kind else {
        panic!("assignment");
    };
    target.as_mut().expect("resolved target").binding = right;

    let error = build_document(&program, &source).expect_err("assignment drift must fail closed");
    assert!(error.message().contains("binding reference"));
}
