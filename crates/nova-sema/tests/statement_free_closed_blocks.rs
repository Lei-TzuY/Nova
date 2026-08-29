use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{analyze, hir::ExpressionKind};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "statement-free-block.nv", text);
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

fn has_code(output: &nova_sema::AnalysisOutput, code: &str) -> bool {
    output
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == code)
}

#[test]
fn statement_free_bool_blocks_refine_if_and_while_flow() {
    for text in [
        "fn main() -> Int { var value: Int; if { { true } } { value = 42; () } else { () }; value }",
        "fn main() -> Int { var value: Int; while { { 3 > 2 } } { value = 42; break; } value }",
    ] {
        let output = analyze_text(text);
        assert!(
            output.is_success(),
            "source: {text}; diagnostics: {:?}",
            output.diagnostics
        );
    }
}

#[test]
fn statement_free_int_blocks_participate_in_closed_arithmetic() {
    let selected = analyze_text(
        "fn main() -> Int { var value: Int; if ({ 1 } + { { 1 } }) == { 2 } { value = 42; () } else { () }; value }",
    );
    assert!(selected.is_success(), "{:?}", selected.diagnostics);

    let overflow = analyze_text("fn main() -> Int { 9223372036854775807 + { 1 } }");
    assert!(has_code(&overflow, "N3031"), "{:?}", overflow.diagnostics);

    let zero_divisor = analyze_text("fn main() -> Int { 10 / { 3 - { 3 } } }");
    assert!(
        has_code(&zero_divisor, "N3032"),
        "{:?}",
        zero_divisor.diagnostics
    );
}

#[test]
fn statement_free_enum_blocks_preserve_direct_constructor_proofs() {
    let output = analyze_text(
        "enum Choice { A, B } fn main() -> Int { var value: Int; if { Choice::A } == { { Choice::A } } { value = 42; () } else { () }; value }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
}

#[test]
fn statement_bearing_blocks_remain_dynamic() {
    let condition = analyze_text(
        "fn main() -> Int { var value: Int; if { (); true } { value = 42; () } else { () }; value }",
    );
    assert!(has_code(&condition, "N3009"), "{:?}", condition.diagnostics);

    let arithmetic = analyze_text("fn main() -> Int { 9223372036854775807 + { (); 1 } }");
    assert!(
        arithmetic.is_success(),
        "statement-bearing block must remain runtime-checked: {:?}",
        arithmetic.diagnostics
    );
}

#[test]
fn transparent_blocks_are_not_folded_out_of_hir() {
    let output = analyze_text("fn main() -> Bool { { { true } } }");
    assert!(output.is_success(), "{:?}", output.diagnostics);
    let tail = output.program.functions[0]
        .body
        .tail
        .as_deref()
        .expect("main should retain its block tail");
    assert!(matches!(tail.kind, ExpressionKind::Block(_)));
}
