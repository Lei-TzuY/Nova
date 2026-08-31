use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(
        SourceId::new(0),
        "analyzer-block-local-record-tags.nv",
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

fn code_count(output: &AnalysisOutput, code: &str) -> usize {
    output
        .diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code == code)
        .count()
}

#[test]
fn block_local_record_alias_preserves_field_tag() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } fn main() -> Int { let holder = { let local = new Holder { choice: Choice::A }; local }; var result: Int; match holder.choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn chained_block_local_record_aliases_preserve_summary() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } fn main() -> Int { let holder = { let first = new Holder { choice: Choice::A }; let second = first; let third = second; third }; var result: Int; match holder.choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn block_local_enum_alias_can_seed_record_field_summary() {
    let output = analyze_text(
        "enum Maybe { None, Some(Int) } record Holder { maybe: Maybe } fn runtime(flag: Bool) -> Int { if flag { 1 } else { 2 } } fn main(flag: Bool) -> Int { let holder = { let maybe = Maybe::Some(runtime(flag)); let local = new Holder { maybe: maybe }; local }; var result: Int; match holder.maybe { Maybe::None => 0, Maybe::Some(value) => { result = value; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn runtime_statement_before_local_alias_does_not_erase_summary() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } fn runtime(flag: Bool) -> Int { if flag { 1 } else { 2 } } fn main(flag: Bool) -> Int { let holder = { runtime(flag); let local = new Holder { choice: Choice::A }; local }; var result: Int; match holder.choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn mutable_block_local_record_alias_remains_conservative() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } fn main(flag: Bool) -> Int { let holder = { var local = new Holder { choice: Choice::A }; if flag { local = new Holder { choice: Choice::B }; } else { (); }; local }; var result: Int; match holder.choice { Choice::A => { result = 1; 0 }, Choice::B => { result = 2; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

#[test]
fn dynamic_block_local_enum_field_remains_conservative() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } fn main(flag: Bool) -> Int { let holder = { let local = new Holder { choice: if flag { Choice::A } else { Choice::B } }; local }; var result: Int; match holder.choice { Choice::A => { result = 1; 0 }, Choice::B => { result = 2; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

#[test]
fn selected_if_block_local_alias_preserves_summary() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } fn main() -> Int { let holder = if true { let local = new Holder { choice: Choice::A }; local } else { new Holder { choice: Choice::B } }; var result: Int; match holder.choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn block_local_shadowing_uses_resolved_binding_identity() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } fn main() -> Int { let holder = { let local = new Holder { choice: Choice::A }; { let local = new Holder { choice: Choice::B }; local }; local }; var result: Int; match holder.choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}
