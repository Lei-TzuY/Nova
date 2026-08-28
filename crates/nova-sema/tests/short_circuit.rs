use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(SourceId::new(0), "short-circuit.nv", text);
    let lexed = lex(&source);
    assert!(lexed.is_success(), "lex diagnostics: {:?}", lexed.diagnostics);
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
fn dynamic_and_or_rhs_initialization_is_conditional() {
    for text in [
        "fn f(flag: Bool) -> Int { var value: Int; flag && { value = 1; true }; value }",
        "fn f(flag: Bool) -> Int { var value: Int; flag || { value = 1; false }; value }",
    ] {
        let output = analyze_text(text);
        assert_eq!(codes(&output), vec!["N3009"], "{text}");
    }
}

#[test]
fn dynamic_and_or_noncontinuing_rhs_does_not_make_expression_noncontinuing() {
    for text in [
        "fn f(flag: Bool) -> Int { flag && { return 1; }; 2 }",
        "fn f(flag: Bool) -> Int { flag || { return 1; }; 2 }",
    ] {
        let output = analyze_text(text);
        assert!(output.is_success(), "{text}: {:?}", output.diagnostics);
        assert_eq!(output.program.functions[0].body.ty.to_string(), "Int");
    }
}

#[test]
fn dynamic_and_or_rhs_breaks_remain_reachable_loop_exits() {
    for text in [
        "fn f(flag: Bool) -> Int { while true { flag && { break; true }; } }",
        "fn f(flag: Bool) -> Int { while true { flag || { break; false }; } }",
    ] {
        let output = analyze_text(text);
        assert_eq!(codes(&output), vec!["N3007"], "{text}");
    }
}
