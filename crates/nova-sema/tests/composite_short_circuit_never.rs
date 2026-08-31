use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "composite-short-circuit-never.nv", text);
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

#[test]
fn closed_immutable_true_and_noncontinuing_rhs_is_noncontinuing() {
    let output = analyze_text("fn main() -> Int { { let flag = true; flag } && { return 42; } }");
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert!(output.program.functions[0].body.ty.is_never());
}

#[test]
fn closed_immutable_false_or_noncontinuing_rhs_is_noncontinuing() {
    let output = analyze_text("fn main() -> Int { { let flag = false; flag } || { return 42; } }");
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert!(output.program.functions[0].body.ty.is_never());
}

#[test]
fn selected_if_and_record_projection_proofs_propagate_required_rhs_never() {
    for text in [
        "fn main() -> Int { (if true { true } else { false }) && { return 42; } }",
        "record Flag { value: Bool } fn main() -> Int { (new Flag { value: false }).value || { return 42; } }",
    ] {
        let output = analyze_text(text);
        assert!(
            output.is_success(),
            "source: {text}; diagnostics: {:?}",
            output.diagnostics
        );
        assert!(
            output.program.functions[0].body.ty.is_never(),
            "source: {text}"
        );
    }
}

#[test]
fn selected_match_bool_proof_propagates_required_rhs_never() {
    let output = analyze_text(
        "enum Choice { A, B } fn main() -> Int { (match Choice::A { Choice::A => true, Choice::B => false }) && { return 42; } }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert!(output.program.functions[0].body.ty.is_never());
}

#[test]
fn composite_skipped_rhs_remains_continuing() {
    for text in [
        "fn main() -> Int { { let flag = false; flag } && { return 1; }; 42 }",
        "fn main() -> Int { { let flag = true; flag } || { return 1; }; 42 }",
    ] {
        let output = analyze_text(text);
        assert!(
            output.is_success(),
            "source: {text}; diagnostics: {:?}",
            output.diagnostics
        );
        assert!(
            !output.program.functions[0].body.ty.is_never(),
            "source: {text}"
        );
    }
}

#[test]
fn dynamic_short_circuit_left_remains_conservative() {
    for text in [
        "fn main(flag: Bool) -> Int { flag && { return 1; }; 42 }",
        "fn main(flag: Bool) -> Int { flag || { return 1; }; 42 }",
    ] {
        let output = analyze_text(text);
        assert!(
            output.is_success(),
            "source: {text}; diagnostics: {:?}",
            output.diagnostics
        );
        assert!(
            !output.program.functions[0].body.ty.is_never(),
            "source: {text}"
        );
    }
}
