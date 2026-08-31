use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "analyzer-nested-record-tags.nv", text);
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
fn three_level_projection_preserves_enum_tag() {
    let output = analyze_text(
        "enum Choice { A, B } record Leaf { choice: Choice } record Middle { leaf: Leaf } record Root { middle: Middle } fn main() -> Int { let root = new Root { middle: new Middle { leaf: new Leaf { choice: Choice::A } } }; var result: Int; match root.middle.leaf.choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn dynamic_siblings_at_multiple_levels_do_not_block_nested_tag() {
    let output = analyze_text(
        "enum Choice { A, B } record Leaf { choice: Choice, audit: Int } record Root { leaf: Leaf, outer_audit: Int } fn runtime(flag: Bool) -> Int { if flag { 1 } else { 2 } } fn main(flag: Bool) -> Int { let root = new Root { leaf: new Leaf { choice: Choice::A, audit: runtime(flag) }, outer_audit: runtime(flag) }; var result: Int; match root.leaf.choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn nested_record_alias_chain_preserves_summary() {
    let output = analyze_text(
        "enum Choice { A, B } record Leaf { choice: Choice } record Root { leaf: Leaf } fn main() -> Int { let leaf = new Leaf { choice: Choice::A }; let leaf_alias = leaf; let root = new Root { leaf: leaf_alias }; let root_alias = root; var result: Int; match root_alias.leaf.choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn dynamic_nested_enum_tag_remains_conservative() {
    let output = analyze_text(
        "enum Choice { A, B } record Leaf { choice: Choice } record Root { leaf: Leaf } fn main(flag: Bool) -> Int { let root = new Root { leaf: new Leaf { choice: if flag { Choice::A } else { Choice::B } } }; var result: Int; match root.leaf.choice { Choice::A => { result = 1; 0 }, Choice::B => { result = 2; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

#[test]
fn mutable_nested_record_source_blocks_summary() {
    let output = analyze_text(
        "enum Choice { A, B } record Leaf { choice: Choice } record Root { leaf: Leaf } fn main(flag: Bool) -> Int { var leaf = new Leaf { choice: Choice::A }; if flag { leaf = new Leaf { choice: Choice::B }; } else { (); }; let root = new Root { leaf: leaf }; var result: Int; match root.leaf.choice { Choice::A => { result = 1; 0 }, Choice::B => { result = 2; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

#[test]
fn nested_payload_tag_does_not_close_runtime_payload() {
    let output = analyze_text(
        "enum Maybe { None, Some(Bool) } record Leaf { maybe: Maybe } record Root { leaf: Leaf } fn main(flag: Bool) -> Int { let root = new Root { leaf: new Leaf { maybe: Maybe::Some(flag) } }; var result: Int; match root.leaf.maybe { Maybe::None => 0, Maybe::Some(value) => { if value { result = 1; } else { result = 2; }; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}
