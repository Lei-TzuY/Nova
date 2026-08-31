use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "analyzer-match-payload-tags.nv", text);
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
fn direct_constructor_record_payload_binding_preserves_summary() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } enum Wrap { Empty, Value(Holder) } fn main() -> Int { let holder = match Wrap::Value(new Holder { choice: Choice::A }) { Wrap::Empty => new Holder { choice: Choice::B }, Wrap::Value(value) => value, }; var result: Int; match holder.choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 2);
}

#[test]
fn direct_constructor_enum_payload_binding_can_seed_record_field() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } enum Wrap { Empty, Value(Choice) } fn main() -> Int { let holder = match Wrap::Value(Choice::A) { Wrap::Empty => new Holder { choice: Choice::B }, Wrap::Value(value) => new Holder { choice: value }, }; var result: Int; match holder.choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 2);
}

#[test]
fn record_payload_summary_survives_dynamic_sibling() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice, audit: Int } enum Wrap { Empty, Value(Holder) } fn runtime(flag: Bool) -> Int { if flag { 1 } else { 2 } } fn main(flag: Bool) -> Int { let holder = match Wrap::Value(new Holder { choice: Choice::A, audit: runtime(flag) }) { Wrap::Empty => new Holder { choice: Choice::B, audit: 0 }, Wrap::Value(value) => value, }; var result: Int; match holder.choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 2);
}

#[test]
fn payload_bearing_enum_payload_preserves_tag_without_closing_inner_payload() {
    let output = analyze_text(
        "enum Maybe { None, Some(Int) } record Holder { maybe: Maybe } enum Wrap { Empty, Value(Maybe) } fn runtime(flag: Bool) -> Int { if flag { 1 } else { 2 } } fn main(flag: Bool) -> Int { let holder = match Wrap::Value(Maybe::Some(runtime(flag))) { Wrap::Empty => new Holder { maybe: Maybe::None }, Wrap::Value(value) => new Holder { maybe: value }, }; var result: Int; match holder.maybe { Maybe::None => 0, Maybe::Some(value) => { result = value; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 2);
}

#[test]
fn mutable_record_payload_source_remains_conservative() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } enum Wrap { Empty, Value(Holder) } fn main(flag: Bool) -> Int { var source = new Holder { choice: Choice::A }; if flag { source = new Holder { choice: Choice::B }; } else { (); }; let holder = match Wrap::Value(source) { Wrap::Empty => new Holder { choice: Choice::A }, Wrap::Value(value) => value, }; var result: Int; match holder.choice { Choice::A => { result = 1; 0 }, Choice::B => { result = 2; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn dynamically_selected_record_payload_remains_conservative() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } enum Wrap { Empty, Value(Holder) } fn main(flag: Bool) -> Int { let holder = match Wrap::Value(if flag { new Holder { choice: Choice::A } } else { new Holder { choice: Choice::B } }) { Wrap::Empty => new Holder { choice: Choice::A }, Wrap::Value(value) => value, }; var result: Int; match holder.choice { Choice::A => { result = 1; 0 }, Choice::B => { result = 2; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn enum_alias_scrutinee_keeps_payload_provenance_conservative() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } enum Wrap { Empty, Value(Holder) } fn main() -> Int { let source = new Holder { choice: Choice::A }; let wrapped = Wrap::Value(source); let holder = match wrapped { Wrap::Empty => new Holder { choice: Choice::B }, Wrap::Value(value) => value, }; var result: Int; match holder.choice { Choice::A => { result = 1; 0 }, Choice::B => { result = 2; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn block_shadowing_payload_binding_does_not_reuse_outer_summary() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } enum Wrap { Empty, Value(Holder) } fn main(flag: Bool) -> Int { let holder = match Wrap::Value(new Holder { choice: Choice::A }) { Wrap::Empty => new Holder { choice: Choice::A }, Wrap::Value(value) => { let value = if flag { new Holder { choice: Choice::A } } else { new Holder { choice: Choice::B } }; value }, }; var result: Int; match holder.choice { Choice::A => { result = 1; 0 }, Choice::B => { result = 2; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}
