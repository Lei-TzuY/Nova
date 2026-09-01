use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(
        SourceId::new(0),
        "closed-selector-arithmetic-failures.nv",
        text,
    );
    let lexed = lex(&source);
    assert!(lexed.is_success(), "{:?}", lexed.diagnostics);
    let parsed = parse(&source, &lexed.tokens);
    assert!(parsed.is_success(), "{:?}", parsed.diagnostics);
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
fn closed_match_condition_scrutinee_propagates_zero_division() {
    let output = analyze_text(
        r#"
        enum Choice { A, B }
        fn main() -> Int {
            if match {
                let zero = 0;
                let bad = 1 / zero;
                Choice::A
            } {
                Choice::A => true,
                Choice::B => false,
            } {
                1
            } else {
                0
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
}

#[test]
fn closed_record_projection_base_propagates_overflow() {
    let output = analyze_text(
        r#"
        record Holder { flag: Bool }
        fn main() -> Int {
            if ({
                let max = 9223372036854775807;
                let bad = max + 1;
                new Holder { flag: true }
            }).flag {
                1
            } else {
                0
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3031"), 1, "{:?}", output.diagnostics);
}

#[test]
fn direct_static_match_scrutinee_propagates_zero_division() {
    let output = analyze_text(
        r#"
        enum Choice { A, B }
        fn main() -> Int {
            match {
                let zero = 0;
                let bad = 1 / zero;
                Choice::A
            } {
                Choice::A => 1,
                Choice::B => 2,
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
}

#[test]
fn dynamic_selector_arithmetic_remains_runtime_only() {
    let output = analyze_text(
        r#"
        enum Choice { A, B }
        fn identity(value: Int) -> Int { value }
        fn main() -> Int {
            if match {
                let zero = identity(0);
                let bad = 1 / zero;
                Choice::A
            } {
                Choice::A => true,
                Choice::B => false,
            } {
                1
            } else {
                0
            }
        }
        "#,
    );

    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3032"), 0, "{:?}", output.diagnostics);
}

#[test]
fn unselected_selector_branch_does_not_report_arithmetic_failure() {
    let output = analyze_text(
        r#"
        enum Choice { A, B }
        fn main() -> Int {
            match if true {
                Choice::A
            } else {
                let zero = 0;
                let bad = 1 / zero;
                Choice::B
            } {
                Choice::A => 1,
                Choice::B => 2,
            }
        }
        "#,
    );

    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3032"), 0, "{:?}", output.diagnostics);
}
