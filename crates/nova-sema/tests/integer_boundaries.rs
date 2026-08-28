use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::hir::{ExpressionKind, Type};
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "integer-boundaries.nv", text);
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
    analyze(&parsed.program)
}

fn function<'a>(output: &'a AnalysisOutput, name: &str) -> &'a nova_sema::hir::Function {
    output
        .program
        .functions
        .iter()
        .find(|function| function.name == name)
        .unwrap_or_else(|| panic!("missing function {name}"))
}

#[test]
fn accepts_both_signed_int_endpoints() {
    for (source, expected) in [
        ("fn main() -> Int { 9223372036854775807 }", i64::MAX),
        ("fn main() -> Int { -9223372036854775808 }", i64::MIN),
        ("fn main() -> Int { -(9223372036854775808) }", i64::MIN),
    ] {
        let output = analyze_text(source);
        assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
        let main = function(&output, "main");
        assert_eq!(main.body.ty, Type::Int);
        let tail = main.body.tail.as_deref().expect("main should have a tail");
        assert_eq!(tail.ty, Type::Int);
        assert_eq!(tail.kind, ExpressionKind::Integer(expected));
    }
}

#[test]
fn rejects_positive_two_to_the_sixty_third_semantically() {
    let output = analyze_text("fn main() -> Int { 9223372036854775808 }");
    assert_eq!(output.diagnostics.len(), 1, "{:?}", output.diagnostics);
    assert_eq!(output.diagnostics[0].code, "N3030");
    let tail = function(&output, "main")
        .body
        .tail
        .as_deref()
        .expect("main should retain recovery HIR");
    assert_eq!(tail.ty, Type::Error);
    assert!(matches!(tail.kind, ExpressionKind::Error));
}

#[test]
fn double_negation_of_min_is_preflighted_without_folding_hir() {
    let output = analyze_text("fn main() -> Int { --9223372036854775808 }");
    assert_eq!(output.diagnostics.len(), 1, "{:?}", output.diagnostics);
    assert_eq!(output.diagnostics[0].code, "N3031");
    let tail = function(&output, "main")
        .body
        .tail
        .as_deref()
        .expect("main should retain recovery HIR");
    assert_eq!(tail.ty, Type::Error);
    let ExpressionKind::Unary { operand, .. } = &tail.kind else {
        panic!("preflight must not fold the outer negation");
    };
    assert_eq!(operand.kind, ExpressionKind::Integer(i64::MIN));
}
