use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{analyze, hir::ExpressionKind};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> nova_sema::AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "test.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "lex diagnostics: {:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "parse diagnostics: {:?}", parsed.diagnostics);
    analyze(&parsed.program)
}

fn codes(text: &str) -> Vec<String> {
    analyze_text(text)
        .diagnostics
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect()
}

#[test]
fn rejects_closed_literal_arithmetic_overflow_during_semantic_analysis() {
    for text in [
        "fn main() -> Int { 9223372036854775807 + 1 }",
        "fn main() -> Int { -9223372036854775808 - 1 }",
        "fn main() -> Int { 9223372036854775807 * 2 }",
        "fn main() -> Int { --9223372036854775808 }",
        "fn main() -> Int { -9223372036854775808 / -1 }",
        "fn main() -> Int { -9223372036854775808 % -1 }",
    ] {
        let actual = codes(text);
        assert!(actual.contains(&"N3031".to_owned()), "source: {text}; codes: {actual:?}");
    }
}

#[test]
fn rejects_closed_literal_zero_divisors_during_semantic_analysis() {
    for text in [
        "fn main() -> Int { 10 / (3 - 3) }",
        "fn main() -> Int { 10 % (2 - 2) }",
    ] {
        let actual = codes(text);
        assert!(actual.contains(&"N3032".to_owned()), "source: {text}; codes: {actual:?}");
    }
}

#[test]
fn invalid_constant_arithmetic_is_error_typed() {
    let output = analyze_text("fn main() -> Int { 9223372036854775807 + 1 }");
    let tail = output.program.functions[0]
        .body
        .tail
        .as_deref()
        .expect("main has a tail expression");
    assert!(tail.ty.is_error());
}

#[test]
fn dynamic_equivalents_remain_runtime_checked() {
    for text in [
        "fn one() -> Int { 1 } fn main() -> Int { 9223372036854775807 + one() }",
        "fn zero() -> Int { 0 } fn main() -> Int { 10 / zero() }",
        "fn min() -> Int { -9223372036854775808 } fn minus_one() -> Int { -1 } fn main() -> Int { min() % minus_one() }",
    ] {
        let output = analyze_text(text);
        assert!(output.is_success(), "source: {text}; diagnostics: {:?}", output.diagnostics);
    }
}

#[test]
fn successful_constant_arithmetic_is_validated_but_not_folded() {
    let output = analyze_text("fn main() -> Int { (20 + 22) * 1 }");
    assert!(output.is_success(), "{:?}", output.diagnostics);
    let tail = output.program.functions[0]
        .body
        .tail
        .as_deref()
        .expect("main has a tail expression");
    assert!(matches!(tail.kind, ExpressionKind::Binary { .. }));
}
