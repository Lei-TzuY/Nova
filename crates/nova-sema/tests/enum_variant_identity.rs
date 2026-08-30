use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{analyze, hir::ExpressionKind};
use nova_source::{SourceFile, SourceId};

#[test]
fn enum_hir_retains_resolved_variant_spelling_and_slot() {
    let source = SourceFile::new(
        SourceId::new(0),
        "enum-variant-identity.nv",
        "enum Choice { Left(Int), Right(Int), } fn main() -> Int { match Choice::Right(7) { Choice::Left(value) => 0, Choice::Right(value) => value, } }",
    );
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);

    let tail = analyzed.program.functions[0]
        .body
        .tail
        .as_deref()
        .expect("match tail");
    let ExpressionKind::Match {
        scrutinee, arms, ..
    } = &tail.kind
    else {
        panic!("expected match HIR");
    };
    let ExpressionKind::EnumConstructor {
        variant_name,
        variant_index,
        ..
    } = &scrutinee.kind
    else {
        panic!("expected enum constructor scrutinee");
    };
    assert_eq!(variant_name, "Right");
    assert_eq!(*variant_index, 1);
    assert_eq!(arms.len(), 2);
    assert_eq!(arms[0].variant_name, "Left");
    assert_eq!(arms[0].variant_index, 0);
    assert_eq!(arms[1].variant_name, "Right");
    assert_eq!(arms[1].variant_index, 1);
}
