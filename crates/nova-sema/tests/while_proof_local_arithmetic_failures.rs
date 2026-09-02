use nova_lexer::lex;
use nova_parser::parse;
use nova_sema::{AnalysisOutput, analyze};
use nova_source::{SourceFile, SourceId};

fn analyze_text(text: &str) -> AnalysisOutput {
    let source = SourceFile::new(
        SourceId::new(0),
        "while-proof-local-arithmetic-failures.nv",
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
fn dynamic_while_body_reports_outer_payload_failure() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn runtime_bool(value: Bool) -> Bool { value }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    while runtime_bool(true) {
                        let bad = 1 / x;
                        break;
                    }
                    0
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
}

#[test]
fn dynamic_while_body_reports_each_outer_payload_failure() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn runtime_bool(value: Bool) -> Bool { value }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    while runtime_bool(true) {
                        let first = 1 / x;
                        let second = 2 / x;
                        break;
                    }
                    0
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 2, "{:?}", output.diagnostics);
}

#[test]
fn proven_false_while_skips_body_failure() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    while false {
                        let bad = 1 / x;
                        break;
                    }
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
fn dynamic_while_preserves_reachable_following_failure() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn runtime_bool(value: Bool) -> Bool { value }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    while runtime_bool(false) {
                        break;
                    }
                    let bad = 3 / x;
                    0
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
}

#[test]
fn proven_false_while_preserves_reachable_following_failure() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    while false {
                        break;
                    }
                    let bad = 4 / x;
                    0
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
}

#[test]
fn body_local_shadow_does_not_reuse_outer_closed_payload() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn runtime_bool(value: Bool) -> Bool { value }
        fn runtime_int(value: Int) -> Int { value }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    while runtime_bool(true) {
                        let x = runtime_int(1);
                        let dynamic = 1 / x;
                        break;
                    }
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
fn dynamic_divisor_inside_loop_body_remains_runtime_only() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn runtime_bool(value: Bool) -> Bool { value }
        fn runtime_int(value: Int) -> Int { value }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    let divisor = runtime_int(x);
                    while runtime_bool(true) {
                        let dynamic = 1 / divisor;
                        break;
                    }
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
fn while_condition_reports_proof_local_failure_before_body() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    while 1 / x == 0 {
                        break;
                    }
                    0
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
}

#[test]
fn proven_true_while_with_reachable_break_preserves_following_failure() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    while true {
                        break;
                    }
                    let bad = 1 / x;
                    0
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
}

#[test]
fn proven_true_while_with_potential_break_preserves_following_failure() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn runtime_bool(value: Bool) -> Bool { value }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    while true {
                        if runtime_bool(true) {
                            break;
                        } else {
                            continue;
                        };
                    }
                    let bad = 2 / x;
                    0
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
}

#[test]
fn nested_loop_break_does_not_exit_proven_true_outer_loop() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    while true {
                        while true {
                            break;
                        }
                    }
                    let diagnostic_only = 3 / x;
                    0
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 0, "{:?}", output.diagnostics);
}

#[test]
fn unreachable_break_does_not_exit_proven_true_loop() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    while true {
                        continue;
                        break;
                    }
                    let diagnostic_only = 4 / x;
                    0
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 0, "{:?}", output.diagnostics);
}

#[test]
fn rejected_call_does_not_create_a_loop_exit_for_following_failure() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn runtime_bool(value: Bool) -> Bool { value }
        fn takes_int(value: Int) -> Int { value }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    while true {
                        takes_int(if runtime_bool(true) {
                            break;
                        } else {
                            false
                        });
                    }
                    let diagnostic_only = 5 / x;
                    0
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 0, "{:?}", output.diagnostics);
}

#[test]
fn short_circuited_break_does_not_exit_proven_true_loop() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    while true {
                        false && {
                            break;
                            true
                        };
                    }
                    let diagnostic_only = 6 / x;
                    0
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 0, "{:?}", output.diagnostics);
}

#[test]
fn forced_short_circuit_break_exits_proven_true_loop() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    while true {
                        true && {
                            break;
                            true
                        };
                    }
                    let reachable = 7 / x;
                    0
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 1, "{:?}", output.diagnostics);
}

#[test]
fn statically_unselected_match_break_does_not_exit_proven_true_loop() {
    let output = analyze_text(
        r#"
        enum Wrap { Empty, Value(Int) }
        enum Choice { Stay, Exit }

        fn main() -> Int {
            match Wrap::Value(0) {
                Wrap::Empty => 0,
                Wrap::Value(x) => {
                    while true {
                        match Choice::Stay {
                            Choice::Stay => (),
                            Choice::Exit => { break; },
                        };
                    }
                    let diagnostic_only = 8 / x;
                    0
                },
            }
        }
        "#,
    );

    assert_eq!(code_count(&output, "N3032"), 0, "{:?}", output.diagnostics);
}
