use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "noncontinuing-successors.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "lex diagnostics: {:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "parse diagnostics: {:?}", parsed.diagnostics);
    analyze(&parsed.program)
}

#[test]
fn noncontinuing_if_condition_suppresses_execution_failures_in_both_branches() {
    let output = analyze_text(
        "fn main() -> Int { if { return 7; false } { 1 / 0 } else { 2 / 0 } }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
}

#[test]
fn noncontinuing_while_condition_makes_body_diagnostic_only_but_keeps_loop_control_legal() {
    let output = analyze_text(
        "fn main() -> Int { while { return 7; false } { 1 / 0; break; } }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
}

#[test]
fn noncontinuing_match_scrutinee_suppresses_execution_failures_in_arms() {
    let output = analyze_text(
        "enum Flag { A, B } fn main() -> Int { match { return 7; Flag::A } { Flag::A => 1 / 0, Flag::B => 2 / 0, } }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
}

#[test]
fn unreachable_successors_still_receive_static_diagnostics() {
    for text in [
        "fn main() -> Int { if { return 7; false } { missing } else { 0 } }",
        "fn main() -> Int { while { return 7; false } { missing; } }",
        "enum Flag { A, B } fn main() -> Int { match { return 7; Flag::A } { Flag::A => missing, Flag::B => 0, } }",
    ] {
        let output = analyze_text(text);
        assert!(
            output.diagnostics.iter().any(|diagnostic| diagnostic.code == "N3003"),
            "source: {text}; diagnostics: {:?}",
            output.diagnostics
        );
    }
}

#[test]
fn ordinary_reachable_successors_keep_constant_execution_failures() {
    for text in [
        "fn main() -> Int { if true { 1 / 0 } else { 0 } }",
        "fn main() -> Int { while true { 1 / 0; break; } 0 }",
        "enum Flag { A, B } fn main() -> Int { match Flag::A { Flag::A => 1 / 0, Flag::B => 0, } }",
    ] {
        let output = analyze_text(text);
        assert!(
            output.diagnostics.iter().any(|diagnostic| diagnostic.code == "N3032"),
            "source: {text}; diagnostics: {:?}",
            output.diagnostics
        );
    }
}
