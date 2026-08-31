use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "analyzer-immutable-enum-tags.nv", text);
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
fn function_scope_immutable_dynamic_payload_binding_preserves_tag() {
    let output = analyze_text(
        "enum Maybe { None, Some(Int) } fn runtime(flag: Bool) -> Int { if flag { 1 } else { 2 } } fn main(flag: Bool) -> Int { let maybe = Maybe::Some(runtime(flag)); var result: Int; match maybe { Maybe::None => 0, Maybe::Some(value) => { result = value; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn immutable_enum_alias_chain_preserves_tag() {
    let output = analyze_text(
        "enum Maybe { None, Some(Int) } fn runtime(flag: Bool) -> Int { if flag { 1 } else { 2 } } fn main(flag: Bool) -> Int { let first = Maybe::Some(runtime(flag)); let second = first; var result: Int; match second { Maybe::None => 0, Maybe::Some(value) => { result = value; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn immutable_selected_if_initializer_records_tag() {
    let output = analyze_text(
        "enum Maybe { None, Some(Int) } fn runtime(flag: Bool) -> Int { if flag { 1 } else { 2 } } fn main(flag: Bool) -> Int { let maybe = if true { Maybe::Some(runtime(flag)) } else { Maybe::None }; var result: Int; match maybe { Maybe::None => 0, Maybe::Some(value) => { result = value; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn mutable_enum_binding_remains_conservative() {
    let output = analyze_text(
        "enum Choice { A, B } fn main(flag: Bool) -> Int { var choice = Choice::A; if flag { choice = Choice::B; } else { (); }; var result: Int; match choice { Choice::A => { result = 1; 0 }, Choice::B => { result = 2; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

#[test]
fn dynamic_immutable_enum_initializer_remains_conservative() {
    let output = analyze_text(
        "enum Choice { A, B } fn main(flag: Bool) -> Int { let choice = if flag { Choice::A } else { Choice::B }; var result: Int; match choice { Choice::A => { result = 1; 0 }, Choice::B => { result = 2; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

#[test]
fn nested_shadowing_uses_resolved_immutable_tag_fact() {
    let output = analyze_text(
        "enum Choice { A, B } fn main() -> Int { let choice = Choice::A; { let choice = Choice::B; match choice { Choice::A => 0, Choice::B => 1, }; }; var result: Int; match choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 2);
}

#[test]
fn invalid_annotation_does_not_export_a_tag_fact() {
    let output = analyze_text(
        "enum Choice { A, B } enum Other { A, B } fn main() -> Int { let choice: Other = Choice::A; match choice { Other::A => 0, Other::B => 1, } }",
    );
    assert_eq!(code_count(&output, "N3004"), 1);
    assert_eq!(code_count(&output, "N3034"), 0);
}
