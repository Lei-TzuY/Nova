use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{
    analyze,
    hir::{ExpressionKind, StatementKind},
};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "binding-reference.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    analyzed
}

#[test]
fn binding_read_retains_resolved_declaration_identity_under_shadowing() {
    let analyzed = analyze_text("fn main() -> Int { let x: Int = 1; { let x: Int = 2; x } }");
    let main = &analyzed.program.functions[0];
    let StatementKind::Binding { binding: outer, .. } = &main.body.statements[0].kind else {
        panic!("outer binding");
    };
    let block = main.body.tail.as_deref().expect("block tail");
    let ExpressionKind::Block(block) = &block.kind else {
        panic!("inner block");
    };
    let StatementKind::Binding { binding: inner, .. } = &block.statements[0].kind else {
        panic!("inner binding");
    };
    let reference = block.tail.as_deref().expect("inner read");
    let ExpressionKind::Binding(reference) = &reference.kind else {
        panic!("binding read");
    };
    assert_eq!(reference.binding, inner.id);
    assert_eq!(reference.binding_name, "x");
    assert_eq!(reference.declaration_span, inner.span);
    assert_ne!(reference.declaration_span, outer.span);
}

#[test]
fn assignment_retains_resolved_target_identity() {
    let analyzed =
        analyze_text("fn main() -> Int { var left: Int = 1; var right: Int = 2; left = 3; left }");
    let main = &analyzed.program.functions[0];
    let StatementKind::Binding { binding: left, .. } = &main.body.statements[0].kind else {
        panic!("left binding");
    };
    let StatementKind::Assignment { target, .. } = &main.body.statements[2].kind else {
        panic!("assignment");
    };
    let target = target.as_ref().expect("resolved target");
    assert_eq!(target.binding, left.id);
    assert_eq!(target.binding_name, left.name);
    assert_eq!(target.declaration_span, left.span);
}
