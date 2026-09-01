use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(
        SourceId::new(0),
        "closed-condition-arithmetic-failures.nv",
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
fn closed_bool_block_binding_reports_zero_division() {
    let output = analyze_text(
        r#"
        fn main() -> Int {
            if {
                let zero = 0;
                let bad = 1 / zero;
                true
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
fn closed_bool_block_discarded_expression_reports_zero_division() {
    let output = analyze_text(
        r#"
        fn main() -> Int {
            if {
                let zero = 0;
                1 / zero;
                true
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
fn closed_bool_block_binding_reports_overflow() {
    let output = analyze_text(
        r#"
        fn main() -> Int {
            if {
                let max = 9223372036854775807;
                let bad = max + 1;
                true
            } {
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
fn selected_match_condition_propagates_closed_payload_arithmetic_failure() {
    let output = analyze_text(
        r#"
        enum Choice { Value(Int), Other }
        fn main() -> Int {
            if match Choice::Value(0) {
                Choice::Value(zero) => {
                    let bad = 1 / zero;
                    true
                },
                Choice::Other => false,
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
fn dynamic_block_local_divisor_remains_runtime_checked() {
    let output = analyze_text(
        r#"
        fn identity(value: Int) -> Int { value }
        fn main() -> Int {
            if {
                let zero = identity(0);
                let bad = 1 / zero;
                true
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
