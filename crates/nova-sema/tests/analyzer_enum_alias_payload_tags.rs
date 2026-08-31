use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(
        SourceId::new(0),
        "analyzer-enum-alias-payload-tags.nv",
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
fn immutable_enum_alias_preserves_record_payload_summary() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } enum Wrap { Empty, Value(Holder) } fn main() -> Int { let source = new Holder { choice: Choice::A }; let wrapped = Wrap::Value(source); let holder = match wrapped { Wrap::Empty => new Holder { choice: Choice::B }, Wrap::Value(value) => value, }; var result: Int; match holder.choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 2);
}

#[test]
fn chained_enum_aliases_preserve_payload_summary() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } enum Wrap { Empty, Value(Holder) } fn main() -> Int { let first = Wrap::Value(new Holder { choice: Choice::A }); let second = first; let third = second; let holder = match third { Wrap::Empty => new Holder { choice: Choice::B }, Wrap::Value(value) => value, }; var result: Int; match holder.choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 2);
}

#[test]
fn block_local_enum_alias_preserves_payload_summary() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } enum Wrap { Empty, Value(Holder) } fn main() -> Int { let holder = { let wrapped = Wrap::Value(new Holder { choice: Choice::A }); match wrapped { Wrap::Empty => new Holder { choice: Choice::B }, Wrap::Value(value) => value, } }; var result: Int; match holder.choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 2);
}

#[test]
fn enum_payload_tag_survives_through_enum_alias() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } enum Wrap { Empty, Value(Choice) } fn main() -> Int { let wrapped = Wrap::Value(Choice::A); let holder = match wrapped { Wrap::Empty => new Holder { choice: Choice::B }, Wrap::Value(value) => new Holder { choice: value }, }; var result: Int; match holder.choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 2);
}

#[test]
fn aliased_record_payload_keeps_known_field_despite_dynamic_sibling() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice, audit: Int } enum Wrap { Empty, Value(Holder) } fn runtime(flag: Bool) -> Int { if flag { 1 } else { 2 } } fn main(flag: Bool) -> Int { let wrapped = Wrap::Value(new Holder { choice: Choice::A, audit: runtime(flag) }); let holder = match wrapped { Wrap::Empty => new Holder { choice: Choice::B, audit: 0 }, Wrap::Value(value) => value, }; var result: Int; match holder.choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 2);
}

#[test]
fn dynamically_selected_payload_inside_alias_remains_conservative() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } enum Wrap { Empty, Value(Holder) } fn main(flag: Bool) -> Int { let wrapped = Wrap::Value(if flag { new Holder { choice: Choice::A } } else { new Holder { choice: Choice::B } }); let holder = match wrapped { Wrap::Empty => new Holder { choice: Choice::A }, Wrap::Value(value) => value, }; var result: Int; match holder.choice { Choice::A => { result = 1; 0 }, Choice::B => { result = 2; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn mutable_enum_alias_remains_conservative() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } enum Wrap { Empty, Value(Holder) } fn main(flag: Bool) -> Int { var wrapped = Wrap::Value(new Holder { choice: Choice::A }); if flag { wrapped = Wrap::Empty; } else { (); }; let holder = match wrapped { Wrap::Empty => new Holder { choice: Choice::B }, Wrap::Value(value) => value, }; var result: Int; match holder.choice { Choice::A => { result = 1; 0 }, Choice::B => { result = 2; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 0);
}

#[test]
fn enum_alias_payload_from_mutable_record_source_remains_conservative() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } enum Wrap { Empty, Value(Holder) } fn main(flag: Bool) -> Int { var source = new Holder { choice: Choice::A }; if flag { source = new Holder { choice: Choice::B }; } else { (); }; let wrapped = Wrap::Value(source); let holder = match wrapped { Wrap::Empty => new Holder { choice: Choice::A }, Wrap::Value(value) => value, }; var result: Int; match holder.choice { Choice::A => { result = 1; 0 }, Choice::B => { result = 2; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn inner_same_name_binding_still_shadows_aliased_payload_summary() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } enum Wrap { Empty, Value(Holder) } fn main(flag: Bool) -> Int { let wrapped = Wrap::Value(new Holder { choice: Choice::A }); let holder = match wrapped { Wrap::Empty => new Holder { choice: Choice::A }, Wrap::Value(value) => { let value = if flag { new Holder { choice: Choice::A } } else { new Holder { choice: Choice::B } }; value }, }; var result: Int; match holder.choice { Choice::A => { result = 1; 0 }, Choice::B => { result = 2; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}
