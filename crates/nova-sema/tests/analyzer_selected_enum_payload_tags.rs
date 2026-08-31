use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(
        SourceId::new(0),
        "analyzer-selected-enum-payload-tags.nv",
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
fn selected_true_if_preserves_record_payload_summary() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } enum Wrap { Empty, Value(Holder) } fn main() -> Int { let wrapped = if true { Wrap::Value(new Holder { choice: Choice::A }) } else { Wrap::Value(new Holder { choice: Choice::B }) }; let holder = match wrapped { Wrap::Empty => new Holder { choice: Choice::B }, Wrap::Value(value) => value, }; var result: Int; match holder.choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 2);
}

#[test]
fn selected_false_if_preserves_else_payload_summary() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } enum Wrap { Empty, Value(Holder) } fn main() -> Int { let wrapped = if false { Wrap::Value(new Holder { choice: Choice::A }) } else { Wrap::Value(new Holder { choice: Choice::B }) }; let holder = match wrapped { Wrap::Empty => new Holder { choice: Choice::A }, Wrap::Value(value) => value, }; var result: Int; match holder.choice { Choice::A => 0, Choice::B => { result = 42; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 2);
}

#[test]
fn selected_match_preserves_enum_result_payload_summary() {
    let output = analyze_text(
        "enum Switch { Off, On } enum Choice { A, B } record Holder { choice: Choice } enum Wrap { Empty, Value(Holder) } fn main() -> Int { let wrapped = match Switch::On { Switch::Off => Wrap::Value(new Holder { choice: Choice::B }), Switch::On => Wrap::Value(new Holder { choice: Choice::A }), }; let holder = match wrapped { Wrap::Empty => new Holder { choice: Choice::B }, Wrap::Value(value) => value, }; var result: Int; match holder.choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 3);
}

#[test]
fn selected_match_can_forward_enum_payload_tag() {
    let output = analyze_text(
        "enum Switch { Off, On } enum Choice { A, B } enum Wrap { Empty, Value(Choice) } record Holder { choice: Choice } fn main() -> Int { let wrapped = match Switch::On { Switch::Off => Wrap::Value(Choice::B), Switch::On => Wrap::Value(Choice::A), }; let holder = match wrapped { Wrap::Empty => new Holder { choice: Choice::B }, Wrap::Value(value) => new Holder { choice: value }, }; var result: Int; match holder.choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 3);
}

#[test]
fn dynamic_if_payload_selection_remains_conservative() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } enum Wrap { Empty, Value(Holder) } fn main(flag: Bool) -> Int { let wrapped = if flag { Wrap::Value(new Holder { choice: Choice::A }) } else { Wrap::Value(new Holder { choice: Choice::B }) }; let holder = match wrapped { Wrap::Empty => new Holder { choice: Choice::A }, Wrap::Value(value) => value, }; var result: Int; match holder.choice { Choice::A => { result = 1; 0 }, Choice::B => { result = 2; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 0);
}

#[test]
fn payload_dependent_selected_match_result_remains_conservative() {
    let output = analyze_text(
        "enum Gate { Empty, Value(Bool) } enum Choice { A, B } record Holder { choice: Choice } enum Wrap { Empty, Value(Holder) } fn main(flag: Bool) -> Int { let wrapped = match Gate::Value(flag) { Gate::Empty => Wrap::Value(new Holder { choice: Choice::A }), Gate::Value(value) => if value { Wrap::Value(new Holder { choice: Choice::A }) } else { Wrap::Value(new Holder { choice: Choice::B }) }, }; let holder = match wrapped { Wrap::Empty => new Holder { choice: Choice::A }, Wrap::Value(value) => value, }; var result: Int; match holder.choice { Choice::A => { result = 1; 0 }, Choice::B => { result = 2; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 1);
}

#[test]
fn selected_if_branch_block_local_alias_preserves_payload_summary() {
    let output = analyze_text(
        "enum Choice { A, B } record Holder { choice: Choice } enum Wrap { Empty, Value(Holder) } fn main() -> Int { let wrapped = if true { let local = Wrap::Value(new Holder { choice: Choice::A }); local } else { Wrap::Value(new Holder { choice: Choice::B }) }; let holder = match wrapped { Wrap::Empty => new Holder { choice: Choice::B }, Wrap::Value(value) => value, }; var result: Int; match holder.choice { Choice::A => { result = 42; 0 }, Choice::B => 0, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 2);
}

#[test]
fn dynamic_match_scrutinee_keeps_payload_summary_conservative() {
    let output = analyze_text(
        "enum Switch { Off, On } enum Choice { A, B } record Holder { choice: Choice } enum Wrap { Empty, Value(Holder) } fn main(flag: Bool) -> Int { let wrapped = match if flag { Switch::Off } else { Switch::On } { Switch::Off => Wrap::Value(new Holder { choice: Choice::A }), Switch::On => Wrap::Value(new Holder { choice: Choice::B }), }; let holder = match wrapped { Wrap::Empty => new Holder { choice: Choice::A }, Wrap::Value(value) => value, }; var result: Int; match holder.choice { Choice::A => { result = 1; 0 }, Choice::B => { result = 2; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3034"), 0);
}
