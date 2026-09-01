use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(
        SourceId::new(0),
        "dynamic-composite-proof-local-failures.nv",
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
fn dynamic_if_reachable_branch_reports_outer_payload_failure() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn runtime_bool(value: Bool) -> Bool { value }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => if runtime_bool(true) {
                    let bad = 1 / x;
                    0
                } else {
                    0
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
}

#[test]
fn dynamic_if_collects_failures_from_both_reachable_branches() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn runtime_bool(value: Bool) -> Bool { value }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => if runtime_bool(true) {
                    let first = 1 / x;
                    0
                } else {
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
fn dynamic_match_collects_failures_from_each_reachable_arm() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }
        enum Choice { A, B }

        fn choose(value: Bool) -> Choice {
            if value { Choice::A } else { Choice::B }
        }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => match choose(true) {
                    Choice::A => {
                        let first = 1 / x;
                        0
                    },
                    Choice::B => {
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
fn dynamic_match_payload_binding_shadows_outer_closed_binding() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }
        enum Choice { A(Int), B }

        fn choose(value: Int) -> Choice { Choice::A(value) }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => match choose(1) {
                    Choice::A(x) => {
                        let dynamic = 1 / x;
                        0
                    },
                    Choice::B => {
                        let closed = 2 / x;
                        0
                    },
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
}

#[test]
fn proven_unselected_if_branch_remains_diagnostic_only() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => if false {
                    let bad = 1 / x;
                    0
                } else {
                    0
                },
            }
        }
        "#,
    );

    assert!(output.is_success(), "{:?}", output.diagnostics);
    assert_eq!(code_count(&output, "N3032"), 0, "{:?}", output.diagnostics);
}
