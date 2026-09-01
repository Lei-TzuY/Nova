use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(
        SourceId::new(0),
        "assignment-proof-local-arithmetic-failures.nv",
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
fn selected_payload_assignment_reports_zero_divisor() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn main() -> Int {
            var sink: Int = 0;
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    sink = 1 / x;
                    0
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
}

#[test]
fn selected_payload_assignment_reports_each_sibling_failure() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn main() -> Int {
            var sink: Int = 0;
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    sink = (1 / x) + (2 / x);
                    0
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 2, "{:?}", output.diagnostics);
}

#[test]
fn selected_payload_assignment_reports_overflow() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn main() -> Int {
            var sink: Int = 0;
            match Wrap::Value(9223372036854775807) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    sink = x + 1;
                    0
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3031"), 1, "{:?}", output.diagnostics);
}

#[test]
fn dynamic_assignment_divisor_remains_runtime_only() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn runtime_int(value: Int) -> Int { value }

        fn main() -> Int {
            var sink: Int = 0;
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    let divisor = runtime_int(x);
                    sink = 1 / divisor;
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
fn assignment_preserves_following_reachable_failure() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn main() -> Int {
            var sink: Int = 0;
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    sink = 0;
                    let bad = 1 / x;
                    bad
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
}

#[test]
fn uninitialized_binding_preserves_following_reachable_failure() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    var scratch: Int;
                    let bad = 1 / x;
                    bad
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
}

#[test]
fn break_stops_following_execution_failure_collection() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    while true {
                        break;
                        let bad = 1 / x;
                        bad;
                    }
                    0
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 0, "{:?}", output.diagnostics);
}

#[test]
fn continue_stops_following_execution_failure_collection() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    while true {
                        continue;
                        let bad = 1 / x;
                        bad;
                    }
                    0
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 0, "{:?}", output.diagnostics);
}
