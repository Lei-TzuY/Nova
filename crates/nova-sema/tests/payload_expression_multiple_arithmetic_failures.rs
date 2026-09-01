use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(
        SourceId::new(0),
        "payload-expression-multiple-arithmetic-failures.nv",
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
fn payload_dependent_binary_tree_reports_each_failure() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    let value = (1 / x) + (2 / x);
                    0
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 2, "{:?}", output.diagnostics);
}

#[test]
fn payload_dependent_call_arguments_report_each_failure() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn consume(left: Int, right: Int) -> Int { left + right }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    let value = consume(1 / x, 2 / x);
                    0
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 2, "{:?}", output.diagnostics);
}

#[test]
fn payload_dependent_record_fields_report_each_failure() {
    let output = analyze_text(
        r#"
        record Pair { left: Int, right: Int }
        enum Wrap { Empty, Value(Int) }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    let pair = new Pair {
                        left: 1 / x,
                        right: 2 / x,
                    };
                    0
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 2, "{:?}", output.diagnostics);
}

#[test]
fn dynamic_payload_expression_siblings_remain_runtime_only() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn runtime(value: Int) -> Int { value }
        fn consume(left: Int, right: Int) -> Int { left + right }

        fn main() -> Int {
            match Wrap::Value(runtime(0)) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    let value = consume(1 / x, 2 / x);
                    0
                },
            }
        }
        "#,
    );

    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3032"), 0, "{:?}", output.diagnostics);
}

#[test]
fn unselected_payload_expression_siblings_report_no_failures() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn consume(left: Int, right: Int) -> Int { left + right }

        fn main() -> Int {
            match Wrap::Empty {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    let value = consume(1 / x, 2 / x);
                    0
                },
            }
        }
        "#,
    );

    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3032"), 0, "{:?}", output.diagnostics);
}
