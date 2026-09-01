use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(
        SourceId::new(0),
        "analyzer-recursive-structural-tags.nv",
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
fn nested_enum_payload_preserves_record_summary() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } enum Inner { Empty, Value(Holder) } enum Outer { Empty, Value(Inner) } fn main() -> Int { let outer = Outer::Value(Inner::Value(new Holder { choice: Choice::A })); let holder = match outer { Outer::Empty => new Holder { choice: Choice::B }, Outer::Value(inner) => match inner { Inner::Empty => new Holder { choice: Choice::B }, Inner::Value(value) => value, }, }; var result: Int; match holder.choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 3);
}

#[test]
fn nested_enum_payload_preserves_inner_enum_tag() {
    let output = analyze_text(
        "enum Leaf { A, B } enum Inner { Empty, Value(Leaf) } enum Outer { Empty, Value(Inner) } fn main() -> Int { let outer = Outer::Value(Inner::Value(Leaf::A)); let leaf = match outer { Outer::Empty => Leaf::B, Outer::Value(inner) => match inner { Inner::Empty => Leaf::B, Inner::Value(value) => value, }, }; var result: Int; match leaf { Leaf::A => { result = 42; 0 }, Leaf::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 3);
}

#[test]
fn record_field_preserves_payload_bearing_enum_tree() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } enum Inner { Empty, Value(Holder) } enum Outer { Empty, Value(Inner) } record Envelope { wrapped: Outer } fn main() -> Int { let envelope = new Envelope { wrapped: Outer::Value(Inner::Value(new Holder { choice: Choice::A })) }; let holder = match envelope.wrapped { Outer::Empty => new Holder { choice: Choice::B }, Outer::Value(inner) => match inner { Inner::Empty => new Holder { choice: Choice::B }, Inner::Value(value) => value, }, }; var result: Int; match holder.choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 3);
}

#[test]
fn dynamic_record_sibling_does_not_erase_nested_tree() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } enum Inner { Empty, Value(Holder) } enum Outer { Empty, Value(Inner) } record Envelope { wrapped: Outer, audit: Int } fn runtime(flag: Bool) -> Int { if flag { 1 } else { 2 } } fn main(flag: Bool) -> Int { let envelope = new Envelope { wrapped: Outer::Value(Inner::Value(new Holder { choice: Choice::A })), audit: runtime(flag) }; let holder = match envelope.wrapped { Outer::Empty => new Holder { choice: Choice::B }, Outer::Value(inner) => match inner { Inner::Empty => new Holder { choice: Choice::B }, Inner::Value(value) => value, }, }; var result: Int; match holder.choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 3);
}

#[test]
fn selected_if_payload_preserves_recursive_tree() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } enum Inner { Empty, Value(Holder) } enum Outer { Empty, Value(Inner) } fn main() -> Int { let outer = Outer::Value(if true { Inner::Value(new Holder { choice: Choice::A }) } else { Inner::Value(new Holder { choice: Choice::B }) }); let holder = match outer { Outer::Empty => new Holder { choice: Choice::B }, Outer::Value(inner) => match inner { Inner::Empty => new Holder { choice: Choice::B }, Inner::Value(value) => value, }, }; var result: Int; match holder.choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 3);
}

#[test]
fn dynamic_nested_payload_selection_remains_conservative() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } enum Inner { Empty, Value(Holder) } enum Outer { Empty, Value(Inner) } fn main(flag: Bool) -> Int { let outer = Outer::Value(if flag { Inner::Value(new Holder { choice: Choice::A }) } else { Inner::Value(new Holder { choice: Choice::B }) }); let holder = match outer { Outer::Empty => new Holder { choice: Choice::A }, Outer::Value(inner) => match inner { Inner::Empty => new Holder { choice: Choice::A }, Inner::Value(value) => value, }, }; var result: Int; match holder.choice { Choice::A => { result = 1; 0 }, Choice::B => { result = 2; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn mutable_nested_enum_source_remains_conservative() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } enum Inner { Empty, Value(Holder) } enum Outer { Empty, Value(Inner) } fn main(flag: Bool) -> Int { var inner = Inner::Value(new Holder { choice: Choice::A }); if flag { inner = Inner::Empty; } else { (); }; let outer = Outer::Value(inner); let holder = match outer { Outer::Empty => new Holder { choice: Choice::A }, Outer::Value(value) => match value { Inner::Empty => new Holder { choice: Choice::A }, Inner::Value(payload) => payload, }, }; var result: Int; match holder.choice { Choice::A => { result = 1; 0 }, Choice::B => { result = 2; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn inner_shadowing_does_not_reuse_recursive_payload_facts() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } enum Inner { Empty, Value(Holder) } enum Outer { Empty, Value(Inner) } fn main(flag: Bool) -> Int { let outer = Outer::Value(Inner::Value(new Holder { choice: Choice::A })); let holder = match outer { Outer::Empty => new Holder { choice: Choice::A }, Outer::Value(inner) => match inner { Inner::Empty => new Holder { choice: Choice::A }, Inner::Value(value) => { let value = if flag { new Holder { choice: Choice::A } } else { new Holder { choice: Choice::B } }; value }, }, }; var result: Int; match holder.choice { Choice::A => { result = 1; 0 }, Choice::B => { result = 2; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 2);
}
