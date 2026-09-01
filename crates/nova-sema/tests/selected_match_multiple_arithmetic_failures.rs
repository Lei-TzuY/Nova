use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(
        SourceId::new(0),
        "selected-match-multiple-arithmetic-failures.nv",
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
fn selected_int_payload_reports_each_zero_divisor() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    let first = 1 / x;
                    let second = 2 / x;
                    0
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 2, "{:?}", output.diagnostics);
}

#[test]
fn selected_int_payload_reports_distinct_failure_classes() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    let max = 9223372036854775807;
                    let first = 1 / x;
                    let second = max + 1;
                    0
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3031"), 1, "{:?}", output.diagnostics);
}

#[test]
fn nested_selected_payload_environment_reports_each_failure() {
    let output = analyze_text(
        r#"
        enum Inner { Empty, Value(Int) }
        enum Outer { Empty, Value(Inner) }

        fn main() -> Int {
            match Outer::Value(Inner::Value(0)) {
                Outer::Empty => 0,
                Outer::Value(inner) => match inner {
                    Inner::Empty => 0,
                    Inner::Value(x) => {
                        let first = 1 / x;
                        let second = 2 / x;
                        0
                    },
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 2, "{:?}", output.diagnostics);
}

#[test]
fn selected_if_inside_payload_arm_reports_each_failure() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => if true {
                    let first = 1 / x;
                    let second = 2 / x;
                    0
                } else {
                    0
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 2, "{:?}", output.diagnostics);
}

#[test]
fn dynamic_payload_remains_runtime_only() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn runtime(value: Int) -> Int { value }

        fn main() -> Int {
            match Wrap::Value(runtime(0)) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    let first = 1 / x;
                    let second = 2 / x;
                    0
                },
            }
        }
        "#,
    );

    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3032"), 0, "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3031"), 0, "{:?}", output.diagnostics);
}

#[test]
fn proven_unselected_payload_arm_reports_no_execution_failures() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn main() -> Int {
            match Wrap::Empty {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    let first = 1 / x;
                    let second = 2 / x;
                    0
                },
            }
        }
        "#,
    );

    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3032"), 0, "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3031"), 0, "{:?}", output.diagnostics);
}
