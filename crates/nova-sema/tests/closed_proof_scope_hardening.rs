use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "closed-proof-scope-hardening.nv", text);
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

fn has_code(output: &AnalysisOutput, code: &str) -> bool {
    output
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == code)
}

#[test]
fn shadowing_initializer_reads_outer_binding_but_tail_reads_inner_binding() {
    let output = analyze_text(
        "fn main() -> Int { var answer: Int; if { let value = 40; { let value = value + 2; value } == 42 && value == 40 } { answer = 7; () } else { () }; answer }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
}

#[test]
fn cross_type_shadowing_uses_resolved_binding_identity() {
    let output = analyze_text(
        "fn main() -> Int { var answer: Int; if { let value = true; { let value = if value { 42 } else { 0 }; value } == 42 && value } { answer = 7; () } else { () }; answer }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
}

#[test]
fn nested_block_environment_does_not_leak_into_following_statement() {
    let output = analyze_text(
        "fn main() -> Int { var answer: Int; if { let value = 1; { let value = 2; value }; value == 1 } { answer = 7; () } else { () }; answer }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
}

#[test]
fn selected_if_environment_does_not_leak_into_following_statement() {
    let output = analyze_text(
        "fn main() -> Int { var answer: Int; if { let value = 1; if true { let value = 2; value == 2 } else { false }; value == 1 } { answer = 7; () } else { () }; answer }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
}

#[test]
fn selected_match_payload_environment_does_not_leak_after_match_statement() {
    let output = analyze_text(
        "enum Box { Value(Int) } fn main() -> Int { var answer: Int; if { let value = 1; match Box::Value(2) { Box::Value(value) => value == 2 }; value == 1 } { answer = 7; () } else { () }; answer }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
}

#[test]
fn nested_same_name_match_payloads_resolve_to_innermost_binding() {
    let output = analyze_text(
        "enum Inner { Value(Int) } enum Outer { Value(Inner) } fn main() -> Int { var answer: Int; if match Outer::Value(Inner::Value(42)) { Outer::Value(value) => match value { Inner::Value(value) => value == 42 } } { answer = 7; () } else { () }; answer }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
}

#[test]
fn shadowed_closed_divisor_still_reaches_static_zero_division_preflight() {
    let output = analyze_text(
        "fn main() -> Int { 10 / { let divisor = 1; { let divisor = divisor - 1; divisor } } }",
    );
    assert!(has_code(&output, "N3032"), "{:?}", output.diagnostics);
}

#[test]
fn shadowed_closed_operand_still_reaches_static_overflow_preflight() {
    let output = analyze_text(
        "fn main() -> Int { 9223372036854775807 + { let value = 0; { let value = value + 1; value } } }",
    );
    assert!(has_code(&output, "N3031"), "{:?}", output.diagnostics);
}

#[test]
fn mutable_shadow_keeps_the_condition_runtime_only_without_poisoning_outer_identity() {
    let output = analyze_text(
        "fn main() -> Int { var answer: Int; if { let value = 1; { var value = 1; value } == 1 && value == 1 } { answer = 7; () } else { () }; answer }",
    );
    assert!(has_code(&output, "N3009"), "{:?}", output.diagnostics);
}
