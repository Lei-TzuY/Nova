use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(
        SourceId::new(0),
        "static-match-composite-usefulness.nv",
        text,
    );
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

#[test]
fn selected_if_enum_scrutinee_drives_match_dataflow_and_usefulness() {
    let output = analyze_text(
        "enum Choice { A, B } fn main() -> Int { var value: Int; match if true { Choice::A } else { Choice::B } { Choice::A => { value = 42; 0 }, Choice::B => 0, }; value }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(codes(&output), vec!["N3034"]);
}

#[test]
fn closed_record_projection_scrutinee_drives_match_selection() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } fn main() -> Int { var value: Int; match (new Holder { choice: Choice::B }).choice { Choice::A => 0, Choice::B => { value = 42; 0 }, }; value }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(codes(&output), vec!["N3034"]);
}

#[test]
fn closed_immutable_binding_block_scrutinee_drives_match_selection() {
    let output = analyze_text(
        "enum Choice { A, B } fn main() -> Int { var value: Int; match { let choice = Choice::A; choice } { Choice::A => { value = 42; 0 }, Choice::B => 0, }; value }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(codes(&output), vec!["N3034"]);
}

#[test]
fn selected_if_payload_constructor_keeps_payload_binding_flow() {
    let output = analyze_text(
        "enum Maybe { None, Some(Int) } fn main() -> Int { var value: Int; match if true { Maybe::Some(42) } else { Maybe::None } { Maybe::None => 0, Maybe::Some(inner) => { value = inner; 0 }, }; value }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(codes(&output), vec!["N3034"]);
}

#[test]
fn dynamic_call_scrutinee_remains_conservative() {
    let output = analyze_text(
        "enum Choice { A, B } fn choose(flag: Bool) -> Choice { if flag { Choice::A } else { Choice::B } } fn main(flag: Bool) -> Int { var value: Int; match choose(flag) { Choice::A => { value = 1; 0 }, Choice::B => { value = 2; 0 }, }; value }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

#[test]
fn direct_constructor_with_dynamic_payload_keeps_existing_fast_path() {
    let output = analyze_text(
        "enum Maybe { None, Some(Int) } fn runtime(flag: Bool) -> Int { if flag { 1 } else { 2 } } fn main(flag: Bool) -> Int { var value: Int; match Maybe::Some(runtime(flag)) { Maybe::None => 0, Maybe::Some(inner) => { value = inner; 0 }, }; value }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(codes(&output), vec!["N3034"]);
}

#[test]
fn static_errors_in_nonselected_composite_arm_still_suppress_usefulness_warning() {
    let output = analyze_text(
        "enum Choice { A, B } fn main() -> Int { match if true { Choice::A } else { Choice::B } { Choice::A => 0, Choice::B => true, } }",
    );
    assert_eq!(codes(&output), vec!["N3004"]);
}
