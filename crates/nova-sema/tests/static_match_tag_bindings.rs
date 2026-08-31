use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "static-match-tag-bindings.nv", text);
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
fn immutable_dynamic_payload_binding_preserves_static_tag() {
    let output = analyze_text(
        "enum Maybe { None, Some(Int) } fn runtime(flag: Bool) -> Int { if flag { 1 } else { 2 } } fn main(flag: Bool) -> Int { let maybe = Maybe::Some(runtime(flag)); var result: Int; match maybe { Maybe::None => 0, Maybe::Some(value) => { result = value; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn immutable_record_binding_projects_tag_with_dynamic_sibling() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice, audit: Int } fn runtime(flag: Bool) -> Int { if flag { 1 } else { 2 } } fn main(flag: Bool) -> Int { let holder = new Holder { choice: Choice::A, audit: runtime(flag) }; var result: Int; match holder.choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn closed_binding_survives_later_dynamic_statement_in_tag_block() {
    let output = analyze_text(
        "enum Choice { A, B } fn runtime(flag: Bool) -> Int { if flag { 1 } else { 2 } } fn main(flag: Bool) -> Int { var result: Int; match { let choose = true; runtime(flag); if choose { Choice::A } else { Choice::B } } { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn selected_payload_enum_alias_preserves_known_nested_tag() {
    let output = analyze_text(
        "enum Inner { A(Int), B } enum Outer { Empty, Wrap(Inner) } fn runtime(flag: Bool) -> Int { if flag { 1 } else { 2 } } fn main(flag: Bool) -> Int { var result: Int; match match Outer::Wrap(Inner::A(runtime(flag))) { Outer::Empty => Inner::B, Outer::Wrap(inner) => inner, } { Inner::A(value) => { result = value; 0 }, Inner::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 2);
}

#[test]
fn selected_payload_alias_with_dynamic_tag_remains_conservative() {
    let output = analyze_text(
        "enum Inner { A, B } enum Outer { Empty, Wrap(Inner) } fn main(flag: Bool) -> Int { var result: Int; match match Outer::Wrap(if flag { Inner::A } else { Inner::B }) { Outer::Empty => Inner::A, Outer::Wrap(inner) => inner, } { Inner::A => { result = 1; 0 }, Inner::B => { result = 2; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn tag_alias_does_not_promote_runtime_bool_payload_to_constant_condition() {
    let output = analyze_text(
        "enum Outer { Empty, Wrap(Bool) } enum Choice { A, B } fn main(flag: Bool) -> Int { var result: Int; match match Outer::Wrap(flag) { Outer::Empty => Choice::A, Outer::Wrap(value) => if value { Choice::A } else { Choice::B }, } { Choice::A => { result = 1; 0 }, Choice::B => { result = 2; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn static_binding_identity_respects_shadowed_immutable_names() {
    let output = analyze_text(
        "enum Choice { A, B } fn main() -> Int { var result: Int; match { let choice = Choice::A; { let choice = Choice::B; choice }; choice } { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}
