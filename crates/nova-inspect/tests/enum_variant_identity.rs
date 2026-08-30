use nova_inspect::build_document;
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{analyze, hir::ExpressionKind};
use nova_source::{SourceFile, SourceId};

fn checked(text: &str) -> (SourceFile, nova_sema::hir::Program) {
    let source = SourceFile::new(SourceId::new(0), "enum-variant-inspect.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
    let analyzed = analyze(&parsed.program);
    assert!(analyzed.is_success(), "{:?}", analyzed.diagnostics);
    (source, analyzed.program)
}

#[test]
fn rejects_constructor_variant_name_slot_drift() {
    let (source, mut program) = checked(
        "enum Choice { Left(Int), Right(Int), } fn main() -> Int { match Choice::Left(7) { Choice::Left(value) => value, Choice::Right(value) => 0, } }",
    );
    let tail = program.functions[0]
        .body
        .tail
        .as_deref_mut()
        .expect("match tail");
    let ExpressionKind::Match { scrutinee, .. } = &mut tail.kind else {
        panic!("match HIR");
    };
    let ExpressionKind::EnumConstructor { variant_index, .. } = &mut scrutinee.kind else {
        panic!("constructor HIR");
    };
    *variant_index = 1;

    let error = build_document(&program, &source).expect_err("variant drift must fail closed");
    assert!(error.message().contains("enum construction variant"));
}

#[test]
fn rejects_match_arm_variant_name_slot_drift() {
    let (source, mut program) = checked(
        "enum Flag { Off, On, } fn main() -> Int { match Flag::Off { Flag::Off => 1, Flag::On => 2, } }",
    );
    let tail = program.functions[0]
        .body
        .tail
        .as_deref_mut()
        .expect("match tail");
    let ExpressionKind::Match { arms, .. } = &mut tail.kind else {
        panic!("match HIR");
    };
    arms[0].variant_index = 1;
    arms[1].variant_index = 0;

    let error = build_document(&program, &source).expect_err("pattern drift must fail closed");
    assert!(error.message().contains("match variant"));
}
