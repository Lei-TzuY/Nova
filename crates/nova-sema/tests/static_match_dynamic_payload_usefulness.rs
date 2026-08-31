use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(
        SourceId::new(0),
        "static-match-dynamic-payload-usefulness.nv",
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

fn codes(output: &AnalysisOutput) -> Vec<&str> {
    output
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect()
}

#[test]
fn selected_if_with_dynamic_payload_still_proves_variant_tag() {
    let output = analyze_text(
        "enum Maybe { None, Some(Int) } fn runtime(flag: Bool) -> Int { if flag { 1 } else { 2 } } fn main(flag: Bool) -> Int { var result: Int; match if true { Maybe::Some(runtime(flag)) } else { Maybe::None } { Maybe::None => 0, Maybe::Some(value) => { result = value; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(codes(&output), vec!["N3034"]);
}

#[test]
fn selected_false_if_with_dynamic_payload_still_proves_variant_tag() {
    let output = analyze_text(
        "enum Maybe { None, Some(Int) } fn runtime(flag: Bool) -> Int { if flag { 1 } else { 2 } } fn main(flag: Bool) -> Int { var result: Int; match if false { Maybe::None } else { Maybe::Some(runtime(flag)) } { Maybe::None => 0, Maybe::Some(value) => { result = value; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(codes(&output), vec!["N3034"]);
}

#[test]
fn dynamic_payload_value_is_not_promoted_to_closed_value_proof() {
    let output = analyze_text(
        "enum Maybe { None, Some(Int) } fn runtime(flag: Bool) -> Int { if flag { 1 } else { 2 } } fn main(flag: Bool) -> Int { var result: Int; while match if true { Maybe::Some(runtime(flag)) } else { Maybe::None } { Maybe::None => false, Maybe::Some(value) => value == 1, } { result = 42; break; } result }",
    );
    assert_eq!(codes(&output), vec!["N3009"]);
}

#[test]
fn dynamic_if_condition_remains_conservative_even_with_known_branch_tags() {
    let output = analyze_text(
        "enum Maybe { None, Some(Int) } fn runtime(flag: Bool) -> Int { if flag { 1 } else { 2 } } fn main(flag: Bool) -> Int { var result: Int; match if flag { Maybe::Some(runtime(flag)) } else { Maybe::None } { Maybe::None => { result = 1; 0 }, Maybe::Some(value) => { result = value; 0 }, }; result }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

#[test]
fn selected_if_dynamic_payload_preserves_nonselected_arm_type_checking() {
    let output = analyze_text(
        "enum Maybe { None, Some(Int) } fn runtime(flag: Bool) -> Int { if flag { 1 } else { 2 } } fn main(flag: Bool) -> Int { match if true { Maybe::Some(runtime(flag)) } else { Maybe::None } { Maybe::None => true, Maybe::Some(value) => value, } }",
    );
    assert_eq!(codes(&output), vec!["N3004"]);
}
