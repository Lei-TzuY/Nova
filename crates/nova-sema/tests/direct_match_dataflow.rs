use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::hir::{StatementKind, Type};
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "direct-match.nv", text);
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

fn codes(output: &AnalysisOutput) -> Vec<&str> {
    output
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect()
}

const CHOICE: &str = "enum Choice { A, B }";

#[test]
fn direct_match_selects_definite_initialization_from_the_live_arm() {
    let output = analyze_text(&format!(
        "{CHOICE} fn f() -> Int {{ var value: Int; match Choice::A {{ Choice::A => {{ value = 1; 0 }}, Choice::B => 0, }}; value }}"
    ));
    assert!(output.is_success(), "{:?}", output.diagnostics);

    let output = analyze_text(&format!(
        "{CHOICE} fn f() -> Int {{ var value: Int; match Choice::A {{ Choice::A => 0, Choice::B => {{ value = 1; 0 }}, }}; value }}"
    ));
    assert_eq!(codes(&output), vec!["N3009"]);
}

#[test]
fn direct_match_payload_variant_selects_the_live_arm() {
    let output = analyze_text(
        "enum Result { Empty, Value(Int) } fn f() -> Int { var value: Int; match Result::Value(41) { Result::Empty => 0, Result::Value(payload) => { value = payload + 1; 0 }, }; value }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
}

#[test]
fn direct_match_dead_breaks_do_not_create_guaranteed_loop_exits() {
    let output = analyze_text(&format!(
        "{CHOICE} fn f() -> Int {{ while true {{ match Choice::A {{ Choice::A => {{ continue; }}, Choice::B => {{ break; }}, }}; }} }}"
    ));
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert!(output.program.functions[0].body.ty.is_never());
}

#[test]
fn direct_match_live_breaks_remain_guaranteed_loop_exits() {
    let output = analyze_text(&format!(
        "{CHOICE} fn f() -> Int {{ while true {{ match Choice::A {{ Choice::A => {{ break; }}, Choice::B => {{ continue; }}, }}; }} }}"
    ));
    assert_eq!(codes(&output), vec!["N3007"]);
}

#[test]
fn direct_match_uses_the_live_arm_for_noncontinuation() {
    let output = analyze_text(&format!(
        "{CHOICE} fn f() -> Int {{ match Choice::A {{ Choice::A => {{ return 1; }}, Choice::B => 2, }}; 3 }}"
    ));
    assert!(output.is_success(), "{:?}", output.diagnostics);
    let StatementKind::Expression(expression) =
        &output.program.functions[0].body.statements[0].kind
    else {
        panic!("expected match expression statement");
    };
    assert_eq!(expression.ty, Type::Never);
}

#[test]
fn direct_match_dead_arms_remain_statically_checked() {
    let output = analyze_text(&format!(
        "{CHOICE} fn f() -> Int {{ match Choice::A {{ Choice::A => 1, Choice::B => missing, }} }}"
    ));
    assert_eq!(codes(&output), vec!["N3003"]);

    let output = analyze_text(&format!(
        "{CHOICE} fn f() -> Int {{ match Choice::A {{ Choice::A => 1, Choice::B => false, }} }}"
    ));
    assert_eq!(codes(&output), vec!["N3004"]);
}

#[test]
fn dynamic_match_keeps_all_arms_in_the_flow_merge() {
    let output = analyze_text(&format!(
        "{CHOICE} fn f(choice: Choice) -> Int {{ var value: Int; match choice {{ Choice::A => {{ value = 1; 0 }}, Choice::B => 0, }}; value }}"
    ));
    assert_eq!(codes(&output), vec!["N3009"]);
}
