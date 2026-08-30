use nova_interpreter::{Value, execute};
use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{
    analyze,
    hir::{ExpressionKind, StatementKind},
};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "enum-variant-identity.nv", text);
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
fn rejects_same_payload_type_constructor_slot_retargeting() {
    let mut analyzed = analyze_text(
        "enum Choice { Left(Int), Right(Int), } fn main() -> Int { match Choice::Left(7) { Choice::Left(value) => value, Choice::Right(value) => 0, } }",
    );
    let tail = analyzed.program.functions[0]
        .body
        .tail
        .as_deref_mut()
        .expect("match tail");
    let ExpressionKind::Match { scrutinee, .. } = &mut tail.kind else {
        panic!("match HIR");
    };
    let ExpressionKind::EnumConstructor {
        variant_name,
        variant_index,
        ..
    } = &mut scrutinee.kind
    else {
        panic!("constructor HIR");
    };
    assert_eq!(variant_name, "Left");
    *variant_index = 1;

    let error = execute(&analyzed.program).expect_err("retargeted constructor must fail closed");
    assert_eq!(error.code, "N4005");
}

#[test]
fn rejects_exhaustive_same_shape_match_arm_swap() {
    let mut analyzed = analyze_text(
        "enum Flag { Off, On, } fn main() -> Int { match Flag::Off { Flag::Off => 1, Flag::On => 2, } }",
    );
    let tail = analyzed.program.functions[0]
        .body
        .tail
        .as_deref_mut()
        .expect("match tail");
    let ExpressionKind::Match { arms, .. } = &mut tail.kind else {
        panic!("match HIR");
    };
    arms[0].variant_index = 1;
    arms[1].variant_index = 0;

    let error = execute(&analyzed.program).expect_err("retargeted patterns must fail closed");
    assert_eq!(error.code, "N4005");
}

#[test]
fn constructor_payload_return_precedes_malformed_variant_identity() {
    let mut analyzed = analyze_text(
        "enum Choice { Left(Int), Right(Int), } fn main() -> Int { Choice::Left({ return 9; 0 }); 0 }",
    );
    let StatementKind::Expression(expression) =
        &mut analyzed.program.functions[0].body.statements[0].kind
    else {
        panic!("constructor statement");
    };
    let ExpressionKind::EnumConstructor { variant_index, .. } = &mut expression.kind else {
        panic!("constructor HIR");
    };
    *variant_index = 99;

    let value = execute(&analyzed.program)
        .expect("structured return must win before value-only identity validation");
    assert_eq!(value, Value::Int(9));
}

#[test]
fn match_scrutinee_return_precedes_malformed_arm_identity() {
    let mut analyzed = analyze_text(
        "enum Flag { Off, On, } fn main() -> Int { match { return 8; Flag::Off } { Flag::Off => 1, Flag::On => 2, } }",
    );
    let tail = analyzed.program.functions[0]
        .body
        .tail
        .as_deref_mut()
        .expect("match tail");
    let ExpressionKind::Match { arms, .. } = &mut tail.kind else {
        panic!("match HIR");
    };
    arms[0].variant_index = 99;

    let value = execute(&analyzed.program)
        .expect("scrutinee return must win before value-only arm validation");
    assert_eq!(value, Value::Int(8));
}
