use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(
        SourceId::new(0),
        "return-proof-local-arithmetic-failures.nv",
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
fn selected_payload_return_reports_zero_divisor() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    return 1 / x;
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
}

#[test]
fn selected_payload_return_reports_each_sibling_failure() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    return (1 / x) + (2 / x);
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 2, "{:?}", output.diagnostics);
}

#[test]
fn selected_payload_return_reports_overflow() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn main() -> Int {
            match Wrap::Value(9223372036854775807) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    return x + 1;
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3031"), 1, "{:?}", output.diagnostics);
}

#[test]
fn dynamic_return_divisor_remains_runtime_only() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn runtime_int(value: Int) -> Int { value }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    let divisor = runtime_int(x);
                    return 1 / divisor;
                },
            }
        }
        "#,
    );

    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3032"), 0, "{:?}", output.diagnostics);
}

#[test]
fn proven_unselected_return_does_not_report_failure() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    if false {
                        return 1 / x;
                    } else {
                        0
                    };
                    return 0;
                },
            }
        }
        "#,
    );

    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3032"), 0, "{:?}", output.diagnostics);
}

#[test]
fn dynamic_if_returns_report_each_potential_failure() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn runtime_bool(value: Bool) -> Bool { value }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    if runtime_bool(true) {
                        return 1 / x;
                    } else {
                        return 2 / x;
                    }
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 2, "{:?}", output.diagnostics);
}

#[test]
fn static_tag_selects_only_the_reachable_return_arm() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }
        enum Choice { A(Int), B }

        fn runtime_int(value: Int) -> Int { value }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(zero) => match Choice::A(runtime_int(1)) {
                    Choice::A(_) => {
                        return 1 / zero;
                    },
                    Choice::B => {
                        return 2 / zero;
                    },
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
}

#[test]
fn return_stops_following_execution_failure_collection() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    return 0;
                    let unreachable = 1 / x;
                    unreachable
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 0, "{:?}", output.diagnostics);
}
