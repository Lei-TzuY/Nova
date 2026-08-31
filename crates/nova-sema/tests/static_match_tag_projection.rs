use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "static-match-tag-projection.nv", text);
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

fn code_count(output: &AnalysisOutput, code: &str) -> usize {
    output
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == code)
        .count()
}

#[test]
fn nested_selected_match_with_dynamic_payload_projects_result_tag() {
    let output = analyze_text(
        "enum Maybe { None, Some(Int) } enum Choice { A, B } fn runtime(flag: Bool) -> Int { if flag { 1 } else { 2 } } fn main(flag: Bool) -> Int { var result: Int; match match Maybe::Some(runtime(flag)) { Maybe::None => Choice::B, Maybe::Some(value) => Choice::A, } { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 2);
}

#[test]
fn record_projection_ignores_dynamic_sibling_for_variant_tag() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice, audit: Int } fn runtime(flag: Bool) -> Int { if flag { 1 } else { 2 } } fn main(flag: Bool) -> Int { var result: Int; match (new Holder { choice: Choice::A, audit: runtime(flag) }).choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn record_projection_preserves_dynamic_payload_tag() {
    let output = analyze_text(
        "enum Maybe { None, Some(Int) } record Holder { maybe: Maybe } fn runtime(flag: Bool) -> Int { if flag { 1 } else { 2 } } fn main(flag: Bool) -> Int { var result: Int; match (new Holder { maybe: Maybe::Some(runtime(flag)) }).maybe { Maybe::None => 0, Maybe::Some(value) => { result = value; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn statement_bearing_block_can_expose_direct_tail_tag() {
    let output = analyze_text(
        "enum Choice { A, B } fn runtime(flag: Bool) -> Int { if flag { 1 } else { 2 } } fn main(flag: Bool) -> Int { var result: Int; match { runtime(flag); Choice::A } { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn selected_if_branch_can_expose_tag_after_dynamic_statement() {
    let output = analyze_text(
        "enum Choice { A, B } fn runtime(flag: Bool) -> Int { if flag { 1 } else { 2 } } fn main(flag: Bool) -> Int { var result: Int; match if true { runtime(flag); Choice::A } else { Choice::B } { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn mutable_binding_tail_remains_conservative() {
    let output = analyze_text(
        "enum Choice { A, B } fn main() -> Int { var result: Int; match { var choice = Choice::A; choice = Choice::B; choice } { Choice::A => { result = 1; 0 }, Choice::B => { result = 2; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

#[test]
fn nested_match_result_depending_on_dynamic_payload_remains_conservative() {
    let output = analyze_text(
        "enum Maybe { None, Some(Bool) } enum Choice { A, B } fn main(flag: Bool) -> Int { var result: Int; match match Maybe::Some(flag) { Maybe::None => Choice::A, Maybe::Some(value) => if value { Choice::A } else { Choice::B }, } { Choice::A => { result = 1; 0 }, Choice::B => { result = 2; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}
