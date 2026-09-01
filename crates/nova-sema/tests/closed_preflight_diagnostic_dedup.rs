use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(
        SourceId::new(0),
        "closed-preflight-diagnostic-dedup.nv",
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
fn nested_closed_blocks_report_one_zero_divisor() {
    let output = analyze_text(
        r#"
        fn main() -> Int {
            {
                {
                    let zero = 0;
                    let bad = 1 / zero;
                    ()
                };
                ()
            };
            0
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
}

#[test]
fn selected_if_with_nested_closed_block_reports_one_overflow() {
    let output = analyze_text(
        r#"
        fn main() -> Int {
            if true {
                {
                    let max = 9223372036854775807;
                    let bad = max + 1;
                    ()
                }
            } else {
                ()
            };
            0
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3031"), 1, "{:?}", output.diagnostics);
}

#[test]
fn selected_match_with_nested_closed_block_reports_one_zero_divisor() {
    let output = analyze_text(
        r#"
        enum Choice { A, B }
        fn main() -> Int {
            match Choice::A {
                Choice::A => {
                    {
                        let zero = 0;
                        let bad = 1 / zero;
                        ()
                    }
                },
                Choice::B => (),
            };
            0
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
}

#[test]
fn direct_constant_failure_inside_nested_block_is_not_reported_twice() {
    let output = analyze_text(
        r#"
        fn main() -> Int {
            {
                let bad = 10 / 0;
                ()
            };
            0
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
}

#[test]
fn distinct_reachable_failures_at_distinct_spans_are_both_reported() {
    let output = analyze_text(
        r#"
        fn main() -> Int {
            {
                let zero = 0;
                let first = 1 / zero;
                ()
            };
            {
                let zero = 0;
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
fn dynamic_and_proven_unselected_nested_failures_remain_suppressed() {
    let output = analyze_text(
        r#"
        fn identity(value: Int) -> Int { value }
        fn main() -> Int {
            {
                let zero = identity(0);
                let bad = 1 / zero;
                ()
            };
            if true {
                0
            } else {
                {
                    {
                        let zero = 0;
                        let bad = 1 / zero;
                        ()
                    };
                    1
                }
            }
        }
        "#,
    );

    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3032"), 0, "{:?}", output.diagnostics);
}
