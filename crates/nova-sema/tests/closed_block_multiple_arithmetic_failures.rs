use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(
        SourceId::new(0),
        "closed-block-multiple-arithmetic-failures.nv",
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
fn one_closed_block_reports_each_distinct_zero_divisor() {
    let output = analyze_text(
        r#"
        fn main() -> Int {
            {
                let zero = 0;
                let first = 1 / zero;
                let second = 2 / zero;
                ()
            };
            0
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 2, "{:?}", output.diagnostics);
}

#[test]
fn one_closed_block_reports_distinct_failure_classes() {
    let output = analyze_text(
        r#"
        fn main() -> Int {
            {
                let zero = 0;
                let max = 9223372036854775807;
                let first = 1 / zero;
                let second = max + 1;
                ()
            };
            0
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3031"), 1, "{:?}", output.diagnostics);
}

#[test]
fn proven_unselected_block_still_reports_no_execution_failures() {
    let output = analyze_text(
        r#"
        fn main() -> Int {
            if true {
                0
            } else {
                {
                    let zero = 0;
                    let max = 9223372036854775807;
                    let first = 1 / zero;
                    let second = max + 1;
                    ()
                };
                1
            }
        }
        "#,
    );

    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3032"), 0, "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3031"), 0, "{:?}", output.diagnostics);
}
