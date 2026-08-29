use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze, hir::Type};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "function-equality.nv", text);
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
fn matching_function_signatures_are_equality_comparable() {
    let output = analyze_text(
        "fn first() -> Int { 1 }\n\
         fn second() -> Int { 2 }\n\
         fn main() -> Bool { first == first && first != second }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(output.program.functions[2].body.ty, Type::Bool);
}

#[test]
fn different_function_signatures_remain_non_comparable() {
    let output = analyze_text(
        "fn integer() -> Int { 1 }\n\
         fn boolean() -> Bool { true }\n\
         fn main() -> Bool { integer == boolean }",
    );
    assert_eq!(codes(&output), vec!["N3004"]);
}

#[test]
fn direct_function_identity_refines_reachability_through_empty_blocks() {
    let output = analyze_text(
        "fn target() -> Int { 1 }\n\
         fn main() -> Int {\n\
             var value: Int;\n\
             if { target } == target { value = 42; () } else { () };\n\
             value\n\
         }",
    );
    assert!(output.is_success(), "{:?}", output.diagnostics);
}

#[test]
fn local_function_alias_equality_remains_dynamic_for_flow() {
    let output = analyze_text(
        "fn target() -> Int { 1 }\n\
         fn main() -> Int {\n\
             let alias = target;\n\
             var value: Int;\n\
             if alias == target { value = 42; () } else { () };\n\
             value\n\
         }",
    );
    assert_eq!(codes(&output), vec!["N3009"]);
}
