use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(
        SourceId::new(0),
        "analyzer-selected-payload-lowering-facts.nv",
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
fn three_level_selected_payloads_seed_nested_lowering() {
    let output = analyze_text(
        "enum Leaf { A, B } enum Inner { A(Leaf), B(Leaf) } enum Outer { A(Inner), B(Inner) } fn main() -> Int { let outer = Outer::A(Inner::A(Leaf::A)); match outer { Outer::A(inner) => match inner { Inner::A(leaf) => match leaf { Leaf::A => 1, Leaf::B => 2, }, Inner::B(_) => 0, }, Outer::B(_) => 0, } }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 3);
}

#[test]
fn selected_record_payload_seeds_field_match() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } enum Wrap { Empty, Value(Holder) } fn main() -> Int { let wrapped = Wrap::Value(new Holder { choice: Choice::A }); match wrapped { Wrap::Empty => 0, Wrap::Value(holder) => match holder.choice { Choice::A => 1, Choice::B => 2, }, } }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 2);
}

#[test]
fn enum_alias_scrutinee_seeds_selected_payload() {
    let output = analyze_text(
        "enum Inner { A, B } enum Outer { Empty, Value(Inner) } fn main() -> Int { let first = Outer::Value(Inner::A); let second = first; match second { Outer::Empty => 0, Outer::Value(inner) => match inner { Inner::A => 1, Inner::B => 2, }, } }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 2);
}

#[test]
fn record_field_scrutinee_seeds_selected_payload() {
    let output = analyze_text(
        "enum Inner { A, B } enum Outer { Empty, Value(Inner) } record Envelope { outer: Outer } fn main() -> Int { let envelope = new Envelope { outer: Outer::Value(Inner::A) }; match envelope.outer { Outer::Empty => 0, Outer::Value(inner) => match inner { Inner::A => 1, Inner::B => 2, }, } }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 2);
}

#[test]
fn unselected_arm_does_not_receive_payload_facts() {
    let output = analyze_text(
        "enum Inner { A, B } enum Outer { A(Inner), B(Inner) } fn main() -> Int { let outer = Outer::A(Inner::A); match outer { Outer::A(inner) => match inner { Inner::A => 1, Inner::B => 2, }, Outer::B(inner) => match inner { Inner::A => 3, Inner::B => 4, }, } }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 2);
}

#[test]
fn dynamic_outer_scrutinee_remains_conservative() {
    let output = analyze_text(
        "enum Inner { A, B } enum Outer { A(Inner), B(Inner) } fn main(flag: Bool) -> Int { let outer = if flag { Outer::A(Inner::A) } else { Outer::B(Inner::B) }; match outer { Outer::A(inner) => match inner { Inner::A => 1, Inner::B => 2, }, Outer::B(inner) => match inner { Inner::A => 3, Inner::B => 4, }, } }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 0);
}

#[test]
fn mutable_payload_source_remains_conservative() {
    let output = analyze_text(
        "enum Inner { A, B } enum Outer { Empty, Value(Inner) } fn main(flag: Bool) -> Int { var inner = Inner::A; if flag { inner = Inner::B; } else { (); }; let outer = Outer::Value(inner); match outer { Outer::Empty => 0, Outer::Value(value) => match value { Inner::A => 1, Inner::B => 2, }, } }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn same_name_shadowing_overrides_seeded_payload_fact() {
    let output = analyze_text(
        "enum Inner { A, B } enum Outer { Empty, Value(Inner) } fn main(flag: Bool) -> Int { let outer = Outer::Value(Inner::A); match outer { Outer::Empty => 0, Outer::Value(inner) => { let inner = if flag { Inner::A } else { Inner::B }; match inner { Inner::A => 1, Inner::B => 2, } }, } }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn bool_payload_does_not_become_a_value_fact() {
    let output = analyze_text(
        "enum Inner { A, B } enum Outer { Empty, Value(Bool) } fn main() -> Int { let outer = Outer::Value(true); match outer { Outer::Empty => 0, Outer::Value(flag) => { let inner = if flag { Inner::A } else { Inner::B }; match inner { Inner::A => 1, Inner::B => 2, } }, } }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}
