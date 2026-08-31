use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "analyzer-selected-record-tags.nv", text);
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
fn selected_true_if_record_initializer_preserves_field_tag() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice, audit: Int } fn runtime(flag: Bool) -> Int { if flag { 1 } else { 2 } } fn main(flag: Bool) -> Int { let holder = if true { new Holder { choice: Choice::A, audit: runtime(flag) } } else { new Holder { choice: Choice::B, audit: 0 } }; var result: Int; match holder.choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn selected_false_if_record_initializer_preserves_else_tag() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } fn main() -> Int { let holder = if false { new Holder { choice: Choice::A } } else { new Holder { choice: Choice::B } }; var result: Int; match holder.choice { Choice::A => 0, Choice::B => { result = 42; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn selected_match_record_initializer_preserves_result_tag() {
    let output = analyze_text(
        "enum Switch { Off, On(Int) } enum Choice { A, B } record Holder { choice: Choice } fn runtime(flag: Bool) -> Int { if flag { 1 } else { 2 } } fn main(flag: Bool) -> Int { let holder = match Switch::On(runtime(flag)) { Switch::Off => new Holder { choice: Choice::B }, Switch::On(value) => new Holder { choice: Choice::A }, }; var result: Int; match holder.choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 2);
}

#[test]
fn statement_bearing_block_can_expose_record_tail_summary() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } fn runtime(flag: Bool) -> Int { if flag { 1 } else { 2 } } fn main(flag: Bool) -> Int { let holder = { runtime(flag); new Holder { choice: Choice::A } }; var result: Int; match holder.choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn dynamic_if_record_initializer_remains_conservative() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } fn main(flag: Bool) -> Int { let holder = if flag { new Holder { choice: Choice::A } } else { new Holder { choice: Choice::B } }; var result: Int; match holder.choice { Choice::A => { result = 1; 0 }, Choice::B => { result = 2; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

#[test]
fn payload_dependent_selected_match_record_result_remains_conservative() {
    let output = analyze_text(
        "enum Wrap { Value(Bool) } enum Choice { A, B } record Holder { choice: Choice } fn main(flag: Bool) -> Int { let holder = match Wrap::Value(flag) { Wrap::Value(value) => if value { new Holder { choice: Choice::A } } else { new Holder { choice: Choice::B } }, }; var result: Int; match holder.choice { Choice::A => { result = 1; 0 }, Choice::B => { result = 2; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 0);
}

#[test]
fn block_local_record_alias_remains_outside_analyzer_scope_summary() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } fn main() -> Int { let holder = { let local = new Holder { choice: Choice::A }; local }; var result: Int; match holder.choice { Choice::A => { result = 1; 0 }, Choice::B => { result = 2; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}
