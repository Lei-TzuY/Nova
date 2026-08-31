use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "analyzer-record-field-tags.nv", text);
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
fn immutable_record_field_tag_survives_dynamic_sibling() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice, audit: Int } fn runtime(flag: Bool) -> Int { if flag { 1 } else { 2 } } fn main(flag: Bool) -> Int { let holder = new Holder { choice: Choice::A, audit: runtime(flag) }; var result: Int; match holder.choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn immutable_record_alias_chain_preserves_field_tag() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } fn main() -> Int { let first = new Holder { choice: Choice::A }; let second = first; var result: Int; match second.choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn record_field_can_reuse_an_immutable_enum_alias_tag() {
    let output = analyze_text(
        "enum Maybe { None, Some(Int) } record Holder { maybe: Maybe } fn runtime(flag: Bool) -> Int { if flag { 1 } else { 2 } } fn main(flag: Bool) -> Int { let maybe = Maybe::Some(runtime(flag)); let holder = new Holder { maybe: maybe }; var result: Int; match holder.maybe { Maybe::None => 0, Maybe::Some(value) => { result = value; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn payload_bearing_record_field_preserves_tag_without_closing_payload() {
    let output = analyze_text(
        "enum Maybe { None, Some(Bool) } record Holder { maybe: Maybe } fn main(flag: Bool) -> Int { let holder = new Holder { maybe: Maybe::Some(flag) }; var result: Int; match holder.maybe { Maybe::None => 0, Maybe::Some(value) => { if value { result = 1; } else { result = 2; }; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn dynamic_enum_field_tag_remains_conservative() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } fn main(flag: Bool) -> Int { let holder = new Holder { choice: if flag { Choice::A } else { Choice::B } }; var result: Int; match holder.choice { Choice::A => { result = 1; 0 }, Choice::B => { result = 2; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

#[test]
fn mutable_record_binding_remains_conservative() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } fn main(flag: Bool) -> Int { var holder = new Holder { choice: Choice::A }; if flag { holder = new Holder { choice: Choice::B }; } else { (); }; var result: Int; match holder.choice { Choice::A => { result = 1; 0 }, Choice::B => { result = 2; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

#[test]
fn nested_record_projection_preserves_known_tag() {
    let output = analyze_text(
        "enum Choice { A, B } record Inner { choice: Choice } record Outer { inner: Inner } fn main() -> Int { let outer = new Outer { inner: new Inner { choice: Choice::A } }; var result: Int; match outer.inner.choice { Choice::A => { result = 1; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}
